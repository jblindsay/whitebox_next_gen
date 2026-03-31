# wbvector

Pure-Rust library for reading and writing common vector GIS formats with a single in-memory feature model intended to serve as the vector engine for [Whitebox](https://www.whiteboxgeo.com).

## Table of Contents

- [Mission](#mission)
- [The Whitebox Project](#the-whitebox-project)
- [Is wbvector Only for Whitebox?](#is-wbvector-only-for-whitebox)
- [What wbvector Is Not](#what-wbvector-is-not)
- [Features](#features)
- [Supported Formats](#supported-formats)
- [Format Properties (Detailed Comparison)](#format-properties-detailed-comparison)
- [Install](#install)
- [Quick Start](#quick-start)
- [Format Drivers](#format-drivers)
- [Generic Sniffed I/O](#generic-sniffed-io)
- [Geometry and Attributes](#geometry-and-attributes)
- [CRS Model](#crs-model)
- [Architecture](#architecture)
- [Performance Notes](#performance-notes)
- [Examples](#examples)
- [OSM Examples](#osm-examples)
- [Known Limitations](#known-limitations)
- [Development](#development)
- [License](#license)

## Mission

- Provide multi-format vector GIS I/O for Whitebox applications and workflows.
- Support a single unified in-memory feature model that round-trips correctly across all supported formats.
- Cover the most commonly used open vector formats in a single pure-Rust library.
- Minimize external dependencies and avoid native/C++ linkage.

## The Whitebox Project

Whitebox is a collection of related open-source geospatial data analysis software. The Whitebox project began in 2009 at the [University of Guelph](https://geg.uoguelph.ca), Canada, developed by [Dr. John Lindsay](https://jblindsay.github.io/ghrg/index.html) a professor of geomatics. Whitebox has long served as Dr. Lindsay's platform for disseminating the output of his geomatics-based research and has developed an extensive worldwide user base. In 2021 Dr. Lindsay and Anthony Francioni founded [Whitebox Geospatial Inc.](https://www.whiteboxgeo.com) in order to ensure the sustainable and ongoing development of this open-source geospatial project. We are currently working on the next iteration of the Whitebox software, **Whitebox Next Gen**. This crate is part of that larger effort.

## Is wbvector Only for Whitebox?

No. `wbvector` is developed primarily to support Whitebox, but it is not restricted to Whitebox projects.

- **Whitebox-first**: API and roadmap decisions prioritize Whitebox vector I/O needs.
- **General-purpose**: the crate is usable as a standalone multi-format vector library in other Rust geospatial applications.
- **Format-complete**: 11 supported formats with full round-trip read/write, cross-format conversion, and CRS propagation make it broadly useful.

## What wbvector Is Not

`wbvector` is a format I/O and feature model layer. It is **not** a full vector analysis framework.

- Not a spatial analysis library (overlay, buffering, simplification belong in `wbtopology`).
- Not a rendering or visualization engine.
- Not a spatial database engine (GeoPackage support is focused on single-layer workflows).
- Not a network or protocol layer (all I/O is file-based).

## Features

- Easy cross-format conversion (`read` into a `Layer`, then `write` to another format)
- Layer reprojection utilities backed by `wbprojection`
- Pure Rust implementation (core external dependencies include `thiserror`, `flatbuffers`, and `wbprojection`)

## Supported Formats

| Format | Read | Write | Notes |
|--------|:----:|:-----:|-------|
| **FlatGeobuf** (`.fgb`) | ✓ | ✓ | High-performance binary interchange |
| **GeoJSON** (`.geojson`) | ✓ | ✓ | Web-friendly JSON text format |
| **GeoPackage** (`.gpkg`) | ✓ | ✓ | SQLite container; supports multi-layer workflows |
| **Geography Markup Language** (`.gml`) | ✓ | ✓ | Standards-based XML exchange |
| **GPS Exchange Format** (`.gpx`) | ✓ | ✓ | GPS tracks/routes/waypoints |
| **Keyhole Markup Language** (`.kml`) | ✓ | ✓ | Google Earth-style visualization format |
| **MapInfo Interchange** (`.mif` + `.mid`) | ✓ | ✓ | Legacy MapInfo interoperability |
| **ESRI Shapefile** (`.shp` + sidecars) | ✓ | ✓ | Broad legacy GIS compatibility |
| **GeoParquet** (`.parquet`) | ✓ | ✓ | Optional `geoparquet` feature |
| **Keyhole Markup Zipped** (`.kmz`) | ✓ | ✓ | Optional `kmz` feature |
| **OpenStreetMap PBF** (`.osm.pbf`) | ✓ | — | Read-only; optional `osmpbf` feature |

## Format Properties (Detailed Comparison)

Supported formats are summarized above; this section provides deeper trade-off guidance.

| Format | Encoding | Supports attributes | Geometry richness | CRS metadata | Multi-layer / collection support | Typical size / performance profile | Best use case | `wbvector` support |
|---|---|---|---|---|---|---|---|---|
| GeoPackage (`.gpkg`) | Binary SQLite container | Yes | High (Simple Features + collections) | Yes | Yes (multiple layers/tables) | Good for larger datasets and mixed table/layer projects | Project archives and multi-layer desktop workflows | Read + Write |
| FlatGeobuf (`.fgb`) | Binary | Yes | High (Simple Features + collections) | Yes | Single layer per file | Compact and fast for sequential/stream-like workflows | High-performance interchange and large vector delivery | Read + Write |
| GeoJSON (`.geojson`) | ASCII/UTF-8 JSON text | Yes | High (Simple Features + collections) | Limited in RFC 7946 practice | FeatureCollection in one document | Verbose text; great interoperability, usually larger/slower than binary | Web APIs, debugging, and human-readable exchange | Read + Write |
| ESRI Shapefile (`.shp` + sidecars) | Mixed binary (`.shp/.shx/.dbf`) + optional text `.prj` | Yes | Medium (no native GeometryCollection) | Limited (`.prj`) | Single layer per dataset | Fast and widely supported, but constrained schema/geometry model | Maximum compatibility with legacy GIS tools | Read + Write |
| KML (`.kml`) | ASCII/UTF-8 XML text | Yes (name/description/ExtendedData) | Medium-High (including MultiGeometry) | Implicit lon/lat WGS84 | Document/Folder hierarchy | Good for visualization exchange; text/XML overhead | Google Earth-style visualization and sharing | Read + Write |
| KMZ (`.kmz`) | Binary ZIP container with KML | Yes (via embedded KML) | Medium-High (KML geometry model) | Implicit lon/lat WGS84 | KML document packaged in ZIP | Smaller transfer size than KML due to compression | Compressed KML distribution and email/web transfer | Read + Write *(optional `kmz` feature)* |
| GPX (`.gpx`) | ASCII/UTF-8 XML text | Yes (metadata + extensions) | Medium (points, routes, tracks) | Implicit WGS84 | Single GPX document with many records | Lightweight for GPS tracks; text overhead grows with point count | GPS traces, routes, and outdoor navigation workflows | Read + Write |
| GeoParquet (`.parquet`) | Binary Parquet columnar | Yes | High (WKB geometry column) | Yes (GeoParquet metadata) | Table-oriented, columnar storage | Compact analytics-friendly columnar layout; strong scan performance | Cloud/data-lake analytics and columnar interchange | Read + Write *(optional `geoparquet` feature)* |
| GML (`.gml`) | ASCII/UTF-8 XML text | Yes | High (Simple Features + collections) | Yes | Feature collections in one document | Very interoperable but often verbose/heavy XML | Standards-driven enterprise/government data exchange | Read + Write |
| MapInfo Interchange (`.mif` + `.mid`) | Mixed text (`.mif` geometry + `.mid` attributes) | Yes | Medium (common vector primitives) | Limited / driver-dependent | Single dataset pair | Plain-text interchange; moderate performance and size | Legacy MapInfo interoperability and migration | Read + Write |
| OSM PBF (`.osm.pbf`) | Binary Protocol Buffers | Yes (tag map) | Medium-High (depends on OSM primitives) | Implicit WGS84 | Planet/extract object collections | Very compact for OSM extracts; efficient for large-scale reads | Large OpenStreetMap extracts and network base data ingestion | Read-only *(optional `osmpbf` feature)* |

## Install
```toml
[dependencies]
wbvector = "0.1"

# Optional features example:
# wbvector = { version = "0.1", features = ["geoparquet", "kmz", "osmpbf"] }
```

## Quick Start

```rust
use wbvector::feature::{FieldDef, FieldType, Layer};
use wbvector::geometry::{Geometry, GeometryType};
use wbvector::{self, flatgeobuf, geojson, geopackage, gml, gpx, kml, mapinfo, shapefile, VectorFormat};

fn main() -> wbvector::Result<()> {
    let mut layer = Layer::new("cities")
        .with_geom_type(GeometryType::Point)
        .with_crs_epsg(4326);

    layer.add_field(FieldDef::new("name", FieldType::Text));
    layer.add_field(FieldDef::new("population", FieldType::Integer));

    layer.add_feature(
        Some(Geometry::point(-0.1278, 51.5074)),
        &[("name", "London".into()), ("population", 9_000_000i64.into())],
    )?;
    let _from_json = geojson::read("cities.geojson")?;
    let _from_gpkg = geopackage::read("cities.gpkg")?;
    let _from_gml = gml::read("cities.gml")?;
    let _from_gpx = gpx::read("cities.gpx")?;
    let _from_kml = kml::read("cities.kml")?;
    let _from_mif = mapinfo::read("cities.mif")?;
    let _from_shp = shapefile::read("cities")?;

    // Generic sniffed read (format auto-detected)
    let _sniffed = wbvector::read("cities.gpkg")?;

    // Format conversion (GeoJSON -> GeoPackage)
    let converted = geojson::read("cities.geojson")?;
    geopackage::write(&converted, "cities_from_geojson.gpkg")?;

    // Generic write with explicit format enum
    wbvector::write(&converted, "cities_from_geojson.fgb", VectorFormat::FlatGeobuf)?;

    Ok(())
}
```

## Format Drivers

Each format module follows a similar API style:

- `flatgeobuf::read(path)`, `flatgeobuf::write(&layer, path)`, `flatgeobuf::from_bytes(bytes)`, `flatgeobuf::to_bytes(&layer)`
- `geojson::read(path)`, `geojson::write(&layer, path)`, `geojson::parse_str(text)`, `geojson::to_string(&layer)`
    - `geojson::write` enforces RFC 7946 output coordinates (EPSG:4326 lon/lat) by auto-reprojecting when input CRS metadata is present and not already WGS 84.
- `geopackage::read(path)`, `geopackage::write(&layer, path)`, `geopackage::list_layers(path)`, `geopackage::read_layer(path, name)`, `geopackage::write_layers(&[...], path)`
- `geoparquet::read(path)`, `geoparquet::write(&layer, path)`, `geoparquet::write_with_options(&layer, path, &options)`, `GeoParquetWriteOptions` (row-group/page/batch/compression tuning; includes `for_large_files()` and `for_interactive_files()` presets) *(requires `geoparquet` feature)*
- `gml::read(path)`, `gml::write(&layer, path)`, `gml::parse_str(text)`, `gml::to_string(&layer)`
- `gpx::read(path)`, `gpx::write(&layer, path)`, `gpx::parse_str(text)`, `gpx::to_string(&layer)`
- `kml::read(path)`, `kml::write(&layer, path)`, `kml::parse_str(text)`, `kml::to_string(&layer)`
- `kmz::read(path)`, `kmz::write(&layer, path)`, `kmz::from_bytes(bytes)`, `kmz::to_bytes(&layer)` *(requires `kmz` feature)*
- `mapinfo::read(path)`, `mapinfo::write(&layer, path)`, `mapinfo::parse_pair_str(mif, mid)`, `mapinfo::to_pair_string(&layer)`
- `osmpbf::read(path)`, `osmpbf::read_with_options(path, &options)`, `osmpbf::read_from_reader(reader)`, `osmpbf::read_from_reader_with_options(reader, &options)`, `OsmPbfReadOptions::with_include_tag_keys(...)` *(requires `osmpbf` feature, read-only)*
- `shapefile::read(path)` / `shapefile::write(&layer, path)`

## Generic Sniffed I/O

`wbvector` exposes crate-level helpers that detect format from extension and lightweight file sniffing:

```rust
use wbvector::{self, VectorFormat};

fn demo() -> wbvector::Result<()> {
    let layer = wbvector::read("roads.gpkg")?; // auto-detected as GeoPackage
    wbvector::write(&layer, "roads.fgb", VectorFormat::FlatGeobuf)?;
    Ok(())
}
```

Feature-gated formats are detected only when their feature is enabled (`geoparquet`, `kmz`, `osmpbf`).

## Geometry and Attributes

`Geometry` supports:

- `Point`, `LineString`, `Polygon`
- `MultiPoint`, `MultiLineString`, `MultiPolygon`
- `GeometryCollection`

`FieldValue` supports:

- `Integer`, `Float`, `Text`, `Boolean`
- `Blob`, `Date`, `DateTime`, `Null`

## CRS Model

`Layer` stores CRS metadata in a single `crs` object (`Option<Crs>`).

- Accessors/mutators:
    - `layer.crs_epsg()`, `layer.crs_wkt()` — Query existing CRS metadata
    - `layer.set_crs_epsg(...)`, `layer.set_crs_wkt(...)` — Replace CRS with optional values (removes CRS if both are `None`)
    - `layer.assign_crs_epsg(epsg_code)` — Replace entire CRS with EPSG code only; any existing WKT is cleared
    - `layer.assign_crs_wkt(wkt_string)` — Replace entire CRS with WKT definition only; any existing EPSG is cleared

`wbvector` includes vector reprojection helpers:

- Layer methods:
    - `layer.reproject_to_epsg(dst_epsg)`
    - `layer.reproject_from_to_epsg(src_epsg, dst_epsg)`
    - `layer.reproject_to_epsg_with_options(dst_epsg, &options)`
    - `layer.reproject_from_to_epsg_with_options(src_epsg, dst_epsg, &options)`
- Module functions:
    - `reproject::layer_to_epsg(&layer, dst_epsg)`
    - `reproject::layer_from_to_epsg(&layer, src_epsg, dst_epsg)`
    - `reproject::layer_with_crs(&layer, &src_crs, &dst_crs, dst_epsg_hint)`
    - `reproject::layer_to_epsg_with_options(...)`
    - `reproject::layer_from_to_epsg_with_options(...)`
    - `reproject::layer_with_crs_options(...)`

Example:

```rust
use wbvector::feature::Layer;

fn to_web_mercator(layer: &Layer) -> wbvector::Result<Layer> {
        layer.reproject_to_epsg(3857)
}
```

Options example:

```rust
use wbvector::reproject::{
    AntimeridianPolicy,
    TopologyPolicy,
    TransformFailurePolicy,
    VectorReprojectOptions,
};

let options = VectorReprojectOptions::new()
    .with_failure_policy(TransformFailurePolicy::SetNullGeometry)
    .with_antimeridian_policy(AntimeridianPolicy::SplitAt180)
    .with_max_segment_length(0.25)
    .with_topology_policy(TopologyPolicy::ValidateAndFixOrientation);
```

Behavior notes:

- All geometry variants are transformed, including `GeometryCollection`.
- Attributes and schema are preserved.
- Output CRS metadata is updated to destination EPSG/WKT when an EPSG destination is used.
- `reproject_to_epsg` accepts source CRS metadata from either `layer.crs.epsg` or `layer.crs.wkt`.
- EPSG extraction from WKT/SRS metadata uses `wbprojection` parsing helpers with adaptive identification for authority-missing WKT.
- WKT/SRS EPSG detection is authority-first (`AUTHORITY` / `ID` / `EPSG:` / URN / HTTP CRS references), then adaptive best-match over currently supported EPSG codes.
- `wbvector` ingestion paths currently use lenient adaptive matching so legacy CRS metadata remains interoperable; strict ambiguity-reject behavior is available in `wbprojection` policy APIs when callers need fail-fast semantics.
- `TransformFailurePolicy` controls whether feature-level transform errors abort, null geometry, or skip features.
- `AntimeridianPolicy::NormalizeLon180` normalizes longitudes for EPSG:4326 destination outputs.
- `AntimeridianPolicy::SplitAt180` splits line geometries crossing ±180 into multipart lines for EPSG:4326 outputs.
- `with_max_segment_length(...)` densifies line/ring segments before reprojection (in source CRS units).
- `SplitAt180` splits crossing polygons into multipart polygon output and assigns hole parts to split polygon parts.
- `with_topology_policy(...)` enables polygon topology checks (`Validate`) or checks plus ring-orientation normalization (`ValidateAndFixOrientation`).

GML CRS metadata notes:

- GML writing emits canonical root `srsName` values in URI form when EPSG metadata is known.
- GML writing preserves CRS WKT metadata at the collection level when available.
- GML reading considers both feature-level and root-level CRS metadata fields when reconstructing CRS hints.

## Examples

Run examples from this crate root:

```bash
cargo run --example flatgeobuf_io
cargo run --example geojson_io
cargo run --example geopackage_io
cargo run --features geoparquet --example geoparquet_io -- path/to/input.parquet [path/to/output.parquet]
cargo run --example gml_io
cargo run --example gpx_io
cargo run --example kml_io
cargo run --features kmz --example kmz_io
cargo run --example mapinfo_io
cargo run --features osmpbf --example osmpbf_io -- path/to/extract.osm.pbf [--highways-only] [--named-only] [--polygons-only] [--tag-keys=name,highway]
cargo run --example reproject_io
cargo run --example shapefile_io
cargo run --example convert
```

The `convert` example round-trips default formats, and also includes GeoParquet when run with `--features geoparquet`.

Example source files:

- [examples/flatgeobuf_io.rs](examples/flatgeobuf_io.rs)
- [examples/geojson_io.rs](examples/geojson_io.rs)
- [examples/geopackage_io.rs](examples/geopackage_io.rs)
- [examples/geoparquet_io.rs](examples/geoparquet_io.rs)
- [examples/gml_io.rs](examples/gml_io.rs)
- [examples/gpx_io.rs](examples/gpx_io.rs)
- [examples/kml_io.rs](examples/kml_io.rs)
- [examples/kmz_io.rs](examples/kmz_io.rs)
- [examples/mapinfo_io.rs](examples/mapinfo_io.rs)
- [examples/osmpbf_io.rs](examples/osmpbf_io.rs)
- [examples/reproject_io.rs](examples/reproject_io.rs)
- [examples/shapefile_io.rs](examples/shapefile_io.rs)
- [examples/convert.rs](examples/convert.rs)

## OSM Examples

Read all ways from an OSM PBF extract:

```rust
#[cfg(feature = "osmpbf")]
{
    let layer = wbvector::osmpbf::read("extract.osm.pbf")?;
    println!("{} ways", layer.len());
}
```

Read only named highways and keep a compact `osm_tags` payload:

```rust
#[cfg(feature = "osmpbf")]
{
    use wbvector::osmpbf::{self, OsmPbfReadOptions};

    let options = OsmPbfReadOptions::new()
        .with_highways_only(true)
        .with_named_ways_only(true)
        .with_include_tag_keys(["name", "highway", "maxspeed"]);

    let layer = osmpbf::read_with_options("extract.osm.pbf", &options)?;
    println!("{} named highways", layer.len());
}
```

Equivalent CLI example:

```bash
cargo run --features osmpbf --example osmpbf_io -- \
  path/to/extract.osm.pbf --highways-only --named-only --tag-keys=name,highway,maxspeed
```

## Architecture

```
wbvector/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs               ← public API, format dispatch, sniffed I/O
│   ├── error.rs             ← VectorError, Result
│   ├── feature.rs           ← Layer, Feature, Schema, FieldDef, FieldValue, FieldType, Crs
│   ├── geometry.rs          ← Coord, Ring, Geometry, GeometryType, BBox
│   ├── reproject.rs         ← vector reprojection API backed by wbprojection
│   ├── flatgeobuf/          ← FlatGeobuf (.fgb)
│   ├── geojson/             ← GeoJSON (.geojson)
│   ├── geopackage/          ← GeoPackage (.gpkg) + internal pure-Rust SQLite engine
│   ├── gml/                 ← Geography Markup Language (.gml)
│   ├── gpx/                 ← GPS Exchange Format (.gpx)
│   ├── kml/                 ← Keyhole Markup Language (.kml)
│   ├── mapinfo/             ← MapInfo Interchange (.mif/.mid)
│   ├── shapefile/           ← ESRI Shapefile (.shp + sidecars)
│   ├── geoparquet/          ← GeoParquet (.parquet) — optional `geoparquet` feature
│   ├── kmz/                 ← KMZ (.kmz) — optional `kmz` feature
│   └── osmpbf/              ← OpenStreetMap PBF (.osm.pbf) — optional `osmpbf` feature, read-only
├── examples/                ← runnable examples for each format driver
└── data/                    ← test fixtures (excluded from publish)
```

### Design principles

- **Single feature model**: `Layer`/`Feature`/`Geometry`/`FieldValue` are the only in-memory representation; all format drivers bridge to and from this model.
- **Format independence**: adding a new format requires only a new module implementing `read` and `write`; the core model does not change.
- **Pure Rust core**: the GeoPackage SQLite engine is implemented from scratch in Rust; no `rusqlite` or native `libsqlite3` linkage is required.
- **Optional features for heavy dependencies**: GeoParquet (`parquet`), KMZ (`zip`), and OSM PBF (`osmpbfreader`) are feature-gated.

## Performance Notes

- **FlatGeobuf** is the fastest format for large sequential reads/writes; its flat binary layout with spatial index enables efficient streaming.
- **GeoPackage** is well-suited for multi-layer projects and moderately large datasets, but has SQLite I/O overhead relative to binary formats.
- **GeoJSON** is the slowest large-dataset format due to text parsing and serialization overhead; prefer binary formats for data pipelines.
- **GeoParquet** offers the best performance for columnar analytics workloads (large scans, attribute filters); write tuning is available via `GeoParquetWriteOptions`.
- **Shapefile** has a fixed-size attribute file that scales predictably but is constrained by the format's geometry and schema model.
- Reprojection (`layer.reproject_to_epsg`) transforms all coordinates in-memory; performance scales linearly with feature and vertex count.
- Format auto-detection (sniffed I/O) adds only lightweight header inspection overhead, not a full parse.

## Known Limitations

- `wbvector` is an I/O and feature model layer; spatial analysis operations (overlay, buffer, simplify) belong in `wbtopology`.
- Shapefile has no native `GeometryCollection` type; writing a `GeometryCollection` layer encodes affected features as null geometry.
- Shapefile field names are limited to 10 characters (dBASE III constraint); longer names are truncated on write.
- OSM PBF support is read-only; writing OSM-format data is not supported.
- GeoPackage multi-layer support uses `geopackage::write_layers`; the single-layer convenience API writes and reads a single default layer.
- GeoParquet read compatibility targets GeoParquet 1.0 canonical patterns; some producer-specific extensions or alternate geometry column conventions may require workarounds.
- WKT-based CRS inference uses `wbprojection` adaptive matching; authority-marker-free WKT strings may produce ambiguous EPSG identification.
- In-memory geometry is flat (XY or XYZ); M values are not preserved through round-trips.

## Development

Common local checks from the crate root:

```bash
cargo test
cargo run --example convert
cargo doc --no-deps --open
```

## Notes and Current Behavior

- Shapefile geometry support follows native Shapefile constraints.
- Writing a `GeometryCollection` to Shapefile is encoded as null geometry (Shapefile has no native `GeometryCollection` type).
- Layer CRS can be attached via `with_crs_epsg(...)`, `with_crs_wkt(...)`, or `with_crs(...)`.

### GeoParquet Status

- Date/DateTime/Json schema fidelity is preserved through GeoParquet roundtrips.
- Metadata/type inference has been hardened for external producer patterns.
- Large-file and interactive write tuning is available through `GeoParquetWriteOptions` presets and overrides.
- Interoperability tests cover edge cases such as non-default primary geometry columns and alternate CRS metadata forms.

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
