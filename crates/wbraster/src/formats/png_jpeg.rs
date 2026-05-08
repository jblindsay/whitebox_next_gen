//! PNG / JPEG raster format support with world-file and optional CRS sidecars.
//!
//! This module provides simple image-backed raster IO for:
//! - PNG (`.png`) with optional sidecar world files (`.pgw`, `.pngw`, `.wld`) and `.prj` WKT CRS
//! - JPEG (`.jpg`, `.jpeg`) with optional sidecar world files (`.jgw`, `.jpgw`, `.jpegw`, `.wld`) and `.prj` WKT CRS
//!
//! Scope and limitations:
//! - Read and write pixel data for common color models.
//! - Reads world files only for non-rotated georeferencing (`B=0`, `D=0`).
//! - Writes non-rotated world files.
//! - Reads optional `.prj` WKT sidecars for CRS metadata.
//! - Writes optional `.prj` sidecars if raster has WKT CRS.

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

use jpeg_decoder::PixelFormat;

use crate::crs_info::CrsInfo;
use crate::error::{RasterError, Result};
use crate::io_utils::{extension_lower, with_extension};
use crate::raster::{DataType, Raster, RasterConfig};

#[derive(Debug, Clone, Copy)]
struct WorldFile {
    a: f64,
    d: f64,
    b: f64,
    e: f64,
    c: f64,
    f: f64,
}

/// Read a PNG raster with optional world-file georeferencing and `.prj` CRS sidecar.
/// 
/// PNG files are read in RGB/RGBA format and treated as single-band rasters.
/// Georeferencing is optionally loaded from accompanying `.pgw`, `.pngw`, or `.wld` world files.
/// CRS information is loaded from accompanying `.prj` sidecar file if available.
pub fn read_png(path: &str) -> Result<Raster> {
    let file = File::open(path)?;
    let decoder = png::Decoder::new(BufReader::new(file));
    let mut reader = decoder.read_info().map_err(|e| RasterError::CorruptData(format!("PNG decode error: {e}")))?;
    let out_size = reader
        .output_buffer_size()
        .ok_or_else(|| RasterError::CorruptData("PNG decode error: unknown output buffer size".into()))?;
    let mut buf = vec![0u8; out_size];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| RasterError::CorruptData(format!("PNG decode error: {e}")))?;
    let bytes = &buf[..info.buffer_size()];

    let cols = info.width as usize;
    let rows = info.height as usize;
    let npix = cols * rows;

    let (bands, data_type, data) = match (info.color_type, info.bit_depth) {
        (png::ColorType::Grayscale, png::BitDepth::Eight) => {
            let mut data = vec![0.0; npix];
            for i in 0..npix {
                data[i] = bytes[i] as f64;
            }
            (1usize, DataType::U8, data)
        }
        (png::ColorType::GrayscaleAlpha, png::BitDepth::Eight) => {
            let mut data = vec![0.0; npix * 2];
            for p in 0..npix {
                data[p] = bytes[p * 2] as f64;
                data[npix + p] = bytes[p * 2 + 1] as f64;
            }
            (2usize, DataType::U8, data)
        }
        (png::ColorType::Rgb, png::BitDepth::Eight) => {
            let mut data = vec![0.0; npix * 3];
            for p in 0..npix {
                data[p] = bytes[p * 3] as f64;
                data[npix + p] = bytes[p * 3 + 1] as f64;
                data[npix * 2 + p] = bytes[p * 3 + 2] as f64;
            }
            (3usize, DataType::U8, data)
        }
        (png::ColorType::Rgba, png::BitDepth::Eight) => {
            let mut data = vec![0.0; npix * 4];
            for p in 0..npix {
                data[p] = bytes[p * 4] as f64;
                data[npix + p] = bytes[p * 4 + 1] as f64;
                data[npix * 2 + p] = bytes[p * 4 + 2] as f64;
                data[npix * 3 + p] = bytes[p * 4 + 3] as f64;
            }
            (4usize, DataType::U8, data)
        }
        (png::ColorType::Grayscale, png::BitDepth::Sixteen) => {
            let mut data = vec![0.0; npix];
            for i in 0..npix {
                let off = i * 2;
                data[i] = u16::from_be_bytes([bytes[off], bytes[off + 1]]) as f64;
            }
            (1usize, DataType::U16, data)
        }
        (png::ColorType::Rgb, png::BitDepth::Sixteen) => {
            let mut data = vec![0.0; npix * 3];
            for p in 0..npix {
                let off = p * 6;
                data[p] = u16::from_be_bytes([bytes[off], bytes[off + 1]]) as f64;
                data[npix + p] = u16::from_be_bytes([bytes[off + 2], bytes[off + 3]]) as f64;
                data[npix * 2 + p] = u16::from_be_bytes([bytes[off + 4], bytes[off + 5]]) as f64;
            }
            (3usize, DataType::U16, data)
        }
        (ct, bd) => {
            return Err(RasterError::UnsupportedDataType(format!(
                "PNG color/depth combination not supported: {:?} {:?}",
                ct, bd
            )));
        }
    };

    let mut cfg = config_from_world_file(path, cols, rows, bands, data_type)?;
    if let Some(crs) = read_prj_sidecar(path)? {
        cfg.crs = crs;
    }
    Raster::from_data(cfg, data)
}

