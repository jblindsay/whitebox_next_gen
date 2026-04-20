//! JPEG 2000 / GeoJP2 adapter for wbraster.

#[cfg(feature = "jpeg2000-vendored-bridge")]
use wbjpeg2000 as dj2k;

use super::jpeg2000_core as jp2;
use crate::error::{RasterError, Result};
use crate::raster::{DataType, Raster, RasterConfig};
use crate::crs_info::CrsInfo;

/// Default target quality used for JPEG2000 lossy output when no compression
/// option is provided.
pub const JPEG2000_DEFAULT_LOSSY_QUALITY_DB: f32 = 35.0;

fn color_interpretation_from_jpeg2000(
    color_space: jp2::ColorSpace,
    bands: usize,
    data_type: DataType,
) -> &'static str {
    match color_space {
        jp2::ColorSpace::Srgb => {
            if bands == 1 && data_type == DataType::U32 {
                "packed_rgb"
            } else {
                "rgb"
            }
        }
        jp2::ColorSpace::YCbCr => "ycbcr",
        jp2::ColorSpace::Greyscale => "gray",
        jp2::ColorSpace::MultiBand => "multiband",
    }
}

/// Typed compression choices for JPEG2000 writes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Jpeg2000Compression {
    /// Reversible 5/3 wavelet compression.
    Lossless,
    /// Irreversible 9/7 wavelet compression with a target quality (dB).
    Lossy {
        /// Target quality in dB.
        quality_db: f32,
    },
}

impl Jpeg2000Compression {
    fn to_core(self) -> jp2::CompressionMode {
        match self {
            Self::Lossless => jp2::CompressionMode::Lossless,
            Self::Lossy { quality_db } => jp2::CompressionMode::Lossy { quality_db },
        }
    }
}

/// Typed write options for JPEG2000 / GeoJP2 output.
#[derive(Debug, Clone, Default)]
pub struct Jpeg2000WriteOptions {
    /// Compression mode.
    pub compression: Option<Jpeg2000Compression>,
    /// Number of decomposition levels.
    pub decomp_levels: Option<u8>,
    /// Optional color space override.
    pub color_space: Option<jp2::ColorSpace>,
}

/// Read JPEG2000 / GeoJP2 from `path`.
pub fn read(path: &str) -> Result<Raster> {
    eprintln!("[jpeg2000::read] path={}", path);
    let jp2f = jp2::GeoJp2::open(path)
        .map_err(|e| RasterError::Other(format!("JPEG2000 read error: {e}")))?;

    let cols = jp2f.width() as usize;
    let rows = jp2f.height() as usize;

    // Keep external decode bridge feature-gated so core can be built without it.
    #[cfg(feature = "jpeg2000-vendored-bridge")]
    let (bands, data_type, data) = match decode_samples_with_dj2k(path, rows, cols) {
        Ok(decoded) => {
            eprintln!("[jpeg2000::read] Successfully decoded with vendored bridge");
            decoded
        },
        Err(ext_err) => {
            eprintln!("[jpeg2000::read] Vendored bridge failed: {}, falling back to native", ext_err);
            // Avoid silently returning potentially corrupted multiband output
            // from the legacy native path if vendored decode fails.
            if jp2f.component_count() > 1 {
                return Err(RasterError::Other(format!(
                    "JPEG2000 vendored decode failed for multiband image: {ext_err}"
                )));
            }
            eprintln!("[jpeg2000::read] Using native decoder");
            decode_samples_with_internal_reader(&jp2f, rows, cols)?
        }
    };

    #[cfg(not(feature = "jpeg2000-vendored-bridge"))]
    let (bands, data_type, data) = {
        eprintln!("[jpeg2000::read] Using native decoder (vendor bridge disabled)");
        decode_samples_with_internal_reader(&jp2f, rows, cols)?
    };

    let mut x_min = 0.0;
    let mut y_min = 0.0;
    let mut cell_size = 1.0;
    let mut cell_size_y = Some(1.0);
    if let Some(gtx) = jp2f.geo_transform() {
        x_min = gtx.x_origin;
        cell_size = gtx.pixel_width.abs();
        cell_size_y = Some(gtx.pixel_height.abs());
        let y_max = gtx.y_origin;
        y_min = y_max + gtx.pixel_height * rows as f64;
    }

    let crs = CrsInfo {
        epsg: jp2f.epsg().map(u32::from),
        ..Default::default()
    };

    let nodata = jp2f.no_data().unwrap_or(-9999.0);
    let data_type = data_type;
    let color_space = jp2f.color_space();

    let metadata = vec![
        (
            "jpeg2000_compression".into(),
            if jp2f.is_lossless() {
                "lossless".into()
            } else {
                "lossy".into()
            },
        ),
        (
            "jpeg2000_color_space".into(),
            format!("{:?}", color_space).to_ascii_lowercase(),
        ),
        (
            "color_interpretation".into(),
            color_interpretation_from_jpeg2000(color_space, bands, data_type).to_string(),
        ),
    ];

    let cfg = RasterConfig {
        cols,
        rows,
        bands,
        x_min,
        y_min,
        cell_size,
        cell_size_y,
        nodata,
        data_type,
        crs: crs,        metadata,
    };

    Raster::from_data(cfg, data)
}

