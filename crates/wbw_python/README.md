# Whitebox Workflows for Python

**THIS CRATE IS CURRENTLY EXPERIMENTAL AND IS IN AN EARLY DEVELOPMENTAL STAGE. IT IS NOT INTENDED FOR PUBLIC USAGE AT PRESENT.**

Whitebox Workflows for Python is the Python interface for the Whitebox backend runtime.

> **Tool Reference:** For a complete listing of all available tools, their parameters, and
> usage examples see **[TOOLS.md](TOOLS.md)** (API conventions, I/O, and math operators)
> and the themed reference documents below.
>
> | Theme | Reference document |
> |---|---|
> | Hydrology | [docs/tools_hydrology.md](docs/tools_hydrology.md) |
> | GIS | [docs/tools_gis.md](docs/tools_gis.md) |
> | Remote Sensing | [docs/tools_remote_sensing.md](docs/tools_remote_sensing.md) |
> | Geomorphometry | [docs/tools_geomorphometry.md](docs/tools_geomorphometry.md) |
> | Precision Agriculture | [docs/tools_agriculture.md](docs/tools_agriculture.md) |
> | LiDAR Processing | [docs/tools_lidar_processing.md](docs/tools_lidar_processing.md) |
> | Stream Network Analysis | [docs/tools_stream_network_analysis.md](docs/tools_stream_network_analysis.md) |

## Development install

From the workspace root:

```bash
./scripts/dev_python_install.sh
```

This runs a local editable install using maturin against
`crates/wbw_python/Cargo.toml`.

## Quick smoke test

```bash
python3 crates/wbw_python/examples/python_import_smoke_test.py
```

## Licensing and startup behavior

The Python runtime supports three licensing modes:

- Tier-only mode (existing behavior):
    - `RuntimeSession(include_pro=..., tier="open|pro|enterprise")`
    - `list_tools_json_with_options(include_pro=..., tier=...)`
    - `run_tool_json_with_options(...)`
- Signed-entitlement mode (explicit envelope input):
    - `RuntimeSession.from_signed_entitlement_json(...)`
    - `WbEnvironment.from_signed_entitlement_json(...)`
    - `list_tools_json_with_entitlement_options(...)`
    - `run_tool_json_with_entitlement_options(...)`
- Floating-id convenience mode (legacy-style ergonomics):
    - `RuntimeSession.from_floating_license_id(...)`
    - `WbEnvironment.from_floating_license_id(...)`
    - `whitebox_tools(floating_license_id=...)`
- Provider bootstrap mode (new):
    - Automatically used by `new_with_options(include_pro=True, ...)` paths when
        `WBW_LICENSE_PROVIDER_URL` is set in the environment.

### Teaching/lab/notebook environments (no admin privileges)

Floating-id startup no longer requires setting system environment variables.
You can pass the floating license id and provider URL directly in Python code,
which works well in locked-down computing labs, Jupyter sessions, and hosted
Python notebook environments where users cannot set persistent machine env vars.

### Provider bootstrap policy (Pro builds)

When `include_pro=True` and `WBW_LICENSE_PROVIDER_URL` is configured, startup
attempts to bootstrap from the provider:

1. Load local license state from disk.
2. Refresh public keys from provider.
3. Re-verify cached entitlement.
4. Attempt lease refresh/acquire.
5. Resolve entitled capabilities, or fallback tier by policy.

Policy is controlled by `WBW_LICENSE_POLICY`:

- `fail_open` (default): if provider is unavailable or no valid entitlement is
    found, startup falls back to tier runtime using the configured fallback tier
    (commonly `open`).
- `fail_closed`: startup returns a license-denied error if provider bootstrap
    cannot establish a valid entitlement path.

If the local persistence file does not exist, startup handles this as a
first-run condition and continues according to policy (not a crash path).

### Licensing environment variables

| Variable | Purpose |
|---|---|
| `WBW_LICENSE_PROVIDER_URL` | Enables provider bootstrap mode when set. |
| `WBW_LICENSE_POLICY` | `fail_open` (default) or `fail_closed`. |
| `WBW_LICENSE_LEASE_SECONDS` | Optional lease target duration in seconds. |
| `WBW_LICENSE_STATE_PATH` | Optional explicit local license-state file path. |

### Floating license behavior (current status)

