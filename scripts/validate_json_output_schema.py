#!/usr/bin/env python3
"""Validate json output fixtures against docs/json-output-schema-v1.schema.json.

Expected fixture naming:
- valid*.json: must pass schema validation
- invalid*.json: must fail schema validation
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, cast


def parse_args() -> argparse.Namespace:
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--schema",
        default=str(root / "docs" / "json-output-schema-v1.schema.json"),
        help="Path to JSON Schema file",
    )
    parser.add_argument(
        "--fixtures",
        default=str(root / "docs" / "fixtures" / "json-output"),
        help="Path to schema fixture directory",
    )
    return parser.parse_args()


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        print(f"JSON parse error in {path}: {exc}")
        raise


def main() -> int:
    args = parse_args()

    schema_path = Path(args.schema)
    fixture_dir = Path(args.fixtures)

    if not schema_path.exists():
        print(f"Schema file not found: {schema_path}")
        return 2
    if not fixture_dir.exists():
        print(f"Fixture directory not found: {fixture_dir}")
        return 2

    try:
        from jsonschema import Draft202012Validator
    except ModuleNotFoundError:
        print(
            "Missing dependency: jsonschema. "
            "Install with 'python -m pip install jsonschema'."
        )
        return 2

    schema = cast(dict[str, Any], load_json(schema_path))
    validator = Draft202012Validator(schema)

    valid_files = sorted(fixture_dir.glob("valid*.json"))
    invalid_files = sorted(fixture_dir.glob("invalid*.json"))

    if not valid_files:
        print("No valid fixtures found (expected files matching valid*.json)")
        return 1
    if not invalid_files:
        print("No invalid fixtures found (expected files matching invalid*.json)")
        return 1

    failures: list[str] = []

    for fixture in valid_files:
        data = load_json(fixture)
        errors = sorted(validator.iter_errors(data), key=lambda err: list(err.path))
        if errors:
            msg = errors[0].message
            failures.append(f"VALID fixture failed: {fixture} :: {msg}")

    for fixture in invalid_files:
        data = load_json(fixture)
        errors = sorted(validator.iter_errors(data), key=lambda err: list(err.path))
        if not errors:
            failures.append(f"INVALID fixture unexpectedly passed: {fixture}")

    if failures:
        print("Schema fixture validation failed:")
        for failure in failures:
            print(f"- {failure}")
        return 1

    print(
        "Schema fixture validation passed "
        f"({len(valid_files)} valid, {len(invalid_files)} invalid fixtures)."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
