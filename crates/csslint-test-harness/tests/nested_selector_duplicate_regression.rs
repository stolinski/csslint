use std::path::Path;

use csslint_core::{Diagnostic, FileId};

fn run_diagnostics(source: &str) -> Vec<Diagnostic> {
    let extraction =
        csslint_extractor::extract_styles(FileId::new(901), Path::new("fixture.css"), source);
    assert!(
        extraction.diagnostics.is_empty(),
        "unexpected extraction diagnostics: {:#?}",
        extraction.diagnostics
    );
    assert_eq!(
        extraction.styles.len(),
        1,
        "fixture should produce one extracted style block"
    );
    let parsed = csslint_parser::parse_style(&extraction.styles[0]).expect("fixture should parse");
    let semantic = csslint_semantic::build_semantic_model(&parsed);
    csslint_rules::run_rules(&semantic)
}

fn duplicate_selector_count(diagnostics: &[Diagnostic]) -> usize {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.rule_id.as_str() == "no_duplicate_selectors")
        .count()
}

#[test]
fn nested_same_relative_selector_in_different_parents_is_not_duplicate() {
    let source = r#"
.foo {
  &:hover { color: red; }
}

.bar {
  &:hover { color: red; }
}
"#;

    let diagnostics = run_diagnostics(source);
    let duplicate_count = duplicate_selector_count(&diagnostics);

    assert_eq!(
        duplicate_count, 0,
        "unexpected duplicate selector diagnostics"
    );
}

#[test]
fn nested_same_relative_selector_in_same_parent_is_duplicate() {
    let source = r#"
.foo {
  &:hover { color: red; }
  &:hover { color: blue; }
}
"#;

    let diagnostics = run_diagnostics(source);
    let duplicate_count = duplicate_selector_count(&diagnostics);

    assert_eq!(
        duplicate_count, 1,
        "expected one duplicate selector diagnostic"
    );
}

#[test]
fn nested_duplicate_detection_is_deterministic_for_same_input() {
    let source = r#"
.foo {
  &:hover { color: red; }
  &:hover { color: blue; }
}
"#;

    let first = run_diagnostics(source)
        .into_iter()
        .map(|diagnostic| diagnostic.rule_id.as_str().to_string())
        .collect::<Vec<_>>();
    let second = run_diagnostics(source)
        .into_iter()
        .map(|diagnostic| diagnostic.rule_id.as_str().to_string())
        .collect::<Vec<_>>();

    assert_eq!(
        first, second,
        "diagnostic ordering must be deterministic for nested duplicate selector regression"
    );
}
