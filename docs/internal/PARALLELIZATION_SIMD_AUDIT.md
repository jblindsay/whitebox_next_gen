# Parallelization & SIMD Optimization Audit: wblidar Crate

**Date**: March 30, 2026  
**Scope**: LAS, LAZ, E57, PLY reader/writer implementations  
**Dependencies**: 
- `rayon` v1.10 (optional, feature-gated: `copc-parallel`)
- `wide` v1.2 (always available; provides portable SIMD with software fallback)

---

## Executive Summary

| Format | Reader Parallelization | Writer Parallelization | SIMD Optimizations | Feature Flags |
|--------|------------------------|-------------------------|--------------------|---------------|
| **LAS** | ❌ None | ⚠️ SIMD only (no threading) | ✅ SIMD bbox accumulation | None |
| **LAZ** | ❌ None (chunk-at-a-time) | ⚠️ SIMD only (no threading) | ✅ SIMD rounding/coords | None |
| **COPC** | ❌ None (node-at-a-time) | ✅ **Parallel node encoding** | ✅ SIMD bbox, spatial sort | `copc-parallel` (Rayon) |
| **E57** | ❌ None (sequential page load) | ❌ None (buffered, no parallelization) | ❌ None | None |
| **PLY** | ❌ None (streaming per-point) | ❌ None (per-point encode) | ❌ None | None |

**Key Insight**: Only COPC has comprehensive parallelization. LAS/LAZ are purely sequential with targeted SIMD optimizations. E57/PLY are strictly sequential with no SIMD or parallelism.

---

## 1. LAS (Uncompressed) Format

### Reader Implementation
**File**: [src/las/reader.rs](src/las/reader.rs)

#### Current State:
- **Parallelization**: ❌ None
- **SIMD**: ✅ Per-point coordinate scaling (f64x4)
  - **Line 122–131**: `decode_xyz()` uses `f64x4` SIMD to compute x/y/z scale+offset in one operation
  - Codec: `[xi as f64, yi as f64, zi as f64, 0.0] * scale + offset` (4th lane unused)
  
#### Streaming Model:
- Sequential point-by-point reading from file buffer
- Per-format PDRF decoder dispatch (no bulk decompression needed)
- 15 PDRF variants (0-15) handled via explicit match dispatch
  
#### Hotspots:
1. **Coordinate decoding loop** – Per-point SIMD is already optimized; no batching opportunity without buffering entire dataset
2. **Flag/metadata unpacking** – Bit-level operations per byte; could theoretically use SIMD but overhead likely exceeds gain
3. **VLR parsing** – Negligible (once per file)

#### Opportunities:
- **Batch coordinate transformation** (Low complexity, Medium impact): If reader were modified to buffer N points before returning, could batch SIMD on coordinate arrays. But this breaks streaming semantics; not recommended without API change.
- **Parallel VLR parsing** (Low complexity, Low impact): VLRs are sequential and typically small; parallelization overhead likely exceeds benefit.

---

### Writer Implementation
**File**: [src/las/writer.rs](src/las/writer.rs)

#### Current State:
- **Parallelization**: ❌ None
- **SIMD**: ✅ Per-point bounding-box accumulation (f64x4)
  - **Lines 279–289**: Branchless SIMD min/max on [x, y, z, _] lanes
  - Accumulates running bbox using `mins.min(coords)` and `maxs.max(coords)` in vectorized ops
  
#### Streaming Model:
- Per-point sequential writes to output buffer
- Header back-patched at finish with final bbox and point counts

#### Hotspots:
1. **Bounding box accumulation** – Already SIMD-optimized (lines 279–289); does 3D min/max in 2 SIMD ops vs. 6 scalar ops.
2. **Point encode dispatch** – Format-specific byte packing; cannot be parallelized without buffering.

#### Opportunities:
- **Batch bbox computation** (Low complexity, Low impact): Similar to reader; streaming model doesn't favor buffering.
- **Parallel header back-patching** (Low complexity, Negligible impact): Single seek+write operation; parallelization overhead exceeds cost.

