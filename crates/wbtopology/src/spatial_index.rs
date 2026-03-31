//! Spatial index for fast envelope-based candidate filtering.
//!
//! This module provides a lightweight, dependency-free packed spatial index
//! over geometry envelopes. It follows a read-mostly STR-style packing scheme
//! so range and nearest-neighbor queries can prune whole branches instead of
//! scanning a flat list.

use crate::algorithms::distance::geometry_distance;
use crate::geom::{Coord, Envelope, Geometry};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

const DEFAULT_NODE_CAPACITY: usize = 8;

/// One indexed geometry record.
#[derive(Debug, Clone)]
pub struct IndexedGeometry {
    /// User-facing id assigned by insertion order.
    pub id: usize,
    /// Envelope cached for fast filtering.
    pub envelope: Envelope,
    /// Stored geometry.
    pub geometry: Geometry,
}

/// A read-mostly spatial index over geometries.
///
/// The index is built as a packed hierarchy of envelopes. Insertions are still
/// supported, but they rebuild the packed tree to keep query performance
/// predictable.
#[derive(Debug, Clone)]
pub struct SpatialIndex {
    entries: Vec<IndexedGeometry>,
    nodes: Vec<TreeNode>,
    root: Option<usize>,
    node_capacity: usize,
}

#[derive(Debug, Clone)]
struct TreeNode {
    envelope: Envelope,
    children: NodeChildren,
}

#[derive(Debug, Clone)]
enum NodeChildren {
    Leaf(Vec<usize>),
    Internal(Vec<usize>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SearchItem {
    Node(usize),
    Entry(usize),
}

#[derive(Debug, Clone, Copy)]
struct QueueEntry {
    distance: f64,
    item: SearchItem,
}

impl PartialEq for QueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.distance.to_bits() == other.distance.to_bits() && self.item == other.item
    }
}

impl Eq for QueueEntry {}

impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .distance
            .total_cmp(&self.distance)
            .then_with(|| self.item.cmp(&other.item))
    }
}

impl Default for SpatialIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl SpatialIndex {
    /// Create an empty spatial index.
    pub fn new() -> Self {
        Self::with_node_capacity(DEFAULT_NODE_CAPACITY)
    }

    /// Create an empty spatial index with the given packed-node capacity.
    pub fn with_node_capacity(node_capacity: usize) -> Self {
        Self {
            entries: vec![],
            nodes: vec![],
            root: None,
            node_capacity: node_capacity.max(2),
        }
    }

    /// Build an index from geometries; ids are assigned in slice order.
    pub fn from_geometries(geometries: &[Geometry]) -> Self {
        Self::build_str(geometries, DEFAULT_NODE_CAPACITY)
    }

    /// Build a packed STR-style index from geometries.
    pub fn build_str(geometries: &[Geometry], node_capacity: usize) -> Self {
        let mut entries = Vec::with_capacity(geometries.len());
        for geometry in geometries {
            if let Some(envelope) = geometry.envelope() {
                let id = entries.len();
                entries.push(IndexedGeometry {
                    id,
                    envelope,
                    geometry: geometry.clone(),
                });
            }
        }

        let mut idx = Self {
            entries,
            nodes: vec![],
            root: None,
            node_capacity: node_capacity.max(2),
        };
        idx.rebuild();
        idx
    }

