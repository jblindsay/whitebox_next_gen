# wblidar Implementation Plan

## Overview

This document describes the current architecture, capabilities, and implementation strategy for **wblidar**, a high-performance Rust library for reading and writing LiDAR point-cloud data in industry-standard formats.

**Key Principle**: Minimal allocations, minimal external dependencies, standards-compliant encoding only (no legacy formats).

---

## 1. Supported Formats

| Format | Read | Write | PDRF Support | Notes |
|--------|------|-------|------|-------|
| **LAS**    | ✓ | ✓ | 0-15 | Versions 1.1–1.5 |
| **LAZ**    | ✓ | ✓ | 0-15 | Standards-compliant LASzip v2/v3 only |
| **COPC**   | ✓ | ✓ | Input 0-15 (stored as 6/7/8) | Cloud-Optimized Point Cloud (LAS 1.4 + EPT hierarchy) |
| **PLY**    | ✓ | ✓ | N/A | ASCII & binary (little/big-endian) |
| **E57**    | ✓ | ✓ | N/A | ASTM E2807; CRC-32 validation; optional zlib blobs |

### Detailed Format Support

#### LAS / LAZ
- **Read**: Point Data Record Formats 0–15 (PDRF)
- **Write**: 
    - **Point10** (PDRF0–5): Pointwise arithmetic encoding, lazy streaming
    - **Point14 family** (PDRF6–15): Layered arithmetic encoding, eager full-buffer loading
- **LAS Version**: Reads/writes 1.1–1.5 (with CRS from WKT/EPSG VLRs)
- **Compression**: LASzip v2 (Point10) and v3 (Point14) only

#### COPC
- **Class Specification**: Cloud-Optimized Point Cloud (stored in LAZ with EPT hierarchy)
- **Point Format**: Accepts input PDRF0-15, stores in Point14 family (PDRF6/7/8) based on available attributes
- **Octree Structure**:
  - Variable-depth (default max 8 levels)
  - Configurable max points per node (50k–100k default)
    - **Spatial Ordering**: User-configurable (`Auto`, `Morton`, `Hilbert`); `Auto` keeps GPS-time-first fallback-to-Morton behavior
- **CRS Support**: Full WKT/EPSG propagation through VLRs
- **Features**:
  - Node-level point buffering and compression
  - Spatial bounding-box hierarchy for query optimization
  - GPSTime separation for predictive efficiency (when applicable)

#### PLY, E57
- Supported via frontend API but not core focus
- E57 includes optional zlib compression, CRC-32 checksums per spec

---

## 2. Core Architecture

### 2.1 Point10 (PDRF0–3) – Lazy Streaming

**Design**: Process points one chunk at a time without materializing entire point cloud.

```rust
// Internal workflow:
1. Read/parse VLRs & LAS header
2. For each chunk:
   a. Read compressed chunk bytes
   b. Decode with LASzip v2 arithmetic codec
   c. Yield PointRecord one-by-one
   d. Release buffer (no buffering between chunks)
```

**Writer Pattern**:
- Point0 (classic): No GPS time, no RGB → smallest footprint
- Point1–3: GPS time and/or RGB added

**Pointwise Arithmetic Encoding**:
- Each point encoded independently with point-wise context
- Delta prediction applied to X, Y, Z
- RGB/classification ranges predicted using previous point context
- Model resets per point

### 2.2 Point14 (PDRF6–8) – Eager Layered Loading

**Design**: Load all points into memory first, then decode using **layered arithmetic encoding**.

```rust
// Internal workflow:
1. Parse VLRs & LAS header
2. Read ALL chunk table entries
3. Buffer entire point cloud in memory (one PointRecord per point)
4. Decode using layered arithmetic codec:
   a. X layer: all X values with delta prediction
   b. Y layer: all Y values with delta prediction
   c. Z layer: all Z values with delta prediction
   d. Intensity layer: all intensities
   // ... etc for each attribute layer
5. Return materialized point cloud
```

**Why Eager Loading?**
- Layered encoding requires global statistics to initialize predictor context
- Cannot decode one point without full layer context
- Trade-off: Higher memory (all points in RAM) for correctness + performance

