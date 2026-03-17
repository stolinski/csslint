# Step 3: Extractor and Source Mapping

## Purpose

Build a fast, fault-tolerant extraction layer for `.css`, `.vue`, and `.svelte` that preserves exact source mapping. All downstream diagnostics and fixes depend on this being correct.

## Requirements

1. Support `.css` direct parsing.
2. Process all `<style>` blocks from `.vue` files (extract inline blocks, emit diagnostics for unsupported external-source blocks).
3. Extract `<style>` block from `.svelte` files.
4. Preserve byte offsets into original file.
5. Capture style-block metadata needed by semantic scope logic.
6. Never panic on malformed component files.

## Output Contract

```rust
pub struct ExtractedStyle {
    pub file_id: FileId,
    pub block_index: u32,
    pub content: String,
    pub start_offset: usize,
    pub end_offset: usize,
    pub lang: StyleLang,   // v1 currently only Css accepted for lint
    pub scoped: bool,
    pub module: bool,
    pub framework: FrameworkKind, // Css | Vue | Svelte
}
```

`start_offset` and `end_offset` are offsets in the original file byte stream.

## Extraction Strategy by File Type

### `.css`

- Single `ExtractedStyle` block.
- `scoped = false`, `module = false`, `framework = Css`.
- `start_offset = 0`, `end_offset = file_len`.

### `.vue`

- Use a fast scanner for `<style ...>` opening and `</style>` closing tags.
- Parse opening tag attributes minimally:
  - `scoped`
  - `module` and `module="..."`
  - `src`
  - `lang`
- Extract every valid style block in order.
- For `lang != css`, mark as unsupported, skip lint for that block, and emit an error diagnostic.
- For `<style module>` and `<style module="name">`, set `module = true`; semantic phase treats these blocks as scoped contexts for native scoped rules in v1.
- For `<style src="...">`, do not resolve external files in v1. Emit a non-fatal warning diagnostic and skip extraction for that block.
- If a Vue style block contains both `src` and inline content, still skip extraction and emit one warning diagnostic.

Policy reference: `docs/vue-style-policy-v1.md`.

### `.svelte`

- Extract `<style ...>` in the component source.
- Record attrs (`lang`, optional global markers if present via conventions).
- Default scope behavior is handled later in semantic phase, not extractor.

## Scanner Implementation Notes

- Use byte-level scanning for speed.
- Keep parser state simple and deterministic.
- Do not attempt full HTML AST parsing in v1.
- Handle quoted attributes robustly (`"` and `'`).
- Ignore `<style>` inside comments only if confidently detectable; otherwise prefer conservative extraction and parser fallback handling.

## Source Mapping Design

### Offset-First Model

- All diagnostics and fixes use byte spans in original file coordinates.
- Line/column conversion occurs at report time using line index map.

### Line Index Map

- Build once per file: vector of line-start offsets.
- Support LF and CRLF without mutating source.
- Convert `offset -> (line, column)` via binary search.

### Mapping Formula

- `global_start = extracted.start_offset + local_start`
- `global_end = extracted.start_offset + local_end`

## Error Handling

- Unclosed `<style>` block: emit extraction warning and skip incomplete block.
- Invalid attribute syntax: fallback to defaults for unknown attrs.
- Unsupported `lang`: skip lint for that block and emit an error diagnostic.
- Vue `<style src>`: emit warning and skip that block; external file may still be linted if discovered directly by CLI file traversal.
- Empty style content: still emit block so empty-rule logic can run where applicable.

## Testing Plan

### Unit Tests

- CSS direct extraction.
- Vue single and multi-style blocks.
- Vue scoped/module combinations.
- Vue `<style src>` handling (warning + skip).
- Svelte style extraction with spacing/attrs variants.
- CRLF and LF files.

### Integration Tests

- Diagnostic location in `.vue` maps to original file lines.
- Diagnostic location in `.svelte` maps to original file lines.
- Multiple blocks do not shift each other offsets.
- Vue module blocks map and execute as scoped-context style blocks.

### Edge Cases

- Missing closing tags.
- Embedded `<style>` text inside strings/comments.
- Vue `<style src>` with inline content.
- Empty style tags.
- Very large files.

## Performance Constraints

- O(n) scan per file.
- Avoid regex-heavy extraction loops.
- Avoid string copying until a style block is confirmed.

## Deliverables

- `csslint-extractor` implementation.
- Line index mapper utility in `csslint-core` or extractor support module.
- Fixture corpus under `tests/native/extractor`.

## Exit Criteria

- Extraction fixtures pass.
- Offset-to-line/column assertions pass for `.css`, `.vue`, `.svelte`.
- Malformed inputs produce controlled diagnostics, no panics.

## Risks and Mitigations

- **Risk**: false style block detection in complex templates.
  - **Mitigation**: strong fixture coverage and conservative scanner state machine.
- **Risk**: off-by-one location bugs.
  - **Mitigation**: dedicated mapping tests for start/end boundaries and CRLF handling.
