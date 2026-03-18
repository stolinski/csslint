# Rule: `no_overqualified_selectors`

## Intent

Disallow selectors that combine type selectors with class/id qualifiers in ways that reduce maintainability without adding value.

## Compatibility Tag

`compatible`

## Algorithm Summary

1. Visit selectors in source order.
2. Split selectors into compound segments around combinators and commas.
3. Detect segments that start with a type selector and later include class/id qualifiers (for example, `div.foo`, `a#id`, `:global(button#cta)`).
4. Report overqualified selectors at selector span.

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
- Rationale: selector rewrites can change specificity and matching behavior.

## Known Divergences from Stylelint

- v1 keeps a simpler option surface and conservative matching.
- Matching uses conservative string segmentation rather than full selector-grammar option coverage.

## Complexity and Performance Notes

- Time: O(number of selector parts)
- Memory: constant per selector evaluation
