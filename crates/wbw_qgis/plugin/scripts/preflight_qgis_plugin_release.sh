#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SRC_DIR="$PLUGIN_ROOT/whitebox_workflows_qgis"
STAGE_ROOT="$PLUGIN_ROOT/.upload_stage"
STAGE_DIR="$STAGE_ROOT/whitebox_workflows_for_qgis"

if [[ ! -d "$SRC_DIR" ]]; then
  echo "ERROR: plugin source directory not found: $SRC_DIR" >&2
  exit 1
fi

VERSION=""
if [[ ${1:-} == "--version" ]]; then
  VERSION="${2:-}"
  if [[ -z "$VERSION" ]]; then
    echo "ERROR: --version requires a value" >&2
    exit 1
  fi
fi

if [[ -z "$VERSION" ]]; then
  VERSION="$(awk -F= '/^version=/{print $2; exit}' "$SRC_DIR/metadata.txt" | tr -d '[:space:]')"
fi

if [[ -z "$VERSION" ]]; then
  echo "ERROR: unable to resolve plugin version from metadata.txt" >&2
  exit 1
fi

ZIP_PATH="$PLUGIN_ROOT/whitebox_workflows_for_qgis-${VERSION}.zip"

echo "== QGIS Plugin Preflight =="
echo "Source: $SRC_DIR"
echo "Version: $VERSION"
echo "Output zip: $ZIP_PATH"

for required in metadata.txt LICENSE LICENSE-APACHE LICENSE-MIT __init__.py plugin.py; do
  if [[ ! -f "$SRC_DIR/$required" ]]; then
    echo "ERROR: missing required file: $SRC_DIR/$required" >&2
    exit 1
  fi
done

echo "- Required files: OK"

if command -v pycodestyle >/dev/null 2>&1; then
  echo "- Running pycodestyle (warnings only, does not block packaging)..."
  pycodestyle "$SRC_DIR"/*.py || true
else
  echo "- pycodestyle not installed; skipping style preflight"
fi

if command -v bandit >/dev/null 2>&1; then
  echo "- Running bandit security scan (JSON report in .upload_stage)..."
  mkdir -p "$STAGE_ROOT"
  bandit -q -r "$SRC_DIR" -f json -o "$STAGE_ROOT/bandit_report.json" || true
else
  echo "- bandit not installed; skipping security preflight"
fi

rm -rf "$STAGE_ROOT"
mkdir -p "$STAGE_DIR"

rsync -a \
  --exclude '__pycache__/' \
  --exclude '*.pyc' \
  --exclude '.DS_Store' \
  --exclude '._*' \
  "$SRC_DIR/" "$STAGE_DIR/"

find "$STAGE_ROOT" -name '__pycache__' -type d -prune -exec rm -rf {} +
find "$STAGE_ROOT" -name '.DS_Store' -type f -delete
find "$STAGE_ROOT" -name '._*' -type f -delete

rm -f "$ZIP_PATH"
(
  cd "$STAGE_ROOT"
  zip -r "$ZIP_PATH" whitebox_workflows_for_qgis >/dev/null
)

echo "- Archive created"

if ! unzip -Z1 "$ZIP_PATH" | awk 'BEGIN{found=0} $0=="whitebox_workflows_for_qgis/"{found=1} END{exit !found}'; then
  echo "ERROR: zip root folder is not whitebox_workflows_for_qgis" >&2
  exit 1
fi

if unzip -Z1 "$ZIP_PATH" | awk '/__pycache__\// || /\.DS_Store$/ || /\/\._/ {found=1} END{exit !found}'; then
  echo "ERROR: zip contains forbidden files (__pycache__/.DS_Store/._*)" >&2
  exit 1
fi

echo "- Archive structure checks: OK"
echo "DONE: $ZIP_PATH"
