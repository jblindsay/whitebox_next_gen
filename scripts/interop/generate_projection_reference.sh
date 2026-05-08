#!/usr/bin/env bash
set -euo pipefail

# Generate projection reference outputs using PROJ (cs2cs).
# Input:
#   artifacts/interop/projection/inputs/phase_a_points.csv
# Outputs:
#   artifacts/interop/projection/reference/phase_a_reference.csv
#   artifacts/interop/projection/reference/phase_a_reference.json
#   artifacts/interop/projection/reference/proj_toolchain.txt

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
INPUT_CSV="$ROOT_DIR/artifacts/interop/projection/inputs/phase_a_points.csv"
OUT_DIR="$ROOT_DIR/artifacts/interop/projection/reference"
OUT_CSV="$OUT_DIR/phase_a_reference.csv"
OUT_JSON="$OUT_DIR/phase_a_reference.json"
TOOLCHAIN_TXT="$OUT_DIR/proj_toolchain.txt"

if [[ ! -f "$INPUT_CSV" ]]; then
  echo "Missing input CSV: $INPUT_CSV" >&2
  exit 1
fi

if ! command -v cs2cs >/dev/null 2>&1; then
  echo "Missing required command: cs2cs" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"

{
  echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "cs2cs: $(command -v cs2cs)"
  echo "proj: $(command -v proj || true)"
  echo "projinfo: $(command -v projinfo || true)"
  echo "proj version: $(proj 2>&1 | head -n 1 || true)"
} > "$TOOLCHAIN_TXT"

echo "case_id,crs,lon,lat,x_ref,y_ref,inv_lon_ref,inv_lat_ref,category,notes" > "$OUT_CSV"

tail -n +2 "$INPUT_CSV" | while IFS=, read -r case_id crs lon lat category notes; do
  forward_raw=$(printf "%s %s\n" "$lat" "$lon" | cs2cs EPSG:4326 "$crs" -f "%.12f")
  x_ref=$(printf "%s" "$forward_raw" | awk '{print $1}')
  y_ref=$(printf "%s" "$forward_raw" | awk '{print $2}')

  inverse_raw=$(printf "%s %s\n" "$x_ref" "$y_ref" | cs2cs "$crs" EPSG:4326 -f "%.12f")
  inv_lat_ref=$(printf "%s" "$inverse_raw" | awk '{print $1}')
  inv_lon_ref=$(printf "%s" "$inverse_raw" | awk '{print $2}')

  echo "$case_id,$crs,$lon,$lat,$x_ref,$y_ref,$inv_lon_ref,$inv_lat_ref,$category,$notes" >> "$OUT_CSV"
done

python3 - "$OUT_CSV" "$OUT_JSON" <<'PY'
import csv
import json
import sys
from pathlib import Path

in_csv = Path(sys.argv[1])
out_json = Path(sys.argv[2])
rows = []
with in_csv.open("r", encoding="utf-8", newline="") as f:
    for row in csv.DictReader(f):
        row["lon"] = float(row["lon"])
        row["lat"] = float(row["lat"])
        row["x_ref"] = float(row["x_ref"])
        row["y_ref"] = float(row["y_ref"])
        row["inv_lon_ref"] = float(row["inv_lon_ref"])
        row["inv_lat_ref"] = float(row["inv_lat_ref"])
        rows.append(row)

payload = {
    "dataset": "phase_a_reference",
    "row_count": len(rows),
    "rows": rows,
}
out_json.write_text(json.dumps(payload, indent=2), encoding="utf-8")
PY

echo "Generated references: $OUT_CSV"
echo "Generated references JSON: $OUT_JSON"
echo "Captured toolchain metadata: $TOOLCHAIN_TXT"
