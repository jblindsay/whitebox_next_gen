# Supported HDF Product Layouts (Current Scope)

Date: 2026-05-31

This matrix summarizes the currently validated HDF layout support across `wbhdf`, `wblidar`, and `wbraster`.

## Scope Matrix

| Family | Example path | Metadata discovery | Payload decode | Consumer integration | Current status |
|---|---|---|---|---|---|
| GEDI (HDF5) | `/BEAM0000/elev_lowestmode` | Yes | Yes (bounded contiguous `f32` windows) | `wblidar` adapter path | Supported (initial Tier 1 path) |
| ICESat-2 ATL08 (HDF5) | `/gt1l/land_segments/canopy/h_canopy` | Yes (beam-group discovery + v1 header ranking) | Yes (bounded first-chunk `f32` + fill mapping) | `wblidar` adapter + product registry | Supported (initial Tier 1 path) |
| VIIRS (HDF5/NetCDF4-style) | `/HDFEOS/GRIDS/.../XDim` | Yes | Yes for validated contiguous references and validated bounded multilevel chunk-index windows on real fixtures (VNP13/VNP21 reference paths) | `wbhdf` validated decode path + bounded `wbraster` geolocation URI reads | Supported (bounded validated scope) |
| MODIS (HDF4/HDF-EOS2) | `/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01` | Yes | Yes (bounded SDS `i16` window decode path with guardrails) | `wbhdf` validation + `wbraster` URI dispatch | Partial |
| Generic HDF5 raster URI in `wbraster` | `scene.h5#dataset=/dataset/path` | URI recognized | Yes (bounded scalar scope): metadata-driven contiguous scalar materialization (`f32`/`f64`) and bounded chunked recursive traversal scalar materialization (`f32`/`f64`) when object-header + chunk-index metadata resolve cleanly, plus bounded VIIRS science-field URI materialization (`i16`/`u16`/`u8`) through validated multilevel chunk-index traversal; validated GEDI/VIIRS contiguous references, real VNP21 geolocation + science URI reads, real VNP13 NDVI/EVI/EVI2 URI reads, and synthetic one-chunk, two-chunk, two-chunk-deflate, two-leaf, internal-root-success, multilevel-root-success with sibling internal fanout, malformed-multilevel-root, and malformed-multilevel-fanout coverage | `wbraster::Raster::read` | Supported (bounded validated scope) |
| HDF4 raster URI in `wbraster` | `scene.hdf:///Grid/Field` | Yes | Yes for 2D `DFNT_INT16` SDS | `wbraster::Raster::read` | Supported (initial bounded path) |

## URI Contract (Current)

`wbraster` currently accepts canonical form:

- `container.ext#dataset=/absolute/dataset/path`

and also accepts a legacy alias for compatibility:

- `container.ext:///absolute/dataset/path`

Examples:

- `sample.hdf#dataset=/GridA/FieldA`
- `sample.h5#dataset=/ScienceData/NDVI`

Behavior:

- HDF4 (`.hdf`, `.h4`): attempts bounded raster materialization for 2D `DFNT_INT16` SDS datasets.
- HDF5/NetCDF (`.h5`, `.hdf5`, `.he5`, `.nc`): materializes contiguous scalar datasets when object-header metadata resolves a supported contiguous layout and datatype width (`4` or `8` bytes). Also supports a bounded chunked fallback for scalar layouts whose records are reachable from the chunk index address through leaf nodes and bounded internal-node recursion using the current internal-record shape, with either no declared filter or a single deflate/zlib filter. Validated reference paths include GEDI `/BEAM0000/elev_lowestmode`, VIIRS `/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/XDim`, real VNP21 URI reads for geolocation (`/VIIRS_Swath_LSTE/Geolocation Fields/latitude`, `/VIIRS_Swath_LSTE/Geolocation Fields/longitude`) plus selected science fields (`/VIIRS_Swath_LSTE/Data Fields/LST`, `/VIIRS_Swath_LSTE/Data Fields/LST_err`), and real VNP13 URI reads for selected vegetation fields (`/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days NDVI`, `...EVI`, `...EVI2`); malformed-tree layouts and non-scalar layouts remain out of current `wbraster` scope and fail with explicit diagnostics.

## Troubleshooting

### 1) Missing dataset path

Symptom:
- Error contains `dataset path resolution failed`.

Checks:
- Ensure path starts at root and is exact, e.g. `/GridName/DataFieldName`.
- For ATL08, ensure beam group path exists (`/gt1l`, `/gt2r`, etc.).

### 2) Unsupported filter/layout

Symptom:
- Error contains `UnsupportedLayout` or filter/decode diagnostics.

Checks:
- Confirm dataset uses currently validated layout/filter combinations.
- Re-run with smaller bounded windows if probing a large or unusual product.

### 3) Fill-value mismatch expectations

Symptom:
- Unexpected nodata counts or canopy values.

Checks:
- Verify dataset fill sentinel in metadata/object-header messages.
- Confirm consumer mapping semantics (`fill -> nodata`) for that path.

### 4) HDF5 URI in `wbraster` fails for some paths

Symptom:
- Error indicates `HDF5 raster materialization could not resolve supported layout` with contiguous/chunked detail, or unsupported scalar width for the dataset URI.

Meaning:
- URI parsing is working as designed; this is an intentional staged boundary.
- Current `wbraster` HDF5 materialization support is metadata-driven for contiguous scalar datasets and bounded chunked recursive traversal scalar datasets (`f32`/`f64`) with validated real multilevel VIIRS geolocation URI coverage.
- Use validated `wblidar` Tier 1 paths for broader HDF5 payload reads.

## Boundaries to Remember

- The implementation is intentionally scoped and product-layout targeted, not a full general-purpose HDF4/HDF5 reader.
- Non-raster LiDAR HDF integration is primarily routed through `wblidar` provider/registry dispatch.
- `wbraster` HDF URI support is currently a bounded bridge: HDF4 2D `DFNT_INT16`, validated HDF5 contiguous scalar (`f32`/`f64`) materialization, and validated bounded chunked recursive scalar fallback using the current internal-record shape; broader complex/non-scalar HDF5 raster materialization remains later work.