#[cfg(feature = "jpeg2000-vendored-bridge")]
/// Detect and correct for dicom-toolkit-jpeg2000 precision misreporting bug.
///
/// Some JPEG2000 decoders (including dicom-toolkit-jpeg2000) misinterpret the
/// precision field in certain JP2 headers, causing an off-by-one bit-depth error.
/// This manifests as a 2x scaling error for certain files (e.g., Sentinel-2 B03).
///
/// This function checks if the decoder reports an odd bit-depth (typically 15)
/// for a file with 16-bit samples. JP2 bit-depths should match bytes_per_sample:
/// - bit_depth <= 8 → bytes_per_sample = 1
/// - bit_depth > 8 → bytes_per_sample = 2 (16-bit storage)
/// An off-by-one bit-depth (e.g., 15 instead of 16) with 2-byte storage
/// indicates a dequantization error that requires 2x correction.
fn detect_and_correct_precision_bug(reported_bit_depth: u8, bytes_per_sample: u8) -> f64 {
    // JP2 convention: bit_depth > 8 always uses 2-byte storage
    // If we have 2-byte storage but reported bit_depth is <= 15, check for off-by-one
    if bytes_per_sample == 2 && reported_bit_depth > 8 && reported_bit_depth < 16 {
        // Likely precision off-by-one: actual is 16-bit but reported as 15-bit
        // This causes 2x scaling error during dequantization
        eprintln!("[bridge] PRECISION BUG DETECTED: reported {} bits with 2-byte storage", reported_bit_depth);
        2.0
    } else {
        1.0
    }
}

