#!/usr/bin/env bash
set -euo pipefail

# Runs the JPEG2000 native-vs-bridge differential corpus across several
# decoder mode profiles and prints compact KPI summaries for fast comparison.

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

FIXTURES_DEFAULT="tests/fixtures/rgb_8x8_lossless.jp2,tests/fixtures/sentinel_style_16x16_4band_lossless.jp2,tests/fixtures/tiled_rgb_64x64_block32_lossless.jp2"
FIXTURES="${JPEG2000_DIFF_FIXTURES:-$FIXTURES_DEFAULT}"
TEST_NAME="jpeg2000_native_vs_bridge_differential_corpus"

run_profile() {
  local name="$1"
  shift
  local env_args=("$@")

  echo ""
  echo "=== profile: ${name} ==="
  local log_file
  log_file="$(mktemp)"

  if [[ ${#env_args[@]} -gt 0 ]]; then
    if env JPEG2000_DIFF_FIXTURES="$FIXTURES" "${env_args[@]}" cargo test -p wbraster "$TEST_NAME" -- --nocapture >"$log_file" 2>&1; then
      :
    else
      echo "status: FAILED"
      tail -n 30 "$log_file"
      rm -f "$log_file"
      return 1
    fi
  elif env JPEG2000_DIFF_FIXTURES="$FIXTURES" cargo test -p wbraster "$TEST_NAME" -- --nocapture >"$log_file" 2>&1; then
    :
  else
    echo "status: FAILED"
    tail -n 30 "$log_file"
    rm -f "$log_file"
    return 1
  fi

  local summary
  summary="$(grep "JPEG2000 differential summary:" "$log_file" | tail -n 1 || true)"
  if [[ -z "$summary" ]]; then
    echo "status: FAILED (no summary line found)"
    tail -n 30 "$log_file"
    rm -f "$log_file"
    return 1
  fi

  echo "$summary"
  grep "^SAMPLE_VALUE_MISMATCH|" "$log_file" | head -n 3 || true
  rm -f "$log_file"
}

echo "Running JPEG2000 parity matrix"
echo "fixtures=${FIXTURES}"

run_profile "baseline_standard"
run_profile "standard_runmode_on" JPEG2000_STDJK_ENABLE_RUNMODE=1
run_profile "legacy_all_subbands" JPEG2000_DIFF_FORCE_LEGACY_T1=1
run_profile "legacy_ll_only" JPEG2000_DIFF_FORCE_LEGACY_T1_LL=1
run_profile "legacy_hf_only" JPEG2000_DIFF_FORCE_LEGACY_T1_HF=1

echo ""
echo "Matrix run complete."