use nalgebra::{DMatrix, DVector, Matrix3, Vector3};
use rayon::prelude::*;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use wbcore::{
    parse_optional_output_path, LicenseTier, Tool, ToolArgs, ToolCategory, ToolContext,
    ToolError, ToolExample, ToolManifest, ToolMetadata, ToolParamDescriptor, ToolParamSpec,
    ToolRunResult, ToolStability,
};
use wbraster::{CrsInfo, Raster, RasterConfig, RasterFormat, ResampleMethod};

use crate::memory_store;

#[derive(Clone, Debug)]
struct Gcp {
    pixel_x: f64,
    pixel_y: f64,
    map_x: f64,
    map_y: f64,
}

#[derive(Clone, Debug)]
enum TransformModel {
    Affine { ax: [f64; 3], ay: [f64; 3] },
    Projective { h: Matrix3<f64> },
    Polynomial { order: u8, ax: Vec<f64>, ay: Vec<f64> },
    ThinPlateSpline { gcps: Vec<Gcp>, ax: [f64; 3], ay: [f64; 3], wx: Vec<f64>, wy: Vec<f64> },
}

#[derive(Clone, Debug)]
struct GcpResidual {
    index: usize,
    dx: f64,
    dy: f64,
    radial_error: f64,
}

#[derive(Clone, Debug)]
struct ReportData {
    requested_transform: String,
    applied_transform: String,
    requested_order: Option<u8>,
    effective_order: Option<u8>,
    gcp_count: usize,
    rmse: f64,
    warnings: Vec<String>,
    residuals: Vec<GcpResidual>,
    downgraded: bool,
}

pub struct GeoreferenceRasterFromControlPointsTool;

fn required_terms(order: u8) -> usize {
    match order {
        1 => 3,
        2 => 6,
        3 => 10,
        _ => 0,
    }
}

fn resample_method_from_str(value: &str) -> Result<ResampleMethod, ToolError> {
    match value.to_ascii_lowercase().as_str() {
        "nearest" | "nearest_neighbor" => Ok(ResampleMethod::Nearest),
        "bilinear" => Ok(ResampleMethod::Bilinear),
        "cubic" => Ok(ResampleMethod::Cubic),
        "lanczos" => Ok(ResampleMethod::Lanczos),
        "average" | "mean" => Ok(ResampleMethod::Average),
        "min" | "minimum" => Ok(ResampleMethod::Min),
        "max" | "maximum" => Ok(ResampleMethod::Max),
        "mode" | "modal" => Ok(ResampleMethod::Mode),
        "median" => Ok(ResampleMethod::Median),
        "stddev" | "std_dev" | "standard_deviation" => Ok(ResampleMethod::StdDev),
        _ => Err(ToolError::Validation(
            "parameter 'resample' must be one of: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev".to_string(),
        )),
    }
}

fn load_raster(path: &str, label: &str) -> Result<Raster, ToolError> {
    if memory_store::raster_is_memory_path(path) {
        let id = memory_store::raster_path_to_id(path)
            .ok_or_else(|| ToolError::Execution(format!("invalid memory path for '{label}'")))?;
        memory_store::get_raster_arc_by_id(id)
            .map(|r| r.as_ref().clone())
            .ok_or_else(|| ToolError::Execution(format!("memory raster not found for '{label}'")))
    } else {
        Raster::read(Path::new(path))
            .map_err(|e| ToolError::Execution(format!("failed reading '{label}': {e}")))
    }
}

fn write_raster(r: &Raster, path: &str, label: &str) -> Result<(), ToolError> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| {
                ToolError::Execution(format!("failed creating output directory for '{label}': {e}"))
            })?;
        }
    }
    r.write(path, RasterFormat::GeoTiff)
        .map_err(|e| ToolError::Execution(format!("failed writing '{label}': {e}")))
}

fn read_control_points(path: &str) -> Result<Vec<Gcp>, ToolError> {
    let text = fs::read_to_string(path)
        .map_err(|e| ToolError::Execution(format!("failed reading control points CSV: {e}")))?;
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| ToolError::Validation("control_points CSV is empty".to_string()))?;
    let headers: Vec<String> = header
        .split(',')
        .map(|s| s.trim().to_ascii_lowercase())
        .collect();
    let idx = |names: &[&str]| -> Option<usize> {
        headers.iter().position(|h| names.iter().any(|n| h == n))
    };
    let px = idx(&["pixel_x", "source_col", "col", "x", "pixelcol"]) 
        .ok_or_else(|| ToolError::Validation("control_points CSV must include pixel_x/source_col".to_string()))?;
    let py = idx(&["pixel_y", "source_row", "row", "y", "pixelrow"]) 
        .ok_or_else(|| ToolError::Validation("control_points CSV must include pixel_y/source_row".to_string()))?;
    let mx = idx(&["map_x", "target_x", "x_map", "world_x"]) 
        .ok_or_else(|| ToolError::Validation("control_points CSV must include map_x/target_x".to_string()))?;
    let my = idx(&["map_y", "target_y", "y_map", "world_y"]) 
        .ok_or_else(|| ToolError::Validation("control_points CSV must include map_y/target_y".to_string()))?;

    let mut out = Vec::new();
    for (line_no, line) in lines.enumerate() {
        let cols: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if cols.len() <= px.max(py).max(mx).max(my) {
            return Err(ToolError::Validation(format!(
                "control_points CSV line {} has too few columns",
                line_no + 2
            )));
        }
        let parse = |i: usize| -> Result<f64, ToolError> {
            cols[i].parse::<f64>().map_err(|_| {
                ToolError::Validation(format!("invalid numeric control point value on line {}", line_no + 2))
            })
        };
        out.push(Gcp {
            pixel_x: parse(px)?,
            pixel_y: parse(py)?,
            map_x: parse(mx)?,
            map_y: parse(my)?,
        });
    }
    if out.is_empty() {
        return Err(ToolError::Validation("control_points CSV contains no GCP rows".to_string()));
    }
    Ok(out)
}

