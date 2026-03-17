#![forbid(unsafe_code)]

use csslint_core::{Diagnostic, RuleId, Severity};
use csslint_semantic::CssSemanticModel;

pub fn run_rules(semantic: &CssSemanticModel) -> Vec<Diagnostic> {
    semantic
        .rules
        .iter()
        .filter(|rule| !rule.is_at_rule && rule.declaration_ids.is_empty())
        .map(|rule| {
            Diagnostic::new(
                RuleId::from("no_empty_rules"),
                Severity::Warn,
                "Empty rule block detected",
                rule.span,
                semantic.file_id,
            )
        })
        .collect()
}
