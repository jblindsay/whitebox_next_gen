# Tool API Reference (All Tools)

This chapter documents tool wrappers by theme.

For R usage, invoke tools either with:

- generic execution (`wbw_run_tool("tool_id", args = list(...), session = s)`)
- generated session wrappers (`s$tool_id(...)`)

Argument names and contracts in the shared tool docs map directly to R `args`
keys and wrapper parameters.

Important call-style note:

- Names shown in included lists are canonical tool identifiers.
- For generic execution, call by identifier through wbw_run_tool.
- For wrapper execution, use generated session methods on the session object.
- The nested category/subcategory convention is the canonical shape for Python
	WbEnvironment tool namespaces; in R, generated session wrappers and
	wbw_run_tool are the practical call surfaces.

For an exhaustive tool_id-to-R-call lookup, see
[Tool Call Paths (R)](./api-tool-call-paths-r.md).

## Math And Statistics

{{#include ../../../wbw_python/docs/tools_math.md}}

## Hydrology

{{#include ../../../wbw_python/docs/tools_hydrology.md}}

## GIS And Vector

{{#include ../../../wbw_python/docs/tools_gis.md}}

## Remote Sensing

{{#include ../../../wbw_python/docs/tools_remote_sensing.md}}

## Geomorphometry And Terrain Signatures

{{#include ../../../wbw_python/docs/tools_geomorphometry.md}}

## Precision Agriculture

{{#include ../../../wbw_python/docs/tools_agriculture.md}}

## LiDAR Processing

{{#include ../../../wbw_python/docs/tools_lidar_processing.md}}

## Stream Network Analysis

{{#include ../../../wbw_python/docs/tools_stream_network_analysis.md}}

## Data Tools

{{#include ../../../wbw_python/docs/tools_data_tools.md}}
