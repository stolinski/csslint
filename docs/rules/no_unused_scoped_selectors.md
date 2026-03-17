# Rule: `no_unused_scoped_selectors`

Status: **deferred plugin candidate (not in core v1 defaults)**

## Intent

Detect likely unused selectors inside scoped component styles while minimizing false positives.

## Compatibility Tag

`native`

## Algorithm Summary (plugin path)

1. Run only in scoped style contexts.
   - includes Vue `<style scoped>`, Vue `<style module>`, and Svelte `<style>` in v1
2. Read template usage facts from a template-usage provider.
3. Compare scoped selector candidates against template usage with confidence scoring.
4. Report only high-confidence likely-unused selectors.

Provider contract reference:

- `docs/template-usage-provider-spec-v1.md`

## Config Options and Defaults

```json
{
  "level": "warn"
}
```

Expected to be configured through plugin rule namespace when plugin loading is enabled.

## Default Severity

`warn`

## Fix Support and Safety

- Fix support: `none`
- Rationale: removing selectors can break styling and requires stronger confidence than v1 provides.

## Known Divergences from Stylelint

- Native framework-aware rule; no direct Stylelint equivalent.
- Stylelint core does not provide this as a built-in rule.

## Complexity and Performance Notes

- Time: approximately O(number of scoped selector candidates + provider lookup)
- Memory: bounded by selector candidate index and template usage sets

See `docs/plugin-surface-v1.md` for extension architecture.
