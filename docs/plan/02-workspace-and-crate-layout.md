# Step 2: Workspace and Crate Layout

## Purpose

Create a Rust workspace layout that enforces clean boundaries between extraction, parsing, semantic modeling, rule execution, fixing, and CLI/reporting.

The layout must optimize for:

- performance (minimal copying, predictable data flow)
- maintainability (clear ownership per crate)
- testability (small integration surfaces)

## Proposed Workspace Structure

```text
csslint/
  Cargo.toml
  rust-toolchain.toml
  crates/
    csslint-core/
    csslint-extractor/
    csslint-parser/
    csslint-semantic/
    csslint-rules/
    csslint-plugin-api/   (optional, API-only crate for extension contracts)
    csslint-fix/
    csslint-config/
    csslint-cli/
    csslint-test-harness/
  tests/
    compat/
    native/
    fix/
    perf/
```

## Crate Responsibilities

### `csslint-core`

- Fundamental shared types and traits.
- IDs, spans, diagnostics, severity, file metadata.
- No dependency on Lightning CSS or CLI concerns.

### `csslint-extractor`

- Input file loading and style block extraction.
- `.css` direct, `.vue` style block scan, `.svelte` style block scan.
- Produces `ExtractedStyle` and source offset metadata.

### `csslint-parser`

- Wraps Lightning CSS parse calls and parser options.
- Converts parse errors to project diagnostics.
- Exposes typed parse results to semantic crate.

### `csslint-semantic`

- Builds normalized semantic model once per style block.
- Owns selector normalization and scope annotations.
- Builds indexes consumed by rules.

### `csslint-rules`

- Rule registry and implementations.
- Rule engine dispatcher and visitor lifecycle.
- Rule configuration interpretation (`off/warn/error`).

### `csslint-plugin-api` (optional in v1)

- Typed extension contracts for rule packs and analysis providers.
- No runtime loader required for v1.
- Keeps future plugin features isolated from core engine internals.

### `csslint-fix`

- Fix collection, conflict resolution, application.
- Idempotency and overlap safety enforcement.

### `csslint-config`

- Config file discovery, parsing, schema validation.
- Presets and target configuration handling.

### `csslint-cli`

- CLI argument parsing.
- File traversal orchestration.
- Reporter selection and process exit behavior.

### `csslint-test-harness`

- Shared fixture parser, assertion helpers, snapshot helpers.
- Compatibility fixture ingestion and native fixture execution.

## Dependency Direction Rules

Dependencies should flow in one direction only:

`core -> extractor/parser -> semantic -> rules/fix -> cli`

Rules:

- `csslint-core` may not depend on any internal crate.
- `csslint-rules` cannot import CLI code.
- `csslint-parser` should be the only crate that directly touches Lightning parser APIs.
- No cyclic internal dependencies.

## Core Type Contracts

The following types must be defined in `csslint-core` and reused everywhere:

- `FileId`
- `Span { start, end }`
- `Diagnostic { rule_id, severity, message, span, file_id, fix? }`
- `Fix { span, replacement, rule_id, priority }`
- `RuleId`
- `Severity`
- `Scope`

This avoids serialization mismatches and conversion overhead.

## Build and Tooling Policy

- Pin Rust toolchain version with `rust-toolchain.toml`.
- Enforce `cargo fmt` and `clippy` in CI.
- Use strict linting (`deny(warnings)` in CI).
- Keep optional profiling flags behind feature gates.

## Testing Layout and Ownership

- Unit tests live with crate internals.
- Cross-crate behavior tested in `tests/` integration suites.
- Compatibility fixtures live in `tests/compat/stylelint`.
- Framework fixtures live in `tests/native/{vue,svelte}`.
- Perf harness lives in `tests/perf` (can run as separate CI job).

## CI Job Breakdown

1. `check`: `cargo check --workspace`
2. `lint`: `cargo fmt --check` + `cargo clippy`
3. `unit`: crate unit tests
4. `integration`: compat/native/fix integration suites
5. `perf`: benchmark smoke + regression threshold checks

## Milestones

### Milestone A

- Workspace builds with all crates stubbed.
- Core types compile and are consumed by extractor/parser skeletons.

### Milestone B

- End-to-end flow works for one simple rule on `.css`.
- CLI can parse one file and report one diagnostic.

### Milestone C

- Full architecture in place and ready for rule batch implementation.

## Deliverables

- Workspace skeleton committed.
- Crate responsibility README per crate.
- CI pipeline with staged jobs.

## Exit Criteria

- `cargo check --workspace` and base tests pass.
- No forbidden dependency direction violations.
- Team agrees crate ownership boundaries are final for v1.

## Risks and Mitigations

- **Risk**: too many abstractions too early.
  - **Mitigation**: start with minimal public APIs and expand only when required.
- **Risk**: parser/semantic boundary leaks Lightning internals.
  - **Mitigation**: enforce typed project-owned model at semantic boundary.
