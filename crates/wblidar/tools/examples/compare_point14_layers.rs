use std::env;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};

use wblidar::io::PointReader;
use wblidar::las::reader::LasReader;
use wblidar::laz::standard_point14::encode_standard_layered_chunk_point14_v3_constant_attributes;
use wblidar::{Error, PointRecord, Result};

#[derive(Debug, Clone)]
struct LayerSlices<'a> {
    xy: &'a [u8],
    z: &'a [u8],
    classification: &'a [u8],
    flags: &'a [u8],
    intensity: &'a [u8],
    scan_angle: &'a [u8],
    user_data: &'a [u8],
    point_source: &'a [u8],
    gps_time: &'a [u8],
}

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let las_path = args.next().ok_or_else(usage_error)?;
    let laz_path = args.next().ok_or_else(usage_error)?;

    let las_file = File::open(&las_path)?;
    let mut las = LasReader::new(BufReader::new(las_file))?;
    let hdr = las.header().clone();
    let fmt = hdr.point_data_format;

    let mut points = Vec::<PointRecord>::new();
    let mut point = PointRecord::default();
    while las.read_point(&mut point)? {
        let mut scaled = point;
        scaled.x = ((point.x - hdr.x_offset) / hdr.x_scale).round();
        scaled.y = ((point.y - hdr.y_offset) / hdr.y_scale).round();
        scaled.z = ((point.z - hdr.z_offset) / hdr.z_scale).round();
        points.push(scaled);
    }

    let ours = encode_standard_layered_chunk_point14_v3_constant_attributes(
        &points,
        fmt,
        [1.0, 1.0, 1.0],
        [0.0, 0.0, 0.0],
    )?;

    let mut laz = File::open(&laz_path)?;
    laz.seek(SeekFrom::Start(96))?;
    let mut u32buf = [0u8; 4];
    laz.read_exact(&mut u32buf)?;
    let offset_to_point_data = u32::from_le_bytes(u32buf) as u64;

    laz.seek(SeekFrom::Start(offset_to_point_data))?;
    let mut u64buf = [0u8; 8];
    laz.read_exact(&mut u64buf)?;
    let chunk_table_offset = u64::from_le_bytes(u64buf);
    let payload_start = offset_to_point_data + 8;
    let payload_len = chunk_table_offset.saturating_sub(payload_start) as usize;
    laz.seek(SeekFrom::Start(payload_start))?;
    let mut external = vec![0u8; payload_len];
    laz.read_exact(&mut external)?;

    println!("points={}", points.len());
    println!("ours_len={}", ours.len());
    println!("external_len={}", external.len());

    let seed_len = fmt.core_size() as usize;
    let ours_layers = split_layers(&ours, seed_len)?;
    let external_layers = split_layers(&external, seed_len)?;

    compare_layer("xy", ours_layers.xy, external_layers.xy);
    compare_layer("z", ours_layers.z, external_layers.z);
    compare_layer("classification", ours_layers.classification, external_layers.classification);
    compare_layer("flags", ours_layers.flags, external_layers.flags);
    compare_layer("intensity", ours_layers.intensity, external_layers.intensity);
    compare_layer("scan_angle", ours_layers.scan_angle, external_layers.scan_angle);
    compare_layer("user_data", ours_layers.user_data, external_layers.user_data);
    compare_layer("point_source", ours_layers.point_source, external_layers.point_source);
    compare_layer("gps_time", ours_layers.gps_time, external_layers.gps_time);

    Ok(())
}

fn compare_layer(name: &str, ours: &[u8], external: &[u8]) {
    let first_mismatch = ours.iter().zip(external.iter()).position(|(a, b)| a != b);
    let equal = ours == external;
    println!(
        "layer={} ours_len={} external_len={} equal={}",
        name,
        ours.len(),
        external.len(),
        equal
    );
    if let Some(i) = first_mismatch {
        let start = i.saturating_sub(8);
        let end = (i + 12).min(ours.len()).min(external.len());
        println!("  first_mismatch={}", i);
        println!("  ours_window={}", hex_prefix(&ours[start..end]));
        println!("  external_window={}", hex_prefix(&external[start..end]));
    }
}

fn split_layers(payload: &[u8], seed_len: usize) -> Result<LayerSlices<'_>> {
    if payload.len() < seed_len + 40 {
        return Err(Error::InvalidValue {
            field: "examples.compare_point14_layers.header",
            detail: "payload too small for seed+header".to_string(),
        });
    }

    let tail = &payload[seed_len..];
    let len_xy = read_u32_le_at(tail, 4)? as usize;
    let len_z = read_u32_le_at(tail, 8)? as usize;
    let len_classification = read_u32_le_at(tail, 12)? as usize;
    let len_flags = read_u32_le_at(tail, 16)? as usize;
    let len_intensity = read_u32_le_at(tail, 20)? as usize;
    let len_scan_angle = read_u32_le_at(tail, 24)? as usize;
    let len_user_data = read_u32_le_at(tail, 28)? as usize;
    let len_point_source = read_u32_le_at(tail, 32)? as usize;
    let len_gps_time = read_u32_le_at(tail, 36)? as usize;

    let mut off = 40usize;
    let xy = slice_layer(tail, &mut off, len_xy)?;
    let z = slice_layer(tail, &mut off, len_z)?;
    let classification = slice_layer(tail, &mut off, len_classification)?;
    let flags = slice_layer(tail, &mut off, len_flags)?;
    let intensity = slice_layer(tail, &mut off, len_intensity)?;
    let scan_angle = slice_layer(tail, &mut off, len_scan_angle)?;
    let user_data = slice_layer(tail, &mut off, len_user_data)?;
    let point_source = slice_layer(tail, &mut off, len_point_source)?;
    let gps_time = slice_layer(tail, &mut off, len_gps_time)?;

    Ok(LayerSlices {
        xy,
        z,
        classification,
        flags,
        intensity,
        scan_angle,
        user_data,
        point_source,
        gps_time,
    })
}

fn slice_layer<'a>(tail: &'a [u8], off: &mut usize, len: usize) -> Result<&'a [u8]> {
    if *off + len > tail.len() {
        return Err(Error::InvalidValue {
            field: "examples.compare_point14_layers.payload",
            detail: "declared layer size exceeds payload tail".to_string(),
        });
    }
    let out = &tail[*off..*off + len];
    *off += len;
    Ok(out)
}

fn read_u32_le_at(bytes: &[u8], offset: usize) -> Result<u32> {
    if offset + 4 > bytes.len() {
        return Err(Error::InvalidValue {
            field: "examples.compare_point14_layers.header",
            detail: format!("u32 offset {} out of range", offset),
        });
    }
    Ok(u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ]))
}

fn hex_prefix(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join("")
}

fn usage_error() -> Error {
    Error::InvalidValue {
        field: "examples.compare_point14_layers.usage",
        detail: "Usage: cargo run -p wblidar --example compare_point14_layers -- <input.las> <reference.laz>".to_string(),
    }
}
