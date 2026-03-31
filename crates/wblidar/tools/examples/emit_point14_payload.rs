use std::env;
use std::fs::File;
use std::io::{BufReader, Write};

use wblidar::io::PointReader;
use wblidar::las::reader::LasReader;
use wblidar::laz::standard_point14::encode_standard_layered_chunk_point14_v3_constant_attributes;
use wblidar::{Error, PointRecord, Result};

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let in_path = args.next().ok_or_else(usage_error)?;
    let out_path = args.next().ok_or_else(usage_error)?;

    let mut reader = LasReader::new(BufReader::new(File::open(&in_path).map_err(Error::Io)?))?;
    let hdr = reader.header().clone();

    let mut points = Vec::<PointRecord>::new();
    let mut p = PointRecord::default();
    while reader.read_point(&mut p)? {
        let mut scaled = p;
        scaled.x = ((p.x - hdr.x_offset) / hdr.x_scale).round();
        scaled.y = ((p.y - hdr.y_offset) / hdr.y_scale).round();
        scaled.z = ((p.z - hdr.z_offset) / hdr.z_scale).round();
        points.push(scaled);
    }

    if points.is_empty() {
        return Err(Error::InvalidValue {
            field: "emit_point14_payload.input",
            detail: "input LAS contains zero points".to_string(),
        });
    }

    let bytes = encode_standard_layered_chunk_point14_v3_constant_attributes(
        &points,
        hdr.point_data_format,
        [1.0, 1.0, 1.0],
        [0.0, 0.0, 0.0],
    )?;

    let mut out = File::create(&out_path).map_err(Error::Io)?;
    out.write_all(&bytes).map_err(Error::Io)?;
    println!("points={} payload_bytes={}", points.len(), bytes.len());
    println!("output={out_path}");
    Ok(())
}

fn usage_error() -> Error {
    Error::Projection(
        "Usage: cargo run -p wblidar --example emit_point14_payload -- <input.las> <output.bin>"
            .to_string(),
    )
}