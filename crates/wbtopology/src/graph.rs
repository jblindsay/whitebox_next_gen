//! Topology graph utilities built from noded linework.

use std::collections::{HashMap, HashSet};

use crate::geom::{Coord, LineString};
use crate::noding::node_linestrings;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NodeKey {
    Exact(u64, u64),
    Quantized(i64, i64),
}

/// Graph node with coordinate and outgoing directed-edge ids.
#[derive(Debug, Clone)]
pub struct GraphNode {
    /// Node id.
    pub id: usize,
    /// Node coordinate.
    pub coord: Coord,
    /// Directed edges whose `from` is this node.
    pub outgoing: Vec<usize>,
}

/// Directed edge in a half-edge style structure.
#[derive(Debug, Clone)]
pub struct DirectedEdge {
    /// Directed-edge id.
    pub id: usize,
    /// Source node id.
    pub from: usize,
    /// Destination node id.
    pub to: usize,
    /// Twin edge id (opposite direction of the same segment).
    pub sym: usize,
    /// Edge angle from source node, in radians.
    pub angle: f64,
}

/// Lightweight topology graph for noded linework.
#[derive(Debug, Clone)]
pub struct TopologyGraph {
    /// Graph nodes.
    pub nodes: Vec<GraphNode>,
    /// Directed edges.
    pub edges: Vec<DirectedEdge>,
}

impl TopologyGraph {
    /// Build a topology graph from input linestrings.
    ///
    /// Input lines are noded first; each noded segment becomes one undirected graph edge
    /// represented by two directed edges.
    pub fn from_linestrings(lines: &[LineString], epsilon: f64) -> Self {
        let eps = normalized_eps(epsilon);
        let noded = node_linestrings(lines, eps);

        let mut nodes = Vec::<GraphNode>::new();
        let mut edges = Vec::<DirectedEdge>::new();
        let mut node_index = HashMap::<NodeKey, usize>::new();
        let mut segment_keys = HashSet::<(usize, usize)>::new();

        for ls in &noded {
            if ls.coords.len() != 2 {
                continue;
            }

            let a = ls.coords[0];
            let b = ls.coords[1];
            let a_id = get_or_insert_node(a, eps, &mut node_index, &mut nodes);
            let b_id = get_or_insert_node(b, eps, &mut node_index, &mut nodes);

            if a_id == b_id {
                continue;
            }

            let key = if a_id < b_id { (a_id, b_id) } else { (b_id, a_id) };
            if !segment_keys.insert(key) {
                continue;
            }

            let e0_id = edges.len();
            let e1_id = e0_id + 1;

            let e0 = DirectedEdge {
                id: e0_id,
                from: a_id,
                to: b_id,
                sym: e1_id,
                angle: edge_angle(a, b),
            };
            let e1 = DirectedEdge {
                id: e1_id,
                from: b_id,
                to: a_id,
                sym: e0_id,
                angle: edge_angle(b, a),
            };

            nodes[a_id].outgoing.push(e0_id);
            nodes[b_id].outgoing.push(e1_id);

            edges.push(e0);
            edges.push(e1);
        }

        for node in &mut nodes {
            node.outgoing
                .sort_by(|ea, eb| edges[*ea].angle.total_cmp(&edges[*eb].angle));
        }

        Self { nodes, edges }
    }

    /// Number of nodes.
    #[inline]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of directed edges.
    #[inline]
    pub fn directed_edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Number of undirected edges.
    #[inline]
    pub fn edge_count(&self) -> usize {
        self.edges.len() / 2
    }

    /// Find node id for a coordinate under epsilon equality.
    pub fn find_node(&self, p: Coord, epsilon: f64) -> Option<usize> {
        let eps = normalized_eps(epsilon);
        self.nodes
            .iter()
            .find(|n| nearly_eq_coord(n.coord, p, eps))
            .map(|n| n.id)
    }

    /// Find a directed edge by approximate from/to coordinates.
    pub fn find_directed_edge(&self, from: Coord, to: Coord, epsilon: f64) -> Option<usize> {
        let eps = normalized_eps(epsilon);
        self.edges
            .iter()
            .find(|e| {
                nearly_eq_coord(self.nodes[e.from].coord, from, eps)
                    && nearly_eq_coord(self.nodes[e.to].coord, to, eps)
            })
            .map(|e| e.id)
    }

    /// Next outgoing edge around a node in counterclockwise angular order.
    pub fn next_ccw_around_node(&self, node_id: usize, edge_id: usize) -> Option<usize> {
        let node = self.nodes.get(node_id)?;
        let pos = node.outgoing.iter().position(|eid| *eid == edge_id)?;
        let next = (pos + 1) % node.outgoing.len();
        node.outgoing.get(next).copied()
    }

