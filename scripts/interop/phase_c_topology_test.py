#!/usr/bin/env python3
"""Phase C topology stress runner (TC01-TC07 synthetic corpus).

This runner executes topology-sensitive operations on synthetic fixtures and
writes per-case artifacts under artifacts/interop/results/topology.
"""

from __future__ import annotations

import json
import argparse
import shutil
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional, Tuple

ROOT = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen")
SYNTHETIC_DIR = ROOT / "artifacts/interop/topology/corpus/synthetic"
COMPLEX_DIR = ROOT / "artifacts/interop/topology/corpus/complex"
RESULTS_ROOT = ROOT / "artifacts/interop/results/topology"
SUMMARY_JSON = ROOT / "artifacts/interop/results/phase_c_topology_results.json"

CASE_FILES = {
    "TC01": "TC01_self_intersection_bow_tie.geojson",
    "TC02": "TC02_nearly_coincident_edges_sliver.geojson",
    "TC03": "TC03_ring_orientation_anomalies.geojson",
    "TC04": "TC04_duplicate_near_duplicate_vertices.geojson",
    "TC05": "TC05_tiny_gaps_and_overlaps.geojson",
    "TC06": "TC06_point_touch_boundaries.geojson",
    "TC07": "TC07_multipart_edge_cases.geojson",
}


