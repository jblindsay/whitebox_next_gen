use serde_json::json;
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::f64::consts::PI;
use std::path::Path;
use std::time::Instant;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use wbcore::{
    parse_optional_output_path, LicenseTier, Tool, ToolArgs, ToolCategory, ToolContext, ToolError,
    ToolExample, ToolManifest, ToolMetadata, ToolParamDescriptor, ToolParamSpec, ToolRunResult,
    ToolStability,
};
use wbraster::DataType;
use wbraster::{ResolvedOpticalBundle, SensorBundleRegistry};
use wbprojection::Crs;

use crate::memory_store;
use crate::tools::slope_aspect_from_dem;

fn current_utc_rfc3339() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400;
    let y400 = days / 146097;
    let d1 = days % 146097;
    let y100 = (d1 / 36524).min(3);
    let d2 = d1 - y100 * 36524;
    let y4 = d2 / 1461;
    let d3 = d2 % 1461;
    let y1 = (d3 / 365).min(3);
    let doy = d3 - y1 * 365;
    let year = 1970 + y400 * 400 + y100 * 100 + y4 * 4 + y1;
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let month_days: &[u64] = if leap {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u64;
    let mut remaining = doy;
    for &md in month_days {
        if remaining < md { break; }
        remaining -= md;
        month += 1;
    }
    let day = remaining + 1;
    format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}Z")
}

pub struct TerrainCorrectedOpticalTool;

const SUMMARY_SCHEMA_VERSION: &str = "1.0.0";

// ── helpers ──────────────────────────────────────────────────────────────────

fn load_raster(path: &str, label: &str) -> Result<wbraster::Raster, ToolError> {
    if memory_store::raster_is_memory_path(path) {
        let id = memory_store::raster_path_to_id(path)
            .ok_or_else(|| ToolError::Execution(format!("invalid memory path for '{label}'")))?;
        memory_store::get_raster_arc_by_id(id)
            .map(|r| r.as_ref().clone())
            .ok_or_else(|| ToolError::Execution(format!("memory raster not found for '{label}'")))
    } else {
        wbraster::Raster::read(std::path::Path::new(path))
            .map_err(|e| ToolError::Execution(format!("failed reading '{label}': {e}")))
    }
}

fn write_raster(r: &wbraster::Raster, path: &str, label: &str) -> Result<(), ToolError> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ToolError::Execution(format!("failed creating output directory for '{label}': {e}"))
            })?;
        }
    }
    r.write(path, wbraster::RasterFormat::GeoTiff)
        .map_err(|e| ToolError::Execution(format!("failed writing '{label}': {e}")))
}

fn write_summary(path: &str, value: &serde_json::Value) -> Result<(), ToolError> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ToolError::Execution(format!("failed creating summary directory: {e}"))
            })?;
        }
    }
    std::fs::write(
        path,
        serde_json::to_string_pretty(value)
            .map_err(|e| ToolError::Execution(e.to_string()))?,
    )
    .map_err(|e| ToolError::Execution(format!("failed writing summary JSON: {e}")))
}

fn require_epsg(raster: &wbraster::Raster, label: &str) -> Result<u32, ToolError> {
    raster.crs.epsg.ok_or_else(|| {
        ToolError::Validation(format!(
            "'{label}' is missing CRS EPSG metadata; CRS harmonization requires EPSG on all raster inputs"
        ))
    })
}

fn same_grid(left: &wbraster::Raster, right: &wbraster::Raster) -> bool {
    const EPS: f64 = 1.0e-9;
    left.rows == right.rows
        && left.cols == right.cols
        && (left.x_min - right.x_min).abs() <= EPS
        && (left.y_min - right.y_min).abs() <= EPS
    && (left.x_max() - right.x_max()).abs() <= EPS
    && (left.y_max() - right.y_max()).abs() <= EPS
}

fn harmonize_to_reference(
    raster: wbraster::Raster,
    label: &str,
    reference: &wbraster::Raster,
    reference_label: &str,
    ctx: &ToolContext,
    resample: wbraster::ResampleMethod,
) -> Result<wbraster::Raster, ToolError> {
    let src_epsg = require_epsg(&raster, label)?;
    let dst_epsg = require_epsg(reference, reference_label)?;

    if src_epsg == dst_epsg && same_grid(&raster, reference) {
        return Ok(raster);
    }

    ctx.progress.info(&format!(
        "terrain_corrected_optical_analytics: reprojecting '{label}' from EPSG:{src_epsg} to match '{reference_label}' EPSG:{dst_epsg} grid"
    ));

    let options = wbraster::ReprojectOptions::new(dst_epsg, resample)
        .with_size(reference.cols, reference.rows)
        .with_extent(wbraster::Extent {
            x_min: reference.x_min,
            y_min: reference.y_min,
            x_max: reference.x_max(),
            y_max: reference.y_max(),
        });

    raster.reproject_with_options(&options).map_err(|e| {
        ToolError::Execution(format!(
            "failed reprojecting '{label}' to match '{reference_label}' grid: {e}"
        ))
    })
}

fn build_reflectance_stack(
    red: &wbraster::Raster,
    nir: &wbraster::Raster,
    green: Option<&wbraster::Raster>,
    blue: Option<&wbraster::Raster>,
) -> wbraster::Raster {
    let bands = 2 + green.is_some() as usize + blue.is_some() as usize;
    let mut stack = wbraster::Raster::new(wbraster::RasterConfig {
        rows: red.rows,
        cols: red.cols,
        bands,
        nodata: red.nodata,
        x_min: red.x_min,
        y_min: red.y_min,
        cell_size: red.cell_size_x,
        cell_size_y: Some(red.cell_size_y),
        data_type: red.data_type,
        crs: red.crs.clone(),
        metadata: red.metadata.clone(),
    });

    let n = red.rows * red.cols;
    let green_band_index = if green.is_some() { Some(2usize) } else { None };
    let blue_band_index = if blue.is_some() {
        Some(if green.is_some() { 3usize } else { 2usize })
    } else {
        None
    };

    stack.data.par_fill_with(|idx| {
        let band = idx / n;
        let i = idx % n;
        if band == 0 {
            red.data.get_f64(i)
        } else if band == 1 {
            nir.data.get_f64(i)
        } else if Some(band) == green_band_index {
            green.map(|g| g.data.get_f64(i)).unwrap_or(red.nodata)
        } else if Some(band) == blue_band_index {
            blue.map(|b| b.data.get_f64(i)).unwrap_or(red.nodata)
        } else {
            red.nodata
        }
    });

    stack
}

/// Convert degrees to radians.
#[inline]
fn deg2rad(d: f64) -> f64 { d * PI / 180.0 }

/// Cosine of the solar incidence angle between a surface normal and the solar beam.
///
/// `slope_rad` — surface slope in radians
/// `aspect_rad` — surface aspect in radians (measured clockwise from north)
/// `solar_zenith_rad` — solar zenith angle in radians
/// `solar_azimuth_rad` — solar azimuth in radians (measured clockwise from north)
#[inline]
fn cos_incidence(slope_rad: f64, aspect_rad: f64, solar_zenith_rad: f64, solar_azimuth_rad: f64) -> f64 {
    let cos_sz = solar_zenith_rad.cos();
    let sin_sz = solar_zenith_rad.sin();
    let cos_slope = slope_rad.cos();
    let sin_slope = slope_rad.sin();
    let azimuth_diff = solar_azimuth_rad - aspect_rad;
    cos_sz * cos_slope + sin_sz * sin_slope * azimuth_diff.cos()
}

/// C-correction factor: corrects surface reflectance for topographic illumination.
///
/// Given a linear regression IL = m * cos_i + b across the scene, C = b / m.
/// Returns `None` when the correction would be degenerate (m ≈ 0).
#[inline]
fn c_correction_factor(m: f64, b: f64) -> Option<f64> {
    if m.abs() < 1e-8 { None } else { Some(b / m) }
}

/// Apply C-correction to a reflectance value.
///
/// ρ_corrected = ρ_raw * (cos_z + c) / (cos_i + c)
#[inline]
fn apply_c_correction(reflectance: f64, cos_z: f64, cos_i: f64, c: f64) -> f64 {
    let denom = cos_i + c;
    if denom.abs() < 1e-10 { reflectance } else { reflectance * (cos_z + c) / denom }
}

// ── linear regression helpers ────────────────────────────────────────────────

struct LinRegResult { m: f64, b: f64, r_squared: f64, n: usize }

