# Vector/GIS Tools Documentation Audit
## Whitebox OSS - Tool Registry Analysis

Generated: 2026-06-04

---

## Summary

Total Vector/GIS Tools Found: **20+ major vector operations** extracted from `register_default_tools()` in [lib.rs](crates/wbtools_oss/src/lib.rs).

---

## Tools Organized by Functional Category

### 1. SPATIAL OPERATIONS (Proximity & Analysis)

| Tool ID | Display Name | Struct Name | Metadata Summary | Manifest Summary |
|---------|-------------|------------|-----------------|-----------------|
| `buffer_vector` | Buffer Vector | BufferVectorTool | Creates polygon buffers around point, line, and polygon vector geometries with configurable cap and join styles. | Creates polygon buffers around point, line, and polygon vector geometries with configurable cap and join styles. |
| `near` | Near | NearTool | Finds the nearest feature in a near layer and writes NEAR_FID and NEAR_DIST attributes. | Finds the nearest feature in a near layer and writes NEAR_FID and NEAR_DIST attributes. |
| `select_by_location` | Select By Location | SelectByLocationTool | Extracts target features that satisfy a spatial relationship to query features. | Extracts target features that satisfy a spatial relationship to query features. |
| `spatial_join` | Spatial Join | SpatialJoinTool | Joins attributes from a join layer onto target features using a spatial predicate. | Joins attributes from a join layer onto target features using a spatial predicate. |

**Key Features:**
- `buffer_vector`: Configurable cap/join styles (round, flat, square; bevel, mitre), dissolve option, arc resolution control
- `near`: Supports optional max_distance filtering
- `select_by_location`: Multiple predicates (intersects, within, contains, touches, crosses, overlaps, disjoint, within_distance)
- `spatial_join`: Multiple strategies (first, last, count, sum, mean, min, max), field prefix customization

---

### 2. GEOMETRY OPERATIONS (Simplification, Smoothing, Densification)

| Tool ID | Display Name | Struct Name | Metadata Summary | Manifest Summary |
|---------|-------------|------------|-----------------|-----------------|
| `simplify_features` | Simplify Features | SimplifyFeaturesTool | Simplifies vector geometries using Douglas-Peucker tolerance. | Simplifies vector geometries using Douglas-Peucker tolerance. |
| `smooth_vectors` | Smooth Vectors | SmoothVectorsTool | Smooths polyline or polygon vectors using a moving-average filter. | Smooths polyline or polygon vectors using a moving-average filter. |
| `densify_features` | Densify Features | DensifyFeaturesTool | Adds vertices along line and polygon boundaries at a specified spacing. | Adds vertices along line and polygon boundaries at a specified spacing. |

**Key Features:**
- `simplify_features`: Douglas-Peucker tolerance-based simplification
- `smooth_vectors`: Configurable filter window size (odd, ≥3)
- `densify_features`: Maximum vertex spacing control

---

### 3. GEOMETRY EXTRACTION & POINT OPERATIONS

| Tool ID | Display Name | Struct Name | Metadata Summary | Manifest Summary |
|---------|-------------|------------|-----------------|-----------------|
| `centroid_vector` | Centroid Vector | CentroidVectorTool | Computes centroid points from vector features. | Computes centroid points from vector features. |
| `representative_point_vector` | Representative Point Vector | RepresentativePointVectorTool | Computes representative points guaranteed to lie on or within input geometries. | Computes representative points guaranteed to lie on or within input geometries. |
| `points_along_lines` | Points Along Lines | PointsAlongLinesTool | Creates regularly spaced point features along input line geometries. | Creates regularly spaced point features along input line geometries. |

**Key Features:**
- `representative_point_vector`: Guarantees point location within/on geometry (more robust than centroid for complex shapes)
- `points_along_lines`: Configurable spacing, optional endpoint inclusion

---

### 4. INTEROP & FORMAT CONVERSION

| Tool ID | Display Name | Struct Name | Metadata Summary | Manifest Summary |
|---------|-------------|------------|-----------------|-----------------|
| `reproject_vector` | Reproject Vector | ReprojectVectorTool | Reprojects an input vector layer to a destination EPSG code. | Reprojects an input vector layer to a destination EPSG code. |
| `add_geometry_attributes` | Add Geometry Attributes | AddGeometryAttributesTool | Adds area, length, perimeter, and centroid attributes to vector features. | Adds area, length, perimeter, and centroid attributes to vector features. |

**Key Features:**
- `reproject_vector`: EPSG code-based reprojection
- `add_geometry_attributes`: Multi-mode measurement (auto, planar, geodesic), selective attribute output

---

### 5. ATTRIBUTE/SCHEMA MANAGEMENT

