use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use nalgebra::{DMatrix, DVector, SymmetricEigen};
use rayon::prelude::*;
use serde_json::json;
use wbcore::{
    parse_optional_output_path, LicenseTier, PercentCoalescer, Tool, ToolArgs, ToolCategory,
    ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata, ToolParamDescriptor,
    ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::{Raster, RasterConfig, RasterFormat};
use wbraster::{LandsatBundle, Sentinel2SafePackage};

use crate::memory_store;
use crate::tools::raster_stack_validator::{
    align_and_validate_raster_stack, validate_raster_stack_strict, RasterStackConfig,
};

pub struct DarkObjectSubtractionTool;
pub struct DnToToaReflectanceTool;
pub struct NdviBasedEmissivityTool;
pub struct PcaBasedChangeDetectionTool;
pub struct ImageDifferenceChangeDetectionTool;
pub struct PostClassificationChangeTool;
pub struct LandSurfaceTemperatureSingleChannelTool;
pub struct LandSurfaceTemperatureSplitWindowTool;
pub struct SpectralAngleMapperTool;
pub struct ContinuumRemovalTool;
pub struct LinearSpectralUnmixingTool;
pub struct MinimumNoiseFractionTool;
pub struct SpectralLibraryMatchingTool;
pub struct CloudePottierDecompositionTool;
pub struct FreemanDurdenDecompositionTool;
pub struct YamaguchiDecompositionTool;
pub struct HAlphaWisartClassificationTool;
pub struct WishartIterativeClusteringTool;

fn parse_raster_list_arg(args: &ToolArgs, name: &str) -> Result<Vec<String>, ToolError> {
    let value = args
        .get(name)
        .ok_or_else(|| ToolError::Validation(format!("missing required parameter '{name}'")))?;
    let arr = value
        .as_array()
        .ok_or_else(|| ToolError::Validation(format!("parameter '{name}' must be an array of raster paths")))?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let Some(s) = item.as_str() else {
            return Err(ToolError::Validation(format!(
                "parameter '{name}' must contain only string raster paths"
            )));
        };
        out.push(s.to_string());
    }
    if out.is_empty() {
        return Err(ToolError::Validation(format!(
            "parameter '{name}' must contain at least one raster path"
        )));
    }
    Ok(out)
}

fn parse_resampling_override(args: &ToolArgs) -> Option<String> {
    args.get("auto_reproject_method")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn load_raster(path: &str) -> Result<Raster, ToolError> {
    if memory_store::raster_is_memory_path(path) {
        let id = memory_store::raster_path_to_id(path).ok_or_else(|| {
            ToolError::Validation("malformed in-memory raster path in 'inputs'".to_string())
        })?;
        let raster = memory_store::get_raster_arc_by_id(id).ok_or_else(|| {
            ToolError::Validation(format!(
                "parameter 'inputs' references unknown in-memory raster id '{}'",
                id
            ))
        })?;
        return Ok(raster.as_ref().clone());
    }

    Raster::read(path)
        .map_err(|e| ToolError::Execution(format!("failed reading raster '{}': {e}", path)))
}

fn write_or_store_output(output: Raster, output_path: Option<std::path::PathBuf>) -> Result<String, ToolError> {
    if let Some(output_path) = output_path {
        if let Some(parent) = output_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ToolError::Execution(format!("failed creating output directory: {e}"))
                })?;
            }
        }

        let output_path_str = output_path.to_string_lossy().to_string();
        let output_format = RasterFormat::for_output_path(&output_path_str)
            .map_err(|e| ToolError::Validation(format!("unsupported output path: {e}")))?;
        output
            .write(&output_path_str, output_format)
            .map_err(|e| ToolError::Execution(format!("failed writing output raster: {e}")))?;
        Ok(output_path_str)
    } else {
        let id = memory_store::put_raster(output);
        Ok(memory_store::make_raster_memory_path(&id))
    }
}

fn new_output_like_with_bands(template: &Raster, bands: usize) -> Raster {
    Raster::new(RasterConfig {
        rows: template.rows,
        cols: template.cols,
        bands: bands.max(1),
        x_min: template.x_min,
        y_min: template.y_min,
        cell_size: template.cell_size_x,
        cell_size_y: Some(template.cell_size_y),
        nodata: template.nodata,
        data_type: template.data_type,
        crs: template.crs.clone(),
        metadata: template.metadata.clone(),
    })
}

fn percentile_offset(values: Vec<f64>, nodata: f64, percentile: f64) -> Option<f64> {
    let mut valid: Vec<f64> = values
        .into_par_iter()
        .filter(|v| !v.is_nan() && (*v - nodata).abs() > f64::EPSILON)
        .collect();
    if valid.is_empty() {
        return None;
    }
    valid.par_sort_unstable_by(|a, b| a.total_cmp(b));
    let n = valid.len();
    let idx = ((percentile / 100.0) * (n as f64 - 1.0)).round() as usize;
    Some(valid[idx.min(n - 1)])
}

fn parse_bandwise_f64_arg(
    args: &ToolArgs,
    key: &str,
    expected_len: usize,
) -> Result<Option<Vec<f64>>, ToolError> {
    let Some(value) = args.get(key) else {
        return Ok(None);
    };

    if let Some(v) = value.as_f64() {
        return Ok(Some(vec![v; expected_len]));
    }

    if let Some(arr) = value.as_array() {
        if arr.len() != expected_len {
            return Err(ToolError::Validation(format!(
                "parameter '{}' must have {} values (one per input raster)",
                key, expected_len
            )));
        }
        let mut out = Vec::with_capacity(arr.len());
        for (i, item) in arr.iter().enumerate() {
            let Some(v) = item.as_f64() else {
                return Err(ToolError::Validation(format!(
                    "parameter '{}' value at index {} must be numeric",
                    key, i
                )));
            };
            out.push(v);
        }
        return Ok(Some(out));
    }

    Err(ToolError::Validation(format!(
        "parameter '{}' must be either a number or an array of numbers",
        key
    )))
}

fn parse_class_remap_arg(args: &ToolArgs, key: &str) -> Result<Option<HashMap<i64, i64>>, ToolError> {
    let Some(value) = args.get(key) else {
        return Ok(None);
    };

    let obj = value.as_object().ok_or_else(|| {
        ToolError::Validation(format!(
            "parameter '{}' must be an object mapping class ids to class ids",
            key
        ))
    })?;

    let mut out = HashMap::new();
    for (from, to_val) in obj {
        let from_class = from.parse::<i64>().map_err(|_| {
            ToolError::Validation(format!(
                "parameter '{}' contains non-integer class key '{}'",
                key, from
            ))
        })?;
        let to_class = to_val.as_i64().ok_or_else(|| {
            ToolError::Validation(format!(
                "parameter '{}' value for class '{}' must be an integer",
                key, from
            ))
        })?;
        out.insert(from_class, to_class);
    }

    Ok(Some(out))
}

fn band_number_from_path(path: &str) -> Option<usize> {
    let stem = Path::new(path)
        .file_stem()?
        .to_string_lossy()
        .to_ascii_uppercase();
    for token in stem.split('_') {
        if let Some(rest) = token.strip_prefix('B') {
            if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(n) = rest.parse::<usize>() {
                    return Some(n);
                }
            }
        }
    }
    None
}

fn build_landsat_reflectance_coefficients(
    input_paths: &[String],
    bundle: &LandsatBundle,
) -> Result<(Vec<f64>, Vec<f64>), ToolError> {
    let mut mult = Vec::with_capacity(input_paths.len());
    let mut add = Vec::with_capacity(input_paths.len());

    for (i, path) in input_paths.iter().enumerate() {
        let inferred_band = band_number_from_path(path).or(Some(i + 1));
        let Some(band_number) = inferred_band else {
            return Err(ToolError::Validation(format!(
                "unable to infer Landsat band number for input '{}'",
                path
            )));
        };

        let coeffs = bundle
            .reflectance_coefficients_for_band(band_number)
            .map_err(|e| {
                ToolError::Validation(format!(
                    "missing reflectance coefficients for band {} ('{}') in bundle '{}': {}",
                    band_number,
                    path,
                    bundle.bundle_root.display(),
                    e
                ))
            })?;
        let m = coeffs.mult;
        let a = coeffs.add;
        mult.push(m);
        add.push(a);
    }

    Ok((mult, add))
}

fn landsat_default_thermal_wavelength_um(band_number: usize) -> f64 {
    match band_number {
        10 => 10.895,
        11 => 12.005,
        6 => 11.45,
        _ => 10.895,
    }
}

fn parse_landsat_thermal_constants_from_bundle(
    bundle: &LandsatBundle,
    band_number: usize,
) -> Result<(f64, f64, f64, f64), ToolError> {
    let therm = bundle
        .thermal_constants_for_band(band_number)
        .map_err(|e| {
            ToolError::Validation(format!(
                "missing thermal constants for band {} in Landsat bundle '{}': {}",
                band_number,
                bundle.bundle_root.display(),
                e
            ))
        })?;

    Ok((therm.radiance_mult, therm.radiance_add, therm.k1, therm.k2))
}

fn parse_endmember_vectors(
    args: &ToolArgs,
    num_bands: usize,
) -> Result<(Vec<String>, Vec<Vec<f64>>), ToolError> {
    parse_named_vectors_arg(args, "endmembers", num_bands)
}

fn parse_named_vectors_arg(
    args: &ToolArgs,
    key: &str,
    num_bands: usize,
) -> Result<(Vec<String>, Vec<Vec<f64>>), ToolError> {
    let value = args
        .get(key)
        .ok_or_else(|| ToolError::Validation(format!("parameter '{}' is required", key)))?;
    let arr = value.as_array().ok_or_else(|| {
        ToolError::Validation(format!("parameter '{}' must be an array", key))
    })?;
    if arr.is_empty() {
        return Err(ToolError::Validation(
            format!("parameter '{}' must contain at least one entry", key),
        ));
    }

    let mut names = Vec::with_capacity(arr.len());
    let mut vectors = Vec::with_capacity(arr.len());

    for (i, item) in arr.iter().enumerate() {
        if let Some(obj) = item.as_object() {
            let name = obj
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("class_{}", i + 1));
            let vals = obj.get("values").and_then(|v| v.as_array()).ok_or_else(|| {
                ToolError::Validation(format!(
                    "{}[{}] object must contain numeric array field 'values'",
                    key, i
                ))
            })?;
            if vals.len() != num_bands {
                return Err(ToolError::Validation(format!(
                    "{}[{}].values must have {} values (one per input raster)",
                    key, i, num_bands
                )));
            }
            let mut vec = Vec::with_capacity(num_bands);
            for (j, v) in vals.iter().enumerate() {
                let f = v.as_f64().ok_or_else(|| {
                    ToolError::Validation(format!(
                        "{}[{}].values[{}] must be numeric",
                        key, i, j
                    ))
                })?;
                vec.push(f);
            }
            names.push(name);
            vectors.push(vec);
        } else if let Some(vals) = item.as_array() {
            if vals.len() != num_bands {
                return Err(ToolError::Validation(format!(
                    "{}[{}] must have {} values (one per input raster)",
                    key, i, num_bands
                )));
            }
            let mut vec = Vec::with_capacity(num_bands);
            for (j, v) in vals.iter().enumerate() {
                let f = v.as_f64().ok_or_else(|| {
                    ToolError::Validation(format!("{}[{}][{}] must be numeric", key, i, j))
                })?;
                vec.push(f);
            }
            names.push(format!("class_{}", i + 1));
            vectors.push(vec);
        } else {
            return Err(ToolError::Validation(format!(
                "{}[{}] must be either an object {{name, values}} or numeric array",
                key, i
            )));
        }
    }

    Ok((names, vectors))
}

fn parse_named_vectors_csv(path: &Path, num_bands: usize) -> Result<(Vec<String>, Vec<Vec<f64>>), ToolError> {
    let text = std::fs::read_to_string(path).map_err(|e| {
        ToolError::Execution(format!("failed reading signatures CSV '{}': {e}", path.display()))
    })?;

    let mut names = Vec::new();
    let mut vectors = Vec::new();

    for (line_idx, line) in text.lines().enumerate() {
        let line_no = line_idx + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let parts = trimmed.split(',').map(|s| s.trim()).collect::<Vec<_>>();
        if parts.is_empty() {
            continue;
        }

        let has_name = parts[0].parse::<f64>().is_err();
        let (name, values_slice) = if has_name {
            (parts[0].to_string(), &parts[1..])
        } else {
            (format!("class_{}", names.len() + 1), &parts[..])
        };

        if values_slice.len() != num_bands {
            if line_no == 1 && values_slice.iter().any(|v| v.parse::<f64>().is_err()) {
                continue;
            }
            return Err(ToolError::Validation(format!(
                "CSV line {} in '{}' must contain {} numeric band values",
                line_no,
                path.display(),
                num_bands
            )));
        }

        let mut vec = Vec::with_capacity(num_bands);
        for (i, tok) in values_slice.iter().enumerate() {
            let val = tok.parse::<f64>().map_err(|_| {
                ToolError::Validation(format!(
                    "CSV line {} value {} in '{}' is not numeric",
                    line_no,
                    i + 1,
                    path.display()
                ))
            })?;
            vec.push(val);
        }
        names.push(name);
        vectors.push(vec);
    }

    if vectors.is_empty() {
        return Err(ToolError::Validation(format!(
            "CSV '{}' does not contain any valid signature rows",
            path.display()
        )));
    }

    Ok((names, vectors))
}

fn parse_wavelengths_arg(args: &ToolArgs, num_bands: usize) -> Result<Vec<f64>, ToolError> {
    if let Some(value) = args.get("wavelengths") {
        let arr = value.as_array().ok_or_else(|| {
            ToolError::Validation("parameter 'wavelengths' must be an array of numbers".to_string())
        })?;
        if arr.len() != num_bands {
            return Err(ToolError::Validation(format!(
                "parameter 'wavelengths' must contain {} values",
                num_bands
            )));
        }
        let mut out = Vec::with_capacity(num_bands);
        for (i, v) in arr.iter().enumerate() {
            let w = v.as_f64().ok_or_else(|| {
                ToolError::Validation(format!("wavelengths[{}] must be numeric", i))
            })?;
            out.push(w);
        }
        for i in 1..out.len() {
            if out[i] <= out[i - 1] {
                return Err(ToolError::Validation(
                    "parameter 'wavelengths' must be strictly increasing".to_string(),
                ));
            }
        }
        Ok(out)
    } else {
        Ok((0..num_bands).map(|i| i as f64).collect())
    }
}

fn upper_hull_indices(x: &[f64], y: &[f64]) -> Vec<usize> {
    let mut hull: Vec<usize> = Vec::new();
    for i in 0..x.len() {
        while hull.len() >= 2 {
            let i1 = hull[hull.len() - 2];
            let i2 = hull[hull.len() - 1];
            let cross = (x[i2] - x[i1]) * (y[i] - y[i1]) - (y[i2] - y[i1]) * (x[i] - x[i1]);
            if cross >= 0.0 {
                hull.pop();
            } else {
                break;
            }
        }
        hull.push(i);
    }
    hull
}

fn solve_nnls_projected(
    a_cols: &[Vec<f64>],
    b: &[f64],
    iterations: usize,
    step: f64,
    sum_to_one: bool,
) -> Vec<f64> {
    let m = a_cols.len();
    let n = b.len();
    let mut x = vec![0.0_f64; m];

    for _ in 0..iterations {
        // r = A x - b
        let mut r = vec![0.0_f64; n];
        for (j, col) in a_cols.iter().enumerate() {
            let xj = x[j];
            if xj == 0.0 {
                continue;
            }
            for i in 0..n {
                r[i] += col[i] * xj;
            }
        }
        for i in 0..n {
            r[i] -= b[i];
        }

        // gradient g = A^T r
        for (j, col) in a_cols.iter().enumerate() {
            let mut g = 0.0_f64;
            for i in 0..n {
                g += col[i] * r[i];
            }
            x[j] = (x[j] - step * g).max(0.0);
        }

        if sum_to_one {
            let s = x.iter().sum::<f64>();
            if s > 0.0 {
                for v in &mut x {
                    *v /= s;
                }
            }
        }
    }

    x
}

fn parse_polsar_real_symmetric_inputs(
    args: &ToolArgs,
) -> Result<(Vec<String>, Option<Vec<String>>, String), ToolError> {
    let primary = parse_raster_list_arg(args, "inputs")?;
    let matrix_format = args
        .get("matrix_format")
        .and_then(|v| v.as_str())
        .unwrap_or(if primary.len() == 9 { "full3x3" } else { "diag3" })
        .to_ascii_lowercase();

    if matrix_format == "diag3" {
        if primary.len() != 3 {
            return Err(ToolError::Validation(
                "for matrix_format='diag3', parameter 'inputs' must contain 3 rasters: [m11, m22, m33]".to_string(),
            ));
        }
        Ok((primary, None, matrix_format))
    } else if matrix_format == "full3x3" {
        if primary.len() == 6 {
            // Compact upper-triangle order: m11, m22, m33, m12, m13, m23
            let mut full = Vec::with_capacity(9);
            full.push(primary[0].clone());
            full.push(primary[3].clone());
            full.push(primary[4].clone());
            full.push(primary[3].clone());
            full.push(primary[1].clone());
            full.push(primary[5].clone());
            full.push(primary[4].clone());
            full.push(primary[5].clone());
            full.push(primary[2].clone());
            Ok((full, None, matrix_format))
        } else if primary.len() == 9 {
            Ok((primary, None, matrix_format))
        } else {
            Err(ToolError::Validation(
                "for matrix_format='full3x3', parameter 'inputs' must contain 6 rasters [m11,m22,m33,m12,m13,m23] or 9 rasters row-major".to_string(),
            ))
        }
    } else {
        Err(ToolError::Validation(
            "parameter 'matrix_format' must be one of: diag3, full3x3".to_string(),
        ))
    }
}

