# Test Quality Audit Tracking

Last updated: 2026-03-19
Scope: Rust test files in this repo (`crates/**/tests/**/*.rs` plus `#[cfg(test)]` modules in crate sources).

## Rubric and scoring

- Dimensions: Correctness, Edge Cases, Error Paths, Type-Level Safety, Isolation, Description Quality, Structural Organization, Regression Detection.
- Weight mapping used for weighted score: `High=3`, `Medium=2`, `Low-Medium=1.5`.
- `Type-Level Safety` was marked `N/A` for all audited files and excluded from denominator.
- Weighted score formula per file: `sum(score * weight) / sum(5 * weight) * 100`, rounded to nearest integer.
- Rating bands:
  - `Excellent` 87-100 (No action needed)
  - `Good` 70-86 (Low priority improvements)
  - `Adequate` 50-69 (Medium priority, schedule in current phase)
  - `Weak` 30-49 (High priority rewrite)
  - `Insufficient` <30 (Critical)

## Status summary

- Files audited: `32/32`
- Files updated: `31/32`
- Ratings: `8 Excellent`, `24 Good`, `0 Adequate`, `0 Weak`, `0 Insufficient`
- Open questions logged: `0` (both previously open items resolved)

## Markdown hygiene for this audit

- Keep future test-quality audit iterations in this same file; do not create per-wave/per-crate audit markdown files.
- Add new rows/notes in place and keep only active decisions + resolved-decision summaries.
- If detailed intermediate notes are needed during execution, keep them in ephemeral `scratch/` files and remove them before merge.

## Integration test files (`crates/**/tests/**/*.rs`)

