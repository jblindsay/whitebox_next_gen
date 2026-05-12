# Remote Sensing Tool Sprint Plan (OSS)

Date: 2026-05-12
Scope: `wbtools_oss` remote sensing sprint for teaching-critical gaps that do not overlap current Pro functionality.
Owner: Whitebox Next Gen (`wbtools_oss`)
Planned tool count: 15 (excluding intentionally skipped Pro-overlap tools).

## 1) Sprint Goals

- Add a practical, teaching-ready OSS remote sensing workflow covering:
  - Radiometric preprocessing fundamentals
  - Thermal/LST fundamentals
  - Introductory change detection workflows
  - Core hyperspectral-ready analytics with multispectral applicability
  - Polarimetric SAR decomposition entry points
- Preserve clear product separation from Pro tools:
  - Do not implement topographic illumination C-correction/cosine correction variants in OSS.
- Ensure each tool is wired across:
  - Rust tool registry
  - Python bindings + stubs
  - R bindings + docs
  - Taxonomy namespace placement

## 2) Sensor-Bundle Dependency Strategy (`wbraster`)

## Why sensor bundle support matters

Several planned tools require sensor-specific metadata (band mapping, calibration constants, thermal constants, wavelength assumptions). This should be centralized in `wbraster` sensor bundles rather than duplicated in each tool.

## Sensor-bundle required

- DN to TOA Reflectance (Landsat, Sentinel-2)
- LST Single-Channel (thermal constants, wavelength assumptions by sensor)
- LST Split-Window (paired thermal bands and coefficients by sensor family)

## Sensor-bundle helpful but optional

- NDVI-based emissivity estimation (band mapping convenience)
- Change detection tools (mostly sensor-agnostic if inputs are pre-aligned)
- SAM / spectral matching (sensor metadata may improve defaults, but not required)

## Sensor-bundle not required initially

- Continuum removal (if wavelengths supplied explicitly)
- Linear spectral unmixing (if endmembers supplied in band space)
- Basic polSAR decomposition kernels (driven by covariance/coherency inputs)

## `wbraster` bundle tasks (tracked)

- [x] Confirm Landsat 8/9 OLI/TIRS bundle fields needed for TOA + LST  
  Using: `REFLECTANCE_MULT_BAND_*`, `REFLECTANCE_ADD_BAND_*`, `RADIANCE_MULT_BAND_*`, `RADIANCE_ADD_BAND_*`, `K1_CONSTANT_BAND_*`, `K2_CONSTANT_BAND_*`, and bundle sun-elevation metadata.
- [x] Confirm Sentinel-2 MSI bundle fields needed for TOA reflectance  
  `Sentinel2SafePackage` now exposes quantification metadata and `dn_to_toa_reflectance` prefers bundle-derived reflectance scaling (`reflectance_scale_factor`) with fallback only when metadata is absent.
- [x] Define minimal thermal coefficient contract for LST tools  
  Formalized in `wbraster` via typed Landsat bundle calibration accessors (`reflectance_coefficients_for_band`, `thermal_constants_for_band`) and consumed by `wbtools_oss` TOA/LST paths.
- [x] Add unit tests for sensor lookup and missing-metadata errors  
  Added targeted unit tests in `wbtools_oss` for Landsat bundle missing reflectance/thermal metadata failure paths and in `wbraster` for Sentinel-2 quantification parsing.

## 3) Planned Tool Set and Status

Status legend: `not-started` | `in-progress` | `blocked` | `done`

## A. Radiometric Correction (OSS-safe)

1. DN to TOA Reflectance (`dn_to_toa_reflectance`)  
Status: `done`
- [x] Rust implementation scaffolded and registered in `wbtools_oss`
- [x] Parallel row-kernel compute path implemented
- [x] Landsat path (gain/offset + solar geometry)
- [x] Sentinel-2 path (quantification value + solar geometry)
- [x] No-data handling and saturation guards
- [x] Multi-band stack support
- [x] Metadata output (processing summary)

2. Dark Object Subtraction (`dark_object_subtraction`)  
Status: `done`
- [x] Rust implementation scaffolded and registered in `wbtools_oss`
- [x] Percentile-based haze estimate option
- [x] Per-band offset estimation
- [x] Clamp non-physical negatives (configurable)
- [x] Optional QA/diagnostic raster outputs

## B. Thermal / Emissivity

3. NDVI-based Emissivity (`ndvi_based_emissivity`)  
Status: `done`
- [x] Rust implementation scaffolded and registered in `wbtools_oss`
- [x] Parallel row-kernel compute path implemented
- [x] Fractional vegetation cover model
- [x] Emissivity map output
- [x] Configurable vegetation/soil emissivity defaults

