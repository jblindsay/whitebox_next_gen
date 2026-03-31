# wbgeotiff

`wbgeotiff` is the core GeoTIFF engine for the Whitebox project. It provides fast, pure-Rust read/write support for TIFF, GeoTIFF, BigTIFF, and Cloud Optimized GeoTIFF (COG) so Whitebox crates and other Rust geospatial applications can reliably ingest and emit georeferenced raster data.

## Table of Contents

- [Mission](#mission)
- [The Whitebox Project](#the-whitebox-project)
- [Is wbgeotiff Only for Whitebox?](#is-wbgeotiff-only-for-whitebox)
- [What wbgeotiff Is Not](#what-wbgeotiff-is-not)
- [Supported Formats](#supported-formats)
- [Compression Codecs](#compression-codecs)
- [Design Goals](#design-goals)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Examples](#examples)
- [API Overview](#api-overview)
- [Architecture](#architecture)
- [Performance Notes](#performance-notes)
- [Known Limitations](#known-limitations)
- [Relationship to Other Whitebox Crates](#relationship-to-other-whitebox-crates)
- [License](#license)

## Mission

- Provide robust GeoTIFF / COG I/O for Whitebox crates and applications.
- Keep all TIFF encoding/decoding logic in Rust with no GDAL or libtiff dependency.
- Prioritize standards compliance, interoperability, and a strongly typed API that higher layers can wrap ergonomically.

## The Whitebox Project

Whitebox is a collection of related open-source geospatial data analysis software. The Whitebox project began in 2009 at the [University of Guelph](https://geg.uoguelph.ca), Canada, developed by [Dr. John Lindsay](https://jblindsay.github.io/ghrg/index.html) a professor of geomatics. Whitebox has long served as Dr. Lindsay's platform for disseminating the output of his geomatics-based research and has developed an extensive worldwide user base. In 2021 Dr. Lindsay and Anthony Francioni founded [Whitebox Geospatial Inc.](https://www.whiteboxgeo.com) in order to ensure the sustainable and ongoing development of this open-source geospatial project. We are currently working on the next iteration of the Whitebox software, **Whitebox Next Gen**. This crate is part of that larger effort.

## Is wbgeotiff Only for Whitebox?

No. `wbgeotiff` is developed primarily to support Whitebox, but it is not restricted to Whitebox projects.

- **Whitebox-first**: API and roadmap decisions prioritize Whitebox I/O needs.
- **General-purpose**: the crate is usable as a standalone GeoTIFF engine in other Rust geospatial applications.
- **Interop-focused**: standards-compliant GeoTIFF / BigTIFF / COG output makes it suitable for broader tooling and data pipelines.

## What wbgeotiff Is Not

`wbgeotiff` is a low-level TIFF/GeoTIFF I/O engine. It is **not** a full raster abstraction layer.

- Not a multi-format raster library (for ENVI, SAGA, PCRaster, Zarr, etc. see `wbraster`).
- Not a raster analysis or processing library (filtering, statistics, reprojection belong in higher-level Whitebox tooling).
- Not a rendering or visualization engine.
- Not a GeoTIFF metadata editing tool (IFD-level tag surgery is out of scope).

## Supported Formats

| Format | Read | Write | Notes |
|--------|:----:|:-----:|-------|
| **GeoTIFF** | yes | yes | Classic TIFF with GeoKey metadata |
| **BigTIFF** | yes | yes | 64-bit offset TIFF for files &gt; 4 GiB |
| **Cloud Optimized GeoTIFF (COG)** | yes | yes | COG layout with overview pyramid |
| **Stripped TIFF** | yes | yes | Row-oriented storage |
| **Tiled TIFF** | yes | yes | Block-oriented storage for random access |

## Compression Codecs

All codecs are built in. No optional feature flag is required.

| Codec | TIFF Tag | Read | Write | Notes |
|-------|:--------:|:----:|:-----:|-------|
| None | 1 | yes | yes | Uncompressed baseline |
| PackBits | 32773 | yes | yes | Simple run-length encoding |
| LZW | 5 | yes | yes | TIFF classic LZW |
| Deflate (ZIP) | 8 / 32946 | yes | yes | zlib/DEFLATE; recommended for scientific rasters |
| JPEG | 6 / 7 | yes | yes | Lossy; suitable for RGB imagery |
| WebP | 50001 | yes | yes | Modern lossy/lossless for imagery |
| JPEG XL | 50002 | yes | yes | Next-generation codec |

## Design Goals

- **No GDAL dependency**: pure Rust, no native libtiff or GDAL runtime required.
- **Low-level, strongly typed API**: sample formats, compression, and georeferencing are explicit and type-safe so higher layers can expose ergonomic wrappers without leaking implementation details.
- **COG-native**: Cloud Optimized GeoTIFF is a first-class write path with overview pyramid support.
- **BigTIFF capable**: handle rasters larger than 4 GiB transparently.
- **Minimal dependencies**: keep dependency surface tight and auditable.
- **Whitebox integration**: maintain a stable API for Whitebox crate consumption.

## Features

| Feature | API |
|---|---|
| Read GeoTIFF/BigTIFF | `GeoTiff` |
| Write stripped/tiled GeoTIFF | `GeoTiffWriter` + `WriteLayout` |
| Write COG | `CogWriter` |
| Compression selection | `Compression` |
| GeoTransform + EPSG/GeoKeys | `GeoTransform`, `GeoKeyDirectory` |
| Typed sample formats | `SampleFormat` |

## Installation

Workspace/path dependency:

```toml
[dependencies]
wbgeotiff = { path = "../wbgeotiff" }
```

## Quick Start

### Read a GeoTIFF

```rust
use wbgeotiff::GeoTiff;

let tiff = GeoTiff::open("dem.tif")?;
println!("{}x{} bands={} bigtiff={}", tiff.width(), tiff.height(), tiff.band_count(), tiff.is_bigtiff);

let band0: Vec<f32> = tiff.read_band_f32(0)?;
println!("first sample: {}", band0[0]);
# Ok::<(), wbgeotiff::GeoTiffError>(())
```

### Write a tiled GeoTIFF

```rust
use wbgeotiff::{Compression, GeoTiffWriter, GeoTransform, SampleFormat, WriteLayout};

let width = 1024u32;
let height = 1024u32;
let data = vec![0.0f32; (width * height) as usize];

GeoTiffWriter::new(width, height, 1)
		.layout(WriteLayout::Tiled { tile_width: 256, tile_height: 256 })
		.compression(Compression::Deflate)
		.sample_format(SampleFormat::IeeeFloat)
		.geo_transform(GeoTransform::north_up(500_000.0, 10.0, 4_500_000.0, -10.0))
		.epsg(32632)
		.write_f32("out.tif", &data)?;
# Ok::<(), wbgeotiff::GeoTiffError>(())
```

### Write a COG

```rust
use wbgeotiff::{CogWriter, Compression, GeoTransform, Resampling};

let width = 4096u32;
let height = 4096u32;
let data = vec![1.0f32; (width * height) as usize];

CogWriter::new(width, height, 1)
		.compression(Compression::Deflate)
		.tile_size(512)
		.resampling(Resampling::Average)
		.geo_transform(GeoTransform::north_up(-180.0, 0.087890625, 90.0, -0.087890625))
		.epsg(4326)
		.write_f32("out.cog.tif", &data)?;
# Ok::<(), wbgeotiff::GeoTiffError>(())
```

## Examples

The crate includes runnable examples under `examples/`:

- `read_geotiff.rs` - open a raster, print metadata, and read band 0 as `f32`.
- `write_tiled_geotiff.rs` - create a tiled GeoTIFF with Deflate compression.
- `write_cog.rs` - create a Cloud Optimized GeoTIFF with overviews.
- `write_read_u16.rs` - write a `u16` tiled GeoTIFF and read it back.

Run them with Cargo:

```bash
cargo run -p wbgeotiff --example read_geotiff -- path/to/input.tif
cargo run -p wbgeotiff --example write_tiled_geotiff -- path/to/output.tif
cargo run -p wbgeotiff --example write_cog -- path/to/output.cog.tif
cargo run -p wbgeotiff --example write_read_u16 -- path/to/output_u16.tif
```

## API Overview

- `GeoTiff`: open and inspect existing TIFF/GeoTIFF/BigTIFF datasets.
- `GeoTiffWriter`: create classic GeoTIFF or BigTIFF outputs.
- `CogWriter`: create Cloud Optimized GeoTIFF outputs with overviews.
- `Compression`: codec enum (`None`, `Lzw`, `Deflate`, `PackBits`, `Jpeg`, `WebP`, `JpegXl`).
- `WriteLayout`: stripped vs tiled layout for standard GeoTIFF writes.
- `GeoTransform`: affine georeferencing helpers (for north-up and general transforms).
- `GeoKeyDirectory`: lower-level GeoKey control when you need explicit keys.

## Architecture

```
wbgeotiff/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs          ← public API exports
│   ├── reader.rs       ← GeoTiff IFD parser, band reader, typed decode paths
│   ├── writer.rs       ← GeoTiffWriter, standard stripped/tiled write
│   ├── cog_writer.rs   ← CogWriter, COG layout + overview pyramid
│   ├── codec/          ← per-codec encode/decode implementations
│   ├── geotransform.rs ← GeoTransform affine helpers
│   ├── georef.rs       ← GeoKeyDirectory, EPSG binding, GeoKeys
│   ├── types.rs        ← Compression, WriteLayout, SampleFormat, Resampling
│   └── error.rs        ← GeoTiffError, Result
├── examples/           ← four runnable examples
```

### Design principles

- **IFD-based reader**: full IFD chain traversal; multi-band and multi-page TIFFs are supported via the band index.
- **Lazy decode**: samples are decoded on demand per `read_band_*` call, not on open.
- **COG layout**: `CogWriter` writes overviews before the main IFD to conform to the COG spec.
- **Strongly typed writes**: `write_f32`, `write_u16`, etc. encode samples with the correct `SampleFormat` and `BitsPerSample` TIFF tags automatically.
- Pure Rust implementation, no GDAL runtime dependency.
- Low-level, strongly typed API so higher layers can expose ergonomic wrappers.

## Performance Notes

- `wbgeotiff` uses buffered I/O (`BufReader`/`BufWriter`) to minimize system call overhead.
- Tiled read/write has lower per-band overhead for large rasters compared to stripped I/O because tiles decode independently.
- COG writes complete the full overview pyramid in a single pass; no separate tool step is required.
- Deflate compression is recommended for scientific rasters (good compression ratio with fast decompression).
- For very large datasets (several GiB+), enable BigTIFF with `.bigtiff(true)` on `GeoTiffWriter`.

## Known Limitations

- `wbgeotiff` is a low-level TIFF engine; higher-level multi-format raster workflows should use `wbraster`.
- JPEG and WebP compression are lossy for floating-point sample data; prefer Deflate or LZW for scientific DEMs and grids.
- Multi-band writes store all bands in a single IFD; separate-file multi-band workflows are not supported.
- Reading an entire large tiled TIFF band into memory requires sufficient contiguous RAM; tile-by-tile access is recommended for out-of-core workflows.
- COG HTTP range fetching is not in scope; for COPC/LAZ HTTP range reads see `wblidar`.
- Some vendor-specific private TIFF tag extensions are preserved as raw IFD entries but not interpreted.
- Some draft or proprietary TIFF compression variants (e.g. LERC, tag 34887) are not yet implemented.

## Relationship to Other Whitebox Crates

- `wbraster` uses `wbgeotiff` for GeoTIFF/COG format support and adds higher-level
	raster abstractions and multi-format IO.
- `wbprojection` can depend on the same shared GeoTIFF engine for projection-related
	metadata workflows without creating circular dependencies.

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