    /// Number of live (non-removed) indexed entries.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.iter().filter(|e| e.id != usize::MAX).count()
    }

    /// True when there are no live indexed entries.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.iter().all(|e| e.id == usize::MAX)
    }

    /// Packed-node capacity used by this index.
    #[inline]
    pub fn node_capacity(&self) -> usize {
        self.node_capacity
    }

    /// Tree depth in packed levels.
    pub fn depth(&self) -> usize {
        let Some(root) = self.root else {
            return 0;
        };
        self.node_depth(root)
    }

    /// Insert one geometry and return its assigned id.
    ///
    /// Empty geometries are ignored and return `None`.
    pub fn insert(&mut self, geometry: Geometry) -> Option<usize> {
        let envelope = geometry.envelope()?;
        let id = self.entries.len();
        self.entries.push(IndexedGeometry {
            id,
            envelope,
            geometry,
        });
        self.rebuild();
        Some(id)
    }

    /// Return all ids whose envelopes intersect `env`.
    pub fn query_envelope(&self, env: Envelope) -> Vec<usize> {
        let mut out = Vec::new();
        let Some(root) = self.root else {
            return out;
        };

        let mut stack = vec![root];
        while let Some(node_idx) = stack.pop() {
            let node = &self.nodes[node_idx];
            if !node.envelope.intersects(&env) {
                continue;
            }

            match &node.children {
                NodeChildren::Leaf(entry_ids) => {
                    for &entry_id in entry_ids {
                        let entry = &self.entries[entry_id];
                        if entry.id != usize::MAX && entry.envelope.intersects(&env) {
                            out.push(entry.id);
                        }
                    }
                }
                NodeChildren::Internal(children) => {
                    stack.extend(children.iter().rev().copied());
                }
            }
        }

        out
    }

    /// Return all ids whose envelopes contain point `p`.
    pub fn query_point(&self, p: Coord) -> Vec<usize> {
        self.query_envelope(Envelope::new(p.x, p.y, p.x, p.y))
    }

    /// Return all ids with envelopes intersecting geometry `g` envelope.
    pub fn query_geometry(&self, g: &Geometry) -> Vec<usize> {
        match g.envelope() {
            Some(env) => self.query_envelope(env),
            None => Vec::new(),
        }
    }

    /// Borrow one indexed geometry by id.
    ///
    /// Returns `None` when the id was never assigned or has since been removed.
    pub fn get(&self, id: usize) -> Option<&IndexedGeometry> {
        self.entries.get(id).filter(|e| e.id != usize::MAX)
    }

    /// Iterate over all active entries in insertion order.
    pub fn all_entries(&self) -> impl Iterator<Item = &IndexedGeometry> {
        self.entries.iter().filter(|e| e.id != usize::MAX)
    }

    /// Remove the entry with the given id.
    ///
    /// The entry is tombstoned so its slot is no longer returned by queries or
    /// iterators. The packed tree is rebuilt after removal.
    ///
    /// ID semantics:
    /// - `remove` preserves ids of surviving entries.
    /// - removed ids are not reused until [`compact`](Self::compact) is called.
    ///
    /// Returns `true` when an active entry was removed, `false` when the id was
    /// already absent.
    pub fn remove(&mut self, id: usize) -> bool {
        if id >= self.entries.len() || self.entries[id].id == usize::MAX {
            return false;
        }
        // Tombstone the entry; IDs of surviving entries are preserved.
        self.entries[id].id = usize::MAX;
        self.rebuild();
        true
    }

    /// Return the `k` nearest indexed geometries to `target`, ordered by
    /// ascending exact geometry distance.
    ///
    /// Returns fewer than `k` entries when the index has fewer than `k`
    /// elements. Returns an empty `Vec` when the index is empty or `k == 0`.
    pub fn nearest_k(&self, target: &Geometry, k: usize) -> Vec<(usize, f64)> {
        if k == 0 {
            return vec![];
        }
        let Some(target_env) = target.envelope() else {
            return vec![];
        };
        let Some(root) = self.root else {
            return vec![];
        };

        // Priority queue: min-heap by lower-bound distance.
        let mut queue: BinaryHeap<QueueEntry> = BinaryHeap::new();
        queue.push(QueueEntry {
            distance: envelope_distance_lower_bound(target_env, self.nodes[root].envelope),
            item: SearchItem::Node(root),
        });

        let mut results: Vec<(usize, f64)> = Vec::with_capacity(k);

        while let Some(candidate) = queue.pop() {
            // Cut-off: lower bound already exceeds the k-th best candidate.
            if results.len() == k && candidate.distance > results[k - 1].1 {
                break;
            }

            match candidate.item {
                SearchItem::Node(node_idx) => match &self.nodes[node_idx].children {
                    NodeChildren::Leaf(entry_ids) => {
                        for &entry_id in entry_ids {
                            let entry = &self.entries[entry_id];
                            if entry.id == usize::MAX {
                                continue; // tombstone
                            }
                            let lb = envelope_distance_lower_bound(target_env, entry.envelope);
                            if results.len() == k && lb > results[k - 1].1 {
                                continue;
                            }
                            queue.push(QueueEntry {
                                distance: lb,
                                item: SearchItem::Entry(entry_id),
                            });
                        }
                    }
                    NodeChildren::Internal(children) => {
                        for &child_idx in children {
                            let lb = envelope_distance_lower_bound(
                                target_env,
                                self.nodes[child_idx].envelope,
                            );
                            if results.len() == k && lb > results[k - 1].1 {
                                continue;
                            }
                            queue.push(QueueEntry {
                                distance: lb,
                                item: SearchItem::Node(child_idx),
                            });
                        }
                    }
                },
                SearchItem::Entry(entry_id) => {
                    let entry = &self.entries[entry_id];
                    if entry.id == usize::MAX {
                        continue; // tombstone
                    }
                    let d = geometry_distance(target, &entry.geometry);
                    // Insert into sorted results, keeping at most k.
                    let pos = results.partition_point(|&(_, bd)| bd <= d);
                    if pos < k {
                        results.insert(pos, (entry.id, d));
                        if results.len() > k {
                            results.pop();
                        }
                    }
                }
            }
        }

        results
    }

    /// Return nearest indexed geometry to `target` using exact geometry distance.
    ///
    /// Returns `(id, distance)` or `None` when index is empty.
    pub fn nearest_neighbor(&self, target: &Geometry) -> Option<(usize, f64)> {
        let target_env = target.envelope()?;
        let root = self.root?;

        let mut best_id = None;
        let mut best_d = f64::INFINITY;
        let mut queue = BinaryHeap::new();
        queue.push(QueueEntry {
            distance: envelope_distance_lower_bound(target_env, self.nodes[root].envelope),
            item: SearchItem::Node(root),
        });

        while let Some(candidate) = queue.pop() {
            if candidate.distance > best_d {
                break;
            }

            match candidate.item {
                SearchItem::Node(node_idx) => match &self.nodes[node_idx].children {
                    NodeChildren::Leaf(entry_ids) => {
                        for &entry_id in entry_ids {
                            let entry = &self.entries[entry_id];
                            let lower_bound =
                                envelope_distance_lower_bound(target_env, entry.envelope);
                            if lower_bound > best_d {
                                continue;
                            }
                            queue.push(QueueEntry {
                                distance: lower_bound,
                                item: SearchItem::Entry(entry_id),
                            });
                        }
                    }
                    NodeChildren::Internal(children) => {
                        for &child_idx in children {
                            let lower_bound = envelope_distance_lower_bound(
                                target_env,
                                self.nodes[child_idx].envelope,
                            );
                            if lower_bound > best_d {
                                continue;
                            }
                            queue.push(QueueEntry {
                                distance: lower_bound,
                                item: SearchItem::Node(child_idx),
                            });
                        }
                    }
                },
                SearchItem::Entry(entry_id) => {
                    let entry = &self.entries[entry_id];
                    if entry.id == usize::MAX {
                        continue; // tombstone
                    }
                    let d = geometry_distance(target, &entry.geometry);
                    if d < best_d {
                        best_d = d;
                        best_id = Some(entry.id);
                    }
                }
            }
        }

        best_id.map(|id| (id, best_d))
    }

    fn rebuild(&mut self) {
        // Collect (position_in_entries_vec, &entry) for every live entry so
        // that leaf-node indices always refer to valid positions in self.entries
        // even when tombstones have left gaps.
        let live: Vec<(usize, &IndexedGeometry)> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.id != usize::MAX)
            .collect();
        let Some((nodes, root)) = build_packed_tree_with_pos(&live, self.node_capacity) else {
            self.nodes.clear();
            self.root = None;
            return;
        };
        self.nodes = nodes;
        self.root = Some(root);
    }

    /// Compact the index by removing tombstoned entries and reassigning dense ids.
    ///
    /// After many [`remove`](Self::remove) calls the internal `entries` Vec will
    /// hold gaps.  Call `compact` to reclaim that memory.  Note: compaction
    /// **changes the ids** of all surviving entries; any ids held outside the
    /// index become stale.
    pub fn compact(&mut self) {
        let active: Vec<IndexedGeometry> = self
            .entries
            .drain(..)
            .filter(|e| e.id != usize::MAX)
            .enumerate()
            .map(|(new_id, mut e)| {
                e.id = new_id;
                e
            })
            .collect();
        self.entries = active;
        self.rebuild();
    }

    fn node_depth(&self, node_idx: usize) -> usize {
        match &self.nodes[node_idx].children {
            NodeChildren::Leaf(_) => 1,
            NodeChildren::Internal(children) => {
                1 + children
                    .iter()
                    .map(|&child_idx| self.node_depth(child_idx))
                    .max()
                    .unwrap_or(0)
            }
        }
    }
}

