use std::path::Path;

use csslint_core::{Diagnostic, FileId};

#[test]
fn wave4_rules_cover_native_vue_and_svelte_cases() {
    run_native_case(
        "Fixture.vue",
        "<template><article class=\"card\"></article></template>\n<style scoped>\n:global(.theme-root) { color: red; }\n.card { margin-left: 1rem; }\n</style>\n",
        FileId::new(940),
    );

    run_native_case(
        "Fixture.svelte",
        "<script>let ready = true;</script>\n<style>\n:global(#app) { color: red; }\n.title { right: 0; }\n</style>\n",
        FileId::new(941),
    );
}

#[test]
fn vue_style_src_blocks_emit_warning_and_skip_framework_rules() {
    let diagnostics = lint_source(
        "Fixture.vue",
        "<template><div class=\"card\"></div></template>\n<style src=\"./external.css\"></style>\n",
        FileId::new(943),
    );

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .to_ascii_lowercase()
                .contains("style src")
        }),
        "style src should emit a non-fatal extraction warning"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.rule_id.as_str() != "no_global_leaks"),
        "style src blocks should be skipped for no_global_leaks"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.rule_id.as_str() != "prefer_logical_properties"),
        "style src blocks should be skipped for prefer_logical_properties"
    );
}

#[test]
fn prefer_logical_properties_also_runs_for_plain_css() {
    let diagnostics = lint_source(
        "fixture.css",
        ".card { padding-right: 1rem; }",
        FileId::new(942),
    );

    let logical = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.rule_id.as_str() == "prefer_logical_properties")
        .collect::<Vec<_>>();
    assert_eq!(logical.len(), 1);
    assert!(
        logical[0].fix.is_some(),
        "logical-property warning should be fixable"
    );
    assert!(diagnostics
        .iter()
        .all(|diagnostic| diagnostic.rule_id.as_str() != "no_global_leaks"));
}

fn run_native_case(path: &str, source: &str, file_id: FileId) {
    let diagnostics = lint_source(path, source, file_id);

    let leaks = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.rule_id.as_str() == "no_global_leaks")
        .collect::<Vec<_>>();
    assert_eq!(
        leaks.len(),
        1,
        "{path} should report exactly one no_global_leaks diagnostic"
    );
    assert!(
        leaks.iter().all(|diagnostic| diagnostic.fix.is_none()),
        "{path} no_global_leaks diagnostics should not be fixable"
    );

    let logical = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.rule_id.as_str() == "prefer_logical_properties")
        .collect::<Vec<_>>();
    assert_eq!(
        logical.len(),
        1,
        "{path} should report exactly one prefer_logical_properties diagnostic"
    );
    assert!(
        logical.iter().all(|diagnostic| diagnostic.fix.is_some()),
        "{path} prefer_logical_properties diagnostics should include fixes"
    );

    assert_eq!(
        leaks.len() + logical.len(),
        2,
        "{path} fixture should emit exactly one framework diagnostic per wave4 rule"
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
