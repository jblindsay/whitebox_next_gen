# Whitebox Workflows for Python — Geomorphometry Tools

This document covers all **Geomorphometry** tools exposed through the `WbEnvironment` API.
For common conventions, Raster I/O, and math operators see [TOOLS.md](../TOOLS.md).

---

## Geomorphometry

Terrain analysis and land-surface form tools available on `WbEnvironment`.

### Tools (Alphabetical)

- `wbe.accumulation_curvature`
- `wbe.average_normal_vector_angular_deviation`
- `wbe.aspect`
- `wbe.assess_route`
- `wbe.average_horizon_distance`
- `wbe.breakline_mapping`
- `wbe.casorati_curvature`
- `wbe.circular_variance_of_aspect`
- `wbe.contours_from_points`
- `wbe.contours_from_raster`
- `wbe.curvedness`
- `wbe.convergence_index`
- `wbe.dem_void_filling`
- `wbe.deviation_from_mean_elevation`
- `wbe.difference_curvature`
- `wbe.difference_from_mean_elevation`
- `wbe.directional_relief`
- `wbe.downslope_index`
- `wbe.edge_density`
- `wbe.embankment_mapping`
- `wbe.elev_above_pit`
- `wbe.elev_above_pit_dist`
- `wbe.elev_relative_to_min_max`
- `wbe.elev_relative_to_watershed_min_max`
- `wbe.elevation_percentile`
- `wbe.exposure_towards_wind_flux`
- `wbe.max_downslope_elev_change`
- `wbe.max_upslope_elev_change`
- `wbe.feature_preserving_smoothing`
- `wbe.gaussian_curvature`
- `wbe.fetch_analysis`
- `wbe.fill_missing_data`
- `wbe.find_ridges`
- `wbe.geomorphons`
- `wbe.generating_function`
- `wbe.horizon_angle`
- `wbe.horizon_area`
- `wbe.hillshade`
- `wbe.hypsometric_analysis`
- `wbe.hypsometrically_tinted_hillshade`
- `wbe.local_hypsometric_analysis`
- `wbe.low_points_on_headwater_divides`
- `wbe.map_off_terrain_objects`
- `wbe.horizontal_excess_curvature`
- `wbe.maximal_curvature`
- `wbe.max_difference_from_mean`
- `wbe.max_anisotropy_dev`
- `wbe.max_anisotropy_dev_signature`
- `wbe.max_branch_length`
- `wbe.max_elevation_deviation`
- `wbe.max_elev_dev_signature`
- `wbe.mean_curvature`
- `wbe.min_downslope_elev_change`
- `wbe.minimal_curvature`
- `wbe.multidirectional_hillshade`
- `wbe.multiscale_curvatures`
- `wbe.multiscale_elevated_index`
- `wbe.multiscale_elevation_percentile`
- `wbe.multiscale_low_lying_index`
- `wbe.multiscale_roughness`
- `wbe.multiscale_roughness_signature`
- `wbe.multiscale_std_dev_normals`
- `wbe.multiscale_std_dev_normals_signature`
- `wbe.multiscale_topographic_position_image`
- `wbe.num_downslope_neighbours`
- `wbe.num_upslope_neighbours`
- `wbe.openness`
- `wbe.pennock_landform_classification`
- `wbe.plan_curvature`
- `wbe.percent_elev_range`
- `wbe.principal_curvature_direction`
- `wbe.profile`
- `wbe.profile_curvature`
- `wbe.relative_aspect`
- `wbe.relative_topographic_position`
- `wbe.remove_off_terrain_objects`
- `wbe.relative_stream_power_index`
- `wbe.ring_curvature`
- `wbe.rotor`
- `wbe.ruggedness_index`
- `wbe.sediment_transport_index`
- `wbe.shape_index`
- `wbe.soil_landscape_classification`
- `wbe.spherical_std_dev_of_normals`
- `wbe.sky_view_factor`
- `wbe.shadow_animation`
- `wbe.shadow_image`
- `wbe.skyline_analysis`
- `wbe.smooth_vegetation_residual`
- `wbe.slope`
- `wbe.slope_vs_aspect_plot`
- `wbe.slope_vs_elev_plot`
- `wbe.standard_deviation_of_slope`
- `wbe.surface_area_ratio`
- `wbe.tangential_curvature`
- `wbe.time_in_daylight`
- `wbe.topographic_hachures`
- `wbe.topographic_position_animation`
- `wbe.topo_render`
- `wbe.total_curvature`
- `wbe.unsphericity`
- `wbe.visibility_index`
- `wbe.vertical_excess_curvature`
- `wbe.viewshed`
- `wbe.wetness_index`

### Basic terrain tools

#### `wbe.slope`

```
wbe.slope(input, units='degrees', z_factor=1.0, output_path=None, callback=None) -> Raster
```

Calculates slope gradient for each cell in a DEM.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `units` | `str` | `'degrees'` | Output units: `'degrees'`, `'radians'`, or `'percent'` |
| `z_factor` | `float` | `1.0` | Z conversion factor when vertical and horizontal units differ |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.aspect`

```
wbe.aspect(input, z_factor=1.0, output_path=None, callback=None) -> Raster
```

Calculates slope aspect (orientation, degrees clockwise from north) for each cell in a DEM. Flat surfaces return −1.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `z_factor` | `float` | `1.0` | Z conversion factor when vertical and horizontal units differ |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.convergence_index`

```
wbe.convergence_index(input, z_factor=1.0, output_path=None, callback=None) -> Raster
```

Calculates the convergence/divergence index based on neighbour aspects relative to the center cell.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `z_factor` | `float` | `1.0` | Z conversion factor when vertical and horizontal units differ |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.hillshade`

```
wbe.hillshade(input, azimuth=315.0, altitude=30.0, z_factor=1.0, output_path=None, callback=None) -> Raster
```

Produces a shaded-relief image from a DEM. Output values are scaled 0–32767.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `azimuth` | `float` | `315.0` | Illumination azimuth, degrees clockwise from north (0–360) |
| `altitude` | `float` | `30.0` | Illumination altitude above horizon, degrees (0–90) |
| `z_factor` | `float` | `1.0` | Z conversion factor when vertical and horizontal units differ |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

### Single-scale curvature signature
#### `wbe.multidirectional_hillshade`

```
wbe.multidirectional_hillshade(input, altitude=30.0, z_factor=1.0, full_360_mode=False, output_path=None, callback=None) -> Raster
```

Produces a weighted multi-azimuth shaded-relief image. In the default 4-direction mode illumination azimuths of 225°, 270°, 315°, and 360° are combined with weights 0.1, 0.4, 0.4, 0.1. In 360-degree mode eight evenly spaced azimuths are used. Output values are scaled 0–32767.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `altitude` | `float` | `30.0` | Illumination altitude above horizon, degrees (0–90) |
| `z_factor` | `float` | `1.0` | Z conversion factor when vertical and horizontal units differ |
| `full_360_mode` | `bool` | `False` | Use 8 azimuths (360°) instead of 4 |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.ruggedness_index`

```
wbe.ruggedness_index(input, output_path=None, callback=None) -> Raster
```

Calculates the terrain ruggedness index (TRI) after Riley et al. (1999). Each cell value is the root-mean-square deviation of the 8 surrounding elevation differences from the centre cell.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.surface_area_ratio`

```
wbe.surface_area_ratio(input, output_path=None, callback=None) -> Raster
```

Calculates the ratio of 3D surface area to planimetric cell area using the Jenness (2004) triangular-facet method. Values ≥ 1; flat terrain returns 1.0. Geographic (lat/lon) rasters apply per-row cosine scaling.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.elev_relative_to_min_max`

```
wbe.elev_relative_to_min_max(input, output_path=None, callback=None) -> Raster
```

Expresses each elevation as a percentage (0–100) of the raster's global elevation range: `(z − min) / (max − min) × 100`.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.wetness_index`

```
wbe.wetness_index(sca, slope, output_path=None, callback=None) -> Raster
```

Calculates the topographic wetness index (TWI): `ln(SCA / tan(slope))`. Cells where slope ≤ 0° or SCA ≤ 0 are output as nodata.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `sca` | `Raster` | required | Specific catchment area raster |
| `slope` | `Raster` | required | Slope raster in degrees |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.relative_stream_power_index`

```
wbe.relative_stream_power_index(sca, slope, exponent=1.0, output_path=None, callback=None) -> Raster
```

Calculates the relative stream power index: `SCA^p * tan(slope)`, where `slope` is in degrees and `p` is a user-controlled exponent.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `sca` | `Raster` | required | Specific catchment area raster |
| `slope` | `Raster` | required | Slope raster in degrees |
| `exponent` | `float` | `1.0` | Specific catchment area exponent `p` |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.sediment_transport_index`

```
wbe.sediment_transport_index(sca, slope, sca_exponent=0.4, slope_exponent=1.3, output_path=None, callback=None) -> Raster
```

