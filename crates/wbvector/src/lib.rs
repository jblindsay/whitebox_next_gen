//! # wbvector
//!
//! Pure-Rust library for reading and writing common vector GIS data formats.
//!
//! ## Supported formats
//!
//! | Format      | Read | Write | Extension |
//! |-------------|------|-------|-----------|
//! | FlatGeobuf  | ✓    | ✓     | `.fgb`    |
//! | GeoJSON     | ✓    | ✓     | `.geojson`|
//! | GeoPackage  | ✓    | ✓     | `.gpkg`   |
//! | GeoParquet* | ✓    | ✓     | `.parquet`|
//! | GML         | ✓    | ✓     | `.gml`    |
//! | GPX         | ✓    | ✓     | `.gpx`    |
//! | KML         | ✓    | ✓     | `.kml`    |
//! | KMZ*        | ✓    | ✓     | `.kmz`    |
//! | MapInfo MIF | ✓    | ✓     | `.mif`    |
//! | OSM PBF**   | ✓    | -     | `.osm.pbf`|
//! | Shapefile   | ✓    | ✓     | `.shp`    |
//!
//! `*` GeoParquet and KMZ supports are optional and require `geoparquet` / `kmz` features.
//! `**` OSM PBF support is optional and requires the `osmpbf` crate feature.
//!
//! ## Common API
//!
//! All format drivers convert to/from the same in-memory types:
//!
//! * [`feature::Layer`]      — named collection of features + schema
//! * [`feature::Feature`]    — optional geometry + attribute values
//! * [`feature::FieldValue`] — typed attribute value
//! * [`geometry::Geometry`]  — OGC Simple Features geometry
//!
//! You can also use crate-level sniffed I/O:
//!
//! ```rust,ignore
//! let layer = wbvector::read("roads.gpkg")?; // auto-detect format
//! wbvector::write(&layer, "roads.fgb", wbvector::VectorFormat::FlatGeobuf)?;
//! # Ok::<(), wbvector::error::GeoError>(())
//! ```
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use wbvector::{flatgeobuf, geojson, geopackage, gml, gpx, kml, mapinfo, shapefile};
//! #[cfg(feature = "geoparquet")]
//! use wbvector::geoparquet;
//! #[cfg(feature = "kmz")]
//! use wbvector::kmz;
//! #[cfg(feature = "osmpbf")]
//! use wbvector::osmpbf;
//! use wbvector::feature::{Layer, FieldDef, FieldType};
//! use wbvector::geometry::{Geometry, GeometryType};
//!
//! // Build a layer in memory
//! let mut layer = Layer::new("cities")
//!     .with_geom_type(GeometryType::Point)
//!     .with_crs_epsg(4326);
//!
//! layer.add_field(FieldDef::new("name",       FieldType::Text));
//! layer.add_field(FieldDef::new("population", FieldType::Integer));
//!
//! layer.add_feature(
//!     Some(Geometry::point(-0.1278, 51.5074)),
//!     &[("name", "London".into()), ("population", 9_000_000i64.into())],
//! )?;
//!
//! // Write to every format
//! flatgeobuf::write(&layer,  "cities.fgb")?;
//! geojson::write(&layer,     "cities.geojson")?;
//! geopackage::write(&layer,  "cities.gpkg")?;
//! gml::write(&layer,         "cities.gml")?;
//! gpx::write(&layer,         "cities.gpx")?;
//! kml::write(&layer,         "cities.kml")?;
//! #[cfg(feature = "kmz")]
//! kmz::write(&layer,         "cities.kmz")?;
//! mapinfo::write(&layer,     "cities.mif")?;
//! shapefile::write(&layer,   "cities")?;       // → cities.shp/.shx/.dbf/.prj
//!
//! // Read back from any format — same Layer type
//! let from_fgb  = flatgeobuf::read("cities.fgb")?;
//! let from_json = geojson::read("cities.geojson")?;
//! let from_gpkg = geopackage::read("cities.gpkg")?;
//! #[cfg(feature = "geoparquet")]
//! geoparquet::write(&layer, "cities.parquet")?;
//! let options = geoparquet::GeoParquetWriteOptions::for_large_files();
//! geoparquet::write_with_options(&layer, "cities_tuned.parquet", &options)?;
//! let fast_options = geoparquet::GeoParquetWriteOptions::for_interactive_files();
//! geoparquet::write_with_options(&layer, "cities_fast.parquet", &fast_options)?;
//! let from_parquet = geoparquet::read("cities.parquet")?;
//! let from_gml  = gml::read("cities.gml")?;
//! let from_gpx  = gpx::read("cities.gpx")?;
//! let from_kml  = kml::read("cities.kml")?;
//! #[cfg(feature = "kmz")]
//! let from_kmz  = kmz::read("cities.kmz")?;
//! let from_mif  = mapinfo::read("cities.mif")?;
//! let from_shp  = shapefile::read("cities")?;
//! #[cfg(feature = "osmpbf")]
//! let from_osm  = osmpbf::read("extract.osm.pbf")?;
//! # Ok::<(), wbvector::error::GeoError>(())
//! ```
//!
//! ## Format conversion
//!
//! Because all drivers share the same [`feature::Layer`] type, conversion
//! between any pair of formats is two lines:
//!
//! ```rust,ignore
//! let layer = shapefile::read("roads")?;
//! geopackage::write(&layer, "roads.gpkg")?;
//! ```
//!
//! ## Dependencies
//!
//! Core external dependencies include `thiserror`, `flatbuffers`, and `wbprojection`.
//! All format codecs — including the SQLite engine powering GeoPackage I/O —
//! are implemented from scratch in pure Rust.