    /// Face-traversal hook: next edge keeping face on the left.
    ///
    /// Given an incoming directed edge `u -> v`, this method moves to `v` and
    /// returns the outgoing edge selected by taking the predecessor of `v -> u`
    /// in `v`'s CCW-sorted edge list.
    pub fn next_left_face_edge(&self, incoming_edge_id: usize) -> Option<usize> {
        let incoming = self.edges.get(incoming_edge_id)?;
        let at = incoming.to;
        let back = incoming.sym;
        let node = self.nodes.get(at)?;
        if node.outgoing.is_empty() {
            return None;
        }

        let pos = node.outgoing.iter().position(|eid| *eid == back)?;
        let prev = if pos == 0 {
            node.outgoing.len() - 1
        } else {
            pos - 1
        };
        node.outgoing.get(prev).copied()
    }

    /// Extract all left-face cycles as closed rings.
    ///
    /// Rings are returned as closed linestrings (`first == last`).
    /// This includes both bounded and unbounded face cycles.
    pub fn extract_face_rings(&self, epsilon: f64) -> Vec<LineString> {
        let eps = normalized_eps(epsilon);
        let mut rings = Vec::<LineString>::new();
        let mut visited = vec![false; self.edges.len()];

        for start in 0..self.edges.len() {
            if visited[start] {
                continue;
            }

            let mut coords = Vec::<Coord>::new();
            let mut edge_ids = Vec::<usize>::new();
            let mut current = start;
            let mut ok = false;

            for _ in 0..=self.edges.len() {
                if visited[current] && current != start {
                    break;
                }

                visited[current] = true;
                edge_ids.push(current);
                let e = &self.edges[current];
                coords.push(self.nodes[e.from].coord);

                let Some(next) = self.next_left_face_edge(current) else {
                    break;
                };

                if next == start {
                    coords.push(self.nodes[self.edges[start].from].coord);
                    ok = true;
                    break;
                }
                current = next;
            }

            if !ok || coords.len() < 4 {
                continue;
            }

            if !nearly_eq_coord(coords[0], *coords.last().unwrap_or(&coords[0]), eps) {
                continue;
            }

            // Reject near-degenerate rings.
            if signed_area(&coords).abs() <= eps * eps {
                continue;
            }

            rings.push(LineString::new(coords));
        }

        rings
    }

    /// Extract bounded face cycles as closed rings.
    ///
    /// Bounded rings are identified by positive signed area under the
    /// left-face traversal convention.
    pub fn extract_bounded_face_rings(&self, epsilon: f64) -> Vec<LineString> {
        let eps = normalized_eps(epsilon);
        self.extract_face_rings(eps)
            .into_iter()
            .filter(|ls| signed_area(&ls.coords) > eps * eps)
            .collect()
    }
}

fn normalized_eps(epsilon: f64) -> f64 {
    if epsilon.is_finite() {
        epsilon.abs().max(1.0e-12)
    } else {
        1.0e-12
    }
}

fn edge_angle(a: Coord, b: Coord) -> f64 {
    (b.y - a.y).atan2(b.x - a.x)
}

fn node_key(c: Coord, eps: f64) -> NodeKey {
    if eps > 0.0 && eps.is_finite() {
        let qx = (c.x / eps).round() as i64;
        let qy = (c.y / eps).round() as i64;
        NodeKey::Quantized(qx, qy)
    } else {
        NodeKey::Exact(c.x.to_bits(), c.y.to_bits())
    }
}

fn get_or_insert_node(
    c: Coord,
    eps: f64,
    index: &mut HashMap<NodeKey, usize>,
    nodes: &mut Vec<GraphNode>,
) -> usize {
    let key = node_key(c, eps);
    if let Some(existing) = index.get(&key) {
        return *existing;
    }

    let id = nodes.len();
    nodes.push(GraphNode {
        id,
        coord: c,
        outgoing: Vec::new(),
    });
    index.insert(key, id);
    id
}

fn nearly_eq_coord(a: Coord, b: Coord, eps: f64) -> bool {
    (a.x - b.x).abs() <= eps && (a.y - b.y).abs() <= eps
}

fn signed_area(coords: &[Coord]) -> f64 {
    if coords.len() < 4 {
        return 0.0;
    }
    let mut s = 0.0;
    for i in 0..(coords.len() - 1) {
        let a = coords[i];
        let b = coords[i + 1];
        s += a.x * b.y - b.x * a.y;
    }
    0.5 * s
}
