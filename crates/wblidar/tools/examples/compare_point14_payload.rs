use std::env;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};

use wblidar::io::PointReader;
use wblidar::las::header::PointDataFormat;
use wblidar::las::reader::LasReader;
use wblidar::laz::fields::point14::RawPoint14;
use wblidar::laz::standard_point14::{ 
    decode_standard_layered_chunk_point14_v3,
    encode_standard_layered_chunk_point14_v3_constant_attributes,
};
use wblidar::laz::LaszipItemSpec;
use wblidar::{Error, PointRecord, Result};

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let las_path = args.next().ok_or_else(usage_error)?;
    let laz_path = args.next().ok_or_else(usage_error)?;
    let ours_payload_path = args.next();

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

    let mut dbg_last_scan = points[0].scan_angle;
    let mut dbg_last_gps = points[0].gps_time.map(|v| v.0.to_bits() as i64).unwrap_or(0);
    let mut dbg_scan_changes = Vec::with_capacity(points.len().saturating_sub(1));
    let mut dbg_gps_changes = Vec::with_capacity(points.len().saturating_sub(1));
    for p in points.iter().skip(1) {
        let gps_bits = p.gps_time.map(|v| v.0.to_bits() as i64).unwrap_or(0);
        let scan_change = p.scan_angle != dbg_last_scan;
        let gps_change = gps_bits != dbg_last_gps;
        dbg_scan_changes.push(scan_change);
        dbg_gps_changes.push(gps_change);
        dbg_last_scan = p.scan_angle;
        dbg_last_gps = gps_bits;
    }
    println!(
        "tail_scan_values={:?}",
        points
            .iter()
            .rev()
            .take(6)
            .map(|p| p.scan_angle)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
    );
    println!(
        "tail_scan_changes={:?}",
        dbg_scan_changes
            .iter()
            .rev()
            .take(6)
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
    );
    println!(
        "tail_gps_changes={:?}",
        dbg_gps_changes
            .iter()
            .rev()
            .take(6)
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
    );

    let raws: Vec<RawPoint14> = points
        .iter()
        .copied()
        .map(|p| RawPoint14::from_point_record(p, fmt, [1.0, 1.0, 1.0], [0.0, 0.0, 0.0]).unwrap())
        .collect();
    println!(
        "tail_raw_scan_values={:?}",
        raws
            .iter()
            .rev()
            .take(6)
            .map(|r| r.scan_angle)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
    );

    let ours = encode_standard_layered_chunk_point14_v3_constant_attributes(
        &points,
        fmt,
        [1.0, 1.0, 1.0],
        [0.0, 0.0, 0.0],
    )?;

    if let Some(path) = ours_payload_path {
        std::fs::write(&path, &ours)?;
        println!("wrote_ours_payload={}", path);
    }

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
    println!("equal={}", ours == external);
    print_header("ours", &ours, fmt.core_size().into());
    print_header("external", &external, fmt.core_size().into());
    println!("ours_prefix={}", hex_prefix(&ours, 64));
    println!("external_prefix={}", hex_prefix(&external, 64));

    let mismatch = ours.iter().zip(external.iter()).position(|(a, b)| a != b);
    match mismatch {
        Some(i) => println!("first_mismatch_byte={}", i),
        None if ours.len() == external.len() => println!("first_mismatch_byte=none"),
        None => println!("first_mismatch_byte=prefix_equal"),
    }

    let item_specs = item_specs_for_format(fmt);
    let ours_decoded = decode_standard_layered_chunk_point14_v3(
        &ours,
        points.len(),
        &item_specs,
        fmt,
        [1.0, 1.0, 1.0],
        [0.0, 0.0, 0.0],
    )?;
    let external_decoded = decode_standard_layered_chunk_point14_v3(
        &external,
        points.len(),
        &item_specs,
        fmt,
        [1.0, 1.0, 1.0],
        [0.0, 0.0, 0.0],
    )?;

    let first_decoded_mismatch = ours_decoded
        .iter()
        .zip(external_decoded.iter())
        .position(|(a, b)| !same_core_point(a, b));
    match first_decoded_mismatch {
        Some(i) => {
            println!("first_decoded_mismatch_index={}", i);
            println!("ours_decoded_scan={}", ours_decoded[i].scan_angle);
            println!("external_decoded_scan={}", external_decoded[i].scan_angle);
            println!("ours_decoded_gps={:?}", ours_decoded[i].gps_time);
            println!("external_decoded_gps={:?}", external_decoded[i].gps_time);
        }
        None => println!("first_decoded_mismatch_index=none"),
    }

    let ours_vs_source = ours_decoded
        .iter()
        .zip(points.iter())
        .position(|(a, b)| !same_core_point(a, b));
    let external_vs_source = external_decoded
        .iter()
        .zip(points.iter())
        .position(|(a, b)| !same_core_point(a, b));
    println!("ours_vs_source_first_mismatch={:?}", ours_vs_source);
    println!("external_vs_source_first_mismatch={:?}", external_vs_source);

    Ok(())
}

fn hex_prefix(bytes: &[u8], max_len: usize) -> String {
    bytes.iter()
        .take(max_len)
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join("")
}

fn print_header(label: &str, bytes: &[u8], point_len: usize) {
    if bytes.len() < point_len + 40 {
        println!("{}_header=truncated", label);
        return;
    }
    let tail = &bytes[point_len..];
    let fields: Vec<u32> = (0..10)
        .map(|i| {
            let start = i * 4;
            u32::from_le_bytes([
                tail[start],
                tail[start + 1],
                tail[start + 2],
                tail[start + 3],
            ])
        })
        .collect();
    println!(
        "{}_header=count={} xy={} z={} class={} flags={} intensity={} scan={} user={} point_src={} gps={}",
        label,
        fields[0],
        fields[1],
        fields[2],
        fields[3],
        fields[4],
        fields[5],
        fields[6],
        fields[7],
        fields[8],
        fields[9],
    );
}

fn item_specs_for_format(fmt: PointDataFormat) -> Vec<LaszipItemSpec> {
    let mut items = vec![LaszipItemSpec {
        item_type: 10,
        item_size: fmt.core_size() as u16,
        item_version: 3,
    }];

    match fmt {
        PointDataFormat::Pdrf7 => {
            items.push(LaszipItemSpec {
                item_type: 11,
                item_size: 6,
                item_version: 3,
            });
        }
        PointDataFormat::Pdrf8 => {
            items.push(LaszipItemSpec {
                item_type: 11,
                item_size: 6,
                item_version: 3,
            });
            items.push(LaszipItemSpec {
                item_type: 12,
                item_size: 2,
                item_version: 3,
            });
        }
        _ => {}
    }

    items
}

fn same_core_point(a: &PointRecord, b: &PointRecord) -> bool {
    a.x.to_bits() == b.x.to_bits()
        && a.y.to_bits() == b.y.to_bits()
        && a.z.to_bits() == b.z.to_bits()
        && a.return_number == b.return_number
        && a.number_of_returns == b.number_of_returns
        && a.classification == b.classification
        && a.flags == b.flags
        && a.scan_angle == b.scan_angle
        && a.user_data == b.user_data
        && a.point_source_id == b.point_source_id
        && a.gps_time.map(|v| v.0.to_bits()) == b.gps_time.map(|v| v.0.to_bits())
}

fn usage_error() -> Error {
    Error::Projection(
        "Usage: cargo run -p wblidar --example compare_point14_payload -- <input.las> <reference.laz> [ours_payload.bin]"
            .to_string(),
    )
}