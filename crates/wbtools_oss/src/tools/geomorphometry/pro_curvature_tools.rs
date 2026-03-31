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

use wbraster::memory_store;

// --- derivative mask constants for selective computation (Optimization 1) ------

/// Bitflags for selective derivative computation.
/// Only compute p, q, r, s, t that are actually needed by the specific operation.
const DERIV_P: u8 = 0x01;
const DERIV_Q: u8 = 0x02;
const DERIV_R: u8 = 0x04;
const DERIV_S: u8 = 0x08;
const DERIV_T: u8 = 0x10;
const DERIV_ALL: u8 = DERIV_P | DERIV_Q | DERIV_R | DERIV_S | DERIV_T;

/// Result struct for selective derivatives (Optimization 1)
#[derive(Clone, Copy)]
struct SelectiveDerivatives {
    p: f64,
    q: f64,
    r: f64,
    s: f64,
    t: f64,
}

// --- public structs for each PRO curvature tool --------------------------------

pub struct MinimalCurvatureTool;
pub struct MaximalCurvatureTool;
pub struct ShapeIndexTool;
pub struct CurvednessTool;
pub struct UnsphericityCurvatureTool;
pub struct RingCurvatureTool;
pub struct RotorTool;
pub struct DifferenceCurvatureTool;
pub struct HorizontalExcessCurvatureTool;
pub struct VerticalExcessCurvatureTool;
pub struct AccumulationCurvatureTool;
pub struct GeneratingFunctionTool;
pub struct PrincipalCurvatureDirectionTool;
pub struct CasoratiCurvatureTool;

// --- operation enum -----------------------------------------------------------

#[derive(Clone, Copy)]
enum ProCurvatureOp {
    Minimal,
    Maximal,
    ShapeIndex,
    Curvedness,
    Unsphericity,
    Ring,
    Rotor,
    Difference,
    HorizontalExcess,
    VerticalExcess,
    Accumulation,
    GeneratingFunction,
    PrincipalCurvatureDirection,
    Casorati,
}

impl ProCurvatureOp {
    fn id(self) -> &'static str {
        match self {
            Self::Minimal => "minimal_curvature",
            Self::Maximal => "maximal_curvature",
            Self::ShapeIndex => "shape_index",
            Self::Curvedness => "curvedness",
            Self::Unsphericity => "unsphericity",
            Self::Ring => "ring_curvature",
            Self::Rotor => "rotor",
            Self::Difference => "difference_curvature",
            Self::HorizontalExcess => "horizontal_excess_curvature",
            Self::VerticalExcess => "vertical_excess_curvature",
            Self::Accumulation => "accumulation_curvature",
            Self::GeneratingFunction => "generating_function",
            Self::PrincipalCurvatureDirection => "principal_curvature_direction",
            Self::Casorati => "casorati_curvature",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::Minimal => "Minimal Curvature",
            Self::Maximal => "Maximal Curvature",
            Self::ShapeIndex => "Shape Index",
            Self::Curvedness => "Curvedness",
            Self::Unsphericity => "Unsphericity",
            Self::Ring => "Ring Curvature",
            Self::Rotor => "Rotor",
            Self::Difference => "Difference Curvature",
            Self::HorizontalExcess => "Horizontal Excess Curvature",
            Self::VerticalExcess => "Vertical Excess Curvature",
            Self::Accumulation => "Accumulation Curvature",
            Self::GeneratingFunction => "Generating Function",
            Self::PrincipalCurvatureDirection => "Principal Curvature Direction",
            Self::Casorati => "Casorati Curvature",
        }
    }

    fn summary(self) -> &'static str {
        match self {
            Self::Minimal => "Calculates minimal (minimum principal) curvature from a DEM.",
            Self::Maximal => "Calculates maximal (maximum principal) curvature from a DEM.",
            Self::ShapeIndex => "Calculates the shape index surface form descriptor from a DEM.",
            Self::Curvedness => "Calculates the curvedness surface form descriptor from a DEM.",
            Self::Unsphericity => "Calculates the unsphericity curvature (half the difference of principal curvatures) from a DEM.",
            Self::Ring => "Calculates ring curvature (squared flow-line twisting) from a DEM.",
            Self::Rotor => "Calculates the rotor (flow-line twisting) from a DEM.",
            Self::Difference => "Calculates difference curvature from a DEM.",
            Self::HorizontalExcess => "Calculates horizontal excess curvature from a DEM.",
            Self::VerticalExcess => "Calculates vertical excess curvature from a DEM.",
            Self::Accumulation => "Calculates accumulation curvature from a DEM.",
            Self::GeneratingFunction => "Calculates generating function from a DEM.",
            Self::PrincipalCurvatureDirection => "Calculates the principal curvature direction angle (degrees).",
            Self::Casorati => "Calculates Casorati curvature from a DEM.",
        }
    }

    /// Optimization 1: Return bitmask indicating which derivatives are needed.
    /// Only compute p, q, r, s, t that this operation actually requires.
    #[inline]
    fn required_derivatives(self) -> u8 {
        match self {
            // All operations except principal_curvature_direction need r, t, s (second derivatives)
            // p and q are used in most but only to compute g2 = p² + q²
            Self::Minimal | Self::Maximal | Self::ShapeIndex | Self::Curvedness
            | Self::Unsphericity | Self::Ring | Self::Rotor | Self::Difference
            | Self::HorizontalExcess | Self::VerticalExcess | Self::Accumulation
            | Self::Casorati => DERIV_ALL, // Most operations use all derivatives
            
            Self::PrincipalCurvatureDirection => 
                DERIV_R | DERIV_S | DERIV_T, // Only needs second derivatives (r, s, t)
            
            Self::GeneratingFunction => DERIV_ALL, // Needs all five for complex formula
        }
    }
}