"Floating license ID only" activation on a brand-new machine is now supported
when provider bootstrap is enabled and the provider exposes the floating
activation endpoint.

Current implementation supports:

- Lease renew/acquire after a valid entitlement is already present in local
    state (or explicitly supplied via entitlement APIs).
- Graceful first-run behavior when no persistence file exists (policy-driven
    fail-open/fail-closed).
- New-machine online activation by exchanging floating license id + machine id
    for a signed entitlement during bootstrap.

Requirements for floating activation:

- `WBW_LICENSE_PROVIDER_URL` must be configured.
- `WBW_FLOATING_LICENSE_ID` must be configured.
- Provider must expose `POST /api/v2/entitlements/activate-floating`.

### Operational runbook (online-first)

#### Existing machine (has local state)

1. Configure provider URL and policy env vars.
2. Start runtime with `include_pro=True`.
3. Runtime loads local state, re-verifies entitlement, then renews/acquires
     lease as needed.

#### New machine (no local state file)

1. Configure provider URL and policy env vars.
2. Provide `WBW_FLOATING_LICENSE_ID` (and optional machine/customer id vars).
3. Runtime attempts floating activation, persists the signed entitlement, then
    proceeds with lease lifecycle.
4. If activation/bootstrap fails, runtime follows policy:
    - `fail_open`: fall back to configured tier (typically `open`)
    - `fail_closed`: return license-denied

### Troubleshooting

| Symptom | Likely cause | Current behavior | Action |
|---|---|---|---|
| Pro tools not visible on a new machine | Floating activation failed (missing ID, unauthorized ID, provider unavailable) | Falls back to `open` in `fail_open`; denied in `fail_closed` | Set `WBW_FLOATING_LICENSE_ID`, verify provider endpoint and floating-ID allow-list |
| Startup returns license-denied | `WBW_LICENSE_POLICY=fail_closed` and bootstrap cannot establish valid entitlement | Constructor fails | Switch to `fail_open` for OSS fallback or provide valid entitlement state |
| Startup cannot reach provider | Network/provider outage or bad URL | `fail_open` fallback or `fail_closed` error | Check `WBW_LICENSE_PROVIDER_URL`, connectivity, and server health |
| Invalid signature / unknown key id | Key rotation mismatch or bad key config | Entitlement verification fails | Refresh provider public keys, verify `kid` and signing key alignment |

Additional floating activation env vars:

- `WBW_FLOATING_LICENSE_ID` (required for floating-ID activation)
- `WBW_MACHINE_ID` (optional override; defaults to hostname when available)
- `WBW_CUSTOMER_ID` (optional customer id hint)

### Script example: floating license id (online, new machine)

```python
import json
import os
import whitebox_workflows

session = whitebox_workflows.RuntimeSession.from_floating_license_id(
    floating_license_id="FLOAT-ABC-123",
    include_pro=True,
    fallback_tier="open",
    provider_url="https://your-provider.example.com",
)

os.environ.setdefault("WBW_LICENSE_POLICY", "fail_open")
os.environ.setdefault("WBW_LICENSE_LEASE_SECONDS", "3600")

tools = json.loads(session.list_tools_json())
print(f"Visible tools: {len(tools)}")
print("raster_power visible:", any(t.get("id") == "raster_power" for t in tools))
```

Equivalent `WbEnvironment` pattern:

```python
import whitebox_workflows

wbe = whitebox_workflows.WbEnvironment.from_floating_license_id(
    floating_license_id="FLOAT-ABC-123",
    include_pro=True,
    fallback_tier="open",
    provider_url="https://your-provider.example.com",
)
```

Legacy-style compatibility helper:

```python
import whitebox_workflows

wbe = whitebox_workflows.whitebox_tools("FLOAT-ABC-123")
# Optional explicit settings:
# wbe = whitebox_workflows.whitebox_tools(
#     floating_license_id="FLOAT-ABC-123",
#     include_pro=True,
#     tier="open",
#     provider_url="https://your-provider.example.com",
#     machine_id="lab-station-12",
# )
```

Full script: [examples/licensing_floating_online_example.py](examples/licensing_floating_online_example.py)

### Script examples: offline mode

#### A) Offline OSS fallback (no provider)

