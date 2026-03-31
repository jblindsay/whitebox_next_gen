//! Reprojection example.
//!
//! Demonstrates vector reprojection APIs:
//! - Layer convenience methods (`reproject_to_epsg`)
//! - Reproject module functions (`reproject::layer_from_to_epsg`)
//! - Options-based reprojection (`SplitAt180`, densification, topology policy)
//!
//! Usage: cargo run --example reproject_io

use std::path::PathBuf;

use wbvector::feature::{FieldDef, FieldType, Layer};
use wbvector::geometry::{Coord, Geometry, GeometryType};
use wbvector::{geojson, reproject, Result};

fn data_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("reproject");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn crossing_polygon_layer() -> Layer {
    let mut layer = Layer::new("dateline_poly")
        .with_geom_type(GeometryType::Polygon)
        .with_crs_epsg(4326);
    layer.add_field(FieldDef::new("name", FieldType::Text));

    // Exterior intentionally clockwise, hole intentionally counter-clockwise;
    // SplitAt180 + ValidateAndFixOrientation will normalize orientation.
    let exterior_cw = vec![
        Coord::xy(179.0, 0.0),
        Coord::xy(179.0, 4.0),
        Coord::xy(-179.0, 4.0),
        Coord::xy(-179.0, 0.0),
    ];
    let hole_ccw = vec![
        Coord::xy(179.1, 1.0),
        Coord::xy(179.4, 1.0),
        Coord::xy(179.4, 2.0),
        Coord::xy(179.1, 2.0),
    ];

    layer
        .add_feature(
            Some(Geometry::polygon(exterior_cw, vec![hole_ccw])),
            &[("name", "crossing_polygon".into())],
        )
        .unwrap();

    layer
}

fn sample_layer() -> Layer {
    let mut layer = Layer::new("cities")
        .with_geom_type(GeometryType::Point)
        .with_crs_epsg(4326);
    layer.add_field(FieldDef::new("name", FieldType::Text));

    layer
        .add_feature(Some(Geometry::point(-75.0, 45.0)), &[("name", "Ottawa".into())])
        .unwrap();
    layer
        .add_feature(Some(Geometry::point(-0.1278, 51.5074)), &[("name", "London".into())])
        .unwrap();
    layer
}

fn main() -> Result<()> {
    let dir = data_dir();
    let layer_4326 = sample_layer();

    // Method-style reprojection.
    let layer_3857 = layer_4326.reproject_to_epsg(3857)?;

    // Function-style reprojection back to 4326.
    let layer_back = reproject::layer_from_to_epsg(&layer_3857, 3857, 4326)?;

    let out_3857 = dir.join("cities_3857.geojson");
    let out_back = dir.join("cities_roundtrip_4326.geojson");
    geojson::write(&layer_3857, &out_3857)?;
    geojson::write(&layer_back, &out_back)?;

    println!("Wrote:");
    println!("  {}", out_3857.display());
    println!("  {}", out_back.display());

    println!("\nCRS summary:");
    println!("  source EPSG: {:?}", layer_4326.crs_epsg());
    println!("  projected EPSG: {:?}", layer_3857.crs_epsg());
    println!("  roundtrip EPSG: {:?}", layer_back.crs_epsg());

    if let Some(Geometry::Point(c)) = &layer_3857.features[0].geometry {
        println!("\nFirst projected point (approx Web Mercator): x={:.3}, y={:.3}", c.x, c.y);
    }

    // Options-style reprojection with antimeridian split + topology normalization.
    let crossing = crossing_polygon_layer();
    let options = reproject::VectorReprojectOptions::new()
        .with_failure_policy(reproject::TransformFailurePolicy::SetNullGeometry)
        .with_antimeridian_policy(reproject::AntimeridianPolicy::SplitAt180)
        .with_max_segment_length(0.25)
        .with_topology_policy(reproject::TopologyPolicy::ValidateAndFixOrientation);

    let crossing_split = crossing.reproject_to_epsg_with_options(4326, &options)?;
    let out_split = dir.join("crossing_polygon_split_4326.geojson");
    geojson::write(&crossing_split, &out_split)?;
    println!("  {}", out_split.display());

    if let Some(geom) = &crossing_split.features[0].geometry {
        println!("\nSplit geometry type: {}", geom.geom_type().as_str());
    }

    println!("\nReprojection example OK ✓");
    Ok(())
}
