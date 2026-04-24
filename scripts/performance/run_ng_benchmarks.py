#!/usr/bin/env python3
"""Run NG benchmark commands with hard-stop timeout caps.

Timeout cap per run:
    min(10 * legacy_runtime_s, legacy_runtime_s + 1800)
If legacy runtime is missing/invalid, fallback default cap is used.
"""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import subprocess
import time
from pathlib import Path


DEFAULT_CAP_S = 1800.0


def parse_bool(value: str) -> bool:
    return str(value).strip().lower() in {"1", "true", "yes", "y"}


def parse_float(value: str) -> float | None:
    try:
        out = float(value)
    except (TypeError, ValueError):
        return None
    if out <= 0 or not (out < float("inf")):
        return None
    return out


def compute_cap(legacy_runtime_s: float | None) -> float:
    if legacy_runtime_s is None:
        return DEFAULT_CAP_S
    return min(10.0 * legacy_runtime_s, legacy_runtime_s + 1800.0)


def run_one(command: str, timeout_s: float) -> tuple[str, float, int | None, str]:
    start = time.perf_counter()
    try:
        proc = subprocess.run(
            command,
            shell=True,
            executable="/bin/zsh",
            capture_output=True,
            text=True,
            timeout=timeout_s,
            check=False,
        )
        elapsed = time.perf_counter() - start
        output = (proc.stdout or "") + (proc.stderr or "")
        if proc.returncode == 0:
            return "PASS", elapsed, proc.returncode, output
        return "FAIL_EXIT", elapsed, proc.returncode, output
    except subprocess.TimeoutExpired as exc:
        elapsed = time.perf_counter() - start
        output = (exc.stdout or "") + (exc.stderr or "")
        return "FAIL_OVERRUN", elapsed, None, output


def main() -> int:
    parser = argparse.ArgumentParser(description="Run NG benchmarks with overrun protection")
    parser.add_argument(
        "--manifest",
        default="docs/performance/ng_benchmark_manifest.csv",
        help="CSV manifest with tool_name,enabled,dataset_id,legacy_runtime_s,ng_command",
    )
    parser.add_argument(
        "--results",
        default="docs/performance/ng_benchmark_results.csv",
        help="Output CSV for benchmark results",
    )
    parser.add_argument(
        "--max-runs",
        type=int,
        default=0,
        help="Optional limit of enabled runs (0 = all)",
    )
    args = parser.parse_args()

    manifest_path = Path(args.manifest)
    results_path = Path(args.results)

    if not manifest_path.exists():
        raise SystemExit(f"Manifest not found: {manifest_path}")

    rows = []
    with manifest_path.open("r", newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        for row in reader:
            rows.append(row)

    enabled_rows = [r for r in rows if parse_bool(r.get("enabled", ""))]
    if args.max_runs > 0:
        enabled_rows = enabled_rows[: args.max_runs]

    results_path.parent.mkdir(parents=True, exist_ok=True)
    write_header = not results_path.exists()

    with results_path.open("a", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(
            f,
            fieldnames=[
                "timestamp",
                "tool_name",
                "dataset_id",
                "legacy_runtime_s",
                "timeout_cap_s",
                "status",
                "next_gen_runtime_s",
                "delta_pct",
                "return_code",
                "command",
                "notes",
            ],
        )
        if write_header:
            writer.writeheader()

        for row in enabled_rows:
            tool_name = row.get("tool_name", "").strip()
            dataset_id = row.get("dataset_id", "").strip()
            command = row.get("ng_command", "").strip()
            notes = row.get("notes", "").strip()
            legacy_runtime = parse_float(row.get("legacy_runtime_s", ""))

            if not command:
                status = "SKIP_NO_COMMAND"
                elapsed = 0.0
                return_code = None
                output = ""
                timeout_cap = compute_cap(legacy_runtime)
            else:
                timeout_cap = compute_cap(legacy_runtime)
                status, elapsed, return_code, output = run_one(command, timeout_cap)

            delta_pct = ""
            if legacy_runtime and elapsed > 0 and status in {"PASS", "FAIL_EXIT", "FAIL_OVERRUN"}:
                delta_pct = f"{((elapsed - legacy_runtime) / legacy_runtime) * 100.0:.2f}"

            writer.writerow(
                {
                    "timestamp": dt.datetime.now().isoformat(timespec="seconds"),
                    "tool_name": tool_name,
                    "dataset_id": dataset_id,
                    "legacy_runtime_s": "" if legacy_runtime is None else f"{legacy_runtime:.6f}",
                    "timeout_cap_s": f"{timeout_cap:.3f}",
                    "status": status,
                    "next_gen_runtime_s": "" if elapsed <= 0 else f"{elapsed:.6f}",
                    "delta_pct": delta_pct,
                    "return_code": "" if return_code is None else str(return_code),
                    "command": command,
                    "notes": notes,
                }
            )

            # Keep terminal output tiny but useful for long batches.
            print(f"[{status}] {tool_name} ({dataset_id}) elapsed={elapsed:.3f}s cap={timeout_cap:.3f}s")
            if status in {"FAIL_EXIT", "FAIL_OVERRUN"} and output:
                preview = "\\n".join(output.splitlines()[-10:])
                print(preview)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
