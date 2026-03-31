//! Run a reproducible COPC parity benchmark matrix and emit CSV.
//!
//! Usage:
//!   cargo run -p wblidar --example copc_parity_benchmark_csv -- \
//!     <input.las> <output_prefix> [reference_qgis.copc.laz] [report.csv]

use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;

use wblidar::copc::reader::CopcReader;
use wblidar::copc::{CopcNodePointOrdering, CopcWriter, CopcWriterConfig};
use wblidar::io::{PointReader, PointWriter};
use wblidar::las::reader::LasReader;
use wblidar::las::writer::WriterConfig;
use wblidar::{Error, PointRecord, Result};

#[derive(Clone, Copy)]
struct BenchConfig {
    name: &'static str,
    max_points_per_node: usize,
    compression_level: u32,
}

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let in_path = args.next().ok_or_else(usage_error)?;
    let out_prefix = args.next().ok_or_else(usage_error)?;
    let qgis_ref = args.next();
    let csv_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(format!("{out_prefix}_parity_report.csv")));

    let source_size = std::fs::metadata(&in_path)?.len();

    let in_file = File::open(&in_path).map_err(Error::Io)?;
    let mut reader = LasReader::new(BufReader::new(in_file))?;
    let hdr = reader.header().clone();
    let crs = reader.crs().cloned();

    let mut p = PointRecord::default();
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut min_z = f64::INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    let mut total_points = 0u64;

    while reader.read_point(&mut p)? {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
        min_z = min_z.min(p.z);
        max_z = max_z.max(p.z);
        total_points += 1;
    }

    let center_x = (min_x + max_x) * 0.5;
    let center_y = (min_y + max_y) * 0.5;
    let center_z = (min_z + max_z) * 0.5;
    let halfsize = ((max_x - min_x).max(max_y - min_y).max(max_z - min_z) * 0.5).max(1.0);
    let spacing = (halfsize * 2.0 / 1024.0).max(0.000_001);

    let configs = [
        BenchConfig {
            name: "default_100k_lvl6",
            max_points_per_node: 100_000,
            compression_level: 6,
        },
        BenchConfig {
            name: "default_100k_lvl9",
            max_points_per_node: 100_000,
            compression_level: 9,
        },
        BenchConfig {
            name: "smaller_50k_lvl6",
            max_points_per_node: 50_000,
            compression_level: 6,
        },
        BenchConfig {
            name: "larger_200k_lvl6",
            max_points_per_node: 200_000,
            compression_level: 6,
        },
    ];

    let qgis_bytes = qgis_ref
        .as_ref()
        .and_then(|p| std::fs::metadata(p).ok())
        .map(|m| m.len());

    if let Some(parent) = csv_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let mut out = BufWriter::new(File::create(&csv_path)?);
    writeln!(
        out,
        "config,output_file,bytes,miB,source_bytes,ratio_vs_source,data_nodes,total_points,max_points_per_node,compression_level,qgis_ref_bytes,delta_vs_qgis_bytes,ratio_vs_qgis"
    )?;

    println!("Input: {in_path}");
    println!("  points={total_points} source_bytes={source_size}");

    for cfg in &configs {
        let out_path = format!("{out_prefix}_{}.copc.laz", cfg.name);

        let las_cfg = WriterConfig {
            point_data_format: hdr.point_data_format,
            x_scale: hdr.x_scale,
            y_scale: hdr.y_scale,
            z_scale: hdr.z_scale,
            x_offset: hdr.x_offset,
            y_offset: hdr.y_offset,
            z_offset: hdr.z_offset,
            system_identifier: hdr.system_identifier.clone(),
            generating_software: "wblidar: copc_parity_benchmark_csv".to_string(),
            vlrs: Vec::new(),
            crs: crs.clone(),
            extra_bytes_per_point: 0,
        };

        let copc_cfg = CopcWriterConfig {
            las: las_cfg,
            center_x,
            center_y,
            center_z,
            halfsize,
            spacing,
            max_depth: 8,
            max_points_per_node: cfg.max_points_per_node,
            node_point_ordering: CopcNodePointOrdering::Auto,
            compression_level: cfg.compression_level,
        };

        let mut writer = CopcWriter::new(BufWriter::new(File::create(&out_path)?), copc_cfg);
        let mut rd = LasReader::new(BufReader::new(File::open(&in_path)?))?;
        let mut tmp = PointRecord::default();
        while rd.read_point(&mut tmp)? {
            writer.write_point(&tmp)?;
        }
        writer.finish()?;

        let out_bytes = std::fs::metadata(&out_path)?.len();
        let r = CopcReader::new(BufReader::new(File::open(&out_path)?))?;
        let data_nodes = r
            .hierarchy
            .entries
            .iter()
            .filter(|e| e.point_count > 0 && e.byte_size > 0)
            .count();

        let (delta_vs_qgis, ratio_vs_qgis) = if let Some(q) = qgis_bytes {
            let delta = out_bytes as i128 - q as i128;
            let ratio = out_bytes as f64 / q.max(1) as f64;
            (delta.to_string(), format!("{ratio:.6}"))
        } else {
            (String::new(), String::new())
        };

        writeln!(
            out,
            "{},{},{},{:.6},{},{:.6},{},{},{},{},{},{},{}",
            cfg.name,
            out_path,
            out_bytes,
            out_bytes as f64 / (1024.0 * 1024.0),
            source_size,
            out_bytes as f64 / source_size.max(1) as f64,
            data_nodes,
            total_points,
            cfg.max_points_per_node,
            cfg.compression_level,
            qgis_bytes.map(|v| v.to_string()).unwrap_or_default(),
            delta_vs_qgis,
            ratio_vs_qgis,
        )?;

        println!(
            "  {} -> {} bytes ({:.3} MiB), data_nodes={}",
            cfg.name,
            out_bytes,
            out_bytes as f64 / (1024.0 * 1024.0),
            data_nodes
        );
    }

    out.flush()?;
    println!("Wrote report: {}", csv_path.display());
    Ok(())
}

fn usage_error() -> Error {
    Error::Projection(
        "Usage: cargo run -p wblidar --example copc_parity_benchmark_csv -- <input.las> <output_prefix> [reference_qgis.copc.laz] [report.csv]"
            .to_string(),
    )
}
