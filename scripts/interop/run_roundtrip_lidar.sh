#!/usr/bin/env bash
set -euo pipefail

# Phase B lidar interop harness (v1 starter).
# This script currently validates environment and records tool versions.
# Case execution commands should be added incrementally (L01-L03).

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/artifacts/interop/results/lidar"
REPORT="$OUT_DIR/environment_lidar.txt"

mkdir -p "$OUT_DIR"

required=(pdal)
for cmd in "${required[@]}"; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "Missing required command: $cmd" >&2
    exit 1
  fi
done

{
  echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "pdal: $(command -v pdal)"
  echo "pdal version: $(pdal --version 2>/dev/null || true)"
} > "$REPORT"

echo "Lidar interop environment report written: $REPORT"
echo "TODO: implement case pipelines L01-L03."
