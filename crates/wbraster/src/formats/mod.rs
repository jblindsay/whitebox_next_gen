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
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

const GEDI_ELEV_LOWESTMODE_PATH: &str = "/BEAM0000/elev_lowestmode";
const VIIRS_XDIM_PATH: &str = "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/XDim";

#[derive(Debug, Clone, PartialEq, Eq)]
struct HdfDatasetUri {
    container_path: String,
    dataset_path: String,
}

fn parse_hdf_dataset_uri(path: &str) -> Option<HdfDatasetUri> {
    let (container_path, raw_dataset_path) = if let Some((container, dataset)) = path.split_once("#dataset=") {
        (container, dataset)
    } else if let Some((container, dataset)) = path.split_once(":///") {
        // Legacy alias retained for backward compatibility.
        (container, dataset)
    } else {
        return None;
    };
    if container_path.is_empty() {
        return None;
    }

    let container_ext = extension_lower(container_path);
    let is_hdf_container = matches!(
        container_ext.as_str(),
        "h5" | "hdf5" | "he5" | "nc" | "hdf" | "h4"
    );
    if !is_hdf_container {
        return None;
    }

    let trimmed = raw_dataset_path.trim();
    if trimmed.is_empty() {
        return None;
    }
    let dataset_path = format!("/{}", trimmed.trim_start_matches('/'));

    Some(HdfDatasetUri {
        container_path: container_path.to_string(),
        dataset_path,
    })
}

pub(crate) fn is_hdf_dataset_uri(path: &str) -> bool {
    parse_hdf_dataset_uri(path).is_some()
}

pub(crate) fn read_hdf_dataset_uri(path: &str) -> Result<Raster> {
    let parsed = parse_hdf_dataset_uri(path).ok_or_else(|| {
        RasterError::UnknownFormat(
            "expected HDF dataset URI in canonical form 'container.h5#dataset=/absolute/dataset/path' (legacy alias 'container.h5:///absolute/dataset/path' is also accepted)".to_string(),
        )
    })?;

    let container_ext = extension_lower(&parsed.container_path);
    match container_ext.as_str() {
        "hdf" | "h4" => read_hdf4_raster_dataset_uri(&parsed),
        "h5" | "hdf5" | "he5" | "nc" => read_hdf5_raster_dataset_uri(path, &parsed),
        other => Err(RasterError::UnknownFormat(format!(
            "unsupported HDF container extension '.{other}' in dataset URI"
        ))),
    }
}

fn read_hdf5_raster_dataset_uri(path: &str, parsed: &HdfDatasetUri) -> Result<Raster> {
    let container_path = Path::new(&parsed.container_path);
    wbhdf::dataset::resolve_dataset_in_file(container_path, &parsed.dataset_path)
        .map_err(|err| RasterError::Other(format!("HDF5 dataset path resolution failed: {err}")))?;

    let materialization_scope = if parsed.dataset_path == GEDI_ELEV_LOWESTMODE_PATH {
        "gedi_l2b_contiguous_elev_lowestmode_v1"
    } else if parsed.dataset_path == VIIRS_XDIM_PATH {
        "viirs_xdim_contiguous_v1"
    } else {
        "generic_contiguous_hdf5_v1"
    };

    let resolved = resolve_hdf5_staged_contiguous_layout(
        container_path,
        &parsed.dataset_path,
        materialization_scope,
    );

    let resolved = match resolved {
        Ok(value) => value,
        Err(contiguous_err) => {
            return read_hdf5_raster_dataset_uri_from_chunked_single_leaf(
                path,
                parsed,
                contiguous_err,
            );
        }
    };

    let metadata = vec![
        ("hdf_container_path".to_string(), parsed.container_path.clone()),
        ("hdf_dataset_path".to_string(), parsed.dataset_path.clone()),
        (
            "hdf_materialization_scope".to_string(),
            resolved.materialization_scope.clone(),
        ),
    ];

    match resolved.bytes_per_value {
        4 => {
            let values = wbhdf::dataset::read_contiguous_f32_window_in_file(
                container_path,
                resolved.byte_offset,
                resolved.element_count,
                wbhdf::datatypes::Endianness::Little,
            )
            .map_err(|err| RasterError::Other(format!("HDF5 contiguous decode failed: {err}")))?;

            crate::raster::Raster::from_data_native(
                crate::raster::RasterConfig {
                    cols: resolved.cols,
                    rows: resolved.rows,
                    bands: 1,
                    x_min: 0.0,
                    y_min: 0.0,
                    cell_size: 1.0,
                    cell_size_y: Some(1.0),
                    nodata: -9999.0,
                    data_type: crate::raster::DataType::F32,
                    crs: crate::CrsInfo::default(),
                    metadata,
                },
                crate::raster::RasterData::F32(values),
            )
        }
        8 => {
            let values = wbhdf::dataset::read_contiguous_f64_window_in_file(
                container_path,
                resolved.byte_offset,
                resolved.element_count,
                wbhdf::datatypes::Endianness::Little,
            )
            .map_err(|err| RasterError::Other(format!("HDF5 contiguous decode failed: {err}")))?;

            crate::raster::Raster::from_data_native(
                crate::raster::RasterConfig {
                    cols: resolved.cols,
                    rows: resolved.rows,
                    bands: 1,
                    x_min: 0.0,
                    y_min: 0.0,
                    cell_size: 1.0,
                    cell_size_y: Some(1.0),
                    nodata: -9999.0,
                    data_type: crate::raster::DataType::F64,
                    crs: crate::CrsInfo::default(),
                    metadata,
                },
                crate::raster::RasterData::F64(values),
            )
        }
        _ => Err(RasterError::Other(format!(
            "HDF5 contiguous materialization currently supports only 4-byte and 8-byte scalar values for dataset URI '{}'",
            path
        ))),
    }
}

