//! Zarr v2 and v3 write/read example.
//!
//! Usage: cargo run --example zarr_io

use std::path::PathBuf;

use wbraster::{CrsInfo, DataType, Raster, RasterConfig, RasterFormat, Result};

fn data_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("examples_zarr");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn make_raster() -> Raster {
    let mut r = Raster::new(RasterConfig {
        cols: 20,
        rows: 12,
        bands: 1,
        x_min: 10.0,
        y_min: 20.0,
        cell_size: 2.0,
        nodata: -9999.0,
        data_type: DataType::F32,
        crs: CrsInfo::from_epsg(4326),
        ..Default::default()
    });
    for row in 0..r.rows {
        for col in 0..r.cols {
            let v = (row * r.cols + col) as f64 * 0.25;
            r.set(0, row as isize, col as isize, v).unwrap();
        }
    }
    r
}

fn main() -> Result<()> {
    let dir = data_dir();

    let mut z2 = make_raster();
    z2.metadata.push(("zarr_version".into(), "2".into()));
    z2.metadata.push(("zarr_dimension_separator".into(), "/".into()));
    let z2_path = dir.join("surface_v2.zarr");
    z2.write(z2_path.to_str().unwrap(), RasterFormat::Zarr)?;
    let z2_loaded = Raster::read(z2_path.to_str().unwrap())?;

    let mut z3 = make_raster();
    z3.metadata.push(("zarr_version".into(), "3".into()));
    z3.metadata.push(("zarr_chunk_key_encoding".into(), "default".into()));
    z3.metadata.push(("zarr_dimension_separator".into(), "/".into()));
    let z3_path = dir.join("surface_v3.zarr");
    z3.write(z3_path.to_str().unwrap(), RasterFormat::Zarr)?;
    let z3_loaded = Raster::read(z3_path.to_str().unwrap())?;

    println!("Zarr v2 sample (row=3,col=4): {}", z2_loaded.get(0, 3, 4));
    println!("Zarr v3 sample (row=3,col=4): {}", z3_loaded.get(0, 3, 4));
    println!("zarr_io example OK");

    Ok(())
}