def run_cmd(cmd: str, timeout: int = 90) -> Tuple[int, str, str]:
    try:
        proc = subprocess.run(
            cmd,
            shell=True,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        return proc.returncode, proc.stdout.strip(), proc.stderr.strip()
    except subprocess.TimeoutExpired:
        return 1, "", "Timeout"
    except Exception as exc:
        return 1, "", str(exc)


def get_wbw_env():
    try:
        import whitebox_workflows as wbw

        return wbw.WbEnvironment(), None
    except Exception as exc:
        return None, str(exc)


def extract_layer_name(vector_path: Path) -> Optional[str]:
    rc, out, _ = run_cmd(f"ogrinfo -ro '{vector_path}'")
    if rc != 0:
        return None
    for line in out.splitlines():
        line = line.strip()
        if line.startswith("Layer name:"):
            return line.split(":", 1)[1].strip()
        # Common ogrinfo summary format: `1: layer_name (GeometryType)`
        if ":" in line and line[:1].isdigit():
            try:
                return line.split(":", 1)[1].strip().split(" ", 1)[0]
            except Exception:
                continue
    return None


def feature_count(vector_path: Path) -> Optional[int]:
    rc, out, _ = run_cmd(f"ogrinfo -ro -so -al '{vector_path}'")
    if rc != 0:
        return None
    for line in out.splitlines():
        line = line.strip()
        if line.lower().startswith("feature count:"):
            try:
                return int(line.split(":", 1)[1].strip())
            except Exception:
                return None
    return None


def invalid_geometry_count(vector_path: Path, layer_name: str) -> Optional[int]:
    sql = (
        f"SELECT SUM(CASE WHEN ST_IsValid(geometry)=0 THEN 1 ELSE 0 END) AS invalid_n "
        f"FROM {layer_name}"
    )
    rc, out, _ = run_cmd(
        f"ogrinfo '{vector_path}' -dialect sqlite -sql \"{sql}\""
    )
    if rc != 0:
        return None
    for line in out.splitlines():
        line = line.strip()
        if line.lower().startswith("invalid_n"):
            # invalid_n (Integer) = 1
            try:
                return int(line.split("=")[-1].strip())
            except Exception:
                return None
    return None


def ensure_clean(path: Path) -> None:
    if path.exists():
        if path.is_dir():
            shutil.rmtree(path)
        else:
            path.unlink()


def run_sql_to_gpkg(src: Path, sql: str, out_gpkg: Path, timeout: int = 120) -> Tuple[str, str]:
    ensure_clean(out_gpkg)
    rc, out, err = run_cmd(
        f"ogr2ogr -f GPKG '{out_gpkg}' '{src}' -dialect sqlite -sql \"{sql}\"",
        timeout=timeout,
    )
    if rc != 0:
        return "FAIL", err or out or "ogr2ogr failed"
    cnt = feature_count(out_gpkg)
    return "PASS", f"output_features={cnt if cnt is not None else 'unknown'}"


def run_case(case_id: str, src_file: Path, env, corpus_kind: str) -> Dict[str, object]:
    case_dir = RESULTS_ROOT / corpus_kind / case_id
    case_dir.mkdir(parents=True, exist_ok=True)

    layer_name = extract_layer_name(src_file)
    if not layer_name:
        return {
            "status": "FAIL",
            "note": "Could not detect input layer name",
            "operations": {},
        }

    input_fc = feature_count(src_file)
    invalid_n = invalid_geometry_count(src_file, layer_name)

    ops: Dict[str, Dict[str, str]] = {}

    # 1) Buffer
    ops["buffer"] = {}
    s, n = run_sql_to_gpkg(
        src_file,
        f"SELECT ST_Buffer(geometry, 0.0001) AS geometry, * FROM {layer_name}",
        case_dir / "buffer.gpkg",
    )
    ops["buffer"] = {"status": s, "note": n}

    # 2) Simplify
    s, n = run_sql_to_gpkg(
        src_file,
        f"SELECT ST_Simplify(geometry, 0.0000001) AS geometry, * FROM {layer_name}",
        case_dir / "simplify.gpkg",
    )
    ops["simplify"] = {"status": s, "note": n}

    # 3) Dissolve / Union
    if (input_fc or 0) > 1:
        s, n = run_sql_to_gpkg(
            src_file,
            f"SELECT ST_Union(geometry) AS geometry FROM {layer_name}",
            case_dir / "union.gpkg",
        )
        ops["union"] = {"status": s, "note": n}
    else:
        ops["union"] = {"status": "NOT_APPLICABLE", "note": "single-feature input"}

    # 4) Pairwise intersection stress
    if (input_fc or 0) > 1:
        s, n = run_sql_to_gpkg(
            src_file,
            (
                f"SELECT a.id AS aid, b.id AS bid, ST_Intersection(a.geometry, b.geometry) AS geometry "
                f"FROM {layer_name} a JOIN {layer_name} b ON a.id < b.id"
            ),
            case_dir / "intersection_pairs.gpkg",
        )
        ops["intersection_pairs"] = {"status": s, "note": n}
    else:
        ops["intersection_pairs"] = {"status": "NOT_APPLICABLE", "note": "single-feature input"}

    # 5) wbvector read/write sanity on synthetic fixture
    rw_status = "PASS"
    rw_note = ""
    try:
        vec = env.read_vector(str(src_file))
        rw_out = case_dir / "wbw_roundtrip.geojson"
        env.write_vector(vec, str(rw_out))
        out_vec = env.read_vector(str(rw_out))
        if vec.feature_count() != out_vec.feature_count():
            rw_status = "FAIL"
            rw_note = f"feature count mismatch {vec.feature_count()} vs {out_vec.feature_count()}"
        else:
            rw_note = f"features={vec.feature_count()}"
    except Exception as exc:
        rw_status = "FAIL"
        rw_note = str(exc)
    ops["wbw_roundtrip"] = {"status": rw_status, "note": rw_note}

    failing_ops = [k for k, v in ops.items() if v["status"] == "FAIL"]
    case_status = "PASS" if not failing_ops else "FAIL"

    case_result = {
        "status": case_status,
        "input_file": str(src_file),
        "layer_name": layer_name,
        "input_feature_count": input_fc,
        "input_invalid_geometry_count": invalid_n,
        "operations": ops,
        "failed_operations": failing_ops,
    }

    (case_dir / "results.json").write_text(
        json.dumps(case_result, indent=2) + "\n", encoding="utf-8"
    )
    return case_result


def main() -> int:
    parser = argparse.ArgumentParser(description="Run Phase C topology synthetic/complex corpus")
    parser.add_argument(
        "--corpus",
        choices=["synthetic", "complex", "all"],
        default="synthetic",
        help="Which corpus to run",
    )
    args = parser.parse_args()

    print("=" * 70)
    print(f"Phase C Topology Runner ({args.corpus})")
    print(f"Timestamp: {datetime.now().isoformat()}")
    print("=" * 70)

    if shutil.which("ogrinfo") is None or shutil.which("ogr2ogr") is None:
        print("FAIL: ogrinfo/ogr2ogr not found on PATH")
        return 1

    env, err = get_wbw_env()
    if env is None:
        print(f"FAIL: whitebox_workflows import failed: {err}")
        return 1

    RESULTS_ROOT.mkdir(parents=True, exist_ok=True)

    corpora = []
    if args.corpus in ("synthetic", "all"):
        corpora.append(("synthetic", SYNTHETIC_DIR))
    if args.corpus in ("complex", "all"):
        corpora.append(("complex", COMPLEX_DIR))

    results: Dict[str, Dict[str, object]] = {}
    for corpus_kind, corpus_dir in corpora:
        for case_id in sorted(CASE_FILES.keys()):
            src = corpus_dir / CASE_FILES[case_id]
            result_key = f"{corpus_kind}:{case_id}"
            print(f"{result_key}: {src.name}...", end=" ", flush=True)
            if not src.exists():
                results[result_key] = {
                    "status": "NOT_STARTED",
                    "note": f"missing fixture: {src}",
                    "operations": {},
                }
                print("⊘ NOT_STARTED")
                continue

            result = run_case(case_id, src, env, corpus_kind)
            results[result_key] = result
            if result["status"] == "PASS":
                print("✓ PASS")
            else:
                print("✗ FAIL")

    passed = sum(1 for v in results.values() if v["status"] == "PASS")
    failed = sum(1 for v in results.values() if v["status"] == "FAIL")
    skipped = sum(1 for v in results.values() if v["status"] == "NOT_STARTED")

    summary = {
        "phase": "C",
        "mode": args.corpus,
        "timestamp": datetime.now().isoformat(),
        "counts": {
            "total": len(results),
            "passed": passed,
            "failed": failed,
            "not_started": skipped,
        },
        "results": results,
    }

    SUMMARY_JSON.parent.mkdir(parents=True, exist_ok=True)
    SUMMARY_JSON.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")

    print("\n" + "=" * 70)
    print("Summary")
    print("=" * 70)
    print(f"Total:       {len(results)} cases")
    print(f"Passed:      {passed}")
    print(f"Failed:      {failed}")
    print(f"Not Started: {skipped}")
    print(f"Results saved: {SUMMARY_JSON}")

    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
