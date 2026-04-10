//! Terrain analysis tools: ruggedness index, surface area ratio,
//! elevation relative to min/max, and wetness index.

use rayon::prelude::*;
use serde_json::json;
use wbprojection::{Crs, EpsgIdentifyPolicy, identify_epsg_from_wkt_with_policy};
use wbcore::{
    parse_optional_output_path, parse_raster_path_arg, parse_vector_path_arg, LicenseTier, PercentCoalescer, Tool,
    ToolArgs, ToolCategory, ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata,
    ToolParamDescriptor, ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::{DataType, Raster, RasterConfig, RasterFormat};

use crate::memory_store;

pub struct RuggednessIndexTool;
pub struct SurfaceAreaRatioTool;
pub struct ElevRelativeToMinMaxTool;
pub struct WetnessIndexTool;
pub struct PercentElevRangeTool;
pub struct RelativeTopographicPositionTool;
pub struct NumDownslopeNeighboursTool;
pub struct NumUpslopeNeighboursTool;
pub struct MaxDownslopeElevChangeTool;
pub struct MaxUpslopeElevChangeTool;
pub struct MinDownslopeElevChangeTool;
pub struct ElevationPercentileTool;
pub struct DownslopeIndexTool;
pub struct ElevAbovePitTool;
pub struct DirectionalReliefTool;
pub struct ExposureTowardsWindFluxTool;
pub struct RelativeAspectTool;
pub struct EdgeDensityTool;
pub struct SphericalStdDevOfNormalsTool;
pub struct AverageNormalVectorAngularDeviationTool;
pub struct HypsometricAnalysisTool;
pub struct ProfileTool;
pub struct SlopeVsAspectPlotTool;
pub struct SlopeVsElevPlotTool;
pub struct ElevAbovePitDistTool;
pub struct CircularVarianceOfAspectTool;
pub struct FetchAnalysisTool;
pub struct FindRidgesTool;
pub struct MaxBranchLengthTool;
pub struct GeomorphonsTool;
pub struct PennockLandformClassificationTool;
pub struct ViewshedTool;
pub struct BreaklineMappingTool;
pub struct LowPointsOnHeadwaterDividesTool;
pub struct AssessRouteTool;

#[derive(Clone, Copy)]
struct AssessRouteMetrics {
    avg_slope: f64,
    min_elev: f64,
    max_elev: f64,
    relief: f64,
    sinuosity: f64,
    chg_in_slp: f64,
    visibility: Option<f64>,
}

// ---------------------------------------------------------------------------
// Shared core helpers
// ---------------------------------------------------------------------------

struct TerrainAnalysisCore;

impl TerrainAnalysisCore {
    fn parse_input(args: &ToolArgs) -> Result<String, ToolError> {
        parse_raster_path_arg(args, "input")
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
                    "parameter 'input' references unknown in-memory raster id '{}'",
                    id
                ))
            });
        }
        Raster::read(path)
            .map_err(|e| ToolError::Execution(format!("failed reading input raster: {}", e)))
    }

    fn load_named_raster(args: &ToolArgs, key: &str) -> Result<Raster, ToolError> {
        let path = parse_raster_path_arg(args, key)?;
        if memory_store::raster_is_memory_path(&path) {
            let id = memory_store::raster_path_to_id(&path).ok_or_else(|| {
                ToolError::Validation(format!(
                    "parameter '{}' has malformed in-memory raster path",
                    key
                ))
            })?;
            return memory_store::get_raster_by_id(id).ok_or_else(|| {
                ToolError::Validation(format!(
                    "parameter '{}' references unknown in-memory raster id '{}'",
                    key, id
                ))
            });
        }
        Raster::read(&path)
            .map_err(|e| ToolError::Execution(format!("failed reading '{}' raster: {}", key, e)))
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
                .map_err(|e| {
                    ToolError::Execution(format!("failed writing output raster: {e}"))
                })?;
            Ok(output_path_str)
        } else {
            let id = memory_store::put_raster(output);
            Ok(memory_store::make_raster_memory_path(&id))
        }
    }

    fn build_result(output_locator: String) -> ToolRunResult {
        let mut outputs = std::collections::BTreeMap::new();
        outputs.insert("path".to_string(), json!(output_locator));
        ToolRunResult {
            outputs,
            ..Default::default()
        }
    }

    fn solve_3x3(
        a11: f64,
        a12: f64,
        a13: f64,
        a21: f64,
        a22: f64,
        a23: f64,
        a31: f64,
        a32: f64,
        a33: f64,
        b1: f64,
        b2: f64,
        b3: f64,
    ) -> Option<(f64, f64, f64)> {
        fn det3(
            a11: f64,
            a12: f64,
            a13: f64,
            a21: f64,
            a22: f64,
            a23: f64,
            a31: f64,
            a32: f64,
            a33: f64,
        ) -> f64 {
            a11 * (a22 * a33 - a23 * a32)
                - a12 * (a21 * a33 - a23 * a31)
                + a13 * (a21 * a32 - a22 * a31)
        }

        let det = det3(a11, a12, a13, a21, a22, a23, a31, a32, a33);
        if det.abs() < 1e-12 {
            return None;
        }

        let det_x = det3(b1, a12, a13, b2, a22, a23, b3, a32, a33);
        let det_y = det3(a11, b1, a13, a21, b2, a23, a31, b3, a33);
        let det_z = det3(a11, a12, b1, a21, a22, b2, a31, a32, b3);
        Some((det_x / det, det_y / det, det_z / det))
    }

    fn detrend_raster_to_residuals(input: &Raster) -> Raster {
        let rows = input.rows;
        let cols = input.cols;
        let band = 0isize;

        let stats = (0..rows)
            .into_par_iter()
            .map(|row| {
                let mut sum_y = 0.0;
                let mut sum_xr_y = 0.0;
                let mut sum_xc_y = 0.0;
                let mut sum_xr = 0.0;
                let mut sum_xc = 0.0;
                let mut sum_xr_xr = 0.0;
                let mut sum_xc_xc = 0.0;
                let mut sum_xr_xc = 0.0;
                let mut n = 0.0;
                let r = row as f64;

                for col in 0..cols {
                    let c = col as f64;
                    let z = input.get(band, row as isize, col as isize);
                    if input.is_nodata(z) {
                        continue;
                    }
                    sum_y += z;
                    sum_xr_y += r * z;
                    sum_xc_y += c * z;
                    sum_xr += r;
                    sum_xc += c;
                    sum_xr_xr += r * r;
                    sum_xc_xc += c * c;
                    sum_xr_xc += r * c;
                    n += 1.0;
                }

                (
                    sum_y,
                    sum_xr_y,
                    sum_xc_y,
                    sum_xr,
                    sum_xc,
                    sum_xr_xr,
                    sum_xc_xc,
                    sum_xr_xc,
                    n,
                )
            })
            .reduce(
                || (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
                |a, b| {
                    (
                        a.0 + b.0,
                        a.1 + b.1,
                        a.2 + b.2,
                        a.3 + b.3,
                        a.4 + b.4,
                        a.5 + b.5,
                        a.6 + b.6,
                        a.7 + b.7,
                        a.8 + b.8,
                    )
                },
            );

        let Some((b0, b1r, b1c)) = Self::solve_3x3(
            stats.8, stats.3, stats.4,
            stats.3, stats.5, stats.7,
            stats.4, stats.7, stats.6,
            stats.0, stats.1, stats.2,
        ) else {
            return input.clone();
        };

        let cfg = RasterConfig {
            cols: input.cols,
            rows: input.rows,
            bands: 1,
            x_min: input.x_min,
            y_min: input.y_min,
            cell_size: input.cell_size_x,
            cell_size_y: Some(input.cell_size_y),
            nodata: input.nodata,
            data_type: DataType::F64,
            crs: input.crs.clone(),
            metadata: input.metadata.clone(),
        };
        let mut residuals = Raster::new(cfg);

        let row_data: Vec<Vec<f64>> = (0..rows)
            .into_par_iter()
            .map(|row| {
                let mut row_out = vec![input.nodata; cols];
                let r = row as f64;
                for (col, value) in row_out.iter_mut().enumerate() {
                    let c = col as f64;
                    let z = input.get(band, row as isize, col as isize);
                    if input.is_nodata(z) {
                        continue;
                    }
                    *value = z - (b0 + b1r * r + b1c * c);
                }
                row_out
            })
            .collect();

        for (row, row_values) in row_data.iter().enumerate() {
            residuals
                .set_row_slice(0, row as isize, row_values)
                .expect("residual row write should succeed");
        }

        residuals
    }

    fn build_geomorphon_lookup() -> [u16; 6561] {
        let mut gtc = [u16::MAX; 6561];

        for val in 0..6561usize {
            let mut pattern = [0usize; 8];
            let mut rev_pattern = [0usize; 8];
            let mut value = val;
            for i in 0..8 {
                let digit = value % 3;
                pattern[i] = digit;
                rev_pattern[7 - i] = digit;
                value /= 3;
            }

            let mut best = usize::MAX;
            for shift in 0..8usize {
                let mut code = 0usize;
                let mut rev_code = 0usize;
                let mut power = 1usize;
                for i in 0..8usize {
                    let idx = (i + 8usize - shift) % 8usize;
                    code += pattern[idx] * power;
                    rev_code += rev_pattern[idx] * power;
                    power *= 3;
                }
                best = best.min(code).min(rev_code);
            }
            gtc[val] = best as u16;
        }

        gtc
    }

    fn parse_filter_sizes(args: &ToolArgs) -> (usize, usize) {
        let mut filter_size_x = args
            .get("filter_size_x")
            .or_else(|| args.get("filterx"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(11);
        let mut filter_size_y = args
            .get("filter_size_y")
            .or_else(|| args.get("filtery"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(filter_size_x);
        filter_size_x = filter_size_x.max(3);
        filter_size_y = filter_size_y.max(3);
        if filter_size_x % 2 == 0 {
            filter_size_x += 1;
        }
        if filter_size_y % 2 == 0 {
            filter_size_y += 1;
        }
        (filter_size_x, filter_size_y)
    }

    fn idx(row: usize, col: usize, cols: usize) -> usize {
        row * cols + col
    }

    fn rect_sum(sum: &[f64], cols: usize, y1: usize, x1: usize, y2: usize, x2: usize) -> f64 {
        let a = sum[Self::idx(y2, x2, cols)];
        let b = if y1 > 0 { sum[Self::idx(y1 - 1, x2, cols)] } else { 0.0 };
        let c = if x1 > 0 { sum[Self::idx(y2, x1 - 1, cols)] } else { 0.0 };
        let d = if y1 > 0 && x1 > 0 { sum[Self::idx(y1 - 1, x1 - 1, cols)] } else { 0.0 };
        a - b - c + d
    }

    fn rect_count(count: &[i64], cols: usize, y1: usize, x1: usize, y2: usize, x2: usize) -> i64 {
        let a = count[Self::idx(y2, x2, cols)];
        let b = if y1 > 0 { count[Self::idx(y1 - 1, x2, cols)] } else { 0 };
        let c = if x1 > 0 { count[Self::idx(y2, x1 - 1, cols)] } else { 0 };
        let d = if y1 > 0 && x1 > 0 { count[Self::idx(y1 - 1, x1 - 1, cols)] } else { 0 };
        a - b - c + d
    }

    fn build_integrals(input: &Raster, band: isize) -> (Vec<f64>, Vec<f64>, Vec<i64>) {
        let rows = input.rows;
        let cols = input.cols;
        let mut sum = vec![0.0; rows * cols];
        let mut sum_sq = vec![0.0; rows * cols];
        let mut count = vec![0i64; rows * cols];

        for row in 0..rows {
            let mut row_sum = 0.0;
            let mut row_sum_sq = 0.0;
            let mut row_count = 0i64;
            for col in 0..cols {
                let val = input.get(band, row as isize, col as isize);
                if !input.is_nodata(val) {
                    row_sum += val;
                    row_sum_sq += val * val;
                    row_count += 1;
                }
                let idx = Self::idx(row, col, cols);
                if row > 0 {
                    let above = Self::idx(row - 1, col, cols);
                    sum[idx] = row_sum + sum[above];
                    sum_sq[idx] = row_sum_sq + sum_sq[above];
                    count[idx] = row_count + count[above];
                } else {
                    sum[idx] = row_sum;
                    sum_sq[idx] = row_sum_sq;
                    count[idx] = row_count;
                }
            }
        }
        (sum, sum_sq, count)
    }

    fn build_integral_from_values(values: &[f64], rows: usize, cols: usize) -> (Vec<f64>, Vec<i64>) {
        let mut sum = vec![0.0; rows * cols];
        let mut count = vec![0i64; rows * cols];
        for row in 0..rows {
            let mut row_sum = 0.0;
            let mut row_count = 0i64;
            for col in 0..cols {
                let idx = row * cols + col;
                let v = values[idx];
                if v.is_finite() {
                    row_sum += v;
                    row_count += 1;
                }
                if row > 0 {
                    let above = (row - 1) * cols + col;
                    sum[idx] = row_sum + sum[above];
                    count[idx] = row_count + count[above];
                } else {
                    sum[idx] = row_sum;
                    count[idx] = row_count;
                }
            }
        }
        (sum, count)
    }

    fn gaussian_blur_values(values: &[f64], rows: usize, cols: usize, sigma: f64) -> Vec<f64> {
        if sigma <= 0.0 {
            return values.to_vec();
        }
        let radius = (sigma * 3.0).ceil() as isize;
        if radius <= 0 {
            return values.to_vec();
        }
        let mut kernel = Vec::with_capacity((radius * 2 + 1) as usize);
        let mut ksum = 0.0;
        for i in -radius..=radius {
            let w = (-(i * i) as f64 / (2.0 * sigma * sigma)).exp();
            kernel.push(w);
            ksum += w;
        }
        if ksum > 0.0 {
            for w in &mut kernel {
                *w /= ksum;
            }
        }

        let mut temp = vec![f64::NAN; rows * cols];
        for r in 0..rows {
            for c in 0..cols {
                let idx = r * cols + c;
                if !values[idx].is_finite() {
                    continue;
                }
                let mut s = 0.0;
                let mut wsum = 0.0;
                for k in -radius..=radius {
                    let cc = c as isize + k;
                    if cc < 0 || cc >= cols as isize {
                        continue;
                    }
                    let v = values[r * cols + cc as usize];
                    if !v.is_finite() {
                        continue;
                    }
                    let w = kernel[(k + radius) as usize];
                    s += w * v;
                    wsum += w;
                }
                if wsum > 0.0 {
                    temp[idx] = s / wsum;
                }
            }
        }

        let mut out = vec![f64::NAN; rows * cols];
        for r in 0..rows {
            for c in 0..cols {
                let idx = r * cols + c;
                if !temp[idx].is_finite() {
                    continue;
                }
                let mut s = 0.0;
                let mut wsum = 0.0;
                for k in -radius..=radius {
                    let rr = r as isize + k;
                    if rr < 0 || rr >= rows as isize {
                        continue;
                    }
                    let v = temp[rr as usize * cols + c];
                    if !v.is_finite() {
                        continue;
                    }
                    let w = kernel[(k + radius) as usize];
                    s += w * v;
                    wsum += w;
                }
                if wsum > 0.0 {
                    out[idx] = s / wsum;
                }
            }
        }
        out
    }

    fn compute_normals_from_values(
        values: &[f64],
        rows: usize,
        cols: usize,
        cell_size_x: f64,
        cell_size_y: f64,
        z_factor: f64,
    ) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut nx = vec![f64::NAN; rows * cols];
        let mut ny = vec![f64::NAN; rows * cols];
        let mut nz = vec![f64::NAN; rows * cols];
        let resx = cell_size_x.abs().max(f64::EPSILON);
        let resy = cell_size_y.abs().max(f64::EPSILON);

        for r in 0..rows {
            for c in 0..cols {
                let idx = r * cols + c;
                let zc = values[idx];
                if !zc.is_finite() {
                    continue;
                }
                let z = |dr: isize, dc: isize| {
                    let rr = r as isize + dr;
                    let cc = c as isize + dc;
                    if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                        return zc;
                    }
                    let v = values[rr as usize * cols + cc as usize];
                    if v.is_finite() { v } else { zc }
                };

                let z1 = z(-1, -1) * z_factor;
                let z2 = z(-1, 0) * z_factor;
                let z3 = z(-1, 1) * z_factor;
                let z4 = z(0, -1) * z_factor;
                let z6 = z(0, 1) * z_factor;
                let z7 = z(1, -1) * z_factor;
                let z8 = z(1, 0) * z_factor;
                let z9 = z(1, 1) * z_factor;

                let dzdx = ((z3 + 2.0 * z6 + z9) - (z1 + 2.0 * z4 + z7)) / (8.0 * resx);
                let dzdy = ((z7 + 2.0 * z8 + z9) - (z1 + 2.0 * z2 + z3)) / (8.0 * resy);
                let mut x = -dzdx;
                let mut y = -dzdy;
                let mut zc_n = 1.0;
                let m = (x * x + y * y + zc_n * zc_n).sqrt();
                if m > f64::EPSILON {
                    x /= m;
                    y /= m;
                    zc_n /= m;
                    nx[idx] = x;
                    ny[idx] = y;
                    nz[idx] = zc_n;
                }
            }
        }
        (nx, ny, nz)
    }

    fn parse_raster_input_list(args: &ToolArgs, key: &str) -> Result<Vec<String>, ToolError> {
        let v = args
            .get(key)
            .ok_or_else(|| ToolError::Validation(format!("missing required parameter '{}'", key)))?;
        if let Some(s) = v.as_str() {
            let items = s
                .split([';', ','])
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(|p| p.to_string())
                .collect::<Vec<_>>();
            if items.is_empty() {
                return Err(ToolError::Validation(format!("parameter '{}' contains no input paths", key)));
            }
            return Ok(items);
        }
        if let Some(arr) = v.as_array() {
            let items = arr
                .iter()
                .filter_map(|x| x.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>();
            if items.is_empty() {
                return Err(ToolError::Validation(format!("parameter '{}' contains no input paths", key)));
            }
            return Ok(items);
        }
        Err(ToolError::Validation(format!("parameter '{}' must be a string list or array", key)))
    }

    fn parse_vector_points(layer: &wbvector::Layer) -> Result<Vec<(f64, f64)>, ToolError> {
        let mut points = Vec::new();
        for feature in &layer.features {
            if let Some(geom) = &feature.geometry {
                match geom {
                    wbvector::Geometry::Point(c) => points.push((c.x, c.y)),
                    wbvector::Geometry::MultiPoint(cs) => {
                        points.extend(cs.iter().map(|c| (c.x, c.y)));
                    }
                    wbvector::Geometry::GeometryCollection(gs) => {
                        for g in gs {
                            match g {
                                wbvector::Geometry::Point(c) => points.push((c.x, c.y)),
                                wbvector::Geometry::MultiPoint(cs) => {
                                    points.extend(cs.iter().map(|c| (c.x, c.y)));
                                }
                                _ => {
                                    return Err(ToolError::Validation(
                                        "stations input must contain only point geometries"
                                            .to_string(),
                                    ))
                                }
                            }
                        }
                    }
                    _ => {
                        return Err(ToolError::Validation(
                            "stations input must contain only point geometries".to_string(),
                        ))
                    }
                }
            }
        }
        if points.is_empty() {
            return Err(ToolError::Validation(
                "stations input must contain at least one point feature".to_string(),
            ));
        }
        Ok(points)
    }

    fn write_vector_output(
        layer: &wbvector::Layer,
        output_path: Option<std::path::PathBuf>,
        default_name: &str,
    ) -> Result<String, ToolError> {
        let out = output_path.unwrap_or_else(|| std::env::temp_dir().join(default_name));
        if let Some(parent) = out.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ToolError::Execution(format!("failed creating output directory: {e}"))
                })?;
            }
        }
        let out_str = out.to_string_lossy().to_string();
        let fmt = match wbvector::VectorFormat::detect(&out_str) {
            Ok(f) => f,
            Err(_) => {
                if std::path::Path::new(&out_str).extension().is_none() {
                    wbvector::VectorFormat::Shapefile
                } else {
                    return Err(ToolError::Validation(format!(
                        "unsupported vector output path '{}'",
                        out_str
                    )));
                }
            }
        };
        wbvector::write(layer, &out_str, fmt).map_err(|e| {
            ToolError::Execution(format!("failed writing output vector: {}", e))
        })?;
        Ok(out_str)
    }

    fn write_hypsometric_html(
        output_path: &std::path::Path,
        title: &str,
        series: &[(String, Vec<(f64, f64)>, f64)],
    ) -> Result<(), ToolError> {
        let width = 980.0;
        let height = 560.0;
        let pad_l = 80.0;
        let pad_r = 210.0;
        let pad_t = 40.0;
        let pad_b = 70.0;

        let min_y = series
            .iter()
            .flat_map(|(_, pts, _)| pts.iter().map(|(_, y)| *y))
            .fold(f64::INFINITY, f64::min)
            .floor();
        let max_y = series
            .iter()
            .flat_map(|(_, pts, _)| pts.iter().map(|(_, y)| *y))
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil();
        let min_y = if min_y.is_finite() { min_y } else { 0.0 };
        let max_y = if max_y.is_finite() { max_y } else { 1.0 };
        let y_rng = (max_y - min_y).max(1.0);

        let sx = |x: f64| pad_l + x / 100.0 * (width - pad_l - pad_r);
        let sy = |y: f64| height - pad_b - (y - min_y) / y_rng * (height - pad_t - pad_b);

        let palette = [
            "#1f77b4", "#ff7f0e", "#2ca02c", "#d62728", "#9467bd", "#8c564b", "#17becf",
            "#bcbd22",
        ];

        let mut grid = String::new();
        for i in 0..=10 {
            let x = i as f64 * 10.0;
            let px = sx(x);
            grid.push_str(&format!("<line x1='{:.2}' y1='{:.2}' x2='{:.2}' y2='{:.2}' stroke='#d0d0d0'/>", px, pad_t, px, height - pad_b));
        }
        for i in 0..=8 {
            let yv = min_y + i as f64 / 8.0 * y_rng;
            let py = sy(yv);
            grid.push_str(&format!("<line x1='{:.2}' y1='{:.2}' x2='{:.2}' y2='{:.2}' stroke='#d0d0d0'/>", pad_l, py, width - pad_r, py));
        }

        let mut lines = String::new();
        let mut legend = String::new();
        for (i, (name, pts, hi)) in series.iter().enumerate() {
            if pts.is_empty() {
                continue;
            }
            let color = palette[i % palette.len()];
            let poly = pts
                .iter()
                .map(|(x, y)| format!("{:.2},{:.2}", sx(*x), sy(*y)))
                .collect::<Vec<_>>()
                .join(" ");
            lines.push_str(&format!("<polyline fill='none' stroke='{}' stroke-width='2.6' points='{}'/>", color, poly));
            legend.push_str(&format!("<div style='margin:6px 0'><span style='display:inline-block;width:22px;height:3px;background:{};vertical-align:middle;margin-right:8px'></span>{} (HI={:.3})</div>", color, name, hi));
        }

        let html = format!(
            "<!doctype html><html><head><meta charset='utf-8'><title>{}</title><style>body{{margin:0;background:#ececec;font-family:Helvetica,Arial,sans-serif;color:#111}}main{{max-width:1200px;margin:0 auto;padding:24px}}h1{{margin:0 0 12px 0;font-size:30px;font-weight:600}}.card{{background:#f3f3f3;border:1px solid #cfcfcf;border-radius:10px;padding:12px}}.axis{{font-size:14px;fill:#222}}</style></head><body><main><h1>{}</h1><div class='card'><svg viewBox='0 0 {} {}' width='100%' height='{}'>{}<rect x='{:.2}' y='{:.2}' width='{:.2}' height='{:.2}' fill='none' stroke='#5a5a5a' stroke-width='1.2'/>{}<text x='{:.2}' y='{:.2}' class='axis'>% Area Above</text><text transform='translate({:.2},{:.2}) rotate(-90)' class='axis'>Elevation</text></svg><div style='margin-top:10px'>{}</div></div></main></body></html>",
            title,
            title,
            width,
            height,
            height,
            grid,
            pad_l,
            pad_t,
            width - pad_l - pad_r,
            height - pad_t - pad_b,
            lines,
            (pad_l + (width - pad_l - pad_r) / 2.0) - 40.0,
            height - 20.0,
            24.0,
            (height - pad_b + pad_t) / 2.0,
            legend
        );

        std::fs::write(output_path, html)
            .map_err(|e| ToolError::Execution(format!("failed writing hypsometric HTML: {e}")))
    }

    fn percentile_sorted(values: &[f64], pct: f64) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        let p = pct.clamp(0.0, 1.0);
        let idx = ((values.len() as f64) * p).floor() as usize;
        values[idx.min(values.len() - 1)]
    }

    fn elongation_ratio_from_polar(angles_deg: &[f64], radii: &[f64]) -> f64 {
        if angles_deg.len() != radii.len() || angles_deg.len() < 3 {
            return 0.0;
        }
        let mut xs = Vec::with_capacity(radii.len());
        let mut ys = Vec::with_capacity(radii.len());
        for (a, r) in angles_deg.iter().zip(radii.iter()) {
            let theta = a.to_radians();
            xs.push(r * theta.cos());
            ys.push(r * theta.sin());
        }

        let n = xs.len() as f64;
        let mx = xs.iter().sum::<f64>() / n;
        let my = ys.iter().sum::<f64>() / n;
        let mut cxx = 0.0;
        let mut cxy = 0.0;
        let mut cyy = 0.0;
        for i in 0..xs.len() {
            let dx = xs[i] - mx;
            let dy = ys[i] - my;
            cxx += dx * dx;
            cxy += dx * dy;
            cyy += dy * dy;
        }
        cxx /= n;
        cxy /= n;
        cyy /= n;

        let tr = cxx + cyy;
        let det = (cxx * cyy - cxy * cxy).max(0.0);
        let disc = (tr * tr - 4.0 * det).max(0.0).sqrt();
        let l1 = ((tr + disc) * 0.5).max(0.0);
        let l2 = ((tr - disc) * 0.5).max(0.0);
        if l1 <= f64::EPSILON {
            return 0.0;
        }
        (1.0 - (l2.sqrt() / l1.sqrt())).clamp(0.0, 1.0)
    }

    fn write_slope_vs_aspect_html(
        output_path: &std::path::Path,
        dem_name: &str,
        angles_deg: &[f64],
        p25: &[f64],
        p50: &[f64],
        p75: &[f64],
        elong25: f64,
        elong50: f64,
        elong75: f64,
    ) -> Result<(), ToolError> {
        let width = 920.0;
        let height = 650.0;
        let cx = 300.0;
        let cy = 330.0;
        let radius = 225.0;
        let right_x = 610.0;
        let max_slope = p75
            .iter()
            .chain(p50.iter())
            .chain(p25.iter())
            .copied()
            .filter(|v| v.is_finite())
            .fold(0.0_f64, f64::max)
            .max(1.0);

        let to_xy = |angle_deg: f64, val: f64| {
            let theta = angle_deg.to_radians();
            let rr = (val / max_slope).clamp(0.0, 1.2) * radius;
            let x = cx + rr * theta.sin();
            let y = cy - rr * theta.cos();
            (x, y)
        };

        let mut grid = String::new();
        for i in 1..=4 {
            let rr = radius * i as f64 / 4.0;
            let label = max_slope * i as f64 / 4.0;
            grid.push_str(&format!(
                "<circle cx='{:.2}' cy='{:.2}' r='{:.2}' fill='none' stroke='#d7d7d7'/>",
                cx, cy, rr
            ));
            grid.push_str(&format!(
                "<text x='{:.2}' y='{:.2}' font-size='12' fill='#666'>{:.1}</text>",
                cx + rr + 6.0,
                cy + 4.0,
                label
            ));
        }
        for a in (0..360).step_by(45) {
            let (x, y) = to_xy(a as f64, max_slope);
            grid.push_str(&format!(
                "<line x1='{:.2}' y1='{:.2}' x2='{:.2}' y2='{:.2}' stroke='#d7d7d7'/>",
                cx, cy, x, y
            ));
            let (lx, ly) = to_xy(a as f64, max_slope * 1.08);
            grid.push_str(&format!(
                "<text x='{:.2}' y='{:.2}' font-size='12' text-anchor='middle' fill='#555'>{}</text>",
                lx,
                ly + 4.0,
                a
            ));
        }

        let polyline = |vals: &[f64], color: &str| {
            let mut pts = String::new();
            for (a, v) in angles_deg.iter().zip(vals.iter()) {
                let (x, y) = to_xy(*a, *v);
                pts.push_str(&format!("{:.2},{:.2} ", x, y));
            }
            if let (Some(a0), Some(v0)) = (angles_deg.first(), vals.first()) {
                let (x, y) = to_xy(*a0, *v0);
                pts.push_str(&format!("{:.2},{:.2}", x, y));
            }
            format!("<polyline fill='none' stroke='{}' stroke-width='2.5' points='{}'/>", color, pts)
        };

        let series_svg = format!(
            "{}{}{}",
            polyline(p75, "#c23b22"),
            polyline(p50, "#1f77b4"),
            polyline(p25, "#2a9d3c")
        );

        let html = format!(
            "<!doctype html><html><head><meta charset='utf-8'><title>Slope vs Aspect</title><style>body{{margin:0;background:#ececec;font-family:Helvetica,Arial,sans-serif;color:#111}}main{{max-width:1200px;margin:0 auto;padding:24px}}h1{{margin:0 0 12px 0;font-size:30px;font-weight:600}}.card{{background:#f3f3f3;border:1px solid #cfcfcf;border-radius:10px;padding:12px}}table{{border-collapse:collapse;margin-top:12px}}th,td{{border:1px solid #c6c6c6;padding:6px 10px}}th{{background:#efefef;text-align:left}}td.num{{text-align:right}}</style></head><body><main><h1>Slope vs Aspect</h1><div class='card'><p><strong>Input DEM:</strong> {}</p><svg viewBox='0 0 {} {}' width='100%' height='{}'>{}<g>{}</g><g><rect x='{:.2}' y='{:.2}' width='18' height='3' fill='#c23b22'/><text x='{:.2}' y='{:.2}' font-size='13'>75th percentile</text><rect x='{:.2}' y='{:.2}' width='18' height='3' fill='#1f77b4'/><text x='{:.2}' y='{:.2}' font-size='13'>Median</text><rect x='{:.2}' y='{:.2}' width='18' height='3' fill='#2a9d3c'/><text x='{:.2}' y='{:.2}' font-size='13'>25th percentile</text></g></svg><table><tr><th>Percentile</th><th>Elongation ratio</th></tr><tr><td>25th</td><td class='num'>{:.4}</td></tr><tr><td>50th</td><td class='num'>{:.4}</td></tr><tr><td>75th</td><td class='num'>{:.4}</td></tr></table></div></main></body></html>",
            dem_name,
            width,
            height,
            height,
            grid,
            series_svg,
            right_x,
            180.0,
            right_x + 26.0,
            184.0,
            right_x,
            208.0,
            right_x + 26.0,
            212.0,
            right_x,
            236.0,
            right_x + 26.0,
            240.0,
            elong25,
            elong50,
            elong75
        );

        std::fs::write(output_path, html)
            .map_err(|e| ToolError::Execution(format!("failed writing slope-vs-aspect HTML: {e}")))
    }

    fn write_slope_vs_elev_html(
        output_path: &std::path::Path,
        series: &[(String, Vec<(f64, f64)>)],
    ) -> Result<(), ToolError> {
        let width = 980.0;
        let height = 560.0;
        let pad_l = 80.0;
        let pad_r = 220.0;
        let pad_t = 40.0;
        let pad_b = 70.0;

        let min_x = series
            .iter()
            .flat_map(|(_, pts)| pts.iter().map(|(x, _)| *x))
            .fold(f64::INFINITY, f64::min);
        let max_x = series
            .iter()
            .flat_map(|(_, pts)| pts.iter().map(|(x, _)| *x))
            .fold(f64::NEG_INFINITY, f64::max);
        let min_y = series
            .iter()
            .flat_map(|(_, pts)| pts.iter().map(|(_, y)| *y))
            .fold(f64::INFINITY, f64::min)
            .floor();
        let max_y = series
            .iter()
            .flat_map(|(_, pts)| pts.iter().map(|(_, y)| *y))
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil();
        let min_x = if min_x.is_finite() { min_x } else { 0.0 };
        let max_x = if max_x.is_finite() { max_x } else { 1.0 };
        let min_y = if min_y.is_finite() { min_y } else { 0.0 };
        let max_y = if max_y.is_finite() { max_y } else { 1.0 };
        let x_rng = (max_x - min_x).max(1.0e-6);
        let y_rng = (max_y - min_y).max(1.0);

        let sx = |x: f64| pad_l + (x - min_x) / x_rng * (width - pad_l - pad_r);
        let sy = |y: f64| height - pad_b - (y - min_y) / y_rng * (height - pad_t - pad_b);

        let palette = [
            "#1f77b4", "#ff7f0e", "#2ca02c", "#d62728", "#9467bd", "#8c564b", "#17becf",
            "#bcbd22",
        ];

        let mut grid = String::new();
        for i in 0..=10 {
            let x = min_x + i as f64 / 10.0 * x_rng;
            let px = sx(x);
            grid.push_str(&format!("<line x1='{:.2}' y1='{:.2}' x2='{:.2}' y2='{:.2}' stroke='#d0d0d0'/>", px, pad_t, px, height - pad_b));
        }
        for i in 0..=8 {
            let y = min_y + i as f64 / 8.0 * y_rng;
            let py = sy(y);
            grid.push_str(&format!("<line x1='{:.2}' y1='{:.2}' x2='{:.2}' y2='{:.2}' stroke='#d0d0d0'/>", pad_l, py, width - pad_r, py));
        }

        let mut lines = String::new();
        let mut legend = String::new();
        for (i, (name, pts)) in series.iter().enumerate() {
            if pts.is_empty() {
                continue;
            }
            let color = palette[i % palette.len()];
            let poly = pts
                .iter()
                .map(|(x, y)| format!("{:.2},{:.2}", sx(*x), sy(*y)))
                .collect::<Vec<_>>()
                .join(" ");
            lines.push_str(&format!("<polyline fill='none' stroke='{}' stroke-width='2.4' points='{}'/>", color, poly));
            legend.push_str(&format!("<div style='margin:6px 0'><span style='display:inline-block;width:22px;height:3px;background:{};vertical-align:middle;margin-right:8px'></span>{}</div>", color, name));
        }

        let html = format!(
            "<!doctype html><html><head><meta charset='utf-8'><title>Slope-Elevation Analysis</title><style>body{{margin:0;background:#ececec;font-family:Helvetica,Arial,sans-serif;color:#111}}main{{max-width:1200px;margin:0 auto;padding:24px}}h1{{margin:0 0 12px 0;font-size:30px;font-weight:600}}.card{{background:#f3f3f3;border:1px solid #cfcfcf;border-radius:10px;padding:12px}}.axis{{font-size:14px;fill:#222}}</style></head><body><main><h1>Slope-Elevation Analysis</h1><div class='card'><svg viewBox='0 0 {} {}' width='100%' height='{}'>{}<rect x='{:.2}' y='{:.2}' width='{:.2}' height='{:.2}' fill='none' stroke='#5a5a5a' stroke-width='1.2'/>{}<text x='{:.2}' y='{:.2}' class='axis'>Average Slope (degrees)</text><text transform='translate({:.2},{:.2}) rotate(-90)' class='axis'>Elevation</text></svg><div style='margin-top:10px'>{}</div></div></main></body></html>",
            width,
            height,
            height,
            grid,
            pad_l,
            pad_t,
            width - pad_l - pad_r,
            height - pad_t - pad_b,
            lines,
            (pad_l + (width - pad_l - pad_r) / 2.0) - 70.0,
            height - 20.0,
            24.0,
            (height - pad_b + pad_t) / 2.0,
            legend
        );

        std::fs::write(output_path, html)
            .map_err(|e| ToolError::Execution(format!("failed writing slope-vs-elev HTML: {e}")))
    }

    fn hypsometric_analysis_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "hypsometric_analysis",
            display_name: "Hypsometric Analysis",
            summary: "Creates a hypsometric (area-elevation) curve HTML report for one or more DEMs.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Input DEM paths as ';' or ',' separated string, or list.", required: true },
                ToolParamSpec { name: "watershed", description: "Optional watershed rasters matching each input DEM.", required: false },
                ToolParamSpec { name: "output", description: "Output HTML report path.", required: false },
            ],
        }
    }

    fn hypsometric_analysis_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!("dem.tif"));
        defaults.insert("output".to_string(), json!("hypsometric_analysis.html"));
        ToolManifest {
            id: "hypsometric_analysis".to_string(),
            display_name: "Hypsometric Analysis".to_string(),
            summary: "Creates a hypsometric (area-elevation) curve HTML report for one or more DEMs.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "signature".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_hypsometric_analysis(args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = Self::parse_raster_input_list(args, "inputs")?;
        let watershed_paths = if args.get("watershed").is_some() {
            Some(Self::parse_raster_input_list(args, "watershed")?)
        } else {
            None
        };
        if let Some(ws) = &watershed_paths {
            if ws.len() != input_paths.len() {
                return Err(ToolError::Validation(
                    "watershed list length must match inputs list length".to_string(),
                ));
            }
        }

        let output_path = parse_optional_output_path(args, "output")?
            .unwrap_or_else(|| std::env::temp_dir().join("hypsometric_analysis.html"));
        if let Some(parent) = output_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ToolError::Execution(format!("failed creating output directory: {e}"))
                })?;
            }
        }

        let mut series: Vec<(String, Vec<(f64, f64)>, f64)> = Vec::new();

        for (i, in_path) in input_paths.iter().enumerate() {
            let dem = Self::load_raster(in_path)?;
            if dem.bands < 1 {
                continue;
            }

            let ws = if let Some(ws_list) = &watershed_paths {
                Some(Self::load_raster(&ws_list[i])?)
            } else {
                None
            };

            let mut groups: std::collections::BTreeMap<i64, Vec<f64>> = std::collections::BTreeMap::new();
            for r in 0..dem.rows {
                for c in 0..dem.cols {
                    let z = dem.get(0, r as isize, c as isize);
                    if dem.is_nodata(z) {
                        continue;
                    }
                    let key = if let Some(w) = &ws {
                        let wv = w.get(0, r as isize, c as isize);
                        if w.is_nodata(wv) || wv == 0.0 {
                            continue;
                        }
                        wv.round() as i64
                    } else {
                        1
                    };
                    groups.entry(key).or_default().push(z);
                }
            }

            for (gid, mut vals) in groups {
                if vals.len() < 2 {
                    continue;
                }
                vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let n = vals.len();
                let min = vals[0];
                let max = vals[n - 1];
                let range = (max - min).max(f64::EPSILON);
                let mean = vals.iter().sum::<f64>() / n as f64;
                let hi = (mean - min) / range;

                let mut pts = Vec::with_capacity(101);
                for p in 0..=100 {
                    let frac_above = p as f64 / 100.0;
                    let idx = ((1.0 - frac_above) * (n as f64 - 1.0)).round() as usize;
                    let idx = idx.min(n - 1);
                    pts.push((p as f64, vals[idx]));
                }
                let name = if ws.is_some() {
                    format!("{}_ws{}", std::path::Path::new(in_path).file_stem().and_then(|s| s.to_str()).unwrap_or("dem"), gid)
                } else {
                    std::path::Path::new(in_path)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("dem")
                        .to_string()
                };
                series.push((name, pts, hi));
            }
        }

        if series.is_empty() {
            return Err(ToolError::Validation(
                "hypsometric_analysis found no valid elevation samples".to_string(),
            ));
        }

        Self::write_hypsometric_html(&output_path, "Hypsometric Analysis", &series)?;

        let mut outputs = std::collections::BTreeMap::new();
        outputs.insert("path".to_string(), json!(output_path.to_string_lossy().to_string()));
        Ok(ToolRunResult {
            outputs,
            ..Default::default()
        })
    }

    fn slope_vs_aspect_plot_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "slope_vs_aspect_plot",
            display_name: "Slope Vs Aspect Plot",
            summary: "Creates an HTML radial slope-vs-aspect analysis plot for an input DEM.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Pro,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "aspect_bin_size", description: "Aspect bin size in degrees (default 2.0).", required: false },
                ToolParamSpec { name: "min_slope", description: "Ignore slopes below this threshold in degrees (default 0.1).", required: false },
                ToolParamSpec { name: "z_factor", description: "Vertical scaling factor for elevation values.", required: false },
                ToolParamSpec { name: "output", description: "Output HTML report path.", required: false },
            ],
        }
    }

    fn profile_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "profile",
            display_name: "Profile",
            summary: "Creates an HTML elevation profile plot for one or more input polyline features sampled from a surface raster.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "lines_vector",
                    description: "Input polyline vector path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "surface",
                    description: "Input surface raster path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output HTML report path.",
                    required: false,
                },
            ],
        }
    }

    fn profile_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("lines_vector".to_string(), json!("lines.shp"));
        defaults.insert("surface".to_string(), json!("dem.tif"));
        defaults.insert("output".to_string(), json!("profile.html"));

        ToolManifest {
            id: "profile".to_string(),
            display_name: "Profile".to_string(),
            summary:
                "Creates an HTML elevation profile plot for one or more input polyline features sampled from a surface raster."
                    .to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec![
                "geomorphometry".to_string(),
                "profile".to_string(),
                "plot".to_string(),
                "html".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn collect_profile_lines(geom: &wbvector::Geometry, lines: &mut Vec<Vec<wbvector::Coord>>) {
        match geom {
            wbvector::Geometry::LineString(coords) => {
                if coords.len() >= 2 {
                    lines.push(coords.clone());
                }
            }
            wbvector::Geometry::MultiLineString(parts) => {
                for part in parts {
                    if part.len() >= 2 {
                        lines.push(part.clone());
                    }
                }
            }
            wbvector::Geometry::GeometryCollection(geoms) => {
                for g in geoms {
                    Self::collect_profile_lines(g, lines);
                }
            }
            _ => {}
        }
    }

    fn write_profile_html(
        output_path: &std::path::Path,
        title: &str,
        surface_name: &str,
        series: &[(String, Vec<(f64, f64)>)],
    ) -> Result<(), ToolError> {
        let width = 980.0;
        let height = 560.0;
        let pad_l = 90.0;
        let pad_r = 220.0;
        let pad_t = 40.0;
        let pad_b = 70.0;

        let min_x = series
            .iter()
            .flat_map(|(_, pts)| pts.iter().map(|(x, _)| *x))
            .fold(f64::INFINITY, f64::min);
        let max_x = series
            .iter()
            .flat_map(|(_, pts)| pts.iter().map(|(x, _)| *x))
            .fold(f64::NEG_INFINITY, f64::max);
        let min_y = series
            .iter()
            .flat_map(|(_, pts)| pts.iter().map(|(_, y)| *y))
            .fold(f64::INFINITY, f64::min)
            .floor();
        let max_y = series
            .iter()
            .flat_map(|(_, pts)| pts.iter().map(|(_, y)| *y))
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil();

        let min_x = if min_x.is_finite() { min_x } else { 0.0 };
        let max_x = if max_x.is_finite() { max_x } else { 1.0 };
        let min_y = if min_y.is_finite() { min_y } else { 0.0 };
        let max_y = if max_y.is_finite() { max_y } else { 1.0 };

        let x_rng = (max_x - min_x).max(1.0);
        let y_rng = (max_y - min_y).max(1.0);
        let sx = |x: f64| pad_l + (x - min_x) / x_rng * (width - pad_l - pad_r);
        let sy = |y: f64| height - pad_b - (y - min_y) / y_rng * (height - pad_t - pad_b);

        let palette = [
            "#1f77b4", "#ff7f0e", "#2ca02c", "#d62728", "#9467bd", "#8c564b", "#17becf",
            "#bcbd22",
        ];

        let mut grid = String::new();
        for i in 0..=10 {
            let x = min_x + i as f64 / 10.0 * x_rng;
            let px = sx(x);
            grid.push_str(&format!(
                "<line x1='{:.2}' y1='{:.2}' x2='{:.2}' y2='{:.2}' stroke='#d0d0d0'/>",
                px,
                pad_t,
                px,
                height - pad_b
            ));
        }
        for i in 0..=8 {
            let yv = min_y + i as f64 / 8.0 * y_rng;
            let py = sy(yv);
            grid.push_str(&format!(
                "<line x1='{:.2}' y1='{:.2}' x2='{:.2}' y2='{:.2}' stroke='#d0d0d0'/>",
                pad_l,
                py,
                width - pad_r,
                py
            ));
        }

        let mut lines = String::new();
        let mut legend = String::new();
        for (i, (name, pts)) in series.iter().enumerate() {
            if pts.is_empty() {
                continue;
            }
            let color = palette[i % palette.len()];
            let poly = pts
                .iter()
                .map(|(x, y)| format!("{:.2},{:.2}", sx(*x), sy(*y)))
                .collect::<Vec<_>>()
                .join(" ");
            lines.push_str(&format!(
                "<polyline fill='none' stroke='{}' stroke-width='2.2' points='{}'/>",
                color, poly
            ));
            legend.push_str(&format!(
                "<div style='margin:6px 0'><span style='display:inline-block;width:22px;height:3px;background:{};vertical-align:middle;margin-right:8px'></span>{}</div>",
                color, name
            ));
        }

        let html = format!(
            "<!doctype html><html><head><meta charset='utf-8'><title>{}</title><style>body{{margin:0;background:#ececec;font-family:Helvetica,Arial,sans-serif;color:#111}}main{{max-width:1200px;margin:0 auto;padding:24px}}h1{{margin:0 0 12px 0;font-size:30px;font-weight:600}}.card{{background:#f3f3f3;border:1px solid #cfcfcf;border-radius:10px;padding:12px}}.meta{{margin:0 0 10px 2px;font-size:14px}}</style></head><body><main><h1>{}</h1><div class='card'><p class='meta'><strong>Input Surface</strong>: {}</p><svg viewBox='0 0 {} {}' width='100%' height='{}'>{}<rect x='{:.2}' y='{:.2}' width='{:.2}' height='{:.2}' fill='none' stroke='#5a5a5a' stroke-width='1.2'/>{}<text x='{:.2}' y='{:.2}' style='font-size:14px;fill:#222'>Distance</text><text transform='translate({:.2},{:.2}) rotate(-90)' style='font-size:14px;fill:#222'>Elevation</text></svg><div style='margin-top:10px'>{}</div></div></main></body></html>",
            title,
            title,
            surface_name,
            width,
            height,
            height,
            grid,
            pad_l,
            pad_t,
            width - pad_l - pad_r,
            height - pad_t - pad_b,
            lines,
            width / 2.0 - 32.0,
            height - 18.0,
            20.0,
            height / 2.0 + 42.0,
            legend,
        );

        std::fs::write(output_path, html)
            .map_err(|e| ToolError::Execution(format!("failed writing HTML report: {}", e)))
    }

    fn run_profile(args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let lines_path = parse_vector_path_arg(args, "lines_vector")
            .or_else(|_| parse_vector_path_arg(args, "lines"))
            .or_else(|_| parse_vector_path_arg(args, "input"))?;
        let surface_path = parse_raster_path_arg(args, "surface")
            .or_else(|_| parse_raster_path_arg(args, "dem"))
            .or_else(|_| parse_raster_path_arg(args, "input_surface"))?;

        let output_path = parse_optional_output_path(args, "output")?
            .unwrap_or_else(|| std::env::temp_dir().join("profile.html"));
        if let Some(parent) = output_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ToolError::Execution(format!("failed creating output directory: {e}"))
                })?;
            }
        }

        let lines = wbvector::read(&lines_path).map_err(|e| {
            ToolError::Execution(format!("failed reading lines vector '{}': {}", lines_path, e))
        })?;
        let surface = Self::load_raster(&surface_path)?;
        let nodata = surface.nodata;

        let mut xdata: Vec<Vec<f64>> = Vec::new();
        let mut ydata: Vec<Vec<f64>> = Vec::new();
        let mut names: Vec<String> = Vec::new();

        for (record_num, feature) in lines.features.iter().enumerate() {
            let mut parts = Vec::new();
            if let Some(geom) = &feature.geometry {
                Self::collect_profile_lines(geom, &mut parts);
            }
            for (part_num, part) in parts.iter().enumerate() {
                let mut px = Vec::new();
                let mut py = Vec::new();
                let mut dist = 0.0;

                for i in 0..part.len() - 1 {
                    let p0 = &part[i];
                    let p1 = &part[i + 1];
                    let start = surface.world_to_pixel(p0.x, p0.y);
                    let end = surface.world_to_pixel(p1.x, p1.y);
                    let (st_col, st_row, end_col, end_row) = if let (
                        Some((sc, sr)),
                        Some((ec, er)),
                    ) = (start, end)
                    {
                        (sc, sr, ec, er)
                    } else {
                        continue;
                    };

                    let mut dx = (end_col - st_col) as f64;
                    let mut dy = (end_row - st_row) as f64;
                    let path_dist = (dx * dx + dy * dy).sqrt();
                    if path_dist <= f64::EPSILON {
                        continue;
                    }

                    let num_steps = path_dist.ceil() as isize;
                    dx /= path_dist;
                    dy /= path_dist;

                    let dist_step = ((p0.x - p1.x) * (p0.x - p1.x) + (p0.y - p1.y) * (p0.y - p1.y))
                        .sqrt()
                        / path_dist;

                    if num_steps > 0 {
                        for j in 1..num_steps {
                            let col = (st_col as f64 + j as f64 * dx) as isize;
                            let row = (st_row as f64 + j as f64 * dy) as isize;
                            let z = surface.get(0, row, col);
                            dist += dist_step;
                            if z != nodata {
                                px.push(dist);
                                py.push(z);
                            }
                        }
                    }
                }

                if px.len() > 1 {
                    xdata.push(px);
                    ydata.push(py);
                    if parts.len() > 1 {
                        names.push(format!("Profile {} Part {}", record_num + 1, part_num + 1));
                    } else {
                        names.push(format!("Profile {}", record_num + 1));
                    }
                }
            }
        }

        if xdata.is_empty() {
            return Err(ToolError::Validation(
                "profile found no valid sampled line segments".to_string(),
            ));
        }

        let mut series = Vec::with_capacity(xdata.len());
        for i in 0..xdata.len() {
            let pts = xdata[i]
                .iter()
                .zip(ydata[i].iter())
                .map(|(x, y)| (*x, *y))
                .collect::<Vec<_>>();
            series.push((names[i].clone(), pts));
        }

        let surface_name = std::path::Path::new(&surface_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("surface");
        Self::write_profile_html(&output_path, "Profile", surface_name, &series)?;

        let mut outputs = std::collections::BTreeMap::new();
        outputs.insert("path".to_string(), json!(output_path.to_string_lossy().to_string()));
        Ok(ToolRunResult {
            outputs,
            ..Default::default()
        })
    }

    fn slope_vs_aspect_plot_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("aspect_bin_size".to_string(), json!(2.0));
        defaults.insert("min_slope".to_string(), json!(0.1));
        defaults.insert("z_factor".to_string(), json!(1.0));
        defaults.insert("output".to_string(), json!("slope_vs_aspect_plot.html"));
        ToolManifest {
            id: "slope_vs_aspect_plot".to_string(),
            display_name: "Slope Vs Aspect Plot".to_string(),
            summary: "Creates an HTML radial slope-vs-aspect analysis plot for an input DEM.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Pro,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "plot".to_string(), "html".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_slope_vs_aspect_plot(args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let aspect_bin_size = args
            .get("aspect_bin_size")
            .or_else(|| args.get("bin_size"))
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0);
        if aspect_bin_size <= 0.0 {
            return Err(ToolError::Validation(
                "parameter 'aspect_bin_size' must be greater than 0".to_string(),
            ));
        }
        let min_slope = args
            .get("min_slope")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.1)
            .max(0.0);
        let z_factor = args
            .get("z_factor")
            .or_else(|| args.get("zfactor"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let output_path = parse_optional_output_path(args, "output")?
            .unwrap_or_else(|| std::env::temp_dir().join("slope_vs_aspect_plot.html"));
        if let Some(parent) = output_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ToolError::Execution(format!("failed creating output directory: {e}"))
                })?;
            }
        }

        let dem = Self::load_raster(&input_path)?;
        if dem.bands < 1 {
            return Err(ToolError::Validation(
                "input DEM must have at least one band".to_string(),
            ));
        }

        let num_bins = (360.0 / aspect_bin_size).ceil() as usize;
        let mut bins: Vec<Vec<f64>> = vec![Vec::new(); num_bins.max(1)];
        let rows = dem.rows;
        let cols = dem.cols;
        let band = 0isize;
        let res = ((dem.cell_size_x.abs() + dem.cell_size_y.abs()) * 0.5).max(f64::EPSILON);
        let is_geographic = Self::raster_is_geographic(&dem);

        let geographic_row_metrics = if is_geographic {
            let src_crs = if let Some(code) = dem.crs.epsg {
                Some(Crs::from_epsg(code).map_err(|e| {
                    ToolError::Validation(format!("failed parsing DEM CRS EPSG {}: {}", code, e))
                })?)
            } else if let Some(wkt) = dem.crs.wkt.as_deref() {
                Some(wbprojection::from_wkt(wkt).map_err(|e| {
                    ToolError::Validation(format!("failed parsing DEM CRS WKT: {}", e))
                })?)
            } else {
                None
            };

            if let Some(src) = src_crs {
                let wgs84 = Crs::from_epsg(4326).map_err(|e| {
                    ToolError::Execution(format!("failed constructing EPSG:4326 CRS: {}", e))
                })?;
                let center_x = (dem.x_min + dem.x_max()) * 0.5;
                let center_y = (dem.y_min + dem.y_max()) * 0.5;
                let (center_lon, center_lat) = src
                    .transform_to(center_x, center_y, &wgs84)
                    .map_err(|e| {
                        ToolError::Execution(format!(
                            "failed transforming DEM center to geographic coordinates: {}",
                            e
                        ))
                    })?;

                let zone = (((center_lon + 180.0) / 6.0).floor() as i32 + 1).clamp(1, 60) as u32;
                let utm_epsg = if center_lat >= 0.0 {
                    32600 + zone
                } else {
                    32700 + zone
                };
                let metric_crs = Crs::from_epsg(utm_epsg).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed constructing local projected CRS EPSG:{}: {}",
                        utm_epsg, e
                    ))
                })?;

                let sample_col = (cols / 2) as isize;
                let west_col = if sample_col > 0 {
                    sample_col - 1
                } else {
                    sample_col + 1
                };

                let mut metrics = vec![(res, res, res, res, res); rows];
                for (r, metric) in metrics.iter_mut().enumerate() {
                    let row = r as isize;
                    let row_n = (row - 1).max(0);
                    let row_s = (row + 1).min(rows as isize - 1);

                    let x0 = dem.col_center_x(sample_col);
                    let xw = dem.col_center_x(west_col);
                    let y0 = dem.row_center_y(row);
                    let yn = dem.row_center_y(row_n);
                    let ys = dem.row_center_y(row_s);

                    let project = |x: f64, y: f64| src.transform_to(x, y, &metric_crs);
                    let d2 = |p1: (f64, f64), p2: (f64, f64)| {
                        let dx = p2.0 - p1.0;
                        let dy = p2.1 - p1.1;
                        (dx * dx + dy * dy).sqrt()
                    };

                    let p0 = project(x0, y0).map_err(|e| {
                        ToolError::Execution(format!(
                            "failed transforming DEM coordinate to local projected CRS: {}",
                            e
                        ))
                    })?;
                    let pw0 = project(xw, y0).map_err(|e| {
                        ToolError::Execution(format!(
                            "failed transforming DEM coordinate to local projected CRS: {}",
                            e
                        ))
                    })?;
                    let p0s = project(x0, ys).map_err(|e| {
                        ToolError::Execution(format!(
                            "failed transforming DEM coordinate to local projected CRS: {}",
                            e
                        ))
                    })?;
                    let pws = project(xw, ys).map_err(|e| {
                        ToolError::Execution(format!(
                            "failed transforming DEM coordinate to local projected CRS: {}",
                            e
                        ))
                    })?;
                    let p0n = project(x0, yn).map_err(|e| {
                        ToolError::Execution(format!(
                            "failed transforming DEM coordinate to local projected CRS: {}",
                            e
                        ))
                    })?;
                    let pwn = project(xw, yn).map_err(|e| {
                        ToolError::Execution(format!(
                            "failed transforming DEM coordinate to local projected CRS: {}",
                            e
                        ))
                    })?;

                    let b = d2(p0, pw0).max(f64::EPSILON);
                    let d = d2(p0, p0s).max(f64::EPSILON);
                    let e = d2(p0, p0n).max(f64::EPSILON);
                    let a = d2(p0s, pws).max(f64::EPSILON);
                    let c = d2(p0n, pwn).max(f64::EPSILON);
                    *metric = (a, b, c, d, e);
                }
                Some(metrics)
            } else {
                None
            }
        } else {
            None
        };

        for r in 0..rows {
            let row = r as isize;
            for c in 0..cols {
                let col = c as isize;
                let zc = dem.get(band, row, col);
                if dem.is_nodata(zc) {
                    continue;
                }

                let z = |dr: isize, dc: isize| {
                    let v = dem.get(band, row + dr, col + dc);
                    if dem.is_nodata(v) {
                        zc * z_factor
                    } else {
                        v * z_factor
                    }
                };

                let (p, q) = if !is_geographic {
                    // 5x5 Florinsky derivatives for projected coordinates (legacy-compatible).
                    let mut zz = [0.0f64; 25];
                    let mut k = 0usize;
                    for dr in -2..=2 {
                        for dc in -2..=2 {
                            zz[k] = z(dr, dc);
                            k += 1;
                        }
                    }

                    let q = (44.0 * (zz[3] + zz[23] - zz[1] - zz[21])
                        + 31.0
                            * (zz[0] + zz[20] - zz[4] - zz[24]
                                + 2.0 * (zz[8] + zz[18] - zz[6] - zz[16]))
                        + 17.0 * (zz[14] - zz[10] + 4.0 * (zz[13] - zz[11]))
                        + 5.0 * (zz[9] + zz[19] - zz[5] - zz[15]))
                        / (420.0 * res);

                    let p = (44.0 * (zz[5] + zz[9] - zz[15] - zz[19])
                        + 31.0
                            * (zz[20] + zz[24] - zz[0] - zz[4]
                                + 2.0 * (zz[6] + zz[8] - zz[16] - zz[18]))
                        + 17.0 * (zz[2] - zz[22] + 4.0 * (zz[7] - zz[17]))
                        + 5.0 * (zz[1] + zz[3] - zz[21] - zz[23]))
                        / (420.0 * res);

                    (p, q)
                } else {
                    // Geographic coordinates use legacy local-distance derivatives (Florinsky).
                    let z0 = z(-1, -1);
                    let z1 = z(-1, 0);
                    let z2 = z(-1, 1);
                    let z3 = z(0, -1);
                    let z5 = z(0, 1);
                    let z6 = z(1, -1);
                    let z7 = z(1, 0);
                    let z8 = z(1, 1);

                    let (a, b, c, d, e) = if let Some(metrics) = &geographic_row_metrics {
                        metrics[r]
                    } else {
                        let mid_lat = dem.row_center_y(row).to_radians();
                        let dx = (dem.cell_size_x.abs() * 111_320.0 * mid_lat.cos().abs().max(1.0e-8))
                            .max(f64::EPSILON);
                        let dy = (dem.cell_size_y.abs() * 111_320.0).max(f64::EPSILON);
                        (dx, dx, dx, dy, dy)
                    };

                    let p_num = a * a * c * d * (d + e) * (z2 - z0)
                        + b * (a * a * d * d + c * c * e * e) * (z5 - z3)
                        + a * c * c * e * (d + e) * (z8 - z6);
                    let p_den = 2.0
                        * (a * a * c * c * (d + e) * (d + e)
                            + b * b * (a * a * d * d + c * c * e * e));
                    let p = if p_den.abs() > f64::EPSILON {
                        p_num / p_den
                    } else {
                        0.0
                    };

                    let q_num = (d * d * (a.powi(4) + b.powi(4) + b * b * c * c)
                        + c * c * e * e * (a * a - b * b))
                        * (z0 + z2)
                        - (d * d * (a.powi(4) + c.powi(4) + b * b * c * c)
                            - e * e * (a.powi(4) + c.powi(4) + a * a * b * b))
                            * (z3 + z5)
                        - (e * e * (b.powi(4) + c.powi(4) + a * a * b * b)
                            - a * a * d * d * (b * b - c * c))
                            * (z6 + z8)
                        + d * d
                            * (b.powi(4) * (z1 - 3.0 * zc * z_factor)
                                + c.powi(4) * (3.0 * z1 - zc * z_factor)
                                + (a.powi(4) - 2.0 * b * b * c * c) * (z1 - zc * z_factor))
                        + e * e
                            * (a.powi(4) * (zc * z_factor - 3.0 * z7)
                                + b.powi(4) * (3.0 * zc * z_factor - z7)
                                + (c.powi(4) - 2.0 * a * a * b * b) * (zc * z_factor - z7))
                        - 2.0 * (a * a * d * d * (b * b - c * c) * z7
                            + c * c * e * e * (a * a - b * b) * z1);
                    let q_den = 3.0 * d * e * (d + e) * (a.powi(4) + b.powi(4) + c.powi(4));
                    let q = if q_den.abs() > f64::EPSILON {
                        q_num / q_den
                    } else {
                        0.0
                    };

                    (p, q)
                };

                let grad_mag = (p * p + q * q).sqrt();
                if grad_mag <= f64::EPSILON {
                    continue;
                }
                let slope_deg = grad_mag.atan().to_degrees();
                if slope_deg <= min_slope {
                    continue;
                }

                let sign_p = if p != 0.0 { p.signum() } else { 0.0 };
                let sign_q = if q != 0.0 { q.signum() } else { 0.0 };
                let ratio = (-q / grad_mag).clamp(-1.0, 1.0);
                let mut aspect = -90.0 * (1.0 - sign_q) * (1.0 - sign_p.abs())
                    + 180.0 * (1.0 + sign_p)
                    - 180.0 / std::f64::consts::PI * sign_p * ratio.acos();
                if aspect < 0.0 {
                    aspect += 360.0;
                }
                if aspect >= 360.0 {
                    aspect -= 360.0;
                }
                let b = ((aspect / aspect_bin_size).floor() as usize).min(bins.len() - 1);
                bins[b].push(slope_deg);
            }
        }

        let non_empty = bins.iter().any(|b| !b.is_empty());
        if !non_empty {
            return Err(ToolError::Validation(
                "slope_vs_aspect_plot found no valid slope/aspect samples".to_string(),
            ));
        }

        for b in &mut bins {
            b.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        }

        let mut angles = Vec::with_capacity(bins.len());
        let mut p25 = Vec::with_capacity(bins.len());
        let mut p50 = Vec::with_capacity(bins.len());
        let mut p75 = Vec::with_capacity(bins.len());
        for i in 0..bins.len() {
            angles.push(i as f64 * aspect_bin_size);
            if !bins[i].is_empty() {
                p25.push(Self::percentile_sorted(&bins[i], 0.25));
                p50.push(Self::percentile_sorted(&bins[i], 0.50));
                p75.push(Self::percentile_sorted(&bins[i], 0.75));
                continue;
            }

            let mut found = None;
            for d in 1..bins.len() {
                let l = (i + bins.len() - d) % bins.len();
                let r = (i + d) % bins.len();
                if !bins[l].is_empty() {
                    found = Some(l);
                    break;
                }
                if !bins[r].is_empty() {
                    found = Some(r);
                    break;
                }
            }
            let src = found.unwrap_or(0);
            p25.push(Self::percentile_sorted(&bins[src], 0.25));
            p50.push(Self::percentile_sorted(&bins[src], 0.50));
            p75.push(Self::percentile_sorted(&bins[src], 0.75));
        }

        let elong25 = Self::elongation_ratio_from_polar(&angles, &p25);
        let elong50 = Self::elongation_ratio_from_polar(&angles, &p50);
        let elong75 = Self::elongation_ratio_from_polar(&angles, &p75);
        let dem_name = std::path::Path::new(&input_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("dem");
        Self::write_slope_vs_aspect_html(
            &output_path,
            dem_name,
            &angles,
            &p25,
            &p50,
            &p75,
            elong25,
            elong50,
            elong75,
        )?;

        let mut outputs = std::collections::BTreeMap::new();
        outputs.insert("path".to_string(), json!(output_path.to_string_lossy().to_string()));
        Ok(ToolRunResult {
            outputs,
            ..Default::default()
        })
    }

    fn slope_vs_elev_plot_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "slope_vs_elev_plot",
            display_name: "Slope Vs Elev Plot",
            summary: "Creates an HTML slope-vs-elevation analysis chart for one or more DEMs.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Input DEM paths as ';' or ',' separated string, or list.", required: true },
                ToolParamSpec { name: "watershed", description: "Optional watershed rasters matching each input DEM.", required: false },
                ToolParamSpec { name: "output", description: "Output HTML report path.", required: false },
            ],
        }
    }

    fn slope_vs_elev_plot_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!("dem.tif"));
        defaults.insert("output".to_string(), json!("slope_vs_elev_plot.html"));
        ToolManifest {
            id: "slope_vs_elev_plot".to_string(),
            display_name: "Slope Vs Elev Plot".to_string(),
            summary: "Creates an HTML slope-vs-elevation analysis chart for one or more DEMs.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "plot".to_string(), "html".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_slope_vs_elev_plot(args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = Self::parse_raster_input_list(args, "inputs")?;
        let watershed_paths = if args.get("watershed").is_some() {
            Some(Self::parse_raster_input_list(args, "watershed")?)
        } else {
            None
        };
        if let Some(ws) = &watershed_paths {
            if ws.len() != input_paths.len() {
                return Err(ToolError::Validation(
                    "watershed list length must match inputs list length".to_string(),
                ));
            }
        }

        let output_path = parse_optional_output_path(args, "output")?
            .unwrap_or_else(|| std::env::temp_dir().join("slope_vs_elev_plot.html"));
        if let Some(parent) = output_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ToolError::Execution(format!("failed creating output directory: {e}"))
                })?;
            }
        }

        let mut series: Vec<(String, Vec<(f64, f64)>)> = Vec::new();

        for (i, in_path) in input_paths.iter().enumerate() {
            let dem = Self::load_raster(in_path)?;
            if dem.bands < 1 {
                continue;
            }
            let ws = if let Some(ws_list) = &watershed_paths {
                let w = Self::load_raster(&ws_list[i])?;
                if w.rows != dem.rows || w.cols != dem.cols {
                    return Err(ToolError::Validation(
                        "watershed raster dimensions must match corresponding DEM".to_string(),
                    ));
                }
                Some(w)
            } else {
                None
            };

            let rows = dem.rows;
            let cols = dem.cols;
            let band = 0isize;
            let cell_x = dem.cell_size_x.abs().max(f64::EPSILON);
            let cell_y = dem.cell_size_y.abs().max(f64::EPSILON);
            let z_row: Vec<f64> = if Self::raster_is_geographic(&dem) {
                (0..rows)
                    .map(|r| {
                        let lat = dem.row_center_y(r as isize).to_radians();
                        1.0 / (111_320.0 * lat.cos().abs().max(1.0e-8))
                    })
                    .collect()
            } else {
                vec![1.0; rows]
            };

            let mut grouped: std::collections::BTreeMap<i64, Vec<(f64, f64)>> =
                std::collections::BTreeMap::new();

            for r in 0..rows {
                let row = r as isize;
                let zf = z_row[r];
                for c in 0..cols {
                    let col = c as isize;
                    let zc = dem.get(band, row, col);
                    if dem.is_nodata(zc) {
                        continue;
                    }

                    let gid = if let Some(w) = &ws {
                        let wv = w.get(0, row, col);
                        if w.is_nodata(wv) || wv == 0.0 {
                            continue;
                        }
                        wv.round() as i64
                    } else {
                        1
                    };

                    let z = |dr: isize, dc: isize| {
                        let v = dem.get(band, row + dr, col + dc);
                        if dem.is_nodata(v) {
                            zc * zf
                        } else {
                            v * zf
                        }
                    };
                    let n0 = z(-1, -1);
                    let n1 = z(-1, 0);
                    let n2 = z(-1, 1);
                    let n3 = z(0, 1);
                    let n4 = z(1, 1);
                    let n5 = z(1, 0);
                    let n6 = z(1, -1);
                    let n7 = z(0, -1);
                    let fx = (n2 - n4 + 2.0 * (n1 - n5) + n0 - n6) / (8.0 * cell_x);
                    let fy = (n6 - n4 + 2.0 * (n7 - n3) + n0 - n2) / (8.0 * cell_y);
                    let slope_deg = (fx * fx + fy * fy).sqrt().atan().to_degrees();
                    if !slope_deg.is_finite() {
                        continue;
                    }
                    grouped.entry(gid).or_default().push((zc, slope_deg));
                }
            }

            let dem_name = std::path::Path::new(in_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("dem")
                .to_string();
            for (gid, vals) in grouped {
                if vals.len() < 2 {
                    continue;
                }

                let min = vals.iter().map(|(z, _)| *z).fold(f64::INFINITY, f64::min);
                let max = vals
                    .iter()
                    .map(|(z, _)| *z)
                    .fold(f64::NEG_INFINITY, f64::max);
                let mut num_bins = ((max - min) as usize) / 5;
                let min_bins = (vals.len() as f64).log2().ceil() as usize + 1;
                if num_bins < min_bins {
                    num_bins = min_bins;
                }
                num_bins = num_bins.max(8);
                let range = (max - min + 0.00001).max(f64::EPSILON);
                let bin_width = range / num_bins as f64;

                let mut count = vec![0usize; num_bins];
                let mut slope_sum = vec![0.0; num_bins];
                for (z, s) in vals {
                    let b = (((z - min) / bin_width).floor() as usize).min(num_bins - 1);
                    count[b] += 1;
                    slope_sum[b] += s;
                }

                let mut pts = Vec::new();
                for b in 0..num_bins {
                    if count[b] == 0 {
                        continue;
                    }
                    let avg_slope = slope_sum[b] / count[b] as f64;
                    let elev = min + b as f64 * bin_width;
                    pts.push((avg_slope, elev));
                }
                if pts.len() < 2 {
                    continue;
                }
                let name = if ws.is_some() {
                    format!("{}_ws{}", dem_name, gid)
                } else {
                    dem_name.clone()
                };
                series.push((name, pts));
            }
        }

        if series.is_empty() {
            return Err(ToolError::Validation(
                "slope_vs_elev_plot found no valid elevation/slope samples".to_string(),
            ));
        }

        Self::write_slope_vs_elev_html(&output_path, &series)?;

        let mut outputs = std::collections::BTreeMap::new();
        outputs.insert("path".to_string(), json!(output_path.to_string_lossy().to_string()));
        Ok(ToolRunResult {
            outputs,
            ..Default::default()
        })
    }

    fn elev_above_pit_dist_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "elev_above_pit_dist",
            display_name: "Elev Above Pit Dist",
            summary: "Compatibility alias for elev_above_pit.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn elev_above_pit_dist_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        ToolManifest {
            id: "elev_above_pit_dist".to_string(),
            display_name: "Elev Above Pit Dist".to_string(),
            summary: "Compatibility alias for elev_above_pit.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_elev_above_pit_dist(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        Self::run_elev_above_pit(args, ctx)
    }

    fn circular_variance_of_aspect_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "circular_variance_of_aspect",
            display_name: "Circular Variance Of Aspect",
            summary: "Calculates local circular variance of aspect within a moving neighbourhood.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "filter", description: "Odd neighbourhood size in cells (default 11).", required: false },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn circular_variance_of_aspect_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("filter".to_string(), json!(11));
        ToolManifest {
            id: "circular_variance_of_aspect".to_string(),
            display_name: "Circular Variance Of Aspect".to_string(),
            summary: "Calculates local circular variance of aspect within a moving neighbourhood.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "texture".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_circular_variance_of_aspect(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let mut filter_size = args.get("filter").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(11).max(3);
        if filter_size % 2 == 0 {
            filter_size += 1;
        }
        let mid = filter_size / 2;

        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let sigma = (mid as f64 + 0.5) / 3.0;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running circular_variance_of_aspect");

            let mut values = vec![f64::NAN; rows * cols];
            for r in 0..rows {
                for c in 0..cols {
                    let v = input.get(band, r as isize, c as isize);
                    if !input.is_nodata(v) {
                        values[r * cols + c] = v;
                    }
                }
            }

            let smoothed = if filter_size > 3 {
                Self::gaussian_blur_values(&values, rows, cols, sigma)
            } else {
                values
            };

            let mut cos_a = vec![f64::NAN; rows * cols];
            let mut sin_a = vec![f64::NAN; rows * cols];
            let mut flat = vec![false; rows * cols];
            for r in 0..rows {
                for c in 0..cols {
                    let idx = r * cols + c;
                    let zc = smoothed[idx];
                    if !zc.is_finite() {
                        continue;
                    }
                    let z = |dr: isize, dc: isize| {
                        let rr = r as isize + dr;
                        let cc = c as isize + dc;
                        if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                            return zc;
                        }
                        let v = smoothed[rr as usize * cols + cc as usize];
                        if v.is_finite() { v } else { zc }
                    };
                    let z1 = z(-1, -1);
                    let z2 = z(-1, 0);
                    let z3 = z(-1, 1);
                    let z4 = z(0, -1);
                    let z6 = z(0, 1);
                    let z7 = z(1, -1);
                    let z8 = z(1, 0);
                    let z9 = z(1, 1);
                    let dzdx = ((z3 + 2.0 * z6 + z9) - (z1 + 2.0 * z4 + z7)) / 8.0;
                    let dzdy = ((z7 + 2.0 * z8 + z9) - (z1 + 2.0 * z2 + z3)) / 8.0;
                    let slope_mag = (dzdx * dzdx + dzdy * dzdy).sqrt();
                    if slope_mag <= f64::EPSILON {
                        flat[idx] = true;
                        continue;
                    }
                    let mut aspect = dzdy.atan2(-dzdx);
                    if aspect < 0.0 {
                        aspect += std::f64::consts::PI * 2.0;
                    }
                    cos_a[idx] = aspect.cos();
                    sin_a[idx] = aspect.sin();
                }
            }

            let (sum_cos, count_cos) = Self::build_integral_from_values(&cos_a, rows, cols);
            let (sum_sin, _) = Self::build_integral_from_values(&sin_a, rows, cols);

            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let idx = r * cols + c;
                        let z = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z) {
                            continue;
                        }
                        if flat[idx] {
                            row_out[c] = 0.0;
                            continue;
                        }
                        let y1 = r.saturating_sub(mid);
                        let x1 = c.saturating_sub(mid);
                        let y2 = (r + mid).min(rows - 1);
                        let x2 = (c + mid).min(cols - 1);
                        let n = Self::rect_count(&count_cos, cols, y1, x1, y2, x2);
                        if n <= 0 {
                            row_out[c] = 0.0;
                            continue;
                        }
                        let cbar = Self::rect_sum(&sum_cos, cols, y1, x1, y2, x2) / n as f64;
                        let sbar = Self::rect_sum(&sum_sin, cols, y1, x1, y2, x2) / n as f64;
                        let rlen = (cbar * cbar + sbar * sbar).sqrt().clamp(0.0, 1.0);
                        row_out[c] = 1.0 - rlen;
                    }
                    row_out
                })
                .collect();

            for (r, row) in row_data.iter().enumerate() {
                output.set_row_slice(band, r as isize, row).map_err(|e| {
                    ToolError::Execution(format!("failed writing row {}: {}", r, e))
                })?;
            }
            ctx.progress.progress((band_idx + 1) as f64 / bands as f64);
        }

        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn fetch_analysis_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "fetch_analysis",
            display_name: "Fetch Analysis",
            summary: "Computes upwind distance to the first topographic obstacle along a specified azimuth.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "azimuth", description: "Wind azimuth in degrees clockwise from north.", required: false },
                ToolParamSpec { name: "hgt_inc", description: "Height increment threshold in m/m (default 0.05).", required: false },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn fetch_analysis_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("azimuth".to_string(), json!(0.0));
        defaults.insert("hgt_inc".to_string(), json!(0.05));
        ToolManifest {
            id: "fetch_analysis".to_string(),
            display_name: "Fetch Analysis".to_string(),
            summary: "Computes upwind distance to the first topographic obstacle along a specified azimuth.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "visibility".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_fetch_analysis(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let azimuth = args.get("azimuth").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let hgt_inc = args.get("hgt_inc").and_then(|v| v.as_f64()).unwrap_or(0.05);

        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let is_geographic = Self::raster_is_geographic(&input);

        let mut cell_size = (input.cell_size_x.abs() + input.cell_size_y.abs()) / 2.0;
        if is_geographic {
            let lat_mid = input.row_center_y((rows / 2) as isize).to_radians();
            cell_size *= 111_320.0 * lat_mid.cos().abs().max(1e-8);
        }

        let mut az = azimuth % 360.0;
        if az < 0.0 {
            az += 360.0;
        }
        let theta = az.to_radians();
        let dx = theta.sin();
        let dy = -theta.cos();
        let max_steps = ((rows * rows + cols * cols) as f64).sqrt() as usize + 2;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running fetch_analysis");
            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let z0 = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z0) {
                            continue;
                        }
                        let mut found = None;
                        let mut edge_dist = 0.0;
                        for step in 1..=max_steps {
                            let t = step as f64;
                            let x = c as f64 + dx * t;
                            let y = r as f64 + dy * t;
                            if x < 0.0 || y < 0.0 || x >= (cols - 1) as f64 || y >= (rows - 1) as f64 {
                                break;
                            }
                            let dist = t * cell_size;
                            edge_dist = dist;
                            if let Some(z) = Self::bilinear_sample(&input, band, y, x) {
                                if z >= z0 + dist * hgt_inc {
                                    found = Some(dist);
                                    break;
                                }
                            }
                        }
                        row_out[c] = found.unwrap_or(-edge_dist);
                    }
                    row_out
                })
                .collect();

            for (r, row) in row_data.iter().enumerate() {
                output.set_row_slice(band, r as isize, row).map_err(|e| {
                    ToolError::Execution(format!("failed writing row {}: {}", r, e))
                })?;
            }
            ctx.progress.progress((band_idx + 1) as f64 / bands as f64);
        }

        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn find_ridges_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "find_ridges",
            display_name: "Find Ridges",
            summary: "Identifies potential ridge and peak cells in a DEM, with optional line thinning.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "line_thin", description: "Apply iterative line thinning (default true).", required: false },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn find_ridges_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("line_thin".to_string(), json!(true));
        ToolManifest {
            id: "find_ridges".to_string(),
            display_name: "Find Ridges".to_string(),
            summary: "Identifies potential ridge and peak cells in a DEM, with optional line thinning.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "ridges".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_find_ridges(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let line_thin = args.get("line_thin").and_then(|v| v.as_bool()).unwrap_or(true);

        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running find_ridges");
            let mut grid = vec![vec![nodata; cols]; rows];

            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let z = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z) {
                            continue;
                        }
                        let n = input.get(band, r as isize - 1, c as isize);
                        let s = input.get(band, r as isize + 1, c as isize);
                        let w = input.get(band, r as isize, c as isize - 1);
                        let e = input.get(band, r as isize, c as isize + 1);
                        if (!input.is_nodata(n) && !input.is_nodata(s) && n < z && s < z)
                            || (!input.is_nodata(w) && !input.is_nodata(e) && w < z && e < z)
                        {
                            row_out[c] = 1.0;
                        } else {
                            row_out[c] = 0.0;
                        }
                    }
                    row_out
                })
                .collect();

            for (r, row) in row_data.into_iter().enumerate() {
                grid[r] = row;
            }

            if line_thin {
                let dx = [1isize, 1, 1, 0, -1, -1, -1, 0];
                let dy = [-1isize, 0, 1, 1, 1, 0, -1, -1];
                let elements: [Vec<usize>; 8] = [
                    vec![6, 7, 0, 4, 3, 2],
                    vec![7, 0, 1, 3, 5],
                    vec![0, 1, 2, 4, 5, 6],
                    vec![1, 2, 3, 5, 7],
                    vec![2, 3, 4, 6, 7, 0],
                    vec![3, 4, 5, 7, 1],
                    vec![4, 5, 6, 0, 1, 2],
                    vec![5, 6, 7, 1, 3],
                ];
                let vals: [Vec<f64>; 8] = [
                    vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
                    vec![0.0, 0.0, 0.0, 1.0, 1.0],
                    vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
                    vec![0.0, 0.0, 0.0, 1.0, 1.0],
                    vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
                    vec![0.0, 0.0, 0.0, 1.0, 1.0],
                    vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
                    vec![0.0, 0.0, 0.0, 1.0, 1.0],
                ];

                let mut changed = true;
                while changed {
                    changed = false;
                    for a in 0..8 {
                        let mut to_remove = Vec::new();
                        for r in 0..rows {
                            for c in 0..cols {
                                if grid[r][c] <= 0.0 || grid[r][c] == nodata {
                                    continue;
                                }
                                let mut nbs = [0.0; 8];
                                for i in 0..8 {
                                    let rr = r as isize + dy[i];
                                    let cc = c as isize + dx[i];
                                    if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                                        nbs[i] = 0.0;
                                    } else {
                                        let v = grid[rr as usize][cc as usize];
                                        nbs[i] = if v == nodata { 0.0 } else { v };
                                    }
                                }
                                let mut pattern_match = true;
                                for i in 0..elements[a].len() {
                                    if (nbs[elements[a][i]] - vals[a][i]).abs() > f64::EPSILON {
                                        pattern_match = false;
                                        break;
                                    }
                                }
                                if pattern_match {
                                    to_remove.push((r, c));
                                }
                            }
                        }
                        if !to_remove.is_empty() {
                            changed = true;
                            for (r, c) in to_remove {
                                grid[r][c] = 0.0;
                            }
                        }
                    }
                }
            }

            for (r, row) in grid.iter().enumerate() {
                output.set_row_slice(band, r as isize, row).map_err(|e| {
                    ToolError::Execution(format!("failed writing row {}: {}", r, e))
                })?;
            }
            ctx.progress.progress((band_idx + 1) as f64 / bands as f64);
        }

        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn assess_route_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "assess_route",
            display_name: "Assess Route",
            summary: "Segments route lines and evaluates per-segment terrain slope, relief, sinuosity, roughness, and visibility metrics from a DEM.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Pro,
            params: vec![
                ToolParamSpec {
                    name: "routes",
                    description: "Input polyline routes vector path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "dem",
                    description: "Input DEM raster path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "segment_length",
                    description: "Target route segment length in map units (default 100.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "search_radius",
                    description: "Visibility search radius in grid cells (default 15, minimum 4).",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output vector path.",
                    required: false,
                },
            ],
        }
    }

    fn assess_route_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("routes".to_string(), json!("routes.shp"));
        defaults.insert("dem".to_string(), json!("dem.tif"));
        defaults.insert("segment_length".to_string(), json!(100.0));
        defaults.insert("search_radius".to_string(), json!(15));
        defaults.insert("output".to_string(), json!("assessed_routes.shp"));

        ToolManifest {
            id: "assess_route".to_string(),
            display_name: "Assess Route".to_string(),
            summary: "Segments route lines and evaluates per-segment terrain metrics from a DEM."
                .to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Pro,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec![
                "geomorphometry".to_string(),
                "route".to_string(),
                "vector".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn collect_route_parts(geom: &wbvector::Geometry, parts: &mut Vec<Vec<wbvector::Coord>>) {
        match geom {
            wbvector::Geometry::LineString(coords) => {
                if coords.len() >= 2 {
                    parts.push(coords.clone());
                }
            }
            wbvector::Geometry::MultiLineString(lines) => {
                for line in lines {
                    if line.len() >= 2 {
                        parts.push(line.clone());
                    }
                }
            }
            wbvector::Geometry::GeometryCollection(geoms) => {
                for g in geoms {
                    Self::collect_route_parts(g, parts);
                }
            }
            _ => {}
        }
    }

    fn split_route_part_by_length(
        part: &[wbvector::Coord],
        segment_length: f64,
    ) -> Vec<Vec<wbvector::Coord>> {
        if part.len() < 2 || segment_length <= 0.0 {
            return Vec::new();
        }

        let mut segments: Vec<Vec<wbvector::Coord>> = Vec::new();
        let mut current: Vec<wbvector::Coord> = vec![part[0].clone()];
        let mut accum_dist = 0.0;

        let mut idx = 1usize;
        while idx < part.len() {
            let p1 = current[current.len() - 1].clone();
            let p2 = part[idx].clone();
            let seg_dx = p2.x - p1.x;
            let seg_dy = p2.y - p1.y;
            let seg_len = (seg_dx * seg_dx + seg_dy * seg_dy).sqrt();

            if seg_len <= f64::EPSILON {
                idx += 1;
                continue;
            }

            if accum_dist + seg_len <= segment_length + f64::EPSILON {
                current.push(p2);
                accum_dist += seg_len;
                idx += 1;
            } else {
                let ratio = (segment_length - accum_dist) / seg_len;
                let split = wbvector::Coord::xy(p1.x + ratio * seg_dx, p1.y + ratio * seg_dy);
                current.push(split.clone());
                if current.len() >= 2 {
                    segments.push(current.clone());
                }
                current = vec![split];
                accum_dist = 0.0;
            }
        }

        if current.len() >= 2 {
            segments.push(current);
        }

        segments
    }

    fn visibility_openness(
        dem: &Raster,
        row: isize,
        col: isize,
        search_radius: isize,
    ) -> Option<f64> {
        if row < 0 || col < 0 || row >= dem.rows as isize || col >= dem.cols as isize {
            return None;
        }

        let z0 = dem.get(0, row, col);
        if dem.is_nodata(z0) {
            return None;
        }

        let res_x = dem.cell_size_x.abs();
        let res_y = dem.cell_size_y.abs();
        let res_diag = (res_x * res_x + res_y * res_y).sqrt();
        let diag_radius = ((search_radius as f64 * res_x) / res_diag).max(1.0) as isize;

        let dirs: [(isize, isize, f64, isize); 8] = [
            (-1, 0, res_y, search_radius),
            (-1, 1, res_diag, diag_radius),
            (0, 1, res_x, search_radius),
            (1, 1, res_diag, diag_radius),
            (1, 0, res_y, search_radius),
            (1, -1, res_diag, diag_radius),
            (0, -1, res_x, search_radius),
            (-1, -1, res_diag, diag_radius),
        ];

        let mut openness_sum = 0.0;
        let mut n_profiles = 0.0;

        for (dr, dc, cell_dist, max_n) in dirs {
            let mut max_theta = f64::NEG_INFINITY;
            for n in 1..=max_n {
                let rr = row + dr * n;
                let cc = col + dc * n;
                if rr < 0 || cc < 0 || rr >= dem.rows as isize || cc >= dem.cols as isize {
                    break;
                }
                let zn = dem.get(0, rr, cc);
                if dem.is_nodata(zn) {
                    continue;
                }
                let dist = cell_dist * n as f64;
                if dist <= f64::EPSILON {
                    continue;
                }
                let theta = ((zn - z0) / dist).atan();
                if theta > max_theta {
                    max_theta = theta;
                }
            }

            if max_theta.is_finite() {
                openness_sum += 90.0 - max_theta.to_degrees();
                n_profiles += 1.0;
            }
        }

        if n_profiles > 0.0 {
            Some(openness_sum / n_profiles)
        } else {
            None
        }
    }

    fn compute_assess_route_metrics(
        dem: &Raster,
        segment: &[wbvector::Coord],
        search_radius: isize,
    ) -> Option<AssessRouteMetrics> {
        if segment.len() < 2 {
            return None;
        }

        let mut cumulative = vec![0.0f64; segment.len()];
        for i in 1..segment.len() {
            let dx = segment[i].x - segment[i - 1].x;
            let dy = segment[i].y - segment[i - 1].y;
            cumulative[i] = cumulative[i - 1] + (dx * dx + dy * dy).sqrt();
        }
        let total_len = cumulative[cumulative.len() - 1];
        if total_len <= f64::EPSILON {
            return None;
        }

        let sample_step = (dem.cell_size_x.abs().min(dem.cell_size_y.abs()) * 0.5).max(1e-6);
        let mut sample_dists: Vec<f64> = vec![0.0];
        let mut d = sample_step;
        while d < total_len {
            sample_dists.push(d);
            d += sample_step;
        }
        if (sample_dists[sample_dists.len() - 1] - total_len).abs() > 1e-9 {
            sample_dists.push(total_len);
        }

        let mut samples: Vec<(f64, f64, f64, f64)> = Vec::new();
        let mut max_visibility = f64::NEG_INFINITY;
        let mut seg_idx = 0usize;

        for sd in sample_dists {
            while seg_idx + 1 < cumulative.len() && cumulative[seg_idx + 1] < sd {
                seg_idx += 1;
            }
            if seg_idx + 1 >= cumulative.len() {
                break;
            }

            let d0 = cumulative[seg_idx];
            let d1 = cumulative[seg_idx + 1];
            let frac = if (d1 - d0).abs() <= f64::EPSILON {
                0.0
            } else {
                (sd - d0) / (d1 - d0)
            };

            let x = segment[seg_idx].x + frac * (segment[seg_idx + 1].x - segment[seg_idx].x);
            let y = segment[seg_idx].y + frac * (segment[seg_idx + 1].y - segment[seg_idx].y);
            let row_f = (dem.y_max() - y) / dem.cell_size_y - 0.5;
            let col_f = (x - dem.x_min) / dem.cell_size_x - 0.5;

            if row_f <= 0.0
                || col_f <= 0.0
                || row_f >= (dem.rows - 1) as f64
                || col_f >= (dem.cols - 1) as f64
            {
                continue;
            }

            if let Some(z) = Self::bilinear_sample(dem, 0, row_f, col_f) {
                samples.push((sd, z, x, y));
                let row = row_f.round() as isize;
                let col = col_f.round() as isize;
                if let Some(v) = Self::visibility_openness(dem, row, col, search_radius) {
                    if v > max_visibility {
                        max_visibility = v;
                    }
                }
            }
        }

        if samples.len() < 2 {
            return None;
        }

        let mut min_elev = f64::INFINITY;
        let mut max_elev = f64::NEG_INFINITY;
        for (_, z, _, _) in &samples {
            min_elev = min_elev.min(*z);
            max_elev = max_elev.max(*z);
        }

        let mut slope_dist = 0.0;
        for i in 1..samples.len() {
            slope_dist += samples[i].0 - samples[i - 1].0;
        }
        if slope_dist <= f64::EPSILON {
            return None;
        }

        let mut avg_slope = 0.0;
        let mut dist_3d = 0.0;
        let mut chg_in_slp = 0.0;
        for i in 1..samples.len() {
            let run = samples[i].0 - samples[i - 1].0;
            if run <= f64::EPSILON {
                continue;
            }
            let rise = samples[i].1 - samples[i - 1].1;
            let slope = (rise.abs() / run).atan().to_degrees();
            avg_slope += (run / slope_dist) * slope;
            dist_3d += (run * run + rise * rise).sqrt();

            if i < samples.len() - 1 {
                let z = samples[i].1;
                if (z > samples[i - 1].1 && z > samples[i + 1].1)
                    || (z < samples[i - 1].1 && z < samples[i + 1].1)
                {
                    chg_in_slp += 1.0;
                }
            }
        }

        let dx = samples[0].2 - samples[samples.len() - 1].2;
        let dy = samples[0].3 - samples[samples.len() - 1].3;
        let dz = samples[0].1 - samples[samples.len() - 1].1;
        let end_dist_3d = (dx * dx + dy * dy + dz * dz).sqrt();
        let sinuosity = if end_dist_3d > f64::EPSILON {
            dist_3d / end_dist_3d
        } else {
            1.0
        };

        Some(AssessRouteMetrics {
            avg_slope,
            min_elev,
            max_elev,
            relief: max_elev - min_elev,
            sinuosity,
            chg_in_slp: chg_in_slp / samples.len() as f64 * 100.0,
            visibility: if max_visibility.is_finite() {
                Some(max_visibility)
            } else {
                None
            },
        })
    }

    fn run_assess_route(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let routes_path = parse_vector_path_arg(args, "routes")
            .or_else(|_| parse_vector_path_arg(args, "input"))?;
        let dem_path = parse_raster_path_arg(args, "dem")
            .or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
        let output_path = parse_optional_output_path(args, "output")?;
        let segment_length = args
            .get("segment_length")
            .or_else(|| args.get("length"))
            .and_then(|v| v.as_f64())
            .unwrap_or(100.0);
        if !segment_length.is_finite() || segment_length <= 0.0 {
            return Err(ToolError::Validation(
                "parameter 'segment_length' must be a positive finite number".to_string(),
            ));
        }

        let search_radius = args
            .get("search_radius")
            .or_else(|| args.get("dist"))
            .and_then(|v| v.as_u64())
            .unwrap_or(15)
            .max(4) as isize;

        ctx.progress.info("running assess_route");
        let routes = wbvector::read(&routes_path).map_err(|e| {
            ToolError::Execution(format!("failed reading routes vector '{}': {}", routes_path, e))
        })?;
        let dem = Self::load_raster(&dem_path)?;

        if Self::raster_is_geographic(&dem) {
            return Err(ToolError::Validation(
                "assess_route requires a projected DEM".to_string(),
            ));
        }

        let mut output = wbvector::Layer::new("assess_route")
            .with_geom_type(wbvector::GeometryType::LineString);
        if let Some(crs) = routes.crs.clone() {
            output = output.with_crs(crs);
        }

        output.add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));
        output.add_field(wbvector::FieldDef::new("PARENT_ID", wbvector::FieldType::Integer));
        output.add_field(wbvector::FieldDef::new("AVG_SLOPE", wbvector::FieldType::Float));
        output.add_field(wbvector::FieldDef::new("MIN_ELEV", wbvector::FieldType::Float));
        output.add_field(wbvector::FieldDef::new("MAX_ELEV", wbvector::FieldType::Float));
        output.add_field(wbvector::FieldDef::new("RELIEF", wbvector::FieldType::Float));
        output.add_field(wbvector::FieldDef::new("SINUOSITY", wbvector::FieldType::Float));
        output.add_field(wbvector::FieldDef::new("CHG_IN_SLP", wbvector::FieldType::Float));
        output.add_field(wbvector::FieldDef::new("VISIBILITY", wbvector::FieldType::Float));

        let mut copied_parent_fields: Vec<(usize, String)> = Vec::new();
        for (i, def) in routes.schema.fields().iter().enumerate() {
            if def.name.eq_ignore_ascii_case("FID") {
                continue;
            }
            output.add_field(def.clone());
            copied_parent_fields.push((i, def.name.clone()));
        }

        let mut out_fid = 1i64;
        for (feature_idx, feature) in routes.features.iter().enumerate() {
            if let Some(geom) = &feature.geometry {
                let mut parts: Vec<Vec<wbvector::Coord>> = Vec::new();
                Self::collect_route_parts(geom, &mut parts);
                for part in parts {
                    let segments = Self::split_route_part_by_length(&part, segment_length);
                    for segment in segments {
                        let metrics = Self::compute_assess_route_metrics(&dem, &segment, search_radius);

                        let mut out_feature = wbvector::Feature::with_geometry(
                            output.features.len() as u64,
                            wbvector::Geometry::line_string(segment),
                            output.schema.len(),
                        );
                        out_feature
                            .set(&output.schema, "FID", wbvector::FieldValue::Integer(out_fid))
                            .map_err(|e| {
                                ToolError::Execution(format!(
                                    "failed assigning output attribute FID: {}",
                                    e
                                ))
                            })?;
                        out_feature
                            .set(
                                &output.schema,
                                "PARENT_ID",
                                wbvector::FieldValue::Integer(feature_idx as i64 + 1),
                            )
                            .map_err(|e| {
                                ToolError::Execution(format!(
                                    "failed assigning output attribute PARENT_ID: {}",
                                    e
                                ))
                            })?;

                        if let Some(m) = metrics {
                            out_feature
                                .set(
                                    &output.schema,
                                    "AVG_SLOPE",
                                    wbvector::FieldValue::Float(m.avg_slope),
                                )
                                .map_err(|e| {
                                    ToolError::Execution(format!(
                                        "failed assigning output attribute AVG_SLOPE: {}",
                                        e
                                    ))
                                })?;
                            out_feature
                                .set(
                                    &output.schema,
                                    "MIN_ELEV",
                                    wbvector::FieldValue::Float(m.min_elev),
                                )
                                .map_err(|e| {
                                    ToolError::Execution(format!(
                                        "failed assigning output attribute MIN_ELEV: {}",
                                        e
                                    ))
                                })?;
                            out_feature
                                .set(
                                    &output.schema,
                                    "MAX_ELEV",
                                    wbvector::FieldValue::Float(m.max_elev),
                                )
                                .map_err(|e| {
                                    ToolError::Execution(format!(
                                        "failed assigning output attribute MAX_ELEV: {}",
                                        e
                                    ))
                                })?;
                            out_feature
                                .set(
                                    &output.schema,
                                    "RELIEF",
                                    wbvector::FieldValue::Float(m.relief),
                                )
                                .map_err(|e| {
                                    ToolError::Execution(format!(
                                        "failed assigning output attribute RELIEF: {}",
                                        e
                                    ))
                                })?;
                            out_feature
                                .set(
                                    &output.schema,
                                    "SINUOSITY",
                                    wbvector::FieldValue::Float(m.sinuosity),
                                )
                                .map_err(|e| {
                                    ToolError::Execution(format!(
                                        "failed assigning output attribute SINUOSITY: {}",
                                        e
                                    ))
                                })?;
                            out_feature
                                .set(
                                    &output.schema,
                                    "CHG_IN_SLP",
                                    wbvector::FieldValue::Float(m.chg_in_slp),
                                )
                                .map_err(|e| {
                                    ToolError::Execution(format!(
                                        "failed assigning output attribute CHG_IN_SLP: {}",
                                        e
                                    ))
                                })?;
                            out_feature
                                .set(
                                    &output.schema,
                                    "VISIBILITY",
                                    if let Some(v) = m.visibility {
                                        wbvector::FieldValue::Float(v)
                                    } else {
                                        wbvector::FieldValue::Null
                                    },
                                )
                                .map_err(|e| {
                                    ToolError::Execution(format!(
                                        "failed assigning output attribute VISIBILITY: {}",
                                        e
                                    ))
                                })?;
                        }

                        for (src_idx, field_name) in &copied_parent_fields {
                            let value = feature
                                .attributes
                                .get(*src_idx)
                                .cloned()
                                .unwrap_or(wbvector::FieldValue::Null);
                            out_feature
                                .set(&output.schema, field_name, value)
                                .map_err(|e| {
                                    ToolError::Execution(format!(
                                        "failed assigning copied parent attribute '{}': {}",
                                        field_name, e
                                    ))
                                })?;
                        }

                        output.push(out_feature);
                        out_fid += 1;
                    }
                }
            }

            ctx.progress
                .progress((feature_idx + 1) as f64 / routes.features.len().max(1) as f64);
        }

        let out = Self::write_vector_output(&output, output_path, "assessed_routes.shp")?;
        Ok(Self::build_result(out))
    }

    fn breakline_mapping_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "breakline_mapping",
            display_name: "Breakline Mapping",
            summary: "Maps breaklines by thresholding log-transformed curvedness and vectorizing thinned linear features.",
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Pro,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input DEM raster path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "threshold",
                    description: "Minimum log-curvedness threshold used for breakline extraction (default 0.8).",
                    required: false,
                },
                ToolParamSpec {
                    name: "min_length",
                    description: "Minimum output line length in grid cells (default 3).",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output vector path (default temporary .shp).",
                    required: false,
                },
            ],
        }
    }

    fn breakline_mapping_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("threshold".to_string(), json!(0.8));
        defaults.insert("min_length".to_string(), json!(3));
        defaults.insert("output".to_string(), json!("breaklines.shp"));

        let mut ex_args = ToolArgs::new();
        ex_args.insert("input".to_string(), json!("dem.tif"));
        ex_args.insert("threshold".to_string(), json!(2.0));
        ex_args.insert("min_length".to_string(), json!(6));
        ex_args.insert("output".to_string(), json!("breaklines.shp"));

        ToolManifest {
            id: "breakline_mapping".to_string(),
            display_name: "Breakline Mapping".to_string(),
            summary: "Maps breaklines by thresholding log-transformed curvedness and vectorizing thinned linear features.".to_string(),
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Pro,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input DEM raster path or typed raster object.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "threshold".to_string(),
                    description: "Minimum log-curvedness threshold used for breakline extraction (default 0.8).".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "min_length".to_string(),
                    description: "Minimum output line length in grid cells (default 3).".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output vector path (default temporary .shp).".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_breakline_mapping".to_string(),
                description: "Extract breakline vectors from a DEM.".to_string(),
                args: ex_args,
            }],
            tags: vec![
                "geomorphometry".to_string(),
                "breaklines".to_string(),
                "curvature".to_string(),
                "vectorization".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn run_breakline_mapping(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let threshold = args
            .get("threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.8);
        let min_length = args
            .get("min_length")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(3)
            .max(2);

        let input = Self::load_raster(&input_path)?;
        let rows = input.rows;
        let cols = input.cols;
        let nodata = input.nodata;
        let band = 0isize;

        let mut resx = input.cell_size_x.abs();
        let mut resy = input.cell_size_y.abs();
        if Self::raster_is_geographic(&input) {
            let mid_lat = ((input.y_min + input.y_max()) * 0.5).to_radians();
            let m_per_deg = 111_320.0;
            resx *= m_per_deg * mid_lat.cos().abs().max(1.0e-8);
            resy *= m_per_deg;
        }
        let res = ((resx + resy) * 0.5).max(1.0e-9);
        let log_multiplier = if res < 1.0 {
            10f64.powi(2)
        } else if res < 10.0 {
            10f64.powi(3)
        } else if res < 100.0 {
            10f64.powi(4)
        } else if res < 1000.0 {
            10f64.powi(5)
        } else if res < 5000.0 {
            10f64.powi(6)
        } else if res < 10000.0 {
            10f64.powi(7)
        } else if res < 75000.0 {
            10f64.powi(8)
        } else {
            10f64.powi(9)
        };

        // Stage 1: curvedness estimate and log transform.
        let curv: Vec<Vec<f64>> = (0..rows)
            .into_par_iter()
            .map(|r| {
                let mut row_out = vec![0.0; cols];
                for c in 0..cols {
                    let zc = input.get(band, r as isize, c as isize);
                    if input.is_nodata(zc) {
                        row_out[c] = nodata;
                        continue;
                    }
                    let z = |dr: isize, dc: isize| {
                        let rr = r as isize + dr;
                        let cc = c as isize + dc;
                        if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                            return zc;
                        }
                        let v = input.get(band, rr, cc);
                        if input.is_nodata(v) { zc } else { v }
                    };

                    let z1 = z(-1, -1);
                    let z2 = z(-1, 0);
                    let z3 = z(-1, 1);
                    let z4 = z(0, -1);
                    let z6 = z(0, 1);
                    let z7 = z(1, -1);
                    let z8 = z(1, 0);
                    let z9 = z(1, 1);

                    let p = ((z3 + z6 + z9) - (z1 + z4 + z7)) / (6.0 * resx);
                    let q = ((z7 + z8 + z9) - (z1 + z2 + z3)) / (6.0 * resy);
                    let r2 = (z1 - 2.0 * z2 + z3 + z4 - 2.0 * zc + z6 + z7 - 2.0 * z8 + z9)
                        / (3.0 * resx * resx);
                    let t2 = (z1 - 2.0 * z4 + z7 + z2 - 2.0 * zc + z8 + z3 - 2.0 * z6 + z9)
                        / (3.0 * resy * resy);
                    let s2 = (z1 - z3 - z7 + z9) / (4.0 * resx * resy);

                    let d = 1.0 + p * p + q * q;
                    let mean_curv = -((1.0 + q * q) * r2 - 2.0 * p * q * s2 + (1.0 + p * p) * t2)
                        / (2.0 * d.powf(1.5));
                    let gauss_curv = (r2 * t2 - s2 * s2) / (d * d);
                    let discr = (mean_curv * mean_curv - gauss_curv).max(0.0).sqrt();
                    let kmin = mean_curv - discr;
                    let kmax = mean_curv + discr;
                    let curvedness = ((kmin * kmin + kmax * kmax) * 0.5).sqrt();
                    row_out[c] = (1.0 + log_multiplier * curvedness).ln();
                }
                row_out
            })
            .collect();

        let dx = [1isize, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1isize, 0, 1, 1, 1, 0, -1, -1];
        let compute_progress = PercentCoalescer::new(1, 99);

        // Stage 2: threshold and directional local-min suppression.
        let mut thinned = vec![vec![0.0; cols]; rows];
        for r in 0..rows {
            for c in 0..cols {
                let z = curv[r][c];
                if z == nodata || !z.is_finite() || z < threshold {
                    continue;
                }

                let n = |i: usize| -> f64 {
                    let rr = r as isize + dy[i];
                    let cc = c as isize + dx[i];
                    if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                        return z;
                    }
                    curv[rr as usize][cc as usize]
                };
                let n0 = n(0);
                let n1 = n(1);
                let n2 = n(2);
                let n3 = n(3);
                let n4 = n(4);
                let n5 = n(5);
                let n6 = n(6);
                let n7 = n(7);

                let suppress = (z < n0 && z < n1 && z < n2)
                    || (z < n1 && z < n3)
                    || (z < n2 && z < n3 && z < n4)
                    || (z < n3 && z < n5)
                    || (z < n4 && z < n5 && z < n6)
                    || (z < n5 && z < n7)
                    || (z < n6 && z < n7 && z < n0)
                    || (z < n7 && z < n1)
                    || (z < n0 && z < n1 && z < n2 && z < n3 && z < n4 && z < n5 && z < n6 && z < n7);
                if !suppress {
                    thinned[r][c] = z;
                }
            }
            compute_progress.emit_unit_fraction(ctx.progress, (r + 1) as f64 / (rows as f64 * 4.0));
        }

        // Remove singleton and simple elbow cells.
        for r in 0..rows {
            for c in 0..cols {
                if thinned[r][c] <= 0.0 {
                    continue;
                }
                let mut present: Vec<usize> = Vec::new();
                for i in 0..8 {
                    let rr = r as isize + dy[i];
                    let cc = c as isize + dx[i];
                    if rr >= 0
                        && cc >= 0
                        && rr < rows as isize
                        && cc < cols as isize
                        && thinned[rr as usize][cc as usize] > 0.0
                    {
                        present.push(i);
                    }
                }
                if present.is_empty() {
                    thinned[r][c] = 0.0;
                } else if present.len() == 2 {
                    let mut d = present[1] as isize - present[0] as isize;
                    if d == 7 {
                        d = 1;
                    }
                    if d == 1 {
                        thinned[r][c] = 0.0;
                    }
                }
            }
        }

        // Remove diagonal bridge cells.
        for r in 0..rows {
            for c in 0..cols {
                if thinned[r][c] <= 0.0 {
                    continue;
                }
                let getp = |i: usize| -> bool {
                    let rr = r as isize + dy[i];
                    let cc = c as isize + dx[i];
                    rr >= 0
                        && cc >= 0
                        && rr < rows as isize
                        && cc < cols as isize
                        && thinned[rr as usize][cc as usize] > 0.0
                };
                if (getp(7) && getp(1))
                    || (getp(1) && getp(3))
                    || (getp(3) && getp(5))
                    || (getp(5) && getp(7))
                {
                    thinned[r][c] = 0.0;
                }
            }
            ctx.progress
                .progress((rows + r + 1) as f64 / (rows as f64 * 4.0));
        }

        // Stage 3: label connected line components, preferring endpoint starts.
        let mut line_id = vec![0i32; rows * cols];
        let mut feature_cells: Vec<Vec<(usize, usize)>> = vec![Vec::new()];
        let mut next_id = 1i32;

        let degree = |r: usize, c: usize, grid: &Vec<Vec<f64>>| -> usize {
            let mut n = 0usize;
            for i in 0..8 {
                let rr = r as isize + dy[i];
                let cc = c as isize + dx[i];
                if rr >= 0
                    && cc >= 0
                    && rr < rows as isize
                    && cc < cols as isize
                    && grid[rr as usize][cc as usize] > 0.0
                {
                    n += 1;
                }
            }
            n
        };

        let mut seed_cells: Vec<(usize, usize)> = Vec::new();
        for r in 0..rows {
            for c in 0..cols {
                if thinned[r][c] > 0.0 && degree(r, c, &thinned) == 1 {
                    seed_cells.push((r, c));
                }
            }
        }
        for r in 0..rows {
            for c in 0..cols {
                if thinned[r][c] > 0.0 {
                    seed_cells.push((r, c));
                }
            }
        }

        for (sr, sc) in seed_cells {
            let si = Self::idx(sr, sc, cols);
            if thinned[sr][sc] <= 0.0 || line_id[si] != 0 {
                continue;
            }
            let mut comp: Vec<(usize, usize)> = Vec::new();
            let mut stack = vec![(sr, sc)];
            line_id[si] = next_id;
            while let Some((r, c)) = stack.pop() {
                comp.push((r, c));
                for i in 0..8 {
                    let rr = r as isize + dy[i];
                    let cc = c as isize + dx[i];
                    if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                        continue;
                    }
                    let rru = rr as usize;
                    let ccu = cc as usize;
                    if thinned[rru][ccu] <= 0.0 {
                        continue;
                    }
                    let ni = Self::idx(rru, ccu, cols);
                    if line_id[ni] == 0 {
                        line_id[ni] = next_id;
                        stack.push((rru, ccu));
                    }
                }
            }
            feature_cells.push(comp);
            next_id += 1;
        }

        // Stage 4: vectorize each sufficiently long component.
        let mut layer = wbvector::Layer::new("breaklines").with_geom_type(wbvector::GeometryType::LineString);
        layer.add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));
        layer.add_field(wbvector::FieldDef::new("AVG_CURV", wbvector::FieldType::Float));
        layer.add_field(wbvector::FieldDef::new("LENGTH", wbvector::FieldType::Float));

        let mut out_fid = 1i64;
        for comp in feature_cells.iter().skip(1) {
            if comp.len() < min_length {
                continue;
            }

            let comp_set: std::collections::HashSet<(usize, usize)> = comp.iter().copied().collect();
            let mut start = comp[0];
            for &(r, c) in comp {
                let mut deg = 0usize;
                for i in 0..8 {
                    let rr = r as isize + dy[i];
                    let cc = c as isize + dx[i];
                    if rr >= 0
                        && cc >= 0
                        && rr < rows as isize
                        && cc < cols as isize
                        && comp_set.contains(&(rr as usize, cc as usize))
                    {
                        deg += 1;
                    }
                }
                if deg == 1 {
                    start = (r, c);
                    break;
                }
            }

            let mut ordered: Vec<(usize, usize)> = Vec::with_capacity(comp.len());
            let mut used: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
            let mut cur = start;
            let mut prev: Option<(usize, usize)> = None;
            loop {
                ordered.push(cur);
                used.insert(cur);
                let mut next_candidates: Vec<(usize, usize)> = Vec::new();
                for i in 0..8 {
                    let rr = cur.0 as isize + dy[i];
                    let cc = cur.1 as isize + dx[i];
                    if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                        continue;
                    }
                    let ncell = (rr as usize, cc as usize);
                    if !comp_set.contains(&ncell) || used.contains(&ncell) {
                        continue;
                    }
                    if prev.is_some() && prev.unwrap() == ncell {
                        continue;
                    }
                    next_candidates.push(ncell);
                }
                if next_candidates.is_empty() {
                    break;
                }
                next_candidates.sort_by(|a, b| {
                    let za = thinned[a.0][a.1];
                    let zb = thinned[b.0][b.1];
                    zb.partial_cmp(&za).unwrap_or(std::cmp::Ordering::Equal)
                });
                prev = Some(cur);
                cur = next_candidates[0];
            }

            if ordered.len() < min_length {
                continue;
            }

            let mut coords: Vec<wbvector::Coord> = ordered
                .iter()
                .map(|(r, c)| wbvector::Coord::xy(input.col_center_x(*c as isize), input.row_center_y(*r as isize)))
                .collect();

            if coords.len() > 4 {
                let mut smooth = coords.clone();
                for i in 1..coords.len() - 1 {
                    smooth[i].x = (coords[i - 1].x + coords[i].x + coords[i + 1].x) / 3.0;
                    smooth[i].y = (coords[i - 1].y + coords[i].y + coords[i + 1].y) / 3.0;
                }
                coords = smooth;
            }

            let mut length = 0.0;
            for i in 1..coords.len() {
                let dx = coords[i].x - coords[i - 1].x;
                let dy = coords[i].y - coords[i - 1].y;
                length += (dx * dx + dy * dy).sqrt();
            }
            let avg_curv = ordered
                .iter()
                .map(|(r, c)| thinned[*r][*c])
                .sum::<f64>()
                / ordered.len() as f64;

            layer
                .add_feature(
                    Some(wbvector::Geometry::line_string(coords)),
                    &[
                        ("FID", wbvector::FieldValue::Integer(out_fid)),
                        ("AVG_CURV", wbvector::FieldValue::Float(avg_curv)),
                        ("LENGTH", wbvector::FieldValue::Float(length)),
                    ],
                )
                .map_err(|e| ToolError::Execution(format!("failed building output feature: {}", e)))?;
            out_fid += 1;
        }

        ctx.progress.progress(1.0);
        let out = Self::write_vector_output(&layer, output_path, "breaklines.shp")?;
        Ok(Self::build_result(out))
    }

    fn pennock_landform_classification_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "pennock_landform_classification",
            display_name: "Pennock Landform Classification",
            summary: "Classifies landform elements into seven Pennock et al. (1987) terrain classes.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "slope_threshold", description: "Slope threshold in degrees (default 3.0).", required: false },
                ToolParamSpec { name: "prof_curv_threshold", description: "Profile curvature threshold in degrees (default 0.1).", required: false },
                ToolParamSpec { name: "plan_curv_threshold", description: "Plan curvature threshold in degrees (default 0.0).", required: false },
                ToolParamSpec { name: "z_factor", description: "Optional vertical scaling factor (if < 0 and CRS is geographic, estimated from mid-latitude).", required: false },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn pennock_landform_classification_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("slope_threshold".to_string(), json!(3.0));
        defaults.insert("prof_curv_threshold".to_string(), json!(0.1));
        defaults.insert("plan_curv_threshold".to_string(), json!(0.0));
        defaults.insert("z_factor".to_string(), json!(1.0));
        ToolManifest {
            id: "pennock_landform_classification".to_string(),
            display_name: "Pennock Landform Classification".to_string(),
            summary: "Classifies landform elements into seven Pennock et al. (1987) terrain classes.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "classification".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_pennock_landform_classification(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        const CLASSIFICATION_KEY: &str = "CLASSIFICATION KEY\nValue  Class\n1      Convergent Footslope\n2      Divergent Footslope\n3      Convergent Shoulder\n4      Divergent Shoulder\n5      Convergent Backslope\n6      Divergent Backslope\n7      Level\n";

        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let slope_threshold = args
            .get("slope_threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(3.0);
        let prof_threshold = args
            .get("prof_curv_threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.1);
        let plan_threshold = args
            .get("plan_curv_threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let mut z_factor = args
            .get("z_factor")
            .or_else(|| args.get("zfactor"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let input = Self::load_raster(&input_path)?;
        if Self::raster_is_geographic(&input) && z_factor < 0.0 {
            let mid_lat = input.y_min + (input.rows as f64 * input.cell_size_y) * 0.5;
            if (-90.0..=90.0).contains(&mid_lat) {
                z_factor = 1.0 / (111_320.0 * mid_lat.to_radians().cos().abs().max(1e-8));
            }
        }

        let output_cfg = RasterConfig {
            cols: input.cols,
            rows: input.rows,
            bands: 1,
            x_min: input.x_min,
            y_min: input.y_min,
            cell_size: input.cell_size_x,
            cell_size_y: Some(input.cell_size_y),
            nodata: -128.0,
            data_type: DataType::I8,
            crs: input.crs.clone(),
            metadata: Vec::new(),
        };
        let mut output = Raster::new(output_cfg);

        let rows = input.rows;
        let cols = input.cols;
        let out_nodata = output.nodata;
        let cell_size = input.cell_size_x.abs().max(f64::EPSILON);
        let cell_size_times2 = cell_size * 2.0;
        let cell_size_sqrd = cell_size * cell_size;
        let four_times_cell_size_sqrd = cell_size_sqrd * 4.0;
        let eight_grid_res = cell_size * 8.0;
        let dx = [1isize, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1isize, 0, 1, 1, 1, 0, -1, -1];
        let band = 0isize;

        ctx.progress.info("running pennock_landform_classification");

        let row_data: Vec<Vec<f64>> = (0..rows)
            .into_par_iter()
            .map(|r| {
                let mut row_out = vec![out_nodata; cols];
                for c in 0..cols {
                    let mut z = input.get(band, r as isize, c as isize);
                    if input.is_nodata(z) {
                        continue;
                    }

                    z *= z_factor;
                    let mut n = [0.0_f64; 8];
                    for i in 0..8 {
                        let v = input.get(band, r as isize + dy[i], c as isize + dx[i]);
                        n[i] = if input.is_nodata(v) { z } else { v * z_factor };
                    }

                    let zx = (n[1] - n[5]) / cell_size_times2;
                    let zy = (n[7] - n[3]) / cell_size_times2;
                    let zxx = (n[1] - 2.0 * z + n[5]) / cell_size_sqrd;
                    let zyy = (n[7] - 2.0 * z + n[3]) / cell_size_sqrd;
                    let zxy = (-n[6] + n[0] + n[4] - n[2]) / four_times_cell_size_sqrd;

                    let zx2 = zx * zx;
                    let zy2 = zy * zy;
                    let p = zx2 + zy2;
                    if p <= 0.0 {
                        continue;
                    }
                    let q = p + 1.0;

                    let fy = (n[6] - n[4] + 2.0 * (n[7] - n[3]) + n[0] - n[2]) / eight_grid_res;
                    let fx = (n[2] - n[4] + 2.0 * (n[1] - n[5]) + n[0] - n[6]) / eight_grid_res;
                    let slope = (fx * fx + fy * fy).sqrt().atan().to_degrees();
                    let denom = p * q.powf(1.5);
                    let plan = -((zxx * zy2 - 2.0 * zxy * zx * zy + zyy * zx2) / denom).to_degrees();
                    let prof = -((zxx * zx2 - 2.0 * zxy * zx * zy + zyy * zy2) / denom).to_degrees();

                    row_out[c] = if prof < -prof_threshold && plan <= -plan_threshold && slope > slope_threshold {
                        1.0
                    } else if prof < -prof_threshold && plan > plan_threshold && slope > slope_threshold {
                        2.0
                    } else if prof > prof_threshold && plan <= plan_threshold && slope > slope_threshold {
                        3.0
                    } else if prof > prof_threshold && plan > plan_threshold && slope > slope_threshold {
                        4.0
                    } else if prof >= -prof_threshold
                        && prof < prof_threshold
                        && slope > slope_threshold
                        && plan <= -plan_threshold
                    {
                        5.0
                    } else if prof >= -prof_threshold
                        && prof < prof_threshold
                        && slope > slope_threshold
                        && plan > plan_threshold
                    {
                        6.0
                    } else if slope <= slope_threshold {
                        7.0
                    } else {
                        out_nodata
                    };
                }
                row_out
            })
            .collect();

        for (r, row) in row_data.iter().enumerate() {
            output
                .set_row_slice(0, r as isize, row)
                .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
        }
        ctx.progress.progress(1.0);

        let mut outputs = std::collections::BTreeMap::new();
        outputs.insert(
            "path".to_string(),
            json!(Self::write_or_store_output(output, output_path)?),
        );
        outputs.insert("classification_key".to_string(), json!(CLASSIFICATION_KEY));
        Ok(ToolRunResult {
            outputs,
            ..Default::default()
        })
    }

    fn geomorphons_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "geomorphons",
            display_name: "Geomorphons",
            summary: "Classifies landforms using 8-direction line-of-sight ternary patterns based on zenith-nadir angle differences, or 10 common geomorphon forms. Authored by Dan Newman and John Lindsay.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "search_distance", description: "Maximum look-up distance in cells per direction (default 50); the endpoint cell is included.", required: false },
                ToolParamSpec { name: "flatness_threshold", description: "Flatness threshold in degrees (default 0.0), applied to the absolute zenith-nadir angle difference.", required: false },
                ToolParamSpec { name: "flatness_distance", description: "Distance in cells after which the flatness threshold tapers with horizon distance (default 0).", required: false },
                ToolParamSpec { name: "skip_distance", description: "Distance in cells to skip before evaluating line-of-sight (default 0).", required: false },
                ToolParamSpec { name: "output_forms", description: "If true, outputs 10 common landform classes; otherwise outputs canonical geomorphon ternary codes.", required: false },
                ToolParamSpec { name: "analyze_residuals", description: "If true, detrends the DEM using a fitted linear plane before classification.", required: false },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn geomorphons_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("search_distance".to_string(), json!(50));
        defaults.insert("flatness_threshold".to_string(), json!(0.0));
        defaults.insert("flatness_distance".to_string(), json!(0));
        defaults.insert("skip_distance".to_string(), json!(0));
        defaults.insert("output_forms".to_string(), json!(true));
        defaults.insert("analyze_residuals".to_string(), json!(false));
        ToolManifest {
            id: "geomorphons".to_string(),
            display_name: "Geomorphons".to_string(),
            summary: "Classifies landforms using 8-direction line-of-sight ternary patterns based on zenith-nadir angle differences, or 10 common geomorphon forms. Authored by Dan Newman and John Lindsay.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "classification".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_geomorphons(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let mut search_distance = args
            .get("search_distance")
            .or_else(|| args.get("search"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(50)
            .max(1);
        let flatness_threshold = args
            .get("flatness_threshold")
            .or_else(|| args.get("threshold"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
            .max(0.0)
            .to_radians();
        let mut flatness_distance = args
            .get("flatness_distance")
            .or_else(|| args.get("fdist"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(0);
        let skip_distance = args
            .get("skip_distance")
            .or_else(|| args.get("skip"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(0);
        let output_forms = args
            .get("output_forms")
            .or_else(|| args.get("forms"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let analyze_residuals = args
            .get("analyze_residuals")
            .or_else(|| args.get("residuals"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if search_distance < 1 {
            search_distance = 1;
        }
        if (flatness_distance > 0 && flatness_distance <= skip_distance)
            || flatness_distance >= search_distance
        {
            flatness_distance = 0;
        }

        let source = std::sync::Arc::new(Self::load_raster(&input_path)?);
        let analysis_input = if analyze_residuals {
            ctx.progress.info("calculating residuals");
            std::sync::Arc::new(Self::detrend_raster_to_residuals(&source))
        } else {
            source.clone()
        };

        let output_cfg = RasterConfig {
            cols: source.cols,
            rows: source.rows,
            bands: 1,
            x_min: source.x_min,
            y_min: source.y_min,
            cell_size: source.cell_size_x,
            cell_size_y: Some(source.cell_size_y),
            nodata: i16::MIN as f64,
            data_type: DataType::I16,
            crs: source.crs.clone(),
            metadata: Vec::new(),
        };
        let mut output = Raster::new(output_cfg);

        let rows = analysis_input.rows;
        let cols = analysis_input.cols;
        let band = 0isize;
        let out_nodata = output.nodata;
        let search_distance_i = search_distance as isize;
        let cell_size_x = source.cell_size_x.abs();
        let cell_size_y = source.cell_size_y.abs();
        let flatness_distance_f64 = flatness_distance as f64;
        let flatness_threshold_tan = flatness_threshold.tan();
        let skip = (skip_distance + 1) as isize;
        let half_pi = std::f64::consts::FRAC_PI_2;
        let gtc = std::sync::Arc::new(Self::build_geomorphon_lookup());

        const CLASS_MAP: [[u8; 9]; 9] = [
            [1, 1, 1, 8, 8, 9, 9, 9, 10],
            [1, 1, 8, 8, 8, 9, 9, 9, 0],
            [1, 4, 6, 6, 7, 7, 9, 0, 0],
            [4, 4, 6, 6, 6, 7, 0, 0, 0],
            [4, 4, 5, 6, 6, 0, 0, 0, 0],
            [3, 3, 5, 5, 0, 0, 0, 0, 0],
            [3, 3, 3, 0, 0, 0, 0, 0, 0],
            [3, 3, 0, 0, 0, 0, 0, 0, 0],
            [2, 0, 0, 0, 0, 0, 0, 0, 0],
        ];
        const DX: [isize; 8] = [0, 1, 1, 1, 0, -1, -1, -1];
        const DY: [isize; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];

        let mut step_lengths = [0.0_f64; 8];
        let mut flat_threshold_heights = [0.0_f64; 8];
        for dir in 0..8usize {
            let step_length = ((DX[dir] as f64 * cell_size_x).powi(2)
                + (DY[dir] as f64 * cell_size_y).powi(2))
                .sqrt();
            step_lengths[dir] = step_length;
            flat_threshold_heights[dir] = flatness_threshold_tan * flatness_distance_f64 * step_length;
        }

        let mut inv_distances = vec![[0.0_f64; 8]; search_distance + 1];
        for step in 1..=search_distance {
            let step_f = step as f64;
            for dir in 0..8usize {
                inv_distances[step][dir] = 1.0 / (step_f * step_lengths[dir]);
            }
        }

        ctx.progress.info("running geomorphons");

        let row_data: Vec<Vec<f64>> = (0..rows)
            .into_par_iter()
            .map(|row| {
                let mut row_out = vec![out_nodata; cols];
                let row_i = row as isize;
                let rows_less_one = rows as isize - 1;
                let cols_less_one = cols as isize - 1;

                for col in 0..cols {
                    let col_i = col as isize;
                    if row_i < skip
                        || row_i > rows_less_one - skip
                        || col_i < skip
                        || col_i > cols_less_one - skip
                    {
                        continue;
                    }

                    let z = analysis_input.get(band, row_i, col_i);
                    if analysis_input.is_nodata(z) {
                        continue;
                    }

                    let mut count_pos = 0usize;
                    let mut count_neg = 0usize;
                    let mut pattern = [1usize; 8];

                    'directions: for dir in 0..8usize {
                        let mut zenith_slope = f64::NEG_INFINITY;
                        let mut nadir_slope = f64::INFINITY;
                        let mut zenith_step = 0usize;
                        let mut nadir_step = 0usize;
                        let mut d = skip;
                        let mut r = row_i + d * DY[dir];
                        let mut c = col_i + d * DX[dir];
                        if r < 0 || r > rows_less_one || c < 0 || c > cols_less_one {
                            continue 'directions;
                        }

                        while d <= search_distance_i {
                            let z2 = analysis_input.get(band, r, c);
                            if !analysis_input.is_nodata(z2) {
                                let slope = (z2 - z) * inv_distances[d as usize][dir];
                                if slope > zenith_slope {
                                    zenith_slope = slope;
                                    zenith_step = d as usize;
                                }
                                if slope < nadir_slope {
                                    nadir_slope = slope;
                                    nadir_step = d as usize;
                                }
                            }

                            d += 1;
                            if d > search_distance_i {
                                break;
                            }
                            r = row_i + d * DY[dir];
                            c = col_i + d * DX[dir];
                            if r < 0 || r > rows_less_one || c < 0 || c > cols_less_one {
                                continue 'directions;
                            }
                        }

                        let zenith_distance = zenith_step as f64 * step_lengths[dir];
                        let nadir_distance = nadir_step as f64 * step_lengths[dir];
                        let zenith_threshold = if flatness_distance > 0
                            && zenith_step > flatness_distance
                        {
                            flat_threshold_heights[dir].atan2(zenith_distance)
                        } else {
                            flatness_threshold
                        };
                        let nadir_threshold = if flatness_distance > 0
                            && nadir_step > flatness_distance
                        {
                            flat_threshold_heights[dir].atan2(nadir_distance)
                        } else {
                            flatness_threshold
                        };
                        let zenith_angle = if zenith_step > 0 {
                            zenith_slope.atan()
                        } else {
                            -half_pi
                        };
                        let nadir_angle = if nadir_step > 0 {
                            nadir_slope.atan()
                        } else {
                            half_pi
                        };
                        let relief_threshold = zenith_threshold.max(nadir_threshold);
                        let signed_relief = zenith_angle + nadir_angle;
                        if signed_relief > relief_threshold {
                            pattern[dir] = 2;
                            count_pos += 1;
                        } else if signed_relief < -relief_threshold {
                            pattern[dir] = 0;
                            count_neg += 1;
                        }
                    }

                    row_out[col] = if output_forms {
                        CLASS_MAP[count_neg][count_pos] as f64
                    } else {
                        let mut power = 1usize;
                        let mut code = 0usize;
                        for digit in pattern {
                            code += digit * power;
                            power *= 3;
                        }
                        gtc[code] as f64
                    };
                }

                row_out
            })
            .collect();

        for (row, row_values) in row_data.iter().enumerate() {
            output
                .set_row_slice(0, row as isize, row_values)
                .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", row, e)))?;
        }

        ctx.progress.progress(1.0);
        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn viewshed_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "viewshed",
            display_name: "Viewshed",
            summary: "Computes station visibility counts from point stations over a DEM.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "stations", description: "Input point vector file of viewing stations.", required: true },
                ToolParamSpec { name: "height", description: "Viewing station height above ground in DEM z units (default 2.0).", required: false },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn viewshed_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("stations".to_string(), json!("stations.gpkg"));
        defaults.insert("height".to_string(), json!(2.0));
        ToolManifest {
            id: "viewshed".to_string(),
            display_name: "Viewshed".to_string(),
            summary: "Computes station visibility counts from point stations over a DEM.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "visibility".to_string(), "terrain".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_viewshed(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let stations_path = parse_vector_path_arg(args, "stations")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let height = args.get("height").and_then(|v| v.as_f64()).unwrap_or(2.0).max(0.0);

        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let nodata = input.nodata;

        let stations_layer = wbvector::read(&stations_path).map_err(|e| {
            ToolError::Validation(format!("failed reading stations vector '{}': {}", stations_path, e))
        })?;
        let station_points = Self::parse_vector_points(&stations_layer)?;

        let mut station_pixels: Vec<(usize, usize, f64)> = Vec::new();
        for (x, y) in station_points {
            if let Some((col, row)) = input.world_to_pixel(x, y) {
                if row >= 0 && col >= 0 && (row as usize) < rows && (col as usize) < cols {
                    let z = input.get(0, row, col);
                    if !input.is_nodata(z) {
                        station_pixels.push((row as usize, col as usize, z + height));
                    }
                }
            }
        }
        if station_pixels.is_empty() {
            return Err(ToolError::Validation(
                "no station points intersect valid DEM cells".to_string(),
            ));
        }

        let mut counts = vec![0.0; rows * cols];
        let visibility_progress = PercentCoalescer::new(1, 99);
        for (stn_idx, (sr, sc, sz)) in station_pixels.iter().enumerate() {
            ctx.progress.info(&format!("running viewshed station {} of {}", stn_idx + 1, station_pixels.len()));
            let station_vis: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![0.0; cols];
                    for c in 0..cols {
                        let tz = input.get(0, r as isize, c as isize);
                        if input.is_nodata(tz) {
                            continue;
                        }
                        if r == *sr && c == *sc {
                            row_out[c] = 1.0;
                            continue;
                        }
                        let dr = r as f64 - *sr as f64;
                        let dc = c as f64 - *sc as f64;
                        let steps = dr.abs().max(dc.abs()) as usize;
                        if steps <= 1 {
                            row_out[c] = 1.0;
                            continue;
                        }

                        let mut visible = true;
                        for step in 1..steps {
                            let t = step as f64 / steps as f64;
                            let rr = *sr as f64 + dr * t;
                            let cc = *sc as f64 + dc * t;
                            let expected = *sz + (tz - *sz) * t;
                            if let Some(zs) = Self::bilinear_sample(&input, 0, rr, cc) {
                                if zs > expected {
                                    visible = false;
                                    break;
                                }
                            }
                        }
                        if visible {
                            row_out[c] = 1.0;
                        }
                    }
                    row_out
                })
                .collect();

            for (r, row) in station_vis.iter().enumerate() {
                for (c, v) in row.iter().enumerate() {
                    counts[r * cols + c] += *v;
                }
            }
            visibility_progress.emit_unit_fraction(
                ctx.progress,
                (stn_idx + 1) as f64 / station_pixels.len().max(1) as f64,
            );
        }

        for r in 0..rows {
            let mut row_out = vec![nodata; cols];
            for c in 0..cols {
                let z = input.get(0, r as isize, c as isize);
                if !input.is_nodata(z) {
                    row_out[c] = counts[r * cols + c];
                }
            }
            output.set_row_slice(0, r as isize, &row_out).map_err(|e| {
                ToolError::Execution(format!("failed writing row {}: {}", r, e))
            })?;
        }

        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn low_points_on_headwater_divides_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "low_points_on_headwater_divides",
            display_name: "Low Points on Headwater Divides",
            summary: "Locates low pass points along divides between neighboring headwater subbasins.",
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Pro,
            params: vec![
                ToolParamSpec {
                    name: "dem",
                    description: "Input depressionless DEM raster path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "streams",
                    description: "Input stream raster path (positive values indicate channel cells).",
                    required: true,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output vector path (default temporary .shp).",
                    required: false,
                },
            ],
        }
    }

    fn low_points_on_headwater_divides_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("dem".to_string(), json!("dem.tif"));
        defaults.insert("streams".to_string(), json!("streams.tif"));
        defaults.insert(
            "output".to_string(),
            json!("low_points_on_headwater_divides.shp"),
        );

        let mut example_args = ToolArgs::new();
        example_args.insert("dem".to_string(), json!("dem.tif"));
        example_args.insert("streams".to_string(), json!("streams.tif"));
        example_args.insert(
            "output".to_string(),
            json!("low_points_on_headwater_divides.shp"),
        );

        ToolManifest {
            id: "low_points_on_headwater_divides".to_string(),
            display_name: "Low Points on Headwater Divides".to_string(),
            summary: "Locates low pass points along divides between neighboring headwater subbasins.".to_string(),
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Pro,
            params: vec![
                ToolParamDescriptor {
                    name: "dem".to_string(),
                    description: "Input depressionless DEM raster path or typed raster object.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "streams".to_string(),
                    description: "Input stream raster path (positive values indicate channel cells).".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output vector path (default temporary .shp).".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_low_points_on_headwater_divides".to_string(),
                description: "Find low pass points between neighboring headwater basins.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "geomorphometry".to_string(),
                "streams".to_string(),
                "subbasins".to_string(),
                "passes".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn run_low_points_on_headwater_divides(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let dem_path = parse_raster_path_arg(args, "dem")
            .or_else(|_| parse_raster_path_arg(args, "input"))?;
        let streams_path = parse_raster_path_arg(args, "streams")
            .or_else(|_| parse_raster_path_arg(args, "streams_raster"))?;
        let output_path = parse_optional_output_path(args, "output")?;

        let dem = Self::load_raster(&dem_path)?;
        let streams = Self::load_raster(&streams_path)?;
        if dem.rows != streams.rows || dem.cols != streams.cols {
            return Err(ToolError::Validation(
                "dem and streams rasters must have matching dimensions".to_string(),
            ));
        }

        let rows = dem.rows;
        let cols = dem.cols;
        let streams_nodata = streams.nodata;

        let cell_size_x = dem.cell_size_x.abs();
        let cell_size_y = dem.cell_size_y.abs();
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        let dx = [1isize, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1isize, 0, 1, 1, 1, 0, -1, -1];
        let inflow_vals = [4i8, 5, 6, 7, 0, 1, 2, 3];
        let grid_lengths = [
            diag_cell_size,
            cell_size_x,
            diag_cell_size,
            cell_size_y,
            diag_cell_size,
            cell_size_x,
            diag_cell_size,
            cell_size_y,
        ];
        let compute_progress = PercentCoalescer::new(1, 99);

        let mut pointer = vec![-2i8; rows * cols];
        for r in 0..rows {
            for c in 0..cols {
                let z = dem.get(0, r as isize, c as isize);
                if dem.is_nodata(z) {
                    continue;
                }
                let mut dir = -1i8;
                let mut max_slope = f64::MIN;
                for i in 0..8 {
                    let rr = r as isize + dy[i];
                    let cc = c as isize + dx[i];
                    if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                        continue;
                    }
                    let zn = dem.get(0, rr, cc);
                    if dem.is_nodata(zn) {
                        continue;
                    }
                    let slope = (z - zn) / grid_lengths[i];
                    if slope > max_slope && slope > 0.0 {
                        max_slope = slope;
                        dir = i as i8;
                    }
                }
                pointer[Self::idx(r, c, cols)] = dir;
            }
            compute_progress.emit_unit_fraction(ctx.progress, (r + 1) as f64 / (rows as f64 * 5.0));
        }

        let is_stream = |v: f64| -> bool { !(v == streams_nodata || v == 0.0) };

        let mut headwaters = vec![0i32; rows * cols];
        let mut valleys = vec![0i32; rows * cols];
        let mut channel_heads: Vec<(usize, usize)> = Vec::new();
        let mut next_headwater_id = 1i32;

        for r in 0..rows {
            for c in 0..cols {
                let idx = Self::idx(r, c, cols);
                let z = dem.get(0, r as isize, c as isize);
                if dem.is_nodata(z) {
                    headwaters[idx] = -1;
                    valleys[idx] = -1;
                    continue;
                }

                let sv = streams.get(0, r as isize, c as isize);
                if is_stream(sv) {
                    let mut inflow = 0usize;
                    for i in 0..8 {
                        let rr = r as isize + dy[i];
                        let cc = c as isize + dx[i];
                        if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                            continue;
                        }
                        let nsv = streams.get(0, rr, cc);
                        if is_stream(nsv)
                            && pointer[Self::idx(rr as usize, cc as usize, cols)] == inflow_vals[i]
                        {
                            inflow += 1;
                        }
                    }
                    if inflow == 0 {
                        headwaters[idx] = next_headwater_id;
                        channel_heads.push((r, c));
                        next_headwater_id += 1;
                    }
                    valleys[idx] = 1;
                }
            }
            compute_progress.emit_unit_fraction(ctx.progress, (rows + r + 1) as f64 / (rows as f64 * 5.0));
        }

        let mut stack = channel_heads.clone();
        while let Some((r, c)) = stack.pop() {
            let hid = headwaters[Self::idx(r, c, cols)];
            for i in 0..8 {
                let rr = r as isize + dy[i];
                let cc = c as isize + dx[i];
                if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                    continue;
                }
                let ni = Self::idx(rr as usize, cc as usize, cols);
                if headwaters[ni] == 0 && pointer[ni] == inflow_vals[i] {
                    headwaters[ni] = hid;
                    stack.push((rr as usize, cc as usize));
                }
            }
        }

        let mut stream_link_id = 1i32;
        let mut stack_stream_cells: Vec<(usize, usize)> = Vec::new();
        for &(start_r, start_c) in &channel_heads {
            valleys[Self::idx(start_r, start_c, cols)] = stream_link_id;
            stack_stream_cells.push((start_r, start_c));

            let mut r = start_r as isize;
            let mut c = start_c as isize;
            loop {
                let dir = pointer[Self::idx(r as usize, c as usize, cols)];
                if dir < 0 {
                    break;
                }
                let rr = r + dy[dir as usize];
                let cc = c + dx[dir as usize];
                if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                    break;
                }
                let ni = Self::idx(rr as usize, cc as usize, cols);
                if valleys[ni] == 1 {
                    let mut inflow = 0usize;
                    for i in 0..8 {
                        let r2 = rr + dy[i];
                        let c2 = cc + dx[i];
                        if r2 < 0 || c2 < 0 || r2 >= rows as isize || c2 >= cols as isize {
                            continue;
                        }
                        let n2 = Self::idx(r2 as usize, c2 as usize, cols);
                        if valleys[n2] >= 1 && pointer[n2] == inflow_vals[i] {
                            inflow += 1;
                        }
                    }
                    if inflow > 1 {
                        stream_link_id += 1;
                    }
                    valleys[ni] = stream_link_id;
                    stack_stream_cells.push((rr as usize, cc as usize));
                    r = rr;
                    c = cc;
                } else {
                    break;
                }
            }
            stream_link_id += 1;
        }

        while let Some((r, c)) = stack_stream_cells.pop() {
            let valley_id = valleys[Self::idx(r, c, cols)];
            for i in 0..8 {
                let rr = r as isize + dy[i];
                let cc = c as isize + dx[i];
                if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                    continue;
                }
                let ni = Self::idx(rr as usize, cc as usize, cols);
                if !dem.is_nodata(dem.get(0, rr, cc))
                    && pointer[ni] == inflow_vals[i]
                    && valleys[ni] == 0
                {
                    valleys[ni] = valley_id;
                    stack_stream_cells.push((rr as usize, cc as usize));
                }
            }
        }

        let mut best_by_pair: std::collections::BTreeMap<(i32, i32), (f64, usize, usize, i32)> =
            std::collections::BTreeMap::new();
        for r in 0..rows {
            for c in 0..cols {
                let idx = Self::idx(r, c, cols);
                let head_id = headwaters[idx];
                if head_id <= 0 {
                    continue;
                }
                let valley1 = valleys[idx];
                for i in 0..8 {
                    let rr = r as isize + dy[i];
                    let cc = c as isize + dx[i];
                    if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                        continue;
                    }
                    let ni = Self::idx(rr as usize, cc as usize, cols);
                    let head2 = headwaters[ni];
                    if head2 == head_id || head2 == -1 {
                        continue;
                    }
                    let valley2 = valleys[ni];
                    if valley1 <= 0 || valley2 <= 0 || valley1 == valley2 {
                        continue;
                    }
                    let key = (valley1.min(valley2), valley1.max(valley2));
                    let z = dem.get(0, r as isize, c as isize);
                    if dem.is_nodata(z) {
                        continue;
                    }
                    match best_by_pair.get(&key) {
                        Some((z0, _, _, _)) if *z0 <= z => {}
                        _ => {
                            best_by_pair.insert(key, (z, r, c, head_id));
                        }
                    }
                }
            }
            compute_progress.emit_unit_fraction(ctx.progress, (rows * 4 + r + 1) as f64 / (rows as f64 * 5.0));
        }

        let mut layer = wbvector::Layer::new("low_points_on_headwater_divides")
            .with_geom_type(wbvector::GeometryType::Point);
        layer.add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));
        layer.add_field(wbvector::FieldDef::new("HEIGHT", wbvector::FieldType::Float));
        layer.add_field(wbvector::FieldDef::new(
            "HEADWTR_ID",
            wbvector::FieldType::Integer,
        ));

        let mut fid = 1i64;
        for (_pair, (z, r, c, hid)) in best_by_pair {
            let x = dem.col_center_x(c as isize);
            let y = dem.row_center_y(r as isize);
            layer
                .add_feature(
                    Some(wbvector::Geometry::Point(wbvector::Coord::xy(x, y))),
                    &[
                        ("FID", wbvector::FieldValue::Integer(fid)),
                        ("HEIGHT", wbvector::FieldValue::Float(z)),
                        (
                            "HEADWTR_ID",
                            wbvector::FieldValue::Integer(hid as i64),
                        ),
                    ],
                )
                .map_err(|e| {
                    ToolError::Execution(format!("failed building output feature: {}", e))
                })?;
            fid += 1;
        }

        let out = Self::write_vector_output(
            &layer,
            output_path,
            "low_points_on_headwater_divides.shp",
        )?;
        Ok(Self::build_result(out))
    }

    fn percent_elev_range_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "percent_elev_range",
            display_name: "Percent Elevation Range",
            summary: "Calculates local topographic position as percent of neighbourhood elevation range.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "filter_size_x", description: "Odd filter width in cells (default 11). Alias: filterx.", required: false },
                ToolParamSpec { name: "filter_size_y", description: "Odd filter height in cells (default filter_size_x). Alias: filtery.", required: false },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn percent_elev_range_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("filter_size_x".to_string(), json!(11));
        defaults.insert("filter_size_y".to_string(), json!(11));
        ToolManifest {
            id: "percent_elev_range".to_string(),
            display_name: "Percent Elevation Range".to_string(),
            summary: "Calculates local topographic position as percent of neighbourhood elevation range.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "local-relief".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_percent_elev_range(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let (filter_size_x, filter_size_y) = Self::parse_filter_sizes(args);
        let mid_x = filter_size_x / 2;
        let mid_y = filter_size_y / 2;

        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running percent_elev_range");
            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let z = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z) {
                            continue;
                        }
                        let y1 = r.saturating_sub(mid_y);
                        let x1 = c.saturating_sub(mid_x);
                        let y2 = (r + mid_y).min(rows - 1);
                        let x2 = (c + mid_x).min(cols - 1);
                        let mut min_v = f64::INFINITY;
                        let mut max_v = f64::NEG_INFINITY;
                        for rr in y1..=y2 {
                            for cc in x1..=x2 {
                                let v = input.get(band, rr as isize, cc as isize);
                                if !input.is_nodata(v) {
                                    min_v = min_v.min(v);
                                    max_v = max_v.max(v);
                                }
                            }
                        }
                        if min_v.is_finite() && max_v.is_finite() {
                            let range = max_v - min_v;
                            row_out[c] = if range > 0.0 { (z - min_v) / range * 100.0 } else { 0.0 };
                        }
                    }
                    row_out
                })
                .collect();

            for (r, row) in row_data.iter().enumerate() {
                output.set_row_slice(band, r as isize, row).map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            }
            ctx.progress.progress((band_idx + 1) as f64 / bands as f64);
        }
        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn relative_topographic_position_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "relative_topographic_position",
            display_name: "Relative Topographic Position",
            summary: "Calculates RTP using neighbourhood min, mean, and max elevation values.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "filter_size_x", description: "Odd filter width in cells (default 11). Alias: filterx.", required: false },
                ToolParamSpec { name: "filter_size_y", description: "Odd filter height in cells (default filter_size_x). Alias: filtery.", required: false },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn relative_topographic_position_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("filter_size_x".to_string(), json!(11));
        defaults.insert("filter_size_y".to_string(), json!(11));
        ToolManifest {
            id: "relative_topographic_position".to_string(),
            display_name: "Relative Topographic Position".to_string(),
            summary: "Calculates RTP using neighbourhood min, mean, and max elevation values.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "local-relief".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_relative_topographic_position(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let (filter_size_x, filter_size_y) = Self::parse_filter_sizes(args);
        let mid_x = filter_size_x / 2;
        let mid_y = filter_size_y / 2;

        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running relative_topographic_position");
            let (sum, _, count) = Self::build_integrals(&input, band);
            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let z = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z) {
                            continue;
                        }
                        let y1 = r.saturating_sub(mid_y);
                        let x1 = c.saturating_sub(mid_x);
                        let y2 = (r + mid_y).min(rows - 1);
                        let x2 = (c + mid_x).min(cols - 1);

                        let n = Self::rect_count(&count, cols, y1, x1, y2, x2);
                        if n <= 0 {
                            row_out[c] = 0.0;
                            continue;
                        }
                        let mean = Self::rect_sum(&sum, cols, y1, x1, y2, x2) / n as f64;

                        let mut min_v = f64::INFINITY;
                        let mut max_v = f64::NEG_INFINITY;
                        for rr in y1..=y2 {
                            for cc in x1..=x2 {
                                let v = input.get(band, rr as isize, cc as isize);
                                if !input.is_nodata(v) {
                                    min_v = min_v.min(v);
                                    max_v = max_v.max(v);
                                }
                            }
                        }
                        if !min_v.is_finite() || !max_v.is_finite() {
                            row_out[c] = 0.0;
                            continue;
                        }
                        row_out[c] = if z < mean {
                            let den = mean - min_v;
                            if den > 0.0 { (z - mean) / den } else { 0.0 }
                        } else {
                            let den = max_v - mean;
                            if den > 0.0 { (z - mean) / den } else { 0.0 }
                        };
                    }
                    row_out
                })
                .collect();
            for (r, row) in row_data.iter().enumerate() {
                output.set_row_slice(band, r as isize, row).map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            }
            ctx.progress.progress((band_idx + 1) as f64 / bands as f64);
        }
        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn num_downslope_neighbours_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "num_downslope_neighbours",
            display_name: "Num Downslope Neighbours",
            summary: "Counts the number of 8-neighbour cells lower than each DEM cell.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn num_downslope_neighbours_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        ToolManifest {
            id: "num_downslope_neighbours".to_string(),
            display_name: "Num Downslope Neighbours".to_string(),
            summary: "Counts the number of 8-neighbour cells lower than each DEM cell.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "flow".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_num_downslope_neighbours(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let offsets = [(-1isize, -1isize), (0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0)];

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running num_downslope_neighbours");
            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let z = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z) {
                            continue;
                        }
                        let mut count = 0.0;
                        for (dx, dy) in offsets {
                            let v = input.get(band, r as isize + dy, c as isize + dx);
                            if !input.is_nodata(v) && v < z {
                                count += 1.0;
                            }
                        }
                        row_out[c] = count;
                    }
                    row_out
                })
                .collect();
            for (r, row) in row_data.iter().enumerate() {
                output.set_row_slice(band, r as isize, row).map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            }
            ctx.progress.progress((band_idx + 1) as f64 / bands as f64);
        }
        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn num_upslope_neighbours_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "num_upslope_neighbours",
            display_name: "Num Upslope Neighbours",
            summary: "Counts the number of 8-neighbour cells higher than each DEM cell.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn num_upslope_neighbours_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        ToolManifest {
            id: "num_upslope_neighbours".to_string(),
            display_name: "Num Upslope Neighbours".to_string(),
            summary: "Counts the number of 8-neighbour cells higher than each DEM cell.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "flow".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_num_upslope_neighbours(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let offsets = [(-1isize, -1isize), (0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0)];

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running num_upslope_neighbours");
            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let z = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z) {
                            continue;
                        }
                        let mut count = 0.0;
                        for (dx, dy) in offsets {
                            let v = input.get(band, r as isize + dy, c as isize + dx);
                            if !input.is_nodata(v) && v > z {
                                count += 1.0;
                            }
                        }
                        row_out[c] = count;
                    }
                    row_out
                })
                .collect();
            for (r, row) in row_data.iter().enumerate() {
                output.set_row_slice(band, r as isize, row).map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            }
            ctx.progress.progress((band_idx + 1) as f64 / bands as f64);
        }
        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn max_downslope_elev_change_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "max_downslope_elev_change",
            display_name: "Max Downslope Elev Change",
            summary: "Calculates the maximum elevation drop to lower neighbouring cells.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn max_downslope_elev_change_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        ToolManifest {
            id: "max_downslope_elev_change".to_string(),
            display_name: "Max Downslope Elev Change".to_string(),
            summary: "Calculates the maximum elevation drop to lower neighbouring cells.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "flow".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_max_downslope_elev_change(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let cell_size_x = input.cell_size_x.abs().max(f64::EPSILON);
        let cell_size_y = input.cell_size_y.abs().max(f64::EPSILON);
        let diag = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        let offsets = [(-1isize, -1isize, diag), (0, -1, cell_size_y), (1, -1, diag), (1, 0, cell_size_x), (1, 1, diag), (0, 1, cell_size_y), (-1, 1, diag), (-1, 0, cell_size_x)];

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running max_downslope_elev_change");
            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let z = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z) {
                            continue;
                        }
                        let mut max_slope = f64::NEG_INFINITY;
                        let mut max_drop = 0.0;
                        for (dx, dy, dist) in offsets {
                            let v = input.get(band, r as isize + dy, c as isize + dx);
                            if !input.is_nodata(v) && v < z {
                                let slope = (z - v) / dist;
                                if slope > max_slope {
                                    max_slope = slope;
                                    max_drop = z - v;
                                }
                            }
                        }
                        row_out[c] = if max_slope > 0.0 { max_drop } else { 0.0 };
                    }
                    row_out
                })
                .collect();
            for (r, row) in row_data.iter().enumerate() {
                output.set_row_slice(band, r as isize, row).map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            }
            ctx.progress.progress((band_idx + 1) as f64 / bands as f64);
        }
        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn max_upslope_elev_change_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "max_upslope_elev_change",
            display_name: "Max Upslope Elev Change",
            summary: "Calculates the maximum elevation gain to higher neighbouring cells.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn max_upslope_elev_change_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        ToolManifest {
            id: "max_upslope_elev_change".to_string(),
            display_name: "Max Upslope Elev Change".to_string(),
            summary: "Calculates the maximum elevation gain to higher neighbouring cells.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "flow".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_max_upslope_elev_change(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let cell_size_x = input.cell_size_x.abs().max(f64::EPSILON);
        let cell_size_y = input.cell_size_y.abs().max(f64::EPSILON);
        let diag = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        let offsets = [(-1isize, -1isize, diag), (0, -1, cell_size_y), (1, -1, diag), (1, 0, cell_size_x), (1, 1, diag), (0, 1, cell_size_y), (-1, 1, diag), (-1, 0, cell_size_x)];

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running max_upslope_elev_change");
            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let z = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z) {
                            continue;
                        }
                        let mut max_slope = f64::NEG_INFINITY;
                        let mut max_rise = 0.0;
                        for (dx, dy, dist) in offsets {
                            let v = input.get(band, r as isize + dy, c as isize + dx);
                            if !input.is_nodata(v) && v > z {
                                let slope = (v - z) / dist;
                                if slope > max_slope {
                                    max_slope = slope;
                                    max_rise = v - z;
                                }
                            }
                        }
                        row_out[c] = if max_slope > 0.0 { max_rise } else { 0.0 };
                    }
                    row_out
                })
                .collect();
            for (r, row) in row_data.iter().enumerate() {
                output.set_row_slice(band, r as isize, row).map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            }
            ctx.progress.progress((band_idx + 1) as f64 / bands as f64);
        }
        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn min_downslope_elev_change_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "min_downslope_elev_change",
            display_name: "Min Downslope Elev Change",
            summary: "Calculates the minimum non-negative elevation drop to neighbouring cells.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn min_downslope_elev_change_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        ToolManifest {
            id: "min_downslope_elev_change".to_string(),
            display_name: "Min Downslope Elev Change".to_string(),
            summary: "Calculates the minimum non-negative elevation drop to neighbouring cells.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "flow".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_min_downslope_elev_change(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let cell_size_x = input.cell_size_x.abs().max(f64::EPSILON);
        let cell_size_y = input.cell_size_y.abs().max(f64::EPSILON);
        let diag = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        let offsets = [(-1isize, -1isize, diag), (0, -1, cell_size_y), (1, -1, diag), (1, 0, cell_size_x), (1, 1, diag), (0, 1, cell_size_y), (-1, 1, diag), (-1, 0, cell_size_x)];

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running min_downslope_elev_change");
            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let z = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z) {
                            continue;
                        }
                        let mut min_slope = f64::INFINITY;
                        let mut min_drop = 0.0;
                        for (dx, dy, dist) in offsets {
                            let v = input.get(band, r as isize + dy, c as isize + dx);
                            if !input.is_nodata(v) {
                                let slope = (z - v) / dist;
                                if slope >= 0.0 && slope < min_slope {
                                    min_slope = slope;
                                    min_drop = z - v;
                                }
                            }
                        }
                        row_out[c] = if min_slope < f64::INFINITY { min_drop } else { 0.0 };
                    }
                    row_out
                })
                .collect();
            for (r, row) in row_data.iter().enumerate() {
                output.set_row_slice(band, r as isize, row).map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            }
            ctx.progress.progress((band_idx + 1) as f64 / bands as f64);
        }
        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn elevation_percentile_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "elevation_percentile",
            display_name: "Elevation Percentile",
            summary: "Calculates the local percentile rank of each cell elevation within a neighbourhood window.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "filter_size_x", description: "Odd filter width in cells (default 11). Alias: filterx.", required: false },
                ToolParamSpec { name: "filter_size_y", description: "Odd filter height in cells (default filter_size_x). Alias: filtery.", required: false },
                ToolParamSpec { name: "sig_digits", description: "Number of significant decimal digits to preserve during binning (default 2).", required: false },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn elevation_percentile_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("filter_size_x".to_string(), json!(11));
        defaults.insert("filter_size_y".to_string(), json!(11));
        defaults.insert("sig_digits".to_string(), json!(2));
        ToolManifest {
            id: "elevation_percentile".to_string(),
            display_name: "Elevation Percentile".to_string(),
            summary: "Calculates the local percentile rank of each cell elevation within a neighbourhood window.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "local-relief".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_elevation_percentile(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let (filter_size_x, filter_size_y) = Self::parse_filter_sizes(args);
        let mid_x = filter_size_x / 2;
        let mid_y = filter_size_y / 2;
        let sig_digits = args
            .get("sig_digits")
            .and_then(|v| v.as_i64())
            .unwrap_or(2)
            .clamp(0, 9) as i32;
        let multiplier = 10f64.powi(sig_digits);

        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running elevation_percentile");
            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let z = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z) {
                            continue;
                        }
                        let z_bin = (z * multiplier).floor();
                        let y1 = r.saturating_sub(mid_y);
                        let x1 = c.saturating_sub(mid_x);
                        let y2 = (r + mid_y).min(rows - 1);
                        let x2 = (c + mid_x).min(cols - 1);
                        let mut n = 0.0;
                        let mut n_less = 0.0;
                        for rr in y1..=y2 {
                            for cc in x1..=x2 {
                                let v = input.get(band, rr as isize, cc as isize);
                                if !input.is_nodata(v) {
                                    n += 1.0;
                                    if (v * multiplier).floor() < z_bin {
                                        n_less += 1.0;
                                    }
                                }
                            }
                        }
                        if n > 0.0 {
                            row_out[c] = n_less / n * 100.0;
                        }
                    }
                    row_out
                })
                .collect();
            for (r, row) in row_data.iter().enumerate() {
                output.set_row_slice(band, r as isize, row).map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            }
            ctx.progress.progress((band_idx + 1) as f64 / bands as f64);
        }
        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn downslope_index_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "downslope_index",
            display_name: "Downslope Index",
            summary: "Calculates Hjerdt et al. (2004) downslope index using D8 flow directions.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "vertical_drop", description: "Vertical drop threshold d for downslope traversal.", required: true },
                ToolParamSpec { name: "output_type", description: "Output metric: tangent, degrees, radians, distance (default tangent).", required: false },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn max_branch_length_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "max_branch_length",
            display_name: "Max Branch Length",
            summary: "Calculates the maximum branch length between D8 flowpaths initiated from neighbouring cells.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "log_transform", description: "Apply natural log transform to positive outputs.", required: false },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn max_branch_length_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("log_transform".to_string(), json!(false));
        ToolManifest {
            id: "max_branch_length".to_string(),
            display_name: "Max Branch Length".to_string(),
            summary: "Calculates maximum branch length between neighbouring D8 flowpaths, useful for highlighting divides.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "hydrology".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_max_branch_length(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let log_transform = args
            .get("log_transform")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        output.data_type = DataType::F32;
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        let cell_size_x = input.cell_size_x.abs().max(f64::EPSILON);
        let cell_size_y = input.cell_size_y.abs().max(f64::EPSILON);
        let diag = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        let dx = [1isize, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1isize, 0, 1, 1, 1, 0, -1, -1];
        let lengths = [diag, cell_size_x, diag, cell_size_y, diag, cell_size_x, diag, cell_size_y];

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running max_branch_length");

            let flow_nodata = -2i8;
            let flow_dir_and_pits: Vec<(i8, bool)> = (0..rows * cols)
                .into_par_iter()
                .map(|idx| {
                    let r = idx / cols;
                    let c = idx % cols;
                    let z = input.get(band, r as isize, c as isize);
                    if input.is_nodata(z) {
                        return (flow_nodata, false);
                    }

                    let mut dir = -1i8;
                    let mut best_slope = f64::NEG_INFINITY;
                    let mut neighbouring_nodata = false;
                    for i in 0..8 {
                        let zn = input.get(band, r as isize + dy[i], c as isize + dx[i]);
                        if input.is_nodata(zn) {
                            neighbouring_nodata = true;
                            continue;
                        }
                        let slope = (z - zn) / lengths[i];
                        if slope > 0.0 && slope > best_slope {
                            best_slope = slope;
                            dir = i as i8;
                        }
                    }

                    let interior_pit = dir < 0 && !neighbouring_nodata;
                    (dir, interior_pit)
                })
                .collect();

            let mut flow_dir = vec![flow_nodata; rows * cols];
            let mut interior_pit_found = false;
            for (i, (dir, is_interior_pit)) in flow_dir_and_pits.into_iter().enumerate() {
                flow_dir[i] = dir;
                if is_interior_pit {
                    interior_pit_found = true;
                }
            }

            if interior_pit_found {
                ctx.progress.info("warning: interior pit cells were found in input DEM; consider depression-filling before running max_branch_length");
            }

            let mut out_vals = vec![0.0f64; rows * cols];
            let mut paths = vec![0isize; rows * cols];
            let mut path_lengths = vec![0.0f64; rows * cols];

            for r in 0..rows {
                for c in 0..cols {
                    let i0 = r * cols + c;
                    if flow_dir[i0] < 0 {
                        if input.is_nodata(input.get(band, r as isize, c as isize)) {
                            out_vals[i0] = nodata;
                        }
                        continue;
                    }

                    let mut trace_pair = |mut r1: isize, mut c1: isize, mut r2: isize, mut c2: isize, marker: isize| -> (f64, f64) {
                        let mut dist1 = 0.0;
                        let mut dist2 = 0.0;
                        let mut flag1 = true;
                        let mut flag2 = true;

                        while flag1 || flag2 {
                            if flag1 {
                                if r1 < 0 || c1 < 0 || r1 >= rows as isize || c1 >= cols as isize {
                                    flag1 = false;
                                } else {
                                    let i = r1 as usize * cols + c1 as usize;
                                    if paths[i] == marker {
                                        flag1 = false;
                                        flag2 = false;
                                        dist2 = path_lengths[i];
                                    } else {
                                        paths[i] = marker;
                                        path_lengths[i] = dist1;
                                        let dir = flow_dir[i];
                                        if dir >= 0 {
                                            let d = dir as usize;
                                            r1 += dy[d];
                                            c1 += dx[d];
                                            dist1 += lengths[d];
                                        } else {
                                            flag1 = false;
                                        }
                                    }
                                }
                            }

                            if flag2 {
                                if r2 < 0 || c2 < 0 || r2 >= rows as isize || c2 >= cols as isize {
                                    flag2 = false;
                                } else {
                                    let i = r2 as usize * cols + c2 as usize;
                                    if paths[i] == marker {
                                        flag1 = false;
                                        flag2 = false;
                                        dist1 = path_lengths[i];
                                    } else {
                                        paths[i] = marker;
                                        path_lengths[i] = dist2;
                                        let dir = flow_dir[i];
                                        if dir >= 0 {
                                            let d = dir as usize;
                                            r2 += dy[d];
                                            c2 += dx[d];
                                            dist2 += lengths[d];
                                        } else {
                                            flag2 = false;
                                        }
                                    }
                                }
                            }
                        }

                        (dist1, dist2)
                    };

                    if c + 1 < cols {
                        let ir = r * cols + (c + 1);
                        if flow_dir[ir] >= 0 {
                            let marker = (r * cols + c + 1) as isize;
                            let (dist1, dist2) = trace_pair(r as isize, c as isize, r as isize, (c + 1) as isize, marker);
                            if dist1 > out_vals[i0] {
                                out_vals[i0] = dist1;
                            }
                            if dist2 > out_vals[ir] {
                                out_vals[ir] = dist2;
                            }
                        }
                    }

                    if r + 1 < rows {
                        let ib = (r + 1) * cols + c;
                        if flow_dir[ib] >= 0 {
                            let marker = -((r * cols + c + 1) as isize);
                            let (dist1, dist2) = trace_pair(r as isize, c as isize, (r + 1) as isize, c as isize, marker);
                            if dist1 > out_vals[i0] {
                                out_vals[i0] = dist1;
                            }
                            if dist2 > out_vals[ib] {
                                out_vals[ib] = dist2;
                            }
                        }
                    }
                }
            }

            if log_transform {
                for r in 0..rows {
                    for c in 0..cols {
                        let idx = r * cols + c;
                        let z = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z) {
                            out_vals[idx] = nodata;
                        } else if out_vals[idx] > 0.0 {
                            out_vals[idx] = out_vals[idx].ln();
                        } else {
                            out_vals[idx] = nodata;
                        }
                    }
                }
            } else {
                for r in 0..rows {
                    for c in 0..cols {
                        let idx = r * cols + c;
                        let z = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z) {
                            out_vals[idx] = nodata;
                        }
                    }
                }
            }

            for r in 0..rows {
                let start = r * cols;
                let end = start + cols;
                output
                    .set_row_slice(band, r as isize, &out_vals[start..end])
                    .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            }

            ctx.progress.progress((band_idx + 1) as f64 / bands as f64);
        }

        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn downslope_index_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("vertical_drop".to_string(), json!(2.0));
        defaults.insert("output_type".to_string(), json!("tangent"));
        ToolManifest {
            id: "downslope_index".to_string(),
            display_name: "Downslope Index".to_string(),
            summary: "Calculates Hjerdt et al. (2004) downslope index using D8 flow directions.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "hydrology".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_downslope_index(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let vertical_drop = args
            .get("vertical_drop")
            .or_else(|| args.get("drop"))
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("missing required numeric parameter 'vertical_drop'".to_string()))?;
        if vertical_drop <= 0.0 {
            return Err(ToolError::Validation("parameter 'vertical_drop' must be > 0".to_string()));
        }
        let output_type = args
            .get("output_type")
            .and_then(|v| v.as_str())
            .unwrap_or("tangent")
            .to_ascii_lowercase();

        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let cell_size_x = input.cell_size_x.abs().max(f64::EPSILON);
        let cell_size_y = input.cell_size_y.abs().max(f64::EPSILON);
        let diag = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        let offsets = [(-1isize, -1isize), (0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0)];
        let lengths = [diag, cell_size_y, diag, cell_size_x, diag, cell_size_y, diag, cell_size_x];

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running downslope_index");

            let flow_dir: Vec<i8> = (0..rows * cols)
                .into_par_iter()
                .map(|idx| {
                    let r = idx / cols;
                    let c = idx % cols;
                    let z = input.get(band, r as isize, c as isize);
                    if input.is_nodata(z) {
                        return -1;
                    }
                    let mut best = -1i8;
                    let mut best_slope = f64::NEG_INFINITY;
                    for (i, (dx, dy)) in offsets.iter().enumerate() {
                        let v = input.get(band, r as isize + dy, c as isize + dx);
                        if input.is_nodata(v) {
                            continue;
                        }
                        let slope = (z - v) / lengths[i];
                        if slope > 0.0 && slope > best_slope {
                            best_slope = slope;
                            best = i as i8;
                        }
                    }
                    best
                })
                .collect();

            let max_steps = rows * cols;
            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let z0 = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z0) {
                            continue;
                        }
                        let mut rr = r as isize;
                        let mut cc = c as isize;
                        let mut dist = 0.0;
                        let mut z_drop = z0;
                        let mut steps = 0usize;
                        while steps < max_steps {
                            steps += 1;
                            let idx = rr as usize * cols + cc as usize;
                            let dir = flow_dir[idx];
                            if dir < 0 {
                                break;
                            }
                            let (dx, dy) = offsets[dir as usize];
                            rr += dy;
                            cc += dx;
                            if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                                break;
                            }
                            dist += lengths[dir as usize];
                            let zn = input.get(band, rr, cc);
                            if input.is_nodata(zn) {
                                break;
                            }
                            z_drop = zn;
                            if (z0 - zn) >= vertical_drop {
                                break;
                            }
                        }
                        row_out[c] = if dist > 0.0 {
                            let t = (z0 - z_drop) / dist;
                            if output_type.contains("dist") {
                                dist
                            } else if output_type.contains("deg") || output_type.contains("slop") {
                                t.atan().to_degrees()
                            } else if output_type.contains("rad") {
                                t.atan()
                            } else {
                                t
                            }
                        } else {
                            0.0
                        };
                    }
                    row_out
                })
                .collect();

            for (r, row) in row_data.iter().enumerate() {
                output.set_row_slice(band, r as isize, row).map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            }
            ctx.progress.progress((band_idx + 1) as f64 / bands as f64);
        }
        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn elev_above_pit_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "elev_above_pit",
            display_name: "Elev Above Pit",
            summary: "Calculates elevation above the nearest downslope pit cell (or edge sink).",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn elev_above_pit_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        ToolManifest {
            id: "elev_above_pit".to_string(),
            display_name: "Elev Above Pit".to_string(),
            summary: "Calculates elevation above the nearest downslope pit cell (or edge sink).".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "relative-elevation".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_elev_above_pit(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let cell_size_x = input.cell_size_x.abs().max(f64::EPSILON);
        let cell_size_y = input.cell_size_y.abs().max(f64::EPSILON);
        let diag = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        let offsets = [(-1isize, -1isize), (0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0)];
        let lengths = [diag, cell_size_y, diag, cell_size_x, diag, cell_size_y, diag, cell_size_x];
        let inflowing_vals = [4i8, 5i8, 6i8, 7i8, 0i8, 1i8, 2i8, 3i8];

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running elev_above_pit");

            let flow_dir: Vec<i8> = (0..rows * cols)
                .into_par_iter()
                .map(|idx| {
                    let r = idx / cols;
                    let c = idx % cols;
                    let z = input.get(band, r as isize, c as isize);
                    if input.is_nodata(z) {
                        return -2;
                    }
                    let mut best = -1i8;
                    let mut best_slope = f64::NEG_INFINITY;
                    for (i, (dx, dy)) in offsets.iter().enumerate() {
                        let v = input.get(band, r as isize + dy, c as isize + dx);
                        if input.is_nodata(v) {
                            continue;
                        }
                        let slope = (z - v) / lengths[i];
                        if slope > 0.0 && slope > best_slope {
                            best_slope = slope;
                            best = i as i8;
                        }
                    }
                    best
                })
                .collect();

            let mut row_data = vec![vec![nodata; cols]; rows];
            let mut stack: Vec<(usize, usize, f64)> = Vec::with_capacity(rows * cols / 8);
            for r in 0..rows {
                for c in 0..cols {
                    let idx = r * cols + c;
                    if flow_dir[idx] == -1 {
                        row_data[r][c] = 0.0;
                        let z = input.get(band, r as isize, c as isize);
                        stack.push((r, c, z));
                    }
                }
            }

            while let Some((r, c, basin_z)) = stack.pop() {
                for n in 0..8 {
                    let rn = r as isize + offsets[n].1;
                    let cn = c as isize + offsets[n].0;
                    if rn < 0 || cn < 0 || rn >= rows as isize || cn >= cols as isize {
                        continue;
                    }
                    let rr = rn as usize;
                    let cc = cn as usize;
                    let nidx = rr * cols + cc;
                    if flow_dir[nidx] == inflowing_vals[n] {
                        let zn = input.get(band, rn, cn);
                        if !input.is_nodata(zn) {
                            row_data[rr][cc] = zn - basin_z;
                            stack.push((rr, cc, basin_z));
                        }
                    }
                }
            }

            for (r, row) in row_data.iter().enumerate() {
                output
                    .set_row_slice(band, r as isize, row)
                    .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            }
            ctx.progress.progress((band_idx + 1) as f64 / bands as f64);
        }

        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn directional_relief_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "directional_relief",
            display_name: "Directional Relief",
            summary: "Calculates directional relief by ray-tracing elevation in a specified azimuth.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "azimuth", description: "Direction azimuth in degrees clockwise from north.", required: false },
                ToolParamSpec { name: "max_dist", description: "Optional maximum ray distance in map units.", required: false },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn directional_relief_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("azimuth".to_string(), json!(0.0));
        ToolManifest {
            id: "directional_relief".to_string(),
            display_name: "Directional Relief".to_string(),
            summary: "Calculates directional relief by ray-tracing elevation in a specified azimuth.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "relief".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn bilinear_sample(input: &Raster, band: isize, y: f64, x: f64) -> Option<f64> {
        if y < 0.0 || x < 0.0 {
            return None;
        }
        let y0 = y.floor() as isize;
        let x0 = x.floor() as isize;
        let y1 = y0 + 1;
        let x1 = x0 + 1;
        if y1 >= input.rows as isize || x1 >= input.cols as isize {
            return None;
        }
        let z00 = input.get(band, y0, x0);
        let z10 = input.get(band, y1, x0);
        let z01 = input.get(band, y0, x1);
        let z11 = input.get(band, y1, x1);
        if input.is_nodata(z00) || input.is_nodata(z10) || input.is_nodata(z01) || input.is_nodata(z11) {
            return None;
        }
        let tx = x - x0 as f64;
        let ty = y - y0 as f64;
        let a = z00 * (1.0 - tx) + z01 * tx;
        let b = z10 * (1.0 - tx) + z11 * tx;
        Some(a * (1.0 - ty) + b * ty)
    }

    fn run_directional_relief(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let mut azimuth = args
            .get("azimuth")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        while azimuth < 0.0 {
            azimuth += 360.0;
        }
        while azimuth >= 360.0 {
            azimuth -= 360.0;
        }
        let max_dist = args
            .get("max_dist")
            .and_then(|v| v.as_f64())
            .filter(|v| *v > 0.0)
            .unwrap_or(f64::INFINITY);

        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        let theta = azimuth.to_radians();
        let dx = theta.sin();
        let dy = -theta.cos();

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running directional_relief");

            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];

                    let row_cell_size = if Self::raster_is_geographic(&input) {
                        let lat_rad = input.row_center_y(r as isize).to_radians();
                        ((input.cell_size_x.abs() + input.cell_size_y.abs()) / 2.0) * 111_111.0 * lat_rad.cos().abs().max(1e-6)
                    } else {
                        (input.cell_size_x.abs() + input.cell_size_y.abs()) / 2.0
                    }
                    .max(f64::EPSILON);

                    for c in 0..cols {
                        let z0 = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z0) {
                            continue;
                        }

                        let mut total = 0.0;
                        let mut n = 0.0;
                        let mut step = 1.0;
                        loop {
                            let yy = r as f64 + dy * step;
                            let xx = c as f64 + dx * step;
                            if yy <= 0.0 || xx <= 0.0 || yy >= (rows - 1) as f64 || xx >= (cols - 1) as f64 {
                                break;
                            }
                            let dist = step * row_cell_size;
                            if dist > max_dist {
                                break;
                            }
                            if let Some(z) = Self::bilinear_sample(&input, band, yy, xx) {
                                total += z;
                                n += 1.0;
                            }
                            step += 1.0;
                        }

                        if n > 0.0 {
                            row_out[c] = total / n - z0;
                        }
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

        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn exposure_towards_wind_flux_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "exposure_towards_wind_flux",
            display_name: "Exposure Towards Wind Flux",
            summary: "Calculates terrain exposure relative to a dominant wind azimuth and upwind horizon angle.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "azimuth", description: "Dominant wind azimuth in degrees clockwise from north.", required: false },
                ToolParamSpec { name: "max_dist", description: "Maximum search distance for upwind horizon angle in map units.", required: false },
                ToolParamSpec { name: "z_factor", description: "Optional vertical scaling factor for projected DEMs.", required: false },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn exposure_towards_wind_flux_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("azimuth".to_string(), json!(0.0));
        defaults.insert("max_dist".to_string(), json!(f64::INFINITY));
        defaults.insert("z_factor".to_string(), json!(1.0));
        ToolManifest {
            id: "exposure_towards_wind_flux".to_string(),
            display_name: "Exposure Towards Wind Flux".to_string(),
            summary: "Calculates terrain exposure relative to dominant wind direction and upwind horizon shielding.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec![
                "geomorphometry".to_string(),
                "terrain".to_string(),
                "wind".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn run_exposure_towards_wind_flux(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let z_factor = args
            .get("z_factor")
            .or_else(|| args.get("zfactor"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        let mut max_dist = args
            .get("max_dist")
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::INFINITY);
        let mut azimuth = args
            .get("azimuth")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
            % 360.0;
        if azimuth < 0.0 {
            azimuth += 360.0;
        }

        let line_slope = if azimuth < 180.0 {
            (90.0 - azimuth).to_radians().tan()
        } else {
            (270.0 - azimuth).to_radians().tan()
        };

        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let cell_size_x = input.cell_size_x.abs().max(f64::EPSILON);
        let cell_size_y = input.cell_size_y.abs().max(f64::EPSILON);
        let eight_grid_res = cell_size_x * 8.0;
        if max_dist <= 5.0 * cell_size_x {
            return Err(ToolError::Validation(
                "parameter 'max_dist' must be larger than 5 x cell size".to_string(),
            ));
        }

        let diag_length = ((rows as f64 * cell_size_y).powi(2) + (cols as f64 * cell_size_x).powi(2)).sqrt();
        if max_dist > diag_length {
            max_dist = diag_length;
        }

        let mut row_z_factors = vec![z_factor; rows];
        if Self::raster_is_geographic(&input) {
            row_z_factors = (0..rows)
                .map(|r| {
                    let lat = input.row_center_y(r as isize).to_radians();
                    let denom = 111_320.0 * lat.cos().abs().max(1.0e-8);
                    1.0 / denom
                })
                .collect();
        }

        let x_step: isize;
        let y_step: isize;
        if azimuth > 0.0 && azimuth <= 90.0 {
            x_step = 1;
            y_step = 1;
        } else if azimuth <= 180.0 {
            x_step = 1;
            y_step = -1;
        } else if azimuth <= 270.0 {
            x_step = -1;
            y_step = -1;
        } else {
            x_step = -1;
            y_step = 1;
        }

        let mut offsets: Vec<(isize, isize, isize, isize, f64, f64)> = Vec::new();
        if line_slope.abs() > f64::EPSILON {
            let mut y = 0.0f64;
            loop {
                y += y_step as f64;
                let x = y / line_slope;
                let dist = (x * cell_size_x).hypot(-y * cell_size_y);
                if dist > max_dist {
                    break;
                }
                let x1 = x.floor() as isize;
                let x2 = x1 + 1;
                let y1 = -y as isize;
                let weight = x - x1 as f64;
                offsets.push((x1, y1, x2, y1, weight, dist));
            }
        }

        let mut x = 0.0f64;
        loop {
            x += x_step as f64;
            let y = -(line_slope * x);
            let dist = (x * cell_size_x).hypot(y * cell_size_y);
            if dist > max_dist {
                break;
            }
            let y1 = y.floor() as isize;
            let y2 = y1 + 1;
            let x1 = x as isize;
            let weight = y - y1 as f64;
            offsets.push((x1, y1, x1, y2, weight, dist));
        }
        offsets.sort_by(|a, b| a.5.partial_cmp(&b.5).unwrap_or(std::cmp::Ordering::Equal));

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running exposure_towards_wind_flux");

            let slope_aspect_rows: Vec<(Vec<f64>, Vec<f64>)> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut aspect_row = vec![nodata; cols];
                    let mut slope_row = vec![nodata; cols];
                    let row = r as isize;
                    let zf = row_z_factors[r];

                    for c in 0..cols {
                        let col = c as isize;
                        let zc = input.get(band, row, col);
                        if input.is_nodata(zc) {
                            continue;
                        }

                        let z = |dr: isize, dc: isize| {
                            let v = input.get(band, row + dr, col + dc);
                            if input.is_nodata(v) { zc * zf } else { v * zf }
                        };

                        let n0 = z(-1, -1);
                        let n1 = z(-1, 0);
                        let n2 = z(-1, 1);
                        let n3 = z(0, 1);
                        let n4 = z(1, 1);
                        let n5 = z(1, 0);
                        let n6 = z(1, -1);
                        let n7 = z(0, -1);

                        let mut fx = (n2 - n4 + 2.0 * (n1 - n5) + n0 - n6) / eight_grid_res;
                        if fx == 0.0 {
                            fx = 0.00001;
                        }
                        let fy = (n6 - n4 + 2.0 * (n7 - n3) + n0 - n2) / eight_grid_res;
                        aspect_row[c] = 180.0 - (fy / fx).atan().to_degrees() + 90.0 * (fx / fx.abs());
                        slope_row[c] = (fx * fx + fy * fy).sqrt().atan();
                    }

                    (aspect_row, slope_row)
                })
                .collect();

            let aspect_rows: Vec<Vec<f64>> = slope_aspect_rows.iter().map(|(a, _)| a.clone()).collect();
            let slope_rows: Vec<Vec<f64>> = slope_aspect_rows.into_iter().map(|(_, s)| s).collect();

            let horizon_rows: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    let row = r as isize;
                    let early_stopping_slope = 80f64.to_radians().tan();
                    for c in 0..cols {
                        let col = c as isize;
                        let current_elev = input.get(band, row, col);
                        if input.is_nodata(current_elev) {
                            continue;
                        }

                        let mut current_max_slope = f64::NEG_INFINITY;
                        let mut current_max_elev = f64::NEG_INFINITY;
                        for off in &offsets {
                            let x1 = col + off.0;
                            let y1 = row + off.1;
                            let x2 = col + off.2;
                            let y2 = row + off.3;

                            let mut z1 = input.get(band, y1, x1);
                            let mut z2 = input.get(band, y2, x2);
                            let z1_nodata = input.is_nodata(z1);
                            let z2_nodata = input.is_nodata(z2);
                            if z1_nodata && z2_nodata {
                                break;
                            } else if z1_nodata {
                                z1 = z2;
                            } else if z2_nodata {
                                z2 = z1;
                            }

                            let z = z1 + off.4 * (z2 - z1);
                            if z > current_max_elev {
                                current_max_elev = z;
                                let slope = (z - current_elev) / off.5;
                                if slope > current_max_slope {
                                    current_max_slope = slope;
                                    if slope > early_stopping_slope {
                                        break;
                                    }
                                }
                            }
                        }

                        if current_max_slope.is_finite() {
                            row_out[c] = current_max_slope.atan();
                        } else {
                            row_out[c] = 0.0;
                        }
                    }
                    row_out
                })
                .collect();

            let azimuth_rad = azimuth.to_radians();
            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let aspect = aspect_rows[r][c];
                        if aspect == nodata {
                            continue;
                        }
                        let mut rel_aspect = (azimuth_rad - aspect.to_radians()).abs();
                        if rel_aspect > std::f64::consts::PI {
                            rel_aspect = 2.0 * std::f64::consts::PI - rel_aspect;
                        }
                        let slope = slope_rows[r][c];
                        let horizon = horizon_rows[r][c].max(0.0);
                        row_out[c] = horizon.sin() * slope.cos()
                            + horizon.cos() * slope.sin() * rel_aspect.cos();
                    }
                    row_out
                })
                .collect();

            for (r, row) in row_data.iter().enumerate() {
                output.set_row_slice(band, r as isize, row).map_err(|e| {
                    ToolError::Execution(format!("failed writing row {}: {}", r, e))
                })?;
            }
            output.data_type = DataType::F32;
            ctx.progress.progress((band_idx + 1) as f64 / bands as f64);
        }

        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn relative_aspect_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "relative_aspect",
            display_name: "Relative Aspect",
            summary: "Calculates terrain aspect relative to a user-specified azimuth (0 to 180 degrees).",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "azimuth", description: "Reference azimuth in degrees clockwise from north.", required: false },
                ToolParamSpec { name: "z_factor", description: "Optional vertical scaling factor.", required: false },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn relative_aspect_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("azimuth".to_string(), json!(0.0));
        defaults.insert("z_factor".to_string(), json!(1.0));
        ToolManifest {
            id: "relative_aspect".to_string(),
            display_name: "Relative Aspect".to_string(),
            summary: "Calculates terrain aspect relative to a user-specified azimuth (0 to 180 degrees).".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "aspect".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_relative_aspect(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let mut azimuth = args
            .get("azimuth")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        while azimuth < 0.0 {
            azimuth += 360.0;
        }
        while azimuth >= 360.0 {
            azimuth -= 360.0;
        }
        let z_factor = args
            .get("z_factor")
            .or_else(|| args.get("zfactor"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let resx = input.cell_size_x.abs().max(f64::EPSILON);
        let resy = input.cell_size_y.abs().max(f64::EPSILON);

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running relative_aspect");

            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let zc = input.get(band, r as isize, c as isize);
                        if input.is_nodata(zc) {
                            continue;
                        }

                        let z = |dr: isize, dc: isize| {
                            let v = input.get(band, r as isize + dr, c as isize + dc);
                            if input.is_nodata(v) { zc } else { v }
                        };

                        let z1 = z(-1, -1) * z_factor;
                        let z2 = z(-1, 0) * z_factor;
                        let z3 = z(-1, 1) * z_factor;
                        let z4 = z(0, -1) * z_factor;
                        let z6 = z(0, 1) * z_factor;
                        let z7 = z(1, -1) * z_factor;
                        let z8 = z(1, 0) * z_factor;
                        let z9 = z(1, 1) * z_factor;

                        let dzdx = ((z3 + 2.0 * z6 + z9) - (z1 + 2.0 * z4 + z7)) / (8.0 * resx);
                        let dzdy = ((z7 + 2.0 * z8 + z9) - (z1 + 2.0 * z2 + z3)) / (8.0 * resy);
                        if dzdx.abs() < f64::EPSILON && dzdy.abs() < f64::EPSILON {
                            row_out[c] = -1.0;
                            continue;
                        }

                        let mut aspect = 90.0 - dzdy.atan2(-dzdx).to_degrees();
                        if aspect < 0.0 {
                            aspect += 360.0;
                        }
                        if aspect >= 360.0 {
                            aspect -= 360.0;
                        }

                        let mut rel = (aspect - azimuth).abs();
                        if rel > 180.0 {
                            rel = 360.0 - rel;
                        }
                        row_out[c] = rel;
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

        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn edge_density_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "edge_density",
            display_name: "Edge Density",
            summary: "Calculates local density of breaks-in-slope using angular normal-vector differences.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "filter_size", description: "Odd neighbourhood size in cells (default 11). Alias: filter.", required: false },
                ToolParamSpec { name: "norm_diff", description: "Normal-vector angular threshold in degrees (default 5).", required: false },
                ToolParamSpec { name: "z_factor", description: "Optional vertical scaling factor.", required: false },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn edge_density_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("filter_size".to_string(), json!(11));
        defaults.insert("norm_diff".to_string(), json!(5.0));
        defaults.insert("z_factor".to_string(), json!(1.0));
        ToolManifest {
            id: "edge_density".to_string(),
            display_name: "Edge Density".to_string(),
            summary: "Calculates local density of breaks-in-slope using angular normal-vector differences.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "roughness".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_edge_density(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let mut filter_size = args
            .get("filter_size")
            .or_else(|| args.get("filter"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(11)
            .max(3);
        if filter_size % 2 == 0 {
            filter_size += 1;
        }
        let mid = filter_size / 2;
        let norm_diff = args
            .get("norm_diff")
            .and_then(|v| v.as_f64())
            .unwrap_or(5.0)
            .clamp(0.0, 90.0);
        let threshold = norm_diff.to_radians().cos();
        let z_factor = args
            .get("z_factor")
            .or_else(|| args.get("zfactor"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let resx = input.cell_size_x.abs().max(f64::EPSILON);
        let resy = input.cell_size_y.abs().max(f64::EPSILON);

        let offsets = [(-1isize, -1isize), (0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0)];

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running edge_density");

            let normals: Vec<Option<[f64; 3]>> = (0..rows * cols)
                .into_par_iter()
                .map(|idx| {
                    let r = idx / cols;
                    let c = idx % cols;
                    let zc = input.get(band, r as isize, c as isize);
                    if input.is_nodata(zc) {
                        return None;
                    }
                    let z = |dr: isize, dc: isize| {
                        let v = input.get(band, r as isize + dr, c as isize + dc);
                        if input.is_nodata(v) { zc } else { v }
                    };
                    let z1 = z(-1, -1) * z_factor;
                    let z2 = z(-1, 0) * z_factor;
                    let z3 = z(-1, 1) * z_factor;
                    let z4 = z(0, -1) * z_factor;
                    let z6 = z(0, 1) * z_factor;
                    let z7 = z(1, -1) * z_factor;
                    let z8 = z(1, 0) * z_factor;
                    let z9 = z(1, 1) * z_factor;

                    let dzdx = ((z3 + 2.0 * z6 + z9) - (z1 + 2.0 * z4 + z7)) / (8.0 * resx);
                    let dzdy = ((z7 + 2.0 * z8 + z9) - (z1 + 2.0 * z2 + z3)) / (8.0 * resy);
                    let mut nx = -dzdx;
                    let mut ny = -dzdy;
                    let mut nz = 1.0;
                    let m = (nx * nx + ny * ny + nz * nz).sqrt();
                    if m <= f64::EPSILON {
                        return None;
                    }
                    nx /= m;
                    ny /= m;
                    nz /= m;
                    Some([nx, ny, nz])
                })
                .collect();

            let edge_mask: Vec<f64> = (0..rows * cols)
                .into_par_iter()
                .map(|idx| {
                    let r = idx / cols;
                    let c = idx % cols;
                    let center = normals[idx];
                    if center.is_none() {
                        return nodata;
                    }
                    let n0 = center.unwrap();
                    let mut is_edge = false;
                    for (dx, dy) in offsets {
                        let rr = r as isize + dy;
                        let cc = c as isize + dx;
                        if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                            continue;
                        }
                        let nidx = rr as usize * cols + cc as usize;
                        if let Some(nn) = normals[nidx] {
                            let dot = (n0[0] * nn[0] + n0[1] * nn[1] + n0[2] * nn[2]).clamp(-1.0, 1.0);
                            if dot < threshold {
                                is_edge = true;
                                break;
                            }
                        }
                    }
                    if is_edge { 1.0 } else { 0.0 }
                })
                .collect();

            let mut sum = vec![0.0; rows * cols];
            let mut count = vec![0i64; rows * cols];
            for r in 0..rows {
                let mut row_sum = 0.0;
                let mut row_count = 0i64;
                for c in 0..cols {
                    let idx = r * cols + c;
                    let v = edge_mask[idx];
                    if v != nodata {
                        row_sum += v;
                        row_count += 1;
                    }
                    if r > 0 {
                        let a = (r - 1) * cols + c;
                        sum[idx] = row_sum + sum[a];
                        count[idx] = row_count + count[a];
                    } else {
                        sum[idx] = row_sum;
                        count[idx] = row_count;
                    }
                }
            }

            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let idx = r * cols + c;
                        if edge_mask[idx] == nodata {
                            continue;
                        }
                        let y1 = r.saturating_sub(mid);
                        let x1 = c.saturating_sub(mid);
                        let y2 = (r + mid).min(rows - 1);
                        let x2 = (c + mid).min(cols - 1);
                        let n = Self::rect_count(&count, cols, y1, x1, y2, x2);
                        if n > 0 {
                            let s = Self::rect_sum(&sum, cols, y1, x1, y2, x2);
                            row_out[c] = (s / n as f64).clamp(0.0, 1.0);
                        }
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

        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn spherical_std_dev_of_normals_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "spherical_std_dev_of_normals",
            display_name: "Spherical Std Dev Of Normals",
            summary: "Calculates spherical standard deviation of local surface normals.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "filter_size", description: "Odd neighbourhood size in cells (default 11). Alias: filter.", required: false },
                ToolParamSpec { name: "z_factor", description: "Optional vertical scaling factor.", required: false },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn spherical_std_dev_of_normals_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("filter_size".to_string(), json!(11));
        defaults.insert("z_factor".to_string(), json!(1.0));
        ToolManifest {
            id: "spherical_std_dev_of_normals".to_string(),
            display_name: "Spherical Std Dev Of Normals".to_string(),
            summary: "Calculates spherical standard deviation of local surface normals.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "roughness".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_spherical_std_dev_of_normals(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let mut filter_size = args
            .get("filter_size")
            .or_else(|| args.get("filter"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(11)
            .max(3);
        if filter_size % 2 == 0 {
            filter_size += 1;
        }
        let mid = filter_size / 2;
        let z_factor = args
            .get("z_factor")
            .or_else(|| args.get("zfactor"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running spherical_std_dev_of_normals");

            let mut base = vec![f64::NAN; rows * cols];
            for r in 0..rows {
                for c in 0..cols {
                    let v = input.get(band, r as isize, c as isize);
                    if !input.is_nodata(v) {
                        base[r * cols + c] = v;
                    }
                }
            }

            let sigma = (mid as f64 + 0.5) / 3.0;
            let smoothed = Self::gaussian_blur_values(&base, rows, cols, sigma.max(1.0));
            let (nx, ny, nz) = Self::compute_normals_from_values(
                &smoothed,
                rows,
                cols,
                input.cell_size_x,
                input.cell_size_y,
                z_factor,
            );

            let (sum_x, count_x) = Self::build_integral_from_values(&nx, rows, cols);
            let (sum_y, _) = Self::build_integral_from_values(&ny, rows, cols);
            let (sum_z, _) = Self::build_integral_from_values(&nz, rows, cols);

            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let idx = r * cols + c;
                        if !nx[idx].is_finite() {
                            continue;
                        }
                        let y1 = r.saturating_sub(mid);
                        let x1 = c.saturating_sub(mid);
                        let y2 = (r + mid).min(rows - 1);
                        let x2 = (c + mid).min(cols - 1);
                        let n = Self::rect_count(&count_x, cols, y1, x1, y2, x2) as f64;
                        if n <= 0.0 {
                            continue;
                        }
                        let sx = Self::rect_sum(&sum_x, cols, y1, x1, y2, x2);
                        let sy = Self::rect_sum(&sum_y, cols, y1, x1, y2, x2);
                        let sz = Self::rect_sum(&sum_z, cols, y1, x1, y2, x2);
                        let rlen = (sx * sx + sy * sy + sz * sz).sqrt();
                        let rn = (rlen / n).clamp(1e-12, 1.0);
                        row_out[c] = (-2.0 * rn.ln()).sqrt() * 180.0 / std::f64::consts::PI;
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

        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    fn average_normal_vector_angular_deviation_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "average_normal_vector_angular_deviation",
            display_name: "Average Normal Vector Angular Deviation",
            summary: "Calculates local mean angular deviation between original and smoothed surface normals.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "filter_size", description: "Odd neighbourhood size in cells (default 11). Alias: filter.", required: false },
                ToolParamSpec { name: "z_factor", description: "Optional vertical scaling factor.", required: false },
                ToolParamSpec { name: "output", description: "Optional output path.", required: false },
            ],
        }
    }

    fn average_normal_vector_angular_deviation_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("filter_size".to_string(), json!(11));
        defaults.insert("z_factor".to_string(), json!(1.0));
        ToolManifest {
            id: "average_normal_vector_angular_deviation".to_string(),
            display_name: "Average Normal Vector Angular Deviation".to_string(),
            summary: "Calculates local mean angular deviation between original and smoothed surface normals.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "roughness".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_average_normal_vector_angular_deviation(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let mut filter_size = args
            .get("filter_size")
            .or_else(|| args.get("filter"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(11)
            .max(3);
        if filter_size % 2 == 0 {
            filter_size += 1;
        }
        let mid = filter_size / 2;
        let z_factor = args
            .get("z_factor")
            .or_else(|| args.get("zfactor"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running average_normal_vector_angular_deviation");

            let mut base = vec![f64::NAN; rows * cols];
            for r in 0..rows {
                for c in 0..cols {
                    let v = input.get(band, r as isize, c as isize);
                    if !input.is_nodata(v) {
                        base[r * cols + c] = v;
                    }
                }
            }

            let sigma = (mid as f64 + 0.5) / 3.0;
            let smoothed = Self::gaussian_blur_values(&base, rows, cols, sigma.max(1.0));
            let (onx, ony, onz) = Self::compute_normals_from_values(
                &base,
                rows,
                cols,
                input.cell_size_x,
                input.cell_size_y,
                z_factor,
            );
            let (snx, sny, snz) = Self::compute_normals_from_values(
                &smoothed,
                rows,
                cols,
                input.cell_size_x,
                input.cell_size_y,
                z_factor,
            );

            let mut diff = vec![f64::NAN; rows * cols];
            for i in 0..rows * cols {
                if onx[i].is_finite() && snx[i].is_finite() {
                    let dot = (onx[i] * snx[i] + ony[i] * sny[i] + onz[i] * snz[i]).clamp(-1.0, 1.0);
                    diff[i] = dot.acos().to_degrees();
                }
            }

            let (sum, count) = Self::build_integral_from_values(&diff, rows, cols);
            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let idx = r * cols + c;
                        if !diff[idx].is_finite() {
                            continue;
                        }
                        let y1 = r.saturating_sub(mid);
                        let x1 = c.saturating_sub(mid);
                        let y2 = (r + mid).min(rows - 1);
                        let x2 = (c + mid).min(cols - 1);
                        let n = Self::rect_count(&count, cols, y1, x1, y2, x2) as f64;
                        if n > 0.0 {
                            let s = Self::rect_sum(&sum, cols, y1, x1, y2, x2);
                            row_out[c] = s / n;
                        }
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

        Ok(Self::build_result(Self::write_or_store_output(output, output_path)?))
    }

    // -----------------------------------------------------------------------
    // Ruggedness Index (TRI) — Riley et al. (1999)
    // -----------------------------------------------------------------------

    fn ruggedness_index_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "ruggedness_index",
            display_name: "Ruggedness Index",
            summary: "Calculates the terrain ruggedness index (TRI) after Riley et al. (1999).",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input DEM raster path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output path. If omitted, output remains in memory.",
                    required: false,
                },
            ],
        }
    }

    fn ruggedness_index_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));
        example_args.insert("output".to_string(), json!("tri.tif"));

        ToolManifest {
            id: "ruggedness_index".to_string(),
            display_name: "Ruggedness Index".to_string(),
            summary: "Calculates the terrain ruggedness index (TRI) after Riley et al. (1999)."
                .to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input DEM raster path or typed raster object.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output path. If omitted, result is stored in memory."
                        .to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_tri".to_string(),
                description: "Compute TRI from a DEM.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "geomorphometry".to_string(),
                "terrain".to_string(),
                "ruggedness".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn run_ruggedness_index(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("running ruggedness_index");
        ctx.progress.info("reading input raster");
        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    let row = r as isize;
                    for c in 0..cols {
                        let col = c as isize;
                        let z_centre = input.get(band, row, col);
                        if input.is_nodata(z_centre) {
                            continue;
                        }
                        let offsets: [(isize, isize); 8] = [
                            (-1, -1), (0, -1), (1, -1),
                            (-1,  0),           (1,  0),
                            (-1,  1), (0,  1), (1,  1),
                        ];
                        let mut sum_sq = 0.0_f64;
                        let mut n = 0_usize;
                        for (ox, oy) in offsets {
                            let v = input.get(band, row + oy, col + ox);
                            if !input.is_nodata(v) {
                                let diff = v - z_centre;
                                sum_sq += diff * diff;
                                n += 1;
                            }
                        }
                        if n > 0 {
                            row_out[c] = (sum_sq / n as f64).sqrt();
                        }
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
        Ok(Self::build_result(output_locator))
    }

    // -----------------------------------------------------------------------
    // Surface Area Ratio — Jenness (2004)
    // -----------------------------------------------------------------------

    fn surface_area_ratio_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "surface_area_ratio",
            display_name: "Surface Area Ratio",
            summary: "Calculates the ratio of 3D surface area to planimetric area using the Jenness (2004) method.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input DEM raster path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output path. If omitted, output remains in memory.",
                    required: false,
                },
            ],
        }
    }

    fn surface_area_ratio_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));
        example_args.insert("output".to_string(), json!("surface_area_ratio.tif"));

        ToolManifest {
            id: "surface_area_ratio".to_string(),
            display_name: "Surface Area Ratio".to_string(),
            summary: "Calculates the ratio of 3D surface area to planimetric area using the Jenness (2004) method.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input DEM raster path or typed raster object.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output path. If omitted, result is stored in memory.".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_surface_area_ratio".to_string(),
                description: "Compute surface area ratio from a DEM.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "geomorphometry".to_string(),
                "terrain".to_string(),
                "surface-area".to_string(),
                "jenness".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn run_surface_area_ratio(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        // Jenness (2004) method: 8 triangular facets sharing the centre cell.
        // Neighbours are labelled 0..8 in reading order:
        //   0=NW  1=N  2=NE
        //   3=W   4=C  5=E
        //   6=SW  7=S  8=SE
        //
        // 16 distance pairs (half 3D edge lengths between adjacent nodes):
        //   each distance = sqrt(planar^2 + zdiff^2) / 2
        //
        // DIST_PAIRS[i] = [from_idx, to_idx] into the 9-cell neighbourhood.
        // TRIANGLE_SIDES[t] = [d0, d1, d2] indices into the 16 distances.

        // Horizontal/vertical/diagonal neighbour pairs (Jenness 2004, Table 1)
        const DIST_PAIRS: [[usize; 2]; 16] = [
            [0, 1], [1, 2], [3, 4], [4, 5], [6, 7], [7, 8], // h/v pairs
            [0, 3], [1, 4], [2, 5], [3, 6], [4, 7], [5, 8], // h/v pairs (N-S axis)
            [0, 4], [1, 5], [3, 7], [4, 8], // diagonal pairs
        ];

        // 8 triangles; each entry is three indices into DIST_PAIRS distances.
        const TRIANGLE_SIDES: [[usize; 3]; 8] = [
            [6, 7, 12],  // NW triangle
            [7, 8, 13],  // N  triangle
            [8, 9, 15],  // NE triangle (corrected)
            [0, 7, 12],  // W  triangle
            [1, 8, 13],  // E  triangle
            [3, 7, 14],  // SW triangle (corrected)
            [4, 8, 15],  // S  triangle (corrected)
            [5, 9, 15],  // SE triangle
        ];

        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("running surface_area_ratio");
        ctx.progress.info("reading input raster");
        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let is_geographic = Self::raster_is_geographic(&input);
        let base_res_x = input.cell_size_x.abs().max(f64::EPSILON);
        let base_res_y = input.cell_size_y.abs().max(f64::EPSILON);

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    let row = r as isize;

                    // Per-row geographic scaling
                    let (resx, resy) = if is_geographic {
                        let mid_lat = input.row_center_y(row).to_radians();
                        (base_res_x * 111_111.0 * mid_lat.cos(), base_res_y * 111_111.0)
                    } else {
                        (base_res_x, base_res_y)
                    };
                    let diag = (resx * resx + resy * resy).sqrt();
                    let cell_area = resx * resy;

                    // Planar distances between all pairs of nodes
                    let planar: [f64; 16] = [
                        resx, resx, resx, resx, resx, resx, // horizontal
                        resy, resy, resy, resy, resy, resy, // vertical
                        diag, diag, diag, diag,              // diagonal
                    ];

                    for c in 0..cols {
                        let col = c as isize;
                        // Read 3x3 neighbourhood; nodata filled with centre value
                        let z_centre = input.get(band, row, col);
                        if input.is_nodata(z_centre) {
                            continue;
                        }
                        let offsets: [(isize, isize); 9] = [
                            (-1, -1), (0, -1), (1, -1),
                            (-1,  0), (0,  0), (1,  0),
                            (-1,  1), (0,  1), (1,  1),
                        ];
                        let mut z = [0.0f64; 9];
                        let mut nodata_flags = [false; 9];
                        for (i, (ox, oy)) in offsets.iter().enumerate() {
                            let v = input.get(band, row + oy, col + ox);
                            if input.is_nodata(v) {
                                z[i] = z_centre;
                                nodata_flags[i] = true;
                            } else {
                                z[i] = v;
                            }
                        }

                        // Compute 16 half-3D distances
                        let mut d = [0.0f64; 16];
                        for (k, pair) in DIST_PAIRS.iter().enumerate() {
                            let zdiff = z[pair[1]] - z[pair[0]];
                            d[k] = (planar[k] * planar[k] + zdiff * zdiff).sqrt() / 2.0;
                        }

                        // Heron's formula for each of the 8 triangles
                        let mut total_area = 0.0_f64;
                        let mut valid_triangles = 0_usize;
                        for sides in TRIANGLE_SIDES.iter() {
                            // Skip triangles involving nodata-flagged non-centre nodes
                            // (centre is always index 4; we flag non-centre nodata)
                            let p = d[sides[0]];
                            let q = d[sides[1]];
                            let r_side = d[sides[2]];
                            let s = (p + q + r_side) / 2.0;
                            let area_sq = s * (s - p) * (s - q) * (s - r_side);
                            if area_sq > 0.0 {
                                total_area += area_sq.sqrt();
                                valid_triangles += 1;
                            }
                        }

                        if valid_triangles == 0 || cell_area == 0.0 {
                            continue;
                        }

                        // Adjust denominator for missing triangles (nodata corners)
                        let missing = nodata_flags.iter().filter(|&&f| f).count();
                        let effective_area =
                            (cell_area - missing as f64 * cell_area / 8.0).max(f64::EPSILON);
                        row_out[c] = total_area / effective_area;
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
        Ok(Self::build_result(output_locator))
    }

    // -----------------------------------------------------------------------
    // Elevation Relative to Min/Max
    // -----------------------------------------------------------------------

    fn elev_relative_to_min_max_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "elev_relative_to_min_max",
            display_name: "Elevation Relative to Min/Max",
            summary: "Expresses each elevation as a percentage (0–100) of the raster's elevation range.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input DEM raster path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output path. If omitted, output remains in memory.",
                    required: false,
                },
            ],
        }
    }

    fn elev_relative_to_min_max_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));
        example_args.insert("output".to_string(), json!("elev_relative.tif"));

        ToolManifest {
            id: "elev_relative_to_min_max".to_string(),
            display_name: "Elevation Relative to Min/Max".to_string(),
            summary: "Expresses each elevation as a percentage (0–100) of the raster's elevation range."
                .to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input DEM raster path or typed raster object.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output path. If omitted, result is stored in memory.".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_elev_relative".to_string(),
                description: "Normalise a DEM to 0–100%.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "geomorphometry".to_string(),
                "terrain".to_string(),
                "normalisation".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn run_elev_relative_to_min_max(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("running elev_relative_to_min_max");
        ctx.progress.info("reading input raster");
        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        for band_idx in 0..bands {
            let band = band_idx as isize;

            // Parallel min/max reduce
            let (min_val, max_val) = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut mn = f64::INFINITY;
                    let mut mx = f64::NEG_INFINITY;
                    for c in 0..cols {
                        let v = input.get(band, r as isize, c as isize);
                        if !input.is_nodata(v) {
                            if v < mn { mn = v; }
                            if v > mx { mx = v; }
                        }
                    }
                    (mn, mx)
                })
                .reduce(
                    || (f64::INFINITY, f64::NEG_INFINITY),
                    |(mn1, mx1), (mn2, mx2)| (mn1.min(mn2), mx1.max(mx2)),
                );

            if min_val.is_infinite() || max_val.is_infinite() {
                // All nodata — leave output as-is
                continue;
            }
            let range = (max_val - min_val).max(f64::EPSILON);

            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    let row = r as isize;
                    for c in 0..cols {
                        let v = input.get(band, row, c as isize);
                        if !input.is_nodata(v) {
                            row_out[c] = (v - min_val) / range * 100.0;
                        }
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
        Ok(Self::build_result(output_locator))
    }

    // -----------------------------------------------------------------------
    // Wetness Index (TWI)
    // -----------------------------------------------------------------------

    fn wetness_index_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "wetness_index",
            display_name: "Wetness Index",
            summary: "Calculates the topographic wetness index ln(SCA / tan(slope)).",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "sca",
                    description: "Specific catchment area (SCA) raster path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "slope",
                    description: "Slope raster in degrees.",
                    required: true,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output path. If omitted, output remains in memory.",
                    required: false,
                },
            ],
        }
    }

    fn wetness_index_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("sca".to_string(), json!("sca.tif"));
        defaults.insert("slope".to_string(), json!("slope.tif"));

        let mut example_args = ToolArgs::new();
        example_args.insert("sca".to_string(), json!("sca.tif"));
        example_args.insert("slope".to_string(), json!("slope.tif"));
        example_args.insert("output".to_string(), json!("twi.tif"));

        ToolManifest {
            id: "wetness_index".to_string(),
            display_name: "Wetness Index".to_string(),
            summary: "Calculates the topographic wetness index ln(SCA / tan(slope))."
                .to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "sca".to_string(),
                    description: "Specific catchment area raster path or typed raster object."
                        .to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "slope".to_string(),
                    description: "Slope raster in degrees.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output path. If omitted, result is stored in memory."
                        .to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_twi".to_string(),
                description: "Compute TWI from SCA and slope rasters.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "geomorphometry".to_string(),
                "terrain".to_string(),
                "hydrology".to_string(),
                "wetness".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn run_wetness_index(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("running wetness_index");
        ctx.progress.info("reading SCA raster");
        let sca = Self::load_named_raster(args, "sca")?;
        ctx.progress.info("reading slope raster");
        let slope = Self::load_named_raster(args, "slope")?;

        if sca.rows != slope.rows || sca.cols != slope.cols {
            return Err(ToolError::Validation(
                "sca and slope rasters must have the same dimensions".to_string(),
            ));
        }

        let mut output = sca.clone();
        let rows = sca.rows;
        let cols = sca.cols;
        let bands = sca.bands;
        let nodata = sca.nodata;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    let row = r as isize;
                    for c in 0..cols {
                        let col = c as isize;
                        let sca_val = sca.get(band, row, col);
                        let slope_val = slope.get(band, row, col);
                        if sca.is_nodata(sca_val) || slope.is_nodata(slope_val) {
                            continue;
                        }
                        let slope_rad = slope_val.to_radians();
                        let tan_slope = slope_rad.tan();
                        if tan_slope <= 0.0 || sca_val <= 0.0 {
                            // Undefined: flat areas or negative SCA → nodata
                            continue;
                        }
                        row_out[c] = (sca_val / tan_slope).ln();
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
        Ok(Self::build_result(output_locator))
    }
}

// ---------------------------------------------------------------------------
// Tool trait implementations
// ---------------------------------------------------------------------------

impl Tool for RuggednessIndexTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainAnalysisCore::ruggedness_index_metadata()
    }

    fn manifest(&self) -> ToolManifest {
        TerrainAnalysisCore::ruggedness_index_manifest()
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_ruggedness_index(args, ctx)
    }
}

