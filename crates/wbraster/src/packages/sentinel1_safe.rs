//! Sentinel-1 SAFE package reader.
//!
//! This module provides package-level discovery for Sentinel-1 SAFE products.
//! It focuses on measurement raster discovery while leaving pixel decoding to
//! the existing raster readers (typically GeoTIFF).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::error::{RasterError, Result};
use crate::raster::{DataType, Raster, RasterConfig};

/// Sentinel-1 radiometric calibration target derived from calibration LUTs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sentinel1CalibrationTarget {
    /// Sigma-nought backscatter in linear power units.
    SigmaNought,
    /// Beta-nought backscatter in linear power units.
    BetaNought,
    /// Gamma backscatter in linear power units.
    Gamma,
}

/// One Sentinel-1 calibration vector row parsed from calibration XML.
#[derive(Debug, Clone, PartialEq)]
pub struct Sentinel1CalibrationVector {
    /// Image line index for this vector.
    pub line: usize,
    /// Sample locations covered by this vector.
    pub pixels: Vec<usize>,
    /// Sigma-nought LUT values aligned with `pixels`.
    pub sigma_nought: Vec<f64>,
    /// Beta-nought LUT values aligned with `pixels`.
    pub beta_nought: Vec<f64>,
    /// Gamma LUT values aligned with `pixels`.
    pub gamma: Vec<f64>,
    /// DN LUT values aligned with `pixels` when present.
    pub dn: Vec<f64>,
}

/// Parsed Sentinel-1 calibration LUT for a measurement asset.
#[derive(Debug, Clone, PartialEq)]
pub struct Sentinel1CalibrationLut {
    /// Calibration vectors ordered by image line.
    pub vectors: Vec<Sentinel1CalibrationVector>,
}

impl Sentinel1CalibrationLut {
    /// Interpolate a calibration LUT value for the given image row/column.
    pub fn interpolated_value(
        &self,
        row: usize,
        col: usize,
        target: Sentinel1CalibrationTarget,
    ) -> Option<f64> {
        if self.vectors.is_empty() {
            return None;
        }

        let upper_line_idx = self.vectors.partition_point(|v| v.line < row);
        let lower_line_idx = upper_line_idx.saturating_sub(1);
        let upper_line_idx = upper_line_idx.min(self.vectors.len().saturating_sub(1));

        let lower = &self.vectors[lower_line_idx];
        let upper = &self.vectors[upper_line_idx];
        let lower_value = interpolate_vector_value(lower, col, target)?;
        let upper_value = interpolate_vector_value(upper, col, target)?;

        if lower.line == upper.line {
            return Some(lower_value);
        }

        Some(interpolate_between_samples(
            lower.line,
            lower_value,
            upper.line,
            upper_value,
            row,
        ))
    }
}

/// One Sentinel-1 noise vector row parsed from noise XML.
#[derive(Debug, Clone, PartialEq)]
pub struct Sentinel1NoiseVector {
    /// Image line index for this vector.
    pub line: usize,
    /// Sample locations covered by this vector.
    pub pixels: Vec<usize>,
    /// Thermal noise LUT values aligned with `pixels`.
    pub noise: Vec<f64>,
}

/// Parsed Sentinel-1 thermal noise LUT for a measurement asset.
#[derive(Debug, Clone, PartialEq)]
pub struct Sentinel1NoiseLut {
    /// Noise vectors ordered by image line.
    pub vectors: Vec<Sentinel1NoiseVector>,
}

impl Sentinel1NoiseLut {
    /// Interpolate noise LUT value for the given image row/column.
    pub fn interpolated_value(&self, row: usize, col: usize) -> Option<f64> {
        if self.vectors.is_empty() {
            return None;
        }

        let upper_line_idx = self.vectors.partition_point(|v| v.line < row);
        let lower_line_idx = upper_line_idx.saturating_sub(1);
        let upper_line_idx = upper_line_idx.min(self.vectors.len().saturating_sub(1));

        let lower = &self.vectors[lower_line_idx];
        let upper = &self.vectors[upper_line_idx];
        let lower_value = interpolate_along_pixels(&lower.pixels, &lower.noise, col)?;
        let upper_value = interpolate_along_pixels(&upper.pixels, &upper.noise, col)?;

        if lower.line == upper.line {
            return Some(lower_value);
        }

        Some(interpolate_between_samples(
            lower.line,
            lower_value,
            upper.line,
            upper_value,
            row,
        ))
    }
}

/// A single state vector from a Sentinel-1 annotation orbit list.
///
/// Coordinates are in an Earth-Centred Earth-Fixed (ECEF) reference frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Sentinel1OrbitVector {
    /// UTC time string for this state vector (ISO-8601).
    pub time: String,
    /// ECEF position in metres as `[x, y, z]`.
    pub position: [f64; 3],
    /// ECEF velocity in metres per second as `[x, y, z]`.
    pub velocity: [f64; 3],
}

/// A single geolocation grid point from the Sentinel-1 annotation XML.
///
/// The geolocation grid provides sparse (line, pixel) → (lat, lon, height,
/// incidence angle) mappings for precise geolocation and radiometric analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct Sentinel1GeolocationGridPoint {
    /// Image line index.
    pub line: usize,
    /// Image pixel (column) index.
    pub pixel: usize,
    /// Geodetic latitude in decimal degrees (WGS84).
    pub latitude: f64,
    /// Geodetic longitude in decimal degrees (WGS84).
    pub longitude: f64,
    /// Ellipsoidal height above WGS84 in metres.
    pub height: f64,
    /// Local incidence angle in degrees (angle from vertical to range direction).
    pub incidence_angle: f64,
    /// Elevation angle in degrees above horizontal.
    pub elevation_angle: f64,
}

/// Sentinel-1 geolocation grid parsed from an annotation XML.
///
/// The grid is sparsely sampled across image lines and pixels; all
/// interpolation methods use bilinear interpolation over the irregular grid.
#[derive(Debug, Clone, PartialEq)]
pub struct Sentinel1GeolocationGrid {
    /// Grid points sorted by `(line, pixel)`.
    pub points: Vec<Sentinel1GeolocationGridPoint>,
}

impl Sentinel1GeolocationGrid {
    /// Bilinearly interpolate `(latitude, longitude)` at the given image `(row, col)`.
    pub fn interpolated_lat_lon(&self, row: usize, col: usize) -> Option<(f64, f64)> {
        let lat = self.interpolate_field(row, col, |p| p.latitude)?;
        let lon = self.interpolate_field(row, col, |p| p.longitude)?;
        Some((lat, lon))
    }

    /// Bilinearly interpolate ellipsoidal height in metres at `(row, col)`.
    pub fn interpolated_height(&self, row: usize, col: usize) -> Option<f64> {
        self.interpolate_field(row, col, |p| p.height)
    }

    /// Bilinearly interpolate local incidence angle in degrees at `(row, col)`.
    pub fn interpolated_incidence_angle(&self, row: usize, col: usize) -> Option<f64> {
        self.interpolate_field(row, col, |p| p.incidence_angle)
    }

    /// Bilinearly interpolate elevation angle in degrees at `(row, col)`.
    pub fn interpolated_elevation_angle(&self, row: usize, col: usize) -> Option<f64> {
        self.interpolate_field(row, col, |p| p.elevation_angle)
    }

    fn interpolate_field<F>(&self, row: usize, col: usize, field: F) -> Option<f64>
    where
        F: Fn(&Sentinel1GeolocationGridPoint) -> f64,
    {
        if self.points.is_empty() {
            return None;
        }
        // Collect unique sorted line indices from the grid.
        let mut lines: Vec<usize> = self.points.iter().map(|p| p.line).collect();
        lines.sort_unstable();
        lines.dedup();

        let upper_idx = lines.partition_point(|&l| l < row);
        let lower_idx = upper_idx.saturating_sub(1);
        let upper_idx = upper_idx.min(lines.len().saturating_sub(1));

        let line0 = lines[lower_idx];
        let line1 = lines[upper_idx];

        let val0 = interpolate_grid_row(&self.points, line0, col, &field)?;
        let val1 = interpolate_grid_row(&self.points, line1, col, &field)?;
        if line0 == line1 {
            return Some(val0);
        }
        Some(interpolate_between_samples(line0, val0, line1, val1, row))
    }
}

/// One burst from a Sentinel-1 SLC TOPS annotation XML.
#[derive(Debug, Clone, PartialEq)]
pub struct Sentinel1Burst {
    /// UTC azimuth start time for the first line of this burst (ISO-8601).
    pub azimuth_time: String,
    /// Byte offset to the first sample of this burst in the measurement TIFF.
    pub byte_offset: u64,
    /// Number of image lines covered by this burst (from `<linesPerBurst>`).
    pub lines_per_burst: usize,
    /// Number of range samples per burst line (from `<samplesPerBurst>`).
    pub samples_per_burst: usize,
    /// First valid sample index per line; `-1` means the line is entirely invalid.
    pub first_valid_samples: Vec<i32>,
    /// Last valid sample index per line; `-1` means the line is entirely invalid.
    pub last_valid_samples: Vec<i32>,
}

