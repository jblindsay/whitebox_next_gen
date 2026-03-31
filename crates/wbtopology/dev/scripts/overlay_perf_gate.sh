#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="overlay"
BASELINE_CSV="${ROOT_DIR}/data/perf/overlay_bench_baseline.csv"
BASELINE_SET="false"
CURRENT_CSV=""
WRITE_CURRENT=""
REPEATS="7"
DEFAULT_THRESHOLD="5.0"
OVERLAY_DEFAULT_THRESHOLD=""
VORONOI_DEFAULT_THRESHOLD=""
TRIANGULATION_DEFAULT_THRESHOLD=""
BOOTSTRAP_BASELINE="false"
SKIP_BUILD="false"
FEATURES=""
COMPARE_SERIAL_PARALLEL="false"

DIFF_ARGS=()

usage() {
  cat <<EOF
Usage:
  scripts/overlay_perf_gate.sh [options]

Options:
  --target <overlay|voronoi|triangulation|all>
                                   Benchmark/gate target (default: overlay)
  --baseline <path>                 Baseline CSV path (default: data/perf/overlay_bench_baseline.csv)
  --current <path>                  Existing current CSV path. If omitted, benchmark is run to generate one.
  --write-current <path>            Persist generated current CSV to this path.
  --repeats <n>                     Repeat count passed to overlay_bench --repeats (default: 7)
  --features <list>                 Cargo feature list used for benchmark/build runs (e.g. parallel)
  --default-threshold <pct>         Default max regression percent (default: 5.0)
  --overlay-default-threshold <pct> Default max regression percent for overlay when --target all.
  --voronoi-default-threshold <pct> Default max regression percent for voronoi when --target all.
  --triangulation-default-threshold <pct>
                                   Default max regression percent for triangulation when --target all.
  --op-threshold <op=pct>           Operation-specific threshold (repeatable)
  --case-threshold <case=pct>       Case-specific threshold (repeatable)
  --case-op-threshold <case:op=pct> Case+operation threshold (repeatable)
  --compare-serial-parallel         Generate serial and parallel snapshots and gate parallel against serial.
  --bootstrap-baseline              Write generated current CSV to baseline path and exit success.
  --skip-build                      Skip cargo check --examples.
  -h, --help                        Show this help.

Notes:
  - Wildcards with '*' are supported by diff selectors.
  - Quote wildcard selectors in zsh, e.g. --case-threshold 'nc_*=9'.
EOF
}

require_value() {
  local flag="$1"
  local value="${2:-}"
  if [[ -z "$value" ]]; then
    echo "Missing value after ${flag}" >&2
    exit 2
  fi
}

ORIGINAL_ARGS=("$@")

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      require_value "$1" "${2:-}"
      TARGET="$2"
      shift 2
      ;;
    --baseline)
      require_value "$1" "${2:-}"
      BASELINE_CSV="$2"
      BASELINE_SET="true"
      shift 2
      ;;
    --current)
      require_value "$1" "${2:-}"
      CURRENT_CSV="$2"
      shift 2
      ;;
    --write-current)
      require_value "$1" "${2:-}"
      WRITE_CURRENT="$2"
      shift 2
      ;;
    --repeats)
      require_value "$1" "${2:-}"
      REPEATS="$2"
      shift 2
      ;;
    --default-threshold)
      require_value "$1" "${2:-}"
      DEFAULT_THRESHOLD="$2"
      shift 2
      ;;
    --overlay-default-threshold)
      require_value "$1" "${2:-}"
      OVERLAY_DEFAULT_THRESHOLD="$2"
      shift 2
      ;;
    --voronoi-default-threshold)
      require_value "$1" "${2:-}"
      VORONOI_DEFAULT_THRESHOLD="$2"
      shift 2
      ;;
    --triangulation-default-threshold)
      require_value "$1" "${2:-}"
      TRIANGULATION_DEFAULT_THRESHOLD="$2"
      shift 2
      ;;
    --features)
      require_value "$1" "${2:-}"
      FEATURES="$2"
      shift 2
      ;;
    --compare-serial-parallel)
      COMPARE_SERIAL_PARALLEL="true"
      shift
      ;;
    --op-threshold|--case-threshold|--case-op-threshold)
      require_value "$1" "${2:-}"
      DIFF_ARGS+=("$1" "$2")
      shift 2
      ;;
    --bootstrap-baseline)
      BOOTSTRAP_BASELINE="true"
      shift
      ;;
    --skip-build)
      SKIP_BUILD="true"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ "${TARGET}" != "overlay" && "${TARGET}" != "voronoi" && "${TARGET}" != "triangulation" && "${TARGET}" != "all" ]]; then
  echo "Invalid --target value: ${TARGET} (expected overlay, voronoi, triangulation, or all)" >&2
  exit 2
