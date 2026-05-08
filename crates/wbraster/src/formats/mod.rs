//! Format registry and auto-detection.

pub mod esri_ascii;
pub mod esri_binary;
pub mod esri_float;
pub mod grass_ascii;
pub mod surfer;
pub mod pcraster;
pub mod saga;
pub mod idrisi;
pub mod er_mapper;
pub mod envi;
pub mod geotiff;
pub mod geopackage;
pub mod jpeg2000;
pub mod png_jpeg;
pub mod zarr;
pub mod xyz;
pub mod dted;
pub mod hfa;
pub(crate) mod geopackage_sqlite;
pub(crate) mod zarr_v3;
pub(crate) mod jpeg2000_core;

#[cfg(test)]
mod jpeg2000_validation_tests;

use crate::error::{Result, RasterError};
use crate::raster::Raster;
use crate::io_utils::extension_lower;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};

/// Supported raster file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RasterFormat {
    /// Esri ASCII Grid (`.asc`, `.grd`).
    EsriAscii,
    /// Esri Binary Grid workspace (directory or `.adf` float grid file).
    EsriBinary,
    /// GRASS ASCII Raster (`.asc`, `.txt`).
    GrassAscii,
    /// Surfer GRD (`.grd`) — supports DSAA (ASCII) and DSRB (Surfer 7 binary).
    SurferGrd,
    /// PCRaster raster (`.map`) CSF format.
    Pcraster,
    /// SAGA GIS Binary Grid (`.sdat` / `.sgrd`).
    Saga,
    /// Idrisi/TerrSet Raster (`.rst` / `.rdc`).
    Idrisi,
    /// ER Mapper Raster (`.ers` / data file).
    ErMapper,
    /// ENVI HDR Labelled Raster (`.img`, `.dat`, `.bin` + `.hdr`).
    Envi,
    /// GeoTIFF / BigTIFF / COG (`.tif`, `.tiff`).
    GeoTiff,
    /// GeoPackage raster (`.gpkg`) phase 4.
    GeoPackage,
    /// JPEG 2000 / GeoJP2 (`.jp2`).
    Jpeg2000,
    /// JPEG image + world file (`.jpg`, `.jpeg` + `.jgw`/`.wld`).
    Jpeg,
    /// PNG image + world file (`.png` + `.pgw`/`.wld`).
    Png,
    /// Zarr v2 raster store (`.zarr` directory).
    Zarr,
    /// Esri Binary Float Grid (`.flt` + `.hdr`).
    EsriFloat,
    /// XYZ ASCII raster (`.xyz` — whitespace or comma-delimited X Y Z points).
    Xyz,
    /// DTED elevation tile (`.dt0`, `.dt1`, `.dt2`).
    Dted,
    /// ERDAS IMAGINE HFA raster (`.img`) — read-only.
    HfaImg,
}

impl RasterFormat {
    /// Attempt to detect the raster format from a file path.
    pub fn detect(path: &str) -> Result<Self> {
        let p = std::path::Path::new(path);
        if p.is_dir() {
            let hdr = p.join("hdr.adf");
            let data = p.join("w001001.adf");
            if hdr.exists() && data.exists() {
                return Ok(Self::EsriBinary);
            }
            if p.join(".zarray").exists() || p.join("zarr.json").exists() {
                return Ok(Self::Zarr);
            }
        }

        if p.is_file() && pcraster::is_pcraster_file(path) {
            return Ok(Self::Pcraster);
        }

        let ext = extension_lower(path);
        match ext.as_str() {
            "grd" => detect_grd(path),
            "map" => detect_map(path),
            "asc" | "txt" => detect_ascii_text(path),
            "adf" => Ok(Self::EsriBinary),
            "sgrd" | "sdat" => Ok(Self::Saga),
            "rdc" | "rst" => Ok(Self::Idrisi),
            "ers" => Ok(Self::ErMapper),
            "hdr" => detect_hdr(path),
            "flt" => Ok(Self::EsriFloat),
            "tif" | "tiff" => Ok(Self::GeoTiff),
            "gpkg" => Ok(Self::GeoPackage),
            "jp2" => Ok(Self::Jpeg2000),
            "jpg" | "jpeg" => Ok(Self::Jpeg),
            "png" => Ok(Self::Png),
            "zarr" => Ok(Self::Zarr),
            "xyz" => Ok(Self::Xyz),
            "dt0" | "dt1" | "dt2" => Ok(Self::Dted),
            // .img — could be ERDAS IMAGINE HFA or ENVI labelled.
            // Disambiguate by sniffing the HFA magic bytes first.
            "img" => detect_img(path),
            // Other ENVI data files: check for a sidecar .hdr
            "dat" | "bin" | "raw" | "bil" | "bsq" | "bip" => {
                let hdr = crate::io_utils::with_extension(path, "hdr");
                if std::path::Path::new(&hdr).exists() {
                    Ok(Self::Envi)
                } else {
                    Err(RasterError::UnknownFormat(format!(
                        ".{ext} — no matching .hdr sidecar found"
                    )))
                }
            }
            other => Err(RasterError::UnknownFormat(format!(".{other}"))),
        }
    }