---

## 2. LAZ (Compressed) Format

### Reader Implementation
**File**: [src/laz/reader.rs](src/laz/reader.rs)

#### Current State:
- **Parallelization**: ❌ None
- **SIMD**: ✅ Per-point coordinate rounding (in codec)
  - **File**: [src/laz/codec.rs](src/laz/codec.rs), **Line 27**: `f64x4::new([p.x, p.y, p.z, 0.0]).round()` SIMD rounding before delta encoding
  
#### Streaming Model:
- Chunk-at-a-time decompression
- Chunk table read determines byte offsets for each compressed block
- DEFLATE decompression then LASzip arithmetic decoding per chunk

#### Hotspots (in order of impact):
1. **Point decompression loop** (HIGH impact)
   - **File**: [src/laz/standard_point10.rs](src/laz/standard_point10.rs), **Lines 749+**: `decode_standard_pointwise_chunk_point10_v2()`
   - Sequential arithmetic decoding of each point field (x, y, z, intensity, flags, etc.)
   - Cannot parallelize without breaking arithmetic coder state dependency
   - **Bottleneck**: Single-threaded arithmetic decoder per chunk; scales linearly with chunk size

2. **DEFLATE decompression** (HIGH impact)
   - Handled via Rust's built-in `flate2` (already uses available parallelism for input buffering)
   - Chunks are decompressed sequentially (no inter-chunk parallelism)

3. **Chunk table validation** (LOW impact)
   - **File**: [src/laz/reader.rs](src/laz/reader.rs), **Lines 34–99**: `validate_standard_chunk_table_entries()`
   - Could be parallelized for very large chunk tables but is typically negligible

#### Dependencies:
- Arithmetic coder maintains **per-point state machine** (cannot parallelize within a chunk)
- Data dependencies: Each `decompress_symbol()` call depends on previous model state

#### Opportunities:
- **Parallel chunk decompression** (Medium complexity, HIGH impact)
  - Chunks are independent; can spawn rayon thread pool to decompress multiple chunks concurrently
  - Requires buffering compressed chunks into memory (already done in some reader paths)
  - **Estimate**: 4–8× speedup on 8-core systems for large LAZ files

- **SIMD within arithmetic decoder** (High complexity, Medium impact)
  - Arithmetic coding is inherently sequential (state updates depend on previous bit)
  - Micro-optimizations possible (branch prediction, bit-level cache efficiency) but limited upside

---

### Writer Implementation
**File**: [src/laz/writer.rs](src/laz/writer.rs)

#### Current State:
- **Parallelization**: ❌ None
- **SIMD**: ✅ Per-point coordinate rounding (f64x4)
  - **Lines 9**: Import of `wide::f64x4`
  - Used implicitly via codec path (same as reader)

#### Streaming Model:
- Buffering chunk to `chunk_buf` (default 50,000 points)
- On flush, encodes chunk via `encode_standard_pointwise_chunk_point10_v2()`
- DEFLATE compress + write chunk table entry

#### Hotspots:
1. **Chunk encoding loop** (HIGH impact)
   - **File**: [src/laz/standard_point10_write.rs](src/laz/standard_point10_write.rs), **Lines 774+**: `encode_standard_pointwise_chunk_point10_v2()`
   - Arithmetic encoding of each point; stateful, cannot parallelize within chunk

2. **DEFLATE compression** (HIGH impact)
   - Chunks compressed sequentially
   - Compression level 0–9 (default 6); higher levels are CPU-bound

#### Opportunities:
- **Parallel chunk encoding** (Medium complexity, HIGH impact)
  - Multiple chunks can be encoded in parallel if they arrive from upstream buffering
  - Total speedup depends on buffering strategy (not currently implemented)

- **Tunable compression level** (Low complexity, Low-Medium impact)
  - Already supported via `compression_level` config; parallelization of DEFLATE itself would help

---

## 3. COPC (Cloud-Optimized Point Cloud) Format

### Reader Implementation
**File**: [src/copc/reader.rs](src/copc/reader.rs)

