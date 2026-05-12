# TopoJSON Read and Write Implementation Plan

Date: 2026-05-12
Status: active implementation (Phase 1 complete; Phase 2 in progress)
Scope: wbvector format driver addition

## Current Progress Snapshot (2026-05-12)

Completed in code:
- Core TopoJSON read and write module integrated in wbvector.
- `VectorFormat` detect/read/write routing includes `.topojson` and JSON sniff support.
- TopoJSON-specific error variants implemented in `GeoError`.
- Baseline TopoJSON example added: `crates/wbvector/examples/topojson_io.rs`.
- Fixture corpus added under `crates/wbvector/tests/fixtures/topojson_io`.
- Fixture-driven TopoJSON tests added for parser breadth and transform decode behavior.
- Interop-oriented fixture variants added for object-map multi-member topologies,
  foreign members/bbox tolerance, and explicit reversed arc references.
- Added producer-style compatibility fixtures for mapshaper-like geometry
  collections, topojson-server-like quantized transforms, and FeatureCollection
  null-geometry handling.
- Added fixture provenance manifest and a parity test that enforces one-to-one
  coverage between manifest entries and TopoJSON fixture files.
- Writer quality upgrade implemented: ring canonicalization now normalizes rotated/reversed equivalent rings to improve deterministic arc reuse.
- Optional writer options implemented: quantization-enabled output (`transform` + delta arcs) and optional root `bbox` emission.
- Downstream format visibility updates started in wbw_python and wbw_r:
  - `.topojson` extension inference added for vector output coercion.
  - Supported format manuals updated.
  - TopoJSON roundtrip example scripts added and indexed.
- Downstream wbw_qgis visibility updates completed:
  - Plugin vector path/parameter heuristics now recognize `.topojson`.
  - QGIS manual supported-format table includes TopoJSON read/write.
  - QGIS manual now includes an end-to-end TopoJSON conversion-chain workflow.
  - QGIS manual scenario library expanded with additional TopoJSON workflows.

Still pending for full Phase 2 (optional):
- External-source fixture provenance import pass (pinning canonical third-party
  sample files) for long-term compatibility tracking.

## Goal

Add TopoJSON support to wbvector with both read and write capability from the first implementation milestone.

This plan is intentionally dependency-conservative:
- No new crate dependencies for initial implementation.
- Reuse existing in-house JSON parsing and geometry conversion patterns.
- Consider external dependencies only if a hard blocker appears and measurable value is clear.

## Constraints

1. Read and write must both be delivered, not reader-only.
2. New dependencies are disallowed unless they provide clearly irreplaceable value.
3. Keep compatibility with existing Layer, Feature, FieldValue, and Geometry models.
4. Preserve existing wbvector format-driver style and sniffed I/O integration.

## Recommended Delivery Shape

Deliver in two internal phases inside one sprint stream, with both read and write available at Phase 1 completion.

Phase 1 (minimum complete):
- Read Topology and decode into Layer.
- Write Layer to valid Topology JSON.
- Focus on correctness and compatibility first; topology compression quality can be basic.

Phase 2 (quality and parity):
- Improve arc sharing and canonicalization quality in writer.
- Expand compatibility with more object structures and edge cases.
- Tighten roundtrip invariants and fixture breadth.

## Format Scope

Target TopoJSON elements for first pass:
- type: Topology
- transform: scale and translate (optional, support both present and absent)
- arcs: delta-encoded coordinate sequences
- objects:
  - GeometryCollection
  - Point, MultiPoint
  - LineString, MultiLineString
  - Polygon, MultiPolygon
  - Feature-like properties and id payloads where present
- bbox (read-through, optional write)

## Non-Goals For First Pass

- Perfectly optimal topology compression.
- Aggressive global arc deduplication heuristics beyond deterministic baseline.
- Quantization tuning UI/options in public API.

## Architecture Plan

## 1. New module

Add new format module:
- src/topojson/mod.rs

Public API shape should mirror existing format drivers:
- read(path)
- write(layer, path)
- parse_str(text)
- to_string(layer)

## 2. Parsing strategy

Use in-house JSON value model and parser strategy patterned after geojson module.

Implementation options:
- Preferred: extract a tiny internal JSON core module shared by geojson and topojson.
- Alternate: keep parser local to topojson for lower refactor risk.

Decision rule:
- If extraction can be done without destabilizing geojson tests, share parser.
- If not, duplicate parser now and refactor later.

## 3. Topology decoding