fi

if [[ "${TARGET}" == "all" ]]; then
  SCRIPT_PATH="${BASH_SOURCE[0]}"
  pass_args=()
  skip_next=""
  for arg in "${ORIGINAL_ARGS[@]}"; do
    if [[ -n "${skip_next}" ]]; then
      skip_next=""
      continue
    fi
    if [[ "${arg}" == "--target" ]]; then
      skip_next="--target"
      continue
    fi
    if [[ "${arg}" == "--overlay-default-threshold" ]]; then
      skip_next="--overlay-default-threshold"
      continue
    fi
    if [[ "${arg}" == "--voronoi-default-threshold" ]]; then
      skip_next="--voronoi-default-threshold"
      continue
    fi
    if [[ "${arg}" == "--triangulation-default-threshold" ]]; then
      skip_next="--triangulation-default-threshold"
      continue
    fi
    if [[ "${arg}" == "--default-threshold" ]]; then
      skip_next="--default-threshold"
      continue
    fi
    pass_args+=("${arg}")
  done

  echo "Running combined performance gate for overlay, voronoi, and triangulation targets..."

  overlay_threshold="${DEFAULT_THRESHOLD}"
  voronoi_threshold="${DEFAULT_THRESHOLD}"
  triangulation_threshold="${DEFAULT_THRESHOLD}"
  if [[ -n "${OVERLAY_DEFAULT_THRESHOLD}" ]]; then
    overlay_threshold="${OVERLAY_DEFAULT_THRESHOLD}"
  fi
  if [[ -n "${VORONOI_DEFAULT_THRESHOLD}" ]]; then
    voronoi_threshold="${VORONOI_DEFAULT_THRESHOLD}"
  fi
  if [[ -n "${TRIANGULATION_DEFAULT_THRESHOLD}" ]]; then
    triangulation_threshold="${TRIANGULATION_DEFAULT_THRESHOLD}"
  fi

  set +e
  "${SCRIPT_PATH}" --target overlay --default-threshold "${overlay_threshold}" "${pass_args[@]}"
  overlay_status=$?
  "${SCRIPT_PATH}" --target voronoi --default-threshold "${voronoi_threshold}" "${pass_args[@]}"
  voronoi_status=$?
  "${SCRIPT_PATH}" --target triangulation --default-threshold "${triangulation_threshold}" "${pass_args[@]}"
  triangulation_status=$?
  set -e

  if [[ ${overlay_status} -ne 0 || ${voronoi_status} -ne 0 || ${triangulation_status} -ne 0 ]]; then
    echo "Combined performance gate failed (overlay=${overlay_status}, voronoi=${voronoi_status}, triangulation=${triangulation_status})." >&2
    exit 1
  fi

  echo "Combined performance gate passed."
  exit 0
fi

if [[ "${BASELINE_SET}" != "true" ]]; then
  if [[ "${TARGET}" == "overlay" ]]; then
    BASELINE_CSV="${ROOT_DIR}/data/perf/overlay_bench_baseline.csv"
  elif [[ "${TARGET}" == "voronoi" ]]; then
    BASELINE_CSV="${ROOT_DIR}/data/perf/voronoi_bench_baseline.csv"
  else
    BASELINE_CSV="${ROOT_DIR}/data/perf/triangulation_bench_baseline.csv"
  fi
fi

BENCH_EXAMPLE="overlay_bench"
DIFF_EXAMPLE="overlay_bench_diff"
if [[ "${TARGET}" == "voronoi" ]]; then
  BENCH_EXAMPLE="voronoi_bench"
  DIFF_EXAMPLE="voronoi_bench_diff"
elif [[ "${TARGET}" == "triangulation" ]]; then
  BENCH_EXAMPLE="triangulation_bench"
  DIFF_EXAMPLE="triangulation_bench_diff"
fi

if [[ -z "${CURRENT_CSV}" ]]; then
  CURRENT_CSV="$(mktemp /tmp/wbtopology_bench_current.XXXXXX)"
  CURRENT_IS_TEMP="true"
else
  CURRENT_IS_TEMP="false"
fi

cleanup() {
  if [[ "${CURRENT_IS_TEMP}" == "true" && -f "${CURRENT_CSV}" ]]; then
    rm -f "${CURRENT_CSV}"
  fi
}
trap cleanup EXIT

