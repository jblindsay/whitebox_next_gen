//! GeoPackage (`.gpkg`) reader and writer.
//!
//! GeoPackage is an OGC standard (OGC 12-128r18) for storing vector and raster
//! geospatial data in a SQLite 3 database with a well-defined schema.
//!
//! ## Mandatory tables
//! | Table                    | Purpose                              |
//! |--------------------------|--------------------------------------|
//! | `gpkg_spatial_ref_sys`   | SRS / CRS definitions                |
//! | `gpkg_contents`          | Per-layer metadata (bbox, SRS, type) |
//! | `gpkg_geometry_columns`  | Geometry column name and type        |
//!
//! ## Feature tables
//! Each vector layer is a user table containing:
//! * `fid` — integer primary key
//! * a geometry column (BLOB: GeoPackage-WKB)
//! * additional attribute columns
//!
//! ## GeoPackage WKB
//! `GP` (2 bytes) + flags (1 byte) + srs_id (4 bytes LE) + optional envelope
//! (32 bytes for XY) + ISO WKB geometry.

mod sqlite;

use std::path::Path;
use crate::crs;
use crate::error::{GeoError, Result};
use crate::feature::{FieldDef, FieldType, FieldValue, Feature, Layer};
use crate::geometry::{Geometry, GeometryType};
use sqlite::{Db, SqlVal, Row};

// ══════════════════════════════════════════════════════════════════════════════
// Required GeoPackage table DDL
// ══════════════════════════════════════════════════════════════════════════════

const DDL_SRS: &str = "\
CREATE TABLE gpkg_spatial_ref_sys (\
  srs_name TEXT NOT NULL,\
    srs_id INTEGER NOT NULL,\
  organization TEXT NOT NULL,\
  organization_coordsys_id INTEGER NOT NULL,\
  definition TEXT NOT NULL,\
  description TEXT\
)";

const DDL_CONTENTS: &str = "\
CREATE TABLE gpkg_contents (\
    table_name TEXT NOT NULL,\
  data_type TEXT NOT NULL,\
    identifier TEXT,\
  description TEXT DEFAULT '',\
  last_change DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),\
  min_x REAL,\
  min_y REAL,\
  max_x REAL,\
    max_y REAL,\
    srs_id INTEGER\
)";

const DDL_GEOM_COLS: &str = "\
CREATE TABLE gpkg_geometry_columns (\
  table_name TEXT NOT NULL,\
  column_name TEXT NOT NULL,\
  geometry_type_name TEXT NOT NULL,\
  srs_id INTEGER NOT NULL,\
  z TINYINT NOT NULL,\
    m TINYINT NOT NULL\
)";

// ══════════════════════════════════════════════════════════════════════════════
// Public API
// ══════════════════════════════════════════════════════════════════════════════

/// Read the first feature layer from a GeoPackage file.
pub fn read<P: AsRef<Path>>(path: P) -> Result<Layer> {
    let data = std::fs::read(path).map_err(GeoError::Io)?;
    let db   = Db::from_bytes(data)?;
    read_first_layer(&db)
}

/// Read a named layer from a GeoPackage file.
pub fn read_layer<P: AsRef<Path>>(path: P, layer_name: &str) -> Result<Layer> {
    let data = std::fs::read(path).map_err(GeoError::Io)?;
    let db   = Db::from_bytes(data)?;
    extract_layer(&db, layer_name)
}

/// List all feature layer names in a GeoPackage file.
pub fn list_layers<P: AsRef<Path>>(path: P) -> Result<Vec<String>> {
    let data = std::fs::read(path).map_err(GeoError::Io)?;
    let db   = Db::from_bytes(data)?;
    layer_names(&db)
}

/// Write a single [`Layer`] to a GeoPackage file.
pub fn write<P: AsRef<Path>>(layer: &Layer, path: P) -> Result<()> {
    let db = layers_to_db(&[layer])?;
    std::fs::write(path, db.to_bytes()).map_err(GeoError::Io)
}

