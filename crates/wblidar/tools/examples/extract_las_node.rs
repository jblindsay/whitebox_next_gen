use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter};

use wblidar::copc::VoxelKey;
use wblidar::io::{PointReader, PointWriter};
use wblidar::las::reader::LasReader;
use wblidar::las::writer::{LasWriter, WriterConfig};
use wblidar::{Error, PointRecord, Result};

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let in_path = args.next().ok_or_else(usage_error)?;
    let level: i32 = args
        .next()
        .ok_or_else(usage_error)?
        .parse()
        .map_err(|_| usage_error())?;
    let x: i32 = args
        .next()
        .ok_or_else(usage_error)?
        .parse()
        .map_err(|_| usage_error())?;
    let y: i32 = args
        .next()
        .ok_or_else(usage_error)?
        .parse()
        .map_err(|_| usage_error())?;
    let z: i32 = args
        .next()
        .ok_or_else(usage_error)?
        .parse()
        .map_err(|_| usage_error())?;
    let out_path = args.next().ok_or_else(usage_error)?;
    let target = VoxelKey { level, x, y, z };

    let in_file = File::open(&in_path).map_err(Error::Io)?;
    let mut reader = LasReader::new(BufReader::new(in_file))?;
    let hdr = reader.header().clone();
    let crs = reader.crs().cloned();

    let mut buf = PointRecord::default();
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut min_z = f64::INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    while reader.read_point(&mut buf)? {
        min_x = min_x.min(buf.x);
        max_x = max_x.max(buf.x);
        min_y = min_y.min(buf.y);
        max_y = max_y.max(buf.y);
        min_z = min_z.min(buf.z);
        max_z = max_z.max(buf.z);
    }

    let center_x = (min_x + max_x) * 0.5;
    let center_y = (min_y + max_y) * 0.5;
    let center_z = (min_z + max_z) * 0.5;
    let halfsize = ((max_x - min_x).max(max_y - min_y).max(max_z - min_z) * 0.5).max(1.0);
    let spacing = (halfsize * 2.0 / 1024.0).max(0.000_001);

    let cfg = WriterConfig {
        point_data_format: hdr.point_data_format,
        x_scale: hdr.x_scale,
        y_scale: hdr.y_scale,
        z_scale: hdr.z_scale,
        x_offset: hdr.x_offset,
        y_offset: hdr.y_offset,
        z_offset: hdr.z_offset,
        system_identifier: hdr.system_identifier,
        generating_software: "extract_las_node".to_string(),
        vlrs: Vec::new(),
        crs,
        extra_bytes_per_point: hdr.extra_bytes_count,
    };

    let out = BufWriter::new(File::create(&out_path).map_err(Error::Io)?);
    let mut writer = LasWriter::new(out, cfg)?;

    let in_file = File::open(&in_path).map_err(Error::Io)?;
    let mut reader = LasReader::new(BufReader::new(in_file))?;
    let mut matched = 0u64;
    while reader.read_point(&mut buf)? {
        if classify_point(
            buf.x,
            buf.y,
            buf.z,
            center_x,
            center_y,
            center_z,
            halfsize,
            8,
            spacing,
        ) == target
        {
            writer.write_point(&buf)?;
            matched += 1;
        }
    }
    writer.finish()?;

    println!("target=({}, {}, {}, {})", target.level, target.x, target.y, target.z);
    println!("matched_points={matched}");
    println!("output={out_path}");
    Ok(())
}

fn classify_point(
    px: f64,
    py: f64,
    pz: f64,
    cx: f64,
    cy: f64,
    cz: f64,
    hs: f64,
    max_depth: u32,
    spacing: f64,
) -> VoxelKey {
    let mut lx = 0i32;
    let mut ly = 0i32;
    let mut lz = 0i32;
    let mut cx_cur = cx;
    let mut cy_cur = cy;
    let mut cz_cur = cz;
    let mut cur_hs = hs;
    let mut out = VoxelKey {
        level: 0,
        x: 0,
        y: 0,
        z: 0,
    };

    for level in 0..max_depth {
        if cur_hs * 2.0 <= spacing {
            break;
        }

        cur_hs *= 0.5;

        let nx = if px >= cx_cur { 1 } else { 0 };
        let ny = if py >= cy_cur { 1 } else { 0 };
        let nz = if pz >= cz_cur { 1 } else { 0 };

        lx = lx * 2 + nx;
        ly = ly * 2 + ny;
        lz = lz * 2 + nz;

        cx_cur += if nx == 1 { cur_hs } else { -cur_hs };
        cy_cur += if ny == 1 { cur_hs } else { -cur_hs };
        cz_cur += if nz == 1 { cur_hs } else { -cur_hs };

        out = VoxelKey {
            level: level as i32 + 1,
            x: lx,
            y: ly,
            z: lz,
        };
    }

    out
}

fn usage_error() -> Error {
    Error::Projection(
        "Usage: cargo run -p wblidar --example extract_las_node -- <input.las> <level> <x> <y> <z> <output.las>"
            .to_string(),
    )
}