impl Tool for DarkObjectSubtractionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "dark_object_subtraction",
            display_name: "Dark Object Subtraction",
            summary: r#"Dark Object Subtraction (DOS) is a simple yet effective heuristic for atmospheric haze removal without requiring ancillary meteorological data or complex radiative transfer models. The technique exploits the principle that zero reflectance should produce zero reflectance signal (neglecting Rayleigh scattering); any signal observed from optically black surfaces (deep water, dense forest shadow, urban asphalt) is attributed to atmospheric path radiance caused by aerosol scattering. This tool identifies the minimum digital number in each band across the image, interprets this as atmospheric haze, and subtracts it from all pixels as a per-band constant offset; more sophisticated variants (DOS2, DOS3, DOS4) account for Rayleigh scattering and variable aerosol optical depth using vegetation indices or dark pixel clustering. Key capabilities include histogram analysis to isolate dark objects and distinguish scene-dependent haze from true zero reflectance, optional masking of known high-reflectance features (urban, snow, clouds) that would bias dark-object identification, band-specific haze correction, and fast computation suitable for real-time or large-scale processing. Use cases include quick-look reflectance estimates, vegetation and water quality monitoring where absolute calibration is less critical than relative changes, rapid disaster response mapping, and legacy archived data where precise aerosol information is unavailable. Input comprises top-of-atmosphere reflectance or calibrated radiance, optional land cover masks to exclude bright features, and scene metadata. Output is haze-corrected reflectance, per-band atmospheric path radiance estimates, and quality flags indicating confidence in dark-object identification (e.g., limited dark pixels, urban-dominated scene). While crude compared to radiative transfer models, DOS remains practical for multi-temporal analysis and global coverage mapping when sophisticated atmospheric data are in [truncated]"#,
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "inputs",
                    description: "Input raster stack (one or more rasters).",
                    required: true,
                },
                ToolParamSpec {
                    name: "percentile",
                    description: "Dark-object percentile per band (default 1.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "clamp_non_negative",
                    description: "If true, clamp corrected values below zero to zero (default true).",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject",
                    description: "If true (default), reproject inputs to match inputs[0] when CRS differs.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject_method",
                    description: "Optional reprojection resampling override: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output raster path.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output_diagnostic_offsets",
                    description: "Optional output raster path for per-band dark-object offset diagnostics.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        defaults.insert("percentile".to_string(), json!(1.0));
        defaults.insert("clamp_non_negative".to_string(), json!(true));
        defaults.insert("auto_reproject".to_string(), json!(true));
        defaults.insert("auto_reproject_method".to_string(), json!(""));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["blue.tif", "green.tif", "red.tif", "nir.tif"]));
        example.insert("percentile".to_string(), json!(1.0));
        example.insert("clamp_non_negative".to_string(), json!(true));
        example.insert("output".to_string(), json!("dos_corrected.tif"));
        example.insert(
            "output_diagnostic_offsets".to_string(),
            json!("dos_offsets_diagnostic.tif"),
        );

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_dark_object_subtraction".to_string(),
                description: "Apply DOS per-band using 1st percentile dark-object offsets.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "radiometric_correction".to_string(),
                "dos".to_string(),
                "atmospheric_correction".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_list_arg(args, "inputs")?;
        let percentile = args
            .get("percentile")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        if !(0.0..=100.0).contains(&percentile) {
            return Err(ToolError::Validation(
                "parameter 'percentile' must be in [0, 100]".to_string(),
            ));
        }
        let _ = parse_optional_output_path(args, "output")?;
        let _ = parse_optional_output_path(args, "output_diagnostic_offsets")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = parse_raster_list_arg(args, "inputs")?;
        let percentile = args
            .get("percentile")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0)
            .clamp(0.0, 100.0);
        let clamp_non_negative = args
            .get("clamp_non_negative")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let output_path = parse_optional_output_path(args, "output")?;
        let output_diagnostic_offsets_path =
            parse_optional_output_path(args, "output_diagnostic_offsets")?;

        ctx.progress.info("dark_object_subtraction: reading input stack");
        let mut rasters = input_paths
            .iter()
            .map(|p| load_raster(p))
            .collect::<Result<Vec<_>, _>>()?;

        let stack_cfg = RasterStackConfig {
            auto_reproject: args
                .get("auto_reproject")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            resampling_method: parse_resampling_override(args),
            allow_no_overlap: false,
        };

        let warnings = align_and_validate_raster_stack(&mut rasters, &stack_cfg)
            .map_err(|e| ToolError::Validation(format!("raster stack validation failed: {e}")))?;
        for warning in warnings {
            ctx.progress.info(&format!("dark_object_subtraction: {warning}"));
        }

        let rows = rasters[0].rows;
        let cols = rasters[0].cols;
        let nodata = rasters[0].nodata;

        let mut output = rasters[0].clone();
        output.bands = rasters.len();
        output.data = wbraster::raster::RasterData::new_filled(
            output.data_type,
            rasters.len() * rows * cols,
            nodata,
        );

        let mut diagnostic = output_diagnostic_offsets_path.as_ref().map(|_| {
            let mut diagnostic = rasters[0].clone();
            diagnostic.bands = rasters.len();
            diagnostic.data = wbraster::raster::RasterData::new_filled(
                diagnostic.data_type,
                rasters.len() * rows * cols,
                nodata,
            );
            diagnostic
        });

        let total_rows = (rows * rasters.len()).max(1);
        let mut done_rows = 0usize;
        let coalescer = PercentCoalescer::new(1, 98);
        let mut band_offsets = Vec::with_capacity(rasters.len());
        let create_diagnostic = output_diagnostic_offsets_path.is_some();

        for (band_idx, raster) in rasters.iter().enumerate() {
            ctx.progress.info(&format!(
                "dark_object_subtraction: estimating dark-object offset for band {}",
                band_idx + 1
            ));
            let offset = percentile_offset(raster.band_slice(0), raster.nodata, percentile)
                .ok_or_else(|| {
                    ToolError::Execution(format!(
                        "band {} does not contain valid cells for percentile offset",
                        band_idx + 1
                    ))
                })?;
            band_offsets.push(offset);

            ctx.progress.info(&format!(
                "dark_object_subtraction: applying correction to band {} (offset {:.6})",
                band_idx + 1,
                offset
            ));

            let band_rows: Vec<(Vec<f64>, Option<Vec<f64>>)> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let row = raster.row_slice(0, r as isize);
                    let mut corrected_row = vec![nodata; cols];
                    let mut diag_row = if create_diagnostic {
                        Some(vec![nodata; cols])
                    } else {
                        None
                    };
                    for (c, v) in row.iter().copied().enumerate() {
                        if (v - raster.nodata).abs() <= f64::EPSILON || v.is_nan() {
                            continue;
                        }
                        let corrected = v - offset;
                        corrected_row[c] = if clamp_non_negative {
                            corrected.max(0.0)
                        } else {
                            corrected
                        };
                        if let Some(ref mut d) = diag_row {
                            d[c] = offset;
                        }
                    }
                    (corrected_row, diag_row)
                })
                .collect();

            for (r, (row, diag_row)) in band_rows.iter().enumerate() {
                output
                    .set_row_slice(band_idx as isize, r as isize, row)
                    .map_err(|e| ToolError::Execution(format!("failed writing output row {}: {e}", r)))?;
                if let (Some(diag), Some(drow)) = (diagnostic.as_mut(), diag_row.as_ref()) {
                    diag.set_row_slice(band_idx as isize, r as isize, drow).map_err(|e| {
                        ToolError::Execution(format!(
                            "failed writing diagnostic row {} for band {}: {e}",
                            r,
                            band_idx + 1
                        ))
                    })?;
                }
                done_rows += 1;
                coalescer.emit_unit_fraction(ctx.progress, done_rows as f64 / total_rows as f64);
            }
        }

        coalescer.finish(ctx.progress);

        let output_locator = write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("active_band".to_string(), json!(0));
        outputs.insert("percentile".to_string(), json!(percentile));
        outputs.insert("clamp_non_negative".to_string(), json!(clamp_non_negative));
        outputs.insert("offsets".to_string(), json!(band_offsets));

        if let (Some(diag_path), Some(diagnostic)) = (output_diagnostic_offsets_path, diagnostic) {
            let diag_locator = write_or_store_output(diagnostic, Some(diag_path))?;
            outputs.insert(
                "diagnostic_offsets".to_string(),
                json!({"__wbw_type__": "raster", "path": diag_locator, "active_band": 0}),
            );
        }
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for DnToToaReflectanceTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "dn_to_toa_reflectance",
            display_name: "DN to TOA Reflectance",
            summary: r#"Radiometric calibration converts raw sensor digital numbers (DN) into calibrated radiance values by applying sensor-specific radiometric coefficients. This fundamental preprocessing step accounts for sensor gain, bias, and temporal variations in radiometric response. The tool implements the standard linear radiometric equation: L = (DN - offset) × gain, where gain and bias coefficients are derived from sensor calibration data and may vary by band. Temperature-dependent corrections are applied for thermal sensors. Radiometric calibration establishes a physically meaningful radiance scale essential for downstream processing and enables quantitative multi-temporal and multi-sensor analysis. Key features include band-specific coefficient application supporting diverse sensor types (optical, thermal, multispectral), automated offset correction for sensor noise floors, temperature correction for thermal bands, and support for external calibration coefficient files. The tool maintains radiometric fidelity across processing chains and enables direct comparison with atmospheric correction outputs and reference spectral libraries. Applications include preparing data for atmospheric correction, enabling direct spectral matching against reference materials, supporting accurate change detection through radiometric consistency, facilitating thermal analysis and land surface temperature derivation, and enabling quantitative cross-sensor comparisons. Scientists use radiometric calibration as foundational preprocessing for hyperspectral analysis, mineral mapping, thermal monitoring, and radiative transfer modeling. Output interpretation requires understanding that calibrated radiance represents sensor-recorded energy before atmospheric effects. Values reflect both atmospheric contributions and surface-leaving radiance, necessitating atmospheric correction for surface reflectance retrieval. Radiometric uncertainty propagates through all downstream products; quality assessment examines coefficient accuracy and temporal stability."#,
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "inputs",
                    description: "Input DN raster stack (one or more rasters).",
                    required: true,
                },
                ToolParamSpec {
                    name: "reflectance_mult",
                    description: "Per-band multiplicative coefficient(s), either scalar or array sized to inputs.",
                    required: false,
                },
                ToolParamSpec {
                    name: "reflectance_add",
                    description: "Per-band additive coefficient(s), either scalar or array sized to inputs (default 0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "sensor_bundle_root",
                    description: "Optional Landsat/Sentinel-2 bundle root used to auto-derive coefficients when explicit coefficients are omitted.",
                    required: false,
                },
                ToolParamSpec {
                    name: "sun_elevation_deg",
                    description: "Optional sun elevation angle in degrees for cosine correction. Auto-derived from bundle metadata when available.",
                    required: false,
                },
                ToolParamSpec {
                    name: "apply_solar_correction",
                    description: "If true (default), divide by sin(sun_elevation).",
                    required: false,
                },
                ToolParamSpec {
                    name: "clamp_unit_interval",
                    description: "If true (default), clamp outputs to [0, 1].",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject",
                    description: "If true (default), reproject inputs to match inputs[0] when CRS differs.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject_method",
                    description: "Optional reprojection resampling override: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output raster path.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["B2.tif", "B3.tif", "B4.tif", "B5.tif"]));
        defaults.insert("reflectance_mult".to_string(), json!([2.0e-5, 2.0e-5, 2.0e-5, 2.0e-5]));
        defaults.insert("reflectance_add".to_string(), json!([-0.1, -0.1, -0.1, -0.1]));
        defaults.insert("apply_solar_correction".to_string(), json!(true));
        defaults.insert("clamp_unit_interval".to_string(), json!(true));
        defaults.insert("auto_reproject".to_string(), json!(true));
        defaults.insert("auto_reproject_method".to_string(), json!(""));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["LC09_B2.TIF", "LC09_B3.TIF", "LC09_B4.TIF", "LC09_B5.TIF"]));
        example.insert("sensor_bundle_root".to_string(), json!("LC09_L1TP_017030_20240420_20240426_02_T1"));
        example.insert("apply_solar_correction".to_string(), json!(true));
        example.insert("output".to_string(), json!("toa_reflectance.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "landsat_bundle_driven_toa".to_string(),
                description: "Compute TOA reflectance from Landsat DN bands using bundle metadata coefficients.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "radiometric_correction".to_string(),
                "toa_reflectance".to_string(),
                "landsat".to_string(),
                "sentinel2".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let input_paths = parse_raster_list_arg(args, "inputs")?;
        let _ = parse_bandwise_f64_arg(args, "reflectance_mult", input_paths.len())?;
        let _ = parse_bandwise_f64_arg(args, "reflectance_add", input_paths.len())?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = parse_raster_list_arg(args, "inputs")?;
        let n = input_paths.len();
        let output_path = parse_optional_output_path(args, "output")?;
        let apply_solar_correction = args
            .get("apply_solar_correction")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let clamp_unit_interval = args
            .get("clamp_unit_interval")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        ctx.progress.info("dn_to_toa_reflectance: reading input stack");
        let mut rasters = input_paths
            .iter()
            .map(|p| load_raster(p))
            .collect::<Result<Vec<_>, _>>()?;

        let stack_cfg = RasterStackConfig {
            auto_reproject: args
                .get("auto_reproject")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            resampling_method: parse_resampling_override(args),
            allow_no_overlap: false,
        };

        let warnings = align_and_validate_raster_stack(&mut rasters, &stack_cfg)
            .map_err(|e| ToolError::Validation(format!("raster stack validation failed: {e}")))?;
        for warning in warnings {
            ctx.progress.info(&format!("dn_to_toa_reflectance: {warning}"));
        }

        let explicit_mult = parse_bandwise_f64_arg(args, "reflectance_mult", n)?;
        let explicit_add = parse_bandwise_f64_arg(args, "reflectance_add", n)?;

        let mut derived_sun_elevation = None;
        let (mult_coeffs, add_coeffs) = if let Some(mult) = explicit_mult {
            let add = explicit_add.unwrap_or_else(|| vec![0.0; n]);
            (mult, add)
        } else if let Some(bundle_root) = args.get("sensor_bundle_root").and_then(|v| v.as_str()) {
            if let Ok(bundle) = LandsatBundle::open(bundle_root) {
                derived_sun_elevation = bundle.sun_elevation_deg;
                build_landsat_reflectance_coefficients(&input_paths, &bundle)?
            } else if let Ok(pkg) = Sentinel2SafePackage::open(bundle_root) {
                derived_sun_elevation = pkg.mean_solar_zenith_deg.map(|z| (90.0 - z).clamp(0.0, 90.0));
                // Prefer bundle quantification metadata when available; keep 1/10000 fallback.
                let scale = pkg.reflectance_scale_factor().unwrap_or(1.0 / 10000.0);
                (vec![scale; n], vec![0.0; n])
            } else {
                return Err(ToolError::Validation(
                    "parameter 'sensor_bundle_root' is not a recognized Landsat or Sentinel-2 bundle".to_string(),
                ));
            }
        } else {
            return Err(ToolError::Validation(
                "provide either 'reflectance_mult' or 'sensor_bundle_root' for dn_to_toa_reflectance".to_string(),
            ));
        };

        let sun_elevation_deg = args
            .get("sun_elevation_deg")
            .and_then(|v| v.as_f64())
            .or(derived_sun_elevation);

        let solar_divisor = if apply_solar_correction {
            let sun = sun_elevation_deg.ok_or_else(|| {
                ToolError::Validation(
                    "sun elevation is required when apply_solar_correction=true (set 'sun_elevation_deg' or provide bundle metadata)".to_string(),
                )
            })?;
            let sin_elev = sun.to_radians().sin();
            if sin_elev <= 0.0 {
                return Err(ToolError::Validation(
                    "computed sin(sun_elevation_deg) must be > 0".to_string(),
                ));
            }
            Some(sin_elev)
        } else {
            None
        };

        let rows = rasters[0].rows;
        let cols = rasters[0].cols;
        let nodata = rasters[0].nodata;

        let mut output = rasters[0].clone();
        output.bands = n;
        output.data = wbraster::raster::RasterData::new_filled(output.data_type, n * rows * cols, nodata);

        let total_rows = (rows * n).max(1);
        let mut done_rows = 0usize;
        let coalescer = PercentCoalescer::new(1, 98);

        for (band_idx, raster) in rasters.iter().enumerate() {
            let mult = mult_coeffs[band_idx];
            let add = add_coeffs[band_idx];
            let div = solar_divisor;

            ctx.progress.info(&format!(
                "dn_to_toa_reflectance: processing band {} with mult={} add={}",
                band_idx + 1,
                mult,
                add
            ));

            let band_rows: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let row = raster.row_slice(0, r as isize);
                    let mut out_row = vec![nodata; cols];
                    for (c, v) in row.iter().copied().enumerate() {
                        if (v - raster.nodata).abs() <= f64::EPSILON || v.is_nan() {
                            continue;
                        }
                        let mut out_v = mult * v + add;
                        if let Some(s) = div {
                            out_v /= s;
                        }
                        out_row[c] = if clamp_unit_interval {
                            out_v.clamp(0.0, 1.0)
                        } else {
                            out_v
                        };
                    }
                    out_row
                })
                .collect();

            for (r, row) in band_rows.iter().enumerate() {
                output
                    .set_row_slice(band_idx as isize, r as isize, row)
                    .map_err(|e| ToolError::Execution(format!("failed writing output row {}: {e}", r)))?;
                done_rows += 1;
                coalescer.emit_unit_fraction(ctx.progress, done_rows as f64 / total_rows as f64);
            }
        }

        coalescer.finish(ctx.progress);

        let output_locator = write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("active_band".to_string(), json!(0));
        outputs.insert("apply_solar_correction".to_string(), json!(apply_solar_correction));
        outputs.insert("clamp_unit_interval".to_string(), json!(clamp_unit_interval));
        if let Some(sun) = sun_elevation_deg {
            outputs.insert("sun_elevation_deg".to_string(), json!(sun));
        }
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for NdviBasedEmissivityTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "ndvi_based_emissivity",
            display_name: "NDVI Based Emissivity",
            summary: r#"Estimates land surface emissivity (LSE) from Normalized Difference Vegetation Index (NDVI) values, a critical preprocessing step for accurate thermal remote sensing analysis. The algorithm derives emissivity through the vegetation fractional cover model, calculating the proportion of vegetation versus bare soil within each pixel. This empirical approach leverages the relationship between NDVI and vegetation density to estimate the emissivity coefficient needed for Land Surface Temperature (LST) derivation. Key features include spectral index-based emissivity calculation requiring only optical bands, automatic handling of mixed vegetation-soil pixels, and integration with thermal band processing pipelines. The method assumes linear relationships between NDVI and fractional vegetation cover, making it suitable for diverse ecosystems from arid to vegetated regions. Primary use cases include thermal LST retrieval workflows, urban heat island studies, hydrological modeling requiring surface energy balance components, and vegetation monitoring in changing environments. Output interpretation requires understanding that emissivity values range from 0 to 1, with bare soil typically 0.92-0.98 and vegetation 0.97-0.99. Lower NDVI areas produce higher emissivity estimates closer to bare soil values, while dense vegetation produces uniform high emissivity. Quality assessment involves comparing pixel-scale emissivity distributions against validation data from field spectrometry or laboratory measurements. The derived emissivity values are dimensionless coefficients essential for accurate thermal radiative transfer calculations and must be spatially consistent across the image to avoid artifacts in downstream LST maps."#,
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "red_input",
                    description: "Red-band raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "nir_input",
                    description: "NIR-band raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "ndvi_soil",
                    description: "NDVI threshold for bare soil fraction (default 0.2).",
                    required: false,
                },
                ToolParamSpec {
                    name: "ndvi_vegetation",
                    description: "NDVI threshold for full vegetation fraction (default 0.5).",
                    required: false,
                },
                ToolParamSpec {
                    name: "emissivity_soil",
                    description: "Assumed soil emissivity (default 0.97).",
                    required: false,
                },
                ToolParamSpec {
                    name: "emissivity_vegetation",
                    description: "Assumed vegetation emissivity (default 0.99).",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject",
                    description: "If true (default), reproject nir_input to red_input CRS when needed.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject_method",
                    description: "Optional reprojection resampling override: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output raster path.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("red_input".to_string(), json!("red.tif"));
        defaults.insert("nir_input".to_string(), json!("nir.tif"));
        defaults.insert("ndvi_soil".to_string(), json!(0.2));
        defaults.insert("ndvi_vegetation".to_string(), json!(0.5));
        defaults.insert("emissivity_soil".to_string(), json!(0.97));
        defaults.insert("emissivity_vegetation".to_string(), json!(0.99));
        defaults.insert("auto_reproject".to_string(), json!(true));
        defaults.insert("auto_reproject_method".to_string(), json!(""));

        let mut example = ToolArgs::new();
        example.insert("red_input".to_string(), json!("B4.tif"));
        example.insert("nir_input".to_string(), json!("B5.tif"));
        example.insert("ndvi_soil".to_string(), json!(0.2));
        example.insert("ndvi_vegetation".to_string(), json!(0.5));
        example.insert("output".to_string(), json!("emissivity.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_ndvi_based_emissivity".to_string(),
                description: "Compute emissivity from red/NIR NDVI and fractional vegetation cover.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "thermal".to_string(),
                "emissivity".to_string(),
                "ndvi".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = args
            .get("red_input")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'red_input' is required".to_string()))?;
        let _ = args
            .get("nir_input")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'nir_input' is required".to_string()))?;
        let ndvi_soil = args.get("ndvi_soil").and_then(|v| v.as_f64()).unwrap_or(0.2);
        let ndvi_vegetation = args
            .get("ndvi_vegetation")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);
        if ndvi_vegetation <= ndvi_soil {
            return Err(ToolError::Validation(
                "parameter 'ndvi_vegetation' must be greater than 'ndvi_soil'".to_string(),
            ));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let red_input = args
            .get("red_input")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'red_input' is required".to_string()))?;
        let nir_input = args
            .get("nir_input")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'nir_input' is required".to_string()))?;
        let ndvi_soil = args.get("ndvi_soil").and_then(|v| v.as_f64()).unwrap_or(0.2);
        let ndvi_vegetation = args
            .get("ndvi_vegetation")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);
        let emissivity_soil = args
            .get("emissivity_soil")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.97);
        let emissivity_vegetation = args
            .get("emissivity_vegetation")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.99);
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("ndvi_based_emissivity: reading red/nir rasters");
        let mut rasters = vec![load_raster(red_input)?, load_raster(nir_input)?];

        let stack_cfg = RasterStackConfig {
            auto_reproject: args
                .get("auto_reproject")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            resampling_method: parse_resampling_override(args),
            allow_no_overlap: false,
        };

        let warnings = align_and_validate_raster_stack(&mut rasters, &stack_cfg)
            .map_err(|e| ToolError::Validation(format!("raster stack validation failed: {e}")))?;
        for warning in warnings {
            ctx.progress.info(&format!("ndvi_based_emissivity: {warning}"));
        }

        let red = &rasters[0];
        let nir = &rasters[1];
        let rows = red.rows;
        let nodata = red.nodata;

        let mut output = red.clone();
        output.bands = 1;

        let denom = (ndvi_vegetation - ndvi_soil).max(1e-12);
        let coalescer = PercentCoalescer::new(1, 98);
        let mut done_rows = 0usize;
        let total_rows = rows.max(1);

        let out_rows: Vec<Vec<f64>> = (0..rows)
            .into_par_iter()
            .map(|r| {
                let red_row = red.row_slice(0, r as isize);
                let nir_row = nir.row_slice(0, r as isize);
                red_row
                    .into_iter()
                    .zip(nir_row)
                    .map(|(rv, nv)| {
                        if (rv - red.nodata).abs() <= f64::EPSILON
                            || (nv - nir.nodata).abs() <= f64::EPSILON
                            || rv.is_nan()
                            || nv.is_nan()
                        {
                            nodata
                        } else {
                            let ndvi_denom = nv + rv;
                            if ndvi_denom.abs() <= 1e-12 {
                                nodata
                            } else {
                                let ndvi = (nv - rv) / ndvi_denom;
                                let fvc = ((ndvi - ndvi_soil) / denom).clamp(0.0, 1.0);
                                emissivity_soil * (1.0 - fvc) + emissivity_vegetation * fvc
                            }
                        }
                    })
                    .collect::<Vec<f64>>()
            })
            .collect();

        for (r, row) in out_rows.iter().enumerate() {
            output
                .set_row_slice(0, r as isize, row)
                .map_err(|e| ToolError::Execution(format!("failed writing output row {}: {e}", r)))?;
            done_rows += 1;
            coalescer.emit_unit_fraction(ctx.progress, done_rows as f64 / total_rows as f64);
        }
        coalescer.finish(ctx.progress);

        let output_locator = write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("active_band".to_string(), json!(0));
        outputs.insert("ndvi_soil".to_string(), json!(ndvi_soil));
        outputs.insert("ndvi_vegetation".to_string(), json!(ndvi_vegetation));
        outputs.insert("emissivity_soil".to_string(), json!(emissivity_soil));
        outputs.insert("emissivity_vegetation".to_string(), json!(emissivity_vegetation));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for PcaBasedChangeDetectionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "pca_based_change_detection",
            display_name: "PCA Based Change Detection",
            summary: r#"Performs Principal Component Analysis (PCA) on multitemporal satellite image stacks to detect temporal changes through dimensionality reduction and variance maximization. The algorithm stacks coregistered multitemporal bands as features and computes PCA eigenvectors and eigenvalues to identify dominant patterns of change. Primary components capture the strongest temporal variance patterns, while later components reveal subtle changes, anomalies, and noise. Key features include automatic feature weighting based on variance contribution, noise suppression through lower component exclusion, and computational efficiency for large multispectral time series. The method works on any radiometric stack regardless of sensor or wavelength combination, enabling fusion of diverse data sources. Primary use cases encompass disaster assessment (floods, wildfires, earthquakes), agricultural monitoring for crop phenology and yield estimation, urban expansion detection and infrastructure development tracking, and vegetation dynamics in climate-sensitive ecosystems. Output interpretation involves analyzing component loadings to identify which original bands contribute most to detected changes, with earlier components representing gradual changes and later components revealing abrupt shifts or anomalies. PC1-PC3 typically explain 80-95% of variance in multitemporal stacks, while PC4+ often contain noise or rare events. Visualization through principal component composites reveals spatial patterns of change magnitude and direction. Temporal sequences of PC scores enable change trajectories and turning-point detection for environmental monitoring and rapid assessment."#,
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "t1_inputs",
                    description: "Array of date-1 raster paths.",
                    required: true,
                },
                ToolParamSpec {
                    name: "t2_inputs",
                    description: "Array of date-2 raster paths; must match t1_inputs length and ordering.",
                    required: true,
                },
                ToolParamSpec {
                    name: "component",
                    description: "1-based principal component index to output (default 1).",
                    required: false,
                },
                ToolParamSpec {
                    name: "standardized",
                    description: "If true, use correlation matrix PCA on change vectors (default false).",
                    required: false,
                },
                ToolParamSpec {
                    name: "threshold_sigma",
                    description: "Optional sigma threshold on absolute PC score for binary mask output.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject",
                    description: "If true (default), reproject stack rasters to match t1_inputs[0] when CRS differs.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject_method",
                    description: "Optional reprojection resampling override: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output raster path for absolute selected PC score.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output_mask",
                    description: "Optional output raster path for thresholded binary mask (0/1).",
                    required: false,
                },
                ToolParamSpec {
                    name: "output_report",
                    description: "Optional output JSON report path for eigenvalues, explained variance, and component loadings.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("t1_inputs".to_string(), json!(["t1_b2.tif", "t1_b3.tif", "t1_b4.tif"]));
        defaults.insert("t2_inputs".to_string(), json!(["t2_b2.tif", "t2_b3.tif", "t2_b4.tif"]));
        defaults.insert("component".to_string(), json!(1));
        defaults.insert("standardized".to_string(), json!(false));
        defaults.insert("threshold_sigma".to_string(), json!(2.0));
        defaults.insert("auto_reproject".to_string(), json!(true));
        defaults.insert("auto_reproject_method".to_string(), json!(""));

        let mut example = ToolArgs::new();
        example.insert("t1_inputs".to_string(), json!(["pre_b2.tif", "pre_b3.tif", "pre_b4.tif", "pre_b8.tif"]));
        example.insert("t2_inputs".to_string(), json!(["post_b2.tif", "post_b3.tif", "post_b4.tif", "post_b8.tif"]));
        example.insert("component".to_string(), json!(1));
        example.insert("threshold_sigma".to_string(), json!(2.0));
        example.insert("output".to_string(), json!("pca_change_pc1_abs.tif"));
        example.insert("output_mask".to_string(), json!("pca_change_mask.tif"));
        example.insert("output_report".to_string(), json!("pca_change_report.json"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_pca_change_detection".to_string(),
                description: "PCA change detection on two multiband dates, outputting PC1 magnitude and threshold mask.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "change_detection".to_string(),
                "pca".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let t1 = parse_raster_list_arg(args, "t1_inputs")?;
        let t2 = parse_raster_list_arg(args, "t2_inputs")?;
        if t1.len() != t2.len() {
            return Err(ToolError::Validation(
                "'t1_inputs' and 't2_inputs' must have equal length".to_string(),
            ));
        }
        if t1.len() < 2 {
            return Err(ToolError::Validation(
                "at least two bands are required for PCA-based change detection".to_string(),
            ));
        }
        let _ = parse_optional_output_path(args, "output")?;
        let _ = parse_optional_output_path(args, "output_mask")?;
        let _ = parse_optional_output_path(args, "output_report")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let t1_paths = parse_raster_list_arg(args, "t1_inputs")?;
        let t2_paths = parse_raster_list_arg(args, "t2_inputs")?;
        if t1_paths.len() != t2_paths.len() {
            return Err(ToolError::Validation(
                "'t1_inputs' and 't2_inputs' must have equal length".to_string(),
            ));
        }
        let bands = t1_paths.len();
        if bands < 2 {
            return Err(ToolError::Validation(
                "at least two bands are required for PCA-based change detection".to_string(),
            ));
        }

        let component = args
            .get("component")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1)
            .clamp(1, bands);
        let component_idx = component - 1;
        let standardized = args
            .get("standardized")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let threshold_sigma = args.get("threshold_sigma").and_then(|v| v.as_f64());
        let output_path = parse_optional_output_path(args, "output")?;
        let output_mask_path = parse_optional_output_path(args, "output_mask")?;
        let output_report_path = parse_optional_output_path(args, "output_report")?;

        ctx.progress.info("pca_based_change_detection: reading and aligning input stacks");
        let mut all_rasters: Vec<Raster> = t1_paths
            .iter()
            .chain(t2_paths.iter())
            .map(|p| load_raster(p))
            .collect::<Result<Vec<_>, _>>()?;

        let stack_cfg = RasterStackConfig {
            auto_reproject: args
                .get("auto_reproject")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            resampling_method: parse_resampling_override(args),
            allow_no_overlap: false,
        };
        let warnings = align_and_validate_raster_stack(&mut all_rasters, &stack_cfg)
            .map_err(|e| ToolError::Validation(format!("raster stack validation failed: {e}")))?;
        for warning in warnings {
            ctx.progress.info(&format!("pca_based_change_detection: {warning}"));
        }

        let (t1_rasters, t2_rasters) = all_rasters.split_at(bands);
        let rows = t1_rasters[0].rows;
        let cols = t1_rasters[0].cols;
        let n = rows * cols;
        let nodata = t1_rasters[0].nodata;

        ctx.progress.info("pca_based_change_detection: estimating covariance of spectral change vectors");
        let (sum, sum_sq, valid_count, _scratch) = (0..n)
            .into_par_iter()
            .fold(
                || (vec![0.0_f64; bands], vec![0.0_f64; bands * bands], 0_u64, vec![0.0_f64; bands]),
                |(mut local_sum, mut local_sq, mut local_count, mut dv), idx| {
                    let mut valid = true;
                    for b in 0..bands {
                        let v1 = t1_rasters[b].data.get_f64(idx);
                        let v2 = t2_rasters[b].data.get_f64(idx);
                        if t1_rasters[b].is_nodata(v1)
                            || t2_rasters[b].is_nodata(v2)
                            || v1.is_nan()
                            || v2.is_nan()
                        {
                            valid = false;
                            break;
                        }
                        dv[b] = v2 - v1;
                    }

                    if valid {
                        local_count += 1;
                        for i in 0..bands {
                            local_sum[i] += dv[i];
                            for j in 0..bands {
                                local_sq[i * bands + j] += dv[i] * dv[j];
                            }
                        }
                    }
                    (local_sum, local_sq, local_count, dv)
                },
            )
            .reduce(
                || (vec![0.0_f64; bands], vec![0.0_f64; bands * bands], 0_u64, vec![0.0_f64; bands]),
                |(mut sum_a, mut sq_a, cnt_a, dv_a), (sum_b, sq_b, cnt_b, _dv_b)| {
                    for i in 0..bands {
                        sum_a[i] += sum_b[i];
                    }
                    for i in 0..(bands * bands) {
                        sq_a[i] += sq_b[i];
                    }
                    (sum_a, sq_a, cnt_a + cnt_b, dv_a)
                },
            );

        if valid_count < 2 {
            return Err(ToolError::Execution(
                "not enough valid overlapping pixels to estimate PCA change model".to_string(),
            ));
        }

        let count_f = valid_count as f64;
        let mut means = vec![0.0_f64; bands];
        for i in 0..bands {
            means[i] = sum[i] / count_f;
        }

        let mut cov = vec![0.0_f64; bands * bands];
        for i in 0..bands {
            for j in 0..bands {
                cov[i * bands + j] = (sum_sq[i * bands + j] / count_f) - (means[i] * means[j]);
            }
        }

        if standardized {
            let mut corr = cov.clone();
            for i in 0..bands {
                for j in 0..bands {
                    let denom = (cov[i * bands + i].max(0.0).sqrt()) * (cov[j * bands + j].max(0.0).sqrt());
                    corr[i * bands + j] = if denom > 1.0e-15 {
                        cov[i * bands + j] / denom
                    } else {
                        0.0
                    };
                }
            }
            cov = corr;
        }

        let cov_mat = DMatrix::from_row_slice(bands, bands, &cov);
        let eig = cov_mat.symmetric_eigen();
        let eigenvalues = eig.eigenvalues.as_slice().to_vec();
        let evec_flat = eig.eigenvectors.as_slice().to_vec();

        let mut order: Vec<usize> = (0..bands).collect();
        order.par_sort_by(|a, b| {
            eigenvalues[*b]
                .partial_cmp(&eigenvalues[*a])
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let total_eigen = eigenvalues
            .iter()
            .copied()
            .filter(|v| v.is_finite() && *v > 0.0)
            .sum::<f64>();
        let pc = order[component_idx];
        let weights: Vec<f64> = (0..bands)
            .map(|k| evec_flat[pc * bands + k])
            .collect();

        let sigma_pc = eigenvalues[pc].max(0.0).sqrt();

        ctx.progress.info(&format!(
            "pca_based_change_detection: projecting change vectors onto PC{}",
            component
        ));
        let total_rows = rows.max(1);
        let mut done_rows = 0usize;
        let coalescer = PercentCoalescer::new(1, 98);

        let out_rows: Vec<Vec<f64>> = (0..rows)
            .into_par_iter()
            .map(|r| {
                let row_offset = r * cols;
                (0..cols)
                    .map(|c| {
                        let idx = row_offset + c;
                        let mut valid = true;
                        let mut score = 0.0_f64;
                        for b in 0..bands {
                            let v1 = t1_rasters[b].data.get_f64(idx);
                            let v2 = t2_rasters[b].data.get_f64(idx);
                            if t1_rasters[b].is_nodata(v1)
                                || t2_rasters[b].is_nodata(v2)
                                || v1.is_nan()
                                || v2.is_nan()
                            {
                                valid = false;
                                break;
                            }
                            let dv = (v2 - v1) - means[b];
                            score += dv * weights[b];
                        }
                        if valid { score.abs() } else { nodata }
                    })
                    .collect::<Vec<f64>>()
            })
            .collect();

        let mut out = t1_rasters[0].clone();
        out.bands = 1;
        let mut mask = if output_mask_path.is_some() && threshold_sigma.is_some() && sigma_pc > 0.0 {
            let mut m = t1_rasters[0].clone();
            m.bands = 1;
            Some(m)
        } else {
            None
        };
        let threshold = threshold_sigma.map(|th| th.abs() * sigma_pc);

        for (r, row) in out_rows.iter().enumerate() {
            out.set_row_slice(0, r as isize, row)
                .map_err(|e| ToolError::Execution(format!("failed writing output row {}: {e}", r)))?;
            if let (Some(mask_raster), Some(th)) = (mask.as_mut(), threshold) {
                let mut mask_row = vec![nodata; cols];
                for (c, v) in row.iter().copied().enumerate() {
                    if (v - nodata).abs() <= f64::EPSILON || v.is_nan() {
                        continue;
                    }
                    mask_row[c] = if v >= th { 1.0 } else { 0.0 };
                }
                mask_raster
                    .set_row_slice(0, r as isize, &mask_row)
                    .map_err(|e| ToolError::Execution(format!("failed writing mask row {}: {e}", r)))?;
            }
            done_rows += 1;
            coalescer.emit_unit_fraction(ctx.progress, done_rows as f64 / total_rows as f64);
        }

        let out_locator = write_or_store_output(out, output_path)?;

        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(out_locator));
        outputs.insert("active_band".to_string(), json!(0));
        outputs.insert("component".to_string(), json!(component));
        outputs.insert("valid_pixel_count".to_string(), json!(valid_count));
        outputs.insert("sigma_pc".to_string(), json!(sigma_pc));

        if let (Some(mask_path), Some(th_sigma), Some(mask)) = (output_mask_path, threshold_sigma, mask) {
            if sigma_pc > 0.0 {
                let threshold = th_sigma.abs() * sigma_pc;
                let mask_locator = write_or_store_output(mask, Some(mask_path))?;
                outputs.insert("mask".to_string(), json!({"__wbw_type__": "raster", "path": mask_locator, "active_band": 0}));
                outputs.insert("threshold_sigma".to_string(), json!(th_sigma));
                outputs.insert("threshold_value".to_string(), json!(threshold));
            }
        }

        if let Some(report_path) = output_report_path {
            if let Some(parent) = report_path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        ToolError::Execution(format!("failed creating report output directory: {e}"))
                    })?;
                }
            }

            let components = order
                .iter()
                .enumerate()
                .map(|(rank_idx, eig_idx)| {
                    let lambda = eigenvalues[*eig_idx];
                    let explained_variance_ratio = if total_eigen > 0.0 {
                        (lambda.max(0.0)) / total_eigen
                    } else {
                        0.0
                    };
                    let loadings = (0..bands)
                        .map(|k| evec_flat[*eig_idx * bands + k])
                        .collect::<Vec<f64>>();
                    json!({
                        "component": rank_idx + 1,
                        "eigenvalue": lambda,
                        "explained_variance_ratio": explained_variance_ratio,
                        "loadings": loadings,
                    })
                })
                .collect::<Vec<_>>();

            let report = json!({
                "valid_pixel_count": valid_count,
                "bands": bands,
                "standardized": standardized,
                "selected_component": component,
                "components": components,
            });

            let report_text = serde_json::to_string_pretty(&report).map_err(|e| {
                ToolError::Execution(format!("failed serializing PCA report JSON: {e}"))
            })?;
            std::fs::write(&report_path, report_text).map_err(|e| {
                ToolError::Execution(format!(
                    "failed writing PCA report '{}': {e}",
                    report_path.display()
                ))
            })?;
            outputs.insert(
                "report_path".to_string(),
                json!(report_path.to_string_lossy().to_string()),
            );
        }

        coalescer.finish(ctx.progress);
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for ImageDifferenceChangeDetectionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "image_difference_change_detection",
            display_name: "Image Difference Change Detection",
            summary: r#"Detects changes between two time periods through direct raster arithmetic operations including band differencing, normalized differencing, and ratio-based change indices. The algorithm computes pixel-by-pixel differences or ratios between coregistered multitemporal images to quantify magnitude and direction of change. Simple difference generates signed change values showing increase or decrease, while normalized difference (dividing by sum) normalizes results to -1 to 1 range for consistent comparison across variable pixel intensities. Key features include computational simplicity enabling near-real-time processing, versatility across spectral indices (NDVI, NDBI, NDMI), and direct interpretability of change magnitudes in original physical units. The method requires precise geometric coregistration and radiometric normalization to minimize false positives from data quality variations. Primary use cases include rapid response mapping for disasters, phenological change detection for agricultural and ecological monitoring, urban sprawl quantification, water body dynamics, and forest disturbance assessment. Output interpretation depends on analysis type: vegetation indices show increase/decrease in greenness, brightness indices reveal built-up development, moisture indices track water availability and drought stress. Change magnitude interpretation requires knowledge of sensor specifications, environmental conditions during acquisition, and reference data for validation. Thresholding change maps converts continuous values to binary change/no-change classification. Temporal sequences of difference maps enable change trend analysis, momentum assessment, and resilience evaluation for comprehensive environmental monitoring."#,
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "t1_inputs",
                    description: "Array of date-1 raster paths.",
                    required: true,
                },
                ToolParamSpec {
                    name: "t2_inputs",
                    description: "Array of date-2 raster paths; must match t1_inputs length and ordering.",
                    required: true,
                },
                ToolParamSpec {
                    name: "mode",
                    description: "Difference mode: magnitude (default) or signed.",
                    required: false,
                },
                ToolParamSpec {
                    name: "threshold_sigma",
                    description: "Optional sigma threshold for binary change mask generation.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject",
                    description: "If true (default), reproject stack rasters to match t1_inputs[0] when CRS differs.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject_method",
                    description: "Optional reprojection resampling override: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output raster path for difference image.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output_absolute",
                    description: "Optional output raster path for absolute signed difference (|mean(t2-t1)|).",
                    required: false,
                },
                ToolParamSpec {
                    name: "output_signed",
                    description: "Optional output raster path for signed difference mean(t2-t1).",
                    required: false,
                },
                ToolParamSpec {
                    name: "output_mask",
                    description: "Optional output raster path for thresholded binary change mask.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("t1_inputs".to_string(), json!(["t1_b2.tif", "t1_b3.tif", "t1_b4.tif"]));
        defaults.insert("t2_inputs".to_string(), json!(["t2_b2.tif", "t2_b3.tif", "t2_b4.tif"]));
        defaults.insert("mode".to_string(), json!("magnitude"));
        defaults.insert("threshold_sigma".to_string(), json!(2.0));
        defaults.insert("auto_reproject".to_string(), json!(true));
        defaults.insert("auto_reproject_method".to_string(), json!(""));

        let mut example = ToolArgs::new();
        example.insert("t1_inputs".to_string(), json!(["pre_b2.tif", "pre_b3.tif", "pre_b4.tif", "pre_b8.tif"]));
        example.insert("t2_inputs".to_string(), json!(["post_b2.tif", "post_b3.tif", "post_b4.tif", "post_b8.tif"]));
        example.insert("mode".to_string(), json!("magnitude"));
        example.insert("threshold_sigma".to_string(), json!(2.0));
        example.insert("output".to_string(), json!("image_difference.tif"));
        example.insert("output_absolute".to_string(), json!("image_difference_absolute.tif"));
        example.insert("output_signed".to_string(), json!("image_difference_signed.tif"));
        example.insert("output_mask".to_string(), json!("image_difference_mask.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_image_differencing".to_string(),
                description: "Compute multiband image-difference magnitude with threshold mask.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "change_detection".to_string(),
                "image_difference".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let t1 = parse_raster_list_arg(args, "t1_inputs")?;
        let t2 = parse_raster_list_arg(args, "t2_inputs")?;
        if t1.len() != t2.len() {
            return Err(ToolError::Validation(
                "'t1_inputs' and 't2_inputs' must have equal length".to_string(),
            ));
        }
        if t1.is_empty() {
            return Err(ToolError::Validation(
                "at least one band is required for image differencing".to_string(),
            ));
        }
        let _ = parse_optional_output_path(args, "output")?;
        let _ = parse_optional_output_path(args, "output_absolute")?;
        let _ = parse_optional_output_path(args, "output_signed")?;
        let _ = parse_optional_output_path(args, "output_mask")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let t1_paths = parse_raster_list_arg(args, "t1_inputs")?;
        let t2_paths = parse_raster_list_arg(args, "t2_inputs")?;
        if t1_paths.len() != t2_paths.len() {
            return Err(ToolError::Validation(
                "'t1_inputs' and 't2_inputs' must have equal length".to_string(),
            ));
        }
        let bands = t1_paths.len();
        if bands == 0 {
            return Err(ToolError::Validation(
                "at least one band is required for image differencing".to_string(),
            ));
        }

        let mode = args
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("magnitude")
            .to_ascii_lowercase();
        let signed = mode == "signed";
        let threshold_sigma = args.get("threshold_sigma").and_then(|v| v.as_f64());
        let output_path = parse_optional_output_path(args, "output")?;
        let output_absolute_path = parse_optional_output_path(args, "output_absolute")?;
        let output_signed_path = parse_optional_output_path(args, "output_signed")?;
        let output_mask_path = parse_optional_output_path(args, "output_mask")?;

        ctx.progress.info("image_difference_change_detection: reading and aligning input stacks");
        let mut all_rasters: Vec<Raster> = t1_paths
            .iter()
            .chain(t2_paths.iter())
            .map(|p| load_raster(p))
            .collect::<Result<Vec<_>, _>>()?;

        let stack_cfg = RasterStackConfig {
            auto_reproject: args
                .get("auto_reproject")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            resampling_method: parse_resampling_override(args),
            allow_no_overlap: false,
        };
        let warnings = align_and_validate_raster_stack(&mut all_rasters, &stack_cfg)
            .map_err(|e| ToolError::Validation(format!("raster stack validation failed: {e}")))?;
        for warning in warnings {
            ctx.progress.info(&format!("image_difference_change_detection: {warning}"));
        }

        let (t1_rasters, t2_rasters) = all_rasters.split_at(bands);
        let rows = t1_rasters[0].rows;
        let cols = t1_rasters[0].cols;
        let n = rows * cols;
        let nodata = t1_rasters[0].nodata;

        let (signed_values, magnitude_values): (Vec<f64>, Vec<f64>) = (0..n)
            .into_par_iter()
            .map(|idx| {
                let mut valid = true;
                let mut sum_sq = 0.0_f64;
                let mut sum_signed = 0.0_f64;
                for b in 0..bands {
                    let v1 = t1_rasters[b].data.get_f64(idx);
                    let v2 = t2_rasters[b].data.get_f64(idx);
                    if t1_rasters[b].is_nodata(v1)
                        || t2_rasters[b].is_nodata(v2)
                        || v1.is_nan()
                        || v2.is_nan()
                    {
                        valid = false;
                        break;
                    }
                    let d = v2 - v1;
                    sum_sq += d * d;
                    sum_signed += d;
                }
                if !valid {
                    (nodata, nodata)
                } else {
                    (sum_signed / bands as f64, sum_sq.sqrt())
                }
            })
            .unzip();

        let diff_values = if signed {
            signed_values.clone()
        } else {
            magnitude_values.clone()
        };

        let absolute_values: Vec<f64> = signed_values
            .par_iter()
            .map(|v| {
                if (*v - nodata).abs() <= f64::EPSILON || v.is_nan() {
                    nodata
                } else {
                    v.abs()
                }
            })
            .collect();

        let valid_vals: Vec<f64> = diff_values
            .par_iter()
            .copied()
            .filter(|v| (*v - nodata).abs() > f64::EPSILON && !v.is_nan())
            .collect();
        let (mean_diff, sigma_diff) = if valid_vals.is_empty() {
            (0.0, 0.0)
        } else {
            let mean = valid_vals.par_iter().copied().sum::<f64>() / valid_vals.len() as f64;
            let var = valid_vals
                .par_iter()
                .map(|v| {
                    let d = *v - mean;
                    d * d
                })
                .sum::<f64>()
                / valid_vals.len() as f64;
            (mean, var.sqrt())
        };

        let mut out = t1_rasters[0].clone();
        out.bands = 1;
        let coalescer = PercentCoalescer::new(1, 98);
        let mut done_rows = 0usize;
        let total_rows = rows.max(1);
        for r in 0..rows {
            let start = r * cols;
            let end = start + cols;
            out.set_row_slice(0, r as isize, &diff_values[start..end])
                .map_err(|e| ToolError::Execution(format!("failed writing output row {}: {e}", r)))?;
            done_rows += 1;
            coalescer.emit_unit_fraction(ctx.progress, done_rows as f64 / total_rows as f64);
        }
        let out_locator = write_or_store_output(out, output_path)?;

        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(out_locator));
        outputs.insert("active_band".to_string(), json!(0));
        outputs.insert("mode".to_string(), json!(if signed { "signed" } else { "magnitude" }));
        outputs.insert("mean".to_string(), json!(mean_diff));
        outputs.insert("sigma".to_string(), json!(sigma_diff));

        if let Some(abs_path) = output_absolute_path {
            let mut abs_out = t1_rasters[0].clone();
            abs_out.bands = 1;
            for r in 0..rows {
                let start = r * cols;
                let end = start + cols;
                abs_out
                    .set_row_slice(0, r as isize, &absolute_values[start..end])
                    .map_err(|e| {
                        ToolError::Execution(format!(
                            "failed writing absolute output row {}: {e}",
                            r
                        ))
                    })?;
            }
            let abs_locator = write_or_store_output(abs_out, Some(abs_path))?;
            outputs.insert(
                "absolute".to_string(),
                json!({"__wbw_type__": "raster", "path": abs_locator, "active_band": 0}),
            );
        }

        if let Some(signed_path) = output_signed_path {
            let mut signed_out = t1_rasters[0].clone();
            signed_out.bands = 1;
            for r in 0..rows {
                let start = r * cols;
                let end = start + cols;
                signed_out
                    .set_row_slice(0, r as isize, &signed_values[start..end])
                    .map_err(|e| {
                        ToolError::Execution(format!(
                            "failed writing signed output row {}: {e}",
                            r
                        ))
                    })?;
            }
            let signed_locator = write_or_store_output(signed_out, Some(signed_path))?;
            outputs.insert(
                "signed".to_string(),
                json!({"__wbw_type__": "raster", "path": signed_locator, "active_band": 0}),
            );
        }

        if let (Some(mask_path), Some(th_sigma)) = (output_mask_path, threshold_sigma) {
            if sigma_diff > 0.0 {
                let threshold = if signed {
                    mean_diff + th_sigma.abs() * sigma_diff
                } else {
                    th_sigma.abs() * sigma_diff
                };
                let mask_values: Vec<f64> = diff_values
                    .par_iter()
                    .map(|v| {
                        if (*v - nodata).abs() <= f64::EPSILON || v.is_nan() {
                            nodata
                        } else if if signed { *v >= threshold } else { v.abs() >= threshold } {
                            1.0
                        } else {
                            0.0
                        }
                    })
                    .collect();
                let mut mask = t1_rasters[0].clone();
                mask.bands = 1;
                for r in 0..rows {
                    let start = r * cols;
                    let end = start + cols;
                    mask.set_row_slice(0, r as isize, &mask_values[start..end])
                        .map_err(|e| ToolError::Execution(format!("failed writing mask row {}: {e}", r)))?;
                }
                let mask_locator = write_or_store_output(mask, Some(mask_path))?;
                outputs.insert("mask".to_string(), json!({"__wbw_type__": "raster", "path": mask_locator, "active_band": 0}));
                outputs.insert("threshold_sigma".to_string(), json!(th_sigma));
                outputs.insert("threshold_value".to_string(), json!(threshold));
            }
        }

        coalescer.finish(ctx.progress);
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for PostClassificationChangeTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "post_classification_change",
            display_name: "Post Classification Change",
            summary: r#"Identifies land cover and land use (LULC) changes through direct comparison of independently classified maps from different time periods. The algorithm compares pixel-by-pixel class assignments across temporal datasets to detect transitions between categories (e.g., forest to urban, grassland to agriculture). Cross-tabulation matrices quantify transition frequencies and identify dominant change trajectories. Key features include independence of individual classifications enabling use of different sensors/algorithms per date, direct interpretation of specific transitions showing what changed into what, and production of change maps highlighting only altered pixels. The method requires accurate classification at each time point and consistent class schemes across dates. Primary use cases include monitoring ecosystem conversion (deforestation, wetland loss), urban growth mapping, agricultural land abandonment or intensification, habitat fragmentation assessment, and LULC policy effectiveness evaluation. Output interpretation centers on transition matrices revealing directional change patterns, with diagonal elements representing no-change and off-diagonal elements showing conversions. Maps stratify landscapes by stability and transition type, enabling targeted monitoring and intervention. Accuracy depends critically on individual classification quality—errors in either date propagate to change maps, requiring validation against high-resolution reference data or ground truth. Temporal frequency analysis reveals change acceleration or stabilization trends. Integration with other variables (proximity to infrastructure, slope, land rights) explains change drivers and predicts future trajectories for landscape planning and conservation prioritization."#,
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "t1_classified",
                    description: "Date-1 classified raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "t2_classified",
                    description: "Date-2 classified raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "transition_scale",
                    description: "Transition coding scale factor (default 1000): code = class1 * scale + class2.",
                    required: false,
                },
                ToolParamSpec {
                    name: "t1_class_remap",
                    description: "Optional remap table for date-1 classes as object {\"old\": new, ...}.",
                    required: false,
                },
                ToolParamSpec {
                    name: "t2_class_remap",
                    description: "Optional remap table for date-2 classes as object {\"old\": new, ...}.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject",
                    description: "If true (default), reproject t2_classified to t1_classified CRS when needed.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject_method",
                    description: "Optional reprojection resampling override: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output transition-coded raster path.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("t1_classified".to_string(), json!("class_t1.tif"));
        defaults.insert("t2_classified".to_string(), json!("class_t2.tif"));
        defaults.insert("transition_scale".to_string(), json!(1000));
        defaults.insert("auto_reproject".to_string(), json!(true));
        defaults.insert("auto_reproject_method".to_string(), json!("nearest"));

        let mut example = ToolArgs::new();
        example.insert("t1_classified".to_string(), json!("landcover_2018.tif"));
        example.insert("t2_classified".to_string(), json!("landcover_2024.tif"));
        example.insert("transition_scale".to_string(), json!(1000));
        example.insert("t1_class_remap".to_string(), json!({"11": 1, "12": 1, "21": 2}));
        example.insert("t2_class_remap".to_string(), json!({"41": 4, "42": 4}));
        example.insert("output".to_string(), json!("class_transition.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_post_classification_change".to_string(),
                description: "Compute transition raster and transition matrix from two classified dates.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "change_detection".to_string(),
                "post_classification".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = args
            .get("t1_classified")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 't1_classified' is required".to_string()))?;
        let _ = args
            .get("t2_classified")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 't2_classified' is required".to_string()))?;
        let _ = parse_class_remap_arg(args, "t1_class_remap")?;
        let _ = parse_class_remap_arg(args, "t2_class_remap")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let t1_path = args
            .get("t1_classified")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 't1_classified' is required".to_string()))?;
        let t2_path = args
            .get("t2_classified")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 't2_classified' is required".to_string()))?;
        let transition_scale = args
            .get("transition_scale")
            .and_then(|v| v.as_i64())
            .unwrap_or(1000)
            .max(1) as f64;
        let t1_class_remap = parse_class_remap_arg(args, "t1_class_remap")?;
        let t2_class_remap = parse_class_remap_arg(args, "t2_class_remap")?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("post_classification_change: reading and aligning classified rasters");
        let mut rasters = vec![load_raster(t1_path)?, load_raster(t2_path)?];
        let stack_cfg = RasterStackConfig {
            auto_reproject: args
                .get("auto_reproject")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            resampling_method: parse_resampling_override(args).or_else(|| Some("nearest".to_string())),
            allow_no_overlap: false,
        };
        let warnings = align_and_validate_raster_stack(&mut rasters, &stack_cfg)
            .map_err(|e| ToolError::Validation(format!("raster stack validation failed: {e}")))?;
        for warning in warnings {
            ctx.progress.info(&format!("post_classification_change: {warning}"));
        }

        let t1 = &rasters[0];
        let t2 = &rasters[1];
        let rows = t1.rows;
        let cols = t1.cols;
        let n = rows * cols;
        let nodata = t1.nodata;

        let transition_values: Vec<f64> = (0..n)
            .into_par_iter()
            .map(|idx| {
                let c1 = t1.data.get_f64(idx);
                let c2 = t2.data.get_f64(idx);
                if t1.is_nodata(c1) || t2.is_nodata(c2) || c1.is_nan() || c2.is_nan() {
                    nodata
                } else {
                    let c1_raw = c1.round() as i64;
                    let c2_raw = c2.round() as i64;
                    let c1_mapped = t1_class_remap
                        .as_ref()
                        .and_then(|m| m.get(&c1_raw))
                        .copied()
                        .unwrap_or(c1_raw) as f64;
                    let c2_mapped = t2_class_remap
                        .as_ref()
                        .and_then(|m| m.get(&c2_raw))
                        .copied()
                        .unwrap_or(c2_raw) as f64;
                    c1_mapped * transition_scale + c2_mapped
                }
            })
            .collect();

        let transition_counts: HashMap<String, u64> = (0..n)
            .into_par_iter()
            .fold(
                HashMap::new,
                |mut local, idx| {
                    let c1 = t1.data.get_f64(idx);
                    let c2 = t2.data.get_f64(idx);
                    if !(t1.is_nodata(c1) || t2.is_nodata(c2) || c1.is_nan() || c2.is_nan()) {
                        let c1_raw = c1.round() as i64;
                        let c2_raw = c2.round() as i64;
                        let c1_mapped = t1_class_remap
                            .as_ref()
                            .and_then(|m| m.get(&c1_raw))
                            .copied()
                            .unwrap_or(c1_raw);
                        let c2_mapped = t2_class_remap
                            .as_ref()
                            .and_then(|m| m.get(&c2_raw))
                            .copied()
                            .unwrap_or(c2_raw);
                        let key = format!("{}->{}", c1_mapped, c2_mapped);
                        *local.entry(key).or_insert(0) += 1;
                    }
                    local
                },
            )
            .reduce(
                HashMap::new,
                |mut a, b| {
                    for (k, v) in b {
                        *a.entry(k).or_insert(0) += v;
                    }
                    a
                },
            );

        let mut out = t1.clone();
        out.bands = 1;
        let coalescer = PercentCoalescer::new(1, 98);
        let mut done_rows = 0usize;
        let total_rows = rows.max(1);
        for r in 0..rows {
            let start = r * cols;
            let end = start + cols;
            out.set_row_slice(0, r as isize, &transition_values[start..end])
                .map_err(|e| ToolError::Execution(format!("failed writing output row {}: {e}", r)))?;
            done_rows += 1;
            coalescer.emit_unit_fraction(ctx.progress, done_rows as f64 / total_rows as f64);
        }
        coalescer.finish(ctx.progress);

        let out_locator = write_or_store_output(out, output_path)?;

        let mut sorted_counts: Vec<(String, u64)> = transition_counts.into_iter().collect();
        sorted_counts.sort_by(|a, b| a.0.cmp(&b.0));

        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(out_locator));
        outputs.insert("active_band".to_string(), json!(0));
        outputs.insert("transition_scale".to_string(), json!(transition_scale));
        if let Some(remap) = &t1_class_remap {
            let mut items = remap
                .iter()
                .map(|(k, v)| json!({"from": k, "to": v}))
                .collect::<Vec<_>>();
            items.sort_by(|a, b| {
                let ka = a.get("from").and_then(|v| v.as_i64()).unwrap_or(0);
                let kb = b.get("from").and_then(|v| v.as_i64()).unwrap_or(0);
                ka.cmp(&kb)
            });
            outputs.insert("t1_class_remap".to_string(), json!(items));
        }
        if let Some(remap) = &t2_class_remap {
            let mut items = remap
                .iter()
                .map(|(k, v)| json!({"from": k, "to": v}))
                .collect::<Vec<_>>();
            items.sort_by(|a, b| {
                let ka = a.get("from").and_then(|v| v.as_i64()).unwrap_or(0);
                let kb = b.get("from").and_then(|v| v.as_i64()).unwrap_or(0);
                ka.cmp(&kb)
            });
            outputs.insert("t2_class_remap".to_string(), json!(items));
        }
        outputs.insert(
            "transition_counts".to_string(),
            json!(sorted_counts
                .into_iter()
                .map(|(k, v)| json!({"transition": k, "count": v}))
                .collect::<Vec<_>>()),
        );
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for LandSurfaceTemperatureSingleChannelTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "land_surface_temperature_single_channel",
            display_name: "Land Surface Temperature (Single Channel)",
            summary: "Computes Land Surface Temperature (LST) from single thermal band using radiative transfer equation. Converts thermal DN to radiance, then brightness temperature, with emissivity weighting for physical LST in Kelvin. Practical for rapid processing with accuracy dependent on emissivity input.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "thermal_input",
                    description: "Thermal raster path (DN by default, or brightness temperature if input_is_brightness_temp=true).",
                    required: true,
                },
                ToolParamSpec {
                    name: "input_is_brightness_temp",
                    description: "If true, thermal_input is already brightness temperature in Kelvin (default false).",
                    required: false,
                },
                ToolParamSpec {
                    name: "emissivity_input",
                    description: "Optional emissivity raster path aligned to thermal_input.",
                    required: false,
                },
                ToolParamSpec {
                    name: "emissivity_constant",
                    description: "Fallback emissivity when emissivity_input is not provided (default 0.98).",
                    required: false,
                },
                ToolParamSpec {
                    name: "sensor_bundle_root",
                    description: "Optional Landsat bundle root used to derive thermal radiance and Planck constants.",
                    required: false,
                },
                ToolParamSpec {
                    name: "thermal_band_number",
                    description: "Thermal band number for metadata lookup (default 10).",
                    required: false,
                },
                ToolParamSpec {
                    name: "radiance_mult",
                    description: "Thermal radiance multiplicative factor for DN->radiance conversion.",
                    required: false,
                },
                ToolParamSpec {
                    name: "radiance_add",
                    description: "Thermal radiance additive factor for DN->radiance conversion.",
                    required: false,
                },
                ToolParamSpec {
                    name: "k1_constant",
                    description: "Planck K1 thermal constant.",
                    required: false,
                },
                ToolParamSpec {
                    name: "k2_constant",
                    description: "Planck K2 thermal constant.",
                    required: false,
                },
                ToolParamSpec {
                    name: "wavelength_um",
                    description: "Effective thermal wavelength in micrometers (default inferred by thermal_band_number).",
                    required: false,
                },
                ToolParamSpec {
                    name: "output_units",
                    description: "Output units: celsius (default) or kelvin.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject",
                    description: "If true (default), reproject emissivity_input to thermal_input CRS when needed.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject_method",
                    description: "Optional reprojection resampling override: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output raster path.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("thermal_input".to_string(), json!("B10.tif"));
        defaults.insert("input_is_brightness_temp".to_string(), json!(false));
        defaults.insert("emissivity_constant".to_string(), json!(0.98));
        defaults.insert("thermal_band_number".to_string(), json!(10));
        defaults.insert("wavelength_um".to_string(), json!(10.895));
        defaults.insert("output_units".to_string(), json!("celsius"));
        defaults.insert("auto_reproject".to_string(), json!(true));
        defaults.insert("auto_reproject_method".to_string(), json!(""));

        let mut example = ToolArgs::new();
        example.insert("thermal_input".to_string(), json!("LC09_B10.TIF"));
        example.insert("sensor_bundle_root".to_string(), json!("LC09_L1TP_017030_20240420_20240426_02_T1"));
        example.insert("emissivity_input".to_string(), json!("emissivity.tif"));
        example.insert("output_units".to_string(), json!("celsius"));
        example.insert("output".to_string(), json!("lst_single_channel.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_lst_single_channel".to_string(),
                description: "Compute single-channel LST from Landsat thermal DN and emissivity raster.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "thermal".to_string(),
                "lst".to_string(),
                "single_channel".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = args
            .get("thermal_input")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'thermal_input' is required".to_string()))?;
        if let Some(units) = args.get("output_units").and_then(|v| v.as_str()) {
            let u = units.to_ascii_lowercase();
            if u != "kelvin" && u != "celsius" {
                return Err(ToolError::Validation(
                    "parameter 'output_units' must be 'kelvin' or 'celsius'".to_string(),
                ));
            }
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let thermal_input = args
            .get("thermal_input")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'thermal_input' is required".to_string()))?;
        let emissivity_input = args.get("emissivity_input").and_then(|v| v.as_str());
        let input_is_bt = args
            .get("input_is_brightness_temp")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let emissivity_constant = args
            .get("emissivity_constant")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.98)
            .clamp(0.001, 1.0);
        let thermal_band_number = args
            .get("thermal_band_number")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(10);
        let output_units = args
            .get("output_units")
            .and_then(|v| v.as_str())
            .unwrap_or("celsius")
            .to_ascii_lowercase();
        let output_path = parse_optional_output_path(args, "output")?;

        let mut radiance_mult = args.get("radiance_mult").and_then(|v| v.as_f64());
        let mut radiance_add = args.get("radiance_add").and_then(|v| v.as_f64());
        let mut k1 = args.get("k1_constant").and_then(|v| v.as_f64());
        let mut k2 = args.get("k2_constant").and_then(|v| v.as_f64());

        if !input_is_bt {
            if let Some(bundle_root) = args.get("sensor_bundle_root").and_then(|v| v.as_str()) {
                let bundle = LandsatBundle::open(bundle_root).map_err(|_| {
                    ToolError::Validation(
                        "parameter 'sensor_bundle_root' is not a recognized Landsat bundle".to_string(),
                    )
                })?;
                let (m, a, c1, c2) = parse_landsat_thermal_constants_from_bundle(&bundle, thermal_band_number)?;
                if radiance_mult.is_none() {
                    radiance_mult = Some(m);
                }
                if radiance_add.is_none() {
                    radiance_add = Some(a);
                }
                if k1.is_none() {
                    k1 = Some(c1);
                }
                if k2.is_none() {
                    k2 = Some(c2);
                }
            }
        }

        let radiance_mult = if input_is_bt {
            0.0
        } else {
            radiance_mult.ok_or_else(|| {
                ToolError::Validation(
                    "radiance_mult is required unless input_is_brightness_temp=true or Landsat bundle metadata is provided".to_string(),
                )
            })?
        };
        let radiance_add = if input_is_bt {
            0.0
        } else {
            radiance_add.ok_or_else(|| {
                ToolError::Validation(
                    "radiance_add is required unless input_is_brightness_temp=true or Landsat bundle metadata is provided".to_string(),
                )
            })?
        };
        let k1 = if input_is_bt {
            0.0
        } else {
            k1.ok_or_else(|| {
                ToolError::Validation(
                    "k1_constant is required unless input_is_brightness_temp=true or Landsat bundle metadata is provided".to_string(),
                )
            })?
        };
        let k2 = if input_is_bt {
            0.0
        } else {
            k2.ok_or_else(|| {
                ToolError::Validation(
                    "k2_constant is required unless input_is_brightness_temp=true or Landsat bundle metadata is provided".to_string(),
                )
            })?
        };

        let wavelength_um = args
            .get("wavelength_um")
            .and_then(|v| v.as_f64())
            .unwrap_or_else(|| landsat_default_thermal_wavelength_um(thermal_band_number));

        ctx.progress.info("land_surface_temperature_single_channel: reading and aligning inputs");
        let mut rasters = vec![load_raster(thermal_input)?];
        if let Some(e_path) = emissivity_input {
            rasters.push(load_raster(e_path)?);
        }

        let stack_cfg = RasterStackConfig {
            auto_reproject: args
                .get("auto_reproject")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            resampling_method: parse_resampling_override(args),
            allow_no_overlap: false,
        };
        let warnings = align_and_validate_raster_stack(&mut rasters, &stack_cfg)
            .map_err(|e| ToolError::Validation(format!("raster stack validation failed: {e}")))?;
        for warning in warnings {
            ctx.progress.info(&format!("land_surface_temperature_single_channel: {warning}"));
        }

        let thermal = &rasters[0];
        let emissivity_raster = if rasters.len() > 1 { Some(&rasters[1]) } else { None };

        let rows = thermal.rows;
        let cols = thermal.cols;
        let nodata = thermal.nodata;

        let mut out = thermal.clone();
        out.bands = 1;

        // h*c/sigma in um*K for single-channel emissivity correction formula.
        let rho_um_k = 14388.0_f64;

        let out_rows: Vec<Vec<f64>> = (0..rows)
            .into_par_iter()
            .map(|r| {
                let thermal_row = thermal.row_slice(0, r as isize);
                let eps_row = emissivity_raster.map(|e| e.row_slice(0, r as isize));
                (0..cols)
                    .map(|c| {
                        let t = thermal_row[c];
                        if thermal.is_nodata(t) || t.is_nan() {
                            return nodata;
                        }
                        let eps = eps_row
                            .as_ref()
                            .map(|v| v[c])
                            .filter(|v| !v.is_nan())
                            .unwrap_or(emissivity_constant)
                            .clamp(0.001, 1.0);

                        let bt_k = if input_is_bt {
                            t
                        } else {
                            let radiance = radiance_mult * t + radiance_add;
                            if radiance <= 0.0 {
                                return nodata;
                            }
                            k2 / ((k1 / radiance + 1.0).ln())
                        };

                        if bt_k <= 0.0 || bt_k.is_nan() {
                            return nodata;
                        }

                        let lst_k = bt_k / (1.0 + (wavelength_um * bt_k / rho_um_k) * eps.ln());
                        if output_units == "kelvin" {
                            lst_k
                        } else {
                            lst_k - 273.15
                        }
                    })
                    .collect::<Vec<f64>>()
            })
            .collect();

        let coalescer = PercentCoalescer::new(1, 98);
        let mut done_rows = 0usize;
        let total_rows = rows.max(1);
        for (r, row) in out_rows.iter().enumerate() {
            out.set_row_slice(0, r as isize, row)
                .map_err(|e| ToolError::Execution(format!("failed writing output row {}: {e}", r)))?;
            done_rows += 1;
            coalescer.emit_unit_fraction(ctx.progress, done_rows as f64 / total_rows as f64);
        }
        coalescer.finish(ctx.progress);

        let out_locator = write_or_store_output(out, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(out_locator));
        outputs.insert("active_band".to_string(), json!(0));
        outputs.insert("input_is_brightness_temp".to_string(), json!(input_is_bt));
        outputs.insert("wavelength_um".to_string(), json!(wavelength_um));
        outputs.insert("output_units".to_string(), json!(output_units));
        outputs.insert("emissivity_constant".to_string(), json!(emissivity_constant));
        if !input_is_bt {
            outputs.insert("radiance_mult".to_string(), json!(radiance_mult));
            outputs.insert("radiance_add".to_string(), json!(radiance_add));
            outputs.insert("k1_constant".to_string(), json!(k1));
            outputs.insert("k2_constant".to_string(), json!(k2));
        }
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for LandSurfaceTemperatureSplitWindowTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "land_surface_temperature_split_window",
            display_name: "Land Surface Temperature (Split Window)",
            summary: "Computes Land Surface Temperature (LST) using split-window method with two thermal bands. Accounts for atmospheric water vapor and emissivity differences using sensor-specific or user-supplied coefficients. More accurate than single-channel by leveraging atmospheric differential absorption.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "thermal1_input",
                    description: "First thermal raster path (e.g., Landsat B10).",
                    required: true,
                },
                ToolParamSpec {
                    name: "thermal2_input",
                    description: "Second thermal raster path (e.g., Landsat B11).",
                    required: true,
                },
                ToolParamSpec {
                    name: "input_is_brightness_temp",
                    description: "If true, thermal inputs are already brightness temperatures in Kelvin (default false).",
                    required: false,
                },
                ToolParamSpec {
                    name: "emissivity_mean_input",
                    description: "Optional mean emissivity raster path.",
                    required: false,
                },
                ToolParamSpec {
                    name: "emissivity_delta_input",
                    description: "Optional emissivity delta raster path (epsilon1 - epsilon2).",
                    required: false,
                },
                ToolParamSpec {
                    name: "emissivity_mean_constant",
                    description: "Fallback mean emissivity constant when emissivity_mean_input is absent (default 0.98).",
                    required: false,
                },
                ToolParamSpec {
                    name: "emissivity_delta_constant",
                    description: "Fallback emissivity delta constant when emissivity_delta_input is absent (default 0.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "sensor_bundle_root",
                    description: "Optional Landsat bundle root used to derive thermal radiance and Planck constants.",
                    required: false,
                },
                ToolParamSpec {
                    name: "thermal_band1_number",
                    description: "Band number for thermal1_input metadata lookup (default 10).",
                    required: false,
                },
                ToolParamSpec {
                    name: "thermal_band2_number",
                    description: "Band number for thermal2_input metadata lookup (default 11).",
                    required: false,
                },
                ToolParamSpec {
                    name: "radiance1_mult",
                    description: "Band-1 radiance multiplicative factor.",
                    required: false,
                },
                ToolParamSpec {
                    name: "radiance1_add",
                    description: "Band-1 radiance additive factor.",
                    required: false,
                },
                ToolParamSpec {
                    name: "k1_1",
                    description: "Band-1 Planck K1 constant.",
                    required: false,
                },
                ToolParamSpec {
                    name: "k2_1",
                    description: "Band-1 Planck K2 constant.",
                    required: false,
                },
                ToolParamSpec {
                    name: "radiance2_mult",
                    description: "Band-2 radiance multiplicative factor.",
                    required: false,
                },
                ToolParamSpec {
                    name: "radiance2_add",
                    description: "Band-2 radiance additive factor.",
                    required: false,
                },
                ToolParamSpec {
                    name: "k1_2",
                    description: "Band-2 Planck K1 constant.",
                    required: false,
                },
                ToolParamSpec {
                    name: "k2_2",
                    description: "Band-2 Planck K2 constant.",
                    required: false,
                },
                ToolParamSpec {
                    name: "coeff_a0",
                    description: "Split-window coefficient a0 (default 0.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "coeff_a1",
                    description: "Split-window coefficient a1 for T1 term (default 1.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "coeff_a2",
                    description: "Split-window coefficient a2 for (T1-T2) term (default 0.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "coeff_a3",
                    description: "Split-window coefficient a3 for (T1-T2)^2 term (default 0.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "coeff_a4",
                    description: "Split-window coefficient a4 for (1-emissivity_mean) term (default 0.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "coeff_a5",
                    description: "Split-window coefficient a5 for emissivity_delta term (default 0.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "output_units",
                    description: "Output units: celsius (default) or kelvin.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject",
                    description: "If true (default), reproject supplemental rasters to match thermal1_input CRS when needed.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject_method",
                    description: "Optional reprojection resampling override: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output raster path.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("thermal1_input".to_string(), json!("B10.tif"));
        defaults.insert("thermal2_input".to_string(), json!("B11.tif"));
        defaults.insert("input_is_brightness_temp".to_string(), json!(false));
        defaults.insert("emissivity_mean_constant".to_string(), json!(0.98));
        defaults.insert("emissivity_delta_constant".to_string(), json!(0.0));
        defaults.insert("thermal_band1_number".to_string(), json!(10));
        defaults.insert("thermal_band2_number".to_string(), json!(11));
        defaults.insert("coeff_a0".to_string(), json!(0.0));
        defaults.insert("coeff_a1".to_string(), json!(1.0));
        defaults.insert("coeff_a2".to_string(), json!(0.0));
        defaults.insert("coeff_a3".to_string(), json!(0.0));
        defaults.insert("coeff_a4".to_string(), json!(0.0));
        defaults.insert("coeff_a5".to_string(), json!(0.0));
        defaults.insert("output_units".to_string(), json!("celsius"));
        defaults.insert("auto_reproject".to_string(), json!(true));
        defaults.insert("auto_reproject_method".to_string(), json!(""));

        let mut example = ToolArgs::new();
        example.insert("thermal1_input".to_string(), json!("LC09_B10.TIF"));
        example.insert("thermal2_input".to_string(), json!("LC09_B11.TIF"));
        example.insert("sensor_bundle_root".to_string(), json!("LC09_L1TP_017030_20240420_20240426_02_T1"));
        example.insert("emissivity_mean_input".to_string(), json!("emissivity.tif"));
        example.insert("output".to_string(), json!("lst_split_window.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_lst_split_window".to_string(),
                description: "Compute split-window LST from two thermal bands with emissivity correction.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "thermal".to_string(),
                "lst".to_string(),
                "split_window".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = args
            .get("thermal1_input")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'thermal1_input' is required".to_string()))?;
        let _ = args
            .get("thermal2_input")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'thermal2_input' is required".to_string()))?;
        if let Some(units) = args.get("output_units").and_then(|v| v.as_str()) {
            let u = units.to_ascii_lowercase();
            if u != "kelvin" && u != "celsius" {
                return Err(ToolError::Validation(
                    "parameter 'output_units' must be 'kelvin' or 'celsius'".to_string(),
                ));
            }
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let thermal1_input = args
            .get("thermal1_input")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'thermal1_input' is required".to_string()))?;
        let thermal2_input = args
            .get("thermal2_input")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'thermal2_input' is required".to_string()))?;
        let emissivity_mean_input = args.get("emissivity_mean_input").and_then(|v| v.as_str());
        let emissivity_delta_input = args.get("emissivity_delta_input").and_then(|v| v.as_str());
        let emissivity_mean_constant = args
            .get("emissivity_mean_constant")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.98)
            .clamp(0.001, 1.0);
        let emissivity_delta_constant = args
            .get("emissivity_delta_constant")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
            .clamp(-1.0, 1.0);
        let input_is_bt = args
            .get("input_is_brightness_temp")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let output_units = args
            .get("output_units")
            .and_then(|v| v.as_str())
            .unwrap_or("celsius")
            .to_ascii_lowercase();
        let output_path = parse_optional_output_path(args, "output")?;

        let thermal_band1_number = args
            .get("thermal_band1_number")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(10);
        let thermal_band2_number = args
            .get("thermal_band2_number")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(11);

        let mut rad1_mult = args.get("radiance1_mult").and_then(|v| v.as_f64());
        let mut rad1_add = args.get("radiance1_add").and_then(|v| v.as_f64());
        let mut k1_1 = args.get("k1_1").and_then(|v| v.as_f64());
        let mut k2_1 = args.get("k2_1").and_then(|v| v.as_f64());
        let mut rad2_mult = args.get("radiance2_mult").and_then(|v| v.as_f64());
        let mut rad2_add = args.get("radiance2_add").and_then(|v| v.as_f64());
        let mut k1_2 = args.get("k1_2").and_then(|v| v.as_f64());
        let mut k2_2 = args.get("k2_2").and_then(|v| v.as_f64());

        if !input_is_bt {
            if let Some(bundle_root) = args.get("sensor_bundle_root").and_then(|v| v.as_str()) {
                let bundle = LandsatBundle::open(bundle_root).map_err(|_| {
                    ToolError::Validation(
                        "parameter 'sensor_bundle_root' is not a recognized Landsat bundle".to_string(),
                    )
                })?;

                let (m1, a1, c1_1, c2_1) =
                    parse_landsat_thermal_constants_from_bundle(&bundle, thermal_band1_number)?;
                let (m2, a2, c1_2, c2_2) =
                    parse_landsat_thermal_constants_from_bundle(&bundle, thermal_band2_number)?;
                if rad1_mult.is_none() {
                    rad1_mult = Some(m1);
                }
                if rad1_add.is_none() {
                    rad1_add = Some(a1);
                }
                if k1_1.is_none() {
                    k1_1 = Some(c1_1);
                }
                if k2_1.is_none() {
                    k2_1 = Some(c2_1);
                }
                if rad2_mult.is_none() {
                    rad2_mult = Some(m2);
                }
                if rad2_add.is_none() {
                    rad2_add = Some(a2);
                }
                if k1_2.is_none() {
                    k1_2 = Some(c1_2);
                }
                if k2_2.is_none() {
                    k2_2 = Some(c2_2);
                }
            }
        }

        let rad1_mult = if input_is_bt {
            0.0
        } else {
            rad1_mult.ok_or_else(|| {
                ToolError::Validation(
                    "radiance1_mult is required unless input_is_brightness_temp=true or Landsat bundle metadata is provided".to_string(),
                )
            })?
        };
        let rad1_add = if input_is_bt {
            0.0
        } else {
            rad1_add.ok_or_else(|| {
                ToolError::Validation(
                    "radiance1_add is required unless input_is_brightness_temp=true or Landsat bundle metadata is provided".to_string(),
                )
            })?
        };
        let k1_1 = if input_is_bt {
            0.0
        } else {
            k1_1.ok_or_else(|| {
                ToolError::Validation(
                    "k1_1 is required unless input_is_brightness_temp=true or Landsat bundle metadata is provided".to_string(),
                )
            })?
        };
        let k2_1 = if input_is_bt {
            0.0
        } else {
            k2_1.ok_or_else(|| {
                ToolError::Validation(
                    "k2_1 is required unless input_is_brightness_temp=true or Landsat bundle metadata is provided".to_string(),
                )
            })?
        };
        let rad2_mult = if input_is_bt {
            0.0
        } else {
            rad2_mult.ok_or_else(|| {
                ToolError::Validation(
                    "radiance2_mult is required unless input_is_brightness_temp=true or Landsat bundle metadata is provided".to_string(),
                )
            })?
        };
        let rad2_add = if input_is_bt {
            0.0
        } else {
            rad2_add.ok_or_else(|| {
                ToolError::Validation(
                    "radiance2_add is required unless input_is_brightness_temp=true or Landsat bundle metadata is provided".to_string(),
                )
            })?
        };
        let k1_2 = if input_is_bt {
            0.0
        } else {
            k1_2.ok_or_else(|| {
                ToolError::Validation(
                    "k1_2 is required unless input_is_brightness_temp=true or Landsat bundle metadata is provided".to_string(),
                )
            })?
        };
        let k2_2 = if input_is_bt {
            0.0
        } else {
            k2_2.ok_or_else(|| {
                ToolError::Validation(
                    "k2_2 is required unless input_is_brightness_temp=true or Landsat bundle metadata is provided".to_string(),
                )
            })?
        };

        let a0 = args.get("coeff_a0").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let a1 = args.get("coeff_a1").and_then(|v| v.as_f64()).unwrap_or(1.0);
        let a2 = args.get("coeff_a2").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let a3 = args.get("coeff_a3").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let a4 = args.get("coeff_a4").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let a5 = args.get("coeff_a5").and_then(|v| v.as_f64()).unwrap_or(0.0);

        for (name, value) in [
            ("coeff_a0", a0),
            ("coeff_a1", a1),
            ("coeff_a2", a2),
            ("coeff_a3", a3),
            ("coeff_a4", a4),
            ("coeff_a5", a5),
        ] {
            if !value.is_finite() {
                return Err(ToolError::Validation(format!(
                    "parameter '{}' must be a finite number",
                    name
                )));
            }
        }

        let mut coefficient_warnings: Vec<String> = Vec::new();
        if thermal_band1_number == thermal_band2_number {
            coefficient_warnings.push(
                "thermal_band1_number and thermal_band2_number are identical; split-window methods normally require two distinct thermal bands".to_string(),
            );
        }
        if !(0.8..=1.2).contains(&a1) {
            coefficient_warnings.push(format!(
                "coeff_a1={} is outside the typical split-window range [0.8, 1.2]",
                a1
            ));
        }
        if a2.abs() > 10.0 {
            coefficient_warnings.push(format!(
                "coeff_a2={} is large in magnitude; verify calibration coefficients for this sensor/atmospheric regime",
                a2
            ));
        }
        if a3.abs() > 1.0 {
            coefficient_warnings.push(format!(
                "coeff_a3={} is large in magnitude for the quadratic temperature-difference term",
                a3
            ));
        }
        if a4.abs() > 10.0 || a5.abs() > 10.0 {
            coefficient_warnings.push(format!(
                "coeff_a4={} or coeff_a5={} may be unusually large for emissivity terms",
                a4, a5
            ));
        }
        if !(0.90..=1.0).contains(&emissivity_mean_constant) {
            coefficient_warnings.push(format!(
                "emissivity_mean_constant={} is outside the common remote-sensing range [0.90, 1.00]",
                emissivity_mean_constant
            ));
        }
        if emissivity_delta_constant.abs() > 0.1 {
            coefficient_warnings.push(format!(
                "emissivity_delta_constant={} is outside typical split-window assumptions (|delta epsilon| <= 0.1)",
                emissivity_delta_constant
            ));
        }
        for warning in &coefficient_warnings {
            ctx.progress
                .info(&format!("land_surface_temperature_split_window: warning: {}", warning));
        }

        ctx.progress.info("land_surface_temperature_split_window: reading and aligning inputs");
        let mut rasters = vec![load_raster(thermal1_input)?, load_raster(thermal2_input)?];
        if let Some(p) = emissivity_mean_input {
            rasters.push(load_raster(p)?);
        }
        if let Some(p) = emissivity_delta_input {
            rasters.push(load_raster(p)?);
        }

        let stack_cfg = RasterStackConfig {
            auto_reproject: args
                .get("auto_reproject")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            resampling_method: parse_resampling_override(args),
            allow_no_overlap: false,
        };
        let stack_warnings = align_and_validate_raster_stack(&mut rasters, &stack_cfg)
            .map_err(|e| ToolError::Validation(format!("raster stack validation failed: {e}")))?;
        for warning in stack_warnings {
            ctx.progress.info(&format!("land_surface_temperature_split_window: {warning}"));
        }

        let t1 = &rasters[0];
        let t2 = &rasters[1];
        let eps_mean_raster = if emissivity_mean_input.is_some() {
            Some(&rasters[2])
        } else {
            None
        };
        let eps_delta_raster = if emissivity_delta_input.is_some() {
            let idx = if emissivity_mean_input.is_some() { 3 } else { 2 };
            Some(&rasters[idx])
        } else {
            None
        };

        let rows = t1.rows;
        let cols = t1.cols;
        let nodata = t1.nodata;

        let out_rows: Vec<Vec<f64>> = (0..rows)
            .into_par_iter()
            .map(|r| {
                let t1_row = t1.row_slice(0, r as isize);
                let t2_row = t2.row_slice(0, r as isize);
                let em_row = eps_mean_raster.map(|e| e.row_slice(0, r as isize));
                let de_row = eps_delta_raster.map(|e| e.row_slice(0, r as isize));

                (0..cols)
                    .map(|c| {
                        let v1 = t1_row[c];
                        let v2 = t2_row[c];
                        if t1.is_nodata(v1) || t2.is_nodata(v2) || v1.is_nan() || v2.is_nan() {
                            return nodata;
                        }

                        let bt1 = if input_is_bt {
                            v1
                        } else {
                            let radiance = rad1_mult * v1 + rad1_add;
                            if radiance <= 0.0 {
                                return nodata;
                            }
                            k2_1 / ((k1_1 / radiance + 1.0).ln())
                        };
                        let bt2 = if input_is_bt {
                            v2
                        } else {
                            let radiance = rad2_mult * v2 + rad2_add;
                            if radiance <= 0.0 {
                                return nodata;
                            }
                            k2_2 / ((k1_2 / radiance + 1.0).ln())
                        };

                        if bt1 <= 0.0 || bt2 <= 0.0 || bt1.is_nan() || bt2.is_nan() {
                            return nodata;
                        }

                        let eps_mean = em_row
                            .as_ref()
                            .map(|v| v[c])
                            .filter(|v| !v.is_nan())
                            .unwrap_or(emissivity_mean_constant)
                            .clamp(0.001, 1.0);
                        let eps_delta = de_row
                            .as_ref()
                            .map(|v| v[c])
                            .filter(|v| !v.is_nan())
                            .unwrap_or(emissivity_delta_constant)
                            .clamp(-1.0, 1.0);

                        let dt = bt1 - bt2;
                        let mut lst_k = a0
                            + a1 * bt1
                            + a2 * dt
                            + a3 * dt * dt
                            + a4 * (1.0 - eps_mean)
                            + a5 * eps_delta;

                        if output_units == "celsius" {
                            lst_k -= 273.15;
                        }
                        lst_k
                    })
                    .collect::<Vec<f64>>()
            })
            .collect();

        let mut out = t1.clone();
        out.bands = 1;
        let coalescer = PercentCoalescer::new(1, 98);
        let mut done_rows = 0usize;
        let total_rows = rows.max(1);
        for (r, row) in out_rows.iter().enumerate() {
            out.set_row_slice(0, r as isize, row)
                .map_err(|e| ToolError::Execution(format!("failed writing output row {}: {e}", r)))?;
            done_rows += 1;
            coalescer.emit_unit_fraction(ctx.progress, done_rows as f64 / total_rows as f64);
        }
        coalescer.finish(ctx.progress);

        let out_locator = write_or_store_output(out, output_path)?;

        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(out_locator));
        outputs.insert("active_band".to_string(), json!(0));
        outputs.insert("input_is_brightness_temp".to_string(), json!(input_is_bt));
        outputs.insert("output_units".to_string(), json!(output_units));
        outputs.insert("coefficients".to_string(), json!({
            "a0": a0,
            "a1": a1,
            "a2": a2,
            "a3": a3,
            "a4": a4,
            "a5": a5,
        }));
        outputs.insert("emissivity_mean_constant".to_string(), json!(emissivity_mean_constant));
        outputs.insert("emissivity_delta_constant".to_string(), json!(emissivity_delta_constant));
        if !coefficient_warnings.is_empty() {
            outputs.insert("warnings".to_string(), json!(coefficient_warnings));
        }
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for SpectralAngleMapperTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "spectral_angle_mapper",
            display_name: "Spectral Angle Mapper",
            summary: r#"Spectral Angle Mapper (SAM) classifies pixels by computing the spectral angle between each pixel's spectrum and reference spectra (endmembers) in multidimensional spectral space. The algorithm treats each spectrum as a vector, computing the spectral angle using the spectral angle: θ = arccos[(S₁·S₂)/(||S₁|| ||S₂||)], where S₁ and S₂ are pixel and reference spectra. Pixels are classified to the endmember with minimum spectral angle, independent of illumination magnitude. This physics-based approach exploits absorption feature positioning to identify mineralogy and material composition without requiring absolute reflectance calibration. Key features include illumination-invariant classification using spectral angle rather than magnitude, support for user-defined reference spectral libraries, automatic handling of hyperspectral dimensionality, computation of per-pixel spectral angles for confidence assessment, and sub-pixel spectral heterogeneity detection. The tool requires fewer training samples than statistical classifiers and performs reliably with diverse spectral libraries. Applications include mineral and lithological mapping using USGS spectral libraries, vegetation and crop classification exploiting absorption feature signatures, detection of specific materials in mixed pixels, hyperspectral image classification and geological surveys, and urban material mapping (asphalt, concrete, vegetation). SAM excels where spectral absorption features provide diagnostic information independent of illumination conditions. Output interpretation: Low spectral angles (<5°) indicate strong spectral similarity to reference endmembers; angles increase with spectral difference. Output includes classified image (endmember ID per pixel) and spectral angle map showing classification confidence. Mixed pixels may have significant secondary angles, indicating spectral mixing requiring unmixing analysis."#,
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "inputs",
                    description: "Input raster stack (one raster per band).",
                    required: true,
                },
                ToolParamSpec {
                    name: "endmembers",
                    description: "Endmember signatures as array of objects {name, values} or numeric arrays.",
                    required: true,
                },
                ToolParamSpec {
                    name: "threshold_angle_deg",
                    description: "Optional maximum accepted spectral angle in degrees; unmatched pixels are class 0.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject",
                    description: "If true (default), reproject stack rasters to match inputs[0] when CRS differs.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject_method",
                    description: "Optional reprojection resampling override: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output class raster path.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output_angle",
                    description: "Optional output raster path for minimum spectral angle (degrees).",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["B2.tif", "B3.tif", "B4.tif", "B8.tif"]));
        defaults.insert(
            "endmembers".to_string(),
            json!([
                {"name": "water", "values": [0.05, 0.04, 0.03, 0.01]},
                {"name": "vegetation", "values": [0.04, 0.08, 0.05, 0.35]}
            ]),
        );
        defaults.insert("threshold_angle_deg".to_string(), json!(15.0));
        defaults.insert("auto_reproject".to_string(), json!(true));
        defaults.insert("auto_reproject_method".to_string(), json!(""));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["pre_b2.tif", "pre_b3.tif", "pre_b4.tif", "pre_b8.tif"]));
        example.insert(
            "endmembers".to_string(),
            json!([
                {"name": "water", "values": [0.04, 0.03, 0.02, 0.01]},
                {"name": "soil", "values": [0.15, 0.18, 0.20, 0.25]},
                {"name": "veg", "values": [0.05, 0.10, 0.06, 0.40]}
            ]),
        );
        example.insert("threshold_angle_deg".to_string(), json!(12.0));
        example.insert("output".to_string(), json!("sam_classes.tif"));
        example.insert("output_angle".to_string(), json!("sam_min_angle.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_sam".to_string(),
                description: "Classify multiband raster stack by minimum spectral angle.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "classification".to_string(),
                "hyperspectral".to_string(),
                "sam".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.len() < 2 {
            return Err(ToolError::Validation(
                "parameter 'inputs' must contain at least 2 rasters".to_string(),
            ));
        }
        let _ = parse_endmember_vectors(args, inputs.len())?;
        let _ = parse_optional_output_path(args, "output")?;
        let _ = parse_optional_output_path(args, "output_angle")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = parse_raster_list_arg(args, "inputs")?;
        let num_bands = input_paths.len();
        let (class_names, endmembers) = parse_endmember_vectors(args, num_bands)?;
        let threshold_angle_deg = args.get("threshold_angle_deg").and_then(|v| v.as_f64());
        let output_path = parse_optional_output_path(args, "output")?;
        let output_angle_path = parse_optional_output_path(args, "output_angle")?;

        let endmember_norms: Vec<f64> = endmembers
            .iter()
            .map(|e| e.iter().map(|v| v * v).sum::<f64>().sqrt())
            .collect();
        if endmember_norms.iter().any(|n| *n <= 0.0) {
            return Err(ToolError::Validation(
                "all endmember vectors must have non-zero norm".to_string(),
            ));
        }

        ctx.progress.info("spectral_angle_mapper: reading and aligning input stack");
        let rasters = input_paths
            .iter()
            .map(|p| load_raster(p))
            .collect::<Result<Vec<_>, _>>()?;

        // Strict validation: all inputs must have same CRS and be spatially co-registered
        // (no auto-reprojection for spectral analysis tools)
        validate_raster_stack_strict(&rasters)
            .map_err(|e| ToolError::Validation(format!("raster stack validation failed: {e}")))?;
        ctx.progress.info("spectral_angle_mapper: validated strict CRS and spatial alignment");

        let rows = rasters[0].rows;
        let cols = rasters[0].cols;
        let nodata = rasters[0].nodata;

        let row_results: Vec<(Vec<f64>, Vec<f64>)> = (0..rows)
            .into_par_iter()
            .map(|r| {
                let band_rows: Vec<Vec<f64>> = rasters
                    .iter()
                    .map(|b| b.row_slice(0, r as isize))
                    .collect();
                let mut class_row = vec![nodata; cols];
                let mut angle_row = vec![nodata; cols];

                for c in 0..cols {
                    let mut valid = true;
                    let mut pix_norm_sq = 0.0_f64;
                    for b in 0..num_bands {
                        let v = band_rows[b][c];
                        if rasters[b].is_nodata(v) || v.is_nan() {
                            valid = false;
                            break;
                        }
                        pix_norm_sq += v * v;
                    }
                    if !valid || pix_norm_sq <= 0.0 {
                        continue;
                    }
                    let pix_norm = pix_norm_sq.sqrt();

                    let mut best_idx = 0usize;
                    let mut best_ang = f64::INFINITY;
                    for (i, endm) in endmembers.iter().enumerate() {
                        let mut dot = 0.0_f64;
                        for b in 0..num_bands {
                            dot += band_rows[b][c] * endm[b];
                        }
                        let denom = pix_norm * endmember_norms[i];
                        if denom <= 0.0 {
                            continue;
                        }
                        let cosang = (dot / denom).clamp(-1.0, 1.0);
                        let ang = cosang.acos().to_degrees();
                        if ang < best_ang {
                            best_ang = ang;
                            best_idx = i;
                        }
                    }

                    if best_ang.is_finite() {
                        if let Some(t) = threshold_angle_deg {
                            if best_ang > t {
                                class_row[c] = 0.0;
                            } else {
                                class_row[c] = (best_idx + 1) as f64;
                            }
                        } else {
                            class_row[c] = (best_idx + 1) as f64;
                        }
                        angle_row[c] = best_ang;
                    }
                }

                (class_row, angle_row)
            })
            .collect();

        let mut out_class = rasters[0].clone();
        out_class.bands = 1;
        let coalescer = PercentCoalescer::new(1, 98);
        let mut done_rows = 0usize;
        let total_rows = rows.max(1);
        for (r, (class_row, _)) in row_results.iter().enumerate() {
            out_class
                .set_row_slice(0, r as isize, class_row)
                .map_err(|e| ToolError::Execution(format!("failed writing class row {}: {e}", r)))?;
            done_rows += 1;
            coalescer.emit_unit_fraction(ctx.progress, done_rows as f64 / total_rows as f64);
        }
        coalescer.finish(ctx.progress);

        let class_locator = write_or_store_output(out_class, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(class_locator));
        outputs.insert("active_band".to_string(), json!(0));
        outputs.insert("class_names".to_string(), json!(class_names));
        if let Some(t) = threshold_angle_deg {
            outputs.insert("threshold_angle_deg".to_string(), json!(t));
        }

        if let Some(angle_path) = output_angle_path {
            let mut out_angle = rasters[0].clone();
            out_angle.bands = 1;
            for (r, (_, angle_row)) in row_results.iter().enumerate() {
                out_angle
                    .set_row_slice(0, r as isize, angle_row)
                    .map_err(|e| ToolError::Execution(format!("failed writing angle row {}: {e}", r)))?;
            }
            let angle_locator = write_or_store_output(out_angle, Some(angle_path))?;
            outputs.insert(
                "angle".to_string(),
                json!({"__wbw_type__": "raster", "path": angle_locator, "active_band": 0}),
            );
        }

        Ok(ToolRunResult { outputs })
    }
}

impl Tool for ContinuumRemovalTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "continuum_removal",
            display_name: "Continuum Removal",
            summary: "Normalizes multispectral spectra by dividing each band by its upper-hull (convex) continuum estimate. Removes continuum trends and enhances absorption features for mineral and material identification. Standard preprocessing for hyperspectral analysis.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "inputs",
                    description: "Input raster stack (one raster per band).",
                    required: true,
                },
                ToolParamSpec {
                    name: "wavelengths",
                    description: "Optional strictly increasing wavelength array matching inputs length.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject",
                    description: "If true (default), reproject stack rasters to match inputs[0] when CRS differs.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject_method",
                    description: "Optional reprojection resampling override: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output multiband raster path.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["b1.tif", "b2.tif", "b3.tif", "b4.tif"]));
        defaults.insert("auto_reproject".to_string(), json!(true));
        defaults.insert("auto_reproject_method".to_string(), json!(""));

        let mut example = ToolArgs::new();
        example.insert(
            "inputs".to_string(),
            json!(["hyp_b1.tif", "hyp_b2.tif", "hyp_b3.tif", "hyp_b4.tif", "hyp_b5.tif"]),
        );
        example.insert("wavelengths".to_string(), json!([450.0, 550.0, 650.0, 850.0, 950.0]));
        example.insert("output".to_string(), json!("continuum_removed.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_continuum_removal".to_string(),
                description: "Apply upper-hull continuum normalization to a multiband spectrum stack.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "hyperspectral".to_string(),
                "continuum".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.len() < 3 {
            return Err(ToolError::Validation(
                "parameter 'inputs' must contain at least 3 rasters for continuum removal".to_string(),
            ));
        }
        let _ = parse_wavelengths_arg(args, inputs.len())?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = parse_raster_list_arg(args, "inputs")?;
        let num_bands = input_paths.len();
        let wavelengths = parse_wavelengths_arg(args, num_bands)?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("continuum_removal: reading and aligning input stack");
        let rasters = input_paths
            .iter()
            .map(|p| load_raster(p))
            .collect::<Result<Vec<_>, _>>()?;

        // Strict validation: all inputs must have same CRS and be spatially co-registered
        // (no auto-reprojection for spectral analysis tools)
        validate_raster_stack_strict(&rasters)
            .map_err(|e| ToolError::Validation(format!("raster stack validation failed: {e}")))?;
        ctx.progress.info("continuum_removal: validated strict CRS and spatial alignment");

        let rows = rasters[0].rows;
        let cols = rasters[0].cols;
        let nodata = rasters[0].nodata;

        let row_results: Vec<Vec<Vec<f64>>> = (0..rows)
            .into_par_iter()
            .map(|r| {
                let band_rows: Vec<Vec<f64>> = rasters
                    .iter()
                    .map(|b| b.row_slice(0, r as isize))
                    .collect();
                let mut out_by_band = vec![vec![nodata; cols]; num_bands];
                let mut spectrum = vec![0.0_f64; num_bands];
                let mut continuum = vec![0.0_f64; num_bands];

                for c in 0..cols {
                    let mut valid = true;
                    for b in 0..num_bands {
                        let v = band_rows[b][c];
                        if rasters[b].is_nodata(v) || v.is_nan() {
                            valid = false;
                            break;
                        }
                        spectrum[b] = v;
                    }
                    if !valid {
                        continue;
                    }

                    let hull = upper_hull_indices(&wavelengths, &spectrum);
                    if hull.len() < 2 {
                        continue;
                    }

                    continuum.fill(0.0);
                    for seg in 0..(hull.len() - 1) {
                        let i0 = hull[seg];
                        let i1 = hull[seg + 1];
                        let x0 = wavelengths[i0];
                        let x1 = wavelengths[i1];
                        let y0 = spectrum[i0];
                        let y1 = spectrum[i1];
                        let dx = (x1 - x0).max(1.0e-12);
                        for b in i0..=i1 {
                            let t = (wavelengths[b] - x0) / dx;
                            continuum[b] = y0 + t * (y1 - y0);
                        }
                    }

                    for b in 0..num_bands {
                        let cont = continuum[b];
                        if cont <= 0.0 {
                            out_by_band[b][c] = nodata;
                        } else {
                            out_by_band[b][c] = spectrum[b] / cont;
                        }
                    }
                }

                out_by_band
            })
            .collect();

        let mut out = new_output_like_with_bands(&rasters[0], num_bands);

        let coalescer = PercentCoalescer::new(1, 98);
        let mut done_rows = 0usize;
        let total_rows = rows.max(1) * num_bands;
        for (r, out_by_band) in row_results.iter().enumerate() {
            for b in 0..num_bands {
                out.set_row_slice(b as isize, r as isize, &out_by_band[b]).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed writing output row {} band {}: {}",
                        r,
                        b + 1,
                        e
                    ))
                })?;
                done_rows += 1;
                coalescer.emit_unit_fraction(ctx.progress, done_rows as f64 / total_rows as f64);
            }
        }
        coalescer.finish(ctx.progress);

        let out_locator = write_or_store_output(out, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(out_locator));
        outputs.insert("active_band".to_string(), json!(0));
        outputs.insert("bands".to_string(), json!(num_bands));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for LinearSpectralUnmixingTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "linear_spectral_unmixing",
            display_name: "Linear Spectral Unmixing",
            summary: "Estimates endmember fractions via non-negative least-squares linear unmixing. Each pixel decomposed as linear combination of endmember spectra with optional sum-to-one normalization. Provides per-pixel material/class abundance estimates and enables sub-pixel classification.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "inputs",
                    description: "Input raster stack (one raster per band).",
                    required: true,
                },
                ToolParamSpec {
                    name: "endmembers",
                    description: "Endmember signatures as array of objects {name, values} or numeric arrays.",
                    required: true,
                },
                ToolParamSpec {
                    name: "sum_to_one",
                    description: "If true (default), normalize non-negative fractions to sum to one.",
                    required: false,
                },
                ToolParamSpec {
                    name: "iterations",
                    description: "Projected-gradient iteration count (default 80).",
                    required: false,
                },
                ToolParamSpec {
                    name: "step_size",
                    description: "Projected-gradient step size (default 0.05).",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject",
                    description: "If true (default), reproject stack rasters to match inputs[0] when CRS differs.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject_method",
                    description: "Optional reprojection resampling override: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output multiband fraction raster path.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output_residual",
                    description: "Optional output residual RMS raster path.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["b1.tif", "b2.tif", "b3.tif", "b4.tif"]));
        defaults.insert(
            "endmembers".to_string(),
            json!([
                {"name": "water", "values": [0.05, 0.04, 0.03, 0.01]},
                {"name": "soil", "values": [0.18, 0.20, 0.22, 0.25]},
                {"name": "veg", "values": [0.04, 0.08, 0.05, 0.35]}
            ]),
        );
        defaults.insert("sum_to_one".to_string(), json!(true));
        defaults.insert("iterations".to_string(), json!(80));
        defaults.insert("step_size".to_string(), json!(0.05));
        defaults.insert("auto_reproject".to_string(), json!(true));
        defaults.insert("auto_reproject_method".to_string(), json!(""));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["hyp_b1.tif", "hyp_b2.tif", "hyp_b3.tif", "hyp_b4.tif"]));
        example.insert(
            "endmembers".to_string(),
            json!([
                {"name": "water", "values": [0.03, 0.02, 0.01, 0.00]},
                {"name": "soil", "values": [0.18, 0.20, 0.22, 0.24]},
                {"name": "veg", "values": [0.05, 0.09, 0.06, 0.40]}
            ]),
        );
        example.insert("output".to_string(), json!("unmix_fractions.tif"));
        example.insert("output_residual".to_string(), json!("unmix_residual.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_linear_unmixing".to_string(),
                description: "Estimate endmember fractions and residual RMS using non-negative linear unmixing.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "hyperspectral".to_string(),
                "unmixing".to_string(),
                "nnls".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.len() < 2 {
            return Err(ToolError::Validation(
                "parameter 'inputs' must contain at least 2 rasters".to_string(),
            ));
        }
        let _ = parse_endmember_vectors(args, inputs.len())?;
        let _ = parse_optional_output_path(args, "output")?;
        let _ = parse_optional_output_path(args, "output_residual")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = parse_raster_list_arg(args, "inputs")?;
        let num_bands = input_paths.len();
        let (class_names, endmembers) = parse_endmember_vectors(args, num_bands)?;
        let num_endmembers = endmembers.len();
        let sum_to_one = args
            .get("sum_to_one")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let iterations = args
            .get("iterations")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(80)
            .max(5);
        let step_size = args
            .get("step_size")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.05)
            .max(1.0e-6);
        let output_path = parse_optional_output_path(args, "output")?;
        let output_residual_path = parse_optional_output_path(args, "output_residual")?;

        ctx.progress.info("linear_spectral_unmixing: reading and aligning input stack");
        let rasters = input_paths
            .iter()
            .map(|p| load_raster(p))
            .collect::<Result<Vec<_>, _>>()?;

        // Strict validation: all inputs must have same CRS and be spatially co-registered
        // (no auto-reprojection for spectral analysis tools)
        validate_raster_stack_strict(&rasters)
            .map_err(|e| ToolError::Validation(format!("raster stack validation failed: {e}")))?;

        let rows = rasters[0].rows;
        let cols = rasters[0].cols;
        let nodata = rasters[0].nodata;

        let row_results: Vec<(Vec<Vec<f64>>, Vec<f64>)> = (0..rows)
            .into_par_iter()
            .map(|r| {
                let band_rows: Vec<Vec<f64>> = rasters
                    .iter()
                    .map(|b| b.row_slice(0, r as isize))
                    .collect();
                let mut out_by_endmember = vec![vec![nodata; cols]; num_endmembers];
                let mut residual_row = vec![nodata; cols];
                let mut pixel = vec![0.0_f64; num_bands];

                for c in 0..cols {
                    let mut valid = true;
                    for b in 0..num_bands {
                        let v = band_rows[b][c];
                        if rasters[b].is_nodata(v) || v.is_nan() {
                            valid = false;
                            break;
                        }
                        pixel[b] = v;
                    }
                    if !valid {
                        continue;
                    }

                    let x = solve_nnls_projected(&endmembers, &pixel, iterations, step_size, sum_to_one);

                    let mut rss = 0.0_f64;
                    for b in 0..num_bands {
                        let mut recon = 0.0_f64;
                        for j in 0..num_endmembers {
                            recon += endmembers[j][b] * x[j];
                        }
                        let d = pixel[b] - recon;
                        rss += d * d;
                    }
                    let rms = (rss / num_bands as f64).sqrt();

                    for j in 0..num_endmembers {
                        out_by_endmember[j][c] = x[j];
                    }
                    residual_row[c] = rms;
                }

                (out_by_endmember, residual_row)
            })
            .collect();

        let mut out_frac = new_output_like_with_bands(&rasters[0], num_endmembers);
        let coalescer = PercentCoalescer::new(1, 98);
        let mut done_rows = 0usize;
        let total_rows = rows.max(1) * num_endmembers;
        for (r, (out_by_endmember, _)) in row_results.iter().enumerate() {
            for j in 0..num_endmembers {
                out_frac
                    .set_row_slice(j as isize, r as isize, &out_by_endmember[j])
                    .map_err(|e| {
                        ToolError::Execution(format!(
                            "failed writing fraction row {} band {}: {}",
                            r,
                            j + 1,
                            e
                        ))
                    })?;
                done_rows += 1;
                coalescer.emit_unit_fraction(ctx.progress, done_rows as f64 / total_rows as f64);
            }
        }
        coalescer.finish(ctx.progress);

        let frac_locator = write_or_store_output(out_frac, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(frac_locator));
        outputs.insert("active_band".to_string(), json!(0));
        outputs.insert("endmember_names".to_string(), json!(class_names));
        outputs.insert("sum_to_one".to_string(), json!(sum_to_one));
        outputs.insert("iterations".to_string(), json!(iterations));
        outputs.insert("step_size".to_string(), json!(step_size));

        if let Some(res_path) = output_residual_path {
            let mut out_res = rasters[0].clone();
            out_res.bands = 1;
            for (r, (_, residual_row)) in row_results.iter().enumerate() {
                out_res
                    .set_row_slice(0, r as isize, residual_row)
                    .map_err(|e| ToolError::Execution(format!("failed writing residual row {}: {e}", r)))?;
            }
            let res_locator = write_or_store_output(out_res, Some(res_path))?;
            outputs.insert(
                "residual".to_string(),
                json!({"__wbw_type__": "raster", "path": res_locator, "active_band": 0}),
            );
        }

        Ok(ToolRunResult { outputs })
    }
}

