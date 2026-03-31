//! Extract points from a COPC file using combined hierarchy-key queries,
//! then write the result to LAS for visual QA.
//!
//! Usage:
//!   cargo run -p wblidar --example copc_query_extract_las -- <input.copc.laz> <output.las> [max_depth]

use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter};

use wblidar::copc::hierarchy::VoxelKey;
use wblidar::copc::reader::{CopcBoundingBox, CopcReader};
use wblidar::io::PointWriter;
use wblidar::las::reader::LasReader;
use wblidar::las::writer::{LasWriter, WriterConfig};
use wblidar::{Error, Result};

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let in_path = args.next().ok_or_else(usage_error)?;
    let out_path = args.next().ok_or_else(usage_error)?;
    let max_depth = args
        .next()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(3);

    let crs = {
        let las_reader = LasReader::new(BufReader::new(File::open(&in_path).map_err(Error::Io)?))?;
        las_reader.crs().cloned()
    };

    let file = BufReader::new(File::open(&in_path).map_err(Error::Io)?);
    let mut reader = CopcReader::new(file)?;

    let all_data_keys = reader.data_node_keys();
    let subtree_root = all_data_keys
        .iter()
        .find(|k| k.level > 0)
        .copied()
        .unwrap_or(VoxelKey::ROOT);

    // Use a centered quarter-cube bbox as a practical default QA subset.
    let q = reader.info.halfsize * 0.25;
    let bbox = CopcBoundingBox {
        min_x: reader.info.center_x - q,
        max_x: reader.info.center_x + q,
        min_y: reader.info.center_y - q,
        max_y: reader.info.center_y + q,
        min_z: reader.info.center_z - q,
        max_z: reader.info.center_z + q,
    };

    let mut keys = reader.query_data_node_keys(Some(subtree_root), Some(bbox), Some(max_depth));
    keys.sort_by_key(|k| (k.level, k.x, k.y, k.z));

    let mut points = Vec::new();
    let mut node_count = 0usize;
    for key in &keys {
        let before = points.len();
        let _ = reader.read_node(*key, &mut points)?;
        if points.len() > before {
            node_count += 1;
        }
    }

    let hdr = reader.header().clone();
    let cfg = WriterConfig {
        point_data_format: hdr.point_data_format,
        x_scale: hdr.x_scale,
        y_scale: hdr.y_scale,
        z_scale: hdr.z_scale,
        x_offset: hdr.x_offset,
        y_offset: hdr.y_offset,
        z_offset: hdr.z_offset,
        system_identifier: hdr.system_identifier,
        generating_software: "wblidar example: copc_query_extract_las".to_string(),
        vlrs: Vec::new(),
        crs,
        extra_bytes_per_point: hdr.extra_bytes_count,
    };

    let out = BufWriter::new(File::create(&out_path).map_err(Error::Io)?);
    let mut writer = LasWriter::new(out, cfg)?;
    for p in &points {
        writer.write_point(p)?;
    }
    writer.finish()?;

    println!("input={in_path}");
    println!("output={out_path}");
    println!("max_depth={max_depth}");
    println!(
        "subtree_root=({}, {}, {}, {})",
        subtree_root.level, subtree_root.x, subtree_root.y, subtree_root.z
    );
    println!("candidate_keys={}", keys.len());
    println!("nodes_with_points={node_count}");
    println!("written_points={}", points.len());
    println!(
        "note=combined query may include internal and leaf nodes; overlapping node coverage can duplicate samples by design"
    );

    Ok(())
}

fn usage_error() -> Error {
    Error::Projection(
        "Usage: cargo run -p wblidar --example copc_query_extract_las -- <input.copc.laz> <output.las> [max_depth]"
            .to_string(),
    )
}
