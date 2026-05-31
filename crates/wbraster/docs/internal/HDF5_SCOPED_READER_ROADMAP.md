# HDF Scoped Reader Roadmap

Date: 2026-05-12 (updated 2026-05-31; MODIS/VIIRS scope update)  
Status: Active planning (post-launch execution)  
Owner: `wbraster` core  
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
- [ ] Confirm initial MODIS targets: `MOD09`/`MYD09`, `MOD13`/`MYD13`, and `MOD11`/`MYD11` family layouts.
- [ ] Prototype minimal HDF4 container parsing and SDS/grid enumeration on one representative MODIS product.
- [ ] Document internal module boundaries for HDF5 and HDF4/HDF-EOS2 decoding within `wbhdf`.

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
- [ ] Integration tests: read GEDI/ICESat-2 tiles → export as GeoTIFF → validate pixel values
- [ ] Integration tests: read at least one VIIRS gridded product and one MODIS targeted product path
- [ ] Design documentation (B-tree algorithm, chunk layout assumptions, format notes)
- [ ] User guide: how to read GEDI/ICESat-2 via wbraster

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

### Implementation Start Checklist (Week 1)

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
- [ ] `wbhdf` crate builds and tests pass (including synthetic B-tree tests).
- [ ] At least one GEDI/ICESat-2 sample can be opened and traversed through metadata + chunk address lookup path.
- [ ] A written non-breaking complex API plan is checked in and approved before touching `wbraster` public method signatures.

### Implementation Start Checklist (Week 2)

Objective: complete the first end-to-end decode path (chunk -> decompress -> typed values)
and land initial `wblidar` Tier 1 ingestion plumbing.

**Day 1: Phase 3 decode pipeline wiring**
- [ ] Implement chunk read pipeline (`offset -> compressed bytes -> decompressed bytes -> typed decode`).
- [ ] Add explicit endianness utilities for `F32`/`F64`/`I16` decoding from HDF5 payloads.
- [ ] Add unit tests for decode correctness from synthetic little/big-endian byte fixtures.

**Day 2: Fill value and nodata mapping**
- [ ] Parse and apply HDF5 fill-value metadata during dataset materialization.
- [ ] Define deterministic mapping from HDF5 fill values to consumer nodata semantics.
- [ ] Add tests that verify fill handling and valid-cell counts against reference expectations.

**Day 3: First reference-comparison harness**
- [ ] Add comparison utility to validate decoded arrays against reference outputs (h5py/GDAL exports).
- [ ] Add toleranced float comparison mode for `F32` products.
- [ ] Run first GEDI variable validation and capture discrepancies in `FORMAT_NOTES`.
- [ ] Identify first VIIRS reference product and expected validation outputs.

**Day 4: Initial `wblidar` adapter slice (Tier 1 primary path)**
- [ ] Add a minimal `wblidar` adapter interface that can request dataset reads from `wbhdf`.
- [ ] Implement one concrete Tier 1 mapping path (GEDI L2B canopy-height style variable).
- [ ] Add integration test proving `wblidar` can ingest this path without pre-conversion.

**Day 5: Hardening and ergonomics**
- [ ] Normalize error taxonomy for decode failures (invalid chunk, filter failure, datatype mismatch, unsupported layout).
- [ ] Add structured debug metadata in errors (dataset path, chunk coordinate, file offset) for rapid troubleshooting.
- [ ] Add concise API examples for targeted reads in crate docs.

**Week 2 exit criteria**
- [ ] One Tier 1 dataset path is decoded end-to-end with reference-checked values.
- [ ] Fill/nodata behavior is explicitly tested and documented.
- [ ] A first `wblidar` ingestion adapter path is merged behind temporary stabilization guardrails (if needed).
- [ ] No changes to existing scalar `wbraster` API signatures.

### Implementation Start Checklist (Week 3)

Objective: harden integration quality, validate operational reliability, and define explicit
criteria for moving from temporary stabilization guardrails to default integration paths.

**Day 1: Multi-product Tier 1 coverage expansion**
- [ ] Add a second Tier 1 ingestion path (either ATL08 or ATL03) through the `wblidar` adapter layer.
- [ ] Validate dataset-path discovery for beam/group-structured products (dynamic group enumeration).
- [ ] Add fixture-backed tests that verify path selection and missing-path error behavior.
- [ ] Decide first VIIRS target product and first MODIS target product for remote-sensing validation.

Suggested initial remote-sensing validation picks:
- VIIRS: start with `VNP09`, then `VNP13`, then `VNP21`.
- MODIS: start with `MOD09`, then `MOD13`, then `MOD11`.

**Day 2: Operational robustness and observability**
- [ ] Add decode/runtime diagnostics counters (chunks visited, chunks decoded, filter failures, unsupported-layout counts).
- [ ] Add bounded-memory read safeguards for large products (chunk-at-a-time flow assertions).
- [ ] Add regression tests for malformed metadata and partial-file corruption handling.

**Day 3: `wbraster` integration hardening (raster-like datasets only)**
- [ ] Land HDF raster dispatch for supported raster-like datasets with clear unsupported-layout errors elsewhere.
- [ ] Add integration tests for `Raster::read("data.h5:///path")` success and failure semantics.
- [ ] Confirm no regressions in existing format detection/read/write behavior.

**Day 4: Documentation and operator guidance**
- [ ] Add a concise user-facing "supported HDF product layouts" matrix.
- [ ] Add troubleshooting guidance for common Tier 1 issues (missing dataset path, unsupported filter, fill-value mismatch).
- [ ] Update `FORMAT_NOTES` with all known product-specific caveats discovered in Weeks 1-3.

**Day 5: Default-integration readiness review**
- [ ] Run readiness checklist for removing temporary stabilization guardrails.
- [ ] Verify CI/local test matrix includes Tier 1 smoke coverage and non-HDF5 regression coverage.
- [ ] Record go/no-go decision with blockers and next actions.

**Week 3 exit criteria**
- [ ] Two Tier 1 product paths are validated end-to-end.
- [ ] `wblidar` Tier 1 ingestion is reliable for supported layouts with actionable diagnostics.
- [ ] `wbraster` HDF5 raster-like reads are stable and non-regressive.
- [ ] A documented readiness decision exists for default integration enablement.
- [ ] Existing scalar `wbraster` API contracts remain unchanged.

### Default Integration Enablement Gate (Decision Checklist)

Enable default integration only when all items below are true:

- [ ] Tier 1 supported-layout matrix is documented and tested.
- [ ] Unsupported layouts fail fast with explicit, user-actionable errors.
- [ ] Reference-comparison tolerances are established and met for validated sample products.
- [ ] No high-severity regressions in non-HDF raster readers/writers.
- [ ] `wblidar` workflows demonstrate no required pre-conversion for validated Tier 1 paths.
- [ ] Rollback plan is documented (how to re-apply temporary stabilization guardrails if needed).
- [ ] MODIS support scope is explicitly bounded to named product families if the HDF4/HDF-EOS2 companion path is enabled.

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
- [ ] `wbraster::Raster::read("gedi_tile.h5:///GEDI04_B_2020011143935_O08005_02_T14003_02_002_02_CH_VEG_QUAL_02.h5")` works
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
