use std::path::Path;

use csslint_core::{Diagnostic, FileId};

const FIXABLE_RULES: [&str; 4] = [
    "no_empty_rules",
    "no_duplicate_declarations",
    "no_legacy_vendor_prefixes",
    "prefer_logical_properties",
];

#[test]
fn fixable_rules_are_idempotent_across_css_vue_and_svelte() {
    run_idempotency_case(
        "fixture.css",
        ".empty {}\n.box { color: red; color: red; -webkit-transform: rotate(0); display: -webkit-flex; margin-left: 1rem; }\n",
    );

    run_idempotency_case(
        "Fixture.vue",
        "<template><div class=\"box\"></div></template>\n<style scoped>\n.empty {}\n.box { color: red; color: red; -webkit-transform: rotate(0); display: -webkit-flex; margin-left: 1rem; }\n</style>\n",
    );

    run_idempotency_case(
        "Fixture.svelte",
        "<script>let count = 0;</script>\n<style>\n.empty {}\n.box { color: red; color: red; -webkit-transform: rotate(0); display: -webkit-flex; margin-left: 1rem; }\n</style>\n",
    );
}

#[test]
fn fixable_suite_unsupported_style_language_is_visible_and_not_fixable() {
    let source = "<template><div class=\"box\"></div></template>\n<style lang=\"scss\">\n.empty {}\n.box { color: red; color: red; -webkit-transform: rotate(0); display: -webkit-flex; margin-left: 1rem; }\n</style>\n";
    let diagnostics = lint_source("Fixture.vue", source, FileId::new(952));
    let fixable = fixable_diagnostics(&diagnostics);

    assert!(
        fixable.is_empty(),
        "unsupported lang blocks should not be classified as fixable rule diagnostics"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .to_ascii_lowercase()
            .contains("unsupported")),
        "unsupported lang should be emitted as extractor diagnostics"
    );

    let run = run_fix_engine(FileId::new(952), source, &fixable);
    assert_eq!(
        run.applied, 0,
        "no fixes should apply on unsupported lang blocks"
    );
    assert_eq!(
        run.rejected, 0,
        "no fix proposals should be rejected when none exist"
    );
}

fn run_idempotency_case(path: &str, source: &str) {
    let first_pass = lint_source(path, source, FileId::new(950));
    let fixable_first = fixable_diagnostics(&first_pass);

    for rule in FIXABLE_RULES {
        assert!(
            fixable_first
                .iter()
                .any(|diagnostic| diagnostic.rule_id.as_str() == rule),
            "{path} should report {rule} in first pass"
        );
    }
    assert!(
        fixable_first
            .iter()
            .all(|diagnostic| diagnostic.fix.is_some()),
        "{path} fixable diagnostics should all include fix proposals"
    );

    let first_fix_run = run_fix_engine(FileId::new(950), source, &fixable_first);
    assert!(
        first_fix_run.rejected == 0,
        "{path} should not reject valid fixable proposals"
    );
    assert!(
        first_fix_run.applied > 0,
        "{path} should apply at least one fix"
    );
    let fixed_source = first_fix_run.updated;

    let second_pass = lint_source(path, &fixed_source, FileId::new(951));
    let fixable_second = fixable_diagnostics(&second_pass);
    assert!(
        fixable_second.is_empty(),
        "{path} should be clean for all fixable rules after first fix pass"
    );

    let second_fix_run = run_fix_engine(FileId::new(951), &fixed_source, &fixable_second);
    assert_eq!(
        second_fix_run.applied, 0,
        "{path} second fix pass must be a no-op"
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

fn fixable_diagnostics(diagnostics: &[Diagnostic]) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .filter(|diagnostic| FIXABLE_RULES.contains(&diagnostic.rule_id.as_str()))
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
