#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Fail when perf summary regresses past budget thresholds"
    )
    parser.add_argument(
        "--summary",
        default="artifacts/perf/perf-summary.json",
        help="Current perf summary JSON",
    )
    parser.add_argument(
        "--baseline",
        default="tests/perf/baseline/perf-summary.json",
        help="Baseline perf summary JSON",
    )
    parser.add_argument(
        "--runtime-budget-percent",
        type=float,
        default=20.0,
        help="Allowed runtime regression percentage",
    )
    parser.add_argument(
        "--memory-budget-percent",
        type=float,
        default=20.0,
        help="Allowed peak RSS regression percentage",
    )
    parser.add_argument(
        "--override-rationale",
        default="",
        help="Explicit rationale string that allows budget override",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    summary = json.loads(Path(args.summary).read_text())
    baseline = json.loads(Path(args.baseline).read_text())

    baseline_by_corpus = {entry["corpusId"]: entry for entry in baseline["corpora"]}
    violations: list[str] = []

    for corpus in summary["corpora"]:
        corpus_id = corpus["corpusId"]
        base = baseline_by_corpus.get(corpus_id)
        if base is None:
            violations.append(f"missing baseline corpus '{corpus_id}'")
            continue

        runtime_ratio = ratio(
            corpus["csslint"]["totalMs"], base["csslint"]["totalMs"]
        )
        memory_ratio = ratio(
            corpus["csslint"].get("peakRssBytes", 0),
            base["csslint"].get("peakRssBytes", 0),
        )

        runtime_limit = 1.0 + (args.runtime_budget_percent / 100.0)
        memory_limit = 1.0 + (args.memory_budget_percent / 100.0)

        if runtime_ratio > runtime_limit:
            violations.append(
                f"{corpus_id}: runtime regression {runtime_ratio:.3f}x exceeds {runtime_limit:.3f}x"
            )
        if memory_ratio > memory_limit:
            violations.append(
                f"{corpus_id}: peak RSS regression {memory_ratio:.3f}x exceeds {memory_limit:.3f}x"
            )

    if violations and args.override_rationale.strip():
        print("Perf budget override accepted with rationale:")
        print(args.override_rationale.strip())
        for violation in violations:
            print(f"- {violation}")
        return 0

    if violations:
        print("Perf regression budget check failed:")
        for violation in violations:
            print(f"- {violation}")
        print(
            "Set --override-rationale '<reason>' to accept intentional regressions with documented rationale."
        )
        return 1

    print("Perf regression budget check passed.")
    return 0


def ratio(current: float, baseline: float) -> float:
    if baseline <= 0:
        return 0.0
    return current / baseline


if __name__ == "__main__":
    raise SystemExit(main())
