# Stylelint Compatibility Corpus

This directory pins and translates a curated subset of upstream Stylelint rule
tests into CSSLint-native compatibility fixtures.

## Source Pin

- Pinned source metadata: `tests/compat/stylelint/source-pin.json`
- Upstream repository: `stylelint/stylelint`
- Pinned commit SHA: `5bd2d21e8a6b47f529314284d162e6dcb37ef681`

The commit SHA is intentionally fixed so fixture imports are reproducible.

## Suite Mapping

- Rule suite map: `tests/compat/stylelint/suite-map.json`
- Mapping scope follows `docs/plan/09-stylelint-compatibility-harness.md`
- `importMode: partial` entries indicate explicit v1 subset imports

## Directory Layout

- `upstream/`: pinned raw JS test snapshots used as importer input
- `imported/`: generated CSSLint-native fixture files
- `skip-manifest.yaml`: explicit skips with reason codes
- `baseline/`: ratchet baseline used by compatibility metrics checks

## Update Workflow

1. Update `source-pin.json` to a new Stylelint commit.
2. Reconcile `suite-map.json` for any source file or option changes.
3. Re-run the fixture importer to regenerate `imported/` fixtures:

   `node scripts/import_stylelint_fixtures.mjs`

   Drift-only check mode:

   `node scripts/import_stylelint_fixtures.mjs --check`

4. Review and update `skip-manifest.yaml` entries as needed.
5. Re-run compatibility harness and ratchet checks before merging.
