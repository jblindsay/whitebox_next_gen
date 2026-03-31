//! GeoJSON read / write / parse example.
//!
//! Usage: cargo run --example geojson_io

use wbvector::feature::{FieldDef, FieldType, Layer};
use wbvector::geometry::{Coord, Geometry, GeometryType};
use wbvector::{geojson, Result};
use std::path::PathBuf;

fn data_dir() -> PathBuf {
  let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("data")
    .join("geojson_io");
  std::fs::create_dir_all(&dir).unwrap();
  dir
}

const SAMPLE_JSON: &str = r#"{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "geometry": { "type": "Point", "coordinates": [-122.4194, 37.7749] },
      "properties": { "city": "San Francisco", "pop": 873965, "founded": 1776 }
    },
    {
      "type": "Feature",
      "geometry": {
        "type": "Polygon",
        "coordinates": [[
          [-122.51, 37.71], [-122.35, 37.71],
          [-122.35, 37.83], [-122.51, 37.83],
          [-122.51, 37.71]
        ]]
      },
      "properties": { "city": "SF bbox", "pop": null, "founded": null }
    }
  ]
}"#;

fn main() -> Result<()> {
    // ── Parse from string ─────────────────────────────────────────────────────
    let layer = geojson::parse_str(SAMPLE_JSON)?;
    println!("Parsed {} features", layer.len());
    println!("Schema:");
    for fd in layer.schema.fields() {
        println!("  {} : {}", fd.name, fd.field_type);
    }
    println!();

    for feat in layer.iter() {
        let city = feat.get(&layer.schema, "city")?;
        let pop  = feat.get(&layer.schema, "pop")?;
        print!("  {} — pop={}", city, pop);
        if let Some(g) = &feat.geometry {
            match g {
                Geometry::Point(c) => print!("  (Point {:.4},{:.4})", c.x, c.y),
                Geometry::Polygon { exterior, .. } => print!("  (Polygon {} verts)", exterior.len()),
                _ => {}
            }
        }
        println!();
    }

    // ── Write to file and read back ───────────────────────────────────────────
    let dir  = data_dir();
    let path = dir.join("sf.geojson");

    geojson::write(&layer, &path)?;
    println!("\nWrote {}", path.display());

    let loaded = geojson::read(&path)?;
    println!("Read back {} features", loaded.len());

    // ── Build and serialise a MultiPolygon layer ──────────────────────────────
    let mut ml = Layer::new("countries").with_geom_type(GeometryType::MultiPolygon).with_epsg(4326);
    ml.add_field(FieldDef::new("iso2", FieldType::Text));
    ml.add_feature(
        Some(Geometry::multi_polygon(vec![
            (vec![Coord::xy(0.,0.), Coord::xy(1.,0.), Coord::xy(0.5,1.)], vec![]),
            (vec![Coord::xy(2.,0.), Coord::xy(3.,0.), Coord::xy(2.5,1.)], vec![]),
        ])),
        &[("iso2", "XX".into())],
    )?;

    let json_str = geojson::to_string(&ml);
    println!("\nMultiPolygon GeoJSON (first 120 chars):");
    println!("  {}…", &json_str[..120.min(json_str.len())]);

    // ── GeometryCollection roundtrip ──────────────────────────────────────────
    let gc_json = r#"{"type":"GeometryCollection","geometries":[
        {"type":"Point","coordinates":[1,2]},
        {"type":"LineString","coordinates":[[0,0],[1,1],[2,0]]}
    ]}"#;
    let gc_layer = geojson::parse_str(gc_json)?;
    assert!(matches!(gc_layer[0].geometry, Some(Geometry::GeometryCollection(_))));
    println!("\nGeometryCollection parsed OK ✓");

    println!("\nGeoJSON example OK ✓");
    Ok(())
}
