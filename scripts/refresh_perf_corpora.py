#!/usr/bin/env python3

from __future__ import annotations

import hashlib
import json
import shutil
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CORPORA_ROOT = ROOT / "tests/perf/corpora"
STYLELINT_IMPORTED_ROOT = ROOT / "tests/compat/stylelint/imported"


def main() -> int:
    corpus_inputs = {
        "css-only": collect_css_only_inputs(),
        "vue-heavy": collect_vue_inputs(),
        "svelte-heavy": collect_svelte_inputs(),
    }
    corpus_inputs["mixed"] = (
        corpus_inputs["css-only"]
        + corpus_inputs["vue-heavy"]
        + corpus_inputs["svelte-heavy"]
    )

    summary = []
    for corpus_id, inputs in corpus_inputs.items():
        copied = sync_corpus(corpus_id, inputs)
        summary.append(
            {
                "corpusId": corpus_id,
                "files": len(copied),
                "totalBytes": sum(item[1].stat().st_size for item in copied),
                "digest": digest_outputs(copied),
            }
        )

    manifest_path = CORPORA_ROOT / "manifest.json"
    manifest_path.write_text(json.dumps({"schemaVersion": 1, "corpora": summary}, indent=2) + "\n")
    print(f"refreshed perf corpora snapshots -> {manifest_path}")
    for row in summary:
        print(
            f"- {row['corpusId']}: {row['files']} files, {row['totalBytes']} bytes, digest={row['digest']}"
        )
    return 0


def sync_corpus(corpus_id: str, inputs: list[tuple[str, bytes]]) -> list[tuple[str, Path]]:
    output_root = CORPORA_ROOT / corpus_id / "realworld"
    if output_root.exists():
        shutil.rmtree(output_root)
    output_root.mkdir(parents=True, exist_ok=True)

    written: list[tuple[str, Path]] = []
    for relative_name, content in sorted(inputs, key=lambda item: item[0]):
        destination = output_root / relative_name
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(content)
        written.append((relative_name, destination))
    return written


def collect_css_only_inputs() -> list[tuple[str, bytes]]:
    inputs: list[tuple[str, bytes]] = []

    # Existing hand-authored perf fixtures remain in corpus root.
    inputs.append(("native/css-basic-indexes.css", load_bytes("tests/native/semantic/css-basic-indexes/input.css")))

    for imported_file in sorted(STYLELINT_IMPORTED_ROOT.glob("*.json")):
        fixture = json.loads(imported_file.read_text())
        rule = fixture["stylelint"]["rule"]
        for case in fixture["cases"]:
            if "skip" in case:
                continue
            case_id = case["id"]
            css = ensure_trailing_newline(case["input"]).encode("utf-8")
            filename = f"stylelint/{rule}/{case_id}.css"
            inputs.append((filename, css))

    return inputs


def collect_vue_inputs() -> list[tuple[str, bytes]]:
    source_paths = [
        "tests/native/vue/rules/scoped-overqualified-selector/input.vue",
        "tests/native/vue/rules/scoped-duplicate-selectors/input.vue",
        "tests/native/vue/rules/plain-global-no-leak/input.vue",
        "tests/native/vue/rules/module-full-global-leak/input.vue",
        "tests/native/vue/rules/scoped-mixed-global-allowed/input.vue",
        "tests/native/vue/rules/scoped-full-global-leak/input.vue",
        "tests/native/vue/scope/plain-global/input.vue",
        "tests/native/vue/scope/module-default/input.vue",
        "tests/native/vue/scope/scoped-full-global/input.vue",
        "tests/native/vue/scope/scoped-partial-global/input.vue",
        "tests/native/vue/extractor/multi-style-order/input.vue",
        "tests/native/vue/extractor/module-variants/input.vue",
        "tests/native/shared/fix/vue-component-boundary/input.vue",
        "tests/native/shared/mapping/vue-crlf-multi-style/input.vue",
        "tests/native/template-usage/vue/medium-ternary-classes/input.vue",
        "tests/native/template-usage/vue/static-class-id/input.vue",
    ]
    return to_named_inputs(source_paths)


def collect_svelte_inputs() -> list[tuple[str, bytes]]:
    source_paths = [
        "tests/native/svelte/rules/default-local-no-leak/input.svelte",
        "tests/native/svelte/rules/default-overqualified-selector/input.svelte",
        "tests/native/svelte/rules/default-duplicate-selectors/input.svelte",
        "tests/native/svelte/rules/default-mixed-global-allowed/input.svelte",
        "tests/native/svelte/rules/default-full-global-leak/input.svelte",
        "tests/native/svelte/scope/default-full-global/input.svelte",
        "tests/native/svelte/scope/default-partial-global/input.svelte",
        "tests/native/svelte/scope/default-scoped/input.svelte",
        "tests/native/svelte/extractor/basic-style/input.svelte",
        "tests/native/shared/fix/svelte-component-boundary/input.svelte",
        "tests/native/shared/mapping/svelte-lf-single-style/input.svelte",
        "tests/native/template-usage/svelte/low-dynamic-expression/input.svelte",
        "tests/native/template-usage/svelte/static-class-directive/input.svelte",
    ]
    return to_named_inputs(source_paths)


def to_named_inputs(source_paths: list[str]) -> list[tuple[str, bytes]]:
    inputs: list[tuple[str, bytes]] = []
    for source_path in source_paths:
        path = ROOT / source_path
        suffix = path.suffix
        filename = source_path.replace("tests/native/", "native/").replace("/input", "")
        if not filename.endswith(suffix):
            filename = f"{filename}{suffix}"
        inputs.append((filename, load_bytes(source_path)))
    return inputs


def load_bytes(relative_path: str) -> bytes:
    return (ROOT / relative_path).read_bytes()


def digest_outputs(files: list[tuple[str, Path]]) -> str:
    hasher = hashlib.sha256()
    for relative_name, path in files:
        hasher.update(relative_name.encode("utf-8"))
        hasher.update(b"\x00")
        hasher.update(path.read_bytes())
        hasher.update(b"\x00")
    return hasher.hexdigest()[:16]


def ensure_trailing_newline(value: str) -> str:
    if value.endswith("\n"):
        return value
    return f"{value}\n"


if __name__ == "__main__":
    raise SystemExit(main())