Calculates the sediment transport index (LS factor): `(n + 1) * (SCA / 22.13)^n * (sin(slope) / 0.0896)^m`, where `slope` is in degrees, `n` is `sca_exponent`, and `m` is `slope_exponent`.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `sca` | `Raster` | required | Specific catchment area raster |
| `slope` | `Raster` | required | Slope raster in degrees |
| `sca_exponent` | `float` | `0.4` | Specific catchment area exponent `n` |
| `slope_exponent` | `float` | `1.3` | Slope exponent `m` |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.elev_relative_to_watershed_min_max`

```
wbe.elev_relative_to_watershed_min_max(dem, watersheds, output_path=None, callback=None) -> Raster
```

Expresses each DEM elevation as a percentage (0-100) of the min-max range within its watershed zone.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `dem` | `Raster` | required | Input DEM raster |
| `watersheds` | `Raster` | required | Watershed ID raster |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.percent_elev_range`

```
wbe.percent_elev_range(input, filter_size_x=11, filter_size_y=None, output_path=None, callback=None) -> Raster
```

Computes local topographic position as a percent of neighbourhood elevation range.

#### `wbe.relative_topographic_position`

```
wbe.relative_topographic_position(input, filter_size_x=11, filter_size_y=None, output_path=None, callback=None) -> Raster
```

Computes relative topographic position using neighbourhood min, mean, and max, scaled to [-1, 1].

#### `wbe.num_downslope_neighbours`

```
wbe.num_downslope_neighbours(input, output_path=None, callback=None) -> Raster
```

Counts the number of 8-neighbour cells that are lower than each DEM cell.

#### `wbe.num_upslope_neighbours`

```
wbe.num_upslope_neighbours(input, output_path=None, callback=None) -> Raster
```

Counts the number of 8-neighbour cells that are higher than each DEM cell.

#### `wbe.max_downslope_elev_change`

```
wbe.max_downslope_elev_change(input, output_path=None, callback=None) -> Raster
```

Computes the maximum elevation drop from each cell to any lower neighbour.

#### `wbe.max_upslope_elev_change`

```
wbe.max_upslope_elev_change(input, output_path=None, callback=None) -> Raster
```

Computes the maximum elevation gain from each cell to any higher neighbour.

#### `wbe.min_downslope_elev_change`

```
wbe.min_downslope_elev_change(input, output_path=None, callback=None) -> Raster
```

Computes the minimum non-negative elevation drop from each cell to neighbours.

#### `wbe.elevation_percentile`

```
wbe.elevation_percentile(input, filter_size_x=11, filter_size_y=None, sig_digits=2, output_path=None, callback=None) -> Raster
```

Computes the local percentile rank (0-100) of each elevation value within a moving neighbourhood.

#### `wbe.downslope_index`

```
wbe.downslope_index(input, vertical_drop=2.0, output_type='tangent', output_path=None, callback=None) -> Raster
```

Computes the Hjerdt et al. downslope index using a D8 flow path to the first point that reaches the requested vertical drop.

#### `wbe.elev_above_pit`

```
wbe.elev_above_pit(input, output_path=None, callback=None) -> Raster
```

Calculates each cell elevation above the nearest downslope pit (or edge sink).

#### `wbe.elev_above_pit_dist`

```
wbe.elev_above_pit_dist(input, output_path=None, callback=None) -> Raster
```

Compatibility alias of `wbe.elev_above_pit`.

#### `wbe.circular_variance_of_aspect`

```
wbe.circular_variance_of_aspect(input, filter=11, output_path=None, callback=None) -> Raster
```

Calculates local circular variance of aspect (0 = uniform aspect, 1 = highly variable aspect).

#### `wbe.hypsometric_analysis`

```
wbe.hypsometric_analysis(inputs, watershed=None, output_path=None, callback=None) -> str
```

Creates an HTML hypsometric curve report for one or more DEMs, with optional watershed grouping.

#### `wbe.profile`

```
wbe.profile(lines_vector, surface, output_path=None, callback=None) -> str
```

Creates an HTML profile plot of elevation versus distance sampled along one or more input polyline features.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `lines_vector` | `Vector` | required | Input polyline vector containing profile lines |
| `surface` | `Raster` | required | Input surface raster sampled along each line |
| `output_path` | `str \| None` | `None` | Output HTML report path |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.hypsometrically_tinted_hillshade`

```
wbe.hypsometrically_tinted_hillshade(input, solar_altitude=45.0, hillshade_weight=0.5, brightness=0.5, atmospheric_effects=0.0, palette='atlas', reverse_palette=False, full_360_mode=False, z_factor=1.0, output_path=None, callback=None) -> Raster
```

Creates a Swiss-style terrain rendering by blending hypsometric tint with multi-azimuth hillshade and optional atmospheric haze effects.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM/DSM raster |
| `solar_altitude` | `float` | `45.0` | Illumination altitude in degrees [0, 90] |
| `hillshade_weight` | `float` | `0.5` | Relative hillshade contribution in [0, 1] |
| `brightness` | `float` | `0.5` | Brightness tuning in [0, 1] |
| `atmospheric_effects` | `float` | `0.0` | Atmospheric haze amount in [0, 1] |
| `palette` | `str` | `'atlas'` | Hypsometric palette name |
| `reverse_palette` | `bool` | `False` | Reverse palette ordering |
| `full_360_mode` | `bool` | `False` | Use 8-direction illumination (true) instead of 4-direction mode |
| `z_factor` | `float` | `1.0` | Vertical scaling factor for elevations |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.local_hypsometric_analysis`

```
wbe.local_hypsometric_analysis(input, min_scale=4, step_size=1, num_steps=10, step_nonlinearity=1.0, output_path=None, output_scale_path=None, callback=None) -> tuple[Raster, Raster]
```

Computes local hypsometric integral (HI) across a nonlinearly sampled range of neighbourhood scales and returns two rasters: the minimum HI value found at each cell and the scale (filter size) where that minimum occurs.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `min_scale` | `int` | `4` | Minimum half-window radius in cells |
| `step_size` | `int` | `1` | Base step for scale sampling |
| `num_steps` | `int` | `10` | Number of sampled scales |
| `step_nonlinearity` | `float` | `1.0` | Nonlinearity exponent for scale spacing |
| `output_path` | `str \| None` | `None` | Optional path for HI-minimum magnitude raster |
| `output_scale_path` | `str \| None` | `None` | Optional path for scale raster |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.low_points_on_headwater_divides`

```
wbe.low_points_on_headwater_divides(dem, streams, output_path=None, callback=None) -> Vector
```

Locates low pass points on divides between neighboring headwater subbasins using a DEM-derived D8 flow network and an input streams raster.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `dem` | `Raster` | required | Input depressionless DEM raster |
| `streams` | `Raster` | required | Input stream raster (positive cells are channels) |
| `output_path` | `str \| None` | `None` | Output vector path; omit to use a temporary file |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.slope_vs_aspect_plot`

```
wbe.slope_vs_aspect_plot(input, aspect_bin_size=2.0, min_slope=0.1, z_factor=1.0, output_path=None, callback=None) -> str
```

Creates an HTML radial slope-vs-aspect plot showing 25th/50th/75th percentile slope by aspect bin.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `aspect_bin_size` | `float` | `2.0` | Aspect bin width in degrees |
| `min_slope` | `float` | `0.1` | Minimum slope threshold (degrees) included in analysis |
| `z_factor` | `float` | `1.0` | Vertical scaling factor for elevations |
| `output_path` | `str \| None` | `None` | Output HTML path |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.slope_vs_elev_plot`

```
wbe.slope_vs_elev_plot(inputs, watershed=None, output_path=None, callback=None) -> str
```

Creates an HTML slope-vs-elevation plot for one or more DEMs, with optional watershed grouping per DEM.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `inputs` | `list[Raster]` | required | One or more input DEM rasters |
| `watershed` | `list[Raster] \| None` | `None` | Optional watershed rasters matching each DEM |
| `output_path` | `str \| None` | `None` | Output HTML path |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.directional_relief`

```
wbe.directional_relief(input, azimuth=0.0, max_dist=None, output_path=None, callback=None) -> Raster
```

Calculates directional relief as the difference between each cell and the mean elevation sampled along a ray in the specified azimuth.

#### `wbe.exposure_towards_wind_flux`

```
wbe.exposure_towards_wind_flux(input, azimuth=0.0, max_dist=None, z_factor=1.0, output_path=None, callback=None) -> Raster
```

Calculates exposure of each terrain cell to a dominant wind direction by combining local slope/aspect with the upwind horizon angle.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `azimuth` | `float` | `0.0` | Dominant wind azimuth in degrees clockwise from north |
| `max_dist` | `float \| None` | `None` | Optional maximum search distance for horizon-angle tracing |
| `z_factor` | `float` | `1.0` | Z conversion factor when vertical and horizontal units differ |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.fetch_analysis`

```
wbe.fetch_analysis(input, azimuth=0.0, hgt_inc=0.05, output_path=None, callback=None) -> Raster
```

Computes upwind distance to the nearest topographic obstacle; edge-truncated fetch values are negative.

#### `wbe.relative_aspect`

```
wbe.relative_aspect(input, azimuth=0.0, z_factor=1.0, output_path=None, callback=None) -> Raster
```

Calculates aspect relative to a specified azimuth; output ranges from 0° (aligned) to 180° (opposed).

#### `wbe.edge_density`

```
wbe.edge_density(input, filter_size=11, norm_diff=5.0, z_factor=1.0, output_path=None, callback=None) -> Raster
```

Calculates local density (0–1) of breaks-in-slope based on normal-vector angular differences.

#### `wbe.find_ridges`

```
wbe.find_ridges(input, line_thin=True, output_path=None, callback=None) -> Raster
```

Identifies potential ridge and peak cells, with optional post-processing line thinning.

#### `wbe.breakline_mapping`

```
wbe.breakline_mapping(dem, threshold=0.8, min_length=3, output_path=None, callback=None) -> Vector
```

Maps breaklines by thresholding log-transformed curvedness and vectorizing connected, thinned high-curvature line features.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `dem` | `Raster` | required | Input DEM raster |
| `threshold` | `float` | `0.8` | Minimum log-curvedness threshold |
| `min_length` | `int` | `3` | Minimum output line length in grid cells |
| `output_path` | `str \| None` | `None` | Output vector path; omit to use a temporary file |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.assess_route`

