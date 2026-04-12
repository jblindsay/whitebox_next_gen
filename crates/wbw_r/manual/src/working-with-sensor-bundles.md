# Working with Sensor Bundles

This chapter documents supported sensor bundle families and common workflows.

Bundle APIs provide a common operational interface across heterogeneous sensor
families. The conceptual goal is to standardize ingestion and inspection steps
even when archive layout, naming conventions, or QA assets differ between
providers.

## Supported Families

- Generic auto-detect: `wbw_read_bundle(...)`
- Landsat: `wbw_read_landsat(...)`
- Sentinel-1 SAFE: `wbw_read_sentinel1(...)`
- Sentinel-2 SAFE: `wbw_read_sentinel2(...)`
- PlanetScope: `wbw_read_planetscope(...)`
- ICEYE: `wbw_read_iceye(...)`
- DIMAP: `wbw_read_dimap(...)`
- Maxar/WorldView: `wbw_read_maxar_worldview(...)`
- RADARSAT-2: `wbw_read_radarsat2(...)`
- RCM: `wbw_read_rcm(...)`

## Common Inspection Pattern

Start here to establish what each bundle contains before choosing analysis
channels.

```r
library(whiteboxworkflows)

b <- wbw_read_bundle('BUNDLE_ROOT')
print(b$family)
print(b$key_summary())
print(b$list_band_keys())
print(b$list_measurement_keys())
print(b$list_qa_keys())
print(b$list_asset_keys())
```

## Family-Specific Examples

These examples follow one shared shape across providers: load, inspect,
sample key channels, then generate QA-ready outputs.

### Sentinel-2

Use this for optical band workflows where band identity is explicit.

```r
s2 <- wbw_read_sentinel2('S2_SCENE.SAFE')
red <- s2$read_band('B04')
nir <- s2$read_band('B08')
rgb <- s2$write_true_colour('s2_true_colour.tif')
```

### Landsat

Use this when metadata such as cloud cover and processing level drive filtering.

```r
ls <- wbw_read_landsat('LC09_SCENE')
print(ls$processing_level())
print(ls$cloud_cover_percent())
preview <- ls$read_band(ls$list_band_keys()[[1]])
```

### Sentinel-1 / SAR Families

Use this pattern for SAR measurement access and quick false-colour QA.

```r
s1 <- wbw_read_sentinel1('S1_SCENE.SAFE')
print(s1$polarizations())
meas <- s1$read_measurement(s1$list_measurement_keys()[[1]])
fc <- s1$write_false_colour('s1_false_colour.tif')
```

### PlanetScope / ICEYE / DIMAP / Maxar-WorldView / RADARSAT-2 / RCM

Use this loop for multi-provider intake checks with a consistent script shape.

```r
loaders <- list(
  wbw_read_planetscope,
  wbw_read_iceye,
  wbw_read_dimap,
  wbw_read_maxar_worldview,
  wbw_read_radarsat2,
  wbw_read_rcm
)

paths <- list('PLANETSCOPE_SCENE', 'ICEYE_SCENE', 'DIMAP_SCENE', 'MAXAR_SCENE', 'RADARSAT2_SCENE', 'RCM_SCENE')

for (i in seq_along(loaders)) {
  b <- loaders[[i]](paths[[i]])
  print(c(b$family, b$bundle_root))
}
```

## Recommended Workflow

1. Open with family-specific reader when known.
2. Inspect key groups with `key_summary()` and `list_*_keys()`.
3. Read representative channels via `read_any()` or family-specific methods.
4. Generate preview/composite rasters for QA.
5. Persist derived outputs and record source family + key choices.

## Sensor Bundle Object Method Reference

### Metadata and Key Discovery

| Method | Description |
|---|---|
| `metadata` | Return bundle metadata (family, product descriptors, CRS hints). |
| `family`, `bundle_root`, `path` | Return family ID and resolved bundle root/path. |
| `key_summary` | Summarize available key groups and counts. |
| `list_band_keys`, `list_measurement_keys` | List band/measurement keys. |
| `list_qa_keys`, `list_aux_keys`, `list_asset_keys` | List QA, auxiliary, and asset keys. |
| `resolve_key`, `has_key` | Resolve and verify key existence across key types. |

### Raster Access by Key

| Method | Description |
|---|---|
| `read_any` | Read key by searching configured key types. |
| `read_band` | Read band key as raster. |
| `read_measurement` | Read measurement key as raster. |
| `read_qa_layer`, `read_aux_layer`, `read_asset` | Read QA/auxiliary/asset keys as rasters. |
| `read_preview_raster` | Return selected preview raster descriptor. |

### Composite Generation

| Method | Description |
|---|---|
| `write_true_colour` | Write true-colour composite raster. |
| `write_false_colour` | Write false-colour composite raster. |

### Common Metadata Accessors

| Method | Description |
|---|---|
| `acquisition_datetime_utc` | Return acquisition timestamp when available. |
| `processing_level`, `product_type` | Return processing/product descriptors. |
| `tile_id`, `mission`, `acquisition_mode` | Return common platform and acquisition descriptors. |
| `cloud_cover_percent`, `polarizations` | Return cloud or polarization metadata when available. |
