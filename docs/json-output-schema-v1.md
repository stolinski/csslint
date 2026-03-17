# JSON Output Schema (v1)

## Purpose

Define a stable machine-readable output contract for CI and editor integrations.

Normative validation schema:

- `docs/json-output-schema-v1.schema.json`

This markdown file is the human-readable companion.

Validation fixtures and check script:

- fixtures: `docs/fixtures/json-output/`
- validator: `scripts/validate_json_output_schema.py`
- CI workflow: `.github/workflows/json-output-schema-check.yml`

## Top-Level Shape

```json
{
  "schemaVersion": 1,
  "tool": "csslint",
  "summary": {
    "filesScanned": 0,
    "filesLinted": 0,
    "errors": 0,
    "warnings": 0,
    "fixesApplied": 0,
    "durationMs": 0,
    "exitCode": 0
  },
  "diagnostics": [],
  "internalErrors": [],
  "timing": {
    "parseMs": 0,
    "semanticMs": 0,
    "rulesMs": 0,
    "fixMs": 0
  }
}
```

## Field Definitions

## `summary`

- `filesScanned`: files discovered by CLI traversal
- `filesLinted`: files actually linted after extraction and filters
- `errors`: count of diagnostics with severity `error`
- `warnings`: count of diagnostics with severity `warn`
- `fixesApplied`: number of edits applied in `--fix` mode
- `durationMs`: total run duration in milliseconds
- `exitCode`: resolved process exit code (`0`, `1`, or `2`)

## Exit Code Contract (Normative)

`summary.exitCode` uses fixed v1 semantics:

- `0`: lint run completed with no error-severity diagnostics and no runtime/config/internal failures.
- `1`: lint run completed and found one or more error-severity diagnostics.
- `2`: runtime/config/internal failure occurred.

Precedence is strict: `2` overrides `1`, and `1` overrides `0`.

## `diagnostics[]`

Each diagnostic object:

```json
{
  "filePath": "src/App.svelte",
  "ruleId": "no_global_leaks",
  "severity": "error",
  "message": "Scoped style contains accidental global selector",
  "span": {
    "startOffset": 128,
    "endOffset": 149,
    "startLine": 12,
    "startColumn": 3,
    "endLine": 12,
    "endColumn": 24
  },
  "fix": {
    "available": false
  }
}
```

Rules:

- offsets are byte offsets in original file coordinates
- line/column are 1-based
- `fix` is always present; if unavailable, set `available: false`

## `internalErrors[]`

Structured non-rule failures (runtime/config/internal):

```json
{
  "kind": "config_error",
  "message": "Unknown rule id: no_foo",
  "filePath": ".csslint"
}
```

Allowed `kind` values in v1:

- `config_error`
- `runtime_error`
- `internal_error`

## `timing`

Phase timings are optional in normal mode and should default to `0` if not collected.

## Stability Policy

- `schemaVersion` increments on breaking shape changes.
- New optional fields may be added in v1 without version bump.
- Existing field names and meanings are stable for the v1 line.
