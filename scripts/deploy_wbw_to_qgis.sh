#!/usr/bin/env bash
# -----------------------------------------------------------------------------
# deploy_wbw_to_qgis.sh — Build and deploy whitebox_workflows to QGIS
#
# Usage:
#   ./scripts/deploy_wbw_to_qgis.sh           # open-mode wheel
#   ./scripts/deploy_wbw_to_qgis.sh --pro     # pro-enabled wheel
#   ./scripts/deploy_wbw_to_qgis.sh --no-build --pro   # install existing wheel
#
# What it does:
#   1. Optionally builds the wheel with maturin (defaults to pro if --pro given)
#   2. Installs it to the QGIS plugin pip target directory
#   3. Prints a QGIS Python Console reload command to paste
# -----------------------------------------------------------------------------
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CRATE_DIR="$REPO_ROOT/crates/wbw_python"
WHEELS_DIR="$REPO_ROOT/target/wheels"

QGIS_PROFILE_DIR="$HOME/Library/Application Support/QGIS/QGIS4/profiles/default"
QGIS_TARGET_DIR="$QGIS_PROFILE_DIR/python/whitebox_workflows_lib"

BUILD=true
FEATURES=""

for arg in "$@"; do
  case "$arg" in
    --pro)        FEATURES="--features pro" ;;
    --no-build)   BUILD=false ;;
    --upgrade)    : ;;  # accepted but handled by pre-clean below
  esac
done

# ----- Build -----
if [ "$BUILD" = true ]; then
  echo "==> Building whitebox_workflows wheel $FEATURES ..."
  cd "$CRATE_DIR"
  source "$REPO_ROOT/.venv-wbw/bin/activate"
  maturin build --release $FEATURES 2>&1 | tail -8
  deactivate 2>/dev/null || true
fi

# ----- Find the wheel -----
WHEEL=$(ls -t "$WHEELS_DIR"/whitebox_workflows-*.whl 2>/dev/null | head -1)
if [ -z "$WHEEL" ]; then
  echo "ERROR: No wheel found in $WHEELS_DIR. Run without --no-build first."
  exit 1
fi
echo "==> Wheel: $(basename "$WHEEL")"

# ----- Install to QGIS target dir -----
mkdir -p "$QGIS_TARGET_DIR"
echo "==> Installing to: $QGIS_TARGET_DIR"

# Pre-clean existing package files so pip --target doesn't warn about existing dirs.
rm -rf "$QGIS_TARGET_DIR/whitebox_workflows"
rm -rf "$QGIS_TARGET_DIR"/whitebox_workflows-*.dist-info

python3 -m pip install \
  --target "$QGIS_TARGET_DIR" \
  --no-deps \
  "$WHEEL"

echo ""
echo "==> Done. Reload in QGIS Python Console with:"
echo ""
echo '    import importlib, sys'
echo '    for key in list(sys.modules): '
echo '        if "whitebox" in key: del sys.modules[key]'
echo '    import qgis.utils'
echo '    qgis.utils.unloadPlugin("whitebox_workflows_qgis")'
echo '    qgis.utils.loadPlugin("whitebox_workflows_qgis")'
echo '    qgis.utils.startPlugin("whitebox_workflows_qgis")'
echo '    print("Reloaded")'
echo ""
