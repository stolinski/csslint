#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use csslint_core::{Diagnostic, FileId, Fix, RuleId, Severity, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedFix {
    pub file_id: FileId,
    pub span: Span,
    pub replacement: String,
    pub rule_id: RuleId,
    pub severity: Severity,
    pub priority: u16,
}

impl StagedFix {
    fn from_diagnostic(diagnostic: &Diagnostic) -> Option<Self> {
        let fix = diagnostic.fix.as_ref()?;
        Some(Self {
            file_id: diagnostic.file_id,
            span: fix.span,
            replacement: fix.replacement.clone(),
            rule_id: fix.rule_id.clone(),
            severity: diagnostic.severity,
            priority: fix.priority,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectedFixReason {
    MissingFileLength,
    InvalidSpan,
    OutOfBounds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedFix {
    pub file_id: FileId,
    pub span: Span,
    pub rule_id: RuleId,
    pub severity: Severity,
    pub priority: u16,
    pub reason: RejectedFixReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FixCollection {
    pub staged_by_file: BTreeMap<FileId, Vec<StagedFix>>,
    pub rejected: Vec<RejectedFix>,
}

impl FixCollection {
    pub fn staged_count(&self) -> usize {
        self.staged_by_file
            .values()
            .map(std::vec::Vec::len)
            .sum::<usize>()
    }
}

pub fn collect_fix_proposals(
    diagnostics: &[Diagnostic],
    file_lengths: &BTreeMap<FileId, usize>,
) -> FixCollection {
    let mut collection = FixCollection::default();

    for diagnostic in diagnostics {
        let Some(staged_fix) = StagedFix::from_diagnostic(diagnostic) else {
            continue;
        };

        let Some(&file_len) = file_lengths.get(&staged_fix.file_id) else {
            collection.rejected.push(rejected_fix(
                &staged_fix,
                RejectedFixReason::MissingFileLength,
            ));
            continue;
        };

        if staged_fix.span.start > staged_fix.span.end {
            collection
                .rejected
                .push(rejected_fix(&staged_fix, RejectedFixReason::InvalidSpan));
            continue;
        }

        if staged_fix.span.end > file_len {
            collection
                .rejected
                .push(rejected_fix(&staged_fix, RejectedFixReason::OutOfBounds));
            continue;
        }

        collection
            .staged_by_file
            .entry(staged_fix.file_id)
            .or_default()
            .push(staged_fix);
    }

    collection
}

pub fn collect_fix_proposals_for_file(
    file_id: FileId,
    source: &str,
    diagnostics: &[Diagnostic],
) -> FixCollection {
    let mut file_lengths = BTreeMap::new();
    file_lengths.insert(file_id, source.len());

    collect_fix_proposals(diagnostics, &file_lengths)
}

fn rejected_fix(staged_fix: &StagedFix, reason: RejectedFixReason) -> RejectedFix {
    RejectedFix {
        file_id: staged_fix.file_id,
        span: staged_fix.span,
        rule_id: staged_fix.rule_id.clone(),
        severity: staged_fix.severity,
        priority: staged_fix.priority,
        reason,
    }
}

pub fn apply_fixes(source: &str, fixes: &[Fix]) -> (String, usize) {
    if fixes.is_empty() {
        return (source.to_string(), 0);
    }

    let mut updated = source.to_string();
    let mut applied = 0;
    let mut ordered = fixes.to_vec();
    ordered.sort_by(|left, right| right.span.start.cmp(&left.span.start));

    for fix in ordered {
        if fix.span.start > fix.span.end || fix.span.end > updated.len() {
            continue;
        }

        updated.replace_range(fix.span.start..fix.span.end, &fix.replacement);
        applied += 1;
    }

    (updated, applied)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use csslint_core::{Diagnostic, FileId, Fix, RuleId, Severity, Span};

    use super::{collect_fix_proposals, RejectedFixReason};

    #[test]
    fn stages_valid_fix_proposals_per_file() {
        let diagnostics = vec![
            diagnostic_with_fix(FileId::new(1), Span::new(0, 4), Severity::Warn, 100),
            diagnostic_without_fix(FileId::new(1), Span::new(10, 12), Severity::Warn),
            diagnostic_with_fix(FileId::new(2), Span::new(4, 6), Severity::Error, 200),
        ];
        let file_lengths = BTreeMap::from([(FileId::new(1), 20usize), (FileId::new(2), 8usize)]);

        let collection = collect_fix_proposals(&diagnostics, &file_lengths);

        assert_eq!(collection.staged_by_file.len(), 2);
        assert_eq!(collection.staged_count(), 2);
        assert!(collection.rejected.is_empty());
    }

    #[test]
    fn rejects_invalid_span_and_out_of_bounds_proposals() {
        let diagnostics = vec![
            diagnostic_with_fix(FileId::new(1), Span::new(8, 4), Severity::Warn, 10),
            diagnostic_with_fix(FileId::new(1), Span::new(2, 24), Severity::Warn, 10),
        ];
        let file_lengths = BTreeMap::from([(FileId::new(1), 12usize)]);

        let collection = collect_fix_proposals(&diagnostics, &file_lengths);

        assert_eq!(collection.staged_count(), 0);
        assert_eq!(collection.rejected.len(), 2);
        assert_eq!(
            collection.rejected[0].reason,
            RejectedFixReason::InvalidSpan
        );
        assert_eq!(
            collection.rejected[1].reason,
            RejectedFixReason::OutOfBounds
        );
    }

    #[test]
    fn rejects_proposals_when_file_length_is_missing() {
        let diagnostics = vec![diagnostic_with_fix(
            FileId::new(10),
            Span::new(0, 2),
            Severity::Error,
            5,
        )];

        let collection = collect_fix_proposals(&diagnostics, &BTreeMap::new());

        assert_eq!(collection.staged_count(), 0);
        assert_eq!(collection.rejected.len(), 1);
        assert_eq!(
            collection.rejected[0].reason,
            RejectedFixReason::MissingFileLength
        );
    }

    fn diagnostic_with_fix(
        file_id: FileId,
        span: Span,
        severity: Severity,
        priority: u16,
    ) -> Diagnostic {
        Diagnostic::new(
            RuleId::from("test_rule"),
            severity,
            "fixture diagnostic",
            span,
            file_id,
        )
        .with_fix(Fix {
            span,
            replacement: String::from("replacement"),
            rule_id: RuleId::from("test_rule"),
            priority,
        })
    }

    fn diagnostic_without_fix(file_id: FileId, span: Span, severity: Severity) -> Diagnostic {
        Diagnostic::new(
            RuleId::from("test_rule"),
            severity,
            "fixture diagnostic",
            span,
            file_id,
        )
    }
}