/// Ordered list of bursts from a Sentinel-1 SLC TOPS annotation XML.
#[derive(Debug, Clone, PartialEq)]
pub struct Sentinel1BurstList {
    /// Lines per burst as declared in `<swathTiming/linesPerBurst>`.
    pub lines_per_burst: usize,
    /// Samples per burst line as declared in `<swathTiming/samplesPerBurst>`.
    pub samples_per_burst: usize,
    /// Bursts in acquisition order.
    pub bursts: Vec<Sentinel1Burst>,
}

/// A parsed Sentinel-1 SAFE package.
#[derive(Debug, Clone)]
pub struct Sentinel1SafePackage {
    /// Root SAFE directory path.
    pub safe_root: PathBuf,
    /// Product type extracted from `manifest.safe` when present.
    pub product_type: Option<String>,
    /// Acquisition mode extracted from `manifest.safe` when present.
    pub acquisition_mode: Option<String>,
    /// Polarization mode extracted from `manifest.safe` when present.
    pub polarization: Option<String>,
    /// Acquisition start time in UTC extracted from `manifest.safe` when present.
    pub acquisition_datetime_utc: Option<String>,
    /// Approximate geographic bounding box `[west, south, east, north]` in decimal
    /// degrees derived from footprint coordinates in `manifest.safe` when present.
    pub spatial_bounds: Option<[f64; 4]>,
    /// Canonical measurement key -> resolved raster path.
    pub measurements: BTreeMap<String, PathBuf>,
    /// Canonical annotation key -> resolved XML path.
    pub annotation: BTreeMap<String, PathBuf>,
    /// Canonical calibration key -> resolved XML path.
    pub calibration: BTreeMap<String, PathBuf>,
    /// Canonical noise key -> resolved XML path.
    pub noise: BTreeMap<String, PathBuf>,
}

impl Sentinel1SafePackage {
    /// Open and parse a Sentinel-1 SAFE package directory.
    pub fn open(safe_root: impl AsRef<Path>) -> Result<Self> {
        let safe_root = safe_root.as_ref().to_path_buf();
        if !safe_root.is_dir() {
            return Err(RasterError::Other(format!(
                "SAFE root is not a directory: {}",
                safe_root.display()
            )));
        }

        let manifest = safe_root.join("manifest.safe");
        if !manifest.is_file() {
            return Err(RasterError::MissingField(
                "manifest.safe (required for Sentinel-1 SAFE)".to_string(),
            ));
        }
        let manifest_text = fs::read_to_string(&manifest)?;

        let product_type = extract_first_text(
            &manifest_text,
            &["productType", "s1sarl1:productType", "s1sarl2:productType"],
        );
        let acquisition_mode = extract_first_text(
            &manifest_text,
            &[
                "mode",
                "s1sarl1:mode",
                "s1sarl2:mode",
                "instrumentMode",
                "s1sarl1:instrumentMode",
            ],
        );
        let polarization = extract_first_text(
            &manifest_text,
            &[
                "transmitterReceiverPolarisation",
                "polarisation",
                "s1sarl1:transmitterReceiverPolarisation",
            ],
        );
        let acquisition_datetime_utc = extract_first_text(
            &manifest_text,
            &[
                "startTime",
                "safe:startTime",
                "acquisitionStartTime",
            ],
        );
        let spatial_bounds = extract_tag_text(&manifest_text, "coordinates")
            .and_then(|coords| parse_gml_bounds(&coords));

        let mut files = Vec::new();
        collect_files_recursive(&safe_root, &mut files)?;

        let mut measurements = BTreeMap::new();
        let mut annotation = BTreeMap::new();
        let mut calibration = BTreeMap::new();
        let mut noise = BTreeMap::new();
        for path in files {
            if has_tiff_ext(&path) && is_measurement_path(&path) {
                let key = canonical_measurement_key(&path);
                measurements.insert(key, path);
                continue;
            }
            if has_xml_ext(&path) && is_noise_path(&path) {
                let key = canonical_asset_key(&path);
                noise.insert(key, path);
                continue;
            }
            if has_xml_ext(&path) && is_calibration_path(&path) {
                let key = canonical_asset_key(&path);
                calibration.insert(key, path);
                continue;
            }
            if has_xml_ext(&path) && is_annotation_path(&path) {
                let key = canonical_asset_key(&path);
                annotation.insert(key, path);
            }
        }

        if measurements.is_empty() {
            return Err(RasterError::MissingField(
                "no Sentinel-1 measurement TIFF files found in SAFE package".to_string(),
            ));
        }

        Ok(Self {
            safe_root,
            product_type,
            acquisition_mode,
            polarization,
            acquisition_datetime_utc,
            spatial_bounds,
            measurements,
            annotation,
            calibration,
            noise,
        })
    }

    /// List canonical measurement keys available in this package.
    pub fn list_measurement_keys(&self) -> Vec<String> {
        self.measurements.keys().cloned().collect()
    }

    /// List canonical annotation keys available in this package.
    pub fn list_annotation_keys(&self) -> Vec<String> {
        self.annotation.keys().cloned().collect()
    }

    /// List canonical calibration keys available in this package.
    pub fn list_calibration_keys(&self) -> Vec<String> {
        self.calibration.keys().cloned().collect()
    }

    /// List canonical noise keys available in this package.
    pub fn list_noise_keys(&self) -> Vec<String> {
        self.noise.keys().cloned().collect()
    }

    /// Resolve a canonical measurement key to a raster file path.
    pub fn measurement_path(&self, key: &str) -> Option<&Path> {
        self.measurements
            .get(&key.to_ascii_uppercase())
            .map(PathBuf::as_path)
    }

    /// Resolve a canonical annotation key to an XML path.
    pub fn annotation_path(&self, key: &str) -> Option<&Path> {
        self.annotation
            .get(&key.to_ascii_uppercase())
            .map(PathBuf::as_path)
    }

    /// Resolve a canonical calibration key to an XML path.
    pub fn calibration_path(&self, key: &str) -> Option<&Path> {
        self.calibration
            .get(&key.to_ascii_uppercase())
            .map(PathBuf::as_path)
    }

    /// Resolve a canonical noise key to an XML path.
    pub fn noise_path(&self, key: &str) -> Option<&Path> {
        self.noise
            .get(&key.to_ascii_uppercase())
            .map(PathBuf::as_path)
    }

    /// Read a canonical measurement directly as a [`Raster`].
    pub fn read_measurement(&self, key: &str) -> Result<Raster> {
        let p = self.measurement_path(key).ok_or_else(|| {
            RasterError::MissingField(format!("measurement '{}' not found in SAFE package", key))
        })?;
        Raster::read(p)
    }

    /// Read and parse the calibration LUT associated with a measurement key.
    pub fn read_calibration_lut(&self, key: &str) -> Result<Sentinel1CalibrationLut> {
        let path = self.calibration_path(key).ok_or_else(|| {
            RasterError::MissingField(format!("calibration '{}' not found in SAFE package", key))
        })?;
        let xml = fs::read_to_string(path)?;
        parse_calibration_lut(&xml)
    }

    /// Read a measurement and calibrate it to linear backscatter units.
    ///
    /// The calibrated output is computed as $(DN / LUT)^2$ using the selected
    /// calibration target and bilinear interpolation across calibration vectors.
    pub fn read_calibrated_measurement(
        &self,
        key: &str,
        target: Sentinel1CalibrationTarget,
    ) -> Result<Raster> {
        let measurement = self.read_measurement(key)?;
        let calibration = self.read_calibration_lut(key)?;
        calibrate_measurement_raster(&measurement, &calibration, target)
    }

    /// Read and parse the thermal noise LUT associated with a measurement key.
    pub fn read_noise_lut(&self, key: &str) -> Result<Sentinel1NoiseLut> {
        let path = self
            .noise_path(key)
            .ok_or_else(|| RasterError::MissingField(format!("noise '{}' not found in SAFE package", key)))?;
        let xml = fs::read_to_string(path)?;
        parse_noise_lut(&xml)
    }

    /// Read, calibrate, and thermal-noise-correct a measurement.
    ///
    /// Noise correction is applied in linear units as
    /// `max(0, calibrated - noise_lut)`.
    pub fn read_noise_corrected_calibrated_measurement(
        &self,
        key: &str,
        target: Sentinel1CalibrationTarget,
    ) -> Result<Raster> {
        let calibrated = self.read_calibrated_measurement(key, target)?;
        let noise = self.read_noise_lut(key)?;
        apply_noise_correction(&calibrated, &noise)
    }

    /// Read a measurement, calibrate it to linear backscatter, then convert to
    /// decibels using `10 × log₁₀(linear)`.
    ///
    /// Pixels where the calibrated value is ≤ 0 or equals nodata are written
    /// as nodata in the output.
    pub fn read_calibrated_measurement_db(
        &self,
        key: &str,
        target: Sentinel1CalibrationTarget,
    ) -> Result<Raster> {
        let linear = self.read_calibrated_measurement(key, target)?;
        linear_to_db(&linear)
    }

    /// Read, calibrate, noise-correct, and convert to decibels.
    ///
    /// Noise correction (`max(0, calibrated − noise)`) is applied in linear
    /// units before the dB conversion.  Zero or negative values are set to
    /// nodata.
    pub fn read_noise_corrected_calibrated_measurement_db(
        &self,
        key: &str,
        target: Sentinel1CalibrationTarget,
    ) -> Result<Raster> {
        let linear = self.read_noise_corrected_calibrated_measurement(key, target)?;
        linear_to_db(&linear)
    }

