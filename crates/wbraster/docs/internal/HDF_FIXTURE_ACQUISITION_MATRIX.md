# HDF Fixture Acquisition Matrix

Date: 2026-05-31
Status: Draft for implementation kickoff
Owner: wbhdf / wbraster / wblidar

## Purpose

Track where high-quality fixtures come from, what authentication is required,
and how tests should behave when external assets are unavailable.

## Fixture Tiers

- Tier A (committed): tiny synthetic or reduced fixtures (KB to low MB), committed in-repo.
- Tier B (external cache): realistic product fixtures (100MB+) fetched on demand, not committed.
- Tier C (optional extended): large or difficult-to-access fixtures used for manual validation.

## Acquisition Matrix

| Family | Product Targets | Container Format | Primary Source | Auth Required | Automation Confidence | Fallback Source(s) | Notes |
|---|---|---|---|---|---|---|---|
| GEDI | GEDI L2B sample granules/tiles | HDF5 | NASA LP DAAC / Earthdata | Yes (Earthdata account/token) | Medium | NASA CMR search workflows; user-provided fixtures | High value for wblidar Tier 1. Plan for scripted fetch with token in local env only. |
| ICESat-2 | ATL03, ATL08 | HDF5 | NSIDC DAAC / Earthdata | Yes (Earthdata account/token) | Medium | User-provided fixtures | Core Tier 1 input; prioritize one small ATL03 and one ATL08 granule for smoke paths. |
| VIIRS | VNP09, VNP13, VNP21 (and VJ variants) | HDF5 / NetCDF4-style | NASA LAADS DAAC | Usually yes (Earthdata auth flow for downloads) | Medium-High | LP DAAC mirror where available; user-provided fixtures | Strong long-term fixture availability; LAADS API transition in progress, keep fetch tooling current. |
| MODIS | MOD09, MOD13, MOD11 (and MYD variants) | HDF4 / HDF-EOS2 | NASA LAADS/LP DAAC | Usually yes (Earthdata auth flow for downloads) | Medium-High | User-provided fixtures | High practical availability and broad historical coverage; good candidate for reproducible fixture set. |
| Sentinel-5P | Representative L2 products | NetCDF (HDF5-backed) | Copernicus Data Space Ecosystem (CDSE) | Account login typically required for API flows | Medium | Alternative Copernicus mirrors; user-provided fixtures | Useful for HDF-backed NetCDF behavior; lower priority than GEDI/ICESat-2/MODIS/VIIRS. |
| ALOS-2 MLC | PALSAR-2 MLC style products | HDF5 (complex) | Mission/provider-specific portals | Varies; often constrained | Low | User-provided fixtures (preferred) | Highest fixture risk. Publicly easy data tends to be ScanSAR NRB GeoTIFF, not the complex MLC path needed here. |

## Practical Recommendation

- Start implementation validation with: GEDI + ATL03/ATL08 + one VIIRS + one MODIS target.
- Treat ALOS-2 MLC as contingent on fixture procurement from project-owned or user-provided sources.
- Require all external fixture tests to skip gracefully when credentials/files are absent.

## Proposed Environment Variables

- WBHDF_FIXTURE_DIR: local directory containing large external fixtures.
- WBHDF_EARTHDATA_TOKEN: token used by optional fetch helpers (never committed).
- WBHDF_ENABLE_EXTERNAL_FIXTURES: if set to 1, run Tier B/C tests.

## CI Policy (Lightweight)

- Default CI: Tier A only.
- Optional CI job (manual or scheduled): Tier B smoke if secure credentials are available.
- Local developer workflow: Tier A always; Tier B when fixture directory is configured.

## First Fixture Pull Priorities

1. GEDI L2B small representative file with known canopy-height dataset path.
2. ICESat-2 ATL08 representative file with beam-group hierarchy.
3. VIIRS VNP09 representative gridded product.
4. MODIS MOD09 representative gridded product.

## Open Risks

- NASA endpoint and API transitions may require fetch-script updates.
- Terms/access constraints can vary by endpoint and may change over time.
- ALOS-2 complex fixtures may remain manual-procurement only in the near term.
