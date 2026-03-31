use std::error::Error;

use wbgeotiff::{CogWriter, Compression, GeoTransform, Resampling};

fn main() -> Result<(), Box<dyn Error>> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "out.cog.tif".to_string());

    let width = 4096u32;
    let height = 4096u32;
    let data = vec![1.0f32; (width * height) as usize];

    CogWriter::new(width, height, 1)
        .compression(Compression::Deflate)
        .tile_size(512)
        .resampling(Resampling::Average)
        .geo_transform(GeoTransform::north_up(-180.0, 0.087890625, 90.0, -0.087890625))
        .epsg(4326)
        .write_f32(&output, &data)?;

    println!("wrote {}", output);
    Ok(())
}
