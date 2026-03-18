#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::panic::{self, AssertUnwindSafe};

use csslint_config::Config;
use csslint_core::{Diagnostic, FileId, Fix, RuleId, Scope, Severity, Span};
use csslint_semantic::{CssSemanticModel, DeclarationNode, RuleNode, SelectorNode};

pub type SelectorCallback = fn(&CssSemanticModel, &SelectorNode, &mut RuleRuntimeCtx);
pub type DeclarationCallback = fn(&CssSemanticModel, &DeclarationNode, &mut RuleRuntimeCtx);
pub type RuleNodeCallback = fn(&CssSemanticModel, &RuleNode, &mut RuleRuntimeCtx);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleMeta {
    pub id: RuleId,
    pub description: &'static str,
    pub default_severity: Severity,
    pub fixable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDiagnostic {
    pub rule_id: Option<RuleId>,
    pub message: String,
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

pub trait RulePack {
    fn id(&self) -> &'static str;
    fn register(&self, registry: &mut RuleRegistry);
}

pub fn register_rule_pack<P: RulePack>(registry: &mut RuleRegistry, pack: &P) {
    pack.register(registry);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderFrameworkKind {
    Vue,
    Svelte,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderStatus {
    Complete,
    Partial,
    FailedRecoverable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UsageKind {
    Class,
    Id,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UsageSource {
    StaticAttribute,
    FrameworkDirectiveLiteral,
    BindingLiteralBranch,
    DynamicExpressionHeuristic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageFact {
    pub kind: UsageKind,
    pub name: String,
    pub confidence: Confidence,
    pub source: UsageSource,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderDiagnostic {
    pub message: String,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateUsageOutput {
    pub status: ProviderStatus,
    pub facts: Vec<UsageFact>,
    pub unknown_regions: Vec<Span>,
    pub diagnostics: Vec<ProviderDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleBlockRef {
    pub block_index: u32,
    pub start_offset: usize,
    pub end_offset: usize,
    pub scoped: bool,
    pub module: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateUsageInput {
    pub file_id: FileId,
    pub file_path: String,
    pub framework: ProviderFrameworkKind,
    pub source: String,
    pub styles: Vec<StyleBlockRef>,
}

pub trait UsageProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn collect(&self, input: &TemplateUsageInput) -> TemplateUsageOutput;
}

#[derive(Default)]
pub struct UsageProviderRegistry {
    providers: BTreeMap<&'static str, Box<dyn UsageProvider>>,
}

impl UsageProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<P>(&mut self, provider: P) -> Result<(), String>
    where
        P: UsageProvider + 'static,
    {
        let id = provider.id();
        if self.providers.contains_key(id) {
            return Err(format!("duplicate usage provider registration: {id}"));
        }
        self.providers.insert(id, Box::new(provider));
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&dyn UsageProvider> {
        self.providers.get(id).map(|provider| provider.as_ref())
    }
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

    pub fn report_with_fix(&mut self, message: impl Into<String>, span: Span, fix: Fix) {
        if self.severity == Severity::Off {
            return;
        }

        self.diagnostics.push(
            Diagnostic::new(
                self.rule_id.clone(),
                self.severity,
                message,
                span,
                self.file_id,
            )
            .with_fix(fix),
        );
    }

    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    fn report_rule_runtime_failure(&mut self, message: impl Into<String>, span: Span) {
        self.diagnostics.push(Diagnostic::new(
            self.rule_id.clone(),
            Severity::Error,
            message,
            span,
            self.file_id,
        ));
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
    let _ = registry.register(NoDuplicateDeclarations);
    let _ = registry.register(NoLegacyVendorPrefixes);
    let _ = registry.register(NoDuplicateSelectors);
    let _ = registry.register(NoUnknownProperties);
    let _ = registry.register(NoOverqualifiedSelectors);
    let _ = registry.register(NoInvalidValues);
    let _ = registry.register(NoDeprecatedFeatures);
    let _ = registry.register(PreferLogicalProperties);
    let _ = registry.register(NoGlobalLeaks);

    for meta in placeholder_rule_metas() {
        let _ = registry.register(PlaceholderRule { meta });
    }

    registry
}

pub fn run_rules(semantic: &CssSemanticModel) -> Vec<Diagnostic> {
    run_rules_with_config(semantic, &Config::default()).unwrap_or_default()
}

pub fn run_rules_with_config(
    semantic: &CssSemanticModel,
    config: &Config,
) -> Result<Vec<Diagnostic>, Vec<ConfigDiagnostic>> {
    let registry = core_registry();
    run_with_registry(semantic, &registry, config)
}

fn run_with_registry(
    semantic: &CssSemanticModel,
    registry: &RuleRegistry,
    config: &Config,
) -> Result<Vec<Diagnostic>, Vec<ConfigDiagnostic>> {
    let known_rule_ids = registry
        .ordered_meta()
        .into_iter()
        .map(|meta| meta.id)
        .collect::<BTreeSet<_>>();

    let config_diagnostics = config
        .rules
        .keys()
        .filter(|rule_id| !known_rule_ids.contains(*rule_id))
        .map(|rule_id| ConfigDiagnostic {
            rule_id: Some(rule_id.clone()),
            message: format!("Unknown rule id: {rule_id}"),
        })
        .collect::<Vec<_>>();

    if !config_diagnostics.is_empty() {
        return Err(config_diagnostics);
    }

    let mut active_rules = Vec::new();

    for rule in registry.ordered_rules() {
        let meta = rule.meta();
        let severity = config
            .rules
            .get(&meta.id)
            .copied()
            .unwrap_or(meta.default_severity);

        if severity == Severity::Off {
            continue;
        }

        let visitor = rule.create(RuleContext { semantic, severity });
        active_rules.push(ActiveRule {
            on_selector: visitor.on_selector,
            on_declaration: visitor.on_declaration,
            on_rule: visitor.on_rule,
            runtime: RuleRuntimeCtx::new(semantic.file_id, meta.id, severity),
            failed: false,
        });
    }

    let mut rule_subscribers = Vec::new();
    let mut selector_subscribers = Vec::new();
    let mut declaration_subscribers = Vec::new();

    for (index, active_rule) in active_rules.iter().enumerate() {
        if let Some(callback) = active_rule.on_rule {
            rule_subscribers.push(RuleSubscriber {
                rule_index: index,
                callback,
            });
        }
        if let Some(callback) = active_rule.on_selector {
            selector_subscribers.push(SelectorSubscriber {
                rule_index: index,
                callback,
            });
        }
        if let Some(callback) = active_rule.on_declaration {
            declaration_subscribers.push(DeclarationSubscriber {
                rule_index: index,
                callback,
            });
        }
    }

    for node in &semantic.rules {
        for subscriber in &rule_subscribers {
            let active_rule = &mut active_rules[subscriber.rule_index];
            if active_rule.failed {
                continue;
            }

            let result = panic::catch_unwind(AssertUnwindSafe(|| {
                (subscriber.callback)(semantic, node, &mut active_rule.runtime)
            }));
            if result.is_err() {
                active_rule.failed = true;
                active_rule.runtime.report_rule_runtime_failure(
                    "Rule runtime panic was contained during rule-node dispatch",
                    node.span,
                );
            }
        }
    }

    for node in &semantic.selectors {
        for subscriber in &selector_subscribers {
            let active_rule = &mut active_rules[subscriber.rule_index];
            if active_rule.failed {
                continue;
            }

            let result = panic::catch_unwind(AssertUnwindSafe(|| {
                (subscriber.callback)(semantic, node, &mut active_rule.runtime)
            }));
            if result.is_err() {
                active_rule.failed = true;
                active_rule.runtime.report_rule_runtime_failure(
                    "Rule runtime panic was contained during selector dispatch",
                    node.span,
                );
            }
        }
    }

    for node in &semantic.declarations {
        for subscriber in &declaration_subscribers {
            let active_rule = &mut active_rules[subscriber.rule_index];
            if active_rule.failed {
                continue;
            }

            let result = panic::catch_unwind(AssertUnwindSafe(|| {
                (subscriber.callback)(semantic, node, &mut active_rule.runtime)
            }));
            if result.is_err() {
                active_rule.failed = true;
                active_rule.runtime.report_rule_runtime_failure(
                    "Rule runtime panic was contained during declaration dispatch",
                    node.span,
                );
            }
        }
    }

    let mut diagnostics = Vec::new();
    for active_rule in active_rules {
        diagnostics.extend(active_rule.runtime.into_diagnostics());
    }
    sort_diagnostics(&mut diagnostics);
    Ok(diagnostics)
}

pub fn sort_diagnostics(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort_by(|left, right| {
        (
            left.file_id.get(),
            left.span.start,
            left.span.end,
            severity_sort_key(left.severity),
            left.rule_id.as_str(),
            left.message.as_str(),
        )
            .cmp(&(
                right.file_id.get(),
                right.span.start,
                right.span.end,
                severity_sort_key(right.severity),
                right.rule_id.as_str(),
                right.message.as_str(),
            ))
    });
}

pub fn merge_and_sort_diagnostics(batches: Vec<Vec<Diagnostic>>) -> Vec<Diagnostic> {
    let total = batches.iter().map(Vec::len).sum();
    let mut merged = Vec::with_capacity(total);
    for mut batch in batches {
        merged.append(&mut batch);
    }
    sort_diagnostics(&mut merged);
    merged
}

fn severity_sort_key(severity: Severity) -> u8 {
    match severity {
        Severity::Off => 0,
        Severity::Warn => 1,
        Severity::Error => 2,
    }
}

struct ActiveRule {
    on_selector: Option<SelectorCallback>,
    on_declaration: Option<DeclarationCallback>,
    on_rule: Option<RuleNodeCallback>,
    runtime: RuleRuntimeCtx,
    failed: bool,
}

struct RuleSubscriber {
    rule_index: usize,
    callback: RuleNodeCallback,
}

struct SelectorSubscriber {
    rule_index: usize,
    callback: SelectorCallback,
}

struct DeclarationSubscriber {
    rule_index: usize,
    callback: DeclarationCallback,
}

struct NoEmptyRules;
struct NoDuplicateDeclarations;
struct NoLegacyVendorPrefixes;
struct NoDuplicateSelectors;
struct NoUnknownProperties;
struct NoOverqualifiedSelectors;
struct NoInvalidValues;
struct NoDeprecatedFeatures;
struct PreferLogicalProperties;
struct NoGlobalLeaks;

struct PlaceholderRule {
    meta: RuleMeta,
}

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

impl Rule for NoDuplicateDeclarations {
    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: RuleId::from("no_duplicate_declarations"),
            description: "Disallow duplicate declarations in a rule block",
            default_severity: Severity::Error,
            fixable: true,
        }
    }

    fn create(&self, _ctx: RuleContext<'_>) -> RuleVisitor {
        RuleVisitor {
            on_selector: None,
            on_declaration: None,
            on_rule: Some(no_duplicate_declarations_on_rule),
        }
    }
}

impl Rule for NoLegacyVendorPrefixes {
    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: RuleId::from("no_legacy_vendor_prefixes"),
            description: "Disallow legacy vendor-prefixed properties/values",
            default_severity: Severity::Warn,
            fixable: true,
        }
    }

    fn create(&self, _ctx: RuleContext<'_>) -> RuleVisitor {
        RuleVisitor {
            on_selector: None,
            on_declaration: Some(no_legacy_vendor_prefixes_on_declaration),
            on_rule: None,
        }
    }
}

impl Rule for NoDuplicateSelectors {
    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: RuleId::from("no_duplicate_selectors"),
            description: "Disallow duplicate selectors",
            default_severity: Severity::Error,
            fixable: false,
        }
    }

    fn create(&self, _ctx: RuleContext<'_>) -> RuleVisitor {
        RuleVisitor {
            on_selector: Some(no_duplicate_selectors_on_selector),
            on_declaration: None,
            on_rule: None,
        }
    }
}

impl Rule for NoUnknownProperties {
    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: RuleId::from("no_unknown_properties"),
            description: "Disallow unknown CSS properties",
            default_severity: Severity::Error,
            fixable: false,
        }
    }

    fn create(&self, _ctx: RuleContext<'_>) -> RuleVisitor {
        RuleVisitor {
            on_selector: None,
            on_declaration: Some(no_unknown_properties_on_declaration),
            on_rule: None,
        }
    }
}

impl Rule for NoOverqualifiedSelectors {
    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: RuleId::from("no_overqualified_selectors"),
            description: "Disallow overqualified selectors",
            default_severity: Severity::Warn,
            fixable: false,
        }
    }

    fn create(&self, _ctx: RuleContext<'_>) -> RuleVisitor {
        RuleVisitor {
            on_selector: Some(no_overqualified_selectors_on_selector),
            on_declaration: None,
            on_rule: None,
        }
    }
}

impl Rule for NoInvalidValues {
    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: RuleId::from("no_invalid_values"),
            description: "Disallow invalid declaration values in v1 high-confidence subset",
            default_severity: Severity::Error,
            fixable: false,
        }
    }

    fn create(&self, _ctx: RuleContext<'_>) -> RuleVisitor {
        RuleVisitor {
            on_selector: None,
            on_declaration: Some(no_invalid_values_on_declaration),
            on_rule: None,
        }
    }
}