Read path core steps:
1. Parse JSON and validate root type is Topology.
2. Read transform if present.
3. Decode arcs from delta arrays to absolute coordinates:
- Keep integer-like precision as f64 in Coord.
- Apply transform if present.
4. Resolve geometry objects:
- Support negative arc indices (reverse arc semantics).
- Assemble LineString, Polygon rings, and multi variants from arc references.
5. Build Layer:
- Map properties to schema using existing inference patterns.
- Populate geometry and attributes into features.

## 4. Topology encoding (writer)

Writer first-pass strategy:
1. Convert Layer geometries to linework/rings.
2. Build deterministic arc table:
- Start with simple canonical arc keying for exact coordinate matches and reversed matches.
- Reuse arc indices across features when exact matches exist.
3. Emit objects as GeometryCollection under a default named object.
4. Emit Topology with arcs and optional transform disabled by default for first pass.

Phase 2 writer improvements:
- Optional transform quantization mode.
- Better arc splitting/canonicalization to increase sharing.

## 5. Crate integration

Update crate exports and format routing:
- src/lib.rs:
  - add topojson module
  - add VectorFormat variant for TopoJson
  - wire detect logic for .topojson and .json sniff
  - wire read and write dispatch
- README format tables and feature lists
- examples: add topojson_io example

## 6. Error model

Extend GeoError with TopoJSON-specific variants:
- TopoJsonParse with offset and message
- TopoJsonMissing for required members
- TopoJsonType for unknown or unsupported type tokens
- TopoJsonTopology for arc/object resolution errors

## Test Plan

## Unit tests

1. Parse and write minimal valid topology.
2. Arc delta decoding correctness with and without transform.
3. Negative arc index reversal correctness.
4. Polygon ring assembly from multiple arcs.
5. Properties and id mapping.

## Roundtrip tests

1. Layer -> TopoJSON -> Layer preserves feature count, schema shape, geometry type, and coordinates within tolerance.
2. Topology -> Layer -> Topology structural validity and deterministic output ordering.

## Fixture tests

Add fixture corpus under tests/fixtures/topojson_io:
- simple_points.topojson
- shared_boundary_polygons.topojson
- multilines_shared_arcs.topojson
- transform_quantized_example.topojson
- mixed_geometry_collection.topojson

## Interop sanity tests

Use offline known-good TopoJSON examples from trusted sources as text fixtures only.
No runtime GEOS/JTS or external topology engine dependency.

## Complexity Estimate

Reader:
- Medium effort.
- Main complexity: arc resolution and transform handling.

Writer:
- Medium-high effort.
- Main complexity: deterministic arc table generation and reasonable sharing behavior.

Combined first usable read+write delivery:
- Approximately 1 to 2 focused weeks, depending on fixture breadth and writer quality target.

## Risk Register

1. Arc orientation/reversal bugs.
- Mitigation: explicit negative-index tests and geometry invariants.

2. Writer over-fragmented arcs reduce sharing quality.
- Mitigation: phased improvement; baseline correctness first.

3. Property/schema drift across mixed object types.
- Mitigation: reuse existing schema inference and widening strategy.

4. JSON parser drift between geojson and topojson.
- Mitigation: shared internal parser if low-risk, otherwise strict duplicate test coverage.

## Sprint Execution Checklist

1. Create topojson module with parse and serialize scaffolding.
2. Implement arc decode and object resolve pipeline.
3. Implement baseline writer with deterministic arc indexing.
4. Wire VectorFormat detect/read/write integration.
5. Add fixture corpus and roundtrip tests.
6. Update README and examples.
7. Run:
- cargo test -p wbvector
- cargo check -p wbvector

## Task Execution Board (Estimated Hours)

The board below is designed for checkpoint commits and restart safety.

1. TopoJSON module scaffold and API wiring in wbvector
- Estimate: 3 to 5 hours
- Outputs: src/topojson/mod.rs with read, write, parse_str, to_string; basic tests compiling

2. JSON handling strategy implementation
- Estimate: 3 to 6 hours
- Outputs: either shared internal JSON core with geojson or local topojson parser copy with tests

3. Arc decode and transform application
- Estimate: 4 to 8 hours
- Outputs: robust decode path for arcs with optional transform scale and translate

4. Geometry object resolution from arc references
- Estimate: 6 to 10 hours
- Outputs: Point, MultiPoint, LineString, MultiLineString, Polygon, MultiPolygon, GeometryCollection mapping