#### Current State:
- **Parallelization**: ❌ None (node-at-a-time sequential read)
- **SIMD**: ⚠️ Per-point only (via LAZ codec for decompressed points)
- **Feature Flag**: `copc-parallel` controls writer parallelization (reader is always sequential)

#### Streaming Model:
- Hierarchical point cloud with voxel-based spatial indexing
- Nodes are read one at a time via hierarchy traversal
- Each node's point data is LAZ-compressed; decompression is sequential

#### Opportunities:
- **Parallel multi-node read** (Medium complexity, HIGH impact)
  - Nodes in different octree levels can be read concurrently
  - Requires `copc-parallel` feature and rayon

---

### Writer Implementation
**File**: [src/copc/writer.rs](src/copc/writer.rs)

#### Current State:
- **Parallelization**: ✅ **YES** (feature-gated via `copc-parallel`)
- **SIMD**: ✅ Multiple optimizations present

#### SIMD Optimizations:
1. **Bounding box accumulation** (Line 1280–1293)
   - **Lines 1280+**: `bounding_box()` function uses f64x4 min/max on [x, y, z]
   - Efficient bbox computation for node clustering

2. **Coordinate rounding** (via codec, implicit)
   - Same `f64x4` rounding as LAZ

3. **Frontend bbox** (Line 462–489)
   - **File**: [src/frontend.rs](src/frontend.rs), **Lines 462+**: `default_copc_config()` uses SIMD bbox accumulation
   - Computes spatial extent for octree initialization

#### Parallelization (Rayon-based):

**1. Parallel Node Encoding** (Lines 543–577)
```rust
// Feature-gated on copc-parallel
if sorted_keys.len() >= parallel_node_encode_min_nodes() 
   && total_points >= parallel_node_encode_min_points() {
    sorted_keys.par_iter()  // Rayon parallel iterator
        .map(|&key| encode_node_chunk(key, &nodes[&key], ...))
        .collect::<Result<Vec<_>>>()
}
```

**Feature Flags & Tuning** (Lines 28–71):
- `DEFAULT_PARALLEL_NODE_ENCODE_MIN_NODES`: 16 nodes threshold
- `DEFAULT_PARALLEL_SORT_MIN_POINTS`: 80,000 points threshold  
- `DEFAULT_PARALLEL_NODE_ENCODE_MIN_POINTS`: 400,000 total points threshold
- **Environment variables** for tuning:
  - `WBLIDAR_COPC_PARALLEL_MIN_POINTS` (node encoding threshold)
  - `WBLIDAR_COPC_PARALLEL_SORT_MIN_POINTS` (sorting threshold)

**Lines 43–71**: Threshold functions with env var overrides

**2. Parallel Sort by Morton Code** (Lines 664–700)
```rust
#[cfg(feature = "copc-parallel")]
{
    if points.len() >= parallel_sort_min_points() {
        codes.par_sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    } else {
        codes.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    }
}
```

- **Lines 684**: `par_sort_by()` used for Morton code sorting
- Improves spatial locality before compression

**3. Parallel Sort by Hilbert Code** (Lines 715+)
```rust
codes.par_sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
```

- Same pattern as Morton sorting
- Hilbert curve provides better spatial coherence than Morton

#### Hotspots:

1. **Node Point Ordering** (HIGH impact)
   - **Lines 665–697**: Sort calls; parallelized when point count ≥ 80k
   - Sorting large arrays benefits significantly from parallelism

2. **Node Chunk Encoding** (HIGH impact)
   - **Lines 543–577**: Parallel `.par_iter()` over sorted node keys
   - Each node's compression is independent; natural parallelization opportunity
   - Threshold: ≥16 nodes or ≥400k total points

3. **Voxel hierarchy construction** (MEDIUM impact)
   - Spatial binning of points into octree nodes; inherently sequential due to shared voxel map

#### Opportunities (Beyond Current Parallelization):

- **Parallel chunk decompression on read** (Medium complexity, HIGH impact)
  - Reader is still sequential; could apply same parallel chunk pattern as writer
  - Not yet implemented