impl Tool for SurfaceAreaRatioTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainAnalysisCore::surface_area_ratio_metadata()
    }

    fn manifest(&self) -> ToolManifest {
        TerrainAnalysisCore::surface_area_ratio_manifest()
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_surface_area_ratio(args, ctx)
    }
}

impl Tool for ElevRelativeToMinMaxTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainAnalysisCore::elev_relative_to_min_max_metadata()
    }

    fn manifest(&self) -> ToolManifest {
        TerrainAnalysisCore::elev_relative_to_min_max_manifest()
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_elev_relative_to_min_max(args, ctx)
    }
}

impl Tool for WetnessIndexTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainAnalysisCore::wetness_index_metadata()
    }

    fn manifest(&self) -> ToolManifest {
        TerrainAnalysisCore::wetness_index_manifest()
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_wetness_index(args, ctx)
    }
}

impl Tool for PercentElevRangeTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::percent_elev_range_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::percent_elev_range_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_percent_elev_range(args, ctx)
    }
}

impl Tool for RelativeTopographicPositionTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::relative_topographic_position_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::relative_topographic_position_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_relative_topographic_position(args, ctx)
    }
}

impl Tool for NumDownslopeNeighboursTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::num_downslope_neighbours_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::num_downslope_neighbours_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_num_downslope_neighbours(args, ctx)
    }
}

impl Tool for NumUpslopeNeighboursTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::num_upslope_neighbours_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::num_upslope_neighbours_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_num_upslope_neighbours(args, ctx)
    }
}

impl Tool for MaxDownslopeElevChangeTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::max_downslope_elev_change_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::max_downslope_elev_change_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_max_downslope_elev_change(args, ctx)
    }
}

impl Tool for MaxUpslopeElevChangeTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::max_upslope_elev_change_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::max_upslope_elev_change_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_max_upslope_elev_change(args, ctx)
    }
}

impl Tool for MinDownslopeElevChangeTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::min_downslope_elev_change_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::min_downslope_elev_change_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_min_downslope_elev_change(args, ctx)
    }
}

impl Tool for ElevationPercentileTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::elevation_percentile_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::elevation_percentile_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_elevation_percentile(args, ctx)
    }
}

impl Tool for DownslopeIndexTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::downslope_index_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::downslope_index_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        let vertical_drop = args
            .get("vertical_drop")
            .or_else(|| args.get("drop"))
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("missing required numeric parameter 'vertical_drop'".to_string()))?;
        if vertical_drop <= 0.0 {
            return Err(ToolError::Validation("parameter 'vertical_drop' must be > 0".to_string()));
        }
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_downslope_index(args, ctx)
    }
}

