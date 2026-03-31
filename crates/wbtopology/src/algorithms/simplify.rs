//! Geometry simplification algorithms.

use std::collections::{HashMap, HashSet};

use crate::geom::{Coord, Geometry, LineString, LinearRing, Polygon};
use crate::topology::{intersects, is_simple_linestring, is_valid_polygon, touches};

#[inline]
fn sqr(x: f64) -> f64 {
    x * x
}

#[inline]
fn dist2(a: Coord, b: Coord) -> f64 {
    sqr(a.x - b.x) + sqr(a.y - b.y)
}

fn point_segment_distance2(p: Coord, a: Coord, b: Coord) -> f64 {
    let vx = b.x - a.x;
    let vy = b.y - a.y;
    let len2 = vx * vx + vy * vy;
    if len2 == 0.0 {
        return dist2(p, a);
    }
    let t = ((p.x - a.x) * vx + (p.y - a.y) * vy) / len2;
    let t = t.clamp(0.0, 1.0);
    let proj = Coord::xy(a.x + t * vx, a.y + t * vy);
    dist2(p, proj)
}

fn douglas_peucker_mark(points: &[Coord], first: usize, last: usize, tol2: f64, keep: &mut [bool]) {
    if last <= first + 1 {
        return;
    }

    let a = points[first];
    let b = points[last];
    let mut max_d2 = -1.0;
    let mut idx = first;

    for i in (first + 1)..last {
        let d2 = point_segment_distance2(points[i], a, b);
        if d2 > max_d2 {
            max_d2 = d2;
            idx = i;
        }
    }

    if max_d2 > tol2 {
        keep[idx] = true;
        douglas_peucker_mark(points, first, idx, tol2, keep);
        douglas_peucker_mark(points, idx, last, tol2, keep);
    }
}

/// Douglas-Peucker simplification for a linestring.
pub fn simplify_linestring(ls: &LineString, tolerance: f64) -> LineString {
    if ls.coords.len() <= 2 || tolerance <= 0.0 {
        return ls.clone();
    }

    let n = ls.coords.len();
    let tol2 = tolerance * tolerance;
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[n - 1] = true;

    douglas_peucker_mark(&ls.coords, 0, n - 1, tol2, &mut keep);

    let out: Vec<Coord> = ls
        .coords
        .iter()
        .enumerate()
        .filter_map(|(i, c)| keep[i].then_some(*c))
        .collect();

    LineString::new(out)
}

/// Douglas-Peucker simplification for a linear ring.
///
/// Ensures closure and minimum cardinality for polygon validity.
pub fn simplify_ring(ring: &LinearRing, tolerance: f64) -> LinearRing {
    if ring.coords.len() <= 4 || tolerance <= 0.0 {
        return ring.clone();
    }

    let mut open = ring.coords.clone();
    if open.len() >= 2 && open.first().zip(open.last()).map(|(a, b)| a.xy_eq(b)).unwrap_or(false) {
        open.pop();
    }
    if open.len() < 3 {
        return ring.clone();
    }

    let ls = LineString::new(open);
    let mut simp = simplify_linestring(&ls, tolerance).coords;

    if simp.len() < 3 {
        return ring.clone();
    }

    if simp.first() != simp.last() {
        simp.push(simp[0]);
    }

    // A valid non-degenerate ring needs at least 4 coords including closure.
    if simp.len() < 4 {
        return ring.clone();
    }

    LinearRing::new(simp)
}

/// Simplify polygon rings using Douglas-Peucker.
pub fn simplify_polygon(poly: &Polygon, tolerance: f64) -> Polygon {
    if tolerance <= 0.0 {
        return poly.clone();
    }

    let exterior = simplify_ring(&poly.exterior, tolerance);
    let holes: Vec<LinearRing> = poly
        .holes
        .iter()
        .map(|h| simplify_ring(h, tolerance))
        .filter(|h| h.coords.len() >= 4)
        .collect();

    Polygon::new(exterior, holes)
}

