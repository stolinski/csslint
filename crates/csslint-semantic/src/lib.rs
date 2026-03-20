#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use csslint_core::{map_local_span_to_global, FileId, Scope, Span};
use csslint_parser::{ParsedRule, ParsedStyle};

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
    pub selectors_by_duplicate_key: BTreeMap<String, Vec<SelectorId>>,
    pub duplicate_key_by_selector: BTreeMap<SelectorId, String>,
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

    for parsed_rule in &parsed.rules {
        let rule_id = RuleNodeId(rules.len() as u32);
        let mut selector_ids = Vec::new();
        let mut declaration_ids = Vec::new();

        match parsed_rule {
            ParsedRule::AtRule(at_rule) => {
                let global_span = map_local_span_to_global(parsed.span.start, at_rule.span);
                let local_segment = parsed
                    .content
                    .get(at_rule.span.start..at_rule.span.end)
                    .unwrap_or("")
                    .trim();
                let (name, prelude) = parse_at_rule_segment(local_segment);

                at_rules.push(AtRuleNode {
                    id: AtRuleId(at_rule_id_counter),
                    name,
                    prelude,
                    span: global_span,
                });
                at_rule_id_counter += 1;

                rules.push(RuleNode {
                    id: rule_id,
                    selector_ids,
                    declaration_ids,
                    span: global_span,
                    is_at_rule: true,
                });
            }
            ParsedRule::Style(style_rule) => {
                let rule_span = map_local_span_to_global(parsed.span.start, style_rule.span);
                let selector_span =
                    map_local_span_to_global(parsed.span.start, style_rule.selector_span);

                for raw_selector in &style_rule.selectors {
                    let selector_id = SelectorId(selector_id_counter);
                    selector_id_counter += 1;

                    let normalized = normalize_selector(raw_selector);
                    let parts = selector_parts(raw_selector, parsed.scope);
                    let node = SelectorNode {
                        id: selector_id,
                        rule_id,
                        raw: raw_selector.clone(),
                        normalized: normalized.clone(),
                        parts: parts.clone(),
                        span: selector_span,
                    };

                    for class_name in classes_in_selector(&node.normalized) {
                        indexes
                            .selectors_by_class
                            .entry(class_name)
                            .or_default()
                            .push(selector_id);
                    }

                    let selector_scopes =
                        parts.iter().map(|part| part.scope).collect::<BTreeSet<_>>();
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

                    let duplicate_key = selector_duplicate_key(
                        &style_rule.ancestor_selector_context,
                        &style_rule.at_rule_context,
                        &node.normalized,
                    );
                    indexes
                        .selectors_by_duplicate_key
                        .entry(duplicate_key.clone())
                        .or_default()
                        .push(selector_id);
                    indexes
                        .duplicate_key_by_selector
                        .insert(selector_id, duplicate_key);

                    selectors.push(node);
                    selector_ids.push(selector_id);
                }

                for declaration in &style_rule.declarations {
                    let declaration_id = DeclarationId(declaration_id_counter);
                    declaration_id_counter += 1;

                    let global_span = map_local_span_to_global(parsed.span.start, declaration.span);
                    let node = DeclarationNode {
                        id: declaration_id,
                        rule_id,
                        property: declaration.property.clone(),
                        property_known: csslint_parser::is_known_property_name(
                            &declaration.property,
                        ),
                        value: declaration.value.clone(),
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
                    is_at_rule: false,
                });
            }
        }
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

fn parse_at_rule_segment(segment: &str) -> (String, String) {
    let trimmed = segment.trim();
    let without_at = trimmed.strip_prefix('@').unwrap_or(trimmed);
    let mut parts = without_at.split_whitespace();
    let name = parts.next().unwrap_or("").to_string();
    let prelude = parts.collect::<Vec<_>>().join(" ");
    (name, prelude)
}

fn selector_duplicate_key(
    ancestor_selector_context: &str,
    at_rule_context: &str,
    normalized_selector: &str,
) -> String {
    format!("{at_rule_context}||{ancestor_selector_context}||{normalized_selector}")
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use csslint_core::{FileId, Scope};

    use crate::build_semantic_model;

    fn parse_style(file_id: u32, file_name: &str, source: &str) -> csslint_parser::ParsedStyle {
        let extraction =
            csslint_extractor::extract_styles(FileId::new(file_id), Path::new(file_name), source);
        csslint_parser::parse_style(&extraction.styles[0]).expect("fixture css should parse")
    }

    #[test]
    fn builds_semantic_nodes_and_indexes() {
        let parsed = parse_style(2, "fixture.css", ".a, .b { color: red; margin: 0; }");

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
        let parsed = parse_style(3, "empty.css", ".empty {}");

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
        let parsed = parse_style(
            4,
            "Scoped.vue",
            "<template></template><style scoped>.foo :global(.bar) .baz { color: red; }</style>",
        );

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

    #[test]
    fn keeps_nested_rule_declarations_isolated() {
        let parsed = parse_style(
            5,
            "nested.css",
            "@media (max-width: 800px) {\n  .card { display: flex; background: var(--bg); }\n  .button { display: flex; background: var(--bg); }\n}",
        );

        let semantic = build_semantic_model(&parsed);
        let declaration_counts = semantic
            .rules
            .iter()
            .filter(|rule| !rule.is_at_rule)
            .map(|rule| rule.declaration_ids.len())
            .collect::<Vec<_>>();

        assert_eq!(declaration_counts, vec![2, 2]);
    }

    #[test]
    fn duplicate_selector_keys_include_at_rule_context() {
        let parsed = parse_style(
            6,
            "contexts.css",
            ".card { color: red; } @media (min-width: 600px) { .card { color: blue; } }",
        );

        let semantic = build_semantic_model(&parsed);
        assert_eq!(semantic.selectors.len(), 2);
        assert_eq!(semantic.indexes.selectors_by_duplicate_key.len(), 2);
    }

    #[test]
    fn parses_at_rule_name_and_prelude() {
        let parsed = parse_style(
            7,
            "atrule.css",
            "@media (min-width: 600px) { .card { color: red; } }",
        );

        let semantic = build_semantic_model(&parsed);
        assert_eq!(semantic.at_rules.len(), 1);
        assert_eq!(semantic.at_rules[0].name, "media");
        assert!(
            semantic.at_rules[0].prelude.is_empty(),
            "v1 semantic at-rule nodes currently capture the at-keyword head only"
        );
    }

    #[test]
    fn scope_indexes_deduplicate_multiple_global_segments_per_selector() {
        let parsed = parse_style(
            8,
            "Scoped.vue",
            "<template></template><style scoped>.root :global(.a) :global(.b) .leaf { color: red; }</style>",
        );

        let semantic = build_semantic_model(&parsed);
        assert_eq!(semantic.selectors.len(), 1);
        assert_eq!(
            semantic.indexes.selectors_by_scope[&Scope::VueScoped].len(),
            1
        );
        assert_eq!(semantic.indexes.selectors_by_scope[&Scope::Global].len(), 1);
    }

    #[test]
    fn malformed_global_selector_segment_falls_back_to_default_scope() {
        let parts = super::parts_for_token(":global(.missing", Scope::VueScoped);

        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].scope, Scope::VueScoped);
        assert_eq!(parts[0].value, ":global(.missing");
    }
}
