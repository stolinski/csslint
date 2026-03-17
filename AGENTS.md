# CSSLint Agent Guide

This file is the operating contract for coding agents working in this repository.

## Mission

- Build a next-gen CSS linter that is faster than Stylelint and publishes comparative benchmark reports on releases.
- Support `.css`, `.vue` `<style>`, and `.svelte` `<style>` as first-class inputs.
- Keep behavior deterministic, typed, and safe for CI usage.

## v1 Scope Lock

Treat these as hard constraints unless docs are explicitly updated:

- In scope: Rust-first architecture, Lightning CSS parser, deterministic rule engine, safe `--fix`, JSON reporting contract.
- Out of scope: SCSS/LESS parsing, full Stylelint compatibility, arbitrary JS plugins, PostCSS plugin interop.
- Deferred rule: `no_unused_scoped_selectors` stays out of core v1 defaults; it is a plugin-surface candidate.

Reference: `docs/plan/01-scope-and-success-criteria.md`, `docs/deferrals-v1.md`, `docs/open-questions.md`.

## Source of Truth Files

Always align implementation and tests with these docs:

- Plan index and execution order: `docs/plan/README.md`
- Rule catalog and defaults: `docs/rule-catalog-v1.md`
- JSON reporter contract: `docs/json-output-schema-v1.schema.json` and `docs/json-output-schema-v1.md`
- Vue style behavior: `docs/vue-style-policy-v1.md`
- Plugin/provider design: `docs/plugin-surface-v1.md` and `docs/template-usage-provider-spec-v1.md`
- Release gates and numeric metrics: `docs/success-metrics.md`

If code and docs diverge, update docs in the same change.

## Architecture Rules

Follow the planned workspace boundaries:

- Intended crates: `csslint-core`, `csslint-extractor`, `csslint-parser`, `csslint-semantic`, `csslint-rules`, `csslint-fix`, `csslint-config`, `csslint-cli`, `csslint-test-harness` (plus optional `csslint-plugin-api`).
- Dependency direction: `core -> extractor/parser -> semantic -> rules/fix -> cli`.
- `csslint-parser` is the only crate that directly touches Lightning CSS parser APIs.
- No cyclic internal dependencies.

Reference: `docs/plan/02-workspace-and-crate-layout.md`.

## Extraction and Framework Policy

- `.css`: lint directly.
- `.vue`: extract `<style>` blocks in order with offset-accurate mapping.
- Vue `<style module>` and `<style module="name">`: mark `module=true` and treat as scoped context for native scoped rules in v1.
- Vue `<style src>`: emit non-fatal warning and skip extraction in SFC pipeline (do not resolve external files in v1).
- Unsupported style languages (for example `lang="scss"`, `lang="less"`): emit error diagnostic and skip that block.
- `.svelte`: extract `<style>` and apply Svelte scoped semantics in semantic phase.

Reference: `docs/plan/03-extractor-and-source-mapping.md`, `docs/vue-style-policy-v1.md`, `docs/open-questions.md`.

## Semantic and Rule Engine Contracts

- Parse each style block exactly once.
- Build semantic indexes once; rules consume semantic model and must not re-parse.
- Run enabled rules in one semantic traversal with typed callbacks.
- Deterministic ordering is mandatory for diagnostics and fixes.
- Process files in parallel only if final merged output remains globally deterministic.

Reference: `docs/plan/04-parser-and-semantic-model.md`, `docs/plan/05-rule-engine.md`.

## Rule Set and Defaults (v1)

Implement and maintain these rule IDs exactly as named:

- `no_unknown_properties`
- `no_invalid_values` (v1 subset)
- `no_duplicate_selectors`
- `no_duplicate_declarations`
- `no_empty_rules`
- `no_legacy_vendor_prefixes`
- `no_overqualified_selectors`
- `prefer_logical_properties`
- `no_global_leaks`
- `no_deprecated_features`

Rule severities/presets come from `docs/rule-catalog-v1.md`. Unknown rule IDs and invalid severities are config errors.

Per-rule specs must follow the common template in `docs/rules/README.md`.

## Fix Engine Safety Rules

- Fix coordinates use original-file byte offsets.
- Resolve overlaps deterministically with documented tie-breakers.
- Apply edits in descending offset order.
- Guarantee `--fix` idempotency.
- Keep diagnostics even when conflicting fixes are dropped.

Reference: `docs/plan/07-fix-engine.md`.

## CLI and Config Contract

- Supported v1 commands:
  - `csslint <path>`
  - `csslint <path> --fix`
  - `csslint <path> --format json`
- Config file is `.csslint` and JSON only in v1.
- Exit codes:
  - `0` no errors
  - `1` lint errors found
  - `2` runtime/config/internal failure
- Exit code precedence: runtime/config/internal failures win over lint findings.

Reference: `docs/plan/08-cli-and-config.md`, `docs/open-questions.md`.

## Testing and CI Expectations

Changes should preserve or improve these gates:

- Correctness: determinism, mapping accuracy, fix idempotency, malformed-input reliability.
- Compatibility: Stylelint harness pass/skip/fail reporting with explicit skip manifest.
- Framework native suite: Vue/Svelte scoped behavior and mapping are green.
- Performance: benchmark artifacts published; no >20% regression in median runtime or peak memory vs baseline unless explicitly accepted.

Reference: `docs/success-metrics.md`, `docs/plan/09-stylelint-compatibility-harness.md`, `docs/plan/10-native-framework-suite.md`, `docs/plan/11-performance-and-hardening.md`.

## Reporting Contract

- Keep JSON output compatible with `schemaVersion: 1`.
- Maintain stable fields for `summary`, `diagnostics`, `internalErrors`, and `timing`.
- Offsets are byte-based; line/column are 1-based.
- `fix` object is always present per diagnostic (`available: false` when not fixable).

Reference: `docs/json-output-schema-v1.schema.json`, `docs/json-output-schema-v1.md`.

## Change Management

- Avoid scope creep; capture out-of-scope requests in `docs/deferrals-v1.md` with reason, impact, milestone, and owner.
- Keep `docs/open-questions.md` updated when decisions are made.
- Document known compatibility divergences near affected rule specs and harness configs.

## Execution Order for Large Work

When implementing from scratch, follow this order:

1. scope lock and decisions
2. workspace/crate layout
3. extractor + source mapping
4. parser + semantic model
5. rule engine
6. first rule batches
7. fix engine
8. CLI + config + reporters
9. Stylelint compatibility harness
10. Vue/Svelte native suite
11. performance hardening

Reference: `docs/plan/README.md`.
