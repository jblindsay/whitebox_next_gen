# Raster Analysis

Raster data are the backbone of most geospatial analysis workflows. They encode continuous or categorical phenomena as regular grids of cell values, allowing mathematical, statistical, and spatial operations to be applied efficiently across an entire landscape. Whitebox Workflows for Python (WbW-Py) provides an extensive raster processing toolkit covering basic arithmetic, focal and zonal statistics, reclassification, resampling, interpolation, proximity analysis, morphological operations, and raster-to-vector conversion.

---

## Core Concepts

Raster analysis requires understanding these fundamental terms:

- **Cell (pixel)**: The smallest unit of a raster. Each cell stores a single value (integer or floating-point) and has a uniform spatial extent (e.g. 10 m × 10 m).
- **Data type**: Integer (Int8, Int16, Int32) for categorical data; Float32 or Float64 for continuous measurements. Data type affects precision, file size, and computation speed.
- **NoData value**: Sentinel value representing missing or invalid data (e.g. -9999 or NaN). Critical for masking water, clouds, or invalid measurements in focal operations.
- **Spatial reference (CRS)**: Coordinate system and projection. Mismatched CRS between rasters causes silent misalignment; always verify before overlay operations.
- **Extent**: The bounding box (xmin, ymin, xmax, ymax) of the raster in real-world coordinates.
- **Cell size (resolution)**: Cell width and height in map units. Coarser resolution is faster but loses detail; finer resolution requires more memory and computation.
- **Focal operation**: Uses neighbourhood values (e.g. 3×3 kernel) to compute output. Examples: moving average, Sobel edge detection, local extrema.
- **Zonal operation**: Aggregates grid values by zone (polygon or categorical layer). Examples: mean by land-cover class, sum by administrative boundary.
- **Reclassification**: Reassigning cell values according to lookup rules. Common for categorizing continuous data (e.g. slope classes) or remapping land-cover codes.
- **Resampling**: Changing cell size or alignment. Methods: nearest-neighbour (preserves categories), bilinear (smooth), cubic (smoother).

---

## Reading and Writing Rasters

Every raster workflow begins with reading data into memory:

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
wbe.working_directory = '/data/rasters'
wbe.verbose = True

dem = wbe.read_raster('dem.tif')
landcover = wbe.read_raster('landcover.tif')

# Read multiple files at once — returns a list
[slope_r, aspect_r, curvature_r] = wbe.read_rasters('slope.tif',
                                                      'aspect.tif',
                                                      'curvature.tif')
```

Write results with optional LZW compression (recommended for floating-point grids):

```python
wbe.write_raster(dem, 'dem_processed.tif', compress=True)
```

### Printing GeoTIFF Metadata

Inspect the spatial reference and encoding of any GeoTIFF:

```python
wbe.raster.general.print_geotiff_tags(dem)
```

---

## Creating New Rasters

Use `new_raster()` or `new_raster_from_base_raster()` to create blank grids for accumulation or custom calculations:

```python
# Create a raster with the same extent and resolution as the DEM, filled with 0
output = wbe.conversion.raster_vector_conversion.new_raster_from_base_raster(base=dem, initial_value=0.0)

# Iterate cell-by-cell (illustrative; use raster_calculator for large grids)
for row in range(dem.configs.rows):
    for col in range(dem.configs.columns):
        val = dem[row, col]
        if val != dem.configs.nodata:
            output[row, col] = val * 0.001  # convert mm to m
```

For most cell-by-cell transforms, `raster_calculator()` is many times faster because it operates in compiled Rust without Python loop overhead.

---

## Raster Calculator

`raster_calculator()` evaluates an expression string against one or more input rasters. Rasters are referenced by quoted variable names whose order must match the `input_rasters` list:

```python
# Scale DEM from centimetres to metres
dem_m = wbe.raster.general.raster_calculator("'dem' / 100.0", [dem])

