# whiteboxworkflows R Package

whiteboxworkflows is the R package layer for wbw_r.

The package API is being modernized with emphasis on:
- session-first ergonomics,
- clearer discovery helpers,
- explicit licensing startup modes,
- practical interoperability with R spatial packages.

## Table of contents

- [Current API highlights](#current-api-highlights)
- [Migration quick map](#migration-quick-map)
- [Python to R API map](#python-to-r-api-map)
- [Development install](#development-install)
- [Quick smoke test](#quick-smoke-test)
- [Recommended examples](#recommended-examples)
- [Recommended API pattern](#recommended-api-pattern)
- [Quick start examples by workflow type](#quick-start-examples-by-workflow-type)
- [Raster output controls](#raster-output-controls)
- [Extensionless defaults](#extensionless-defaults)
- [R interoperability](#r-interoperability)
- [Supported file formats](#supported-file-formats)
- [Licensing overview](#licensing-overview)
- [Discovery APIs](#discovery-apis)
- [Tests](#tests)

## Current API highlights

- Stable session facade:
  - `wbw_session(...)`
  - `wbw_tool_ids(session = ...)`
  - `wbw_has_tool(tool_id, session = ...)`
  - `wbw_run_tool(tool_id, args = list(), session = ...)`
  - `wbw_run_tool_with_progress(...)`
- First typed object entry point:
  - `wbw_read_raster(...)`
  - `wbw_read_vector(...)`
  - `wbw_read_lidar(...)`
  - `wbw_read_bundle(...)`
  - `wbw_raster` wrapper with metadata/accessors, math methods, and raster IO helpers:
    - metadata/accessors: `metadata()`, `file_path()`, `band_count()`, `active_band()`, `crs_epsg()`, `crs_wkt()`
    - binary math: `add()`, `subtract()`, `multiply()`, `divide()`
    - unary math: `abs()`, `ceil()`, `floor()`, `round()`, `square()`, `sqrt()`, `log10()`, `log2()`, `sin()`, `cos()`, `tan()`, `sinh()`, `cosh()`, `tanh()`, `exp()`, `exp2()`
    - conversion/io: `to_array()`, `to_stars()`, `deep_copy()`, `write()`
  - `wbw_vector` wrapper with `metadata()`, `to_terra()`, and optional `to_sf()`
  - `wbw_lidar` wrapper with `metadata()`, `get_short_filename()`, `deep_copy()`, and `write()`
  - `wbw_sensor_bundle` wrapper with `metadata()`, `list_*_keys()`, `read_*()` bundle data access, preview selection, and true/false-colour composite helpers
- Discovery helpers:
  - `wbw_search_tools(...)`
  - `wbw_describe_tool(...)`
- Licensing-aware startup in one API:
  - open mode
  - signed entitlement mode
  - floating activation mode
- Raster bridge helpers:
  - `wbw_raster_to_array(...)`, `wbw_array_to_raster(...)` via `terra`
  - `wbw_raster_to_stars(...)`, `wbw_stars_to_raster(...)` via `stars`

## Migration quick map

| Earlier style | Current style |
|---|---|
| direct low-level JSON calls only | `wbw_session(...)` + helper functions |
| direct raster path handling | `wbw_read_raster(...)` |
| direct vector path handling | `wbw_read_vector(...)` |
| direct lidar path handling | `wbw_read_lidar(...)` |
| direct bundle root handling | `wbw_read_bundle(...)` |
| `wbw_list_tools(...)` for all checks | `wbw_tool_ids(...)` and `wbw_has_tool(...)` |
| manual metadata filtering over `wbw_list_tools(...)` | `wbw_search_tools(...)` and `wbw_describe_tool(...)` |
| manual progress JSON parsing | `wbw_run_tool_with_progress(...)` |
| ad hoc empty argument encoding | pass `args = list()` |

## Python to R API map

| Python | R target | Status |
|---|---|---|
| `WbEnvironment()` | `wbw_session()` | complete |
| `list_tools()` | `wbw_list_tools()`, `wbw_tool_ids()` | complete |
| `has_tool()` | `wbw_has_tool()` | complete |
| `search_tools()`, `describe_tool()` | `wbw_search_tools()`, `wbw_describe_tool()` | complete |
| `run_tool(...)` | `wbw_run_tool(..., session=)` | complete |
| progress execution helpers | `wbw_run_tool_with_progress(..., session=)` | complete |
| `read_raster(path)` | `wbw_read_raster(path)` | complete |
| raster accessors (`file_path`, `band_count`, `active_band`, CRS) | same methods on `wbw_raster` | complete |
| raster math (`add`, `subtract`, `multiply`, `divide`) | same methods on `wbw_raster` | complete |
| common unary raster math (`abs`, `sqrt`, trig/log/exp family) | same methods on `wbw_raster` | complete |
| `read_vector(path)` | `wbw_read_vector(path)` | complete |
| `read_lidar(path)` | `wbw_read_lidar(path)` | complete |
| sensor bundle reader helpers | `wbw_read_bundle()` and family-specific readers | complete |
| bundle preview/composite helper flows | `read_preview_raster()`, `write_true_colour()`, `write_false_colour()` | complete |
| raster S3 arithmetic operators (`+`, `-`, `*`, `/`) | `+.wbw_raster`, `-.wbw_raster`, `*.wbw_raster`, `/.wbw_raster` | complete (unary `-` gives clear error) |
| vector attribute table read/iterate | not yet implemented — use `to_terra()` or `to_sf()` bridge | not yet implemented |
| lidar full point-cloud array roundtrip | not yet implemented — metadata and file-backed helpers only | not yet implemented |

## Development install

```bash
R CMD INSTALL crates/wbw_r/r-package/whiteboxworkflows
```

Optional build environment variables:

- `WBW_R_PACKAGE_PRO=true` to build against the Pro runtime.
- `WBW_R_PACKAGE_RELEASE=true` to build Rust in release mode.

## Quick smoke test

```bash
Rscript -e 'library(whiteboxworkflows); s <- wbw_session(); cat(length(wbw_tool_ids(session = s)), "\n")'
```

## Recommended examples

| Order | Script | Focus |
|---|---|---|
| 1 | [inst/examples/generated_session_example.R](inst/examples/generated_session_example.R) | Session and discovery |
| 2 | [inst/examples/raster_object_quickstart.R](inst/examples/raster_object_quickstart.R) | Typed raster wrapper quickstart |
| 3 | [inst/examples/vector_object_quickstart.R](inst/examples/vector_object_quickstart.R) | Typed vector wrapper quickstart |
| 4 | [inst/examples/lidar_object_quickstart.R](inst/examples/lidar_object_quickstart.R) | Typed lidar wrapper quickstart |
| 5 | [inst/examples/sensor_bundle_quickstart.R](inst/examples/sensor_bundle_quickstart.R) | Sensor bundle inspection and data access |
| 6 | [inst/examples/sensor_bundle_multi_family_preview.R](inst/examples/sensor_bundle_multi_family_preview.R) | Multi-family bundle preview workflow |
| 7 | [inst/examples/raster_array_roundtrip.R](inst/examples/raster_array_roundtrip.R) | terra and stars roundtrip |

## Recommended API pattern

```r
library(whiteboxworkflows)

s <- wbw_session()

if (!wbw_has_tool("slope", session = s)) {
  stop("slope tool is not visible")
}

result <- wbw_run_tool(
  "slope",
  args = list(dem = "dem.tif", output = "slope.tif"),
  session = s
)
```

## Quick start examples by workflow type

### Session and discovery

```r
s <- wbw_session()
print(s)
ids <- wbw_tool_ids(session = s)
```

### Progress-aware execution

```r
result <- wbw_run_tool_with_progress(
  "slope",
  args = list(dem = "dem.tif", output = "slope.tif"),
  session = wbw_session(),
  on_progress = wbw_print_progress
)
str(result$progress)
```

For custom verbosity, use the progress-printer factory:

```r
progress_cb <- wbw_make_progress_printer(
  min_increment = 5,
  show_messages = TRUE
)

result <- wbw_run_tool_with_progress(
  "slope",
  args = list(dem = "dem.tif", output = "slope.tif"),
  session = wbw_session(),
  on_progress = progress_cb
)
```

The built-in printer also handles messages that contain embedded percentages
when a numeric `pct` value is missing (for example:
`"Progress (loop 1 of 2): 50%"`).

For a custom callback, `on_progress` receives normalized `(pct, message)` values
from the event stream. A defensive pattern is:

```r
progress_tracker <- local({
  last <- -1L
  function(pct = NA_real_, message = "") {
    msg <- if (is.null(message)) "" else as.character(message[[1]])

    if (!is.numeric(pct) || length(pct) == 0L || is.na(pct[[1]])) {
      m <- regexpr("(-?[0-9]+(\\.[0-9]+)?)\\s*%", msg, perl = TRUE)
      if (m[[1]] >= 0L) {
        token <- regmatches(msg, m)
        pct <- as.numeric(sub("%.*$", "", token))
      } else {
        pct <- NA_real_
      }
    }

    if (is.numeric(pct) && length(pct) > 0L && !is.na(pct[[1]])) {
      value <- as.numeric(pct[[1]])
      if (value <= 1.0) value <- value * 100.0
      pct_int <- as.integer(max(0, min(100, floor(value))))
      if (pct_int > last) {
        cat(sprintf("%d%%\n", pct_int))
        last <<- pct_int
      }
    }

    if (nzchar(msg)) {
      cat(msg, "\n", sep = "")
    }
  }
})

result <- wbw_run_tool_with_progress(
  "slope",
  args = list(dem = "dem.tif", output = "slope.tif"),
  session = wbw_session(),
  on_progress = progress_tracker
)
```

### Typed raster wrapper

```r
dem <- wbw_read_raster("dem.tif")
meta <- dem$metadata()
print(dem)

arr <- dem$to_array()

# Default write behavior
dem$write("dem_copy.tif", overwrite = TRUE)

# Extensionless path defaults to COG-style GeoTIFF
dem$write("dem_copy", overwrite = TRUE) # writes dem_copy.tif

# Write with explicit GeoTIFF/COG controls
dem$write(
  "dem_cog.tif",
  overwrite = TRUE,
  options = list(
    compress = TRUE,
    strict_format_options = TRUE,
    geotiff = list(
      compression = "deflate",
      bigtiff = FALSE,
      layout = "cog",
      tile_size = 512
    )
  )
)

# Session-level writer helpers
s <- wbw_session()
s$write_raster(
  dem,
  "dem_tiled.tif",
  options = list(
    geotiff = list(
      layout = "tiled",
      tile_width = 256,
      tile_height = 256
    )
  )
)
```

## Raster output controls

Raster writes can be controlled through:

- `wbw_write_raster(...)`
- `wbw_write_rasters(...)`
- `session$write_raster(...)`
- `session$write_rasters(...)`
- `wbw_raster$write(...)`

Write options are passed with `options = list(...)`.

Supported keys:

- `compress` (`TRUE`/`FALSE`): convenience GeoTIFF compression toggle.
  - `TRUE` maps to `deflate`.
  - `FALSE` maps to uncompressed GeoTIFF.
- `strict_format_options` (`TRUE`/`FALSE`): when `TRUE`, using GeoTIFF options
  with non-GeoTIFF output paths raises an error.
- `geotiff` (list): GeoTIFF/COG-specific controls.
  - `compression`: `none`, `deflate`, `lzw`, `packbits`, `jpeg`, `webp`, `jpegxl`
  - `bigtiff`: `TRUE` or `FALSE`
  - `layout`: `standard`, `stripped`, `tiled`, `cog`
  - `rows_per_strip` (for `stripped`)
  - `tile_width`, `tile_height` (for `tiled`)
  - `tile_size` (for `cog`)

Notes:

- For `.tif`/`.tiff` outputs, GeoTIFF options are applied.
- For non-GeoTIFF outputs, GeoTIFF options are ignored unless
  `strict_format_options = TRUE`.
- Backend GeoTIFF default compression is Deflate unless explicitly overridden.

### Common output profiles

```r
# 1) Standard GeoTIFF (default backend behavior)
wbw_write_raster(dem, "out_standard.tif")

# 2) Explicit stripped GeoTIFF
wbw_write_raster(
  dem,
  "out_stripped.tif",
  options = list(
    geotiff = list(
      layout = "stripped",
      rows_per_strip = 32
    )
  )
)

# 3) Explicit tiled GeoTIFF
wbw_write_raster(
  dem,
  "out_tiled.tif",
  options = list(
    geotiff = list(
      layout = "tiled",
      tile_width = 256,
      tile_height = 256
    )
  )
)

# 4) Cloud-Optimized GeoTIFF (COG)
wbw_write_raster(
  dem,
  "out_cog.tif",
  options = list(
    compress = TRUE,
    geotiff = list(
      layout = "cog",
      tile_size = 512,
      bigtiff = FALSE
    )
  )
)
```

### Extensionless defaults

When `output_path` has no extension:

- `wbw_write_raster(...)` writes COG-style GeoTIFF to `*.tif`
- `wbw_vector$write(...)` writes GeoPackage to `*.gpkg`
- `wbw_lidar$write(...)` writes COPC to `*.copc.laz`

Examples:

```r
wbw_write_raster(dem, "my_file")      # my_file.tif (COG-style default)
roads$write("my_file")                # my_file.gpkg
lidar$write("my_file", overwrite=TRUE) # my_file.copc.laz
```

### Typed vector wrapper

```r
roads <- wbw_read_vector("roads.gpkg")
meta <- roads$metadata()
print(roads)

tv <- roads$to_terra()

# Extensionless path defaults to GeoPackage
roads$write("roads_copy") # writes roads_copy.gpkg
```

### Typed lidar wrapper

```r
lidar <- wbw_read_lidar("points.las")
meta <- lidar$metadata()
print(lidar)

copied <- lidar$deep_copy("points_copy.las", overwrite = TRUE)

# Extensionless path defaults to COPC
lidar$write("points_copy", overwrite = TRUE) # writes points_copy.copc.laz

# Optional format-specific write controls
lidar$write(
  "points_copy.copc.laz",
  overwrite = TRUE,
  options = list(
    copc = list(
      max_points_per_node = 75000L,
      max_depth = 8L,
      node_point_ordering = "hilbert"
    )
  )
)

lidar$write(
  "points_copy.laz",
  overwrite = TRUE,
  options = list(
    laz = list(
      chunk_size = 25000L,
      compression_level = 7L
    )
  )
)
```

### Sensor bundle wrapper

```r
bundle <- wbw_read_bundle("LC09_SCENE")
meta <- bundle$metadata()
print(bundle)

band_keys <- bundle$list_band_keys()
qa_keys <- bundle$list_qa_keys()

if (length(band_keys) > 0) {
  preview <- bundle$read_band(band_keys[[1]])
  print(preview)
}

preview_info <- bundle$read_preview_raster()
if (!is.null(preview_info)) {
  print(preview_info$raster)
}

# Optional output writers when suitable channels are available in the bundle.
# tc <- bundle$write_true_colour("true_colour.tif")
# fc <- bundle$write_false_colour("false_colour.tif")
```

Notes:
- `write_true_colour()` and `write_false_colour()` are physically meaningful for optical bundles (e.g., Landsat/Sentinel-2).
- These helpers write derived raster outputs (e.g., GeoTIFF quicklooks); they do not write or mutate sensor bundle packages.
- For SAR bundles, these helpers use pseudo-colour defaults (for example VV/VH combinations) to provide quick-look visualization.
- Channel detection is intelligent: when called with a specific `family`, defaults are expanded by probing available keys in the bundle (bands, measurements, assets) to adapt to provider-specific naming conventions, improving robustness across SAR families.
- Some SAR SLC products may expose measurement rasters in formats not yet supported by all downstream composite paths; `read_measurement()` remains the stable fallback.

### No-argument tools

```r
# For tools with no parameters, empty args are supported.
result <- wbw_run_tool_with_progress("tool_id_with_no_args", args = list(), session = wbw_session())
```

## R interoperability

### terra bridge

```r
arr <- wbw_raster_to_array("dem.tif")
arr2 <- arr + 1
wbw_array_to_raster(arr2, "dem_plus1.tif", template_path = "dem.tif", overwrite = TRUE)
```

### stars bridge

```r
s <- wbw_raster_to_stars("dem.tif")
s2 <- s + 1
wbw_stars_to_raster(s2, "dem_plus1_stars.tif", overwrite = TRUE)
```

Install optional bridge dependencies:

```r
install.packages("terra")
install.packages("stars")
```

## Supported file formats

whiteboxworkflows file format support comes from backend crates:

- Raster formats come from [`wbraster`](../../../wbraster).
- Vector formats come from [`wbvector`](../../../wbvector).
- LiDAR formats come from [`wblidar`](../../../wblidar).

### Raster (via wbraster)

Read/write support includes:

- GeoTIFF / BigTIFF / COG (`.tif`, `.tiff`)
- JPEG2000 / GeoJP2 (`.jp2`)
- GeoPackage raster (`.gpkg`)
- ENVI (`.hdr` with sidecar data files)
- ER Mapper (`.ers`)
- Esri ASCII (`.asc`, `.grd`)
- Esri Binary Grid (`.adf` workspace)
- GRASS ASCII (`.asc`, `.txt`)
- Idrisi (`.rdc`, `.rst`)
- PCRaster (`.map`)
- SAGA (`.sgrd`, `.sdat`)
- Surfer GRD (`.grd`)
- Zarr (`.zarr`)

#### Satellite sensor bundles (read-only)

`wbraster` also supports read-only satellite sensor bundle ingestion. These are
package-level readers (bundle metadata + band/measurement/asset resolution), not
generic raster write targets.

Supported bundle families:

- Sentinel-2 SAFE
- Sentinel-1 SAFE
- Landsat Collection bundles
- ICEYE bundles
- PlanetScope bundles
- SPOT/Pleiades DIMAP bundles
- Maxar/WorldView bundles
- RADARSAT-2 bundles
- RCM bundles

### Vector (via wbvector)

Read/write support includes:

- Shapefile (`.shp` + sidecars)
- GeoPackage (`.gpkg`)
- GeoJSON (`.geojson`)
- FlatGeobuf (`.fgb`)
- GML (`.gml`)
- GPX (`.gpx`)
- KML (`.kml`)
- MapInfo Interchange (`.mif` + `.mid`)

Additional feature-gated formats in `wbvector`:

- GeoParquet (`.parquet`)
- KMZ (`.kmz`)
- OSM PBF (`.osm.pbf`, read-only)

### LiDAR (via wblidar)

Read/write support includes:

- LAS
- LAZ
- COPC
- PLY
- E57

## Licensing overview

Startup modes:

- Open mode: `wbw_session()`.
- Signed entitlement: `wbw_session(signed_entitlement_json=..., public_key_kid=..., public_key_b64url=...)`.
- Floating online activation: `wbw_session(floating_license_id=..., provider_url=...)`.

## Discovery APIs

```r
s <- wbw_session()
tools <- wbw_list_tools(session = s)
ids <- wbw_tool_ids(session = s)
has_slope <- wbw_has_tool("slope", session = s)
matches <- wbw_search_tools("lidar")
slope <- wbw_describe_tool("slope")
```

Family-specific bundle readers are also available when you know the expected product type:

```r
landsat <- wbw_read_landsat("LC09_SCENE")
sentinel2 <- wbw_read_sentinel2("S2A_SCENE.SAFE")
```

## Tests

```bash
Rscript -e 'testthat::test_local("crates/wbw_r/r-package/whiteboxworkflows")'
```

Optional real-fixture test roots:
- `WBW_TEST_DATA_ROOT` for non-SAR real fixtures (for example Sentinel-2 SAFE samples).
- `WBW_SAR_FIXTURE_ROOT` for SAR fixtures (for example `.../sar_fixtures`).

The current test suite (140+ passing tests) validates:
- low-level JSON listing,
- session facade construction and listing consistency,
- progress helper dispatch behavior,
- typed raster wrapper construction, metadata, accessors, write/copy, and math methods,
- typed vector wrapper construction, metadata, and write/copy,
- typed lidar wrapper construction, metadata, and copy helpers,
- sensor bundle wrapper construction, band/preview access, and composite helpers,
- SAR bundle composite path coverage,
- raster/vector parity gate (wrapper count vs visible manifest),
- licensing failure-path assertions (entitlement guard, invalid key, missing file, floating startup errors).
