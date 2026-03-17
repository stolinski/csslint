# LESSONS

- Keep scope boundaries and rule tags synchronized between `docs/plan/01-scope-and-success-criteria.md` and `docs/rule-catalog-v1.md`; treat both as lock files for v1 behavior.
- Preset names are shortcuts only; canonical behavior is explicit expansion plus user override, with unknown IDs/severities treated as hard config errors.
- For JSON reporter contracts, update both the schema and the companion markdown together; precedence rules may be documented semantically even when the schema cannot enforce flow logic.
- Release gates are not complete until each one names a concrete CI lane, blocking condition, and expected artifact.
