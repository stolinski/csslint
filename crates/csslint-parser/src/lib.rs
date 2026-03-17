#![forbid(unsafe_code)]

use csslint_core::{Diagnostic, RuleId, Severity, Span};
use csslint_extractor::ExtractedStyle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedStyle {
    pub source: String,
    pub span: Span,
    pub file_id: csslint_core::FileId,
    pub scope: csslint_core::Scope,
}

pub fn parse_style(style: &ExtractedStyle) -> Result<ParsedStyle, Diagnostic> {
    if style.source.trim().is_empty() {
        return Err(Diagnostic::new(
            RuleId::from("parser_empty_input"),
            Severity::Error,
            "Style block is empty and cannot be parsed",
            style.span,
            style.file_id,
        ));
    }

    Ok(ParsedStyle {
        source: style.source.clone(),
        span: style.span,
        file_id: style.file_id,
        scope: style.scope,
    })
}
