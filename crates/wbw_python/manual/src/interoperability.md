# Interoperability

This chapter provides practical exchange patterns between WbW-Py and common Python geospatial tooling.

Interoperability is best thought of as controlled boundary crossing. Each
conversion introduces potential differences in metadata conventions, numeric
precision, CRS representation, and schema typing. The workflows in this chapter
focus on explicit handoff points and roundtrip validation so multi-library
pipelines remain trustworthy.

## Copy-Boundary Model

- `to_numpy()` / `from_numpy()` are explicit in-memory exchange boundaries.
- Rasterio/GeoPandas/rioxarray flows are file-based boundaries.
- Always validate metadata after roundtrip (`metadata()`, CRS, dimensions, schema).

## NumPy Roundtrip

Use this when you need direct numeric control for custom raster math.

```python
import numpy as np
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
r = wbe.read_raster('dem.tif')
a = r.to_numpy(dtype='float64')
a = np.where(np.isfinite(a), a + 1.0, a)

r2 = wb.Raster.from_numpy(a, r, output_path='dem_plus1.tif')
```

## Rasterio Roundtrip

Use this for compatibility with rasterio-centric ecosystems and workflows.

```python
import rasterio
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
r = wbe.read_raster('dem.tif')
wbe.write_raster(r, 'dem_for_rasterio.tif')

with rasterio.open('dem_for_rasterio.tif') as src:
    arr = src.read(1)
    profile = src.profile

arr = arr * 1.02
profile.update(dtype='float32', count=1)
with rasterio.open('dem_rio_out.tif', 'w', **profile) as dst:
    dst.write(arr.astype('float32'), 1)

r_back = wbe.read_raster('dem_rio_out.tif')
print(r_back.metadata())
```

## GeoPandas and Shapely Roundtrip

Use this pattern for vector enrichment and geometry filtering in the broader
Python geospatial stack.

```python
import geopandas as gpd
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
v = wbe.read_vector('roads.gpkg')
wbe.write_vector(v, 'roads_for_gpd.gpkg')

gdf = gpd.read_file('roads_for_gpd.gpkg')
gdf['length_m'] = gdf.length
gdf = gdf[gdf['length_m'] > 20.0]
gdf.to_file('roads_filtered.gpkg', driver='GPKG')

v_back = wbe.read_vector('roads_filtered.gpkg')
print(v_back.schema())
```

## xarray/rioxarray Roundtrip

Use this when you need labeled-array operations or rolling-window processing.

```python
import rioxarray as rxr
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
r = wbe.read_raster('dem.tif')
wbe.write_raster(r, 'dem_for_xarray.tif')

da = rxr.open_rasterio('dem_for_xarray.tif').squeeze(drop=True)
da_smoothed = da.rolling(x=3, y=3, center=True).mean()
da_smoothed.rio.to_raster('dem_xarray_smoothed.tif')

r_back = wbe.read_raster('dem_xarray_smoothed.tif')
print(r_back.metadata())
```

## pyproj CRS Workflow

Use this for explicit CRS introspection and conversion checks outside full file
I/O steps.

```python
from pyproj import CRS
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
r = wbe.read_raster('dem.tif')

src = CRS.from_epsg(r.metadata().epsg_code)
dst = CRS.from_epsg(32618)
print(src.to_string(), '->', dst.to_string())

r_utm = wbe.reproject_raster(r, dst_epsg=dst.to_epsg(), resample='bilinear')
```

## Validation Checklist

1. Check CRS after every roundtrip.
2. Check dimensions and nodata for raster flows.
3. Check schema and representative attributes for vector flows.
4. Prefer stable formats (`.tif`, `.gpkg`) for routine exchange.
