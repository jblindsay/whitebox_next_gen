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
use std::collections::BTreeSet;
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
            let values = match wbhdf::dataset::read_contiguous_f32_window_in_file(
                container_path,
                resolved.byte_offset,
                resolved.element_count,
                wbhdf::datatypes::Endianness::Little,
            ) {
                Ok(values) => values,
                Err(err) => {
                    return read_hdf5_raster_dataset_uri_from_chunked_single_leaf(
                        path,
                        parsed,
                        RasterError::Other(format!("HDF5 contiguous decode failed: {err}")),
                    );
                }
            };

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
            let values = match wbhdf::dataset::read_contiguous_f64_window_in_file(
                container_path,
                resolved.byte_offset,
                resolved.element_count,
                wbhdf::datatypes::Endianness::Little,
            ) {
                Ok(values) => values,
                Err(err) => {
                    return read_hdf5_raster_dataset_uri_from_chunked_single_leaf(
                        path,
                        parsed,
                        RasterError::Other(format!("HDF5 contiguous decode failed: {err}")),
                    );
                }
            };

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
    if let Ok(value) =
        resolve_hdf5_viirs_vnp21_bounded_layout(container_path, &parsed.dataset_path)
    {
        return materialize_hdf5_chunked_layout_to_raster(parsed, value);
    }

    let resolved = match resolve_hdf5_staged_chunked_single_leaf_layout(container_path, &parsed.dataset_path) {
        Ok(value) => value,
        Err(chunked_err) => {
            match resolve_hdf5_viirs_vnp21_bounded_layout(container_path, &parsed.dataset_path) {
                Ok(value) => value,
                Err(viirs_vnp21_err) => match resolve_hdf5_viirs_vnp13_bounded_layout(
                    container_path,
                    &parsed.dataset_path,
                ) {
                    Ok(value) => value,
                    Err(viirs_vnp13_err) => {
                        return Err(RasterError::Other(format!(
                            "HDF5 raster materialization could not resolve supported layout for dataset URI '{}': contiguous_path_error='{}'; chunked_single_leaf_error='{}'; viirs_vnp21_bounded_fallback_error='{}'; viirs_vnp13_bounded_fallback_error='{}'",
                            path, contiguous_error, chunked_err, viirs_vnp21_err, viirs_vnp13_err
                        )));
                    }
                },
            }
        }
    };

    materialize_hdf5_chunked_layout_to_raster(parsed, resolved)
}

fn materialize_hdf5_chunked_layout_to_raster(
    parsed: &HdfDatasetUri,
    resolved: ResolvedHdf5ChunkedSingleLeafLayout,
) -> Result<Raster> {

    let mut metadata = vec![
        ("hdf_container_path".to_string(), parsed.container_path.clone()),
        ("hdf_dataset_path".to_string(), parsed.dataset_path.clone()),
        (
            "hdf_materialization_scope".to_string(),
            resolved.materialization_scope.clone(),
        ),
    ];

    let georef_hint = derive_hdf5_georef_hint(
        Path::new(&parsed.container_path),
        &parsed.dataset_path,
    );
    if let Some(extra_metadata) = georef_hint
        .as_ref()
        .map(|hint| hint.metadata.clone())
    {
        metadata.extend(extra_metadata);
    }

    let x_min = georef_hint.as_ref().map(|hint| hint.x_min).unwrap_or(0.0);
    let y_min = georef_hint.as_ref().map(|hint| hint.y_min).unwrap_or(0.0);
    let cell_size = georef_hint
        .as_ref()
        .map(|hint| hint.cell_size)
        .unwrap_or(1.0);
    let cell_size_y = georef_hint
        .as_ref()
        .and_then(|hint| hint.cell_size_y)
        .or(Some(1.0));

    match resolved.data {
        Hdf5ChunkedDecodedData::F32(values) => crate::raster::Raster::from_data_native(
            crate::raster::RasterConfig {
                cols: resolved.cols,
                rows: resolved.rows,
                bands: 1,
                x_min,
                y_min,
                cell_size,
                cell_size_y,
                nodata: resolved.nodata,
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
                x_min,
                y_min,
                cell_size,
                cell_size_y,
                nodata: resolved.nodata,
                data_type: crate::raster::DataType::F64,
                crs: crate::CrsInfo::default(),
                metadata,
            },
            crate::raster::RasterData::F64(values),
        ),
        Hdf5ChunkedDecodedData::I16(values) => crate::raster::Raster::from_data_native(
            crate::raster::RasterConfig {
                cols: resolved.cols,
                rows: resolved.rows,
                bands: 1,
                x_min,
                y_min,
                cell_size,
                cell_size_y,
                nodata: resolved.nodata,
                data_type: crate::raster::DataType::I16,
                crs: crate::CrsInfo::default(),
                metadata,
            },
            crate::raster::RasterData::I16(values),
        ),
        Hdf5ChunkedDecodedData::U16(values) => crate::raster::Raster::from_data_native(
            crate::raster::RasterConfig {
                cols: resolved.cols,
                rows: resolved.rows,
                bands: 1,
                x_min,
                y_min,
                cell_size,
                cell_size_y,
                nodata: resolved.nodata,
                data_type: crate::raster::DataType::U16,
                crs: crate::CrsInfo::default(),
                metadata,
            },
            crate::raster::RasterData::U16(values),
        ),
        Hdf5ChunkedDecodedData::U8(values) => crate::raster::Raster::from_data_native(
            crate::raster::RasterConfig {
                cols: resolved.cols,
                rows: resolved.rows,
                bands: 1,
                x_min,
                y_min,
                cell_size,
                cell_size_y,
                nodata: resolved.nodata,
                data_type: crate::raster::DataType::U8,
                crs: crate::CrsInfo::default(),
                metadata,
            },
            crate::raster::RasterData::U8(values),
        ),
    }
}

