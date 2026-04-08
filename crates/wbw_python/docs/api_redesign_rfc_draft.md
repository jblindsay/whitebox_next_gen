# Whitebox Workflows Python API Redesign RFC (Draft)

Date: 2026-04-08
Status: Draft for design review
Scope: End-to-end API design for wbw_python, beyond only raster/vector/lidar object harmonization

## 1. Why This RFC Exists

The API is already in a breaking-change window. This creates a rare opportunity to optimize for:

- elegant and discoverable user experience,
- consistency across data types and tool families,
- reproducibility and diagnosability,
- clear licensing and capability boundaries,
- long-term maintainability.

This RFC proposes a full-surface design direction, not just tactical fixes.

## 2. Design Goals

1. Be intuitive for Python users and familiar enough for ArcPy users.
2. Avoid global hidden state as much as practical.
3. Make object behavior consistent across Raster, Vector, and Lidar.
4. Keep tool discoverability high while avoiding autocomplete overload.
5. Support robust production scripting with strong diagnostics.
6. Make licensing explicit, inspectable, and predictable.
7. Interoperate cleanly with common Python geospatial and numerical libraries.

## 3. Non-Goals

1. Full syntax compatibility with ArcPy.
2. Immediate removal of all legacy names.
3. Rewriting every tool wrapper before design decisions are finalized.

## 4. Comparison With ArcPy

ArcPy strengths to emulate:

1. Strong geoprocessing mental model: input -> tool -> output.
2. Powerful environment controls.
3. Clear expectation that tools form composable workflows.

ArcPy pain points to avoid:

1. Heavy implicit global state and side effects.
2. Inconsistent return types and error structures.
3. Uneven typing and discoverability experience.

Proposed Whitebox stance:

1. Keep ArcPy-like conceptual flow.
2. Use explicit session/environment objects.
3. Prefer typed, structured returns and exceptions.

## 5. Proposed Top-Level API Shape

### 5.1 Session-Centered Runtime

Introduce a single primary runtime object as the entry point:

- WbSession (new, preferred)

Responsibilities:

- runtime configuration,
- license/capability context,
- tool catalog and execution,
- IO helpers and object factories,
- logging/progress hooks.

Compatibility:

- WbEnvironment remains as a compatibility alias/facade in transition.

### 5.2 Data Object Protocol (Shared)

Raster, Vector, and Lidar should implement a common core protocol:

- metadata()
- save(output_path, ...)
- copy(output_path=None)
- reproject(...)
- exists()
- path (or file_path)

Type-specific APIs remain available, but core verbs become predictable.

### 5.3 Tool Discovery and Invocation

Keep category/domain namespace discovery, but standardize invocation results:

- ToolResult
  - outputs (typed objects and/or scalars)
  - messages
  - warnings
  - timing
  - provenance record id (optional)

Add strict and permissive modes:

- strict=True: unknown args and type mismatches fail fast.
- strict=False: best-effort coercion with warnings.

### 5.4 Execution and Materialization Policy

The API keeps the geoprocessing mental model input -> tool -> output, but the runtime should not require disk writes for every intermediate step.

Default behavior:

1. Memory-first intermediate outputs.
2. Disk materialization only when explicitly requested or operationally required.

Required materialization cases:

1. User explicitly provides output_path/save target.
2. User creates a named checkpoint.
3. Runtime memory pressure requires spill-to-disk.
4. Process boundary or tool requirement requires file-backed input.

Proposed controls:

1. session.execution_policy = memory_first | hybrid | disk_first
2. session.checkpoint(obj, path)
3. session.auto_cleanup_temps = True|False

Design objective:

Preserve conceptual clarity while reducing unnecessary file I/O for long workflows.

## 6. Data Model Harmonization

### 6.1 Metadata

Replace split metadata surfaces with explicit metadata classes:

- Raster.metadata() -> RasterMetadata
- Vector.metadata() -> VectorMetadata
- Lidar.metadata() -> LidarMetadata

Keep compatibility aliases during migration:

- Raster.configs() remains, documented as compatibility.

### 6.2 Vector Attributes (First-Class)

Expose a coherent attribute API:

- vector.schema()
- vector.attributes(feature_index)
- vector.attribute(feature_index, field_name)
- vector.update_attributes(feature_index, values)
- vector.update_attribute(feature_index, field_name, value)
- vector.add_field(...)

