#!/usr/bin/env python3
"""Compare two wbphotogrammetry dataset pipeline reports.

This script compares JSON outputs produced by:
  cargo run -p wbphotogrammetry --example run_dataset_pipeline -- ...

It focuses on quality-sensitive metrics (especially dense/MVS-related fields)
plus key runtime numbers.

Usage:
  python crates/wbphotogrammetry/examples/compare_dataset_reports.py \
    --baseline /tmp/wb/base_report.json \
    --candidate /tmp/wb/cand_report.json
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple


# (label, json-path list, better-when-lower)
METRICS: List[Tuple[str, List[str], bool]] = [
    ("dsm.valid_cells", ["dsm_stats", "valid_cells"], False),
    ("dsm.vertical_rmse_m", ["dsm_stats", "vertical_rmse_m"], True),
    ("dsm.mean_local_relief_m", ["dsm_stats", "mean_local_relief_m"], False),
    ("dsm.p95_local_relief_m", ["dsm_stats", "p95_local_relief_m"], False),
    (
        "dsm.mvs_mean_reference_completeness_pct",
        ["dsm_stats", "mvs_mean_reference_completeness_pct"],
        False,
    ),
    ("dsm.low_confidence_cells_pct", ["dsm_stats", "low_confidence_cells_pct"], True),
    ("mosaic.coverage_pct", ["mosaic_coverage_pct"], False),
    ("qa.recommended_actions", ["qa", "recommended_actions"], True),
    ("timing.dense_s", ["timing", "dense_s"], True),
    ("timing.total_s", ["timing_total_s"], True),
]


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Compare wbphotogrammetry dataset reports.")
    p.add_argument("--baseline", required=True, help="Path to baseline report JSON")
    p.add_argument("--candidate", required=True, help="Path to candidate report JSON")
    p.add_argument(
        "--fail-on-regression",
        action="store_true",
        help="Exit non-zero if any metric regresses",
    )
    p.add_argument(
        "--regression-tolerance-pct",
        type=float,
        default=0.0,
        help="Allowed regression percent before failing (default: 0)",
    )
    return p.parse_args()


def load_json(path: Path) -> Dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def get_path(payload: Dict[str, Any], path: List[str]) -> Optional[float]:
    cur: Any = payload
    for key in path:
        if key == "timing_total_s":
            if not isinstance(cur, dict) or "timing" not in cur or not isinstance(cur["timing"], dict):
                return None
            t = cur["timing"]
            vals = [
                float(t.get("ingest_s", 0.0)),
                float(t.get("feature_s", 0.0)),
                float(t.get("alignment_s", 0.0)),
                float(t.get("dense_s", 0.0)),
                float(t.get("mosaic_s", 0.0)),
            ]
            return sum(vals)
        if not isinstance(cur, dict) or key not in cur:
            return None
        cur = cur[key]

    if isinstance(cur, list):
        return float(len(cur))
    if isinstance(cur, (int, float)):
        return float(cur)
    return None


def pct_delta(b: float, c: float, lower_is_better: bool) -> float:
    if b == 0.0:
        return 0.0 if c == 0.0 else float("inf")
    raw = ((c - b) / b) * 100.0
    return raw if lower_is_better else -raw


def trend_word(score_pct: float) -> str:
    if score_pct > 0:
        return "improved"
    if score_pct < 0:
        return "regressed"
    return "flat"


def fmt(v: Optional[float]) -> str:
    if v is None:
        return "n/a"
    return f"{v:.6f}"


def main() -> int:
    args = parse_args()
    base = load_json(Path(args.baseline).expanduser().resolve())
    cand = load_json(Path(args.candidate).expanduser().resolve())

    print("metric | baseline | candidate | normalized_delta_pct | trend")
    print("-" * 96)

    regressions: List[Tuple[str, float]] = []

    for label, path, lower_is_better in METRICS:
        b = get_path(base, path)
        c = get_path(cand, path)
        if b is None or c is None:
            print(f"{label} | {fmt(b)} | {fmt(c)} | n/a | missing")
            continue

        score = pct_delta(b, c, lower_is_better)
        print(f"{label} | {fmt(b)} | {fmt(c)} | {score: .2f}% | {trend_word(score)}")

        if score < -args.regression_tolerance_pct:
            regressions.append((label, score))

    if args.fail_on_regression and regressions:
        print("\nRegression threshold exceeded:")
        for label, score in regressions:
            print(f"  - {label}: {score:.2f}%")
        return 2

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
