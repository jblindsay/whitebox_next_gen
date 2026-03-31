//! Attribute COPC compressed bytes by Point14 layer across all data nodes.
//!
//! Usage:
//!   cargo run -p wblidar --example copc_layer_attribution -- <file1.copc.laz> [file2.copc.laz ...]
//!
//! This tool parses each node chunk using the standard Point14 layered layout
//! and reports aggregate bytes per layer and bytes-per-point.

use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};

use wblidar::copc::reader::CopcReader;
use wblidar::las::header::PointDataFormat;
use wblidar::Result;

const CORE_LAYER_NAMES: [&str; 9] = [
    "xy",
    "z",
    "classification",
    "flags",
    "intensity",
    "scan_angle",
    "user_data",
    "point_source",
    "gps_time",
];

#[derive(Default)]
struct ParseStats {
    chunks: usize,
    points: u64,
    layer_totals: BTreeMap<String, u64>,
    layer_order: Vec<String>,
    parse_failures: usize,
}

fn main() -> Result<()> {
    let inputs: Vec<String> = env::args().skip(1).collect();
    if inputs.is_empty() {
        eprintln!(
            "Usage: cargo run -p wblidar --example copc_layer_attribution -- <file1.copc.laz> [file2.copc.laz ...]"
        );
        std::process::exit(2);
    }

    for path in &inputs {
        let stats = attribute_file(path)?;
        print_report(path, &stats)?;
    }

    Ok(())
}

fn attribute_file(path: &str) -> Result<ParseStats> {
    let input = BufReader::new(File::open(path)?);
    let mut reader = CopcReader::new(input)?;
    let header = reader.header().clone();

    let fmt = header.point_data_format;
    let has_rgb = matches!(fmt, PointDataFormat::Pdrf7 | PointDataFormat::Pdrf8);
    let has_nir = matches!(fmt, PointDataFormat::Pdrf8);
    let extra_bytes = header.extra_bytes_count as usize;

    let mut layer_names: Vec<String> = CORE_LAYER_NAMES.iter().map(|s| s.to_string()).collect();
    if has_rgb {
        layer_names.push("rgb".to_string());
    }
    if has_nir {
        layer_names.push("nir".to_string());
    }
    for i in 0..extra_bytes {
        layer_names.push(format!("extra_{i}"));
    }

    let mut stats = ParseStats {
        layer_order: layer_names.clone(),
        ..ParseStats::default()
    };

    let mut file = File::open(path)?;
    let seed_len = usize::from(fmt.core_size()) + extra_bytes;

    let mut entries = reader
        .hierarchy
        .entries
        .iter()
        .filter(|e| e.point_count > 0 && e.byte_size > 0)
        .collect::<Vec<_>>();
    entries.sort_by_key(|e| (e.key.level, e.key.x, e.key.y, e.key.z));

    for entry in entries {
        let chunk_size = entry.byte_size as usize;
        let mut chunk = vec![0u8; chunk_size];
        file.seek(SeekFrom::Start(entry.offset))?;
        if let Err(_) = file.read_exact(&mut chunk) {
            stats.parse_failures += 1;
            continue;
        }

        match parse_point14_layer_sizes(&chunk, seed_len, has_rgb, has_nir, extra_bytes) {
            Some((point_count, layer_sizes)) => {
                stats.chunks += 1;
                stats.points = stats.points.saturating_add(point_count as u64);
                for (name, size) in layer_names.iter().zip(layer_sizes.iter()) {
                    let e = stats.layer_totals.entry(name.clone()).or_insert(0);
                    *e = e.saturating_add(*size as u64);
                }
            }
            None => {
                stats.parse_failures += 1;
            }
        }
    }

    Ok(stats)
}

fn parse_point14_layer_sizes(
    chunk: &[u8],
    seed_len: usize,
    has_rgb: bool,
    has_nir: bool,
    extra_bytes: usize,
) -> Option<(u32, Vec<u32>)> {
    if chunk.len() < seed_len + 4 {
        return None;
    }

    let mut pos = seed_len;
    let point_count = read_u32(chunk, &mut pos)?;
    let layer_count = 9 + if has_rgb { 1 } else { 0 } + if has_nir { 1 } else { 0 } + extra_bytes;

    let mut layer_sizes = Vec::with_capacity(layer_count);
    for _ in 0..layer_count {
        layer_sizes.push(read_u32(chunk, &mut pos)?);
    }

    let payload_sum: usize = layer_sizes.iter().map(|&n| n as usize).sum();
    if pos + payload_sum > chunk.len() {
        return None;
    }

    Some((point_count, layer_sizes))
}

fn read_u32(bytes: &[u8], pos: &mut usize) -> Option<u32> {
    if *pos + 4 > bytes.len() {
        return None;
    }
    let out = u32::from_le_bytes([
        bytes[*pos],
        bytes[*pos + 1],
        bytes[*pos + 2],
        bytes[*pos + 3],
    ]);
    *pos += 4;
    Some(out)
}

fn print_report(path: &str, stats: &ParseStats) -> Result<()> {
    let file_size = std::fs::metadata(path)?.len();
    let point_count = stats.points.max(1);

    println!("file={path}");
    println!("file_size_bytes={}", file_size);
    println!("parsed_chunks={}", stats.chunks);
    println!("parse_failures={}", stats.parse_failures);
    println!("parsed_points={}", stats.points);
    println!(
        "payload_ratio_vs_file={:.4}",
        stats
            .layer_totals
            .values()
            .fold(0u64, |acc, v| acc.saturating_add(*v)) as f64
            / file_size.max(1) as f64
    );
    println!("layer,bytes,bytes_per_point,percent_of_file");

    for name in &stats.layer_order {
        let bytes = *stats.layer_totals.get(name).unwrap_or(&0);
        let bpp = bytes as f64 / point_count as f64;
        let pct = 100.0 * bytes as f64 / file_size.max(1) as f64;
        println!("{name},{bytes},{bpp:.6},{pct:.4}");
    }

    println!();
    Ok(())
}
