# Open Questions and Decisions

## Purpose

Track unresolved product and implementation decisions that affect v1 behavior.

## Resolved Decisions

| ID | Topic | Decision |
| --- | --- | --- |
| Q-001 | Config file name and format | Use `.csslint` JSON only in v1. |
| Q-002 | Performance gate shape | Comparative benchmark reporting versus Stylelint; no fixed speedup multiplier gate for v1. |
| Q-003 | Stylelint comparison baseline | Use latest Stylelint release and record exact version in benchmark artifacts. |
| Q-004 | Unsupported style languages | Default to error diagnostics for unsupported `lang` blocks (example: `lang="scss"`, `lang="less"`). |
| Q-005 | JSON reporter schema details | Normative schema is `docs/json-output-schema-v1.schema.json` with companion docs in `docs/json-output-schema-v1.md`. |
| Q-006 | Vue `<style module>` behavior | Treat `<style module>` and `<style module="name">` as scoped contexts for native scoped rules in v1 (`docs/vue-style-policy-v1.md`). |
| Q-007 | Vue `<style src>` behavior | Emit a non-fatal warning and skip extraction for `<style src>` blocks in v1 (no external resolution) (`docs/vue-style-policy-v1.md`). |
| Q-008 | `no_unused_scoped_selectors` confidence model | Defer from core v1 and implement via plugin surface with template-usage provider (`docs/plugin-surface-v1.md`). |
| Q-009 | Exit-code precedence with mixed failures | Runtime/config/internal failure wins (`2`) over lint findings (`1`) per `docs/json-output-schema-v1.md` and `docs/json-output-schema-v1.schema.json`. |
| Q-010 | Template usage provider contract | Typed input/output/confidence contract documented in `docs/template-usage-provider-spec-v1.md`. |

## Open Questions

None currently.

## Clarification: Template Parsing and Scoped-Selector Usage

For Vue/Svelte components, selector usage truth usually comes from template markup. If v1 does not parse template AST:

- the linter cannot reliably know whether `.foo` in `<style>` is used in `<template>`
- full unused-selector detection becomes heuristic

This is why `no_unused_scoped_selectors` is deferred from core v1 and mapped to plugin-surface design.

## Compatibility Threshold Clarification

"Overall and per-rule pass thresholds" means:

- overall: imported Stylelint cases passed across all selected rules
- per-rule: pass rate for each individual mapped rule

Current v1 default in this repo is ratchet-based instead of fixed percentages:

- always publish pass/skip/fail counts
- do not allow silent pass-rate drops without an explicit deferral/divergence update