impl Rule for NoDeprecatedFeatures {
    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: RuleId::from("no_deprecated_features"),
            description: "Disallow deprecated CSS features for v1 baseline targets",
            default_severity: Severity::Warn,
            fixable: false,
        }
    }

    fn create(&self, _ctx: RuleContext<'_>) -> RuleVisitor {
        RuleVisitor {
            on_selector: None,
            on_declaration: Some(no_deprecated_features_on_declaration),
            on_rule: Some(no_deprecated_features_on_rule),
        }
    }
}

impl Rule for PreferLogicalProperties {
    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: RuleId::from("prefer_logical_properties"),
            description: "Prefer logical over physical directional properties",
            default_severity: Severity::Warn,
            fixable: true,
        }
    }

    fn create(&self, _ctx: RuleContext<'_>) -> RuleVisitor {
        RuleVisitor {
            on_selector: None,
            on_declaration: Some(prefer_logical_properties_on_declaration),
            on_rule: None,
        }
    }
}

impl Rule for NoGlobalLeaks {
    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: RuleId::from("no_global_leaks"),
            description: "Disallow high-confidence global leaks in scoped style blocks",
            default_severity: Severity::Error,
            fixable: false,
        }
    }

    fn create(&self, _ctx: RuleContext<'_>) -> RuleVisitor {
        RuleVisitor {
            on_selector: Some(no_global_leaks_on_selector),
            on_declaration: None,
            on_rule: None,
        }
    }
}

impl Rule for PlaceholderRule {
    fn meta(&self) -> RuleMeta {
        self.meta.clone()
    }

    fn create(&self, _ctx: RuleContext<'_>) -> RuleVisitor {
        RuleVisitor::empty()
    }
}

fn placeholder_rule_metas() -> Vec<RuleMeta> {
    vec![]
}

fn no_empty_rules_on_rule(_semantic: &CssSemanticModel, rule: &RuleNode, ctx: &mut RuleRuntimeCtx) {
    if rule.is_at_rule || !rule.declaration_ids.is_empty() {
        return;
    }

    if rule.span.is_empty() {
        ctx.report("Empty rule block detected", rule.span);
        return;
    }

    ctx.report_with_fix(
        "Empty rule block detected",
        rule.span,
        Fix {
            span: rule.span,
            replacement: String::new(),
            rule_id: RuleId::from("no_empty_rules"),
            priority: 100,
        },
    );
}

fn no_duplicate_declarations_on_rule(
    semantic: &CssSemanticModel,
    rule: &RuleNode,
    ctx: &mut RuleRuntimeCtx,
) {
    let mut seen = BTreeSet::new();

    for declaration_id in &rule.declaration_ids {
        let Some(declaration) = semantic.declarations.get(declaration_id.0 as usize) else {
            continue;
        };

        let property = declaration.property.to_ascii_lowercase();
        let key = (property, declaration.value.clone());
        if seen.insert(key) {
            continue;
        }

        ctx.report_with_fix(
            format!(
                "Duplicate declaration '{}: {}' detected",
                declaration.property, declaration.value
            ),
            declaration.span,
            Fix {
                span: declaration.span,
                replacement: String::new(),
                rule_id: RuleId::from("no_duplicate_declarations"),
                priority: 200,
            },
        );
    }
}

