use std::error::Error;

use wbgeotiff::{Compression, GeoTiffWriter, GeoTransform, SampleFormat, WriteLayout};

fn main() -> Result<(), Box<dyn Error>> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "out.tif".to_string());

    let width = 1024u32;
    let height = 1024u32;
    let data = vec![0.0f32; (width * height) as usize];

    GeoTiffWriter::new(width, height, 1)
        .layout(WriteLayout::Tiled {
            tile_width: 256,
            tile_height: 256,
        })
        .compression(Compression::Deflate)
        .sample_format(SampleFormat::IeeeFloat)
        .geo_transform(GeoTransform::north_up(500_000.0, 10.0, 4_500_000.0, -10.0))
        .epsg(32632)
        .write_f32(&output, &data)?;

    println!("wrote {}", output);
    Ok(())
}