    /// Infer output format strictly from the file extension for write targets.
    ///
    /// Unlike [`Self::detect`], this does not inspect existing file content and
    /// works for paths that do not exist yet.
    pub fn for_output_path(path: &str) -> Result<Self> {
        let ext = extension_lower(path);
        match ext.as_str() {
            "asc" => Ok(Self::EsriAscii),
            "grd" => Ok(Self::SurferGrd),
            "map" => Ok(Self::Pcraster),
            "sgrd" | "sdat" => Ok(Self::Saga),
            "rdc" | "rst" => Ok(Self::Idrisi),
            "ers" => Ok(Self::ErMapper),
            "hdr" => Ok(Self::Envi),
            "flt" => Ok(Self::EsriFloat),
            "tif" | "tiff" => Ok(Self::GeoTiff),
            "gpkg" => Ok(Self::GeoPackage),
            "jp2" => Ok(Self::Jpeg2000),
            "jpg" | "jpeg" => Ok(Self::Jpeg),
            "png" => Ok(Self::Png),
            "zarr" => Ok(Self::Zarr),
            "txt" => Ok(Self::GrassAscii),
            "xyz" => Ok(Self::Xyz),
            "dt0" | "dt1" | "dt2" => Ok(Self::Dted),
            "img" | "dat" | "bin" | "raw" | "bil" | "bsq" | "bip" => Ok(Self::Envi),
            "" => Err(RasterError::UnknownFormat(
                "missing file extension for output path".to_string(),
            )),
            other => Err(RasterError::UnknownFormat(format!(".{other}"))),
        }
    }

    /// Human-readable name of the format.
    pub fn name(&self) -> &'static str {
        match self {
            Self::EsriAscii => "Esri ASCII Grid",
            Self::EsriBinary => "Esri Binary Grid",
            Self::GrassAscii => "GRASS ASCII Raster",
            Self::SurferGrd => "Surfer GRD",
            Self::Pcraster => "PCRaster",
            Self::Saga => "SAGA GIS Binary Grid",
            Self::Idrisi => "Idrisi/TerrSet Raster",
            Self::ErMapper => "ER Mapper",
            Self::Envi => "ENVI HDR Labelled Raster",
            Self::GeoTiff => "GeoTIFF / BigTIFF / COG",
            Self::GeoPackage => "GeoPackage Raster (Phase 4)",
            Self::Jpeg2000 => "JPEG 2000 / GeoJP2",
            Self::Jpeg => "JPEG + World File",
            Self::Png => "PNG + World File",
            Self::Zarr => "Zarr v2",
            Self::EsriFloat => "Esri Float Grid",
            Self::Xyz => "XYZ ASCII Grid",
            Self::Dted => "DTED Elevation",
            Self::HfaImg => "ERDAS IMAGINE HFA",
        }
    }

    /// Read a raster from `path` using this format's reader.
    pub fn read(&self, path: &str) -> Result<Raster> {
        match self {
            Self::EsriAscii  => esri_ascii::read(path),
            Self::EsriBinary => esri_binary::read(path),
            Self::GrassAscii => grass_ascii::read(path),
            Self::SurferGrd  => surfer::read(path),
            Self::Pcraster   => pcraster::read(path),
            Self::Saga       => saga::read(path),
            Self::Idrisi     => idrisi::read(path),
            Self::ErMapper   => er_mapper::read(path),
            Self::Envi       => envi::read(path),
            Self::GeoTiff    => geotiff::read(path),
            Self::GeoPackage => geopackage::read(path),
            Self::Jpeg2000   => jpeg2000::read(path),
            Self::Jpeg       => png_jpeg::read_jpeg(path),
            Self::Png        => png_jpeg::read_png(path),
            Self::Zarr       => zarr::read(path),
            Self::EsriFloat  => esri_float::read(path),
            Self::Xyz        => xyz::read(path),
            Self::Dted       => dted::read(path),
            Self::HfaImg     => hfa::read(path),
        }
    }

    /// Write `raster` to `path` using this format's writer.
    pub fn write(&self, raster: &Raster, path: &str) -> Result<()> {
        match self {
            Self::EsriAscii  => esri_ascii::write(raster, path),
            Self::EsriBinary => esri_binary::write(raster, path),
            Self::GrassAscii => grass_ascii::write(raster, path),
            Self::SurferGrd  => surfer::write(raster, path),
            Self::Pcraster   => pcraster::write(raster, path),
            Self::Saga       => saga::write(raster, path),
            Self::Idrisi     => idrisi::write(raster, path),
            Self::ErMapper   => er_mapper::write(raster, path),
            Self::Envi       => envi::write(raster, path),
            Self::GeoTiff    => geotiff::write(raster, path),
            Self::GeoPackage => geopackage::write(raster, path),
            Self::Jpeg2000   => jpeg2000::write(raster, path),
            Self::Jpeg       => png_jpeg::write_jpeg(raster, path),
            Self::Png        => png_jpeg::write_png(raster, path),
            Self::Zarr       => zarr::write(raster, path),
            Self::EsriFloat  => esri_float::write(raster, path),
            Self::Xyz        => xyz::write(raster, path),
            Self::Dted       => dted::write(raster, path),
            Self::HfaImg     => Err(RasterError::UnsupportedDataType(
                "ERDAS IMAGINE HFA is read-only in this implementation; \
                 use GeoTIFF (.tif) for output".into(),
            )),
        }
    }
}