```python
import json
import os
import whitebox_workflows

os.environ.pop("WBW_LICENSE_PROVIDER_URL", None)
os.environ.pop("WBW_FLOATING_LICENSE_ID", None)

session = whitebox_workflows.RuntimeSession(include_pro=False, tier="open")
tools = json.loads(session.list_tools_json())
print(f"Offline OSS tools: {len(tools)}")
```

#### B) Offline signed-entitlement mode (no provider call)

```python
import json
import whitebox_workflows

with open("signed_entitlement.json", "r", encoding="utf-8") as f:
    signed_entitlement_json = f.read()

session = whitebox_workflows.RuntimeSession.from_signed_entitlement_json(
    signed_entitlement_json=signed_entitlement_json,
    public_key_kid="k1",
    public_key_b64url="REPLACE_WITH_PROVIDER_PUBLIC_KEY",
    include_pro=True,
    fallback_tier="open",
)

tools = json.loads(session.list_tools_json())
print(f"Offline entitlement tools: {len(tools)}")
```

Full script set: [examples/licensing_offline_example.py](examples/licensing_offline_example.py)

---

## Recommended API: WbEnvironment

The primary API follows the Whitebox Workflows style: object-based data access
plus environment-level processing methods.

---

### Setting Up the Whitebox Environment

```python
import whitebox_workflows

wbe = whitebox_workflows.WbEnvironment()
wbe.working_directory = '/path/to/data'
wbe.verbose = True
wbe.max_procs = -1
print(wbe.version())
print(wbe.license_type())
print(wbe.license_info())
```

---

### Working With Raster Data

#### General Raster I/O And Processing

```python
import whitebox_workflows
import json

wbe = whitebox_workflows.WbEnvironment()
wbe.working_directory = '/path/to/data'

# --- Reading ---
dem = wbe.read_raster('dem.tif')
rasters = wbe.read_rasters(['dem.tif', 'slope.tif'], parallel=True)

# --- Metadata ---
print(dem.get_short_filename())
print(dem.get_file_extension())
print(dem.num_cells())
print(dem.num_valid_cells())
print(dem.calculate_mean_and_stdev())

cfg = dem.configs()
print(cfg.rows, cfg.columns, cfg.minimum, cfg.maximum)
print(dem.get_data_size_in_bytes(), dem.size_of())

# --- Multiband support ---
# dem.band_count, dem.active_band, dem.band(i)
print(dem.band_count)
dem.active_band = 1
z_b1 = dem.get_value(10, 20)          # uses active_band
z_b2 = dem.get_value(10, 20, band=2)  # explicit band override
red = dem.band(0)
nir = dem.band(3)

# --- Cell and row access ---
z = dem.get_value(10, 20)
dem.set_value(10, 20, z + 1.0)
z2 = dem[10, 20]
dem[10, 20] = z2
row_vals = dem.get_row_data(10)
dem.increment(10, 20, 1.0)
dem.decrement(10, 20, 1.0)
dem.increment_row_data(10, [0.0] * len(row_vals))
print(dem.get_x_from_column(20), dem.get_y_from_row(10))
print(dem.get_column_from_x(500000.0), dem.get_row_from_y(4800000.0))
print(dem.calculate_clip_values(1.0))
dem.update_min_max()

# --- Copy / construct from existing ---
dem_copy = dem.deep_copy()
dem_blank = whitebox_workflows.Raster.new_from_other(dem, data_type='f32')
dem_clone = whitebox_workflows.Raster.new_from_other_with_data(dem, data_type='f64')

# --- Progress callback ---
def on_event(event_json: str) -> None:
    evt = json.loads(event_json)
    if evt.get('type') == 'progress':
        print(f"Progress: {float(evt.get('percent', 0.0)) * 100.0:.1f}%")
    elif evt.get('type') == 'message':
        print(evt.get('message', ''))

# --- Tool calls on the environment ---
# See TOOLS.md for a complete reference of every tool and its parameters.
result      = wbe.sqrt(dem, 'output.tif')
result2     = wbe.ln(dem)                         # auto-generated output path
result_cb   = wbe.sqrt(dem, 'output_cb.tif', callback=on_event)

# --- Binary raster tools on the environment ---
dem_sum   = wbe.add(dem, dem_copy, 'dem_sum.tif')
dem_diff  = wbe.subtract(dem, dem_copy)
dem_prod  = wbe.multiply(dem, dem_copy)
dem_ratio = wbe.divide(dem, dem_copy)

# Alias forms
dem_diff_alias  = wbe.sub(dem, dem_copy)
dem_prod_alias  = wbe.mul(dem, dem_copy)
dem_ratio_alias = wbe.div(dem, dem_copy)

# Memory-first chaining: omit output_path for in-memory intermediate results
tmp = wbe.add(dem, dem_copy)
dem_sum2 = wbe.add(tmp, dem, 'dem_sum2.tif')

tmp_prod = wbe.multiply(dem, dem_copy)
tmp_diff = wbe.subtract(tmp_prod, dem)
dem_chain = wbe.divide(tmp_diff, dem_copy, 'dem_chain.tif')

# Native Python operators on Raster objects
sum_op = dem + dem_copy
diff_op = dem - dem_copy
prod_op = dem * dem_copy
ratio_op = dem / dem_copy

# Raster-to-raster operators are memory-first by default; persist explicitly when needed
final_op = (sum_op - dem).div(dem_copy, output_path='final_op.tif')

# Augmented-assignment operators also work and stay memory-first by default
work = dem
work += dem_copy
work -= dem
work *= dem_copy
work /= dem_copy
persistent_work = work.add(dem, output_path='persistent_work.tif')

# band_mode: 'all' (default), 'active', 'list'
result_active  = wbe.sqrt(dem, 'sqrt_active.tif',  band_mode='active', callback=on_event)
result_bands   = wbe.sqrt(dem, 'sqrt_bands.tif',   band_mode='list', bands=[0, 3], callback=on_event)

# --- Unary math methods on Raster objects (selected examples) ---
# See TOOLS.md for the full list.
result3 = dem.sqrt()
result4 = abs(dem)
result5 = dem.sqrt('output_raster_cb.tif', callback=on_event)

# --- Writing ---
# Use write_raster to copy/rename outputs to a final location.
# If output paths were supplied during processing this step is optional.
# Set remove_after_write=True to release a memory-backed raster after saving.
wbe.write_raster(result, 'final_output.tif', compress=True)
wbe.write_raster(result, 'final_output.tif', compress=True, remove_after_write=True)
wbe.remove_raster_from_memory(result)
wbe.clear_raster_memory()
wbe.write_text('run complete', 'run_log.txt')
```

