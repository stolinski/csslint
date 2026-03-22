# Rule: `no_legacy_vendor_prefixes`

## Intent

Disallow legacy vendor-prefixed properties and values when modern equivalents are expected.

## Compatibility Tag

`compatible`

## Algorithm Summary

1. Visit declarations and declaration values.
2. Match prefixed properties against an explicit allowlist snapshot from Stylelint `isAutoprefixable.mjs`.
3. Match prefixed values against an explicit allowlist snapshot from Stylelint `isAutoprefixable.mjs`.
4. Apply the `-webkit-background-size` safety guard before reporting/fixing property cases.
5. Report only allowlisted prefixed usages with replacement hints/fixes.

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
- Autofix only rewrites allowlisted one-to-one mappings with known unprefixed equivalents.
- Non-allowlisted prefixed properties/values are ignored to avoid unsafe fallback churn.

## Known Divergences from Stylelint

- Imported compatibility suites: `property-no-vendor-prefix` and `value-no-vendor-prefix` (`tests/compat/stylelint/imported/*.json`).
- v1 does not attempt full historical browser fallback modeling.
- Prefix replacement set is explicit and conservative (generated from pinned Stylelint snapshot via `scripts/extract_stylelint_vendor_allowlist.mjs`).
- PostCSS-integrated value-parser cases are explicitly skipped via `postcss_integration` entries in `tests/compat/stylelint/skip-manifest.yaml`.

## Complexity and Performance Notes

- Time: O(number of declarations)
- Memory: small shared prefix lookup tables
