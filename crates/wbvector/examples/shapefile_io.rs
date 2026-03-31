//! Shapefile read / write example.
//!
//! Usage: cargo run --example shapefile_io

use wbvector::feature::{FieldDef, FieldType, Layer};
use wbvector::geometry::{Coord, Geometry, GeometryType};
use wbvector::{shapefile, Result};
use std::path::PathBuf;

fn data_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("shapefile_io");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn main() -> Result<()> {
    // ── Build a layer in memory ───────────────────────────────────────────────
    let mut layer = Layer::new("airports")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);

    layer.add_field(FieldDef::new("code",     FieldType::Text).width(4));
    layer.add_field(FieldDef::new("name",     FieldType::Text).width(80));
    layer.add_field(FieldDef::new("elev_m",   FieldType::Integer));
    layer.add_field(FieldDef::new("intl",     FieldType::Boolean));

    layer.add_feature(
        Some(Geometry::point(-0.4543, 51.4775)),
        &[
            ("code",   "LHR".into()),
            ("name",   "Heathrow".into()),
            ("elev_m", 25i64.into()),
            ("intl",   true.into()),
        ],
    )?;
    layer.add_feature(
        Some(Geometry::point(2.5479, 49.0097)),
        &[
            ("code",   "CDG".into()),
            ("name",   "Charles de Gaulle".into()),
            ("elev_m", 119i64.into()),
            ("intl",   true.into()),
        ],
    )?;
    layer.add_feature(
        Some(Geometry::point(13.4050, 52.3667)),
        &[
            ("code",   "BER".into()),
            ("name",   "Berlin Brandenburg".into()),
            ("elev_m", 48i64.into()),
            ("intl",   true.into()),
        ],
    )?;

    // ── Write ─────────────────────────────────────────────────────────────────
    let dir   = data_dir();
    let base  = dir.join("airports");
    println!("Writing Shapefile to {}", base.display());
    shapefile::write(&layer, &base)?;

    // ── Read back ─────────────────────────────────────────────────────────────
    let loaded = shapefile::read(&base)?;
    println!("\nRead {} features", loaded.len());
    println!("Schema: {:?}", loaded.schema.fields().iter().map(|f| &f.name).collect::<Vec<_>>());

    for feat in loaded.iter() {
        if let Some(Geometry::Point(c)) = &feat.geometry {
            let code = feat.get(&loaded.schema, "code").unwrap();
            let name = feat.get(&loaded.schema, "name").unwrap();
            let elev = feat.get(&loaded.schema, "elev_m").unwrap();
            println!("  {} — {} @ ({:.4}, {:.4}), elev {} m", code, name, c.x, c.y, elev);
        }
    }

    // ── Polygon layer ─────────────────────────────────────────────────────────
    let mut polylayer = Layer::new("blocks")
        .with_geom_type(GeometryType::Polygon)
        .with_epsg(4326);
    polylayer.add_field(FieldDef::new("block_id", FieldType::Integer));

    polylayer.add_feature(
        Some(Geometry::polygon(
            vec![
                Coord::xy(0.0, 0.0),
                Coord::xy(1.0, 0.0),
                Coord::xy(1.0, 1.0),
                Coord::xy(0.0, 1.0),
            ],
            vec![],
        )),
        &[("block_id", 1i64.into())],
    )?;

    let poly_base = dir.join("blocks");
    shapefile::write(&polylayer, &poly_base)?;
    let loaded_poly = shapefile::read(&poly_base)?;
    println!("\nPolygon layer: {} features", loaded_poly.len());
    if let Some(Geometry::Polygon { exterior, interiors }) = &loaded_poly[0].geometry {
        println!("  exterior ring has {} vertices, {} holes", exterior.len(), interiors.len());
    }

    println!("\nShapefile example OK ✓");
    Ok(())
}
