# Whitebox Workflows for Python — Tool Reference

This document lists every tool exposed through the `WbEnvironment` API and the
`whitebox_workflows` module-level functions.  For a general overview of the API,
data types, and how to configure the environment see
[README.md](README.md).

---

## Table of Contents

- [Common conventions](#common-conventions)
- [Raster I/O](#raster-io)
- [Unary raster math — wbe.* methods](#unary-raster-math--wbe-methods)
- [Binary raster math — wbe.* methods](#binary-raster-math--wbe-methods)
- [Raster object methods](#raster-object-methods)
  - [Unary math](#unary-math-raster-methods)
  - [Binary / scalar math](#binary--scalar-math-raster-methods)
  - [Comparison and logical](#comparison-and-logical-raster-methods)
  - [In-place mutation](#in-place-mutation-raster-methods)
  - [Utility](#utility-raster-methods)
- [Vector I/O](#vector-io)
- [Lidar I/O](#lidar-io)
- [Reprojection](#reprojection)

### Themed tool reference documents

Detailed reference for each tool theme is in a dedicated document:

| Theme | Document |
|---|---|
| Math & Statistics | [docs/tools_math.md](docs/tools_math.md) |
| Hydrology | [docs/tools_hydrology.md](docs/tools_hydrology.md) |
| GIS | [docs/tools_gis.md](docs/tools_gis.md) |
| Remote Sensing | [docs/tools_remote_sensing.md](docs/tools_remote_sensing.md) |
| Geomorphometry | [docs/tools_geomorphometry.md](docs/tools_geomorphometry.md) |
| Precision Agriculture | [docs/tools_agriculture.md](docs/tools_agriculture.md) |
| LiDAR Processing | [docs/tools_lidar_processing.md](docs/tools_lidar_processing.md) |
| Stream Network Analysis | [docs/tools_stream_network_analysis.md](docs/tools_stream_network_analysis.md) |
| Data Tools | [docs/tools_data_tools.md](docs/tools_data_tools.md) |


---

## Common Conventions

Examples in this document use `wbe` as a conventional variable name for a
`WbEnvironment` instance. You may use any instance name (`env`, `wb`, `my_env`,
etc.); method behavior is identical.

### Tool tiers

Tool availability depends on runtime options:

- `Tier = OSS`: available in the default runtime (`include_pro=False`, `tier='open'`).
- `Tier = Pro`: requires Pro runtime visibility (`include_pro=True`, `tier='pro'` or higher).

When using generic runtime entry points (`run_tool_*` / `list_tools_json_*`), make
sure `include_pro=True` is set to list and execute Pro-only tools.

### Output paths

Every tool that produces a raster accepts an optional `output_path` (or
`output_path=None`).  When omitted the result is stored in memory and returned
as a `Raster` object that can be passed directly into the next tool call without
a disk round-trip.  Specify a path to write the result to disk immediately.

```python
# In-memory intermediate (no disk write)
tmp = wbe.sqrt(dem)

# Write to disk at the same step
result = wbe.sqrt(dem, output_path='dem_sqrt.tif')
```

### Progress callbacks

Tools that accept a `callback` parameter will call it with a JSON string for
each progress or message event:

```python
import json

def make_progress_callback(step_percent: int = 1):
    """Return a callback that prints progress only every `step_percent` points."""
    last_bucket = -1

    def on_event(event_json: str) -> None:
        nonlocal last_bucket
        evt = json.loads(event_json)
        evt_type = evt.get('type')

        if evt_type == 'progress':
            pct = float(evt.get('percent', 0.0)) * 100.0
            bucket = int(pct // step_percent)
            if bucket > last_bucket:
                last_bucket = bucket
                # Clamp display to 100% in case of slight floating-point overshoot.
                shown = min(bucket * step_percent, 100)
                print(f"  {shown}%")
        elif evt_type == 'message':
            print(f"  {evt.get('message', '')}")

    return on_event


# Example: only print every 1% progress increment
on_event = make_progress_callback(step_percent=1)
```

For tools with very frequent progress events, a time-based throttle can be
useful as well:

```python
import json
import time

def make_timed_progress_callback(min_interval_sec: float = 0.5):
    """Return a callback that prints progress at most once per interval."""
    last_print = 0.0

    def on_event(event_json: str) -> None:
        nonlocal last_print
        evt = json.loads(event_json)
        evt_type = evt.get('type')

        if evt_type == 'progress':
            now = time.monotonic()
            if now - last_print >= min_interval_sec:
                last_print = now
                pct = float(evt.get('percent', 0.0)) * 100.0
                print(f"  {pct:.1f}%")
        elif evt_type == 'message':
            print(f"  {evt.get('message', '')}")

    return on_event


# Example: print at most once every 0.5 seconds
on_event = make_timed_progress_callback(min_interval_sec=0.5)
```

### band_mode

Unary and binary raster tools accept an optional `band_mode` parameter:

| Value | Behaviour |
|-------|-----------|
| `'all'` (default) | Process every band |
| `'active'` | Process only the current `active_band` |
| `'list'` | Process the bands listed in `bands=[0, 2, ...]` |

---

## Raster I/O

### Tools (Alphabetical)

- [`wbe.read_raster`](#wberead_raster)
- [`wbe.read_rasters`](#wberead_rasters)
- [`wbe.write_raster`](#wbewrite_raster)
- [`wbe.write_text`](#wbewrite_text)

### `wbe.read_raster`

```
read_raster(file_name: str) -> Raster
```

Reads a raster from disk.  The path is resolved against
`wbe.working_directory`.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `file_name` | `str` | required | Relative or absolute path to the raster file |

**Example**

```python
dem = wbe.read_raster('dem.tif')
```

---

### `wbe.read_rasters`

```
read_rasters(file_names: list[str], parallel: bool = True) -> list[Raster]
```

Reads multiple rasters, optionally in parallel.

**Example**

```python
rasters = wbe.read_rasters(['dem.tif', 'slope.tif'], parallel=True)
```

---

### `wbe.write_raster`

```
write_raster(raster: Raster, output_path: str, compress: bool = False, remove_after_write: bool = False) -> None
```

Writes a raster to disk.  Use to copy or rename an in-memory result to a final
location.  If the raster was already written during tool execution this step is
optional. When `remove_after_write=True`, a memory-backed raster is removed from
the in-process raster store after a successful write.

**Example**

```python
wbe.write_raster(result, 'final_output.tif', compress=True)
wbe.write_raster(result, 'final_output.tif', compress=True, remove_after_write=True)
```

---

### `wbe.remove_raster_from_memory`

```
remove_raster_from_memory(raster: Raster) -> bool
```

Removes a memory-backed raster from the in-process raster store. Returns `True`
if a stored raster was removed and `False` if the raster was not memory-backed
or was already absent.

**Example**

```python
wbe.remove_raster_from_memory(result)
```

---

### `wbe.clear_raster_memory`

```
clear_raster_memory() -> int
```

Clears the in-process raster memory store and returns the number of rasters
removed.

**Example**

```python
removed = wbe.clear_raster_memory()
```

---

### `wbe.raster_memory_count`

```
raster_memory_count() -> int
```

Returns the number of rasters currently held in the in-process raster store.

**Example**

```python
count = wbe.raster_memory_count()
```

---

### `wbe.raster_memory_bytes`

```
raster_memory_bytes() -> int
```

Returns an estimate of the heap bytes held by all rasters in the in-process raster store.
Only the typed cell-data buffer is counted; metadata and allocator overhead are excluded.

**Example**

```python
print(f"store holds {wbe.raster_memory_bytes() / 1_048_576:.1f} MiB")
```

---

### `wbe.write_text`

```
write_text(text: str, file_name: str) -> None
```

Writes a plain-text string to a file resolved against `wbe.working_directory`.

**Example**

```python
wbe.write_text('processing complete', 'run_log.txt')
```

---

## Unary Raster Math — wbe.* Methods

All unary tools share the same signature:

```
wbe.<tool>(input, output_path=None, callback=None, band_mode='all', bands=None)
    -> Raster
```

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |
| `band_mode` | `str` | `'all'` | `'all'`, `'active'`, or `'list'` |
| `bands` | `list[int] \| None` | `None` | Band indices when `band_mode='list'` |

### Available unary tools

| Method | Description |
|--------|-------------|
| `wbe.abs(input, ...)` | Absolute value |
| `wbe.ceil(input, ...)` | Ceiling (round up to nearest integer) |
| `wbe.floor(input, ...)` | Floor (round down to nearest integer) |
| `wbe.round(input, ...)` | Round to nearest integer |
| `wbe.sqrt(input, ...)` | Square root |
| `wbe.square(input, ...)` | Square (x²) |
| `wbe.ln(input, ...)` | Natural logarithm |
| `wbe.log10(input, ...)` | Base-10 logarithm |
| `wbe.sin(input, ...)` | Sine (radians) |
| `wbe.cos(input, ...)` | Cosine (radians) |

**Examples**

```python
dem_abs  = wbe.abs(dem)
dem_sqrt = wbe.sqrt(dem)
dem_ln   = wbe.ln(dem, output_path='dem_ln.tif')
dem_sin  = wbe.sin(dem)
dem_sq   = wbe.square(dem)
```

---

## Binary Raster Math — wbe.* Methods

All binary tools share the same signature:

```
wbe.<tool>(input1, input2, output_path=None, callback=None) -> Raster
```

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input1` | `Raster` | required | First input raster |
| `input2` | `Raster` | required | Second input raster |
| `output_path` | `str | None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable | None` | `None` | Progress/message event handler |

### Available binary tools

| Method | Alias | Description |
|--------|-------|-------------|
| `wbe.add(input1, input2, ...)` | | Element-wise addition |
| `wbe.subtract(input1, input2, ...)` | `wbe.sub` | Element-wise subtraction |
| `wbe.multiply(input1, input2, ...)` | `wbe.mul` | Element-wise multiplication |
| `wbe.divide(input1, input2, ...)` | `wbe.div` | Element-wise division |

**Examples**

```python
sum_r  = wbe.add(dem1, dem2)
diff_r = wbe.subtract(dem1, dem2, output_path='diff.tif')
prod_r = wbe.multiply(dem1, dem2)
quot_r = wbe.divide(dem1, dem2)
```

---

## Themed Tool Sets

For detailed listings of tool parameters and usage examples for each themed area,
see the dedicated reference documents linked in the [Table of Contents](#table-of-contents).

---

## Raster Object Methods

`Raster` objects support a rich set of in-memory operations.  All methods
that produce a new raster share the same optional kwargs:

```
raster.<method>(..., output_path=None, callback=None, band_mode='all', bands=None)
    -> Raster
```

---

### Unary Math — Raster Methods

```
raster.<method>(output_path=None, callback=None, band_mode='all', bands=None) -> Raster
```

| Method | Operator | Description |
|--------|----------|-------------|
| `abs(...)` | `abs()` built-in | Absolute value |
| `ceil(...)` | | Ceiling |
| `floor(...)` | | Floor |
| `round(...)` | | Round to nearest integer |
| `sqrt(...)` | | Square root |
| `square(...)` | | Square (x²) |
| `ln(...)` | | Natural logarithm |
| `log10(...)` | | Base-10 logarithm |
| `sin(...)` | | Sine (radians) |
| `cos(...)` | | Cosine (radians) |
| `neg(...)` | `-raster` | Negate all valid values |
| `is_finite(...)` | | 1.0 where finite, 0.0 otherwise |
| `is_infinite(...)` | | 1.0 where infinite |
| `is_nan(...)` | | 1.0 where NaN |

**Examples**

```python
dem_abs  = dem.abs()
dem_neg  = -dem
dem_sqrt = dem.sqrt(output_path='dem_sqrt.tif')
```

---

### Binary / Scalar Math — Raster Methods

```
raster.<method>(other, output_path=None, band_mode='all', bands=None) -> Raster
```

`other` may be another `Raster` or a scalar `float` / `int`.

| Method | Operator | Description |
|--------|----------|-------------|
| `add(other, ...)` | `+` | Addition |
| `subtract(other, ...)` | `-` | Subtraction |
| `multiply(other, ...)` | `*` | Multiplication |
| `divide(other, ...)` | `/` | Division |
| `pow(other, ...)` | `**` | Raise to a power (alias: `power`) |
| `max(other, ...)` | | Element-wise maximum |
| `min(other, ...)` | | Element-wise minimum |
| `atan2(other, ...)` | | Two-argument arctangent |
| `clamp(min_value, max_value, ...)` | | Clamp values to [min, max] |

**Examples**

```python
dem_sum   = dem + dem2
dem_plus5 = dem.add(5.0)
dem_pow2  = dem.pow(2.0)
dem_clamp = dem.clamp(0.0, 1000.0)
dem_max   = dem.max(0.0)

# Operator chaining stays memory-first
result = (dem + dem2) * 0.5
final  = result.divide(dem3, output_path='final.tif')
```

---

### Comparison and Logical — Raster Methods

```
raster.<method>(other, output_path=None, band_mode='all', bands=None) -> Raster
```

| Method | Operator | Description |
|--------|----------|-------------|
| `gt(other, ...)` | `>` | Greater than |
| `ge(other, ...)` | `>=` | Greater than or equal |
| `lt(other, ...)` | `<` | Less than |
| `le(other, ...)` | `<=` | Less than or equal |
| `eq(other, ...)` | `==` | Equal |
| `ne(other, ...)` | `!=` | Not equal |
| `logical_and(other, ...)` | `&` | Logical AND (alias: `and_`) |
| `logical_or(other, ...)` | `\|` | Logical OR (alias: `or_`) |
| `logical_xor(other, ...)` | `^` | Logical XOR (alias: `xor_`) |
| `logical_not(...)` | `~` | Logical NOT (alias: `not_`) |

---

### In-Place Mutation — Raster Methods

In-place methods modify the raster in memory and return `None`.

```
raster.<method>(other, band_mode='all', bands=None) -> None
```

| Method | Operator | Description |
|--------|----------|-------------|
| `add_in_place(other, ...)` | `+=` | Add |
| `sub_in_place(other, ...)` | `-=` | Subtract |
| `mul_in_place(other, ...)` | `*=` | Multiply |
| `div_in_place(other, ...)` | `/=` | Divide |
| `pow_in_place(other, ...)` | | Raise to a power |
| `eq_in_place(other, ...)` | | Equal (in-place predicate) |
| `ne_in_place(other, ...)` | | Not-equal |
| `gt_in_place(other, ...)` | | Greater-than |
| `ge_in_place(other, ...)` | | Greater-or-equal |
| `lt_in_place(other, ...)` | | Less-than |
| `le_in_place(other, ...)` | | Less-or-equal |
| `logical_and_in_place(other, ...)` | | Logical AND |
| `logical_or_in_place(other, ...)` | | Logical OR |
| `logical_xor_in_place(other, ...)` | | Logical XOR |
| `logical_not_in_place(...)` | | Logical NOT |

**Examples**

```python
dem += 5.0        # calls add_in_place via __iadd__
dem -= dem2
dem *= 0.5
```

---

### Utility — Raster Methods

#### Properties

| Property | Description |
|----------|-------------|
| `band_count` | Number of bands (read-only) |
| `active_band` | Current active band index (settable) |

#### Cell access

| Method | Description |
|--------|-------------|
| `get_value(row, col, band=None)` | Value at (row, col) |
| `set_value(row, col, value, band=None)` | Set value at (row, col) |
| `get_row_data(row, band=None)` | All column values for a row |
| `set_row_data(row, values, band=None)` | Set all column values for a row |
| `increment(row, col, value, band=None)` | Add value to cell |
| `decrement(row, col, value, band=None)` | Subtract value from cell |
| `increment_row_data(row, values, band=None)` | Increment a whole row |
| `decrement_row_data(row, values, band=None)` | Decrement a whole row |
| `is_cell_nodata(row, col, band=None)` | True if cell is nodata |
| `raster[row, col]` | Same as `get_value` |
| `raster[row, col] = v` | Same as `set_value` |

#### Coordinate helpers

| Method | Description |
|--------|-------------|
| `get_x_from_column(col)` | Easting for column centre |
| `get_y_from_row(row)` | Northing for row centre |
| `get_column_from_x(x)` | Column index for easting |
| `get_row_from_y(y)` | Row index for northing |

#### Statistics

| Method | Description |
|--------|-------------|
| `num_cells()` | Total cell count |
| `num_valid_cells()` | Count of non-nodata cells |
| `calculate_mean()` | Band mean |
| `calculate_mean_and_stdev()` | `(mean, stdev)` tuple |
| `calculate_clip_values(percent)` | `(low, high)` linear stretch clip |
| `update_min_max()` | Refresh cached statistics |

#### File info

| Method | Description |
|--------|-------------|
| `absolute_path()` | Absolute path string |
| `get_short_filename()` | File stem (no extension) |
| `get_file_extension()` | Extension without dot |
| `get_data_size_in_bytes()` | In-memory data size (alias: `size_of`) |
| `get_file_size_in_bytes()` | File size on disk |

#### CRS

| Method | Description |
|--------|-------------|
| `set_crs_wkt(wkt)` | Set CRS from WKT string |
| `set_crs_epsg(epsg)` | Set CRS from EPSG code |
| `clear_crs()` | Remove CRS information |

#### Reprojection (on Raster)

```
raster.reproject(
    dst_epsg: int,
    output_path=None,
    callback=None,
    resample='nearest',
    cols=None,
    rows=None,
    extent=None,
    x_res=None,
    y_res=None,
    snap_x=None,
    snap_y=None,
    nodata_policy='partial_kernel',
    antimeridian_policy='auto',
    grid_size_policy='expand',
    destination_footprint='none',
) -> Raster
```

Convenience shortcuts:

| Method | Resample | Description |
|--------|----------|-------------|
| `reproject_nearest(dst_epsg, output_path=None, callback=None)` | nearest | Alias for `reproject` with nearest-neighbour |
| `reproject_bilinear(dst_epsg, output_path=None, callback=None)` | bilinear | Alias with bilinear interpolation |
| `reproject_to_match_grid(template, output_path=None, callback=None)` | nearest | Match another raster's grid exactly |
| `reproject_to_match_resolution(template, output_path=None, callback=None)` | nearest | Match another raster's cell size |
| `reproject_to_match_resolution_in_epsg(dst_epsg, template, output_path=None, callback=None)` | nearest | Reproject to EPSG then match resolution |

---

## Vector I/O

### Tools

- [`wbe.read_vector`](#wberead_vector)
- [`wbe.read_vectors`](#wberead_vectors)
- [`wbe.write_vector`](#wbewrite_vector)

### `wbe.read_vector`

```
read_vector(file_name: str) -> Vector
```

Reads a vector dataset (Shapefile, GeoPackage, etc.) from disk.
The path is resolved against `wbe.working_directory`.

**Example**

```python
roads = wbe.read_vector('roads.gpkg')
```

---

### `wbe.read_vectors`

```
read_vectors(file_names: list[str], parallel: bool = True) -> list[Vector]
```

Reads multiple vector files.

**Example**

```python
layers = wbe.read_vectors(['roads.gpkg', 'buildings.shp'])
```

---

### `wbe.write_vector`

```
write_vector(vector: Vector, output_path: str) -> None
```

Writes a `Vector` object to disk.

**Example**

```python
wbe.write_vector(result_vector, 'output.gpkg')
```

---

## Lidar I/O

### Tools

- [`wbe.read_lidar`](#wberead_lidar)
- [`wbe.read_lidars`](#wberead_lidars)
- [`wbe.write_lidar`](#wbewrite_lidar)

### `wbe.read_lidar`

```
read_lidar(file_name: str, file_mode: str = 'r') -> Lidar
```

Reads a LiDAR point cloud (.las / .laz) from disk.
The path is resolved against `wbe.working_directory`.

**Example**

```python
cloud = wbe.read_lidar('pointcloud.laz')
```

---

### `wbe.read_lidars`

```
read_lidars(file_names: list[str], parallel: bool = True) -> list[Lidar]
```

Reads multiple LiDAR files.

**Example**

```python
clouds = wbe.read_lidars(['flight1.las', 'flight2.las'])
```

---

### `wbe.write_lidar`

```
write_lidar(lidar: Lidar, output_path: str) -> None
```

Writes a `Lidar` object to disk.

**Example**

```python
wbe.write_lidar(filtered_cloud, 'output.las')
```

---

## Reprojection

All reprojection functions accept an optional `callback` for progress events.

### `wbe.reproject_raster`

```
reproject_raster(
    input: Raster,
    dst_epsg: int,
    output_path: str | None = None,
    callback: callable | None = None,
    resample: str = 'nearest',
    cols: int | None = None,
    rows: int | None = None,
    extent: tuple[float, float, float, float] | None = None,
    x_res: float | None = None,
    y_res: float | None = None,
    snap_x: float | None = None,
    snap_y: float | None = None,
    nodata_policy: str = 'partial_kernel',
    antimeridian_policy: str = 'auto',
    grid_size_policy: str = 'expand',
    destination_footprint: str = 'none',
) -> Raster
```

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Source raster |
| `dst_epsg` | `int` | required | Target CRS as EPSG code |
| `output_path` | `str \| None` | `None` | Output path (memory if omitted) |
| `resample` | `str` | `'nearest'` | `'nearest'`, `'bilinear'`, `'cubic'`, `'average'` |
| `nodata_policy` | `str` | `'partial_kernel'` | `'partial_kernel'`, `'use_nodata'` |
| `antimeridian_policy` | `str` | `'auto'` | `'auto'`, `'keep'`, `'split'` |
| `grid_size_policy` | `str` | `'expand'` | `'expand'`, `'match_source'` |
| `destination_footprint` | `str` | `'none'` | `'none'`, `'source'` |

**Example**

```python
dem_wgs84 = wbe.reproject_raster(dem, dst_epsg=4326, resample='bilinear')
```

---

### `wbe.reproject_vector`

```
reproject_vector(
    input: Vector,
    dst_epsg: int,
    output_path: str | None = None,
    callback: callable | None = None,
    failure_policy: str = 'error',
    antimeridian_policy: str = 'keep',
    max_segment_length: float | None = None,
    topology_policy: str = 'none',
) -> Vector
```

**Example**

```python
roads_wgs84 = wbe.reproject_vector(roads, dst_epsg=4326)
```

---

### `wbe.reproject_lidar`

```
reproject_lidar(
    input: Lidar,
    dst_epsg: int,
    output_path: str | None = None,
    callback: callable | None = None,
    use_3d_transform: bool = False,
    failure_policy: str = 'error',
) -> Lidar
```

**Example**

```python
cloud_utm = wbe.reproject_lidar(cloud, dst_epsg=32617)
```

---

### `wbe.reproject_rasters`

```
reproject_rasters(
    inputs: list[Raster],
    dst_epsg: int,
    output_dir: str | None = None,
    callback: callable | None = None,
    resample: str = 'bilinear',
    nodata_policy: str = 'use_nodata',
    antimeridian_policy: str = 'keep',
    grid_size_policy: str = 'match_source',
    destination_footprint: str = 'source',
) -> list[Raster]
```

Batch-reprojects multiple rasters.  Each output is written to `output_dir`
(defaults to `wbe.working_directory`) with `_epsg{dst_epsg}` appended to the
stem.

---

### `wbe.reproject_vectors`

```
reproject_vectors(
    inputs: list[Vector],
    dst_epsg: int,
    output_dir: str | None = None,
    callback: callable | None = None,
    failure_policy: str = 'error',
    antimeridian_policy: str = 'keep',
    max_segment_length: float | None = None,
    topology_policy: str = 'none',
) -> list[Vector]
```

---

### `wbe.reproject_lidars`

```
reproject_lidars(
    inputs: list[Lidar],
    dst_epsg: int,
    output_dir: str | None = None,
    callback: callable | None = None,
    use_3d_transform: bool = False,
    failure_policy: str = 'error',
) -> list[Lidar]
```

---
