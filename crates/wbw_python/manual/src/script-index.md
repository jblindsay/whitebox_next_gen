# Script Index

Use this index to map manual chapters to runnable examples.

Use this chapter as a bridge from concept to execution. When adapting examples,
start from the script closest to your data type and operational goal, then copy
its structure before changing parameters. This tends to produce safer edits than
assembling a workflow from isolated snippets.

## Quick Start and Core API

- `crates/wbw_python/examples/quickstart_harmonized_api.py` - minimal end-to-end startup and first tool run
- `crates/wbw_python/examples/wbenvironment_example.py` - environment configuration and discovery baseline
- `crates/wbw_python/examples/current_api_data_handling_demo.py` - data object handling patterns across core types

## Raster Workflows

- `crates/wbw_python/examples/raster_numpy_roundtrip.py` - single-band array conversion and writeback
- `crates/wbw_python/examples/raster_numpy_multiband_roundtrip.py` - multiband array access and persistence
- `crates/wbw_python/examples/unary_raster_tools_example.py` - tool-driven raster transform chain

## Vector Workflows

- `crates/wbw_python/examples/vector_attributes_harmonized_api.py` - schema inspection and attribute updates
- `crates/wbw_python/examples/vector_multifile_write_demo.py` - format-aware vector export behavior

## Lidar Workflows

- `crates/wbw_python/examples/lidar_write_options.py` - LAZ and COPC output option tuning
- `crates/wbw_python/examples/lidar_numpy_roundtrip_smoke_test.py` - point-field numpy roundtrip validation
- `crates/wbw_python/examples/lidar_chunked_pipeline.py` - chunked processing pattern for large clouds

## Sensor Bundle Workflows

- `crates/wbw_python/examples/sensor_bundle_overview.py` - cross-family inspection and key discovery

## Interoperability and Validation

- `crates/wbw_python/examples/interop_roundtrip_smoke_test.py` - cross-library roundtrip sanity checks

## Licensing

- `crates/wbw_python/examples/licensing_offline_example.py` - local entitlement startup pattern
- `crates/wbw_python/examples/licensing_floating_online_example.py` - floating-license provider bootstrap pattern

## Dynamic Output Smoke Tests

- `crates/wbw_python/examples/dynamic_single_output_smoke_test.py` - runtime single-output behavior checks
- `crates/wbw_python/examples/dynamic_multi_output_smoke_test.py` - runtime multi-output behavior checks
