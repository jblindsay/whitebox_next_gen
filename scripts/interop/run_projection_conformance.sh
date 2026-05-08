#!/usr/bin/env bash
set -euo pipefail

# Run projection conformance checks against reference outputs.
# Inputs:
#   artifacts/interop/projection/reference/phase_a_reference.csv
# Outputs:
#   artifacts/interop/results/projection/phase_a_results.csv
#   artifacts/interop/results/projection/summary_phase_a.json

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REF_DIR="$ROOT_DIR/artifacts/interop/projection/reference"
OUT_DIR="$ROOT_DIR/artifacts/interop/results/projection"
REF_CSV="$REF_DIR/phase_a_reference.csv"
RESULTS_CSV="$OUT_DIR/phase_a_results.csv"
SUMMARY_JSON="$OUT_DIR/summary_phase_a.json"

mkdir -p "$OUT_DIR"

if [[ ! -f "$REF_CSV" ]]; then
  echo "Missing reference CSV: $REF_CSV" >&2
  echo "Run scripts/interop/generate_projection_reference.sh first." >&2
  exit 1
fi

if ! command -v cs2cs >/dev/null 2>&1; then
  echo "Missing required command: cs2cs" >&2
  exit 1
fi

python3 - "$REF_CSV" "$RESULTS_CSV" "$SUMMARY_JSON" <<'PY'
import csv
import json
import math
import subprocess
import sys
from pathlib import Path

ref_csv = Path(sys.argv[1])
results_csv = Path(sys.argv[2])
summary_json = Path(sys.argv[3])

FWD_TOL_METERS = 0.10
INV_TOL_DEGREES = 1e-7


def run_cs2cs(src: str, dst: str, x: float, y: float, decimals: int = 12):
	fmt = f"%.{decimals}f"
	payload = f"{x} {y}\n".encode("utf-8")
	cmd = ["cs2cs", src, dst, "-f", fmt]
	p = subprocess.run(cmd, input=payload, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True)
	parts = p.stdout.decode("utf-8").strip().split()
	if len(parts) < 2:
		raise RuntimeError(f"Unexpected cs2cs output: {p.stdout!r}")
	return float(parts[0]), float(parts[1])


def abs_err(a: float, b: float) -> float:
	return abs(a - b)


rows_out = []
counts = {
	"total": 0,
	"pass": 0,
	"fail": 0,
}

with ref_csv.open("r", encoding="utf-8", newline="") as f:
	for row in csv.DictReader(f):
		case_id = row["case_id"]
		crs = row["crs"]
		lon = float(row["lon"])
		lat = float(row["lat"])
		x_ref = float(row["x_ref"])
		y_ref = float(row["y_ref"])
		inv_lon_ref = float(row["inv_lon_ref"])
		inv_lat_ref = float(row["inv_lat_ref"])

		# Forward: EPSG:4326 expects latitude-longitude axis ordering for EPSG use.
		x_calc, y_calc = run_cs2cs("EPSG:4326", crs, lat, lon)

		# Inverse: projected input x/y to geographic output (lat, lon).
		inv_lat_calc, inv_lon_calc = run_cs2cs(crs, "EPSG:4326", x_calc, y_calc)

		fwd_x_err = abs_err(x_calc, x_ref)
		fwd_y_err = abs_err(y_calc, y_ref)
		inv_lon_err = abs_err(inv_lon_calc, inv_lon_ref)
		inv_lat_err = abs_err(inv_lat_calc, inv_lat_ref)

		# For geographic targets, forward units are degrees; rely on inverse tolerance.
		if crs in {"EPSG:4326", "EPSG:4269"}:
			fwd_ok = True
		else:
			fwd_ok = (fwd_x_err <= FWD_TOL_METERS) and (fwd_y_err <= FWD_TOL_METERS)

		inv_ok = (inv_lon_err <= INV_TOL_DEGREES) and (inv_lat_err <= INV_TOL_DEGREES)
		status = "PASS" if (fwd_ok and inv_ok) else "FAIL"

		counts["total"] += 1
		if status == "PASS":
			counts["pass"] += 1
		else:
			counts["fail"] += 1

		rows_out.append({
			"case_id": case_id,
			"crs": crs,
			"status": status,
			"fwd_x_err": fwd_x_err,
			"fwd_y_err": fwd_y_err,
			"inv_lon_err": inv_lon_err,
			"inv_lat_err": inv_lat_err,
			"fwd_tol_m": FWD_TOL_METERS,
			"inv_tol_deg": INV_TOL_DEGREES,
		})

with results_csv.open("w", encoding="utf-8", newline="") as f:
	writer = csv.DictWriter(
		f,
		fieldnames=[
			"case_id",
			"crs",
			"status",
			"fwd_x_err",
			"fwd_y_err",
			"inv_lon_err",
			"inv_lat_err",
			"fwd_tol_m",
			"inv_tol_deg",
		],
	)
	writer.writeheader()
	writer.writerows(rows_out)

summary = {
	"dataset": "phase_a_projection_conformance",
	"total_cases": counts["total"],
	"passed": counts["pass"],
	"failed": counts["fail"],
	"status": "PASS" if counts["fail"] == 0 else "FAIL",
	"tolerances": {
		"forward_meters": FWD_TOL_METERS,
		"inverse_degrees": INV_TOL_DEGREES,
	},
}
summary_json.write_text(json.dumps(summary, indent=2), encoding="utf-8")
PY

echo "Wrote conformance results: $RESULTS_CSV"
echo "Wrote conformance summary: $SUMMARY_JSON"