fn decode_samples_with_dj2k(path: &str, rows: usize, cols: usize) -> Result<(usize, DataType, Vec<f64>)> {
    let bytes = std::fs::read(path)?;
    let image = dj2k::Image::new(&bytes, &dj2k::DecodeSettings::default())
        .map_err(|e| RasterError::Other(format!("JPEG2000 decode init error: {e}")))?;
    
    eprintln!("[bridge] Image dimensions: {}x{}", image.width(), image.height());
    eprintln!("[bridge] Color space: {:?}", image.color_space());
    eprintln!("[bridge] Original bit depth: {}", image.original_bit_depth());
    
    let raw = image
        .decode_native()
        .map_err(|e| RasterError::Other(format!("JPEG2000 native decode error: {e}")))?;

    
    eprintln!("[bridge] Raw dimensions: {}x{}", raw.width, raw.height);
    eprintln!("[bridge] Raw bytes_per_sample: {}", raw.bytes_per_sample);
    eprintln!("[bridge] Raw num_components: {}", raw.num_components);
    eprintln!("[bridge] Raw bit_depth: {}", raw.bit_depth);
    eprintln!("[bridge] Raw data length: {}", raw.data.len());
    
    let bands = raw.num_components as usize;
    let npix = rows * cols;
    let expected_samples = npix
        .checked_mul(bands)
        .ok_or_else(|| RasterError::CorruptData("JPEG2000 sample count overflow".into()))?;

    if raw.width as usize != cols || raw.height as usize != rows {
        return Err(RasterError::CorruptData(format!(
            "JPEG2000 dimension mismatch: metadata={}x{}, decoded={}x{}",
            cols, rows, raw.width, raw.height
        )));
    }

    let mut data = vec![0.0; expected_samples];
    match raw.bytes_per_sample {
        1 => {
            if raw.data.len() != expected_samples {
                return Err(RasterError::CorruptData(format!(
                    "JPEG2000 decoded byte count mismatch: expected {}, got {}",
                    expected_samples,
                    raw.data.len()
                )));
            }
            for p in 0..npix {
                for b in 0..bands {
                    data[b * npix + p] = raw.data[p * bands + b] as f64;
                }
            }
            eprintln!("[bridge] 8-bit path: first 10 pixels (component 0): {:?}", 
                     &data[0..10.min(npix)]);
            Ok((bands, DataType::U8, data))
        }
        2 => {
            if raw.data.len() != expected_samples * 2 {
                return Err(RasterError::CorruptData(format!(
                    "JPEG2000 decoded byte count mismatch: expected {}, got {}",
                    expected_samples * 2,
                    raw.data.len()
                )));
            }
            for p in 0..npix {
                for b in 0..bands {
                    let sample_idx = p * bands + b;
                    let byte_idx = sample_idx * 2;
                    let sample = u16::from_le_bytes([raw.data[byte_idx], raw.data[byte_idx + 1]]);
                    data[b * npix + p] = sample as f64;
                }
            }
            
            eprintln!("[bridge] 16-bit path: first 10 pixels (component 0): {:?}", 
                     &data[0..10.min(npix)]);
            
            // WORKAROUND: dicom crate may misreport precision for some JPEG2000 files.
            // Some Sentinel-2 images report 15-bit but contain 16-bit data.
            // This causes a 2x scaling error in dequantization (2^15 vs 2^16).
            let scale_correction = detect_and_correct_precision_bug(raw.bit_depth, raw.bytes_per_sample as u8);
            
            if scale_correction > 1.0 {
                eprintln!("[bridge] PRECISION CORRECTION: Applying {:.1}x scale factor", scale_correction);
                eprintln!("[bridge]   - Decoder reported bit_depth: {}", raw.bit_depth);
                for v in data.iter_mut() {
                    *v *= scale_correction;
                }
            }
            
            eprintln!("[bridge] 16-bit path: first 10 pixels after correction: {:?}", 
                     &data[0..10.min(npix)].iter().map(|&x| x as u16).collect::<Vec<_>>());
            Ok((bands, DataType::U16, data))
        }
        other => Err(RasterError::Other(format!(
            "Unsupported JPEG2000 native sample width: {} bytes",
            other
        ))),
    }
}

fn decode_samples_with_internal_reader(
    jp2f: &jp2::GeoJp2,
    rows: usize,
    cols: usize,
) -> Result<(usize, DataType, Vec<f64>)> {
    let bands = jp2f.component_count() as usize;
    let npix = rows * cols;

    let all_chunky = jp2f
        .read_all_components()
        .map_err(|e| RasterError::Other(format!("JPEG2000 decode error: {e}")))?;
    if all_chunky.len() != npix * bands {
        return Err(RasterError::CorruptData(format!(
            "JPEG2000 decoded sample count mismatch: expected {}, got {}",
            npix * bands,
            all_chunky.len()
        )));
    }

    let mut data = vec![0.0; npix * bands];
    for p in 0..npix {
        for b in 0..bands {
            data[b * npix + p] = all_chunky[p * bands + b] as f64;
        }
    }
    let data_type = map_data_type(jp2f.pixel_type())?;
    Ok((bands, data_type, data))
}

/// Write JPEG2000 / GeoJP2 to `path`.
pub fn write(raster: &Raster, path: &str) -> Result<()> {
    write_with_options(raster, path, &Jpeg2000WriteOptions::default())
}

