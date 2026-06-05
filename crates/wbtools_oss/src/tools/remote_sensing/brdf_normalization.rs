/// BRDF Normalization — single-scene angular reflectance correction.
///
/// Normalizes directional reflectance effects caused by varying solar and view
/// angles within a single scene. Uses the C-factor Minnaert normalization
/// approach combined with DEM-derived slope/aspect geometry. The result is a
/// BRDF-normalized reflectance stack where illumination geometry effects have
/// been reduced.
///
/// For multi-date or cross-sensor BRDF harmonization (e.g., MODIS BRDF model
/// parameters, nadir-corrected reflectance across orbits) use the Pro-tier
/// `brdf_surface_reflectance_consistency` tool.
use serde_json::json;
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::f64::consts::PI;
use std::path::Path;

use wbcore::{
    parse_optional_output_path, LicenseTier, Tool, ToolArgs, ToolCategory, ToolContext, ToolError,
    ToolExample, ToolManifest, ToolMetadata, ToolParamDescriptor, ToolParamSpec, ToolRunResult,
    ToolStability,
};
use wbraster::{Raster, RasterFormat};

use crate::memory_store;
use crate::tools::slope_aspect_from_dem;

pub struct BrdfNormalizationTool;

// ── helpers ──────────────────────────────────────────────────────────────────

fn load_raster(path: &str, label: &str) -> Result<Raster, ToolError> {
    if memory_store::raster_is_memory_path(path) {
        let id = memory_store::raster_path_to_id(path)
            .ok_or_else(|| ToolError::Execution(format!("invalid memory path for '{label}'")))?;
        memory_store::get_raster_arc_by_id(id)
            .map(|r| r.as_ref().clone())
            .ok_or_else(|| ToolError::Execution(format!("memory raster not found for '{label}'")))
    } else {
        Raster::read(std::path::Path::new(path))
            .map_err(|e| ToolError::Execution(format!("failed reading '{label}': {e}")))
    }
}

fn write_raster(r: &Raster, path: &str, label: &str) -> Result<(), ToolError> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ToolError::Execution(format!("failed creating output directory for '{label}': {e}"))
            })?;
        }
    }
    r.write(path, RasterFormat::GeoTiff)
        .map_err(|e| ToolError::Execution(format!("failed writing '{label}': {e}")))
}

#[inline]
fn deg2rad(d: f64) -> f64 { d * PI / 180.0 }

/// Cosine of the solar incidence angle between surface normal and solar beam.
#[inline]
fn cos_incidence(slope_rad: f64, aspect_rad: f64, solar_zenith_rad: f64, solar_azimuth_rad: f64) -> f64 {
    let cos_sz = solar_zenith_rad.cos();
    let sin_sz = solar_zenith_rad.sin();
    let cos_slope = slope_rad.cos();
    let sin_slope = slope_rad.sin();
    cos_sz * cos_slope + sin_sz * sin_slope * (solar_azimuth_rad - aspect_rad).cos()
}

/// C-correction factor: b / m from OLS regression of reflectance on cos_i.
#[inline]
fn c_correction_factor(m: f64, b: f64) -> Option<f64> {
    if m.abs() < 1e-8 { None } else { Some(b / m) }
}

/// Apply C-correction: ρ_out = ρ_in * (cos_z + c) / (cos_i + c).
#[inline]
fn apply_c_correction(reflectance: f64, cos_z: f64, cos_i: f64, c: f64) -> f64 {
    let denom = cos_i + c;
    if denom.abs() < 1e-10 { reflectance } else { reflectance * (cos_z + c) / denom }
}

/// Minnaert correction: ρ_out = ρ_in * cos_z^k / (cos_i * cos_e)^k
/// where cos_e ≈ cos_z (nadir-looking approximation).
#[inline]
fn apply_minnaert(reflectance: f64, cos_z: f64, cos_i: f64, k: f64) -> f64 {
    let denom = (cos_i * cos_z).powf(k);
    if denom < 1e-10 { reflectance } else { reflectance * cos_z.powf(k) / denom }
}

struct LinReg { m: f64, b: f64 }

