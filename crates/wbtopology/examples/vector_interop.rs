//! wbvector interoperability example.
//!
//! Usage: cargo run --example vector_interop

use std::path::PathBuf;

use wbtopology::{vector_io, Coord, Geometry, LineString, LinearRing, Polygon, Result};

fn data_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("examples_vector_interop");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn main() -> Result<()> {
    let geoms = vec![
        Geometry::Point(Coord::xy(-0.1278, 51.5074)),
        Geometry::LineString(LineString::new(vec![
            Coord::xy(-0.2, 51.48),
            Coord::xy(-0.1, 51.5),
            Coord::xy(0.0, 51.52),
        ])),
        Geometry::Polygon(Polygon::new(
            LinearRing::new(vec![
                Coord::xy(-0.15, 51.49),
                Coord::xy(-0.05, 51.49),
                Coord::xy(-0.05, 51.55),
                Coord::xy(-0.15, 51.55),
            ]),
            vec![],
        )),
    ];

    let out = data_dir().join("topology_geoms.geojson");
    vector_io::write_geometries(out.to_str().unwrap(), &geoms)?;

    let loaded = vector_io::read_geometries(out.to_str().unwrap())?;
    println!("Read {} geometries from {}", loaded.len(), out.display());
    println!("vector_interop example OK");
    Ok(())
}
