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

    let first_fixes = diagnostics_to_fixes(&wave1_first);
    assert!(
        !first_fixes.is_empty(),
        "{path} should produce safe wave1 fixes in first pass"
    );

    let (fixed_source, applied_first) = csslint_fix::apply_fixes(source, &first_fixes);
    assert!(applied_first > 0, "{path} should apply at least one fix");

    let second_pass = lint_source(path, &fixed_source, FileId::new(901));
    let wave1_second = wave1_diagnostics(&second_pass);
    assert!(
        wave1_second.is_empty(),
        "{path} should be clean for wave1 rules after first fix pass"
    );

    let second_fixes = diagnostics_to_fixes(&wave1_second);
    let (second_fixed_source, applied_second) =
        csslint_fix::apply_fixes(&fixed_source, &second_fixes);
    assert_eq!(applied_second, 0, "{path} second fix pass must be no-op");
    assert_eq!(
        second_fixed_source, fixed_source,
        "{path} second pass should not change output"
    );
}

fn lint_source(path: &str, source: &str, file_id: FileId) -> Vec<Diagnostic> {
    let extraction = csslint_extractor::extract_styles(file_id, Path::new(path), source);
    let mut diagnostics = extraction.diagnostics;

    for style in extraction.styles {
        if let Ok(parsed) = csslint_parser::parse_style(&style) {
            let semantic = csslint_semantic::build_semantic_model(&parsed);
            diagnostics.extend(csslint_rules::run_rules(&semantic));
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

fn diagnostics_to_fixes(diagnostics: &[Diagnostic]) -> Vec<csslint_core::Fix> {
    diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic.fix.clone())
        .collect()
}
