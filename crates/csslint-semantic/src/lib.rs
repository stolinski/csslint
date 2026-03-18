#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use csslint_core::{map_local_span_to_global, FileId, Scope, Span};
use csslint_parser::ParsedStyle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleNodeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SelectorId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeclarationId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AtRuleId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssSemanticModel {
    pub file_id: FileId,
    pub span: Span,
    pub scope: Scope,
    pub source: String,
    pub rules: Vec<RuleNode>,
    pub selectors: Vec<SelectorNode>,
    pub declarations: Vec<DeclarationNode>,
    pub at_rules: Vec<AtRuleNode>,
    pub indexes: SemanticIndexes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleNode {
    pub id: RuleNodeId,
    pub selector_ids: Vec<SelectorId>,
    pub declaration_ids: Vec<DeclarationId>,
    pub span: Span,
    pub is_at_rule: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectorNode {
    pub id: SelectorId,
    pub rule_id: RuleNodeId,
    pub raw: String,
    pub normalized: String,
    pub parts: Vec<SelectorPart>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectorPart {
    pub value: String,
    pub kind: SelectorPartKind,
    pub scope: Scope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorPartKind {
    Class,
    Id,
    Tag,
    Pseudo,
    Attribute,
    Combinator,
    Universal,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclarationNode {
    pub id: DeclarationId,
    pub rule_id: RuleNodeId,
    pub property: String,
    pub property_known: bool,
    pub value: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtRuleNode {
    pub id: AtRuleId,
    pub name: String,
    pub prelude: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SemanticIndexes {
    pub selectors_by_class: BTreeMap<String, Vec<SelectorId>>,
    pub declarations_by_prop: BTreeMap<String, Vec<DeclarationId>>,
    pub declarations_by_rule: BTreeMap<RuleNodeId, Vec<DeclarationId>>,
    pub selectors_by_scope: BTreeMap<Scope, Vec<SelectorId>>,
    pub selectors_by_normalized: BTreeMap<String, Vec<SelectorId>>,
}

pub fn build_semantic_model(parsed: &ParsedStyle) -> CssSemanticModel {
    let mut rules = Vec::new();
    let mut selectors = Vec::new();
    let mut declarations = Vec::new();
    let mut at_rules = Vec::new();
    let mut indexes = SemanticIndexes::default();

    let mut selector_id_counter = 0u32;
    let mut declaration_id_counter = 0u32;
    let mut at_rule_id_counter = 0u32;

    let rule_blocks = parse_rule_blocks(&parsed.content);
    for (rule_index, block) in rule_blocks.into_iter().enumerate() {
        let rule_id = RuleNodeId(rule_index as u32);
        let rule_span = map_local_span_to_global(parsed.span.start, block.span);
        let mut selector_ids = Vec::new();
        let mut declaration_ids = Vec::new();

        if block.selector.trim_start().starts_with('@') {
            let (name, prelude) = parse_at_rule_head(&block.selector);
            at_rules.push(AtRuleNode {
                id: AtRuleId(at_rule_id_counter),
                name,
                prelude,
                span: rule_span,
            });
            at_rule_id_counter += 1;
        } else {
            for raw_selector in split_selectors(&block.selector) {
                let selector_id = SelectorId(selector_id_counter);
                selector_id_counter += 1;

                let normalized = normalize_selector(&raw_selector);
                let parts = selector_parts(&raw_selector, parsed.scope);
                let node = SelectorNode {
                    id: selector_id,
                    rule_id,
                    raw: raw_selector,
                    normalized: normalized.clone(),
                    parts: parts.clone(),
                    span: map_local_span_to_global(parsed.span.start, block.selector_span),
                };

                for class_name in classes_in_selector(&node.normalized) {
                    indexes
                        .selectors_by_class
                        .entry(class_name)
                        .or_default()
                        .push(selector_id);
                }

                let selector_scopes = parts.iter().map(|part| part.scope).collect::<BTreeSet<_>>();
                for selector_scope in selector_scopes {
                    indexes
                        .selectors_by_scope
                        .entry(selector_scope)
                        .or_default()
                        .push(selector_id);
                }
                indexes
                    .selectors_by_normalized
                    .entry(normalized)
                    .or_default()
                    .push(selector_id);

                selectors.push(node);
                selector_ids.push(selector_id);
            }
        }

        for declaration in parse_declarations(&block.body, block.body_start) {
            let declaration_id = DeclarationId(declaration_id_counter);
            declaration_id_counter += 1;

            let global_span = map_local_span_to_global(parsed.span.start, declaration.span);
            let node = DeclarationNode {
                id: declaration_id,
                rule_id,
                property: declaration.property.clone(),
                property_known: csslint_parser::is_known_property_name(&declaration.property),
                value: declaration.value,
                span: global_span,
            };

            indexes
                .declarations_by_prop
                .entry(node.property.to_ascii_lowercase())
                .or_default()
                .push(declaration_id);
            indexes
                .declarations_by_rule
                .entry(rule_id)
                .or_default()
                .push(declaration_id);

            declarations.push(node);
            declaration_ids.push(declaration_id);
        }

        rules.push(RuleNode {
            id: rule_id,
            selector_ids,
            declaration_ids,
            span: rule_span,
            is_at_rule: block.selector.trim_start().starts_with('@'),
        });
    }

    CssSemanticModel {
        file_id: parsed.file_id,
        span: parsed.span,
        scope: parsed.scope,
        source: parsed.content.clone(),
        rules,
        selectors,
        declarations,
        at_rules,
        indexes,
    }
}

#[derive(Debug, Clone)]
struct RuleBlock {
    selector: String,
    body: String,
    span: Span,
    selector_span: Span,
    body_start: usize,
}

fn parse_rule_blocks(source: &str) -> Vec<RuleBlock> {
    let mut blocks = Vec::new();
    let bytes = source.as_bytes();
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        let Some(open_brace) = find_byte(bytes, b'{', cursor) else {
            break;
        };

        let Some(close_brace) = find_matching_brace(bytes, open_brace) else {
            break;
        };

        let selector_region = &source[cursor..open_brace];
        let (selector, selector_span) = trimmed_region(selector_region, cursor);
        let body_start = open_brace + 1;
        let body = source
            .get(body_start..close_brace)
            .unwrap_or("")
            .to_string();

        if !selector.is_empty() {
            blocks.push(RuleBlock {
                selector,
                body,
                span: Span::new(selector_span.start, close_brace + 1),
                selector_span,
                body_start,
            });
        }

        cursor = close_brace + 1;
    }

    blocks
}

fn find_byte(bytes: &[u8], byte: u8, start: usize) -> Option<usize> {
    bytes
        .iter()
        .enumerate()
        .skip(start)
        .find_map(|(index, current)| (*current == byte).then_some(index))
}

fn find_matching_brace(bytes: &[u8], open_index: usize) -> Option<usize> {
    let mut depth = 1usize;
    let mut quote: Option<u8> = None;
    let mut index = open_index + 1;

    while index < bytes.len() {
        let current = bytes[index];
        if let Some(active_quote) = quote {
            if current == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }

        if current == b'"' || current == b'\'' {
            quote = Some(current);
            index += 1;
            continue;
        }

        if current == b'{' {
            depth += 1;
        } else if current == b'}' {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }

        index += 1;
    }

    None
}

fn trimmed_region(input: &str, absolute_start: usize) -> (String, Span) {
    let bytes = input.as_bytes();
    let mut start = 0usize;
    let mut end = bytes.len();

    while start < end && bytes[start].is_ascii_whitespace() {
        start += 1;
    }
    while end > start && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }

    (
        input.get(start..end).unwrap_or("").to_string(),
        Span::new(absolute_start + start, absolute_start + end),
    )
}

fn split_selectors(selector_list: &str) -> Vec<String> {
    selector_list
        .split(',')
        .map(str::trim)
        .filter(|selector| !selector.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn normalize_selector(raw: &str) -> String {
    let mut normalized = String::new();
    let mut pending_space = false;
    let mut quote: Option<char> = None;
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;

    for current in raw.trim().chars() {
        if let Some(active_quote) = quote {
            normalized.push(current);
            if current == active_quote {
                quote = None;
            }
            continue;
        }

        match current {
            '"' | '\'' => {
                if pending_space && should_emit_space(&normalized, current) {
                    normalized.push(' ');
                }
                pending_space = false;
                quote = Some(current);
                normalized.push(current);
            }
            '[' => {
                if pending_space && should_emit_space(&normalized, current) {
                    normalized.push(' ');
                }
                pending_space = false;
                bracket_depth += 1;
                normalized.push(current);
            }
            ']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                normalized.push(current);
            }
            '(' => {
                paren_depth += 1;
                normalized.push(current);
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                normalized.push(current);
            }
            ',' => {
                while normalized.ends_with(' ') {
                    normalized.pop();
                }
                normalized.push(',');
                pending_space = true;
            }
            _ if current.is_ascii_whitespace() => {
                if bracket_depth == 0 && paren_depth == 0 {
                    pending_space = true;
                } else {
                    normalized.push(current);
                }
            }
            _ => {
                if pending_space && should_emit_space(&normalized, current) {
                    normalized.push(' ');
                }
                pending_space = false;
                normalized.push(current);
            }
        }
    }

    normalized
}

fn should_emit_space(current: &str, next: char) -> bool {
    let _ = next;
    if current.is_empty() {
        return false;
    }

    !current.ends_with(' ')
}

fn selector_parts(raw: &str, scope: Scope) -> Vec<SelectorPart> {
    let mut parts = Vec::new();
    for token in raw.split_whitespace().filter(|part| !part.is_empty()) {
        parts.extend(parts_for_token(token, scope));
    }

    parts
}

fn parts_for_token(token: &str, default_scope: Scope) -> Vec<SelectorPart> {
    let mut parts = Vec::new();
    let mut cursor = token;

    while let Some(global_start) = cursor.find(":global(") {
        let prefix = cursor.get(..global_start).unwrap_or("");
        if !prefix.is_empty() {
            parts.push(SelectorPart {
                value: prefix.to_string(),
                kind: selector_part_kind(prefix),
                scope: default_scope,
            });
        }

        let global_expr = cursor.get(global_start + 8..).unwrap_or("");
        let Some(global_end) = find_closing_paren(global_expr) else {
            parts.push(SelectorPart {
                value: cursor.to_string(),
                kind: selector_part_kind(cursor),
                scope: default_scope,
            });
            return parts;
        };

        let global_value = global_expr.get(0..global_end).unwrap_or("").trim();
        if !global_value.is_empty() {
            parts.push(SelectorPart {
                value: global_value.to_string(),
                kind: selector_part_kind(global_value),
                scope: Scope::Global,
            });
        }

        cursor = global_expr.get(global_end + 1..).unwrap_or("");
    }

    if !cursor.is_empty() {
        parts.push(SelectorPart {
            value: cursor.to_string(),
            kind: selector_part_kind(cursor),
            scope: default_scope,
        });
    }

    parts
}

fn find_closing_paren(input: &str) -> Option<usize> {
    let mut depth = 1usize;
    for (index, current) in input.char_indices() {
        if current == '(' {
            depth += 1;
        } else if current == ')' {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }

    None
}

fn selector_part_kind(part: &str) -> SelectorPartKind {
    if part == "*" {
        SelectorPartKind::Universal
    } else if matches!(part, ">" | "+" | "~") {
        SelectorPartKind::Combinator
    } else if part.starts_with('.') {
        SelectorPartKind::Class
    } else if part.starts_with('#') {
        SelectorPartKind::Id
    } else if part.starts_with(":") {
        SelectorPartKind::Pseudo
    } else if part.starts_with('[') {
        SelectorPartKind::Attribute
    } else if part
        .chars()
        .next()
        .map(|ch| ch.is_ascii_alphabetic())
        .unwrap_or(false)
    {
        SelectorPartKind::Tag
    } else {
        SelectorPartKind::Unknown
    }
}

fn classes_in_selector(selector: &str) -> Vec<String> {
    let bytes = selector.as_bytes();
    let mut classes = Vec::new();
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] == b'.' {
            let start = index + 1;
            let mut end = start;
            while end < bytes.len() {
                let current = bytes[end];
                if current.is_ascii_alphanumeric() || current == b'_' || current == b'-' {
                    end += 1;
                    continue;
                }
                break;
            }

            if end > start {
                classes.push(selector[start..end].to_string());
            }

            index = end;
            continue;
        }

        index += 1;
    }

    classes
}

#[derive(Debug, Clone)]
struct ParsedDeclaration {
    property: String,
    value: String,
    span: Span,
}

fn parse_declarations(body: &str, body_start: usize) -> Vec<ParsedDeclaration> {
    let bytes = body.as_bytes();
    let mut declarations = Vec::new();
    let mut segment_start = 0usize;

    for (index, current) in bytes.iter().copied().enumerate() {
        if current != b';' {
            continue;
        }

        push_declaration_segment(
            body,
            body_start,
            segment_start,
            index + 1,
            &mut declarations,
        );
        segment_start = index + 1;
    }

    if segment_start < bytes.len() {
        push_declaration_segment(
            body,
            body_start,
            segment_start,
            bytes.len(),
            &mut declarations,
        );
    }

    declarations
}

fn push_declaration_segment(
    body: &str,
    body_start: usize,
    segment_start: usize,
    segment_end: usize,
    target: &mut Vec<ParsedDeclaration>,
) {
    let segment = body.get(segment_start..segment_end).unwrap_or("");
    let Some(colon_offset) = segment.find(':') else {
        return;
    };

    let property = segment
        .get(0..colon_offset)
        .unwrap_or("")
        .trim()
        .to_string();
    if property.is_empty() {
        return;
    }

    let value = segment
        .get(colon_offset + 1..)
        .unwrap_or("")
        .trim()
        .trim_end_matches(';')
        .trim()
        .to_string();

    let absolute_start = body_start + segment_start;
    let absolute_end = body_start + segment_end;
    target.push(ParsedDeclaration {
        property,
        value,
        span: Span::new(absolute_start, absolute_end),
    });
}

fn parse_at_rule_head(selector: &str) -> (String, String) {
    let trimmed = selector.trim();
    let without_at = trimmed.strip_prefix('@').unwrap_or(trimmed);
    let mut parts = without_at.split_whitespace();
    let name = parts.next().unwrap_or("").to_string();
    let prelude = parts.collect::<Vec<_>>().join(" ");
    (name, prelude)
}

#[cfg(test)]
mod tests {
    use csslint_core::{FileId, Scope, Span};
    use csslint_parser::ParsedStyle;

    use crate::build_semantic_model;

    #[test]
    fn builds_semantic_nodes_and_indexes() {
        let parsed = ParsedStyle {
            content: ".a, .b { color: red; margin: 0; }".to_string(),
            span: Span::new(10, 44),
            file_id: FileId::new(2),
            scope: Scope::Global,
            parsed_with_lightning: true,
        };

        let semantic = build_semantic_model(&parsed);
        assert_eq!(semantic.rules.len(), 1);
        assert_eq!(semantic.selectors.len(), 2);
        assert_eq!(semantic.declarations.len(), 2);
        assert_eq!(semantic.indexes.declarations_by_prop["color"].len(), 1);
        assert_eq!(semantic.indexes.selectors_by_class["a"].len(), 1);
        assert_eq!(semantic.indexes.selectors_by_scope[&Scope::Global].len(), 2);
    }

    #[test]
    fn tracks_empty_declaration_rules() {
        let parsed = ParsedStyle {
            content: ".empty {}".to_string(),
            span: Span::new(0, 9),
            file_id: FileId::new(3),
            scope: Scope::Global,
            parsed_with_lightning: true,
        };

        let semantic = build_semantic_model(&parsed);
        assert_eq!(semantic.rules.len(), 1);
        assert!(semantic.rules[0].declaration_ids.is_empty());
    }

    #[test]
    fn normalizes_selector_whitespace_conservatively() {
        assert_eq!(super::normalize_selector("  .foo   .bar   "), ".foo .bar");
        assert_eq!(super::normalize_selector(".foo   >   .bar"), ".foo > .bar");
    }

    #[test]
    fn keeps_attribute_and_quote_spacing_intact() {
        assert_eq!(
            super::normalize_selector("[data-title=\"hello   world\"]   .x"),
            "[data-title=\"hello   world\"] .x"
        );
        assert_eq!(
            super::normalize_selector(".icon\\+name   +   a"),
            ".icon\\+name + a"
        );
    }

    #[test]
    fn applies_scope_defaults_and_global_overrides() {
        let parsed = ParsedStyle {
            content: ".foo :global(.bar) .baz { color: red; }".to_string(),
            span: Span::new(0, 38),
            file_id: FileId::new(4),
            scope: Scope::VueScoped,
            parsed_with_lightning: true,
        };

        let semantic = build_semantic_model(&parsed);
        let selector = &semantic.selectors[0];
        let scopes = selector
            .parts
            .iter()
            .map(|part| part.scope)
            .collect::<Vec<_>>();
        assert_eq!(
            scopes,
            vec![Scope::VueScoped, Scope::Global, Scope::VueScoped]
        );
        assert_eq!(
            semantic.indexes.selectors_by_scope[&Scope::VueScoped].len(),
            1
        );
        assert_eq!(semantic.indexes.selectors_by_scope[&Scope::Global].len(), 1);
    }
}
