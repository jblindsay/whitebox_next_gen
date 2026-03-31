# Reprojection parity fixtures

This folder stores deterministic fixture rasters used by [tests/reprojection_parity.rs](../../reprojection_parity.rs).

## Fixture set

- `src_epsg4326_small.asc` / `src_epsg4326_small.tif`: source raster in EPSG:4326.
- `expected_epsg3857_*.tif`: GDAL/PROJ reference outputs in EPSG:3857 with fixed grid (`14x12`).

Current core parity assertions run for these resamplers:
- `near`
- `bilinear`
- `cubic`
- `lanczos`

Additional generated fixtures (`average`, `min`, `max`, `mode`, `med`) are present for future parity expansion as algorithms converge.

## Regenerating fixtures

From the repository root:

```bash
./tests/fixtures/reprojection_parity/generate_fixtures.sh
```

Requirements:
- `gdal_translate`
- `gdalwarp`

The script is deterministic: it overwrites existing expected files with the same grid settings each run.

## Optional verbose metrics

To print overlap/MAE/RMSE/max-error metrics for each parity case during test runs:

```bash
WB_PARITY_VERBOSE=1 cargo test --test reprojection_parity -- --nocapture
```