/// Write multiple layers to a GeoPackage file.
pub fn write_layers<P: AsRef<Path>>(layers: &[&Layer], path: P) -> Result<()> {
    let db = layers_to_db(layers)?;
    std::fs::write(path, db.to_bytes()).map_err(GeoError::Io)
}

// ══════════════════════════════════════════════════════════════════════════════
// DB → Layer
// ══════════════════════════════════════════════════════════════════════════════

fn layer_names(db: &Db) -> Result<Vec<String>> {
    // Try gpkg_contents first
    if db.table_meta("gpkg_contents").is_some() {
        let rows = db.select_all("gpkg_contents")?;
        return Ok(rows.iter()
            .filter(|r| r.get(1).and_then(|v| v.as_str()) == Some("features"))
            .filter_map(|r| r.get(0).and_then(|v| v.as_str()).map(|s| s.to_owned()))
            .collect());
    }
    // Fall back: all non-system tables
    Ok(db.table_names().into_iter()
        .filter(|n| !n.starts_with("gpkg_") && !n.starts_with("sqlite_"))
        .map(|s| s.to_owned())
        .collect())
}

fn read_first_layer(db: &Db) -> Result<Layer> {
    let names = layer_names(db)?;
    let name  = names.into_iter().next()
        .ok_or_else(|| GeoError::GpkgSchema("no feature layers found".into()))?;
    extract_layer(db, &name)
}

fn extract_layer(db: &Db, name: &str) -> Result<Layer> {
    let meta = db.table_meta(name)
        .ok_or_else(|| GeoError::GpkgSchema(format!("table '{name}' not found")))?;

    // Identify geometry column and SRS
    let (geom_col, srs_id, geom_type_name) = geometry_column_info(db, name)?;

    let mut layer = Layer::new(name);
    layer.set_crs_epsg(if srs_id > 0 { Some(srs_id as u32) } else { None });
    layer.set_crs_wkt(
        spatial_ref_wkt(db, srs_id)
            .or_else(|| layer.crs_epsg().and_then(crs::ogc_wkt_from_epsg))
    );
    if layer.crs_epsg().is_none() {
        layer.set_crs_epsg(layer.crs_wkt().and_then(crs::epsg_from_wkt_lenient));
    }
    layer.geom_type = parse_geom_type_name(&geom_type_name);

    // Identify non-geometry, non-fid columns
    let all_cols = &meta.columns;
    let fid_col  = "fid";
    let attr_cols: Vec<(usize, &str)> = all_cols.iter().enumerate()
        .filter(|(_, n)| n.as_str() != fid_col && n.as_str() != geom_col)
        .map(|(i, n)| (i, n.as_str()))
        .collect();

    // Read all rows for schema inference
    let all_rows = db.select_all(name)?;

    // Infer column types
    let inferred = infer_types(&all_rows, &attr_cols);
    for (_, col_name) in &attr_cols {
        let ft = inferred.get(*col_name).copied().unwrap_or(FieldType::Text);
        layer.add_field(FieldDef::new(*col_name, ft));
    }

    let geom_idx = all_cols.iter().position(|n| n == &geom_col);
    let fid_idx  = all_cols.iter().position(|n| n == fid_col);

    for (feat_idx, row) in all_rows.iter().enumerate() {
        let fid = fid_idx
            .and_then(|i| row.get(i))
            .and_then(|v| v.as_i64())
            .unwrap_or(feat_idx as i64) as u64;

        let geom = geom_idx
            .and_then(|i| row.get(i))
            .and_then(|v| v.as_blob())
            .and_then(|b| Geometry::from_gpkg_wkb(b).ok().map(|(g, _)| g));

        let mut attrs = vec![FieldValue::Null; attr_cols.len()];
        for (field_idx, (row_idx, _)) in attr_cols.iter().enumerate() {
            if let Some(sv) = row.get(*row_idx) {
                attrs[field_idx] = sqlval_to_field(sv);
            }
        }

        layer.push(Feature { fid, geometry: geom, attributes: attrs });
    }

    Ok(layer)
}

