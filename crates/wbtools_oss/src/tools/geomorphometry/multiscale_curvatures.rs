use std::collections::BTreeMap;

use rayon::prelude::*;
use serde_json::json;
use wbprojection::{Crs, EpsgIdentifyPolicy, identify_epsg_from_wkt_with_policy};
use wbcore::{PercentCoalescer, 
    parse_optional_output_path, parse_raster_path_arg, LicenseTier, Tool, ToolArgs, ToolCategory,
    ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata, ToolParamDescriptor,
    ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::{memory_store, Raster, RasterConfig, RasterFormat};

pub struct MultiscaleCurvaturesTool;

#[derive(Clone, Copy)]
enum CurvatureType {
    Accumulation,
    Curvedness,
    Difference,
    Gaussian,
    GeneratingFunction,
    HorizontalExcess,
    Maximal,
    Mean,
    Minimal,
    Plan,
    Profile,
    Ring,
    Rotor,
    ShapeIndex,
    Tangential,
    Total,
    Unsphericity,
    VerticalExcess,
}

impl CurvatureType {
    fn from_str(s: &str) -> Self {
        let v = s.to_ascii_lowercase();
        if v.contains("accum") {
            Self::Accumulation
        } else if v.contains("curvedness") {
            Self::Curvedness
        } else if v.contains("diff") {
            Self::Difference
        } else if v.contains("gaussian") {
            Self::Gaussian
        } else if v.contains("generating") {
            Self::GeneratingFunction
        } else if v.contains("horizontal") {
            Self::HorizontalExcess
        } else if v.contains("maxi") {
            Self::Maximal
        } else if v.contains("mean") {
            Self::Mean
        } else if v.contains("mini") {
            Self::Minimal
        } else if v.contains("plan") {
            Self::Plan
        } else if v.contains("profile") {
            Self::Profile
        } else if v.contains("ring") {
            Self::Ring
        } else if v.contains("rot") {
            Self::Rotor
        } else if v.contains("shape") {
            Self::ShapeIndex
        } else if v.contains("tang") {
            Self::Tangential
        } else if v.contains("total") {
            Self::Total
        } else if v.contains("unspher") {
            Self::Unsphericity
        } else if v.contains("vertical") {
            Self::VerticalExcess
        } else {
            Self::Profile
        }
    }
}

#[derive(Clone, Copy)]
struct MultiCfg {
    curv_type: CurvatureType,
    min_scale: isize,
    step: isize,
    num_steps: isize,
    step_nonlinearity: f64,
    log_transform: bool,
    standardize: bool,
}

impl MultiscaleCurvaturesTool {
    fn parse_input(args: &ToolArgs) -> Result<String, ToolError> {
        parse_raster_path_arg(args, "input").or_else(|_| parse_raster_path_arg(args, "dem"))
    }

    fn parse_output_mag(args: &ToolArgs) -> Result<Option<std::path::PathBuf>, ToolError> {
        if args.get("output").is_some() {
            parse_optional_output_path(args, "output")
        } else {
            parse_optional_output_path(args, "out_mag")
        }
    }

    fn parse_output_scale(args: &ToolArgs) -> Result<Option<std::path::PathBuf>, ToolError> {
        parse_optional_output_path(args, "out_scale")
    }

    fn parse_config(args: &ToolArgs) -> MultiCfg {
        let curv_type = args
            .get("curv_type")
            .and_then(|v| v.as_str())
            .map(CurvatureType::from_str)
            .unwrap_or(CurvatureType::Profile);
        let min_scale = args
            .get("min_scale")
            .and_then(|v| v.as_i64())
            .unwrap_or(1)
            .max(0) as isize;
        let step = args
            .get("step")
            .and_then(|v| v.as_i64())
            .unwrap_or(1)
            .max(1) as isize;
        let num_steps = args
            .get("num_steps")
            .and_then(|v| v.as_i64())
            .unwrap_or(10)
            .max(1) as isize;
        let step_nonlinearity = args
            .get("step_nonlinearity")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0)
            .clamp(1.0, 4.0);
        let log_transform = args
            .get("log_transform")
            .or_else(|| args.get("log"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let standardize = args
            .get("standardize")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        MultiCfg {
            curv_type,
            min_scale,
            step,
            num_steps,
            step_nonlinearity,
            log_transform,
            standardize,
        }
    }

    fn load_raster(path: &str) -> Result<Raster, ToolError> {
        if memory_store::raster_is_memory_path(path) {
            let id = memory_store::raster_path_to_id(path).ok_or_else(|| {
                ToolError::Validation("parameter 'input' has malformed in-memory path".to_string())
            })?;
            return memory_store::get_raster_by_id(id).ok_or_else(|| {
                ToolError::Validation(format!(
                    "parameter 'input' references unknown in-memory raster id '{}'",
                    id
                ))
            });
        }
        Raster::read(path).map_err(|e| ToolError::Execution(format!("failed reading input raster: {e}")))
    }

    fn log_multiplier(res: f64) -> f64 {
        match res {
            x if (0.0..1.0).contains(&x) => 10f64.powi(2),
            x if (1.0..10.0).contains(&x) => 10f64.powi(3),
            x if (10.0..100.0).contains(&x) => 10f64.powi(4),
            x if (100.0..1000.0).contains(&x) => 10f64.powi(5),
            x if (1000.0..5000.0).contains(&x) => 10f64.powi(6),
            x if (5000.0..10000.0).contains(&x) => 10f64.powi(7),
            x if (10000.0..75000.0).contains(&x) => 10f64.powi(8),
            _ => 10f64.powi(9),
        }
    }

    fn raster_is_geographic(input: &Raster) -> bool {
        let epsg = input.crs.epsg.or_else(|| {
            input
                .crs
                .wkt
                .as_deref()
                .and_then(|w| identify_epsg_from_wkt_with_policy(w, EpsgIdentifyPolicy::Lenient))
        });

        if let Some(code) = epsg {
            if let Ok(crs) = Crs::from_epsg(code) {
                return crs.is_geographic();
            }
        }
        false
    }

    #[inline]
    fn haversine_distance_m(lat1_deg: f64, lon1_deg: f64, lat2_deg: f64, lon2_deg: f64) -> f64 {
        let r = 6_371_008.8_f64;
        let lat1 = lat1_deg.to_radians();
        let lon1 = lon1_deg.to_radians();
        let lat2 = lat2_deg.to_radians();
        let lon2 = lon2_deg.to_radians();
        let dlat = lat2 - lat1;
        let dlon = lon2 - lon1;
        let a = (dlat / 2.0).sin().powi(2)
            + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        r * c
    }

    fn write_or_store_output(
        output: Raster,
        output_path: Option<std::path::PathBuf>,
    ) -> Result<String, ToolError> {
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

    fn radius_for_step(cfg: MultiCfg, s: isize) -> isize {
        cfg.min_scale + ((cfg.step as f64 * s as f64).powf(cfg.step_nonlinearity)).floor() as isize
    }

    fn build_gaussian_kernel_1d(radius: isize, sigma: f64) -> Vec<f64> {
        if radius <= 0 || sigma <= f64::EPSILON {
            return vec![1.0];
        }
        let mut k = Vec::with_capacity((radius * 2 + 1) as usize);
        let mut sum = 0.0;
        let two_sigma2 = 2.0 * sigma * sigma;
        for i in -radius..=radius {
            let w = (-(i * i) as f64 / two_sigma2).exp();
            k.push(w);
            sum += w;
        }
        if sum > 0.0 {
            for w in &mut k {
                *w /= sum;
            }
        }
        k
    }

    fn gaussian_blur_band(input: &Raster, band: isize, radius: isize, nodata: f64) -> Vec<f64> {
        let rows = input.rows as isize;
        let cols = input.cols as isize;
        if radius <= 0 {
            let mut out = vec![nodata; (rows * cols) as usize];
            for r in 0..rows {
                for c in 0..cols {
                    out[(r * cols + c) as usize] = input.get(band, r, c);
                }
            }
            return out;
        }

        let sigma = (radius as f64 + 0.5) / 3.0;
        let kernel = Self::build_gaussian_kernel_1d(radius, sigma);

        let mut tmp = vec![nodata; (rows * cols) as usize];
        for r in 0..rows {
            for c in 0..cols {
                let z = input.get(band, r, c);
                if input.is_nodata(z) {
                    continue;
                }
                let mut wsum = 0.0;
                let mut ssum = 0.0;
                for (k, w) in (-radius..=radius).zip(kernel.iter().copied()) {
                    let v = input.get(band, r, c + k);
                    if !input.is_nodata(v) {
                        wsum += w;
                        ssum += w * v;
                    }
                }
                if wsum > 0.0 {
                    tmp[(r * cols + c) as usize] = ssum / wsum;
                }
            }
        }

        let mut out = vec![nodata; (rows * cols) as usize];
        for r in 0..rows {
            for c in 0..cols {
                let z = tmp[(r * cols + c) as usize];
                if z == nodata {
                    continue;
                }
                let mut wsum = 0.0;
                let mut ssum = 0.0;
                for (k, w) in (-radius..=radius).zip(kernel.iter().copied()) {
                    let rr = (r + k).clamp(0, rows - 1);
                    let v = tmp[(rr * cols + c) as usize];
                    if v != nodata {
                        wsum += w;
                        ssum += w * v;
                    }
                }
                if wsum > 0.0 {
                    out[(r * cols + c) as usize] = ssum / wsum;
                }
            }
        }

        out
    }

    fn pick_or_center(smoothed: &[f64], rows: isize, cols: isize, r: isize, c: isize, dr: isize, dc: isize) -> f64 {
        let rr = (r + dr).clamp(0, rows - 1);
        let cc = (c + dc).clamp(0, cols - 1);
        let center = smoothed[(r * cols + c) as usize];
        let v = smoothed[(rr * cols + cc) as usize];
        if v.is_finite() { v } else { center }
    }

    fn finite_or_zero(v: f64) -> f64 {
        if v.is_finite() { v } else { 0.0 }
    }

    fn compute_curvature_value_projected(
        curv_type: CurvatureType,
        smoothed: &[f64],
        rows: isize,
        cols: isize,
        r0: isize,
        c0: isize,
        res: f64,
    ) -> f64 {
        let center = smoothed[(r0 * cols + c0) as usize];
        if !center.is_finite() {
            return f64::NAN;
        }

        let mut z = [0.0f64; 25];
        let mut i = 0usize;
        for rr in -2..=2 {
            for cc in -2..=2 {
                let v = Self::pick_or_center(smoothed, rows, cols, r0, c0, rr, cc);
                z[i] = if v.is_finite() { v } else { center };
                i += 1;
            }
        }

        let r = 1.0 / (35.0 * res * res)
            * (2.0 * (z[0] + z[4] + z[5] + z[9] + z[10] + z[14] + z[15] + z[19] + z[20] + z[24])
                - 2.0 * (z[2] + z[7] + z[12] + z[17] + z[22])
                - z[1] - z[3] - z[6] - z[8] - z[11] - z[13] - z[16] - z[18] - z[21] - z[23]);

        let t = 1.0 / (35.0 * res * res)
            * (2.0 * (z[0] + z[1] + z[2] + z[3] + z[4] + z[20] + z[21] + z[22] + z[23] + z[24])
                - 2.0 * (z[10] + z[11] + z[12] + z[13] + z[14])
                - z[5] - z[6] - z[7] - z[8] - z[9] - z[15] - z[16] - z[17] - z[18] - z[19]);

        let s = 1.0 / (100.0 * res * res)
            * (z[8] + z[16] - z[6] - z[18] + 4.0 * (z[4] + z[20] - z[0] - z[24])
                + 2.0 * (z[3] + z[9] + z[15] + z[21] - z[1] - z[5] - z[19] - z[23]));

        let p = 1.0 / (420.0 * res)
            * (44.0 * (z[3] + z[23] - z[1] - z[21])
                + 31.0 * (z[0] + z[20] - z[4] - z[24] + 2.0 * (z[8] + z[18] - z[6] - z[16]))
                + 17.0 * (z[14] - z[10] + 4.0 * (z[13] - z[11]))
                + 5.0 * (z[9] + z[19] - z[5] - z[15]));

        let q = 1.0 / (420.0 * res)
            * (44.0 * (z[5] + z[9] - z[15] - z[19])
                + 31.0 * (z[20] + z[24] - z[0] - z[4] + 2.0 * (z[6] + z[8] - z[16] - z[18]))
                + 17.0 * (z[2] - z[22] + 4.0 * (z[7] - z[17]))
                + 5.0 * (z[1] + z[3] - z[21] - z[23]));

        let h = 1.0 / (10.0 * res.powi(3))
            * (z[0] + z[1] + z[2] + z[3] + z[4] - z[20] - z[21] - z[22] - z[23] - z[24]
                + 2.0 * (z[15] + z[16] + z[17] + z[18] + z[19] - z[5] - z[6] - z[7] - z[8] - z[9]));

        let g = 1.0 / (10.0 * res.powi(3))
            * (z[4] + z[9] + z[14] + z[19] + z[24] - z[0] - z[5] - z[10] - z[15] - z[20]
                + 2.0 * (z[1] + z[6] + z[11] + z[16] + z[21] - z[3] - z[8] - z[13] - z[18] - z[23]));

        let m = 1.0 / (70.0 * res.powi(3))
            * (z[6] + z[16] - z[8] - z[18] + 4.0 * (z[4] + z[10] + z[24] - z[0] - z[14] - z[20])
                + 2.0 * (z[3] + z[5] + z[11] + z[15] + z[23] - z[1] - z[9] - z[13] - z[19] - z[21]));

        let k = 1.0 / (70.0 * res.powi(3))
            * (z[16] + z[18] - z[6] - z[8] + 4.0 * (z[0] + z[4] + z[22] - z[2] - z[20] - z[24])
                + 2.0 * (z[5] + z[9] + z[17] + z[21] + z[23] - z[1] - z[3] - z[7] - z[15] - z[19]));

        let w = 1.0 + p * p + q * q;
        let g2 = p * p + q * q;

        let mean_curv = Self::finite_or_zero(-((1.0 + q * q) * r - 2.0 * p * q * s + (1.0 + p * p) * t)
            / (2.0 * w.powf(1.5)));
        let gaussian_curv = Self::finite_or_zero((r * t - s * s) / w.powi(2));
        let disc = (mean_curv * mean_curv - gaussian_curv).max(0.0);
        let sqrt_disc = disc.sqrt();
        let minimal_curv = Self::finite_or_zero(mean_curv - sqrt_disc);
        let maximal_curv = Self::finite_or_zero(mean_curv + sqrt_disc);
        let diff_curv = if g2 <= f64::EPSILON {
            0.0
        } else {
            Self::finite_or_zero(
                (q * q * r - 2.0 * p * q * s + p * p * t) / (g2 * w.sqrt())
                    - ((1.0 + q * q) * r - 2.0 * p * q * s + (1.0 + p * p) * t)
                        / (2.0 * w.powf(1.5)),
            )
        };

        let curvedness = Self::finite_or_zero(((minimal_curv * minimal_curv + maximal_curv * maximal_curv) / 2.0).sqrt());
        let shape_index = {
            let denom = maximal_curv - minimal_curv;
            if denom.abs() <= f64::EPSILON {
                0.0
            } else {
                Self::finite_or_zero(2.0 / std::f64::consts::PI * ((maximal_curv + minimal_curv) / denom).atan())
            }
        };
        let unsphericity = Self::finite_or_zero(sqrt_disc);
        let vertical_excess = Self::finite_or_zero(unsphericity + diff_curv);
        let accumulation = Self::finite_or_zero(mean_curv * mean_curv - diff_curv * diff_curv);
        let rotor = if g2 <= f64::EPSILON {
            0.0
        } else {
            Self::finite_or_zero(((p * p - q * q) * s - p * q * (r - t)) / g2.powi(3).sqrt())
        };
        let horizontal_curv = if g2 <= f64::EPSILON {
            0.0
        } else {
            Self::finite_or_zero((q * q * r - 2.0 * p * q * s + p * p * t) / (g2 * w.sqrt()))
        };
        let generating_fn = if g2 <= f64::EPSILON {
            0.0
        } else {
            Self::finite_or_zero(
                (q.powi(3) * g - 3.0 * p * q * q * k + 3.0 * p * p * q * m - p.powi(3) * h)
                    / (g2.powi(3) * w).sqrt()
                    - horizontal_curv * rotor * (2.0 + 3.0 * g2) / w,
            )
        };
        let plan_curv = if g2 <= f64::EPSILON {
            0.0
        } else {
            Self::finite_or_zero(-(q * q * r - 2.0 * p * q * s + p * p * t) / g2.powi(3).sqrt())
        };
        let profile_curv = if g2 <= f64::EPSILON {
            0.0
        } else {
            Self::finite_or_zero(-(p * p * r + 2.0 * p * q * s + q * q * t) / (g2 * w.powf(1.5)))
        };
        let ring_curv = if g2 <= f64::EPSILON {
            0.0
        } else {
            Self::finite_or_zero((((p * p - q * q) * s - p * q * (r - t)) / (g2 * w)).powi(2))
        };
        let tan_curv = if g2 <= f64::EPSILON {
            0.0
        } else {
            Self::finite_or_zero(-(q * q * r - 2.0 * p * q * s + p * p * t) / (g2 * w.sqrt()))
        };
        // Keep legacy multiscale definition (plugin parity).
        let total_curv = Self::finite_or_zero(r * r + 2.0 * s * s + t * t);

        match curv_type {
            CurvatureType::Accumulation => accumulation,
            CurvatureType::Curvedness => curvedness,
            CurvatureType::Difference => diff_curv,
            CurvatureType::Gaussian => gaussian_curv,
            CurvatureType::GeneratingFunction => generating_fn,
            CurvatureType::HorizontalExcess => horizontal_curv,
            CurvatureType::Maximal => maximal_curv,
            CurvatureType::Mean => mean_curv,
            CurvatureType::Minimal => minimal_curv,
            CurvatureType::Plan => plan_curv,
            CurvatureType::Profile => profile_curv,
            CurvatureType::Ring => ring_curv,
            CurvatureType::Rotor => rotor,
            CurvatureType::ShapeIndex => shape_index,
            CurvatureType::Tangential => tan_curv,
            CurvatureType::Total => total_curv,
            CurvatureType::Unsphericity => unsphericity,
            CurvatureType::VerticalExcess => vertical_excess,
        }
    }

    fn compute_curvature_value_geographic(
        curv_type: CurvatureType,
        smoothed: &[f64],
        input: &Raster,
        rows: isize,
        cols: isize,
        r0: isize,
        c0: isize,
    ) -> f64 {
        let center = smoothed[(r0 * cols + c0) as usize];
        if !center.is_finite() {
            return f64::NAN;
        }

        let mut z = [0.0f64; 9];
        let mut i = 0usize;
        for rr in -1..=1 {
            for cc in -1..=1 {
                let v = Self::pick_or_center(smoothed, rows, cols, r0, c0, rr, cc);
                z[i] = if v.is_finite() { v } else { center };
                i += 1;
            }
        }

        let phi1 = input.row_center_y(r0);
        let lambda1 = input.col_center_x(c0);
        let b = Self::haversine_distance_m(phi1, lambda1, phi1, input.col_center_x(c0 - 1)).max(f64::EPSILON);
        let d = Self::haversine_distance_m(phi1, lambda1, input.row_center_y(r0 + 1), lambda1).max(f64::EPSILON);
        let e = Self::haversine_distance_m(phi1, lambda1, input.row_center_y(r0 - 1), lambda1).max(f64::EPSILON);
        let a = Self::haversine_distance_m(
            input.row_center_y(r0 + 1),
            input.col_center_x(c0),
            input.row_center_y(r0 + 1),
            input.col_center_x(c0 - 1),
        )
        .max(f64::EPSILON);
        let c = Self::haversine_distance_m(
            input.row_center_y(r0 - 1),
            input.col_center_x(c0),
            input.row_center_y(r0 - 1),
            input.col_center_x(c0 - 1),
        )
        .max(f64::EPSILON);

        let r = (c * c * (z[0] + z[2] - 2.0 * z[1])
            + b * b * (z[3] + z[5] - 2.0 * z[4])
            + a * a * (z[6] + z[8] - 2.0 * z[7]))
            / (a.powi(4) + b.powi(4) + c.powi(4));

        let t = 2.0
            / (3.0 * d * e * (d + e) * (a.powi(4) + b.powi(4) + c.powi(4)))
            * ((d * (a.powi(4) + b.powi(4) + b * b * c * c) - c * c * e * (a * a - b * b))
                * (z[0] + z[2])
                - (d * (a.powi(4) + c.powi(4) + b * b * c * c)
                    + e * (a.powi(4) + c.powi(4) + a * a * b * b))
                    * (z[3] + z[5])
                + (e * (b.powi(4) + c.powi(4) + a * a * b * b)
                    + a * a * d * (b * b - c * c))
                    * (z[6] + z[8])
                + d * (b.powi(4) * (z[1] - 3.0 * z[4])
                    + c.powi(4) * (3.0 * z[1] - z[4])
                    + (a.powi(4) - 2.0 * b * b * c * c) * (z[1] - z[4]))
                + e * (a.powi(4) * (3.0 * z[7] - z[4])
                    + b.powi(4) * (z[7] - 3.0 * z[4])
                    + (c.powi(4) - 2.0 * a * a * b * b) * (z[7] - z[4]))
                - 2.0 * (a * a * d * (b * b - c * c) * z[7]
                    - c * c * e * (a * a - b * b) * z[1]));

        let s = (c * (a * a * (d + e) + b * b * e) * (z[2] - z[0])
            - b * (a * a * d - c * c * e) * (z[3] - z[5])
            + a * (c * c * (d + e) + b * b * d) * (z[6] - z[8]))
            / (2.0 * (a * a * c * c * (d + e).powi(2) + b * b * (a * a * d * d + c * c * e * e)));

        let p = (a * a * c * d * (d + e) * (z[2] - z[0])
            + b * (a * a * d * d + c * c * e * e) * (z[5] - z[3])
            + a * c * c * e * (d + e) * (z[8] - z[6]))
            / (2.0 * (a * a * c * c * (d + e).powi(2) + b * b * (a * a * d * d + c * c * e * e)));

        let q = 1.0 / (3.0 * d * e * (d + e) * (a.powi(4) + b.powi(4) + c.powi(4)))
            * ((d * d * (a.powi(4) + b.powi(4) + b * b * c * c) + c * c * e * e * (a * a - b * b))
                * (z[0] + z[2])
                - (d * d * (a.powi(4) + c.powi(4) + b * b * c * c)
                    - e * e * (a.powi(4) + c.powi(4) + a * a * b * b))
                    * (z[3] + z[5])
                - (e * e * (b.powi(4) + c.powi(4) + a * a * b * b)
                    - a * a * d * d * (b * b - c * c))
                    * (z[6] + z[8])
                + d * d * (b.powi(4) * (z[1] - 3.0 * z[4])
                    + c.powi(4) * (3.0 * z[1] - z[4])
                    + (a.powi(4) - 2.0 * b * b * c * c) * (z[1] - z[4]))
                + e * e * (a.powi(4) * (z[4] - 3.0 * z[7])
                    + b.powi(4) * (3.0 * z[4] - z[7])
                    + (c.powi(4) - 2.0 * a * a * b * b) * (z[4] - z[7]))
                - 2.0 * (a * a * d * d * (b * b - c * c) * z[7]
                    + c * c * e * e * (a * a - b * b) * z[1]));

        let w = 1.0 + p * p + q * q;
        let g2 = p * p + q * q;

        // Optimization 2: Autovectorization improvements - use mul_add and avoid powf(1.5)
        let w_sqrt = w.sqrt();
        let w_pow_1p5 = w * w_sqrt; // w^1.5 = w * sqrt(w)
        
        let mean_curv = Self::finite_or_zero(-((1.0 + q * q).mul_add(r, 
            (1.0 + p * p).mul_add(t, -2.0 * p * q * s)))
            / (2.0 * w_pow_1p5));
        
        // Optimization 2: Replace powi(2) with multiplication
        let gaussian_curv = Self::finite_or_zero((r * t - s * s) / (w * w));
        let disc = (mean_curv * mean_curv - gaussian_curv).max(0.0);
        let sqrt_disc = disc.sqrt();
        let minimal_curv = Self::finite_or_zero(mean_curv - sqrt_disc);
        let maximal_curv = Self::finite_or_zero(mean_curv + sqrt_disc);
        let diff_curv = if g2 <= f64::EPSILON {
            0.0
        } else {
            Self::finite_or_zero(
                (q * q.mul_add(r, p * p.mul_add(t, -2.0 * p * q * s))) / (g2 * w_sqrt)
                    - ((1.0 + q * q).mul_add(r, (1.0 + p * p).mul_add(t, -2.0 * p * q * s)))
                        / (2.0 * w_pow_1p5),
            )
        };

        // Optimization 2: Replace powi(2) with multiplication
        let curvedness = Self::finite_or_zero(((minimal_curv * minimal_curv + maximal_curv * maximal_curv) / 2.0).sqrt());
        let shape_index = {
            let denom = maximal_curv - minimal_curv;
            if denom.abs() <= f64::EPSILON {
                0.0
            } else {
                Self::finite_or_zero(2.0 / std::f64::consts::PI * ((maximal_curv + minimal_curv) / denom).atan())
            }
        };
        let unsphericity = Self::finite_or_zero(sqrt_disc);
        let vertical_excess = Self::finite_or_zero(unsphericity + diff_curv);
        let accumulation = Self::finite_or_zero(mean_curv * mean_curv - diff_curv * diff_curv);
        let rotor = if g2 <= f64::EPSILON {
            0.0
        } else {
            // Optimization 2: Replace powi(3) with multiplication chain
            let g2_sqrt = g2.sqrt();
            Self::finite_or_zero((p * p - q * q).mul_add(s, -p * q * (r - t)) / (g2 * g2_sqrt))
        };
        let horizontal_curv = if g2 <= f64::EPSILON {
            0.0
        } else {
            Self::finite_or_zero((q * q.mul_add(r, p * p.mul_add(t, -2.0 * p * q * s))) / (g2 * w_sqrt))
        };
        let plan_curv = if g2 <= f64::EPSILON {
            0.0
        } else {
            // Optimization 2: Replace powi(3) with multiplication chain
            let g2_sqrt = g2.sqrt();
            Self::finite_or_zero(-(q * q.mul_add(r, p * p.mul_add(t, -2.0 * p * q * s)) / (g2 * g2_sqrt)))
        };
        let profile_curv = if g2 <= f64::EPSILON {
            0.0
        } else {
            Self::finite_or_zero(-(p * p.mul_add(r, q * q.mul_add(t, 2.0 * p * q * s)) / (g2 * w_pow_1p5)))
        };
        let ring_curv = if g2 <= f64::EPSILON {
            0.0
        } else {
            // Optimization 2: Replace powi(2) with multiplication
            let ratio = (p * p - q * q).mul_add(s, -p * q * (r - t)) / (g2 * w);
            Self::finite_or_zero(ratio * ratio)
        };
        let tan_curv = if g2 <= f64::EPSILON {
            0.0
        } else {
            Self::finite_or_zero(-(q * q.mul_add(r, p * p.mul_add(t, -2.0 * p * q * s))) / (g2 * w_sqrt))
        };
        let total_curv = Self::finite_or_zero(r * r + 2.0 * s * s + t * t);

        match curv_type {
            CurvatureType::Accumulation => accumulation,
            CurvatureType::Curvedness => curvedness,
            CurvatureType::Difference => diff_curv,
            CurvatureType::Gaussian => gaussian_curv,
            CurvatureType::GeneratingFunction => 0.0,
            CurvatureType::HorizontalExcess => horizontal_curv,
            CurvatureType::Maximal => maximal_curv,
            CurvatureType::Mean => mean_curv,
            CurvatureType::Minimal => minimal_curv,
            CurvatureType::Plan => plan_curv,
            CurvatureType::Profile => profile_curv,
            CurvatureType::Ring => ring_curv,
            CurvatureType::Rotor => rotor,
            CurvatureType::ShapeIndex => shape_index,
            CurvatureType::Tangential => tan_curv,
            CurvatureType::Total => total_curv,
            CurvatureType::Unsphericity => unsphericity,
            CurvatureType::VerticalExcess => vertical_excess,
        }
    }

    fn apply_log(v: f64, log_multiplier: f64, curv_type: CurvatureType, log_transform: bool) -> f64 {
        if !log_transform {
            return v;
        }
        if matches!(curv_type, CurvatureType::ShapeIndex) {
            return v;
        }
        v.signum() * (1.0 + log_multiplier * v.abs()).ln()
    }

    fn run_impl(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_mag_path = Self::parse_output_mag(args)?;
        let output_scale_path = Self::parse_output_scale(args)?;
        let cfg = Self::parse_config(args);

        let input = Self::load_raster(&input_path)?;
        let is_geographic = Self::raster_is_geographic(&input);
        if is_geographic && matches!(cfg.curv_type, CurvatureType::GeneratingFunction) {
            return Err(ToolError::Validation(
                "curv_type='generating_function' is not supported for geographic CRS inputs"
                    .to_string(),
            ));
        }
        let mut output_mag = input.clone();
        let mut output_scale = Raster::new(RasterConfig {
            rows: input.rows,
            cols: input.cols,
            bands: input.bands,
            nodata: -32768.0,
            x_min: input.x_min,
            y_min: input.y_min,
            cell_size: input.cell_size_x,
            cell_size_y: Some(input.cell_size_y),
            ..Default::default()
        });

        let rows = input.rows as isize;
        let coalescer = PercentCoalescer::new(1, 99);
        let cols = input.cols as isize;
        let nodata = input.nodata;
        let res = ((input.cell_size_x.abs() + input.cell_size_y.abs()) / 2.0).max(f64::EPSILON);
        let log_multiplier = Self::log_multiplier(res);

        let total_steps = (input.bands as isize * cfg.num_steps).max(1) as f64;
        let mut completed_steps = 0.0;

        for band_idx in 0..input.bands {
            let band = band_idx as isize;
            let mut best_mag = vec![nodata; (rows * cols) as usize];
            let mut best_scale = vec![-32768.0; (rows * cols) as usize];

            for s in 0..cfg.num_steps {
                let radius = Self::radius_for_step(cfg, s);
                let smoothed = Self::gaussian_blur_band(&input, band, radius, nodata);

                let mut curv = vec![nodata; (rows * cols) as usize];
                curv.par_chunks_mut(cols as usize)
                    .enumerate()
                    .for_each(|(r, row)| {
                        let rr = r as isize;
                        for c in 0..cols {
                            let idx = c as usize;
                            let center = smoothed[(rr * cols + c) as usize];
                            if !center.is_finite() || (center == nodata) {
                                continue;
                            }
                            let mut v = if is_geographic {
                                Self::compute_curvature_value_geographic(
                                    cfg.curv_type,
                                    &smoothed,
                                    &input,
                                    rows,
                                    cols,
                                    rr,
                                    c,
                                )
                            } else {
                                Self::compute_curvature_value_projected(
                                    cfg.curv_type,
                                    &smoothed,
                                    rows,
                                    cols,
                                    rr,
                                    c,
                                    res,
                                )
                            };
                            v = Self::apply_log(v, log_multiplier, cfg.curv_type, cfg.log_transform);
                            row[idx] = if v.is_finite() { v } else { 0.0 };
                        }
                    });

                if cfg.standardize {
                    let (sum, count) = curv
                        .iter()
                        .filter(|v| **v != nodata)
                        .fold((0.0f64, 0usize), |(acc, n), v| (acc + *v, n + 1));
                    if count > 0 {
                        let mean = sum / count as f64;
                        let sq_sum = curv
                            .iter()
                            .filter(|v| **v != nodata)
                            .map(|v| {
                                let d = *v - mean;
                                d * d
                            })
                            .sum::<f64>();
                        let std = (sq_sum / count as f64).sqrt();
                        if std > f64::EPSILON {
                            for v in &mut curv {
                                if *v != nodata {
                                    *v = (*v - mean) / std;
                                }
                            }
                        } else {
                            for v in &mut curv {
                                if *v != nodata {
                                    *v = 0.0;
                                }
                            }
                        }
                    }
                }

                for idx in 0..curv.len() {
                    let v = curv[idx];
                    if v == nodata {
                        best_mag[idx] = nodata;
                        best_scale[idx] = -32768.0;
                        continue;
                    }
                    if best_mag[idx] == nodata || v.abs() > best_mag[idx].abs() {
                        best_mag[idx] = v;
                        best_scale[idx] = radius as f64;
                    }
                }

                completed_steps += 1.0;
                coalescer.emit_unit_fraction(ctx.progress, completed_steps / total_steps);
            }

            for r in 0..rows {
                let start = (r * cols) as usize;
                let end = start + cols as usize;
                output_mag
                    .set_row_slice(band, r, &best_mag[start..end])
                    .map_err(|e| ToolError::Execution(format!("failed writing magnitude row {r}: {e}")))?;
                output_scale
                    .set_row_slice(band, r, &best_scale[start..end])
                    .map_err(|e| ToolError::Execution(format!("failed writing scale row {r}: {e}")))?;
            }
        }

        let mag_locator = Self::write_or_store_output(output_mag, output_mag_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(mag_locator));
        outputs.insert("active_band".to_string(), json!(0));

        if cfg.num_steps > 1 {
            if let Some(scale_path) = output_scale_path {
                let scale_locator = Self::write_or_store_output(output_scale, Some(scale_path))?;
                outputs.insert("scale_path".to_string(), json!(scale_locator));
            }
        }

        Ok(ToolRunResult { outputs })
    }
}

impl Tool for MultiscaleCurvaturesTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "multiscale_curvatures",
            display_name: "Multiscale Curvatures",
            summary: "Calculates multiscale curvatures and curvature indices using Gaussian scale-space smoothing.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input DEM raster path or typed raster object. Alias: dem.",
                    required: true,
                },
                ToolParamSpec {
                    name: "curv_type",
                    description: "Curvature type (e.g., profile, plan, mean, gaussian, shape, rotor, generating).",
                    required: false,
                },
                ToolParamSpec {
                    name: "out_mag",
                    description: "Optional output magnitude raster path. Alias: output.",
                    required: false,
                },
                ToolParamSpec {
                    name: "out_scale",
                    description: "Optional scale mosaic output path (used when num_steps > 1).",
                    required: false,
                },
                ToolParamSpec {
                    name: "min_scale",
                    description: "Minimum search-neighbourhood radius in grid cells (default 1).",
                    required: false,
                },
                ToolParamSpec {
                    name: "step",
                    description: "Base step size (default 1).",
                    required: false,
                },
                ToolParamSpec {
                    name: "num_steps",
                    description: "Number of sampled scale steps (default 10).",
                    required: false,
                },
                ToolParamSpec {
                    name: "step_nonlinearity",
                    description: "Step nonlinearity factor (default 1.0, clamped to [1.0, 4.0]).",
                    required: false,
                },
                ToolParamSpec {
                    name: "log_transform",
                    description: "Apply signed log transform to output values. Alias: log.",
                    required: false,
                },
                ToolParamSpec {
                    name: "standardize",
                    description: "Standardize each scale raster to z-scores before selecting max absolute values.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("curv_type".to_string(), json!("profile"));
        defaults.insert("min_scale".to_string(), json!(1));
        defaults.insert("step".to_string(), json!(1));
        defaults.insert("num_steps".to_string(), json!(10));
        defaults.insert("step_nonlinearity".to_string(), json!(1.0));
        defaults.insert("log_transform".to_string(), json!(false));
        defaults.insert("standardize".to_string(), json!(false));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));
        example_args.insert("curv_type".to_string(), json!("unsphericity"));
        example_args.insert("out_mag".to_string(), json!("multiscale_mag.tif"));
        example_args.insert("out_scale".to_string(), json!("multiscale_scale.tif"));
        example_args.insert("min_scale".to_string(), json!(1));
        example_args.insert("step".to_string(), json!(1));
        example_args.insert("num_steps".to_string(), json!(20));
        example_args.insert("step_nonlinearity".to_string(), json!(1.0));
        example_args.insert("log_transform".to_string(), json!(true));
        example_args.insert("standardize".to_string(), json!(true));

        ToolManifest {
            id: "multiscale_curvatures".to_string(),
            display_name: "Multiscale Curvatures".to_string(),
            summary: "Calculates multiscale curvatures and curvature-based indices from a DEM.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input DEM raster path or typed raster object. Alias: dem.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "curv_type".to_string(),
                    description: "Curvature type.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "out_mag".to_string(),
                    description: "Optional output magnitude path. Alias: output.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "out_scale".to_string(),
                    description: "Optional output scale path when num_steps > 1.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "min_scale".to_string(),
                    description: "Minimum search-neighbourhood radius in grid cells.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "step".to_string(),
                    description: "Step size.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "num_steps".to_string(),
                    description: "Number of sampled scale steps.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "step_nonlinearity".to_string(),
                    description: "Step nonlinearity factor.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "log_transform".to_string(),
                    description: "Apply signed log transform. Alias: log.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "standardize".to_string(),
                    description: "Standardize each scale to z-scores.".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "scale_mosaic_unsphericity".to_string(),
                description: "Create a multiscale unsphericity magnitude raster and scale mosaic.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "geomorphometry".to_string(),
                "multiscale".to_string(),
                "curvature".to_string(),
                "terrain".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = Self::parse_input(args)?;
        let _ = Self::parse_output_mag(args)?;
        let _ = Self::parse_output_scale(args)?;
        let cfg = Self::parse_config(args);
        if cfg.step < 1 {
            return Err(ToolError::Validation("parameter 'step' must be >= 1".to_string()));
        }
        if cfg.num_steps < 1 {
            return Err(ToolError::Validation("parameter 'num_steps' must be >= 1".to_string()));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        Self::run_impl(args, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wbcore::{AllowAllCapabilities, ProgressSink, ToolContext};

    struct NoopProgress;
    impl ProgressSink for NoopProgress {}

    fn make_ctx() -> ToolContext<'static> {
        static PROGRESS: NoopProgress = NoopProgress;
        static CAPS: AllowAllCapabilities = AllowAllCapabilities;
        ToolContext {
            progress: &PROGRESS,
            capabilities: &CAPS,
        }
    }

    fn make_constant_raster(rows: usize, cols: usize, value: f64) -> Raster {
        let cfg = RasterConfig {
            rows,
            cols,
            bands: 1,
            nodata: -9999.0,
            ..Default::default()
        };
        let mut r = Raster::new(cfg);
        for row in 0..rows as isize {
            for col in 0..cols as isize {
                r.set(0, row, col, value).unwrap();
            }
        }
        r
    }

    #[test]
    fn multiscale_curvatures_constant_dem_returns_zero_profile() {
        let mut args = ToolArgs::new();
        let in_id = memory_store::put_raster(make_constant_raster(32, 32, 5.0));
        args.insert(
            "input".to_string(),
            json!(memory_store::make_raster_memory_path(&in_id)),
        );
        args.insert("curv_type".to_string(), json!("profile"));
        args.insert("min_scale".to_string(), json!(1));
        args.insert("step".to_string(), json!(1));
        args.insert("num_steps".to_string(), json!(3));
        args.insert("log_transform".to_string(), json!(false));

        let tool = MultiscaleCurvaturesTool;
        let res = tool.run(&args, &make_ctx()).unwrap();
        let out_path = res.outputs.get("path").unwrap().as_str().unwrap();
        let out_id = memory_store::raster_path_to_id(out_path).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();

        assert!(out.get(0, 16, 16).abs() < 1e-10);
    }
}
