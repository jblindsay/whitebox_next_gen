//! OSM PBF read-only example.
//!
//! Requires feature: `osmpbf`
//! Usage: cargo run --features osmpbf --example osmpbf_io -- path/to/extract.osm.pbf [--highways-only] [--named-only] [--polygons-only] [--tag-keys=name,highway]

#[cfg(feature = "osmpbf")]
fn main() -> wbvector::Result<()> {
    use std::path::PathBuf;
    use wbvector::osmpbf::{self, OsmPbfReadOptions};

    let mut args = std::env::args().skip(1);
    let arg = args.next().ok_or_else(|| {
        wbvector::GeoError::NotImplemented(
            "Usage: cargo run --features osmpbf --example osmpbf_io -- path/to/extract.osm.pbf [--highways-only] [--named-only] [--polygons-only] [--tag-keys=name,highway]".into(),
        )
    })?;

    let mut options = OsmPbfReadOptions::new();
    for flag in args {
        match flag.as_str() {
            "--highways-only" => options = options.with_highways_only(true),
            "--named-only" => options = options.with_named_ways_only(true),
            "--polygons-only" => options = options.with_polygons_only(true),
            _ if flag.starts_with("--tag-keys=") => {
                let keys = flag
                    .trim_start_matches("--tag-keys=")
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_owned())
                    .collect::<Vec<_>>();
                options = options.with_include_tag_keys(keys);
            }
            _ => {
                return Err(wbvector::GeoError::NotImplemented(format!(
                    "Unknown flag '{flag}'. Use --highways-only, --named-only, --polygons-only, --tag-keys=k1,k2"
                )));
            }
        }
    }

    let path = PathBuf::from(arg);
    let layer = osmpbf::read_with_options(&path, &options)?;

    println!("Read {} features from {}", layer.len(), path.display());
    println!("Geometry type: {:?}", layer.geom_type);
    println!("Fields: {}", layer.schema.len());

    Ok(())
}

#[cfg(not(feature = "osmpbf"))]
fn main() {
    eprintln!("Enable feature 'osmpbf': cargo run --features osmpbf --example osmpbf_io -- path/to/extract.osm.pbf [--highways-only] [--named-only] [--polygons-only] [--tag-keys=name,highway]");
}
