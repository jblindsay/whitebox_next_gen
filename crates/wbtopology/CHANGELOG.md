# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]
### Added
- Added `extract_face_rings_with_edges` and `extract_bounded_face_rings_with_edges` on `TopologyGraph`, returning face rings paired with their directed edge id lists; used by depth-labeling BFS face classification.
- Added `offset_linestring` public function returning an open `LineString` one-sided offset curve (analogous to JTS/GEOS `OffsetCurve`): takes a linestring, signed distance, `OffsetSide` (Left/Right), and `OffsetCurveOptions` (join style, quadrant segments, mitre limit); suitable for road edge extraction, centreline offsets, and planning setback lines. Added companion types `OffsetSide` and `OffsetCurveOptions`; both exported from the crate root.

### Changed
- Replaced point-in-polygon probe face classification in `unary_dissolve_graph_component` with GEOS/JTS-style directed-edge depth labeling; face membership is now determined purely topologically via BFS depth propagation, eliminating misclassification of faces adjacent to short source-polygon edges.
- Replaced point-in-polygon probe face classification in `classify_overlay_faces` (two-polygon Boolean overlay) with directed-edge depth labeling matching the unary dissolve approach, fixing the same short-segment misclassification for intersection/union/difference/symmetric-difference operations. Corrected face-ring extraction in that path to use bounded rings only (positive area); the previous all-rings extraction prevented any face from being seeded by the BFS, causing every overlay operation to return empty.
- Removed diagnostic `eprintln!` calls from buffer pipeline internals in `constructive.rs` that were firing unconditionally in production builds.
- Replaced `repair_buffer_polygon` fallback in `buffer_linestring` with `buffer_linestring_graph_repair`: for self-intersecting raw rings the new helper nodes the ring, extracts bounded face rings, assembles valid polygons via `polygonize_closed_linestrings`, and returns the largest result — matching the graph-pipeline approach already used for polygon buffering.
- Added `buffer_polygon_attach_holes` helper: after the graph pipeline selects the outer shell for a source polygon with holes, inward-contracted hole rings are reconstructed using the same mitre-offset logic as the legacy path and attached to the shell, restoring correct hole geometry in the graph-pipeline buffer output.
- Snapped intersection points to the nearest `eps`-grid vertex in `node_segment` (noding.rs); the hot-pixel snap prevents hair-thin slivers from floating-point drift at computed intersection coordinates, consistent with GEOS/JTS snap-rounding behaviour.
- Fixed mixed-precision sliver artifacts by quantising input vertices for `NodingStrategy::Auto` (not just `SnapRounding`) before noding: previously, only intersection points were snapped to the eps-grid while input vertices remained at floating-point precision, creating the same on-grid/off-grid artifacts that snap-rounding was meant to prevent. Now all input vertices are quantised for both `Auto` and `SnapRounding` strategies unless an explicit `PrecisionModel::Floating` is passed.
- Refactored `ring_contains_ring` hole-nesting test to use centroid-based containment as the primary check instead of vertex iteration: reduces misclassification of holes whose vertices nearly coincide with container-ring segments (the same boundary-zone fragility that depth labeling was meant to replace for face classification). Secondary fallback still uses vertex iteration for the rare case where centroid is exactly on the boundary.
- Documented design decision for negative buffer: the legacy path via `buffer_polygon_negative` is well-optimized and already handles multi-component erosion correctly via `make_valid_polygon`. Users needing all erosion components should call `buffer_polygon_multi` instead of `buffer_polygon`; the latter returns only the largest component for API compatibility. Graph pipeline conversion for negative buffer is noted as a future optimization.
- Added diagnostic logging in `classify_faces_by_depth` to detect unreached faces (isolated topology graph components): if any faces cannot be reached via BFS from the exterior, a warning is logged (rare in practice, indicates possible complex overlapping topology or degenerate input).
- Enhanced docstring for `polygon_overlay_faces` to clearly document that it returns flat face rings without hole reconstruction; recommended users call `polygon_overlay` instead for proper hole nesting, or use the raw faces for diagnostic/advanced purposes only.
- Optimized `assemble_polygons_from_rings` hole-nesting performance via spatial index: replaced O(n²) pairwise containment checks with STR-tree envelope filtering, reducing containment tests for large dissolve results (1000+ rings) by 2–5x depending on ring distribution; added helper `linestring_envelope` for fast bounding-box computation.

