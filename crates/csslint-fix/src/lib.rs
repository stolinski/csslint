#![forbid(unsafe_code)]

use std::cmp::Ordering;
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
    OverlapsHigherPriorityFix,
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FixResolution {
    pub accepted_by_file: BTreeMap<FileId, Vec<StagedFix>>,
    pub dropped: Vec<RejectedFix>,
}

impl FixResolution {
    pub fn accepted_count(&self) -> usize {
        self.accepted_by_file
            .values()
            .map(std::vec::Vec::len)
            .sum::<usize>()
    }
}

pub fn resolve_overlaps(staged_by_file: &BTreeMap<FileId, Vec<StagedFix>>) -> FixResolution {
    let mut resolution = FixResolution::default();

    for (file_id, staged_fixes) in staged_by_file {
        let (accepted, mut dropped) = resolve_file_overlaps(staged_fixes);
        resolution.accepted_by_file.insert(*file_id, accepted);
        resolution.dropped.append(&mut dropped);
    }

    resolution
}

pub fn resolve_file_overlaps(staged_fixes: &[StagedFix]) -> (Vec<StagedFix>, Vec<RejectedFix>) {
    let mut candidates = staged_fixes.to_vec();
    candidates.sort_by(compare_fix_precedence);

    let mut accepted: Vec<StagedFix> = Vec::new();
    let mut dropped = Vec::new();

    for candidate in candidates {
        if accepted
            .iter()
            .any(|winner| spans_overlap(candidate.span, winner.span))
        {
            dropped.push(rejected_fix(
                &candidate,
                RejectedFixReason::OverlapsHigherPriorityFix,
            ));
            continue;
        }

        accepted.push(candidate);
    }

    accepted.sort_by(compare_fix_position);
    (accepted, dropped)
}

fn compare_fix_precedence(left: &StagedFix, right: &StagedFix) -> Ordering {
    severity_rank(right.severity)
        .cmp(&severity_rank(left.severity))
        .then_with(|| right.priority.cmp(&left.priority))
        .then_with(|| left.span.len().cmp(&right.span.len()))
        .then_with(|| left.rule_id.cmp(&right.rule_id))
        .then_with(|| left.span.start.cmp(&right.span.start))
        .then_with(|| left.span.end.cmp(&right.span.end))
        .then_with(|| left.replacement.cmp(&right.replacement))
}

fn compare_fix_position(left: &StagedFix, right: &StagedFix) -> Ordering {
    left.span
        .start
        .cmp(&right.span.start)
        .then_with(|| left.span.end.cmp(&right.span.end))
        .then_with(|| left.rule_id.cmp(&right.rule_id))
}

fn severity_rank(severity: Severity) -> u8 {
    match severity {
        Severity::Error => 3,
        Severity::Warn => 2,
        Severity::Off => 1,
    }
}

fn spans_overlap(left: Span, right: Span) -> bool {
    left.start < right.end && right.start < left.end
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

    let staged = fixes
        .iter()
        .map(|fix| StagedFix {
            file_id: FileId::new(0),
            span: fix.span,
            replacement: fix.replacement.clone(),
            rule_id: fix.rule_id.clone(),
            severity: Severity::Warn,
            priority: fix.priority,
        })
        .collect::<Vec<_>>();

    let (accepted, _dropped) = resolve_file_overlaps(&staged);
    apply_resolved_fixes(source, &accepted)
}