fn read_hdf5_raster_dataset_uri_from_chunked_single_leaf(
    path: &str,
    parsed: &HdfDatasetUri,
    contiguous_error: RasterError,
) -> Result<Raster> {
    let container_path = Path::new(&parsed.container_path);
    let resolved = resolve_hdf5_staged_chunked_single_leaf_layout(container_path, &parsed.dataset_path)
        .map_err(|chunked_err| {
            RasterError::Other(format!(
                "HDF5 raster materialization could not resolve supported layout for dataset URI '{}': contiguous_path_error='{}'; chunked_single_leaf_error='{}'",
                path, contiguous_error, chunked_err
            ))
        })?;

    let metadata = vec![
        ("hdf_container_path".to_string(), parsed.container_path.clone()),
        ("hdf_dataset_path".to_string(), parsed.dataset_path.clone()),
        (
            "hdf_materialization_scope".to_string(),
            resolved.materialization_scope.clone(),
        ),
    ];

    match resolved.data {
        Hdf5ChunkedDecodedData::F32(values) => crate::raster::Raster::from_data_native(
            crate::raster::RasterConfig {
                cols: resolved.cols,
                rows: resolved.rows,
                bands: 1,
                x_min: 0.0,
                y_min: 0.0,
                cell_size: 1.0,
                cell_size_y: Some(1.0),
                nodata: -9999.0,
                data_type: crate::raster::DataType::F32,
                crs: crate::CrsInfo::default(),
                metadata,
            },
            crate::raster::RasterData::F32(values),
        ),
        Hdf5ChunkedDecodedData::F64(values) => crate::raster::Raster::from_data_native(
            crate::raster::RasterConfig {
                cols: resolved.cols,
                rows: resolved.rows,
                bands: 1,
                x_min: 0.0,
                y_min: 0.0,
                cell_size: 1.0,
                cell_size_y: Some(1.0),
                nodata: -9999.0,
                data_type: crate::raster::DataType::F64,
                crs: crate::CrsInfo::default(),
                metadata,
            },
            crate::raster::RasterData::F64(values),
        ),
    }
}