// --- shared implementation driver ---------------------------------------------

struct ProCurvatureCore;

impl ProCurvatureCore {
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
        let mut idx = 0usize;
        for oy in -2..=2 {
            for ox in -2..=2 {
                let v = input.get(band, row + oy, col + ox);
                z[idx] = if input.is_nodata(v) {
                    z_center * z_factor
                } else {
                    v * z_factor
                };
                idx += 1;
            }
        }
        Some(z)
    }

    #[inline]
    fn geographic_resolution_m(input: &Raster, row: isize, col: isize) -> f64 {
        let phi1 = input.row_center_y(row);
        let lambda1 = input.col_center_x(col);

        let x_res = Self::haversine_distance_m(phi1, lambda1, phi1, input.col_center_x(col + 1));
        let y_res = Self::haversine_distance_m(phi1, lambda1, input.row_center_y(row + 1), lambda1);

        ((x_res + y_res) / 2.0).max(f64::EPSILON)
    }

    fn generating_function_value(
        input: &Raster,
        band: isize,
        row: isize,
        col: isize,
        z_factor: f64,
        is_geographic: bool,
        dx: f64,
        dy: f64,
    ) -> Option<f64> {
        let z = Self::neighbourhood_5x5(input, band, row, col, z_factor)?;
        let res = if is_geographic {
            Self::geographic_resolution_m(input, row, col)
        } else {
            (dx + dy) / 2.0
        }
        .max(f64::EPSILON);

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

        let g2 = p * p + q * q;
        if g2 <= f64::EPSILON {
            return Some(0.0);
        }

        let w = 1.0 + g2;
        let rotor = ((p * p - q * q) * s - p * q * (r - t)) / g2.powi(3).sqrt();
        let horizontal_curv = (q * q * r - 2.0 * p * q * s + p * p * t) / (g2 * w.sqrt());
        let generating_fn = (q.powi(3) * g - 3.0 * p * q * q * k + 3.0 * p * p * q * m - p.powi(3) * h)
            / (g2.powi(3) * w).sqrt()
            - horizontal_curv * rotor * (2.0 + 3.0 * g2) / w;

        Some(if generating_fn.is_finite() { generating_fn } else { 0.0 })
    }

    fn derivatives_projected(
        input: &Raster,
        band: isize,
        row: isize,
        col: isize,
        z_factor: f64,
        dx: f64,
        dy: f64,
        mask: u8,
    ) -> Option<SelectiveDerivatives> {
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

        // Optimization 1: Only compute derivatives that are actually needed
        let p = if (mask & DERIV_P) != 0 {
            (z6 - z4) / (2.0 * dx)
        } else {
            0.0
        };
        let q = if (mask & DERIV_Q) != 0 {
            (z2 - z8) / (2.0 * dy)
        } else {
            0.0
        };
        let r = if (mask & DERIV_R) != 0 {
            (z4 - 2.0 * z5 + z6) / (dx * dx)
        } else {
            0.0
        };
        let t = if (mask & DERIV_T) != 0 {
            (z2 - 2.0 * z5 + z8) / (dy * dy)
        } else {
            0.0
        };
        let s = if (mask & DERIV_S) != 0 {
            (-z1 + z3 + z7 - z9) / (4.0 * dx * dy)
        } else {
            0.0
        };
        Some(SelectiveDerivatives { p, q, r, s, t })
    }

    fn derivatives_geographic(
        input: &Raster,
        band: isize,
        row: isize,
        col: isize,
        z_factor: f64,
        mask: u8,
    ) -> Option<SelectiveDerivatives> {
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

        // Optimization 1: Only compute derivatives that are needed
        let r = if (mask & DERIV_R) != 0 {
            (c * c * (z[0] + z[2] - 2.0 * z[1])
                + b * b * (z[3] + z[5] - 2.0 * z[4])
                + a * a * (z[6] + z[8] - 2.0 * z[7]))
                / (a.powi(4) + b.powi(4) + c.powi(4))
        } else {
            0.0
        };

        let t_denom = 3.0 * d * e * (d + e) * (a.powi(4) + b.powi(4) + c.powi(4));
        let t = if (mask & DERIV_T) != 0 {
            2.0
                / t_denom
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
                        - c * c * e * (a * a - b * b) * z[1]))
        } else {
            0.0
        };

        let s = if (mask & DERIV_S) != 0 {
            (c * (a * a * (d + e) + b * b * e) * (z[2] - z[0])
                - b * (a * a * d - c * c * e) * (z[3] - z[5])
                + a * (c * c * (d + e) + b * b * d) * (z[6] - z[8]))
                / (2.0 * (a * a * c * c * (d + e).powi(2) + b * b * (a * a * d * d + c * c * e * e)))
        } else {
            0.0
        };

        let p = if (mask & DERIV_P) != 0 {
            (a * a * c * d * (d + e) * (z[2] - z[0])
                + b * (a * a * d * d + c * c * e * e) * (z[5] - z[3])
                + a * c * c * e * (d + e) * (z[8] - z[6]))
                / (2.0 * (a * a * c * c * (d + e).powi(2) + b * b * (a * a * d * d + c * c * e * e)))
        } else {
            0.0
        };

        let q = if (mask & DERIV_Q) != 0 {
            1.0 / (3.0 * d * e * (d + e) * (a.powi(4) + b.powi(4) + c.powi(4)))
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
                        + c * c * e * e * (a * a - b * b) * z[1]))
        } else {
            0.0
        };

        Some(SelectiveDerivatives { p, q, r, s, t })
    }

    fn metadata_for(op: ProCurvatureOp) -> ToolMetadata {
        ToolMetadata {
            id: op.id(),
            display_name: op.display_name(),
            summary: op.summary(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
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
                    description: "Optional log-transform of output values (default false). Alias: log.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output path. If omitted, result is stored in memory.",
                    required: false,
                },
            ],
        }
    }

    fn manifest_for(op: ProCurvatureOp) -> ToolManifest {
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
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input DEM raster path or typed raster object.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "z_factor".to_string(),
                    description: "Optional z conversion factor (default 1.0). Alias: zfactor.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "log_transform".to_string(),
                    description: "Optional log-transform of output values (default false). Alias: log.".to_string(),
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

    fn run_with_op(
        op: ProCurvatureOp,
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
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
        
        // Optimization 1: Get the derivative mask for this operation
        let deriv_mask = op.required_derivatives();

        if is_geographic && matches!(op, ProCurvatureOp::GeneratingFunction) {
            ctx.progress.info(
                "warning: generating_function in geographic CRS uses local distance approximation",
            );
        }

        for band_idx in 0..bands {
            let band = band_idx as isize;

            // Optimization 4: Process in tiles for better cache locality
            const TILE_SIZE: usize = 64;
            let tile_overlap = 1; // 1-cell overlap for neighborhood stencil
            
            let tile_rows = (rows + TILE_SIZE - 1) / TILE_SIZE;
            let tile_cols = (cols + TILE_SIZE - 1) / TILE_SIZE;

            let row_data: Vec<Vec<f64>> = (0..tile_rows * tile_cols)
                .into_par_iter()
                .flat_map(|tile_idx| {
                    let tile_row = tile_idx / tile_cols;
                    let tile_col = tile_idx % tile_cols;
                    
                    let r_start = (tile_row * TILE_SIZE).saturating_sub(tile_overlap);
                    let r_end = ((tile_row + 1) * TILE_SIZE + tile_overlap).min(rows);
                    let c_start = (tile_col * TILE_SIZE).saturating_sub(tile_overlap);
                    let c_end = ((tile_col + 1) * TILE_SIZE + tile_overlap).min(cols);
                    
                    let mut tile_rows_vec = vec![vec![nodata; cols]; r_end - r_start];
                    
                    for r in r_start..r_end {
                        let row_out = &mut tile_rows_vec[r - r_start];
                        for c in c_start..c_end {
                            let row = r as isize;
                            let col = c as isize;
                            
                            let derivs = if is_geographic {
                                Self::derivatives_geographic(&input, band, row, col, z_factor, deriv_mask)
                            } else {
                                Self::derivatives_projected(&input, band, row, col, z_factor, dx, dy, deriv_mask)
                            };
                            let Some(d) = derivs else {
                                continue;
                            };

                            let p = d.p;
                            let q = d.q;
                            let r = d.r;
                            let s = d.s;
                            let t = d.t;

                            let g2 = p * p + q * q;
                            let w  = 1.0 + g2; // 1 + p² + q²

                            // Optimization 2: Use mul_add and replace powf(1.5) with x * x.sqrt()
                            let w_sqrt = w.sqrt();
                            let w_pow_1p5 = w * w_sqrt; // 1.5 exponent = w^1.5 = w * w^0.5
                            
                            // Common intermediates used by several tools.
                            // mean: Florinsky (2016) Ch. 2 p. 19
                            let mean_curv = -((1.0 + q * q).mul_add(r, 
                                (1.0 + p * p).mul_add(t, -2.0 * p * q * s)))
                                / (2.0 * w_pow_1p5);
                            
                            // gaussian: Florinsky (2016) Ch. 2 p. 18
                            // Optimization 2: Replace powi(2) with * and use mul_add
                            let r_t_minus_s2 = r * t - s * s;
                            let w_squared = w * w;
                            let gaussian_curv = r_t_minus_s2 / w_squared;

                            // discriminant = mean² – K ≥ 0 by the principal-curvature identity;
                            // clamp to zero to guard against floating-point noise.
                            let disc = (mean_curv * mean_curv - gaussian_curv).max(0.0);
                            let sqrt_disc = disc.sqrt();

                            let minimal_curv = mean_curv - sqrt_disc;
                            let maximal_curv = mean_curv + sqrt_disc;

                            // diff_curv: Florinsky (2016) Ch. 2 p. 50 eq. 2.100
                            // (tangential – profile direction curvature difference)
                            let diff_curv = if g2 > f64::EPSILON {
                                // Optimization 2: Use mul_add for efficiency
                                let numerator = q * q.mul_add(r, 
                                    p * p.mul_add(t, -2.0 * p * q * s));
                                let denominator = g2 * w_sqrt;
                                numerator / denominator
                                    - ((1.0 + q * q).mul_add(r, 
                                        (1.0 + p * p).mul_add(t, -2.0 * p * q * s)))
                                        / (2.0 * w_pow_1p5)
                            } else {
                                0.0
                            };

                            let mut curv = match op {
                                // Minimal (lowest principal) curvature.
                                ProCurvatureOp::Minimal => minimal_curv,
                                // Maximal (highest principal) curvature.
                                ProCurvatureOp::Maximal => maximal_curv,

                                // Shape index: Koenderink & van Doorn (1992);
                                // Florinsky (2017) eq. 26. Defined only when maximal ≠ minimal.
                                ProCurvatureOp::ShapeIndex => {
                                    let denom = maximal_curv - minimal_curv;
                                    if denom.abs() <= f64::EPSILON {
                                        0.0
                                    } else {
                                        2.0 / std::f64::consts::PI
                                            * ((maximal_curv + minimal_curv) / denom).atan()
                                    }
                                }

                                // Curvedness: RMS of principal curvatures.
                                // Optimization 2: Replace powi(2) with * multiplication
                                ProCurvatureOp::Curvedness => {
                                    ((minimal_curv * minimal_curv + maximal_curv * maximal_curv) / 2.0)
                                        .sqrt()
                                }

                                // Unsphericity: half the range of principal curvatures.
                                ProCurvatureOp::Unsphericity => sqrt_disc,

                                // Ring curvature: Florinsky (2016)
                                // Defined only when g2 > 0.
                                ProCurvatureOp::Ring => {
                                    if g2 <= f64::EPSILON {
                                        0.0
                                    } else {
                                        let num = (p * p - q * q).mul_add(s, -p * q * (r - t));
                                        let denom = g2 * w;
                                        (num / denom) * (num / denom)  // Optimization 2: x² instead of powi(2)
                                    }
                                }

                                // Rotor (flow-line twisting): Florinsky (2016)
                                // Defined only when g2 > 0.
                                ProCurvatureOp::Rotor => {
                                    if g2 <= f64::EPSILON {
                                        0.0
                                    } else {
                                        ((p * p - q * q).mul_add(s, -p * q * (r - t)))
                                            / (g2 * g2.sqrt() * g2)  // Optimization 2: g2^(3/2) = g2 * g2.sqrt()
                                    }
                                }

                                // Difference curvature (tangential – mean).
                                ProCurvatureOp::Difference => diff_curv,

                                // Horizontal excess: unsphericity – difference curvature.
                                ProCurvatureOp::HorizontalExcess => {
                                    if g2 <= f64::EPSILON { 0.0 } else { sqrt_disc - diff_curv }
                                }

                                // Vertical excess: unsphericity + difference curvature.
                                ProCurvatureOp::VerticalExcess => {
                                    if g2 <= f64::EPSILON { 0.0 } else { sqrt_disc + diff_curv }
                                }

                                // Accumulation curvature: mean² – diff².
                                ProCurvatureOp::Accumulation => {
                                    if g2 <= f64::EPSILON {
                                        0.0
                                    } else {
                                        mean_curv * mean_curv - diff_curv * diff_curv
                                    }
                                }

                                // Generating function (Shary & Stepanov, 1991; Florinsky, 2016).
                                ProCurvatureOp::GeneratingFunction => Self::generating_function_value(
                                    &input,
                                    band,
                                    row,
                                    col,
                                    z_factor,
                                    is_geographic,
                                    dx,
                                    dy,
                                )
                                .unwrap_or(0.0),

                                // Principal curvature direction angle in degrees [0, 180),
                                // derived from the Hessian eigenvector orientation.
                                ProCurvatureOp::PrincipalCurvatureDirection => {
                                    let theta_deg = 0.5 * (2.0 * s).atan2(r - t).to_degrees();
                                    theta_deg.rem_euclid(180.0)
                                }

                                // Casorati curvature (same scalar magnitude as curvedness).
                                // Optimization 2: Replace powi(2) with multiplication
                                ProCurvatureOp::Casorati => {
                                    ((minimal_curv * minimal_curv + maximal_curv * maximal_curv) / 2.0)
                                        .sqrt()
                                }
                            };

                            if log_transform && !matches!(op, ProCurvatureOp::PrincipalCurvatureDirection) {
                                // Florinsky (2016) p. 244 eq. 8.1
                                curv = curv.signum() * (1.0 + log_multiplier * curv.abs()).ln();
                            }

                            row_out[c] = curv;
                        }
                    }
                    (r_start..r_end).map(|r| tile_rows_vec[r - r_start].clone()).collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();

            // Flatten tile results back into single-row chunks and write
            let mut row_idx = 0;
            for chunk in row_data.chunks(rows) {
                for (r, row) in chunk.iter().take(rows - row_idx).enumerate() {
                    output
                        .set_row_slice(band, (row_idx + r) as isize, row)
                        .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", row_idx + r, e)))?;
                }
                row_idx += chunk.len().min(rows - row_idx);
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

// --- macro to implement Tool for each struct ----------------------------------

macro_rules! define_pro_curvature_tool {
    ($tool:ident, $op:expr) => {
        impl Tool for $tool {
            fn metadata(&self) -> ToolMetadata {
                ProCurvatureCore::metadata_for($op)
            }

            fn manifest(&self) -> ToolManifest {
                ProCurvatureCore::manifest_for($op)
            }

            fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
                let _ = ProCurvatureCore::parse_input(args)?;
                let _ = parse_optional_output_path(args, "output")?;
                let _ = ProCurvatureCore::parse_z_factor(args);
                let _ = ProCurvatureCore::parse_log_transform(args);
                Ok(())
            }

            fn run(
                &self,
                args: &ToolArgs,
                ctx: &ToolContext,
            ) -> Result<ToolRunResult, ToolError> {
                ProCurvatureCore::run_with_op($op, args, ctx)
            }
        }
    };
}

define_pro_curvature_tool!(MinimalCurvatureTool,          ProCurvatureOp::Minimal);
define_pro_curvature_tool!(MaximalCurvatureTool,          ProCurvatureOp::Maximal);
define_pro_curvature_tool!(ShapeIndexTool,                ProCurvatureOp::ShapeIndex);
define_pro_curvature_tool!(CurvednessTool,                ProCurvatureOp::Curvedness);
define_pro_curvature_tool!(UnsphericityCurvatureTool,     ProCurvatureOp::Unsphericity);
define_pro_curvature_tool!(RingCurvatureTool,             ProCurvatureOp::Ring);
define_pro_curvature_tool!(RotorTool,                     ProCurvatureOp::Rotor);
define_pro_curvature_tool!(DifferenceCurvatureTool,       ProCurvatureOp::Difference);
define_pro_curvature_tool!(HorizontalExcessCurvatureTool, ProCurvatureOp::HorizontalExcess);
define_pro_curvature_tool!(VerticalExcessCurvatureTool,   ProCurvatureOp::VerticalExcess);
define_pro_curvature_tool!(AccumulationCurvatureTool,     ProCurvatureOp::Accumulation);
define_pro_curvature_tool!(GeneratingFunctionTool,         ProCurvatureOp::GeneratingFunction);
define_pro_curvature_tool!(PrincipalCurvatureDirectionTool, ProCurvatureOp::PrincipalCurvatureDirection);
define_pro_curvature_tool!(CasoratiCurvatureTool,         ProCurvatureOp::Casorati);

// --- tests --------------------------------------------------------------------

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

    fn make_quadratic_raster(rows: usize, cols: usize) -> Raster {
        let cfg = RasterConfig {
            rows,
            cols,
            bands: 1,
            nodata: -9999.0,
            ..Default::default()
        };
        let mut r = Raster::new(cfg);
        let cx = (cols as f64 - 1.0) / 2.0;
        let cy = (rows as f64 - 1.0) / 2.0;
        for row in 0..rows as isize {
            for col in 0..cols as isize {
                let x = col as f64 - cx;
                let y = row as f64 - cy;
                let z = x * x + 0.5 * x * y + 0.2 * y * y;
                r.set(0, row, col, z).unwrap();
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
    fn pro_curvature_tools_constant_raster_returns_zero() {
        let tools: Vec<(&dyn Tool, &str)> = vec![
            (&MinimalCurvatureTool,          "minimal"),
            (&MaximalCurvatureTool,          "maximal"),
            (&ShapeIndexTool,                "shape_index"),
            (&CurvednessTool,                "curvedness"),
            (&UnsphericityCurvatureTool,     "unsphericity"),
            (&RingCurvatureTool,             "ring"),
            (&RotorTool,                     "rotor"),
            (&DifferenceCurvatureTool,       "difference"),
            (&HorizontalExcessCurvatureTool, "horizontal_excess"),
            (&VerticalExcessCurvatureTool,   "vertical_excess"),
            (&AccumulationCurvatureTool,     "accumulation"),
            (&GeneratingFunctionTool,         "generating_function"),
            (&PrincipalCurvatureDirectionTool, "principal_curvature_direction"),
            (&CasoratiCurvatureTool,          "casorati"),
        ];

        for (tool, name) in tools {
            let mut args = ToolArgs::new();
            args.insert("z_factor".to_string(), json!(1.0));
            args.insert("log_transform".to_string(), json!(false));
            let out = run_with_memory(tool, &mut args, make_constant_raster(20, 20, 10.0));
            assert!(
                out.get(0, 10, 10).abs() < 1e-10,
                "{} should return ~0 on constant raster, got {}",
                name,
                out.get(0, 10, 10)
            );
        }
    }

    #[test]
    fn generating_function_constant_raster_returns_zero() {
        let mut args = ToolArgs::new();
        args.insert("z_factor".to_string(), json!(1.0));
        args.insert("log_transform".to_string(), json!(false));
        let out = run_with_memory(&GeneratingFunctionTool, &mut args, make_constant_raster(25, 25, 42.0));
        assert!(
            out.get(0, 12, 12).abs() < 1e-10,
            "generating_function should return ~0 on constant raster, got {}",
            out.get(0, 12, 12)
        );
    }

    #[test]
    fn principal_curvature_direction_is_in_expected_range() {
        let mut args = ToolArgs::new();
        args.insert("z_factor".to_string(), json!(1.0));
        args.insert("log_transform".to_string(), json!(false));
        let out = run_with_memory(
            &PrincipalCurvatureDirectionTool,
            &mut args,
            make_quadratic_raster(41, 41),
        );
        let v = out.get(0, 20, 20);
        assert!(v.is_finite(), "principal_curvature_direction should be finite");
        assert!(
            (0.0..180.0).contains(&v),
            "principal_curvature_direction should be in [0, 180), got {}",
            v
        );
    }
}
