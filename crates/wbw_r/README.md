# Whitebox Workflows for R

Whitebox Workflows for R is the R interface for the Whitebox backend runtime.

The API is in active modernization, with emphasis on:
- clearer session ergonomics,
- better discoverability in editors,
- robust package-native loading through extendr,
- practical interoperability with R spatial tooling.

**THIS CRATE IS CURRENTLY EXPERIMENTAL AND IS IN AN EARLY DEVELOPMENTAL STAGE. IT IS NOT INTENDED FOR PUBLIC USAGE AT PRESENT.**

## Parity status

Current parity against `wbw_python` is uneven across layers.

| Layer | Status | Notes |
|---|---|---|
| Tool call coverage | High | Generated wrappers and facade expose most visible tools. |
| Runtime and licensing | High | Open, entitlement, and floating startup paths are implemented. |
| Typed data-object workflows | Partial | Raster, vector, lidar, and sensor-bundle wrappers now exist, including bundle key-list/read helpers plus preview and true/false-colour composite helper methods; broader family ergonomics remain. |
| Docs and examples | Partial | Structure is closer to Python and object quickstarts now exist for raster/vector/lidar/sensor bundles, including preview/composite helper examples, but advanced family-specific flows are still thinner than Python. |

Execution plan:
- [R_API_PARITY_PLAN.md](R_API_PARITY_PLAN.md)

## Table of contents

