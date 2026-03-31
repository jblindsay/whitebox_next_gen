#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

WBPROJ_SPEEDUP_MIN="${WBPROJ_SPEEDUP_MIN:-1.02}"
WBRASTER_SPEEDUP_MIN="${WBRASTER_SPEEDUP_MIN:-1.02}"

TMP_PROJ="$(mktemp)"
TMP_RASTER="$(mktemp)"
trap 'rm -f "$TMP_PROJ" "$TMP_RASTER"' EXIT

check_threshold() {
  local value="$1"
  local threshold="$2"
  awk -v v="$value" -v t="$threshold" 'BEGIN { exit !(v + 0 >= t + 0) }'
}

echo "[simd-guardrail] Running wbprojection SIMD benchmark example..."
cargo run --release -p wbprojection --example simd_batch_transform >"$TMP_PROJ"

WBPROJ_SPEEDUP="$(awk '/Speedup:/{v=$2; sub(/x$/, "", v); print v; exit}' "$TMP_PROJ")"
if [[ -z "$WBPROJ_SPEEDUP" ]]; then
  echo "[simd-guardrail] Could not parse wbprojection speedup." >&2
  tail -n 40 "$TMP_PROJ" >&2
  exit 1
fi

if grep -Eq 'Kernel sample [0-9]+ match: false' "$TMP_PROJ"; then
  echo "[simd-guardrail] wbprojection kernel correctness check failed." >&2
  tail -n 60 "$TMP_PROJ" >&2
  exit 1
fi

if ! grep -Fq 'Geocentric batch path matches scalar: true' "$TMP_PROJ"; then
  echo "[simd-guardrail] wbprojection geocentric batch correctness check failed." >&2
  tail -n 60 "$TMP_PROJ" >&2
  exit 1
fi

if ! check_threshold "$WBPROJ_SPEEDUP" "$WBPROJ_SPEEDUP_MIN"; then
  echo "[simd-guardrail] wbprojection speedup ${WBPROJ_SPEEDUP}x is below threshold ${WBPROJ_SPEEDUP_MIN}x." >&2
  tail -n 60 "$TMP_PROJ" >&2
  exit 1
fi

echo "[simd-guardrail] wbprojection speedup ${WBPROJ_SPEEDUP}x (threshold ${WBPROJ_SPEEDUP_MIN}x)"

echo "[simd-guardrail] Running wbraster SIMD benchmark example..."
cargo run --release -p wbraster --example simd_stats_compute >"$TMP_RASTER"

WBRASTER_SPEEDUPS=()
while IFS= read -r speedup; do
  WBRASTER_SPEEDUPS+=("$speedup")
done < <(awk '/Speedup:/{v=$2; sub(/x$/, "", v); print v}' "$TMP_RASTER")

if [[ "${#WBRASTER_SPEEDUPS[@]}" -lt 2 ]]; then
  echo "[simd-guardrail] Could not parse wbraster speedups." >&2
  tail -n 60 "$TMP_RASTER" >&2
  exit 1
fi

if ! grep -Fq 'Full-raster scalar/SIMD match: true' "$TMP_RASTER"; then
  echo "[simd-guardrail] wbraster full-raster correctness check failed." >&2
  tail -n 60 "$TMP_RASTER" >&2
  exit 1
fi

if ! grep -Fq 'Band scalar/SIMD match: true' "$TMP_RASTER"; then
  echo "[simd-guardrail] wbraster band correctness check failed." >&2
  tail -n 60 "$TMP_RASTER" >&2
  exit 1
fi

for idx in 0 1; do
  speedup="${WBRASTER_SPEEDUPS[$idx]}"
  if ! check_threshold "$speedup" "$WBRASTER_SPEEDUP_MIN"; then
    echo "[simd-guardrail] wbraster speedup ${speedup}x is below threshold ${WBRASTER_SPEEDUP_MIN}x." >&2
    tail -n 60 "$TMP_RASTER" >&2
    exit 1
  fi
done

echo "[simd-guardrail] wbraster speedups ${WBRASTER_SPEEDUPS[0]}x, ${WBRASTER_SPEEDUPS[1]}x (threshold ${WBRASTER_SPEEDUP_MIN}x)"
echo "[simd-guardrail] PASS"
