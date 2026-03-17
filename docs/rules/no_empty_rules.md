# Rule: `no_empty_rules`

## Intent

Disallow empty rule blocks.

## Compatibility Tag

`compatible`

## Algorithm Summary

1. Visit each rule block node.
2. Ignore whitespace and comments when determining block content.
3. Report blocks with no effective declarations or nested content.

## Config Options and Defaults

```json
{
  "level": "warn"
}
```

## Default Severity

`warn`

## Fix Support and Safety

- Fix support: `yes`
- v1 autofix removes the entire empty block when span boundaries are trusted.
- If span integrity is uncertain, skip fix and keep diagnostic.

## Known Divergences from Stylelint

- v1 preserves strict offset safety over aggressive normalization around comments.

## Complexity and Performance Notes

- Time: O(number of rule blocks)
- Memory: constant per block
