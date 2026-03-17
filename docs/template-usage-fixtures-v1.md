# Template Usage Fixture Format (v1)

## Purpose

Define a small, consistent fixture shape for testing the `template_usage` provider.

This format is used by provider-level tests and plugin-path rule tests that consume provider output.

## Directory Layout

```text
tests/native/template-usage/
  vue/
    static-class-id/
      input.vue
      expected.json
    medium-ternary-classes/
      input.vue
      expected.json
  svelte/
    static-class-directive/
      input.svelte
      expected.json
    low-dynamic-expression/
      input.svelte
      expected.json
  shared/
    failed-recoverable/
      input.vue
      expected.json
```

## Fixture Files

- `input.vue` or `input.svelte`: full component source text
- `expected.json`: expected `TemplateUsageOutput` in normalized JSON form

Optional:

- `notes.md`: short reasoning for unusual cases

## `expected.json` Shape

```json
{
  "status": "Complete",
  "facts": [
    {
      "kind": "Class",
      "name": "button",
      "confidence": "High",
      "source": "StaticAttribute",
      "span": { "start": 42, "end": 48 }
    }
  ],
  "unknownRegions": [],
  "diagnostics": []
}
```

## Enum Values

- `status`: `Complete`, `Partial`, `FailedRecoverable`
- `kind`: `Class`, `Id`
- `confidence`: `High`, `Medium`, `Low`
- `source`:
  - `StaticAttribute`
  - `FrameworkDirectiveLiteral`
  - `BindingLiteralBranch`
  - `DynamicExpressionHeuristic`

## Normalization Rules

Before comparing actual vs expected output:

1. Sort `facts` by `(kind, name, span.start, span.end, confidence)`.
2. Sort `diagnostics` by `(span.start?, span.end?, message)`.
3. Keep exact enum casing.
4. Use source-file byte offsets for span values.

## Required Case Categories

Each framework should include at least one case for:

1. static class/id extraction (`High`)
2. finite literal branch extraction (`Medium`)
3. dynamic expression fallback (`Low`)
4. recoverable parser failure (`FailedRecoverable`)

## Rule-Integration Fixture Pairing

For plugin-path `no_unused_scoped_selectors` tests:

- pair a provider fixture with a corresponding style-selector fixture
- assert the rule only reports unused selectors when provider output allows high-confidence conclusions
