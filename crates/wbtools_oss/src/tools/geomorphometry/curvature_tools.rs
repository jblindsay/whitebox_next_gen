use std::collections::BTreeMap;

use rayon::prelude::*;
use serde_json::json;
use wbprojection::{Crs, EpsgIdentifyPolicy, identify_epsg_from_wkt_with_policy};
use wbcore::{
    parse_optional_output_path, parse_raster_path_arg, LicenseTier, Tool, ToolArgs, ToolCategory,
    ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata, ToolParamDescriptor,
    ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::{Raster, RasterFormat};

use crate::memory_store;

pub struct PlanCurvatureTool;
pub struct ProfileCurvatureTool;
pub struct TangentialCurvatureTool;
pub struct TotalCurvatureTool;
pub struct MeanCurvatureTool;
pub struct GaussianCurvatureTool;

#[derive(Clone, Copy)]
enum CurvatureOp {
    Plan,
    Profile,
    Tangential,
    Total,
    Mean,
    Gaussian,
}

impl CurvatureOp {
    fn id(self) -> &'static str {
        match self {
            Self::Plan => "plan_curvature",
            Self::Profile => "profile_curvature",
            Self::Tangential => "tangential_curvature",
            Self::Total => "total_curvature",
            Self::Mean => "mean_curvature",
            Self::Gaussian => "gaussian_curvature",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::Plan => "Plan Curvature",
            Self::Profile => "Profile Curvature",
            Self::Tangential => "Tangential Curvature",
            Self::Total => "Total Curvature",
            Self::Mean => "Mean Curvature",
            Self::Gaussian => "Gaussian Curvature",
        }
    }

    fn summary(self) -> &'static str {
        match self {
            Self::Plan => "Calculates plan (contour) curvature from a DEM.",
            Self::Profile => "Calculates profile curvature from a DEM.",
            Self::Tangential => "Calculates tangential curvature from a DEM.",
            Self::Total => "Calculates total curvature from a DEM.",
            Self::Mean => "Calculates mean curvature from a DEM.",
            Self::Gaussian => "Calculates Gaussian curvature from a DEM.",
        }
    }

    fn license_tier(self) -> LicenseTier {
        LicenseTier::Open
    }
}

impl PlanCurvatureTool {
    fn parse_input(args: &ToolArgs) -> Result<String, ToolError> {
        parse_raster_path_arg(args, "input")
    }

