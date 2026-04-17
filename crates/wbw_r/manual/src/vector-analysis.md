# Vector Analysis

Vector GIS analysis in WbW-R covers attribute management, geometric measurement, shape analysis, spatial overlay, proximity tools, spatial joins, and vector-to-raster conversion. All computation runs in the Whitebox backend through `wbw_run_tool()`; R handles session management, sequencing, and result processing.

---

## Core Concepts

Vector analysis depends on understanding these core concepts:

- **Feature geometry**: Points (single coordinate pairs), lines (ordered sequences of coordinate pairs), and polygons (rings of coordinates forming closed boundaries). Each feature type supports different analyses.
- **Topology**: The spatial relationships between features (adjacency, containment, intersection). Topological errors (overshoots, undershoots, self-intersections) corrupt overlay operations and topology queries.
- **Attribute table**: The database associated with each feature layer, carrying descriptive fields and values. Attribute queries filter features; joins link external tables.
- **Spatial index**: Internal index structure (R-tree or quadtree) enabling fast spatial queries. Used by intersection, containment, and proximity operations. Always build spatial indices on frequently queried layers.
- **Envelope (bounding box)**: The minimum rectangular boundary of a feature or layer; used for quick spatial culling before geometry tests.
- **Buffer**: A polygon created at a fixed distance around a feature. Buffers model proximity zones and are fundamental to distance-based analysis.
- **Overlay (intersection, union, difference)**: Combining two polygon layers to create a new layer. Union merges boundaries; intersection keeps overlapping area only; difference removes one layer from another.
- **Dissolve (aggregation)**: Merging adjacent features with identical attribute values, creating larger aggregate features. Reduces feature count and simplifies geometry.
- **Spatial join**: Associating features from one layer with features from another based on spatial relationship (overlap, containment, proximity). Assigns attributes across layers.
- **Proximity analysis**: Finding nearest features, distances, or connectivity. Foundation for network analysis, market analysis, and accessibility studies.

---

## Session Setup

```r
library(whiteboxworkflows)

s <- wbw_session()
setwd('/data/vector')
```

---

## Reading and Writing Vectors

```r
polys  <- wbw_read_vector('parcels.shp')
lines  <- wbw_read_vector('roads.shp')
points <- wbw_read_vector('samples.shp')

meta <- polys$metadata()
cat('Feature count:', meta$num_features, '\n')
cat('Geometry type:', meta$geom_type, '\n')
cat('CRS:', meta$wkt, '\n')
```

---

## Attribute Management

```r
# Add a new field
wbw_run_tool('add_field', args = list(
  i         = polys$file_path(),
  output    = 'parcels_v2.shp',
  field_name = 'AREA_HA',
  field_type = 'Float'
), session = s)

# Delete an unwanted field
wbw_run_tool('delete_field', args = list(
  i          = 'parcels_v2.shp',
  output     = 'parcels_v3.shp',
  field_name = 'OLD_FIELD'
), session = s)

# Rename a field
wbw_run_tool('rename_field', args = list(
  i             = 'parcels_v3.shp',
  output        = 'parcels_v4.shp',
  input_field   = 'AREA_HA',
  output_field  = 'HECTARES'
), session = s)

# Extract features by attribute value
wbw_run_tool('extract_by_attribute', args = list(
  i            = polys$file_path(),
  output       = 'large_parcels.shp',
  field        = 'AREA_M2',
  filter_value = 10000.0,
  filter_type  = 'Greater Than'
), session = s)
```

---

## Geometric Measurements

```r
# Add area, perimeter, and basic shape metrics to polygon attribute table
wbw_run_tool('add_polygon_coordinates_to_table', args = list(
  i      = polys$file_path(),
  output = 'parcels_geom.shp'
), session = s)

# Polygon shape index — compactness
wbw_run_tool('compactness_ratio', args = list(
  i = polys$file_path(), output = 'parcels_compact.shp'), session = s)

wbw_run_tool('elongation_ratio', args = list(
  i = polys$file_path(), output = 'parcels_elong.shp'), session = s)

wbw_run_tool('related_circumscribing_circle', args = list(
  i = polys$file_path(), output = 'parcels_rcc.shp'), session = s)

wbw_run_tool('patch_orientation', args = list(
  i = polys$file_path(), output = 'parcels_orient.shp'), session = s)

wbw_run_tool('radius_of_gyration', args = list(
  i = polys$file_path(), output = 'parcels_rog.shp'), session = s)
```

---

## Geometric Operations