# Compute HAND (height above nearest drainage) from DEM and stream mask
hand_expr = "if('stream' > 0.0, 0.0, 'dem' - 'stream_elev')"
hand = wbe.raster.general.raster_calculator(hand_expr, [stream_raster, dem, stream_elev])
```

The expression supports:
- Arithmetic operators: `+`, `-`, `*`, `/`, `^` (power), `%` (modulo)
- Comparison operators: `<`, `>`, `<=`, `>=`, `==`, `!=`
- Logical operators: `&&`, `||`, `!`
- Mathematical functions: `sqrt()`, `abs()`, `ln()`, `log()`, `exp()`, `sin()`, `cos()`, `tan()`, `atan()`, `atan2()`
- Conditional expression: `if(condition, true_value, false_value)`
- Special tokens: `nodata`, `null`, `rows`, `columns`, `row`, `column`, `cellsize`, `north`, `south`, `east`, `west`

### Practical Examples

```python
# Mask out NoData and values below sea level
masked = wbe.raster.general.raster_calculator(
    "if('dem' == nodata || 'dem' < 0.0, nodata, 'dem')",
    [dem]
)

# Compute normalised slope (0–1)
slope = wbe.terrain.derivatives.slope(dem)
slope_norm = wbe.raster.general.raster_calculator(
    "'slope' / maxvalue",
    [slope]
)

# Create binary mask for steep slopes (>30°)
steep = wbe.raster.general.raster_calculator("if('slope' > 30.0, 1.0, 0.0)", [slope])
```

---

## Reclassification

Reclassification converts continuous values into categorical classes, or remaps existing category codes.

### Manual Reclassification

```python
# reclass() takes new_value, min, max triplets
# Format: [[new_val, min_exclusive, max_inclusive], ...]
# Reclassify slope into five morphology classes
slope_class = wbe.raster.reclass_mask.reclass(
    raster=slope,
    reclass_vals=[
        [1, 0, 5],      # flat: 0–5°
        [2, 5, 15],     # gentle: 5–15°
        [3, 15, 30],    # moderate: 15–30°
        [4, 30, 45],    # steep: 30–45°
        [5, 45, 90],    # very steep: >45°
    ]
)
wbe.write_raster(slope_class, 'slope_classes.tif')
```

### Equal-Interval Reclassification

```python
# Automatically divide the value range into n equal-width intervals
dem_classes = wbe.raster.reclass_mask.reclass_equal_interval(dem, num_intervals=10)
wbe.write_raster(dem_classes, 'dem_equal_interval.tif')
```

### Set NoData Values

```python
# Convert all pixels equal to -9999 to NoData
corrected = wbe.conversion.raster_vector_conversion.set_nodata_value(dem, back_value=-9999.0)

# Convert NoData back to zero (useful for accumulation grids)
zero_bg = wbe.conversion.raster_vector_conversion.convert_nodata_to_zero(dem)
```

---

## Focal (Neighbourhood) Statistics

Focal statistics compute a statistic within a moving window centred on each cell, producing a smoothed or derivative surface.

### Filter Operations

```python
# Mean filter (low-pass smoothing)
dem_smooth = wbe.remote_sensing.filters.mean_filter(dem, filter_size_x=5, filter_size_y=5)

# Gaussian filter — weighted mean using Gaussian kernel
dem_gauss = wbe.remote_sensing.filters.gaussian_filter(dem, sigma=2.0)

# Median filter — robust against outliers
dem_median = wbe.remote_sensing.filters.median_filter(dem, filter_size_x=5, filter_size_y=5)

# Feature-preserving smoothing — preserves ridges and channels
dem_fp = wbe.terrain.general.feature_preserving_smoothing(dem, filter_size=11,
                                           normal_diff_threshold=30.0)

# Standard deviation filter — highlights textural variation
sd_texture = wbe.remote_sensing.filters.standard_deviation_filter(dem, filter_size_x=9, filter_size_y=9)

# Range filter — local elevation range (micro-roughness)
local_range = wbe.remote_sensing.filters.range_filter(dem, filter_size_x=9, filter_size_y=9)

