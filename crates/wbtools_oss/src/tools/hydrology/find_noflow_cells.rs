use serde_json::json;
use std::sync::mpsc;
use std::thread;
use wbcore::{
    LicenseTier, Tool, ToolArgs, ToolCategory, ToolContext, ToolError, ToolExample, ToolManifest,
    ToolMetadata, ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::{DataType, Raster, RasterConfig};

use super::{build_result, parse_dem_and_output, write_or_store_output, DX, DY};

pub struct FindNoflowCellsTool;

impl Tool for FindNoflowCellsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "find_noflow_cells",
            display_name: "Find Noflow Cells",
            summary: "Finds DEM cells that have no lower D8 neighbour.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
                ToolParamSpec {
                    name: "interior_only",
                    description: "Only flag interior cells (exclude raster border cells)",
                    required: false,
                },
                ToolParamSpec { name: "output", description: "Output raster path", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("interior_only".to_string(), json!(false));
        ToolManifest {
            id: "find_noflow_cells".to_string(),
            display_name: "Find Noflow Cells".to_string(),
            summary: "Finds DEM cells that have no lower D8 neighbour.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![ToolExample {
                name: "find_noflow".to_string(),
                description: "Identify pits, flats, and edge no-flow cells in a DEM".to_string(),
                args: ToolArgs::new(),
            }],
            tags: vec!["hydrology".to_string(), "diagnostics".to_string(), "dem".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        wbcore::parse_raster_path_arg(args, "dem")
            .or_else(|_| wbcore::parse_raster_path_arg(args, "input"))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let t_total = std::time::Instant::now();
        let (dem, output_path) = parse_dem_and_output(args)?;
        eprintln!("[find_noflow_cells] DEM load:         {:.3}s  ({} rows × {} cols)",
            t_total.elapsed().as_secs_f64(), dem.rows, dem.cols);
        let rows = dem.rows as isize;
        let cols = dem.cols as isize;
        let nodata = dem.nodata;
        let interior_only = args
            .get("interior_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut num_procs = thread::available_parallelism()
            .map(|n| n.get() as isize)
            .unwrap_or(1);
        if num_procs < 1 {
            num_procs = 1;
        }

        // Materialize band 0 once as a BandView: flat f64 buffer with a bounds-safe
        // get(row, col) that returns nodata for OOB — no explicit bounds checks needed
        // at any call site in the kernel.
        let t1 = std::time::Instant::now();
        let view = std::sync::Arc::new(dem.band_view(0));
        eprintln!("[find_noflow_cells] band_view:        {:.3}s", t1.elapsed().as_secs_f64());

        let out_cfg = RasterConfig {
            cols: dem.cols,
            rows: dem.rows,
            bands: dem.bands,
            x_min: dem.x_min,
            y_min: dem.y_min,
            cell_size: dem.cell_size_x,
            cell_size_y: Some(dem.cell_size_y),
            nodata,
            data_type: DataType::F64,
            crs: dem.crs.clone(),
            metadata: dem.metadata.clone(),
        };
        let mut out = Raster::new(out_cfg);

        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let view = view.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut row_data = vec![nodata; cols as usize];

                    for col in 0..cols {
                        if interior_only
                            && (row == 0 || col == 0 || row + 1 == rows || col + 1 == cols)
                        {
                            continue;
                        }

                        let z = view.get(row, col);
                        if view.is_nodata(z) {
                            continue;
                        }

                        let mut has_no_lower_neighbour = 1.0;
                        for n in 0..8 {
                            let zn = view.get(row + DY[n], col + DX[n]);
                            if zn < z && !view.is_nodata(zn) {
                                has_no_lower_neighbour = nodata;
                                break;
                            }
                        }

                        row_data[col as usize] = has_no_lower_neighbour;
                    }

                    let _ = tx.send((row, row_data));
                }
            });
        }
        drop(tx);
        let t2 = std::time::Instant::now();

        for row in 0..rows {
            let (r, row_data) = rx.recv().map_err(|e| {
                ToolError::Execution(format!("error receiving data from worker thread: {}", e))
            })?;

            out.set_row_slice(0, r, &row_data).map_err(|e| {
                ToolError::Execution(format!("failed writing output row {}: {}", r, e))
            })?;

            if rows > 1 {
                ctx.progress.progress(row as f64 / (rows - 1) as f64);
            } else {
                ctx.progress.progress(1.0);
            }
        }
        eprintln!("[find_noflow_cells] kernel+recv:      {:.3}s", t2.elapsed().as_secs_f64());

        let t3 = std::time::Instant::now();
        let result = build_result(write_or_store_output(out, output_path)?);
        eprintln!("[find_noflow_cells] write output:     {:.3}s", t3.elapsed().as_secs_f64());
        eprintln!("[find_noflow_cells] TOTAL:            {:.3}s", t_total.elapsed().as_secs_f64());
        Ok(result)
    }
}