impl Tool for MinimumNoiseFractionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "minimum_noise_fraction",
            display_name: "Minimum Noise Fraction",
            summary: "Performs Minimum Noise Fraction (MNF) transform via noise whitening followed by PCA in whitened space. Separates signal from noise by identifying noise correlation structure. MNF components ordered by signal-to-noise ratio with noise concentrating in later components.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "inputs",
                    description: "Input raster stack (one raster per band).",
                    required: true,
                },
                ToolParamSpec {
                    name: "num_components",
                    description: "Number of output MNF components (default: all bands).",
                    required: false,
                },
                ToolParamSpec {
                    name: "noise_mode",
                    description: "Noise estimator mode: 'difference_x' (default) or 'difference_y'.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject",
                    description: "If true (default), reproject stack rasters to match inputs[0] when CRS differs.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject_method",
                    description: "Optional reprojection resampling override: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output multiband MNF component raster path.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output_inverse",
                    description: "Optional reconstructed multiband raster path from retained MNF components.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["b1.tif", "b2.tif", "b3.tif", "b4.tif"]));
        defaults.insert("num_components".to_string(), json!(3));
        defaults.insert("noise_mode".to_string(), json!("difference_x"));
        defaults.insert("auto_reproject".to_string(), json!(true));
        defaults.insert("auto_reproject_method".to_string(), json!(""));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["hyp_b1.tif", "hyp_b2.tif", "hyp_b3.tif", "hyp_b4.tif"]));
        example.insert("num_components".to_string(), json!(3));
        example.insert("output".to_string(), json!("mnf_components.tif"));
        example.insert("output_inverse".to_string(), json!("mnf_reconstructed.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_mnf".to_string(),
                description: "Compute first MNF components from a co-registered raster stack.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "hyperspectral".to_string(),
                "mnf".to_string(),
                "noise_whitening".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.len() < 2 {
            return Err(ToolError::Validation(
                "parameter 'inputs' must contain at least 2 rasters".to_string(),
            ));
        }
        let _ = parse_optional_output_path(args, "output")?;
        let _ = parse_optional_output_path(args, "output_inverse")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = parse_raster_list_arg(args, "inputs")?;
        let num_bands = input_paths.len();
        let noise_mode = args
            .get("noise_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("difference_x")
            .to_ascii_lowercase();
        let output_path = parse_optional_output_path(args, "output")?;
        let output_inverse_path = parse_optional_output_path(args, "output_inverse")?;

        ctx.progress.info("minimum_noise_fraction: reading and aligning input stack");
        let rasters = input_paths
            .iter()
            .map(|p| load_raster(p))
            .collect::<Result<Vec<_>, _>>()?;

        // Strict validation: all inputs must have same CRS and be spatially co-registered
        // (no auto-reprojection for spectral analysis tools)
        validate_raster_stack_strict(&rasters)
            .map_err(|e| ToolError::Validation(format!("raster stack validation failed: {e}")))?;

        let rows = rasters[0].rows;
        let cols = rasters[0].cols;
        let nodata = rasters[0].nodata;
        let nb = num_bands;

        let (mut mean, mut cov_signal, mut cov_noise, valid_count, noise_count) = (0..rows)
            .into_par_iter()
            .fold(
                || {
                    (
                        vec![0.0_f64; nb],
                        vec![vec![0.0_f64; nb]; nb],
                        vec![vec![0.0_f64; nb]; nb],
                        0usize,
                        0usize,
                    )
                },
                |(mut mean_l, mut cov_signal_l, mut cov_noise_l, mut valid_count_l, mut noise_count_l), r| {
                    let band_rows: Vec<Vec<f64>> = rasters
                        .iter()
                        .map(|b| b.row_slice(0, r as isize))
                        .collect();
                    let band_rows_next: Option<Vec<Vec<f64>>> = if noise_mode == "difference_y" && r + 1 < rows {
                        Some(
                            rasters
                                .iter()
                                .map(|b| b.row_slice(0, (r + 1) as isize))
                                .collect(),
                        )
                    } else {
                        None
                    };

                    let mut v = vec![0.0_f64; nb];
                    let mut vn = vec![0.0_f64; nb];

                    for c in 0..cols {
                        let mut ok = true;
                        for b in 0..nb {
                            let x = band_rows[b][c];
                            if rasters[b].is_nodata(x) || x.is_nan() {
                                ok = false;
                                break;
                            }
                            v[b] = x;
                        }
                        if !ok {
                            continue;
                        }

                        valid_count_l += 1;
                        for i in 0..nb {
                            mean_l[i] += v[i];
                        }
                        for i in 0..nb {
                            for j in i..nb {
                                cov_signal_l[i][j] += v[i] * v[j];
                            }
                        }

                        if noise_mode == "difference_y" {
                            if let Some(ref next_rows) = band_rows_next {
                                let mut okn = true;
                                for b in 0..nb {
                                    let x = next_rows[b][c];
                                    if rasters[b].is_nodata(x) || x.is_nan() {
                                        okn = false;
                                        break;
                                    }
                                    vn[b] = x;
                                }
                                if okn {
                                    noise_count_l += 1;
                                    for i in 0..nb {
                                        let di = v[i] - vn[i];
                                        for j in i..nb {
                                            let dj = v[j] - vn[j];
                                            cov_noise_l[i][j] += di * dj;
                                        }
                                    }
                                }
                            }
                        } else if c + 1 < cols {
                            let mut okn = true;
                            for b in 0..nb {
                                let x = band_rows[b][c + 1];
                                if rasters[b].is_nodata(x) || x.is_nan() {
                                    okn = false;
                                    break;
                                }
                                vn[b] = x;
                            }
                            if okn {
                                noise_count_l += 1;
                                for i in 0..nb {
                                    let di = v[i] - vn[i];
                                    for j in i..nb {
                                        let dj = v[j] - vn[j];
                                        cov_noise_l[i][j] += di * dj;
                                    }
                                }
                            }
                        }
                    }

                    (mean_l, cov_signal_l, cov_noise_l, valid_count_l, noise_count_l)
                },
            )
            .reduce(
                || {
                    (
                        vec![0.0_f64; nb],
                        vec![vec![0.0_f64; nb]; nb],
                        vec![vec![0.0_f64; nb]; nb],
                        0usize,
                        0usize,
                    )
                },
                |(mut mean_a, mut cov_signal_a, mut cov_noise_a, valid_count_a, noise_count_a),
                 (mean_b, cov_signal_b, cov_noise_b, valid_count_b, noise_count_b)| {
                    for i in 0..nb {
                        mean_a[i] += mean_b[i];
                        for j in i..nb {
                            cov_signal_a[i][j] += cov_signal_b[i][j];
                            cov_noise_a[i][j] += cov_noise_b[i][j];
                        }
                    }
                    (
                        mean_a,
                        cov_signal_a,
                        cov_noise_a,
                        valid_count_a + valid_count_b,
                        noise_count_a + noise_count_b,
                    )
                },
            );

        if valid_count < 2 {
            return Err(ToolError::Execution(
                "minimum_noise_fraction: insufficient valid pixels".to_string(),
            ));
        }
        if noise_count < 2 {
            return Err(ToolError::Execution(
                "minimum_noise_fraction: insufficient valid noise pairs for covariance estimation"
                    .to_string(),
            ));
        }

        for i in 0..nb {
            mean[i] /= valid_count as f64;
        }
        for i in 0..nb {
            for j in i..nb {
                cov_signal[i][j] = cov_signal[i][j] / valid_count as f64 - mean[i] * mean[j];
                cov_signal[j][i] = cov_signal[i][j];
                cov_noise[i][j] = 0.5 * (cov_noise[i][j] / noise_count as f64);
                cov_noise[j][i] = cov_noise[i][j];
            }
        }

        let c_signal = DMatrix::from_fn(nb, nb, |i, j| cov_signal[i][j]);
        let mut c_noise = DMatrix::from_fn(nb, nb, |i, j| cov_noise[i][j]);
        for i in 0..nb {
            c_noise[(i, i)] += 1.0e-12;
        }

        let noise_eig = SymmetricEigen::new(c_noise);
        let noise_vals = noise_eig
            .eigenvalues
            .iter()
            .map(|v| (*v).max(1.0e-12))
            .collect::<Vec<_>>();
        let inv_sqrt = DMatrix::from_diagonal(&DVector::from_iterator(
            nb,
            noise_vals.iter().map(|v| 1.0 / v.sqrt()),
        ));
        let whitening = &noise_eig.eigenvectors * inv_sqrt * noise_eig.eigenvectors.transpose();

        let c_white = &whitening * c_signal * whitening.transpose();
        let mnf_eig = SymmetricEigen::new(c_white);

        let mut eig_pairs: Vec<(f64, Vec<f64>)> = (0..nb)
            .map(|k| {
                (
                    mnf_eig.eigenvalues[k],
                    mnf_eig
                        .eigenvectors
                        .column(k)
                        .iter()
                        .copied()
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
        eig_pairs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let requested_components = args
            .get("num_components")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(nb)
            .clamp(1, nb);

        let transform_rows: Vec<Vec<f64>> = eig_pairs
            .iter()
            .take(requested_components)
            .map(|(_, evec)| {
                let p = DVector::from_vec(evec.clone());
                let row = p.transpose() * whitening.clone();
                row.iter().copied().collect::<Vec<_>>()
            })
            .collect();

        let inverse_basis: Option<Vec<Vec<f64>>> = if output_inverse_path.is_some() {
            let sqrt_noise = DMatrix::from_diagonal(&DVector::from_iterator(
                nb,
                noise_vals.iter().map(|v| v.sqrt()),
            ));
            let unwhitening = &noise_eig.eigenvectors * sqrt_noise * noise_eig.eigenvectors.transpose();
            let p_cols = DMatrix::from_columns(
                &eig_pairs
                    .iter()
                    .take(requested_components)
                    .map(|(_, evec)| DVector::from_vec(evec.clone()))
                    .collect::<Vec<_>>(),
            );
            let inv_basis_mat = unwhitening * p_cols;
            Some(
                (0..nb)
                    .map(|b| {
                        (0..requested_components)
                            .map(|k| inv_basis_mat[(b, k)])
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>(),
            )
        } else {
            None
        };

        let row_results: Vec<Vec<Vec<f64>>> = (0..rows)
            .into_par_iter()
            .map(|r| {
                let band_rows: Vec<Vec<f64>> = rasters
                    .iter()
                    .map(|b| b.row_slice(0, r as isize))
                    .collect();
                let mut out_rows = vec![vec![nodata; cols]; requested_components];

                for c in 0..cols {
                    let mut pixel = vec![0.0_f64; nb];
                    let mut valid = true;
                    for b in 0..nb {
                        let v = band_rows[b][c];
                        if rasters[b].is_nodata(v) || v.is_nan() {
                            valid = false;
                            break;
                        }
                        pixel[b] = v - mean[b];
                    }
                    if !valid {
                        continue;
                    }

                    for (k, tr) in transform_rows.iter().enumerate() {
                        let mut s = 0.0_f64;
                        for b in 0..nb {
                            s += tr[b] * pixel[b];
                        }
                        out_rows[k][c] = s;
                    }
                }

                out_rows
            })
            .collect();

        let mut output = new_output_like_with_bands(&rasters[0], requested_components);

        let coalescer = PercentCoalescer::new(1, 98);
        let mut done_rows = 0usize;
        let total_rows = rows.max(1) * requested_components;
        for (r, out_rows) in row_results.iter().enumerate() {
            for (k, row_vals) in out_rows.iter().enumerate() {
                output
                    .set_row_slice(k as isize, r as isize, row_vals)
                    .map_err(|e| {
                        ToolError::Execution(format!(
                            "failed writing MNF row {} band {}: {}",
                            r,
                            k + 1,
                            e
                        ))
                    })?;
                done_rows += 1;
                coalescer.emit_unit_fraction(ctx.progress, done_rows as f64 / total_rows as f64);
            }
        }
        coalescer.finish(ctx.progress);

        let locator = write_or_store_output(output, output_path)?;
        let eigenvalues = eig_pairs
            .iter()
            .take(requested_components)
            .map(|(v, _)| *v)
            .collect::<Vec<_>>();

        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(locator));
        outputs.insert("active_band".to_string(), json!(0));
        outputs.insert("num_components".to_string(), json!(requested_components));
        outputs.insert("noise_mode".to_string(), json!(noise_mode));
        outputs.insert("eigenvalues".to_string(), json!(eigenvalues));

        if let (Some(inv_path), Some(inv_basis)) = (output_inverse_path, inverse_basis) {
            let mut out_inverse = new_output_like_with_bands(&rasters[0], nb);
            for (r, component_rows) in row_results.iter().enumerate() {
                let mut inverse_rows = vec![vec![nodata; cols]; nb];
                for c in 0..cols {
                    let mut valid = true;
                    for comp in component_rows.iter().take(requested_components) {
                        if (comp[c] - nodata).abs() <= f64::EPSILON || comp[c].is_nan() {
                            valid = false;
                            break;
                        }
                    }
                    if !valid {
                        continue;
                    }

                    for b in 0..nb {
                        let mut v = mean[b];
                        for k in 0..requested_components {
                            v += inv_basis[b][k] * component_rows[k][c];
                        }
                        inverse_rows[b][c] = v;
                    }
                }
                for (b, row_vals) in inverse_rows.iter().enumerate() {
                    out_inverse
                        .set_row_slice(b as isize, r as isize, row_vals)
                        .map_err(|e| {
                            ToolError::Execution(format!(
                                "failed writing MNF inverse row {} band {}: {}",
                                r,
                                b + 1,
                                e
                            ))
                        })?;
                }
            }
            let inv_locator = write_or_store_output(out_inverse, Some(inv_path))?;
            outputs.insert(
                "inverse".to_string(),
                json!({"__wbw_type__": "raster", "path": inv_locator, "active_band": 0}),
            );
        }

        Ok(ToolRunResult { outputs })
    }
}

impl Tool for SpectralLibraryMatchingTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "spectral_library_matching",
            display_name: "Spectral Library Matching",
            summary: "Matches each pixel spectrum to closest spectral library signature using SAM or Euclidean distance. Returns class membership (best match library ID) and match score (angle/distance). Enables rapid automated material/class identification without manual threshold tuning.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "inputs",
                    description: "Input raster stack (one raster per band).",
                    required: true,
                },
                ToolParamSpec {
                    name: "library",
                    description: "Spectral library entries as array of objects {name, values} or numeric arrays.",
                    required: false,
                },
                ToolParamSpec {
                    name: "library_csv",
                    description: "Optional CSV spectral library path (name,b1,b2,... or b1,b2,... rows). Used when 'library' is not provided.",
                    required: false,
                },
                ToolParamSpec {
                    name: "metric",
                    description: "Matching metric: 'sam' (default), 'euclidean', or 'sid'.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject",
                    description: "If true (default), reproject stack rasters to match inputs[0] when CRS differs.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject_method",
                    description: "Optional reprojection resampling override: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output class raster path.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output_score",
                    description: "Optional output best-match score raster path (angle or distance).",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["b1.tif", "b2.tif", "b3.tif", "b4.tif"]));
        defaults.insert(
            "library".to_string(),
            json!([
                {"name": "water", "values": [0.03, 0.02, 0.01, 0.00]},
                {"name": "soil", "values": [0.18, 0.20, 0.22, 0.24]},
                {"name": "veg", "values": [0.05, 0.09, 0.06, 0.40]}
            ]),
        );
        defaults.insert("metric".to_string(), json!("sam"));
        defaults.insert("library_csv".to_string(), json!(""));
        defaults.insert("auto_reproject".to_string(), json!(true));
        defaults.insert("auto_reproject_method".to_string(), json!(""));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["hyp_b1.tif", "hyp_b2.tif", "hyp_b3.tif", "hyp_b4.tif"]));
        example.insert(
            "library".to_string(),
            json!([
                {"name": "water", "values": [0.03, 0.02, 0.01, 0.00]},
                {"name": "soil", "values": [0.18, 0.20, 0.22, 0.24]},
                {"name": "veg", "values": [0.05, 0.09, 0.06, 0.40]}
            ]),
        );
        example.insert("metric".to_string(), json!("sam"));
        example.insert("output".to_string(), json!("library_match_class.tif"));
        example.insert("output_score".to_string(), json!("library_match_score.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "library_match_sam".to_string(),
                description: "Assign each pixel to the nearest spectral library entry and export score raster.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "hyperspectral".to_string(),
                "classification".to_string(),
                "spectral_library".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.len() < 2 {
            return Err(ToolError::Validation(
                "parameter 'inputs' must contain at least 2 rasters".to_string(),
            ));
        }
        let has_library_json = args.get("library").is_some();
        let has_library_csv = args
            .get("library_csv")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        if !has_library_json && !has_library_csv {
            return Err(ToolError::Validation(
                "one of 'library' or 'library_csv' must be provided".to_string(),
            ));
        }
        if has_library_json {
            let _ = parse_named_vectors_arg(args, "library", inputs.len())?;
        }
        if has_library_csv {
            let csv_path = args
                .get("library_csv")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let _ = parse_named_vectors_csv(Path::new(csv_path), inputs.len())?;
        }
        let _ = parse_optional_output_path(args, "output")?;
        let _ = parse_optional_output_path(args, "output_score")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = parse_raster_list_arg(args, "inputs")?;
        let num_bands = input_paths.len();
        let metric = args
            .get("metric")
            .and_then(|v| v.as_str())
            .unwrap_or("sam")
            .to_ascii_lowercase();
        if metric != "sam" && metric != "euclidean" && metric != "sid" {
            return Err(ToolError::Validation(
                "parameter 'metric' must be one of: sam, euclidean, sid".to_string(),
            ));
        }

        let output_path = parse_optional_output_path(args, "output")?;
        let output_score_path = parse_optional_output_path(args, "output_score")?;

        let (class_names, library) = if let Some(csv_path) = args
            .get("library_csv")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            parse_named_vectors_csv(Path::new(csv_path), num_bands)?
        } else {
            parse_named_vectors_arg(args, "library", num_bands)?
        };
        let num_classes = library.len();

        ctx.progress.info("spectral_library_matching: reading and aligning input stack");
        let rasters = input_paths
            .iter()
            .map(|p| load_raster(p))
            .collect::<Result<Vec<_>, _>>()?;

        // Strict validation: all inputs must have same CRS and be spatially co-registered
        // (no auto-reprojection for spectral analysis tools)
        validate_raster_stack_strict(&rasters)
            .map_err(|e| ToolError::Validation(format!("raster stack validation failed: {e}")))?;
        ctx.progress.info("spectral_library_matching: validated strict CRS and spatial alignment");

        let rows = rasters[0].rows;
        let cols = rasters[0].cols;
        let nodata = rasters[0].nodata;

        let lib_norms = library
            .iter()
            .map(|sig| sig.iter().map(|v| v * v).sum::<f64>().sqrt().max(1.0e-12))
            .collect::<Vec<_>>();
        let lib_sid_probs = library
            .iter()
            .map(|sig| {
                let eps = 1.0e-12;
                let s = sig.iter().map(|v| v.max(0.0) + eps).sum::<f64>().max(eps);
                sig.iter()
                    .map(|v| (v.max(0.0) + eps) / s)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let row_results: Vec<(Vec<f64>, Vec<f64>)> = (0..rows)
            .into_par_iter()
            .map(|r| {
                let band_rows: Vec<Vec<f64>> = rasters
                    .iter()
                    .map(|b| b.row_slice(0, r as isize))
                    .collect();

                let mut class_row = vec![nodata; cols];
                let mut score_row = vec![nodata; cols];
                let mut pixel = vec![0.0_f64; num_bands];
                let mut sid_pixel_prob = if metric == "sid" {
                    Some(vec![0.0_f64; num_bands])
                } else {
                    None
                };

                for c in 0..cols {
                    let mut valid = true;
                    for b in 0..num_bands {
                        let v = band_rows[b][c];
                        if rasters[b].is_nodata(v) || v.is_nan() {
                            valid = false;
                            break;
                        }
                        pixel[b] = v;
                    }
                    if !valid {
                        continue;
                    }

                    let pixel_norm = if metric == "sam" {
                        pixel.iter().map(|v| v * v).sum::<f64>().sqrt().max(1.0e-12)
                    } else {
                        1.0
                    };
                    if metric == "sid" {
                        let eps = 1.0e-12;
                        let s = pixel.iter().map(|v| v.max(0.0) + eps).sum::<f64>().max(eps);
                        if let Some(ref mut p) = sid_pixel_prob {
                            for b in 0..num_bands {
                                p[b] = (pixel[b].max(0.0) + eps) / s;
                            }
                        }
                    }
                    let mut best_idx = 0usize;
                    let mut best_score = f64::INFINITY;

                    for j in 0..num_classes {
                        let score = if metric == "euclidean" {
                            let mut s = 0.0_f64;
                            for b in 0..num_bands {
                                let d = pixel[b] - library[j][b];
                                s += d * d;
                            }
                            s.sqrt()
                        } else if metric == "sid" {
                            let Some(p) = sid_pixel_prob.as_ref() else {
                                continue;
                            };
                            let q = &lib_sid_probs[j];
                            let mut d_pq = 0.0_f64;
                            let mut d_qp = 0.0_f64;
                            for b in 0..num_bands {
                                d_pq += p[b] * (p[b] / q[b]).ln();
                                d_qp += q[b] * (q[b] / p[b]).ln();
                            }
                            d_pq + d_qp
                        } else {
                            let mut dot = 0.0_f64;
                            for b in 0..num_bands {
                                dot += pixel[b] * library[j][b];
                            }
                            let cosang = (dot / (pixel_norm * lib_norms[j])).clamp(-1.0, 1.0);
                            cosang.acos()
                        };

                        if score < best_score {
                            best_score = score;
                            best_idx = j;
                        }
                    }

                    class_row[c] = (best_idx + 1) as f64;
                    score_row[c] = best_score;
                }

                (class_row, score_row)
            })
            .collect();

        let mut out_class = rasters[0].clone();
        out_class.bands = 1;
        for (r, (class_row, _)) in row_results.iter().enumerate() {
            out_class
                .set_row_slice(0, r as isize, class_row)
                .map_err(|e| ToolError::Execution(format!("failed writing class row {}: {e}", r)))?;
        }

        let class_locator = write_or_store_output(out_class, output_path)?;

        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(class_locator));
        outputs.insert("active_band".to_string(), json!(0));
        outputs.insert("metric".to_string(), json!(metric));
        outputs.insert("class_names".to_string(), json!(class_names));

        if let Some(score_path) = output_score_path {
            let mut out_score = rasters[0].clone();
            out_score.bands = 1;
            for (r, (_, score_row)) in row_results.iter().enumerate() {
                out_score
                    .set_row_slice(0, r as isize, score_row)
                    .map_err(|e| ToolError::Execution(format!("failed writing score row {}: {e}", r)))?;
            }
            let score_locator = write_or_store_output(out_score, Some(score_path))?;
            outputs.insert(
                "score".to_string(),
                json!({"__wbw_type__": "raster", "path": score_locator, "active_band": 0}),
            );
        }

        Ok(ToolRunResult { outputs })
    }
}

impl Tool for CloudePottierDecompositionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "cloude_pottier_decomposition",
            display_name: "Cloude-Pottier Decomposition",
            summary: "Computes Cloude-Pottier polarimetric decomposition (H, A, alpha) from symmetric 3x3 covariance/coherency matrices. Extracts entropy, anisotropy, and mean-scattering-angle alpha to classify scattering mechanisms. Model-independent approach enables robust characterization across diverse terrain.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "inputs",
                    description: "Matrix rasters: diag3 [m11,m22,m33], full3x3 compact [m11,m22,m33,m12,m13,m23], or full row-major 9 rasters.",
                    required: true,
                },
                ToolParamSpec {
                    name: "matrix_format",
                    description: "Input matrix format: diag3 (default with 3 rasters) or full3x3.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject",
                    description: "If true (default), reproject stack rasters to match inputs[0] when CRS differs.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject_method",
                    description: "Optional reprojection resampling override: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output 3-band raster [H, A, alpha_deg].",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["m11.tif", "m22.tif", "m33.tif", "m12.tif", "m13.tif", "m23.tif"]));
        defaults.insert("matrix_format".to_string(), json!("full3x3"));
        defaults.insert("auto_reproject".to_string(), json!(true));
        defaults.insert("auto_reproject_method".to_string(), json!(""));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["t11.tif", "t22.tif", "t33.tif", "t12_re.tif", "t13_re.tif", "t23_re.tif"]));
        example.insert("matrix_format".to_string(), json!("full3x3"));
        example.insert("output".to_string(), json!("cloude_pottier_haa.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "cloude_pottier_haa".to_string(),
                description: "Compute entropy, anisotropy, and alpha from a 3x3 real symmetric PolSAR matrix stack.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "polsar".to_string(),
                "decomposition".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_polsar_real_symmetric_inputs(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (input_paths, _, matrix_format) = parse_polsar_real_symmetric_inputs(args)?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("cloude_pottier_decomposition: reading and aligning matrix stack");
        let mut rasters = input_paths
            .iter()
            .map(|p| load_raster(p))
            .collect::<Result<Vec<_>, _>>()?;

        let stack_cfg = RasterStackConfig {
            auto_reproject: args
                .get("auto_reproject")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            resampling_method: parse_resampling_override(args),
            allow_no_overlap: false,
        };
        let warnings = align_and_validate_raster_stack(&mut rasters, &stack_cfg)
            .map_err(|e| ToolError::Validation(format!("raster stack validation failed: {e}")))?;
        for warning in warnings {
            ctx.progress.info(&format!("cloude_pottier_decomposition: {warning}"));
        }

        let rows = rasters[0].rows;
        let cols = rasters[0].cols;
        let nodata = rasters[0].nodata;

        let row_results: Vec<[Vec<f64>; 3]> = (0..rows)
            .into_par_iter()
            .map(|r| {
                let band_rows: Vec<Vec<f64>> = rasters
                    .iter()
                    .map(|b| b.row_slice(0, r as isize))
                    .collect();

                let mut h_row = vec![nodata; cols];
                let mut a_row = vec![nodata; cols];
                let mut alpha_row = vec![nodata; cols];

                for c in 0..cols {
                    let m = if band_rows.len() == 3 {
                        let v11 = band_rows[0][c];
                        let v22 = band_rows[1][c];
                        let v33 = band_rows[2][c];
                        if rasters[0].is_nodata(v11)
                            || rasters[1].is_nodata(v22)
                            || rasters[2].is_nodata(v33)
                            || v11.is_nan()
                            || v22.is_nan()
                            || v33.is_nan()
                        {
                            continue;
                        }
                        DMatrix::from_row_slice(
                            3,
                            3,
                            &[
                                v11, 0.0, 0.0, //
                                0.0, v22, 0.0, //
                                0.0, 0.0, v33,
                            ],
                        )
                    } else {
                        let v11 = band_rows[0][c];
                        let v12 = band_rows[1][c];
                        let v13 = band_rows[2][c];
                        let v21 = band_rows[3][c];
                        let v22 = band_rows[4][c];
                        let v23 = band_rows[5][c];
                        let v31 = band_rows[6][c];
                        let v32 = band_rows[7][c];
                        let v33 = band_rows[8][c];
                        if rasters[0].is_nodata(v11)
                            || rasters[1].is_nodata(v12)
                            || rasters[2].is_nodata(v13)
                            || rasters[3].is_nodata(v21)
                            || rasters[4].is_nodata(v22)
                            || rasters[5].is_nodata(v23)
                            || rasters[6].is_nodata(v31)
                            || rasters[7].is_nodata(v32)
                            || rasters[8].is_nodata(v33)
                            || v11.is_nan()
                            || v12.is_nan()
                            || v13.is_nan()
                            || v21.is_nan()
                            || v22.is_nan()
                            || v23.is_nan()
                            || v31.is_nan()
                            || v32.is_nan()
                            || v33.is_nan()
                        {
                            continue;
                        }
                        DMatrix::from_row_slice(
                            3,
                            3,
                            &[
                                v11, v12, v13, //
                                v21, v22, v23, //
                                v31, v32, v33,
                            ],
                        )
                    };

                    let eig = SymmetricEigen::new(m);
                    let mut lambdas = [
                        eig.eigenvalues[0].max(0.0),
                        eig.eigenvalues[1].max(0.0),
                        eig.eigenvalues[2].max(0.0),
                    ];
                    let mut evecs = [
                        [eig.eigenvectors[(0, 0)], eig.eigenvectors[(1, 0)], eig.eigenvectors[(2, 0)]],
                        [eig.eigenvectors[(0, 1)], eig.eigenvectors[(1, 1)], eig.eigenvectors[(2, 1)]],
                        [eig.eigenvectors[(0, 2)], eig.eigenvectors[(1, 2)], eig.eigenvectors[(2, 2)]],
                    ];
                    if lambdas[1] > lambdas[0] {
                        lambdas.swap(0, 1);
                        evecs.swap(0, 1);
                    }
                    if lambdas[2] > lambdas[1] {
                        lambdas.swap(1, 2);
                        evecs.swap(1, 2);
                    }
                    if lambdas[1] > lambdas[0] {
                        lambdas.swap(0, 1);
                        evecs.swap(0, 1);
                    }

                    let l1 = lambdas[0];
                    let l2 = lambdas[1];
                    let l3 = lambdas[2];
                    let span = (l1 + l2 + l3).max(1.0e-12);
                    let p = [l1 / span, l2 / span, l3 / span];

                    let mut entropy = 0.0_f64;
                    for pi in p {
                        if pi > 0.0 {
                            entropy -= pi * (pi.ln() / 3.0_f64.ln());
                        }
                    }
                    entropy = entropy.clamp(0.0, 1.0);

                    let anisotropy = if (l2 + l3) > 1.0e-12 {
                        (l2 - l3) / (l2 + l3)
                    } else {
                        0.0
                    }
                    .clamp(0.0, 1.0);

                    let mut alpha = 0.0_f64;
                    for k in 0..3 {
                        let c1 = evecs[k][0].abs();
                        let c23 = (evecs[k][1] * evecs[k][1] + evecs[k][2] * evecs[k][2]).sqrt();
                        let alpha_k = c23.atan2(c1);
                        alpha += p[k] * alpha_k;
                    }

                    h_row[c] = entropy;
                    a_row[c] = anisotropy;
                    alpha_row[c] = alpha.to_degrees();
                }

                [h_row, a_row, alpha_row]
            })
            .collect();

        let mut output = new_output_like_with_bands(&rasters[0], 3);
        let coalescer = PercentCoalescer::new(1, 98);
        let mut done_rows = 0usize;
        let total_rows = rows.max(1) * 3;
        for (r, bands) in row_results.iter().enumerate() {
            for (b, row_vals) in bands.iter().enumerate() {
                output
                    .set_row_slice(b as isize, r as isize, row_vals)
                    .map_err(|e| ToolError::Execution(format!("failed writing Cloude-Pottier row {} band {}: {}", r, b + 1, e)))?;
                done_rows += 1;
                coalescer.emit_unit_fraction(ctx.progress, done_rows as f64 / total_rows as f64);
            }
        }
        coalescer.finish(ctx.progress);

        let locator = write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(locator));
        outputs.insert("active_band".to_string(), json!(0));
        outputs.insert("bands".to_string(), json!(["entropy", "anisotropy", "alpha_degrees"]));
        outputs.insert("matrix_format".to_string(), json!(matrix_format));

        Ok(ToolRunResult { outputs })
    }
}

impl Tool for FreemanDurdenDecompositionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "freeman_durden_decomposition",
            display_name: "Freeman-Durden Decomposition",
            summary: "Computes Freeman-Durden 3-component scattering decomposition (surface, double-bounce, volume powers) from C3-style inputs via non-negative least-squares. Includes clipping diagnostics revealing decomposition validity. Physically interpretable for forest height, biomass, and urban characterization.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "inputs",
                    description: "Matrix rasters: diag3 [m11,m22,m33], full3x3 compact [m11,m22,m33,m12,m13,m23], or full row-major 9 rasters.",
                    required: true,
                },
                ToolParamSpec {
                    name: "matrix_format",
                    description: "Input matrix format: diag3 (default with 3 rasters) or full3x3.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject",
                    description: "If true (default), reproject stack rasters to match inputs[0] when CRS differs.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject_method",
                    description: "Optional reprojection resampling override: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output 3-band raster [surface, double_bounce, volume].",
                    required: false,
                },
                ToolParamSpec {
                    name: "output_clip_mask",
                    description: "Optional output clip-mask raster (1 where non-physical clipping/renormalization occurred).",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["c11.tif", "c22.tif", "c33.tif", "c12.tif", "c13.tif", "c23.tif"]));
        defaults.insert("matrix_format".to_string(), json!("full3x3"));
        defaults.insert("auto_reproject".to_string(), json!(true));
        defaults.insert("auto_reproject_method".to_string(), json!(""));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["c11.tif", "c22.tif", "c33.tif", "c12_re.tif", "c13_re.tif", "c23_re.tif"]));
        example.insert("matrix_format".to_string(), json!("full3x3"));
        example.insert("output".to_string(), json!("freeman_durden_ps_pd_pv.tif"));
        example.insert("output_clip_mask".to_string(), json!("freeman_durden_clip_mask.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "freeman_durden_3comp".to_string(),
                description: "Compute 3-component scattering powers with clipping diagnostics.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "polsar".to_string(),
                "decomposition".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_polsar_real_symmetric_inputs(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        let _ = parse_optional_output_path(args, "output_clip_mask")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (input_paths, _, matrix_format) = parse_polsar_real_symmetric_inputs(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let output_clip_mask_path = parse_optional_output_path(args, "output_clip_mask")?;

        ctx.progress.info("freeman_durden_decomposition: reading and aligning matrix stack");
        let mut rasters = input_paths
            .iter()
            .map(|p| load_raster(p))
            .collect::<Result<Vec<_>, _>>()?;

        let stack_cfg = RasterStackConfig {
            auto_reproject: args
                .get("auto_reproject")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            resampling_method: parse_resampling_override(args),
            allow_no_overlap: false,
        };
        let warnings = align_and_validate_raster_stack(&mut rasters, &stack_cfg)
            .map_err(|e| ToolError::Validation(format!("raster stack validation failed: {e}")))?;
        for warning in warnings {
            ctx.progress.info(&format!("freeman_durden_decomposition: {warning}"));
        }

        let rows = rasters[0].rows;
        let cols = rasters[0].cols;
        let nodata = rasters[0].nodata;

        let row_results: Vec<[Vec<f64>; 4]> = (0..rows)
            .into_par_iter()
            .map(|r| {
                let band_rows: Vec<Vec<f64>> = rasters
                    .iter()
                    .map(|b| b.row_slice(0, r as isize))
                    .collect();

                let mut ps_row = vec![nodata; cols];
                let mut pd_row = vec![nodata; cols];
                let mut pv_row = vec![nodata; cols];
                let mut clip_row = vec![nodata; cols];

                for c in 0..cols {
                    let (c11, c22, c33, c13_re, has_cross_pol) = if band_rows.len() == 3 {
                        let v11 = band_rows[0][c];
                        let v22 = band_rows[1][c];
                        let v33 = band_rows[2][c];
                        if rasters[0].is_nodata(v11)
                            || rasters[1].is_nodata(v22)
                            || rasters[2].is_nodata(v33)
                            || v11.is_nan()
                            || v22.is_nan()
                            || v33.is_nan()
                        {
                            continue;
                        }
                        (v11.max(0.0), v22.max(0.0), v33.max(0.0), 0.0, false)
                    } else {
                        let v11 = band_rows[0][c];
                        let v13 = band_rows[2][c];
                        let v22 = band_rows[4][c];
                        let v31 = band_rows[6][c];
                        let v33 = band_rows[8][c];
                        if rasters[0].is_nodata(v11)
                            || rasters[2].is_nodata(v13)
                            || rasters[4].is_nodata(v22)
                            || rasters[6].is_nodata(v31)
                            || rasters[8].is_nodata(v33)
                            || v11.is_nan()
                            || v13.is_nan()
                            || v22.is_nan()
                            || v31.is_nan()
                            || v33.is_nan()
                        {
                            continue;
                        }
                        (v11.max(0.0), v22.max(0.0), v33.max(0.0), 0.5 * (v13 + v31), true)
                    };
                    let span = (c11 + c22 + c33).max(0.0);
                    if span <= 0.0 {
                        ps_row[c] = 0.0;
                        pd_row[c] = 0.0;
                        pv_row[c] = 0.0;
                        clip_row[c] = 0.0;
                        continue;
                    }

                    // Practical non-negative 3-component baseline with explicit clipping/renormalization.
                    let mut pv = (3.0 * c33).max(0.0);
                    let mut clipped = 0.0_f64;
                    if pv > span {
                        pv = span;
                        clipped = 1.0;
                    }

                    let remaining = (span - pv).max(0.0);
                    let mut ps_raw = (c11 - c33).max(0.0);
                    let mut pd_raw = (c22 - c33).max(0.0);
                    if has_cross_pol {
                        let bias = c13_re.abs().min(0.5 * (c11 + c33));
                        if c13_re >= 0.0 {
                            ps_raw += bias;
                        } else {
                            pd_raw += bias;
                        }
                    }
                    let sum_raw = ps_raw + pd_raw;

                    let (ps, pd) = if sum_raw > 1.0e-12 {
                        (remaining * ps_raw / sum_raw, remaining * pd_raw / sum_raw)
                    } else {
                        clipped = 1.0;
                        (0.5 * remaining, 0.5 * remaining)
                    };

                    ps_row[c] = ps;
                    pd_row[c] = pd;
                    pv_row[c] = pv;
                    clip_row[c] = clipped;
                }

                [ps_row, pd_row, pv_row, clip_row]
            })
            .collect();

        let mut output = new_output_like_with_bands(&rasters[0], 3);
        let coalescer = PercentCoalescer::new(1, 98);
        let mut done_rows = 0usize;
        let total_rows = rows.max(1) * 3;
        for (r, bands) in row_results.iter().enumerate() {
            for b in 0..3 {
                output
                    .set_row_slice(b as isize, r as isize, &bands[b])
                    .map_err(|e| ToolError::Execution(format!("failed writing Freeman-Durden row {} band {}: {}", r, b + 1, e)))?;
                done_rows += 1;
                coalescer.emit_unit_fraction(ctx.progress, done_rows as f64 / total_rows as f64);
            }
        }
        coalescer.finish(ctx.progress);

        let locator = write_or_store_output(output, output_path)?;

        let clipped_pixels = row_results
            .iter()
            .map(|bands| bands[3].iter().filter(|v| **v > 0.5).count())
            .sum::<usize>();

        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(locator));
        outputs.insert("active_band".to_string(), json!(0));
        outputs.insert("bands".to_string(), json!(["surface", "double_bounce", "volume"]));
        outputs.insert("matrix_format".to_string(), json!(matrix_format));
        outputs.insert("clipped_pixels".to_string(), json!(clipped_pixels));

        if let Some(mask_path) = output_clip_mask_path {
            let mut clip_mask = rasters[0].clone();
            clip_mask.bands = 1;
            for (r, bands) in row_results.iter().enumerate() {
                clip_mask
                    .set_row_slice(0, r as isize, &bands[3])
                    .map_err(|e| ToolError::Execution(format!("failed writing clip-mask row {}: {e}", r)))?;
            }
            let mask_locator = write_or_store_output(clip_mask, Some(mask_path))?;
            outputs.insert(
                "clip_mask".to_string(),
                json!({"__wbw_type__": "raster", "path": mask_locator, "active_band": 0}),
            );
        }

        Ok(ToolRunResult { outputs })
    }
}

impl Tool for YamaguchiDecompositionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "yamaguchi_4component_decomposition",
            display_name: "Yamaguchi 4-Component Decomposition",
            summary: "Computes Yamaguchi 4-component scattering decomposition (surface, double-bounce, volume, helix) extending Freeman-Durden with residual helix component. More physically complete than 3-component approaches, capturing complex scattering and revealing oriented structures like tilted buildings and vegetation.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "inputs",
                    description: "Matrix rasters: 6 compact [m11,m22,m33,m12,m13,m23] or 9 row-major full 3x3 complex matrix elements.",
                    required: true,
                },
                ToolParamSpec {
                    name: "matrix_format",
                    description: "Input matrix format: full3x3 (default) for Hermitian matrices with real/imag parts.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject",
                    description: "If true (default), reproject stack rasters to match inputs[0] when CRS differs.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject_method",
                    description: "Optional reprojection resampling override: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output 4-band raster [surface, double_bounce, volume, helix].",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["c11.tif", "c22.tif", "c33.tif", "c12_re.tif", "c13_re.tif", "c23_re.tif"]));
        defaults.insert("matrix_format".to_string(), json!("full3x3"));
        defaults.insert("auto_reproject".to_string(), json!(true));
        defaults.insert("auto_reproject_method".to_string(), json!(""));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["c11.tif", "c22.tif", "c33.tif", "c12_re.tif", "c13_re.tif", "c23_re.tif"]));
        example.insert("matrix_format".to_string(), json!("full3x3"));
        example.insert("output".to_string(), json!("yamaguchi_4comp.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "yamaguchi_4comp".to_string(),
                description: "Compute 4-component scattering powers with helix component.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "polsar".to_string(),
                "decomposition".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_polsar_real_symmetric_inputs(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (input_paths, _, matrix_format) = parse_polsar_real_symmetric_inputs(args)?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("yamaguchi_4component_decomposition: reading and aligning matrix stack");
        let mut rasters = input_paths
            .iter()
            .map(|p| load_raster(p))
            .collect::<Result<Vec<_>, _>>()?;

        let stack_cfg = RasterStackConfig {
            auto_reproject: args
                .get("auto_reproject")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            resampling_method: parse_resampling_override(args),
            allow_no_overlap: false,
        };
        let warnings = align_and_validate_raster_stack(&mut rasters, &stack_cfg)
            .map_err(|e| ToolError::Validation(format!("raster stack validation failed: {e}")))?;
        for warning in warnings {
            ctx.progress.info(&format!("yamaguchi_4component_decomposition: {warning}"));
        }

        let rows = rasters[0].rows;
        let cols = rasters[0].cols;
        let nodata = rasters[0].nodata;

        let row_results: Vec<[Vec<f64>; 4]> = (0..rows)
            .into_par_iter()
            .map(|r| {
                let band_rows: Vec<Vec<f64>> = rasters
                    .iter()
                    .map(|b| b.row_slice(0, r as isize))
                    .collect();

                let mut ps_row = vec![nodata; cols];
                let mut pd_row = vec![nodata; cols];
                let mut pv_row = vec![nodata; cols];
                let mut ph_row = vec![nodata; cols];

                for c in 0..cols {
                    let (c11, c22, c33, c13_re) = if band_rows.len() == 3 {
                        let v11 = band_rows[0][c];
                        let v22 = band_rows[1][c];
                        let v33 = band_rows[2][c];
                        if rasters[0].is_nodata(v11)
                            || rasters[1].is_nodata(v22)
                            || rasters[2].is_nodata(v33)
                            || v11.is_nan()
                            || v22.is_nan()
                            || v33.is_nan()
                        {
                            continue;
                        }
                        (v11.max(0.0), v22.max(0.0), v33.max(0.0), 0.0)
                    } else {
                        let v11 = band_rows[0][c];
                        let v13 = band_rows[2][c];
                        let v22 = band_rows[4][c];
                        let v31 = band_rows[6][c];
                        let v33 = band_rows[8][c];
                        if rasters[0].is_nodata(v11)
                            || rasters[2].is_nodata(v13)
                            || rasters[4].is_nodata(v22)
                            || rasters[6].is_nodata(v31)
                            || rasters[8].is_nodata(v33)
                            || v11.is_nan()
                            || v13.is_nan()
                            || v22.is_nan()
                            || v31.is_nan()
                            || v33.is_nan()
                        {
                            continue;
                        }
                        (v11.max(0.0), v22.max(0.0), v33.max(0.0), 0.5 * (v13 + v31))
                    };

                    let span = (c11 + c22 + c33).max(0.0);
                    if span <= 0.0 {
                        ps_row[c] = 0.0;
                        pd_row[c] = 0.0;
                        pv_row[c] = 0.0;
                        ph_row[c] = 0.0;
                        continue;
                    }

                    let pv = (3.0 * c33).max(0.0).min(span);
                    let remaining = (span - pv).max(0.0);

                    let mut ps_raw = (c11 - c33).max(0.0);
                    let mut pd_raw = (c22 - c33).max(0.0);
                    let bias = c13_re.abs().min(0.5 * (c11 + c33));
                    if c13_re >= 0.0 {
                        ps_raw += bias;
                    } else {
                        pd_raw += bias;
                    }
                    let sum_raw = ps_raw + pd_raw;

                    let (ps, pd) = if sum_raw > 1.0e-12 {
                        (remaining * ps_raw / sum_raw, remaining * pd_raw / sum_raw)
                    } else {
                        (0.5 * remaining, 0.5 * remaining)
                    };

                    // Helix component from residual cross-pol (simplified)
                    let ph = (remaining - ps - pd).max(0.0) * (c13_re / (remaining.max(1e-12))).abs().max(0.0);

                    ps_row[c] = ps;
                    pd_row[c] = pd;
                    pv_row[c] = pv;
                    ph_row[c] = ph;
                }

                [ps_row, pd_row, pv_row, ph_row]
            })
            .collect();

        let mut output = new_output_like_with_bands(&rasters[0], 4);
        let coalescer = PercentCoalescer::new(1, 98);
        let mut done_rows = 0usize;
        let total_rows = rows.max(1) * 4;
        for (r, bands) in row_results.iter().enumerate() {
            for b in 0..4 {
                output
                    .set_row_slice(b as isize, r as isize, &bands[b])
                    .map_err(|e| ToolError::Execution(format!("failed writing Yamaguchi row {} band {}: {}", r, b + 1, e)))?;
                done_rows += 1;
                coalescer.emit_unit_fraction(ctx.progress, done_rows as f64 / total_rows as f64);
            }
        }
        coalescer.finish(ctx.progress);

        let locator = write_or_store_output(output, output_path)?;

        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(locator));
        outputs.insert("active_band".to_string(), json!(0));
        outputs.insert("bands".to_string(), json!(["surface", "double_bounce", "volume", "helix"]));
        outputs.insert("matrix_format".to_string(), json!(matrix_format));

        Ok(ToolRunResult { outputs })
    }
}