/// Write JPEG2000 / GeoJP2 to `path` with typed options.
pub fn write_with_options(raster: &Raster, path: &str, opts: &Jpeg2000WriteOptions) -> Result<()> {
    let width = raster.cols as u32;
    let height = raster.rows as u32;
    let bands = raster.bands as u16;

    let compression = opts
        .compression
        .unwrap_or(Jpeg2000Compression::Lossy {
            quality_db: JPEG2000_DEFAULT_LOSSY_QUALITY_DB,
        })
        .to_core();

    let epsg = raster.crs.epsg.and_then(|v| u16::try_from(v).ok());
    let gt_xform = jp2::GeoTransform::north_up(
        raster.x_min,
        raster.cell_size_x,
        raster.y_max(),
        -raster.cell_size_y,
    );

    let mut writer = jp2::GeoJp2Writer::new(width, height, bands)
        .compression(compression)
        .geo_transform(gt_xform)
        .no_data(raster.nodata);

    if let Some(levels) = opts.decomp_levels {
        writer = writer.decomp_levels(levels);
    }
    if let Some(color_space) = opts.color_space {
        writer = writer.color_space(color_space);
    }
    if let Some(code) = epsg {
        writer = writer.epsg(code);
    }

    write_with_writer(writer, path, raster)
}

fn map_data_type(pixel_type: jp2::PixelType) -> Result<DataType> {
    match pixel_type {
        jp2::PixelType::Uint8 => Ok(DataType::U8),
        jp2::PixelType::Uint16 => Ok(DataType::U16),
        jp2::PixelType::Int16 => Ok(DataType::I16),
        jp2::PixelType::Int32 => Ok(DataType::I32),
        jp2::PixelType::Float32 => Ok(DataType::F32),
        jp2::PixelType::Float64 => Ok(DataType::F64),
    }
}

fn raster_to_chunky_u8(r: &Raster) -> Vec<u8> {
    let npix = r.rows * r.cols;
    let mut out = Vec::with_capacity(npix * r.bands);
    for p in 0..npix {
        let row = p / r.cols;
        let col = p % r.cols;
        for b in 0..r.bands {
            let v = r
                .get_raw(b as isize, row as isize, col as isize)
                .unwrap_or(r.nodata);
            out.push(v as u8);
        }
    }
    out
}

fn raster_to_chunky_u16(r: &Raster) -> Vec<u16> {
    let npix = r.rows * r.cols;
    let mut out = Vec::with_capacity(npix * r.bands);
    for p in 0..npix {
        let row = p / r.cols;
        let col = p % r.cols;
        for b in 0..r.bands {
            let v = r
                .get_raw(b as isize, row as isize, col as isize)
                .unwrap_or(r.nodata);
            out.push(v as u16);
        }
    }
    out
}

fn raster_to_chunky_i16(r: &Raster) -> Vec<i16> {
    let npix = r.rows * r.cols;
    let mut out = Vec::with_capacity(npix * r.bands);
    for p in 0..npix {
        let row = p / r.cols;
        let col = p % r.cols;
        for b in 0..r.bands {
            let v = r
                .get_raw(b as isize, row as isize, col as isize)
                .unwrap_or(r.nodata);
            out.push(v as i16);
        }
    }
    out
}

fn raster_to_chunky_f32(r: &Raster) -> Vec<f32> {
    let npix = r.rows * r.cols;
    let mut out = Vec::with_capacity(npix * r.bands);
    for p in 0..npix {
        let row = p / r.cols;
        let col = p % r.cols;
        for b in 0..r.bands {
            let v = r
                .get_raw(b as isize, row as isize, col as isize)
                .unwrap_or(r.nodata);
            out.push(v as f32);
        }
    }
    out
}

fn raster_to_chunky_f64(r: &Raster) -> Vec<f64> {
    let npix = r.rows * r.cols;
    let mut out = Vec::with_capacity(npix * r.bands);
    for p in 0..npix {
        let row = p / r.cols;
        let col = p % r.cols;
        for b in 0..r.bands {
            let v = r
                .get_raw(b as isize, row as isize, col as isize)
                .unwrap_or(r.nodata);
            out.push(v);
        }
    }
    out
}