    fn parse_z_factor(args: &ToolArgs) -> f64 {
        args.get("z_factor")
            .or_else(|| args.get("zfactor"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0)
    }

    fn parse_log_transform(args: &ToolArgs) -> bool {
        args.get("log_transform")
            .or_else(|| args.get("log"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    fn load_raster(path: &str) -> Result<Raster, ToolError> {
        if memory_store::raster_is_memory_path(path) {
            let id = memory_store::raster_path_to_id(path).ok_or_else(|| {
                ToolError::Validation(
                    "parameter 'input' has malformed in-memory raster path".to_string(),
                )
            })?;
            return memory_store::get_raster_by_id(id).ok_or_else(|| {
                ToolError::Validation(format!(
                    "parameter 'input' references unknown in-memory raster id '{}': store entry is missing",
                    id
                ))
            });
        }

        Raster::read(path)
            .map_err(|e| ToolError::Execution(format!("failed reading input raster: {}", e)))
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

    fn neighbourhood(input: &Raster, band: isize, row: isize, col: isize, z_factor: f64) -> Option<[f64; 9]> {
        let z5 = input.get(band, row, col);
        if input.is_nodata(z5) {
            return None;
        }
        let offsets = [
            (-1isize, -1isize), (0, -1), (1, -1),
            (-1, 0),          (0, 0),  (1, 0),
            (-1, 1),          (0, 1),  (1, 1),
        ];
        let mut z = [0.0f64; 9];
        for (i, (ox, oy)) in offsets.iter().enumerate() {
            let v = input.get(band, row + *oy, col + *ox);
            z[i] = if input.is_nodata(v) { z5 * z_factor } else { v * z_factor };
        }
        Some(z)
    }

    fn derivatives_projected(
        input: &Raster,
        band: isize,
        row: isize,
        col: isize,
        z_factor: f64,
        dx: f64,
        dy: f64,
    ) -> Option<(f64, f64, f64, f64, f64)> {
        let z = Self::neighbourhood(input, band, row, col, z_factor)?;
        let z1 = z[0];
        let z2 = z[1];
        let z3 = z[2];
        let z4 = z[3];
        let z5 = z[4];
        let z6 = z[5];
        let z7 = z[6];
        let z8 = z[7];
        let z9 = z[8];

        let p = (z6 - z4) / (2.0 * dx);
        let q = (z2 - z8) / (2.0 * dy);
        let r2 = (z4 - 2.0 * z5 + z6) / (dx * dx);
        let t = (z2 - 2.0 * z5 + z8) / (dy * dy);
        let s = (-z1 + z3 + z7 - z9) / (4.0 * dx * dy);
        Some((p, q, r2, s, t))
    }

    fn derivatives_geographic(
        input: &Raster,
        band: isize,
        row: isize,
        col: isize,
        z_factor: f64,
    ) -> Option<(f64, f64, f64, f64, f64)> {
        let z = Self::neighbourhood(input, band, row, col, z_factor)?;

        let phi1 = input.row_center_y(row);
        let lambda1 = input.col_center_x(col);
        let b = Self::haversine_distance_m(phi1, lambda1, phi1, input.col_center_x(col - 1)).max(f64::EPSILON);
        let d = Self::haversine_distance_m(phi1, lambda1, input.row_center_y(row + 1), lambda1).max(f64::EPSILON);
        let e = Self::haversine_distance_m(phi1, lambda1, input.row_center_y(row - 1), lambda1).max(f64::EPSILON);

        let a = Self::haversine_distance_m(
            input.row_center_y(row + 1),
            input.col_center_x(col),
            input.row_center_y(row + 1),
            input.col_center_x(col - 1),
        )
        .max(f64::EPSILON);
        let c = Self::haversine_distance_m(
            input.row_center_y(row - 1),
            input.col_center_x(col),
            input.row_center_y(row - 1),
            input.col_center_x(col - 1),
        )
        .max(f64::EPSILON);

        let r2 = (c * c * (z[0] + z[2] - 2.0 * z[1])
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

        Some((p, q, r2, s, t))
    }

    #[allow(dead_code)]
    fn neighbourhood_5x5(
        input: &Raster,
        band: isize,
        row: isize,
        col: isize,
        z_factor: f64,
    ) -> Option<[f64; 25]> {
        let z_center = input.get(band, row, col);
        if input.is_nodata(z_center) {
            return None;
        }
        let mut z = [0.0f64; 25];
        let mut i = 0usize;
        for oy in -2..=2 {
            for ox in -2..=2 {
                let v = input.get(band, row + oy, col + ox);
                z[i] = if input.is_nodata(v) {
                    z_center * z_factor
                } else {
                    v * z_factor
                };
                i += 1;
            }
        }
        Some(z)
    }

    #[allow(dead_code)]
    fn projected_5x5_derivs(z: &[f64; 25], res: f64) -> (f64, f64, f64, f64, f64, f64, f64, f64, f64) {
        let r = 1.0 / (35.0 * res * res)
            * (2.0
                * (z[0] + z[4] + z[5] + z[9] + z[10] + z[14] + z[15] + z[19] + z[20] + z[24])
                - 2.0 * (z[2] + z[7] + z[12] + z[17] + z[22])
                - z[1]
                - z[3]
                - z[6]
                - z[8]
                - z[11]
                - z[13]
                - z[16]
                - z[18]
                - z[21]
                - z[23]);

        let t = 1.0 / (35.0 * res * res)
            * (2.0 * (z[0] + z[1] + z[2] + z[3] + z[4] + z[20] + z[21] + z[22] + z[23] + z[24])
                - 2.0 * (z[10] + z[11] + z[12] + z[13] + z[14])
                - z[5]
                - z[6]
                - z[7]
                - z[8]
                - z[9]
                - z[15]
                - z[16]
                - z[17]
                - z[18]
                - z[19]);

        let s = 1.0 / (100.0 * res * res)
            * (z[8]
                + z[16]
                - z[6]
                - z[18]
                + 4.0 * (z[4] + z[20] - z[0] - z[24])
                + 2.0 * (z[3] + z[9] + z[15] + z[21] - z[1] - z[5] - z[19] - z[23]));

        let p = 1.0 / (420.0 * res)
            * (44.0 * (z[3] + z[23] - z[1] - z[21])
                + 31.0
                    * (z[0] + z[20] - z[4] - z[24] + 2.0 * (z[8] + z[18] - z[6] - z[16]))
                + 17.0 * (z[14] - z[10] + 4.0 * (z[13] - z[11]))
                + 5.0 * (z[9] + z[19] - z[5] - z[15]));

        let q = 1.0 / (420.0 * res)
            * (44.0 * (z[5] + z[9] - z[15] - z[19])
                + 31.0
                    * (z[20] + z[24] - z[0] - z[4] + 2.0 * (z[6] + z[8] - z[16] - z[18]))
                + 17.0 * (z[2] - z[22] + 4.0 * (z[7] - z[17]))
                + 5.0 * (z[1] + z[3] - z[21] - z[23]));

        let h = 1.0 / (10.0 * res.powi(3))
            * (z[0]
                + z[1]
                + z[2]
                + z[3]
                + z[4]
                - z[20]
                - z[21]
                - z[22]
                - z[23]
                - z[24]
                + 2.0
                    * (z[15] + z[16] + z[17] + z[18] + z[19]
                        - z[5]
                        - z[6]
                        - z[7]
                        - z[8]
                        - z[9]));

        let g = 1.0 / (10.0 * res.powi(3))
            * (z[4]
                + z[9]
                + z[14]
                + z[19]
                + z[24]
                - z[0]
                - z[5]
                - z[10]
                - z[15]
                - z[20]
                + 2.0
                    * (z[1] + z[6] + z[11] + z[16] + z[21]
                        - z[3]
                        - z[8]
                        - z[13]
                        - z[18]
                        - z[23]));

        let m = 1.0 / (70.0 * res.powi(3))
            * (z[6]
                + z[16]
                - z[8]
                - z[18]
                + 4.0 * (z[4] + z[10] + z[24] - z[0] - z[14] - z[20])
                + 2.0
                    * (z[3] + z[5] + z[11] + z[15] + z[23]
                        - z[1]
                        - z[9]
                        - z[13]
                        - z[19]
                        - z[21]));

        let k = 1.0 / (70.0 * res.powi(3))
            * (z[16]
                + z[18]
                - z[6]
                - z[8]
                + 4.0 * (z[0] + z[4] + z[22] - z[2] - z[20] - z[24])
                + 2.0
                    * (z[5] + z[9] + z[17] + z[21] + z[23]
                        - z[1]
                        - z[3]
                        - z[7]
                        - z[15]
                        - z[19]));

        (p, q, r, s, t, h, g, m, k)
    }

    #[allow(dead_code)]
    fn gaussian_scale_space_smooth(input: &Raster, radius: usize) -> Raster {
        if radius == 0 {
            return input.clone();
        }

        let filter_size = radius * 2 + 1;
        if filter_size <= 3 {
            // Legacy behaviour: no smoothing branch for 3x3 scale-space filter.
            return input.clone();
        }

        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let sigma = (radius as f64 + 0.5) / 3.0;

        let mut out = input.clone();

        for b in 0..bands {
            let band = b as isize;
            let src: Vec<f64> = (0..rows * cols)
                .into_par_iter()
                .map(|idx| {
                    let row = idx / cols;
                    let col = idx % cols;
                    input.get(band, row as isize, col as isize)
                })
                .collect();

            let result = if sigma < 1.8 {
                // Direct Gaussian convolution for narrower scales.
                let recip = 1.0 / ((2.0 * std::f64::consts::PI).sqrt() * sigma);
                let two_sigma_sq = 2.0 * sigma * sigma;

                let mut filter_size_smooth = 3usize;
                for i in 0..250usize {
                    let w = recip * (-((i * i) as f64) / two_sigma_sq).exp();
                    if w <= 0.001 {
                        filter_size_smooth = i * 2 + 1;
                        break;
                    }
                }
                if filter_size_smooth % 2 == 0 {
                    filter_size_smooth += 1;
                }
                filter_size_smooth = filter_size_smooth.max(3);
                let mid = (filter_size_smooth / 2) as isize;

                let mut offsets: Vec<(isize, isize, f64)> =
                    Vec::with_capacity(filter_size_smooth * filter_size_smooth);
                for fy in 0..filter_size_smooth {
                    for fx in 0..filter_size_smooth {
                        let dx = fx as isize - mid;
                        let dy = fy as isize - mid;
                        let w = recip * (-((dx * dx + dy * dy) as f64) / two_sigma_sq).exp();
                        offsets.push((dx, dy, w));
                    }
                }

                let mut smoothed = vec![nodata; rows * cols];
                smoothed
                    .par_chunks_mut(cols)
                    .enumerate()
                    .for_each(|(row, row_out)| {
                        for (col, cell_out) in row_out.iter_mut().enumerate() {
                            let idx = row * cols + col;
                            if src[idx] == nodata {
                                continue;
                            }

                            let mut sum_w = 0.0;
                            let mut sum_z = 0.0;
                            for (dx, dy, w) in &offsets {
                                let rr = row as isize + *dy;
                                let cc = col as isize + *dx;
                                if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                                    continue;
                                }
                                let z = src[rr as usize * cols + cc as usize];
                                if z == nodata {
                                    continue;
                                }
                                sum_w += *w;
                                sum_z += *w * z;
                            }

                            if sum_w > 0.0 {
                                *cell_out = sum_z / sum_w;
                            }
                        }
                    });
                smoothed
            } else {
                // Fast almost-Gaussian smoothing for broader scales.
                let n = 4usize;
                let w_ideal = (12.0 * sigma * sigma / n as f64 + 1.0).sqrt();
                let mut wl = w_ideal.floor() as isize;
                if wl % 2 == 0 {
                    wl -= 1;
                }
                let wu = wl + 2;
                let m = ((12.0 * sigma * sigma
                    - (n as isize * wl * wl) as f64
                    - (4 * n as isize * wl) as f64
                    - (3 * n as isize) as f64)
                    / (-4 * wl - 4) as f64)
                    .round() as isize;

                let valid: Vec<u32> = src
                    .par_iter()
                    .map(|&z| if z == nodata { 0 } else { 1 })
                    .collect();
                let mut i_n = vec![0u32; rows * cols];
                for row in 0..rows {
                    let mut row_sum = 0u32;
                    for col in 0..cols {
                        row_sum += valid[row * cols + col];
                        let idx = row * cols + col;
                        i_n[idx] = if row > 0 {
                            i_n[(row - 1) * cols + col] + row_sum
                        } else {
                            row_sum
                        };
                    }
                }

                let mut current = src.clone();
                let mut next = vec![nodata; rows * cols];
                let mut integral = vec![0.0f64; rows * cols];

                let get_u32 = |grid: &[u32], r: usize, c: usize| -> u32 { grid[r * cols + c] };
                let get_f64 = |grid: &[f64], r: usize, c: usize| -> f64 { grid[r * cols + c] };

                for iter in 0..n {
                    let midpoint = if iter as isize <= m { wl / 2 } else { wu / 2 };

                    for row in 0..rows {
                        let mut row_sum = 0.0f64;
                        for col in 0..cols {
                            let idx = row * cols + col;
                            let v = if current[idx] == nodata {
                                0.0
                            } else {
                                current[idx]
                            };
                            row_sum += v;
                            integral[idx] = if row > 0 {
                                row_sum + integral[(row - 1) * cols + col]
                            } else {
                                row_sum
                            };
                        }
                    }

                    next
                        .par_chunks_mut(cols)
                        .enumerate()
                        .for_each(|(row, row_out)| {
                            let mut y1 = row as isize - midpoint - 1;
                            if y1 < 0 {
                                y1 = 0;
                            }
                            let mut y2 = row as isize + midpoint;
                            if y2 >= rows as isize {
                                y2 = rows as isize - 1;
                            }

                            for (col, cell_out) in row_out.iter_mut().enumerate() {
                                let idx = row * cols + col;
                                if src[idx] == nodata {
                                    *cell_out = nodata;
                                    continue;
                                }

                                let mut x1 = col as isize - midpoint - 1;
                                if x1 < 0 {
                                    x1 = 0;
                                }
                                let mut x2 = col as isize + midpoint;
                                if x2 >= cols as isize {
                                    x2 = cols as isize - 1;
                                }

                                let y1u = y1 as usize;
                                let y2u = y2 as usize;
                                let x1u = x1 as usize;
                                let x2u = x2 as usize;

                                let num_cells = get_u32(&i_n, y2u, x2u)
                                    + get_u32(&i_n, y1u, x1u)
                                    - get_u32(&i_n, y1u, x2u)
                                    - get_u32(&i_n, y2u, x1u);

                                if num_cells > 0 {
                                    let sum = get_f64(&integral, y2u, x2u)
                                        + get_f64(&integral, y1u, x1u)
                                        - get_f64(&integral, y1u, x2u)
                                        - get_f64(&integral, y2u, x1u);
                                    *cell_out = sum / num_cells as f64;
                                } else {
                                    *cell_out = 0.0;
                                }
                            }
                        });

                    std::mem::swap(&mut current, &mut next);
                }

                current
            };

            for row in 0..rows {
                let start = row * cols;
                let end = start + cols;
                let _ = out.set_row_slice(band, row as isize, &result[start..end]);
            }
        }

        out
    }

    fn metadata_for(op: CurvatureOp) -> ToolMetadata {
        ToolMetadata {
            id: op.id(),
            display_name: op.display_name(),
            summary: op.summary(),
            category: ToolCategory::Raster,
            license_tier: op.license_tier(),
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input DEM raster path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "z_factor",
                    description: "Optional z conversion factor when vertical and horizontal units differ (default 1.0). Alias: zfactor.",
                    required: false,
                },
                ToolParamSpec {
                    name: "log_transform",
                    description: "Optional log-transform of output values for readability (default false). Alias: log.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output path. If omitted, output remains in memory.",
                    required: false,
                },
            ],
        }
    }

    fn manifest_for(op: CurvatureOp) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("z_factor".to_string(), json!(1.0));
        defaults.insert("log_transform".to_string(), json!(false));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));
        example_args.insert("z_factor".to_string(), json!(1.0));
        example_args.insert("log_transform".to_string(), json!(false));
        example_args.insert("output".to_string(), json!(format!("{}.tif", op.id())));

        ToolManifest {
            id: op.id().to_string(),
            display_name: op.display_name().to_string(),
            summary: op.summary().to_string(),
            category: ToolCategory::Raster,
            license_tier: op.license_tier(),
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input DEM raster path or typed raster object.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "z_factor".to_string(),
                    description: "Optional z conversion factor when vertical and horizontal units differ (default 1.0). Alias: zfactor.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "log_transform".to_string(),
                    description: "Optional log-transform of output values for readability (default false). Alias: log.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output path. If omitted, result is stored in memory.".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: format!("basic_{}", op.id()),
                description: format!("Calculates {} from a DEM.", op.id()),
                args: example_args,
            }],
            tags: vec![
                "geomorphometry".to_string(),
                "terrain".to_string(),
                "curvature".to_string(),
                op.id().to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
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

    fn run_with_op(op: CurvatureOp, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let z_factor = Self::parse_z_factor(args);
        let log_transform = Self::parse_log_transform(args);

        ctx.progress.info(&format!("running {}", op.id()));
        ctx.progress.info("reading input raster");

        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();

        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let dx = input.cell_size_x.abs().max(f64::EPSILON);
        let dy = input.cell_size_y.abs().max(f64::EPSILON);
        let is_geographic = Self::raster_is_geographic(&input);
        let log_multiplier = Self::log_multiplier((dx + dy) / 2.0);

        for band_idx in 0..bands {
            let band = band_idx as isize;

            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let row = r as isize;
                        let col = c as isize;
                        let derivs = if is_geographic {
                            Self::derivatives_geographic(&input, band, row, col, z_factor)
                        } else {
                            Self::derivatives_projected(&input, band, row, col, z_factor, dx, dy)
                        };
                        let Some((p, q, r2, s, t)) = derivs else {
                            continue;
                        };

                        let g2 = p * p + q * q;
                        let w = 1.0 + g2; // 1 + p² + q²
                        let mean_curv = -((1.0 + q * q) * r2 - 2.0 * p * q * s + (1.0 + p * p) * t)
                            / (2.0 * w.powf(1.5));
                        let gaussian_curv = (r2 * t - s * s) / w.powi(2);
                        let mut curv = match op {
                            CurvatureOp::Plan => {
                                if g2 <= f64::EPSILON {
                                    0.0
                                } else {
                                    let denom = g2.powf(1.5);
                                    if denom <= f64::EPSILON { 0.0 }
                                    else { -(q * q * r2 - 2.0 * p * q * s + p * p * t) / denom }
                                }
                            }
                            CurvatureOp::Profile => {
                                if g2 <= f64::EPSILON {
                                    0.0
                                } else {
                                    let denom = g2 * w.powf(1.5);
                                    if denom <= f64::EPSILON { 0.0 }
                                    else { -(p * p * r2 + 2.0 * p * q * s + q * q * t) / denom }
                                }
                            }
                            CurvatureOp::Tangential => {
                                if g2 <= f64::EPSILON {
                                    0.0
                                } else {
                                    let denom = g2.sqrt() * w.sqrt();
                                    if denom <= f64::EPSILON { 0.0 }
                                    else { -(q * q * r2 - 2.0 * p * q * s + p * p * t) / denom }
                                }
                            }
                            CurvatureOp::Total => r2 + t,
                            // Florinsky (2016) Ch. 2 p. 19 — denominator always ≥ 2, no guard needed.
                            CurvatureOp::Mean => mean_curv,
                            // Florinsky (2016) Ch. 2 p. 18 — denominator always ≥ 1.
                            CurvatureOp::Gaussian => gaussian_curv,
                        };

                        if log_transform {
                            curv = curv.signum() * (1.0 + log_multiplier * curv.abs()).ln();
                        }

                        row_out[c] = curv;
                    }
                    row_out
                })
                .collect();

