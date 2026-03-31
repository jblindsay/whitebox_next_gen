# Hydrology Diagnostics Parity Checklist

Reference legacy source:
- /Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology

Scope:
- New backend implementations in wbtools_oss
- Focused on DEM and pointer diagnostics adjacent to flow routing and watershed delineation

## Status Legend
- done: implemented and compiled in new backend
- partial: present but still behaviorally simplified vs legacy
- todo: not yet implemented in new backend

## Tool-by-Tool Status
- find_noflow_cells: done
- num_inflowing_neighbours: done
- find_parallel_flow: done

## Current Implementation Notes
- `find_noflow_cells` identifies valid DEM cells that lack any lower D8 neighbour.
- `num_inflowing_neighbours` derives a D8 field from a DEM and counts inflowing neighbours per cell.
- `find_parallel_flow` detects stream cells whose local D8 directions run parallel to neighboring stream cells.

## Validation Status
- cargo check -p wbtools_oss -p whitebox_workflows: pass
- cargo test -p wbtools_oss --test registry_integration -- --nocapture: pass

## High-Value Remaining Parity Work
- Add regression tests for irregular nodata margins and interior flats.
- Add ESRI pointer compatibility coverage where relevant for pointer-driven diagnostics.
- Expand diagnostics coverage toward `find_parallel_flow` on more realistic stream masks.
