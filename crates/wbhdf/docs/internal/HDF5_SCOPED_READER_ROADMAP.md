# HDF Scoped Reader Roadmap

Date: 2026-05-12 (updated 2026-05-31; MODIS/VIIRS scope update)  
Status: Active planning (post-launch execution)  
Owner: `wbhdf` core  
Scope: Targeted HDF4/HDF5 ingestion for scientific satellite datasets (GEDI, ICESat-2, MODIS, VIIRS, Sentinel-5P)

---

## Executive Summary

HDF4/HDF5 read support is a recurring friction point across multiple WNG toolsets:
- **Lidar tools** (`wblidar`, lidar QA/confidence pipelines): GEDI, ICESat-2
- **Remote sensing tools** (`wbtools_oss`, thermal/spectral analytics): MODIS, VIIRS, Sentinel-5P atmospheric products
- **Teaching workflows**: Reproducible multi-source analyses without external format conversion

Rather than implementing full-spec HDF4/HDF5 readers, this roadmap proposes a **scoped,
targeted reader strategy** covering only the product layouts and filter combinations actually
encountered in practice. The HDF5 path remains the primary implementation track; HDF4/HDF-EOS2
support is added specifically to unlock high-value MODIS product families. Feasibility estimate:
**3–6 weeks** for the HDF5 path, with additional targeted effort for MODIS/HDF4-EOS2 support.

**Key decisions (confirmed):**
- `wbhdf` will be a separate crate and the canonical targeted HDF container decoder
  (implemented HDF5-first, with scoped HDF4/HDF-EOS2 support where needed).
- MODIS support is valuable enough to justify a targeted HDF4/HDF-EOS2 decode track within the
  `wbhdf` umbrella, but not a full general-purpose HDF4 implementation.
- Integration into `wbraster` and `wblidar` should be first-class once implementation is stable,
  not hidden behind a long-term feature gate.
- Tier 1 ingestion is operationally `wblidar`-first; `wbraster` integration focuses on datasets
  that are naturally represented as raster products.

---

## 0. Plan Governance (Single-Track, Not Parallel)

This roadmap uses two views of the same work, with one clear hierarchy:

1. **Phases are the authoritative plan** for scope, architecture boundaries, acceptance criteria,
  and completion status.
2. **Week/Day checklists are execution cadence logs** used to sequence incremental delivery,
  evidence capture, and test milestones.
3. **No parallel implementation tracks exist.** Week entries must map to one or more phase
  deliverables.
4. **Conflict rule:** if a Week/Day entry appears to conflict with a Phase definition, the Phase
  definition wins and the Week/Day item must be updated.

### Phase-to-Week Mapping (Current)

| Phase | Purpose | Week/Day checklist role |
|---|---|---|
| Phase 1 | HDF5 superblock/object-header foundations | Week 1 Day 1-2 + Week 2 Day 1-3 execution checkpoints |
| Phase 1b | HDF4/HDF-EOS2 MODIS feasibility + payload bridge milestones | Week 3 remote-sensing checkpoints + dated Phase 1b evidence items |
| Phase 2 | HDF5 B-tree chunk indexing | Week 1 Day 3-4 kickoff and subsequent hardening |
| Phase 3 | Dataset decode/materialization pipeline | Week 2 Day 1-3 implementation steps |
| Phase 4 | `wblidar`/`wbraster` integration | Week 2 Day 4 + Week 3 Day 3 planned execution |
| Phase 5 | Test and docs hardening | Week 3 Day 4-5 planned execution |

### Operating Rule Going Forward

- Keep the Phase sections as the canonical source of truth.
- Keep Week/Day sections as implementation journals that reference Phase scope.
- When adding new work, first place it under the relevant Phase deliverables, then add any
  Week/Day sequencing notes.

---

## 1. Problem Statement

### Current Friction Points

| Dataset | Format | Toolset | Current Workaround |
|---|---|---|---|
| GEDI L2A/L2B | HDF5 | wblidar | User must pre-convert via GDAL or rasterize separately |
| ICESat-2 ATL03/ATL08 | HDF5 | wblidar | User must pre-convert via nsidc-convert or gedi4-subsetter scripts |
| MODIS land products | HDF4 / HDF-EOS2 | wbtools_oss | User must pre-convert via GDAL, MRT, or external science-tool exports |
| VIIRS land/environmental products | HDF5 / NetCDF4 | wbtools_oss | User must extract via GDAL, Python, or NASA/NOAA preprocessing tools |
| Sentinel-5P L2 | NetCDF (HDF5-backed) | wbtools_oss | User must extract via `xarray` or `gdal_translate` |
| ALOS-2 MLC (some distribution portals) | HDF5 | wbtools_oss (PolSAR) | User must pre-convert via SNAP or PolSARpro |

Each conversion step introduces potential data loss, complicates reproducibility, and raises the
barrier to entry for teaching users. Native HDF5 ingestion removes this friction entirely.

### Why Not Full HDF4/HDF5 Spec?

A complete HDF5 implementation in pure Rust would require:
- 10,000+ lines to handle all filter types (SZIP, LZF, Scaleoffset, etc.)
- Complex B-tree v2 implementation (non-trivial state management)
- Ongoing maintenance burden for edge cases
- Likely still need C linkage for SZIP filter (proprietary, no pure-Rust port)

**Not justified for the use case.** A targeted reader handling the small set of filters and
product layouts actually in use is maintainable and practical.

---

## 2. Scope: Datasets Covered

### Tier 1 (Core: post-launch + 6 months; wblidar-first)

**GEDI L2A/L2B — Canopy Height, Biomass**

- Product: NASA DAAC GEDI L2B Gridded Canopy Height Model (L2B_CH_MW019W_02_003_05_R31000M_MU)
- File: `~130 MB` per tile, HDF5 with GZIP compression
- Schema: Fixed compound datasets (one 2D array per variable, attributes for metadata)
- Filters needed: GZIP only
- Why: Direct input to `wblidar` tool pipelines; cross-validation with lidar QA outputs

**ICESat-2 ATL08 — Vegetation/Terrain Heights**

- Product: ATL08 Land and Vegetation Height (beam groups; photon segments)
- File: `~1 GB` per granule, HDF5 with GZIP, variable-length strings
- Schema: Hierarchical group structure (fixed groups per beam, variable-length records)
- Filters needed: GZIP only
- Why: Airborne lidar simulation / canopy height validation; educational value (compare spaceborne to terrestrial lidar)

**ICESat-2 ATL03 — Raw Photon Cloud**

- Product: ATL03 Sea Ice Freeboard (or geolocated photons, general)
- File: `~300 MB` per granule, HDF5 with GZIP
- Schema: Hierarchical (20 beams × 3 photon segments + ancillary)
- Filters needed: GZIP only
- Why: Terrain modeling, waveform decomposition; supporting the lidar QA pipeline

### Tier 1b (Core remote sensing companion: post-launch + 6-12 months)

**VIIRS Land/Surface Products — Moderate-resolution optical and thermal products**

- Format family: HDF5 / NetCDF4-style science products
- Why: VIIRS is the operational successor to MODIS for many land-surface workflows and is a better
  fit for the primary `wbhdf` path than MODIS.
- Initial focus: raster-like gridded products and well-structured science datasets that do not
  require broad CF/HDF5 generalization.
- First target product families:
  - `VNP09` / `VJ109` surface reflectance
  - `VNP13` / `VJ113` vegetation index
  - `VNP21` / `VJ121` land surface temperature

**MODIS Land Products — Surface Reflectance, Vegetation, Land Surface Temperature**

- Format family: HDF4 / HDF-EOS2
- Why: MODIS remains one of the highest-value omitted legacy sensor families for Whitebox remote
  sensing workflows, especially teaching and long time-series analyses.
- Initial focus: raster/grid-style product families, not all swath products.
- Constraint: this requires a dedicated HDF4/HDF-EOS2 module path inside `wbhdf` rather than
  reuse of the core HDF5 path implementation.
- First target product families:
  - `MOD09` / `MYD09` surface reflectance
  - `MOD13` / `MYD13` vegetation index
  - `MOD11` / `MYD11` land surface temperature

### Tier 2 (Extended: 12+ months, Complex Type Dependent)

**ALOS-2 PALSAR-2 MLC (Multi-Look Complex)** — L-Band PolSAR Coherency Matrices

- File: HDF5, typically GZIP
- Filters: GZIP only
- Schema: Fixed compound datasets; complex-valued float32/float64 per element
- **Blocker:** Requires `wbraster` complex data type support (see Section 5 below)
- Why: L-band PolSAR data completeness for vegetation penetration studies; enables full-polarimetry workflows without SNAP export step
- Timeline: Phase 2 of `wbhdf` implementation, contingent on `wbraster` `ComplexF32`/`ComplexF64` support

**Sentinel-5P L2** — Atmospheric Columns (NO₂, O₃, CO, CH₄)

- File: NetCDF (HDF5 backend) via CF conventions
- Filters: GZIP + some Scaleoffset
- Why: Teaching atmospheric correction / validation context for optical tools
- Lower priority than Tier 1 (can use GDAL for now; nice-to-have for reproducibility)

### Additional Product-Family Notes

- **VIIRS** is a closer architectural fit to the primary `wbhdf` effort because it is commonly
  distributed in HDF5/NetCDF4-class product layouts.
- **MODIS** offers large practical value but should be handled as a targeted HDF4/HDF-EOS2
  companion effort, not by widening the main HDF5 implementation into a general multi-format
  science container library.

---

## 3. Complex Data Type Dependency (ALOS-2 Blocker)

### Current State

`wbraster::DataType` enum currently supports only real-valued types:

```rust
pub enum DataType {
    F32, F64, I8, U8, I16, U16, I32, U32, I64, U64,
    // Missing: ComplexF32, ComplexF64
}
```

### Why This Matters

ALOS-2 PALSAR-2 MLC products store **complex-valued coherency matrices**:
- Each element is `ComplexF32` or `ComplexF64` (I + Q components)
- Sentinel-1 cross-pol phase information is also inherently complex
- Interferometric products (coherence, phase) are complex-valued

Without `ComplexF32`/`ComplexF64` support in `wbraster`, even if `wbhdf` successfully
reads the HDF5 file, construction of a `Raster` will fail with a data type mismatch.

### Recommendation

**ALOS-2 HDF5 reading should be added to this roadmap as Phase 2, but its completion
depends on a **separate architectural decision** in `wbraster` to support complex data types.**

Once complex types are available in `wbraster`, extending `wbhdf` to read ALOS-2 MLC
files is trivial (additional 3–5 days of work). The HDF5 format reading is straightforward;
the blocker is the raster data model.

### Related Scope Items

Complex data type support in `wbraster` would also enable:
- Interferometric SAR products (phase, coherence)
- Complex spectral unmixing residuals (if modeled as complex)
- Waveform-domain lidar decomposition (amplitude + phase)

MODIS/VIIRS support would also enable:
- Long-horizon vegetation monitoring workflows spanning MODIS-to-VIIRS continuity
- Thermal and land-surface teaching workflows without external format conversion
- Repeatable moderate-resolution time-series analyses inside Whitebox-native pipelines

---

## 4. Architecture: Scoped Reader Implementation

### Option A: `wbraster/src/formats/hdf5.rs` (integrated)

**Pros:**
- `RasterFormat::Hdf5` variant, fits existing format enum
- Automatic GDAL-style format detection
- Single dependency graph

**Cons:**
- Couples format reading into `wbraster`; HDF5 is used by non-raster tools (wblidar point clouds)
- Larger `wbraster` binary; complexity creep
- Harder to reuse cleanly from other crates

### Option B: Separate `wbhdf` crate (recommended)

**Pros:**
- Reusable by `wbraster`, `wblidar`, future tools (wblidar point ingest, wbgeospectral endmember libraries, etc.)
- Cleaner dependency boundary
- Can be versioned/released independently
- Simpler for community contributions / peer review
- Pure-Rust HDF5 primitive could have academic interest outside WNG

**Cons:**
- Additional crate to maintain
- Slightly more boilerplate in format dispatch and crate wiring

**Decision:** Option B. Create `wbhdf` as a focused, peer-reviewable library.

Integration policy:
- During early implementation, temporary gating may be used only for stabilization.
- After validation, HDF support is wired in as a standard integration path in both
  `wblidar` and `wbraster` (no long-term optional feature requirement for users).
- MODIS/HDF4-EOS2 support should live in a targeted module path under `wbhdf`, with clean
  internal boundaries from HDF5-first code.

Crate naming guidance:
- The project should adopt `wbhdf` from the outset as the primary crate name.
- Use module boundaries within `wbhdf` to keep HDF5-first implementation work scoped while
  preserving a format-inclusive public identity.

### Canonical Dataset URI Contract

The reader surface needs one stable dataset-addressing format shared by `wbraster`, `wblidar`,
and future bindings.

**Canonical form:**
- `container_path#dataset=/absolute/path/inside/container`

**Examples:**
- `tile.h5#dataset=/GEDI04_B/BEAM0000/geolocation/lat_lowestmode`
- `VNP09.A2024123.h10v04.001.nc#dataset=/HDFEOS/GRIDS/VNP_Grid_1km_2D/Data Fields/SurfReflect_M1`
- `MOD09A1.A2024121.h10v04.061.hdf#dataset=/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01`

**Contract rules:**
- Dataset paths are absolute and begin with `/`.
- URL query syntax (for example `?dataset=`) is not canonical and should not be emitted by docs.
- Support legacy aliases temporarily in parsing only; normalize to canonical form in diagnostics.
- Keep this contract stable across releases to preserve script/tool compatibility.
- If escaping is required for uncommon characters, use percent-encoding only inside the
  `dataset` value.

