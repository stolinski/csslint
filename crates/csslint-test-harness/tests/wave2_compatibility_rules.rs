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
    assert!(
        wave2.len() >= 3,
        "imported.css should emit at least one diagnostic for each wave2 rule"
    );
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

#[test]
fn wave2_unsupported_style_language_reports_error_and_skips_rule_evaluation() {
    let diagnostics = lint_source(
        "Fixture.vue",
        "<template><article class=\"card\"></article></template>\n<style lang=\"scss\">\narticle.card { colr: red; }\narticle.card { color: blue; }\n</style>\n",
        FileId::new(923),
    );

    let wave2 = wave2_diagnostics(&diagnostics);
    assert!(
        wave2.is_empty(),
        "unsupported lang blocks should be skipped by wave2 rule evaluation"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .to_ascii_lowercase()
                .contains("unsupported")
        }),
        "unsupported lang should emit an extractor diagnostic"
    );
}

fn run_native_case(path: &str, source: &str, file_id: FileId) {
    let diagnostics = lint_source(path, source, file_id);
    let wave2 = wave2_diagnostics(&diagnostics);

    assert!(
        wave2.len() >= 3,
        "{path} should emit at least one diagnostic for each wave2 rule"
    );
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

fn wave2_diagnostics(diagnostics: &[Diagnostic]) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .filter(|diagnostic| WAVE2_RULES.contains(&diagnostic.rule_id.as_str()))
        .cloned()
        .collect()
}
