# Projection and Georeferencing

Accurate coordinate reference systems (CRS) and georeferencing are foundational to all GIS work. This chapter covers tools for assigning, reprojecting, and transforming spatial data between coordinate systems, as well as tools for georeferencing rasters from ground control points.

## Key Concepts

- **CRS / Projection**: A mathematical model that defines how geographic coordinates map to a flat surface. All spatial data in a GIS project must share a common CRS for overlay and analysis to be meaningful.
- **Reprojection**: Transforming data from one CRS to another. Whitebox Workflows supports epoch-aware datum transformations for sub-metre accuracy.
- **Georeferencing**: Assigning spatial coordinates to a raster image using ground control points (GCPs), typically for aerial photography, scanned maps, or satellite imagery without embedded metadata.
- **Orthorectification**: Correcting geometric distortions in aerial/satellite imagery caused by terrain relief and sensor tilt.

## Tool Reference

The tools in this chapter are accessible from the QGIS Processing Toolbox under **Whitebox Workflows → Projection and Georeferencing**.

