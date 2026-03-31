//! Voronoi diagram construction from Delaunay triangulation.

#[cfg(feature = "parallel")]
use rayon::prelude::*;

use crate::geom::{Coord, Envelope, LinearRing, Polygon};
use crate::precision::PrecisionModel;
use crate::triangulation::delaunay_triangulation;

#[cfg(feature = "parallel")]
const PARALLEL_MIN_SITES: usize = 2048;

/// Configuration options for Voronoi diagram generation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoronoiOptions {
    /// Epsilon used by triangulation and clipping tolerances.
    pub epsilon: f64,
    /// Optional precision snapping applied before Voronoi construction.
    pub precision: Option<PrecisionModel>,
    /// Optional clip envelope; when not set, an automatic clip is used.
    pub clip: Option<Envelope>,
}

impl Default for VoronoiOptions {
    fn default() -> Self {
        Self {
            epsilon: 1.0e-9,
            precision: None,
            clip: None,
        }
    }
}

/// Voronoi diagram clipped to an axis-aligned envelope.
#[derive(Debug, Clone, PartialEq)]
pub struct VoronoiDiagram {
    /// Unique Voronoi sites after epsilon-based deduplication.
    pub sites: Vec<Coord>,
    /// Voronoi cell polygons aligned with [`Self::sites`].
    pub cells: Vec<Polygon>,
    /// Clipping envelope used for bounded output.
    pub clip: Envelope,
}

/// Compute a clipped Voronoi diagram from input points.
///
/// The output is clipped to an automatically selected envelope that expands the
/// input extent to ensure bounded cells.
pub fn voronoi_diagram(points: &[Coord], epsilon: f64) -> VoronoiDiagram {
    if points.is_empty() {
        return VoronoiDiagram {
            sites: vec![],
            cells: vec![],
            clip: Envelope::new(0.0, 0.0, 0.0, 0.0),
        };
    }

    let clip = auto_clip_envelope(points);
    voronoi_diagram_with_clip(points, epsilon, clip)
}

/// Compute a clipped Voronoi diagram from points snapped under `precision`.
pub fn voronoi_diagram_with_precision(points: &[Coord], precision: PrecisionModel) -> VoronoiDiagram {
    let mut snapped = points.to_vec();
    precision.apply_coords_in_place(&mut snapped);
    voronoi_diagram(&snapped, precision.epsilon())
}

/// Compute a Voronoi diagram clipped to `clip`.
pub fn voronoi_diagram_with_clip(points: &[Coord], epsilon: f64, clip: Envelope) -> VoronoiDiagram {
    let tri = delaunay_triangulation(points, epsilon);
    let sites = tri.points;

    if sites.is_empty() {
        return VoronoiDiagram {
            sites,
            cells: vec![],
            clip,
        };
    }

    if sites.len() == 1 {
        return VoronoiDiagram {
            sites,
            cells: vec![rect_polygon(clip)],
            clip,
        };
    }

    let neighbors = build_site_neighbors(sites.len(), &tri.triangles);
    #[cfg(feature = "parallel")]
    let cells = if sites.len() >= PARALLEL_MIN_SITES {
        sites
            .par_iter()
            .enumerate()
            .map(|(i, &p)| build_cell_for_site(i, p, &sites, &neighbors, clip, epsilon))
            .collect()
    } else {
        sites
            .iter()
            .enumerate()
            .map(|(i, &p)| build_cell_for_site(i, p, &sites, &neighbors, clip, epsilon))
            .collect()
    };

    #[cfg(not(feature = "parallel"))]
    let cells = sites
        .iter()
        .enumerate()
        .map(|(i, &p)| build_cell_for_site(i, p, &sites, &neighbors, clip, epsilon))
        .collect();

    VoronoiDiagram { sites, cells, clip }
}

/// Compute a Voronoi diagram clipped to `clip` from points snapped under `precision`.
pub fn voronoi_diagram_with_clip_with_precision(
    points: &[Coord],
    precision: PrecisionModel,
    clip: Envelope,
) -> VoronoiDiagram {
    let mut snapped = points.to_vec();
    precision.apply_coords_in_place(&mut snapped);
    voronoi_diagram_with_clip(&snapped, precision.epsilon(), clip)
}

/// Compute a Voronoi diagram using advanced options.
pub fn voronoi_diagram_with_options(points: &[Coord], options: VoronoiOptions) -> VoronoiDiagram {
    let eps = options.epsilon;
    match (options.precision, options.clip) {
        (Some(pm), Some(clip)) => {
            let mut snapped = points.to_vec();
            pm.apply_coords_in_place(&mut snapped);
            voronoi_diagram_with_clip(&snapped, eps.max(pm.epsilon()), clip)
        }
        (Some(pm), None) => {
            let mut snapped = points.to_vec();
            pm.apply_coords_in_place(&mut snapped);
            voronoi_diagram(&snapped, eps.max(pm.epsilon()))
        }
        (None, Some(clip)) => voronoi_diagram_with_clip(points, eps, clip),
        (None, None) => voronoi_diagram(points, eps),
    }
}

