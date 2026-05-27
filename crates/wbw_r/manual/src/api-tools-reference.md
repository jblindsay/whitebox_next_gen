# Tool API Reference (All Tools)

This chapter documents tool wrappers by theme.

For R usage, invoke tools either with:

- generic execution (`wbw_run_tool("tool_id", args = list(...), session = s)`)
- generated session wrappers (`s$tool_id(...)`)

Argument names and contracts in the shared tool docs map directly to R `args`
keys and wrapper parameters.

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
