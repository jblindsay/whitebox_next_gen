# Script Index

Use this index to map manual chapters to runnable examples.

Treat this chapter as the transition from concept to implementation. When
building a new workflow, start from the closest example script and adapt it with
your paths and parameters rather than composing from isolated snippets. This
usually produces safer and more maintainable scripts.

## Core Session and Discovery

- `crates/wbw_r/examples/generated_session_example.R` - baseline session creation and discovery flow
- `crates/wbw_r/r-package/whiteboxworkflows/inst/examples/generated_session_example.R` - package-scoped session bootstrap pattern
- `crates/wbw_r/r-package/whiteboxworkflows/inst/examples/golden_path_workflows.R` - representative end-to-end workflow chain

## Raster Workflows

- `crates/wbw_r/r-package/whiteboxworkflows/inst/examples/raster_object_quickstart.R` - object lifecycle and metadata checks
- `crates/wbw_r/r-package/whiteboxworkflows/inst/examples/raster_array_roundtrip.R` - array conversion and persistence roundtrip

## Vector Workflows

- `crates/wbw_r/r-package/whiteboxworkflows/inst/examples/vector_object_quickstart.R` - schema-first vector workflow baseline

## Lidar Workflows

- `crates/wbw_r/r-package/whiteboxworkflows/inst/examples/lidar_object_quickstart.R` - lidar object lifecycle and metadata checks
- `crates/wbw_r/r-package/whiteboxworkflows/inst/examples/lidar_chunked_matrix_streaming.R` - chunked matrix processing at scale
- `crates/wbw_r/examples/lidar_write_options.R` - output option tuning for lidar formats

## Sensor Bundle Workflows

- `crates/wbw_r/r-package/whiteboxworkflows/inst/examples/sensor_bundle_quickstart.R` - bundle intake and key inspection baseline
- `crates/wbw_r/r-package/whiteboxworkflows/inst/examples/sensor_bundle_multi_family_preview.R` - cross-family preview pattern

## Licensing

- `crates/wbw_r/examples/licensing_offline.R` - local entitlement startup example
- `crates/wbw_r/examples/licensing_floating_online.R` - floating license provider startup example
