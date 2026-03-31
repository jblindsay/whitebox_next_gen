use std::env;
use std::fs::File;
use std::io::BufReader;

use wblidar::io::PointReader;
use wblidar::las::reader::LasReader;
use wblidar::{Error, PointRecord, Result};

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let a_path = args.next().ok_or_else(usage_error)?;
    let b_path = args.next().ok_or_else(usage_error)?;

    let mut a = LasReader::new(BufReader::new(File::open(&a_path).map_err(Error::Io)?))?;
    let mut b = LasReader::new(BufReader::new(File::open(&b_path).map_err(Error::Io)?))?;

    let mut pa = PointRecord::default();
    let mut pb = PointRecord::default();
    let mut idx: u64 = 0;
    let mut va: Vec<PointKey> = Vec::new();
    let mut vb: Vec<PointKey> = Vec::new();
    let mut first_mismatch_reported = false;
    let mut mismatch_count: u64 = 0;
    let mut scan_mismatch_count: u64 = 0;
    let mut other_field_mismatch_count: u64 = 0;

    loop {
        let ha = a.read_point(&mut pa)?;
        let hb = b.read_point(&mut pb)?;

        if !ha && !hb {
            let mut sa = va;
            let mut sb = vb;
            sa.sort_unstable();
            sb.sort_unstable();
            let unordered_equal = sa == sb;
            println!(
                "equal=true points={idx} unordered_equal={unordered_equal} mismatches={} scan_only_mismatches={} other_mismatches={}",
                mismatch_count,
                scan_mismatch_count,
                other_field_mismatch_count
            );
            return Ok(());
        }

        if ha != hb {
            println!("equal=false reason=length_mismatch at_index={idx} a_has={ha} b_has={hb}");
            return Ok(());
        }

        if !same_point(&pa, &pb) {
            mismatch_count += 1;
            if same_except_scan_angle(&pa, &pb) {
                scan_mismatch_count += 1;
            } else {
                other_field_mismatch_count += 1;
            }

            if mismatch_count <= 20 {
                println!(
                    "mismatch#{mismatch_count} index={idx} a_scan={} b_scan={} a_gps={:?} b_gps={:?}",
                    pa.scan_angle,
                    pb.scan_angle,
                    pa.gps_time,
                    pb.gps_time
                );
            }

            if !first_mismatch_reported {
                println!("equal=false reason=point_mismatch at_index={idx}");
                println!(
                    "a: x={} y={} z={} rn={} nr={} class={} flags={} scan={} user={} ps={} gps={:?}",
                    pa.x,
                    pa.y,
                    pa.z,
                    pa.return_number,
                    pa.number_of_returns,
                    pa.classification,
                    pa.flags,
                    pa.scan_angle,
                    pa.user_data,
                    pa.point_source_id,
                    pa.gps_time
                );
                println!(
                    "b: x={} y={} z={} rn={} nr={} class={} flags={} scan={} user={} ps={} gps={:?}",
                    pb.x,
                    pb.y,
                    pb.z,
                    pb.return_number,
                    pb.number_of_returns,
                    pb.classification,
                    pb.flags,
                    pb.scan_angle,
                    pb.user_data,
                    pb.point_source_id,
                    pb.gps_time
                );
                first_mismatch_reported = true;
            }
        }

        va.push(PointKey::from_point(&pa));
        vb.push(PointKey::from_point(&pb));

        idx += 1;
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PointKey {
    x_bits: u64,
    y_bits: u64,
    z_bits: u64,
    return_number: u8,
    number_of_returns: u8,
    classification: u8,
    flags: u8,
    scan_angle: i16,
    user_data: u8,
    point_source_id: u16,
    gps_bits: u64,
}

impl PointKey {
    fn from_point(p: &PointRecord) -> Self {
        Self {
            x_bits: p.x.to_bits(),
            y_bits: p.y.to_bits(),
            z_bits: p.z.to_bits(),
            return_number: p.return_number,
            number_of_returns: p.number_of_returns,
            classification: p.classification,
            flags: p.flags,
            scan_angle: p.scan_angle,
            user_data: p.user_data,
            point_source_id: p.point_source_id,
            gps_bits: p.gps_time.map(|v| v.0.to_bits()).unwrap_or(0),
        }
    }
}

fn same_point(a: &PointRecord, b: &PointRecord) -> bool {
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

    fn same_except_scan_angle(a: &PointRecord, b: &PointRecord) -> bool {
        a.x.to_bits() == b.x.to_bits()
        && a.y.to_bits() == b.y.to_bits()
        && a.z.to_bits() == b.z.to_bits()
        && a.return_number == b.return_number
        && a.number_of_returns == b.number_of_returns
        && a.classification == b.classification
        && a.flags == b.flags
        && a.user_data == b.user_data
        && a.point_source_id == b.point_source_id
        && a.gps_time.map(|v| v.0.to_bits()) == b.gps_time.map(|v| v.0.to_bits())
    }

fn usage_error() -> Error {
    Error::Projection(
        "Usage: cargo run -p wblidar --example compare_las_points -- <a.las> <b.las>".to_string(),
    )
}