/// Read a JPEG raster with optional world-file georeferencing and `.prj` CRS sidecar.
/// 
/// JPEG files are decoded to RGB or grayscale depending on the image format.
/// Georeferencing is optionally loaded from accompanying `.jgw`, `.jpgw`, `.jpegw`, or `.wld` world files.
/// CRS information is loaded from accompanying `.prj` sidecar file if available.
pub fn read_jpeg(path: &str) -> Result<Raster> {
    let file = File::open(path)?;
    let mut decoder = jpeg_decoder::Decoder::new(BufReader::new(file));
    let pixels = decoder
        .decode()
        .map_err(|e| RasterError::CorruptData(format!("JPEG decode error: {e}")))?;
    let info = decoder
        .info()
        .ok_or_else(|| RasterError::CorruptData("JPEG decode error: missing image info".into()))?;

    let cols = info.width as usize;
    let rows = info.height as usize;
    let npix = cols * rows;

    let (bands, data): (usize, Vec<f64>) = match info.pixel_format {
        PixelFormat::L8 => {
            let mut out = vec![0.0; npix];
            for p in 0..npix {
                out[p] = pixels[p] as f64;
            }
            (1, out)
        }
        PixelFormat::RGB24 => {
            let mut out = vec![0.0; npix * 3];
            for p in 0..npix {
                out[p] = pixels[p * 3] as f64;
                out[npix + p] = pixels[p * 3 + 1] as f64;
                out[npix * 2 + p] = pixels[p * 3 + 2] as f64;
            }
            (3, out)
        }
        PixelFormat::CMYK32 => {
            // Convert CMYK to RGB with a simple subtractive model.
            let mut out = vec![0.0; npix * 3];
            for p in 0..npix {
                let c = pixels[p * 4] as u16;
                let m = pixels[p * 4 + 1] as u16;
                let y = pixels[p * 4 + 2] as u16;
                let k = pixels[p * 4 + 3] as u16;
                let r = 255u16.saturating_sub((c + k).min(255)) as u8;
                let g = 255u16.saturating_sub((m + k).min(255)) as u8;
                let b = 255u16.saturating_sub((y + k).min(255)) as u8;
                out[p] = r as f64;
                out[npix + p] = g as f64;
                out[npix * 2 + p] = b as f64;
            }
            (3, out)
        }
        pf => {
            return Err(RasterError::UnsupportedDataType(format!(
                "JPEG pixel format not supported: {:?}",
                pf
            )));
        }
    };

    let mut cfg = config_from_world_file(path, cols, rows, bands, DataType::U8)?;
    if let Some(crs) = read_prj_sidecar(path)? {
        cfg.crs = crs;
    }
    Raster::from_data(cfg, data)
}

