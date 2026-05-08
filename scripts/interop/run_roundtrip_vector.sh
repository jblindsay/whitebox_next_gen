#!/usr/bin/env bash
set -euo pipefail

# Phase B vector interop harness (v1 starter).
# This script currently validates environment and records tool versions.
# Case execution commands should be added incrementally (V01-V04).

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/artifacts/interop/results/vector"
REPORT="$OUT_DIR/environment_vector.txt"

mkdir -p "$OUT_DIR"

required=(ogr2ogr ogrinfo)
for cmd in "${required[@]}"; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "Missing required command: $cmd" >&2
    exit 1
  fi
done

{
  echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "ogr2ogr: $(command -v ogr2ogr)"
  echo "ogrinfo: $(command -v ogrinfo)"
  echo "gdal/ogr version: $(ogrinfo --version 2>/dev/null || true)"
  if command -v qgis_process >/dev/null 2>&1; then
    echo "qgis_process: $(command -v qgis_process)"
  else
    echo "qgis_process: NOT FOUND (V01 QGIS-produced artifacts currently blocked in this environment)"
  fi
} > "$REPORT"

echo "Vector interop environment report written: $REPORT"
echo "TODO: implement case pipelines V01-V04."
