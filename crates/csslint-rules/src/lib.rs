#![forbid(unsafe_code)]

use csslint_core::{Diagnostic, RuleId, Severity};
use csslint_semantic::SemanticStyle;

pub fn run_rules(semantic: &SemanticStyle) -> Vec<Diagnostic> {
    let has_empty_block = semantic.source.contains("{}") || semantic.source.contains("{ }");
    if !has_empty_block {
        return Vec::new();
    }

    vec![Diagnostic::new(
        RuleId::from("no_empty_rules"),
        Severity::Warn,
        "Empty rule block detected",
        semantic.span,
        semantic.file_id,
    )]
}
