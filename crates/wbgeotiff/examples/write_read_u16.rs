use std::error::Error;

use wbgeotiff::{Compression, GeoTiff, GeoTiffWriter, GeoTransform, SampleFormat, WriteLayout};

fn main() -> Result<(), Box<dyn Error>> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "out_u16.tif".to_string());

    let width = 512u32;
    let height = 512u32;

    // Create a simple u16 gradient surface.
    let mut data = Vec::<u16>::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            let v = ((x + y) % 65535) as u16;
            data.push(v);
        }
    }

    GeoTiffWriter::new(width, height, 1)
        .layout(WriteLayout::Tiled {
            tile_width: 256,
            tile_height: 256,
        })
        .compression(Compression::Deflate)
        .sample_format(SampleFormat::Uint)
        .geo_transform(GeoTransform::north_up(0.0, 30.0, 0.0, -30.0))
        .epsg(4326)
        .write_u16(&output, &data)?;

    // Read back and print basic stats to verify u16 roundtrip.
    let tiff = GeoTiff::open(&output)?;
    let read_back = tiff.read_band_u16(0)?;
    let min = read_back.iter().copied().min().unwrap_or(0);
    let max = read_back.iter().copied().max().unwrap_or(0);

    println!(
        "wrote={} size={}x{} samples={} min={} max={}",
        output,
        tiff.width(),
        tiff.height(),
        read_back.len(),
        min,
        max
    );

    Ok(())
}
