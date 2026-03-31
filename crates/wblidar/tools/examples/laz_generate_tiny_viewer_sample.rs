//! Generate a tiny standards-compliant LAZ sample for external viewer checks.
//!
//! Usage:
//!   cargo run -p wblidar --example laz_generate_tiny_viewer_sample -- <output.laz>

use std::env;
use std::fs::File;

use wblidar::io::PointWriter;
use wblidar::las::header::PointDataFormat;
use wblidar::las::writer::WriterConfig;
use wblidar::laz::{LazWriter, LazWriterConfig};
use wblidar::point::{GpsTime, PointRecord, Rgb16};
use wblidar::{Error, Result};

fn main() -> Result<()> {
    let out_path = env::args().nth(1).ok_or_else(usage_error)?;

    let mut cfg = LazWriterConfig::default();
    cfg.standards_compliant = true;
    cfg.chunk_size = 2;
    cfg.las = WriterConfig {
        point_data_format: PointDataFormat::Pdrf3,
        generating_software: "wblidar example: laz_generate_tiny_viewer_sample".to_string(),
        ..WriterConfig::default()
    };

    let mut writer = LazWriter::new(File::create(&out_path)?, cfg)?;

    let pts = [
        PointRecord {
            x: 100.0,
            y: 100.0,
            z: 10.0,
            intensity: 800,
            classification: 2,
            gps_time: Some(GpsTime(1.0)),
            color: Some(Rgb16 { red: 65535, green: 0, blue: 0 }),
            ..PointRecord::default()
        },
        PointRecord {
            x: 101.0,
            y: 100.5,
            z: 10.5,
            intensity: 1200,
            classification: 1,
            gps_time: Some(GpsTime(2.0)),
            color: Some(Rgb16 { red: 0, green: 65535, blue: 0 }),
            ..PointRecord::default()
        },
        PointRecord {
            x: 102.0,
            y: 101.0,
            z: 11.0,
            intensity: 1600,
            classification: 1,
            gps_time: Some(GpsTime(3.0)),
            color: Some(Rgb16 { red: 0, green: 0, blue: 65535 }),
            ..PointRecord::default()
        },
    ];

    for p in &pts {
        writer.write_point(p)?;
    }
    writer.finish()?;

    println!("Generated tiny standards-compliant LAZ sample:");
    println!("  path = {}", out_path);
    println!("  pdrf = 3 (Point10 + GPSTime + RGB)");
    println!("  points = {}", pts.len());
    Ok(())
}

fn usage_error() -> Error {
    Error::Projection(
        "Usage: cargo run -p wblidar --example laz_generate_tiny_viewer_sample -- <output.laz>"
            .to_string(),
    )
}
