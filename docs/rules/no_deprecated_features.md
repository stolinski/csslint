# Rule: `no_deprecated_features`

## Intent

Report use of CSS features marked deprecated or disallowed by project compatibility policy for configured targets.

## Compatibility Tag

`native`

## Algorithm Summary

1. Visit declarations and at-rules covered by policy.
2. Match usage against the v1 baseline deprecation policy table.
   - declaration properties: `clip`, `zoom`, `box-flex-group`
   - declaration values: `display: box`, `display: inline-box`
   - at-rules: `@viewport`, `@-ms-viewport`, `@-moz-document`
3. Report deprecated usage with a target-profile message (`v1-baseline`).

## Config Options and Defaults

```json
{
  "level": "warn"
}
```

## Default Severity

`warn`

## Fix Support and Safety

- Fix support: `none`
- Rationale: deprecation replacements are feature-specific and often not mechanically safe.

## Known Divergences from Stylelint

- Policy is project-owned and target-aware.
- v1 uses a built-in baseline profile and does not yet expose user-configurable target profiles.
- Not tied to Stylelint deprecation behavior.

## Complexity and Performance Notes

- Time: O(number of policy-checkable nodes)
- Memory: shared compatibility policy lookup structures
