//! Generate a representative LAZ/COPC artifact bundle for all LAS PDRFs (0-10).
//!
//! Usage:
//!   cargo run -p wblidar --release --example generate_all_pdrf_artifact_bundle -- <input.{las|laz}> [output_dir]
//!
//! Output:
//!   - <output_dir>/<stem>_pdrfN.laz (N = 0..10)
//!   - <output_dir>/<stem>_pdrfN.copc.laz (requested input format N for COPC)
//!   - <output_dir>/bundle_manifest.csv (requested/actual summary)

use std::env;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use wblidar::copc::{CopcWriter, CopcWriterConfig};
use wblidar::io::{PointReader, PointWriter};
use wblidar::las::reader::LasReader;
use wblidar::las::writer::WriterConfig;
use wblidar::las::PointDataFormat;
use wblidar::laz::{LazReader, LazWriter, LazWriterConfig};
use wblidar::point::{GpsTime, PointRecord, Rgb16, WaveformPacket};
use wblidar::{Error, Result};

const ALL_PDRFS: [PointDataFormat; 11] = [
    PointDataFormat::Pdrf0,
    PointDataFormat::Pdrf1,
    PointDataFormat::Pdrf2,
    PointDataFormat::Pdrf3,
    PointDataFormat::Pdrf4,
    PointDataFormat::Pdrf5,
    PointDataFormat::Pdrf6,
    PointDataFormat::Pdrf7,
    PointDataFormat::Pdrf8,
    PointDataFormat::Pdrf9,
    PointDataFormat::Pdrf10,
];

fn choose_standards_laz_target_pdrf(requested: PointDataFormat) -> PointDataFormat {
    use PointDataFormat::*;
    match requested {
        Pdrf9 => Pdrf6,
        Pdrf10 => Pdrf7,
        _ => requested,
    }
}

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let in_path = args.next().ok_or_else(usage_error)?;
    let in_path_ref = Path::new(&in_path);

    let out_dir = if let Some(dir) = args.next() {
        PathBuf::from(dir)
    } else {
        default_output_dir(in_path_ref)
    };
    fs::create_dir_all(&out_dir).map_err(Error::Io)?;

    let (header, crs) = {
        let meta_reader = LasReader::new(BufReader::new(File::open(&in_path).map_err(Error::Io)?))?;
        (meta_reader.header().clone(), meta_reader.crs().cloned())
    };

    println!("Input: {}", in_path);
    println!(
        "  declared points: {} | source PDRF {} | extra bytes/point {}",
        header.point_count(),
        header.point_data_format as u8,
        header.extra_bytes_count
    );

    let source_points = read_all_points(&in_path)?;
    println!("  streamed points: {}", source_points.len());

    if source_points.is_empty() {
        return Err(Error::Projection("Input cloud has zero points".to_string()));
    }

    let (center_x, center_y, center_z, halfsize) = compute_copc_cube(&source_points);
    println!(
        "  COPC cube center ({center_x:.3}, {center_y:.3}, {center_z:.3}) halfsize={halfsize:.3}"
    );

    let stem = in_path_ref
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| Error::Projection("Could not derive input stem".to_string()))?
        .to_string();

    let mut manifest_rows = Vec::with_capacity(ALL_PDRFS.len());

    for pdrf in ALL_PDRFS {
        println!("\nGenerating requested PDRF {} artifacts...", pdrf as u8);
        let adapted = adapt_points_for_pdrf(&source_points, pdrf);

        let laz_name = format!("{stem}_pdrf{}.laz", pdrf as u8);
        let laz_path = out_dir.join(&laz_name);
        let laz_target = choose_standards_laz_target_pdrf(pdrf);
        write_laz(
            &laz_path,
            laz_target,
            &adapted,
            &header,
            crs.clone(),
            "wblidar example: generate_all_pdrf_artifact_bundle",
        )?;
        let actual_laz_pdrf = read_header_pdrf(&laz_path)?;

        let copc_name = format!("{stem}_pdrf{}.copc.laz", pdrf as u8);
        let copc_path = out_dir.join(&copc_name);
        write_copc(
            &copc_path,
            pdrf,
            &adapted,
            &header,
            crs.clone(),
            center_x,
            center_y,
            center_z,
            halfsize,
            "wblidar example: generate_all_pdrf_artifact_bundle",
        )?;

        let actual_copc_pdrf = read_header_pdrf(&copc_path)?;
        println!(
            "  LAZ requested PDRF {} -> header PDRF {}",
            pdrf as u8,
            actual_laz_pdrf as u8
        );
        println!(
            "  COPC requested PDRF {} -> header PDRF {}",
            pdrf as u8,
            actual_copc_pdrf as u8
        );

        let qgis_safe = !actual_laz_pdrf.has_waveform();
        println!(
            "  QGIS-safe: {}",
            if qgis_safe { "yes" } else { "NO (waveform PDRF — QGIS/PDAL will reject)" }
        );

        manifest_rows.push((
            pdrf as u8,
            laz_name,
            copc_name,
            actual_laz_pdrf as u8,
            actual_copc_pdrf as u8,
            qgis_safe,
            adapted.len() as u64,
        ));
    }

    let manifest_path = out_dir.join("bundle_manifest.csv");
    write_manifest(&manifest_path, &manifest_rows)?;

    println!("\nBundle generation complete.");
    println!("  output dir: {}", out_dir.display());
    println!("  manifest:   {}", manifest_path.display());
    Ok(())
}