**Supported Point Formats**:
- Pdrf6: Point14 without RGB or NIR
- Pdrf7: Point14 with RGB
- Pdrf8: Point14 with NIR (rarely used in practice)

**Partial Point14 Handling**:
- Control via environment variable: `WBLIDAR_FAIL_ON_PARTIAL_POINT14`
- Default: Lenient (pad missing attributes with 0)
- Strict mode: Fail with error if partial points detected
- Function: `fail_on_partial_point14()` used internally

### 2.3 COPC – Octree + Point14

**Design**: Spatial index for large-scale point clouds with hierarchical query support.

```rust
// One octree node stores:
1. Spatial bounds: (center_x, center_y, center_z, halfsize)
2. Point cloud metadata: count, bounding box, bounds in georeferenced coordinates
3. Compressed point data (Point14 layered encoding)
4. Child node references (up to 8 children, max_depth=8 typical)
```

**Spatial Ordering** (Morton Curve):
- Points sorted by Morton (Z-order space-filling curve) key before writing
- Improves delta predictor performance (nearby points in sequence)
- Reduces redundant coordinates in compressed output
- Reduces file size ~5–10% vs unsorted

**Node Splitting Strategy**:
- Split when point count exceeds `max_points_per_node`
- 50k points per node: balanced compression + query granularity (default)
- 100k points per node: tighter compression, larger query results
- `max_depth=8`: Maximum octree depth (256 max leaf nodes per side)

**Writer Pattern**:
```rust
let cfg = CopcWriterConfig {
    las: WriterConfig { /* LAS metadata */ },
    center_x, center_y, center_z, halfsize,  // Root octree cell
    spacing,                                   // Grid spacing for initial LOD
    max_depth: 8,
    max_points_per_node: 50_000,  // Tunable (10k–100k typical)
    compression_level: 6,           // COPC writer path currently does not use this knob directly
};

let mut writer = CopcWriter::new(output_file, cfg);
for point in &all_points {
    writer.write_point(&point)?;
}
writer.finish()?;  // Triggers octree building & writing
```

---

## 3. Compression Strategy

### 3.1 Removed: WB-Native DEFLATE

**Rationale**: Simplified codebase, improved maintainability, standards compliance.

**What was removed**:
- `flate2` dependency
- `encode_wb_native_deflate_chunk()` / `decode_wb_native_deflate_chunk()` functions
- Sequential chunk reading path (Point10 fallback decoder)
- Legacy reader test cases (except for backward compatibility testing)

**Impact**:
- All LAZ files now output standards-compliant LASzip v2/v3 only
- QGIS, PDAL, CloudCompare fully compatible (verified via external tools)
- File sizes unchanged (standards LASzip v2/v3 equivalent compression)

**Current Status**: ✅ Removed. All 177 unit tests passing.

### 3.2 Active: LASzip v2 (Point10)

**Algorithm**: Pointwise arithmetic encoding + optional entropy residual correction.

**Configuration**:
```rust
let mut cfg = LazWriterConfig::default();
cfg.las.point_data_format = PointDataFormat::Pdrf0;  // or Pdrf1/2/3
cfg.chunk_size = 50_000;          // Points per chunk
cfg.compression_level = 6;         // 0–9 (Point14 uses this to tune effective chunk target size)
```

**Compression Performance** (typical):
- Original LAS: 100% (baseline)
- LAZ Point10: ~ 25–35% (4:1 to 2.8:1 compression)
- Depends on: point density, terrain regularity, RGB/intensity variation

### 3.3 Active: LASzip v3 (Point14) with Layered Encoding

**Algorithm**: Layered arithmetic encoding on grouped attributes.

**Layers** (processed sequentially):
1. **X/Y/Z**: Predictive delta encoding with spatial context
2. **GPS Time**: Separate predictor for temporal deltas (when present)
3. **Intensity**: Range prediction based on return count
4. **Classification/Return**: Bit-level encoding
5. **RGB**: Packed bit-field encoding (if Pdrf7/8)
6. **NIR** (Pdrf8 only): Similar to RGB layer

