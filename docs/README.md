# Documentation Index

This repository keeps documentation in a few stable buckets to avoid sprawl.

## Core specs (source of truth)

- Plan and execution order: `docs/plan/README.md`
- Rule catalog: `docs/rule-catalog-v1.md`
- Rule specs: `docs/rules/README.md`
- JSON reporter contract: `docs/json-output-schema-v1.md`
- Scope and policy docs: `docs/open-questions.md`, `docs/deferrals-v1.md`,
  `docs/success-metrics.md`, `docs/vue-style-policy-v1.md`

## Operational docs

- Test-quality audit tracking: `docs/ops/test-quality-audit-tracking.md`

## Area READMEs

- Stylelint compatibility fixtures: `tests/compat/stylelint/README.md`
- Native framework fixtures: `tests/native/README.md`
- Performance corpora and benchmarks: `tests/perf/README.md`

## Hygiene rules

- Reuse existing docs first; append sections instead of creating one-off markdown files.
- Keep recurring audit/process notes under `docs/ops/`.
- Keep fixture usage notes in a single area README (`tests/*/README.md`).
