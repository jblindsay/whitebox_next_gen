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

Use generic accessors for dynamic workflows:
- `wbe.category(name)`
- `wbe.domain(name)`

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
| `hydrology`, `terrain`, `raster`, `vector`, `lidar`, `remote_sensing`, `conversion`, `topology_tools` | Typed category namespaces for tool discovery and execution. |
| `other`, `precision_agriculture`, `streams` | Additional category/domain namespaces available in the environment. |
