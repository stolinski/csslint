# csslint-fuzz

Fuzz targets for panic hardening of core ingestion phases.

Targets:

- `extractor`: component/style extraction over arbitrary bytes
- `parser_wrapper`: parser wrapper on extracted style blocks
- `semantic_builder`: semantic model build on parser output

Usage:

```bash
cargo install cargo-fuzz
cargo fuzz run extractor
cargo fuzz run parser_wrapper
cargo fuzz run semantic_builder
```

These fuzz targets are best-effort local/nightly tools and are paired with the
malformed corpus CI lane in `csslint-test-harness` for stable reliability
coverage on every change.
