//! GeoParquet read/write example.
//!
//! Requires feature: `geoparquet`
//! Usage:
//!   cargo run --features geoparquet --example geoparquet_io -- path/to/input.parquet
//!   cargo run --features geoparquet --example geoparquet_io -- path/to/input.parquet path/to/output.parquet

#[cfg(feature = "geoparquet")]
fn main() -> wbvector::Result<()> {
    use std::path::PathBuf;
    use wbvector::geoparquet;

    let mut args = std::env::args().skip(1);
    let input = args.next().ok_or_else(|| {
        wbvector::GeoError::NotImplemented(
            "Usage: cargo run --features geoparquet --example geoparquet_io -- path/to/input.parquet [path/to/output.parquet]".into(),
        )
    })?;
    let output = args.next();

    let in_path = PathBuf::from(input);
    let layer = geoparquet::read(&in_path)?;

    println!("Read {} features from {}", layer.len(), in_path.display());
    println!("Geometry type: {:?}", layer.geom_type);
    println!("Fields: {}", layer.schema.len());

    if let Some(out) = output {
        let out_path = PathBuf::from(out);
        geoparquet::write(&layer, &out_path)?;
        println!("Wrote {} features to {}", layer.len(), out_path.display());
    }

    Ok(())
}

#[cfg(not(feature = "geoparquet"))]
fn main() {
    eprintln!("Enable feature 'geoparquet': cargo run --features geoparquet --example geoparquet_io -- path/to/input.parquet [path/to/output.parquet]");
}