fn no_duplicate_selectors_on_selector(
    semantic: &CssSemanticModel,
    selector: &SelectorNode,
    ctx: &mut RuleRuntimeCtx,
) {
    let Some(selector_ids) = semantic
        .indexes
        .selectors_by_normalized
        .get(&selector.normalized)
    else {
        return;
    };

    if selector_ids.len() <= 1 {
        return;
    }

    if selector_ids.first().copied() == Some(selector.id) {
        return;
    }

    ctx.report(
        format!("Duplicate selector '{}' detected", selector.normalized),
        selector.span,
    );
}

fn no_unknown_properties_on_declaration(
    _semantic: &CssSemanticModel,
    declaration: &DeclarationNode,
    ctx: &mut RuleRuntimeCtx,
) {
    if declaration.property.starts_with("--") || declaration.property_known {
        return;
    }

    ctx.report(
        format!("Unknown property '{}' detected", declaration.property),
        declaration.span,
    );
}

fn no_overqualified_selectors_on_selector(
    _semantic: &CssSemanticModel,
    selector: &SelectorNode,
    ctx: &mut RuleRuntimeCtx,
) {
    if selector_contains_overqualification(&selector.raw) {
        ctx.report(
            format!("Overqualified selector '{}' detected", selector.normalized),
            selector.span,
        );
    }
}

fn no_invalid_values_on_declaration(
    _semantic: &CssSemanticModel,
    declaration: &DeclarationNode,
    ctx: &mut RuleRuntimeCtx,
) {
    if declaration.property.starts_with("--") || !declaration.property_known {
        return;
    }

    let normalized_value = normalize_value_for_checks(&declaration.value);
    if normalized_value.is_empty() {
        ctx.report(
            format!(
                "Invalid value for property '{}': value cannot be empty",
                declaration.property
            ),
            declaration.span,
        );
        return;
    }

    if is_css_wide_keyword(normalized_value) {
        return;
    }

    let property = declaration.property.to_ascii_lowercase();
    let invalid = match property.as_str() {
        "display" => keyword_value_is_invalid(normalized_value, &DISPLAY_KEYWORDS),
        "position" => keyword_value_is_invalid(normalized_value, &POSITION_KEYWORDS),
        "visibility" => keyword_value_is_invalid(normalized_value, &VISIBILITY_KEYWORDS),
        "box-sizing" => keyword_value_is_invalid(normalized_value, &BOX_SIZING_KEYWORDS),
        "overflow" | "overflow-x" | "overflow-y" => {
            keyword_value_is_invalid(normalized_value, &OVERFLOW_KEYWORDS)
        }
        "flex-direction" => keyword_value_is_invalid(normalized_value, &FLEX_DIRECTION_KEYWORDS),
        "flex-wrap" => keyword_value_is_invalid(normalized_value, &FLEX_WRAP_KEYWORDS),
        "opacity" => opacity_value_is_invalid(normalized_value),
        _ => false,
    };

    if invalid {
        ctx.report(
            format!(
                "Invalid value '{}' for property '{}' in v1 validation subset",
                normalized_value, declaration.property
            ),
            declaration.span,
        );
    }
}

fn no_deprecated_features_on_declaration(
    _semantic: &CssSemanticModel,
    declaration: &DeclarationNode,
    ctx: &mut RuleRuntimeCtx,
) {
    let property = declaration.property.to_ascii_lowercase();
    if DEPRECATED_PROPERTIES.contains(&property.as_str()) {
        ctx.report(
            format!(
                "Deprecated feature '{}' is disallowed for {} targets",
                declaration.property, DEPRECATION_TARGET_PROFILE
            ),
            declaration.span,
        );
        return;
    }

    if property != "display" {
        return;
    }

    let normalized_value = normalize_value_for_checks(&declaration.value);
    if DISPLAY_DEPRECATED_VALUES
        .iter()
        .any(|deprecated| normalized_value.eq_ignore_ascii_case(deprecated))
    {
        ctx.report(
            format!(
                "Deprecated display value '{}' is disallowed for {} targets",
                normalized_value, DEPRECATION_TARGET_PROFILE
            ),
            declaration.span,
        );
    }
}

fn no_deprecated_features_on_rule(
    semantic: &CssSemanticModel,
    rule: &RuleNode,
    ctx: &mut RuleRuntimeCtx,
) {
    if !rule.is_at_rule {
        return;
    }

    let Some(name) = at_rule_name_from_source(semantic, rule) else {
        return;
    };

    if DEPRECATED_AT_RULES.contains(&name.as_str()) {
        ctx.report(
            format!(
                "Deprecated at-rule '@{}' is disallowed for {} targets",
                name, DEPRECATION_TARGET_PROFILE
            ),
            rule.span,
        );
    }
}

fn prefer_logical_properties_on_declaration(
    semantic: &CssSemanticModel,
    declaration: &DeclarationNode,
    ctx: &mut RuleRuntimeCtx,
) {
    let property = declaration.property.to_ascii_lowercase();
    let Some(logical_property) = physical_to_logical_property(property.as_str()) else {
        return;
    };

    let message = format!(
        "Prefer logical property '{}' over physical '{}'",
        logical_property, declaration.property
    );

    if let Some(fix) = declaration_replacement_fix(
        semantic,
        declaration,
        &declaration.property,
        logical_property,
        "prefer_logical_properties",
        350,
    ) {
        ctx.report_with_fix(message, declaration.span, fix);
    } else {
        ctx.report(message, declaration.span);
    }
}

fn no_global_leaks_on_selector(
    semantic: &CssSemanticModel,
    selector: &SelectorNode,
    ctx: &mut RuleRuntimeCtx,
) {
    if !is_scoped_style_context(semantic.scope) {
        return;
    }

    if !selector.raw.contains(":global(") {
        return;
    }

    if selector
        .parts
        .iter()
        .any(|part| part.scope != Scope::Global)
    {
        return;
    }

    ctx.report(
        format!(
            "Selector '{}' leaks globally from a scoped style block",
            selector.normalized
        ),
        selector.span,
    );
}

fn no_legacy_vendor_prefixes_on_declaration(
    semantic: &CssSemanticModel,
    declaration: &DeclarationNode,
    ctx: &mut RuleRuntimeCtx,
) {
    if let Some((prefix, unprefixed_property)) = strip_legacy_prefix(&declaration.property) {
        let message = format!(
            "Legacy vendor-prefixed property '{}' detected; use '{}'",
            declaration.property, unprefixed_property
        );

        if let Some(fix) = declaration_replacement_fix(
            semantic,
            declaration,
            &declaration.property,
            unprefixed_property,
            "no_legacy_vendor_prefixes",
            300,
        ) {
            ctx.report_with_fix(message, declaration.span, fix);
        } else {
            let _ = prefix;
            ctx.report(message, declaration.span);
        }
    }

    if let Some((prefixed_value, unprefixed_value)) = prefixed_value_variant(&declaration.value) {
        let message = format!(
            "Legacy vendor-prefixed value '{}' detected; use '{}'",
            prefixed_value, unprefixed_value
        );

        if let Some(fix) = declaration_replacement_fix(
            semantic,
            declaration,
            prefixed_value,
            &unprefixed_value,
            "no_legacy_vendor_prefixes",
            301,
        ) {
            ctx.report_with_fix(message, declaration.span, fix);
        } else {
            ctx.report(message, declaration.span);
        }
    }
}

fn declaration_replacement_fix(
    semantic: &CssSemanticModel,
    declaration: &DeclarationNode,
    needle: &str,
    replacement: &str,
    rule_id: &'static str,
    priority: u16,
) -> Option<Fix> {
    let local_start = declaration.span.start.checked_sub(semantic.span.start)?;
    let local_end = declaration.span.end.checked_sub(semantic.span.start)?;
    let segment = semantic.source.get(local_start..local_end)?;
    let replace_at = segment.find(needle)?;

    let mut rewritten = String::with_capacity(segment.len() - needle.len() + replacement.len());
    rewritten.push_str(&segment[..replace_at]);
    rewritten.push_str(replacement);
    rewritten.push_str(&segment[replace_at + needle.len()..]);

    Some(Fix {
        span: declaration.span,
        replacement: rewritten,
        rule_id: RuleId::from(rule_id),
        priority,
    })
}