/// Ordinary least-squares regression of y on x from paired slices.
fn ols(x: &[f64], y: &[f64]) -> LinRegResult {
    let n = x.len().min(y.len());
    if n < 2 {
        return LinRegResult { m: 0.0, b: 0.0, r_squared: 0.0, n };
    }
    let sum_x: f64 = x[..n].iter().sum();
    let sum_y: f64 = y[..n].iter().sum();
    let sum_xx: f64 = x[..n].iter().map(|v| v * v).sum();
    let sum_xy: f64 = x[..n].iter().zip(y[..n].iter()).map(|(xi, yi)| xi * yi).sum();
    let n_f = n as f64;
    let denom = n_f * sum_xx - sum_x * sum_x;
    if denom.abs() < 1e-15 {
        return LinRegResult { m: 0.0, b: sum_y / n_f, r_squared: 0.0, n };
    }
    let m = (n_f * sum_xy - sum_x * sum_y) / denom;
    let b = (sum_y - m * sum_x) / n_f;
    let mean_y = sum_y / n_f;
    let ss_tot: f64 = y[..n].iter().map(|yi| (yi - mean_y).powi(2)).sum();
    let ss_res: f64 = y[..n].iter().zip(x[..n].iter()).map(|(yi, xi)| (yi - (m * xi + b)).powi(2)).sum();
    let r_squared = if ss_tot.abs() < 1e-15 { 0.0 } else { 1.0 - ss_res / ss_tot };
    LinRegResult { m, b, r_squared, n }
}

// ── profile config ───────────────────────────────────────────────────────────

struct ProfileSettings {
    cloud_threshold_fraction: f64,
    cloud_shadow_threshold_fraction: f64,
    min_cos_i: f64,
    correction_weight_red: f64,
    correction_weight_nir: f64,
    regression_sample_step: usize,
}

fn profile_settings(profile: &str) -> ProfileSettings {
    match profile {
        "conservative" => ProfileSettings {
            cloud_threshold_fraction: 2000.0 / 4095.0,
            cloud_shadow_threshold_fraction: 200.0 / 4095.0,
            min_cos_i: 0.05,
            correction_weight_red: 1.0,
            correction_weight_nir: 1.0,
            regression_sample_step: 1,
        },
        "fast" => ProfileSettings {
            cloud_threshold_fraction: 2500.0 / 4095.0,
            cloud_shadow_threshold_fraction: 150.0 / 4095.0,
            min_cos_i: 0.02,
            correction_weight_red: 0.9,
            correction_weight_nir: 0.9,
            regression_sample_step: 4,
        },
        _ => ProfileSettings {
            cloud_threshold_fraction: 2200.0 / 4095.0,
            cloud_shadow_threshold_fraction: 180.0 / 4095.0,
            min_cos_i: 0.03,
            correction_weight_red: 0.95,
            correction_weight_nir: 0.95,
            regression_sample_step: 2,
        },
    }
}

fn infer_radiometric_scale(red: &wbraster::Raster, nir: &wbraster::Raster) -> (f64, String) {
    let mut observed_max = 0.0_f64;
    let n = red.rows * red.cols;
    for i in 0..n {
        let rv = red.data.get_f64(i);
        let nv = nir.data.get_f64(i);
        if red.is_nodata(rv) || nir.is_nodata(nv) {
            continue;
        }
        observed_max = observed_max.max(rv).max(nv);
    }

    if observed_max <= 1.5 {
        return (1.0, "reflectance_0_1".to_string());
    }

    let (scale, mode) = match red.data_type {
        DataType::U8 | DataType::I8 => (255.0, "uint8_like"),
        DataType::U16 | DataType::I16 => {
            if observed_max <= 1100.0 {
                (1023.0, "uint10_like")
            } else if observed_max <= 5000.0 {
                (4095.0, "uint12_like")
            } else if observed_max <= 20000.0 {
                (16383.0, "uint14_like")
            } else {
                (65535.0, "uint16_like")
            }
        }
        _ => {
            if observed_max <= 300.0 {
                (255.0, "uint8_like")
            } else if observed_max <= 1200.0 {
                (1023.0, "uint10_like")
            } else if observed_max <= 5000.0 {
                (4095.0, "uint12_like")
            } else if observed_max <= 20000.0 {
                (16383.0, "uint14_like")
            } else if observed_max <= 70000.0 {
                (65535.0, "uint16_like")
            } else {
                (observed_max.max(1.0), "observed_max")
            }
        }
    };

    (scale, mode.to_string())
}

#[derive(Clone, Copy, Debug)]
enum QaMaskFormat {
    Auto,
    LandsatQaPixel,
    Sentinel2Scl,
    Sentinel2Qa60,
    Binary,
}

