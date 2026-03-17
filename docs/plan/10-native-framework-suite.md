# Step 10: Native Framework Suite (Vue and Svelte)

## Purpose

Build a dedicated test suite for framework-specific behavior that Stylelint tests do not cover: scoped style semantics, `:global()` handling, and framework-aware leak detection.

## Scope

- Vue SFC `<style>`, `<style scoped>`, `<style module>`
- Vue SFC `<style src>` handling (warning + skip in v1)
- Svelte `<style>` with default scoped behavior
- mixed-scope selectors via `:global(...)`
- framework-specific rule behavior for global leakage and scoped selector semantics

## Test Suite Layout

```text
tests/native/
  vue/
    extractor/
    scope/
    rules/
  svelte/
    extractor/
    scope/
    rules/
  shared/
    mapping/
    fix/
```

## Required Fixture Categories

## Extraction and Mapping

- single and multiple style blocks
- scoped/module attribute combinations
- Vue `<style src>` warning and skip behavior
- line/column mapping into original component file
- CRLF handling in component sources

## Scope Semantics

- Vue scoped default behavior
- Vue module default scoped behavior (for native scoped rules)
- Vue non-scoped global behavior
- Svelte default scoped behavior
- `:global(...)` full selector escape
- `:global(...)` partial selector escape (mixed scopes)
- nested selectors with mixed scope

## Rule Behavior

- `no_global_leaks`
- `no_duplicate_selectors` in scoped contexts
- `no_overqualified_selectors` in component style blocks

## Fix Behavior

- fix span application inside `<style>` only
- no corruption of `<template>` or `<script>` regions
- idempotency for fixable rules in component files

## Scope Truth Table (v1)

| Context | Default scope |
| --- | --- |
| `.css` | global |
| Vue `<style scoped>` | scoped |
| Vue `<style module>` | scoped |
| Vue `<style>` (non-scoped) | global |
| Svelte `<style>` | scoped |
| `:global(...)` part | global override |

Mixed scope in a selector is valid and must be preserved.

## Vue `<style src>` v1 Behavior

- Do not resolve external source files from Vue SFC style blocks in v1.
- Emit one non-fatal warning diagnostic per `<style src>` block.
- Skip extraction for that block.
- External referenced files can still be linted if they are included as direct CLI input.

Policy reference: `docs/vue-style-policy-v1.md`.

## `no_global_leaks` v1 Behavior

Rule intent:

- warn/error when scoped blocks include accidental global selectors
- allow explicit global escape patterns (`:global(...)`)

v1 conservative policy:

- only flag high-confidence leak patterns
- avoid aggressive inference in ambiguous nested selectors

## Deferred Plugin Candidate

`no_unused_scoped_selectors` is deferred from core v1 and planned as a plugin-surface rule backed by template usage providers.

Reference: `docs/plugin-surface-v1.md`.
Fixture format: `docs/template-usage-fixtures-v1.md`.

## Golden Fixture Format

Each native fixture should include:

- full component source
- expected diagnostics (rule, span, message)
- expected fixed source (if applicable)
- notes for scope interpretation

## Regression Strategy

- every framework-specific bug gets a reproducer fixture
- include source mapping assertions for every regression fixture
- track false-positive regressions as high-priority test additions

## Deliverables

- framework fixture corpus for Vue and Svelte
- native test runner integrated in CI
- scope semantics reference fixtures
- framework rule behavior docs

## Exit Criteria

- Native framework suite green in CI.
- Mixed-scope selector handling verified.
- No critical source mapping regressions in component files.

## Risks and Mitigations

- **Risk**: plugin-path scoped-usage rule has noisy early behavior.
  - **Mitigation**: keep it out of core v1 defaults and gate via plugin rollout.
- **Risk**: fixes touching non-style regions.
  - **Mitigation**: strict offset boundaries from extractor metadata.
