# wbhdf

`wbhdf` is a scoped pure-Rust HDF container reader for the Whitebox project. It focuses on targeted, validated HDF dataset access patterns used by Whitebox workflows, with HDF5-first implementation and bounded HDF4/HDF-EOS2 support.

## Table of Contents

- [Mission](#mission)
- [The Whitebox Project](#the-whitebox-project)
- [Is wbhdf Only for Whitebox?](#is-wbhdf-only-for-whitebox)
- [What wbhdf Is Not](#what-wbhdf-is-not)
- [Features](#features)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [API Overview](#api-overview)
- [Architecture](#architecture)
- [Compilation Features](#compilation-features)
- [Known Limitations](#known-limitations)
- [Relationship to Other Whitebox Crates](#relationship-to-other-whitebox-crates)
- [License](#license)

## Mission

- Provide robust HDF container access for Whitebox workflows that need direct reads from known product families.
- Keep all HDF decode logic in Rust with no native `libhdf5` linkage.
- Prioritize bounded parsing, explicit diagnostics, and reproducible validation over broad speculative format coverage.

## The Whitebox Project

[Whitebox](https://www.whiteboxgeo.com) is a suite of open-source geospatial data analysis software with roots at the [University of Guelph](https://geg.uoguelph.ca), Canada, where [Dr. John Lindsay](https://jblindsay.github.io/ghrg/index.html) began the project in 2009. Over more than fifteen years it has grown into a widely used platform for geomorphometry, spatial hydrology, LiDAR processing, and remote sensing research. In 2021 Dr. Lindsay and Anthony Francioni founded [Whitebox Geospatial Inc.](https://www.whiteboxgeo.com) to ensure the project's long-term, sustainable development. **Whitebox Next Gen** is the current major iteration of that work, and this crate is part of that larger effort.

Whitebox Next Gen is a ground-up redesign that improves on its predecessor in nearly every dimension:

- **CRS & reprojection** — Full read/write of coordinate reference system metadata across raster, vector, and LiDAR data, with multiple resampling methods for raster reprojection.
- **Raster I/O** — More robust GeoTIFF handling (including Cloud-Optimized GeoTIFFs), plus newly supported formats such as GeoPackage Raster and JPEG2000.
- **Vector I/O** — Expanded from Esri Shapefile-only to 11 formats, including GeoPackage, FlatGeobuf, GeoParquet, and other modern interchange formats.
- **Vector topology** — A new, dedicated topology engine (`wbtopology`) enabling robust overlay, buffering, and related operations.
- **LiDAR I/O** — Full support for LAS 1.0–1.5, LAZ, COPC, E57, and PLY via `wblidar`, a high-performance, modern LiDAR I/O engine.
- **Frontends** — Whitebox Workflows for Python (WbW-Python), Whitebox Workflows for R (WbW-R), and a QGIS 4-compliant plugin are in active development.

## Is wbhdf Only for Whitebox?

No. `wbhdf` is developed primarily to support Whitebox, but it is not restricted to Whitebox projects.

- **Whitebox-first**: API and roadmap decisions prioritize Whitebox ingestion needs.
- **General-purpose**: the crate is usable as a standalone HDF reader for targeted, known-layout workloads.
- **Interop-focused**: explicit diagnostics and bounded decode paths make it suitable for controlled production pipelines.

## What wbhdf Is Not

`wbhdf` is a targeted HDF reader. It is **not** a full general-purpose HDF framework.

- Not a complete standards-level HDF4 or HDF5 implementation.
- Not a native-wrapper for `libhdf5`/`libhdf4`.
- Not a broad plugin system for every filter or layout variant.
- Not a direct replacement for higher-level raster or LiDAR analysis tooling.

## Features

- HDF5 signature/superblock probing and object-header traversal utilities.
- Dataset path resolution and bounded contiguous window readers (`f32`, `f64`).
- Bounded chunk-index traversal and chunk-payload decoding helpers for selected scalar paths.
- HDF4/HDF-EOS2 metadata probing and targeted SDS decode helpers (bounded windows).
- Dataset-scoped metadata text search/report helpers for fixture-backed validation.
- Reference comparison helpers for `f32`/`f64` exact and tolerance-based validation.
- Structured error taxonomy for unsupported layouts, datatype mismatches, invalid chunks, and filter failures.

## Installation

Crates.io dependency:

```toml
[dependencies]
wbhdf = "0.1"
```

Local workspace/path dependency:

```toml
[dependencies]
wbhdf = { path = "../wbhdf" }
```

## Quick Start

Read a bounded contiguous `f32` window when a known byte offset is available:

```rust,no_run
use std::path::Path;
use wbhdf::dataset::{read_contiguous_f32_window_in_file, resolve_dataset_in_file};
use wbhdf::datatypes::Endianness;

let file_path = Path::new("/data/sample.h5");
let _descriptor = resolve_dataset_in_file(file_path, "/BEAM0000/elev_lowestmode")?;
let values = read_contiguous_f32_window_in_file(file_path, 1_012_683, 4, Endianness::Little)?;
assert_eq!(values.len(), 4);
# Ok::<(), wbhdf::WbhdfError>(())
```

Read a bounded HDF4 SDS `i16` window from a canonical dataset path:

```rust,no_run
use std::path::Path;
use wbhdf::hdf4::decode_hdf4_sds_i16_window_at_in_file;

let file_path = Path::new("/data/MOD09A1.example.hdf");
let values = decode_hdf4_sds_i16_window_at_in_file(
		file_path,
		"/mod09a1_sur_refl/Data Fields/sur_refl_b01",
		0,
		256,
)?;
assert!(!values.is_empty());
# Ok::<(), wbhdf::WbhdfError>(())
```

## API Overview

- `dataset`:
	- `resolve_dataset_in_file(...)`
	- `read_contiguous_f32_window_in_file(...)`
	- `read_contiguous_f64_window_in_file(...)`
	- bounded chunked decode helpers for selected scalar paths
- `hdf4`:
	- `probe_hdf4_eos_metadata_in_file(...)`
	- `resolve_hdf4_dataset_path(...)`
	- `decode_hdf4_sds_i16_window_at_in_file(...)`
- `attributes`:
	- `dataset_metadata_contains_text_in_file(...)`
	- `dataset_metadata_text_report_in_file(...)`
- `compare`:
	- `compare_f32_exact(...)`, `compare_f32_with_tolerance(...)`
	- `compare_f64_exact(...)`, `compare_f64_with_tolerance(...)`
- `WbhdfError` / `WbhdfResult`:
	- explicit diagnostics for unsupported layouts, datatype mismatches, chunk/filter failures, and invalid inputs.

## Architecture

```text
wbhdf/
├── Cargo.toml
├── README.md
├── docs/
│   ├── DESIGN.md
│   ├── FORMAT_NOTES.md
│   └── SUPPORTED_HDF_PRODUCT_LAYOUTS.md
├── src/
│   ├── lib.rs          <- public API exports
│   ├── error.rs        <- WbhdfError / WbhdfResult
│   ├── superblock.rs   <- HDF5 signature and superblock probing
│   ├── object_header.rs<- object header parsing helpers
│   ├── btree.rs        <- bounded chunk-index traversal helpers
│   ├── dataset.rs      <- dataset path and window/chunked decode helpers
│   ├── datatypes.rs    <- typed endianness-aware scalar decode
│   ├── filters.rs      <- gzip/zlib filter helpers
│   ├── hdf4.rs         <- bounded HDF4/HDF-EOS2 probing/decode helpers
│   ├── attributes.rs   <- metadata text/report helpers
│   ├── compare.rs      <- numeric comparison helpers
│   └── fixtures.rs     <- fixture path/env resolution helpers
└── tests/
		└── integration_tests.rs
```

## Compilation Features

`wbhdf` currently ships without optional Cargo feature flags.

## Known Limitations

- Coverage is intentionally scoped to validated product/layout paths.
- Unsupported layouts fail fast with explicit diagnostics rather than best-effort decode.
- Not all HDF filters and datatype families are currently implemented.
- Integration/default-enable decisions are governed by the internal rollout gate and smoke/regression evidence.

## Relationship to Other Whitebox Crates

- `wbraster` uses `wbhdf` for targeted HDF dataset URI materialization on currently supported paths.
- `wblidar` uses `wbhdf` adapters for direct ingestion of validated HDF-backed product paths.
- `wbprojection` remains independent; CRS/reprojection behavior is handled in projection and raster/vector layers, not in `wbhdf` itself.

## License

Licensed under either:

- Apache License, Version 2.0
- MIT license

at your option.