fn read_hdf4_raster_dataset_uri(parsed: &HdfDatasetUri) -> Result<Raster> {
    let container_path = Path::new(&parsed.container_path);
    let summary = wbhdf::hdf4::probe_hdf4_eos_metadata_in_file(container_path)
        .map_err(|err| RasterError::Other(format!("HDF4 metadata probe failed: {err}")))?;
    let resolved = wbhdf::hdf4::resolve_hdf4_dataset_path(&summary, &parsed.dataset_path)
        .map_err(|err| RasterError::Other(format!("HDF4 dataset path resolution failed: {err}")))?;

    if resolved.shape.len() != 2 {
        return Err(RasterError::Other(format!(
            "HDF4 raster URI currently supports only 2D datasets; '{}' resolved to shape {:?}",
            parsed.dataset_path, resolved.shape
        )));
    }
    if resolved.data_type.as_deref() != Some("DFNT_INT16") {
        return Err(RasterError::UnsupportedDataType(format!(
            "HDF4 raster URI currently supports only DFNT_INT16 datasets; '{}' resolved to {:?}",
            parsed.dataset_path, resolved.data_type
        )));
    }

    let rows = resolved.shape[0];
    let cols = resolved.shape[1];
    let total_values = rows.checked_mul(cols).ok_or_else(|| {
        RasterError::Other(format!(
            "HDF4 raster dimensions overflow for '{}' with shape {:?}",
            parsed.dataset_path, resolved.shape
        ))
    })?;

    let data = wbhdf::hdf4::decode_hdf4_sds_i16_window_at_in_file(
        container_path,
        &parsed.dataset_path,
        0,
        total_values,
    )
    .map_err(|err| RasterError::Other(format!("HDF4 raster decode failed: {err}")))?;
    if data.len() != total_values {
        return Err(RasterError::Other(format!(
            "HDF4 raster decode returned {} values but {} were expected for '{}'",
            data.len(),
            total_values,
            parsed.dataset_path
        )));
    }

    let geometry = wbhdf::hdf4::derive_hdf4_grid_geometry_for_dataset(&summary, &parsed.dataset_path).ok();
    let (x_min, y_min, cell_size_x, cell_size_y) = if let Some(g) = geometry {
        (
            g.upper_left_mtrs.0,
            g.lower_right_mtrs.1,
            g.pixel_size_x.abs(),
            g.pixel_size_y.abs(),
        )
    } else {
        (0.0, 0.0, 1.0, 1.0)
    };

    let mut metadata = Vec::<(String, String)>::new();
    metadata.push(("hdf_container_path".to_string(), parsed.container_path.clone()));
    metadata.push(("hdf_dataset_path".to_string(), parsed.dataset_path.clone()));
    if let Some(projection) = resolved.projection {
        metadata.push(("hdf_projection".to_string(), projection));
    }

    crate::raster::Raster::from_data_native(
        crate::raster::RasterConfig {
            cols,
            rows,
            bands: 1,
            x_min,
            y_min,
            cell_size: cell_size_x,
            cell_size_y: Some(cell_size_y),
            nodata: -32768.0,
            data_type: crate::raster::DataType::I16,
            crs: crate::CrsInfo::default(),
            metadata,
        },
        crate::raster::RasterData::I16(data),
    )
}

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

#[derive(Debug, Clone)]
struct ResolvedHdf5ContiguousLayout {
    byte_offset: u64,
    bytes_per_value: usize,
    element_count: usize,
    rows: usize,
    cols: usize,
    materialization_scope: String,
}

#[derive(Debug, Clone)]
struct CandidateHdf5ContiguousLayout {
    byte_offset: u64,
    byte_len: u64,
    bytes_per_value: usize,
    rows: Option<usize>,
    cols: Option<usize>,
    score: usize,
    distance: usize,
    object_header_offset: usize,
}

#[derive(Debug, Clone)]
enum Hdf5ChunkedDecodedData {
    F32(Vec<f32>),
    F64(Vec<f64>),
}

#[derive(Debug, Clone)]
struct ResolvedHdf5ChunkedSingleLeafLayout {
    rows: usize,
    cols: usize,
    data: Hdf5ChunkedDecodedData,
    materialization_scope: String,
}

#[derive(Debug, Clone)]
struct CandidateHdf5ChunkedSingleLeafLayout {
    row_count: usize,
    col_count: usize,
    chunked_layout: wbhdf::object_header::ChunkedLayoutMessage,
    num_dimensions: usize,
    chunk_rows: usize,
    chunk_cols: usize,
    datatype_size: usize,
    uses_deflate: bool,
    distance: usize,
    score: usize,
}

