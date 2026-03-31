# wbtopology

A pure-Rust topology suite inspired by JTS, designed for predictable performance with minimal dependencies and no unsafe code, and intended to serve as the computational geometry engine for [Whitebox](https://www.whiteboxgeo.com).

Optional parallel execution is available via the `parallel` feature (Rayon), intended for large datasets.

- Parallel paths use adaptive size thresholds to avoid small-input scheduling overhead.

## Table of Contents

- [Mission](#mission)
- [The Whitebox Project](#the-whitebox-project)
- [Is wbtopology Only for Whitebox?](#is-wbtopology-only-for-whitebox)
- [What wbtopology Is Not](#what-wbtopology-is-not)
- [Design goals](#design-goals)
- [Current capabilities](#current-capabilities)
- [Installation](#installation)
- [Quick start](#quick-start)
- [wbvector interop](#wbvector-interop)
- [Examples](#examples)
- [Parallel Feature](#parallel-feature)
- [CI Perf Gate](#ci-perf-gate)
- [Performance Optimization Strategy](#performance-optimization-strategy)
- [Extreme Diagnostics](#extreme-diagnostics)
- [Known Limitations](#known-limitations)
- [License](#license)

## Mission

- Provide robust computational geometry and topology operations for Whitebox applications and workflows.
- Implement the core spatial predicates, overlay, buffer, hull, simplification, triangulation, and Voronoi operations needed by geospatial analysis tools.
- Keep all geometry logic in pure Rust with no unsafe code and minimal dependencies.
- Prioritize correctness and predictable behavior for production data pipelines.

## The Whitebox Project

Whitebox is a collection of related open-source geospatial data analysis software. The Whitebox project began in 2009 at the [University of Guelph](https://geg.uoguelph.ca), Canada, developed by [Dr. John Lindsay](https://jblindsay.github.io/ghrg/index.html) a professor of geomatics. Whitebox has long served as Dr. Lindsay's platform for disseminating the output of his geomatics-based research and has developed an extensive worldwide user base. In 2021 Dr. Lindsay and Anthony Francioni founded [Whitebox Geospatial Inc.](https://www.whiteboxgeo.com) in order to ensure the sustainable and ongoing development of this open-source geospatial project. We are currently working on the next iteration of the Whitebox software, **Whitebox Next Gen**. This crate is part of that larger effort.

## Is wbtopology Only for Whitebox?

No. `wbtopology` is developed primarily to support Whitebox, but it is not restricted to Whitebox projects.

- **Whitebox-first**: API and roadmap decisions prioritize Whitebox geospatial processing needs.
- **General-purpose**: the crate is usable as a standalone computational geometry library in other Rust geospatial applications.
- **JTS-inspired**: the API and geometry model are inspired by JTS, making it familiar to users coming from Java GIS tooling.

## What wbtopology Is Not

`wbtopology` is a computational geometry engine. It is **not** a full GIS framework.

- Not a format I/O library (file reading/writing belongs in `wbvector`).
- Not a rendering or visualization engine.
- Not a coordinate reference system engine (CRS and projection belong in `wbprojection`).
- Not a full JTS/GEOS replacement; coverage is broad but some less-common predicates, graph operations, or topology validation methods may be absent.

## Design goals

- Pure Rust only
- No unsafe code
- Keep dependencies minimal
- High performance for both interactive and batch-scale workloads
- Fast core predicates and topology checks
- Interoperability with wbvector for vector read/write workflows

## Current capabilities

- Core geometry model:
  - `Coord` with `x`, `y`, and optional `z`
  - `Point`, `LineString`, `Polygon`
  - `MultiPoint`, `MultiLineString`, `MultiPolygon`, `GeometryCollection`
- Predicates:
  - `intersects`, `contains`, `within`, `touches`, `crosses`, `overlaps`
  - `covers`, `covered_by`, `disjoint`
  - precision-aware and epsilon-aware variants for all major predicates
- Measurement and proximity:
  - `geometry_area`, `geometry_length`, `geometry_centroid`
  - `geometry_distance`, `nearest_points`, `is_within_distance`
- Hulls:
  - `convex_hull`, `convex_hull_geometry`
  - `concave_hull`, `concave_hull_geometry`
  - precision-aware hull wrappers: `*_with_precision`
  - advanced concave hull tuning via `ConcaveHullOptions`
  - scale-free concavity control via `relative_edge_length_ratio`
  - selectable concave hull backend via `ConcaveHullEngine` (`Delaunay` or `FastRefine`)
- Affine transforms:
  - `translate`, `scale`, `rotate`
- Simplification:
  - Douglas-Peucker: `simplify_linestring`, `simplify_ring`, `simplify_polygon`, `simplify_geometry`
  - conservative topology-preserving variants: `simplify_linestring_topology_preserving`, `simplify_ring_topology_preserving`, `simplify_polygon_topology_preserving`, `simplify_geometry_topology_preserving`
  - shared-boundary polygon coverage simplification: `simplify_polygon_coverage_topology_preserving`
- Constructive operations:
  - `buffer_point`, `buffer_linestring`, `buffer_polygon`
  - precision-aware buffer wrappers: `*_with_precision`
  - configurable caps/joins via `BufferOptions`
  - hole-aware polygon buffering with collapse/drop handling
  - `make_valid_polygon`, `polygonize_closed_linestrings`
- Prepared geometry:
  - `PreparedPolygon` for repeated fast containment/intersection checks
- Spatial index:
  - `SpatialIndex` with packed STR hierarchy
  - envelope/point/geometry query, `nearest_neighbor`, `nearest_k`
  - mutation APIs: `insert`, `remove`, and `compact`
  - stable-id semantics: `remove` preserves surviving ids; `compact` may reassign ids densely
- Noding and graph foundations:
  - `node_linestrings`
  - `TopologyGraph` construction and face-ring extraction helpers
- Overlay:
  - dissolved outputs: `polygon_intersection`, `polygon_union`, `polygon_difference`, `polygon_sym_diff`
  - one-pass multi-op API: `polygon_overlay_all`
  - face decomposition APIs: `*_faces`
- Triangulation and Voronoi:
  - Delaunay APIs with options/precision/constraints
  - Voronoi APIs with automatic or explicit clipping
- DE-9IM relation matrix:
  - `relate`, `relate_with_precision`, `relate_with_epsilon`
- Interoperability:
  - WKB/WKT: `to_wkb`, `from_wkb`, `to_wkt`, `from_wkt`
  - wbvector layer/file interop via `vector_io`

Notes:

- Topology, predicates, overlay, buffering, hulls, triangulation, and simplification operate in XY; optional Z values are carried on coordinates and preserved where practical.

## Installation

Crates.io dependency:

```toml
[dependencies]
wbtopology = "0.1"
```

Enable the optional `parallel` feature when you want Rayon-backed parallel execution on larger workloads:

```toml
[dependencies]
wbtopology = { version = "0.1", features = ["parallel"] }
```

Local workspace/path dependency:

```toml
[dependencies]
wbtopology = { path = "../wbtopology" }
```

## Quick start

Then import the APIs you need:

```rust
use wbtopology::{contains, intersects, Coord, Geometry, LinearRing, Polygon};

let poly = Geometry::Polygon(Polygon::new(
    LinearRing::new(vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(10.0, 0.0),
        Coord::xy(10.0, 10.0),
        Coord::xy(0.0, 10.0),
    ]),
    vec![],
));

let p = Geometry::Point(Coord::xy(5.0, 5.0));
assert!(contains(&poly, &p));
assert!(intersects(&poly, &p));
```

Multi-geometry + distance example:

```rust
use wbtopology::{
  geometry_distance, intersects, Coord, Geometry, LineString,
};

let g1 = Geometry::MultiPoint(vec![
  Coord::xy(0.0, 0.0),
  Coord::xy(10.0, 0.0),
]);
let g2 = Geometry::LineString(LineString::new(vec![
  Coord::xy(5.0, -2.0),
  Coord::xy(5.0, 2.0),
]));

assert!(!intersects(&g1, &g2));
assert_eq!(geometry_distance(&g1, &g2), 5.0);
```

Prepared polygon query example:

```rust
use wbtopology::{Coord, LinearRing, Polygon, PreparedPolygon};

let poly = Polygon::new(
  LinearRing::new(vec![
    Coord::xy(0.0, 0.0),
    Coord::xy(10.0, 0.0),
    Coord::xy(10.0, 10.0),
    Coord::xy(0.0, 10.0),
  ]),
  vec![],
);

let prepared = PreparedPolygon::new(poly);
assert!(prepared.contains_coord(Coord::xy(5.0, 5.0)));
```

Simplification example:

```rust
use wbtopology::{
  simplify_linestring,
  simplify_polygon_topology_preserving,
  Coord, LineString, LinearRing, Polygon,
};

let ls = LineString::new(vec![
  Coord::xy(0.0, 0.0),
  Coord::xy(1.0, 0.01),
  Coord::xy(2.0, 0.0),
]);
let _dp = simplify_linestring(&ls, 0.05);

let poly = Polygon::new(
  LinearRing::new(vec![
    Coord::xy(0.0, 0.0),
    Coord::xy(4.0, 0.1),
    Coord::xy(8.0, 0.0),
    Coord::xy(8.0, 8.0),
    Coord::xy(0.0, 8.0),
    Coord::xy(0.0, 0.0),
  ]),
  vec![],
);
let _topo = simplify_polygon_topology_preserving(&poly, 0.25);
```

Notes:

- The default `simplify_*` APIs use Douglas-Peucker vertex reduction.
- The `*_topology_preserving` APIs are more conservative and only remove vertices when the result remains simple or polygon-valid under wbtopology's current validity checks.
- `simplify_polygon_coverage_topology_preserving` is the dataset-level option for polygon coverages with exact shared boundaries; it simplifies shared chains once and rebuilds all polygons from the same chain set.

Precision model example:

```rust
use wbtopology::{Coord, PrecisionModel};

let pm = PrecisionModel::Fixed { scale: 100.0 };
let snapped = pm.apply_coord(Coord::xy(1.23456, 7.89012));
assert_eq!(snapped, Coord::xy(1.23, 7.89));
```

Constructive buffer example:

```rust
use wbtopology::{
  buffer_linestring, BufferCapStyle, BufferJoinStyle, BufferOptions,
  Coord, LineString,
};

let ls = LineString::new(vec![
  Coord::xy(0.0, 0.0),
  Coord::xy(5.0, 0.0),
  Coord::xy(5.0, 5.0),
]);

let poly = buffer_linestring(
  &ls,
  1.0,
  BufferOptions {
    cap_style: BufferCapStyle::Round,
    join_style: BufferJoinStyle::Mitre,
    mitre_limit: 3.0,
    ..Default::default()
  },
);

assert!(!poly.exterior.coords.is_empty());
```

Precision-aware buffer example:

```rust
use wbtopology::{
  buffer_point_with_precision, BufferOptions, Coord, PrecisionModel,
};

let pm = PrecisionModel::Fixed { scale: 10.0 }; // 0.1 grid
let poly = buffer_point_with_precision(
  Coord::xy(0.03, 0.07),
  1.0,
  BufferOptions::default(),
  pm,
);

assert!(!poly.exterior.coords.is_empty());
```

Relate matrix example:

```rust
use wbtopology::{relate, Coord, Geometry};

let a = Geometry::Point(Coord::xy(0.0, 0.0));
let b = Geometry::Point(Coord::xy(0.0, 0.0));
let m = relate(&a, &b);
assert_eq!(m.as_str9().len(), 9);
assert!(m.matches("T********"));
```

Precision-aware relate and predicate example:

```rust
use wbtopology::{
  relate_with_epsilon, relate_with_precision, intersects_with_precision,
  Coord, Geometry, PrecisionModel,
};

let pm = PrecisionModel::Fixed { scale: 1000.0 };
let a = Geometry::Point(Coord::xy(1.0004, 2.0004));
let b = Geometry::Point(Coord::xy(1.0005, 2.0005));

assert!(intersects_with_precision(&a, &b, pm));
let m = relate_with_precision(&a, &b, pm);
assert_eq!(m.as_str9().len(), 9);

let m_eps = relate_with_epsilon(&a, &b, 5.0e-4);
assert!(m_eps.matches("0********"));
```

Epsilon-aware predicate example (without snapping):

```rust
use wbtopology::{contains_with_epsilon, intersects_with_epsilon, Coord, Geometry, LinearRing, Polygon};

let a = Geometry::Point(Coord::xy(1.0, 2.0));
let b = Geometry::Point(Coord::xy(1.0 + 1.0e-5, 2.0));
assert!(intersects_with_epsilon(&a, &b, 1.0e-4));

let poly = Geometry::Polygon(Polygon::new(
  LinearRing::new(vec![
    Coord::xy(0.0, 0.0),
    Coord::xy(1.0, 0.0),
    Coord::xy(1.0, 1.0),
    Coord::xy(0.0, 1.0),
  ]),
  vec![],
));
let p = Geometry::Point(Coord::xy(1.0 + 5.0e-5, 0.5));
assert!(contains_with_epsilon(&poly, &p, 1.0e-4));
```

WKB/WKT example:

```rust
use wbtopology::{from_wkb, from_wkt, to_wkb, to_wkt, Coord, Geometry};

let g = Geometry::Point(Coord::xy(1.0, 2.0));

let wkt = to_wkt(&g);
let _parsed_wkt = from_wkt(&wkt).unwrap();

let wkb = to_wkb(&g);
let _parsed_wkb = from_wkb(&wkb).unwrap();
```

Spatial index example:

```rust
use wbtopology::{Coord, Geometry, SpatialIndex};

let geoms = vec![
  Geometry::Point(Coord::xy(0.0, 0.0)),
  Geometry::Point(Coord::xy(5.0, 5.0)),
];

let idx = SpatialIndex::from_geometries(&geoms);
let hits = idx.query_point(Coord::xy(0.0, 0.0));
assert_eq!(hits.len(), 1);
```

Spatial index mutation and id-semantics example:

```rust
use wbtopology::{Coord, Geometry, SpatialIndex};

let geoms = vec![
  Geometry::Point(Coord::xy(0.0, 0.0)), // id 0
  Geometry::Point(Coord::xy(5.0, 0.0)), // id 1
  Geometry::Point(Coord::xy(9.0, 0.0)), // id 2
];
let mut idx = SpatialIndex::from_geometries(&geoms);

assert!(idx.remove(1));
assert!(idx.get(1).is_none());

// `remove` keeps surviving ids stable.
let ids_after_remove: Vec<usize> = idx.all_entries().map(|e| e.id).collect();
assert_eq!(ids_after_remove, vec![0, 2]);

// `compact` reassigns ids densely and may invalidate external id handles.
idx.compact();
let ids_after_compact: Vec<usize> = idx.all_entries().map(|e| e.id).collect();
assert_eq!(ids_after_compact, vec![0, 1]);
```

Hull example:

```rust
use wbtopology::{concave_hull, convex_hull, Coord, Geometry};

let pts = vec![
  Coord::xy(0.0, 0.0),
  Coord::xy(4.0, 0.0),
  Coord::xy(4.0, 4.0),
  Coord::xy(0.0, 4.0),
  Coord::xy(2.0, 2.0),
];

let convex = convex_hull(&pts, 1.0e-12);
assert!(matches!(convex, Geometry::Polygon(_)));

let concave = concave_hull(&pts, 3.0, 1.0e-12);
assert!(matches!(concave, Geometry::Polygon(_) | Geometry::MultiPolygon(_)));
```

Advanced concave hull options example:

```rust
use wbtopology::{concave_hull_with_options, ConcaveHullOptions, Coord, Geometry};

let pts = vec![
  Coord::xy(0.0, 0.0),
  Coord::xy(2.0, 0.0),
  Coord::xy(4.0, 0.0),
  Coord::xy(4.0, 2.0),
  Coord::xy(4.0, 4.0),
  Coord::xy(2.0, 4.0),
  Coord::xy(0.0, 4.0),
  Coord::xy(0.0, 2.0),
  Coord::xy(2.0, 2.0),
];

let hull = concave_hull_with_options(
  &pts,
  ConcaveHullOptions {
    max_edge_length: 3.1,
    allow_disjoint: false,
    min_area: 0.5,
    ..Default::default()
  },
);

assert!(matches!(hull, Geometry::Polygon(_) | Geometry::MultiPolygon(_)));
```

Relative concavity example:

```rust
use wbtopology::{concave_hull_with_options, ConcaveHullOptions, Coord};

let pts = vec![
  Coord::xy(0.0, 0.0),
  Coord::xy(2.0, 0.0),
  Coord::xy(4.0, 0.0),
  Coord::xy(4.0, 2.0),
  Coord::xy(4.0, 4.0),
  Coord::xy(2.0, 4.0),
  Coord::xy(0.0, 4.0),
  Coord::xy(0.0, 2.0),
  Coord::xy(2.0, 2.0),
];

let _hull = concave_hull_with_options(
  &pts,
  ConcaveHullOptions {
    relative_edge_length_ratio: Some(0.35),
    ..Default::default()
  },
);
```

Concave hull engine selection example:

```rust
use wbtopology::{concave_hull_with_options, ConcaveHullEngine, ConcaveHullOptions, Coord};

let pts = vec![
  Coord::xy(0.0, 0.0),
  Coord::xy(2.0, 0.0),
  Coord::xy(4.0, 0.0),
  Coord::xy(4.0, 2.0),
  Coord::xy(4.0, 4.0),
  Coord::xy(2.0, 4.0),
  Coord::xy(0.0, 4.0),
  Coord::xy(0.0, 2.0),
  Coord::xy(2.0, 2.0),
];

let _fast_hull = concave_hull_with_options(
  &pts,
  ConcaveHullOptions {
    engine: ConcaveHullEngine::FastRefine,
    max_edge_length: 3.0,
    ..Default::default()
  },
);
```

Polygon erosion multi-component example:

```rust
use wbtopology::{buffer_polygon_multi, BufferOptions, Coord, LinearRing, Polygon};

let poly = Polygon::new(
  LinearRing::new(vec![
    Coord::xy(0.0, 0.0),
    Coord::xy(10.0, 0.0),
    Coord::xy(10.0, 10.0),
    Coord::xy(0.0, 10.0),
  ]),
  vec![],
);

let comps = buffer_polygon_multi(&poly, -1.0, BufferOptions::default());
assert!(!comps.is_empty());
```

## wbvector interop

```rust,no_run
use wbtopology::vector_io;

let geoms = vector_io::read_geometries("input.geojson")?;
vector_io::write_geometries("output.gpkg", &geoms)?;
# Ok::<(), wbtopology::TopologyError>(())
```

## Examples

The repository ships focused examples for interop, benchmarking, and perf diffing.

Core examples:

- `cargo run --example vector_interop`

Criterion benches:

- `cargo bench --bench spatial_index_bench`
- `cargo bench --bench hull_bench`

Maintainer benchmark and diff utilities:

- Internal benchmark utility sources now live under `dev/examples/internal/` and are excluded from the published crate package.
- Use the `dev/scripts/overlay_perf_gate.sh` helper and `dev/perf/` baselines for repository-local perf gating workflows.

Notes:

- `vector_interop` demonstrates wbvector-backed file round-tripping through wbtopology.
- Internal benchmark utilities (noding/overlay/triangulation/voronoi) remain available in the repository under `dev/examples/internal/` for maintainer use.
- Overlay CSV rows are `case,operation,iters,total_us,avg_us,repeats,min_avg_us,max_avg_us`.
- Triangulation CSV appends `triangles,input_points`.
- Voronoi CSV appends `cells,total_vertices`.
- `all_ops_speedup_x` rows in overlay CSV report `all_ops_separate / all_ops_onepass`.
- Diff examples compare median runtime columns and exit with code 1 when regressions exceed configured thresholds.
- Threshold precedence is `case-op` over `case` over `op` over positional default.
- Quote wildcard selectors in shells like zsh, for example `'nc_*=9'` or `'*:union=7'`.

## Parallel Feature

- Enable parallel execution (Rayon): `cargo test --features parallel`
- Run perf gating workflow (including parallel runs as configured): `bash dev/scripts/overlay_perf_gate.sh`

The `parallel` feature is opt-in and keeps the default dependency footprint small.

Current parallel paths focus on larger workloads where scheduling overhead is worth it:

- noding candidate/intersection work
- overlay face-selection / classification hotspots
- Voronoi cell construction for larger site sets

Small workloads are intentionally left on lower-overhead serial paths.

## CI Perf Gate

Use [dev/scripts/overlay_perf_gate.sh](dev/scripts/overlay_perf_gate.sh) to generate benchmark snapshots and run threshold-gated perf comparisons.

Targets:

- `overlay` (default)
- `voronoi`
- `triangulation`
- `all`

Bootstrap a baseline snapshot (first run):

- `dev/scripts/overlay_perf_gate.sh --bootstrap-baseline --repeats 7`
- `dev/scripts/overlay_perf_gate.sh --target voronoi --bootstrap-baseline --repeats 5`
- `dev/scripts/overlay_perf_gate.sh --target triangulation --bootstrap-baseline --repeats 5`
- `dev/scripts/overlay_perf_gate.sh --target all --bootstrap-baseline --repeats 5`

Run the gate against baseline with threshold policy:

- `dev/scripts/overlay_perf_gate.sh --default-threshold 5.0 --op-threshold intersection=8 --case-threshold 'nc_*=9' --case-op-threshold 'nc_complex:intersection=12'`
- `dev/scripts/overlay_perf_gate.sh --target voronoi --default-threshold 5.0 --case-threshold 'uniform_*=8' --case-threshold 'clustered_*=10'`
- `dev/scripts/overlay_perf_gate.sh --target triangulation --default-threshold 5.0 --case-threshold 'uniform_*=8' --case-threshold 'clustered_*=10'`
- `dev/scripts/overlay_perf_gate.sh --target all --default-threshold 5.0`
- `dev/scripts/overlay_perf_gate.sh --target all --overlay-default-threshold 20 --voronoi-default-threshold 5 --triangulation-default-threshold 8`

Compare parallel mode directly against serial mode:

- `dev/scripts/overlay_perf_gate.sh --compare-serial-parallel --repeats 7 --default-threshold 5.0`
- `dev/scripts/overlay_perf_gate.sh --compare-serial-parallel --repeats 7 --op-threshold union=8 --case-threshold 'nc_*=10'`
- `dev/scripts/overlay_perf_gate.sh --target voronoi --compare-serial-parallel --repeats 5 --default-threshold 5.0`
- `dev/scripts/overlay_perf_gate.sh --target triangulation --compare-serial-parallel --repeats 5 --default-threshold 5.0`

For small fixtures, thread scheduling overhead can dominate; prefer higher repeat counts and thresholds focused on medium/complex cases.

Useful options:

- `--baseline <path>` to override baseline path (default `dev/perf/overlay_bench_baseline.csv`)
- `--target <overlay|voronoi|triangulation|all>` to choose benchmark and diff workflow (`all` runs all targets)
- `--overlay-default-threshold <pct>` to override default threshold for overlay when using `--target all`
- `--voronoi-default-threshold <pct>` to override default threshold for voronoi when using `--target all`
- `--triangulation-default-threshold <pct>` to override default threshold for triangulation when using `--target all`
- `--write-current <path>` to save the generated current snapshot as an artifact
- `--features <list>` to run the benchmark/build under specific cargo features (for example `parallel`)
- `--compare-serial-parallel` to benchmark serial and parallel in one run and gate parallel against serial
- `--skip-build` to skip `cargo check --examples` when build has already been validated

## Performance Optimization Strategy

wbtopology is optimized across algorithm choice, memory behavior, numeric robustness, and
measurement workflows so performance gains are practical and repeatable.

- Adaptive execution paths:
  - per-op and one-pass overlay APIs (`polygon_overlay_all`) with dispatch heuristics that route tiny/hole-rich inputs to lower-overhead paths
  - non-crossing boundary fallback for overlays to avoid unnecessary arrangement construction
  - adaptive parallel thresholds so small workloads remain serial while larger workloads scale across cores
- Candidate pruning and complexity reduction:
  - noding uses grid and sweep-line AABB candidate filtering before exact segment intersection tests
  - selective face classification and dissolved reconstruction keep work proportional to useful geometry
- Allocation and dataflow efficiency:
  - one-pass overlay classification reused across intersection/union/difference/symmetric-difference outputs
  - classification data is stored in compact side arrays to reduce cloning and intermediate churn
- Numeric stability without blanket slowdowns:
  - adaptive roundoff-aware tolerances and uncertainty-triggered high-precision orientation fallback
  - scale-aware noding tolerances for very large coordinate magnitudes
- Parallelism as an opt-in capability:
  - Rayon behind `parallel` feature keeps default dependency footprint minimal
  - high-cost loops in noding and overlay face classification are parallelized when profitable
- Built-in perf observability and gating:
  - `overlay_bench` reports repeated-run medians and spread (`min_avg_us`, `max_avg_us`)
  - speedup telemetry rows (`all_ops_speedup_x`) track separate-vs-one-pass efficiency per fixture
  - `overlay_bench_diff` supports default/op/case/case-op thresholds with wildcard selectors for CI gating
  - [dev/scripts/overlay_perf_gate.sh](dev/scripts/overlay_perf_gate.sh) automates baseline bootstrap, diff checks, and serial-vs-parallel comparisons

## Extreme Diagnostics

An opt-in extreme fixture corpus is available for frontier robustness tracking.
It is intentionally non-blocking for default CI.

- Run diagnostics corpus: `cargo test --test overlay_fixture_extreme_diagnostics_tests -- --ignored --nocapture`
- Enable strict failure mode: `WBTOPOLOGY_EXTREME_STRICT=1 cargo test --test overlay_fixture_extreme_diagnostics_tests -- --ignored --nocapture`

Extreme cases are defined in `tests/fixtures/overlay_invariants_extreme.txt`.

## Known Limitations

- `wbtopology` focuses on XY geometry; Z values are preserved on coordinates but are not used in spatial predicates, overlay, buffering, hulls, or triangulation computations.
- Overlay operations (intersection, union, difference, symmetric difference) operate on single-layer polygon collections; multi-layer cross-join overlays require external loop coordination.
- Buffer output for polygon collections uses per-polygon buffering; dissolving overlapping buffer rings into a single output requires a subsequent union pass.
- Delaunay triangulation and Voronoi use an incremental algorithm; very large point clouds may require batching for memory efficiency.
- Simplification is vertex-reduction only (Douglas-Peucker and topology-preserving variants); curve-fitting or spline generalization is not supported.
- DE-9IM `relate` computations are exact for simple geometry types; complex `GeometryCollection` DE-9IM semantics are approximated.
- File I/O (reading/writing vector files) is out of scope; for that see `wbvector`. The `vector_io` interop module is a convenience bridge, not a full format driver.
- Optional parallelism (`parallel` feature) uses Rayon and adaptive size thresholds; very small workloads on large thread pools may see scheduling overhead rather than speedup.

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
