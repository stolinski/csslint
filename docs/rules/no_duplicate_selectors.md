# Rule: `no_duplicate_selectors`

## Intent

Disallow duplicate selectors within the same rule context.

## Compatibility Tag

`compatible`

## Algorithm Summary

1. Visit selector nodes in source order.
2. Compute conservative normalized selector key.
3. Include parent at-rule context in duplicate key.
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

- Normalization is conservative to avoid false positives.
- v1 favors fewer false duplicates over aggressive equivalence collapsing.

## Complexity and Performance Notes

- Time: O(number of selectors)
- Memory: hash map of seen selector keys per context
