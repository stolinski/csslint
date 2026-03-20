# Deferrals (v1)

## Purpose

Track intentionally deferred work so v1 scope stays stable without losing roadmap visibility.

## Deferral Entry Format

Each entry includes:

- title
- reason for deferral
- impact
- proposed milestone
- owner

## Current Deferrals

| ID | Title | Reason For Deferral | Impact | Proposed Milestone | Owner |
| --- | --- | --- | --- | --- | --- |
| D-001 | SCSS and LESS parsing support | Explicit v1 non-goal; parser and rule semantics expand significantly | Users with SCSS/LESS cannot lint those files in v1 | v2 | TBD |
| D-002 | Full Stylelint compatibility | Would force broad PostCSS/custom syntax behavior that conflicts with v1 architecture goals | Some imported compatibility cases remain skipped/divergent | v2 | TBD |
| D-003 | Advanced directive-comment behavior parity | v1 supports core `csslint-disable*` and `stylelint-disable*` aliases, but not full Stylelint reporting/description option matrix | Some niche suppression workflows may differ from Stylelint | v1.5 | TBD |
| D-004 | Arbitrary JS plugin execution API | Security, determinism, and performance complexity beyond v1 | No third-party runtime plugin ecosystem in v1 | v2 | TBD |
| D-005 | PostCSS plugin ecosystem interoperability | Conflicts with Lightning-first architecture and v1 constraints | Existing PostCSS lint pipelines need migration effort | v2 | TBD |
| D-006 | Core built-in `no_unused_scoped_selectors` rule | Requires cross-AST template usage analysis and confidence model beyond core v1 scope | Rule is removed from core v1 defaults and targeted for plugin-surface implementation (contract in `docs/template-usage-provider-spec-v1.md`) | v1.5 | TBD |
| D-007 | Rich include/exclude glob config model | v1 now supports `.csslintignore` and `--ignore-path`, but full glob policy controls are still intentionally minimal | Advanced monorepo/path workflows may still need wrappers | v1.5 | TBD |
| D-008 | Vue `<style src>` external file resolution | v1 extractor does not resolve remote/relative external style sources from SFCs | `<style src>` blocks are warned and skipped unless files are linted directly | v1.5 | TBD |

## Review Policy

- Deferrals are reviewed before each minor release.
- Any v1 scope change must update this file and linked plan docs.
- A deferral can only be removed when ownership and acceptance criteria are added to an implementation plan.
