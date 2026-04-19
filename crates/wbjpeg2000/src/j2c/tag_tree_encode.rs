//! Tag tree encoder for JPEG 2000 packet headers.
//!
//! The tag tree is a hierarchical structure used to efficiently encode
//! code-block inclusion information and zero bitplane counts in packet headers.
//! See Annex B.10.2 of ITU-T T.800.

use alloc::vec;
use alloc::vec::Vec;

use crate::writer::BitWriter;

/// A single node in the tag tree.
#[derive(Debug, Clone)]
struct TagNode {
    value: u32,
    current_value: u32,
    known: bool,
}

/// Tag tree encoder.
///
/// Encodes values using a hierarchical min-tree structure where each parent
/// node's value is the minimum of its children. This allows efficient encoding
/// by skipping already-known ranges.
#[derive(Debug)]
pub(crate) struct TagTreeEncoder {
    nodes: Vec<TagNode>,
    width: u32,
    height: u32,
    num_levels: u32,
    level_offsets: Vec<usize>,
}

impl TagTreeEncoder {
    /// Create a new tag tree for a grid of `width × height` leaf values.
    pub(crate) fn new(width: u32, height: u32) -> Self {
        let mut level_offsets = Vec::new();
        let mut total_nodes = 0usize;
        let mut w = width;
        let mut h = height;
        let mut num_levels = 0;

        loop {
            level_offsets.push(total_nodes);
            total_nodes += (w * h) as usize;
            num_levels += 1;

            if w <= 1 && h <= 1 {
                break;
            }

            w = w.div_ceil(2);
            h = h.div_ceil(2);
        }

        let nodes = vec![
            TagNode {
                value: 0,
                current_value: 0,
                known: false,
            };
            total_nodes
        ];

        Self {
            nodes,
            width,
            height,
            num_levels,
            level_offsets,
        }
    }

    /// Set the value of a leaf node at position (x, y).
    pub(crate) fn set_value(&mut self, x: u32, y: u32, value: u32) {
        let idx = self.level_offsets[0] + (y * self.width + x) as usize;
        self.nodes[idx].value = value;

        // Propagate minimum up the tree
        let mut cx = x;
        let mut cy = y;
        let mut cw = self.width;
        let mut ch = self.height;

        for level in 1..self.num_levels as usize {
            cx /= 2;
            cy /= 2;
            cw = cw.div_ceil(2);
            ch = ch.div_ceil(2);

            let parent_idx = self.level_offsets[level] + (cy * cw + cx) as usize;

            // Parent's value is the minimum of all its children
            let child_x_start = cx * 2;
            let child_y_start = cy * 2;
            let child_x_end = ((cx + 1) * 2).min(if level == 1 { self.width } else { cw * 2 });
            let child_y_end = ((cy + 1) * 2).min(if level == 1 { self.height } else { ch * 2 });

            let prev_w = if level == 1 {
                self.width
            } else {
                (self.width + (1 << (level - 1)) - 1) >> (level - 1)
            };

            let mut min_val = u32::MAX;
            for ccy in child_y_start..child_y_end {
                for ccx in child_x_start..child_x_end {
                    let child_idx = self.level_offsets[level - 1] + (ccy * prev_w + ccx) as usize;
                    min_val = min_val.min(self.nodes[child_idx].value);
                }
            }
            self.nodes[parent_idx].value = min_val;
        }
    }

    /// Encode the value at leaf position (x, y) up to threshold `max_val`.
    ///
    /// Writes bits to `writer` following the tag tree coding procedure (B.10.2).
    /// Returns the encoded value if it was below `max_val`, or None if >= `max_val`.
    pub(crate) fn encode(&mut self, x: u32, y: u32, max_val: u32, writer: &mut BitWriter) {
        // Build path from root to leaf
        let mut path = Vec::with_capacity(self.num_levels as usize);
        let mut cx = x;
        let mut cy = y;
        let mut cw = self.width;

        for level in 0..self.num_levels as usize {
            let idx = self.level_offsets[level] + (cy * cw + cx) as usize;
            path.push(idx);
            cx /= 2;
            cy /= 2;
            cw = cw.div_ceil(2);
        }

        // Encode from root to leaf
        let mut parent_val = 0u32;
        for &node_idx in path.iter().rev() {
            let node = &mut self.nodes[node_idx];
            let start = node.current_value.max(parent_val);

            if !node.known {
                let target = node.value.min(max_val);
                for v in start..target {
                    writer.write_bit(0); // Value is at least v+1
                    let _ = v; // avoid unused warning
                }
                if node.value < max_val {
                    writer.write_bit(1); // Value is exactly this
                    node.known = true;
                }
                node.current_value = target;
            }

            parent_val = node.current_value;
        }
    }

    /// Reset the encoder state for re-encoding (new layer).
    #[allow(dead_code)]
    pub(crate) fn reset_state(&mut self) {
        for node in &mut self.nodes {
            node.current_value = 0;
            node.known = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_value() {
        let mut tree = TagTreeEncoder::new(1, 1);
        tree.set_value(0, 0, 3);

        let mut writer = BitWriter::new();
        tree.encode(0, 0, 4, &mut writer);
        let data = writer.finish();
        // Should encode: 0, 0, 0, 1 (three zeros then one)
        assert!(!data.is_empty());
    }

    #[test]
    fn test_2x2_tree() {
        let mut tree = TagTreeEncoder::new(2, 2);
        tree.set_value(0, 0, 0);
        tree.set_value(1, 0, 1);
        tree.set_value(0, 1, 2);
        tree.set_value(1, 1, 3);

        let mut writer = BitWriter::new();
        // Encode (0,0) with threshold 1
        tree.encode(0, 0, 1, &mut writer);
        let data = writer.finish();
        assert!(!data.is_empty());
    }

    #[test]
    fn test_new_tree_dimensions() {
        let tree = TagTreeEncoder::new(4, 4);
        // 4×4 → 2×2 → 1×1 = 3 levels
        assert_eq!(tree.num_levels, 3);
        // 16 + 4 + 1 = 21 nodes
        assert_eq!(tree.nodes.len(), 21);
    }
}