#![deny(missing_docs)]

mod crs;
pub mod error;
pub mod feature;
pub mod flatgeobuf;
pub mod geojson;
pub mod geometry;
pub mod geopackage;
#[cfg(feature = "geoparquet")]
pub mod geoparquet;
pub mod gml;
pub mod gpx;
pub mod kml;
#[cfg(feature = "kmz")]
pub mod kmz;
pub mod mapinfo;
#[cfg(feature = "osmpbf")]
pub mod osmpbf;
pub mod reproject;
pub mod shapefile;

// Re-export the most commonly used types at the crate root
pub use error::{GeoError, Result};
pub use feature::{Crs, Feature, FieldDef, FieldType, FieldValue, Layer, Schema};
pub use geometry::{BBox, Coord, Geometry, GeometryType, Ring};

/// Supported vector formats for crate-level sniffed I/O.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorFormat {
	/// FlatGeobuf binary vector format (`.fgb`).
	FlatGeobuf,
	/// GeoJSON text format (`.geojson`).
	GeoJson,
	/// GeoPackage SQLite container format (`.gpkg`).
	GeoPackage,
	#[cfg(feature = "geoparquet")]
	/// GeoParquet columnar format (`.parquet`).
	GeoParquet,
	/// Geography Markup Language XML format (`.gml`).
	Gml,
	/// GPS Exchange Format XML format (`.gpx`).
	Gpx,
	/// Keyhole Markup Language XML format (`.kml`).
	Kml,
	#[cfg(feature = "kmz")]
	/// Zipped KML container format (`.kmz`).
	Kmz,
	/// MapInfo interchange format (`.mif` + `.mid`).
	MapInfoMif,
	#[cfg(feature = "osmpbf")]
	/// OpenStreetMap PBF binary format (`.osm.pbf`).
	OsmPbf,
	/// ESRI Shapefile dataset (`.shp` + sidecars).
	Shapefile,
}

impl VectorFormat {
	/// Detect format from extension and lightweight file sniffing.
	pub fn detect<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
		let path = path.as_ref();
		let file_name_lc = path
			.file_name()
			.and_then(|s| s.to_str())
			.unwrap_or("")
			.to_ascii_lowercase();
		let ext_lc = path
			.extension()
			.and_then(|s| s.to_str())
			.unwrap_or("")
			.to_ascii_lowercase();

		if file_name_lc.ends_with(".osm.pbf") {
			#[cfg(feature = "osmpbf")]
			{
				return Ok(Self::OsmPbf);
			}
			#[cfg(not(feature = "osmpbf"))]
			{
				return Err(GeoError::NotImplemented(
					"OSM PBF support requires enabling the `osmpbf` feature".into(),
				));
			}
		}

		match ext_lc.as_str() {
			"fgb" => return Ok(Self::FlatGeobuf),
			"geojson" => return Ok(Self::GeoJson),
			"gpkg" => return Ok(Self::GeoPackage),
			"gml" => return Ok(Self::Gml),
			"gpx" => return Ok(Self::Gpx),
			"kml" => return Ok(Self::Kml),
			"mif" => return Ok(Self::MapInfoMif),
			"shp" => return Ok(Self::Shapefile),
			"parquet" => {
				#[cfg(feature = "geoparquet")]
				{
					return Ok(Self::GeoParquet);
				}
				#[cfg(not(feature = "geoparquet"))]
				{
					return Err(GeoError::NotImplemented(
						"GeoParquet support requires enabling the `geoparquet` feature".into(),
					));
				}
			}
			"kmz" => {
				#[cfg(feature = "kmz")]
				{
					return Ok(Self::Kmz);
				}
				#[cfg(not(feature = "kmz"))]
				{
					return Err(GeoError::NotImplemented(
						"KMZ support requires enabling the `kmz` feature".into(),
					));
				}
			}
			"json" => return Ok(Self::GeoJson),
			"xml" => {
				if let Some(kind) = sniff_xml(path)? {
					return Ok(kind);
				}
			}
			_ => {}
		}

