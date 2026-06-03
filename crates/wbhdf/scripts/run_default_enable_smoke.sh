#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

# Tier 1 / HDF dispatch smoke checks
cargo check -p wbraster
cargo test -p wbraster raster_read_hdf -- --nocapture

# Representative non-HDF regressions
cargo test -p wbraster raster::tests::get_set -- --nocapture
cargo test -p wbraster raster::tests::statistics -- --nocapture
cargo test -p wbraster --test scalar_api_contract -- --nocapture
cargo test -p wbraster --test integration roundtrip_esri_ascii -- --nocapture
cargo test -p wbraster --test integration roundtrip_geotiff -- --nocapture

# Core wbhdf multilevel traversal regressions
cargo test -p wbhdf multilevel_internal_fanout -- --nocapture
cargo test -p wbhdf budget_exhaustion -- --nocapture
