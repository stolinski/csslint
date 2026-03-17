# Rule: `no_deprecated_features`

## Intent

Report use of CSS features marked deprecated or disallowed by project compatibility policy for configured targets.

## Compatibility Tag

`native`

## Algorithm Summary

1. Visit declarations, at-rules, and selector constructs covered by policy.
2. Match feature usage against project compatibility/deprecation table.
3. Evaluate match under active target profile.
4. Report deprecated or unsupported usage with actionable message.

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
- Not tied to Stylelint deprecation behavior.

## Complexity and Performance Notes

- Time: O(number of policy-checkable nodes)
- Memory: shared compatibility policy lookup structures
