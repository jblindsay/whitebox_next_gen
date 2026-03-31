//! Noding utilities for linework.
//!
//! Milestone B foundation: this module splits line segments at
//! intersection/junction points to build noded linework for future overlay graph steps.

use crate::algorithms::segment::{point_on_segment_eps, segments_intersect_eps};
use crate::geom::{Coord, LineString};
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy)]
struct SegmentRef {
    a: Coord,
    b: Coord,
}

#[derive(Debug, Clone, Copy)]
struct SegmentAabb {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

#[derive(Debug, Clone, Copy)]
struct GridSpec {
    origin_x: f64,
    origin_y: f64,
    cell_size: f64,
}

#[cfg(feature = "parallel")]
const PARALLEL_MIN_SEGMENTS: usize = 256;
const SWEEP_MIN_SEGMENTS: usize = 512;

/// Split a set of linestrings into noded segment fragments.
///
/// Intersections and endpoint-on-segment junctions are identified under `epsilon`.
/// Each output linestring is a 2-point segment between adjacent node points.
pub fn node_linestrings(lines: &[LineString], epsilon: f64) -> Vec<LineString> {
    let eps = epsilon.abs();
    let segments = collect_segments(lines);
    let candidate_lists = build_candidate_lists(&segments, eps);

    #[cfg(feature = "parallel")]
    {
        if segments.len() >= PARALLEL_MIN_SEGMENTS {
            let parts: Vec<Vec<LineString>> = segments
                .par_iter()
                .copied()
                .enumerate()
                .map(|(i, seg)| node_segment(i, seg, &segments, &candidate_lists[i], eps))
                .collect();
            return parts.into_iter().flatten().collect();
        }

        let mut out = Vec::new();
        for (i, seg) in segments.iter().copied().enumerate() {
            out.extend(node_segment(i, seg, &segments, &candidate_lists[i], eps));
        }
        return out;
    }

    #[cfg(not(feature = "parallel"))]
    {
        let mut out = Vec::new();
        for (i, seg) in segments.iter().copied().enumerate() {
            out.extend(node_segment(i, seg, &segments, &candidate_lists[i], eps));
        }
        out
    }
}

fn node_segment(
    i: usize,
    seg: SegmentRef,
    segments: &[SegmentRef],
    candidates: &[usize],
    eps: f64,
) -> Vec<LineString> {
    let mut split_points = vec![seg.a, seg.b];

    for &j in candidates {
        if i == j {
            continue;
        }
        let other = segments[j];

        if !segments_intersect_eps(seg.a, seg.b, other.a, other.b, eps) {
            continue;
        }

        if point_on_segment_eps(other.a, seg.a, seg.b, eps) {
            push_unique_eps(&mut split_points, other.a, eps);
        }
        if point_on_segment_eps(other.b, seg.a, seg.b, eps) {
            push_unique_eps(&mut split_points, other.b, eps);
        }

        // For collinear overlap, also include this segment's endpoints when they lie
        // on the other segment so overlap boundaries are always materialized.
        if point_on_segment_eps(seg.a, other.a, other.b, eps) {
            push_unique_eps(&mut split_points, seg.a, eps);
        }
        if point_on_segment_eps(seg.b, other.a, other.b, eps) {
            push_unique_eps(&mut split_points, seg.b, eps);
        }

        if let Some(p) = segment_intersection_point(seg.a, seg.b, other.a, other.b, eps) {
            push_unique_eps(&mut split_points, p, eps);
        }
    }

    split_points.sort_by(|p1, p2| {
        let t1 = segment_param(seg.a, seg.b, *p1, eps);
        let t2 = segment_param(seg.a, seg.b, *p2, eps);
        t1.total_cmp(&t2)
    });

    let mut out = Vec::new();
    for w in split_points.windows(2) {
        let p0 = w[0];
        let p1 = w[1];
        if distance2(p0, p1) > eps * eps {
            out.push(LineString::new(vec![p0, p1]));
        }
    }
    out
}

fn build_candidate_lists(segments: &[SegmentRef], eps: f64) -> Vec<Vec<usize>> {
    let n = segments.len();
    if n == 0 {
        return Vec::new();
    }

    // Small inputs are usually faster with direct pairwise scans.
    if n < 128 {
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let mut v = Vec::with_capacity(n.saturating_sub(1));
            for j in 0..n {
                if i != j {
                    v.push(j);
                }
            }
            out.push(v);
        }
        return out;
    }

