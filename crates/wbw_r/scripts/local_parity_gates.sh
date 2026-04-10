#!/usr/bin/env bash
set -euo pipefail

# Local parity gates for wbw_r.
# This script intentionally replaces hosted CI checks to avoid GitHub Actions minutes.

repo_root="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$repo_root"

echo "[1/3] Wrapper parity gate"
cargo test -p wbw_r r_wrapper_module_generation_matches_manifest_count_and_names --quiet

echo "[2/3] Build/install R package"
R CMD INSTALL crates/wbw_r/r-package/whiteboxworkflows --no-multiarch

echo "[3/3] R package smoke tests"
Rscript -e 'testthat::test_local("crates/wbw_r/r-package/whiteboxworkflows")'

echo "Local parity gates passed."
