# Watershed / Basin Tools Parity Checklist

Tracks porting status for hydrology watershed and basin delineation tools.

## Status Legend

- ✅ Done – ported, Python wrapper added, docs written, integration tests passing
- 🔲 Todo – not yet ported
- ⚠️ Partial – code exists but tests or docs incomplete

---

## Tool Status

| Tool | OSS impl | Python wrapper | Docs | Tests | Status |
|------|----------|----------------|------|-------|--------|
| `basins` | ✅ | ✅ | ✅ | ✅ | ✅ Done |
| `watershed_from_raster_pour_points` | ✅ | ✅ | ✅ | ✅ | ✅ Done |
| `watershed` (vector) | ✅ | ✅ | ✅ | ✅ | ✅ Done |
| `subbasins` | ✅ | ✅ | ✅ | ✅ | ✅ Done |
| `isobasins` | ✅ | ✅ | ✅ | ✅ | ✅ Done |
| `hillslopes` | ✅ | ✅ | ✅ | ✅ | ✅ Done |
| `strahler_order_basins` | ✅ | ✅ | ✅ | ✅ | ✅ Done |
| `jenson_snap_pour_points` | ✅ | ✅ | ✅ | ✅ | ✅ Done |

---

## Remaining Work

All watershed and basin tools have been ported. No outstanding items.

---

## Test Coverage Gaps

- `watershed_from_raster_pour_points`: no round-trip test against legacy whitebox_workflows output; no multi-watershed test with >2 outlets.
- `watershed` (vector): single-outlet test only; needs multi-outlet and NoData boundary tests.
- `basins`: single-outlet 1×3 test only; needs a 2D multi-basin test.
- ESRI pointer encoding parity not yet verified for any watershed tool.

---

## Notes

- All tools use the same two-pass downstream-walk labeling algorithm (see `run_watershed_labeling` in `hydrology/mod.rs`).
- Tools that share `build_flow_dir_and_mark_nodata` + `run_watershed_labeling` helpers: `watershed_from_raster_pour_points`, `watershed`.
- `basins` uses an equivalent inline approach with edge-outlet seeding.
- The `world_to_pixel` return convention is `(col, row)` — this must be respected when indexing the output grid from vector coordinates.
