# LiDAR Processing Tools

This document defines the LiDAR processing porting roadmap and CRS requirements for the backend migration. Tool-level API reference entries will be added here as each tool is ported.

For shared conventions (paths, callback payloads, optional output_path behavior), see [TOOLS.md](../TOOLS.md).

## Overview

LiDAR processing tools consume point-cloud datasets (LAS/LAZ/COPC/PLY/E57 via wblidar) and produce one or more of:
- LiDAR outputs (filtered/modified point clouds)
- Raster outputs (grids, interpolations, density, DSM/DTM-style derivatives)
- Vector outputs (contours, footprints, TIN-based products)
- Diagnostic products (metadata/statistics)

## Implementation Priorities

- Phase 1: LiDAR-to-raster interpolation and gridding tools.
- Phase 2: LiDAR-to-LiDAR processing tools.
- Phase 3: LiDAR-to-vector and specialized diagnostics/analysis tools.

The canonical checklist and status are tracked in [lidar_parity_checklist.md](lidar_parity_checklist.md).

## CRS Policy (Required)

These requirements are mandatory for all new LiDAR ports.

1. LiDAR-to-raster outputs must inherit CRS metadata from input LiDAR.
- If source has EPSG, set raster CRS EPSG.
- If source has WKT only, set raster CRS WKT.
- If both exist, preserve both.

2. Multi-input LiDAR tools must run in a single reference CRS.
- If all inputs already match, proceed directly.
- If inputs differ but are transformable, reproject to a common reference CRS before interpolation/aggregation.
- If CRS cannot be resolved, fail fast with a clear validation error.

3. LiDAR-to-vector outputs must carry LiDAR CRS.
- Output vector layer CRS must be assigned from source/reference LiDAR CRS.

4. Tests are required for CRS behavior.
- EPSG-only input
- WKT-only input
- Mismatched multi-input CRS
- Missing CRS metadata failure mode

## First Batch Targets

- lidar_nearest_neighbour_gridding
- lidar_idw_interpolation
- lidar_tin_gridding
- lidar_radial_basis_function_interpolation
- lidar_sibson_interpolation

Rationale:
- high user impact
- representative interpolation pathways
- establishes reusable CRS propagation/alignment patterns for all other LiDAR-to-raster tools

## Current Python API

The following convenience methods are available on `WbEnvironment`:

1. `lidar_nearest_neighbour_gridding(input=None, resolution=1.0, search_radius=2.5, interpolation_parameter="elevation", returns_included="all", excluded_classes=None, min_elev=None, max_elev=None, output_path=None, callback=None)`
2. `lidar_idw_interpolation(input=None, resolution=1.0, weight=1.0, search_radius=2.5, interpolation_parameter="elevation", returns_included="all", excluded_classes=None, min_elev=None, max_elev=None, min_points=1, output_path=None, callback=None)`
3. `lidar_tin_gridding(input=None, resolution=1.0, max_triangle_edge_length=inf, interpolation_parameter="elevation", returns_included="all", excluded_classes=None, min_elev=None, max_elev=None, output_path=None, callback=None)`
4. `lidar_radial_basis_function_interpolation(input=None, resolution=1.0, num_points=15, search_radius=0.0, func_type="thinplatespline", poly_order="none", weight=0.1, interpolation_parameter="elevation", returns_included="all", excluded_classes=None, min_elev=None, max_elev=None, output_path=None, callback=None)`
5. `lidar_sibson_interpolation(input=None, resolution=1.0, interpolation_parameter="elevation", returns_included="all", excluded_classes=None, min_elev=None, max_elev=None, output_path=None, callback=None)`
6. `lidar_block_maximum(input=None, resolution=1.0, interpolation_parameter="elevation", returns_included="all", excluded_classes=None, min_elev=None, max_elev=None, output_path=None, callback=None)`
7. `lidar_block_minimum(input=None, resolution=1.0, interpolation_parameter="elevation", returns_included="all", excluded_classes=None, min_elev=None, max_elev=None, output_path=None, callback=None)`
8. `lidar_point_density(input=None, resolution=1.0, search_radius=2.5, returns_included="all", excluded_classes=None, min_elev=None, max_elev=None, output_path=None, callback=None)`
9. `lidar_digital_surface_model(input=None, resolution=1.0, search_radius=0.5, min_elev=None, max_elev=None, max_triangle_edge_length=inf, output_path=None, callback=None)`
10. `lidar_hillshade(input=None, resolution=1.0, search_radius=-1.0, azimuth=315.0, altitude=30.0, returns_included="all", excluded_classes=None, min_elev=None, max_elev=None, output_path=None, callback=None)`
11. `filter_lidar_classes(input=None, excluded_classes=None, output_path=None, callback=None)`
12. `lidar_shift(input=None, x_shift=0.0, y_shift=0.0, z_shift=0.0, output_path=None, callback=None)`
13. `remove_duplicates(input=None, include_z=False, output_path=None, callback=None)`
14. `filter_lidar_scan_angles(input=None, threshold=0, output_path=None, callback=None)`
15. `filter_lidar_noise(input=None, output_path=None, callback=None)`
16. `lidar_thin(input=None, resolution=1.0, method="first", save_filtered=False, output_path=None, filtered_output_path=None, callback=None)`
17. `lidar_elevation_slice(input=None, minz=-inf, maxz=inf, classify=False, in_class_value=2, out_class_value=1, output_path=None, callback=None)`
18. `lidar_join(inputs, output_path=None, callback=None)`
19. `lidar_thin_high_density(input=None, density=1.0, resolution=1.0, save_filtered=False, output_path=None, filtered_output_path=None, callback=None)`
20. `lidar_tile(input, tile_width=1000.0, tile_height=1000.0, origin_x=0.0, origin_y=0.0, min_points_in_tile=2, output_laz_format=True, output_directory=None, callback=None)`
21. `sort_lidar(sort_criteria, input=None, output_path=None, callback=None)`
22. `filter_lidar_by_percentile(input=None, percentile=0.0, block_size=1.0, output_path=None, callback=None)`
23. `split_lidar(split_criterion, input=None, interval=5.0, min_pts=5, output_directory=None, callback=None)`
24. `lidar_remove_outliers(input=None, search_radius=2.0, elev_diff=50.0, use_median=False, classify=False, output_path=None, callback=None)`
25. `normalize_lidar(input, dtm, no_negatives=False, output_path=None, callback=None)`
26. `height_above_ground(input, output_path=None, callback=None)`
27. `lidar_ground_point_filter(input=None, search_radius=2.0, min_neighbours=0, slope_threshold=45.0, height_threshold=1.0, classify=False, height_above_ground=False, output_path=None, callback=None)`
28. `filter_lidar(statement, input=None, output_path=None, callback=None)`
29. `modify_lidar(statement, input=None, output_path=None, callback=None)`
30. `filter_lidar_by_reference_surface(input, ref_surface, query="within", threshold=0.0, classify=False, true_class_value=2, false_class_value=1, preserve_classes=False, output_path=None, callback=None)`
31. `classify_lidar(input=None, search_radius=2.5, grd_threshold=0.1, oto_threshold=1.0, linearity_threshold=0.5, planarity_threshold=0.85, num_iter=30, facade_threshold=0.5, output_path=None, callback=None)`
32. `classify_buildings_in_lidar(in_lidar, building_footprints, output_path=None, callback=None)`
33. `ascii_to_las(input_ascii_files, pattern, epsg_code=4326, output_directory=None, callback=None)`
34. `las_to_ascii(input=None, output_path=None, callback=None)`
35. `select_tiles_by_polygon(input_directory, output_directory, polygons, callback=None)`
36. `lidar_info(input, output_path=None, show_point_density=True, show_vlrs=True, show_geokeys=True, callback=None)`
37. `lidar_histogram(input, output_path=None, parameter="elevation", clip_percent=1.0, callback=None)`
38. `lidar_point_stats(input=None, resolution=1.0, num_points=False, num_pulses=False, avg_points_per_pulse=False, z_range=False, intensity_range=False, predominant_class=False, output_directory=None, callback=None)`
39. `lidar_contour(input=None, contour_interval=10.0, base_contour=0.0, smooth=5, interpolation_parameter="elevation", returns_included="all", excluded_classes=None, min_elev=-inf, max_elev=inf, max_triangle_edge_length=inf, output_path=None, callback=None)`
40. `lidar_tile_footprint(input=None, output_hulls=False, output_path=None, callback=None)`
41. `las_to_shapefile(input=None, output_multipoint=False, output_path=None, callback=None)`
42. `lidar_construct_vector_tin(input=None, returns_included="all", excluded_classes=None, min_elev=-inf, max_elev=inf, max_triangle_edge_length=inf, output_path=None, callback=None)`
43. `lidar_hex_bin(input, width, orientation="h", output_path=None, callback=None)`
44. `lidar_point_return_analysis(input, create_output=False, output_path=None, report_path=None, callback=None)`
45. `flightline_overlap(input=None, resolution=1.0, output_path=None, callback=None)`
46. `recover_flightline_info(input, max_time_diff=5.0, pt_src_id=False, user_data=False, rgb=False, output_path=None, callback=None)`
47. `find_flightline_edge_points(input, output_path=None, callback=None)`
48. `lidar_tophat_transform(input, search_radius, output_path=None, callback=None)`
49. `normal_vectors(input, search_radius=-1.0, output_path=None, callback=None)`
50. `lidar_kappa(classification_lidar, reference_lidar, report_path, resolution=1.0, output_class_accuracy=False, output_path=None, callback=None)`
51. `lidar_eigenvalue_features(input=None, num_neighbours=None, search_radius=None, output_path=None, callback=None)`
52. `lidar_ransac_planes(input, search_radius=2.0, num_iterations=50, num_samples=10, inlier_threshold=0.15, acceptable_model_size=30, max_planar_slope=75.0, classify=False, only_last_returns=False, output_path=None, callback=None)`
53. `lidar_rooftop_analysis(lidar_inputs, building_footprints, search_radius=2.0, num_iterations=50, num_samples=10, inlier_threshold=0.15, acceptable_model_size=30, max_planar_slope=75.0, norm_diff_threshold=2.0, azimuth=180.0, altitude=30.0, output_path=None, callback=None)`

