# Step 11: Performance and Hardening

## Purpose

Validate and protect the core promise of the project: significantly better performance than Stylelint with reliable behavior under real-world and malformed inputs.

## Performance Goals

- Publish comparative speed and throughput metrics against Stylelint on agreed corpus.
- No fixed speedup multiplier is required in v1.
- Stable memory usage with documented upper bounds.
- Deterministic output regardless of thread count.

## Benchmark Corpus Design

Use multiple corpus types:

1. large pure CSS libraries
2. Vue-heavy component repositories
3. Svelte-heavy component repositories
4. mixed monorepo samples

Each corpus should include cold and warm run measurements.

## Metrics to Capture

- total run time
- files per second
- MB per second
- p50/p95 per-file processing time
- peak RSS memory
- parse time vs semantic time vs rules time vs fix time

## Benchmark Methodology

- pin machine profile and runtime environment
- pin Stylelint comparison version
- run multiple iterations and report median + variance
- isolate I/O-heavy effects where possible

## Regression Budgets

Define CI thresholds for:

- runtime regression percentage
- memory regression percentage
- rule-specific hotspot regressions

If threshold exceeded, CI fails for performance lane.

## Optimization Priorities

1. eliminate extra parse passes
2. reduce string cloning and allocations
3. cache normalized selector/value computations
4. optimize rule dispatch subscription map
5. tune bounded thread pool for per-file parallelism

Always optimize based on profiler evidence.

## Hardening Strategy

## Fuzzing

- extractor fuzz target
- parser wrapper fuzz target
- semantic builder fuzz target

Goals:

- no panics
- bounded memory behavior
- controlled diagnostics for malformed input

## Malformed Input Corpus

- broken `<style>` tags
- invalid selectors
- truncated declarations
- pathological nested constructs

Run malformed corpus in CI as reliability lane.

## Determinism Validation

- run identical inputs with varying thread counts
- assert identical diagnostics order and fix outputs
- include mixed file type projects in determinism tests

## Soak and Stress Testing

- long-running lint on large repositories
- monitor memory growth and runtime stability
- capture and triage intermittent failures

## Observability for Engineering

Add optional internal profiling output:

- per-phase timing
- top N slow files
- top N expensive rules

Keep this behind debug/profiling flag in v1.

## Deliverables

- benchmark harness and scripts
- perf baseline report vs Stylelint
- fuzz targets and malformed corpus tests
- determinism test suite
- CI perf regression jobs

## Exit Criteria

- Benchmark comparison report is generated and published.
- No critical panic paths in fuzz and malformed corpus tests.
- Deterministic output validated under parallel execution.

## Risks and Mitigations

- **Risk**: chasing micro-optimizations without impact.
  - **Mitigation**: profile-first optimization policy.
- **Risk**: perf improvements reduce correctness.
  - **Mitigation**: keep correctness and mapping gates mandatory before perf wins are accepted.
