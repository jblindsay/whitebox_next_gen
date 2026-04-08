# Whitebox Workflows for Python

Whitebox Workflows for Python is the Python interface for the Whitebox backend runtime.

The API is in active modernization, with emphasis on:
- clearer data-object ergonomics,
- better discoverability and IntelliSense,
- memory-first workflows,
- stronger interoperability with Python data tooling.

## Current API highlights

- Harmonized metadata access:
  - `Raster.metadata()`
  - `Vector.metadata()`
  - `Lidar.metadata()`
- Backward compatibility remains for `Raster.configs()`.
- Vector attribute readability aliases:
  - `schema()`, `attributes()`, `attribute()`
  - `update_attributes()`, `update_attribute()`, `add_field()`
- Dataset-aware vector write/copy for multifile formats.
- Raster and NumPy bridge:
  - `Raster.to_numpy(...)`
  - `Raster.from_numpy(...)`

## Migration quick map

Common updates from legacy style to the harmonized API:

| Legacy style | Current style |
|---|---|
| `r.configs()` | `r.metadata()` |
| `v.attribute_fields()` | `v.schema()` |
| `v.get_attributes(i)` | `v.attributes(i)` |
| `v.get_attribute(i, field)` | `v.attribute(i, field)` |
| `v.set_attributes(i, values)` | `v.update_attributes(i, values)` |
| `v.set_attribute(i, field, value)` | `v.update_attribute(i, field, value)` |
| `v.add_attribute_field(...)` | `v.add_field(...)` |

Notes:
- Legacy method names remain available for compatibility.
- New aliases are intended to improve readability and consistency across object types.

## Tool reference docs

- [TOOLS.md](TOOLS.md)
- [docs/tools_hydrology.md](docs/tools_hydrology.md)
- [docs/tools_gis.md](docs/tools_gis.md)
- [docs/tools_remote_sensing.md](docs/tools_remote_sensing.md)
- [docs/tools_geomorphometry.md](docs/tools_geomorphometry.md)
- [docs/tools_agriculture.md](docs/tools_agriculture.md)
- [docs/tools_lidar_processing.md](docs/tools_lidar_processing.md)
- [docs/tools_stream_network_analysis.md](docs/tools_stream_network_analysis.md)

Design and migration notes:
- [docs/v2_migration_guide.md](docs/v2_migration_guide.md)
- [docs/data_object_api_harmonization_proposal.md](docs/data_object_api_harmonization_proposal.md)
- [docs/api_redesign_rfc_draft.md](docs/api_redesign_rfc_draft.md)

## Development install

From workspace root:

```bash
./scripts/dev_python_install.sh
```

This performs an editable install via maturin for the wbw_python crate.

## Quick smoke test

```bash
python3 crates/wbw_python/examples/python_import_smoke_test.py
```

## Recommended examples

**Suggested run order for new users:**

| Order | Script | Focus |
|---|---|---|
| 1 | [examples/quickstart_harmonized_api.py](examples/quickstart_harmonized_api.py) | Raster/vector/lidar metadata quickstart |
| 2 | [examples/current_api_data_handling_demo.py](examples/current_api_data_handling_demo.py) | End-to-end object read/process/write |
| 3 | [examples/vector_attributes_harmonized_api.py](examples/vector_attributes_harmonized_api.py) | Vector schema + attribute access aliases |
| 4 | [examples/vector_multifile_write_demo.py](examples/vector_multifile_write_demo.py) | Shapefile/MapInfo dataset-aware outputs |
| 5 | [examples/raster_numpy_roundtrip.py](examples/raster_numpy_roundtrip.py) | 2D NumPy roundtrip |
| 6 | [examples/raster_numpy_multiband_roundtrip.py](examples/raster_numpy_multiband_roundtrip.py) | 3D NumPy roundtrip (bands-first and rows-cols-bands) |
| — | [examples/licensing_offline_example.py](examples/licensing_offline_example.py) | Offline signed entitlement startup |
| — | [examples/licensing_floating_online_example.py](examples/licensing_floating_online_example.py) | Floating license startup |

