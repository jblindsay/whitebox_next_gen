//! GeoTIFF + COG write/read example.
//!
//! Usage: cargo run --example geotiff_cog_io

use std::path::PathBuf;

use wbraster::{
    CogWriteOptions, CrsInfo, DataType, GeoTiffCompression, Raster, RasterConfig, RasterFormat,
    Result,
};

fn data_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("examples_geotiff");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn main() -> Result<()> {
    let mut raster = Raster::new(RasterConfig {
        cols: 64,
        rows: 48,
        bands: 1,
        x_min: -180.0,
        y_min: -90.0,
        cell_size: 0.01,
        nodata: -9999.0,
        data_type: DataType::F32,
        crs: CrsInfo::from_epsg(4326),
        ..Default::default()
    });

    for row in 0..raster.rows {
        for col in 0..raster.cols {
            let v = (row as f64).sin() + (col as f64).cos();
            raster.set(0, row as isize, col as isize, v)?;
        }
    }

    let dir = data_dir();
    let tif = dir.join("surface.tif");
    let cog = dir.join("surface_cog.tif");

    raster.write(tif.to_str().unwrap(), RasterFormat::GeoTiff)?;

    let cog_opts = CogWriteOptions {
        compression: Some(GeoTiffCompression::Deflate),
        bigtiff: Some(false),
        tile_size: Some(256),
    };
    raster.write_cog_with_options(cog.to_str().unwrap(), &cog_opts)?;

    let loaded = Raster::read(cog.to_str().unwrap())?;
    println!("Loaded COG dims: {} x {}", loaded.cols, loaded.rows);
    println!("Loaded EPSG: {:?}", loaded.crs.epsg);
    println!("geotiff_cog_io example OK");

    Ok(())
}