```r
# Centroids
wbw_run_tool('centroid_vector', args = list(
  i = polys$file_path(), output = 'centroids.shp'), session = s)

# Convex hull
wbw_run_tool('convex_hull', args = list(
  i = polys$file_path(), output = 'convex_hulls.shp'), session = s)

# Minimum bounding envelopes
wbw_run_tool('minimum_bounding_envelope', args = list(
  i = polys$file_path(), output = 'mbe.shp'), session = s)

# Smooth vector polygons
wbw_run_tool('smooth_vectors', args = list(
  i = polys$file_path(), output = 'parcels_smooth.shp', filter = 5), session = s)

# Simplify features (Douglas-Peucker)
wbw_run_tool('simplify_line_or_polygon', args = list(
  i = polys$file_path(), output = 'parcels_simplified.shp',
  dist = 5.0, remove_spurs = TRUE, errors_only = FALSE), session = s)

# Dissolve polygons on field value
wbw_run_tool('dissolve', args = list(
  i = polys$file_path(), output = 'parcels_dissolved.shp',
  field = 'LAND_USE', snap_tol = 0.001), session = s)
```

---

## Spatial Overlay

```r
# Clip
wbw_run_tool('clip', args = list(
  i        = polys$file_path(),
  clip     = 'study_area.shp',
  output   = 'clipped.shp',
  snap_tol = 0.001
), session = s)

# Intersect
wbw_run_tool('intersect', args = list(
  i        = polys$file_path(),
  overlay  = 'zones.shp',
  output   = 'intersection.shp',
  snap_tol = 0.001
), session = s)

# Erase
wbw_run_tool('erase', args = list(
  i        = polys$file_path(),
  erase    = 'exclusion_areas.shp',
  output   = 'erased.shp',
  snap_tol = 0.001
), session = s)

# Union
wbw_run_tool('union', args = list(
  i        = polys$file_path(),
  overlay  = 'other_layer.shp',
  output   = 'union.shp',
  snap_tol = 0.001
), session = s)

# Symmetrical difference
wbw_run_tool('symmetrical_difference', args = list(
  i        = polys$file_path(),
  overlay  = 'other_layer.shp',
  output   = 'symdiff.shp',
  snap_tol = 0.001
), session = s)
```

---

## Proximity Analysis

```r
# Euclidean distance from vector features
wbw_run_tool('vector_points_to_raster', args = list(
  i = points$file_path(), output = 'points.tif', field = 'FID',
  assign = 'last', nodata = TRUE, cell_size = 5.0, base = 'dem.tif'), session = s)
wbw_run_tool('euclidean_distance', args = list(
  i = 'points.tif', output = 'euclidean_dist.tif'), session = s)

# Voronoi diagram
wbw_run_tool('voronoi_diagram', args = list(
  i = points$file_path(), output = 'voronoi.shp'), session = s)
```

---

## Select by Location

```r
wbw_run_tool('select_by_location', args = list(
  input   = polys$file_path(),
  select  = 'stream_buffer.shp',
  output  = 'parcels_near_streams.shp',
  condition = 'within'
), session = s)
```

---

## Spatial Join

```r
wbw_run_tool('spatial_join', args = list(
  target  = points$file_path(),
  join    = polys$file_path(),
  output  = 'points_joined.shp',
  condition = 'within',
  attr    = 'first'
), session = s)
```

### Aggregation Strategies

```r
# Join and aggregate field values from nearest polygon
for (stat in c('count', 'sum', 'mean', 'min', 'max')) {
  wbw_run_tool('spatial_join', args = list(
    target    = 'zones.shp',
    join      = 'observations.shp',
    output    = paste0('zones_', stat, '.shp'),
    condition = 'contains',
    attr      = stat
  ), session = s)
}
```

---

## Vector-to-Raster Conversion

```r
# Rasterize polygon layer
wbw_run_tool('vector_polygons_to_raster', args = list(
  i = polys$file_path(), output = 'parcels_raster.tif',
  field = 'LAND_USE_ID', nodata = TRUE, cell_size = 5.0, base = 'dem.tif'), session = s)

# Rasterize line layer
wbw_run_tool('vector_lines_to_raster', args = list(
  i = lines$file_path(), output = 'roads_raster.tif',
  field = 'FID', nodata = TRUE, cell_size = 5.0, base = 'dem.tif'), session = s)

# Rasterize points
wbw_run_tool('vector_points_to_raster', args = list(
  i = points$file_path(), output = 'points_raster.tif',
  field = 'VALUE', assign = 'max', nodata = TRUE, cell_size = 5.0), session = s)
```

---

## Field Calculator

```r
# Compute area in hectares and write to existing field
wbw_run_tool('field_calculator', args = list(
  i         = polys$file_path(),
  output    = 'parcels_calc.shp',
  field_name = 'AREA_HA',
  py_statement = '@Area / 10000.0',
  analyse   = FALSE
), session = s)
```

---

## Point Cluster Analysis