5. Baseline writer with deterministic arc indexing and reverse arc reuse
- Estimate: 8 to 14 hours
- Outputs: valid Topology JSON output from Layer with stable ordering

6. Error model extension and diagnostics
- Estimate: 2 to 4 hours
- Outputs: GeoError TopoJSON variants and clear failure messages

7. wbvector format registration and sniffed I/O integration
- Estimate: 2 to 4 hours
- Outputs: VectorFormat variant, detect routing for .topojson, read/write dispatch updates

8. Fixture corpus and roundtrip tests
- Estimate: 6 to 10 hours
- Outputs: topojson fixture set, unit tests, roundtrip tests, interop sanity tests

9. Examples and primary docs updates
- Estimate: 2 to 4 hours
- Outputs: topojson_io example plus README supported format updates

10. Downstream integration updates across frontends/manuals
- Estimate: 4 to 8 hours
- Outputs: wbw_python, wbw_r, and wbw_qgis docs/examples/manual touchpoints updated

Total estimated effort:
- Minimum: 40 hours
- Typical: 52 to 64 hours
- High-confidence with extra hardening: 72 hours

## Suggested Commit Sequence

Keep commits small and restart-friendly.

1. wbvector: scaffold topojson module and public API surface
2. wbvector: add TopoJSON parser/value handling path
3. wbvector: implement arc decode plus transform support
4. wbvector: map TopoJSON objects to Geometry and Layer
5. wbvector: implement TopoJSON writer with deterministic arc indexing
6. wbvector: register TopoJSON in VectorFormat detect/read/write routing
7. wbvector: add TopoJSON fixture corpus and roundtrip coverage
8. wbvector: add topojson_io example and README support table updates
9. wbw_python: update format docs/examples/manual for TopoJSON I/O
10. wbw_r: update format docs/examples/manual for TopoJSON I/O
11. wbw_qgis: update plugin/manual format visibility and user guidance
12. docs: add release notes and cross-crate change summary

## Downstream Implications and Knock-On Checklist

TopoJSON support in wbvector affects more than the core crate. The following items should be tracked in the same sprint stream.

## A. wbw_python API and docs

Target paths:
- crates/wbw_python/src
- crates/wbw_python/manual
- crates/wbw_python/docs
- crates/wbw_python/examples

Required updates:
1. Verify Python-facing vector read/write wrappers accept .topojson paths through existing format sniffing.
2. Add TopoJSON to supported format tables in Python docs/manual.
3. Add at least one Python example converting GeoJSON or Shapefile to TopoJSON and back.
4. Add manual notes on topology semantics and expected roundtrip behavior.
5. Add or update Python tests that exercise TopoJSON read and write through user-facing API entry points.

## B. wbw_r API and docs

Target paths:
- crates/wbw_r/src
- crates/wbw_r/manual
- crates/wbw_r/docs
- crates/wbw_r/examples

Required updates:
1. Verify R wrappers can read/write .topojson via existing vector I/O functions.
2. Add TopoJSON to R manual supported-format lists.
3. Add R example script for TopoJSON conversion workflow.
4. Document any caveats around arc sharing and deterministic output behavior.
5. Add or update R integration tests for TopoJSON roundtrip paths.

## C. wbw_qgis plugin and manual

Target paths:
- crates/wbw_qgis/plugin
- crates/wbw_qgis/manual
- crates/wbw_qgis/docs

Required updates:
1. Confirm plugin format support lists include TopoJSON where applicable.
2. Add short user guidance for importing and exporting TopoJSON datasets.
3. Add regression check that plugin workflows do not hide or reject .topojson file targets.

## D. Tooling, taxonomy, and generated artifacts

1. Confirm whether any generated format-support manifests need regeneration after adding TopoJSON.
2. If format support is surfaced in generated docs or wrappers, regenerate and commit synchronized outputs.
3. Ensure examples index pages and any TOOLS or capability docs reflect TopoJSON support.

## E. QA and release implications

1. Add TopoJSON fixtures to CI-relevant test paths for wbvector.
2. Add at least one cross-crate smoke test path (wbvector plus one frontend wrapper) before release.
3. Update wbvector CHANGELOG and downstream changelogs where maintained.
4. Add release note entry summarizing:
- TopoJSON read support
- TopoJSON write support
- Known limitations in first release (for example, non-optimal arc sharing).

## F. File-Level Touch Matrix (Downstream)

Use this as a concrete checklist of likely files to modify when TopoJSON support lands.