**Compression Performance** (typical):
- Original LAS Point14: 100%
- LAZ Point14: ~ 30–40% (3.3:1 to 2.5:1 compression)
- Better than Point10 due to additional attributes/structure

**RGB14 Encoding Note**:
- Bit allocations: 14-bit + 64-symbol symbol alphabet (standardized)
- Fixed model sizes: 128-symbol predictor alphabet
- Bug history: Was using 64-symbol direct codebook (fixed in earlier session)
- All tests pass ✅

### 3.4 COPC Compression Tuning

**Chunk Size Impact** (LAZ to COPC conversion):
- 10k points/node: Overhead from many small chunks (63.5 MB observed)
- 50k points/node: Balanced compression (57.5 MB observed) ← default
- 100k points/node: Tighter packing (~5–10% smaller than 50k)

**Ordering Impact** (COPC Morton curve):
- GPS-time sort (unsorted spatially): 63 MB
- Morton curve sort: 57.5 MB (−8.7%)
- Improvement: Predictor sees nearby points in sequence

**File Size Gap Analysis** (vs. QGIS-generated COPC):
- wblidar (50k Morton): 57.5 MB
- QGIS COPC (reference)  : 31.7 MB
- Gap: ~81% larger (unexplained, likely different source data or tuning)

---

## 4. Tunable Parameters

### Reader Configuration

#### LasReader
```rust
let reader = LasReader::new(buf_reader)?;
let header = reader.header();  // LasHeader with PDRF, scales, bounds
let crs = reader.crs();        // Optional CRS from VLRs
```

**Environment Variables**:
- `WBLIDAR_FAIL_ON_PARTIAL_POINT14`: Set to enable strict mode (fail on partial Point14 records)

#### CopcReader
```rust
let copc = CopcReader::new(buf_reader)?;
let root = copc.root_node()?;
let children = copc.children_of(&root)?;
// Spatial queries on node hierarchy
```

**Features**:
- Node-level spatial bounding boxes
- Lazy loading of compressed chunks
- Optional query bounds filtering

### Writer Configuration

#### LazWriterConfig
```rust
pub struct LazWriterConfig {
    pub las: WriterConfig,      // Base LAS settings
    pub chunk_size: usize,       // Points per chunk (default 50k)
    pub compression_level: u32,  // 0–9 (default 6, currently ignored)
    pub standards_compliant: bool, // Deprecated field (always true)
}
```

**Recommended Settings**:
- `chunk_size = 50_000`: Standard balance of I/O vs. compression efficiency
- `chunk_size = 100_000`: For very large clouds (>100M points)
- `chunk_size = 10_000`: Small-scale or streaming scenarios

#### CopcWriterConfig
```rust
pub struct CopcWriterConfig {
    pub las: WriterConfig,
    pub center_x, center_y, center_z: f64,  // Root octree cell center
    pub halfsize: f64,                       // Root octree half-size
    pub spacing: f64,                        // LOD spacing at root
    pub max_depth: u32,                      // Octree depth (default 8)
    pub max_points_per_node: usize,          // Polishing threshold (default 10k)
    pub compression_level: u32,              // Ignored for Point14 (future use)
}
```

**Spatial Configuration**:
- Choose `center_x/y/z` to enclose all points
- `halfsize` should approximate √(volume of bounding box)
- `spacing`: Grid cell size at root level (typically `2*halfsize / 1024`)

**Node Splitting**:
- Experiment with `max_points_per_node ∈ [10k, 50k, 100k]`
- Higher values → fewer nodes, tighter compression, slower spatial queries
- Lower values → more nodes, looser compression, faster point filtering

---

## 5. Known Limitations

### Point Format Restrictions

1. **COPC storage normalization**: Input PDRF0–15 is accepted, but output COPC storage is normalized to PDRF6/7/8

### COPC-Specific

1. **Eager Loading**: All points materialized in RAM (problematic for >10GB clouds)
   - Mitigation: Tile-based external octree building recommended for massive datasets