#### Working With Sensor Bundles

```python
import whitebox_workflows

wbe = whitebox_workflows.WbEnvironment()
wbe.working_directory = '/path/to/data'

# Unified detection returns a read-only Bundle with family + resolved root.
bundle = wbe.read_bundle('/data/LC09_L2SP_018030_20240202')
print(bundle.family)      # e.g. landsat, sentinel2_safe, sentinel1_safe, planetscope
print(bundle.bundle_root) # directory or extracted archive root

# Family-specific wrappers validate expected type.
landsat = wbe.read_landsat('/data/LC09_L2SP_018030_20240202')
s2 = wbe.read_sentinel2('/data/S2A_MSIL2A_20260401T000000.SAFE')
s1 = wbe.read_sentinel1('/data/S1A_IW_GRDH_20260401T000000.SAFE')
planetscope = wbe.read_planetscope('/data/PSScene_bundle')
iceye = wbe.read_iceye('/data/ICEYE_bundle')
dimap = wbe.read_dimap('/data/SPOT_or_Pleiades_DIMAP')
maxar = wbe.read_maxar_worldview('/data/Maxar_WorldView_bundle')
radarsat2 = wbe.read_radarsat2('/data/RADARSAT2_bundle')
rcm = wbe.read_rcm('/data/RCM_bundle')

# Capability discovery + asset reads.
print(landsat.list_band_keys())
print(s2.list_qa_keys())
print(s2.list_aux_keys())
print(s1.list_measurement_keys())
print(iceye.list_asset_keys())

b4 = landsat.read_band('B4')
scl = s2.read_qa_layer('SCL')
vv = s1.read_measurement('IW_GRD_VV')
iceye_vv = iceye.read_asset('VV')
```

Bundle metadata and capabilities exposed in Python:
- Common identity:
  - `bundle.family`, `bundle.bundle_root`, `bundle.__repr__()`.
- Full parsed metadata snapshot:
  - `metadata_json()` returns a pretty-printed JSON object with family-specific fields.