fn design_terms(order: u8, x: f64, y: f64) -> Vec<f64> {
    let mut terms = vec![1.0, x, y];
    if order >= 2 {
        terms.extend([x * x, x * y, y * y]);
    }
    if order >= 3 {
        terms.extend([x * x * x, x * x * y, x * y * y, y * y * y]);
    }
    terms
}

fn fit_affine(gcps: &[Gcp]) -> Result<TransformModel, ToolError> {
    let n = gcps.len();
    let mut a = DMatrix::<f64>::zeros(2 * n, 6);
    let mut b = DVector::<f64>::zeros(2 * n);
    for (i, g) in gcps.iter().enumerate() {
        let r = 2 * i;
        a[(r, 0)] = 1.0;
        a[(r, 1)] = g.pixel_x;
        a[(r, 2)] = g.pixel_y;
        b[r] = g.map_x;
        a[(r + 1, 3)] = 1.0;
        a[(r + 1, 4)] = g.pixel_x;
        a[(r + 1, 5)] = g.pixel_y;
        b[r + 1] = g.map_y;
    }
    let sol = a.qr().solve(&b).ok_or_else(|| ToolError::Execution("affine fit failed".to_string()))?;
    Ok(TransformModel::Affine {
        ax: [sol[0], sol[1], sol[2]],
        ay: [sol[3], sol[4], sol[5]],
    })
}

fn fit_projective(gcps: &[Gcp]) -> Result<TransformModel, ToolError> {
    let n = gcps.len();
    let mut a = DMatrix::<f64>::zeros(2 * n, 8);
    let mut b = DVector::<f64>::zeros(2 * n);
    for (i, g) in gcps.iter().enumerate() {
        let r = 2 * i;
        a[(r, 0)] = g.pixel_x;
        a[(r, 1)] = g.pixel_y;
        a[(r, 2)] = 1.0;
        a[(r, 6)] = -g.map_x * g.pixel_x;
        a[(r, 7)] = -g.map_x * g.pixel_y;
        b[r] = g.map_x;
        a[(r + 1, 3)] = g.pixel_x;
        a[(r + 1, 4)] = g.pixel_y;
        a[(r + 1, 5)] = 1.0;
        a[(r + 1, 6)] = -g.map_y * g.pixel_x;
        a[(r + 1, 7)] = -g.map_y * g.pixel_y;
        b[r + 1] = g.map_y;
    }
    let sol = a.qr().solve(&b).ok_or_else(|| ToolError::Execution("projective fit failed".to_string()))?;
    Ok(TransformModel::Projective {
        h: Matrix3::new(sol[0], sol[1], sol[2], sol[3], sol[4], sol[5], sol[6], sol[7], 1.0),
    })
}

fn fit_polynomial(gcps: &[Gcp], order: u8) -> Result<TransformModel, ToolError> {
    let terms = required_terms(order);
    let n = gcps.len();
    let mut a = DMatrix::<f64>::zeros(n, terms);
    let mut bx = DVector::<f64>::zeros(n);
    let mut by = DVector::<f64>::zeros(n);
    for (i, g) in gcps.iter().enumerate() {
        let t = design_terms(order, g.pixel_x, g.pixel_y);
        for (j, v) in t.iter().enumerate() {
            a[(i, j)] = *v;
        }
        bx[i] = g.map_x;
        by[i] = g.map_y;
    }
    let ax = a.clone().qr().solve(&bx).ok_or_else(|| ToolError::Execution("polynomial x fit failed".to_string()))?;
    let ay = a.qr().solve(&by).ok_or_else(|| ToolError::Execution("polynomial y fit failed".to_string()))?;
    Ok(TransformModel::Polynomial {
        order,
        ax: ax.iter().copied().collect(),
        ay: ay.iter().copied().collect(),
    })
}