impl Tool for MaxBranchLengthTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::max_branch_length_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::max_branch_length_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_max_branch_length(args, ctx)
    }
}

impl Tool for ElevAbovePitTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::elev_above_pit_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::elev_above_pit_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_elev_above_pit(args, ctx)
    }
}

impl Tool for DirectionalReliefTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::directional_relief_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::directional_relief_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_directional_relief(args, ctx)
    }
}

impl Tool for ExposureTowardsWindFluxTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::exposure_towards_wind_flux_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::exposure_towards_wind_flux_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        if let Some(v) = args.get("max_dist").and_then(|v| v.as_f64()) {
            if !(v.is_infinite() || v > 0.0) {
                return Err(ToolError::Validation(
                    "parameter 'max_dist' must be positive when provided".to_string(),
                ));
            }
        }
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_exposure_towards_wind_flux(args, ctx)
    }
}

impl Tool for RelativeAspectTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::relative_aspect_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::relative_aspect_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_relative_aspect(args, ctx)
    }
}

impl Tool for EdgeDensityTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::edge_density_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::edge_density_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_edge_density(args, ctx)
    }
}

impl Tool for SphericalStdDevOfNormalsTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::spherical_std_dev_of_normals_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::spherical_std_dev_of_normals_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_spherical_std_dev_of_normals(args, ctx)
    }
}

