//! Generate a small standards-compliant LAZ file for external viewer smoke tests.
//!
//! Usage:
//!   cargo run -p wblidar --example laz_generate_viewer_sample -- <output.laz>

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
    cfg.chunk_size = 512;
    cfg.las = WriterConfig {
        point_data_format: PointDataFormat::Pdrf8,
        extra_bytes_per_point: 2,
        x_scale: 0.001,
        y_scale: 0.001,
        z_scale: 0.001,
        generating_software: "wblidar example: laz_generate_viewer_sample".to_string(),
        ..WriterConfig::default()
    };

    let mut writer = LazWriter::new(File::create(&out_path)?, cfg)?;

    // Keep point count within one chunk for stable external viewer smoke tests.
    let nx = 16u16;
    let ny = 16u16;
    let mut idx = 0u32;
    for row in 0..ny {
        for col in 0..nx {
            let x = f64::from(col) * 0.5;
            let y = f64::from(row) * 0.5;
            let z = (x * 0.08) + (y * 0.03);

            let mut p = PointRecord {
                x,
                y,
                z,
                intensity: 1200u16.saturating_add((row + col) * 3),
                classification: if (row + col) % 9 == 0 { 2 } else { 1 },
                return_number: 1,
                number_of_returns: 1,
                user_data: (idx % 255) as u8,
                point_source_id: 42,
                gps_time: Some(GpsTime(1000.0 + f64::from(idx) * 0.0025)),
                color: Some(Rgb16 {
                    red: 500 + col * 20,
                    green: 700 + row * 18,
                    blue: 300 + ((row + col) * 8),
                }),
                nir: Some(400 + ((row * 2 + col) % 120)),
                ..PointRecord::default()
            };

            p.extra_bytes.data[0] = (row % 256) as u8;
            p.extra_bytes.data[1] = (col % 256) as u8;
            p.extra_bytes.len = 2;

            writer.write_point(&p)?;
            idx += 1;
        }
    }

    writer.finish()?;

    println!("Generated standards-compliant LAZ sample:");
    println!("  path = {}", out_path);
    println!("  pdrf = 8 (Point14 + RGB + NIR)");
    println!("  points = {}", u32::from(nx) * u32::from(ny));
    println!("  extra_bytes_per_point = 2");
    Ok(())
}

fn usage_error() -> Error {
    Error::Projection(
        "Usage: cargo run -p wblidar --example laz_generate_viewer_sample -- <output.laz>"
            .to_string(),
    )
}
