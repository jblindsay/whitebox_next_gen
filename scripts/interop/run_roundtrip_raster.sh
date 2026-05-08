#!/usr/bin/env bash
set -euo pipefail

# Phase B raster interop harness (v1 starter).
# This script currently validates environment and records tool versions.
# Case execution commands should be added incrementally (R01-R08).

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/artifacts/interop/results/raster"
REPORT="$OUT_DIR/environment_raster.txt"

mkdir -p "$OUT_DIR"

required=(gdal_translate gdalinfo)
for cmd in "${required[@]}"; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "Missing required command: $cmd" >&2
    exit 1
  fi
done

{
  echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "gdal_translate: $(command -v gdal_translate)"
  echo "gdalinfo: $(command -v gdalinfo)"
  echo "gdal version: $(gdalinfo --version 2>/dev/null || true)"
} > "$REPORT"

echo "Raster interop environment report written: $REPORT"
echo "TODO: implement case pipelines R01-R08."
