# CSSLint

Fast, deterministic CSS linting for modern projects.

This project is a Rust-first linter focused on CI-safe behavior and first-class support for:

- `.css`
- `.vue` `<style>` blocks
- `.svelte` `<style>` blocks

It is currently in active development toward a scoped v1.

## Current CLI

```bash
csslint <path> [--config <path>] [--ignore-path <path>] [--targets <profile>] [--code-frame] [--profile] [--fix] [--format json|pretty]
```

Core v1 commands:

- `csslint <path>`
- `csslint <path> --fix`
- `csslint <path> --format json`

Exit codes:

- `0` no errors
- `1` lint errors found
- `2` runtime/config/internal failure

## Install

From this repo clone (works now):

```bash
cargo install --path crates/csslint-cli --force
```

Or build locally and run from `target`:

```bash
cargo build --release
./target/release/csslint --help
```

From GitHub Releases (after a tagged release):

1. Open the repo's latest release page.
2. Download the archive for your platform:
   - `csslint-linux-x86_64.tar.gz`
   - `csslint-macos-x86_64.tar.gz`
   - `csslint-windows-x86_64.zip`
3. Verify checksum with the matching `.sha256` file.
4. Extract and place `csslint` (or `csslint.exe`) on your `PATH`.

Quick try from extracted binary:

```bash
./csslint --help
./csslint /path/to/repo --format json
```

## Quick Start

```bash
# lint
csslint .

# apply safe fixes
csslint . --fix

# CI-friendly JSON output
csslint . --format json
```

## Config

v1 config file:

- file name: `.csslint`
- format: JSON only

See `docs/plan/08-cli-and-config.md` and `docs/rule-catalog-v1.md` for canonical rule and preset behavior.

## Documentation

- Docs index: `docs/README.md`
- Plan and execution order: `docs/plan/README.md`
- Rule catalog and defaults: `docs/rule-catalog-v1.md`
- JSON output contract: `docs/json-output-schema-v1.md`
