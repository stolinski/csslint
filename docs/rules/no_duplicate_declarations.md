# Rule: `no_duplicate_declarations`

## Intent

Disallow duplicate declarations of the same property in a declaration block when they are likely accidental.

## Compatibility Tag

`compatible`

## Algorithm Summary

1. For each rule block, normalize declaration property names.
2. Track seen `(property, value)` pairs in source order.
3. Report only exact duplicate pairs after first occurrence.
4. Preserve distinct-value declarations as fallback patterns.

## Config Options and Defaults

```json
{
  "level": "error"
}
```

## Default Severity

`error`

## Fix Support and Safety

- Fix support: `yes` (safe subset)
- v1 autofix only removes exact duplicate declarations when both property and value match.
- Non-exact duplicates are reported without automatic edits.

## Known Divergences from Stylelint

- v1 fix behavior is intentionally narrower for idempotency safety.
- Advanced duplicate option matrix is deferred.

## Complexity and Performance Notes

- Time: O(number of declarations per block)
- Memory: temporary property-group map per block
