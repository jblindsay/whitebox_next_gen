# Vector Analysis

Vector data are the primary format for discrete geographic features — points, lines, and polygons representing everything from sample locations and road networks to property parcels and watershed boundaries. Whitebox Workflows for Python (WbW-Py) provides a comprehensive set of vector analysis tools covering attribute management, geometric measurement, spatial overlay, proximity analysis, topology repair, shape analysis, and vector-to-raster conversion.

---

## Core Concepts

Vector analysis depends on understanding these core concepts:

- **Feature geometry**: Points (single coordinate pairs), lines (ordered sequences of coordinate pairs), and polygons (rings of coordinates forming closed boundaries). Each feature type supports different analyses.
- **Topology**: The spatial relationships between features (adjacency, containment, intersection). Topological errors (overshoots, undershoots, self-intersections) corrupt overlay operations and topology queries.
- **Attribute table**: The database associated with each feature layer, carrying descriptive fields and values. Attribute queries filter features; joins link external tables.
- **Spatial index**: Internal index structure (R-tree or quadtree) enabling fast spatial queries. Used by intersection, containment, and proximity operations. Always build spatial indices on frequently queries layers.
- **Envelope (bounding box)**: The minimum rectangular boundary of a feature or layer; used for quick spatial culling before geometry tests.
- **Buffer**: A polygon created at a fixed distance around a feature. Buffers model proximity zones and are fundamental to distance-based analysis.
- **Overlay (intersection, union, difference)**: Combining two polygon layers to create a new layer. Union merges boundaries; intersection keeps overlapping area only; difference removes one layer from another.
- **Dissolve (aggregation)**: Merging adjacent features with identical attribute values, creating larger aggregate features. Reduces feature count and simplifies geometry.
- **Spatial join**: Associating features from one layer with features from another based on spatial relationship (overlap, containment, proximity). Assigns attributes across layers.
- **Proximity analysis**: Finding nearest features, distances, or connectivity. Foundation for network analysis, market analysis, and accessibility studies.

---

## Reading and Writing Vectors

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
wbe.working_directory = '/data/vectors'

# Read a single vector
watersheds = wbe.read_vector('watersheds.shp')
streams    = wbe.read_vector('streams.shp')
outlets    = wbe.read_vector('outlets.shp')

# Read multiple at once
[roads, buildings, parks] = wbe.read_vectors('roads.shp', 'buildings.shp', 'parks.shp')

# Write results
wbe.write_vector(watersheds, 'watersheds_processed.shp')
```

---

## Attribute Table Management

### Adding and Removing Fields

```python
# Add a new numeric field
watersheds = wbe.add_field(watersheds, field_name='AREA_KM2', field_type='Float')

# Rename a field
watersheds = wbe.rename_field(watersheds,
                               old_field_name='OBJECTID',
                               new_field_name='WS_ID')

# Delete a field
watersheds = wbe.delete_field(watersheds, field_name='TEMP_FIELD')

# Reset the entire attribute table to only an FID column
watersheds = wbe.reinitialize_attribute_table(watersheds)
```

### Filtering by Attribute

```python
# Select features where upstream area exceeds 50 km²
large_ws = wbe.extract_by_attribute(watersheds,
                                     field_name='AREA_KM2',
                                     operator='>',
                                     value=50.0)
wbe.write_vector(large_ws, 'large_watersheds.shp')
```

### Joining Tables

```python
# Merge a CSV attribute table into a vector by a shared key field
import csv
merged = wbe.merge_table_with_csv(watersheds,
                                   csv_file='watershed_stats.csv',
                                   join_field='WS_ID')
```

### Exporting the Attribute Table

```python
wbe.export_table_to_csv(watersheds, 'watershed_attributes.csv')
```

### Listing Unique Values

```python
wbe.list_unique_values(watersheds, field_name='REGION')
```

---

## Geometric Measurement

### Polygon Area and Perimeter

```python
# Compute polygon area (adds AREA field to attribute table)
watersheds = wbe.polygon_area(watersheds)

# Compute perimeter (adds PERIMETER field)
watersheds = wbe.polygon_perimeter(watersheds)
```

### Shape Indices

Shape indices quantify the geometric complexity and elongation of polygon features. They are widely used in ecology (patch metrics), hydrology (watershed form), and urban analysis:

```python
# Compactness ratio — measures how closely a polygon approximates a circle
# Perfectly circular = 1.0; lower values = more elongated
watersheds = wbe.compactness_ratio(watersheds)

