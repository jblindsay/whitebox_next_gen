#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! # wbtopology
//!
//! A pure-Rust topology suite inspired by JTS.
//! The crate is dependency-light (currently zero external dependencies), avoids
//! unsafe code, and focuses on predictable performance for core geometry
//! predicates and operations.
//!
//! ## Selected APIs
//!
//! - Hulls: [`convex_hull`], [`convex_hull_geometry`], [`concave_hull`],
//!   [`concave_hull_geometry`], precision-aware `*_with_precision` wrappers,
//!   and [`ConcaveHullOptions`] for advanced concave-hull tuning, including
//!   scale-free `relative_edge_length_ratio` control and selectable
//!   [`ConcaveHullEngine`] backends.
//! - Constructive buffering: [`buffer_point`], [`buffer_linestring`], [`buffer_polygon`],
//!   and [`buffer_polygon_multi`] for erosion results that may split into multiple components.
//! - Spatial indexing: [`SpatialIndex`] with packed STR hierarchy, nearest-neighbor,
//!   and top-k nearest (`nearest_k`) lookup.
//!
//! ## `buffer_polygon_multi` Example
//!
//! ```
//! use wbtopology::{buffer_polygon_multi, BufferOptions, Coord, LinearRing, Polygon};
//!
//! let poly = Polygon::new(
//!     LinearRing::new(vec![
//!         Coord::xy(0.0, 0.0),
//!         Coord::xy(10.0, 0.0),
//!         Coord::xy(10.0, 10.0),
//!         Coord::xy(0.0, 10.0),
//!     ]),
//!     vec![],
//! );
//!
//! let parts = buffer_polygon_multi(&poly, -1.0, BufferOptions::default());
//! assert!(!parts.is_empty());
//! ```
//!
//! ## `SpatialIndex` Remove vs Compact Semantics
//!
//! ```
//! use wbtopology::{Coord, Geometry, SpatialIndex};
//!
//! let geoms = vec![
//!     Geometry::Point(Coord::xy(0.0, 0.0)), // id 0
//!     Geometry::Point(Coord::xy(5.0, 0.0)), // id 1
//!     Geometry::Point(Coord::xy(9.0, 0.0)), // id 2
//! ];
//! let mut idx = SpatialIndex::from_geometries(&geoms);
//!
//! idx.remove(1);
//! let ids_after_remove: Vec<usize> = idx.all_entries().map(|e| e.id).collect();
//! assert_eq!(ids_after_remove, vec![0, 2]);
//!
//! // Compaction reassigns dense ids.
//! idx.compact();
//! let ids_after_compact: Vec<usize> = idx.all_entries().map(|e| e.id).collect();
//! assert_eq!(ids_after_compact, vec![0, 1]);
//! ```

pub mod algorithms;
pub mod constructive;
pub mod error;
pub mod geom;
pub mod graph;
pub mod hull;
pub mod io;
pub mod natural_neighbour;
pub mod noding;
pub mod overlay;
pub mod precision;
pub mod relate;
pub mod spatial_index;
pub mod topology;
pub mod triangulation;
pub mod vector_io;
pub mod voronoi;

pub use error::{Result, TopologyError};
pub use constructive::{
	buffer_linestring,
	buffer_linestring_with_precision,
	buffer_polygon,
	buffer_polygon_multi,
	buffer_polygon_with_precision,
	buffer_point,
	buffer_point_with_precision,
	make_valid_polygon,
	polygonize_closed_linestrings,
	BufferCapStyle,
	BufferJoinStyle,
	BufferOptions,
};
pub use algorithms::distance::{coord_dist, geometry_distance, is_within_distance, nearest_points};
pub use algorithms::measurements::{
	geometry_area, geometry_centroid, geometry_length, linestring_length, polygon_area,
	polygon_centroid, ring_signed_area,
};
pub use algorithms::simplify::{
	simplify_geometry,
	simplify_polygon_coverage_topology_preserving,
	simplify_geometry_topology_preserving,
	simplify_linestring,
	simplify_linestring_topology_preserving,
	simplify_polygon,
	simplify_polygon_topology_preserving,
	simplify_ring,
	simplify_ring_topology_preserving,
};
pub use algorithms::transform::{rotate, scale, translate};
pub use geom::{Coord, Envelope, Geometry, LineString, LinearRing, Polygon};
pub use graph::{DirectedEdge, GraphNode, TopologyGraph};
pub use hull::{
	concave_hull,
	concave_hull_geometry,
	concave_hull_geometry_with_options,
	concave_hull_geometry_with_precision,
	concave_hull_with_options,
	concave_hull_with_precision,
	convex_hull,
	convex_hull_geometry,
	convex_hull_geometry_with_precision,
	convex_hull_with_precision,
	ConcaveHullEngine,
	ConcaveHullOptions,
};
pub use io::{from_wkb, from_wkt, to_wkb, to_wkt};
pub use natural_neighbour::PreparedSibsonInterpolator;
pub use noding::node_linestrings;
pub use overlay::{
	polygon_difference,
	polygon_difference_with_precision,
	polygon_difference_faces,
	polygon_intersection,
	polygon_intersection_with_precision,
	polygon_intersection_faces,
	polygon_overlay,
	polygon_overlay_all,
	polygon_overlay_all_with_precision,
	polygon_overlay_with_precision,
	polygon_overlay_faces,
	polygon_sym_diff,
	polygon_sym_diff_with_precision,
	polygon_sym_diff_faces,
	polygon_union,
	polygon_union_with_precision,
	polygon_union_faces,
	polygon_unary_dissolve,
	OverlayOutputs,
	OverlayOp,
	UnaryDissolveGroup,
};
pub use precision::PrecisionModel;
pub use relate::{relate, relate_with_epsilon, relate_with_precision, Location, RelateMatrix};
pub use spatial_index::{IndexedGeometry, SpatialIndex};
pub use topology::{
	contains,
	contains_with_epsilon,
	contains_with_precision,
	covered_by,
	covered_by_with_epsilon,
	covered_by_with_precision,
	covers,
	covers_with_epsilon,
	covers_with_precision,
	crosses,
	crosses_with_epsilon,
	crosses_with_precision,
	disjoint,
	disjoint_with_epsilon,
	disjoint_with_precision,
	intersects,
	intersects_with_epsilon,
	intersects_with_precision,
	is_simple_linestring,
	is_valid_polygon,
	overlaps,
	overlaps_with_epsilon,
	overlaps_with_precision,
	touches,
	touches_with_epsilon,
	touches_with_precision,
	within,
	within_with_epsilon,
	within_with_precision,
	PreparedPolygon,
};
pub use triangulation::{
	delaunay_triangulation,
	delaunay_triangulation_with_constraints,
	delaunay_triangulation_with_options,
	delaunay_triangulation_with_options_checked,
	delaunay_triangulation_with_precision,
	DelaunayTriangulation,
	TriangulationOptions,
};
pub use voronoi::{
	voronoi_diagram,
	voronoi_diagram_with_clip,
	voronoi_diagram_with_clip_with_precision,
	voronoi_diagram_with_options,
	voronoi_diagram_with_precision,
	VoronoiOptions,
	VoronoiDiagram,
};
