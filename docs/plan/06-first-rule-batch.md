# Step 6: First Rule Batch

## Purpose

Ship a practical v1 rule set in phases, prioritizing high impact, fix safety, and compatibility where it matters most.

## Implementation Waves

## Wave 1 (Quick Wins, Fixable)

1. `no_empty_rules`
2. `no_duplicate_declarations`
3. `no_legacy_vendor_prefixes` (property + value)

Why first:

- straightforward semantics
- high user value
- good early fix engine validation

## Wave 2 (Compatibility Priority)

4. `no_duplicate_selectors`
5. `no_unknown_properties`
6. `no_overqualified_selectors`

Why second:

- strong overlap with existing Stylelint expectations
- good candidate for imported fixture parity

## Wave 3 (Controlled Complexity)

7. `no_invalid_values` (v1 subset)
8. `no_deprecated_features` (target-aware policy)

Why third:

- value grammar and compatibility policy are broad and need incremental rollout

## Wave 4 (Framework Native)

9. `no_global_leaks`
10. `prefer_logical_properties`

Why fourth:

- requires stable scope model and framework-native fixtures

## Rule-Level Specification Template (Required)

Every rule must ship with a spec file containing:

- intent
- compatibility tag (`compatible`, `inspired`, `native`)
- algorithm summary
- config options and defaults
- severity default
- fix support and safety constraints
- known divergences from Stylelint (if any)
- complexity/perf notes

## V1 Rule/Test Matrix

Each rule must be tested across dimensions below:

- input type: `.css`, `.vue`, `.svelte`
- mode: lint-only, fix
- source: imported Stylelint fixtures, native fixtures
- output: diagnostics, spans, messages, fix output

### Example Matrix Entry

`no_duplicate_selectors`

- `.css`: imported + native
- `.vue`: native scoped/non-scoped
- `.svelte`: native mixed `:global()`
- fix: none
- required: deterministic duplicate detection with selector normalization

## Compatibility Mapping (Initial)

- `no_empty_rules` <- Stylelint `block-no-empty`
- `no_duplicate_selectors` <- `no-duplicate-selectors`
- `no_unknown_properties` <- `property-no-unknown`
- `no_legacy_vendor_prefixes` <- `property-no-vendor-prefix`, `value-no-vendor-prefix`
- `no_overqualified_selectors` <- `selector-no-qualifying-type`
- `no_duplicate_declarations` <- `declaration-block-no-duplicate-properties`
- `no_invalid_values` <- partial from `declaration-property-value-no-unknown`

## Rule-Specific Notes

## `no_invalid_values` v1 Scope

- focus on high-confidence invalid values
- avoid overaggressive parsing in ambiguous cases
- leverage Lightning parsing insights where possible
- defer complex custom grammar extensions to post-v1

## Deferred Plugin Candidate: `no_unused_scoped_selectors`

- not in core v1 default rule set
- moved to plugin-surface candidate for template-aware analysis
- see `docs/plugin-surface-v1.md`

## Done Criteria per Rule

A rule is done only when all are true:

1. Rule spec document committed.
2. Unit and integration tests pass.
3. Compatibility fixtures (if applicable) pass threshold.
4. Framework fixtures (if applicable) pass.
5. Fix idempotency validated for fixable rules.

## Sequencing and Staffing

- Implement two to three rules at a time max.
- Pair one compatibility-heavy rule with one native rule to keep both tracks moving.
- Do not start new rule before previous wave reaches green status in CI.

## Deliverables

- Rule implementations for all v1 rules.
- Per-rule spec docs in `docs/rules/`.
- Rule matrix tracking file in `docs/plan/rule-test-matrix.md`.

## Exit Criteria

- 10 v1 rules implemented and passing required matrix coverage.
- No unresolved critical false-positive issue in default preset.
- Fixable rules meet idempotency constraints.

## Risks and Mitigations

- **Risk**: spending too long chasing full Stylelint parity.
  - **Mitigation**: enforce compatible/inspired/native labeling and divergence docs.
- **Risk**: noisy framework rules hurt adoption.
  - **Mitigation**: conservative defaults and dedicated native fixture validation.