/// Simplify any geometry recursively.
pub fn simplify_geometry(g: &Geometry, tolerance: f64) -> Geometry {
    match g {
        Geometry::Point(_) => g.clone(),
        Geometry::LineString(ls) => Geometry::LineString(simplify_linestring(ls, tolerance)),
        Geometry::Polygon(poly) => Geometry::Polygon(simplify_polygon(poly, tolerance)),
        Geometry::MultiPoint(_) => g.clone(),
        Geometry::MultiLineString(lines) => Geometry::MultiLineString(
            lines
                .iter()
                .map(|ls| simplify_linestring(ls, tolerance))
                .collect(),
        ),
        Geometry::MultiPolygon(polys) => Geometry::MultiPolygon(
            polys
                .iter()
                .map(|p| simplify_polygon(p, tolerance))
                .collect(),
        ),
        Geometry::GeometryCollection(parts) => Geometry::GeometryCollection(
            parts
                .iter()
                .map(|p| simplify_geometry(p, tolerance))
                .collect(),
        ),
    }
}

/// Conservative topology-preserving simplification for a linestring.
///
/// Vertices are removed only when the simplified output remains simple
/// (non-self-intersecting) and the removed vertex stays within `tolerance` of
/// its replacement segment.
pub fn simplify_linestring_topology_preserving(ls: &LineString, tolerance: f64) -> LineString {
    if ls.coords.len() <= 2 || tolerance <= 0.0 {
        return ls.clone();
    }
    if !is_simple_linestring(ls) {
        return ls.clone();
    }

    let tol2 = tolerance * tolerance;
    let mut coords = ls.coords.clone();

    loop {
        let mut best_idx = None;
        let mut best_d2 = f64::INFINITY;

        for i in 1..(coords.len() - 1) {
            let d2 = point_segment_distance2(coords[i], coords[i - 1], coords[i + 1]);
            if d2 > tol2 || d2 >= best_d2 {
                continue;
            }

            let candidate = remove_open_vertex(&coords, i);
            let candidate_ls = LineString::new(candidate.clone());
            if is_simple_linestring(&candidate_ls) {
                best_idx = Some(i);
                best_d2 = d2;
            }
        }

        let Some(i) = best_idx else {
            break;
        };
        coords = remove_open_vertex(&coords, i);
    }

    LineString::new(coords)
}

/// Conservative topology-preserving simplification for a linear ring.
///
/// Vertices are removed only when the resulting ring remains closed and simple.
pub fn simplify_ring_topology_preserving(ring: &LinearRing, tolerance: f64) -> LinearRing {
    if ring.coords.len() <= 4 || tolerance <= 0.0 {
        return ring.clone();
    }

    let Some(mut open) = ring_open_coords(&ring.coords) else {
        return ring.clone();
    };
    if open.len() <= 3 {
        return ring.clone();
    }

    let tol2 = tolerance * tolerance;
    loop {
        let mut best_idx = None;
        let mut best_d2 = f64::INFINITY;

        for i in 0..open.len() {
            if open.len() <= 3 {
                break;
            }
            let prev = open[(i + open.len() - 1) % open.len()];
            let curr = open[i];
            let next = open[(i + 1) % open.len()];
            let d2 = point_segment_distance2(curr, prev, next);
            if d2 > tol2 || d2 >= best_d2 {
                continue;
            }

            let candidate_open = remove_ring_vertex_open(&open, i);
            let candidate_ring = LinearRing::new(close_ring(candidate_open.clone()));
            let candidate_ls = LineString::new(candidate_ring.coords.clone());
            if candidate_ring.coords.len() >= 4 && is_simple_linestring(&candidate_ls) {
                best_idx = Some(i);
                best_d2 = d2;
            }
        }

        let Some(i) = best_idx else {
            break;
        };
        open = remove_ring_vertex_open(&open, i);
    }

    LinearRing::new(close_ring(open))
}