#[derive(Debug, Clone)]
struct Hdf5GeorefHint {
    x_min: f64,
    y_min: f64,
    cell_size: f64,
    cell_size_y: Option<f64>,
    metadata: Vec<(String, String)>,
}

fn derive_hdf5_georef_hint(container_path: &Path, dataset_path: &str) -> Option<Hdf5GeorefHint> {
    if dataset_path.starts_with("/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/") {
        return derive_viirs_vnp13_grid_georef_hint(container_path);
    }

    if dataset_path.starts_with("/VIIRS_Swath_LSTE/") {
        return Some(Hdf5GeorefHint {
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            cell_size_y: Some(1.0),
            metadata: vec![
                ("hdf_georef_model".to_string(), "swath_geolocation".to_string()),
                (
                    "hdf_georef_latitude_path".to_string(),
                    "/VIIRS_Swath_LSTE/Geolocation Fields/latitude".to_string(),
                ),
                (
                    "hdf_georef_longitude_path".to_string(),
                    "/VIIRS_Swath_LSTE/Geolocation Fields/longitude".to_string(),
                ),
            ],
        });
    }

    None
}

fn derive_viirs_vnp13_grid_georef_hint(container_path: &Path) -> Option<Hdf5GeorefHint> {
    const XDIM_PATH: &str = "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/XDim";
    const YDIM_PATH: &str = "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/YDim";

    let x_layout = resolve_hdf5_staged_contiguous_layout(
        container_path,
        XDIM_PATH,
        "viirs_vnp13_xdim_georef_probe",
    )
    .ok()?;
    let y_layout = resolve_hdf5_staged_contiguous_layout(
        container_path,
        YDIM_PATH,
        "viirs_vnp13_ydim_georef_probe",
    )
    .ok()?;

    if x_layout.bytes_per_value != 8 || y_layout.bytes_per_value != 8 {
        return None;
    }

    let x_values = wbhdf::dataset::read_contiguous_f64_window_in_file(
        container_path,
        x_layout.byte_offset,
        x_layout.element_count,
        wbhdf::datatypes::Endianness::Little,
    )
    .ok()?;
    let y_values = wbhdf::dataset::read_contiguous_f64_window_in_file(
        container_path,
        y_layout.byte_offset,
        y_layout.element_count,
        wbhdf::datatypes::Endianness::Little,
    )
    .ok()?;

    if x_values.len() < 2 || y_values.len() < 2 {
        return None;
    }

    let dx = (x_values[1] - x_values[0]).abs();
    let dy = (y_values[1] - y_values[0]).abs();
    if dx == 0.0 || dy == 0.0 {
        return None;
    }

    let x_min_center = x_values
        .iter()
        .fold(f64::INFINITY, |acc, v| if *v < acc { *v } else { acc });
    let y_min_center = y_values
        .iter()
        .fold(f64::INFINITY, |acc, v| if *v < acc { *v } else { acc });

    Some(Hdf5GeorefHint {
        x_min: x_min_center - 0.5 * dx,
        y_min: y_min_center - 0.5 * dy,
        cell_size: dx,
        cell_size_y: Some(-dy),
        metadata: vec![
            ("hdf_georef_model".to_string(), "grid_affine_from_dim_arrays".to_string()),
            ("hdf_georef_xdim_path".to_string(), XDIM_PATH.to_string()),
            ("hdf_georef_ydim_path".to_string(), YDIM_PATH.to_string()),
        ],
    })
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
    I16(Vec<i16>),
    U16(Vec<u16>),
    U8(Vec<u8>),
}

#[derive(Debug, Clone)]
struct ResolvedHdf5ChunkedSingleLeafLayout {
    rows: usize,
    cols: usize,
    nodata: f64,
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
    filter_pipeline: Option<wbhdf::object_header::FilterPipelineMessage>,
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