fn write_with_writer(writer: jp2::GeoJp2Writer, path: &str, raster: &Raster) -> Result<()> {
    match raster.data_type {
        DataType::U8 => writer
            .write_u8(path, &raster_to_chunky_u8(raster))
            .map_err(|e| RasterError::Other(format!("JPEG2000 write error: {e}"))),
        DataType::U16 => writer
            .write_u16(path, &raster_to_chunky_u16(raster))
            .map_err(|e| RasterError::Other(format!("JPEG2000 write error: {e}"))),
        DataType::I16 => writer
            .write_i16(path, &raster_to_chunky_i16(raster))
            .map_err(|e| RasterError::Other(format!("JPEG2000 write error: {e}"))),
        DataType::F32 => writer
            .write_f32(path, &raster_to_chunky_f32(raster))
            .map_err(|e| RasterError::Other(format!("JPEG2000 write error: {e}"))),
        DataType::F64 => writer
            .write_f64(path, &raster_to_chunky_f64(raster))
            .map_err(|e| RasterError::Other(format!("JPEG2000 write error: {e}"))),
        _ => Err(RasterError::UnsupportedDataType(format!(
            "JPEG2000 writer does not currently support {} output",
            raster.data_type
        ))),
    }
}

#[cfg(all(test, feature = "jpeg2000-vendored-bridge"))]
mod adapter_read_path_tests {
    use super::*;

    #[test]
    fn a7_adapter_read_multiband_supported_fixture() {
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/sentinel_style_16x16_4band_lossless.jp2"
        );
        let ras = read(fixture).expect("A7: 4-band fixture should decode through adapter");

        assert_eq!(ras.bands, 4, "A7: expected 4 bands");
        assert_eq!(ras.rows, 16, "A7: expected 16 rows");
        assert_eq!(ras.cols, 16, "A7: expected 16 cols");
        assert_eq!(ras.data_type, DataType::U16, "A7: expected U16 output");

        // Expected pattern from fixture generation: base=(band+1)*1000 + row*16 + col.
        assert_eq!(ras.get(0, 0, 0) as u16, 1000, "A7 probe b0(0,0)");
        assert_eq!(ras.get(0, 15, 15) as u16, 1255, "A7 probe b0(15,15)");
        assert_eq!(ras.get(3, 0, 0) as u16, 4000, "A7 probe b3(0,0)");
        assert_eq!(ras.get(3, 15, 15) as u16, 4255, "A7 probe b3(15,15)");
    }

    #[test]
    fn a7_adapter_read_errors_when_bridge_fails_on_multiband() {
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/multiband_5ch_16x16_lossless.jp2"
        );
        let err = read(fixture).expect_err("A7: 5-band fixture should fail bridge path and refuse native multiband fallback");

        match err {
            RasterError::Other(msg) => {
                assert!(
                    msg.contains("vendored decode failed for multiband image"),
                    "A7: expected explicit multiband bridge-failure guard, got: {msg}"
                );
            }
            other => panic!("A7: expected RasterError::Other, got {other:?}"),
        }
    }
}