---

## 4. E57 Format

### Reader Implementation
**File**: [src/e57/reader.rs](src/e57/reader.rs)

#### Current State:
- **Parallelization**: ❌ None
- **SIMD**: ❌ None
- **Streaming Model**: Full pre-load into memory

#### Design:
- Loads entire binary point data section into memory (lines 68–79)
- Page-by-page reading with CRC validation (no parallelization of CRC checks)
- Sequential point parsing from `raw_data` buffer

#### Hotspots:
1. **Page loading & CRC validation** (MEDIUM impact)
   - **Lines 74–79**: Loop over pages reading sequentially
   - CRC checks are performed inline; could be parallelized
   - **Bottleneck**: I/O + CRC compute serialized

2. **Point field unpacking** (MEDIUM-HIGH impact)
   - **Lines 108+**: Per-point field decoding (x, y, z, intensity, color, etc.)
   - Sequential per-point reads from fixed-size records

#### Opportunities:

- **Parallel page loading & CRC** (Medium complexity, MEDIUM impact)
  - Pages are independent; CRC can be computed in parallel
  - Requires buffering multiple pages in memory concurrently
  - **Estimate**: 2–4× speedup on 8-core systems

- **Batch field unpacking with SIMD** (High complexity, MEDIUM impact)
  - E57 permits arbitrary field subsets; no fixed record layout
  - Could batch little-endian int unpacking for coordinate arrays
  - Would require significant refactoring; limited by field variability

---

### Writer Implementation
**File**: [src/e57/writer.rs](src/e57/writer.rs)

#### Current State:
- **Parallelization**: ❌ None
- **SIMD**: ❌ None
- **Design**: Full point buffering; sequential write

#### Design:
- All points buffered in `self.points` (line 56)
- On `finish()`: writes header, then pages with CRC, then XML metadata
- No opportunities for parallelization due to sequential page CRC generation

#### Bottlenecks:
1. **Point buffering** – Full dataset in memory; large files problematic
2. **Sequential page writes** – CRC computation per page; no inter-page parallelism

#### Opportunities:

- **Streaming chunk writes** (High complexity, MEDIUM impact)
  - Refactor to accumulate points in chunks (e.g., 100k points)
  - Compute CRC + write each chunk concurrently
  - Would break current buffering model

---

## 5. PLY Format

### Reader Implementation
**File**: [src/ply/reader.rs](src/ply/reader.rs)

#### Current State:
- **Parallelization**: ❌ None
- **SIMD**: ❌ None
- **Streaming**: Per-point sequential reading

#### Design:
- Parses flexible ASCII/binary header (lines 89–133)
- Per-point reads based on property descriptors
- Three encoding paths: ASCII, binary LE, binary BE

#### Hotspots:
1. **ASCII parsing** (HIGH impact for ASCII files)
   - **Lines ~150–180**: String tokenization, parsing to f64/u16/u8 per field
   - Text parsing is inherently sequential; splitting/parsing each line

2. **Binary fixed-record reading** (MEDIUM impact)
   - Direct byte reads; faster than ASCII but still sequential

#### Opportunities:

- **Batch ASCII parsing** (High complexity, MEDIUM impact)
  - Buffer N lines; parse in parallel
  - Requires custom line-splitting + SIMD parsing (e.g., `regex` or custom SIMD tokenizer)
  - **Estimate**: 2–6× speedup on ASCII files (encoding-dependent)

- **SIMD field unpacking** (Medium complexity, MEDIUM impact)
  - Pack multiple fixed-record bytes into SIMD registers
  - Useful for coordinate arrays or packed integer data
  - Limited application due to format variability

---

### Writer Implementation
**File**: [src/ply/writer.rs](src/ply/writer.rs)

#### Current State:
- **Parallelization**: ❌ None
- **SIMD**: ❌ None
- **Streaming**: Per-point sequential writes

#### Design:
- Header written first with known point count
- Per-point encoding dispatched by encoding format
- Three encoding paths: ASCII, binary LE, binary BE

