# wbraster Complex DataType Day 5 Design Stub

Date: 2026-05-31
Status: Draft (Day 5 prerequisite planning)
Scope: Additive, non-breaking complex-type enablement

## Goal

Introduce complex-valued raster support in wbraster without breaking existing
scalar API contracts.

## Non-Breaking Rules

- Keep existing scalar methods and signatures unchanged.
- Add complex support using additive enum/storage/method extensions.
- Preserve behavior of existing real-valued formats and tools.
- Defer any behavior-changing optimizations until after parity tests exist.

## Proposed Additions

### 1) DataType expansion

Add:
- DataType::ComplexF32
- DataType::ComplexF64

Consequence:
- Exhaustive matches on DataType across the crate must be updated.

### 2) RasterData expansion

Add storage variants:
- RasterData::ComplexF32(Vec<(f32, f32)>)
- RasterData::ComplexF64(Vec<(f64, f64)>)

Notes:
- Tuple storage keeps dependency surface minimal for initial implementation.
- A later pass can evaluate a dedicated complex-number type if needed.

### 3) Additive complex accessors

Add new methods (examples):
- get_complex(band, row, col) -> Option<(f64, f64)>
- set_complex(band, row, col, re, im) -> Result<()>
- band_to_vec_complex_f64(band) -> Vec<(f64, f64)>

Do not change:
- get(...)->f64
- set(..., value:f64)
- statistics() semantics for scalar rasters

### 4) Nodata/fill approach for complex

Initial policy proposal:
- Keep scalar nodata as-is for scalar rasters.
- For complex rasters, treat nodata as explicit validity state in ingestion layer
  (preferred), or fallback to component-wise sentinels where necessary.
- Do not reinterpret scalar nodata logic for complex values in this phase.

## Rollout Sequencing

1. Add enum/storage variants and compile fixes only.
2. Add complex accessors and constructor paths.
3. Keep scalar methods unchanged and validated by compile-only and behavior tests.
4. Add format readers/writers for complex payloads in follow-up milestones.

## Risks

- Large number of match sites in format modules can cause accidental behavior drift.
- Format-specific type maps may need explicit rejection paths for unsupported complex layouts.
- Statistics and visualization assumptions are scalar-centric and must remain isolated.

## Acceptance Criteria for Day 5 Stub

- Design note committed.
- DataType match-site audit committed with classification.
- Compile-only scalar API placeholder tests added.
- No change to existing scalar public method signatures.