/// Conservative topology-preserving simplification for a polygon.
///
/// Vertices are removed only when the candidate polygon remains valid under the
/// crate's polygon validity checks.
pub fn simplify_polygon_topology_preserving(poly: &Polygon, tolerance: f64) -> Polygon {
    if tolerance <= 0.0 {
        return poly.clone();
    }
    if !is_valid_polygon(poly) {
        return poly.clone();
    }

    let tol2 = tolerance * tolerance;
    let mut exterior = match ring_open_coords(&poly.exterior.coords) {
        Some(coords) => coords,
        None => return poly.clone(),
    };
    let mut holes: Vec<Vec<Coord>> = poly
        .holes
        .iter()
        .map(|h| ring_open_coords(&h.coords))
        .collect::<Option<Vec<_>>>()
        .unwrap_or_default();

    loop {
        let mut best = None;
        let mut best_d2 = f64::INFINITY;

        for i in 0..exterior.len() {
            if exterior.len() <= 3 {
                break;
            }
            let prev = exterior[(i + exterior.len() - 1) % exterior.len()];
            let curr = exterior[i];
            let next = exterior[(i + 1) % exterior.len()];
            let d2 = point_segment_distance2(curr, prev, next);
            if d2 > tol2 || d2 >= best_d2 {
                continue;
            }

            let candidate_exterior = remove_ring_vertex_open(&exterior, i);
            let candidate = build_polygon_from_open_rings(&candidate_exterior, &holes);
            if is_valid_polygon(&candidate) {
                best = Some((true, i));
                best_d2 = d2;
            }
        }

        for hole_idx in 0..holes.len() {
            if holes[hole_idx].len() <= 3 {
                continue;
            }
            for i in 0..holes[hole_idx].len() {
                let prev = holes[hole_idx][(i + holes[hole_idx].len() - 1) % holes[hole_idx].len()];
                let curr = holes[hole_idx][i];
                let next = holes[hole_idx][(i + 1) % holes[hole_idx].len()];
                let d2 = point_segment_distance2(curr, prev, next);
                if d2 > tol2 || d2 >= best_d2 {
                    continue;
                }

                let mut candidate_holes = holes.clone();
                candidate_holes[hole_idx] = remove_ring_vertex_open(&candidate_holes[hole_idx], i);
                let candidate = build_polygon_from_open_rings(&exterior, &candidate_holes);
                if is_valid_polygon(&candidate) {
                    best = Some((false, hole_idx * 1_000_000 + i));
                    best_d2 = d2;
                }
            }
        }

        let Some((is_exterior, packed_idx)) = best else {
            break;
        };

        if is_exterior {
            exterior = remove_ring_vertex_open(&exterior, packed_idx);
        } else {
            let hole_idx = packed_idx / 1_000_000;
            let vertex_idx = packed_idx % 1_000_000;
            holes[hole_idx] = remove_ring_vertex_open(&holes[hole_idx], vertex_idx);
        }
    }

    build_polygon_from_open_rings(&exterior, &holes)
}

/// Conservative topology-preserving simplification for any geometry.
pub fn simplify_geometry_topology_preserving(g: &Geometry, tolerance: f64) -> Geometry {
    match g {
        Geometry::Point(_) => g.clone(),
        Geometry::LineString(ls) => {
            Geometry::LineString(simplify_linestring_topology_preserving(ls, tolerance))
        }
        Geometry::Polygon(poly) => {
            Geometry::Polygon(simplify_polygon_topology_preserving(poly, tolerance))
        }
        Geometry::MultiPoint(_) => g.clone(),
        Geometry::MultiLineString(lines) => Geometry::MultiLineString(
            lines
                .iter()
                .map(|ls| simplify_linestring_topology_preserving(ls, tolerance))
                .collect(),
        ),
        Geometry::MultiPolygon(polys) => Geometry::MultiPolygon(
            polys
                .iter()
                .map(|p| simplify_polygon_topology_preserving(p, tolerance))
                .collect(),
        ),
        Geometry::GeometryCollection(parts) => Geometry::GeometryCollection(
            parts
                .iter()
                .map(|p| simplify_geometry_topology_preserving(p, tolerance))
                .collect(),
        ),
    }
}

/// Topology-preserving simplification for an exact shared-boundary polygon coverage.
///
/// This simplifies shared boundary chains once and rebuilds every polygon from
/// the same chain set, preserving edge sharing between adjacent polygons.
///
/// Current scope:
/// - input polygons are assumed to form an exact coverage when they share edges
/// - shared boundaries must already match exactly vertex-for-vertex
/// - acceptance is conservative: candidate changes are kept only when all
///   polygons remain valid and polygon pairs do not develop interior overlap
pub fn simplify_polygon_coverage_topology_preserving(
    polys: &[Polygon],
    tolerance: f64,
) -> Vec<Polygon> {
    if polys.is_empty() || tolerance <= 0.0 {
        return polys.to_vec();
    }
    if polys.iter().any(|poly| !is_valid_polygon(poly)) {
        return polys.to_vec();
    }

    let coverage = match CoverageTopology::build(polys) {
        Some(coverage) => coverage,
        None => return polys.to_vec(),
    };

    let mut chains = coverage.chains.clone();
    for chain_idx in 0..chains.len() {
        let candidate_coords = if chains[chain_idx].is_cycle {
            let ring = LinearRing::new(close_ring(chains[chain_idx].coords.clone()));
            let simplified = simplify_ring_topology_preserving(&ring, tolerance);
            match ring_open_coords(&simplified.coords) {
                Some(coords) => coords,
                None => continue,
            }
        } else {
            simplify_linestring_topology_preserving(
                &LineString::new(chains[chain_idx].coords.clone()),
                tolerance,
            )
            .coords
        };

        if candidate_coords.len() >= chains[chain_idx].coords.len() {
            continue;
        }

        let original_coords = chains[chain_idx].coords.clone();
        chains[chain_idx].coords = candidate_coords;
        let rebuilt = coverage.rebuild_polygons(&chains);
        if coverage_is_valid(&rebuilt) {
            continue;
        }
        chains[chain_idx].coords = original_coords;
    }

    coverage.rebuild_polygons(&chains)
}