wbvector core:
- crates/wbvector/src/lib.rs
- crates/wbvector/src/error.rs
- crates/wbvector/src/topojson/mod.rs
- crates/wbvector/examples/topojson_io.rs
- crates/wbvector/README.md
- crates/wbvector/CHANGELOG.md
- crates/wbvector/tests/fixtures/topojson_io/*

wbw_python integration:
- crates/wbw_python/src/wb_environment.rs
- crates/wbw_python/tool_taxonomy.toml
- crates/wbw_python/tool_taxonomy.resolved.json
- crates/wbw_python/manual/src/supported-data-formats.md
- crates/wbw_python/manual/src/working-with-vectors.md
- crates/wbw_python/manual/src/interoperability.md
- crates/wbw_python/manual/src/script-index.md
- crates/wbw_python/manual/src/SUMMARY.md
- crates/wbw_python/examples/* (add one TopoJSON conversion example)
- crates/wbw_python/CHANGELOG.md

wbw_r integration:
- crates/wbw_r/r-package/whiteboxworkflows/R/facade.R
- crates/wbw_r/r-package/whiteboxworkflows/R/bindings.R
- crates/wbw_r/r-package/whiteboxworkflows/R/zz_generated_wrappers.R
- crates/wbw_r/r-package/whiteboxworkflows/inst/extdata/tool_taxonomy.resolved.json
- crates/wbw_r/manual/src/supported-data-formats.md
- crates/wbw_r/manual/src/working-with-vectors.md
- crates/wbw_r/manual/src/interoperability.md
- crates/wbw_r/manual/src/script-index.md
- crates/wbw_r/manual/src/SUMMARY.md
- crates/wbw_r/r-package/whiteboxworkflows/inst/examples/* (add one TopoJSON workflow script)

wbw_qgis integration:
- crates/wbw_qgis/plugin/whitebox_workflows_qgis/tool_taxonomy.resolved.json
- crates/wbw_qgis/plugin/whitebox_workflows_qgis/discovery.py
- crates/wbw_qgis/plugin/whitebox_workflows_qgis/provider.py
- crates/wbw_qgis/plugin/whitebox_workflows_qgis/help_provider.py
- crates/wbw_qgis/manual/src/data-formats.md
- crates/wbw_qgis/manual/src/vector-analysis.md
- crates/wbw_qgis/manual/src/recipes.md
- crates/wbw_qgis/manual/src/SUMMARY.md

Project-level sync and release docs:
- scripts/sync_tool_taxonomy.py
- docs/performance/* if format benchmark notes are captured there

## G. First 48 Hours Execution Script

Use this exact order when starting implementation work to minimize drift.

Day 1:
1. Implement wbvector TopoJSON read and write baseline in a feature-complete but simple form.
2. Wire VectorFormat detection and dispatch for .topojson.
3. Add fixture corpus and core roundtrip tests.
4. Run:
- cargo test -p wbvector
- cargo check -p wbvector

Day 2:
1. Add example and README updates in wbvector.
2. Regenerate taxonomy/resolved artifacts if required by integration surfaces.
3. Update wbw_python manual and add Python example.
4. Update wbw_r manual and add R example.
5. Update wbw_qgis data format manual and plugin visibility checks.
6. Run:
- cargo check -p wbw_python
- cargo check -p wbw_r
- cargo check -p wbvector

Stabilization gate before merge:
1. Confirm at least one cross-crate smoke workflow passes:
- read TopoJSON -> write GeoJSON
- read GeoJSON -> write TopoJSON
2. Confirm manuals and script indexes include the new format references.

## Cross-Crate Definition of Done

A TopoJSON release is complete when all conditions below are true:

1. wbvector read and write support for TopoJSON is active with passing tests.
2. wbvector README and example set include TopoJSON usage.
3. wbw_python user-facing docs/manual include TopoJSON and one working example.
4. wbw_r user-facing docs/manual include TopoJSON and one working example.
5. wbw_qgis user docs reflect TopoJSON availability where relevant.
6. Cross-crate smoke checks pass for at least one TopoJSON conversion workflow.
7. No new dependencies were introduced unless separately approved with explicit rationale.

## Definition of Done

1. wbvector can read .topojson files into Layer.
2. wbvector can write Layer out as valid Topology JSON.
3. Full wbvector test suite passes.
4. New fixture corpus and roundtrip tests are active.
5. No new dependencies introduced for initial delivery.