Legacy methods map directly:

- get_attributes/get_attribute/set_attributes/set_attribute/add_attribute_field.

### 6.3 Write and Copy Semantics

All save/copy APIs must be format-aware:

- single-file fast-path for true single-file formats,
- dataset-aware logic for multi-file formats.

Vector formats requiring dataset-aware handling:

- Shapefile (.shp + sidecars)
- MapInfo MIF/MID pair (.mif + .mid)

## 7. Licensing and Capability Model

### 7.1 Current Baseline

Current API already supports:

- signed entitlement bootstrap,
- floating license bootstrap,
- include_pro/fallback tier controls,
- session-level license inspection.

### 7.2 Proposed Licensing Design Principles

1. Licensing should be explicit and session-scoped.
2. Tool capability visibility should match executable capability by default.
3. Licensing failures should be structured and actionable.
4. Offline and online flows should share one mental model.

### 7.3 Proposed Objects

- LicenseContext
  - mode: open | signed_entitlement | floating
  - tier: open | pro | enterprise
  - include_pro: bool
  - source: file | json | provider
  - expires_at: optional timestamp
  - diagnostics: structured details

- CapabilitySet
  - allowed_tools
  - locked_tools
  - feature_flags (for future non-tool capabilities)

- LicenseError (structured)
  - code
  - message
  - remediation
  - retryable

### 7.4 API Surface Proposal

Session constructors:

- WbSession.open(...)
- WbSession.from_signed_entitlement(...)
- WbSession.from_floating_license(...)

Session inspection:

- session.license()
- session.capabilities()
- session.can_run(tool_id)

Catalog behavior:

- default list_tools() shows runnable tools only.
- list_tools(include_locked=True) can show locked tools with status.

### 7.5 UX and Security Notes

1. Never require users to parse free-text license strings for logic.
2. Provide stable machine-readable fields.
3. Avoid leaking sensitive entitlement payloads in logs.
4. Keep provider URL and machine/customer IDs explicit for auditability.

## 8. Environment Model

Use explicit environment scope and inheritance:

- session-level defaults (working dir, max_procs, verbosity, temp paths)
- per-call override arguments

No silent global mutation beyond current session unless explicitly requested.

Execution-related environment defaults:

- execution_policy
- temp_workspace
- memory_spill_threshold
- auto_cleanup_temps

## 9. Error and Diagnostics Model

All critical API operations should return/raise structured diagnostics:

- ValidationError
- DataIOError
- ToolExecutionError
- LicenseError

Each includes:

- code,
- summary,
- context object,
- suggested remediation.

## 10. Typing and Discoverability

1. Keep category/domain-first autocomplete design.
2. Provide typed stubs for dynamic surfaces.
3. Keep flat aliases optional and deprioritized in IntelliSense.
4. Ensure metadata and result objects are fully typed.

## 11. Python Ecosystem Interoperability

Interoperability should be a first-class design goal, not an afterthought.

### 11.1 Target Libraries

Numerical:

- NumPy
- xarray
- Dask arrays

Geospatial vector:

- GeoPandas
- Shapely
- PyProj

Geospatial raster:

- Rasterio (interchange workflows)
- rioxarray

Tabular and columnar:

- pandas
- pyarrow

### 11.2 Core Interop Principles

1. Zero-surprise conversions: shape, nodata, and CRS behavior must be explicit.
2. No hard dependency bloat in the core package.
3. Clear copy-vs-view semantics for array conversions.
4. Round-trip fidelity should be testable and documented.

### 11.3 Raster and NumPy Interop (Priority)

Proposed API surface:

- raster.to_numpy(masked=False, band=None, dtype=None, copy=True)
- WbRaster.from_numpy(array, *, transform=None, crs=None, nodata=None, output_path=None)

Semantics:

1. Default shape is row-major 2D for single-band, 3D for multi-band.
2. masked=True returns numpy.ma.MaskedArray using nodata mask.
3. copy=False may return shared memory only when safe; otherwise raises or falls back to copy with warning.
4. dtype controls explicit casting rules and overflow checks.

Metadata contract for from_numpy:

- transform and crs are optional but strongly recommended.
- if omitted, metadata defaults are explicit and surfaced in RasterMetadata warnings.