Notes:
- `returns_included` supports `all`, `first`, and `last`.
- `interpolation_parameter` supports `elevation`, `intensity`, `class`, `return_number`, `number_of_returns`, `scan_angle`, `time`, `rgb`, and `user_data`.
- `excluded_classes` accepts integer classes (e.g., `[7, 18]` in Python).
- Backend tools now support legacy-style batch mode when `input` is omitted for all currently ported LiDAR raster tools.
- In batch mode, the backend scans the current working directory for supported LiDAR formats (LAS, LAZ, COPC, PLY, E57), processes tiles in parallel, and writes per-tile GeoTIFF outputs with tool-specific suffixes.
- For interpolation tools, batch mode also includes points from neighboring batch tiles when processing each target tile, which helps reduce interpolation edge effects at tile boundaries.
- Python convenience methods now accept `input=None` to trigger backend batch mode.
- In batch mode, the returned Python `Raster` is a placeholder pointing to the first written output tile; all batch outputs are written directly to disk and not retained as in-memory raster objects.
- For LiDAR-to-LiDAR tools in batch mode, the returned Python `Lidar` is similarly a placeholder path to the first written output tile, while all batch outputs are written directly to disk.

## Per-Tool Parameter Notes

### lidar_nearest_neighbour_gridding
- Use when preserving local point values is preferred over smoothing.
- `search_radius` controls nodata behavior away from points.
- Backend batch-mode suffix: `_nn.tif`.
- In backend batch mode, each target tile can include neighboring tile points for edge-effect reduction.

### lidar_idw_interpolation
- `weight` controls distance decay. Higher values emphasize closer points.
- `search_radius <= 0` triggers k-nearest fallback; `min_points` controls k.
- Backend batch-mode suffix: `_idw.tif`.
- In backend batch mode, each target tile can include neighboring tile points for edge-effect reduction.

### lidar_tin_gridding
- Uses Delaunay triangles with barycentric interpolation.
- `max_triangle_edge_length <= 0` means no edge-length limit.
- Backend batch-mode suffix: `_tin.tif`.
- In backend batch mode, each target tile can include neighboring tile points for edge-effect reduction.

### lidar_radial_basis_function_interpolation
- `num_points` sets the local neighbourhood size.
- `func_type` options: `thinplatespline`, `polyharmonic`, `gaussian`, `multiquadric`, `inversemultiquadric`.
- `poly_order` controls local polynomial trend correction: `none`, `constant`, or `quadratic`.
- Backend batch-mode suffix: `_rbf.tif`.
- In backend batch mode, each target tile can include neighboring tile points for edge-effect reduction.

### lidar_sibson_interpolation
- True Sibson natural-neighbour interpolation intended for smooth terrain surfaces.
- No explicit weight/radius tuning parameter is required for common use.
- Backend batch-mode suffix: `_sibson.tif`.
- In backend batch mode, each target tile can include neighboring tile points for edge-effect reduction.

### lidar_block_maximum
- Computes a cell-wise maximum from filtered LiDAR points for rapid surface approximation.
- Backend batch-mode suffix: `_block_max.tif`.

### lidar_block_minimum
- Computes a cell-wise minimum from filtered LiDAR points for rapid lower-envelope approximation.
- Backend batch-mode suffix: `_block_min.tif`.

