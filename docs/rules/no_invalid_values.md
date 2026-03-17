# Rule: `no_invalid_values`

## Intent

Report declarations whose values are syntactically invalid for the given property in the v1 supported subset.

## Compatibility Tag

`inspired`

## Algorithm Summary

1. Visit each declaration node.
2. Skip declarations that are out of v1 value-validation scope.
3. Validate high-confidence property/value pairs using parser and semantic metadata.
4. Report invalid value diagnostics at declaration value span.

## Config Options and Defaults

```json
{
  "level": "error"
}
```

v1 scope is intentionally narrow and high-confidence.

## Default Severity

`error`

## Fix Support and Safety

- Fix support: `none`
- Rationale: value correction is ambiguous without intent.

## Known Divergences from Stylelint

- This rule is a subset of Stylelint value-unknown behavior.
- Complex custom grammars and advanced function validation are deferred.

## Complexity and Performance Notes

- Time: O(number of declarations in supported subset)
- Memory: bounded by parser/semantic structures already built
