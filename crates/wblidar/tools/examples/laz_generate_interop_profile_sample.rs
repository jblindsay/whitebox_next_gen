//! Generate a standards-compliant LAZ sample using the same point pattern
//! as the external interoperability harness.
//!
//! Usage:
//!   cargo run -p wblidar --example laz_generate_interop_profile_sample -- <output.laz>

use std::env;
use std::fs::File;

use wblidar::io::PointWriter;
use wblidar::las::header::PointDataFormat;
use wblidar::las::writer::WriterConfig;
use wblidar::laz::{LazWriter, LazWriterConfig};
use wblidar::point::{GpsTime, PointRecord, Rgb16};
use wblidar::{Error, Result};

fn build_point(pdrf: PointDataFormat, idx: u16, extra_bytes: u16) -> PointRecord {
    let mut point = PointRecord {
        x: f64::from(idx),
        y: 100.0 + f64::from(idx),
        z: 200.0 + f64::from(idx),
        intensity: 1000 + idx,
        classification: (idx % 5) as u8,
        return_number: 1,
        number_of_returns: 1,
        user_data: (idx % 255) as u8,
        point_source_id: 500 + idx,
        ..PointRecord::default()
    };

    if pdrf.has_gps_time() {
        point.gps_time = Some(GpsTime(10.0 + f64::from(idx)));
    }

    if pdrf.has_rgb() {
        point.color = Some(Rgb16 {
            red: 100 + idx,
            green: 200 + idx,
            blue: 300 + idx,
        });
    }

    if extra_bytes > 0 {
        for i in 0..usize::from(extra_bytes) {
            point.extra_bytes.data[i] = (u16::try_from(i).unwrap_or(0) as u8).wrapping_add(idx as u8);
        }
        point.extra_bytes.len = extra_bytes as u8;
    }

    point
}

fn main() -> Result<()> {
    let out_path = env::args().nth(1).ok_or_else(usage_error)?;

    let mut cfg = LazWriterConfig::default();
    cfg.standards_compliant = true;
    cfg.chunk_size = 2;
    cfg.las = WriterConfig {
        point_data_format: PointDataFormat::Pdrf3,
        extra_bytes_per_point: 2,
        generating_software: "wblidar example: laz_generate_interop_profile_sample".to_string(),
        ..WriterConfig::default()
    };

    let mut writer = LazWriter::new(File::create(&out_path)?, cfg)?;
    for idx in 0..3u16 {
        let point = build_point(PointDataFormat::Pdrf3, idx, 2);
        writer.write_point(&point)?;
    }
    writer.finish()?;

    println!("Generated standards-compliant LAZ interop-profile sample:");
    println!("  path = {}", out_path);
    println!("  pdrf = 3");
    println!("  points = 3");
    println!("  extra_bytes_per_point = 2");
    Ok(())
}

fn usage_error() -> Error {
    Error::Projection(
        "Usage: cargo run -p wblidar --example laz_generate_interop_profile_sample -- <output.laz>"
            .to_string(),
    )
}
