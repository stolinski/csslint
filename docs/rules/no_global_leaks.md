# Rule: `no_global_leaks`

## Intent

Prevent accidental global selector leakage from scoped component style blocks.

## Compatibility Tag

`native`

## Algorithm Summary

1. Run only in scoped style contexts.
   - includes Vue `<style scoped>`, Vue `<style module>`, and Svelte `<style>` in v1
2. Inspect selector scope annotations from semantic model.
3. Allow mixed selectors with scoped anchors plus explicit global escapes (`.local :global(.x)`).
4. Report only high-confidence leak patterns: selectors that are entirely global escapes in scoped blocks (global parts only).

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
- Rationale: automatic scope rewrites can alter runtime selector behavior.

## Known Divergences from Stylelint

- Native framework-aware behavior; no direct Stylelint mapping.
- v1 intentionally avoids ambiguous nested inference and only flags global-only escapes.

## Complexity and Performance Notes

- Time: O(number of selectors in scoped blocks)
- Memory: reuses semantic scope metadata, no extra AST pass