fn build_cell_for_site(
    i: usize,
    p: Coord,
    sites: &[Coord],
    neighbors: &[Vec<usize>],
    clip: Envelope,
    epsilon: f64,
) -> Polygon {
    let mut cell = rect_coords(clip);
    let mut scratch = Vec::<Coord>::with_capacity(cell.len() + 8);
    for &j in &neighbors[i] {
        let q = sites[j];
        let nx = q.x - p.x;
        let ny = q.y - p.y;
        if nx == 0.0 && ny == 0.0 {
            continue;
        }
        let c = 0.5 * ((q.x * q.x + q.y * q.y) - (p.x * p.x + p.y * p.y));
        clip_polygon_halfplane_into(&cell, &mut scratch, nx, ny, c, epsilon);
        std::mem::swap(&mut cell, &mut scratch);
        scratch.clear();
        if cell.len() < 3 {
            break;
        }
    }

    if cell.len() < 3 {
        Polygon::new(LinearRing::new(vec![]), vec![])
    } else {
        Polygon::new(LinearRing::new(cell), vec![])
    }
}

fn auto_clip_envelope(points: &[Coord]) -> Envelope {
    let mut min_x = points[0].x;
    let mut min_y = points[0].y;
    let mut max_x = points[0].x;
    let mut max_y = points[0].y;

    for &p in &points[1..] {
        if p.x < min_x {
            min_x = p.x;
        }
        if p.x > max_x {
            max_x = p.x;
        }
        if p.y < min_y {
            min_y = p.y;
        }
        if p.y > max_y {
            max_y = p.y;
        }
    }

    let dx = max_x - min_x;
    let dy = max_y - min_y;
    let pad = dx.max(dy).max(1.0) * 2.0;
    Envelope::new(min_x - pad, min_y - pad, max_x + pad, max_y + pad)
}

fn rect_coords(env: Envelope) -> Vec<Coord> {
    vec![
        Coord::xy(env.min_x, env.min_y),
        Coord::xy(env.max_x, env.min_y),
        Coord::xy(env.max_x, env.max_y),
        Coord::xy(env.min_x, env.max_y),
    ]
}

fn rect_polygon(env: Envelope) -> Polygon {
    Polygon::new(LinearRing::new(rect_coords(env)), vec![])
}

fn build_site_neighbors(site_count: usize, triangles: &[[usize; 3]]) -> Vec<Vec<usize>> {
    let mut neighbors = vec![Vec::<usize>::new(); site_count];
    for &t in triangles {
        let a = t[0];
        let b = t[1];
        let c = t[2];

        if a < site_count && b < site_count {
            neighbors[a].push(b);
            neighbors[b].push(a);
        }
        if b < site_count && c < site_count {
            neighbors[b].push(c);
            neighbors[c].push(b);
        }
        if c < site_count && a < site_count {
            neighbors[c].push(a);
            neighbors[a].push(c);
        }
    }

    // Deduplicate per-site neighbor lists with an O(total_neighbors) stamp pass.
    let mut seen = vec![0u32; site_count];
    let mut stamp = 1u32;
    for nbrs in &mut neighbors {
        if nbrs.len() <= 1 {
            if stamp == u32::MAX {
                seen.fill(0);
                stamp = 1;
            }
            stamp = stamp.wrapping_add(1);
            continue;
        }

        if stamp == u32::MAX {
            seen.fill(0);
            stamp = 1;
        }

        let mut write = 0usize;
        for read in 0..nbrs.len() {
            let id = nbrs[read];
            if id >= site_count {
                continue;
            }
            if seen[id] == stamp {
                continue;
            }
            seen[id] = stamp;
            nbrs[write] = id;
            write += 1;
        }
        nbrs.truncate(write);
        stamp = stamp.wrapping_add(1);
    }

    neighbors
}

fn clip_polygon_halfplane_into(
    poly: &[Coord],
    out: &mut Vec<Coord>,
    nx: f64,
    ny: f64,
    c: f64,
    eps: f64,
) {
    if poly.len() < 3 {
        out.clear();
        return;
    }

    out.clear();
    out.reserve(poly.len() + 4);
    let n = poly.len();

    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        let da = nx * a.x + ny * a.y - c;
        let db = nx * b.x + ny * b.y - c;
        let a_in = da <= eps;
        let b_in = db <= eps;

        if a_in && b_in {
            out.push(b);
        } else if a_in && !b_in {
            if let Some(x) = segment_line_intersection(a, b, nx, ny, c, eps) {
                out.push(x);
            }
        } else if !a_in && b_in {
            if let Some(x) = segment_line_intersection(a, b, nx, ny, c, eps) {
                out.push(x);
            }
            out.push(b);
        }
    }

    dedup_consecutive(out, eps);
}

fn segment_line_intersection(a: Coord, b: Coord, nx: f64, ny: f64, c: f64, eps: f64) -> Option<Coord> {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let denom = nx * dx + ny * dy;
    if denom.abs() <= eps.abs().max(1.0e-12) {
        return None;
    }

    let t = (c - (nx * a.x + ny * a.y)) / denom;
    let tc = t.clamp(0.0, 1.0);
    Some(Coord::interpolate_segment(a, b, tc))
}

fn dedup_consecutive(coords: &mut Vec<Coord>, eps: f64) {
    if coords.is_empty() {
        return;
    }

    let tol = eps.abs().max(1.0e-12);
    let mut write = 1usize;
    for read in 1..coords.len() {
        let p = coords[read];
        let q = coords[write - 1];
        if (p.x - q.x).abs() <= tol && (p.y - q.y).abs() <= tol {
            continue;
        }
        coords[write] = p;
        write += 1;
    }
    coords.truncate(write);

    if coords.len() >= 2 {
        let first = coords[0];
        let last = coords[coords.len() - 1];
        if (first.x - last.x).abs() <= tol && (first.y - last.y).abs() <= tol {
            coords.pop();
        }
    }
}
