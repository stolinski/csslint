# Performance Corpus Fixtures

This directory hosts deterministic benchmark corpora used by `perf_corpus_bench`.

- `corpora/css-only`: native CSS inputs
- `corpora/vue-heavy`: Vue SFC-heavy sample inputs
- `corpora/svelte-heavy`: Svelte-heavy sample inputs
- `corpora/mixed`: mixed `.css`, `.vue`, and `.svelte` sample inputs

Benchmark protocol defaults:

- cold iterations: `1`
- warm iterations: `5`

Run locally:

```bash
cargo run -p csslint-test-harness --bin perf_corpus_bench -- --output artifacts/perf/perf-corpus-summary.json
npx --yes --package stylelint@16.15.0 --package postcss-html@1.7.0 node scripts/stylelint_perf_benchmark.mjs --output artifacts/perf/stylelint-summary.json
python3 scripts/build_perf_summary.py --csslint artifacts/perf/perf-corpus-summary.json --stylelint artifacts/perf/stylelint-summary.json --output-json artifacts/perf/perf-summary.json --output-md artifacts/perf/perf-summary.md
```
