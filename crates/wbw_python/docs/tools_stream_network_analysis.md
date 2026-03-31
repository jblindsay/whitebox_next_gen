# Stream Network Analysis Tools

This document provides comprehensive documentation for stream network analysis tools. For common raster I/O conventions, parameter formats, and usage patterns, see [the main TOOLS.md reference](../TOOLS.md#common-conventions).

## Overview

Stream network analysis tools operate on digital elevation models (DEMs) to extract, analyze, and characterize stream networks. These tools use D8 (eight-directional) flow direction algorithms to trace flow paths and compute stream properties.

### Key Concepts

- **D8 Pointer**: A raster encoding the direction of steepest descent from each cell to one of its 8 neighbors. Required input for most tools.
- **Streams Raster**: A binary or continuous raster where positive values mark stream cells; zero and NoData values mark non-stream cells.
- **Stream Order**: A hierarchical classification system ranking stream segments based on tributaries and confluence patterns.
- **Stream Link**: A continuous stream segment between junctions (confluences) or from a source to the first junction.

## Tool Index

### Stream Ordering Tools

Stream ordering systems classify streams hierarchically based on network structure. Each system makes different assumptions about what constitutes the "main stem" and how order changes at confluences.

### Strahler Stream Order

Assigns stream order based on the algorithm: headwater links are order 1; when two links of equal order join, the downstream link is order+1; when links of different orders join, the downstream link takes the higher order.

**Parameters:**
- `d8_pntr` (Raster): D8 flow direction pointer raster
- `streams_raster` (Raster): Stream network raster (positive values = streams)
- `esri_pntr` (bool, optional): If true, use ESRI-style pointer values; otherwise use Whitebox style (default: false)
- `zero_background` (bool, optional): If true, assign zero to non-stream cells; otherwise use NoData (default: false)
- `output` (str, optional): Output raster path

**Output:** Raster with Strahler order values

**References:**
- Strahler, A. N. (1957). Quantitative analysis of watershed geomorphology. EOS Transactions American Geophysical Union, 38(6), 913-920.

### Horton Stream Order

Assigns stream order starting from Strahler order, then replaces all cells along the main trunk (longest path from outlet) with the outlet's order value. This emphasizes the main channel rather than tributary networks.

**Parameters:** Same as Strahler Stream Order

**Output:** Raster with Horton order values

**References:**
- Horton, R. E. (1945). Erosional development of streams and their drainage basins. GSA Bulletin, 56(3), 275-370.

### Hack Stream Order

Assigns order from the outlet upstream: the outlet (main stem) is order 1, tributaries to the main stem are order 2, and tributary order increases upstream. Unlike Strahler, order increases away from the outlet, which is useful when outlet location is certain but headwater extent is uncertain.

**Parameters:** Same as Strahler Stream Order

**Output:** Raster with Hack order values

**References:**
- Hack, J. T. (1957). Studies of longitudinal stream profiles in Virginia and Maryland. USGS Professional Paper 294-B.

### Topological Stream Order

Assigns order based on the count of upstream links (topological distance from the outlet). Each confluence increases order by one moving downstream.

**Parameters:** Same as Strahler Stream Order

**Output:** Raster with topological order values

## Stream Magnitude and Magnitude-Related Tools

### Shreve Stream Magnitude

Calculates Shreve stream magnitude: the sum of magnitude values of upstream headwater cells (all headwaters count as magnitude 1). Magnitude increases when tributaries join—new magnitude = sum of upstream tributaries.

**Parameters:** Same as Strahler Stream Order

**Output:** Raster with Shreve magnitude values

**References:**
- Shreve, R. L. (1966). Statistical law of stream numbers. Journal of Geology, 74(1), 17-37.

### Stream Link Identifier

Assigns unique integer identifiers to each stream link (continuous segment between junctions or from headwater to first junction). Useful for extracting and analyzing individual segments.

**Parameters:**
- `d8_pntr` (Raster): D8 flow direction pointer
- `streams_raster` (Raster): Stream network raster
- `esri_pntr` (bool, optional): ESRI-style pointer flag
- `zero_background` (bool, optional): Zero background flag
- `output` (str, optional): Output raster path

**Output:** Raster with unique link identifiers

### Stream Link Class

Classifies each stream link by type:
- 1: Exterior (headwater link with no upstream)
- 2: Interior (link with 1+ upstream tributaries, drains to another link)
- 3: Source (similar to exterior)
- 4: Link (standard interior link)
- 5: Sink (terminal outlet)

**Parameters:** Same as Stream Link Identifier

**Output:** Raster with classified link types

### Stream Link Length

Calculates total length for each stream link. All cells in the same link have the same value (the complete link length).

**Parameters)**
- `d8_pntr` (Raster): D8 flow direction pointer
- `streams_raster` (Raster): Stream network raster
- `dem` (Raster): Digital elevation model (for cell size/distance calculation)
- `esri_pntr` (bool, optional): ESRI-style pointer flag
- `output` (str, optional): Output raster path

