#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Build perf comparison summary from csslint + stylelint raw outputs"
    )
    parser.add_argument(
        "--csslint",
        default="artifacts/perf/perf-corpus-summary.json",
        help="Path to csslint benchmark output",
    )
    parser.add_argument(
        "--stylelint",
        default="artifacts/perf/stylelint-summary.json",
        help="Path to stylelint benchmark output",
    )
    parser.add_argument(
        "--output-json",
        default="artifacts/perf/perf-summary.json",
        help="Destination JSON summary path",
    )
    parser.add_argument(
        "--output-md",
        default="artifacts/perf/perf-summary.md",
        help="Destination markdown summary path",
    )
    parser.add_argument(
        "--runtime-budget-percent",
        type=float,
        default=20.0,
        help="Runtime regression budget percentage",
    )
    parser.add_argument(
        "--memory-budget-percent",
        type=float,
        default=20.0,
        help="Memory regression budget percentage",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    csslint_payload = json.loads(Path(args.csslint).read_text())
    stylelint_payload = json.loads(Path(args.stylelint).read_text())

    stylelint_map = {entry["corpusId"]: entry for entry in stylelint_payload["corpora"]}
    corpus_rows = []
    for csslint_corpus in csslint_payload["corpora"]:
        corpus_id = csslint_corpus["corpusId"]
        stylelint_corpus = stylelint_map.get(corpus_id)
        if stylelint_corpus is None:
            raise SystemExit(f"missing stylelint corpus for '{corpus_id}'")

        css_median = csslint_corpus["warmMedian"]
        style_median = stylelint_corpus["warmMedian"]
        runtime_ratio = ratio(style_median["totalMs"], css_median["totalMs"])
        throughput_ratio = ratio(
            css_median["filesPerSecond"], style_median["filesPerSecond"]
        )

        corpus_rows.append(
            {
                "corpusId": corpus_id,
                "files": csslint_corpus["files"],
                "totalBytes": csslint_corpus["totalBytes"],
                "corpusDigest": csslint_corpus["corpusDigest"],
                "csslint": {
                    "totalMs": css_median["totalMs"],
                    "filesPerSecond": css_median["filesPerSecond"],
                    "mbPerSecond": css_median["mbPerSecond"],
                    "p50FileMs": css_median["p50FileMs"],
                    "p95FileMs": css_median["p95FileMs"],
                    "peakRssBytes": css_median.get("peakRssBytes", 0),
                    "parseMs": css_median["parseMs"],
                    "semanticMs": css_median["semanticMs"],
                    "rulesMs": css_median["rulesMs"],
                },
                "stylelint": {
                    "totalMs": style_median["totalMs"],
                    "filesPerSecond": style_median["filesPerSecond"],
                    "mbPerSecond": style_median["mbPerSecond"],
                    "p50FileMs": style_median["p50FileMs"],
                    "p95FileMs": style_median["p95FileMs"],
                    "peakRssBytes": style_median["peakRssBytes"],
                },
                "ratios": {
                    "runtime": runtime_ratio,
                    "throughput": throughput_ratio,
                },
            }
        )

    payload = {
        "schemaVersion": 1,
        "toolVersions": {
            "csslint": "workspace",
            "stylelint": stylelint_payload.get("stylelintVersion", "unknown"),
        },
        "protocol": csslint_payload["protocol"],
        "budgets": {
            "runtimeRegressionPercent": args.runtime_budget_percent,
            "memoryRegressionPercent": args.memory_budget_percent,
        },
        "corpora": corpus_rows,
    }

    output_json = Path(args.output_json)
    output_md = Path(args.output_md)
    output_json.parent.mkdir(parents=True, exist_ok=True)
    output_md.parent.mkdir(parents=True, exist_ok=True)

    output_json.write_text(json.dumps(payload, indent=2) + "\n")
    output_md.write_text(render_markdown(payload))
    print(f"wrote {output_json}")
    print(f"wrote {output_md}")
    return 0


def ratio(numerator: float, denominator: float) -> float:
    if denominator == 0:
        return 0.0
    return numerator / denominator


def render_markdown(payload: dict) -> str:
    lines = [
        "# Perf Summary",
        "",
        f"- stylelint version: `{payload['toolVersions']['stylelint']}`",
        (
            "- protocol: "
            f"{payload['protocol']['coldIterations']} cold / "
            f"{payload['protocol']['warmIterations']} warm iterations"
        ),
        "",
        "| Corpus | Files | csslint ms | stylelint ms | Runtime ratio | Throughput ratio |",
        "| --- | ---: | ---: | ---: | ---: | ---: |",
    ]

    for corpus in payload["corpora"]:
        lines.append(
            "| {corpus} | {files} | {css_ms:.2f} | {style_ms:.2f} | {runtime:.2f}x | {throughput:.2f}x |".format(
                corpus=corpus["corpusId"],
                files=corpus["files"],
                css_ms=corpus["csslint"]["totalMs"],
                style_ms=corpus["stylelint"]["totalMs"],
                runtime=corpus["ratios"]["runtime"],
                throughput=corpus["ratios"]["throughput"],
            )
        )

    lines.append("")
    return "\n".join(lines)


if __name__ == "__main__":
    raise SystemExit(main())