    let bboxes = collect_segment_bboxes(segments, eps);

    // For larger datasets, a simple x-sweep often avoids grid hash overhead while
    // still generating exact AABB-overlap candidates.
    if n >= SWEEP_MIN_SEGMENTS {
        return build_candidate_lists_sweep(&bboxes);
    }

    build_candidate_lists_grid(&bboxes, eps)
}

fn build_candidate_lists_sweep(bboxes: &[SegmentAabb]) -> Vec<Vec<usize>> {
    let n = bboxes.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| bboxes[i].min_x.total_cmp(&bboxes[j].min_x));

    let mut out = vec![Vec::<usize>::new(); n];
    let mut active: Vec<usize> = Vec::new();

    for &i in &order {
        let bb_i = bboxes[i];

        // Remove segments whose x-extent is fully left of current segment.
        active.retain(|&j| bboxes[j].max_x >= bb_i.min_x);

        for &j in &active {
            let bb_j = bboxes[j];
            if bb_i.max_y < bb_j.min_y || bb_j.max_y < bb_i.min_y {
                continue;
            }
            out[i].push(j);
            out[j].push(i);
        }

        active.push(i);
    }

    for v in &mut out {
        v.sort_unstable();
        v.dedup();
    }

    out
}

fn build_candidate_lists_grid(bboxes: &[SegmentAabb], eps: f64) -> Vec<Vec<usize>> {
    let n = bboxes.len();
    let grid = build_grid_spec(&bboxes, eps);

    let mut cell_map: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    let mut spill_segments: Vec<usize> = Vec::new();
    let mut is_spill = vec![false; n];

    // Very large cell coverage can create too many buckets for a single segment.
    const MAX_CELLS_PER_SEGMENT: i64 = 4096;

    for (idx, bb) in bboxes.iter().enumerate() {
        let (ix0, iy0, ix1, iy1) = bbox_cell_range(*bb, grid);
        let nx = ix1 - ix0 + 1;
        let ny = iy1 - iy0 + 1;
        if nx * ny > MAX_CELLS_PER_SEGMENT {
            spill_segments.push(idx);
            is_spill[idx] = true;
            continue;
        }

        for ix in ix0..=ix1 {
            for iy in iy0..=iy1 {
                cell_map.entry((ix, iy)).or_default().push(idx);
            }
        }
    }

    let mut out = vec![Vec::<usize>::new(); n];
    for i in 0..n {
        if is_spill[i] {
            let mut all = Vec::with_capacity(n.saturating_sub(1));
            for j in 0..n {
                if i != j {
                    all.push(j);
                }
            }
            out[i] = all;
            continue;
        }

        let bb = bboxes[i];
        let (ix0, iy0, ix1, iy1) = bbox_cell_range(bb, grid);
        let mut seen: HashSet<usize> = HashSet::new();

        for &j in &spill_segments {
            if j != i {
                seen.insert(j);
            }
        }

        for ix in ix0..=ix1 {
            for iy in iy0..=iy1 {
                if let Some(indices) = cell_map.get(&(ix, iy)) {
                    for &j in indices {
                        if j != i {
                            seen.insert(j);
                        }
                    }
                }
            }
        }

        let mut candidates: Vec<usize> = seen.into_iter().collect();
        candidates.sort_unstable();
        out[i] = candidates;
    }

    out
}