/// Write a raster as PNG and emit world-file / optional `.prj` sidecars.
pub fn write_png(raster: &Raster, path: &str) -> Result<()> {
    let cols = raster.cols;
    let rows = raster.rows;

    let (color, depth, bytes) = match raster.bands {
        1 => {
            if raster.data_type == DataType::U16 {
                (png::ColorType::Grayscale, png::BitDepth::Sixteen, raster_to_chunky_u16_be_bytes(raster, 1)?)
            } else {
                (png::ColorType::Grayscale, png::BitDepth::Eight, raster_to_chunky_u8(raster, 1)?)
            }
        }
        2 => (png::ColorType::GrayscaleAlpha, png::BitDepth::Eight, raster_to_chunky_u8(raster, 2)?),
        3 => {
            if raster.data_type == DataType::U16 {
                (png::ColorType::Rgb, png::BitDepth::Sixteen, raster_to_chunky_u16_be_bytes(raster, 3)?)
            } else {
                (png::ColorType::Rgb, png::BitDepth::Eight, raster_to_chunky_u8(raster, 3)?)
            }
        }
        4 => (png::ColorType::Rgba, png::BitDepth::Eight, raster_to_chunky_u8(raster, 4)?),
        b => {
            return Err(RasterError::UnsupportedDataType(format!(
                "PNG writer supports 1, 2, 3, or 4 bands; got {b}"
            )));
        }
    };

    let file = File::create(path)?;
    let w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, cols as u32, rows as u32);
    encoder.set_color(color);
    encoder.set_depth(depth);
    let mut writer = encoder
        .write_header()
        .map_err(|e| RasterError::Other(format!("PNG encode error: {e}")))?;
    writer
        .write_image_data(&bytes)
        .map_err(|e| RasterError::Other(format!("PNG encode error: {e}")))?;

    write_world_file(path, raster, "pgw")?;
    write_prj_sidecar(raster, path)
}

/// Write a raster as JPEG and emit world-file / optional `.prj` sidecars.
pub fn write_jpeg(raster: &Raster, path: &str) -> Result<()> {
    let cols = raster.cols;
    let rows = raster.rows;

    let (bytes, color) = match raster.bands {
        1 => (raster_to_chunky_u8(raster, 1)?, jpeg_encoder::ColorType::Luma),
        3 => (raster_to_chunky_u8(raster, 3)?, jpeg_encoder::ColorType::Rgb),
        b => {
            return Err(RasterError::UnsupportedDataType(format!(
                "JPEG writer supports 1 or 3 bands; got {b}"
            )));
        }
    };

    let quality = jpeg_quality_from_metadata(raster).unwrap_or(90);

    let file = File::create(path)?;
    let mut w = BufWriter::new(file);
    let encoder = jpeg_encoder::Encoder::new(&mut w, quality);
    encoder
        .encode(&bytes, cols as u16, rows as u16, color)
        .map_err(|e| RasterError::Other(format!("JPEG encode error: {e}")))?;

    write_world_file(path, raster, "jgw")?;
    write_prj_sidecar(raster, path)
}

fn config_from_world_file(
    image_path: &str,
    cols: usize,
    rows: usize,
    bands: usize,
    data_type: DataType,
) -> Result<RasterConfig> {
    let mut cfg = RasterConfig {
        cols,
        rows,
        bands,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: Some(-1.0),
        nodata: -9999.0,
        data_type,
        ..Default::default()
    };

    if let Some(wf) = read_world_file(image_path)? {
        if wf.a <= 0.0 {
            return Err(RasterError::CorruptData(
                "World file has non-positive X pixel size (A <= 0)".into(),
            ));
        }
        if wf.e == 0.0 {
            return Err(RasterError::CorruptData(
                "World file has zero Y pixel size (E == 0)".into(),
            ));
        }
        if wf.b.abs() > 1e-12 || wf.d.abs() > 1e-12 {
            return Err(RasterError::UnsupportedDataType(
                "World-file rotation/shear (B or D non-zero) is not supported".into(),
            ));
        }

        let x_min = wf.c - 0.5 * wf.a;
        let y_min = wf.f + wf.e * (rows as f64 - 0.5);

        cfg.x_min = x_min;
        cfg.y_min = y_min;
        cfg.cell_size = wf.a;
        cfg.cell_size_y = Some(wf.e);
    }

    Ok(cfg)
}

