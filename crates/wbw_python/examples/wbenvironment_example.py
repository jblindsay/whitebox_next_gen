#!/usr/bin/env python3
"""
Example of using the WbEnvironment API for Whitebox tools.

This is a much simpler, more Pythonic API that matches the original
whitebox_workflows style.
"""

import argparse
from pathlib import Path

import whitebox_workflows


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Run Whitebox unary raster tools using WbEnvironment."
    )
    parser.add_argument("input", type=Path, help="Input raster path")
    parser.add_argument("output_dir", type=Path, help="Output directory")
    parser.add_argument(
        "--tools",
        default="abs,sqrt",
        help="Comma-separated tool ids (abs, ceil, floor, round, sqrt, square, ln, log10, sin, cos)",
    )
    parser.add_argument(
        "--auto-output",
        action="store_true",
        help="Let WbEnvironment auto-generate output file names",
    )
    parser.add_argument("--verbose", action="store_true", help="Enable verbose output")
    args = parser.parse_args()

    if not args.input.exists():
        raise FileNotFoundError(f"Input raster does not exist: {args.input}")

    # Create working environment
    wbe = whitebox_workflows.WbEnvironment(include_pro=False, tier="open")
    wbe.working_directory = str(args.input.parent)
    wbe.verbose = args.verbose

    args.output_dir.mkdir(parents=True, exist_ok=True)

    # Parse tool list
    tool_ids = [t.strip() for t in args.tools.split(",") if t.strip()]

    # Read input raster
    print(f"Reading input raster: {args.input}")
    dem = wbe.read_raster(str(args.input.name))

    # Run tools
    for tool_id in tool_ids:
        output_name = f"{args.input.stem}_{tool_id}{args.input.suffix}"
        output_path = str((args.output_dir / output_name).resolve())

        print(f"\nRunning {tool_id}...")

        # Call tool as a method on the environment
        # Much simpler than the old JSON-based API.
        if args.auto_output:
            result = getattr(wbe, tool_id)(dem)
        else:
            result = getattr(wbe, tool_id)(dem, output_path)

        # Raster instance methods are also supported, e.g. dem.sqrt().

        print(f"  Output: {result.file_path}")


if __name__ == "__main__":
    main()
