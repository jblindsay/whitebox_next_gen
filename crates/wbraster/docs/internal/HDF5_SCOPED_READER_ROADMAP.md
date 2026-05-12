# HDF5 Scoped Reader Roadmap

Date: 2026-05-12  
Status: Planning (post-WNG launch priority)  
Owner: `wbraster` core  
Scope: Targeted HDF5 ingestion for scientific satellite datasets (GEDI, ICESat-2, Sentinel-5P)

---

## Executive Summary

HDF5 read support is a recurring friction point across multiple WNG toolsets:
- **Lidar tools** (`wblidar`, lidar QA/confidence pipelines): GEDI, ICESat-2
- **Remote sensing tools** (`wbtools_oss`, thermal/spectral analytics): Sentinel-5P atmospheric products
- **Teaching workflows**: Reproducible multi-source analyses without external format conversion

Rather than implementing a full-spec HDF5 reader (intractable in pure Rust), this roadmap
proposes a **scoped, targeted reader** covering only the product layouts and filter combinations
actually encountered in practice. Feasibility estimate: **3–6 weeks**, post-WNG launch.

**Key decision:** Should this live in `wbraster/src/formats/hdf5.rs` or as a separate `wbhdf5` crate
that can be leveraged by both `wbraster` and `wblidar` independently? Recommended: separate
`wbhdf5` crate for modularity and reuse.

---

## 1. Problem Statement

### Current Friction Points

| Dataset | Format | Toolset | Current Workaround |
|---|---|---|---|
| GEDI L2A/L2B | HDF5 | wblidar | User must pre-convert via GDAL or rasterize separately |
| ICESat-2 ATL03/ATL08 | HDF5 | wblidar | User must pre-convert via nsidc-convert or gedi4-subsetter scripts |
| Sentinel-5P L2 | NetCDF (HDF5-backed) | wbtools_oss | User must extract via `xarray` or `gdal_translate` |
| ALOS-2 MLC (some distribution portals) | HDF5 | wbtools_oss (PolSAR) | User must pre-convert via SNAP or PolSARpro |

Each conversion step introduces potential data loss, complicates reproducibility, and raises the
barrier to entry for teaching users. Native HDF5 ingestion removes this friction entirely.

### Why Not Full HDF5 Spec?

A complete HDF5 implementation in pure Rust would require:
- 10,000+ lines to handle all filter types (SZIP, LZF, Scaleoffset, etc.)
- Complex B-tree v2 implementation (non-trivial state management)
- Ongoing maintenance burden for edge cases
- Likely still need C linkage for SZIP filter (proprietary, no pure-Rust port)

**Not justified for the use case.** A targeted reader handling the small set of filters and
product layouts actually in use is maintainable and practical.

---

## 2. Scope: Datasets Covered

### Tier 1 (Core: WNG launch + 6 months)

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

### Tier 2 (Extended: 12+ months, Complex Type Dependent)

**ALOS-2 PALSAR-2 MLC (Multi-Look Complex)** — L-Band PolSAR Coherency Matrices

- File: HDF5, typically GZIP
- Filters: GZIP only
- Schema: Fixed compound datasets; complex-valued float32/float64 per element
- **Blocker:** Requires `wbraster` complex data type support (see Section 5 below)
- Why: L-band PolSAR data completeness for vegetation penetration studies; enables full-polarimetry workflows without SNAP export step
- Timeline: Phase 2 of `wbhdf5` implementation, contingent on `wbraster` `ComplexF32`/`ComplexF64` support

**Sentinel-5P L2** — Atmospheric Columns (NO₂, O₃, CO, CH₄)

- File: NetCDF (HDF5 backend) via CF conventions
- Filters: GZIP + some Scaleoffset
- Why: Teaching atmospheric correction / validation context for optical tools
- Lower priority than Tier 1 (can use GDAL for now; nice-to-have for reproducibility)

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

Without `ComplexF32`/`ComplexF64` support in `wbraster`, even if `wbhdf5` successfully
reads the HDF5 file, construction of a `Raster` will fail with a data type mismatch.

### Recommendation