    /// Parse orbit state vectors from the annotation XML for the given key.
    ///
    /// Returns all `<orbit>` entries from the annotation's `<orbitList>`,
    /// each carrying an ECEF position (m) and velocity (m/s).
    pub fn read_orbit_vectors(&self, key: &str) -> Result<Vec<Sentinel1OrbitVector>> {
        let path = self.annotation_path(key).ok_or_else(|| {
            RasterError::MissingField(format!(
                "annotation '{}' not found in SAFE package",
                key
            ))
        })?;
        let xml = fs::read_to_string(path)?;
        parse_orbit_vectors(&xml)
    }

    /// Parse the geolocation grid from the annotation XML associated with `key`.
    ///
    /// The returned [`Sentinel1GeolocationGrid`] supports bilinear interpolation
    /// of latitude, longitude, height, incidence angle, and elevation angle at any
    /// image `(row, col)`.
    pub fn read_geolocation_grid(&self, key: &str) -> Result<Sentinel1GeolocationGrid> {
        let path = self.annotation_path(key).ok_or_else(|| {
            RasterError::MissingField(format!(
                "annotation '{}' not found in SAFE package",
                key
            ))
        })?;
        let xml = fs::read_to_string(path)?;
        parse_geolocation_grid(&xml)
    }

    /// Parse SLC TOPS burst metadata from the annotation XML associated with `key`.
    ///
    /// Returns `Err` if the annotation XML contains no `<swathTiming>` element,
    /// which is expected for GRD products where burst metadata is not applicable.
    pub fn read_burst_list(&self, key: &str) -> Result<Sentinel1BurstList> {
        let path = self.annotation_path(key).ok_or_else(|| {
            RasterError::MissingField(format!(
                "annotation '{}' not found in SAFE package",
                key
            ))
        })?;
        let xml = fs::read_to_string(path)?;
        parse_burst_list(&xml)
    }

    /// Return the unique polarization suffixes present among measurement keys.
    ///
    /// Keys follow the `MODE_PRODUCT_POL` convention, so this extracts and
    /// deduplicates the trailing `POL` component (e.g. `"VV"`, `"VH"`).
    pub fn list_polarizations(&self) -> Vec<String> {
        let mut pols: Vec<String> = self
            .measurements
            .keys()
            .filter_map(|k| k.rsplit('_').next().map(str::to_string))
            .collect();
        pols.sort();
        pols.dedup();
        pols
    }

    /// Read all available measurement rasters as a map of canonical key → [`Raster`].
    pub fn read_all_measurements(&self) -> Result<BTreeMap<String, Raster>> {
        let entries: Vec<(String, Raster)> = self
            .list_measurement_keys()
            .into_par_iter()
            .map(|key| Ok((key.clone(), self.read_measurement(&key)?)))
            .collect::<Result<Vec<_>>>()?;
        Ok(entries.into_iter().collect())
    }

    /// Read all measurement rasters whose polarization suffix matches `pol`.
    ///
    /// `pol` is matched case-insensitively (e.g. `"VV"`, `"vv"`, and `"Vv"`
    /// all match a key ending in `_VV`).
    pub fn read_measurements_for_polarization(
        &self,
        pol: &str,
    ) -> Result<BTreeMap<String, Raster>> {
        let pol_upper = pol.to_ascii_uppercase();
        let suffix = format!("_{pol_upper}");
        let keys: Vec<String> = self
            .list_measurement_keys()
            .into_iter()
            .filter(|k| k.ends_with(&suffix))
            .collect();
        let entries: Vec<(String, Raster)> = keys
            .into_par_iter()
            .map(|key| Ok((key.clone(), self.read_measurement(&key)?)))
            .collect::<Result<Vec<_>>>()?;
        Ok(entries.into_iter().collect())
    }

    /// Calibrate all measurements whose polarization matches `pol`.
    ///
    /// Returns a map of canonical key → calibrated [`Raster`] in linear backscatter
    /// units using the given radiometric `target`.
    pub fn read_calibrated_measurements_for_polarization(
        &self,
        pol: &str,
        target: Sentinel1CalibrationTarget,
    ) -> Result<BTreeMap<String, Raster>> {
        let pol_upper = pol.to_ascii_uppercase();
        let suffix = format!("_{pol_upper}");
        let keys: Vec<String> = self
            .list_measurement_keys()
            .into_iter()
            .filter(|k| k.ends_with(&suffix))
            .collect();
        let entries: Vec<(String, Raster)> = keys
            .into_par_iter()
            .map(|key| Ok((key.clone(), self.read_calibrated_measurement(&key, target)?)))
            .collect::<Result<Vec<_>>>()?;
        Ok(entries.into_iter().collect())
    }
}

fn has_tiff_ext(path: &Path) -> bool {
    path.extension()
        .map(|e| {
            let ext = e.to_string_lossy();
            ext.eq_ignore_ascii_case("tif") || ext.eq_ignore_ascii_case("tiff")
        })
        .unwrap_or(false)
}

fn has_xml_ext(path: &Path) -> bool {
    path.extension()
        .map(|e| e.to_string_lossy().eq_ignore_ascii_case("xml"))
        .unwrap_or(false)
}

fn is_measurement_path(path: &Path) -> bool {
    path.components().any(|c| {
        c.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("measurement")
    })
}

fn is_annotation_path(path: &Path) -> bool {
    path.components().any(|c| {
        c.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("annotation")
    })
}

fn is_calibration_path(path: &Path) -> bool {
    let mut has_annotation = false;
    let mut has_calibration = false;
    for c in path.components() {
        let s = c.as_os_str().to_string_lossy();
        if s.eq_ignore_ascii_case("annotation") {
            has_annotation = true;
        }
        if s.eq_ignore_ascii_case("calibration") {
            has_calibration = true;
        }
    }
    has_annotation && has_calibration
}

fn is_noise_path(path: &Path) -> bool {
    let mut has_annotation = false;
    let mut has_calibration = false;
    let mut has_noise = false;
    for c in path.components() {
        let s = c.as_os_str().to_string_lossy();
        if s.eq_ignore_ascii_case("annotation") {
            has_annotation = true;
        }
        if s.eq_ignore_ascii_case("calibration") {
            has_calibration = true;
        }
        if s.eq_ignore_ascii_case("noise") {
            has_noise = true;
        }
    }
    has_annotation && has_calibration && has_noise
}

fn canonical_measurement_key(path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "measurement".to_string());

    let lower = stem.to_ascii_lowercase();
    let parts: Vec<&str> = lower.split('-').collect();
    if parts.len() >= 4 {
        let mode = parts[1].to_ascii_uppercase();
        let product = parts[2].to_ascii_uppercase();
        let pol = parts[3].to_ascii_uppercase();
        return format!("{}_{}_{}", mode, product, pol);
    }

    stem.to_ascii_uppercase()
}

fn canonical_asset_key(path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "asset".to_string());

    // Strip known type prefixes used in calibration/noise filenames before
    // parsing, so keys align with the matching measurement key.
    let lower = stem.to_ascii_lowercase();
    let stripped = lower
        .strip_prefix("calibration-")
        .or_else(|| lower.strip_prefix("noise-"))
        .unwrap_or(&lower);

    let parts: Vec<&str> = stripped.split('-').collect();
    if parts.len() >= 4 {
        let mode = parts[1].to_ascii_uppercase();
        let product = parts[2].to_ascii_uppercase();
        let pol = parts[3].to_ascii_uppercase();
        return format!("{}_{}_{}", mode, product, pol);
    }

    stem.to_ascii_uppercase()
}

fn collect_files_recursive(root: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            collect_files_recursive(&p, out)?;
        } else {
            out.push(p);
        }
    }
    Ok(())
}

fn parse_calibration_lut(xml: &str) -> Result<Sentinel1CalibrationLut> {
    let mut vectors = Vec::new();
    for block in extract_tag_blocks(xml, "calibrationVector") {
        let line = extract_tag_text(&block, "line")
            .ok_or_else(|| RasterError::MissingField("calibrationVector/line".to_string()))?
            .parse::<usize>()
            .map_err(|e| RasterError::Other(format!("invalid calibration line value: {e}")))?;
        let pixels = parse_usize_list(
            &extract_tag_text(&block, "pixel")
                .ok_or_else(|| RasterError::MissingField("calibrationVector/pixel".to_string()))?,
        )?;
        let sigma_nought = parse_f64_list(
            &extract_tag_text(&block, "sigmaNought")
                .ok_or_else(|| RasterError::MissingField("calibrationVector/sigmaNought".to_string()))?,
        )?;
        let beta_nought = parse_f64_list(
            &extract_tag_text(&block, "betaNought")
                .ok_or_else(|| RasterError::MissingField("calibrationVector/betaNought".to_string()))?,
        )?;
        let gamma = parse_f64_list(
            &extract_tag_text(&block, "gamma")
                .ok_or_else(|| RasterError::MissingField("calibrationVector/gamma".to_string()))?,
        )?;
        let dn = match extract_tag_text(&block, "dn") {
            Some(text) => parse_f64_list(&text)?,
            None => Vec::new(),
        };

        validate_calibration_vector_lengths(&pixels, &sigma_nought, "sigmaNought")?;
        validate_calibration_vector_lengths(&pixels, &beta_nought, "betaNought")?;
        validate_calibration_vector_lengths(&pixels, &gamma, "gamma")?;
        if !dn.is_empty() {
            validate_calibration_vector_lengths(&pixels, &dn, "dn")?;
        }

        vectors.push(Sentinel1CalibrationVector {
            line,
            pixels,
            sigma_nought,
            beta_nought,
            gamma,
            dn,
        });
    }

    if vectors.is_empty() {
        return Err(RasterError::MissingField(
            "no calibrationVector blocks found in calibration XML".to_string(),
        ));
    }

    vectors.sort_by_key(|v| v.line);
    Ok(Sentinel1CalibrationLut { vectors })
}

