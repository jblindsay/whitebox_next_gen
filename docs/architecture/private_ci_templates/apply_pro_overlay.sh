#!/usr/bin/env bash
set -euo pipefail

# Private CI-only helper.
#
# Purpose:
# - In target-state shim design: copy private Pro crate into the public shim slot.
# - In legacy path design: ensure the expected sibling path exists via symlink.
#
# Expected checkout layout in CI workspace:
#   $GITHUB_WORKSPACE/whitebox_next_gen
#   $GITHUB_WORKSPACE/wbtools_pro
#
# Required env guard:
#   ALLOW_PRIVATE_OVERLAY=true

if [[ "${ALLOW_PRIVATE_OVERLAY:-}" != "true" ]]; then
  echo "ERROR: Refusing to apply private overlay without ALLOW_PRIVATE_OVERLAY=true"
  exit 1
fi

PUBLIC_ROOT="${PUBLIC_ROOT:-${GITHUB_WORKSPACE:-}/whitebox_next_gen}"
PRIVATE_PRO_ROOT="${PRIVATE_PRO_ROOT:-${GITHUB_WORKSPACE:-}/wbtools_pro}"
SHIM_DIR="${PUBLIC_ROOT}/crates/wbtools_pro_shim"
LEGACY_LINK="${PUBLIC_ROOT}/../wbtools_pro"

if [[ ! -d "$PUBLIC_ROOT" ]]; then
  echo "ERROR: PUBLIC_ROOT not found: $PUBLIC_ROOT"
  exit 1
fi

if [[ ! -f "$PRIVATE_PRO_ROOT/Cargo.toml" ]]; then
  echo "ERROR: PRIVATE_PRO_ROOT does not look like a Rust crate repo: $PRIVATE_PRO_ROOT"
  exit 1
fi

echo "Applying private overlay"
echo "  PUBLIC_ROOT:      $PUBLIC_ROOT"
echo "  PRIVATE_PRO_ROOT: $PRIVATE_PRO_ROOT"

if [[ -d "$SHIM_DIR" ]]; then
  echo "Detected shim layout at $SHIM_DIR; syncing private crate into shim slot"
  rm -rf "$SHIM_DIR"
  mkdir -p "$(dirname "$SHIM_DIR")"
  rsync -a --delete --exclude '.git/' "$PRIVATE_PRO_ROOT/" "$SHIM_DIR/"
  echo "Shim overlay applied"
else
  echo "Shim layout not found; applying legacy sibling-path compatibility link"
  mkdir -p "$(dirname "$LEGACY_LINK")"
  ln -sfn "$PRIVATE_PRO_ROOT" "$LEGACY_LINK"
  echo "Legacy overlay link applied: $LEGACY_LINK -> $PRIVATE_PRO_ROOT"
fi

# Show effective dependency locations for quick debugging.
if [[ -f "$PUBLIC_ROOT/crates/wbw_python/Cargo.toml" ]]; then
  echo "--- wbw_python dep line ---"
  grep -n "wbtools_pro" "$PUBLIC_ROOT/crates/wbw_python/Cargo.toml" || true
fi
if [[ -f "$PUBLIC_ROOT/crates/wbw_r/Cargo.toml" ]]; then
  echo "--- wbw_r dep line ---"
  grep -n "wbtools_pro" "$PUBLIC_ROOT/crates/wbw_r/Cargo.toml" || true
fi

echo "Private overlay complete"