| File | Updated | Corr | Edge | Err | Type | Iso | Desc | Struct | Regr | Score | Rating | Notes |
|---|---|---:|---:|---:|---|---:|---:|---:|---:|---:|---|---|
| `crates/csslint-cli/tests/exit_codes_and_reporters.rs` | Yes | 5 | 5 | 5 | N/A | 4 | 4 | 4 | 5 | 94 | Excellent | Added JSON `fix` contract checks; tightened unterminated-directive expectation to deterministic no-op output |
| `crates/csslint-test-harness/tests/smoke_pipeline.rs` | Yes | 4 | 3 | 3 | N/A | 5 | 4 | 4 | 3 | 72 | Good | Strengthened extraction/rule assertions + negative control |
| `crates/csslint-test-harness/tests/parser_integration.rs` | Yes | 4 | 3 | 4 | N/A | 5 | 4 | 4 | 4 | 79 | Good | Added extraction preconditions and tighter malformed assertions |
| `crates/csslint-test-harness/tests/semantic_snapshots.rs` | Yes | 5 | 4 | 3 | N/A | 4 | 4 | 4 | 5 | 84 | Good | Added non-empty corpus and fixture contract checks |
| `crates/csslint-test-harness/tests/extractor_fixtures.rs` | Yes | 5 | 4 | 3 | N/A | 4 | 4 | 4 | 5 | 84 | Good | Added non-empty fixture and single-input enforcement |
| `crates/csslint-test-harness/tests/extractor_malformed.rs` | Yes | 4 | 4 | 5 | N/A | 4 | 4 | 4 | 4 | 84 | Good | Added malformed corpus non-empty + severity guards |
| `crates/csslint-test-harness/tests/malformed_reliability.rs` | Yes | 4 | 4 | 5 | N/A | 4 | 3 | 3 | 4 | 80 | Good | Added aggregate execution invariants |
| `crates/csslint-test-harness/tests/determinism_parallel.rs` | Yes | 5 | 5 | 3 | N/A | 4 | 4 | 4 | 5 | 84 | Good | Expanded thread-count/rerun determinism checks |
| `crates/csslint-test-harness/tests/wave2_compatibility_rules.rs` | Yes | 4 | 4 | 4 | N/A | 4 | 4 | 4 | 4 | 80 | Good | Added unsupported-lang error path; tightened wave2 expectations |
| `crates/csslint-test-harness/tests/wave3_controlled_rules.rs` | Yes | 4 | 4 | 4 | N/A | 4 | 4 | 4 | 4 | 80 | Good | Added unsupported-lang path; stricter wave3 diagnostic assertions |
| `crates/csslint-test-harness/tests/wave4_framework_rules.rs` | Yes | 4 | 4 | 5 | N/A | 4 | 4 | 4 | 4 | 84 | Good | Added Vue `<style src>` skip/warn path and count checks |
| `crates/csslint-test-harness/tests/wave1_fixable_rules.rs` | Yes | 5 | 4 | 4 | N/A | 4 | 4 | 4 | 5 | 87 | Excellent | Added unsupported-lang non-fixable path and idempotency guards |
| `crates/csslint-test-harness/tests/fix_idempotency_fixable_rules.rs` | Yes | 5 | 4 | 4 | N/A | 4 | 4 | 4 | 5 | 87 | Excellent | Added unsupported-lang path and stronger fixability assertions |
| `crates/csslint-test-harness/tests/fix_engine_overlap_matrix.rs` | Yes | 5 | 5 | 5 | N/A | 5 | 4 | 4 | 5 | 96 | Excellent | Added second-pass `rejected == 0` idempotency check |
| `crates/csslint-test-harness/tests/stylelint_compat_harness.rs` | Yes | 4 | 3 | 3 | N/A | 3 | 4 | 4 | 4 | 71 | Good | Added schema/mode + totals-consistency invariants |
| `crates/csslint-test-harness/tests/rule_test_matrix_gate.rs` | Yes | 5 | 4 | 2 | N/A | 4 | 4 | 5 | 5 | 82 | Good | Added explicit non-fixable checks and parser-diagnostic surfacing |
| `crates/csslint-test-harness/tests/native_framework_fixtures.rs` | Yes | 5 | 4 | 4 | N/A | 4 | 4 | 4 | 5 | 87 | Excellent | Added unsupported-lang and `<style src>` extractor-path tests |
| `crates/csslint-test-harness/tests/native_scope_truth_table.rs` | No | 5 | 4 | 2 | N/A | 4 | 4 | 4 | 4 | 76 | Good | Audited; no unambiguous changes required in this pass |
| `crates/csslint-test-harness/tests/native_framework_rule_behavior.rs` | Yes | 4 | 4 | 2 | N/A | 4 | 4 | 4 | 5 | 76 | Good | Added rerun determinism assertion for target-rule counts |
| `crates/csslint-test-harness/tests/native_component_fix_safety.rs` | Yes | 5 | 4 | 3 | N/A | 4 | 4 | 4 | 5 | 84 | Good | Added fix-span-within-style-region and second-pass assertions |
| `crates/csslint-test-harness/tests/perf_smoke.rs` | Yes | 4 | 3 | 3 | N/A | 5 | 4 | 4 | 4 | 75 | Good | Added warmup + median-of-samples perf assertion and CI-configurable runner thresholds |
| `crates/csslint-test-harness/tests/modern_nested_css_regression.rs` | Yes | 4 | 2 | 3 | N/A | 5 | 4 | 3 | 4 | 70 | Good | Added extraction-safety preconditions |
| `crates/csslint-test-harness/tests/nested_selector_duplicate_regression.rs` | Yes | 5 | 4 | 3 | N/A | 5 | 4 | 4 | 5 | 86 | Good | Hardened helper + deterministic rerun test |

## Inline `#[cfg(test)]` modules

