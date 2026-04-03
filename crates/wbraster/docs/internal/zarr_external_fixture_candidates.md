# Zarr External Fixture Candidates (Phase 1 Item 2)

Purpose: track concrete internet sources for real-world geospatial Zarr fixtures and how to wire them into local, opt-in integration tests.

## Current Test Hooks

`tests/integration.rs` includes two opt-in external fixture tests:

- `external_zarr_v2_fixture_smoke_local_path`
  - env var: `WBRASTER_EXTERNAL_ZARR_V2_FIXTURE`
- `external_zarr_v3_fixture_smoke_local_path`
  - env var: `WBRASTER_EXTERNAL_ZARR_V3_FIXTURE`

These tests are skipped by default and only run when the corresponding local fixture path is set.

Optional parity env vars are also supported (per fixture prefix):

- `*_EXPECT_ROWS`
- `*_EXPECT_COLS`
- `*_EXPECT_NODATA`
- `*_EXPECT_CELL` with format: `row,col,value[,tol]`

Examples:

- `WBRASTER_EXTERNAL_ZARR_V2_EXPECT_ROWS=4201`
- `WBRASTER_EXTERNAL_ZARR_V2_EXPECT_COLS=8401`
- `WBRASTER_EXTERNAL_ZARR_V2_EXPECT_CELL=0,0,-9999,1e-6`

## Verified Public Geospatial v2 Candidates

1. NOAA AORC v1.1 (AWS Open Data)
- Bucket listing root:
  - `https://noaa-nws-aorc-v1-1-1km.s3.amazonaws.com/`
- Verified yearly Zarr store prefix example:
  - `https://noaa-nws-aorc-v1-1-1km.s3.amazonaws.com/?prefix=1979.zarr/&max-keys=200`
- Verified metadata object presence:
  - `https://noaa-nws-aorc-v1-1-1km.s3.amazonaws.com/?prefix=1979.zarr/.zmetadata`
- Notes:
  - Public, anonymous-readable listing via HTTPS.
  - Geospatial gridded forcing data.
  - The store appears to be Zarr v2 (`.zgroup`, `.zattrs`, `.zmetadata` keys are present).

2. IPFS-hosted geospatial Zarr examples from IPFS docs
- Example CID #1 (42 MB):
  - `https://ipfs.io/ipfs/bafybeiesyutuduzqwvu4ydn7ktihjljicywxeth6wtgd5zi4ynxzqngx4m`
- Example CID #2 (883 MB):
  - `https://ipfs.io/ipfs/bafybeif52irmuurpb27cujwpqhtbg5w6maw4d7zppg2lqgpew25gs5eczm`
- Notes:
  - Both expose directory listings including `.zgroup` and `.zattrs`.
  - These appear to be Zarr v2 examples (not `zarr.json` roots).

## Discovery/Index Sources (Useful for finding more fixtures)

1. Pangeo STAC catalog root
- URL: `https://raw.githubusercontent.com/pangeo-data/pangeo-datastore-stac/master/master/catalog.json`

2. Pangeo Hydrology catalog
- URL: `https://raw.githubusercontent.com/pangeo-data/pangeo-datastore-stac/master/master/hydro/catalog.json`

3. Planetary Computer STAC API root
- URL: `https://planetarycomputer.microsoft.com/api/stac/v1/`
- Notes:
  - Rich source of geospatial Zarr assets (`zarr-https`, `zarr-abfs` in collection assets).
  - Some direct blob URLs may require signing or specific access policies.

4. Cloud-Native Geospatial guide Zarr examples
- URL: `https://guide.cloudnativegeo.org/zarr/zarr-in-practice.html`
- Notes:
  - Contains concrete v2 remote example:
    - `https://ncsa.osn.xsede.org/Pangeo/pangeo-forge/gpcp-feedstock/gpcp.zarr`
  - Also references Planetary Computer Zarr access patterns.

## Access Constraints Observed

During probing, several promising cloud-hosted `*.zarr` roots were not directly readable anonymously from this environment (for example, some Azure Blob endpoints reported public access restrictions). For this reason, fixture acquisition should remain explicit and local, then fed into env-gated tests.

Also observed: confidently verifiable public geospatial v3 roots (with direct `zarr.json`) are still scarce relative to v2 in broadly anonymous-access archives.

## Recommended Next Step

1. Materialize one stable public geospatial v2 fixture locally (AORC or IPFS example), then set parity env vars (`*_EXPECT_ROWS`, etc.) so the test becomes a stronger conformance check.
2. For v3, use an external producer pipeline (e.g., xarray/zarr-python v3) to create a tiny geospatial fixture locally from a publicly available source tile while we continue searching for a stable anonymous geospatial v3 root.
2. Run:

```bash
WBRASTER_EXTERNAL_ZARR_V2_FIXTURE=/absolute/path/to/v2_fixture.zarr \
WBRASTER_EXTERNAL_ZARR_V3_FIXTURE=/absolute/path/to/v3_fixture.zarr \
WBRASTER_EXTERNAL_ZARR_V2_EXPECT_ROWS=<rows> \
WBRASTER_EXTERNAL_ZARR_V2_EXPECT_COLS=<cols> \
WBRASTER_EXTERNAL_ZARR_V3_EXPECT_ROWS=<rows> \
WBRASTER_EXTERNAL_ZARR_V3_EXPECT_COLS=<cols> \
cargo test -p wbraster external_zarr_
```

3. Once fixture provenance and access are stable, promote to checked-in tiny fixtures (or scripted downloads) and expand assertions beyond smoke checks.
