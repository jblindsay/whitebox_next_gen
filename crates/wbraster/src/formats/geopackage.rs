//! GeoPackage raster (`.gpkg`) reader and writer.
//!
//! Current profile:
//! - Multi-band raster I/O (`raster_tiles` or `raster_tiles_bN` tables)
//! - Native data type persistence via raw tile payloads (default for non-`U8`)
//! - Optional image tile encoding (`PNG`/`JPEG`) for quantized imagery workflows
//! - One or more zoom levels (`zoom_level=0..max_zoom`)
//! - Side metadata persistence in `wbraster_gpkg_raster_metadata` and
//!   `wbraster_gpkg_kv_metadata`

use std::io::Cursor;
use std::io::{Read, Write};

use crate::error::{RasterError, Result};
use crate::raster::{DataType, Raster, RasterConfig};
use crate::formats::geopackage_sqlite::{Db, SqlVal};

const DEFAULT_TILE_SIZE: usize = 256;
const MIN_TILE_SIZE: usize = 16;
const MAX_TILE_SIZE: usize = 4096;

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

const DDL_TILE_MATRIX_SET: &str = "\
CREATE TABLE gpkg_tile_matrix_set (\
  table_name TEXT NOT NULL PRIMARY KEY,\
  srs_id INTEGER NOT NULL,\
  min_x REAL NOT NULL,\
  min_y REAL NOT NULL,\
  max_x REAL NOT NULL,\
  max_y REAL NOT NULL\
)";

const DDL_TILE_MATRIX: &str = "\
CREATE TABLE gpkg_tile_matrix (\
  table_name TEXT NOT NULL,\
  zoom_level INTEGER NOT NULL,\
  matrix_width INTEGER NOT NULL,\
  matrix_height INTEGER NOT NULL,\
  tile_width INTEGER NOT NULL,\
  tile_height INTEGER NOT NULL,\
  pixel_x_size REAL NOT NULL,\
  pixel_y_size REAL NOT NULL\
)";

const DDL_EXTENSIONS: &str = "\
CREATE TABLE gpkg_extensions (\
    table_name TEXT,\
    column_name TEXT,\
    extension_name TEXT NOT NULL,\
    definition TEXT NOT NULL,\
    scope TEXT NOT NULL\
)";

const DDL_WBRASTER_RASTER_METADATA: &str = "\
CREATE TABLE wbraster_gpkg_raster_metadata (\
    dataset_name TEXT NOT NULL,\
    base_table_name TEXT NOT NULL,\
    band_count INTEGER NOT NULL,\
    data_type TEXT NOT NULL,\
    nodata REAL NOT NULL,\
    tile_encoding TEXT NOT NULL,\
    max_zoom INTEGER NOT NULL,\
    raw_compression TEXT NOT NULL\
)";

const DDL_WBRASTER_KV_METADATA: &str = "\
CREATE TABLE wbraster_gpkg_kv_metadata (\
    dataset_name TEXT NOT NULL,\
    key TEXT NOT NULL,\
    value TEXT NOT NULL\
)";

