# Rule: `prefer_logical_properties`

## Intent

Encourage logical properties over physical directional properties for better writing-mode and i18n resilience.

## Compatibility Tag

`native`

## Algorithm Summary

1. Visit declarations.
2. Detect physical directional properties with one-to-one logical mappings.
3. Map to logical equivalents and report with suggested replacement.
4. Attach safe autofix that rewrites only the property name.

Current v1 mapping set includes:

- `margin-left/right` -> `margin-inline-start/end`
- `padding-left/right` -> `padding-inline-start/end`
- `border-left/right` and `border-left/right-{color|style|width}` -> inline start/end equivalents
- `left/right` -> `inset-inline-start/end`
- `top/bottom` -> `inset-block-start/end`

## Config Options and Defaults

```json
{
  "level": "warn"
}
```

## Default Severity

`warn`

## Fix Support and Safety

- Fix support: `yes` (one-to-one mappings only)
- Autofix is limited to mappings with unambiguous logical equivalents.
- Multi-value or shorthand conversions remain diagnostic-only in v1.

## Known Divergences from Stylelint

- This is a project-native rule with no direct Stylelint equivalent.

## Complexity and Performance Notes

- Time: O(number of declarations)
- Memory: constant plus property mapping table