fn tps_kernel(r2: f64) -> f64 {
    if r2 <= 0.0 {
        0.0
    } else {
        r2 * r2.ln()
    }
}

fn build_tps_system(gcps: &[Gcp]) -> DMatrix<f64> {
    let n = gcps.len();
    let dim = n + 3;
    let mut system = DMatrix::<f64>::zeros(dim, dim);

    for i in 0..n {
        let gi = &gcps[i];
        system[(i, n)] = 1.0;
        system[(i, n + 1)] = gi.pixel_x;
        system[(i, n + 2)] = gi.pixel_y;
        system[(n, i)] = 1.0;
        system[(n + 1, i)] = gi.pixel_x;
        system[(n + 2, i)] = gi.pixel_y;

        for j in 0..n {
            let gj = &gcps[j];
            let dx = gi.pixel_x - gj.pixel_x;
            let dy = gi.pixel_y - gj.pixel_y;
            system[(i, j)] = tps_kernel(dx * dx + dy * dy);
        }
    }

    system
}

fn fit_thin_plate_spline(gcps: &[Gcp]) -> Result<TransformModel, ToolError> {
    let system = build_tps_system(gcps);
    let n = gcps.len();
    let mut rhs_x = DVector::<f64>::zeros(n + 3);
    let mut rhs_y = DVector::<f64>::zeros(n + 3);

    for (i, g) in gcps.iter().enumerate() {
        rhs_x[i] = g.map_x;
        rhs_y[i] = g.map_y;
    }

    let sol_x = system.clone().lu().solve(&rhs_x).ok_or_else(|| ToolError::Execution("thin-plate spline fit failed for x coordinates".to_string()))?;
    let sol_y = system.lu().solve(&rhs_y).ok_or_else(|| ToolError::Execution("thin-plate spline fit failed for y coordinates".to_string()))?;

    Ok(TransformModel::ThinPlateSpline {
        gcps: gcps.to_vec(),
        ax: [sol_x[n], sol_x[n + 1], sol_x[n + 2]],
        ay: [sol_y[n], sol_y[n + 1], sol_y[n + 2]],
        wx: sol_x.iter().take(n).copied().collect(),
        wy: sol_y.iter().take(n).copied().collect(),
    })
}

fn forward(model: &TransformModel, x: f64, y: f64) -> (f64, f64) {
    match model {
        TransformModel::Affine { ax, ay } => (
            ax[0] + ax[1] * x + ax[2] * y,
            ay[0] + ay[1] * x + ay[2] * y,
        ),
        TransformModel::Projective { h } => {
            let v = h * Vector3::new(x, y, 1.0);
            (v[0] / v[2], v[1] / v[2])
        }
        TransformModel::Polynomial { order, ax, ay } => {
            let t = design_terms(*order, x, y);
            let mx = ax.iter().zip(t.iter()).map(|(a, b)| a * b).sum();
            let my = ay.iter().zip(t.iter()).map(|(a, b)| a * b).sum();
            (mx, my)
        }
        TransformModel::ThinPlateSpline { gcps, ax, ay, wx, wy } => {
            let mut mx = ax[0] + ax[1] * x + ax[2] * y;
            let mut my = ay[0] + ay[1] * x + ay[2] * y;
            for (i, gcp) in gcps.iter().enumerate() {
                let dx = x - gcp.pixel_x;
                let dy = y - gcp.pixel_y;
                let basis = tps_kernel(dx * dx + dy * dy);
                mx += wx[i] * basis;
                my += wy[i] * basis;
            }
            (mx, my)
        }
    }
}

fn jacobian_poly(order: u8, ax: &[f64], ay: &[f64], x: f64, y: f64) -> [[f64; 2]; 2] {
    let mut dx_dx = ax[1];
    let mut dx_dy = ax[2];
    let mut dy_dx = ay[1];
    let mut dy_dy = ay[2];
    if order >= 2 {
        dx_dx += 2.0 * ax[3] * x + ax[4] * y;
        dx_dy += ax[4] * x + 2.0 * ax[5] * y;
        dy_dx += 2.0 * ay[3] * x + ay[4] * y;
        dy_dy += ay[4] * x + 2.0 * ay[5] * y;
    }
    if order >= 3 {
        dx_dx += 3.0 * ax[6] * x * x + 2.0 * ax[7] * x * y + ax[8] * y * y;
        dx_dy += ax[7] * x * x + 2.0 * ax[8] * x * y + 3.0 * ax[9] * y * y;
        dy_dx += 3.0 * ay[6] * x * x + 2.0 * ay[7] * x * y + ay[8] * y * y;
        dy_dy += ay[7] * x * x + 2.0 * ay[8] * x * y + 3.0 * ay[9] * y * y;
    }
    [[dx_dx, dx_dy], [dy_dx, dy_dy]]
}