impl Tool for AverageNormalVectorAngularDeviationTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::average_normal_vector_angular_deviation_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::average_normal_vector_angular_deviation_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_average_normal_vector_angular_deviation(args, ctx)
    }
}

impl Tool for HypsometricAnalysisTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::hypsometric_analysis_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::hypsometric_analysis_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_raster_input_list(args, "inputs")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_hypsometric_analysis(args, ctx)
    }
}

impl Tool for ProfileTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainAnalysisCore::profile_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainAnalysisCore::profile_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "lines_vector")
            .or_else(|_| parse_vector_path_arg(args, "lines"))
            .or_else(|_| parse_vector_path_arg(args, "input"))?;
        let _ = parse_raster_path_arg(args, "surface")
            .or_else(|_| parse_raster_path_arg(args, "dem"))
            .or_else(|_| parse_raster_path_arg(args, "input_surface"))?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_profile(args, ctx)
    }
}

impl Tool for SlopeVsAspectPlotTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::slope_vs_aspect_plot_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::slope_vs_aspect_plot_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        if let Some(v) = args
            .get("aspect_bin_size")
            .or_else(|| args.get("bin_size"))
            .and_then(|v| v.as_f64())
        {
            if v <= 0.0 {
                return Err(ToolError::Validation(
                    "parameter 'aspect_bin_size' must be greater than 0".to_string(),
                ));
            }
        }
        if let Some(v) = args.get("min_slope").and_then(|v| v.as_f64()) {
            if v < 0.0 {
                return Err(ToolError::Validation(
                    "parameter 'min_slope' must be >= 0".to_string(),
                ));
            }
        }
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_slope_vs_aspect_plot(args, ctx)
    }
}

