# Step 4: Parser and Semantic Model

## Purpose

Use Lightning CSS for a single parse per style block, then build a normalized semantic model and indexes so rules can run without re-parsing or re-walking raw AST structures.

## Objectives

1. Parse once, reuse everywhere.
2. Normalize selectors/declarations for deterministic rule behavior.
3. Build framework-aware scope metadata (`scoped` vs `global`).
4. Provide indexes to reduce rule-time complexity.

## Parsing Layer

## Inputs

- `ExtractedStyle` from extractor.
- parser options including configured browser targets.

## Outputs

- parse tree wrapper (Lightning-backed internal representation).
- parse diagnostics with source spans mapped to original file offsets.

## Parser Policy

- One parse call per style block.
- No per-rule parser invocations.
- Recover gracefully from parse failures where possible.

## Target-Aware Behavior Clarification

Lightning parser support helps with modern syntax validity and normalization, but lint policy still needs explicit rules for:

- unsupported-by-target diagnostics
- deprecated feature diagnostics
- project policy constraints (style quality rules)

## Semantic Model

```rust
pub struct CssSemanticModel {
    pub rules: Vec<RuleNode>,
    pub selectors: Vec<SelectorNode>,
    pub declarations: Vec<DeclarationNode>,
    pub at_rules: Vec<AtRuleNode>,
    pub indexes: SemanticIndexes,
}

pub struct SemanticIndexes {
    pub selectors_by_class: HashMap<InternedStr, Vec<SelectorId>>,
    pub declarations_by_prop: HashMap<InternedStr, Vec<DeclarationId>>,
    pub declarations_by_rule: HashMap<RuleId, Vec<DeclarationId>>,
    pub selectors_by_scope: HashMap<Scope, Vec<SelectorId>>,
}
```

## Selector Representation

```rust
pub struct SelectorNode {
    pub id: SelectorId,
    pub raw: InternedStr,
    pub normalized: InternedStr,
    pub parts: Vec<SelectorPart>,
    pub span: Span,
}

pub struct SelectorPart {
    pub value: InternedStr,
    pub kind: SelectorPartKind, // class, id, tag, pseudo, attribute, combinator, etc.
    pub scope: Scope,           // scoped | global
}
```

## Normalization Strategy (v1)

- Normalize ignorable whitespace.
- Normalize equivalent escape forms where safe.
- Preserve token order except where semantic equivalence is explicit and tested.
- Keep both raw and normalized forms to avoid user-facing message confusion.

v1 rule: prefer conservative normalization to avoid false duplicate reports.

## Scope Annotation Strategy

Base scope from file/block metadata:

- `.css`: `global`
- Vue `<style scoped>`: `scoped`
- Vue `<style module>`: `scoped` (for native scoped rules in v1)
- Vue non-scoped `<style>`: `global`
- Svelte `<style>`: `scoped` by default

Vue module note:

- v1 treats module blocks as scoped contexts for native scoping behavior.
- v1 does not model class-name remapping details from CSS Modules semantics.

Override behavior:

- `:global(...)` marks enclosed selector parts as `global`.
- mixed scope in one selector is allowed and preserved.

Example transformation:

```text
.foo :global(.bar) .baz
=>
[.foo scoped] [.bar global] [.baz scoped]
```

## Indexing Strategy

Indexes should support common rule queries:

- duplicate selector detection
- declaration duplicate checks
- unknown/deprecated property checks
- scope leak detection

All indexes are built once during semantic build.

## Memory and Throughput Strategy

- Use interned strings for repeated property names/selectors.
- Keep node storage contiguous where possible.
- Avoid cloning large raw strings.
- Retain lightweight node IDs for cross-references.

## Testing Plan

### Parser Integration Tests

- modern CSS syntax inputs
- nested selectors
- escaping and attribute selector variants
- malformed CSS recovery behavior

### Semantic Snapshot Tests

- selector normalization outputs
- scope annotation outputs
- index population correctness

### Framework Scope Tests

- Vue scoped/non-scoped/module behavior
- Svelte default scoped behavior
- mixed `:global()` selectors

## Deliverables

- parser wrapper crate implementation.
- semantic model structures and builder.
- selector normalization and scope annotator modules.
- semantic snapshots and regression fixtures.

## Exit Criteria

- No rule requires direct Lightning AST traversal beyond semantic builder.
- Semantic indexes are sufficient for first 6 rules without additional passes.
- Scope semantics validated by native Vue/Svelte fixtures.

## Risks and Mitigations

- **Risk**: normalization overreach causing false positives.
  - **Mitigation**: conservative normalization and fixture-driven validation.
- **Risk**: ambiguous scoping in complex selectors.
  - **Mitigation**: explicit mixed-scope part modeling and focused edge-case tests.