fn inverse_map(model: &TransformModel, x: f64, y: f64, initial: (f64, f64)) -> Option<(f64, f64)> {
    match model {
        TransformModel::Affine { ax, ay } => {
            let det = ax[1] * ay[2] - ax[2] * ay[1];
            if det.abs() < 1e-12 { return None; }
            let dx = x - ax[0];
            let dy = y - ay[0];
            Some(((dx * ay[2] - dy * ax[2]) / det, (ax[1] * dy - ay[1] * dx) / det))
        }
        TransformModel::Projective { h } => {
            let inv = h.try_inverse()?;
            let v = inv * Vector3::new(x, y, 1.0);
            Some((v[0] / v[2], v[1] / v[2]))
        }
        TransformModel::Polynomial { order, ax, ay } => {
            let mut px = initial.0;
            let mut py = initial.1;
            for _ in 0..20 {
                let (fx, fy) = forward(model, px, py);
                let dx = fx - x;
                let dy = fy - y;
                if dx.abs() + dy.abs() < 1e-8 { return Some((px, py)); }
                let j = jacobian_poly(*order, ax, ay, px, py);
                let det = j[0][0] * j[1][1] - j[0][1] * j[1][0];
                if det.abs() < 1e-12 { return None; }
                let step_x = (dx * j[1][1] - dy * j[0][1]) / det;
                let step_y = (j[0][0] * dy - j[1][0] * dx) / det;
                px -= step_x;
                py -= step_y;
            }
            None
        }
        TransformModel::ThinPlateSpline { gcps, ax, ay, wx, wy } => {
            let mut px = initial.0;
            let mut py = initial.1;
            for _ in 0..30 {
                let (fx, fy) = forward(model, px, py);
                let dx = fx - x;
                let dy = fy - y;
                if dx.abs() + dy.abs() < 1e-8 {
                    return Some((px, py));
                }

                let mut jxx = ax[1];
                let mut jxy = ax[2];
                let mut jyx = ay[1];
                let mut jyy = ay[2];

                for (i, gcp) in gcps.iter().enumerate() {
                    let ddx = px - gcp.pixel_x;
                    let ddy = py - gcp.pixel_y;
                    let r2 = ddx * ddx + ddy * ddy;
                    if r2 > 0.0 {
                        let factor_x = 2.0 * wx[i] * (r2.ln() + 1.0);
                        let factor_y = 2.0 * wy[i] * (r2.ln() + 1.0);
                        jxx += factor_x * ddx;
                        jxy += factor_x * ddy;
                        jyx += factor_y * ddx;
                        jyy += factor_y * ddy;
                    }
                }

                let det = jxx * jyy - jxy * jyx;
                if det.abs() < 1e-12 {
                    return None;
                }

                let step_x = (dx * jyy - dy * jxy) / det;
                let step_y = (jxx * dy - jyx * dx) / det;
                px -= step_x;
                py -= step_y;
            }
            None
        }
    }
}

fn sample_nearest(src: &Raster, band: usize, x: f64, y: f64) -> f64 {
    let c = x.round() as isize;
    let r = y.round() as isize;
    if c < 0 || r < 0 || c as usize >= src.cols || r as usize >= src.rows {
        return src.nodata;
    }
    let idx = band * src.rows * src.cols + r as usize * src.cols + c as usize;
    src.data.get_f64(idx)
}

fn sample_bilinear(src: &Raster, band: usize, x: f64, y: f64) -> f64 {
    let c0 = x.floor() as isize;
    let r0 = y.floor() as isize;
    let c1 = c0 + 1;
    let r1 = r0 + 1;
    if c0 < 0 || r0 < 0 || c1 as usize >= src.cols || r1 as usize >= src.rows {
        return src.nodata;
    }
    let base = band * src.rows * src.cols;
    let v00 = src.data.get_f64(base + r0 as usize * src.cols + c0 as usize);
    let v01 = src.data.get_f64(base + r0 as usize * src.cols + c1 as usize);
    let v10 = src.data.get_f64(base + r1 as usize * src.cols + c0 as usize);
    let v11 = src.data.get_f64(base + r1 as usize * src.cols + c1 as usize);
    if src.is_nodata(v00) || src.is_nodata(v01) || src.is_nodata(v10) || src.is_nodata(v11) {
        return src.nodata;
    }
    let dc = x - c0 as f64;
    let dr = y - r0 as f64;
    (1.0 - dr) * ((1.0 - dc) * v00 + dc * v01) + dr * ((1.0 - dc) * v10 + dc * v11)
}

