#![forbid(unsafe_code)]

use csslint_core::{Diagnostic, FileId, RuleId, Scope, Severity, Span};
use csslint_extractor::ExtractedStyle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedStyle {
    pub content: String,
    pub span: Span,
    pub file_id: FileId,
    pub scope: Scope,
}

pub fn parse_style(style: &ExtractedStyle) -> Result<ParsedStyle, Box<Diagnostic>> {
    if style.content.trim().is_empty() {
        return Err(Box::new(Diagnostic::new(
            RuleId::from("parser_empty_input"),
            Severity::Error,
            "Style block is empty and cannot be parsed",
            style.span(),
            style.file_id,
        )));
    }

    Ok(ParsedStyle {
        content: style.content.clone(),
        span: style.span(),
        file_id: style.file_id,
        scope: style.scope,
    })
}