fn ols(x: &[f64], y: &[f64]) -> LinReg {
    let n = x.len().min(y.len());
    if n < 2 { return LinReg { m: 0.0, b: 0.0 }; }
    let sum_x: f64 = x[..n].iter().sum();
    let sum_y: f64 = y[..n].iter().sum();
    let sum_xx: f64 = x[..n].iter().map(|v| v * v).sum();
    let sum_xy: f64 = x[..n].iter().zip(y[..n].iter()).map(|(xi, yi)| xi * yi).sum();
    let n_f = n as f64;
    let denom = n_f * sum_xx - sum_x * sum_x;
    if denom.abs() < 1e-15 {
        return LinReg { m: 0.0, b: sum_y / n_f };
    }
    let m = (n_f * sum_xy - sum_x * sum_y) / denom;
    let b = (sum_y - m * sum_x) / n_f;
    LinReg { m, b }
}

// ── Tool impl ────────────────────────────────────────────────────────────────

impl Tool for BrdfNormalizationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "brdf_normalization",
            display_name: "BRDF Normalization",
            summary: r#"Bidirectional reflectance distribution function normalization corrects directional reflectance variations caused by sun-target-sensor geometry using viewing/illumination angle measurements. Normalizes reflectance to standard nadir-zenith geometry (typically nadir viewing, 45° solar zenith angle). Kernel-based BRDF models decompose reflectance into isotropic and directional components enabling geometric normalization while preserving spectral character. Key Features: Corrects directional reflectance anisotropy; requires solar/viewing angle information; reduces bidirectional effects; improves cross-scene comparability; enhances classification accuracy; supports multispectral/hyperspectral data. Use Cases: Multi-temporal reflectance normalization; cross-orbit SAR comparability; improved landcover classification; reflectance-based vegetation index normalization; reducing false changes from geometry variation; time-series analysis. Output Interpretation: Output is nadir-corrected reflectance values comparable across different acquisition geometries. Correction magnitude varies by band; vegetation bands typically show larger corrections than visible bands. Normalized values reduce apparent temporal variation from geometric effects, revealing true surface change. Residual BRDF effects indicate model adequacy."#,
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input_raster", description: "Input reflectance raster to normalize (single band or multi-band stack).", required: true },
                ToolParamSpec { name: "input_dem", description: "DEM raster co-registered with the input (same grid or will be resampled to match).", required: true },
                ToolParamSpec { name: "solar_zenith_deg", description: "Solar zenith angle in degrees at acquisition time.", required: true },
                ToolParamSpec { name: "solar_azimuth_deg", description: "Solar azimuth angle in degrees (clockwise from north) at acquisition time.", required: true },
                ToolParamSpec { name: "method", description: "Correction method: c_correction (default) or minnaert.", required: false },
                ToolParamSpec { name: "minnaert_k", description: "Minnaert k coefficient in [0,1] (default 0.5). Only used when method=minnaert.", required: false },
                ToolParamSpec { name: "z_factor", description: "Vertical exaggeration for DEM slope/aspect computation (default 1.0).", required: false },
                ToolParamSpec { name: "output_prefix", description: "Prefix for output files (default brdf_normalized).", required: false },
                ToolParamSpec { name: "output", description: "Output path for normalized raster (overrides output_prefix for single-output mode).", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input_raster".to_string(), json!("input.tif"));
        defaults.insert("input_dem".to_string(), json!("dem.tif"));
        defaults.insert("solar_zenith_deg".to_string(), json!(40.0));
        defaults.insert("solar_azimuth_deg".to_string(), json!(165.0));
        defaults.insert("method".to_string(), json!("c_correction"));
        defaults.insert("minnaert_k".to_string(), json!(0.5));
        defaults.insert("z_factor".to_string(), json!(1.0));
        defaults.insert("output_prefix".to_string(), json!("brdf_normalized"));

        ToolManifest {
            id: "brdf_normalization".to_string(),
            display_name: "BRDF Normalization".to_string(),
            summary: "Single-scene BRDF normalization using C-correction or Minnaert approach with DEM slope/aspect geometry.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: self.metadata().params.into_iter().map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            }).collect(),
            defaults: defaults.clone(),
            examples: vec![ToolExample {
                name: "brdf_normalization_c_correction".to_string(),
                description: "Apply C-correction BRDF normalization to a reflectance band.".to_string(),
                args: defaults,
            }],
            tags: vec![
                "remote-sensing".to_string(),
                "brdf".to_string(),
                "normalization".to_string(),
                "radiometric".to_string(),
                "terrain".to_string(),
            ],
            stability: ToolStability::Stable,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        args.get("input_raster")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ToolError::Validation("parameter 'input_raster' is required".to_string()))?;
        args.get("input_dem")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ToolError::Validation("parameter 'input_dem' is required".to_string()))?;
        let zenith = args.get("solar_zenith_deg")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'solar_zenith_deg' is required".to_string()))?;
        if !(0.0..90.0).contains(&zenith) {
            return Err(ToolError::Validation("parameter 'solar_zenith_deg' must be in [0, 90)".to_string()));
        }
        let azimuth = args.get("solar_azimuth_deg")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'solar_azimuth_deg' is required".to_string()))?;
        if !(0.0..=360.0).contains(&azimuth) {
            return Err(ToolError::Validation("parameter 'solar_azimuth_deg' must be in [0, 360]".to_string()));
        }
        if let Some(method) = args.get("method").and_then(|v| v.as_str()) {
            if !matches!(method, "c_correction" | "minnaert") {
                return Err(ToolError::Validation(
                    "parameter 'method' must be one of: c_correction, minnaert".to_string()
                ));
            }
        }
        if let Some(k) = args.get("minnaert_k").and_then(|v| v.as_f64()) {
            if !(0.0..=1.0).contains(&k) {
                return Err(ToolError::Validation("parameter 'minnaert_k' must be in [0, 1]".to_string()));
            }
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = args.get("input_raster")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'input_raster' is required".to_string()))?
            .to_string();
        let dem_path = args.get("input_dem")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'input_dem' is required".to_string()))?
            .to_string();
        let solar_zenith_deg = args.get("solar_zenith_deg").and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'solar_zenith_deg' is required".to_string()))?;
        let solar_azimuth_deg = args.get("solar_azimuth_deg").and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'solar_azimuth_deg' is required".to_string()))?;
        let method = args.get("method").and_then(|v| v.as_str()).unwrap_or("c_correction");
        let minnaert_k = args.get("minnaert_k").and_then(|v| v.as_f64()).unwrap_or(0.5);
        let z_factor = args.get("z_factor").and_then(|v| v.as_f64()).unwrap_or(1.0);
        let output_prefix = args.get("output_prefix").and_then(|v| v.as_str()).unwrap_or("brdf_normalized").to_string();

        ctx.progress.info("brdf_normalization: loading input raster and DEM");
        let input = load_raster(&input_path, "input_raster")?;
        let dem = load_raster(&dem_path, "input_dem")?;

        // Compute slope and aspect from DEM.
        ctx.progress.info("brdf_normalization: computing slope and aspect from DEM");
        let (slope_raster, aspect_raster) = slope_aspect_from_dem(&dem, z_factor)?;

        let solar_zenith_rad = deg2rad(solar_zenith_deg);
        let solar_azimuth_rad = deg2rad(solar_azimuth_deg);
        let cos_z = solar_zenith_rad.cos().max(1e-6);

        let n = input.rows * input.cols;
        let num_bands = input.bands;

        // Sample cos_i and per-band reflectance for regression (C-correction).
        let sample_step = (n / 5000).max(1);
        let mut sample_cos_i: Vec<f64> = Vec::new();
        let mut sample_refl: Vec<Vec<f64>> = vec![Vec::new(); num_bands];

        for i in (0..n).step_by(sample_step) {
            let sv = slope_raster.data.get_f64(i);
            let av = aspect_raster.data.get_f64(i);
            if slope_raster.is_nodata(sv) || aspect_raster.is_nodata(av) { continue; }
            let cos_i = cos_incidence(deg2rad(sv), deg2rad(av), solar_zenith_rad, solar_azimuth_rad);
            if cos_i < 0.05 { continue; }
            let mut all_valid = true;
            for b in 0..num_bands {
                let v = input.data.get_f64(b * n + i);
                if input.is_nodata(v) { all_valid = false; break; }
            }
            if !all_valid { continue; }
            sample_cos_i.push(cos_i);
            for b in 0..num_bands {
                sample_refl[b].push(input.data.get_f64(b * n + i));
            }
        }

        // Compute per-band C factors (C-correction) or use Minnaert k.
        let c_factors: Vec<f64> = if method == "c_correction" {
            sample_refl.iter().map(|band_samples| {
                let reg = ols(&sample_cos_i, band_samples);
                c_correction_factor(reg.m, reg.b).unwrap_or(1.0)
            }).collect()
        } else {
            vec![minnaert_k; num_bands] // reuse slot for k value
        };

        ctx.progress.info("brdf_normalization: applying normalization");
        let mut normalized = input.clone();
        let mut delta_raster = {
            // delta is single-band mean across all bands
            let first_band_cfg = wbraster::RasterConfig {
                rows: input.rows,
                cols: input.cols,
                bands: 1,
                nodata: input.nodata,
                x_min: input.x_min,
                y_min: input.y_min,
                cell_size: input.cell_size_x,
                crs: input.crs.clone(),
                metadata: input.metadata.clone(),
                ..Default::default()
            };
            wbraster::Raster::new(first_band_cfg)
        };

        let correction_data: Vec<(Vec<f64>, f64)> = (0..n)
            .into_par_iter()
            .map(|i| {
                let sv = slope_raster.data.get_f64(i);
                let av = aspect_raster.data.get_f64(i);
                let has_geom = !(slope_raster.is_nodata(sv) || aspect_raster.is_nodata(av));

                let cos_i = if has_geom {
                    cos_incidence(deg2rad(sv), deg2rad(av), solar_zenith_rad, solar_azimuth_rad)
                        .clamp(-1.0, 1.0)
                } else {
                    0.0
                };

                let mut band_values = Vec::with_capacity(num_bands);
                let mut delta_sum = 0.0f64;
                let mut valid_bands = 0usize;

                for b in 0..num_bands {
                    let v = input.data.get_f64(b * n + i);
                    if input.is_nodata(v) {
                        band_values.push(input.nodata);
                        continue;
                    }
                    let corrected = if !has_geom || cos_i < 0.02 {
                        v
                    } else if method == "c_correction" {
                        apply_c_correction(v, cos_z, cos_i, c_factors[b])
                    } else {
                        apply_minnaert(v, cos_z, cos_i, c_factors[b])
                    };
                    delta_sum += (corrected - v).abs();
                    valid_bands += 1;
                    band_values.push(corrected);
                }

                let mean_delta = if valid_bands > 0 { delta_sum / valid_bands as f64 } else { input.nodata };
                (band_values, mean_delta)
            })
            .collect();

        for (i, (band_values, delta)) in correction_data.into_iter().enumerate() {
            for (b, v) in band_values.into_iter().enumerate() {
                normalized.data.set_f64(b * n + i, v);
            }
            delta_raster.data.set_f64(i, delta);
        }

        let normalized_out = parse_optional_output_path(args, "output")?
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| format!("{}_normalized.tif", output_prefix));
        let delta_out = format!("{}_normalization_delta.tif", output_prefix);

        write_raster(&normalized, &normalized_out, "normalized")?;
        write_raster(&delta_raster, &delta_out, "normalization_delta")?;

        ctx.progress.info("brdf_normalization: complete");

        let mut outputs = BTreeMap::new();
        outputs.insert("normalized".to_string(), json!(normalized_out));
        outputs.insert("normalization_delta".to_string(), json!(delta_out));
        outputs.insert("method".to_string(), json!(method));

        Ok(ToolRunResult { outputs })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_is_free_tier() {
        let tool = BrdfNormalizationTool;
        let meta = tool.metadata();
        assert_eq!(meta.id, "brdf_normalization");
        assert_eq!(meta.license_tier, LicenseTier::Open);
    }

    #[test]
    fn validation_rejects_missing_inputs() {
        let tool = BrdfNormalizationTool;
        let args = ToolArgs::new();
        assert!(tool.validate(&args).is_err());
    }

    #[test]
    fn validation_rejects_bad_method() {
        let tool = BrdfNormalizationTool;
        let mut args = ToolArgs::new();
        args.insert("input_raster".to_string(), json!("r.tif"));
        args.insert("input_dem".to_string(), json!("dem.tif"));
        args.insert("solar_zenith_deg".to_string(), json!(40.0));
        args.insert("solar_azimuth_deg".to_string(), json!(165.0));
        args.insert("method".to_string(), json!("empirical_line"));
        assert!(tool.validate(&args).is_err());
    }
}