```
wbe.assess_route(routes, dem, segment_length=100.0, search_radius=15, output_path=None, callback=None) -> Vector
```

Splits polyline routes into equal-length segments and computes per-segment metrics including average slope, relief, sinuosity, breaks-in-slope, and maximum local openness visibility.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `routes` | `Vector` | required | Input polyline routes vector |
| `dem` | `Raster` | required | Input projected DEM raster |
| `segment_length` | `float` | `100.0` | Target segment length in map units |
| `search_radius` | `int` | `15` | Visibility search radius in grid cells (minimum effective value is 4) |
| `output_path` | `str \| None` | `None` | Output vector path; omit to use a temporary file |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.contours_from_raster`

```
wbe.contours_from_raster(input, contour_interval=10.0, base_contour=0.0, smoothing_filter_size=9, deflection_tolerance=10.0, output_path=None, callback=None) -> Vector
```

Creates vector contour polylines from a raster surface using contour interval and base elevation controls. Optional smoothing and deflection-based simplification reduce vertex density.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster surface (e.g., DEM) |
| `contour_interval` | `float` | `10.0` | Contour interval |
| `base_contour` | `float` | `0.0` | Base contour value |
| `smoothing_filter_size` | `int` | `9` | Smoothing filter size (odd integer preferred) |
| `deflection_tolerance` | `float` | `10.0` | Minimum bend angle (degrees) retained during simplification |
| `output_path` | `str \| None` | `None` | Output vector path; defaults to `contours_from_raster.shp` in working directory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.contours_from_points`

```
wbe.contours_from_points(input, field_name=None, use_z_values=False, max_triangle_edge_length=None, contour_interval=10.0, base_contour=0.0, smoothing_filter_size=9, output_path=None, callback=None) -> Vector
```

Creates vector contour polylines from point elevations by building a Delaunay triangulation and extracting contour intersections through each triangle.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Vector` | required | Input point or multipoint vector |
| `field_name` | `str \| None` | `None` | Numeric elevation field; required when `use_z_values=False` unless a numeric field can be auto-selected |
| `use_z_values` | `bool` | `False` | Use geometry Z values for elevations |
| `max_triangle_edge_length` | `float \| None` | `None` | Optional maximum triangle edge length used in contouring |
| `contour_interval` | `float` | `10.0` | Contour interval |
| `base_contour` | `float` | `0.0` | Base contour value |
| `smoothing_filter_size` | `int` | `9` | Smoothing filter size (odd integer preferred) |
| `output_path` | `str \| None` | `None` | Output vector path; defaults to `contours_from_points.shp` in working directory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.topographic_hachures`

```
wbe.topographic_hachures(dem, contour_interval=10.0, base_contour=0.0, deflection_tolerance=10.0, filter_size=9, separation=2.0, distmin=0.5, distmax=2.0, discretization=0.5, turnmax=45.0, slopemin=0.5, depth=16, output_path=None, callback=None) -> Vector
```

Creates topographic hachure polylines from a DEM by extracting contours and tracing slope-following flowlines between and across contour levels. This OSS port preserves the legacy authorship note for the original implementation because it was not authored by John Lindsay.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `dem` | `Raster` | required | Input DEM raster |
| `contour_interval` | `float` | `10.0` | Contour interval |
| `base_contour` | `float` | `0.0` | Base contour value |
| `deflection_tolerance` | `float` | `10.0` | Minimum bend angle retained when simplifying contour seeds |
| `filter_size` | `int` | `9` | Contour smoothing filter size |
| `separation` | `float` | `2.0` | Nominal hachure seed spacing in average-cell units |
| `distmin` | `float` | `0.5` | Minimum spacing multiplier used to truncate nearby hachures |
| `distmax` | `float` | `2.0` | Maximum spacing multiplier used to insert additional hachures |
| `discretization` | `float` | `0.5` | Flowline step size in average-cell units |
| `turnmax` | `float` | `45.0` | Maximum allowed turn angle in traced hachures |
| `slopemin` | `float` | `0.5` | Minimum slope angle required for continued tracing |
| `depth` | `int` | `16` | Recursive infill depth for divergence areas |
| `output_path` | `str \| None` | `None` | Output vector path; defaults to `topographic_hachures.shp` in working directory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.topographic_position_animation`

```
wbe.topographic_position_animation(input, palette='soft', min_scale=1, num_steps=10, step_nonlinearity=1.0, image_height=600, delay=250, label='', use_dev_max=False, output_path=None, callback=None) -> tuple[str, str]
```

Creates an animated multiscale topographic position visualization, returning the generated HTML viewer path and GIF path.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `palette` | `str` | `'soft'` | Palette name used for the DEV animation |
| `min_scale` | `int` | `1` | Minimum analysis scale in cells |
| `num_steps` | `int` | `10` | Number of animation frames/scales |
| `step_nonlinearity` | `float` | `1.0` | Nonlinear exponent controlling scale spacing |
| `image_height` | `int` | `600` | Output animation height in pixels |
| `delay` | `int` | `250` | GIF frame delay in milliseconds |
| `label` | `str` | `''` | Optional label drawn in the animation viewer |
| `use_dev_max` | `bool` | `False` | Use cumulative maximum absolute DEV instead of per-step DEV |
| `output_path` | `str \| None` | `None` | Output HTML path; the GIF is written beside it |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.geomorphons`

```
wbe.geomorphons(input, search_distance=50, flatness_threshold=1.0, flatness_distance=0, skip_distance=0, output_forms=True, analyze_residuals=False, output_path=None, callback=None) -> Raster
```

Classifies landforms using the geomorphons line-of-sight method. For each direction, the tool compares zenith and nadir angle magnitudes using a ternary rule: positive (`2`) when the zenith-nadir difference exceeds the flatness threshold, negative (`0`) when it is below the negative threshold, and flat (`1`) otherwise. When `output_forms=True`, outputs the 10 common landform classes; otherwise outputs raw ternary geomorphon codes ordered counter-clockwise from east.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `search_distance` | `int` | `50` | Maximum look-up distance in cells per direction (endpoint cell is included) |
| `flatness_threshold` | `float` | `1.0` | Flatness threshold angle in degrees, applied to the zenith-nadir angle difference |
| `flatness_distance` | `int` | `0` | Distance in cells after which the flatness threshold tapers with horizon distance |
| `skip_distance` | `int` | `0` | Distance in cells skipped before beginning line-of-sight evaluation |
| `output_forms` | `bool` | `True` | Output 10 common landform classes instead of raw ternary geomorphon codes |
| `analyze_residuals` | `bool` | `False` | Detrend the DEM with a fitted linear plane before classification |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

`search_distance` and `flatness_distance` are interpreted in **cell steps** (not a fixed map-unit radius), which keeps directional scanning behavior consistent for rasters with non-square pixel sizes. Horizon angles are still computed from true map-space distances.

Common landform classes when `output_forms=True`:

```
1   Flat
2   Peak
3   Ridge
4   Shoulder
5   Spur
6   Slope
7   Hollow
8   Footslope
9   Valley
10  Pit
```

#### `wbe.pennock_landform_classification`

```
wbe.pennock_landform_classification(input, slope_threshold=3.0, prof_curv_threshold=0.1, plan_curv_threshold=0.0, z_factor=1.0, output_path=None, callback=None) -> tuple[Raster, str]
```

Classifies each DEM cell into one of seven Pennock et al. (1987) landform classes using slope, plan curvature, and profile curvature thresholds.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `slope_threshold` | `float` | `3.0` | Slope threshold in degrees used to separate level terrain |
| `prof_curv_threshold` | `float` | `0.1` | Profile curvature threshold (degrees) |
| `plan_curv_threshold` | `float` | `0.0` | Plan curvature threshold (degrees) |
| `z_factor` | `float` | `1.0` | Vertical scaling factor; if negative and CRS is geographic, an approximate value is inferred from latitude |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

Returns a `(Raster, str)` tuple where the string is a human-readable classification key:

```
1  Convergent Footslope
2  Divergent Footslope
3  Convergent Shoulder
4  Divergent Shoulder
5  Convergent Backslope
6  Divergent Backslope
7  Level
```

#### `wbe.soil_landscape_classification`

```
wbe.soil_landscape_classification(
    input,
    flat_slope_threshold=3.0,
    profile_curvature_threshold=0.01,
    plan_curvature_threshold=0.01,
    fine_scale=2.0,
    coarse_scale=8.0,
    z_factor=1.0,
    output_prefix=None,
    landform_polygons_output=None,
    callback=None,
) -> tuple[Raster, Raster, Vector, str]
```