fn strip_legacy_prefix(input: &str) -> Option<(&'static str, &str)> {
    LEGACY_PREFIXES.iter().find_map(|prefix| {
        input
            .strip_prefix(prefix)
            .filter(|suffix| !suffix.is_empty())
            .map(|suffix| (*prefix, suffix))
    })
}

fn prefixed_value_variant(value: &str) -> Option<(&str, String)> {
    let trimmed = value.trim();
    for prefix in LEGACY_PREFIXES {
        if let Some(suffix) = trimmed.strip_prefix(prefix) {
            if suffix.is_empty() {
                continue;
            }
            return Some((trimmed, suffix.to_string()));
        }
    }

    None
}

fn selector_contains_overqualification(selector: &str) -> bool {
    let bytes = selector.as_bytes();
    let mut segment_start = 0usize;
    let mut index = 0usize;
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut quote: Option<u8> = None;

    while index < bytes.len() {
        let current = bytes[index];

        if let Some(active_quote) = quote {
            if current == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }

        match current {
            b'"' | b'\'' => {
                quote = Some(current);
                index += 1;
            }
            b'[' => {
                bracket_depth += 1;
                index += 1;
            }
            b']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                index += 1;
            }
            b'(' => {
                paren_depth += 1;
                index += 1;
            }
            b')' => {
                paren_depth = paren_depth.saturating_sub(1);
                index += 1;
            }
            b'>' | b'+' | b'~' | b',' if bracket_depth == 0 && paren_depth == 0 => {
                if compound_is_overqualified(selector.get(segment_start..index).unwrap_or("")) {
                    return true;
                }
                segment_start = index + 1;
                index += 1;
            }
            _ if current.is_ascii_whitespace() && bracket_depth == 0 && paren_depth == 0 => {
                if compound_is_overqualified(selector.get(segment_start..index).unwrap_or("")) {
                    return true;
                }

                index += 1;
                while index < bytes.len() && bytes[index].is_ascii_whitespace() {
                    index += 1;
                }
                segment_start = index;
            }
            _ => {
                index += 1;
            }
        }
    }

    compound_is_overqualified(selector.get(segment_start..).unwrap_or(""))
}

fn compound_is_overqualified(compound: &str) -> bool {
    let raw = compound.trim();
    if raw.is_empty() {
        return false;
    }

    let candidate = strip_global_wrapper(raw);
    if candidate.is_empty() {
        return false;
    }

    let bytes = candidate.as_bytes();
    let mut index = 0usize;

    if let Some(namespace_end) = bytes.iter().position(|byte| *byte == b'|') {
        if namespace_end == 0 || is_ident_like_slice(&bytes[..namespace_end]) || bytes[0] == b'*' {
            index = namespace_end + 1;
        }
    }

    if index >= bytes.len() {
        return false;
    }

    if bytes[index] == b'*' || !bytes[index].is_ascii_alphabetic() {
        return false;
    }

    let tag_end = consume_ident(bytes, index);
    if tag_end == index {
        return false;
    }

    has_class_or_id_qualifier(&bytes[tag_end..])
}

fn strip_global_wrapper(compound: &str) -> &str {
    let trimmed = compound.trim();
    if let Some(rest) = trimmed.strip_prefix(":global(") {
        if let Some(inner) = rest.strip_suffix(')') {
            return inner.trim();
        }
    }

    trimmed
}

fn is_ident_like_slice(input: &[u8]) -> bool {
    !input.is_empty()
        && input
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_' || *byte == b'-')
}

fn consume_ident(bytes: &[u8], start: usize) -> usize {
    let mut index = start;
    while index < bytes.len() {
        let current = bytes[index];
        if current.is_ascii_alphanumeric() || current == b'_' || current == b'-' {
            index += 1;
            continue;
        }
        break;
    }
    index
}

fn has_class_or_id_qualifier(tail: &[u8]) -> bool {
    let mut index = 0usize;
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut quote: Option<u8> = None;

    while index < tail.len() {
        let current = tail[index];

        if let Some(active_quote) = quote {
            if current == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }

        match current {
            b'"' | b'\'' => {
                quote = Some(current);
            }
            b'[' => {
                bracket_depth += 1;
            }
            b']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
            }
            b'(' => {
                paren_depth += 1;
            }
            b')' => {
                paren_depth = paren_depth.saturating_sub(1);
            }
            b'.' | b'#' if bracket_depth == 0 && paren_depth == 0 => {
                return true;
            }
            _ => {}
        }

        index += 1;
    }

    false
}

fn keyword_value_is_invalid(value: &str, allowed_keywords: &[&str]) -> bool {
    if value.is_empty() || contains_dynamic_css_function(value) {
        return false;
    }

    if !is_single_identifier(value) {
        return false;
    }

    !allowed_keywords
        .iter()
        .any(|allowed| value.eq_ignore_ascii_case(allowed))
}

fn opacity_value_is_invalid(value: &str) -> bool {
    if contains_dynamic_css_function(value) {
        return false;
    }

    if let Ok(opacity) = value.parse::<f32>() {
        return !(0.0..=1.0).contains(&opacity);
    }

    is_single_identifier(value)
}

fn is_single_identifier(value: &str) -> bool {
    !value.is_empty()
        && !value
            .as_bytes()
            .iter()
            .any(|byte| byte.is_ascii_whitespace())
        && value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn contains_dynamic_css_function(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    ["var(", "env(", "calc(", "min(", "max(", "clamp(", "attr("]
        .iter()
        .any(|needle| lower.contains(needle))
}

fn normalize_value_for_checks(raw_value: &str) -> &str {
    let trimmed = raw_value.trim();
    if trimmed.len() >= IMPORTANT_SUFFIX.len()
        && trimmed[trimmed.len() - IMPORTANT_SUFFIX.len()..].eq_ignore_ascii_case(IMPORTANT_SUFFIX)
    {
        return trimmed[..trimmed.len() - IMPORTANT_SUFFIX.len()].trim_end();
    }

    trimmed
}

fn is_css_wide_keyword(value: &str) -> bool {
    CSS_WIDE_KEYWORDS
        .iter()
        .any(|keyword| value.eq_ignore_ascii_case(keyword))
}

fn at_rule_name_from_source(semantic: &CssSemanticModel, rule: &RuleNode) -> Option<String> {
    let local_start = rule.span.start.checked_sub(semantic.span.start)?;
    let local_end = rule.span.end.checked_sub(semantic.span.start)?;
    let segment = semantic.source.get(local_start..local_end)?.trim_start();

    let rest = segment.strip_prefix('@')?;
    let end = rest
        .find(|current: char| current.is_ascii_whitespace() || current == '{')
        .unwrap_or(rest.len());
    let name = rest.get(..end)?.trim();
    if name.is_empty() {
        return None;
    }

    Some(name.to_ascii_lowercase())
}

fn physical_to_logical_property(property: &str) -> Option<&'static str> {
    match property {
        "margin-left" => Some("margin-inline-start"),
        "margin-right" => Some("margin-inline-end"),
        "padding-left" => Some("padding-inline-start"),
        "padding-right" => Some("padding-inline-end"),
        "border-left" => Some("border-inline-start"),
        "border-right" => Some("border-inline-end"),
        "border-left-color" => Some("border-inline-start-color"),
        "border-right-color" => Some("border-inline-end-color"),
        "border-left-style" => Some("border-inline-start-style"),
        "border-right-style" => Some("border-inline-end-style"),
        "border-left-width" => Some("border-inline-start-width"),
        "border-right-width" => Some("border-inline-end-width"),
        "left" => Some("inset-inline-start"),
        "right" => Some("inset-inline-end"),
        "top" => Some("inset-block-start"),
        "bottom" => Some("inset-block-end"),
        _ => None,
    }
}

fn is_scoped_style_context(scope: Scope) -> bool {
    !matches!(scope, Scope::Global)
}