fn ddl_tile_table(table_name: &str) -> String {
        format!(
                "CREATE TABLE {table_name} (\
    id INTEGER PRIMARY KEY AUTOINCREMENT,\
    zoom_level INTEGER NOT NULL,\
    tile_column INTEGER NOT NULL,\
    tile_row INTEGER NOT NULL,\
    tile_data BLOB NOT NULL\
)"
        )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TileFormat {
        Png,
        Jpeg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StoredTileEncoding {
    Raw,
    Png,
    Jpeg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RawTileCompression {
    None,
    Deflate,
}

impl RawTileCompression {
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Deflate => "deflate",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "none" => Some(Self::None),
            "deflate" | "zlib" => Some(Self::Deflate),
            _ => None,
        }
    }
}

impl StoredTileEncoding {
    fn as_str(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Png => "png",
            Self::Jpeg => "jpeg",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "raw" => Some(Self::Raw),
            "png" => Some(Self::Png),
            "jpg" | "jpeg" => Some(Self::Jpeg),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct GeoPackageWriteOptions {
        tile_format: TileFormat,
        tile_format_explicit: bool,
        tile_encoding: Option<StoredTileEncoding>,
    raw_compression: Option<RawTileCompression>,
        jpeg_quality: u8,
        max_zoom: usize,
    tile_size: usize,
    dataset_name: String,
    base_table_name: String,
}

#[derive(Debug, Clone)]
struct DatasetMetadata {
    dataset_name: String,
    base_table_name: String,
    band_count: usize,
    data_type: DataType,
    nodata: f64,
    tile_encoding: StoredTileEncoding,
    raw_compression: RawTileCompression,
}

/// Read GeoPackage raster.
pub fn read(path: &str) -> Result<Raster> {
    let data = std::fs::read(path)?;
    let db = Db::from_bytes(data)
        .map_err(|e| RasterError::Other(format!("GeoPackage open failed: {e}")))?;

    if let Some(meta) = read_wbraster_dataset_metadata(
        &db,
        preferred_dataset_name_from_env().as_deref(),
        false,
    )? {
        return read_wbraster_dataset(&db, &meta);
    }

    let (table_name, srs_id) = find_tile_table(&db)?;
    let (min_x, min_y, max_x, max_y) = read_tile_matrix_set(&db, &table_name)?;
    let matrix = read_zoom0_matrix(&db, &table_name)?;

    let cols_from_bounds = ((max_x - min_x) / matrix.pixel_x_size).round().max(1.0) as usize;
    let rows_from_bounds = ((max_y - min_y) / matrix.pixel_y_size).round().max(1.0) as usize;

    let mut raster = Raster::new(RasterConfig {
        cols: cols_from_bounds,
        rows: rows_from_bounds,
        bands: 1,
        x_min: min_x,
        y_min: min_y,
        cell_size: matrix.pixel_x_size,
        cell_size_y: Some(matrix.pixel_y_size),
        nodata: 0.0,
        data_type: DataType::U8,
        crs: if srs_id > 0 {
            crate::crs_info::CrsInfo::from_epsg(srs_id as u32)
        } else {
            crate::crs_info::CrsInfo::default()
        },
        ..Default::default()
    });

    let rows = db
        .select_all(&table_name)
        .map_err(|e| RasterError::Other(format!("GeoPackage tile read failed: {e}")))?;

    for row in rows {
        let zoom = row.get(1).and_then(SqlVal::as_i64).unwrap_or(-1);
        if zoom != matrix.zoom_level {
            continue;
        }
        let tile_col = row.get(2).and_then(SqlVal::as_i64).unwrap_or(-1);
        let tile_row = row.get(3).and_then(SqlVal::as_i64).unwrap_or(-1);
        let tile_blob = row
            .get(4)
            .and_then(SqlVal::as_blob)
            .ok_or_else(|| RasterError::CorruptData("missing tile_data blob".into()))?;

        let (tile_w, tile_h, tile_px) = decode_tile_to_gray_u8(tile_blob)?;

        let tile_col = tile_col as usize;
        let tile_row = tile_row as usize;
        for y in 0..tile_h {
            let global_row = tile_row * matrix.tile_height + y;
            if global_row >= raster.rows {
                continue;
            }
            for x in 0..tile_w {
                let global_col = tile_col * matrix.tile_width + x;
                if global_col >= raster.cols {
                    continue;
                }
                let v = tile_px[y * tile_w + x] as f64;
                raster.set(0, global_row as isize, global_col as isize, v)?;
            }
        }
    }

    Ok(raster)
}

/// Read a specific `wbraster` dataset from a GeoPackage by dataset name.
pub fn read_dataset(path: &str, dataset_name: &str) -> Result<Raster> {
    let data = std::fs::read(path)?;
    let db = Db::from_bytes(data)
        .map_err(|e| RasterError::Other(format!("GeoPackage open failed: {e}")))?;

    if let Some(meta) = read_wbraster_dataset_metadata(&db, Some(dataset_name), true)? {
        return read_wbraster_dataset(&db, &meta);
    }

    Err(RasterError::CorruptData(format!(
        "dataset '{}' not found in wbraster_gpkg_raster_metadata",
        dataset_name
    )))
}

/// List all dataset names recorded in `wbraster_gpkg_raster_metadata`.
pub fn list_datasets(path: &str) -> Result<Vec<String>> {
    let data = std::fs::read(path)?;
    let db = Db::from_bytes(data)
        .map_err(|e| RasterError::Other(format!("GeoPackage open failed: {e}")))?;

    if db.table_meta("wbraster_gpkg_raster_metadata").is_none() {
        return Ok(Vec::new());
    }

    let rows = db
        .select_all("wbraster_gpkg_raster_metadata")
        .map_err(|e| RasterError::Other(format!("GeoPackage metadata read failed: {e}")))?;
    let mut names = Vec::new();
    for row in rows {
        let Some(name) = row.first().and_then(SqlVal::as_str) else {
            continue;
        };
        if !names.iter().any(|n| n == name) {
            names.push(name.to_owned());
        }
    }
    Ok(names)
}

/// Write GeoPackage raster (tiled, optional pyramids, multi-band + native dtype).
pub fn write(raster: &Raster, path: &str) -> Result<()> {
    let opts = parse_write_options(raster);
    let encoding = resolve_tile_encoding(raster, &opts);
    let raw_compression = resolve_raw_tile_compression(&opts, encoding);

    let mut db = Db::new_empty();
    db.create_table(DDL_SRS)
        .map_err(|e| RasterError::Other(format!("GeoPackage schema create failed: {e}")))?;
    db.create_table(DDL_CONTENTS)
        .map_err(|e| RasterError::Other(format!("GeoPackage schema create failed: {e}")))?;
    db.create_table(DDL_TILE_MATRIX_SET)
        .map_err(|e| RasterError::Other(format!("GeoPackage schema create failed: {e}")))?;
    db.create_table(DDL_TILE_MATRIX)
        .map_err(|e| RasterError::Other(format!("GeoPackage schema create failed: {e}")))?;
    db.create_table(DDL_EXTENSIONS)
        .map_err(|e| RasterError::Other(format!("GeoPackage schema create failed: {e}")))?;
    db.create_table(DDL_WBRASTER_RASTER_METADATA)
        .map_err(|e| RasterError::Other(format!("GeoPackage schema create failed: {e}")))?;
    db.create_table(DDL_WBRASTER_KV_METADATA)
        .map_err(|e| RasterError::Other(format!("GeoPackage schema create failed: {e}")))?;

    seed_srs(&mut db)?;

    let srs_id = raster.crs.epsg.unwrap_or(4326) as i64;
    if srs_id != 4326 && srs_id > 0 {
        ensure_epsg_srs_row(&mut db, srs_id)?;
    }

    let extent = raster.extent();
    let dataset_name = opts.dataset_name.as_str();
    let base_table_name = opts.base_table_name.as_str();

    register_wbraster_extensions(&mut db, base_table_name)?;
    register_metadata_table_contents(&mut db, dataset_name, srs_id)?;

    db.insert(
        "wbraster_gpkg_raster_metadata",
        vec![
            SqlVal::Text(dataset_name.into()),
            SqlVal::Text(base_table_name.into()),
            SqlVal::Int(raster.bands as i64),
            SqlVal::Text(raster.data_type.as_str().into()),
            SqlVal::Real(raster.nodata),
            SqlVal::Text(encoding.as_str().into()),
            SqlVal::Int(opts.max_zoom as i64),
            SqlVal::Text(raw_compression.as_str().into()),
        ],
    )
    .map_err(|e| RasterError::Other(format!("GeoPackage dataset metadata insert failed: {e}")))?;

    for (k, v) in &raster.metadata {
        db.insert(
            "wbraster_gpkg_kv_metadata",
            vec![
                SqlVal::Text(dataset_name.into()),
                SqlVal::Text(k.clone()),
                SqlVal::Text(v.clone()),
            ],
        )
        .map_err(|e| RasterError::Other(format!("GeoPackage key/value metadata insert failed: {e}")))?;
    }

    for band in 0..raster.bands {
        let table_name = band_table_name(base_table_name, band, raster.bands);
        db.create_table(&ddl_tile_table(&table_name))
            .map_err(|e| RasterError::Other(format!("GeoPackage schema create failed: {e}")))?;

        db.insert(
            "gpkg_contents",
            vec![
                SqlVal::Text(table_name.clone()),
                SqlVal::Text("tiles".into()),
                SqlVal::Text(table_name.clone()),
                SqlVal::Text(format!("wbraster GeoPackage raster band {}", band + 1)),
                SqlVal::Text("2024-01-01T00:00:00Z".into()),
                SqlVal::Real(extent.x_min),
                SqlVal::Real(extent.y_min),
                SqlVal::Real(extent.x_max),
                SqlVal::Real(extent.y_max),
                SqlVal::Int(srs_id),
            ],
        )
        .map_err(|e| RasterError::Other(format!("GeoPackage contents insert failed: {e}")))?;

        db.insert(
            "gpkg_tile_matrix_set",
            vec![
                SqlVal::Text(table_name.clone()),
                SqlVal::Int(srs_id),
                SqlVal::Real(extent.x_min),
                SqlVal::Real(extent.y_min),
                SqlVal::Real(extent.x_max),
                SqlVal::Real(extent.y_max),
            ],
        )
        .map_err(|e| RasterError::Other(format!("GeoPackage tile_matrix_set insert failed: {e}")))?;

        let levels = build_pyramid_levels_for_band(raster, band, opts.max_zoom)?;
        for (zoom_level, level) in levels.iter().enumerate() {
            let matrix_width = level.cols.div_ceil(opts.tile_size);
            let matrix_height = level.rows.div_ceil(opts.tile_size);

            db.insert(
                "gpkg_tile_matrix",
                vec![
                    SqlVal::Text(table_name.clone()),
                    SqlVal::Int(zoom_level as i64),
                    SqlVal::Int(matrix_width as i64),
                    SqlVal::Int(matrix_height as i64),
                    SqlVal::Int(opts.tile_size as i64),
                    SqlVal::Int(opts.tile_size as i64),
                    SqlVal::Real(level.pixel_x_size),
                    SqlVal::Real(level.pixel_y_size),
                ],
            )
            .map_err(|e| RasterError::Other(format!("GeoPackage tile_matrix insert failed: {e}")))?;

            for tile_row in 0..matrix_height {
                for tile_col in 0..matrix_width {
                    let tile = extract_level_tile(level, tile_col, tile_row, opts.tile_size);
                    let tile_data = encode_tile_blob(
                        &tile,
                        raster.data_type,
                        encoding,
                        raw_compression,
                        opts.jpeg_quality,
                        opts.tile_size,
                        opts.tile_size,
                    )?;
                    db.insert(
                        &table_name,
                        vec![
                            SqlVal::Null,
                            SqlVal::Int(zoom_level as i64),
                            SqlVal::Int(tile_col as i64),
                            SqlVal::Int(tile_row as i64),
                            SqlVal::Blob(tile_data),
                        ],
                    )
                    .map_err(|e| RasterError::Other(format!("GeoPackage tile insert failed: {e}")))?;
                }
            }
        }
    }

    std::fs::write(path, db.to_bytes())?;
    Ok(())
}

#[derive(Debug, Clone)]
struct LevelGrid {
    cols: usize,
    rows: usize,
    pixel_x_size: f64,
    pixel_y_size: f64,
    nodata: f64,
    data: Vec<f64>,
}

fn parse_write_options(raster: &Raster) -> GeoPackageWriteOptions {
    let mut tile_format = TileFormat::Png;
    let mut tile_format_explicit = false;
    let mut tile_encoding = None;
    let mut raw_compression = None;
    let mut jpeg_quality = 85u8;
    let mut max_zoom = 0usize;
    let mut tile_size = DEFAULT_TILE_SIZE;
    let mut dataset_name = "wbraster_dataset".to_owned();
    let mut base_table_name = "raster_tiles".to_owned();

    for (k, v) in &raster.metadata {
        let key = k.trim().to_ascii_lowercase();
        let val = v.trim().to_ascii_lowercase();
        match key.as_str() {
            "gpkg_tile_format" => {
                tile_format_explicit = true;
                if matches!(val.as_str(), "jpg" | "jpeg") {
                    tile_format = TileFormat::Jpeg;
                } else {
                    tile_format = TileFormat::Png;
                }
            }
            "gpkg_tile_encoding" => {
                tile_encoding = StoredTileEncoding::from_str(&val);
            }
            "gpkg_raw_compression" => {
                if let Some(comp) = RawTileCompression::from_str(&val) {
                    raw_compression = Some(comp);
                }
            }
            "gpkg_jpeg_quality" => {
                if let Ok(q) = val.parse::<u8>() {
                    jpeg_quality = q.clamp(1, 100);
                }
            }
            "gpkg_max_zoom" => {
                if let Ok(z) = val.parse::<usize>() {
                    max_zoom = z;
                }
            }
            "gpkg_tile_size" => {
                if let Ok(size) = val.parse::<usize>() {
                    if (MIN_TILE_SIZE..=MAX_TILE_SIZE).contains(&size) {
                        tile_size = size;
                    }
                }
            }
            "gpkg_dataset_name" => {
                if let Some(name) = sanitize_sql_identifier(v.trim()) {
                    dataset_name = name;
                }
            }
            "gpkg_base_table_name" => {
                if let Some(name) = sanitize_sql_identifier(v.trim()) {
                    base_table_name = name;
                }
            }
            _ => {}
        }
    }

    GeoPackageWriteOptions {
        tile_format,
        tile_format_explicit,
        tile_encoding,
        raw_compression,
        jpeg_quality,
        max_zoom,
        tile_size,
        dataset_name,
        base_table_name,
    }
}

fn sanitize_sql_identifier(name: &str) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut chars = trimmed.chars();
    let first = chars.next()?;
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return None;
    }
    if chars.any(|c| !(c == '_' || c.is_ascii_alphanumeric())) {
        return None;
    }
    Some(trimmed.to_owned())
}

fn register_wbraster_extensions(db: &mut Db, base_table_name: &str) -> Result<()> {
    db.insert(
        "gpkg_extensions",
        vec![
            SqlVal::Text(base_table_name.into()),
            SqlVal::Text("tile_data".into()),
            SqlVal::Text("org.whiteboxgeo.wbraster.raw_tiles".into()),
            SqlVal::Text("https://www.whiteboxgeo.com/specs/wbraster-gpkg-raster-raw-tiles".into()),
            SqlVal::Text("read-write".into()),
        ],
    )
    .map_err(|e| RasterError::Other(format!("GeoPackage extension insert failed: {e}")))?;

    db.insert(
        "gpkg_extensions",
        vec![
            SqlVal::Null,
            SqlVal::Null,
            SqlVal::Text("org.whiteboxgeo.wbraster.metadata".into()),
            SqlVal::Text("https://www.whiteboxgeo.com/specs/wbraster-gpkg-raster-metadata".into()),
            SqlVal::Text("read-write".into()),
        ],
    )
    .map_err(|e| RasterError::Other(format!("GeoPackage extension insert failed: {e}")))?;

    Ok(())
}

fn register_metadata_table_contents(db: &mut Db, dataset_name: &str, srs_id: i64) -> Result<()> {
    db.insert(
        "gpkg_contents",
        vec![
            SqlVal::Text("wbraster_gpkg_raster_metadata".into()),
            SqlVal::Text("attributes".into()),
            SqlVal::Text(format!("{dataset_name}_meta")),
            SqlVal::Text("wbraster GeoPackage raster metadata".into()),
            SqlVal::Text("2024-01-01T00:00:00Z".into()),
            SqlVal::Null,
            SqlVal::Null,
            SqlVal::Null,
            SqlVal::Null,
            SqlVal::Int(srs_id),
        ],
    )
    .map_err(|e| RasterError::Other(format!("GeoPackage contents insert failed: {e}")))?;

    db.insert(
        "gpkg_contents",
        vec![
            SqlVal::Text("wbraster_gpkg_kv_metadata".into()),
            SqlVal::Text("attributes".into()),
            SqlVal::Text(format!("{dataset_name}_kv")),
            SqlVal::Text("wbraster GeoPackage key/value metadata".into()),
            SqlVal::Text("2024-01-01T00:00:00Z".into()),
            SqlVal::Null,
            SqlVal::Null,
            SqlVal::Null,
            SqlVal::Null,
            SqlVal::Int(srs_id),
        ],
    )
    .map_err(|e| RasterError::Other(format!("GeoPackage contents insert failed: {e}")))?;

    Ok(())
}

fn resolve_tile_encoding(raster: &Raster, opts: &GeoPackageWriteOptions) -> StoredTileEncoding {
    if let Some(enc) = opts.tile_encoding {
        return enc;
    }
    if opts.tile_format_explicit {
        return match opts.tile_format {
            TileFormat::Png => StoredTileEncoding::Png,
            TileFormat::Jpeg => StoredTileEncoding::Jpeg,
        };
    }
    if raster.data_type == DataType::U8 {
        StoredTileEncoding::Png
    } else {
        StoredTileEncoding::Raw
    }
}

fn resolve_raw_tile_compression(
    opts: &GeoPackageWriteOptions,
    encoding: StoredTileEncoding,
) -> RawTileCompression {
    if encoding != StoredTileEncoding::Raw {
        return RawTileCompression::None;
    }
    opts.raw_compression.unwrap_or(RawTileCompression::Deflate)
}

fn build_pyramid_levels_for_band(raster: &Raster, band: usize, max_zoom: usize) -> Result<Vec<LevelGrid>> {
    let mut level0 = Vec::with_capacity(raster.cols * raster.rows);
    for row in 0..raster.rows {
        for col in 0..raster.cols {
            let mut v = raster.get(band as isize, row as isize, col as isize);
            if raster.is_nodata(v) || !v.is_finite() {
                v = raster.nodata;
            }
            level0.push(v);
        }
    }

    let mut levels = vec![LevelGrid {
        cols: raster.cols,
        rows: raster.rows,
        pixel_x_size: raster.cell_size_x,
        pixel_y_size: raster.cell_size_y,
        nodata: raster.nodata,
        data: level0,
    }];

    for _ in 0..max_zoom {
        let prev = levels.last().expect("level0 exists");
        if prev.cols == 1 && prev.rows == 1 {
            break;
        }
        levels.push(downsample_level(prev));
    }
    Ok(levels)
}

fn downsample_level(prev: &LevelGrid) -> LevelGrid {
    let next_cols = prev.cols.div_ceil(2);
    let next_rows = prev.rows.div_ceil(2);
    let mut out = vec![prev.nodata; next_cols * next_rows];

    for y in 0..next_rows {
        for x in 0..next_cols {
            let src_x = x * 2;
            let src_y = y * 2;
            let mut sum = 0.0f64;
            let mut count = 0usize;
            for dy in 0..2 {
                for dx in 0..2 {
                    let px = src_x + dx;
                    let py = src_y + dy;
                    if px < prev.cols && py < prev.rows {
                        let value = prev.data[py * prev.cols + px];
                        if value.is_finite()
                            && if prev.nodata.is_nan() {
                                !value.is_nan()
                            } else {
                                (value - prev.nodata).abs() >= 1e-10 * prev.nodata.abs().max(1.0)
                            }
                        {
                            sum += value;
                            count += 1;
                        }
                    }
                }
            }
            out[y * next_cols + x] = if count > 0 {
                sum / count as f64
            } else {
                prev.nodata
            };
        }
    }

    LevelGrid {
        cols: next_cols,
        rows: next_rows,
        pixel_x_size: prev.pixel_x_size * 2.0,
        pixel_y_size: prev.pixel_y_size * 2.0,
        nodata: prev.nodata,
        data: out,
    }
}

fn extract_level_tile(level: &LevelGrid, tile_col: usize, tile_row: usize, tile_size: usize) -> Vec<f64> {
    let mut tile = vec![level.nodata; tile_size * tile_size];
    for y in 0..tile_size {
        let row = tile_row * tile_size + y;
        if row >= level.rows {
            continue;
        }
        for x in 0..tile_size {
            let col = tile_col * tile_size + x;
            if col >= level.cols {
                continue;
            }
            tile[y * tile_size + x] = level.data[row * level.cols + col];
        }
    }
    tile
}

fn encode_tile_blob(
    tile: &[f64],
    data_type: DataType,
    encoding: StoredTileEncoding,
    raw_compression: RawTileCompression,
    jpeg_quality: u8,
    tile_width: usize,
    tile_height: usize,
) -> Result<Vec<u8>> {
    match encoding {
        StoredTileEncoding::Raw => encode_raw_tile(tile, data_type, raw_compression),
        StoredTileEncoding::Png => {
            let q: Vec<u8> = tile
                .iter()
                .map(|v| v.round().clamp(0.0, 255.0) as u8)
                .collect();
            encode_png_gray_u8(tile_width as u32, tile_height as u32, &q)
        }
        StoredTileEncoding::Jpeg => {
            let q: Vec<u8> = tile
                .iter()
                .map(|v| v.round().clamp(0.0, 255.0) as u8)
                .collect();
            encode_jpeg_gray_u8(tile_width as u16, tile_height as u16, &q, jpeg_quality)
        }
    }
}

fn encode_raw_tile(tile: &[f64], data_type: DataType, compression: RawTileCompression) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(tile.len() * data_type.size_bytes());
    match data_type {
        DataType::U8 => {
            for &v in tile {
                out.push(v as u8);
            }
        }
        DataType::I8 => {
            for &v in tile {
                out.push((v as i8) as u8);
            }
        }
        DataType::U16 => {
            for &v in tile {
                out.extend_from_slice(&(v as u16).to_le_bytes());
            }
        }
        DataType::I16 => {
            for &v in tile {
                out.extend_from_slice(&(v as i16).to_le_bytes());
            }
        }
        DataType::U32 => {
            for &v in tile {
                out.extend_from_slice(&(v as u32).to_le_bytes());
            }
        }
        DataType::I32 => {
            for &v in tile {
                out.extend_from_slice(&(v as i32).to_le_bytes());
            }
        }
        DataType::U64 => {
            for &v in tile {
                out.extend_from_slice(&(v as u64).to_le_bytes());
            }
        }
        DataType::I64 => {
            for &v in tile {
                out.extend_from_slice(&(v as i64).to_le_bytes());
            }
        }
        DataType::F32 => {
            for &v in tile {
                out.extend_from_slice(&(v as f32).to_le_bytes());
            }
        }
        DataType::F64 => {
            for &v in tile {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
    }
    match compression {
        RawTileCompression::None => Ok(out),
        RawTileCompression::Deflate => compress_deflate(&out),
    }
}

fn decode_raw_tile_to_f64(
    bytes: &[u8],
    data_type: DataType,
    compression: RawTileCompression,
    expected_cells: usize,
) -> Result<Vec<f64>> {
    let decompressed = match compression {
        RawTileCompression::None => bytes.to_vec(),
        RawTileCompression::Deflate => decompress_deflate(bytes)?,
    };

    let expected_bytes = expected_cells
        .checked_mul(data_type.size_bytes())
        .ok_or_else(|| RasterError::CorruptData("raw tile size overflow".into()))?;
    if decompressed.len() < expected_bytes {
        return Err(RasterError::CorruptData(format!(
            "raw tile too small: expected at least {expected_bytes} bytes, found {}",
            decompressed.len()
        )));
    }

    let mut out = Vec::with_capacity(expected_cells);
    match data_type {
        DataType::U8 => {
            out.extend(decompressed[..expected_cells].iter().map(|v| *v as f64));
        }
        DataType::I8 => {
            out.extend(decompressed[..expected_cells].iter().map(|v| (*v as i8) as f64));
        }
        DataType::U16 => {
            for chunk in decompressed[..expected_bytes].chunks_exact(2) {
                out.push(u16::from_le_bytes([chunk[0], chunk[1]]) as f64);
            }
        }
        DataType::I16 => {
            for chunk in decompressed[..expected_bytes].chunks_exact(2) {
                out.push(i16::from_le_bytes([chunk[0], chunk[1]]) as f64);
            }
        }
        DataType::U32 => {
            for chunk in decompressed[..expected_bytes].chunks_exact(4) {
                out.push(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as f64);
            }
        }
        DataType::I32 => {
            for chunk in decompressed[..expected_bytes].chunks_exact(4) {
                out.push(i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as f64);
            }
        }
        DataType::U64 => {
            for chunk in decompressed[..expected_bytes].chunks_exact(8) {
                out.push(u64::from_le_bytes([
                    chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
                ]) as f64);
            }
        }
        DataType::I64 => {
            for chunk in decompressed[..expected_bytes].chunks_exact(8) {
                out.push(i64::from_le_bytes([
                    chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
                ]) as f64);
            }
        }
        DataType::F32 => {
            for chunk in decompressed[..expected_bytes].chunks_exact(4) {
                out.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as f64);
            }
        }
        DataType::F64 => {
            for chunk in decompressed[..expected_bytes].chunks_exact(8) {
                out.push(f64::from_le_bytes([
                    chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
                ]));
            }
        }
    }
    Ok(out)
}

fn compress_deflate(bytes: &[u8]) -> Result<Vec<u8>> {
    use flate2::{write::ZlibEncoder, Compression};
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(bytes)
        .map_err(|e| RasterError::Other(format!("deflate write failed: {e}")))?;
    encoder
        .finish()
        .map_err(|e| RasterError::Other(format!("deflate finish failed: {e}")))
}

fn decompress_deflate(bytes: &[u8]) -> Result<Vec<u8>> {
    use flate2::read::ZlibDecoder;
    let mut decoder = ZlibDecoder::new(bytes);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|e| RasterError::CorruptData(format!("deflate decode failed: {e}")))?;
    Ok(out)
}