### lidar_point_density
- Counts nearby points around each cell center within `search_radius` and reports density per area.
- Backend batch-mode suffix: `_density.tif`.

### lidar_digital_surface_model
- Uses local top-surface candidate filtering and TIN gridding for a DSM-style output.
- Supports `max_triangle_edge_length` masking for long-edge facet suppression.
- Backend batch-mode suffix: `_dsm.tif`.

### lidar_hillshade
- Derives a hillshade raster from LiDAR-derived local surface values.
- Supports illumination controls via `azimuth` and `altitude`.
- Supports optional `search_radius` compatibility parameter for legacy call-shape parity.
- Backend batch-mode suffix: `_hillshade.tif`.

### lidar_contour
- Builds contour lines from LiDAR points using TIN-based contouring.
- Supports interval/base controls, interpolation parameter selection, returns/class filters, and optional triangle-edge-length masking.
- In batch mode (`input=None`), processes all LiDAR files in the working directory and writes sibling contour shapefiles.

### lidar_tile_footprint
- Generates polygon footprints per tile with summary attributes (`LAS_NM`, `NUM_PNTS`, `Z_MIN`, `Z_MAX`).
- Writes bounding boxes by default; set `output_hulls=True` to write convex hull footprints.
- In batch mode (`input=None`), writes a single combined footprint layer for all LiDAR tiles in the working directory.

### las_to_shapefile
- Converts LiDAR points to vector output.
- By default writes one point feature per LiDAR point with key attributes (`Z`, `INTENSITY`, `CLASS`, return fields).
- If `output_multipoint=True`, writes a single multipoint feature instead.
- In batch mode (`input=None`), writes one shapefile per LiDAR tile in the working directory.

### lidar_construct_vector_tin
- Builds a triangular mesh vector layer directly from LiDAR points using Delaunay triangulation.
- Supports return filtering, class filtering, elevation limits, and optional max triangle-edge filtering.
- In batch mode (`input=None`), writes one `_tin.shp` output per LiDAR tile in the working directory.

### lidar_hex_bin
- Bins LiDAR points to a hexagonal polygon grid and summarizes per-cell point count and min/max z/intensity.
- `width` sets the distance between opposite hex sides.
- `orientation` supports `h` (pointy-up) and `v` (flat-up).

### lidar_point_return_analysis
- Produces a return-sequence quality-control report (`report_path`) covering missing returns, duplicates, and `r > n` anomalies.
- If `create_output=True`, also writes a classified QC LiDAR output (`output`) with class assignments for problematic return groups.

### flightline_overlap
- Builds a raster whose cell values equal the number of distinct `point_source_id` values present in each cell.
- In batch mode (`input=None`), scans the working directory for LiDAR tiles and writes one `_flightline_overlap.tif` raster per tile.

### recover_flightline_info
- Sorts points by GPS time and starts a new inferred flightline when the time gap exceeds `max_time_diff` seconds.
- Writes inferred flightline identifiers into any combination of `point_source_id`, `user_data`, and RGB.
- If no output field flags are enabled, RGB output is enabled automatically.

### find_flightline_edge_points
- Filters the input LiDAR to only points carrying the LAS `edge_of_flight_line` flag.
- Returns a LiDAR output containing just the edge points.

### lidar_tophat_transform
- Applies a white top-hat transform to point elevations by subtracting a locally opened surface from each point z value.
- `search_radius` controls the XY neighbourhood used for the erosion and dilation steps.

### normal_vectors
- Estimates local plane normals from neighbouring points and writes a compatibility color encoding based on normal direction.
- If `search_radius <= 0`, the backend estimates a nominal point spacing and derives a default neighbourhood size automatically.

### lidar_kappa
- Compares classifications in two LiDAR datasets using nearest-point matching and writes an HTML agreement report.
- Also writes a raster of per-cell classification agreement percentages at `resolution`.
- `output_class_accuracy` is accepted for legacy call-shape parity; the class-agreement raster is produced regardless.

### lidar_eigenvalue_features
- Computes local PCA-based neighbourhood features and writes a binary `.eigen` output plus a JSON schema sidecar.
- Supports single-file mode (`input=...`) and working-directory batch mode (`input=None`).
- `num_neighbours` must be at least `7` when specified; `search_radius` can be used alone or together with neighbour-count limiting.

### lidar_ransac_planes
- Uses local RANSAC plane fitting to identify planar points.
- If `classify=False`, non-planar points are filtered out; if `classify=True`, all points are retained and tagged as planar vs non-planar by class value.
- `only_last_returns=True` restricts model fitting to late returns.