pub fn apply_resolved_fixes(source: &str, fixes: &[StagedFix]) -> (String, usize) {
    if fixes.is_empty() {
        return (source.to_string(), 0);
    }

    let source_len = source.len();
    let mut updated = source.as_bytes().to_vec();
    let mut applied = 0usize;
    let mut ordered = fixes.to_vec();
    ordered.sort_by(|left, right| {
        right
            .span
            .start
            .cmp(&left.span.start)
            .then_with(|| right.span.end.cmp(&left.span.end))
            .then_with(|| left.rule_id.cmp(&right.rule_id))
    });

    for fix in ordered {
        if fix.span.start > fix.span.end || fix.span.end > source_len {
            continue;
        }

        if !source.is_char_boundary(fix.span.start) || !source.is_char_boundary(fix.span.end) {
            continue;
        }

        updated.splice(
            fix.span.start..fix.span.end,
            fix.replacement.as_bytes().iter().copied(),
        );
        applied += 1;
    }

    match String::from_utf8(updated) {
        Ok(value) => (value, applied),
        Err(_) => (source.to_string(), 0),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use csslint_core::{Diagnostic, FileId, Fix, RuleId, Severity, Span};

    use super::{
        apply_fixes, apply_resolved_fixes, collect_fix_proposals, resolve_file_overlaps,
        resolve_overlaps, RejectedFixReason, StagedFix,
    };

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

    #[test]
    fn overlap_resolver_prefers_higher_severity() {
        let staged = vec![
            staged_fix(
                FileId::new(1),
                Span::new(1, 6),
                "warn_rule",
                Severity::Warn,
                100,
            ),
            staged_fix(
                FileId::new(1),
                Span::new(1, 6),
                "error_rule",
                Severity::Error,
                0,
            ),
        ];

        let (accepted, dropped) = resolve_file_overlaps(&staged);

        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].rule_id.as_str(), "error_rule");
        assert_eq!(dropped.len(), 1);
    }

    #[test]
    fn overlap_resolver_prefers_higher_priority_for_equal_severity() {
        let staged = vec![
            staged_fix(
                FileId::new(1),
                Span::new(0, 4),
                "low_priority",
                Severity::Warn,
                1,
            ),
            staged_fix(
                FileId::new(1),
                Span::new(0, 4),
                "high_priority",
                Severity::Warn,
                900,
            ),
        ];

        let (accepted, dropped) = resolve_file_overlaps(&staged);

        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].rule_id.as_str(), "high_priority");
        assert_eq!(dropped.len(), 1);
    }

    #[test]
    fn overlap_resolver_prefers_shorter_span_then_rule_id() {
        let shorter = staged_fix(
            FileId::new(1),
            Span::new(4, 7),
            "shorter_rule",
            Severity::Warn,
            50,
        );
        let longer = staged_fix(
            FileId::new(1),
            Span::new(2, 10),
            "longer_rule",
            Severity::Warn,
            50,
        );

        let (accepted, dropped) = resolve_file_overlaps(&[longer, shorter.clone()]);
        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].rule_id.as_str(), "shorter_rule");
        assert_eq!(dropped.len(), 1);

        let alpha = staged_fix(
            FileId::new(1),
            Span::new(2, 6),
            "alpha_rule",
            Severity::Warn,
            5,
        );
        let beta = staged_fix(
            FileId::new(1),
            Span::new(2, 6),
            "beta_rule",
            Severity::Warn,
            5,
        );
        let (accepted_tie, dropped_tie) = resolve_file_overlaps(&[beta, alpha.clone()]);
        assert_eq!(accepted_tie.len(), 1);
        assert_eq!(accepted_tie[0].rule_id.as_str(), "alpha_rule");
        assert_eq!(dropped_tie.len(), 1);
    }

    #[test]
    fn overlap_resolver_is_input_order_independent() {
        let fixes = vec![
            staged_fix(
                FileId::new(7),
                Span::new(1, 5),
                "z_rule",
                Severity::Warn,
                10,
            ),
            staged_fix(
                FileId::new(7),
                Span::new(1, 5),
                "a_rule",
                Severity::Warn,
                10,
            ),
            staged_fix(
                FileId::new(7),
                Span::new(20, 25),
                "later_rule",
                Severity::Warn,
                10,
            ),
        ];

        let mut first_order = BTreeMap::new();
        first_order.insert(FileId::new(7), fixes.clone());
        let mut second_order = BTreeMap::new();
        second_order.insert(
            FileId::new(7),
            vec![fixes[2].clone(), fixes[0].clone(), fixes[1].clone()],
        );

        let first = resolve_overlaps(&first_order);
        let second = resolve_overlaps(&second_order);

        assert_eq!(first.accepted_by_file, second.accepted_by_file);
        assert_eq!(first.dropped.len(), second.dropped.len());
    }

    #[test]
    fn descending_applier_handles_mixed_non_overlapping_ranges() {
        let source = "alpha beta gamma";
        let fixes = vec![
            staged_fix(
                FileId::new(1),
                Span::new(6, 10),
                "swap_beta",
                Severity::Warn,
                1,
            ),
            staged_fix(
                FileId::new(1),
                Span::new(11, 16),
                "trim_gamma",
                Severity::Warn,
                1,
            ),
        ];

        let mut accepted = fixes;
        accepted[0].replacement = "BETA".to_string();
        accepted[1].replacement = "g".to_string();

        let (updated, applied) = apply_resolved_fixes(source, &accepted);
        assert_eq!(updated, "alpha BETA g");
        assert_eq!(applied, 2);
    }

    #[test]
    fn descending_applier_preserves_unedited_crlf_segments() {
        let source = "a\r\nb\r\nc\r\n";
        let mut fix = staged_fix(
            FileId::new(1),
            Span::new(3, 4),
            "replace_b",
            Severity::Warn,
            1,
        );
        fix.replacement = "bee".to_string();

        let (updated, applied) = apply_resolved_fixes(source, &[fix]);
        assert_eq!(updated, "a\r\nbee\r\nc\r\n");
        assert_eq!(applied, 1);
    }

    #[test]
    fn descending_applier_skips_non_char_boundary_spans() {
        let source = "aéz";
        let mut invalid_fix = staged_fix(
            FileId::new(1),
            Span::new(2, 3),
            "invalid_boundary",
            Severity::Warn,
            1,
        );
        invalid_fix.replacement = "x".to_string();

        let (updated, applied) = apply_resolved_fixes(source, &[invalid_fix]);
        assert_eq!(updated, source);
        assert_eq!(applied, 0);
    }

    #[test]
    fn legacy_apply_fixes_path_resolves_overlaps_before_applying() {
        let source = "abcdef";
        let fixes = vec![
            Fix {
                span: Span::new(1, 4),
                replacement: "X".to_string(),
                rule_id: RuleId::from("low_priority"),
                priority: 1,
            },
            Fix {
                span: Span::new(2, 3),
                replacement: "Y".to_string(),
                rule_id: RuleId::from("high_priority"),
                priority: 100,
            },
        ];

        let (updated, applied) = apply_fixes(source, &fixes);
        assert_eq!(updated, "abYdef");
        assert_eq!(applied, 1);
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

    fn staged_fix(
        file_id: FileId,
        span: Span,
        rule_id: &'static str,
        severity: Severity,
        priority: u16,
    ) -> StagedFix {
        StagedFix {
            file_id,
            span,
            replacement: format!("replacement_for_{rule_id}"),
            rule_id: RuleId::from(rule_id),
            severity,
            priority,
        }
    }
}
