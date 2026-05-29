# LiDAR Roadmap Implementation Checklist
Date: 2026-05-28
Status: Execution checklist draft
Scope: Convert LiDAR roadmap into actionable milestone work items

Related documents:
- docs/internal/LIDAR_GAP_ROADMAP_2026-05-28.md
- docs/internal/LIDAR_IMAGE_V1_SPEC_2026-05-28.md
- docs/internal/LIDAR_POINT_NEIGHBOURHOOD_METRICS_PLAN_2026-05-28.md

## 1. Milestone Overview
- M0: Planning and contracts
- M1: Shared intensity normalization module
- M2: lidar_image core v1
- M3: lidar_neighbourhood_point_metrics core v1
- M4: Wrappers, QGIS, docs, and validation
- M5: Next-wave LiDAR gaps (flightline correction bridge, canopy metrics, dormant workflow resolution)

## 2. Dependency Order
1. M0 -> M1
2. M1 -> M2 and M3
3. M2 + M3 -> M4
4. M4 -> M5

## 3. Detailed Checklist

## M0. Planning and Contracts

### M0-T01 Define normalization schema contract
- Priority: P0
- Owner: TBD
- Dependencies: none
- Tasks:
- Define normalization mode enum and parameter names.
- Define metadata keys for output provenance.
- Define warning and fallback semantics when required metadata is missing.
- Acceptance criteria:
- Contract doc section approved and linked from related specs.
- Exact field names frozen for v1 wrappers and QGIS labels.

### M0-T02 Define shared utility API surface
- Priority: P0
- Owner: TBD
- Dependencies: M0-T01
- Tasks:
- Define core function signatures for normalization and metadata emission.
- Define error model (hard errors vs warnings).
- Acceptance criteria:
- API sketch accepted for both lidar_image and neighbourhood metrics use.

### M0-T03 Benchmark fixture inventory
- Priority: P0
- Owner: TBD
- Dependencies: none
- Tasks:
- Identify small, medium, large test clouds and strip-overlap example.
- Record expected runtime envelope targets (relative targets acceptable).
- Acceptance criteria:
- Fixture list documented with source paths and intended checks.

## M1. Shared Intensity Normalization Module

### M1-T01 Implement none and range modes
- Priority: P0
- Owner: TBD
- Dependencies: M0-T02
- Tasks:
- Implement pass-through (none) mode.
- Implement range normalization mode with documented assumptions.
- Acceptance criteria:
- Unit tests for deterministic outputs and no-op behavior.

### M1-T02 Implement range_angle mode (guarded)
- Priority: P0
- Owner: TBD
- Dependencies: M1-T01
- Tasks:
- Add angle-aware adjustment path with explicit fallback when incidence estimate is unavailable.
- Acceptance criteria:
- Clear warnings emitted when approximation path is used.
- Tests cover both full and fallback paths.

### M1-T03 Implement strip_robust mode
- Priority: P0
- Owner: TBD
- Dependencies: M1-T01
- Tasks:
- Per-strip robust normalization path using strip grouping field.
- Acceptance criteria:
- Multi-strip test confirms reduced strip artifacts in summary metrics.

### M1-T04 Metadata and provenance writer
- Priority: P0
- Owner: TBD
- Dependencies: M1-T01
- Tasks:
- Emit normalization mode and parameter metadata.
- Emit warning/fallback flags in outputs.
- Acceptance criteria:
- Output metadata contains required keys and values for all modes.

## M2. lidar_image Core V1

### M2-T01 Tool scaffold and argument parser
- Priority: P0
- Owner: TBD
- Dependencies: M0-T01
- Tasks:
- Add tool entrypoint and parse v1 parameter surface.
- Acceptance criteria:
- Tool appears in registry and runs with minimal valid args.

### M2-T02 Intensity mode with methods nearest mean max
- Priority: P0
- Owner: TBD
- Dependencies: M2-T01
- Tasks:
- Implement cell aggregation methods for intensity channel.
- Acceptance criteria:
- Synthetic tests pass for nearest, mean, and max semantics.

### M2-T03 RGB mode with scaling options
- Priority: P0
- Owner: TBD
- Dependencies: M2-T01
- Tasks:
- Implement 3-band output for RGB-capable inputs.
- Implement scaling options: auto, none, stretch_percentile.
- Acceptance criteria:
- RGB test fixture outputs valid 3-band raster with expected channel behavior.

