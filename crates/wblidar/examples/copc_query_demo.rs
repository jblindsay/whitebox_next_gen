//! Demonstrate COPC hierarchy key queries (subtree, bbox, max-depth).
//!
//! Usage:
//!   cargo run -p wblidar --example copc_query_demo -- <input.copc.laz> [max_depth]

use std::env;
use std::fs::File;
use std::io::BufReader;

use wblidar::copc::hierarchy::VoxelKey;
use wblidar::copc::reader::{CopcBoundingBox, CopcReader};
use wblidar::Result;

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let input = args.next().ok_or_else(|| {
        wblidar::Error::InvalidValue {
            field: "args",
            detail: "usage: copc_query_demo <input.copc.laz> [max_depth]".to_string(),
        }
    })?;
    let max_depth = args
        .next()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(2);

    let file = BufReader::new(File::open(&input).map_err(wblidar::Error::Io)?);
    let mut reader = CopcReader::new(file)?;

    let mut all_keys = reader.data_node_keys();
    all_keys.sort_by_key(|k| (k.level, k.x, k.y, k.z));
    println!("file: {input}");
    println!("data nodes: {}", all_keys.len());

    let depth_keys = reader.data_node_keys_max_depth(max_depth);
    println!("data nodes at max depth {max_depth}: {}", depth_keys.len());

    let subtree_root = all_keys
        .iter()
        .find(|k| k.level > 0)
        .copied()
        .unwrap_or(VoxelKey::ROOT);
    let subtree_keys = reader.data_node_keys_subtree(subtree_root);
    println!(
        "subtree root ({},{},{},{}) node count: {}",
        subtree_root.level,
        subtree_root.x,
        subtree_root.y,
        subtree_root.z,
        subtree_keys.len()
    );

    let q = reader.info.halfsize * 0.25;
    let bbox = CopcBoundingBox {
        min_x: reader.info.center_x - q,
        max_x: reader.info.center_x + q,
        min_y: reader.info.center_y - q,
        max_y: reader.info.center_y + q,
        min_z: reader.info.center_z - q,
        max_z: reader.info.center_z + q,
    };
    let bbox_keys = reader.data_node_keys_bbox(bbox);
    println!("bbox-intersecting data nodes: {}", bbox_keys.len());

    let combined = reader.query_data_node_keys(Some(subtree_root), Some(bbox), Some(max_depth));
    println!("combined query data nodes: {}", combined.len());

    if let Some(first_key) = combined.first().copied() {
        let mut points = Vec::new();
        let n = reader.read_node(first_key, &mut points)?;
        println!(
            "read {} point(s) from first combined key ({},{},{},{})",
            n, first_key.level, first_key.x, first_key.y, first_key.z
        );
    }

    Ok(())
}