impl Tool for SlopeVsElevPlotTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::slope_vs_elev_plot_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::slope_vs_elev_plot_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_raster_input_list(args, "inputs")?;
        let _ = parse_optional_output_path(args, "output")?;
        if let Some(ws) = args.get("watershed") {
            let _ = ws;
            let inputs = TerrainAnalysisCore::parse_raster_input_list(args, "inputs")?;
            let watersheds = TerrainAnalysisCore::parse_raster_input_list(args, "watershed")?;
            if watersheds.len() != inputs.len() {
                return Err(ToolError::Validation(
                    "watershed list length must match inputs list length".to_string(),
                ));
            }
        }
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_slope_vs_elev_plot(args, ctx)
    }
}

impl Tool for ElevAbovePitDistTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::elev_above_pit_dist_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::elev_above_pit_dist_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_elev_above_pit_dist(args, ctx)
    }
}

impl Tool for CircularVarianceOfAspectTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::circular_variance_of_aspect_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::circular_variance_of_aspect_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_circular_variance_of_aspect(args, ctx)
    }
}

impl Tool for FetchAnalysisTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::fetch_analysis_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::fetch_analysis_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_fetch_analysis(args, ctx)
    }
}

impl Tool for FindRidgesTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::find_ridges_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::find_ridges_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_find_ridges(args, ctx)
    }
}

