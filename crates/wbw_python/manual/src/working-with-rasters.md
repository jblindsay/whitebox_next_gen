# Working with Rasters

This chapter documents practical raster workflows in WbW-Py with emphasis on
inspection, iteration, modification, and persistence.

## Raster Lifecycle

Typical lifecycle:
1. Read raster.
2. Inspect metadata.
3. Transform values (tool-driven or array-driven).
4. Persist outputs with explicit options when needed.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
dem = wbe.read_raster('dem.tif')
meta = dem.metadata()

print(meta.rows, meta.columns)
print('EPSG:', meta.epsg_code)
print('NoData:', meta.nodata)
```

## Iterating Through Grid Cells

For cell-level logic, convert to NumPy and iterate safely.

```python
import numpy as np
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
r = wbe.read_raster('dem.tif')
a = r.to_numpy(all_bands=False, dtype='float64')
meta = r.metadata()

rows, cols = a.shape
for row in range(rows):
    for col in range(cols):
        z = a[row, col]
        if np.isfinite(z) and z != meta.nodata:
            # Example transform: clamp negatives.
            if z < 0:
                a[row, col] = 0.0
```

## Writing Modified Data Back

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
base = wbe.read_raster('dem.tif')
a = base.to_numpy(all_bands=False)
a = a * 1.05

out = wb.Raster.from_numpy(a, base, output_path='dem_scaled.tif')
wbe.write_raster(out, 'dem_scaled_cog.tif', options={
    'compress': True,
    'strict_format_options': True,
    'geotiff': {'layout': 'cog', 'tile_size': 512},
})
```

## Multi-Band Iteration

```python
import numpy as np
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
rgb = wbe.read_raster('multiband.tif')
arr = rgb.to_numpy(all_bands=True, dtype='float32')
# arr shape is typically (bands, rows, cols)

bands, rows, cols = arr.shape
for b in range(bands):
    for row in range(rows):
        for col in range(cols):
            v = arr[b, row, col]
            if np.isfinite(v):
                arr[b, row, col] = max(v, 0.0)

wb.Raster.from_numpy(arr, rgb, output_path='multiband_clamped.tif')
```

## Tool-First Raster Processing

Use tools for most heavy computation, then iterate only where custom logic is needed.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
dem = wbe.read_raster('dem.tif')
filled = wbe.hydrology.fill_depressions(dem)
slope = wbe.terrain.slope(filled)

# Optional post-processing pass in NumPy.
a = slope.to_numpy(all_bands=False)
a[a < 0] = 0
slope_fixed = wb.Raster.from_numpy(a, slope)

wbe.write_raster(slope_fixed, 'slope_fixed.tif')
```

## NoData and Performance Guidance

- Always check `metadata().nodata` when doing per-cell iteration.
- Prefer vectorized NumPy operations over nested Python loops when possible.
- Use tool methods (`wbe.hydrology.*`, `wbe.raster.*`, `wbe.terrain.*`) for large transforms.
- Persist only final outputs where possible to keep memory-first workflows efficient.
