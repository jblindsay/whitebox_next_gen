//! Basic raster creation, write, read, and sampling example.
//!
//! Usage: cargo run --example raster_basics

use std::path::PathBuf;

use wbraster::{DataType, Raster, RasterConfig, RasterFormat, Result};

fn data_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("examples_raster_basics");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn main() -> Result<()> {
    let mut raster = Raster::new(RasterConfig {
        cols: 8,
        rows: 6,
        bands: 1,
        x_min: 500_000.0,
        y_min: 4_500_000.0,
        cell_size: 10.0,
        nodata: -9999.0,
        data_type: DataType::F32,
        ..Default::default()
    });

    for row in 0..raster.rows {
        for col in 0..raster.cols {
            let value = (row * raster.cols + col) as f64;
            raster.set(0, row as isize, col as isize, value)?;
        }
    }

    raster.set(0, 2, 3, raster.nodata)?;

    let out = data_dir().join("basic.asc");
    raster.write(out.to_str().unwrap(), RasterFormat::EsriAscii)?;

    let loaded = Raster::read(out.to_str().unwrap())?;

    println!("Loaded raster: {} cols x {} rows", loaded.cols, loaded.rows);
    println!("Cell (row=1, col=2) = {}", loaded.get(0, 1, 2));
    println!("Cell (row=2, col=3) nodata? {}", loaded.is_nodata(loaded.get(0, 2, 3)));

    let x = loaded.col_center_x(4);
    let y = loaded.row_center_y(3);
    let (col, row) = loaded.world_to_pixel(x, y).unwrap();
    let sampled = loaded.get_opt(0, row, col);
    println!("Sample at center of (row=3,col=4): {:?}", sampled);

    println!("raster_basics example OK");
    Ok(())
}