Runs the workflow soil-landscape classifier and returns:
- landform-units raster
- multiscale-signature raster
- landform polygons vector
- summary JSON path

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `flat_slope_threshold` | `float` | `3.0` | Slope threshold in degrees for flat/summit/depression separation |
| `profile_curvature_threshold` | `float` | `0.01` | Absolute threshold for profile-curvature convex/concave separation |
| `plan_curvature_threshold` | `float` | `0.01` | Absolute threshold for plan-curvature convergent/divergent separation |
| `fine_scale` | `float` | `2.0` | Fine-scale smoothing radius |
| `coarse_scale` | `float` | `8.0` | Coarse-scale smoothing radius |
| `z_factor` | `float` | `1.0` | Vertical exaggeration factor |
| `output_prefix` | `str \| None` | `None` | Prefix for generated outputs |
| `landform_polygons_output` | `str \| None` | `None` | Optional explicit polygon output path |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.spherical_std_dev_of_normals`

```
wbe.spherical_std_dev_of_normals(input, filter_size=11, z_factor=1.0, output_path=None, callback=None) -> Raster
```

Calculates spherical standard deviation (degrees) of local surface-normal vectors.

#### `wbe.average_normal_vector_angular_deviation`

```
wbe.average_normal_vector_angular_deviation(input, filter_size=11, z_factor=1.0, output_path=None, callback=None) -> Raster
```

Calculates local mean angular deviation (degrees) between original and smoothed DEM normals.

#### `wbe.openness`

```
wbe.openness(input, dist=20, pos_output_path=None, neg_output_path=None, callback=None) -> tuple[Raster, Raster]
```

Computes Yokoyama et al. (2002) topographic openness using 8-directional horizon angles. Returns positive openness (high on convex landforms like ridges) and negative openness (high on concave landforms like valleys). Both values are in degrees (0–90).

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `dist` | `int` | `20` | Search distance in cells |
| `pos_output_path` | `str \| None` | `None` | Optional output file path for positive openness |
| `neg_output_path` | `str \| None` | `None` | Optional output file path for negative openness |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

Tier: `Pro`

#### `wbe.difference_from_mean_elevation`

```
wbe.difference_from_mean_elevation(input, filter_size_x=11, filter_size_y=None, output_path=None, callback=None) -> Raster
```

Calculates the difference between each elevation and the mean elevation of its local neighbourhood. This is a local-relief measure and is implemented using summed-area tables for efficient large-window filtering.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `filter_size_x` | `int` | `11` | Filter width in cells; values are coerced to odd sizes >= 3 |
| `filter_size_y` | `int \| None` | `None` | Filter height in cells; defaults to `filter_size_x` |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.deviation_from_mean_elevation`

```
wbe.deviation_from_mean_elevation(input, filter_size_x=11, filter_size_y=None, output_path=None, callback=None) -> Raster
```

Calculates the local elevation z-score `(z - mean) / std_dev` using a moving neighbourhood. This normalizes the local relief by local roughness and uses summed-area tables for filter-size-invariant performance.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `filter_size_x` | `int` | `11` | Filter width in cells; values are coerced to odd sizes >= 3 |
| `filter_size_y` | `int \| None` | `None` | Filter height in cells; defaults to `filter_size_x` |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.standard_deviation_of_slope`

```
wbe.standard_deviation_of_slope(input, filter_size=11, z_factor=1.0, output_path=None, callback=None) -> Raster
```

Calculates the local standard deviation of slope as a roughness measure. Slope is derived from the DEM (Horn kernel), then neighbourhood standard deviation is computed with summed-area tables.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `filter_size` | `int` | `11` | Neighbourhood width/height in cells; coerced to odd sizes >= 3 |
| `z_factor` | `float` | `1.0` | Z conversion factor when vertical and horizontal units differ |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.feature_preserving_smoothing`

```
wbe.feature_preserving_smoothing(input, filter_size=11, normal_diff_threshold=8.0, iterations=3, max_elevation_diff=None, z_factor=1.0, output_path=None, callback=None) -> Raster
```

Smooths DEM roughness while preserving terrain breaks-in-slope by filtering and applying a smoothed normal-vector field over iterative updates.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `filter_size` | `int` | `11` | Odd neighbourhood size for normal-field smoothing |
| `normal_diff_threshold` | `float` | `8.0` | Maximum angular normal difference (degrees) included in smoothing |
| `iterations` | `int` | `3` | Number of elevation update iterations |
| `max_elevation_diff` | `float \| None` | `None` | Maximum allowed absolute change from original elevation per cell |
| `z_factor` | `float` | `1.0` | Z conversion factor when vertical and horizontal units differ |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.fill_missing_data`

```
wbe.fill_missing_data(input, filter_size=11, weight=2.0, exclude_edge_nodata=False, output_path=None, callback=None) -> Raster
```

Fills NoData gaps using inverse-distance weighting of valid cells on the edge of data holes.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `filter_size` | `int` | `11` | Search radius in grid cells |
| `weight` | `float` | `2.0` | Inverse-distance power exponent |
| `exclude_edge_nodata` | `bool` | `False` | Exclude NoData regions connected to raster edges |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.remove_off_terrain_objects`

```
wbe.remove_off_terrain_objects(input, filter_size=11, slope_threshold=15.0, output_path=None, callback=None) -> Raster
```

Creates a bare-earth DEM from a surface DEM by detecting steep off-terrain objects (e.g., buildings and canopy residuals) after white top-hat normalization, then backfilling and interpolating the removed regions.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `filter_size` | `int` | `11` | Maximum expected object size in cells; coerced to odd size >= 3 |
| `slope_threshold` | `float` | `15.0` | Minimum OTO edge slope (degrees) used in backfill rule |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.map_off_terrain_objects`

```
wbe.map_off_terrain_objects(input, max_slope=90.0, min_feature_size=0, output_path=None, callback=None) -> Raster
```

Maps connected off-terrain object segments in a DSM using slope-constrained region growing; small mapped segments can be merged into a background class.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DSM/DEM raster |
| `max_slope` | `float` | `90.0` | Maximum connecting slope in degrees; lower values separate steeper objects |
| `min_feature_size` | `int` | `0` | Minimum retained segment size in cells; smaller segments are assigned to background class |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.embankment_mapping`

```
wbe.embankment_mapping(dem, roads_vector, search_dist=2.5, min_road_width=6.0, typical_embankment_width=30.0, typical_embankment_max_height=2.0, embankment_max_width=60.0, max_upwards_increment=0.05, spillout_slope=4.0, remove_embankments=False, output_path=None, output_dem_path=None, callback=None) -> Tuple[Raster, Raster | None]
```

Maps transportation embankment cells using road-constrained seed growth and morphometric criteria; optionally outputs an embankment-removed DEM interpolated from embankment-edge elevations.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `dem` | `Raster` | required | Input DEM raster |
| `roads_vector` | `Vector` | required | Input polyline road/transportation network |
| `search_dist` | `float` | `2.5` | Seed repositioning distance in map units |
| `min_road_width` | `float` | `6.0` | Minimum road-width mapping distance in map units |
| `typical_embankment_width` | `float` | `30.0` | Typical embankment width in map units |
| `typical_embankment_max_height` | `float` | `2.0` | Typical embankment maximum height |
| `embankment_max_width` | `float` | `60.0` | Maximum embankment width in map units |
| `max_upwards_increment` | `float` | `0.05` | Maximum upward increment allowed during growth |
| `spillout_slope` | `float` | `4.0` | Maximum spillout slope (degrees) for uphill transitions |
| `remove_embankments` | `bool` | `False` | Also create embankment-removed DEM output |
| `output_path` | `str \| None` | `None` | Output embankment mask raster path |
| `output_dem_path` | `str \| None` | `None` | Output embankment-removed DEM path when enabled |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.smooth_vegetation_residual`

```
wbe.smooth_vegetation_residual(input, max_scale=30, dev_threshold=1.0, scale_threshold=5, output_path=None, callback=None) -> Raster
```

Suppresses residual vegetation roughness in DEMs by identifying cells with high local standardized elevation deviation (DEV) at small scales and re-interpolating masked cells from nearby non-masked neighbours.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `max_scale` | `int` | `30` | Maximum DEV half-window radius in cells |
| `dev_threshold` | `float` | `1.0` | Minimum DEV value used to flag roughness cells |
| `scale_threshold` | `int` | `5` | Maximum scale considered vegetation roughness |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.dem_void_filling`

```
wbe.dem_void_filling(input, fill, mean_plane_dist=20, edge_treatment='dem', weight_value=2.0, output_path=None, callback=None) -> Raster
```

Fills voids in an input DEM by fusing elevations from a secondary fill DEM. The method computes a DEM-of-difference in overlap areas and interpolates near-edge offsets so void fills transition seamlessly.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster containing voids |
| `fill` | `Raster` | required | Fill DEM raster used to populate void cells |
| `mean_plane_dist` | `int` | `20` | Distance in cells from void edge beyond which offsets are set to mean overlap offset |
| `edge_treatment` | `str` | `'dem'` | Void-edge handling: `'dem'`, `'fill'`, or `'average'` |
| `weight_value` | `float` | `2.0` | IDW power for offset interpolation near void edges |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.max_branch_length`

```
wbe.max_branch_length(input, log_transform=False, output_path=None, callback=None) -> Raster
```

Calculates the maximum branch length (`Bmax`) between each cell's D8 flowpath and the flowpaths of its right and lower neighbours. High values often occur near drainage divides.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `log_transform` | `bool` | `False` | Apply natural-log transform to positive output values |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.max_difference_from_mean`

