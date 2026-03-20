# CSS Linter Plan Index

This folder contains the planning docs for the v1 next-gen CSS linter. These are ordered for reading and execution, but no implementation is assumed yet.

## Recommended Order

1. [01-scope-and-success-criteria.md](./01-scope-and-success-criteria.md)
2. [02-workspace-and-crate-layout.md](./02-workspace-and-crate-layout.md)
3. [03-extractor-and-source-mapping.md](./03-extractor-and-source-mapping.md)
4. [04-parser-and-semantic-model.md](./04-parser-and-semantic-model.md)
5. [05-rule-engine.md](./05-rule-engine.md)
6. [06-first-rule-batch.md](./06-first-rule-batch.md)
7. [07-fix-engine.md](./07-fix-engine.md)
8. [08-cli-and-config.md](./08-cli-and-config.md)
9. [09-stylelint-compatibility-harness.md](./09-stylelint-compatibility-harness.md)
10. [10-native-framework-suite.md](./10-native-framework-suite.md)
11. [11-performance-and-hardening.md](./11-performance-and-hardening.md)

## Dependency Map

Core dependency flow:

`01 -> 02 -> 03 -> 04 -> 05 -> 06 -> 07 -> 08 -> 09/10 -> 11`

Detailed dependencies:

- `02` depends on `01`
- `03` depends on `02`
- `04` depends on `03`
- `05` depends on `04`
- `06` depends on `05`
- `07` depends on `05` and `06`
- `08` depends on `05`, `06`, and `07`
- `09` depends on `06` and `08`
- `10` depends on `03`, `04`, and `06`
- `11` depends on all prior steps (especially `05` through `10`)

## Question-Phase Reading Focus

If you are still clarifying decisions before coding, focus in this order:

1. `01` for scope and release gates
2. `06` for exact v1 rule commitments
3. `09` for Stylelint test import boundaries
4. `10` for Vue/Svelte scope semantics
5. `11` for performance acceptance criteria

## Output of This Plan

When planning is complete, this folder should let you do three things quickly:

- lock v1 scope and avoid scope creep
- convert each step into backlog tasks with clear dependencies
- start implementation without re-deciding architecture fundamentals

## Related Readiness Doc

- [Replacement Readiness Checklist](../replacement-readiness-checklist.md)
- [Documentation Index](../README.md)