### lidar_rooftop_analysis
- Identifies rooftop facets inside `building_footprints` and writes polygon output with rooftop attributes such as `MAX_ELEV`, `HILLSHADE`, `SLOPE`, `ASPECT`, and `AREA`.
- `lidar_inputs` accepts one or more LiDAR tiles covering the buildings of interest.
- Current parity implementation outputs convex-hull roof facets per detected planar segment.

### filter_lidar_classes
- Removes points whose classification is listed in `excluded_classes`.
- Backend batch-mode suffix: `_filtered_cls` with LiDAR output extension.

### lidar_shift
- Applies additive `x_shift`, `y_shift`, and `z_shift` offsets to each point.
- Backend batch-mode suffix: `_shifted` with LiDAR output extension.

### remove_duplicates
- Removes duplicate points by x/y coordinates, optionally including z when `include_z=True`.
- Backend batch-mode suffix: `_dedup` with LiDAR output extension.

### filter_lidar_scan_angles
- Removes points with absolute scan angle greater than `threshold`.
- `threshold` uses LAS scan-angle integer units (`1 unit = 0.006°`).
- Backend batch-mode suffix: `_scan_filtered` with LiDAR output extension.

### filter_lidar_noise
- Removes points classified as low noise (`class=7`) or high noise (`class=18`).
- Backend batch-mode suffix: `_denoised` with LiDAR output extension.

### lidar_thin
- Keeps at most one point per grid cell at `resolution`.
- `method` supports `first`, `last`, `lowest`, `highest`, and `nearest`.
- Backend batch-mode suffix: `_thinned` with LiDAR output extension.
- If `save_filtered=True`, filtered-out points are also written and exposed as `filtered_path` in backend outputs (wrapper return remains the kept-point `Lidar`).

### lidar_elevation_slice
- In filter mode (`classify=False`), keeps only points with `minz <= z <= maxz`.
- In classify mode (`classify=True`), keeps all points and reassigns class values using `in_class_value` and `out_class_value`.
- Backend batch-mode suffix: `_elev_slice` with LiDAR output extension.

### lidar_join
- Merges multiple input LiDAR files into a single output point cloud.
- Expects `inputs` to be a list of LiDAR objects/paths.

### lidar_thin_high_density
- Performs density-aware thinning by x/y blocks and z bins.
- `density` sets target points-per-area threshold; areas above threshold are thinned.
- If `save_filtered=True`, filtered points are also written and returned via backend `filtered_path` metadata.
- Backend batch-mode suffix: `_thinned_hd` with LiDAR output extension.

### lidar_tile
- Splits a single input LiDAR into row/column tile outputs using a regular grid.
- Writes multiple files to `output_directory` (or `<input_stem>/` by default) and returns a placeholder path to one written tile.

### sort_lidar
- Sorts points by one or more criteria (e.g., `"x 100, y 100, z"`).
- Supports optional bin sizes per criterion for grouped sorting.
- Backend batch-mode suffix: `_sorted` with LiDAR output extension.

### filter_lidar_by_percentile
- Selects one representative point per grid block by elevation percentile.
- `percentile=0` selects local minima, `100` selects maxima, `50` approximates medians.
- Excludes withheld and noise-classified points from percentile candidate sets.
- Backend batch-mode suffix: `_percentile` with LiDAR output extension.

### split_lidar
- Splits a LiDAR file into multiple outputs using a grouping criterion (`num_pts`, `x`, `y`, `z`, `intensity`, `class`, `user_data`, `point_source_id`, `scan_angle`, `time`).
- `interval` controls bin width for numeric criteria and points-per-file when `split_criterion="num_pts"`.
- `min_pts` filters sparse split outputs.
- In both single and batch mode, tool returns a placeholder path to one written split file plus backend `output_count` metadata.

### lidar_remove_outliers
- Computes local elevation residuals from neighborhood mean/median and either filters or reclassifies outliers.
- `classify=False`: removes outliers; `classify=True`: keeps all points and assigns low/high noise classes 7/18.
- Backend batch-mode suffix: `_outliers_removed` (filter mode) or `_outliers_classified` (classify mode).

### normalize_lidar
- Uses an input DTM raster to convert LiDAR elevations to height above terrain (`z = z_lidar - z_dtm`).
- `dtm` may be provided as a path string or typed raster object.
- If `no_negatives=True`, negative normalized values are clamped to `0.0`.