```
wbe.max_difference_from_mean(input, min_scale=1, max_scale=100, step_size=1, output_path=None, output_scale_path=None, callback=None) -> tuple[Raster, Raster]
```

Computes difference-from-mean over multiple neighbourhood scales and returns two rasters: maximum signed magnitude and the scale (half-window radius) where that maximum absolute response occurs.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `min_scale` | `int` | `1` | Minimum half-window radius in cells |
| `max_scale` | `int` | `100` | Maximum half-window radius in cells |
| `step_size` | `int` | `1` | Scale increment |
| `output_path` | `str \| None` | `None` | Optional path for magnitude raster |
| `output_scale_path` | `str \| None` | `None` | Optional path for scale raster |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.max_elevation_deviation`

```
wbe.max_elevation_deviation(input, min_scale=1, max_scale=100, step_size=1, output_path=None, output_scale_path=None, callback=None) -> tuple[Raster, Raster]
```

Computes standardized elevation deviation over multiple neighbourhood scales and returns two rasters: maximum signed DEV magnitude and the scale (half-window radius) where that maximum absolute response occurs.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `min_scale` | `int` | `1` | Minimum half-window radius in cells |
| `max_scale` | `int` | `100` | Maximum half-window radius in cells |
| `step_size` | `int` | `1` | Scale increment |
| `output_path` | `str \| None` | `None` | Optional path for DEVmax magnitude raster |
| `output_scale_path` | `str \| None` | `None` | Optional path for scale raster |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.max_anisotropy_dev`

```
wbe.max_anisotropy_dev(input, min_scale=3, max_scale=100, step_size=2, output_path=None, output_scale_path=None, callback=None) -> tuple[Raster, Raster]
```

Computes the anisotropy of standardized elevation deviation over multiple neighbourhood scales and returns two rasters: maximum anisotropy magnitude and the scale (half-window radius) where that maximum response occurs.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `min_scale` | `int` | `3` | Minimum half-window radius in cells |
| `max_scale` | `int` | `100` | Maximum half-window radius in cells |
| `step_size` | `int` | `2` | Scale increment |
| `output_path` | `str \| None` | `None` | Optional path for anisotropy magnitude raster |
| `output_scale_path` | `str \| None` | `None` | Optional path for scale raster |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.max_elev_dev_signature`

```
wbe.max_elev_dev_signature(input, points, min_scale=1, max_scale=100, step_size=10, output_path=None, callback=None) -> str
```

Generates an HTML report containing multiscale elevation-deviation (DEV) signatures for each input point site.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `points` | `Vector` | required | Input point or multipoint vector |
| `min_scale` | `int` | `1` | Minimum half-window radius in cells |
| `max_scale` | `int` | `100` | Maximum half-window radius in cells |
| `step_size` | `int` | `10` | Scale increment |
| `output_path` | `str \| None` | `None` | Optional HTML output path |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.max_anisotropy_dev_signature`

```
wbe.max_anisotropy_dev_signature(input, points, min_scale=1, max_scale=100, step_size=1, output_path=None, callback=None) -> str
```

Generates an HTML report containing multiscale anisotropy signatures for each input point site.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `points` | `Vector` | required | Input point or multipoint vector |
| `min_scale` | `int` | `1` | Minimum half-window radius in cells |
| `max_scale` | `int` | `100` | Maximum half-window radius in cells |
| `step_size` | `int` | `1` | Scale increment |
| `output_path` | `str \| None` | `None` | Optional HTML output path |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.multiscale_topographic_position_image`

```
wbe.multiscale_topographic_position_image(local, meso, broad, hillshade=None, lightness=1.2, output_path=None, callback=None) -> Raster
```

Creates a packed RGB multiscale topographic-position image from local, meso, and broad DEVmax rasters. Optionally modulates colours using a hillshade raster.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `local` | `Raster` | required | Local-scale DEVmax raster (mapped to blue channel) |
| `meso` | `Raster` | required | Meso-scale DEVmax raster (mapped to green channel) |
| `broad` | `Raster` | required | Broad-scale DEVmax raster (mapped to red channel) |
| `hillshade` | `Raster \| None` | `None` | Optional hillshade raster for illumination modulation |
| `lightness` | `float` | `1.2` | Logistic lightness scaling factor |
| `output_path` | `str \| None` | `None` | Output file path |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.multiscale_elevation_percentile`

```
wbe.multiscale_elevation_percentile(input, min_scale=4, max_scale=100, step_size=1, sig_digits=2, output_path=None, output_scale_path=None, callback=None) -> tuple[Raster, Raster]
```

Computes local elevation percentile across a range of neighbourhood scales and returns two rasters: the most extreme percentile magnitude (furthest from 50) and the scale where that response occurs.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `min_scale` | `int` | `4` | Minimum half-window radius in cells |
| `max_scale` | `int` | `100` | Maximum half-window radius in cells |
| `step_size` | `int` | `1` | Scale increment |
| `sig_digits` | `int` | `2` | Significant decimal digits preserved during percentile binning |
| `output_path` | `str \| None` | `None` | Optional path for percentile magnitude raster |
| `output_scale_path` | `str \| None` | `None` | Optional path for scale raster |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.multiscale_elevated_index`

```
wbe.multiscale_elevated_index(input, min_scale=2, step_size=1, num_steps=100, step_nonlinearity=1.1, output_path=None, output_scale_path=None, callback=None) -> tuple[Raster, Raster]
```

Computes a standardized multiscale elevated index using Gaussian scale-space smoothing. Returns two rasters: the maximum positive standardized residual magnitude and the scale where that maximum occurs.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `min_scale` | `int` | `2` | Minimum half-window radius in cells |
| `step_size` | `int` | `1` | Base step for scale sampling |
| `num_steps` | `int` | `100` | Number of sampled scales |
| `step_nonlinearity` | `float` | `1.1` | Nonlinearity exponent for scale spacing |
| `output_path` | `str \| None` | `None` | Optional path for magnitude raster |
| `output_scale_path` | `str \| None` | `None` | Optional path for scale raster |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.multiscale_low_lying_index`

```
wbe.multiscale_low_lying_index(input, min_scale=2, step_size=1, num_steps=100, step_nonlinearity=1.1, output_path=None, output_scale_path=None, callback=None) -> tuple[Raster, Raster]
```

Computes a standardized multiscale low-lying index using Gaussian scale-space smoothing. Returns two rasters: the maximum negative standardized residual magnitude (reported as positive values) and the scale where that maximum occurs.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `min_scale` | `int` | `2` | Minimum half-window radius in cells |
| `step_size` | `int` | `1` | Base step for scale sampling |
| `num_steps` | `int` | `100` | Number of sampled scales |
| `step_nonlinearity` | `float` | `1.1` | Nonlinearity exponent for scale spacing |
| `output_path` | `str \| None` | `None` | Optional path for magnitude raster |
| `output_scale_path` | `str \| None` | `None` | Optional path for scale raster |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.multiscale_roughness`

```
wbe.multiscale_roughness(input, min_scale=1, max_scale=100, step_size=1, z_factor=1.0, output_path=None, output_scale_path=None, callback=None) -> tuple[Raster, Raster]
```

Computes multiscale roughness from local angular differences between DEM surface normals and scale-smoothed normals. Returns maximum roughness magnitude and the scale where it occurs.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `min_scale` | `int` | `1` | Minimum half-window radius in cells |
| `max_scale` | `int` | `100` | Maximum half-window radius in cells |
| `step_size` | `int` | `1` | Scale increment |
| `z_factor` | `float` | `1.0` | Z conversion factor when vertical and horizontal units differ |
| `output_path` | `str \| None` | `None` | Optional path for roughness magnitude raster |
| `output_scale_path` | `str \| None` | `None` | Optional path for scale raster |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.multiscale_std_dev_normals`

```
wbe.multiscale_std_dev_normals(input, min_scale=1, step=1, num_steps=10, step_nonlinearity=1.0, z_factor=1.0, output_path=None, output_scale_path=None, callback=None) -> tuple[Raster, Raster]
```

Computes the maximum spherical standard deviation of surface normals over a nonlinearly sampled set of scales. Returns magnitude and the scale where that maximum occurs.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `min_scale` | `int` | `1` | Minimum half-window radius in cells |
| `step` | `int` | `1` | Base step used by nonlinear scale schedule |
| `num_steps` | `int` | `10` | Number of sampled scales |
| `step_nonlinearity` | `float` | `1.0` | Nonlinearity exponent for scale schedule |
| `z_factor` | `float` | `1.0` | Z conversion factor when vertical and horizontal units differ |
| `output_path` | `str \| None` | `None` | Optional path for magnitude raster |
| `output_scale_path` | `str \| None` | `None` | Optional path for scale raster |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.multiscale_std_dev_normals_signature`

```
wbe.multiscale_std_dev_normals_signature(input, points, min_scale=1, step=1, num_steps=10, step_nonlinearity=1.0, z_factor=1.0, output_path=None, callback=None) -> str
```

Generates an HTML report containing spherical-standard-deviation signatures across nonlinearly sampled scales for each input point site.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `points` | `Vector` | required | Input point or multipoint vector |
| `min_scale` | `int` | `1` | Minimum half-window radius in cells |
| `step` | `int` | `1` | Base step used by nonlinear scale schedule |
| `num_steps` | `int` | `10` | Number of sampled scales |
| `step_nonlinearity` | `float` | `1.0` | Nonlinearity exponent for scale schedule |
| `z_factor` | `float` | `1.0` | Z conversion factor when vertical and horizontal units differ |
| `output_path` | `str \| None` | `None` | Optional HTML output path |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.multiscale_roughness_signature`