fn detect_grd(path: &str) -> Result<RasterFormat> {
    let mut f = File::open(path)?;
    let mut first4 = [0u8; 4];
    if f.read_exact(&mut first4).is_ok() {
        if &first4 == b"DSAA" {
            return Ok(RasterFormat::SurferGrd);
        }
        if i32::from_le_bytes(first4) == 0x4252_5344 {
            return Ok(RasterFormat::SurferGrd);
        }
    }
    Ok(RasterFormat::EsriAscii)
}

fn detect_ascii_text(path: &str) -> Result<RasterFormat> {
    let f = File::open(path)?;
    let reader = BufReader::new(f);
    let mut saw_esri = false;
    let mut saw_grass = false;

    for line in reader.lines().take(32) {
        let line = line?;
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if let Some((key, _)) = crate::io_utils::parse_key_value(t) {
            if matches!(key.as_str(), "ncols" | "nrows" | "xllcorner" | "xllcenter" | "yllcorner" | "yllcenter" | "cellsize" | "nodata_value") {
                saw_esri = true;
            }
        }
        if let Some((k, _)) = t.split_once(':') {
            let k = k.trim().to_ascii_lowercase();
            if matches!(k.as_str(), "north" | "south" | "east" | "west" | "rows" | "cols" | "null" | "type") {
                saw_grass = true;
            }
        }
    }

    if saw_grass && !saw_esri {
        Ok(RasterFormat::GrassAscii)
    } else {
        Ok(RasterFormat::EsriAscii)
    }
}

/// Disambiguate `.hdr` files: ENVI headers start with the token `ENVI` on the
/// first non-empty line; Esri Float Grid headers start with `ncols`.
fn detect_hdr(path: &str) -> Result<RasterFormat> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    if let Ok(file) = File::open(path) {
        for line_result in BufReader::new(file).lines() {
            let Ok(line) = line_result else { break };
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let first = trimmed.split_ascii_whitespace().next().unwrap_or("").to_ascii_uppercase();
            return match first.as_str() {
                "ENVI" => Ok(RasterFormat::Envi),
                _ => Ok(RasterFormat::EsriFloat),
            };
        }
    }
    // Fallback: assume ENVI for unknown .hdr
    Ok(RasterFormat::Envi)
}

/// Disambiguate `.img` files: ERDAS IMAGINE HFA files start with the magic
/// bytes `EHFA_HEADER_TAG\0`; everything else is assumed to be an ENVI
/// labelled raster (which requires a `.hdr` sidecar).
fn detect_img(path: &str) -> Result<RasterFormat> {
    use std::io::Read;
    const HFA_MAGIC_PREFIX: &[u8] = b"EHFA_HEADER_TAG";
    if let Ok(mut f) = File::open(path) {
        let mut magic = [0u8; 16];
        if f.read_exact(&mut magic).is_ok() && magic.starts_with(HFA_MAGIC_PREFIX) {
            return Ok(RasterFormat::HfaImg);
        }
    }
    // Fallback: look for an ENVI .hdr sidecar.
    let hdr = crate::io_utils::with_extension(path, "hdr");
    if std::path::Path::new(&hdr).exists() {
        Ok(RasterFormat::Envi)
    } else {
        Err(RasterError::UnknownFormat(
            ".img — not recognized as HFA (missing EHFA_HEADER_TAG) or ENVI (no .hdr sidecar)".into(),
        ))
    }
}

fn detect_map(path: &str) -> Result<RasterFormat> {
    if pcraster::is_pcraster_file(path) {
        Ok(RasterFormat::Pcraster)
    } else {
        Err(RasterError::UnknownFormat(
            ".map — not recognized as PCRaster CSF map".into(),
        ))
    }
}

impl std::fmt::Display for RasterFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}