**Error contract:**
- Missing dataset selector -> `MissingDatasetSelector`
- Dataset path not found -> `DatasetPathNotFound`
- Container parsed but unsupported layout -> `UnsupportedLayout`
- Required filter unavailable -> `UnsupportedFilter`

### Module Structure for `wbhdf`

```
wbhdf/
  src/
    lib.rs                  — Public API (Reader trait, error types)
    superblock.rs           — HDF5 file superblock + root group discovery
    object_header.rs        — Object header message parsing (HIGH COMPLEXITY)
    btree.rs                — B-tree v1 chunk lookup (HIGH COMPLEXITY)
    dataset.rs              — Dataset class, chunk assembly
    filters.rs              — GZIP decompression (via miniz_oxide)
    datatypes.rs            — F32, F64, I16, strings (fixed schema)
    attributes.rs           — Metadata reading (group + dataset attributes)
  tests/
    fixtures/               — Real GEDI/ICESat-2 file samples (check in as binary or download in CI)
    integration_tests.rs    — Round-trip read → raster conversion
  docs/
    DESIGN.md               — B-tree traversal algorithm, chunk layout
    FORMAT_NOTES.md         — HDF5 spec excerpts, gotchas we hit
  Cargo.toml                — Pure Rust deps only (miniz_oxide, byteorder, etc.)
```

### Dependency Policy

**Allowed:**
- `miniz_oxide` (pure Rust GZIP)
- `byteorder` (byte utilities)
- `nalgebra` (if needed for coordinate transforms in GEDI/ICESat-2 geolocation)
- Standard library only

**Not allowed:**
- `hdf5` crate (C linkage)
- `libhdf5` system dependency
- SZIP (proprietary; skip for now)

---

## 5. Implementation Roadmap

### Phase 1: Superblock + Object Header Message Parsing (~1 week)

**Deliverables:**
- [x] Superblock reader (file signature, root object header address)
- [x] Object header message dispatch (attribute, dataspace, datatype, storage layout messages)
- [x] Basic datatype descriptors (float32, float64, int16, fixed-length strings)

**Validation:**
- Read file structure of a simple GEDI tile without error
- Extract dataset names and dimensions

### Phase 1b: HDF4/HDF-EOS2 Feasibility Slice for MODIS (~3-5 days)

**Deliverables:**
- [x] Confirm initial MODIS targets: `MOD09`/`MYD09`, `MOD13`/`MYD13`, and `MOD11`/`MYD11` family layouts.
  Evidence (2026-05-31): local fixture intake now includes `MOD09A1`/`MYD09A1`, `MOD13A1`/`MYD13A1`, and `MOD11A2`/`MYD11A2` variants under the MODIS fixture directory.
- [x] Prototype minimal HDF4 container parsing and SDS/grid enumeration on one representative MODIS product.
  Evidence (2026-05-31): added `wbhdf::hdf4::probe_hdf4_eos_metadata_in_file` that validates HDF4 signature and enumerates structured EOS metadata (`GridName`, `DataFieldName`, `DataType`, `DimList`, plus grid dimension sizes from `XDim=`/`YDim=` tokens) from embedded `StructMetadata.0` text; parser now also captures grid georeferencing metadata (`UpperLeftPointMtrs`, `LowerRightMtrs`, `Projection`, `ProjParams`, `SphereCode`) for MODIS grid context; added `resolve_hdf4_grid_field(...)`, `resolve_hdf4_dataset_path(...)`, and `enumerate_hdf4_dataset_paths(...)` to resolve explicit grid/field pairs and canonical dataset paths (`/GridName/DataFieldName`) into inferred field shapes and dispatch-ready path inventories; fixture-backed integration tests verify expected enumerations, inferred shapes, and georeferencing tokens for `MOD09A1`, `MOD11A2`, and `MOD13A1` samples.
  Current boundary: this is metadata-level HDF4 feasibility probing only; it does not yet decode SDS payload arrays.

- [x] Add first SDS payload-read bridge milestone with deterministic unsupported diagnostics.
  Evidence (2026-05-31): added `prepare_hdf4_sds_decode_attempt(...)` and
  `decode_hdf4_sds_i16_in_file(...)` in `wbhdf::hdf4`; canonical MODIS-style SDS paths now
  resolve into explicit decode-attempt descriptors and return stable `UnsupportedLayout` errors
  carrying resolved context (grid, field, data type, shape, projection) when payload-byte decode
  is not yet available. Fixture-backed integration test verifies this behavior on
  `MOD09A1` `sur_refl_b01`.

- [x] Derive raster geometry metadata from parsed MODIS HDF4 grid descriptors.
  Evidence (2026-05-31): added `derive_hdf4_grid_geometry(...)` and
  `derive_hdf4_grid_geometry_for_dataset(...)` in `wbhdf::hdf4` to compute rows/cols,
  pixel spacing, and GDAL-style geotransform from inferred shape +
  `UpperLeftPointMtrs`/`LowerRightMtrs`; fixture-backed tests verify expected geometry values for
  `MOD09A1`, `MOD11A2`, and `MOD13A1` resolved fields.

- [x] Add structured SDS decode readiness reporting for preflight gating.
  Evidence (2026-05-31): added `assess_hdf4_sds_i16_decode_readiness(...)` in `wbhdf::hdf4`.
  Callers now receive resolved field metadata, optional derived geometry, and explicit blocker
  reasons as structured data instead of relying only on string parsing from decode errors.
  Fixture-backed MOD09 readiness assertions confirm that only the backend-not-implemented blocker
  remains for `sur_refl_b01`.

- [x] Parse HDF4 descriptor tables and surface payload-candidate blockers from file contents.
  Evidence (2026-05-31): added DD traversal (`parse_hdf4_data_descriptors*`) and
  expected-length i16 candidate discovery (`find_hdf4_sds_i16_payload_candidates*`) for canonical
  SDS paths. Added in-file readiness API (`assess_hdf4_sds_i16_decode_readiness_in_file`) that
  reports descriptor-scan-derived blockers (no matching candidates vs mapping-not-implemented).
  Fixture-backed MOD09 tests confirm descriptor enumeration and deterministic candidate-blocker
  reporting.

- [x] Add ranked descriptor-candidate inspection to guide next mapping work.
  Evidence (2026-05-31): added `rank_hdf4_sds_i16_payload_candidates*` with candidate scoring by
  payload-length delta plus signature hints (`gzip`/`zlib`/`ascii`/`binary`) and preview bytes.
  In-file readiness now appends nearest-descriptor context into blockers when exact-size matches
  are absent, improving actionable diagnostics for MOD09 SDS mapping follow-up.

- [x] Add heuristic descriptor-to-field mapping layer for SDS pre-decode targeting.
  Evidence (2026-05-31): added `map_hdf4_sds_i16_descriptor_heuristic*` that selects a
  provisional descriptor mapping using deterministic priority (exact-length > compressed-like
  nearest > nearest overall) and exposes rationale + considered-candidate count; in-file
  readiness now appends heuristic mapping summaries to blockers when exact-size candidate mapping
  is unresolved.

- [x] Add bounded payload probe over heuristic-selected descriptor.
  Evidence (2026-05-31): added `probe_hdf4_sds_i16_payload_window*` to classify selected
  descriptor payloads (`compressed_payload`, `textual_payload`, `decoded_preview`, etc.) and to
  emit bounded little-endian/big-endian i16 previews for binary payload candidates. Readiness and
  unsupported-decode diagnostics now include probe status/rationale and preview context.

- [x] Add bounded compressed-payload probe decode for heuristic-selected candidates.
  Evidence (2026-05-31): probe path now attempts bounded `gzip`/`zlib` decompression on
  compressed-signature descriptors and emits i16 previews from decompressed bytes when successful;
  otherwise returns deterministic compressed-status diagnostics with decode failure rationale.

- [x] Add bounded i16 window decode-attempt API from probe-selected payloads.
  Evidence (2026-05-31): added `attempt_decode_hdf4_sds_i16_window_in_file(...)` to return
  little-endian i16 preview windows when probe decode succeeds and to return deterministic
  unsupported diagnostics with probe `status` + `rationale` when decode remains unavailable.

- [x] Add heuristic mapping confidence labels for MODIS decode targeting.
  Evidence (2026-05-31): `map_hdf4_sds_i16_descriptor_heuristic*` now reports confidence
  (`high`/`medium`/`low`/`none`) and readiness blockers include the confidence label so the
  provisional nature of candidate selection is explicit in diagnostics.

- [x] Allow the public MODIS/HDF4 decode entrypoint to return a probe preview window.
  Evidence (2026-05-31): `decode_hdf4_sds_i16_in_file(...)` now returns the mapped probe's
  little-endian preview when available, so the first real bounded window can surface through the
  user-facing decode API instead of only through the probe helper.

- [x] Prefer the more plausible preview endianness for MODIS/HDF4 decode returns.
  Evidence (2026-05-31): the public decode path now compares little-endian and big-endian probe
  previews with a lightweight plausibility score and returns the more realistic one when both are
  available.

- [x] Add start-offset bounded SDS window decode and route window attempts through it.
  Evidence (2026-05-31): added `decode_hdf4_sds_i16_window_at_in_file(path, dataset_path,
  start_value, max_values)` with bounded descriptor-selected payload decode (raw and
  gzip/zlib-backed) and start-offset i16 window extraction; `attempt_decode_hdf4_sds_i16_window_in_file(...)`
  now delegates to this API for centralized behavior.
  Follow-up evidence (2026-05-31): fixture-backed integration test
  `modis_mod09_hdf4_offset_window_decode_is_exercised` now exercises both start=0 and start=4
  decode attempts for MOD09 `/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01`, asserting either
  bounded decoded windows or deterministic status-bearing diagnostics.

- [x] Add deterministic out-of-bounds diagnostics for offset-window decode starts.
  Evidence (2026-05-31): `decode_hdf4_sds_i16_window_at_in_file(...)` now has fixture-backed
  integration coverage (`modis_mod09_hdf4_offset_window_decode_reports_out_of_bounds_start`) that
  asserts invalid-input diagnostics contain an out-of-bounds message when start indices exceed
  decoded payload length.

- [x] Strengthen MOD09 candidate-discovery payload assertions.
  Evidence (2026-05-31): `modis_mod09_hdf4_descriptor_enumeration_and_candidate_discovery` now
  compares successful `attempt_decode_hdf4_sds_i16_window_in_file(...)` results against the probe
  previews for `/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01`, so the candidate-discovery path
  checks actual payload windows rather than just descriptor-ranking metadata.

- [x] Add a real payload-window assertion for MOD13 NDVI.
  Evidence (2026-05-31): `modis_mod13_hdf4_eos_metadata_probe_enumerates_expected_fields` now
  runs `probe_hdf4_sds_i16_payload_window_in_file(...)` and
  `attempt_decode_hdf4_sds_i16_window_in_file(...)` on
  `/MODIS_Grid_16DAY_500m_VI/500m 16 days NDVI`, comparing successful windows against the probe
  previews and preserving structured diagnostics when decode is still blocked.

- [x] Add a real payload-window assertion for MOD11 LST_Day_1km.
  Evidence (2026-05-31): `modis_mod11_hdf4_eos_metadata_probe_enumerates_expected_fields` now
  runs `probe_hdf4_sds_i16_payload_window_in_file(...)` and
  `attempt_decode_hdf4_sds_i16_window_in_file(...)` on
  `/MODIS_Grid_8Day_1km_LST/LST_Day_1km`, comparing successful windows against the probe
  previews and preserving structured diagnostics when decode is still blocked.

- [x] Add report-based documented field-vocabulary diagnostics for MODIS core fixtures.
  Evidence (2026-06-01): fixture-backed integration tests
  `modis_mod09_documented_field_vocabulary_is_discoverable_with_reports`,
  `modis_mod11_documented_field_vocabulary_is_discoverable_with_reports`, and
  `modis_mod13_documented_field_vocabulary_is_discoverable_with_reports` now validate
  documented grid/field vocabulary discoverability with explicit present/missing diagnostics
  via `dataset_metadata_text_report_in_file(...)` for MOD09/MOD11/MOD13 reference datasets.

- [x] Extend report-based documented field-vocabulary diagnostics to Aqua companion fixtures.
  Evidence (2026-06-01): fixture-backed integration tests
  `myd09_documented_field_vocabulary_is_discoverable_with_reports`,
  `myd11_documented_field_vocabulary_is_discoverable_with_reports`, and
  `myd13_documented_field_vocabulary_is_discoverable_with_reports` now validate
  documented grid/field vocabulary discoverability with explicit present/missing diagnostics
  via `dataset_metadata_text_report_in_file(...)` for MYD09/MYD11/MYD13 companion datasets.

- [x] Add VIIRS HDF4 swath metadata enumeration coverage.
  Evidence (2026-05-31): `viirs_vnp09_hdf4_eos_metadata_probe_enumerates_expected_fields` now
  probes the HDF4 `VNP09_NRT` swath fixture and verifies the embedded EOS metadata enumerates the
  expected reflectance and quality field names (`375m Surface Reflectance Band I1`,
  `375m Surface Reflectance Band I2`, `375m Surface Reflectance Band I3`,
  `750m Surface Reflectance Band M1`, `750m Surface Reflectance Band M2`,
  `750m Surface Reflectance Band M3`, `750m Surface Reflectance Band M4`,
  `750m Surface Reflectance Band M5`, `750m Surface Reflectance Band M7`,
  `750m Surface Reflectance Band M8`, `750m Surface Reflectance Band M10`,
  `750m Surface Reflectance Band M11`,
  `land_water_mask`, `QF1 Surface Reflectance`, `QF2 Surface Reflectance`,
  `QF3 Surface Reflectance`, `QF4 Surface Reflectance`, `QF5 Surface Reflectance`,
  `QF6 Surface Reflectance`, `QF7 Surface Reflectance`), keeping VIIRS HDF4 fixture coverage
  aligned even

