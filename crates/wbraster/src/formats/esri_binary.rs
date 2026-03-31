//! Esri Binary Grid format.
//!
//! An Esri binary grid is a *directory* (workspace) named `<gridname>/` that
//! contains several files.  The files relevant to a single-band float raster:
//!
//! | File | Purpose |
//! |---|---|
//! | `hdr.adf` | Big-endian binary header (50 bytes) |
//! | `w001001.adf` | Float32 tile data (big-endian) |
//! | `w001001x.adf` | Tile index |
//! | `dblbnd.adf` | Double-precision extent |
//! | `prj.adf` | Projection WKT (text) |
//! | `sta.adf` | Statistics (8 × f64 big-endian) |
//!
//! This implementation handles the most common case: a single-band float32
//! grid stored as a single tile that spans the entire grid.  We write in the
//! same format so that ArcGIS / GDAL can read the output without a GDAL build.
//!
//! References:
//! - <https://gdal.org/drivers/raster/aig.html>
//! - Frank Warmerdam's reverse-engineering notes in GDAL source (aig*.c)

use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

use crate::error::{Result, RasterError};
use crate::io_utils::*;
use crate::raster::{DataType, Raster, RasterConfig};

// ─── Public API ───────────────────────────────────────────────────────────────

/// Read an Esri Binary Grid.
/// `path` may be the grid directory, the `hdr.adf` file, or `w001001.adf`.
pub fn read(path: &str) -> Result<Raster> {
    let grid_dir = resolve_grid_dir(path)?;
    read_from_dir(&grid_dir)
}

/// Write an Esri Binary Grid into a new (or overwritten) workspace directory.
/// `path` should be the desired grid directory name (e.g. `"mygrid"` or `"mygrid/"`).
pub fn write(raster: &Raster, path: &str) -> Result<()> {
    // Resolve the grid directory — strip trailing .adf if a file was given
    let dir = if path.ends_with(".adf") {
        Path::new(path).parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string())
    } else {
        path.trim_end_matches('/').to_string()
    };
    if raster.bands != 1 {
        return Err(RasterError::UnsupportedDataType(
            "Esri Binary Grid writer currently supports single-band rasters only".into(),
        ));
    }
    write_to_dir(raster, &dir)
}

// ─── Constants ────────────────────────────────────────────────────────────────

const HDR_ADF_MAGIC: [u8; 8] = [0x00, 0x00, 0x27, 0x0A, 0xFF, 0xFF, 0xFB, 0xF8];
const DATA_MAGIC: [u8; 8]   = [0x00, 0x00, 0x27, 0x0A, 0xFF, 0xFF, 0xFB, 0xF8];

// ─── Read ─────────────────────────────────────────────────────────────────────

fn resolve_grid_dir(path: &str) -> Result<String> {
    let p = Path::new(path);
    if p.is_dir() {
        return Ok(path.trim_end_matches('/').to_string());
    }
    if p.is_file() {
        // Strip the filename to get the parent dir
        return Ok(p.parent()
            .map(|d| d.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string()));
    }
    // Maybe the directory exists without trailing slash
    if Path::new(path).exists() {
        return Ok(path.to_string());
    }
    Err(RasterError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("grid directory not found: {path}"),
    )))
}

fn read_from_dir(dir: &str) -> Result<Raster> {
    // ── hdr.adf ───────────────────────────────────────────────────────────
    let hdr_path = format!("{dir}/hdr.adf");
    let hdr = read_hdr_adf(&hdr_path)?;

    // ── dblbnd.adf — double-precision extent ──────────────────────────────
    let (x_min, y_min, x_max, y_max) = read_dblbnd(dir)?;
    let cell_size_x = (x_max - x_min) / hdr.cols as f64;
    let cell_size_y = (y_max - y_min) / hdr.rows as f64;

    // ── prj.adf — projection ──────────────────────────────────────────────
    let crs = read_prj_adf(dir);

    // ── w001001.adf — tile data ───────────────────────────────────────────
    let tile_path = format!("{dir}/w001001.adf");
    let data = read_tile_data(&tile_path, hdr.cols, hdr.rows, hdr.nodata)?;

    let cfg = RasterConfig {
        cols: hdr.cols,
        rows: hdr.rows,
        x_min,
        y_min,
        cell_size: cell_size_x,
        cell_size_y: Some(cell_size_y),
        nodata: hdr.nodata,
        data_type: DataType::F32,
        crs: crs,        ..Default::default()
    };
    Raster::from_data(cfg, data)
}

// ── Header struct ─────────────────────────────────────────────────────────────

struct HdrAdf {
    cols: usize,
    rows: usize,
    nodata: f64,
}