### height_above_ground
- Converts each point elevation to height above the nearest ground-classified point (`class=2`).
- Ground-classified points are assigned `z=0.0` in output.
- Fails with a validation/runtime error if there are no ground-classified points.

### lidar_ground_point_filter
- Applies a slope-and-height neighborhood test to identify off-terrain points.
- `classify=True` writes classification labels (`ground=2`, `off-terrain=1`); otherwise off-terrain points are removed.
- Supports optional `height_above_ground=True` output z normalization from local neighborhood minima.
- Backend batch-mode suffix: `_ground_filtered` with LiDAR output extension.

### filter_lidar
- Filters points with a boolean `statement` evaluated against point attributes.
- Supports core variables: coordinates (`x,y,z`), returns (`ret,nret,is_late,...`), class flags, scan metrics, color/time, and file min/mid/max stats.
- Backend batch-mode suffix: `_filtered` with LiDAR output extension.

### modify_lidar
- Applies assignment expressions to mutate LiDAR point attributes (e.g., `z = z + 1`, `class = if(z > 10, 2, class)`, `rgb = (255,0,0)`).
- Supports semicolon-separated multi-expression statements evaluated per point.
- Supports no-input batch mode with `_modified` output suffix.

### filter_lidar_by_reference_surface
- Compares each point elevation against a raster reference surface using query modes: `within`, `<`, `<=`, `>`, `>=`.
- In filter mode (`classify=False`), outputs only points that satisfy the query.
- In classify mode, all points are retained and classes are written using `true_class_value` and `false_class_value` (or preserved when `preserve_classes=True`).

### classify_lidar
- Performs basic neighborhood-based classification into ground (`2`), building (`6`), vegetation (`5`), and unclassified (`1`).
- Uses `search_radius`, `grd_threshold`, and `oto_threshold` as primary controls, with RANSAC-style local planarity/linearity estimation.
- Supports no-input batch mode; backend batch-mode suffix: `_classified` with LiDAR output extension.
- `linearity_threshold`, `planarity_threshold`, `num_iter`, and `facade_threshold` are actively used during segmentation and refinement stages.

### classify_buildings_in_lidar
- Reclassifies points to building class (`6`) when they fall within polygon building footprints.
- Input polygons should represent building outlines in the same CRS as the LiDAR input.

### ascii_to_las
- Converts one or more ASCII point files into LAS output files.
- `pattern` must include `x,y,z` and can include: `i,c,rn,nr,time,sa,r,g,b`.
- If `rn` is used, `nr` must also be used; RGB fields must be supplied as a full `r,g,b` set.
- `epsg_code` sets output LAS CRS metadata; outputs are written to `output_directory` when provided.

### las_to_ascii
- Converts an input LiDAR file to CSV with columns based on available attributes (time/RGB columns are included when present).
- If `input=None`, runs in batch mode over LiDAR files in the current working directory and returns a placeholder path to one produced CSV.

### select_tiles_by_polygon
- Scans LiDAR tiles in `input_directory` and copies selected tiles to `output_directory` based on polygon overlap.
- Tile selection uses representative tile-bounding-box sample points (corners, center, and edge midpoints) against polygon geometry.
- Returns the output directory path and writes selected tile files directly to disk.

### lidar_info
- Produces a LiDAR summary report with point counts, coordinate/intensity ranges, class counts, and return counts.
- Writes to `.txt` or `.html` report output and returns the report path.

### lidar_histogram
- Builds a histogram report for one parameter (`elevation`, `intensity`, `scan angle`, `class`, or `time`).
- Supports lower/upper tail clipping using `clip_percent`.
- Writes an HTML report and returns its path.

### lidar_point_stats
- Generates one or more summary rasters (point count, pulse count, avg points/pulse, z range, intensity range, predominant class).
- If no output flags are set, all point-stat rasters are generated.
- Returns the output directory path containing generated GeoTIFF rasters.

### lidar_classify_subset
- Reclassifies points in a base cloud when they spatially match points in a subset cloud.
- Uses 3D nearest-neighbour matching with configurable `tolerance` (map units).
- Assigns `subset_class_value` to matches and `nonsubset_class_value` to non-matches (`255` preserves original classes).

### clip_lidar_to_polygon
- Retains only points that lie within input polygon geometry (`polygons` vector path/object).
- Supports polygon holes; points inside holes are excluded from output.

### erase_polygon_from_lidar
- Removes points that lie within input polygon geometry (`polygons` vector path/object).
- Complement of `clip_lidar_to_polygon`; points outside polygons are retained.

