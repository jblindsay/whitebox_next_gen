//! Raster reprojection example.
//!
//! Usage: cargo run --example reproject_io

use std::path::PathBuf;

use wbraster::{
    CrsInfo, DataType, DestinationFootprint, GridSizePolicy, NodataPolicy, Raster, RasterConfig,
    ReprojectOptions, ResampleMethod, Result,
};

fn data_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("examples_reproject");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn main() -> Result<()> {
    let mut src = Raster::new(RasterConfig {
        cols: 120,
        rows: 80,
        bands: 1,
        x_min: -80.0,
        y_min: 35.0,
        cell_size: 0.01,
        nodata: -9999.0,
        data_type: DataType::F32,
        crs: CrsInfo::from_epsg(4326),
        ..Default::default()
    });

    for row in 0..src.rows {
        for col in 0..src.cols {
            let lon = src.col_center_x(col as isize);
            let lat = src.row_center_y(row as isize);
            let v = (lat.to_radians().sin() * lon.to_radians().cos()) * 1000.0;
            src.set(0, row as isize, col as isize, v)?;
        }
    }

    let opts = ReprojectOptions::new(3857, ResampleMethod::Bilinear)
        .with_grid_size_policy(GridSizePolicy::Expand)
        .with_destination_footprint(DestinationFootprint::SourceBoundary)
        .with_nodata_policy(NodataPolicy::PartialKernel)
        .with_resolution(1000.0, 1000.0);

    let projected = src.reproject_with_options(&opts)?;

    let out = data_dir().join("projected_3857.tif");
    projected.write(out.to_str().unwrap(), wbraster::RasterFormat::GeoTiff)?;

    println!("Source: {} x {}, EPSG {:?}", src.cols, src.rows, src.crs.epsg);
    println!(
        "Projected: {} x {}, EPSG {:?}",
        projected.cols, projected.rows, projected.crs.epsg
    );
    println!("Wrote {}", out.display());
    println!("reproject_io example OK");

    Ok(())
}
