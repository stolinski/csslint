use std::path::Path;

use csslint_core::{Diagnostic, FileId, Fix, RuleId, Severity, Span};

const FIXABLE_RULES: [&str; 4] = [
    "no_empty_rules",
    "no_duplicate_declarations",
    "no_legacy_vendor_prefixes",
    "prefer_logical_properties",
];

#[test]
fn overlap_matrix_pipeline_honors_deterministic_tie_breakers() {
    let file_id = FileId::new(42);
    let source = "abcdefghij";
    let diagnostics = vec![
        diagnostic_with_fix(
            file_id,
            "warn_long",
            Severity::Warn,
            Span::new(2, 7),
            "WARN_LONG",
            10,
        ),
        diagnostic_with_fix(
            file_id,
            "error_low",
            Severity::Error,
            Span::new(2, 7),
            "ERROR_LOW",
            1,
        ),
        diagnostic_with_fix(
            file_id,
            "error_high_long",
            Severity::Error,
            Span::new(2, 7),
            "ERROR_HIGH_LONG",
            20,
        ),
        diagnostic_with_fix(
            file_id,
            "error_high_short_beta",
            Severity::Error,
            Span::new(3, 6),
            "BETA",
            20,
        ),
        diagnostic_with_fix(
            file_id,
            "alpha",
            Severity::Error,
            Span::new(3, 6),
            "ALPHA",
            20,
        ),
    ];

    let run = run_fix_pipeline(file_id, source, &diagnostics);
    assert_eq!(run.rejected, 0);
    assert_eq!(run.applied, 1);
    assert_eq!(run.updated, "abcALPHAghij");
}

#[test]
fn fix_pipeline_handles_crlf_and_unicode_offsets_safely() {
    let file_id = FileId::new(43);
    let source = "a\r\nbé\r\nc";
    let diagnostics = vec![
        diagnostic_with_fix(
            file_id,
            "replace_b",
            Severity::Warn,
            Span::new(3, 4),
            "bee",
            10,
        ),
        diagnostic_with_fix(
            file_id,
            "invalid_unicode_boundary",
            Severity::Warn,
            Span::new(4, 5),
            "x",
            9,
        ),
        diagnostic_with_fix(
            file_id,
            "out_of_bounds",
            Severity::Warn,
            Span::new(40, 41),
            "z",
            1,
        ),
    ];

    let run = run_fix_pipeline(file_id, source, &diagnostics);
    assert_eq!(run.rejected, 1, "out-of-bounds fix should be rejected");
    assert_eq!(run.applied, 1, "only the valid CRLF-safe fix should apply");
    assert_eq!(run.updated, "a\r\nbeeé\r\nc");
}

#[test]
fn repeated_fix_pipeline_is_noop_after_first_pass() {
    let source = ".empty {}\n.box { color: red; color: red; -webkit-transform: rotate(0); display: -webkit-flex; margin-left: 1rem; }\n";

    let first = lint_source("fixture.css", source, FileId::new(950));
    let first_fixable = fixable_diagnostics(&first);
    let first_run = run_fix_pipeline(FileId::new(950), source, &first_fixable);

    assert_eq!(first_run.rejected, 0);
    assert!(first_run.applied > 0);

    let second = lint_source("fixture.css", &first_run.updated, FileId::new(951));
    let second_fixable = fixable_diagnostics(&second);
    let second_run = run_fix_pipeline(FileId::new(951), &first_run.updated, &second_fixable);

    assert_eq!(second_run.applied, 0);
    assert_eq!(second_run.rejected, 0);
    assert_eq!(second_run.updated, first_run.updated);
}

#[derive(Debug)]
struct FixRun {
    updated: String,
    applied: usize,
    rejected: usize,
}

fn run_fix_pipeline(file_id: FileId, source: &str, diagnostics: &[Diagnostic]) -> FixRun {
    let collection = csslint_fix::collect_fix_proposals_for_file(file_id, source, diagnostics);
    let staged = collection
        .staged_by_file
        .get(&file_id)
        .cloned()
        .unwrap_or_default();
    let (accepted, _dropped) = csslint_fix::resolve_file_overlaps(&staged);
    let (updated, applied) = csslint_fix::apply_resolved_fixes(source, &accepted);

    FixRun {
        updated,
        applied,
        rejected: collection.rejected.len(),
    }
}

fn diagnostic_with_fix(
    file_id: FileId,
    rule_id: &'static str,
    severity: Severity,
    span: Span,
    replacement: &str,
    priority: u16,
) -> Diagnostic {
    Diagnostic::new(
        RuleId::from(rule_id),
        severity,
        "fixture diagnostic",
        span,
        file_id,
    )
    .with_fix(Fix {
        span,
        replacement: replacement.to_string(),
        rule_id: RuleId::from(rule_id),
        priority,
    })
}

fn lint_source(path: &str, source: &str, file_id: FileId) -> Vec<Diagnostic> {
    let extraction = csslint_extractor::extract_styles(file_id, Path::new(path), source);
    let mut diagnostics = extraction.diagnostics;

    for style in extraction.styles {
        match csslint_parser::parse_style(&style) {
            Ok(parsed) => {
                let semantic = csslint_semantic::build_semantic_model(&parsed);
                diagnostics.extend(csslint_rules::run_rules(&semantic));
            }
            Err(diagnostic) => diagnostics.push(*diagnostic),
        }
    }

    diagnostics
}

fn fixable_diagnostics(diagnostics: &[Diagnostic]) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .filter(|diagnostic| FIXABLE_RULES.contains(&diagnostic.rule_id.as_str()))
        .cloned()
        .collect()
}