### classify_overlap_points
- Identifies overlap points in grid cells containing multiple point-source IDs and either classifies (`class=12`) or filters them.
- Supported criteria: `max scan angle`, `not min point source id`, `not min time`, `multiple point source IDs`.
- `filter=True` removes overlap points; otherwise flagged points are reclassified.

### lidar_segmentation
- Segments points into connected components using XY neighbourhood (`search_radius`) and vertical continuity (`max_z_diff`).
- Writes per-segment colours into point RGB values; largest segment receives dark green `(25,120,0)`.
- Compatibility parameters for legacy call-shape are accepted (`num_iterations`, `num_samples`, `inlier_threshold`, `acceptable_model_size`, `max_planar_slope`, `norm_diff_threshold`).
- Optional `ground=True` assigns class `2` to the largest segment and class `1` to other segmented points.

### individual_tree_segmentation
- Segments individual tree points using a mean-shift mode-seeking workflow over LiDAR points.
- Algorithm inspiration and credit: this implementation is inspired by the self-adaptive mean-shift tree-segmentation approach described in the 2020 MDPI Remote Sensing article on individual-tree segmentation, and by the grid-accelerated approximation ideas in the MeanShift++ paper (arXiv, 2021).
- Defaults to vegetation-only segmentation (`only_use_veg=True`, `veg_classes="3,4,5"`) and height filtering (`min_height=2.0`).
- Uses adaptive per-seed horizontal bandwidth by default (`adaptive_bandwidth=True`) based on local neighbour density and angular sector crown-radius cues.
- Adaptive controls: `adaptive_neighbors` (default `24`) and `adaptive_sector_count` (default `8`), with fallback to bounded height-based scaling when adaptive mode is disabled.
- Supports optional MeanShift++-style grid approximation (`grid_acceleration=True`) that runs mode updates against aggregated grid cells for faster large-cloud processing.
- Grid approximation control: `grid_cell_size` (default `0.5` in XY map units).
- Optional post-grid exact refinement: `grid_refine_exact=True` with `grid_refine_iterations` (default `2`) to recover local precision after coarse grid updates.
- Optional tiled seed scheduling for very large inputs: `tile_size` (default `0.0`, disabled) and `tile_overlap` (default `0.0`, must be smaller than `tile_size`).
- Supports deterministic random segment colouring (`output_id_mode="rgb"`), optional id storage in `user_data` / `point_source_id`, and optional sidecar CSV output (`output_sidecar_csv=True`).
- Includes performance controls: `threads` and `simd`.

Benchmarking notes (developer reference):
- Benchmark target: `individual_tree_segmentation_bench` in [Rust/whitebox_next_gen/crates/wbtools_oss/benches/individual_tree_segmentation_bench.rs](Rust/whitebox_next_gen/crates/wbtools_oss/benches/individual_tree_segmentation_bench.rs)
- Run command:
	- `cargo bench -p wbtools_oss --bench individual_tree_segmentation_bench`
- Suggested result-record template:

| Date | Dataset | Points | Mode | Threads | SIMD | Wall time (ms) | Assigned veg points | Segment count |
|------|---------|--------|------|---------|------|----------------|---------------------|---------------|
| YYYY-MM-DD | name | n | exact / grid_accel | t | true/false | value | value | value |

Expanded local matrix (2026-03-24):

| Date | Dataset | Points | Mode | Threads | SIMD | Wall time (ms) | Assigned veg points | Segment count |
|------|---------|--------|------|---------|------|----------------|---------------------|---------------|
| 2026-03-24 | synthetic small_t32_p180 | 5952 | exact | auto | true | 28.28 (median) | 5536 | 32 |
| 2026-03-24 | synthetic small_t32_p180 | 5952 | grid_accel | auto | true | 9.20 (median) | 5536 | 32 |
| 2026-03-24 | synthetic small_t32_p180 | 5952 | grid_refine | auto | true | 14.29 (median) | 5536 | 32 |
| 2026-03-24 | synthetic small_t32_p180 | 5952 | grid_refine_tiled | auto | true | 14.86 (median) | 5536 | 32 |
| 2026-03-24 | synthetic small_t32_p180 | 5952 | grid_refine_tiled_t1 | 1 | true | 61.15 (median) | 5536 | 32 |
| 2026-03-24 | synthetic medium_t64_p220 | 14464 | exact | auto | true | 90.10 (median) | 13632 | 64 |
| 2026-03-24 | synthetic medium_t64_p220 | 14464 | grid_accel | auto | true | 28.40 (median) | 13632 | 64 |
| 2026-03-24 | synthetic medium_t64_p220 | 14464 | grid_refine | auto | true | 44.48 (median) | 13632 | 64 |
| 2026-03-24 | synthetic medium_t64_p220 | 14464 | grid_refine_tiled | auto | true | 48.20 (median) | 13632 | 64 |
| 2026-03-24 | synthetic medium_t64_p220 | 14464 | grid_refine_tiled_t1 | 1 | true | 191.91 (median) | 13632 | 64 |
| 2026-03-24 | synthetic medium_t64_p220 | 14464 | grid_refine_tiled_t4 | 4 | true | 72.74 (median) | 13632 | 64 |

