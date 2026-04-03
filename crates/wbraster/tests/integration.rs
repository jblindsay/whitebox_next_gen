//! Integration tests: round-trip every format through write→read and verify
//! that all pixel values, extents, and nodata values are preserved.

use wbraster::{
    CogWriteOptions, DataType, GeoTiffCompression, GeoTiffLayout, GeoTiffWriteOptions,
    Jpeg2000Compression, Jpeg2000WriteOptions, Raster, RasterConfig, RasterFormat,
};
use flate2::Compression;
use flate2::write::{GzEncoder, ZlibEncoder};
use serde_json::json;
use std::io::Write;
use std::env::temp_dir;
use std::time::{SystemTime, UNIX_EPOCH};

// ─── helpers ─────────────────────────────────────────────────────────────────

fn tmp(suffix: &str) -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let pid = std::process::id();
    temp_dir()
        .join(format!("wbraster_integ_{pid}_{ts}{suffix}"))
        .to_string_lossy()
        .into_owned()
}

fn env_var_trimmed(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
}

fn env_var_usize(name: &str) -> Option<usize> {
    env_var_trimmed(name).and_then(|v| v.parse::<usize>().ok())
}

fn env_var_f64(name: &str) -> Option<f64> {
    env_var_trimmed(name).and_then(|v| v.parse::<f64>().ok())
}

fn assert_external_fixture_expectations(r: &Raster, prefix: &str) {
    let rows_var = format!("{prefix}_EXPECT_ROWS");
    if let Some(expected_rows) = env_var_usize(&rows_var) {
        assert_eq!(
            r.rows, expected_rows,
            "{rows_var} mismatch: expected {expected_rows}, got {}",
            r.rows
        );
    }

    let cols_var = format!("{prefix}_EXPECT_COLS");
    if let Some(expected_cols) = env_var_usize(&cols_var) {
        assert_eq!(
            r.cols, expected_cols,
            "{cols_var} mismatch: expected {expected_cols}, got {}",
            r.cols
        );
    }

    let nodata_var = format!("{prefix}_EXPECT_NODATA");
    if let Some(expected_nodata) = env_var_f64(&nodata_var) {
        assert!(
            (r.nodata - expected_nodata).abs() < 1e-9,
            "{nodata_var} mismatch: expected {expected_nodata}, got {}",
            r.nodata
        );
    }

    // Optional single-cell assertion for quick parity checks against known values.
    // Format: "row,col,value[,tol]"; default tol = 1e-6.
    let cell_var = format!("{prefix}_EXPECT_CELL");
    if let Some(cell_spec) = env_var_trimmed(&cell_var) {
        let parts: Vec<&str> = cell_spec.split(',').map(|s| s.trim()).collect();
        assert!(
            parts.len() == 3 || parts.len() == 4,
            "{cell_var} must be 'row,col,value[,tol]', got '{cell_spec}'"
        );

        let row = parts[0]
            .parse::<usize>()
            .unwrap_or_else(|_| panic!("{cell_var}: invalid row '{}': {cell_spec}", parts[0]));
        let col = parts[1]
            .parse::<usize>()
            .unwrap_or_else(|_| panic!("{cell_var}: invalid col '{}': {cell_spec}", parts[1]));
        let expected_value = parts[2].parse::<f64>().unwrap_or_else(|_| {
            panic!("{cell_var}: invalid value '{}': {cell_spec}", parts[2])
        });
        let tol = if parts.len() == 4 {
            parts[3].parse::<f64>().unwrap_or_else(|_| {
                panic!("{cell_var}: invalid tol '{}': {cell_spec}", parts[3])
            })
        } else {
            1e-6
        };

        assert!(
            row < r.rows && col < r.cols,
            "{cell_var}: row/col out of range for raster size {}x{}: '{cell_spec}'",
            r.rows,
            r.cols
        );
        let actual = r.get(0, row as isize, col as isize);
        assert!(
            (actual - expected_value).abs() <= tol,
            "{cell_var} mismatch at ({row},{col}): expected {expected_value} +/- {tol}, got {actual}"
        );
    }
}

fn make_test_raster() -> Raster {
    let cfg = RasterConfig {
        cols: 6,
        rows: 4,
        x_min: 100.0,
        y_min: -30.0,
        cell_size: 0.5,
        nodata: -9999.0,
        data_type: DataType::F32,
        ..Default::default()
    };
    let data: Vec<f64> = (0..24)
        .map(|i| if i == 5 { -9999.0 } else { i as f64 * 0.5 })
        .collect();
    Raster::from_data(cfg, data).unwrap()
}