| Tool ID | Display Name | Struct Name | Metadata Summary | Manifest Summary |
|---------|-------------|------------|-----------------|-----------------|
| `field_calculator` | Field Calculator | FieldCalculatorTool | Calculates a field value from an expression using feature attributes and geometry variables; supports SQL-style CASE, CAST, null checks, and UPDATE ... SET ... [WHERE ...] wrappers. | Calculates a field value from an expression using feature attributes and geometry variables; supports SQL-style CASE, CAST, null checks, and UPDATE ... SET ... [WHERE ...] wrappers. |
| `add_field` | Add Field | AddFieldTool | Adds a new attribute field with an optional default value. | Adds a new attribute field with an optional default value. |
| `delete_field` | Delete Field | DeleteFieldTool | Deletes one or more attribute fields from a vector layer. | Deletes one or more attribute fields from a vector layer. |
| `rename_field` | Rename Field | RenameFieldTool | Renames an attribute field in a vector layer. | Renames an attribute field in a vector layer. |

**Key Features:**
- `field_calculator`: Expression evaluation with SQL-style CASE/CAST, geometry variable support ($area, etc.), preview/output modes
- `delete_field`: Comma-delimited multi-field deletion
- `add_field`: Type control (integer, float, text, boolean), optional defaults

---

### 6. CLIPPING & GEOMETRIC FILTERING

| Tool ID | Display Name | Struct Name | Metadata Summary | Manifest Summary |
|---------|-------------|------------|-----------------|-----------------|
| `line_polygon_clip` | Line Polygon Clip | LinePolygonClipTool | Clips line features to polygon interiors and outputs clipped line segments. | Clips line features to polygon interiors and outputs clipped line segments. |

**Key Features:**
- True segment clipping (not just intersection filtering)
- Outputs clipped line segments within polygon interiors

---

### 7. POLYGON/HULL OPERATIONS

| Tool ID | Display Name | Struct Name | Metadata Summary | Manifest Summary |
|---------|-------------|------------|-----------------|-----------------|
| `concave_hull` | Concave Hull | ConcaveHullTool | Creates concave hull polygons around input feature coordinates using the concaveman algorithm. | Creates concave hull polygons around input feature coordinates using the concaveman algorithm. |
| `voronoi_diagram` | Voronoi Diagram | VoronoiDiagramTool | Creates Voronoi (Thiessen) polygons from input point locations. | Creates Voronoi (Thiessen) polygons from input point locations. |
| `polygonize` | Polygonize | PolygonizeTool | Creates polygons from input linework, including intersecting/open segments where enclosed faces can be formed. | Creates polygons from input linework, including intersecting/open segments where enclosed faces can be formed. |

**Key Features:**
- `concave_hull`: Concavity ratio control, max edge length threshold, robustness epsilon
- `voronoi_diagram`: Input point/multipoint support
- `polygonize`: Multi-input support (array or semicolon/comma-delimited), snap tolerance

---

### 8. RANDOM & SAMPLING OPERATIONS

| Tool ID | Display Name | Struct Name | Metadata Summary | Manifest Summary |
|---------|-------------|------------|-----------------|-----------------|
| `random_points_in_polygon` | Random Points In Polygon | RandomPointsInPolygonTool | Generates random points uniformly within input polygon geometries. | Generates random points uniformly within input polygon geometries. |

**Key Features:**
- Uniform distribution within polygons
- Optional seed for reproducibility

---

### 9. GRID GENERATION

| Tool ID | Display Name | Struct Name | Metadata Summary | Manifest Summary |
|---------|-------------|------------|-----------------|-----------------|
| `hexagonal_grid_from_raster_base` | Hexagonal Grid From Raster Base | HexagonalGridFromRasterBaseTool | Creates a hexagonal polygon grid covering a raster extent. | Creates a hexagonal polygon grid covering a raster extent. |
| `hexagonal_grid_from_vector_base` | Hexagonal Grid From Vector Base | HexagonalGridFromVectorBaseTool | Creates a hexagonal polygon grid covering a vector-layer bounding extent. | Creates a hexagonal polygon grid covering a vector-layer bounding extent. |
| `vector_hex_binning` | Vector Hex Binning | VectorHexBinningTool | Aggregates point features into hexagonal bins, counting points per hex cell. | Aggregates point features into hexagonal bins, counting points per hex cell. |

**Key Features:**
- `hexagonal_grid_*`: Configurable hex width, orientation (horizontal/vertical)
- `vector_hex_binning`: Point aggregation with per-hex counts

---

### 10. TRIANGULATION & TESSELLATION

| Tool ID | Display Name | Struct Name | Metadata Summary | Manifest Summary |
|---------|-------------|------------|-----------------|-----------------|
| `construct_vector_tin` | Construct Vector TIN | ConstructVectorTinTool | Constructs a triangular irregular network (TIN) from an input point set using Delaunay triangulation. | Constructs a triangular irregular network (TIN) from an input point set using Delaunay triangulation. |

**Key Features:**
- Delaunay triangulation from point set
- Field-based or Z-value elevation support
- Optional max triangle edge length filtering

---

### 11. SUMMARY STATISTICS & REPORTING