fn geometry_column_info(db: &Db, table_name: &str) -> Result<(String, i64, String)> {
    if db.table_meta("gpkg_geometry_columns").is_some() {
        if let Ok(rows) = db.select_all("gpkg_geometry_columns") {
            for row in &rows {
                let tn = row.get(0).and_then(|v| v.as_str()).unwrap_or("");
                if tn == table_name {
                    let col   = row.get(1).and_then(|v| v.as_str()).unwrap_or("geom").to_owned();
                    let srs   = row.get(3).and_then(|v| v.as_i64()).unwrap_or(4326);
                    let gtype = row.get(2).and_then(|v| v.as_str()).unwrap_or("GEOMETRY").to_owned();
                    return Ok((col, srs, gtype));
                }
            }
        }
    }
    // Heuristic fallback
    if let Some(meta) = db.table_meta(table_name) {
        for col in &meta.columns {
            let lc = col.to_ascii_lowercase();
            if ["geom","geometry","shape","wkb_geometry","the_geom"].contains(&lc.as_str()) {
                return Ok((col.clone(), 4326, "GEOMETRY".into()));
            }
        }
    }
    Ok(("geom".into(), 4326, "GEOMETRY".into()))
}

fn spatial_ref_wkt(db: &Db, srs_id: i64) -> Option<String> {
    if db.table_meta("gpkg_spatial_ref_sys").is_none() {
        return None;
    }

    let rows = db.select_all("gpkg_spatial_ref_sys").ok()?;
    for row in rows {
        if row.get(1).and_then(|v| v.as_i64()) == Some(srs_id) {
            let definition = row.get(4).and_then(|v| v.as_str())?.trim();
            if definition.is_empty() || definition.eq_ignore_ascii_case("undefined") {
                return None;
            }
            return Some(definition.to_owned());
        }
    }
    None
}

fn infer_types(rows: &[Row], cols: &[(usize, &str)]) -> std::collections::HashMap<String, FieldType> {
    let mut map: std::collections::HashMap<String, FieldType> = std::collections::HashMap::new();
    for row in rows {
        for &(row_idx, col_name) in cols {
            if let Some(sv) = row.get(row_idx) {
                let ft = match sv {
                    SqlVal::Null    => continue,
                    SqlVal::Int(_)  => FieldType::Integer,
                    SqlVal::Real(_) => FieldType::Float,
                    SqlVal::Blob(_) => FieldType::Blob,
                    SqlVal::Text(s) => if looks_like_date(s) { FieldType::Date } else { FieldType::Text },
                };
                let e = map.entry(col_name.to_owned()).or_insert(ft);
                *e = FieldValue::widen_type(*e, ft);
            }
        }
    }
    map
}

fn looks_like_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 10 && b[4] == b'-' && b[7] == b'-'
}

fn sqlval_to_field(v: &SqlVal) -> FieldValue {
    match v {
        SqlVal::Null    => FieldValue::Null,
        SqlVal::Int(n)  => FieldValue::Integer(*n),
        SqlVal::Real(n) => FieldValue::Float(*n),
        SqlVal::Text(s) => FieldValue::Text(s.clone()),
        SqlVal::Blob(b) => FieldValue::Blob(b.clone()),
    }
}