fn collect_segment_bboxes(segments: &[SegmentRef], eps: f64) -> Vec<SegmentAabb> {
    segments
        .iter()
        .map(|s| SegmentAabb {
            min_x: s.a.x.min(s.b.x) - eps,
            min_y: s.a.y.min(s.b.y) - eps,
            max_x: s.a.x.max(s.b.x) + eps,
            max_y: s.a.y.max(s.b.y) + eps,
        })
        .collect()
}

fn build_grid_spec(bboxes: &[SegmentAabb], eps: f64) -> GridSpec {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    for bb in bboxes {
        min_x = min_x.min(bb.min_x);
        min_y = min_y.min(bb.min_y);
    }

    let mut sum_len = 0.0;
    for bb in bboxes {
        let dx = (bb.max_x - bb.min_x).abs();
        let dy = (bb.max_y - bb.min_y).abs();
        sum_len += (dx * dx + dy * dy).sqrt();
    }

    let avg_len = if bboxes.is_empty() {
        1.0
    } else {
        sum_len / bboxes.len() as f64
    };

    GridSpec {
        origin_x: min_x,
        origin_y: min_y,
        cell_size: avg_len.max(eps * 8.0).max(1.0e-9),
    }
}

fn cell_index(v: f64, origin: f64, cell: f64) -> i64 {
    ((v - origin) / cell).floor() as i64
}

fn bbox_cell_range(bb: SegmentAabb, grid: GridSpec) -> (i64, i64, i64, i64) {
    (
        cell_index(bb.min_x, grid.origin_x, grid.cell_size),
        cell_index(bb.min_y, grid.origin_y, grid.cell_size),
        cell_index(bb.max_x, grid.origin_x, grid.cell_size),
        cell_index(bb.max_y, grid.origin_y, grid.cell_size),
    )
}

fn collect_segments(lines: &[LineString]) -> Vec<SegmentRef> {
    let mut out = Vec::new();
    for ls in lines {
        if ls.coords.len() < 2 {
            continue;
        }
        for i in 0..(ls.coords.len() - 1) {
            out.push(SegmentRef {
                a: ls.coords[i],
                b: ls.coords[i + 1],
            });
        }
    }
    out
}

fn push_unique_eps(points: &mut Vec<Coord>, p: Coord, eps: f64) {
    if points.iter().any(|q| nearly_eq_coord(*q, p, eps)) {
        return;
    }
    points.push(p);
}

fn nearly_eq_coord(a: Coord, b: Coord, eps: f64) -> bool {
    (a.x - b.x).abs() <= eps && (a.y - b.y).abs() <= eps
}

fn distance2(a: Coord, b: Coord) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

fn segment_param(a: Coord, b: Coord, p: Coord, eps: f64) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    if dx.abs() >= dy.abs() {
        if dx.abs() <= eps {
            0.0
        } else {
            (p.x - a.x) / dx
        }
    } else if dy.abs() <= eps {
        0.0
    } else {
        (p.y - a.y) / dy
    }
}

fn segment_intersection_point(a1: Coord, a2: Coord, b1: Coord, b2: Coord, eps: f64) -> Option<Coord> {
    let r_x = a2.x - a1.x;
    let r_y = a2.y - a1.y;
    let s_x = b2.x - b1.x;
    let s_y = b2.y - b1.y;

    let denom = r_x * s_y - r_y * s_x;
    let denom_roundoff = ((r_x * s_y).abs() + (r_y * s_x).abs()).max(1.0) * 8.0 * f64::EPSILON;
    let len_scale = (r_x.abs() + r_y.abs()).max(s_x.abs() + s_y.abs()).max(1.0);
    let denom_tol = denom_roundoff.max(eps * len_scale);
    if denom.abs() <= denom_tol {
        return None;
    }

    let qmp_x = b1.x - a1.x;
    let qmp_y = b1.y - a1.y;
    let t = (qmp_x * s_y - qmp_y * s_x) / denom;
    let u = (qmp_x * r_y - qmp_y * r_x) / denom;

    if t < -eps || t > 1.0 + eps || u < -eps || u > 1.0 + eps {
        return None;
    }

    Some(Coord::interpolate_segment(a1, a2, t))
}
