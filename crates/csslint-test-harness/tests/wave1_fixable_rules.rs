use std::path::Path;

use csslint_core::{Diagnostic, FileId};

const WAVE1_RULES: [&str; 3] = [
    "no_empty_rules",
    "no_duplicate_declarations",
    "no_legacy_vendor_prefixes",
];

#[test]
fn wave1_fixable_rules_are_idempotent_across_css_vue_and_svelte() {
    run_wave1_idempotency_case(
        "fixture.css",
        ".empty {}\n.box { color: red; color: red; -webkit-transform: rotate(0); display: -webkit-flex; }\n",
    );

    run_wave1_idempotency_case(
        "Fixture.vue",
        "<template><div class=\"box\"></div></template>\n<style scoped>\n.empty {}\n.box { color: red; color: red; -webkit-transform: rotate(0); display: -webkit-flex; }\n</style>\n",
    );

    run_wave1_idempotency_case(
        "Fixture.svelte",
        "<script>let count = 0;</script>\n<style>\n.empty {}\n.box { color: red; color: red; -webkit-transform: rotate(0); display: -webkit-flex; }\n</style>\n",
    );
}

#[test]
fn wave1_unsupported_style_language_is_reported_and_produces_no_fixes() {
    let source = "<template><div class=\"box\"></div></template>\n<style lang=\"scss\">\n.empty {}\n.box { color: red; color: red; -webkit-transform: rotate(0); display: -webkit-flex; }\n</style>\n";
    let diagnostics = lint_source("Fixture.vue", source, FileId::new(902));
    let wave1 = wave1_diagnostics(&diagnostics);

    assert!(
        wave1.is_empty(),
        "unsupported lang blocks should not produce wave1 rule diagnostics"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .to_ascii_lowercase()
            .contains("unsupported")),
        "unsupported lang should be represented in extraction diagnostics"
    );

    let fix_run = run_fix_engine(FileId::new(902), source, &wave1);
    assert_eq!(
        fix_run.applied, 0,
        "no fixes should apply when wave1 diagnostics are empty"
    );
    assert_eq!(
        fix_run.rejected, 0,
        "no wave1 fix proposals should be rejected"
    );
}

fn run_wave1_idempotency_case(path: &str, source: &str) {
    let first_pass = lint_source(path, source, FileId::new(900));
    let wave1_first = wave1_diagnostics(&first_pass);
    assert!(
        wave1_first
            .iter()
            .any(|diagnostic| diagnostic.rule_id.as_str() == "no_empty_rules"),
        "{path} should report no_empty_rules in first pass"
    );
    assert!(
        wave1_first
            .iter()
            .any(|diagnostic| diagnostic.rule_id.as_str() == "no_duplicate_declarations"),
        "{path} should report no_duplicate_declarations in first pass"
    );
    assert!(
        wave1_first
            .iter()
            .any(|diagnostic| diagnostic.rule_id.as_str() == "no_legacy_vendor_prefixes"),
        "{path} should report no_legacy_vendor_prefixes in first pass"
    );

    let first_fix_run = run_fix_engine(FileId::new(900), source, &wave1_first);
    assert!(
        first_fix_run.rejected == 0,
        "{path} should not reject valid wave1 proposals"
    );
    assert!(
        first_fix_run.applied > 0,
        "{path} should apply at least one wave1 fix"
    );
    assert!(
        wave1_first
            .iter()
            .all(|diagnostic| diagnostic.fix.is_some()),
        "{path} wave1 diagnostics should all be fixable"
    );
    let fixed_source = first_fix_run.updated;

    let second_pass = lint_source(path, &fixed_source, FileId::new(901));
    let wave1_second = wave1_diagnostics(&second_pass);
    assert!(
        wave1_second.is_empty(),
        "{path} should be clean for wave1 rules after first fix pass"
    );

    let second_fix_run = run_fix_engine(FileId::new(901), &fixed_source, &wave1_second);
    assert_eq!(
        second_fix_run.applied, 0,
        "{path} second fix pass must be no-op"
    );
    assert_eq!(
        second_fix_run.updated, fixed_source,
        "{path} second pass should not change output"
    );
    assert_eq!(
        second_fix_run.rejected, 0,
        "{path} second pass should not reject any proposals"
    );
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

fn wave1_diagnostics(diagnostics: &[Diagnostic]) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .filter(|diagnostic| WAVE1_RULES.contains(&diagnostic.rule_id.as_str()))
        .cloned()
        .collect()
}

struct FixRun {
    updated: String,
    applied: usize,
    rejected: usize,
}

fn run_fix_engine(file_id: FileId, source: &str, diagnostics: &[Diagnostic]) -> FixRun {
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