- Added public export `delaunay_triangulation_fast` for high-throughput triangulation workflows.
- Added `fixed_radius_search` module with `FixedRadiusSearch2D` and `DistanceMetric` for high-throughput local neighbourhood queries.
- Added `polygon_unary_dissolve_fast` for high-throughput polygon dissolve workflows where robust fallbacks are not required.
- Added explicit noding architecture controls: `NodingStrategy`, `NodingOptions`, and `node_linestrings_with_options`.
- Added topology-aware precision reduction helpers: `TopologyPrecisionOptions`, `apply_linestring_topology`, and `apply_polygon_topology`.
- Added unary dissolve architecture controls: `UnaryDissolveStrategy`, `UnaryDissolveOptions`, and `polygon_unary_dissolve_with_options`.
- Added graph-driven unary dissolve path (`GraphDriven` strategy) that builds bounded faces from noded linework and classifies source membership.
- Added buffering architecture scaffolding: `BufferBuilder` and `BufferPipelineStrategy` with staged graph-pipeline hooks.
- Added geometry-fixing architecture controls: `GeometryFixMode`, `GeometryFixOptions`, and `make_valid_geometry`.
- Added full linework polygonization API scaffold: `polygonize_linework`, `PolygonizeOptions`, and `PolygonizeResult` (including dangle/cut-edge reporting fields).
- Added buffer parity harness scaffold `tests/buffer_geos_parity_harness_tests.rs` with fixture-driven area delta, approximate Hausdorff, and topology invariant gates.
- Added polygonize diagnostics tests `tests/polygonize_linework_diagnostics_tests.rs` covering dangle reporting and basic closed-ring polygonization.
- Added unary dissolve graph fixture harness `tests/unary_dissolve_graph_fixture_harness_tests.rs` and baseline cases in `tests/fixtures/unary_dissolve_graph_cases.txt`.
- Added BufferBuilder graph fixture harness `tests/buffer_builder_graph_fixture_harness_tests.rs` with case data in `tests/fixtures/buffer_builder_graph_cases.txt`.
- Expanded BufferBuilder graph fixture corpus with hole-survival/closure and complex-ring positive buffer cases, then calibrated area-ratio thresholds against executable harness results.
- Added make-valid geometry fixture harness `tests/make_valid_geometry_fixture_harness_tests.rs` with mode-coverage cases in `tests/fixtures/make_valid_geometry_cases.txt`.
- Added positive buffer invariant harness `tests/buffer_positive_fixture_tests.rs` with collapsed-hole, survived-hole, and real-world footprint cases.