            for (r, row) in row_data.iter().enumerate() {
                output
                    .set_row_slice(band, r as isize, row)
                    .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            }

            ctx.progress.progress((band_idx + 1) as f64 / bands as f64);
        }

        let output_locator = Self::write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("active_band".to_string(), json!(0));
        Ok(ToolRunResult { outputs })
    }
}

macro_rules! define_curvature_tool {
    ($tool:ident, $op:expr) => {
        impl Tool for $tool {
            fn metadata(&self) -> ToolMetadata {
                PlanCurvatureTool::metadata_for($op)
            }

            fn manifest(&self) -> ToolManifest {
                PlanCurvatureTool::manifest_for($op)
            }

            fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
                let _ = PlanCurvatureTool::parse_input(args)?;
                let _ = parse_optional_output_path(args, "output")?;
                let _ = PlanCurvatureTool::parse_z_factor(args);
                let _ = PlanCurvatureTool::parse_log_transform(args);
                Ok(())
            }

            fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
                PlanCurvatureTool::run_with_op($op, args, ctx)
            }
        }
    };
}

define_curvature_tool!(PlanCurvatureTool, CurvatureOp::Plan);
define_curvature_tool!(ProfileCurvatureTool, CurvatureOp::Profile);
define_curvature_tool!(TangentialCurvatureTool, CurvatureOp::Tangential);
define_curvature_tool!(TotalCurvatureTool, CurvatureOp::Total);
define_curvature_tool!(MeanCurvatureTool, CurvatureOp::Mean);
define_curvature_tool!(GaussianCurvatureTool, CurvatureOp::Gaussian);

