#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../../.." && pwd)"
CRATE_DIR="$ROOT_DIR/crates/wbw_r"
PACKAGE_DIR="$CRATE_DIR/r-package/whiteboxworkflows"
BUILD_PROFILE="${BUILD_PROFILE:-debug}"
FEATURE_ARGS=""
INCLUDE_PRO=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --pro)
      INCLUDE_PRO=true
      FEATURE_ARGS="--features pro"
      shift
      ;;
    --release)
      BUILD_PROFILE=release
      shift
      ;;
    *)
      echo "Unknown argument: $1" >&2
      echo "Usage: $(basename "$0") [--pro] [--release]" >&2
      exit 2
      ;;
  esac
done

pushd "$ROOT_DIR" >/dev/null
if [[ "$BUILD_PROFILE" == "release" ]]; then
  cargo build -p wbw_r ${FEATURE_ARGS} --release
else
  cargo build -p wbw_r ${FEATURE_ARGS}
fi

EXT=".so"
if [[ "$(uname -s)" == "Darwin" ]]; then
  EXT=".dylib"
elif [[ "$(uname -s)" == "MINGW"* || "$(uname -s)" == "MSYS"* || "$(uname -s)" == "CYGWIN"* ]]; then
  EXT=".dll"
fi

LIB_SRC="$ROOT_DIR/target/$BUILD_PROFILE/libwbw_r$EXT"
LIB_DEST_DIR="$PACKAGE_DIR/inst/libs"
LIB_DEST="$LIB_DEST_DIR/libwbw_r$EXT"

mkdir -p "$LIB_DEST_DIR"
cp "$LIB_SRC" "$LIB_DEST"

cargo run -p wbw_r ${FEATURE_ARGS} --example generate_r_wrappers -- $([[ "$INCLUDE_PRO" == true ]] && echo --include-pro) --tier "$([[ "$INCLUDE_PRO" == true ]] && echo pro || echo open)" --output "$CRATE_DIR/generated/wbw_tools_generated.R"
cp "$CRATE_DIR/generated/wbw_tools_generated.R" "$PACKAGE_DIR/R/zz_generated_wrappers.R"

popd >/dev/null

echo "Staged native library at $LIB_DEST"
echo "Refreshed generated wrappers in $PACKAGE_DIR/R/zz_generated_wrappers.R"