# Percentile filter — local quantile (robust position index)
ep = wbe.remote_sensing.filters.percentile_filter(dem, filter_size_x=11, filter_size_y=11)
```

### Diversity and Majority Filters

```python
# Count the number of unique values in a neighbourhood (for categorical rasters)
diversity = wbe.remote_sensing.filters.diversity_filter(landcover, filter_size_x=7, filter_size_y=7)

# Majority filter — replace each cell with the most frequent class in neighbourhood
smoothed_lc = wbe.remote_sensing.filters.majority_filter(landcover, filter_size_x=5, filter_size_y=5)
```

### Morphological Filters (Binary/Raster Objects)

Morphological operations on raster objects (connected foreground regions) are essential for post-classification cleanup.

```python
# Dilation — grow foreground by one cell in each direction
dilated = wbe.remote_sensing.filters.closing(binary_raster, filter_size_x=3, filter_size_y=3)

# Erosion — shrink foreground
eroded = wbe.remote_sensing.filters.opening(binary_raster, filter_size_x=3, filter_size_y=3)

# Top-hat transform — isolates bright or dark features within a local window
tophat_white = wbe.remote_sensing.filters.tophat_transform(dem, filter_size_x=21, filter_size_y=21,
                                    variant='white')  # bright features above background
tophat_black = wbe.remote_sensing.filters.tophat_transform(dem, filter_size_x=21, filter_size_y=21,
                                    variant='black')  # dark depressions below background
```

---

## Summary and Zonal Statistics

### Global Summary Statistics

```python
wbe.raster.general.raster_summary_stats(dem)  # prints count, mean, std, min, max, range to console
```

### Histogram

```python
wbe.raster.general.raster_histogram(dem, output_html_file='dem_histogram.html')
```

### Zonal Statistics

Zonal statistics compute per-zone summary metrics for one raster given a second raster that defines zones:

```python
# Compute mean, St. dev., min, max of the DEM within each land-cover class
zonal_stats = wbe.raster.general.zonal_statistics(
    raster=dem,
    zones=landcover,
    stat='mean',           # 'mean', 'min', 'max', 'std', 'var', 'count', 'sum'
    cell_size_is_area=False
)
wbe.write_raster(zonal_stats, 'mean_elevation_per_class.tif')
```

### Percent Overlays

```python
# Fraction of neighbourhood cells below/above/equal to focal cell value
pct_less  = wbe.raster.overlay_math.percent_less_than(dem, filter_size_x=9, filter_size_y=9)
pct_great = wbe.raster.overlay_math.percent_greater_than(dem, filter_size_x=9, filter_size_y=9)
pct_equal = wbe.raster.overlay_math.percent_equal_to(dem, filter_size_x=9, filter_size_y=9)
```

### Unique-Value Enumeration

```python
wbe.raster.general.list_unique_values_raster(
    landcover,
    output_path='landcover_unique_values.csv',
)  # writes category counts to CSV and returns report JSON
```

---

## Overlay Operations

Overlay operations combine multiple rasters into a single output, handling conflicting values with a defined rule.

```python
# Sum overlay — add all values at each cell
sum_result = wbe.raster.overlay_math.sum_overlay(rasters=[r1, r2, r3])

# Average overlay — mean of all rasters
avg = wbe.raster.overlay_math.average_overlay(rasters=[r1, r2, r3])

# Maximum overlay — highest value among all rasters
max_r = wbe.raster.overlay_math.max_overlay(rasters=[r1, r2, r3])

# Pick from a list based on an index raster (values 1..N index which raster to use)
selected = wbe.raster.overlay_math.pick_from_list(index=index_raster, rasters=[r1, r2, r3])

