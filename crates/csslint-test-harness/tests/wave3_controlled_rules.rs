use std::path::Path;

use csslint_core::{Diagnostic, FileId};

const WAVE3_RULES: [&str; 2] = ["no_invalid_values", "no_deprecated_features"];

#[test]
fn wave3_rules_cover_imported_css_cases() {
    let diagnostics = lint_source(
        "imported.css",
        "@viewport { width: device-width; }\n.card { display: squish; }\n",
        FileId::new(930),
    );

    let wave3 = wave3_diagnostics(&diagnostics);
    assert_eq!(
        wave3.len(),
        2,
        "imported.css should emit exactly two wave3 diagnostics"
    );
    assert_rule_presence("imported.css", &wave3, "no_invalid_values");
    assert_rule_presence("imported.css", &wave3, "no_deprecated_features");
    assert!(
        wave3.iter().all(|diagnostic| diagnostic.fix.is_none()),
        "wave3 rules should not produce autofixes"
    );
}

#[test]
fn wave3_rules_cover_native_vue_and_svelte_cases() {
    run_native_case(
        "Fixture.vue",
        "<template><article class=\"card\"></article></template>\n<style scoped>\n@viewport { width: device-width; }\narticle.card { display: squish; }\n</style>\n",
        FileId::new(931),
    );

    run_native_case(
        "Fixture.svelte",
        "<script>let ready = true;</script>\n<style>\n.title { display: squish; clip: rect(1px,2px,3px,4px); }\n</style>\n",
        FileId::new(932),
    );
}

#[test]
fn wave3_unsupported_style_language_reports_error_and_skips_rules() {
    let diagnostics = lint_source(
        "Fixture.vue",
        "<template><article class=\"card\"></article></template>\n<style lang=\"scss\">\n@viewport { width: device-width; }\narticle.card { display: squish; }\n</style>\n",
        FileId::new(933),
    );
    let wave3 = wave3_diagnostics(&diagnostics);

    assert!(
        wave3.is_empty(),
        "unsupported lang blocks should be skipped for wave3 rules"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .to_ascii_lowercase()
            .contains("unsupported")),
        "unsupported lang should emit an extractor diagnostic"
    );
}

fn run_native_case(path: &str, source: &str, file_id: FileId) {
    let diagnostics = lint_source(path, source, file_id);
    let wave3 = wave3_diagnostics(&diagnostics);

    assert_eq!(
        wave3.len(),
        2,
        "{path} should emit exactly two wave3 diagnostics"
    );
    assert_rule_presence(path, &wave3, "no_invalid_values");
    assert_rule_presence(path, &wave3, "no_deprecated_features");
    assert!(
        wave3.iter().all(|diagnostic| diagnostic.fix.is_none()),
        "{path} wave3 rules should not produce autofixes"
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

fn wave3_diagnostics(diagnostics: &[Diagnostic]) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .filter(|diagnostic| WAVE3_RULES.contains(&diagnostic.rule_id.as_str()))
        .cloned()
        .collect()
}
