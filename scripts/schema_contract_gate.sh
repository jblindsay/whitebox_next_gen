#!/usr/bin/env bash
set -euo pipefail

# CI-light schema contract gate for OSS + Pro without GitHub Actions.
# Run from repository root: ./scripts/schema_contract_gate.sh

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PRO_DIR="$ROOT_DIR/../wbtools_pro"

echo "[schema-gate] running OSS strict runtime metadata checks"
"$ROOT_DIR/.venv-wbw/bin/python" "$ROOT_DIR/scripts/schema_coverage_report.py" --strict --sample-limit 10

echo "[schema-gate] running Pro explicit resolver coverage checks"
python3 "$PRO_DIR/scripts/pro_schema_coverage_report.py" --strict --sample-limit 10

echo "[schema-gate] all schema contract checks passed"
