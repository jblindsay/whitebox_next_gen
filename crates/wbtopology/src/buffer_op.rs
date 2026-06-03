//! BufferOp orchestration scaffold for global curve/noding/graph buffering.
//!
//! This module provides a restart-safe entry point for the vector `buffer_vector`
//! rewrite. The design mirrors GEOS/JTS BufferOp staging, but keeps behavior
//! conservative while the full face-label pipeline is being completed.

use crate::constructive::{
    buffer_linestring_curve_set, buffer_polygon_curve_set, BufferOptions, PolygonizeOptions,
};
use crate::geom::{LineString, Polygon};
use crate::noding::{node_linestrings_with_options, NodingOptions, NodingStrategy};
use crate::overlay::polygon_unary_union;

/// Configuration for [`BufferOp`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BufferOpOptions {
    /// Buffer style options used during curve generation.
    pub buffer: BufferOptions,
    /// Noding options applied to the global curve set.
    pub noding: NodingOptions,
    /// Predicate epsilon used for polygonization and dissolve.
    pub epsilon: f64,
}

impl Default for BufferOpOptions {
    fn default() -> Self {
        Self {
            buffer: BufferOptions::default(),
            noding: NodingOptions {
                epsilon: 1.0e-9,
                strategy: NodingStrategy::SnapRounding,
                precision: None,
            },
            epsilon: 1.0e-9,
        }
    }
}

/// Stage counters emitted by [`BufferOp::run_linestrings_dissolved`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BufferOpStats {
    /// Number of input source lines.
    pub input_lines: usize,
    /// Number of raw curves generated.
    pub raw_curves: usize,
    /// Number of curves after global noding.
    pub noded_curves: usize,
    /// Number of candidate polygons from polygonization.
    pub candidate_polygons: usize,
    /// Number of dissolved output polygons.
    pub dissolved_polygons: usize,
}

/// Result bundle for staged BufferOp execution.
#[derive(Debug, Clone, PartialEq)]
pub struct BufferOpResult {
    /// Dissolved output polygons.
    pub polygons: Vec<Polygon>,
    /// Stage counters to help diagnose regressions while iterating.
    pub stats: BufferOpStats,
}

/// GEOS-style buffer orchestrator for batched buffering workflows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BufferOp {
    /// Runtime options for all pipeline stages.
    pub options: BufferOpOptions,
}

impl BufferOp {
    /// Create a new `BufferOp` with explicit options.
    pub fn new(options: BufferOpOptions) -> Self {
        Self { options }
    }

    /// Run the global buffering pipeline for line input and return dissolved output.
    ///
    /// Current behavior keeps the staged architecture (curve generation -> global noding
    /// -> polygonization -> dissolve) while face-depth labeling is being integrated.
    ///
    /// This makes the execution flow explicit and testable without changing public
    /// tool semantics in one large rewrite.
    pub fn run_linestrings_dissolved(
        &self,
        lines: &[LineString],
        distance: f64,
    ) -> BufferOpResult {
        let mut stats = BufferOpStats {
            input_lines: lines.len(),
            ..BufferOpStats::default()
        };

        if !distance.is_finite() || distance <= 0.0 || lines.is_empty() {
            return BufferOpResult {
                polygons: Vec::new(),
                stats,
            };
        }

        let curves = self.collect_raw_curves(lines, distance);
        stats.raw_curves = curves.len();
        if curves.is_empty() {
            return BufferOpResult {
                polygons: Vec::new(),
                stats,
            };
        }

        self.dissolve_curve_set(curves, stats)
    }

    /// Run the global buffering pipeline for polygon input and return dissolved output.
    pub fn run_polygons_dissolved(&self, polygons: &[Polygon], distance: f64) -> BufferOpResult {
        let mut stats = BufferOpStats {
            input_lines: polygons.len(),
            ..BufferOpStats::default()
        };

        if !distance.is_finite() || distance <= 0.0 || polygons.is_empty() {
            return BufferOpResult {
                polygons: Vec::new(),
                stats,
            };
        }

        let mut curves = Vec::<LineString>::new();
        for poly in polygons {
            curves.extend(buffer_polygon_curve_set(poly, distance, self.options.buffer));
        }
        stats.raw_curves = curves.len();

        self.dissolve_curve_set(curves, stats)
    }

    fn collect_raw_curves(&self, lines: &[LineString], distance: f64) -> Vec<LineString> {
        let mut curves = Vec::<LineString>::new();
        for line in lines {
            curves.extend(buffer_linestring_curve_set(line, distance, self.options.buffer));
        }
        curves
    }

    fn dissolve_curve_set(&self, curves: Vec<LineString>, mut stats: BufferOpStats) -> BufferOpResult {
        if curves.is_empty() {
            return BufferOpResult {
                polygons: Vec::new(),
                stats,
            };
        }

        let noded = node_linestrings_with_options(&curves, self.options.noding);
        stats.noded_curves = noded.len();
        if noded.is_empty() {
            return BufferOpResult {
                polygons: Vec::new(),
                stats,
            };
        }

        let poly_result = crate::constructive::polygonize_linework(
            &noded,
            PolygonizeOptions {
                epsilon: self.options.epsilon,
                noding: self.options.noding,
            },
        );
        stats.candidate_polygons = poly_result.polygons.len();

        if poly_result.polygons.is_empty() {
            return BufferOpResult {
                polygons: Vec::new(),
                stats,
            };
        }

        let dissolved = polygon_unary_union(&poly_result.polygons, self.options.epsilon);
        stats.dissolved_polygons = dissolved.len();

        BufferOpResult {
            polygons: dissolved,
            stats,
        }
    }
}

impl Default for BufferOp {
    fn default() -> Self {
        Self {
            options: BufferOpOptions::default(),
        }
    }
}