```
wbe.multiscale_roughness_signature(input, points, min_scale=1, max_scale=100, step_size=1, z_factor=1.0, output_path=None, callback=None) -> str
```

Generates an HTML report containing multiscale roughness signatures for each input point site.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `points` | `Vector` | required | Input point or multipoint vector |
| `min_scale` | `int` | `1` | Minimum half-window radius in cells |
| `max_scale` | `int` | `100` | Maximum half-window radius in cells |
| `step_size` | `int` | `1` | Scale increment |
| `z_factor` | `float` | `1.0` | Z conversion factor when vertical and horizontal units differ |
| `output_path` | `str \| None` | `None` | Optional HTML output path |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.horizon_angle`

```
wbe.horizon_angle(input, azimuth=0.0, max_dist=None, output_path=None, callback=None) -> Raster
```

Computes horizon angle (maximum slope) along a specified azimuth direction.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM/DSM raster |
| `azimuth` | `float` | `0.0` | Azimuth in degrees [0, 360) |
| `max_dist` | `float \| None` | `None` | Maximum search distance in map units |
| `output_path` | `str \| None` | `None` | Output file path |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.sky_view_factor`

```
wbe.sky_view_factor(input, az_fraction=5.0, max_dist=None, observer_hgt_offset=0.05, output_path=None, callback=None) -> Raster
```

Computes sky-view factor as the proportion of visible sky from each cell.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM/DSM raster |
| `az_fraction` | `float` | `5.0` | Azimuth sampling increment in degrees [1, 45] |
| `max_dist` | `float \| None` | `None` | Maximum search distance in map units |
| `observer_hgt_offset` | `float` | `0.05` | Observer height offset above terrain |
| `output_path` | `str \| None` | `None` | Output file path |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.horizon_area`

```
wbe.horizon_area(input, az_fraction=5.0, max_dist=None, observer_hgt_offset=0.05, output_path=None, callback=None) -> Raster
```

Computes area enclosed by the horizon polygon for each cell.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM/DSM raster |
| `az_fraction` | `float` | `5.0` | Azimuth sampling increment in degrees [1, 45] |
| `max_dist` | `float \| None` | `None` | Maximum search distance in map units |
| `observer_hgt_offset` | `float` | `0.05` | Observer height offset above terrain |
| `output_path` | `str \| None` | `None` | Output file path |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.average_horizon_distance`

```
wbe.average_horizon_distance(input, az_fraction=5.0, max_dist=None, observer_hgt_offset=0.05, output_path=None, callback=None) -> Raster
```

Computes average distance to the horizon across sampled azimuth directions.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM/DSM raster |
| `az_fraction` | `float` | `5.0` | Azimuth sampling increment in degrees [1, 45] |
| `max_dist` | `float \| None` | `None` | Maximum search distance in map units |
| `observer_hgt_offset` | `float` | `0.05` | Observer height offset above terrain |
| `output_path` | `str \| None` | `None` | Output file path |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.skyline_analysis`

```
wbe.skyline_analysis(input, points, max_dist=None, observer_hgt_offset=0.05, output_as_polygons=True, az_fraction=1.0, output_path=None, report_path=None, callback=None) -> (Vector, str)
```

Computes skyline/horizon characteristics from observer point locations. Returns a horizon vector layer (polygon or line output) and an HTML report path.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM/DSM raster |
| `points` | `Vector` | required | Input point or multipoint observer locations |
| `max_dist` | `float \| None` | `None` | Maximum horizon search distance in map units |
| `observer_hgt_offset` | `float` | `0.05` | Observer height offset above terrain |
| `output_as_polygons` | `bool` | `True` | Output polygon horizon footprints when true, otherwise line strings |
| `az_fraction` | `float` | `1.0` | Azimuth sampling increment in degrees [0.01, 45] |
| `output_path` | `str \| None` | `None` | Output vector file path |
| `report_path` | `str \| None` | `None` | Output HTML report path |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.time_in_daylight`

```
wbe.time_in_daylight(input, az_fraction=5.0, max_dist=None, latitude=None, longitude=None, utc_offset=None, start_day=1, end_day=365, start_time='sunrise', end_time='sunset', output_path=None, callback=None) -> Raster
```

Computes the proportion of daytime that each cell is illuminated (not shadowed), based on sampled solar positions through the specified date and time windows. Latitude/longitude are optional overrides; when omitted they are inferred automatically from the input DEM CRS and extent center. `utc_offset` is also optional; when omitted it is estimated from the inferred/overridden center longitude.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM/DSM raster |
| `az_fraction` | `float` | `5.0` | Azimuth bin size in degrees (0, 360) |
| `max_dist` | `float \| None` | `None` | Maximum horizon search distance in map units |
| `latitude` | `float \| None` | `None` | Optional latitude override in degrees |
| `longitude` | `float \| None` | `None` | Optional longitude override in degrees |
| `utc_offset` | `str \| None` | `None` | Optional UTC offset used for almanac generation; inferred from longitude when omitted |
| `start_day` | `int` | `1` | Start day-of-year (1..366) |
| `end_day` | `int` | `365` | End day-of-year (1..366) |
| `start_time` | `str` | `'sunrise'` | Start time `HH:MM:SS` or `'sunrise'` |
| `end_time` | `str` | `'sunset'` | End time `HH:MM:SS` or `'sunset'` |
| `output_path` | `str \| None` | `None` | Output file path |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.shadow_image`

```
wbe.shadow_image(input, palette='soft', max_dist=None, date='21/06/2021', time='13:00', location='43.5448/-80.2482/-4', output_path=None, callback=None) -> Raster
```

Generates a terrain shadow-intensity image for a specified local date/time and location using horizon-angle shadowing and solar position.
The `palette` parameter controls hypsometric tinting before shadow modulation; use `white` or `none` for grayscale intensity-only output.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM/DSM raster |
| `palette` | `str` | `'soft'` | Hypsometric palette name (`atlas`, `high_relief`, `arid`, `earthtones`, `soft`, `muted`, `light_quant`, `turbo`, `purple`, `viridis`, `green_yellow`, `pink_yellow_green`, `blue_yellow_red`, `deep`, `imhof`, `blue_green_yellow`, `dem`, `grey`, `white`/`none`) |
| `max_dist` | `float \| None` | `None` | Maximum horizon search distance in map units |
| `date` | `str` | `'21/06/2021'` | Date in `DD/MM/YYYY` format |
| `time` | `str` | `'13:00'` | Local time in `HH:MM` or `HH:MMAM/PM` format |
| `location` | `str` | `'43.5448/-80.2482/-4'` | `LAT/LON/UTC_OFFSET` string |
| `output_path` | `str \| None` | `None` | Output file path |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.shadow_animation`

```
wbe.shadow_animation(input, date='21/06/2021', time_interval=30, location='43.5448/-80.2482/-4', palette='soft', max_dist=None, image_height=600, delay=250, label='', output_path=None, callback=None) -> tuple[str, str]
```

