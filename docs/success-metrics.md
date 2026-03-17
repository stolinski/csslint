# Success Metrics (v1)

## Purpose

Define the measurable release gates for v1 and make CI reporting explicit.

This document is intentionally aligned with current product decisions:

- benchmark for comparative speed, not a fixed 5x gate
- Stylelint baseline uses latest release at benchmark time
- compatibility quality is tracked and ratcheted, not blocked on an arbitrary pass number

## Decision Snapshot

- Performance target model: **comparative reporting** (no fixed multiplier gate).
- Comparison baseline: **Stylelint latest** (exact version must be recorded in each report).
- Config file format: **JSON only**, file name **`.csslint`**.
- Unsupported `lang` blocks (`scss`, `less`, etc.): default **error diagnostic**.

## Benchmark Corpus and Method

Benchmarks must run both tools on equivalent inputs and capture the same run metadata.

### Corpus Types

1. CSS-only projects
2. Vue-heavy projects
3. Svelte-heavy projects
4. Mixed repositories

Each corpus run records:

- total files
- total CSS bytes extracted/linted
- include/exclude patterns
- host machine profile (CPU, memory, OS)
- exact tool versions (`csslint`, `stylelint`, Node, Rust)

### Run Protocol

- Run both tools with equivalent rule intent where overlap exists.
- Run each benchmark at least 5 warm iterations.
- Optionally record one cold iteration for context.
- Publish median and variance for each metric.

## Metrics to Publish

For every corpus and tool:

- total runtime (ms)
- files per second
- MB per second
- p50 and p95 per-file time
- peak RSS memory
- phase timing for csslint (parse/semantic/rules/fix)

Derived comparative metrics:

- runtime ratio (`stylelint_ms / csslint_ms`)
- throughput ratio (`csslint_files_per_sec / stylelint_files_per_sec`)

## CI Gates

## Correctness Gates (Blocking)

- Determinism suite green (identical diagnostics/fixes across repeat runs).
- Mapping suite green (correct line/column and offset behavior).
- Fix idempotency suite green for all fixable rules.
- No panic in malformed corpus reliability lane.

## Performance Gates (Blocking)

- Benchmark job must run and produce a report artifact.
- csslint must not regress against `main` baseline by:
  - more than 20 percent median runtime, or
  - more than 20 percent peak memory,
  unless explicitly accepted in PR notes with rationale.

Note: v1 does not require a fixed speedup multiplier over Stylelint, but every release report must include the comparison table.

## Compatibility Gates (Blocking)

- Compatibility harness runs for all selected imported suites.
- Pass/skip/fail is reported per rule and globally.
- Pass rate may not drop from the previous baseline without a linked deferral or known-divergence update.

## Reporting Artifacts

Each CI benchmark lane should publish:

- machine profile
- tool versions
- corpus definition digest
- raw numbers table (both tools)
- comparative ratios
- regression verdict versus baseline

Recommended artifact paths:

- `artifacts/perf/perf-summary.json`
- `artifacts/perf/perf-summary.md`
- `artifacts/compat/compat-summary.json`

## Release Checklist

Before v1 release, all are required:

1. Correctness gates pass.
2. Performance lane green with comparison report attached.
3. Compatibility lane green with ratchet policy satisfied.
4. Open performance deviations (if any) documented in release notes.
