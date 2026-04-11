# Working with Sensor Bundles

This chapter documents supported sensor bundle families and common workflows.

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

### Sentinel-2

```python
s2 = wbe.read_sentinel2('S2_SCENE.SAFE')
red = s2.read_band('B04')
nir = s2.read_band('B08')
rgb = s2.true_colour_composite(wbe, output_path='s2_true_colour.tif')
```

### Landsat

```python
ls = wbe.read_landsat('LC09_SCENE')
print(ls.processing_level(), ls.cloud_cover_percent())
preview = ls.read_band(ls.list_band_keys()[0])
```

### Sentinel-1 / SAR Families

```python
s1 = wbe.read_sentinel1('S1_SCENE.SAFE')
print(s1.polarizations())
meas = s1.read_measurement(s1.list_measurement_keys()[0])
false_colour = s1.false_colour_composite(wbe, output_path='s1_false_colour.tif')
```

### PlanetScope / ICEYE / DIMAP / Maxar-WorldView / RADARSAT-2 / RCM

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