fn read_world_file(image_path: &str) -> Result<Option<WorldFile>> {
    for candidate in world_file_candidates(image_path) {
        let Ok(file) = File::open(&candidate) else { continue };
        let reader = BufReader::new(file);
        let mut vals: Vec<f64> = Vec::with_capacity(6);
        for line in reader.lines() {
            let line = line?;
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            let v = t.parse::<f64>().map_err(|_| RasterError::ParseError {
                field: "world file value".into(),
                value: t.to_string(),
                expected: "floating-point number".into(),
            })?;
            vals.push(v);
            if vals.len() == 6 {
                break;
            }
        }
        if vals.len() == 6 {
            return Ok(Some(WorldFile {
                a: vals[0],
                d: vals[1],
                b: vals[2],
                e: vals[3],
                c: vals[4],
                f: vals[5],
            }));
        }
    }
    Ok(None)
}

fn write_world_file(image_path: &str, raster: &Raster, preferred_ext: &str) -> Result<()> {
    let world_path = with_extension(image_path, preferred_ext);
    let a = raster.cell_size_x;
    let d = 0.0_f64;
    let b = 0.0_f64;
    let e = -raster.cell_size_y;
    let c = raster.x_min + 0.5 * a;
    let f = raster.y_max() - 0.5 * raster.cell_size_y;

    let mut w = BufWriter::new(File::create(world_path)?);
    writeln!(w, "{:.15}", a)?;
    writeln!(w, "{:.15}", d)?;
    writeln!(w, "{:.15}", b)?;
    writeln!(w, "{:.15}", e)?;
    writeln!(w, "{:.15}", c)?;
    writeln!(w, "{:.15}", f)?;
    Ok(())
}

fn read_prj_sidecar(image_path: &str) -> Result<Option<CrsInfo>> {
    let prj_path = with_extension(image_path, "prj");
    match std::fs::read_to_string(&prj_path) {
        Ok(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                Ok(Some(CrsInfo::from_wkt(trimmed)))
            }
        }
        Err(_) => Ok(None),
    }
}

fn write_prj_sidecar(raster: &Raster, image_path: &str) -> Result<()> {
    if let Some(ref wkt) = raster.crs.wkt {
        let trimmed = wkt.trim();
        if !trimmed.is_empty() {
            let prj_path = with_extension(image_path, "prj");
            std::fs::write(prj_path, trimmed)?;
        }
    }
    Ok(())
}

fn world_file_candidates(image_path: &str) -> Vec<String> {
    let ext = extension_lower(image_path);
    let mut out: Vec<String> = Vec::new();
    match ext.as_str() {
        "png" => {
            out.push(with_extension(image_path, "pgw"));
            out.push(with_extension(image_path, "pngw"));
            out.push(with_extension(image_path, "wld"));
        }
        "jpg" => {
            out.push(with_extension(image_path, "jgw"));
            out.push(with_extension(image_path, "jpgw"));
            out.push(with_extension(image_path, "wld"));
        }
        "jpeg" => {
            out.push(with_extension(image_path, "jgw"));
            out.push(with_extension(image_path, "jpegw"));
            out.push(with_extension(image_path, "wld"));
        }
        _ => {
            out.push(with_extension(image_path, "wld"));
        }
    }
    out
}

fn raster_to_chunky_u8(raster: &Raster, expected_bands: usize) -> Result<Vec<u8>> {
    if raster.bands != expected_bands {
        return Err(RasterError::UnsupportedDataType(format!(
            "Expected {} bands, got {}",
            expected_bands, raster.bands
        )));
    }

    let npix = raster.cols * raster.rows;
    let mut out = Vec::with_capacity(npix * raster.bands);
    for p in 0..npix {
        let row = (p / raster.cols) as isize;
        let col = (p % raster.cols) as isize;
        for b in 0..raster.bands {
            let v = raster.get_raw(b as isize, row, col).unwrap_or(raster.nodata);
            out.push(f64_to_u8(v));
        }
    }
    Ok(out)
}

