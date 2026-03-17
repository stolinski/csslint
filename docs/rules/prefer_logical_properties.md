# Rule: `prefer_logical_properties`

## Intent

Encourage logical properties over physical directional properties for better writing-mode and i18n resilience.

## Compatibility Tag

`native`

## Algorithm Summary

1. Visit declarations.
2. Detect physical directional properties (for example, `margin-left`, `padding-right`).
3. Map to logical equivalents when available.
4. Report with suggested logical replacement.

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