cd "${ROOT_DIR}"

if [[ "${SKIP_BUILD}" != "true" ]]; then
  if [[ -n "${FEATURES}" ]]; then
    cargo check --examples -q --features "${FEATURES}"
  else
    cargo check --examples -q
  fi
fi

if [[ "${COMPARE_SERIAL_PARALLEL}" == "true" ]]; then
  SERIAL_CSV="$(mktemp /tmp/wbtopology_bench_serial.XXXXXX)"
  PARALLEL_CSV="$(mktemp /tmp/wbtopology_bench_parallel.XXXXXX)"
  trap 'cleanup; rm -f "${SERIAL_CSV}" "${PARALLEL_CSV}"' EXIT

  echo "Generating serial benchmark snapshot..."
  cargo run --release --example "${BENCH_EXAMPLE}" -- --csv --repeats "${REPEATS}" > "${SERIAL_CSV}"

  echo "Generating parallel benchmark snapshot..."
  cargo run --release --features parallel --example "${BENCH_EXAMPLE}" -- --csv --repeats "${REPEATS}" > "${PARALLEL_CSV}"

  if [[ -n "${WRITE_CURRENT}" ]]; then
    mkdir -p "$(dirname "${WRITE_CURRENT}")"
    cp "${PARALLEL_CSV}" "${WRITE_CURRENT}"
    echo "Wrote parallel benchmark snapshot: ${WRITE_CURRENT}"
  fi

  set +e
  if (( ${#DIFF_ARGS[@]} > 0 )); then
    cargo run --release --example "${DIFF_EXAMPLE}" -- \
      "${SERIAL_CSV}" \
      "${PARALLEL_CSV}" \
      "${DEFAULT_THRESHOLD}" \
      "${DIFF_ARGS[@]}"
  else
    cargo run --release --example "${DIFF_EXAMPLE}" -- \
      "${SERIAL_CSV}" \
      "${PARALLEL_CSV}" \
      "${DEFAULT_THRESHOLD}"
  fi
  DIFF_STATUS=$?
  set -e

  if [[ ${DIFF_STATUS} -ne 0 ]]; then
    echo "Serial-vs-parallel performance gate failed (exit ${DIFF_STATUS})." >&2
    exit ${DIFF_STATUS}
  fi

  echo "Serial-vs-parallel performance gate passed."
  exit 0
fi

if [[ ! -f "${CURRENT_CSV}" || "${CURRENT_IS_TEMP}" == "true" ]]; then
  if [[ -n "${FEATURES}" ]]; then
    cargo run --release --features "${FEATURES}" --example "${BENCH_EXAMPLE}" -- --csv --repeats "${REPEATS}" > "${CURRENT_CSV}"
  else
    cargo run --release --example "${BENCH_EXAMPLE}" -- --csv --repeats "${REPEATS}" > "${CURRENT_CSV}"
  fi
fi

if [[ -n "${WRITE_CURRENT}" ]]; then
  mkdir -p "$(dirname "${WRITE_CURRENT}")"
  cp "${CURRENT_CSV}" "${WRITE_CURRENT}"
  echo "Wrote current benchmark snapshot: ${WRITE_CURRENT}"
fi

if [[ "${BOOTSTRAP_BASELINE}" == "true" ]]; then
  mkdir -p "$(dirname "${BASELINE_CSV}")"
  cp "${CURRENT_CSV}" "${BASELINE_CSV}"
  echo "Bootstrapped baseline snapshot: ${BASELINE_CSV}"
  exit 0
fi

if [[ ! -f "${BASELINE_CSV}" ]]; then
  echo "Baseline file not found: ${BASELINE_CSV}" >&2
  echo "Run with --bootstrap-baseline to create it first." >&2
  exit 2
fi

set +e
if (( ${#DIFF_ARGS[@]} > 0 )); then
  cargo run --release --example "${DIFF_EXAMPLE}" -- \
    "${BASELINE_CSV}" \
    "${CURRENT_CSV}" \
    "${DEFAULT_THRESHOLD}" \
    "${DIFF_ARGS[@]}"
else
  cargo run --release --example "${DIFF_EXAMPLE}" -- \
    "${BASELINE_CSV}" \
    "${CURRENT_CSV}" \
    "${DEFAULT_THRESHOLD}"
fi
DIFF_STATUS=$?
set -e

if [[ ${DIFF_STATUS} -ne 0 ]]; then
  echo "Performance gate failed (exit ${DIFF_STATUS})." >&2
  exit ${DIFF_STATUS}
fi

echo "Performance gate passed."
