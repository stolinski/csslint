# Step 7: Fix Engine

## Purpose

Implement a safe and deterministic autofix pipeline that can apply multiple rule fixes in one pass without offset corruption or conflicting edits.

## Core Requirements

1. Collect fixes from all rules.
2. Detect and resolve overlapping edits.
3. Apply edits in stable order.
4. Guarantee idempotency for `--fix` runs.
5. Preserve source mapping correctness.

## Fix Data Model

```rust
pub struct Fix {
    pub file_id: FileId,
    pub start: usize,
    pub end: usize,
    pub replacement: String,
    pub rule_id: RuleId,
    pub severity: Severity,
    pub priority: u16,
}
```

Coordinates are always original-file byte offsets.

## Fix Collection Flow

1. Rules call `propose_fix()` through context.
2. Engine validates basic span integrity (`start <= end`, in-bounds).
3. Valid fixes are staged per file.
4. Conflict resolver runs before application.

## Conflict Resolution Policy

Two fixes conflict if ranges overlap in byte space.

Tie-break order (deterministic):

1. severity (`error` > `warn`)
2. explicit rule priority
3. shorter edit span preferred
4. lexicographic rule ID (final tie-break)

Dropped fixes should be traceable in debug output.

## Application Algorithm

1. Sort non-conflicting fixes by `start` descending.
2. Apply replacements from end to start.
3. Emit final fixed content and updated diagnostics summary.

Descending application avoids offset shifting complexity.

## Idempotency Contract

For any file:

- run `lint --fix` once -> output A
- run `lint --fix` again on A -> no new edits, output remains A

Idempotency tests are required for every fixable rule.

## Safety Constraints

- No cross-file transactional edits in v1.
- If span integrity cannot be trusted, skip fix and keep diagnostic.
- Preserve original newline style and unaffected bytes.

## Interaction with Diagnostics

- Diagnostics can include optional fix hints.
- For conflicting fix sets, diagnostics still emitted even if some fixes dropped.
- Optional verbose mode can show why a fix was dropped.

## Testing Plan

### Unit Tests

- overlap detection
- tie-break determinism
- descending application correctness

### Integration Tests

- multi-rule conflicting edits
- multi-fix same rule scenarios
- CRLF file fix spans
- Unicode edge offsets

### Property/Regression Tests

- repeated `--fix` idempotency
- no panics on malformed fix proposals

## Performance Considerations

- conflict resolution should be O(n log n) due to sort.
- avoid copying file buffers repeatedly.
- apply fixes with preallocated output buffer where feasible.

## Deliverables

- `csslint-fix` crate implementation.
- conflict resolver module.
- fix applier module.
- idempotency and overlap test suites.

## Exit Criteria

- All fixable rules pass idempotency tests.
- Overlapping edits are resolved deterministically.
- Fixed output preserves non-edited bytes exactly.

## Risks and Mitigations

- **Risk**: hidden overlap bugs from mixed-rule fixes.
  - **Mitigation**: exhaustive overlap fixture matrix and deterministic conflict policy.
- **Risk**: line/column drift after fixes.
  - **Mitigation**: keep original-span diagnostics for current run and re-lint for post-fix reporting if needed.
