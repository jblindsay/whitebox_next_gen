#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
SRC_ASC="$ROOT_DIR/src_epsg4326_small.asc"
SRC_TIF="$ROOT_DIR/src_epsg4326_small.tif"

# Build source GeoTIFF with explicit CRS.
gdal_translate -q -a_srs EPSG:4326 "$SRC_ASC" "$SRC_TIF"

# Common target grid in Web Mercator.
TE_MIN_X=-2.0
TE_MIN_Y=-2.0
TE_MAX_X=0.0
TE_MAX_Y=0.0
OUT_COLS=14
OUT_ROWS=12

# GDAL resampling names mapped to fixture filenames.
for R in near bilinear cubic lanczos average min max mode med; do
  OUT="$ROOT_DIR/expected_epsg3857_${R}.tif"
  gdalwarp -q -overwrite \
    -s_srs EPSG:4326 \
    -t_srs EPSG:3857 \
    -r "$R" \
    -te_srs EPSG:4326 \
    -te "$TE_MIN_X" "$TE_MIN_Y" "$TE_MAX_X" "$TE_MAX_Y" \
    -ts "$OUT_COLS" "$OUT_ROWS" \
    "$SRC_TIF" "$OUT"
done

echo "Generated parity fixtures in: $ROOT_DIR"
