use std::path::Path;

use csslint_core::FileId;

#[test]
fn does_not_emit_false_positives_for_nested_and_modern_css() {
    let source = r#"
@media (max-width: 800px) {
  .card {
    display: flex;
    background: var(--bg);
    padding: 8px;
  }

  .button {
    display: flex;
    background: var(--bg);
    padding: 8px;
  }
}

@property --button-bg {
  syntax: "<color>";
  inherits: false;
  initial-value: transparent;
}

.demo {
  pointer-events: none;
  outline-offset: 2px;
  scrollbar-width: thin;
  scroll-behavior: smooth;
  overscroll-behavior: contain;
  contain: paint;
}
"#;

    let extraction =
        csslint_extractor::extract_styles(FileId::new(900), Path::new("fixture.css"), source);
    assert!(
        extraction.diagnostics.is_empty(),
        "unexpected extraction diagnostics: {:#?}",
        extraction.diagnostics
    );
    assert_eq!(
        extraction.styles.len(),
        1,
        "fixture should produce a single extracted style block"
    );
    let parsed = csslint_parser::parse_style(&extraction.styles[0]).expect("fixture should parse");
    let semantic = csslint_semantic::build_semantic_model(&parsed);
    let diagnostics = csslint_rules::run_rules(&semantic);

    assert!(
        diagnostics.is_empty(),
        "expected no diagnostics, found: {diagnostics:#?}"
    );
}