### 11.4 Xarray and Dask Interop

Proposed API surface:

- raster.to_xarray(name=None, chunks=None)
- WbRaster.from_xarray(data_array, output_path=None, nodata=None)

Semantics:

1. Preserve coordinate axes and CRS metadata where available.
2. chunks enables lazy Dask-backed arrays when Dask is present.
3. If xarray or dask is unavailable, raise optional-dependency error with install hint.

### 11.5 Vector Interop

Proposed API surface:

- vector.to_geodataframe()
- WbVector.from_geodataframe(gdf, output_path=None)
- vector.to_wkb_list() / vector.to_wkt_list() for lightweight exchange

Attribute semantics:

1. Preserve field names, nullability, and value types as far as target library supports.
2. On lossy conversions, emit conversion diagnostics.

### 11.6 Lidar Interop

Near-term practical path:

- lidar.to_numpy(fields=[...], include_header=False)
- WbLidar.from_numpy(points, schema, output_path, crs=None)

Future optional integrations:

- PDAL/laspy bridge helpers, kept in optional extra modules.

### 11.7 Dependency Strategy

Use optional extras to avoid forcing large dependency trees:

- wbw[interop-numpy]
- wbw[interop-vector]
- wbw[interop-raster]
- wbw[interop-full]

Core wbw_python should remain usable without these extras.

### 11.8 Error Model for Interop

Add structured conversion errors:

- InteropDependencyError
- InteropSchemaError
- InteropDataLossWarning (warning class)

Each includes library name and actionable install or remediation guidance.

### 11.9 Suggested Phase Ordering

1. Phase A: Raster <-> NumPy and Vector <-> GeoPandas high-value bridges.
2. Phase B: xarray/dask integration and richer vector geometry interchange.
3. Phase C: lidar structured array bridges and advanced ecosystem adapters.

## 12. Migration Plan

Phase A: additive, non-breaking

1. Add WbSession aliasing current WbEnvironment internals.
2. Add metadata() methods and typed metadata classes.
3. Add vector attribute alias methods.
4. Add structured license()/capabilities() inspection helpers.

Phase B: behavior hardening

1. Make vector save/copy dataset-aware for Shapefile and MIF/MID.
2. Add unified save/copy naming across data objects.
3. Add structured tool results for selected high-impact wrappers.
4. Add execution policy plumbing for memory-first intermediates and explicit checkpointing.

Phase C: deprecation and cleanup

1. Deprecate legacy naming where replacement exists.
2. Keep compatibility shims for a documented window.
3. Remove deprecated paths in next major version.

## 13. Acceptance Criteria

1. New users can complete read -> inspect metadata -> run tool -> write workflow with one consistent pattern across Raster/Vector/Lidar.
2. ArcPy users can identify equivalent concepts quickly.
3. Licensing state is inspectable via machine-readable API without parsing text.
4. Vector save/copy preserves required dataset components for multi-file formats.
5. Type stubs provide reliable autocomplete for all core objects.
6. Raster to NumPy conversion is one-step and documented, with deterministic nodata semantics.
7. At least one vector bridge path (GeoPandas) is available with schema/attribute fidelity tests.
8. Long multi-step workflows can run without mandatory intermediate disk writes unless policy or tool constraints require it.

## 14. Open Questions

1. Do we want WbSession as a new class name, or keep WbEnvironment as canonical and only modernize behavior?
2. Should tool calls always return ToolResult, or only when requested via an option?
3. Should session-level logging/provenance be on by default or opt-in?
4. What deprecation timeline is acceptable for legacy method names?
5. Should NumPy become a required dependency or remain optional with an extra?
6. What minimum metadata fidelity is required for from_numpy constructors?
7. What default spill threshold is safe across typical user hardware?

## 15. Immediate Next Design Tasks

1. Freeze canonical names for session, metadata, and vector attribute APIs.
2. Define metadata schemas (RasterMetadata, VectorMetadata, LidarMetadata).
3. Define LicenseContext and CapabilitySet schemas.
4. Draft a minimal implementation slice for Phase A with tests and stubs.
5. Draft interop contracts for Raster <-> NumPy and Vector <-> GeoPandas.
6. Draft execution policy semantics and checkpoint API contract.