fn parse_geom_type_name(s: &str) -> Option<GeometryType> {
    match s.to_ascii_uppercase().trim_end_matches(|c: char| c == 'Z' || c == 'M') {
        "POINT"              => Some(GeometryType::Point),
        "LINESTRING"         => Some(GeometryType::LineString),
        "POLYGON"            => Some(GeometryType::Polygon),
        "MULTIPOINT"         => Some(GeometryType::MultiPoint),
        "MULTILINESTRING"    => Some(GeometryType::MultiLineString),
        "MULTIPOLYGON"       => Some(GeometryType::MultiPolygon),
        "GEOMETRYCOLLECTION" => Some(GeometryType::GeometryCollection),
        _                    => None,
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Layer → DB
// ══════════════════════════════════════════════════════════════════════════════

fn layers_to_db(layers: &[&Layer]) -> Result<Db> {
    let mut db = Db::new_empty();

    // Create mandatory GeoPackage tables
    db.create_table(DDL_SRS)?;
    db.create_table(DDL_CONTENTS)?;
    db.create_table(DDL_GEOM_COLS)?;

    // Insert standard SRS rows required by the spec
    seed_srs(&mut db)?;

    for layer in layers { write_layer(&mut db, layer)?; }

    Ok(db)
}

fn seed_srs(db: &mut Db) -> Result<()> {
    // WGS 84 (EPSG:4326)
    db.insert("gpkg_spatial_ref_sys", vec![
        SqlVal::Text("WGS 84 geodetic".into()),
        SqlVal::Int(4326),
        SqlVal::Text("EPSG".into()),
        SqlVal::Int(4326),
        SqlVal::Text(r#"GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]"#.into()),
        SqlVal::Null,
    ])?;
    // Undefined Cartesian
    db.insert("gpkg_spatial_ref_sys", vec![
        SqlVal::Text("Undefined Cartesian SRS".into()),
        SqlVal::Int(-1),
        SqlVal::Text("NONE".into()),
        SqlVal::Int(-1),
        SqlVal::Text("undefined".into()),
        SqlVal::Null,
    ])?;
    // Undefined Geographic
    db.insert("gpkg_spatial_ref_sys", vec![
        SqlVal::Text("Undefined geographic SRS".into()),
        SqlVal::Int(0),
        SqlVal::Text("NONE".into()),
        SqlVal::Int(0),
        SqlVal::Text("undefined".into()),
        SqlVal::Null,
    ])?;
    Ok(())
}

fn write_layer(db: &mut Db, layer: &Layer) -> Result<()> {
    let table   = &layer.name;
    let geom_col = "geom";
    let srs_id  = layer.crs_epsg().unwrap_or(4326) as i64;
    let gt_name = layer.geom_type.map(|g| g.as_str().to_ascii_uppercase())
                       .unwrap_or_else(|| "GEOMETRY".into());

    ensure_srs_row(db, layer, srs_id)?;

    // CREATE TABLE for this layer
    let mut col_defs = format!("  fid INTEGER PRIMARY KEY,\n  {geom_col} BLOB");
    for fd in layer.schema.fields() {
        let sql_type = match fd.field_type {
            FieldType::Integer  => "INTEGER",
            FieldType::Float    => "REAL",
            FieldType::Boolean  => "INTEGER",
            FieldType::Blob     => "BLOB",
            _                   => "TEXT",
        };
        col_defs.push_str(&format!(",\n  {} {}", fd.name, sql_type));
    }
    let create_sql = format!("CREATE TABLE {table} (\n{col_defs}\n)");
    db.create_table(&create_sql)?;

    // Register in gpkg_geometry_columns
    db.insert("gpkg_geometry_columns", vec![
        SqlVal::Text(table.clone()),
        SqlVal::Text(geom_col.into()),
        SqlVal::Text(gt_name),
        SqlVal::Int(srs_id),
        SqlVal::Int(0), // z
        SqlVal::Int(0), // m
    ])?;

    // Register in gpkg_contents
    let mut bb_vals = [SqlVal::Null, SqlVal::Null, SqlVal::Null, SqlVal::Null];
    if let Some(bb) = layer.features.iter()
        .filter_map(|f| f.geometry.as_ref().and_then(|g| g.bbox()))
        .reduce(|mut a, b| { a.expand_to(&b); a })
    {
        bb_vals = [SqlVal::Real(bb.min_x), SqlVal::Real(bb.min_y),
                   SqlVal::Real(bb.max_x), SqlVal::Real(bb.max_y)];
    }

    db.insert("gpkg_contents", vec![
        SqlVal::Text(table.clone()),
        SqlVal::Text("features".into()),
        SqlVal::Text(table.clone()),
        SqlVal::Text(String::new()),
        SqlVal::Text("2024-01-01T00:00:00Z".into()),
        bb_vals[0].clone(), bb_vals[1].clone(), bb_vals[2].clone(), bb_vals[3].clone(),
        SqlVal::Int(srs_id),
    ])?;

    // Insert feature rows
    for feat in &layer.features {
        let geom_blob = feat.geometry.as_ref()
            .map(|g| SqlVal::Blob(g.to_gpkg_wkb(srs_id as i32)))
            .unwrap_or(SqlVal::Null);

        let mut row: Vec<SqlVal> = vec![SqlVal::Null, geom_blob]; // fid = NULL → AUTOINCREMENT

        for val in &feat.attributes {
            row.push(field_to_sqlval(val));
        }
        // Pad if feature has fewer attributes than schema columns
        while row.len() < 2 + layer.schema.len() {
            row.push(SqlVal::Null);
        }

        db.insert(table, row)?;
    }

    Ok(())
}

fn ensure_srs_row(db: &mut Db, layer: &Layer, srs_id: i64) -> Result<()> {
    if srs_id <= 0 || srs_row_exists(db, srs_id)? {
        return Ok(());
    }

    let epsg = srs_id as u32;
    let definition = layer.crs_wkt().map(|w| w.to_owned())
        .or_else(|| crs::ogc_wkt_from_epsg(epsg))
        .unwrap_or_else(|| "undefined".to_owned());
    let srs_name = crs::crs_name_from_epsg(epsg)
        .unwrap_or_else(|| format!("EPSG:{epsg}"));

    db.insert("gpkg_spatial_ref_sys", vec![
        SqlVal::Text(srs_name),
        SqlVal::Int(srs_id),
        SqlVal::Text("EPSG".into()),
        SqlVal::Int(srs_id),
        SqlVal::Text(definition),
        SqlVal::Null,
    ])?;

    Ok(())
}

fn srs_row_exists(db: &Db, srs_id: i64) -> Result<bool> {
    let rows = db.select_all("gpkg_spatial_ref_sys")?;
    Ok(rows.iter().any(|row| row.get(1).and_then(|v| v.as_i64()) == Some(srs_id)))
}

fn field_to_sqlval(v: &FieldValue) -> SqlVal {
    match v {
        FieldValue::Null        => SqlVal::Null,
        FieldValue::Integer(n)  => SqlVal::Int(*n),
        FieldValue::Float(n)    => SqlVal::Real(*n),
        FieldValue::Boolean(b)  => SqlVal::Int(*b as i64),
        FieldValue::Text(s) | FieldValue::Date(s) | FieldValue::DateTime(s) => SqlVal::Text(s.clone()),
        FieldValue::Blob(b)     => SqlVal::Blob(b.clone()),
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature::{FieldDef, FieldType};
    use crate::geometry::{Coord, Geometry, GeometryType};

    fn point_layer() -> Layer {
        let mut l = Layer::new("cities")
            .with_geom_type(GeometryType::Point)
            .with_epsg(4326);
        l.add_field(FieldDef::new("name",       FieldType::Text));
        l.add_field(FieldDef::new("population", FieldType::Integer));
        l.add_feature(
            Some(Geometry::point(-0.1278, 51.5074)),
            &[("name", "London".into()), ("population", 9_000_000i64.into())],
        ).unwrap();
        l.add_feature(
            Some(Geometry::point(2.3522, 48.8566)),
            &[("name", "Paris".into()), ("population", 2_100_000i64.into())],
        ).unwrap();
        l
    }

    fn polygon_layer() -> Layer {
        let mut l = Layer::new("regions").with_geom_type(GeometryType::Polygon).with_epsg(4326);
        l.add_field(FieldDef::new("region_id", FieldType::Integer));
        l.add_feature(
            Some(Geometry::polygon(
                vec![Coord::xy(0.,0.), Coord::xy(10.,0.), Coord::xy(10.,10.), Coord::xy(0.,10.)],
                vec![],
            )),
            &[("region_id", 42i64.into())],
        ).unwrap();
        l
    }

    fn large_point_layer(count: usize) -> Layer {
        let mut l = Layer::new("large_points")
            .with_geom_type(GeometryType::Point)
            .with_epsg(2958);
        l.add_field(FieldDef::new("name", FieldType::Text));
        l.add_field(FieldDef::new("id", FieldType::Integer));

        for i in 0..count {
            let x = 500_000.0 + (i as f64) * 0.5;
            let y = 4_820_000.0 + (i as f64) * 0.5;
            let name = format!("pt_{i:05}_{}", "x".repeat(96));
            l.add_feature(
                Some(Geometry::point(x, y)),
                &[("name", name.into()), ("id", (i as i64).into())],
            )
            .unwrap();
        }

        l
    }

    #[test]
    fn roundtrip_points() {
        let dir  = tempfile::tempdir().unwrap();
        let path = dir.path().join("cities.gpkg");
        let l1   = point_layer();
        write(&l1, &path).unwrap();
        let l2 = read(&path).unwrap();
        assert_eq!(l2.len(), 2);
        if let Some(Geometry::Point(c)) = &l2[0].geometry {
            assert!((c.x - (-0.1278)).abs() < 1e-6);
        } else { panic!("expected Point"); }
    }

    #[test]
    fn attributes_preserved() {
        let dir  = tempfile::tempdir().unwrap();
        let path = dir.path().join("cities.gpkg");
        write(&point_layer(), &path).unwrap();
        let l = read(&path).unwrap();
        let name = l[0].get(&l.schema, "name").unwrap();
        assert_eq!(name.as_str(), Some("London"));
        let pop  = l[0].get(&l.schema, "population").unwrap().as_i64();
        assert_eq!(pop, Some(9_000_000));
    }

    #[test]
    fn roundtrip_polygon() {
        let dir  = tempfile::tempdir().unwrap();
        let path = dir.path().join("regions.gpkg");
        write(&polygon_layer(), &path).unwrap();
        let l = read(&path).unwrap();
        assert_eq!(l.len(), 1);
        assert!(matches!(l[0].geometry, Some(Geometry::Polygon { .. })));
    }

    #[test]
    fn list_layers_works() {
        let dir   = tempfile::tempdir().unwrap();
        let path  = dir.path().join("multi.gpkg");
        write_layers(&[&point_layer(), &polygon_layer()], &path).unwrap();
        let names = list_layers(&path).unwrap();
        assert!(names.contains(&"cities".to_owned()));
        assert!(names.contains(&"regions".to_owned()));
    }

    #[test]
    fn read_named_layer() {
        let dir   = tempfile::tempdir().unwrap();
        let path  = dir.path().join("multi.gpkg");
        write_layers(&[&point_layer(), &polygon_layer()], &path).unwrap();
        let l = read_layer(&path, "regions").unwrap();
        assert_eq!(l.len(), 1);
    }

    #[test]
    fn preserves_non_default_epsg_and_wkt() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mercator.gpkg");

        let mut layer = Layer::new("mercator_pts")
            .with_geom_type(GeometryType::Point)
            .with_epsg(3857);
        layer.add_field(FieldDef::new("name", FieldType::Text));
        layer.add_feature(
            Some(Geometry::point(0.0, 0.0)),
            &[("name", "origin".into())],
        ).unwrap();

        write(&layer, &path).unwrap();
        let out = read(&path).unwrap();

        assert_eq!(out.crs_epsg(), Some(3857));
        assert!(out.crs_wkt().map(|w| !w.trim().is_empty()).unwrap_or(false));
    }

    #[test]
    fn large_end_to_end_roundtrip_preserves_all_features() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("large_points.gpkg");
        let expected = 5000usize;

        let layer = large_point_layer(expected);
        write(&layer, &path).unwrap();

        let out = read(&path).unwrap();
        assert_eq!(out.len(), expected);

        let out_named = read_layer(&path, "large_points").unwrap();
        assert_eq!(out_named.len(), expected);

        let db = Db::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let rows = db.select_all("large_points").unwrap();
        assert_eq!(rows.len(), expected);
    }
}