fn parse_noise_lut(xml: &str) -> Result<Sentinel1NoiseLut> {
    let mut vectors = Vec::new();
    for block in extract_tag_blocks(xml, "noiseVector") {
        let line = extract_tag_text(&block, "line")
            .ok_or_else(|| RasterError::MissingField("noiseVector/line".to_string()))?
            .parse::<usize>()
            .map_err(|e| RasterError::Other(format!("invalid noise line value: {e}")))?;
        let pixels = parse_usize_list(
            &extract_tag_text(&block, "pixel")
                .ok_or_else(|| RasterError::MissingField("noiseVector/pixel".to_string()))?,
        )?;
        let noise = parse_f64_list(
            &extract_tag_text(&block, "noiseLut")
                .or_else(|| extract_tag_text(&block, "noiseRangeLut"))
                .ok_or_else(|| {
                    RasterError::MissingField(
                        "noiseVector/noiseLut or noiseVector/noiseRangeLut".to_string(),
                    )
                })?,
        )?;
        validate_calibration_vector_lengths(&pixels, &noise, "noiseLut")?;

        vectors.push(Sentinel1NoiseVector { line, pixels, noise });
    }

    if vectors.is_empty() {
        return Err(RasterError::MissingField(
            "no noiseVector blocks found in noise XML".to_string(),
        ));
    }

    vectors.sort_by_key(|v| v.line);
    Ok(Sentinel1NoiseLut { vectors })
}

fn validate_calibration_vector_lengths(
    pixels: &[usize],
    values: &[f64],
    field_name: &str,
) -> Result<()> {
    if pixels.len() != values.len() {
        return Err(RasterError::Other(format!(
            "calibration vector length mismatch for {field_name}: {} pixels vs {} values",
            pixels.len(),
            values.len()
        )));
    }
    Ok(())
}

fn parse_usize_list(text: &str) -> Result<Vec<usize>> {
    text.split_whitespace()
        .map(|token| {
            token
                .parse::<usize>()
                .map_err(|e| RasterError::Other(format!("invalid integer list value '{token}': {e}")))
        })
        .collect()
}

fn parse_f64_list(text: &str) -> Result<Vec<f64>> {
    text.split_whitespace()
        .map(|token| {
            token
                .parse::<f64>()
                .map_err(|e| RasterError::Other(format!("invalid numeric list value '{token}': {e}")))
        })
        .collect()
}

fn calibrate_measurement_raster(
    measurement: &Raster,
    calibration: &Sentinel1CalibrationLut,
    target: Sentinel1CalibrationTarget,
) -> Result<Raster> {
    let mut output = Raster::new(RasterConfig {
        cols: measurement.cols,
        rows: measurement.rows,
        bands: measurement.bands,
        x_min: measurement.x_min,
        y_min: measurement.y_min,
        cell_size: measurement.cell_size_x,
        cell_size_y: Some(measurement.cell_size_y),
        nodata: measurement.nodata,
        data_type: DataType::F32,
        crs: measurement.crs.clone(),
        metadata: measurement.metadata.clone(),
    });

    output.metadata.push((
        "sentinel1_calibration_target".to_string(),
        match target {
            Sentinel1CalibrationTarget::SigmaNought => "sigma_nought",
            Sentinel1CalibrationTarget::BetaNought => "beta_nought",
            Sentinel1CalibrationTarget::Gamma => "gamma",
        }
        .to_string(),
    ));

    for band in 0..measurement.bands {
        for row in 0..measurement.rows {
            for col in 0..measurement.cols {
                let value = measurement.get(band as isize, row as isize, col as isize);
                if value == measurement.nodata {
                    continue;
                }

                let lut = calibration.interpolated_value(row, col, target).ok_or_else(|| {
                    RasterError::MissingField(format!(
                        "no calibration LUT value available for row {row}, col {col}"
                    ))
                })?;
                if !lut.is_finite() || lut == 0.0 {
                    return Err(RasterError::Other(format!(
                        "invalid calibration LUT value at row {row}, col {col}: {lut}"
                    )));
                }

                let calibrated = (value / lut).powi(2);
                output.set(band as isize, row as isize, col as isize, calibrated)?;
            }
        }
    }

    Ok(output)
}

fn apply_noise_correction(measurement: &Raster, noise: &Sentinel1NoiseLut) -> Result<Raster> {
    let mut output = Raster::new(RasterConfig {
        cols: measurement.cols,
        rows: measurement.rows,
        bands: measurement.bands,
        x_min: measurement.x_min,
        y_min: measurement.y_min,
        cell_size: measurement.cell_size_x,
        cell_size_y: Some(measurement.cell_size_y),
        nodata: measurement.nodata,
        data_type: DataType::F32,
        crs: measurement.crs.clone(),
        metadata: measurement.metadata.clone(),
    });
    output
        .metadata
        .push(("sentinel1_noise_corrected".to_string(), "true".to_string()));

    for band in 0..measurement.bands {
        for row in 0..measurement.rows {
            for col in 0..measurement.cols {
                let value = measurement.get(band as isize, row as isize, col as isize);
                if value == measurement.nodata {
                    continue;
                }
                let noise_value = noise.interpolated_value(row, col).ok_or_else(|| {
                    RasterError::MissingField(format!(
                        "no noise LUT value available for row {row}, col {col}"
                    ))
                })?;
                if !noise_value.is_finite() || noise_value < 0.0 {
                    return Err(RasterError::Other(format!(
                        "invalid noise LUT value at row {row}, col {col}: {noise_value}"
                    )));
                }
                let corrected = (value - noise_value).max(0.0);
                output.set(band as isize, row as isize, col as isize, corrected)?;
            }
        }
    }

    Ok(output)
}

fn interpolate_vector_value(
    vector: &Sentinel1CalibrationVector,
    col: usize,
    target: Sentinel1CalibrationTarget,
) -> Option<f64> {
    let values = match target {
        Sentinel1CalibrationTarget::SigmaNought => &vector.sigma_nought,
        Sentinel1CalibrationTarget::BetaNought => &vector.beta_nought,
        Sentinel1CalibrationTarget::Gamma => &vector.gamma,
    };
    interpolate_along_pixels(&vector.pixels, values, col)
}

fn interpolate_along_pixels(pixels: &[usize], values: &[f64], col: usize) -> Option<f64> {
    if pixels.is_empty() || values.is_empty() || pixels.len() != values.len() {
        return None;
    }
    if pixels.len() == 1 {
        return Some(values[0]);
    }

    let upper_idx = pixels.partition_point(|&pixel| pixel < col);
    let lower_idx = upper_idx.saturating_sub(1);
    let upper_idx = upper_idx.min(pixels.len().saturating_sub(1));

    let x0 = pixels[lower_idx];
    let x1 = pixels[upper_idx];
    let y0 = values[lower_idx];
    let y1 = values[upper_idx];
    if x0 == x1 {
        return Some(y0);
    }

    Some(interpolate_between_samples(x0, y0, x1, y1, col))
}

fn interpolate_between_samples(x0: usize, y0: f64, x1: usize, y1: f64, x: usize) -> f64 {
    if x0 == x1 {
        return y0;
    }
    let t = (x as f64 - x0 as f64) / (x1 as f64 - x0 as f64);
    y0 + t * (y1 - y0)
}

fn extract_first_text(xml: &str, tags: &[&str]) -> Option<String> {
    for tag in tags {
        if let Some(v) = extract_tag_text(xml, tag) {
            return Some(v);
        }
    }
    None
}

fn extract_tag_blocks(xml: &str, tag_name: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut i = 0usize;
    while i < xml.len() {
        let Some(rel_start) = xml[i..].find('<') else {
            break;
        };
        let start = i + rel_start;
        let Some(rel_end) = xml[start..].find('>') else {
            break;
        };
        let end = start + rel_end;
        let header = &xml[start + 1..end];
        if header.starts_with('/')
            || header.trim_end().ends_with('/')
            || !header_contains_tag_name(header, tag_name)
        {
            i = end + 1;
            continue;
        }

        let content_start = end + 1;
        let mut depth = 1usize;
        let mut cursor = content_start;
        while cursor < xml.len() {
            let Some(inner_rel_start) = xml[cursor..].find('<') else {
                break;
            };
            let inner_start = cursor + inner_rel_start;
            let Some(inner_rel_end) = xml[inner_start..].find('>') else {
                break;
            };
            let inner_end = inner_start + inner_rel_end;
            let inner_header = &xml[inner_start + 1..inner_end];

            if !inner_header.starts_with('/')
                && !inner_header.trim_end().ends_with('/')
                && header_contains_tag_name(inner_header, tag_name)
            {
                depth += 1;
            } else if inner_header.starts_with('/')
                && header_contains_tag_name(inner_header.trim_start_matches('/'), tag_name)
            {
                depth -= 1;
                if depth == 0 {
                    blocks.push(xml[content_start..inner_start].to_string());
                    cursor = inner_end + 1;
                    break;
                }
            }

            cursor = inner_end + 1;
        }
        i = cursor;
    }
    blocks
}