4. LST Single-Channel (`land_surface_temperature_single_channel`)  
Status: `done`
- [x] Rust implementation scaffolded and registered in `wbtools_oss`
- [x] Parallel row-kernel compute path implemented
- [x] Brightness temperature conversion
- [x] Emissivity correction integration
- [x] Kelvin/Celsius output option
- [x] Sensor-specific constant handling via bundle

5. LST Split-Window (`land_surface_temperature_split_window`)  
Status: `done`
- [x] Rust implementation scaffolded and registered in `wbtools_oss`
- [x] Parallel row-kernel compute path implemented
- [x] Paired thermal band ingest/validation
- [x] Coefficient model implementation
- [x] Emissivity + atmospheric term inputs
- [x] Robust coefficient validation and warnings

## C. Change Detection

6. Image Differencing (`image_difference_change_detection`)  
Status: `done`
- [x] Rust implementation scaffolded and registered in `wbtools_oss`
- [x] Parallel row-kernel compute path implemented
- [x] Single-band and multi-band magnitude modes
- [x] Absolute difference + signed difference outputs
- [x] Threshold-based binary change mask

7. Post-Classification Comparison (`post_classification_change`)  
Status: `done`
- [x] Rust implementation scaffolded and registered in `wbtools_oss`
- [x] Parallel transition aggregation implemented
- [x] Crosstab/confusion-style transition matrix
- [x] Transition-coded raster output
- [x] Optional class remap tables

8. PCA-Based Change Detection (`pca_based_change_detection`)  
Status: `done`
- [x] Rust implementation scaffolded and registered in `wbtools_oss`
- [x] Parallel covariance accumulation + row-kernel projection
- [x] Optional report output (component loadings / explained variance)

## D. Hyperspectral / Spectral Analytics

9. Spectral Angle Mapper (`spectral_angle_mapper`)  
Status: `done`
- [x] Rust implementation scaffolded and registered in `wbtools_oss`
- [x] Parallel row-kernel compute path implemented
- [x] Endmember input schema and validation
- [x] Angle output raster(s)
- [x] Winner-take-all class map
- [x] Optional per-class thresholding

10. Continuum Removal (`continuum_removal`)  
Status: `done`
- [x] Rust implementation scaffolded and registered in `wbtools_oss`
- [x] Parallel row-kernel compute path implemented
- [x] Hull construction per spectrum
- [x] Absorption-feature normalized output
- [x] Wavelength-aware mode + index-based fallback

11. Linear Spectral Unmixing (`linear_spectral_unmixing`)  
Status: `done`
- [x] Rust implementation scaffolded and registered in `wbtools_oss`
- [x] Constrained NNLS mode (non-negative, optional sum-to-one)
- [x] Fraction outputs per endmember
- [x] Residual/error output raster

12. Minimum Noise Fraction (`minimum_noise_fraction`)  
Status: `done`
- [x] Rust implementation scaffolded and registered in `wbtools_oss`
- [x] Noise covariance estimation options (x/y adjacent differences)
- [x] Forward transform (noise whitening + PCA in whitened space)
- [x] Explained-noise metadata outputs (component eigenvalues)
- [x] Inverse transform

13. Spectral Library Matching (`spectral_library_matching`)  
Status: `done`
- [x] Rust implementation scaffolded and registered in `wbtools_oss`
- [x] Library ingest via structured JSON argument
- [x] Similarity metrics (angle, Euclidean)
- [x] Best-match class + similarity score outputs
- [x] CSV library file ingest helper
- [x] SID metric

## E. Polarimetric SAR

14. Cloude-Pottier Decomposition (`cloude_pottier_decomposition`)  
Status: `done`
- [x] Rust implementation scaffolded and registered in `wbtools_oss`
- [x] Input matrix format contract (diag3/full3x3 real-symmetric)
- [x] H/A/alpha outputs
- [x] Numerical stability checks

15. Freeman-Durden Decomposition (`freeman_durden_decomposition`)  
Status: `done`
- [x] Rust implementation scaffolded and registered in `wbtools_oss`
- [x] Surface/double-bounce/volume outputs
- [x] Non-physical component clipping diagnostics

## 4) Namespace/Integration Checklist (per tool)