fn band_table_name(base_table_name: &str, band: usize, total_bands: usize) -> String {
    if total_bands <= 1 {
        base_table_name.to_owned()
    } else {
        format!("{base_table_name}_b{}", band + 1)
    }
}

fn read_wbraster_dataset_metadata(
    db: &Db,
    preferred_name: Option<&str>,
    require_preferred: bool,
) -> Result<Option<DatasetMetadata>> {
    if db.table_meta("wbraster_gpkg_raster_metadata").is_none() {
        return Ok(None);
    }

    let rows = db
        .select_all("wbraster_gpkg_raster_metadata")
        .map_err(|e| RasterError::Other(format!("GeoPackage metadata read failed: {e}")))?;
    if rows.is_empty() {
        return Ok(None);
    }

    let mut candidates = Vec::with_capacity(rows.len());
    for row in &rows {
        candidates.push(parse_dataset_metadata_row(row));
    }

    validate_dataset_metadata_consistency(&candidates)?;

    if require_preferred {
        if let Some(name) = preferred_name {
            let found = candidates
                .iter()
                .find(|m| m.dataset_name == name && dataset_has_required_tables(db, m))
                .cloned();
            return Ok(found);
        }
        return Ok(None);
    }

    if let Some(chosen) = choose_dataset_metadata(&candidates, preferred_name, db) {
        return Ok(Some(chosen));
    }

    Ok(candidates.into_iter().next())
}

