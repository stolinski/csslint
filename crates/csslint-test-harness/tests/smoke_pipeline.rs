use std::path::Path;

use csslint_core::FileId;

#[test]
fn emits_no_empty_rules_diagnostic_for_empty_block() {
    let file_id = FileId::new(7);
    let extraction =
        csslint_extractor::extract_styles(file_id, Path::new("fixture.css"), ".btn {}");
    assert!(
        extraction.diagnostics.is_empty(),
        "plain css extraction should not emit diagnostics"
    );
    assert_eq!(
        extraction.styles.len(),
        1,
        "plain css fixture should yield exactly one style"
    );

    let style = &extraction.styles[0];
    assert_eq!(style.content, ".btn {}");
    assert_eq!(style.start_offset, 0);
    assert_eq!(style.end_offset, style.content.len());

    let parsed = csslint_parser::parse_style(style).expect("parser should accept empty block");
    assert!(parsed.parsed_with_lightning);

    let semantic = csslint_semantic::build_semantic_model(&parsed);
    let diagnostics = csslint_rules::run_rules(&semantic);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_id.as_str(), "no_empty_rules");
    assert_eq!(diagnostics[0].severity.as_str(), "warn");
    assert!(
        diagnostics[0]
            .message
            .to_ascii_lowercase()
            .contains("empty"),
        "diagnostic message should explain empty rule"
    );
}

#[test]
fn emits_no_diagnostics_for_non_empty_rule_block() {
    let extraction = csslint_extractor::extract_styles(
        FileId::new(8),
        Path::new("fixture.css"),
        ".btn { color: red; }",
    );
    let parsed = csslint_parser::parse_style(&extraction.styles[0])
        .expect("parser should accept non-empty declaration block");
    let semantic = csslint_semantic::build_semantic_model(&parsed);
    let diagnostics = csslint_rules::run_rules(&semantic);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.rule_id.as_str() != "no_empty_rules"),
        "non-empty declaration block should not trigger no_empty_rules"
    );
}