# Elongation ratio — based on minimum bounding box dimensions
watersheds = wbe.elongation_ratio(watersheds)

# Linearity index — R² of an RMA regression through hull vertices
# Higher values indicate long, narrow linear shapes
watersheds = wbe.linearity_index(watersheds)

# Related circumscribing circle
watersheds = wbe.related_circumscribing_circle(watersheds)

# Boundary shape complexity
watersheds = wbe.boundary_shape_complexity(watersheds)

# Hole proportion — fraction of polygon area that is holes
watersheds = wbe.hole_proportion(watersheds)

# Shape complexity (vector)
watersheds = wbe.shape_complexity_index_vector(watersheds)

# Patch orientation (degrees from north of long axis)
watersheds = wbe.patch_orientation(watersheds)

# Narrowness index
watersheds = wbe.narrowness_index(watersheds)

# Radius of gyration (area-weighted centroid distance)
watersheds = wbe.radius_of_gyration(watersheds)
```

### Point Coordinate Addition

```python
# Add X, Y (and optionally Z) coordinate columns to a point vector
sample_pts = wbe.add_point_coordinates_to_table(sample_pts)
```

---

## Centroids, Bounding Boxes, and Convex Hulls

```python
# Point centroid of each polygon
centroids = wbe.centroid_vector(watersheds)
wbe.write_vector(centroids, 'watershed_centroids.shp')

# Minimum bounding box for each polygon
bboxes = wbe.minimum_bounding_box(watersheds)

# Minimum bounding circle
circles = wbe.minimum_bounding_circle(watersheds)

# Minimum bounding envelope (overall)
envelope = wbe.minimum_bounding_envelope(watersheds)

# Minimum convex hull
hull = wbe.minimum_convex_hull(watersheds)

# Layer footprint (bounding box of entire layer)
footprint = wbe.layer_footprint_vector(watersheds)

# Long axis and short axis lines
long_axis  = wbe.polygon_long_axis(watersheds)
short_axis = wbe.polygon_short_axis(watersheds)
```

---

## Smoothing, Simplification, and Geometry Operations

```python
# Smooth vertices by averaging — reduces digitising artefacts
smooth = wbe.smooth_vectors(streams, filter_size=5)

# Douglas-Peucker line simplification
simplified = wbe.simplify_features(streams, snap_distance=10.0)

# Split long lines into segments of maximum length
segmented = wbe.split_vector_lines(streams, segment_length=1000.0)

# Extend line endpoints by a specified distance
extended = wbe.extend_vector_lines(streams, dist=50.0, extend_type='both')

# Split polygons or lines using another line layer
split_polys = wbe.split_with_lines(watersheds, split_lines=roads)

# Lines to polygon conversion (close and fill each polyline)
polys_from_lines = wbe.lines_to_polygons(outline_lines)

# Polygons to lines (extract boundary lines)
lines_from_polys = wbe.polygons_to_lines(watersheds)

# Convert multipart features to singlepart
single = wbe.multipart_to_singlepart(watersheds)

# Convert singlepart to multipart by shared attribute value
multi = wbe.singlepart_to_multipart(parcels, field_name='OWNER_ID')

# Merge all features in two or more files into one layer
merged_streams = wbe.merge_vectors(streams_a, streams_b)

# Clean topology (remove duplicate vertices and degenerate features)
cleaned = wbe.clean_vector(streams)

# Remove polygon holes smaller than a threshold
no_holes = wbe.remove_polygon_holes(watersheds)
```

---

## Spatial Overlay

WbW-Py supports the full suite of vector set-theoretic overlay operations:

### Clip

Cuts one layer to the extent of another, retaining only features within the clip polygon:

```python
# Clip roads to a study area polygon
roads_clipped = wbe.clip(input=roads, clip=study_area)
wbe.write_vector(roads_clipped, 'roads_study_area.shp')
```

### Intersect

Returns the geometric intersection of two layers, keeping the portions where they overlap and combining attributes from both:

```python
soil_in_watershed = wbe.intersect(input=soil_polygons, overlay=watershed_boundary,
                                   snap_tolerance=1e-6)
