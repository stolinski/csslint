# Vue Style Block Policy (v1)

## Purpose

Define concrete v1 behavior for Vue SFC style block variants so extractor, semantic scope, and native rules behave consistently.

## `<style module>` Policy

- Recognize both `<style module>` and `<style module="name">`.
- Extract as a normal style block with `module = true` metadata.
- Treat as `scoped` context for native scoped rules in v1:
  - `no_global_leaks`
- Plugin-path candidate: template-aware `no_unused_scoped_selectors` can consume this metadata later.
- Continue to honor explicit `:global(...)` escapes.

v1 limitation:

- No CSS Modules class-name remapping model is implemented.
- Behavior is scoped-context linting, not full module-runtime simulation.

## `<style src>` Policy

- Recognize `<style src="...">`.
- Do not resolve or fetch external source files in v1.
- Emit one non-fatal warning diagnostic for each external-source block.
- Skip extraction/linting for that block in the SFC pipeline.

If a block contains both `src` and inline CSS, v1 still warns and skips the block.

## Diagnostic Guidance

Suggested diagnostic IDs/messages for consistency:

- `unsupported_style_lang`: unsupported style language in component block.
- `unsupported_external_style_src`: Vue `<style src>` is not resolved in v1.

## Rationale

- Keeps extractor deterministic and fast.
- Avoids hidden filesystem/import resolution complexity in v1.
- Preserves useful scoped-rule behavior for Vue module blocks.
