/// Debug the exact encoder behavior for chunk 51 tail points (970-975).
use std::fs::File;
use std::io::BufReader;

use wblidar::io::PointReader;
use wblidar::las::header::PointDataFormat;
use wblidar::las::reader::LasReader;
use wblidar::laz::fields::point14::RawPoint14;

fn main() -> std::io::Result<()> {
    let las_file = File::open("/tmp/carlos_chunk51_from_source.las")?;
    let mut las = LasReader::new(BufReader::new(las_file)).map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::Other, "Failed to open LAS")
    })?;
    let hdr = las.header().clone();
    let fmt = hdr.point_data_format;

    let mut points = Vec::new();
    let mut point = wblidar::PointRecord::default();
    while las.read_point(&mut point).map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::Other, "Failed to read point")
    })? {
        let mut scaled = point;
        scaled.x = ((point.x - hdr.x_offset) / hdr.x_scale).round();
        scaled.y = ((point.y - hdr.y_offset) / hdr.y_scale).round();
        scaled.z = ((point.z - hdr.z_offset) / hdr.z_scale).round();
        points.push(scaled);
    }

    println!("Total points: {}", points.len());
    println!();

    // Convert to RawPoint14
    let raws: Vec<RawPoint14> = points
        .iter()
        .copied()
        .map(|p| {
            RawPoint14::from_point_record(p, fmt, [1.0, 1.0, 1.0], [0.0, 0.0, 0.0])
                .unwrap()
        })
        .collect();

    // Compute change flags
    let mut scan_changes = Vec::new();
    let mut gps_changes = Vec::new();
    let mut last_scan = raws[0].scan_angle;
    let mut last_gps = raws[0].gps_time.to_bits() as i64;

    for (i, raw) in raws.iter().skip(1).enumerate() {
        scan_changes.push(raw.scan_angle != last_scan);
        gps_changes.push((raw.gps_time.to_bits() as i64) != last_gps);
        last_scan = raw.scan_angle;
        last_gps = raw.gps_time.to_bits() as i64;
    }

    // Print tail 10 points (965-974)
    println!("=== Tail 10 points (965-974) ===");
    for pi in 965..975.min(raws.len()) {
        let i = pi - 1; // Index in change arrays
        let raw = &raws[pi];
        let scan_ch = if i < scan_changes.len() {
            scan_changes[i]
        } else {
            false
        };
        let gps_ch = if i < gps_changes.len() {
            gps_changes[i]
        } else {
            false
        };
        let corr = raw.scan_angle as i32 - raws[pi - 1].scan_angle as i32;

        println!(
            "Point[{}]: scan={:6} scan_change={} gps_change={} corr={:6}",
            pi, raw.scan_angle, scan_ch, gps_ch, corr
        );
    }

    println!();
    println!("=== Change flag analysis ===");
    println!("scan_changes length: {}", scan_changes.len());
    println!("gps_changes length: {}", gps_changes.len());
    println!("scan_changes for indices 969-973:");
    for i in 969..974.min(scan_changes.len()) {
        println!("  [{}] = {}", i, scan_changes[i]);
    }

    Ok(())
}