    let headers = wbhdf::object_header::discover_v1_object_headers_in_file(container_path, 512)
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

            let filter_pipeline = match header.filter_pipelines.first() {
                Some(pipeline) => {
                    if is_supported_filter_pipeline(pipeline) {
                        Some(pipeline.clone())
                    } else {
                        continue;
                    }
                }
                None => None,
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
                filter_pipeline,
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
            let decoded = decode_chunk_record_f32(
                container_path,
                record,
                candidate.filter_pipeline.as_ref(),
            )
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
            let decoded = decode_chunk_record_f64(
                container_path,
                record,
                candidate.filter_pipeline.as_ref(),
            )
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
        nodata: -9999.0,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViirsFieldKind {
    F32,
    I16,
    U16,
    U8,
}

impl ViirsFieldKind {
    fn datatype_size(self) -> usize {
        match self {
            Self::F32 => 4,
            Self::I16 | Self::U16 => 2,
            Self::U8 => 1,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ViirsFieldCatalogEntry {
    dataset_path: &'static str,
    product: &'static str,
    kind: ViirsFieldKind,
    nodata: f64,
    preferred_chunk_index_address: Option<u64>,
}

const VIIRS_VNP21_FIELD_CATALOG: &[ViirsFieldCatalogEntry] = &[
    ViirsFieldCatalogEntry {
        dataset_path: "/VIIRS_Swath_LSTE/Geolocation Fields/latitude",
        product: "VNP21",
        kind: ViirsFieldKind::F32,
        nodata: -9999.0,
        preferred_chunk_index_address: Some(5_504_u64),
    },
    ViirsFieldCatalogEntry {
        dataset_path: "/VIIRS_Swath_LSTE/Geolocation Fields/longitude",
        product: "VNP21",
        kind: ViirsFieldKind::F32,
        nodata: -9999.0,
        preferred_chunk_index_address: Some(31_324_409_u64),
    },
    ViirsFieldCatalogEntry {
        dataset_path: "/VIIRS_Swath_LSTE/Data Fields/LST",
        product: "VNP21",
        kind: ViirsFieldKind::U16,
        nodata: 0.0,
        preferred_chunk_index_address: Some(65_387_786_u64),
    },
    ViirsFieldCatalogEntry {
        dataset_path: "/VIIRS_Swath_LSTE/Data Fields/LST_err",
        product: "VNP21",
        kind: ViirsFieldKind::U8,
        nodata: 0.0,
        preferred_chunk_index_address: Some(78_971_646_u64),
    },
    ViirsFieldCatalogEntry {
        dataset_path: "/VIIRS_Swath_LSTE/Data Fields/View_angle",
        product: "VNP21",
        kind: ViirsFieldKind::U8,
        nodata: 255.0,
        preferred_chunk_index_address: Some(88_316_419_u64),
    },
    ViirsFieldCatalogEntry {
        dataset_path: "/VIIRS_Swath_LSTE/Data Fields/Emis_ASTER",
        product: "VNP21",
        kind: ViirsFieldKind::U8,
        nodata: 0.0,
        preferred_chunk_index_address: Some(76_256_310_u64),
    },
    ViirsFieldCatalogEntry {
        dataset_path: "/VIIRS_Swath_LSTE/Data Fields/PWV",
        product: "VNP21",
        kind: ViirsFieldKind::U16,
        nodata: 0.0,
        preferred_chunk_index_address: Some(80_778_887_u64),
    },
    ViirsFieldCatalogEntry {
        dataset_path: "/VIIRS_Swath_LSTE/Data Fields/QC",
        product: "VNP21",
        kind: ViirsFieldKind::U16,
        nodata: 0.0,
        preferred_chunk_index_address: Some(70_375_762_u64),
    },
    ViirsFieldCatalogEntry {
        dataset_path: "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
        product: "VNP21",
        kind: ViirsFieldKind::U8,
        nodata: 255.0,
        preferred_chunk_index_address: Some(88_214_296_u64),
    },
    ViirsFieldCatalogEntry {
        dataset_path: "/VIIRS_Swath_LSTE/Data Fields/Emis_14",
        product: "VNP21",
        kind: ViirsFieldKind::U8,
        nodata: 0.0,
        preferred_chunk_index_address: Some(71_150_869_u64),
    },
    ViirsFieldCatalogEntry {
        dataset_path: "/VIIRS_Swath_LSTE/Data Fields/Emis_15",
        product: "VNP21",
        kind: ViirsFieldKind::U8,
        nodata: 0.0,
        preferred_chunk_index_address: Some(73_223_084_u64),
    },
    ViirsFieldCatalogEntry {
        dataset_path: "/VIIRS_Swath_LSTE/Data Fields/Emis_16",
        product: "VNP21",
        kind: ViirsFieldKind::U8,
        nodata: 0.0,
        preferred_chunk_index_address: Some(74_719_447_u64),
    },
    ViirsFieldCatalogEntry {
        dataset_path: "/VIIRS_Swath_LSTE/Data Fields/Emis_14_err",
        product: "VNP21",
        kind: ViirsFieldKind::U16,
        nodata: 0.0,
        preferred_chunk_index_address: Some(79_369_210_u64),
    },
    ViirsFieldCatalogEntry {
        dataset_path: "/VIIRS_Swath_LSTE/Data Fields/Emis_15_err",
        product: "VNP21",
        kind: ViirsFieldKind::U16,
        nodata: 0.0,
        preferred_chunk_index_address: Some(80_003_781_u64),
    },
    ViirsFieldCatalogEntry {
        dataset_path: "/VIIRS_Swath_LSTE/Data Fields/Emis_16_err",
        product: "VNP21",
        kind: ViirsFieldKind::U16,
        nodata: 0.0,
        preferred_chunk_index_address: Some(80_440_648_u64),
    },
];

const VIIRS_VNP13_FIELD_CATALOG: &[ViirsFieldCatalogEntry] = &[
    ViirsFieldCatalogEntry {
        dataset_path: "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days NDVI",
        product: "VNP13",
        kind: ViirsFieldKind::I16,
        nodata: -3000.0,
        preferred_chunk_index_address: Some(112_552_u64),
    },
    ViirsFieldCatalogEntry {
        dataset_path: "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days EVI",
        product: "VNP13",
        kind: ViirsFieldKind::I16,
        nodata: -3000.0,
        preferred_chunk_index_address: Some(115_168_u64),
    },
    ViirsFieldCatalogEntry {
        dataset_path: "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days EVI2",
        product: "VNP13",
        kind: ViirsFieldKind::I16,
        nodata: -3000.0,
        preferred_chunk_index_address: Some(117_784_u64),
    },
];

fn find_viirs_field_entry(
    dataset_path: &str,
    expected_product: &str,
) -> Option<&'static ViirsFieldCatalogEntry> {
    VIIRS_VNP21_FIELD_CATALOG
        .iter()
        .chain(VIIRS_VNP13_FIELD_CATALOG.iter())
        .find(|entry| entry.dataset_path == dataset_path && entry.product == expected_product)
}

#[derive(Debug, Clone)]
struct ResolvedViirsProfileLayout {
    chunk_index_address: u64,
    num_dimensions: usize,
    row_count: usize,
    row_width: usize,
    chunk_row_height: usize,
    chunk_dimensions: Vec<u32>,
}

fn resolve_hdf5_viirs_vnp21_bounded_layout(
    container_path: &Path,
    dataset_path: &str,
) -> Result<ResolvedHdf5ChunkedSingleLeafLayout> {
    let field = find_viirs_field_entry(dataset_path, "VNP21").ok_or_else(|| {
        RasterError::Other(format!(
            "dataset '{}' is outside VNP21 field catalog scope",
            dataset_path
        ))
    })?;

    resolve_hdf5_viirs_catalog_layout(container_path, field)
}

fn resolve_hdf5_viirs_catalog_layout(
    container_path: &Path,
    field: &ViirsFieldCatalogEntry,
) -> Result<ResolvedHdf5ChunkedSingleLeafLayout> {
    wbhdf::dataset::resolve_dataset_in_file(container_path, field.dataset_path)
        .map_err(|err| RasterError::Other(format!(
            "{} catalog dataset-path resolution failed for '{}': {err}",
            field.product, field.dataset_path
        )))?;

    let discovered = match discover_viirs_profile_layout(
        container_path,
        field.dataset_path,
        field.kind.datatype_size(),
        field.product,
    ) {
        Ok(layout) => layout,
        Err(err) => default_viirs_profile_layout_for_field(field).ok_or(err)?,
    };

    let row_width = discovered.row_width;
    let num_dimensions = discovered.num_dimensions;
    let chunk_index_address = field
        .preferred_chunk_index_address
        .unwrap_or(discovered.chunk_index_address);
    let initial_max_leaf_nodes = 512_usize;
    let initial_max_records = 8_192_usize;

    let records = wbhdf::btree::read_chunked_storage_records_bounded_in_file(
        container_path,
        chunk_index_address,
        num_dimensions,
        initial_max_leaf_nodes,
        initial_max_records,
    )
    .map_err(|err| RasterError::Other(format!(
        "{} catalog chunk-index traversal failed for '{}': {err}",
        field.product, field.dataset_path
    )))?;

    if records.is_empty() {
        return Err(RasterError::Other(format!(
            "{} catalog decode found no chunk records for '{}'",
            field.product, field.dataset_path
        )));
    }

    let row_dimension_index = if field.product == "VNP13" {
        infer_row_dimension_index_from_records(&records, num_dimensions, discovered.row_count)
            .unwrap_or(0)
    } else {
        infer_vnp21_row_dimension_index_from_records(&records, num_dimensions, row_width)
            .unwrap_or(0)
    };

    let chunk_row_height = discovered
        .chunk_dimensions
        .get(row_dimension_index)
        .and_then(|value| usize::try_from(*value).ok())
        .filter(|value| *value > 0)
        .unwrap_or(discovered.chunk_row_height);

    let max_row_offset = records
        .iter()
        .map(|record| {
            record
                .chunk_offsets
                .get(row_dimension_index)
                .copied()
                .unwrap_or(0)
        })
        .max()
        .unwrap_or(0);

    let row_count_from_offsets = usize::try_from(max_row_offset)
        .ok()
        .and_then(|max_row| max_row.checked_add(chunk_row_height))
        .ok_or_else(|| {
            RasterError::Other("VNP21 bounded fallback row-count overflow".to_string())
        })?;

    let row_count = discovered.row_count.max(row_count_from_offsets);

    let tuned_max_records = estimate_chunk_record_budget(
        row_count,
        row_width,
        &discovered.chunk_dimensions,
        row_dimension_index,
    )
    .unwrap_or(initial_max_records)
    .max(records.len())
    .clamp(2_400, 65_536);
    let tuned_max_leaf_nodes = (tuned_max_records / 32).clamp(64, 4_096);

    let data = match field.kind {
        ViirsFieldKind::F32 => {
            let values = wbhdf::dataset::decode_chunked_f32_row_major_window_in_file(
                container_path,
                field.dataset_path,
                chunk_index_address,
                num_dimensions,
                row_dimension_index,
                0,
                row_count,
                0,
                row_width,
                row_width,
                chunk_row_height,
                wbhdf::datatypes::Endianness::Little,
                tuned_max_leaf_nodes,
                tuned_max_records,
            )
            .map_err(|err| {
                RasterError::Other(format!(
                    "{} catalog f32 row-major decode failed for '{}': {err}",
                    field.product, field.dataset_path
                ))
            })?;
            Hdf5ChunkedDecodedData::F32(values)
        }
        ViirsFieldKind::I16 => {
            let values = wbhdf::dataset::decode_chunked_i16_row_major_window_in_file(
                container_path,
                field.dataset_path,
                chunk_index_address,
                num_dimensions,
                row_dimension_index,
                0,
                row_count,
                0,
                row_width,
                row_width,
                wbhdf::datatypes::Endianness::Little,
                tuned_max_leaf_nodes,
                tuned_max_records,
            )
            .map_err(|err| {
                RasterError::Other(format!(
                    "{} catalog i16 row-major decode failed for '{}': {err}",
                    field.product, field.dataset_path
                ))
            })?;
            Hdf5ChunkedDecodedData::I16(values)
        }
        ViirsFieldKind::U16 => {
            let values = wbhdf::dataset::decode_chunked_u16_row_major_window_in_file(
                container_path,
                field.dataset_path,
                chunk_index_address,
                num_dimensions,
                row_dimension_index,
                0,
                row_count,
                0,
                row_width,
                row_width,
                chunk_row_height,
                wbhdf::datatypes::Endianness::Little,
                tuned_max_leaf_nodes,
                tuned_max_records,
            )
            .map_err(|err| {
                RasterError::Other(format!(
                    "{} catalog u16 row-major decode failed for '{}': {err}",
                    field.product, field.dataset_path
                ))
            })?;
            Hdf5ChunkedDecodedData::U16(values)
        }
        ViirsFieldKind::U8 => {
            let values = wbhdf::dataset::decode_chunked_u8_row_major_window_in_file(
                container_path,
                field.dataset_path,
                chunk_index_address,
                num_dimensions,
                row_dimension_index,
                0,
                row_count,
                0,
                row_width,
                row_width,
                chunk_row_height,
                tuned_max_leaf_nodes,
                tuned_max_records,
            )
            .map_err(|err| {
                RasterError::Other(format!(
                    "{} catalog u8 row-major decode failed for '{}': {err}",
                    field.product, field.dataset_path
                ))
            })?;
            Hdf5ChunkedDecodedData::U8(values)
        }
    };

    Ok(ResolvedHdf5ChunkedSingleLeafLayout {
        rows: row_count,
        cols: row_width,
        nodata: field.nodata,
        data,
        materialization_scope: format!(
            "{}_catalog_chunked_multilevel_hdf5_v1",
            field.product.to_lowercase()
        ),
    })
}

fn default_viirs_profile_layout_for_field(
    field: &ViirsFieldCatalogEntry,
) -> Option<ResolvedViirsProfileLayout> {
    let chunk_index_address = field.preferred_chunk_index_address?;
    if field.product == "VNP13" {
        return Some(ResolvedViirsProfileLayout {
            chunk_index_address,
            num_dimensions: 3,
            row_count: 2_400,
            row_width: 2_400,
            chunk_row_height: 1,
            chunk_dimensions: vec![1, 2_400, 2],
        });
    }

    Some(ResolvedViirsProfileLayout {
        chunk_index_address,
        num_dimensions: 3,
        row_count: 0,
        row_width: 3_200,
        chunk_row_height: 16,
        chunk_dimensions: vec![16, 3_200, 2],
    })
}

fn discovered_layout_has_expected_chunk_payload_shape(
    container_path: &Path,
    layout: &ResolvedViirsProfileLayout,
    expected_datatype_size: usize,
) -> bool {
    let records = match wbhdf::btree::read_chunked_storage_records_bounded_in_file(
        container_path,
        layout.chunk_index_address,
        layout.num_dimensions,
        64,
        2_048,
    ) {
        Ok(records) => records,
        Err(_) => return false,
    };

    let Some(record) = records.iter().find(|record| record.chunk_size > 0) else {
        return false;
    };

    let payload = match wbhdf::btree::read_chunk_payload_in_file(
        container_path,
        record.chunk_address,
        record.chunk_size,
    ) {
        Ok(payload) => payload,
        Err(_) => return false,
    };

    // VIIRS profile fields are expected to be either deflate-compressed or raw.
    let decoded = wbhdf::filters::decompress_zlib(&payload).unwrap_or(payload);

    let Some(chunk_rows_raw) = layout.chunk_dimensions.first().copied() else {
        return false;
    };
    let Ok(chunk_rows) = usize::try_from(chunk_rows_raw) else {
        return false;
    };
    let Some(min_len) = chunk_rows
        .checked_mul(layout.row_width)
        .and_then(|len| len.checked_mul(expected_datatype_size))
    else {
        return false;
    };

    decoded.len() >= min_len
}

fn discover_viirs_profile_layout(
    container_path: &Path,
    dataset_path: &str,
    expected_datatype_size: usize,
    profile_name: &str,
) -> Result<ResolvedViirsProfileLayout> {
    let bytes = fs::read(container_path)
        .map_err(|err| RasterError::Other(format!("HDF5 container read failed: {err}")))?;
    let marker_offsets = collect_marker_offsets_for_dataset_path(&bytes, dataset_path);
    let full_path_offsets = collect_ascii_marker_offsets(&bytes, dataset_path);
    let tail_component = dataset_path
        .rsplit('/')
        .find(|component| !component.is_empty())
        .ok_or_else(|| {
            RasterError::Other(format!(
                "{} bounded fallback dataset path has no terminal component: '{}'",
                profile_name, dataset_path
            ))
        })?;
    let tail_marker_offsets = collect_ascii_marker_offsets(&bytes, tail_component);

    let headers = wbhdf::object_header::discover_v1_object_headers_in_file(container_path, 8_192)
        .map_err(|err| {
            RasterError::Other(format!(
                "{} bounded fallback v1 object-header discovery failed: {err}",
                profile_name
            ))
        })?;

    let mut candidates = Vec::<(usize, usize, usize, usize, ResolvedViirsProfileLayout)>::new();
    for header in headers {
        let datatype_size = header
            .datatypes
            .first()
            .map(|datatype| datatype.size as usize);
        let datatype_matches = datatype_size == Some(expected_datatype_size);

        let Some((rows, cols)) = header
            .dataspaces
            .first()
            .and_then(|dataspace| rows_cols_from_dimensions(&dataspace.dimensions).ok().flatten())
        else {
            continue;
        };

        for chunked_layout in &header.chunked_layouts {
            if chunked_layout.layout_class != 2 || chunked_layout.index_address == 0 {
                continue;
            }
            if chunked_layout.num_dimensions < 2 || chunked_layout.chunk_dimensions.is_empty() {
                continue;
            }

            let chunk_row_height = usize::try_from(chunked_layout.chunk_dimensions[0]).map_err(|_| {
                RasterError::Other(format!(
                    "{} bounded fallback chunk-row-height does not fit usize for dataset '{}'",
                    profile_name, dataset_path
                ))
            })?;
            let Some(&chunk_col_width_raw) = chunked_layout.chunk_dimensions.get(1) else {
                continue;
            };
            let chunk_col_width = usize::try_from(chunk_col_width_raw).map_err(|_| {
                RasterError::Other(format!(
                    "{} bounded fallback chunk-col-width does not fit usize for dataset '{}'",
                    profile_name, dataset_path
                ))
            })?;
            if chunk_row_height == 0 || cols == 0 {
                continue;
            }
            if chunk_col_width != cols {
                continue;
            }

            let distance = nearest_marker_distance(header.offset, &marker_offsets);
            let full_path_distance = nearest_marker_distance(header.offset, &full_path_offsets);
            let tail_distance = nearest_marker_distance(header.offset, &tail_marker_offsets);
            let mut score = 0usize;
            score += 8;
            if datatype_matches {
                score += 8;
            }
            if full_path_distance <= 16 * 1024 {
                score += 16;
            } else if full_path_distance <= 128 * 1024 {
                score += 12;
            } else if full_path_distance <= 512 * 1024 {
                score += 8;
            } else if full_path_distance <= 2 * 1024 * 1024 {
                score += 4;
            }
            if tail_distance <= 16 * 1024 {
                score += 10;
            } else if tail_distance <= 128 * 1024 {
                score += 7;
            } else if tail_distance <= 512 * 1024 {
                score += 4;
            } else if tail_distance <= 2 * 1024 * 1024 {
                score += 2;
            }
            if distance <= 16 * 1024 {
                score += 6;
            } else if distance <= 128 * 1024 {
                score += 4;
            } else if distance <= 512 * 1024 {
                score += 2;
            }

            candidates.push((
                score,
                full_path_distance,
                tail_distance,
                distance,
                ResolvedViirsProfileLayout {
                    chunk_index_address: chunked_layout.index_address,
                    num_dimensions: chunked_layout.num_dimensions as usize,
                    row_count: rows,
                    row_width: cols,
                    chunk_row_height,
                    chunk_dimensions: chunked_layout.chunk_dimensions.clone(),
                },
            ));
        }
    }

    candidates.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then(left.1.cmp(&right.1))
            .then(left.2.cmp(&right.2))
            .then(left.3.cmp(&right.3))
    });

    for (_, _, _, _, layout) in candidates {
        if discovered_layout_has_expected_chunk_payload_shape(
            container_path,
            &layout,
            expected_datatype_size,
        ) {
            return Ok(layout);
        }
    }

    Err(RasterError::Other(format!(
        "{} bounded fallback could not derive chunk-layout metadata for dataset '{}'",
        profile_name, dataset_path
    )))
}

fn infer_vnp21_row_dimension_index_from_records(
    records: &[wbhdf::btree::ChunkedStorageLeafRecord],
    num_dimensions: usize,
    row_width: usize,
) -> Option<usize> {
    let row_width_u64 = u64::try_from(row_width).ok()?;
    let mut best: Option<(usize, u64, usize)> = None;

    for dim in 0..num_dimensions {
        let mut offsets = BTreeSet::new();
        let mut max_offset = 0_u64;
        let mut valid = true;

        for record in records {
            let Some(offset) = record.chunk_offsets.get(dim).copied() else {
                valid = false;
                break;
            };
            offsets.insert(offset);
            max_offset = max_offset.max(offset);
        }

        if !valid || offsets.is_empty() {
            continue;
        }
        if max_offset >= row_width_u64 {
            continue;
        }

        let distinct = offsets.len();
        if distinct <= 1 && max_offset == 0 {
            continue;
        }
        match best {
            None => best = Some((dim, max_offset, distinct)),
            Some((_, best_max, best_distinct)) => {
                if distinct > best_distinct
                    || (distinct == best_distinct && max_offset < best_max)
                {
                    best = Some((dim, max_offset, distinct));
                }
            }
        }
    }

    best.map(|(dim, _, _)| dim)
}

fn resolve_hdf5_viirs_vnp13_bounded_layout(
    container_path: &Path,
    dataset_path: &str,
) -> Result<ResolvedHdf5ChunkedSingleLeafLayout> {
    let field = find_viirs_field_entry(dataset_path, "VNP13").ok_or_else(|| {
        RasterError::Other(format!(
            "dataset '{}' is outside VNP13 field catalog scope",
            dataset_path
        ))
    })?;

    resolve_hdf5_viirs_catalog_layout(container_path, field)
}

fn infer_row_dimension_index_from_records(
    records: &[wbhdf::btree::ChunkedStorageLeafRecord],
    num_dimensions: usize,
    row_count: usize,
) -> Option<usize> {
    let row_limit = u64::try_from(row_count).ok()?;
    let mut best: Option<(usize, usize, u64)> = None;

    for dim in 0..num_dimensions {
        let mut offsets = BTreeSet::new();
        let mut max_offset = 0_u64;
        let mut valid = true;

        for record in records {
            let Some(offset) = record.chunk_offsets.get(dim).copied() else {
                valid = false;
                break;
            };
            if offset >= row_limit {
                valid = false;
                break;
            }
            offsets.insert(offset);
            max_offset = max_offset.max(offset);
        }

        if !valid || offsets.is_empty() {
            continue;
        }

        let distinct_count = offsets.len();
        let candidate = (dim, distinct_count, max_offset);

        match best {
            None => best = Some(candidate),
            Some((_, best_distinct, best_max)) => {
                if distinct_count > best_distinct
                    || (distinct_count == best_distinct && max_offset > best_max)
                {
                    best = Some(candidate);
                }
            }
        }
    }

    best.map(|(dim, _, _)| dim)
}

fn estimate_chunk_record_budget(
    row_count: usize,
    row_width: usize,
    chunk_dimensions: &[u32],
    row_dimension_index: usize,
) -> Option<usize> {
    let row_chunk_height = chunk_dimensions
        .get(row_dimension_index)
        .and_then(|value| usize::try_from(*value).ok())
        .filter(|value| *value > 0)?;

    let chunk_col_span = chunk_dimensions
        .iter()
        .enumerate()
        .filter(|(idx, _)| *idx != row_dimension_index)
        .try_fold(1usize, |acc, (_, dim)| {
            let next = usize::try_from(*dim).ok()?;
            acc.checked_mul(next)
        })
        .filter(|value| *value > 0)
        .unwrap_or(row_width.max(1));

    let row_chunks = row_count.div_ceil(row_chunk_height);
    let col_chunks = row_width.max(1).div_ceil(chunk_col_span.max(1));
    let estimated = row_chunks.checked_mul(col_chunks)?;
    estimated.checked_mul(4)
}

fn is_supported_filter_pipeline(pipeline: &wbhdf::object_header::FilterPipelineMessage) -> bool {
    if pipeline.filters.is_empty() {
        return true;
    }

    let mut seen_deflate = 0usize;
    let mut seen_shuffle = 0usize;
    for filter in &pipeline.filters {
        match filter.id {
            1 => seen_deflate += 1,
            2 => seen_shuffle += 1,
            _ => return false,
        }
    }

    seen_deflate <= 1 && seen_shuffle <= 1
}

fn hdf5_unshuffle(bytes: &[u8], element_size: usize) -> Result<Vec<u8>> {
    if element_size == 0 {
        return Err(RasterError::Other(
            "HDF5 unshuffle requires element_size >= 1".to_string(),
        ));
    }
    if element_size == 1 {
        return Ok(bytes.to_vec());
    }
    if bytes.len() % element_size != 0 {
        return Err(RasterError::Other(format!(
            "HDF5 unshuffle payload length {} is not divisible by element_size {}",
            bytes.len(),
            element_size
        )));
    }

    let elements = bytes.len() / element_size;
    let mut out = vec![0_u8; bytes.len()];
    for byte_index in 0..element_size {
        let src_offset = byte_index * elements;
        for elem_index in 0..elements {
            out[elem_index * element_size + byte_index] = bytes[src_offset + elem_index];
        }
    }
    Ok(out)
}

fn decode_chunk_payload_with_filter_pipeline(
    payload: Vec<u8>,
    filter_pipeline: Option<&wbhdf::object_header::FilterPipelineMessage>,
    element_size: usize,
) -> Result<Vec<u8>> {
    let mut decoded = payload;

    let Some(pipeline) = filter_pipeline else {
        return Ok(decoded);
    };
    if pipeline.filters.is_empty() {
        return Ok(decoded);
    }

    for filter in pipeline.filters.iter().rev() {
        match filter.id {
            1 => {
                decoded = wbhdf::filters::decompress_zlib(&decoded).map_err(|err| {
                    RasterError::Other(format!("HDF5 chunk zlib decode failed: {err}"))
                })?;
            }
            2 => {
                decoded = hdf5_unshuffle(&decoded, element_size)?;
            }
            id => {
                return Err(RasterError::Other(format!(
                    "unsupported HDF5 chunk filter id '{}' in pipeline",
                    id
                )));
            }
        }
    }

    Ok(decoded)
}

fn decode_chunk_record_f32(
    container_path: &Path,
    record: &wbhdf::btree::ChunkedStorageLeafRecord,
    filter_pipeline: Option<&wbhdf::object_header::FilterPipelineMessage>,
) -> Result<Vec<f32>> {
    let payload = wbhdf::btree::read_chunk_payload_in_file(
        container_path,
        record.chunk_address,
        record.chunk_size,
    )
    .map_err(|err| RasterError::Other(format!("HDF5 chunk payload read failed: {err}")))?;
    let decoded_bytes = decode_chunk_payload_with_filter_pipeline(payload, filter_pipeline, 4)?;
    wbhdf::datatypes::decode_f32_slice(&decoded_bytes, wbhdf::datatypes::Endianness::Little)
        .map_err(RasterError::Other)
}

fn decode_chunk_record_f64(
    container_path: &Path,
    record: &wbhdf::btree::ChunkedStorageLeafRecord,
    filter_pipeline: Option<&wbhdf::object_header::FilterPipelineMessage>,
) -> Result<Vec<f64>> {
    let payload = wbhdf::btree::read_chunk_payload_in_file(
        container_path,
        record.chunk_address,
        record.chunk_size,
    )
    .map_err(|err| RasterError::Other(format!("HDF5 chunk payload read failed: {err}")))?;
    let decoded_bytes = decode_chunk_payload_with_filter_pipeline(payload, filter_pipeline, 8)?;
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
