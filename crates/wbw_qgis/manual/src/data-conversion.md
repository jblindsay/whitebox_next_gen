# Data Conversion and Format Tools

Data conversion tools handle format transformations, topology repairs, and attribute table operations that are essential plumbing in any GIS workflow. These tools prepare data for analysis, export results to standard formats, and ensure geometric and topological consistency.

## Key Concepts

- **Vector-Raster Conversion**: Many analysis pipelines require moving between vector and raster representations. Whitebox provides precise control over cell size, nodata handling, and attribute transfer during conversion.
- **Topology Repair**: Real-world vector data often contains geometric errors — dangling arcs, unclosed polygons, multipart features — that cause downstream analysis failures. The topology tools detect and fix these automatically.
- **Attribute Table I/O**: Joining external CSVs, merging attribute tables across layers, and exporting to standard tabular formats is routine data preparation work.

## Tool Reference

The tools in this chapter are accessible from the QGIS Processing Toolbox under **Whitebox Workflows → Data Conversion**.

