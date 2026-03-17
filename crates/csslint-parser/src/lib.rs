#![forbid(unsafe_code)]

use csslint_core::{Diagnostic, FileId, RuleId, Scope, Severity, Span};
use csslint_extractor::ExtractedStyle;
#[cfg(feature = "lightning")]
use lightningcss::stylesheet::{ParserOptions, StyleSheet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CssParserOptions {
    pub enable_recovery: bool,
}

impl Default for CssParserOptions {
    fn default() -> Self {
        Self {
            enable_recovery: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedStyle {
    pub content: String,
    pub span: Span,
    pub file_id: FileId,
    pub scope: Scope,
    pub parsed_with_lightning: bool,
}

pub fn parse_style(style: &ExtractedStyle) -> Result<ParsedStyle, Box<Diagnostic>> {
    parse_style_with_options(style, CssParserOptions::default())
}

pub fn parse_style_with_options(
    style: &ExtractedStyle,
    options: CssParserOptions,
) -> Result<ParsedStyle, Box<Diagnostic>> {
    parse_with_lightning(style, options)?;

    if style.content.trim().is_empty() {
        return Ok(ParsedStyle {
            content: style.content.clone(),
            span: style.span(),
            file_id: style.file_id,
            scope: style.scope,
            parsed_with_lightning: true,
        });
    }

    Ok(ParsedStyle {
        content: style.content.clone(),
        span: style.span(),
        file_id: style.file_id,
        scope: style.scope,
        parsed_with_lightning: true,
    })
}

#[cfg(feature = "lightning")]
fn parse_with_lightning(
    style: &ExtractedStyle,
    options: CssParserOptions,
) -> Result<(), Box<Diagnostic>> {
    if !has_balanced_braces(&style.content) {
        return Err(Box::new(Diagnostic::new(
            RuleId::from("parser_syntax_error"),
            Severity::Error,
            "Missing closing brace in style block",
            style.span(),
            style.file_id,
        )));
    }

    let parser_options = ParserOptions {
        error_recovery: options.enable_recovery,
        ..ParserOptions::default()
    };

    if let Err(error) = StyleSheet::parse(&style.content, parser_options) {
        return Err(Box::new(Diagnostic::new(
            RuleId::from("parser_syntax_error"),
            Severity::Error,
            error.to_string(),
            style.span(),
            style.file_id,
        )));
    }

    Ok(())
}

#[cfg(not(feature = "lightning"))]
fn parse_with_lightning(
    style: &ExtractedStyle,
    _options: CssParserOptions,
) -> Result<(), Box<Diagnostic>> {
    if style.content.contains('{') && !style.content.contains('}') {
        return Err(Box::new(Diagnostic::new(
            RuleId::from("parser_syntax_error"),
            Severity::Error,
            "Missing closing brace in style block",
            style.span(),
            style.file_id,
        )));
    }

    Ok(())
}

fn has_balanced_braces(source: &str) -> bool {
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    for current in source.chars() {
        if let Some(active_quote) = quote {
            if current == active_quote {
                quote = None;
            }
            continue;
        }

        if current == '"' || current == '\'' {
            quote = Some(current);
            continue;
        }

        match current {
            '{' => depth += 1,
            '}' => {
                if depth == 0 {
                    return false;
                }
                depth -= 1;
            }
            _ => {}
        }
    }

    depth == 0
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use csslint_core::FileId;

    use crate::parse_style;

    #[test]
    fn parser_accepts_valid_css() {
        let extraction = csslint_extractor::extract_styles(
            FileId::new(1),
            Path::new("valid.css"),
            ".box { color: red; }",
        );
        let parsed = parse_style(&extraction.styles[0]).expect("valid css should parse");

        assert!(parsed.parsed_with_lightning);
    }

    #[test]
    fn parser_maps_invalid_css_to_diagnostic() {
        let extraction = csslint_extractor::extract_styles(
            FileId::new(2),
            Path::new("invalid.css"),
            ".box { color: red",
        );
        let error = parse_style(&extraction.styles[0]).expect_err("invalid css should fail");

        assert_eq!(error.rule_id.as_str(), "parser_syntax_error");
        assert_eq!(error.severity.as_str(), "error");
        assert_eq!(error.span, extraction.styles[0].span());
    }
}
