# Environment and Discovery

This chapter covers environment lifecycle and tool discovery patterns.

Environment setup is where workflow reliability begins. By explicitly creating
and configuring a runtime environment, you make script behavior predictable
across machines and sessions. Discovery APIs then let you verify capability
before execution, which avoids long-running failures caused by missing tools,
unexpected categories, or version mismatches.

## Create and Configure Environment

This example establishes an explicit runtime configuration. In production
scripts, this is where you set working directory and verbosity policy.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
wbe.working_directory = '/path/to/data'
wbe.verbose = True

print(wbe.version())
print(wbe.license_type())
```

## Namespace Discovery

Use discovery to understand available capability before writing long workflows
or generating dynamic tooling UIs.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()

print('categories:', wbe.categories())
print('domains:', wbe.domain_namespaces())
print('terrain tools sample:', wbe.terrain.list_tools()[:15])
```

## Search and Describe Tools

This pattern is useful when you know a task but not the exact tool identifier.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()

matches = wbe.search_tools('slope')
for m in matches[:5]:
	print(m.get('tool_id', m))

desc = wbe.describe_tool('slope')
print(desc)
```

## Discovering Parameter Values with describe_tool

`describe_tool` returns a dictionary containing structured parameter metadata.
Each entry in the `params` list includes:

| Key | Always present | Description |
|---|---|---|
| `name` | yes | Parameter name as used in tool calls |
| `description` | yes | Human-readable description |
| `required` | yes | `True` if the parameter must be supplied |
| `type` | when set | Semantic type hint: `"string"`, `"float"`, `"int"`, `"bool"`, `"path"`, `"array[int]"` |
| `choices` | when set | List of valid string values for constrained parameters |
| `default_value` | when set | Default value as a string, for display purposes |

Use this to enumerate valid values before calling a tool:

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()

desc = wbe.describe_tool('lidar_tin_gridding')
for p in desc['params']:
    name = p['name']
    choices = p.get('choices')
    default = p.get('default_value')
    if choices:
        print(f"{name}: choices={choices}, default={default!r}")
    else:
        print(f"{name}: type={p.get('type', 'any')}, default={default!r}")
```

Example output (selected parameters):

```
interpolation_parameter: choices=['elevation', 'intensity', 'class', 'return_number', 'number_of_returns', 'scan_angle', 'time', 'rgb', 'user_data'], default='elevation'
returns_included: choices=['all', 'first', 'last'], default='all'
triangulation_backend: choices=['auto', 'delaunator', 'wbtopology'], default='auto'
triangulation_thin_method: choices=['nearest_center', 'min_value', 'max_value'], default='nearest_center'
triangulation_thin_cell_size: type=float, default='0.0'
```

This is especially useful for building dynamic UIs, generating documentation,
or writing validation helpers that do not hard-code allowed values.

## Schema-Aware Tool Metadata JSON

For canonical parameter I/O typing, prefer `get_tool_info_json` (or
`get_tool_metadata_json`) over name/description heuristics. The JSON payload
includes `io_role` and `data_kind` per parameter.

```python
import json
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
tool = json.loads(wbe.get_tool_info_json('slope'))

for p in tool.get('params', []):
    print(
        p['name'],
        'role=', p.get('io_role', 'unknown'),
        'kind=', p.get('data_kind', 'unknown')
    )
```

| Field | Meaning |
|---|---|
| `io_role` | Parameter role: `input`, `output`, or non-I/O `argument`. |
| `data_kind` | Canonical family such as `raster`, `vector`, `lidar`, `table`, `json`, `text`, `file`, `bool`, `number`, or `string`. |

Use these fields for integration logic such as destination widget selection,
layer-output handling, and catalog validation.

## Validate Tool Availability

Use hard checks like this in batch scripts so failures occur immediately, before
expensive processing begins.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
available = set(wbe.list_tools())

required = {'fill_depressions', 'd8_flow_accum'}
missing = sorted(required - available)
if missing:
	raise RuntimeError(f'Missing required tools: {missing}')