fn extract_tag_text(xml: &str, tag_name: &str) -> Option<String> {
    let mut i = 0usize;
    let bytes = xml.as_bytes();
    while i < bytes.len() {
        let rel_start = xml[i..].find('<')?;
        let start = i + rel_start;
        let rel_end = xml[start..].find('>')?;
        let end = start + rel_end;
        let header = &xml[start + 1..end];
        if !header.starts_with('/') && header_contains_tag_name(header, tag_name) {
            let close = xml[end + 1..].find('<')?;
            let text = xml[end + 1..end + 1 + close].trim();
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
        i = end + 1;
    }
    None
}

fn header_contains_tag_name(header: &str, tag_name: &str) -> bool {
    let name = header
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim_end_matches('/');
    if name.eq_ignore_ascii_case(tag_name) {
        return true;
    }
    if let Some(idx) = name.rfind(':') {
        return name[idx + 1..].eq_ignore_ascii_case(tag_name);
    }
    false
}

/// Convert a linear-power raster to decibels in-place: `10 × log₁₀(v)`.
///
/// Pixels where the value is ≤ 0 or equal to nodata remain as nodata.
fn linear_to_db(linear: &Raster) -> Result<Raster> {
    let mut output = Raster::new(RasterConfig {
        cols: linear.cols,
        rows: linear.rows,
        bands: linear.bands,
        x_min: linear.x_min,
        y_min: linear.y_min,
        cell_size: linear.cell_size_x,
        cell_size_y: Some(linear.cell_size_y),
        nodata: linear.nodata,
        data_type: DataType::F32,
        crs: linear.crs.clone(),
        metadata: linear.metadata.clone(),
    });
    output
        .metadata
        .push(("sentinel1_scale".to_string(), "dB".to_string()));

    for band in 0..linear.bands {
        for row in 0..linear.rows {
            for col in 0..linear.cols {
                let value = linear.get(band as isize, row as isize, col as isize);
                if value == linear.nodata || value <= 0.0 {
                    continue;
                }
                let db = 10.0 * value.log10();
                output.set(band as isize, row as isize, col as isize, db)?;
            }
        }
    }

    Ok(output)
}

/// Parse orbit state vectors from annotation XML.
fn parse_orbit_vectors(xml: &str) -> Result<Vec<Sentinel1OrbitVector>> {
    let mut vectors = Vec::new();
    for block in extract_tag_blocks(xml, "orbit") {
        let time = extract_tag_text(&block, "time")
            .ok_or_else(|| RasterError::MissingField("orbit/time".to_string()))?;

        let pos_block = extract_tag_blocks(&block, "position")
            .into_iter()
            .next()
            .ok_or_else(|| RasterError::MissingField("orbit/position".to_string()))?;
        let px = extract_tag_text(&pos_block, "x")
            .ok_or_else(|| RasterError::MissingField("orbit/position/x".to_string()))?
            .parse::<f64>()
            .map_err(|e| RasterError::Other(format!("invalid orbit position x: {e}")))?;
        let py = extract_tag_text(&pos_block, "y")
            .ok_or_else(|| RasterError::MissingField("orbit/position/y".to_string()))?
            .parse::<f64>()
            .map_err(|e| RasterError::Other(format!("invalid orbit position y: {e}")))?;
        let pz = extract_tag_text(&pos_block, "z")
            .ok_or_else(|| RasterError::MissingField("orbit/position/z".to_string()))?
            .parse::<f64>()
            .map_err(|e| RasterError::Other(format!("invalid orbit position z: {e}")))?;

        let vel_block = extract_tag_blocks(&block, "velocity")
            .into_iter()
            .next()
            .ok_or_else(|| RasterError::MissingField("orbit/velocity".to_string()))?;
        let vx = extract_tag_text(&vel_block, "x")
            .ok_or_else(|| RasterError::MissingField("orbit/velocity/x".to_string()))?
            .parse::<f64>()
            .map_err(|e| RasterError::Other(format!("invalid orbit velocity x: {e}")))?;
        let vy = extract_tag_text(&vel_block, "y")
            .ok_or_else(|| RasterError::MissingField("orbit/velocity/y".to_string()))?
            .parse::<f64>()
            .map_err(|e| RasterError::Other(format!("invalid orbit velocity y: {e}")))?;
        let vz = extract_tag_text(&vel_block, "z")
            .ok_or_else(|| RasterError::MissingField("orbit/velocity/z".to_string()))?
            .parse::<f64>()
            .map_err(|e| RasterError::Other(format!("invalid orbit velocity z: {e}")))?;

        vectors.push(Sentinel1OrbitVector {
            time,
            position: [px, py, pz],
            velocity: [vx, vy, vz],
        });
    }

    if vectors.is_empty() {
        return Err(RasterError::MissingField(
            "no orbit blocks found in annotation XML".to_string(),
        ));
    }

    vectors.sort_by(|a, b| a.time.cmp(&b.time));
    Ok(vectors)
}

/// Parse geographic bounding box `[west, south, east, north]` from a
/// `<gml:coordinates>` string containing whitespace-separated `lat,lon` pairs.
fn parse_gml_bounds(coords: &str) -> Option<[f64; 4]> {
    let mut lats = Vec::new();
    let mut lons = Vec::new();
    for pair in coords.split_whitespace() {
        let nums: Vec<&str> = pair.split(',').collect();
        if nums.len() >= 2 {
            if let (Ok(lat), Ok(lon)) = (nums[0].parse::<f64>(), nums[1].parse::<f64>()) {
                lats.push(lat);
                lons.push(lon);
            }
        }
    }
    if lats.is_empty() {
        return None;
    }
    let south = lats.iter().cloned().fold(f64::INFINITY, f64::min);
    let north = lats.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let west = lons.iter().cloned().fold(f64::INFINITY, f64::min);
    let east = lons.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    Some([west, south, east, north])
}

fn parse_geolocation_grid(xml: &str) -> Result<Sentinel1GeolocationGrid> {
    let mut points = Vec::new();
    for block in extract_tag_blocks(xml, "geolocationGridPoint") {
        let line = extract_tag_text(&block, "line")
            .ok_or_else(|| RasterError::MissingField("geolocationGridPoint/line".to_string()))?
            .parse::<usize>()
            .map_err(|e| RasterError::Other(format!("invalid geolocation line: {e}")))?;
        let pixel = extract_tag_text(&block, "pixel")
            .ok_or_else(|| RasterError::MissingField("geolocationGridPoint/pixel".to_string()))?
            .parse::<usize>()
            .map_err(|e| RasterError::Other(format!("invalid geolocation pixel: {e}")))?;
        let latitude = extract_tag_text(&block, "latitude")
            .ok_or_else(|| {
                RasterError::MissingField("geolocationGridPoint/latitude".to_string())
            })?
            .parse::<f64>()
            .map_err(|e| RasterError::Other(format!("invalid latitude: {e}")))?;
        let longitude = extract_tag_text(&block, "longitude")
            .ok_or_else(|| {
                RasterError::MissingField("geolocationGridPoint/longitude".to_string())
            })?
            .parse::<f64>()
            .map_err(|e| RasterError::Other(format!("invalid longitude: {e}")))?;
        let height = extract_tag_text(&block, "height")
            .ok_or_else(|| RasterError::MissingField("geolocationGridPoint/height".to_string()))?
            .parse::<f64>()
            .map_err(|e| RasterError::Other(format!("invalid height: {e}")))?;
        let incidence_angle = extract_tag_text(&block, "incidenceAngle")
            .ok_or_else(|| {
                RasterError::MissingField("geolocationGridPoint/incidenceAngle".to_string())
            })?
            .parse::<f64>()
            .map_err(|e| RasterError::Other(format!("invalid incidenceAngle: {e}")))?;
        let elevation_angle = extract_tag_text(&block, "elevationAngle")
            .ok_or_else(|| {
                RasterError::MissingField("geolocationGridPoint/elevationAngle".to_string())
            })?
            .parse::<f64>()
            .map_err(|e| RasterError::Other(format!("invalid elevationAngle: {e}")))?;

        points.push(Sentinel1GeolocationGridPoint {
            line,
            pixel,
            latitude,
            longitude,
            height,
            incidence_angle,
            elevation_angle,
        });
    }

    if points.is_empty() {
        return Err(RasterError::MissingField(
            "no geolocationGridPoint blocks found in annotation XML".to_string(),
        ));
    }

    points.sort_by(|a, b| a.line.cmp(&b.line).then(a.pixel.cmp(&b.pixel)));
    Ok(Sentinel1GeolocationGrid { points })
}

fn parse_burst_list(xml: &str) -> Result<Sentinel1BurstList> {
    let lines_per_burst = extract_tag_text(xml, "linesPerBurst")
        .ok_or_else(|| {
            RasterError::MissingField(
                "linesPerBurst not found — not an SLC TOPS annotation".to_string(),
            )
        })?
        .parse::<usize>()
        .map_err(|e| RasterError::Other(format!("invalid linesPerBurst: {e}")))?;
    let samples_per_burst = extract_tag_text(xml, "samplesPerBurst")
        .ok_or_else(|| RasterError::MissingField("samplesPerBurst".to_string()))?
        .parse::<usize>()
        .map_err(|e| RasterError::Other(format!("invalid samplesPerBurst: {e}")))?;

    let mut bursts = Vec::new();
    for block in extract_tag_blocks(xml, "burst") {
        let azimuth_time = extract_tag_text(&block, "azimuthTime")
            .ok_or_else(|| RasterError::MissingField("burst/azimuthTime".to_string()))?;
        let byte_offset = extract_tag_text(&block, "byteOffset")
            .ok_or_else(|| RasterError::MissingField("burst/byteOffset".to_string()))?
            .parse::<u64>()
            .map_err(|e| RasterError::Other(format!("invalid byteOffset: {e}")))?;
        let first_valid_samples = parse_i32_list(
            &extract_tag_text(&block, "firstValidSample")
                .ok_or_else(|| {
                    RasterError::MissingField("burst/firstValidSample".to_string())
                })?,
        )?;
        let last_valid_samples = parse_i32_list(
            &extract_tag_text(&block, "lastValidSample")
                .ok_or_else(|| {
                    RasterError::MissingField("burst/lastValidSample".to_string())
                })?,
        )?;

        bursts.push(Sentinel1Burst {
            azimuth_time,
            byte_offset,
            lines_per_burst,
            samples_per_burst,
            first_valid_samples,
            last_valid_samples,
        });
    }

    if bursts.is_empty() {
        return Err(RasterError::MissingField(
            "no burst blocks found — not an SLC TOPS annotation".to_string(),
        ));
    }

    Ok(Sentinel1BurstList {
        lines_per_burst,
        samples_per_burst,
        bursts,
    })
}

fn interpolate_grid_row<F>(
    points: &[Sentinel1GeolocationGridPoint],
    line: usize,
    col: usize,
    field: &F,
) -> Option<f64>
where
    F: Fn(&Sentinel1GeolocationGridPoint) -> f64,
{
    let mut sorted: Vec<(usize, f64)> = points
        .iter()
        .filter(|p| p.line == line)
        .map(|p| (p.pixel, field(p)))
        .collect();
    sorted.sort_by_key(|&(pixel, _)| pixel);
    if sorted.is_empty() {
        return None;
    }
    let pixels: Vec<usize> = sorted.iter().map(|(p, _)| *p).collect();
    let values: Vec<f64> = sorted.iter().map(|(_, v)| *v).collect();
    interpolate_along_pixels(&pixels, &values, col)
}

fn parse_i32_list(text: &str) -> Result<Vec<i32>> {
    text.split_whitespace()
        .map(|token| {
            token
                .parse::<i32>()
                .map_err(|e| RasterError::Other(format!("invalid i32 value '{token}': {e}")))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::RasterFormat;

    #[test]
    fn parses_minimal_sentinel1_safe_structure() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let safe = tmp.path().join("S1A_TEST_SLC.SAFE");
        fs::create_dir_all(&safe).expect("create safe root");

        let manifest = r#"
            <xfdu:XFDU>
              <metadataSection>
                <metadataObject>
                  <metadataWrap>
                    <xmlData>
                      <s1sarl1:standAloneProductInformation>
                        <s1sarl1:productType>SLC</s1sarl1:productType>
                        <s1sarl1:mode>IW</s1sarl1:mode>
                        <s1sarl1:transmitterReceiverPolarisation>VV</s1sarl1:transmitterReceiverPolarisation>
                      </s1sarl1:standAloneProductInformation>
                    </xmlData>
                  </metadataWrap>
                </metadataObject>
                <metadataObject ID="acquisitionPeriod">
                  <metadataWrap>
                    <xmlData>
                      <safe:acquisitionPeriod>
                        <safe:startTime>2026-03-31T15:20:00.000000</safe:startTime>
                        <safe:stopTime>2026-03-31T15:20:12.000000</safe:stopTime>
                      </safe:acquisitionPeriod>
                    </xmlData>
                  </metadataWrap>
                </metadataObject>
              </metadataSection>
            </xfdu:XFDU>
        "#;
        fs::write(safe.join("manifest.safe"), manifest).expect("write manifest");

        let measurement_dir = safe.join("measurement");
        fs::create_dir_all(&measurement_dir).expect("create measurement dir");
        fs::write(
            measurement_dir.join("s1a-iw-grd-vv-20260331t152000.tiff"),
            b"",
        )
        .expect("write measurement tiff");

        let annotation_dir = safe.join("annotation");
        fs::create_dir_all(&annotation_dir).expect("create annotation dir");
        fs::write(
            annotation_dir.join("s1a-iw-grd-vv-20260331t152000-001.xml"),
            b"<annotation/>",
        )
        .expect("write annotation xml");

        let calibration_dir = annotation_dir.join("calibration");
        fs::create_dir_all(&calibration_dir).expect("create calibration dir");
        fs::write(
            calibration_dir.join("calibration-s1a-iw-grd-vv-20260331t152000-001.xml"),
            b"<calibration/>",
        )
        .expect("write calibration xml");
        let noise_dir = calibration_dir.join("noise");
        fs::create_dir_all(&noise_dir).expect("create noise dir");
        fs::write(
            noise_dir.join("noise-s1a-iw-grd-vv-20260331t152000-001.xml"),
            b"<noise/>",
        )
        .expect("write noise xml");

        let pkg = Sentinel1SafePackage::open(&safe).expect("open sentinel-1 safe");
        assert_eq!(pkg.product_type.as_deref(), Some("SLC"));
        assert_eq!(pkg.acquisition_mode.as_deref(), Some("IW"));
        assert_eq!(pkg.polarization.as_deref(), Some("VV"));
        assert_eq!(
            pkg.acquisition_datetime_utc.as_deref(),
            Some("2026-03-31T15:20:00.000000")
        );
        assert_eq!(pkg.measurements.len(), 1);
        assert!(pkg.measurement_path("IW_GRD_VV").is_some());
        assert_eq!(pkg.annotation.len(), 1);
        assert!(pkg.annotation_path("IW_GRD_VV").is_some());
        assert_eq!(pkg.calibration.len(), 1);
        assert!(pkg.calibration_path("IW_GRD_VV").is_some());
        assert_eq!(pkg.noise.len(), 1);
        assert!(pkg.noise_path("IW_GRD_VV").is_some());
    }

    #[test]
    fn reads_and_applies_calibration_lut() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let safe = tmp.path().join("S1A_IW_GRD_TEST.SAFE");
        fs::create_dir_all(&safe).expect("create safe root");
        fs::write(safe.join("manifest.safe"), "<xfdu>Sentinel-1</xfdu>")
            .expect("write manifest");

        let measurement_dir = safe.join("measurement");
        fs::create_dir_all(&measurement_dir).expect("create measurement dir");
        let measurement_path = measurement_dir.join("s1a-iw-grd-vv-20260401t120000.tiff");
        let measurement = Raster::from_data(
            RasterConfig {
                cols: 2,
                rows: 2,
                bands: 1,
                x_min: 0.0,
                y_min: 0.0,
                cell_size: 1.0,
                cell_size_y: Some(1.0),
                nodata: -9999.0,
                data_type: DataType::F32,
                ..Default::default()
            },
            vec![2.0, 4.0, 6.0, 8.0],
        )
        .expect("create measurement raster");
        measurement
            .write(&measurement_path, RasterFormat::GeoTiff)
            .expect("write measurement geotiff");

        let calibration_dir = safe.join("annotation").join("calibration");
        fs::create_dir_all(&calibration_dir).expect("create calibration dir");
        let calibration_xml = r#"
            <calibration>
              <calibrationVectorList count="1">
                <calibrationVector>
                  <line>0</line>
                  <pixel count="2">0 1</pixel>
                  <sigmaNought count="2">1 2</sigmaNought>
                  <betaNought count="2">2 4</betaNought>
                  <gamma count="2">4 8</gamma>
                  <dn count="2">1 1</dn>
                </calibrationVector>
              </calibrationVectorList>
            </calibration>
        "#;
        fs::write(
            calibration_dir.join("calibration-s1a-iw-grd-vv-20260401t120000-001.xml"),
            calibration_xml,
        )
        .expect("write calibration xml");

        let pkg = Sentinel1SafePackage::open(&safe).expect("open sentinel-1 safe");
        let lut = pkg
            .read_calibration_lut("IW_GRD_VV")
            .expect("read calibration lut");
        assert_eq!(lut.vectors.len(), 1);
        assert_eq!(
            lut.interpolated_value(0, 1, Sentinel1CalibrationTarget::SigmaNought),
            Some(2.0)
        );

        let sigma = pkg
            .read_calibrated_measurement("IW_GRD_VV", Sentinel1CalibrationTarget::SigmaNought)
            .expect("calibrate sigma");
        assert_eq!(sigma.get(0, 0, 0), 4.0);
        assert_eq!(sigma.get(0, 0, 1), 4.0);
        assert_eq!(sigma.get(0, 1, 0), 36.0);
        assert_eq!(sigma.get(0, 1, 1), 16.0);

        let beta = pkg
            .read_calibrated_measurement("IW_GRD_VV", Sentinel1CalibrationTarget::BetaNought)
            .expect("calibrate beta");
        assert_eq!(beta.get(0, 0, 0), 1.0);
        assert_eq!(beta.get(0, 0, 1), 1.0);
    }

    #[test]
    fn reads_and_applies_noise_lut() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let safe = tmp.path().join("S1A_IW_GRD_TEST.SAFE");
        fs::create_dir_all(&safe).expect("create safe root");
        fs::write(safe.join("manifest.safe"), "<xfdu>Sentinel-1</xfdu>")
            .expect("write manifest");

        let measurement_dir = safe.join("measurement");
        fs::create_dir_all(&measurement_dir).expect("create measurement dir");
        let measurement_path = measurement_dir.join("s1a-iw-grd-vv-20260401t120000.tiff");
        let measurement = Raster::from_data(
            RasterConfig {
                cols: 2,
                rows: 2,
                bands: 1,
                x_min: 0.0,
                y_min: 0.0,
                cell_size: 1.0,
                cell_size_y: Some(1.0),
                nodata: -9999.0,
                data_type: DataType::F32,
                ..Default::default()
            },
            vec![2.0, 4.0, 6.0, 8.0],
        )
        .expect("create measurement raster");
        measurement
            .write(&measurement_path, RasterFormat::GeoTiff)
            .expect("write measurement geotiff");

        let calibration_dir = safe.join("annotation").join("calibration");
        fs::create_dir_all(&calibration_dir).expect("create calibration dir");
        let calibration_xml = r#"
            <calibration>
              <calibrationVectorList count="1">
                <calibrationVector>
                  <line>0</line>
                  <pixel count="2">0 1</pixel>
                  <sigmaNought count="2">1 2</sigmaNought>
                  <betaNought count="2">2 4</betaNought>
                  <gamma count="2">4 8</gamma>
                </calibrationVector>
              </calibrationVectorList>
            </calibration>
        "#;
        fs::write(
            calibration_dir.join("calibration-s1a-iw-grd-vv-20260401t120000-001.xml"),
            calibration_xml,
        )
        .expect("write calibration xml");

        let noise_dir = calibration_dir.join("noise");
        fs::create_dir_all(&noise_dir).expect("create noise dir");
        let noise_xml = r#"
            <noise>
              <noiseVectorList count="1">
                <noiseVector>
                  <line>0</line>
                  <pixel count="2">0 1</pixel>
                  <noiseLut count="2">1 3</noiseLut>
                </noiseVector>
              </noiseVectorList>
            </noise>
        "#;
        fs::write(
            noise_dir.join("noise-s1a-iw-grd-vv-20260401t120000-001.xml"),
            noise_xml,
        )
        .expect("write noise xml");

        let pkg = Sentinel1SafePackage::open(&safe).expect("open sentinel-1 safe");
        let noise_lut = pkg.read_noise_lut("IW_GRD_VV").expect("read noise lut");
        assert_eq!(noise_lut.vectors.len(), 1);
        assert_eq!(noise_lut.interpolated_value(0, 1), Some(3.0));

        let corrected = pkg
            .read_noise_corrected_calibrated_measurement(
                "IW_GRD_VV",
                Sentinel1CalibrationTarget::SigmaNought,
            )
            .expect("noise corrected sigma0");
        assert_eq!(corrected.get(0, 0, 0), 3.0);
        assert_eq!(corrected.get(0, 0, 1), 1.0);
        assert_eq!(corrected.get(0, 1, 0), 35.0);
        assert_eq!(corrected.get(0, 1, 1), 13.0);
    }

    #[test]
    fn slc_subswath_keys_are_distinct() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let safe = tmp.path().join("S1A_IW_SLC_TEST.SAFE");
        fs::create_dir_all(&safe).expect("create safe root");
        fs::write(safe.join("manifest.safe"), "<xfdu>Sentinel-1</xfdu>")
            .expect("write manifest");

        let measurement_dir = safe.join("measurement");
        fs::create_dir_all(&measurement_dir).expect("create measurement dir");
        fs::write(
            measurement_dir.join("s1a-iw1-slc-vv-20260401t120000.tiff"),
            b"",
        )
        .expect("write iw1");
        fs::write(
            measurement_dir.join("s1a-iw2-slc-vv-20260401t120000.tiff"),
            b"",
        )
        .expect("write iw2");
        fs::write(
            measurement_dir.join("s1a-iw3-slc-vv-20260401t120000.tiff"),
            b"",
        )
        .expect("write iw3");

        let pkg = Sentinel1SafePackage::open(&safe).expect("open sentinel-1 safe");
        assert_eq!(pkg.measurements.len(), 3);
        assert!(pkg.measurement_path("IW1_SLC_VV").is_some());
        assert!(pkg.measurement_path("IW2_SLC_VV").is_some());
        assert!(pkg.measurement_path("IW3_SLC_VV").is_some());
    }

    #[test]
    fn calibrated_measurement_db_applies_log10_transform() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let safe = tmp.path().join("S1A_IW_GRD_DB_TEST.SAFE");
        fs::create_dir_all(&safe).expect("create safe root");
        fs::write(safe.join("manifest.safe"), "<xfdu>Sentinel-1</xfdu>")
            .expect("write manifest");

        let measurement_dir = safe.join("measurement");
        fs::create_dir_all(&measurement_dir).expect("create measurement dir");
        let measurement_path = measurement_dir.join("s1a-iw-grd-vv-20260401t130000.tiff");
        // DN = 10 at both pixels; sigma LUT = 1 → calibrated = (10/1)^2 = 100
        let measurement = Raster::from_data(
            RasterConfig {
                cols: 2,
                rows: 1,
                bands: 1,
                x_min: 0.0,
                y_min: 0.0,
                cell_size: 1.0,
                cell_size_y: Some(1.0),
                nodata: -9999.0,
                data_type: DataType::F32,
                ..Default::default()
            },
            vec![10.0, 10.0],
        )
        .expect("create measurement raster");
        measurement
            .write(&measurement_path, RasterFormat::GeoTiff)
            .expect("write measurement geotiff");

        let calibration_dir = safe.join("annotation").join("calibration");
        fs::create_dir_all(&calibration_dir).expect("create calibration dir");
        let calibration_xml = r#"
            <calibration>
              <calibrationVectorList count="1">
                <calibrationVector>
                  <line>0</line>
                  <pixel count="2">0 1</pixel>
                  <sigmaNought count="2">1 1</sigmaNought>
                  <betaNought count="2">1 1</betaNought>
                  <gamma count="2">1 1</gamma>
                </calibrationVector>
              </calibrationVectorList>
            </calibration>
        "#;
        fs::write(
            calibration_dir.join("calibration-s1a-iw-grd-vv-20260401t130000-001.xml"),
            calibration_xml,
        )
        .expect("write calibration xml");

        let pkg = Sentinel1SafePackage::open(&safe).expect("open sentinel-1 safe");
        let db = pkg
            .read_calibrated_measurement_db("IW_GRD_VV", Sentinel1CalibrationTarget::SigmaNought)
            .expect("calibrate to dB");
        // 10 * log10(100) = 20.0
        let expected = 20.0_f64;
        assert!((db.get(0, 0, 0) - expected).abs() < 1e-4, "col 0: {}", db.get(0, 0, 0));
        assert!((db.get(0, 0, 1) - expected).abs() < 1e-4, "col 1: {}", db.get(0, 0, 1));
        // Verify metadata tag was set
        assert!(db
            .metadata
            .iter()
            .any(|(k, v)| k == "sentinel1_scale" && v == "dB"));
    }

    #[test]
    fn reads_orbit_vectors_from_annotation_xml() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let safe = tmp.path().join("S1A_IW_GRD_ORBIT_TEST.SAFE");
        fs::create_dir_all(&safe).expect("create safe root");
        fs::write(safe.join("manifest.safe"), "<xfdu>Sentinel-1</xfdu>")
            .expect("write manifest");

        let measurement_dir = safe.join("measurement");
        fs::create_dir_all(&measurement_dir).expect("create measurement dir");
        fs::write(
            measurement_dir.join("s1a-iw-grd-vv-20260401t140000.tiff"),
            b"",
        )
        .expect("write measurement tiff");

        let annotation_dir = safe.join("annotation");
        fs::create_dir_all(&annotation_dir).expect("create annotation dir");
        let annotation_xml = r#"
            <product>
              <generalAnnotation>
                <orbitList count="2">
                  <orbit>
                    <time>2026-04-01T14:00:00.000000</time>
                    <frame>Earth Fixed</frame>
                    <position>
                      <x>6354000.0</x>
                      <y>1000000.0</y>
                      <z>2000000.0</z>
                    </position>
                    <velocity>
                      <x>-1000.0</x>
                      <y>500.0</y>
                      <z>7000.0</z>
                    </velocity>
                  </orbit>
                  <orbit>
                    <time>2026-04-01T14:00:10.000000</time>
                    <frame>Earth Fixed</frame>
                    <position>
                      <x>6344000.0</x>
                      <y>1005000.0</y>
                      <z>2070000.0</z>
                    </position>
                    <velocity>
                      <x>-1001.0</x>
                      <y>501.0</y>
                      <z>7001.0</z>
                    </velocity>
                  </orbit>
                </orbitList>
              </generalAnnotation>
            </product>
        "#;
        fs::write(
            annotation_dir.join("s1a-iw-grd-vv-20260401t140000-001.xml"),
            annotation_xml,
        )
        .expect("write annotation xml");

        let pkg = Sentinel1SafePackage::open(&safe).expect("open sentinel-1 safe");
        let orbits = pkg
            .read_orbit_vectors("IW_GRD_VV")
            .expect("read orbit vectors");

        assert_eq!(orbits.len(), 2);
        assert_eq!(orbits[0].time, "2026-04-01T14:00:00.000000");
        assert_eq!(orbits[0].position, [6354000.0, 1000000.0, 2000000.0]);
        assert_eq!(orbits[0].velocity, [-1000.0, 500.0, 7000.0]);
        assert_eq!(orbits[1].time, "2026-04-01T14:00:10.000000");
        assert_eq!(orbits[1].position, [6344000.0, 1005000.0, 2070000.0]);
    }

    #[test]
    fn parses_spatial_bounds_from_manifest_coordinates() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let safe = tmp.path().join("S1A_IW_GRD_BOUNDS_TEST.SAFE");
        fs::create_dir_all(&safe).expect("create safe root");

        // Typical S-1 footprint: lat,lon pairs spaced in whitespace
        let manifest = r#"
            <xfdu:XFDU>
              <metadataSection>
                <metadataObject ID="measurementFrameSet">
                  <metadataWrap>
                    <xmlData>
                      <safe:frameSet>
                        <safe:frame>
                          <safe:footPrint srsName="urn:ogc:def:crs:EPSG::4326">
                            <gml:coordinates>48.5,-80.1 48.5,-75.3 46.2,-75.3 46.2,-80.1 48.5,-80.1</gml:coordinates>
                          </safe:footPrint>
                        </safe:frame>
                      </safe:frameSet>
                    </xmlData>
                  </metadataWrap>
                </metadataObject>
              </metadataSection>
            </xfdu:XFDU>
        "#;
        fs::write(safe.join("manifest.safe"), manifest).expect("write manifest");

        let measurement_dir = safe.join("measurement");
        fs::create_dir_all(&measurement_dir).expect("create measurement dir");
        fs::write(
            measurement_dir.join("s1a-iw-grd-vv-20260401t150000.tiff"),
            b"",
        )
        .expect("write measurement tiff");

        let pkg = Sentinel1SafePackage::open(&safe).expect("open sentinel-1 safe");
        let bounds = pkg.spatial_bounds.expect("spatial bounds should be set");
        // Pairs are lat,lon so lats=[48.5,48.5,46.2,46.2], lons=[-80.1,-75.3,-75.3,-80.1]
        // [west, south, east, north] = [-80.1, 46.2, -75.3, 48.5]
        assert!((bounds[0] - (-80.1)).abs() < 1e-6, "west: {}", bounds[0]);
        assert!((bounds[1] - 46.2).abs() < 1e-6, "south: {}", bounds[1]);
        assert!((bounds[2] - (-75.3)).abs() < 1e-6, "east: {}", bounds[2]);
        assert!((bounds[3] - 48.5).abs() < 1e-6, "north: {}", bounds[3]);
    }

    #[test]
    fn reads_geolocation_grid_from_annotation_xml() {
        // 2×2 sparse grid: lines 0 and 100, pixels 0 and 100.
        let xml = r#"
            <product>
              <geolocationGrid>
                <geolocationGridPointList count="4">
                  <geolocationGridPoint>
                    <line>0</line><pixel>0</pixel>
                    <latitude>48.0</latitude><longitude>-80.0</longitude>
                    <height>0.0</height>
                    <incidenceAngle>30.0</incidenceAngle>
                    <elevationAngle>60.0</elevationAngle>
                  </geolocationGridPoint>
                  <geolocationGridPoint>
                    <line>0</line><pixel>100</pixel>
                    <latitude>48.0</latitude><longitude>-79.0</longitude>
                    <height>0.0</height>
                    <incidenceAngle>32.0</incidenceAngle>
                    <elevationAngle>58.0</elevationAngle>
                  </geolocationGridPoint>
                  <geolocationGridPoint>
                    <line>100</line><pixel>0</pixel>
                    <latitude>47.0</latitude><longitude>-80.0</longitude>
                    <height>0.0</height>
                    <incidenceAngle>31.0</incidenceAngle>
                    <elevationAngle>59.0</elevationAngle>
                  </geolocationGridPoint>
                  <geolocationGridPoint>
                    <line>100</line><pixel>100</pixel>
                    <latitude>47.0</latitude><longitude>-79.0</longitude>
                    <height>0.0</height>
                    <incidenceAngle>33.0</incidenceAngle>
                    <elevationAngle>57.0</elevationAngle>
                  </geolocationGridPoint>
                </geolocationGridPointList>
              </geolocationGrid>
            </product>
        "#;
        let grid = parse_geolocation_grid(xml).expect("parse geolocation grid");
        assert_eq!(grid.points.len(), 4);

        // Corner (0,0): exact match.
        assert_eq!(grid.interpolated_incidence_angle(0, 0), Some(30.0));
        assert_eq!(grid.interpolated_lat_lon(0, 0), Some((48.0, -80.0)));

        // Top-row midpoint (0, 50): mean of 30.0 and 32.0 = 31.0.
        let ia_top = grid.interpolated_incidence_angle(0, 50).unwrap();
        assert!((ia_top - 31.0).abs() < 1e-6, "top midpoint: {ia_top}");

        // Grid centre (50, 50): bilinear mean of all four = 31.5.
        let ia_centre = grid.interpolated_incidence_angle(50, 50).unwrap();
        assert!((ia_centre - 31.5).abs() < 1e-6, "centre: {ia_centre}");

        // Grid centre lat/lon: 47.5, -79.5.
        let (lat, lon) = grid.interpolated_lat_lon(50, 50).unwrap();
        assert!((lat - 47.5).abs() < 1e-6, "lat: {lat}");
        assert!((lon - (-79.5)).abs() < 1e-6, "lon: {lon}");
    }

    #[test]
    fn reads_burst_list_from_slc_annotation_xml() {
        let xml = r#"
            <product>
              <swathTiming>
                <linesPerBurst>4</linesPerBurst>
                <samplesPerBurst>10</samplesPerBurst>
                <burstList count="2">
                  <burst>
                    <azimuthTime>2026-04-01T14:00:00.000000</azimuthTime>
                    <byteOffset>0</byteOffset>
                    <firstValidSample count="4">-1 0 0 -1</firstValidSample>
                    <lastValidSample count="4">-1 9 9 -1</lastValidSample>
                  </burst>
                  <burst>
                    <azimuthTime>2026-04-01T14:00:01.000000</azimuthTime>
                    <byteOffset>800</byteOffset>
                    <firstValidSample count="4">0 0 0 0</firstValidSample>
                    <lastValidSample count="4">9 9 9 9</lastValidSample>
                  </burst>
                </burstList>
              </swathTiming>
            </product>
        "#;
        let bl = parse_burst_list(xml).expect("parse burst list");
        assert_eq!(bl.lines_per_burst, 4);
        assert_eq!(bl.samples_per_burst, 10);
        assert_eq!(bl.bursts.len(), 2);

        let b0 = &bl.bursts[0];
        assert_eq!(b0.azimuth_time, "2026-04-01T14:00:00.000000");
        assert_eq!(b0.byte_offset, 0);
        assert_eq!(b0.first_valid_samples, vec![-1, 0, 0, -1]);
        assert_eq!(b0.last_valid_samples, vec![-1, 9, 9, -1]);

        let b1 = &bl.bursts[1];
        assert_eq!(b1.byte_offset, 800);
        assert_eq!(b1.first_valid_samples, vec![0, 0, 0, 0]);
    }

    #[test]
    fn list_polarizations_extracts_unique_pols() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let safe = tmp.path().join("S1A_IW_GRD_DUAL_POL_TEST.SAFE");
        fs::create_dir_all(&safe).expect("create safe root");
        fs::write(safe.join("manifest.safe"), "<xfdu>Sentinel-1</xfdu>")
            .expect("write manifest");

        let mdir = safe.join("measurement");
        fs::create_dir_all(&mdir).expect("create measurement dir");
        fs::write(mdir.join("s1a-iw-grd-vv-20260401t120000.tiff"), b"")
            .expect("write VV");
        fs::write(mdir.join("s1a-iw-grd-vh-20260401t120000.tiff"), b"")
            .expect("write VH");

        let pkg = Sentinel1SafePackage::open(&safe).expect("open");
        let mut pols = pkg.list_polarizations();
        pols.sort();
        assert_eq!(pols, vec!["VH", "VV"]);

        // keys_for_polarization filters correctly
        let vv_keys: Vec<String> = pkg
            .list_measurement_keys()
            .into_iter()
            .filter(|k| k.ends_with("_VV"))
            .collect();
        assert_eq!(vv_keys.len(), 1);
        assert_eq!(vv_keys[0], "IW_GRD_VV");
    }
}