- [x] Add report-based documented swath-vocabulary diagnostics for VNP09.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp09_documented_swath_vocabulary_is_discoverable_with_reports` now validates
  documented I-band/M-band/QF vocabulary discoverability using
  `dataset_metadata_text_report_in_file(...)` with swath metadata token anchors and
  explicit present/missing diagnostics.
  though the current grid-field payload decoder does not yet model swath paths.

- [x] Add report-based documented field-vocabulary diagnostics for VIIRS M3/I4 fixtures.
  Evidence (2026-06-01): fixture-backed integration tests
  `viirs_m3_documented_field_vocabulary_is_discoverable_with_reports` and
  `viirs_i4_documented_field_vocabulary_is_discoverable_with_reports` now validate
  documented science/geolocation vocabulary discoverability with explicit present/missing
  diagnostics via `dataset_metadata_text_report_in_file(...)`.

- [x] Add report-based documented field-vocabulary diagnostics for ATL08/GEDI fixture probes.
  Evidence (2026-06-01): fixture-backed integration tests
  `atl08_documented_field_vocabulary_is_discoverable_with_reports` and
  `gedi_documented_field_vocabulary_is_discoverable_with_reports` now validate
  canonical beam/segment/science vocabulary discoverability with explicit
  present/missing diagnostics via `dataset_metadata_text_report_in_file(...)`.

- [x] Add VNP21 NetCDF/HDF5 swath discoverability coverage.
  Evidence (2026-05-31): `viirs_vnp21_netcdf_metadata_probe_discovers_swath_group_and_lst_path` now
  verifies the new `VNP21_NRT` sample exposes the `VIIRS_Swath_LSTE` top-level group and that the
  LST, LST_err, Emis_14, Emis_14_err, Emis_15, Emis_15_err, Emis_16, Emis_16_err,
  Emis_ASTER, latitude, longitude, and View_angle dataset paths are discoverable through the
  existing HDF5 path-marker heuristic.

- [x] Add VNP21 LST payload-window decode coverage through chunked swath storage.
  Evidence (2026-05-31): fixture-backed integration test
  `viirs_vnp21_lst_row_major_window_matches_h5dump_reference` now validates a bounded
  `/VIIRS_Swath_LSTE/Data Fields/LST` window (`start=(800,1600), count=(2,4)`) against
  `h5dump` reference values via the reusable `decode_chunked_u16_row_major_window_in_file(...)`
  helper and a hardened multi-level v1 chunk-index traversal path.

- [x] Add VNP21 LST_err payload-window + semantic normalization coverage.
  Evidence (2026-05-31): fixture-backed integration test
  `viirs_vnp21_lst_err_row_major_window_and_semantics_match_h5dump_reference` now validates a
  bounded `/VIIRS_Swath_LSTE/Data Fields/LST_err` window (`start=(800,1600), count=(2,4)`) against
  `h5dump` reference values via the reusable `decode_chunked_u8_row_major_window_in_file(...)`
  helper and asserts attribute-driven semantics (`_FillValue=0`, `valid_range=[1,255]`,
  `scale_factor=0.04`, `add_offset=0`) on decoded values.

- [x] Add VNP21 View_angle payload-window + semantic normalization coverage.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_view_angle_row_major_window_and_semantics_match_h5dump_reference` now validates a
  bounded `/VIIRS_Swath_LSTE/Data Fields/View_angle` non-origin window
  (`start=(1234,987), count=(2,6)`) against `h5dump` reference values via the reusable
  `decode_chunked_u8_row_major_window_in_file(...)` helper and asserts attribute-driven semantics
  (`_FillValue=255`, `valid_range=[0,180]`, `scale_factor=0.5`, `add_offset=0`) on decoded values.

- [x] Add VNP21 latitude/longitude payload-window decode coverage through chunked swath storage.
  Evidence (2026-06-01): fixture-backed integration tests
  `viirs_vnp21_latitude_row_major_window_matches_h5dump_reference` and
  `viirs_vnp21_longitude_row_major_window_matches_h5dump_reference` now validate bounded non-origin
  `/VIIRS_Swath_LSTE/Geolocation Fields/latitude` and
  `/VIIRS_Swath_LSTE/Geolocation Fields/longitude` windows (`start=(1234,987), count=(2,6)`) against
  `h5dump` reference values via the reusable `decode_chunked_f32_row_major_window_in_file(...)`
  helper, including tolerance-based comparison and valid-range sanity checks.

- [x] Add VNP21 Emis_ASTER payload-window + semantic normalization coverage.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_emis_aster_row_major_window_and_semantics_match_h5dump_reference` now validates a
  bounded `/VIIRS_Swath_LSTE/Data Fields/Emis_ASTER` window (`start=(800,1600), count=(2,4)`) against
  `h5dump` reference values via the reusable `decode_chunked_u8_row_major_window_in_file(...)`
  helper and asserts attribute-driven semantics (`_FillValue=0`, `valid_range=[1,255]`,
  `scale_factor=0.002`, `add_offset=0.49`) on decoded values.

- [x] Add VNP21 PWV payload-window + semantic normalization coverage.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_pwv_row_major_window_and_semantics_match_h5dump_reference` now validates a
  bounded non-origin `/VIIRS_Swath_LSTE/Data Fields/PWV` window (`start=(1234,987), count=(2,6)`)
  against `h5dump` reference values via the reusable `decode_chunked_u16_row_major_window_in_file(...)`
  helper and asserts attribute-driven scaling semantics (`valid_range=[0,65535]`,
  `scale_factor=0.001`, `add_offset=0`).

- [x] Add VNP21 QC payload-window coverage with unscaled QA semantics checks.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_qc_row_major_window_and_semantics_match_h5dump_reference` now validates a bounded
  non-origin `/VIIRS_Swath_LSTE/Data Fields/QC` window (`start=(1234,987), count=(2,6)`) and a
  complementary origin window (`start=(800,1600), count=(2,4)`) against
  `h5dump` reference values via the reusable `decode_chunked_u16_row_major_window_in_file(...)`
  helper, including unscaled QA semantics checks (`format=unscaled`,
  `valid_range=[0,65535]`, `scale_factor=1`, `add_offset=0`).

- [x] Add VNP21 oceanpix payload-window coverage with categorical QA semantics checks.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_oceanpix_row_major_window_and_semantics_match_h5dump_reference` now validates a bounded
  non-origin `/VIIRS_Swath_LSTE/Data Fields/oceanpix` window (`start=(1234,987), count=(2,6)`) and a
  complementary origin window (`start=(800,1600), count=(2,4)`) against
  `h5dump` reference values via the reusable `decode_chunked_u8_row_major_window_in_file(...)`
  helper, including categorical QA semantics checks (`format=unscaled`,
  `valid_range=[0,2]`, `scale_factor=1`, `add_offset=0`).

- [x] Add cross-field QA bit-pattern contract regression for VNP21 QC/oceanpix.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_qc_oceanpix_cross_field_bit_pattern_contract_is_stable` validates a stable
  two-window QA contract between `/VIIRS_Swath_LSTE/Data Fields/QC` and
  `/VIIRS_Swath_LSTE/Data Fields/oceanpix`, including (a) non-origin window pattern constraints,
  (b) complementary origin-window baseline patterns, and (c) stable observed QC bit-pattern masks.

- [x] Add observed QC bitfield-interpretation regression coverage for VNP21.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_qc_observed_bitfield_interpretation_matches_oceanpix_windows` now decodes
  observed QC bytes into explicit bit-level components (high byte and low bits 0/5/6/7), then
  validates stable non-origin and origin window contracts against paired oceanpix categories.

- [x] Add observed QC/oceanpix state-histogram contract regression for VNP21.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_qc_oceanpix_observed_state_histogram_contract_is_stable` now asserts stable
  paired-state counts across combined non-origin and origin windows for observed
  `(QC, oceanpix)` tuples, hardening fixture-level QA state-distribution expectations.

- [x] Add QC/oceanpix row-alignment contract regression for VNP21.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_qc_oceanpix_row_alignment_contract_is_stable` now asserts stable row-major
  row/column alignment patterns for paired `QC` and `oceanpix` windows (non-origin and origin),
  hardening ordering-sensitive decode behavior in addition to value/state checks.

- [x] Add additional non-origin QC/oceanpix window regression for VNP21.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_qc_oceanpix_additional_nonorigin_window_matches_h5dump_reference` now validates
  an additional bounded window (`start=(1000,1007), count=(2,10)`) for both
  `/VIIRS_Swath_LSTE/Data Fields/QC` and `/VIIRS_Swath_LSTE/Data Fields/oceanpix` against
  `h5dump` references, broadening observed QA-state coverage to include `QC=64192` in a real
  oceanpix=0 slice.

- [x] Add observed-bit semantics regression for the additional VNP21 oceanpix=0 slice.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_qc_observed_bits_in_additional_oceanpix0_window_are_stable` now decodes
  `QC` values from the additional bounded window (`start=(1000,1007), count=(2,10)`) into
  explicit observed bit components and asserts stable oceanpix=0 pairing semantics,
  including high-byte transitions (`0xFA`/`0xFE`) with stable low-byte/bit-state behavior.

- [x] Add cross-window observed-profile classification regression for VNP21 QC.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_qc_observed_profile_classification_is_stable_across_windows` now classifies
  observed QC byte patterns into stable fixture-level profiles and validates their paired
  oceanpix counts across multiple bounded windows.

- [x] Add origin-baseline-inclusive observed-profile classification regression for VNP21 QC.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_qc_observed_profile_classification_with_origin_baseline_is_stable` now extends
  profile/pairing checks to include the origin baseline window, tightening three-window
  fixture-level QC/oceanpix profile contracts.

- [x] Add deterministic known/unknown QC observed-profile classifier regression.
  Evidence (2026-06-01): integration test
  `viirs_vnp21_qc_observed_profile_classifier_maps_known_and_unknown_states` now validates
  stable profile mapping for known observed QC states and explicit unknown-state fallback,
  reducing risk of accidental classifier drift while QA contracts expand.

- [x] Add inland-water QC/oceanpix observed-profile regression for VNP21.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_qc_oceanpix_inland_window_profile_contract_is_stable` now validates an
  additional bounded inland-water window (`start=(1500,2500), count=(2,4)`) with
  `QC=7` and `oceanpix=1`, extending observed QA profile coverage beyond previously
  covered oceanpix categories.

- [x] Add exhaustive multi-window QC/oceanpix profile contract regression.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_qc_oceanpix_profile_contract_is_exhaustive_across_key_windows` now aggregates
  observed profile/oceanpix pairings across key bounded windows and asserts stable counts with
  explicit no-unknown-classification guardrails.

- [x] Add cross-window QC profile-to-bit invariant regression.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_qc_profile_bit_invariants_are_stable_across_key_windows` now enforces stable
  bit-level semantics (`high_byte`, `low_byte`, and observed low-bit flags) for each known
  fixture profile across the key bounded windows, with explicit unknown-profile rejection.

- [x] Add raw QC-state whitelist-by-category regression across key VNP21 windows.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_qc_raw_state_whitelist_by_oceanpix_is_stable_across_key_windows` now enforces a
  stable whitelist of observed raw `QC` values for each `oceanpix` category and rejects category
  drift outside the validated key-window set.

- [x] Add non-overlapping QC/oceanpix profile-cluster regression for VNP21.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_qc_nonoverlapping_window_cluster_profile_contract_is_stable` now validates a
  second bounded window cluster with stable inland-known (`QC=7`, `oceanpix=1`) and land-unknown
  (`QC=15/50`, `oceanpix=0`) profile behavior, extending drift detection beyond prior key windows.

- [x] Add documented QA/category vocabulary discoverability contract for VNP21.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_qc_oceanpix_documented_semantics_vocabulary_is_discoverable_and_consistent` now
  asserts discoverability of documented metadata vocabulary (`"Quality Control for LST and
  emissivity"`, `"land ocean inland_water"`, `"ocean pixels"`) and verifies bounded-window
  `oceanpix` observations remain consistent with the full documented value space (`0..2`).

- [x] Add documented QC vocabulary + observed bitfield-family consistency contract.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_qc_documented_semantics_vocabulary_and_observed_bitfield_families_are_consistent`
  now couples documented QC metadata vocabulary discoverability with stable bounded-window
  high/low-byte family expectations for observed QC states.

- [x] Add reusable dataset metadata-text assertion utility and migrate spec-aligned VNP21 checks.
  Evidence (2026-06-01): added `wbhdf::attributes::{dataset_metadata_contains_text_in_file,
  dataset_metadata_contains_all_texts_in_file}` and migrated documented-semantics VNP21 tests to
  use utility-backed assertions rather than direct fixture-byte scans.

- [x] Add missing-term metadata utility and upgrade VNP21 diagnostic assertions.
  Evidence (2026-06-01): added
  `wbhdf::attributes::dataset_metadata_missing_texts_in_file` and migrated spec-aligned VNP21
  assertions to require empty missing-term lists, improving failure diagnostics with explicit
  missing-vocabulary reporting.

- [x] Add report-style metadata utility with present/missing vocabulary breakdown.
  Evidence (2026-06-01): added
  `wbhdf::attributes::dataset_metadata_text_report_in_file` and upgraded spec-aligned VNP21
  semantics assertions to report both present and missing terms in failure messages.

- [x] Add VNP21 Emis_14 payload-window + semantic normalization coverage.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_emis14_row_major_window_and_semantics_match_h5dump_reference` now validates a
  bounded `/VIIRS_Swath_LSTE/Data Fields/Emis_14` window (`start=(800,1600), count=(2,4)`) against
  `h5dump` reference values via the reusable `decode_chunked_u8_row_major_window_in_file(...)`
  helper and asserts attribute-driven semantics (`_FillValue=0`, `valid_range=[1,255]`,
  `scale_factor=0.002`, `add_offset=0.49`) on decoded values.