```

## Category Access Patterns

Prefer direct properties when available:
- `wbe.hydrology`
- `wbe.terrain`
- `wbe.raster`
- `wbe.vector`
- `wbe.lidar`
- `wbe.remote_sensing`
- `wbe.conversion`
- `wbe.streams`
- `wbe.precision_agriculture`
- `wbe.other`

Use generic accessors for dynamic workflows:
- `wbe.category(name)`
- `wbe.domain(name)`

Projection/georeferencing tools are cataloged under the canonical category key
`projection_georeferencing` and can be queried with:

```python
proj_tools = wbe.category('projection_georeferencing').list_tools()
print(proj_tools[:10])
```

## WbEnvironment Method Reference

This reference lists callable `WbEnvironment` methods with brief descriptions.
Special Python dunder methods are intentionally omitted.

### Constructors

| Method | Description |
|---|---|
| `from_floating_license_id` | Create an environment using a floating license identifier and provider settings. |
| `from_signed_entitlement_file` | Create an environment from a signed entitlement file and public-key metadata. |
| `from_signed_entitlement_json` | Create an environment from signed entitlement JSON and public-key metadata. |

### Runtime and Licensing

| Method | Description |
|---|---|
| `version` | Return the Whitebox runtime version string. |
| `license_type` | Return the active license mode (for example `open`, floating, or entitlement-backed). |
| `license_info` | Return detailed license status information for diagnostics. |

### Discovery and Introspection

| Method | Description |
|---|---|
| `list_tools` | Return all available tool IDs visible in the current environment. |
| `categories` | Return the list of top-level tool categories. |
| `category` | Return a category namespace object by name. |
| `domain_namespaces` | Return available domain namespace names. |
| `domain` | Return a domain namespace object by name. |
| `describe_tool` | Return metadata and parameter details for a specific tool ID. |
| `get_tool_metadata_json` | Return canonical metadata JSON for one tool ID, including `io_role`/`data_kind`. |
| `get_tool_info_json` | Alias of `get_tool_metadata_json` for cross-binding API parity. |
| `search_tools` | Search tools by keyword or phrase. |
| `list_tools_detailed` | Return expanded metadata for all visible tools. |

### Core Data Readers

| Method | Description |
|---|---|
| `read_raster` | Read one raster into a `Raster` object. |
| `read_vector` | Read one vector dataset into a `Vector` object. |
| `read_lidar` | Read one lidar dataset into a `Lidar` object. |
| `read_rasters` | Read multiple rasters, optionally in parallel. |
| `read_vectors` | Read multiple vectors, optionally in parallel. |
| `read_lidars` | Read multiple lidar datasets, optionally in parallel. |

### Sensor Bundle Readers and Composites

| Method | Description |
|---|---|
| `read_bundle` | Auto-detect and read a supported sensor bundle. |
| `read_landsat` | Read a Landsat bundle with family-specific parsing. |
| `read_sentinel2` | Read a Sentinel-2 SAFE bundle. |
| `read_sentinel1` | Read a Sentinel-1 SAFE bundle. |
| `read_planetscope` | Read a PlanetScope bundle. |
| `read_iceye` | Read an ICEYE bundle. |
| `read_dimap` | Read a DIMAP bundle. |
| `read_maxar_worldview` | Read a Maxar/WorldView bundle. |
| `read_radarsat2` | Read a RADARSAT-2 bundle. |
| `read_rcm` | Read a RADARSAT Constellation Mission (RCM) bundle. |
| `true_colour_composite` | Build a true-colour composite raster from a bundle source. |
| `false_colour_composite` | Build a false-colour composite raster from a bundle source. |

### Reprojection Helpers

| Method | Description |
|---|---|
| `reproject_raster` | Reproject one raster with explicit resampling and grid controls. |
| `reproject_vector` | Reproject one vector dataset with policy controls. |
| `reproject_lidar` | Reproject one lidar dataset with transform and failure controls. |
| `reproject_rasters` | Batch reproject multiple rasters. |
| `reproject_vectors` | Batch reproject multiple vector datasets. |
| `reproject_lidars` | Batch reproject multiple lidar datasets. |

### Writers and Raster-Memory Management

| Method | Description |
|---|---|
| `write_raster` | Write one raster to disk with optional format options. |
| `write_rasters` | Write multiple rasters to disk in one call. |
| `remove_raster_from_memory` | Drop a specific memory-backed raster from the environment cache. |
| `clear_raster_memory` | Clear all memory-backed rasters tracked by the environment. |
| `clear_memory` | Clear all memory-backed rasters, vectors, and LiDAR objects tracked by the environment. |
| `raster_memory_count` | Return the count of memory-backed rasters currently tracked. |
| `raster_memory_bytes` | Return the estimated bytes used by tracked memory-backed rasters. |
| `write_vector` | Write one vector dataset to disk with optional format options. |
| `write_lidar` | Write one lidar dataset to disk with optional format options. |
| `write_text` | Write plain text content to a file path. |

### Key Environment Properties

| Property | Description |
|---|---|
| `working_directory` | Default working directory used for relative paths. |
| `verbose` | Controls environment/runtime status output emitted by the bindings. |
| `max_procs` | Maximum process count used by eligible parallel operations. |
| `projection` | Namespace for CRS and coordinate transformation helper methods. |
| `topology` | Namespace for geometry-topology helper methods. |
| `hydrology`, `terrain`, `raster`, `vector`, `lidar`, `remote_sensing` | Primary category namespaces for tool discovery and execution. |
| `precision_agriculture` | Pro-tier precision agriculture tools (yield zoning, irrigation, crop stress, trafficability). |
| `conversion`, `streams`, `other` | Additional category namespaces available in the environment. |