# Weighted sum — apply a weight to each contributing raster
weighted = wbe.raster.overlay_math.weighted_sum(rasters=[r1, r2], weights=[0.6, 0.4])
```

### Multi-Criteria Weighted Overlay

`weighted_overlay()` normalises each factor raster, applies class weights, and sums the products — a standard MCE technique in site suitability analysis:

```python
# Suitability analysis: each raster already reclassified 1–5 scale
suitability = wbe.raster.overlay_math.weighted_overlay(
    factor_rasters=[slope_suitability, aspect_suitability, soil_suitability],
    weights=[0.4, 0.2, 0.4],
    scale_max=5.0,
    cost_factors=[False, False, False]   # True = cost (lower = better)
)
wbe.write_raster(suitability, 'suitability.tif')
```

---

## Aggregation and Resampling

### Aggregate Raster

Reduce resolution by a specified factor using a summary statistic. Useful for multi-scale analyses:

```python
# Aggregate to 5× coarser resolution using mean
coarse_dem = wbe.raster.general.aggregate_raster(dem, agg_factor=5, type='mean')

# Other types: 'sum', 'max', 'min'
coarse_lc = wbe.raster.general.aggregate_raster(landcover, agg_factor=5, type='majority')
```

### Resample

Resample a raster to match the grid of another raster or to a specified cell size. Supports several interpolation methods:

```python
# Bilinear interpolation for continuous data
resampled = wbe.remote_sensing.enhancement_contrast.resample(source_raster, base_raster=dem, method='bilinear')

# Nearest neighbour — preserves discrete categories
resampled_cat = wbe.remote_sensing.enhancement_contrast.resample(landcover, base_raster=dem, method='nn')

# Cubic convolution — smooth surfaces
resampled_cc = wbe.remote_sensing.enhancement_contrast.resample(dem, base_raster=fine_base, method='cc')
```

---

## Proximity Analysis

Proximity tools compute distances, directions, and allocations based on the locations of source cells.

### Euclidean Distance and Direction

```python
# Distance of every cell from the nearest stream cell
streams_raster = wbe.streams.network_extraction.rasterize_streams(streams_vector, base_raster=dem)
dist_to_streams = wbe.raster.distance_cost.euclidean_distance(streams_raster)
wbe.write_raster(dist_to_streams, 'dist_to_streams.tif')

# Which stream cell is each cell nearest to?
allocated = wbe.raster.distance_cost.euclidean_allocation(streams_raster)
```

### Cost-Distance Analysis

Cost-distance analysis finds the least-cost path across a landscape where movement is not uniform. The cost raster encodes the cost of traversing one cell:

```python
# Build cost and source layers
cost_surface = wbe.raster.general.raster_calculator(
    "'slope' * 0.1 + 'landcover_cost'",
    [slope, landcover_cost]
)
# Accumulate cost from source points
cost_dist = wbe.raster.distance_cost.cost_distance(source_raster, cost_surface)
wbe.write_raster(cost_dist, 'cost_distance.tif')

# Allocate each cell to the nearest source by cost
cost_alloc = wbe.raster.distance_cost.cost_allocation(source_raster, cost_distance=cost_dist)

# Trace the optimal path between two points
cost_path = wbe.raster.distance_cost.cost_pathway(source=start_raster, destination=end_raster,
                              cost_surface=cost_surface)
wbe.write_raster(cost_path, 'optimal_path.tif')
```

### Buffer Distance

Create a buffer zone around all features in a raster at a specified distance:

```python
buffered = wbe.raster.distance_cost.buffer_raster(streams_raster, size=100.0, gridcells=False)
wbe.write_raster(buffered, 'stream_buffer_100m.tif')
```

---

## Raster Object Analysis

The `clump` tool identifies connected groups of cells with the same value and assigns each group a unique identifier. This is analogous to spatial dissolve on a raster:

```python
# Identify connected patches in the land-cover raster
clumped = wbe.raster.general.clump(landcover, diag=True, zero_back=True)

# Filter out small patches (<10 cells) from a binary mask
large_patches = wbe.raster.general.filter_raster_features_by_area(binary_mask,
                                                     threshold=10,
                                                     background_val=0)

# Shape complexity of each raster object
complexity = wbe.raster.general.shape_complexity_index_raster(clumped)

# Raster area per clump
clump_area = wbe.raster.general.raster_area(clumped, out_text=False)

