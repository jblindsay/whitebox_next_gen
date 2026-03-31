//! Generate fresh LAZ/COPC test artifacts after cleanup.
//! Uses standards-compliant LASzip v2/v3 encoding (no legacy wb-native).
//!
//! Usage:
//!   cargo run -p wblidar --release --example generate_clean_test_artifacts -- <input.las>
//!
//! Generates:
//!   - LAZ Point10 (PDRF0, 50k chunks)
//!   - LAZ Point14 (PDRF7, 50k chunks)
//!   - COPC Point14 (50k nodes, Morton ordering)
//!   - COPC Point14 (100k nodes, Morton ordering)

use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter};

use wblidar::copc::{CopcWriter, CopcWriterConfig};
use wblidar::io::{PointReader, PointWriter};
use wblidar::las::reader::LasReader;
use wblidar::laz::LazReader;
use wblidar::laz::{LazWriter, LazWriterConfig};
use wblidar::las::writer::WriterConfig;
use wblidar::las::PointDataFormat;
use wblidar::{Error, PointRecord, Result};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <input.las>", args[0]);
        return Ok(());
    }

    let in_path = &args[1];
    println!("Generating clean test artifacts from: {in_path}\n");

    // Read source file and collect metadata
    let in_file = File::open(in_path).map_err(Error::Io)?;
    let mut reader = LasReader::new(BufReader::new(in_file))?;
    let hdr = reader.header().clone();
    let crs = reader.crs().cloned();

    println!(
        "Input: {} points, PDRF{}, scale ({:.6}, {:.6}, {:.6})",
        hdr.point_count_64.unwrap_or(hdr.legacy_point_count as u64),
        hdr.point_data_format as u8,
        hdr.x_scale, hdr.y_scale, hdr.z_scale
    );
    println!("CRS: {:?}\n", crs);

    // Read all points into memory
    let mut all_points = Vec::new();
    let mut buf = PointRecord::default();
        let is_laz = in_path.to_ascii_lowercase().ends_with(".laz");
        if is_laz {
            let mut laz_reader = LazReader::new(BufReader::new(File::open(in_path).map_err(Error::Io)?))?;
            while laz_reader.read_point(&mut buf)? {
                all_points.push(buf.clone());
            }
        } else {
            while reader.read_point(&mut buf)? {
                all_points.push(buf.clone());
            }
    }
    println!("✓ Read {} points from source\n", all_points.len());

    // ── Generate LAZ Point10 (PDRF0, standards-compliant, 50k chunks) ───
    {
        let out_path = "artifacts/ponui_clean_pdrf0_point10_50k.laz";
        println!("Generating: {}", out_path);

        let mut laz_cfg = LazWriterConfig::default();
        laz_cfg.las.point_data_format = PointDataFormat::Pdrf0;
        laz_cfg.las.x_scale = hdr.x_scale;
        laz_cfg.las.y_scale = hdr.y_scale;
        laz_cfg.las.z_scale = hdr.z_scale;
        laz_cfg.las.x_offset = hdr.x_offset;
        laz_cfg.las.y_offset = hdr.y_offset;
        laz_cfg.las.z_offset = hdr.z_offset;
        laz_cfg.las.system_identifier = hdr.system_identifier.clone();
        laz_cfg.las.generating_software = "wblidar: generate_clean_test_artifacts".to_string();
        laz_cfg.las.crs = crs.clone();
        laz_cfg.chunk_size = 50_000;
        laz_cfg.compression_level = 6;

        {
            let out_file = File::create(out_path).map_err(Error::Io)?;
            let mut writer = LazWriter::new(BufWriter::new(out_file), laz_cfg)?;
            for pt in &all_points {
                writer.write_point(pt)?;
            }
            writer.finish()?;
        }

        let size_mb = std::fs::metadata(out_path)
            .ok()
            .map(|m| m.len() as f64 / 1_048_576.0)
            .unwrap_or(0.0);
        println!("  → {:.2} MB\n", size_mb);
    }

    // ── Generate LAZ Point14 (PDRF7, standards-compliant, 50k chunks) ───
    {
        let out_path = "artifacts/ponui_clean_pdrf7_point14_50k.laz";
        println!("Generating: {}", out_path);

        let mut laz_cfg = LazWriterConfig::default();
        laz_cfg.las.point_data_format = PointDataFormat::Pdrf7;
        laz_cfg.las.x_scale = hdr.x_scale;
        laz_cfg.las.y_scale = hdr.y_scale;
        laz_cfg.las.z_scale = hdr.z_scale;
        laz_cfg.las.x_offset = hdr.x_offset;
        laz_cfg.las.y_offset = hdr.y_offset;
        laz_cfg.las.z_offset = hdr.z_offset;
        laz_cfg.las.system_identifier = hdr.system_identifier.clone();
        laz_cfg.las.generating_software = "wblidar: generate_clean_test_artifacts".to_string();
        laz_cfg.las.crs = crs.clone();
        laz_cfg.chunk_size = 50_000;
        laz_cfg.compression_level = 6;

        {
            let out_file = File::create(out_path).map_err(Error::Io)?;
            let mut writer = LazWriter::new(BufWriter::new(out_file), laz_cfg)?;
            for pt in &all_points {
                writer.write_point(pt)?;
            }
            writer.finish()?;
        }

        let size_mb = std::fs::metadata(out_path)
            .ok()
            .map(|m| m.len() as f64 / 1_048_576.0)
            .unwrap_or(0.0);
        println!("  → {:.2} MB\n", size_mb);
    }

    // Compute COPC bounds
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut min_z = f64::INFINITY;
    let mut max_z = f64::NEG_INFINITY;

    for pt in &all_points {
        min_x = min_x.min(pt.x);
        max_x = max_x.max(pt.x);
        min_y = min_y.min(pt.y);
        max_y = max_y.max(pt.y);
        min_z = min_z.min(pt.z);
        max_z = max_z.max(pt.z);
    }

    let center_x = (min_x + max_x) * 0.5;
    let center_y = (min_y + max_y) * 0.5;
    let center_z = (min_z + max_z) * 0.5;
    let halfsize = ((max_x - min_x).max(max_y - min_y).max(max_z - min_z) * 0.5).max(1.0);

    println!(
        "COPC bounds: center ({:.2}, {:.2}, {:.2}), halfsize={:.2}\n",
        center_x, center_y, center_z, halfsize
    );

    // ── Generate COPC (Point14, 50k nodes per octree node) ───
    {
        let out_path = "artifacts/ponui_clean_copc_pdrf7_50k_morton.copc.laz";
        println!("Generating: {} (50k nodes)", out_path);

        let las_cfg = WriterConfig {
            point_data_format: PointDataFormat::Pdrf7,
            x_scale: hdr.x_scale,
            y_scale: hdr.y_scale,
            z_scale: hdr.z_scale,
            x_offset: hdr.x_offset,
            y_offset: hdr.y_offset,
            z_offset: hdr.z_offset,
            system_identifier: hdr.system_identifier.clone(),
            generating_software: "wblidar: generate_clean_test_artifacts".to_string(),
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
            spacing: (halfsize * 2.0 / 1024.0).max(0.000_001),
            max_depth: 8,
            max_points_per_node: 50_000,
                    compression_level: 6,
        };

        {
            let out_file = File::create(out_path).map_err(Error::Io)?;
            let mut writer = CopcWriter::new(BufWriter::new(out_file), copc_cfg);
            for pt in &all_points {
                writer.write_point(pt)?;
            }
            writer.finish()?;
        }

        let size_mb = std::fs::metadata(out_path)
            .ok()
            .map(|m| m.len() as f64 / 1_048_576.0)
            .unwrap_or(0.0);
        println!("  → {:.2} MB\n", size_mb);
    }

    // ── Generate COPC (Point14, 100k nodes per octree node) ───
    {
        let out_path = "artifacts/ponui_clean_copc_pdrf7_100k_morton.copc.laz";
        println!("Generating: {} (100k nodes)", out_path);

        let las_cfg = WriterConfig {
            point_data_format: PointDataFormat::Pdrf7,
            x_scale: hdr.x_scale,
            y_scale: hdr.y_scale,
            z_scale: hdr.z_scale,
            x_offset: hdr.x_offset,
            y_offset: hdr.y_offset,
            z_offset: hdr.z_offset,
            system_identifier: hdr.system_identifier.clone(),
            generating_software: "wblidar: generate_clean_test_artifacts".to_string(),
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
            spacing: (halfsize * 2.0 / 1024.0).max(0.000_001),
            max_depth: 8,
            max_points_per_node: 100_000,
                    compression_level: 6,
        };

        {
            let out_file = File::create(out_path).map_err(Error::Io)?;
            let mut writer = CopcWriter::new(BufWriter::new(out_file), copc_cfg);
            for pt in &all_points {
                writer.write_point(pt)?;
            }
            writer.finish()?;
        }

        let size_mb = std::fs::metadata(out_path)
            .ok()
            .map(|m| m.len() as f64 / 1_048_576.0)
            .unwrap_or(0.0);
        println!("  → {:.2} MB\n", size_mb);
    }

    println!("✓ All clean test artifacts generated successfully!");
    println!("  Ready for validation: PDAL, QGIS, CloudCompare\n");
    
    Ok(())
}
