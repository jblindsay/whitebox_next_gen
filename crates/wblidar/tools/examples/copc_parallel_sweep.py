#!/usr/bin/env python3
"""Run COPC serial baseline + parallel threshold sweeps and report timings.

Usage:
  python3 crates/wblidar/examples/copc_parallel_sweep.py \
    --input /path/to/input.las \
    --out-dir /path/to/output_dir \
    [--qgis-ref /path/to/reference.copc.laz]
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import shutil
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List


@dataclass
class RunResult:
    name: str
    mode: str
    min_nodes: int | None
    min_points: int | None
    sort_min_points: int | None
    seconds: float
    report_csv: Path
    parity_ok: bool


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser()
    p.add_argument("--input", required=True, type=Path, help="Input LAS path")
    p.add_argument("--out-dir", required=True, type=Path, help="Output directory for reports/files")
    p.add_argument("--qgis-ref", type=Path, default=None, help="Optional COPC reference file")
    return p.parse_args()


def run_benchmark(
    repo_root: Path,
    input_path: Path,
    out_prefix: Path,
    report_csv: Path,
    qgis_ref: Path | None,
    parallel: bool,
    env_overrides: Dict[str, str] | None,
) -> float:
    cmd = [
        "cargo",
        "run",
        "--release",
        "-p",
        "wblidar",
    ]
    if parallel:
        cmd.extend(["--features", "copc-parallel"])
    cmd.extend([
        "--example",
        "copc_parity_benchmark_csv",
        "--",
        str(input_path),
        str(out_prefix),
        str(qgis_ref) if qgis_ref is not None else "",
        str(report_csv),
    ])

    env = os.environ.copy()
    if env_overrides:
        env.update(env_overrides)

    t0 = time.perf_counter()
    proc = subprocess.run(cmd, cwd=repo_root, env=env, capture_output=True, text=True)
    t1 = time.perf_counter()
    if proc.returncode != 0:
        print(proc.stdout)
        print(proc.stderr)
        raise RuntimeError(f"benchmark command failed ({proc.returncode})")

    return t1 - t0


def load_metrics(csv_path: Path) -> Dict[str, Dict[str, str]]:
    with csv_path.open(newline="") as f:
        rows = list(csv.DictReader(f))

    keys = [
        "config",
        "bytes",
        "ratio_vs_source",
        "data_nodes",
        "total_points",
        "max_points_per_node",
        "compression_level",
    ]
    return {r["config"]: {k: r[k] for k in keys} for r in rows}


def parity_matches(a: Path, b: Path) -> bool:
    return load_metrics(a) == load_metrics(b)


def main() -> None:
    args = parse_args()
    repo_root = Path(__file__).resolve().parents[3]

    if not args.input.exists():
        raise FileNotFoundError(args.input)

    out_dir = args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    qgis_ref = args.qgis_ref if args.qgis_ref and args.qgis_ref.exists() else None

    sweep = [
        {"name": "p1_default", "min_nodes": None, "min_points": None, "sort_min_points": None},
        {"name": "p2_low_gate", "min_nodes": 4, "min_points": 100_000, "sort_min_points": 30_000},
        {"name": "p3_mid_gate", "min_nodes": 8, "min_points": 200_000, "sort_min_points": 50_000},
        {"name": "p4_high_gate", "min_nodes": 16, "min_points": 400_000, "sort_min_points": 80_000},
    ]

    results: List[RunResult] = []

    # Warm build for both feature sets to reduce compile noise in timing.
    subprocess.run(["cargo", "build", "--release", "-p", "wblidar"], cwd=repo_root, check=True)
    subprocess.run(["cargo", "build", "--release", "-p", "wblidar", "--features", "copc-parallel"], cwd=repo_root, check=True)

    baseline_report = out_dir / "serial_baseline_report.csv"
    baseline_prefix = out_dir / "serial_baseline"
    baseline_secs = run_benchmark(
        repo_root,
        args.input,
        baseline_prefix,
        baseline_report,
        qgis_ref,
        parallel=False,
        env_overrides=None,
    )
    results.append(
        RunResult(
            name="serial_baseline",
            mode="serial",
            min_nodes=None,
            min_points=None,
            sort_min_points=None,
            seconds=baseline_secs,
            report_csv=baseline_report,
            parity_ok=True,
        )
    )

    for cfg in sweep:
        name = cfg["name"]
        report = out_dir / f"{name}_report.csv"
        prefix = out_dir / name

        env_overrides: Dict[str, str] = {}
        if cfg["min_nodes"] is not None:
            env_overrides["WBLIDAR_COPC_PARALLEL_MIN_NODES"] = str(cfg["min_nodes"])
        if cfg["min_points"] is not None:
            env_overrides["WBLIDAR_COPC_PARALLEL_MIN_POINTS"] = str(cfg["min_points"])
        if cfg["sort_min_points"] is not None:
            env_overrides["WBLIDAR_COPC_PARALLEL_SORT_MIN_POINTS"] = str(cfg["sort_min_points"])

        secs = run_benchmark(
            repo_root,
            args.input,
            prefix,
            report,
            qgis_ref,
            parallel=True,
            env_overrides=env_overrides,
        )
        same = parity_matches(baseline_report, report)
        results.append(
            RunResult(
                name=name,
                mode="parallel",
                min_nodes=cfg["min_nodes"],
                min_points=cfg["min_points"],
                sort_min_points=cfg["sort_min_points"],
                seconds=secs,
                report_csv=report,
                parity_ok=same,
            )
        )

    summary_csv = out_dir / "summary.csv"
    with summary_csv.open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow([
            "name",
            "mode",
            "min_nodes",
            "min_points",
            "sort_min_points",
            "seconds",
            "speedup_vs_serial",
            "parity_ok",
            "report_csv",
        ])
        serial_secs = results[0].seconds
        for r in results:
            speedup = serial_secs / r.seconds if r.seconds > 0 else 0.0
            w.writerow([
                r.name,
                r.mode,
                "" if r.min_nodes is None else r.min_nodes,
                "" if r.min_points is None else r.min_points,
                "" if r.sort_min_points is None else r.sort_min_points,
                f"{r.seconds:.3f}",
                f"{speedup:.4f}",
                "yes" if r.parity_ok else "no",
                str(r.report_csv),
            ])

    best_parallel = min((r for r in results if r.mode == "parallel"), key=lambda r: r.seconds)
    print(f"Serial baseline: {results[0].seconds:.3f}s")
    print(
        "Best parallel: "
        f"{best_parallel.name} at {best_parallel.seconds:.3f}s "
        f"({results[0].seconds / best_parallel.seconds:.3f}x) parity={best_parallel.parity_ok}"
    )
    print(f"Wrote summary: {summary_csv}")


if __name__ == "__main__":
    main()