**Output:** Raster with link lengths

### Stream Link Slope

Calculates average slope for each stream link. Slope is computed as vertical drop over horizontal distance along the stream.

**Parameters:**
- `d8_pntr` (Raster): D8 flow direction pointer
- `streams_raster` (Raster): Stream network raster
- `dem` (Raster): Digital elevation model
- `esri_pntr` (bool, optional): ESRI-style pointer flag
- `output` (str, optional): Output raster path

**Output:** Raster with link slopes

### Stream Slope Continuous

Calculates slope at each stream cell using neighbors along the flow direction. Provides cell-by-cell slope rather than constant per-link values.

**Parameters:** Same as Stream Link Slope

**Output:** Raster with per-cell slopes

## Stream Network Extraction Tools

### Extract Streams

Extracts stream network from flow accumulation raster using a user-defined threshold. Cells with flow accumulation >= threshold are marked as streams.

**Parameters:**
- `flow_accumulation` (Raster): Flow accumulation raster (output from a flow accumulation tool)
- `threshold` (float): Minimum flow accumulation value to designate as stream
- `output` (str, optional): Output raster path

**Output:** Binary stream raster (1 = stream, 0 = non-stream)

### Extract Valleys

Extracts valley networks (broader low-lying areas) from a DEM. Offers multiple algorithms:
- **Laplace/Quadratic**: Curvature-based valley detection
- **Lindsay-Curvature**: Custom curvature index
- **Peucker-Douglas**: Tangential curvature method

**Parameters:**
- `dem` (Raster): Digital elevation model
- `type` (str): Valley detection method (default: "quadratic")
- `thin` (bool, optional): Thin valley lines to single-cell width (default: false)
- `output` (str, optional): Output raster path

**Output:** Valley network raster

## Stream Network Distance and Upstream Analysis Tools

### Distance to Outlet

Calculates downstream channel distance along the D8 network to the outlet for each stream cell.

**Parameters:**
- `d8_pntr` (Raster): D8 flow direction pointer
- `streams_raster` (Raster): Stream network raster
- `dem` (Raster): Digital elevation model (for distance calculation)
- `esri_pntr` (bool, optional): ESRI-style pointer flag
- `output` (str, optional): Output raster path

**Output:** Raster with distance to outlet (in horizontal distance units)

### Length of Upstream Channels

Calculates total upstream channel length for each stream cell. Useful for identifying headwater areas and stream densities.

**Parameters:**
- `d8_pntr` (Raster): D8 flow direction pointer
- `streams_raster` (Raster): Stream network raster
- `dem` (Raster): Digital elevation model
- `esri_pntr` (bool, optional): ESRI-style pointer flag
- `output` (str, optional): Output raster path

**Output:** Raster with upstream channel lengths

### Farthest Channel Head

Calculates upstream distance to the most distant channel head for each stream cell.

**Parameters:** Same as Length of Upstream Channels

**Output:** Raster with distance to farthest upstream head

## Stream Main Stem and Tributary Tools

### Find Main Stem

Identifies cells belonging to the main channel (longest stream path from outlet). Outputs a binary mask.

**Parameters:**
- `d8_pntr` (Raster): D8 flow direction pointer
- `streams_raster` (Raster): Stream network raster
- `esri_pntr` (bool, optional): ESRI-style pointer flag
- `output` (str, optional): Output raster path

**Output:** Boolean raster (1 = main stem, 0 = tributary)

### Tributary Identifier

Assigns unique identifier to each tributary system. Useful for analyzing tributary properties.

**Parameters:** Same as Find Main Stem

**Output:** Raster with tributary identifiers

### Remove Short Streams

Prunes stream links shorter than a minimum length threshold. Useful for cleaning noisy stream networks.

**Parameters:**
- `d8_pntr` (Raster): D8 flow direction pointer
- `streams_raster` (Raster): Stream network raster
- `dem` (Raster): Digital elevation model
- `min_length` (float): Minimum link length to retain (in map units)
- `esri_pntr` (bool, optional): ESRI-style pointer flag
- `output` (str, optional): Output raster path

**Output:** Pruned stream raster with short links removed

## Stream Network Conversion Tools

### Raster Streams to Vector

Converts a raster stream network to a vector line layer. Each stream link becomes a PolyLine feature with attributes showing stream value and link length.

**Parameters:**
- `streams` (Raster): Stream network raster
- `d8_pointer` (Raster): D8 flow direction pointer
- `esri_pointer` (bool, optional): ESRI-style pointer flag
- `all_vertices` (bool, optional): Include all vertices or only direction changes (default: false)

