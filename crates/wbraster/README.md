# wbraster

A Rust library for reading and writing common raster GIS formats intended to serve as the raster engine for [Whitebox](https://www.whiteboxgeo.com).

## Table of Contents

- [Mission](#mission)
- [The Whitebox Project](#the-whitebox-project)
- [Is wbraster Only for Whitebox?](#is-wbraster-only-for-whitebox)
- [What wbraster Is Not](#what-wbraster-is-not)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Examples](#examples)
- [Supported Formats](#supported-formats)
- [SAFE Bundle Support](#safe-bundle-support)
- [Landsat Collection Bundle Support](#landsat-collection-bundle-support)
- [ICEYE Bundle Support](#iceye-bundle-support)
- [PlanetScope Bundle Support](#planetscope-bundle-support)
- [SPOT/Pleiades DIMAP Bundle Support](#spotpleiades-dimap-bundle-support)
- [Maxar/WorldView Bundle Support](#maxarworldview-bundle-support)
- [RADARSAT-2 Bundle Support](#radarsat-2-bundle-support)
- [RCM Bundle Support](#rcm-bundle-support)
- [Unified Sensor Bundle Detection](#unified-sensor-bundle-detection)
- [Real-Sample Smoke Tests (Opt-In)](#real-sample-smoke-tests-opt-in)
- [Bundle Canonical Key Reference](#bundle-canonical-key-reference)
- [Coordinate Reference System (CRS)](#coordinate-reference-system-crs)
- [Common GeoTIFF / COG Workflows](#common-geotiff--cog-workflows)
- [Architecture](#architecture)
- [Data Type Support Per Format](#data-type-support-per-format)
- [GeoTIFF / COG Notes](#geotiff--cog-notes)
- [JPEG2000 / GeoJP2 Notes](#jpeg2000--geojp2-notes)
- [ENVI Metadata Key Reference](#envi-metadata-key-reference)
- [Zarr Notes](#zarr-notes)
- [Compilation Features](#compilation-features)
- [Performance](#performance)
- [Benchmarking](#benchmarking)
- [Known Limitations](#known-limitations)
- [License](#license)

## Mission

- Provide multi-format raster GIS I/O for Whitebox applications and workflows.
- Support the broadest practical range of raster formats in a single pure-Rust library.
- Maintain correct coordinate registration, CRS propagation, and data-type fidelity across all formats.
- Minimize external C/native dependencies by delegating to purpose-built pure-Rust codecs where possible.

## The Whitebox Project

[Whitebox](https://www.whiteboxgeo.com) is a suite of open-source geospatial data analysis software with roots at the [University of Guelph](https://geg.uoguelph.ca), Canada, where [Dr. John Lindsay](https://jblindsay.github.io/ghrg/index.html) began the project in 2009. Over more than fifteen years it has grown into a widely used platform for geomorphometry, spatial hydrology, LiDAR processing, and remote sensing research. In 2021 Dr. Lindsay and Anthony Francioni founded [Whitebox Geospatial Inc.](https://www.whiteboxgeo.com) to ensure the project's long-term, sustainable development. **Whitebox Next Gen** is the current major iteration of that work, and this crate is part of that larger effort.

Whitebox Next Gen is a ground-up redesign that improves on its predecessor in nearly every dimension:

- **CRS & reprojection** — Full read/write of coordinate reference system metadata across raster, vector, and LiDAR data, with multiple resampling methods for raster reprojection.
- **Raster I/O** — More robust GeoTIFF handling (including Cloud-Optimized GeoTIFFs), plus newly supported formats such as GeoPackage Raster and JPEG2000.
- **Vector I/O** — Expanded from Esri Shapefile-only to 11 formats, including GeoPackage, FlatGeobuf, GeoParquet, and other modern interchange formats.
- **Vector topology** — A new, dedicated topology engine (`wbtopology`) enabling robust overlay, buffering, and related operations.
- **LiDAR I/O** — Full support for LAS 1.0–1.5, LAZ, COPC, E57, and PLY via `wblidar`, a high-performance, modern LiDAR I/O engine.
- **Frontends** — Whitebox Workflows for Python (WbW-Python), Whitebox Workflows for R (WbW-R), and a QGIS 4-compliant plugin are in active development.

## Is wbraster Only for Whitebox?

No. `wbraster` is developed primarily to support Whitebox, but it is not restricted to Whitebox projects.

- **Whitebox-first**: API and roadmap decisions prioritize Whitebox raster I/O needs.
- **General-purpose**: the crate is usable as a standalone multi-format raster library in other Rust geospatial applications.
- **Format-complete**: 13 supported formats with full round-trip read/write, typed band access, and CRS propagation make it broadly useful.

## What wbraster Is Not

`wbraster` is a format I/O and CRS layer. It is **not** a full raster analysis framework.

- Not a raster processing or analysis library (filtering, terrain analysis, histogram operations belong in Whitebox tooling).
- Not a rendering or visualization engine.
- Not a remote sensing pipeline (radiometric correction, band math, and similar operations belong in the tooling layer).
- Not a distributed or chunked processing framework (Zarr MVP support is focused on local filesystem stores).

## Installation

Crates.io dependency:

```toml
[dependencies]
wbraster = "0.1"
```

`wbraster` enables `zstd-native` by default. If you prefer pure-Rust decode-only Zstandard support, disable default features and enable `zstd-pure-rust-decode` instead:

```toml
[dependencies]
wbraster = { version = "0.1", default-features = false, features = ["zstd-pure-rust-decode"] }
```

Local workspace/path dependency:

```toml
[dependencies]
wbraster = { path = "../wbraster" }
```

## Quick Start

```rust
use wbraster::{
  CogWriteOptions,
  DataType,
  GeoTiffCompression,
  Jpeg2000Compression,
  Jpeg2000WriteOptions,
  JPEG2000_DEFAULT_LOSSY_QUALITY_DB,
  Raster,
  RasterConfig,
  RasterFormat,
};

let mut r = Raster::new(RasterConfig {
  cols: 100,
  rows: 100,
  bands: 1,
  x_min: 0.0,
  y_min: 0.0,
  cell_size: 1.0,
  nodata: -9999.0,
  data_type: DataType::F32,
  ..Default::default()
});

r.set(0, 50isize, 50isize, 42.0).unwrap();
r.set(0, 10isize, 10isize, -9999.0).unwrap();

r.write("dem.tif", RasterFormat::GeoTiff).unwrap();
r.write_cog("dem_cog_fast.tif").unwrap();

let cog_opts = CogWriteOptions {
  compression: Some(GeoTiffCompression::Deflate),
  bigtiff: Some(false),
  tile_size: Some(256),
};
r.write_cog_with_options("dem_cog_opts.tif", &cog_opts).unwrap();

let jp2_opts = Jpeg2000WriteOptions {
  compression: Some(Jpeg2000Compression::Lossy {
    quality_db: JPEG2000_DEFAULT_LOSSY_QUALITY_DB,
  }),
  decomp_levels: Some(5),
  color_space: None,
};
r.write_jpeg2000_with_options("dem.jp2", &jp2_opts).unwrap();

let r2 = Raster::read("dem.tif").unwrap();
assert_eq!(r2.get(0, 50isize, 50isize), 42.0);
assert!(r2.is_nodata(r2.get(0, 10isize, 10isize)));
assert_eq!(r2.get_opt(0, 10isize, 10isize), None);

let (col, row) = r2.world_to_pixel(50.5, 50.5).unwrap();
println!("pixel=({col},{row}) center=({:.3},{:.3})", r2.col_center_x(col), r2.row_center_y(row));
```

Note: pixel accessors use `(band, row, col)` with signed `row/col` (`isize`).
For single-band data use `band=0`. Out-of-bounds queries return the nodata
sentinel for `get`; use `get_opt` for valid-only optional access.

## Examples

The crate includes runnable examples in `examples/`:

- `raster_basics` (core create/read/write/sampling)
- `esri_ascii_io`
- `geotiff_cog_io`
- `geopackage_io`
- `zarr_io` (v2 and v3)
- `reproject_io`

Run with:

```bash
cargo run --example raster_basics
```

## Supported Formats

| Format | Extension(s) | Read | Write | Notes |
|---|---|:---:|:---:|---|
| **ENVI HDR Labelled** | `.hdr` + sidecar data | ✓ | ✓ | Multi-band (`BSQ`/`BIL`/`BIP`) |
| **ER Mapper** | `.ers` + data | ✓ | ✓ | Hierarchical header; reg-coord aware |
| **Esri ASCII Grid** | `.asc`, `.grd` | ✓ | ✓ | Handles `xllcorner` and `xllcenter` |
| **Esri Binary Grid** | workspace dir / `.adf` | ✓ | ✓ | Single-band float32, big-endian |
| **GeoTIFF / BigTIFF / COG** | `.tif`, `.tiff` | ✓ | ✓ | Stripped/tiled GeoTIFF + BigTIFF + COG writer |
| **GeoPackage Raster (Phase 4)** | `.gpkg` | ✓ | ✓ | Multi-band tiled raster; native-type raw tiles + PNG/JPEG options + extension registration |
| **GRASS ASCII Raster** | `.asc`, `.txt` | ✓ | ✓ | Header with `north/south/east/west`, `rows/cols` |
| **Idrisi/TerrSet Raster** | `.rdc` / `.rst` | ✓ | ✓ | byte, integer, real, RGB24 |
| **JPEG 2000 / GeoJP2** | `.jp2` | ✓ | ✓ | Pure-Rust JP2/GeoJP2 reader + writer |
| **PCRaster** | `.map` | ✓ | ✓ | CSF parser + value-scale aware writer (`UINT1`/`INT4`/`REAL4`/`REAL8`) |
| **SAGA GIS Binary** | `.sgrd` / `.sdat` | ✓ | ✓ | All SAGA data types; row-flip handled |
| **Surfer GRD** | `.grd` | ✓ | ✓ | Reads DSAA (ASCII) + DSRB (Surfer 7); writes DSAA by default, DSRB with `surfer_format=dsrb` |
| **Zarr v2/v3 (MVP)** | `.zarr` | ✓ | ✓ | 2D + 3D (`band,y,x`) chunked arrays |

### SAFE Bundle Support

`wbraster` includes package-level SAFE readers for Sentinel missions and a
mission auto-detection API.

**Sentinel-2 (`Sentinel2SafePackage`):**

```rust
use wbraster::Sentinel2SafePackage;

let pkg = Sentinel2SafePackage::open("S2A_MSIL2A_20260401T105021_N0510_R051_T32TQM_20260401T134528.SAFE")?;

println!("tile: {:?}", pkg.tile_id);
println!("solar zenith: {:?}°", pkg.mean_solar_zenith_deg);
println!("cloud cover: {:?}%", pkg.cloud_coverage_assessment);
println!("processing baseline: {:?}", pkg.processing_baseline);
println!("bands: {:?}", pkg.list_band_keys());
println!("qa layers: {:?}", pkg.list_qa_keys());
println!("aux layers: {:?}", pkg.list_aux_keys()); // AOT, WVP, TCI (L2A)

// resolve a spectral band path
if let Some(b04) = pkg.band_path("B04") {
    println!("red band at: {}", b04.display());
}

// resolve a QA layer
if let Some(scl) = pkg.qa_path("SCL") {
    println!("scene classification at: {}", scl.display());
}

// resolve L2A auxiliary layers (Aerosol Optical Thickness, Water Vapour Pressure)
if let Some(aot) = pkg.aux_path("AOT") {
  println!("AOT layer at: {}", aot.display());
}
if let Some(wvp) = pkg.aux_path("WVP") {
  println!("WVP layer at: {}", wvp.display());
}
```

**Sentinel-1 (`Sentinel1SafePackage`):**

```rust
use wbraster::Sentinel1SafePackage;

let pkg = Sentinel1SafePackage::open("S1A_IW_GRD_1SDV_20260401T052347_20260401T052412_053000_066E58_9F91.SAFE")?;

println!("product type: {:?}", pkg.product_type);
println!("acquisition mode: {:?}", pkg.acquisition_mode);
println!("polarization: {:?}", pkg.polarization);
println!("acquired at: {:?}", pkg.acquisition_datetime_utc);
println!("bounds (W/S/E/N): {:?}", pkg.spatial_bounds);
println!("measurements: {:?}", pkg.list_measurement_keys());

// resolve a measurement raster — key is MODE_PRODUCT_POL
if let Some(vv) = pkg.measurement_path("IW_GRD_VV") {
    println!("VV measurement raster at: {}", vv.display());
}

// read VV measurement directly as a Raster
let raster = pkg.read_measurement("IW_GRD_VV")?;

// read a calibrated raster in linear sigma0 units
let sigma0 = pkg.read_calibrated_measurement(
  "IW_GRD_VV",
  wbraster::Sentinel1CalibrationTarget::SigmaNought,
)?;

// or immediately in dB
let sigma0_db = pkg.read_calibrated_measurement_db(
  "IW_GRD_VV",
  wbraster::Sentinel1CalibrationTarget::SigmaNought,
)?;

// apply thermal-noise correction in linear units after calibration
let sigma0_nc = pkg.read_noise_corrected_calibrated_measurement(
  "IW_GRD_VV",
  wbraster::Sentinel1CalibrationTarget::SigmaNought,
)?;

// or noise-corrected directly in dB
let sigma0_nc_db = pkg.read_noise_corrected_calibrated_measurement_db(
  "IW_GRD_VV",
  wbraster::Sentinel1CalibrationTarget::SigmaNought,
)?;

// parse ECEF orbit state vectors from the annotation XML
let orbits = pkg.read_orbit_vectors("IW_GRD_VV")?;
println!("first orbit vector time: {}", orbits[0].time);
println!("position (m): {:?}", orbits[0].position);
println!("velocity (m/s): {:?}", orbits[0].velocity);

// bilinearly-interpolated geolocation grid (lat/lon/height/incidence angle)
let grid = pkg.read_geolocation_grid("IW_GRD_VV")?;
let (lat, lon) = grid.interpolated_lat_lon(512, 1024).unwrap();
let inc_angle = grid.interpolated_incidence_angle(512, 1024).unwrap();

// SLC TOPS burst metadata (returns Err for GRD products)
if let Ok(burst_list) = pkg.read_burst_list("IW1_SLC_VV") {
  println!("{} bursts, {} lines each", burst_list.bursts.len(), burst_list.lines_per_burst);
}

// multi-polarization batch operations
let pols = pkg.list_polarizations(); // e.g. ["VH", "VV"]
let vv_rasters = pkg.read_measurements_for_polarization("VV")?;
let vv_calibrated = pkg.read_calibrated_measurements_for_polarization(
  "VV",
  wbraster::Sentinel1CalibrationTarget::SigmaNought,
)?;
```

**Unified mission detection (`detect_safe_mission` / `open_safe_bundle`):**

```rust
use wbraster::{detect_safe_mission, open_safe_bundle, SafeBundle, SafeMission};

// inspect mission type without opening the full package
match detect_safe_mission("unknown.SAFE")? {
    SafeMission::Sentinel1 => println!("Sentinel-1 product"),
    SafeMission::Sentinel2 => println!("Sentinel-2 product"),
    SafeMission::Unknown   => println!("unrecognised SAFE bundle"),
}

// open and dispatch on variant
match open_safe_bundle("my_product.SAFE")? {
    SafeBundle::Sentinel1(pkg) => {
        println!("S1 measurements: {:?}", pkg.list_measurement_keys());
    }
    SafeBundle::Sentinel2(pkg) => {
        println!("S2 bands: {:?}", pkg.list_band_keys());
    }
}
```

This is package-level support (metadata + band/QA/measurement discovery), which
complements the raster I/O support for JPEG2000/GeoJP2 and GeoTIFF.

### Landsat Collection Bundle Support

`wbraster` also includes package-level Landsat Collection bundle support based on
MTL metadata plus GeoTIFF scene assets.

```rust
use wbraster::LandsatBundle;

let bundle = LandsatBundle::open("LC09_L2SP_018030_20240202_20240210_02_T1")?;

println!("mission: {:?}", bundle.mission);
println!("processing level: {:?}", bundle.processing_level);
println!("product id: {:?}", bundle.product_id);
println!("path/row: {:?}", bundle.path_row);
println!("cloud cover: {:?}%", bundle.cloud_cover_percent);

println!("bands: {:?}", bundle.list_band_keys());
println!("qa layers: {:?}", bundle.list_qa_keys());
println!("aux layers: {:?}", bundle.list_aux_keys());

if let Some(red) = bundle.band_path("B4") {
  println!("red band path: {}", red.display());
}
if let Some(qa_pixel) = bundle.qa_path("QA_PIXEL") {
  println!("QA pixel path: {}", qa_pixel.display());
}

let red = bundle.read_band("B4")?;
let qa = bundle.read_qa_layer("QA_PIXEL")?;
```

### ICEYE Bundle Support

`wbraster` includes an initial ICEYE bundle reader for COG/GeoTIFF assets with
XML metadata.

```rust
use wbraster::IceyeBundle;

let bundle = IceyeBundle::open("ICEYE_SCENE_DIR")?;

println!("product type: {:?}", bundle.product_type);
println!("mode: {:?}", bundle.acquisition_mode);
println!("acquired at: {:?}", bundle.acquisition_datetime_utc);
println!("polarization: {:?}", bundle.polarization);
println!("orbit direction: {:?}", bundle.orbit_direction);
println!("look direction: {:?}", bundle.look_direction);
println!("incidence near/far: {:?} / {:?}", bundle.incidence_angle_near_deg, bundle.incidence_angle_far_deg);
println!("assets: {:?}", bundle.list_asset_keys());
println!("pols: {:?}", bundle.list_polarizations());

if let Some(path) = bundle.asset_path("VV") {
  println!("VV asset at: {}", path.display());
}

let vv = bundle.read_asset("VV")?;
let vv_assets = bundle.read_assets_for_polarization("VV")?;
```

### PlanetScope Bundle Support

`wbraster` includes package-level PlanetScope support for common GeoTIFF assets
with JSON/XML sidecars.

```rust
use wbraster::PlanetScopeBundle;

let bundle = PlanetScopeBundle::open("PLANETSCOPE_SCENE_DIR")?;

println!("scene id: {:?}", bundle.scene_id);
println!("acquired at: {:?}", bundle.acquisition_datetime_utc);
println!("product type: {:?}", bundle.product_type);
println!("bands: {:?}", bundle.list_band_keys());
println!("qa layers: {:?}", bundle.list_qa_keys());

let red = bundle.read_band("B4")?;
if let Some(udm2) = bundle.qa_path("UDM2") {
  println!("udm2 mask path: {}", udm2.display());
}
```

### SPOT/Pleiades DIMAP Bundle Support

`wbraster` includes package-level DIMAP support for SPOT/Pleiades products
(`DIM_*.XML` plus JP2/GeoTIFF assets).

```rust
use wbraster::DimapBundle;

let bundle = DimapBundle::open("DIMAP_SCENE_DIR")?;

println!("mission: {:?}", bundle.mission);
println!("scene id: {:?}", bundle.scene_id);
println!("bands: {:?}", bundle.list_band_keys());

let pan = bundle.read_band("PAN")?;
```

### Maxar/WorldView Bundle Support

`wbraster` includes package-level Maxar/WorldView support for `.IMD`/XML
metadata and JP2/GeoTIFF assets.

```rust
use wbraster::MaxarWorldViewBundle;

let bundle = MaxarWorldViewBundle::open("MAXAR_WORLDVIEW_SCENE_DIR")?;

println!("satellite: {:?}", bundle.satellite);
println!("scene id: {:?}", bundle.scene_id);
println!("bands: {:?}", bundle.list_band_keys());

let blue = bundle.read_band("B2")?;
```

### RADARSAT-2 Bundle Support

`wbraster` includes an initial RADARSAT-2 bundle reader for GeoTIFF imagery and
`product.xml` metadata.

```rust
use wbraster::Radarsat2Bundle;

let bundle = Radarsat2Bundle::open("RS2_SCENE_DIR")?;

println!("product type: {:?}", bundle.product_type);
println!("mode: {:?}", bundle.acquisition_mode);
println!("acquired at: {:?}", bundle.acquisition_datetime_utc);
println!("pols: {:?}", bundle.polarizations);
println!("incidence near/far: {:?} / {:?}", bundle.incidence_angle_near_deg, bundle.incidence_angle_far_deg);
println!("spacing range/azimuth: {:?} / {:?}", bundle.pixel_spacing_range_m, bundle.pixel_spacing_azimuth_m);
println!("measurements: {:?}", bundle.list_measurement_keys());

let hh = bundle.read_measurement("HH")?;
let hh_set = bundle.read_measurements_for_polarization("HH")?;
```

### RCM Bundle Support

`wbraster` includes an initial RCM bundle reader for GeoTIFF imagery and XML
metadata.

```rust
use wbraster::RcmBundle;

let bundle = RcmBundle::open("RCM_SCENE_DIR")?;

println!("product type: {:?}", bundle.product_type);
println!("mode: {:?}", bundle.acquisition_mode);
println!("acquired at: {:?}", bundle.acquisition_datetime_utc);
println!("pols: {:?}", bundle.polarizations);
println!("incidence near/far: {:?} / {:?}", bundle.incidence_angle_near_deg, bundle.incidence_angle_far_deg);
println!("spacing range/azimuth: {:?} / {:?}", bundle.pixel_spacing_range_m, bundle.pixel_spacing_azimuth_m);
println!("measurements: {:?}", bundle.list_measurement_keys());

let vv = bundle.read_measurement("VV")?;
let vv_set = bundle.read_measurements_for_polarization("VV")?;
```

### Unified Sensor Bundle Detection

Use a single entrypoint to detect and open a supported bundle family
(Sentinel SAFE, Landsat, ICEYE, PlanetScope, DIMAP, Maxar/WorldView, RADARSAT-2, RCM):

```rust
use wbraster::{
  detect_sensor_bundle_family,
  open_sensor_bundle,
  SensorBundle,
  SensorBundleFamily,
};

match detect_sensor_bundle_family("some_bundle_root")? {
  SensorBundleFamily::Landsat => println!("Landsat bundle"),
  SensorBundleFamily::Iceye => println!("ICEYE bundle"),
  SensorBundleFamily::PlanetScope => println!("PlanetScope bundle"),
  SensorBundleFamily::Dimap => println!("SPOT/Pleiades DIMAP bundle"),
  SensorBundleFamily::MaxarWorldView => println!("Maxar/WorldView bundle"),
  SensorBundleFamily::Radarsat2 => println!("RADARSAT-2 bundle"),
  SensorBundleFamily::Rcm => println!("RCM bundle"),
  SensorBundleFamily::Sentinel1Safe => println!("Sentinel-1 SAFE"),
  SensorBundleFamily::Sentinel2Safe => println!("Sentinel-2 SAFE"),
  SensorBundleFamily::Unknown => println!("Unknown bundle"),
}

match open_sensor_bundle("some_bundle_root")? {
  SensorBundle::Landsat(pkg) => println!("bands: {:?}", pkg.list_band_keys()),
  SensorBundle::Iceye(pkg) => println!("assets: {:?}", pkg.list_asset_keys()),
  SensorBundle::PlanetScope(pkg) => println!("bands: {:?}", pkg.list_band_keys()),
  SensorBundle::Dimap(pkg) => println!("bands: {:?}", pkg.list_band_keys()),
  SensorBundle::MaxarWorldView(pkg) => println!("bands: {:?}", pkg.list_band_keys()),
  SensorBundle::Radarsat2(pkg) => println!("pols: {:?}", pkg.polarizations),
  SensorBundle::Rcm(pkg) => println!("pols: {:?}", pkg.polarizations),
  SensorBundle::Safe(pkg) => println!("SAFE bundle: {:?}", pkg),
}

// Also supports archive paths (.zip, .tar, .tar.gz, .tgz)
let opened = wbraster::open_sensor_bundle_path("LC09_scene_bundle.tar.gz")?;
match opened.bundle {
  SensorBundle::Landsat(pkg) => println!("Landsat bands: {:?}", pkg.list_band_keys()),
  _ => {}
}
// If opened from archive, you can optionally clean up the extracted temp tree:
if let Some(extracted_root) = opened.extracted_root {
  // std::fs::remove_dir_all(extracted_root)?;
}
```

### Bundle Canonical Key Reference

PlanetScope canonical keys:
- Bands: `B1`, `B2`, `B3`, `B4`, `B5`, `B6`, `B7`, `B8`, `ANALYTIC`
- QA: `UDM`, `UDM2`

DIMAP canonical keys:
- Bands: `PAN`, `B0`, `B1`, `B2`, `B3`, `B4`, `B5`, `SWIR`, `SWIR1`, `SWIR2`

Maxar/WorldView canonical keys:
- Bands: `PAN`, `B1`, `B2`, `B3`, `B4`, `B5`, `RE`, `Y`, `N2`, `SWIR`, `SWIR1`, `SWIR2`

### Real-Sample Smoke Tests (Opt-In)

The package readers include opt-in smoke tests that open local real datasets
when environment variables are set.

Set one or both variables to a local dataset path:

- `WBRASTER_LANDSAT_SAMPLE`: path to a Landsat scene directory.
- `WBRASTER_LANDSAT_SAMPLE_EXPECT_KEYS`: optional comma-separated expected canonical keys present in the sample (bands/QA/aux; e.g. `B2,B3,B4,QA_PIXEL`).
- `WBRASTER_S2_SAFE_SAMPLE`: path to a Sentinel-2 `.SAFE` root directory.
- `WBRASTER_S2_SAFE_SAMPLE_EXPECT_KEYS`: optional comma-separated expected canonical keys present in the sample (bands/QA/aux; e.g. `B02,B03,B04,MSK_CLDPRB`).
- `WBRASTER_ICEYE_SAMPLE`: path to an ICEYE scene directory.
- `WBRASTER_ICEYE_SAMPLE_EXPECT_KEYS`: optional comma-separated expected canonical asset keys present in the sample (e.g. `VV` or `VV_2`).
- `WBRASTER_ICEYE_OPEN_DATA_SAMPLE`: path to a local ICEYE Open Data scene directory (e.g. downloaded from the public STAC catalog).
- `WBRASTER_ICEYE_OPEN_DATA_SAMPLE_EXPECT_KEYS`: optional comma-separated expected canonical asset keys present in the sample.
- `WBRASTER_PLANETSCOPE_SAMPLE`: path to a PlanetScope scene directory.
- `WBRASTER_PLANETSCOPE_SAMPLE_EXPECT_PROFILES`: optional comma-separated expected PlanetScope profiles (e.g. `ANALYTIC,ANALYTIC_SR`).
- `WBRASTER_DIMAP_SAMPLE`: path to a SPOT/Pleiades DIMAP scene directory.
- `WBRASTER_DIMAP_SAMPLE_EXPECT_PROFILES`: optional comma-separated expected DIMAP profiles (e.g. `MS,PAN`).
- `WBRASTER_MAXAR_SAMPLE`: path to a Maxar/WorldView scene directory.
- `WBRASTER_MAXAR_SAMPLE_EXPECT_PROFILES`: optional comma-separated expected Maxar profiles (e.g. `MS,PAN`).
- `WBRASTER_RADARSAT2_SAMPLE`: path to a RADARSAT-2 scene directory.
- `WBRASTER_RADARSAT2_SAMPLE_EXPECT_KEYS`: optional comma-separated expected canonical measurement keys present in the sample (e.g. `HH,HV`).
- `WBRASTER_RCM_SAMPLE`: path to an RCM scene directory.
- `WBRASTER_RCM_SAMPLE_EXPECT_KEYS`: optional comma-separated expected canonical measurement keys present in the sample (e.g. `VV,VH`).

Run the smoke tests:

```bash
export WBRASTER_LANDSAT_SAMPLE="/path/to/LC08_or_LC09_or_LE07_scene"
export WBRASTER_LANDSAT_SAMPLE_EXPECT_KEYS="B2,B3,B4,QA_PIXEL"
cargo test -p wbraster opens_real_landsat_sample_when_env_set

export WBRASTER_S2_SAFE_SAMPLE="/path/to/S2A_or_S2B_product.SAFE"
export WBRASTER_S2_SAFE_SAMPLE_EXPECT_KEYS="B02,B03,B04,MSK_CLDPRB"
cargo test -p wbraster opens_real_s2_safe_sample_when_env_set

export WBRASTER_ICEYE_SAMPLE="/path/to/ICEYE_scene_dir"
export WBRASTER_ICEYE_SAMPLE_EXPECT_KEYS="VV"
cargo test -p wbraster opens_real_iceye_sample_when_env_set

export WBRASTER_ICEYE_OPEN_DATA_SAMPLE="/path/to/ICEYE_open_data_scene_dir"
export WBRASTER_ICEYE_OPEN_DATA_SAMPLE_EXPECT_KEYS="VV"
cargo test -p wbraster opens_real_iceye_open_data_sample_when_env_set

export WBRASTER_PLANETSCOPE_SAMPLE="/path/to/PLANETSCOPE_scene_dir"
export WBRASTER_PLANETSCOPE_SAMPLE_EXPECT_PROFILES="ANALYTIC,ANALYTIC_SR"
cargo test -p wbraster opens_real_planetscope_sample_when_env_set

export WBRASTER_DIMAP_SAMPLE="/path/to/SPOT_or_PLEIADES_DIMAP_scene_dir"
export WBRASTER_DIMAP_SAMPLE_EXPECT_PROFILES="MS,PAN"
cargo test -p wbraster opens_real_dimap_sample_when_env_set

export WBRASTER_MAXAR_SAMPLE="/path/to/MAXAR_or_WORLDVIEW_scene_dir"
export WBRASTER_MAXAR_SAMPLE_EXPECT_PROFILES="MS,PAN"
cargo test -p wbraster opens_real_maxar_worldview_sample_when_env_set

export WBRASTER_RADARSAT2_SAMPLE="/path/to/RADARSAT2_scene_dir"
export WBRASTER_RADARSAT2_SAMPLE_EXPECT_KEYS="HH,HV"
cargo test -p wbraster opens_real_radarsat2_sample_when_env_set

export WBRASTER_RCM_SAMPLE="/path/to/RCM_scene_dir"
export WBRASTER_RCM_SAMPLE_EXPECT_KEYS="VV,VH"
cargo test -p wbraster opens_real_rcm_sample_when_env_set
```

If a variable is unset (or points to a missing directory), its smoke test
returns early and is treated as a no-op.

For ICEYE Open Data, point the variable at a local directory containing at
least one product `.tif` and one metadata `.json` sidecar from the same scene.
One way to build that directory is:

```bash
mkdir -p /tmp/iceye_open_scene
curl -L "https://iceye-open-data-catalog.s3.amazonaws.com/data/dwell-fine/ICEYE_KDWN1B_20240305T105517Z_3521349_X23_SLEDF/ICEYE_KDWN1B_20240305T105517Z_3521349_X23_SLEDF_GRD.tif" -o /tmp/iceye_open_scene/ICEYE_KDWN1B_GRD.tif
curl -L "https://iceye-open-data-catalog.s3.amazonaws.com/data/dwell-fine/ICEYE_KDWN1B_20240305T105517Z_3521349_X23_SLEDF/ICEYE_KDWN1B_20240305T105517Z_3521349_X23_SLEDF_GRD.json" -o /tmp/iceye_open_scene/ICEYE_KDWN1B_GRD.json
export WBRASTER_ICEYE_OPEN_DATA_SAMPLE="/tmp/iceye_open_scene"
cargo test -p wbraster opens_real_iceye_open_data_sample_when_env_set
```

### Public Sample Sources (Recommended)

These links are useful for obtaining legal/public sample scenes for local smoke tests:

- Sentinel-2 SAFE: Copernicus Data Space Ecosystem and related Sentinel distribution mirrors.
- Landsat Collection: USGS EarthExplorer and USGS/Cloud public mirrors for Collection 2 products.
- ICEYE Open Data: ICEYE public STAC catalog/object storage (example command shown above).
- PlanetScope: Planet documentation and sample/data-access portals for account holders.
- SPOT/Pleiades DIMAP: Airbus sample/data portals for account holders and trial datasets.
- Maxar/WorldView: Maxar Open Data program and account-based sample data portals.
- RADARSAT-2 and RCM: typically license-gated; use organizationally licensed sample scenes where available.

### CRS support matrix

| Format | EPSG | WKT | PROJ4 | Notes |
|---|:---:|:---:|:---:|---|
| ENVI HDR Labelled | – | ✓ | – | Uses ENVI `coordinate system string`; preserves `map info` CRS tokens in metadata |
| ER Mapper | – | ~ | – | Preserves `CoordinateSpace` tokens (`er_datum`/`er_projection`/`er_coordinate_type`); WKT set only for WKT-like legacy datum values |
| Esri ASCII Grid | – | ~ | – | Reads/writes optional `.prj` sidecar; WKT is used when `.prj` content is WKT-like |
| Esri Binary Grid | – | ✓ | – | Reads/writes `prj.adf` |
| GeoTIFF / BigTIFF / COG | ✓ | – | – | Uses `raster.crs.epsg` |
| GeoPackage Raster (Phase 4) | ✓ | – | – | Uses `srs_id` in GeoPackage metadata tables |
| GRASS ASCII Raster | – | ~ | – | Reads/writes optional `.prj` sidecar; WKT is used when `.prj` content is WKT-like |
| Idrisi/TerrSet Raster | – | ~ | – | Reads/writes optional `.ref` sidecar; WKT is used when `.ref` content is WKT-like |
| JPEG 2000 / GeoJP2 | ✓ | – | – | Uses `raster.crs.epsg` |
| PCRaster | – | ~ | – | Reads/writes optional `.prj` sidecar; WKT is used when `.prj` content is WKT-like |
| SAGA GIS Binary | – | ✓ | – | Reads/writes optional `.prj` sidecar WKT (metadata key `saga_prj_text`, legacy alias `saga_prj_wkt`) |
| Surfer GRD | – | ~ | – | Reads/writes optional `.prj` sidecar; WKT is used when `.prj` content is WKT-like |
| Zarr v2/v3 (MVP) | ✓ | ✓ | ✓ | Uses metadata keys (`crs_epsg`/`epsg`, `crs_wkt`/`spatial_ref`, `crs_proj4`/`proj4`) |

Legend: `✓` supported, `–` not currently supported, `~` limited/custom representation.

See [CRS / spatial reference (CRS)](#crs--spatial-reference-crs) for setup/read-back examples and workflow guidance.
See [Sidecar metadata keys](#sidecar-metadata-keys) for format-specific sidecar CRS metadata names.

## Coordinate Reference System (CRS)

`Raster` stores CRS metadata in `raster.crs` using `CrsInfo`:

- `epsg: Option<u32>`
- `wkt: Option<String>`
- `proj4: Option<String>`

`CrsInfo` helpers:

- `CrsInfo::from_epsg(code)` sets `epsg` and also populates canonical OGC WKT when available.
- `CrsInfo::from_wkt(text)` stores `wkt` and infers `epsg` using adaptive matching (lenient default), including many legacy WKT cases without explicit authority tokens.
- `CrsInfo::from_wkt_with_policy(text, policy)` selects inference policy explicitly.
- `CrsInfo::from_wkt_strict(text)` rejects ambiguous matches (keeps `epsg=None` when no single best candidate exists).

`wbraster` stores and propagates CRS metadata and includes built-in
EPSG-to-EPSG reprojection/resampling APIs.

```rust
use wbraster::{Raster, RasterConfig, RasterFormat, CrsInfo, DataType};

let mut r = Raster::new(RasterConfig {
  cols: 256,
  rows: 256,
  bands: 1,
  x_min: -180.0,
  y_min: -90.0,
  cell_size: 0.01,
  nodata: -9999.0,
  data_type: DataType::F32,
  ..Default::default()
});

// Set CRS before writing (method 1: direct field assignment with CrsInfo helper)
r.crs = CrsInfo::from_epsg(4326);
r.write("dem.tif", RasterFormat::GeoTiff).unwrap();

// Alternative: use convenience methods for CRS assignment
r.assign_crs_epsg(4326);  // Sets EPSG code
r.assign_crs_wkt(wkt_string);  // Sets WKT definition
r.write("dem.tif", RasterFormat::GeoTiff).unwrap();

// Read CRS back
let r2 = Raster::read("dem.tif").unwrap();
println!("EPSG = {:?}", r2.crs.epsg);
println!("WKT  = {:?}", r2.crs.wkt.as_deref());
println!("PROJ = {:?}", r2.crs.proj4.as_deref());
```

**CRS Assignment Methods:**

`Raster` provides convenience methods for assigning CRS metadata:

- `raster.assign_crs_epsg(epsg_code)` — Replaces the entire CRS with a new `CrsInfo` containing only the EPSG code. Any existing WKT or PROJ4 fields are cleared to ensure consistency.
- `raster.assign_crs_wkt(wkt_string)` — Replaces the entire CRS with a new `CrsInfo` containing only the WKT definition. Any existing EPSG or PROJ4 fields are cleared to ensure consistency.

These methods ensure CRS consistency by preventing conflicting metadata (e.g., EPSG:4326 with WKT for EPSG:3857). They are useful when discovering CRS metadata after raster creation or when overriding existing CRS information. Remember to call `write()` after assignment to persist changes to file.

Format CRS behavior (current):

- GeoTIFF / COG: reads/writes EPSG via `raster.crs.epsg`.
- JPEG 2000 / GeoJP2: reads/writes EPSG via `raster.crs.epsg`.
- ENVI: reads/writes WKT via `coordinate system string`; also preserves/writes `map info` CRS tokens via metadata keys (`envi_map_projection`, `envi_map_datum`, `envi_map_units`).
- ER Mapper: preserves `CoordinateSpace` fields as metadata (`er_datum`, `er_projection`, `er_coordinate_type`); only WKT-like legacy `Datum` values populate `raster.crs.wkt`.
- Esri ASCII Grid: reads/writes optional `.prj` sidecar text via metadata key `esri_ascii_prj_text`; WKT-like content populates `raster.crs.wkt`.
- Esri Binary Grid: reads/writes WKT via `prj.adf`.
- GRASS ASCII Raster: reads/writes optional `.prj` sidecar text via metadata key `grass_ascii_prj_text`; WKT-like content populates `raster.crs.wkt`.
- Idrisi/TerrSet: reads/writes optional `.ref` sidecar text via metadata key `idrisi_ref_text`; WKT-like content populates `raster.crs.wkt`.
- PCRaster: reads/writes optional `.prj` sidecar text via metadata key `pcraster_prj_text`; WKT-like content populates `raster.crs.wkt`.
- SAGA GIS Binary: reads/writes WKT via optional `.prj` sidecar using metadata key `saga_prj_text` (legacy alias `saga_prj_wkt` also accepted).
- Surfer GRD: reads/writes optional `.prj` sidecar text via metadata key `surfer_prj_text`; WKT-like content populates `raster.crs.wkt`.
- Zarr v2/v3: reads/writes EPSG/WKT/PROJ4 metadata (`crs_epsg`/`epsg`, `crs_wkt`/`spatial_ref`, `crs_proj4`/`proj4`).
- Other formats: typically no dedicated CRS field; preserve CRS externally when needed.

### Reprojection

`wbraster` includes EPSG-to-EPSG raster reprojection using `wbprojection` with nearest-neighbor, bilinear, cubic, Lanczos-3, and thematic 3x3 resampling:

```rust
use wbraster::{
  AntimeridianPolicy,
  DestinationFootprint,
  GridSizePolicy,
  NodataPolicy,
  Raster,
  ReprojectOptions,
  ResampleMethod,
};

let input = Raster::read("input.tif")?;

// Requires input.crs.epsg to be set
let out_3857 = input.reproject_to_epsg(3857, ResampleMethod::Nearest)?;

// Convenience aliases
let out_4326 = out_3857.reproject_to_epsg_nearest(4326)?;
let out_3857_bilinear = input.reproject_to_epsg_bilinear(3857)?;
let out_3857_cubic = input.reproject_to_epsg_cubic(3857)?;
let out_3857_lanczos = input.reproject_to_epsg_lanczos(3857)?;
let out_3857_average = input.reproject_to_epsg_average(3857)?;
let out_3857_min = input.reproject_to_epsg_min(3857)?;
let out_3857_max = input.reproject_to_epsg_max(3857)?;
let out_3857_mode = input.reproject_to_epsg_mode(3857)?;
let out_3857_median = input.reproject_to_epsg_median(3857)?;
let out_3857_stddev = input.reproject_to_epsg_stddev(3857)?;

// Explicit output grid controls + nodata policy helper (fluent)
let opts = ReprojectOptions::new(3857, ResampleMethod::Bilinear)
  .with_size(2048, 2048)
  // or resolution-first sizing:
  // .with_resolution(30.0, 30.0)
  // .with_square_resolution(30.0)
  // optional snap alignment for resolution-derived grids:
  // .with_snap_origin(0.0, 0.0)
  // choose resolution sizing behavior: Expand (default) or FitInside
  .with_grid_size_policy(GridSizePolicy::Expand)
  // optionally mask cells outside transformed source footprint:
  .with_destination_footprint(DestinationFootprint::SourceBoundary)
  .with_nodata_policy(NodataPolicy::Fill)
  // optional for EPSG:4326 default extent derivation:
  // .with_antimeridian_policy(AntimeridianPolicy::Wrap)
  ;
// optionally: .with_extent(Extent { x_min: ..., y_min: ..., x_max: ..., y_max: ... })
let out_custom = input.reproject_with_options(&opts)?;

// Antimeridian policy comparison for EPSG:4326 outputs
let out_auto = input.reproject_with_options(
  &ReprojectOptions::new(4326, ResampleMethod::Nearest)
    .with_antimeridian_policy(AntimeridianPolicy::Auto)
)?;
let out_linear = input.reproject_with_options(
  &ReprojectOptions::new(4326, ResampleMethod::Nearest)
    .with_antimeridian_policy(AntimeridianPolicy::Linear)
)?;
let out_wrap = input.reproject_with_options(
  &ReprojectOptions::new(4326, ResampleMethod::Nearest)
    .with_antimeridian_policy(AntimeridianPolicy::Wrap)
)?;

// Match an existing reference grid exactly (CRS + extent + rows/cols)
let reference = Raster::read("reference_grid.tif")?;
let aligned = input.reproject_to_match_grid(&reference, ResampleMethod::Bilinear)?;

// Match reference CRS + resolution + snap origin, but keep auto-derived extent
let aligned_res = input.reproject_to_match_resolution(&reference, ResampleMethod::Bilinear)?;

// Match reference resolution/snap but force a different destination EPSG
let aligned_res_3857 = input.reproject_to_match_resolution_in_epsg(
  3857,
  &reference,
  ResampleMethod::Bilinear
)?;

// Advanced: explicit CRS objects (bypasses source `raster.crs.epsg` requirement)
let src_crs = wbprojection::Crs::from_epsg(4326)?;
let dst_crs = wbprojection::Crs::from_epsg(3857)?;
let out_custom_crs = input.reproject_with_crs(
  &src_crs,
  &dst_crs,
  &ReprojectOptions::new(3857, ResampleMethod::Bilinear)
)?;
```

Quick helper matrix:

| Helper | Destination CRS | Destination extent | Destination rows/cols | Resolution/snap source |
|---|---|---|---|---|
| `reproject_to_match_grid` | `target_grid.crs.epsg` | `target_grid.extent()` | `target_grid.cols/rows` | implied by target grid |
| `reproject_to_match_resolution` | `reference_grid.crs.epsg` | auto-derived from source footprint | derived from extent + resolution | `reference_grid.cell_size_*` + `reference_grid.(x_min,y_min)` |
| `reproject_to_match_resolution_in_epsg` | explicit `dst_epsg` | auto-derived from source footprint | derived from extent + resolution | reference resolution/snap transformed (if needed) to destination CRS |

Current capabilities (production-ready core):
- Standard convenience APIs are EPSG-based (source `raster.crs.epsg` + destination EPSG).
- Advanced custom-CRS path is available via `reproject_with_crs`.
- Includes `reproject_to_match_grid` for exact reference-grid alignment when target raster has EPSG.
- Includes `reproject_to_match_resolution` for reference resolution/snap alignment with auto-derived extent.
- Includes `reproject_to_match_resolution_in_epsg` for cross-CRS resolution/snap alignment using local transform at reference origin.
- Default output `rows`/`cols` equals input unless overridden in `ReprojectOptions`.
- Default output extent is derived from transformed sampled source-boundary points (corners + edge densification) unless overridden in `ReprojectOptions`.
- Optional resolution controls (`x_res`, `y_res`) can derive `cols`/`rows` from extent.
- Optional snap origin (`snap_x`, `snap_y`) aligns resolution-derived output grid bounds to a shared origin.
- If both size and resolution are provided, explicit `cols`/`rows` take precedence.
- For geographic outputs (EPSG:4326), antimeridian handling policy is configurable:
  - `Auto` (default): choose wrapped bounds only when narrower than linear bounds.
  - `Linear`: always use linear min/max longitude bounds.
  - `Wrap`: always use wrapped minimal-arc longitude bounds.
  - Practical guidance:
    - Use `Auto` for most workflows (safe default).
    - Use `Linear` when downstream tooling expects conventional min/max longitudes.
    - Use `Wrap` when working with dateline-crossing regions and you want the tightest longitude span.
- Resampling methods: nearest-neighbor (`Nearest`), bilinear (`Bilinear`), cubic (`Cubic`), Lanczos-3 (`Lanczos`), and thematic 3x3 (`Average`, `Min`, `Max`, `Mode`, `Median`, `StdDev`).
- Resolution-derived grid sizing policy (`ReprojectOptions.grid_size_policy`):
  - `Expand` (default): expands to fully cover requested extent.
  - `FitInside`: keeps generated grid within requested extent.
- Destination footprint handling (`ReprojectOptions.destination_footprint`):
  - `None` (default): no transformed-footprint masking.
  - `SourceBoundary`: masks destination cells outside transformed source boundary ring.
- Interpolation nodata policy (`ReprojectOptions.nodata_policy`):
  - `Strict`: requires full valid interpolation kernel.
  - `PartialKernel` (default): renormalizes over available valid kernel samples.
  - `Fill`: uses strict interpolation, then falls back to nearest-neighbor.

### Sidecar metadata keys

| Format | Sidecar | Preferred metadata key | Compatibility alias(es) |
|---|---|---|---|
| Esri ASCII Grid | `.prj` | `esri_ascii_prj_text` | – |
| GRASS ASCII Raster | `.prj` | `grass_ascii_prj_text` | – |
| Idrisi/TerrSet Raster | `.ref` | `idrisi_ref_text` | – |
| PCRaster | `.prj` | `pcraster_prj_text` | – |
| SAGA GIS Binary | `.prj` | `saga_prj_text` | `saga_prj_wkt` |
| Surfer GRD | `.prj` | `surfer_prj_text` | – |

## Common GeoTIFF / COG Workflows

```rust
use wbraster::{CogWriteOptions, GeoTiffCompression, Raster};

// Read any GeoTIFF-family input (GeoTIFF, BigTIFF, COG)
let input = Raster::read("input.tif").unwrap();

// Or use convenience defaults (deflate + tile 512)
input.write_cog("output_default.cog.tif").unwrap();

// Or choose a custom tile size while keeping convenience defaults
input.write_cog_with_tile_size("output_tile256.cog.tif", 256).unwrap();

// Or use COG-focused options without full GeoTIFF layout types
let cog_opts = CogWriteOptions {
  compression: Some(GeoTiffCompression::Deflate),
  bigtiff: Some(false),
  tile_size: Some(256),
};
input.write_cog_with_options("output_opts.cog.tif", &cog_opts).unwrap();
```

For non-COG GeoTIFF layouts (e.g., stripped/tiled non-COG output), use the
full `GeoTiffWriteOptions` + `Raster::write_geotiff_with_options(...)` API.

If you specifically need the lower-level TIFF / GeoTIFF / BigTIFF / COG engine rather than the higher-level multi-format raster abstraction, see [wbgeotiff](https://docs.rs/wbgeotiff).

## Architecture

```
wbraster/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs               ← public API + crate docs
│   ├── raster.rs            ← Raster core, typed storage, band helpers, iterators
│   ├── error.rs             ← RasterError, Result
│   ├── io_utils.rs          ← byte-order primitives, text helpers
│   ├── crs_info.rs       ← CrsInfo (WKT / EPSG / PROJ4)
│   └── formats/
│       ├── mod.rs            ← RasterFormat enum + auto-detect/dispatch
│       ├── envi.rs           ← ENVI HDR Labelled Raster (BSQ/BIL/BIP)
│       ├── er_mapper.rs      ← ER Mapper
│       ├── esri_ascii.rs     ← Esri ASCII Grid
│       ├── esri_binary.rs    ← Esri Binary Grid
│       ├── geopackage.rs     ← GeoPackage raster Phase 4 (multi-band tiled)
│       ├── geopackage_sqlite.rs ← low-level SQLite helpers for GeoPackage
│       ├── geotiff.rs        ← GeoTIFF / BigTIFF / COG adapter; delegates to `wbgeotiff` crate
│       ├── jpeg2000.rs       ← JPEG 2000 / GeoJP2 adapter for Raster
│       ├── jpeg2000_core/    ← integrated JPEG2000/GeoJP2 engine
│       │   ├── mod.rs
│       │   ├── reader.rs
│       │   ├── writer.rs
│       │   ├── boxes.rs
│       │   ├── codestream.rs
│       │   ├── wavelet.rs
│       │   ├── entropy.rs
│       │   ├── geo_meta.rs
│       │   ├── types.rs
│       │   └── error.rs
│       ├── grass_ascii.rs    ← GRASS ASCII Raster
│       ├── idrisi.rs         ← Idrisi/TerrSet Raster
│       ├── pcraster.rs       ← PCRaster (CSF)
│       ├── saga.rs           ← SAGA GIS Binary
│       ├── surfer.rs         ← Surfer GRD (DSAA/DSRB)
│       ├── zarr.rs           ← Zarr v2 + v3 dispatch
│       └── zarr_v3.rs        ← Zarr v3 implementation
├── tests/
│   └── integration.rs        ← cross-format round-trip integration tests
├── benches/
│   └── raster_access.rs      ← typed vs generic access benchmarks
```

### Design principles

- **Small dependency surface** — GeoTIFF I/O delegates to the standalone `wbgeotiff` crate; CRS reprojection delegates to `wbprojection`; Zarr support uses `serde_json` plus compression crates (`flate2`, `lz4_flex`, and optional zstd backend via `zstd-native` or `zstd-pure-rust-decode`).
- **Typed internal representation** — raster cells are stored in native typed
  buffers (`u8`, `u16`, `f32`, etc.) via `RasterData`, while convenience APIs
  still expose `f64` access where needed.
- **Performance** — buffered I/O (`BufReader` / `BufWriter` with 512 KiB
  buffers), row-level slicing, and in-place `map_valid` mutation.
- **Correctness** — each format correctly handles coordinate conventions
  (corner vs. center registration, top-to-bottom vs. bottom-to-top row order,
  byte-order flags).

## Data Type Support Per Format

| Format | U8 | I16 | I32 | F32 | F64 |
|---|:---:|:---:|:---:|:---:|:---:|
| ENVI HDR Labelled | ✓ | ✓ | ✓ | ✓ | ✓ |
| ER Mapper | ✓ | ✓ | ✓ | ✓ | ✓ |
| Esri ASCII Grid | ✓¹ | ✓¹ | ✓¹ | ✓ | ✓ |
| Esri Binary Grid | – | – | – | ✓ | – |
| GeoTIFF / COG | ✓ | ✓ | ✓ | ✓ | ✓ |
| GeoPackage Raster (Phase 4) | ✓ | ✓ | ✓ | ✓ | ✓ |
| GRASS ASCII Raster | ✓¹ | ✓¹ | ✓¹ | ✓ | ✓ |
| Idrisi/TerrSet Raster | ✓ | ✓ | – | ✓ | – |
| JPEG 2000 / GeoJP2 | ✓ | ✓ | – | ✓ | ✓ |
| PCRaster | ✓ | ✓ | ✓ | ✓ | ✓ |
| SAGA GIS Binary | ✓ | ✓ | ✓ | ✓ | ✓ |
| Surfer GRD | – | – | – | ✓ | ✓ |
| Zarr v2/v3 (MVP) | ✓ | ✓ | ✓ | ✓ | ✓ |

¹ ASCII stores all types as text; write uses the `data_type` field for hint only.
### Auto-detect Notes
- `.grd` is signature-sniffed: `DSAA`/`DSRB` routes to `SurferGrd`, otherwise `EsriAscii`.
- `.asc`/`.txt` are header-sniffed between `GrassAscii` and `EsriAscii`.
- `.map` is signature-sniffed for PCRaster CSF (`RUU CROSS SYSTEM MAP FORMAT`).
- `.gpkg` routes to `GeoPackage` raster Phase 4.

### GeoPackage Phase 4 Notes
- Writer defaults: single zoom (`0`); tile encoding defaults to PNG for `U8` and raw native tiles for non-`U8` data.
- Optional write-time metadata keys:
  - `gpkg_max_zoom`: non-negative integer (number of additional pyramid levels)
  - `gpkg_tile_size`: tile width/height in pixels (`16..4096`, default `256`)
  - `gpkg_tile_format`: `png` or `jpeg`/`jpg` (image-encoded/quantized tiles)
  - `gpkg_tile_encoding`: `raw`, `png`, or `jpeg` (overrides default encoding selection)
  - `gpkg_raw_compression`: `none` or `deflate` (applies when `gpkg_tile_encoding=raw`; default is `deflate` unless explicitly overridden)
  - `gpkg_jpeg_quality`: `1..100` (used when `gpkg_tile_format=jpeg`)
  - `gpkg_dataset_name`: SQL identifier override for internal dataset name (default `wbraster_dataset`)
  - `gpkg_base_table_name`: SQL identifier override for tile table base name (default `raster_tiles`)
- Reader selects the finest available zoom level from `gpkg_tile_matrix` and supports both raw native tiles and PNG/JPEG tile blobs.
- Writer registers `gpkg_extensions` rows for custom wbraster raw-tile and metadata extensions.
- When multiple `wbraster_gpkg_raster_metadata` dataset rows are present, reader selection can be overridden using environment variable `WBRASTER_GPKG_DATASET=<dataset_name>`.
- Programmatic dataset helpers are available via `geopackage::list_datasets(path)` and `geopackage::read_dataset(path, dataset_name)`.

### GeoPackage phase progression

| Phase | Key capabilities |
|---|---|
| Phase 1 | Multi-band read/write, native dtype-preserving raw tiles, PNG/JPEG tile options, metadata side tables |
| Phase 2 | Tiling/compression policy controls (`gpkg_tile_size`, `gpkg_raw_compression`) with tested defaults and overrides |
| Phase 3 | Interoperability hardening: `gpkg_extensions` registration, configurable/sanitized dataset/table naming, robust multi-dataset selection and metadata consistency validation |
| Phase 4 | Explicit multi-dataset APIs (`geopackage::list_datasets`, `geopackage::read_dataset`) with strict named-dataset reads |

### PCRaster Notes
- Reader supports core CSF cell representations: `CR_UINT1`, `CR_INT4`, `CR_REAL4`, `CR_REAL8`.
- Writer supports:
  - `VS_BOOLEAN` / `VS_LDD` as `CR_UINT1`
  - `VS_NOMINAL` / `VS_ORDINAL` as `CR_UINT1` or `CR_INT4`
  - `VS_SCALAR` / `VS_DIRECTION` as `CR_REAL4` or `CR_REAL8`
- Optional metadata overrides on write:
  - `pcraster_valuescale`: `boolean|nominal|ordinal|scalar|direction|ldd`
  - `pcraster_cellrepr`: `uint1|int4|real4|real8`

## GeoTIFF / COG Notes

- `RasterFormat::GeoTiff` reads GeoTIFF, BigTIFF, and COG files.
- GeoTIFF / COG support in `wbraster` is implemented on top of the standalone [wbgeotiff](https://docs.rs/wbgeotiff) crate.
- Write mode defaults to deflate-compressed GeoTIFF.
- `Raster::write_cog(path)` writes a COG with convenience defaults
  (deflate compression, tile size 512, BigTIFF disabled).
- `Raster::write_cog_with_tile_size(path, tile_size)` does the same with a
  caller-specified COG tile size.
- `Raster::write_cog_with_options(path, &CogWriteOptions)` exposes only
  COG-relevant knobs (`compression`, `bigtiff`, `tile_size`).

### Preferred typed write API

- For COG output, prefer `Raster::write_cog_with_options(path, &CogWriteOptions)`.
- GeoTIFF/COG metadata write controls are no longer consumed.
- Leaving an option as `None` uses built-in defaults.

| COG typed field | Type | Effect | Default |
|---|---|---|---|
| `compression` | `Option<GeoTiffCompression>` | Compression codec | `deflate` |
| `bigtiff` | `Option<bool>` | Force BigTIFF container | `false` |
| `tile_size` | `Option<u32>` | COG tile size (pixels) | `512` |

Notes:
- For full GeoTIFF control beyond COG-focused fields, use
  `Raster::write_geotiff_with_options(path, &GeoTiffWriteOptions)`.

| Advanced GeoTIFF field | Type | Effect | Default |
|---|---|---|---|
| `compression` | `Option<GeoTiffCompression>` | Compression codec | `deflate` |
| `bigtiff` | `Option<bool>` | Force BigTIFF container | `false` |
| `layout` | `Option<GeoTiffLayout>` | Writer mode/layout | `GeoTiffLayout::Standard` |

### GeoTIFF/COG metadata key reference

The GeoTIFF adapter populates read-back metadata descriptors for input files.

The keys below are **read-back descriptors only**; writer configuration now
uses `GeoTiffWriteOptions` exclusively.

| Metadata key | Direction | Purpose | Accepted values / default | Notes |
|---|---|---|---|---|
| `geotiff_compression` | Read | Source compression codec descriptor | `none`, `lzw`, `deflate`, `packbits`, `jpeg`, `webp`, `jpeg-xl` | Populated by reader; informational. |
| `geotiff_is_bigtiff` | Read | Source file container type | `true` / `false` | Populated by reader; informational. |
| `geotiff_is_cog_candidate` | Read | Source compression suggests COG-friendly profile | `true` / `false` (or omitted) | Populated by reader; informational hint only. |

Notes:
- CRS/EPSG on write is taken from `raster.crs.epsg`.
- Configure write behavior with `GeoTiffWriteOptions` (`compression`, `bigtiff`, `layout`).

## JPEG2000 / GeoJP2 Notes

- `RasterFormat::Jpeg2000` reads and writes `.jp2` files.
- Default write mode is lossy (`Jpeg2000Compression::Lossy { quality_db: JPEG2000_DEFAULT_LOSSY_QUALITY_DB }`).
- Current integration is focused on adapter wiring and typed write options; treat
  production decode compatibility as evolving.

### Preferred typed write API

- `Raster::write_jpeg2000_with_options(path, &Jpeg2000WriteOptions)` configures
  JPEG2000 output.
- Leaving an option as `None` uses built-in defaults.

| Typed field | Type | Effect | Default |
|---|---|---|---|
| `compression` | `Option<Jpeg2000Compression>` | Lossless/lossy mode | `Lossy { quality_db: JPEG2000_DEFAULT_LOSSY_QUALITY_DB }` |
| `decomp_levels` | `Option<u8>` | Number of wavelet decomposition levels | writer default (`5`) |
| `color_space` | `Option<ColorSpace>` | Output JP2 color space | inferred from band count |

## ENVI Metadata Key Reference

| Metadata key | Direction | Purpose | Accepted values / default | Notes |
|---|---|---|---|---|
| `envi_interleave` | Read + Write | Interleave mode | `bsq`, `bil`, `bip` / default `bsq` | Reader writes this key into `Raster.metadata`; writer consumes it for both header and data layout. |
| `description` | Read + Write | ENVI header description string | any string / default write: `Created by gis_raster` | Reader populates from `description = {...}` if present; writer emits it to header. |
| `envi_map_projection` | Read + Write | ENVI `map info` projection token | any string / default write: `Geographic Lat/Lon` | Preserved from `map info` first field when present. |
| `envi_map_datum` | Read + Write | ENVI `map info` datum token | any string / optional | Parsed from `map info` datum position when present; used on write if supplied. |
| `envi_map_units` | Read + Write | ENVI `map info` units hint | any string / optional | Parsed from `map info` units slot when present; written as `units=<value>`. |
| `envi_coordinate_system_string` | Read + Write | ENVI `coordinate system string` raw WKT | WKT string / optional | Reader mirrors this value; writer uses `raster.crs.wkt` first, then this key as fallback. |

Notes:
- ENVI header field `data file` is parsed internally on read for sidecar resolution, but is not persisted as a `Raster.metadata` key.

## Zarr Notes

- Zarr support targets **local filesystem stores** for both v2 and v3.
- Reads and writes **2D arrays** and **3D arrays** in `(band, y, x)` form.
- Supported compressors: `zlib`, `gzip`, `zstd`, `lz4`, or none.
- `zstd` behavior is feature-gated:
  - `zstd-native` (default): read + write via native `zstd` bindings.
  - `zstd-pure-rust-decode`: read-only zstd decode via `ruzstd`; zstd encoding is unavailable.
- Default write uses `zlib` level 6 and Zarr v2.
- Select write version with metadata key `zarr_version` (`2` default, `3` for v3).

### Validation mode

Both readers support explicit validation strictness via the `zarr_validation_mode` attribute in `.zattrs`
(v2) or in the `attributes` block of `zarr.json` (v3):

- `strict` (default): fails on conflicting or invalid geospatial metadata.
- `lenient`: performs best-effort reads, ignoring non-critical metadata conflicts (e.g., a `GeoTransform`
  string that disagrees with explicit origin/cell-size keys).

### Nodata conventions

Nodata is resolved in this precedence order:

1. explicit `nodata` attribute
2. `_FillValue` (CF convention, used by `xarray`, `rioxarray`)
3. `missing_value`
4. Zarr `fill_value`
5. default −9999

Zarr stores produced by CF-convention tools that write only `_FillValue` are therefore read correctly
without any extra configuration.

### CRS interoperability

The readers recognize a broad set of CRS representations:

| Source convention | Recognized attribute key(s) |
|---|---|
| Whitebox native | `crs_epsg`, `crs_wkt`, `crs_proj4` |
| Common aliases | `epsg`, `spatial_ref`, `proj4` |
| Plain `"EPSG:NNNN"` string | `crs` |
| OGC URN / URL | `crs` (e.g., `urn:ogc:def:crs:EPSG::4326`, `https://www.opengis.net/def/crs/EPSG/0/4326`) |
| Object with `properties.name` | `crs` (e.g., `{"properties": {"name": "EPSG:4326"}}`) |
| Object with `id.authority/code` | `crs` (e.g., `{"id": {"authority": "EPSG", "code": "4326"}}`) |
| CF `grid_mapping` named object | `grid_mapping` key referencing an object; extracts EPSG, WKT, or proj4 |
| GDAL-style `GeoTransform` | six-element affine string; origin and cell size are recovered from it |
| Affine `transform` array | `[x_min, cell_x, 0, y_min, 0, cell_y]` |

### Multi-scale group support (OME-NGFF)

`Raster::read("store.zarr")` automatically detects **OME-NGFF multi-scale groups** for both v2 and v3.

When the path points to a group root:

- **OME-NGFF `multiscales` attribute present** — levels are read from `multiscales[0].datasets[].path`;
  level 0 (finest resolution) is opened by default.
- **No OME attributes** — consecutive numeric sub-directories (`0/`, `1/`, `2/`, …) are scanned; the
  first valid array is opened.

To open a specific resolution level, point the path directly at the sub-array directory:

```rust
// Default: opens finest resolution (level 0)
let full_res = Raster::read("image.zarr")?;

// Direct path: opens coarser level 1
let half_res = Raster::read("image.zarr/1")?;
```

### Zarr v3 `transpose` codec

The v3 reader supports the `transpose` codec for F-order and custom-permutation arrays:

- `"F"` order reverses the natural axis order.
- A numeric permutation array (e.g., `[2, 1, 0]`) specifies the exact axis mapping.
- C-order arrays (including those with an explicit `"C"` transpose entry) are read without remapping.

### `dimension_names` validation (v3)

In strict mode, the v3 reader rejects 3D arrays whose `dimension_names` place spatial axes before the
band axis (e.g., `["y", "x", "band"]`) with an actionable error. Users can resolve this by adding a
`transpose` codec in the producer or setting `zarr_validation_mode = "lenient"` in the store attributes.
Standard 2D layouts and unrecognized dimension names are always accepted.

### v2 specifics

- Set chunk-key style by adding metadata key `zarr_dimension_separator` (`/` or `.`) before writing.
- Geospatial metadata is written to `.zattrs` (`_ARRAY_DIMENSIONS`, `transform`, `x_min`, `y_min`,
  `cell_size_x`, `cell_size_y`, `nodata`, `crs_epsg`, `crs_wkt`, `crs_proj4`).
- Chunk controls: `zarr_chunk_rows`, `zarr_chunk_cols`, `zarr_chunk_bands`.

### v3 specifics

- Supports regular chunk grids with C-order traversal (multi-chunk included).
- Supports chunk key encoding `default` and `v2` with `.` or `/` separators.
- Supports `bytes` codec pipeline with optional `transpose` codec plus compressor.
- Compressors: `zlib`, `gzip`, `zstd`, `lz4`.
- Geospatial metadata/CRS is stored in `zarr.json` under `attributes`.
- Chunk controls: `zarr_chunk_rows`, `zarr_chunk_cols`, `zarr_chunk_bands`.

### Zarr v3 write example

```rust
use wbraster::{Raster, RasterConfig, RasterFormat, DataType};

let mut r = Raster::new(RasterConfig {
  cols: 1024,
  rows: 1024,
  x_min: 0.0,
  y_min: 0.0,
  cell_size: 1.0,
  nodata: -9999.0,
  data_type: DataType::F32,
  ..Default::default()
});

// Request Zarr v3 output with custom chunking and key encoding.
r.metadata.push(("zarr_version".into(), "3".into()));
r.metadata.push(("zarr_chunk_rows".into(), "256".into()));
r.metadata.push(("zarr_chunk_cols".into(), "256".into()));
r.metadata.push(("zarr_chunk_key_encoding".into(), "default".into()));
r.metadata.push(("zarr_dimension_separator".into(), "/".into()));
r.metadata.push(("zarr_compressor".into(), "zstd".into()));
r.metadata.push(("zarr_compression_level".into(), "3".into()));

r.write("dem_v3.zarr", RasterFormat::Zarr).unwrap();
```

### Zarr v2 advanced write example

```rust
use wbraster::{Raster, RasterConfig, RasterFormat, DataType};

let mut r = Raster::new(RasterConfig {
  cols: 1024,
  rows: 1024,
  x_min: 0.0,
  y_min: 0.0,
  cell_size: 1.0,
  nodata: -9999.0,
  data_type: DataType::F32,
  ..Default::default()
});

// Optional v2 controls for chunk key style.
r.metadata.push(("zarr_version".into(), "2".into()));
r.metadata.push(("zarr_dimension_separator".into(), "/".into()));

r.write("dem_v2.zarr", RasterFormat::Zarr).unwrap();
```

### Zarr metadata key quick reference

| Metadata key | Version | Purpose | Accepted values / default |
|---|---|---|---|
| `zarr_version` | v2 + v3 | Select writer implementation / read descriptor | `2` (default), `3` | Reader populates `2` or `3` in output metadata. |
| `zarr_dimension_separator` | v2 + v3 | Chunk key separator | `.` or `/` | Also accepts alias key `zarr_chunk_separator` on write. |
| `zarr_chunk_separator` | v2 + v3 | Alias for separator key | `.` or `/` | Alias of `zarr_dimension_separator` (write-time lookup). |
| `zarr_chunk_bands` | v2 + v3 | Band chunk depth for 3D (`band,y,x`) writes | positive integer, clamped to `[1, bands]`, default `1` | Used by both v2 and v3 writers for multiband chunking. |
| `zarr_chunk_rows` | v2 + v3 | Chunk height for writes | positive integer, clamped to `[1, rows]`, default `min(rows, 256)` | |
| `zarr_chunk_cols` | v2 + v3 | Chunk width for writes | positive integer, clamped to `[1, cols]`, default `min(cols, 256)` | |
| `zarr_chunk_key_encoding` | v3 | Chunk key encoding style | `default` (default), `v2` | Reader populates this key for v3 stores. |
| `zarr_compressor` | v3 | Compression algorithm | `zlib` (default), `gzip`, `gz`, `zstd`, `lz4`, `none` | v3 writer uses this to build codec pipeline. |
| `zarr_compression_level` | v3 | Compression level hint | integer; optional | Applied only when compressor supports configurable level. |
| `zarr_validation_mode` | v2 + v3 | Read-time validation strictness | `strict` (default), `lenient` | Set in store attributes (`.zattrs` or `zarr.json`). |

## Zarr Implementation Status

What is currently supported:

- Local filesystem stores, v2 and v3
- 2D and 3D `(band, y, x)` arrays, including multi-chunk
- `bytes` codec + optional compressor (`zlib`, `gzip`, `zstd`, `lz4`)
- v3 `transpose` codec (F-order, C-order, and explicit permutation)
- Chunk key encoding `default` and `v2` with `.` or `/` separators
- Write-time chunk controls (`zarr_chunk_rows`, `zarr_chunk_cols`, `zarr_chunk_bands`) for both v2 and v3
- Strict and lenient validation modes via `zarr_validation_mode`
- CF-convention nodata fallbacks (`_FillValue`, `missing_value`)
- Broad CRS representation interoperability (aliases, object-style, OGC URN/URL, CF `grid_mapping`, `GeoTransform`, affine `transform`)
- `dimension_names` semantic validation for 3D v3 arrays
- OME-NGFF multi-scale group detection and automatic level-0 selection (v2 and v3)
- External fixture smoke tests (env-gated) for parity verification

Not currently supported:

- Remote / cloud stores (S3, HTTP) — use `rclone` mount or pre-download as a workaround
- Arbitrary N-dimensional arrays (only 2D and 3D `band,y,x` layouts)
- Zarr v3 codec extensions beyond `bytes`, `transpose`, and the listed compressors

See also: [SIMD guardrail check](../../README.md#simd-guardrail-check) for a script you can run locally to verify speedup and correctness.

## Performance

This library uses the [`wide`](https://github.com/Lokathor/wide) crate to provide **SIMD optimizations** for selected raster-processing hot paths. The current coverage includes:

- statistics accumulation over raster ranges, with explicit scalar and SIMD benchmark modes
- the strict bicubic 4x4 kernel reduction used during reprojection sampling

The `wide` crate offers portable SIMD abstractions that work across x86-64, ARM, WebAssembly, and other platforms without requiring `unsafe` code in end-user applications.

SIMD is **enabled by default** in this crate. There is currently no feature flag required to turn SIMD paths on.

This is a **temporary implementation strategy** until [Portable SIMD](https://github.com/rust-lang/rfcs/blob/master/text/2948-portable-simd.md) stabilizes in Rust. Once portable SIMD is available in stable Rust, `wbraster` will transparently migrate to that standard approach while maintaining the same performance characteristics.

You can run the current statistics benchmark example with:

```bash
cargo run --release --example simd_stats_compute
```

That example compares scalar and SIMD statistics paths directly and validates that both modes return matching results.

## Benchmarking

Run the raster access benchmark suite:

```bash
cargo bench --bench raster_access
```

Save results to a timestamped file:

```bash
mkdir -p benches/results && cargo bench --bench raster_access | tee "benches/results/raster_access_$(date +%Y%m%d_%H%M%S).txt"
```

If your shell does not support that `date` format, use:

```bash
mkdir -p benches/results && cargo bench --bench raster_access | tee benches/results/raster_access_latest.txt
```

Current benchmark groups:

- `f32_access` — sequential scan: `iter_f64` vs direct `f32` typed-slice access.
- `u16_access` — sequential scan: `iter_f64` vs direct `u16` typed-slice access.
- `random_access` — scattered reads: `get_raw(band,col,row)` vs direct typed indexing with precomputed probe indices.

Interpretation tips:

- Compare `typed_*` vs `iter_f64` to estimate conversion overhead during full-array scans.
- Compare `typed_*_direct` vs `get_raw_*` to isolate bounds/indexing overhead in random-access workloads.
- Use relative speedup in your target data type as the decision signal for choosing generic vs typed code paths.

### Results template

Record representative medians from your local run (same machine/config for fair comparison):

| Benchmark ID | Baseline (ns/iter) | Current (ns/iter) | Speedup (`baseline/current`) | Notes |
|---|---:|---:|---:|---|
| `f32_access/iter_f64/512x512` |  |  |  |  |
| `f32_access/typed_f32_slice/512x512` |  |  |  |  |
| `f32_access/iter_f64/2048x2048` |  |  |  |  |
| `f32_access/typed_f32_slice/2048x2048` |  |  |  |  |
| `u16_access/iter_f64/512x512` |  |  |  |  |
| `u16_access/typed_u16_slice/512x512` |  |  |  |  |
| `u16_access/iter_f64/2048x2048` |  |  |  |  |
| `u16_access/typed_u16_slice/2048x2048` |  |  |  |  |
| `random_access/get_raw_f32/2048x2048` |  |  |  |  |
| `random_access/typed_f32_direct/2048x2048` |  |  |  |  |
| `random_access/get_raw_u16/2048x2048` |  |  |  |  |
| `random_access/typed_u16_direct/2048x2048` |  |  |  |  |

## Compilation Features

| Feature | Default | Purpose |
|---------|:-------:|---------|
| `zstd-native` | **yes** | Native-linked Zstandard bindings for read + write. Best throughput on most platforms. |
| `zstd-pure-rust-decode` | no | Pure-Rust `ruzstd`-backed Zstandard **decode only**. Cannot write zstd. Suitable for WebAssembly or environments where native linking is unavailable. |

At most one zstd variant should be enabled at a time. Example — disable native, enable pure-Rust decode:

```toml
[dependencies]
wbraster = { version = "0.1", default-features = false, features = ["zstd-pure-rust-decode"] }
```

Example — no zstd at all:

```toml
[dependencies]
wbraster = { version = "0.1", default-features = false }
```

All other codecs (Deflate/zlib, LZ4, LZW, JPEG, WebP, PNG, JPEG XL) are unconditionally included and require no feature flag.

## Known Limitations

- `wbraster` focuses on format I/O; raster analysis and processing operations belong in higher-level Whitebox tooling.
- Zarr support targets local filesystem stores and 2D/3D `(band, y, x)` arrays; remote/cloud stores (S3, HTTP) are not natively supported — use a `rclone` FUSE mount or pre-download as a workaround; arbitrary N-dimensional arrays are not supported.
- GeoPackage Raster (Phase 4) supports single-dataset read by default; multi-dataset disambiguation is handled via explicit API or the `WBRASTER_GPKG_DATASET` environment variable.
- JPEG 2000 / GeoJP2 codec compatibility is evolving; treat production decode compatibility as active work.
- Reprojection uses EPSG-based source CRS metadata; formats that store CRS only as WKT require adaptive EPSG identification which may fail for uncommon or authority-marker-free WKT strings.
- BigTIFF write produces valid BigTIFF files but downstream tool compatibility (when consuming from non-GDAL tools) may vary.

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