const LEGACY_PREFIXES: [&str; 4] = ["-webkit-", "-moz-", "-ms-", "-o-"];
const IMPORTANT_SUFFIX: &str = "!important";
const DEPRECATION_TARGET_PROFILE: &str = "v1-baseline";

const CSS_WIDE_KEYWORDS: [&str; 5] = ["inherit", "initial", "unset", "revert", "revert-layer"];

const DISPLAY_KEYWORDS: [&str; 22] = [
    "none",
    "block",
    "inline",
    "inline-block",
    "contents",
    "flow-root",
    "flex",
    "inline-flex",
    "grid",
    "inline-grid",
    "table",
    "inline-table",
    "table-row",
    "table-cell",
    "table-column",
    "table-column-group",
    "table-row-group",
    "table-header-group",
    "table-footer-group",
    "table-caption",
    "list-item",
    "run-in",
];

const POSITION_KEYWORDS: [&str; 5] = ["static", "relative", "absolute", "fixed", "sticky"];
const VISIBILITY_KEYWORDS: [&str; 3] = ["visible", "hidden", "collapse"];
const BOX_SIZING_KEYWORDS: [&str; 2] = ["content-box", "border-box"];
const OVERFLOW_KEYWORDS: [&str; 6] = ["visible", "hidden", "clip", "scroll", "auto", "overlay"];
const FLEX_DIRECTION_KEYWORDS: [&str; 4] = ["row", "row-reverse", "column", "column-reverse"];
const FLEX_WRAP_KEYWORDS: [&str; 3] = ["nowrap", "wrap", "wrap-reverse"];

