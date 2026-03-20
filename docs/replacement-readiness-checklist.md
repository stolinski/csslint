# Replacement Readiness Checklist (Stylelint Migration)

## Purpose

Define what must be true before CSSLint can be called a strong replacement for Stylelint in most projects.

## Current Snapshot

- Core rule engine, fix safety, determinism, malformed-input reliability, and framework suites are in CI.
- Stylelint compatibility is real but still curated (not full upstream parity).
- Modern CSS corpus has expanded substantially, but real-world project breadth is still limited.

## Must-Have Gates for "Replacement Ready"

| Area | Gate | Status |
| --- | --- | --- |
| Core correctness | Full workspace tests green and deterministic output | in place |
| Fix safety | Idempotent fixes, overlap resolution, no template/script corruption | in place |
| Framework support | Vue/Svelte extraction + scoped behavior + mapping tests | in place |
| Compatibility breadth | Large imported Stylelint suite coverage with ratchet | partial |
| Rule option parity | Common Stylelint option patterns covered or intentionally mapped | partial |
| CLI migration ergonomics | `.csslintignore`, inline suppressions, JSON/pretty/reporting e2e | partial |
| Real-world confidence | Dogfood corpus from multiple real repos with false-positive ratchet | missing |
| Hardening depth | Fuzz/property-based parser+semantic robustness | missing |
| Platform parity | Linux/macOS/Windows CI matrix | partial |

## What Is Not Fully Tested Yet

### 1) Compatibility breadth is still limited

- Imported compatibility map is limited to selected suites (`tests/compat/stylelint/suite-map.json`).
- Full baseline is still a subset (`tests/compat/stylelint/baseline/compat-summary.json`).
- Known skipped categories remain (`tests/compat/stylelint/skip-manifest.yaml`):
  - `scss_less`
  - `custom_syntax`
  - `postcss_integration`
  - `unsupported_option`

### 2) Advanced Stylelint option behavior is not covered

- Several rule docs explicitly note subset behavior or deferred option matrices:
  - `docs/rules/no_unknown_properties.md`
  - `docs/rules/no_duplicate_declarations.md`
  - `docs/rules/no_invalid_values.md`
  - `docs/rules/no_overqualified_selectors.md`

### 3) CLI feature coverage is stronger at unit level than e2e level

- New ignore/suppression behavior has solid unit coverage in `crates/csslint-cli/src/main.rs` tests.
- Binary integration tests (`crates/csslint-cli/tests/exit_codes_and_reporters.rs`) focus on exit/reporters and do not yet exhaustively exercise all ignore/suppression/fix interaction paths.

### 4) Fuzz/property-based hardening is not implemented

- Hardening plan calls for fuzz targets (`docs/plan/11-performance-and-hardening.md`), but no fuzz targets currently exist in repo.

### 5) Platform matrix coverage is not broad

- CI includes dedicated macOS/Windows migration lanes (`cross-platform-migration`) for workspace check + CLI migration e2e + compatibility harness (`.github/workflows/rust-ci.yml`).
- Full parity is still partial because not every lane runs on all platforms.

### 6) Plugin API is minimally tested

- `csslint-plugin-api` currently has no dedicated test suite.

## Priority Gap-Closing Plan

1. Expand imported compatibility suites and shrink skip reasons with explicit milestones.
2. Add CLI e2e tests for `.csslintignore`, `--ignore-path`, `csslint-disable*`, `stylelint-disable*`, and `--fix` interaction.
3. Add parser/semantic fuzz targets and run them in CI/nightly.
4. Add macOS and Windows CI jobs for key lanes (check, unit, compat-fast).
5. Build a real-repo regression pack and ratchet false-positive rates per rule.

## Exit Signal

Treat CSSLint as a practical Stylelint replacement when:

- compatibility corpus is broad and stable,
- skipped compatibility reasons are mostly intentional non-goals,
- real-repo false-positive rate is low and ratcheted,
- and cross-platform CI + hardening lanes are consistently green.
