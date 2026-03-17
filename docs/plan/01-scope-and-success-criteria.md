# Step 1: Scope and Success Criteria

## Purpose

Define exactly what v1 includes, what it intentionally excludes, and how we decide whether the release is successful. This document is the contract that prevents scope creep and keeps engineering decisions aligned with performance and reliability goals.

## Product Goals (v1)

1. Deliver a next-gen CSS linter that is materially faster than Stylelint, with comparative benchmark reporting on every release.
2. Support `.css`, `.vue` (`<style>`), and `.svelte` (`<style>`) first-class.
3. Avoid PostCSS/custom syntax plumbing by using Lightning CSS as the core parser.
4. Build a deterministic, typed rule engine with stable diagnostics and safe autofix.
5. Ship with zero/low-config defaults so teams can adopt quickly.

## Non-Goals (v1)

- SCSS and LESS parsing.
- Full Stylelint compatibility.
- Arbitrary JS plugin execution.
- PostCSS plugin ecosystem interoperability.
- Deep template-aware dead selector analysis in core ruleset (deferred; plugin-surface candidate).

## Rule Scope Contract

Each rule must be tagged with one of:

- `compatible`: close behavior target to Stylelint equivalent.
- `inspired`: similar intent, implementation adapted to this architecture.
- `native`: framework-aware or project-specific behavior not present in Stylelint.

### Initial v1 Rule Set (target 10)

- `no_unknown_properties` (`compatible`)
- `no_invalid_values` (`inspired` in v1 subset)
- `no_duplicate_selectors` (`compatible`)
- `no_duplicate_declarations` (`compatible`)
- `no_empty_rules` (`compatible`)
- `no_legacy_vendor_prefixes` (`compatible` for prop/value variants)
- `no_overqualified_selectors` (`compatible`)
- `prefer_logical_properties` (`native`)
- `no_global_leaks` (`native`)
- `no_deprecated_features` (`native`, target-aware)

## Definition of "Target-Aware"

Target-aware linting is split into two concerns:

- **Syntax/spec validity**: does this property/value parse and match known CSS grammar?
  - Primary source: Lightning CSS parsing and value understanding.
- **Compatibility policy**: is this feature unsupported/deprecated for configured browser targets?
  - Primary source: project-owned compatibility policy rules (optionally aided by Lightning target info).

Important: transpilation capability does not automatically equal lint policy. We still need explicit diagnostics policy for unsupported/deprecated usage.

## Success Metrics

## Performance

- Comparative benchmark reports against Stylelint are required for every release candidate.
- No fixed speedup multiplier is required in v1; performance trend and ratio must be published.
- Bound peak memory per file count tier (baseline and max thresholds documented in perf plan).

## Correctness and Reliability

- Deterministic diagnostics ordering across runs and platforms.
- No panics on malformed files (graceful diagnostics).
- Accurate line/column mapping back to original source files.
- `--fix` idempotency: second run produces no additional edits.

## Product Experience

- Zero-config run is useful and low-noise.
- `--format json` stable enough for CI machine parsing.
- Config model stays minimal and predictable.

## v1 Release Gates

Release cannot ship unless all gates pass:

1. **Core correctness gate**
   - Rule tests green.
   - Mapping and fix safety tests green.
2. **Compatibility gate**
   - Imported Stylelint subset is fully reported and does not regress without documented deferral/divergence updates.
3. **Framework gate**
   - Vue/Svelte native suite green.
4. **Performance gate**
   - Benchmark and regression budgets pass and comparison report is published.
5. **Determinism gate**
   - Repeat runs produce identical diagnostics and fixes.

## Out-of-Scope Handling (Deferral Log)

Any requested item outside this document must be logged as a deferral entry with:

- title
- reason for deferral
- impact
- proposed milestone (`v1.5`, `v2`, etc.)
- owner

This keeps roadmap pressure visible without destabilizing v1.

## Decision Log (must be locked before implementation)

- Rule list and compatibility tags finalized.
- Default preset contents finalized.
- JSON output schema versioned (`v1`).
- Severity defaults (`warn` vs `error`) decided per rule.
- Target configuration defaults decided (`defaults` browsers profile or explicit target baseline).

## Deliverables

- `docs/plan/01-scope-and-success-criteria.md` (this file)
- `docs/rule-catalog-v1.md` (per-rule tags and defaults)
- `docs/success-metrics.md` (numerical thresholds)
- `docs/deferrals-v1.md` (living backlog of non-v1 items)
- `docs/open-questions.md` (tracked open decisions and defaults)
- `docs/json-output-schema-v1.schema.json` (machine-validated output contract)
- `docs/json-output-schema-v1.md` (human-readable schema companion)
- `docs/vue-style-policy-v1.md` (concrete v1 Vue `<style module>` and `<style src>` behavior)
- `docs/plugin-surface-v1.md` (typed extension model for future template-aware rules)
- `docs/template-usage-provider-spec-v1.md` (typed provider IO and confidence contract)

## Exit Criteria

- Team sign-off on goals, non-goals, and release gates.
- Rule catalog and config defaults frozen for v1.
- CI jobs mapped to every release gate.

## Main Risks and Mitigations

- **Risk**: scope creep from compatibility requests.
  - **Mitigation**: strict compatible/inspired/native labels and deferral process.
- **Risk**: perf goals missed due to late architecture changes.
  - **Mitigation**: commit to single-parse/single-traversal architecture and track perf from first runnable prototype.
- **Risk**: low trust from incorrect locations/fixes.
  - **Mitigation**: treat source mapping and fix idempotency as hard gates, not polish work.