fn read_hdr_adf(path: &str) -> Result<HdrAdf> {
    let buf = fs::read(path)?;
    if buf.len() < 50 {
        return Err(RasterError::CorruptData(
            format!("hdr.adf too short ({} bytes)", buf.len())
        ));
    }

    // Magic / version at offset 0
    // Offset 16: cols (i32 BE), offset 20: rows (i32 BE)
    // Offset 24: tile_x_size, offset 28: tile_y_size  (i32 BE, ignored for single-tile)
    // Offset 40: nodata (f64 BE) — only valid if flag set at offset 48
    let cols = read_i32_be(&buf, 16) as usize;
    let rows = read_i32_be(&buf, 20) as usize;

    // nodata: stored as f64 BE at byte 40; byte 48 is nodata-present flag
    let nodata_present = buf.get(48).copied().unwrap_or(0);
    let nodata = if nodata_present != 0 {
        read_f64_be(&buf, 40)
    } else {
        -9999.0
    };

    Ok(HdrAdf { cols, rows, nodata })
}

fn read_dblbnd(dir: &str) -> Result<(f64, f64, f64, f64)> {
    let path = format!("{dir}/dblbnd.adf");
    let buf = fs::read(&path)?;
    if buf.len() < 32 {
        return Err(RasterError::CorruptData(
            format!("dblbnd.adf too short ({} bytes)", buf.len())
        ));
    }
    let x_min = read_f64_be(&buf, 0);
    let y_min = read_f64_be(&buf, 8);
    let x_max = read_f64_be(&buf, 16);
    let y_max = read_f64_be(&buf, 24);
    Ok((x_min, y_min, x_max, y_max))
}

fn read_prj_adf(dir: &str) -> crate::crs_info::CrsInfo {
    use crate::crs_info::CrsInfo;
    let path = format!("{dir}/prj.adf");
    match fs::read_to_string(&path) {
        Ok(wkt) if !wkt.trim().is_empty() => CrsInfo::from_wkt(wkt.trim()),
        _ => CrsInfo::default(),
    }
}

fn read_tile_data(path: &str, cols: usize, rows: usize, nodata: f64) -> Result<Vec<f64>> {
    let mut file = BufReader::with_capacity(512 * 1024, File::open(path)?);
    // The tile data file begins with an 8-byte magic header, then the raw F32 data
    // is stored big-endian.  AIG tiles can have a more complex structure but the
    // common single-band float32 layout stores raw cells after an index.
    // We detect the layout by checking the file size.
    let expected_bytes = cols * rows * 4; // f32

    // Skip the file header magic (8 bytes) + optional tile index
    // We rely on file size to determine whether an index is present.
    let file_meta = fs::metadata(path)?;
    let file_size = file_meta.len() as usize;

    // Determine data offset: most real files either have 8-byte header or 28-byte header
    let data_offset = if file_size == expected_bytes { 0 }
    else if file_size == expected_bytes + 8 { 8 }
    else if file_size >= expected_bytes + 28 { 28 }
    else if file_size > expected_bytes { file_size - expected_bytes }
    else {
        return Err(RasterError::CorruptData(format!(
            "tile data file {path} is {} bytes; expected at least {expected_bytes}",
            file_size
        )));
    };

    file.seek(SeekFrom::Start(data_offset as u64))?;

    let n = cols * rows;
    let mut data = Vec::with_capacity(n);
    for _ in 0..n {
        let v = read_f32_be_stream(&mut file)? as f64;
        // The AIG nodata is typically ~1.175e-38 (the minimum positive f32)
        // Map it to the raster nodata
        let v = if (v - 1.175_494_e-38_f32 as f64).abs() < 1e-40 { nodata } else { v };
        data.push(v);
    }
    Ok(data)
}

// ─── Write ────────────────────────────────────────────────────────────────────

fn write_to_dir(raster: &Raster, dir: &str) -> Result<()> {
    fs::create_dir_all(dir)?;

    write_hdr_adf(raster, dir)?;
    write_dblbnd(raster, dir)?;
    write_tile_data(raster, dir)?;
    write_tile_index(raster, dir)?;
    write_sta_adf(raster, dir)?;
    if !raster.crs.is_unknown() {
        write_prj_adf(raster, dir)?;
    }
    Ok(())
}

