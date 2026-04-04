#!/usr/bin/env python3
"""Run a wbphotogrammetry benchmark matrix across many datasets.

This script discovers dataset folders (e.g. sibling folders next to Toledo),
runs `run_dataset_pipeline` for each dataset, and writes a single matrix
summary JSON.

Large-dataset safeguards:
- `--max-images-per-dataset`: cap frame count via a staged symlink subset.
- `--max-dataset-gb`: skip datasets larger than this size.
- `--max-total-gb`: stop when cumulative selected dataset size exceeds budget.
- `--dry-run`: inventory only, no pipeline execution.

Example:
  python crates/wbphotogrammetry/examples/run_dataset_matrix.py \
    --datasets-root /Users/johnlindsay/Documents/programming/Rust/drone_sfm_real_flight/datasets \
    --out-dir target/wbphotogrammetry_dataset_matrix \
    --profile balanced --feature-method rootsift --resolution 0.1 \
    --max-images-per-dataset 120 --max-dataset-gb 8 --max-total-gb 20
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import tempfile
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Tuple

SUPPORTED_EXTS = {".jpg", ".jpeg", ".png", ".tif", ".tiff"}
WORKSPACE_ROOT = Path(__file__).resolve().parents[3]


@dataclass
class DatasetPlan:
    name: str
    dataset_root: str
    images_dir: str
    image_count: int
    bytes_total: int
    selected: bool
    selection_reason: str
    staged_images_dir: Optional[str] = None
    staged_image_count: Optional[int] = None


@dataclass
class DatasetRunResult:
    name: str
    images_dir: str
    out_dir: str
    report_path: Optional[str]
    qa_status: Optional[str]
    qa_actions_count: Optional[int]
    frame_count: Optional[int]
    timing_total_s: Optional[float]
    dsm_valid_cells: Optional[int]
    dsm_vertical_rmse_m: Optional[float]
    low_confidence_cells_pct: Optional[float]
    mvs_mean_reference_completeness_pct: Optional[float]
    mosaic_max_seam_delta: Optional[float]
    success: bool
    error: Optional[str]


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Run wbphotogrammetry across a dataset matrix.")
    p.add_argument("--datasets-root", required=True, help="Root folder containing dataset subfolders")
    p.add_argument("--out-dir", required=True, help="Directory for per-dataset outputs and matrix summary")
    p.add_argument("--profile", default="balanced", help="Processing profile: fast|balanced|survey")
    p.add_argument("--feature-method", default="rootsift", help="Feature method for run_dataset_pipeline")
    p.add_argument("--camera-model", default="auto", help="Camera model for run_dataset_pipeline")
    p.add_argument("--resolution", type=float, default=0.1, help="Dense resolution in metres")
    p.add_argument("--reduced-solver-mode", default="sparse-pcg", help="Reduced solver mode")
    p.add_argument("--include-regex", default="", help="Only include dataset names matching this regex")
    p.add_argument("--exclude-regex", default="", help="Exclude dataset names matching this regex")
    p.add_argument("--max-datasets", type=int, default=0, help="Max selected datasets (0 = no limit)")
    p.add_argument("--max-images-per-dataset", type=int, default=0, help="Frame cap per dataset (0 = no cap)")
    p.add_argument("--max-dataset-gb", type=float, default=0.0, help="Skip datasets larger than this size in GiB (0 = no limit)")
    p.add_argument("--max-total-gb", type=float, default=0.0, help="Stop selecting datasets when cumulative selected size exceeds this GiB budget (0 = no limit)")
    p.add_argument("--dry-run", action="store_true", help="Inventory/plan only, do not execute pipeline")
    return p.parse_args()


def list_image_files(images_dir: Path) -> List[Path]:
    files = []
    for p in sorted(images_dir.iterdir()):
        if p.is_file() and p.suffix.lower() in SUPPORTED_EXTS:
            files.append(p)
    return files


def dir_size_bytes(paths: Sequence[Path]) -> int:
    total = 0
    for p in paths:
        try:
            total += p.stat().st_size
        except OSError:
            continue
    return total


def discover_dataset_images_dir(dataset_root: Path) -> Optional[Path]:
    images_dir = dataset_root / "images"
    if images_dir.is_dir():
        files = list_image_files(images_dir)
        if files:
            return images_dir
    if dataset_root.is_dir():
        files = list_image_files(dataset_root)
        if files:
            return dataset_root
    return None


def select_frame_subset(files: List[Path], max_images: int) -> List[Path]:
    if max_images <= 0 or len(files) <= max_images:
        return files
    # Uniform stride selection preserves temporal/spatial spread better than head-truncation.
    stride = len(files) / float(max_images)
    out = []
    for i in range(max_images):
        idx = min(int(round(i * stride)), len(files) - 1)
        out.append(files[idx])
    # Deduplicate any rounded collisions while preserving order.
    seen = set()
    uniq = []
    for f in out:
        if f in seen:
            continue
        seen.add(f)
        uniq.append(f)
    return uniq


def stage_symlink_subset(files: List[Path], dataset_name: str) -> Tuple[tempfile.TemporaryDirectory, Path]:
    tmp = tempfile.TemporaryDirectory(prefix=f"wbmatrix_{dataset_name}_")
    stage_dir = Path(tmp.name) / "images"
    stage_dir.mkdir(parents=True, exist_ok=True)
    for i, src in enumerate(files):
        dst = stage_dir / f"{i:05d}_{src.name}"
        os.symlink(src, dst)
    return tmp, stage_dir


def discover_plans(args: argparse.Namespace) -> List[DatasetPlan]:
    root = Path(args.datasets_root).expanduser().resolve()
    include_rx = re.compile(args.include_regex) if args.include_regex else None
    exclude_rx = re.compile(args.exclude_regex) if args.exclude_regex else None

    plans: List[DatasetPlan] = []
    selected_count = 0
    selected_bytes = 0
    max_dataset_bytes = int(args.max_dataset_gb * (1024 ** 3)) if args.max_dataset_gb > 0 else 0
    max_total_bytes = int(args.max_total_gb * (1024 ** 3)) if args.max_total_gb > 0 else 0

    for child in sorted(root.iterdir()):
        if not child.is_dir():
            continue
        name = child.name
        if include_rx and not include_rx.search(name):
            continue
        if exclude_rx and exclude_rx.search(name):
            continue

        images_dir = discover_dataset_images_dir(child)
        if images_dir is None:
            continue
        files = list_image_files(images_dir)
        if not files:
            continue

        bytes_total = dir_size_bytes(files)
        selected = True
        reason = "selected"

        if args.max_datasets > 0 and selected_count >= args.max_datasets:
            selected = False
            reason = "max_datasets_limit"
        elif max_dataset_bytes > 0 and bytes_total > max_dataset_bytes:
            selected = False
            reason = "max_dataset_gb_exceeded"
        elif max_total_bytes > 0 and (selected_bytes + bytes_total) > max_total_bytes:
            selected = False
            reason = "max_total_gb_exceeded"

        if selected:
            selected_count += 1
            selected_bytes += bytes_total

        plans.append(
            DatasetPlan(
                name=name,
                dataset_root=str(child),
                images_dir=str(images_dir),
                image_count=len(files),
                bytes_total=bytes_total,
                selected=selected,
                selection_reason=reason,
            )
        )

    return plans


def run_one_dataset(args: argparse.Namespace, plan: DatasetPlan, matrix_out_dir: Path) -> DatasetRunResult:
    dataset_out = matrix_out_dir / plan.name
    dataset_out.mkdir(parents=True, exist_ok=True)

    images_dir = Path(plan.images_dir)
    files = list_image_files(images_dir)
    selected_files = select_frame_subset(files, args.max_images_per_dataset)

    stage_tmp = None
    effective_images_dir = images_dir
    if len(selected_files) < len(files):
        stage_tmp, effective_images_dir = stage_symlink_subset(selected_files, plan.name)
        plan.staged_images_dir = str(effective_images_dir)
        plan.staged_image_count = len(selected_files)

    cmd = [
        "cargo", "run", "-p", "wbphotogrammetry", "--example", "run_dataset_pipeline", "--",
        "--images-dir", str(effective_images_dir),
        "--out-dir", str(dataset_out),
        "--profile", args.profile,
        "--feature-method", args.feature_method,
        "--camera-model", args.camera_model,
        "--resolution", f"{args.resolution}",
        "--reduced-solver-mode", args.reduced_solver_mode,
    ]

    try:
        proc = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            check=True,
            cwd=str(WORKSPACE_ROOT),
        )
    except subprocess.CalledProcessError as e:
        if stage_tmp is not None:
            stage_tmp.cleanup()
        return DatasetRunResult(
            name=plan.name,
            images_dir=str(effective_images_dir),
            out_dir=str(dataset_out),
            report_path=None,
            qa_status=None,
            qa_actions_count=None,
            frame_count=None,
            timing_total_s=None,
            dsm_valid_cells=None,
            dsm_vertical_rmse_m=None,
            low_confidence_cells_pct=None,
            mvs_mean_reference_completeness_pct=None,
            mosaic_max_seam_delta=None,
            success=False,
            error=(e.stderr or e.stdout or str(e)).strip()[:4000],
        )

    if stage_tmp is not None:
        stage_tmp.cleanup()

    report_path = dataset_out / f"{args.profile}_{args.feature_method}_report.json"
    if not report_path.exists():
        return DatasetRunResult(
            name=plan.name,
            images_dir=str(effective_images_dir),
            out_dir=str(dataset_out),
            report_path=None,
            qa_status=None,
            qa_actions_count=None,
            frame_count=None,
            timing_total_s=None,
            dsm_valid_cells=None,
            dsm_vertical_rmse_m=None,
            low_confidence_cells_pct=None,
            mvs_mean_reference_completeness_pct=None,
            mosaic_max_seam_delta=None,
            success=False,
            error=(proc.stdout + "\n" + proc.stderr).strip()[-4000:],
        )

    payload = json.loads(report_path.read_text(encoding="utf-8"))
    timing = payload.get("timing", {}) if isinstance(payload, dict) else {}
    total_s = float(timing.get("ingest_s", 0.0)) + float(timing.get("feature_s", 0.0)) + float(timing.get("alignment_s", 0.0)) + float(timing.get("dense_s", 0.0)) + float(timing.get("mosaic_s", 0.0))
    dsm = payload.get("dsm_stats", {}) if isinstance(payload, dict) else {}
    mosaic_stats = payload.get("mosaic_stats", {}) if isinstance(payload, dict) else {}
    qa = payload.get("qa", {}) if isinstance(payload, dict) else {}

    return DatasetRunResult(
        name=plan.name,
        images_dir=str(effective_images_dir),
        out_dir=str(dataset_out),
        report_path=str(report_path),
        qa_status=qa.get("status"),
        qa_actions_count=len(qa.get("recommended_actions", [])) if isinstance(qa.get("recommended_actions"), list) else None,
        frame_count=payload.get("frame_count"),
        timing_total_s=total_s,
        dsm_valid_cells=dsm.get("valid_cells"),
        dsm_vertical_rmse_m=dsm.get("vertical_rmse_m"),
        low_confidence_cells_pct=dsm.get("low_confidence_cells_pct"),
        mvs_mean_reference_completeness_pct=dsm.get("mvs_mean_reference_completeness_pct"),
        mosaic_max_seam_delta=mosaic_stats.get("max_seam_delta"),
        success=True,
        error=None,
    )


def summarize_results(results: List[DatasetRunResult]) -> Dict[str, object]:
    ok = [r for r in results if r.success]
    failed = [r for r in results if not r.success]
    qa_counts: Dict[str, int] = {}
    for r in ok:
        k = r.qa_status or "unknown"
        qa_counts[k] = qa_counts.get(k, 0) + 1
    return {
        "dataset_runs": len(results),
        "successful": len(ok),
        "failed": len(failed),
        "qa_status_counts": qa_counts,
        "mean_timing_total_s": (sum(r.timing_total_s or 0.0 for r in ok) / len(ok)) if ok else 0.0,
        "mean_mvs_reference_completeness_pct": (
            sum((r.mvs_mean_reference_completeness_pct or 0.0) for r in ok) / len(ok)
        ) if ok else 0.0,
        "mean_vertical_rmse_m": (
            sum((r.dsm_vertical_rmse_m or 0.0) for r in ok) / len(ok)
        ) if ok else 0.0,
        "max_mosaic_seam_delta": max((r.mosaic_max_seam_delta or 0.0) for r in ok) if ok else 0.0,
    }


def main() -> int:
    args = parse_args()
    out_dir = Path(args.out_dir).expanduser().resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    plans = discover_plans(args)
    selected = [p for p in plans if p.selected]

    matrix = {
        "config": {
            "datasets_root": str(Path(args.datasets_root).expanduser().resolve()),
            "out_dir": str(out_dir),
            "profile": args.profile,
            "feature_method": args.feature_method,
            "camera_model": args.camera_model,
            "resolution": args.resolution,
            "reduced_solver_mode": args.reduced_solver_mode,
            "max_datasets": args.max_datasets,
            "max_images_per_dataset": args.max_images_per_dataset,
            "max_dataset_gb": args.max_dataset_gb,
            "max_total_gb": args.max_total_gb,
            "dry_run": args.dry_run,
        },
        "plans": [asdict(p) for p in plans],
        "results": [],
        "summary": {},
    }

    if args.dry_run:
        matrix["summary"] = {
            "datasets_discovered": len(plans),
            "datasets_selected": len(selected),
            "dry_run_only": True,
        }
        out_path = out_dir / "dataset_matrix_summary.json"
        out_path.write_text(json.dumps(matrix, indent=2) + "\n", encoding="utf-8")
        print(f"[dry-run] discovered={len(plans)} selected={len(selected)}")
        print(f"summary: {out_path}")
        return 0

    results: List[DatasetRunResult] = []
    for p in selected:
        print(f"[run] dataset={p.name} images={p.image_count} size_gb={p.bytes_total / (1024 ** 3):.2f}")
        r = run_one_dataset(args, p, out_dir)
        results.append(r)
        status = "ok" if r.success else "failed"
        print(f"[done] dataset={p.name} status={status} qa={r.qa_status} total_s={r.timing_total_s}")

    matrix["results"] = [asdict(r) for r in results]
    matrix["summary"] = summarize_results(results)

    out_path = out_dir / "dataset_matrix_summary.json"
    out_path.write_text(json.dumps(matrix, indent=2) + "\n", encoding="utf-8")
    print(f"summary: {out_path}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
