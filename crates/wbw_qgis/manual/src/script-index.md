# Workflow Index

This index provides task-first entry points for WbW-QGIS workflows.
Each entry links to the chapter section where the complete step-by-step
example can be found, and lists the key Processing Toolbox tool IDs needed
for the task.

---

## Terrain Analysis

| Task | Chapter section | Key tools |
|------|----------------|-----------|
| Compute slope from DEM | [Terrain Analysis — Step 2](terrain-analysis.md) | `whitebox_workflows:slope` |
| Compute aspect from DEM | [Terrain Analysis — Step 3](terrain-analysis.md) | `whitebox_workflows:aspect` |
| Generate hillshade for visualisation | [Terrain Analysis — Step 4](terrain-analysis.md) | `whitebox_workflows:hillshade` |
| Compute profile and plan curvature | [Terrain Analysis — Step 5](terrain-analysis.md) | `whitebox_workflows:profile_curvature`, `whitebox_workflows:plan_curvature` |
| Classify terrain into landform elements | [Terrain Analysis — Geomorphons](terrain-analysis.md) | `whitebox_workflows:geomorphons` |
| Compute topographic wetness index | [Terrain Analysis — Step 6](terrain-analysis.md) | `whitebox_workflows:wetness_index` |
| Fill depressions before terrain derivatives | [Terrain Analysis — Step 1](terrain-analysis.md) | `whitebox_workflows:fill_depressions` |

---

## Spatial Hydrology

| Task | Chapter section | Key tools |
|------|----------------|-----------|
| Condition DEM for hydrologic routing | [Spatial Hydrology — Step 1](spatial-hydrology.md) | `whitebox_workflows:breach_depressions_least_cost` |
| Derive D8 flow direction | [Spatial Hydrology — Step 2](spatial-hydrology.md) | `whitebox_workflows:d8_pointer` |
| Compute flow accumulation | [Spatial Hydrology — Step 3](spatial-hydrology.md) | `whitebox_workflows:d8_flow_accumulation` |
| Extract stream network from accumulation | [Spatial Hydrology — Step 4](spatial-hydrology.md) | `whitebox_workflows:extract_streams` |
| Snap pour points to channel raster | [Spatial Hydrology — Step 5](spatial-hydrology.md) | `whitebox_workflows:snap_pour_points` |
| Delineate watershed / catchment | [Spatial Hydrology — Step 6](spatial-hydrology.md) | `whitebox_workflows:watershed` |
| Compute Topographic Wetness Index | [Spatial Hydrology — TWI](spatial-hydrology.md) | `whitebox_workflows:wetness_index` |

---

## LiDAR Processing

| Task | Chapter section | Key tools |
|------|----------------|-----------|
| QA — inspect point cloud statistics | [LiDAR Processing — Step 1](lidar-processing.md) | `whitebox_workflows:lidar_point_stats` |
| Thin high-density point cloud | [LiDAR Processing — Step 2](lidar-processing.md) | `whitebox_workflows:lidar_thin` |
| Classify ground returns | [LiDAR Processing — Step 3](lidar-processing.md) | `whitebox_workflows:lidar_ground_point_filter` |
| Build DTM from ground-classified cloud | [LiDAR Processing — Step 4](lidar-processing.md) | `whitebox_workflows:lidar_idw_interpolation` |
| Build DSM from first returns | [LiDAR Processing — Step 5](lidar-processing.md) | `whitebox_workflows:lidar_idw_interpolation` |
| Derive canopy height model (CHM) | [LiDAR Processing — Step 6](lidar-processing.md) | `whitebox_workflows:canopy_height_model` |
| Normalise heights above ground | [LiDAR Processing — Step 7](lidar-processing.md) | `whitebox_workflows:height_above_ground` |

---

## Remote Sensing

| Task | Chapter section | Key tools |
|------|----------------|-----------|
| Compute NDVI from multispectral image | [Remote Sensing — Step 2](remote-sensing.md) | `whitebox_workflows:ndvi` |
| Threshold vegetation from NDVI | [Remote Sensing — Step 3](remote-sensing.md) | QGIS Raster Calculator |
| NDVI-based change detection | [Remote Sensing — Steps 4–5](remote-sensing.md) | QGIS Raster Calculator |
| Reduce bands with PCA | [Remote Sensing — PCA](remote-sensing.md) | `whitebox_workflows:principal_component_analysis` |
| Segment image into homogeneous objects | [Remote Sensing — Segmentation](remote-sensing.md) | `whitebox_workflows:image_segmentation` |

---

## Raster Analysis

