#![forbid(unsafe_code)]

use std::path::Path;

use csslint_core::{FileId, Scope, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleSyntax {
    Css,
    Vue,
    Svelte,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedStyle {
    pub file_id: FileId,
    pub syntax: StyleSyntax,
    pub scope: Scope,
    pub span: Span,
    pub source: String,
}

pub fn extract_styles(file_id: FileId, file_path: &Path, source: &str) -> Vec<ExtractedStyle> {
    let syntax = match file_path.extension().and_then(|ext| ext.to_str()) {
        Some("vue") => StyleSyntax::Vue,
        Some("svelte") => StyleSyntax::Svelte,
        _ => StyleSyntax::Css,
    };

    let scope = match syntax {
        StyleSyntax::Css => Scope::Global,
        StyleSyntax::Vue => Scope::VueScoped,
        StyleSyntax::Svelte => Scope::SvelteScoped,
    };

    vec![ExtractedStyle {
        file_id,
        syntax,
        scope,
        span: Span::new(0, source.len()),
        source: source.to_string(),
    }]
}