| File | Updated | Corr | Edge | Err | Type | Iso | Desc | Struct | Regr | Score | Rating | Notes |
|---|---|---:|---:|---:|---|---:|---:|---:|---:|---:|---|---|
| `crates/csslint-core/src/lib.rs` | Yes | 4 | 4 | 2 | N/A | 5 | 4 | 4 | 3 | 72 | Good | Added line-index offset clamp + standalone CR handling tests |
| `crates/csslint-extractor/src/lib.rs` | Yes | 4 | 4 | 4 | N/A | 5 | 4 | 4 | 4 | 82 | Good | Added case-insensitive tag parsing and unclosed-style warning path |
| `crates/csslint-parser/src/lib.rs` | Yes | 4 | 4 | 3 | N/A | 4 | 4 | 4 | 4 | 76 | Good | Added string-literal brace handling + property coverage expansion |
| `crates/csslint-semantic/src/lib.rs` | Yes | 4 | 4 | 3 | N/A | 4 | 4 | 4 | 4 | 76 | Good | Added at-rule head-behavior assertions, multi-`:global()` scope-index dedupe, and malformed global-token fallback coverage |
| `crates/csslint-rules/src/lib.rs` | Yes | 5 | 4 | 5 | N/A | 4 | 4 | 4 | 5 | 91 | Excellent | Added severity-off override suppression test |
| `crates/csslint-fix/src/lib.rs` | Yes | 5 | 4 | 4 | N/A | 5 | 4 | 4 | 5 | 89 | Excellent | Added invalid/out-of-bounds span hardening test |
| `crates/csslint-config/src/lib.rs` | Yes | 5 | 4 | 5 | N/A | 4 | 4 | 4 | 4 | 87 | Excellent | Added missing-file and directory config path validation tests |
| `crates/csslint-cli/src/main.rs` | Yes | 4 | 4 | 4 | N/A | 3 | 4 | 4 | 4 | 78 | Good | Added parser tests for missing/unsupported `--format` + duplicate `--config` |
| `crates/csslint-test-harness/src/stylelint_compat.rs` | Yes | 4 | 4 | 3 | N/A | 5 | 4 | 4 | 4 | 79 | Good | Added mixed-case parsing, nested disable-depth, EOF-open range, and unterminated-comment suppression parser coverage |

## Resolved questions

- `OQ-001` (`crates/csslint-cli/tests/exit_codes_and_reporters.rs`): Resolved to deterministic no-op behavior for unterminated stylelint directive comments in v1 (exit code `0`, empty diagnostics, no internal errors).
- `OQ-002` (`crates/csslint-test-harness/tests/perf_smoke.rs`): Resolved to explicit CI runner-class configuration via perf job env vars (`CSSLINT_PERF_SMOKE_MAX_SECS`, sample/iteration knobs).

## Verification commands and status

Ran after applying audit-driven test updates:

1. `cargo fmt --all` -> pass
2. `cargo clippy --workspace --all-targets -- -D warnings` -> pass (after resolving two `needless_lifetimes` clippy errors in `crates/csslint-parser/src/lib.rs`)
3. `cargo check --workspace` -> pass
4. `cargo build --workspace` -> pass
5. `cargo test --workspace` -> pass
6. `cargo test -p csslint-test-harness --test smoke_pipeline` -> pass

Relevant CI-parity suites:

- `cargo test -p csslint-test-harness --test extractor_malformed --test malformed_reliability` -> pass
- `cargo test -p csslint-test-harness --test determinism_parallel` -> pass
- `cargo test -p csslint-test-harness --test native_framework_fixtures --test native_scope_truth_table --test native_framework_rule_behavior --test native_component_fix_safety` -> pass

Follow-up pass for OQ resolution and Adequate->Good upgrades:

- `cargo test -p csslint-cli --test exit_codes_and_reporters malformed_or_unterminated_stylelint_directives_do_not_crash_e2e` -> pass
- `cargo test -p csslint-test-harness --test perf_smoke` -> pass
- `cargo test -p csslint-semantic` -> pass
- `cargo test -p csslint-test-harness --lib` -> pass
- `cargo clippy --workspace --all-targets -- -D warnings` -> pass
- `cargo test --workspace` -> pass

## Implementation bug fixes

- No confirmed production implementation bugs were found by the audit updates.
- One non-functional lint compliance fix was applied in parser helper signatures to satisfy clippy (`needless_lifetimes`).