2. **No Incremental Updates**: Cannot append points to existing COPC file
3. **No Adaptive Ordering Heuristics**: Ordering policy is user-selected (`Auto`/`Morton`/`Hilbert`) and not yet auto-tuned per dataset
4. **Feature-gated Parallelism**: Multi-threading is available in selected paths via the umbrella `parallel` feature or the granular `copc-parallel` / `laz-parallel` flags, but is not enabled by default

### Performance Boundaries

- **Streaming LAZ (Point10)**: Efficient up to ~500M points
- **Eager Point14/COPC**: Practical limit ~50M points (memory constraint)
- **Compression overhead**: ~5–10% for Point14 arithmetic encoding math

---

## 6. Development Workflow

### Testing Strategy

**Unit Tests** (174 total, all passing):
- `test_standards_compliant_point10_pdrf0_roundtrip_reads_via_lazreader` [✓]
- `test_standards_compliant_point14_roundtrip` [✓]
- `test_copc_octree_hierarchy` [✓]
- Point14 partial point tracking [✓]
- RGB14 encoding roundtrip [✓]

**Validation Tools**:
- **PDAL**: `pdal info` & `pdal validate` (LAS/LAZ roundtrip)
- **QGIS**: Visual inspection for spatial correctness, metadata
- **CloudCompare**: Point cloud alignment, profile comparison
- **CoPC Conformance**: Upload to https://validate.copc.io for external validation

### Build & Run

```bash
# Build all crates
cargo build -p wblidar --all-targets

# Run tests
cargo test -p wblidar

# Run public examples
cargo run -p wblidar --example laz_write_standards -- input.las output.laz
cargo run -p wblidar --example las_to_copc -- input.las output.copc.laz
cargo run -p wblidar --example laz_to_copc -- input.laz output.copc.laz
cargo run -p wblidar --example copc_query_demo -- input.copc.laz

# Internal tools (not Cargo examples; live in tools/examples/)
# Benchmarks: tools/examples/copc_compression_tuning.rs, laz_parallel_parity_benchmark.rs, etc.
# Diagnostics: tools/examples/chunk_table_diagnostic.rs, inspect_copc_node.rs, etc.
# Artifact generators: tools/examples/generate_clean_test_artifacts.rs, copc_validation_fixture_pack.rs, etc.

# Release build (optimized)
cargo build -p wblidar --release

# Clean codebase (fix warnings)
cargo fix --lib -p wblidar --allow-dirty
```

### Recent Code Changes (This Session)

| Change | Module | Impact | Status |
|--------|--------|--------|--------|
| RGB14 bit-6 algorithm fix | `laz/codec.rs` | Fixed QGIS reading; all tests pass | ✅ Complete |
| Remove flate2 dependency | `Cargo.toml` | Simplified build; no wb-native | ✅ Complete |
| Remove wb-native encode/decode | `laz/writer.rs`, `laz/reader.rs` | Standards-only output | ✅ Complete |
| Default standards_compliant=true | `laz/writer.rs` | Simplified config logic | ✅ Complete |
| LAZ chunk tuning (10k→50k) | `examples/` | ~6.8% size improvement | ✅ Benchmarked |
| COPC Morton curve ordering | `copc/writer.rs` | ~8.7% compression gain | ✅ Implemented |
| Generate clean test artifacts | `examples/` | Artifact generation pipeline | ✅ Working |

---

## 7. API Examples

### Example 1: Read LAS, Convert to LAZ (Point10)

```rust
use std::fs::File;
use std::io::{BufReader, BufWriter};
use wblidar::las::reader::LasReader;
use wblidar::laz::{LazWriter, LazWriterConfig};
use wblidar::las::PointDataFormat;
use wblidar::io::{PointReader, PointWriter};
use wblidar::PointRecord;

fn main() -> wblidar::Result<()> {
    // Read header metadata
    let in_file = File::open("input.las")?;
    let mut reader = LasReader::new(BufReader::new(in_file))?;
    let header = reader.header().clone();
    let crs = reader.crs().cloned();

    // Configure writer
    let mut cfg = LazWriterConfig::default();
    cfg.las.point_data_format = PointDataFormat::Pdrf0;
    cfg.las.x_scale = header.x_scale;
    cfg.las.y_scale = header.y_scale;
    cfg.las.z_scale = header.z_scale;
    cfg.las.x_offset = header.x_offset;
    cfg.las.y_offset = header.y_offset;
    cfg.las.z_offset = header.z_offset;
    cfg.las.crs = crs;
    cfg.chunk_size = 50_000;

    // Convert to LAZ
    let out_file = File::create("output.laz")?;
    let mut writer = LazWriter::new(BufWriter::new(out_file), cfg)?;
    let mut point = PointRecord::default();
    while reader.read_point(&mut point)? {
        writer.write_point(&point)?;
    }
    writer.finish()?;

    Ok(())
}
```

