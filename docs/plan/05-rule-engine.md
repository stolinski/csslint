# Step 5: Rule Engine

## Purpose

Implement a deterministic, typed rule engine that runs all enabled rules in one semantic traversal and produces stable diagnostics and optional fixes.

## Design Principles

1. Single semantic traversal, many rule listeners.
2. Deterministic output independent of parallel file execution.
3. Typed events and context, no stringly runtime APIs.
4. Rule isolation and error containment.

## Rule API (Rust-Oriented)

```rust
pub trait Rule {
    fn meta(&self) -> RuleMeta;
    fn create(&self, ctx: RuleContext) -> RuleVisitor;
}

pub struct RuleMeta {
    pub id: RuleId,
    pub description: &'static str,
    pub default_severity: Severity,
    pub fixable: bool,
}

pub struct RuleVisitor {
    pub on_selector: Option<fn(&SelectorNode, &mut RuleRuntimeCtx)>,
    pub on_declaration: Option<fn(&DeclarationNode, &mut RuleRuntimeCtx)>,
    pub on_rule: Option<fn(&RuleNode, &mut RuleRuntimeCtx)>,
}
```

## Execution Pipeline

1. Load config and determine enabled rules.
2. Instantiate visitor callbacks for enabled rules.
3. Traverse semantic model nodes in stable order.
4. Dispatch node events to subscribed visitors.
5. Collect diagnostics and fixes.
6. Sort diagnostics by stable key.

## Determinism Requirements

- Rule registration order must be stable (e.g., by rule ID).
- Node traversal order must be stable (in source order).
- Diagnostic sort key:
  - file path
  - start offset
  - end offset
  - severity
  - rule ID
  - message

## Context API

Rules should receive a constrained context to avoid architecture leaks:

- read-only semantic view
- file metadata and scope metadata
- config/options for that rule
- `report()` for diagnostics
- `propose_fix()` for optional edits

Rules should not mutate global engine state directly.

## Rule Config Handling

- Support severity levels: `off`, `warn`, `error`.
- Disabled rules are not instantiated.
- Rule option parsing is validated up-front.
- Invalid rule config yields configuration diagnostics before linting starts.

## Rule Isolation and Safety

- Rule runtime failures should not crash the linter process.
- In debug/dev mode, internal rule errors can be surfaced as internal diagnostics.
- In release mode, prefer controlled failure for the file and continue processing others.

## Performance Strategy

- Precompute callback subscription map per event type.
- Avoid per-node allocations during dispatch.
- Reuse temporary buffers where possible.

## Parallelism Strategy

- Process files in parallel.
- Process each file's rules single-threaded for deterministic per-file behavior.
- Merge results across files with global deterministic sorting.

## Extension Hooks (Design Target)

To support future non-core rules (for example template-aware selector usage), engine design should include typed extension hooks:

- rule-pack registration interface
- optional provider data channel (for external analysis facts)
- capability declaration per rule

v1 runtime can keep this compile-time/internal only, but API boundaries should not block later plugin loading models.

Reference: `docs/plugin-surface-v1.md`.

Provider contract details: `docs/template-usage-provider-spec-v1.md`.

## Testing Plan

### Engine Tests

- callback dispatch order
- rule enable/disable behavior
- rule config validation path
- deterministic output under repeated runs

### Integration Tests

- multiple rules reporting on same span
- severity overrides from config
- mixed file types in one run

### Reliability Tests

- rule panic containment test
- large file dispatch performance smoke test

## Deliverables

- `csslint-rules` engine core.
- rule registry and metadata table.
- deterministic result sorter.
- engine integration tests.
- extension-hook contracts documented for future plugin packs.

## Exit Criteria

- First rule batch runs fully through engine in one pass.
- Deterministic snapshot tests pass across repeated runs.
- No per-rule semantic traversal introduced.

## Risks and Mitigations

- **Risk**: dispatch overhead with many rules.
  - **Mitigation**: event subscription map and zero-cost no-op path.
- **Risk**: non-determinism from parallel merge.
  - **Mitigation**: strict final global sorting and deterministic tie-breakers.
