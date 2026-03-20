# Native Test Fixtures

This directory hosts framework-native fixture corpora used by `csslint-test-harness`
integration tests.

## Fixture sets

- `extractor/`
  - Extractor behavior and source-mapping fixtures for `.css`, `.vue`, and `.svelte` inputs.
  - Each case includes source input (`input.css`, `input.vue`, or `input.svelte`) and
    `expected.json` containing extracted style block metadata and expected diagnostics.
- `extractor-malformed/`
  - Malformed component inputs used to verify extractor reliability.
  - Every corpus file should execute without panic and emit controlled extractor diagnostics.
- `semantic/`
  - Parser + semantic integration snapshots.
  - Cases include source inputs plus `expected.json` for normalized selectors,
    selector-part scope annotations, declaration properties, and index summaries.
- `template-usage/`
  - Provider-fixture scaffolding described in `docs/template-usage-fixtures-v1.md`.
  - Case directories include `input.vue` or `input.svelte` with `expected.json` and can
    expand as provider behavior is implemented.
