use std::path::Path;

use csslint_core::FileId;

#[test]
fn parses_modern_selector_features() {
    let source = ":is(.a, .b) > .c[data-kind=\"x\"] { color: red; }";
    let extraction =
        csslint_extractor::extract_styles(FileId::new(100), Path::new("modern.css"), source);
    assert_eq!(extraction.styles.len(), 1);
    let parsed = csslint_parser::parse_style(&extraction.styles[0])
        .expect("modern selector syntax should parse");

    assert!(parsed.parsed_with_lightning);
}

#[test]
fn parses_escaped_selector_tokens() {
    let source = ".icon\\+btn { color: red; }";
    let extraction =
        csslint_extractor::extract_styles(FileId::new(101), Path::new("escaped.css"), source);
    assert_eq!(extraction.styles.len(), 1);
    let parsed =
        csslint_parser::parse_style(&extraction.styles[0]).expect("escaped selector should parse");

    assert!(parsed.parsed_with_lightning);
}

#[test]
fn maps_malformed_css_to_parser_diagnostic() {
    let source = ".broken { color: red";
    let extraction =
        csslint_extractor::extract_styles(FileId::new(102), Path::new("broken.css"), source);
    assert_eq!(extraction.styles.len(), 1);
    let diagnostic = csslint_parser::parse_style(&extraction.styles[0])
        .expect_err("malformed css should fail parse");

    assert_eq!(diagnostic.rule_id.as_str(), "parser_syntax_error");
    assert_eq!(diagnostic.severity.as_str(), "error");
    assert!(
        !diagnostic.message.is_empty(),
        "parser diagnostic should include a human-readable message"
    );
    assert!(
        diagnostic.span.start < diagnostic.span.end,
        "diagnostic span should point to a non-empty malformed region"
    );
    assert!(
        diagnostic.span.end <= source.len(),
        "diagnostic span should be bounded by source length"
    );
}