| Task | Chapter section | Key tools |
|------|----------------|-----------|
| Compute distance from a binary feature raster | [Raster Analysis — Step 1](raster-analysis.md) | `whitebox_workflows:euclidean_distance` |
| Reclassify raster into suitability scores | [Raster Analysis — Steps 2–4](raster-analysis.md) | `whitebox_workflows:reclass_from_file` |
| Combine reclassified factors by weight | [Raster Analysis — Step 5](raster-analysis.md) | QGIS Raster Calculator |
| Summarise raster values within polygons | [Raster Analysis — Step 6](raster-analysis.md) | QGIS Zonal Statistics |
| Smooth raster with focal mean | [Raster Analysis — Focal Statistics](raster-analysis.md) | `whitebox_workflows:mean_filter` |

---

## Vector Analysis

| Task | Chapter section | Key tools |
|------|----------------|-----------|
| Validate and repair polygon geometry | [Vector Analysis — Step 1](vector-analysis.md) | QGIS `native:fixgeometries` |
| Add area, perimeter, centroid attributes | [Vector Analysis — Step 2](vector-analysis.md) | `whitebox_workflows:add_geometry_attributes` |
| Join attributes from overlapping polygons | [Vector Analysis — Step 3](vector-analysis.md) | `whitebox_workflows:spatial_join` |
| Compute distance to nearest feature | [Vector Analysis — Step 4](vector-analysis.md) | `whitebox_workflows:near` |
| Select features by spatial predicate | [Vector Analysis — Step 5](vector-analysis.md) | QGIS Select by Expression |
| Simplify polygon boundaries | [Vector Analysis — Simplify](vector-analysis.md) | `whitebox_workflows:simplify_features` |
| Convert GeoPackage to TopoJSON and back | [Vector Analysis — TopoJSON Conversion Chain](vector-analysis.md) | `whitebox_workflows:add_geometry_attributes`, QGIS Export |
| Simplify shared boundaries and emit TopoJSON | [Vector Analysis — TopoJSON Boundary-Preserving Generalization Chain](vector-analysis.md) | `whitebox_workflows:simplify_features`, QGIS Export |
| Convert TopoJSON transport input, enrich, and re-emit | [Vector Analysis — TopoJSON Transport + Enrichment Return Chain](vector-analysis.md) | QGIS Export, `whitebox_workflows:add_geometry_attributes`, `whitebox_workflows:spatial_join`, `whitebox_workflows:near` |

---

## Network Analysis

| Task | Chapter section | Key tools |
|------|----------------|-----------|
| Compute Strahler stream order | [Network Analysis — Step 1](network-analysis.md) | `whitebox_workflows:strahler_stream_order` |
| Compute Shreve stream magnitude | [Network Analysis — Step 2](network-analysis.md) | `whitebox_workflows:shreve_stream_magnitude` |
| Convert raster stream network to vector | [Network Analysis — Step 3](network-analysis.md) | `whitebox_workflows:raster_streams_to_vector` |
| Compute road segment lengths for routing | [Network Analysis — Road Service Area](network-analysis.md) | `whitebox_workflows:add_geometry_attributes` |
| Delineate road service area | [Network Analysis — Road Service Area](network-analysis.md) | QGIS `native:serviceareafromlayer` |

---

## Linear Referencing

| Task | Chapter section | Key tools |
|------|----------------|-----------|
| Add measure (M) values to route network | [Linear Referencing — Step 1](linear-referencing.md) | QGIS `native:setmvalue` |
| Validate unique route IDs | [Linear Referencing — Step 2](linear-referencing.md) | QGIS Python Console check |
| Locate point events along routes | [Linear Referencing — Step 3](linear-referencing.md) | `whitebox_workflows:locate_point_events` |
| Locate line events along routes | [Linear Referencing — Step 4](linear-referencing.md) | `whitebox_workflows:locate_line_events` |
| Calibrate M-values against control points | [Linear Referencing — Calibrate](linear-referencing.md) | `whitebox_workflows:calibrate_route` |

---

## By Data Type

### Raster input tasks
- Fill depressions → see [Terrain Analysis](terrain-analysis.md) / [Spatial Hydrology](spatial-hydrology.md)
- Slope, aspect, curvature → see [Terrain Analysis](terrain-analysis.md)
- Flow direction, accumulation → see [Spatial Hydrology](spatial-hydrology.md)
- Reclassification, suitability → see [Raster Analysis](raster-analysis.md)
- Spectral indices, PCA, change → see [Remote Sensing](remote-sensing.md)

### Point cloud input tasks
- Ground classification, DTM/DSM/CHM → see [LiDAR Processing](lidar-processing.md)
- Height normalisation → see [LiDAR Processing](lidar-processing.md)

### Vector input tasks
- Geometry validation, overlay, joins → see [Vector Analysis](vector-analysis.md)
- Network ordering, service areas → see [Network Analysis](network-analysis.md)
- Route events, calibration → see [Linear Referencing](linear-referencing.md)