#### Hotspots:
1. **ASCII formatting** (HIGH impact)
   - **Lines ~170+**: `write!()` macro calls for each field per point
   - Text formatting is sequential; not easily vectorized

2. **Binary packing** (MEDIUM impact)
   - Byte-by-byte little/big-endian writes

#### Opportunities:

- **Batch ASCII formatting** (High complexity, MEDIUM impact)
  - Buffer N points; format N lines in parallel
  - Requires intermediate buffering
  - **Estimate**: 2–4× speedup

- **Batch binary encoding with SIMD** (Medium complexity, MEDIUM impact)
  - Pack coordinates + intensity into vectorized format
  - Could write SIMD-packed chunks of coordinates in one operation

---

## Gap Analysis & Priority Matrix

### High-Impact, Low-Complexity Optimizations:

| Format | Optimization | Complexity | Impact | ROI | Feasibility |
|--------|--------------|------------|--------|-----|-------------|
| LAZ | Parallel chunk decomp | 🟡 Medium | 🔴 HIGH | **Excellent** | 🟢 High |
| LAZ | Parallel chunk encoding | 🟡 Medium | 🔴 HIGH | **Excellent** | 🟢 High |
| E57 | Parallel page I/O + CRC | 🟡 Medium | 🟡 MEDIUM | **Good** | 🟡 Medium |
| PLY (ASCII) | Batch line parsing | 🔴 High | 🔴 HIGH | **Good** | 🟡 Medium |
| LAS | Batch bbox (API change) | 🔴 High | 🟢 LOW | Poor | 🔴 Low |
| E57 | Streaming write refactor | 🔴 High | 🟡 MEDIUM | Poor | 🔴 Low |

### Recommendation Ranking:

1. **LAZ Reader: Parallel Chunk Decompression** (Phase 1)
   - Add rayon-controlled multi-chunk decompression
   - Breaks streaming model minimally (buffer entire chunk table)
   - Estimated speedup: 4–8× on 8-core systems
   - Lines of code: ~150–200

2. **LAZ Writer: Parallel Chunk Encoding** (Phase 1)
   - Parallel encoding of buffered chunks before flush
   - Trivial integration with existing buffer model
   - Estimated speedup: 4–8× on 8-core systems
   - Lines of code: ~80–120

3. **E57 Reader: Parallel Page CRC** (Phase 2)
   - Parallelize CRC computation during page loading
   - Estimated speedup: 2–3× on 8-core systems
   - Lines of code: ~100–150

4. **PLY Reader (ASCII): Batch Parsing** (Phase 2, lower priority)
   - Custom SIMD-friendly line splitter + parallel formatting
   - High complexity; require new dependency or custom SIMD code
   - Estimated speedup: 2–6× (highly encoding-dependent)
   - Lines of code: ~300–500

---

## Feature Flag & Configuration Status

### Current Feature Flags:
```toml
[features]
default = []
copc-http = ["dep:reqwest"]
copc-parallel = ["dep:rayon"]  # Enables Rayon in COPC writer only
```

### Environment Variables (COPC tuning only):
- `WBLIDAR_COPC_PARALLEL_MIN_POINTS` (default: 400,000)
- `WBLIDAR_COPC_PARALLEL_SORT_MIN_POINTS` (default: 80,000)

### Recommendation:
Add a new feature flag `laz-parallel` to gate LAZ reader/writer parallelization:
```toml
[features]
laz-parallel = ["dep:rayon"]
e57-parallel = ["dep:rayon"]
```

---

## Summary Table: Optimization Locations

