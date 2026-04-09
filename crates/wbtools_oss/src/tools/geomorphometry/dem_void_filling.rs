use rayon::prelude::*;
use serde_json::json;
use wbcore::{
    parse_optional_output_path, parse_raster_path_arg, LicenseTier, Tool, ToolArgs, ToolCategory,
    ToolContext, ToolError, ToolManifest, ToolMetadata, ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::{Raster, RasterFormat};
use wbraster::memory_store;

pub struct DemVoidFillingTool;

#[derive(Clone, Copy, PartialEq, Eq)]
enum EdgeTreatment {
    UseDem,
    UseFill,
    Average,
}

struct DemVoidFillingCore;

impl DemVoidFillingCore {
    fn parse_input(args: &ToolArgs) -> Result<String, ToolError> {
        parse_raster_path_arg(args, "input")
    }

    fn parse_fill(args: &ToolArgs) -> Result<String, ToolError> {
        parse_raster_path_arg(args, "fill")
    }

    fn load_raster(path: &str) -> Result<Raster, ToolError> {
        if memory_store::raster_is_memory_path(path) {
            let id = memory_store::raster_path_to_id(path).ok_or_else(|| {
                ToolError::Validation("malformed in-memory raster path".to_string())
            })?;
            return memory_store::get_raster_by_id(id)
                .ok_or_else(|| ToolError::Validation(format!("unknown in-memory raster id '{}'", id)));
        }
        Raster::read(path)
            .map_err(|e| ToolError::Execution(format!("failed reading raster: {}", e)))
    }

    fn write_or_store_output(output: Raster, output_path: Option<std::path::PathBuf>) -> Result<String, ToolError> {
        if let Some(path) = output_path {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        ToolError::Execution(format!("failed creating output directory: {e}"))
                    })?;
                }
            }
            let output_path = path.to_string_lossy().to_string();
            let fmt = RasterFormat::for_output_path(&output_path)
                .map_err(|e| ToolError::Validation(format!("unsupported output path: {e}")))?;
            output
                .write(&output_path, fmt)
                .map_err(|e| ToolError::Execution(format!("failed writing output raster: {}", e)))?;
            Ok(output_path)
        } else {
            let id = memory_store::put_raster(output);
            Ok(memory_store::make_raster_memory_path(&id))
        }
    }

    fn bilinear_sample_by_pixel(input: &Raster, band: isize, row: f64, col: f64) -> Option<f64> {
        if row < 0.0 || col < 0.0 {
            return None;
        }
        let r0 = row.floor() as isize;
        let c0 = col.floor() as isize;
        let r1 = r0 + 1;
        let c1 = c0 + 1;
        if r1 >= input.rows as isize || c1 >= input.cols as isize {
            return None;
        }

        let z00 = input.get(band, r0, c0);
        let z10 = input.get(band, r1, c0);
        let z01 = input.get(band, r0, c1);
        let z11 = input.get(band, r1, c1);
        if input.is_nodata(z00) || input.is_nodata(z10) || input.is_nodata(z01) || input.is_nodata(z11) {
            return None;
        }

        let tx = col - c0 as f64;
        let ty = row - r0 as f64;
        let a = z00 * (1.0 - tx) + z01 * tx;
        let b = z10 * (1.0 - tx) + z11 * tx;
        Some(a * (1.0 - ty) + b * ty)
    }

    fn dem_void_filling_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "dem_void_filling",
            display_name: "DEM Void Filling",
            summary: "Fills voids in a DEM by fusing elevations from a secondary fill DEM with seamless edge transitions.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input DEM raster (with voids) path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "fill",
                    description: "Fill DEM raster path or typed raster object used to populate voids.",
                    required: true,
                },
                ToolParamSpec {
                    name: "mean_plane_dist",
                    description: "Distance in cells from void edges beyond which offsets are set to the global mean offset (default 20).",
                    required: false,
                },
                ToolParamSpec {
                    name: "edge_treatment",
                    description: "Void-edge treatment: 'dem', 'fill', or 'average' (default 'dem').",
                    required: false,
                },
                ToolParamSpec {
                    name: "weight_value",
                    description: "Inverse-distance interpolation power for near-edge offset interpolation (default 2.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output path.",
                    required: false,
                },
            ],
        }
    }

    fn dem_void_filling_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("fill".to_string(), json!("fill_dem.tif"));
        defaults.insert("mean_plane_dist".to_string(), json!(20));
        defaults.insert("edge_treatment".to_string(), json!("dem"));
        defaults.insert("weight_value".to_string(), json!(2.0));

        ToolManifest {
            id: "dem_void_filling".to_string(),
            display_name: "DEM Void Filling".to_string(),
            summary: "Fills DEM voids using a secondary surface and interpolated elevation offsets for seamless fusion.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec![
                "geomorphometry".to_string(),
                "void-filling".to_string(),
                "dem".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn run_dem_void_filling(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let dem_path = Self::parse_input(args)?;
        let fill_path = Self::parse_fill(args)?;
        let output_path = parse_optional_output_path(args, "output")?;

        let mean_plane_dist = args
            .get("mean_plane_dist")
            .and_then(|v| v.as_u64())
            .unwrap_or(20) as usize;
        let edge_treatment = args
            .get("edge_treatment")
            .or_else(|| args.get("edge"))
            .and_then(|v| v.as_str())
            .unwrap_or("dem")
            .to_ascii_lowercase();
        let edge_treatment = if edge_treatment.contains("fill") {
            EdgeTreatment::UseFill
        } else if edge_treatment.contains("avg") {
            EdgeTreatment::Average
        } else {
            EdgeTreatment::UseDem
        };
        let weight_value = args
            .get("weight_value")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0)
            .max(0.1);

        let dem = Self::load_raster(&dem_path)?;
        let fill = Self::load_raster(&fill_path)?;

        let rows = dem.rows;
        let cols = dem.cols;
        let dem_nodata = dem.nodata;
        let fill_nodata = fill.nodata;
        let band = 0isize;

        if rows == 0 || cols == 0 {
            return Err(ToolError::Validation("input DEM has zero rows or columns".to_string()));
        }

        ctx.progress.info("resampling fill DEM to input grid");

        let fill_resampled_rows: Vec<Vec<f64>> = (0..rows)
            .into_par_iter()
            .map(|r| {
                let mut row_out = vec![fill_nodata; cols];
                for c in 0..cols {
                    let x = dem.col_center_x(c as isize);
                    let y = dem.row_center_y(r as isize);
                    let row_src = (fill.y_max() - y) / fill.cell_size_y;
                    let col_src = (x - fill.x_min) / fill.cell_size_x;
                    if let Some(z) = Self::bilinear_sample_by_pixel(&fill, band, row_src, col_src) {
                        row_out[c] = z;
                    }
                }
                row_out
            })
            .collect();

        let mut fill_resampled = vec![fill_nodata; rows * cols];
        for r in 0..rows {
            let start = r * cols;
            let end = start + cols;
            fill_resampled[start..end].copy_from_slice(&fill_resampled_rows[r]);
        }

        ctx.progress.info("finding void edges and offset surface");

        let dx = [1isize, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1isize, 0, 1, 1, 1, 0, -1, -1];

        let edge_rows: Vec<Vec<bool>> = (0..rows)
            .into_par_iter()
            .map(|r| {
                let mut row_out = vec![false; cols];
                for c in 0..cols {
                    let z = dem.get(band, r as isize, c as isize);
                    if dem.is_nodata(z) {
                        continue;
                    }
                    let mut is_edge = false;
                    for n in 0..8 {
                        let rr = r as isize + dy[n];
                        let cc = c as isize + dx[n];
                        if rr >= 0 && cc >= 0 && rr < rows as isize && cc < cols as isize {
                            let zn = dem.get(band, rr, cc);
                            if dem.is_nodata(zn) {
                                is_edge = true;
                                break;
                            }
                        }
                    }
                    row_out[c] = is_edge;
                }
                row_out
            })
            .collect();

        let mut edges = vec![false; rows * cols];
        for r in 0..rows {
            for c in 0..cols {
                edges[r * cols + c] = edge_rows[r][c];
            }
        }

        let mut dem_adj = vec![dem_nodata; rows * cols];
        let mut dod = vec![dem_nodata; rows * cols];
        let mut sum_dod = 0.0;
        let mut num_dod = 0usize;

        for r in 0..rows {
            for c in 0..cols {
                let idx = r * cols + c;
                let dem_z = dem.get(band, r as isize, c as isize);
                let fill_z = fill_resampled[idx];
                dem_adj[idx] = dem_z;

                if dem.is_nodata(fill_z) || dem.is_nodata(dem_z) {
                    continue;
                }

                if !edges[idx] || edge_treatment == EdgeTreatment::UseDem {
                    let v = dem_z - fill_z;
                    dod[idx] = v;
                    sum_dod += v;
                    num_dod += 1;
                } else if edge_treatment == EdgeTreatment::UseFill {
                    dem_adj[idx] = dem_nodata;
                } else {
                    let avg = 0.5 * (dem_z + fill_z);
                    dem_adj[idx] = avg;
                    let v = avg - fill_z;
                    dod[idx] = v;
                    sum_dod += v;
                    num_dod += 1;
                }
            }
        }

        let mean_offset = if num_dod > 0 {
            sum_dod / num_dod as f64
        } else {
            0.0
        };

        let radius = mean_plane_dist as isize;
        let mut kernel_dx = Vec::<isize>::new();
        let mut kernel_dy = Vec::<isize>::new();
        let mut kernel_w = Vec::<f64>::new();

        if radius > 0 {
            for yy in -radius..=radius {
                for xx in -radius..=radius {
                    let dist = (xx as f64).hypot(yy as f64);
                    if dist <= radius as f64 && dist > 0.0 {
                        kernel_dx.push(xx);
                        kernel_dy.push(yy);
                        kernel_w.push(1.0 / dist.powf(weight_value));
                    }
                }
            }
        }

        let mut offsets = dod.clone();

        if radius > 0 {
            for r in 0..rows {
                for c in 0..cols {
                    let idx = r * cols + c;
                    if !dem.is_nodata(dem_adj[idx]) || dem.is_nodata(fill_resampled[idx]) {
                        continue;
                    }

                    let mut near_edge = false;
                    for n in 0..kernel_dx.len() {
                        let rr = r as isize + kernel_dy[n];
                        let cc = c as isize + kernel_dx[n];
                        if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                            continue;
                        }
                        let j = rr as usize * cols + cc as usize;
                        if !dem.is_nodata(dod[j]) {
                            near_edge = true;
                            break;
                        }
                    }
                    if !near_edge {
                        offsets[idx] = mean_offset;
                    }
                }
            }
        }

        ctx.progress.info("interpolating offsets and writing output");

        let out_rows: Vec<Vec<f64>> = (0..rows)
            .into_par_iter()
            .map(|r| {
                let mut row_out = vec![dem_nodata; cols];
                for c in 0..cols {
                    let idx = r * cols + c;
                    let fill_z = fill_resampled[idx];
                    let dem_z = dem_adj[idx];

                    if !dem.is_nodata(dem_z) {
                        row_out[c] = dem_z;
                        continue;
                    }
                    if dem.is_nodata(fill_z) {
                        row_out[c] = dem_nodata;
                        continue;
                    }

                    let mut off = offsets[idx];
                    if dem.is_nodata(off) {
                        let mut sum_w = 0.0;
                        let mut sum_off = 0.0;
                        if radius > 0 {
                            for n in 0..kernel_dx.len() {
                                let rr = r as isize + kernel_dy[n];
                                let cc = c as isize + kernel_dx[n];
                                if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                                    continue;
                                }
                                let j = rr as usize * cols + cc as usize;
                                let v = offsets[j];
                                if !dem.is_nodata(v) {
                                    let w = kernel_w[n];
                                    sum_off += v * w;
                                    sum_w += w;
                                }
                            }
                        }
                        off = if sum_w > 0.0 { sum_off / sum_w } else { mean_offset };
                    }

                    row_out[c] = fill_z + off;
                }
                row_out
            })
            .collect();

        let mut output = dem.clone();
        for r in 0..rows {
            output
                .set_row_slice(band, r as isize, &out_rows[r])
                .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            ctx.progress.progress((r + 1) as f64 / rows as f64);
        }

        let output_locator = Self::write_or_store_output(output, output_path)?;
        let mut outputs = std::collections::BTreeMap::new();
        outputs.insert("path".to_string(), json!(output_locator));
        Ok(ToolRunResult {
            outputs,
            ..Default::default()
        })
    }
}