impl Tool for GeomorphonsTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::geomorphons_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::geomorphons_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        if let Some(v) = args
            .get("search_distance")
            .or_else(|| args.get("search"))
            .and_then(|v| v.as_u64())
        {
            if v == 0 {
                return Err(ToolError::Validation("parameter 'search_distance' must be >= 1".to_string()));
            }
        }
        if let Some(v) = args
            .get("flatness_threshold")
            .or_else(|| args.get("threshold"))
            .and_then(|v| v.as_f64())
        {
            if v < 0.0 {
                return Err(ToolError::Validation("parameter 'flatness_threshold' must be >= 0".to_string()));
            }
        }
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_geomorphons(args, ctx)
    }
}

impl Tool for PennockLandformClassificationTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::pennock_landform_classification_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::pennock_landform_classification_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        if let Some(v) = args.get("slope_threshold").and_then(|v| v.as_f64()) {
            if v < 0.0 {
                return Err(ToolError::Validation("parameter 'slope_threshold' must be >= 0".to_string()));
            }
        }
        if let Some(v) = args.get("prof_curv_threshold").and_then(|v| v.as_f64()) {
            if v < 0.0 {
                return Err(ToolError::Validation("parameter 'prof_curv_threshold' must be >= 0".to_string()));
            }
        }
        if let Some(v) = args.get("plan_curv_threshold").and_then(|v| v.as_f64()) {
            if v < 0.0 {
                return Err(ToolError::Validation("parameter 'plan_curv_threshold' must be >= 0".to_string()));
            }
        }
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_pennock_landform_classification(args, ctx)
    }
}

impl Tool for ViewshedTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::viewshed_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::viewshed_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_vector_path_arg(args, "stations")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_viewshed(args, ctx)
    }
}

impl Tool for AssessRouteTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainAnalysisCore::assess_route_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainAnalysisCore::assess_route_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "routes").or_else(|_| parse_vector_path_arg(args, "input"))?;
        let _ = parse_raster_path_arg(args, "dem").or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
        let _ = parse_optional_output_path(args, "output")?;
        if let Some(v) = args.get("segment_length").or_else(|| args.get("length")).and_then(|v| v.as_f64()) {
            if !v.is_finite() || v <= 0.0 {
                return Err(ToolError::Validation(
                    "parameter 'segment_length' must be a positive finite number".to_string(),
                ));
            }
        }
        if let Some(v) = args
            .get("search_radius")
            .or_else(|| args.get("dist"))
            .and_then(|v| v.as_u64())
        {
            if v == 0 {
                return Err(ToolError::Validation(
                    "parameter 'search_radius' must be >= 1".to_string(),
                ));
            }
        }
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_assess_route(args, ctx)
    }
}