fn default_output_dir(in_path: &Path) -> PathBuf {
    let stem = in_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("source");
    PathBuf::from("artifacts").join(format!("{stem}_all_pdrf_bundle"))
}

fn read_all_points(in_path: &str) -> Result<Vec<PointRecord>> {
    let mut points = Vec::<PointRecord>::new();
    let mut p = PointRecord::default();
    let lower = in_path.to_ascii_lowercase();

    if lower.ends_with(".laz") {
        let mut reader = LazReader::new(BufReader::new(File::open(in_path).map_err(Error::Io)?))?;
        while reader.read_point(&mut p)? {
            points.push(p);
        }
    } else {
        let mut reader = LasReader::new(BufReader::new(File::open(in_path).map_err(Error::Io)?))?;
        while reader.read_point(&mut p)? {
            points.push(p);
        }
    }

    Ok(points)
}

fn adapt_points_for_pdrf(points: &[PointRecord], pdrf: PointDataFormat) -> Vec<PointRecord> {
    points
        .iter()
        .enumerate()
        .map(|(idx, src)| adapt_point_for_pdrf(src, pdrf, idx as u64))
        .collect()
}

fn adapt_point_for_pdrf(src: &PointRecord, pdrf: PointDataFormat, idx: u64) -> PointRecord {
    let mut out = *src;

    if pdrf.has_gps_time() {
        if out.gps_time.is_none() {
            out.gps_time = Some(GpsTime(idx as f64 * 0.01));
        }
    } else {
        out.gps_time = None;
    }

    if pdrf.has_rgb() {
        if out.color.is_none() {
            let base = (idx as u16).wrapping_mul(17);
            out.color = Some(Rgb16 {
                red: 10_000u16.wrapping_add(base),
                green: 20_000u16.wrapping_add(base),
                blue: 30_000u16.wrapping_add(base),
            });
        }
    } else {
        out.color = None;
    }

    if pdrf.has_nir() {
        if out.nir.is_none() {
            out.nir = Some(40_000u16.wrapping_add((idx as u16).wrapping_mul(7)));
        }
    } else {
        out.nir = None;
    }

    if pdrf.has_waveform() {
        if out.waveform.is_none() {
            out.waveform = Some(WaveformPacket {
                descriptor_index: 1,
                byte_offset: idx,
                packet_size: 32,
                return_point_location: 0.5,
                dx: 0.0,
                dy: 0.0,
                dz: 0.0,
            });
        }
    } else {
        out.waveform = None;
    }

    out
}

