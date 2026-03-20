#![forbid(unsafe_code)]

use css_dataset::PROPERTIES as KNOWN_CSS_PROPERTIES;
use csslint_core::{Diagnostic, FileId, LineIndex, RuleId, Scope, Severity, Span, TargetProfile};
use csslint_extractor::ExtractedStyle;
#[cfg(feature = "lightning")]
use lightningcss::printer::{Printer, PrinterOptions};
#[cfg(feature = "lightning")]
use lightningcss::properties::PropertyId;
#[cfg(feature = "lightning")]
use lightningcss::rules::{style::StyleRule, CssRule, CssRuleList, Location};
#[cfg(feature = "lightning")]
use lightningcss::stylesheet::{ParserOptions, StyleSheet};
#[cfg(feature = "lightning")]
use lightningcss::traits::ToCss;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CssParserOptions {
    pub enable_recovery: bool,
    pub targets: TargetProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedStyle {
    pub content: String,
    pub span: Span,
    pub file_id: FileId,
    pub scope: Scope,
    pub parsed_with_lightning: bool,
    pub rules: Vec<ParsedRule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedRule {
    Style(ParsedStyleRule),
    AtRule(ParsedAtRule),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedStyleRule {
    pub span: Span,
    pub selector_span: Span,
    pub selectors: Vec<String>,
    pub ancestor_selector_context: String,
    pub at_rule_context: String,
    pub declarations: Vec<ParsedDeclaration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAtRule {
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDeclaration {
    pub property: String,
    pub value: String,
    pub span: Span,
}

pub fn parse_style(style: &ExtractedStyle) -> Result<ParsedStyle, Box<Diagnostic>> {
    parse_style_with_options(style, CssParserOptions::default())
}

pub fn parse_style_with_options(
    style: &ExtractedStyle,
    options: CssParserOptions,
) -> Result<ParsedStyle, Box<Diagnostic>> {
    let parsed_rules = parse_with_lightning(style, options)?;

    Ok(ParsedStyle {
        content: style.content.clone(),
        span: style.span(),
        file_id: style.file_id,
        scope: style.scope,
        parsed_with_lightning: cfg!(feature = "lightning"),
        rules: parsed_rules,
    })
}

pub fn is_known_property_name(property: &str) -> bool {
    let canonical = property.trim();
    if canonical.is_empty() || canonical.starts_with("--") {
        return true;
    }

    if canonical.starts_with('-') {
        return true;
    }

    let lower = canonical.to_ascii_lowercase();
    if KNOWN_CSS_PROPERTIES.contains(&lower.as_str()) {
        return true;
    }

    #[cfg(feature = "lightning")]
    {
        !matches!(PropertyId::from(canonical), PropertyId::Custom(_))
    }

    #[cfg(not(feature = "lightning"))]
    {
        fallback_known_property_name(canonical)
    }
}

#[cfg(not(feature = "lightning"))]
fn fallback_known_property_name(property: &str) -> bool {
    let lower = property.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "all"
            | "animation"
            | "background"
            | "border"
            | "color"
            | "display"
            | "flex"
            | "font"
            | "gap"
            | "grid"
            | "height"
            | "inset"
            | "margin"
            | "opacity"
            | "outline"
            | "padding"
            | "position"
            | "text"
            | "transform"
            | "transition"
            | "width"
    )
}

#[cfg(feature = "lightning")]
fn parse_with_lightning(
    style: &ExtractedStyle,
    options: CssParserOptions,
) -> Result<Vec<ParsedRule>, Box<Diagnostic>> {
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

    let stylesheet = StyleSheet::parse(&style.content, parser_options).map_err(|error| {
        Box::new(Diagnostic::new(
            RuleId::from("parser_syntax_error"),
            Severity::Error,
            error.to_string(),
            style.span(),
            style.file_id,
        ))
    })?;

    let line_index = LineIndex::new(&style.content);
    let mut parsed_rules = Vec::new();
    collect_rules_from_list(
        &style.content,
        line_index.line_starts(),
        &stylesheet.rules,
        "",
        "",
        &mut parsed_rules,
    );

    Ok(parsed_rules)
}

#[cfg(not(feature = "lightning"))]
fn parse_with_lightning(
    style: &ExtractedStyle,
    _options: CssParserOptions,
) -> Result<Vec<ParsedRule>, Box<Diagnostic>> {
    if style.content.contains('{') && !style.content.contains('}') {
        return Err(Box::new(Diagnostic::new(
            RuleId::from("parser_syntax_error"),
            Severity::Error,
            "Missing closing brace in style block",
            style.span(),
            style.file_id,
        )));
    }

    Ok(Vec::new())
}

#[cfg(feature = "lightning")]
fn collect_rules_from_list<R: Clone>(
    source: &str,
    line_starts: &[usize],
    rules: &CssRuleList<'_, R>,
    ancestor_selector_context: &str,
    at_rule_context: &str,
    target: &mut Vec<ParsedRule>,
) {
    for rule in &rules.0 {
        match rule {
            CssRule::Style(style_rule) => {
                let parsed_style_rule = build_parsed_style_rule(
                    source,
                    line_starts,
                    style_rule,
                    ancestor_selector_context,
                    at_rule_context,
                );
                let child_selector_context = append_context(
                    ancestor_selector_context,
                    &selector_list_context_key(&parsed_style_rule.selectors),
                );
                target.push(ParsedRule::Style(parsed_style_rule));
                collect_rules_from_list(
                    source,
                    line_starts,
                    &style_rule.rules,
                    &child_selector_context,
                    at_rule_context,
                    target,
                );
            }
            CssRule::Media(media_rule) => {
                push_at_rule(source, line_starts, media_rule.loc, target);
                let child_at_context = append_context(
                    at_rule_context,
                    &at_rule_context_fragment("media", source, line_starts, media_rule.loc),
                );
                collect_rules_from_list(
                    source,
                    line_starts,
                    &media_rule.rules,
                    ancestor_selector_context,
                    &child_at_context,
                    target,
                );
            }
            CssRule::Supports(supports_rule) => {
                push_at_rule(source, line_starts, supports_rule.loc, target);
                let child_at_context = append_context(
                    at_rule_context,
                    &at_rule_context_fragment("supports", source, line_starts, supports_rule.loc),
                );
                collect_rules_from_list(
                    source,
                    line_starts,
                    &supports_rule.rules,
                    ancestor_selector_context,
                    &child_at_context,
                    target,
                );
            }
            CssRule::MozDocument(document_rule) => {
                push_at_rule(source, line_starts, document_rule.loc, target);
                let child_at_context = append_context(
                    at_rule_context,
                    &at_rule_context_fragment("document", source, line_starts, document_rule.loc),
                );
                collect_rules_from_list(
                    source,
                    line_starts,
                    &document_rule.rules,
                    ancestor_selector_context,
                    &child_at_context,
                    target,
                );
            }
            CssRule::LayerBlock(layer_rule) => {
                push_at_rule(source, line_starts, layer_rule.loc, target);
                let child_at_context = append_context(
                    at_rule_context,
                    &at_rule_context_fragment("layer", source, line_starts, layer_rule.loc),
                );
                collect_rules_from_list(
                    source,
                    line_starts,
                    &layer_rule.rules,
                    ancestor_selector_context,
                    &child_at_context,
                    target,
                );
            }
            CssRule::Container(container_rule) => {
                push_at_rule(source, line_starts, container_rule.loc, target);
                let child_at_context = append_context(
                    at_rule_context,
                    &at_rule_context_fragment("container", source, line_starts, container_rule.loc),
                );
                collect_rules_from_list(
                    source,
                    line_starts,
                    &container_rule.rules,
                    ancestor_selector_context,
                    &child_at_context,
                    target,
                );
            }
            CssRule::Scope(scope_rule) => {
                push_at_rule(source, line_starts, scope_rule.loc, target);
                let child_at_context = append_context(
                    at_rule_context,
                    &at_rule_context_fragment("scope", source, line_starts, scope_rule.loc),
                );
                collect_rules_from_list(
                    source,
                    line_starts,
                    &scope_rule.rules,
                    ancestor_selector_context,
                    &child_at_context,
                    target,
                );
            }
            CssRule::StartingStyle(starting_rule) => {
                push_at_rule(source, line_starts, starting_rule.loc, target);
                let child_at_context = append_context(
                    at_rule_context,
                    &at_rule_context_fragment(
                        "starting-style",
                        source,
                        line_starts,
                        starting_rule.loc,
                    ),
                );
                collect_rules_from_list(
                    source,
                    line_starts,
                    &starting_rule.rules,
                    ancestor_selector_context,
                    &child_at_context,
                    target,
                );
            }
            CssRule::Nesting(nesting_rule) => {
                push_at_rule(source, line_starts, nesting_rule.loc, target);
                let nested_at_context = append_context(
                    at_rule_context,
                    &at_rule_context_fragment("nesting", source, line_starts, nesting_rule.loc),
                );
                let parsed_style_rule = build_parsed_style_rule(
                    source,
                    line_starts,
                    &nesting_rule.style,
                    ancestor_selector_context,
                    &nested_at_context,
                );
                let child_selector_context = append_context(
                    ancestor_selector_context,
                    &selector_list_context_key(&parsed_style_rule.selectors),
                );
                target.push(ParsedRule::Style(parsed_style_rule));
                collect_rules_from_list(
                    source,
                    line_starts,
                    &nesting_rule.style.rules,
                    &child_selector_context,
                    &nested_at_context,
                    target,
                );
            }
            CssRule::Import(import_rule) => {
                push_at_rule(source, line_starts, import_rule.loc, target);
            }
            CssRule::Keyframes(keyframes_rule) => {
                push_at_rule(source, line_starts, keyframes_rule.loc, target);
            }
            CssRule::FontFace(font_face_rule) => {
                push_at_rule(source, line_starts, font_face_rule.loc, target);
            }
            CssRule::FontPaletteValues(font_palette_rule) => {
                push_at_rule(source, line_starts, font_palette_rule.loc, target);
            }
            CssRule::FontFeatureValues(font_feature_rule) => {
                push_at_rule(source, line_starts, font_feature_rule.loc, target);
            }
            CssRule::Page(page_rule) => {
                push_at_rule(source, line_starts, page_rule.loc, target);
            }
            CssRule::CounterStyle(counter_style_rule) => {
                push_at_rule(source, line_starts, counter_style_rule.loc, target);
            }
            CssRule::Namespace(namespace_rule) => {
                push_at_rule(source, line_starts, namespace_rule.loc, target);
            }
            CssRule::Viewport(viewport_rule) => {
                push_at_rule(source, line_starts, viewport_rule.loc, target);
            }
            CssRule::CustomMedia(custom_media_rule) => {
                push_at_rule(source, line_starts, custom_media_rule.loc, target);
            }
            CssRule::LayerStatement(layer_statement_rule) => {
                push_at_rule(source, line_starts, layer_statement_rule.loc, target);
            }
            CssRule::Property(property_rule) => {
                push_at_rule(source, line_starts, property_rule.loc, target);
            }
            CssRule::ViewTransition(view_transition_rule) => {
                push_at_rule(source, line_starts, view_transition_rule.loc, target);
            }
            CssRule::Unknown(unknown_rule) => {
                push_at_rule(source, line_starts, unknown_rule.loc, target);
            }
            CssRule::NestedDeclarations(_) | CssRule::Custom(_) | CssRule::Ignored => {}
        }
    }
}

#[cfg(feature = "lightning")]
fn at_rule_context_fragment(
    name: &str,
    source: &str,
    line_starts: &[usize],
    loc: Location,
) -> String {
    let offset = source_location_to_offset(source, line_starts, loc.line, loc.column);
    format!("{name}@{offset}")
}

fn append_context(base: &str, fragment: &str) -> String {
    if base.is_empty() {
        return fragment.to_string();
    }

    if fragment.is_empty() {
        return base.to_string();
    }

    format!("{base}|{fragment}")
}

fn selector_list_context_key(selectors: &[String]) -> String {
    let mut items = selectors
        .iter()
        .map(|selector| selector.trim().to_string())
        .filter(|selector| !selector.is_empty())
        .collect::<Vec<_>>();
    items.sort();
    items.join(",")
}

#[cfg(feature = "lightning")]
fn push_at_rule(source: &str, line_starts: &[usize], loc: Location, target: &mut Vec<ParsedRule>) {
    let start = source_location_to_offset(source, line_starts, loc.line, loc.column);
    if start >= source.len() {
        return;
    }

    let (at_start, end) = at_rule_head_span(source, start);
    if end <= at_start {
        return;
    }

    target.push(ParsedRule::AtRule(ParsedAtRule {
        span: Span::new(at_start, end),
    }));
}

#[cfg(feature = "lightning")]
fn at_rule_head_span(source: &str, start: usize) -> (usize, usize) {
    let bytes = source.as_bytes();
    let mut at_start = start;
    while at_start < bytes.len() && bytes[at_start].is_ascii_whitespace() {
        at_start += 1;
    }

    if bytes.get(at_start) != Some(&b'@') {
        return (start, start.saturating_add(1).min(bytes.len()));
    }

    let mut end = at_start + 1;
    while end < bytes.len() {
        let byte = bytes[end];
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_') {
            end += 1;
            continue;
        }
        break;
    }

    (at_start, end)
}

#[cfg(feature = "lightning")]
fn build_parsed_style_rule<R: Clone>(
    source: &str,
    line_starts: &[usize],
    style_rule: &StyleRule<'_, R>,
    ancestor_selector_context: &str,
    at_rule_context: &str,
) -> ParsedStyleRule {
    if source.is_empty() {
        return ParsedStyleRule {
            span: Span::new(0, 0),
            selector_span: Span::new(0, 0),
            selectors: Vec::new(),
            ancestor_selector_context: ancestor_selector_context.to_string(),
            at_rule_context: at_rule_context.to_string(),
            declarations: Vec::new(),
        };
    }

    let selector_start = source_location_to_offset(
        source,
        line_starts,
        style_rule.loc.line,
        style_rule.loc.column,
    );
    let open_brace = find_next_open_brace(source, selector_start).unwrap_or(selector_start);
    let close_brace = find_matching_brace(source, open_brace).unwrap_or(open_brace);
    let selector_span = trimmed_span(source, selector_start, open_brace);

    let mut selectors = style_rule
        .selectors
        .0
        .iter()
        .filter_map(to_css_string)
        .collect::<Vec<_>>();
    if selectors.is_empty() {
        let raw_selector_list = source
            .get(selector_span.start..selector_span.end)
            .unwrap_or("");
        selectors = split_selector_list(raw_selector_list);
    }

    let body_start = open_brace.saturating_add(1).min(source.len());
    let body_end = close_brace.min(source.len());
    let declarations = parse_top_level_declarations(source, body_start, body_end);

    ParsedStyleRule {
        span: Span::new(
            selector_span.start,
            close_brace.saturating_add(1).min(source.len()),
        ),
        selector_span,
        selectors,
        ancestor_selector_context: ancestor_selector_context.to_string(),
        at_rule_context: at_rule_context.to_string(),
        declarations,
    }
}

#[cfg(feature = "lightning")]
fn to_css_string<T: ToCss>(node: &T) -> Option<String> {
    let mut output = String::new();
    let mut printer = Printer::new(&mut output, PrinterOptions::default());
    node.to_css(&mut printer).ok()?;
    Some(output)
}

fn parse_top_level_declarations(source: &str, start: usize, end: usize) -> Vec<ParsedDeclaration> {
    let bytes = source.as_bytes();
    if start >= end || end > bytes.len() {
        return Vec::new();
    }

    let mut declarations = Vec::new();
    let mut segment_start = start;
    let mut index = start;
    let mut quote: Option<u8> = None;
    let mut in_comment = false;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;

    while index < end {
        let current = bytes[index];

        if in_comment {
            if current == b'*' && bytes.get(index + 1) == Some(&b'/') {
                in_comment = false;
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }

        if let Some(active_quote) = quote {
            if current == b'\\' {
                index = index.saturating_add(2);
                continue;
            }

            if current == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }

        if current == b'/' && bytes.get(index + 1) == Some(&b'*') {
            in_comment = true;
            index += 2;
            continue;
        }

        if current == b'\'' || current == b'"' {
            quote = Some(current);
            index += 1;
            continue;
        }

        match current {
            b'(' => paren_depth += 1,
            b')' => paren_depth = paren_depth.saturating_sub(1),
            b'[' => bracket_depth += 1,
            b']' => bracket_depth = bracket_depth.saturating_sub(1),
            b'{' => {
                if paren_depth == 0 && bracket_depth == 0 {
                    brace_depth += 1;
                }
            }
            b'}' => {
                if paren_depth == 0 && bracket_depth == 0 {
                    brace_depth = brace_depth.saturating_sub(1);
                }
            }
            b';' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                push_declaration_segment(source, segment_start, index + 1, &mut declarations);
                segment_start = index + 1;
            }
            _ => {}
        }

        index += 1;
    }

    if segment_start < end {
        push_declaration_segment(source, segment_start, end, &mut declarations);
    }

    declarations
}

fn push_declaration_segment(
    source: &str,
    segment_start: usize,
    segment_end: usize,
    target: &mut Vec<ParsedDeclaration>,
) {
    if segment_start >= segment_end {
        return;
    }

    let segment = source.get(segment_start..segment_end).unwrap_or("");
    let Some(colon_offset) = find_declaration_colon(segment) else {
        return;
    };

    let property = segment.get(..colon_offset).unwrap_or("").trim().to_string();
    if property.is_empty() || !is_plausible_property_name(&property) {
        return;
    }

    let value = segment
        .get(colon_offset + 1..)
        .unwrap_or("")
        .trim()
        .trim_end_matches(';')
        .trim()
        .to_string();

    target.push(ParsedDeclaration {
        property,
        value,
        span: Span::new(segment_start, segment_end),
    });
}

fn find_declaration_colon(segment: &str) -> Option<usize> {
    let bytes = segment.as_bytes();
    let mut index = 0usize;
    let mut quote: Option<u8> = None;
    let mut in_comment = false;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;

    while index < bytes.len() {
        let current = bytes[index];

        if in_comment {
            if current == b'*' && bytes.get(index + 1) == Some(&b'/') {
                in_comment = false;
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }

        if let Some(active_quote) = quote {
            if current == b'\\' {
                index = index.saturating_add(2);
                continue;
            }

            if current == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }

        if current == b'/' && bytes.get(index + 1) == Some(&b'*') {
            in_comment = true;
            index += 2;
            continue;
        }

        if current == b'\'' || current == b'"' {
            quote = Some(current);
            index += 1;
            continue;
        }

        match current {
            b'(' => paren_depth += 1,
            b')' => paren_depth = paren_depth.saturating_sub(1),
            b'[' => bracket_depth += 1,
            b']' => bracket_depth = bracket_depth.saturating_sub(1),
            b':' if paren_depth == 0 && bracket_depth == 0 => return Some(index),
            _ => {}
        }

        index += 1;
    }

    None
}

fn is_plausible_property_name(property: &str) -> bool {
    let trimmed = property.trim();
    if trimmed.is_empty() {
        return false;
    }

    if let Some(custom_name) = trimmed.strip_prefix("--") {
        return !custom_name.is_empty()
            && custom_name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'));
    }

    if trimmed.bytes().any(|byte| {
        matches!(
            byte,
            b'{' | b'}' | b':' | b';' | b'\'' | b'"' | b'(' | b')' | b',' | b'.' | b'#'
        )
    }) {
        return false;
    }

    let bytes = trimmed.as_bytes();
    let mut cursor = 0usize;
    if bytes[cursor] == b'-' {
        cursor += 1;
    }

    if cursor >= bytes.len() {
        return false;
    }

    let first = bytes[cursor];
    if !first.is_ascii_alphabetic() && first != b'_' {
        return false;
    }

    bytes[cursor + 1..]
        .iter()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b'-'))
}

fn split_selector_list(selector_list: &str) -> Vec<String> {
    let mut selectors = Vec::new();
    let mut segment_start = 0usize;
    let mut quote: Option<char> = None;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;

    for (index, current) in selector_list.char_indices() {
        if let Some(active_quote) = quote {
            if current == active_quote {
                quote = None;
            }
            continue;
        }

        match current {
            '"' | '\'' => quote = Some(current),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            ',' if paren_depth == 0 && bracket_depth == 0 => {
                if let Some(segment) = selector_list.get(segment_start..index) {
                    let trimmed = segment.trim();
                    if !trimmed.is_empty() {
                        selectors.push(trimmed.to_string());
                    }
                }
                segment_start = index + 1;
            }
            _ => {}
        }
    }

    if let Some(segment) = selector_list.get(segment_start..) {
        let trimmed = segment.trim();
        if !trimmed.is_empty() {
            selectors.push(trimmed.to_string());
        }
    }

    selectors
}

fn trimmed_span(source: &str, start: usize, end: usize) -> Span {
    let bytes = source.as_bytes();
    let safe_start = start.min(bytes.len());
    let safe_end = end.min(bytes.len()).max(safe_start);
    let mut local_start = safe_start;
    let mut local_end = safe_end;

    while local_start < local_end && bytes[local_start].is_ascii_whitespace() {
        local_start += 1;
    }
    while local_end > local_start && bytes[local_end - 1].is_ascii_whitespace() {
        local_end -= 1;
    }

    Span::new(local_start, local_end)
}

fn find_next_open_brace(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut index = start.min(bytes.len());
    let mut quote: Option<u8> = None;
    let mut in_comment = false;

    while index < bytes.len() {
        let current = bytes[index];

        if in_comment {
            if current == b'*' && bytes.get(index + 1) == Some(&b'/') {
                in_comment = false;
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }

        if let Some(active_quote) = quote {
            if current == b'\\' {
                index = index.saturating_add(2);
                continue;
            }
            if current == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }

        if current == b'/' && bytes.get(index + 1) == Some(&b'*') {
            in_comment = true;
            index += 2;
            continue;
        }

        if current == b'\'' || current == b'"' {
            quote = Some(current);
            index += 1;
            continue;
        }

        if current == b'{' {
            return Some(index);
        }

        index += 1;
    }

    None
}

fn find_matching_brace(source: &str, open_index: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    if open_index >= bytes.len() || bytes[open_index] != b'{' {
        return None;
    }

    let mut depth = 1usize;
    let mut index = open_index + 1;
    let mut quote: Option<u8> = None;
    let mut in_comment = false;

    while index < bytes.len() {
        let current = bytes[index];

        if in_comment {
            if current == b'*' && bytes.get(index + 1) == Some(&b'/') {
                in_comment = false;
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }

        if let Some(active_quote) = quote {
            if current == b'\\' {
                index = index.saturating_add(2);
                continue;
            }
            if current == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }

        if current == b'/' && bytes.get(index + 1) == Some(&b'*') {
            in_comment = true;
            index += 2;
            continue;
        }

        if current == b'\'' || current == b'"' {
            quote = Some(current);
            index += 1;
            continue;
        }

        match current {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }

        index += 1;
    }

    None
}

#[cfg(feature = "lightning")]
fn source_location_to_offset(source: &str, line_starts: &[usize], line: u32, column: u32) -> usize {
    if source.is_empty() {
        return 0;
    }

    let max_line_index = line_starts.len().saturating_sub(1);
    let line_index = (line as usize).min(max_line_index);
    let line_start = line_starts[line_index];
    let line_end = line_starts
        .get(line_index + 1)
        .copied()
        .unwrap_or(source.len())
        .min(source.len());

    let target_units = column.saturating_sub(1) as usize;
    if target_units == 0 {
        return line_start;
    }

    let mut consumed_units = 0usize;
    let line_slice = source.get(line_start..line_end).unwrap_or("");
    for (index, current) in line_slice.char_indices() {
        consumed_units += current.len_utf16();
        let offset = line_start + index + current.len_utf8();
        if consumed_units >= target_units {
            return offset;
        }
    }

    line_end
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

    use crate::{is_known_property_name, parse_style, ParsedRule};

    #[test]
    fn parser_accepts_valid_css() {
        let extraction = csslint_extractor::extract_styles(
            FileId::new(1),
            Path::new("valid.css"),
            ".box { color: red; }",
        );
        let parsed = parse_style(&extraction.styles[0]).expect("valid css should parse");

        assert!(parsed.parsed_with_lightning);
        assert!(parsed
            .rules
            .iter()
            .any(|rule| matches!(rule, ParsedRule::Style(_))));
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

    #[test]
    fn parser_allows_braces_inside_string_literals() {
        let extraction = csslint_extractor::extract_styles(
            FileId::new(21),
            Path::new("strings.css"),
            ".box { content: \"}\"; color: red; }",
        );

        let parsed =
            parse_style(&extraction.styles[0]).expect("css with string braces should parse");
        assert!(parsed
            .rules
            .iter()
            .any(|rule| matches!(rule, ParsedRule::Style(_))));
    }

    #[test]
    fn recognizes_known_and_unknown_property_names() {
        assert!(is_known_property_name("color"));
        assert!(is_known_property_name("pointer-events"));
        assert!(is_known_property_name("outline-offset"));
        assert!(is_known_property_name("-webkit-line-clamp"));
        assert!(is_known_property_name("--brand-color"));
        assert!(!is_known_property_name("colr"));
    }

    #[test]
    fn collects_nested_style_rules_without_cross_rule_declarations() {
        let extraction = csslint_extractor::extract_styles(
            FileId::new(3),
            Path::new("nested.css"),
            "@media (max-width: 800px) {\n  .card { display: flex; background: var(--bg); }\n  .button { display: flex; background: var(--bg); }\n}",
        );
        let parsed = parse_style(&extraction.styles[0]).expect("nested css should parse");

        let style_rules = parsed
            .rules
            .iter()
            .filter_map(|rule| match rule {
                ParsedRule::Style(style_rule) => Some(style_rule),
                ParsedRule::AtRule(_) => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(style_rules.len(), 2);
        assert_eq!(style_rules[0].declarations.len(), 2);
        assert_eq!(style_rules[1].declarations.len(), 2);
        assert_eq!(style_rules[0].declarations[0].property, "display");
        assert_eq!(style_rules[0].declarations[1].property, "background");
        assert_eq!(style_rules[1].declarations[0].property, "display");
        assert_eq!(style_rules[1].declarations[1].property, "background");
    }

    #[test]
    fn excludes_property_rule_descriptors_from_style_declarations() {
        let extraction = csslint_extractor::extract_styles(
            FileId::new(4),
            Path::new("property.css"),
            "@property --button-bg {\n  syntax: \"<color>\";\n  inherits: false;\n  initial-value: transparent;\n}\n\n.demo { pointer-events: none; }",
        );
        let parsed = parse_style(&extraction.styles[0]).expect("@property css should parse");

        let style_rules = parsed
            .rules
            .iter()
            .filter_map(|rule| match rule {
                ParsedRule::Style(style_rule) => Some(style_rule),
                ParsedRule::AtRule(_) => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(style_rules.len(), 1);
        assert_eq!(style_rules[0].declarations.len(), 1);
        assert_eq!(style_rules[0].declarations[0].property, "pointer-events");
    }
}