impl Tool for BreaklineMappingTool {
    fn metadata(&self) -> ToolMetadata { TerrainAnalysisCore::breakline_mapping_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainAnalysisCore::breakline_mapping_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainAnalysisCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        if let Some(v) = args.get("threshold").and_then(|v| v.as_f64()) {
            if !v.is_finite() || v < 0.0 {
                return Err(ToolError::Validation(
                    "parameter 'threshold' must be a non-negative finite number".to_string(),
                ));
            }
        }
        if let Some(v) = args.get("min_length").and_then(|v| v.as_u64()) {
            if v == 0 {
                return Err(ToolError::Validation(
                    "parameter 'min_length' must be >= 1".to_string(),
                ));
            }
        }
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_breakline_mapping(args, ctx)
    }
}

impl Tool for LowPointsOnHeadwaterDividesTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainAnalysisCore::low_points_on_headwater_divides_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainAnalysisCore::low_points_on_headwater_divides_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "dem").or_else(|_| parse_raster_path_arg(args, "input"))?;
        let _ = parse_raster_path_arg(args, "streams").or_else(|_| parse_raster_path_arg(args, "streams_raster"))?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainAnalysisCore::run_low_points_on_headwater_divides(args, ctx)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
            cell_size: 10.0,
            ..Default::default()
        };
        let mut raster = Raster::new(cfg);
        for row in 0..rows as isize {
            for col in 0..cols as isize {
                raster.set(0, row, col, value).unwrap();
            }
        }
        raster
    }

    fn make_constant_raster_with_cell_sizes(
        rows: usize,
        cols: usize,
        value: f64,
        cell_size_x: f64,
        cell_size_y: Option<f64>,
    ) -> Raster {
        let cfg = RasterConfig {
            rows,
            cols,
            bands: 1,
            nodata: -9999.0,
            cell_size: cell_size_x,
            cell_size_y,
            ..Default::default()
        };
        let mut raster = Raster::new(cfg);
        for row in 0..rows as isize {
            for col in 0..cols as isize {
                raster.set(0, row, col, value).unwrap();
            }
        }
        raster
    }

    fn make_ramp_raster(rows: usize, cols: usize) -> Raster {
        let cfg = RasterConfig {
            rows,
            cols,
            bands: 1,
            nodata: -9999.0,
            cell_size: 10.0,
            ..Default::default()
        };
        let mut raster = Raster::new(cfg);
        for row in 0..rows as isize {
            for col in 0..cols as isize {
                raster.set(0, row, col, row as f64 * cols as f64 + col as f64).unwrap();
            }
        }
        raster
    }

    fn run_single_input_tool(tool: &dyn Tool, input: Raster) -> Raster {
        let id = memory_store::put_raster(input);
        let input_path = memory_store::make_raster_memory_path(&id);
        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(input_path));
        let result = tool.run(&args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap();
        let out_id = memory_store::raster_path_to_id(out_path).unwrap();
        memory_store::get_raster_by_id(out_id).unwrap()
    }

    fn run_downslope_index_tool(input: Raster, vertical_drop: f64, output_type: &str) -> Raster {
        let id = memory_store::put_raster(input);
        let input_path = memory_store::make_raster_memory_path(&id);
        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(input_path));
        args.insert("vertical_drop".to_string(), json!(vertical_drop));
        args.insert("output_type".to_string(), json!(output_type));
        let result = DownslopeIndexTool.run(&args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap();
        let out_id = memory_store::raster_path_to_id(out_path).unwrap();
        memory_store::get_raster_by_id(out_id).unwrap()
    }

    fn run_max_branch_length_tool(input: Raster, log_transform: bool) -> Raster {
        let id = memory_store::put_raster(input);
        let input_path = memory_store::make_raster_memory_path(&id);
        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(input_path));
        args.insert("log_transform".to_string(), json!(log_transform));
        let result = MaxBranchLengthTool.run(&args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap();
        let out_id = memory_store::raster_path_to_id(out_path).unwrap();
        memory_store::get_raster_by_id(out_id).unwrap()
    }

    #[test]
    fn ruggedness_index_flat_raster_returns_zero() {
        let out = run_single_input_tool(&RuggednessIndexTool, make_constant_raster(7, 7, 100.0));
        let v = out.get(0, 3, 3);
        assert!(v.abs() < 1e-10, "expected 0.0, got {v}");
    }

    #[test]
    fn surface_area_ratio_flat_raster_is_one() {
        let out = run_single_input_tool(&SurfaceAreaRatioTool, make_constant_raster(7, 7, 50.0));
        let v = out.get(0, 3, 3);
        assert!((v - 1.0).abs() < 1e-10, "expected 1.0, got {v}");
    }

    #[test]
    fn elev_relative_to_min_max_spans_zero_to_hundred() {
        let out = run_single_input_tool(&ElevRelativeToMinMaxTool, make_ramp_raster(5, 5));
        let min_v = out.get(0, 0, 0);
        let max_v = out.get(0, 4, 4);
        assert!(min_v.abs() < 1e-10, "expected min 0.0, got {min_v}");
        assert!((max_v - 100.0).abs() < 1e-10, "expected max 100.0, got {max_v}");
    }

    #[test]
    fn wetness_index_matches_expected_formula() {
        let sca_id = memory_store::put_raster(make_constant_raster(7, 7, std::f64::consts::E));
        let slope_id = memory_store::put_raster(make_constant_raster(7, 7, 45.0));
        let mut args = ToolArgs::new();
        args.insert(
            "sca".to_string(),
            json!(memory_store::make_raster_memory_path(&sca_id)),
        );
        args.insert(
            "slope".to_string(),
            json!(memory_store::make_raster_memory_path(&slope_id)),
        );
        let result = WetnessIndexTool.run(&args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap();
        let out_id = memory_store::raster_path_to_id(out_path).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        let v = out.get(0, 3, 3);
        assert!((v - 1.0).abs() < 1e-6, "expected 1.0, got {v}");
    }

    #[test]
    fn num_downslope_neighbours_peak_center_is_eight() {
        let mut dem = make_constant_raster(5, 5, 0.0);
        dem.set(0, 2, 2, 10.0).unwrap();
        let out = run_single_input_tool(&NumDownslopeNeighboursTool, dem);
        assert!((out.get(0, 2, 2) - 8.0).abs() < 1e-10);
    }

    #[test]
    fn num_upslope_neighbours_pit_center_is_eight() {
        let mut dem = make_constant_raster(5, 5, 10.0);
        dem.set(0, 2, 2, 0.0).unwrap();
        let out = run_single_input_tool(&NumUpslopeNeighboursTool, dem);
        assert!((out.get(0, 2, 2) - 8.0).abs() < 1e-10);
    }

    #[test]
    fn downslope_elev_change_flat_surface_is_zero() {
        let dem = make_constant_raster(5, 5, 10.0);
        let out_max = run_single_input_tool(&MaxDownslopeElevChangeTool, dem.clone());
        let out_min = run_single_input_tool(&MinDownslopeElevChangeTool, dem);
        assert!(out_max.get(0, 2, 2).abs() < 1e-10);
        assert!(out_min.get(0, 2, 2).abs() < 1e-10);
    }

    #[test]
    fn max_upslope_elev_change_flat_surface_is_zero() {
        let dem = make_constant_raster(5, 5, 10.0);
        let out_max = run_single_input_tool(&MaxUpslopeElevChangeTool, dem);
        assert!(out_max.get(0, 2, 2).abs() < 1e-10);
    }

    #[test]
    fn elev_above_pit_flat_surface_is_zero() {
        let dem = make_constant_raster(5, 5, 10.0);
        let out = run_single_input_tool(&ElevAbovePitTool, dem);
        assert!(out.get(0, 2, 2).abs() < 1e-10);
    }

    #[test]
    fn directional_relief_flat_surface_is_zero() {
        let dem = make_constant_raster(7, 7, 10.0);
        let id = memory_store::put_raster(dem);
        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&id)));
        args.insert("azimuth".to_string(), json!(315.0));
        let result = DirectionalReliefTool.run(&args, &make_ctx()).unwrap();
        let out_id = memory_store::raster_path_to_id(result.outputs.get("path").unwrap().as_str().unwrap()).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        assert!(out.get(0, 3, 3).abs() < 1e-10);
    }

    #[test]
    fn exposure_towards_wind_flux_flat_surface_is_zero() {
        let dem = make_constant_raster(7, 7, 10.0);
        let id = memory_store::put_raster(dem);
        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&id)));
        args.insert("azimuth".to_string(), json!(315.0));
        args.insert("max_dist".to_string(), json!(100.0));
        let result = ExposureTowardsWindFluxTool.run(&args, &make_ctx()).unwrap();
        let out_id =
            memory_store::raster_path_to_id(result.outputs.get("path").unwrap().as_str().unwrap())
                .unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        assert!(out.get(0, 3, 3).abs() < 1e-2);
    }

    #[test]
    fn relative_aspect_flat_surface_is_undefined() {
        let dem = make_constant_raster(7, 7, 10.0);
        let id = memory_store::put_raster(dem);
        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&id)));
        args.insert("azimuth".to_string(), json!(180.0));
        let result = RelativeAspectTool.run(&args, &make_ctx()).unwrap();
        let out_id = memory_store::raster_path_to_id(result.outputs.get("path").unwrap().as_str().unwrap()).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        assert!((out.get(0, 3, 3) + 1.0).abs() < 1e-10);
    }

    #[test]
    fn edge_density_flat_surface_is_zero() {
        let dem = make_constant_raster(7, 7, 10.0);
        let id = memory_store::put_raster(dem);
        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&id)));
        args.insert("filter_size".to_string(), json!(5));
        args.insert("norm_diff".to_string(), json!(5.0));
        let result = EdgeDensityTool.run(&args, &make_ctx()).unwrap();
        let out_id = memory_store::raster_path_to_id(result.outputs.get("path").unwrap().as_str().unwrap()).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        assert!(out.get(0, 3, 3).abs() < 1e-10);
    }

    #[test]
    fn spherical_std_dev_of_normals_flat_surface_is_zero() {
        let dem = make_constant_raster(9, 9, 10.0);
        let id = memory_store::put_raster(dem);
        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&id)));
        args.insert("filter_size".to_string(), json!(5));
        let result = SphericalStdDevOfNormalsTool.run(&args, &make_ctx()).unwrap();
        let out_id = memory_store::raster_path_to_id(result.outputs.get("path").unwrap().as_str().unwrap()).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        assert!(out.get(0, 4, 4).abs() < 1e-8);
    }

    #[test]
    fn average_normal_vector_angular_deviation_flat_surface_is_zero() {
        let dem = make_constant_raster(9, 9, 10.0);
        let id = memory_store::put_raster(dem);
        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&id)));
        args.insert("filter_size".to_string(), json!(5));
        let result = AverageNormalVectorAngularDeviationTool.run(&args, &make_ctx()).unwrap();
        let out_id = memory_store::raster_path_to_id(result.outputs.get("path").unwrap().as_str().unwrap()).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        assert!(out.get(0, 4, 4).abs() < 1e-8);
    }

    #[test]
    fn elev_above_pit_dist_alias_flat_surface_is_zero() {
        let dem = make_constant_raster(5, 5, 10.0);
        let out = run_single_input_tool(&ElevAbovePitDistTool, dem);
        assert!(out.get(0, 2, 2).abs() < 1e-10);
    }

    #[test]
    fn hypsometric_analysis_writes_html_report() {
        let dem = make_ramp_raster(8, 8);
        let id = memory_store::put_raster(dem);
        let in_path = memory_store::make_raster_memory_path(&id);
        let out_path = std::env::temp_dir().join("hypsometric_analysis_test.html");

        let mut args = ToolArgs::new();
        args.insert("inputs".to_string(), json!(in_path));
        args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
        let result = HypsometricAnalysisTool.run(&args, &make_ctx()).unwrap();
        let p = result.outputs.get("path").unwrap().as_str().unwrap();
        assert!(std::path::Path::new(p).exists());
        let html = std::fs::read_to_string(p).unwrap();
        assert!(html.contains("Hypsometric Analysis"));
    }

    #[test]
    fn slope_vs_aspect_plot_writes_html_report() {
        let dem = make_ramp_raster(16, 16);
        let id = memory_store::put_raster(dem);
        let in_path = memory_store::make_raster_memory_path(&id);
        let out_path = std::env::temp_dir().join("slope_vs_aspect_plot_test.html");

        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(in_path));
        args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
        let result = SlopeVsAspectPlotTool.run(&args, &make_ctx()).unwrap();
        let p = result.outputs.get("path").unwrap().as_str().unwrap();
        assert!(std::path::Path::new(p).exists());
        let html = std::fs::read_to_string(p).unwrap();
        assert!(html.contains("Slope vs Aspect"));
    }

    #[test]
    fn slope_vs_elev_plot_writes_html_report() {
        let dem = make_ramp_raster(16, 16);
        let id = memory_store::put_raster(dem);
        let in_path = memory_store::make_raster_memory_path(&id);
        let out_path = std::env::temp_dir().join("slope_vs_elev_plot_test.html");

        let mut args = ToolArgs::new();
        args.insert("inputs".to_string(), json!(vec![in_path]));
        args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));
        let result = SlopeVsElevPlotTool.run(&args, &make_ctx()).unwrap();
        let p = result.outputs.get("path").unwrap().as_str().unwrap();
        assert!(std::path::Path::new(p).exists());
        let html = std::fs::read_to_string(p).unwrap();
        assert!(html.contains("Slope-Elevation Analysis"));
    }

    #[test]
    fn percent_and_rtp_center_peak_positive() {
        let mut dem = make_constant_raster(5, 5, 0.0);
        dem.set(0, 2, 2, 10.0).unwrap();
        let id = memory_store::put_raster(dem);
        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&id)));
        args.insert("filter_size_x".to_string(), json!(3));
        args.insert("filter_size_y".to_string(), json!(3));

        let r1 = PercentElevRangeTool.run(&args, &make_ctx()).unwrap();
        let id1 = memory_store::raster_path_to_id(r1.outputs.get("path").unwrap().as_str().unwrap()).unwrap();
        let out1 = memory_store::get_raster_by_id(id1).unwrap();
        assert!((out1.get(0, 2, 2) - 100.0).abs() < 1e-10);

        let r2 = RelativeTopographicPositionTool.run(&args, &make_ctx()).unwrap();
        let id2 = memory_store::raster_path_to_id(r2.outputs.get("path").unwrap().as_str().unwrap()).unwrap();
        let out2 = memory_store::get_raster_by_id(id2).unwrap();
        assert!((out2.get(0, 2, 2) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn geomorphons_flat_surface_returns_flat_class() {
        let input = make_constant_raster(7, 7, 100.0);
        let id = memory_store::put_raster(input);
        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&id)));
        args.insert("search_distance".to_string(), json!(3));
        args.insert("output_forms".to_string(), json!(true));
        let result = GeomorphonsTool.run(&args, &make_ctx()).unwrap();
        let out_id = memory_store::raster_path_to_id(result.outputs.get("path").unwrap().as_str().unwrap()).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        assert_eq!(out.data_type, DataType::I16);
        assert!((out.get(0, 3, 3) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn geomorphons_flat_surface_raw_code_is_canonical_flat() {
        let input = make_constant_raster(7, 7, 100.0);
        let id = memory_store::put_raster(input);
        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&id)));
        args.insert("search_distance".to_string(), json!(3));
        args.insert("output_forms".to_string(), json!(false));
        let result = GeomorphonsTool.run(&args, &make_ctx()).unwrap();
        let out_id = memory_store::raster_path_to_id(result.outputs.get("path").unwrap().as_str().unwrap()).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        assert!((out.get(0, 3, 3) - 3280.0).abs() < 1e-10);
    }

    #[test]
    fn geomorphons_direction_classification_uses_zenith_nadir_difference() {
        let mut input = make_constant_raster(9, 9, -9999.0);
        input.set(0, 4, 4, 0.0).unwrap();
        input.set(0, 4, 5, 10.0).unwrap();
        let id = memory_store::put_raster(input);
        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&id)));
        args.insert("search_distance".to_string(), json!(1));
        args.insert("flatness_threshold".to_string(), json!(0.0));
        args.insert("output_forms".to_string(), json!(false));

        let result = GeomorphonsTool.run(&args, &make_ctx()).unwrap();
        let out_id = memory_store::raster_path_to_id(result.outputs.get("path").unwrap().as_str().unwrap()).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        let center_code = out.get(0, 4, 4);
        assert!((center_code - 3280.0).abs() > 1e-10, "expected non-flat code, got {center_code}");
    }

    #[test]
    fn geomorphons_search_distance_includes_endpoint_cell() {
        let mut input = make_constant_raster(11, 11, -9999.0);
        input.set(0, 5, 5, 0.0).unwrap();
        input.set(0, 5, 7, 10.0).unwrap();
        let id = memory_store::put_raster(input);
        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&id)));
        args.insert("search_distance".to_string(), json!(2));
        args.insert("flatness_threshold".to_string(), json!(0.0));
        args.insert("output_forms".to_string(), json!(false));

        let result = GeomorphonsTool.run(&args, &make_ctx()).unwrap();
        let out_id = memory_store::raster_path_to_id(result.outputs.get("path").unwrap().as_str().unwrap()).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        let center_code = out.get(0, 5, 5);
        assert!((center_code - 3280.0).abs() > 1e-10, "expected endpoint sample to influence classification, got {center_code}");
    }

    #[test]
    fn geomorphons_cell_distance_behavior_is_consistent_for_anisotropic_pixels() {
        let mut isotropic = make_constant_raster_with_cell_sizes(11, 11, -9999.0, 10.0, Some(10.0));
        isotropic.set(0, 5, 5, 0.0).unwrap();
        isotropic.set(0, 3, 5, 10.0).unwrap();
        isotropic.set(0, 7, 5, 10.0).unwrap();

        let mut anisotropic = make_constant_raster_with_cell_sizes(11, 11, -9999.0, 10.0, Some(20.0));
        anisotropic.set(0, 5, 5, 0.0).unwrap();
        anisotropic.set(0, 3, 5, 10.0).unwrap();
        anisotropic.set(0, 7, 5, 10.0).unwrap();

        let iso_id = memory_store::put_raster(isotropic);
        let aniso_id = memory_store::put_raster(anisotropic);

        let mut iso_args = ToolArgs::new();
        iso_args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&iso_id)));
        iso_args.insert("search_distance".to_string(), json!(2));
        iso_args.insert("flatness_threshold".to_string(), json!(0.0));
        iso_args.insert("output_forms".to_string(), json!(false));

        let mut aniso_args = ToolArgs::new();
        aniso_args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&aniso_id)));
        aniso_args.insert("search_distance".to_string(), json!(2));
        aniso_args.insert("flatness_threshold".to_string(), json!(0.0));
        aniso_args.insert("output_forms".to_string(), json!(false));

        let iso_result = GeomorphonsTool.run(&iso_args, &make_ctx()).unwrap();
        let iso_out_id = memory_store::raster_path_to_id(iso_result.outputs.get("path").unwrap().as_str().unwrap()).unwrap();
        let iso_out = memory_store::get_raster_by_id(iso_out_id).unwrap();

        let aniso_result = GeomorphonsTool.run(&aniso_args, &make_ctx()).unwrap();
        let aniso_out_id = memory_store::raster_path_to_id(aniso_result.outputs.get("path").unwrap().as_str().unwrap()).unwrap();
        let aniso_out = memory_store::get_raster_by_id(aniso_out_id).unwrap();

        let iso_code = iso_out.get(0, 5, 5);
        let aniso_code = aniso_out.get(0, 5, 5);
        assert!((iso_code - aniso_code).abs() < 1e-10, "expected consistent cell-distance behavior, isotropic={iso_code}, anisotropic={aniso_code}");
    }

    #[test]
    fn elevation_percentile_center_is_expected_for_ramp() {
        let mut args = ToolArgs::new();
        let dem = make_ramp_raster(3, 3);
        let id = memory_store::put_raster(dem);
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&id)));
        args.insert("filter_size_x".to_string(), json!(3));
        args.insert("filter_size_y".to_string(), json!(3));
        args.insert("sig_digits".to_string(), json!(2));
        let result = ElevationPercentileTool.run(&args, &make_ctx()).unwrap();
        let out_id = memory_store::raster_path_to_id(result.outputs.get("path").unwrap().as_str().unwrap()).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        let center = out.get(0, 1, 1);
        assert!((center - (4.0 / 9.0 * 100.0)).abs() < 1e-4, "unexpected percentile: {center}");
    }

    #[test]
    fn downslope_index_simple_ramp_has_positive_tangent() {
        let cfg = RasterConfig {
            rows: 1,
            cols: 3,
            bands: 1,
            nodata: -9999.0,
            cell_size: 10.0,
            ..Default::default()
        };
        let mut dem = Raster::new(cfg);
        dem.set(0, 0, 0, 3.0).unwrap();
        dem.set(0, 0, 1, 2.0).unwrap();
        dem.set(0, 0, 2, 1.0).unwrap();
        let out = run_downslope_index_tool(dem, 1.0, "tangent");
        let v = out.get(0, 0, 0);
        assert!((v - 0.1).abs() < 1e-5, "unexpected tangent: {v}");
    }

    #[test]
    fn max_branch_length_returns_positive_on_simple_convergent_surface() {
        let cfg = RasterConfig {
            rows: 2,
            cols: 2,
            bands: 1,
            nodata: -9999.0,
            cell_size: 10.0,
            ..Default::default()
        };
        let mut dem = Raster::new(cfg);
        dem.set(0, 0, 0, 4.0).unwrap();
        dem.set(0, 0, 1, 3.0).unwrap();
        dem.set(0, 1, 0, 3.0).unwrap();
        dem.set(0, 1, 1, 2.0).unwrap();

        let out = run_max_branch_length_tool(dem, false);
        assert!(out.get(0, 0, 0) > 10.0);
    }

    #[test]
    fn max_branch_length_log_transform_sets_nonpositive_to_nodata() {
        let dem = make_constant_raster(5, 5, 10.0);
        let out = run_max_branch_length_tool(dem, true);
        assert_eq!(out.get(0, 2, 2), out.nodata);
    }

    #[test]
    fn circular_variance_of_aspect_flat_surface_is_zero() {
        let dem = make_constant_raster(9, 9, 10.0);
        let id = memory_store::put_raster(dem);
        let mut args = ToolArgs::new();
        args.insert(
            "input".to_string(),
            json!(memory_store::make_raster_memory_path(&id)),
        );
        args.insert("filter".to_string(), json!(5));
        let result = CircularVarianceOfAspectTool.run(&args, &make_ctx()).unwrap();
        let out_id = memory_store::raster_path_to_id(result.outputs.get("path").unwrap().as_str().unwrap()).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        assert!(out.get(0, 4, 4).abs() < 1e-8);
    }

    #[test]
    fn fetch_analysis_flat_surface_returns_negative_fetch() {
        let dem = make_constant_raster(9, 9, 10.0);
        let id = memory_store::put_raster(dem);
        let mut args = ToolArgs::new();
        args.insert(
            "input".to_string(),
            json!(memory_store::make_raster_memory_path(&id)),
        );
        args.insert("azimuth".to_string(), json!(315.0));
        args.insert("hgt_inc".to_string(), json!(0.05));
        let result = FetchAnalysisTool.run(&args, &make_ctx()).unwrap();
        let out_id = memory_store::raster_path_to_id(result.outputs.get("path").unwrap().as_str().unwrap()).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        assert!(out.get(0, 4, 4) < 0.0);
    }

    #[test]
    fn find_ridges_detects_simple_peak() {
        let mut dem = make_constant_raster(7, 7, 0.0);
        dem.set(0, 3, 3, 10.0).unwrap();
        let id = memory_store::put_raster(dem);
        let mut args = ToolArgs::new();
        args.insert(
            "input".to_string(),
            json!(memory_store::make_raster_memory_path(&id)),
        );
        args.insert("line_thin".to_string(), json!(false));
        let result = FindRidgesTool.run(&args, &make_ctx()).unwrap();
        let out_id = memory_store::raster_path_to_id(result.outputs.get("path").unwrap().as_str().unwrap()).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        assert!((out.get(0, 3, 3) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn breakline_mapping_outputs_lines_for_simple_escarpment() {
        let cfg = RasterConfig {
            rows: 21,
            cols: 21,
            bands: 1,
            nodata: -9999.0,
            cell_size: 10.0,
            ..Default::default()
        };
        let mut dem = Raster::new(cfg);
        for r in 0..21isize {
            for c in 0..21isize {
                let z = if c < 10 { 100.0 } else { 130.0 };
                dem.set(0, r, c, z).unwrap();
            }
        }

        let id = memory_store::put_raster(dem);
        let out_path = std::env::temp_dir().join("breakline_mapping_test.shp");
        let mut args = ToolArgs::new();
        args.insert(
            "input".to_string(),
            json!(memory_store::make_raster_memory_path(&id)),
        );
        args.insert("threshold".to_string(), json!(0.5));
        args.insert("min_length".to_string(), json!(3));
        args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

        let result = BreaklineMappingTool.run(&args, &make_ctx()).unwrap();
        let p = result.outputs.get("path").unwrap().as_str().unwrap();
        assert!(std::path::Path::new(p).exists());
        let layer = wbvector::read(p).unwrap();
        assert!(!layer.features.is_empty(), "expected extracted breakline features");
    }

    #[test]
    fn assess_route_outputs_segment_attributes() {
        let cfg = RasterConfig {
            rows: 24,
            cols: 24,
            bands: 1,
            nodata: -9999.0,
            cell_size: 10.0,
            ..Default::default()
        };
        let mut dem = Raster::new(cfg);
        for r in 0..24isize {
            for c in 0..24isize {
                dem.set(0, r, c, r as f64 + c as f64).unwrap();
            }
        }
        let dem_id = memory_store::put_raster(dem);

        let mut routes = wbvector::Layer::new("routes").with_geom_type(wbvector::GeometryType::LineString);
        routes.add_field(wbvector::FieldDef::new("NAME", wbvector::FieldType::Text));
        routes
            .add_feature(
                Some(wbvector::Geometry::line_string(vec![
                    wbvector::Coord::xy(15.0, 225.0),
                    wbvector::Coord::xy(225.0, 15.0),
                ])),
                &[("NAME", wbvector::FieldValue::Text("main_route".to_string()))],
            )
            .unwrap();

        let routes_path = std::env::temp_dir().join("assess_route_input_routes.shp");
        wbvector::write(&routes, routes_path.as_path(), wbvector::VectorFormat::Shapefile)
            .unwrap();

        let out_path = std::env::temp_dir().join("assess_route_output_segments.shp");
        let mut args = ToolArgs::new();
        args.insert("routes".to_string(), json!(routes_path.to_string_lossy().to_string()));
        args.insert(
            "dem".to_string(),
            json!(memory_store::make_raster_memory_path(&dem_id)),
        );
        args.insert("segment_length".to_string(), json!(100.0));
        args.insert("search_radius".to_string(), json!(8));
        args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

        let result = AssessRouteTool.run(&args, &make_ctx()).unwrap();
        let p = result.outputs.get("path").unwrap().as_str().unwrap();
        assert!(std::path::Path::new(p).exists());

        let out_layer = wbvector::read(p).unwrap();
        assert!(out_layer.features.len() >= 2, "expected segmented route features");
        let avg_slope_idx = out_layer.schema.field_index("AVG_SLOPE").unwrap();
        let vis_idx = out_layer.schema.field_index("VISIBILITY").unwrap();

        let mut has_avg_slope = false;
        for feat in &out_layer.features {
            let avg_val = &feat.attributes[avg_slope_idx];
            if let Some(v) = avg_val.as_f64() {
                assert!(v.is_finite());
                has_avg_slope = true;
            } else if !matches!(avg_val, wbvector::FieldValue::Null) {
                panic!("expected AVG_SLOPE to be numeric or null");
            }

            let vis_val = &feat.attributes[vis_idx];
            assert!(
                vis_val.as_f64().is_some() || matches!(vis_val, wbvector::FieldValue::Null),
                "expected VISIBILITY to be numeric or null"
            );
        }
        assert!(has_avg_slope, "expected at least one segment with computed AVG_SLOPE");
    }

    #[test]
    fn profile_generates_html_output() {
        let cfg = RasterConfig {
            rows: 12,
            cols: 12,
            bands: 1,
            nodata: -9999.0,
            cell_size: 1.0,
            ..Default::default()
        };
        let mut dem = Raster::new(cfg);
        for r in 0..12isize {
            for c in 0..12isize {
                dem.set(0, r, c, (r + c) as f64).unwrap();
            }
        }
        let dem_id = memory_store::put_raster(dem);

        let mut lines = wbvector::Layer::new("profiles").with_geom_type(wbvector::GeometryType::LineString);
        lines
            .add_feature(
                Some(wbvector::Geometry::line_string(vec![
                    wbvector::Coord::xy(1.5, 10.5),
                    wbvector::Coord::xy(10.5, 1.5),
                ])),
                &[],
            )
            .unwrap();
        let lines_path = std::env::temp_dir().join("profile_lines_test.shp");
        wbvector::write(&lines, lines_path.as_path(), wbvector::VectorFormat::Shapefile).unwrap();

        let mut args = ToolArgs::new();
        args.insert(
            "lines_vector".to_string(),
            json!(lines_path.to_string_lossy().to_string()),
        );
        args.insert(
            "surface".to_string(),
            json!(memory_store::make_raster_memory_path(&dem_id)),
        );
        let out_path = std::env::temp_dir().join("profile_test_output.html");
        args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

        let result = ProfileTool.run(&args, &make_ctx()).unwrap();
        let p = result.outputs.get("path").unwrap().as_str().unwrap();
        assert!(std::path::Path::new(p).exists());
        let html = std::fs::read_to_string(p).unwrap();
        assert!(html.contains("Profile"));
    }

    #[test]
    fn low_points_on_headwater_divides_outputs_vector_points() {
        let cfg = RasterConfig {
            rows: 7,
            cols: 7,
            bands: 1,
            nodata: -9999.0,
            cell_size: 10.0,
            ..Default::default()
        };
        let mut dem = Raster::new(cfg.clone());
        let mut streams = Raster::new(cfg);

        for r in 0..7isize {
            for c in 0..7isize {
                let d_left = ((r - 3).abs() + (c - 2).abs()) as f64;
                let d_right = ((r - 3).abs() + (c - 4).abs()) as f64;
                let z = d_left.min(d_right) + 1.0;
                dem.set(0, r, c, z).unwrap();
                streams.set(0, r, c, 0.0).unwrap();
            }
        }
        dem.set(0, 3, 2, 0.0).unwrap();
        dem.set(0, 3, 4, 0.0).unwrap();
        streams.set(0, 3, 2, 1.0).unwrap();
        streams.set(0, 3, 4, 1.0).unwrap();

        let dem_id = memory_store::put_raster(dem);
        let streams_id = memory_store::put_raster(streams);

        let mut args = ToolArgs::new();
        args.insert(
            "dem".to_string(),
            json!(memory_store::make_raster_memory_path(&dem_id)),
        );
        args.insert(
            "streams".to_string(),
            json!(memory_store::make_raster_memory_path(&streams_id)),
        );
        let out_path = std::env::temp_dir().join("low_points_on_headwater_divides_test.shp");
        args.insert("output".to_string(), json!(out_path.to_string_lossy().to_string()));

        let result = LowPointsOnHeadwaterDividesTool
            .run(&args, &make_ctx())
            .unwrap();
        let p = result.outputs.get("path").unwrap().as_str().unwrap();
        assert!(std::path::Path::new(p).exists());

        let layer = wbvector::read(p).unwrap();
        assert!(!layer.features.is_empty(), "expected at least one low-point feature");
    }
}