Creates an interactive terrain-shadow animation for a specified date and location, returning the generated HTML viewer path and GIF path.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM/DSM raster |
| `date` | `str` | `'21/06/2021'` | Date in `DD/MM/YYYY` format |
| `time_interval` | `int` | `30` | Frame interval in minutes |
| `location` | `str` | `'43.5448/-80.2482/-4'` | `LAT/LON/UTC_OFFSET` string |
| `palette` | `str` | `'soft'` | Hypsometric palette name used in the rendered frames |
| `max_dist` | `float \| None` | `None` | Maximum horizon search distance in map units |
| `image_height` | `int` | `600` | Output animation height in pixels |
| `delay` | `int` | `250` | GIF frame delay in milliseconds |
| `label` | `str` | `''` | Optional label drawn in the animation viewer |
| `output_path` | `str \| None` | `None` | Output HTML path; the GIF is written beside it |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.topo_render`

```
wbe.topo_render(input, palette='soft', reverse_palette=False, azimuth=315.0, altitude=30.0, clipping_polygon=None, background_hgt_offset=10.0, background_clr=(255, 255, 255, 255), attenuation_parameter=0.3, ambient_light=0.2, z_factor=1.0, max_dist=None, output_path=None, callback=None) -> Raster
```

Creates a pseudo-3D rendered topographic image using hypsometric tinting, hillshade, horizon-based shadowing, and distance attenuation.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM/DSM raster |
| `palette` | `str` | `'soft'` | Hypsometric palette name (`atlas`, `high_relief`, `arid`, `earthtones`, `soft`, `muted`, `light_quant`, `turbo`, `purple`, `viridis`, `green_yellow`, `pink_yellow_green`, `blue_yellow_red`, `deep`, `imhof`, `blue_green_yellow`, `dem`, `grey`, `white`/`none`) |
| `reverse_palette` | `bool` | `False` | Reverse palette ordering |
| `azimuth` | `float` | `315.0` | Light-source azimuth in degrees [0, 360] |
| `altitude` | `float` | `30.0` | Light-source altitude in degrees [0, 90] |
| `clipping_polygon` | `Vector \| None` | `None` | Optional polygon vector mask; only DEM cells inside polygon(s) are rendered |
| `background_hgt_offset` | `float` | `10.0` | Vertical offset from minimum DEM elevation to background plane |
| `background_clr` | `tuple[int,int,int,int]` | `(255, 255, 255, 255)` | Background colour as RGBA |
| `attenuation_parameter` | `float` | `0.3` | Distance attenuation exponent |
| `ambient_light` | `float` | `0.2` | Ambient light amount in [0, 1] |
| `z_factor` | `float` | `1.0` | Vertical exaggeration multiplier |
| `max_dist` | `float \| None` | `None` | Maximum horizon search distance in map units |
| `output_path` | `str \| None` | `None` | Output file path |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

#### `wbe.visibility_index`

```
wbe.visibility_index(input, station_height=2.0, resolution_factor=8, max_dist=None, output_path=None, callback=None) -> Raster
```

Computes a terrain visibility index using sampled viewsheds.

#### `wbe.viewshed`

```
wbe.viewshed(input, stations, height=2.0, output_path=None, callback=None) -> Raster
```

Computes per-cell visibility counts from one or more viewing stations in a point vector layer.

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `station_height` | `float` | `2.0` | Observer station height above terrain |
| `resolution_factor` | `int` | `8` | Sampling resolution factor in [1, 25] |
| `max_dist` | `float \| None` | `None` | Maximum search distance in map units; omit for full extent |
| `output_path` | `str \| None` | `None` | Output file path |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

### Single-scale curvature signature

All single-scale curvature tools share this signature:

```
wbe.<tool>(input, z_factor=1.0, log_transform=False, output_path=None, callback=None) -> Raster
```

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input DEM raster |
| `z_factor` | `float` | `1.0` | Z conversion factor |
| `log_transform` | `bool` | `False` | Apply log transform to output values |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

### Tier map

| Method | Tool ID | Tier |
|--------|---------|------|
| `wbe.slope(...)` | `slope` | OSS |
| `wbe.aspect(...)` | `aspect` | OSS |
| `wbe.convergence_index(...)` | `convergence_index` | OSS |
| `wbe.contours_from_points(...)` | `contours_from_points` | OSS |
| `wbe.contours_from_raster(...)` | `contours_from_raster` | OSS |
| `wbe.dem_void_filling(...)` | `dem_void_filling` | Pro |
| `wbe.max_branch_length(...)` | `max_branch_length` | OSS |
| `wbe.hillshade(...)` | `hillshade` | OSS |
| `wbe.multidirectional_hillshade(...)` | `multidirectional_hillshade` | OSS |
| `wbe.ruggedness_index(...)` | `ruggedness_index` | OSS |
| `wbe.surface_area_ratio(...)` | `surface_area_ratio` | OSS |
| `wbe.elev_relative_to_min_max(...)` | `elev_relative_to_min_max` | OSS |
| `wbe.wetness_index(...)` | `wetness_index` | OSS |
| `wbe.relative_stream_power_index(...)` | `relative_stream_power_index` | OSS |
| `wbe.sediment_transport_index(...)` | `sediment_transport_index` | OSS |
| `wbe.elev_relative_to_watershed_min_max(...)` | `elev_relative_to_watershed_min_max` | OSS |
| `wbe.percent_elev_range(...)` | `percent_elev_range` | OSS |
| `wbe.relative_topographic_position(...)` | `relative_topographic_position` | OSS |
| `wbe.num_downslope_neighbours(...)` | `num_downslope_neighbours` | OSS |
| `wbe.num_upslope_neighbours(...)` | `num_upslope_neighbours` | OSS |
| `wbe.max_downslope_elev_change(...)` | `max_downslope_elev_change` | OSS |
| `wbe.max_upslope_elev_change(...)` | `max_upslope_elev_change` | OSS |
| `wbe.min_downslope_elev_change(...)` | `min_downslope_elev_change` | OSS |
| `wbe.elevation_percentile(...)` | `elevation_percentile` | OSS |
| `wbe.downslope_index(...)` | `downslope_index` | OSS |
| `wbe.elev_above_pit(...)` | `elev_above_pit` | OSS |
| `wbe.elev_above_pit_dist(...)` | `elev_above_pit_dist` | OSS |
| `wbe.circular_variance_of_aspect(...)` | `circular_variance_of_aspect` | OSS |
| `wbe.hypsometric_analysis(...)` | `hypsometric_analysis` | OSS |
| `wbe.profile(...)` | `profile` | OSS |
| `wbe.hypsometrically_tinted_hillshade(...)` | `hypsometrically_tinted_hillshade` | OSS |
| `wbe.slope_vs_aspect_plot(...)` | `slope_vs_aspect_plot` | OSS |
| `wbe.slope_vs_elev_plot(...)` | `slope_vs_elev_plot` | OSS |
| `wbe.directional_relief(...)` | `directional_relief` | OSS |
| `wbe.exposure_towards_wind_flux(...)` | `exposure_towards_wind_flux` | OSS |
| `wbe.fetch_analysis(...)` | `fetch_analysis` | OSS |
| `wbe.relative_aspect(...)` | `relative_aspect` | OSS |
| `wbe.edge_density(...)` | `edge_density` | OSS |
| `wbe.find_ridges(...)` | `find_ridges` | OSS |
| `wbe.assess_route(...)` | `assess_route` | OSS |
| `wbe.breakline_mapping(...)` | `breakline_mapping` | OSS |
| `wbe.geomorphons(...)` | `geomorphons` | OSS |
| `wbe.pennock_landform_classification(...)` | `pennock_landform_classification` | OSS |
| `wbe.spherical_std_dev_of_normals(...)` | `spherical_std_dev_of_normals` | OSS |
| `wbe.average_normal_vector_angular_deviation(...)` | `average_normal_vector_angular_deviation` | OSS |
| `wbe.difference_from_mean_elevation(...)` | `difference_from_mean_elevation` | OSS |
| `wbe.deviation_from_mean_elevation(...)` | `deviation_from_mean_elevation` | OSS |
| `wbe.standard_deviation_of_slope(...)` | `standard_deviation_of_slope` | OSS |
| `wbe.feature_preserving_smoothing(...)` | `feature_preserving_smoothing` | OSS |
| `wbe.fill_missing_data(...)` | `fill_missing_data` | OSS |
| `wbe.remove_off_terrain_objects(...)` | `remove_off_terrain_objects` | OSS |
| `wbe.low_points_on_headwater_divides(...)` | `low_points_on_headwater_divides` | OSS |
| `wbe.map_off_terrain_objects(...)` | `map_off_terrain_objects` | OSS |
| `wbe.embankment_mapping(...)` | `embankment_mapping` | OSS |
| `wbe.local_hypsometric_analysis(...)` | `local_hypsometric_analysis` | OSS |
| `wbe.smooth_vegetation_residual(...)` | `smooth_vegetation_residual` | OSS |
| `wbe.max_difference_from_mean(...)` | `max_difference_from_mean` | OSS |
| `wbe.max_anisotropy_dev(...)` | `max_anisotropy_dev` | OSS |
| `wbe.max_anisotropy_dev_signature(...)` | `max_anisotropy_dev_signature` | OSS |
| `wbe.max_elevation_deviation(...)` | `max_elevation_deviation` | OSS |
| `wbe.max_elev_dev_signature(...)` | `max_elev_dev_signature` | OSS |
| `wbe.multiscale_elevated_index(...)` | `multiscale_elevated_index` | OSS |
| `wbe.multiscale_elevation_percentile(...)` | `multiscale_elevation_percentile` | OSS |
| `wbe.multiscale_low_lying_index(...)` | `multiscale_low_lying_index` | OSS |
| `wbe.multiscale_roughness(...)` | `multiscale_roughness` | OSS |
| `wbe.multiscale_roughness_signature(...)` | `multiscale_roughness_signature` | OSS |
| `wbe.multiscale_std_dev_normals(...)` | `multiscale_std_dev_normals` | OSS |
| `wbe.multiscale_std_dev_normals_signature(...)` | `multiscale_std_dev_normals_signature` | OSS |
| `wbe.multiscale_topographic_position_image(...)` | `multiscale_topographic_position_image` | OSS |
| `wbe.horizon_angle(...)` | `horizon_angle` | OSS |
| `wbe.sky_view_factor(...)` | `sky_view_factor` | OSS |
| `wbe.horizon_area(...)` | `horizon_area` | OSS |
| `wbe.average_horizon_distance(...)` | `average_horizon_distance` | OSS |
| `wbe.skyline_analysis(...)` | `skyline_analysis` | OSS |
| `wbe.time_in_daylight(...)` | `time_in_daylight` | OSS |
| `wbe.shadow_image(...)` | `shadow_image` | OSS |
| `wbe.shadow_animation(...)` | `shadow_animation` | OSS |
| `wbe.topographic_hachures(...)` | `topographic_hachures` | OSS |
| `wbe.topographic_position_animation(...)` | `topographic_position_animation` | OSS |
| `wbe.topo_render(...)` | `topo_render` | OSS |
| `wbe.visibility_index(...)` | `visibility_index` | OSS |
| `wbe.viewshed(...)` | `viewshed` | OSS |
| `wbe.plan_curvature(...)` | `plan_curvature` | OSS |
| `wbe.profile_curvature(...)` | `profile_curvature` | OSS |
| `wbe.tangential_curvature(...)` | `tangential_curvature` | OSS |
| `wbe.total_curvature(...)` | `total_curvature` | OSS |
| `wbe.mean_curvature(...)` | `mean_curvature` | OSS |
| `wbe.gaussian_curvature(...)` | `gaussian_curvature` | OSS |
| `wbe.minimal_curvature(...)` | `minimal_curvature` | OSS |
| `wbe.maximal_curvature(...)` | `maximal_curvature` | OSS |
| `wbe.shape_index(...)` | `shape_index` | OSS |
| `wbe.curvedness(...)` | `curvedness` | OSS |
| `wbe.unsphericity(...)` | `unsphericity` | OSS |
| `wbe.ring_curvature(...)` | `ring_curvature` | OSS |
| `wbe.rotor(...)` | `rotor` | OSS |
| `wbe.difference_curvature(...)` | `difference_curvature` | OSS |
| `wbe.horizontal_excess_curvature(...)` | `horizontal_excess_curvature` | OSS |
| `wbe.vertical_excess_curvature(...)` | `vertical_excess_curvature` | OSS |
| `wbe.accumulation_curvature(...)` | `accumulation_curvature` | OSS |
| `wbe.generating_function(...)` | `generating_function` | OSS |
| `wbe.principal_curvature_direction(...)` | `principal_curvature_direction` | OSS |
| `wbe.casorati_curvature(...)` | `casorati_curvature` | OSS |
| `wbe.openness(...)` | `openness` | OSS |

### `wbe.multiscale_curvatures`

```
wbe.multiscale_curvatures(
    input,
    curv_type='profile',
    min_scale=4,
    step=1,
    num_steps=10,
    step_nonlinearity=1.0,
    log_transform=True,
    standardize=False,
    output_path=None,
    callback=None,
) -> Raster
```

Tier: `OSS`

`multiscale_curvatures` always uses Gaussian scale-space smoothing by default. There is no hidden runtime switch to toggle to box smoothing.

**Example**

```python
ms = wbe.multiscale_curvatures(
    dem,
    curv_type='shape_index',
    min_scale=1,
    step=2,
    num_steps=8,
    step_nonlinearity=1.0,
)
```

### Basic terrain examples

```python
slope_deg  = wbe.slope(dem)                             # degrees
slope_pct  = wbe.slope(dem, units='percent')
asp        = wbe.aspect(dem)
hs         = wbe.hillshade(dem, azimuth=315.0, altitude=30.0)
hs_steep   = wbe.hillshade(dem, azimuth=270.0, altitude=45.0, z_factor=2.0)
mhs        = wbe.multidirectional_hillshade(dem, altitude=30.0)
mhs_360    = wbe.multidirectional_hillshade(dem, altitude=30.0, full_360_mode=True)
tri        = wbe.ruggedness_index(dem)
sar        = wbe.surface_area_ratio(dem)
elev_pct   = wbe.elev_relative_to_min_max(dem)
twi        = wbe.wetness_index(sca, slope_deg)
rsp        = wbe.relative_stream_power_index(sca, slope_deg)
sti        = wbe.sediment_transport_index(sca, slope_deg)
erwm       = wbe.elev_relative_to_watershed_min_max(dem, watersheds)
per        = wbe.percent_elev_range(dem, filter_size_x=11)
rtp        = wbe.relative_topographic_position(dem, filter_size_x=11)
n_dn       = wbe.num_downslope_neighbours(dem)
max_drop   = wbe.max_downslope_elev_change(dem)
min_drop   = wbe.min_downslope_elev_change(dem)
diff_mean  = wbe.difference_from_mean_elevation(dem, filter_size_x=11)
dev_mean   = wbe.deviation_from_mean_elevation(dem, filter_size_x=11)
std_slope  = wbe.standard_deviation_of_slope(dem, filter_size=11)
pos_open, neg_open = wbe.openness(dem, dist=20)  # Pro: positive and negative openness
max_diff, max_scale = wbe.max_difference_from_mean(dem, min_scale=1, max_scale=100, step_size=2)
max_dev, dev_scale = wbe.max_elevation_deviation(dem, min_scale=1, max_scale=100, step_size=2)
mep, mep_scale = wbe.multiscale_elevation_percentile(dem, min_scale=4, max_scale=100, step_size=2, sig_digits=2)
# local_dev, meso_dev, and broad_dev are DEVmax rasters computed for distinct scale ranges.
mtp        = wbe.multiscale_topographic_position_image(local_dev, meso_dev, broad_dev)
svf        = wbe.sky_view_factor(dem, az_fraction=5.0)
hzn_ang    = wbe.horizon_angle(dem, azimuth=315.0)
hzn_area   = wbe.horizon_area(dem)
hzn_dist   = wbe.average_horizon_distance(dem)
vis_idx    = wbe.visibility_index(dem, station_height=2.0)
```

### Openness tuple workflow

```python
# 1) Compute positive and negative openness.
pos_open, neg_open = wbe.openness(dem, dist=20)

