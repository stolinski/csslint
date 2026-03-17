#!/usr/bin/env python3

import sys
import tomllib
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


def extract_dep_names(section: object) -> set[str]:
    if not isinstance(section, dict):
        return set()
    return {name for name in section.keys() if isinstance(name, str)}


def load_workspace_manifests(repo_root: Path) -> dict[str, dict[str, object]]:
    crates_dir = repo_root / "crates"
    manifests = sorted(crates_dir.glob("*/Cargo.toml"))
    crates: dict[str, dict[str, object]] = {}

    for manifest in manifests:
        parsed = tomllib.loads(manifest.read_text(encoding="utf-8"))
        package = parsed.get("package")
        if not isinstance(package, dict):
            continue

        crate_name = package.get("name")
        if not isinstance(crate_name, str) or not crate_name.startswith("csslint-"):
            continue

        deps = extract_dep_names(parsed.get("dependencies"))
        crates[crate_name] = {
            "manifest": manifest,
            "dependencies": deps,
        }

    return crates


def main() -> int:
    repo_root = Path(__file__).resolve().parents[1]
    packages = load_workspace_manifests(repo_root)

    errors: list[str] = []
    for crate_name in sorted(packages):
        if crate_name not in ALLOWED_INTERNAL_DEPS:
            errors.append(f"No dependency policy entry for crate '{crate_name}'")

    for crate_name in sorted(ALLOWED_INTERNAL_DEPS):
        if crate_name not in packages:
            errors.append(f"Expected crate '{crate_name}' is missing from workspace")

    graph: dict[str, set[str]] = {}
    lightning_dependents: list[str] = []
    for crate_name, package_data in packages.items():
        deps = package_data["dependencies"]
        if not isinstance(deps, set):
            continue

        internal_deps = {name for name in deps if name.startswith("csslint-")}
        graph[crate_name] = internal_deps

        if "lightningcss" in deps:
            lightning_dependents.append(crate_name)

        allowed = ALLOWED_INTERNAL_DEPS.get(crate_name, set())
        for dep_name in sorted(internal_deps):
            if dep_name not in allowed:
                errors.append(
                    f"Forbidden internal dependency: {crate_name} -> {dep_name}"
                )

    cycle = detect_cycle(graph)
    if cycle is not None:
        errors.append("Internal dependency cycle detected: " + " -> ".join(cycle))

    if "csslint-parser" not in lightning_dependents:
        errors.append("csslint-parser must declare the lightningcss dependency")

    for crate_name in sorted(lightning_dependents):
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