impl Tool for DemVoidFillingTool {
    fn metadata(&self) -> ToolMetadata {
        DemVoidFillingCore::dem_void_filling_metadata()
    }

    fn manifest(&self) -> ToolManifest {
        DemVoidFillingCore::dem_void_filling_manifest()
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = DemVoidFillingCore::parse_input(args)?;
        let _ = DemVoidFillingCore::parse_fill(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        DemVoidFillingCore::run_dem_void_filling(args, ctx)
    }
}

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

    fn make_constant_raster(rows: usize, cols: usize, value: f64, nodata: f64) -> Raster {
        let cfg = RasterConfig {
            rows,
            cols,
            bands: 1,
            nodata,
            cell_size: 10.0,
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
    fn dem_void_filling_fills_simple_center_void() {
        let nodata = -9999.0;
        let mut dem = make_constant_raster(7, 7, 10.0, nodata);
        dem.set(0, 3, 3, nodata).unwrap();
        let fill = make_constant_raster(7, 7, 20.0, nodata);

        let dem_id = memory_store::put_raster(dem);
        let fill_id = memory_store::put_raster(fill);

        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&dem_id)));
        args.insert("fill".to_string(), json!(memory_store::make_raster_memory_path(&fill_id)));
        args.insert("mean_plane_dist".to_string(), json!(2));
        args.insert("edge_treatment".to_string(), json!("dem"));
        args.insert("weight_value".to_string(), json!(2.0));

        let result = DemVoidFillingTool.run(&args, &make_ctx()).unwrap();
        let out_id = memory_store::raster_path_to_id(result.outputs.get("path").unwrap().as_str().unwrap()).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        let center = out.get(0, 3, 3);
        assert!((center - 10.0).abs() < 1e-6, "expected center near 10.0, got {center}");
    }
}