**Output:** Vector layer with stream PolyLines

### Rasterize Streams

Converts a vector stream line layer to a raster grid.

**Parameters:**
- `input_vector` (Vector): Input stream network vector layer
- `reference_raster` (Raster): Reference raster for grid specification
- `field` (str, optional): Attribute field for output values (default: stream ID)
- `output` (str, optional): Output raster path

**Output:** Raster stream network

### Repair Stream Vector Topology

Repairs broken topology in vector stream networks, including:
- Snapping nearly-overlapping endpoints
- Splitting lines at intersections
- Removing duplicate segments
- Removing invalid geometries

**Parameters:**
- `input_vector` (Vector): Input stream vector layer
- `snap_distance` (float): Maximum distance for snapping endpoints (default: 1 cell size)
- `output` (str, optional): Output vector path

**Output:** Repaired vector stream layer

## Long Profile Tools

Long profiles are plots of elevation against downstream distance. Useful for analyzing stream gradient and identifying bedrock steps.

### Long Profile

Creates an interactive SVG line graph showing elevation against distance to outlet for the entire stream network. Outputs an HTML document with embedded SVG.

**Parameters:**
- `d8_pointer` (Raster): D8 flow direction pointer
- `streams_raster` (Raster): Stream network raster
- `dem` (Raster): Digital elevation model
- `output_html_file` (str): Output HTML file path
- `esri_pointer` (bool, optional): ESRI-style pointer flag

**Output:** Interactive HTML document with longitudinal profile

### Long Profile from Points

Creates long profiles for sample points along stream network. Useful for extracting profiles at specific locations.

**Parameters:**
- `points_vector` (Vector): Vector points at profile sample locations
- `d8_pointer` (Raster): D8 flow direction pointer
- `streams_raster` (Raster): Stream network raster
- `dem` (Raster): Digital elevation model
- `output_folder` (str): Output folder for HTML files
- `esri_pointer` (bool, optional): ESRI-style pointer flag

**Output:** Set of HTML profile documents (one per input point)

## Comprehensive Stream Analysis Tools

### Vector Stream Network Analysis

Performs comprehensive analysis on a vector stream network, generating multiple output layers:
- Stream links with order, magnitude, length, and slope
- Stream thalwegs (centerlines)
- Drainage basins (catchments)
- Stream confluences (junctions)

**Parameters:**
- `input_vector` (Vector): Input stream network
- `dem` (Raster, optional): Digital elevation model for slope/aspect analysis
- `output_folder` (str): Output folder for generated layers

**Output:** Multiple vector layers with comprehensive stream attributes

## Professional Tools (Pro License Required)

### Prune Vector Streams

Advanced vector stream pruning based on Shreve magnitude and stream type criteria. Allows selective removal of minor tributaries while preserving main stem structure.

**Parameters:**
- `input` (Vector): Input stream network
- `magnitude_threshold` (float): Minimum Shreve magnitude to retain (default: 2)
- `output` (str, optional): Output vector path

**Output:** Pruned stream network

### River Centerlines

Extracts river centerlines from a binary water mask raster using medial axis transformation. Useful for generating channel centerlines from satellite or aerial imagery.

**Parameters:**
- `water_raster` (Raster): Binary water/non-water raster
- `output` (str, optional): Output vector centerline path

**Output:** Vector centerline PolyLines

## Usage Notes

### D8 Pointer Conventions

Stream network tools accept both Whitebox and ESRI D8 pointer conventions. Whitebox convention:
- 1 = E, 2 = NE, 4 = N, 8 = NW, 16 = W, 32 = SW, 64 = S, 128 = SE

ESRI convention:
- 1 = E, 2 = SE, 4 = S, 8 = SW, 16 = W, 32 = NW, 64 = N, 128 = NE

Set `esri_pntr=True` when using ESRI-convention pointers.

### DEM Preparation

Before extracting streams, DEMs should be preprocessed to remove topographic depressions and flat areas. Tools are typically:
1. `fill_depressions` or `breach_depressions_least_cost` (from geomorphometry tools)
2. `d8_pointer` (compute D8 flow directions on depressionless DEM)
3. `flow_accumulation` (compute d8 flow accumulation)

### Typical Workflow

```python
# Example workflow (pseudocode)
dem_filled = fill_depressions(dem)
d8_ptr = d8_pointer(dem_filled)
flow_accum = flow_accumulation(d8_ptr)
streams = extract_streams(flow_accum, threshold=100)
strahler_order = strahler_stream_order(d8_ptr, streams)
streams_vector = raster_streams_to_vector(streams, d8_ptr)
```

## See Also

- [Geomorphometry Tools](tools_geomorphometry.md) - Related DEM analysis tools
- [TOOLS.md](../TOOLS.md) - Common conventions and raster I/O methods
