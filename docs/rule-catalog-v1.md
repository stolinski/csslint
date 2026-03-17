# Rule Catalog (v1)

## Purpose

Define the canonical v1 rule IDs, compatibility tags, default severities, fixability, and preset membership.

This file is the source of truth for config defaults.

## Rule IDs and Defaults

| Rule ID | Tag | Default (recommended) | Fixable | Stylelint relationship |
| --- | --- | --- | --- | --- |
| `no_unknown_properties` | compatible | error | no | `property-no-unknown` |
| `no_invalid_values` | inspired | error | no | partial `declaration-property-value-no-unknown` |
| `no_duplicate_selectors` | compatible | error | no | `no-duplicate-selectors` |
| `no_duplicate_declarations` | compatible | error | yes | `declaration-block-no-duplicate-properties` |
| `no_empty_rules` | compatible | warn | yes | `block-no-empty` |
| `no_legacy_vendor_prefixes` | compatible | warn | yes | `property-no-vendor-prefix`, `value-no-vendor-prefix` |
| `no_overqualified_selectors` | compatible | warn | no | `selector-no-qualifying-type` |
| `prefer_logical_properties` | native | warn | yes | none |
| `no_global_leaks` | native | error | no | none |
| `no_deprecated_features` | native | warn | no | none |

## Presets

## `recommended` (default)

Balanced defaults focused on low-noise adoption.

```json
{
  "no_unknown_properties": "error",
  "no_invalid_values": "error",
  "no_duplicate_selectors": "error",
  "no_duplicate_declarations": "error",
  "no_empty_rules": "warn",
  "no_legacy_vendor_prefixes": "warn",
  "no_overqualified_selectors": "warn",
  "prefer_logical_properties": "warn",
  "no_global_leaks": "error",
  "no_deprecated_features": "warn"
}
```

## `strict`

Escalates all v1 rules to error.

```json
{
  "no_unknown_properties": "error",
  "no_invalid_values": "error",
  "no_duplicate_selectors": "error",
  "no_duplicate_declarations": "error",
  "no_empty_rules": "error",
  "no_legacy_vendor_prefixes": "error",
  "no_overqualified_selectors": "error",
  "prefer_logical_properties": "error",
  "no_global_leaks": "error",
  "no_deprecated_features": "error"
}
```

## `minimal`

High-confidence essentials for low-friction adoption.

```json
{
  "no_unknown_properties": "error",
  "no_invalid_values": "error",
  "no_duplicate_selectors": "error",
  "no_duplicate_declarations": "error",
  "no_global_leaks": "error"
}
```

## Configuration Contract

- Rule keys are stable snake_case IDs.
- Allowed values are `off`, `warn`, `error`.
- Unknown rule IDs are config errors.
- Invalid severity values are config errors.

## Per-Rule Specs

Detailed behavior, options, and divergences live in:

- `docs/rules/no_unknown_properties.md`
- `docs/rules/no_invalid_values.md`
- `docs/rules/no_duplicate_selectors.md`
- `docs/rules/no_duplicate_declarations.md`
- `docs/rules/no_empty_rules.md`
- `docs/rules/no_legacy_vendor_prefixes.md`
- `docs/rules/no_overqualified_selectors.md`
- `docs/rules/prefer_logical_properties.md`
- `docs/rules/no_global_leaks.md`
- `docs/rules/no_deprecated_features.md`

Deferred plugin candidate:

- `docs/rules/no_unused_scoped_selectors.md`
