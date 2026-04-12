# Working with Rasters

This chapter documents practical raster workflows in WbW-Py with emphasis on
inspection, iteration, modification, and persistence.

Raster workflows usually alternate between high-performance tool operations and
targeted custom logic. The important concept is to choose the lowest-cost path
for each step: use backend tools for heavy transformations, and reserve
NumPy-level iteration for domain-specific adjustments that tools do not expose
directly.

## Raster Lifecycle

This lifecycle helps you separate inspection from transformation so assumptions
about CRS, resolution, and nodata are explicit before heavy operations.

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

Use this pattern only when tool methods or vectorized operations cannot express
your custom rule directly.

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

This example shows the common pattern of deriving a new raster while preserving
georeferencing context from a base raster.

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

Use this structure when per-band logic differs or when your transform depends on
band-specific rules.

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

This is the preferred pattern for scale: run optimized tools first, then apply
targeted custom fixes only where necessary.

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

## Raster Object Method Reference

The tables below focus on callable `Raster` methods. Common simple properties
such as `file_path`, `file_name`, `active_band`, and `band_count` are omitted
from the tables to keep the reference readable.

### Construction, Conversion, and Summary

| Method | Description |
|---|---|
| `from_numpy` | Create a raster from a NumPy array while inheriting grid geometry from a base raster. |
| `band` | Return a band-specific raster view for multiband data. |
| `to_numpy` | Export the active band or all bands to NumPy for custom numeric work. |
| `deep_copy` | Write a full raster copy to a derived or explicit output path. |
| `new_from_other`, `new_from_other_with_data` | Create a new raster that inherits geometry from another raster, optionally with a new data buffer. |
| `metadata` | Return the `RasterConfigs` summary for rows, columns, resolution, bounds, and nodata. |
| `calculate_clip_values` | Calculate percentile-based lower and upper clip values. |
| `calculate_mean`, `calculate_mean_and_stdev` | Compute simple raster summary statistics. |
| `normalize` | Produce a normalized raster suitable for display or downstream scaling steps. |
| `reinitialize_values` | Reset all cells to a single constant value. |
| `update_min_max` | Recompute cached minimum and maximum values after edits. |
| `num_cells`, `num_valid_cells` | Report total cell count and valid-cell count. |
| `size_of`, `get_data_size_in_bytes` | Report approximate in-memory or backing-data size. |
| `is_memory_backed` | Indicate whether the raster is currently memory-backed. |
| `get_short_filename`, `get_file_extension` | Return convenience filename information for reporting or output naming. |

### Grid Navigation and Direct Editing

| Method | Description |
|---|---|
| `get_value`, `set_value` | Read or write an individual cell value. |
| `get_row_data`, `set_row_data` | Read or replace an entire row of raster values. |
| `increment`, `decrement` | Add to or subtract from a single cell value. |
| `increment_row_data`, `decrement_row_data` | Add to or subtract from every value in a row. |
| `is_cell_nodata` | Check whether a specific cell is nodata. |
| `get_row_from_y`, `get_y_from_row` | Convert between world Y coordinates and raster row indices. |
| `get_column_from_x`, `get_x_from_column` | Convert between world X coordinates and raster column indices. |

### CRS and Reprojection

| Method | Description |
|---|---|
| `crs_wkt`, `crs_epsg` | Inspect the raster CRS as WKT text or inferred/declared EPSG code. |
| `set_crs_wkt`, `set_crs_epsg` | Assign CRS metadata without moving the raster grid. |
| `clear_crs` | Remove CRS metadata when it is known to be wrong or must be re-assigned. |
| `reproject` | Reproject the raster with explicit control over resampling, extent, resolution, and grid policies. |
| `reproject_nearest`, `reproject_bilinear` | Reproject with a fixed resampling method for common nearest-neighbour or bilinear cases. |
| `reproject_to_match_grid` | Reproject onto the exact grid geometry of a target raster. |
| `reproject_to_match_resolution` | Reproject while matching the cell resolution of a reference raster. |
| `reproject_to_match_resolution_in_epsg` | Reproject to a target EPSG while borrowing cell resolution from a reference raster. |

### Unary Math and Numeric Transforms

| Method | Description |
|---|---|
| `abs` | Absolute value transform. |
| `acos`, `arccos` | Inverse cosine transform. |
| `acosh` | Inverse hyperbolic cosine transform. |
| `asin`, `arcsin` | Inverse sine transform. |
| `asinh` | Inverse hyperbolic sine transform. |
| `atan`, `arctan` | Inverse tangent transform. |
| `atanh` | Inverse hyperbolic tangent transform. |
| `cbrt` | Cube-root transform. |
| `ceil`, `floor`, `round`, `trunc` | Standard rounding-family transforms. |
| `clamp` | Limit values to a minimum and maximum range. |
| `cos`, `cosh`, `sin`, `sinh`, `tan`, `tanh` | Trigonometric and hyperbolic transforms. |
| `degrees`, `to_degrees`, `radians`, `to_radians` | Convert angular units between radians and degrees. |
| `exp`, `exp2`, `expm1` | Exponential transforms. |
| `ln`, `log10`, `log1p`, `log2` | Natural-log and common logarithmic transforms. |
| `neg`, `signum`, `sqrt`, `square`, `recip` | Negation, sign extraction, square root, squaring, and reciprocal transforms. |
| `is_finite`, `is_infinite`, `is_nan`, `is_nodata` | Build masks from numeric validity tests. |
| `logical_not`, `logical_not_in_place`, `not_` | Logical-not style mask inversion. |

### Binary Arithmetic and Comparisons

| Method | Description |
|---|---|
| `add`, `add_in_place` | Add another raster or scalar, either to a new raster or in place. |
| `sub`, `subtract`, `sub_in_place` | Subtract another raster or scalar. |
| `mul`, `multiply`, `mul_in_place` | Multiply by another raster or scalar. |
| `div`, `divide`, `div_in_place` | Divide by another raster or scalar. |
| `pow`, `power`, `pow_in_place` | Raise values to a raster/scalar power. |
| `atan2` | Compute two-argument arctangent from paired raster/scalar inputs. |
| `min`, `max` | Compute cellwise minima or maxima. |
| `eq`, `eq_in_place` | Equality comparison. |
| `ne`, `ne_in_place` | Inequality comparison. |
| `gt`, `gt_in_place` | Greater-than comparison. |
| `ge`, `ge_in_place` | Greater-than-or-equal comparison. |
| `lt`, `lt_in_place` | Less-than comparison. |
| `le`, `le_in_place` | Less-than-or-equal comparison. |

### Logical Combination

| Method | Description |
|---|---|
| `and_` | Bitwise-style cellwise AND combination. |
| `or_` | Bitwise-style cellwise OR combination. |
| `xor_` | Bitwise-style cellwise XOR combination. |
| `logical_and`, `logical_and_in_place` | Logical AND for boolean or mask rasters. |
| `logical_or`, `logical_or_in_place` | Logical OR for boolean or mask rasters. |
| `logical_xor`, `logical_xor_in_place` | Logical XOR for boolean or mask rasters. |