| Tool ID | Display Name | Struct Name | Metadata Summary | Manifest Summary |
|---------|-------------|------------|-----------------|-----------------|
| `vector_summary_statistics` | Vector Summary Statistics | VectorSummaryStatisticsTool | Computes grouped summary statistics for a numeric field and writes the result to CSV. | Computes grouped summary statistics for a numeric field and writes the result to CSV. |

**Key Features:**
- Grouped aggregation by category field
- CSV output for integration with downstream analysis

---

## Licensing & Stability

| Field | Value |
|-------|-------|
| **License Tier** | Open (all tools) |
| **Stability** | Experimental (all tools) |

---

## Additional Tools Registered but Requiring Documentation

The following vector tools are registered in `lib.rs` but require separate investigation:

### Geometry Conversion (Legacy/Need Verification)
- LinesToPolygonsTool (ID: ?)
- PolygonsToLinesTool (ID: ?)
- MultipartToSinglepartTool (ID: ?)
- SinglepartToMultipartTool (ID: ?)
- MergeVectorsTool (ID: ?)

### Attribute Operations (Legacy/Need Verification)
- ExtractNodesTool (ID: ?)
- FilterVectorFeaturesByAreaTool (ID: ?)

### Network/Routing (Outside GIS Tier1/2)
- ShortestPathNetworkTool
- NetworkOdCostMatrixTool
- ClosestFacilityNetworkTool
- VehicleRoutingCvrpTool
- (10+ additional network routing tools)

### Spatial Statistics (Phase A/B/C/D)
- SpatialJoinTool (see above)
- OrdinaryKrigingTool
- LocalOrdinaryKrigingTool
- SimpleKrigingTool
- UniversalKrigingTool
- SpaceTimeKrigingTool
- OrdinaryCoKrigingTool
- (Additional spatial stats tools)

---

## Parameter Pattern Analysis

### Common Parameter Patterns

**Input/Output:**
- `input` (required): Vector layer path (most tools)
- `output` (required): Output vector path (most tools)
- `output` (optional): Some tools allow preview-only mode (e.g., `field_calculator`)

**Geometry-Based:**
- `distance` / `spacing`: Numeric distance parameters (buffer, densify, points_along_lines)
- `tolerance`: Simplification tolerance (simplify_features)
- `predicate`: Spatial relationship selector (select_by_location, spatial_join)

**Aggregation/Computation:**
- `field_type`: Output field type selector (add_field, field_calculator)
- `strategy`: Join/aggregation strategy (spatial_join: first, last, count, sum, mean, min, max)
- `measurement_mode`: Unit selection (add_geometry_attributes: auto, planar, geodesic)

**Miscellaneous:**
- `preview_rows`: Preview mode (field_calculator)
- `seed`: RNG seed (random_points_in_polygon)
- `overwrite`: Schema conflict handling (field_calculator, add_field)

---

## Documentation Enhancement Recommendations

### 1. **Expand Tool Summaries**
Currently: 1-2 sentence descriptions
Recommend: 3-4 sentence expanded summaries including:
- Use case examples
- Typical workflow context
- Performance considerations (where relevant)

### 2. **Add Parameter Guidance**
Currently: Parameter descriptions are terse
Recommend: Add context for:
- Default values and why they're chosen
- Valid ranges or examples
- Common gotchas/edge cases

### 3. **Unified Example Section**
Create workflow examples combining multiple tools:
- "Vector Cleanup Workflow": CleanVectorTool → SimplifyFeaturesTool → SmoothVectorsTool
- "Spatial Analysis": SelectByLocationTool → SpatialJoinTool → VectorSummaryStatisticsTool
- "Grid-Based Analysis": HexagonalGridFromVectorBaseTool → PointsAlongLinesTool → VectorHexBinningTool

### 4. **Legacy Port Tags**
Several tools tagged `legacy-port` (hex grids, voronoi). Recommend:
- Migrate to dedicated "Legacy Tools" section in docs
- Note original source/reference
- Flag for future architectural review

### 5. **Cross-Reference Matrix**
Create a tool interdependency matrix showing:
- Which tools output formats compatible with which inputs
- Common tool chains/workflows
- Example pipeline: Buffer → Simplify → Add Geometry → Field Calculator

---

## Files for Implementation

- **Metadata Extraction**: [crates/wbtools_oss/src/tools/gis/mod.rs](crates/wbtools_oss/src/tools/gis/mod.rs) (lines 12539+)
- **Registry**: [crates/wbtools_oss/src/lib.rs](crates/wbtools_oss/src/lib.rs) (`register_default_tools()`)
- **Tool Definitions**: Distributed across `tools/gis/mod.rs` with inline impl blocks

---

## Next Steps

1. **Verify missing tools** (LinesToPolygonsTool, MultipartToSinglepartTool, etc.) locations
2. **Extract manifest examples** for each tool to build workflow recipes
3. **Expand parameter descriptions** with unit guidance and common values
4. **Build HTML/Markdown documentation** from tool metadata
5. **Create interactive tool explorer** for user-facing documentation site
