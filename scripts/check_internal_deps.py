#!/usr/bin/env python3

import json
import subprocess
import sys
from pathlib import Path


ALLOWED_INTERNAL_DEPS = {
    "csslint-core": set(),
    "csslint-extractor": {"csslint-core"},
    "csslint-parser": {"csslint-core", "csslint-extractor"},
    "csslint-semantic": {"csslint-core", "csslint-extractor", "csslint-parser"},
    "csslint-rules": {"csslint-core", "csslint-semantic", "csslint-config"},
    "csslint-fix": {"csslint-core"},
    "csslint-config": {"csslint-core"},
    "csslint-cli": {
        "csslint-core",
        "csslint-extractor",
        "csslint-parser",
        "csslint-semantic",
        "csslint-rules",
        "csslint-fix",
        "csslint-config",
    },
    "csslint-test-harness": {
        "csslint-core",
        "csslint-extractor",
        "csslint-parser",
        "csslint-semantic",
        "csslint-rules",
        "csslint-fix",
        "csslint-config",
        "csslint-cli",
    },
    "csslint-plugin-api": {"csslint-core"},
}


def cargo_metadata(repo_root: Path) -> dict:
    try:
        completed = subprocess.run(
            ["cargo", "metadata", "--format-version", "1"],
            cwd=repo_root,
            capture_output=True,
            text=True,
            check=False,
        )
    except FileNotFoundError as error:
        print(
            "cargo not found in PATH; install Rust toolchain before running dependency checks",
            file=sys.stderr,
        )
        raise SystemExit(2) from error

    if completed.returncode != 0:
        print(completed.stderr.strip() or "failed to run cargo metadata", file=sys.stderr)
        raise SystemExit(completed.returncode)

    return json.loads(completed.stdout)


def detect_cycle(graph: dict[str, set[str]]) -> list[str] | None:
    visited: set[str] = set()
    visiting: set[str] = set()
    path: list[str] = []

    def dfs(node: str) -> list[str] | None:
        visited.add(node)
        visiting.add(node)
        path.append(node)

        for neighbor in sorted(graph.get(node, set())):
            if neighbor in visiting:
                start = path.index(neighbor)
                return path[start:] + [neighbor]
            if neighbor not in visited:
                cycle = dfs(neighbor)
                if cycle is not None:
                    return cycle

        visiting.remove(node)
        path.pop()
        return None

    for name in sorted(graph):
        if name in visited:
            continue
        cycle = dfs(name)
        if cycle is not None:
            return cycle

    return None


def main() -> int:
    repo_root = Path(__file__).resolve().parents[1]
    metadata = cargo_metadata(repo_root)

    packages = {
        package["name"]: package
        for package in metadata["packages"]
        if package["name"].startswith("csslint-")
    }

    errors: list[str] = []
    for crate_name in sorted(packages):
        if crate_name not in ALLOWED_INTERNAL_DEPS:
            errors.append(f"No dependency policy entry for crate '{crate_name}'")

    for crate_name in sorted(ALLOWED_INTERNAL_DEPS):
        if crate_name not in packages:
            errors.append(f"Expected crate '{crate_name}' is missing from workspace")

    graph: dict[str, set[str]] = {}
    for crate_name, package in packages.items():
        internal_deps = {
            dep["name"]
            for dep in package["dependencies"]
            if dep["name"].startswith("csslint-")
        }
        graph[crate_name] = internal_deps

        allowed = ALLOWED_INTERNAL_DEPS.get(crate_name, set())
        for dep_name in sorted(internal_deps):
            if dep_name not in allowed:
                errors.append(
                    f"Forbidden internal dependency: {crate_name} -> {dep_name}"
                )

    cycle = detect_cycle(graph)
    if cycle is not None:
        errors.append("Internal dependency cycle detected: " + " -> ".join(cycle))

    lightning_dependents = sorted(
        crate_name
        for crate_name, package in packages.items()
        if any(dep["name"] == "lightningcss" for dep in package["dependencies"])
    )

    if "csslint-parser" not in lightning_dependents:
        errors.append("csslint-parser must declare the lightningcss dependency")

    for crate_name in lightning_dependents:
        if crate_name != "csslint-parser":
            errors.append(
                f"Only csslint-parser may depend on lightningcss (found in {crate_name})"
            )

    if errors:
        print("Dependency policy violations found:")
        for error in errors:
            print(f"- {error}")
        return 1

    print("Dependency policy check passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