**ALOS-2 HDF5 reading should be added to this roadmap as Phase 2, but its completion
depends on a **separate architectural decision** in `wbraster` to support complex data types.**

Once complex types are available in `wbraster`, extending `wbhdf5` to read ALOS-2 MLC
files is trivial (additional 3–5 days of work). The HDF5 format reading is straightforward;
the blocker is the raster data model.

### Related Scope Items

Complex data type support in `wbraster` would also enable:
- Interferometric SAR products (phase, coherence)
- Complex spectral unmixing residuals (if modeled as complex)
- Waveform-domain lidar decomposition (amplitude + phase)

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

### Option B: Separate `wbhdf5` crate (recommended)

**Pros:**
- Reusable by `wbraster`, `wblidar`, future tools (wblidar point ingest, wbgeospectral endmember libraries, etc.)
- Cleaner dependency boundary
- Can be versioned/released independently
- Simpler for community contributions / peer review
- Pure-Rust HDF5 primitive could have academic interest outside WNG

**Cons:**
- Additional crate to maintain
- `wbraster` users need explicit `wbhdf5` import for HDF5 support (not transparent)
- Slightly more boilerplate in wbraster format dispatch

**Decision:** Option B. Create `wbhdf5` as a focused, peer-reviewable library. `wbraster` gains an optional feature `hdf5_support` that adds `RasterFormat::Hdf5` and depends on `wbhdf5`.

### Module Structure for `wbhdf5`

```
wbhdf5/
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

### Phase 4: Integration with `wbraster` (~3 days)

**Deliverables:**
- [ ] `wbraster/src/formats/hdf5.rs` dispatch layer
- [ ] `Raster` construction from HDF5 datasets
- [ ] GIS metadata propagation (CRS from attributes, georeferencing from GEDI/ICESat-2 layouts)
- [ ] Feature gate: `cargo build --features hdf5_support`

**Validation:**
- `wbraster::Raster::read("data.h5:///path/to/dataset")` succeeds
- Output raster dimensions and nodata match reference

### Phase 5: Testing + Documentation (~1 week)

**Deliverables:**
- [ ] Unit tests for B-tree traversal (synthetic trees + GEDI sample validation)
- [ ] Integration tests: read GEDI/ICESat-2 tiles → export as GeoTIFF → validate pixel values
- [ ] Design documentation (B-tree algorithm, chunk layout assumptions, format notes)
- [ ] User guide: how to read GEDI/ICESat-2 via wbraster

### Phase 2 (Contingent): ALOS-2 PALSAR-2 MLC Support (~1 week)

**Prerequisites:**
- `wbraster` gains `ComplexF32` and `ComplexF64` variants in `DataType` enum
- `wbraster` I/O paths updated to handle complex values

**Deliverables:**
- [ ] `wbhdf5` dataset reader extended to recognize complex HDF5 datatypes
- [ ] Mapping of HDF5 complex types → `wbraster` `ComplexF32`/`ComplexF64`
- [ ] Endianness conversion for complex components
- [ ] Integration tests: read ALOS-2 MLC tile → validate coherency matrix elements

**Timeline:** Begin after Phase 1 validation and once `wbraster` complex type support is available (estimated Q1 2027).

---

## 6. Risk Mitigation

### Risk: HDF5 File Format Variations

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
2. **Post-Launch Priority (Q3–Q4 2026):** Begin `wbhdf5` design + Phase 1–2 implementation while community feedback on sprint tools is gathered.
3. **Integration (Q1 2027):** Wire `wbhdf5` into `wbraster` and `wblidar` workloads.

This sequencing allows:
- WNG to launch on schedule without HDF5 dependency
- Community to validate the remote sensing sprint before adding lidar-specific format support
- Time to collect real GEDI/ICeSat-2 samples and validate the B-tree implementation thoroughly

---

## 8. Success Criteria

**By end of Phase 5:**

- [ ] Pure-Rust `wbhdf5` crate reads GEDI L2B, ICESat-2 ATL03/ATL08 files without errors
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
- HDF4 support (completely different format; separate effort if needed)
- Parallel I/O (not applicable for teaching workflows)
