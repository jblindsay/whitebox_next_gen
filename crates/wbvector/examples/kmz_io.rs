//! KMZ read/write example.
//!
//! Requires feature: `kmz`
//! Usage: cargo run --features kmz --example kmz_io

#[cfg(feature = "kmz")]
fn main() -> wbvector::Result<()> {
    use std::path::PathBuf;
    use wbvector::feature::{FieldDef, FieldType, Layer};
    use wbvector::geometry::{Coord, Geometry, GeometryType};
    use wbvector::kmz;

    fn data_dir() -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("kmz_io");
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    let mut layer = Layer::new("trails")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);

    layer.add_field(FieldDef::new("name", FieldType::Text));
    layer.add_feature(
        Some(Geometry::line_string(vec![
            Coord::xy(-79.40, 43.70),
            Coord::xy(-79.39, 43.71),
            Coord::xy(-79.38, 43.72),
        ])),
        &[("name", "Ridge Trail".into())],
    )?;

    let path = data_dir().join("trails.kmz");
    kmz::write(&layer, &path)?;
    let back = kmz::read(&path)?;

    println!("Wrote {} feature(s) to {}", layer.len(), path.display());
    println!("Read back {} feature(s), {} field(s)", back.len(), back.schema.len());

    Ok(())
}

#[cfg(not(feature = "kmz"))]
fn main() {
    eprintln!("Enable feature 'kmz': cargo run --features kmz --example kmz_io");
}
