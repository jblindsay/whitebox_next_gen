use std::collections::BTreeSet;
use std::env;
use std::fs::File;
use std::io::BufReader;

use wblidar::copc::reader::CopcReader;
use wblidar::copc::VoxelKey;
use wblidar::io::PointWriter;
use wblidar::las::writer::{LasWriter, WriterConfig};
use wblidar::Result;

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let path = args.next().expect("usage: inspect_copc_node <file.copc.laz> <chunk_index>");
    let chunk_index: usize = args
        .next()
        .expect("missing chunk_index")
        .parse()
        .expect("chunk_index must be usize");
    let export_path = args.next();

    let file = File::open(&path)?;
    let mut reader = CopcReader::new(BufReader::new(file))?;

    let mut keys: Vec<VoxelKey> = reader
        .hierarchy
        .entries
        .iter()
        .filter(|e| e.point_count > 0 && e.byte_size > 0)
        .map(|e| e.key)
        .collect();
    keys.sort_by_key(|k| (k.level, k.x, k.y, k.z));

    let key = keys[chunk_index];
    let entry = *reader.hierarchy.find(key).unwrap();
    let mut points = Vec::new();
    reader.read_node(key, &mut points)?;
    let hdr = reader.header().clone();

    let mut scanner_channels = BTreeSet::new();
    let mut return_numbers = BTreeSet::new();
    let mut num_returns = BTreeSet::new();
    let mut classifs = BTreeSet::new();
    let mut point_sources = BTreeSet::new();
    let mut scan_angles = BTreeSet::new();
    let mut user_data = BTreeSet::new();
    let mut has_gps = false;
    let mut gps_diffs = BTreeSet::new();
    let mut prev_gps_bits: Option<i64> = None;

    for p in &points {
        scanner_channels.insert((p.flags >> 4) & 0x03);
        return_numbers.insert(p.return_number);
        num_returns.insert(p.number_of_returns);
        classifs.insert(p.classification);
        point_sources.insert(p.point_source_id);
        scan_angles.insert(p.scan_angle);
        user_data.insert(p.user_data);
        if let Some(g) = p.gps_time {
            has_gps = true;
            let bits = g.0.to_bits() as i64;
            if let Some(prev) = prev_gps_bits {
                gps_diffs.insert(bits.wrapping_sub(prev));
            }
            prev_gps_bits = Some(bits);
        }
    }

    println!("file={path}");
    println!("chunk_index={chunk_index}");
    println!("key=({}, {}, {}, {})", key.level, key.x, key.y, key.z);
    println!("entry_offset={} entry_byte_size={} entry_point_count={}", entry.offset, entry.byte_size, entry.point_count);
    println!("decoded_points={}", points.len());
    println!("scanner_channels={:?}", scanner_channels);
    println!("return_numbers={:?}", return_numbers);
    println!("number_of_returns={:?}", num_returns);
    println!("classifications={:?}", classifs);
    println!("point_sources_sample_count={}", point_sources.len());
    println!("scan_angles_sample_count={}", scan_angles.len());
    println!("user_data_sample_count={}", user_data.len());
    println!("has_gps={}", has_gps);
    println!("gps_diff_sample_count={}", gps_diffs.len());
    println!("first_gps_diffs={:?}", gps_diffs.iter().take(12).collect::<Vec<_>>());

    if let Some(export_path) = export_path {
        let out = File::create(&export_path)?;
        let cfg = WriterConfig {
            point_data_format: hdr.point_data_format,
            x_scale: hdr.x_scale,
            y_scale: hdr.y_scale,
            z_scale: hdr.z_scale,
            x_offset: hdr.x_offset,
            y_offset: hdr.y_offset,
            z_offset: hdr.z_offset,
            system_identifier: hdr.system_identifier,
            generating_software: "inspect_copc_node".to_string(),
            vlrs: Vec::new(),
            crs: None,
            extra_bytes_per_point: hdr.extra_bytes_count,
        };
        let mut writer = LasWriter::new(out, cfg)?;
        for p in &points {
            writer.write_point(p)?;
        }
        writer.finish()?;
        println!("exported_las={export_path}");
    }

    Ok(())
}