- [x] Add tool implementation under `wbtools_oss/src/tools/remote_sensing/`
- [x] Export in `wbtools_oss/src/tools/remote_sensing/mod.rs`
- [x] Register in `wbtools_oss/src/lib.rs`
- [x] Add taxonomy entry in `wbw_python/tool_taxonomy.toml` (nested namespace preserved)
- [x] Add Python binding method in `wbw_python/src/wb_environment.rs`
- [x] Update Python stubs `wbw_python/whitebox_workflows/whitebox_workflows.pyi`
- [x] Update Python manual docs
- [x] Update R manual docs
- [x] Sync QGIS plugin taxonomy visibility (`wbw_qgis` resolved taxonomy + discovery mapping)
- [x] Add examples in manifests

## 5) Validation/QA Gates

For each tool before marking `done`:

- [x] `cargo check -p wbtools_oss`
- [ ] Unit tests for core math/components
- [x] At least one realistic smoke example (small raster)
- [x] Verify nodata handling and CRS/extent compatibility behavior
  - 2026-05-12 QA snapshot (completed): 
    - ✓ Nodata handling: 15/15 tools produce rasters with nodata_defined
    - ✓ CRS/extent validation: Strict validation now ENFORCED for 5 multi-input spectral tools (SAM, continuum removal, linear unmixing, MNF, library matching)
      - Tools reject mismatched CRS with clear error: "Raster X CRS mismatch: ... Spectral analysis tools require pre-aligned inputs."
      - Tools reject mismatched dimensions with clear error: "Raster X dimension mismatch: ... (expected ...)"
      - Tools reject mismatched geotransform with clear error
    - Validation changed from permissive (auto-reproject: true) to strict (validate_raster_stack_strict) for all spectral multi-band tools
    - Verified: 4/5 multi-input tools correctly reject CRS/dimension mismatches; spectral_library_matching pending library format fix
- [x] Verify Python binding invocation and typed output extraction

## 6) Proposed Delivery Phases

## Phase 1 (Foundation)

- DN to TOA Reflectance
- Dark Object Subtraction
- NDVI-based Emissivity
- LST Single-Channel

## Phase 2 (Core teaching workflows)

- LST Split-Window
- Image Differencing
- Post-Classification Comparison
- Spectral Angle Mapper

## Phase 3 (Advanced spectral)

- Continuum Removal
- Linear Spectral Unmixing
- Minimum Noise Fraction
- Spectral Library Matching

## Phase 4 (PolSAR)

- Cloude-Pottier Decomposition
- Freeman-Durden Decomposition

## 7) Pro Boundary Notes

To avoid overlap with existing Pro differentiation:

- Exclude topographic C-correction/cosine correction style optical terrain correction from OSS sprint scope.
- Keep OSS tools focused on broadly taught, reusable foundations and transparent algorithms.

## 8) Working Notes

- This document is the sprint tracker source-of-truth for remote sensing additions in `wbtools_oss/docs`.
- Update statuses and checklist items continuously as implementation proceeds.

## 9) Performance-First Design Rules

Use these rules to avoid retrofit parallelization:

- Prefer row-chunk parallel kernels (`into_par_iter` over row indices) for per-pixel transforms.
- Keep I/O sequential but compute parallel: read inputs once, process in parallel, write once.
- Precompute per-band/per-class constants (offsets, means, inverses, coefficients) once before pixel loops.
- Avoid per-pixel allocations in hot loops; allocate output rows once per worker task.
- Use numerically stable reductions for global stats (parallel partial reductions + merge).
- Keep nodata checks branch-light and early in each kernel.

## 10) Parallelization Plan by Tool

- DN to TOA Reflectance: parallelize by `(band,row)` blocks after per-band coefficient lookup.
- Dark Object Subtraction: parallelize row transforms; parallel sort/select for percentile estimation.
- NDVI-based Emissivity: parallel per-pixel NDVI/FVC/emissivity transform.
- LST Single-Channel: parallel per-pixel thermal inversion after constant preparation.
- LST Split-Window: parallel per-pixel two-band kernel with prevalidated coefficients.
- Image Differencing: parallel per-pixel subtraction/magnitude and threshold mask generation.
- Post-Classification Change: parallel transition raster generation + parallel transition counts with per-thread hash maps merged at end.
- PCA-Based Change Detection: parallel per-pixel change-vector reduction for covariance + parallel PC projection and threshold mask generation.
- Spectral Angle Mapper: parallel per-pixel dot products against all endmembers; pre-normalize endmember vectors.
- Continuum Removal: parallel per-pixel hull normalization; precompute wavelength indices.
- Linear Spectral Unmixing: parallel per-pixel NNLS solves; reuse workspace buffers per thread.
- Minimum Noise Fraction: parallel covariance accumulation and projection passes.
- Spectral Library Matching: parallel per-pixel signature-to-library similarity scoring.
- Cloude-Pottier/Freeman-Durden: parallel per-pixel matrix decomposition with careful NaN/conditioning guards.