- Common typed metadata helpers (family-dependent; returns `None` when unavailable):
  - `acquisition_datetime_utc()`, `product_type()`, `acquisition_mode()`, `processing_level()`.
  - `mission()`, `scene_id()`, `tile_id()`, `collection_number()`, `processing_baseline()`.
  - `cloud_cover_percent()`, `sun_azimuth_deg()`, `sun_elevation_deg()`, `sun_zenith_deg()`.
  - `view_angle_deg()`, `off_nadir_angle_deg()`.
  - `polarization()`, `orbit_direction()`, `look_direction()`.
  - `incidence_angle_near_deg()`, `incidence_angle_far_deg()`.
  - `pixel_spacing_range_m()`, `pixel_spacing_azimuth_m()`.
  - `path_row()`, `spatial_bounds()`.
- Vector-valued metadata helpers:
  - `polarizations()` returns `list[str]` (empty list when unavailable).
- Optical bundle assets:
  - `list_band_keys()`, `read_band(key)`.
- QA/aux assets (family-dependent):
  - `list_qa_keys()`, `read_qa_layer(key)`.
  - `list_aux_keys()`, `read_aux_layer(key)`.
- SAR measurement assets (family-dependent):
  - `list_measurement_keys()`, `read_measurement(key)`.
- ICEYE assets:
  - `list_asset_keys()`, `read_asset(key)`.

Notes:
- Bundle methods are read-only and do not take a `file_mode` parameter.
- Sentinel-2 exposes solar zenith via `sun_zenith_deg()` (not `sun_elevation_deg()`).
- `spatial_bounds()` currently returns `[minx, miny, maxx, maxy]` for Sentinel-1 SAFE bundles.

---

### Working With Vector Data

```python
import whitebox_workflows

wbe = whitebox_workflows.WbEnvironment()
wbe.working_directory = '/path/to/data'

# --- Reading ---
roads = wbe.read_vector('roads.shp')

# --- Metadata ---
print(roads.get_short_filename(), roads.get_file_extension(), roads.exists())
print(roads.absolute_path(), roads.parent_directory())
print(roads.feature_count())

# --- Attribute table schema ---
print(roads.attribute_field_names())
print(roads.attribute_fields())

# --- Attribute reads ---
attrs0 = roads.get_attributes(0)   # dict for one feature
name0 = roads.get_attribute(0, 'NAME')

# --- Attribute edits (persisted to the vector dataset) ---
roads.set_attribute(0, 'NAME', 'Main Street')
roads.set_attributes(1, {'NAME': 'Second Street'})
roads.add_attribute_field('priority', 'integer', True, 0, 0, 1)
roads.set_attribute(0, 'priority', 3)

# --- Copy ---
roads_copy = roads.deep_copy()

# --- Writing ---
wbe.write_vector(roads, 'roads_copy.shp')
```

Attribute API notes:
- Supported field types for `add_attribute_field(...)`: `integer`, `float`, `text`, `boolean`, `blob`, `date`, `datetime`, `json`.
- `get_attribute` and `get_attributes` return Python-native values (`int`, `float`, `str`, `bool`, `bytes`, or `None`).
- `set_attribute` and `set_attributes` enforce target field types and write changes back to the source dataset path.

---

### Working With Lidar Data

```python
import whitebox_workflows

wbe = whitebox_workflows.WbEnvironment()
wbe.working_directory = '/path/to/data'

# --- Reading ---
tile      = wbe.read_lidar('tile1.laz')
all_tiles = wbe.read_lidars(['tile1.laz', 'tile2.laz'], parallel=True)

# --- Metadata ---
print(tile.get_short_filename(), tile.get_file_extension(), tile.exists())
print(tile.get_file_size_in_bytes(), tile.get_last_modified_unix_seconds())

# --- Copy ---
tile_copy = tile.deep_copy()

# --- Writing ---
wbe.write_lidar(tile, 'tile1_copy.laz')
```

---

### Working With Projections

