//! Cross-format conversion example.
//!
//! Demonstrates that all four drivers share the same `Layer` type, so
//! converting between any pair of formats is always two lines.
//!
//! Usage: cargo run --example convert

use wbvector::feature::{FieldDef, FieldType, Layer};
use wbvector::geometry::{Coord, Geometry, GeometryType};
use wbvector::{shapefile, geojson, flatgeobuf, geopackage, gml, gpx, kml, mapinfo, Result};
#[cfg(feature = "geoparquet")]
use wbvector::geoparquet;
use std::path::PathBuf;

fn data_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("convert");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sample_layer() -> Layer {
    let mut l = Layer::new("rivers")
        .with_geom_type(GeometryType::LineString)
        .with_epsg(4326);
    l.add_field(FieldDef::new("name",      FieldType::Text));
    l.add_field(FieldDef::new("length_km", FieldType::Float));
    l.add_field(FieldDef::new("country",   FieldType::Text));

    l.add_feature(
        Some(Geometry::line_string(vec![
            Coord::xy(30.0, 31.0), Coord::xy(31.5, 30.0),
            Coord::xy(32.5, 28.0), Coord::xy(33.0, 25.0),
        ])),
        &[
            ("name",      "Nile".into()),
            ("length_km", 6650.0f64.into()),
            ("country",   "EG".into()),
        ],
    ).unwrap();
    l.add_feature(
        Some(Geometry::line_string(vec![
            Coord::xy(-73.4, -3.5), Coord::xy(-56.0, -2.5),
            Coord::xy(-50.0, -1.0), Coord::xy(-49.5,  0.5),
        ])),
        &[
            ("name",      "Amazon".into()),
            ("length_km", 6400.0f64.into()),
            ("country",   "BR".into()),
        ],
    ).unwrap();
    l.add_feature(
        Some(Geometry::line_string(vec![
            Coord::xy(121.5, 31.2), Coord::xy(116.4, 39.9),
        ])),
        &[
            ("name",      "Yangtze".into()),
            ("length_km", 6300.0f64.into()),
            ("country",   "CN".into()),
        ],
    ).unwrap();
    l
}

fn verify_layer(l: &Layer, label: &str) {
    println!("  [{label}] {} features, {} fields", l.len(), l.schema.len());
    for f in l.iter() {
        let name = f.get(&l.schema, "name").unwrap();
        let len  = f.get(&l.schema, "length_km").unwrap();
        if let Some(Geometry::LineString(cs)) = &f.geometry {
            println!("    {} ({} km, {} vertices)", name, len, cs.len());
        }
    }
}