# Highest or lowest position among several rasters
highest = wbe.raster.overlay_math.highest_position(rasters=[r1, r2, r3])   # which raster has max value?
lowest  = wbe.raster.overlay_math.lowest_position(rasters=[r1, r2, r3])
```

---

## Raster-to-Vector Conversion

```python
# Convert raster polygons to vector polygons
polygons = wbe.conversion.raster_vector_conversion.raster_to_vector_polygons(landcover)
wbe.write_vector(polygons, 'landcover_polygons.shp')

# Convert raster lines to vector lines
lines = wbe.conversion.raster_vector_conversion.raster_to_vector_lines(stream_raster)
wbe.write_vector(lines, 'stream_lines.shp')

# Convert raster points (non-zero cells) to vector points
pts = wbe.conversion.raster_vector_conversion.raster_to_vector_points(peak_cells)
wbe.write_vector(pts, 'peaks.shp')
```

---

## Interpolation from Point Clouds

When field observation points or vector point layers represent a surface, interpolate them to a raster:

```python
sample_pts = wbe.read_vector('soil_sample_points.shp')

# Inverse Distance Weighting (IDW)
idw = wbe.raster.general.idw_interpolation(points=sample_pts, field='OM_PCT',
                             cell_size=10.0, radius=250.0, weight=2.0)
wbe.write_raster(idw, 'soil_om_idw.tif')

# Natural Neighbour (Sibson) interpolation
nn = wbe.raster.local_neighborhood.natural_neighbour_interpolation(points=sample_pts, field='OM_PCT',
                                          cell_size=10.0)
wbe.write_raster(nn, 'soil_om_nn.tif')

# Radial Basis Function interpolation
rbf = wbe.raster.general.radial_basis_function_interpolation(points=sample_pts, field='OM_PCT',
                                               cell_size=10.0, radius=250.0)
wbe.write_raster(rbf, 'soil_om_rbf.tif')

# TIN interpolation from vector points
tin = wbe.raster.general.tin_interpolation(points=sample_pts, field_name='OM_PCT', cell_size=10.0)
wbe.write_raster(tin, 'soil_om_tin.tif')
```

---

## Geostatistical Simulation

The turning-bands simulation creates spatially autocorrelated random fields — useful for uncertainty analysis and Monte Carlo simulation of landscape processes:

```python
sim = wbe.raster.general.turning_bands_simulation(
    base_raster=dem,
    range=500.0,       # autocorrelation range (metres)
    sill_height=1.0,   # sill variance
    num_lines=1000
)
wbe.write_raster(sim, 'random_field_simulated.tif')
```

---

## Statistical Tests on Raster Distributions

```python
# Test whether a raster's values are Gaussian
wbe.raster.general.ks_normality_test(dem, output_html_file='normality_test.html', num_samples=5000)

# Paired-sample t-test to compare two rasters cell-by-cell
wbe.raster.general.paired_sample_t_test(dem, dem_lidar,
                          output_html_file='ttest.html', num_samples=5000)

# Two-sample KS test for distributional differences
wbe.raster.general.two_sample_ks_test(dem, dem_lidar,
                        output_html_file='ks_test.html', num_samples=5000)

# Wilcoxon signed-rank test (non-parametric alternative to t-test)
wbe.raster.general.wilcoxon_signed_rank_test(dem, dem_lidar,
                               output_html_file='wilcoxon.html', num_samples=5000)
```

---

## Contour Generation

```python
# Contours from DEM raster
contours = wbe.vector.sampling_gridding.contours_from_raster(dem, contour_interval=5.0, base_contour=0.0,
                                     smooth=9, tolerance=5.0)
wbe.write_vector(contours, 'contours_5m.shp')
```

---

## WbW-Pro Spotlight: Field Trafficability and Operation Planning

- **Problem:** Plan equipment timing and field access under variable moisture
    and weather conditions.
- **Tool:** `field_trafficability_and_operation_planning`
- **Typical inputs:** DEM, normalized soil-moisture raster, optional
    rainfall-risk raster.
- **Typical outputs:** Trafficability score raster, operation-class raster,
    and summary outputs.

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()

result = wbe.run_tool(
    'field_trafficability_and_operation_planning',
    {
        'dem': 'field_dem.tif',
        'soil_moisture': 'soil_moisture_norm.tif',
        'rainfall_forecast': 'rainfall_risk_norm.tif',
        'output_prefix': 'field_12_trafficability'
    }
)
print(result)
```

