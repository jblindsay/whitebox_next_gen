# Tool API Reference (All Tools)

This chapter documents tool wrappers by theme. It is sourced from the shared
tool-reference markdown files to reduce drift between wrappers and manual docs.

Important call-style note:

- Names shown in the included lists (for example d8_flow_accum) are canonical
	tool identifiers, not flat global methods on WbEnvironment.
- Preferred invocation is:
	- for `general` subcategory tools: category-level form (for example `wbe.raster.arctan(...)`)
	- for non-`general` tools: nested category/subcategory form

- Examples of nested category/subcategory style:
	- wbe.hydrology.flow_routing.d8_flow_accum(...)
	- wbe.hydrology.depressions_storage.fill_depressions(...)
- Category-level calls are supported across the API surface and are preferred
	for `general` tools.
- Generic invocation always works through wbe.run_tool(tool_id, args).

For an exhaustive tool_id-to-call-path lookup, see
[Tool Call Paths (Python)](./api-tool-call-paths.md).

## Math And Statistics

{{#include ../../docs/tools_math.md}}

## Hydrology

{{#include ../../docs/tools_hydrology.md}}

## GIS And Vector

{{#include ../../docs/tools_gis.md}}

## Remote Sensing

{{#include ../../docs/tools_remote_sensing.md}}

## Geomorphometry And Terrain Signatures

{{#include ../../docs/tools_geomorphometry.md}}

## Precision Agriculture

{{#include ../../docs/tools_agriculture.md}}

## LiDAR Processing

{{#include ../../docs/tools_lidar_processing.md}}

## Stream Network Analysis

{{#include ../../docs/tools_stream_network_analysis.md}}

## Data Tools

{{#include ../../docs/tools_data_tools.md}}