/// Build a packed STR tree from `(position_in_entries, &entry)` pairs.
///
/// Leaf nodes store the *position* values as-is, so queries index directly
/// into `SpatialIndex::entries` without any offset arithmetic.  This works
/// correctly even when tombstoned entries have left gaps in the Vec.
fn build_packed_tree_with_pos(
    live: &[(usize, &IndexedGeometry)],
    node_capacity: usize,
) -> Option<(Vec<TreeNode>, usize)> {
    if live.is_empty() {
        return None;
    }

    let cap = node_capacity.max(2);
    let mut nodes = Vec::<TreeNode>::new();

    // Sort indices over `live`; leaf nodes will store the mapped positions.
    let sort_ids = (0..live.len()).collect::<Vec<_>>();
    let leaf_groups = str_group(sort_ids, cap, |i| envelope_center(live[i].1.envelope));

    let mut current_level = Vec::<usize>::with_capacity(leaf_groups.len());
    for group in leaf_groups {
        // Map sort-local indices to actual positions in `self.entries`.
        let positions: Vec<usize> = group.iter().map(|&i| live[i].0).collect();
        let envelope = group
            .iter()
            .fold(live[group[0]].1.envelope, |acc, &i| merge_envelopes(acc, live[i].1.envelope));
        nodes.push(TreeNode {
            envelope,
            children: NodeChildren::Leaf(positions),
        });
        current_level.push(nodes.len() - 1);
    }

    while current_level.len() > 1 {
        let parent_groups = str_group(current_level, cap, |node_idx| {
            envelope_center(nodes[node_idx].envelope)
        });
        let mut next_level = Vec::<usize>::with_capacity(parent_groups.len());
        for group in parent_groups {
            let envelope = group_envelope_nodes(&nodes, &group);
            nodes.push(TreeNode {
                envelope,
                children: NodeChildren::Internal(group),
            });
            next_level.push(nodes.len() - 1);
        }
        current_level = next_level;
    }

    Some((nodes, current_level[0]))
}

