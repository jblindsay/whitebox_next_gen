//! Benchmark Point14 `compression_level` behavior in LazWriter.
//!
//! Usage:
//!   cargo run -p wblidar --release --example laz_point14_compression_level_benchmark -- \
//!     <input.las> <output_prefix>

use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::time::Instant;

use wblidar::io::{PointReader, PointWriter};
use wblidar::las::header::PointDataFormat;
use wblidar::las::reader::LasReader;
use wblidar::laz::{LazWriter, LazWriterConfig};
use wblidar::{PointRecord, Result};

#[derive(Clone, Copy)]
struct RunConfig {
    level: u32,
}

fn point14_effective_chunk_size(base_chunk_size: u32, compression_level: u32) -> u32 {
    let base = base_chunk_size.max(1);
    let lvl = compression_level.min(9);
    match lvl {
        0 => (base / 2).max(1),
        1 => ((base.saturating_mul(2)) / 3).max(1),
        2 => ((base.saturating_mul(3)) / 4).max(1),
        3..=6 => base,
        7 => (base.saturating_mul(5)) / 4,
        8 => (base.saturating_mul(3)) / 2,
        _ => base.saturating_mul(2),
    }
    .max(1)
}

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let input = args.next().ok_or_else(usage_error)?;
    let output_prefix = args.next().ok_or_else(usage_error)?;

    let src_size = std::fs::metadata(&input)?.len();
    let file = File::open(&input)?;
    let mut reader = LasReader::new(BufReader::new(file))?;
    let hdr = reader.header().clone();
    let crs = reader.crs().cloned();

    let mut points = Vec::<PointRecord>::new();
    let mut p = PointRecord::default();
    while reader.read_point(&mut p)? {
        points.push(p);
    }

    let point_data_format = if hdr.point_data_format.is_v14() || hdr.point_data_format.is_v15() {
        hdr.point_data_format
    } else {
        PointDataFormat::Pdrf7
    };

    let configs = [
        RunConfig { level: 0 },
        RunConfig { level: 3 },
        RunConfig { level: 6 },
        RunConfig { level: 9 },
    ];

    let csv_path = PathBuf::from(format!("{output_prefix}_point14_compression_level_report.csv"));
    let mut csv = File::create(&csv_path)?;
    writeln!(
        csv,
        "compression_level,effective_chunk_size,seconds,bytes,miB,ratio_vs_source,point_count,point_data_format"
    )?;

    println!("Input: {input}");
    println!("Source bytes: {src_size}");
    println!("Points: {}", points.len());
    println!("Output PDRF: {:?}", point_data_format);

    for cfg in configs {
        let out_path = format!("{output_prefix}_lvl{}.laz", cfg.level);
        let mut laz_cfg = LazWriterConfig::default();
        laz_cfg.compression_level = cfg.level;
        laz_cfg.chunk_size = 50_000;
        laz_cfg.las.point_data_format = point_data_format;
        laz_cfg.las.x_scale = hdr.x_scale;
        laz_cfg.las.y_scale = hdr.y_scale;
        laz_cfg.las.z_scale = hdr.z_scale;
        laz_cfg.las.x_offset = hdr.x_offset;
        laz_cfg.las.y_offset = hdr.y_offset;
        laz_cfg.las.z_offset = hdr.z_offset;
        laz_cfg.las.extra_bytes_per_point = hdr.extra_bytes_count;
        laz_cfg.las.crs = crs.clone();

        let start = Instant::now();
        {
            let out = BufWriter::new(File::create(&out_path)?);
            let mut writer = LazWriter::new(out, laz_cfg)?;
            for pt in &points {
                writer.write_point(pt)?;
            }
            writer.finish()?;
        }
        let elapsed = start.elapsed().as_secs_f64();
        let bytes = std::fs::metadata(&out_path)?.len();
        let mib = bytes as f64 / (1024.0 * 1024.0);
        let ratio = bytes as f64 / src_size as f64;
        let eff_chunk = point14_effective_chunk_size(50_000, cfg.level);

        println!(
            "  lvl {}: {:.3}s, {} bytes ({:.3} MiB), ratio {:.4}, eff_chunk {}",
            cfg.level, elapsed, bytes, mib, ratio, eff_chunk
        );

        writeln!(
            csv,
            "{},{},{:.6},{},{:.6},{:.6},{},{}",
            cfg.level,
            eff_chunk,
            elapsed,
            bytes,
            mib,
            ratio,
            points.len(),
            point_data_format as u8
        )?;
    }

    println!("Wrote report: {}", csv_path.display());
    Ok(())
}

fn usage_error() -> wblidar::Error {
    wblidar::Error::InvalidValue {
        field: "args",
        detail: "usage: laz_point14_compression_level_benchmark <input.las> <output_prefix>"
            .to_string(),
    }
}