Observed speed/quality summary:
- Quality parity on synthetic data: assigned-point and segment-count ratios are 1.0 for all measured variants versus exact.
- Fastest profile in this matrix: grid_accel (about 3.07x faster on small, 3.17x faster on medium versus exact).
- Grid refine improves precision pathway while retaining speedup (about 1.98x small, 2.03x medium versus exact).
- Tiling is useful for scalability controls, but on these moderate synthetic sizes it adds overhead unless required for memory/partitioning constraints.

**Parameter tuning guidance:** For recommended parameter profiles tailored to your use case (speed-critical, precision-optimized, balanced, or memory-constrained), see **Section 16 (Parameter tuning guide)** in [lidar_individual_tree_segmentation_design.md](lidar_individual_tree_segmentation_design.md#16-parameter-tuning-guide-by-use-case). This guide provides ready-to-use parameter sets plus hyperparameter tuning directions for common forest types.

### individual_tree_detection
- Identifies tree top points in a LiDAR cloud using local maxima detection.
- Detects points that are the highest within a local search neighbourhood in XY, with optional height range constraints.
- Search neighbourhood size can vary with height: `min_search_radius` applies at lower heights, `max_search_radius` at upper heights, with linear interpolation between.
- Requires vegetation-only filtering by default (`only_use_veg=True`, vegetation classes 3, 4, 5); can be disabled if input is unclassified.
- Outputs a vector shapefile of tree top points with attributes: `FID` (1-based point index) and `Z` (height value).
- Use case: rapid treetop detection for forest inventory, DBH estimation, or canopy analysis; complements individual_tree_segmentation for cases where only crown centroids are needed.
- Parameters:
  - `min_search_radius` (default 1.0): search neighbourhood radius at minimum height
  - `max_search_radius` (optional): neighbourhood size at upper heights; if not specified, uses min_search_radius (constant neighbourhood)
  - `min_height` (default 0.0): minimum height threshold (points below this are skipped)
  - `max_height` (optional): height value where max_search_radius applies (linear interpolation between min and max)
  - `only_use_veg` (default true): if true, process only vegetation classes 3, 4, 5; if false, use all non-withheld, non-noise points

### lidar_segmentation_based_filter
- Performs neighbourhood-connected low-relief ground extraction using `search_radius` and `max_z_diff`.
- `classify_points=False` outputs only extracted ground-like points.
- `classify_points=True` retains all points and assigns class `2` (ground-like) or class `1` (other).

### lidar_colourize
- Assigns point RGB values from an overlapping raster image sampled at each point XY location.
- `image` can be provided as a raster object/path; out-of-image or nodata samples are assigned black.

### colourize_based_on_class
- Applies class-based colours for LAS classes 0-18, with optional per-class overrides via `clr_str`.
- Supports optional intensity blending (`intensity_blending_amount` in percent).
- `use_unique_clrs_for_buildings=True` assigns unique colours to connected building clusters (`class=6`) using `search_radius`.
- Supports batch mode when `input=None`.

### colourize_based_on_point_returns
- Applies colours by return type: only, first, intermediate, and last returns.
- Supports optional intensity blending (`intensity_blending_amount` in percent).
- Supports batch mode when `input=None`.

## Callback Payload Examples

All LiDAR interpolation methods emit JSON strings through `callback` using message/progress events.

Example message event:

```json
{"type":"message","message":"reading input lidar"}
```

Example progress event:

```json
{"type":"progress","percent":0.63}
```

`percent` is normalized in [0, 1] and reaches 1.0 on completion.

Example:

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
lidar = wbe.read_lidar("tile.laz")

dem = wbe.lidar_idw_interpolation(
	lidar,
	resolution=1.0,
	weight=2.0,
	returns_included="last",
	excluded_classes=[7, 18],
)
```

## Notes

- This document is intentionally planning-oriented for now.
- As each tool is ported, add full parameter and examples sections in this file.
