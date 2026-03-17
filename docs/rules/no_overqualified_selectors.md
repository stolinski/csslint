# Rule: `no_overqualified_selectors`

## Intent

Disallow selectors that combine type selectors with class/id qualifiers in ways that reduce maintainability without adding value.

## Compatibility Tag

`compatible`

## Algorithm Summary

1. Visit selector parts from semantic model.
2. Detect qualifying patterns (for example, `div.foo`, `a#id`) based on rule policy.
3. Ignore contexts explicitly exempted by v1 policy.
4. Report overqualified patterns at selector span.

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

## Complexity and Performance Notes

- Time: O(number of selector parts)
- Memory: constant per selector evaluation