fn resolve_hdf5_staged_contiguous_layout(
    container_path: &Path,
    dataset_path: &str,
    materialization_scope: &str,
) -> Result<ResolvedHdf5ContiguousLayout> {
        let bytes = fs::read(container_path)
            .map_err(|err| RasterError::Other(format!("HDF5 container read failed: {err}")))?;
        let marker_offsets = collect_marker_offsets_for_dataset_path(&bytes, dataset_path);

        let parsed = wbhdf::object_header::probe_file_object_headers(container_path)
            .map_err(|err| RasterError::Other(format!("HDF5 object-header probe failed: {err}")))?;

        let mut candidates = Vec::<CandidateHdf5ContiguousLayout>::new();
        for header in &parsed.v2_headers {
            let dimensions = header
                .dataspaces
                .first()
                .map(|dataspace| dataspace.dimensions.clone())
                .unwrap_or_default();
            let datatype_size = header.datatypes.first().map(|datatype| datatype.size as usize);

            for message in &header.messages {
                if let Some((byte_offset, byte_len)) = parse_v2_contiguous_layout_message(&bytes, message) {
                    let candidate = build_contiguous_candidate(
                        byte_offset,
                        byte_len,
                        &dimensions,
                        datatype_size,
                        &marker_offsets,
                        header.offset,
                    )?;
                    if let Some(candidate) = candidate {
                        candidates.push(candidate);
                    }
                }
            }

            for continuation in &header.continuations {
                if let Ok(chunk) = wbhdf::object_header::parse_continuation_chunk_in_file(container_path, continuation) {
                    for layout in &chunk.layouts {
                        if layout.layout_class != 1 || layout.data_size == 0 {
                            continue;
                        }

                        let candidate = build_contiguous_candidate(
                            layout.data_address,
                            layout.data_size,
                            &dimensions,
                            datatype_size,
                            &marker_offsets,
                            header.offset,
                        )?;
                        if let Some(candidate) = candidate {
                            candidates.push(candidate);
                        }
                    }
                }
            }
        }

        candidates.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then(left.distance.cmp(&right.distance))
                .then(left.object_header_offset.cmp(&right.object_header_offset))
        });

        let selected = candidates.into_iter().next().ok_or_else(|| {
            RasterError::Other(format!(
                "HDF5 raster materialization could not resolve contiguous layout metadata for dataset '{}'",
                dataset_path
            ))
        })?;

        let element_count = usize::try_from(selected.byte_len)
            .ok()
            .and_then(|byte_len| byte_len.checked_div(selected.bytes_per_value))
            .ok_or_else(|| {
                RasterError::Other(format!(
                    "HDF5 contiguous layout byte length overflow for dataset '{}': {}",
                    dataset_path, selected.byte_len
                ))
            })?;

        let (rows, cols) = if let (Some(rows), Some(cols)) = (selected.rows, selected.cols) {
            (rows, cols)
        } else {
            (1, element_count)
        };

        Ok(ResolvedHdf5ContiguousLayout {
            byte_offset: selected.byte_offset,
            bytes_per_value: selected.bytes_per_value,
            element_count,
            rows,
            cols,
            materialization_scope: materialization_scope.to_string(),
        })
}

fn build_contiguous_candidate(
        byte_offset: u64,
        byte_len: u64,
        dimensions: &[u64],
        datatype_size: Option<usize>,
        marker_offsets: &[usize],
        object_header_offset: usize,
    ) -> Result<Option<CandidateHdf5ContiguousLayout>> {
        if byte_len == 0 {
            return Ok(None);
        }

        let byte_len_usize = usize::try_from(byte_len).map_err(|_| {
            RasterError::Other(format!(
                "HDF5 contiguous layout byte length does not fit usize: {}",
                byte_len
            ))
        })?;
        let bytes_per_value = datatype_size.unwrap_or(0);
        if !matches!(bytes_per_value, 4 | 8) {
            return Ok(None);
        }

        if byte_len_usize % bytes_per_value != 0 {
            return Ok(None);
        }

        let inferred_element_count = byte_len_usize / bytes_per_value;
        let (rows, cols, dims_match) = match rows_cols_from_dimensions(dimensions)? {
            Some((rows, cols)) if rows.checked_mul(cols) == Some(inferred_element_count) => {
                (Some(rows), Some(cols), true)
            }
            Some(_) => (None, None, false),
            None => (None, None, false),
        };

        let distance = nearest_marker_distance(object_header_offset, marker_offsets);
        let mut score = 0usize;
        score += 8;
        if dims_match {
            score += 6;
        }
        if distance <= 16 * 1024 {
            score += 6;
        } else if distance <= 128 * 1024 {
            score += 4;
        } else if distance <= 512 * 1024 {
            score += 2;
        }

        Ok(Some(CandidateHdf5ContiguousLayout {
            byte_offset,
            byte_len,
            bytes_per_value,
            rows,
            cols,
            score,
            distance,
            object_header_offset,
        }))
}