#[cfg(all(test, feature = "jpeg2000-vendored-bridge"))]
mod differential_tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::fmt::Write as _;

    #[derive(Default, Debug)]
    struct DiffSummary {
        fixtures_total: usize,
        multicomponent_fixtures: usize,
        ok: usize,
        native_error: usize,
        native_unsupported_packet_header_markers: usize,
        native_unsupported_poc: usize,
        bridge_error: usize,
        metadata_mismatch: usize,
        sample_count_mismatch: usize,
        sample_value_mismatch: usize,
        multicomponent_native_error: usize,
        multicomponent_metadata_mismatch: usize,
        multicomponent_sample_value_mismatch: usize,
    }

    fn parse_fixture_list(var: &str) -> Vec<String> {
        std::env::var(var)
            .ok()
            .map(|raw| {
                raw.split(['\n', ';'])
                    .flat_map(|s| s.split(','))
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    fn parse_fixture_file(var: &str) -> Vec<String> {
        let path = match std::env::var(var) {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };

        let content = match std::fs::read_to_string(&path) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Skipping fixture file {}: {}", path, e);
                return Vec::new();
            }
        };

        content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(ToOwned::to_owned)
            .collect()
    }

    fn parse_env_usize(var: &str) -> Option<usize> {
        std::env::var(var).ok().and_then(|v| v.parse::<usize>().ok())
    }

    fn json_escape(input: &str) -> String {
        let mut out = String::with_capacity(input.len() + 8);
        for ch in input.chars() {
            match ch {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                c if c.is_control() => {
                    let _ = write!(&mut out, "\\u{:04x}", c as u32);
                }
                c => out.push(c),
            }
        }
        out
    }

    fn first_mismatch_index(a: &[f64], b: &[f64], eps: f64) -> Option<usize> {
        a.iter()
            .zip(b.iter())
            .position(|(x, y)| (x - y).abs() > eps)
    }

    fn mismatch_position(idx: usize, cols: usize, rows: usize) -> (usize, usize, usize, usize) {
        let npix = rows.saturating_mul(cols).max(1);
        let band = idx / npix;
        let pixel = idx % npix;
        let row = pixel / cols.max(1);
        let col = pixel % cols.max(1);
        (band, row, col, pixel)
    }

    fn is_packet_header_marker_workflow_error(msg: &str) -> bool {
        msg.contains("PPM/PPT") || msg.contains("PLM/PLT/PPM/PPT")
    }

    #[test]
    fn jpeg2000_native_vs_bridge_differential_corpus() {
        let mut fixture_set: BTreeSet<String> = BTreeSet::new();
        fixture_set.extend(parse_fixture_list("JPEG2000_DIFF_FIXTURES"));
        fixture_set.extend(parse_fixture_file("JPEG2000_DIFF_FIXTURE_FILE"));
        let fixtures: Vec<String> = fixture_set.into_iter().collect();
        if fixtures.is_empty() {
            eprintln!(
                "Skipping differential corpus test: set JPEG2000_DIFF_FIXTURES or JPEG2000_DIFF_FIXTURE_FILE"
            );
            return;
        }

        let enforce = std::env::var("JPEG2000_DIFF_ENFORCE")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let eps = std::env::var("JPEG2000_DIFF_EPS")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0);

        let report_path = std::env::var("JPEG2000_DIFF_REPORT").ok();
        let max_native_error = parse_env_usize("JPEG2000_DIFF_MAX_NATIVE_ERROR");
        let max_bridge_error = parse_env_usize("JPEG2000_DIFF_MAX_BRIDGE_ERROR");
        let max_metadata_mismatch = parse_env_usize("JPEG2000_DIFF_MAX_METADATA_MISMATCH");
        let max_sample_count_mismatch = parse_env_usize("JPEG2000_DIFF_MAX_SAMPLE_COUNT_MISMATCH");
        let max_sample_value_mismatch = parse_env_usize("JPEG2000_DIFF_MAX_SAMPLE_VALUE_MISMATCH");
        let max_multicomponent_native_error = parse_env_usize("JPEG2000_DIFF_MAX_MULTICOMPONENT_NATIVE_ERROR");
        let max_multicomponent_metadata_mismatch = parse_env_usize("JPEG2000_DIFF_MAX_MULTICOMPONENT_METADATA_MISMATCH");
        let max_multicomponent_sample_value_mismatch = parse_env_usize("JPEG2000_DIFF_MAX_MULTICOMPONENT_SAMPLE_VALUE_MISMATCH");
        let min_ok = parse_env_usize("JPEG2000_DIFF_MIN_OK");
        let has_thresholds = max_native_error.is_some()
            || max_bridge_error.is_some()
            || max_metadata_mismatch.is_some()
            || max_sample_count_mismatch.is_some()
            || max_sample_value_mismatch.is_some()
            || max_multicomponent_native_error.is_some()
            || max_multicomponent_metadata_mismatch.is_some()
            || max_multicomponent_sample_value_mismatch.is_some()
            || min_ok.is_some();

        let mut summary = DiffSummary::default();
        let mut details: Vec<String> = Vec::new();

        for path in fixtures {
            summary.fixtures_total += 1;

            let jp2f = match jp2::GeoJp2::open(&path) {
                Ok(v) => v,
                Err(e) => {
                    summary.native_error += 1;
                    details.push(format!("NATIVE_OPEN_ERROR|{}|{}", path, e));
                    continue;
                }
            };

            let rows = jp2f.height() as usize;
            let cols = jp2f.width() as usize;
            let is_multicomponent = jp2f.component_count() > 1;
            if is_multicomponent {
                summary.multicomponent_fixtures += 1;
            }

            let native = decode_samples_with_internal_reader(&jp2f, rows, cols);
            let bridge = decode_samples_with_dj2k(&path, rows, cols);

            let (native_bands, native_dtype, native_data) = match native {
                Ok(v) => v,
                Err(e) => {
                    let msg = e.to_string();
                    summary.native_error += 1;
                    if is_multicomponent {
                        summary.multicomponent_native_error += 1;
                    }
                    if is_packet_header_marker_workflow_error(&msg) {
                        summary.native_unsupported_packet_header_markers += 1;
                    }
                    if msg.contains("POC") {
                        summary.native_unsupported_poc += 1;
                    }
                    details.push(format!("NATIVE_DECODE_ERROR|{}|{}", path, msg));
                    continue;
                }
            };

            let (bridge_bands, bridge_dtype, bridge_data) = match bridge {
                Ok(v) => v,
                Err(e) => {
                    summary.bridge_error += 1;
                    details.push(format!("BRIDGE_DECODE_ERROR|{}|{}", path, e));
                    continue;
                }
            };

            if native_bands != bridge_bands || native_dtype != bridge_dtype {
                summary.metadata_mismatch += 1;
                if is_multicomponent {
                    summary.multicomponent_metadata_mismatch += 1;
                }
                details.push(format!(
                    "METADATA_MISMATCH|{}|native=({}, {:?}) bridge=({}, {:?})",
                    path, native_bands, native_dtype, bridge_bands, bridge_dtype
                ));
                continue;
            }

            if native_data.len() != bridge_data.len() {
                summary.sample_count_mismatch += 1;
                details.push(format!(
                    "SAMPLE_COUNT_MISMATCH|{}|native={} bridge={}",
                    path,
                    native_data.len(),
                    bridge_data.len()
                ));
                continue;
            }

            if let Some(i) = first_mismatch_index(&native_data, &bridge_data, eps) {
                summary.sample_value_mismatch += 1;
                if is_multicomponent {
                    summary.multicomponent_sample_value_mismatch += 1;
                }
                let (band, row, col, pixel) = mismatch_position(i, cols, rows);
                let abs_err = (native_data[i] - bridge_data[i]).abs();
                details.push(format!(
                    "SAMPLE_VALUE_MISMATCH|{}|idx={} band={} row={} col={} pixel={} native={} bridge={} abs_err={} eps={}",
                    path, i, band, row, col, pixel, native_data[i], bridge_data[i], abs_err, eps
                ));
                continue;
            }

            summary.ok += 1;
        }

        eprintln!(
            "JPEG2000 differential summary: fixtures_total={} multicomponent_fixtures={} ok={} native_error={} native_unsupported_packet_header_markers={} native_unsupported_poc={} bridge_error={} metadata_mismatch={} sample_count_mismatch={} sample_value_mismatch={} multicomponent_native_error={} multicomponent_metadata_mismatch={} multicomponent_sample_value_mismatch={}",
            summary.fixtures_total,
            summary.multicomponent_fixtures,
            summary.ok,
            summary.native_error,
            summary.native_unsupported_packet_header_markers,
            summary.native_unsupported_poc,
            summary.bridge_error,
            summary.metadata_mismatch,
            summary.sample_count_mismatch,
            summary.sample_value_mismatch,
            summary.multicomponent_native_error,
            summary.multicomponent_metadata_mismatch,
            summary.multicomponent_sample_value_mismatch
        );
        for line in &details {
            eprintln!("{}", line);
        }

        if let Some(path) = report_path.as_ref() {
            let mut report = String::new();
            let _ = writeln!(&mut report, "{{");
            let _ = writeln!(&mut report, "  \"fixtures\": {},", summary.fixtures_total);
            let _ = writeln!(&mut report, "  \"multicomponent_fixtures\": {},", summary.multicomponent_fixtures);
            let _ = writeln!(&mut report, "  \"eps\": {},", eps);
            let _ = writeln!(&mut report, "  \"summary\": {{");
            let _ = writeln!(&mut report, "    \"ok\": {},", summary.ok);
            let _ = writeln!(&mut report, "    \"native_error\": {},", summary.native_error);
            let _ = writeln!(
                &mut report,
                "    \"native_unsupported_packet_header_markers\": {},",
                summary.native_unsupported_packet_header_markers
            );
            let _ = writeln!(
                &mut report,
                "    \"native_unsupported_poc\": {},",
                summary.native_unsupported_poc
            );
            let _ = writeln!(&mut report, "    \"bridge_error\": {},", summary.bridge_error);
            let _ = writeln!(&mut report, "    \"metadata_mismatch\": {},", summary.metadata_mismatch);
            let _ = writeln!(&mut report, "    \"sample_count_mismatch\": {},", summary.sample_count_mismatch);
            let _ = writeln!(&mut report, "    \"sample_value_mismatch\": {},", summary.sample_value_mismatch);
            let _ = writeln!(
                &mut report,
                "    \"multicomponent_native_error\": {},",
                summary.multicomponent_native_error
            );
            let _ = writeln!(
                &mut report,
                "    \"multicomponent_metadata_mismatch\": {},",
                summary.multicomponent_metadata_mismatch
            );
            let _ = writeln!(
                &mut report,
                "    \"multicomponent_sample_value_mismatch\": {}",
                summary.multicomponent_sample_value_mismatch
            );
            let _ = writeln!(&mut report, "  }},");
            let _ = writeln!(&mut report, "  \"details\": [");
            for (i, line) in details.iter().enumerate() {
                let comma = if i + 1 == details.len() { "" } else { "," };
                let _ = writeln!(&mut report, "    \"{}\"{}", json_escape(line), comma);
            }
            let _ = writeln!(&mut report, "  ]");
            let _ = writeln!(&mut report, "}}");

            if let Some(parent) = std::path::Path::new(path).parent() {
                if !parent.as_os_str().is_empty() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        eprintln!(
                            "Failed to create report directory {}: {}",
                            parent.display(),
                            e
                        );
                    }
                }
            }

            if let Err(e) = std::fs::write(path, report) {
                eprintln!("Failed to write JPEG2000 differential report to {}: {}", path, e);
            }
        }

        if enforce {
            let mismatches = summary.native_error
                + summary.bridge_error
                + summary.metadata_mismatch
                + summary.sample_count_mismatch
                + summary.sample_value_mismatch;

            if has_thresholds {
                if let Some(v) = max_native_error {
                    assert!(summary.native_error <= v, "native_error {} exceeds threshold {}", summary.native_error, v);
                }
                if let Some(v) = max_bridge_error {
                    assert!(summary.bridge_error <= v, "bridge_error {} exceeds threshold {}", summary.bridge_error, v);
                }
                if let Some(v) = max_metadata_mismatch {
                    assert!(summary.metadata_mismatch <= v, "metadata_mismatch {} exceeds threshold {}", summary.metadata_mismatch, v);
                }
                if let Some(v) = max_sample_count_mismatch {
                    assert!(summary.sample_count_mismatch <= v, "sample_count_mismatch {} exceeds threshold {}", summary.sample_count_mismatch, v);
                }
                if let Some(v) = max_sample_value_mismatch {
                    assert!(summary.sample_value_mismatch <= v, "sample_value_mismatch {} exceeds threshold {}", summary.sample_value_mismatch, v);
                }
                if let Some(v) = max_multicomponent_native_error {
                    assert!(
                        summary.multicomponent_native_error <= v,
                        "multicomponent_native_error {} exceeds threshold {}",
                        summary.multicomponent_native_error,
                        v
                    );
                }
                if let Some(v) = max_multicomponent_metadata_mismatch {
                    assert!(
                        summary.multicomponent_metadata_mismatch <= v,
                        "multicomponent_metadata_mismatch {} exceeds threshold {}",
                        summary.multicomponent_metadata_mismatch,
                        v
                    );
                }
                if let Some(v) = max_multicomponent_sample_value_mismatch {
                    assert!(
                        summary.multicomponent_sample_value_mismatch <= v,
                        "multicomponent_sample_value_mismatch {} exceeds threshold {}",
                        summary.multicomponent_sample_value_mismatch,
                        v
                    );
                }
                if let Some(v) = min_ok {
                    assert!(summary.ok >= v, "ok {} is below minimum threshold {}", summary.ok, v);
                }
            } else {
                assert_eq!(
                    mismatches, 0,
                    "Differential corpus enforcement failed with {} mismatches/errors",
                    mismatches
                );
            }
        }
    }
}


