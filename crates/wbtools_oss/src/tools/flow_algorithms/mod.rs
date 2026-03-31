use std::collections::BTreeMap;
use std::f64::consts::PI;

use serde_json::json;
use wbprojection::{identify_epsg_from_wkt_with_policy, Crs, EpsgIdentifyPolicy};
use wbcore::{
    parse_optional_output_path, parse_raster_path_arg, LicenseTier, Tool, ToolArgs, ToolCategory, ToolContext, ToolError,
    ToolExample, ToolManifest, ToolMetadata, ToolParamSpec, ToolRunResult, ToolStability,
};
use rand::RngExt;
use wbraster::{DataType, Raster, RasterFormat};

use crate::memory_store;

pub struct D8PointerTool;
pub struct D8FlowAccumTool;
pub struct DInfPointerTool;
pub struct DInfFlowAccumTool;
pub struct FD8PointerTool;
pub struct FD8FlowAccumTool;
pub struct Rho8PointerTool;
pub struct Rho8FlowAccumTool;
pub struct MDInfFlowAccumTool;
pub struct QinFlowAccumulationTool;
pub struct QuinnFlowAccumulationTool;
pub struct MinimalDispersionFlowAlgorithmTool;

const DX: [isize; 8] = [1, 1, 1, 0, -1, -1, -1, 0];
const DY: [isize; 8] = [-1, 0, 1, 1, 1, 0, -1, -1];
const INFLOWING_VALS: [i8; 8] = [4, 5, 6, 7, 0, 1, 2, 3];

fn in_bounds(r: isize, c: isize, rows: usize, cols: usize) -> bool {
    r >= 0 && c >= 0 && (r as usize) < rows && (c as usize) < cols
}

fn idx(r: usize, c: usize, cols: usize) -> usize {
    r * cols + c
}

fn load_raster(path: &str) -> Result<Raster, ToolError> {
    if memory_store::raster_is_memory_path(path) {
        let id = memory_store::raster_path_to_id(path)
            .ok_or_else(|| ToolError::Validation("malformed in-memory raster path".to_string()))?;
        return memory_store::get_raster_by_id(id)
            .ok_or_else(|| ToolError::Validation(format!("unknown in-memory raster id '{}'", id)));
    }
    Raster::read(path).map_err(|e| ToolError::Execution(format!("failed reading input raster: {}", e)))
}

fn write_or_store_output(output: Raster, output_path: Option<std::path::PathBuf>) -> Result<String, ToolError> {
    if let Some(output_path) = output_path {
        if let Some(parent) = output_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| ToolError::Execution(format!("failed creating output directory: {}", e)))?;
            }
        }
        let output_path_str = output_path.to_string_lossy().to_string();
        let output_format = RasterFormat::for_output_path(&output_path_str)
            .map_err(|e| ToolError::Validation(format!("unsupported output path: {}", e)))?;
        output
            .write(&output_path_str, output_format)
            .map_err(|e| ToolError::Execution(format!("failed writing output raster: {}", e)))?;
        Ok(output_path_str)
    } else {
        let id = memory_store::put_raster(output);
        Ok(memory_store::make_raster_memory_path(&id))
    }
}

fn build_result(path: String) -> ToolRunResult {
    let mut outputs = BTreeMap::new();
    outputs.insert("path".to_string(), json!(path));
    ToolRunResult {
        outputs,
        ..Default::default()
    }
}

fn typed_raster_output(path: String) -> serde_json::Value {
    json!({"__wbw_type__": "raster", "path": path, "active_band": 0})
}

fn build_dual_raster_result(flow_dir_path: String, flow_accum_path: String) -> ToolRunResult {
    let flow_dir = typed_raster_output(flow_dir_path);
    let flow_accum = typed_raster_output(flow_accum_path);
    let mut outputs = BTreeMap::new();
    outputs.insert("flow_dir".to_string(), flow_dir.clone());
    outputs.insert("flow_accum".to_string(), flow_accum.clone());
    outputs.insert("__wbw_type__".to_string(), json!("tuple"));
    outputs.insert("items".to_string(), json!([flow_dir, flow_accum]));
    ToolRunResult {
        outputs,
        ..Default::default()
    }
}