fn rows_cols_from_dimensions(dimensions: &[u64]) -> Result<Option<(usize, usize)>> {
        if dimensions.is_empty() {
            return Ok(None);
        }

        if dimensions.len() == 1 {
            let cols = usize::try_from(dimensions[0]).map_err(|_| {
                RasterError::Other(format!(
                    "HDF5 dataspace dimension does not fit usize: {}",
                    dimensions[0]
                ))
            })?;
            return Ok(Some((1, cols)));
        }

        let rows = usize::try_from(dimensions[0]).map_err(|_| {
            RasterError::Other(format!(
                "HDF5 dataspace dimension does not fit usize: {}",
                dimensions[0]
            ))
        })?;
        let mut cols = 1usize;
        for dimension in &dimensions[1..] {
            let dim = usize::try_from(*dimension).map_err(|_| {
                RasterError::Other(format!(
                    "HDF5 dataspace dimension does not fit usize: {}",
                    dimension
                ))
            })?;
            cols = cols.checked_mul(dim).ok_or_else(|| {
                RasterError::Other("HDF5 dataspace column-product overflow".to_string())
            })?;
        }

        Ok(Some((rows, cols)))
}

fn parse_v2_contiguous_layout_message(
    bytes: &[u8],
    message: &wbhdf::object_header::ObjectHeaderMessageHeader,
) -> Option<(u64, u64)> {
        if message.type_id != 0x08 || message.size < 18 {
            return None;
        }
        let end = message.data_offset.checked_add(message.size as usize)?;
        if end > bytes.len() {
            return None;
        }

        let layout_class = bytes[message.data_offset + 1];
        if layout_class != 1 {
            return None;
        }

        let data_address = u64::from_le_bytes(
            bytes[message.data_offset + 2..message.data_offset + 10]
                .try_into()
                .ok()?,
        );
        let data_size = u64::from_le_bytes(
            bytes[message.data_offset + 10..message.data_offset + 18]
                .try_into()
                .ok()?,
        );
        Some((data_address, data_size))
}

fn collect_marker_offsets_for_dataset_path(bytes: &[u8], dataset_path: &str) -> Vec<usize> {
        let mut offsets = collect_ascii_marker_offsets(bytes, dataset_path);
        for component in dataset_path.split('/').filter(|component| !component.is_empty()) {
            offsets.extend(collect_ascii_marker_offsets(bytes, component));
        }
        offsets.sort_unstable();
        offsets.dedup();
        offsets
}

fn collect_ascii_marker_offsets(bytes: &[u8], marker: &str) -> Vec<usize> {
        let marker_bytes = marker.as_bytes();
        if marker_bytes.is_empty() || marker_bytes.len() > bytes.len() {
            return Vec::new();
        }

        bytes
            .windows(marker_bytes.len())
            .enumerate()
            .filter_map(|(offset, window)| (window == marker_bytes).then_some(offset))
            .collect()
}

fn nearest_marker_distance(anchor_offset: usize, markers: &[usize]) -> usize {
    markers
        .iter()
        .map(|offset| anchor_offset.abs_diff(*offset))
        .min()
        .unwrap_or(usize::MAX / 2)
}

