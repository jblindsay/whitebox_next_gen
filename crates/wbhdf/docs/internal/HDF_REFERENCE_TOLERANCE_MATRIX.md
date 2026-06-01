# HDF Reference Tolerance Matrix

Date: 2026-06-01  
Status: Active

This document defines explicit reference-comparison tolerance contracts for currently
validated sample-product checkpoints.

## Scope

- The values below apply to fixture-backed comparisons against external reference
  extraction outputs (for example, h5dump).
- Tolerances are absolute unless explicitly stated otherwise.
- These tolerances are for deterministic decode validation and are not intended as
  generalized science-uncertainty models.

## Current Tolerance Contracts

| Product Family | Dataset Path | Data Type | Reference Source | Absolute Tolerance | Validation Test |
|---|---|---|---|---|---|
| GEDI02_A | /BEAM0000/elev_lowestmode | f32 | h5dump | 1e-5 | gedi_elev_lowestmode_contiguous_window_matches_h5dump_reference |
| VIIRS VNP13A4N | /HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/XDim | f64 | h5dump | 1e-8 | viirs_vnp13_xdim_contiguous_window_matches_h5dump_reference |
| VIIRS VNP21_NRT | /VIIRS_Swath_LSTE/Geolocation Fields/latitude | f32 | h5dump | 1e-4 | viirs_vnp21_latitude_row_major_window_matches_h5dump_reference |
| VIIRS VNP21_NRT | /VIIRS_Swath_LSTE/Geolocation Fields/longitude | f32 | h5dump | 1e-4 | viirs_vnp21_longitude_row_major_window_matches_h5dump_reference |

## Utility-Level Contract

Comparison utilities used by the tolerance-gated tests:

- compare_f32_with_tolerance(actual, expected, abs_tolerance)
- compare_f64_with_tolerance(actual, expected, abs_tolerance)

Both utilities enforce:

- equal array lengths,
- finite non-negative tolerance values,
- mismatch counts and maximum absolute difference reporting.

## Change Control

When adding a new reference-checked product path:

1. Record the dataset path, reference extraction command/source, and tolerance value here.
2. Add or update fixture-backed tests to enforce that tolerance explicitly.
3. Update roadmap evidence entries that depend on the default-enable tolerance gate.