fn compute_copc_cube(points: &[PointRecord]) -> (f64, f64, f64, f64) {
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
    let halfsize = ((max_x - min_x)
        .max(max_y - min_y)
        .max(max_z - min_z)
        * 0.5)
        .max(1.0);

    (center_x, center_y, center_z, halfsize)
}

fn write_laz(
    out_path: &Path,
    pdrf: PointDataFormat,
    points: &[PointRecord],
    source_header: &wblidar::las::header::LasHeader,
    crs: Option<wblidar::Crs>,
    generator_name: &str,
) -> Result<()> {
    let mut cfg = LazWriterConfig::default();
    cfg.chunk_size = 50_000;
    cfg.compression_level = 6;
    cfg.las = WriterConfig {
        point_data_format: pdrf,
        x_scale: source_header.x_scale,
        y_scale: source_header.y_scale,
        z_scale: source_header.z_scale,
        x_offset: source_header.x_offset,
        y_offset: source_header.y_offset,
        z_offset: source_header.z_offset,
        system_identifier: source_header.system_identifier.clone(),
        generating_software: generator_name.to_string(),
        vlrs: Vec::new(),
        crs,
        extra_bytes_per_point: source_header.extra_bytes_count,
    };

    let out = File::create(out_path).map_err(Error::Io)?;
    let mut writer = LazWriter::new(BufWriter::new(out), cfg)?;
    for p in points {
        writer.write_point(p)?;
    }
    writer.finish()?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_copc(
    out_path: &Path,
    requested_pdrf: PointDataFormat,
    points: &[PointRecord],
    source_header: &wblidar::las::header::LasHeader,
    crs: Option<wblidar::Crs>,
    center_x: f64,
    center_y: f64,
    center_z: f64,
    halfsize: f64,
    generator_name: &str,
) -> Result<()> {
    let las_cfg = WriterConfig {
        point_data_format: requested_pdrf,
        x_scale: source_header.x_scale,
        y_scale: source_header.y_scale,
        z_scale: source_header.z_scale,
        x_offset: source_header.x_offset,
        y_offset: source_header.y_offset,
        z_offset: source_header.z_offset,
        system_identifier: source_header.system_identifier.clone(),
        generating_software: generator_name.to_string(),
        vlrs: Vec::new(),
        crs,
        extra_bytes_per_point: source_header.extra_bytes_count,
    };

    let cfg = CopcWriterConfig {
        las: las_cfg,
        center_x,
        center_y,
        center_z,
        halfsize,
        spacing: (halfsize * 2.0 / 1024.0).max(0.000_001),
        max_depth: 8,
        max_points_per_node: 50_000,
        compression_level: 6,
    };

    let out = File::create(out_path).map_err(Error::Io)?;
    let mut writer = CopcWriter::new(BufWriter::new(out), cfg);
    for p in points {
        writer.write_point(p)?;
    }
    writer.finish()?;
    Ok(())
}

fn read_header_pdrf(path: &Path) -> Result<PointDataFormat> {
    let reader = LasReader::new(BufReader::new(File::open(path).map_err(Error::Io)?))?;
    Ok(reader.header().point_data_format)
}

fn write_manifest(
    manifest_path: &Path,
    rows: &[(u8, String, String, u8, u8, bool, u64)],
) -> Result<()> {
    let mut f = File::create(manifest_path).map_err(Error::Io)?;
    writeln!(
        f,
        "requested_pdrf,laz_file,copc_file,laz_header_pdrf,copc_header_pdrf,qgis_safe,point_count"
    )
    .map_err(Error::Io)?;

    for (requested, laz_file, copc_file, actual_laz_pdrf, actual_copc_pdrf, qgis_safe, point_count) in rows {
        writeln!(
            f,
            "{requested},{laz_file},{copc_file},{actual_laz_pdrf},{actual_copc_pdrf},{qgis_safe},{point_count}"
        )
        .map_err(Error::Io)?;
    }

    Ok(())
}

fn usage_error() -> Error {
    Error::Projection(
        "Usage: cargo run -p wblidar --release --example generate_all_pdrf_artifact_bundle -- <input.{las|laz}> [output_dir]"
            .to_string(),
    )
}