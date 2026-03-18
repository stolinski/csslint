#!/usr/bin/env python3

"""Validate the Stylelint compatibility skip manifest."""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from pathlib import Path

ALLOWED_REASON_CODES = {
    "custom_syntax",
    "scss_less",
    "directive_comments",
    "postcss_integration",
    "unsupported_option",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate tests/compat/stylelint/skip-manifest.yaml against imported fixtures"
    )
    parser.add_argument(
        "--manifest",
        default="tests/compat/stylelint/skip-manifest.yaml",
        help="Path to skip-manifest.yaml (JSON-compatible YAML)",
    )
    parser.add_argument(
        "--imported-root",
        default="tests/compat/stylelint/imported",
        help="Directory containing imported fixture JSON files",
    )
    return parser.parse_args()


def read_json(path: Path) -> dict:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise ValueError(f"failed to parse JSON from {path}: {error}") from error


def build_fixture_indexes(imported_root: Path) -> tuple[dict[tuple[str, str], dict], dict[tuple[str, str], dict]]:
    fixture_cases: dict[tuple[str, str], dict] = {}
    fixture_skips: dict[tuple[str, str], dict] = {}

    for fixture_path in sorted(imported_root.glob("*.json")):
        fixture = read_json(fixture_path)
        stylelint = fixture.get("stylelint")
        if not isinstance(stylelint, dict):
            raise ValueError(f"fixture missing stylelint block: {fixture_path}")

        stylelint_rule = stylelint.get("rule")
        if not isinstance(stylelint_rule, str) or not stylelint_rule:
            raise ValueError(f"fixture missing stylelint.rule: {fixture_path}")

        for case in fixture.get("cases", []):
            case_id = case.get("id")
            if not isinstance(case_id, str) or not case_id:
                raise ValueError(f"fixture case missing id in {fixture_path}")

            key = (stylelint_rule, case_id)
            fixture_cases[key] = case

            skip = case.get("skip")
            if isinstance(skip, dict):
                fixture_skips[key] = skip

    return fixture_cases, fixture_skips


def validate_manifest(
    manifest: dict,
    fixture_cases: dict[tuple[str, str], dict],
    fixture_skips: dict[tuple[str, str], dict],
) -> tuple[list[str], Counter]:
    errors: list[str] = []
    counts: Counter = Counter()

    skips = manifest.get("skips")
    if not isinstance(skips, list):
        return ["skip manifest missing 'skips' array"], counts

    manifest_keys: set[tuple[str, str]] = set()

    for index, entry in enumerate(skips):
        if not isinstance(entry, dict):
            errors.append(f"skip entry {index} is not an object")
            continue

        stylelint_rule = entry.get("stylelintRule")
        case_id = entry.get("caseId")
        reason_code = entry.get("reasonCode")

        if not isinstance(stylelint_rule, str) or not stylelint_rule:
            errors.append(f"skip entry {index} missing stylelintRule")
            continue

        if not isinstance(case_id, str) or not case_id:
            errors.append(f"skip entry {index} missing caseId")
            continue

        if not isinstance(reason_code, str) or not reason_code:
            errors.append(f"skip entry {index} missing reasonCode")
            continue

        if reason_code not in ALLOWED_REASON_CODES:
            allowed = ", ".join(sorted(ALLOWED_REASON_CODES))
            errors.append(
                f"skip entry {index} has invalid reasonCode '{reason_code}' (expected one of: {allowed})"
            )
            continue

        key = (stylelint_rule, case_id)
        manifest_keys.add(key)
        counts[reason_code] += 1

        if key not in fixture_cases:
            errors.append(
                f"skip entry {index} references unknown case {stylelint_rule}:{case_id}"
            )
            continue

        fixture_skip = fixture_skips.get(key)
        if fixture_skip is None:
            errors.append(
                f"skip entry {index} references case without embedded skip metadata: {stylelint_rule}:{case_id}"
            )
            continue

        embedded_reason = fixture_skip.get("reasonCode")
        if embedded_reason != reason_code:
            errors.append(
                "skip reason mismatch for "
                f"{stylelint_rule}:{case_id} (manifest={reason_code}, fixture={embedded_reason})"
            )

    for key in sorted(fixture_skips):
        if key not in manifest_keys:
            errors.append(
                f"fixture case has embedded skip metadata but is missing from manifest: {key[0]}:{key[1]}"
            )

    return errors, counts


def main() -> int:
    args = parse_args()
    manifest_path = Path(args.manifest)
    imported_root = Path(args.imported_root)

    if not manifest_path.exists():
        print(f"skip manifest not found: {manifest_path}", file=sys.stderr)
        return 1

    if not imported_root.exists():
        print(f"imported fixture root not found: {imported_root}", file=sys.stderr)
        return 1

    manifest = read_json(manifest_path)
    fixture_cases, fixture_skips = build_fixture_indexes(imported_root)
    errors, counts = validate_manifest(manifest, fixture_cases, fixture_skips)

    if errors:
        print("stylelint skip manifest validation failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    total = sum(counts.values())
    print(f"stylelint skip manifest valid: {total} skipped case(s)")
    for reason_code in sorted(ALLOWED_REASON_CODES):
        print(f"- {reason_code}: {counts[reason_code]}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
