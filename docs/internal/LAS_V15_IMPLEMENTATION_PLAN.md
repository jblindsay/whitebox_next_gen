# LAS v1.5 Support Implementation Plan

**Status**: Planning Phase  
**Date**: March 30, 2026  
**Target Release**: Q3 2026  
**Estimated Effort**: ~60 developer-hours

---

## Table of Contents

1. [LAS v1.5 Specification Overview](#las-v15-specification-overview)
2. [Current wblidar Status](#current-wblidar-status)
3. [Implementation Phases](#implementation-phases)
4. [Risk Assessment](#risk-assessment)
5. [Testing Strategy](#testing-strategy)
6. [Backward Compatibility](#backward-compatibility)

---

## LAS v1.5 Specification Overview

### New Point Data Record Formats (PDRFs)

LAS v1.5 introduces **PDRFs 11–15**, extending beyond v1.4's PDRF 0–10:

| PDRF | Version | Core Size | Attributes |
|------|---------|-----------|------------|
| 0–5  | 1.0–1.3 | 20–26 B   | Legacy formats |
| 6–10 | 1.4     | 26–38 B   | Extended returns, RGB, NIR, waveform |
| 11   | 1.5     | 30 B      | PDRF 6 + legacy RGB byte |
| 12   | 1.5     | 32 B      | PDRF 6 + extended 16-bit RGB |
| 13   | 1.5     | 40 B      | PDRF 6 + 16-bit RGB + 16-bit NIR |
| 14   | 1.5     | 36 B      | PDRF 6 + 16-bit RGB + waveform |
| 15   | 1.5     | 44 B      | PDRF 6 + 16-bit RGB + 16-bit NIR + waveform |

#### Key Changes

- **Extended Returns**: Return numbers and number-of-returns expand from 4-bit (0–15) to 8-bit (0–255)
- **16-bit RGB**: RGB values move from embedded to explicit 16-bit channels (extended color depth)
- **NIR Support**: Wider adoption of near-infrared channel across more PDRFs
- **Global Encoding Flags**: New bits (11, 12) for extended return/classification encoding
- **Header Version**: `version_minor = 5` (v1.4 was `4`); core 375-byte structure unchanged

### Backward Compatibility

- ✅ PDRFs 0–10 remain binary-identical to v1.4
- ⚠️ LAS 1.4 readers will **reject** files with PDRFs 11–15
- ✅ LAS 1.5 readers should seamlessly handle v1.0–1.4 files

---

## Current wblidar Status

### Supported Versions

Currently: **LAS 1.0–1.4** only

### Current PDRF Support

- **PDRFs 0–10**: Fully implemented
- **PDRFs 11–15**: Not recognized; parsing will fail

### Key Files Requiring Updates

```
crates/wblidar/src/
├── las/
│   ├── header.rs          (version check: hardcoded <= 4)
│   ├── reader.rs          (PDRF 0-10 decoders only)
│   ├── writer.rs          (writes v1.4 always; PDRF 0-10 only)
│   └── point.rs           (PointDataFormat enum: 0-10 only)
├── laz/
│   ├── standard_point10.rs (PDRFs 0-5 codec)
│   ├── standard_point14.rs (PDRFs 6-10 codec)
│   └── reader.rs/writer.rs (dispatch tables tied to PDRF enum)
└── copc/
    ├── writer.rs          (hierarchy assumes fixed point sizes)
    └── reader.rs
```

---

## Implementation Phases

### Phase 1: Foundation (~14 hours)

**Goal**: Enable v1.5 file recognition and PDRF dispatch

#### 1.1 Update Header Version Check

**File**: `crates/wblidar/src/las/header.rs`

- Extend version_minor check from `<= 4` to `<= 5`
- Add comments explaining LAS v1.5 support
- No breaking changes to existing v1.0–1.4 logic

#### 1.2 Extend PointDataFormat Enum

**File**: `crates/wblidar/src/las/point.rs`

```rust
pub enum PointDataFormat {
    Pdrf0 = 0, ..., Pdrf10 = 10,
    // NEW:
    Pdrf11 = 11,  // 30 bytes: PDRF 6 + legacy RGB
    Pdrf12 = 12,  // 32 bytes: PDRF 6 + 16-bit RGB
    Pdrf13 = 13,  // 40 bytes: PDRF 6 + 16-bit RGB + 16-bit NIR
    Pdrf14 = 14,  // 36 bytes: PDRF 6 + 16-bit RGB + waveform
    Pdrf15 = 15,  // 44 bytes: PDRF 6 + 16-bit RGB + 16-bit NIR + waveform
}
```

#### 1.3 Add Size Table

```rust
pub fn core_size(&self) -> u16 {
    match self {
        // ... existing 0-10 ...
        Pdrf11 => 30,
        Pdrf12 => 32,
        Pdrf13 => 40,
        Pdrf14 => 36,
        Pdrf15 => 44,
    }
}
```

#### 1.4 Add Helper Methods

```rust
pub fn is_v15(&self) -> bool {
    matches!(self, Self::Pdrf11 | Self::Pdrf12 | Self::Pdrf13 | Self::Pdrf14 | Self::Pdrf15)
}

pub fn has_extended_rgb(&self) -> bool {
    matches!(self, Self::Pdrf12 | Self::Pdrf13 | Self::Pdrf14 | Self::Pdrf15)
}

pub fn has_nir(&self) -> bool {
    matches!(self, Self::Pdrf13 | Self::Pdrf15)
}
```

#### 1.5 Update Helpers

Update existing methods like `has_rgb()`, `has_gps_time()`, `has_waveform()` to include v1.5 PDRFs where applicable.

**Outcome**: wblidar recognizes v1.5 files but cannot read them yet.

---

### Phase 2: LAS Reader/Writer (~14 hours)

**Goal**: Support uncompressed read/write of LAS 1.5

#### 2.1 LAS Reader: Add PDRF 11–15 Decoders

**File**: `crates/wblidar/src/las/reader.rs`

Add 5 new functions: `decode_pdrf11()`, `decode_pdrf12()`, ..., `decode_pdrf15()`

Key differences from v1.4:
- **Return numbers**: Full 8-bit value (not masked to 4-bit)
- **Number of returns**: Full 8-bit value (not masked to 4-bit)
- **RGB**: Explicit 16-bit channels (vs. embedded in flags for v1.4)
- **NIR**: Present in PDRFs 13 & 15 as 16-bit value
- **Extended classification**: May read 8-bit classification if global encoding flag set

#### 2.2 LAS Writer: Add PDRF 11–15 Encoders

**File**: `crates/wblidar/src/las/writer.rs`

Add 5 new functions: `encode_pdrf11()`, ..., `encode_pdrf15()`

Additional logic:
- Accept version parameter in writer config; output `version_minor = 5` for v1.5 mode
- Set global encoding flags when using extended returns (bit 11) or extended classification (bit 12)
- Map PointRecord fields to v1.5 PDRF structure

#### 2.3 Default Writer Behavior

- Default output remains **LAS 1.4** (backward compatible)
- User must explicitly opt-in to v1.5 output

**Outcome**: Can read/write uncompressed LAS 1.5 files; LAZ still unsupported.

---

### Phase 3: LAZ Codec (~16 hours)

**Goal**: Support compressed LAS 1.5 via LASzip v1.5

#### 3.1 Create standard_point15.rs

**File**: `crates/wblidar/src/laz/standard_point15.rs` (new)

Implement LASzip v1.5 arithmetic layer codec for PDRFs 11–15:
- Adapt predictor context models from `standard_point14.rs`
- Build new arithmetic symbol models for 8-bit returns (vs. 4-bit)
- Add RGB predictor contexts for 16-bit channels
- Add optional NIR predictor contexts (PDRFs 13, 15)

#### 3.2 Update LAZ Reader

**File**: `crates/wblidar/src/laz/reader.rs`

Route PDRF 11–15 chunks to Point15 decoder (existing PDRF 6–10 → Point14).

#### 3.3 Update LAZ Writer

**File**: `crates/wblidar/src/laz/writer.rs`

- Generate correct LASzip VLR metadata for PDRFs 11–15
- Route encode calls to Point15 layer for v1.5 PDRFs

#### ⚠️ Blocker: LASzip Specification

**Current Status**: LASzip v1.5 codec specification not yet finalized by ASPRS (as of March 2026).

**Action Items**:
1. Contact ASPRS Encoding Working Group for codec details
2. Alternatively, reverse-engineer from PDAL's `io_las/io/` v1.5 codec (if available)
3. Coordinate with LASzip reference implementation maintainers

**Outcome**: LAZ files can compress/decompress LAS 1.5 data (pending spec availability).

---

### Phase 4: COPC Integration (~8 hours)

**Goal**: Support v1.5 PDRFs in Cloud-Optimized Point Cloud (COPC)

#### 4.1 Update COPC Writer

**File**: `crates/wblidar/src/copc/writer.rs`

- Ensure hierarchy node serialization handles variable-size point records (PDRFs 11–15)
- Update point encoding dispatch to use v1.5 PDRFs when appropriate

#### 4.2 Update COPC Reader

**File**: `crates/wblidar/src/copc/reader.rs`

- Verify hierarchy parsing handles v1.5 headers
- Test point decoder dispatch against all PDRFs (0–15)

#### 4.3 Testing

Round-trip: v1.5 LAS → COPC → LAZ, verify bit-for-bit equality

**Outcome**: COPC fully supports v1.5 PDRFs and spatial indexing.

---

## Risk Assessment

### High-Risk Areas

| Component | Risk | Mitigation |
|-----------|------|-----------|
| **LASzip v1.5 Spec** | **CRITICAL** | Contact ASPRS; may need reverse-engineering from reference implementations |
| **Point15 Codec** | **HIGH** | Arithmetic context models for 8-bit returns untested; requires careful validation |
| **RGB Predictor** | **MEDIUM** | 16-bit RGB channels require new predictor contexts; needs detailed testing |
| **Backward Compat** | **LOW** | Old decoders isolated; comprehensive test suite validates no regressions |

### Medium-Risk Areas

- **Extended Classification**: Global encoding flag bit 12 handling
- **Waveform Data**: PDRFs 14–15 with waveform support
- **Test Fixtures**: Lack of public v1.5 test files; in-memory fixtures required

---

## Testing Strategy

### Unit Tests (Backward Compatible)

- ✅ **Existing v1.0–1.4 test suite**: Run unchanged; must pass 100%
- ✅ **New v1.5 tests**: Isolated to `#[cfg(test)] mod v15_tests { ... }`
- ✅ **PDRF dispatch correctness**: Property tests covering all 16 formats (0–15)

### Integration Tests

- Round-trip: LAS 1.5 (in-memory) → read → compare
- Synthetic fixtures: Generate LAS 1.5 files with known point data
- LAZ round-trip: LAS 1.5 → compress → decompress → compare
- COPC round-trip: LAS 1.5 → COPC octree → point read → compare

### Real-World Fixtures

- Pending availability from ASPRS, PDAL, QGIS, or major LiDAR vendors
- Coordinate with community for v1.5 test data

---

## Backward Compatibility

### Guaranteed Preservation

| Component | Change | Impact |
|-----------|--------|--------|
| **PDRFs 0–10 decoders** | ✅ **No change** | Existing code paths untouched |
| **PDRFs 0–10 encoders** | ✅ **No change** | Existing code paths untouched |
| **v1.0–1.4 readers** | ✅ **Enhanced** | Recognize v1.5 headers but fail on PDRFs 11–15 until Phase 2 |
| **Default writer** | ✅ **v1.4 by default** | Users must opt-in to v1.5 |
| **Existing tests** | ✅ **All pass** | No modifications to legacy test fixtures |

### Validate

Each phase includes a regression test:
```bash
cargo test -p wblidar --lib 2>&1 | grep "test result: ok"
```
Must show **all 159+ tests passing** after each phase.

---

## Effort Breakdown

| Phase | Component | Hours | Blocking | Risk |
|-------|-----------|-------|----------|------|
| 1 | Header/PDRF enum | 14 | No | LOW |
| 2 | LAS reader/writer | 14 | Yes | MEDIUM |
| 3 | LAZ Point15 codec | 16 | **Yes** | **HIGH** |
| 4 | COPC integration | 8 | No | MEDIUM |
| — | Testing & fixtures | 8 | — | MEDIUM |
| — | **Total** | **60** | — | — |

### Effort Scenarios

| Scenario | Scope | Effort | Deliverable |
|----------|-------|--------|-------------|
| **A** | Uncompressed v1.5 only | 14+14 = **28 hrs** | Read/write LAS 1.5 (no LAZ) |
| **B** | Full v1.5 stack | ~60 hrs | LAS/LAZ/COPC 1.5 |
| **C** | Full + reverse-engineering | 80+ hrs | Include LASzip spec work |

---

## Implementation Order

### Recommended Sequence

1. **Phase 1** (Foundation) → Enables file recognition
2. **Phase 2** (LAS I/O) → Unblocks uncompressed workflows
3. **Phase 3** (LAZ) → Requires LASzip spec; can run in parallel with Phase 2 if spec obtained early
4. **Phase 4** (COPC) → Depends on Phase 3 completion

### Critical Path

```
Phase 1 (14h) → Phase 2 (14h) ⟶ Phase 4 (8h)
                      ↘
                    Phase 3 (16h) — awaiting LASzip spec
```

**Recommendation**: Complete Phases 1–2 immediately. Start Phase 3 research in parallel; implement once ASPRS provides codec specification.

---

## Dependencies

### External Dependencies

- **LASzip Specification v1.5**: Not yet public; coordinate with ASPRS
- **Test Fixtures**: Pending community contribution or PDAL/QGIS export
- **Reference Implementation**: Check PDAL, liblas, or QGIS for v1.5 codec hints

### Internal Dependencies

```
Header (Phase 1)
  ↓
PointDataFormat Enum (Phase 1)
  ↓
LAS Reader/Writer (Phase 2)
  ↓ (input)
LAZ Point15 Codec (Phase 3)
  ↓ (input)
COPC Integration (Phase 4)
```

---

## Success Criteria

### Phase 1 Completion

- ✅ `cargo test -p wblidar --lib` passes (159+ tests)
- ✅ Version parser accepts `version_minor == 5`
- ✅ PDRF enum recognizes 11–15
- ✅ Helper methods (`is_v15()`, etc.) correct

### Phase 2 Completion

- ✅ Uncompressed LAS 1.5 reads correctly
- ✅ Uncompressed LAS 1.5 writes correctly
- ✅ Round-trip: generate LAS 1.5 → read → compare (bit-exact)
- ✅ No regression: v1.0–1.4 tests pass

### Phase 3 Completion

- ✅ LAZ 1.5 files compress/decompress correctly
- ✅ Round-trip: LAS 1.5 + LAZ → COPC → decompress (byte-exact)
- ✅ LASzip VLR metadata generation correct

### Phase 4 Completion

- ✅ COPC octree handles all PDRFs (0–15)
- ✅ Hierarchy serialization supports variable point sizes
- ✅ Full suite tests pass

---

## References

- **LAS 1.5 Standard**: ASPRS E89-2025 (available from ASPRS.org)
- **LASzip Specification**: Awaiting v1.5 publication
- **PDAL Implementation**: https://github.com/PDAL/PDAL (reference for v1.5 codec)
- **QGIS Support**: Pending beta releases

---

## Revision History

| Date | Author | Status | Notes |
|------|--------|--------|-------|
| 2026-03-30 | AI Assistant | Planning | Initial assessment and phase breakdown |

