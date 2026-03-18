# Rule: `no_invalid_values`

## Intent

Report declarations whose values are syntactically invalid for the given property in the v1 supported subset.

## Compatibility Tag

`inspired`

## Algorithm Summary

1. Visit each declaration node.
2. Skip declarations that are out of v1 value-validation scope.
   - unknown properties and custom properties
   - complex/dynamic values (`var()`, `env()`, `calc()`, `min()`, `max()`, `clamp()`, `attr()`)
3. Normalize value checks by trimming and removing a trailing `!important`.
4. Validate only high-confidence property/value subsets:
   - keyword sets: `display`, `position`, `visibility`, `box-sizing`, `overflow*`, `flex-direction`, `flex-wrap`
   - numeric range: `opacity` in `[0, 1]`
5. Report invalid subset matches at declaration span.

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

- Imported compatibility suite: partial `declaration-property-value-no-unknown` (`tests/compat/stylelint/imported/declaration-property-value-no-unknown.json`).
- This rule is a subset of Stylelint value-unknown behavior.
- Multi-token grammar validation is intentionally skipped to avoid noisy false positives.
- Complex custom grammars and advanced function validation are deferred.

## Complexity and Performance Notes

- Time: O(number of declarations in supported subset)
- Memory: bounded by parser/semantic structures already built
