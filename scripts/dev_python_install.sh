#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE_DIR="$ROOT_DIR/crates/wbw_python"
ENABLE_PRO="${WBW_PYTHON_ENABLE_PRO:-0}"

usage() {
  cat <<'EOF'
Usage: ./scripts/dev_python_install.sh [--pro|--open|--help]

Options:
  --pro     Build wbw_python with Cargo feature 'pro' enabled.
  --open    Build the default open-only wbw_python extension.
  --help    Show this help message.

Environment:
  WBW_PYTHON_ENABLE_PRO=1  Equivalent to --pro.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --pro)
      ENABLE_PRO=1
      ;;
    --open)
      ENABLE_PRO=0
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
  shift
done

if [[ ! -d "$CRATE_DIR" ]]; then
  echo "wbw_python crate directory not found: $CRATE_DIR" >&2
  exit 1
fi

if ! command -v maturin >/dev/null 2>&1; then
  echo "maturin is not installed. Install it with: python3 -m pip install maturin" >&2
  exit 1
fi

cd "$CRATE_DIR"

if [[ "$ENABLE_PRO" == "1" ]]; then
  echo "Installing whitebox_workflows with Pro support enabled"
  maturin develop --release --features pro
else
  echo "Installing whitebox_workflows in open-only mode"
  maturin develop --release
fi

echo "whitebox_workflows installed in current Python environment"
