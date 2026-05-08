#!/usr/bin/env bash
set -euo pipefail

ROOT="/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen"
FIXTURE="$ROOT/artifacts/interop/fixtures/vector/V04/source_input.geojson"
OUT_DIR="$ROOT/artifacts/interop/results/vector/V04_repro"
OUT_FGB="$OUT_DIR/source_gdal.fgb"

mkdir -p "$OUT_DIR"
rm -f "$OUT_FGB"

echo "[1/3] Creating FlatGeobuf from fixture via GDAL"
ogr2ogr -f FlatGeobuf "$OUT_FGB" "$FIXTURE"

echo "[2/3] First 8 bytes (expected to reproduce magic variant)"
xxd -g 1 -l 8 "$OUT_FGB"

echo "[3/3] Reading with wbw_python"
python3 - << 'PY'
import whitebox_workflows as wbw
from pathlib import Path
p = Path('/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/vector/V04_repro/source_gdal.fgb')
env = wbw.WbEnvironment()
layer = env.read_vector(str(p))
print('feature_count=', layer.feature_count())
PY

echo "[extra] Producer feature count via ogrinfo"
ogrinfo -ro -so -al "$OUT_FGB" | grep -i "Feature Count" || true

echo "Reproducer complete: $OUT_FGB"
