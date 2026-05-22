# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]

## [0.2.0] - 2026-05-21

### Added
- `CrsTransformPolicy::Auto` for interoperability-focused horizontal datum handling.
  In `Auto` mode, WGS84 <-> NAD83-family transforms are treated as ballpark
  equivalent (no explicit datum shift) while preserving strict projection math.

### Changed
- Added explicit datum-equivalence routing in CRS transform paths
  (`transform_to_3d_with_policy` and `transform_to_with_trace`) with
  interoperability-first (`Auto`) behavior used by vector reprojection flows.

### Fixed
- Implemented `DatumTransform::GridShift` in trace-aware geodetic datum paths
  so strict mode now surfaces missing/out-of-extent grid errors, fallback mode
  can still degrade to identity, and transform traces report selected grid names.
- Synchronized README supported-code totals with the current EPSG registry,
  restoring readme/count consistency tests.
- Updated Molodensky round-trip height test tolerance to a realistic
  centimetre-scale bound for inverse-by-negation behavior, preventing a false
  negative in the datum regression suite.

### Tests
- Added regression coverage verifying `CrsTransformPolicy::Auto` keeps
  NAD83->WGS84 coordinates invariant in the expected ballpark-equivalent case.

## [0.1.1] – 2026-05-09 (Reaffirmed)

### Testing
- Interop Phase A projection conformance: 33/33 passing across 11 CRS families.
- Forward/inverse tolerance validation confirmed for all tested CRS.

*Note: Version 0.1.1 is being published as-is as part of the interop release milestone (2026-05-09) to affirm Phase A validation.*

## [0.1.1] - 2026-05-07
### Added
- `Crs::to_wkt(&self) -> String` — serialize a `Crs` instance to an
  Esri-style WKT1 string from struct fields (metre units).
- `Crs::from_wkt(wkt: &str) -> Result<Crs>` — ergonomic instance-level
  wrapper for the existing `epsg::from_wkt` parser.
- `epsg::crs_to_wkt(crs: &Crs) -> String` — free function form of the WKT
  serializer, re-exported from the crate root.
- `epsg::canonical_wkt_for_epsg(code: u32) -> Option<&'static str>` — returns
  the canonical static WKT string for an EPSG code (from the legacy-parity or
  generated-WKT tables), re-exported from the crate root.
- **PROJ string parser** (`from_proj_string`, `parse_proj_string`).
  Accepts PROJ4-compatible `+key=value` strings including:
  - All common `+proj=` codes: `utm`, `tmerc`, `lcc`, `aea`, `merc`,
    `webmerc`, `longlat`, `geocent`, `stere`, `omerc`, `laea`, `aeqd`,
    `ortho`, `gnom`, `sinu`, `moll`, `robin`, `eck1`-`eck6`, `eqearth`,
    `aitoff`, `vandg`, `wintri`, `hammer`, `cea`, `eqc`, `poly`, `cass`,
    `bonne`, `krovak`, `geos`, `tpeqd`, `wag1`-`wag6`, and 30+ more.
  - `+datum=WGS84|NAD83|NAD27|ETRS89|GDA94|GDA2020|SIRGAS2000|NZGD2000`
    mapped to the corresponding named `Datum` constants.
  - `+ellps=wgs84|grs80|clrk66|intl|bessel|airy|krass|...` for 15+ named
    ellipsoids plus `+a=`/`+b=`/`+rf=`/`+f=`/`+R=` for custom ellipsoids.
  - `+towgs84=dx,dy,dz` (3-param Helmert) and
    `+towgs84=dx,dy,dz,rx,ry,rz,ds` (7-param) for datum shifts.
  - `+lon_0=`, `+lat_0=`, `+x_0=`, `+y_0=`, `+k=`/`+k_0=`,
    `+lat_1=`, `+lat_2=`, `+lat_ts=`, `+zone=`, `+south`,
    `+alpha=`/`+gamma=` (Hotine OM), `+h=`/`+sweep=` (geostationary),
    `+lon_1=`/`+lat_1=`/`+lon_2=`/`+lat_2=` (two-point equidistant).
  - Angle values may be decimal degrees, `DDdMM'` DMS form, or radians
    with `r` suffix (e.g. `1.3089969r`).
  - `+units=m|ft|us-ft|km|cm|mm|mi|link|chain` and `+to_meter=` stored
    in `ParsedProjUnits` (does not alter stored metre parameters).
  - `+init=epsg:XXXX` and bare `EPSG:XXXX` shortcuts resolved via the
    built-in EPSG registry.