		// Shapefile convenience: accept base path without extension.
		if path.extension().is_none() {
			let shp = path.with_extension("shp");
			if shp.exists() {
				return Ok(Self::Shapefile);
			}
		}

		if path.is_file() {
			let sig = read_signature(path, 16)?;
			if sig.starts_with(&flatgeobuf::MAGIC) {
				return Ok(Self::FlatGeobuf);
			}
			if sig.starts_with(b"SQLite format 3\0") {
				return Ok(Self::GeoPackage);
			}
			if sig.starts_with(b"PAR1") {
				#[cfg(feature = "geoparquet")]
				{
					return Ok(Self::GeoParquet);
				}
			}
			if sig.starts_with(b"PK\x03\x04") {
				#[cfg(feature = "kmz")]
				{
					return Ok(Self::Kmz);
				}
			}
			if let Some(kind) = sniff_xml(path)? {
				return Ok(kind);
			}
		}

		Err(GeoError::UnknownFormat(path.display().to_string()))
	}

	/// Read using this format driver.
	pub fn read<P: AsRef<std::path::Path>>(&self, path: P) -> Result<Layer> {
		match self {
			Self::FlatGeobuf => flatgeobuf::read(path),
			Self::GeoJson => geojson::read(path),
			Self::GeoPackage => geopackage::read(path),
			#[cfg(feature = "geoparquet")]
			Self::GeoParquet => geoparquet::read(path),
			Self::Gml => gml::read(path),
			Self::Gpx => gpx::read(path),
			Self::Kml => kml::read(path),
			#[cfg(feature = "kmz")]
			Self::Kmz => kmz::read(path),
			Self::MapInfoMif => mapinfo::read(path),
			#[cfg(feature = "osmpbf")]
			Self::OsmPbf => osmpbf::read(path),
			Self::Shapefile => shapefile::read(path),
		}
	}

	/// Write using this format driver.
	pub fn write<P: AsRef<std::path::Path>>(&self, layer: &Layer, path: P) -> Result<()> {
		match self {
			Self::FlatGeobuf => flatgeobuf::write(layer, path),
			Self::GeoJson => geojson::write(layer, path),
			Self::GeoPackage => geopackage::write(layer, path),
			#[cfg(feature = "geoparquet")]
			Self::GeoParquet => geoparquet::write(layer, path),
			Self::Gml => gml::write(layer, path),
			Self::Gpx => gpx::write(layer, path),
			Self::Kml => kml::write(layer, path),
			#[cfg(feature = "kmz")]
			Self::Kmz => kmz::write(layer, path),
			Self::MapInfoMif => mapinfo::write(layer, path),
			#[cfg(feature = "osmpbf")]
			Self::OsmPbf => Err(GeoError::NotImplemented(
				"OSM PBF writer is not implemented".into(),
			)),
			Self::Shapefile => shapefile::write(layer, path),
		}
	}
}

/// Generic vector read with automatic format sniffing.
pub fn read<P: AsRef<std::path::Path>>(path: P) -> Result<Layer> {
	let fmt = VectorFormat::detect(path.as_ref())?;
	fmt.read(path)
}

/// Generic vector write when the target format is known.
pub fn write<P: AsRef<std::path::Path>>(layer: &Layer, path: P, format: VectorFormat) -> Result<()> {
	format.write(layer, path)
}

fn read_signature(path: &std::path::Path, n: usize) -> Result<Vec<u8>> {
	use std::io::Read;
	let mut f = std::fs::File::open(path)?;
	let mut buf = vec![0u8; n];
	let read_n = f.read(&mut buf)?;
	buf.truncate(read_n);
	Ok(buf)
}

fn sniff_xml(path: &std::path::Path) -> Result<Option<VectorFormat>> {
	use std::io::Read;
	let mut f = match std::fs::File::open(path) {
		Ok(f) => f,
		Err(_) => return Ok(None),
	};
	let mut buf = vec![0u8; 4096];
	let n = f.read(&mut buf)?;
	buf.truncate(n);
	let txt = String::from_utf8_lossy(&buf).to_ascii_lowercase();
	if txt.contains("<kml") {
		return Ok(Some(VectorFormat::Kml));
	}
	if txt.contains("<gpx") {
		return Ok(Some(VectorFormat::Gpx));
	}
	if txt.contains("<gml") || txt.contains("opengis.net/gml") {
		return Ok(Some(VectorFormat::Gml));
	}
	Ok(None)
}