### M2-T04 Integrate shared normalization module
- Priority: P0
- Owner: TBD
- Dependencies: M1-T01, M1-T04, M2-T02
- Tasks:
- Wire normalization modes to intensity pipeline.
- Acceptance criteria:
- All normalization modes selectable and reflected in output metadata.

### M2-T05 Performance hardening
- Priority: P0
- Owner: TBD
- Dependencies: M2-T02
- Tasks:
- Introduce chunked processing and thread-local accumulators.
- Acceptance criteria:
- No unbounded memory growth on large fixture.
- Runtime target meets M0 benchmark envelope.

## M3. lidar_neighbourhood_point_metrics Core V1

### M3-T01 Tool scaffold and sidecar schema output
- Priority: P0
- Owner: TBD
- Dependencies: M0-T01
- Tasks:
- Add tool entrypoint and sidecar output writer.
- Acceptance criteria:
- Tool writes metrics sidecar plus schema JSON.

### M3-T02 Neighbourhood engine sphere and cylinder
- Priority: P0
- Owner: TBD
- Dependencies: M3-T01
- Tasks:
- Implement shared neighbourhood retrieval abstraction.
- Acceptance criteria:
- Deterministic neighborhood selection tests pass for both modes.

### M3-T03 Geometry and density metric family
- Priority: P0
- Owner: TBD
- Dependencies: M3-T02
- Tasks:
- Implement height, roughness/plane residual, and density metrics.
- Acceptance criteria:
- Synthetic expected-value tests pass.

### M3-T04 Return and intensity metric family
- Priority: P0
- Owner: TBD
- Dependencies: M3-T03, M1-T01
- Tasks:
- Add return-structure metrics.
- Add raw and normalized intensity metrics.
- Acceptance criteria:
- Metrics availability flags emitted when data fields are missing.

### M3-T05 One-pass optimization and scratch reuse
- Priority: P0
- Owner: TBD
- Dependencies: M3-T03
- Tasks:
- Ensure one neighborhood query per point for selected metric set.
- Reuse per-thread scratch buffers.
- Acceptance criteria:
- Profiling confirms no per-metric neighbor re-query path.

## M4. Wrappers, QGIS, Docs, Validation

### M4-T01 Python wrappers and docs
- Priority: P0
- Owner: TBD
- Dependencies: M2-T04, M3-T04
- Tasks:
- Add Python signatures and examples.
- Acceptance criteria:
- Wrappers compile and examples execute on fixtures.

### M4-T02 R wrappers and docs
- Priority: P0
- Owner: TBD
- Dependencies: M2-T04, M3-T04
- Tasks:
- Add R wrapper signatures and examples.
- Acceptance criteria:
- Generated docs and wrappers consistent with tool args.

### M4-T03 QGIS integration and typing checks
- Priority: P0
- Owner: TBD
- Dependencies: M2-T04, M3-T04
- Tasks:
- Ensure output and parameter typing in QGIS processing dialogs are correct.
- Verify temporary output materialization behavior.
- Acceptance criteria:
- No mis-typed numeric parameters.
- No incorrect file extension forcing for non-layer outputs.

### M4-T04 Validation and benchmark report
- Priority: P0
- Owner: TBD
- Dependencies: M4-T01, M4-T02, M4-T03
- Tasks:
- Run correctness suite and benchmark fixtures.
- Publish internal benchmark summary.
- Acceptance criteria:
- Report generated with pass/fail and runtime summary.

## M5. Next-Wave Gaps

### M5-T01 Flightline diagnostics-to-correction bridge design
- Priority: P1
- Owner: TBD
- Dependencies: M4-T04
- Tasks:
- Draft correction workflow and QA output plan.
- Acceptance criteria:
- Design spec approved for implementation.

### M5-T02 Canopy structure base metrics tooling
- Priority: P1
- Owner: TBD
- Dependencies: M3-T02
- Tasks:
- Draft and implement first transparent canopy metric set.
- Acceptance criteria:
- At least one canopy metric product with documented equations and tests.

### M5-T03 Resolve dormant lidar_dem_full_workflow status
- Priority: P1
- Owner: TBD
- Dependencies: none
- Tasks:
- Decide port, supersede, or retire-with-guidance.
- Acceptance criteria:
- Explicit decision documented and reflected in user-facing notes.

