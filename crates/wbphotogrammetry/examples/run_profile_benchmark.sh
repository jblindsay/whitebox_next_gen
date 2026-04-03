#!/usr/bin/env bash
set -euo pipefail

# Reproducible wbphotogrammetry profiling sweep.
#
# Usage:
#   ./crates/wbphotogrammetry/examples/run_profile_benchmark.sh
#   ./crates/wbphotogrammetry/examples/run_profile_benchmark.sh --out-dir /tmp/wbprofiles --repeats 5 --profile balanced
#   ./crates/wbphotogrammetry/examples/run_profile_benchmark.sh --reduced-solver-mode dense-lu

OUT_DIR="${PWD}/target/wbphotogrammetry_profiles"
REPEATS=3
PROFILE="balanced"
RESOLUTION="0.12"
REDUCED_SOLVER_MODE="sparse-pcg"
FRAMES=(30 60 120)

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out-dir)
      OUT_DIR="$2"
      shift 2
      ;;
    --repeats)
      REPEATS="$2"
      shift 2
      ;;
    --profile)
      PROFILE="$2"
      shift 2
      ;;
    --resolution)
      RESOLUTION="$2"
      shift 2
      ;;
    --reduced-solver-mode)
      REDUCED_SOLVER_MODE="$2"
      shift 2
      ;;
    --frames)
      IFS=',' read -r -a FRAMES <<< "$2"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

mkdir -p "$OUT_DIR"

echo "Running wbphotogrammetry profile sweep"
echo "  out_dir:    $OUT_DIR"
echo "  repeats:    $REPEATS"
echo "  profile:    $PROFILE"
echo "  resolution: $RESOLUTION"
echo "  reduced_solver_mode: $REDUCED_SOLVER_MODE"
echo "  frames:     ${FRAMES[*]}"

for n in "${FRAMES[@]}"; do
  OUT_FILE="$OUT_DIR/profile_${PROFILE}_${n}f_${REPEATS}r.json"
  echo
echo "[benchmark] frames=$n -> $OUT_FILE"
  cargo run -p wbphotogrammetry --example profile_pipeline -- \
    --frames "$n" \
    --repeats "$REPEATS" \
    --profile "$PROFILE" \
    --resolution "$RESOLUTION" \
    --reduced-solver-mode "$REDUCED_SOLVER_MODE" \
    --json-out "$OUT_FILE"
done

echo
echo "Benchmark sweep complete. JSON reports in: $OUT_DIR"