Run from this directory:

```bash
python examples/quickstart_harmonized_api.py
```

## Recommended API pattern

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
wbe.working_directory = '/path/to/data'

dem = wbe.read_raster('dem.tif')
meta = dem.metadata()
print(meta.rows, meta.columns, meta.nodata)

filled = wbe.hydrology.fill_depressions(dem)
accum = wbe.hydrology.d8_flow_accum(filled)
wbe.write_raster(accum, 'flow_accum.tif')
```

## Memory-first execution model

Default behavior is memory-first for intermediates:

- If an output path is omitted, operations return memory-backed objects.
- Persist only when needed with `write_raster`, `write_vector`, or `write_lidar`.
- This reduces unnecessary disk I/O in chained workflows.

Example:

```python
tmp = wbe.raster_tools.sqrt(dem)
out = wbe.raster_tools.log10(tmp)
wbe.write_raster(out, 'sqrt_log10.tif')
```

### ArcPy-style I/O vs wbw_python memory-first chaining

ArcPy workflows often write each intermediate to disk as `input -> tool -> output`,
which can increase I/O overhead in multi-step pipelines.

In wbw_python, intermediate objects are memory-backed unless an explicit output path
is provided. This means you can chain operations in memory and persist only final
artifacts, reducing unnecessary disk writes.

## Reprojection patterns

```python
dst_epsg = 32618

dem_utm = wbe.reproject_raster(dem, dst_epsg=dst_epsg)
roads = wbe.read_vector('roads.shp')
roads_utm = wbe.reproject_vector(roads, dst_epsg=dst_epsg)

wbe.write_raster(dem_utm, 'dem_utm.tif')
wbe.write_vector(roads_utm, 'roads_utm.shp')
```

### Reprojection best practices

- **Preserve precision**: Use high-precision resampling (`'bilinear'` or `'cubic'`) for continuous data; `'nearest'` for categorical.
- **Verify CRS**: Always inspect `crs_epsg()` before and after reprojection to confirm the transform.
- **CRS mismatch**: If input CRS is unknown or incorrect, call `set_crs_epsg()` before reprojection.
- **Memory-first chaining**: Reprojection returns memory-backed objects; persist with `write_raster` or `write_vector`.
- **Coordinate order**: EPSG defines lat/lon order; Whitebox always uses lon/lat internal. Transforms are applied automatically.

## NumPy interoperability

```python
arr = dem.to_numpy(dtype='float64')
arr = arr + 1.0
new_dem = wb.Raster.from_numpy(arr, dem, output_path='dem_plus1.tif')
```

Multiband workflows support both `(bands, rows, cols)` and `(rows, cols, bands)` 3D arrays.

## Licensing overview

The runtime supports open and licensed modes.

- Open mode: instantiate `WbEnvironment()` directly.
- Signed entitlement mode: bootstrap from signed JSON or file.
- Floating license mode: bootstrap with floating id plus provider URL.

See:
- [examples/licensing_offline_example.py](examples/licensing_offline_example.py)
- [examples/licensing_floating_online_example.py](examples/licensing_floating_online_example.py)

## Discovery APIs

```python
tools = wbe.list_tools()
categories = wbe.categories()
info = wbe.describe_tool('slope')
matches = wbe.search_tools('flow accumulation')
```

## IntelliSense in VS Code

If completions are stale, ensure VS Code uses the same interpreter where wbw_python is installed.

1. Run Python: Select Interpreter.
2. Select your .venv-wbw interpreter.
3. Reload window.
4. Restart language server.

Optional workspace pin in .vscode/settings.json:

```json
{
  "python.defaultInterpreterPath": "/Users/<you>/Documents/programming/Rust/whitebox_next_gen/.venv-wbw/bin/python"
}
```

If needed, remove legacy global path overrides such as:
- `python.analysis.extraPaths`
- `python.autoComplete.extraPaths`
