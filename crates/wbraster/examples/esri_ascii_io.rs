//! Esri ASCII read/write roundtrip example.
//!
//! Usage: cargo run --example esri_ascii_io

use std::path::PathBuf;

use wbraster::{DataType, Raster, RasterConfig, RasterFormat, Result};

fn data_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("examples_esri_ascii");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn main() -> Result<()> {
    let cfg = RasterConfig {
        cols: 5,
        rows: 4,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 30.0,
        nodata: -9999.0,
        data_type: DataType::F32,
        ..Default::default()
    };

    let data: Vec<f64> = (0..(cfg.cols * cfg.rows))
        .map(|i| if i == 7 { -9999.0 } else { i as f64 * 0.5 })
        .collect();

    let raster = Raster::from_data(cfg, data)?;

    let path = data_dir().join("terrain.asc");
    raster.write(path.to_str().unwrap(), RasterFormat::EsriAscii)?;

    let loaded = Raster::read(path.to_str().unwrap())?;
    println!("Format detected as: {}", wbraster::RasterFormat::detect(path.to_str().unwrap())?.name());
    println!("Loaded dims: {} x {}", loaded.cols, loaded.rows);
    println!("Value (row=1,col=1): {}", loaded.get(0, 1, 1));
    println!("Nodata at (row=1,col=2): {}", loaded.is_nodata(loaded.get(0, 1, 2)));
    println!("esri_ascii_io example OK");

    Ok(())
}