fn str_group<T, F>(mut items: Vec<T>, node_capacity: usize, coord_fn: F) -> Vec<Vec<T>>
where
    T: Copy,
    F: Fn(T) -> (f64, f64),
{
    if items.is_empty() {
        return vec![];
    }

    let cap = node_capacity.max(2);
    items.sort_by(|&a, &b| coord_fn(a).0.total_cmp(&coord_fn(b).0));

    let group_count = ceil_div(items.len(), cap);
    let slice_count = ceil_sqrt(group_count.max(1));
    let slice_size = ceil_div(items.len(), slice_count.max(1));
    let mut groups = Vec::<Vec<T>>::new();

    for mut slice in items.chunks(slice_size.max(1)).map(|chunk| chunk.to_vec()) {
        slice.sort_by(|&a, &b| coord_fn(a).1.total_cmp(&coord_fn(b).1));
        for group in slice.chunks(cap) {
            groups.push(group.to_vec());
        }
    }

    groups
}

fn group_envelope_nodes(nodes: &[TreeNode], ids: &[usize]) -> Envelope {
    let mut envelope = nodes[ids[0]].envelope;
    for &id in &ids[1..] {
        envelope = merge_envelopes(envelope, nodes[id].envelope);
    }
    envelope
}

fn envelope_center(env: Envelope) -> (f64, f64) {
    ((env.min_x + env.max_x) * 0.5, (env.min_y + env.max_y) * 0.5)
}

fn merge_envelopes(a: Envelope, b: Envelope) -> Envelope {
    Envelope::new(
        a.min_x.min(b.min_x),
        a.min_y.min(b.min_y),
        a.max_x.max(b.max_x),
        a.max_y.max(b.max_y),
    )
}

fn ceil_div(n: usize, d: usize) -> usize {
    if n == 0 {
        0
    } else {
        1 + (n - 1) / d.max(1)
    }
}

fn ceil_sqrt(n: usize) -> usize {
    if n <= 1 {
        return n;
    }

    let mut root = 1usize;
    while root.saturating_mul(root) < n {
        root += 1;
    }
    root
}

fn envelope_distance_lower_bound(a: Envelope, b: Envelope) -> f64 {
    let dx = if a.max_x < b.min_x {
        b.min_x - a.max_x
    } else if b.max_x < a.min_x {
        a.min_x - b.max_x
    } else {
        0.0
    };

    let dy = if a.max_y < b.min_y {
        b.min_y - a.max_y
    } else if b.max_y < a.min_y {
        a.min_y - b.max_y
    } else {
        0.0
    };

    (dx * dx + dy * dy).sqrt()
}
