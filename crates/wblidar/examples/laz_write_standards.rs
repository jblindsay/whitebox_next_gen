//! Write a standards-compliant LAZ file.
//!
//! This example converts an input LAS/LAZ file into a LAZ file emitted through
//! `LazWriterConfig::standards_compliant = true`.
//!
//! Usage:
//!   cargo run -p wblidar --example laz_write_standards -- <input.{las|laz}> <output.laz>

use std::env;
use std::fs::File;
use std::io::BufReader;

use wblidar::io::{PointReader, PointWriter};
use wblidar::las::header::PointDataFormat;
use wblidar::las::reader::LasReader;
use wblidar::las::writer::WriterConfig;
use wblidar::laz::{LazReader, LazWriter, LazWriterConfig};
use wblidar::{Error, PointRecord, Result};

fn choose_standards_target_pdrf(src: PointDataFormat) -> PointDataFormat {
    use PointDataFormat::*;
    match src {
        // Standards writer supports these directly.
        Pdrf0 | Pdrf1 | Pdrf2 | Pdrf3 | Pdrf6 | Pdrf7 | Pdrf8 => src,
        // Waveform-bearing formats are promoted to nearest non-waveform standards target.
        Pdrf4 | Pdrf9 => Pdrf6,
        Pdrf5 | Pdrf10 => Pdrf7,
        // LAS 1.5+ extended formats: promote to base Point14.
        _ => Pdrf6,
    }
}

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let in_path = args.next().ok_or_else(usage_error)?;
    let out_path = args.next().ok_or_else(usage_error)?;

    let (header, crs) = {
        let reader = LasReader::new(BufReader::new(File::open(&in_path).map_err(Error::Io)?))?;
        (reader.header().clone(), reader.crs().cloned())
    };

    let src_pdrf = header.point_data_format;
    let dst_pdrf = choose_standards_target_pdrf(src_pdrf);

    let mut cfg = LazWriterConfig::default();
    cfg.standards_compliant = true;
    // Use default 50k chunk size for better compression across all Point formats.
    // (was 10k, which is too small; reduces compression context reuse)
    // cfg.chunk_size = cfg.chunk_size; // default is 50_000
    cfg.las = WriterConfig {
        point_data_format: dst_pdrf,
        x_scale: header.x_scale,
        y_scale: header.y_scale,
        z_scale: header.z_scale,
        x_offset: header.x_offset,
        y_offset: header.y_offset,
        z_offset: header.z_offset,
        system_identifier: header.system_identifier.clone(),
        generating_software: "wblidar example: laz_write_standards".to_string(),
        vlrs: Vec::new(),
        crs,
        extra_bytes_per_point: header.extra_bytes_count,
    };

    println!("Input:  {}", in_path);
    println!(
        "  PDRF {} -> PDRF {} | declared points {}",
        src_pdrf as u8,
        dst_pdrf as u8,
        header.point_count()
    );
    if src_pdrf != dst_pdrf {
        println!(
            "  note: source PDRF {} promoted to standards-compatible PDRF {}",
            src_pdrf as u8,
            dst_pdrf as u8
        );
    }
    println!(
        "  declared extra bytes per point = {}",
        header.extra_bytes_count
    );

    let out = File::create(&out_path).map_err(Error::Io)?;
    let mut writer = LazWriter::new(out, cfg)?;

    let mut p = PointRecord::default();
    let mut written = 0u64;
    let in_path_lower = in_path.to_ascii_lowercase();
    if in_path_lower.ends_with(".laz") {
        let mut reader = LazReader::new(BufReader::new(File::open(&in_path).map_err(Error::Io)?))?;
        while reader.read_point(&mut p)? {
            writer.write_point(&p)?;
            written += 1;
        }
    } else {
        let mut reader = LasReader::new(BufReader::new(File::open(&in_path).map_err(Error::Io)?))?;
        while reader.read_point(&mut p)? {
            writer.write_point(&p)?;
            written += 1;
        }
    }

    writer.finish()?;

    let mut verify_reader = LazReader::new(BufReader::new(File::open(&out_path).map_err(Error::Io)?))?;
    let mut verified = 0u64;
    while verify_reader.read_point(&mut p)? {
        verified += 1;
    }

    println!("Output: {}", out_path);
    println!("  written points = {}", written);
    println!("  verified points = {}", verified);
    println!("Done.");
    Ok(())
}

fn usage_error() -> Error {
    Error::Projection(
        "Usage: cargo run -p wblidar --example laz_write_standards -- <input.{las|laz}> <output.laz>"
            .to_string(),
    )
}