fn resolve_hdf5_staged_chunked_single_leaf_layout(
    container_path: &Path,
    dataset_path: &str,
) -> Result<ResolvedHdf5ChunkedSingleLeafLayout> {
    let bytes = fs::read(container_path)
        .map_err(|err| RasterError::Other(format!("HDF5 container read failed: {err}")))?;
    let marker_offsets = collect_marker_offsets_for_dataset_path(&bytes, dataset_path);

    let headers = wbhdf::object_header::discover_v1_object_headers_in_file(container_path, 4096)
        .map_err(|err| RasterError::Other(format!("HDF5 v1 object-header discovery failed: {err}")))?;

    let mut candidates = Vec::<CandidateHdf5ChunkedSingleLeafLayout>::new();
    for header in headers {
        let Some(datatype_size) = header.datatypes.first().map(|datatype| datatype.size as usize) else {
            continue;
        };
        if !matches!(datatype_size, 4 | 8) {
            continue;
        }

        let Some((rows, cols)) = header
            .dataspaces
            .first()
            .and_then(|dataspace| rows_cols_from_dimensions(&dataspace.dimensions).ok().flatten())
        else {
            continue;
        };

        rows.checked_mul(cols).ok_or_else(|| {
            RasterError::Other(format!(
                "HDF5 chunked layout dimensions overflow for dataset '{}'",
                dataset_path
            ))
        })?;

        for chunked_layout in &header.chunked_layouts {
            if chunked_layout.layout_class != 2 {
                continue;
            }
            if chunked_layout.chunk_dimensions.is_empty() {
                continue;
            }

            let Some((chunk_rows, chunk_cols)) = rows_cols_from_chunk_dimensions(&chunked_layout.chunk_dimensions)? else {
                continue;
            };
            if chunk_rows == 0 || chunk_cols == 0 {
                continue;
            }
            if rows % chunk_rows != 0 || cols % chunk_cols != 0 {
                continue;
            }

            let uses_deflate = match header.filter_pipelines.first() {
                Some(pipeline) => {
                    if pipeline.filters.is_empty() {
                        false
                    } else if pipeline.filters.len() == 1 && pipeline.filters[0].id == 1 {
                        true
                    } else {
                        continue;
                    }
                }
                None => false,
            };

            let distance = nearest_marker_distance(header.offset, &marker_offsets);
            let mut score = 0usize;
            score += 8;
            if distance <= 16 * 1024 {
                score += 6;
            } else if distance <= 128 * 1024 {
                score += 4;
            } else if distance <= 512 * 1024 {
                score += 2;
            }

            candidates.push(CandidateHdf5ChunkedSingleLeafLayout {
                row_count: rows,
                col_count: cols,
                chunked_layout: chunked_layout.clone(),
                num_dimensions: chunked_layout.num_dimensions as usize,
                chunk_rows,
                chunk_cols,
                datatype_size,
                uses_deflate,
                distance,
                score,
            });
        }
    }

    candidates.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then(left.distance.cmp(&right.distance))
    });

    let candidate = candidates.into_iter().next().ok_or_else(|| {
        RasterError::Other(format!(
            "HDF5 chunked single-leaf layout candidate discovery failed for dataset '{}'",
            dataset_path
        ))
    })?;

    let expected_chunks = (candidate.row_count / candidate.chunk_rows)
        .checked_mul(candidate.col_count / candidate.chunk_cols)
        .ok_or_else(|| {
            RasterError::Other(format!(
                "HDF5 chunked expected-chunk-count overflow for dataset '{}'",
                dataset_path
            ))
        })?;

    let records = wbhdf::btree::read_chunked_storage_records_bounded_in_file(
        container_path,
        candidate.chunked_layout.index_address,
        candidate.num_dimensions,
        expected_chunks,
        expected_chunks,
    )
    .map_err(|err| RasterError::Other(format!("HDF5 chunked leaf decode failed: {err}")))?;
    if records.len() != expected_chunks {
        return Err(RasterError::Other(format!(
            "HDF5 chunked bounded leaf traversal currently requires exactly {} records for dataset '{}' but found {}",
            expected_chunks,
            dataset_path,
            records.len()
        )));
    }

    let data = if candidate.datatype_size == 4 {
        let total_values = candidate.row_count.checked_mul(candidate.col_count).ok_or_else(|| {
            RasterError::Other(format!(
                "HDF5 chunked assembled-size overflow for dataset '{}'",
                dataset_path
            ))
        })?;
        let mut assembled = vec![0.0_f32; total_values];
        for record in &records {
            let decoded = decode_chunk_record_f32(container_path, record, candidate.uses_deflate)
                .map_err(|err| RasterError::Other(format!("HDF5 chunked f32 decode failed: {err}")))?;
            let expected_chunk_values = candidate.chunk_rows.checked_mul(candidate.chunk_cols).ok_or_else(|| {
                RasterError::Other(format!(
                    "HDF5 chunked chunk-size overflow for dataset '{}'",
                    dataset_path
                ))
            })?;
            if decoded.len() != expected_chunk_values {
                return Err(RasterError::Other(format!(
                    "HDF5 chunked f32 decoded value count mismatch for dataset '{}': expected {}, found {}",
                    dataset_path,
                    expected_chunk_values,
                    decoded.len()
                )));
            }
            place_chunk_f32(
                &mut assembled,
                candidate.row_count,
                candidate.col_count,
                candidate.chunk_rows,
                candidate.chunk_cols,
                &record.chunk_offsets,
                &decoded,
                dataset_path,
            )?;
        }
        Hdf5ChunkedDecodedData::F32(assembled)
    } else {
        let total_values = candidate.row_count.checked_mul(candidate.col_count).ok_or_else(|| {
            RasterError::Other(format!(
                "HDF5 chunked assembled-size overflow for dataset '{}'",
                dataset_path
            ))
        })?;
        let mut assembled = vec![0.0_f64; total_values];
        for record in &records {
            let decoded = decode_chunk_record_f64(container_path, record, candidate.uses_deflate)
                .map_err(|err| RasterError::Other(format!("HDF5 chunked f64 decode failed: {err}")))?;
            let expected_chunk_values = candidate.chunk_rows.checked_mul(candidate.chunk_cols).ok_or_else(|| {
                RasterError::Other(format!(
                    "HDF5 chunked chunk-size overflow for dataset '{}'",
                    dataset_path
                ))
            })?;
            if decoded.len() != expected_chunk_values {
                return Err(RasterError::Other(format!(
                    "HDF5 chunked f64 decoded value count mismatch for dataset '{}': expected {}, found {}",
                    dataset_path,
                    expected_chunk_values,
                    decoded.len()
                )));
            }
            place_chunk_f64(
                &mut assembled,
                candidate.row_count,
                candidate.col_count,
                candidate.chunk_rows,
                candidate.chunk_cols,
                &record.chunk_offsets,
                &decoded,
                dataset_path,
            )?;
        }
        Hdf5ChunkedDecodedData::F64(assembled)
    };

    Ok(ResolvedHdf5ChunkedSingleLeafLayout {
        rows: candidate.row_count,
        cols: candidate.col_count,
        data,
        materialization_scope: "generic_chunked_single_leaf_hdf5_v1".to_string(),
    })
}