fn main() -> Result<()> {
    let dir = data_dir();
    let source = sample_layer();
    println!("Source layer: {} features\n", source.len());

    // ── Write to every format ─────────────────────────────────────────────────
    let shp_path  = dir.join("rivers");
    let json_path = dir.join("rivers.geojson");
    let fgb_path  = dir.join("rivers.fgb");
    let gpkg_path = dir.join("rivers.gpkg");
    #[cfg(feature = "geoparquet")]
    let parquet_path = dir.join("rivers.parquet");
    let gml_path  = dir.join("rivers.gml");
    let gpx_path  = dir.join("rivers.gpx");
    let kml_path  = dir.join("rivers.kml");
    let mif_path  = dir.join("rivers.mif");

    shapefile::write(&source,  &shp_path)?;
    geojson::write(&source,    &json_path)?;
    flatgeobuf::write(&source, &fgb_path)?;
    geopackage::write(&source, &gpkg_path)?;
    #[cfg(feature = "geoparquet")]
    geoparquet::write(&source, &parquet_path)?;
    gml::write(&source,        &gml_path)?;
    gpx::write(&source,        &gpx_path)?;
    kml::write(&source,        &kml_path)?;
    mapinfo::write(&source,    &mif_path)?;

    println!("Written to:");
    println!("  {}.shp / .shx / .dbf / .prj", shp_path.display());
    println!("  {}", json_path.display());
    println!("  {}", fgb_path.display());
    println!("  {}", gpkg_path.display());
    #[cfg(feature = "geoparquet")]
    println!("  {}", parquet_path.display());
    println!("  {}", gml_path.display());
    println!("  {}", gpx_path.display());
    println!("  {}", kml_path.display());
    println!("  {} / {}", mif_path.display(), mif_path.with_extension("mid").display());
    println!();

    // ── Read back from every format ───────────────────────────────────────────
    let from_shp  = shapefile::read(&shp_path)?;
    let from_json = geojson::read(&json_path)?;
    let from_fgb  = flatgeobuf::read(&fgb_path)?;
    let from_gpkg = geopackage::read(&gpkg_path)?;
    #[cfg(feature = "geoparquet")]
    let from_parquet = geoparquet::read(&parquet_path)?;
    let from_gml  = gml::read(&gml_path)?;
    let from_gpx  = gpx::read(&gpx_path)?;
    let from_kml  = kml::read(&kml_path)?;
    let from_mif  = mapinfo::read(&mif_path)?;

    verify_layer(&from_shp,  "Shapefile");
    verify_layer(&from_json, "GeoJSON  ");
    verify_layer(&from_fgb,  "FlatGeobuf");
    verify_layer(&from_gpkg, "GeoPackage");
    #[cfg(feature = "geoparquet")]
    verify_layer(&from_parquet, "GeoParquet");
    verify_layer(&from_gml,  "GML      ");
    verify_layer(&from_gpx,  "GPX      ");
    verify_layer(&from_kml,  "KML      ");
    verify_layer(&from_mif,  "MapInfo  ");

    println!();

    // ── Cross-format conversion: Shapefile → GeoPackage ──────────────────────
    println!("Converting Shapefile → GeoPackage …");
    let shp_layer  = shapefile::read(&shp_path)?;
    let out_path   = dir.join("rivers_from_shp.gpkg");
    geopackage::write(&shp_layer, &out_path)?;
    let verify = geopackage::read(&out_path)?;
    println!("  GeoPackage has {} features ✓", verify.len());

    // ── Cross-format: GeoJSON → FlatGeobuf ────────────────────────────────────
    println!("Converting GeoJSON → FlatGeobuf …");
    let json_layer = geojson::read(&json_path)?;
    let fgb_out    = dir.join("rivers_from_json.fgb");
    flatgeobuf::write(&json_layer, &fgb_out)?;
    let verify2 = flatgeobuf::read(&fgb_out)?;
    println!("  FlatGeobuf has {} features ✓", verify2.len());

    // ── Cross-format: GML → GeoJSON ──────────────────────────────────────────
    println!("Converting GML → GeoJSON …");
    let gml_layer = gml::read(&gml_path)?;
    let json_out = dir.join("rivers_from_gml.geojson");
    geojson::write(&gml_layer, &json_out)?;
    let verify3 = geojson::read(&json_out)?;
    println!("  GeoJSON has {} features ✓", verify3.len());

    // ── Cross-format: KML → GeoPackage ─────────────────────────────────────
    println!("Converting KML → GeoPackage …");
    let kml_layer = kml::read(&kml_path)?;
    let gpkg_out = dir.join("rivers_from_kml.gpkg");
    geopackage::write(&kml_layer, &gpkg_out)?;
    let verify4 = geopackage::read(&gpkg_out)?;
    println!("  GeoPackage has {} features ✓", verify4.len());

    #[cfg(feature = "geoparquet")]
    {
        // ── Cross-format: GeoParquet → GeoJSON ─────────────────────────────
        println!("Converting GeoParquet → GeoJSON …");
        let parquet_layer = geoparquet::read(&parquet_path)?;
        let json_out = dir.join("rivers_from_parquet.geojson");
        geojson::write(&parquet_layer, &json_out)?;
        let verify5 = geojson::read(&json_out)?;
        println!("  GeoJSON has {} features ✓", verify5.len());
    }

    // ── Bbox query ────────────────────────────────────────────────────────────
    let all = from_gpkg;
    let query = wbvector::BBox::new(-80.0, -10.0, -40.0, 10.0);
    let hits  = all.features_in_bbox(&query);
    println!("\nFeatures intersecting bbox [-80,-10 → -40,10]: {}", hits.len());
    for h in hits {
        let name = h.get(&all.schema, "name").unwrap();
        println!("  {name}");
    }

    println!("\nConvert example OK ✓");
    Ok(())
}