- [Parity status](#parity-status)
- [Current API highlights](#current-api-highlights)
- [Migration quick map](#migration-quick-map)
- [Python to R API map](#python-to-r-api-map)
- [Tool reference docs](#tool-reference-docs)
- [Development install](#development-install)
- [Quick smoke test](#quick-smoke-test)
- [Recommended examples](#recommended-examples)
- [Recommended API pattern](#recommended-api-pattern)
- [Quick start examples by workflow type](#quick-start-examples-by-workflow-type)
- [Progress and feedback model](#progress-and-feedback-model)
- [R interoperability strategy](#r-interoperability-strategy)
- [terra interoperability](#terra-interoperability)
- [stars interoperability](#stars-interoperability)
- [Licensing overview](#licensing-overview)
- [Licensing and Pro workflows](#licensing-and-pro-workflows)
- [Discovery APIs](#discovery-apis)
- [Generated wrappers and package scaffold](#generated-wrappers-and-package-scaffold)
- [Testing](#testing)

## Current API highlights

- Session-centric facade for idiomatic R usage:
  - `wbw_session(...)`
  - `wbw_tool_ids(session = ...)`
  - `wbw_has_tool(tool_id, session = ...)`
  - `wbw_run_tool(tool_id, args = list(), session = ...)`
  - `wbw_run_tool_with_progress(...)`
- Typed object slices now available:
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
  - `wbw_sensor_bundle` wrapper with `metadata()`, `list_*_keys()`, `read_*()`, preview selection, and true/false-colour composite helpers
- Discovery helpers:
  - `wbw_search_tools(...)`
  - `wbw_describe_tool(...)`
- Licensing-aware startup support in the facade:
  - open mode
  - signed entitlement mode
  - floating online activation mode
- R-native raster bridge helpers:
  - `wbw_raster_to_array(...)` and `wbw_array_to_raster(...)` via `terra`
  - `wbw_raster_to_stars(...)` and `wbw_stars_to_raster(...)` via `stars`
- Package-native integration path through `R CMD INSTALL` and extendr exports.

## Migration quick map

Common updates from low-level JSON calls toward the stable R facade:

| Earlier style | Current style |
|---|---|
| `whitebox_tools(...)` only | `wbw_session(...)` for explicit session construction |
| ad hoc raster path handling | `wbw_read_raster(...)` for typed raster wrapper construction |
| ad hoc vector path handling | `wbw_read_vector(...)` for typed vector wrapper construction |
| ad hoc lidar path handling | `wbw_read_lidar(...)` for typed lidar wrapper construction |
| ad hoc bundle root handling | `wbw_read_bundle(...)` for typed bundle wrapper construction |
| `wbw_list_tools(...)` for all checks | `wbw_tool_ids(...)` and `wbw_has_tool(...)` for fast checks |
| manual tool metadata filtering | `wbw_search_tools(...)` and `wbw_describe_tool(...)` |
| `run_tool_json_with_options(...)` direct usage | `wbw_run_tool(..., session = s)` |
| custom progress parsing from JSON | `wbw_run_tool_with_progress(...)` |
| ad hoc empty arg encoding | pass `args = list()` and let facade encode properly |

Notes:
- Low-level JSON functions remain available and useful for wrappers/tooling.
- The facade is the recommended user-facing surface.

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

## Tool reference docs

The R and Python bindings share the same backend registry, so Python tool docs are
currently the most complete catalog of tool-level parameters and behavior:

- [../wbw_python/TOOLS.md](../wbw_python/TOOLS.md)
- [../wbw_python/docs/tools_hydrology.md](../wbw_python/docs/tools_hydrology.md)
- [../wbw_python/docs/tools_gis.md](../wbw_python/docs/tools_gis.md)
- [../wbw_python/docs/tools_remote_sensing.md](../wbw_python/docs/tools_remote_sensing.md)
- [../wbw_python/docs/tools_geomorphometry.md](../wbw_python/docs/tools_geomorphometry.md)
- [../wbw_python/docs/tools_agriculture.md](../wbw_python/docs/tools_agriculture.md)
- [../wbw_python/docs/tools_lidar_processing.md](../wbw_python/docs/tools_lidar_processing.md)
- [../wbw_python/docs/tools_stream_network_analysis.md](../wbw_python/docs/tools_stream_network_analysis.md)

R package-level usage docs:
- [r-package/whiteboxworkflows/README.md](r-package/whiteboxworkflows/README.md)

## Development install

From workspace root:

```bash
R CMD INSTALL crates/wbw_r/r-package/whiteboxworkflows
```

Optional build environment variables:

- `WBW_R_PACKAGE_PRO=true` to build against a Pro-enabled runtime.
- `WBW_R_PACKAGE_RELEASE=true` to compile Rust in release mode.

## Quick smoke test

```bash
Rscript -e 'library(whiteboxworkflows); s <- wbw_session(); cat(length(wbw_tool_ids(session = s)), "\n")'
```

## Recommended examples

Suggested run order for new users:

| Order | Script | Focus |
|---|---|---|
| 1 | [examples/generated_session_example.R](examples/generated_session_example.R) | Basic session and tool invocation |
| 2 | [r-package/whiteboxworkflows/inst/examples/raster_object_quickstart.R](r-package/whiteboxworkflows/inst/examples/raster_object_quickstart.R) | Typed raster wrapper quickstart |
| 3 | [r-package/whiteboxworkflows/inst/examples/vector_object_quickstart.R](r-package/whiteboxworkflows/inst/examples/vector_object_quickstart.R) | Typed vector wrapper quickstart |
| 4 | [r-package/whiteboxworkflows/inst/examples/lidar_object_quickstart.R](r-package/whiteboxworkflows/inst/examples/lidar_object_quickstart.R) | Typed lidar wrapper quickstart |
| 5 | [r-package/whiteboxworkflows/inst/examples/sensor_bundle_quickstart.R](r-package/whiteboxworkflows/inst/examples/sensor_bundle_quickstart.R) | Sensor bundle inspection and data access |
| 6 | [r-package/whiteboxworkflows/inst/examples/sensor_bundle_multi_family_preview.R](r-package/whiteboxworkflows/inst/examples/sensor_bundle_multi_family_preview.R) | Multi-family bundle preview workflow |
| 7 | [r-package/whiteboxworkflows/inst/examples/raster_array_roundtrip.R](r-package/whiteboxworkflows/inst/examples/raster_array_roundtrip.R) | terra/stars raster exchange |
| 8 | [examples/licensing_offline.R](examples/licensing_offline.R) | Signed entitlement startup |
| 9 | [examples/licensing_floating_online.R](examples/licensing_floating_online.R) | Floating online startup |

## Recommended API pattern

```r
library(whiteboxworkflows)

s <- wbw_session()

ids <- wbw_tool_ids(session = s)
if (!wbw_has_tool("slope", session = s)) {
  stop("slope is not visible in this session")
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
library(whiteboxworkflows)

s <- wbw_session()
print(s)

ids <- wbw_tool_ids(session = s)
cat("Visible tools:", length(ids), "\n")
```

### Tool execution with explicit session

```r
library(whiteboxworkflows)

s <- wbw_session()
out <- wbw_run_tool(
  "slope",
  args = list(dem = "dem.tif", output = "slope.tif"),
  session = s
)
str(out)
```

### Typed raster wrapper

```r
library(whiteboxworkflows)

dem <- wbw_read_raster("dem.tif")
meta <- dem$metadata()
print(dem)

arr <- dem$to_array()
```

### Typed vector wrapper

```r
library(whiteboxworkflows)

roads <- wbw_read_vector("roads.gpkg")
meta <- roads$metadata()
print(roads)
```

### Typed lidar wrapper

```r
library(whiteboxworkflows)

lidar <- wbw_read_lidar("points.las")
meta <- lidar$metadata()
print(lidar)

copy <- lidar$deep_copy("points_copy.las", overwrite = TRUE)
```

### Sensor bundle wrapper

```r
library(whiteboxworkflows)

bundle <- wbw_read_bundle("LC09_SCENE")
meta <- bundle$metadata()
print(bundle)

keys <- bundle$list_band_keys()
if (length(keys) > 0) {
  preview <- bundle$read_band(keys[[1]])
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

### Licensing-aware startup

```r
library(whiteboxworkflows)

# Open mode
s_open <- wbw_session()

# Signed entitlement mode
s_ent <- wbw_session(
  signed_entitlement_json = signed_entitlement_json,
  public_key_kid = "k1",
  public_key_b64url = "REPLACE_WITH_PROVIDER_PUBLIC_KEY",
  include_pro = TRUE,
  tier = "open"
)

# Floating mode
s_float <- wbw_session(
  floating_license_id = "fl_12345",
  include_pro = TRUE,
  tier = "open",
  provider_url = "https://license.example.com",
  machine_id = "machine-01",
  customer_id = "customer-abc"
)
```

## Progress and feedback model

`wbw_run_tool_with_progress(...)` returns a structured progress payload and accepts an optional `on_progress` callback invoked once per progress event.

```r
library(whiteboxworkflows)

s <- wbw_session()
progress_result <- wbw_run_tool_with_progress(
  "slope",
  args = list(dem = "dem.tif", output = "slope.tif"),
  session = s,
  on_progress = function(pct, message) {
    cat(sprintf("[%3g%%] %s\n", pct, message))
  }
)

str(progress_result$progress)
```

For no-argument tools, `args = list()` is supported and encoded as an empty JSON object.

## R interoperability strategy

- Matrix/array-style raster exchange: use `terra` bridge helpers.
- Native multidimensional raster objects: use `stars` bridge helpers.
- Keep Whitebox as the geoprocessing engine and exchange data at stable boundaries.

## terra interoperability

```r
library(whiteboxworkflows)

arr <- wbw_raster_to_array("dem.tif")
arr2 <- arr + 1
wbw_array_to_raster(arr2, "dem_plus1.tif", template_path = "dem.tif", overwrite = TRUE)
```

## stars interoperability

```r
library(whiteboxworkflows)

s <- wbw_raster_to_stars("dem.tif")
s2 <- s + 1
wbw_stars_to_raster(s2, "dem_plus1_stars.tif", overwrite = TRUE)
```

## Licensing overview

The R runtime supports open and licensed modes.

- Open mode: `wbw_session()`.
- Signed entitlement mode: `wbw_session(signed_entitlement_json=..., ...)` or `entitlement_file=...`.
- Floating license mode: `wbw_session(floating_license_id=..., provider_url=..., ...)`.

See:
- [examples/licensing_offline.R](examples/licensing_offline.R)
- [examples/licensing_floating_online.R](examples/licensing_floating_online.R)

## Licensing and Pro workflows

### 1) Choose a startup mode

- Open mode for open-tier workflows and development.
- Signed entitlement mode for offline or pre-issued access.
- Floating mode for online provider activation and renewal.

### 2) Keep initialization centralized

Use one startup function in your app and pass the session through downstream code.

```r
new_wbw_session <- function(mode = c("open", "entitlement", "floating")) {
  mode <- match.arg(mode)
  if (mode == "open") return(wbw_session())
  if (mode == "entitlement") {
    return(wbw_session(
      signed_entitlement_json = signed_entitlement_json,
      public_key_kid = "k1",
      public_key_b64url = "REPLACE_WITH_PROVIDER_PUBLIC_KEY",
      include_pro = TRUE,
      tier = "open"
    ))
  }
  wbw_session(
    floating_license_id = "fl_12345",
    include_pro = TRUE,
    tier = "open",
    provider_url = "https://license.example.com"
  )
}
```

### 3) Check Pro visibility early

```r
required <- c("sar_coregistration", "refined_lee_filter")
available <- wbw_tool_ids(session = s)
missing <- setdiff(required, available)
if (length(missing) > 0) {
  stop(sprintf("Missing required Pro tools: %s", paste(missing, collapse = ", ")))
}
```

### 4) Keep open fallback explicit

For mixed deployments, keep explicit open-mode fallback branches so behavior is
predictable when Pro tools are unavailable.

## Discovery APIs

```r
s <- wbw_session()
matches <- wbw_search_tools("lidar")
tool <- wbw_describe_tool("slope")
```

Bundle family convenience readers are also available:

```r
landsat <- wbw_read_landsat("LC09_SCENE")
sentinel1 <- wbw_read_sentinel1("S1_SCENE.SAFE")
sentinel2 <- wbw_read_sentinel2("S2A_SCENE.SAFE")
```

```r
s <- wbw_session()
tools <- wbw_list_tools(session = s)
ids <- wbw_tool_ids(session = s)
has_slope <- wbw_has_tool("slope", session = s)
```

## Generated wrappers and package scaffold

`wbw_r` includes both a generated-wrapper workflow and a package scaffold.

Generate wrappers:

```bash
cargo run -p wbw_r --example generate_r_wrappers -- --tier open --output crates/wbw_r/generated/wbw_tools_generated.R
```

Package scaffold path:
- [r-package/whiteboxworkflows](r-package/whiteboxworkflows)

Useful development sync script:

```bash
bash crates/wbw_r/scripts/dev_r_package_sync.sh
```

## Testing

Rust tests:

```bash
cargo test -p wbw_r
```

R package tests:

```bash
Rscript -e 'testthat::test_local("crates/wbw_r/r-package/whiteboxworkflows")'
```

Optional real-fixture test roots:
- `WBW_TEST_DATA_ROOT` for non-SAR real fixtures.
- `WBW_SAR_FIXTURE_ROOT` for SAR fixtures.

Test coverage includes runtime/listing smoke checks, session facade behavior,
progress-helper dispatch behavior, and wrapper parity gates.
