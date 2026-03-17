# Plugin Surface (v1 Design)

## Purpose

Define an extension surface that allows future rules (including template-aware unused selector checks) without changing the core lint pipeline model.

## Scope for v1

v1 design target is a **typed Rust extension surface**, not runtime JavaScript plugins.

In scope:

- stable traits for rule registration
- optional provider traits for extra analysis data (for example template usage facts)
- deterministic integration contracts

Out of scope in v1:

- arbitrary JS plugin execution
- PostCSS plugin interoperability
- untrusted runtime code loading

## Extension Model

## 1) Rule Packs

Rule packs are Rust crates that register one or more rules.

```rust
pub trait RulePack {
    fn id(&self) -> &'static str;
    fn register(&self, registry: &mut RuleRegistry);
}
```

v1 assumption: packs are linked at build time (feature flags), not dynamically loaded.

## 2) Analysis Providers

Providers can supply derived facts that core CSS semantic traversal does not include.

Example future provider for template-aware checks:

```rust
pub trait UsageProvider {
    fn id(&self) -> &'static str;
    fn collect(&self, input: &ProviderInput) -> ProviderOutput;
}
```

Candidate provider outputs:

- class usage set
- id usage set
- confidence metadata (`high`, `medium`, `low`)

Rules may opt into provider data but must degrade gracefully when provider output is unavailable.

## 3) Capabilities and Safety

Each plugin rule declares capabilities, for example:

- needs selector events
- needs declaration events
- needs provider `template_usage`
- fixable or non-fixable

Engine requirements:

- deterministic output with and without providers
- no global mutable shared state across files
- panic containment identical to core rules

## Config Direction

Future config shape (reserved, not active in v1 runtime):

```json
{
  "plugins": ["org/template-usage-pack"],
  "rules": {
    "org/no-unused-scoped-selectors": "warn"
  }
}
```

Until runtime plugin loading exists, v1 treats plugin packs as compile-time linked extensions.

## Candidate Use Case: Unused Scoped Selectors

`no_unused_scoped_selectors` is deferred from core v1 defaults and is the reference use case for this plugin surface.

Typed provider contract reference:

- `docs/template-usage-provider-spec-v1.md`

Planned architecture for that rule:

1. CSS engine provides scoped selector candidates.
2. Template usage provider extracts class/id usage from Vue/Svelte template AST.
3. Rule compares candidate selectors against usage facts.
4. Rule reports only high-confidence unused selectors.

## Acceptance Criteria for Plugin Surface Design

1. Rule engine supports registering non-core rule packs.
2. Provider API shape is documented and typed.
   - `docs/template-usage-provider-spec-v1.md`
3. Core deterministic and safety guarantees remain unchanged.
4. Deferred template-aware rule has a clear integration path.
