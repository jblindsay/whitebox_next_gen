use std::error::Error;

use wbgeotiff::GeoTiff;

fn main() -> Result<(), Box<dyn Error>> {
    let input = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "dem.tif".to_string());

    let tiff = GeoTiff::open(&input)?;
    println!(
        "file={} size={}x{} bands={} bigtiff={}",
        input,
        tiff.width(),
        tiff.height(),
        tiff.band_count(),
        tiff.is_bigtiff
    );

    let band0: Vec<f32> = tiff.read_band_f32(0)?;
    println!("band0_samples={} first_sample={}", band0.len(), band0[0]);

    Ok(())
}