fn remove_open_vertex(coords: &[Coord], idx: usize) -> Vec<Coord> {
    coords
        .iter()
        .enumerate()
        .filter_map(|(i, c)| (i != idx).then_some(*c))
        .collect()
}

fn remove_ring_vertex_open(coords: &[Coord], idx: usize) -> Vec<Coord> {
    coords
        .iter()
        .enumerate()
        .filter_map(|(i, c)| (i != idx).then_some(*c))
        .collect()
}

fn ring_open_coords(coords: &[Coord]) -> Option<Vec<Coord>> {
    if coords.len() < 4 || !coords.first()?.xy_eq(coords.last()?) {
        return None;
    }
    let mut open = coords.to_vec();
    open.pop();
    Some(open)
}

fn close_ring(mut coords: Vec<Coord>) -> Vec<Coord> {
    if let Some(first) = coords.first().copied() {
        coords.push(first);
    }
    coords
}

fn build_polygon_from_open_rings(exterior: &[Coord], holes: &[Vec<Coord>]) -> Polygon {
    Polygon::new(
        LinearRing::new(close_ring(exterior.to_vec())),
        holes
            .iter()
            .map(|hole| LinearRing::new(close_ring(hole.clone())))
            .collect(),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct CoordKey(u64, u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct EdgeKey(CoordKey, CoordKey);

#[derive(Debug, Clone)]
struct CoverageChain {
    coords: Vec<Coord>,
    is_cycle: bool,
}

#[derive(Debug, Clone)]
struct RingSpec {
    poly_idx: usize,
    hole_idx: Option<usize>,
    occurrences: Vec<ChainOccurrence>,
}

#[derive(Debug, Clone, Copy)]
struct ChainOccurrence {
    chain_id: usize,
    forward: bool,
}

#[derive(Debug, Clone)]
struct CoverageTopology {
    chains: Vec<CoverageChain>,
    rings: Vec<RingSpec>,
    polygon_count: usize,
}

impl CoverageTopology {
    fn build(polys: &[Polygon]) -> Option<Self> {
        let mut ring_inputs = Vec::<(usize, Option<usize>, Vec<Coord>)>::new();
        for (poly_idx, poly) in polys.iter().enumerate() {
            ring_inputs.push((poly_idx, None, ring_open_coords(&poly.exterior.coords)?));
            for (hole_idx, hole) in poly.holes.iter().enumerate() {
                ring_inputs.push((poly_idx, Some(hole_idx), ring_open_coords(&hole.coords)?));
            }
        }

        let mut coord_lookup = HashMap::<CoordKey, Coord>::new();
        let mut adjacency = HashMap::<CoordKey, Vec<CoordKey>>::new();
        let mut edge_set = HashSet::<EdgeKey>::new();

        for (_, _, ring) in &ring_inputs {
            for &coord in ring {
                coord_lookup.insert(coord_key(coord), coord);
            }
            for i in 0..ring.len() {
                let a = coord_key(ring[i]);
                let b = coord_key(ring[(i + 1) % ring.len()]);
                let edge = edge_key(a, b);
                if edge_set.insert(edge) {
                    adjacency.entry(a).or_default().push(b);
                    adjacency.entry(b).or_default().push(a);
                }
            }
        }

        for neighbors in adjacency.values_mut() {
            neighbors.sort_unstable();
            neighbors.dedup();
        }

        let anchors: HashSet<CoordKey> = adjacency
            .iter()
            .filter_map(|(k, neighbors)| (neighbors.len() != 2).then_some(*k))
            .collect();

        let mut visited = HashSet::<EdgeKey>::new();
        let mut chains = Vec::<CoverageChain>::new();
        let mut edge_to_chain = HashMap::<EdgeKey, usize>::new();

        let mut anchor_keys: Vec<CoordKey> = anchors.iter().copied().collect();
        anchor_keys.sort_unstable();
        for anchor in anchor_keys {
            let Some(neighbors) = adjacency.get(&anchor) else {
                continue;
            };
            for &next in neighbors {
                let edge = edge_key(anchor, next);
                if visited.contains(&edge) {
                    continue;
                }
                let (edge_keys, coords) = trace_open_chain(
                    anchor,
                    next,
                    &adjacency,
                    &mut visited,
                    &coord_lookup,
                    &anchors,
                )?;
                let chain_id = chains.len();
                for edge_key in &edge_keys {
                    edge_to_chain.insert(*edge_key, chain_id);
                }
                chains.push(CoverageChain {
                    coords,
                    is_cycle: false,
                });
            }
        }

        let mut remaining_edges: Vec<EdgeKey> = edge_set.difference(&visited).copied().collect();
        remaining_edges.sort_unstable();
        for edge in remaining_edges {
            if visited.contains(&edge) {
                continue;
            }
            let (edge_keys, coords) = trace_cycle_chain(
                edge.0,
                edge.1,
                &adjacency,
                &mut visited,
                &coord_lookup,
            )?;
            let chain_id = chains.len();
            for edge_key in &edge_keys {
                edge_to_chain.insert(*edge_key, chain_id);
            }
            chains.push(CoverageChain {
                coords,
                is_cycle: true,
            });
        }

        let mut rings = Vec::<RingSpec>::new();
        for (poly_idx, hole_idx, ring) in ring_inputs {
            rings.push(RingSpec {
                poly_idx,
                hole_idx,
                occurrences: decompose_ring_occurrences(&ring, &edge_to_chain, &chains),
            });
        }

        Some(Self {
            chains,
            rings,
            polygon_count: polys.len(),
        })
    }

    fn rebuild_polygons(&self, chains: &[CoverageChain]) -> Vec<Polygon> {
        let mut exteriors = vec![None::<LinearRing>; self.polygon_count];
        let mut holes = vec![Vec::<(usize, LinearRing)>::new(); self.polygon_count];

        for ring in &self.rings {
            let rebuilt = rebuild_ring_from_occurrences(&ring.occurrences, chains);
            match ring.hole_idx {
                None => exteriors[ring.poly_idx] = Some(rebuilt),
                Some(hole_idx) => holes[ring.poly_idx].push((hole_idx, rebuilt)),
            }
        }

        (0..self.polygon_count)
            .map(|poly_idx| {
                holes[poly_idx].sort_by_key(|(hole_idx, _)| *hole_idx);
                Polygon::new(
                    exteriors[poly_idx].clone().unwrap_or_else(|| LinearRing::new(vec![])),
                    holes[poly_idx]
                        .iter()
                        .map(|(_, ring)| ring.clone())
                        .collect(),
                )
            })
            .collect()
    }
}

fn coord_key(coord: Coord) -> CoordKey {
    CoordKey(coord.x.to_bits(), coord.y.to_bits())
}

fn edge_key(a: CoordKey, b: CoordKey) -> EdgeKey {
    if a <= b {
        EdgeKey(a, b)
    } else {
        EdgeKey(b, a)
    }
}

fn trace_open_chain(
    start: CoordKey,
    next: CoordKey,
    adjacency: &HashMap<CoordKey, Vec<CoordKey>>,
    visited: &mut HashSet<EdgeKey>,
    coord_lookup: &HashMap<CoordKey, Coord>,
    anchors: &HashSet<CoordKey>,
) -> Option<(Vec<EdgeKey>, Vec<Coord>)> {
    let mut edge_keys = Vec::<EdgeKey>::new();
    let mut coord_keys = vec![start, next];
    let mut prev = start;
    let mut current = next;
    visited.insert(edge_key(start, next));
    edge_keys.push(edge_key(start, next));

    loop {
        if current != start && anchors.contains(&current) {
            break;
        }
        let neighbors = adjacency.get(&current)?;
        let candidate = neighbors.iter().copied().find(|&neighbor| {
            neighbor != prev && !visited.contains(&edge_key(current, neighbor))
        });
        let Some(next_key) = candidate else {
            break;
        };
        let edge = edge_key(current, next_key);
        visited.insert(edge);
        edge_keys.push(edge);
        coord_keys.push(next_key);
        prev = current;
        current = next_key;
    }

    Some((
        edge_keys,
        coord_keys
            .into_iter()
            .map(|key| *coord_lookup.get(&key).unwrap())
            .collect(),
    ))
}

fn trace_cycle_chain(
    start: CoordKey,
    next: CoordKey,
    adjacency: &HashMap<CoordKey, Vec<CoordKey>>,
    visited: &mut HashSet<EdgeKey>,
    coord_lookup: &HashMap<CoordKey, Coord>,
) -> Option<(Vec<EdgeKey>, Vec<Coord>)> {
    let mut edge_keys = vec![edge_key(start, next)];
    let mut coord_keys = vec![start, next];
    visited.insert(edge_key(start, next));
    let mut prev = start;
    let mut current = next;

    loop {
        let neighbors = adjacency.get(&current)?;
        let candidate = neighbors.iter().copied().find(|&neighbor| {
            neighbor != prev && !visited.contains(&edge_key(current, neighbor))
        });
        let Some(next_key) = candidate else {
            break;
        };
        let edge = edge_key(current, next_key);
        visited.insert(edge);
        edge_keys.push(edge);
        if next_key == start {
            break;
        }
        coord_keys.push(next_key);
        prev = current;
        current = next_key;
    }

    Some((
        edge_keys,
        coord_keys
            .into_iter()
            .map(|key| *coord_lookup.get(&key).unwrap())
            .collect(),
    ))
}

fn decompose_ring_occurrences(
    ring: &[Coord],
    edge_to_chain: &HashMap<EdgeKey, usize>,
    chains: &[CoverageChain],
) -> Vec<ChainOccurrence> {
    let edge_count = ring.len();
    if edge_count == 0 {
        return vec![];
    }

    let chain_ids: Vec<usize> = (0..edge_count)
        .map(|i| edge_to_chain[&edge_key(coord_key(ring[i]), coord_key(ring[(i + 1) % edge_count]))])
        .collect();

    let all_same = chain_ids.iter().all(|chain_id| *chain_id == chain_ids[0]);
    if all_same {
        return vec![ChainOccurrence {
            chain_id: chain_ids[0],
            forward: chain_forward_for_edge(&chains[chain_ids[0]], ring[0], ring[1 % edge_count]),
        }];
    }

    let start = (0..edge_count)
        .find(|&i| chain_ids[i] != chain_ids[(i + edge_count - 1) % edge_count])
        .unwrap_or(0);

    let mut occurrences = Vec::<ChainOccurrence>::new();
    let mut consumed = 0usize;
    while consumed < edge_count {
        let idx = (start + consumed) % edge_count;
        let chain_id = chain_ids[idx];
        occurrences.push(ChainOccurrence {
            chain_id,
            forward: chain_forward_for_edge(&chains[chain_id], ring[idx], ring[(idx + 1) % edge_count]),
        });
        consumed += 1;
        while consumed < edge_count && chain_ids[(start + consumed) % edge_count] == chain_id {
            consumed += 1;
        }
    }

    occurrences
}

fn chain_forward_for_edge(chain: &CoverageChain, a: Coord, b: Coord) -> bool {
    if chain.coords.len() < 2 {
        return true;
    }
    if !chain.is_cycle {
        return chain.coords[0] == a && chain.coords[1] == b;
    }

    for i in 0..chain.coords.len() {
        let next = (i + 1) % chain.coords.len();
        if chain.coords[i] == a && chain.coords[next] == b {
            return true;
        }
        if chain.coords[i] == b && chain.coords[next] == a {
            return false;
        }
    }
    true
}

fn rebuild_ring_from_occurrences(
    occurrences: &[ChainOccurrence],
    chains: &[CoverageChain],
) -> LinearRing {
    if occurrences.is_empty() {
        return LinearRing::new(vec![]);
    }

    let mut coords = Vec::<Coord>::new();
    for occurrence in occurrences {
        let mut chain_coords = chains[occurrence.chain_id].coords.clone();
        if !occurrence.forward {
            chain_coords.reverse();
        }
        if coords.is_empty() {
            coords.extend(chain_coords);
        } else {
            coords.extend(chain_coords.into_iter().skip(1));
        }
    }
    LinearRing::new(coords)
}

fn coverage_is_valid(polys: &[Polygon]) -> bool {
    if polys.iter().any(|poly| !is_valid_polygon(poly)) {
        return false;
    }

    for i in 0..polys.len() {
        for j in (i + 1)..polys.len() {
            let a = Geometry::Polygon(polys[i].clone());
            let b = Geometry::Polygon(polys[j].clone());
            if intersects(&a, &b) && !touches(&a, &b) {
                return false;
            }
        }
    }

    true
}