wbe.write_vector(soil_in_watershed, 'soil_in_watershed.shp')
```

### Erase (Difference)

Removes the area of one layer from another:

```python
# Remove urban areas from the vegetation layer
rural_veg = wbe.erase(input=vegetation, erase_layer=urban_boundaries)
```

### Union

Combines two polygon layers and divides overlapping areas, retaining all features from both:

```python
combined = wbe.union(input=zoning, overlay=flood_zones)
wbe.write_vector(combined, 'zoning_flood_overlay.shp')
```

### Symmetrical Difference

Returns only the non-overlapping portions of each layer:

```python
sym_diff = wbe.symmetrical_difference(input=year1_polygons, overlay=year2_polygons)
```

### Dissolve

Merges features that share a common attribute value:

```python
# Merge all polygons of the same land-cover class
dissolved = wbe.dissolve(input=landcover_polygons, field_name='CLASS')
wbe.write_vector(dissolved, 'landcover_dissolved.shp')
```

---

## Proximity and Near Analysis

### Euclidean Distance to Nearest Feature

```python
# Find the distance from each sample point to the nearest road
near_result = wbe.near(input=sample_pts, feature=roads)
# Adds NEAR_DIST and NEAR_FID fields to sample_pts
wbe.write_vector(near_result, 'samples_near_roads.shp')
```

### Voronoi Diagram (Thiessen Polygons)

Thiessen polygons partition space so every location is assigned to the nearest source point:

```python
voronoi = wbe.voronoi_diagram(sample_pts)
wbe.write_vector(voronoi, 'thiessen_polygons.shp')
```

### Convex Hull and Medoid

```python
hull_pts = wbe.minimum_convex_hull(sample_pts)  # minimum convex hull of point set
med      = wbe.medoid(sample_pts)               # geometric median of a set of points
```

---

## Select by Location

Spatial queries allow selection of features based on their geometric relationship to a second layer:

```python
# Select all stream segments that intersect wetland polygons
streams_in_wetlands = wbe.select_by_location(
    input=streams,
    comparison=wetlands,
    geometry_type='intersects'
)
wbe.write_vector(streams_in_wetlands, 'streams_in_wetlands.shp')
```

---

## Spatial Join

Spatial join transfers attributes from a join layer to an input layer based on spatial proximity or overlap:

```python
# Join soil class to sample points based on the polygon they fall within
pts_with_soil = wbe.spatial_join(
    input=sample_pts,
    join_layer=soil_polygons,
    join_type='within',       # 'within', 'intersects', 'nearest'
    strategy='first',         # 'first', 'last', 'count', 'sum', 'mean', 'min', 'max'
    field_name='SOIL_CLASS'
)
wbe.write_vector(pts_with_soil, 'samples_with_soil.shp')
```

---

## Vector Grids

Create regular grids of vector polygons covering a raster or vector extent. Useful for stratified sampling and landscape analysis at fixed spatial scales:

```python
# Hexagonal grid with resolution based on an existing raster
hex_grid = wbe.hexagonal_grid_from_raster_base(dem)
wbe.write_vector(hex_grid, 'hexgrid.shp')

# Rectangular grid
rec_grid = wbe.rectangular_grid_from_raster_base(dem)
wbe.write_vector(rec_grid, 'recgrid.shp')

# Hexagonal grid with resolution based on a vector extent
hex_v = wbe.hexagonal_grid_from_vector_base(watersheds, width=500.0)
```

---

## Vector-to-Raster Conversion

```python
# Rasterize polygon layer (burn polygon attribute value into grid cells)
lc_raster = wbe.vector_polygons_to_raster(
    input=landcover_polygons,
    field_name='CLASS_ID',
    cell_size=30.0
)
wbe.write_raster(lc_raster, 'landcover_raster.tif')

# Rasterize line layer
roads_raster = wbe.vector_lines_to_raster(
    input=roads,
    field_name='FID',
    cell_size=10.0
)