| Format | Component | Location | Line(s) | Type | Current Status |
|--------|-----------|----------|---------|------|-----------------|
| **LAS** | Read xyz | reader.rs | 122–131 | SIMD | ✅ Implemented |
| **LAS** | Write bbox | writer.rs | 279–289 | SIMD | ✅ Implemented |
| **LAZ** | Codec rounding | codec.rs | 27 | SIMD | ✅ Implemented |
| **LAZ** | Chunk decode | standard_point10.rs | 749+ | Serial | ❌ No parallelization |
| **LAZ** | Chunk encode | standard_point10_write.rs | 774+ | Serial | ❌ No parallelization |
| **COPC** | Node encode | writer.rs | 543–577 | Rayon | ✅ Implemented (gated) |
| **COPC** | Sort Morton | writer.rs | 684 | Rayon | ✅ Implemented (gated) |
| **COPC** | Sort Hilbert | writer.rs | 715+ | Rayon | ✅ Implemented (gated) |
| **COPC** | Bbox | writer.rs | 1280–1293 | SIMD | ✅ Implemented |
| **E57** | Read pages | reader.rs | 74–79 | Serial | ❌ No parallelization |
| **E57** | Write CRC | writer.rs | (scattered) | Serial | ❌ No parallelization |
| **PLY** | Parse ASCII | reader.rs | ~150–180 | Serial | ❌ No parallelization |
| **PLY** | Format ASCII | writer.rs | ~170+ | Serial | ❌ No parallelization |

---

## Files & Line References (Quick Index)

### LAS:
- Reader: [src/las/reader.rs](src/las/reader.rs) (SIMD: L122–131)
- Writer: [src/las/writer.rs](src/las/writer.rs) (SIMD: L279–289)

### LAZ:
- Reader: [src/laz/reader.rs](src/laz/reader.rs)
- Writer: [src/laz/writer.rs](src/laz/writer.rs) (L9: wide import)
- Codec: [src/laz/codec.rs](src/laz/codec.rs) (L27: SIMD rounding)
- Point10 Decode: [src/laz/standard_point10.rs](src/laz/standard_point10.rs) (L749+)
- Point10 Encode: [src/laz/standard_point10_write.rs](src/laz/standard_point10_write.rs) (L774+)
- Point14 Decode: [src/laz/standard_point14.rs](src/laz/standard_point14.rs)
- Point14 Encode: [src/laz/standard_point14.rs](src/laz/standard_point14.rs) (encode functions)
- Chunk ops: [src/laz/chunk.rs](src/laz/chunk.rs) (compress/decompress utilities)

### COPC:
- Writer: [src/copc/writer.rs](src/copc/writer.rs)
  - Node encoding (Rayon): L543–577
  - Sort Morton (Rayon): L684
  - Sort Hilbert (Rayon): L715+
  - Bbox SIMD: L1280–1293
  - Feature flags/tuning: L28–71
- Reader: [src/copc/reader.rs](src/copc/reader.rs) (no parallelization)

### E57:
- Reader: [src/e57/reader.rs](src/e57/reader.rs) (page loading: L74–79)
- Writer: [src/e57/writer.rs](src/e57/writer.rs) (buffered model)

### PLY:
- Reader: [src/ply/reader.rs](src/ply/reader.rs) (ASCII parsing: L~150–180)
- Writer: [src/ply/writer.rs](src/ply/writer.rs) (formatting: L~170+)

### Frontend:
- Bbox init: [src/frontend.rs](src/frontend.rs) (L462+: SIMD bbox for COPC config)

### Cargo.toml:
- Feature gates: [Cargo.toml](Cargo.toml) (L12–13)

---

## Conclusion

**Current State**:
- COPC is the most optimized format with comprehensive parallelization (writer) over sorted nodes
- LAS/LAZ have targeted SIMD for coordinate handling but NO parallelization
- E57/PLY are purely sequential with no SIMD or parallelism

**Quick Wins**:
1. LAZ reader/writer parallel chunk decompression/encoding (highest ROI, medium effort)
2. E57 parallel page CRC (medium ROI, medium effort)
3. PLY ASCII batch parsing (high potential but higher complexity)

**Architecture Notes**:
- Streaming APIs (LAS/LAZ/E57/PLY readers) inherit sequential behavior; batching would require API redesign or buffering adapters
- COPC's hierarchical structure naturally supports parallelization (already exploited)
- `wide` SIMD library provides portable vectorization; expansion opportunities limited by format constraints

