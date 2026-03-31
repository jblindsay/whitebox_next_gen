//! Convert an uncompressed LAS 1.4 file into a COPC (.copc.laz) file.
//!
//! Usage:
//!   cargo run -p wblidar --example las_to_copc -- <input.las> <output.copc.laz>
//!
//! Useful for testing COPC output with real-world point data rather than
//! synthetic fixtures.

use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter};

use wblidar::copc::{CopcWriter, CopcWriterConfig};
use wblidar::io::{PointReader, PointWriter};
use wblidar::las::reader::LasReader;
use wblidar::las::writer::WriterConfig;
use wblidar::{Error, PointRecord, Result};

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let in_path  = args.next().ok_or_else(usage_error)?;
    let out_path = args.next().ok_or_else(usage_error)?;

    // ── Pass 1: inspect header and compute bounds from streamed points ───
    let in_file  = File::open(&in_path).map_err(Error::Io)?;
    let mut reader = LasReader::new(BufReader::new(in_file))?;
    let hdr  = reader.header().clone();
    let crs  = reader.crs().cloned();

    println!("Input:  {in_path}");
    println!("  PDRF {} | {} points | scale {},{},{} | offset {},{},{}",
        hdr.point_data_format as u8,
        hdr.point_count_64.unwrap_or(hdr.legacy_point_count as u64),
        hdr.x_scale, hdr.y_scale, hdr.z_scale,
        hdr.x_offset, hdr.y_offset, hdr.z_offset,
    );
    if let Some(c) = &crs {
        println!("  CRS: EPSG:{:?}", c.epsg);
    } else {
        println!("  CRS: none");
    }

    let mut buf = PointRecord::default();
    let mut min_x = f64::INFINITY;  let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;  let mut max_y = f64::NEG_INFINITY;
    let mut min_z = f64::INFINITY;  let mut max_z = f64::NEG_INFINITY;
    let mut n_read: u64 = 0;
    while reader.read_point(&mut buf)? {
        min_x = min_x.min(buf.x);  max_x = max_x.max(buf.x);
        min_y = min_y.min(buf.y);  max_y = max_y.max(buf.y);
        min_z = min_z.min(buf.z);  max_z = max_z.max(buf.z);
        n_read += 1;
    }
    println!("  Read {n_read} points (pass 1)");

    // ── Compute COPC cube bounds from observed extents ────────────────────
    let center_x = (min_x + max_x) * 0.5;
    let center_y = (min_y + max_y) * 0.5;
    let center_z = (min_z + max_z) * 0.5;
    let halfsize = ((max_x - min_x).max(max_y - min_y).max(max_z - min_z) * 0.5).max(1.0);
    let spacing  = (halfsize * 2.0 / 1024.0).max(0.000_001);

    println!("  Cube center ({center_x},{center_y},{center_z}) halfsize={halfsize} spacing={spacing:.6}");

    // ── Write COPC output ─────────────────────────────────────────────────
    let las_cfg = WriterConfig {
        point_data_format: hdr.point_data_format,
        x_scale: hdr.x_scale,
        y_scale: hdr.y_scale,
        z_scale: hdr.z_scale,
        x_offset: hdr.x_offset,
        y_offset: hdr.y_offset,
        z_offset: hdr.z_offset,
        system_identifier: hdr.system_identifier.clone(),
        generating_software: "wblidar example: las_to_copc".to_string(),
        vlrs: Vec::new(),
        crs,
        extra_bytes_per_point: 0,
    };

    let cfg = CopcWriterConfig {
        las: las_cfg,
        center_x, center_y, center_z,
        halfsize, spacing,
        ..CopcWriterConfig::default()
    };

    let out = BufWriter::new(File::create(&out_path).map_err(Error::Io)?);
    let mut writer = CopcWriter::new(out, cfg);

    // ── Pass 2: stream points into COPC writer ────────────────────────────
    let in_file_2 = File::open(&in_path).map_err(Error::Io)?;
    let mut reader_2 = LasReader::new(BufReader::new(in_file_2))?;
    let mut written: u64 = 0;
    while reader_2.read_point(&mut buf)? {
        writer.write_point(&buf)?;
        written += 1;
    }
    writer.finish()?;
    println!("  Wrote {written} points (pass 2)");

    println!("Output: {out_path}");
    println!("Done.");
    Ok(())
}

fn usage_error() -> Error {
    Error::Projection(
        "Usage: cargo run -p wblidar --example las_to_copc -- <input.las> <output.copc.laz>"
            .to_string(),
    )
}