# 2) Visualize positive openness (ridges/peaks).
pos_hs = wbe.hillshade(pos_open, azimuth=315.0, altitude=30.0)

# 3) Visualize negative openness (valleys/basins).
neg_hs = wbe.hillshade(neg_open, azimuth=315.0, altitude=30.0)
```

### `max_difference_from_mean` tuple workflow

```python
# 1) Compute multiscale local-relief response.
max_diff, max_scale = wbe.max_difference_from_mean(
    dem,
    min_scale=1,
    max_scale=75,
    step_size=2,
)

# 2) Use magnitude for visualization or thresholding.
max_diff_hs = wbe.hillshade(max_diff, azimuth=315.0, altitude=35.0)

# 3) Use the scale raster to inspect dominant feature size.
#    Larger values indicate broader landform scale.
max_scale.write("dominant_scale.tif")
```

Returns:
- first raster: maximum signed difference-from-mean magnitude.
- second raster: half-window radius (cells) at which the maximum absolute response occurs.

### `max_elevation_deviation` tuple workflow

```python
# 1) Compute multiscale standardized local-relief response (DEVmax).
max_dev, dev_scale = wbe.max_elevation_deviation(
    dem,
    min_scale=1,
    max_scale=75,
    step_size=2,
)

# 2) Visualize magnitude response.
max_dev_hs = wbe.hillshade(max_dev, azimuth=315.0, altitude=35.0)

# 3) Use scale raster to inspect dominant topographic-position scale.
dev_scale.write("dominant_dev_scale.tif")
```

Returns:
- first raster: maximum signed standardized deviation magnitude.
- second raster: half-window radius (cells) at which the maximum absolute response occurs.

### DEVmax-to-MTP workflow (concrete scale ranges)

```python
# Build three DEVmax rasters using distinct neighbourhood-scale ranges.
# Example ranges:
#   local = 1..8 cells
#   meso  = 9..32 cells
#   broad = 33..128 cells
local_dev, local_scale = wbe.max_elevation_deviation(
    dem,
    min_scale=1,
    max_scale=8,
    step_size=1,
)
meso_dev, meso_scale = wbe.max_elevation_deviation(
    dem,
    min_scale=9,
    max_scale=32,
    step_size=2,
)
broad_dev, broad_scale = wbe.max_elevation_deviation(
    dem,
    min_scale=33,
    max_scale=128,
    step_size=4,
)

# Optional illumination surface for visual clarity.
shade = wbe.multidirectional_hillshade(dem, altitude=35.0, full_360_mode=True)

# Compose the multiscale topographic position image.
mtp = wbe.multiscale_topographic_position_image(
    local_dev,
    meso_dev,
    broad_dev,
    hillshade=shade,
    lightness=1.2,
    output_path="mtp_local_meso_broad.tif",
)
```

Interpretation guide:
- blue-dominant areas: strongest topographic prominence at local scales
- green-dominant areas: strongest topographic prominence at meso scales
- red-dominant areas: strongest topographic prominence at broad scales

### `multiscale_elevation_percentile` tuple workflow

```python
# 1) Compute multiscale elevation percentile response.
mep, mep_scale = wbe.multiscale_elevation_percentile(
    dem,
    min_scale=4,
    max_scale=100,
    step_size=2,
    sig_digits=2,
)

# 2) Visualize extreme percentile magnitude.
mep_hs = wbe.hillshade(mep, azimuth=315.0, altitude=35.0)

# 3) Inspect the scale of strongest percentile response.
mep_scale.write("multiscale_elevation_percentile_scale.tif")
```

Returns:
- first raster: most extreme elevation percentile (furthest from 50).
- second raster: half-window radius (cells) where that extreme percentile occurs.

### Single-scale examples

```python
plan = wbe.plan_curvature(dem, z_factor=1.0)
mean = wbe.mean_curvature(dem, z_factor=1.0, log_transform=False)
minc = wbe.minimal_curvature(dem, z_factor=1.0)
genf = wbe.generating_function(dem, z_factor=1.0)
pcd = wbe.principal_curvature_direction(dem, z_factor=1.0)
```

---

