# wbtopology Buffer Dissolve GEOS-Parity Plan (2026-05-07)

## Objective
Close the extreme performance gap for dense line-network buffering and dissolve workloads by removing structural work amplification in wbtopology.

Target context from latest user benchmark:
- QGIS/GEOS: about 1.48s
- wbtopology dissolve: killed after more than 10 minutes
- wbtopology non-dissolve: about 1.55s (regressed from prior less than 0.3s)

## Root-Cause Ranking (Most Likely First)
1. Repeated polygon pair overlay attempts during dissolve after polygonization.
2. High-frequency expensive pre-union geometric predicates per candidate pair.
3. Iterative restart merge behavior on one large connected component.
4. Polygonize then unary-union double-stage for line dissolve (duplicated topology work).
5. Missing GEOS-style line-input simplification before curve generation.
6. Non-dissolve still triggering expensive robustness paths too often.

## Ranked Implementation Plan

### Step 1 (Highest Priority)
Implement one-pass BuildArea-style line dissolve path and bypass pairwise polygon unary union for line dissolve.

Scope:
- Build global line buffer curves.
- Node once.
- Build topology graph once.
- Extract bounded faces once.
- Classify/select final faces topologically.
- Emit dissolved polygons directly (no pairwise polygon-union loop over polygonized faces).

Expected impact:
- 5x to 25x on dense roads datasets.

Risk:
- Medium (hole and touch topology handling).

Primary files:
- crates/wbtools_oss/src/tools/gis/mod.rs
- crates/wbtopology/src/constructive.rs
- crates/wbtopology/src/overlay.rs

### Step 2
Partition linework into connected components before heavy graph stages, and process components independently.

Expected impact:
- 2x to 8x on multi-cluster datasets.

Risk:
- Low to medium.

Primary files:
- crates/wbtopology/src/constructive.rs
- crates/wbtopology/src/graph.rs

### Step 3
Tighten candidate generation for fallback unions and delay expensive geometric predicates until after cheap envelope/index filtering.

Expected impact:
- 1.5x to 4x on large fallback union workloads.

Risk:
- Medium.

Primary files:
- crates/wbtopology/src/overlay.rs

### Step 4
Add GEOS-style line simplification control before line buffer curve generation.

Expected impact:
- 1.3x to 3x (input-dependent).

Risk:
- Medium (must guard parity and topology).

Primary files:
- crates/wbtopology/src/constructive.rs
- crates/wbtools_oss/src/tools/gis/mod.rs

### Step 5 (Non-Dissolve Recovery)
Restore strict non-dissolve fast path by aggressively gating expensive repair/fallback logic to clearly invalid outputs only.

Expected impact:
- 2x to 8x for non-dissolve line buffering.

Risk:
- Low to medium.

Primary files:
- crates/wbtools_oss/src/tools/gis/mod.rs
- crates/wbtopology/src/constructive.rs

## Execution Order
1. Step 1 and Step 5 together (largest user-visible impact first).
2. Step 2.
3. Step 3.
4. Step 4.

## Checkpoint Gates
After each step, verify with user benchmark script and record result:

Gate A (after Step 1+5):
- Dissolve runtime trend improves materially.
- Non-dissolve recovers toward historical baseline.

Gate B (after Step 2):
- Better scaling as network size and disconnected clusters increase.

Gate C (after Step 3):
- Fewer long-tail stalls in large connected dissolve components.

Gate D (after Step 4):
- Additional speedup without unacceptable geometry drift.

## Success Criteria
- Significant reduction from 10+ minute dissolve toward low-second or tens-of-seconds range as interim milestone.
- Non-dissolve restored close to prior behavior.
- No known regressions in existing hole/closed-loop correctness cases.

## Notes for Implementation
- Prefer structural reductions in topology work over micro-optimizations.
- Keep changelog updated for each completed step.
- Do not run compile/benchmark commands unless explicitly requested in that turn.
