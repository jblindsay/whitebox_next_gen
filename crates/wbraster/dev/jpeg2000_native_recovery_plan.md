# JPEG2000 Native Recovery Plan (Post-Bridge)

Status: external pure-Rust bridge is currently enabled by default via `jpeg2000-external-bridge`, but optional.
Goal: restore full native decode reliability in `wbraster` so the bridge can be removed.

## Why this plan exists

The previous native decoder path in `src/formats/jpeg2000_core` produced severe lattice/checkerboard corruption on real Sentinel JP2 scenes, while the bridge decoder produced plausible outputs for the same files.

## Immediate architecture constraints

- Keep everything pure Rust.
- Minimize long-term external coupling in foundational crates.
- Use the bridge as a temporary correctness oracle, not permanent core architecture.

## High-probability native bug classes

1. Packet and code-block extraction is oversimplified.
- Native code currently treats tile payload extraction as contiguous and may split component data by equal chunking in some paths.
- Real JP2 codestreams use packet headers, progression order logic, and variable code-block lengths.

2. Bitstream reader and marker-stuffing behavior.
- JPEG2000 entropy streams require exact bit unstuffing around `0xFF` boundaries.
- Any off-by-one here can cause periodic block artifacts and saturated values.

3. Tier-1 context/coding pass fidelity.
- Native entropy implementation is intentionally simplified.
- External files may rely on details not represented by the simplified pass/context handling.

4. Tile-part / progression handling.
- Real-world products frequently use packet progression and tile-part structure beyond single simplistic assumptions.

## Port-and-compare strategy

Treat the bridge decoder as a reference implementation and migrate correctness-critical logic in order:

1. Add native instrumentation hooks.
- Emit debug counters for parsed markers, packet counts, code-block payload sizes, and decoded coefficient ranges.

2. Build deterministic corpus tests.
- Add fixture-based tests for representative Sentinel JP2 samples.
- Assert non-trivial value distribution and row-pattern sanity (guard against repeating checkerboard signatures).

3. Replace packet parsing path first.
- Port packet/header parsing concepts from bridge crate into `jpeg2000_core` while preserving current public API.
- Avoid equal-size component slicing assumptions entirely.

4. Replace entropy bitstream reader semantics second.
- Align marker-stuffing and bit-reader behavior with proven decoder logic.

5. Validate tier-1 decode contexts against corpus.
- If needed, port/align context modeling and coding pass sequencing incrementally.

6. Remove bridge default.
- Flip default feature set to native-only once corpus passes.
- Keep bridge feature only for transitional comparison for one release cycle.

7. Remove bridge dependency.
- Delete feature and dependency after native parity is sustained.

## Build modes currently available

- Default (bridge enabled):
  - `cargo check -p wbraster`
- Native-only validation (bridge disabled):
  - `cargo check -p wbraster`

## License note

The bridge crate is dual-licensed MIT OR Apache-2.0, which is compatible with `wbraster` licensing for learning/porting patterns. Preserve attribution where appropriate if directly reusing substantial code blocks.