fn parse_input_and_output(args: &ToolArgs) -> Result<(Raster, Option<std::path::PathBuf>), ToolError> {
    let input_path = parse_raster_path_arg(args, "dem")
        .or_else(|_| parse_raster_path_arg(args, "raster"))
        .or_else(|_| parse_raster_path_arg(args, "input"))?;
    let output_path = parse_optional_output_path(args, "output")?;
    Ok((load_raster(&input_path)?, output_path))
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

fn haversine_distance_m(lat1_deg: f64, lon1_deg: f64, lat2_deg: f64, lon2_deg: f64) -> f64 {
    let r = 6_371_000.0_f64;
    let phi1 = lat1_deg.to_radians();
    let phi2 = lat2_deg.to_radians();
    let lambda1 = lon1_deg.to_radians();
    let lambda2 = lon2_deg.to_radians();
    let hav_delta_phi = (1.0 - (phi2 - phi1).cos()) / 2.0;
    let hav_delta_lambda = phi1.cos() * phi2.cos() * (1.0 - (lambda2 - lambda1).cos()) / 2.0;
    2.0 * r * (hav_delta_phi + hav_delta_lambda).sqrt().asin()
}

fn vincenty_distance_m(start: (f64, f64), end: (f64, f64)) -> f64 {
    let a: f64 = 6378137.0;
    let b: f64 = 6356752.314245;
    let f: f64 = 1.0 / 298.257223563;
    let p1 = (start.0.to_radians(), start.1.to_radians());
    let p2 = (end.0.to_radians(), end.1.to_radians());
    let l = p2.1 - p1.1;
    let (tan_u1, tan_u2) = ((1.0 - f) * p1.0.tan(), (1.0 - f) * p2.0.tan());
    let (cos_u1, cos_u2) = (
        1.0 / (1.0 + tan_u1 * tan_u1).sqrt(),
        1.0 / (1.0 + tan_u2 * tan_u2).sqrt(),
    );
    let (sin_u1, sin_u2) = (tan_u1 * cos_u1, tan_u2 * cos_u2);
    let mut lambda = l;
    let mut iter_limit = 100;
    let (cos_sq_alpha, sin_sigma, cos_sigma, cos2_sigma_m, sigma) = loop {
        let sin_lambda = lambda.sin();
        let cos_lambda = lambda.cos();
        let sin_sq_sigma = (cos_u2 * sin_lambda) * (cos_u2 * sin_lambda)
            + (cos_u1 * sin_u2 - sin_u1 * cos_u2 * cos_lambda)
                * (cos_u1 * sin_u2 - sin_u1 * cos_u2 * cos_lambda);
        if sin_sq_sigma == 0.0 {
            return 0.0;
        }
        let sin_sigma = sin_sq_sigma.sqrt();
        let cos_sigma = sin_u1 * sin_u2 + cos_u1 * cos_u2 * cos_lambda;
        let sigma = sin_sigma.atan2(cos_sigma);
        let sin_alpha = cos_u1 * cos_u2 * sin_lambda / sin_sigma;
        let cos_sq_alpha = 1.0 - sin_alpha * sin_alpha;
        let cos2_sigma_m = if cos_sq_alpha != 0.0 {
            cos_sigma - 2.0 * sin_u1 * sin_u2 / cos_sq_alpha
        } else {
            0.0
        };
        let c = f / 16.0 * cos_sq_alpha * (4.0 + f * (4.0 - 3.0 * cos_sq_alpha));
        let lambda_prime = lambda;
        lambda = l
            + (1.0 - c)
                * f
                * sin_alpha
                * (sigma
                    + c * sin_sigma
                        * (cos2_sigma_m + c * cos_sigma * (-1.0 + 2.0 * cos2_sigma_m * cos2_sigma_m)));
        iter_limit -= 1;
        if (lambda - lambda_prime).abs() <= 1e-12 || iter_limit <= 0 {
            break (cos_sq_alpha, sin_sigma, cos_sigma, cos2_sigma_m, sigma);
        }
    };
    if iter_limit <= 0 {
        return haversine_distance_m(start.0, start.1, end.0, end.1);
    }
    let u_sq = cos_sq_alpha * (a * a - b * b) / (b * b);
    let cap_a = 1.0 + u_sq / 16384.0 * (4096.0 + u_sq * (-768.0 + u_sq * (320.0 - 175.0 * u_sq)));
    let cap_b = u_sq / 1024.0 * (256.0 + u_sq * (-128.0 + u_sq * (74.0 - 47.0 * u_sq)));
    let delta_sigma = cap_b
        * sin_sigma
        * (cos2_sigma_m
            + cap_b / 4.0
                * (cos_sigma * (-1.0 + 2.0 * cos2_sigma_m * cos2_sigma_m)
                    - cap_b / 6.0
                        * cos2_sigma_m
                        * (-3.0 + 4.0 * sin_sigma * sin_sigma)
                        * (-3.0 + 4.0 * cos2_sigma_m * cos2_sigma_m)));
    b * cap_a * (sigma - delta_sigma)
}

fn should_use_haversine(input: &Raster) -> bool {
    let phi1 = input.row_center_y(0);
    let lambda1 = input.col_center_x(0);
    let phi2 = phi1;
    let lambda2 = input.col_center_x(1);
    let linear_res = vincenty_distance_m((phi1, lambda1), (phi2, lambda2));
    if linear_res == 0.0 {
        return true;
    }
    let linear_res2 = haversine_distance_m(phi1, lambda1, phi2, lambda2);
    let diff = 100.0 * (linear_res - linear_res2).abs() / linear_res;
    diff < 0.5
}

fn geo_distance_m(use_haversine: bool, start: (f64, f64), end: (f64, f64)) -> f64 {
    if use_haversine {
        haversine_distance_m(start.0, start.1, end.0, end.1)
    } else {
        vincenty_distance_m(start, end)
    }
}

fn d8_dir_from_dem(input: &Raster) -> Vec<i8> {
    let rows = input.rows;
    let cols = input.cols;
    let nodata = input.nodata;
    let cell_x = input.cell_size_x;
    let cell_y = input.cell_size_y;
    let diag = (cell_x * cell_x + cell_y * cell_y).sqrt();
    let lens = [diag, cell_x, diag, cell_y, diag, cell_x, diag, cell_y];
    let mut out = vec![-2i8; rows * cols];

    for r in 0..rows {
        for c in 0..cols {
            let i = idx(r, c, cols);
            let mut best_dir = -1i8;
            let mut best_slope = f64::MIN;
            for k in 0..8 {
                let rn = r as isize + DY[k];
                let cn = c as isize + DX[k];
                if !in_bounds(rn, cn, rows, cols) {
                    continue;
                }
                let z = input.get(0, rn, cn);
                let z0 = input.get(0, r as isize, c as isize);
                if z == nodata || z0 == nodata {
                    continue;
                }
                let slope = (z0 - z) / lens[k];
                if slope > best_slope {
                    best_slope = slope;
                    best_dir = k as i8;
                }
            }
            out[i] = best_dir;
        }
    }

    out
}

fn dinf_pointer_from_dem(input: &Raster) -> Vec<f64> {
    if raster_is_geographic(input) {
        return dinf_pointer_from_dem_geographic(input);
    }
    let rows = input.rows;
    let cols = input.cols;
    let nodata = input.nodata;
    let cell_x = input.cell_size_x;
    let cell_y = input.cell_size_y;
    let diag = (cell_x * cell_x + cell_y * cell_y).sqrt();
    let grid_res = (cell_x + cell_y) / 2.0;
    let ac_vals = [0.0, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 4.0];
    let af_vals = [1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
    let e1_col = [1, 0, 0, -1, -1, 0, 0, 1];
    let e1_row = [0, -1, -1, 0, 0, 1, 1, 0];
    let e2_col = [1, 1, -1, -1, -1, -1, 1, 1];
    let e2_row = [-1, -1, -1, -1, 1, 1, 1, 1];
    let atan_of_1 = 1.0_f64.atan();
    const HALF_PI: f64 = PI / 2.0;
    let mut out = vec![nodata; rows * cols];

    for r in 0..rows {
        for c in 0..cols {
            let i = idx(r, c, cols);
            let e0 = input.get(0, r as isize, c as isize);
            if e0 == nodata {
                continue;
            }
            let mut dir = 360.0;
            let mut max_slope = f64::MIN;
            for k in 0..8 {
                let rn1 = r as isize + e1_row[k];
                let cn1 = c as isize + e1_col[k];
                let rn2 = r as isize + e2_row[k];
                let cn2 = c as isize + e2_col[k];
                if !in_bounds(rn1, cn1, rows, cols) || !in_bounds(rn2, cn2, rows, cols) {
                    continue;
                }
                let e1 = input.get(0, rn1, cn1);
                let e2 = input.get(0, rn2, cn2);
                if e1 == nodata || e2 == nodata {
                    continue;
                }
                let ac = ac_vals[k];
                let af = af_vals[k];
                let mut s;
                let mut r_ang;
                if e0 > e1 && e0 > e2 {
                    let s1 = (e0 - e1) / grid_res;
                    let s2 = (e1 - e2) / grid_res;
                    r_ang = if s1 != 0.0 { (s2 / s1).atan() } else { PI / 2.0 };
                    s = (s1 * s1 + s2 * s2).sqrt();
                    if (s1 < 0.0 && s2 <= 0.0) || (s1 == 0.0 && s2 < 0.0) {
                        s *= -1.0;
                    }
                    if r_ang < 0.0 || r_ang > atan_of_1 {
                        if r_ang < 0.0 {
                            r_ang = 0.0;
                            s = s1;
                        } else {
                            r_ang = atan_of_1;
                            s = (e0 - e2) / diag;
                        }
                    }
                } else if e0 > e1 || e0 > e2 {
                    if e0 > e1 {
                        r_ang = 0.0;
                        s = (e0 - e1) / grid_res;
                    } else {
                        r_ang = atan_of_1;
                        s = (e0 - e2) / diag;
                    }
                } else {
                    continue;
                }
                if s >= max_slope && s != 0.00001 {
                    max_slope = s;
                    dir = af * r_ang + ac * HALF_PI;
                }
            }

            out[i] = if max_slope > 0.0 {
                let mut az = 360.0 - dir.to_degrees() + 90.0;
                if az > 360.0 {
                    az -= 360.0;
                }
                az
            } else {
                -1.0
            };
        }
    }

    out
}

fn dinf_pointer_from_dem_geographic(input: &Raster) -> Vec<f64> {
    let rows = input.rows;
    let cols = input.cols;
    let nodata = input.nodata;
    let use_haversine = should_use_haversine(input);
    let ac_vals = [0.0, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 4.0];
    let af_vals = [1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
    let e1_col = [1, 0, 0, -1, -1, 0, 0, 1];
    let e1_row = [0, -1, -1, 0, 0, 1, 1, 0];
    let e2_col = [1, 1, -1, -1, -1, -1, 1, 1];
    let e2_row = [-1, -1, -1, -1, 1, 1, 1, 1];
    let atan_of_1 = 1.0_f64.atan();
    const HALF_PI: f64 = PI / 2.0;
    let mut out = vec![nodata; rows * cols];

    for r in 0..rows {
        for c in 0..cols {
            let i = idx(r, c, cols);
            let e0 = input.get(0, r as isize, c as isize);
            if e0 == nodata {
                continue;
            }
            let phi0 = input.row_center_y(r as isize);
            let lambda0 = input.col_center_x(c as isize);
            let mut dir = 360.0;
            let mut max_slope = f64::MIN;

            for k in 0..8 {
                let rn1 = r as isize + e1_row[k];
                let cn1 = c as isize + e1_col[k];
                let rn2 = r as isize + e2_row[k];
                let cn2 = c as isize + e2_col[k];
                if !in_bounds(rn1, cn1, rows, cols) || !in_bounds(rn2, cn2, rows, cols) {
                    continue;
                }
                let e1 = input.get(0, rn1, cn1);
                let e2 = input.get(0, rn2, cn2);
                if e1 == nodata || e2 == nodata {
                    continue;
                }

                let phi1 = input.row_center_y(rn1);
                let lambda1 = input.col_center_x(cn1);
                let phi2 = input.row_center_y(rn2);
                let lambda2 = input.col_center_x(cn2);

                let ac = ac_vals[k];
                let af = af_vals[k];
                let mut s;
                let mut r_ang;
                if e0 > e1 && e0 > e2 {
                    let grid_res = geo_distance_m(use_haversine, (phi1, lambda1), (phi2, lambda2));
                    let s2 = (e1 - e2) / grid_res;
                    let grid_res = geo_distance_m(use_haversine, (phi0, lambda0), (phi1, lambda1));
                    let s1 = (e0 - e1) / grid_res;
                    r_ang = if s1 != 0.0 { (s2 / s1).atan() } else { PI / 2.0 };
                    s = (s1 * s1 + s2 * s2).sqrt();
                    if (s1 < 0.0 && s2 <= 0.0) || (s1 == 0.0 && s2 < 0.0) {
                        s *= -1.0;
                    }
                    if r_ang < 0.0 || r_ang > atan_of_1 {
                        if r_ang < 0.0 {
                            r_ang = 0.0;
                            s = s1;
                        } else {
                            r_ang = atan_of_1;
                            let diag = geo_distance_m(use_haversine, (phi0, lambda0), (phi2, lambda2));
                            s = (e0 - e2) / diag;
                        }
                    }
                } else if e0 > e1 || e0 > e2 {
                    if e0 > e1 {
                        r_ang = 0.0;
                        let grid_res = geo_distance_m(use_haversine, (phi0, lambda0), (phi1, lambda1));
                        s = (e0 - e1) / grid_res;
                    } else {
                        r_ang = atan_of_1;
                        let diag = geo_distance_m(use_haversine, (phi0, lambda0), (phi2, lambda2));
                        s = (e0 - e2) / diag;
                    }
                } else {
                    continue;
                }

                if s >= max_slope && s != 0.00001 {
                    max_slope = s;
                    dir = af * r_ang + ac * HALF_PI;
                }
            }

            out[i] = if max_slope > 0.0 {
                let mut az = 360.0 - dir.to_degrees() + 90.0;
                if az > 360.0 {
                    az -= 360.0;
                }
                az
            } else {
                -1.0
            };
        }
    }

    out
}

fn dinf_inflow_count(flow_dir: &[f64], rows: usize, cols: usize, nodata: f64) -> Vec<i8> {
    let start_fd = [180.0, 225.0, 270.0, 315.0, 0.0, 45.0, 90.0, 135.0];
    let end_fd = [270.0, 315.0, 360.0, 45.0, 90.0, 135.0, 180.0, 225.0];
    let mut inflow = vec![-1i8; rows * cols];
    for r in 0..rows {
        for c in 0..cols {
            let i = idx(r, c, cols);
            let dir = flow_dir[i];
            if dir == nodata {
                continue;
            }
            let mut count = 0i8;
            for k in 0..8 {
                let rn = r as isize + DY[k];
                let cn = c as isize + DX[k];
                if !in_bounds(rn, cn, rows, cols) {
                    continue;
                }
                let n_dir = flow_dir[idx(rn as usize, cn as usize, cols)];
                if n_dir >= 0.0 {
                    if k != 3 {
                        if n_dir > start_fd[k] && n_dir < end_fd[k] {
                            count += 1;
                        }
                    } else if n_dir > start_fd[k] || n_dir < end_fd[k] {
                        count += 1;
                    }
                }
            }
            inflow[i] = count;
        }
    }
    inflow
}

fn dinf_flow_accum_core(
    flow_dir: &[f64],
    rows: usize,
    cols: usize,
    nodata: f64,
    convergence_threshold: f64,
) -> Vec<f64> {
    let n = rows * cols;
    let mut out = vec![nodata; n];
    let mut inflow = dinf_inflow_count(flow_dir, rows, cols, nodata);
    let mut stack = Vec::<usize>::with_capacity(n);

    for i in 0..n {
        if flow_dir[i] != nodata {
            out[i] = 1.0;
            if inflow[i] == 0 {
                stack.push(i);
            }
        }
    }

    while let Some(i) = stack.pop() {
        let dir = flow_dir[i];
        inflow[i] = -1;
        if dir >= 0.0 {
            let r = i / cols;
            let c = i % cols;
            let (mut p1, mut p2, a1, b1, a2, b2) = if dir < 45.0 {
                ((45.0 - dir) / 45.0, dir / 45.0, c as isize, r as isize - 1, c as isize + 1, r as isize - 1)
            } else if dir < 90.0 {
                ((90.0 - dir) / 45.0, (dir - 45.0) / 45.0, c as isize + 1, r as isize - 1, c as isize + 1, r as isize)
            } else if dir < 135.0 {
                ((135.0 - dir) / 45.0, (dir - 90.0) / 45.0, c as isize + 1, r as isize, c as isize + 1, r as isize + 1)
            } else if dir < 180.0 {
                ((180.0 - dir) / 45.0, (dir - 135.0) / 45.0, c as isize + 1, r as isize + 1, c as isize, r as isize + 1)
            } else if dir < 225.0 {
                ((225.0 - dir) / 45.0, (dir - 180.0) / 45.0, c as isize, r as isize + 1, c as isize - 1, r as isize + 1)
            } else if dir < 270.0 {
                ((270.0 - dir) / 45.0, (dir - 225.0) / 45.0, c as isize - 1, r as isize + 1, c as isize - 1, r as isize)
            } else if dir < 315.0 {
                ((315.0 - dir) / 45.0, (dir - 270.0) / 45.0, c as isize - 1, r as isize, c as isize - 1, r as isize - 1)
            } else {
                ((360.0 - dir) / 45.0, (dir - 315.0) / 45.0, c as isize - 1, r as isize - 1, c as isize, r as isize - 1)
            };

            if out[i] >= convergence_threshold {
                if p1 >= p2 {
                    p1 = 1.0;
                    p2 = 0.0;
                    if in_bounds(b2, a2, rows, cols) {
                        let n2 = idx(b2 as usize, a2 as usize, cols);
                        if inflow[n2] > 0 {
                            inflow[n2] -= 1;
                            if inflow[n2] == 0 {
                                stack.push(n2);
                            }
                        }
                    }
                } else {
                    p1 = 0.0;
                    p2 = 1.0;
                    if in_bounds(b1, a1, rows, cols) {
                        let n1 = idx(b1 as usize, a1 as usize, cols);
                        if inflow[n1] > 0 {
                            inflow[n1] -= 1;
                            if inflow[n1] == 0 {
                                stack.push(n1);
                            }
                        }
                    }
                }
            }

            if p1 > 0.0 && in_bounds(b1, a1, rows, cols) {
                let n1 = idx(b1 as usize, a1 as usize, cols);
                if out[n1] != nodata {
                    out[n1] += out[i] * p1;
                    if inflow[n1] > 0 {
                        inflow[n1] -= 1;
                        if inflow[n1] == 0 {
                            stack.push(n1);
                        }
                    }
                }
            }
            if p2 > 0.0 && in_bounds(b2, a2, rows, cols) {
                let n2 = idx(b2 as usize, a2 as usize, cols);
                if out[n2] != nodata {
                    out[n2] += out[i] * p2;
                    if inflow[n2] > 0 {
                        inflow[n2] -= 1;
                        if inflow[n2] == 0 {
                            stack.push(n2);
                        }
                    }
                }
            }
        }
    }

    out
}

fn apply_dinf_output_type(
    accum: &mut [f64],
    input: &Raster,
    out_type: &str,
    log_transform: bool,
) {
    if !raster_is_geographic(input) || out_type == "cells" {
        let mut area = input.cell_size_x * input.cell_size_y;
        let mut grid_size = (input.cell_size_x + input.cell_size_y) / 2.0;
        if out_type == "cells" {
            area = 1.0;
            grid_size = 1.0;
        } else if out_type == "ca" {
            grid_size = 1.0;
        }
        for v in accum.iter_mut() {
            if *v == input.nodata || *v == -32768.0 {
                *v = input.nodata;
            } else {
                let scaled = *v * area / grid_size;
                *v = if log_transform { scaled.ln() } else { scaled };
            }
        }
        return;
    }

    let use_haversine = should_use_haversine(input);
    for r in 0..input.rows {
        for c in 0..input.cols {
            let i = idx(r, c, input.cols);
            if input.get(0, r as isize, c as isize) == input.nodata || accum[i] == -32768.0 {
                accum[i] = input.nodata;
                continue;
            }
            let phi1 = input.row_center_y(r as isize);
            let lambda1 = input.col_center_x(c as isize - 1);
            let phi2 = phi1;
            let lambda2 = input.col_center_x(c as isize + 1);
            let grid_length_x = geo_distance_m(use_haversine, (phi1, lambda1), (phi2, lambda2)) / 2.0;

            let phi1 = input.row_center_y(r as isize - 1);
            let lambda1 = input.col_center_x(c as isize);
            let phi2 = input.row_center_y(r as isize + 1);
            let lambda2 = lambda1;
            let grid_length_y = geo_distance_m(use_haversine, (phi1, lambda1), (phi2, lambda2)) / 2.0;

            let cell_area = grid_length_x * grid_length_y;
            let mut avg_cell_size = (grid_length_x + grid_length_y) / 2.0;
            if out_type == "ca" {
                avg_cell_size = 1.0;
            }
            let scaled = accum[i] * cell_area / avg_cell_size;
            accum[i] = if log_transform { scaled.ln() } else { scaled };
        }
    }
}

fn fd8_pointer_from_dem(input: &Raster) -> Vec<f64> {
    let rows = input.rows;
    let cols = input.cols;
    let nodata = input.nodata;
    let out_nodata = -32768.0;
    let mut out = vec![out_nodata; rows * cols];

    for r in 0..rows {
        for c in 0..cols {
            let i = idx(r, c, cols);
            let z = input.get(0, r as isize, c as isize);
            if z == nodata {
                continue;
            }
            let mut dir = 0.0;
            for n in 0..8 {
                let zn = input.get(0, r as isize + DY[n], c as isize + DX[n]);
                if zn != nodata && zn < z {
                    dir += (1u16 << n) as f64;
                }
            }
            out[i] = dir;
        }
    }

    out
}

fn fd8_inflow_count(input: &Raster) -> Vec<i8> {
    let rows = input.rows;
    let cols = input.cols;
    let nodata = input.nodata;
    let mut inflow = vec![-1i8; rows * cols];

    for r in 0..rows {
        for c in 0..cols {
            let i = idx(r, c, cols);
            let z = input.get(0, r as isize, c as isize);
            if z == nodata {
                continue;
            }
            let mut count = 0i8;
            for n in 0..8 {
                let zn = input.get(0, r as isize + DY[n], c as isize + DX[n]);
                if zn != nodata && zn > z {
                    count += 1;
                }
            }
            inflow[i] = count;
        }
    }

    inflow
}

fn fd8_flow_accum_core(input: &Raster, exponent: f64, convergence_threshold: f64) -> Vec<f64> {
    let rows = input.rows;
    let cols = input.cols;
    let nodata = input.nodata;
    let n = rows * cols;
    let mut out = vec![nodata; n];
    let mut inflow = fd8_inflow_count(input);
    let mut stack = Vec::<usize>::with_capacity(n);

    for i in 0..n {
        if inflow[i] >= 0 {
            out[i] = 1.0;
            if inflow[i] == 0 {
                stack.push(i);
            }
        }
    }

    let is_geo = raster_is_geographic(input);
    let use_haversine = if is_geo { should_use_haversine(input) } else { false };
    let cell_x = input.cell_size_x;
    let cell_y = input.cell_size_y;
    let diag = (cell_x * cell_x + cell_y * cell_y).sqrt();
    let lens = [diag, cell_x, diag, cell_y, diag, cell_x, diag, cell_y];

    while let Some(i) = stack.pop() {
        let r = i / cols;
        let c = i % cols;
        let z = input.get(0, r as isize, c as isize);
        let fa = out[i];
        inflow[i] = -1;

        let mut total_weights = 0.0;
        let mut weights = [0.0; 8];
        let mut downslope = [false; 8];

        if fa < convergence_threshold {
            for n in 0..8 {
                let rn = r as isize + DY[n];
                let cn = c as isize + DX[n];
                if !in_bounds(rn, cn, rows, cols) {
                    continue;
                }
                let zn = input.get(0, rn, cn);
                if zn == nodata || zn >= z {
                    continue;
                }
                let slope = if is_geo {
                    let start = (input.row_center_y(r as isize), input.col_center_x(c as isize));
                    let end = (input.row_center_y(rn), input.col_center_x(cn));
                    (z - zn) / geo_distance_m(use_haversine, start, end)
                } else {
                    (z - zn) / lens[n]
                };
                weights[n] = slope.powf(exponent);
                total_weights += weights[n];
                downslope[n] = true;
            }
        } else {
            let mut dir = 0usize;
            let mut max_slope = f64::MIN;
            for n in 0..8 {
                let rn = r as isize + DY[n];
                let cn = c as isize + DX[n];
                if !in_bounds(rn, cn, rows, cols) {
                    continue;
                }
                let zn = input.get(0, rn, cn);
                if zn == nodata {
                    continue;
                }
                let slope = if is_geo {
                    let start = (input.row_center_y(r as isize), input.col_center_x(c as isize));
                    let end = (input.row_center_y(rn), input.col_center_x(cn));
                    (z - zn) / geo_distance_m(use_haversine, start, end)
                } else {
                    (z - zn) / lens[n]
                };
                if slope > 0.0 {
                    downslope[n] = true;
                    if slope > max_slope {
                        max_slope = slope;
                        dir = n;
                    }
                }
            }
            if max_slope >= 0.0 {
                weights[dir] = 1.0;
                total_weights = 1.0;
            }
        }

        if total_weights > 0.0 {
            for n in 0..8 {
                if !downslope[n] {
                    continue;
                }
                let rn = r as isize + DY[n];
                let cn = c as isize + DX[n];
                let ni = idx(rn as usize, cn as usize, cols);
                out[ni] += fa * (weights[n] / total_weights);
                if inflow[ni] > 0 {
                    inflow[ni] -= 1;
                    if inflow[ni] == 0 {
                        stack.push(ni);
                    }
                }
            }
        }
    }

    out
}

/// Counts neighbours with strictly higher elevation for MDInf topo-sort seeding.
/// Returns -1 for nodata cells.
fn mdinf_inflow_count(input: &Raster) -> Vec<i8> {
    let rows = input.rows;
    let cols = input.cols;
    let nodata = input.nodata;
    // MDInf facet vertex order: N, NW, W, SW, S, SE, E, NE
    let xd: [isize; 8] = [0, -1, -1, -1, 0, 1, 1, 1];
    let yd: [isize; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];
    let mut out = vec![-1i8; rows * cols];
    for r in 0..rows {
        for c in 0..cols {
            let i = idx(r, c, cols);
            let z = input.get(0, r as isize, c as isize);
            if z == nodata {
                continue;
            }
            let mut count = 0i8;
            for k in 0..8 {
                let rn = r as isize + yd[k];
                let cn = c as isize + xd[k];
                if in_bounds(rn, cn, rows, cols) {
                    let zn = input.get(0, rn, cn);
                    if zn != nodata && zn > z {
                        count += 1;
                    }
                }
            }
            out[i] = count;
        }
    }
    out
}

/// MD-Infinity flow accumulation core (Seibert & McGlynn 2007).
/// Returns per-cell accumulated values (nodata cells keep the raster nodata value).
fn mdinf_flow_accum_core(input: &Raster, exponent: f64, convergence_threshold: f64) -> Vec<f64> {
    let rows = input.rows;
    let cols = input.cols;
    let nodata = input.nodata;
    let cell_x = input.cell_size_x;
    let cell_y = input.cell_size_y;
    let grid_res = (cell_x + cell_y) / 2.0;
    let diag = (cell_x * cell_x + cell_y * cell_y).sqrt();

    // MDInf facet vertex offsets: N, NW, W, SW, S, SE, E, NE
    let xd: [isize; 8] = [0, -1, -1, -1, 0, 1, 1, 1];
    let yd: [isize; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];
    // Distance multipliers: 1 for cardinal vertices (k=0,2,4,6), sqrt(2) for diagonal (k=1,3,5,7)
    let dd = [1.0_f64, std::f64::consts::SQRT_2, 1.0, std::f64::consts::SQRT_2,
              1.0, std::f64::consts::SQRT_2, 1.0, std::f64::consts::SQRT_2];
    // Grid lengths for D8-like convergent fallback (DX/DY = NE,E,SE,S,SW,W,NW,N)
    let grid_lengths = [diag, cell_x, diag, cell_y, diag, cell_x, diag, cell_y];
    let quarter_pi = PI / 4.0;

    let n = rows * cols;
    let mut out = vec![nodata; n];
    let mut inflow = mdinf_inflow_count(input);
    let mut stack = Vec::<usize>::with_capacity(n);

    for i in 0..n {
        if inflow[i] >= 0 {
            out[i] = 1.0;
            if inflow[i] == 0 {
                stack.push(i);
            }
        }
    }

    while let Some(i) = stack.pop() {
        let r = i / cols;
        let c = i % cols;
        let z = input.get(0, r as isize, c as isize);
        let fa = out[i];
        inflow[i] = -1;

        let mut weights = [0.0f64; 8];
        let mut downslope = [false; 8];

        if fa < convergence_threshold {
            // Dispersive MDInf triangular-facet routing
            let mut r_facet = [0.0f64; 8];
            let mut s_facet = [nodata; 8];

            // Compute flow direction and slope for each of the 8 triangular facets
            for fac in 0..8usize {
                let ii = (fac + 1) % 8;
                let p1 = input.get(0, r as isize + yd[fac], c as isize + xd[fac]);
                let p2 = input.get(0, r as isize + yd[ii], c as isize + xd[ii]);

                if p1 < z && p1 != nodata {
                    downslope[fac] = true;
                }

                if p1 != nodata && p2 != nodata {
                    let z1 = p1 - z;
                    let z2 = p2 - z;
                    let nx = (yd[fac] as f64 * z2 - yd[ii] as f64 * z1) * grid_res;
                    let ny = (xd[ii] as f64 * z1 - xd[fac] as f64 * z2) * grid_res;
                    let nz = (xd[fac] * yd[ii] - xd[ii] * yd[fac]) as f64 * grid_res * grid_res;

                    let hr = if nx == 0.0 {
                        if ny >= 0.0 { 0.0 } else { PI }
                    } else if nx >= 0.0 {
                        PI / 2.0 - (ny / nx).atan()
                    } else {
                        3.0 * PI / 2.0 - (ny / nx).atan()
                    };

                    let hs = {
                        let denom_sq = nx * nx + ny * ny + nz * nz;
                        if denom_sq == 0.0 { 0.0 } else { -(nz / denom_sq.sqrt()).acos().tan() }
                    };

                    // Clip hr to facet bounds if it falls outside [fac*pi/4, (fac+1)*pi/4]
                    let (hr, hs) = if hr < fac as f64 * quarter_pi
                        || hr > (fac + 1) as f64 * quarter_pi
                    {
                        if p1 < p2 {
                            (fac as f64 * quarter_pi, (z - p1) / (dd[fac] * grid_res))
                        } else {
                            (ii as f64 * quarter_pi, (z - p2) / (dd[ii] * grid_res))
                        }
                    } else {
                        (hr, hs)
                    };

                    r_facet[fac] = hr;
                    s_facet[fac] = hs;
                } else if p1 != nodata && p1 < z {
                    // p2 is nodata; use cardinal direction toward p1
                    r_facet[fac] = fac as f64 / 4.0 * PI;
                    s_facet[fac] = (z - p1) / (dd[ii] * grid_res);
                }
            }

            // Compute valley magnitudes from facet slopes
            let mut valley = [0.0f64; 8];
            let mut valley_sum = 0.0f64;
            let mut valley_max = 0.0f64;
            let mut i_max = 0usize;

            for fac in 0..8usize {
                let ii = (fac + 1) % 8;
                let prev = (fac + 7) % 8;
                if s_facet[fac] > 0.0 {
                    valley[fac] = if r_facet[fac] > fac as f64 * quarter_pi
                        && r_facet[fac] < (fac + 1) as f64 * quarter_pi
                    {
                        s_facet[fac] // direction inside facet
                    } else if r_facet[fac] == r_facet[ii] {
                        s_facet[fac] // adjacent facets share direction
                    } else if s_facet[ii] == nodata
                        && r_facet[fac] == (fac + 1) as f64 * quarter_pi
                    {
                        s_facet[fac] // direction on upper boundary; neighbour facet is nodata
                    } else if s_facet[prev] == nodata
                        && r_facet[fac] == fac as f64 * quarter_pi
                    {
                        s_facet[fac] // direction on lower boundary; neighbour facet is nodata
                    } else {
                        0.0
                    };
                }

                if exponent != 1.0 {
                    valley[fac] = valley[fac].powf(exponent);
                }
                valley_sum += valley[fac];
                if valley_max < valley[fac] {
                    i_max = fac;
                    valley_max = valley[fac];
                }
            }

            // Normalise and compute per-neighbour weights via angular interpolation
            if valley_sum > 0.0 {
                if exponent < 10.0 {
                    for fac in 0..8 {
                        valley[fac] /= valley_sum;
                        weights[fac] = 0.0;
                    }
                } else {
                    for fac in 0..8 {
                        valley[fac] = if fac == i_max { 1.0 } else { 0.0 };
                        weights[fac] = 0.0;
                    }
                }

                // Facet 7 spans [7*pi/4, 2*pi]; if its direction was recorded as 0
                // (the wrap-around of 2*pi) adjust to 2*pi to keep weights non-negative.
                if r_facet[7] == 0.0 {
                    r_facet[7] = 2.0 * PI;
                }

                for fac in 0..8usize {
                    let ii = (fac + 1) % 8;
                    if valley[fac] > 0.0 {
                        weights[fac] += valley[fac]
                            * ((fac + 1) as f64 * quarter_pi - r_facet[fac])
                            / quarter_pi;
                        weights[ii] += valley[fac]
                            * (r_facet[fac] - fac as f64 * quarter_pi)
                            / quarter_pi;
                    }
                }
            }

            // Propagate accumulated value to downslope XD/YD neighbours
            for k in 0..8 {
                if downslope[k] {
                    let rn = r as isize + yd[k];
                    let cn = c as isize + xd[k];
                    let ni = idx(rn as usize, cn as usize, cols);
                    if weights[k] > 0.0 {
                        out[ni] += fa * weights[k];
                    }
                    if inflow[ni] > 0 {
                        inflow[ni] -= 1;
                        if inflow[ni] == 0 {
                            stack.push(ni);
                        }
                    }
                }
            }
        } else {
            // Convergent (D8-like) routing above convergence threshold
            let mut best_dir = 0usize;
            let mut max_slope = f64::MIN;
            let mut total_w = 0.0f64;
            for k in 0..8 {
                let rn = r as isize + DY[k];
                let cn = c as isize + DX[k];
                if !in_bounds(rn, cn, rows, cols) {
                    continue;
                }
                let zn = input.get(0, rn, cn);
                if zn == nodata {
                    continue;
                }
                let slope = (z - zn) / grid_lengths[k];
                if slope > 0.0 {
                    downslope[k] = true;
                    if slope > max_slope {
                        max_slope = slope;
                        best_dir = k;
                    }
                }
            }
            if max_slope >= 0.0 {
                weights[best_dir] = 1.0;
                total_w = 1.0;
            }
            for k in 0..8 {
                if downslope[k] {
                    let rn = r as isize + DY[k];
                    let cn = c as isize + DX[k];
                    let ni = idx(rn as usize, cn as usize, cols);
                    if total_w > 0.0 {
                        out[ni] += fa * (weights[k] / total_w);
                    }
                    if inflow[ni] > 0 {
                        inflow[ni] -= 1;
                        if inflow[ni] == 0 {
                            stack.push(ni);
                        }
                    }
                }
            }
        }
    }

    out
}

fn minimal_dispersion_core(input: &Raster, p: f64, out_type: &str, esri_style: bool) -> (Vec<u16>, Vec<f64>, Vec<u16>) {
    let rows = input.rows;
    let cols = input.cols;
    let n = rows * cols;
    let nodata = input.nodata;
    let is_geo = raster_is_geographic(input);
    let use_haversine = if is_geo { should_use_haversine(input) } else { false };

    // Step 1: calculate D8 pointer and D-infinity direction.
    let mut d8_flow_ptr = vec![0u16; n];
    let mut interior_pit_found = false;
    let out_vals: [u16; 8] = if esri_style {
        [128, 1, 2, 4, 8, 16, 32, 64]
    } else {
        [1, 2, 4, 8, 16, 32, 64, 128]
    };

    let cell_x = input.cell_size_x;
    let cell_y = input.cell_size_y;
    let diag = (cell_x * cell_x + cell_y * cell_y).sqrt();
    let lens = [diag, cell_x, diag, cell_y, diag, cell_x, diag, cell_y];

    for r in 0..rows {
        for c in 0..cols {
            let i = idx(r, c, cols);
            let z = input.get(0, r as isize, c as isize);
            if z == nodata {
                continue;
            }

            let mut best = -1i8;
            let mut max_slope = f64::MIN;
            let mut neighbouring_nodata = false;
            for k in 0..8 {
                let rn = r as isize + DY[k];
                let cn = c as isize + DX[k];
                if !in_bounds(rn, cn, rows, cols) {
                    neighbouring_nodata = true;
                    continue;
                }
                let zn = input.get(0, rn, cn);
                if zn == nodata {
                    neighbouring_nodata = true;
                    continue;
                }
                let slope = if is_geo {
                    let start = (input.row_center_y(r as isize), input.col_center_x(c as isize));
                    let end = (input.row_center_y(rn), input.col_center_x(cn));
                    let d = geo_distance_m(use_haversine, start, end);
                    (z - zn) / d
                } else {
                    (z - zn) / lens[k]
                };
                if slope > max_slope && slope > 0.0 {
                    max_slope = slope;
                    best = k as i8;
                }
            }

            if best >= 0 {
                d8_flow_ptr[i] = out_vals[best as usize];
            } else if !neighbouring_nodata {
                interior_pit_found = true;
            }
        }
    }

    let dinf_flow_dir = if is_geo {
        dinf_pointer_from_dem_geographic(input)
    } else {
        dinf_pointer_from_dem(input)
    };

    // Step 2: find source cells and perform path-correction on D8 pointers.
    let inflowing_vals_u16: [u16; 8] = if esri_style {
        [8, 16, 32, 64, 128, 1, 2, 4]
    } else {
        [16, 32, 64, 128, 1, 2, 4, 8]
    };
    let dist = [diag, cell_x, diag, cell_y, diag, cell_x, diag, cell_y];
    let mut stack = Vec::<(isize, isize, f64)>::with_capacity(n);
    let mut sources = Vec::<(isize, isize, f64)>::with_capacity(n);
    for r in 0..rows {
        for c in 0..cols {
            if input.get(0, r as isize, c as isize) == nodata {
                continue;
            }
            if d8_flow_ptr[idx(r, c, cols)] == 0 {
                stack.push((r as isize, c as isize, 0.0));
            }
        }
    }

    while let Some((row, col, cur_dist)) = stack.pop() {
        let mut num_inflowing = 0i8;
        for k in 0..8 {
            let rn = row + DY[k];
            let cn = col + DX[k];
            if !in_bounds(rn, cn, rows, cols) {
                continue;
            }
            let ni = idx(rn as usize, cn as usize, cols);
            if d8_flow_ptr[ni] == inflowing_vals_u16[k] {
                num_inflowing += 1;
                stack.push((rn, cn, cur_dist + dist[k]));
            }
        }
        if num_inflowing == 0 {
            sources.push((row, col, cur_dist));
        }
    }
    sources.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    let left: [u16; 8] = [128, 1, 2, 4, 8, 16, 32, 64];
    let right: [u16; 8] = [2, 4, 8, 16, 32, 64, 128, 1];
    let d8_alphas = [45.0, 90.0, 135.0, 180.0, 225.0, 270.0, 315.0, 360.0];
    let mut visited = vec![-1i8; n];
    let mut pntr_matches = [0usize; 129];
    if !esri_style {
        pntr_matches[1] = 0;
        pntr_matches[2] = 1;
        pntr_matches[4] = 2;
        pntr_matches[8] = 3;
        pntr_matches[16] = 4;
        pntr_matches[32] = 5;
        pntr_matches[64] = 6;
        pntr_matches[128] = 7;
    } else {
        pntr_matches[1] = 1;
        pntr_matches[2] = 2;
        pntr_matches[4] = 3;
        pntr_matches[8] = 4;
        pntr_matches[16] = 5;
        pntr_matches[32] = 6;
        pntr_matches[64] = 7;
        pntr_matches[128] = 0;
    }

    while let Some((row, col, _)) = sources.pop() {
        let i = idx(row as usize, col as usize, cols);
        let mut fd = d8_flow_ptr[i];
        if fd == 0 {
            continue;
        }
        let mut row_n = row;
        let mut col_n = col;
        let mut accumulated_deflection = 0.0;
        loop {
            if !in_bounds(row_n, col_n, rows, cols) {
                break;
            }
            let ii = idx(row_n as usize, col_n as usize, cols);
            if visited[ii] == 1 {
                break;
            }
            visited[ii] = 1;
            let z = input.get(0, row_n, col_n);

            fd = d8_flow_ptr[ii];
            if fd == 0 {
                break;
            }
            let mut n_dir = pntr_matches[fd as usize];
            let alpha1 = dinf_flow_dir[ii];
            let alpha2 = d8_alphas[n_dir];
            let diff = (alpha1 - alpha2 + 180.0) % 360.0 - 180.0;
            accumulated_deflection += diff.to_radians().tan() * dist[n_dir];

            if accumulated_deflection >= cell_x {
                let fd_new = right[n_dir];
                let n_new = pntr_matches[fd_new as usize];
                let test_r = row_n + DY[n_new];
                let test_c = col_n + DX[n_new];
                if in_bounds(test_r, test_c, rows, cols) {
                    let zn = input.get(0, test_r, test_c);
                    if zn <= z {
                        let old_r = row_n + DY[n_dir];
                        let old_c = col_n + DX[n_dir];
                        if in_bounds(old_r, old_c, rows, cols) {
                            sources.push((old_r, old_c, 0.0));
                        }
                        d8_flow_ptr[ii] = fd_new;
                        n_dir = n_new;
                        accumulated_deflection -= cell_x;
                    }
                }
            } else if accumulated_deflection <= -cell_x {
                let fd_new = left[n_dir];
                let n_new = pntr_matches[fd_new as usize];
                let test_r = row_n + DY[n_new];
                let test_c = col_n + DX[n_new];
                if in_bounds(test_r, test_c, rows, cols) {
                    let zn = input.get(0, test_r, test_c);
                    if zn <= z {
                        let old_r = row_n + DY[n_dir];
                        let old_c = col_n + DX[n_dir];
                        if in_bounds(old_r, old_c, rows, cols) {
                            sources.push((old_r, old_c, 0.0));
                        }
                        d8_flow_ptr[ii] = fd_new;
                        n_dir = n_new;
                        accumulated_deflection += cell_x;
                    }
                }
            }

            row_n += DY[n_dir];
            col_n += DX[n_dir];
            if !in_bounds(row_n, col_n, rows, cols) || input.get(0, row_n, col_n) == nodata {
                break;
            }
        }
    }

    // Step 3: introduce minimal dispersion by resolving artifact sources.
    let mut pntr_modified = d8_flow_ptr.clone();
    let mut inflowing_count = vec![-1i16; n];
    let mut accum_stack = Vec::<usize>::with_capacity(n);
    for r in 0..rows {
        for c in 0..cols {
            let i = idx(r, c, cols);
            let z = input.get(0, r as isize, c as isize);
            if z == nodata {
                continue;
            }
            let mut highest_neighbour = f64::NEG_INFINITY;
            let mut highest_idx = 0usize;
            let mut num_inflowing = 0i16;
            for k in 0..8 {
                let rn = r as isize + DY[k];
                let cn = c as isize + DX[k];
                if in_bounds(rn, cn, rows, cols) {
                    let ni = idx(rn as usize, cn as usize, cols);
                    if d8_flow_ptr[ni] == inflowing_vals_u16[k] {
                        num_inflowing += 1;
                    }
                    let zn = input.get(0, rn, cn);
                    if zn > highest_neighbour {
                        highest_neighbour = zn;
                        highest_idx = k;
                    }
                }
            }
            inflowing_count[i] = num_inflowing;
            if num_inflowing == 0 {
                if highest_neighbour > z && p < 1.0 {
                    let rn = r as isize + DY[highest_idx];
                    let cn = c as isize + DX[highest_idx];
                    if in_bounds(rn, cn, rows, cols) {
                        let ni = idx(rn as usize, cn as usize, cols);
                        pntr_modified[ni] |= inflowing_vals_u16[highest_idx];
                        inflowing_count[i] = 1;
                    }
                } else {
                    accum_stack.push(i);
                }
            }
        }
    }

    // Step 4: flow accumulation through topological traversal.
    let mut accum = vec![nodata; n];
    if out_type == "cells" {
        for r in 0..rows {
            for c in 0..cols {
                let i = idx(r, c, cols);
                if input.get(0, r as isize, c as isize) != nodata {
                    accum[i] = 1.0;
                }
            }
        }
    } else if !is_geo {
        let cell_area = cell_x * cell_y;
        for r in 0..rows {
            for c in 0..cols {
                let i = idx(r, c, cols);
                if input.get(0, r as isize, c as isize) != nodata {
                    accum[i] = cell_area;
                }
            }
        }
    } else {
        for r in 0..rows {
            for c in 0..cols {
                let i = idx(r, c, cols);
                if input.get(0, r as isize, c as isize) == nodata {
                    continue;
                }
                let phi0 = input.row_center_y(r as isize);
                let lambda0 = input.col_center_x(c as isize);
                let gx = geo_distance_m(
                    use_haversine,
                    (phi0, lambda0),
                    (phi0, input.col_center_x(c as isize + 1)),
                );
                let gy = geo_distance_m(
                    use_haversine,
                    (phi0, lambda0),
                    (input.row_center_y(r as isize + 1), lambda0),
                );
                accum[i] = if out_type == "cells" { 1.0 } else { gx * gy };
            }
        }
    }

    let flow_directions: [u16; 8] = [1, 2, 4, 8, 16, 32, 64, 128];
    while let Some(i) = accum_stack.pop() {
        let fd = pntr_modified[i];
        if fd == 0 {
            continue;
        }
        let r = i / cols;
        let c = i % cols;
        let fa = accum[i];

        let mut num_out = 0.0;
        let mut weights = [0.0_f64; 8];
        for k in 0..8 {
            if (fd & flow_directions[k]) > 0 {
                num_out += 1.0;
            }
        }

        if num_out > 0.0 {
            let pb_fd = if d8_flow_ptr[i] > 0 {
                (d8_flow_ptr[i] as f64).log2() as usize
            } else {
                0usize
            };

            if num_out == 1.0 {
                weights[pb_fd] = 1.0;
            } else {
                for k in 0..8 {
                    if (fd & flow_directions[k]) > 0 {
                        weights[k] = (1.0 - p) / num_out;
                    }
                }
                weights[pb_fd] += p;
            }

            for k in 0..8 {
                if (fd & flow_directions[k]) == 0 {
                    continue;
                }
                let rn = r as isize + DY[k];
                let cn = c as isize + DX[k];
                if !in_bounds(rn, cn, rows, cols) {
                    continue;
                }
                let ni = idx(rn as usize, cn as usize, cols);
                accum[ni] += weights[k] * fa;
                if inflowing_count[ni] > 0 {
                    inflowing_count[ni] -= 1;
                    if inflowing_count[ni] == 0 {
                        accum_stack.push(ni);
                    }
                }
            }
        }
    }

    if interior_pit_found {
        // keep parity behavior by retaining flag path; currently no direct warnings in tool runtime
    }
    (pntr_modified, accum, d8_flow_ptr)
}

fn apply_mdfa_output_type(
    accum: &mut [f64],
    pntr_modified: &[u16],
    input: &Raster,
    p: f64,
    out_type: &str,
    log_transform: bool,
) {
    let rows = input.rows;
    let cols = input.cols;
    let nodata = input.nodata;
    let flow_directions: [u16; 8] = [1, 2, 4, 8, 16, 32, 64, 128];

    if out_type == "sca" {
        let is_geo = raster_is_geographic(input);
        let use_haversine = if is_geo { should_use_haversine(input) } else { false };

        if !is_geo {
            let cell_x = input.cell_size_x;
            let cell_y = input.cell_size_y;
            let avg = (cell_x + cell_y) / 2.0;
            let flow_widths = if p < 1.0 {
                let fw = avg * (std::f64::consts::SQRT_2 - 1.0);
                [fw, fw, fw, fw, fw, fw, fw, fw]
            } else {
                [avg, cell_y, avg, cell_x, avg, cell_y, avg, cell_x]
            };

            for r in 0..rows {
                for c in 0..cols {
                    let i = idx(r, c, cols);
                    if input.get(0, r as isize, c as isize) == nodata {
                        accum[i] = -32768.0;
                        continue;
                    }
                    let fd = pntr_modified[i];
                    let mut total_flow_width = 0.0;
                    let mut num_out = 0.0;
                    for k in 0..8 {
                        if (fd & flow_directions[k]) > 0 {
                            total_flow_width += flow_widths[k];
                            num_out += 1.0;
                        }
                    }
                    if total_flow_width > 0.0 {
                        if num_out == 1.0 {
                            total_flow_width = avg;
                        }
                        let v = accum[i] / total_flow_width;
                        accum[i] = if log_transform { v.ln() } else { v };
                    } else {
                        let v = accum[i] / flow_widths[0];
                        accum[i] = if log_transform { v.ln() } else { v };
                    }
                }
            }
        } else {
            for r in 0..rows {
                let phi0 = input.row_center_y(r as isize);
                for c in 0..cols {
                    let i = idx(r, c, cols);
                    if input.get(0, r as isize, c as isize) == nodata {
                        accum[i] = -32768.0;
                        continue;
                    }
                    let lambda0 = input.col_center_x(c as isize);
                    let cell_x = geo_distance_m(
                        use_haversine,
                        (phi0, lambda0),
                        (phi0, input.col_center_x(c as isize + 1)),
                    );
                    let cell_y = geo_distance_m(
                        use_haversine,
                        (phi0, lambda0),
                        (input.row_center_y(r as isize + 1), lambda0),
                    );
                    let avg = (cell_x + cell_y) / 2.0;
                    let flow_widths = if p < 1.0 {
                        let fw = avg * (std::f64::consts::SQRT_2 - 1.0);
                        [fw, fw, fw, fw, fw, fw, fw, fw]
                    } else {
                        [avg, cell_y, avg, cell_x, avg, cell_y, avg, cell_x]
                    };

                    let fd = pntr_modified[i];
                    let mut total_flow_width = 0.0;
                    let mut num_out = 0.0;
                    for k in 0..8 {
                        if (fd & flow_directions[k]) > 0 {
                            total_flow_width += flow_widths[k];
                            num_out += 1.0;
                        }
                    }
                    if total_flow_width > 0.0 {
                        if num_out == 1.0 {
                            total_flow_width = avg;
                        }
                        let v = accum[i] / total_flow_width;
                        accum[i] = if log_transform { v.ln() } else { v };
                    } else {
                        let v = accum[i] / flow_widths[0];
                        accum[i] = if log_transform { v.ln() } else { v };
                    }
                }
            }
        }
    } else if log_transform {
        for r in 0..rows {
            for c in 0..cols {
                let i = idx(r, c, cols);
                if input.get(0, r as isize, c as isize) == nodata {
                    accum[i] = -32768.0;
                } else {
                    let z = accum[i];
                    accum[i] = if z > 0.0 { z.ln() } else { 0.0 };
                }
            }
        }
    }
}

fn qin_flow_accum_core(input: &Raster, exponent: f64, max_slope_deg: f64, convergence_threshold: f64) -> Vec<f64> {
    let rows = input.rows;
    let cols = input.cols;
    let nodata = input.nodata;
    let n = rows * cols;
    let mut out = vec![nodata; n];
    let mut inflow = fd8_inflow_count(input);
    let mut stack = Vec::<usize>::with_capacity(n);

    for i in 0..n {
        if inflow[i] >= 0 {
            out[i] = 1.0;
            if inflow[i] == 0 {
                stack.push(i);
            }
        }
    }

    let cell_x = input.cell_size_x;
    let cell_y = input.cell_size_y;
    let diag = (cell_x * cell_x + cell_y * cell_y).sqrt();
    let grid_lengths = [diag, cell_x, diag, cell_y, diag, cell_x, diag, cell_y];
    let mut upper_slope = max_slope_deg.min(90.0).to_radians().tan();
    if upper_slope < 0.0 {
        upper_slope = 0.0;
    }

    while let Some(i) = stack.pop() {
        let r = i / cols;
        let c = i % cols;
        let z = input.get(0, r as isize, c as isize);
        let fa = out[i];
        inflow[i] = -1;

        let mut weights = [0.0_f64; 8];
        let mut downslope = [false; 8];
        let mut total_weights = 0.0;

        if fa < convergence_threshold {
            let mut max_slope = f64::MIN;
            for k in 0..8 {
                let rn = r as isize + DY[k];
                let cn = c as isize + DX[k];
                if !in_bounds(rn, cn, rows, cols) {
                    continue;
                }
                let zn = input.get(0, rn, cn);
                if zn == nodata {
                    continue;
                }
                let slope = (z - zn) / grid_lengths[k];
                if slope > 0.0 {
                    downslope[k] = true;
                    if slope > max_slope {
                        max_slope = slope;
                    }
                }
            }

            if max_slope > 0.0 {
                let f = if upper_slope > 0.0 {
                    let e = max_slope.min(upper_slope);
                    (e / upper_slope) * (exponent - 1.1) + 1.1
                } else {
                    exponent
                };

                for k in 0..8 {
                    if !downslope[k] {
                        continue;
                    }
                    let rn = r as isize + DY[k];
                    let cn = c as isize + DX[k];
                    let zn = input.get(0, rn, cn);
                    let slope = (z - zn) / grid_lengths[k];
                    let w = slope.powf(f);
                    weights[k] = w;
                    total_weights += w;
                }
            }
        } else {
            // Convergent (D8-like) routing above convergence threshold.
            let mut best_dir = 0usize;
            let mut max_slope = f64::MIN;
            for k in 0..8 {
                let rn = r as isize + DY[k];
                let cn = c as isize + DX[k];
                if !in_bounds(rn, cn, rows, cols) {
                    continue;
                }
                let zn = input.get(0, rn, cn);
                if zn == nodata {
                    continue;
                }
                let slope = (z - zn) / grid_lengths[k];
                if slope > 0.0 {
                    downslope[k] = true;
                    if slope > max_slope {
                        max_slope = slope;
                        best_dir = k;
                    }
                }
            }
            if max_slope >= 0.0 {
                weights[best_dir] = 1.0;
                total_weights = 1.0;
            }
        }

        for k in 0..8 {
            if !downslope[k] {
                continue;
            }
            let rn = r as isize + DY[k];
            let cn = c as isize + DX[k];
            let ni = idx(rn as usize, cn as usize, cols);
            if total_weights > 0.0 {
                out[ni] += fa * (weights[k] / total_weights);
            }
            if inflow[ni] > 0 {
                inflow[ni] -= 1;
                if inflow[ni] == 0 {
                    stack.push(ni);
                }
            }
        }
    }

    out
}

fn quinn_flow_accum_core(input: &Raster, exponent: f64, convergence_threshold: f64) -> Vec<f64> {
    let rows = input.rows;
    let cols = input.cols;
    let nodata = input.nodata;
    let n = rows * cols;
    let mut out = vec![nodata; n];
    let mut inflow = fd8_inflow_count(input);
    let mut stack = Vec::<usize>::with_capacity(n);

    for i in 0..n {
        if inflow[i] >= 0 {
            out[i] = 1.0;
            if inflow[i] == 0 {
                stack.push(i);
            }
        }
    }

    let cell_x = input.cell_size_x;
    let cell_y = input.cell_size_y;
    let diag = (cell_x * cell_x + cell_y * cell_y).sqrt();
    let grid_lengths = [diag, cell_x, diag, cell_y, diag, cell_x, diag, cell_y];

    while let Some(i) = stack.pop() {
        let r = i / cols;
        let c = i % cols;
        let z = input.get(0, r as isize, c as isize);
        let fa = out[i];
        inflow[i] = -1;

        let mut weights = [0.0_f64; 8];
        let mut downslope = [false; 8];
        let mut total_weights = 0.0;
        let mut converged = fa >= convergence_threshold;

        if !converged {
            let f = if convergence_threshold.is_finite() {
                (fa / convergence_threshold + 1.0).powf(exponent)
            } else {
                1.0
            };
            if f > 50.0 {
                converged = true;
            } else {
                for k in 0..8 {
                    let rn = r as isize + DY[k];
                    let cn = c as isize + DX[k];
                    if !in_bounds(rn, cn, rows, cols) {
                        continue;
                    }
                    let zn = input.get(0, rn, cn);
                    if zn == nodata {
                        continue;
                    }
                    let slope = (z - zn) / grid_lengths[k];
                    if slope > 0.0 {
                        downslope[k] = true;
                        let w = slope.powf(f);
                        weights[k] = w;
                        total_weights += w;
                    }
                }
            }
        }

        if converged {
            // Convergent (D8-like) routing.
            let mut best_dir = 0usize;
            let mut max_slope = f64::MIN;
            for k in 0..8 {
                let rn = r as isize + DY[k];
                let cn = c as isize + DX[k];
                if !in_bounds(rn, cn, rows, cols) {
                    continue;
                }
                let zn = input.get(0, rn, cn);
                if zn == nodata {
                    continue;
                }
                let slope = (z - zn) / grid_lengths[k];
                if slope > 0.0 {
                    downslope[k] = true;
                    if slope > max_slope {
                        max_slope = slope;
                        best_dir = k;
                    }
                }
            }
            if max_slope >= 0.0 {
                weights[best_dir] = 1.0;
                total_weights = 1.0;
            }
        }

        for k in 0..8 {
            if !downslope[k] {
                continue;
            }
            let rn = r as isize + DY[k];
            let cn = c as isize + DX[k];
            let ni = idx(rn as usize, cn as usize, cols);
            if total_weights > 0.0 {
                out[ni] += fa * (weights[k] / total_weights);
            }
            if inflow[ni] > 0 {
                inflow[ni] -= 1;
                if inflow[ni] == 0 {
                    stack.push(ni);
                }
            }
        }
    }

    out
}

/// Returns 0-7 direction per cell (−2 = nodata, −1 = no downslope).
/// Uses the Rho8 stochastic perturbation: diagonal-neighbour distances are
/// divided by (2 − U) where U ∼ Uniform[0, 1).
fn rho8_dir_from_dem(input: &Raster) -> Vec<i8> {
    let rows = input.rows;
    let cols = input.cols;
    let nodata = input.nodata;
    let is_geo = raster_is_geographic(input);
    let use_haversine = if is_geo { should_use_haversine(input) } else { false };
    let mut rng = rand::rng();
    let mut out = vec![-2i8; rows * cols];

    for r in 0..rows {
        for c in 0..cols {
            let i = idx(r, c, cols);
            let z = input.get(0, r as isize, c as isize);
            if z == nodata {
                continue; // leave as -2 (nodata marker)
            }
            let mut best_dir = -1i8;
            let mut best_slope = f64::MIN;
            for k in 0..8usize {
                let rn = r as isize + DY[k];
                let cn = c as isize + DX[k];
                if !in_bounds(rn, cn, rows, cols) {
                    continue;
                }
                let z_n = input.get(0, rn, cn);
                if z_n == nodata {
                    continue;
                }
                let is_cardinal = matches!(k, 1 | 3 | 5 | 7);
                let slope = if is_geo {
                    let lat1 = input.row_center_y(r as isize);
                    let lon1 = input.col_center_x(c as isize);
                    let (lat2, lon2) = if is_cardinal {
                        (input.row_center_y(rn), input.col_center_x(cn))
                    } else {
                        // For diagonal k, approximate distance via adjacent cardinal cell (k+1)
                        (
                            input.row_center_y(r as isize + DY[k + 1]),
                            input.col_center_x(c as isize + DX[k + 1]),
                        )
                    };
                    let d = geo_distance_m(use_haversine, (lat1, lon1), (lat2, lon2));
                    if is_cardinal {
                        (z - z_n) / d
                    } else {
                        (z - z_n) / (d * (2.0 - rng.random_range(0.0_f64..1.0_f64)))
                    }
                } else if is_cardinal {
                    z - z_n
                } else {
                    (z - z_n) / (2.0 - rng.random_range(0.0_f64..1.0_f64))
                };
                if slope > best_slope && slope > 0.0 {
                    best_slope = slope;
                    best_dir = k as i8;
                }
            }
            out[i] = best_dir;
        }
    }
    out
}

fn d8_dir_from_pointer(input: &Raster, esri_style: bool) -> Vec<i8> {
    let rows = input.rows;
    let cols = input.cols;
    let nodata = input.nodata;
    let mut mapping = [-2i8; 129];
    if !esri_style {
        mapping[1] = 0;
        mapping[2] = 1;
        mapping[4] = 2;
        mapping[8] = 3;
        mapping[16] = 4;
        mapping[32] = 5;
        mapping[64] = 6;
        mapping[128] = 7;
    } else {
        mapping[1] = 1;
        mapping[2] = 2;
        mapping[4] = 3;
        mapping[8] = 4;
        mapping[16] = 5;
        mapping[32] = 6;
        mapping[64] = 7;
        mapping[128] = 0;
    }

    let mut out = vec![-2i8; rows * cols];
    for r in 0..rows {
        for c in 0..cols {
            let i = idx(r, c, cols);
            let z = input.get(0, r as isize, c as isize);
            if z == nodata {
                continue;
            }
            if z > 0.0 {
                let zi = z as usize;
                if zi < mapping.len() {
                    out[i] = mapping[zi];
                } else {
                    out[i] = -1;
                }
            } else {
                out[i] = -1;
            }
        }
    }
    out
}

fn d8_pointer_values_from_dir(dirs: &[i8], rows: usize, cols: usize, nodata: f64, esri_style: bool) -> Vec<f64> {
    let out_vals = if esri_style {
        [128.0, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0]
    } else {
        [1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0]
    };
    let mut out = vec![nodata; rows * cols];
    for r in 0..rows {
        for c in 0..cols {
            let i = idx(r, c, cols);
            let d = dirs[i];
            out[i] = if d >= 0 { out_vals[d as usize] } else { 0.0 };
        }
    }
    out
}

fn d8_flow_accum_core(flow_dir: &[i8], rows: usize, cols: usize, nodata: f64) -> Vec<f64> {
    let n = rows * cols;
    let mut out = vec![nodata; n];
    let mut inflow = vec![-1i8; n];

    for r in 0..rows {
        for c in 0..cols {
            let i = idx(r, c, cols);
            if flow_dir[i] == -2 {
                continue;
            }
            out[i] = 1.0;
            let mut count = 0i8;
            for k in 0..8 {
                let rn = r as isize + DY[k];
                let cn = c as isize + DX[k];
                if !in_bounds(rn, cn, rows, cols) {
                    continue;
                }
                let ni = idx(rn as usize, cn as usize, cols);
                if flow_dir[ni] == INFLOWING_VALS[k] {
                    count += 1;
                }
            }
            inflow[i] = count;
        }
    }

    let mut stack = Vec::<usize>::with_capacity(n);
    for i in 0..n {
        if inflow[i] == 0 {
            stack.push(i);
        }
    }

    while let Some(i) = stack.pop() {
        let d = flow_dir[i];
        if d >= 0 {
            let r = i / cols;
            let c = i % cols;
            let rn = r as isize + DY[d as usize];
            let cn = c as isize + DX[d as usize];
            if in_bounds(rn, cn, rows, cols) {
                let ni = idx(rn as usize, cn as usize, cols);
                if out[ni] != nodata {
                    out[ni] += out[i];
                    inflow[ni] -= 1;
                    if inflow[ni] == 0 {
                        stack.push(ni);
                    }
                }
            }
        }
    }

    out
}

fn apply_accum_output_type(
    accum: &mut [f64],
    flow_dir: &[i8],
    input: &Raster,
    nodata: f64,
    out_type: &str,
    log_transform: bool,
) {
    let is_geo = raster_is_geographic(input);
    let use_haversine = if is_geo { should_use_haversine(input) } else { false };

    if !is_geo {
        let mut cell_area = input.cell_size_x * input.cell_size_y;
        let mut flow_width = (input.cell_size_x + input.cell_size_y) / 2.0;

        if out_type == "cells" {
            cell_area = 1.0;
            flow_width = 1.0;
        } else if out_type == "ca" {
            flow_width = 1.0;
        }

        for i in 0..accum.len() {
            if flow_dir[i] == -2 {
                accum[i] = nodata;
                continue;
            }
            let mut v = accum[i] * cell_area / flow_width;
            if log_transform {
                v = v.ln();
            }
            accum[i] = v;
        }
        return;
    }

    for r in 0..input.rows {
        for c in 0..input.cols {
            let i = idx(r, c, input.cols);
            if flow_dir[i] == -2 {
                accum[i] = nodata;
                continue;
            }

            let phi0 = input.row_center_y(r as isize);
            let lambda0 = input.col_center_x(c as isize);
            let cell_x = geo_distance_m(
                use_haversine,
                (phi0, lambda0),
                (phi0, input.col_center_x(c as isize + 1)),
            );
            let cell_y = geo_distance_m(
                use_haversine,
                (phi0, lambda0),
                (input.row_center_y(r as isize + 1), lambda0),
            );

            let (cell_area, flow_width) = if out_type == "cells" {
                (1.0, 1.0)
            } else if out_type == "ca" {
                (cell_x * cell_y, 1.0)
            } else {
                (cell_x * cell_y, (cell_x + cell_y) / 2.0)
            };

            let mut v = accum[i] * cell_area / flow_width;
            if log_transform {
                v = v.ln();
            }
            accum[i] = v;
        }
    }
}

impl Tool for D8PointerTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "d8_pointer",
            display_name: "D8 Pointer",
            summary: "Generates a D8 flow-direction pointer raster from a DEM.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "dem",
                    description: "Input DEM raster",
                    required: true,
                },
                ToolParamSpec {
                    name: "esri_pntr",
                    description: "Use ESRI pointer encoding",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output raster path",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("esri_pntr".to_string(), json!(false));
        ToolManifest {
            id: "d8_pointer".to_string(),
            display_name: "D8 Pointer".to_string(),
            summary: "Generates a D8 flow-direction pointer raster from a DEM.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![ToolExample {
                name: "d8_from_dem".to_string(),
                description: "Compute D8 pointers from a depressionless DEM".to_string(),
                args: ToolArgs::new(),
            }],
            tags: vec!["hydrology".to_string(), "flow-direction".to_string(), "d8".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        parse_raster_path_arg(args, "dem")
            .or_else(|_| parse_raster_path_arg(args, "input"))
            .or_else(|_| parse_raster_path_arg(args, "raster"))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (input, output_path) = parse_input_and_output(args)?;
        let esri = args
            .get("esri_pntr")
            .or_else(|| args.get("esri_pointer"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let dirs = d8_dir_from_dem(&input);
        let mut values = d8_pointer_values_from_dir(&dirs, input.rows, input.cols, -32768.0, esri);
        for r in 0..input.rows {
            for c in 0..input.cols {
                let i = idx(r, c, input.cols);
                if input.get(0, r as isize, c as isize) == input.nodata {
                    values[i] = -32768.0;
                }
            }
        }

        let mut out = input.clone();
        out.data_type = DataType::I16;
        out.nodata = -32768.0;
        for r in 0..input.rows {
            for c in 0..input.cols {
                out.set_unchecked(0, r as isize, c as isize, values[idx(r, c, input.cols)]);
            }
        }

        Ok(build_result(write_or_store_output(out, output_path)?))
    }
}

impl Tool for D8FlowAccumTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "d8_flow_accum",
            display_name: "D8 Flow Accumulation",
            summary: "Calculates D8 flow accumulation from a DEM or D8 pointer raster.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input DEM or D8 pointer raster",
                    required: true,
                },
                ToolParamSpec {
                    name: "out_type",
                    description: "Output type: cells, ca, or sca",
                    required: false,
                },
                ToolParamSpec {
                    name: "log_transform",
                    description: "Log-transform output",
                    required: false,
                },
                ToolParamSpec {
                    name: "clip",
                    description: "Clip display max (accepted for compatibility)",
                    required: false,
                },
                ToolParamSpec {
                    name: "input_is_pointer",
                    description: "Treat input as D8 pointer raster",
                    required: false,
                },
                ToolParamSpec {
                    name: "esri_pntr",
                    description: "Use ESRI pointer encoding for pointer inputs",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output raster path",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("out_type".to_string(), json!("sca"));
        defaults.insert("log_transform".to_string(), json!(false));
        defaults.insert("clip".to_string(), json!(false));
        defaults.insert("input_is_pointer".to_string(), json!(false));
        defaults.insert("esri_pntr".to_string(), json!(false));
        ToolManifest {
            id: "d8_flow_accum".to_string(),
            display_name: "D8 Flow Accumulation".to_string(),
            summary: "Calculates D8 flow accumulation from a DEM or D8 pointer raster.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![ToolExample {
                name: "d8_accum".to_string(),
                description: "Compute D8 specific contributing area from DEM".to_string(),
                args: ToolArgs::new(),
            }],
            tags: vec!["hydrology".to_string(), "flow-accumulation".to_string(), "d8".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        parse_raster_path_arg(args, "input")
            .or_else(|_| parse_raster_path_arg(args, "raster"))
            .or_else(|_| parse_raster_path_arg(args, "dem"))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (input, output_path) = parse_input_and_output(args)?;
        let out_type = args
            .get("out_type")
            .and_then(|v| v.as_str())
            .unwrap_or("sca")
            .to_lowercase();
        let out_type = if out_type.contains("specific") || out_type.contains("sca") {
            "sca"
        } else if out_type.contains("cells") {
            "cells"
        } else {
            "ca"
        };
        let log_transform = args
            .get("log_transform")
            .or_else(|| args.get("log"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let input_is_pointer = args
            .get("input_is_pointer")
            .or_else(|| args.get("pntr"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let esri = args
            .get("esri_pntr")
            .or_else(|| args.get("esri_pointer"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let flow_dir = if input_is_pointer {
            d8_dir_from_pointer(&input, esri)
        } else {
            d8_dir_from_dem(&input)
        };

        let mut accum = d8_flow_accum_core(&flow_dir, input.rows, input.cols, -32768.0);
        apply_accum_output_type(
            &mut accum,
            &flow_dir,
            &input,
            -32768.0,
            out_type,
            log_transform,
        );

        let mut out = input.clone();
        out.data_type = DataType::F32;
        out.nodata = -32768.0;
        for r in 0..input.rows {
            for c in 0..input.cols {
                out.set_unchecked(0, r as isize, c as isize, accum[idx(r, c, input.cols)]);
            }
        }

        Ok(build_result(write_or_store_output(out, output_path)?))
    }
}

impl Tool for DInfPointerTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "dinf_pointer",
            display_name: "D-Infinity Pointer",
            summary: "Generates a D-Infinity flow-direction raster from a DEM.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
                ToolParamSpec { name: "output", description: "Output raster path", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "dinf_pointer".to_string(),
            display_name: "D-Infinity Pointer".to_string(),
            summary: "Generates a D-Infinity flow-direction raster from a DEM.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults: ToolArgs::new(),
            examples: vec![ToolExample {
                name: "dinf_from_dem".to_string(),
                description: "Compute D-Infinity flow directions from a depressionless DEM".to_string(),
                args: ToolArgs::new(),
            }],
            tags: vec!["hydrology".to_string(), "flow-direction".to_string(), "dinf".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        parse_raster_path_arg(args, "dem")
            .or_else(|_| parse_raster_path_arg(args, "input"))
            .or_else(|_| parse_raster_path_arg(args, "raster"))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (input, output_path) = parse_input_and_output(args)?;
        let values = dinf_pointer_from_dem(&input);
        let mut out = input.clone();
        out.data_type = DataType::F32;
        for r in 0..input.rows {
            for c in 0..input.cols {
                out.set_unchecked(0, r as isize, c as isize, values[idx(r, c, input.cols)]);
            }
        }
        Ok(build_result(write_or_store_output(out, output_path)?))
    }
}

impl Tool for DInfFlowAccumTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "dinf_flow_accum",
            display_name: "D-Infinity Flow Accumulation",
            summary: "Calculates D-Infinity flow accumulation from a DEM or D-Infinity pointer raster.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM or D-Infinity pointer raster", required: true },
                ToolParamSpec { name: "out_type", description: "Output type: cells, ca, or sca", required: false },
                ToolParamSpec { name: "convergence_threshold", description: "Threshold above which flow is not dispersed", required: false },
                ToolParamSpec { name: "log_transform", description: "Log-transform output", required: false },
                ToolParamSpec { name: "clip", description: "Clip display max (accepted for compatibility)", required: false },
                ToolParamSpec { name: "input_is_pointer", description: "Treat input as D-Infinity pointer raster", required: false },
                ToolParamSpec { name: "output", description: "Output raster path", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("out_type".to_string(), json!("sca"));
        defaults.insert("convergence_threshold".to_string(), json!(f64::INFINITY));
        defaults.insert("log_transform".to_string(), json!(false));
        defaults.insert("clip".to_string(), json!(false));
        defaults.insert("input_is_pointer".to_string(), json!(false));
        ToolManifest {
            id: "dinf_flow_accum".to_string(),
            display_name: "D-Infinity Flow Accumulation".to_string(),
            summary: "Calculates D-Infinity flow accumulation from a DEM or D-Infinity pointer raster.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![ToolExample {
                name: "dinf_accum".to_string(),
                description: "Compute D-Infinity specific contributing area".to_string(),
                args: ToolArgs::new(),
            }],
            tags: vec!["hydrology".to_string(), "flow-accumulation".to_string(), "dinf".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        parse_raster_path_arg(args, "input")
            .or_else(|_| parse_raster_path_arg(args, "raster"))
            .or_else(|_| parse_raster_path_arg(args, "dem"))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (input, output_path) = parse_input_and_output(args)?;
        let out_type = args
            .get("out_type")
            .and_then(|v| v.as_str())
            .unwrap_or("sca")
            .to_lowercase();
        let out_type = if out_type.contains("specific") || out_type.contains("sca") {
            "sca"
        } else if out_type.contains("cells") {
            "cells"
        } else {
            "ca"
        };
        let convergence_threshold = args
            .get("convergence_threshold")
            .or_else(|| args.get("threshold"))
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::INFINITY);
        let convergence_threshold = if convergence_threshold <= 0.0 { f64::INFINITY } else { convergence_threshold };
        let log_transform = args
            .get("log_transform")
            .or_else(|| args.get("log"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let input_is_pointer = args
            .get("input_is_pointer")
            .or_else(|| args.get("pntr"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let flow_dir = if input_is_pointer {
            let mut dirs = vec![input.nodata; input.rows * input.cols];
            for r in 0..input.rows {
                for c in 0..input.cols {
                    dirs[idx(r, c, input.cols)] = input.get(0, r as isize, c as isize);
                }
            }
            dirs
        } else {
            dinf_pointer_from_dem(&input)
        };

        let mut accum = dinf_flow_accum_core(&flow_dir, input.rows, input.cols, input.nodata, convergence_threshold);
        apply_dinf_output_type(&mut accum, &input, out_type, log_transform);

        let mut out = input.clone();
        out.data_type = DataType::F32;
        for r in 0..input.rows {
            for c in 0..input.cols {
                out.set_unchecked(0, r as isize, c as isize, accum[idx(r, c, input.cols)]);
            }
        }

        Ok(build_result(write_or_store_output(out, output_path)?))
    }
}

impl Tool for MDInfFlowAccumTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "mdinf_flow_accum",
            display_name: "MD-Infinity Flow Accumulation",
            summary: "Calculates MD-Infinity triangular multiple-flow-direction accumulation from a DEM.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
                ToolParamSpec { name: "out_type", description: "Output type: cells, ca, or sca", required: false },
                ToolParamSpec { name: "exponent", description: "Slope weighting exponent", required: false },
                ToolParamSpec { name: "convergence_threshold", description: "Threshold above which flow is not dispersed", required: false },
                ToolParamSpec { name: "log_transform", description: "Log-transform output", required: false },
                ToolParamSpec { name: "clip", description: "Clip display max (accepted for compatibility)", required: false },
                ToolParamSpec { name: "output", description: "Output raster path", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("out_type".to_string(), json!("sca"));
        defaults.insert("exponent".to_string(), json!(1.1));
        defaults.insert("convergence_threshold".to_string(), json!(f64::INFINITY));
        defaults.insert("log_transform".to_string(), json!(false));
        defaults.insert("clip".to_string(), json!(false));
        ToolManifest {
            id: "mdinf_flow_accum".to_string(),
            display_name: "MD-Infinity Flow Accumulation".to_string(),
            summary: "Calculates MD-Infinity triangular multiple-flow-direction accumulation from a DEM."
                .to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![ToolExample {
                name: "mdinf_accum".to_string(),
                description: "Compute MDInf specific contributing area from DEM".to_string(),
                args: ToolArgs::new(),
            }],
            tags: vec![
                "hydrology".to_string(),
                "flow-accumulation".to_string(),
                "mdinf".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        parse_raster_path_arg(args, "dem")
            .or_else(|_| parse_raster_path_arg(args, "input"))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (input, output_path) = parse_input_and_output(args)?;
        let out_type = args
            .get("out_type")
            .and_then(|v| v.as_str())
            .unwrap_or("sca")
            .to_lowercase();
        let out_type = if out_type.contains("specific") || out_type.contains("sca") {
            "sca"
        } else if out_type.contains("cells") {
            "cells"
        } else {
            "ca"
        };
        let exponent = args.get("exponent").and_then(|v| v.as_f64()).unwrap_or(1.1);
        let convergence_threshold = args
            .get("convergence_threshold")
            .or_else(|| args.get("threshold"))
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::INFINITY);
        let convergence_threshold =
            if convergence_threshold <= 0.0 { f64::INFINITY } else { convergence_threshold };
        let log_transform = args
            .get("log_transform")
            .or_else(|| args.get("log"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut accum = mdinf_flow_accum_core(&input, exponent, convergence_threshold);
        apply_dinf_output_type(&mut accum, &input, out_type, log_transform);

        let mut out = input.clone();
        out.data_type = DataType::F32;
        for r in 0..input.rows {
            for c in 0..input.cols {
                out.set_unchecked(0, r as isize, c as isize, accum[idx(r, c, input.cols)]);
            }
        }

        Ok(build_result(write_or_store_output(out, output_path)?))
    }
}

impl Tool for QinFlowAccumulationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "qin_flow_accumulation",
            display_name: "Qin Flow Accumulation",
            summary: "Calculates Qin MFD flow accumulation from a DEM.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
                ToolParamSpec { name: "out_type", description: "Output type: cells, ca, or sca", required: false },
                ToolParamSpec { name: "exponent", description: "Upper-bound exponent parameter", required: false },
                ToolParamSpec { name: "max_slope", description: "Upper-bound slope in degrees", required: false },
                ToolParamSpec { name: "convergence_threshold", description: "Threshold above which flow is not dispersed", required: false },
                ToolParamSpec { name: "log_transform", description: "Log-transform output", required: false },
                ToolParamSpec { name: "clip", description: "Clip display max (accepted for compatibility)", required: false },
                ToolParamSpec { name: "output", description: "Output raster path", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("out_type".to_string(), json!("sca"));
        defaults.insert("exponent".to_string(), json!(10.0));
        defaults.insert("max_slope".to_string(), json!(45.0));
        defaults.insert("convergence_threshold".to_string(), json!(f64::INFINITY));
        defaults.insert("log_transform".to_string(), json!(false));
        defaults.insert("clip".to_string(), json!(false));
        ToolManifest {
            id: "qin_flow_accumulation".to_string(),
            display_name: "Qin Flow Accumulation".to_string(),
            summary: "Calculates Qin MFD flow accumulation from a DEM.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![ToolExample {
                name: "qin_accum".to_string(),
                description: "Compute Qin specific contributing area from DEM".to_string(),
                args: ToolArgs::new(),
            }],
            tags: vec!["hydrology".to_string(), "flow-accumulation".to_string(), "qin".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        parse_raster_path_arg(args, "dem")
            .or_else(|_| parse_raster_path_arg(args, "input"))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (input, output_path) = parse_input_and_output(args)?;
        let out_type = args
            .get("out_type")
            .and_then(|v| v.as_str())
            .unwrap_or("sca")
            .to_lowercase();
        let out_type = if out_type.contains("specific") || out_type.contains("sca") {
            "sca"
        } else if out_type.contains("cells") {
            "cells"
        } else {
            "ca"
        };

        let mut exponent = args.get("exponent").and_then(|v| v.as_f64()).unwrap_or(10.0);
        if exponent < 1.1 {
            exponent = 1.1;
        }
        if exponent >= 50.0 {
            exponent = 50.0;
        }
        let max_slope = args.get("max_slope").and_then(|v| v.as_f64()).unwrap_or(45.0);
        let convergence_threshold = args
            .get("convergence_threshold")
            .or_else(|| args.get("threshold"))
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::INFINITY);
        let convergence_threshold = if convergence_threshold <= 0.0 { f64::INFINITY } else { convergence_threshold };
        let log_transform = args
            .get("log_transform")
            .or_else(|| args.get("log"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut accum = qin_flow_accum_core(&input, exponent, max_slope, convergence_threshold);
        apply_dinf_output_type(&mut accum, &input, out_type, log_transform);

        let mut out = input.clone();
        out.data_type = DataType::F32;
        for r in 0..input.rows {
            for c in 0..input.cols {
                out.set_unchecked(0, r as isize, c as isize, accum[idx(r, c, input.cols)]);
            }
        }

        Ok(build_result(write_or_store_output(out, output_path)?))
    }
}

impl Tool for QuinnFlowAccumulationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "quinn_flow_accumulation",
            display_name: "Quinn Flow Accumulation",
            summary: "Calculates Quinn MFD flow accumulation from a DEM.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
                ToolParamSpec { name: "out_type", description: "Output type: cells, ca, or sca", required: false },
                ToolParamSpec { name: "exponent", description: "Exponent parameter", required: false },
                ToolParamSpec { name: "convergence_threshold", description: "Threshold above which flow is not dispersed", required: false },
                ToolParamSpec { name: "log_transform", description: "Log-transform output", required: false },
                ToolParamSpec { name: "clip", description: "Clip display max (accepted for compatibility)", required: false },
                ToolParamSpec { name: "output", description: "Output raster path", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("out_type".to_string(), json!("sca"));
        defaults.insert("exponent".to_string(), json!(1.1));
        defaults.insert("convergence_threshold".to_string(), json!(f64::INFINITY));
        defaults.insert("log_transform".to_string(), json!(false));
        defaults.insert("clip".to_string(), json!(false));
        ToolManifest {
            id: "quinn_flow_accumulation".to_string(),
            display_name: "Quinn Flow Accumulation".to_string(),
            summary: "Calculates Quinn MFD flow accumulation from a DEM.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![ToolExample {
                name: "quinn_accum".to_string(),
                description: "Compute Quinn specific contributing area from DEM".to_string(),
                args: ToolArgs::new(),
            }],
            tags: vec!["hydrology".to_string(), "flow-accumulation".to_string(), "quinn".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        parse_raster_path_arg(args, "dem")
            .or_else(|_| parse_raster_path_arg(args, "input"))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (input, output_path) = parse_input_and_output(args)?;
        let out_type = args
            .get("out_type")
            .and_then(|v| v.as_str())
            .unwrap_or("sca")
            .to_lowercase();
        let out_type = if out_type.contains("specific") || out_type.contains("sca") {
            "sca"
        } else if out_type.contains("cells") {
            "cells"
        } else {
            "ca"
        };
        let exponent = args.get("exponent").and_then(|v| v.as_f64()).unwrap_or(1.1);
        let convergence_threshold = args
            .get("convergence_threshold")
            .or_else(|| args.get("threshold"))
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::INFINITY);
        let convergence_threshold = if convergence_threshold <= 0.0 { f64::INFINITY } else { convergence_threshold };
        let log_transform = args
            .get("log_transform")
            .or_else(|| args.get("log"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut accum = quinn_flow_accum_core(&input, exponent, convergence_threshold);
        apply_dinf_output_type(&mut accum, &input, out_type, log_transform);

        let mut out = input.clone();
        out.data_type = DataType::F32;
        for r in 0..input.rows {
            for c in 0..input.cols {
                out.set_unchecked(0, r as isize, c as isize, accum[idx(r, c, input.cols)]);
            }
        }

        Ok(build_result(write_or_store_output(out, output_path)?))
    }
}

impl Tool for MinimalDispersionFlowAlgorithmTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "minimal_dispersion_flow_algorithm",
            display_name: "Minimal Dispersion Flow Algorithm",
            summary: "Generates MDFA flow-direction and flow-accumulation rasters from a DEM.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
                ToolParamSpec { name: "out_type", description: "Output type: cells, ca, or sca", required: false },
                ToolParamSpec { name: "path_corrected_direction_preference", description: "Preference parameter p in [0,1]", required: false },
                ToolParamSpec { name: "log_transform", description: "Log-transform accumulation output", required: false },
                ToolParamSpec { name: "clip", description: "Clip display max (accepted for compatibility)", required: false },
                ToolParamSpec { name: "esri_pntr", description: "Use ESRI pointer encoding for flow-direction output", required: false },
                ToolParamSpec { name: "output", description: "Flow accumulation output raster path", required: false },
                ToolParamSpec { name: "flow_dir_output", description: "Flow-direction output raster path", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("out_type".to_string(), json!("sca"));
        defaults.insert("path_corrected_direction_preference".to_string(), json!(0.0));
        defaults.insert("log_transform".to_string(), json!(false));
        defaults.insert("clip".to_string(), json!(false));
        defaults.insert("esri_pntr".to_string(), json!(false));
        ToolManifest {
            id: "minimal_dispersion_flow_algorithm".to_string(),
            display_name: "Minimal Dispersion Flow Algorithm".to_string(),
            summary: "Generates MDFA flow-direction and flow-accumulation rasters from a DEM.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![ToolExample {
                name: "mdfa".to_string(),
                description: "Compute MDFA direction and specific contributing area from DEM".to_string(),
                args: ToolArgs::new(),
            }],
            tags: vec![
                "hydrology".to_string(),
                "flow-direction".to_string(),
                "flow-accumulation".to_string(),
                "mdfa".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        parse_raster_path_arg(args, "dem")
            .or_else(|_| parse_raster_path_arg(args, "input"))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (input, accum_output_path) = parse_input_and_output(args)?;
        let dir_output_path = parse_optional_output_path(args, "flow_dir_output")
            .or_else(|_| parse_optional_output_path(args, "pointer_output"))?;
        let out_type = args
            .get("out_type")
            .and_then(|v| v.as_str())
            .unwrap_or("sca")
            .to_lowercase();
        let out_type = if out_type.contains("specific") || out_type.contains("sca") {
            "sca"
        } else if out_type.contains("cells") {
            "cells"
        } else {
            "ca"
        };
        let p = args
            .get("path_corrected_direction_preference")
            .or_else(|| args.get("p"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        let log_transform = args
            .get("log_transform")
            .or_else(|| args.get("log"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let clip = args
            .get("clip")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let esri = args
            .get("esri_pntr")
            .or_else(|| args.get("esri_pointer"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let (pntr_modified, mut accum, _d8_primary) =
            minimal_dispersion_core(&input, p, out_type, esri);
        apply_mdfa_output_type(&mut accum, &pntr_modified, &input, p, out_type, log_transform);

        let mut dir_out = input.clone();
        dir_out.data_type = DataType::I16;
        dir_out.nodata = -32768.0;
        for r in 0..input.rows {
            for c in 0..input.cols {
                let i = idx(r, c, input.cols);
                if input.get(0, r as isize, c as isize) == input.nodata {
                    dir_out.set_unchecked(0, r as isize, c as isize, -32768.0);
                    continue;
                }
                dir_out.set_unchecked(0, r as isize, c as isize, pntr_modified[i] as f64);
            }
        }

        if clip {
            let mut vals = accum
                .iter()
                .copied()
                .filter(|v| *v != input.nodata && v.is_finite())
                .collect::<Vec<f64>>();
            if !vals.is_empty() {
                vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let idx95 = ((vals.len() as f64) * 0.95).floor() as usize;
                let clip_max = vals[idx95.min(vals.len() - 1)];
                for v in &mut accum {
                    if *v != input.nodata && *v > clip_max {
                        *v = clip_max;
                    }
                }
            }
        }

        let mut accum_out = input.clone();
        accum_out.data_type = DataType::F32;
        for r in 0..input.rows {
            for c in 0..input.cols {
                accum_out.set_unchecked(0, r as isize, c as isize, accum[idx(r, c, input.cols)]);
            }
        }

        let dir_locator = write_or_store_output(dir_out, dir_output_path)?;
        let accum_locator = write_or_store_output(accum_out, accum_output_path)?;
        Ok(build_dual_raster_result(dir_locator, accum_locator))
    }
}

impl Tool for FD8PointerTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "fd8_pointer",
            display_name: "FD8 Pointer",
            summary: "Generates an FD8 multiple-flow-direction pointer raster from a DEM.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
                ToolParamSpec { name: "output", description: "Output raster path", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "fd8_pointer".to_string(),
            display_name: "FD8 Pointer".to_string(),
            summary: "Generates an FD8 multiple-flow-direction pointer raster from a DEM.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults: ToolArgs::new(),
            examples: vec![ToolExample {
                name: "fd8_from_dem".to_string(),
                description: "Compute FD8 pointer values from a depressionless DEM".to_string(),
                args: ToolArgs::new(),
            }],
            tags: vec!["hydrology".to_string(), "flow-direction".to_string(), "fd8".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        parse_raster_path_arg(args, "dem")
            .or_else(|_| parse_raster_path_arg(args, "input"))
            .or_else(|_| parse_raster_path_arg(args, "raster"))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (input, output_path) = parse_input_and_output(args)?;
        let values = fd8_pointer_from_dem(&input);

        let mut out = input.clone();
        out.data_type = DataType::I16;
        out.nodata = -32768.0;
        for r in 0..input.rows {
            for c in 0..input.cols {
                out.set_unchecked(0, r as isize, c as isize, values[idx(r, c, input.cols)]);
            }
        }

        Ok(build_result(write_or_store_output(out, output_path)?))
    }
}

impl Tool for Rho8PointerTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "rho8_pointer",
            display_name: "Rho8 Pointer",
            summary: "Generates a Rho8 stochastic single-flow-direction pointer raster from a DEM.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
                ToolParamSpec { name: "esri_pntr", description: "Use ESRI pointer encoding", required: false },
                ToolParamSpec { name: "output", description: "Output raster path", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("esri_pntr".to_string(), json!(false));
        ToolManifest {
            id: "rho8_pointer".to_string(),
            display_name: "Rho8 Pointer".to_string(),
            summary: "Generates a Rho8 stochastic single-flow-direction pointer raster from a DEM.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![ToolExample {
                name: "rho8_ptr".to_string(),
                description: "Generate Rho8 pointer from DEM".to_string(),
                args: ToolArgs::new(),
            }],
            tags: vec!["hydrology".to_string(), "flow-direction".to_string(), "rho8".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        parse_raster_path_arg(args, "dem")
            .or_else(|_| parse_raster_path_arg(args, "input"))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (input, output_path) = parse_input_and_output(args)?;
        let esri = args
            .get("esri_pntr")
            .or_else(|| args.get("esri_pointer"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let dirs = rho8_dir_from_dem(&input);
        let ptr_vals = d8_pointer_values_from_dir(&dirs, input.rows, input.cols, -32768.0, esri);
        let mut out = input.clone();
        out.data_type = DataType::I16;
        out.nodata = -32768.0;
        for r in 0..input.rows {
            for c in 0..input.cols {
                let i = idx(r, c, input.cols);
                out.set_unchecked(0, r as isize, c as isize, ptr_vals[i]);
            }
        }
        Ok(build_result(write_or_store_output(out, output_path)?))
    }
}

impl Tool for Rho8FlowAccumTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "rho8_flow_accum",
            display_name: "Rho8 Flow Accumulation",
            summary: "Calculates Rho8 flow accumulation from a DEM or Rho8 pointer raster.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM or Rho8 pointer raster", required: true },
                ToolParamSpec { name: "out_type", description: "Output type: cells, ca, or sca", required: false },
                ToolParamSpec { name: "log_transform", description: "Log-transform output", required: false },
                ToolParamSpec { name: "clip", description: "Clip display max (accepted for compatibility)", required: false },
                ToolParamSpec { name: "input_is_pointer", description: "Treat input as Rho8 pointer raster", required: false },
                ToolParamSpec { name: "esri_pntr", description: "Use ESRI pointer encoding for pointer inputs", required: false },
                ToolParamSpec { name: "output", description: "Output raster path", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("out_type".to_string(), json!("sca"));
        defaults.insert("log_transform".to_string(), json!(false));
        defaults.insert("clip".to_string(), json!(false));
        defaults.insert("input_is_pointer".to_string(), json!(false));
        defaults.insert("esri_pntr".to_string(), json!(false));
        ToolManifest {
            id: "rho8_flow_accum".to_string(),
            display_name: "Rho8 Flow Accumulation".to_string(),
            summary: "Calculates Rho8 flow accumulation from a DEM or Rho8 pointer raster.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![ToolExample {
                name: "rho8_accum".to_string(),
                description: "Compute Rho8 specific contributing area from DEM".to_string(),
                args: ToolArgs::new(),
            }],
            tags: vec!["hydrology".to_string(), "flow-accumulation".to_string(), "rho8".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        parse_raster_path_arg(args, "input")
            .or_else(|_| parse_raster_path_arg(args, "raster"))
            .or_else(|_| parse_raster_path_arg(args, "dem"))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (input, output_path) = parse_input_and_output(args)?;
        let out_type = args
            .get("out_type")
            .and_then(|v| v.as_str())
            .unwrap_or("sca")
            .to_lowercase();
        let out_type = if out_type.contains("specific") || out_type.contains("sca") {
            "sca"
        } else if out_type.contains("cells") {
            "cells"
        } else {
            "ca"
        };
        let log_transform = args
            .get("log_transform")
            .or_else(|| args.get("log"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let input_is_pointer = args
            .get("input_is_pointer")
            .or_else(|| args.get("pntr"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let esri = args
            .get("esri_pntr")
            .or_else(|| args.get("esri_pointer"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let flow_dir = if input_is_pointer {
            d8_dir_from_pointer(&input, esri)
        } else {
            rho8_dir_from_dem(&input)
        };

        let mut accum = d8_flow_accum_core(&flow_dir, input.rows, input.cols, -32768.0);
        apply_accum_output_type(
            &mut accum,
            &flow_dir,
            &input,
            -32768.0,
            out_type,
            log_transform,
        );

        let mut out = input.clone();
        out.data_type = DataType::F32;
        out.nodata = -32768.0;
        for r in 0..input.rows {
            for c in 0..input.cols {
                let i = idx(r, c, input.cols);
                out.set_unchecked(0, r as isize, c as isize, accum[i]);
            }
        }
        Ok(build_result(write_or_store_output(out, output_path)?))
    }
}

impl Tool for FD8FlowAccumTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "fd8_flow_accum",
            display_name: "FD8 Flow Accumulation",
            summary: "Calculates FD8 flow accumulation from a DEM.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
                ToolParamSpec { name: "out_type", description: "Output type: cells, ca, or sca", required: false },
                ToolParamSpec { name: "exponent", description: "Slope weighting exponent", required: false },
                ToolParamSpec { name: "convergence_threshold", description: "Threshold above which flow is not dispersed", required: false },
                ToolParamSpec { name: "log_transform", description: "Log-transform output", required: false },
                ToolParamSpec { name: "clip", description: "Clip display max (accepted for compatibility)", required: false },
                ToolParamSpec { name: "output", description: "Output raster path", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("out_type".to_string(), json!("sca"));
        defaults.insert("exponent".to_string(), json!(1.1));
        defaults.insert("convergence_threshold".to_string(), json!(f64::INFINITY));
        defaults.insert("log_transform".to_string(), json!(false));
        defaults.insert("clip".to_string(), json!(false));
        ToolManifest {
            id: "fd8_flow_accum".to_string(),
            display_name: "FD8 Flow Accumulation".to_string(),
            summary: "Calculates FD8 flow accumulation from a DEM.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![ToolExample {
                name: "fd8_accum".to_string(),
                description: "Compute FD8 specific contributing area from DEM".to_string(),
                args: ToolArgs::new(),
            }],
            tags: vec!["hydrology".to_string(), "flow-accumulation".to_string(), "fd8".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        parse_raster_path_arg(args, "dem")
            .or_else(|_| parse_raster_path_arg(args, "input"))
            .or_else(|_| parse_raster_path_arg(args, "raster"))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (input, output_path) = parse_input_and_output(args)?;
        let out_type = args
            .get("out_type")
            .and_then(|v| v.as_str())
            .unwrap_or("sca")
            .to_lowercase();
        let out_type = if out_type.contains("specific") || out_type.contains("sca") {
            "sca"
        } else if out_type.contains("cells") {
            "cells"
        } else {
            "ca"
        };
        let exponent = args.get("exponent").and_then(|v| v.as_f64()).unwrap_or(1.1);
        let convergence_threshold = args
            .get("convergence_threshold")
            .or_else(|| args.get("threshold"))
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::INFINITY);
        let convergence_threshold = if convergence_threshold <= 0.0 { f64::INFINITY } else { convergence_threshold };
        let log_transform = args
            .get("log_transform")
            .or_else(|| args.get("log"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut accum = fd8_flow_accum_core(&input, exponent, convergence_threshold);
        apply_dinf_output_type(&mut accum, &input, out_type, log_transform);

        let mut out = input.clone();
        out.data_type = DataType::F32;
        for r in 0..input.rows {
            for c in 0..input.cols {
                out.set_unchecked(0, r as isize, c as isize, accum[idx(r, c, input.cols)]);
            }
        }

        Ok(build_result(write_or_store_output(out, output_path)?))
    }
}