- [x] Add VNP21 Emis_15 payload-window + semantic normalization coverage.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_emis15_row_major_window_and_semantics_match_h5dump_reference` now validates a
  bounded `/VIIRS_Swath_LSTE/Data Fields/Emis_15` window (`start=(800,1600), count=(2,4)`) against
  `h5dump` reference values via the reusable `decode_chunked_u8_row_major_window_in_file(...)`
  helper and asserts attribute-driven semantics (`_FillValue=0`, `valid_range=[1,255]`,
  `scale_factor=0.002`, `add_offset=0.49`) on decoded values.

- [x] Add VNP21 Emis_16 payload-window + semantic normalization coverage.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_emis16_row_major_window_and_semantics_match_h5dump_reference` now validates a
  bounded `/VIIRS_Swath_LSTE/Data Fields/Emis_16` window (`start=(800,1600), count=(2,4)`) against
  `h5dump` reference values via the reusable `decode_chunked_u8_row_major_window_in_file(...)`
  helper and asserts attribute-driven semantics (`_FillValue=0`, `valid_range=[1,255]`,
  `scale_factor=0.002`, `add_offset=0.49`) on decoded values.

- [x] Add VNP21 Emis_14_err payload-window + semantic normalization coverage.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_emis14_err_row_major_window_and_semantics_match_h5dump_reference` now validates a
  bounded `/VIIRS_Swath_LSTE/Data Fields/Emis_14_err` window (`start=(800,1600), count=(2,4)`)
  against `h5dump` reference values via the reusable
  `decode_chunked_u16_row_major_window_in_file(...)` helper and asserts attribute-driven semantics
  (`_FillValue=0`, `valid_range=[1,65535]`, `scale_factor=0.0001`, `add_offset=0`) on decoded
  values.

- [x] Add VNP21 Emis_15_err payload-window + semantic normalization coverage.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_emis15_err_row_major_window_and_semantics_match_h5dump_reference` now validates a
  bounded `/VIIRS_Swath_LSTE/Data Fields/Emis_15_err` window (`start=(800,1600), count=(2,4)`)
  against `h5dump` reference values via the reusable
  `decode_chunked_u16_row_major_window_in_file(...)` helper and asserts attribute-driven semantics
  (`_FillValue=0`, `valid_range=[1,65535]`, `scale_factor=0.0001`, `add_offset=0`) on decoded
  values.

- [x] Add VNP21 Emis_16_err payload-window + semantic normalization coverage.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_emis16_err_row_major_window_and_semantics_match_h5dump_reference` now validates a
  bounded `/VIIRS_Swath_LSTE/Data Fields/Emis_16_err` window (`start=(800,1600), count=(2,4)`)
  against `h5dump` reference values via the reusable
  `decode_chunked_u16_row_major_window_in_file(...)` helper and asserts attribute-driven semantics
  (`_FillValue=0`, `valid_range=[1,65535]`, `scale_factor=0.0001`, `add_offset=0`) on decoded
  values.

- [x] Harden VNP21 Emis_14_err coverage with nonzero-column and fill-window slices.
  Evidence (2026-06-01): fixture-backed integration tests
  `viirs_vnp21_emis14_err_nonzero_column_window_matches_h5dump_reference` and
  `viirs_vnp21_emis14_err_fill_window_matches_h5dump_reference` now validate additional
  `/VIIRS_Swath_LSTE/Data Fields/Emis_14_err` windows (`start=(1234,987), count=(2,6)` and
  `start=(1500,2500), count=(2,4)`) against `h5dump` references, covering both non-origin
  nonzero data variation and explicit `_FillValue=0` window behavior.

- [x] Harden VNP21 Emis_15_err coverage with non-origin and fill-window slices.
  Evidence (2026-06-01): fixture-backed integration tests
  `viirs_vnp21_emis15_err_nonzero_column_window_matches_h5dump_reference` and
  `viirs_vnp21_emis15_err_fill_window_matches_h5dump_reference` now validate additional
  `/VIIRS_Swath_LSTE/Data Fields/Emis_15_err` windows (`start=(1234,987), count=(2,6)` and
  `start=(1500,2500), count=(2,4)`) against `h5dump` references, covering both non-origin
  non-fill data and explicit `_FillValue=0` window behavior.

- [x] Harden VNP21 Emis_16_err coverage with non-origin and fill-window slices.
  Evidence (2026-06-01): fixture-backed integration tests
  `viirs_vnp21_emis16_err_nonzero_column_window_matches_h5dump_reference` and
  `viirs_vnp21_emis16_err_fill_window_matches_h5dump_reference` now validate additional
  `/VIIRS_Swath_LSTE/Data Fields/Emis_16_err` windows (`start=(1234,987), count=(2,6)` and
  `start=(1500,2500), count=(2,4)`) against `h5dump` references, covering both non-origin
  non-fill data and explicit `_FillValue=0` window behavior.

- [x] Add cross-field semantics consistency coverage for VNP21 emissivity error layers.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_emis_err_cross_field_semantics_contract_is_consistent` validates a shared contract
  across `Emis_14_err`, `Emis_15_err`, and `Emis_16_err`: non-fill windows include expected raw and
  scaled values with `valid_range` lower-bound compliance, while explicit fill windows decode as
  `_FillValue=0` and remain zero after applying `scale_factor=0.0001`.

- [x] Add cross-field semantics consistency coverage for VNP21 emissivity science layers.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_emis_cross_field_semantics_contract_is_consistent` validates a shared contract
  across `Emis_14`, `Emis_15`, and `Emis_16`: non-fill windows include expected raw and scaled
  values with `valid_range` lower-bound compliance, while explicit fill windows decode as
  `_FillValue=0` and map to `add_offset` under `scale_factor=0.002`.

- [x] Consolidate cross-field semantics assertions behind reusable test helpers.
  Evidence (2026-06-01): `integration_tests.rs` now routes both emissivity-science and
  emissivity-error cross-field consistency checks through shared helper assertions, reducing
  duplicated decode/scale/fill contract logic while preserving fixture-backed behavior.

- [x] Generalize semantics helper assertions for metadata-driven field contracts.
  Evidence (2026-06-01): helper assertions now parameterize `valid_range` lower bound,
  `scale_factor`, and `add_offset`, allowing emissivity, emissivity-error, and thermal
  cross-field tests to share one decode/semantic contract path with dataset-specific metadata
  expectations.

- [x] Convert cross-field semantics checks to table-driven case lists.
  Evidence (2026-06-01): cross-field tests now declare field expectations as case rows and route
  them through shared case runners, reducing per-test boilerplate and making future field additions
  a single-row change.

- [x] Add cross-field semantics consistency coverage for VNP21 thermal layers.
  Evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp21_thermal_cross_field_semantics_contract_is_consistent` validates shared contracts
  across `LST` and `LST_err`: non-fill windows include expected raw and scaled values with
  lower-bound valid-range compliance, while explicit fill windows decode as `_FillValue=0` and
  remain zero after their corresponding scale factors.

- [x] Make HDF5 signature probing user-block aware.
  Evidence (2026-05-31): `probe_file_metadata(...)` now finds HDF5 signatures within the first
  4 KiB instead of assuming byte 0, which allows the new VIIRS HDF5 fixtures with 1 KiB user
  blocks to report the correct superblock version and top-level group markers.

- [x] Add VIIRS HDF5 science-path discoverability coverage.
  Evidence (2026-05-31): `viirs_m3_hdf5_fixture_discovers_science_paths` and
  `viirs_i4_hdf5_fixture_discovers_science_paths` now verify the new VIIRS HDF5 fixtures expose
  the expected science dataset paths (`/All_Data/VIIRS-M3-SDR_All/Radiance`,
  `/All_Data/VIIRS-M3-SDR_All/Reflectance`, `/All_Data/VIIRS-I4-IMG-EDR_All/BrightnessTemperature`,
  `/All_Data/VIIRS-I4-IMG-EDR_All/Radiance`) through the user-block-aware superblock probe and
  existing path-marker discovery helper.

- [x] Add a second GEDI granule discoverability check.
  Evidence (2026-05-31): `gedi_new_granule_fixture_discovers_beam_and_elev_paths` now probes the
  new `GEDI02_A_2025190205730_O37237_01_T04940_02_004_02_V002.h5` granule and verifies the
  `BEAM0000` beam plus `shot_number` and `elev_lowestmode` path markers resolve through the
  existing HDF5 metadata helper.

- [x] Add Aqua companion HDF4 smoke coverage.
  Evidence (2026-05-31): `myd09_hdf4_eos_metadata_probe_and_payload_window_are_exercised`,
  `myd11_hdf4_eos_metadata_probe_and_payload_window_are_exercised`, and
  `myd13_hdf4_eos_metadata_probe_and_payload_window_are_exercised` now exercise the new Aqua
  companions to MOD09/MOD11/MOD13, proving the HDF4 metadata helper and payload-window probe can
  discover the expected grid/field names on the MYD fixtures.

- [x] Add deterministic invalid-input diagnostics for zero-length offset-window requests.
  Evidence (2026-05-31): fixture-backed integration coverage
  (`modis_mod09_hdf4_offset_window_decode_rejects_zero_max_values`) now asserts that
  `decode_hdf4_sds_i16_window_at_in_file(...)` returns invalid-input diagnostics for
  `max_values=0` requests.

- [x] Add deterministic oversized-compressed-candidate diagnostics for offset-window decode.
  Evidence (2026-05-31): synthetic unit coverage now asserts
  `decode_hdf4_sds_i16_window_at_in_file(...)` returns
  `status=compressed_payload` diagnostics when a selected compressed descriptor exceeds
  `MAX_COMPRESSED_WINDOW_DECODE_BYTES`.

- [x] Reduce heuristic dependence for MODIS offset-window decode using deterministic exact-candidate ordering.
  Evidence (2026-05-31): `decode_hdf4_sds_i16_window_at_in_file(...)` now attempts exact-length
  in-bounds descriptors in deterministic preference order before heuristic fallback; unit coverage
  verifies that binary exact candidates are selected over textual exact candidates when both are
  present.

- [x] Add explicit exact-candidate attempt diagnostics for decode failures.
  Evidence (2026-05-31): exact-candidate decode failures now include
  `attempted=N/M` diagnostics for ranked exact-length descriptors; unit coverage verifies
  deterministic reporting when all exact candidates fail.
- [x] Document internal module boundaries for HDF5 and HDF4/HDF-EOS2 decoding within `wbhdf`.
  Evidence (2026-05-31): added internal design note `docs/internal/HDF_MODULE_BOUNDARIES.md` defining responsibilities, shared contracts, and integration boundaries for `superblock/object_header/btree/dataset/...` (HDF5 path) and `hdf4.rs` (HDF4/HDF-EOS2 path).

**Validation:**
- Enumerate SDS/grid metadata from one MODIS land product without external conversion.
- Confirm projected/grid metadata extraction path is practical for targeted support.

### Phase 2: B-tree v1 Chunk Indexing (~2 weeks, HIGH COMPLEXITY)

**Deliverables:**
- [ ] B-tree v1 node parsing (internal and leaf nodes)
- [ ] Chunk address lookup by dataset coordinates
- [ ] Handling of non-leaf B-tree traversal
- [ ] Validation against known chunk layouts from GEDI/ICESat-2 samples

**Validation:**
- Correctly locate all chunks for a 2D dataset
- Round-trip read: superblock → B-tree → chunk addresses → decompressed data matches known values

### Phase 3: Dataset Reading + GZIP Decompression (~1 week)

**Deliverables:**
- [ ] Chunk assembly (variable chunk size handling)
- [ ] GZIP filter invocation (via `miniz_oxide::deflate`)
- [ ] F32/F64/I16 endianness conversion (big-endian HDF5 standard)
- [ ] Nodata handling (HDF5 fill values)

**Validation:**
- Read a complete 2D float32 array from GEDI L2B
- Compare with reference output from `h5py` or `gdal_translate`

### Phase 4: Integration with `wbraster` and `wblidar` (~3-5 days)

**Deliverables:**
- [ ] `wblidar` dataset adapters for Tier 1 GEDI/ICESat-2 products (primary Tier 1 integration)
- [ ] `wbraster` dispatch layer for HDF5-backed raster datasets
- [ ] `Raster` construction from HDF5 datasets where raster semantics are valid
- [ ] GIS metadata propagation (CRS from attributes, georeferencing from dataset layouts)
- [ ] Standard integration path in crate defaults after stabilization (no long-term user-facing gate)

**Validation:**
- `wblidar` can ingest Tier 1 GEDI/ICESat-2 inputs directly (no pre-conversion)
- `wbraster::Raster::read("data.h5:///path/to/dataset")` succeeds for supported raster-like datasets
- Output dimensions and nodata/fill handling match reference

### Phase 5: Testing + Documentation (~1 week)