fn validate_dataset_metadata_consistency(candidates: &[DatasetMetadata]) -> Result<()> {
    for (idx, left) in candidates.iter().enumerate() {
        for right in candidates.iter().skip(idx + 1) {
            if left.dataset_name == right.dataset_name
                && !dataset_metadata_compatible(left, right)
            {
                return Err(RasterError::CorruptData(format!(
                    "conflicting metadata rows for dataset '{}' in wbraster_gpkg_raster_metadata",
                    left.dataset_name
                )));
            }
        }
    }
    Ok(())
}

fn dataset_metadata_compatible(a: &DatasetMetadata, b: &DatasetMetadata) -> bool {
    a.base_table_name == b.base_table_name
        && a.band_count == b.band_count
        && a.data_type == b.data_type
        && a.tile_encoding == b.tile_encoding
        && a.raw_compression == b.raw_compression
        && if a.nodata.is_nan() {
            b.nodata.is_nan()
        } else {
            (a.nodata - b.nodata).abs() <= 1e-12 * a.nodata.abs().max(1.0)
        }
}

fn parse_dataset_metadata_row(row: &[SqlVal]) -> DatasetMetadata {
    let dataset_name = row
        .first()
        .and_then(SqlVal::as_str)
        .unwrap_or("wbraster_dataset")
        .to_owned();
    let base_table_name = row
        .get(1)
        .and_then(SqlVal::as_str)
        .unwrap_or("raster_tiles")
        .to_owned();
    let band_count = row.get(2).and_then(SqlVal::as_i64).unwrap_or(1).max(1) as usize;
    let data_type = row
        .get(3)
        .and_then(SqlVal::as_str)
        .and_then(DataType::from_str)
        .unwrap_or(DataType::U8);
    let nodata = row.get(4).and_then(SqlVal::as_f64).unwrap_or(0.0);
    let tile_encoding = row
        .get(5)
        .and_then(SqlVal::as_str)
        .and_then(StoredTileEncoding::from_str)
        .unwrap_or(StoredTileEncoding::Raw);
    let raw_compression = row
        .get(7)
        .and_then(SqlVal::as_str)
        .and_then(RawTileCompression::from_str)
        .unwrap_or(RawTileCompression::None);
    DatasetMetadata {
        dataset_name,
        base_table_name,
        band_count,
        data_type,
        nodata,
        tile_encoding,
        raw_compression,
    }
}

