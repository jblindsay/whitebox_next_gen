# wbprojection

A map projection library for Rust, inspired by [PROJ](https://proj.org/), and intended to serve as the projection engine for [Whitebox](https://www.whiteboxgeo.com).

`wbprojection` provides **forward** and **inverse** transformations between geographic coordinates (longitude/latitude) and projected Cartesian coordinates (easting/northing) for a wide range of map projections. It also supports datum transformations via Helmert parameters, and includes a built-in registry of EPSG codes.

## Table of Contents

- [Mission](#mission)
- [The Whitebox Project](#the-whitebox-project)
- [Is wbprojection Only for Whitebox?](#is-wbprojection-only-for-whitebox)
- [What wbprojection Is Not](#what-wbprojection-is-not)
- [Features](#features)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Using EPSG Codes](#using-epsg-codes)
- [Supported Datums](#supported-datums)
- [Supported Ellipsoids](#supported-ellipsoids)
- [Manual Projection Construction](#manual-projection-construction)
- [CRS-to-CRS Transformation](#crs-to-crs-transformation)
- [Batch Transformation](#batch-transformation)
- [Grid-Shift Datum Workflow (NTv2 / NADCON)](#grid-shift-datum-workflow-ntv2--nadcon)
- [Supported Projections](#supported-projections)
- [Error Handling](#error-handling)
- [Compilation Features](#compilation-features)
- [Performance](#performance)
- [Architecture](#architecture)
- [Known Limitations](#known-limitations)
- [License](#license)

## Mission

- Provide robust CRS and map projection support for Whitebox applications and workflows.
- Cover the most commonly needed EPSG coordinate reference systems in a built-in, dependency-free catalog.
- Keep all projection and datum math in Rust with minimal external dependencies.
- Prioritize standards compliance, PROJ-inspired API design, and predictable behavior.

## The Whitebox Project

Whitebox is a collection of related open-source geospatial data analysis software. The Whitebox project began in 2009 at the [University of Guelph](https://geg.uoguelph.ca), Canada, developed by [Dr. John Lindsay](https://jblindsay.github.io/ghrg/index.html) a professor of geomatics. Whitebox has long served as Dr. Lindsay's platform for disseminating the output of his geomatics-based research and has developed an extensive worldwide user base. In 2021 Dr. Lindsay and Anthony Francioni founded [Whitebox Geospatial Inc.](https://www.whiteboxgeo.com) in order to ensure the sustainable and ongoing development of this open-source geospatial project. We are currently working on the next iteration of the Whitebox software, **Whitebox Next Gen**. This crate is part of that larger effort.

## Is wbprojection Only for Whitebox?

No. `wbprojection` is developed primarily to support Whitebox, but it is not restricted to Whitebox projects.

- **Whitebox-first**: API and roadmap decisions prioritize Whitebox CRS needs.
- **General-purpose**: the crate is usable as a standalone map projection and CRS library in other Rust geospatial applications.
- **Interop-focused**: standards-compliant EPSG registry, WKT export/import, and datum transformation support make it suitable for broader tooling and data pipelines.

## What wbprojection Is Not

`wbprojection` is a CRS and map projection engine. It is **not** intended to be a complete geospatial processing framework.

- Not a replacement for PROJ (coverage is broad but not exhaustive; some exotic projection methods or datum models may not be supported).
- Not a complete WKT/GML parser (`from_wkt` handles common WKT patterns but is not a full OGC/EPSG WKT standards engine).
- Not a pipeline engine for arbitrary coordinate transformations beyond the CRS-to-CRS model.
- Not a datum grid provider (NTv2 / NADCON grid files must be supplied externally).

---

## Features

- **94 projection types** with full forward + inverse support
- **EPSG registry** — instantiate any supported CRS from a numeric code (`Crs::from_epsg(32632)`)
- **Ellipsoidal** formulations (not just spherical approximations)
- **Standard ellipsoid catalog** with name/EPSG ellipsoid-code lookup helpers
- **UTM convenience constructor** for all 60 zones, N and S
- **Datum transformations** across 28 built-in datums (Helmert + grid-shift capable)
- **Grid-shift datum support** with NTv2 (`.gsb`) and NADCON ASCII pair loaders
- **CRS-to-CRS pipelines**: transform directly between any two supported coordinate reference systems
- **Pure Rust**, no C dependencies
- **Zero required dependencies** (only `thiserror` for ergonomic error handling)
- Optional `serde` feature for serialization

---

## Installation

Crates.io dependency:

```toml
[dependencies]
wbprojection = "0.1"
```

Enable `serde` support when you need to serialize CRS or projection-related types:

```toml
[dependencies]
wbprojection = { version = "0.1", features = ["serde"] }
```

Local workspace/path dependency:

```toml
[dependencies]
wbprojection = { path = "../wbprojection" }
```

## Quick Start

## Using EPSG Codes

The easiest way to create a projection is by EPSG code. The built-in registry currently covers **5591 EPSG codes** (**5594 total CRS/projection codes**, including ESRI 54008, 54009, 54030) and requires no external database or network access.

```rust
use wbprojection::Crs;

// WGS84 / UTM zone 32N
let crs = Crs::from_epsg(32632)?;
let (easting, northing) = crs.forward(9.18, 48.78)?;
// → ~513298 m, ~5404253 m  (Stuttgart, Germany)
let (lon, lat) = crs.inverse(easting, northing)?;

// Web Mercator (used by Google Maps, OpenStreetMap)
let crs = Crs::from_epsg(3857)?;
let (x, y) = crs.forward(13.4, 52.5)?;  // Berlin

// British National Grid
let crs = Crs::from_epsg(27700)?;
let (e, n) = crs.forward(-0.1276, 51.5074)?;  // London

// WGS84 geographic (degrees in, metres out)
let crs = Crs::from_epsg(4326)?;

// French Lambert-93
let crs = Crs::from_epsg(2154)?;
let (x, y) = crs.forward(2.35, 48.85)?;  // Paris

// Netherlands RD New
let crs = Crs::from_epsg(28992)?;

// NAD83 / UTM zone 18N
let crs = Crs::from_epsg(26918)?;

// NAD27 / UTM zone 15N
let crs = Crs::from_epsg(26715)?;
```

The top-level `from_epsg` function is also available as a shorthand:

```rust
use wbprojection::from_epsg;

let crs = from_epsg(32755)?;  // WGS84 / UTM zone 55S (eastern Australia)
```

### Querying the registry

```rust
use wbprojection::epsg::{epsg_info, known_epsg_codes};

// Look up metadata without constructing the CRS
if let Some(info) = epsg_info(27700) {
    println!("{}: {} ({})", info.code, info.name, info.area_of_use);
    // → 27700: OSGB 1936 / British National Grid (UK)
}

// List every supported code
let codes = known_epsg_codes();
println!("{} EPSG codes supported", codes.len());
```

### ESRI WKT export

Generate an ESRI-formatted WKT string directly from an EPSG code:

```rust
use wbprojection::to_esri_wkt;

let wkt = to_esri_wkt(32632)?;
```

### OGC WKT export

Generate an OGC-formatted WKT string from an EPSG code:

```rust
use wbprojection::to_ogc_wkt;

let wkt = to_ogc_wkt(32632)?;
println!("{wkt}");
```

Swiss LV03/LV95 exports now use the explicit `Oblique_Stereographic` method name to match their EPSG method semantics rather than the generic stereographic label.

### Limited WKT import

`wbprojection` can now import WKT and SRS references when they embed an EPSG identifier:

```rust
use wbprojection::from_wkt;

let crs = from_wkt("GEOGCRS[\"WGS 84\",ID[\"EPSG\",4326]]")?;
assert!(crs.name.contains("WGS"));
```

When no EPSG identifier is embedded, `from_wkt` now falls back to an internal parser for common WKT1 and WKT2 `GEOGCS`/`GEOGCRS` and `PROJCS`/`PROJCRS` definitions. The importer is designed around the projection methods already supported by `ProjectionKind`, so it can ingest this crate's own WKT exports and many standard projected CRS definitions.

Compound CRS WKT can be parsed with `compound_from_wkt`:

```rust
use wbprojection::compound_from_wkt;

let compound = compound_from_wkt(
    "COMPOUNDCRS[\"Example\",GEOGCRS[\"WGS 84\",DATUM[\"World Geodetic System 1984\",ELLIPSOID[\"WGS 84\",6378137,298.257223563]],PRIMEM[\"Greenwich\",0],ANGLEUNIT[\"degree\",0.0174532925199433]],VERTCRS[\"EGM96 height\",VDATUM[\"EGM96\"],CS[vertical,1],AXIS[\"gravity-related height\",up],LENGTHUNIT[\"metre\",1]]]"
)?;
println!("{}", compound.name);
```

Projected-unit factors in WKT (e.g., US survey foot) are respected for linear parameters such as false easting/northing during import.

Compound parsing is strict by design: `compound_from_wkt` expects exactly one horizontal component (`PROJCRS`/`PROJCS` or `GEOGCRS`/`GEOGCS`) and exactly one vertical component (`VERTCRS`/`VERT_CS`).
Nested compound trees are flattened recursively when they still resolve to exactly one horizontal and one vertical component; ambiguous trees with multiple horizontals or multiple verticals are rejected.

It is still not a complete standards-level WKT engine. Unsupported method names, uncommon unit models, and CRS forms outside the current projection surface will still return an `UnsupportedProjection` error.

### Adaptive EPSG identification for authority-missing WKT

For WKT strings that do not include explicit EPSG authority markers, `wbprojection`
now supports adaptive EPSG identification over all currently supported codes in
the built-in registry.

- `identify_epsg_from_wkt(...)` provides best-match identification in lenient mode.
- `identify_epsg_from_wkt_with_policy(...)` adds explicit match policy control.
- `identify_epsg_from_wkt_report(...)` returns scored top-candidate diagnostics.
- Equivalent CRS-based APIs are available:
    - `identify_epsg_from_crs(...)`
    - `identify_epsg_from_crs_with_policy(...)`
    - `identify_epsg_from_crs_report(...)`

```rust
use wbprojection::{
        EpsgIdentifyPolicy,
        identify_epsg_from_wkt_with_policy,
        identify_epsg_from_wkt_report,
};

let wkt = "PROJCS[\"NAD83_CSRS_UTM_zone_17N\",GEOGCS[\"GCS_NAD83(CSRS)\",DATUM[\"D_North_American_1983_CSRS\",SPHEROID[\"GRS_1980\",6378137,298.257222101]],PRIMEM[\"Greenwich\",0],UNIT[\"Degree\",0.017453292519943295]],PROJECTION[\"Transverse_Mercator\"],PARAMETER[\"latitude_of_origin\",0],PARAMETER[\"central_meridian\",-81],PARAMETER[\"scale_factor\",0.9996],PARAMETER[\"false_easting\",500000],PARAMETER[\"false_northing\",0],UNIT[\"Meter\",1]]";

let epsg_lenient = identify_epsg_from_wkt_with_policy(wkt, EpsgIdentifyPolicy::Lenient);
let epsg_strict = identify_epsg_from_wkt_with_policy(wkt, EpsgIdentifyPolicy::Strict);
let report = identify_epsg_from_wkt_report(wkt, EpsgIdentifyPolicy::Lenient).unwrap();

assert_eq!(epsg_lenient, Some(2958));
assert_eq!(epsg_strict, None); // strict mode rejects ambiguous near-ties
assert!(report.ambiguous);
```

Policy behavior:

- `Lenient`: resolve best candidate when confidence threshold is met.
- `Strict`: require confident and unambiguous top candidate.

The candidate search is built from `known_epsg_codes()`, so identification
coverage automatically expands as additional EPSG entries are added to the
registry.

### Calibration and verification tools

This crate includes maintainer-only tools used to calibrate and verify WKT->EPSG
identification behavior. These sources now live under `dev/examples/internal/`
and are intentionally excluded from the published crate package:

- `epsg_identify_report` (single sample diagnostics)
- `epsg_identify_report_batch` (manifest-driven CSV diagnostics)
- `epsg_identify_verify_manifests` (CI-style pass/fail verification)
- `epsg_identify_manifest_template` (new profile manifest bootstrap)

These are development utilities for repository maintainers rather than part of
the public crate-facing example surface.

### GeoTIFF projection info

Generate GeoTIFF GeoKey information from an EPSG code:

```rust
use wbprojection::to_geotiff_info;

let info = to_geotiff_info(32632)?;
println!("model_type: {}", info.model_type);
println!("projected_cs_type: {:?}", info.projected_cs_type);
```

### Handling unsupported codes

If a code isn't in the registry, `from_epsg` returns an error rather than panicking:

```rust
match Crs::from_epsg(99999) {
    Err(ProjectionError::UnsupportedProjection(msg)) => {
        eprintln!("Not supported: {msg}");
        // Falls back to manual ProjectionParams construction
    }
    Ok(crs) => { /* use it */ }
    Err(e) => eprintln!("Other error: {e}"),
}
```

For opt-in fallback behavior, use policy-based resolution:

```rust
use wbprojection::{Crs, EpsgResolutionPolicy};

// Unknown code resolves to EPSG:4326
let crs = Crs::from_epsg_with_policy(
    99999,
    EpsgResolutionPolicy::FallbackToWgs84,
)?;

// Or choose an explicit fallback EPSG code
let web = Crs::from_epsg_with_policy(
    99999,
    EpsgResolutionPolicy::FallbackToEpsg(3857),
)?;

assert!(crs.name.contains("WGS"));
assert!(web.name.contains("Mercator"));
```

Available policies:

- `Strict` (default behavior)
- `FallbackToEpsg(code)`
- `FallbackToWgs84`
- `FallbackToWebMercator`

### 3D CRS Workflows (Geocentric + Vertical)

The library now supports 3D workflows for geocentric CRS and minimal vertical CRS handling.

Use `transform_to_3d` for strict 3D CRS transformations:

- Geographic/Projected <-> Geocentric: supported
- Vertical <-> Vertical: passthrough
- Vertical <-> Geographic/Projected: rejected (strict mode)
- Vertical <-> Geocentric: rejected

```rust
use wbprojection::Crs;

let geog = Crs::from_epsg(7843)?;      // GDA2020 geographic
let geoc = Crs::from_epsg(7842)?;      // GDA2020 geocentric (ECEF)

let (x, y, z) = geog.transform_to_3d(147.0, -35.0, 120.0, &geoc)?;
let (lon, lat, h) = geoc.transform_to_3d(x, y, z, &geog)?;
```

Use `transform_to_3d_preserve_horizontal` for explicit mixed vertical workflows where
horizontal context should be preserved as-is:

- Vertical <-> Geographic/Projected: returns `(x, y, z)` unchanged
- Vertical <-> Vertical: returns `(x, y, z)` unchanged
- Vertical <-> Geocentric: rejected

```rust
use wbprojection::Crs;

let vertical = Crs::from_epsg(7841)?;  // GDA2020 height
let utm = Crs::from_epsg(7846)?;       // GDA2020 / MGA zone 46

// Explicitly preserve horizontal coordinates while carrying height through.
let (x2, y2, z2) = utm.transform_to_3d_preserve_horizontal(
    500_000.0,
    6_120_000.0,
    42.0,
    &vertical,
)?;

assert_eq!((x2, y2, z2), (500_000.0, 6_120_000.0, 42.0));
```

This preserve-horizontal API is intentionally explicit so strict `transform_to_3d` remains
safe by default for ambiguous vertical/horizontal combinations.

If you have external vertical model offsets (for example, geoid undulation values), use:

- `transform_to_3d_preserve_horizontal_with_vertical_offsets(...)`
- `transform_to_3d_preserve_horizontal_with_vertical_offsets_and_policy(...)`
- `transform_to_3d_preserve_horizontal_with_provider(...)`
- `transform_to_3d_preserve_horizontal_with_provider_and_policy(...)`
- `ConstantVerticalOffsetProvider` for fixed-offset convenience
- `GridVerticalOffsetProvider` + `VerticalOffsetGrid` for native bilinear grid sampling

It applies:

- `h_ellps = z + source_to_ellipsoidal_m`
- `z_out = h_ellps - target_to_ellipsoidal_m`

This lets you keep horizontal coordinates unchanged while converting vertical reference
surfaces using offsets computed outside `wbprojection`.

Example provider-based workflow:

```rust
use wbprojection::{Crs, Result};

let vertical = Crs::from_epsg(7841)?;
let utm = Crs::from_epsg(7846)?;

let provider = |x: f64, y: f64, _src: &Crs, _dst: &Crs| -> Result<(f64, f64)> {
    // Replace with model lookup based on x/y and CRS context.
    let source_to_ellipsoidal = 30.0 + x * 0.0;
    let target_to_ellipsoidal = 10.0 + y * 0.0;
    Ok((source_to_ellipsoidal, target_to_ellipsoidal))
};

let (_x2, _y2, _z2) = utm.transform_to_3d_preserve_horizontal_with_provider(
    500_000.0,
    6_120_000.0,
    100.0,
    &vertical,
    &provider,
)?;
```

Example native grid-based workflow:

```rust
use wbprojection::{
    Crs,
    GridVerticalOffsetProvider,
    VerticalOffsetGrid,
    register_vertical_offset_grid,
};

register_vertical_offset_grid(VerticalOffsetGrid::new(
    "src_geoid",
    140.0,
    -40.0,
    10.0,
    10.0,
    2,
    2,
    vec![30.0, 30.0, 30.0, 30.0],
)?)?;

register_vertical_offset_grid(VerticalOffsetGrid::new(
    "dst_geoid",
    140.0,
    -40.0,
    10.0,
    10.0,
    2,
    2,
    vec![10.0, 10.0, 10.0, 10.0],
)?)?;

let geog = Crs::from_epsg(7843)?;
let vertical = Crs::from_epsg(7841)?;
let provider = GridVerticalOffsetProvider::new("src_geoid", "dst_geoid");

let (_x2, _y2, z2) = geog.transform_to_3d_preserve_horizontal_with_provider(
    147.0,
    -35.0,
    100.0,
    &vertical,
    &provider,
)?;

assert!((z2 - 120.0).abs() < 1e-9);
```

To load vertical offset grids from files, use the built-in loaders:

**ISG format** (International Service for the Geoid — EGM2008, EGM96, regional models):

```rust
use std::{fs::File, io::BufReader};
use wbprojection::{load_vertical_grid_from_isg, register_vertical_offset_grid};

let f = File::open("egm2008.isg")?;
let grid = load_vertical_grid_from_isg(BufReader::new(f), "egm2008")?;
register_vertical_offset_grid(grid)?;
```

**Simple header + data format** (manual or custom grids):

```rust
use std::{fs::File, io::BufReader};
use wbprojection::{load_vertical_grid_from_simple_header_grid, register_vertical_offset_grid};

let f = File::open("my_geoid.grid")?;
let grid = load_vertical_grid_from_simple_header_grid(BufReader::new(f), "my_geoid")?;
register_vertical_offset_grid(grid)?;
```

**GTX binary format** (PROJ/NOAA geoid grids):

```rust
use std::{fs::File, io::BufReader};
use wbprojection::{load_vertical_grid_from_gtx, register_vertical_offset_grid};

let f = File::open("geoid.gtx")?;
let grid = load_vertical_grid_from_gtx(BufReader::new(f), "geoid")?;
register_vertical_offset_grid(grid)?;
```

Note: for GeoTIFF-based vertical grids in the Whitebox stack, use `wbraster`
to read raster values and build/register a `VerticalOffsetGrid` in `wbprojection`.

Simple format layout (S-to-N rows, W-to-E columns):

```text
# my geoid grid
lon_min = 140.0
lat_min = -40.0
lon_step = 0.5
lat_step = 0.5
width = 3
height = 3
28.0 28.1 28.2
28.3 28.4 28.5
28.6 28.7 28.8
```

For legacy/vendor codes, use the built-in explicit alias catalog:

```rust
use wbprojection::{
    Crs,
    EpsgResolutionPolicy,
    epsg_alias_catalog,
    resolve_epsg_with_catalog,
};

// Inspect built-in alias mappings (examples include 900913, 102100 -> 3857)
let aliases = epsg_alias_catalog();
assert!(!aliases.is_empty());

// Resolve code using catalog first, then policy fallback
let resolved = resolve_epsg_with_catalog(900913, EpsgResolutionPolicy::Strict)?;
assert_eq!(resolved.resolved_code, 3857);
assert!(resolved.used_alias_catalog);

// Construct CRS directly from alias-enabled path
let web = Crs::from_epsg_with_catalog(102100, EpsgResolutionPolicy::Strict)?;
assert!(web.name.contains("Mercator"));
```

You can also register organization-specific aliases at runtime:

```rust
use wbprojection::{
    Crs,
    EpsgResolutionPolicy,
    clear_runtime_epsg_aliases,
    register_epsg_alias,
    unregister_epsg_alias,
};

clear_runtime_epsg_aliases();
register_epsg_alias(9100001, 4326)?; // local/vendor code -> WGS84

let crs = Crs::from_epsg_with_catalog(9100001, EpsgResolutionPolicy::Strict)?;
assert!(crs.name.contains("WGS"));

let _ = unregister_epsg_alias(9100001);
clear_runtime_epsg_aliases();
```

Quick API reference:

| API | Purpose | Notes |
|---|---|---|
| `Crs::from_epsg(code)` / `from_epsg(code)` | Strict lookup from built-in EPSG registry | Errors if unsupported |
| `Crs::from_epsg_with_policy(code, policy)` / `from_epsg_with_policy(...)` | Strict lookup with optional fallback policy | Does not use alias catalogs |
| `Crs::from_epsg_with_catalog(code, policy)` / `from_epsg_with_catalog(...)` | Catalog-aware lookup | Order: exact -> runtime alias -> built-in alias -> policy fallback |
| `resolve_epsg_with_policy(code, policy)` | Resolve code only (no CRS construction) | Returns `EpsgResolution` metadata |
| `resolve_epsg_with_catalog(code, policy)` | Resolve with alias catalogs | Indicates `used_alias_catalog` / `used_fallback` |
| `epsg_alias_catalog()` | Inspect built-in alias mappings | Static entries (legacy/vendor codes) |
| `register_epsg_alias(src, dst)` | Register runtime alias mapping | `dst` must be supported EPSG |
| `unregister_epsg_alias(src)` | Remove one runtime alias | Returns previous target if present |
| `runtime_epsg_aliases()` | List runtime aliases | Sorted `(source, target)` pairs |
| `clear_runtime_epsg_aliases()` | Clear runtime alias registry | Useful for test setup/teardown |

### EPSG codes covered

Currently supports **5591 EPSG codes** and **5594 total CRS/projection codes** (including ESRI 54008, 54009, 54030).

| Range / Code | Description |
|---|---|
| 4326, 4269, 4267, 4258, 4230, 4617 | Geographic 2D — WGS84, NAD83, NAD27, ETRS89, ED50, NAD83(CSRS) |
| 4283, 4148, 4152, 4167, 4189, 4619 | Geographic 2D — GDA94, Hartebeesthoek94, NAD83(HARN), NZGD2000, RGAF09, SIRGAS95 |
| 4681, 4483, 4624, 4284, 4322, 6318, 4615 | Geographic 2D — REGVEN, Mexico ITRF92, RGFG95, Pulkovo 1942, WGS 72, NAD83(2011), REGCAN95 |
| 4001–4016, 4018–4025, 4027–4029, 4031–4036, 4044–4047, 4052–4055 | Geographic 2D — additional legacy ellipsoid/datum definitions (Airy, Bessel, Clarke, Everest, GRS, Helmert, International, Hughes, authalic spheres) |
| 4026, 4037, 4038, 4048–4051, 4056–4063 | Additional projected systems — MOLDREF99 TM, WGS84 TMzn 35N/36N, RGRDC 2005 Congo TM zones and UTM 33S–35S |
| 3857 | Web Mercator (Google Maps, OpenStreetMap) |
| 3395, 4087, 32662 | World Mercator, World Equidistant Cylindrical, Plate Carrée |
| 3400, 3401, 3402, 3403, 3405, 3406 | Alberta 10-TM variants, VN-2000 UTM zones 48N and 49N |
| 3833, 3834, 3835, 3836, 3837, 3838, 3839, 3840, 3841, 3845, 3846, 3847, 3848, 3849, 3850, 3986, 3987, 3988, 3989, 3991, 3992, 3994, 3997 | Pulkovo and Katanga Gauss-Kruger zones, SWEREF99 RT90 emulation systems, Puerto Rico / St. Croix systems, Mercator 41, Dubai Local TM |
| 32601–32660 | WGS84 / UTM northern hemisphere (all 60 zones) |
| 32701–32760 | WGS84 / UTM southern hemisphere (all 60 zones) |
| 32201–32260, 32301–32360 | WGS72 / UTM north and south hemispheres (all 120 zones) |
| 32401–32460, 32501–32560 | WGS72BE / UTM north and south hemispheres (all 120 zones) |
| 2494–2758 | Pulkovo 1942/1995 Gauss-Kruger CM and 3-degree families (+ Samboja, LKS 1994, Tete outliers) |
| 2463–2491, 20004–20032, 28404–28432, 3329–3335, 4417, 4434, 5631, 5663–5665, 5670–5675 | Pulkovo 1995/1942 Gauss-Kruger 6-degree CM and zone families plus additional adjusted 1958/1983 GK variants |
| 2391–2396, 2400–2442 | Additional projected parity families: Finland GK zones, South Yemen GK zones, RT90 variant, and Beijing 1954 3-degree GK zone/CM systems |
| 2867–2888, 2891–2954 | Additional projected parity families: NAD83(HARN) StatePlane foot/intl-foot systems and mixed regional TM/GK/Mercator/CS63/NAD83(CSRS) MTM systems |
| 4120–4147, 4149–4151, 4153–4166, 4168–4176, 4178–4185 | Additional geographic parity block: legacy geographic datums from Greek/GGRS/KKJ through Madeira and related regional systems |
| 2172–2175 | Pulkovo 1942 Adj 1958 Poland zones II–V (double stereographic and TM) |
| 2188–2192, 2195–2198 | Azores/Madeira UTM systems, ED50 France EuroLambert, NAD83(HARN) UTM 2S, and ETRS89 Kp2000 variants |
| 2205–2213 | NAD83 Kentucky North, ED50 3-degree GK zones 9–15, and ETRS89 TM 30 NE |
| 2200–2204, 2214–2220, 2222–2226, 2228 | ATS77/REGVEN/NAD27/StatePlane and related UTM/LCC/TM families |
| 3580–3751 | NAD83/NSRS2007 StatePlane families, related UTM/HARN systems, and Reunion TM |
| 26901–26923 | NAD83 / UTM north zones 1–23 |
| 6328–6348 | NAD83(2011) / UTM north zones 59, 60, 1–19 |
| 26701–26722 | NAD27 / UTM north zones 1–22 |
| 2955–2962, 3154–3160, 3761, 9709, 9713, 22207–22222, 22307–22322, 22407–22422, 22607–22622, 22707–22722, 22807–22822 | NAD83(CSRS) and NAD83(CSRS) realization UTM north zones (active v1 and v2/v3/v4/v6/v7/v8 families) |
| 25801–25860 | ETRS89 / UTM north zones 1–60 |
| 23001–23060 | ED50 / UTM north zones 1–60 |
| 31965–31985, 6210, 6211, 5396 | SIRGAS 2000 / UTM zones 11N–24N and 17S–26S |
| 5463, 29168–29172, 29187–29195 | SAD69 / UTM zones 17N–22N and 17S–25S (active codes) |
| 24817–24821, 24877–24882 | PSAD56 / UTM zones 17N–21N and 17S–22S |
| 3034, 3035 | ETRS89 LCC Europe, LAEA Europe |
| 3031, 3032, 3413, 3976, 3995, 3996 | Antarctic/Arctic Polar Stereographic variants |
| 2163, 3408, 3409, 3410, 3571, 3572, 3573, 3574, 3575, 3576 | US National Atlas Equal Area, NSIDC EASE-Grid variants, North Pole LAEA regional variants |
| 3832 | WGS 84 / PDC Mercator |
| 6931, 6932, 6933 | NSIDC EASE-Grid 2.0 North/South/Global |
| 8857 | WGS 84 / Equal Earth Greenwich |
| 54008, 54009, 54030 | ESRI World Sinusoidal, Mollweide, Robinson |
| 6707, 6708, 6709 | RDN2008 / UTM zones 32N, 33N, 34N |
| 6732, 6733, 6734, 6735, 6736, 6737, 6738 | GDA94 / MGA zones 41, 42, 43, 44, 46, 47, 59 |
| 7849–7856 | GDA2020 / MGA zones 49–56 |
| 6784, 6786, 6788, 6790, 6800, 6802, 6812, 6814, 6816, 6818, 6820, 6822, 6824, 6826, 6828, 6830, 6832, 6834, 6836, 6838, 6844, 6846, 6848, 6850, 6856, 6858, 6860, 6862 | NAD83(CORS96)/NAD83(2011) Oregon local TM zones (metre) |
| 6870, 6875, 6876 | Albania TM 2010 and RDN2008 Italy TM systems |
| 6915, 6927 | South East Island 1943 / UTM zone 40N, SVY21 / Singapore TM |
| 6956, 6957 | VN-2000 / TM-3 zones 481 and 482 |
| 7257–7355 | NAD83(2011) / Indiana InGCS county systems (metre variants; ftUS variants included where defined) |
| 2443, 2444, 2445, 2446, 2447, 2448, 2449, 2450, 2451, 2452, 2453, 2454, 2455, 2456, 2457, 2458, 2459, 2460, 2461 | JGD2000 / Japan Plane Rectangular coordinate systems I–XIX |
| 6669, 6670, 6671, 6672, 6673, 6674, 6675, 6676, 6677, 6678, 6679, 6680, 6681, 6682, 6683, 6684, 6685, 6686, 6687, 6688, 6689, 6690, 6691, 6692 | JGD2011 / Japan Plane Rectangular coordinate systems I–XIX and UTM zones 51N–55N |
| 5514 | S-JTSK / Krovak East North |
| 27700, 29900 | British National Grid, Irish National Grid |
| 31466–31469 | German Gauss-Krüger zones 2–5 |
| 28992 | Netherlands RD New |
| 2154 | France Lambert-93 |
| 5070, 3577, 3578, 3579 | CONUS Albers Equal Area, Australian Albers, Yukon Albers |
| 28349–28356 | Australia GDA94 / MGA zones 49–56 |
| 32661, 32761 | WGS84 / UPS North, UPS South |
| 2229–2286, 26929–26998*, 2759–2866, 3465–3552, 6355–6627**, 3338 | US State Plane NAD83 — legacy + national meter SPCS83 coverage plus NAD83(HARN), NAD83(NSRS2007), and NAD83(2011) coverage (*excluding 26947, which is unassigned in EPSG; **excluding unassigned gaps 6357–6392 and 6622–6624) |
| 21781, 2056 | Swiss LV03, LV95 |
| 22275–22293 | South Africa Cape Lo projections |
| 3007–3014 | Sweden SWEREF99 local TM zones (12°–17.25°E) |
| 2176–2180 | Poland CS2000 zones 5–8 and CS92 (ETRS89) |
| 2100 | Greece GGRS87 / Greek Grid |
| 23700 | Hungary HD72 / EOV |
| 31700 | Romania Dealul Piscului 1970 / Stereo 70 |
| 3763 | Portugal ETRS89 / TM06 |
| 3765 | Croatia HTRS96 / TM |
| 3301 | Estonia ETRS89 / L-EST97 |
| 5243 | Germany ETRS89 / LCC (N) |
| 2039 | Israel 1993 / Israeli TM Grid |
| 3414 | Singapore SVY21 / TM |
| 2326 | Hong Kong 1980 Grid |
| 3347 | Canada NAD83 / Statistics Canada Lambert |
| 3978 | Canada NAD83 / Atlas Lambert |
| 3174 | NAD83 / Great Lakes and St Lawrence Albers |
| 6350 | NAD83(2011) / CONUS Albers |
| 3111 | Australia GDA94 / VicGrid |
| 3308 | Australia GDA94 / NSW Lambert |
| 7846–7848, 3812, 31256–31258, 31287, 5179, 5181, 5182, 5186, 5187, 3825, 3826, 3112, 3005, 3015, 3767, 2040–2043, 2046–2055, 2057, 2058–2061, 2063, 2064, 2067–2080, 2085–2098, 2105–2138, 2148–2153, 2158–2162, 2164–2170, 2397–2399 | Additional regional systems: Australia GDA2020 MGA 46–48, Belgium Lambert 2008, Austria MGI GK/Lambert, Korea KGD2002/Korean 1985 belts, Taiwan TWD97 TM2 zones, Australia GA Lambert, BC Albers, SWEREF99 18 45, Croatia UTM 33N/LCC, Cote d'Ivoire UTM, Hartebeesthoek94 Lo zones, Rassadiran Nakhl-e Taqi, ED50(ED77) UTM zones 38N-41N, Guinea UTM, Naparima UTM, ELD79 Libya TM/UTM zones, Carthage TM, Yemen NGN96 UTM, South Yemen GK, Hanoi GK, WGS72BE TM, Cuba Norte/Sur, NZGD2000 local circuits plus UTM 58S-60S, Accra grids, Quebec Lambert (CGQ77), NAD83(CSRS) UTM aliases, Sierra Leone grids/UTM, Luxembourg Gauss, MGI Slovenia Grid, and Pulkovo adjusted 3-degree GK zones |
| 7843, 7845, 5513, 2065, 31254, 31255, 31265–31267, 3766, 2048–2055, 2058, 31275, 31276 | Additional regional systems II: GDA2020 geographic/GA LCC, S-JTSK Krovak variants, Austria MGI GK and Balkans zones, Croatia LCC, Hartebeesthoek94 Lo19–Lo33, and ED50(ED77) UTM 38N |
| 7841, 7842 | GDA2020 vertical height CRS and geocentric CRS (ECEF XYZ) |

---

## Supported Datums

Built-in datum definitions currently include:

- `WGS84`
- `NAD83`
- `NAD83(CSRS)`
- `NAD27`
- `ETRS89`
- `ED50`
- `GDA94`
- `GDA2020`
- `CGCS2000`
- `SIRGAS2000`
- `New Beijing`
- `Xian 1980`
- `Antigua 1943`
- `Dominica 1945`
- `Grenada 1953`
- `Montserrat 1958`
- `St. Kitts 1955`
- `NZGD2000`
- `JGD2000`
- `JGD2011`
- `RDN2008`
- `VN2000`
- `OSGB36`
- `DHDN`
- `Pulkovo 1942(58)`
- `Pulkovo 1942(83)`
- `S-JTSK`
- `Belge 1972`
- `Amersfoort`
- `TM65`
- `Katanga 1955`
- `Cape`
- `Puerto Rico 1927`
- `St. Croix`
- `CH1903`
- `CH1903+`
- `South East Island 1943`
- `SVY21`

## Supported Ellipsoids

Built-in constants include:

- `WGS 84`
- `GRS 80`
- `Clarke 1866`
- `International 1924`
- `Bessel 1841`
- `Airy 1830`
- `Airy 1830 Modified`
- `Krassowsky 1940`
- `Clarke 1880 (RGS)`
- `IAU 1976`
- `Sphere`

Additional standard ellipsoids are available through lookup helpers:

- `Ellipsoid::from_name("WGS 72")`
- `Ellipsoid::from_name("GRS 67")`
- `Ellipsoid::from_name("Clarke 1880 (RGS)")`
- `Ellipsoid::from_name("Everest 1830")`
- `Ellipsoid::from_name("Helmert 1906")`
- `Ellipsoid::from_name("Australian National Spheroid")`
- `Ellipsoid::from_name("Fischer 1960")`

Common EPSG ellipsoid codes are also supported:

- `Ellipsoid::from_epsg_ellipsoid(7030)` → WGS 84
- `Ellipsoid::from_epsg_ellipsoid(7019)` → GRS 80
- `Ellipsoid::from_epsg_ellipsoid(7024)` → Krassowsky 1940
- `Ellipsoid::from_epsg_ellipsoid(7049)` → IAU 1976

## Manual Projection Construction

For projections or parameters not in the EPSG registry, you can build a `ProjectionParams` directly.

### UTM (manual)

```rust
use wbprojection::{Projection, ProjectionParams};

let proj = Projection::new(ProjectionParams::utm(32, false))?;
let (easting, northing) = proj.forward(9.18, 48.78)?;
```

### Lambert Conformal Conic

```rust
use wbprojection::{Projection, ProjectionParams, ProjectionKind};

let proj = Projection::new(
    ProjectionParams::new(ProjectionKind::LambertConformalConic {
        lat1: 33.0,
        lat2: Some(45.0),
    })
    .with_lat0(39.0)
    .with_lon0(-96.0),
)?;
let (x, y) = proj.forward(-96.0, 39.0)?;
```

### Custom ellipsoid

```rust
use wbprojection::{Projection, ProjectionParams, ProjectionKind, Ellipsoid};

let proj = Projection::new(
    ProjectionParams::new(ProjectionKind::TransverseMercator)
        .with_lon0(-2.0)
        .with_lat0(49.0)
        .with_scale(0.9996012717)
        .with_false_easting(400_000.0)
        .with_false_northing(-100_000.0)
        .with_ellipsoid(Ellipsoid::from_a_inv_f("Airy 1830", 6_377_563.396, 299.3249646)),
)?;
```

---

## CRS-to-CRS Transformation

Transform directly between any two CRSes. The pipeline automatically handles datum shifts through WGS84 as a pivot.

```rust
use wbprojection::Crs;

// Convert a UTM 32N coordinate into Web Mercator
let src = Crs::from_epsg(32632)?;
let dst = Crs::from_epsg(3857)?;

let (utm_e, utm_n) = src.forward(9.0, 48.0)?;
let (web_x, web_y) = src.transform_to(utm_e, utm_n, &dst)?;

// British National Grid → WGS84 geographic
let bng  = Crs::from_epsg(27700)?;
let wgs  = Crs::from_epsg(4326)?;
let (bng_e, bng_n) = bng.forward(-0.1276, 51.5074)?;
let (lon, lat) = bng.transform_to(bng_e, bng_n, &wgs)?;
```

### Policy-controlled transform behavior

Use `transform_to_with_policy` when you want explicit control over missing/out-of-extent grid-shift handling.

```rust
use wbprojection::{Crs, CrsTransformPolicy};

let src = Crs::from_epsg(4267)?; // NAD27 geographic
let dst = Crs::from_epsg(4326)?; // WGS84 geographic

// Strict (default): errors on missing/out-of-extent grid-shift transforms.
let strict = src.transform_to_with_policy(
    -96.0,
    41.0,
    &dst,
    CrsTransformPolicy::Strict,
)?;

// Fallback mode: if a grid-shift transform cannot be applied,
// it falls back to identity shift instead of returning an error.
let fallback = src.transform_to_with_policy(
    -96.0,
    41.0,
    &dst,
    CrsTransformPolicy::FallbackToIdentityGridShift,
)?;

println!("strict={strict:?}, fallback={fallback:?}");
```

### Transform trace metadata

Use `transform_to_with_trace` to get output coordinates plus which source/target
grid was selected during datum transformation:

```rust
use wbprojection::{Crs, CrsTransformPolicy};

let src = Crs::from_epsg(4267)?;
let dst = Crs::from_epsg(4326)?;

let trace = src.transform_to_with_trace(
    -96.0,
    41.0,
    &dst,
    CrsTransformPolicy::Strict,
)?;

// Equivalent convenience helper for strict mode:
let trace2 = src.transform_to_with_trace_strict(-96.0, 41.0, &dst)?;

println!(
    "x={}, y={}, src_grid={:?}, dst_grid={:?}",
    trace.x, trace.y, trace.source_grid, trace.target_grid
);
```

---

## Batch Transformation

```rust
use wbprojection::{Projection, ProjectionParams, ProjectionKind};
use wbprojection::transform::{CoordTransform, Point2D};

let proj = Projection::new(ProjectionParams::new(ProjectionKind::Mercator))?;
let mut points = vec![
    Point2D::lonlat(0.0, 0.0),
    Point2D::lonlat(10.0, 20.0),
    Point2D::lonlat(-45.0, -30.0),
];
// Transforms points in-place; returns a Vec<Result<()>> per point
let results = proj.transform_fwd_many(&mut points);
```

---

## Grid-Shift Datum Workflow (NTv2 / NADCON)

`wbprojection` can ingest shift grids and apply them through `DatumTransform::GridShift`.

### NTv2 (`.gsb`) example

```rust
use wbprojection::{Crs, Datum, Ellipsoid, ProjectionKind, ProjectionParams, register_ntv2_gsb};
use wbprojection::datum::DatumTransform;

// 1) Load + register a named NTv2 grid
register_ntv2_gsb("./grids/OSTN15_NTv2.gsb", "OSTN15")?;

// 2) Build a custom source CRS that uses that grid-shift datum
let osgb36_grid = Datum {
    name: "OSGB36 (grid)",
    ellipsoid: Ellipsoid::from_a_inv_f("Airy 1830", 6_377_563.396, 299.3249646),
    transform: DatumTransform::GridShift { grid_name: "OSTN15" },
};

let src = Crs::new(
    "OSGB36 geographic (grid)",
    osgb36_grid,
    ProjectionParams::new(ProjectionKind::Geographic),
)?;

// 3) Transform into WGS84 geographic
let wgs84 = Crs::from_epsg(4326)?;
let (lon_wgs84, lat_wgs84) = src.transform_to(-1.54, 53.80, &wgs84)?;
println!("{lon_wgs84:.8}, {lat_wgs84:.8}");
```

### NADCON ASCII pair example

```rust
use wbprojection::{
    Crs, Datum, Ellipsoid, ProjectionKind, ProjectionParams,
    register_nadcon_ascii_pair,
};
use wbprojection::datum::DatumTransform;

// Expects two ASCII files (lon-shift and lat-shift), both in arc-seconds.
// Header line format in each file:
// lon_min lat_min lon_step lat_step width height
register_nadcon_ascii_pair("./grids/conus.los", "./grids/conus.las", "NADCON_CONUS")?;

let nad27_grid = Datum {
    name: "NAD27 (grid)",
    ellipsoid: Ellipsoid::CLARKE1866,
    transform: DatumTransform::GridShift { grid_name: "NADCON_CONUS" },
};

let src = Crs::new(
    "NAD27 geographic (grid)",
    nad27_grid,
    ProjectionParams::new(ProjectionKind::Geographic),
)?;

let dst = Crs::from_epsg(4326)?;
let (lon_wgs84, lat_wgs84) = src.transform_to(-96.0, 41.0, &dst)?;
println!("{lon_wgs84:.8}, {lat_wgs84:.8}");
```

### Registry helpers

You can inspect and manage registered grids using:

- `has_grid(name)`
- `get_grid(name)`
- `unregister_grid(name)`

### NTv2 subgrid helpers

For multi-subgrid NTv2 files, you can now enumerate and target a specific subgrid:

- `list_ntv2_subgrids(path)`
- `load_ntv2_gsb_subgrid(path, grid_name, subgrid_name)`
- `register_ntv2_gsb_subgrid(path, grid_name, subgrid_name)`

To enable runtime coordinate-based subgrid selection (deepest covering subgrid),
register the full NTv2 hierarchy and use `DatumTransform::Ntv2Hierarchy`:

```rust
use wbprojection::{Crs, Datum, Ellipsoid, ProjectionKind, ProjectionParams, register_ntv2_gsb_hierarchy};
use wbprojection::datum::DatumTransform;

register_ntv2_gsb_hierarchy("./grids/my_ntv2.gsb", "MY_NTV2_DATASET")?;

let src = Crs::new(
    "Hierarchical NTv2 datum",
    Datum {
        name: "Hierarchical NTv2 datum",
        ellipsoid: Ellipsoid::WGS84,
        transform: DatumTransform::Ntv2Hierarchy {
            dataset_name: "MY_NTV2_DATASET",
        },
    },
    ProjectionParams::new(ProjectionKind::Geographic),
)?;
```

For debugging/audit logs, you can inspect which hierarchy entry would be selected:

- `resolve_ntv2_hierarchy_grid_name(dataset_name, lon_deg, lat_deg)`
- `resolve_ntv2_hierarchy_subgrid(dataset_name, lon_deg, lat_deg)`

You can also promote any built-in datum to a grid-backed transform using helper builders:

```rust
use wbprojection::Datum;

let osgb36_ntv2 = Datum::OSGB36.with_ntv2_hierarchy("OSTN15_DATASET");
let nad27_grid = Datum::NAD27.with_grid_shift("NADCON_CONUS");
```

### Real-world checklist

Before using a grid-shift transform in production, verify:

- **Correct source/target datums**: the grid must match the exact datum pair you intend to transform.
- **Grid coverage**: your coordinates fall inside the grid extent (outside points return a datum error).
- **Axis/units consistency**: longitudes and latitudes are geographic degrees at the `Crs::transform_to` API boundary.
- **Expected direction**: use a known checkpoint to confirm forward behavior and inverse round-trip.
- **Fallback policy**: decide whether your app should fail hard on missing grids or switch to Helmert-based fallback.
- **QA tolerance**: compare against authoritative reference values for your area of use and record acceptable error thresholds.

---


## Supported Projections

`ProjectionKind` currently supports **94 projection types** (human-readable names returned by `Projection::name()`):

| Projection type | Projection type | Projection type |
|---|---|---|
| Geographic | Geocentric | Vertical |
| Geostationary Satellite View | Mercator | Web Mercator |
| Transverse Mercator | Transverse Mercator South Orientated | UTM |
| Lambert Conformal Conic | Polar Stereographic (North) | Polar Stereographic (South) |
| Albers Equal-Area Conic | Azimuthal Equidistant | Lambert Azimuthal Equal-Area |
| Krovak | Hotine Oblique Mercator | Central Conic |
| Lagrange | Loximuthal | Euler |
| Tissot | Murdoch I | Murdoch II |
| Murdoch III | Perspective Conic | Vitkovsky I |
| Tobler-Mercator | Winkel II | Kavrayskiy V |
| Stereographic | Oblique Stereographic | Orthographic |
| Sinusoidal | Mollweide | McBryde-Thomas Flat-Pole Sine (No. 2) |
| McBryde-Thomas Flat-Polar Sine (No. 1) | McBryde-Thomas Flat-Polar Parabolic | McBryde-Thomas Flat-Polar Quartic |
| Nell | Equal Earth | Cylindrical Equal Area |
| Equirectangular | Robinson | Gnomonic |
| Aitoff | Van der Grinten | Winkel Tripel |
| Hammer | Hatano | Eckert I |
| Eckert II | Eckert III | Eckert IV |
| Eckert V | Miller Cylindrical | Gall Stereographic |
| Gall-Peters | Behrmann | Hobo-Dyer |
| Wagner I | Wagner II | Wagner III |
| Wagner IV | Wagner V | Natural Earth |
| Natural Earth II | Wagner VI | Eckert VI |
| Transverse Cylindrical Equal Area | Polyconic | Cassini-Soldner |
| Bonne | Bonne South Orientated | Craster |
| Putnins P4' | Fahey | Times |
| Patterson | Putnins P3 | Putnins P3' |
| Putnins P5 | Putnins P5' | Putnins P1 |
| Putnins P2 | Putnins P6 | Putnins P6' |
| Quartic Authalic | Foucaut | Winkel I |
| Werenskiold I | Collignon | Nell-Hammer |
| Kavrayskiy VII | Two-Point Equidistant |  |

## Error Handling

All projection functions return `wbprojection::Result<T>` (an alias for `Result<T, ProjectionError>`).

```rust
use wbprojection::ProjectionError;

match crs.forward(0.0, 91.0) {
    Err(ProjectionError::OutOfBounds(msg))           => eprintln!("Out of bounds: {msg}"),
    Err(ProjectionError::ConvergenceFailure { iterations }) => eprintln!("Did not converge after {iterations} iterations"),
    Err(ProjectionError::UnsupportedProjection(msg)) => eprintln!("Unknown EPSG code: {msg}"),
    Err(e)                                           => eprintln!("Error: {e}"),
    Ok((x, y))                                       => println!("{x:.2}, {y:.2}"),
}
```

See also: [SIMD guardrail check](../../README.md#simd-guardrail-check) for a script you can run locally to verify speedup and correctness.

---

## Performance

This library uses the [`wide`](https://github.com/Lokathor/wide) crate to provide **SIMD optimizations** for datum-transformation kernels, with the current focus on batched Helmert ECEF operations (`HelmertParams::apply_simd_batch4` and `HelmertParams::apply_inverse_simd_batch4`). The public batch CRS APIs are available for batched workflows, but they currently still orchestrate per-point CRS transformations around those lower-level kernels rather than vectorizing the entire CRS pipeline end to end.

SIMD is **enabled by default** in this crate. There is currently no feature flag required to turn SIMD paths on.

This is a **temporary implementation strategy** until [Portable SIMD](https://github.com/rust-lang/rfcs/blob/master/text/2948-portable-simd.md) stabilizes in Rust. Once portable SIMD is available in stable Rust, `wbprojection` will transparently migrate to that standard approach while maintaining the same performance characteristics.

You can run the current benchmark example with:

```bash
cargo run --release --example simd_batch_transform
```

That example compares the scalar Helmert kernel against the SIMD batch kernel and also reports timings for the current CRS batch wrapper.

---

## Architecture

```
wbprojection/
├── lib.rs               – public API re-exports, helpers
├── ellipsoid.rs         – Ellipsoid struct + named constants
├── datum.rs             – Datum, HelmertParams, ECEF conversions
├── error.rs             – ProjectionError enum
├── transform.rs         – Point2D, Point3D, CoordTransform trait
├── epsg.rs              – EPSG registry (5591 EPSG codes / 5594 total CRS/projection codes → ProjectionParams)
├── epsg_generated_wkt.rs – Consolidated generated ESRI WKT lookup for EPSG entries
├── crs.rs               – Crs struct with datum + projection + from_epsg
└── projections/
    ├── mod.rs           – Projection, ProjectionParams, ProjectionKind
    ├── mercator.rs
    ├── transverse_mercator.rs
    ├── lambert_conformal_conic.rs
    ├── albers.rs
    ├── axis_oriented.rs
    ├── azimuthal_equidistant.rs
    ├── behrmann.rs
    ├── bonne.rs
    ├── cassini.rs
    ├── collignon.rs
    ├── craster.rs
    ├── eckert_i.rs
    ├── eckert_ii.rs
    ├── eckert_iii.rs
    ├── eckert_iv.rs
    ├── eckert_v.rs
    ├── eckert_vi.rs
    ├── fahey.rs
    ├── gall_stereographic.rs
    ├── krovak.rs
    ├── stereographic.rs
    ├── polar_stereographic.rs
    ├── orthographic.rs
    ├── sinusoidal.rs
    ├── mollweide.rs
    ├── equal_earth.rs
    ├── cylindrical_equal_area.rs
    ├── equirectangular.rs
    ├── geostationary.rs
    ├── robinson.rs
    ├── gnomonic.rs
    ├── hammer.rs
    ├── miller_cylindrical.rs
    ├── kavrayskiy_vii.rs
    ├── aitoff.rs
    ├── gall_peters.rs
    ├── hobo_dyer.rs
    ├── natural_earth.rs
    ├── patterson.rs
    ├── polyconic.rs
    ├── two_point_equidistant.rs
    ├── putnins_p3.rs
    ├── putnins_p3p.rs
    ├── putnins_p4p.rs
    ├── putnins_p5.rs
    ├── putnins_p5p.rs
    ├── putnins_p1.rs
    ├── times.rs
    ├── van_der_grinten.rs
    ├── wagner_ii.rs
    ├── wagner_iv.rs
    ├── wagner_v.rs
    ├── wagner_vi.rs
    ├── werenskiold_i.rs
    └── winkel_tripel.rs
```

---

## Compilation Features

`wbprojection` has minimal optional features.

| Feature | Default | Purpose |
|---------|:-------:|---------|
| `serde` | no | Enables `serde` `Serialize`/`Deserialize` for core CRS types. |

Example:

```toml
[dependencies]
wbprojection = { version = "0.1", features = ["serde"] }
```

The core projection and datum logic, EPSG registry, WKT export/import, and SIMD Helmert batch paths are always enabled and require no feature flag.

## Known Limitations

- Coverage is broad but not exhaustive; some exotic projection methods, datum models, or regional CRS variants may not be supported.
- `from_wkt` and `compound_from_wkt` handle the most common WKT patterns but are not a complete OGC/EPSG WKT standards engine; unsupported method names or uncommon unit models return `UnsupportedProjection`.
- Grid-shift datum transforms (NTv2, NADCON) require externally supplied grid files; `wbprojection` does not bundle or download grids.
- Adaptive EPSG identification (`identify_epsg_from_wkt`) searches the full registry on each call; prefer cached or explicitly known EPSG codes in performance-sensitive paths.
- `transform_to_3d` is strict about valid horizontal/vertical CRS component combinations; mixing unsupported combinations returns an error rather than silently degrading.
- The SIMD Helmert batch path accelerates datum-transformation kernels but does not yet vectorize the full CRS-to-CRS pipeline end-to-end.

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
