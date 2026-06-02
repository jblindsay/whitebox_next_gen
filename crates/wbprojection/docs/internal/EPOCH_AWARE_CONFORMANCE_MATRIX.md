# Epoch-Aware Conformance Matrix

Date: 2026-06-02
Scope: Conformance corridors for epoch-aware and preferred-operation routing in wbprojection.

## Purpose

This document tracks corridor-level conformance targets used by the current prototype tests.
The present tests validate deterministic routing consistency between:
- preferred-operation routing,
- explicit operation-code routing, and
- baseline transform paths.

These checks combine internal routing consistency with date-routed authoritative NRCan fixtures
for practical CSRS readiness.

## Operational CSRS Status

Current usable scope in this repository:

- Active preferred CSRS corridors: v3 -> v8, v4 -> v8, v6 -> v8, and v7 -> v8
	(zone-matched UTM, zones 7-24), operation 10715.
- Evidence model: authoritative-inference is accepted when NRCan operational tools are date-oriented.

## Corridor Matrix

| Corridor | Source CRS | Target CRS | Preferred Operation | Tolerance | Checkpoints |
|---|---:|---:|---:|---:|---:|
| CSRS UTM Zone 17 (v3 -> v8) | 22317 | 22817 | 10715 | 0.001 m | 3 |
| CSRS UTM Zone 7 (v3 -> v8) | 22307 | 22807 | 10715 | 0.001 m | 3 |
| CSRS UTM Zone 12 (v3 -> v8) | 22312 | 22812 | 10715 | 0.001 m | 3 |
| CSRS UTM Zone 15 (v3 -> v8) | 22315 | 22815 | 10715 | 0.001 m | 3 |
| CSRS UTM Zone 20 (v3 -> v8) | 22320 | 22820 | 10715 | 0.001 m | 3 |
| CSRS UTM Zone 22 (v3 -> v8) | 22322 | 22822 | 10715 | 0.001 m | 3 |

## CSRS Realization-Pair Activation Matrix (Projected UTM Corridors)

Status legend:
- Active: preferred operation mapping is enabled in code.
- Pending: candidate corridor not yet activated (missing validated operation metadata/assets/checkpoints).
- N/A: projected realization family not currently represented in the compiled CSRS realization UTM corridor set.

Tracked projected realization families in current registry surface:
- v2: EPSG 22207-22222
- v3: EPSG 22307-22324
- v4: EPSG 22407-22424
- v6: EPSG 22607-22624
- v7: EPSG 22707-22724
- v8: EPSG 22807-22824

Projected-v5 note:
- A projected NAD83(CSRS)v5 UTM corridor family is not currently wired in the same pattern as v2/v3/v4/v6/v7/v8, so projected v5 pair activation is treated as N/A in this matrix for now.

Pending zone-corridor rule (coded):
- None in the currently scoped CSRS forward corridors (to v8) for zones 7-24.

| Source realization | Target realization | Status | Preferred operation |
|---|---|---|---:|
| v2 | v2 | Pending | - |
| v2 | v3 | Pending | - |
| v2 | v4 | Pending | - |
| v2 | v6 | Pending | - |
| v2 | v7 | Pending | - |
| v2 | v8 | Pending | - |
| v3 | v2 | Pending | - |
| v3 | v3 | Pending | - |
| v3 | v4 | Pending | - |
| v3 | v6 | Pending | - |
| v3 | v7 | Pending | - |
| v3 | v8 | Active | 10715 |
| v4 | v2 | Pending | - |
| v4 | v3 | Pending | - |
| v4 | v4 | Pending | - |
| v4 | v6 | Pending | - |
| v4 | v7 | Pending | - |
| v4 | v8 | Active | 10715 |
| v6 | v2 | Pending | - |
| v6 | v3 | Pending | - |
| v6 | v4 | Pending | - |
| v6 | v6 | Pending | - |
| v6 | v7 | Pending | - |
| v6 | v8 | Active | 10715 |
| v7 | v2 | Pending | - |
| v7 | v3 | Pending | - |
| v7 | v4 | Pending | - |
| v7 | v6 | Pending | - |
| v7 | v7 | Pending | - |
| v7 | v8 | Active | 10715 |
| v8 | v2 | Pending | - |
| v8 | v3 | Pending | - |
| v8 | v4 | Pending | - |
| v8 | v6 | Pending | - |
| v8 | v7 | Pending | - |
| v8 | v8 | Pending | - |

## Current Assertions

For each checkpoint in each corridor:
- Preferred-route output matches explicit operation-route output within tolerance.
- Preferred-route output matches baseline output within tolerance.

## Next Expansion Targets

1. Keep zone-matched coverage at 7-24 for all active forward-to-v8 corridors and add explicit checkpoints for every active zone.
2. Define reverse-direction corridor strategy once operation metadata for reverse preference is finalized.
3. Introduce authoritative external reference checkpoints by corridor with approved tolerance bands.
4. Split tolerance matrix by transform class where needed (horizontal only vs 3D).

## Related Tests

- src/tests/integration_tests.rs
- src/tests/epsg_tests.rs

## External Authoritative Sources

- docs/internal/AUTHORITATIVE_CHECKPOINT_SOURCES.md
- src/tests/data/authoritative/nrcan_trx_nad83csrs_to_itrf2014_epoch2010_checkpoints.csv
- src/tests/data/authoritative/nrcan_nad83csrs_epoch_propagation_2010_to_2020_checkpoints.csv
- src/tests/data/authoritative/nrcan_trx_nad83csrs_epoch_2002_to_2010_guelph_vancouver_sample2.csv

Notes:
- The NRCan TRX fixture is authoritative for that TRX workflow and complements preferred-operation corridor consistency checks.
- The NRCan epoch-propagation fixture is authoritative for authenticated web-tool epoch propagation within NAD83(CSRS) using interpolated velocities.
- The corridor matrix above remains an internal-consistency conformance layer for operation 10715 routing; a direct external operation-10715 checkpoint set was not obtainable from the explored NRCan web UI.

## Evidence Tiers Used For CSRS Progress

Tier A (Pair-Activation Grade):
- Explicit source/target realization metadata or explicit EPSG pair is present.
- Eligible to activate Pending realization-pair corridors.

Tier B (Date-Routed Authoritative Grade):
- Authoritative tool outputs are reproducible and run settings are preserved, but realization labels are not explicit in output.
- Eligible for epoch-routing validation and regression checks.
- Not eligible by itself to activate Pending realization-pair corridors.

Current status for the NRCan date-based captures in this repo:
- Classified as Tier B for general date-routed conformance.
- Used to validate epoch-aware behavior.
- Also used as authoritative-inference support for v4 -> v8 activation when combined with EPSG anchor-epoch metadata.