fn fit_and_transform(gcps: &[Gcp], transform_type: &str, order: Option<u8>, allow_auto_downgrade: bool) -> Result<(TransformModel, Vec<String>, bool, Option<u8>), ToolError> {
    let mut warnings = Vec::new();
    let mut downgraded = false;
    let (requested, effective_order) = match transform_type.to_ascii_lowercase().as_str() {
        "affine" => ("affine", None),
        "projective" => ("projective", None),
        "polynomial" => ("polynomial", Some(order.unwrap_or(1))),
        "thin_plate_spline" | "thinplatespline" | "tps" => ("thin_plate_spline", None),
        other => return Err(ToolError::Validation(format!("invalid transform_type '{other}'"))),
    };

    let build = |kind: &str, ord: Option<u8>| -> Result<TransformModel, ToolError> {
        match kind {
            "affine" => fit_affine(gcps),
            "projective" => fit_projective(gcps),
            "polynomial" => fit_polynomial(gcps, ord.unwrap_or(1)),
            "thin_plate_spline" => fit_thin_plate_spline(gcps),
            _ => unreachable!(),
        }
    };

    let min_needed = match (requested, effective_order) {
        ("affine", _) => 3,
        ("projective", _) => 4,
        ("polynomial", Some(o)) => required_terms(o),
        ("thin_plate_spline", _) => 10,
        _ => 0,
    };

    let mut candidate_kind = requested;
    let mut candidate_order = effective_order;
    loop {
        let candidate_needed = match (candidate_kind, candidate_order) {
            ("affine", _) => 3,
            ("projective", _) => 4,
            ("polynomial", Some(o)) => required_terms(o),
            ("thin_plate_spline", _) => 10,
            _ => 0,
        };
        if gcps.len() >= candidate_needed {
            let model = build(candidate_kind, candidate_order)?;
            if downgraded {
                warnings.push(format!("requested transform downgraded to {candidate_kind}"));
            }
            return Ok((model, warnings, downgraded, candidate_order));
        }

        if !allow_auto_downgrade {
            return Err(ToolError::Validation(format!(
                "insufficient control points for {candidate_kind}{:?}: required {}, found {}",
                candidate_order, candidate_needed, gcps.len()
            )));
        }

        downgraded = true;
        match (candidate_kind, candidate_order) {
            ("thin_plate_spline", _) => {
                candidate_kind = "polynomial";
                candidate_order = Some(2);
            }
            ("polynomial", Some(3)) => candidate_order = Some(2),
            ("polynomial", Some(2)) => candidate_order = Some(1),
            ("polynomial", Some(1)) => candidate_kind = "affine",
            ("projective", _) => candidate_kind = "affine",
            ("affine", _) => break,
            _ => break,
        }
    }

    Err(ToolError::Validation(format!(
        "insufficient control points for requested transform '{}'; found {} (minimum {})",
        requested, gcps.len(), min_needed
    )))
}

fn design_matrix_condition_warning(gcps: &[Gcp], model: &TransformModel) -> Option<String> {
    let a = match model {
        TransformModel::Affine { .. } | TransformModel::Projective { .. } => return None,
        TransformModel::Polynomial { order, .. } => {
            let terms = required_terms(*order);
            let mut a = DMatrix::<f64>::zeros(gcps.len(), terms);
            for (i, g) in gcps.iter().enumerate() {
                for (j, v) in design_terms(*order, g.pixel_x, g.pixel_y).iter().enumerate() {
                    a[(i, j)] = *v;
                }
            }
            a
        }
        TransformModel::ThinPlateSpline { .. } => build_tps_system(gcps),
    };
    let svd = a.svd(true, true);
    let mut sigma_max: f64 = 0.0;
    let mut sigma_min = f64::INFINITY;
    for s in svd.singular_values.iter().copied() {
        sigma_max = sigma_max.max(s);
        if s > 0.0 {
            sigma_min = sigma_min.min(s);
        }
    }
    if sigma_min.is_finite() && sigma_min > 0.0 {
        let condition = sigma_max / sigma_min;
        if condition > 1.0e8 {
            return Some(format!("polynomial fit is poorly conditioned (condition number {:.3e})", condition));
        }
    }
    None
}

fn detect_duplicate_gcps(gcps: &[Gcp]) -> Option<usize> {
    for i in 0..gcps.len() {
        for j in (i + 1)..gcps.len() {
            if (gcps[i].pixel_x - gcps[j].pixel_x).abs() < 1e-12
                && (gcps[i].pixel_y - gcps[j].pixel_y).abs() < 1e-12
            {
                return Some(j + 1);
            }
        }
    }
    None
}

fn build_output_raster(src: &Raster, epsg: u32, extent: (f64, f64, f64, f64), cell_size: f64) -> Raster {
    let cols = (((extent.2 - extent.0) / cell_size).ceil() as usize).max(1);
    let rows = (((extent.3 - extent.1) / cell_size).ceil() as usize).max(1);
    Raster::new(RasterConfig {
        rows,
        cols,
        bands: src.bands,
        nodata: src.nodata,
        x_min: extent.0,
        y_min: extent.1,
        cell_size,
        cell_size_y: Some(cell_size),
        data_type: src.data_type,
        crs: CrsInfo::from_epsg(epsg),
        metadata: vec![],
        ..Default::default()
    })
}

impl Tool for GeoreferenceRasterFromControlPointsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "georeference_raster_from_control_points",
            display_name: "Georeference Raster From Control Points",
            summary: r#"The georeference raster from control points tool establishes geographic coordinate systems for ungeoreferenced rasters by fitting polynomial transformation functions through operator-identified ground control points (GCPs), converting pixel coordinates to real-world map projections with quantified accuracy assessment. Users identify homologous pixel locations matching known reference coordinates (extracted from accurate vector maps, GPS measurements, or previously georeferenced imagery), and the tool fits first-order (affine), second-order (quadratic), or third-order (cubic) polynomial transformation equations through least-squares optimization. The tool computes root-mean-square error (RMSE) quantifying transformation accuracy—lower RMSE indicates better fit; typical acceptable thresholds range 0.5-2.0 pixels depending on analytical requirements. Key features include flexible polynomial order selection balancing accuracy against data requirements (affine requires minimum 3 points; quadratic requires 6; cubic requires 10), both forward and inverse transformation computation enabling pixel-to-coordinate and coordinate-to-pixel conversions, and automated RMSE calculation providing quantitative confidence assessment. Use cases span historical map digitization, aerial photograph georeferencing, declassified satellite imagery processing, drone survey integration, and legacy document archive georeferencing. Applications include aerial survey archives requiring integration with modern mapping systems, historical imagery time-series construction for change analysis, drone orthomosaic generation, and localization of field sampling sites. Output interpretation requires evaluating RMSE against project accuracy specifications—sub-pixel RMSE typically indicates excellent fits suitable for high-precision applications; multi-pixel RMSE may be acceptable for reconnaissance-level work. RMSE statistics reveal spatial error distribution; systematic errors suggest missing GCP distribution or non-polynomial distortions. Residual analysis isolates problem GCPs requiring verification. Output georeferenced rasters inherit input coordinate system from EPSG specification; verify EPSG code matches target projection before processing. Resampling method selection (nearest-neighbor for classification, bilinear for continuous data, cubic for photographic quality) affects output spectral accuracy and raster size."#,
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster path.", required: true },
                ToolParamSpec { name: "control_points", description: "Control point CSV path with pixel_x,pixel_y,map_x,map_y (and accepted aliases).", required: true },
                ToolParamSpec { name: "epsg", description: "Target EPSG code.", required: true },
                ToolParamSpec { name: "transform_type", description: "Transform type: affine, projective, polynomial, or thin_plate_spline.", required: false },
                ToolParamSpec { name: "transform_order", description: "Polynomial order when transform_type=polynomial.", required: false },
                ToolParamSpec { name: "resample", description: "Resampling method; default bilinear.", required: false },
                ToolParamSpec { name: "allow_auto_downgrade", description: "Allow automatic downgrade to a simpler transform when GCPs are sparse.", required: false },
                ToolParamSpec { name: "output", description: "Output georeferenced raster path.", required: false },
                ToolParamSpec { name: "report", description: "Output report JSON path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("control_points".to_string(), json!("input_gcps.csv"));
        defaults.insert("epsg".to_string(), json!(32618));
        defaults.insert("transform_type".to_string(), json!("polynomial"));
        defaults.insert("transform_order".to_string(), json!(1));
        defaults.insert("resample".to_string(), json!("bilinear"));
        defaults.insert("allow_auto_downgrade".to_string(), json!(false));
        defaults.insert("output".to_string(), json!("georeferenced.tif"));
        defaults.insert("report".to_string(), json!("georeferenced_report.json"));

        ToolManifest {
            id: "georeference_raster_from_control_points".to_string(),
            display_name: "Georeference Raster From Control Points".to_string(),
            summary: "Fits a transform from GCPs and warps a raster into georeferenced output.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: self.metadata().params.into_iter().map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            }).collect(),
            defaults: defaults.clone(),
            examples: vec![ToolExample {
                name: "georeference_scan".to_string(),
                description: "Georeference a scanned raster from control points using bilinear resampling.".to_string(),
                args: defaults,
            }],
            tags: vec!["raster".to_string(), "projection".to_string(), "georeferencing".to_string()],
            stability: ToolStability::Stable,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        args.get("input").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
            .ok_or_else(|| ToolError::Validation("parameter 'input' is required".to_string()))?;
        args.get("control_points").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
            .ok_or_else(|| ToolError::Validation("parameter 'control_points' is required".to_string()))?;
        args.get("epsg").and_then(|v| v.as_u64())
            .ok_or_else(|| ToolError::Validation("parameter 'epsg' is required".to_string()))?;
        if let Some(t) = args.get("transform_type").and_then(|v| v.as_str()) {
            if !matches!(t.to_ascii_lowercase().as_str(), "affine" | "projective" | "polynomial" | "thin_plate_spline" | "thinplatespline" | "tps") {
                return Err(ToolError::Validation("parameter 'transform_type' must be affine, projective, polynomial, or thin_plate_spline".to_string()));
            }
        }
        if let Some(o) = args.get("transform_order").and_then(|v| v.as_u64()) {
            if !(1..=3).contains(&(o as u8)) {
                return Err(ToolError::Validation("parameter 'transform_order' must be 1, 2, or 3".to_string()));
            }
        }
        if let Some(resample) = args.get("resample").and_then(|v| v.as_str()) {
            resample_method_from_str(resample)?;
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = args.get("input").and_then(|v| v.as_str()).unwrap().to_string();
        let gcp_path = args.get("control_points").and_then(|v| v.as_str()).unwrap().to_string();
        let epsg = args.get("epsg").and_then(|v| v.as_u64()).unwrap() as u32;
        let transform_type = args.get("transform_type").and_then(|v| v.as_str()).unwrap_or("polynomial");
        let transform_order = args.get("transform_order").and_then(|v| v.as_u64()).map(|v| v as u8);
        let allow_auto_downgrade = args.get("allow_auto_downgrade").and_then(|v| v.as_bool()).unwrap_or(false);
        let resample = resample_method_from_str(args.get("resample").and_then(|v| v.as_str()).unwrap_or("bilinear"))?;
        let input = load_raster(&input_path, "input")?;
        let gcps = read_control_points(&gcp_path)?;

        if let Some(n) = detect_duplicate_gcps(&gcps) {
            return Err(ToolError::Validation(format!("duplicate GCP pixel coordinates detected near record {}", n)));
        }

        let (model, mut warnings, downgraded, effective_order) = fit_and_transform(&gcps, transform_type, transform_order, allow_auto_downgrade)?;

        if let Some(msg) = design_matrix_condition_warning(&gcps, &model) {
            warnings.push(msg);
        }

        let residuals: Vec<GcpResidual> = gcps.iter().enumerate().map(|(i, g)| {
            let (mx, my) = forward(&model, g.pixel_x, g.pixel_y);
            let dx = mx - g.map_x;
            let dy = my - g.map_y;
            GcpResidual { index: i + 1, dx, dy, radial_error: (dx * dx + dy * dy).sqrt() }
        }).collect();
        let rmse = (residuals.iter().map(|r| r.dx * r.dx + r.dy * r.dy).sum::<f64>() / residuals.len() as f64).sqrt();

        let mut transformed_pts = Vec::new();
        let samples = 11;
        let cols = input.cols as f64;
        let rows = input.rows as f64;
        for i in 0..samples {
            let t = i as f64 / (samples - 1) as f64;
            let x0 = t * cols;
            let y0 = 0.0;
            let y1 = rows;
            transformed_pts.push(forward(&model, x0, y0));
            transformed_pts.push(forward(&model, x0, y1));
            transformed_pts.push(forward(&model, 0.0, x0));
            transformed_pts.push(forward(&model, cols, x0));
        }
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for (x, y) in &transformed_pts {
            min_x = min_x.min(*x);
            min_y = min_y.min(*y);
            max_x = max_x.max(*x);
            max_y = max_y.max(*y);
        }

        if min_x.partial_cmp(&max_x).is_none() || min_y.partial_cmp(&max_y).is_none() || (max_x - min_x).abs() < 1e-9 || (max_y - min_y).abs() < 1e-9 {
            return Err(ToolError::Execution("derived output extent is invalid or degenerate".to_string()));
        }

        let out_extent_area = (max_x - min_x) * (max_y - min_y);
        let gcp_span_x = gcps.iter().map(|g| g.pixel_x).fold(f64::NEG_INFINITY, f64::max) - gcps.iter().map(|g| g.pixel_x).fold(f64::INFINITY, f64::min);
        let gcp_span_y = gcps.iter().map(|g| g.pixel_y).fold(f64::NEG_INFINITY, f64::max) - gcps.iter().map(|g| g.pixel_y).fold(f64::INFINITY, f64::min);
        if gcp_span_x <= 0.0 || gcp_span_y <= 0.0 {
            return Err(ToolError::Validation("control points do not span a usable 2D area".to_string()));
        }
        if out_extent_area <= 0.0 {
            return Err(ToolError::Execution("derived output extent has non-positive area".to_string()));
        }

        let cx = cols * 0.5;
        let cy = rows * 0.5;
        let (fx, fy) = forward(&model, cx, cy);
        let (fx_dx, fy_dx) = forward(&model, cx + 1.0, cy);
        let (fx_dy, fy_dy) = forward(&model, cx, cy + 1.0);
        let sx = ((fx_dx - fx).powi(2) + (fy_dx - fy).powi(2)).sqrt();
        let sy = ((fx_dy - fx).powi(2) + (fy_dy - fy).powi(2)).sqrt();
        let cell_size = sx.max(sy).max(1e-9);

        let mut out = build_output_raster(&input, epsg, (min_x, min_y, max_x, max_y), cell_size);
        let out_rows = out.rows;
        let out_cols = out.cols;
        let out_bands = out.bands;
        let src_nodata = input.nodata;

        let source_view = {
            let mut r = input.clone();
            r.x_min = 0.0;
            r.y_min = 0.0;
            r.cell_size_x = 1.0;
            r.cell_size_y = 1.0;
            r
        };

        let seed_inverse = fit_affine(&gcps).ok();

        let row_buffers: Vec<Vec<f64>> = (0..out_rows).into_par_iter().map(|row| {
            let mut buf = vec![src_nodata; out_cols * out_bands];
            let wy = out.y_max() - (row as f64 + 0.5) * out.cell_size_y;
            for col in 0..out_cols {
                let wx = out.x_min + (col as f64 + 0.5) * out.cell_size_x;
                let initial = if let Some(seed) = &seed_inverse {
                    inverse_map(seed, wx, wy, (cx, cy)).unwrap_or((cx, cy))
                } else {
                    (cx, cy)
                };
                let (sx0, sy0) = match inverse_map(&model, wx, wy, initial) {
                    Some(v) => v,
                    None => continue,
                };
                for band in 0..out_bands {
                    let val = match resample {
                        ResampleMethod::Nearest => sample_nearest(&source_view, band, sx0, sy0),
                        _ => sample_bilinear(&source_view, band, sx0, sy0),
                    };
                    buf[band * out_cols + col] = val;
                }
            }
            buf
        }).collect();

        for (row, row_buf) in row_buffers.into_iter().enumerate() {
            for band in 0..out_bands {
                for col in 0..out_cols {
                    let idx = band * out_rows * out_cols + row * out_cols + col;
                    out.data.set_f64(idx, row_buf[band * out_cols + col]);
                }
            }
        }

        let output_path = parse_optional_output_path(args, "output")?
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| {
                let mut p = PathBuf::from(&input_path);
                let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("georeferenced");
                p.set_file_name(format!("{stem}_georef.tif"));
                p.to_string_lossy().into_owned()
            });
        write_raster(&out, &output_path, "georeferenced")?;

        let report_path = parse_optional_output_path(args, "report")?
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| {
                let mut p = PathBuf::from(&output_path);
                p.set_extension("json");
                p.to_string_lossy().into_owned()
            });
        let report_csv_path = Path::new(&report_path)
            .with_extension("csv")
            .to_string_lossy()
            .into_owned();

        let requested_transform = transform_type.to_string();
        let applied_transform = match &model {
            TransformModel::Affine { .. } => "affine".to_string(),
            TransformModel::Projective { .. } => "projective".to_string(),
            TransformModel::Polynomial { order, .. } => format!("polynomial{order}"),
            TransformModel::ThinPlateSpline { .. } => "thin_plate_spline".to_string(),
        };
        let outlier_count = residuals.iter().filter(|r| r.radial_error > rmse * 3.0 && r.radial_error > 1e-9).count();
        if outlier_count > 0 {
            warnings.push(format!("{} GCP residuals are > 3x RMSE", outlier_count));
        }

        let report = ReportData {
            requested_transform,
            applied_transform: applied_transform.clone(),
            requested_order: transform_order,
            effective_order,
            gcp_count: gcps.len(),
            rmse,
            warnings: warnings.clone(),
            residuals: residuals.clone(),
            downgraded,
        };
        let report_json = json!({
            "tool_id": "georeference_raster_from_control_points",
            "requested_transform": report.requested_transform,
            "applied_transform": report.applied_transform,
            "requested_order": report.requested_order,
            "effective_order": report.effective_order,
            "gcp_count": report.gcp_count,
            "rmse": report.rmse,
            "downgraded": report.downgraded,
            "warnings": report.warnings,
            "residuals": report.residuals.iter().map(|r| json!({"index":r.index,"dx":r.dx,"dy":r.dy,"radial_error":r.radial_error})).collect::<Vec<_>>(),
            "output": output_path,
        });
        fs::write(&report_path, serde_json::to_string_pretty(&report_json).map_err(|e| ToolError::Execution(format!("failed serializing report JSON: {e}")))?)
            .map_err(|e| ToolError::Execution(format!("failed writing report JSON: {e}")))?;

        let csv_body = std::iter::once("index,dx,dy,radial_error".to_string())
            .chain(residuals.iter().map(|r| format!("{},{:.10},{:.10},{:.10}", r.index, r.dx, r.dy, r.radial_error)))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&report_csv_path, csv_body)
            .map_err(|e| ToolError::Execution(format!("failed writing report CSV: {e}")))?;

        if downgraded {
            warnings.push("automatic downgrade was applied to satisfy minimum GCP count".to_string());
        }

        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), json!(output_path));
        outputs.insert("report_json".to_string(), json!(report_path));
        outputs.insert("report_csv".to_string(), json!(report_csv_path));
        outputs.insert("gcp_count".to_string(), json!(gcps.len()));
        outputs.insert("rmse".to_string(), json!(rmse));
        outputs.insert("applied_transform".to_string(), json!(applied_transform));
        outputs.insert("warnings".to_string(), json!(warnings));

        Ok(ToolRunResult { outputs })
    }
}