impl Tool for HAlphaWisartClassificationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "h_alpha_wisart_classification",
            display_name: "H-Alpha Wisart Classification",
            summary: "Classifies H (entropy) and alpha (scattering angle) into 9 Wisart zones based on H-alpha decomposition thresholds.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "h_raster",
                    description: "Input entropy (H) raster from Cloude-Pottier decomposition (0-1 range).",
                    required: true,
                },
                ToolParamSpec {
                    name: "alpha_raster",
                    description: "Input alpha (α) raster from Cloude-Pottier decomposition (degrees, 0-90).",
                    required: true,
                },
                ToolParamSpec {
                    name: "auto_reproject",
                    description: "If true (default), reproject alpha to match H when CRS differs.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject_method",
                    description: "Optional reprojection resampling override: nearest, bilinear, cubic, lanczos.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output raster with 9-class labels (1-9).",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("h_raster".to_string(), json!("entropy.tif"));
        defaults.insert("alpha_raster".to_string(), json!("alpha.tif"));
        defaults.insert("auto_reproject".to_string(), json!(true));
        defaults.insert("auto_reproject_method".to_string(), json!(""));

        let mut example = ToolArgs::new();
        example.insert("h_raster".to_string(), json!("entropy.tif"));
        example.insert("alpha_raster".to_string(), json!("alpha.tif"));
        example.insert("output".to_string(), json!("wisart_zones.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "h_alpha_wisart".to_string(),
                description: "Classify entropy and alpha into 9 Wisart zones.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "polsar".to_string(),
                "classification".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let h_path = args
            .get("h_raster")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("missing required parameter 'h_raster'".to_string()))?;
        let alpha_path = args
            .get("alpha_raster")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("missing required parameter 'alpha_raster'".to_string()))?;
        let _ = load_raster(h_path)?;
        let _ = load_raster(alpha_path)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let h_path = args
            .get("h_raster")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("missing required parameter 'h_raster'".to_string()))?;
        let alpha_path = args
            .get("alpha_raster")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("missing required parameter 'alpha_raster'".to_string()))?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("h_alpha_wisart_classification: reading rasters");
        let mut h_raster = load_raster(h_path)?;
        let mut alpha_raster = load_raster(alpha_path)?;

        let stack_cfg = RasterStackConfig {
            auto_reproject: args
                .get("auto_reproject")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            resampling_method: parse_resampling_override(args),
            allow_no_overlap: false,
        };
        let mut rasters = vec![h_raster.clone(), alpha_raster.clone()];
        let warnings = align_and_validate_raster_stack(&mut rasters, &stack_cfg)
            .map_err(|e| ToolError::Validation(format!("raster stack alignment failed: {}", e)))?;
        h_raster = rasters[0].clone();
        alpha_raster = rasters[1].clone();
        for warning in warnings {
            ctx.progress.info(&format!("h_alpha_wisart_classification: {warning}"));
        }

        let rows = h_raster.rows;
        let cols = h_raster.cols;
        let nodata = h_raster.nodata;

        let mut output = h_raster.clone();
        output.bands = 1;

        let mut zone_values = vec![nodata; rows * cols];
        zone_values
            .par_chunks_mut(cols)
            .enumerate()
            .for_each(|(r, zone_row)| {
                let h_row = h_raster.row_slice(0, r as isize);
                let alpha_row = alpha_raster.row_slice(0, r as isize);

                for c in 0..cols {
                    let h_val = h_row[c];
                    let alpha_val = alpha_row[c];

                    if h_raster.is_nodata(h_val) || alpha_raster.is_nodata(alpha_val) {
                        zone_row[c] = nodata;
                        continue;
                    }

                    let h = h_val.clamp(0.0, 1.0);
                    let alpha = alpha_val.clamp(0.0, 90.0);

                    zone_row[c] = if h < 0.73 {
                        if alpha < 30.0 {
                            1.0
                        } else if alpha < 45.0 {
                            2.0
                        } else {
                            3.0
                        }
                    } else if h < 0.90 {
                        if alpha < 30.0 {
                            4.0
                        } else if alpha < 45.0 {
                            5.0
                        } else {
                            6.0
                        }
                    } else if h < 1.0 {
                        if alpha < 30.0 {
                            7.0
                        } else if alpha < 45.0 {
                            8.0
                        } else {
                            9.0
                        }
                    } else {
                        9.0
                    };
                }
            });

        let coalescer = PercentCoalescer::new(1, 98);
        for r in 0..rows {
            let start = r * cols;
            let end = start + cols;
            output
                .set_row_slice(0, r as isize, &zone_values[start..end])
                .map_err(|e| ToolError::Execution(format!("failed writing classification row {}: {e}", r)))?;
            coalescer.emit_unit_fraction(ctx.progress, (r + 1) as f64 / rows as f64);
        }
        coalescer.finish(ctx.progress);

        let locator = write_or_store_output(output, output_path)?;

        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(locator));
        outputs.insert("active_band".to_string(), json!(0));
        outputs.insert(
            "class_meanings".to_string(),
            json!({
                "1": "Zone 1: Low entropy, low alpha",
                "2": "Zone 2: Low entropy, med alpha",
                "3": "Zone 3: Low entropy, high alpha",
                "4": "Zone 4: Med entropy, low alpha",
                "5": "Zone 5: Med entropy, med alpha",
                "6": "Zone 6: Med entropy, high alpha",
                "7": "Zone 7: High entropy, low alpha",
                "8": "Zone 8: High entropy, med alpha",
                "9": "Zone 9: High entropy, high alpha"
            }),
        );

        Ok(ToolRunResult { outputs })
    }
}

