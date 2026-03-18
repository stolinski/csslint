use std::path::Path;

use csslint_core::{Diagnostic, FileId};

const WAVE2_RULES: [&str; 3] = [
    "no_duplicate_selectors",
    "no_unknown_properties",
    "no_overqualified_selectors",
];

#[test]
fn wave2_rules_cover_imported_css_compatibility_cases() {
    let diagnostics = lint_source(
        "imported.css",
        "article.card { colr: red; }\narticle.card { color: blue; }\n",
        FileId::new(920),
    );

    let wave2 = wave2_diagnostics(&diagnostics);
    assert_rule_presence("imported.css", &wave2, "no_duplicate_selectors");
    assert_rule_presence("imported.css", &wave2, "no_unknown_properties");
    assert_rule_presence("imported.css", &wave2, "no_overqualified_selectors");
    assert!(
        wave2.iter().all(|diagnostic| diagnostic.fix.is_none()),
        "wave2 compatibility rules should not produce autofixes"
    );
}

#[test]
fn wave2_rules_cover_native_vue_and_svelte_cases() {
    run_native_case(
        "Fixture.vue",
        "<template><article class=\"card\"></article></template>\n<style scoped>\narticle.card { colr: red; }\narticle.card { color: blue; }\n</style>\n",
        FileId::new(921),
    );

    run_native_case(
        "Fixture.svelte",
        "<script>let ready = true;</script>\n<style>\n:global(article.card) { colr: red; }\n:global(article.card) { color: blue; }\n</style>\n",
        FileId::new(922),
    );
}

fn run_native_case(path: &str, source: &str, file_id: FileId) {
    let diagnostics = lint_source(path, source, file_id);
    let wave2 = wave2_diagnostics(&diagnostics);

    assert_rule_presence(path, &wave2, "no_duplicate_selectors");
    assert_rule_presence(path, &wave2, "no_unknown_properties");
    assert_rule_presence(path, &wave2, "no_overqualified_selectors");
    assert!(
        wave2.iter().all(|diagnostic| diagnostic.fix.is_none()),
        "{path} wave2 compatibility rules should not produce autofixes"
    );
}

fn assert_rule_presence(path: &str, diagnostics: &[Diagnostic], rule_id: &str) {
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.rule_id.as_str() == rule_id),
        "{path} should report {rule_id}"
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

fn wave2_diagnostics(diagnostics: &[Diagnostic]) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .filter(|diagnostic| WAVE2_RULES.contains(&diagnostic.rule_id.as_str()))
        .cloned()
        .collect()
}