**Deliverables:**
- [ ] Unit tests for B-tree traversal (synthetic trees + GEDI sample validation)
- [ ] Integration tests: ingest GEDI/ICESat-2 Tier 1 paths through `wblidar` and validate decoded values against references
- [ ] Integration tests: read at least one VIIRS gridded product and one MODIS targeted product path
- [ ] Design documentation (B-tree algorithm, chunk layout assumptions, format notes)
  Progress note (2026-05-31): refreshed `crates/wbhdf/docs/DESIGN.md` from placeholder text to current architecture/state coverage, and added concrete B-tree/chunk design coverage (implemented structures, bounded chunk-read flow, routing rules, safety invariants, and chunked-`wbraster` integration sequence). This deliverable remains open until full multi-chunk assembly design details, key-format notes, and explicit test cross-links are fully documented.
- [ ] User guide: how to read GEDI/ICESat-2 via `wblidar` and raster-like HDF products via `wbraster`

### Phase 2 (Contingent): ALOS-2 PALSAR-2 MLC Support (~1 week)

**Prerequisites:**
- `wbraster` gains `ComplexF32` and `ComplexF64` variants in `DataType` enum
- `wbraster` I/O paths updated to handle complex values

**Deliverables:**
- [ ] `wbhdf` dataset reader extended to recognize complex HDF5 datatypes
- [ ] Mapping of HDF5 complex types → `wbraster` `ComplexF32`/`ComplexF64`
- [ ] Endianness conversion for complex components
- [ ] Integration tests: read ALOS-2 MLC tile → validate coherency matrix elements

**Timeline:** Begin after Phase 1 validation and once `wbraster` complex type support is available (estimated Q1 2027).

### Phase 2a (Prerequisite): Non-Breaking Complex DataType Enablement in `wbraster` (~1-3 weeks)

Goal: introduce complex-valued storage and access without breaking existing scalar APIs.

**Non-breaking principles:**
- Keep existing scalar APIs (`get`, `set`, `statistics`, nodata semantics) unchanged for real rasters.
- Add complex support via additive APIs and enum variants.
- Avoid changing existing function signatures used broadly by current tools.

**Deliverables:**
- [ ] Add `DataType::ComplexF32` and `DataType::ComplexF64`.
- [ ] Add corresponding `RasterData` storage variants.
- [ ] Add additive complex accessors (e.g., typed complex read/write methods) without removing scalar ones.
- [ ] Define complex-safe nodata/fill behavior for ingestion paths (for example, optional component-wise sentinels or explicit validity masks).
- [ ] Preserve existing behavior for all current real-valued formats and tools.

**Compatibility guidance:**
- Adding enum variants may require downstream wildcard/default arms in exhaustive matches.
- This is treated as controlled API expansion, not a semantic break of existing scalar workflows.

**Validation:**
- Existing `wbraster` tests for real-valued rasters continue to pass unchanged.
- New complex-specific tests validate storage, read/write accessors, and conversion boundaries.
- ALOS-2 HDF5 ingestion path can construct in-memory complex rasters without touching existing scalar tool behavior.

### Execution Cadence Checklist (Week 1)

Objective: start execution immediately with low-risk, test-first steps that preserve current APIs.

Process rule for implementation:
- [x] Update crate changelogs whenever code changes land in affected crates (`wbhdf`, `wblidar`, `wbraster`).

**Day 1: Repository and crate scaffolding**
- [x] Create `wbhdf` crate skeleton (`src/lib.rs`, `src/error.rs`, `src/superblock.rs`, `src/object_header.rs`, `src/dataset.rs`, `src/btree.rs`, `src/filters.rs`, `src/datatypes.rs`, `src/attributes.rs`).
- [x] Add crate-level docs stating strict scope (targeted decoder, GZIP-first, no full HDF5 claim).
- [x] Add initial unit-test harness and placeholder integration test file.

**Day 2: Real sample fixture strategy**
- [x] Add fixture manifest file documenting expected sample products and known dataset paths for GEDI/ICESat-2.
- [x] Extend fixture manifest planning to first VIIRS and MODIS target products.
- [x] Use fixture tiers: tiny committed fixtures (KB-MB), plus externally fetched integration fixtures (100MB+).
- [x] Use `HDF_FIXTURE_ACQUISITION_MATRIX.md` as the source-of-truth for source/auth/fallback fixture planning.
- [x] Add fixture-loading utilities that skip gracefully when large test assets are not present locally.
- [x] Add a minimal metadata smoke test target (open file, validate HDF5 signature, list top-level groups).

**Day 3: B-tree v1 implementation kickoff (Phase 2)**
- [x] Implement B-tree v1 node header parsing and typed internal/leaf node structs.
- [x] Add synthetic B-tree tests for traversal order and key range routing.
- [x] Implement chunk-address lookup API with deterministic error reporting.

**Day 4: Chunk lookup validation path**
- [x] Wire B-tree lookup into dataset chunk locator flow (no full decode yet).
- [x] Add tests that validate located chunk addresses against known reference expectations.
- [x] Add `FORMAT_NOTES` entries for any key-layout variations discovered.

**Day 5: Non-breaking complex enablement design stub (Phase 2a prep)**
- [x] Draft `wbraster` design note for additive complex API plan (`DataType` expansion, `RasterData` expansion, additive complex accessors only).
- [x] Enumerate all `DataType` match sites in `wbraster` and classify as: compile-only update, behavior-sensitive update, or deferred update.
- [x] Add compile-only placeholder tests proving existing scalar API usage remains unchanged.
- [x] Confirm `wbhdf` naming is applied consistently in docs, planning notes, and crate wiring.

**Week 1 exit criteria**
- [x] `wbhdf` crate builds and tests pass (including synthetic B-tree tests).
  Evidence (2026-05-31): `cargo test -p wbhdf` -> 13 tests passed (9 unit + 4 integration).
- [x] At least one GEDI/ICESat-2 sample can be opened and traversed through metadata + chunk address lookup path.
  Evidence (2026-05-31): `WBHDF_SMOKE_FILE=/Users/johnlindsay/Documents/data/spaceborne_lidar/ATL08_20181120185605_08120102_007_01.h5 cargo test -p wbhdf metadata_smoke_test_skips_gracefully_without_fixture -- --nocapture` passed against a real ATL08 fixture.
  Follow-up evidence (2026-05-31): `WBHDF_FIXTURE_DIR=/Users/johnlindsay/Documents/data/spaceborne_lidar cargo test -p wbhdf fixture_dir_smoke -- --nocapture` passed real ATL08 and GEDI metadata assertions.
  Follow-up evidence (2026-05-31): `WBHDF_FIXTURE_DIR=/Users/johnlindsay/Documents/data/spaceborne_lidar cargo test -p wbhdf object_header -- --nocapture` passed ATL08 v2 object-header prefix parsing, bounded dataspace/datatype body decoding, first-chunk message-header decoding (`0x01`, `0x03`, `0x05`, `0x10`), first continuation target decoding `(144130, 52)`, first continuation-chunk message-header decoding (`0x10`, `0x15`), chunk-2 contiguous layout decoding `(8500138, 38726)`, and GEDI `OHDR` marker discovery.
  Current boundary: real dataset-path discovery is now validated heuristically for ATL08 (`/gt1l/land_segments/canopy/h_canopy`) and GEDI (`/BEAM0000/shot_number`), and ATL08 first-chunk bodies plus first continuation-chain layout metadata are decodable, but true payload decode and broader message-body traversal remain Week 2 tasks.
- [x] A written non-breaking complex API plan is checked in and approved before touching `wbraster` public method signatures.
  Evidence (2026-05-31): `WBRASTER_COMPLEX_DATATYPE_DAY5_DESIGN.md` and `WBRASTER_DATATYPE_MATCH_SITE_AUDIT_DAY5.md` checked in.

### Execution Cadence Checklist (Week 2)

Objective: complete the first end-to-end decode path (chunk -> decompress -> typed values)
and land initial `wblidar` Tier 1 ingestion plumbing.

