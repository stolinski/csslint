#![forbid(unsafe_code)]

use csslint_core::{FileId, Scope, Span};
use csslint_parser::ParsedStyle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticStyle {
    pub file_id: FileId,
    pub span: Span,
    pub scope: Scope,
    pub content: String,
    pub declaration_count: usize,
}

pub fn build_semantic_model(parsed: &ParsedStyle) -> SemanticStyle {
    let declaration_count = parsed.content.matches(':').count();

    SemanticStyle {
        file_id: parsed.file_id,
        span: parsed.span,
        scope: parsed.scope,
        content: parsed.content.clone(),
        declaration_count,
    }
}
