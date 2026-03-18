# Rule: `no_legacy_vendor_prefixes`

## Intent

Disallow legacy vendor-prefixed properties and values when modern equivalents are expected.

## Compatibility Tag

`compatible`

## Algorithm Summary

1. Visit declarations and declaration values.
2. Match known prefixed property/value patterns.
3. Check if an unprefixed equivalent exists in current lint policy.
4. Report prefixed usage with replacement hint where safe.

## Config Options and Defaults

```json
{
  "level": "warn"
}
```

## Default Severity

`warn`

## Fix Support and Safety

- Fix support: `yes` (safe mapping subset)
- Autofix only rewrites prefixes with one-to-one known modern equivalents.
- Unknown or ambiguous mappings are diagnostic-only.

## Known Divergences from Stylelint

- Imported compatibility suites: `property-no-vendor-prefix` and `value-no-vendor-prefix` (`tests/compat/stylelint/imported/*.json`).
- v1 does not attempt full historical browser fallback modeling.
- Prefix replacement set is explicit and conservative.
- PostCSS-integrated value-parser cases are explicitly skipped via `postcss_integration` entries in `tests/compat/stylelint/skip-manifest.yaml`.

## Complexity and Performance Notes

- Time: O(number of declarations)
- Memory: small shared prefix lookup tables
