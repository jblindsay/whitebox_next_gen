# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]

### Added
- Added an internal Phase 1 execution checklist for WbW-Py usability/capability improvements:
  - `docs/internal/wbw_py_phase1_execution_checklist.md`
- Added an internal alias inventory for Phase 1 canonical/removed/temporary API paths:
  - `docs/internal/wbw_py_alias_inventory.md`
- Added an initial WbW-Py/WbW-R parity ledger with explicit per-change decisions:
  - `docs/internal/wbw_py_wbw_r_parity_ledger.md`
- Added an internal interoperability behavior matrix covering preservation/drift semantics,
  copy boundaries, and verification guidance across NumPy, Rasterio, GeoPandas,
  Shapely, xarray/rioxarray, and pyproj:
  - `docs/internal/wbw_py_interop_behavior_matrix.md`

### Changed
- Started Phase 1 documentation cleanup by adding a "Preferred API conventions" section to `README.md`.
- Improved stub guidance in `whitebox_workflows.pyi` to clarify preferred canonical methods (`metadata()` over legacy alias paths) and the topology utility-vs-tools namespace split.
- Updated internal planning docs/checklists to reflect two constraints: pre-release
  API clarity can take priority over backward compatibility, and significant
  WbW-Py API changes should include explicit WbW-R parity decisions.
- Removed selected high-confusion pre-release aliases from the WbW-Py API surface:
  - `Raster.configs()` (use `Raster.metadata()`)
  - `Vector.get_attributes()` / `Vector.get_attribute()` (use `attributes()` / `attribute()`)
  - `Vector.set_attributes()` / `Vector.set_attribute()` (use `update_attributes()` / `update_attribute()`)
  - `Vector.add_attribute_field()` (use `add_field()`)
  - `WbEnvironment` category property aliases `*_tools` for raster/vector/lidar/remote_sensing (use canonical category properties)
- Added intent-driven README entry points for common tasks (read/process/write/reproject/interop) to improve first-use discovery.
- Expanded top-level `.pyi` guidance comments for `WbEnvironment` categories and utility namespaces (`projection`, `topology`, `topology_tools`) and dynamic category usage.
- Added a README interoperability behavior matrix and copy-vs-view guidance to support
  future user-manual generation from README source content.