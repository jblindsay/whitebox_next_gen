#!/usr/bin/env python3
"""Compare wbphotogrammetry benchmark JSON reports.

This utility compares two directories produced by
examples/run_profile_benchmark.sh and prints per-stage percentage deltas.
Negative deltas indicate faster candidate timings.

Usage:
  python crates/wbphotogrammetry/examples/compare_profile_reports.py \
    --baseline /tmp/wb_bench_base \
    --candidate /tmp/wb_bench_new
"""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, Iterable, List, Tuple


STAGES = ("feature", "alignment", "dense", "mosaic")


@dataclass(frozen=True)
class ReportKey:
    profile: str
    frame_count: int
    repeats: int
    resolution_m: float


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Compare wbphotogrammetry profile JSON reports.")
    parser.add_argument("--baseline", required=True, help="Directory with baseline JSON reports.")
    parser.add_argument("--candidate", required=True, help="Directory with candidate JSON reports.")
    parser.add_argument(
        "--metric",
        choices=("mean", "p50", "p95"),
        default="mean",
        help="Summary statistic to compare. Default: mean",
    )
    parser.add_argument(
        "--fail-if-regression",
        action="store_true",
        help="Exit non-zero if candidate regresses any stage on any shared benchmark key.",
    )
    parser.add_argument(
        "--regression-threshold-pct",
        type=float,
        default=0.0,
        help="Allowed positive regression percentage before failure when --fail-if-regression is enabled. Default: 0.0",
    )
    return parser.parse_args()


def load_reports(root: Path) -> Dict[ReportKey, Dict]:
    reports: Dict[ReportKey, Dict] = {}
    for path in sorted(root.glob("*.json")):
        payload = json.loads(path.read_text(encoding="utf-8"))
        cfg = payload.get("config", {})
        key = ReportKey(
            profile=str(cfg.get("profile", "unknown")),
            frame_count=int(cfg.get("frame_count", -1)),
            repeats=int(cfg.get("repeats", -1)),
            resolution_m=float(cfg.get("resolution_m", -1.0)),
        )
        reports[key] = payload
    return reports


def stage_value(payload: Dict, stage: str, metric: str) -> float:
    summary = payload["summary"][stage]
    suffix = "_s"
    if metric == "mean":
        return float(summary[f"mean{suffix}"])
    if metric == "p50":
        return float(summary[f"p50{suffix}"])
    return float(summary[f"p95{suffix}"])


def pct_delta(baseline: float, candidate: float) -> float:
    if baseline == 0.0:
        return 0.0 if candidate == 0.0 else float("inf")
    return ((candidate - baseline) / baseline) * 100.0


def render(rows: List[Tuple[str, str, float, float, float]]) -> None:
    print("benchmark_key | stage | baseline_s | candidate_s | delta_pct")
    print("-" * 86)
    for key, stage, b, c, d in rows:
        trend = "improved" if d < 0 else ("regressed" if d > 0 else "flat")
        print(f"{key} | {stage:<9} | {b:10.6f} | {c:11.6f} | {d:9.2f}% ({trend})")


def format_key(key: ReportKey) -> str:
    return f"{key.profile}:{key.frame_count}f:{key.repeats}r:{key.resolution_m:g}m"


def main() -> int:
    args = parse_args()
    baseline_dir = Path(args.baseline).expanduser().resolve()
    candidate_dir = Path(args.candidate).expanduser().resolve()

    baseline = load_reports(baseline_dir)
    candidate = load_reports(candidate_dir)

    shared = sorted(set(baseline.keys()) & set(candidate.keys()), key=lambda k: (k.profile, k.frame_count, k.repeats, k.resolution_m))
    if not shared:
        print("No shared benchmark keys found between baseline and candidate directories.")
        return 1

    rows: List[Tuple[str, str, float, float, float]] = []
    regressions: List[Tuple[str, str, float]] = []

    for key in shared:
        base_payload = baseline[key]
        cand_payload = candidate[key]
        key_label = format_key(key)

        for stage in STAGES:
            b = stage_value(base_payload, stage, args.metric)
            c = stage_value(cand_payload, stage, args.metric)
            d = pct_delta(b, c)
            rows.append((key_label, stage, b, c, d))
            if d > args.regression_threshold_pct:
                regressions.append((key_label, stage, d))

    render(rows)

    if args.fail_if_regression and regressions:
        print("\nRegression threshold exceeded:")
        for key_label, stage, d in regressions:
            print(f"  - {key_label} stage={stage} delta={d:.2f}%")
        return 2

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