fn assert_raster_equal(a: &Raster, b: &Raster, tol: f64, label: &str) {
    assert_eq!(a.bands, b.bands, "{label}: bands mismatch");
    assert_eq!(a.cols, b.cols, "{label}: cols mismatch");
    assert_eq!(a.rows, b.rows, "{label}: rows mismatch");
    assert!(
        (a.x_min - b.x_min).abs() < tol,
        "{label}: x_min {:.6} vs {:.6}",
        a.x_min, b.x_min
    );
    assert!(
        (a.y_min - b.y_min).abs() < tol,
        "{label}: y_min {:.6} vs {:.6}",
        a.y_min, b.y_min
    );
    assert!(
        (a.cell_size_x - b.cell_size_x).abs() < tol,
        "{label}: cell_size {:.6} vs {:.6}",
        a.cell_size_x, b.cell_size_x
    );
    for band in 0..a.bands {
        for row in 0..a.rows {
            for col in 0..a.cols {
                let va = a
                    .get_raw(band as isize, row as isize, col as isize)
                    .unwrap();
                let vb = b
                    .get_raw(band as isize, row as isize, col as isize)
                    .unwrap();
                let a_nd = a.is_nodata(va);
                let b_nd = b.is_nodata(vb);
                assert_eq!(
                    a_nd, b_nd,
                    "{label}: nodata mismatch at ({band},{row},{col}): {va} vs {vb}"
                );
                if !a_nd {
                    assert!(
                        (va - vb).abs() <= tol,
                        "{label}: value mismatch at ({band},{row},{col}): {va} vs {vb}"
                    );
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn write_python_style_v3_store(
    dir: &str,
    rows: usize,
    cols: usize,
    chunk_rows: usize,
    chunk_cols: usize,
    encoding_name: &str,
    separator: &str,
    compressor: &str,
    endian: &str,
    data: &[f64],
) {
    std::fs::create_dir_all(dir).unwrap();

    let codecs = if compressor == "none" {
        vec![json!({
            "name": "bytes",
            "configuration": { "endian": endian }
        })]
    } else {
        vec![
            json!({
                "name": "bytes",
                "configuration": { "endian": endian }
            }),
            json!({
                "name": compressor,
                "configuration": { "level": 6 }
            }),
        ]
    };

    let zarr_json = json!({
        "zarr_format": 3,
        "node_type": "array",
        "shape": [rows, cols],
        "data_type": {
            "name": "float32"
        },
        "chunk_grid": {
            "name": "regular",
            "configuration": {
                "chunk_shape": [chunk_rows, chunk_cols]
            }
        },
        "chunk_key_encoding": {
            "name": encoding_name,
            "configuration": {
                "separator": separator
            }
        },
        "fill_value": -9999.0,
        "codecs": codecs,
        "attributes": {
            "x_min": 100.0,
            "y_min": -30.0,
            "cell_size_x": 0.5,
            "cell_size_y": 0.5,
            "nodata": -9999.0
        }
    });
    std::fs::write(
        std::path::Path::new(dir).join("zarr.json"),
        serde_json::to_string_pretty(&zarr_json).unwrap(),
    )
    .unwrap();

    let n_chunk_rows = rows.div_ceil(chunk_rows);
    let n_chunk_cols = cols.div_ceil(chunk_cols);
    for cr in 0..n_chunk_rows {
        for cc in 0..n_chunk_cols {
            let this_rows = (rows - cr * chunk_rows).min(chunk_rows);
            let this_cols = (cols - cc * chunk_cols).min(chunk_cols);
            let mut raw = Vec::with_capacity(this_rows * this_cols * 4);
            for rr in 0..this_rows {
                for cc2 in 0..this_cols {
                    let row = cr * chunk_rows + rr;
                    let col = cc * chunk_cols + cc2;
                    let v = data[row * cols + col] as f32;
                    if endian.eq_ignore_ascii_case("big") {
                        raw.extend_from_slice(&v.to_be_bytes());
                    } else {
                        raw.extend_from_slice(&v.to_le_bytes());
                    }
                }
            }

            let payload = compress_fixture(&raw, compressor);
            let key = if encoding_name == "v2" {
                if separator == "/" {
                    format!("{cr}/{cc}")
                } else {
                    format!("{cr}.{cc}")
                }
            } else if separator == "." {
                format!("c.{cr}.{cc}")
            } else {
                format!("c/{cr}/{cc}")
            };
            let path = std::path::Path::new(dir).join(key);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(path, payload).unwrap();
        }
    }
}

fn compress_fixture(raw: &[u8], compressor: &str) -> Vec<u8> {
    match compressor {
        "none" => raw.to_vec(),
        "zlib" => {
            let mut enc = ZlibEncoder::new(Vec::new(), Compression::new(6));
            enc.write_all(raw).unwrap();
            enc.finish().unwrap()
        }
        "gzip" | "gz" => {
            let mut enc = GzEncoder::new(Vec::new(), Compression::new(6));
            enc.write_all(raw).unwrap();
            enc.finish().unwrap()
        }
        "zstd" => {
            #[cfg(feature = "zstd-native")]
            {
                zstd::stream::encode_all(raw, 3).unwrap()
            }
            #[cfg(all(not(feature = "zstd-native"), feature = "zstd-pure-rust-decode"))]
            {
                ruzstd::encoding::compress_to_vec(raw, ruzstd::encoding::CompressionLevel::Fastest)
            }
            #[cfg(all(not(feature = "zstd-native"), not(feature = "zstd-pure-rust-decode")))]
            {
                panic!("zstd fixture compression requires 'zstd-native' or 'zstd-pure-rust-decode'")
            }
        }
        "lz4" => {
            let mut enc = lz4_flex::frame::FrameEncoder::new(Vec::new());
            enc.write_all(raw).unwrap();
            enc.finish().unwrap()
        }
        other => panic!("unsupported test compressor: {other}"),
    }
}

// ─── Esri ASCII ───────────────────────────────────────────────────────────────

#[test]
fn roundtrip_esri_ascii() {
    let path = tmp(".asc");
    let r = make_test_raster();
    r.write(&path, RasterFormat::EsriAscii).unwrap();
    let r2 = Raster::read(&path).unwrap();
    assert_raster_equal(&r, &r2, 1e-5, "EsriAscii");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn esri_ascii_large() {
    // Verify performance: write/read a moderately large raster
    let path = tmp("_large.asc");
    let cfg = RasterConfig {
        cols: 500,
        rows: 500,
        cell_size: 1.0,
        nodata: -9999.0,
        ..Default::default()
    };
    let data: Vec<f64> = (0..250_000).map(|i| (i % 1000) as f64).collect();
    let r = Raster::from_data(cfg, data).unwrap();
    r.write(&path, RasterFormat::EsriAscii).unwrap();
    let r2 = Raster::read(&path).unwrap();
    assert_eq!(r2.cols, 500);
    assert_eq!(r2.rows, 500);
    let _ = std::fs::remove_file(&path);
}

// ─── GRASS ASCII ─────────────────────────────────────────────────────────────

#[test]
fn roundtrip_grass_ascii() {
    let path = tmp(".txt");
    let r = make_test_raster();
    r.write(&path, RasterFormat::GrassAscii).unwrap();
    let r2 = Raster::read(&path).unwrap();
    assert_raster_equal(&r, &r2, 1e-4, "GrassAscii");
    let _ = std::fs::remove_file(&path);
}

// ─── Surfer GRD ──────────────────────────────────────────────────────────────

#[test]
fn roundtrip_surfer_grd() {
    let path = tmp(".grd");
    let r = make_test_raster();
    r.write(&path, RasterFormat::SurferGrd).unwrap();
    let r2 = Raster::read(&path).unwrap();
    assert_raster_equal(&r, &r2, 1e-4, "SurferGrd");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn roundtrip_surfer_grd_dsrb_via_metadata() {
    let path = tmp("_dsrb.grd");
    let mut r = make_test_raster();
    r.metadata
        .push(("surfer_format".to_string(), "dsrb".to_string()));
    r.write(&path, RasterFormat::SurferGrd).unwrap();

    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(
        i32::from_le_bytes(bytes[0..4].try_into().unwrap()),
        0x4252_5344,
        "Surfer DSRB magic mismatch"
    );

    let r2 = Raster::read(&path).unwrap();
    assert_raster_equal(&r, &r2, 1e-4, "Surfer DSRB");
    let _ = std::fs::remove_file(&path);
}

// ─── PCRaster ────────────────────────────────────────────────────────────────

#[test]
fn roundtrip_pcraster() {
    let path = tmp(".map");
    let r = make_test_raster();
    r.write(&path, RasterFormat::Pcraster).unwrap();
    let r2 = Raster::read(&path).unwrap();
    assert_raster_equal(&r, &r2, 1e-4, "Pcraster");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn roundtrip_pcraster_ordinal_int4() {
    let path = tmp("_ordinal.map");
    let cfg = RasterConfig {
        cols: 6,
        rows: 4,
        x_min: 100.0,
        y_min: -30.0,
        cell_size: 0.5,
        nodata: i32::MIN as f64,
        data_type: DataType::I32,
        metadata: vec![
            ("pcraster_valuescale".into(), "ordinal".into()),
            ("pcraster_cellrepr".into(), "int4".into()),
        ],
        ..Default::default()
    };
    let data: Vec<f64> = (0..24)
        .map(|i| if i == 5 { i32::MIN as f64 } else { (i % 10) as f64 })
        .collect();
    let r = Raster::from_data(cfg, data).unwrap();

    r.write(&path, RasterFormat::Pcraster).unwrap();
    let r2 = Raster::read(&path).unwrap();
    assert_raster_equal(&r, &r2, 1e-9, "Pcraster ordinal int4");
    assert_eq!(r2.data_type, DataType::I32);
    let _ = std::fs::remove_file(&path);
}

// ─── GeoPackage Raster ───────────────────────────────────────────────────────

#[test]
fn roundtrip_geopackage_multiband_native_default_raw() {
    let path = tmp("_multiband_native.gpkg");
    let mut r = Raster::new(RasterConfig {
        cols: 67,
        rows: 59,
        bands: 2,
        x_min: 10.0,
        y_min: -4.0,
        cell_size: 2.0,
        nodata: -32768.0,
        data_type: DataType::I16,
        ..Default::default()
    });

    for row in 0..r.rows {
        for col in 0..r.cols {
            let b0 = (row as i32 - col as i32) as f64;
            let b1 = (row as i32 * 3 + col as i32) as f64;
            r.set(0, row as isize, col as isize, b0).unwrap();
            r.set(1, row as isize, col as isize, b1).unwrap();
        }
    }

    r.write(&path, RasterFormat::GeoPackage).unwrap();
    let r2 = Raster::read(&path).unwrap();

    assert_eq!(r2.cols, r.cols, "GeoPackage native multiband: cols mismatch");
    assert_eq!(r2.rows, r.rows, "GeoPackage native multiband: rows mismatch");
    assert_eq!(r2.bands, r.bands, "GeoPackage native multiband: bands mismatch");
    assert_eq!(r2.data_type, DataType::I16, "GeoPackage native multiband: data_type mismatch");

    for &(col, row) in &[(0isize, 0isize), (8, 5), (66, 58)] {
        assert_eq!(
            r2.get_raw(0, row, col),
            r.get_raw(0, row, col),
            "GeoPackage native multiband: band 0 mismatch at ({col},{row})"
        );
        assert_eq!(
            r2.get_raw(1, row, col),
            r.get_raw(1, row, col),
            "GeoPackage native multiband: band 1 mismatch at ({col},{row})"
        );
    }

    let _ = std::fs::remove_file(&path);
}

#[test]
fn roundtrip_geopackage_with_pyramids_png() {
    let path = tmp("_pyramid_png.gpkg");
    let mut r = make_test_raster();
    r.metadata.push(("gpkg_max_zoom".into(), "2".into()));
    r.metadata.push(("gpkg_tile_format".into(), "png".into()));

    r.write(&path, RasterFormat::GeoPackage).unwrap();
    let r2 = Raster::read(&path).unwrap();

    assert_eq!(r.cols, r2.cols, "GeoPackage pyramid png: cols mismatch");
    assert_eq!(r.rows, r2.rows, "GeoPackage pyramid png: rows mismatch");
    assert_eq!(r.bands, r2.bands, "GeoPackage pyramid png: bands mismatch");
    assert!((r.x_min - r2.x_min).abs() < 1e-9, "GeoPackage pyramid png: x_min mismatch");
    assert!((r.y_min - r2.y_min).abs() < 1e-9, "GeoPackage pyramid png: y_min mismatch");

    // Verify representative non-nodata cells match exactly for PNG tiles.
    let sample_cells = [(1isize, 0isize), (2, 1), (4, 3)];
    for (col, row) in sample_cells {
        let a = r.get_raw(0, row, col).unwrap();
        let b = r2.get_raw(0, row, col).unwrap();
        let a_q = a.round().clamp(0.0, 255.0);
        assert!(
            (a_q - b).abs() <= 1e-9,
            "GeoPackage pyramid png: value mismatch at ({col},{row}): quantized {a_q} vs {b}"
        );
    }

    let _ = std::fs::remove_file(&path);
}

#[test]
fn roundtrip_geopackage_with_pyramids_jpeg() {
    let path = tmp("_pyramid_jpeg.gpkg");
    let mut r = make_test_raster();
    r.metadata.push(("gpkg_max_zoom".into(), "2".into()));
    r.metadata.push(("gpkg_tile_format".into(), "jpeg".into()));
    r.metadata.push(("gpkg_jpeg_quality".into(), "85".into()));

    r.write(&path, RasterFormat::GeoPackage).unwrap();
    let r2 = Raster::read(&path).unwrap();

    assert_eq!(r.cols, r2.cols, "GeoPackage pyramid jpeg: cols mismatch");
    assert_eq!(r.rows, r2.rows, "GeoPackage pyramid jpeg: rows mismatch");
    assert_eq!(r.bands, r2.bands, "GeoPackage pyramid jpeg: bands mismatch");
    assert!((r.x_min - r2.x_min).abs() < 1e-9, "GeoPackage pyramid jpeg: x_min mismatch");
    assert!((r.y_min - r2.y_min).abs() < 1e-9, "GeoPackage pyramid jpeg: y_min mismatch");

    for row in 0..r.rows {
        for col in 0..r.cols {
            let a = r.get_raw(0, row as isize, col as isize).unwrap();
            let b = r2.get_raw(0, row as isize, col as isize).unwrap();
            if !r.is_nodata(a) {
                assert!(
                    (a - b).abs() <= 15.0,
                    "GeoPackage pyramid jpeg: value mismatch at ({col},{row}): {a} vs {b}"
                );
            }
        }
    }

    let _ = std::fs::remove_file(&path);
}

// ─── Esri Binary ─────────────────────────────────────────────────────────────

#[test]
fn roundtrip_esri_binary() {
    let dir = tmp("_esribinary");
    let r = make_test_raster();
    r.write(&dir, RasterFormat::EsriBinary).unwrap();
    let r2 = Raster::read(&dir).unwrap();
    assert_raster_equal(&r, &r2, 1e-4, "EsriBinary");  // f32 precision
    let _ = std::fs::remove_dir_all(&dir);
}

// ─── SAGA ─────────────────────────────────────────────────────────────────────

#[test]
fn roundtrip_saga() {
    let path = tmp(".sgrd");
    let r = make_test_raster();
    r.write(&path, RasterFormat::Saga).unwrap();
    let r2 = Raster::read(&path).unwrap();
    assert_raster_equal(&r, &r2, 1e-4, "SAGA");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(wbraster::io_utils::with_extension(&path, "sdat"));
}

// ─── Idrisi ───────────────────────────────────────────────────────────────────

#[test]
fn roundtrip_idrisi() {
    let path = tmp(".rdc");
    let r = make_test_raster();
    r.write(&path, RasterFormat::Idrisi).unwrap();
    let r2 = Raster::read(&path).unwrap();
    assert_raster_equal(&r, &r2, 1e-4, "Idrisi");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(wbraster::io_utils::with_extension(&path, "rst"));
}

// ─── ER Mapper ────────────────────────────────────────────────────────────────

#[test]
fn roundtrip_er_mapper() {
    let path = tmp(".ers");
    let r = make_test_raster();
    r.write(&path, RasterFormat::ErMapper).unwrap();
    let r2 = Raster::read(&path).unwrap();
    assert_raster_equal(&r, &r2, 1e-4, "ErMapper");
    let data_path = path.trim_end_matches(".ers").to_string();
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&data_path);
}

// ─── ENVI ─────────────────────────────────────────────────────────────────────

#[test]
fn roundtrip_envi() {
    let path = tmp(".hdr");
    let r = make_test_raster();
    r.write(&path, RasterFormat::Envi).unwrap();
    let r2 = Raster::read(&path).unwrap();
    assert_raster_equal(&r, &r2, 1e-4, "ENVI");
    let img = wbraster::io_utils::with_extension(&path, "img");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&img);
}

#[test]
fn roundtrip_envi_multiband_bip() {
    let path = tmp("_mb.hdr");
    let cfg = RasterConfig {
        cols: 6,
        rows: 4,
        bands: 3,
        x_min: 100.0,
        y_min: -30.0,
        cell_size: 0.5,
        nodata: -9999.0,
        data_type: DataType::F32,
        ..Default::default()
    };
    let data: Vec<f64> = (0..(cfg.cols * cfg.rows * cfg.bands))
        .map(|i| if i == 5 { -9999.0 } else { i as f64 * 0.5 })
        .collect();
    let mut r = Raster::from_data(cfg, data).unwrap();
    r.metadata.push(("envi_interleave".into(), "bip".into()));

    r.write(&path, RasterFormat::Envi).unwrap();
    let r2 = Raster::read(&path).unwrap();
    assert_raster_equal(&r, &r2, 1e-4, "ENVI multiband BIP");
    let img = wbraster::io_utils::with_extension(&path, "img");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&img);
}

// ─── GeoTIFF / BigTIFF / COG ────────────────────────────────────────────────

#[test]
fn roundtrip_geotiff() {
    let path = tmp(".tif");
    let r = make_test_raster();

    r.write(&path, RasterFormat::GeoTiff).unwrap();
    let r2 = Raster::read(&path).unwrap();
    assert_raster_equal(&r, &r2, 1e-4, "GeoTIFF");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn roundtrip_geotiff_cog_legacy_metadata_ignored() {
    let path = tmp("_cog.tif");
    let cfg = RasterConfig {
        cols: 7,
        rows: 5,
        bands: 2,
        x_min: 100.0,
        y_min: -30.0,
        cell_size: 0.5,
        nodata: -9999.0,
        data_type: DataType::F32,
        ..Default::default()
    };
    let data: Vec<f64> = (0..(cfg.cols * cfg.rows * cfg.bands))
        .map(|i| if i == 9 { -9999.0 } else { i as f64 * 0.25 })
        .collect();
    let mut r = Raster::from_data(cfg, data).unwrap();

    // Legacy metadata controls are no longer consumed by writer configuration.
    r.metadata.push(("geotiff_cog".into(), "true".into()));
    r.metadata
        .push(("geotiff_compression".into(), "deflate".into()));
    r.metadata
        .push(("geotiff_tile_size".into(), "256".into()));

    r.write(&path, RasterFormat::GeoTiff).unwrap();
    let r2 = Raster::read(&path).unwrap();
    assert_raster_equal(&r, &r2, 1e-4, "GeoTIFF legacy metadata ignored");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn roundtrip_geotiff_typed_options() {
    let path = tmp("_typed.tif");
    let r = make_test_raster();
    let opts = GeoTiffWriteOptions {
        compression: Some(GeoTiffCompression::Deflate),
        bigtiff: Some(false),
        layout: Some(GeoTiffLayout::Standard),
    };

    r.write_geotiff_with_options(&path, &opts).unwrap();
    let r2 = Raster::read(&path).unwrap();
    assert_raster_equal(&r, &r2, 1e-4, "GeoTIFF typed options");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn roundtrip_geotiff_typed_cog() {
    let path = tmp("_typed_cog.tif");
    let cfg = RasterConfig {
        cols: 7,
        rows: 5,
        bands: 2,
        x_min: 100.0,
        y_min: -30.0,
        cell_size: 0.5,
        nodata: -9999.0,
        data_type: DataType::F32,
        ..Default::default()
    };
    let data: Vec<f64> = (0..(cfg.cols * cfg.rows * cfg.bands))
        .map(|i| if i == 9 { -9999.0 } else { i as f64 * 0.25 })
        .collect();
    let r = Raster::from_data(cfg, data).unwrap();

    let opts = GeoTiffWriteOptions {
        compression: Some(GeoTiffCompression::Deflate),
        bigtiff: Some(false),
        layout: Some(GeoTiffLayout::Cog { tile_size: 256 }),
    };

    r.write_geotiff_with_options(&path, &opts).unwrap();
    let r2 = Raster::read(&path).unwrap();
    assert_raster_equal(&r, &r2, 1e-4, "GeoTIFF typed COG");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn roundtrip_geotiff_write_cog_convenience() {
    let path = tmp("_write_cog.tif");
    let r = make_test_raster();

    r.write_cog(&path).unwrap();
    let r2 = Raster::read(&path).unwrap();
    assert_raster_equal(&r, &r2, 1e-4, "GeoTIFF write_cog convenience");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn roundtrip_geotiff_write_cog_with_tile_size_convenience() {
    let path = tmp("_write_cog_tile.tif");
    let r = make_test_raster();

    r.write_cog_with_tile_size(&path, 256).unwrap();
    let r2 = Raster::read(&path).unwrap();
    assert_raster_equal(
        &r,
        &r2,
        1e-4,
        "GeoTIFF write_cog_with_tile_size convenience",
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn roundtrip_geotiff_write_cog_with_options_convenience() {
    let path = tmp("_write_cog_opts.tif");
    let r = make_test_raster();
    let opts = CogWriteOptions {
        compression: Some(GeoTiffCompression::Deflate),
        bigtiff: Some(false),
        tile_size: Some(256),
    };

    r.write_cog_with_options(&path, &opts).unwrap();
    let r2 = Raster::read(&path).unwrap();
    assert_raster_equal(
        &r,
        &r2,
        1e-4,
        "GeoTIFF write_cog_with_options convenience",
    );
    let _ = std::fs::remove_file(&path);
}

// ─── JPEG2000 / GeoJP2 ─────────────────────────────────────────────────────

#[test]
fn write_jpeg2000_dispatch() {
    let path = tmp(".jp2");
    let r = make_test_raster();

    r.write(&path, RasterFormat::Jpeg2000).unwrap();
    assert!(std::path::Path::new(&path).exists());
    assert_eq!(RasterFormat::detect(&path).unwrap(), RasterFormat::Jpeg2000);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn write_jpeg2000_typed_lossless() {
    let path = tmp("_jp2_typed_lossless.jp2");
    let r = make_test_raster();
    let opts = Jpeg2000WriteOptions {
        compression: Some(Jpeg2000Compression::Lossless),
        decomp_levels: Some(5),
        color_space: None,
    };

    r.write_jpeg2000_with_options(&path, &opts).unwrap();
    assert!(std::path::Path::new(&path).exists());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn write_jpeg2000_typed_lossy() {
    let path = tmp("_jp2_typed_lossy.jp2");
    let r = make_test_raster();
    let opts = Jpeg2000WriteOptions {
        compression: Some(Jpeg2000Compression::Lossy { quality_db: 45.0 }),
        decomp_levels: Some(5),
        color_space: None,
    };

    r.write_jpeg2000_with_options(&path, &opts).unwrap();
    assert!(std::path::Path::new(&path).exists());
    let _ = std::fs::remove_file(&path);
}

// ─── Zarr ─────────────────────────────────────────────────────────────────────

#[test]
fn roundtrip_zarr() {
    let dir = tmp(".zarr");
    let r = make_test_raster();
    r.write(&dir, RasterFormat::Zarr).unwrap();
    let r2 = Raster::read(&dir).unwrap();
    assert_raster_equal(&r, &r2, 1e-5, "Zarr");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn roundtrip_zarr_v3() {
    let dir = tmp("_v3.zarr");
    let mut r = make_test_raster();
    r.metadata.push(("zarr_version".into(), "3".into()));
    r.metadata
        .push(("zarr_chunk_key_encoding".into(), "default".into()));
    r.metadata
        .push(("zarr_dimension_separator".into(), "/".into()));

    r.write(&dir, RasterFormat::Zarr).unwrap();
    let r2 = Raster::read(&dir).unwrap();

    assert_raster_equal(&r, &r2, 1e-5, "Zarr v3");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn roundtrip_zarr_v3_multichunk() {
    let dir = tmp("_v3_multichunk.zarr");
    let cfg = RasterConfig {
        cols: 13,
        rows: 11,
        x_min: 100.0,
        y_min: -30.0,
        cell_size: 0.5,
        nodata: -9999.0,
        data_type: DataType::F32,
        ..Default::default()
    };
    let mut data: Vec<f64> = (0..(13 * 11))
        .map(|i| if i == 17 { -9999.0 } else { i as f64 * 0.25 })
        .collect();
    data[142] = 1234.5;

    let mut r = Raster::from_data(cfg, data).unwrap();
    r.metadata.push(("zarr_version".into(), "3".into()));
    r.metadata
        .push(("zarr_chunk_key_encoding".into(), "default".into()));
    r.metadata
        .push(("zarr_dimension_separator".into(), "/".into()));
    r.metadata.push(("zarr_chunk_rows".into(), "4".into()));
    r.metadata.push(("zarr_chunk_cols".into(), "3".into()));

    r.write(&dir, RasterFormat::Zarr).unwrap();
    assert!(std::path::Path::new(&dir).join("c").join("0").join("0").exists());
    assert!(std::path::Path::new(&dir).join("c").join("2").join("4").exists());

    let r2 = Raster::read(&dir).unwrap();
    assert_raster_equal(&r, &r2, 1e-5, "Zarr v3 multichunk");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn roundtrip_zarr_v3_multiband() {
    let dir = tmp("_v3_multiband.zarr");
    let cfg = RasterConfig {
        cols: 7,
        rows: 5,
        bands: 2,
        x_min: 100.0,
        y_min: -30.0,
        cell_size: 0.5,
        nodata: -9999.0,
        data_type: DataType::F32,
        ..Default::default()
    };
    let data: Vec<f64> = (0..(cfg.cols * cfg.rows * cfg.bands))
        .map(|i| if i == 3 { -9999.0 } else { i as f64 * 0.25 })
        .collect();

    let mut r = Raster::from_data(cfg, data).unwrap();
    r.metadata.push(("zarr_version".into(), "3".into()));
    r.metadata
        .push(("zarr_chunk_key_encoding".into(), "default".into()));
    r.metadata
        .push(("zarr_dimension_separator".into(), "/".into()));
    r.metadata.push(("zarr_chunk_bands".into(), "1".into()));
    r.metadata.push(("zarr_chunk_rows".into(), "3".into()));
    r.metadata.push(("zarr_chunk_cols".into(), "4".into()));

    r.write(&dir, RasterFormat::Zarr).unwrap();
    let r2 = Raster::read(&dir).unwrap();
    assert_raster_equal(&r, &r2, 1e-5, "Zarr v3 multiband");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_default_zlib() {
    let dir = tmp("_py_v3_default_zlib.zarr");
    let rows = 9;
    let cols = 8;
    let data: Vec<f64> = (0..rows * cols)
        .map(|i| if i == 13 { -9999.0 } else { i as f64 * 0.2 })
        .collect();
    write_python_style_v3_store(
        &dir,
        rows,
        cols,
        4,
        3,
        "default",
        "/",
        "zlib",
        "little",
        &data,
    );

    let r = Raster::read(&dir).unwrap();
    let expected = Raster::from_data(
        RasterConfig {
            cols,
            rows,
            x_min: 100.0,
            y_min: -30.0,
            cell_size: 0.5,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        },
        data,
    )
    .unwrap();
    assert_raster_equal(&expected, &r, 1e-5, "Python-style Zarr v3 default+zlib");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_v2_zstd() {
    let dir = tmp("_py_v3_v2_zstd.zarr");
    let rows = 7;
    let cols = 10;
    let data: Vec<f64> = (0..rows * cols)
        .map(|i| if i == 0 { -9999.0 } else { (i as f64).sin() * 10.0 })
        .collect();
    write_python_style_v3_store(
        &dir,
        rows,
        cols,
        3,
        4,
        "v2",
        ".",
        "zstd",
        "little",
        &data,
    );

    let r = Raster::read(&dir).unwrap();
    let expected = Raster::from_data(
        RasterConfig {
            cols,
            rows,
            x_min: 100.0,
            y_min: -30.0,
            cell_size: 0.5,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        },
        data,
    )
    .unwrap();
    assert_raster_equal(&expected, &r, 1e-5, "Python-style Zarr v3 v2+zstd");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_default_gzip() {
    let dir = tmp("_py_v3_default_gzip.zarr");
    let rows = 8;
    let cols = 9;
    let data: Vec<f64> = (0..rows * cols)
        .map(|i| if i == 21 { -9999.0 } else { (i as f64) * 0.125 })
        .collect();
    write_python_style_v3_store(
        &dir,
        rows,
        cols,
        3,
        4,
        "default",
        "/",
        "gzip",
        "little",
        &data,
    );

    let r = Raster::read(&dir).unwrap();
    let expected = Raster::from_data(
        RasterConfig {
            cols,
            rows,
            x_min: 100.0,
            y_min: -30.0,
            cell_size: 0.5,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        },
        data,
    )
    .unwrap();
    assert_raster_equal(&expected, &r, 1e-5, "Python-style Zarr v3 default+gzip");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_v2_lz4() {
    let dir = tmp("_py_v3_v2_lz4.zarr");
    let rows = 10;
    let cols = 7;
    let data: Vec<f64> = (0..rows * cols)
        .map(|i| if i == 33 { -9999.0 } else { ((i as f64) - 10.0) * 0.75 })
        .collect();
    write_python_style_v3_store(
        &dir,
        rows,
        cols,
        4,
        3,
        "v2",
        ".",
        "lz4",
        "little",
        &data,
    );

    let r = Raster::read(&dir).unwrap();
    let expected = Raster::from_data(
        RasterConfig {
            cols,
            rows,
            x_min: 100.0,
            y_min: -30.0,
            cell_size: 0.5,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        },
        data,
    )
    .unwrap();
    assert_raster_equal(&expected, &r, 1e-5, "Python-style Zarr v3 v2+lz4");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_big_endian_bytes() {
    let dir = tmp("_py_v3_big_endian.zarr");
    let rows = 6;
    let cols = 11;
    let data: Vec<f64> = (0..rows * cols)
        .map(|i| if i == 12 { -9999.0 } else { (i as f64) * 1.5 - 2.0 })
        .collect();
    write_python_style_v3_store(
        &dir,
        rows,
        cols,
        2,
        5,
        "default",
        "/",
        "none",
        "big",
        &data,
    );

    let r = Raster::read(&dir).unwrap();
    let expected = Raster::from_data(
        RasterConfig {
            cols,
            rows,
            x_min: 100.0,
            y_min: -30.0,
            cell_size: 0.5,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        },
        data,
    )
    .unwrap();
    assert_raster_equal(&expected, &r, 1e-5, "Python-style Zarr v3 big-endian bytes");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_v2_slash_big_endian_gzip() {
    let dir = tmp("_py_v3_v2_slash_big_gzip.zarr");
    let rows = 9;
    let cols = 9;
    let data: Vec<f64> = (0..rows * cols)
        .map(|i| if i == 40 { -9999.0 } else { (i as f64) * 0.33 - 5.0 })
        .collect();
    write_python_style_v3_store(
        &dir,
        rows,
        cols,
        4,
        4,
        "v2",
        "/",
        "gzip",
        "big",
        &data,
    );

    let r = Raster::read(&dir).unwrap();
    let expected = Raster::from_data(
        RasterConfig {
            cols,
            rows,
            x_min: 100.0,
            y_min: -30.0,
            cell_size: 0.5,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        },
        data,
    )
    .unwrap();
    assert_raster_equal(
        &expected,
        &r,
        1e-5,
        "Python-style Zarr v3 v2+slash+big-endian+gzip",
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_transform_only_georef_attrs() {
    let dir = tmp("_py_v3_transform_only_attrs.zarr");
    let rows = 6;
    let cols = 7;
    let data: Vec<f64> = (0..rows * cols)
        .map(|i| if i == 9 { -9999.0 } else { (i as f64) * 0.4 })
        .collect();
    write_python_style_v3_store(
        &dir,
        rows,
        cols,
        3,
        4,
        "default",
        "/",
        "zlib",
        "little",
        &data,
    );

    let zarr_json_path = std::path::Path::new(&dir).join("zarr.json");
    let mut zarr_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&zarr_json_path).unwrap()).unwrap();
    let attrs = zarr_json
        .get_mut("attributes")
        .and_then(serde_json::Value::as_object_mut)
        .expect("zarr.json attributes object missing");
    attrs.remove("x_min");
    attrs.remove("y_min");
    attrs.remove("cell_size_x");
    attrs.remove("cell_size_y");
    attrs.insert(
        "transform".into(),
        json!([
            100.0,
            0.5,
            0.0,
            -27.0,
            0.0,
            -0.5
        ]),
    );
    std::fs::write(
        &zarr_json_path,
        serde_json::to_string_pretty(&zarr_json).unwrap(),
    )
    .unwrap();

    let r = Raster::read(&dir).unwrap();
    let expected = Raster::from_data(
        RasterConfig {
            cols,
            rows,
            x_min: 100.0,
            y_min: -30.0,
            cell_size: 0.5,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        },
        data,
    )
    .unwrap();

    assert_raster_equal(&expected, &r, 1e-5, "Python-style Zarr v3 transform-only attrs");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_crs_alias_attrs() {
    let dir = tmp("_py_v3_crs_alias_attrs.zarr");
    let rows = 5;
    let cols = 6;
    let data: Vec<f64> = (0..rows * cols)
        .map(|i| if i == 11 { -9999.0 } else { (i as f64) * 0.3 })
        .collect();
    write_python_style_v3_store(
        &dir,
        rows,
        cols,
        3,
        3,
        "default",
        "/",
        "zlib",
        "little",
        &data,
    );

    let zarr_json_path = std::path::Path::new(&dir).join("zarr.json");
    let mut zarr_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&zarr_json_path).unwrap()).unwrap();
    let attrs = zarr_json
        .get_mut("attributes")
        .and_then(serde_json::Value::as_object_mut)
        .expect("zarr.json attributes object missing");
    attrs.remove("crs_epsg");
    attrs.remove("crs_wkt");
    attrs.remove("crs_proj4");
    attrs.insert("epsg".into(), json!(32617));
    attrs.insert(
        "spatial_ref".into(),
        json!("PROJCS[\"WGS 84 / UTM zone 17N\",GEOGCS[\"WGS 84\"]]"),
    );
    attrs.insert(
        "proj4".into(),
        json!("+proj=utm +zone=17 +datum=WGS84 +units=m +no_defs"),
    );
    std::fs::write(
        &zarr_json_path,
        serde_json::to_string_pretty(&zarr_json).unwrap(),
    )
    .unwrap();

    let r = Raster::read(&dir).unwrap();
    assert_eq!(r.crs.epsg, Some(32617));
    assert!(r.crs.wkt.as_deref().unwrap_or_default().contains("UTM zone 17N"));
    assert!(r
        .crs
        .proj4
        .as_deref()
        .unwrap_or_default()
        .contains("+proj=utm"));

    let expected = Raster::from_data(
        RasterConfig {
            cols,
            rows,
            x_min: 100.0,
            y_min: -30.0,
            cell_size: 0.5,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        },
        data,
    )
    .unwrap();
    assert_raster_equal(&expected, &r, 1e-5, "Python-style Zarr v3 CRS alias attrs");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_crs_from_epsg_string_attr() {
    let dir = tmp("_py_v3_crs_string_attr.zarr");
    let rows = 4;
    let cols = 5;
    let data: Vec<f64> = (0..rows * cols)
        .map(|i| if i == 6 { -9999.0 } else { (i as f64) * 0.6 })
        .collect();
    write_python_style_v3_store(
        &dir,
        rows,
        cols,
        2,
        3,
        "default",
        "/",
        "zlib",
        "little",
        &data,
    );

    let zarr_json_path = std::path::Path::new(&dir).join("zarr.json");
    let mut zarr_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&zarr_json_path).unwrap()).unwrap();
    let attrs = zarr_json
        .get_mut("attributes")
        .and_then(serde_json::Value::as_object_mut)
        .expect("zarr.json attributes object missing");
    attrs.remove("crs_epsg");
    attrs.remove("epsg");
    attrs.insert("crs".into(), json!("EPSG:3857"));
    std::fs::write(
        &zarr_json_path,
        serde_json::to_string_pretty(&zarr_json).unwrap(),
    )
    .unwrap();

    let r = Raster::read(&dir).unwrap();
    assert_eq!(r.crs.epsg, Some(3857));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_crs_from_epsg_object_attr() {
    let dir = tmp("_py_v3_crs_object_attr.zarr");
    let rows = 4;
    let cols = 5;
    let data: Vec<f64> = (0..rows * cols)
        .map(|i| if i == 6 { -9999.0 } else { (i as f64) * 0.6 })
        .collect();
    write_python_style_v3_store(
        &dir,
        rows,
        cols,
        2,
        3,
        "default",
        "/",
        "zlib",
        "little",
        &data,
    );

    let zarr_json_path = std::path::Path::new(&dir).join("zarr.json");
    let mut zarr_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&zarr_json_path).unwrap()).unwrap();
    let attrs = zarr_json
        .get_mut("attributes")
        .and_then(serde_json::Value::as_object_mut)
        .expect("zarr.json attributes object missing");
    attrs.remove("crs_epsg");
    attrs.remove("epsg");
    attrs.insert(
        "crs".into(),
        json!({
            "type": "name",
            "properties": {
                "name": "EPSG:32618"
            }
        }),
    );
    std::fs::write(
        &zarr_json_path,
        serde_json::to_string_pretty(&zarr_json).unwrap(),
    )
    .unwrap();

    let r = Raster::read(&dir).unwrap();
    assert_eq!(r.crs.epsg, Some(32618));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_crs_from_authority_code_object_attr() {
    let dir = tmp("_py_v3_crs_authority_code_attr.zarr");
    let rows = 4;
    let cols = 5;
    let data: Vec<f64> = (0..rows * cols)
        .map(|i| if i == 6 { -9999.0 } else { (i as f64) * 0.6 })
        .collect();
    write_python_style_v3_store(
        &dir,
        rows,
        cols,
        2,
        3,
        "default",
        "/",
        "zlib",
        "little",
        &data,
    );

    let zarr_json_path = std::path::Path::new(&dir).join("zarr.json");
    let mut zarr_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&zarr_json_path).unwrap()).unwrap();
    let attrs = zarr_json
        .get_mut("attributes")
        .and_then(serde_json::Value::as_object_mut)
        .expect("zarr.json attributes object missing");
    attrs.remove("crs_epsg");
    attrs.remove("epsg");
    attrs.insert(
        "crs".into(),
        json!({
            "id": {
                "authority": "EPSG",
                "code": "3035"
            }
        }),
    );
    std::fs::write(
        &zarr_json_path,
        serde_json::to_string_pretty(&zarr_json).unwrap(),
    )
    .unwrap();

    let r = Raster::read(&dir).unwrap();
    assert_eq!(r.crs.epsg, Some(3035));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_crs_from_ogc_urn_attr() {
    let dir = tmp("_py_v3_crs_ogc_urn_attr.zarr");
    let rows = 4;
    let cols = 5;
    let data: Vec<f64> = (0..rows * cols)
        .map(|i| if i == 6 { -9999.0 } else { (i as f64) * 0.6 })
        .collect();
    write_python_style_v3_store(
        &dir,
        rows,
        cols,
        2,
        3,
        "default",
        "/",
        "zlib",
        "little",
        &data,
    );

    let zarr_json_path = std::path::Path::new(&dir).join("zarr.json");
    let mut zarr_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&zarr_json_path).unwrap()).unwrap();
    let attrs = zarr_json
        .get_mut("attributes")
        .and_then(serde_json::Value::as_object_mut)
        .expect("zarr.json attributes object missing");
    attrs.remove("crs_epsg");
    attrs.remove("epsg");
    attrs.insert("crs".into(), json!("urn:ogc:def:crs:EPSG::26917"));
    std::fs::write(
        &zarr_json_path,
        serde_json::to_string_pretty(&zarr_json).unwrap(),
    )
    .unwrap();

    let r = Raster::read(&dir).unwrap();
    assert_eq!(r.crs.epsg, Some(26917));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_crs_from_ogc_url_attr() {
    let dir = tmp("_py_v3_crs_ogc_url_attr.zarr");
    let rows = 4;
    let cols = 5;
    let data: Vec<f64> = (0..rows * cols)
        .map(|i| if i == 6 { -9999.0 } else { (i as f64) * 0.6 })
        .collect();
    write_python_style_v3_store(
        &dir,
        rows,
        cols,
        2,
        3,
        "default",
        "/",
        "zlib",
        "little",
        &data,
    );

    let zarr_json_path = std::path::Path::new(&dir).join("zarr.json");
    let mut zarr_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&zarr_json_path).unwrap()).unwrap();
    let attrs = zarr_json
        .get_mut("attributes")
        .and_then(serde_json::Value::as_object_mut)
        .expect("zarr.json attributes object missing");
    attrs.remove("crs_epsg");
    attrs.remove("epsg");
    attrs.insert("crs".into(), json!("https://www.opengis.net/def/crs/EPSG/0/3395"));
    std::fs::write(
        &zarr_json_path,
        serde_json::to_string_pretty(&zarr_json).unwrap(),
    )
    .unwrap();

    let r = Raster::read(&dir).unwrap();
    assert_eq!(r.crs.epsg, Some(3395));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_crs_from_grid_mapping_named_object_attr() {
    let dir = tmp("_py_v3_crs_grid_mapping_named_object_attr.zarr");
    let rows = 4;
    let cols = 5;
    let data: Vec<f64> = (0..rows * cols)
        .map(|i| if i == 6 { -9999.0 } else { (i as f64) * 0.6 })
        .collect();
    write_python_style_v3_store(
        &dir,
        rows,
        cols,
        2,
        3,
        "default",
        "/",
        "zlib",
        "little",
        &data,
    );

    let zarr_json_path = std::path::Path::new(&dir).join("zarr.json");
    let mut zarr_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&zarr_json_path).unwrap()).unwrap();
    let attrs = zarr_json
        .get_mut("attributes")
        .and_then(serde_json::Value::as_object_mut)
        .expect("zarr.json attributes object missing");
    attrs.remove("crs_epsg");
    attrs.remove("epsg");
    attrs.remove("crs_wkt");
    attrs.remove("spatial_ref");
    attrs.remove("crs_proj4");
    attrs.remove("proj4");
    attrs.insert("grid_mapping".into(), json!("spatial_ref"));
    attrs.insert(
        "spatial_ref".into(),
        json!({
            "epsg_code": "EPSG:32617",
            "crs_wkt": "PROJCS[\"WGS 84 / UTM zone 17N\",GEOGCS[\"WGS 84\"]]",
            "proj4": "+proj=utm +zone=17 +datum=WGS84 +units=m +no_defs"
        }),
    );
    std::fs::write(
        &zarr_json_path,
        serde_json::to_string_pretty(&zarr_json).unwrap(),
    )
    .unwrap();

    let r = Raster::read(&dir).unwrap();
    assert_eq!(r.crs.epsg, Some(32617));
    assert!(r.crs.wkt.as_deref().unwrap_or_default().contains("UTM zone 17N"));
    assert!(r.crs.proj4.as_deref().unwrap_or_default().contains("+proj=utm"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_geotransform_string_only_attrs() {
    let dir = tmp("_py_v3_geotransform_only_attrs.zarr");
    let rows = 6;
    let cols = 7;
    let data: Vec<f64> = (0..rows * cols)
        .map(|i| if i == 9 { -9999.0 } else { (i as f64) * 0.4 })
        .collect();
    write_python_style_v3_store(
        &dir,
        rows,
        cols,
        3,
        4,
        "default",
        "/",
        "zlib",
        "little",
        &data,
    );

    let zarr_json_path = std::path::Path::new(&dir).join("zarr.json");
    let mut zarr_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&zarr_json_path).unwrap()).unwrap();
    let attrs = zarr_json
        .get_mut("attributes")
        .and_then(serde_json::Value::as_object_mut)
        .expect("zarr.json attributes object missing");
    attrs.remove("x_min");
    attrs.remove("y_min");
    attrs.remove("cell_size_x");
    attrs.remove("cell_size_y");
    attrs.remove("transform");
    attrs.insert("GeoTransform".into(), json!("100 0.5 0 -27 0 -0.5"));
    std::fs::write(
        &zarr_json_path,
        serde_json::to_string_pretty(&zarr_json).unwrap(),
    )
    .unwrap();

    let r = Raster::read(&dir).unwrap();
    let expected = Raster::from_data(
        RasterConfig {
            cols,
            rows,
            x_min: 100.0,
            y_min: -30.0,
            cell_size: 0.5,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        },
        data,
    )
    .unwrap();

    assert_raster_equal(
        &expected,
        &r,
        1e-5,
        "Python-style Zarr v3 GeoTransform-only attrs",
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_fails_on_conflicting_xmin_and_transform() {
    let dir = tmp("_py_v3_conflicting_xmin_transform.zarr");
    let rows = 4;
    let cols = 5;
    let data: Vec<f64> = (0..rows * cols).map(|i| i as f64).collect();
    write_python_style_v3_store(
        &dir,
        rows,
        cols,
        2,
        3,
        "default",
        "/",
        "zlib",
        "little",
        &data,
    );

    let zarr_json_path = std::path::Path::new(&dir).join("zarr.json");
    let mut zarr_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&zarr_json_path).unwrap()).unwrap();
    let attrs = zarr_json
        .get_mut("attributes")
        .and_then(serde_json::Value::as_object_mut)
        .expect("zarr.json attributes object missing");
    attrs.insert(
        "transform".into(),
        json!([
            100.0,
            0.5,
            0.0,
            -28.0,
            0.0,
            -0.5
        ]),
    );
    attrs.insert("x_min".into(), json!(999.0));
    std::fs::write(
        &zarr_json_path,
        serde_json::to_string_pretty(&zarr_json).unwrap(),
    )
    .unwrap();

    let err = Raster::read(&dir).expect_err("expected conflicting metadata error");
    assert!(
        format!("{err}").contains("conflicting geospatial metadata"),
        "unexpected error message: {err}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_fails_on_invalid_geotransform_string() {
    let dir = tmp("_py_v3_invalid_geotransform_string.zarr");
    let rows = 4;
    let cols = 5;
    let data: Vec<f64> = (0..rows * cols).map(|i| i as f64).collect();
    write_python_style_v3_store(
        &dir,
        rows,
        cols,
        2,
        3,
        "default",
        "/",
        "zlib",
        "little",
        &data,
    );

    let zarr_json_path = std::path::Path::new(&dir).join("zarr.json");
    let mut zarr_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&zarr_json_path).unwrap()).unwrap();
    let attrs = zarr_json
        .get_mut("attributes")
        .and_then(serde_json::Value::as_object_mut)
        .expect("zarr.json attributes object missing");
    attrs.remove("transform");
    attrs.remove("x_min");
    attrs.remove("y_min");
    attrs.remove("cell_size_x");
    attrs.remove("cell_size_y");
    attrs.insert("GeoTransform".into(), json!("100 0.5 0 bad 0 -0.5"));
    std::fs::write(
        &zarr_json_path,
        serde_json::to_string_pretty(&zarr_json).unwrap(),
    )
    .unwrap();

    let err = Raster::read(&dir).expect_err("expected invalid geotransform error");
    assert!(
        format!("{err}").contains("invalid geospatial metadata"),
        "unexpected error message: {err}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_lenient_mode_allows_conflicting_georef_metadata() {
    let dir = tmp("_py_v3_lenient_conflicting_xmin_transform.zarr");
    let rows = 4;
    let cols = 5;
    let data: Vec<f64> = (0..rows * cols).map(|i| i as f64).collect();
    write_python_style_v3_store(
        &dir,
        rows,
        cols,
        2,
        3,
        "default",
        "/",
        "zlib",
        "little",
        &data,
    );

    let zarr_json_path = std::path::Path::new(&dir).join("zarr.json");
    let mut zarr_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&zarr_json_path).unwrap()).unwrap();
    let attrs = zarr_json
        .get_mut("attributes")
        .and_then(serde_json::Value::as_object_mut)
        .expect("zarr.json attributes object missing");
    attrs.insert(
        "transform".into(),
        json!([
            100.0,
            0.5,
            0.0,
            -28.0,
            0.0,
            -0.5
        ]),
    );
    attrs.insert("x_min".into(), json!(999.0));
    attrs.insert("zarr_validation_mode".into(), json!("lenient"));
    std::fs::write(
        &zarr_json_path,
        serde_json::to_string_pretty(&zarr_json).unwrap(),
    )
    .unwrap();

    let r = Raster::read(&dir).expect("lenient mode should not fail on conflicts");
    assert!((r.x_min - 999.0).abs() < 1e-10);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_lenient_mode_allows_invalid_geotransform_string() {
    let dir = tmp("_py_v3_lenient_invalid_geotransform_string.zarr");
    let rows = 4;
    let cols = 5;
    let data: Vec<f64> = (0..rows * cols).map(|i| i as f64).collect();
    write_python_style_v3_store(
        &dir,
        rows,
        cols,
        2,
        3,
        "default",
        "/",
        "zlib",
        "little",
        &data,
    );

    let zarr_json_path = std::path::Path::new(&dir).join("zarr.json");
    let mut zarr_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&zarr_json_path).unwrap()).unwrap();
    let attrs = zarr_json
        .get_mut("attributes")
        .and_then(serde_json::Value::as_object_mut)
        .expect("zarr.json attributes object missing");
    attrs.remove("transform");
    attrs.remove("x_min");
    attrs.remove("y_min");
    attrs.remove("cell_size_x");
    attrs.remove("cell_size_y");
    attrs.insert("GeoTransform".into(), json!("100 0.5 0 bad 0 -0.5"));
    attrs.insert("zarr_validation_mode".into(), json!("lenient"));
    std::fs::write(
        &zarr_json_path,
        serde_json::to_string_pretty(&zarr_json).unwrap(),
    )
    .unwrap();

    let r = Raster::read(&dir).expect("lenient mode should ignore invalid geotransform");
    assert_eq!(r.rows, rows);
    assert_eq!(r.cols, cols);

    let _ = std::fs::remove_dir_all(&dir);
}

// ─── Phase 3 / Step 1: CF nodata conventions ─────────────────────────────────

#[test]
fn read_python_style_zarr_v3_nodata_from_cf_fill_value_attr() {
    // Producer writes `_FillValue` instead of `nodata` (xarray CF style).
    let dir = tmp("_py_v3_nodata_from_fill_value.zarr");
    let rows = 4;
    let cols = 5;
    let data: Vec<f64> = (0..rows * cols).map(|i| i as f64).collect();
    write_python_style_v3_store(&dir, rows, cols, 4, 5, "default", "/", "zlib", "little", &data);

    let zarr_json_path = std::path::Path::new(&dir).join("zarr.json");
    let mut zarr_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&zarr_json_path).unwrap()).unwrap();
    let attrs = zarr_json
        .get_mut("attributes")
        .and_then(serde_json::Value::as_object_mut)
        .expect("attributes object missing");
    attrs.remove("nodata");
    attrs.insert("_FillValue".into(), json!(-32768.0_f64));
    std::fs::write(&zarr_json_path, serde_json::to_string_pretty(&zarr_json).unwrap()).unwrap();

    let r = Raster::read(&dir).expect("should read nodata from _FillValue");
    assert!(
        (r.nodata - (-32768.0)).abs() < 1e-6,
        "expected nodata=-32768; got {}",
        r.nodata
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_nodata_from_missing_value_attr() {
    // Producer writes `missing_value` (another CF convention).
    let dir = tmp("_py_v3_nodata_from_missing_value.zarr");
    let rows = 4;
    let cols = 5;
    let data: Vec<f64> = (0..rows * cols).map(|i| i as f64).collect();
    write_python_style_v3_store(&dir, rows, cols, 4, 5, "default", "/", "zlib", "little", &data);

    let zarr_json_path = std::path::Path::new(&dir).join("zarr.json");
    let mut zarr_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&zarr_json_path).unwrap()).unwrap();
    let attrs = zarr_json
        .get_mut("attributes")
        .and_then(serde_json::Value::as_object_mut)
        .expect("attributes object missing");
    attrs.remove("nodata");
    attrs.insert("missing_value".into(), json!(-1.0_f64));
    std::fs::write(&zarr_json_path, serde_json::to_string_pretty(&zarr_json).unwrap()).unwrap();

    let r = Raster::read(&dir).expect("should read nodata from missing_value");
    assert!(
        (r.nodata - (-1.0)).abs() < 1e-6,
        "expected nodata=-1; got {}",
        r.nodata
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_explicit_nodata_takes_precedence_over_cf_fill_value() {
    // When both `nodata` and `_FillValue` are present, the explicit `nodata` key wins.
    let dir = tmp("_py_v3_nodata_precedence.zarr");
    let rows = 4;
    let cols = 5;
    let data: Vec<f64> = (0..rows * cols).map(|i| i as f64).collect();
    write_python_style_v3_store(&dir, rows, cols, 4, 5, "default", "/", "zlib", "little", &data);

    let zarr_json_path = std::path::Path::new(&dir).join("zarr.json");
    let mut zarr_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&zarr_json_path).unwrap()).unwrap();
    let attrs = zarr_json
        .get_mut("attributes")
        .and_then(serde_json::Value::as_object_mut)
        .expect("attributes object missing");
    // nodata remains -9999 from the writer; add a contradicting _FillValue
    attrs.insert("_FillValue".into(), json!(0.0_f64));
    std::fs::write(&zarr_json_path, serde_json::to_string_pretty(&zarr_json).unwrap()).unwrap();

    let r = Raster::read(&dir).expect("should read OK");
    assert!(
        (r.nodata - (-9999.0)).abs() < 1e-6,
        "expected nodata=-9999 (explicit) to win; got {}",
        r.nodata
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ─── Phase 3 / Step 1: dimension_names axis validation ───────────────────────

/// Helper: write a zarr v3 store whose zarr.json includes a given
/// `dimension_names` array in the attributes, along with standard geospatial
/// metadata.  Uses the same 2D float32 layout as the other producer fixtures.
fn write_v3_store_with_dimension_names(
    dir: &str,
    rows: usize,
    cols: usize,
    dimension_names: serde_json::Value,
    extra_attrs: Option<serde_json::Map<String, serde_json::Value>>,
    data: &[f64],
) {
    std::fs::create_dir_all(dir).unwrap();

    let mut attrs = serde_json::Map::new();
    attrs.insert("x_min".into(), json!(100.0_f64));
    attrs.insert("y_min".into(), json!(-30.0_f64));
    attrs.insert("cell_size_x".into(), json!(0.5_f64));
    attrs.insert("cell_size_y".into(), json!(0.5_f64));
    attrs.insert("nodata".into(), json!(-9999.0_f64));
    if let Some(extra) = extra_attrs {
        for (k, v) in extra {
            attrs.insert(k, v);
        }
    }

    let zarr_json = json!({
        "zarr_format": 3,
        "node_type": "array",
        "shape": [rows, cols],
        "data_type": { "name": "float32" },
        "chunk_grid": {
            "name": "regular",
            "configuration": { "chunk_shape": [rows, cols] }
        },
        "chunk_key_encoding": { "name": "default", "configuration": { "separator": "/" } },
        "fill_value": -9999.0,
        "codecs": [
            { "name": "bytes", "configuration": { "endian": "little" } },
            { "name": "zlib", "configuration": { "level": 6 } }
        ],
        "dimension_names": dimension_names,
        "attributes": attrs,
    });
    std::fs::write(
        std::path::Path::new(dir).join("zarr.json"),
        serde_json::to_string_pretty(&zarr_json).unwrap(),
    )
    .unwrap();

    // Write single chunk.
    let mut raw = Vec::with_capacity(rows * cols * 4);
    for v in data.iter() {
        raw.extend_from_slice(&(*v as f32).to_le_bytes());
    }
    use flate2::{write::ZlibEncoder, Compression};
    use std::io::Write as _;
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::new(6));
    enc.write_all(&raw).unwrap();
    let payload = enc.finish().unwrap();

    let chunk_path = std::path::Path::new(dir).join("c/0/0");
    std::fs::create_dir_all(chunk_path.parent().unwrap()).unwrap();
    std::fs::write(chunk_path, payload).unwrap();
}

#[test]
fn read_python_style_zarr_v3_dimension_names_standard_2d_y_x() {
    // Standard ["y","x"] names should be accepted and the store read normally.
    let dir = tmp("_py_v3_dim_names_y_x.zarr");
    let rows = 4;
    let cols = 5;
    let data: Vec<f64> = (0..rows * cols).map(|i| i as f64).collect();
    write_v3_store_with_dimension_names(&dir, rows, cols, json!(["y", "x"]), None, &data);
    let r = Raster::read(&dir).expect("['y','x'] dimension_names should be accepted");
    assert_eq!(r.rows, rows);
    assert_eq!(r.cols, cols);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_dimension_names_lat_lon_2d() {
    // ["lat","lon"] names should also be accepted (CF 2D convention).
    let dir = tmp("_py_v3_dim_names_lat_lon.zarr");
    let rows = 4;
    let cols = 5;
    let data: Vec<f64> = (0..rows * cols).map(|i| i as f64).collect();
    write_v3_store_with_dimension_names(&dir, rows, cols, json!(["lat", "lon"]), None, &data);
    let r = Raster::read(&dir).expect("['lat','lon'] dimension_names should be accepted");
    assert_eq!(r.rows, rows);
    assert_eq!(r.cols, cols);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_dimension_names_unrecognized_2d_accepted() {
    // Unrecognized 2D names should pass through without error (permissive for 2D).
    let dir = tmp("_py_v3_dim_names_unrecognized.zarr");
    let rows = 4;
    let cols = 5;
    let data: Vec<f64> = (0..rows * cols).map(|i| i as f64).collect();
    write_v3_store_with_dimension_names(
        &dir, rows, cols, json!(["dim_0", "dim_1"]), None, &data,
    );
    let r = Raster::read(&dir).expect("unrecognized 2D dimension_names should be accepted");
    assert_eq!(r.rows, rows);
    assert_eq!(r.cols, cols);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_dimension_names_band_last_strict_fails() {
    // A 3D store whose dimension_names suggest a band-last layout (["y","x","band"])
    // should fail in strict mode (the default) because the reader assumes band-first.
    use wbraster::error::RasterError;
    let dir = tmp("_py_v3_dim_names_band_last_strict.zarr");
    // Write as a 3D (1-band) store: shape [1, rows, cols]
    std::fs::create_dir_all(&dir).unwrap();
    let rows: usize = 4;
    let cols: usize = 5;
    let data: Vec<f64> = (0..rows * cols).map(|i| i as f64).collect();

    let zarr_json = json!({
        "zarr_format": 3,
        "node_type": "array",
        "shape": [rows, cols, 1],
        "data_type": { "name": "float32" },
        "chunk_grid": {
            "name": "regular",
            "configuration": { "chunk_shape": [rows, cols, 1] }
        },
        "chunk_key_encoding": { "name": "default", "configuration": { "separator": "/" } },
        "fill_value": -9999.0,
        "codecs": [
            { "name": "bytes", "configuration": { "endian": "little" } }
        ],
        "dimension_names": ["y", "x", "band"],
        "attributes": {
            "x_min": 100.0, "y_min": -30.0,
            "cell_size_x": 0.5, "cell_size_y": 0.5,
            "nodata": -9999.0
        }
    });
    std::fs::write(
        std::path::Path::new(&dir).join("zarr.json"),
        serde_json::to_string_pretty(&zarr_json).unwrap(),
    )
    .unwrap();

    // Write single chunk for 3D [rows, cols, 1] array.
    let mut raw = Vec::with_capacity(rows * cols * 4);
    for v in data.iter() {
        raw.extend_from_slice(&(*v as f32).to_le_bytes());
    }
    let chunk_path = std::path::Path::new(&dir).join("c/0/0/0");
    std::fs::create_dir_all(chunk_path.parent().unwrap()).unwrap();
    std::fs::write(chunk_path, &raw).unwrap();

    let result = Raster::read(&dir);
    assert!(
        result.is_err(),
        "expected error for band-last dimension_names in strict mode; got: {:?}",
        result
    );
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("dimension_names") || err_msg.contains("spatial axes"),
        "expected diagnostic dimension_names error; got: {err_msg}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_python_style_zarr_v3_dimension_names_band_last_lenient_succeeds() {
    // The same band-last store passes in lenient mode (best-effort read).
    let dir = tmp("_py_v3_dim_names_band_last_lenient.zarr");
    let rows: usize = 4;
    let cols: usize = 5;
    let data: Vec<f64> = (0..rows * cols).map(|i| i as f64).collect();
    std::fs::create_dir_all(&dir).unwrap();

    let zarr_json = json!({
        "zarr_format": 3,
        "node_type": "array",
        "shape": [rows, cols, 1],
        "data_type": { "name": "float32" },
        "chunk_grid": {
            "name": "regular",
            "configuration": { "chunk_shape": [rows, cols, 1] }
        },
        "chunk_key_encoding": { "name": "default", "configuration": { "separator": "/" } },
        "fill_value": -9999.0,
        "codecs": [
            { "name": "bytes", "configuration": { "endian": "little" } }
        ],
        "dimension_names": ["y", "x", "band"],
        "attributes": {
            "x_min": 100.0, "y_min": -30.0,
            "cell_size_x": 0.5, "cell_size_y": 0.5,
            "nodata": -9999.0,
            "zarr_validation_mode": "lenient"
        }
    });
    std::fs::write(
        std::path::Path::new(&dir).join("zarr.json"),
        serde_json::to_string_pretty(&zarr_json).unwrap(),
    )
    .unwrap();

    let mut raw = Vec::with_capacity(rows * cols * 4);
    for v in data.iter() {
        raw.extend_from_slice(&(*v as f32).to_le_bytes());
    }
    let chunk_path = std::path::Path::new(&dir).join("c/0/0/0");
    std::fs::create_dir_all(chunk_path.parent().unwrap()).unwrap();
    std::fs::write(chunk_path, &raw).unwrap();

    let r = Raster::read(&dir)
        .expect("lenient mode should allow band-last dimension_names and attempt a read");
    // Shape check: with band-last layout and shape [4,5,1], the reader sees
    // bands=shape[0]=4, rows=shape[1]=5, cols=shape[2]=1 — which is NOT the
    // intended [4 rows × 5 cols × 1 band], but lenient mode accepted the read.
    assert!(r.rows > 0);
    assert!(r.cols > 0);

    let _ = std::fs::remove_dir_all(&dir);
}

// ─── Phase 3 / Step 1: multi-scale (pyramid) group support ───────────────────

/// Write a minimal zarr v3 multi-scale group at `group_dir`.
///
/// Produces a two-level pyramid:
/// - level 0: `rows × cols`, cell_size 0.5  (full resolution)
/// - level 1: `(rows/2) × (cols/2)`, cell_size 1.0  (half resolution)
///
/// Both levels are single-band, uncompressed f32 stores.  `data0` and `data1`
/// are the f32 cell values to embed at each level.
///
/// The group's `zarr.json` includes an OME-NGFF `multiscales` attributes block.
fn write_v3_multiscale_group(
    group_dir: &str,
    rows: usize,
    cols: usize,
    data0: &[f64],   // full-res level  (rows * cols values)
    data1: &[f64],   // half-res level  ((rows/2) * (cols/2) values)
) {
    let rows1 = rows / 2;
    let cols1 = cols / 2;

    // ── helper: write a single-level v3 array ─────────────────────────────
    let write_level = |level_dir: &std::path::Path, r: usize, c: usize, data: &[f64],
                        cell: f64, x_min: f64, y_min: f64| {
        std::fs::create_dir_all(level_dir).unwrap();
        let zarr_json = json!({
            "zarr_format": 3,
            "node_type": "array",
            "shape": [r, c],
            "data_type": { "name": "float32" },
            "chunk_grid": {
                "name": "regular",
                "configuration": { "chunk_shape": [r, c] }
            },
            "chunk_key_encoding": { "name": "default", "configuration": { "separator": "/" } },
            "fill_value": -9999.0,
            "codecs": [{ "name": "bytes", "configuration": { "endian": "little" } }],
            "attributes": {
                "x_min": x_min,
                "y_min": y_min,
                "cell_size_x": cell,
                "cell_size_y": cell,
                "nodata": -9999.0
            }
        });
        std::fs::write(
            level_dir.join("zarr.json"),
            serde_json::to_string_pretty(&zarr_json).unwrap(),
        ).unwrap();

        // Write single uncompressed chunk.
        let mut raw = Vec::with_capacity(r * c * 4);
        for v in data.iter() {
            raw.extend_from_slice(&(*v as f32).to_le_bytes());
        }
        let chunk_path = level_dir.join("c/0/0");
        std::fs::create_dir_all(chunk_path.parent().unwrap()).unwrap();
        std::fs::write(chunk_path, &raw).unwrap();
    };

    let base = std::path::Path::new(group_dir);
    std::fs::create_dir_all(base).unwrap();

    write_level(&base.join("0"), rows,  cols,  data0, 0.5, 100.0, -30.0);
    write_level(&base.join("1"), rows1, cols1, data1, 1.0, 100.0, -30.0);

    // ── group zarr.json with OME-NGFF multiscales attribute ───────────────
    let group_json = json!({
        "zarr_format": 3,
        "node_type": "group",
        "attributes": {
            "multiscales": [{
                "version": "0.5",
                "datasets": [
                    { "path": "0" },
                    { "path": "1" }
                ]
            }]
        }
    });
    std::fs::write(
        base.join("zarr.json"),
        serde_json::to_string_pretty(&group_json).unwrap(),
    ).unwrap();
}

/// Write a minimal zarr v2 multi-scale group at `group_dir`.
///
/// Same two-level layout as the v3 helper above, but uses `.zgroup` /
/// `.zarray` / `.zattrs` conventions.
fn write_v2_multiscale_group(
    group_dir: &str,
    rows: usize,
    cols: usize,
    data0: &[f64],
    data1: &[f64],
) {
    let rows1 = rows / 2;
    let cols1 = cols / 2;

    let write_level = |level_dir: &std::path::Path, r: usize, c: usize, data: &[f64],
                        cell: f64, x_min: f64, y_min: f64| {
        std::fs::create_dir_all(level_dir).unwrap();

        let zarray = json!({
            "zarr_format": 2,
            "shape": [r, c],
            "chunks": [r, c],
            "dtype": "<f4",
            "compressor": null,
            "fill_value": -9999.0,
            "order": "C",
            "filters": null
        });
        std::fs::write(
            level_dir.join(".zarray"),
            serde_json::to_string_pretty(&zarray).unwrap(),
        ).unwrap();

        let zattrs = json!({
            "x_min": x_min,
            "y_min": y_min,
            "cell_size_x": cell,
            "cell_size_y": cell,
            "nodata": -9999.0
        });
        std::fs::write(
            level_dir.join(".zattrs"),
            serde_json::to_string_pretty(&zattrs).unwrap(),
        ).unwrap();

        // Single uncompressed chunk (v2 key "0.0").
        let mut raw = Vec::with_capacity(r * c * 4);
        for v in data.iter() {
            raw.extend_from_slice(&(*v as f32).to_le_bytes());
        }
        std::fs::write(level_dir.join("0.0"), &raw).unwrap();
    };

    let base = std::path::Path::new(group_dir);
    std::fs::create_dir_all(base).unwrap();

    write_level(&base.join("0"), rows,  cols,  data0, 0.5, 100.0, -30.0);
    write_level(&base.join("1"), rows1, cols1, data1, 1.0, 100.0, -30.0);

    // .zgroup at group root.
    std::fs::write(base.join(".zgroup"), r#"{"zarr_format":2}"#).unwrap();

    // .zattrs with OME-NGFF multiscales.
    let zattrs = json!({
        "multiscales": [{
            "version": "0.5",
            "datasets": [
                { "path": "0" },
                { "path": "1" }
            ]
        }]
    });
    std::fs::write(
        base.join(".zattrs"),
        serde_json::to_string_pretty(&zattrs).unwrap(),
    ).unwrap();
}

#[test]
fn read_zarr_v3_multiscale_group_reads_full_res_by_default() {
    // Opening a group root returns the full-resolution (level 0) array.
    let dir = tmp("_v3_multiscale_group_default.zarr");
    let rows = 8;
    let cols = 10;
    let data0: Vec<f64> = (0..rows * cols).map(|i| i as f64).collect();
    let data1: Vec<f64> = (0..(rows / 2) * (cols / 2)).map(|i| (i * 4) as f64).collect();
    write_v3_multiscale_group(&dir, rows, cols, &data0, &data1);

    let r = Raster::read(&dir).expect("v3 group root should open at level 0");
    assert_eq!(r.rows, rows, "expected full-res rows");
    assert_eq!(r.cols, cols, "expected full-res cols");
    assert!(
        (r.cell_size_x - 0.5).abs() < 1e-6,
        "expected level-0 cell size 0.5; got {}",
        r.cell_size_x
    );
    // Spot-check first value from the full-res level.
    assert!(
        (r.get(0, 0, 0) - 0.0).abs() < 1e-3,
        "expected data[0,0,0]=0; got {}",
        r.get(0, 0, 0)
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_zarr_v3_multiscale_group_level1_via_direct_path() {
    // Pointing directly at the level-1 sub-array yields the coarser resolution.
    let dir = tmp("_v3_multiscale_direct_level1.zarr");
    let rows = 8;
    let cols = 10;
    let data0: Vec<f64> = (0..rows * cols).map(|i| i as f64).collect();
    let data1: Vec<f64> = (0..(rows / 2) * (cols / 2)).map(|i| (i * 4) as f64).collect();
    write_v3_multiscale_group(&dir, rows, cols, &data0, &data1);

    let level1_path = format!("{dir}/1");
    let r = Raster::read(&level1_path).expect("direct sub-array path should open level 1");
    assert_eq!(r.rows, rows / 2, "expected half-res rows");
    assert_eq!(r.cols, cols / 2, "expected half-res cols");
    assert!(
        (r.cell_size_x - 1.0).abs() < 1e-6,
        "expected level-1 cell size 1.0; got {}",
        r.cell_size_x
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_zarr_v3_multiscale_group_fallback_no_ome_attrs() {
    // A group without a multiscales attribute block falls back to numeric
    // sub-directory scanning and still opens level 0.
    let dir = tmp("_v3_multiscale_fallback.zarr");
    let rows = 6;
    let cols = 8;
    let data0: Vec<f64> = (0..rows * cols).map(|i| i as f64).collect();
    let data1: Vec<f64> = (0..(rows / 2) * (cols / 2)).map(|i| (i * 4) as f64).collect();
    write_v3_multiscale_group(&dir, rows, cols, &data0, &data1);

    // Overwrite the group zarr.json with one that has NO multiscales attribute.
    let bare_group = json!({
        "zarr_format": 3,
        "node_type": "group",
        "attributes": {}
    });
    std::fs::write(
        std::path::Path::new(&dir).join("zarr.json"),
        serde_json::to_string_pretty(&bare_group).unwrap(),
    ).unwrap();

    let r = Raster::read(&dir).expect("fallback numeric scan should open level 0");
    assert_eq!(r.rows, rows);
    assert_eq!(r.cols, cols);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_zarr_v2_multiscale_group_reads_full_res_by_default() {
    let dir = tmp("_v2_multiscale_group_default.zarr");
    let rows = 8;
    let cols = 10;
    let data0: Vec<f64> = (0..rows * cols).map(|i| i as f64).collect();
    let data1: Vec<f64> = (0..(rows / 2) * (cols / 2)).map(|i| (i * 4) as f64).collect();
    write_v2_multiscale_group(&dir, rows, cols, &data0, &data1);

    let r = Raster::read(&dir).expect("v2 group root should open at level 0");
    assert_eq!(r.rows, rows, "expected full-res rows");
    assert_eq!(r.cols, cols, "expected full-res cols");
    assert!(
        (r.cell_size_x - 0.5).abs() < 1e-6,
        "expected level-0 cell size 0.5; got {}",
        r.cell_size_x
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_zarr_v2_multiscale_group_level1_via_direct_path() {
    let dir = tmp("_v2_multiscale_direct_level1.zarr");
    let rows = 8;
    let cols = 10;
    let data0: Vec<f64> = (0..rows * cols).map(|i| i as f64).collect();
    let data1: Vec<f64> = (0..(rows / 2) * (cols / 2)).map(|i| (i * 4) as f64).collect();
    write_v2_multiscale_group(&dir, rows, cols, &data0, &data1);

    let level1_path = format!("{dir}/1");
    let r = Raster::read(&level1_path).expect("direct v2 sub-array path should open level 1");
    assert_eq!(r.rows, rows / 2);
    assert_eq!(r.cols, cols / 2);
    assert!(
        (r.cell_size_x - 1.0).abs() < 1e-6,
        "expected level-1 cell size 1.0; got {}",
        r.cell_size_x
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_zarr_v2_multiscale_group_fallback_no_ome_attrs() {
    // Group with .zgroup but no .zattrs multiscales — falls back to numeric scan.
    let dir = tmp("_v2_multiscale_fallback.zarr");
    let rows = 6;
    let cols = 8;
    let data0: Vec<f64> = (0..rows * cols).map(|i| i as f64).collect();
    let data1: Vec<f64> = (0..(rows / 2) * (cols / 2)).map(|i| (i * 4) as f64).collect();
    write_v2_multiscale_group(&dir, rows, cols, &data0, &data1);

    // Remove the .zattrs so there is no multiscales block.
    let _ = std::fs::remove_file(std::path::Path::new(&dir).join(".zattrs"));

    let r = Raster::read(&dir).expect("fallback numeric scan should open level 0");
    assert_eq!(r.rows, rows);
    assert_eq!(r.cols, cols);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_zarr_v3_group_with_no_levels_returns_error() {
    // An empty group (no array children) should return an error, not panic.
    let dir = tmp("_v3_empty_group.zarr");
    std::fs::create_dir_all(&dir).unwrap();
    let group_json = json!({ "zarr_format": 3, "node_type": "group", "attributes": {} });
    std::fs::write(
        std::path::Path::new(&dir).join("zarr.json"),
        serde_json::to_string_pretty(&group_json).unwrap(),
    ).unwrap();

    let result = Raster::read(&dir);
    assert!(result.is_err(), "empty group should return an error");
    let msg = format!("{}", result.unwrap_err());
    assert!(
        msg.contains("group") || msg.contains("sub-director"),
        "expected diagnostic about empty group; got: {msg}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn existing_zarr_v3_array_reads_unaffected_by_group_detection() {
    // A plain v3 array at the root (no .zgroup / group node_type) should
    // continue to be read exactly as before.
    let dir = tmp("_v3_plain_array_unaffected.zarr");
    let rows = 4;
    let cols = 5;
    let data: Vec<f64> = (0..rows * cols).map(|i| i as f64).collect();
    write_python_style_v3_store(&dir, rows, cols, 4, 5, "default", "/", "zlib", "little", &data);

    let r = Raster::read(&dir).expect("plain v3 array should still read fine");
    assert_eq!(r.rows, rows);
    assert_eq!(r.cols, cols);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn external_zarr_v2_fixture_smoke_local_path() {
    let Some(path) = env_var_trimmed("WBRASTER_EXTERNAL_ZARR_V2_FIXTURE") else {
        eprintln!(
            "skipping: set WBRASTER_EXTERNAL_ZARR_V2_FIXTURE to a local .zarr directory"
        );
        return;
    };

    let r = Raster::read(&path).unwrap_or_else(|e| {
        panic!(
            "failed reading external v2 fixture at '{}': {e}",
            path
        )
    });
    assert!(r.rows > 0, "external v2 fixture has no rows");
    assert!(r.cols > 0, "external v2 fixture has no cols");
    assert_external_fixture_expectations(&r, "WBRASTER_EXTERNAL_ZARR_V2");
}

#[test]
fn external_zarr_v3_fixture_smoke_local_path() {
    let Some(path) = env_var_trimmed("WBRASTER_EXTERNAL_ZARR_V3_FIXTURE") else {
        eprintln!(
            "skipping: set WBRASTER_EXTERNAL_ZARR_V3_FIXTURE to a local .zarr directory"
        );
        return;
    };

    let r = Raster::read(&path).unwrap_or_else(|e| {
        panic!(
            "failed reading external v3 fixture at '{}': {e}",
            path
        )
    });
    assert!(r.rows > 0, "external v3 fixture has no rows");
    assert!(r.cols > 0, "external v3 fixture has no cols");
    assert_external_fixture_expectations(&r, "WBRASTER_EXTERNAL_ZARR_V3");
}

// ─── Zarr v3 transpose codec ──────────────────────────────────────────────────

/// Write a zarr v3 store whose chunks are encoded with a `transpose` codec so
/// that the reader's inverse-transpose logic is exercised by actual read tests.
///
/// `order_json` is the value passed as `"order"` in `zarr.json`.  The helper
/// writes f64 chunks with the bytes laid out exactly as a spec-compliant zarr
/// v3 writer would after applying `np.transpose(chunk, axes=order)`.
#[allow(clippy::too_many_arguments)]
fn write_transpose_v3_store(
    dir: &str,
    rows: usize,
    cols: usize,
    chunk_rows: usize,
    chunk_cols: usize,
    order_json: &serde_json::Value,
    data: &[f64],
) {
    // Resolve the permutation to a concrete Vec<usize>.
    let ndim = 2usize;
    let order_vec: Vec<usize> = match order_json {
        serde_json::Value::String(s) if s.eq_ignore_ascii_case("C") => vec![0, 1],
        serde_json::Value::String(s) if s.eq_ignore_ascii_case("F") => vec![1, 0],
        serde_json::Value::Array(arr) => arr
            .iter()
            .map(|v| v.as_u64().expect("order element must be integer") as usize)
            .collect(),
        other => panic!("write_transpose_v3_store: unsupported order: {other:?}"),
    };

    std::fs::create_dir_all(dir).unwrap();

    let zarr_json = json!({
        "zarr_format": 3,
        "node_type": "array",
        "shape": [rows, cols],
        "data_type": "float64",
        "chunk_grid": {
            "name": "regular",
            "configuration": { "chunk_shape": [chunk_rows, chunk_cols] }
        },
        "chunk_key_encoding": {
            "name": "default",
            "configuration": { "separator": "/" }
        },
        "codecs": [
            { "name": "transpose", "configuration": { "order": order_json } },
            { "name": "bytes", "configuration": { "endian": "little" } }
        ],
        "fill_value": -9999.0,
        "attributes": {
            "x_min": 100.0,
            "y_min": -30.0,
            "cell_size_x": 0.5,
            "cell_size_y": 0.5,
            "nodata": -9999.0
        }
    });
    std::fs::write(
        std::path::Path::new(dir).join("zarr.json"),
        serde_json::to_string_pretty(&zarr_json).unwrap(),
    )
    .unwrap();

    // Precompute strides once.
    let n_chunk_rows = rows.div_ceil(chunk_rows);
    let n_chunk_cols = cols.div_ceil(chunk_cols);

    for cr in 0..n_chunk_rows {
        for cc in 0..n_chunk_cols {
            let this_rows = (rows - cr * chunk_rows).min(chunk_rows);
            let this_cols = (cols - cc * chunk_cols).min(chunk_cols);

            // Stored (transposed) shape: stored_shape[i] = this_shape[order[i]]
            let this_shape = [this_rows, this_cols];
            let stored_shape: Vec<usize> =
                order_vec.iter().map(|&ax| this_shape[ax]).collect();

            let mut stored_strides = vec![1usize; ndim];
            for d in (0..ndim - 1).rev() {
                stored_strides[d] = stored_strides[d + 1] * stored_shape[d + 1];
            }

            let n = stored_shape.iter().product::<usize>();
            let mut raw = Vec::with_capacity(n * 8);

            for k in 0..n {
                // Decompose stored flat index → stored_coords → local original coords.
                let mut rem = k;
                let mut local_orig = [0usize; 2];
                for i in 0..ndim {
                    let coord_i = rem / stored_strides[i];
                    rem %= stored_strides[i];
                    local_orig[order_vec[i]] = coord_i;
                }
                let row = cr * chunk_rows + local_orig[0];
                let col = cc * chunk_cols + local_orig[1];
                raw.extend_from_slice(&data[row * cols + col].to_le_bytes());
            }

            let key = format!("c/{cr}/{cc}");
            let path = std::path::Path::new(dir).join(&key);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, raw).unwrap();
        }
    }
}

#[test]
fn zarr_v3_transpose_f_order_string_single_chunk() {
    // 3×4 array, one chunk, order="F" (stored as column-major).
    let dir = tmp("_v3_transpose_f_str.zarr");
    let rows = 3;
    let cols = 4;
    let data: Vec<f64> = (0..(rows * cols)).map(|i| i as f64 * 1.5).collect();

    write_transpose_v3_store(&dir, rows, cols, rows, cols, &json!("F"), &data);

    let r = Raster::read(&dir).unwrap();
    assert_eq!(r.rows, rows);
    assert_eq!(r.cols, cols);
    for row in 0..rows {
        for col in 0..cols {
            let expected = data[row * cols + col];
            let got = r.get_raw(0, row as isize, col as isize).unwrap();
            assert!(
                (got - expected).abs() < 1e-9,
                "transpose F-string: mismatch at ({row},{col}): expected {expected}, got {got}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn zarr_v3_transpose_explicit_permutation_single_chunk() {
    // Same geometry as above but order specified as explicit [1, 0] array.
    let dir = tmp("_v3_transpose_explicit.zarr");
    let rows = 3;
    let cols = 4;
    let data: Vec<f64> = (0..(rows * cols)).map(|i| (i as f64) * 0.25 - 1.0).collect();

    write_transpose_v3_store(&dir, rows, cols, rows, cols, &json!([1, 0]), &data);

    let r = Raster::read(&dir).unwrap();
    assert_eq!(r.rows, rows);
    assert_eq!(r.cols, cols);
    for row in 0..rows {
        for col in 0..cols {
            let expected = data[row * cols + col];
            let got = r.get_raw(0, row as isize, col as isize).unwrap();
            assert!(
                (got - expected).abs() < 1e-9,
                "transpose explicit [1,0]: mismatch at ({row},{col}): expected {expected}, got {got}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn zarr_v3_transpose_c_order_is_noop() {
    // order="C" is the identity permutation; values must be readable unchanged.
    let dir = tmp("_v3_transpose_c.zarr");
    let rows = 4;
    let cols = 5;
    let data: Vec<f64> = (0..(rows * cols)).map(|i| i as f64 - 5.0).collect();

    write_transpose_v3_store(&dir, rows, cols, rows, cols, &json!("C"), &data);

    let r = Raster::read(&dir).unwrap();
    for row in 0..rows {
        for col in 0..cols {
            let expected = data[row * cols + col];
            let got = r.get_raw(0, row as isize, col as isize).unwrap();
            assert!(
                (got - expected).abs() < 1e-9,
                "transpose C noop: mismatch at ({row},{col})"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn zarr_v3_transpose_f_order_multichunk() {
    // 9×8 array chunked 4×3 — exercises multiple chunks including boundary chunks.
    let dir = tmp("_v3_transpose_multichunk.zarr");
    let rows = 9;
    let cols = 8;
    let data: Vec<f64> = (0..(rows * cols))
        .map(|i| if i == 13 { -9999.0 } else { i as f64 * 0.2 })
        .collect();

    write_transpose_v3_store(&dir, rows, cols, 4, 3, &json!("F"), &data);

    let r = Raster::read(&dir).unwrap();
    assert_eq!(r.rows, rows);
    assert_eq!(r.cols, cols);
    for row in 0..rows {
        for col in 0..cols {
            let expected = data[row * cols + col];
            let got = r.get_raw(0, row as isize, col as isize).unwrap();
            let is_nodata = r.is_nodata(expected);
            if is_nodata {
                assert!(
                    r.is_nodata(got),
                    "transpose multichunk: expected nodata at ({row},{col}), got {got}"
                );
            } else {
                assert!(
                    (got - expected).abs() < 1e-9,
                    "transpose multichunk: mismatch at ({row},{col}): expected {expected}, got {got}"
                );
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

fn write_v3_metadata_only(dir: &str, zarr_json: serde_json::Value) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        std::path::Path::new(dir).join("zarr.json"),
        serde_json::to_string_pretty(&zarr_json).unwrap(),
    )
    .unwrap();
}

#[test]
fn zarr_v3_rejects_unknown_codec_with_actionable_message() {
    let dir = tmp("_v3_unknown_codec.zarr");
    write_v3_metadata_only(
        &dir,
        json!({
            "zarr_format": 3,
            "node_type": "array",
            "shape": [3, 4],
            "data_type": "float32",
            "chunk_grid": {
                "name": "regular",
                "configuration": { "chunk_shape": [3, 4] }
            },
            "chunk_key_encoding": {
                "name": "default",
                "configuration": { "separator": "/" }
            },
            "codecs": [
                { "name": "bytes", "configuration": { "endian": "little" } },
                { "name": "shuffle", "configuration": {} }
            ],
            "fill_value": -9999.0,
            "attributes": { "x_min": 0.0, "y_min": 0.0, "cell_size_x": 1.0 }
        }),
    );

    let err = Raster::read(&dir).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("unsupported zarr v3 codec 'shuffle'") && msg.contains("cannot safely decode"),
        "expected actionable unknown-codec error, got: {msg}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn zarr_v3_rejects_zero_dimension_shape() {
    let dir = tmp("_v3_zero_dim_shape.zarr");
    write_v3_metadata_only(
        &dir,
        json!({
            "zarr_format": 3,
            "node_type": "array",
            "shape": [0, 4],
            "data_type": "float32",
            "chunk_grid": {
                "name": "regular",
                "configuration": { "chunk_shape": [1, 4] }
            },
            "chunk_key_encoding": {
                "name": "default",
                "configuration": { "separator": "/" }
            },
            "codecs": [
                { "name": "bytes", "configuration": { "endian": "little" } }
            ],
            "fill_value": -9999.0,
            "attributes": { "x_min": 0.0, "y_min": 0.0, "cell_size_x": 1.0 }
        }),
    );

    let err = Raster::read(&dir).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("shape") && msg.contains("zero dimension"),
        "expected zero-dimension shape error, got: {msg}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn zarr_v3_rejects_shape_chunk_rank_mismatch() {
    let dir = tmp("_v3_shape_chunk_rank_mismatch.zarr");
    write_v3_metadata_only(
        &dir,
        json!({
            "zarr_format": 3,
            "node_type": "array",
            "shape": [3, 4],
            "data_type": "float32",
            "chunk_grid": {
                "name": "regular",
                "configuration": { "chunk_shape": [1, 3, 4] }
            },
            "chunk_key_encoding": {
                "name": "default",
                "configuration": { "separator": "/" }
            },
            "codecs": [
                { "name": "bytes", "configuration": { "endian": "little" } }
            ],
            "fill_value": -9999.0,
            "attributes": { "x_min": 0.0, "y_min": 0.0, "cell_size_x": 1.0 }
        }),
    );

    let err = Raster::read(&dir).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("chunk_shape") && msg.contains("shape") && msg.contains("must match"),
        "expected chunk/shape rank mismatch error, got: {msg}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ─── Raster API ───────────────────────────────────────────────────────────────

#[test]
fn statistics_api() {
    let r = make_test_raster();
    let stats = r.statistics();
    assert_eq!(stats.valid_count, 23);   // one nodata cell
    assert_eq!(stats.nodata_count, 1);
    assert!((stats.min - 0.0).abs() < 1e-10);
    assert!((stats.max - 11.5).abs() < 1e-10);
}

#[test]
fn world_to_pixel_api() {
    let r = make_test_raster();
    // x_min=100.0, cell=0.5, cols=6 → x_max=103.0
    // y_min=-30.0, cell=0.5, rows=4 → y_max=-28.0
    assert_eq!(r.world_to_pixel(100.1, -28.1), Some((0, 0)));
    assert_eq!(r.world_to_pixel(102.9, -29.9), Some((5, 3)));
    assert_eq!(r.world_to_pixel(99.0, -28.0), None); // out of bounds
}

#[test]
fn map_valid_api() {
    let mut r = make_test_raster();
    r.map_valid(|v| v * 2.0);
    // Cell (0,0) was 0.0 → still 0.0
    assert_eq!(r.get(0, 0, 0), 0.0);
    // Cell (1,0) was 0.5 → 1.0
    assert!((r.get(0, 0, 1) - 1.0).abs() < 1e-10);
    // Nodata cell (5,0) still nodata
    assert!(r.is_nodata(r.get(0, 0, 5)));
    assert_eq!(r.get_opt(0, 0, 5), None);
}

#[test]
fn iter_valid_api() {
    let r = make_test_raster();
    let valid: Vec<_> = r.iter_valid().collect();
    assert_eq!(valid.len(), 23);
}

#[test]
fn extent_api() {
    let r = make_test_raster();
    let e = r.extent();
    assert!((e.x_min - 100.0).abs() < 1e-10);
    assert!((e.y_min - -30.0).abs() < 1e-10);
    assert!((e.x_max - 103.0).abs() < 1e-10);
    assert!((e.y_max - -28.0).abs() < 1e-10);
}

#[test]
fn display_format() {
    let r = make_test_raster();
    let s = format!("{r}");
    assert!(s.contains("6×4"), "got: {s}");
}