# Rasterize point layer
pts_raster = wbe.vector_points_to_raster(
    input=sample_pts,
    field_name='YIELD',
    assign_op='mean',  # 'first', 'last', 'min', 'max', 'sum', 'mean', 'number'
    cell_size=5.0
)
```

---

## Field Calculator

The field calculator tool evaluates an expression against the attribute table to create or update a field:

```python
# Compute a normalised shape index from existing area and perimeter fields
watersheds = wbe.field_calculator(
    input=watersheds,
    field_name='SHAPE_IDX',
    expression="'PERIMETER' / (2.0 * 3.14159 * sqrt('AREA' / 3.14159))"
)
```

---

## Topological Utilities

### Repair and Validation

```python
# Snap nearby line endpoints to within a tolerance distance
streams_clean = wbe.repair_stream_vector_topology(streams, snap_distance=1.0)

# Fix dangling arcs (lines that overshoot or undershoot intersections)
fixed = wbe.fix_dangling_arcs(streams, snap_distance=1.0)
```

### Line Intersections

```python
# Find all intersection points between two line layers
intersections = wbe.line_intersections(roads, rivers)
wbe.write_vector(intersections, 'road_river_crossings.shp')
```

### Extract Nodes

```python
# Extract all vertices of a line layer as points
nodes = wbe.extract_nodes(streams)
wbe.write_vector(nodes, 'stream_nodes.shp')
```

---

## Polygon Topology

```python
# Polygonise a raster — convert raster regions to vector polygons
polys = wbe.polygonize(classified_raster)
wbe.write_vector(polys, 'class_polygons.shp')
```

---

## Point Cluster Analysis

```python
# Heat map (kernel density estimation)
density = wbe.heat_map(sample_pts, bandwidth=500.0)
wbe.write_raster(density, 'point_density.tif')

# Vector hexagonal binning — count points per hexagon
hex_counts = wbe.vector_hex_binning(sample_pts, width=1000.0, orientation='vertical')
wbe.write_vector(hex_counts, 'hex_counts.shp')
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

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()

result = wbe.run_tool(
    'market_access_and_site_intelligence_workflow',
    {
        'network': 'street_network.shp',
        'sites_existing': 'existing_sites.shp',
        'sites_candidates': 'candidate_sites.shp',
        'demand_surface': 'demand_points.shp',
        'ring_costs': [5.0, 10.0, 15.0],
        'catchments_output': 'candidate_catchments.shp',
        'overlap_analysis_output': 'competitive_overlap.shp',
        'candidate_rank_csv': 'candidate_rankings.csv',
        'executive_summary_json': 'market_summary.json'
    }
)
print(result)
```

> **Note:** This workflow requires a `WbEnvironment` initialized with a valid
> Pro licence.

---

## Complete Vector Analysis Workflow

The following script illustrates a full workflow from raw survey points to a clipped, dissolved, and enriched polygon layer:

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
wbe.working_directory = '/data/vector_analysis'
wbe.verbose = True

# 1. Load layers
parcels       = wbe.read_vector('parcels.shp')
study_boundary = wbe.read_vector('study_boundary.shp')
soil_map      = wbe.read_vector('soil_types.shp')
sample_pts    = wbe.read_vector('sample_points.shp')

# 2. Clip parcels to study boundary
parcels_clip = wbe.clip(input=parcels, clip=study_boundary)

# 3. Compute geometric attributes
parcels_clip = wbe.polygon_area(parcels_clip)
parcels_clip = wbe.polygon_perimeter(parcels_clip)
parcels_clip = wbe.compactness_ratio(parcels_clip)

# 4. Spatial join — assign soil type to each parcel
parcels_with_soil = wbe.spatial_join(
    input=parcels_clip,
    join_layer=soil_map,
    join_type='intersects',
    strategy='first',
    field_name='SOIL_CODE'
)

# 5. Dissolve by soil code to get soil extents within study area
soil_dissolved = wbe.dissolve(input=parcels_with_soil, field_name='SOIL_CODE')
wbe.write_vector(soil_dissolved, 'soil_study_area.shp')

# 6. Spatial join sample points with soil polygons
samples_enriched = wbe.spatial_join(
    input=sample_pts,
    join_layer=soil_dissolved,
    join_type='within',
    strategy='first',
    field_name='SOIL_CODE'
)
samples_enriched = wbe.add_point_coordinates_to_table(samples_enriched)

# 7. Export for external analysis
wbe.export_table_to_csv(samples_enriched, 'samples_with_soil.csv')
print('Vector analysis pipeline complete.')
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