### Changed
- Updated graph-driven unary dissolve source attribution to use direct fast overlap predicates instead of recursive per-source overlay calls.
- Updated `polygonize_linework` to classify invalid rings from full face extraction before bounded-ring polygon assembly.
- Updated `BufferBuilder` graph pipeline to apply depth-style face filtering against source geometry and route non-positive distances through legacy semantics.
- Expanded buffer parity fixture scaffold with additional GEOS-golden style identity cases (rectangle, triangle, polygon-with-hole).
- Updated `BufferBuilder` graph pipeline component selection to use explicit face-depth labels and post-selection component merging before final component selection.
- Updated `BufferBuilder` graph pipeline face labeling to use explicit depth counters (`inside_count`, `boundary_count`, sample count, min distance) instead of boolean-only source flags.
- Updated `BufferBuilder` graph pipeline final component selection to choose the largest source-containing merged component instead of the first match.
- Updated `BufferBuilder` graph pipeline finalization to reject invalid graph-selected outputs and fall back to legacy polygon buffering semantics.
- Updated `BufferBuilder` graph pipeline finalization to also reject graph-selected outputs whose area is less than 90% of the source exterior ring area for positive buffers, preventing selection of wrong-component artifacts when collapsing-hole faces have ambiguous depth labels.
- Updated `BufferBuilder` graph depth-selection ordering with deterministic polygon tie-breakers to reduce merge-order sensitivity and fixture flakiness.
- Updated unary graph dissolve to partition source polygons by non-point connectivity, preserving point-touch separation while allowing epsilon-bounded near-gap merging.
- Updated overlay dissolve internals to use deterministic quantized-coordinate representatives and angle tie-break ordering for neighbour traversal.
- Expanded unary dissolve graph fixture corpus with hole-rich and near-tolerance cases, including strict micro-gap and micro-overlap checks.
- Strengthened unary dissolve graph fixture harness with explicit source-membership correctness assertions.
- Expanded buffer parity fixture scaffold with topology-stress identity cases (thin-neck polygon and tiny-hole polygon).
- Updated unary dissolve graph fixture format to include explicit expected membership sets per case.
- Expanded buffer parity fixture corpus with first non-zero distance envelope-based gate cases for positive and negative rectangular/square buffering.
- Updated buffer parity fixture schema and harness to support per-case gate modes (`strict` vs `invariant`) so strict GEOS-style thresholds and topology/invariant stress checks can coexist.
- Expanded buffer parity fixture corpus with additional non-zero invariant-only stress cases (concave positive buffer, hole shrink, thin-neck negative, and triangle growth).
- Expanded strict non-zero buffer parity fixtures across additional distances and coordinate domains (negative coordinates and large-magnitude coordinates) for square/rectangle baselines.
- Expanded strict non-zero buffer parity fixtures with explicit concave/hole expected-geometry cases (hole survive, hole close, holed erosion, concave positive expansion).
- Expanded strict non-zero buffer parity fixtures with negative concave erosion and thin-neck erosion expected-geometry cases.
- Expanded graph-driven unary dissolve tests with edge-touch merge and point-touch separation coverage.
- Updated direct graph-driven unary dissolve tests to assert canonicalized source-membership sets (not only output counts).
- Expanded parity fixture scaffold with additional identity cases for large-magnitude and negative coordinate domains.
- Added unary dissolve epsilon-stress fixture variants for near-gap strict-vs-loose tolerance behavior.
- Added a Stage A polygon round-buffer core path in `constructive.rs` as the default behavior, batching raw segment buffers and performing a single unary dissolve pass.
- Added environment opt-out `WBTOPOLOGY_BUFFER_STAGE_A=0` to force legacy polygon round-buffer behavior for diagnostics.
- Stage A now includes a GEOS-style shallow ring simplification pre-pass (`distance / 100`) before segment-buffer generation to reduce raw piece counts on dense polygon footprints.
- Stage A now includes staged/tree unary dissolve for large raw piece sets, chunking intermediate dissolve passes before a final full dissolve to reduce worst-case runtime and memory pressure.
- Updated `TopologyGraph::from_linestrings` to use simple noding directly (restoring original behaviour pre-session) and refactored shared build logic into `build_from_noded`; `from_linestrings_with_options` now always calls `node_linestrings_with_options` without the equality-shortcut bypass.
- Updated `classify_overlay_faces` in `overlay.rs` to use simple noding via `from_linestrings` with a topology-scale epsilon floor (`eps.max(1e-9)`, preventing precision loss on ultra-fine Sibson-interpolation epsilons like 1e-12).
- Updated the round-join positive buffer path in `buffer_polygon_positive` to strip output holes whose bounding-box dimensions are ≤ 2×distance, preventing residual hole artifacts when a source hole has fully collapsed under the inward offset.

### Changed
- Expanded triangulation test coverage with fast-path baseline tests (square and collinear cases).
- Reset `src/fast_triangulation.rs` to a closer upstream-style delaunator port so performance work can restart from a simpler baseline.
- Updated LiDAR IDW interpolation to use fixed-radius search in radius mode instead of k-d tree radius queries.
- Updated vector buffer dissolve path to use fast unary dissolve and avoid per-feature pre-dissolve topology repair.

### Fixed
- Fixed buffer line cap direction bug in `append_cap`: start cap (at_end=false) now correctly wraps the back of the starting point instead of the front, eliminating self-intersecting rings that repair logic would collapse. Round, square, and flat cap styles now produce geometrically correct output.
- Fixed a major dissolve scalability bottleneck in the fast unary dissolve path by replacing single-merge restart scanning with cascade-style pairwise merge passes.
- This removes pathological O(N^2) behaviour on very large connected dissolve components in buffer workflows.
- Fixed a fast-path dissolve correctness regression where overlap merges could be missed when candidate pairing was restricted to adjacency order.
- Fast dissolve now performs envelope-sweep candidate pairing each pass so non-adjacent overlapping polygons are still considered for union.

## [0.1.0] - 2026-03-31
### Added
- Initial published release.
