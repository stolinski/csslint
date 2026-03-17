# Rule Test Matrix (v1)

## Purpose

Track required coverage across file types, fixture sources, and fix mode for each v1 rule.

Status values:

- `required`: must be implemented before v1 ship
- `n/a`: not applicable for this rule

## Matrix

| Rule ID | Tag | CSS (Imported) | CSS (Native) | Vue (Scoped/Global) | Svelte (Scoped/Mixed `:global`) | Fix Mode | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `no_unknown_properties` | compatible | required | required | required | required | n/a | Compatible with `property-no-unknown` subset |
| `no_invalid_values` | inspired | required | required | required | required | n/a | v1 is high-confidence subset only |
| `no_duplicate_selectors` | compatible | required | required | required | required | n/a | Deterministic normalization required |
| `no_duplicate_declarations` | compatible | required | required | required | required | required | Idempotency required |
| `no_empty_rules` | compatible | required | required | required | required | required | Empty block behavior must map correctly |
| `no_legacy_vendor_prefixes` | compatible | required | required | required | required | required | Property and value variants |
| `no_overqualified_selectors` | compatible | required | required | required | required | n/a | Scoped context should not cause false positives |
| `prefer_logical_properties` | native | n/a | required | required | required | required | Target-aware messaging if relevant |
| `no_global_leaks` | native | n/a | required | required | required | n/a | Explicit `:global(...)` escapes allowed |
| `no_deprecated_features` | native | n/a | required | required | required | n/a | Requires target policy fixtures |

## Global Gates

Before v1 release:

1. Every `required` matrix cell has passing coverage.
2. Fixable rules have idempotency tests in `.css`, `.vue`, and `.svelte` where applicable.
3. Imported compatibility and native framework suites both run in CI.
4. Source mapping assertions exist for framework fixture diagnostics.