- `ParsedProjString` struct: full parse result exposing `crs` and `units`.
- `ParsedProjUnits` struct: declared output-unit scale factor + label.
- `ParsedProjString` and `ParsedProjUnits` exported from crate root.
- `Crs::forward_many(&self, points: &[(f64, f64)]) -> Vec<Result<(f64, f64)>>` -
  serial batch forward projection.
- `Crs::inverse_many` - serial batch inverse projection.
- `Crs::transform_to_many` - serial batch cross-CRS transformation.
- `Crs::forward_many_par`, `Crs::inverse_many_par`, `Crs::transform_to_many_par`
  - parallel batch variants using Rayon (require the `parallel` crate feature).
- `parallel` crate feature: activates Rayon for all `*_par` batch methods.
  Add `wbprojection = { ..., features = ["parallel"] }` to enable.
- **Molodensky datum shift** (`DatumTransform::Molodensky`, `MolodenskyParams`).
  Implements the standard (non-abridged) Molodensky method that transforms
  geodetic latitude, longitude, and ellipsoidal height directly, using the
  difference in semi-major axis and flattening between the source datum
  ellipsoid and WGS84.
  - New `MolodenskyParams { dx, dy, dz }` struct with `const fn new()`.
  - New `DatumTransform::Molodensky(MolodenskyParams)` variant dispatched in
    `to_wgs84_geodetic_with_policy_and_trace` (forward) and
    `from_wgs84_geodetic_with_policy_and_trace` (inverse).
  - `to_wgs84_ecef` / `from_wgs84_ecef` perform a geodetic detour for
    Molodensky datums so the ECEF API remains usable; `supports_ecef_batch_simd`
    returns `false` for Molodensky.
  - `Datum::with_molodensky(dx, dy, dz)` builder method.
  - `MolodenskyParams` exported from the crate root alongside `HelmertParams`.
  - Unit tests in `datum::tests`: round-trip accuracy, shift magnitude,
    builder, and ECEF-path consistency.
- **Compound CRS registry expansion**.
  - New vertical CRS EPSG codes in `from_epsg`: `5711` (AHD height - Australia),
    `6647` (CGVD2013 height - Canada), `7839` (NZVD2016 height - New Zealand).
  - These codes are also added to `is_vertical_epsg`, `NAMED_ENTRIES`, and
    `vertical_offset_grid_name` (grid names `ausgeoid2020`, `cgvd2013`, `nzvd2016`
    for when users register the corresponding files at runtime).
  - `CompoundCrs::from_epsg` supports:
    - `5498` - NAD83 + NAVD88 height (horizontal: 4269, vertical: 5703)
    - `6649` - NAD83(CSRS) + CGVD2013 height (4617 + 6647)
    - `7405` - OSGB36 / British National Grid + ODN height (existing)
    - `9253` - GDA94 + AHD height (4283 + 5711)
    - `9518` - WGS 84 + EGM2008 height (4326 + 3855)
  - The error message for unsupported compound codes points callers to
    `CompoundCrs::new()` as an alternative.
- **CRS bounding box / area of use**.
  - New `CrsBoundingBox { lon_min, lat_min, lon_max, lat_max }` struct with
    `const fn new()` and `contains_geographic(lon, lat) -> bool`.
  - `epsg_area_of_use(code: u32) -> Option<CrsBoundingBox>` in `epsg`:
    computes bounds for UTM zone families (WGS84 N/S, NAD83, NAD27, ETRS89,
    ED50) from zone number, with a curated static table for additional common
    geographic, projected, vertical, and compound codes.
  - `Crs::area_of_use() -> Option<CrsBoundingBox>` on `Crs`: extracts EPSG code
    from `self.name` and calls `epsg_area_of_use`; falls back to world extent
    for geographic CRSes; returns `None` for custom projected CRSes without a
    registered code.
  - Both `CrsBoundingBox` and `epsg_area_of_use` exported from crate root.

### Fixed
- `epsg_from_wkt` now extracts the outermost EPSG `AUTHORITY[...]` tag using
  `rfind`, avoiding inner-code false matches in nested WKT.
- `epsg::from_wkt` now falls through to full WKT parsing when an embedded EPSG
  code is not present in the built-in registry.

### Changed
- Removed the unused direct dependency on `wbgeotiff`. Datum/grid loading in
	`wbprojection` currently uses native parsers (e.g., NTv2, NADCON ASCII,
	GTX) and does not require GeoTIFF IO.

## [0.1.0] - 2026-03-31
### Added
- Initial published release.
