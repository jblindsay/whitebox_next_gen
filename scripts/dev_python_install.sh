#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE_DIR="$ROOT_DIR/crates/whitebox_python"

if ! command -v maturin >/dev/null 2>&1; then
  echo "maturin is not installed. Install it with: python3 -m pip install maturin" >&2
  exit 1
fi

cd "$CRATE_DIR"
maturin develop --release

echo "whitebox_workflows installed in current Python environment"