fn rows_cols_from_chunk_dimensions(dimensions: &[u32]) -> Result<Option<(usize, usize)>> {
    if dimensions.is_empty() {
        return Ok(None);
    }

    if dimensions.len() == 1 {
        let cols = usize::try_from(dimensions[0]).map_err(|_| {
            RasterError::Other(format!(
                "HDF5 chunk dimension does not fit usize: {}",
                dimensions[0]
            ))
        })?;
        return Ok(Some((1, cols)));
    }

    let rows = usize::try_from(dimensions[0]).map_err(|_| {
        RasterError::Other(format!(
            "HDF5 chunk dimension does not fit usize: {}",
            dimensions[0]
        ))
    })?;
    let mut cols = 1usize;
    for dimension in &dimensions[1..] {
        let dim = usize::try_from(*dimension).map_err(|_| {
            RasterError::Other(format!(
                "HDF5 chunk dimension does not fit usize: {}",
                dimension
            ))
        })?;
        cols = cols.checked_mul(dim).ok_or_else(|| {
            RasterError::Other("HDF5 chunk column-product overflow".to_string())
        })?;
    }
    Ok(Some((rows, cols)))
}

fn decode_chunk_record_f32(
    container_path: &Path,
    record: &wbhdf::btree::ChunkedStorageLeafRecord,
    uses_deflate: bool,
) -> Result<Vec<f32>> {
    let payload = wbhdf::btree::read_chunk_payload_in_file(
        container_path,
        record.chunk_address,
        record.chunk_size,
    )
    .map_err(|err| RasterError::Other(format!("HDF5 chunk payload read failed: {err}")))?;
    let decoded_bytes = if uses_deflate {
        wbhdf::filters::decompress_zlib(&payload)
            .map_err(|err| RasterError::Other(format!("HDF5 chunk zlib decode failed: {err}")))?
    } else {
        payload
    };
    wbhdf::datatypes::decode_f32_slice(&decoded_bytes, wbhdf::datatypes::Endianness::Little)
        .map_err(RasterError::Other)
}