fn preferred_dataset_name_from_env() -> Option<String> {
    let value = std::env::var("WBRASTER_GPKG_DATASET").ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn choose_dataset_metadata(
    candidates: &[DatasetMetadata],
    preferred_name: Option<&str>,
    db: &Db,
) -> Option<DatasetMetadata> {
    if let Some(name) = preferred_name {
        if let Some(m) = candidates
            .iter()
            .find(|m| m.dataset_name == name && dataset_has_required_tables(db, m))
        {
            return Some(m.clone());
        }
    }

    if let Some(m) = candidates
        .iter()
        .find(|m| m.dataset_name == "wbraster_dataset" && dataset_has_required_tables(db, m))
    {
        return Some(m.clone());
    }

    candidates
        .iter()
        .find(|m| dataset_has_required_tables(db, m))
        .cloned()
}

fn dataset_has_required_tables(db: &Db, meta: &DatasetMetadata) -> bool {
    for band in 0..meta.band_count {
        let table_name = band_table_name(&meta.base_table_name, band, meta.band_count);
        if db.table_meta(&table_name).is_none() {
            return false;
        }
    }
    true
}

fn read_wbraster_dataset(db: &Db, meta: &DatasetMetadata) -> Result<Raster> {
    let first_table = band_table_name(&meta.base_table_name, 0, meta.band_count);
    let srs_id = read_srs_id_for_table(db, &first_table).unwrap_or(4326);
    let (min_x, min_y, max_x, max_y) = read_tile_matrix_set(db, &first_table)?;
    let matrix = read_zoom0_matrix(db, &first_table)?;

    let cols_from_bounds = ((max_x - min_x) / matrix.pixel_x_size).round().max(1.0) as usize;
    let rows_from_bounds = ((max_y - min_y) / matrix.pixel_y_size).round().max(1.0) as usize;

    let mut metadata = Vec::new();
    if db.table_meta("wbraster_gpkg_kv_metadata").is_some() {
        let kv_rows = db
            .select_all("wbraster_gpkg_kv_metadata")
            .map_err(|e| RasterError::Other(format!("GeoPackage key/value metadata read failed: {e}")))?;
        for row in kv_rows {
            if row.first().and_then(SqlVal::as_str) != Some(meta.dataset_name.as_str()) {
                continue;
            }
            let key = row.get(1).and_then(SqlVal::as_str).unwrap_or("").to_owned();
            let value = row.get(2).and_then(SqlVal::as_str).unwrap_or("").to_owned();
            metadata.push((key, value));
        }
    }

    let mut raster = Raster::new(RasterConfig {
        cols: cols_from_bounds,
        rows: rows_from_bounds,
        bands: meta.band_count,
        x_min: min_x,
        y_min: min_y,
        cell_size: matrix.pixel_x_size,
        cell_size_y: Some(matrix.pixel_y_size),
        nodata: meta.nodata,
        data_type: meta.data_type,
        crs: if srs_id > 0 {
            crate::crs_info::CrsInfo::from_epsg(srs_id as u32)
        } else {
            crate::crs_info::CrsInfo::default()
        },
        metadata,
        ..Default::default()
    });

    for band in 0..meta.band_count {
        let table_name = band_table_name(&meta.base_table_name, band, meta.band_count);
        let band_matrix = read_zoom0_matrix(db, &table_name)?;
        let rows = db
            .select_all(&table_name)
            .map_err(|e| RasterError::Other(format!("GeoPackage tile read failed: {e}")))?;
        let expected_cells = band_matrix.tile_width * band_matrix.tile_height;

        for row in rows {
            let zoom = row.get(1).and_then(SqlVal::as_i64).unwrap_or(-1);
            if zoom != band_matrix.zoom_level {
                continue;
            }
            let tile_col = row.get(2).and_then(SqlVal::as_i64).unwrap_or(-1);
            let tile_row = row.get(3).and_then(SqlVal::as_i64).unwrap_or(-1);
            let tile_blob = row
                .get(4)
                .and_then(SqlVal::as_blob)
                .ok_or_else(|| RasterError::CorruptData("missing tile_data blob".into()))?;

            let tile_values = match meta.tile_encoding {
                StoredTileEncoding::Raw => decode_raw_tile_to_f64(
                    tile_blob,
                    meta.data_type,
                    meta.raw_compression,
                    expected_cells,
                )?,
                StoredTileEncoding::Png | StoredTileEncoding::Jpeg => {
                    let (tile_w, tile_h, tile_px) = decode_tile_to_gray_u8(tile_blob)?;
                    if tile_w != band_matrix.tile_width || tile_h != band_matrix.tile_height {
                        return Err(RasterError::CorruptData("image tile dimensions do not match tile matrix".into()));
                    }
                    tile_px.into_iter().map(|v| v as f64).collect()
                }
            };

            let tile_col = tile_col as usize;
            let tile_row = tile_row as usize;
            for y in 0..band_matrix.tile_height {
                let global_row = tile_row * band_matrix.tile_height + y;
                if global_row >= raster.rows {
                    continue;
                }
                for x in 0..band_matrix.tile_width {
                    let global_col = tile_col * band_matrix.tile_width + x;
                    if global_col >= raster.cols {
                        continue;
                    }
                    let value = tile_values[y * band_matrix.tile_width + x];
                    raster.set(band as isize, global_row as isize, global_col as isize, value)?;
                }
            }
        }
    }

    Ok(raster)
}

#[derive(Debug, Clone, Copy)]
struct TileMatrixInfo {
    zoom_level: i64,
    tile_width: usize,
    tile_height: usize,
    pixel_x_size: f64,
    pixel_y_size: f64,
}

fn find_tile_table(db: &Db) -> Result<(String, i64)> {
    if let Some(_meta) = db.table_meta("gpkg_contents") {
        let rows = db
            .select_all("gpkg_contents")
            .map_err(|e| RasterError::Other(format!("GeoPackage contents read failed: {e}")))?;
        for row in rows {
            let data_type = row.get(1).and_then(SqlVal::as_str).unwrap_or("");
            if data_type == "tiles" {
                let table_name = row
                    .first()
                    .and_then(SqlVal::as_str)
                    .ok_or_else(|| RasterError::CorruptData("gpkg_contents.table_name missing".into()))?
                    .to_owned();
                let srs_id = row.get(9).and_then(SqlVal::as_i64).unwrap_or(4326);
                return Ok((table_name, srs_id));
            }
        }
    }

    Err(RasterError::CorruptData(
        "no tile table registered in gpkg_contents".into(),
    ))
}

fn read_srs_id_for_table(db: &Db, table_name: &str) -> Option<i64> {
    if db.table_meta("gpkg_contents").is_none() {
        return None;
    }
    let rows = db.select_all("gpkg_contents").ok()?;
    for row in rows {
        if row.first().and_then(SqlVal::as_str) == Some(table_name) {
            return row.get(9).and_then(SqlVal::as_i64);
        }
    }
    None
}

fn read_tile_matrix_set(db: &Db, table_name: &str) -> Result<(f64, f64, f64, f64)> {
    let rows = db
        .select_all("gpkg_tile_matrix_set")
        .map_err(|e| RasterError::Other(format!("GeoPackage tile_matrix_set read failed: {e}")))?;
    for row in rows {
        if row.first().and_then(SqlVal::as_str) == Some(table_name) {
            let min_x = row.get(2).and_then(SqlVal::as_f64).unwrap_or(0.0);
            let min_y = row.get(3).and_then(SqlVal::as_f64).unwrap_or(0.0);
            let max_x = row.get(4).and_then(SqlVal::as_f64).unwrap_or(0.0);
            let max_y = row.get(5).and_then(SqlVal::as_f64).unwrap_or(0.0);
            return Ok((min_x, min_y, max_x, max_y));
        }
    }
    Err(RasterError::CorruptData(format!(
        "gpkg_tile_matrix_set row not found for table '{table_name}'"
    )))
}

fn read_zoom0_matrix(db: &Db, table_name: &str) -> Result<TileMatrixInfo> {
    let rows = db
        .select_all("gpkg_tile_matrix")
        .map_err(|e| RasterError::Other(format!("GeoPackage tile_matrix read failed: {e}")))?;

    let mut best: Option<TileMatrixInfo> = None;
    for row in rows {
        if row.first().and_then(SqlVal::as_str) != Some(table_name) {
            continue;
        }
        let zoom = row.get(1).and_then(SqlVal::as_i64).unwrap_or(0);
        let info = TileMatrixInfo {
            zoom_level: zoom,
            tile_width: row.get(4).and_then(SqlVal::as_i64).unwrap_or(DEFAULT_TILE_SIZE as i64) as usize,
            tile_height: row.get(5).and_then(SqlVal::as_i64).unwrap_or(DEFAULT_TILE_SIZE as i64) as usize,
            pixel_x_size: row.get(6).and_then(SqlVal::as_f64).unwrap_or(1.0),
            pixel_y_size: row.get(7).and_then(SqlVal::as_f64).unwrap_or(1.0),
        };

        match best {
            None => best = Some(info),
            Some(cur) if info.zoom_level < cur.zoom_level => best = Some(info),
            _ => {}
        }
    }

    best.ok_or_else(|| {
        RasterError::CorruptData(format!("gpkg_tile_matrix row not found for table '{table_name}'"))
    })
}

fn seed_srs(db: &mut Db) -> Result<()> {
    db.insert(
        "gpkg_spatial_ref_sys",
        vec![
            SqlVal::Text("WGS 84 geodetic".into()),
            SqlVal::Int(4326),
            SqlVal::Text("EPSG".into()),
            SqlVal::Int(4326),
            SqlVal::Text(
                r#"GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]"#
                    .into(),
            ),
            SqlVal::Null,
        ],
    )
    .map_err(|e| RasterError::Other(format!("GeoPackage SRS seed failed: {e}")))?;

    db.insert(
        "gpkg_spatial_ref_sys",
        vec![
            SqlVal::Text("Undefined Cartesian SRS".into()),
            SqlVal::Int(-1),
            SqlVal::Text("NONE".into()),
            SqlVal::Int(-1),
            SqlVal::Text("undefined".into()),
            SqlVal::Null,
        ],
    )
    .map_err(|e| RasterError::Other(format!("GeoPackage SRS seed failed: {e}")))?;

    db.insert(
        "gpkg_spatial_ref_sys",
        vec![
            SqlVal::Text("Undefined geographic SRS".into()),
            SqlVal::Int(0),
            SqlVal::Text("NONE".into()),
            SqlVal::Int(0),
            SqlVal::Text("undefined".into()),
            SqlVal::Null,
        ],
    )
    .map_err(|e| RasterError::Other(format!("GeoPackage SRS seed failed: {e}")))?;

    Ok(())
}

fn ensure_epsg_srs_row(db: &mut Db, srs_id: i64) -> Result<()> {
    let rows = db
        .select_all("gpkg_spatial_ref_sys")
        .map_err(|e| RasterError::Other(format!("GeoPackage SRS lookup failed: {e}")))?;
    for row in rows {
        if row.get(1).and_then(SqlVal::as_i64) == Some(srs_id) {
            return Ok(());
        }
    }
    db.insert(
        "gpkg_spatial_ref_sys",
        vec![
            SqlVal::Text(format!("EPSG:{srs_id}")),
            SqlVal::Int(srs_id),
            SqlVal::Text("EPSG".into()),
            SqlVal::Int(srs_id),
            SqlVal::Text("undefined".into()),
            SqlVal::Text("Inserted by wbraster GeoPackage writer".into()),
        ],
    )
    .map_err(|e| RasterError::Other(format!("GeoPackage SRS insert failed: {e}")))?;
    Ok(())
}

fn encode_png_gray_u8(width: u32, height: u32, data: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| RasterError::Other(format!("PNG header write failed: {e}")))?;
        writer
            .write_image_data(data)
            .map_err(|e| RasterError::Other(format!("PNG data write failed: {e}")))?;
    }
    Ok(out)
}