### Example 2: Create COPC from Point Cloud

```rust
use wblidar::copc::{CopcWriter, CopcWriterConfig};
use wblidar::las::writer::WriterConfig;
use wblidar::las::PointDataFormat;
use wblidar::io::PointWriter;

fn create_copc(points: &[PointRecord], output_path: &str) -> wblidar::Result<()> {
    // Compute bounds
    let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_z, mut max_z) = (f64::INFINITY, f64::NEG_INFINITY);

    for pt in points {
        min_x = min_x.min(pt.x);
        max_x = max_x.max(pt.x);
        min_y = min_y.min(pt.y);
        max_y = max_y.max(pt.y);
        min_z = min_z.min(pt.z);
        max_z = max_z.max(pt.z);
    }

    let center_x = (min_x + max_x) * 0.5;
    let center_y = (min_y + max_y) * 0.5;
    let center_z = (min_z + max_z) * 0.5;
    let halfsize = ((max_x - min_x).max(max_y - min_y).max(max_z - min_z) * 0.5).max(1.0);

    // Configure COPC writer
    let las_cfg = WriterConfig {
        point_data_format: PointDataFormat::Pdrf7,
        x_scale: 0.001,
        y_scale: 0.001,
        z_scale: 0.001,
        x_offset: min_x,
        y_offset: min_y,
        z_offset: min_z,
        system_identifier: "wblidar".to_string(),
        generating_software: "example.rs".to_string(),
        vlrs: Vec::new(),
        crs: None,
        extra_bytes_per_point: 0,
    };

    let copc_cfg = CopcWriterConfig {
        las: las_cfg,
        center_x,
        center_y,
        center_z,
        halfsize,
        spacing: (halfsize * 2.0 / 1024.0).max(0.000_001),
        max_depth: 8,
        max_points_per_node: 50_000,
        compression_level: 6,
    };

    // Write COPC
    let out_file = File::create(output_path)?;
    let mut writer = CopcWriter::new(BufWriter::new(out_file), copc_cfg);
    for pt in points {
        writer.write_point(pt)?;
    }
    writer.finish()?;

    Ok(())
}
```

### Example 3: Query COPC by Spatial Bounds

```rust
use wblidar::copc::CopcReader;
use std::fs::File;
use std::io::BufReader;

fn query_copc(path: &str, x_min: f64, y_min: f64, z_min: f64,
              x_max: f64, y_max: f64, z_max: f64) -> wblidar::Result<Vec<PointRecord>> {
    let file = File::open(path)?;
    let mut reader = CopcReader::new(BufReader::new(file))?;
    
    let mut results = Vec::new();
    let root = reader.root_node()?;
    
    // Recursively query octree nodes within bounds
    fn recurse(reader: &mut CopcReader, node: &CopcNode, 
               bounds: (f64, f64, f64, f64, f64, f64),
               results: &mut Vec<PointRecord>) -> wblidar::Result<()> {
        // Check if node overlaps query bounds
        if !node_overlaps(&node, bounds) {
            return Ok(());
        }
        
        // Read points from node
        let points = reader.read_node_points(&node)?;
        for pt in points {
            if point_in_bounds(&pt, bounds) {
                results.push(pt);
            }
        }
        
        // Recurse to children
        let children = reader.children_of(&node)?;
        for child in children {
            recurse(reader, &child, bounds, results)?;
        }
        
        Ok(())
    }
    
    recurse(&mut reader, &root, (x_min, y_min, z_min, x_max, y_max, z_max), &mut results)?;
    Ok(results)
}
```