> **Note:** This workflow requires a `WbEnvironment` initialized with a valid
> Pro licence.

---

## Complete Raster Analysis Pipeline

The following script demonstrates a typical DEM-derived terrain model workflow including correction, classification, and proximity analysis:

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
wbe.working_directory = '/data/analysis'
wbe.verbose = True
wbe.max_procs = -1  # use all available cores

# 1. Load raw DEM and correct NoData encoding
dem = wbe.read_raster('raw_dem.tif')
dem = wbe.conversion.raster_vector_conversion.set_nodata_value(dem, back_value=-9999.0)
dem = wbe.terrain.general.fill_missing_data(dem, filter_size=11)

# 2. Smooth to suppress sensor noise
dem_smooth = wbe.terrain.general.feature_preserving_smoothing(dem, filter_size=11,
                                               normal_diff_threshold=30.0,
                                               iterations=3)

# 3. Derive primary terrain attributes
slope      = wbe.terrain.derivatives.slope(dem_smooth, units='degrees')
aspect     = wbe.terrain.derivatives.aspect(dem_smooth)
tpi        = wbe.terrain.general.deviation_from_mean_elevation(dem_smooth, filter_size_x=21,
                                               filter_size_y=21)
relrough   = wbe.terrain.multiscale_signatures.multiscale_roughness(dem_smooth, min_scale=1, max_scale=50, step=2)

# 4. Classify slope into stability classes
slope_class = wbe.raster.reclass_mask.reclass(slope, reclass_vals=[
    [1, 0, 10],   # low risk
    [2, 10, 25],  # moderate risk
    [3, 25, 90],  # high risk
])

# 5. Compute Euclidean distance from streams
streams = wbe.read_raster('streams.tif')
dist_streams = wbe.raster.distance_cost.euclidean_distance(streams)

# 6. Site suitability overlay
# Normalise all factors to a 1–5 scale first ...
suitability = wbe.raster.overlay_math.weighted_overlay(
    factor_rasters=[slope_class, dist_streams_class],
    weights=[0.7, 0.3],
    scale_max=3.0
)

wbe.write_raster(suitability, 'site_suitability.tif', compress=True)
print('Raster analysis pipeline complete.')
```

---

## Tips

- **Choose your data type**: Use integers for categorical data (land cover, classifications) to minimize file size and computation time. Use floating-point (Float32 or Float64) only for continuous measurements (elevation, temperature, probability).
- **Set NoData explicitly**: Ensure your source rasters carry a valid NoData value. Missing NoData declarations can corrupt statistics and focal operations by including invalid pixels as zeros or false elevations.
- **Compress carefully**: LZW and DEFLATE compression work well for most data; avoid if you need rapid random access to interior tiles. Use COMPRESS=JPEG for photographic data only (lossy, unsuitable for analysis).
- **Focal operations require buffering**: Cells at raster edges cannot compute full neighbourhoods. Use `expand()` or accept edge effects; never assume borders are valid in derivative rasters.
- **Zonal statistics are only as good as your zones**: Ensure zone boundaries are topologically clean (no overlaps, no gaps). Overlapping zones cause double-counting; gaps cause NoData regions in output.
- **Reclassification is fast but risky**: Always validate output distributions (histogram) after reclassification. Off-by-one errors in class boundaries can silently produce wrong land-cover or suitability classes.
- **Memory is the constraint for large rasters**: Tiles > 2 GB require out-of-core or streaming processing. Use `read_by_block()` for large files; avoid loading entire rasters into memory if they exceed available RAM.
- **Upsampling introduces artifacts**: Never upsample (finer resolution) without a valid interpolation method. Nearest-neighbour upsampling creates blocky artefacts; bilinear is smoother but may violate data range (e.g. probability values > 1.0).
```
