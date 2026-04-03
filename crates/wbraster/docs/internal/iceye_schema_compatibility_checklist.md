# ICEYE Schema Compatibility Checklist

Last updated: 2026-04-01
Status: **Base reader implemented.** Priorities 1–3 are addressed by the current
`IceyeBundle` implementation. Priorities 4–5 remain open work items.

Primary references:
- https://sar.iceye.com/latest/productFormats/metadata/
- https://sar.iceye.com/latest/productspecification/dataproducts/
- https://sar.iceye.com/latest/opendata/opendata/

## Purpose

Use this checklist to harden `IceyeBundle` parsing/indexing against real ICEYE deliveries
before and during integration of newly received sample scenes.

This checklist is intentionally implementation-focused: each item maps to a field,
filename pattern, or behavior in `src/packages/iceye_bundle.rs`.

## Current Reader Snapshot

Current `IceyeBundle` already supports:
- Bundle-level TIFF discovery with canonical asset keys and collision-safe keying.
- Polarization extraction from metadata and filename tokenization.
- Metadata XML parsing for core fields (product type, mode, acquisition time, orbit/look direction,
  incidence near/far, range/azimuth spacing).
- Unit-tolerant numeric extraction (e.g., values like `2.5 m`, `20.0 deg`).
- Opt-in smoke test via `WBRASTER_ICEYE_SAMPLE`.

## Priority 1: Product Packaging Coverage ✅ (addressed in current reader)

1. Confirm delivery variants include one or more of:
- COG GeoTIFF + JSON metadata (current/public format)
- Legacy formats still seen in the wild until deprecation windows expire

2. Confirm bundle indexing behavior for:
- Single-asset products (`..._GRD.tif`)
- Multi-asset same-pol products (e.g., GRD and derivatives sharing `VV`)
- Mixed-pol products (`VV` + `VH`, etc.)

3. Verify that non-raster sidecars do not affect indexing:
- `.json`, `.xml`, `.png`, `.kml`, `.gif`, `.mp4`

Acceptance criteria:
- `IceyeBundle::open` succeeds.
- `list_asset_keys()` is non-empty and stable.
- No silent overwrites when multiple files imply same polarization key.

## Priority 2: File Naming Compatibility ✅ (addressed in current reader)

ICEYE docs list two relevant naming families:
- Regular naming pattern:
  `ICEYE_<geohash>_<iso-datetime>_<image-id>_<satellite-id>_<imaging-mode>_<product>.<extension>`
- Legacy naming pattern:
  `ICEYE_<satellite-id>_<product>_<imaging-mode>_<image_id>_<iso-datetime>.<extension>`

Checklist:
1. Validate tokenization against both naming styles.
2. Validate separator tolerance (`_`, `-`, `.` and mixed patterns).
3. Validate lower/upper-case robustness for polarization tokens.
4. Validate asset keys for non-pol suffixes/prefixes (do not lose pol inference).

Acceptance criteria:
- `asset_path("VV")` or equivalent still resolves where expected.
- `list_polarizations()` reports correct unique polarization set.

## Priority 3: Metadata Field Fallback Matrix ✅ (addressed in current reader)

### 3.1 Temporal fields
Required candidate tags to check in real samples:
- `datetime`
- `start_datetime`
- `end_datetime`
- existing XML tags already parsed (`acquisition_start_utc`, variants)

Action:
- Add fallback tag aliases if real scenes use JSON-only or alternate XML tag names.

### 3.2 Product/mode fields
Verify and map:
- Product type equivalents (e.g., `GRD`, `SLC`, `SLC-COG`, etc.)
- Mode fields:
  - `sar:instrument_mode` (metadata reference)
  - short mode values from naming conventions (`SLF`, `SM`, `SCW`, `SLEA`, etc.)

Action:
- Add aliases so `acquisition_mode` is populated across XML/JSON variants.