---

## 8. Maintenance & Future Roadmap

### Short-term (Next Release)

1. **Warning Cleanup (Phase 1 complete)**: Low-risk clippy cleanups have landed; next pass targets remaining style/perf hotspots (for example numeric grouping, argument-count hotspots)
2. **Compression Config Tuning (implemented, tuning in progress)**: Point14 now uses `compression_level` to tune effective chunk target size; next step is larger-fixture benchmarking and tuning level-to-chunk breakpoints
3. **Parallel Coverage Expansion**: Extend feature-gated parallelism to additional safe hotspots

**Execution Plan for Item 3 (safety-first):**
- Add opt-in feature gates for each new parallel path (keep serial default unchanged)
- Prioritize embarrassingly parallel units with independent outputs (for example COPC per-node preprocess/compress work before final deterministic write ordering)
- Keep deterministic behavior by preserving stable ordering at merge boundaries (explicit tie-break keys where needed)
- Add threshold gating (env/config) so small workloads stay serial and avoid overhead
- Require parity tests and benchmark checks per hotspot before enabling in release defaults

### Medium-term (Roadmap)

1. **LAS 1.5 Hardening**: Expand external fixture/interoperability coverage for PDRF11–15
2. **Streaming COPC**: Tile-based approach for >10GB clouds
3. **Adaptive Ordering Heuristics**: Add dataset-aware Auto tuning for Morton/Hilbert selection
4. **COPC Append**: Incremental point insertion into existing hierarchy
5. **Point14 Streaming**: Investigate lazy-loading possibility for Point14

### Long-term

1. **GPU Acceleration**: CUDA/Metal for compression/decompression
2. **Other Formats**: PCD, LAD, etc.
3. **CLI Tool**: Dedicated command-line utility (`wblidar-cli`)

---

## 9. Dependency Minimization

### Current External Dependencies

**Core Crates**:
- `wbprojection`: CRS transformation (separate crate)
- `wide`: SIMD operations for predictor performance (optional, benchmarked)

**No External**:
- ✅ Pure Rust arithmetic codec (no DEFLATE / zlib)
- ✅ Pure Rust octree building
- ✅ No C/C++ FFI

### Removed Dependencies

- ❌ `flate2` (1.0): Removed (wb-native DEFLATE abandoned)

**Rationale**: Minimize CVE surface, optimize pure-Rust performance path, reduce binary size.

---

## 10. Testing Checklist

### Before Each Release

- [ ] All 177 unit tests pass (`cargo test`)
- [ ] PDAL roundtrip validation (`pdal info`, `pdal validate`)
- [ ] QGIS visual inspection (metadata, point cloud appearance)
- [ ] CloudCompare alignment (profile comparison)
- [ ] CoPC conformance upload (https://validate.copc.io)
- [ ] File size regression check (vs. baseline)
- [ ] Example scripts execute without error

### Known Test Matrices

**Point Formats**: Pdrf0-Pdrf15
**Compression**: Standards-compliant LASzip v2/v3 only
**CRS**: WKT + EPSG codes (both populated in test files)
**Spatial**: Morton order; GPS-time separation (Pdrf1/3/7)

---

## 11. References

### Standards Documents

- **LAS Specification 1.4** (R15): https://www.asprs.org/a/society/committees/standards/
- **LASzip Specification** (v2 & v3): https://github.com/laszip/laszip
- **CoPC Standard 2.0**: https://copc.io/
- **ASTM E2807** (E57 format): https://www.astm.org/

### External Tools & Validation

- **PDAL** (Point Data Abstraction Library): https://pdal.io/
- **QGIS**: Open-source GIS (excellent for visual validation)
- **CloudCompare**: Point cloud inspection & alignment
- **Azure Tiles**: COPC cloud storage, COG integration

---

## 12. Document History

| Date | Author | Change |
|------|--------|--------|
| 2025-03-30 | Implementation Review | Initial comprehensive implementation plan (this session) |

---

**Last Updated**: 2025-03-30  
**Status**: Comprehensive implementation plan after legacy code cleanup and artifact generation pipeline setup.