#[cfg(test)]
mod tests {
    use super::*;
    use wbcore::{AllowAllCapabilities, ProgressSink, ToolContext};
    use wbraster::RasterConfig;

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

    fn run_with_memory(tool: &dyn Tool, args: &mut ToolArgs, input: Raster) -> Raster {
        let id = memory_store::put_raster(input);
        let input_path = memory_store::make_raster_memory_path(&id);
        args.insert("input".to_string(), json!(input_path));
        let result = tool.run(args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap().to_string();
        let out_id = memory_store::raster_path_to_id(&out_path).unwrap();
        memory_store::get_raster_by_id(out_id).unwrap()
    }

    #[test]
    fn curvature_tools_constant_raster_returns_zero() {
        let mut args = ToolArgs::new();
        args.insert("z_factor".to_string(), json!(1.0));
        args.insert("log_transform".to_string(), json!(false));

        let plan = run_with_memory(&PlanCurvatureTool, &mut args.clone(), make_constant_raster(20, 20, 10.0));
        let prof = run_with_memory(
            &ProfileCurvatureTool,
            &mut args.clone(),
            make_constant_raster(20, 20, 10.0),
        );
        let tan = run_with_memory(
            &TangentialCurvatureTool,
            &mut args.clone(),
            make_constant_raster(20, 20, 10.0),
        );
        let total = run_with_memory(
            &TotalCurvatureTool,
            &mut args,
            make_constant_raster(20, 20, 10.0),
        );

        let mean = run_with_memory(
            &MeanCurvatureTool,
            &mut args.clone(),
            make_constant_raster(20, 20, 10.0),
        );
        let gaussian = run_with_memory(
            &GaussianCurvatureTool,
            &mut args,
            make_constant_raster(20, 20, 10.0),
        );

        assert!(plan.get(0, 10, 10).abs() < 1e-12);
        assert!(prof.get(0, 10, 10).abs() < 1e-12);
        assert!(tan.get(0, 10, 10).abs() < 1e-12);
        assert!(total.get(0, 10, 10).abs() < 1e-12);
        assert!(mean.get(0, 10, 10).abs() < 1e-12);
        assert!(gaussian.get(0, 10, 10).abs() < 1e-12);
    }
}
