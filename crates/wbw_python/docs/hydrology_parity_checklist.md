# Hydrology Depression Removal Parity Checklist

Reference legacy source:
- /Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology

Scope:
- New backend implementations in wbtools_oss
- Initial focus on DEM depression removal and conditioning tools

## Status Legend
- done: implemented and compiled in new backend
- partial: present but still behaviorally simplified vs legacy
- todo: not yet parity-complete

## Tool-by-Tool Status
- breach_depressions_least_cost: done
- breach_single_cell_pits: done
- fill_depressions: done
- fill_depressions_planchon_and_darboux: done
- fill_depressions_wang_and_liu: done
- fill_pits: done

## Current Implementation Notes
- New hydrology subset module added under wbtools_oss.
- fill_depressions now uses a legacy-style pit-to-outlet priority growth and outlet-driven back-fill routine, including flat-fixing from confirmed outlets.
- fill_depressions_wang_and_liu now uses a distinct Wang-Liu priority-flood path seeded from edge-connected nodata boundaries.
- fill_depressions_planchon_and_darboux now uses the iterative multi-direction scan strategy of the legacy Planchon-Darboux workflow.
- breach_depressions_least_cost now applies legacy-style pit preconditioning, least-cost breaching with backlink path carving, and optional unresolved-depression fill.

## Validation Status
- cargo check -p wbtools_oss -p wbtools_pro: pass
- cargo test -p wbtools_oss --test registry_integration -- --nocapture: pass
  - Includes new hydrology-oriented integration checks for:
    - registry presence of all six hydrology depression tools
    - fill_pits single-cell pit conditioning behavior
    - breach_single_cell_pits local breach carving behavior

## High-Value Remaining Parity Work
- Add DEM regression corpus comparisons (legacy vs backend rasters + summary stats) to quantify parity over diverse terrain and nodata patterns.
- Add targeted tests for max_depth, flat_increment, fill_deps, max_cost, and max_dist parameter interactions.
