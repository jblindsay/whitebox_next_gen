# Working with Sensor Bundles

This chapter documents supported sensor bundle families and common workflows.

Bundle APIs abstract away family-specific packaging details so you can focus on
analysis intent: identify useful channels, inspect quality assets, and produce
consistent derived outputs. The conceptual goal is to normalize early-stage
ingestion across heterogeneous providers while preserving enough provenance to
keep downstream interpretation defensible.

## Supported Families

- Generic auto-detect: `read_bundle(...)`
- Landsat: `read_landsat(...)`
- Sentinel-1 SAFE: `read_sentinel1(...)`
- Sentinel-2 SAFE: `read_sentinel2(...)`
- PlanetScope: `read_planetscope(...)`
- ICEYE: `read_iceye(...)`
- DIMAP: `read_dimap(...)`
- Maxar/WorldView: `read_maxar_worldview(...)`
- RADARSAT-2: `read_radarsat2(...)`
- RCM: `read_rcm(...)`

## Common Inspection Pattern

Start with this inspection sequence to quickly understand what content is
available before choosing analysis-specific channels.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
b = wbe.read_bundle('BUNDLE_ROOT')

print('family:', b.family)
print('bands:', b.list_band_keys())
print('measurements:', b.list_measurement_keys())
print('qa:', b.list_qa_keys())
print('assets:', b.list_asset_keys())
```

## Family-Specific Examples

These examples show the same workflow shape across different providers: load,
inspect keys, read representative channels, then generate a preview or derived
artifact.

### Sentinel-2

Use this when building optical workflows from known band identifiers.

```python
s2 = wbe.read_sentinel2('S2_SCENE.SAFE')
red = s2.read_band('B04')
nir = s2.read_band('B08')
rgb = s2.true_colour_composite(wbe, output_path='s2_true_colour.tif')
```

### Landsat

Use this when scene metadata and quality indicators are part of ingestion logic.

```python
ls = wbe.read_landsat('LC09_SCENE')
print(ls.processing_level(), ls.cloud_cover_percent())
preview = ls.read_band(ls.list_band_keys()[0])
```

### Sentinel-1 / SAR Families

Use this for SAR measurement inspection and quick visual QA composites.

```python
s1 = wbe.read_sentinel1('S1_SCENE.SAFE')
print(s1.polarizations())
meas = s1.read_measurement(s1.list_measurement_keys()[0])
false_colour = s1.false_colour_composite(wbe, output_path='s1_false_colour.tif')
```

### PlanetScope / ICEYE / DIMAP / Maxar-WorldView / RADARSAT-2 / RCM

This loop pattern is useful for multi-provider pipelines that need consistent
intake checks.

```python
for loader, path in [
    (wbe.read_planetscope, 'PLANETSCOPE_SCENE'),
    (wbe.read_iceye, 'ICEYE_SCENE'),
    (wbe.read_dimap, 'DIMAP_SCENE'),
    (wbe.read_maxar_worldview, 'MAXAR_SCENE'),
    (wbe.read_radarsat2, 'RADARSAT2_SCENE'),
    (wbe.read_rcm, 'RCM_SCENE'),
]:
    bundle = loader(path)
    print(bundle.family, bundle.bundle_root)
```

## Recommended Workflow

1. Open with family-specific reader when known.
2. Inspect key sets (`list_*_keys`).
3. Read representative channels.
4. Build previews/composites for QA.
5. Persist derived rasters and document source family + key choices.

## Sensor Bundle Object Method Reference

Common bundle properties such as `family` and `bundle_root` are omitted here so
the tables stay focused on callable `Bundle` methods.

### Key Discovery and Data Access

| Method | Description |
|---|---|
| `list_band_keys` | List spectral or image-band keys exposed by the bundle. |
| `list_measurement_keys` | List measurement-layer keys, commonly used in SAR families. |
| `list_qa_keys` | List quality-assurance layer keys. |
| `list_asset_keys` | List auxiliary assets packaged with the bundle. |
| `list_aux_keys` | List auxiliary layers that are not primary bands or measurements. |
| `read_band` | Read a band by key as a raster object. |
| `read_measurement` | Read a measurement layer by key as a raster object. |
| `read_qa_layer` | Read a QA layer by key as a raster object. |
| `read_asset` | Read a named asset from the bundle. |
| `read_aux_layer` | Read an auxiliary layer by key. |

### Scene and Platform Metadata

| Method | Description |
|---|---|
| `metadata_json` | Return the bundle metadata payload as JSON text. |
| `mission`, `product_type`, `processing_level`, `processing_baseline` | Inspect the provider, product family, and processing lineage. |
| `scene_id`, `tile_id`, `collection_number`, `path_row` | Inspect scene identifiers used for cataloging and lineage. |
| `acquisition_datetime_utc`, `acquisition_mode` | Inspect when and how the scene was acquired. |
| `cloud_cover_percent` | Report cloud cover when the bundle family exposes it. |
| `polarization`, `polarizations` | Inspect single-polarization or multi-polarization SAR metadata. |
| `look_direction`, `orbit_direction` | Inspect platform look and orbit geometry. |
| `incidence_angle_near_deg`, `incidence_angle_far_deg`, `off_nadir_angle_deg`, `view_angle_deg` | Inspect sensor/view geometry angles. |
| `sun_azimuth_deg`, `sun_elevation_deg`, `sun_zenith_deg` | Inspect solar geometry for optical scenes. |
| `pixel_spacing_range_m`, `pixel_spacing_azimuth_m` | Inspect SAR-oriented pixel spacing metadata. |
| `spatial_bounds` | Return spatial footprint bounds for the scene. |

### Derived Preview Products

| Method | Description |
|---|---|
| `true_colour_composite` | Build a true-colour preview raster when the bundle family supports it. |
| `false_colour_composite` | Build a false-colour preview raster for quick QA or analysis setup. |
