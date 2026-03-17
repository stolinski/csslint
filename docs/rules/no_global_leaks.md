# Rule: `no_global_leaks`

## Intent

Prevent accidental global selector leakage from scoped component style blocks.

## Compatibility Tag

`native`

## Algorithm Summary

1. Run only in scoped style contexts.
   - includes Vue `<style scoped>`, Vue `<style module>`, and Svelte `<style>` in v1
2. Inspect selector scope annotations from semantic model.
3. Allow explicit global escapes (`:global(...)`) when used intentionally.
4. Report high-confidence accidental global patterns.

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

## Complexity and Performance Notes

- Time: O(number of selectors in scoped blocks)
- Memory: reuses semantic scope metadata, no extra AST pass