fn encode_jpeg_gray_u8(width: u16, height: u16, data: &[u8], quality: u8) -> Result<Vec<u8>> {
    use jpeg_encoder::{ColorType, Encoder};
    let mut out = Vec::new();
    let enc = Encoder::new(&mut out, quality);
    enc.encode(data, width, height, ColorType::Luma)
        .map_err(|e| RasterError::Other(format!("JPEG encode failed: {e}")))?;
    Ok(out)
}

fn decode_tile_to_gray_u8(bytes: &[u8]) -> Result<(usize, usize, Vec<u8>)> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return decode_png_to_gray_u8(bytes);
    }
    if bytes.starts_with(&[0xFF, 0xD8]) {
        return decode_jpeg_to_gray_u8(bytes);
    }
    Err(RasterError::UnsupportedDataType(
        "GeoPackage tile_data must be PNG or JPEG for current MVP reader".into(),
    ))
}

fn decode_png_to_gray_u8(bytes: &[u8]) -> Result<(usize, usize, Vec<u8>)> {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder
        .read_info()
        .map_err(|e| RasterError::Other(format!("PNG read_info failed: {e}")))?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| RasterError::Other(format!("PNG next_frame failed: {e}")))?;
    let src = &buf[..info.buffer_size()];
    let w = info.width as usize;
    let h = info.height as usize;

    let gray = match info.color_type {
        png::ColorType::Grayscale => src.to_vec(),
        png::ColorType::GrayscaleAlpha => src.chunks_exact(2).map(|c| c[0]).collect(),
        png::ColorType::Rgb => src
            .chunks_exact(3)
            .map(|c| ((c[0] as u16 + c[1] as u16 + c[2] as u16) / 3) as u8)
            .collect(),
        png::ColorType::Rgba => src
            .chunks_exact(4)
            .map(|c| ((c[0] as u16 + c[1] as u16 + c[2] as u16) / 3) as u8)
            .collect(),
        png::ColorType::Indexed => {
            return Err(RasterError::UnsupportedDataType(
                "indexed PNG tiles are not supported in GeoPackage reader".into(),
            ));
        }
    };

    Ok((w, h, gray))
}

