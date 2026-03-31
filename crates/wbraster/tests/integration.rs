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
