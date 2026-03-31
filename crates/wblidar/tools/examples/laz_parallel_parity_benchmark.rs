//! Benchmark LAZ parallel vs serial decompression with parity validation.
//!
//! Usage:
//!   cargo run -p wblidar --example laz_parallel_parity_benchmark --features parallel -- \
//!     <input.laz> <output_dir>

use std::env;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::time::Instant;

use wblidar::io::PointReader;
use wblidar::laz::reader::LazReader;
use wblidar::{Error, PointRecord, Result};

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let input_path = args.next().ok_or_else(|| {
        eprintln!("Usage: cargo run --example laz_parallel_parity_benchmark -- <input.laz> [output_dir]");
        Error::Unimplemented("Missing arguments".into())
    })?;
    let output_dir = args.next().unwrap_or_else(|| ".".to_string());

    println!("Input: {}", input_path);

    #[cfg(feature = "laz-parallel")]
    {
        // Test parallel decompression if available.
        let input_file = File::open(&input_path).map_err(Error::Io)?;
        let mut reader = LazReader::new(BufReader::new(input_file))?;

        // Warm up
        let _ = reader.read_all_points_parallel()?;

        // Serial baseline.
        let input_file = File::open(&input_path).map_err(Error::Io)?;
        let mut reader = LazReader::new(BufReader::new(input_file))?;
        let start = Instant::now();
        let serial_points = decode_serial(&mut reader)?;
        let serial_time = start.elapsed();

        println!("  Serial: {:.3}s ({} points)", serial_time.as_secs_f64(), serial_points.len());

        // Parallel run.
        let input_file = File::open(&input_path).map_err(Error::Io)?;
        let mut reader = LazReader::new(BufReader::new(input_file))?;
        let start = Instant::now();
        let parallel_points = reader.read_all_points_parallel()?;
        let parallel_time = start.elapsed();

        let speedup = serial_time.as_secs_f64() / parallel_time.as_secs_f64();
        println!("  Parallel: {:.3}s ({} points) – {:.3}x", 
                 parallel_time.as_secs_f64(), parallel_points.len(), speedup);

        println!("Input points: {}", serial_points.len());

        // Validate parity.
        if serial_points.len() != parallel_points.len() {
            eprintln!("PARITY MISMATCH: point count serial={} vs parallel={}", 
                     serial_points.len(), parallel_points.len());
            return Err(Error::Unimplemented("Parity check failed".into()));
        }

        for (i, (s, p)) in serial_points.iter().zip(parallel_points.iter()).enumerate() {
            if (s.x - p.x).abs() > 1e-10 || (s.y - p.y).abs() > 1e-10 || (s.z - p.z).abs() > 1e-10 {
                eprintln!("PARITY MISMATCH at point {}: serial={:?} vs parallel={:?}", i, s, p);
                return Err(Error::Unimplemented("Parity check failed".into()));
            }
        }

        println!("Parity: ✓ PASS – all points match");

        // Write CSV report.
        let csv_path = PathBuf::from(&output_dir).join("laz_parallel_benchmark.csv");
        let mut csv_file = std::fs::File::create(&csv_path).map_err(Error::Io)?;
        writeln!(csv_file, "mode,seconds,points,speedup")?;
        writeln!(csv_file, "serial,{:.6},{},1.0", serial_time.as_secs_f64(), serial_points.len())?;
        writeln!(csv_file, "parallel,{:.6},{},{:.6}", parallel_time.as_secs_f64(), parallel_points.len(), speedup)?;
        println!("Report: {}", csv_path.display());
    }

    #[cfg(not(feature = "laz-parallel"))]
    {
        println!("parallel decode not enabled. Run with: --features parallel or --features laz-parallel");
    }

    Ok(())
}

/// Decode all points serially using the streaming interface.
fn decode_serial(reader: &mut LazReader<BufReader<File>>) -> Result<Vec<PointRecord>> {
    let mut points = Vec::new();
    let mut p = PointRecord::default();
    while reader.read_point(&mut p)? {
        points.push(p);
    }
    Ok(points)
}
