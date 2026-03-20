# Rule: `no_duplicate_selectors`

## Intent

Disallow duplicate selectors within the same rule context.

## Compatibility Tag

`compatible`

## Algorithm Summary

1. Visit selector nodes in source order.
2. Compute conservative normalized selector key.
3. Use semantic normalized-selector indexes to find repeated keys.
4. Report duplicates after the first occurrence.

## Config Options and Defaults

```json
{
  "level": "error"
}
```

## Default Severity

`error`

## Fix Support and Safety

- Fix support: `none`
- Rationale: auto-removal can change cascade and specificity outcomes.

## Known Divergences from Stylelint

- Imported compatibility suite: `no-duplicate-selectors` (`tests/compat/stylelint/imported/no-duplicate-selectors.json`).
- Normalization is conservative to avoid false positives.
- v1 favors fewer false duplicates over aggressive equivalence collapsing.
- Duplicate keys are partitioned by nested selector ancestry and at-rule context to avoid cross-scope false positives in modern nested CSS.
- Core inline suppressions are supported (`csslint-disable*` and `stylelint-disable*` aliases); full directive-option parity remains a tracked compatibility follow-up.

## Complexity and Performance Notes

- Time: O(number of selectors)
- Memory: hash map of seen selector keys per context