## 4. Definition of Done (Program Level)
- Core tools implemented and registered.
- Python and R wrappers available.
- QGIS integration validated for parameter/output typing.
- Benchmark and correctness reports produced.
- Internal docs cross-linked and consistent.

## 5. Suggested First Sprint Cut
- Include tasks:
- M0-T01, M0-T02, M1-T01, M1-T04, M2-T01, M2-T02
- Sprint goal:
- Deliver first end-to-end intensity image path with explicit normalization metadata and stable argument contract.

## 6. Likely Code Touchpoints (Kickoff Map)
Use this as the default file map when creating implementation issues.

### 6.1 Core LiDAR tool implementation (OSS runtime)
- crates/wbtools_oss/src/tools/lidar_processing/mod.rs
	- Primary implementation location for existing LiDAR tools (including lidar_eigenvalue_features and interpolation tools).
	- Expected home for new lidar_image and lidar_neighbourhood_point_metrics implementations.
- crates/wbtools_oss/src/tools/mod.rs
	- Re-export list for tool structs.
	- Add public re-exports for new tool structs.
- crates/wbtools_oss/src/lib.rs
	- Runtime tool registration in register_default_tools.
	- Add registry entries for new tools.

### 6.2 Shared LiDAR data and format support
- crates/wblidar/src/lib.rs
	- Public LiDAR crate surface (LAS/LAZ/COPC/PLY/E57 and helpers).
- crates/wblidar/tools/src/mod.rs
	- Candidate location for shared reusable helpers if we decide to separate normalization/math utility code from wbtools_oss tool code.

### 6.3 Python bindings and wrappers
- crates/wbw_python/src/wb_environment.rs
	- Add user-facing WbEnvironment methods for new tools.
- crates/wbw_python/src/lib.rs
	- Runtime registry composition and manifest/stub plumbing.
	- Touch only if additional binding/runtime behavior is needed.

### 6.4 R bindings and wrappers
- crates/wbw_r/src/lib.rs
	- Runtime bridge and wrapper-exposed tool execution.
	- Primary touchpoint for any non-generated binding behavior.
- crates/wbw_r/generated/wbw_tools_generated.R
	- Generated wrapper surface (update through normal generation flow).
- crates/wbw_r/r-package/whiteboxworkflows/R/zz_generated_wrappers.R
	- Packaged generated wrappers consumed by users.

### 6.5 QGIS plugin integration
- crates/wbw_qgis/plugin/whitebox_workflows_qgis/algorithm.py
	- Parameter kind inference, output kind typing, temporary output materialization.
- crates/wbw_qgis/plugin/whitebox_workflows_qgis/discovery.py
	- Catalog hydration/normalization and fallback parameter handling.
- crates/wbw_qgis/plugin/whitebox_workflows_qgis/help.py
	- Help generation and metadata-enriched descriptions.
- crates/wbw_qgis/plugin/whitebox_workflows_qgis/provider.py
	- Catalog load and algorithm exposure.
- crates/wbw_qgis/plugin/whitebox_workflows_qgis/tool_taxonomy.resolved.json
	- Taxonomy category/subcategory mapping and visibility metadata.

### 6.6 Docs and planning artifacts
- docs/internal/LIDAR_IMAGE_V1_SPEC_2026-05-28.md
- docs/internal/LIDAR_POINT_NEIGHBOURHOOD_METRICS_PLAN_2026-05-28.md
- docs/internal/LIDAR_GAP_ROADMAP_2026-05-28.md

### 6.7 Suggested task-to-file mapping
- M1 shared normalization module:
	- Start in crates/wbtools_oss/src/tools/lidar_processing/mod.rs
	- Consider extraction to a dedicated helper module after API stabilizes.
- M2 lidar_image:
	- Implement in crates/wbtools_oss/src/tools/lidar_processing/mod.rs
	- Register via crates/wbtools_oss/src/tools/mod.rs and crates/wbtools_oss/src/lib.rs
	- Bind via crates/wbw_python/src/wb_environment.rs and crates/wbw_r/src/lib.rs/generation outputs
	- Validate typing/materialization in crates/wbw_qgis/plugin/whitebox_workflows_qgis/algorithm.py
- M3 lidar_neighbourhood_point_metrics:
	- Same path pattern as M2, with sidecar schema support in tool implementation.
- M4 wrappers/QGIS/docs:
	- Focus bindings and plugin files above plus the internal planning docs.
