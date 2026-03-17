# Rule: `no_unknown_properties`

## Intent

Disallow property names that are not recognized CSS properties.

## Compatibility Tag

`compatible`

## Algorithm Summary

1. Visit each declaration node.
2. Skip custom properties (`--*`).
3. Check property name against known property metadata.
4. Report unknown names with original span and stable message.

## Config Options and Defaults

```json
{
  "level": "error"
}
```

v1 has no additional ignore list options.

## Default Severity

`error`

## Fix Support and Safety

- Fix support: `none`
- Rationale: automatic renaming is unsafe and context-dependent.

## Known Divergences from Stylelint

- v1 intentionally omits advanced ignore options.
- Unknown detection is based on project metadata and parser normalization path.

## Complexity and Performance Notes

- Time: O(number of declarations)
- Memory: constant per declaration check plus shared metadata lookup
