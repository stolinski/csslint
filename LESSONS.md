# LESSONS

- Keep scope boundaries and rule tags synchronized between `docs/plan/01-scope-and-success-criteria.md` and `docs/rule-catalog-v1.md`; treat both as lock files for v1 behavior.
- Preset names are shortcuts only; canonical behavior is explicit expansion plus user override, with unknown IDs/severities treated as hard config errors.
- For JSON reporter contracts, update both the schema and the companion markdown together; precedence rules may be documented semantically even when the schema cannot enforce flow logic.
- Release gates are not complete until each one names a concrete CI lane, blocking condition, and expected artifact.
- Keeping `RuleId` as a shared core type (instead of per-crate strings) prevents conversion shims and keeps config/rules/reporting contracts aligned.
- Dependency direction drift is easiest to catch with a metadata-based policy script; keep parser as the only crate that declares `lightningcss`.
- Baseline Rust CI should always keep five lanes alive (`check`, `lint`, `unit`, `integration`, `perf`) so later milestones have stable enforcement hooks.
- Vue policy invariant: any `<style src>` block is warning+skip in v1, even if inline CSS is present in the same block.
- Keep source mapping offset-first: compute global spans with `start_offset + local_offset`, then convert offsets to line/column with a prebuilt LF/CRLF line index.
- Fixture corpus should assert both block order and `source[start_offset..end_offset] == content` to catch offset regressions immediately.
- Malformed reliability needs explicit script handling too: missing `</script>` should emit a controlled warning and halt extraction safely.
- `lightningcss` currently resolves as prerelease on crates.io, so parser dependency must pin an explicit alpha version (for example `1.0.0-alpha.71`) instead of `1`.
