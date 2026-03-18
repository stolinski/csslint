#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use csslint_core::{Diagnostic, FileId, RuleId, Severity, Span};
use csslint_semantic::{CssSemanticModel, DeclarationNode, RuleNode, SelectorNode};

pub type SelectorCallback = fn(&SelectorNode, &mut RuleRuntimeCtx);
pub type DeclarationCallback = fn(&DeclarationNode, &mut RuleRuntimeCtx);
pub type RuleNodeCallback = fn(&RuleNode, &mut RuleRuntimeCtx);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleMeta {
    pub id: RuleId,
    pub description: &'static str,
    pub default_severity: Severity,
    pub fixable: bool,
}

pub struct RuleVisitor {
    pub on_selector: Option<SelectorCallback>,
    pub on_declaration: Option<DeclarationCallback>,
    pub on_rule: Option<RuleNodeCallback>,
}

impl RuleVisitor {
    pub const fn empty() -> Self {
        Self {
            on_selector: None,
            on_declaration: None,
            on_rule: None,
        }
    }
}

#[derive(Clone, Copy)]
pub struct RuleContext<'a> {
    pub semantic: &'a CssSemanticModel,
    pub severity: Severity,
}

pub trait Rule: Send + Sync {
    fn meta(&self) -> RuleMeta;
    fn create(&self, ctx: RuleContext<'_>) -> RuleVisitor;
}

pub struct RuleRuntimeCtx {
    file_id: FileId,
    rule_id: RuleId,
    severity: Severity,
    diagnostics: Vec<Diagnostic>,
}

impl RuleRuntimeCtx {
    pub fn new(file_id: FileId, rule_id: RuleId, severity: Severity) -> Self {
        Self {
            file_id,
            rule_id,
            severity,
            diagnostics: Vec::new(),
        }
    }

    pub fn report(&mut self, message: impl Into<String>, span: Span) {
        if self.severity == Severity::Off {
            return;
        }

        self.diagnostics.push(Diagnostic::new(
            self.rule_id.clone(),
            self.severity,
            message,
            span,
            self.file_id,
        ));
    }

    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}

#[derive(Default)]
pub struct RuleRegistry {
    rules: BTreeMap<RuleId, Box<dyn Rule>>,
}

impl RuleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<R>(&mut self, rule: R) -> Result<(), String>
    where
        R: Rule + 'static,
    {
        let meta = rule.meta();
        if self.rules.contains_key(&meta.id) {
            return Err(format!("duplicate rule registration: {}", meta.id));
        }

        self.rules.insert(meta.id, Box::new(rule));
        Ok(())
    }

    pub fn ordered_rules(&self) -> Vec<&dyn Rule> {
        self.rules
            .values()
            .map(|rule| rule.as_ref() as &dyn Rule)
            .collect()
    }

    pub fn ordered_meta(&self) -> Vec<RuleMeta> {
        self.ordered_rules()
            .into_iter()
            .map(|rule| rule.meta())
            .collect()
    }
}

pub fn core_registry() -> RuleRegistry {
    let mut registry = RuleRegistry::new();
    let _ = registry.register(NoEmptyRules);
    registry
}

pub fn run_rules(semantic: &CssSemanticModel) -> Vec<Diagnostic> {
    let registry = core_registry();
    let mut diagnostics = Vec::new();

    for rule in registry.ordered_rules() {
        let meta = rule.meta();
        if meta.default_severity == Severity::Off {
            continue;
        }

        let visitor = rule.create(RuleContext {
            semantic,
            severity: meta.default_severity,
        });
        let mut runtime = RuleRuntimeCtx::new(semantic.file_id, meta.id, meta.default_severity);

        if let Some(on_rule) = visitor.on_rule {
            for node in &semantic.rules {
                on_rule(node, &mut runtime);
            }
        }

        if let Some(on_selector) = visitor.on_selector {
            for node in &semantic.selectors {
                on_selector(node, &mut runtime);
            }
        }

        if let Some(on_declaration) = visitor.on_declaration {
            for node in &semantic.declarations {
                on_declaration(node, &mut runtime);
            }
        }

        diagnostics.extend(runtime.into_diagnostics());
    }

    diagnostics
}

struct NoEmptyRules;

impl Rule for NoEmptyRules {
    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: RuleId::from("no_empty_rules"),
            description: "Disallow empty CSS rule blocks",
            default_severity: Severity::Warn,
            fixable: true,
        }
    }

    fn create(&self, _ctx: RuleContext<'_>) -> RuleVisitor {
        RuleVisitor {
            on_selector: None,
            on_declaration: None,
            on_rule: Some(no_empty_rules_on_rule),
        }
    }
}

fn no_empty_rules_on_rule(rule: &RuleNode, ctx: &mut RuleRuntimeCtx) {
    if rule.is_at_rule || !rule.declaration_ids.is_empty() {
        return;
    }

    ctx.report("Empty rule block detected", rule.span);
}

#[cfg(test)]
mod tests {
    use csslint_core::{FileId, Scope, Span};
    use csslint_semantic::{CssSemanticModel, RuleNode, RuleNodeId, SemanticIndexes};

    use super::{core_registry, run_rules, Rule, RuleContext, RuleMeta, RuleRegistry, RuleVisitor};

    struct MockRule {
        id: &'static str,
    }

    impl Rule for MockRule {
        fn meta(&self) -> RuleMeta {
            RuleMeta {
                id: self.id.into(),
                description: "mock",
                default_severity: csslint_core::Severity::Warn,
                fixable: false,
            }
        }

        fn create(&self, _ctx: RuleContext<'_>) -> RuleVisitor {
            RuleVisitor::empty()
        }
    }

    #[test]
    fn registry_orders_rules_by_id() {
        let mut registry = RuleRegistry::new();
        let _ = registry.register(MockRule { id: "z_last" });
        let _ = registry.register(MockRule { id: "a_first" });

        let metas = registry.ordered_meta();
        assert_eq!(metas[0].id.as_str(), "a_first");
        assert_eq!(metas[1].id.as_str(), "z_last");
    }

    #[test]
    fn core_registry_exposes_no_empty_metadata() {
        let metas = core_registry().ordered_meta();
        let no_empty = metas
            .iter()
            .find(|meta| meta.id.as_str() == "no_empty_rules")
            .expect("no_empty_rules metadata should exist");

        assert_eq!(no_empty.default_severity.as_str(), "warn");
        assert!(no_empty.fixable);
    }

    #[test]
    fn no_empty_rule_reports_empty_rule_nodes() {
        let semantic = CssSemanticModel {
            file_id: FileId::new(1),
            span: Span::new(0, 10),
            scope: Scope::Global,
            source: ".a {}".to_string(),
            rules: vec![RuleNode {
                id: RuleNodeId(0),
                selector_ids: vec![],
                declaration_ids: vec![],
                span: Span::new(0, 5),
                is_at_rule: false,
            }],
            selectors: vec![],
            declarations: vec![],
            at_rules: vec![],
            indexes: SemanticIndexes::default(),
        };

        let diagnostics = run_rules(&semantic);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id.as_str(), "no_empty_rules");
    }
}
