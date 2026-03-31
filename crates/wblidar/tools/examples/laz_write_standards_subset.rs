//! Convert the first N points of an input LAS/LAZ to standards-compliant LAZ.
//!
//! Uses a conservative chunk size for external viewer compatibility testing.
//!
//! Usage:
//!   cargo run -p wblidar --example laz_write_standards_subset -- <input.{las|laz}> <output.laz> [max_points]

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
        Pdrf0 | Pdrf1 | Pdrf2 | Pdrf3 | Pdrf6 | Pdrf7 | Pdrf8 => src,
        Pdrf4 | Pdrf9 => Pdrf6,
        Pdrf5 | Pdrf10 => Pdrf7,
    }
}

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let in_path = args.next().ok_or_else(usage_error)?;
    let out_path = args.next().ok_or_else(usage_error)?;
    let max_points = args
        .next()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(100_000);

    let (header, crs) = {
        let reader = LasReader::new(BufReader::new(File::open(&in_path).map_err(Error::Io)?))?;
        (reader.header().clone(), reader.crs().cloned())
    };

    let src_pdrf = header.point_data_format;
    let dst_pdrf = choose_standards_target_pdrf(src_pdrf);

    let mut cfg = LazWriterConfig::default();
    cfg.standards_compliant = true;
    cfg.chunk_size = 2;
    cfg.las = WriterConfig {
        point_data_format: dst_pdrf,
        x_scale: header.x_scale,
        y_scale: header.y_scale,
        z_scale: header.z_scale,
        x_offset: header.x_offset,
        y_offset: header.y_offset,
        z_offset: header.z_offset,
        system_identifier: header.system_identifier.clone(),
        generating_software: "wblidar example: laz_write_standards_subset".to_string(),
        vlrs: Vec::new(),
        crs,
        extra_bytes_per_point: header.extra_bytes_count,
    };

    println!("Input:  {}", in_path);
    println!(
        "  PDRF {} -> PDRF {} | source declared points {}",
        src_pdrf as u8,
        dst_pdrf as u8,
        header.point_count()
    );
    println!("  writing up to {} points", max_points);

    let mut writer = LazWriter::new(File::create(&out_path).map_err(Error::Io)?, cfg)?;

    let mut p = PointRecord::default();
    let mut written = 0u64;
    if in_path.to_ascii_lowercase().ends_with(".laz") {
        let mut reader = LazReader::new(BufReader::new(File::open(&in_path).map_err(Error::Io)?))?;
        while written < max_points && reader.read_point(&mut p)? {
            writer.write_point(&p)?;
            written += 1;
        }
    } else {
        let mut reader = LasReader::new(BufReader::new(File::open(&in_path).map_err(Error::Io)?))?;
        while written < max_points && reader.read_point(&mut p)? {
            writer.write_point(&p)?;
            written += 1;
        }
    }

    writer.finish()?;

    println!("Output: {}", out_path);
    println!("  written points = {}", written);
    println!("Done.");
    Ok(())
}

fn usage_error() -> Error {
    Error::Projection(
        "Usage: cargo run -p wblidar --example laz_write_standards_subset -- <input.{las|laz}> <output.laz> [max_points]"
            .to_string(),
    )
}