**Day 1: Phase 3 decode pipeline wiring**
- [x] Implement chunk read pipeline (`offset -> compressed bytes -> decompressed bytes -> typed decode`).
  Progress (2026-05-31): ATL08 contiguous-layout payload reads now work end-to-end for one real object-header chain (`data_address=8500138`, `data_size=38726`), the payload decodes as a UTF-8 fixed string beginning with the expected XML prolog, and the standalone GZIP filter decode path is now implemented and unit-tested.
  Progress (2026-05-31): Real ATL08 `h_canopy` bounded v1 object-header parsing is now implemented for the Day 1 decode prerequisites (dataspace, datatype, filter pipeline, chunked layout, continuation), with fixture-backed assertions at object-header address `328097` confirming deflate level `6`, chunk index address `326001`, chunk dimensions `{10000, 4}`, and continuation `(2011663, 112)`.
  Evidence (2026-05-31): ATL08 `h_canopy` first real chunk now decodes end-to-end through the bounded v1 chunk index path (`326001 -> chunk_address 9489637 -> zlib decompress -> 10000 little-endian `f32` values), validated by integration test coverage.
  Current boundary: this Day 1 pipeline is currently bounded to first-record leaf decoding for the observed v1 chunk-index shape; generalized non-leaf traversal and broader index variants remain follow-up work.
- [x] Add explicit endianness utilities for `F32`/`F64`/`I16` decoding from HDF5 payloads.
  Evidence (2026-05-31): `wbhdf::datatypes` now provides explicit `Endianness`-aware decode helpers for scalar and slice decoding of `F32`, `F64`, and `I16` payloads.
- [x] Add unit tests for decode correctness from synthetic little/big-endian byte fixtures.
  Evidence (2026-05-31): synthetic little-endian and big-endian decode coverage added for scalar and slice helpers in `crates/wbhdf/src/datatypes.rs`.

**Day 2: Fill value and nodata mapping**
- [x] Parse and apply HDF5 fill-value metadata during dataset materialization.
  Evidence (2026-05-31): bounded v1 fill-value message decoding is implemented in `wbhdf::object_header` and the decoded ATL08 `h_canopy` fill payload (`ff ff 7f 7f` -> little-endian `f32::MAX`) is now consumed by dataset-level fill mapping (`wbhdf::dataset::apply_fill_value_mapping_f32`) in the first real chunk decode path.
- [x] Define deterministic mapping from HDF5 fill values to consumer nodata semantics.
  Evidence (2026-05-31): deterministic mapping rule implemented for `f32`: compare by bit-pattern against fill sentinel, replace matching cells with explicit nodata sentinel (`-9999.0` in current tests), and report `valid_count` + `nodata_count`.
- [x] Add tests that verify fill handling and valid-cell counts against reference expectations.
  Evidence (2026-05-31): synthetic unit tests validate mapping semantics; fixture-backed ATL08 first-chunk test now asserts `valid_count=3640` and `nodata_count=6360` after fill mapping.
  Current boundary: mapping/materialization is currently bounded to the validated `f32` first-chunk path; generalized multi-chunk and additional datatype mappings remain follow-up work.

**Day 3: First reference-comparison harness**
- [x] Add comparison utility to validate decoded arrays against reference outputs (h5py/GDAL exports).
  Evidence (2026-05-31): `wbhdf::compare` now exposes `compare_f32_exact` and structured summary reporting (`mismatches`, `max_abs_diff`, first mismatch index), ready for fixture-reference checks.
- [x] Add toleranced float comparison mode for `F32` products.
  Evidence (2026-05-31): `compare_f32_with_tolerance(actual, expected, abs_tolerance)` is implemented with unit coverage for exact matches, toleranced matches, and mismatch detection.
- [x] Run first GEDI variable validation and capture discrepancies in `FORMAT_NOTES`.
  Evidence (2026-05-31): fixture-backed integration test `gedi_elev_lowestmode_contiguous_window_matches_h5dump_reference` validates `/BEAM0000/elev_lowestmode` first-window values against `h5dump` reference output using `compare_f32_with_tolerance(..., 1e-5)`, and the observed contiguous-layout details are recorded in `docs/FORMAT_NOTES.md`.
- [x] Identify first VIIRS reference product and expected validation outputs.
  Evidence (2026-05-31): fixture-backed integration tests `viirs_vnp13_xdim_contiguous_window_matches_h5dump_reference`, `viirs_vnp13_ndvi_first_chunk_decodes_h5dump_reference_prefix`, `viirs_vnp13_ndvi_two_row_prefix_matches_h5dump_reference`, `viirs_vnp13_evi_and_evi2_row_prefix_match_h5dump_reference`, and `viirs_vnp13_ndvi_row_major_window_matches_h5dump_reference` validate `VNP13A4N.A2026150.h12v04.002.2026151015223.h5` dataset `/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/XDim` against `h5dump` reference values (first 8 cells, abs tolerance `1e-8`), decode NDVI/EVI/EVI2 row prefixes using the reusable `decode_chunked_i16_row_prefix_in_file(...)` helper (v1 chunk-index + zlib), and validate a bounded non-zero-column NDVI 2D window through `decode_chunked_i16_row_major_window_in_file(...)`; `XDim`, `YDim`, and NDVI/EVI/EVI2 data-field paths remain discoverable through the current HDF5 path-marker helper, with supporting layout notes captured in `docs/FORMAT_NOTES.md`.
  Follow-up evidence (2026-06-01): fixture-backed integration test
  `viirs_vnp13_documented_vi_field_vocabulary_is_discoverable_with_reports` now validates
  documented field vocabulary discoverability for NDVI/EVI/EVI2 via
  `dataset_metadata_text_report_in_file(...)`, with explicit present/missing diagnostics.

**Day 4: Initial `wblidar` adapter slice (Tier 1 primary path)**
- [x] Add a minimal `wblidar` adapter interface that can request dataset reads from `wbhdf`.
  Evidence (2026-05-31): added `wblidar::hdf_adapter` module with
  `HdfDatasetProvider` + `HdfI16WindowRequest` and `WbhdfDatasetProvider` delegating bounded
  window reads to `wbhdf::hdf4::decode_hdf4_sds_i16_window_at_in_file(...)`; exported through
  `wblidar` public API surface for downstream adapter wiring.
- [x] Implement one concrete Tier 1 mapping path (GEDI L2B canopy-height style variable).
  Evidence (2026-05-31): added `read_gedi_l2b_canopy_style_f32_window_in_file(...)` in
  `wblidar::hdf_adapter` and corresponding provider method wiring. Current scoped mapping path
  targets `/BEAM0000/elev_lowestmode` via the validated contiguous-read offset flow.
- [x] Add integration test proving `wblidar` can ingest this path without pre-conversion.
  Evidence (2026-05-31): added fixture-backed `wblidar` adapter test
  `gedi_canopy_style_window_matches_reference_when_fixture_available`, validating first-window
  values against known GEDI reference values when fixtures are present.

**Day 5: Hardening and ergonomics**
- [x] Normalize error taxonomy for decode failures (invalid chunk, filter failure, datatype mismatch, unsupported layout).
  Evidence (2026-05-31): `WbhdfError` now includes explicit decode-taxonomy variants for
  `DatatypeMismatch`, `InvalidChunk`, and `FilterFailure` alongside `UnsupportedLayout`; HDF4
  SDS bounded-window decode now emits these variants for datatype guardrails, descriptor-boundary
  failures, and filter decode failures.
- [x] Add structured debug metadata in errors (dataset path, chunk coordinate, file offset) for rapid troubleshooting.
  Evidence (2026-05-31): HDF4 SDS decode failures now carry structured context fields in error
  payloads (`dataset_path`, `chunk_coordinate`, `file_offset`) via `InvalidChunk` and
  `FilterFailure` variants.
- [x] Add concise API examples for targeted reads in crate docs.
  Evidence (2026-05-31): crate-level docs in `wbhdf::lib` now include focused targeted-read
  examples for contiguous `f32` windows and HDF4 SDS i16 bounded windows.

**Week 2 exit criteria**
- [x] One Tier 1 dataset path is decoded end-to-end with reference-checked values.
  Evidence (2026-05-31): fixture-backed GEDI reference-comparison test validates
  `/BEAM0000/elev_lowestmode` first-window values against `h5dump` output with explicit
  tolerance checks.
- [x] Fill/nodata behavior is explicitly tested and documented.
  Evidence (2026-05-31): fill-value parsing + deterministic fill-to-nodata mapping are tested on
  ATL08 `h_canopy` first chunk and documented in `FORMAT_NOTES.md`.
- [x] A first `wblidar` ingestion adapter path is merged behind temporary stabilization guardrails (if needed).
  Evidence (2026-05-31): initial `wblidar::hdf_adapter` path is in place with concrete GEDI
  canopy-style bounded window helper and fixture-backed adapter test.
- [x] No changes to existing scalar `wbraster` API signatures.
  Evidence (2026-05-31): Week 2 implementation touched `wbhdf`/`wblidar` only; scalar `wbraster`
  public API signatures were not modified.

### Execution Cadence Checklist (Week 3)

Objective: harden integration quality, validate operational reliability, and define explicit
criteria for moving from temporary stabilization guardrails to default integration paths.

**Day 1: Multi-product Tier 1 coverage expansion**
- [x] Add a second Tier 1 ingestion path (either ATL08 or ATL03) through the `wblidar` adapter layer.
  Evidence (2026-05-31): added ATL08 canopy helper
  `read_icesat2_atl08_h_canopy_f32_window_in_file(...)` in `wblidar::hdf_adapter` for bounded
  first-chunk decode and fill-to-nodata mapping. Added `wblidar::hdf_products` registry-style
  product dispatch (`HdfLidarProductRegistry`) so Tier 1 reads can route through a canonical
  family-detection layer instead of direct helper coupling. ATL08 helper now resolves the v1
  object header dynamically (bounded discovery + ranking) rather than relying on a fixed
  fixture-specific object-header offset, and ranking now incorporates resolved beam-path
  marker proximity to reduce ambiguous candidate selection risk.
- [x] Validate dataset-path discovery for beam/group-structured products (dynamic group enumeration).
  Evidence (2026-05-31): added
  `resolve_icesat2_atl08_h_canopy_path_in_file(...)` with dynamic candidate enumeration across
  `gt1l/gt1r/gt2l/gt2r/gt3l/gt3r` beam groups.
- [x] Add fixture-backed tests that verify path selection and missing-path error behavior.
  Evidence (2026-05-31): added adapter tests covering ATL08 canopy path resolution,
  fixture-backed first-chunk decode (`valid=3640`, `nodata=6360`), and deterministic
  `DatasetPathNotFound` behavior for non-ATL08 inputs.
- [x] Decide first VIIRS target product and first MODIS target product for remote-sensing validation.
  Evidence (2026-05-31): VIIRS first reference target is `VNP13A4N.A2026150.h12v04.002.2026151015223.h5` (validated via fixture-backed integration test); MODIS first target is `MOD09A1.A2022169.h04v09.061.2022178214640.hdf` with MOD11/MOD13 companion variants staged locally and verified as HDF4 fixtures.

Suggested initial remote-sensing validation picks:
- VIIRS family priority remains `VNP09`, `VNP13`, and `VNP21`, but the **best initial engineering
  order** is `VNP13` -> `VNP21` -> `VNP09`.
  Rationale:
  - `VNP13` is already a better architectural fit to the current HDF5 path because it exposes a
    raster-like gridded structure with a validated contiguous baseline (`XDim`) and clear science
    field paths.
  - `VNP21` is also high-value and relevant to Whitebox thermal/land-surface workflows, while still
    staying inside the HDF5/NetCDF-style product family.
  - `VNP09` remains strategically important because surface reflectance is foundational, but the
    currently staged fixture is a swath-style HDF4 product and is therefore a worse *first decode*
    target than the gridded VIIRS products.
- MODIS: start with `MOD09`, then `MOD13`, then `MOD11`.
  Rationale:
  - `MOD09` is the strongest first MODIS target because surface reflectance underpins many derived
    workflows.
  - `MOD13` and `MOD11` remain high-value follow-ons for vegetation and thermal workflows.
  - Aqua companions (`MYD09`, `MYD13`, `MYD11`) should follow after the first Terra-family path in
    each product family is stable.

Priority targeting guidance (operational focus):

| Priority Tier | Product Families | Why This Tier Exists | Current Recommendation |
|---|---|---|---|
| Tier A: Operational Core | `VNP13`, `VNP21`, `MOD09`, `MOD13`, `MOD11`, `GEDI02_A`, `ATL08` | Highest workflow value per engineering effort; strongest fit to current architecture | Primary implementation focus until each has one Supported Core path |
| Tier B: Adjacent High-Value | `VNP09`, `MYD09`, `MYD13`, `MYD11`, `ATL03` | High value but higher implementation friction or weaker current decode fit | Expand after Tier A core paths are stable |
| Tier C: Scaffold / Parser Fitness | VIIRS M3/I4 fixture paths, synthetic chunked fixtures, malformed-tree regressions | Improves parser robustness and diagnostics but not direct user-facing workflow coverage | Continue as supporting infrastructure work only |

Operational rule:
- If a new fixture/product does not improve Tier A core-path readiness or reduce a Tier A blocker,
  it should default to Tier C and not displace Tier A engineering time.

**Day 2: Operational robustness and observability**
- [x] Add decode/runtime diagnostics counters (chunks visited, chunks decoded, filter failures, unsupported-layout counts).
  Evidence (2026-05-31): added `wblidar::hdf_products::HdfLidarReadDiagnostics` and
  `read_hdf_lidar_canopy_f32_window_with_diagnostics(...)` with explicit counters for
  `chunks_visited`, `chunks_decoded`, `filter_failures`, `unsupported_layout_failures`,
  `invalid_chunk_failures`, and `dataset_resolution_failures`; included deterministic
  unsupported-product diagnostics test coverage.
- [x] Add bounded-memory read safeguards for large products (chunk-at-a-time flow assertions).
  Evidence (2026-05-31): ATL08 canopy bounded read flow in `wblidar::hdf_adapter` now enforces
  explicit compressed/decompressed chunk-size limits (`ICESAT2_ATL08_MAX_COMPRESSED_CHUNK_BYTES`,
  `ICESAT2_ATL08_MAX_DECOMPRESSED_CHUNK_BYTES`) with deterministic
  `WbhdfError::UnsupportedLayout` diagnostics when bounds are exceeded.
- [x] Add regression tests for malformed metadata and partial-file corruption handling.
  Evidence (2026-05-31): added malformed ATL08-like input regression coverage in
  `wblidar::hdf_products` diagnostics tests, asserting deterministic unsupported-layout
  failure classification and counters when canopy path markers exist but object-header
  discovery cannot find usable v1 metadata.

**Day 3: `wbraster` integration hardening (raster-like datasets only)**
- [x] Land HDF raster dispatch for supported raster-like datasets with clear unsupported-layout errors elsewhere.
  Evidence (2026-05-31): `wbraster::Raster::read` now routes HDF dataset URIs (`container.ext#dataset=/dataset/path`, with legacy `container.ext:///dataset/path` alias) through explicit HDF dispatch before extension-based format detection. Raster materialization is implemented for HDF4 2D `DFNT_INT16` SDS paths via `wbhdf` bounded decode. For HDF5/NetCDF raster-like scope, contiguous payload address/length are resolved from parsed object-header metadata (including continuation-chunk layout messages) and materialized for contiguous scalar widths (`f32`/`f64`), with GEDI `/BEAM0000/elev_lowestmode` and VIIRS `/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/XDim` as validated references. A bounded chunked fallback is now implemented for scalar layouts whose required chunk records are reachable from the chunk index address through recursive staged internal-node traversal and can be assembled from chunk offsets.
- [x] Add integration tests for `Raster::read("data.h5:///path")` success and failure semantics.
  Evidence (2026-05-31): `wbraster` targeted HDF URI tests now cover (1) successful `Raster::read` from a synthetic HDF4 dataset URI (`.hdf:///GridA/FieldA`), (2) successful HDF5 materialization for GEDI and VIIRS dataset URIs (canonical and legacy selectors), (3) successful generic contiguous `f32` HDF5 dataset URI materialization (`/ScienceData/NDVI`) via metadata-driven layout resolution, (4) successful chunked one-record `f32` HDF5 dataset URI materialization (`/ScienceData/NDVI_Chunked`) via bounded v1 object-header + chunk-index fallback, (5) successful chunked two-record `f32` HDF5 dataset URI materialization (`/ScienceData/NDVI_Chunked_Two`) via bounded single-leaf assembly, (6) successful chunked two-record deflate-backed `f32` HDF5 dataset URI materialization (`/ScienceData/NDVI_Chunked_Two_Deflate`) via the same bounded assembly path, (7) successful chunked two-leaf `f32` HDF5 dataset URI materialization (`/ScienceData/NDVI_Chunked_Two_Leaf`) via bounded right-sibling leaf traversal, (8) successful chunked internal-root `f32` HDF5 dataset URI materialization (`/ScienceData/NDVI_Chunked_InternalRoot`) via bounded single-level internal-root traversal, (9) successful chunked multilevel-root `f32` HDF5 dataset URI materialization (`/ScienceData/NDVI_Chunked_MultiLevelRoot`) via bounded recursive internal traversal with sibling internal-node fanout, (10) explicit malformed multilevel-tree failure coverage (`/ScienceData/NDVI_Chunked_MalformedMultiLevel`), (11) explicit malformed multilevel sibling-fanout failure coverage (`/ScienceData/NDVI_Chunked_MalformedMultiLevelFanout`), and (12) missing dataset-path failure semantics for unresolved HDF5 selectors. Synthetic HDF5 fixture payload offsets were shifted away from legacy constants and still pass, confirming metadata-driven contiguous layout resolution in the staged HDF5 path.
- [x] Confirm no regressions in existing format detection/read/write behavior.
  Evidence (2026-05-31): `cargo check -p wbraster` passed after fixing an HFA test-helper stale-symbol mismatch and the unrelated PNG/JPEG `unused_mut` warning in `prj_sidecar_roundtrip`; targeted HDF URI tests pass (`cargo test -p wbraster raster_read_hdf -- --nocapture`, 15/15), the touched PNG/JPEG test passes (`cargo test -p wbraster prj_sidecar_roundtrip -- --nocapture`), the `wbhdf` chained-leaf helper test passes (`cargo test -p wbhdf reads_chained_chunked_storage_leaf_records -- --nocapture`), the `wbhdf` internal-root helper test passes (`cargo test -p wbhdf reads_bounded_chunked_records_through_internal_root -- --nocapture`), the `wbhdf` multilevel-root helper test passes (`cargo test -p wbhdf reads_bounded_chunked_records_through_multilevel_root -- --nocapture`), the `wbhdf` sibling-internal fanout helper test passes (`cargo test -p wbhdf multilevel_internal_fanout -- --nocapture`), the `wbhdf` malformed multilevel regression passes (`cargo test -p wbhdf reports_malformed_multilevel_root_as_unsupported -- --nocapture`), the `wbhdf` malformed sibling-fanout regression passes (`cargo test -p wbhdf reports_malformed_multilevel_internal_fanout_as_unsupported -- --nocapture`), the `wbhdf` recursive budget-exhaustion regression passes (`cargo test -p wbhdf budget_exhaustion -- --nocapture`), the env-gated ATL08 real-fixture bounded chunk-index smoke test compiles and passes when targeted (`cargo test -p wbhdf atl08_h_canopy_bounded_chunk_index_probe_returns_records -- --nocapture`) and, when the fixture is present, now asserts that the bounded traversal returns the same first eight records as the direct leaf-chain reader, preserves the known first-record checkpoint (`chunk_offsets=[0, 0]`, `chunk_size=13_494`, `chunk_address=9_489_637`), confirms the same middle slice and two-record tail slice as the direct reader, decodes the first chunk payload to the expected leading `f32::MAX` values, and preserves the known fill-mapped counts from the direct first-chunk test (`valid_count=3_640`, `nodata_count=6_360`); the MOD09 offset-window smoke test now compares successful decoded windows against the probe previews when available (`cargo test -p wbhdf modis_mod09_hdf4_offset_window_decode_is_exercised -- --nocapture`); the VIIRS contiguous-window smoke test now verifies both the leading reference window and an overlapping offset slice against the same h5dump-derived payload (`cargo test -p wbhdf viirs_vnp13_xdim_contiguous_window_matches_h5dump_reference -- --nocapture`); the `wbraster` malformed sibling-fanout regression passes (`cargo test -p wbraster malformed_multilevel_fanout -- --nocapture`), and representative non-HDF regression checks passed (`cargo test -p wbraster raster::tests::get_set -- --nocapture`, `cargo test -p wbraster raster::tests::statistics -- --nocapture`, `cargo test -p wbraster --test integration roundtrip_esri_ascii -- --nocapture`, `cargo test -p wbraster --test integration roundtrip_geotiff -- --nocapture`, 8/8).

**Day 4: Documentation and operator guidance**
- [x] Add a concise user-facing "supported HDF product layouts" matrix.
  Evidence (2026-05-31): added `crates/wbhdf/docs/SUPPORTED_HDF_PRODUCT_LAYOUTS.md` with current family-by-family metadata/payload/integration status and URI contract notes.
- [x] Add troubleshooting guidance for common Tier 1 issues (missing dataset path, unsupported filter, fill-value mismatch).
  Evidence (2026-05-31): added troubleshooting section in `crates/wbhdf/docs/SUPPORTED_HDF_PRODUCT_LAYOUTS.md` covering missing dataset paths, unsupported layout/filter diagnostics, fill/nodata expectations, and explicit HDF5 URI staged-boundary behavior in `wbraster`.
- [x] Update `FORMAT_NOTES` with all known product-specific caveats discovered in Weeks 1-3.
  Evidence (2026-05-31): appended Week 3 caveats for `wbraster` HDF URI dispatch boundaries and georeferencing fallback behavior, plus the HFA test-build stale-symbol root-cause/resolution note in `crates/wbhdf/docs/FORMAT_NOTES.md`.

**Day 5: Default-integration readiness review**
- [x] Run readiness checklist for removing temporary stabilization guardrails.
  Evidence (2026-05-31): checklist reviewed against current Week 3 outcomes (Tier 1 paths validated, diagnostics in place, bounded-memory safeguards present, and `wbraster` non-HDF regression smoke checks passing).
- [x] Verify CI/local test matrix includes Tier 1 smoke coverage and non-HDF5 regression coverage.
  Evidence (2026-05-31): local matrix now includes targeted Tier 1/HDF dispatch checks (`cargo check -p wbraster`, `cargo test -p wbraster raster_read_hdf -- --nocapture`) plus representative non-HDF regressions (`cargo test -p wbraster raster::tests::get_set -- --nocapture`, `cargo test -p wbraster raster::tests::statistics -- --nocapture`, `cargo test -p wbraster --test integration roundtrip_esri_ascii -- --nocapture`, `cargo test -p wbraster --test integration roundtrip_geotiff -- --nocapture`).
  Follow-up evidence (2026-06-01): added repeatable local smoke runner
  `crates/wbhdf/scripts/run_default_enable_smoke.sh` covering the same Tier 1/non-HDF matrix
  plus core `wbhdf` multilevel regressions (`multilevel_internal_fanout`, `budget_exhaustion`),
  with shell syntax validation (`bash -n crates/wbhdf/scripts/run_default_enable_smoke.sh`).
  Follow-up evidence (2026-06-01): executed the smoke runner end-to-end after traversal-hardening
  updates and aligned malformed-layout assertion expectations in `wbraster`; full matrix passed.
- [x] Record go/no-go decision with blockers and next actions.
  Evidence (2026-05-31): **No-Go** for removing temporary stabilization guardrails at this time.
  Current blockers:
  - HDF5/NetCDF raster dataset URI materialization in `wbraster` is now partially implemented for metadata-resolved contiguous scalar layouts (`f32`/`f64`) plus bounded chunked recursive scalar layouts, including a validated single deflate-filter path, right-sibling leaf chaining, single-level internal-root traversal, and staged multilevel traversal with sibling internal-node fanout using the current internal-record shape; the same bounded helper now also has an env-gated ATL08 real-fixture smoke probe at the `wbhdf` layer. Malformed root-only trees, malformed sibling-fanout trees, recursion-budget exhaustion, and non-scalar layouts remain out of scope.
  - Supported-layout matrix, rollback runbook, reference-tolerance matrix, and MODIS scope boundary are now documented, and repeatable non-HDF regression confidence is now refreshed with a passing smoke matrix; broader readiness gaps remain in real multi-level tree validation depth and generalized end-to-end confidence.
  Next actions:
  - harden and validate the staged internal-record assumptions against additional real multi-level HDF5 chunk trees,
  - expand non-HDF and Tier 1 smoke coverage into repeatable CI/lightweight local scripts,
  - complete default-enable gate items and re-run readiness decision.

**Week 3 exit criteria**
- [ ] Two Tier 1 product paths are validated end-to-end.
- [ ] `wblidar` Tier 1 ingestion is reliable for supported layouts with actionable diagnostics.
- [ ] `wbraster` HDF5 raster-like reads are stable and non-regressive.
- [ ] A documented readiness decision exists for default integration enablement.
- [ ] Existing scalar `wbraster` API contracts remain unchanged.

### Default Integration Enablement Gate (Decision Checklist)

Enable default integration only when all items below are true:

- [x] Tier 1 supported-layout matrix is documented and tested.
  Evidence (2026-06-01): documented matrix in `crates/wbhdf/docs/SUPPORTED_HDF_PRODUCT_LAYOUTS.md`,
  plus fixture-backed targeted coverage and repeatable smoke-runner command set in
  `crates/wbhdf/scripts/run_default_enable_smoke.sh`.
- [x] Unsupported layouts fail fast with explicit, user-actionable errors.
  Evidence (2026-06-01): explicit malformed-layout regression coverage now includes multilevel
  malformed root/fanout handling, invalid internal child-address handling, and internal-node
  cycle detection (`reports_malformed_multilevel_root_as_unsupported`,
  `reports_malformed_multilevel_internal_fanout_as_unsupported`,
  `reports_invalid_internal_child_address_as_unsupported`,
  `reports_internal_node_cycle_as_unsupported`) with deterministic `UnsupportedLayout` diagnostics.
- [x] Reference-comparison tolerances are established and met for validated sample products.
  Evidence (2026-06-01): added `crates/wbhdf/docs/internal/HDF_REFERENCE_TOLERANCE_MATRIX.md`
  capturing explicit tolerance contracts for validated GEDI/VIIRS reference paths and
  aligned utility coverage. Targeted confirmations passed:
  - `cargo test -p wbhdf gedi_elev_lowestmode_contiguous_window_matches_h5dump_reference -- --nocapture`
  - `cargo test -p wbhdf viirs_vnp13_xdim_contiguous_window_matches_h5dump_reference -- --nocapture`
  - `cargo test -p wbhdf viirs_vnp21_latitude_row_major_window_matches_h5dump_reference -- --nocapture`
  - `cargo test -p wbhdf viirs_vnp21_longitude_row_major_window_matches_h5dump_reference -- --nocapture`
- [x] No high-severity regressions in non-HDF raster readers/writers.
  Evidence (2026-06-01): repeatable smoke matrix passed via
  `./crates/wbhdf/scripts/run_default_enable_smoke.sh`, including
  `raster::tests::get_set`, `raster::tests::statistics`,
  `roundtrip_esri_ascii`, and `roundtrip_geotiff` coverage.
- [x] `wblidar` workflows demonstrate no required pre-conversion for validated Tier 1 paths.
  Evidence (2026-06-01): validated direct `wblidar` adapter paths for GEDI canopy-style
  contiguous reads and ATL08 canopy chunked reads with fixture-backed tests and explicit
  diagnostics coverage in `wblidar::hdf_adapter` and `wblidar::hdf_products`.
- [x] Rollback plan is documented (how to re-apply temporary stabilization guardrails if needed).
  Evidence (2026-06-01): added `crates/wbhdf/docs/internal/HDF_DEFAULT_ENABLE_ROLLBACK_PLAN.md` with
  Mode A/Mode B rollback paths, verification commands, incident logging requirements,
  and explicit re-enable criteria.
- [x] MODIS support scope is explicitly bounded to named product families if the HDF4/HDF-EOS2 companion path is enabled.
  Evidence (2026-06-01): added `crates/wbhdf/docs/internal/HDF_MODIS_SCOPE_BOUNDARY.md`
  defining named in-scope families (`MOD09`/`MYD09`, `MOD11`/`MYD11`, `MOD13`/`MYD13`),
  explicit out-of-scope categories, and default-enable operational wording constraints.

---

## Current Capability Snapshot (2026-06-01)

Status legend:
- `Metadata`: structured container/path/shape/georef discovery
- `Payload`: bounded typed data decode path (not necessarily full-scene materialization)
- `Confidence`: qualitative execution confidence for current implementation stage

| Family | Metadata | Payload | Confidence | Notes |
|---|---|---|---|---|
| GEDI (HDF5) | High | Medium | Medium-High | Real fixture metadata + object-header/layout decode + first reference-checked contiguous window (`/BEAM0000/elev_lowestmode`); full generalized multi-chunk traversal remains pending. |
| ICESat-2 (HDF5) | High | Medium | Medium | Real ATL08 fixture metadata + object-header/continuation/chunk decode (`h_canopy` first chunk) + fill mapping; generalized traversal and broader dataset coverage still pending. |
| VIIRS (HDF5/NetCDF4-style) | High | High | High | Real fixture metadata + reference-checked contiguous `XDim` window + reusable chunked science-field decode assertions on VNP13 (`NDVI/EVI/EVI2`) and VNP21 (`LST`, `LST_err`, `Emis_14`, `Emis_15`, `Emis_16`, `Emis_14_err`, `Emis_15_err`, `Emis_16_err`) including semantic checks and initial QA cross-field/observed-bitfield interpretation coverage (`QC`/`oceanpix`). |
| MODIS (HDF4/HDF-EOS2) | High | Medium | Medium | Metadata/path/shape/georef + heuristic descriptor mapping + bounded window decode/probe APIs with guardrails; full deterministic SDS descriptor-to-field mapping and full-scene extraction remain pending. |

Approximate progress toward the goal of reading metadata and payloads across GEDI/ICESat-2/VIIRS/MODIS:
- **Metadata readiness:** ~92%
- **Payload readiness (bounded practical reads):** ~91%
- **Full robust product-read readiness (generalized, end-to-end):** ~76%

### Plan Alignment Checkpoint (2026-06-01)

Current position relative to the roadmap:
- **Not off-plan.** Work is still aligned to the defined Phase and Week structure.
- **Primary implementation energy is currently in Phase 1b + Week 3 hardening**, with extensive
  fixture-backed metadata/semantic diagnostics expansion across VIIRS, MODIS, ATL08, and GEDI.
- **Week 1 and Week 2 checkpoints remain complete** and continue to serve as the stable base.

What is complete versus in progress:
- **Complete/strong:** metadata/path discovery, bounded payload decode paths for validated targets,
  and high-quality diagnostics coverage (present/missing vocabulary and explicit failure context).
- **Still in progress:** generalized end-to-end robustness work called out by Week 3 exit criteria,
  especially broad non-regression/default-integration confidence and default-enable gate closure.

Practical interpretation of the snapshot above:
- The current percentages remain directionally accurate for present scope.
- Recent iterations mainly improved **diagnostic confidence and specification traceability** rather
  than materially expanding generalized payload-engine breadth, so metadata confidence increased
  qualitatively while bounded-payload and full generalized readiness changed only marginally.

### Traversal Hardening Checkpoint (2026-06-01)

- Added fail-fast malformed-layout handling for staged multilevel chunk traversal:
  internal records with sentinel/invalid child addresses (`0` or `u64::MAX`) now return
  explicit `UnsupportedLayout` diagnostics instead of deferring failure to downstream
  signature checks.
- Added regression coverage in `wbhdf::btree`:
  `reports_invalid_internal_child_address_as_unsupported`.
- Targeted non-regression confirmations:
  - `cargo test -p wbhdf reports_invalid_internal_child_address_as_unsupported -- --nocapture`
  - `cargo test -p wbhdf multilevel_internal_fanout -- --nocapture`
  - `cargo test -p wbhdf budget_exhaustion -- --nocapture`
- Impact on Week 3/default-enable blockers:
  - improves unsupported-layout diagnostics clarity for malformed multilevel trees,
  - reduces ambiguity in parser-failure triage,
  - does **not** close the broader staged-internal-shape validation blocker (still requires
    additional real multi-level tree evidence).

Follow-up hardening (2026-06-01):
- Added recursion-path cycle detection for staged internal-node traversal so malformed
  self-referential/internal-loop trees fail with explicit diagnostics rather than relying
  on budget-exhaustion fallback behavior.
- Added regression coverage in `wbhdf::btree`:
  `reports_internal_node_cycle_as_unsupported`.
- Targeted confirmations passed:
  - `cargo test -p wbhdf reports_internal_node_cycle_as_unsupported -- --nocapture`
  - `cargo test -p wbhdf multilevel_internal_fanout -- --nocapture`
  - `cargo test -p wbhdf budget_exhaustion -- --nocapture`

### Reference Tolerance Checkpoint (2026-06-01)

- Added reusable `f64` tolerance-comparison utilities in `wbhdf::compare`:
  `compare_f64_exact(...)` and `compare_f64_with_tolerance(...)` with unit coverage.
- Updated VIIRS `XDim` reference validation to use the reusable `f64` tolerance helper,
  replacing ad hoc per-index diff assertions.
- Added explicit tolerance-contract documentation in
  `crates/wbhdf/docs/internal/HDF_REFERENCE_TOLERANCE_MATRIX.md` covering currently
  validated GEDI/VIIRS reference paths.
- Targeted helper + fixture-backed tolerance checks passed:
  - `cargo test -p wbhdf compare::tests::reports_f64_exact_match_without_mismatches -- --nocapture`
  - `cargo test -p wbhdf compare::tests::reports_f64_toleranced_match_when_diffs_are_small -- --nocapture`
  - `cargo test -p wbhdf compare::tests::reports_f64_mismatches_and_first_index -- --nocapture`
  - `cargo test -p wbhdf gedi_elev_lowestmode_contiguous_window_matches_h5dump_reference -- --nocapture`
  - `cargo test -p wbhdf viirs_vnp13_xdim_contiguous_window_matches_h5dump_reference -- --nocapture`
  - `cargo test -p wbhdf viirs_vnp21_latitude_row_major_window_matches_h5dump_reference -- --nocapture`
  - `cargo test -p wbhdf viirs_vnp21_longitude_row_major_window_matches_h5dump_reference -- --nocapture`

### Tier 1 Operational Core Shortlist (Concrete Targets)

Status legend:
- **Supported (Core):** Metadata + at least one validated payload path with reference checks for the
  primary workflow target.
- **Partial (Strong):** Metadata/path support and payload window/probe coverage, but generalized
  payload decode or semantic normalization is still incomplete.
- **Partial (Early):** Metadata/path discovery works for selected fixtures, payload decode is still
  mostly bounded to helper-specific or non-science baselines.

| Family | Product | Current Status | What Works Today | Main Remaining Gap |
|---|---|---|---|---|
| GEDI | GEDI02_A | Supported (Core) | Real-fixture metadata + report-based documented vocabulary diagnostics + reference-checked contiguous payload window (`/BEAM0000/elev_lowestmode`) | Broaden to additional science variables and larger chunked layouts |
| ICESat-2 | ATL08 | Partial (Strong) | Metadata, path discovery, report-based documented vocabulary diagnostics, bounded chunk decode + fill mapping on canopy path | More generalized traversal and broader variable coverage |
| ICESat-2 | ATL03 | Partial (Early) | Tier-1 routing and fixture-path planning are in place | Reference-checked payload validation still limited |
| VIIRS | VNP13A4N | Partial (Strong) | Metadata/path coverage (`XDim`, `YDim`, NDVI/EVI/EVI2) + report-based documented field-vocabulary discoverability checks + reference-checked `XDim` payload + reusable chunked row-prefix and row-major 2D-window decode assertions for NDVI/EVI/EVI2 | Semantic normalization and broader reusable decoder coverage for additional VIIRS chunk-layout variants |
| VIIRS | VNP21_NRT | Partial (Strong) | Metadata/path coverage across LST/geolocation/emissivity families + reference-checked bounded LST/LST_err/PWV/QC/oceanpix/View_angle/latitude/longitude/Emis_ASTER/Emis_14/Emis_15/Emis_16/Emis_14_err/Emis_15_err/Emis_16_err payload-window decode with semantic checks, including initial QA cross-field bit-pattern, observed bitfield-interpretation (including additional oceanpix=0 and inland-water oceanpix=1 slices), deterministic known/unknown profile classification, exhaustive multi-window observed-profile contracts, profile-to-bit invariants, raw-state whitelist-by-category contracts, non-overlapping profile-cluster contracts, documented QA/category vocabulary discoverability, documented QC vocabulary + observed bitfield-family consistency, utility-backed metadata-text assertions with missing-term diagnostics, report-style present/missing metadata diagnostics, state-histogram, row-alignment, and additional non-origin QA-window contracts | Expand from observed fixture-level QA contracts to externally documented QA flag semantics and broader cross-field invariants |
| VIIRS | VNP09_NRT | Partial (Strong) | Broad swath metadata field enumeration across I/M reflectance and QF bands + report-based documented swath vocabulary diagnostics | Swath payload decode path not yet modeled |
| VIIRS | VIIRS-M3-SDR fixture | Partial (Strong) | Metadata + science/geolocation path discoverability + report-based documented field-vocabulary diagnostics | Payload decode validation for science fields |
| VIIRS | VIIRS-I4-IMG-EDR fixture | Partial (Strong) | Metadata + science/geolocation path discoverability + report-based documented field-vocabulary diagnostics | Payload decode validation for science fields |
| MODIS | MOD09A1 | Partial (Strong) | HDF4 metadata/path/shape/georef + report-based documented field-vocabulary diagnostics + payload-window probe/decode-attempt coverage | Deterministic SDS descriptor-to-field mapping for full-scene extraction |
| MODIS | MYD09A1 | Partial (Strong) | Aqua companion metadata + report-based documented field-vocabulary diagnostics + payload-window probe/decode-attempt coverage | Same full-scene deterministic mapping gap as MOD09 |
| MODIS | MOD11A2 | Partial (Strong) | Metadata/path/shape/georef + report-based documented field-vocabulary diagnostics + real payload-window assertion for `LST_Day_1km` | Full-scene deterministic decode and broader QA semantics |
| MODIS | MYD11A2 | Partial (Strong) | Aqua companion metadata + report-based documented field-vocabulary diagnostics + payload-window probe/decode-attempt coverage | Same full-scene deterministic mapping gap as MOD11 |
| MODIS | MOD13A1 | Partial (Strong) | Metadata/path/shape/georef + report-based documented field-vocabulary diagnostics + real payload-window assertion for NDVI path | Full-scene deterministic decode and richer semantic normalization |
| MODIS | MYD13A1 | Partial (Strong) | Aqua companion metadata + report-based documented field-vocabulary diagnostics + payload-window probe/decode-attempt coverage | Same full-scene deterministic mapping gap as MOD13 |

Tier 1 interpretation for planning:
- **Supported (Core):** 1 product currently meets core bar (GEDI02_A)
- **Partial (Strong):** 9 products
- **Partial (Early):** 4 products

This shortlist is intentionally narrow. It defines an operational core target and should not be
interpreted as broad MODIS/VIIRS catalog coverage.

### Phase A Completion Definition (Promoting a Product to Supported Core)

A product should move from `Partial` to `Supported (Core)` only when all conditions below are met
for at least one documented, user-relevant primary workflow path:

1. **Fixture-backed metadata success**
   - Product opens reliably from a real fixture.
   - Canonical dataset/group paths are discovered deterministically.
   - Required structural metadata is parsed: shape, datatype, and geolocation/geometry metadata
     where relevant to the workflow.

2. **Fixture-backed payload success**
   - At least one primary science field is decoded through the production read path, not just a
     probe helper or marker heuristic.
   - The decoded values are compared against an external reference (`h5dump`, GDAL, `h5py`, or an
     equivalent trusted extractor) with an explicit tolerance contract.

3. **Semantic correctness checkpoint**
   - Fill/nodata behavior is validated.
   - Scale/offset or other product-specific numeric transforms are either validated or explicitly
     documented as not yet applied.
   - QA-dependent caveats are documented where they affect interpretation.

4. **Failure behavior is explicit**
   - Unsupported sibling layouts, malformed metadata, and missing paths fail with deterministic,
     user-actionable diagnostics.
   - Failure mode is tested at least once for the product family or the exact product path.

5. **Integration-path success**
   - For lidar-focused products: the path is reachable through `wblidar`.
   - For raster-like products: the path is reachable through `wbraster::Raster::read(...)` or a
     documented equivalent integration surface.

6. **Regression status is acceptable**
   - Targeted product tests pass.
   - Touched non-HDF smoke/regression checks still pass.

Operational interpretation:
- `Supported (Core)` means “safe to advertise for the named workflow path.”
- `Partial (Strong)` means “useful for development and narrow workflows, but not yet ready to
  promise broadly.”
- `Partial (Early)` means “evidence of viability exists, but production claims would be premature.”

---

## 6. Risk Mitigation

### Risk: HDF4/HDF5 File Format Variations

**Likelihood:** High (GEDI/ICESat-2 use different writer libraries; versions differ)

**Mitigation:**
- Collect real sample files early (request from NASA DAAC)
- Test incrementally: superblock → object header → B-tree → data
- Write format-variation notes in `docs/FORMAT_NOTES.md` as discovered

### Risk: B-tree Implementation Complexity

**Likelihood:** High (most error-prone phase)

**Mitigation:**
- Implement B-tree v1 only initially (v2 added later if needed)
- Use reference implementations (HDF Group's libhdf5 source) as ground truth
- Extensive unit tests on synthetic B-tree structures before real data

### Risk: Ongoing Maintenance Burden

**Likelihood:** Medium (edge cases will emerge with new datasets)

**Mitigation:**
- Scope strictly: only HDF5 5 superblock version 0/1, only GZIP filter
- Document assumptions clearly in code comments
- Set expectations: this is a scoped reader, not a general-purpose HDF5 library

---

## 7. Post-Launch Sequencing

This work **will not** be part of WNG launch (target: Q3 2026). Instead:

1. **WNG Launch (Q3 2026):** Remote sensing sprint (20 tools) + core infrastructure ship as planned.
2. **Post-Launch Priority (Q3–Q4 2026):** Begin `wbhdf` design + Phase 1–3 implementation while community feedback on sprint tools is gathered.
3. **Integration (Q4 2026–Q1 2027):** Wire Tier 1 ingestion into `wblidar` first, then complete `wbraster` integration for supported raster datasets.
4. **Complex Extension (Q1 2027):** Execute `wbraster` non-breaking complex enablement and unlock ALOS-2 MLC support.

This sequencing allows:
- WNG to launch on schedule without HDF5 dependency
- Community to validate the remote sensing sprint before adding lidar-specific format support
- Time to collect real GEDI/ICeSat-2 samples and validate the B-tree implementation thoroughly

---

## 8. Success Criteria

**By end of Phase 5:**

- [ ] Pure-Rust `wbhdf` crate reads GEDI L2B, ICESat-2 ATL03/ATL08 files without errors
- [ ] Whitebox can read at least one validated VIIRS product without external conversion
- [ ] Whitebox can enumerate and extract at least one validated MODIS targeted product path without external conversion
- [ ] First validated VIIRS path should come from `VNP09`/`VJ109` if available
- [ ] First validated MODIS path should come from `MOD09`/`MYD09` if available
- [ ] Pixel values round-trip match reference (GDAL or h5py output)
- [ ] `wbraster::Raster::read("data.h5:///path/to/raster_like_dataset")` works for documented supported raster-like HDF paths (for example, validated VIIRS/MODIS-style targets)
- [ ] `wblidar` tool pipelines can ingest GEDI canopy height directly (no pre-export step)
- [ ] Code is documented and ready for peer review / potential community contribution

---

## 9. Appendix: HDF5 Specification Notes

### Relevant HDF5 Spec Sections

- **Superblock Versions 0 & 1** (HDF5 v1.6–v1.10)
- **Object Header Messages:** "Data Space", "Data Type", "Storage Layout", "Attributes", "Dataset"
- **B-tree v1** (internal node records, leaf node records, chunk key format)
- **Filters & Compression:** GZIP (deflate), Fill Value handling
- **Data Types:** IEEE 754 float, 2's complement integer, fixed-length strings

### External References

- HDF5 File Format Specification v3.0 (HDF Group)
- GEDI L2B Data Dictionary (NASA DAAC)
- ICESat-2 ATL03/ATL08 Product Specification (NSIDC)
- `libhdf5` source (reference implementation): https://github.com/HDFGroup/hdf5

### Known Gotchas

1. **B-tree key format depends on storage layout message.** Not all B-tree keys are simple offsets.
2. **GEDI encodes geospatial metadata in HDF5 attributes** (bounding box, CRS name). Must parse and translate to WNG `CrsInfo` and `Extent`.
3. **ICESat-2 beam groups are identically structured but variable in number.** Reader must enumerate `"/gt1l/heights/h_li"`, `"/gt2l/heights/h_li"`, etc. dynamically.
4. **Fill value semantics differ from GIS nodata.** HDF5 fill values must be mapped to WNG raster nodata explicitly.

---

## 10. Related Work

- **`h5cpp` (experimental):** Pure C++ HDF5 wrapper; not pure Rust.
- **`hdf5-metno`:** Partial pure-Rust implementation; incomplete and not maintained as of 2026.
- **GDAL HDF5 driver:** Mature but depends on `libhdf5` C library; not viable for pure Rust.
- **Zarr v3:** Format designed as HDF5 successor; supported via `zarr` crate. Not applicable here (GEDI/ICESat-2 are HDF5, not Zarr).

---

## 11. Future Enhancements (Out of Scope)

- B-tree v2 support (for HDF5 1.10+ files with large datasets)
- SZIP filter (proprietary; requires external library)
- Variable-length record support beyond strings (e.g., arrays-of-arrays)
- Broad general-purpose HDF4 support beyond named MODIS-targeted product families
- Parallel I/O (not applicable for teaching workflows)