### 3.3 Polarization fields
Verify and map:
- XML style: `polarization` / `polarisation`
- Metadata arrays: `sar:polarizations`

Action:
- If `sar:polarizations` appears in sidecar JSON, parse it as fallback.

### 3.4 Geometry/radiometry core fields
Verify and map:
- Look direction:
  - `sar:observation_direction`
  - XML look-side variants
- Orbit direction:
  - `sat:orbit_state`
  - XML orbit variants
- Incidence near/far:
  - `iceye:incidence_angle_near`
  - `iceye:incidence_angle_far`
- Pixel spacing:
  - `sar:pixel_spacing_range`
  - `sar:pixel_spacing_azimuth`

Action:
- Add JSON key fallback parsing for above when XML values are absent.

Acceptance criteria:
- Core metadata fields in `IceyeBundle` are populated from either XML or JSON.
- Missing one source does not zero out values available in the other.

## Priority 4: Amplitude Mapping Awareness ⚠️ (open — not yet implemented)

Metadata reference documents `iceye:amplitude_mapping` (for example `power` transform
with exponent 0.25 for some SLC COG amplitude bands).

Current state:
- Reader does not yet expose amplitude-mapping metadata in struct fields.

Action:
1. Decide if this belongs in package-level metadata now or a later radiometric helper API.
2. If now, add optional field with raw object/text and document semantics.
3. Add a regression test fixture covering `function=minmax`, `clip`, `power` cases.

Why this matters:
- Prevents misinterpretation of amplitude values during downstream analysis.

## Priority 5: Open Data Program Validation ⚠️ (partially open — smoke test added but no fixture-backed tests yet)

ICEYE Open Data Program on AWS indicates publicly accessible COG + JSON assets.

Catalog entry points:
- S3 website browser:
  `http://iceye-open-data-catalog.s3-website-us-west-2.amazonaws.com/?prefix=`
- STAC browser:
  `https://radiantearth.github.io/stac-browser/#/external/iceye-open-data-catalog.s3-us-west-2.amazonaws.com/catalog.json?.language=en`

Action:
1. Pull 2-4 representative open-data scenes across different product/mode combinations.
2. Validate `IceyeBundle::open` and metadata extraction.
3. Add fixture-backed tests for any newly observed schema keys.

## Test Plan Template (When Data Arrives)

1. Basic open
- `IceyeBundle::open(path)` succeeds.
- `list_asset_keys()` non-empty.

2. Metadata completeness
- Check `product_type`, `acquisition_datetime_utc`, `acquisition_mode`,
  `polarization`, `orbit_direction`, `look_direction`, incidence and spacing.

3. Multi-file stability
- Confirm duplicate-pol assets are retained as unique keys.

4. Polarization grouping
- `list_polarizations()` and `read_assets_for_polarization()` include all expected assets.

5. Smoke test wiring
- Set `WBRASTER_ICEYE_SAMPLE` and run:
  `cargo test -p wbraster opens_real_iceye_sample_when_env_set`

## Immediate Follow-up Tasks

1. Add JSON metadata parsing fallback path to `IceyeBundle` (high-value, low-risk).
2. Add tests with synthetic JSON sidecars for:
- `sar:instrument_mode`
- `sar:polarizations`
- `sat:orbit_state`
- `sar:observation_direction`
- `sar:pixel_spacing_*`
- `iceye:incidence_angle_*`
3. Add one integration test using ICEYE open-data sample once a stable public scene URL is selected.

## Definition Of "Good Enough" Before Proprietary Full Scenes

We can consider pre-production compatibility reasonably strong when all conditions hold:
1. Reader passes current unit tests and smoke tests.
2. Reader succeeds on at least 2 distinct public ICEYE open-data scenes.
3. Core fields are populated from XML or JSON fallbacks.
4. No key-collision data loss in multi-asset bundles.
5. Known metadata unknowns are explicitly documented (not silent).