const DEPRECATED_AT_RULES: [&str; 3] = ["viewport", "-ms-viewport", "-moz-document"];
const DEPRECATED_PROPERTIES: [&str; 3] = ["clip", "zoom", "box-flex-group"];
const DISPLAY_DEPRECATED_VALUES: [&str; 2] = ["box", "inline-box"];

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use csslint_config::Config;
    use csslint_core::{Diagnostic, FileId, RuleId, Scope, Severity, Span};
    use csslint_semantic::{
        CssSemanticModel, DeclarationId, DeclarationNode, RuleNode, RuleNodeId, SelectorId,
        SelectorNode, SelectorPart, SelectorPartKind, SemanticIndexes,
    };

    use super::{
        core_registry, merge_and_sort_diagnostics, register_rule_pack, run_rules,
        run_rules_with_config, run_with_registry, sort_diagnostics, Confidence, ProviderDiagnostic,
        ProviderStatus, Rule, RuleContext, RuleMeta, RulePack, RuleRegistry, RuleRuntimeCtx,
        RuleVisitor, TemplateUsageInput, TemplateUsageOutput, UsageFact, UsageKind, UsageProvider,
        UsageProviderRegistry, UsageSource,
    };

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

    #[test]
    fn no_duplicate_selectors_reports_second_and_later_occurrences() {
        let mut indexes = SemanticIndexes::default();
        indexes
            .selectors_by_normalized
            .insert(".btn".to_string(), vec![SelectorId(0), SelectorId(1)]);

        let semantic = CssSemanticModel {
            file_id: FileId::new(7),
            span: Span::new(0, 40),
            scope: Scope::Global,
            source: ".btn { color: red; } .btn { color: blue; }".to_string(),
            rules: vec![
                RuleNode {
                    id: RuleNodeId(0),
                    selector_ids: vec![SelectorId(0)],
                    declaration_ids: vec![DeclarationId(0)],
                    span: Span::new(0, 20),
                    is_at_rule: false,
                },
                RuleNode {
                    id: RuleNodeId(1),
                    selector_ids: vec![SelectorId(1)],
                    declaration_ids: vec![DeclarationId(1)],
                    span: Span::new(20, 40),
                    is_at_rule: false,
                },
            ],
            selectors: vec![
                SelectorNode {
                    id: SelectorId(0),
                    rule_id: RuleNodeId(0),
                    raw: ".btn".to_string(),
                    normalized: ".btn".to_string(),
                    parts: vec![SelectorPart {
                        value: ".btn".to_string(),
                        kind: SelectorPartKind::Class,
                        scope: Scope::Global,
                    }],
                    span: Span::new(0, 4),
                },
                SelectorNode {
                    id: SelectorId(1),
                    rule_id: RuleNodeId(1),
                    raw: ".btn".to_string(),
                    normalized: ".btn".to_string(),
                    parts: vec![SelectorPart {
                        value: ".btn".to_string(),
                        kind: SelectorPartKind::Class,
                        scope: Scope::Global,
                    }],
                    span: Span::new(20, 24),
                },
            ],
            declarations: vec![
                DeclarationNode {
                    id: DeclarationId(0),
                    rule_id: RuleNodeId(0),
                    property: "color".to_string(),
                    property_known: true,
                    value: "red".to_string(),
                    span: Span::new(7, 18),
                },
                DeclarationNode {
                    id: DeclarationId(1),
                    rule_id: RuleNodeId(1),
                    property: "color".to_string(),
                    property_known: true,
                    value: "blue".to_string(),
                    span: Span::new(27, 39),
                },
            ],
            at_rules: vec![],
            indexes,
        };

        let diagnostics = run_rules(&semantic);
        let duplicate_selector_diagnostics = diagnostics
            .into_iter()
            .filter(|diagnostic| diagnostic.rule_id.as_str() == "no_duplicate_selectors")
            .collect::<Vec<_>>();

        assert_eq!(duplicate_selector_diagnostics.len(), 1);
        assert_eq!(duplicate_selector_diagnostics[0].span, Span::new(20, 24));
        assert!(duplicate_selector_diagnostics[0]
            .message
            .contains("Duplicate selector"));
    }

    #[test]
    fn no_unknown_properties_uses_semantic_property_metadata() {
        let semantic = CssSemanticModel {
            file_id: FileId::new(8),
            span: Span::new(0, 28),
            scope: Scope::Global,
            source: ".box { colr: red; --brand: #fff; }".to_string(),
            rules: vec![RuleNode {
                id: RuleNodeId(0),
                selector_ids: vec![],
                declaration_ids: vec![DeclarationId(0), DeclarationId(1)],
                span: Span::new(0, 28),
                is_at_rule: false,
            }],
            selectors: vec![],
            declarations: vec![
                DeclarationNode {
                    id: DeclarationId(0),
                    rule_id: RuleNodeId(0),
                    property: "colr".to_string(),
                    property_known: false,
                    value: "red".to_string(),
                    span: Span::new(7, 17),
                },
                DeclarationNode {
                    id: DeclarationId(1),
                    rule_id: RuleNodeId(0),
                    property: "--brand".to_string(),
                    property_known: false,
                    value: "#fff".to_string(),
                    span: Span::new(18, 32),
                },
            ],
            at_rules: vec![],
            indexes: SemanticIndexes::default(),
        };

        let diagnostics = run_rules(&semantic);
        let unknown_property_diagnostics = diagnostics
            .into_iter()
            .filter(|diagnostic| diagnostic.rule_id.as_str() == "no_unknown_properties")
            .collect::<Vec<_>>();

        assert_eq!(unknown_property_diagnostics.len(), 1);
        assert!(unknown_property_diagnostics[0]
            .message
            .contains("Unknown property 'colr'"));
    }

    #[test]
    fn no_overqualified_selectors_reports_type_plus_class_or_id() {
        let semantic = CssSemanticModel {
            file_id: FileId::new(9),
            span: Span::new(0, 72),
            scope: Scope::Global,
            source: "article.card { color: red; } .card { color: red; } :global(button#cta) { color: red; }"
                .to_string(),
            rules: vec![
                RuleNode {
                    id: RuleNodeId(0),
                    selector_ids: vec![SelectorId(0)],
                    declaration_ids: vec![DeclarationId(0)],
                    span: Span::new(0, 28),
                    is_at_rule: false,
                },
                RuleNode {
                    id: RuleNodeId(1),
                    selector_ids: vec![SelectorId(1)],
                    declaration_ids: vec![DeclarationId(1)],
                    span: Span::new(29, 53),
                    is_at_rule: false,
                },
                RuleNode {
                    id: RuleNodeId(2),
                    selector_ids: vec![SelectorId(2)],
                    declaration_ids: vec![DeclarationId(2)],
                    span: Span::new(54, 90),
                    is_at_rule: false,
                },
            ],
            selectors: vec![
                SelectorNode {
                    id: SelectorId(0),
                    rule_id: RuleNodeId(0),
                    raw: "article.card".to_string(),
                    normalized: "article.card".to_string(),
                    parts: vec![],
                    span: Span::new(0, 12),
                },
                SelectorNode {
                    id: SelectorId(1),
                    rule_id: RuleNodeId(1),
                    raw: ".card".to_string(),
                    normalized: ".card".to_string(),
                    parts: vec![],
                    span: Span::new(29, 34),
                },
                SelectorNode {
                    id: SelectorId(2),
                    rule_id: RuleNodeId(2),
                    raw: ":global(button#cta)".to_string(),
                    normalized: ":global(button#cta)".to_string(),
                    parts: vec![],
                    span: Span::new(54, 73),
                },
            ],
            declarations: vec![
                DeclarationNode {
                    id: DeclarationId(0),
                    rule_id: RuleNodeId(0),
                    property: "color".to_string(),
                    property_known: true,
                    value: "red".to_string(),
                    span: Span::new(14, 25),
                },
                DeclarationNode {
                    id: DeclarationId(1),
                    rule_id: RuleNodeId(1),
                    property: "color".to_string(),
                    property_known: true,
                    value: "red".to_string(),
                    span: Span::new(36, 47),
                },
                DeclarationNode {
                    id: DeclarationId(2),
                    rule_id: RuleNodeId(2),
                    property: "color".to_string(),
                    property_known: true,
                    value: "red".to_string(),
                    span: Span::new(75, 86),
                },
            ],
            at_rules: vec![],
            indexes: SemanticIndexes::default(),
        };

        let diagnostics = run_rules(&semantic);
        let overqualified_selector_diagnostics = diagnostics
            .into_iter()
            .filter(|diagnostic| diagnostic.rule_id.as_str() == "no_overqualified_selectors")
            .collect::<Vec<_>>();

        assert_eq!(overqualified_selector_diagnostics.len(), 2);
        assert_eq!(overqualified_selector_diagnostics[0].span, Span::new(0, 12));
        assert_eq!(
            overqualified_selector_diagnostics[1].span,
            Span::new(54, 73)
        );
    }

    #[test]
    fn no_invalid_values_reports_subset_keyword_and_numeric_issues() {
        let semantic = CssSemanticModel {
            file_id: FileId::new(10),
            span: Span::new(0, 80),
            scope: Scope::Global,
            source: ".a { display: squish; opacity: 1.5; position: absolute; }".to_string(),
            rules: vec![RuleNode {
                id: RuleNodeId(0),
                selector_ids: vec![],
                declaration_ids: vec![DeclarationId(0), DeclarationId(1), DeclarationId(2)],
                span: Span::new(0, 58),
                is_at_rule: false,
            }],
            selectors: vec![],
            declarations: vec![
                DeclarationNode {
                    id: DeclarationId(0),
                    rule_id: RuleNodeId(0),
                    property: "display".to_string(),
                    property_known: true,
                    value: "squish".to_string(),
                    span: Span::new(5, 21),
                },
                DeclarationNode {
                    id: DeclarationId(1),
                    rule_id: RuleNodeId(0),
                    property: "opacity".to_string(),
                    property_known: true,
                    value: "1.5".to_string(),
                    span: Span::new(22, 36),
                },
                DeclarationNode {
                    id: DeclarationId(2),
                    rule_id: RuleNodeId(0),
                    property: "position".to_string(),
                    property_known: true,
                    value: "absolute".to_string(),
                    span: Span::new(37, 56),
                },
            ],
            at_rules: vec![],
            indexes: SemanticIndexes::default(),
        };

        let diagnostics = run_rules(&semantic);
        let invalid_value_diagnostics = diagnostics
            .into_iter()
            .filter(|diagnostic| diagnostic.rule_id.as_str() == "no_invalid_values")
            .collect::<Vec<_>>();

        assert_eq!(invalid_value_diagnostics.len(), 2);
        assert!(invalid_value_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("display")));
        assert!(invalid_value_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("opacity")));
    }

    #[test]
    fn no_invalid_values_skips_complex_values_to_avoid_noise() {
        let semantic = CssSemanticModel {
            file_id: FileId::new(11),
            span: Span::new(0, 72),
            scope: Scope::Global,
            source: ".a { display: inline flex; opacity: var(--opacity); }".to_string(),
            rules: vec![RuleNode {
                id: RuleNodeId(0),
                selector_ids: vec![],
                declaration_ids: vec![DeclarationId(0), DeclarationId(1)],
                span: Span::new(0, 52),
                is_at_rule: false,
            }],
            selectors: vec![],
            declarations: vec![
                DeclarationNode {
                    id: DeclarationId(0),
                    rule_id: RuleNodeId(0),
                    property: "display".to_string(),
                    property_known: true,
                    value: "inline flex".to_string(),
                    span: Span::new(5, 25),
                },
                DeclarationNode {
                    id: DeclarationId(1),
                    rule_id: RuleNodeId(0),
                    property: "opacity".to_string(),
                    property_known: true,
                    value: "var(--opacity)".to_string(),
                    span: Span::new(26, 50),
                },
            ],
            at_rules: vec![],
            indexes: SemanticIndexes::default(),
        };

        let diagnostics = run_rules(&semantic);
        assert!(diagnostics
            .into_iter()
            .all(|diagnostic| diagnostic.rule_id.as_str() != "no_invalid_values"));
    }

    #[test]
    fn no_deprecated_features_reports_policy_matches() {
        let semantic = CssSemanticModel {
            file_id: FileId::new(12),
            span: Span::new(0, 96),
            scope: Scope::Global,
            source: "@viewport { width: device-width; } .a { clip: rect(1px,2px,3px,4px); display: box; }"
                .to_string(),
            rules: vec![
                RuleNode {
                    id: RuleNodeId(0),
                    selector_ids: vec![],
                    declaration_ids: vec![],
                    span: Span::new(0, 33),
                    is_at_rule: true,
                },
                RuleNode {
                    id: RuleNodeId(1),
                    selector_ids: vec![],
                    declaration_ids: vec![DeclarationId(0), DeclarationId(1)],
                    span: Span::new(34, 90),
                    is_at_rule: false,
                },
            ],
            selectors: vec![],
            declarations: vec![
                DeclarationNode {
                    id: DeclarationId(0),
                    rule_id: RuleNodeId(1),
                    property: "clip".to_string(),
                    property_known: true,
                    value: "rect(1px,2px,3px,4px)".to_string(),
                    span: Span::new(40, 69),
                },
                DeclarationNode {
                    id: DeclarationId(1),
                    rule_id: RuleNodeId(1),
                    property: "display".to_string(),
                    property_known: true,
                    value: "box".to_string(),
                    span: Span::new(70, 83),
                },
            ],
            at_rules: vec![],
            indexes: SemanticIndexes::default(),
        };

        let diagnostics = run_rules(&semantic);
        let deprecated_diagnostics = diagnostics
            .into_iter()
            .filter(|diagnostic| diagnostic.rule_id.as_str() == "no_deprecated_features")
            .collect::<Vec<_>>();

        assert_eq!(deprecated_diagnostics.len(), 3);
        assert!(deprecated_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("@viewport")));
        assert!(deprecated_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("clip")));
        assert!(deprecated_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("display value 'box'")));
    }

    #[test]
    fn prefer_logical_properties_reports_and_offers_safe_fix() {
        let semantic = CssSemanticModel {
            file_id: FileId::new(13),
            span: Span::new(0, 34),
            scope: Scope::Global,
            source: ".card { margin-left: 1rem; }".to_string(),
            rules: vec![RuleNode {
                id: RuleNodeId(0),
                selector_ids: vec![],
                declaration_ids: vec![DeclarationId(0)],
                span: Span::new(0, 29),
                is_at_rule: false,
            }],
            selectors: vec![],
            declarations: vec![DeclarationNode {
                id: DeclarationId(0),
                rule_id: RuleNodeId(0),
                property: "margin-left".to_string(),
                property_known: true,
                value: "1rem".to_string(),
                span: Span::new(8, 26),
            }],
            at_rules: vec![],
            indexes: SemanticIndexes::default(),
        };

        let diagnostics = run_rules(&semantic);
        let logical_diagnostics = diagnostics
            .into_iter()
            .filter(|diagnostic| diagnostic.rule_id.as_str() == "prefer_logical_properties")
            .collect::<Vec<_>>();

        assert_eq!(logical_diagnostics.len(), 1);
        assert!(logical_diagnostics[0]
            .message
            .contains("margin-inline-start"));
        let fix = logical_diagnostics[0]
            .fix
            .as_ref()
            .expect("logical property diagnostic should include a fix");
        assert!(fix.replacement.contains("margin-inline-start"));
    }

    #[test]
    fn no_global_leaks_flags_only_fully_global_escapes_in_scoped_contexts() {
        let semantic = CssSemanticModel {
            file_id: FileId::new(14),
            span: Span::new(0, 66),
            scope: Scope::VueScoped,
            source: ":global(.theme) { color: red; } .card :global(.child) { color: red; }"
                .to_string(),
            rules: vec![
                RuleNode {
                    id: RuleNodeId(0),
                    selector_ids: vec![SelectorId(0)],
                    declaration_ids: vec![],
                    span: Span::new(0, 31),
                    is_at_rule: false,
                },
                RuleNode {
                    id: RuleNodeId(1),
                    selector_ids: vec![SelectorId(1)],
                    declaration_ids: vec![],
                    span: Span::new(32, 66),
                    is_at_rule: false,
                },
            ],
            selectors: vec![
                SelectorNode {
                    id: SelectorId(0),
                    rule_id: RuleNodeId(0),
                    raw: ":global(.theme)".to_string(),
                    normalized: ":global(.theme)".to_string(),
                    parts: vec![SelectorPart {
                        value: ".theme".to_string(),
                        kind: SelectorPartKind::Class,
                        scope: Scope::Global,
                    }],
                    span: Span::new(0, 15),
                },
                SelectorNode {
                    id: SelectorId(1),
                    rule_id: RuleNodeId(1),
                    raw: ".card :global(.child)".to_string(),
                    normalized: ".card :global(.child)".to_string(),
                    parts: vec![
                        SelectorPart {
                            value: ".card".to_string(),
                            kind: SelectorPartKind::Class,
                            scope: Scope::VueScoped,
                        },
                        SelectorPart {
                            value: ".child".to_string(),
                            kind: SelectorPartKind::Class,
                            scope: Scope::Global,
                        },
                    ],
                    span: Span::new(32, 53),
                },
            ],
            declarations: vec![],
            at_rules: vec![],
            indexes: SemanticIndexes::default(),
        };

        let diagnostics = run_rules(&semantic);
        let leak_diagnostics = diagnostics
            .into_iter()
            .filter(|diagnostic| diagnostic.rule_id.as_str() == "no_global_leaks")
            .collect::<Vec<_>>();

        assert_eq!(leak_diagnostics.len(), 1);
        assert_eq!(leak_diagnostics[0].span, Span::new(0, 15));
    }

    #[test]
    fn no_global_leaks_is_ignored_for_global_stylesheets() {
        let semantic = CssSemanticModel {
            file_id: FileId::new(15),
            span: Span::new(0, 22),
            scope: Scope::Global,
            source: ":global(.theme) {}".to_string(),
            rules: vec![RuleNode {
                id: RuleNodeId(0),
                selector_ids: vec![SelectorId(0)],
                declaration_ids: vec![],
                span: Span::new(0, 18),
                is_at_rule: false,
            }],
            selectors: vec![SelectorNode {
                id: SelectorId(0),
                rule_id: RuleNodeId(0),
                raw: ":global(.theme)".to_string(),
                normalized: ":global(.theme)".to_string(),
                parts: vec![SelectorPart {
                    value: ".theme".to_string(),
                    kind: SelectorPartKind::Class,
                    scope: Scope::Global,
                }],
                span: Span::new(0, 15),
            }],
            declarations: vec![],
            at_rules: vec![],
            indexes: SemanticIndexes::default(),
        };

        let diagnostics = run_rules(&semantic);
        assert!(diagnostics
            .into_iter()
            .all(|diagnostic| diagnostic.rule_id.as_str() != "no_global_leaks"));
    }

    #[test]
    fn engine_dispatches_each_event_type_once_per_node() {
        let semantic = CssSemanticModel {
            file_id: FileId::new(2),
            span: Span::new(0, 40),
            scope: Scope::Global,
            source: ".a,.b{color:red;margin:0}".to_string(),
            rules: vec![RuleNode {
                id: RuleNodeId(0),
                selector_ids: vec![SelectorId(0), SelectorId(1)],
                declaration_ids: vec![DeclarationId(0), DeclarationId(1)],
                span: Span::new(0, 24),
                is_at_rule: false,
            }],
            selectors: vec![
                SelectorNode {
                    id: SelectorId(0),
                    rule_id: RuleNodeId(0),
                    raw: ".a".to_string(),
                    normalized: ".a".to_string(),
                    parts: vec![SelectorPart {
                        value: ".a".to_string(),
                        kind: SelectorPartKind::Class,
                        scope: Scope::Global,
                    }],
                    span: Span::new(0, 2),
                },
                SelectorNode {
                    id: SelectorId(1),
                    rule_id: RuleNodeId(0),
                    raw: ".b".to_string(),
                    normalized: ".b".to_string(),
                    parts: vec![SelectorPart {
                        value: ".b".to_string(),
                        kind: SelectorPartKind::Class,
                        scope: Scope::Global,
                    }],
                    span: Span::new(3, 5),
                },
            ],
            declarations: vec![
                DeclarationNode {
                    id: DeclarationId(0),
                    rule_id: RuleNodeId(0),
                    property: "color".to_string(),
                    property_known: true,
                    value: "red".to_string(),
                    span: Span::new(8, 17),
                },
                DeclarationNode {
                    id: DeclarationId(1),
                    rule_id: RuleNodeId(0),
                    property: "margin".to_string(),
                    property_known: true,
                    value: "0".to_string(),
                    span: Span::new(18, 26),
                },
            ],
            at_rules: vec![],
            indexes: SemanticIndexes::default(),
        };

        let mut registry = RuleRegistry::new();
        let _ = registry.register(EventCountingRule);

        let empty_config = Config {
            rules: BTreeMap::new(),
        };
        let diagnostics = run_with_registry(&semantic, &registry, &empty_config)
            .expect("dispatch should run without config diagnostics");
        assert_eq!(diagnostics.len(), 5);
    }

    #[test]
    fn disabled_rules_do_not_instantiate() {
        OFF_RULE_CREATE_CALLS.store(0, Ordering::SeqCst);

        let semantic = CssSemanticModel {
            file_id: FileId::new(3),
            span: Span::new(0, 4),
            scope: Scope::Global,
            source: ".a{}".to_string(),
            rules: vec![],
            selectors: vec![],
            declarations: vec![],
            at_rules: vec![],
            indexes: SemanticIndexes::default(),
        };

        let mut registry = RuleRegistry::new();
        let _ = registry.register(OffRule);
        let empty_config = Config {
            rules: BTreeMap::new(),
        };
        let _ = run_with_registry(&semantic, &registry, &empty_config)
            .expect("off rule should not trigger config diagnostics");

        assert_eq!(OFF_RULE_CREATE_CALLS.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn unknown_rule_config_halts_execution_with_config_diagnostic() {
        let semantic = CssSemanticModel {
            file_id: FileId::new(4),
            span: Span::new(0, 4),
            scope: Scope::Global,
            source: ".a{}".to_string(),
            rules: vec![],
            selectors: vec![],
            declarations: vec![],
            at_rules: vec![],
            indexes: SemanticIndexes::default(),
        };

        let mut config = Config::default();
        config
            .rules
            .insert(RuleId::from("not_real"), Severity::Warn);

        let diagnostics =
            run_rules_with_config(&semantic, &config).expect_err("unknown rule should fail");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].rule_id.as_ref().map(RuleId::as_str),
            Some("not_real")
        );
    }

    #[test]
    fn severity_override_is_applied_to_runtime_diagnostics() {
        let semantic = CssSemanticModel {
            file_id: FileId::new(5),
            span: Span::new(0, 6),
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

        let mut config = Config::default();
        config
            .rules
            .insert(RuleId::from("no_empty_rules"), Severity::Error);

        let diagnostics =
            run_rules_with_config(&semantic, &config).expect("config should be valid");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].severity, Severity::Error);
    }

    #[test]
    fn sorts_diagnostics_with_deterministic_key() {
        let mut diagnostics = vec![
            Diagnostic::new(
                RuleId::from("z_rule"),
                Severity::Warn,
                "later rule",
                Span::new(20, 25),
                FileId::new(2),
            ),
            Diagnostic::new(
                RuleId::from("a_rule"),
                Severity::Error,
                "first rule",
                Span::new(10, 11),
                FileId::new(1),
            ),
            Diagnostic::new(
                RuleId::from("a_rule"),
                Severity::Warn,
                "same span lower severity",
                Span::new(10, 11),
                FileId::new(1),
            ),
        ];

        sort_diagnostics(&mut diagnostics);

        assert_eq!(diagnostics[0].file_id, FileId::new(1));
        assert_eq!(diagnostics[0].severity, Severity::Warn);
        assert_eq!(diagnostics[1].file_id, FileId::new(1));
        assert_eq!(diagnostics[1].severity, Severity::Error);
        assert_eq!(diagnostics[2].file_id, FileId::new(2));
    }

    #[test]
    fn merges_batches_then_sorts_globally() {
        let left = vec![Diagnostic::new(
            RuleId::from("b_rule"),
            Severity::Warn,
            "left",
            Span::new(30, 40),
            FileId::new(4),
        )];
        let right = vec![
            Diagnostic::new(
                RuleId::from("a_rule"),
                Severity::Warn,
                "right first",
                Span::new(5, 6),
                FileId::new(2),
            ),
            Diagnostic::new(
                RuleId::from("c_rule"),
                Severity::Warn,
                "right second",
                Span::new(7, 8),
                FileId::new(2),
            ),
        ];

        let merged = merge_and_sort_diagnostics(vec![left, right]);
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[0].message, "right first");
        assert_eq!(merged[1].message, "right second");
        assert_eq!(merged[2].message, "left");
    }

    #[test]
    fn contains_rule_runtime_panics_without_crashing_engine() {
        let semantic = CssSemanticModel {
            file_id: FileId::new(6),
            span: Span::new(0, 8),
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

        let mut registry = RuleRegistry::new();
        let _ = registry.register(PanicRule);
        let _ = registry.register(SafeRule);
        let empty_config = Config {
            rules: BTreeMap::new(),
        };

        let diagnostics = run_with_registry(&semantic, &registry, &empty_config)
            .expect("panic containment should not emit config diagnostics");
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("panic was contained")));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message == "safe rule still ran"));
    }

    #[test]
    fn rule_packs_can_register_rules_at_compile_time() {
        let mut registry = RuleRegistry::new();
        register_rule_pack(&mut registry, &TestRulePack);

        let metas = registry.ordered_meta();
        assert_eq!(metas.len(), 1);
        assert_eq!(metas[0].id.as_str(), "pack_rule");
    }

    #[test]
    fn usage_provider_registry_supports_typed_extension_hooks() {
        let mut providers = UsageProviderRegistry::new();
        assert!(providers.register(DummyUsageProvider).is_ok());
        assert!(providers.get("template_usage").is_some());

        let duplicate = providers
            .register(DummyUsageProvider)
            .expect_err("duplicate providers should fail");
        assert!(duplicate.contains("duplicate usage provider registration"));
    }

    struct EventCountingRule;

    struct PanicRule;

    impl Rule for PanicRule {
        fn meta(&self) -> RuleMeta {
            RuleMeta {
                id: RuleId::from("a_panic_rule"),
                description: "panic rule",
                default_severity: Severity::Warn,
                fixable: false,
            }
        }

        fn create(&self, _ctx: RuleContext<'_>) -> RuleVisitor {
            RuleVisitor {
                on_rule: Some(panic_on_rule),
                on_selector: None,
                on_declaration: None,
            }
        }
    }

    fn panic_on_rule(_semantic: &CssSemanticModel, _node: &RuleNode, _ctx: &mut RuleRuntimeCtx) {
        panic!("simulated rule panic");
    }

    struct SafeRule;

    impl Rule for SafeRule {
        fn meta(&self) -> RuleMeta {
            RuleMeta {
                id: RuleId::from("z_safe_rule"),
                description: "safe rule",
                default_severity: Severity::Warn,
                fixable: false,
            }
        }

        fn create(&self, _ctx: RuleContext<'_>) -> RuleVisitor {
            RuleVisitor {
                on_rule: Some(safe_on_rule),
                on_selector: None,
                on_declaration: None,
            }
        }
    }

    fn safe_on_rule(_semantic: &CssSemanticModel, node: &RuleNode, ctx: &mut RuleRuntimeCtx) {
        ctx.report("safe rule still ran", node.span);
    }

    struct TestRulePack;

    impl RulePack for TestRulePack {
        fn id(&self) -> &'static str {
            "test_pack"
        }

        fn register(&self, registry: &mut RuleRegistry) {
            let _ = registry.register(PackRule);
        }
    }

    struct PackRule;

    impl Rule for PackRule {
        fn meta(&self) -> RuleMeta {
            RuleMeta {
                id: RuleId::from("pack_rule"),
                description: "pack rule",
                default_severity: Severity::Warn,
                fixable: false,
            }
        }

        fn create(&self, _ctx: RuleContext<'_>) -> RuleVisitor {
            RuleVisitor::empty()
        }
    }

    struct DummyUsageProvider;

    impl UsageProvider for DummyUsageProvider {
        fn id(&self) -> &'static str {
            "template_usage"
        }

        fn collect(&self, _input: &TemplateUsageInput) -> TemplateUsageOutput {
            TemplateUsageOutput {
                status: ProviderStatus::Complete,
                facts: vec![UsageFact {
                    kind: UsageKind::Class,
                    name: "demo".to_string(),
                    confidence: Confidence::High,
                    source: UsageSource::StaticAttribute,
                    span: Span::new(0, 4),
                }],
                unknown_regions: Vec::new(),
                diagnostics: vec![ProviderDiagnostic {
                    message: "ok".to_string(),
                    span: None,
                }],
            }
        }
    }

    impl Rule for EventCountingRule {
        fn meta(&self) -> RuleMeta {
            RuleMeta {
                id: RuleId::from("event_counting_rule"),
                description: "test rule",
                default_severity: csslint_core::Severity::Warn,
                fixable: false,
            }
        }

        fn create(&self, _ctx: RuleContext<'_>) -> RuleVisitor {
            RuleVisitor {
                on_rule: Some(report_rule),
                on_selector: Some(report_selector),
                on_declaration: Some(report_declaration),
            }
        }
    }

    fn report_rule(_semantic: &CssSemanticModel, node: &RuleNode, ctx: &mut RuleRuntimeCtx) {
        ctx.report("rule", node.span);
    }

    fn report_selector(
        _semantic: &CssSemanticModel,
        node: &SelectorNode,
        ctx: &mut RuleRuntimeCtx,
    ) {
        ctx.report("selector", node.span);
    }

    fn report_declaration(
        _semantic: &CssSemanticModel,
        node: &DeclarationNode,
        ctx: &mut RuleRuntimeCtx,
    ) {
        ctx.report("declaration", node.span);
    }

    static OFF_RULE_CREATE_CALLS: AtomicUsize = AtomicUsize::new(0);

    struct OffRule;

    impl Rule for OffRule {
        fn meta(&self) -> RuleMeta {
            RuleMeta {
                id: RuleId::from("off_rule"),
                description: "off rule",
                default_severity: csslint_core::Severity::Off,
                fixable: false,
            }
        }

        fn create(&self, _ctx: RuleContext<'_>) -> RuleVisitor {
            OFF_RULE_CREATE_CALLS.fetch_add(1, Ordering::SeqCst);
            RuleVisitor::empty()
        }
    }
}
