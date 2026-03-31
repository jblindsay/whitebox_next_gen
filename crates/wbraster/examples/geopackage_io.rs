//! GeoPackage raster read/write example.
//!
//! Usage: cargo run --example geopackage_io

use std::path::PathBuf;

use wbraster::{DataType, Raster, RasterConfig, RasterFormat, Result};

fn data_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("examples_geopackage");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn main() -> Result<()> {
    let mut raster = Raster::new(RasterConfig {
        cols: 32,
        rows: 24,
        bands: 2,
        x_min: 200_000.0,
        y_min: 4_900_000.0,
        cell_size: 20.0,
        nodata: -32768.0,
        data_type: DataType::I16,
        ..Default::default()
    });

    for row in 0..raster.rows {
        for col in 0..raster.cols {
            let b0 = (row as i32 - col as i32) as f64;
            let b1 = (row as i32 * 2 + col as i32) as f64;
            raster.set(0, row as isize, col as isize, b0)?;
            raster.set(1, row as isize, col as isize, b1)?;
        }
    }

    raster.metadata.push(("gpkg_tile_encoding".into(), "raw".into()));
    raster.metadata.push(("gpkg_raw_compression".into(), "deflate".into()));

    let path = data_dir().join("multiband.gpkg");
    raster.write(path.to_str().unwrap(), RasterFormat::GeoPackage)?;

    let loaded = Raster::read(path.to_str().unwrap())?;
    println!("Loaded GeoPackage bands: {}", loaded.bands);
    println!("Band 0 at (row=5,col=9): {}", loaded.get(0, 5, 9));
    println!("Band 1 at (row=5,col=9): {}", loaded.get(1, 5, 9));
    println!("geopackage_io example OK");

    Ok(())
}