fn write_hdr_adf(raster: &Raster, dir: &str) -> Result<()> {
    let path = format!("{dir}/hdr.adf");
    let mut w = BufWriter::new(File::create(&path)?);
    // Magic
    w.write_all(&HDR_ADF_MAGIC)?;
    // Bytes 8-15: padding
    w.write_all(&[0u8; 8])?;
    // Bytes 16-19: ncols (i32 BE)
    w.write_all(&(raster.cols as i32).to_be_bytes())?;
    // Bytes 20-23: nrows (i32 BE)
    w.write_all(&(raster.rows as i32).to_be_bytes())?;
    // Bytes 24-27: tile_x_size (same as cols for single-tile)
    w.write_all(&(raster.cols as i32).to_be_bytes())?;
    // Bytes 28-31: tile_y_size
    w.write_all(&(raster.rows as i32).to_be_bytes())?;
    // Bytes 32-39: pixel type (1 = float32), rest padding
    w.write_all(&[0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01])?;
    // Bytes 40-47: nodata value (f64 BE)
    w.write_all(&raster.nodata.to_be_bytes())?;
    // Byte 48: nodata present flag
    w.write_all(&[0x01])?;
    // Byte 49: padding
    w.write_all(&[0x00])?;
    w.flush()?;
    Ok(())
}

fn write_dblbnd(raster: &Raster, dir: &str) -> Result<()> {
    let path = format!("{dir}/dblbnd.adf");
    let mut w = BufWriter::new(File::create(&path)?);
    w.write_all(&raster.x_min.to_be_bytes())?;
    w.write_all(&raster.y_min.to_be_bytes())?;
    w.write_all(&raster.x_max().to_be_bytes())?;
    w.write_all(&raster.y_max().to_be_bytes())?;
    w.flush()?;
    Ok(())
}

fn write_tile_data(raster: &Raster, dir: &str) -> Result<()> {
    let path = format!("{dir}/w001001.adf");
    let mut w = BufWriter::with_capacity(512 * 1024, File::create(&path)?);
    // Write magic header
    w.write_all(&DATA_MAGIC)?;
    // Raw f32 big-endian cells
    for v in raster.data.iter_f64() {
        let f = v as f32;
        w.write_all(&f.to_be_bytes())?;
    }
    w.flush()?;
    Ok(())
}

fn write_tile_index(raster: &Raster, dir: &str) -> Result<()> {
    // w001001x.adf: simple 2-entry index (offset=8, size=cols*rows*4)
    let path = format!("{dir}/w001001x.adf");
    let mut w = BufWriter::new(File::create(&path)?);
    let offset: u32 = 8; // data starts after 8-byte magic
    let size: u32 = (raster.cols * raster.rows * 4) as u32;
    w.write_all(&offset.to_be_bytes())?;
    w.write_all(&size.to_be_bytes())?;
    w.flush()?;
    Ok(())
}

fn write_sta_adf(raster: &Raster, dir: &str) -> Result<()> {
    let path = format!("{dir}/sta.adf");
    let mut w = BufWriter::new(File::create(&path)?);
    let stats = raster.statistics();
    // 8 doubles big-endian: min, max, mean, std_dev, (reserved x4)
    w.write_all(&stats.min.to_be_bytes())?;
    w.write_all(&stats.max.to_be_bytes())?;
    w.write_all(&stats.mean.to_be_bytes())?;
    w.write_all(&stats.std_dev.to_be_bytes())?;
    for _ in 0..4 {
        w.write_all(&0.0_f64.to_be_bytes())?;
    }
    w.flush()?;
    Ok(())
}

fn write_prj_adf(raster: &Raster, dir: &str) -> Result<()> {
    if let Some(ref wkt) = raster.crs.wkt {
        let path = format!("{dir}/prj.adf");
        fs::write(&path, wkt.as_bytes())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raster::RasterConfig;
    use tempfile_helper::TempDir;

    // Minimal temp dir helper to avoid external deps
    mod tempfile_helper {
        use std::path::PathBuf;
        pub struct TempDir(pub PathBuf);
        impl TempDir {
            pub fn new() -> Self {
                use std::time::{SystemTime, UNIX_EPOCH};
                let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
                let p = std::env::temp_dir().join(format!("gis_raster_test_{ts}"));
                std::fs::create_dir_all(&p).unwrap();
                TempDir(p)
            }
            pub fn path(&self) -> &str { self.0.to_str().unwrap() }
        }
        impl Drop for TempDir {
            fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
        }
    }

    #[test]
    fn roundtrip_esri_binary() {
        let td = TempDir::new();
        let dir = format!("{}/testgrid", td.path());

        let cfg = RasterConfig {
            cols: 3, rows: 2,
            x_min: 0.0, y_min: 0.0,
            cell_size: 10.0, nodata: -9999.0,
            ..Default::default()
        };
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let r = Raster::from_data(cfg, data).unwrap();

        write_to_dir(&r, &dir).unwrap();
        let r2 = read_from_dir(&dir).unwrap();

        assert_eq!(r2.cols, 3);
        assert_eq!(r2.rows, 2);
        assert!((r2.get(0, 0, 0) - 1.0).abs() < 1e-4);
        assert!((r2.get(0, 1, 2) - 6.0).abs() < 1e-4);
    }
}