fn decode_chunk_record_f64(
    container_path: &Path,
    record: &wbhdf::btree::ChunkedStorageLeafRecord,
    uses_deflate: bool,
) -> Result<Vec<f64>> {
    let payload = wbhdf::btree::read_chunk_payload_in_file(
        container_path,
        record.chunk_address,
        record.chunk_size,
    )
    .map_err(|err| RasterError::Other(format!("HDF5 chunk payload read failed: {err}")))?;
    let decoded_bytes = if uses_deflate {
        wbhdf::filters::decompress_zlib(&payload)
            .map_err(|err| RasterError::Other(format!("HDF5 chunk zlib decode failed: {err}")))?
    } else {
        payload
    };
    wbhdf::datatypes::decode_f64_slice(&decoded_bytes, wbhdf::datatypes::Endianness::Little)
        .map_err(RasterError::Other)
}

fn place_chunk_f32(
    assembled: &mut [f32],
    total_rows: usize,
    total_cols: usize,
    chunk_rows: usize,
    chunk_cols: usize,
    chunk_offsets: &[u64],
    decoded: &[f32],
    dataset_path: &str,
) -> Result<()> {
    if chunk_offsets.len() < 2 {
        return Err(RasterError::Other(format!(
            "HDF5 chunked chunk offsets require at least 2 dimensions for dataset '{}'",
            dataset_path
        )));
    }
    let row_offset = usize::try_from(chunk_offsets[0]).map_err(|_| {
        RasterError::Other(format!(
            "HDF5 chunk row offset does not fit usize for dataset '{}': {}",
            dataset_path, chunk_offsets[0]
        ))
    })?;
    let col_offset = usize::try_from(chunk_offsets[1]).map_err(|_| {
        RasterError::Other(format!(
            "HDF5 chunk col offset does not fit usize for dataset '{}': {}",
            dataset_path, chunk_offsets[1]
        ))
    })?;
    if row_offset + chunk_rows > total_rows || col_offset + chunk_cols > total_cols {
        return Err(RasterError::Other(format!(
            "HDF5 chunk placement exceeds raster bounds for dataset '{}'",
            dataset_path
        )));
    }
    for chunk_row in 0..chunk_rows {
        for chunk_col in 0..chunk_cols {
            let src_index = chunk_row * chunk_cols + chunk_col;
            let dst_index = (row_offset + chunk_row) * total_cols + (col_offset + chunk_col);
            assembled[dst_index] = decoded[src_index];
        }
    }
    Ok(())
}

fn place_chunk_f64(
    assembled: &mut [f64],
    total_rows: usize,
    total_cols: usize,
    chunk_rows: usize,
    chunk_cols: usize,
    chunk_offsets: &[u64],
    decoded: &[f64],
    dataset_path: &str,
) -> Result<()> {
    if chunk_offsets.len() < 2 {
        return Err(RasterError::Other(format!(
            "HDF5 chunked chunk offsets require at least 2 dimensions for dataset '{}'",
            dataset_path
        )));
    }
    let row_offset = usize::try_from(chunk_offsets[0]).map_err(|_| {
        RasterError::Other(format!(
            "HDF5 chunk row offset does not fit usize for dataset '{}': {}",
            dataset_path, chunk_offsets[0]
        ))
    })?;
    let col_offset = usize::try_from(chunk_offsets[1]).map_err(|_| {
        RasterError::Other(format!(
            "HDF5 chunk col offset does not fit usize for dataset '{}': {}",
            dataset_path, chunk_offsets[1]
        ))
    })?;
    if row_offset + chunk_rows > total_rows || col_offset + chunk_cols > total_cols {
        return Err(RasterError::Other(format!(
            "HDF5 chunk placement exceeds raster bounds for dataset '{}'",
            dataset_path
        )));
    }
    for chunk_row in 0..chunk_rows {
        for chunk_col in 0..chunk_cols {
            let src_index = chunk_row * chunk_cols + chunk_col;
            let dst_index = (row_offset + chunk_row) * total_cols + (col_offset + chunk_col);
            assembled[dst_index] = decoded[src_index];
        }
    }
    Ok(())
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
