use std::path::Path;

use csslint_core::FileId;

#[test]
fn emits_no_empty_rules_diagnostic_for_empty_block() {
    let file_id = FileId::new(7);
    let extraction =
        csslint_extractor::extract_styles(file_id, Path::new("fixture.css"), ".btn {}");
    let parsed = csslint_parser::parse_style(&extraction.styles[0])
        .expect("parser should accept non-empty style");
    let semantic = csslint_semantic::build_semantic_model(&parsed);
    let diagnostics = csslint_rules::run_rules(&semantic);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_id.as_str(), "no_empty_rules");
}
