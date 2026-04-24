#!/usr/bin/env python3
from __future__ import annotations

import argparse
import statistics
import time
from pathlib import Path

import whitebox_workflows as wb


def main() -> int:
    parser = argparse.ArgumentParser(description="Benchmark gaussian_curvature in Whitebox Next Gen")
    parser.add_argument("--input", required=True, help="Absolute path to input DEM raster")
    parser.add_argument("--runs", type=int, default=1, help="Number of timed runs (default: 1)")
    parser.add_argument("--z-factor", type=float, default=1.0, help="Elevation conversion factor")
    parser.add_argument(
        "--log-transform",
        action="store_true",
        help="Enable log-transformed output",
    )
    parser.add_argument(
        "--working-directory",
        default=None,
        help="Optional working directory; defaults to DEM parent folder",
    )
    args = parser.parse_args()

    input_path = Path(args.input).expanduser().resolve()
    if not input_path.exists():
        raise SystemExit(f"Input raster not found: {input_path}")
    if args.runs < 1:
        raise SystemExit("--runs must be >= 1")

    workdir = Path(args.working_directory).expanduser().resolve() if args.working_directory else input_path.parent

    wbe = wb.WbEnvironment(include_pro=False, tier="open")
    wbe.working_directory = str(workdir)
    wbe.verbose = False

    dem = wbe.read_raster(str(input_path))

    elapsed_runs: list[float] = []
    for index in range(args.runs):
        start = time.perf_counter()
        _ = wbe.terrain.derivatives.gaussian_curvature(
            input=dem,
            z_factor=args.z_factor,
            log_transform=args.log_transform,
        )
        elapsed = time.perf_counter() - start
        elapsed_runs.append(elapsed)
        print(f"run_{index + 1}_s={elapsed:.6f}")

    print(f"median_s={statistics.median(elapsed_runs):.6f}")
    print(f"mean_s={statistics.fmean(elapsed_runs):.6f}")
    print("timing_scope=kernel_only_excludes_read_write")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
