//! Format registry and auto-detection.

pub mod esri_ascii;
pub mod esri_binary;
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
pub mod zarr;
pub(crate) mod geopackage_sqlite;
pub(crate) mod zarr_v3;
pub(crate) mod jpeg2000_core;

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
    /// Zarr v2 raster store (`.zarr` directory).
    Zarr,
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
            "hdr" => Ok(Self::Envi),
            "tif" | "tiff" => Ok(Self::GeoTiff),
            "gpkg" => Ok(Self::GeoPackage),
            "jp2" => Ok(Self::Jpeg2000),
            "zarr" => Ok(Self::Zarr),
            // ENVI data files are commonly .img or .dat or no extension — check
            // for a sidecar .hdr
            "img" | "dat" | "bin" | "raw" | "bil" | "bsq" | "bip" => {
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
            "tif" | "tiff" => Ok(Self::GeoTiff),
            "gpkg" => Ok(Self::GeoPackage),
            "jp2" => Ok(Self::Jpeg2000),
            "zarr" => Ok(Self::Zarr),
            "txt" => Ok(Self::GrassAscii),
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
            Self::Zarr => "Zarr v2",
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
            Self::Zarr       => zarr::read(path),
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
            Self::Zarr       => zarr::write(raster, path),
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
