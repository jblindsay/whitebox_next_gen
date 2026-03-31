//! FlatGeobuf read / write example.
//!
//! Usage: cargo run --example flatgeobuf_io

use wbvector::feature::{FieldDef, FieldType, Layer};
use wbvector::geometry::{Coord, Geometry, GeometryType};
use wbvector::{flatgeobuf, geojson, Result};
use std::path::PathBuf;

fn data_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("flatgeobuf_io");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn main() -> Result<()> {
    // ── Build a polygon layer ─────────────────────────────────────────────────
    let mut layer = Layer::new("countries")
        .with_geom_type(GeometryType::Polygon)
        .with_epsg(4326);

    layer.add_field(FieldDef::new("name",     FieldType::Text));
    layer.add_field(FieldDef::new("iso3",     FieldType::Text));
    layer.add_field(FieldDef::new("area_km2", FieldType::Float));
    layer.add_field(FieldDef::new("pop",      FieldType::Integer));

    layer.add_feature(
        Some(Geometry::polygon(
            vec![
                Coord::xy(-5.0,  49.0), Coord::xy(2.0,  49.0),
                Coord::xy(2.0,   60.0), Coord::xy(-5.0, 60.0),
            ],
            vec![],
        )),
        &[
            ("name",     "United Kingdom".into()),
            ("iso3",     "GBR".into()),
            ("area_km2", 242495.0f64.into()),
            ("pop",      67_000_000i64.into()),
        ],
    )?;
    layer.add_feature(
        Some(Geometry::polygon(
            vec![
                Coord::xy(-5.0, 35.0), Coord::xy(10.0, 35.0),
                Coord::xy(10.0, 51.0), Coord::xy(-5.0, 51.0),
            ],
            vec![],
        )),
        &[
            ("name",     "France".into()),
            ("iso3",     "FRA".into()),
            ("area_km2", 643801.0f64.into()),
            ("pop",      68_000_000i64.into()),
        ],
    )?;

    // ── In-memory roundtrip ───────────────────────────────────────────────────
    let bytes  = flatgeobuf::to_bytes(&layer);
    println!("Serialised {} bytes", bytes.len());
    assert!(bytes.starts_with(&flatgeobuf::MAGIC));

    let loaded = flatgeobuf::from_bytes(&bytes)?;
    println!("Loaded {} features", loaded.len());

    for feat in loaded.iter() {
        let name = feat.get(&loaded.schema, "name")?;
        let pop  = feat.get(&loaded.schema, "pop")?;
        if let Some(g) = &feat.geometry {
            let bb = g.bbox().unwrap();
            println!("  {} — pop={}, bbox=[{:.1},{:.1},{:.1},{:.1}]",
                name, pop, bb.min_x, bb.min_y, bb.max_x, bb.max_y);
        }
    }

    // ── File roundtrip ────────────────────────────────────────────────────────
    let dir  = data_dir();
    let path = dir.join("countries.fgb");
    flatgeobuf::write(&layer, &path)?;
    println!("\nWrote {}", path.display());

    let from_file = flatgeobuf::read(&path)?;
    println!("Read {} features from file", from_file.len());

    let name0 = from_file[0].get(&from_file.schema, "name")?;
    println!("First feature name: {}", name0);

    let qgis_path = dir.join("countries.geojson");
    geojson::write(&layer, &qgis_path)?;
    println!("Wrote QGIS-ready file {}", qgis_path.display());

    // ── Z-coordinate support ──────────────────────────────────────────────────
    let mut z_layer = Layer::new("peaks").with_geom_type(GeometryType::Point);
    z_layer.add_field(FieldDef::new("peak", FieldType::Text));
    z_layer.add_feature(
        Some(Geometry::point_z(86.9250, 27.9881, 8848.86)),
        &[("peak", "Everest".into())],
    )?;
    let z_bytes = flatgeobuf::to_bytes(&z_layer);
    let z_loaded = flatgeobuf::from_bytes(&z_bytes)?;
    if let Some(Geometry::Point(c)) = &z_loaded[0].geometry {
        println!("\nEverest: ({:.4}, {:.4}, {:.2}m)",
            c.x, c.y, c.z.unwrap_or(0.0));
    }

    println!("\nFlatGeobuf example OK ✓");
    Ok(())
}