fn raster_to_chunky_u16_be_bytes(raster: &Raster, expected_bands: usize) -> Result<Vec<u8>> {
    if raster.bands != expected_bands {
        return Err(RasterError::UnsupportedDataType(format!(
            "Expected {} bands, got {}",
            expected_bands, raster.bands
        )));
    }

    let npix = raster.cols * raster.rows;
    let mut out = Vec::with_capacity(npix * raster.bands * 2);
    for p in 0..npix {
        let row = (p / raster.cols) as isize;
        let col = (p % raster.cols) as isize;
        for b in 0..raster.bands {
            let v = raster.get_raw(b as isize, row, col).unwrap_or(raster.nodata);
            out.extend_from_slice(&f64_to_u16(v).to_be_bytes());
        }
    }
    Ok(out)
}

#[inline]
fn f64_to_u8(v: f64) -> u8 {
    if !v.is_finite() {
        return 0;
    }
    v.round().clamp(0.0, 255.0) as u8
}

#[inline]
fn f64_to_u16(v: f64) -> u16 {
    if !v.is_finite() {
        return 0;
    }
    v.round().clamp(0.0, 65535.0) as u16
}

fn jpeg_quality_from_metadata(raster: &Raster) -> Option<u8> {
    raster
        .metadata
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("jpeg_quality"))
        .and_then(|(_, v)| v.trim().parse::<u8>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn world_file_candidates_png() {
        let cands = world_file_candidates("foo/bar/image.png");
        assert_eq!(cands[0], "foo/bar/image.pgw");
        assert_eq!(cands[1], "foo/bar/image.pngw");
        assert_eq!(cands[2], "foo/bar/image.wld");
    }

    #[test]
    fn world_file_roundtrip_values() {
        let cfg = RasterConfig {
            cols: 10,
            rows: 20,
            bands: 1,
            x_min: 100.0,
            y_min: 200.0,
            cell_size: 2.0,
            cell_size_y: Some(-2.0),
            nodata: -9999.0,
            data_type: DataType::U8,
            ..Default::default()
        };
        let data = vec![1.0; 200];
        let raster = Raster::from_data(cfg, data).unwrap();

        let dir = tempdir().unwrap();
        let img_path = dir.path().join("test.png");
        let img_str = img_path.to_string_lossy().to_string();
        write_world_file(&img_str, &raster, "pgw").unwrap();

        let wf = read_world_file(&img_str).unwrap().unwrap();
        assert!((wf.a - 2.0).abs() < 1e-9);
        assert!((wf.e - -2.0).abs() < 1e-9);
        assert!((wf.c - 101.0).abs() < 1e-9);
        assert!((wf.f - 239.0).abs() < 1e-9);
    }

    #[test]
    fn prj_sidecar_roundtrip() {
        let wkt_epsg_4326 = "GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\",SPHEROID[\"WGS 84\",6378137,298.257223563]],PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]]";
        let mut cfg = RasterConfig {
            cols: 10,
            rows: 10,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            cell_size_y: Some(-1.0),
            nodata: -9999.0,
            data_type: DataType::U8,
            crs: CrsInfo::from_wkt(wkt_epsg_4326),
            ..Default::default()
        };
        let data = vec![1.0; 100];
        let raster = Raster::from_data(cfg, data).unwrap();

        let dir = tempdir().unwrap();
        let img_path = dir.path().join("test.png");
        let img_str = img_path.to_string_lossy().to_string();

        write_prj_sidecar(&raster, &img_str).unwrap();

        let prj_content = std::fs::read_to_string(with_extension(&img_str, "prj")).unwrap();
        assert_eq!(prj_content.trim(), wkt_epsg_4326);

        let crs = read_prj_sidecar(&img_str).unwrap();
        assert!(crs.is_some());
        let crs_info = crs.unwrap();
        assert_eq!(crs_info.wkt.as_deref(), Some(wkt_epsg_4326));
    }
}