```python
import whitebox_workflows
import json

wbe = whitebox_workflows.WbEnvironment()
wbe.working_directory = '/path/to/data'

dem   = wbe.read_raster('dem.tif')
roads = wbe.read_vector('roads.shp')
tile  = wbe.read_lidar('tile1.laz')

def on_event(event_json: str) -> None:
    evt = json.loads(event_json)
    if evt.get('type') == 'progress':
        print(f"Progress: {float(evt.get('percent', 0.0)) * 100.0:.1f}%")
    elif evt.get('type') == 'message':
        print(evt.get('message', ''))

# Reprojection callbacks stream live progress for raster, vector, and lidar
# object methods. Batch environment wrappers report overall per-item progress.

# --- Reading / writing CRS metadata ---
print(dem.crs_epsg())           # e.g. 4326 or None
print(dem.crs_wkt())            # WKT string or None
dem.set_crs_epsg(32618)
dem.set_crs_wkt('GEOGCS["WGS 84",AUTHORITY["EPSG","4326"]]')
print(dem.crs_epsg(strict=True))
dem.clear_crs()

# Vector and Lidar CRS stored in a .prj sidecar
roads.set_crs_epsg(4326)
print(roads.crs_wkt())
print(roads.crs_epsg())
roads.clear_crs()

tile.set_crs_wkt('GEOGCS["WGS 84",AUTHORITY["EPSG","4326"]]')
print(tile.crs_epsg(strict=False))

# --- Raster reprojection ---
dem_utm   = dem.reproject(32618, 'dem_utm.tif', resample='bilinear')
dem_wgs84 = dem.reproject_nearest(4326)
dem_utm_cb = dem.reproject(32618, 'dem_utm_cb.tif', callback=on_event)

# Detailed output grid control
dem_regrid = dem.reproject(
    32618,
    'dem_regrid.tif',
    resample='cubic',
    x_res=10.0,
    y_res=10.0,
    nodata_policy='partial_kernel',
    grid_size_policy='expand',
    destination_footprint='source_boundary',
)

# Match another raster's grid or resolution
dem_copy = dem.deep_copy()
aligned         = dem.reproject_to_match_grid(dem_copy, 'dem_aligned.tif', resample='nearest')
aligned_res     = dem.reproject_to_match_resolution(dem_copy, 'dem_aligned_res.tif', resample='bilinear')
aligned_res_utm = dem.reproject_to_match_resolution_in_epsg(
    3857, dem_copy, 'dem_aligned_3857.tif', resample='bilinear'
)

# --- Vector reprojection ---
roads_3857 = roads.reproject(
    3857,
    'roads_3857.gpkg',
    callback=on_event,
    failure_policy='error',
    antimeridian_policy='keep',
    topology_policy='validate',
)

# --- Lidar reprojection ---
tile_3857 = tile.reproject(
    3857,
    'tile_3857.laz',
    callback=on_event,
    use_3d_transform=True,
    failure_policy='set_nan',
)

# --- Environment wrappers (resolves output paths against wbe.working_directory) ---
utm_via_env   = wbe.reproject_raster(dem, 32618, 'dem_utm_env.tif', callback=on_event, resample='bilinear')
roads_via_env = wbe.reproject_vector(roads, 3857, 'roads_3857_env.gpkg', callback=on_event)
tile_via_env  = wbe.reproject_lidar(tile, 3857, 'tile_3857_env.laz', callback=on_event, use_3d_transform=True)

# --- Batch reprojection ---
# Each item emits a message event at start and a progress event on completion.
# Progress events include 'percent' (overall 0→1), 'item' (0-based), and 'count'.
dem2 = wbe.read_raster('dem2.tif')
dem3 = wbe.read_raster('dem3.tif')
reprojected_dems = wbe.reproject_rasters(
    [dem, dem2, dem3], dst_epsg=32618, output_dir='reprojected/', callback=on_event
)

roads2 = wbe.read_vector('roads2.shp')
roads3 = wbe.read_vector('roads3.shp')
reprojected_roads = wbe.reproject_vectors(
    [roads, roads2, roads3], dst_epsg=3857, output_dir='reprojected/', callback=on_event
)

tile2 = wbe.read_lidar('tile2.las')
reprojected_tiles = wbe.reproject_lidars(
    [tile, tile2], dst_epsg=3857, output_dir='reprojected/', callback=on_event, use_3d_transform=True
)
```

---

### Examples

Run unary tools from the command line:
```bash
python3 crates/wbw_python/examples/wbenvironment_example.py \
    /path/to/input.tif \
    /path/to/output_dir \
    --tools abs,sqrt,ln \
    --verbose
```

---

## Alternative: Direct Function API

If you prefer a more minimal approach:

```python
import whitebox_workflows

# Call tools directly as module functions with Raster objects
r = whitebox_workflows.Raster("input.tif")
whitebox_workflows.abs(r, "output_abs.tif")
whitebox_workflows.sqrt(r, "output_sqrt.tif")
whitebox_workflows.ln(r, "output_ln.tif")
# Optional output path
auto_result = whitebox_workflows.sqrt(r)

# With optional progress callback
def on_progress(event_json: str) -> None:
    import json
    event = json.loads(event_json)
    if event.get("type") == "progress":
        pct = float(event.get("percent", 0.0)) * 100.0
        print(f"Progress: {pct:.1f}%")

whitebox_workflows.sqrt(r, "output.tif", callback=on_progress)
```

Available tools:
- `abs`, `ceil`, `floor`, `round`
- `sqrt`, `square`
- `ln`, `log10`
- `sin`, `cos`

See [TOOLS.md](TOOLS.md) for the complete list with parameter details.  For remote sensing, geomorphometry, and precision agriculture tool signatures see the [themed reference documents](TOOLS.md#themed-tool-reference-documents).

All convenience functions accept optional keyword arguments:
- `callback`: A callable that receives progress/message events as JSON strings
- `include_pro`: Set to `True` to use Pro tools if licensed
- `tier`: License tier (`"open"`, `"pro"`, or `"enterprise"`)

---

## Advanced: RuntimeSession

For maximum control, use the `RuntimeSession` class:

```python
import whitebox_workflows
import json

session = whitebox_workflows.RuntimeSession(include_pro=False, tier="open")

args_json = json.dumps({"input": "input.tif", "output": "output.tif"})
result = session.run_tool_json("abs", args_json)
```

### Typed Plugin Outputs (Heterogeneous Returns)

In addition to `run_tool_json(...)`, the bindings now provide typed execution
helpers that decode tool outputs into Python objects:

- `whitebox_workflows.run_tool(...)`
- `whitebox_workflows.run_tool_with_options(...)`
- `whitebox_workflows.run_tool_stream(...)`
- `whitebox_workflows.run_tool_stream_options(...)`
- `RuntimeSession.run_tool(...)`
- `RuntimeSession.run_tool_stream(...)`

Typed run methods accept either:

- a JSON argument string (same as `run_tool_json`), or
- a Python `dict` containing JSON-compatible values and/or Raster/Vector/Lidar objects.

This allows old-style object-based invocation patterns where tools receive
in-memory Python wrapper objects as parameters.

Example object-based call:

```python
import whitebox_workflows

wbe = whitebox_workflows.WbEnvironment()
r1 = wbe.read_raster("dem_a.tif")
r2 = wbe.read_raster("dem_b.tif")

# Pass Raster objects directly instead of file-path strings
sum_raster = whitebox_workflows.run_tool("add", {
    "input1": r1,
    "input2": r2,
    "output": "dem_sum.tif",
})
```

If `output` is omitted for tools that support it (e.g. `add`), the result can
stay in memory and be chained directly into the next tool call without an
intermediate raster write/read step:

```python
tmp = whitebox_workflows.run_tool("add", {"input1": r1, "input2": r2})
result = whitebox_workflows.run_tool("add", {"input1": tmp, "input2": r1})
```

Typed methods support heterogeneous return structures, including mixed dict/list
values and tuple outputs. To emit typed geospatial objects from a plugin tool,
return output values using a typed envelope:

```json
{"__wbw_type__": "raster", "path": "dem.tif", "active_band": 0}
```

Supported envelope kinds:

- `raster`: `{"__wbw_type__": "raster", "path": "...", "active_band": 0}`
- `vector`: `{"__wbw_type__": "vector", "path": "..."}`
- `lidar`: `{"__wbw_type__": "lidar", "path": "..."}`
- `tuple`: `{"__wbw_type__": "tuple", "items": [ ... ]}`

Example tool output map with mixed return types:

```json
{
    "primary": {"__wbw_type__": "raster", "path": "out1.tif"},
    "secondary": {"__wbw_type__": "raster", "path": "out2.tif"},
    "pair": {
        "__wbw_type__": "tuple",
        "items": [
            {"__wbw_type__": "raster", "path": "out1.tif"},
            {"__wbw_type__": "vector", "path": "features.gpkg"}
        ]
    }
}
```

