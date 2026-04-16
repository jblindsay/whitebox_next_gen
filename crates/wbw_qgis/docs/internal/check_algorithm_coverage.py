#!/usr/bin/env python3
"""
Phase 1 algorithm-generation coverage check.

Run from the workspace root (with the correct Python environment active):

    python crates/wbw_qgis/docs/internal/check_algorithm_coverage.py

Exit code 0 = all thresholds met.
Exit code 1 = one or more thresholds violated (details printed to stdout).
"""
from __future__ import annotations

import collections
import sys
import os

# Allow running from workspace root or from within the plugin directory.
PLUGIN_ROOT = os.path.join(
    os.path.dirname(__file__), "..", "..", "plugin"
)
sys.path.insert(0, os.path.abspath(PLUGIN_ROOT))

from whitebox_workflows_qgis.discovery import discover_tool_catalog
from whitebox_workflows_qgis.algorithm import (
    _extract_enum_options,
    _infer_kind,
)

# ---------------------------------------------------------------------------
# Thresholds — tighten over time as coverage improves.
# ---------------------------------------------------------------------------
THRESHOLDS: dict[str, tuple[str, float]] = {
    # Minimum fraction of params that get a non-string specific type.
    "typed_fraction": (">=", 0.60),
    # Minimum fraction of output-like params that get a destination kind.
    "output_typed_fraction": (">=", 0.90),
    # Maximum fraction classified as plain string (decreases as coverage improves).
    "string_fraction": ("<=", 0.40),
    # Minimum enum params across entire catalog.
    "enum_count": (">=", 5),
    # No tool should have zero params after discovery hydration.
    "zero_param_tools": ("<=", 0),
}

OUTPUT_KINDS = {"file_out", "raster_out", "vector_out", "lidar_out"}
ALL_NON_STRING = {"raster_in", "raster_out", "vector_in", "vector_out",
                  "file_in", "file_out", "lidar_out", "bool", "int",
                  "double", "enum"}


def run() -> int:
    catalog = discover_tool_catalog(include_pro=True, tier="open")

    kind_counts: collections.Counter = collections.Counter()
    enum_count = 0
    total_params = 0
    output_typed = 0
    output_total = 0
    zero_param_tools: list[str] = []

    for item in catalog:
        params = item.get("params") or []
        if not params:
            zero_param_tools.append(str(item.get("id", "")))
        for p in params:
            n = str(p.get("name", ""))
            d = str(p.get("description", ""))
            dv = p.get("default")
            k = _infer_kind(n, d)
            opts = _extract_enum_options(n, d, dv)
            if k == "string" and len(opts) >= 2:
                k = "enum"
            kind_counts[k] += 1
            total_params += 1
            if k == "enum":
                enum_count += 1
            from whitebox_workflows_qgis.algorithm import _looks_like_output
            if _looks_like_output(n, d):
                output_total += 1
                if k in OUTPUT_KINDS:
                    output_typed += 1

    typed = sum(kind_counts[k] for k in ALL_NON_STRING)
    typed_fraction = typed / total_params if total_params else 0.0
    string_fraction = kind_counts["string"] / total_params if total_params else 1.0
    output_typed_fraction = output_typed / output_total if output_total else 1.0

    metrics = {
        "typed_fraction": typed_fraction,
        "string_fraction": string_fraction,
        "output_typed_fraction": output_typed_fraction,
        "enum_count": float(enum_count),
        "zero_param_tools": float(len(zero_param_tools)),
    }

    print("=== Algorithm Coverage Report ===")
    print(f"Catalog tools : {len(catalog)}")
    print(f"Total params  : {total_params}")
    print()
    print("Kind distribution:")
    for k, v in sorted(kind_counts.items(), key=lambda x: -x[1]):
        print(f"  {k:22s}: {v:5d}  ({100*v/total_params:.1f}%)")
    print()
    print("Metrics vs thresholds:")

    failures: list[str] = []
    for name, (op, threshold) in THRESHOLDS.items():
        value = metrics[name]
        if op == ">=":
            passed = value >= threshold
        else:
            passed = value <= threshold
        icon = "OK" if passed else "FAIL"
        print(f"  [{icon}] {name:28s}: {value:.4f}  {op} {threshold}")
        if not passed:
            failures.append(f"  FAIL: {name} = {value:.4f}, expected {op} {threshold}")

    if zero_param_tools:
        print(f"\nTools with zero params ({len(zero_param_tools)}):")
        for tid in zero_param_tools[:20]:
            print(f"  {tid}")

    if failures:
        print("\nThreshold violations:")
        for f in failures:
            print(f)
        return 1

    print("\nAll thresholds passed.")
    return 0


if __name__ == "__main__":
    sys.exit(run())