fn decode_jpeg_to_gray_u8(bytes: &[u8]) -> Result<(usize, usize, Vec<u8>)> {
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(bytes));
    let pixels = decoder
        .decode()
        .map_err(|e| RasterError::Other(format!("JPEG decode failed: {e}")))?;
    let info = decoder
        .info()
        .ok_or_else(|| RasterError::CorruptData("JPEG decode produced no info".into()))?;

    let w = usize::from(info.width);
    let h = usize::from(info.height);
    let gray = match info.pixel_format {
        jpeg_decoder::PixelFormat::L8 => pixels,
        jpeg_decoder::PixelFormat::RGB24 => pixels
            .chunks_exact(3)
            .map(|c| ((c[0] as u16 + c[1] as u16 + c[2] as u16) / 3) as u8)
            .collect(),
        jpeg_decoder::PixelFormat::CMYK32 => pixels
            .chunks_exact(4)
            .map(|c| {
                let r = 255u16.saturating_sub((u16::from(c[0]) + u16::from(c[3])).min(255));
                let g = 255u16.saturating_sub((u16::from(c[1]) + u16::from(c[3])).min(255));
                let b = 255u16.saturating_sub((u16::from(c[2]) + u16::from(c[3])).min(255));
                ((r + g + b) / 3) as u8
            })
            .collect(),
        _ => {
            return Err(RasterError::UnsupportedDataType(
                "unsupported JPEG tile pixel format".into(),
            ));
        }
    };

    Ok((w, h, gray))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::formats::geopackage_sqlite::Db;

    #[test]
    fn geopackage_roundtrip_single_band_u8() {
        let mut raster = Raster::new(RasterConfig {
            cols: 300,
            rows: 280,
            bands: 1,
            x_min: 100.0,
            y_min: 200.0,
            cell_size: 2.0,
            nodata: 0.0,
            data_type: DataType::U8,
            crs: crate::crs_info::CrsInfo::from_epsg(4326),
            ..Default::default()
        });
        for row in 0..raster.rows {
            for col in 0..raster.cols {
                let v = ((row + col) % 256) as f64;
                raster.set(0, row as isize, col as isize, v).unwrap();
            }
        }

        let dir = tempdir().unwrap();
        let path = dir.path().join("test_raster.gpkg");
        write(&raster, path.to_str().unwrap()).unwrap();

        let r2 = read(path.to_str().unwrap()).unwrap();
        assert_eq!(r2.cols, raster.cols);
        assert_eq!(r2.rows, raster.rows);
        assert_eq!(r2.crs.epsg, Some(4326));
        assert_eq!(r2.get(0, 0, 0), raster.get(0, 0, 0));
        assert_eq!(r2.get(0, 42, 17), raster.get(0, 42, 17));
        assert_eq!(r2.get(0, 279, 299), raster.get(0, 279, 299));
    }

    #[test]
    fn geopackage_roundtrip_multiband_native_i16() {
        let mut raster = Raster::new(RasterConfig {
            cols: 64,
            rows: 48,
            bands: 2,
            x_min: 10.0,
            y_min: 20.0,
            cell_size: 1.0,
            nodata: -32768.0,
            data_type: DataType::I16,
            ..Default::default()
        });

        raster.metadata.push(("custom_key".into(), "custom_value".into()));
        for row in 0..raster.rows {
            for col in 0..raster.cols {
                let v0 = (row as i32 - col as i32) as f64;
                let v1 = (row as i32 * 2 + col as i32) as f64;
                raster.set(0, row as isize, col as isize, v0).unwrap();
                raster.set(1, row as isize, col as isize, v1).unwrap();
            }
        }

        let dir = tempdir().unwrap();
        let path = dir.path().join("multiband_i16.gpkg");
        write(&raster, path.to_str().unwrap()).unwrap();

        let r2 = read(path.to_str().unwrap()).unwrap();
        assert_eq!(r2.cols, raster.cols);
        assert_eq!(r2.rows, raster.rows);
        assert_eq!(r2.bands, 2);
        assert_eq!(r2.data_type, DataType::I16);
        assert_eq!(r2.nodata, raster.nodata);
        assert!(r2.metadata.iter().any(|(k, v)| k == "custom_key" && v == "custom_value"));
        assert_eq!(r2.get(0, 3, 5), raster.get(0, 3, 5));
        assert_eq!(r2.get(1, 3, 5), raster.get(1, 3, 5));
        assert_eq!(r2.get(0, 47, 63), raster.get(0, 47, 63));
        assert_eq!(r2.get(1, 47, 63), raster.get(1, 47, 63));
    }

    #[test]
    fn geopackage_write_pyramid_jpeg_from_metadata() {
        let mut raster = Raster::new(RasterConfig {
            cols: 520,
            rows: 400,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: 0.0,
            data_type: DataType::U8,
            ..Default::default()
        });
        raster.metadata.push(("gpkg_tile_format".into(), "jpeg".into()));
        raster.metadata.push(("gpkg_max_zoom".into(), "2".into()));
        raster.metadata.push(("gpkg_jpeg_quality".into(), "70".into()));

        for row in 0..raster.rows {
            for col in 0..raster.cols {
                raster.set(0, row as isize, col as isize, ((row * 3 + col) % 256) as f64).unwrap();
            }
        }

        let dir = tempdir().unwrap();
        let path = dir.path().join("pyramid_jpeg.gpkg");
        write(&raster, path.to_str().unwrap()).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        let db = Db::from_bytes(bytes).unwrap();
        let tm_rows = db.select_all("gpkg_tile_matrix").unwrap();
        assert_eq!(tm_rows.len(), 3);

        let tile_rows = db.select_all("raster_tiles").unwrap();
        assert!(!tile_rows.is_empty());
        let jpeg_tile = tile_rows[0][4].as_blob().unwrap();
        assert!(jpeg_tile.starts_with(&[0xFF, 0xD8]));

        let r2 = read(path.to_str().unwrap()).unwrap();
        assert_eq!(r2.cols, raster.cols);
        assert_eq!(r2.rows, raster.rows);
    }

    #[test]
    fn geopackage_write_honors_custom_tile_size() {
        let mut raster = Raster::new(RasterConfig {
            cols: 300,
            rows: 260,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: 0.0,
            data_type: DataType::U8,
            ..Default::default()
        });
        raster.metadata.push(("gpkg_tile_size".into(), "128".into()));

        for row in 0..raster.rows {
            for col in 0..raster.cols {
                raster.set(0, row as isize, col as isize, ((row + col) % 256) as f64).unwrap();
            }
        }

        let dir = tempdir().unwrap();
        let path = dir.path().join("tile_size_128.gpkg");
        write(&raster, path.to_str().unwrap()).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        let db = Db::from_bytes(bytes).unwrap();
        let tm_rows = db.select_all("gpkg_tile_matrix").unwrap();
        assert!(!tm_rows.is_empty());
        let row = &tm_rows[0];
        assert_eq!(row[4].as_i64(), Some(128));
        assert_eq!(row[5].as_i64(), Some(128));
    }

    #[test]
    fn geopackage_raw_deflate_roundtrip() {
        let mut raster = Raster::new(RasterConfig {
            cols: 96,
            rows: 80,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        });
        raster.metadata.push(("gpkg_tile_encoding".into(), "raw".into()));
        raster.metadata.push(("gpkg_raw_compression".into(), "deflate".into()));

        for row in 0..raster.rows {
            for col in 0..raster.cols {
                raster
                    .set(0, row as isize, col as isize, ((row as f64) * 0.5) + (col as f64))
                    .unwrap();
            }
        }

        let dir = tempdir().unwrap();
        let path = dir.path().join("raw_deflate.gpkg");
        write(&raster, path.to_str().unwrap()).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        let db = Db::from_bytes(bytes).unwrap();
        let meta_rows = db.select_all("wbraster_gpkg_raster_metadata").unwrap();
        assert_eq!(meta_rows.len(), 1);
        assert_eq!(meta_rows[0][5].as_str(), Some("raw"));
        assert_eq!(meta_rows[0][7].as_str(), Some("deflate"));

        let r2 = read(path.to_str().unwrap()).unwrap();
        assert_eq!(r2.data_type, DataType::F32);
        assert!((r2.get(0, 7, 5) - raster.get(0, 7, 5)).abs() < 1e-6);
        assert!((r2.get(0, 79, 95) - raster.get(0, 79, 95)).abs() < 1e-6);
    }

    #[test]
    fn geopackage_raw_defaults_to_deflate_when_unspecified() {
        let mut raster = Raster::new(RasterConfig {
            cols: 48,
            rows: 40,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        });
        raster.metadata.push(("gpkg_tile_encoding".into(), "raw".into()));

        for row in 0..raster.rows {
            for col in 0..raster.cols {
                raster
                    .set(0, row as isize, col as isize, ((row as f64) * 1.25) + (col as f64))
                    .unwrap();
            }
        }

        let dir = tempdir().unwrap();
        let path = dir.path().join("raw_default_deflate.gpkg");
        write(&raster, path.to_str().unwrap()).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        let db = Db::from_bytes(bytes).unwrap();
        let meta_rows = db.select_all("wbraster_gpkg_raster_metadata").unwrap();
        assert_eq!(meta_rows.len(), 1);
        assert_eq!(meta_rows[0][5].as_str(), Some("raw"));
        assert_eq!(meta_rows[0][7].as_str(), Some("deflate"));

        let r2 = read(path.to_str().unwrap()).unwrap();
        assert_eq!(r2.data_type, DataType::F32);
        assert!((r2.get(0, 9, 3) - raster.get(0, 9, 3)).abs() < 1e-6);
        assert!((r2.get(0, 39, 47) - raster.get(0, 39, 47)).abs() < 1e-6);
    }

    #[test]
    fn geopackage_raw_deflate_is_smaller_than_raw_none_for_compressible_data() {
        let mut raster = Raster::new(RasterConfig {
            cols: 256,
            rows: 256,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        });

        for row in 0..raster.rows {
            for col in 0..raster.cols {
                let base = ((row / 16) * 16 + (col / 16)) as f64;
                raster.set(0, row as isize, col as isize, base).unwrap();
            }
        }

        let dir = tempdir().unwrap();
        let none_path = dir.path().join("raw_none.gpkg");
        let deflate_path = dir.path().join("raw_deflate_default.gpkg");

        let mut raster_none = raster.clone();
        raster_none
            .metadata
            .push(("gpkg_tile_encoding".into(), "raw".into()));
        raster_none
            .metadata
            .push(("gpkg_raw_compression".into(), "none".into()));
        write(&raster_none, none_path.to_str().unwrap()).unwrap();

        let mut raster_deflate = raster.clone();
        raster_deflate
            .metadata
            .push(("gpkg_tile_encoding".into(), "raw".into()));
        write(&raster_deflate, deflate_path.to_str().unwrap()).unwrap();

        let size_none = std::fs::metadata(&none_path).unwrap().len();
        let size_deflate = std::fs::metadata(&deflate_path).unwrap().len();
        assert!(
            size_deflate < size_none,
            "expected deflate-compressed raw GeoPackage to be smaller (deflate={size_deflate}, none={size_none})"
        );
    }

    #[test]
    fn geopackage_registers_extensions_and_metadata_contents() {
        let mut raster = Raster::new(RasterConfig {
            cols: 32,
            rows: 24,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: 0.0,
            data_type: DataType::F32,
            ..Default::default()
        });
        raster.metadata.push(("gpkg_tile_encoding".into(), "raw".into()));

        let dir = tempdir().unwrap();
        let path = dir.path().join("extensions.gpkg");
        write(&raster, path.to_str().unwrap()).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        let db = Db::from_bytes(bytes).unwrap();

        let ext_rows = db.select_all("gpkg_extensions").unwrap();
        assert!(ext_rows.len() >= 2);
        assert!(ext_rows.iter().any(|r| r.get(2).and_then(SqlVal::as_str) == Some("org.whiteboxgeo.wbraster.raw_tiles")));
        assert!(ext_rows.iter().any(|r| r.get(2).and_then(SqlVal::as_str) == Some("org.whiteboxgeo.wbraster.metadata")));

        let contents_rows = db.select_all("gpkg_contents").unwrap();
        assert!(contents_rows.iter().any(|r| r.first().and_then(SqlVal::as_str) == Some("wbraster_gpkg_raster_metadata")));
        assert!(contents_rows.iter().any(|r| r.first().and_then(SqlVal::as_str) == Some("wbraster_gpkg_kv_metadata")));
    }

    #[test]
    fn geopackage_honors_custom_dataset_and_base_table_names() {
        let mut raster = Raster::new(RasterConfig {
            cols: 20,
            rows: 10,
            bands: 2,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::I16,
            ..Default::default()
        });
        raster.metadata.push(("gpkg_dataset_name".into(), "demo_ds".into()));
        raster.metadata.push(("gpkg_base_table_name".into(), "demo_tiles".into()));
        for row in 0..raster.rows {
            for col in 0..raster.cols {
                raster.set(0, row as isize, col as isize, (col as i16) as f64).unwrap();
                raster.set(1, row as isize, col as isize, (row as i16) as f64).unwrap();
            }
        }

        let dir = tempdir().unwrap();
        let path = dir.path().join("custom_names.gpkg");
        write(&raster, path.to_str().unwrap()).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        let db = Db::from_bytes(bytes).unwrap();

        let meta_rows = db.select_all("wbraster_gpkg_raster_metadata").unwrap();
        assert_eq!(meta_rows[0][0].as_str(), Some("demo_ds"));
        assert_eq!(meta_rows[0][1].as_str(), Some("demo_tiles"));

        assert!(db.table_meta("demo_tiles_b1").is_some());
        assert!(db.table_meta("demo_tiles_b2").is_some());

        let r2 = read(path.to_str().unwrap()).unwrap();
        assert_eq!(r2.bands, 2);
        assert_eq!(r2.get(0, 7, 3), raster.get(0, 7, 3));
        assert_eq!(r2.get(1, 7, 3), raster.get(1, 7, 3));
    }

    #[test]
    fn geopackage_rejects_invalid_identifier_overrides() {
        assert!(sanitize_sql_identifier("ok_name").is_some());
        assert!(sanitize_sql_identifier("_ok2").is_some());
        assert!(sanitize_sql_identifier("9bad").is_none());
        assert!(sanitize_sql_identifier("bad-name").is_none());
        assert!(sanitize_sql_identifier("bad name").is_none());
    }

    #[test]
    fn choose_dataset_metadata_prefers_named_dataset_when_available() {
        let mut raster = Raster::new(RasterConfig {
            cols: 12,
            rows: 8,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        });
        raster.metadata.push(("gpkg_dataset_name".into(), "alpha".into()));
        raster.metadata.push(("gpkg_base_table_name".into(), "alpha_tiles".into()));

        let dir = tempdir().unwrap();
        let path = dir.path().join("choose_named.gpkg");
        write(&raster, path.to_str().unwrap()).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        let mut db = Db::from_bytes(bytes).unwrap();
        db.insert(
            "wbraster_gpkg_raster_metadata",
            vec![
                SqlVal::Text("beta".into()),
                SqlVal::Text("missing_tiles".into()),
                SqlVal::Int(1),
                SqlVal::Text("float32".into()),
                SqlVal::Real(-9999.0),
                SqlVal::Text("raw".into()),
                SqlVal::Int(0),
                SqlVal::Text("deflate".into()),
            ],
        )
        .unwrap();

        let rows = db.select_all("wbraster_gpkg_raster_metadata").unwrap();
        let candidates: Vec<DatasetMetadata> = rows
            .iter()
            .map(|row| parse_dataset_metadata_row(row))
            .collect();

        let chosen = choose_dataset_metadata(&candidates, Some("alpha"), &db).unwrap();
        assert_eq!(chosen.dataset_name, "alpha");
        assert_eq!(chosen.base_table_name, "alpha_tiles");
    }

    #[test]
    fn choose_dataset_metadata_falls_back_to_valid_dataset() {
        let mut raster = Raster::new(RasterConfig {
            cols: 12,
            rows: 8,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        });
        raster.metadata.push(("gpkg_dataset_name".into(), "alpha".into()));
        raster.metadata.push(("gpkg_base_table_name".into(), "alpha_tiles".into()));

        let dir = tempdir().unwrap();
        let path = dir.path().join("choose_fallback.gpkg");
        write(&raster, path.to_str().unwrap()).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        let mut db = Db::from_bytes(bytes).unwrap();
        db.insert(
            "wbraster_gpkg_raster_metadata",
            vec![
                SqlVal::Text("ghost".into()),
                SqlVal::Text("ghost_tiles".into()),
                SqlVal::Int(2),
                SqlVal::Text("float32".into()),
                SqlVal::Real(-9999.0),
                SqlVal::Text("raw".into()),
                SqlVal::Int(0),
                SqlVal::Text("deflate".into()),
            ],
        )
        .unwrap();

        let rows = db.select_all("wbraster_gpkg_raster_metadata").unwrap();
        let candidates: Vec<DatasetMetadata> = rows
            .iter()
            .map(|row| parse_dataset_metadata_row(row))
            .collect();

        let chosen = choose_dataset_metadata(&candidates, Some("ghost"), &db).unwrap();
        assert_eq!(chosen.dataset_name, "alpha");
        assert_eq!(chosen.base_table_name, "alpha_tiles");
    }

    #[test]
    fn read_errors_on_conflicting_duplicate_dataset_metadata_rows() {
        let mut raster = Raster::new(RasterConfig {
            cols: 10,
            rows: 6,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        });
        raster.metadata.push(("gpkg_dataset_name".into(), "dup_ds".into()));
        raster.metadata.push(("gpkg_base_table_name".into(), "dup_tiles".into()));

        let dir = tempdir().unwrap();
        let path = dir.path().join("dup_conflict.gpkg");
        write(&raster, path.to_str().unwrap()).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        let mut db = Db::from_bytes(bytes).unwrap();
        db.insert(
            "wbraster_gpkg_raster_metadata",
            vec![
                SqlVal::Text("dup_ds".into()),
                SqlVal::Text("dup_tiles_alt".into()),
                SqlVal::Int(2),
                SqlVal::Text("float32".into()),
                SqlVal::Real(-9999.0),
                SqlVal::Text("raw".into()),
                SqlVal::Int(0),
                SqlVal::Text("deflate".into()),
            ],
        )
        .unwrap();
        std::fs::write(&path, db.to_bytes()).unwrap();

        let err = read(path.to_str().unwrap()).unwrap_err();
        assert!(
            format!("{err}").contains("conflicting metadata rows for dataset 'dup_ds'"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn list_datasets_reports_metadata_dataset_names() {
        let mut raster = Raster::new(RasterConfig {
            cols: 8,
            rows: 8,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        });
        raster.metadata.push(("gpkg_dataset_name".into(), "alpha".into()));
        raster.metadata.push(("gpkg_base_table_name".into(), "alpha_tiles".into()));

        let dir = tempdir().unwrap();
        let path = dir.path().join("list_datasets.gpkg");
        write(&raster, path.to_str().unwrap()).unwrap();

        let mut bytes = std::fs::read(&path).unwrap();
        let mut db = Db::from_bytes(std::mem::take(&mut bytes)).unwrap();
        db.insert(
            "wbraster_gpkg_raster_metadata",
            vec![
                SqlVal::Text("beta".into()),
                SqlVal::Text("beta_tiles".into()),
                SqlVal::Int(1),
                SqlVal::Text("float32".into()),
                SqlVal::Real(-9999.0),
                SqlVal::Text("raw".into()),
                SqlVal::Int(0),
                SqlVal::Text("deflate".into()),
            ],
        )
        .unwrap();
        std::fs::write(&path, db.to_bytes()).unwrap();

        let names = list_datasets(path.to_str().unwrap()).unwrap();
        assert_eq!(names.len(), 2);
        assert!(names.iter().any(|n| n == "alpha"));
        assert!(names.iter().any(|n| n == "beta"));
    }

    #[test]
    fn read_dataset_reads_named_dataset() {
        let mut raster = Raster::new(RasterConfig {
            cols: 7,
            rows: 5,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        });
        raster.metadata.push(("gpkg_dataset_name".into(), "alpha".into()));
        raster.metadata.push(("gpkg_base_table_name".into(), "alpha_tiles".into()));
        raster.set(0, 2, 3, 42.5).unwrap();

        let dir = tempdir().unwrap();
        let path = dir.path().join("read_dataset.gpkg");
        write(&raster, path.to_str().unwrap()).unwrap();

        let r = read_dataset(path.to_str().unwrap(), "alpha").unwrap();
        assert_eq!(r.cols, raster.cols);
        assert_eq!(r.rows, raster.rows);
        assert!((r.get(0, 2, 3) - 42.5).abs() < 1e-9);

        let missing = read_dataset(path.to_str().unwrap(), "missing").unwrap_err();
        assert!(format!("{missing}").contains("dataset 'missing' not found"));
    }
}
