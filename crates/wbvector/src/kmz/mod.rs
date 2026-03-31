//! KMZ (zipped KML) reader and writer.
//!
//! This module is available when the crate is built with the `kmz` feature.
//! Internally it stores a single `doc.kml` entry inside a ZIP container.

use std::io::{Cursor, Read, Write};
use std::path::Path;

use zip::write::FileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::{GeoError, Result};
use crate::feature::Layer;
use crate::reproject;

/// Read a KMZ file into a [`Layer`].
pub fn read<P: AsRef<Path>>(path: P) -> Result<Layer> {
    let bytes = std::fs::read(path).map_err(GeoError::Io)?;
    from_bytes(&bytes)
}

/// Parse KMZ bytes into a [`Layer`].
pub fn from_bytes(bytes: &[u8]) -> Result<Layer> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)
        .map_err(|e| GeoError::Kmz(format!("invalid zip archive: {e}")))?;

    let mut preferred_index: Option<usize> = None;
    let mut fallback_index: Option<usize> = None;

    for i in 0..archive.len() {
        let f = archive
            .by_index(i)
            .map_err(|e| GeoError::Kmz(format!("failed reading zip entry {i}: {e}")))?;
        let name = f.name().to_ascii_lowercase();
        if name == "doc.kml" {
            preferred_index = Some(i);
            break;
        }
        if fallback_index.is_none() && name.ends_with(".kml") {
            fallback_index = Some(i);
        }
    }

    let index = preferred_index
        .or(fallback_index)
        .ok_or_else(|| GeoError::Kmz("no .kml entry found in KMZ".into()))?;

    let mut kml_text = String::new();
    let mut f = archive
        .by_index(index)
        .map_err(|e| GeoError::Kmz(format!("failed opening KML entry {index}: {e}")))?;
    f.read_to_string(&mut kml_text)
        .map_err(|e| GeoError::Kmz(format!("failed reading KML entry {index}: {e}")))?;

    crate::kml::parse_str(&kml_text)
}

/// Write a [`Layer`] as KMZ to a file.
pub fn write<P: AsRef<Path>>(layer: &Layer, path: P) -> Result<()> {
    let bytes = to_bytes(layer)?;
    std::fs::write(path, bytes).map_err(GeoError::Io)
}

/// Serialize a [`Layer`] to KMZ bytes.
pub fn to_bytes(layer: &Layer) -> Result<Vec<u8>> {
    let out_layer = prepare_kmz_layer(layer)?;
    let kml = crate::kml::to_string(&out_layer);

    let mut cursor = Cursor::new(Vec::<u8>::new());
    {
        let mut writer = ZipWriter::new(&mut cursor);
        let opts = FileOptions::default().compression_method(CompressionMethod::Deflated);
        writer
            .start_file("doc.kml", opts)
            .map_err(|e| GeoError::Kmz(format!("failed creating doc.kml entry: {e}")))?;
        writer
            .write_all(kml.as_bytes())
            .map_err(|e| GeoError::Kmz(format!("failed writing doc.kml: {e}")))?;
        writer
            .finish()
            .map_err(|e| GeoError::Kmz(format!("failed finalizing KMZ archive: {e}")))?;
    }

    Ok(cursor.into_inner())
}

fn prepare_kmz_layer(layer: &Layer) -> Result<Layer> {
    // KMZ wraps KML, which requires WGS 84 lon/lat coordinates.
    if layer.crs_epsg() == Some(4326) {
        return Ok(layer.clone());
    }

    if layer.crs_epsg().is_some() || layer.crs_wkt().is_some() {
        return reproject::layer_to_epsg(layer, 4326);
    }

    Ok(layer.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature::{FieldDef, FieldType};
    use crate::geometry::{Geometry, GeometryType};

    #[test]
    fn kmz_roundtrip_bytes() {
        let mut layer = Layer::new("places")
            .with_geom_type(GeometryType::Point)
            .with_epsg(4326);
        layer.add_field(FieldDef::new("name", FieldType::Text));
        layer
            .add_feature(Some(Geometry::point(-0.1278, 51.5074)), &[("name", "London".into())])
            .unwrap();

        let bytes = to_bytes(&layer).unwrap();
        let parsed = from_bytes(&bytes).unwrap();

        assert_eq!(parsed.len(), 1);
        assert!(matches!(parsed.features[0].geometry, Some(Geometry::Point(_))));
        assert_eq!(parsed.features[0].get(&parsed.schema, "name").unwrap().as_str(), Some("London"));
    }

    #[test]
    fn kmz_write_reprojects_projected_layer_to_epsg4326() {
        let mut layer = Layer::new("mercator").with_crs_epsg(3857);
        layer
            .add_feature(Some(Geometry::point(111_319.49079327357, 0.0)), &[])
            .unwrap();

        let bytes = to_bytes(&layer).unwrap();
        let parsed = from_bytes(&bytes).unwrap();

        assert_eq!(parsed.crs_epsg(), Some(4326));
        if let Some(Geometry::Point(c)) = &parsed.features[0].geometry {
            assert!((c.x - 1.0).abs() < 1.0e-5);
            assert!(c.y.abs() < 1.0e-9);
        } else {
            panic!("expected Point geometry");
        }
    }
}
