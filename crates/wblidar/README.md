# wblidar

`wblidar` is the core LiDAR I/O engine for the Whitebox project. It provides fast, standards-focused, pure-Rust read/write support for common point-cloud formats so Whitebox tools can reliably ingest and emit LiDAR data.

## Table of Contents

- [Mission](#mission)
- [The Whitebox Project](#the-whitebox-project)
- [Is wblidar Only for Whitebox?](#is-wblidar-only-for-whitebox)
- [What wblidar Is Not](#what-wblidar-is-not)
- [Supported Formats](#supported-formats)
- [Design Goals](#design-goals)
- [Installation](#installation)
- [Compilation Features](#compilation-features)
- [API Overview](#api-overview)
- [Architecture](#architecture)
- [Performance Notes](#performance-notes)
- [Known Limitations](#known-limitations)
- [Validation and Interoperability](#validation-and-interoperability)
- [Suggested Additional Sections (Optional)](#suggested-additional-sections-optional)
- [License](#license)

## Mission

- Provide robust LiDAR format I/O for Whitebox applications and workflows.
- Keep codec logic in Rust with minimal external dependencies.
- Prioritize standards compliance, interoperability, and predictable behavior.

## The Whitebox Project

[Whitebox](https://www.whiteboxgeo.com) is a suite of open-source geospatial data analysis software with roots at the [University of Guelph](https://geg.uoguelph.ca), Canada, where [Dr. John Lindsay](https://jblindsay.github.io/ghrg/index.html) began the project in 2009. Over more than fifteen years it has grown into a widely used platform for geomorphometry, spatial hydrology, LiDAR processing, and remote sensing research. In 2021 Dr. Lindsay and Anthony Francioni founded [Whitebox Geospatial Inc.](https://www.whiteboxgeo.com) to ensure the project's long-term, sustainable development. **Whitebox Next Gen** is the current major iteration of that work, and this crate is part of that larger effort.

Whitebox Next Gen is a ground-up redesign that improves on its predecessor in nearly every dimension:

- **CRS & reprojection** — Full read/write of coordinate reference system metadata across raster, vector, and LiDAR data, with multiple resampling methods for raster reprojection.
- **Raster I/O** — More robust GeoTIFF handling (including Cloud-Optimized GeoTIFFs), plus newly supported formats such as GeoPackage Raster and JPEG2000.
- **Vector I/O** — Expanded from Esri Shapefile-only to 11 formats, including GeoPackage, FlatGeobuf, GeoParquet, and other modern interchange formats.
- **Vector topology** — A new, dedicated topology engine (`wbtopology`) enabling robust overlay, buffering, and related operations.
- **LiDAR I/O** — Full support for LAS 1.0–1.5, LAZ, COPC, E57, and PLY via `wblidar`, a high-performance, modern LiDAR I/O engine.
- **Frontends** — Whitebox Workflows for Python (WbW-Python), Whitebox Workflows for R (WbW-R), and a QGIS 4-compliant plugin are in active development.

## Is wblidar Only for Whitebox?

No. `wblidar` is developed primarily to support Whitebox, but it is not restricted to Whitebox projects.

- **Whitebox-first**: API and roadmap decisions prioritize Whitebox I/O needs.
- **General-purpose**: the crate is usable as a standalone LiDAR I/O engine in other Rust applications.
- **Interop-focused**: standards-compliant LAS/LAZ/COPC/PLY/E57 support makes it suitable for broader tooling and data pipelines.

## What wblidar Is Not

`wblidar` is an I/O and format layer. It is **not** intended to be a full LiDAR processing framework.

- Not a filtering/classification framework.
- Not a replacement for Whitebox analysis/processing tools.
- Not a pipeline engine for arbitrary geospatial transformations.

Point-cloud processing, filtering, segmentation, and analysis belong in the Whitebox frontend/tooling layer.

## Supported Formats

| Format | Read | Write | Notes |
|--------|:----:|:-----:|-------|
| **LAS** | yes | yes | LAS 1.1-1.5, PDRF 0-15 |
| **LAZ** | yes | yes | Standards-compliant LASzip v2/v3 Point10/Point14 codecs |
| **COPC** | yes | yes | COPC 1.0 hierarchy with Point14-family payloads |
| **PLY** | yes | yes | ASCII, binary little-endian, binary big-endian |
| **E57** | yes | yes | ASTM E2807 with CRC-32 page validation |

## Design Goals

- **Standards first**: prefer interoperable, standards-compliant encoding/decoding paths.
- **Pure Rust codecs**: avoid native/C++ LASzip dependency by implementing core codecs in Rust.
- **Streaming I/O APIs**: expose incremental read/write interfaces for large files.
- **Minimal dependencies**: keep dependency surface tight and auditable.
- **Whitebox integration**: maintain a stable API for Whitebox ingestion/export workflows.
- **Predictable behavior**: deterministic output where applicable and explicit error modes.

## Installation

Crates.io dependency:

```toml
[dependencies]
wblidar = "0.1"
```

Enable optional features only when needed:

```toml
[dependencies]
wblidar = { version = "0.1", features = ["copc-http", "parallel"] }
```

Local workspace/path dependency:

```toml
[dependencies]
wblidar = { path = "../wblidar" }
```

Feature notes:

- `copc-http` enables HTTP range fetching for remote COPC access.
- `copc-parallel` enables Rayon-backed parallel work in COPC writing paths.
- `laz-parallel` enables optional parallel LAZ chunk decoding.
- `parallel` enables both `copc-parallel` and `laz-parallel`.

## Compilation Features

`wblidar` uses optional Cargo features for specific capabilities.

| Feature | Default | Purpose |
|---------|:-------:|---------|
| `copc-http` | no | Enables HTTP range fetching support for COPC access (`reqwest`). |
| `parallel` | no | Convenience umbrella feature enabling all current parallel paths. |
| `copc-parallel` | no | Enables Rayon-based parallel work in COPC writing paths (node encoding/sorting thresholds). |
| `laz-parallel` | no | Enables optional parallel LAZ chunk decode paths where beneficial. |

Example:

```bash
cargo build -p wblidar --features "parallel"
```

Use `copc-parallel` or `laz-parallel` individually when you want narrower benchmarking or regression isolation.

## API Overview

`wblidar` exposes two main usage styles:

- **Low-level streaming APIs** via format-specific readers/writers and `PointReader` / `PointWriter` traits.
- **Unified frontend API** via `PointCloud` for format-agnostic workflows.

### 1) Stream LAS -> LAS

This example shows minimal-memory, record-by-record conversion between LAS files using the streaming reader/writer traits.

```rust
use std::fs::File;
use std::io::{BufReader, BufWriter};

use wblidar::{
    io::{PointReader, PointWriter},
    las::{LasReader, LasWriter, WriterConfig},
    PointRecord,
};

fn main() -> wblidar::Result<()> {
    let input = BufReader::new(File::open("input.las")?);
    let mut reader = LasReader::new(input)?;

    let output = BufWriter::new(File::create("output.las")?);
    let mut writer = LasWriter::new(output, WriterConfig::default())?;

    let mut p = PointRecord::default();
    while reader.read_point(&mut p)? {
        writer.write_point(&p)?;
    }
    writer.finish()?;
    Ok(())
}
```

### 2) Format-Agnostic Read/Write

This example shows the high-level `PointCloud` API auto-detecting input format and writing multiple output formats.

```rust
use wblidar::{LidarFormat, PointCloud};

fn main() -> wblidar::Result<()> {
    let cloud = PointCloud::read("input.laz")?;

    cloud.write("out.copc.laz")?;
    cloud.write("out.ply")?;

    // Force output format regardless of extension.
    cloud.write_as("out.data", LidarFormat::E57)?;
    Ok(())
}
```

### 3) Read With Diagnostics

This example shows ingest diagnostics for observability, including partial Point14 recovery counters.

```rust
use wblidar::read_with_diagnostics;

fn main() -> wblidar::Result<()> {
    let (cloud, diag) = read_with_diagnostics("input.copc.laz")?;
    println!("points: {}", cloud.points.len());

    if diag.point14_partial_events > 0 {
        println!(
            "partial Point14 recovery: events={} decoded/expected={}/{}",
            diag.point14_partial_events,
            diag.point14_partial_decoded_points,
            diag.point14_partial_expected_points
        );
    }
    Ok(())
}
```

### 4) Reproject a PointCloud

This example shows a straightforward end-to-end reprojection workflow using `PointCloud` convenience methods.

```rust
use wblidar::PointCloud;

fn main() -> wblidar::Result<()> {
    let mut cloud = PointCloud::read("input.las")?;
    cloud.reproject_in_place_to_epsg(3857)?;
    cloud.write("output_3857.laz")?;
    Ok(())
}
```

### 5) Write COPC with Explicit Spatial Ordering

This example shows COPC writing with explicit root geometry and node point ordering configuration.

```rust
use std::fs::File;
use std::io::BufWriter;

use wblidar::{
    copc::{CopcNodePointOrdering, CopcWriter, CopcWriterConfig},
    io::PointWriter,
    PointRecord,
};

fn main() -> wblidar::Result<()> {
    let out = BufWriter::new(File::create("out.copc.laz")?);
    let cfg = CopcWriterConfig {
        center_x: 500000.0,
        center_y: 6000000.0,
        center_z: 100.0,
        halfsize: 500.0,
        spacing: 5.0,
        node_point_ordering: CopcNodePointOrdering::Auto,
        ..CopcWriterConfig::default()
    };

    let mut writer = CopcWriter::new(out, cfg);
    writer.write_point(&PointRecord::default())?;
    writer.finish()?;
    Ok(())
}
```

### 6) Optional Parallel LAZ Decode (Feature-Gated)

This example shows feature-gated parallel LAZ decode for high-volume workloads where chunk-level parallelism can improve throughput.

```rust
// Requires Cargo feature: parallel or laz-parallel
use std::fs::File;
use std::io::BufReader;

use wblidar::laz::reader::LazReader;

fn main() -> wblidar::Result<()> {
    let input = BufReader::new(File::open("input.laz")?);
    let mut reader = LazReader::new(input)?;

    #[cfg(any(feature = "parallel", feature = "laz-parallel"))]
    {
        let points = reader.read_all_points_parallel()?;
        println!("decoded points: {}", points.len());
    }

    Ok(())
}
```

## Architecture

At a high level:

1. **Common model**: `PointRecord` is the central in-memory point representation.
2. **Traits**: `PointReader` and `PointWriter` provide streaming semantics.
3. **Format modules**: `las`, `laz`, `copc`, `ply`, `e57` encapsulate format-specific details.
4. **Frontend**: `PointCloud` and helper functions provide a unified API for common workflows.

Format notes:

- **LAS**: direct structured read/write with VLR/CRS support.
- **LAZ**: in-house LASzip-compatible codecs for Point10/Point14 families.
- **COPC**: LAZ-backed octree hierarchy with COPC metadata/hierarchy pages.
- **PLY**: ASCII and binary interchange for general point cloud exchange.
- **E57**: standards-oriented reader/writer with integrity checks.

## Performance Notes

- `wblidar` uses SIMD in hot numeric paths where safe and beneficial.
- Optional parallelism is feature-gated and thresholded to avoid regressions on small jobs.
- Streaming APIs are the default path for low-memory workflows.
- Some decode/encode paths intentionally trade memory for correctness and interoperability.

### Point14 `compression_level` Behavior

`LazWriterConfig::compression_level` is now effective for Point14-family LAZ writes.
It tunes the **effective chunk target size** used during encoding:

- Lower levels favor smaller chunks (often faster writes, sometimes slightly larger files).
- Higher levels favor larger chunks (often slightly better compression, potentially more memory/latency per flush).

Current mapping (base `chunk_size` = configured `chunk_size`):

| Level | Effective chunk target |
|---:|---|
| 0 | `chunk_size / 2` |
| 1 | `2 * chunk_size / 3` |
| 2 | `3 * chunk_size / 4` |
| 3-6 | `chunk_size` |
| 7 | `5 * chunk_size / 4` |
| 8 | `3 * chunk_size / 2` |
| 9 | `2 * chunk_size` |

Notes:

- This behavior currently applies to Point14-family LAZ writes.
- Point10 paths continue to use the configured chunk size directly.
- COPC `compression_level` remains independent of this LAZ chunk-size tuning.

Useful environment knobs:

- `WBLIDAR_COPC_PARALLEL_MIN_NODES` (default: `16`, requires `parallel` or `copc-parallel`):
    Minimum number of COPC nodes required before parallel node encoding is considered.
    Effective threshold is `max(WBLIDAR_COPC_PARALLEL_MIN_NODES, 2 * rayon_thread_count)`.
    Increase to reduce thread overhead on smaller jobs; decrease to parallelize sooner.
- `WBLIDAR_COPC_PARALLEL_MIN_POINTS` (default: `400000`, requires `parallel` or `copc-parallel`):
    Minimum total points across candidate COPC nodes before parallel node encoding is used.
    Increase to keep more workloads serial; decrease to enable parallel encoding for smaller datasets.
- `WBLIDAR_COPC_PARALLEL_SORT_MIN_POINTS` (default: `80000`, requires `parallel` or `copc-parallel`):
    Minimum per-node point count before Morton/Hilbert code sorting switches to parallel sort.
    Increase to favor serial sort on medium nodes; decrease to parallelize sort earlier.
- `WBLIDAR_LAZ_PARALLEL_MIN_CHUNKS` (default: `4`, requires `parallel` or `laz-parallel`):
    Minimum non-empty LAZ chunks required before `read_all_points_parallel()` uses parallel decode.
    Increase to avoid parallel overhead on files with few chunks; decrease to parallelize earlier.
- `WBLIDAR_LAZ_PARALLEL_MIN_POINTS` (default: `200000`, requires `parallel` or `laz-parallel`):
    Minimum total points required before `read_all_points_parallel()` uses parallel decode.
    Increase to keep more files on serial fallback; decrease to use parallel decode more aggressively.

### Using the Environment Knobs

Set knobs inline for a single command:

```bash
WBLIDAR_COPC_PARALLEL_MIN_NODES=24 \
WBLIDAR_COPC_PARALLEL_MIN_POINTS=600000 \
WBLIDAR_COPC_PARALLEL_SORT_MIN_POINTS=120000 \
cargo run -p wblidar --features "copc-parallel" --example copc_parity_benchmark_csv -- input.las out_prefix
```

For normal builds, prefer `--features "parallel"`; keep `copc-parallel` for COPC-only benchmarking or regression isolation.

Set LAZ knobs for a single parallel-decode run:

```bash
WBLIDAR_LAZ_PARALLEL_MIN_CHUNKS=8 \
WBLIDAR_LAZ_PARALLEL_MIN_POINTS=500000 \
cargo run -p wblidar --features "laz-parallel" --example laz_parallel_parity_benchmark -- input.laz /tmp/laz_bench
```

For normal builds, prefer `--features "parallel"`; keep `laz-parallel` for LAZ-only benchmarking or regression isolation.

Export knobs for the current shell session:

```bash
export WBLIDAR_COPC_PARALLEL_MIN_NODES=16
export WBLIDAR_COPC_PARALLEL_MIN_POINTS=400000
export WBLIDAR_COPC_PARALLEL_SORT_MIN_POINTS=80000
export WBLIDAR_LAZ_PARALLEL_MIN_CHUNKS=4
export WBLIDAR_LAZ_PARALLEL_MIN_POINTS=200000
```

Quick starting presets:

| Preset | COPC Min Nodes | COPC Min Points | COPC Sort Min Points | LAZ Min Chunks | LAZ Min Points | When to Use |
|---|---:|---:|---:|---:|---:|---|
| Conservative | 32 | 1000000 | 160000 | 12 | 1000000 | Prioritize predictable serial behavior on mixed or smaller jobs |
| Balanced (default-like) | 16 | 400000 | 80000 | 4 | 200000 | Good first choice for most workloads |
| Aggressive | 8 | 150000 | 40000 | 2 | 100000 | Favor parallelism earlier on large multi-core systems |

Notes:

- Knobs are read once per process startup; restart your process to apply changed values.
- Knobs only affect feature-enabled code paths (`parallel`, `copc-parallel`, and `laz-parallel`).

## Known Limitations

- `wblidar` focuses on I/O and format correctness, not higher-level LiDAR processing algorithms.
- COPC payloads are Point14-family; some LAS 1.5-specific fields are promoted or omitted when mapping to COPC-compatible formats.
- Legacy wb-native LAZ DEFLATE paths are intentionally out of scope; standards LASzip-compatible paths are used.
- Some Point14-heavy paths can require substantial memory because layered decode/encode may materialize large in-memory buffers.
- COPC writing is batch-oriented; appending incremental updates to an existing COPC file is not currently supported.
- COPC node ordering is configurable (`Auto`, `Morton`, `Hilbert`) but not yet auto-tuned per dataset.
- Partial Point14 handling defaults to lenient recovery; strict failure mode is opt-in via `WBLIDAR_FAIL_ON_PARTIAL_POINT14`.
- LAZ parallel decode tuning knobs apply to `read_all_points_parallel()`; regular streaming `read_point()` remains serial.
- External interoperability validation is strong but still benefits from broader real-world fixture coverage across toolchains.
- Some advanced paths are feature-gated (`copc-http`, `parallel`, and the granular parallel modes) and are not enabled by default.
- Performance characteristics vary by file structure (for example, chunking strategy can limit parallel speedups on some LAZ datasets).

## Validation and Interoperability

Internal validation checklists and QA procedures are maintained in [`docs/internal/`](docs/internal/). These cover external interoperability workflows (PDAL, LAStools, validate.copc.io) and are intended for maintainers rather than library users.

## Suggested Additional Sections (Optional)

If you want to expand this README further, the highest-value additions would be:

- **Versioning/compatibility policy** (what constitutes a breaking API change).
- **Error-handling guide** (common `Error` variants and recovery guidance).
- **Benchmark methodology** (how performance claims are measured).
- **Contributing guide pointer** (coding conventions, tests, fixture requirements).

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