```r
# Kernel density estimation (heat map)
wbw_run_tool('kernel_density_estimation', args = list(
  i         = points$file_path(),
  output    = 'heatmap.tif',
  bandwidth = 200.0,
  kernel_type = 'quartic',
  cell_size = 5.0,
  base      = 'dem.tif'
), session = s)

# Hexagonal binning
wbw_run_tool('create_hexagonal_vector_grid', args = list(
  i = 'study_area.shp', output = 'hex_grid.shp', width = 500.0, orientation = 'horizontal'), session = s)
wbw_run_tool('spatial_join', args = list(
  target = 'hex_grid.shp', join = points$file_path(),
  output = 'hex_counts.shp', condition = 'contains', attr = 'count'), session = s)
```

---

## WbW-Pro Spotlight: Market Access and Site Intelligence

- **Problem:** Rank candidate sites using repeatable network-access and demand
  logic.
- **Tool:** `market_access_and_site_intelligence_workflow`
- **Typical inputs:** Network, existing sites, candidate sites, demand
  surface, drive-time rings.
- **Typical outputs:** Catchment polygons, competitive-overlap layer,
  candidate-ranking CSV, executive summary JSON.

```r
result <- s$run_tool(
  'market_access_and_site_intelligence_workflow',
  list(
    network                 = 'street_network.shp',
    sites_existing          = 'existing_sites.shp',
    sites_candidates        = 'candidate_sites.shp',
    demand_surface          = 'demand_points.shp',
    ring_costs              = c(5.0, 10.0, 15.0),
    catchments_output       = 'candidate_catchments.shp',
    overlap_analysis_output = 'competitive_overlap.shp',
    candidate_rank_csv      = 'candidate_rankings.csv',
    executive_summary_json  = 'market_summary.json'
  )
)

print(result)
```

> **Note:** This workflow requires a session initialized with a valid Pro
> licence.

---

## Complete Vector Analysis Workflow

```r
library(whiteboxworkflows)

s <- wbw_session()
setwd('/data/vector_workflow')

parcels <- wbw_read_vector('parcels.shp')
study   <- wbw_read_vector('study_area.shp')
streams <- wbw_read_vector('streams.shp')

# 1. Clip to study area
wbw_run_tool('clip', args = list(
  i = parcels$file_path(), clip = study$file_path(),
  output = 'parcels_clipped.shp', snap_tol = 0.001), session = s)

# 2. Add shape metrics
wbw_run_tool('compactness_ratio',
  args = list(i = 'parcels_clipped.shp', output = 'parcels_shape.shp'), session = s)

# 3. Buffer streams and intersect with parcels
wbw_run_tool('buffer_raster', args = list(
  i = streams$file_path(), output = 'stream_buf.shp',
  size = 30.0, gridcells = FALSE), session = s)
wbw_run_tool('intersect', args = list(
  i = 'parcels_shape.shp', overlay = 'stream_buf.shp',
  output = 'riparian_parcels.shp', snap_tol = 0.001), session = s)

# 4. Dissolve by land-use class
wbw_run_tool('dissolve', args = list(
  i = 'riparian_parcels.shp', output = 'riparian_dissolved.shp',
  field = 'LAND_USE', snap_tol = 0.001), session = s)

# 5. Rasterize result
wbw_run_tool('vector_polygons_to_raster', args = list(
  i = 'riparian_dissolved.shp', output = 'riparian.tif',
  field = 'LAND_USE_ID', nodata = TRUE, cell_size = 5.0), session = s)

cat('Vector analysis complete.\n')
```

---

## Tips

- **Always validate topology before analysis**: Run `check_vector_topology()` to detect overshoots, undershoots, self-intersections, and sliver polygons. Topological errors propagate through overlay and spatial join operations.
- **Build spatial indices on large layers**: Large datasets (> 10,000 features) benefit from spatial indexing. Use `build_spatial_index()` explicitly before repeated spatial queries; operations like containment or proximity are fast with indices.
- **Choose your overlay operation carefully**: Union retains all boundaries and combines attributes (can create many small slivers). Intersection keeps only overlapping regions. Difference retains Polygon A minus Polygon B. Test on small subsets first.
- **Dissolve reduces feature count and file size**: After overlay, dissolve by ownership or category to collapse unnecessary edges. Dissolved layers render faster and are cleaner for publication.
- **Spatial joins are sensitive to alignment**: Ensure both input layers use the same CRS and are free of topology errors. Reproject to equal-area projection before computing buffer distances or areas for analysis.
- **Buffer distance and units matter**: Buffer distances are in map units (meters, feet, degrees). Use an equal-area projection if precise areas or distances are critical. Negative buffers can collapse small polygons (inset); test with small buffer values first.
- **Attribute table size is a memory constraint**: Attribute tables with millions of rows and dozens of fields consume RAM. Export to CSV or database for large tables; work with summaries or samples when memory is limited.
- **Point-in-polygon operations scale with complexity**: Containment tests are O(n) per point; on large datasets (> 1 million points), consider spatial index binning or vector-to-raster conversion for speed.
```
