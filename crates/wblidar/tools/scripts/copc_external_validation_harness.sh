#!/usr/bin/env bash
set -euo pipefail

FIXTURE_DIR="${1:-/tmp/wblidar_copc_validation}"
REPORT_PATH="${2:-${FIXTURE_DIR}/external_validation_report.md}"
# Optional: space-separated extra absolute paths to include in the report.
# e.g. EXTRA_FILES="/path/a.copc.laz /path/b.copc.laz"
EXTRA_FILES="${3:-}"

FIXTURE_NAMES=(
  "fixture_pdrf6.copc.laz"
  "fixture_pdrf7.copc.laz"
  "fixture_pdrf8.copc.laz"
)

has_cmd() {
  command -v "$1" >/dev/null 2>&1
}

resolve_lastools_bin() {
  if has_cmd "$1"; then
    command -v "$1"
    return
  fi
  local fallback="/Users/johnlindsay/Documents/programming/LAStools/bin/$1"
  if [[ -x "$fallback" ]]; then
    echo "$fallback"
    return
  fi
  echo ""
}

run_lastools_check() {
  local file="$1"
  local out_las
  out_las="/tmp/$(basename "${file}").decoded.las"

  local laszip_bin
  laszip_bin="$(resolve_lastools_bin laszip)"
  local lasinfo_bin
  lasinfo_bin="$(resolve_lastools_bin lasinfo)"

  if [[ -z "$laszip_bin" ]]; then
    echo "skipped|laszip not found"
    return
  fi

  if ! "$laszip_bin" -i "$file" -o "$out_las" >/tmp/wblidar_lastools_decode.log 2>&1; then
    local msg
    if grep -q "RGBNIR14 has size != 8" /tmp/wblidar_lastools_decode.log; then
      echo "skipped|LAStools/LASzip version lacks RGBNIR14 support"
      return
    fi
    msg=$(grep -m1 "ERROR:" /tmp/wblidar_lastools_decode.log | sed 's/ERROR:[ ]*//' | tr '|' '/')
    if [[ -z "$msg" ]]; then
      msg=$(tail -n 1 /tmp/wblidar_lastools_decode.log | tr '|' '/')
    fi
    echo "fail|${msg}"
    return
  fi

  if [[ -n "$lasinfo_bin" ]]; then
    local count
    # lasinfo writes its report to stderr; redirect 2>&1 to capture it.
    count=$($lasinfo_bin -i "$out_las" 2>&1 | awk -F: '/extended number of point records/ {gsub(/ /, "", $2); print $2; exit}')
    if [[ -z "$count" ]]; then
      count=$($lasinfo_bin -i "$out_las" 2>&1 | awk -F: '/number of point records[^s]/ {gsub(/ /, "", $2); print $2; exit}')
    fi
    echo "pass|decoded with LAStools (points=${count:-unknown})"
  else
    echo "pass|decoded with LAStools (lasinfo unavailable)"
  fi
}

run_pdal_check() {
  local file="$1"

  if ! has_cmd pdal; then
    echo "skipped|pdal not found"
    return
  fi

  if ! pdal info "$file" >/tmp/wblidar_pdal_info.json 2>/tmp/wblidar_pdal_info.err; then
    local msg
    msg=$(tail -n 1 /tmp/wblidar_pdal_info.err | tr '|' '/')
    echo "fail|${msg}"
    return
  fi

  local points
  points=$(grep -E '"count"|"num_points"' /tmp/wblidar_pdal_info.json | head -n 1 | sed 's/[^0-9]//g')
  echo "pass|pdal info succeeded (points=${points:-unknown})"
}

mkdir -p "$(dirname "$REPORT_PATH")"

{
  echo "# COPC External Validation Report"
  echo
  echo "Generated: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  echo "Fixture directory: ${FIXTURE_DIR}"
  echo
  echo "| File | Exists | LAStools | PDAL | validate.copc.io | Notes |"
  echo "|---|---|---|---|---|---|"

  check_file() {
    local file="$1"
    local label="$2"
    if [[ ! -f "$file" ]]; then
      echo "| ${label} | no | n/a | n/a | pending | missing fixture |"
      return
    fi
    lastools_result=$(run_lastools_check "$file")
    pdal_result=$(run_pdal_check "$file")
    lastools_status="${lastools_result%%|*}"
    lastools_note="${lastools_result#*|}"
    pdal_status="${pdal_result%%|*}"
    pdal_note="${pdal_result#*|}"
    notes="LAStools: ${lastools_note}; PDAL: ${pdal_note}"
    notes=${notes//|//}
    echo "| ${label} | yes | ${lastools_status} | ${pdal_status} | pending | ${notes} |"
  }

  for name in "${FIXTURE_NAMES[@]}"; do
    check_file "${FIXTURE_DIR}/${name}" "${name}"
  done

  if [[ -n "$EXTRA_FILES" ]]; then
    for extra in $EXTRA_FILES; do
      check_file "$extra" "$(basename "$extra")"
    done
  fi
} > "$REPORT_PATH"

echo "Wrote report: ${REPORT_PATH}"