impl QaMaskFormat {
    fn parse(value: Option<&str>) -> Result<Self, ToolError> {
        match value.unwrap_or("auto") {
            "auto" => Ok(Self::Auto),
            "landsat_qa_pixel" => Ok(Self::LandsatQaPixel),
            "sentinel2_scl" => Ok(Self::Sentinel2Scl),
            "sentinel2_qa60" => Ok(Self::Sentinel2Qa60),
            "binary" => Ok(Self::Binary),
            other => Err(ToolError::Validation(format!(
                "parameter 'qa_mask_format' invalid: '{other}'. expected one of: auto, landsat_qa_pixel, sentinel2_scl, sentinel2_qa60, binary"
            ))),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::LandsatQaPixel => "landsat_qa_pixel",
            Self::Sentinel2Scl => "sentinel2_scl",
            Self::Sentinel2Qa60 => "sentinel2_qa60",
            Self::Binary => "binary",
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum MaskStrategy {
    Auto,
    QaOnly,
    HeuristicOnly,
    QaPlusHeuristic,
}

impl MaskStrategy {
    fn parse(value: Option<&str>) -> Result<Self, ToolError> {
        match value.unwrap_or("auto") {
            "auto" => Ok(Self::Auto),
            "qa_only" => Ok(Self::QaOnly),
            "heuristic_only" => Ok(Self::HeuristicOnly),
            "qa_plus_heuristic" => Ok(Self::QaPlusHeuristic),
            other => Err(ToolError::Validation(format!(
                "parameter 'mask_strategy' invalid: '{other}'. expected one of: auto, qa_only, heuristic_only, qa_plus_heuristic"
            ))),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::QaOnly => "qa_only",
            Self::HeuristicOnly => "heuristic_only",
            Self::QaPlusHeuristic => "qa_plus_heuristic",
        }
    }
}

fn infer_qa_mask_format(qa: &wbraster::Raster) -> QaMaskFormat {
    let mut max_v: i64 = 0;
    let mut valid = 0usize;
    let mut scl_like = 0usize;
    let mut qa60_like = 0usize;
    let n = qa.rows * qa.cols;

    for i in 0..n {
        let v = qa.data.get_f64(i);
        if qa.is_nodata(v) {
            continue;
        }
        let iv = v.round() as i64;
        max_v = max_v.max(iv);
        valid += 1;
        if (0..=11).contains(&iv) {
            scl_like += 1;
        }
        let qa60_cloud = (iv & (1 << 10)) != 0;
        let qa60_cirrus = (iv & (1 << 11)) != 0;
        if qa60_cloud || qa60_cirrus {
            qa60_like += 1;
        }
    }

    if valid == 0 {
        return QaMaskFormat::Binary;
    }
    if max_v <= 1 {
        return QaMaskFormat::Binary;
    }
    if scl_like * 100 >= valid * 90 {
        return QaMaskFormat::Sentinel2Scl;
    }
    if qa60_like > 0 {
        return QaMaskFormat::Sentinel2Qa60;
    }
    QaMaskFormat::LandsatQaPixel
}

fn qa_mask_to_flags(mask_value: f64, nodata: f64, format: QaMaskFormat) -> Option<(bool, bool)> {
    if (mask_value - nodata).abs() < f64::EPSILON {
        return None;
    }

    let iv = mask_value.round() as i64;
    let (cloud, shadow) = match format {
        QaMaskFormat::Binary => (iv != 0, false),
        QaMaskFormat::Sentinel2Scl => {
            let cloud = matches!(iv, 8 | 9 | 10);
            let shadow = iv == 3;
            (cloud, shadow)
        }
        QaMaskFormat::Sentinel2Qa60 => {
            let cloud = (iv & (1 << 10)) != 0 || (iv & (1 << 11)) != 0;
            (cloud, false)
        }
        QaMaskFormat::LandsatQaPixel => {
            let dilated_cloud = (iv & (1 << 1)) != 0;
            let cirrus = (iv & (1 << 2)) != 0;
            let cloud = (iv & (1 << 3)) != 0;
            let cloud_shadow = (iv & (1 << 4)) != 0;
            (cloud || cirrus || dilated_cloud, cloud_shadow)
        }
        QaMaskFormat::Auto => (false, false),
    };
    Some((cloud, shadow))
}

fn mask_value_from_flags(cloud: bool, shadow: bool) -> f64 {
    if cloud {
        2.0
    } else if shadow {
        3.0
    } else {
        0.0
    }
}

#[derive(Clone, Copy, Debug)]
enum SolarResolveMode {
    Auto,
    Manual,
    Metadata,
    DatetimeLocation,
}

impl SolarResolveMode {
    fn parse(value: Option<&str>) -> Result<Self, ToolError> {
        match value.unwrap_or("auto") {
            "auto" => Ok(Self::Auto),
            "manual" => Ok(Self::Manual),
            "metadata" => Ok(Self::Metadata),
            "datetime_location" => Ok(Self::DatetimeLocation),
            other => Err(ToolError::Validation(format!(
                "parameter 'solar_mode' invalid: '{other}'. expected one of: auto, manual, metadata, datetime_location"
            ))),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Manual => "manual",
            Self::Metadata => "metadata",
            Self::DatetimeLocation => "datetime_location",
        }
    }
}

fn parse_number_from_text(text: &str) -> Option<f64> {
    let mut token = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() || matches!(ch, '.' | '-' | '+' | 'e' | 'E') {
            token.push(ch);
        } else if !token.is_empty() {
            if let Ok(v) = token.parse::<f64>() {
                return Some(v);
            }
            token.clear();
        }
    }
    if !token.is_empty() {
        token.parse::<f64>().ok()
    } else {
        None
    }
}

fn metadata_angle(meta: &[(String, String)], keys: &[&str]) -> Option<f64> {
    for (k, v) in meta {
        let kl = k.to_ascii_lowercase();
        if keys.iter().any(|cand| kl.contains(cand)) {
            if let Some(num) = parse_number_from_text(v) {
                return Some(num);
            }
        }
    }
    None
}

fn extract_solar_from_metadata(raster: &wbraster::Raster) -> Option<(f64, f64)> {
    let zenith_keys = ["solar_zenith", "sun_zenith", "mean_solar_zenith", "sza"];
    let azimuth_keys = ["solar_azimuth", "sun_azimuth", "mean_solar_azimuth", "saa"];
    let z = metadata_angle(&raster.metadata, &zenith_keys)?;
    let a = metadata_angle(&raster.metadata, &azimuth_keys)?;
    Some((z, a))
}

fn extract_solar_from_bundle(bundle: &ResolvedOpticalBundle) -> Option<(f64, f64)> {
    let z = bundle.mean_solar_zenith_deg?;
    let a = bundle.mean_solar_azimuth_deg?;
    Some((z, a))
}

fn raster_centroid_lonlat(raster: &wbraster::Raster) -> Result<(f64, f64), ToolError> {
    let x = raster.x_min + raster.cell_size_x * raster.cols as f64 * 0.5;
    let y = raster.y_min + raster.cell_size_y * raster.rows as f64 * 0.5;
    let epsg = raster.crs.epsg.ok_or_else(|| {
        ToolError::Validation("cannot infer centroid lon/lat: raster CRS EPSG is missing".to_string())
    })?;
    let src = Crs::from_epsg(epsg)
        .map_err(|e| ToolError::Validation(format!("cannot build source CRS EPSG:{epsg}: {e}")))?;
    let wgs84 = Crs::from_epsg(4326)
        .map_err(|e| ToolError::Validation(format!("cannot build WGS84 CRS: {e}")))?;
    src.transform_to(x, y, &wgs84)
        .map_err(|e| ToolError::Validation(format!("failed transforming centroid to lon/lat: {e}")))
}

fn solar_position_from_utc(lat_deg: f64, lon_deg: f64, dt: OffsetDateTime) -> (f64, f64) {
    let day = dt.ordinal() as f64;
    let minutes_utc = (dt.hour() as f64) * 60.0 + (dt.minute() as f64) + (dt.second() as f64) / 60.0;
    let gamma = 2.0 * PI / 365.0 * (day - 1.0 + (minutes_utc / 60.0 - 12.0) / 24.0);

    let eq_time = 229.18
        * (0.000075
            + 0.001868 * gamma.cos()
            - 0.032077 * gamma.sin()
            - 0.014615 * (2.0 * gamma).cos()
            - 0.040849 * (2.0 * gamma).sin());

    let decl = 0.006918
        - 0.399912 * gamma.cos()
        + 0.070257 * gamma.sin()
        - 0.006758 * (2.0 * gamma).cos()
        + 0.000907 * (2.0 * gamma).sin()
        - 0.002697 * (3.0 * gamma).cos()
        + 0.00148 * (3.0 * gamma).sin();

    let true_solar_time = minutes_utc + eq_time + 4.0 * lon_deg;
    let mut hour_angle = true_solar_time / 4.0 - 180.0;
    while hour_angle < -180.0 {
        hour_angle += 360.0;
    }
    while hour_angle > 180.0 {
        hour_angle -= 360.0;
    }

    let lat = lat_deg.to_radians();
    let ha = hour_angle.to_radians();
    let cos_zenith = (lat.sin() * decl.sin() + lat.cos() * decl.cos() * ha.cos()).clamp(-1.0, 1.0);
    let zenith = cos_zenith.acos().to_degrees();

    let az = ha.sin().atan2(ha.cos() * lat.sin() - decl.tan() * lat.cos());
    let mut azimuth = az.to_degrees() + 180.0;
    if azimuth < 0.0 {
        azimuth += 360.0;
    }
    if azimuth >= 360.0 {
        azimuth -= 360.0;
    }

    (zenith, azimuth)
}

// ── Tool implementation ──────────────────────────────────────────────────────

impl Tool for TerrainCorrectedOpticalTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "terrain_corrected_optical_analytics",
            display_name: "Terrain-Corrected Optical Prep",
            summary: r#"Terrain correction normalizes optical reflectance for topographic illumination variations (slope, aspect effects on solar incidence angle). Per-pixel solar incidence angles computed from DEM-derived slopes and aspects compared against nadir zenith reference angle; reflectance normalized to equivalent nadir illumination. Terrain correction essential in mountainous regions where elevation and slope substantially vary across scene, causing false spectral variation. Key Features: DEM-based illumination normalization; corrects slope/aspect effects; preserves spectral character; reduces topographic biasing; improves classification in mountainous terrain; enables multi-temporal comparison. Use Cases: Mountainous terrain analysis; landcover mapping in topographically varied regions; normalized vegetation index generation; change detection across seasons/years; reducing false classification variation from terrain effects. Output Interpretation: Output is terrain-corrected reflectance with reduced topographic variation. Steep south-facing slopes typically show highest corrections; north-facing slopes and flat terrain show minimal change. Correction magnitude indicates illumination variation severity. Residual variations after correction indicate unmodeled effects (atmospheric, vegetation bidirectionality). Corrected reflectance enables fairer multi-temporal comparisons."#,
                category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input_dem", description: "Input DEM raster path co-registered with optical bands.", required: true },
                ToolParamSpec { name: "bundle_root", description: "Optional optical sensor bundle root directory (currently Sentinel-2 SAFE, Landsat, or DIMAP SPOT/Pleiades). When provided, red/NIR (plus optional green/blue), QA when available, and solar metadata are auto-resolved when explicit paths are omitted.", required: false },
                ToolParamSpec { name: "input_red", description: "Input red band raster path. Optional when bundle_root is provided.", required: false },
                ToolParamSpec { name: "input_nir", description: "Input near-infrared band raster path. Optional when bundle_root is provided.", required: false },
                ToolParamSpec { name: "input_green", description: "Input green band raster path (optional).", required: false },
                ToolParamSpec { name: "input_blue", description: "Input blue band raster path (optional). Auto-resolved from bundle metadata when bundle_root is provided.", required: false },
                ToolParamSpec { name: "solar_mode", description: "Solar-angle resolution mode: auto, manual, metadata, datetime_location (default auto).", required: false },
                ToolParamSpec { name: "solar_zenith_deg", description: "Solar zenith angle in degrees at image acquisition time (required in manual mode).", required: false },
                ToolParamSpec { name: "solar_azimuth_deg", description: "Solar azimuth in degrees (clockwise from north) at acquisition time (required in manual mode).", required: false },
                ToolParamSpec { name: "acquisition_datetime_utc", description: "Acquisition timestamp in RFC3339 UTC for datetime_location mode (e.g., 2026-03-31T15:20:00Z).", required: false },
                ToolParamSpec { name: "latitude", description: "Optional latitude in decimal degrees for datetime_location mode; if omitted, raster centroid is used.", required: false },
                ToolParamSpec { name: "longitude", description: "Optional longitude in decimal degrees for datetime_location mode; if omitted, raster centroid is used.", required: false },
                ToolParamSpec { name: "profile", description: "Processing profile: conservative, balanced (default), fast.", required: false },
                ToolParamSpec { name: "cloud_threshold", description: "Optional cloud threshold in source band units; auto-inferred when omitted.", required: false },
                ToolParamSpec { name: "shadow_threshold", description: "Optional cloud-shadow threshold in source band units; auto-inferred when omitted.", required: false },
                ToolParamSpec { name: "qa_mask", description: "Optional QA mask raster path (e.g., Landsat QA_PIXEL, Sentinel-2 SCL/QA60, or binary cloud mask).", required: false },
                ToolParamSpec { name: "qa_mask_format", description: "QA mask encoding: auto, landsat_qa_pixel, sentinel2_scl, sentinel2_qa60, binary (default auto).", required: false },
                ToolParamSpec { name: "mask_strategy", description: "Mask strategy: auto, qa_only, heuristic_only, qa_plus_heuristic (default auto).", required: false },
                ToolParamSpec { name: "z_factor", description: "DEM vertical exaggeration for slope/aspect computation (default 1.0).", required: false },
                ToolParamSpec { name: "output_prefix", description: "Output prefix for all output artifacts (default terrain_corrected).", required: false },
                ToolParamSpec { name: "output", description: "Deprecated — use output_prefix. Ignored.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("bundle_root".to_string(), serde_json::Value::Null);
        defaults.insert("input_red".to_string(), json!("red.tif"));
        defaults.insert("input_nir".to_string(), json!("nir.tif"));
        defaults.insert("input_green".to_string(), serde_json::Value::Null);
        defaults.insert("input_blue".to_string(), serde_json::Value::Null);
        defaults.insert("input_dem".to_string(), json!("dem.tif"));
        defaults.insert("solar_mode".to_string(), json!("auto"));
        defaults.insert("solar_zenith_deg".to_string(), json!(40.0));
        defaults.insert("solar_azimuth_deg".to_string(), json!(165.0));
        defaults.insert("acquisition_datetime_utc".to_string(), serde_json::Value::Null);
        defaults.insert("latitude".to_string(), serde_json::Value::Null);
        defaults.insert("longitude".to_string(), serde_json::Value::Null);
        defaults.insert("profile".to_string(), json!("balanced"));
        defaults.insert("cloud_threshold".to_string(), serde_json::Value::Null);
        defaults.insert("shadow_threshold".to_string(), serde_json::Value::Null);
        defaults.insert("qa_mask".to_string(), serde_json::Value::Null);
        defaults.insert("qa_mask_format".to_string(), json!("auto"));
        defaults.insert("mask_strategy".to_string(), json!("auto"));
        defaults.insert("z_factor".to_string(), json!(1.0));
        defaults.insert("output_prefix".to_string(), json!("terrain_corrected"));

        let example_args = defaults.clone();

        ToolManifest {
            id: "terrain_corrected_optical_analytics".to_string(),
            display_name: "Terrain-Corrected Optical Prep".to_string(),
            summary: "Topographic C-correction of multispectral optical bands using a co-registered DEM. Outputs surface reflectance stack, correction factor, cloud/shadow mask, and quality confidence.".to_string(),
                category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input_dem".to_string(), description: "DEM raster co-registered with optical bands.".to_string(), required: true },
                ToolParamDescriptor { name: "bundle_root".to_string(), description: "Optional optical sensor bundle root directory (currently Sentinel-2 SAFE, Landsat, or DIMAP SPOT/Pleiades) used to auto-resolve required bands, QA when available, and solar metadata when explicit inputs are omitted.".to_string(), required: false },
                ToolParamDescriptor { name: "input_red".to_string(), description: "Red band raster (optional when bundle_root is provided).".to_string(), required: false },
                ToolParamDescriptor { name: "input_nir".to_string(), description: "NIR band raster (optional when bundle_root is provided).".to_string(), required: false },
                ToolParamDescriptor { name: "input_green".to_string(), description: "Green band raster (optional).".to_string(), required: false },
                ToolParamDescriptor { name: "input_blue".to_string(), description: "Blue band raster (optional). Auto-resolved from bundle metadata when bundle_root is provided.".to_string(), required: false },
                ToolParamDescriptor { name: "solar_mode".to_string(), description: "Solar-angle resolution mode: auto, manual, metadata, datetime_location.".to_string(), required: false },
                ToolParamDescriptor { name: "solar_zenith_deg".to_string(), description: "Solar zenith angle in degrees (required in manual mode).".to_string(), required: false },
                ToolParamDescriptor { name: "solar_azimuth_deg".to_string(), description: "Solar azimuth in degrees (clockwise from north, required in manual mode).".to_string(), required: false },
                ToolParamDescriptor { name: "acquisition_datetime_utc".to_string(), description: "Acquisition timestamp in RFC3339 UTC for datetime_location mode.".to_string(), required: false },
                ToolParamDescriptor { name: "latitude".to_string(), description: "Optional latitude for datetime_location mode.".to_string(), required: false },
                ToolParamDescriptor { name: "longitude".to_string(), description: "Optional longitude for datetime_location mode.".to_string(), required: false },
                ToolParamDescriptor { name: "profile".to_string(), description: "Processing profile: conservative, balanced, fast.".to_string(), required: false },
                ToolParamDescriptor { name: "cloud_threshold".to_string(), description: "Optional cloud threshold in source band units. Auto-inferred when omitted.".to_string(), required: false },
                ToolParamDescriptor { name: "shadow_threshold".to_string(), description: "Optional cloud-shadow threshold in source band units. Auto-inferred when omitted.".to_string(), required: false },
                ToolParamDescriptor { name: "qa_mask".to_string(), description: "Optional QA mask raster path.".to_string(), required: false },
                ToolParamDescriptor { name: "qa_mask_format".to_string(), description: "QA mask encoding: auto, landsat_qa_pixel, sentinel2_scl, sentinel2_qa60, binary.".to_string(), required: false },
                ToolParamDescriptor { name: "mask_strategy".to_string(), description: "Mask strategy: auto, qa_only, heuristic_only, qa_plus_heuristic.".to_string(), required: false },
                ToolParamDescriptor { name: "z_factor".to_string(), description: "Vertical exaggeration for slope/aspect.".to_string(), required: false },
                ToolParamDescriptor { name: "output_prefix".to_string(), description: "Prefix for all output artifacts.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "terrain_corrected_balanced".to_string(),
                description: "Apply balanced C-correction to summer optical imagery.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "optical".to_string(),
                "topographic_correction".to_string(),
                "c_correction".to_string(),
                "surface_reflectance".to_string(),
                "cloud_mask".to_string(),
                "remote_sensing".to_string(),
                "pro".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        args.get("input_dem")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'input_dem' is required".to_string()))?;

        let bundle_root = args.get("bundle_root").and_then(|v| v.as_str());
        let has_red = args.get("input_red").and_then(|v| v.as_str()).is_some();
        let has_nir = args.get("input_nir").and_then(|v| v.as_str()).is_some();
        if bundle_root.is_none() && (!has_red || !has_nir) {
            return Err(ToolError::Validation(
                "parameters 'input_red' and 'input_nir' are required unless 'bundle_root' is provided"
                    .to_string(),
            ));
        }

        let solar_mode = SolarResolveMode::parse(args.get("solar_mode").and_then(|v| v.as_str()))?;

        if matches!(solar_mode, SolarResolveMode::Manual) {
            for key in &["solar_zenith_deg", "solar_azimuth_deg"] {
                args.get(*key)
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| ToolError::Validation(format!("parameter '{key}' is required and must be numeric in manual solar_mode")))?;
            }
        }

        if let Some(dt) = args.get("acquisition_datetime_utc").and_then(|v| v.as_str()) {
            OffsetDateTime::parse(dt, &Rfc3339).map_err(|e| {
                ToolError::Validation(format!("parameter 'acquisition_datetime_utc' must be RFC3339 UTC: {e}"))
            })?;
        }

        if let Some(lat) = args.get("latitude").and_then(|v| v.as_f64()) {
            if !(-90.0..=90.0).contains(&lat) {
                return Err(ToolError::Validation(
                    "parameter 'latitude' must be in [-90, 90]".to_string(),
                ));
            }
        }

        if let Some(lon) = args.get("longitude").and_then(|v| v.as_f64()) {
            if !(-180.0..=180.0).contains(&lon) {
                return Err(ToolError::Validation(
                    "parameter 'longitude' must be in [-180, 180]".to_string(),
                ));
            }
        }

        if let Some(z) = args.get("solar_zenith_deg").and_then(|v| v.as_f64()) {
            if !(0.0..=90.0).contains(&z) {
                return Err(ToolError::Validation(
                    "parameter 'solar_zenith_deg' must be in [0, 90]".to_string(),
                ));
            }
        }

        if let Some(profile) = args.get("profile").and_then(|v| v.as_str()) {
            if !matches!(profile, "conservative" | "balanced" | "fast") {
                return Err(ToolError::Validation(
                    "parameter 'profile' must be one of: conservative, balanced, fast".to_string(),
                ));
            }
        }

        if let Some(v) = args.get("cloud_threshold").and_then(|v| v.as_f64()) {
            if v <= 0.0 {
                return Err(ToolError::Validation(
                    "parameter 'cloud_threshold' must be > 0".to_string(),
                ));
            }
        }

        if let Some(v) = args.get("shadow_threshold").and_then(|v| v.as_f64()) {
            if v < 0.0 {
                return Err(ToolError::Validation(
                    "parameter 'shadow_threshold' must be >= 0".to_string(),
                ));
            }
        }

        if let (Some(cloud), Some(shadow)) = (
            args.get("cloud_threshold").and_then(|v| v.as_f64()),
            args.get("shadow_threshold").and_then(|v| v.as_f64()),
        ) {
            if shadow >= cloud {
                return Err(ToolError::Validation(
                    "parameter 'shadow_threshold' must be less than 'cloud_threshold'".to_string(),
                ));
            }
        }

        let _ = QaMaskFormat::parse(args.get("qa_mask_format").and_then(|v| v.as_str()))?;
        let strategy = MaskStrategy::parse(args.get("mask_strategy").and_then(|v| v.as_str()))?;
        if matches!(strategy, MaskStrategy::QaOnly | MaskStrategy::QaPlusHeuristic)
            && args.get("qa_mask").and_then(|v| v.as_str()).is_none()
            && bundle_root.is_none()
        {
            return Err(ToolError::Validation(
                "parameter 'qa_mask' is required when mask_strategy is qa_only or qa_plus_heuristic (unless 'bundle_root' is provided)".to_string(),
            ));
        }

        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let bundle_root = args
            .get("bundle_root")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let resolved_bundle = bundle_root
            .as_deref()
            .map(|root| {
                let registry = SensorBundleRegistry::with_defaults();
                registry.resolve_optical_bundle(Path::new(root))
            })
            .transpose()
            .map_err(|e| ToolError::Execution(format!("failed resolving sensor bundle: {e}")))?;

        let red_path = if let Some(path) = args.get("input_red").and_then(|v| v.as_str()) {
            path.to_string()
        } else if let Some(bundle) = resolved_bundle.as_ref() {
            bundle.red_path.clone().ok_or_else(|| {
                ToolError::Validation(
                    "parameter 'input_red' not provided and sensor bundle does not provide a red band"
                        .to_string(),
                )
            })?
        } else {
            return Err(ToolError::Validation(
                "parameter 'input_red' is required unless 'bundle_root' provides a red band".to_string(),
            ));
        };

        let nir_path = if let Some(path) = args.get("input_nir").and_then(|v| v.as_str()) {
            path.to_string()
        } else if let Some(bundle) = resolved_bundle.as_ref() {
            bundle.nir_path.clone().ok_or_else(|| {
                ToolError::Validation(
                    "parameter 'input_nir' not provided and sensor bundle does not provide a NIR band"
                        .to_string(),
                )
            })?
        } else {
            return Err(ToolError::Validation(
                "parameter 'input_nir' is required unless 'bundle_root' provides a NIR band".to_string(),
            ));
        };

        let dem_path = args.get("input_dem").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'input_dem' is required".to_string()))?;
        let green_path: Option<String> = if let Some(path) = args.get("input_green").and_then(|v| v.as_str()) {
            Some(path.to_string())
        } else {
            resolved_bundle
                .as_ref()
                .and_then(|bundle| bundle.green_path.clone())
        };
        let blue_path: Option<String> = if let Some(path) = args.get("input_blue").and_then(|v| v.as_str()) {
            Some(path.to_string())
        } else {
            resolved_bundle
                .as_ref()
                .and_then(|bundle| bundle.blue_path.clone())
        };
        let solar_mode = SolarResolveMode::parse(args.get("solar_mode").and_then(|v| v.as_str()))?;
        let solar_zenith_manual = args.get("solar_zenith_deg").and_then(|v| v.as_f64());
        let solar_azimuth_manual = args.get("solar_azimuth_deg").and_then(|v| v.as_f64());
        let acquisition_datetime_arg = args
            .get("acquisition_datetime_utc")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let acquisition_datetime_utc = acquisition_datetime_arg.or_else(|| {
            resolved_bundle
                .as_ref()
                .and_then(|bundle| bundle.acquisition_datetime_utc.clone())
        });
        let latitude_arg = args.get("latitude").and_then(|v| v.as_f64());
        let longitude_arg = args.get("longitude").and_then(|v| v.as_f64());
        let profile = args.get("profile").and_then(|v| v.as_str()).unwrap_or("balanced");
        let cloud_threshold_override = args.get("cloud_threshold").and_then(|v| v.as_f64());
        let shadow_threshold_override = args.get("shadow_threshold").and_then(|v| v.as_f64());
        let (qa_mask_path, qa_mask_auto_format_hint) = if let Some(path) = args.get("qa_mask").and_then(|v| v.as_str()) {
            (Some(path.to_string()), None)
        } else if let Some(bundle) = resolved_bundle.as_ref() {
            if let Some(path) = bundle.qa_scl_path.clone() {
                (
                    Some(path),
                    Some(QaMaskFormat::Sentinel2Scl),
                )
            } else if let Some(path) = bundle.qa_qa60_path.clone() {
                (
                    Some(path),
                    Some(QaMaskFormat::Sentinel2Qa60),
                )
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };
        let qa_mask_format_requested = QaMaskFormat::parse(args.get("qa_mask_format").and_then(|v| v.as_str()))?;
        let mask_strategy = MaskStrategy::parse(args.get("mask_strategy").and_then(|v| v.as_str()))?;
        if matches!(mask_strategy, MaskStrategy::QaOnly | MaskStrategy::QaPlusHeuristic)
            && qa_mask_path.is_none()
        {
            return Err(ToolError::Validation(
                "mask_strategy requires QA data, but no QA mask was provided or discovered from the sensor bundle"
                    .to_string(),
            ));
        }
        let z_factor = args.get("z_factor").and_then(|v| v.as_f64()).unwrap_or(1.0).max(0.0001);
        let output_prefix = args.get("output_prefix").and_then(|v| v.as_str()).unwrap_or("terrain_corrected").to_string();

        let ps = profile_settings(profile);
        let run_t0 = Instant::now();
        let mut stage_timings_ms: BTreeMap<String, f64> = BTreeMap::new();

        // ── Step 1: Load optical bands ───────────────────────────────────────
        ctx.progress.info("terrain_corrected_optical_analytics: step 1/5 – loading optical bands");
        let step1_t0 = Instant::now();
        let red = load_raster(&red_path, "input_red")?;
        let nir_raw = load_raster(&nir_path, "input_nir")?;
        let green_raw = green_path.as_deref().map(|p| load_raster(p, "input_green")).transpose()?;
        let blue_raw = blue_path.as_deref().map(|p| load_raster(p, "input_blue")).transpose()?;
        let dem_raw = load_raster(dem_path, "input_dem")?;

        let nir = harmonize_to_reference(
            nir_raw,
            "input_nir",
            &red,
            "input_red",
            ctx,
            wbraster::ResampleMethod::Bilinear,
        )?;
        let green = green_raw
            .map(|g| {
                harmonize_to_reference(
                    g,
                    "input_green",
                    &red,
                    "input_red",
                    ctx,
                    wbraster::ResampleMethod::Bilinear,
                )
            })
            .transpose()?;
        let blue = blue_raw
            .map(|b| {
                harmonize_to_reference(
                    b,
                    "input_blue",
                    &red,
                    "input_red",
                    ctx,
                    wbraster::ResampleMethod::Bilinear,
                )
            })
            .transpose()?;
        let dem_harmonized = harmonize_to_reference(
            dem_raw,
            "input_dem",
            &red,
            "input_red",
            ctx,
            wbraster::ResampleMethod::Bilinear,
        )?;

        let qa_mask_harmonized = qa_mask_path
            .as_deref()
            .map(|p| load_raster(p, "qa_mask"))
            .transpose()?
            .map(|q| {
                harmonize_to_reference(
                    q,
                    "qa_mask",
                    &red,
                    "input_red",
                    ctx,
                    wbraster::ResampleMethod::Nearest,
                )
            })
            .transpose()?;

        let qa_mask_format_detected = if let Some(ref q) = qa_mask_harmonized {
            match qa_mask_format_requested {
                QaMaskFormat::Auto => qa_mask_auto_format_hint.unwrap_or_else(|| infer_qa_mask_format(q)),
                other => other,
            }
        } else {
            QaMaskFormat::Auto
        };

        let n = red.rows * red.cols;

        // Validate NIR dimensions match.
        if nir.rows != red.rows || nir.cols != red.cols {
            return Err(ToolError::Execution(
                "NIR and red bands must have identical row/col dimensions".to_string(),
            ));
        }

        let mut resolved_latitude = latitude_arg;
        let mut resolved_longitude = longitude_arg;

        let (solar_zenith_deg, solar_azimuth_deg, solar_resolution_source) = match solar_mode {
            SolarResolveMode::Manual => {
                let z = solar_zenith_manual.ok_or_else(|| {
                    ToolError::Validation("parameter 'solar_zenith_deg' is required in manual solar_mode".to_string())
                })?;
                let a = solar_azimuth_manual.ok_or_else(|| {
                    ToolError::Validation("parameter 'solar_azimuth_deg' is required in manual solar_mode".to_string())
                })?;
                (z, a, "manual".to_string())
            }
            SolarResolveMode::Metadata => {
                if let Some((z, a)) = extract_solar_from_metadata(&red) {
                    (z, a, "metadata".to_string())
                } else if let Some(bundle) = resolved_bundle.as_ref() {
                    if let Some((z, a)) = extract_solar_from_bundle(bundle) {
                        (z, a, "bundle_metadata".to_string())
                    } else {
                        return Err(ToolError::Validation(
                            "solar_mode=metadata but no usable solar angles were found in raster metadata or bundle metadata"
                                .to_string(),
                        ));
                    }
                } else {
                    return Err(ToolError::Validation(
                        "solar_mode=metadata but no usable solar angles were found in raster metadata"
                            .to_string(),
                    ));
                }
            }
            SolarResolveMode::DatetimeLocation => {
                let dt_text = acquisition_datetime_utc.as_deref().ok_or_else(|| {
                    ToolError::Validation(
                        "solar_mode=datetime_location requires 'acquisition_datetime_utc'"
                            .to_string(),
                    )
                })?;
                let dt = OffsetDateTime::parse(dt_text, &Rfc3339).map_err(|e| {
                    ToolError::Validation(format!(
                        "parameter 'acquisition_datetime_utc' must be RFC3339 UTC: {e}"
                    ))
                })?;
                let (lon, lat) = if let (Some(lat), Some(lon)) = (latitude_arg, longitude_arg) {
                    (lon, lat)
                } else {
                    let (lon, lat) = raster_centroid_lonlat(&red)?;
                    resolved_latitude = Some(lat);
                    resolved_longitude = Some(lon);
                    (lon, lat)
                };
                let (z, a) = solar_position_from_utc(lat, lon, dt);
                (z, a, "datetime_location".to_string())
            }
            SolarResolveMode::Auto => {
                if let (Some(z), Some(a)) = (solar_zenith_manual, solar_azimuth_manual) {
                    (z, a, "manual".to_string())
                } else if let Some((z, a)) = extract_solar_from_metadata(&red) {
                    (z, a, "metadata".to_string())
                } else if let Some(bundle) = resolved_bundle.as_ref() {
                    if let Some((z, a)) = extract_solar_from_bundle(bundle) {
                        (z, a, "bundle_metadata".to_string())
                    } else if let Some(dt_text) = acquisition_datetime_utc.as_deref() {
                        let dt = OffsetDateTime::parse(dt_text, &Rfc3339).map_err(|e| {
                            ToolError::Validation(format!(
                                "parameter 'acquisition_datetime_utc' must be RFC3339 UTC: {e}"
                            ))
                        })?;
                        let (lon, lat) = if let (Some(lat), Some(lon)) = (latitude_arg, longitude_arg) {
                            (lon, lat)
                        } else {
                            let (lon, lat) = raster_centroid_lonlat(&red)?;
                            resolved_latitude = Some(lat);
                            resolved_longitude = Some(lon);
                            (lon, lat)
                        };
                        let (z, a) = solar_position_from_utc(lat, lon, dt);
                        (z, a, "datetime_location".to_string())
                    } else {
                        return Err(ToolError::Validation(
                            "unable to resolve solar angles in auto mode. Provide solar_zenith_deg/solar_azimuth_deg, raster metadata with solar fields, bundle metadata, or acquisition_datetime_utc (+ optional latitude/longitude)"
                                .to_string(),
                        ));
                    }
                } else if let Some(dt_text) = acquisition_datetime_utc.as_deref() {
                    let dt = OffsetDateTime::parse(dt_text, &Rfc3339).map_err(|e| {
                        ToolError::Validation(format!(
                            "parameter 'acquisition_datetime_utc' must be RFC3339 UTC: {e}"
                        ))
                    })?;
                    let (lon, lat) = if let (Some(lat), Some(lon)) = (latitude_arg, longitude_arg) {
                        (lon, lat)
                    } else {
                        let (lon, lat) = raster_centroid_lonlat(&red)?;
                        resolved_latitude = Some(lat);
                        resolved_longitude = Some(lon);
                        (lon, lat)
                    };
                    let (z, a) = solar_position_from_utc(lat, lon, dt);
                    (z, a, "datetime_location".to_string())
                } else {
                    return Err(ToolError::Validation(
                        "unable to resolve solar angles in auto mode. Provide solar_zenith_deg/solar_azimuth_deg, raster metadata with solar fields, bundle metadata, or acquisition_datetime_utc (+ optional latitude/longitude)"
                            .to_string(),
                    ));
                }
            }
        };

        if !(0.0..=90.0).contains(&solar_zenith_deg) {
            return Err(ToolError::Validation(
                format!("resolved solar_zenith_deg={solar_zenith_deg:.3} is outside [0, 90]"),
            ));
        }

        let solar_zenith_rad = deg2rad(solar_zenith_deg);
        let solar_azimuth_rad = deg2rad(solar_azimuth_deg);
        let cos_z = solar_zenith_rad.cos();

        let (radiometric_scale, scale_mode) = infer_radiometric_scale(&red, &nir);
        let cloud_threshold = cloud_threshold_override
            .unwrap_or(ps.cloud_threshold_fraction * radiometric_scale);
        let shadow_threshold = shadow_threshold_override
            .unwrap_or(ps.cloud_shadow_threshold_fraction * radiometric_scale);
        if shadow_threshold >= cloud_threshold {
            return Err(ToolError::Validation(
                "resolved cloud/shadow thresholds are invalid: shadow must be less than cloud"
                    .to_string(),
            ));
        }
        ctx.progress.info(&format!(
            "terrain_corrected_optical_analytics: cloud/shadow thresholds resolved to {:.3}/{:.3} ({})",
            cloud_threshold, shadow_threshold, scale_mode
        ));
        ctx.progress.info(&format!(
            "terrain_corrected_optical_analytics: solar angles resolved from {} (zenith={:.3}, azimuth={:.3})",
            solar_resolution_source, solar_zenith_deg, solar_azimuth_deg
        ));

        if qa_mask_harmonized.is_some() {
            ctx.progress.info(&format!(
                "terrain_corrected_optical_analytics: QA mask enabled with format={} strategy={}",
                qa_mask_format_detected.as_str(),
                mask_strategy.as_str()
            ));
        }
        stage_timings_ms.insert(
            "step1_load_and_harmonize".to_string(),
            step1_t0.elapsed().as_secs_f64() * 1_000.0,
        );

        // ── Step 2: Compute slope and aspect from DEM ────────────────────────
        ctx.progress.info("terrain_corrected_optical_analytics: step 2/5 – slope and aspect from DEM");
        let step2_t0 = Instant::now();

        let (slope_raster, aspect_raster) = slope_aspect_from_dem(&dem_harmonized, z_factor)?;
        stage_timings_ms.insert(
            "step2_slope_aspect".to_string(),
            step2_t0.elapsed().as_secs_f64() * 1_000.0,
        );

        // ── Step 3: Cloud/shadow detection and correction factor ─────────────
        ctx.progress.info("terrain_corrected_optical_analytics: step 3/5 – cloud/shadow masking and cos(i) computation");
        let step3_t0 = Instant::now();

        // Correction factor raster (cos(i) / cos(z)).
        let mut correction_factor = red.clone();
        let mut cloud_shadow_mask = red.clone();
        let nodata_r = red.nodata;

        // Collect cos_i / reflectance pairs for regression (sampled for efficiency).
        let mut sample_cos_i: Vec<f64> = Vec::new();
        let mut sample_red_refl: Vec<f64> = Vec::new();
        let mut sample_nir_refl: Vec<f64> = Vec::new();

        let mut cloud_count: usize = 0;
        let mut shadow_count: usize = 0;
        let mut valid_count: usize = 0;
        let mut heuristic_cloud_count: usize = 0;
        let mut heuristic_shadow_count: usize = 0;
        let mut qa_cloud_count: usize = 0;
        let mut qa_shadow_count: usize = 0;

        for i in 0..n {
            let rv = red.data.get_f64(i);
            let nv = nir.data.get_f64(i);

            if red.is_nodata(rv) || nir.is_nodata(nv) {
                correction_factor.data.set_f64(i, nodata_r);
                cloud_shadow_mask.data.set_f64(i, nodata_r);
                continue;
            }

            let heuristic_cloud = rv > cloud_threshold && nv > cloud_threshold;
            let heuristic_shadow = !heuristic_cloud && rv < shadow_threshold && nv < shadow_threshold;
            if heuristic_cloud {
                heuristic_cloud_count += 1;
            } else if heuristic_shadow {
                heuristic_shadow_count += 1;
            }

            let qa_flags = qa_mask_harmonized.as_ref().and_then(|qa| {
                let qv = qa.data.get_f64(i);
                qa_mask_to_flags(qv, qa.nodata, qa_mask_format_detected)
            });
            if let Some((qc, qs)) = qa_flags {
                if qc {
                    qa_cloud_count += 1;
                }
                if qs {
                    qa_shadow_count += 1;
                }
            }

            let (final_cloud, final_shadow) = match mask_strategy {
                MaskStrategy::HeuristicOnly => (heuristic_cloud, heuristic_shadow),
                MaskStrategy::QaOnly => qa_flags.unwrap_or((false, false)),
                MaskStrategy::QaPlusHeuristic => {
                    let (qc, qs) = qa_flags.unwrap_or((false, false));
                    let cloud = qc || heuristic_cloud;
                    let shadow = !cloud && (qs || heuristic_shadow);
                    (cloud, shadow)
                }
                MaskStrategy::Auto => {
                    if let Some((qc, qs)) = qa_flags {
                        (qc, qs)
                    } else {
                        (heuristic_cloud, heuristic_shadow)
                    }
                }
            };

            let mask_val = mask_value_from_flags(final_cloud, final_shadow);
            if final_cloud {
                cloud_count += 1;
            } else if final_shadow {
                shadow_count += 1;
            } else {
                valid_count += 1;
            }
            cloud_shadow_mask.data.set_f64(i, mask_val);

            let sv = slope_raster.data.get_f64(i);
            let av = aspect_raster.data.get_f64(i);

            if slope_raster.is_nodata(sv) || aspect_raster.is_nodata(av) {
                correction_factor.data.set_f64(i, 1.0);
                continue;
            }

            let slope_rad = deg2rad(sv);
            let aspect_rad = deg2rad(av);
            let cos_i = cos_incidence(slope_rad, aspect_rad, solar_zenith_rad, solar_azimuth_rad)
                .max(-1.0)
                .min(1.0);

            let cf = if cos_i.abs() < ps.min_cos_i { 1.0 } else { cos_z / cos_i.max(1e-6) };
            correction_factor.data.set_f64(i, cf);

            // Sample reflectance/cos_i pairs for regression.
            if mask_val < 1.0 && cos_i >= ps.min_cos_i && (i % ps.regression_sample_step) == 0 {
                sample_cos_i.push(cos_i);
                sample_red_refl.push(rv);
                sample_nir_refl.push(nv);
            }
        }
        stage_timings_ms.insert(
            "step3_mask_and_cos_i".to_string(),
            step3_t0.elapsed().as_secs_f64() * 1_000.0,
        );

        // ── Step 4: C-correction regression and reflectance correction ────────
        ctx.progress.info("terrain_corrected_optical_analytics: step 4/5 – C-correction and reflectance stack");
        let step4_t0 = Instant::now();

        let reg_red = ols(&sample_cos_i, &sample_red_refl);
        let reg_nir = ols(&sample_cos_i, &sample_nir_refl);
        let c_red = c_correction_factor(reg_red.m, reg_red.b).unwrap_or(1.0);
        let c_nir = c_correction_factor(reg_nir.m, reg_nir.b).unwrap_or(1.0);

        let mut corrected_red = red.clone();
        let mut corrected_nir = nir.clone();
        let mut corrected_green = green.as_ref().cloned();
        let mut corrected_blue = blue.as_ref().cloned();
        let mut quality_confidence = red.clone();

        let c_green = if let Some(ref g) = green {
            let mut sg: Vec<f64> = Vec::with_capacity(sample_cos_i.len());
            for (step_idx, &ci) in sample_cos_i.iter().enumerate() {
                let _ = ci;
                let flat_idx = step_idx * ps.regression_sample_step;
                if flat_idx < n {
                    let gv = g.data.get_f64(flat_idx);
                    if !g.is_nodata(gv) { sg.push(gv); }
                }
            }
            let reg_green = ols(&sample_cos_i[..sg.len().min(sample_cos_i.len())], &sg);
            c_correction_factor(reg_green.m, reg_green.b).unwrap_or(1.0)
        } else {
            1.0
        };

        let c_blue = if let Some(ref b) = blue {
            let mut sb: Vec<f64> = Vec::with_capacity(sample_cos_i.len());
            for (step_idx, &ci) in sample_cos_i.iter().enumerate() {
                let _ = ci;
                let flat_idx = step_idx * ps.regression_sample_step;
                if flat_idx < n {
                    let bv = b.data.get_f64(flat_idx);
                    if !b.is_nodata(bv) { sb.push(bv); }
                }
            }
            let reg_blue = ols(&sample_cos_i[..sb.len().min(sample_cos_i.len())], &sb);
            c_correction_factor(reg_blue.m, reg_blue.b).unwrap_or(1.0)
        } else {
            1.0
        };

        let total_valid = (valid_count + cloud_count + shadow_count).max(1);
        let clear_fraction = valid_count as f64 / total_valid as f64;

        #[derive(Clone, Copy)]
        struct CorrectedCell {
            cr: f64,
            cn: f64,
            cg: f64,
            cb: f64,
            conf: f64,
        }

        let corrected_cells: Vec<CorrectedCell> = (0..n)
            .into_par_iter()
            .map(|i| {
                let rv = red.data.get_f64(i);
                let nv = nir.data.get_f64(i);
                let mask_val = cloud_shadow_mask.data.get_f64(i);

                let default_g = green
                    .as_ref()
                    .map(|g| g.data.get_f64(i))
                    .unwrap_or(nodata_r);
                let default_b = blue
                    .as_ref()
                    .map(|b| b.data.get_f64(i))
                    .unwrap_or(nodata_r);

                if red.is_nodata(rv)
                    || nir.is_nodata(nv)
                    || (mask_val - nodata_r).abs() < f64::EPSILON
                {
                    return CorrectedCell {
                        cr: nodata_r,
                        cn: nodata_r,
                        cg: nodata_r,
                        cb: nodata_r,
                        conf: nodata_r,
                    };
                }

                let sv = slope_raster.data.get_f64(i);
                let av = aspect_raster.data.get_f64(i);
                let has_geom = !(slope_raster.is_nodata(sv) || aspect_raster.is_nodata(av));
                let cos_i = if has_geom {
                    let slope_rad = deg2rad(sv);
                    let aspect_rad = deg2rad(av);
                    cos_incidence(slope_rad, aspect_rad, solar_zenith_rad, solar_azimuth_rad)
                        .max(-1.0)
                        .min(1.0)
                } else {
                    0.0
                };

                // Don't correct cloud/shadow pixels — they remain as-is (masked out downstream).
                let (cr, cn) = if mask_val > 0.5 {
                    (rv, nv)
                } else if !has_geom || cos_i < ps.min_cos_i {
                    (rv, nv)
                } else {
                    let wr = ps.correction_weight_red;
                    let wn = ps.correction_weight_nir;
                    let cr_raw = apply_c_correction(rv, cos_z, cos_i, c_red);
                    let cn_raw = apply_c_correction(nv, cos_z, cos_i, c_nir);
                    (rv * (1.0 - wr) + cr_raw * wr, nv * (1.0 - wn) + cn_raw * wn)
                };

                let cg = if let Some(ref g) = green {
                    let gv = g.data.get_f64(i);
                    if mask_val > 0.5 || g.is_nodata(gv) || !has_geom || cos_i < ps.min_cos_i {
                        gv
                    } else {
                        let wg = (ps.correction_weight_red + ps.correction_weight_nir) * 0.5;
                        let cg_raw = apply_c_correction(gv, cos_z, cos_i, c_green);
                        gv * (1.0 - wg) + cg_raw * wg
                    }
                } else {
                    default_g
                };

                let cb = if let Some(ref b) = blue {
                    let bv = b.data.get_f64(i);
                    if mask_val > 0.5 || b.is_nodata(bv) || !has_geom || cos_i < ps.min_cos_i {
                        bv
                    } else {
                        let wb = (ps.correction_weight_red + ps.correction_weight_nir) * 0.5;
                        let cb_raw = apply_c_correction(bv, cos_z, cos_i, c_blue);
                        bv * (1.0 - wb) + cb_raw * wb
                    }
                } else {
                    default_b
                };

                // Quality confidence: penalise clouds, shadows, low cos_i.
                let conf = if mask_val > 0.5 {
                    0.1
                } else if !has_geom {
                    0.7
                } else {
                    // High cos_i → well-illuminated → high confidence.
                    (cos_i / cos_z.max(0.01)).clamp(0.0, 1.0) * 0.85 + 0.10
                };

                CorrectedCell { cr, cn, cg, cb, conf }
            })
            .collect();

        corrected_red.data.par_fill_with(|i| corrected_cells[i].cr);
        corrected_nir.data.par_fill_with(|i| corrected_cells[i].cn);
        quality_confidence
            .data
            .par_fill_with(|i| corrected_cells[i].conf);
        if let Some(ref mut cg) = corrected_green {
            cg.data.par_fill_with(|i| corrected_cells[i].cg);
        }
        if let Some(ref mut cb) = corrected_blue {
            cb.data.par_fill_with(|i| corrected_cells[i].cb);
        }
        stage_timings_ms.insert(
            "step4_correction_and_commit".to_string(),
            step4_t0.elapsed().as_secs_f64() * 1_000.0,
        );

        // ── Step 5: Write outputs ────────────────────────────────────────────
        ctx.progress.info("terrain_corrected_optical_analytics: step 5/5 – writing outputs");
        let step5_t0 = Instant::now();

        let red_out = format!("{}_red_corrected.tif", output_prefix);
        let nir_out = format!("{}_nir_corrected.tif", output_prefix);
        let mask_out = format!("{}_cloud_shadow_mask.tif", output_prefix);
        let cf_out = format!("{}_topographic_correction_factor.tif", output_prefix);
        let qc_out = format!("{}_quality_confidence.tif", output_prefix);
        let stack_out = format!("{}_surface_reflectance_stack.tif", output_prefix);
        let summary_out = format!("{}_summary.json", output_prefix);

        write_raster(&corrected_red, &red_out, "corrected_red")?;
        write_raster(&corrected_nir, &nir_out, "corrected_nir")?;
        write_raster(&cloud_shadow_mask, &mask_out, "cloud_shadow_mask")?;
        write_raster(&correction_factor, &cf_out, "topographic_correction_factor")?;
        write_raster(&quality_confidence, &qc_out, "quality_confidence")?;

        let mut green_out: Option<String> = None;
        if let Some(ref cg) = corrected_green {
            let gp = format!("{}_green_corrected.tif", output_prefix);
            write_raster(cg, &gp, "corrected_green")?;
            green_out = Some(gp);
        }

        let mut blue_out: Option<String> = None;
        if let Some(ref cb) = corrected_blue {
            let bp = format!("{}_blue_corrected.tif", output_prefix);
            write_raster(cb, &bp, "corrected_blue")?;
            blue_out = Some(bp);
        }

        let mut stack_band_order = vec!["red", "nir"];
        if corrected_green.is_some() {
            stack_band_order.push("green");
        }
        if corrected_blue.is_some() {
            stack_band_order.push("blue");
        }

        let reflectance_stack = build_reflectance_stack(
            &corrected_red,
            &corrected_nir,
            corrected_green.as_ref(),
            corrected_blue.as_ref(),
        );
        write_raster(
            &reflectance_stack,
            &stack_out,
            "surface_reflectance_stack",
        )?;
        stage_timings_ms.insert(
            "step5_write_outputs".to_string(),
            step5_t0.elapsed().as_secs_f64() * 1_000.0,
        );
        stage_timings_ms.insert(
            "total_runtime".to_string(),
            run_t0.elapsed().as_secs_f64() * 1_000.0,
        );

        let timings_summary = stage_timings_ms
            .iter()
            .map(|(k, v)| format!("{k}={v:.1}ms"))
            .collect::<Vec<_>>()
            .join(", ");
        ctx.progress
            .info(&format!("terrain_corrected_optical_analytics: timings {timings_summary}"));

        let generated_at_utc = current_utc_rfc3339();

        let qa_status = if clear_fraction < 0.5 {
            "poor_coverage"
        } else if clear_fraction < 0.8 {
            "partial_coverage"
        } else {
            "good_coverage"
        };

        let summary = json!({
            "workflow": "terrain_corrected_optical_analytics",
            "schema_version": SUMMARY_SCHEMA_VERSION,
            "generated_at_utc": generated_at_utc,
            "profile": profile,
            "inputs": {
                "bundle_root": bundle_root,
                "red": red_path,
                "nir": nir_path,
                "dem": dem_path,
                "green": green_path,
                "blue": blue_path,
                "qa_mask": qa_mask_path,
                "acquisition_datetime_utc": acquisition_datetime_utc,
            },
            "parameters": {
                "solar_mode": solar_mode.as_str(),
                "solar_resolution_source": solar_resolution_source,
                "solar_zenith_deg": solar_zenith_deg,
                "solar_azimuth_deg": solar_azimuth_deg,
                "latitude": resolved_latitude,
                "longitude": resolved_longitude,
                "z_factor": z_factor,
                "cos_solar_zenith": cos_z,
                "mask_strategy": mask_strategy.as_str(),
                "qa_mask_format_requested": qa_mask_format_requested.as_str(),
                "qa_mask_format_detected": qa_mask_format_detected.as_str(),
                "radiometric_scale_mode": scale_mode,
                "radiometric_scale": radiometric_scale,
                "cloud_threshold": cloud_threshold,
                "shadow_threshold": shadow_threshold,
                "cloud_threshold_override": cloud_threshold_override,
                "shadow_threshold_override": shadow_threshold_override,
                "bundle_sensor": resolved_bundle.as_ref().map(|b| b.sensor_name.clone()),
            },
            "regression": {
                "red": { "m": reg_red.m, "b": reg_red.b, "r_squared": reg_red.r_squared, "c_factor": c_red, "n_samples": reg_red.n },
                "nir": { "m": reg_nir.m, "b": reg_nir.b, "r_squared": reg_nir.r_squared, "c_factor": c_nir, "n_samples": reg_nir.n },
            },
            "summary": {
                "total_cells": n,
                "valid_cells": valid_count,
                "cloud_cells": cloud_count,
                "shadow_cells": shadow_count,
                "heuristic_cloud_cells": heuristic_cloud_count,
                "heuristic_shadow_cells": heuristic_shadow_count,
                "qa_cloud_cells": qa_cloud_count,
                "qa_shadow_cells": qa_shadow_count,
                "clear_fraction": clear_fraction,
                "status": qa_status,
            },
            "timings_ms": stage_timings_ms,
            "interpretation": {
                "qa_assessment": match qa_status {
                    "good_coverage" => "Clear-sky coverage is high. Topographic correction is reliable.",
                    "partial_coverage" => "Moderate cloud/shadow presence. Results are usable but correction reliability varies spatially.",
                    _ => "Extensive cloud or shadow coverage. Correction results should be applied cautiously.",
                },
            },
            "outputs": {
                "red_corrected": red_out,
                "nir_corrected": nir_out,
                "cloud_shadow_mask": mask_out,
                "topographic_correction_factor": cf_out,
                "quality_confidence": qc_out,
                "green_corrected": green_out,
                "blue_corrected": blue_out,
                "surface_reflectance_stack": stack_out,
                "surface_reflectance_stack_band_order": stack_band_order,
                "summary": summary_out,
            },
        });
        write_summary(&summary_out, &summary)?;

        let mut outputs = BTreeMap::new();
        outputs.insert("red_corrected".to_string(), json!(red_out));
        outputs.insert("nir_corrected".to_string(), json!(nir_out));
        outputs.insert("cloud_shadow_mask".to_string(), json!(mask_out));
        outputs.insert("topographic_correction_factor".to_string(), json!(cf_out));
        outputs.insert("quality_confidence".to_string(), json!(qc_out));
        outputs.insert("surface_reflectance_stack".to_string(), json!(stack_out));
        outputs.insert("summary".to_string(), json!(summary_out));
        if let Some(ref gp) = green_out {
            outputs.insert("green_corrected".to_string(), json!(gp));
        }
        if let Some(ref bp) = blue_out {
            outputs.insert("blue_corrected".to_string(), json!(bp));
        }
        Ok(ToolRunResult { outputs })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_has_free_tier() {
        let tool = TerrainCorrectedOpticalTool;
        let meta = tool.metadata();
        assert_eq!(meta.id, "terrain_corrected_optical_analytics");
        assert_eq!(meta.license_tier, LicenseTier::Open);
    }

    #[test]
    fn validation_rejects_bad_profile() {
        let tool = TerrainCorrectedOpticalTool;
        let mut args = ToolArgs::new();
        args.insert("input_red".to_string(), json!("red.tif"));
        args.insert("input_nir".to_string(), json!("nir.tif"));
        args.insert("input_dem".to_string(), json!("dem.tif"));
        args.insert("solar_zenith_deg".to_string(), json!(40.0));
        args.insert("solar_azimuth_deg".to_string(), json!(165.0));
        args.insert("profile".to_string(), json!("ultra_fast"));
        assert!(tool.validate(&args).is_err());
    }

    #[test]
    fn validation_rejects_zenith_out_of_range() {
        let tool = TerrainCorrectedOpticalTool;
        let mut args = ToolArgs::new();
        args.insert("input_red".to_string(), json!("red.tif"));
        args.insert("input_nir".to_string(), json!("nir.tif"));
        args.insert("input_dem".to_string(), json!("dem.tif"));
        args.insert("solar_zenith_deg".to_string(), json!(95.0));
        args.insert("solar_azimuth_deg".to_string(), json!(165.0));
        assert!(tool.validate(&args).is_err());
    }

    #[test]
    fn validation_accepts_valid_args() {
        let tool = TerrainCorrectedOpticalTool;
        let mut args = ToolArgs::new();
        args.insert("input_red".to_string(), json!("red.tif"));
        args.insert("input_nir".to_string(), json!("nir.tif"));
        args.insert("input_dem".to_string(), json!("dem.tif"));
        args.insert("solar_zenith_deg".to_string(), json!(38.5));
        args.insert("solar_azimuth_deg".to_string(), json!(172.0));
        assert!(tool.validate(&args).is_ok());
    }

    #[test]
    fn validation_accepts_bundle_root_without_explicit_red_nir() {
        let tool = TerrainCorrectedOpticalTool;
        let mut args = ToolArgs::new();
        args.insert("bundle_root".to_string(), json!("scene.SAFE"));
        args.insert("input_dem".to_string(), json!("dem.tif"));
        assert!(tool.validate(&args).is_ok());
    }

    #[test]
    fn cos_incidence_flat_is_cos_zenith() {
        // On a flat surface (slope=0), cos(i) should equal cos(solar_zenith).
        let slope_rad = 0.0_f64;
        let aspect_rad = 0.0_f64;
        let solar_zenith_rad = deg2rad(40.0);
        let solar_azimuth_rad = deg2rad(165.0);
        let ci = cos_incidence(slope_rad, aspect_rad, solar_zenith_rad, solar_azimuth_rad);
        let expected = solar_zenith_rad.cos();
        assert!((ci - expected).abs() < 1e-10, "expected {expected}, got {ci}");
    }

    #[test]
    fn ols_exact_linear_fit() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y: Vec<f64> = x.iter().map(|xi| 2.0 * xi + 3.0).collect();
        let res = ols(&x, &y);
        assert!((res.m - 2.0).abs() < 1e-10, "slope mismatch");
        assert!((res.b - 3.0).abs() < 1e-10, "intercept mismatch");
        assert!((res.r_squared - 1.0).abs() < 1e-10, "r² should be 1.0");
    }
}
