#!/usr/bin/env python3
"""Enforce CI-ready quality gates from dataset_matrix_summary.json.

This script reads the summary JSON produced by run_dataset_matrix.py and exits
non-zero when configured thresholds are not met.

Milestone-2 gate focus:
1. MVS completeness proxy
2. Vertical error proxy
3. Seam artifact proxy

Example:
  python crates/wbphotogrammetry/examples/check_dataset_matrix_gates.py \
    --summary target/wbphotogrammetry_dataset_matrix_run1/dataset_matrix_summary.json \
    --min-successful-runs 4 \
    --min-mean-completeness-pct 12.0 \
    --max-mean-vertical-rmse-m 0.35 \
    --max-seam-delta 0.12
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Dict, List


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Check dataset matrix gates for CI.")
    p.add_argument("--summary", required=True, help="Path to dataset_matrix_summary.json")

    p.add_argument("--min-successful-runs", type=int, default=1)
    p.add_argument("--max-failed-runs", type=int, default=0)

    p.add_argument("--min-mean-completeness-pct", type=float, default=0.0)
    p.add_argument("--max-mean-vertical-rmse-m", type=float, default=1.0)
    p.add_argument("--max-seam-delta", type=float, default=0.2)

    p.add_argument("--fail-on-review", action="store_true", help="Fail if any run has qa_status=Review")
    p.add_argument("--fail-on-fail", action="store_true", help="Fail if any run has qa_status=Fail")
    return p.parse_args()


def load_json(path: Path) -> Dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def as_float(value: Any, default: float = 0.0) -> float:
    if isinstance(value, (int, float)):
        return float(value)
    return default


def optional_float(value: Any) -> float | None:
    if isinstance(value, (int, float)):
        return float(value)
    return None


def mean_from_results(results: List[Dict[str, Any]], key: str) -> float | None:
    vals: List[float] = []
    for row in results:
        if not isinstance(row, dict):
            continue
        value = optional_float(row.get(key))
        if value is not None:
            vals.append(value)
    if not vals:
        return None
    return sum(vals) / float(len(vals))


def max_from_results(results: List[Dict[str, Any]], key: str) -> float | None:
    vals: List[float] = []
    for row in results:
        if not isinstance(row, dict):
            continue
        value = optional_float(row.get(key))
        if value is not None:
            vals.append(value)
    if not vals:
        return None
    return max(vals)


def main() -> int:
    args = parse_args()
    payload = load_json(Path(args.summary).expanduser().resolve())

    summary = payload.get("summary", {}) if isinstance(payload, dict) else {}
    results = payload.get("results", []) if isinstance(payload, dict) else []
    if not isinstance(results, list):
        results = []

    successful = int(summary.get("successful", 0))
    failed = int(summary.get("failed", 0))

    mean_completeness = optional_float(summary.get("mean_mvs_reference_completeness_pct"))
    if mean_completeness is None:
        mean_completeness = mean_from_results(results, "mvs_mean_reference_completeness_pct")

    mean_vertical_rmse = optional_float(summary.get("mean_vertical_rmse_m"))
    if mean_vertical_rmse is None:
        mean_vertical_rmse = mean_from_results(results, "dsm_vertical_rmse_m")

    max_seam_delta = optional_float(summary.get("max_mosaic_seam_delta"))
    if max_seam_delta is None:
        max_seam_delta = max_from_results(results, "mosaic_max_seam_delta")

    qa_review = 0
    qa_fail = 0
    for row in results:
        if not isinstance(row, dict):
            continue
        status = row.get("qa_status")
        if status == "Review":
            qa_review += 1
        elif status == "Fail":
            qa_fail += 1

    failures: List[str] = []

    if successful < args.min_successful_runs:
        failures.append(
            f"successful runs {successful} is below minimum {args.min_successful_runs}"
        )

    if failed > args.max_failed_runs:
        failures.append(
            f"failed runs {failed} exceeds maximum {args.max_failed_runs}"
        )

    if mean_completeness is None:
        failures.append("missing completeness metric in summary/results")
    elif mean_completeness < args.min_mean_completeness_pct:
        failures.append(
            f"mean completeness {mean_completeness:.3f}% is below minimum {args.min_mean_completeness_pct:.3f}%"
        )

    if mean_vertical_rmse is None:
        failures.append("missing vertical RMSE metric in summary/results")
    elif mean_vertical_rmse > args.max_mean_vertical_rmse_m:
        failures.append(
            f"mean vertical RMSE {mean_vertical_rmse:.4f} m exceeds maximum {args.max_mean_vertical_rmse_m:.4f} m"
        )

    if max_seam_delta is None:
        failures.append("missing seam proxy metric in summary/results")
    elif max_seam_delta > args.max_seam_delta:
        failures.append(
            f"max seam delta {max_seam_delta:.4f} exceeds maximum {args.max_seam_delta:.4f}"
        )

    if args.fail_on_review and qa_review > 0:
        failures.append(f"qa review runs present: {qa_review}")

    if args.fail_on_fail and qa_fail > 0:
        failures.append(f"qa fail runs present: {qa_fail}")

    print("matrix_gate_summary")
    print(f"  successful: {successful}")
    print(f"  failed: {failed}")
    if mean_completeness is None:
        print("  mean_mvs_reference_completeness_pct: missing")
    else:
        print(f"  mean_mvs_reference_completeness_pct: {mean_completeness:.3f}")
    if mean_vertical_rmse is None:
        print("  mean_vertical_rmse_m: missing")
    else:
        print(f"  mean_vertical_rmse_m: {mean_vertical_rmse:.4f}")
    if max_seam_delta is None:
        print("  max_mosaic_seam_delta: missing")
    else:
        print(f"  max_mosaic_seam_delta: {max_seam_delta:.4f}")
    print(f"  qa_review_count: {qa_review}")
    print(f"  qa_fail_count: {qa_fail}")

    if failures:
        print("gate_result: FAIL")
        for item in failures:
            print(f"  - {item}")
        return 2

    print("gate_result: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
