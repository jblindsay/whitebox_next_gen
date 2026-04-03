# Sentinel-2 SAFE Reader Plan for wbraster

## Goal

Add native Sentinel-2 SAFE package support in `wbraster` as a package-level reader layer
that reuses existing JPEG2000 decoding, while preserving `Raster` as the core data API.

This is intentionally **not** a new pixel codec. It is a structured package parser +
band/metadata resolver.

---

## Design Principles

1. Keep codecs and package formats separate.
2. Keep `Raster::read(path)` behavior backward compatible.
3. Add SAFE support as opt-in APIs first, then optionally broaden auto-detection.
4. Expose enough metadata for downstream tools (solar angles, acquisition time, QA bands).
5. Keep first milestone read-only and deterministic.

---

## Scope for MVP (Phase 1)

### Supported SAFE variants

- Sentinel-2 Level-2A SAFE (priority)
- Sentinel-2 Level-1C SAFE (secondary; band/QA availability differs)

### Supported package forms

- Unzipped `.SAFE` directory
- Optional future extension: zipped SAFE archives (defer)

### Supported outputs

- Band path resolution for core bands: `B02`, `B03`, `B04`, `B08`
- QA path resolution for: `SCL` (L2A) and `QA60` when present
- Parsed scene metadata: acquisition datetime UTC, mean sun zenith, mean sun azimuth,
  product level, tile identifier

### Non-goals in MVP

- Mosaicking multiple granules automatically
- Resampling to common resolution automatically
- Atmospheric correction logic
- Cloud/shadow classification itself

---

## Proposed API Surface

### New package module in wbraster

- `crates/wbraster/src/packages/sentinel2_safe.rs`
- `crates/wbraster/src/packages/mod.rs`

### Core types

```rust
pub struct Sentinel2SafePackage {
    pub safe_root: PathBuf,
    pub product_level: Sentinel2ProductLevel,
    pub acquisition_datetime_utc: Option<String>,
    pub mean_solar_zenith_deg: Option<f64>,
    pub mean_solar_azimuth_deg: Option<f64>,
    pub tile_id: Option<String>,
    pub bands: BTreeMap<String, PathBuf>,
    pub qa_layers: BTreeMap<String, PathBuf>,
}

pub enum Sentinel2ProductLevel {
    L1C,
    L2A,
    Unknown,
}
```

### Reader methods

```rust
impl Sentinel2SafePackage {
    pub fn open(safe_root: impl AsRef<Path>) -> Result<Self, RasterError>;
    pub fn list_band_keys(&self) -> Vec<String>;
    pub fn band_path(&self, key: &str) -> Option<&Path>;
    pub fn qa_path(&self, key: &str) -> Option<&Path>;
    pub fn read_band(&self, key: &str) -> Result<Raster, RasterError>;
}
```

### Optional convenience method on Raster (Phase 2)

```rust
pub fn read_sentinel2_safe_band(safe_root: impl AsRef<Path>, band: &str) -> Result<Raster, RasterError>
```

---

## Parsing Responsibilities

### Product-level metadata parser

Inputs:
- SAFE root metadata XML (`MTD_MSIL1C.xml` or `MTD_MSIL2A.xml`)

Extract:
- Product level
- Datatake sensing start / acquisition timestamp
- Mean sun zenith angle
- Mean sun azimuth angle
- Tile IDs or granule references

### Granule-level resolver

Inputs:
- `GRANULE/*/IMG_DATA/**` hierarchy

Resolve:
- JP2 path for each spectral band key (`B01..B12`, `B8A`)
- JP2 path for QA layers (`SCL` in L2A, `QA60` if present)
- Resolution suffixes (`10m`, `20m`, `60m`) and naming variants

### Canonical key mapping

Map SAFE filenames to canonical keys:
- `B02`, `B03`, `B04`, `B08`, ...
- `SCL`, `QA60`

This keeps downstream tool integrations independent of SAFE filename conventions.

---

## Integration into terrain_corrected_optical

Once package API exists:

1. Add optional `safe_root` parameter in `terrain_corrected_optical`.
2. If provided and solar_mode is auto/metadata:
   - read mean sun angles from SAFE metadata.
3. If `qa_mask` not provided and SAFE has `SCL` or `QA60`:
   - auto-bind QA source path.
4. If `input_red`/`input_nir` omitted and SAFE root is provided:
   - allow default mapping `B04` -> red, `B08` -> NIR.

Do this in a separate PR after package parser is stable.

---

## Phased Implementation Checklist

## Phase 1: SAFE package parser in wbraster

1. Add `packages/sentinel2_safe.rs` module and public exports.
2. Implement SAFE root detection and product XML discovery.
3. Implement product metadata XML parser for solar angles + timestamp.
4. Implement granule directory walker and JP2 band/QA discovery.
5. Implement canonical key map (`Bxx`, `B8A`, `SCL`, `QA60`).
6. Implement `read_band` via existing `Raster::read` on resolved JP2 path.
7. Add unit tests using synthetic SAFE-like directory fixtures.
8. Add one integration test with a real small SAFE fixture (if license allows in repo).

Acceptance criteria:
- `Sentinel2SafePackage::open` works for at least one L2A package.
- `read_band("B04")` and `read_band("B08")` return valid rasters.
- mean solar angles and acquisition datetime parse successfully.

## Phase 2: Ergonomic API additions

1. Add convenience function(s) for one-call SAFE band reads.
2. Optional auto-detection path in `Raster::read` for SAFE roots (carefully gated).
3. Improve error messages for missing bands/QA and malformed SAFE trees.

Acceptance criteria:
- Users can read SAFE bands with minimal code.
- Backward compatibility maintained for all existing raster reads.

## Phase 3: Tool-level adoption in wbtools_pro

1. Add optional SAFE-aware inputs in `terrain_corrected_optical`.
2. Wire solar metadata auto-resolution to SAFE metadata.
3. Wire QA auto-source (`SCL`/`QA60`) into stage-1 mask strategy logic.
4. Add docs examples for SAFE-only workflow.

Acceptance criteria:
- User can run terrain correction from SAFE root with near-zero manual parameters.

---

## Testing Strategy

### Unit tests

- SAFE root discovery
- XML field extraction edge cases (missing angle tags, malformed numeric values)
- Band key mapping across naming variants
- QA path detection (`SCL`, `QA60`)

### Integration tests

- Parse real L2A SAFE fixture and confirm key outputs
- Read B04/B08 rasters from SAFE and validate dimensions > 0
- Confirm solar metadata populated

### Regression tests

- Ensure no change to non-SAFE format reading paths
- Ensure JP2 reader behavior unchanged for direct JP2 file reads

---

## Risks and Mitigations

1. SAFE naming variations across processing baselines
- Mitigation: use pattern-based matching with canonical key extraction tables and fallback rules.

2. Multi-granule packages causing ambiguous band picks
- Mitigation: require explicit tile selection in non-trivial cases; default only when unambiguous.

3. XML schema drift
- Mitigation: robust optional extraction and version-tolerant parser.

4. Scope creep into atmospheric processing
- Mitigation: keep package parser read-only; no correction logic in `wbraster`.

---

## Recommended Next Step

Start with Phase 1 and add only:
- `Sentinel2SafePackage::open`
- metadata extraction (sun angles + datetime)
- canonical band resolution for B04/B08/SCL/QA60
- `read_band`

This gives immediate leverage for `terrain_corrected_optical` while keeping implementation risk low.
