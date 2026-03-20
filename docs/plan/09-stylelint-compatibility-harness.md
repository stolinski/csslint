# Step 9: Stylelint Compatibility Harness

## Purpose

Reuse high-value Stylelint rule tests as a compatibility corpus for overlapping rules, without inheriting PostCSS/custom syntax complexity that is explicitly out of scope for v1.

## Strategy Summary

1. Pin one Stylelint commit SHA.
2. Import only selected rule test files.
3. Translate fixture cases into project-native fixture format.
4. Maintain explicit skip manifest for unsupported cases.
5. Publish per-rule compatibility pass rates in CI.

Repository implementation files:

- `tests/compat/stylelint/source-pin.json`
- `tests/compat/stylelint/suite-map.json`

## Initial Rule Suites to Import

From Stylelint repository test files:

- `block-no-empty`
- `no-duplicate-selectors`
- `property-no-unknown`
- `property-no-vendor-prefix`
- `value-no-vendor-prefix`
- `declaration-block-no-duplicate-properties`
- `selector-no-qualifying-type`
- partial `declaration-property-value-no-unknown`

## What to Import vs Skip

## Import

- plain CSS accept/reject cases
- deterministic autofix expectations
- parser-agnostic structural rule cases

## Skip (v1)

- SCSS/LESS custom syntax cases
- PostCSS plugin integration tests
- stylelint directive-comment behavior beyond core `stylelint-disable*`/`stylelint-enable*` compatibility unless implemented
- arbitrary custom syntax adapters

## Fixture Translation Pipeline

### Source format

Stylelint uses JS `testRule` structures.

### Target format (project-native)

```yaml
rule: no_duplicate_selectors
config:
  level: error
cases:
  - name: duplicate simple selector
    input: "a {} b {} a {}"
    expected:
      diagnostics:
        - rule: no_duplicate_selectors
          severity: error
          message_contains: "Duplicate selector"
          span:
            line: 1
            column: 11
      fixed: null
```

### Translation rules

- map Stylelint rule name to local rule ID.
- map severity to local level.
- normalize message checks to `message_contains` where exact wording differs.
- keep exact span assertions for stable compatibility confidence.

## Skip Manifest

Maintain `tests/compat/stylelint/skip-manifest.yaml` with fields:

- source file
- case description/name
- reason code (`custom_syntax`, `scss_less`, `directive_comments`, `postcss_integration`, `unsupported_option`)
- note

Skip entries should be explicit, reviewable, and counted in reports.

Validate manifest integrity and reason-code counts with:

- `python3 scripts/validate_stylelint_skip_manifest.py`

## Compatibility Metrics

Track:

- pass/total per rule
- pass/total global
- skipped/total with breakdown by reason code
- fix pass rate for fixable imported cases

Publish these metrics in CI summaries and release notes.

Implementation command:

- `cargo run -p csslint-test-harness --bin stylelint_compat_report -- --mode <fast|full> --output <path> [--baseline <path>] [--enforce-ratchet]`

## CI Integration

Run compatibility in a dedicated lane:

- `compat-fast`: curated core suite for PR checks
- `compat-full`: full imported subset on main/nightly

Do not block unrelated changes on known-skipped cases.

Current ratchet baseline path:

- `tests/compat/stylelint/baseline/compat-summary.json`

## Governance and Upgrades

- Pin source commit SHA for reproducibility.
- Upgrade source SHA deliberately (not continuously).
- On upgrade, re-run importer and diff:
  - added cases
  - removed cases
  - changed expectations

## Documentation Requirements

For each compatible/inspired rule, document:

- imported suite coverage
- known divergences
- skip reasons that affect that rule

## Deliverables

- importer script (JS or Rust utility)
- native fixture schema and runner
- curated imported fixtures
- skip manifest
- compatibility dashboard output in CI

## Exit Criteria

- All selected imported suites execute in CI.
- Pass thresholds met for targeted rules.
- Skips are explicit and intentional.

## Risks and Mitigations

- **Risk**: importer drift as upstream tests change.
  - **Mitigation**: pin SHA and controlled upgrade cadence.
- **Risk**: spending time on non-v1 syntax compatibility.
  - **Mitigation**: strict skip policy aligned to v1 non-goals.