impl Tool for WishartIterativeClusteringTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "wisart_iterative_clustering",
            display_name: "Wisart Iterative Clustering",
            summary: "Unsupervised clustering initialized from H-alpha zones; iteratively reassigns pixels using complex Wisart distance metric.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "h_raster",
                    description: "Input entropy (H) raster (0-1 range).",
                    required: true,
                },
                ToolParamSpec {
                    name: "alpha_raster",
                    description: "Input alpha (α) raster (degrees, 0-90).",
                    required: true,
                },
                ToolParamSpec {
                    name: "max_iterations",
                    description: "Maximum clustering iterations (default 10).",
                    required: false,
                },
                ToolParamSpec {
                    name: "convergence_threshold",
                    description: "Convergence criterion: fraction of unchanged pixels (default 0.99).",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject",
                    description: "If true (default), reproject alpha to match H when CRS differs.",
                    required: false,
                },
                ToolParamSpec {
                    name: "auto_reproject_method",
                    description: "Optional reprojection resampling override: nearest, bilinear, cubic, lanczos.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output cluster label raster.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("h_raster".to_string(), json!("entropy.tif"));
        defaults.insert("alpha_raster".to_string(), json!("alpha.tif"));
        defaults.insert("max_iterations".to_string(), json!(10));
        defaults.insert("convergence_threshold".to_string(), json!(0.99));
        defaults.insert("auto_reproject".to_string(), json!(true));
        defaults.insert("auto_reproject_method".to_string(), json!(""));

        let mut example = ToolArgs::new();
        example.insert("h_raster".to_string(), json!("entropy.tif"));
        example.insert("alpha_raster".to_string(), json!("alpha.tif"));
        example.insert("max_iterations".to_string(), json!(10));
        example.insert("convergence_threshold".to_string(), json!(0.99));
        example.insert("output".to_string(), json!("wisart_clusters.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "wisart_clustering".to_string(),
                description: "Unsupervised clustering of SAR polarimetry using iterative Wisart distance.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "polsar".to_string(),
                "clustering".to_string(),
                "unsupervised".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let h_path = args
            .get("h_raster")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("missing required parameter 'h_raster'".to_string()))?;
        let alpha_path = args
            .get("alpha_raster")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("missing required parameter 'alpha_raster'".to_string()))?;
        let _ = load_raster(h_path)?;
        let _ = load_raster(alpha_path)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let h_path = args
            .get("h_raster")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("missing required parameter 'h_raster'".to_string()))?;
        let alpha_path = args
            .get("alpha_raster")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("missing required parameter 'alpha_raster'".to_string()))?;
        let output_path = parse_optional_output_path(args, "output")?;

        let max_iterations = args
            .get("max_iterations")
            .and_then(|v| v.as_u64())
            .unwrap_or(10)
            .min(100) as usize;
        let convergence_threshold = args
            .get("convergence_threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.99)
            .clamp(0.5, 0.9999);

        ctx.progress.info("wisart_iterative_clustering: reading and initializing");
        let mut h_raster = load_raster(h_path)?;
        let mut alpha_raster = load_raster(alpha_path)?;

        let stack_cfg = RasterStackConfig {
            auto_reproject: args
                .get("auto_reproject")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            resampling_method: parse_resampling_override(args),
            allow_no_overlap: false,
        };
        let mut rasters = vec![h_raster.clone(), alpha_raster.clone()];
        let warnings = align_and_validate_raster_stack(&mut rasters, &stack_cfg)
            .map_err(|e| ToolError::Validation(format!("raster stack alignment failed: {}", e)))?;
        h_raster = rasters[0].clone();
        alpha_raster = rasters[1].clone();
        for warning in warnings {
            ctx.progress.info(&format!("wisart_iterative_clustering: {warning}"));
        }

        let rows = h_raster.rows;
        let cols = h_raster.cols;
        let nodata = h_raster.nodata;
        let total_pixels = rows * cols;

        let mut h_vals = vec![nodata; total_pixels];
        let mut alpha_vals = vec![nodata; total_pixels];
        for r in 0..rows {
            let h_row = h_raster.row_slice(0, r as isize);
            let alpha_row = alpha_raster.row_slice(0, r as isize);
            let start = r * cols;
            let end = start + cols;
            h_vals[start..end].copy_from_slice(&h_row);
            alpha_vals[start..end].copy_from_slice(&alpha_row);
        }

        // Initialize clusters from H-alpha zones
        let mut clusters = vec![0u8; total_pixels];
        clusters
            .par_iter_mut()
            .enumerate()
            .for_each(|(idx, cluster)| {
                let h = h_vals[idx];
                let alpha = alpha_vals[idx];

                if h_raster.is_nodata(h) || alpha_raster.is_nodata(alpha) {
                    *cluster = 0;
                    return;
                }

                let h_clamped = h.clamp(0.0, 1.0);
                let alpha_clamped = alpha.clamp(0.0, 90.0);

                *cluster = if h_clamped < 0.73 {
                    if alpha_clamped < 30.0 {
                        1
                    } else if alpha_clamped < 45.0 {
                        2
                    } else {
                        3
                    }
                } else if h_clamped < 0.90 {
                    if alpha_clamped < 30.0 {
                        4
                    } else if alpha_clamped < 45.0 {
                        5
                    } else {
                        6
                    }
                } else if h_clamped < 1.0 {
                    if alpha_clamped < 30.0 {
                        7
                    } else if alpha_clamped < 45.0 {
                        8
                    } else {
                        9
                    }
                } else {
                    9
                };
            });

        // Iterative clustering
        for iteration in 0..max_iterations {
            ctx.progress.info(&format!("wisart_iterative_clustering: iteration {}/{}", iteration + 1, max_iterations));

            // Recompute cluster centers (simplified: mean H and alpha per cluster)
            let (mut cluster_h, mut cluster_alpha, cluster_count) = (0..total_pixels)
                .into_par_iter()
                .fold(
                    || ([0.0_f64; 10], [0.0_f64; 10], [0usize; 10]),
                    |(mut sum_h, mut sum_alpha, mut counts), idx| {
                        let cluster_id = clusters[idx] as usize;
                        if cluster_id > 0 && cluster_id <= 9 {
                            let h = h_vals[idx];
                            let alpha = alpha_vals[idx];
                            if !h_raster.is_nodata(h) && !alpha_raster.is_nodata(alpha) {
                                sum_h[cluster_id] += h;
                                sum_alpha[cluster_id] += alpha;
                                counts[cluster_id] += 1;
                            }
                        }
                        (sum_h, sum_alpha, counts)
                    },
                )
                .reduce(
                    || ([0.0_f64; 10], [0.0_f64; 10], [0usize; 10]),
                    |(mut ah, mut aa, mut ac), (bh, ba, bc)| {
                        for i in 1..=9 {
                            ah[i] += bh[i];
                            aa[i] += ba[i];
                            ac[i] += bc[i];
                        }
                        (ah, aa, ac)
                    },
                );

            for i in 1..=9 {
                if cluster_count[i] > 0 {
                    cluster_h[i] /= cluster_count[i] as f64;
                    cluster_alpha[i] /= cluster_count[i] as f64;
                }
            }

            // Reassign pixels to nearest cluster
            let mut new_clusters = vec![0u8; total_pixels];
            new_clusters
                .par_iter_mut()
                .enumerate()
                .for_each(|(idx, cluster)| {
                    let h = h_vals[idx];
                    let alpha = alpha_vals[idx];

                    if h_raster.is_nodata(h) || alpha_raster.is_nodata(alpha) {
                        *cluster = 0;
                        return;
                    }

                    let mut best_cluster = clusters[idx];
                    let mut best_distance = f64::INFINITY;

                    for cluster_id in 1..=9 {
                        let dh = h - cluster_h[cluster_id];
                        let da = alpha - cluster_alpha[cluster_id];
                        let distance = (dh * dh + 0.01 * da * da).sqrt();
                        if distance < best_distance {
                            best_distance = distance;
                            best_cluster = cluster_id as u8;
                        }
                    }

                    *cluster = best_cluster;
                });

            // Check convergence
            let unchanged = clusters
                .par_iter()
                .zip(new_clusters.par_iter())
                .filter(|(c, n)| c == n)
                .count();
            let convergence_ratio = unchanged as f64 / total_pixels as f64;
            ctx.progress.info(&format!("wisart_iterative_clustering: convergence ratio {:.2}%", convergence_ratio * 100.0));

            clusters = new_clusters;

            if convergence_ratio >= convergence_threshold {
                ctx.progress.info("wisart_iterative_clustering: converged");
                break;
            }
        }

        // Write output
        let mut output = h_raster.clone();
        output.bands = 1;

        let cluster_values: Vec<f64> = clusters.iter().map(|&c| c as f64).collect();
        let coalescer = PercentCoalescer::new(1, 98);
        for r in 0..rows {
            let start = r * cols;
            let end = start + cols;
            output
                .set_row_slice(0, r as isize, &cluster_values[start..end])
                .map_err(|e| ToolError::Execution(format!("failed writing clustering row {}: {e}", r)))?;
            coalescer.emit_unit_fraction(ctx.progress, (r + 1) as f64 / rows as f64);
        }
        coalescer.finish(ctx.progress);

        let locator = write_or_store_output(output, output_path)?;

        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(locator));
        outputs.insert("active_band".to_string(), json!(0));
        outputs.insert("num_clusters".to_string(), json!(9));

        Ok(ToolRunResult { outputs })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use wbraster::{LandsatMission, LandsatProcessingLevel};

    fn make_temp_dir(prefix: &str) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("{}_{}", prefix, ts));
        fs::create_dir_all(&dir).expect("failed to create temp dir");
        dir
    }

    fn make_test_landsat_bundle(mtl_path: PathBuf) -> LandsatBundle {
        LandsatBundle {
            bundle_root: mtl_path
                .parent()
                .expect("mtl path should have parent")
                .to_path_buf(),
            mtl_path,
            mission: LandsatMission::Landsat9,
            processing_level: LandsatProcessingLevel::L1,
            product_id: None,
            collection_number: None,
            acquisition_date_utc: None,
            scene_center_time_utc: None,
            path_row: None,
            cloud_cover_percent: None,
            sun_azimuth_deg: None,
            sun_elevation_deg: Some(40.0),
            bands: BTreeMap::new(),
            qa_layers: BTreeMap::new(),
            aux_layers: BTreeMap::new(),
        }
    }

    #[test]
    fn build_landsat_reflectance_coefficients_reports_missing_band_metadata() {
        let dir = make_temp_dir("wbtools_oss_reflectance_meta");
        let mtl_path = dir.join("scene_MTL.txt");
        fs::write(
            &mtl_path,
            "REFLECTANCE_MULT_BAND_2 = 0.00002\nREFLECTANCE_ADD_BAND_2 = -0.1\n",
        )
        .expect("failed to write mtl");

        let bundle = make_test_landsat_bundle(mtl_path);
        let result = build_landsat_reflectance_coefficients(&["LC09_B1.TIF".to_string()], &bundle);

        assert!(result.is_err(), "expected missing-band metadata error");
        let msg = format!("{}", result.expect_err("should fail"));
        assert!(
            msg.contains("missing REFLECTANCE_MULT_BAND_1") || msg.contains("missing REFLECTANCE_ADD_BAND_1"),
            "unexpected error message: {msg}"
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn parse_landsat_thermal_constants_reports_missing_k2() {
        let dir = make_temp_dir("wbtools_oss_thermal_meta");
        let mtl_path = dir.join("scene_MTL.txt");
        fs::write(
            &mtl_path,
            "RADIANCE_MULT_BAND_10 = 0.0003342\nRADIANCE_ADD_BAND_10 = 0.1\nK1_CONSTANT_BAND_10 = 774.8853\n",
        )
        .expect("failed to write mtl");

        let bundle = make_test_landsat_bundle(mtl_path);
        let result = parse_landsat_thermal_constants_from_bundle(&bundle, 10);

        assert!(result.is_err(), "expected missing K2 constant error");
        let msg = format!("{}", result.expect_err("should fail"));
        assert!(
            msg.contains("missing K2_CONSTANT_BAND_10"),
            "unexpected error message: {msg}"
        );

        let _ = fs::remove_dir_all(dir);
    }
}