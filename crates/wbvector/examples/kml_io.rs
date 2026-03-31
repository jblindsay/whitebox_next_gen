//! KML read/write example.
//!
//! Usage: cargo run --example kml_io

use wbvector::feature::{FieldDef, FieldType, Layer};
use wbvector::geometry::{Coord, Geometry, GeometryType};
use wbvector::{kml, Result};
use std::path::PathBuf;

fn data_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("kml_io");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn main() -> Result<()> {
    let mut layer = Layer::new("trails")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);

    layer.add_field(FieldDef::new("name", FieldType::Text));
    layer.add_field(FieldDef::new("difficulty", FieldType::Text));

    layer.add_feature(
        Some(Geometry::line_string(vec![
            Coord::xy(-79.40, 43.70),
            Coord::xy(-79.39, 43.71),
            Coord::xy(-79.38, 43.72),
        ])),
        &[("name", "Ridge Trail".into()), ("difficulty", "Moderate".into())],
    )?;

    let dir = data_dir();
    let path = dir.join("trails.kml");
    kml::write(&layer, &path)?;
    let back = kml::read(&path)?;

    println!("Wrote {} feature(s) to {}", layer.len(), path.display());
    println!("Read back {} feature(s), {} field(s)", back.len(), back.schema.len());

    Ok(())
}
