//! Benchmark COPC compression with different configurations to explore max compression potential.
//!
//! Usage:
//!   cargo run -p wblidar --example copc_compression_tuning -- <input.las> <output_prefix>
//!
//! This will generate multiple COPC files with different max_points_per_node settings
//! to measure the impact on file size and hierarchy overhead.

use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use wblidar::copc::{CopcWriter, CopcWriterConfig};
use wblidar::io::{PointReader, PointWriter};
use wblidar::las::reader::LasReader;
use wblidar::las::writer::WriterConfig;
use wblidar::{Error, PointRecord, Result};

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let in_path = args.next().ok_or_else(usage_error)?;
    let out_prefix = args.next().ok_or_else(usage_error)?;

    // ── Load source and compute bounds (same as las_to_copc) ───────────────
    let in_file = File::open(&in_path).map_err(Error::Io)?;
    let mut reader = LasReader::new(BufReader::new(in_file))?;
    let hdr = reader.header().clone();
    let crs = reader.crs().cloned();

    println!("Input: {in_path}");
    println!(
        "  PDRF {} | {} points | scale {},{},{}",
        hdr.point_data_format as u8,
        hdr.point_count_64.unwrap_or(hdr.legacy_point_count as u64),
        hdr.x_scale, hdr.y_scale, hdr.z_scale,
    );

    let mut buf = PointRecord::default();
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut min_z = f64::INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    let mut n_read: u64 = 0;

    while reader.read_point(&mut buf)? {
        min_x = min_x.min(buf.x);
        max_x = max_x.max(buf.x);
        min_y = min_y.min(buf.y);
        max_y = max_y.max(buf.y);
        min_z = min_z.min(buf.z);
        max_z = max_z.max(buf.z);
        n_read += 1;
    }

    let center_x = (min_x + max_x) * 0.5;
    let center_y = (min_y + max_y) * 0.5;
    let center_z = (min_z + max_z) * 0.5;
    let halfsize = ((max_x - min_x).max(max_y - min_y).max(max_z - min_z) * 0.5).max(1.0);
    let spacing = (halfsize * 2.0 / 1024.0).max(0.000_001);

    println!("  {} points, cube center ({},{},{}), halfsize={}", n_read, center_x, center_y, center_z, halfsize);

    // ── Test configurations ────────────────────────────────────────────────
    let configs = vec![
        ("default_10k", 10_000),
        ("medium_20k", 20_000),
        ("large_50k", 50_000),
        ("xlarge_100k", 100_000),
    ];

    println!("\nGenerating COPC files with different max_points_per_node settings:\n");
    println!("{:20} {:15} {:15} {}", "Config", "File Size", "% of Source", "Path");
    println!("{}", "=".repeat(75));

    let source_size = std::fs::metadata(&in_path).ok().map(|m| m.len()).unwrap_or(0) as f64 / (1024.0 * 1024.0);

    for (name, max_points) in configs {
        let out_path = format!("{}_{}.copc.laz", out_prefix, name);

        let las_cfg = WriterConfig {
            point_data_format: hdr.point_data_format,
            x_scale: hdr.x_scale,
            y_scale: hdr.y_scale,
            z_scale: hdr.z_scale,
            x_offset: hdr.x_offset,
            y_offset: hdr.y_offset,
            z_offset: hdr.z_offset,
            system_identifier: hdr.system_identifier.clone(),
            generating_software: "wblidar: copc_compression_tuning".to_string(),
            vlrs: Vec::new(),
            crs: crs.clone(),
            extra_bytes_per_point: 0,
        };

        let cfg = CopcWriterConfig {
            las: las_cfg,
            center_x,
            center_y,
            center_z,
            halfsize,
            spacing,
            max_depth: 8,
            max_points_per_node: max_points,
            compression_level: 6,
        };

        let out = BufWriter::new(File::create(&out_path).map_err(Error::Io)?);
        let mut writer = CopcWriter::new(out, cfg);

        let in_file_2 = File::open(&in_path).map_err(Error::Io)?;
        let mut reader_2 = LasReader::new(BufReader::new(in_file_2))?;
        while reader_2.read_point(&mut buf)? {
            writer.write_point(&buf)?;
        }
        writer.finish()?;

        let file_size = std::fs::metadata(&out_path)
            .ok()
            .map(|m| m.len())
            .unwrap_or(0) as f64 / (1024.0 * 1024.0);

        let pct = if source_size > 0.0 {
            file_size / source_size * 100.0
        } else {
            0.0
        };

        println!("{:20} {:8.2} MB     {:8.1}%     {}", name, file_size, pct, out_path);
    }

    println!("\nNote: Larger max_points_per_node reduces hierarchy overhead but trades query granularity.");
    Ok(())
}

fn usage_error() -> Error {
    Error::Projection(
        "Usage: cargo run -p wblidar --example copc_compression_tuning -- <input.las> <output_prefix>"
            .to_string(),
    )
}
