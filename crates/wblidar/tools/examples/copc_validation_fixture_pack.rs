use std::env;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use wblidar::copc::{CopcWriter, CopcWriterConfig};
use wblidar::io::PointWriter;
use wblidar::las::header::PointDataFormat;
use wblidar::las::writer::WriterConfig;
use wblidar::las::writer::LasWriter;
use wblidar::{Crs, Error, PointRecord, Result, Rgb16};

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let out_dir = args.next().ok_or_else(usage_error)?;
    let args_rest: Vec<String> = args.collect();
    let no_crs  = args_rest.iter().any(|a| a == "--no-crs");
    let las_mode = args_rest.iter().any(|a| a == "--las");
    let out_dir = PathBuf::from(out_dir);
    fs::create_dir_all(&out_dir)?;

    if las_mode {
        let p6 = "fixture_pdrf6.las";
        let p7 = "fixture_pdrf7.las";
        let p8 = "fixture_pdrf8.las";
        write_las_fixture(&out_dir, p6, PointDataFormat::Pdrf6, !no_crs)?;
        write_las_fixture(&out_dir, p7, PointDataFormat::Pdrf7, !no_crs)?;
        write_las_fixture(&out_dir, p8, PointDataFormat::Pdrf8, !no_crs)?;
        println!("Wrote plain LAS 1.4 diagnostic fixtures to {}", out_dir.display());
        println!("- {p6}\n- {p7}\n- {p8}");
        println!("Load these in CloudCompare/QGIS to verify basic structure before testing LAZ.");
        return Ok(());
    }

    let p6 = fixture_name("fixture_pdrf6.copc.laz", no_crs);
    let p7 = fixture_name("fixture_pdrf7.copc.laz", no_crs);
    let p8 = fixture_name("fixture_pdrf8.copc.laz", no_crs);

    write_fixture(&out_dir, &p6, PointDataFormat::Pdrf6, !no_crs)?;
    write_fixture(&out_dir, &p7, PointDataFormat::Pdrf7, !no_crs)?;
    write_fixture(&out_dir, &p8, PointDataFormat::Pdrf8, !no_crs)?;

    println!("Wrote COPC validation fixtures to {}", out_dir.display());
    println!("- {p6}");
    println!("- {p7}");
    println!("- {p8}");
    println!("Upload each file to https://validate.copc.io as part of Milestone 3 validation.");

    Ok(())
}

fn fixture_name(base: &str, no_crs: bool) -> String {
    if !no_crs {
        return base.to_string();
    }
    base.replace(".copc.laz", "_nocrs.copc.laz")
}

fn write_las_fixture(
    out_dir: &Path,
    filename: &str,
    pdrf: PointDataFormat,
    include_crs: bool,
) -> Result<()> {
    let points = build_points(pdrf);
    let cfg = WriterConfig {
        point_data_format: pdrf,
        x_scale: 0.001,
        y_scale: 0.001,
        z_scale: 0.001,
        x_offset: 0.0,
        y_offset: 0.0,
        z_offset: 0.0,
        system_identifier: "wblidar-validation".to_string(),
        generating_software: "wblidar example: copc_validation_fixture_pack".to_string(),
        vlrs: Vec::new(),
        crs: if include_crs { Some(Crs::from_epsg(4326)) } else { None },
        extra_bytes_per_point: 0,
    };
    let out_path = out_dir.join(filename);
    let out = BufWriter::new(File::create(out_path)?);
    let mut writer = LasWriter::new(out, cfg)?;
    for p in &points {
        writer.write_point(p)?;
    }
    writer.finish()?;
    Ok(())
}

fn write_fixture(
    out_dir: &Path,
    filename: &str,
    pdrf: PointDataFormat,
    include_crs: bool,
) -> Result<()> {
    let points = build_points(pdrf);
    let (center_x, center_y, center_z, halfsize, spacing) = copc_bounds_params(&points);

    let las_cfg = WriterConfig {
        point_data_format: pdrf,
        x_scale: 0.001,
        y_scale: 0.001,
        z_scale: 0.001,
        x_offset: 0.0,
        y_offset: 0.0,
        z_offset: 0.0,
        system_identifier: "wblidar-validation".to_string(),
        generating_software: "wblidar example: copc_validation_fixture_pack".to_string(),
        vlrs: Vec::new(),
        crs: if include_crs { Some(Crs::from_epsg(4326)) } else { None },
        extra_bytes_per_point: 0,
    };

    let cfg = CopcWriterConfig {
        las: las_cfg,
        center_x,
        center_y,
        center_z,
        halfsize,
        spacing,
        ..CopcWriterConfig::default()
    };

    let out_path = out_dir.join(filename);
    let out = BufWriter::new(File::create(out_path)?);
    let mut writer = CopcWriter::new(out, cfg);
    for p in &points {
        writer.write_point(p)?;
    }
    writer.finish()?;

    Ok(())
}

fn build_points(pdrf: PointDataFormat) -> Vec<PointRecord> {
    let mut out = Vec::with_capacity(256);

    for i in 0..256u32 {
        let ix = (i % 16) as f64;
        let iy = (i / 16) as f64;

        let mut p = PointRecord {
            x: -80.0 + ix * 0.0004,
            y: 43.0 + iy * 0.0004,
            z: 100.0 + (i % 11) as f64 * 0.05,
            intensity: 1000u16.saturating_add((i % 200) as u16),
            classification: (2 + (i % 6)) as u8,
            user_data: (i % 255) as u8,
            scan_angle: ((i % 15) as i16) - 7,
            point_source_id: 10 + (i % 50) as u16,
            gps_time: Some(wblidar::GpsTime(1_000_000.0 + i as f64 * 0.125)),
            return_number: 1,
            number_of_returns: 1,
            flags: 0,
            ..PointRecord::default()
        };

        if matches!(pdrf, PointDataFormat::Pdrf7 | PointDataFormat::Pdrf8) {
            p.color = Some(Rgb16 {
                red: 1000 + (i % 100) as u16 * 10,
                green: 1500 + (i % 120) as u16 * 8,
                blue: 2000 + (i % 80) as u16 * 12,
            });
        }
        if matches!(pdrf, PointDataFormat::Pdrf8) {
            p.nir = Some(500 + (i % 150) as u16 * 4);
        }

        out.push(p);
    }

    out
}

fn copc_bounds_params(points: &[PointRecord]) -> (f64, f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut min_z = f64::INFINITY;
    let mut max_z = f64::NEG_INFINITY;

    for p in points {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
        min_z = min_z.min(p.z);
        max_z = max_z.max(p.z);
    }

    let center_x = (min_x + max_x) * 0.5;
    let center_y = (min_y + max_y) * 0.5;
    let center_z = (min_z + max_z) * 0.5;
    let halfsize = ((max_x - min_x).max(max_y - min_y).max(max_z - min_z) * 0.5).max(1.0);
    let spacing = (halfsize * 2.0 / 1024.0).max(0.000_001);

    (center_x, center_y, center_z, halfsize, spacing)
}

fn usage_error() -> Error {
    Error::Projection(
        "Usage: cargo run --example copc_validation_fixture_pack -- <output_dir> [--no-crs]".to_string(),
    )
}
