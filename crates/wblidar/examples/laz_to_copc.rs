//! Convert a compressed LAZ file into a COPC (.copc.laz) file.
//!
//! The source may be any LAS 1.x point-data format; points are promoted to the
//! nearest Point14-family format supported by COPC (PDRF6/7/8).
//!
//! PDRF promotion mapping:
//!   PDRF0/1     → PDRF6   (GPS time carried where present; zero-filled otherwise)
//!   PDRF2/3/4/5 → PDRF7   (RGB carried where present; waveform packets discarded)
//!   PDRF6/7/8   → same    (already Point14-family, no promotion needed)
//!
//! Usage:
//!   cargo run -p wblidar --example laz_to_copc -- <input.laz> <output.copc.laz>

use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter};

use wblidar::copc::{CopcWriter, CopcWriterConfig};
use wblidar::io::{PointReader, PointWriter};
use wblidar::las::header::PointDataFormat;
use wblidar::las::reader::LasReader;
use wblidar::las::writer::WriterConfig;
use wblidar::laz::reader::LazReader;
use wblidar::{Error, PointRecord, Result};

/// Promote a source PDRF to the nearest COPC-compatible Point14-family target.
fn promote_pdrf(src: PointDataFormat) -> PointDataFormat {
    use PointDataFormat::*;
    match src {
        Pdrf0 | Pdrf1 => Pdrf6,
        Pdrf2 | Pdrf3 | Pdrf4 | Pdrf5 => Pdrf7,
        Pdrf6 | Pdrf7 | Pdrf8 => src,
        _ => Pdrf6,
    }
}

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let in_path  = args.next().ok_or_else(usage_error)?;
    let out_path = args.next().ok_or_else(usage_error)?;

    // ── Inspect LAS header (header bytes are uncompressed in any LAZ file) ──
    let hdr = {
        let r = LasReader::new(BufReader::new(File::open(&in_path).map_err(Error::Io)?))?;
        r.header().clone()
    };
    let crs = {
        let r = LasReader::new(BufReader::new(File::open(&in_path).map_err(Error::Io)?))?;
        r.crs().cloned()
    };

    let src_pdrf = hdr.point_data_format;
    let dst_pdrf = promote_pdrf(src_pdrf);

    println!("Input:  {in_path}");
    println!("  PDRF {} → PDRF {} | {} points | scale {},{},{} | offset {},{},{}",
        src_pdrf as u8, dst_pdrf as u8,
        hdr.point_count_64.unwrap_or(hdr.legacy_point_count as u64),
        hdr.x_scale, hdr.y_scale, hdr.z_scale,
        hdr.x_offset, hdr.y_offset, hdr.z_offset,
    );
    if let Some(c) = &crs {
        println!("  CRS: EPSG:{:?}", c.epsg);
    } else {
        println!("  CRS: none");
    }

    // ── Pass 1: compute observed bounds from streamed points ──────────────
    let mut buf = PointRecord::default();
    let mut min_x = f64::INFINITY;  let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;  let mut max_y = f64::NEG_INFINITY;
    let mut min_z = f64::INFINITY;  let mut max_z = f64::NEG_INFINITY;
    let mut n_read: u64 = 0;
    {
        let mut reader = LazReader::new(BufReader::new(File::open(&in_path).map_err(Error::Io)?))?;
        while reader.read_point(&mut buf)? {
            min_x = min_x.min(buf.x);  max_x = max_x.max(buf.x);
            min_y = min_y.min(buf.y);  max_y = max_y.max(buf.y);
            min_z = min_z.min(buf.z);  max_z = max_z.max(buf.z);
            n_read += 1;
        }
    }
    println!("  Read {n_read} points (pass 1)");
    println!("  Bounds X [{min_x:.2} .. {max_x:.2}] Y [{min_y:.2} .. {max_y:.2}] Z [{min_z:.2} .. {max_z:.2}]");

    // ── Compute COPC octree cube bounds ───────────────────────────────────
    let center_x = (min_x + max_x) * 0.5;
    let center_y = (min_y + max_y) * 0.5;
    let center_z = (min_z + max_z) * 0.5;
    let halfsize  = ((max_x - min_x).max(max_y - min_y).max(max_z - min_z) * 0.5).max(1.0);
    let spacing   = (halfsize * 2.0 / 1024.0).max(0.000_001);
    println!("  Cube center ({center_x:.2},{center_y:.2},{center_z:.2}) halfsize={halfsize:.2} spacing={spacing:.6}");

    // ── Configure and open COPC writer ────────────────────────────────────
    let las_cfg = WriterConfig {
        point_data_format: dst_pdrf,
        x_scale: hdr.x_scale,
        y_scale: hdr.y_scale,
        z_scale: hdr.z_scale,
        x_offset: hdr.x_offset,
        y_offset: hdr.y_offset,
        z_offset: hdr.z_offset,
        system_identifier: hdr.system_identifier.clone(),
        generating_software: "wblidar example: laz_to_copc".to_string(),
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
    let mut reader = LazReader::new(BufReader::new(File::open(&in_path).map_err(Error::Io)?))?;
    let mut written: u64 = 0;
    while reader.read_point(&mut buf)? {
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
        "Usage: cargo run -p wblidar --example laz_to_copc -- <input.laz> <output.copc.laz>"
            .to_string(),
    )
}
