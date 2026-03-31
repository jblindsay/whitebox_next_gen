//! GML read/write example.
//!
//! Usage: cargo run --example gml_io

use wbvector::feature::{FieldDef, FieldType, Layer};
use wbvector::geometry::{Coord, Geometry, GeometryType};
use wbvector::{gml, Result};
use std::path::PathBuf;

fn data_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("gml_io");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn main() -> Result<()> {
    let mut layer = Layer::new("roads")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);

    layer.add_field(FieldDef::new("name", FieldType::Text));
    layer.add_field(FieldDef::new("speed", FieldType::Integer));

    layer.add_feature(
        Some(Geometry::line_string(vec![
            Coord::xy(-0.10, 51.50),
            Coord::xy(-0.11, 51.52),
            Coord::xy(-0.12, 51.53),
        ])),
        &[("name", "A-Road".into()), ("speed", 50i64.into())],
    )?;

    let dir = data_dir();
    let path = dir.join("roads.gml");
    gml::write(&layer, &path)?;
    let back = gml::read(&path)?;

    println!("Wrote {} feature(s) to {}", layer.len(), path.display());
    println!("Read back {} feature(s), {} field(s)", back.len(), back.schema.len());

    Ok(())
}
