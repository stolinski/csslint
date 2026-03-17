# Step 8: CLI and Config

## Purpose

Provide a predictable, low-friction CLI and config system that supports local development and CI automation without cascading configuration complexity.

## CLI Commands (v1)

- `csslint <path>`
- `csslint <path> --fix`
- `csslint <path> --format json`

Recommended optional flags:

- `--config <path>`
- `--max-warnings <n>`
- `--quiet` (errors only)
- `--targets <query or preset>`
- `--threads <n>`

## Exit Code Policy

- `0`: no errors
- `1`: lint errors found
- `2`: runtime/config/internal failure

Precedence rule:

- if both lint errors and runtime/config/internal failures occur, exit `2`

Warnings should not fail by default unless configured (`--max-warnings=0`).

## Config Model

v1 config file contract:

- file name: `.csslint`
- format: JSON only

Minimal schema (JSON example):

```json
{
  "preset": "recommended",
  "frameworks": ["vue", "svelte"],
  "targets": "defaults",
  "fix": false,
  "rules": {
    "no_unknown_properties": "error",
    "no_duplicate_selectors": "warn"
  }
}
```

## Config Principles

- predictable and explicit
- no deep extends chaining in v1
- strict schema validation
- clear error messages on invalid values

## Presets

- `recommended` (default)
- `strict`
- `minimal`

Each preset must be fully documented as a resolved rule map.

## Config Resolution Order

1. CLI args
2. explicit `--config` file
3. nearest project config by directory traversal
4. built-in defaults

No hidden cascading behavior beyond this order.

## Reporting Formats

### Pretty Output

- concise file/rule/severity lines
- optional code frame
- final summary counts

### JSON Output

- machine-friendly stable schema
- include:
  - file path
  - rule ID
  - severity
  - message
  - start/end offsets
  - line/column
  - fix availability

Schema should be versioned (`"schemaVersion": 1`).

Canonical schema files:

- machine validation: `docs/json-output-schema-v1.schema.json`
- human-readable companion: `docs/json-output-schema-v1.md`

## File Discovery Rules

- include: `.css`, `.vue`, `.svelte`
- ignore by default: `node_modules`, build output dirs, hidden VCS dirs
- support explicit include/exclude patterns in future versions

## Target Configuration

- Accept a default target profile (`defaults`) in v1.
- Optionally accept explicit target query.
- Pass targets into parser and relevant rules.

## Testing Plan

### CLI Integration Tests

- command success/failure exit codes
- lint-only and fix flows
- pretty and json outputs

### Config Tests

- valid config permutations
- invalid schema errors
- preset expansion correctness

### Reporter Tests

- deterministic ordering in output
- schema snapshot tests for JSON
- schema fixture validation in CI via `scripts/validate_json_output_schema.py`

## Deliverables

- `csslint-cli` crate implementation.
- `csslint-config` crate with schema validation.
- reporter modules (pretty/json).
- CLI docs and examples.

## Exit Criteria

- CLI is usable end-to-end on mixed project paths.
- JSON output is stable for CI consumption.
- Config validation errors are clear and actionable.

## Risks and Mitigations

- **Risk**: config ambiguity from too many options.
  - **Mitigation**: minimal v1 schema and strict validation.
- **Risk**: non-deterministic output order in CI.
  - **Mitigation**: single global sorting policy before reporting.
