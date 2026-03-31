//! GeoPackage read / write example.
//!
//! Usage: cargo run --example geopackage_io

use wbvector::feature::{FieldDef, FieldType, Layer};
use wbvector::geometry::{Coord, Geometry, GeometryType};
use wbvector::{geopackage, Result};
use std::path::PathBuf;

fn data_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("geopackage_io");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn main() -> Result<()> {
    let dir = data_dir();

    // ── Single layer ──────────────────────────────────────────────────────────
    let mut cities = Layer::new("cities")
        .with_geom_type(GeometryType::Point)
        .with_epsg(4326);

    cities.add_field(FieldDef::new("name",    FieldType::Text));
    cities.add_field(FieldDef::new("country", FieldType::Text));
    cities.add_field(FieldDef::new("pop_m",   FieldType::Float));

    cities.add_feature(
        Some(Geometry::point(-0.1278, 51.5074)),
        &[("name", "London".into()), ("country", "GB".into()), ("pop_m", 9.0f64.into())],
    )?;
    cities.add_feature(
        Some(Geometry::point(2.3522, 48.8566)),
        &[("name", "Paris".into()), ("country", "FR".into()), ("pop_m", 2.1f64.into())],
    )?;
    cities.add_feature(
        Some(Geometry::point(12.4964, 41.9028)),
        &[("name", "Rome".into()), ("country", "IT".into()), ("pop_m", 2.8f64.into())],
    )?;

    let path = dir.join("cities.gpkg");
    geopackage::write(&cities, &path)?;
    println!("Wrote {} to {}", cities.name, path.display());

    let loaded = geopackage::read(&path)?;
    println!("Read {} features", loaded.len());
    for f in loaded.iter() {
        let name    = f.get(&loaded.schema, "name")?;
        let pop     = f.get(&loaded.schema, "pop_m")?;
        let country = f.get(&loaded.schema, "country")?;
        if let Some(Geometry::Point(c)) = &f.geometry {
            println!("  {name} ({country}): pop={pop}M at ({:.4}, {:.4})", c.x, c.y);
        }
    }

    // ── Multiple layers in one file ───────────────────────────────────────────
    let mut regions = Layer::new("regions")
        .with_geom_type(GeometryType::Polygon)
        .with_epsg(4326);
    regions.add_field(FieldDef::new("name", FieldType::Text));
    regions.add_feature(
        Some(Geometry::polygon(
            vec![
                Coord::xy(-10., 35.), Coord::xy(40., 35.),
                Coord::xy(40.,  72.), Coord::xy(-10., 72.),
            ],
            vec![],
        )),
        &[("name", "Europe".into())],
    )?;

    let multi_path = dir.join("europe.gpkg");
    geopackage::write_layers(&[&cities, &regions], &multi_path)?;

    let names = geopackage::list_layers(&multi_path)?;
    println!("\nLayers in europe.gpkg: {:?}", names);

    let r = geopackage::read_layer(&multi_path, "regions")?;
    println!("'regions' layer: {} feature(s)", r.len());
    if let Some(Geometry::Polygon { exterior, interiors }) = &r[0].geometry {
        println!("  exterior ring: {} vertices, {} holes", exterior.len(), interiors.len());
    }

    // ── Verify bbox ───────────────────────────────────────────────────────────
    let mut l2 = loaded;
    if let Some(bb) = l2.bbox() {
        println!("\nBounding box of cities: [{:.4},{:.4} → {:.4},{:.4}]",
            bb.min_x, bb.min_y, bb.max_x, bb.max_y);
    }

    println!("\nGeoPackage example OK ✓");
    Ok(())
}
