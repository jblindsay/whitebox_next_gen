use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter};

use wblidar::copc::{CopcWriter, CopcWriterConfig};
use wblidar::io::{PointReader, PointWriter};
use wblidar::las::{LasReader, LasWriter, WriterConfig};
use wblidar::reproject::points_in_place_to_epsg;
use wblidar::{Crs, Error, PointRecord, Result};

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let input_las = args.next().ok_or_else(usage_error)?;
    let output_las = args.next().ok_or_else(usage_error)?;
    let dst_epsg: u32 = args
        .next()
        .ok_or_else(usage_error)?
        .parse()
        .map_err(|_| usage_error())?;
    let output_copc = args.next();
    let src_epsg_override = args.next().map(|s| s.parse::<u32>()).transpose().map_err(|_| usage_error())?;

    let input = BufReader::new(File::open(&input_las)?);
    let mut reader = LasReader::new(input)?;

    let mut points = Vec::<PointRecord>::new();
    let mut p = PointRecord::default();
    while reader.read_point(&mut p)? {
        points.push(p);
    }

    let mut src_crs = reader.crs().cloned().unwrap_or_default();
    if src_crs.epsg.is_none() {
        if let Some(src_epsg) = src_epsg_override {
            src_crs = Crs::from_epsg(src_epsg);
        }
    }

    points_in_place_to_epsg(&mut points, &mut src_crs, dst_epsg)?;

    let hdr = reader.header().clone();
    let las_cfg = WriterConfig {
        point_data_format: hdr.point_data_format,
        x_scale: hdr.x_scale,
        y_scale: hdr.y_scale,
        z_scale: hdr.z_scale,
        x_offset: hdr.x_offset,
        y_offset: hdr.y_offset,
        z_offset: hdr.z_offset,
        system_identifier: hdr.system_identifier,
        generating_software: "wblidar example: lidar_reproject".to_string(),
        vlrs: Vec::new(),
        crs: Some(src_crs.clone()),
        extra_bytes_per_point: hdr.extra_bytes_count,
    };

    let out_las = BufWriter::new(File::create(&output_las)?);
    let mut las_writer = LasWriter::new(out_las, las_cfg.clone())?;
    for point in &points {
        las_writer.write_point(point)?;
    }
    las_writer.finish()?;

    if let Some(copc_path) = output_copc {
        let (min_x, max_x, min_y, max_y, min_z, max_z) = bounds(&points);
        let center_x = (min_x + max_x) * 0.5;
        let center_y = (min_y + max_y) * 0.5;
        let center_z = (min_z + max_z) * 0.5;
        let halfsize = ((max_x - min_x).max(max_y - min_y).max(max_z - min_z) * 0.5).max(1.0);
        let spacing = (halfsize * 2.0 / 1024.0).max(0.000_001);

        let copc_cfg = CopcWriterConfig {
            las: las_cfg,
            center_x,
            center_y,
            center_z,
            halfsize,
            spacing,
            ..CopcWriterConfig::default()
        };

        let out_copc = BufWriter::new(File::create(copc_path)?);
        let mut copc_writer = CopcWriter::new(out_copc, copc_cfg);
        for point in &points {
            copc_writer.write_point(point)?;
        }
        copc_writer.finish()?;
    }

    Ok(())
}

fn usage_error() -> Error {
    Error::Projection(
        "Usage: cargo run --example lidar_reproject -- <input.las> <output.las> <dst_epsg> [output.copc.las] [src_epsg_if_input_has_no_crs]"
            .to_string(),
    )
}

fn bounds(points: &[PointRecord]) -> (f64, f64, f64, f64, f64, f64) {
    if points.is_empty() {
        return (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    }

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

    (min_x, max_x, min_y, max_y, min_z, max_z)
}
