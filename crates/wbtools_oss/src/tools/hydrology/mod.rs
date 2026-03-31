use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::BinaryHeap;
use std::collections::HashSet;
use std::collections::VecDeque;

use serde_json::json;
use rayon::prelude::*;
use wbcore::{
	parse_optional_output_path, parse_raster_path_arg, parse_vector_path_arg, LicenseTier, Tool, ToolArgs, ToolCategory,
	ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata, ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::{DataType, Raster, RasterFormat};
use wbvector;

use crate::memory_store;
use super::flow_algorithms::{D8FlowAccumTool, D8PointerTool};
use super::stream_network_analysis::VectorStreamNetworkAnalysisTool;

pub struct BreachDepressionsLeastCostTool;
pub struct BreachSingleCellPitsTool;
pub struct FillDepressionsTool;
pub struct FillDepressionsPlanchonAndDarbouxTool;
pub struct FillDepressionsWangAndLiuTool;
pub struct FillPitsTool;
pub struct DepthInSinkTool;
pub struct SinkTool;
pub struct FlowAccumFullWorkflowTool;
pub struct FindNoflowCellsTool;
pub struct NumInflowingNeighboursTool;
pub struct FindParallelFlowTool;
pub struct BasinsTool;
pub struct WatershedFromRasterPourPointsTool;
pub struct WatershedTool;
pub struct JensonSnapPourPointsTool;
pub struct SnapPourPointsTool;
pub struct SubbasinsTool;
pub struct HillslopesTool;
pub struct StrahlerOrderBasinsTool;
pub struct IsobasinsTool;
pub struct EdgeContaminationTool;
pub struct D8MassFluxTool;
pub struct DInfMassFluxTool;
pub struct FlowLengthDiffTool;
pub struct DownslopeFlowpathLengthTool;
pub struct MaxUpslopeFlowpathLengthTool;
pub struct AverageUpslopeFlowpathLengthTool;
pub struct ElevationAboveStreamTool;
pub struct ElevationAboveStreamEuclideanTool;
pub struct DownslopeDistanceToStreamTool;
pub struct AverageFlowpathSlopeTool;
pub struct MaxUpslopeValueTool;
pub struct LongestFlowpathTool;
pub struct DepthToWaterTool;
pub struct FillBurnTool;
pub struct BurnStreamsAtRoadsTool;
pub struct TraceDownslopeFlowpathsTool;
pub struct FloodOrderTool;
pub struct InsertDamsTool;
pub struct RaiseWallsTool;
pub struct TopologicalBreachBurnTool;
pub struct StochasticDepressionAnalysisTool;
pub struct UnnestBasinsTool;
pub struct UpslopeDepressionStorageTool;
pub struct FlattenLakesTool;
pub struct HydrologicConnectivityTool;
pub struct ImpoundmentSizeIndexTool;

const DX: [isize; 8] = [1, 1, 1, 0, -1, -1, -1, 0];
const DY: [isize; 8] = [-1, 0, 1, 1, 1, 0, -1, -1];

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

fn build_triple_raster_result(
	breached_dem_path: String,
	flow_dir_path: String,
	flow_accum_path: String,
) -> ToolRunResult {
	let breached_dem = typed_raster_output(breached_dem_path);
	let flow_dir = typed_raster_output(flow_dir_path);
	let flow_accum = typed_raster_output(flow_accum_path);
	let mut outputs = BTreeMap::new();
	outputs.insert("breached_dem".to_string(), breached_dem.clone());
	outputs.insert("flow_dir".to_string(), flow_dir.clone());
	outputs.insert("flow_accum".to_string(), flow_accum.clone());
	outputs.insert("__wbw_type__".to_string(), json!("tuple"));
	outputs.insert("items".to_string(), json!([breached_dem, flow_dir, flow_accum]));
	ToolRunResult {
		outputs,
		..Default::default()
	}
}

fn build_pair_raster_result(first_name: &str, first_path: String, second_name: &str, second_path: String) -> ToolRunResult {
	let first = typed_raster_output(first_path);
	let second = typed_raster_output(second_path);
	let mut outputs = BTreeMap::new();
	outputs.insert(first_name.to_string(), first.clone());
	outputs.insert(second_name.to_string(), second.clone());
	outputs.insert("__wbw_type__".to_string(), json!("tuple"));
	outputs.insert("items".to_string(), json!([first, second]));
	ToolRunResult {
		outputs,
		..Default::default()
	}
}

fn build_quad_raster_result(
	first_name: &str,
	first_path: String,
	second_name: &str,
	second_path: String,
	third_name: &str,
	third_path: String,
	fourth_name: &str,
	fourth_path: String,
) -> ToolRunResult {
	let first = typed_raster_output(first_path);
	let second = typed_raster_output(second_path);
	let third = typed_raster_output(third_path);
	let fourth = typed_raster_output(fourth_path);
	let mut outputs = BTreeMap::new();
	outputs.insert(first_name.to_string(), first.clone());
	outputs.insert(second_name.to_string(), second.clone());
	outputs.insert(third_name.to_string(), third.clone());
	outputs.insert(fourth_name.to_string(), fourth.clone());
	outputs.insert("__wbw_type__".to_string(), json!("tuple"));
	outputs.insert("items".to_string(), json!([first, second, third, fourth]));
	ToolRunResult {
		outputs,
		..Default::default()
	}
}

fn build_raster_list_result(paths: Vec<String>) -> ToolRunResult {
	let mut outputs = BTreeMap::new();
	let items: Vec<serde_json::Value> = paths.iter().cloned().map(typed_raster_output).collect();
	outputs.insert("__wbw_type__".to_string(), json!("tuple"));
	outputs.insert("items".to_string(), json!(items));
	outputs.insert("paths".to_string(), json!(paths));
	ToolRunResult {
		outputs,
		..Default::default()
	}
}

fn raster_to_vec(input: &Raster) -> Vec<f64> {
	let mut out = vec![input.nodata; input.rows * input.cols];
	for r in 0..input.rows {
		for c in 0..input.cols {
			out[idx(r, c, input.cols)] = input.get(0, r as isize, c as isize);
		}
	}
	out
}

fn vec_to_raster(template: &Raster, data: &[f64], data_type: DataType) -> Raster {
	let mut out = template.clone();
	out.data_type = data_type;
	for r in 0..template.rows {
		for c in 0..template.cols {
			out.set_unchecked(0, r as isize, c as isize, data[idx(r, c, template.cols)]);
		}
	}
	out
}

fn auto_small_increment(r: &Raster, flat_increment: Option<f64>) -> f64 {
	if let Some(v) = flat_increment {
		if v.is_finite() && v > 0.0 {
			return v;
		}
	}
	let base = (r.cell_size_x.abs() + r.cell_size_y.abs()).max(1.0);
	(base * 1.0e-9).max(1.0e-8)
}

fn auto_small_increment_legacy(r: &Raster, data: &[f64], flat_increment: Option<f64>, fix_flats: bool) -> f64 {
	if !fix_flats {
		return 0.0;
	}
	if let Some(v) = flat_increment {
		if v.is_finite() && v > 0.0 {
			return v;
		}
	}
	let diag = (r.cell_size_x * r.cell_size_x + r.cell_size_y * r.cell_size_y).sqrt().ceil().max(1.0);
	let mut max_elev = f64::NEG_INFINITY;
	for &z in data {
		if z != r.nodata && z.is_finite() && z > max_elev {
			max_elev = z;
		}
	}
	if !max_elev.is_finite() {
		return auto_small_increment(r, None);
	}
	let digits = max_elev.abs().floor() as i64;
	let elev_digits = if digits <= 0 { 1 } else { digits.to_string().len() } as i32;
	let exp = (15 - elev_digits).max(1);
	let elev_multiplier = 10.0_f64.powi(exp);
	let v = (1.0_f64 / elev_multiplier) * diag;
	if v.is_finite() && v > 0.0 {
		v
	} else {
		auto_small_increment(r, None)
	}
}

fn get_oob_as_nodata(data: &[f64], r: isize, c: isize, rows: usize, cols: usize, nodata: f64) -> f64 {
	if in_bounds(r, c, rows, cols) {
		data[idx(r as usize, c as usize, cols)]
	} else {
		nodata
	}
}

fn detect_pits(data: &[f64], rows: usize, cols: usize, nodata: f64) -> Vec<usize> {
	let mut pits = Vec::new();
	for r in 0..rows {
		for c in 0..cols {
			let i = idx(r, c, cols);
			let z = data[i];
			if z == nodata {
				continue;
			}
			let mut has_lower = false;
			for k in 0..8 {
				let rn = r as isize + DY[k];
				let cn = c as isize + DX[k];
				if !in_bounds(rn, cn, rows, cols) {
					continue;
				}
				let zn = data[idx(rn as usize, cn as usize, cols)];
				if zn != nodata && zn < z {
					has_lower = true;
					break;
				}
			}
			if !has_lower {
				pits.push(i);
			}
		}
	}
	pits
}

fn detect_strict_pits_with_raise(data: &mut [f64], rows: usize, cols: usize, nodata: f64, small: f64) -> Vec<(usize, f64)> {
	let mut pits = Vec::new();
	for r in 0..rows {
		for c in 0..cols {
			let i = idx(r, c, cols);
			let z = data[i];
			if z == nodata {
				continue;
			}
			let mut is_pit = true;
			let mut min_zn = f64::INFINITY;
			for k in 0..8 {
				let rn = r as isize + DY[k];
				let cn = c as isize + DX[k];
				let zn = get_oob_as_nodata(data, rn, cn, rows, cols, nodata);
				if zn < min_zn {
					min_zn = zn;
				}
				if zn == nodata || zn < z {
					is_pit = false;
					break;
				}
			}
			if is_pit {
				data[i] = min_zn - small;
				pits.push((i, z));
			}
		}
	}
	pits
}

fn fill_pits_core(data: &mut [f64], rows: usize, cols: usize, nodata: f64, small: f64) {
	let pits = detect_pits(data, rows, cols, nodata);
	for i in pits {
		let r = i / cols;
		let c = i % cols;
		let mut min_neigh = f64::INFINITY;
		for k in 0..8 {
			let rn = r as isize + DY[k];
			let cn = c as isize + DX[k];
			if !in_bounds(rn, cn, rows, cols) {
				continue;
			}
			let zn = data[idx(rn as usize, cn as usize, cols)];
			if zn != nodata && zn < min_neigh {
				min_neigh = zn;
			}
		}
		if min_neigh.is_finite() {
			data[i] = min_neigh + small;
		}
	}
}

fn breach_single_cell_pits_core(data: &mut [f64], rows: usize, cols: usize, nodata: f64) {
	let dx2: [isize; 16] = [2, 2, 2, 2, 2, 1, 0, -1, -2, -2, -2, -2, -2, -1, 0, 1];
	let dy2: [isize; 16] = [-2, -1, 0, 1, 2, 2, 2, 2, 2, 1, 0, -1, -2, -2, -2, -2];
	let breach_cell: [usize; 16] = [0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 0];
	let pits = detect_pits(data, rows, cols, nodata);
	for i in pits {
		let r = i / cols;
		let c = i % cols;
		let z = data[i];
		for n in 0..16 {
			let r2 = r as isize + dy2[n];
			let c2 = c as isize + dx2[n];
			if !in_bounds(r2, c2, rows, cols) {
				continue;
			}
			let z2 = data[idx(r2 as usize, c2 as usize, cols)];
			if z2 == nodata || z2 >= z {
				continue;
			}
			let b = breach_cell[n];
			let rb = r as isize + DY[b];
			let cb = c as isize + DX[b];
			if !in_bounds(rb, cb, rows, cols) {
				continue;
			}
			let bi = idx(rb as usize, cb as usize, cols);
			let candidate = 0.5 * (z + z2);
			if data[bi] == nodata || candidate < data[bi] {
				data[bi] = candidate;
			}
		}
	}
}

#[derive(Clone, Copy)]
struct MinNode {
	elev: f64,
	i: usize,
}

impl PartialEq for MinNode {
	fn eq(&self, other: &Self) -> bool {
		self.i == other.i && self.elev.to_bits() == other.elev.to_bits()
	}
}
impl Eq for MinNode {}
impl PartialOrd for MinNode {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		other.elev.partial_cmp(&self.elev)
	}
}
impl Ord for MinNode {
	fn cmp(&self, other: &Self) -> Ordering {
		self.partial_cmp(other).unwrap_or(Ordering::Equal)
	}
}


#[derive(Clone, Copy)]
struct FlatNode {
	priority: f64,
	i: usize,
	base: f64,
}

impl PartialEq for FlatNode {
	fn eq(&self, other: &Self) -> bool {
		self.i == other.i && self.priority.to_bits() == other.priority.to_bits()
	}
}

impl Eq for FlatNode {}

impl PartialOrd for FlatNode {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		other.priority.partial_cmp(&self.priority)
	}
}

impl Ord for FlatNode {
	fn cmp(&self, other: &Self) -> Ordering {
		self.partial_cmp(other).unwrap_or(Ordering::Equal)
	}
}

fn fill_depressions_core(
	input: &[f64],
	rows: usize,
	cols: usize,
	nodata: f64,
	small: f64,
	max_depth: f64,
	fix_flats: bool,
) -> Vec<f64> {
	let mut out = input.to_vec();
	let mut pits = Vec::<(usize, f64)>::new();
	for r in 1..rows.saturating_sub(1) {
		for c in 1..cols.saturating_sub(1) {
			let i = idx(r, c, cols);
			let z = out[i];
			if z == nodata {
				continue;
			}
			let mut is_pit = true;
			for k in 0..8 {
				let zn = out[idx((r as isize + DY[k]) as usize, (c as isize + DX[k]) as usize, cols)];
				if zn == nodata || zn < z {
					is_pit = false;
					break;
				}
			}
			if is_pit {
				pits.push((i, z));
			}
		}
	}
	if pits.is_empty() {
		return out;
	}

	pits.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
	let mut visited = vec![0u8; rows * cols];
	let mut flats = vec![0u8; rows * cols];
	let mut possible_outlets = Vec::<usize>::new();

	while let Some((pit_i, _)) = pits.pop() {
		if flats[pit_i] == 1 {
			continue;
		}
		let z_pit = out[pit_i];
		let mut heap = BinaryHeap::<MinNode>::new();
		heap.push(MinNode { elev: z_pit, i: pit_i });
		visited[pit_i] = 1;
		let mut outlet_found = false;
		let mut outlet_z = f64::INFINITY;
		let mut queue = VecDeque::<usize>::new();

		while let Some(cell) = heap.pop() {
			let z = cell.elev;
			if outlet_found && z > outlet_z {
				break;
			}
			if z - z_pit > max_depth {
				break;
			}
			let r = cell.i / cols;
			let c = cell.i % cols;
			if !outlet_found {
				for k in 0..8 {
					let rn = r as isize + DY[k];
					let cn = c as isize + DX[k];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					if visited[ni] != 0 {
						continue;
					}
					let zn = out[ni];
					if zn >= z && zn != nodata {
						heap.push(MinNode { elev: zn, i: ni });
						visited[ni] = 1;
					} else if zn != nodata {
						outlet_found = true;
						outlet_z = z;
						queue.push_back(cell.i);
						possible_outlets.push(cell.i);
					}
				}
			} else if z == outlet_z {
				let mut is_outlet = false;
				for k in 0..8 {
					let rn = r as isize + DY[k];
					let cn = c as isize + DX[k];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					if visited[ni] != 0 {
						continue;
					}
					let zn = out[ni];
					if zn < z {
						is_outlet = true;
					} else if zn == outlet_z {
						heap.push(MinNode { elev: zn, i: ni });
						visited[ni] = 1;
					}
				}
				if is_outlet {
					queue.push_back(cell.i);
					possible_outlets.push(cell.i);
				} else {
					visited[cell.i] = 1;
				}
			}
		}

		if outlet_found {
			while let Some(ci) = queue.pop_front() {
				let r = ci / cols;
				let c = ci % cols;
				for k in 0..8 {
					let rn = r as isize + DY[k];
					let cn = c as isize + DX[k];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					if visited[ni] == 1 {
						visited[ni] = 0;
						queue.push_back(ni);
						if out[ni] < outlet_z {
							out[ni] = outlet_z;
							flats[ni] = 1;
						} else if out[ni] == outlet_z {
							flats[ni] = 1;
						}
					}
				}
			}
		} else {
			let mut cleanup = VecDeque::<usize>::new();
			cleanup.push_back(pit_i);
			while let Some(ci) = cleanup.pop_front() {
				let r = ci / cols;
				let c = ci % cols;
				for k in 0..8 {
					let rn = r as isize + DY[k];
					let cn = c as isize + DX[k];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					if visited[ni] == 1 {
						visited[ni] = 0;
						cleanup.push_back(ni);
					}
				}
			}
		}
	}

	if fix_flats && small > 0.0 {
		let mut outlet_heap = BinaryHeap::<MinNode>::new();
		while let Some(i) = possible_outlets.pop() {
			let r = i / cols;
			let c = i % cols;
			let z = out[i];
			let mut is_outlet = false;
			for k in 0..8 {
				let rn = r as isize + DY[k];
				let cn = c as isize + DX[k];
				if !in_bounds(rn, cn, rows, cols) {
					continue;
				}
				let zn = out[idx(rn as usize, cn as usize, cols)];
				if zn != nodata && zn < z {
					is_outlet = true;
					break;
				}
			}
			if is_outlet {
				outlet_heap.push(MinNode { elev: z, i });
			}
		}

		let mut flat_heap = BinaryHeap::<FlatNode>::new();
		while let Some(cell) = outlet_heap.pop() {
			if flats[cell.i] == 3 {
				continue;
			}
			let z = out[cell.i];
			flats[cell.i] = 3;
			let mut same_level = vec![cell.i];
			while let Some(peek) = outlet_heap.peek() {
				if peek.elev == z {
					let p = outlet_heap.pop().expect("heap pop");
					flats[p.i] = 3;
					same_level.push(p.i);
				} else {
					break;
				}
			}

			flat_heap.clear();
			for oi in same_level {
				let r = oi / cols;
				let c = oi % cols;
				for k in 0..8 {
					let rn = r as isize + DY[k];
					let cn = c as isize + DX[k];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					if flats[ni] != 3 && out[ni] == z && out[ni] != nodata {
						out[ni] = z + small;
						flats[ni] = 3;
						flat_heap.push(FlatNode {
							priority: input[ni],
							i: ni,
							base: z,
						});
					}
				}
			}

			while let Some(nc) = flat_heap.pop() {
				let r = nc.i / cols;
				let c = nc.i % cols;
				let zc = out[nc.i];
				for k in 0..8 {
					let rn = r as isize + DY[k];
					let cn = c as isize + DX[k];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					if flats[ni] == 3 {
						continue;
					}
					let zn = out[ni];
					if zn != nodata && zn < zc + small && zn >= nc.base {
						out[ni] = zc + small;
						flats[ni] = 3;
						flat_heap.push(FlatNode {
							priority: input[ni],
							i: ni,
							base: nc.base,
						});
					}
				}
			}
		}
	}

	out
}

fn fill_depressions_wang_and_liu_core(
	input: &[f64],
	rows: usize,
	cols: usize,
	nodata: f64,
	small: f64,
) -> Vec<f64> {
	let background = (i32::MIN + 1) as f64;
	let mut out = vec![background; rows * cols];
	let mut queue = VecDeque::<(isize, isize)>::new();

	for r in 0..rows as isize {
		queue.push_back((r, -1));
		queue.push_back((r, cols as isize));
	}
	for c in 0..cols as isize {
		queue.push_back((-1, c));
		queue.push_back((rows as isize, c));
	}

	let mut heap = BinaryHeap::<MinNode>::new();
	while let Some((r, c)) = queue.pop_front() {
		for k in 0..8 {
			let rn = r + DY[k];
			let cn = c + DX[k];
			if !in_bounds(rn, cn, rows, cols) {
				continue;
			}
			let ni = idx(rn as usize, cn as usize, cols);
			if out[ni] != background {
				continue;
			}
			let zin = input[ni];
			if zin == nodata {
				out[ni] = nodata;
				queue.push_back((rn, cn));
			} else {
				out[ni] = zin;
				heap.push(MinNode { elev: zin, i: ni });
			}
		}
	}

	while let Some(cell) = heap.pop() {
		let r = cell.i / cols;
		let c = cell.i % cols;
		let z = out[cell.i];
		for k in 0..8 {
			let rn = r as isize + DY[k];
			let cn = c as isize + DX[k];
			if !in_bounds(rn, cn, rows, cols) {
				continue;
			}
			let ni = idx(rn as usize, cn as usize, cols);
			if out[ni] != background {
				continue;
			}
			let mut zn = input[ni];
			if zn != nodata {
				if zn < z + small {
					zn = z + small;
				}
				out[ni] = zn;
				heap.push(MinNode { elev: zn, i: ni });
			} else {
				out[ni] = nodata;
			}
		}
	}

	for v in &mut out {
		if *v == background {
			*v = nodata;
		}
	}

	out
}

fn fill_depressions_planchon_and_darboux_core(
	input: &[f64],
	rows: usize,
	cols: usize,
	nodata: f64,
	small: f64,
) -> Vec<f64> {
	let nodata_out = -32768.0;
	let large_value = f64::INFINITY;
	let mut out = vec![large_value; rows * cols];

	let seed_edge = |sr: usize, sc: usize, out: &mut [f64]| {
		let si = idx(sr, sc, cols);
		let z = input[si];
		let w = out[si];
		if z != nodata {
			out[si] = z;
		} else if w == large_value {
			out[si] = nodata_out;
			let mut stack = vec![si];
			while let Some(ci) = stack.pop() {
				let r = ci / cols;
				let c = ci % cols;
				for k in 0..8 {
					let rn = r as isize + DY[k];
					let cn = c as isize + DX[k];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					if out[ni] == large_value {
						let zn = input[ni];
						if zn == nodata {
							out[ni] = nodata_out;
							stack.push(ni);
						} else {
							out[ni] = zn;
						}
					}
				}
			}
		}
	};

	for r in 0..rows {
		seed_edge(r, 0, &mut out);
		seed_edge(r, cols - 1, &mut out);
	}
	for c in 0..cols {
		seed_edge(0, c, &mut out);
		seed_edge(rows - 1, c, &mut out);
	}

	let mut sweep = 0usize;
	let mut changed = true;
	while changed {
		changed = false;
		match sweep {
			0 => {
				for r in 1..rows.saturating_sub(1) {
					for c in 1..cols.saturating_sub(1) {
						let i = idx(r, c, cols);
						let z = input[i];
						let mut w = out[i];
						if w == nodata_out || w <= z {
							continue;
						}
						for k in 0..8 {
							let ni = idx((r as isize + DY[k]) as usize, (c as isize + DX[k]) as usize, cols);
							let wn = out[ni];
							if wn == nodata_out {
								continue;
							}
							let wn2 = wn + small;
							if z >= wn2 {
								out[i] = z;
								changed = true;
								break;
							} else if w > wn2 && wn2 > z {
								out[i] = wn2;
								w = wn2;
								changed = true;
							}
						}
					}
				}
			}
			1 => {
				for r in (1..rows.saturating_sub(1)).rev() {
					for c in (1..cols.saturating_sub(1)).rev() {
						let i = idx(r, c, cols);
						let z = input[i];
						let mut w = out[i];
						if w == nodata_out || w <= z {
							continue;
						}
						for k in 0..8 {
							let ni = idx((r as isize + DY[k]) as usize, (c as isize + DX[k]) as usize, cols);
							let wn = out[ni];
							if wn == nodata_out {
								continue;
							}
							let wn2 = wn + small;
							if z >= wn2 {
								out[i] = z;
								changed = true;
								break;
							} else if w > wn2 && wn2 > z {
								out[i] = wn2;
								w = wn2;
								changed = true;
							}
						}
					}
				}
			}
			2 => {
				for r in 1..rows.saturating_sub(1) {
					for c in (1..cols.saturating_sub(1)).rev() {
						let i = idx(r, c, cols);
						let z = input[i];
						let mut w = out[i];
						if w == nodata_out || w <= z {
							continue;
						}
						for k in 0..8 {
							let ni = idx((r as isize + DY[k]) as usize, (c as isize + DX[k]) as usize, cols);
							let wn = out[ni];
							if wn == nodata_out {
								continue;
							}
							let wn2 = wn + small;
							if z >= wn2 {
								out[i] = z;
								changed = true;
								break;
							} else if w > wn2 && wn2 > z {
								out[i] = wn2;
								w = wn2;
								changed = true;
							}
						}
					}
				}
			}
			_ => {
				for r in (1..rows.saturating_sub(1)).rev() {
					for c in 1..cols.saturating_sub(1) {
						let i = idx(r, c, cols);
						let z = input[i];
						let mut w = out[i];
						if w == nodata_out || w <= z {
							continue;
						}
						for k in 0..8 {
							let ni = idx((r as isize + DY[k]) as usize, (c as isize + DX[k]) as usize, cols);
							let wn = out[ni];
							if wn == nodata_out {
								continue;
							}
							let wn2 = wn + small;
							if z >= wn2 {
								out[i] = z;
								changed = true;
								break;
							} else if w > wn2 && wn2 > z {
								out[i] = wn2;
								w = wn2;
								changed = true;
							}
						}
					}
				}
			}
		}
		sweep = (sweep + 1) % 4;
	}

	out
}

#[derive(Clone, Copy)]
struct CostNode {
	cost: f64,
	steps: usize,
	i: usize,
}

impl PartialEq for CostNode {
	fn eq(&self, other: &Self) -> bool {
		self.i == other.i && self.cost.to_bits() == other.cost.to_bits() && self.steps == other.steps
	}
}
impl Eq for CostNode {}
impl PartialOrd for CostNode {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		other.cost.partial_cmp(&self.cost)
	}
}
impl Ord for CostNode {
	fn cmp(&self, other: &Self) -> Ordering {
		self.partial_cmp(other).unwrap_or(Ordering::Equal)
	}
}

fn breach_depressions_least_cost_core(
	data: &mut [f64],
	rows: usize,
	cols: usize,
	nodata: f64,
	max_cost: f64,
	max_dist: usize,
	small: f64,
	minimize_dist: bool,
) {
	let mut pits = detect_strict_pits_with_raise(data, rows, cols, nodata, small);
	if pits.is_empty() {
		return;
	}
	pits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

	let diag = 2.0f64.sqrt();
	let step_dist = [diag, 1.0, diag, 1.0, diag, 1.0, diag, 1.0];
	let n = rows * cols;
	let backlink_dir = [4usize, 5, 6, 7, 0, 1, 2, 3];
	let mut encountered = vec![0u8; n];
	let mut backlink = vec![-1i8; n];
	let mut path_length = vec![0i16; n];
	let mut touched = Vec::<usize>::new();

	while let Some((pit, _orig)) = pits.pop() {
		let pit_z = data[pit];
		let r0 = pit / cols;
		let c0 = pit % cols;
		let mut still_pit = true;
		for k in 0..8 {
			let rn = r0 as isize + DY[k];
			let cn = c0 as isize + DX[k];
			if !in_bounds(rn, cn, rows, cols) {
				continue;
			}
			let zn = data[idx(rn as usize, cn as usize, cols)];
			if zn != nodata && zn < pit_z {
				still_pit = false;
				break;
			}
		}
		if !still_pit {
			continue;
		}

		encountered[pit] = 1;
		touched.push(pit);

		let mut heap = BinaryHeap::<CostNode>::new();
		heap.push(CostNode { cost: 0.0, steps: 0, i: pit });

		let mut solved = false;
		while let Some(node) = heap.pop() {
			if node.cost > max_cost {
				break;
			}
			let length = path_length[node.i] as usize;
			if length > max_dist {
				continue;
			}
			let zn_node = data[node.i];
			let cost1 = zn_node - pit_z + (length as f64 * small);

			let r = node.i / cols;
			let c = node.i % cols;
			for k in 0..8 {
				let rn = r as isize + DY[k];
				let cn = c as isize + DX[k];
				if !in_bounds(rn, cn, rows, cols) {
					continue;
				}
				let ni = idx(rn as usize, cn as usize, cols);
				if encountered[ni] == 1 {
					continue;
				}
				touched.push(ni);
				let next_steps = length + 1;
				path_length[ni] = next_steps as i16;
				backlink[ni] = backlink_dir[k] as i8;
				let zn = data[ni];
				let zout = pit_z - (next_steps as f64 * small);
				if zn > zout && zn != nodata {
					let cost2 = zn - zout;
					let next_cost = if minimize_dist {
						node.cost + ((cost1 + cost2) / 2.0) * step_dist[k]
					} else {
						node.cost + cost2
					};
					encountered[ni] = 1;
					if next_steps <= max_dist {
						heap.push(CostNode {
							cost: next_cost,
							steps: next_steps,
							i: ni,
						});
					}
				} else {
					let mut ti = ni;
					while backlink[ti] > -1 {
						let b = backlink[ti] as usize;
						let tr = ti / cols;
						let tc = ti % cols;
						let pr = (tr as isize + DY[b]) as usize;
						let pc = (tc as isize + DX[b]) as usize;
						ti = idx(pr, pc, cols);
						let l = path_length[ti] as f64;
						let desired = pit_z - l * small;
						if data[ti] > desired {
							data[ti] = desired;
						}
					}
					solved = true;
					break;
				}
			}
			if solved {
				break;
			}
		}

		for i in touched.drain(..) {
			encountered[i] = 0;
			backlink[i] = -1;
			path_length[i] = 0;
		}
	}
}

fn parse_dem_and_output(args: &ToolArgs) -> Result<(Raster, Option<std::path::PathBuf>), ToolError> {
	let dem_path = parse_raster_path_arg(args, "dem")
		.or_else(|_| parse_raster_path_arg(args, "input"))
		.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
	let output_path = parse_optional_output_path(args, "output")?;
	let dem = load_raster(&dem_path)?;
	Ok((dem, output_path))
}

fn run_fill_like(
	args: &ToolArgs,
	mode: &str,
) -> Result<ToolRunResult, ToolError> {
	let (dem, output_path) = parse_dem_and_output(args)?;
	let mut data = raster_to_vec(&dem);
	let flat_increment = args.get("flat_increment").and_then(|v| v.as_f64());
	let fix_flats = args.get("fix_flats").and_then(|v| v.as_bool()).unwrap_or(true);
	let small = auto_small_increment_legacy(&dem, &data, flat_increment, fix_flats);
	data = match mode {
		"fill_depressions" => {
			let max_depth = args.get("max_depth").and_then(|v| v.as_f64()).unwrap_or(f64::INFINITY);
			fill_depressions_core(&data, dem.rows, dem.cols, dem.nodata, small, max_depth, fix_flats)
		}
		"wang_liu" => fill_depressions_wang_and_liu_core(&data, dem.rows, dem.cols, dem.nodata, if fix_flats { small } else { 0.0 }),
		"planchon" => fill_depressions_planchon_and_darboux_core(&data, dem.rows, dem.cols, dem.nodata, if fix_flats { small } else { 0.0 }),
		_ => {
			return Err(ToolError::Validation("unsupported fill mode".to_string()));
		}
	};

	let out = vec_to_raster(&dem, &data, DataType::F64);
	Ok(build_result(write_or_store_output(out, output_path)?))
}

fn d8_dir_from_dem_local(input: &Raster) -> Vec<i8> {
	let rows = input.rows;
	let cols = input.cols;
	let nodata = input.nodata;
	let mut dirs = vec![-2i8; rows * cols];
	let cell_x = input.cell_size_x;
	let cell_y = input.cell_size_y;
	let diag = (cell_x * cell_x + cell_y * cell_y).sqrt();
	let lengths = [diag, cell_x, diag, cell_y, diag, cell_x, diag, cell_y];

	for r in 0..rows {
		for c in 0..cols {
			let i = idx(r, c, cols);
			let z = input.get(0, r as isize, c as isize);
			if z == nodata {
				continue;
			}
			let mut best_dir = -1i8;
			let mut best_slope = f64::MIN;
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
				let slope = (z - zn) / lengths[k];
				if slope > 0.0 && slope > best_slope {
					best_slope = slope;
					best_dir = k as i8;
				}
			}
			dirs[i] = best_dir;
		}
	}

	dirs
}

fn num_inflowing_from_d8(flow_dir: &[i8], rows: usize, cols: usize, nodata: f64, input: &Raster) -> Vec<f64> {
	let inflowing_vals: [i8; 8] = [4, 5, 6, 7, 0, 1, 2, 3];
	let mut counts = vec![-32768.0; rows * cols];
	for r in 0..rows {
		for c in 0..cols {
			let i = idx(r, c, cols);
			if input.get(0, r as isize, c as isize) == nodata {
				continue;
			}
			let mut count = 0.0;
			for k in 0..8 {
				let rn = r as isize + DY[k];
				let cn = c as isize + DX[k];
				if !in_bounds(rn, cn, rows, cols) {
					continue;
				}
				let ni = idx(rn as usize, cn as usize, cols);
				if flow_dir[ni] == inflowing_vals[k] {
					count += 1.0;
				}
			}
			counts[i] = count;
		}
	}
	counts
}

fn decode_d8_pointer_dir(value: f64, esri_style: bool) -> i8 {
	if value <= 0.0 {
		return -1;
	}
	let mut pntr_matches: [i8; 129] = [0i8; 129];
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
	let idx = value as usize;
	if idx < pntr_matches.len() { pntr_matches[idx] } else { -1 }
}

fn decode_d8_pointer_dir_checked(value: f64, esri_style: bool) -> Result<i8, ToolError> {
	if !value.is_finite() {
		return Err(ToolError::Validation(
			"pointer raster contains non-finite values".to_string(),
		));
	}
	let v = value as i64;
	if !matches!(v, 1 | 2 | 4 | 8 | 16 | 32 | 64 | 128) {
		return Err(ToolError::Validation(
			"pointer raster contains unexpected values; expected D8/Rho8 pointer encoding".to_string(),
		));
	}
	let dir = decode_d8_pointer_dir(value, esri_style);
	if !(0..=7).contains(&dir) {
		return Err(ToolError::Validation(
			"failed to decode pointer direction".to_string(),
		));
	}
	Ok(dir)
}

fn parse_pointer_input(args: &ToolArgs) -> Result<(Raster, Option<std::path::PathBuf>), ToolError> {
	let path = parse_raster_path_arg(args, "d8_pntr")
		.or_else(|_| parse_raster_path_arg(args, "d8_pointer"))
		.or_else(|_| parse_raster_path_arg(args, "input"))?;
	let output_path = parse_optional_output_path(args, "output")?;
	Ok((load_raster(&path)?, output_path))
}

fn parse_optional_output_from_keys(args: &ToolArgs, keys: &[&str]) -> Result<Option<std::path::PathBuf>, ToolError> {
	for key in keys {
		if args.get(*key).is_some() {
			return parse_optional_output_path(args, key);
		}
	}
	Ok(None)
}

fn make_indexed_output_path(base: &std::path::Path, idx_1based: usize) -> std::path::PathBuf {
	let stem = base
		.file_stem()
		.and_then(|s| s.to_str())
		.unwrap_or("output")
		.to_string();
	let ext = base.extension().and_then(|s| s.to_str()).unwrap_or("");
	let name = if ext.is_empty() {
		format!("{}_{}", stem, idx_1based)
	} else {
		format!("{}_{}.{}", stem, idx_1based, ext)
	};
	base.with_file_name(name)
}

fn unique_temp_path(prefix: &str, extension: &str) -> std::path::PathBuf {
	let stamp = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map(|d| d.as_nanos())
		.unwrap_or(0);
	let name = format!("{}_{}_{}.{}", prefix, std::process::id(), stamp, extension.trim_start_matches('.'));
	std::env::temp_dir().join(name)
}

fn gaussian_noise_box_muller(rng: &mut impl rand::RngExt) -> f64 {
	let u1: f64 = rng.random::<f64>().clamp(1.0e-12, 1.0 - 1.0e-12);
	let u2: f64 = rng.random::<f64>();
	(-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

fn box_mean_filter_valid(data: &[f64], rows: usize, cols: usize, nodata: f64, radius: usize) -> Vec<f64> {
	if radius == 0 {
		return data.to_vec();
	}
	let stride = cols + 1;
	let mut integral_sum = vec![0.0f64; (rows + 1) * (cols + 1)];
	let mut integral_count = vec![0u32; (rows + 1) * (cols + 1)];
	for r in 0..rows {
		let mut row_sum = 0.0;
		let mut row_count = 0u32;
		for c in 0..cols {
			let z = data[idx(r, c, cols)];
			if z != nodata {
				row_sum += z;
				row_count += 1;
			}
			let ii = (r + 1) * stride + (c + 1);
			integral_sum[ii] = integral_sum[r * stride + (c + 1)] + row_sum;
			integral_count[ii] = integral_count[r * stride + (c + 1)] + row_count;
		}
	}

	let mut out = vec![nodata; rows * cols];
	for r in 0..rows {
		let r0 = r.saturating_sub(radius);
		let r1 = (r + radius + 1).min(rows);
		for c in 0..cols {
			let i = idx(r, c, cols);
			if data[i] == nodata {
				continue;
			}
			let c0 = c.saturating_sub(radius);
			let c1 = (c + radius + 1).min(cols);
			let a = r0 * stride + c0;
			let b = r0 * stride + c1;
			let cidx = r1 * stride + c0;
			let d = r1 * stride + c1;
			let sum = integral_sum[d] + integral_sum[a] - integral_sum[b] - integral_sum[cidx];
			let count = integral_count[d] + integral_count[a] - integral_count[b] - integral_count[cidx];
			out[i] = if count > 0 { sum / count as f64 } else { data[i] };
		}
	}
	out
}

/// Build a D8 flow-direction vec from a pointer raster, updating `out` to
/// `out_nodata` wherever the pointer holds NoData. Returns direction-index vec
/// (-2 = uninitialized, -1 = no-flow/flat, 0-7 = direction).
fn build_flow_dir_and_mark_nodata(
	pntr: &Raster,
	esri_style: bool,
	out: &mut Vec<f64>,
	out_nodata: f64,
	cols: usize,
) -> Vec<i8> {
	let nodata = pntr.nodata;
	let rows = pntr.rows;
	let mut flow_dir = vec![-2i8; rows * cols];
	for r in 0..rows {
		for c in 0..cols {
			let i = idx(r, c, cols);
			let z = pntr.get(0, r as isize, c as isize);
			if z == nodata {
				out[i] = out_nodata;
			} else {
				flow_dir[i] = decode_d8_pointer_dir(z, esri_style);
			}
		}
	}
	flow_dir
}

/// Two-pass upstream watershed labeling. For each still-unlabeled cell, walks
/// downstream to find an already-labeled outlet, then walks again to label the
/// whole path with that outlet's ID.
fn run_watershed_labeling(
	out: &mut Vec<f64>,
	flow_dir: &[i8],
	rows: usize,
	cols: usize,
	low_value: f64,
	out_nodata: f64,
) {
	for r in 0..rows {
		for c in 0..cols {
			let i = idx(r, c, cols);
			if out[i] != low_value {
				continue;
			}
			let (mut y, mut x) = (r as isize, c as isize);
			let mut outlet_id = out_nodata;
			loop {
				let ii = idx(y as usize, x as usize, cols);
				let dir = flow_dir[ii];
				if dir >= 0 {
					y += DY[dir as usize];
					x += DX[dir as usize];
					if !in_bounds(y, x, rows, cols) {
						break;
					}
					let zn = out[idx(y as usize, x as usize, cols)];
					if zn != low_value {
						outlet_id = zn;
						break;
					}
				} else {
					break;
				}
			}
			let (mut y, mut x) = (r as isize, c as isize);
			out[i] = outlet_id;
			loop {
				let ii = idx(y as usize, x as usize, cols);
				let dir = flow_dir[ii];
				if dir >= 0 {
					y += DY[dir as usize];
					x += DX[dir as usize];
					if !in_bounds(y, x, rows, cols) {
						break;
					}
					let ni = idx(y as usize, x as usize, cols);
					if out[ni] != low_value {
						break;
					}
					out[ni] = outlet_id;
				} else {
					break;
				}
			}
		}
	}
}

fn set_mask_cell(mask: &mut [u8], rows: usize, cols: usize, r: isize, c: isize) {
	if in_bounds(r, c, rows, cols) {
		mask[idx(r as usize, c as usize, cols)] = 1;
	}
}

fn draw_line_cells(mask: &mut [u8], rows: usize, cols: usize, r0: isize, c0: isize, r1: isize, c1: isize) {
	let (mut x0, mut y0) = (c0, r0);
	let (x1, y1) = (c1, r1);
	let dx = (x1 - x0).abs();
	let sx = if x0 < x1 { 1 } else { -1 };
	let dy = -(y1 - y0).abs();
	let sy = if y0 < y1 { 1 } else { -1 };
	let mut err = dx + dy;
	loop {
		set_mask_cell(mask, rows, cols, y0, x0);
		if x0 == x1 && y0 == y1 {
			break;
		}
		let e2 = 2 * err;
		if e2 >= dy {
			err += dy;
			x0 += sx;
		}
		if e2 <= dx {
			err += dx;
			y0 += sy;
		}
	}
}

fn draw_line_cells_collect(cells: &mut Vec<usize>, seen: &mut HashSet<usize>, rows: usize, cols: usize, r0: isize, c0: isize, r1: isize, c1: isize) {
	let (mut x0, mut y0) = (c0, r0);
	let (x1, y1) = (c1, r1);
	let dx = (x1 - x0).abs();
	let sx = if x0 < x1 { 1 } else { -1 };
	let dy = -(y1 - y0).abs();
	let sy = if y0 < y1 { 1 } else { -1 };
	let mut err = dx + dy;
	loop {
		if in_bounds(y0, x0, rows, cols) {
			let i = idx(y0 as usize, x0 as usize, cols);
			if seen.insert(i) {
				cells.push(i);
			}
		}
		if x0 == x1 && y0 == y1 {
			break;
		}
		let e2 = 2 * err;
		if e2 >= dy {
			err += dy;
			x0 += sx;
		}
		if e2 <= dx {
			err += dx;
			y0 += sy;
		}
	}
}

fn collect_line_cells_geometry(rows: usize, cols: usize, dem: &Raster, geom: &wbvector::Geometry) -> Vec<usize> {
	let mut cells = Vec::<usize>::new();
	let mut seen = HashSet::<usize>::new();
	match geom {
		wbvector::Geometry::LineString(coords) => {
			for seg in coords.windows(2) {
				if let (Some((c0, r0)), Some((c1, r1))) = (
					dem.world_to_pixel(seg[0].x, seg[0].y),
					dem.world_to_pixel(seg[1].x, seg[1].y),
				) {
					draw_line_cells_collect(&mut cells, &mut seen, rows, cols, r0, c0, r1, c1);
				}
			}
		}
		wbvector::Geometry::MultiLineString(lines) => {
			for ls in lines {
				for seg in ls.windows(2) {
					if let (Some((c0, r0)), Some((c1, r1))) = (
						dem.world_to_pixel(seg[0].x, seg[0].y),
						dem.world_to_pixel(seg[1].x, seg[1].y),
					) {
						draw_line_cells_collect(&mut cells, &mut seen, rows, cols, r0, c0, r1, c1);
					}
				}
			}
		}
		_ => {}
	}
	cells
}

fn rasterize_line_geometry(mask: &mut [u8], rows: usize, cols: usize, dem: &Raster, geom: &wbvector::Geometry) {
	match geom {
		wbvector::Geometry::LineString(coords) => {
			for seg in coords.windows(2) {
				if let (Some((c0, r0)), Some((c1, r1))) = (
					dem.world_to_pixel(seg[0].x, seg[0].y),
					dem.world_to_pixel(seg[1].x, seg[1].y),
				) {
					draw_line_cells(mask, rows, cols, r0, c0, r1, c1);
				}
			}
		}
		wbvector::Geometry::MultiLineString(lines) => {
			for ls in lines {
				for seg in ls.windows(2) {
					if let (Some((c0, r0)), Some((c1, r1))) = (
						dem.world_to_pixel(seg[0].x, seg[0].y),
						dem.world_to_pixel(seg[1].x, seg[1].y),
					) {
						draw_line_cells(mask, rows, cols, r0, c0, r1, c1);
					}
				}
			}
		}
		_ => {}
	}
}

fn rasterize_polygon_boundaries(mask: &mut [u8], rows: usize, cols: usize, dem: &Raster, geom: &wbvector::Geometry) {
	match geom {
		wbvector::Geometry::Polygon { exterior, interiors } => {
			for seg in exterior.0.windows(2) {
				if let (Some((c0, r0)), Some((c1, r1))) = (
					dem.world_to_pixel(seg[0].x, seg[0].y),
					dem.world_to_pixel(seg[1].x, seg[1].y),
				) {
					draw_line_cells(mask, rows, cols, r0, c0, r1, c1);
				}
			}
			for ring in interiors {
				for seg in ring.0.windows(2) {
					if let (Some((c0, r0)), Some((c1, r1))) = (
						dem.world_to_pixel(seg[0].x, seg[0].y),
						dem.world_to_pixel(seg[1].x, seg[1].y),
					) {
						draw_line_cells(mask, rows, cols, r0, c0, r1, c1);
					}
				}
			}
		}
		wbvector::Geometry::MultiPolygon(polys) => {
			for (exterior, interiors) in polys {
				for seg in exterior.0.windows(2) {
					if let (Some((c0, r0)), Some((c1, r1))) = (
						dem.world_to_pixel(seg[0].x, seg[0].y),
						dem.world_to_pixel(seg[1].x, seg[1].y),
					) {
						draw_line_cells(mask, rows, cols, r0, c0, r1, c1);
					}
				}
				for ring in interiors {
					for seg in ring.0.windows(2) {
						if let (Some((c0, r0)), Some((c1, r1))) = (
							dem.world_to_pixel(seg[0].x, seg[0].y),
							dem.world_to_pixel(seg[1].x, seg[1].y),
						) {
							draw_line_cells(mask, rows, cols, r0, c0, r1, c1);
						}
					}
				}
			}
		}
		_ => {}
	}
}

fn read_vector_layer_aligned_to_dem(dem: &Raster, path: &str, input_name: &str) -> Result<wbvector::Layer, ToolError> {
	let layer = wbvector::read(path).map_err(|e| {
		ToolError::Validation(format!("failed reading {} vector '{}': {}", input_name, path, e))
	})?;

	let dem_epsg = dem.crs.epsg;
	let dem_wkt = dem.crs.wkt.as_deref().map(str::trim).filter(|s| !s.is_empty());
	let layer_epsg = layer.crs_epsg();
	let layer_wkt = layer.crs_wkt().map(str::trim).filter(|s| !s.is_empty());

	if dem_epsg.is_none() && dem_wkt.is_none() {
		return Ok(layer);
	}

	if layer_epsg.is_none() && layer_wkt.is_none() {
		return Err(ToolError::Validation(format!(
			"{} vector has no CRS metadata; cannot verify alignment with DEM CRS",
			input_name
		)));
	}

	let epsg_matches = dem_epsg.is_some() && layer_epsg == dem_epsg;
	let wkt_matches = match (dem_wkt, layer_wkt) {
		(Some(a), Some(b)) => a == b,
		_ => false,
	};
	if epsg_matches || wkt_matches {
		return Ok(layer);
	}

	if let Some(dst_epsg) = dem_epsg {
		let reprojected = layer.reproject_to_epsg(dst_epsg).map_err(|e| {
			ToolError::Validation(format!(
				"{} vector CRS does not match DEM CRS; automatic reprojection to EPSG:{} failed: {}",
				input_name, dst_epsg, e
			))
		})?;
		return Ok(reprojected);
	}

	Err(ToolError::Validation(format!(
		"{} vector CRS does not match DEM CRS and DEM has no EPSG code for automatic reprojection",
		input_name
	)))
}

fn stream_mask_from_vector(dem: &Raster, path: &str, input_name: &str) -> Result<Vec<u8>, ToolError> {
	let layer = read_vector_layer_aligned_to_dem(dem, path, input_name)?;
	let rows = dem.rows;
	let cols = dem.cols;
	let mut mask = vec![0u8; rows * cols];
	for feat in &layer.features {
		if let Some(ref g) = feat.geometry {
			rasterize_line_geometry(&mut mask, rows, cols, dem, g);
		}
	}
	Ok(mask)
}

fn ring_contains_xy(ring: &wbvector::Ring, x: f64, y: f64) -> bool {
	let pts = &ring.0;
	if pts.len() < 3 {
		return false;
	}
	let mut inside = false;
	let mut j = pts.len() - 1;
	for i in 0..pts.len() {
		let xi = pts[i].x;
		let yi = pts[i].y;
		let xj = pts[j].x;
		let yj = pts[j].y;
		let intersects = if (yi > y) != (yj > y) {
			let mut denom = yj - yi;
			if denom.abs() < 1.0e-15 {
				denom = 1.0e-15;
			}
			let x_cross = (xj - xi) * (y - yi) / denom + xi;
			x < x_cross
		} else {
			false
		};
		if intersects {
			inside = !inside;
		}
		j = i;
	}
	inside
}

fn polygon_contains_xy(exterior: &wbvector::Ring, interiors: &[wbvector::Ring], x: f64, y: f64) -> bool {
	if !ring_contains_xy(exterior, x, y) {
		return false;
	}
	for hole in interiors {
		if ring_contains_xy(hole, x, y) {
			return false;
		}
	}
	true
}

fn polygon_bbox_pixels(dem: &Raster, exterior: &wbvector::Ring) -> Option<(usize, usize, usize, usize)> {
	let mut rmin = usize::MAX;
	let mut cmin = usize::MAX;
	let mut rmax = 0usize;
	let mut cmax = 0usize;
	let mut found = false;
	for p in &exterior.0 {
		if let Some((c, r)) = dem.world_to_pixel(p.x, p.y) {
			if in_bounds(r, c, dem.rows, dem.cols) {
				let ru = r as usize;
				let cu = c as usize;
				rmin = rmin.min(ru);
				cmin = cmin.min(cu);
				rmax = rmax.max(ru);
				cmax = cmax.max(cu);
				found = true;
			}
		}
	}
	if found {
		Some((rmin, cmin, rmax, cmax))
	} else {
		None
	}
}

#[derive(Clone, Copy)]
struct DtwNode {
	cost: f64,
	i: usize,
}

impl PartialEq for DtwNode {
	fn eq(&self, other: &Self) -> bool {
		self.i == other.i && self.cost.to_bits() == other.cost.to_bits()
	}
}
impl Eq for DtwNode {}
impl PartialOrd for DtwNode {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		other.cost.partial_cmp(&self.cost)
	}
}
impl Ord for DtwNode {
	fn cmp(&self, other: &Self) -> Ordering {
		self.partial_cmp(other).unwrap_or(Ordering::Equal)
	}
}

impl Tool for BreachDepressionsLeastCostTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "breach_depressions_least_cost",
			display_name: "Breach Depressions Least Cost",
			summary: "Breaches depressions in a DEM using a constrained least-cost pathway search.",
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec {
					name: "dem",
					description: "Input DEM raster",
					required: true,
				},
				ToolParamSpec {
					name: "max_cost",
					description: "Optional maximum breach cost",
					required: false,
				},
				ToolParamSpec {
					name: "max_dist",
					description: "Maximum search distance in cells",
					required: false,
				},
				ToolParamSpec {
					name: "flat_increment",
					description: "Optional flat increment to ensure downslope flow",
					required: false,
				},
				ToolParamSpec {
					name: "fill_deps",
					description: "Optionally fill unresolved depressions after breaching",
					required: false,
				},
				ToolParamSpec {
					name: "minimize_dist",
					description: "Weight breach costs by travel distance",
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
		defaults.insert("max_cost".to_string(), json!(f64::INFINITY));
		defaults.insert("max_dist".to_string(), json!(100));
		defaults.insert("fill_deps".to_string(), json!(false));
		defaults.insert("minimize_dist".to_string(), json!(false));
		ToolManifest {
			id: "breach_depressions_least_cost".to_string(),
			display_name: "Breach Depressions Least Cost".to_string(),
			summary: "Breaches depressions in a DEM using a constrained least-cost pathway search.".to_string(),
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "breach_dem".to_string(),
				description: "Breach depressions before hydrologic flow modeling".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "depression".to_string(), "dem".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let max_cost = args.get("max_cost").and_then(|v| v.as_f64()).unwrap_or(f64::INFINITY);
		let max_dist = args.get("max_dist").and_then(|v| v.as_i64()).unwrap_or(100).max(1) as usize;
		let fill_deps = args.get("fill_deps").and_then(|v| v.as_bool()).unwrap_or(false);
		let minimize_dist = args.get("minimize_dist").and_then(|v| v.as_bool()).unwrap_or(false);
		let mut data = raster_to_vec(&dem);
		let small = auto_small_increment_legacy(&dem, &data, args.get("flat_increment").and_then(|v| v.as_f64()), true);

		breach_depressions_least_cost_core(
			&mut data,
			dem.rows,
			dem.cols,
			dem.nodata,
			max_cost,
			max_dist,
			small,
			minimize_dist,
		);

		if fill_deps {
			data = fill_depressions_core(&data, dem.rows, dem.cols, dem.nodata, small, f64::INFINITY, true);
		}

		let out = vec_to_raster(&dem, &data, DataType::F64);
		Ok(build_result(write_or_store_output(out, output_path)?))
	}
}

impl Tool for BreachSingleCellPitsTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "breach_single_cell_pits",
			display_name: "Breach Single-Cell Pits",
			summary: "Breaches single-cell pits in a DEM by carving one-cell channels.",
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec {
					name: "dem",
					description: "Input DEM raster",
					required: true,
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
		ToolManifest {
			id: "breach_single_cell_pits".to_string(),
			display_name: "Breach Single-Cell Pits".to_string(),
			summary: "Breaches single-cell pits in a DEM by carving one-cell channels.".to_string(),
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults: ToolArgs::new(),
			examples: vec![],
			tags: vec!["hydrology".to_string(), "depression".to_string(), "dem".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let mut data = raster_to_vec(&dem);
		breach_single_cell_pits_core(&mut data, dem.rows, dem.cols, dem.nodata);
		let out = vec_to_raster(&dem, &data, DataType::F64);
		Ok(build_result(write_or_store_output(out, output_path)?))
	}
}

impl Tool for FillDepressionsTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "fill_depressions",
			display_name: "Fill Depressions",
			summary: "Fills depressions in a DEM using a priority-flood strategy with optional flat resolution.",
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec {
					name: "dem",
					description: "Input DEM raster",
					required: true,
				},
				ToolParamSpec {
					name: "fix_flats",
					description: "Apply a small gradient over flats",
					required: false,
				},
				ToolParamSpec {
					name: "flat_increment",
					description: "Optional flat increment",
					required: false,
				},
				ToolParamSpec {
					name: "max_depth",
					description: "Optional maximum fill depth",
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
		defaults.insert("fix_flats".to_string(), json!(true));
		defaults.insert("max_depth".to_string(), json!(f64::INFINITY));
		ToolManifest {
			id: "fill_depressions".to_string(),
			display_name: "Fill Depressions".to_string(),
			summary: "Fills depressions in a DEM using a priority-flood strategy with optional flat resolution.".to_string(),
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![],
			tags: vec!["hydrology".to_string(), "depression".to_string(), "dem".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		run_fill_like(args, "fill_depressions")
	}
}

impl Tool for FillDepressionsPlanchonAndDarbouxTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "fill_depressions_planchon_and_darboux",
			display_name: "Fill Depressions (Planchon and Darboux)",
			summary: "Fills depressions in a DEM with a Planchon-and-Darboux-compatible interface.",
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec {
					name: "dem",
					description: "Input DEM raster",
					required: true,
				},
				ToolParamSpec {
					name: "fix_flats",
					description: "Apply a small gradient over flats",
					required: false,
				},
				ToolParamSpec {
					name: "flat_increment",
					description: "Optional flat increment",
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
		defaults.insert("fix_flats".to_string(), json!(true));
		ToolManifest {
			id: "fill_depressions_planchon_and_darboux".to_string(),
			display_name: "Fill Depressions (Planchon and Darboux)".to_string(),
			summary: "Fills depressions in a DEM with a Planchon-and-Darboux-compatible interface.".to_string(),
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![],
			tags: vec!["hydrology".to_string(), "depression".to_string(), "dem".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		run_fill_like(args, "planchon")
	}
}

impl Tool for FillDepressionsWangAndLiuTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "fill_depressions_wang_and_liu",
			display_name: "Fill Depressions (Wang and Liu)",
			summary: "Fills depressions in a DEM with a Wang-and-Liu-compatible interface.",
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec {
					name: "dem",
					description: "Input DEM raster",
					required: true,
				},
				ToolParamSpec {
					name: "fix_flats",
					description: "Apply a small gradient over flats",
					required: false,
				},
				ToolParamSpec {
					name: "flat_increment",
					description: "Optional flat increment",
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
		defaults.insert("fix_flats".to_string(), json!(true));
		ToolManifest {
			id: "fill_depressions_wang_and_liu".to_string(),
			display_name: "Fill Depressions (Wang and Liu)".to_string(),
			summary: "Fills depressions in a DEM with a Wang-and-Liu-compatible interface.".to_string(),
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![],
			tags: vec!["hydrology".to_string(), "depression".to_string(), "dem".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		run_fill_like(args, "wang_liu")
	}
}

impl Tool for FillPitsTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "fill_pits",
			display_name: "Fill Pits",
			summary: "Fills single-cell pits in a DEM.",
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec {
					name: "dem",
					description: "Input DEM raster",
					required: true,
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
		ToolManifest {
			id: "fill_pits".to_string(),
			display_name: "Fill Pits".to_string(),
			summary: "Fills single-cell pits in a DEM.".to_string(),
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults: ToolArgs::new(),
			examples: vec![],
			tags: vec!["hydrology".to_string(), "depression".to_string(), "dem".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let small = auto_small_increment(&dem, None);
		let mut data = raster_to_vec(&dem);
		fill_pits_core(&mut data, dem.rows, dem.cols, dem.nodata, small);
		let out = vec_to_raster(&dem, &data, DataType::F64);
		Ok(build_result(write_or_store_output(out, output_path)?))
	}
}

impl Tool for DepthInSinkTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "depth_in_sink",
			display_name: "Depth in Sink",
			summary: "Measures the depth each DEM cell lies below a depression-filled surface.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
				ToolParamSpec { name: "zero_background", description: "Set non-sink cells to 0.0 (otherwise NoData)", required: false },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		let mut defaults = ToolArgs::new();
		defaults.insert("zero_background".to_string(), json!(false));
		ToolManifest {
			id: "depth_in_sink".to_string(),
			display_name: "Depth in Sink".to_string(),
			summary: "Measures the depth each DEM cell lies below a depression-filled surface.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "depth_in_sink_example".to_string(),
				description: "Compute sink depth from a DEM".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "depression".to_string(), "sink".to_string(), "dem".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let zero_background = args.get("zero_background").and_then(|v| v.as_bool()).unwrap_or(false);
		let data = raster_to_vec(&dem);
		let filled = fill_depressions_core(&data, dem.rows, dem.cols, dem.nodata, 0.0, f64::INFINITY, false);
		let background = if zero_background { 0.0 } else { dem.nodata };
		let mut out = vec![background; dem.rows * dem.cols];
		for i in 0..out.len() {
			let z = data[i];
			if z == dem.nodata {
				out[i] = dem.nodata;
				continue;
			}
			let depth = filled[i] - z;
			if depth > 0.0 {
				out[i] = depth;
			}
		}
		let output = vec_to_raster(&dem, &out, DataType::F64);
		Ok(build_result(write_or_store_output(output, output_path)?))
	}
}

impl Tool for SinkTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "sink",
			display_name: "Sink",
			summary: "Identifies cells that belong to topographic depressions in a DEM.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
				ToolParamSpec { name: "zero_background", description: "Set non-sink cells to 0.0 (otherwise NoData)", required: false },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		let mut defaults = ToolArgs::new();
		defaults.insert("zero_background".to_string(), json!(false));
		ToolManifest {
			id: "sink".to_string(),
			display_name: "Sink".to_string(),
			summary: "Identifies cells that belong to topographic depressions in a DEM.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "sink_example".to_string(),
				description: "Map sink cells in a DEM".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "depression".to_string(), "sink".to_string(), "dem".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let zero_background = args.get("zero_background").and_then(|v| v.as_bool()).unwrap_or(false);
		let data = raster_to_vec(&dem);
		let filled = fill_depressions_core(&data, dem.rows, dem.cols, dem.nodata, 0.0, f64::INFINITY, false);
		let background = if zero_background { 0.0 } else { dem.nodata };
		let mut out = vec![background; dem.rows * dem.cols];
		for i in 0..out.len() {
			let z = data[i];
			if z == dem.nodata {
				out[i] = dem.nodata;
				continue;
			}
			if filled[i] > z {
				out[i] = 1.0;
			}
		}
		let output = vec_to_raster(&dem, &out, DataType::I16);
		Ok(build_result(write_or_store_output(output, output_path)?))
	}
}

impl Tool for FlowAccumFullWorkflowTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "flow_accum_full_workflow",
			display_name: "Flow Accum Full Workflow",
			summary: "Runs a full non-divergent flow-accumulation workflow and returns breached DEM, flow-direction pointer, and accumulation.",
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec {
					name: "dem",
					description: "Input DEM raster",
					required: true,
				},
				ToolParamSpec {
					name: "out_type",
					description: "Output type: cells, ca, or sca",
					required: false,
				},
				ToolParamSpec {
					name: "log_transform",
					description: "Log-transform accumulation",
					required: false,
				},
				ToolParamSpec {
					name: "clip",
					description: "Clip display max for accumulation",
					required: false,
				},
				ToolParamSpec {
					name: "esri_pntr",
					description: "Use ESRI pointer encoding",
					required: false,
				},
				ToolParamSpec {
					name: "breached_dem_output",
					description: "Optional output path for breached DEM",
					required: false,
				},
				ToolParamSpec {
					name: "flow_dir_output",
					description: "Optional output path for flow-direction raster",
					required: false,
				},
				ToolParamSpec {
					name: "output",
					description: "Optional output path for flow-accumulation raster",
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
		defaults.insert("esri_pntr".to_string(), json!(false));
		ToolManifest {
			id: "flow_accum_full_workflow".to_string(),
			display_name: "Flow Accum Full Workflow".to_string(),
			summary: "Runs a full non-divergent flow-accumulation workflow and returns breached DEM, flow-direction pointer, and accumulation.".to_string(),
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "full_workflow".to_string(),
				description: "Run depression filling + D8 pointer + D8 flow accumulation in one call".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "flow-accumulation".to_string(), "workflow".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let dem_path = parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		let breached_dem_output = parse_optional_output_path(args, "breached_dem_output")?;
		let flow_dir_output = parse_optional_output_path(args, "flow_dir_output")?;
		let flow_accum_output = parse_optional_output_path(args, "output")?;

		let out_type = args
			.get("out_type")
			.and_then(|v| v.as_str())
			.unwrap_or("sca");
		let log_transform = args
			.get("log_transform")
			.or_else(|| args.get("log"))
			.and_then(|v| v.as_bool())
			.unwrap_or(false);
		let clip = args
			.get("clip")
			.and_then(|v| v.as_bool())
			.unwrap_or(false);
		let esri_pntr = args
			.get("esri_pntr")
			.or_else(|| args.get("esri_pointer"))
			.and_then(|v| v.as_bool())
			.unwrap_or(false);

		let mut fill_args = ToolArgs::new();
		fill_args.insert("dem".to_string(), json!(dem_path));
		fill_args.insert("fix_flats".to_string(), json!(true));
		if let Some(path) = breached_dem_output {
			fill_args.insert("output".to_string(), json!(path.to_string_lossy().to_string()));
		}
		let fill_tool = FillDepressionsTool;
		let filled = fill_tool.run(&fill_args, ctx)?;
		let filled_path = filled
			.outputs
			.get("path")
			.and_then(|v| v.as_str())
			.ok_or_else(|| ToolError::Execution("fill_depressions did not return an output path".to_string()))?
			.to_string();

		let mut d8_ptr_args = ToolArgs::new();
		d8_ptr_args.insert("dem".to_string(), json!(filled_path.clone()));
		d8_ptr_args.insert("esri_pntr".to_string(), json!(esri_pntr));
		if let Some(path) = flow_dir_output {
			d8_ptr_args.insert("output".to_string(), json!(path.to_string_lossy().to_string()));
		}
		let d8_ptr_tool = D8PointerTool;
		let flow_ptr = d8_ptr_tool.run(&d8_ptr_args, ctx)?;
		let flow_ptr_path = flow_ptr
			.outputs
			.get("path")
			.and_then(|v| v.as_str())
			.ok_or_else(|| ToolError::Execution("d8_pointer did not return an output path".to_string()))?
			.to_string();

		let mut accum_args = ToolArgs::new();
		accum_args.insert("input".to_string(), json!(flow_ptr_path.clone()));
		accum_args.insert("input_is_pointer".to_string(), json!(true));
		accum_args.insert("out_type".to_string(), json!(out_type));
		accum_args.insert("log_transform".to_string(), json!(log_transform));
		accum_args.insert("clip".to_string(), json!(clip));
		accum_args.insert("esri_pntr".to_string(), json!(esri_pntr));
		if let Some(path) = flow_accum_output {
			accum_args.insert("output".to_string(), json!(path.to_string_lossy().to_string()));
		}
		let d8_accum_tool = D8FlowAccumTool;
		let accum = d8_accum_tool.run(&accum_args, ctx)?;
		let accum_path = accum
			.outputs
			.get("path")
			.and_then(|v| v.as_str())
			.ok_or_else(|| ToolError::Execution("d8_flow_accum did not return an output path".to_string()))?
			.to_string();

		Ok(build_triple_raster_result(filled_path, flow_ptr_path, accum_path))
	}
}

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
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		ToolManifest {
			id: "find_noflow_cells".to_string(),
			display_name: "Find Noflow Cells".to_string(),
			summary: "Finds DEM cells that have no lower D8 neighbour.".to_string(),
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults: ToolArgs::new(),
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
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let rows = dem.rows;
		let cols = dem.cols;
		let nodata = dem.nodata;
		let dirs = d8_dir_from_dem_local(&dem);
		let mut data = vec![nodata; rows * cols];
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				if dem.get(0, r as isize, c as isize) == nodata {
					continue;
				}
				if dirs[i] < 0 {
					data[i] = 1.0;
				} else {
					data[i] = nodata;
				}
			}
		}
		let out = vec_to_raster(&dem, &data, DataType::F64);
		Ok(build_result(write_or_store_output(out, output_path)?))
	}
}

impl Tool for NumInflowingNeighboursTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "num_inflowing_neighbours",
			display_name: "Num Inflowing Neighbours",
			summary: "Counts the number of inflowing D8 neighbours for each DEM cell.",
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
			id: "num_inflowing_neighbours".to_string(),
			display_name: "Num Inflowing Neighbours".to_string(),
			summary: "Counts the number of inflowing D8 neighbours for each DEM cell.".to_string(),
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults: ToolArgs::new(),
			examples: vec![ToolExample {
				name: "count_inflowing".to_string(),
				description: "Count inflowing D8 neighbours on a conditioned DEM".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "diagnostics".to_string(), "dem".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let dirs = d8_dir_from_dem_local(&dem);
		let mut data = num_inflowing_from_d8(&dirs, dem.rows, dem.cols, dem.nodata, &dem);
		for r in 0..dem.rows {
			for c in 0..dem.cols {
				let i = idx(r, c, dem.cols);
				if dem.get(0, r as isize, c as isize) == dem.nodata {
					data[i] = -32768.0;
				}
			}
		}
		let mut out = vec_to_raster(&dem, &data, DataType::I16);
		out.nodata = -32768.0;
		Ok(build_result(write_or_store_output(out, output_path)?))
	}
}

impl Tool for FindParallelFlowTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "find_parallel_flow",
			display_name: "Find Parallel Flow",
			summary: "Identifies stream cells that possess parallel D8 flow directions.",
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "d8_pntr", description: "Input D8 pointer raster", required: true },
				ToolParamSpec { name: "streams", description: "Optional stream raster mask", required: false },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		ToolManifest {
			id: "find_parallel_flow".to_string(),
			display_name: "Find Parallel Flow".to_string(),
			summary: "Identifies stream cells that possess parallel D8 flow directions.".to_string(),
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults: ToolArgs::new(),
			examples: vec![ToolExample {
				name: "parallel_flow".to_string(),
				description: "Detect stream segments with parallel local flow directions".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "diagnostics".to_string(), "d8".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "d8_pntr")
			.or_else(|_| parse_raster_path_arg(args, "d8_pointer"))
			.or_else(|_| parse_raster_path_arg(args, "input"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (pntr, output_path) = parse_pointer_input(args)?;
		let streams = args
			.get("streams")
			.and_then(|v| v.as_str())
			.map(load_raster)
			.transpose()?;
		if let Some(ref streams) = streams {
			if streams.rows != pntr.rows || streams.cols != pntr.cols {
				return Err(ToolError::Validation(
					"streams raster must match pointer raster dimensions".to_string(),
				));
			}
		}

		let rows = pntr.rows;
		let cols = pntr.cols;
		let nodata = pntr.nodata;
		let inflowing_vals = [16.0, 32.0, 64.0, 128.0, 1.0, 2.0, 4.0, 8.0];
		let outflowing_vals = [1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0];
		let mut data = vec![nodata; rows * cols];

		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				let z = pntr.get(0, r as isize, c as isize);
				if z == nodata {
					continue;
				}
				if let Some(ref s) = streams {
					let sv = s.get(0, r as isize, c as isize);
					if sv == s.nodata || sv <= 0.0 {
						continue;
					}
				}

				let mut is_parallel = false;
				for n in 0..8 {
					if (z - outflowing_vals[n]).abs() < f64::EPSILON {
						continue;
					}
					let rn = r as isize + DY[n];
					let cn = c as isize + DX[n];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let zn = pntr.get(0, rn, cn);
					if zn == nodata || (zn - z).abs() >= f64::EPSILON || (zn - inflowing_vals[n]).abs() < f64::EPSILON {
						continue;
					}
					if let Some(ref s) = streams {
						let svn = s.get(0, rn, cn);
						if svn == s.nodata || svn <= 0.0 {
							continue;
						}
					}
					is_parallel = true;
					break;
				}
				data[i] = if is_parallel { 1.0 } else { 0.0 };
			}
		}

		let out = vec_to_raster(&pntr, &data, DataType::I16);
		Ok(build_result(write_or_store_output(out, output_path)?))
	}
}

impl Tool for EdgeContaminationTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "edge_contamination",
			display_name: "Edge Contamination",
			summary: "Identifies DEM cells that are edge-contaminated for D8, MFD/FD8, or D-Infinity flow routing.",
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
				ToolParamSpec { name: "flow_type", description: "Flow algorithm: d8, mfd/fd8, or dinf", required: false },
				ToolParamSpec { name: "z_factor", description: "Optional vertical scaling factor", required: false },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		let mut defaults = ToolArgs::new();
		defaults.insert("flow_type".to_string(), json!("mfd"));
		defaults.insert("z_factor".to_string(), json!(-1.0));
		ToolManifest {
			id: "edge_contamination".to_string(),
			display_name: "Edge Contamination".to_string(),
			summary: "Identifies DEM cells whose upslope area extends beyond the DEM edge for common flow-routing schemes."
				.to_string(),
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "edge_contamination_mfd".to_string(),
				description: "Map edge-contaminated cells using MFD-style routing".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "diagnostics".to_string(), "dem".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let flow_type = args
			.get("flow_type")
			.and_then(|v| v.as_str())
			.unwrap_or("mfd")
			.to_lowercase();
		let use_dinf = flow_type.contains("dinf") || flow_type.contains("d-inf");
		let use_d8 = !use_dinf && flow_type.contains("d8") && !flow_type.contains("fd8") && !flow_type.contains("mfd");
		let use_mfd = !use_dinf && !use_d8;

		let rows = dem.rows;
		let cols = dem.cols;
		let nodata = dem.nodata;
		let cell_x = dem.cell_size_x;
		let cell_y = dem.cell_size_y;
		let diag = (cell_x * cell_x + cell_y * cell_y).sqrt();
		let lengths = [diag, cell_x, diag, cell_y, diag, cell_x, diag, cell_y];
		let grid_res = (cell_x + cell_y) / 2.0;
		let e1_col = [1, 0, 0, -1, -1, 0, 0, 1];
		let e1_row = [0, -1, -1, 0, 0, 1, 1, 0];
		let e2_col = [1, 1, -1, -1, -1, -1, 1, 1];
		let e2_row = [-1, -1, -1, -1, 1, 1, 1, 1];
		let atan_of_1 = 1.0_f64.atan();
		let mut z_factor = args.get("z_factor").and_then(|v| v.as_f64()).unwrap_or(-1.0);
		if !z_factor.is_finite() || z_factor <= 0.0 {
			z_factor = 1.0;
		}

		let mut visited = vec![0u8; rows * cols];
		let mut stack = Vec::<usize>::new();
		let mut edge_stack = Vec::<usize>::new();
		let mut out = vec![0.0; rows * cols];

		let mut seed_boundary = |r: usize, c: usize, stack: &mut Vec<usize>, edge_stack: &mut Vec<usize>| {
			let i = idx(r, c, cols);
			if visited[i] != 0 {
				return;
			}
			visited[i] = 2;
			let z = dem.get(0, r as isize, c as isize);
			if z == nodata {
				stack.push(i);
			} else {
				edge_stack.push(i);
			}
		};

		for r in 0..rows {
			seed_boundary(r, 0, &mut stack, &mut edge_stack);
			if cols > 1 {
				seed_boundary(r, cols - 1, &mut stack, &mut edge_stack);
			}
		}
		for c in 0..cols {
			seed_boundary(0, c, &mut stack, &mut edge_stack);
			if rows > 1 {
				seed_boundary(rows - 1, c, &mut stack, &mut edge_stack);
			}
		}

		while let Some(i) = stack.pop() {
			let r = i / cols;
			let c = i % cols;
			for n in 0..8 {
				let rn = r as isize + DY[n];
				let cn = c as isize + DX[n];
				if !in_bounds(rn, cn, rows, cols) {
					continue;
				}
				let ni = idx(rn as usize, cn as usize, cols);
				if visited[ni] != 0 {
					continue;
				}
				visited[ni] = 2;
				let zn = dem.get(0, rn, cn);
				if zn == nodata {
					stack.push(ni);
				} else {
					edge_stack.push(ni);
				}
			}
		}

		while let Some(i) = edge_stack.pop() {
			out[i] = 1.0;
			let r = i / cols;
			let c = i % cols;
			let z0 = dem.get(0, r as isize, c as isize);
			if z0 == nodata {
				continue;
			}
			let z = z0 * z_factor;

			if use_d8 {
				let mut best_dir = -1i8;
				let mut best_slope = f64::MIN;
				for n in 0..8 {
					let rn = r as isize + DY[n];
					let cn = c as isize + DX[n];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let zn = dem.get(0, rn, cn);
					if zn == nodata {
						continue;
					}
					let slope = (z - zn * z_factor) / lengths[n];
					if slope > best_slope && slope > 0.0 {
						best_slope = slope;
						best_dir = n as i8;
					}
				}
				if best_dir >= 0 {
					let rn = r as isize + DY[best_dir as usize];
					let cn = c as isize + DX[best_dir as usize];
					let ni = idx(rn as usize, cn as usize, cols);
					if visited[ni] == 0 {
						visited[ni] = 2;
						edge_stack.push(ni);
					}
				}
				continue;
			}

			if use_dinf {
				let mut best_slope = f64::MIN;
				let mut best_a: Option<(isize, isize)> = None;
				let mut best_b: Option<(isize, isize)> = None;
				for n in 0..8 {
					let r1 = r as isize + e1_row[n];
					let c1 = c as isize + e1_col[n];
					let r2 = r as isize + e2_row[n];
					let c2 = c as isize + e2_col[n];
					if !in_bounds(r1, c1, rows, cols) || !in_bounds(r2, c2, rows, cols) {
						continue;
					}
					let e1 = dem.get(0, r1, c1);
					let e2 = dem.get(0, r2, c2);
					if e1 == nodata || e2 == nodata {
						continue;
					}
					let e1 = e1 * z_factor;
					let e2 = e2 * z_factor;
					let (mut s, r_ang);
					let mut a: Option<(isize, isize)> = None;
					let mut b: Option<(isize, isize)> = None;
					if z > e1 && z > e2 {
						let s1 = (z - e1) / grid_res;
						let s2 = (e1 - e2) / grid_res;
						r_ang = if s1 != 0.0 { (s2 / s1).atan() } else { std::f64::consts::PI / 2.0 };
						s = (s1 * s1 + s2 * s2).sqrt();
						if (s1 < 0.0 && s2 <= 0.0) || (s1 == 0.0 && s2 < 0.0) {
							s *= -1.0;
						}
						if r_ang < 0.0 {
							s = s1;
						} else if r_ang > atan_of_1 {
							s = (z - e2) / diag;
						}
						a = Some((r1, c1));
						b = Some((r2, c2));
					} else if z > e1 || z > e2 {
						if z > e1 {
							s = (z - e1) / grid_res;
							a = Some((r1, c1));
						} else {
							s = (z - e2) / diag;
							b = Some((r2, c2));
						}
					} else {
						continue;
					}
					if s >= best_slope {
						best_slope = s;
						best_a = a;
						best_b = b;
					}
				}

				if best_slope > 0.0 {
					if let Some((rn, cn)) = best_a {
						let ni = idx(rn as usize, cn as usize, cols);
						if visited[ni] == 0 {
							visited[ni] = 2;
							edge_stack.push(ni);
						}
					}
					if let Some((rn, cn)) = best_b {
						let ni = idx(rn as usize, cn as usize, cols);
						if visited[ni] == 0 {
							visited[ni] = 2;
							edge_stack.push(ni);
						}
					}
				}
				continue;
			}

			if use_mfd {
				for n in 0..8 {
					let rn = r as isize + DY[n];
					let cn = c as isize + DX[n];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let zn = dem.get(0, rn, cn);
					if zn == nodata || zn >= z0 {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					if visited[ni] == 0 {
						visited[ni] = 2;
						edge_stack.push(ni);
					}
				}
			}
		}

		let mut raster = vec_to_raster(&dem, &out, DataType::U8);
		raster.nodata = 0.0;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

impl Tool for D8MassFluxTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "d8_mass_flux",
			display_name: "D8 Mass Flux",
			summary: "Routes mass downslope using D8 flow directions derived from a DEM.",
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
				ToolParamSpec { name: "loading", description: "Loading raster", required: true },
				ToolParamSpec { name: "efficiency", description: "Efficiency raster (0-1 or percent)", required: true },
				ToolParamSpec { name: "absorption", description: "Absorption raster", required: true },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		ToolManifest {
			id: "d8_mass_flux".to_string(),
			display_name: "D8 Mass Flux".to_string(),
			summary: "Performs a D8-based mass-flux accumulation using loading, efficiency, and absorption rasters."
				.to_string(),
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults: ToolArgs::new(),
			examples: vec![ToolExample {
				name: "mass_flux".to_string(),
				description: "Route sediment or nutrient loading downslope with D8 flow routing".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "mass_flux".to_string(), "d8".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))?;
		parse_raster_path_arg(args, "loading")?;
		parse_raster_path_arg(args, "efficiency")?;
		parse_raster_path_arg(args, "absorption")?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let loading_path = parse_raster_path_arg(args, "loading")?;
		let efficiency_path = parse_raster_path_arg(args, "efficiency")?;
		let absorption_path = parse_raster_path_arg(args, "absorption")?;
		let loading = load_raster(&loading_path)?;
		let efficiency = load_raster(&efficiency_path)?;
		let absorption = load_raster(&absorption_path)?;

		let rows = dem.rows;
		let cols = dem.cols;
		if loading.rows != rows
			|| loading.cols != cols
			|| efficiency.rows != rows
			|| efficiency.cols != cols
			|| absorption.rows != rows
			|| absorption.cols != cols
		{
			return Err(ToolError::Validation(
				"dem, loading, efficiency, and absorption rasters must share the same dimensions".to_string(),
			));
		}

		let mut efficiency_max = f64::NEG_INFINITY;
		for r in 0..rows {
			for c in 0..cols {
				let v = efficiency.get(0, r as isize, c as isize);
				if v != efficiency.nodata && v.is_finite() && v > efficiency_max {
					efficiency_max = v;
				}
			}
		}
		let efficiency_multiplier = if efficiency_max > 1.0 { 0.01 } else { 1.0 };

		let dirs = d8_dir_from_dem_local(&dem);
		let mut inflow = vec![-1i32; rows * cols];
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				if dem.get(0, r as isize, c as isize) == dem.nodata {
					continue;
				}
				let mut count = 0i32;
				for k in 0..8 {
					let rn = r as isize + DY[k];
					let cn = c as isize + DX[k];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					if dirs[ni] == [4, 5, 6, 7, 0, 1, 2, 3][k] {
						count += 1;
					}
				}
				inflow[i] = count;
			}
		}

		let mut mass = vec![dem.nodata; rows * cols];
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				if dem.get(0, r as isize, c as isize) == dem.nodata {
					continue;
				}
				let lv = loading.get(0, r as isize, c as isize);
				let ev = efficiency.get(0, r as isize, c as isize);
				let av = absorption.get(0, r as isize, c as isize);
				if lv == loading.nodata || ev == efficiency.nodata || av == absorption.nodata {
					inflow[i] = -1;
					mass[i] = dem.nodata;
					continue;
				}
				mass[i] = lv;
			}
		}

		let mut stack = Vec::<usize>::new();
		for i in 0..(rows * cols) {
			if inflow[i] == 0 {
				stack.push(i);
			}
		}

		while let Some(i) = stack.pop() {
			if inflow[i] < 0 || mass[i] == dem.nodata {
				continue;
			}
			let r = i / cols;
			let c = i % cols;
			let ev = efficiency.get(0, r as isize, c as isize) * efficiency_multiplier;
			let av = absorption.get(0, r as isize, c as isize);
			let routed = (mass[i] - av) * ev;
			let dir = dirs[i];
			if dir >= 0 {
				let rn = r as isize + DY[dir as usize];
				let cn = c as isize + DX[dir as usize];
				if in_bounds(rn, cn, rows, cols) {
					let ni = idx(rn as usize, cn as usize, cols);
					if inflow[ni] >= 0 && mass[ni] != dem.nodata {
						mass[ni] += routed;
						inflow[ni] -= 1;
						if inflow[ni] == 0 {
							stack.push(ni);
						}
					}
				}
			}
		}

		let mut out = vec_to_raster(&dem, &mass, DataType::F32);
		out.nodata = dem.nodata;
		Ok(build_result(write_or_store_output(out, output_path)?))
	}
}

impl Tool for DInfMassFluxTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "dinf_mass_flux",
			display_name: "DInf Mass Flux",
			summary: "Routes mass downslope using D-Infinity flow directions derived from a DEM.",
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
				ToolParamSpec { name: "loading", description: "Loading raster", required: true },
				ToolParamSpec { name: "efficiency", description: "Efficiency raster (0-1 or percent)", required: true },
				ToolParamSpec { name: "absorption", description: "Absorption raster", required: true },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		ToolManifest {
			id: "dinf_mass_flux".to_string(),
			display_name: "DInf Mass Flux".to_string(),
			summary: "Performs a D-Infinity mass-flux accumulation using loading, efficiency, and absorption rasters."
				.to_string(),
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults: ToolArgs::new(),
			examples: vec![ToolExample {
				name: "dinf_mass_flux".to_string(),
				description: "Route sediment or nutrient loading downslope with D-Infinity flow routing".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "mass_flux".to_string(), "dinf".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))?;
		parse_raster_path_arg(args, "loading")?;
		parse_raster_path_arg(args, "efficiency")?;
		parse_raster_path_arg(args, "absorption")?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let loading_path = parse_raster_path_arg(args, "loading")?;
		let efficiency_path = parse_raster_path_arg(args, "efficiency")?;
		let absorption_path = parse_raster_path_arg(args, "absorption")?;
		let loading = load_raster(&loading_path)?;
		let efficiency = load_raster(&efficiency_path)?;
		let absorption = load_raster(&absorption_path)?;

		let rows = dem.rows;
		let cols = dem.cols;
		if loading.rows != rows
			|| loading.cols != cols
			|| efficiency.rows != rows
			|| efficiency.cols != cols
			|| absorption.rows != rows
			|| absorption.cols != cols
		{
			return Err(ToolError::Validation(
				"dem, loading, efficiency, and absorption rasters must share the same dimensions".to_string(),
			));
		}

		let mut efficiency_max = f64::NEG_INFINITY;
		for r in 0..rows {
			for c in 0..cols {
				let v = efficiency.get(0, r as isize, c as isize);
				if v != efficiency.nodata && v.is_finite() && v > efficiency_max {
					efficiency_max = v;
				}
			}
		}
		let efficiency_multiplier = if efficiency_max > 1.0 { 0.01 } else { 1.0 };

		let flow_dir = dinf_angle_from_dem(&dem);
		let start_fd = [180.0f64, 225.0, 270.0, 315.0, 0.0, 45.0, 90.0, 135.0];
		let end_fd = [270.0f64, 315.0, 360.0, 45.0, 90.0, 135.0, 180.0, 225.0];

		let mut inflow = vec![-1i32; rows * cols];
		let mut mass = vec![dem.nodata; rows * cols];
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				if dem.get(0, r as isize, c as isize) == dem.nodata {
					continue;
				}

				let lv = loading.get(0, r as isize, c as isize);
				let ev = efficiency.get(0, r as isize, c as isize);
				let av = absorption.get(0, r as isize, c as isize);
				if lv == loading.nodata || ev == efficiency.nodata || av == absorption.nodata {
					inflow[i] = -1;
					mass[i] = dem.nodata;
					continue;
				}
				mass[i] = lv;

				let mut count = 0i32;
				for n in 0..8 {
					let rn = r as isize + DY[n];
					let cn = c as isize + DX[n];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					let dir = flow_dir[ni];
					if dir < 0.0 {
						continue;
					}
					let receives = if n != 3 {
						dir > start_fd[n] && dir < end_fd[n]
					} else {
						dir > start_fd[n] || dir < end_fd[n]
					};
					if receives {
						count += 1;
					}
				}
				inflow[i] = count;
			}
		}

		let mut stack = Vec::<usize>::new();
		for i in 0..(rows * cols) {
			if inflow[i] == 0 {
				stack.push(i);
			}
		}

		while let Some(i) = stack.pop() {
			if inflow[i] < 0 || mass[i] == dem.nodata {
				continue;
			}
			let r = i / cols;
			let c = i % cols;
			let eff = efficiency.get(0, r as isize, c as isize) * efficiency_multiplier;
			let absorp = absorption.get(0, r as isize, c as isize);
			let routed = (mass[i] - absorp) * eff;

			let dir = flow_dir[i];
			if dir < 0.0 {
				continue;
			}

			let (p1, r1, c1, p2, r2, c2) = if dir < 45.0 {
				((45.0 - dir) / 45.0, r as isize - 1, c as isize, dir / 45.0, r as isize - 1, c as isize + 1)
			} else if dir < 90.0 {
				((90.0 - dir) / 45.0, r as isize - 1, c as isize + 1, (dir - 45.0) / 45.0, r as isize, c as isize + 1)
			} else if dir < 135.0 {
				((135.0 - dir) / 45.0, r as isize, c as isize + 1, (dir - 90.0) / 45.0, r as isize + 1, c as isize + 1)
			} else if dir < 180.0 {
				((180.0 - dir) / 45.0, r as isize + 1, c as isize + 1, (dir - 135.0) / 45.0, r as isize + 1, c as isize)
			} else if dir < 225.0 {
				((225.0 - dir) / 45.0, r as isize + 1, c as isize, (dir - 180.0) / 45.0, r as isize + 1, c as isize - 1)
			} else if dir < 270.0 {
				((270.0 - dir) / 45.0, r as isize + 1, c as isize - 1, (dir - 225.0) / 45.0, r as isize, c as isize - 1)
			} else if dir < 315.0 {
				((315.0 - dir) / 45.0, r as isize, c as isize - 1, (dir - 270.0) / 45.0, r as isize - 1, c as isize - 1)
			} else {
				((360.0 - dir) / 45.0, r as isize - 1, c as isize - 1, (dir - 315.0) / 45.0, r as isize - 1, c as isize)
			};

			if p1 > 0.0 && in_bounds(r1, c1, rows, cols) {
				let ni = idx(r1 as usize, c1 as usize, cols);
				if inflow[ni] >= 0 && mass[ni] != dem.nodata {
					mass[ni] += routed * p1;
					inflow[ni] -= 1;
					if inflow[ni] == 0 {
						stack.push(ni);
					}
				}
			}
			if p2 > 0.0 && in_bounds(r2, c2, rows, cols) {
				let ni = idx(r2 as usize, c2 as usize, cols);
				if inflow[ni] >= 0 && mass[ni] != dem.nodata {
					mass[ni] += routed * p2;
					inflow[ni] -= 1;
					if inflow[ni] == 0 {
						stack.push(ni);
					}
				}
			}
		}

		let mut out = vec_to_raster(&dem, &mass, DataType::F32);
		out.nodata = dem.nodata;
		Ok(build_result(write_or_store_output(out, output_path)?))
	}
}

impl Tool for FlowLengthDiffTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "flow_length_diff",
			display_name: "Flow Length Diff",
			summary: "Calculates the local maximum absolute difference in downslope flowpath length.",
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "d8_pntr", description: "Input D8 pointer raster", required: true },
				ToolParamSpec { name: "esri_pntr", description: "Use ESRI pointer encoding", required: false },
				ToolParamSpec { name: "log_transform", description: "Apply natural-log transform to output", required: false },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		let mut defaults = ToolArgs::new();
		defaults.insert("esri_pntr".to_string(), json!(false));
		defaults.insert("log_transform".to_string(), json!(false));
		ToolManifest {
			id: "flow_length_diff".to_string(),
			display_name: "Flow Length Diff".to_string(),
			summary: "Computes local maximum absolute differences in downslope path length from a D8 pointer raster."
				.to_string(),
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "flow_length_difference".to_string(),
				description: "Map local downslope flowpath-length contrasts".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "d8".to_string(), "flowpath".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "d8_pntr")
			.or_else(|_| parse_raster_path_arg(args, "d8_pointer"))
			.or_else(|_| parse_raster_path_arg(args, "input"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (pntr, output_path) = parse_pointer_input(args)?;
		let esri_style = args
			.get("esri_pntr")
			.or_else(|| args.get("esri_pointer"))
			.and_then(|v| v.as_bool())
			.unwrap_or(false);
		let log_transform = args
			.get("log_transform")
			.or_else(|| args.get("log"))
			.and_then(|v| v.as_bool())
			.unwrap_or(false);

		let rows = pntr.rows;
		let cols = pntr.cols;
		let nodata = pntr.nodata;
		let out_nodata = -32768.0;
		let unknown = -999.0;
		let cell_x = pntr.cell_size_x;
		let cell_y = pntr.cell_size_y;
		let diag = (cell_x * cell_x + cell_y * cell_y).sqrt();
		let lengths = [diag, cell_x, diag, cell_y, diag, cell_x, diag, cell_y];

		let mut flow_dir = vec![-1i8; rows * cols];
		let mut dfl = vec![out_nodata; rows * cols];
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				let v = pntr.get(0, r as isize, c as isize);
				if v == nodata || v < 0.0 {
					dfl[i] = out_nodata;
					flow_dir[i] = -1;
					continue;
				}
				let dir = decode_d8_pointer_dir(v, esri_style);
				if dir < 0 {
					return Err(ToolError::Validation(
						"pointer raster contains unexpected values; expected D8/Rho8 pointer encoding".to_string(),
					));
				}
				flow_dir[i] = dir;
				dfl[i] = unknown;
			}
		}

		let mut path = Vec::<usize>::new();
		let mut steps = Vec::<f64>::new();
		for r in 0..rows {
			for c in 0..cols {
				let start = idx(r, c, cols);
				if dfl[start] != unknown {
					continue;
				}
				path.clear();
				steps.clear();
				let mut cur = start;
				let base: f64;
				loop {
					if dfl[cur] != unknown {
						base = dfl[cur];
						break;
					}
					path.push(cur);
					let dir = flow_dir[cur];
					if dir < 0 {
						base = 0.0;
						break;
					}
					let rr = cur / cols;
					let cc = cur % cols;
					let rn = rr as isize + DY[dir as usize];
					let cn = cc as isize + DX[dir as usize];
					if !in_bounds(rn, cn, rows, cols) {
						base = 0.0;
						break;
					}
					steps.push(lengths[dir as usize]);
					cur = idx(rn as usize, cn as usize, cols);
				}

				let mut dist = base;
				for p in (0..path.len()).rev() {
					if p < steps.len() {
						dist += steps[p];
					}
					dfl[path[p]] = dist;
				}
			}
		}

		let mut out = vec![out_nodata; rows * cols];
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				let z = dfl[i];
				if z == out_nodata {
					continue;
				}
				let mut max_abs = f64::NEG_INFINITY;
				for n in [1usize, 3, 5, 7] {
					let rn = r as isize + DY[n];
					let cn = c as isize + DX[n];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let zn = dfl[idx(rn as usize, cn as usize, cols)];
					if zn == out_nodata {
						continue;
					}
					let d = (z - zn).abs();
					if d > max_abs {
						max_abs = d;
					}
				}
				if max_abs.is_finite() {
					out[i] = if log_transform { max_abs.ln() } else { max_abs };
				}
			}
		}

		let mut raster = vec_to_raster(&pntr, &out, DataType::F32);
		raster.nodata = out_nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

impl Tool for DownslopeFlowpathLengthTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "downslope_flowpath_length",
			display_name: "Downslope Flowpath Length",
			summary: "Computes downslope flowpath length from each cell to an outlet in a D8 pointer raster.",
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "d8_pntr", description: "Input D8 pointer raster", required: true },
				ToolParamSpec { name: "watersheds", description: "Optional watershed raster", required: false },
				ToolParamSpec { name: "weights", description: "Optional per-cell distance weighting raster", required: false },
				ToolParamSpec { name: "esri_pntr", description: "Use ESRI pointer encoding", required: false },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		let mut defaults = ToolArgs::new();
		defaults.insert("esri_pntr".to_string(), json!(false));
		ToolManifest {
			id: "downslope_flowpath_length".to_string(),
			display_name: "Downslope Flowpath Length".to_string(),
			summary: "Computes downslope flowpath length from each cell to an outlet in a D8 pointer raster."
				.to_string(),
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "downslope_length".to_string(),
				description: "Compute downslope flowpath length from a D8 pointer raster".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "d8".to_string(), "flowpath".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "d8_pntr")
			.or_else(|_| parse_raster_path_arg(args, "d8_pointer"))
			.or_else(|_| parse_raster_path_arg(args, "input"))?;
		if args.get("watersheds").is_some() {
			parse_raster_path_arg(args, "watersheds")?;
		}
		if args.get("weights").is_some() {
			parse_raster_path_arg(args, "weights")?;
		}
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (pntr, output_path) = parse_pointer_input(args)?;
		let esri_style = args
			.get("esri_pntr")
			.or_else(|| args.get("esri_pointer"))
			.and_then(|v| v.as_bool())
			.unwrap_or(false);

		let rows = pntr.rows;
		let cols = pntr.cols;
		let out_nodata = -32768.0;
		let cell_x = pntr.cell_size_x;
		let cell_y = pntr.cell_size_y;
		let diag = (cell_x * cell_x + cell_y * cell_y).sqrt();
		let lengths = [diag, cell_x, diag, cell_y, diag, cell_x, diag, cell_y];

		let watersheds = if args.get("watersheds").is_some() {
			let path = parse_raster_path_arg(args, "watersheds")?;
			let ws = load_raster(&path)?;
			if ws.rows != rows || ws.cols != cols {
				return Err(ToolError::Validation(
					"watersheds raster must match pointer raster dimensions".to_string(),
				));
			}
			Some(ws)
		} else {
			None
		};

		let weights = if args.get("weights").is_some() {
			let path = parse_raster_path_arg(args, "weights")?;
			let w = load_raster(&path)?;
			if w.rows != rows || w.cols != cols {
				return Err(ToolError::Validation(
					"weights raster must match pointer raster dimensions".to_string(),
				));
			}
			Some(w)
		} else {
			None
		};

		let mut flow_dir = vec![-1i8; rows * cols];
		let mut ws_id = vec![1.0; rows * cols];
		let mut wgt = vec![1.0; rows * cols];
		let mut out = vec![-999.0; rows * cols];

		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				let z = pntr.get(0, r as isize, c as isize);
				if z == pntr.nodata {
					out[i] = out_nodata;
					continue;
				}
				flow_dir[i] = decode_d8_pointer_dir_checked(z, esri_style)?;
				if let Some(ws) = &watersheds {
					let wz = ws.get(0, r as isize, c as isize);
					if wz == ws.nodata {
						out[i] = out_nodata;
						continue;
					}
					ws_id[i] = wz;
				}
				if let Some(w) = &weights {
					let wz = w.get(0, r as isize, c as isize);
					if wz == w.nodata {
						out[i] = out_nodata;
						continue;
					}
					wgt[i] = wz;
				}
			}
		}

		for r in 0..rows {
			for c in 0..cols {
				let start = idx(r, c, cols);
				if out[start] == out_nodata || out[start] != -999.0 || ws_id[start] <= 0.0 {
					continue;
				}

				let mut dist = 0.0;
				let mut path = Vec::<usize>::new();
				let mut steps = Vec::<f64>::new();
				let mut cur = start;
				let current_id = ws_id[start];

				loop {
					path.push(cur);
					let dir = flow_dir[cur];
					if dir < 0 {
						break;
					}
					let rr = cur / cols;
					let cc = cur % cols;
					let rn = rr as isize + DY[dir as usize];
					let cn = cc as isize + DX[dir as usize];
					if !in_bounds(rn, cn, rows, cols) {
						break;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					if out[ni] == out_nodata || ws_id[ni] != current_id {
						break;
					}
					let step = lengths[dir as usize] * wgt[ni];
					steps.push(step);
					if out[ni] != -999.0 {
						dist += step + out[ni];
						break;
					}
					dist += step;
					cur = ni;
				}

				for p in 0..path.len() {
					out[path[p]] = dist;
					if p < steps.len() {
						dist -= steps[p];
					}
				}
			}
		}

		for i in 0..out.len() {
			if out[i] == -999.0 {
				out[i] = 0.0;
			}
		}

		let mut raster = vec_to_raster(&pntr, &out, DataType::F32);
		raster.nodata = out_nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

impl Tool for MaxUpslopeFlowpathLengthTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "max_upslope_flowpath_length",
			display_name: "Max Upslope Flowpath Length",
			summary: "Computes the maximum upslope flowpath length passing through each DEM cell.",
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
			id: "max_upslope_flowpath_length".to_string(),
			display_name: "Max Upslope Flowpath Length".to_string(),
			summary: "Computes the maximum upslope flowpath length passing through each DEM cell.".to_string(),
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults: ToolArgs::new(),
			examples: vec![ToolExample {
				name: "max_upslope_length".to_string(),
				description: "Compute maximum flowpath length from all upslope sources".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "flowpath".to_string(), "dem".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let rows = dem.rows;
		let cols = dem.cols;
		let nodata = dem.nodata;
		let out_nodata = -32768.0;

		let dirs = d8_dir_from_dem_local(&dem);
		let mut inflow = vec![-1i32; rows * cols];
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				if dem.get(0, r as isize, c as isize) == nodata {
					continue;
				}
				let mut count = 0i32;
				for k in 0..8 {
					let rn = r as isize + DY[k];
					let cn = c as isize + DX[k];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					if dirs[ni] == [4, 5, 6, 7, 0, 1, 2, 3][k] {
						count += 1;
					}
				}
				inflow[i] = count;
			}
		}

		let cell_x = dem.cell_size_x;
		let cell_y = dem.cell_size_y;
		let diag = (cell_x * cell_x + cell_y * cell_y).sqrt();
		let lengths = [diag, cell_x, diag, cell_y, diag, cell_x, diag, cell_y];

		let mut out = vec![out_nodata; rows * cols];
		let mut stack = Vec::<usize>::with_capacity(rows * cols);
		for i in 0..rows * cols {
			if inflow[i] == 0 {
				stack.push(i);
				out[i] = 0.0;
			}
		}

		while let Some(i) = stack.pop() {
			let dir = dirs[i];
			if dir >= 0 {
				let r = i / cols;
				let c = i % cols;
				let rn = r as isize + DY[dir as usize];
				let cn = c as isize + DX[dir as usize];
				if in_bounds(rn, cn, rows, cols) {
					let ni = idx(rn as usize, cn as usize, cols);
					let candidate = out[i] + lengths[dir as usize];
					if out[ni] == out_nodata || candidate > out[ni] {
						out[ni] = candidate;
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

		let mut raster = vec_to_raster(&dem, &out, DataType::F32);
		raster.nodata = out_nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

impl Tool for AverageUpslopeFlowpathLengthTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "average_upslope_flowpath_length",
			display_name: "Average Upslope Flowpath Length",
			summary: "Computes the average upslope flowpath length passing through each DEM cell.",
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
			id: "average_upslope_flowpath_length".to_string(),
			display_name: "Average Upslope Flowpath Length".to_string(),
			summary: "Computes the average upslope flowpath length passing through each DEM cell.".to_string(),
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults: ToolArgs::new(),
			examples: vec![ToolExample {
				name: "average_upslope_length".to_string(),
				description: "Compute average flowpath length from all upslope sources".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "flowpath".to_string(), "dem".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let rows = dem.rows;
		let cols = dem.cols;
		let nodata = dem.nodata;
		let out_nodata = -32768.0;

		let dirs = d8_dir_from_dem_local(&dem);
		let mut inflow = vec![-1i32; rows * cols];
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				if dem.get(0, r as isize, c as isize) == nodata {
					continue;
				}
				let mut count = 0i32;
				for k in 0..8 {
					let rn = r as isize + DY[k];
					let cn = c as isize + DX[k];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					if dirs[ni] == [4, 5, 6, 7, 0, 1, 2, 3][k] {
						count += 1;
					}
				}
				inflow[i] = count;
			}
		}

		let cell_x = dem.cell_size_x;
		let cell_y = dem.cell_size_y;
		let diag = (cell_x * cell_x + cell_y * cell_y).sqrt();
		let lengths = [diag, cell_x, diag, cell_y, diag, cell_x, diag, cell_y];

		let mut path_count = vec![0i64; rows * cols];
		let mut total_len = vec![0.0f64; rows * cols];
		let mut out = vec![out_nodata; rows * cols];
		let mut stack = Vec::<usize>::with_capacity(rows * cols);
		for i in 0..rows * cols {
			if inflow[i] == 0 {
				stack.push(i);
				path_count[i] = 1;
				total_len[i] = 0.0;
			}
		}

		while let Some(i) = stack.pop() {
			let dir = dirs[i];
			if dir >= 0 {
				let r = i / cols;
				let c = i % cols;
				let rn = r as isize + DY[dir as usize];
				let cn = c as isize + DX[dir as usize];
				if in_bounds(rn, cn, rows, cols) {
					let ni = idx(rn as usize, cn as usize, cols);
					total_len[ni] += total_len[i] + (path_count[i] as f64) * lengths[dir as usize];
					path_count[ni] += path_count[i];
					if inflow[ni] > 0 {
						inflow[ni] -= 1;
						if inflow[ni] == 0 {
							stack.push(ni);
						}
					}
				}
			}
		}

		for i in 0..rows * cols {
			if path_count[i] > 0 {
				out[i] = total_len[i] / path_count[i] as f64;
			}
		}

		let mut raster = vec_to_raster(&dem, &out, DataType::F32);
		raster.nodata = out_nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

#[derive(Clone, Copy)]
struct StreamDistNode {
	dist: f64,
	i: usize,
	stream_elev: f64,
}

impl PartialEq for StreamDistNode {
	fn eq(&self, other: &Self) -> bool {
		self.i == other.i && self.dist.to_bits() == other.dist.to_bits()
	}
}
impl Eq for StreamDistNode {}
impl PartialOrd for StreamDistNode {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		other.dist.partial_cmp(&self.dist)
	}
}
impl Ord for StreamDistNode {
	fn cmp(&self, other: &Self) -> Ordering {
		self.partial_cmp(other).unwrap_or(Ordering::Equal)
	}
}

fn dinf_angle_from_dem(dem: &Raster) -> Vec<f64> {
	let rows = dem.rows;
	let cols = dem.cols;
	let nodata = dem.nodata;
	let grid_res = (dem.cell_size_x + dem.cell_size_y) / 2.0;
	let diag = (dem.cell_size_x * dem.cell_size_x + dem.cell_size_y * dem.cell_size_y).sqrt();
	let mut out = vec![-1.0; rows * cols];

	let ac_vals = [0.0f64, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 4.0];
	let af_vals = [1.0f64, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
	let e1_col = [1isize, 0, 0, -1, -1, 0, 0, 1];
	let e1_row = [0isize, -1, -1, 0, 0, 1, 1, 0];
	let e2_col = [1isize, 1, -1, -1, -1, -1, 1, 1];
	let e2_row = [-1isize, -1, -1, -1, 1, 1, 1, 1];
	let atan_of_1 = 1.0f64.atan();
	let half_pi = std::f64::consts::PI / 2.0;

	for r in 0..rows {
		for c in 0..cols {
			let i = idx(r, c, cols);
			let e0 = dem.get(0, r as isize, c as isize);
			if e0 == nodata {
				continue;
			}

			let mut max_slope = f64::MIN;
			let mut dir = 360.0;

			for n in 0..8 {
				let ac = ac_vals[n];
				let af = af_vals[n];
				let r1 = r as isize + e1_row[n];
				let c1 = c as isize + e1_col[n];
				let r2 = r as isize + e2_row[n];
				let c2 = c as isize + e2_col[n];
				if !in_bounds(r1, c1, rows, cols) || !in_bounds(r2, c2, rows, cols) {
					continue;
				}
				let e1 = dem.get(0, r1, c1);
				let e2 = dem.get(0, r2, c2);
				if e1 == nodata || e2 == nodata {
					continue;
				}

				let mut s = f64::MIN;
				let mut rr = 0.0;
				if e0 > e1 && e0 > e2 {
					let s1 = (e0 - e1) / grid_res;
					let s2 = (e1 - e2) / grid_res;
					rr = if s1 != 0.0 { (s2 / s1).atan() } else { std::f64::consts::PI / 2.0 };
					s = (s1 * s1 + s2 * s2).sqrt();
					if (s1 < 0.0 && s2 <= 0.0) || (s1 == 0.0 && s2 < 0.0) {
						s *= -1.0;
					}
					if rr < 0.0 {
						rr = 0.0;
						s = s1;
					} else if rr > atan_of_1 {
						rr = atan_of_1;
						s = (e0 - e2) / diag;
					}
				} else if e0 > e1 || e0 > e2 {
					if e0 > e1 {
						rr = 0.0;
						s = (e0 - e1) / grid_res;
					} else {
						rr = atan_of_1;
						s = (e0 - e2) / diag;
					}
				}

				if s >= max_slope && s > 0.0 {
					max_slope = s;
					dir = af * rr + ac * half_pi;
				}
			}

			if max_slope > 0.0 {
				let mut d = 360.0 - dir.to_degrees() + 90.0;
				if d > 360.0 {
					d -= 360.0;
				}
				out[i] = d;
			}
		}
	}

	out
}

impl Tool for ElevationAboveStreamTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "elevation_above_stream",
			display_name: "Elevation Above Stream",
			summary: "Computes elevation above nearest stream measured along downslope flow paths.",
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
				ToolParamSpec { name: "streams", description: "Input stream raster", required: true },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		ToolManifest {
			id: "elevation_above_stream".to_string(),
			display_name: "Elevation Above Stream".to_string(),
			summary: "Computes elevation above nearest stream measured along downslope flow paths.".to_string(),
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults: ToolArgs::new(),
			examples: vec![ToolExample {
				name: "hand_d8".to_string(),
				description: "Compute HAND-like elevation above streams using D8 flow routing".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "streams".to_string(), "hand".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		parse_raster_path_arg(args, "streams")?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let streams_path = parse_raster_path_arg(args, "streams")?;
		let streams = load_raster(&streams_path)?;
		if streams.rows != dem.rows || streams.cols != dem.cols {
			return Err(ToolError::Validation(
				"streams raster must match DEM dimensions".to_string(),
			));
		}

		let rows = dem.rows;
		let cols = dem.cols;
		let nodata = dem.nodata;
		let dirs = d8_dir_from_dem_local(&dem);
		let inflowing_vals: [i8; 8] = [4, 5, 6, 7, 0, 1, 2, 3];

		let background = f64::MIN;
		let mut out = vec![background; rows * cols];
		let mut stack = Vec::<(usize, f64)>::new();

		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				let z = dem.get(0, r as isize, c as isize);
				if z == nodata {
					out[i] = nodata;
					continue;
				}
				let s = streams.get(0, r as isize, c as isize);
				if s != streams.nodata && s > 0.0 {
					out[i] = 0.0;
					stack.push((i, z));
				} else if dirs[i] == -1 {
					out[i] = nodata;
					stack.push((i, nodata));
				}
			}
		}

		while let Some((i, stream_elev)) = stack.pop() {
			let r = i / cols;
			let c = i % cols;
			for n in 0..8 {
				let rn = r as isize + DY[n];
				let cn = c as isize + DX[n];
				if !in_bounds(rn, cn, rows, cols) {
					continue;
				}
				let ni = idx(rn as usize, cn as usize, cols);
				if dirs[ni] == inflowing_vals[n] && out[ni] == background {
					if stream_elev == nodata {
						out[ni] = nodata;
						stack.push((ni, nodata));
					} else {
						let zn = dem.get(0, rn, cn);
						out[ni] = zn - stream_elev;
						stack.push((ni, stream_elev));
					}
				}
			}
		}

		for v in &mut out {
			if *v == background {
				*v = nodata;
			}
		}

		let mut raster = vec_to_raster(&dem, &out, DataType::F32);
		raster.nodata = nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

impl Tool for ElevationAboveStreamEuclideanTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "elevation_above_stream_euclidean",
			display_name: "Elevation Above Stream Euclidean",
			summary: "Computes elevation above nearest stream using straight-line (Euclidean) proximity.",
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
				ToolParamSpec { name: "streams", description: "Input stream raster", required: true },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		ToolManifest {
			id: "elevation_above_stream_euclidean".to_string(),
			display_name: "Elevation Above Stream Euclidean".to_string(),
			summary: "Computes elevation above nearest stream using straight-line (Euclidean) proximity.".to_string(),
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults: ToolArgs::new(),
			examples: vec![ToolExample {
				name: "hand_euclidean".to_string(),
				description: "Compute stream-relative elevation from nearest stream by Euclidean proximity".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "streams".to_string(), "hand".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		parse_raster_path_arg(args, "streams")?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let streams_path = parse_raster_path_arg(args, "streams")?;
		let streams = load_raster(&streams_path)?;
		if streams.rows != dem.rows || streams.cols != dem.cols {
			return Err(ToolError::Validation(
				"streams raster must match DEM dimensions".to_string(),
			));
		}

		let rows = dem.rows;
		let cols = dem.cols;
		let nodata = dem.nodata;
		let cell_x = dem.cell_size_x;
		let cell_y = dem.cell_size_y;
		let diag = (cell_x * cell_x + cell_y * cell_y).sqrt();
		let step_lengths = [diag, cell_x, diag, cell_y, diag, cell_x, diag, cell_y];

		let mut dist = vec![f64::INFINITY; rows * cols];
		let mut source_elev = vec![nodata; rows * cols];
		let mut heap = BinaryHeap::<StreamDistNode>::new();

		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				let z = dem.get(0, r as isize, c as isize);
				if z == nodata {
					dist[i] = f64::INFINITY;
					continue;
				}
				let s = streams.get(0, r as isize, c as isize);
				if s != streams.nodata && s > 0.0 {
					dist[i] = 0.0;
					source_elev[i] = z;
					heap.push(StreamDistNode { dist: 0.0, i, stream_elev: z });
				}
			}
		}

		while let Some(node) = heap.pop() {
			if node.dist > dist[node.i] {
				continue;
			}
			let r = node.i / cols;
			let c = node.i % cols;
			for n in 0..8 {
				let rn = r as isize + DY[n];
				let cn = c as isize + DX[n];
				if !in_bounds(rn, cn, rows, cols) {
					continue;
				}
				let ni = idx(rn as usize, cn as usize, cols);
				let zn = dem.get(0, rn, cn);
				if zn == nodata {
					continue;
				}
				let cand = node.dist + step_lengths[n];
				if cand < dist[ni] {
					dist[ni] = cand;
					source_elev[ni] = node.stream_elev;
					heap.push(StreamDistNode {
						dist: cand,
						i: ni,
						stream_elev: node.stream_elev,
					});
				}
			}
		}

		let mut out = vec![nodata; rows * cols];
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				let z = dem.get(0, r as isize, c as isize);
				if z == nodata {
					continue;
				}
				if source_elev[i] != nodata {
					out[i] = z - source_elev[i];
				}
			}
		}

		let mut raster = vec_to_raster(&dem, &out, DataType::F32);
		raster.nodata = nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

impl Tool for DownslopeDistanceToStreamTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "downslope_distance_to_stream",
			display_name: "Downslope Distance To Stream",
			summary: "Computes downslope distance from each DEM cell to nearest stream along flow paths.",
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
				ToolParamSpec { name: "streams", description: "Input stream raster", required: true },
				ToolParamSpec { name: "dinf", description: "Use D-infinity routing instead of D8", required: false },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		let mut defaults = ToolArgs::new();
		defaults.insert("dinf".to_string(), json!(false));
		ToolManifest {
			id: "downslope_distance_to_stream".to_string(),
			display_name: "Downslope Distance To Stream".to_string(),
			summary: "Computes downslope distance from each DEM cell to nearest stream along flow paths.".to_string(),
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "distance_to_stream_d8".to_string(),
				description: "Compute downslope distance to stream using D8 routing".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "streams".to_string(), "distance".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		parse_raster_path_arg(args, "streams")?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let streams_path = parse_raster_path_arg(args, "streams")?;
		let streams = load_raster(&streams_path)?;
		if streams.rows != dem.rows || streams.cols != dem.cols {
			return Err(ToolError::Validation(
				"streams raster must match DEM dimensions".to_string(),
			));
		}

		let use_dinf = args.get("dinf").and_then(|v| v.as_bool()).unwrap_or(false);
		let rows = dem.rows;
		let cols = dem.cols;
		let nodata = dem.nodata;
		let inflowing_vals: [i8; 8] = [4, 5, 6, 7, 0, 1, 2, 3];
		let cell_x = dem.cell_size_x;
		let cell_y = dem.cell_size_y;
		let diag = (cell_x * cell_x + cell_y * cell_y).sqrt();
		let lengths = [diag, cell_x, diag, cell_y, diag, cell_x, diag, cell_y];

		let mut out = vec![f64::MIN; rows * cols];

		if !use_dinf {
			let dirs = d8_dir_from_dem_local(&dem);
			let mut stack = Vec::<(usize, f64)>::new();

			for r in 0..rows {
				for c in 0..cols {
					let i = idx(r, c, cols);
					let z = dem.get(0, r as isize, c as isize);
					if z == nodata {
						out[i] = nodata;
						continue;
					}
					let s = streams.get(0, r as isize, c as isize);
					if s != streams.nodata && s > 0.0 {
						out[i] = 0.0;
						stack.push((i, 0.0));
					} else if dirs[i] == -1 {
						out[i] = nodata;
						stack.push((i, nodata));
					}
				}
			}

			while let Some((i, stream_dist)) = stack.pop() {
				let r = i / cols;
				let c = i % cols;
				for n in 0..8 {
					let rn = r as isize + DY[n];
					let cn = c as isize + DX[n];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					if dirs[ni] == inflowing_vals[n] && out[ni] == f64::MIN {
						if stream_dist == nodata {
							out[ni] = nodata;
							stack.push((ni, nodata));
						} else {
							let d = stream_dist + lengths[n];
							out[ni] = d;
							stack.push((ni, d));
						}
					}
				}
			}
		} else {
			let flow_dir = dinf_angle_from_dem(&dem);
			let start_fd = [180.0, 225.0, 270.0, 315.0, 0.0, 45.0, 90.0, 135.0];
			let end_fd = [270.0, 315.0, 360.0, 45.0, 90.0, 135.0, 180.0, 225.0];
			let mut num_outflowing = vec![0i8; rows * cols];
			let mut queue = VecDeque::<usize>::new();

			for r in 0..rows {
				for c in 0..cols {
					let i = idx(r, c, cols);
					let z = dem.get(0, r as isize, c as isize);
					if z == nodata {
						out[i] = nodata;
						num_outflowing[i] = 0;
						continue;
					}
					let dir = flow_dir[i];
					if dir == -1.0 {
						num_outflowing[i] = 0;
					} else if (dir - 0.0).abs() < f64::EPSILON
						|| (dir - 45.0).abs() < f64::EPSILON
						|| (dir - 90.0).abs() < f64::EPSILON
						|| (dir - 135.0).abs() < f64::EPSILON
						|| (dir - 180.0).abs() < f64::EPSILON
						|| (dir - 225.0).abs() < f64::EPSILON
						|| (dir - 270.0).abs() < f64::EPSILON
						|| (dir - 315.0).abs() < f64::EPSILON
						|| (dir - 360.0).abs() < f64::EPSILON
					{
						num_outflowing[i] = 1;
					} else {
						num_outflowing[i] = 2;
					}

					let s = streams.get(0, r as isize, c as isize);
					if s != streams.nodata && s > 0.0 {
						out[i] = 0.0;
						num_outflowing[i] = -1;
						queue.push_back(i);
					} else {
						out[i] = f64::MAX;
					}
				}
			}

			while let Some(i) = queue.pop_front() {
				let r = i / cols;
				let c = i % cols;
				let d0 = out[i];
				for n in 0..8 {
					let rn = r as isize + DY[n];
					let cn = c as isize + DX[n];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					if num_outflowing[ni] <= 0 {
						continue;
					}
					let dir = flow_dir[ni];
					let receives = if n != 3 {
						dir > start_fd[n] && dir < end_fd[n]
					} else {
						dir > start_fd[n] || dir < end_fd[n]
					};
					if receives {
						let cand = d0 + lengths[n];
						if cand < out[ni] {
							out[ni] = cand;
						}
						num_outflowing[ni] -= 1;
						if num_outflowing[ni] == 0 {
							queue.push_back(ni);
						}
					}
				}
			}

			for v in &mut out {
				if *v == f64::MAX {
					*v = nodata;
				}
			}
		}

		for v in &mut out {
			if *v == f64::MIN {
				*v = nodata;
			}
		}

		let mut raster = vec_to_raster(&dem, &out, DataType::F32);
		raster.nodata = nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

impl Tool for DepthToWaterTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "depth_to_water",
			display_name: "Depth To Water",
			summary: "Computes cartographic depth-to-water using least-cost accumulation from stream/lake source features.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
				ToolParamSpec { name: "streams", description: "Optional stream vector (line/multiline)", required: false },
				ToolParamSpec { name: "lakes", description: "Optional waterbody vector (polygon/multipolygon)", required: false },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		ToolManifest {
			id: "depth_to_water".to_string(),
			display_name: "Depth To Water".to_string(),
			summary: "Computes cartographic depth-to-water using least-cost accumulation from stream/lake source features.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults: ToolArgs::new(),
			examples: vec![ToolExample {
				name: "dtw_streams_lakes".to_string(),
				description: "Compute depth-to-water from streams and optional lake polygons".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "wetness".to_string(), "dtw".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		if args.get("streams").is_none() && args.get("lakes").is_none() {
			return Err(ToolError::Validation(
				"either 'streams' and/or 'lakes' must be supplied".to_string(),
			));
		}
		if args.get("streams").is_some() {
			parse_vector_path_arg(args, "streams")?;
		}
		if args.get("lakes").is_some() {
			parse_vector_path_arg(args, "lakes")?;
		}
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let rows = dem.rows;
		let cols = dem.cols;
		let nodata = dem.nodata;

		let mut sources = vec![0u8; rows * cols];
		if args.get("streams").is_some() {
			let streams_path = parse_vector_path_arg(args, "streams")?;
			let layer = read_vector_layer_aligned_to_dem(&dem, &streams_path, "streams")?;
			for feat in &layer.features {
				if let Some(ref g) = feat.geometry {
					rasterize_line_geometry(&mut sources, rows, cols, &dem, g);
				}
			}
		}
		if args.get("lakes").is_some() {
			let lakes_path = parse_vector_path_arg(args, "lakes")?;
			let layer = read_vector_layer_aligned_to_dem(&dem, &lakes_path, "lakes")?;
			for feat in &layer.features {
				if let Some(ref g) = feat.geometry {
					rasterize_polygon_boundaries(&mut sources, rows, cols, &dem, g);
				}
			}
		}

		if !sources.iter().any(|&v| v > 0) {
			return Err(ToolError::Validation(
				"no stream/lake source cells were rasterized; check layer overlap with DEM".to_string(),
			));
		}

		let cell_x = dem.cell_size_x;
		let cell_y = dem.cell_size_y;
		let diag = (cell_x * cell_x + cell_y * cell_y).sqrt();
		let lengths = [diag, cell_x, diag, cell_y, diag, cell_x, diag, cell_y];

		let mut dist = vec![f64::INFINITY; rows * cols];
		let mut heap = BinaryHeap::<DtwNode>::new();
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				let z = dem.get(0, r as isize, c as isize);
				if z == nodata {
					continue;
				}
				if sources[i] > 0 {
					dist[i] = 0.0;
					heap.push(DtwNode { cost: 0.0, i });
				}
			}
		}

		while let Some(node) = heap.pop() {
			if node.cost > dist[node.i] {
				continue;
			}
			let r = node.i / cols;
			let c = node.i % cols;
			let z0 = dem.get(0, r as isize, c as isize);
			for n in 0..8 {
				let rn = r as isize + DY[n];
				let cn = c as isize + DX[n];
				if !in_bounds(rn, cn, rows, cols) {
					continue;
				}
				let ni = idx(rn as usize, cn as usize, cols);
				let zn = dem.get(0, rn, cn);
				if zn == nodata {
					continue;
				}
				let slope = ((zn - z0).abs() / lengths[n]).max(1.0e-6);
				let cand = node.cost + slope * lengths[n];
				if cand < dist[ni] {
					dist[ni] = cand;
					heap.push(DtwNode { cost: cand, i: ni });
				}
			}
		}

		let mut out = vec![nodata; rows * cols];
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				if dem.get(0, r as isize, c as isize) == nodata {
					continue;
				}
				if dist[i].is_finite() {
					out[i] = dist[i];
				}
			}
		}

		let mut raster = vec_to_raster(&dem, &out, DataType::F32);
		raster.nodata = nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

impl Tool for FillBurnTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "fill_burn",
			display_name: "Fill Burn",
			summary: "Hydro-enforces a DEM by burning streams and then filling depressions.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
				ToolParamSpec { name: "streams", description: "Input streams vector (line/multiline)", required: true },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		ToolManifest {
			id: "fill_burn".to_string(),
			display_name: "Fill Burn".to_string(),
			summary: "Hydro-enforces a DEM by burning streams and then filling depressions.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults: ToolArgs::new(),
			examples: vec![ToolExample {
				name: "fill_burn_example".to_string(),
				description: "Burn streams and fill depressions to produce hydro-enforced DEM".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "dem".to_string(), "stream_burning".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		parse_vector_path_arg(args, "streams")?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let streams_path = parse_vector_path_arg(args, "streams")?;
		let mask = stream_mask_from_vector(&dem, &streams_path, "streams")?;

		if !mask.iter().any(|&v| v > 0) {
			return Err(ToolError::Validation(
				"no stream cells were rasterized; check stream layer overlap with DEM".to_string(),
			));
		}

		let mut lowered = raster_to_vec(&dem);
		for i in 0..lowered.len() {
			if mask[i] > 0 && lowered[i] != dem.nodata {
				lowered[i] -= 10_000.0;
			}
		}

		let small = auto_small_increment(&dem, None);
		let mut out = fill_depressions_wang_and_liu_core(&lowered, dem.rows, dem.cols, dem.nodata, small);
		let mut min_diff = f64::INFINITY;
		for i in 0..out.len() {
			if mask[i] > 0 && out[i] != dem.nodata && lowered[i] != dem.nodata {
				let d = lowered[i] + 10_000.0 - out[i];
				if d < min_diff {
					min_diff = d;
				}
			}
		}
		if !min_diff.is_finite() {
			min_diff = 0.0;
		}
		let adj = min_diff - 1.0;
		for i in 0..out.len() {
			if mask[i] > 0 && out[i] != dem.nodata {
				out[i] += adj;
			}
		}

		let mut raster = vec_to_raster(&dem, &out, DataType::F32);
		raster.nodata = dem.nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

impl Tool for BurnStreamsAtRoadsTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "burn_streams_at_roads",
			display_name: "Burn Streams At Roads",
			summary: "Lowers stream elevations near stream-road crossings to breach road embankments in a DEM.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
				ToolParamSpec { name: "streams", description: "Input streams vector", required: true },
				ToolParamSpec { name: "roads", description: "Input roads vector", required: true },
				ToolParamSpec { name: "road_width", description: "Maximum road embankment width (map units)", required: true },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		let mut defaults = ToolArgs::new();
		defaults.insert("road_width".to_string(), json!(0.0));
		ToolManifest {
			id: "burn_streams_at_roads".to_string(),
			display_name: "Burn Streams At Roads".to_string(),
			summary: "Lowers stream elevations near stream-road crossings to breach road embankments in a DEM.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "burn_stream_road_crossings".to_string(),
				description: "Burn stream cells near road intersections".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "dem".to_string(), "roads".to_string(), "stream_burning".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		parse_vector_path_arg(args, "streams")?;
		parse_vector_path_arg(args, "roads")?;
		let road_width = args.get("road_width").and_then(|v| v.as_f64()).unwrap_or(0.0);
		if road_width <= 0.0 {
			return Err(ToolError::Validation("'road_width' must be > 0".to_string()));
		}
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let streams_path = parse_vector_path_arg(args, "streams")?;
		let roads_path = parse_vector_path_arg(args, "roads")?;
		let road_width = args.get("road_width").and_then(|v| v.as_f64()).unwrap_or(0.0);

		let stream_mask = stream_mask_from_vector(&dem, &streams_path, "streams")?;
		let road_mask = stream_mask_from_vector(&dem, &roads_path, "roads")?;

		let rows = dem.rows;
		let cols = dem.cols;
		let mut out = raster_to_vec(&dem);
		let grid_res = ((dem.cell_size_x.abs() + dem.cell_size_y.abs()) / 2.0).max(1.0e-12);
		let width_cells = ((road_width / grid_res).ceil() as usize / 2).max(1);

		let mut intersections = Vec::<usize>::new();
		for i in 0..rows * cols {
			if stream_mask[i] > 0 && road_mask[i] > 0 && out[i] != dem.nodata {
				intersections.push(i);
			}
		}

		for &seed in &intersections {
			let mut q = VecDeque::<(usize, usize)>::new();
			let mut visited = HashSet::<usize>::new();
			let mut touched = Vec::<usize>::new();
			let mut minz = f64::INFINITY;

			q.push_back((seed, 0));
			while let Some((i, d)) = q.pop_front() {
				if !visited.insert(i) {
					continue;
				}
				if stream_mask[i] == 0 || out[i] == dem.nodata {
					continue;
				}
				touched.push(i);
				if out[i] < minz {
					minz = out[i];
				}
				if d >= width_cells {
					continue;
				}
				let r = i / cols;
				let c = i % cols;
				for n in 0..8 {
					let rn = r as isize + DY[n];
					let cn = c as isize + DX[n];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					if stream_mask[ni] > 0 {
						q.push_back((ni, d + 1));
					}
				}
			}

			if minz.is_finite() {
				for i in touched {
					if out[i] > minz {
						out[i] = minz;
					}
				}
			}
		}

		let mut raster = vec_to_raster(&dem, &out, DataType::F32);
		raster.nodata = dem.nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

impl Tool for TraceDownslopeFlowpathsTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "trace_downslope_flowpaths",
			display_name: "Trace Downslope Flowpaths",
			summary: "Traces downslope D8 flowpaths from seed points and outputs a visit-count raster.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "seed_points", description: "Input point vector of seed locations", required: true },
				ToolParamSpec { name: "d8_pntr", description: "Input D8 pointer raster", required: true },
				ToolParamSpec { name: "esri_pntr", description: "Use ESRI pointer encoding", required: false },
				ToolParamSpec { name: "zero_background", description: "Use 0 instead of NoData for background cells", required: false },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		let mut defaults = ToolArgs::new();
		defaults.insert("esri_pntr".to_string(), json!(false));
		defaults.insert("zero_background".to_string(), json!(false));
		ToolManifest {
			id: "trace_downslope_flowpaths".to_string(),
			display_name: "Trace Downslope Flowpaths".to_string(),
			summary: "Marks D8 flowpaths initiated from seed points until no-flow or grid edge.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "trace_flowpaths".to_string(),
				description: "Trace flowpaths from a point vector over a D8 pointer grid".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "d8".to_string(), "flowpath".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "d8_pntr")
			.or_else(|_| parse_raster_path_arg(args, "d8_pointer"))
			.or_else(|_| parse_raster_path_arg(args, "input"))?;
		parse_vector_path_arg(args, "seed_points")
			.or_else(|_| parse_vector_path_arg(args, "seed_pts"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (pntr, output_path) = parse_pointer_input(args)?;
		let seeds_path = parse_vector_path_arg(args, "seed_points")
			.or_else(|_| parse_vector_path_arg(args, "seed_pts"))?;
		let esri_style = args.get("esri_pntr").and_then(|v| v.as_bool()).unwrap_or(false);
		let zero_background = args.get("zero_background").and_then(|v| v.as_bool()).unwrap_or(false);

		let layer = read_vector_layer_aligned_to_dem(&pntr, &seeds_path, "seed_points")?;
		let rows = pntr.rows;
		let cols = pntr.cols;
		let out_nodata = -32768.0f64;
		let background = if zero_background { 0.0 } else { out_nodata };
		let mut out = vec![background; rows * cols];

		let flow_dir = build_flow_dir_and_mark_nodata(&pntr, esri_style, &mut out, out_nodata, cols);

		for feature in &layer.features {
			let Some(ref geom) = feature.geometry else { continue };
			let coords: Vec<(f64, f64)> = match geom {
				wbvector::Geometry::Point(c) => vec![(c.x, c.y)],
				wbvector::Geometry::MultiPoint(cs) => cs.iter().map(|c| (c.x, c.y)).collect(),
				_ => Vec::new(),
			};

			for (x, y) in coords {
				let Some((col, row)) = pntr.world_to_pixel(x, y) else { continue };
				if !in_bounds(row, col, rows, cols) {
					continue;
				}

				let mut r = row;
				let mut c = col;
				let mut steps = 0usize;
				let max_steps = rows * cols;
				while in_bounds(r, c, rows, cols) && steps <= max_steps {
					let i = idx(r as usize, c as usize, cols);
					if out[i] == background {
						out[i] = 1.0;
					} else if out[i] != out_nodata {
						out[i] += 1.0;
					}
					let dir = flow_dir[i];
					if dir < 0 {
						break;
					}
					r += DY[dir as usize];
					c += DX[dir as usize];
					steps += 1;
				}
			}
		}

		let mut raster = vec_to_raster(&pntr, &out, DataType::I32);
		raster.nodata = out_nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

impl Tool for FloodOrderTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "flood_order",
			display_name: "Flood Order",
			summary: "Computes flood order as the priority-flood visitation order from DEM edges inward.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		ToolManifest {
			id: "flood_order".to_string(),
			display_name: "Flood Order".to_string(),
			summary: "Outputs the sequential priority-flood order for each DEM cell.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults: ToolArgs::new(),
			examples: vec![ToolExample {
				name: "flood_order".to_string(),
				description: "Compute flood order from a DEM using priority-flood traversal".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "dem".to_string(), "priority_flood".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let rows = dem.rows;
		let cols = dem.cols;
		let nodata = dem.nodata;
		let num_cells = rows * cols;

		let background = (i32::MIN + 1) as f64;
		let mut filled = vec![background; num_cells];
		let mut out = vec![background; num_cells];

		let mut queue = VecDeque::<(isize, isize)>::with_capacity(num_cells);
		for r in 0..rows {
			queue.push_back((r as isize, -1));
			queue.push_back((r as isize, cols as isize));
		}
		for c in 0..cols {
			queue.push_back((-1, c as isize));
			queue.push_back((rows as isize, c as isize));
		}

		let mut heap = BinaryHeap::<MinNode>::with_capacity(num_cells);
		while let Some((r, c)) = queue.pop_front() {
			for n in 0..8 {
				let rn = r + DY[n];
				let cn = c + DX[n];
				if !in_bounds(rn, cn, rows, cols) {
					continue;
				}
				let ni = idx(rn as usize, cn as usize, cols);
				if filled[ni] != background {
					continue;
				}
				let zin = dem.get(0, rn, cn);
				if zin == nodata {
					filled[ni] = nodata;
					out[ni] = nodata;
					queue.push_back((rn, cn));
				} else {
					filled[ni] = zin;
					heap.push(MinNode { elev: zin, i: ni });
				}
			}
		}

		let mut order_val = 1.0f64;
		while let Some(cell) = heap.pop() {
			let i = cell.i;
			let r = i / cols;
			let c = i % cols;
			let z = filled[i];
			out[i] = order_val;
			order_val += 1.0;

			for n in 0..8 {
				let rn = r as isize + DY[n];
				let cn = c as isize + DX[n];
				if !in_bounds(rn, cn, rows, cols) {
					continue;
				}
				let ni = idx(rn as usize, cn as usize, cols);
				if filled[ni] != background {
					continue;
				}
				let mut zn = dem.get(0, rn, cn);
				if zn == nodata {
					filled[ni] = nodata;
					out[ni] = nodata;
					continue;
				}
				if zn < z {
					zn = z;
				}
				filled[ni] = zn;
				heap.push(MinNode { elev: zn, i: ni });
			}
		}

		for v in &mut out {
			if *v == background {
				*v = nodata;
			}
		}

		let mut raster = vec_to_raster(&dem, &out, DataType::I32);
		raster.nodata = nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

impl Tool for InsertDamsTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "insert_dams",
			display_name: "Insert Dams",
			summary: "Inserts local dam embankments at point locations using a maximum dam length.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
				ToolParamSpec { name: "dam_points", description: "Input dam point vector", required: true },
				ToolParamSpec { name: "dam_length", description: "Maximum dam length (map units)", required: true },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		ToolManifest {
			id: "insert_dams".to_string(),
			display_name: "Insert Dams".to_string(),
			summary: "Adds local dam embankments at specified points using profile-based crest selection.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults: ToolArgs::new(),
			examples: vec![ToolExample {
				name: "insert_dams".to_string(),
				description: "Insert dams at candidate locations from a point layer".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "dem".to_string(), "dams".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		parse_vector_path_arg(args, "dam_points").or_else(|_| parse_vector_path_arg(args, "dam_pts"))?;
		let dam_length = args
			.get("dam_length")
			.or_else(|| args.get("damlength"))
			.and_then(|v| v.as_f64())
			.ok_or_else(|| ToolError::Validation("'dam_length' is required".to_string()))?;
		if dam_length <= 0.0 {
			return Err(ToolError::Validation("'dam_length' must be > 0".to_string()));
		}
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let points_path = parse_vector_path_arg(args, "dam_points").or_else(|_| parse_vector_path_arg(args, "dam_pts"))?;
		let layer = read_vector_layer_aligned_to_dem(&dem, &points_path, "dam_points")?;
		let dam_length = args
			.get("dam_length")
			.or_else(|| args.get("damlength"))
			.and_then(|v| v.as_f64())
			.unwrap_or(0.0);

		let mut point_coords = Vec::<(f64, f64)>::new();
		for feat in &layer.features {
			let Some(ref geom) = feat.geometry else { continue };
			match geom {
				wbvector::Geometry::Point(c) => point_coords.push((c.x, c.y)),
				wbvector::Geometry::MultiPoint(cs) => {
					for c in cs {
						point_coords.push((c.x, c.y));
					}
				}
				_ => {}
			}
		}

		if point_coords.is_empty() {
			return Err(ToolError::Validation(
				"dam_points layer must contain point geometries".to_string(),
			));
		}

		let rows = dem.rows;
		let cols = dem.cols;
		let nodata = dem.nodata;
		let mut out = raster_to_vec(&dem);

		let dx: [isize; 8] = [1, 1, 1, 0, -1, -1, -1, 0];
		let dy: [isize; 8] = [-1, 0, 1, 1, 1, 0, -1, -1];
		let perp1: [usize; 4] = [2, 3, 4, 1];
		let perp2: [usize; 4] = [6, 7, 0, 5];

		let half_len = (dam_length / 2.0).floor().max(1.0) as isize;
		let profile_len = (half_len * 2 + 1) as usize;

		let getz = |r: isize, c: isize| -> f64 {
			if in_bounds(r, c, rows, cols) {
				dem.get(0, r, c)
			} else {
				nodata
			}
		};

		for (x, y) in point_coords {
			let Some((tc, tr)) = dem.world_to_pixel(x, y) else { continue };
			if !in_bounds(tr, tc, rows, cols) {
				continue;
			}

			let target_row = tr;
			let target_col = tc;
			let dam_z = getz(target_row, target_col);
			if dam_z == nodata {
				continue;
			}

			let mut max_dam_height = f64::NEG_INFINITY;
			let mut best_row = target_row;
			let mut best_col = target_col;
			let mut best_dir = 0usize;
			let mut best_profile_filled = vec![f64::NEG_INFINITY; profile_len];

			for row in (target_row - half_len)..=(target_row + half_len) {
				for col in (target_col - half_len)..=(target_col + half_len) {
					let z = getz(row, col);
					if z == nodata {
						continue;
					}

					for dir in 0..4usize {
						let mut profile = vec![f64::NEG_INFINITY; profile_len];
						let mut profile_filled = vec![f64::NEG_INFINITY; profile_len];
						let mut intersects_target = false;
						let mut target_cell = half_len as usize;

						profile[half_len as usize] = z;

						let mut r1 = row;
						let mut c1 = col;
						let mut r2 = row;
						let mut c2 = col;
						for i in 1..=half_len {
							r1 += dy[perp1[dir]];
							c1 += dx[perp1[dir]];
							if r1 == target_row && c1 == target_col {
								intersects_target = true;
								target_cell = (half_len + i) as usize;
							}
							profile[(half_len + i) as usize] = getz(r1, c1);

							r2 += dy[perp2[dir]];
							c2 += dx[perp2[dir]];
							if r2 == target_row && c2 == target_col {
								intersects_target = true;
								target_cell = (half_len - i) as usize;
							}
							profile[(half_len - i) as usize] = getz(r2, c2);
						}

						if !intersects_target {
							continue;
						}

						profile_filled[0] = profile[0];
						for i in 1..profile_len {
							profile_filled[i] = profile_filled[i - 1].max(profile[i]);
						}
						for i in (0..profile_len - 1).rev() {
							profile_filled[i] = profile_filled[i].min(profile_filled[i + 1].max(profile[i]));
						}

						if profile_filled[target_cell] > max_dam_height {
							max_dam_height = profile_filled[target_cell];
							best_row = row;
							best_col = col;
							best_dir = dir;
							best_profile_filled = profile_filled;
						}
					}
				}
			}

			if max_dam_height <= dam_z || !max_dam_height.is_finite() {
				continue;
			}

			let mut r1 = best_row;
			let mut c1 = best_col;
			let mut r2 = best_row;
			let mut c2 = best_col;

			for i in 0..=half_len {
				let ii_a = (half_len + i) as usize;
				let ii_b = (half_len - i) as usize;

				if in_bounds(r1, c1, rows, cols) {
					let o = idx(r1 as usize, c1 as usize, cols);
					if out[o] != nodata {
						out[o] = out[o].max(best_profile_filled[ii_a]);
					}
				}
				if in_bounds(r2, c2, rows, cols) {
					let o = idx(r2 as usize, c2 as usize, cols);
					if out[o] != nodata {
						out[o] = out[o].max(best_profile_filled[ii_b]);
					}
				}

				if best_dir == 0 || best_dir == 2 {
					if in_bounds(r1 - 1, c1, rows, cols) {
						let o = idx((r1 - 1) as usize, c1 as usize, cols);
						if out[o] != nodata {
							out[o] = out[o].max(best_profile_filled[ii_a]);
						}
					}
					if in_bounds(r2 - 1, c2, rows, cols) {
						let o = idx((r2 - 1) as usize, c2 as usize, cols);
						if out[o] != nodata {
							out[o] = out[o].max(best_profile_filled[ii_b]);
						}
					}
				}

				if i < half_len {
					r1 += dy[perp1[best_dir]];
					c1 += dx[perp1[best_dir]];
					r2 += dy[perp2[best_dir]];
					c2 += dx[perp2[best_dir]];
				}
			}
		}

		let mut raster = vec_to_raster(&dem, &out, DataType::F32);
		raster.nodata = nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

impl Tool for RaiseWallsTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "raise_walls",
			display_name: "Raise Walls",
			summary: "Raises DEM elevations along wall line features, with optional breach lines.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
				ToolParamSpec { name: "walls", description: "Input wall vector", required: true },
				ToolParamSpec { name: "breach_lines", description: "Optional breach-line vector", required: false },
				ToolParamSpec { name: "wall_height", description: "Wall height increment", required: false },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		let mut defaults = ToolArgs::new();
		defaults.insert("wall_height".to_string(), json!(100.0));
		ToolManifest {
			id: "raise_walls".to_string(),
			display_name: "Raise Walls".to_string(),
			summary: "Raises DEM elevations along wall vectors and optionally breaches selected crossings.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "raise_walls".to_string(),
				description: "Raise elevation walls and optionally carve breach segments".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "dem".to_string(), "walls".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		parse_vector_path_arg(args, "walls").or_else(|_| parse_vector_path_arg(args, "input_vector"))?;
		if let Some(v) = args.get("wall_height").and_then(|v| v.as_f64()) {
			if !v.is_finite() {
				return Err(ToolError::Validation("'wall_height' must be finite".to_string()));
			}
		}
		if args.get("breach_lines").is_some() {
			parse_vector_path_arg(args, "breach_lines")?;
		}
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let walls_path = parse_vector_path_arg(args, "walls").or_else(|_| parse_vector_path_arg(args, "input_vector"))?;
		let walls = read_vector_layer_aligned_to_dem(&dem, &walls_path, "walls")?;
		let wall_height = args.get("wall_height").and_then(|v| v.as_f64()).unwrap_or(100.0);

		let rows = dem.rows;
		let cols = dem.cols;
		let mut out = raster_to_vec(&dem);
		let mut walled = vec![0u8; rows * cols];

		for feat in &walls.features {
			let Some(ref geom) = feat.geometry else { continue };
			rasterize_line_geometry(&mut walled, rows, cols, &dem, geom);
			rasterize_polygon_boundaries(&mut walled, rows, cols, &dem, geom);
		}

		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				if walled[i] > 0 && out[i] != dem.nodata {
					out[i] += wall_height;
				}
			}
		}

		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				if walled[i] == 0 {
					continue;
				}
				if r > 0 && c + 1 < cols {
					let i_ne = idx(r - 1, c + 1, cols);
					if walled[i_ne] > 0 {
						let i_n = idx(r - 1, c, cols);
						let i_e = idx(r, c + 1, cols);
						if walled[i_n] == 0 && walled[i_e] == 0 {
							if out[i_n] != dem.nodata {
								out[i_n] += wall_height;
								walled[i_n] = 1;
							}
						}
					}
				}
				if r + 1 < rows && c + 1 < cols {
					let i_se = idx(r + 1, c + 1, cols);
					if walled[i_se] > 0 {
						let i_e = idx(r, c + 1, cols);
						let i_s = idx(r + 1, c, cols);
						if walled[i_e] == 0 && walled[i_s] == 0 {
							if out[i_e] != dem.nodata {
								out[i_e] += wall_height;
								walled[i_e] = 1;
							}
						}
					}
				}
			}
		}

		if args.get("breach_lines").is_some() {
			let breach_path = parse_vector_path_arg(args, "breach_lines")?;
			let breaches = read_vector_layer_aligned_to_dem(&dem, &breach_path, "breach_lines")?;
			let mut breach_mask = vec![0u8; rows * cols];
			for feat in &breaches.features {
				let Some(ref geom) = feat.geometry else { continue };
				rasterize_line_geometry(&mut breach_mask, rows, cols, &dem, geom);
				rasterize_polygon_boundaries(&mut breach_mask, rows, cols, &dem, geom);
			}
			for r in 0..rows {
				for c in 0..cols {
					let i = idx(r, c, cols);
					if breach_mask[i] > 0 {
						out[i] = dem.get(0, r as isize, c as isize);
					}
				}
			}
		}

		let mut raster = vec_to_raster(&dem, &out, DataType::F32);
		raster.nodata = dem.nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

impl Tool for TopologicalBreachBurnTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "topological_breach_burn",
			display_name: "Topological Breach Burn",
			summary: "Performs stream burning with topological safeguards and returns streams, burned DEM, D8 pointer, and D8 accumulation outputs.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Pro,
			params: vec![
				ToolParamSpec { name: "streams", description: "Input streams vector", required: true },
				ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
				ToolParamSpec { name: "snap_distance", description: "Snap distance (map units)", required: false },
				ToolParamSpec { name: "out_streams", description: "Optional output path for streams raster", required: false },
				ToolParamSpec { name: "out_dem", description: "Optional output path for burned DEM", required: false },
				ToolParamSpec { name: "out_dir", description: "Optional output path for D8 pointer", required: false },
				ToolParamSpec { name: "out_fa", description: "Optional output path for D8 flow accumulation", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		let mut defaults = ToolArgs::new();
		defaults.insert("snap_distance".to_string(), json!(0.001));
		ToolManifest {
			id: "topological_breach_burn".to_string(),
			display_name: "Topological Breach Burn".to_string(),
			summary: "Burns streams into a DEM, conditions the surface, and returns stream, DEM, pointer, and accumulation rasters.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Pro,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "topological_breach_burn".to_string(),
				description: "Generate topologically conditioned stream-burning outputs".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "stream_burning".to_string(), "d8".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_vector_path_arg(args, "streams")?;
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		if let Some(v) = args.get("snap_distance").and_then(|v| v.as_f64()) {
			if !v.is_finite() || v < 0.0 {
				return Err(ToolError::Validation("'snap_distance' must be >= 0".to_string()));
			}
		}
		Ok(())
	}

	fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let dem_path = parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		let dem = load_raster(&dem_path)?;
		let streams_path = parse_vector_path_arg(args, "streams")?;
		let streams_layer = read_vector_layer_aligned_to_dem(&dem, &streams_path, "streams")?;

		let snap_distance = args.get("snap_distance").and_then(|v| v.as_f64()).unwrap_or(0.001).max(0.0);
		let out_streams = parse_optional_output_from_keys(args, &["out_streams", "output_streams"])?;
		let out_dem = parse_optional_output_from_keys(args, &["out_dem", "output_dem"])?;
		let out_dir = parse_optional_output_from_keys(args, &["out_dir", "output_dir"])?;
		let out_fa = parse_optional_output_from_keys(args, &["out_fa", "output", "output_fa"])?;

		let rows = dem.rows;
		let cols = dem.cols;

		let analysis_path = unique_temp_path("wbtools_oss_tbb_analysis", "geojson");
		let mut analysis_args = ToolArgs::new();
		analysis_args.insert("input_vector".to_string(), json!(streams_path.clone()));
		analysis_args.insert("dem".to_string(), json!(dem_path.clone()));
		analysis_args.insert("output".to_string(), json!(analysis_path.to_string_lossy().to_string()));
		analysis_args.insert("snap".to_string(), json!(snap_distance));
		let vsa_tool = VectorStreamNetworkAnalysisTool;
		vsa_tool.run(&analysis_args, ctx)?;

		let analyzed = wbvector::read(&analysis_path).map_err(|e| {
			ToolError::Execution(format!("failed reading vector_stream_network_analysis output: {}", e))
		})?;
		let tucl_idx = analyzed.schema.field_index("TUCL");
		let trib_idx = analyzed.schema.field_index("TRIB_ID");

		struct LinkCells {
			cells: Vec<usize>,
			tucl: f64,
			trib: i64,
		}

		let mut links = Vec::<LinkCells>::new();
		let mut unique_tucl = Vec::<f64>::new();
		for feat in &analyzed.features {
			let Some(ref geom) = feat.geometry else { continue };
			let cells = collect_line_cells_geometry(rows, cols, &dem, geom);
			if cells.is_empty() {
				continue;
			}
			let tucl = tucl_idx
				.and_then(|ix| feat.get_by_index(ix).and_then(|v| v.as_f64()))
				.unwrap_or(cells.len() as f64)
				.max(0.0);
			let trib = trib_idx
				.and_then(|ix| feat.get_by_index(ix).and_then(|v| v.as_i64()))
				.unwrap_or((feat.fid as i64) + 1)
				.max(1);
			links.push(LinkCells { cells, tucl, trib });
			unique_tucl.push(tucl);
		}

		if links.is_empty() {
			for feat in &streams_layer.features {
				let Some(ref geom) = feat.geometry else { continue };
				let cells = collect_line_cells_geometry(rows, cols, &dem, geom);
				if cells.is_empty() {
					continue;
				}
				let tucl = cells.len() as f64;
				let trib = (feat.fid as i64) + 1;
				links.push(LinkCells { cells, tucl, trib });
				unique_tucl.push(tucl);
			}
		}

		if links.is_empty() {
			return Err(ToolError::Validation(
				"no stream cells were rasterized from analyzed stream network".to_string(),
			));
		}

		unique_tucl.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
		unique_tucl.dedup_by(|a, b| (*a - *b).abs() <= 1.0e-12);
		let mut candidates = Vec::<f64>::new();
		if unique_tucl.len() <= 24 {
			candidates.extend(unique_tucl.iter().copied());
		} else {
			for i in 0..24 {
				let p = i as f64 / 23.0;
				let idx_q = ((unique_tucl.len() - 1) as f64 * p).round() as usize;
				candidates.push(unique_tucl[idx_q]);
			}
			candidates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
			candidates.dedup_by(|a, b| (*a - *b).abs() <= 1.0e-12);
		}

		let total_cells: usize = links.iter().map(|l| l.cells.len()).sum();
		let mut best_threshold = 0.0;
		let mut best_score = f64::NEG_INFINITY;
		for threshold in candidates {
			let mut seen = vec![false; rows * cols];
			let mut owner = vec![0i64; rows * cols];
			let mut conflict = vec![false; rows * cols];
			let mut occupied = 0usize;
			let mut collisions = 0usize;
			let mut kept_cells = 0usize;
			for link in links.iter().filter(|l| l.tucl >= threshold) {
				kept_cells += link.cells.len();
				for &cell in &link.cells {
					if !seen[cell] {
						seen[cell] = true;
						owner[cell] = link.trib;
						occupied += 1;
					} else if owner[cell] != link.trib && !conflict[cell] {
						conflict[cell] = true;
						collisions += 1;
					}
				}
			}
			if occupied == 0 {
				continue;
			}
			let keep_ratio = kept_cells as f64 / (total_cells.max(1) as f64);
			let collision_ratio = collisions as f64 / (occupied as f64);
			let score = keep_ratio - 0.6 * collision_ratio;
			if score > best_score {
				best_score = score;
				best_threshold = threshold;
			}
		}

		let mut stream_trib = vec![0i64; rows * cols];
		let mut stream_tucl = vec![f64::NEG_INFINITY; rows * cols];
		for link in links.iter().filter(|l| l.tucl >= best_threshold) {
			for &cell in &link.cells {
				if link.tucl > stream_tucl[cell] {
					stream_tucl[cell] = link.tucl;
					stream_trib[cell] = link.trib;
				}
			}
		}

		if !stream_trib.iter().any(|&v| v > 0) {
			return Err(ToolError::Validation(
				"stream pruning removed all streams; inputs are likely invalid".to_string(),
			));
		}

		let stream_nodata = -32768.0;
		let mut stream_data = vec![stream_nodata; rows * cols];
		for i in 0..rows * cols {
			if dem.get(0, (i / cols) as isize, (i % cols) as isize) == dem.nodata {
				continue;
			}
			stream_data[i] = if stream_trib[i] > 0 { stream_trib[i] as f64 } else { 0.0 };
		}
		let mut stream_raster = vec_to_raster(&dem, &stream_data, DataType::I32);
		stream_raster.nodata = stream_nodata;
		let stream_path = write_or_store_output(stream_raster, out_streams)?;

		let mut burned = raster_to_vec(&dem);
		let cell_len = ((dem.cell_size_x.abs() + dem.cell_size_y.abs()) / 2.0).max(1.0e-12);
		let burn_depth_base = (snap_distance.max(cell_len) * 10.0).max(1.0);
		let max_tucl = stream_tucl
			.iter()
			.copied()
			.filter(|v| v.is_finite() && *v > 0.0)
			.fold(0.0, f64::max)
			.max(1.0);
		for i in 0..burned.len() {
			if stream_trib[i] > 0 && burned[i] != dem.nodata {
				let rel = (stream_tucl[i] / max_tucl).clamp(0.0, 1.0);
				let depth = burn_depth_base * (1.0 + 0.75 * rel);
				burned[i] -= depth;
			}
		}

		let small = auto_small_increment(&dem, None);
		let conditioned = fill_depressions_wang_and_liu_core(&burned, rows, cols, dem.nodata, small);
		let mut conditioned_raster = vec_to_raster(&dem, &conditioned, DataType::F32);
		conditioned_raster.nodata = dem.nodata;
		let conditioned_path = write_or_store_output(conditioned_raster.clone(), out_dem)?;

		let dirs = d8_dir_from_dem_local(&conditioned_raster);
		let d8_values = [1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0];
		let mut pointer_data = vec![0.0f64; rows * cols];
		for i in 0..rows * cols {
			let z = conditioned_raster.get(0, (i / cols) as isize, (i % cols) as isize);
			if z == dem.nodata {
				pointer_data[i] = dem.nodata;
				continue;
			}
			let dir = dirs[i];
			pointer_data[i] = if dir >= 0 { d8_values[dir as usize] } else { 0.0 };
		}

		for i in 0..rows * cols {
			if stream_trib[i] <= 0 || conditioned_raster.get(0, (i / cols) as isize, (i % cols) as isize) == dem.nodata {
				continue;
			}
			let r = i / cols;
			let c = i % cols;
			let z0 = conditioned_raster.get(0, r as isize, c as isize);
			let mut best_dir = -1i8;
			let mut best_rank = (false, false, f64::NEG_INFINITY, f64::NEG_INFINITY);
			for d in 0..8 {
				let rn = r as isize + DY[d];
				let cn = c as isize + DX[d];
				if !in_bounds(rn, cn, rows, cols) {
					continue;
				}
				let ni = idx(rn as usize, cn as usize, cols);
				if stream_trib[ni] <= 0 {
					continue;
				}
				let zn = conditioned_raster.get(0, rn, cn);
				if zn == dem.nodata {
					continue;
				}
				let same_trib = stream_trib[ni] == stream_trib[i];
				let lower = zn < z0;
				let rank = (same_trib, lower, stream_tucl[ni], -zn);
				if rank > best_rank {
					best_rank = rank;
					best_dir = d as i8;
				}
			}
			if best_dir >= 0 {
				pointer_data[i] = d8_values[best_dir as usize];
			}
		}

		let mut ptr_raster = vec_to_raster(&conditioned_raster, &pointer_data, DataType::I16);
		ptr_raster.nodata = dem.nodata;
		let ptr_path = write_or_store_output(ptr_raster, out_dir)?;

		let mut fa_args = ToolArgs::new();
		fa_args.insert("input".to_string(), json!(ptr_path.clone()));
		fa_args.insert("input_is_pointer".to_string(), json!(true));
		fa_args.insert("out_type".to_string(), json!("sca"));
		if let Some(path) = out_fa {
			fa_args.insert("output".to_string(), json!(path.to_string_lossy().to_string()));
		}
		let d8_fa_tool = D8FlowAccumTool;
		let fa_result = d8_fa_tool.run(&fa_args, ctx)?;
		let fa_path = fa_result
			.outputs
			.get("path")
			.and_then(|v| v.as_str())
			.ok_or_else(|| ToolError::Execution("d8_flow_accum did not return an output path".to_string()))?
			.to_string();

		Ok(build_quad_raster_result(
			"streams",
			stream_path,
			"burned_dem",
			conditioned_path,
			"flow_dir",
			ptr_path,
			"flow_accum",
			fa_path,
		))
	}
}

impl Tool for StochasticDepressionAnalysisTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "stochastic_depression_analysis",
			display_name: "Stochastic Depression Analysis",
			summary: "Estimates per-cell probability of depression occurrence under a stochastic DEM error model.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
				ToolParamSpec { name: "rmse", description: "DEM elevation RMSE", required: true },
				ToolParamSpec { name: "range", description: "Error autocorrelation range (map units)", required: true },
				ToolParamSpec { name: "iterations", description: "Number of Monte Carlo iterations", required: false },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		let mut defaults = ToolArgs::new();
		defaults.insert("iterations".to_string(), json!(100));
		ToolManifest {
			id: "stochastic_depression_analysis".to_string(),
			display_name: "Stochastic Depression Analysis".to_string(),
			summary: "Runs Monte Carlo DEM perturbations and estimates depression-membership probability.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "stochastic_depression_analysis".to_string(),
				description: "Compute depression probability from DEM RMSE and autocorrelation range".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "depressions".to_string(), "stochastic".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		let rmse = args
			.get("rmse")
			.and_then(|v| v.as_f64())
			.ok_or_else(|| ToolError::Validation("'rmse' is required".to_string()))?;
		if !(rmse.is_finite() && rmse > 0.0) {
			return Err(ToolError::Validation("'rmse' must be finite and > 0".to_string()));
		}
		let range = args
			.get("range")
			.and_then(|v| v.as_f64())
			.ok_or_else(|| ToolError::Validation("'range' is required".to_string()))?;
		if !(range.is_finite() && range >= 0.0) {
			return Err(ToolError::Validation("'range' must be finite and >= 0".to_string()));
		}
		if let Some(it) = args.get("iterations").and_then(|v| v.as_u64()) {
			if it == 0 {
				return Err(ToolError::Validation("'iterations' must be >= 1".to_string()));
			}
		}
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let rmse = args.get("rmse").and_then(|v| v.as_f64()).unwrap_or(0.0);
		let range = args.get("range").and_then(|v| v.as_f64()).unwrap_or(0.0);
		let iterations = args.get("iterations").and_then(|v| v.as_u64()).unwrap_or(100).max(1) as usize;

		let rows = dem.rows;
		let cols = dem.cols;
		let nodata = dem.nodata;
		let base = raster_to_vec(&dem);

		let cell_len = ((dem.cell_size_x.abs() + dem.cell_size_y.abs()) / 2.0).max(1.0e-12);
		let radius = ((range / cell_len).round() as isize).max(0) as usize;
		let small = auto_small_increment(&dem, None);
		let smooth_radius = radius.min(8);
		let count_dep = (0..iterations)
			.into_par_iter()
			.fold(
				|| vec![0u32; rows * cols],
				|mut local_counts, _| {
					let mut rng = rand::rng();
					let mut perturbed = vec![nodata; rows * cols];
					for i in 0..rows * cols {
						if base[i] == nodata {
							continue;
						}
						perturbed[i] = base[i] + gaussian_noise_box_muller(&mut rng) * rmse;
					}

					if smooth_radius > 0 {
						perturbed = box_mean_filter_valid(&perturbed, rows, cols, nodata, smooth_radius);
					}

					let filled = fill_depressions_wang_and_liu_core(&perturbed, rows, cols, nodata, small);
					for i in 0..rows * cols {
						if perturbed[i] != nodata && filled[i] != nodata && filled[i] > perturbed[i] {
							local_counts[i] += 1;
						}
					}
					local_counts
				},
			)
			.reduce(
				|| vec![0u32; rows * cols],
				|mut a, b| {
					for i in 0..a.len() {
						a[i] += b[i];
					}
					a
				},
			);

		let mut prob = vec![nodata; rows * cols];
		for i in 0..rows * cols {
			if base[i] != nodata {
				prob[i] = count_dep[i] as f64 / iterations as f64;
			}
		}

		let mut out = vec_to_raster(&dem, &prob, DataType::F32);
		out.nodata = nodata;
		Ok(build_result(write_or_store_output(out, output_path)?))
	}
}

impl Tool for UnnestBasinsTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "unnest_basins",
			display_name: "Unnest Basins",
			summary: "Delineates complete nested basins for pour points and returns one raster per nesting order.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "d8_pntr", description: "Input D8 pointer raster", required: true },
				ToolParamSpec { name: "pour_points", description: "Input pour-point vector", required: true },
				ToolParamSpec { name: "esri_pntr", description: "Interpret pointers as ESRI style", required: false },
				ToolParamSpec { name: "output", description: "Optional base output path for numbered rasters", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		let mut defaults = ToolArgs::new();
		defaults.insert("esri_pntr".to_string(), json!(false));
		ToolManifest {
			id: "unnest_basins".to_string(),
			display_name: "Unnest Basins".to_string(),
			summary: "Creates one basin raster per pour-point nesting level from a D8 pointer grid.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "unnest_basins".to_string(),
				description: "Delineate complete nested basins for station outlets".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "watershed".to_string(), "nested".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "d8_pntr")
			.or_else(|_| parse_raster_path_arg(args, "d8_pointer"))
			.or_else(|_| parse_raster_path_arg(args, "input"))?;
		parse_vector_path_arg(args, "pour_points").or_else(|_| parse_vector_path_arg(args, "pour_pts"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (pntr, _unused) = parse_pointer_input(args)?;
		let pour_path = parse_vector_path_arg(args, "pour_points")
			.or_else(|_| parse_vector_path_arg(args, "pour_pts"))?;
		let layer = read_vector_layer_aligned_to_dem(&pntr, &pour_path, "pour_points")?;
		let esri_style = args.get("esri_pntr").and_then(|v| v.as_bool()).unwrap_or(false);
		let output_base = parse_optional_output_path(args, "output")?;

		let rows = pntr.rows;
		let cols = pntr.cols;
		let out_nodata = -32768.0;
		let low_value = f64::MIN;

		let mut flow_dir = vec![-2i8; rows * cols];
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				let z = pntr.get(0, r as isize, c as isize);
				if z == pntr.nodata {
					flow_dir[i] = -2;
				} else if z > 0.0 {
					flow_dir[i] = decode_d8_pointer_dir_checked(z, esri_style)?;
				} else {
					flow_dir[i] = -1;
				}
			}
		}

		let mut outlets: Vec<(usize, isize, isize)> = Vec::new();
		for feat in &layer.features {
			let Some(ref geom) = feat.geometry else { continue };
			let pts: Vec<(f64, f64)> = match geom {
				wbvector::Geometry::Point(c) => vec![(c.x, c.y)],
				wbvector::Geometry::MultiPoint(ps) => ps.iter().map(|p| (p.x, p.y)).collect(),
				_ => Vec::new(),
			};
			for (x, y) in pts {
				if let Some((c, r)) = pntr.world_to_pixel(x, y) {
					if in_bounds(r, c, rows, cols) {
						outlets.push((outlets.len() + 1, r, c));
					}
				}
			}
		}
		if outlets.is_empty() {
			return Err(ToolError::Validation("no valid pour points found on raster".to_string()));
		}

		let mut outlet_at = vec![0usize; rows * cols];
		for (oid, r, c) in &outlets {
			outlet_at[idx(*r as usize, *c as usize, cols)] = *oid;
		}

		let mut nesting_order = vec![0usize; outlets.len() + 1];
		let mut max_order = 1usize;
		for (oid0, r0, c0) in &outlets {
			let mut cur_order = 1usize;
			let mut r = *r0;
			let mut c = *c0;
			if nesting_order[*oid0] < cur_order {
				nesting_order[*oid0] = cur_order;
			}
			let mut guard = 0usize;
			loop {
				if guard > rows * cols {
					break;
				}
				guard += 1;
				let i = idx(r as usize, c as usize, cols);
				let dir = flow_dir[i];
				if dir < 0 {
					break;
				}
				r += DY[dir as usize];
				c += DX[dir as usize];
				if !in_bounds(r, c, rows, cols) {
					break;
				}
				let down_oid = outlet_at[idx(r as usize, c as usize, cols)];
				if down_oid > 0 {
					cur_order += 1;
					if nesting_order[down_oid] < cur_order {
						nesting_order[down_oid] = cur_order;
						max_order = max_order.max(cur_order);
					} else {
						break;
					}
				}
			}
		}

		let mut paths = Vec::<String>::new();
		for order in 1..=max_order {
			let mut out = vec![low_value; rows * cols];
			for i in 0..rows * cols {
				if flow_dir[i] == -2 {
					out[i] = out_nodata;
				}
			}
			for (oid, r, c) in &outlets {
				if nesting_order[*oid] == order {
					out[idx(*r as usize, *c as usize, cols)] = *oid as f64;
				}
			}

			run_watershed_labeling(&mut out, &flow_dir, rows, cols, low_value, out_nodata);

			let mut raster = vec_to_raster(&pntr, &out, DataType::I32);
			raster.nodata = out_nodata;
			let path = if let Some(ref base) = output_base {
				write_or_store_output(raster, Some(make_indexed_output_path(base, order)))?
			} else {
				write_or_store_output(raster, None)?
			};
			paths.push(path);
		}

		Ok(build_raster_list_result(paths))
	}
}

impl Tool for UpslopeDepressionStorageTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "upslope_depression_storage",
			display_name: "Upslope Depression Storage",
			summary: "Estimates average upslope depression-storage depth from a DEM.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		ToolManifest {
			id: "upslope_depression_storage".to_string(),
			display_name: "Upslope Depression Storage".to_string(),
			summary: "Maps mean upslope depression-storage depth by routing depression depth over a conditioned DEM.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults: ToolArgs::new(),
			examples: vec![ToolExample {
				name: "upslope_depression_storage".to_string(),
				description: "Estimate roughness-related upslope depression storage depth".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "depressions".to_string(), "storage".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let rows = dem.rows;
		let cols = dem.cols;
		let nodata = dem.nodata;

		let base = raster_to_vec(&dem);
		let small = auto_small_increment(&dem, None);
		let filled = fill_depressions_wang_and_liu_core(&base, rows, cols, nodata, small);

		let mut dep_depth = vec![0.0f64; rows * cols];
		for i in 0..rows * cols {
			if base[i] == nodata || filled[i] == nodata {
				dep_depth[i] = nodata;
			} else {
				dep_depth[i] = (filled[i] - base[i]).max(0.0);
			}
		}

		let filled_raster = vec_to_raster(&dem, &filled, DataType::F32);
		let dirs = d8_dir_from_dem_local(&filled_raster);
		let inflowing_vals: [i8; 8] = [4, 5, 6, 7, 0, 1, 2, 3];

		let mut inflow = vec![-1i32; rows * cols];
		let mut ups_sum = vec![0.0f64; rows * cols];
		let mut ups_count = vec![0.0f64; rows * cols];
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				if dep_depth[i] == nodata {
					continue;
				}
				let mut count = 0i32;
				for k in 0..8 {
					let rn = r as isize + DY[k];
					let cn = c as isize + DX[k];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					if dep_depth[ni] != nodata && dirs[ni] == inflowing_vals[k] {
						count += 1;
					}
				}
				inflow[i] = count;
				ups_sum[i] = dep_depth[i];
				ups_count[i] = 1.0;
			}
		}

		let mut stack = Vec::<usize>::with_capacity(rows * cols);
		for i in 0..rows * cols {
			if inflow[i] == 0 {
				stack.push(i);
			}
		}

		while let Some(i) = stack.pop() {
			let dir = dirs[i];
			if dir < 0 {
				continue;
			}
			let r = i / cols;
			let c = i % cols;
			let rn = r as isize + DY[dir as usize];
			let cn = c as isize + DX[dir as usize];
			if !in_bounds(rn, cn, rows, cols) {
				continue;
			}
			let ni = idx(rn as usize, cn as usize, cols);
			if dep_depth[ni] == nodata {
				continue;
			}
			ups_sum[ni] += ups_sum[i];
			ups_count[ni] += ups_count[i];
			inflow[ni] -= 1;
			if inflow[ni] == 0 {
				stack.push(ni);
			}
		}

		let mut out = vec![nodata; rows * cols];
		for i in 0..rows * cols {
			if dep_depth[i] != nodata && ups_count[i] > 0.0 {
				out[i] = ups_sum[i] / ups_count[i];
			}
		}

		let mut raster = vec_to_raster(&dem, &out, DataType::F32);
		raster.nodata = nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

impl Tool for FlattenLakesTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "flatten_lakes",
			display_name: "Flatten Lakes",
			summary: "Sets lake-polygon interior elevations to each lake's minimum perimeter elevation.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
				ToolParamSpec { name: "lakes", description: "Input lake polygon vector", required: true },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		ToolManifest {
			id: "flatten_lakes".to_string(),
			display_name: "Flatten Lakes".to_string(),
			summary: "Flattens lake elevations using minimum perimeter elevation for each polygon.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults: ToolArgs::new(),
			examples: vec![ToolExample {
				name: "flatten_lakes".to_string(),
				description: "Flatten waterbody polygons in a DEM".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "dem".to_string(), "lakes".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		parse_vector_path_arg(args, "lakes")?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let lakes_path = parse_vector_path_arg(args, "lakes")?;
		let layer = read_vector_layer_aligned_to_dem(&dem, &lakes_path, "lakes")?;

		let rows = dem.rows;
		let cols = dem.cols;
		let nodata = dem.nodata;
		let mut out = raster_to_vec(&dem);

		for feat in &layer.features {
			let Some(ref geom) = feat.geometry else { continue };
			let polys: Vec<(wbvector::Ring, Vec<wbvector::Ring>)> = match geom {
				wbvector::Geometry::Polygon { exterior, interiors } => vec![(exterior.clone(), interiors.clone())],
				wbvector::Geometry::MultiPolygon(ps) => ps.clone(),
				_ => Vec::new(),
			};

			for (exterior, interiors) in polys {
				if exterior.0.len() < 3 {
					continue;
				}
				let mut perim = vec![0u8; rows * cols];
				rasterize_polygon_boundaries(&mut perim, rows, cols, &dem, &wbvector::Geometry::Polygon {
					exterior: exterior.clone(),
					interiors: interiors.clone(),
				});
				let mut min_elev = f64::INFINITY;
				for i in 0..rows * cols {
					if perim[i] > 0 && out[i] != nodata && out[i] < min_elev {
						min_elev = out[i];
					}
				}
				if !min_elev.is_finite() {
					continue;
				}

				let Some((rmin, cmin, rmax, cmax)) = polygon_bbox_pixels(&dem, &exterior) else {
					continue;
				};
				for r in rmin..=rmax {
					for c in cmin..=cmax {
						let i = idx(r, c, cols);
						if out[i] == nodata {
							continue;
						}
						let x = dem.col_center_x(c as isize);
						let y = dem.row_center_y(r as isize);
						if polygon_contains_xy(&exterior, &interiors, x, y) {
							out[i] = min_elev;
						}
					}
				}
			}
		}

		let mut raster = vec_to_raster(&dem, &out, DataType::F32);
		raster.nodata = nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

impl Tool for HydrologicConnectivityTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "hydrologic_connectivity",
			display_name: "Hydrologic Connectivity",
			summary: "Calculates downslope unsaturated length (DUL) and upslope disconnected saturated area (UDSA).",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
				ToolParamSpec { name: "exponent", description: "Dispersion exponent (accepted for compatibility)", required: false },
				ToolParamSpec { name: "convergence_threshold", description: "Threshold area for stream initiation in cells", required: false },
				ToolParamSpec { name: "z_factor", description: "Vertical scaling factor", required: false },
				ToolParamSpec { name: "output1", description: "Optional output path for DUL raster", required: false },
				ToolParamSpec { name: "output2", description: "Optional output path for UDSA raster", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		let mut defaults = ToolArgs::new();
		defaults.insert("exponent".to_string(), json!(1.1));
		defaults.insert("convergence_threshold".to_string(), json!(0.0));
		defaults.insert("z_factor".to_string(), json!(1.0));
		ToolManifest {
			id: "hydrologic_connectivity".to_string(),
			display_name: "Hydrologic Connectivity".to_string(),
			summary: "Computes DUL and UDSA connectivity indices from a DEM.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "hydrologic_connectivity".to_string(),
				description: "Calculate DUL and UDSA index rasters".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "connectivity".to_string(), "wetness".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let dem_path = parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		let dem = load_raster(&dem_path)?;
		let out1_path = parse_optional_output_path(args, "output1")?;
		let out2_path = parse_optional_output_path(args, "output2")?;
		let convergence_threshold = args
			.get("convergence_threshold")
			.and_then(|v| v.as_f64())
			.unwrap_or(0.0);
		let mut z_factor = args.get("z_factor").and_then(|v| v.as_f64()).unwrap_or(1.0);
		if !z_factor.is_finite() || z_factor <= 0.0 {
			z_factor = 1.0;
		}

		let rows = dem.rows;
		let cols = dem.cols;
		let nodata = dem.nodata;
		let cell_area = dem.cell_size_x * dem.cell_size_y;
		let cell_len = ((dem.cell_size_x.abs() + dem.cell_size_y.abs()) / 2.0).max(1.0e-12);

		let dirs = d8_dir_from_dem_local(&dem);
		let inflowing_vals: [i8; 8] = [4, 5, 6, 7, 0, 1, 2, 3];

		let mut inflow = vec![-1i32; rows * cols];
		let mut accum = vec![0.0f64; rows * cols];
		let mut topo = Vec::<usize>::with_capacity(rows * cols);
		let mut stack = Vec::<usize>::with_capacity(rows * cols);

		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				let z = dem.get(0, r as isize, c as isize);
				if z == nodata {
					continue;
				}
				accum[i] = 1.0;
				let mut count = 0i32;
				for k in 0..8 {
					let rn = r as isize + DY[k];
					let cn = c as isize + DX[k];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					if dirs[ni] == inflowing_vals[k] {
						count += 1;
					}
				}
				inflow[i] = count;
				if count == 0 {
					stack.push(i);
				}
			}
		}

		while let Some(i) = stack.pop() {
			topo.push(i);
			let dir = dirs[i];
			if dir < 0 {
				continue;
			}
			let r = i / cols;
			let c = i % cols;
			let rn = r as isize + DY[dir as usize];
			let cn = c as isize + DX[dir as usize];
			if !in_bounds(rn, cn, rows, cols) {
				continue;
			}
			let ni = idx(rn as usize, cn as usize, cols);
			if inflow[ni] >= 0 {
				accum[ni] += accum[i];
				inflow[ni] -= 1;
				if inflow[ni] == 0 {
					stack.push(ni);
				}
			}
		}

		let mut wi = vec![nodata; rows * cols];
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				let z = dem.get(0, r as isize, c as isize);
				if z == nodata {
					continue;
				}
				let dir = dirs[i];
				let slope = if dir >= 0 {
					let rn = r as isize + DY[dir as usize];
					let cn = c as isize + DX[dir as usize];
					if in_bounds(rn, cn, rows, cols) {
						let zn = dem.get(0, rn, cn);
						if zn != nodata {
							((z * z_factor - zn * z_factor).max(0.0) / cell_len).max(1.0e-6)
						} else {
							1.0e-6
						}
					} else {
						1.0e-6
					}
				} else {
					1.0e-6
				};
				let sca = (accum[i] * cell_area).max(1.0e-6);
				wi[i] = (sca / slope.max(1.0e-6)).ln();
			}
		}

		let mut conn_dir = vec![-1i8; rows * cols];
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				let z = dem.get(0, r as isize, c as isize);
				if z == nodata {
					continue;
				}
				if convergence_threshold > 0.0 && accum[i] >= convergence_threshold {
					conn_dir[i] = -1;
					continue;
				}
				let mut best = -1i8;
				let mut best_wi = f64::NEG_INFINITY;
				for n in 0..8 {
					let rn = r as isize + DY[n];
					let cn = c as isize + DX[n];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					let zn = dem.get(0, rn, cn);
					if zn == nodata || zn >= z {
						continue;
					}
					if wi[ni] > best_wi {
						best_wi = wi[ni];
						best = n as i8;
					}
				}
				conn_dir[i] = best;
			}
		}

		let mut net_min = wi.clone();
		for &i in topo.iter().rev() {
			let dir = conn_dir[i];
			if dir < 0 {
				continue;
			}
			let r = i / cols;
			let c = i % cols;
			let rn = r as isize + DY[dir as usize];
			let cn = c as isize + DX[dir as usize];
			if !in_bounds(rn, cn, rows, cols) {
				continue;
			}
			let ni = idx(rn as usize, cn as usize, cols);
			net_min[i] = wi[i].min(net_min[ni]);
		}

		let mut dul = vec![nodata; rows * cols];
		for i in 0..rows * cols {
			if wi[i] == nodata {
				continue;
			}
			let wi0 = wi[i];
			let mut count = 0usize;
			let mut cur = i;
			let mut guard = 0usize;
			while guard < rows * cols {
				let dir = conn_dir[cur];
				if dir < 0 {
					break;
				}
				let r = cur / cols;
				let c = cur % cols;
				let rn = r as isize + DY[dir as usize];
				let cn = c as isize + DX[dir as usize];
				if !in_bounds(rn, cn, rows, cols) {
					break;
				}
				cur = idx(rn as usize, cn as usize, cols);
				if wi[cur] < wi0 {
					count += 1;
				}
				guard += 1;
			}
			dul[i] = count as f64 * cell_len;
		}

		let mut inflow2 = vec![-1i32; rows * cols];
		let mut sources = Vec::<usize>::new();
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				if wi[i] == nodata {
					continue;
				}
				let mut count = 0i32;
				for k in 0..8 {
					let rn = r as isize + DY[k];
					let cn = c as isize + DX[k];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					if conn_dir[ni] == inflowing_vals[k] {
						count += 1;
					}
				}
				inflow2[i] = count;
				if count == 0 {
					sources.push(i);
				}
			}
		}

		let mut udsa_counts = vec![0usize; rows * cols];
		for s in sources {
			let ws = wi[s];
			if ws == nodata {
				continue;
			}
			let mut cur = s;
			let mut guard = 0usize;
			loop {
				if ws > wi[cur] {
					udsa_counts[cur] += 1;
				}
				let dir = conn_dir[cur];
				if dir < 0 || guard >= rows * cols {
					break;
				}
				let r = cur / cols;
				let c = cur % cols;
				let rn = r as isize + DY[dir as usize];
				let cn = c as isize + DX[dir as usize];
				if !in_bounds(rn, cn, rows, cols) {
					break;
				}
				cur = idx(rn as usize, cn as usize, cols);
				guard += 1;
			}
		}

		let mut udsa = vec![nodata; rows * cols];
		for i in 0..rows * cols {
			if wi[i] != nodata {
				udsa[i] = udsa_counts[i] as f64 * cell_area;
			}
		}

		let mut dul_raster = vec_to_raster(&dem, &dul, DataType::F32);
		dul_raster.nodata = nodata;
		let mut udsa_raster = vec_to_raster(&dem, &udsa, DataType::F32);
		udsa_raster.nodata = nodata;

		let p1 = write_or_store_output(dul_raster, out1_path)?;
		let p2 = write_or_store_output(udsa_raster, out2_path)?;
		Ok(build_pair_raster_result("dul", p1, "udsa", p2))
	}
}

impl Tool for ImpoundmentSizeIndexTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "impoundment_size_index",
			display_name: "Impoundment Size Index",
			summary: "Estimates impoundment metrics for potential dams of a maximum length at each DEM cell.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
				ToolParamSpec { name: "max_dam_length", description: "Maximum dam length (map units)", required: true },
				ToolParamSpec { name: "output_mean", description: "Include mean flooded depth output", required: false },
				ToolParamSpec { name: "output_max", description: "Include max flooded depth output", required: false },
				ToolParamSpec { name: "output_volume", description: "Include flooded volume output", required: false },
				ToolParamSpec { name: "output_area", description: "Include flooded area output", required: false },
				ToolParamSpec { name: "output_height", description: "Include dam-height output", required: false },
				ToolParamSpec { name: "out_mean", description: "Optional output path for mean depth raster", required: false },
				ToolParamSpec { name: "out_max", description: "Optional output path for max depth raster", required: false },
				ToolParamSpec { name: "out_volume", description: "Optional output path for volume raster", required: false },
				ToolParamSpec { name: "out_area", description: "Optional output path for area raster", required: false },
				ToolParamSpec { name: "out_dam_height", description: "Optional output path for dam-height raster", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		let mut defaults = ToolArgs::new();
		defaults.insert("output_mean".to_string(), json!(false));
		defaults.insert("output_max".to_string(), json!(false));
		defaults.insert("output_volume".to_string(), json!(false));
		defaults.insert("output_area".to_string(), json!(false));
		defaults.insert("output_height".to_string(), json!(false));
		ToolManifest {
			id: "impoundment_size_index".to_string(),
			display_name: "Impoundment Size Index".to_string(),
			summary: "Computes mean/max depth, volume, area, and dam-height impoundment metrics.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "impoundment_index".to_string(),
				description: "Estimate potential impoundment metrics for a chosen maximum dam length".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "impoundment".to_string(), "dam".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		let dam_length = args
			.get("max_dam_length")
			.or_else(|| args.get("damlength"))
			.and_then(|v| v.as_f64())
			.ok_or_else(|| ToolError::Validation("'max_dam_length' is required".to_string()))?;
		if dam_length <= 0.0 {
			return Err(ToolError::Validation("'max_dam_length' must be > 0".to_string()));
		}
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let dem_path = parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		let dem = load_raster(&dem_path)?;

		let dam_length = args
			.get("max_dam_length")
			.or_else(|| args.get("damlength"))
			.and_then(|v| v.as_f64())
			.unwrap_or(0.0);

		let mut include_mean = args.get("output_mean").and_then(|v| v.as_bool()).unwrap_or(false);
		let mut include_max = args.get("output_max").and_then(|v| v.as_bool()).unwrap_or(false);
		let mut include_volume = args.get("output_volume").and_then(|v| v.as_bool()).unwrap_or(false);
		let mut include_area = args.get("output_area").and_then(|v| v.as_bool()).unwrap_or(false);
		let mut include_height = args.get("output_height").and_then(|v| v.as_bool()).unwrap_or(false);

		let out_mean_path = parse_optional_output_path(args, "out_mean")?;
		let out_max_path = parse_optional_output_path(args, "out_max")?;
		let out_volume_path = parse_optional_output_path(args, "out_volume")?;
		let out_area_path = parse_optional_output_path(args, "out_area")?;
		let out_height_path = parse_optional_output_path(args, "out_dam_height")?;

		if out_mean_path.is_some() {
			include_mean = true;
		}
		if out_max_path.is_some() {
			include_max = true;
		}
		if out_volume_path.is_some() {
			include_volume = true;
		}
		if out_area_path.is_some() {
			include_area = true;
		}
		if out_height_path.is_some() {
			include_height = true;
		}

		if !(include_mean || include_max || include_volume || include_area || include_height) {
			return Err(ToolError::Validation(
				"at least one output must be requested via output_* flags or output paths".to_string(),
			));
		}

		let rows = dem.rows;
		let cols = dem.cols;
		let nodata = dem.nodata;
		let grid_area = dem.cell_size_x * dem.cell_size_y;
		let cell_len = ((dem.cell_size_x.abs() + dem.cell_size_y.abs()) / 2.0).max(1.0e-12);
		let radius = ((dam_length / (2.0 * cell_len)).ceil() as isize).max(1);

		let mut out_mean = vec![nodata; rows * cols];
		let mut out_max = vec![nodata; rows * cols];
		let mut out_volume = vec![nodata; rows * cols];
		let mut out_area = vec![nodata; rows * cols];
		let mut out_height = vec![nodata; rows * cols];

		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				let z0 = dem.get(0, r as isize, c as isize);
				if z0 == nodata {
					continue;
				}

				let mut crest = z0;
				for rr in (r as isize - radius)..=(r as isize + radius) {
					for cc in (c as isize - radius)..=(c as isize + radius) {
						if !in_bounds(rr, cc, rows, cols) {
							continue;
						}
						let z = dem.get(0, rr, cc);
						if z != nodata && z > crest {
							crest = z;
						}
					}
				}
				let dam_h = (crest - z0).max(0.0);

				let mut flooded_cells = 0.0f64;
				let mut flooded_vol = 0.0f64;
				let mut max_depth = 0.0f64;
				for rr in (r as isize - radius)..=(r as isize + radius) {
					for cc in (c as isize - radius)..=(c as isize + radius) {
						if !in_bounds(rr, cc, rows, cols) {
							continue;
						}
						let z = dem.get(0, rr, cc);
						if z == nodata || z >= crest {
							continue;
						}
						let d = crest - z;
						flooded_cells += 1.0;
						flooded_vol += d * grid_area;
						if d > max_depth {
							max_depth = d;
						}
					}
				}

				let area = flooded_cells * grid_area;
				let mean = if area > 0.0 { flooded_vol / area } else { 0.0 };

				out_height[i] = dam_h;
				out_area[i] = area;
				out_volume[i] = flooded_vol;
				out_max[i] = max_depth;
				out_mean[i] = mean;
			}
		}

		let mut outputs = BTreeMap::new();
		let mut items = Vec::<serde_json::Value>::new();

		if include_mean {
			let mut r = vec_to_raster(&dem, &out_mean, DataType::F32);
			r.nodata = nodata;
			let p = write_or_store_output(r, out_mean_path)?;
			let t = typed_raster_output(p);
			outputs.insert("mean".to_string(), t.clone());
			items.push(t);
		} else {
			items.push(json!(null));
		}
		if include_max {
			let mut r = vec_to_raster(&dem, &out_max, DataType::F32);
			r.nodata = nodata;
			let p = write_or_store_output(r, out_max_path)?;
			let t = typed_raster_output(p);
			outputs.insert("max".to_string(), t.clone());
			items.push(t);
		} else {
			items.push(json!(null));
		}
		if include_volume {
			let mut r = vec_to_raster(&dem, &out_volume, DataType::F32);
			r.nodata = nodata;
			let p = write_or_store_output(r, out_volume_path)?;
			let t = typed_raster_output(p);
			outputs.insert("volume".to_string(), t.clone());
			items.push(t);
		} else {
			items.push(json!(null));
		}
		if include_area {
			let mut r = vec_to_raster(&dem, &out_area, DataType::F32);
			r.nodata = nodata;
			let p = write_or_store_output(r, out_area_path)?;
			let t = typed_raster_output(p);
			outputs.insert("area".to_string(), t.clone());
			items.push(t);
		} else {
			items.push(json!(null));
		}
		if include_height {
			let mut r = vec_to_raster(&dem, &out_height, DataType::F32);
			r.nodata = nodata;
			let p = write_or_store_output(r, out_height_path)?;
			let t = typed_raster_output(p);
			outputs.insert("dam_height".to_string(), t.clone());
			items.push(t);
		} else {
			items.push(json!(null));
		}

		outputs.insert("__wbw_type__".to_string(), json!("tuple"));
		outputs.insert("items".to_string(), json!(items));
		Ok(ToolRunResult {
			outputs,
			..Default::default()
		})
	}
}

impl Tool for AverageFlowpathSlopeTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "average_flowpath_slope",
			display_name: "Average Flowpath Slope",
			summary: "Calculates average slope gradient of flowpaths passing through each DEM cell.",
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
			id: "average_flowpath_slope".to_string(),
			display_name: "Average Flowpath Slope".to_string(),
			summary: "Calculates average slope gradient of flowpaths passing through each DEM cell.".to_string(),
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults: ToolArgs::new(),
			examples: vec![ToolExample {
				name: "avg_flowpath_slope".to_string(),
				description: "Compute average flowpath slope over a conditioned DEM".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "slope".to_string(), "flowpath".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let rows = dem.rows;
		let cols = dem.cols;
		let nodata = dem.nodata;

		let dirs = d8_dir_from_dem_local(&dem);
		let inflowing_vals: [i8; 8] = [4, 5, 6, 7, 0, 1, 2, 3];
		let mut inflow = vec![-1i32; rows * cols];
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				if dem.get(0, r as isize, c as isize) == nodata {
					continue;
				}
				let mut count = 0i32;
				for k in 0..8 {
					let rn = r as isize + DY[k];
					let cn = c as isize + DX[k];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					if dirs[ni] == inflowing_vals[k] {
						count += 1;
					}
				}
				inflow[i] = count;
			}
		}

		let cell_x = dem.cell_size_x;
		let cell_y = dem.cell_size_y;
		let diag = (cell_x * cell_x + cell_y * cell_y).sqrt();
		let lengths = [diag, cell_x, diag, cell_y, diag, cell_x, diag, cell_y];

		let mut path_count = vec![0i64; rows * cols];
		let mut total_len = vec![0.0f64; rows * cols];
		let mut total_div_elev = vec![0.0f64; rows * cols];
		let mut stack = Vec::<usize>::with_capacity(rows * cols);
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				if inflow[i] == 0 {
					stack.push(i);
					path_count[i] = 1;
					total_len[i] = 0.0;
					total_div_elev[i] = dem.get(0, r as isize, c as isize);
				}
			}
		}

		while let Some(i) = stack.pop() {
			let dir = dirs[i];
			if dir >= 0 {
				let r = i / cols;
				let c = i % cols;
				let rn = r as isize + DY[dir as usize];
				let cn = c as isize + DX[dir as usize];
				if in_bounds(rn, cn, rows, cols) {
					let ni = idx(rn as usize, cn as usize, cols);
					total_len[ni] += total_len[i] + (path_count[i] as f64) * lengths[dir as usize];
					total_div_elev[ni] += total_div_elev[i];
					path_count[ni] += path_count[i];
					if inflow[ni] > 0 {
						inflow[ni] -= 1;
						if inflow[ni] == 0 {
							stack.push(ni);
						}
					}
				}
			}
		}

		let mut out = vec![nodata; rows * cols];
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				let z = dem.get(0, r as isize, c as isize);
				if z == nodata || path_count[i] == 0 {
					continue;
				}
				let mean_len = total_len[i] / path_count[i] as f64;
				if mean_len <= 0.0 {
					out[i] = 0.0;
					continue;
				}
				let mean_div_elev = total_div_elev[i] / path_count[i] as f64;
				let z_diff = mean_div_elev - z;
				out[i] = (z_diff / mean_len).atan().to_degrees();
			}
		}

		let mut raster = vec_to_raster(&dem, &out, DataType::F32);
		raster.nodata = nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

impl Tool for MaxUpslopeValueTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "max_upslope_value",
			display_name: "Max Upslope Value",
			summary: "Propagates maximum upslope value along D8 flowpaths over a DEM.",
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
				ToolParamSpec { name: "values", description: "Input values raster", required: true },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		ToolManifest {
			id: "max_upslope_value".to_string(),
			display_name: "Max Upslope Value".to_string(),
			summary: "Propagates maximum upslope value along D8 flowpaths over a DEM.".to_string(),
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults: ToolArgs::new(),
			examples: vec![ToolExample {
				name: "max_upslope_val".to_string(),
				description: "Map max upslope source value along D8 flowpaths".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "upslope".to_string(), "d8".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		parse_raster_path_arg(args, "values")
			.or_else(|_| parse_raster_path_arg(args, "values_raster"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, output_path) = parse_dem_and_output(args)?;
		let values_path = parse_raster_path_arg(args, "values")
			.or_else(|_| parse_raster_path_arg(args, "values_raster"))?;
		let values = load_raster(&values_path)?;
		if values.rows != dem.rows || values.cols != dem.cols {
			return Err(ToolError::Validation(
				"values raster must match DEM dimensions".to_string(),
			));
		}

		let rows = dem.rows;
		let cols = dem.cols;
		let out_nodata = -32768.0;
		let dirs = d8_dir_from_dem_local(&dem);
		let inflowing_vals: [i8; 8] = [4, 5, 6, 7, 0, 1, 2, 3];

		let mut out = vec![out_nodata; rows * cols];
		let mut inflow = vec![-1i32; rows * cols];
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				let z = dem.get(0, r as isize, c as isize);
				let v = values.get(0, r as isize, c as isize);
				if z == dem.nodata || v == values.nodata {
					continue;
				}
				out[i] = v;
				let mut count = 0i32;
				for k in 0..8 {
					let rn = r as isize + DY[k];
					let cn = c as isize + DX[k];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					if dirs[ni] == inflowing_vals[k] {
						let zn = dem.get(0, rn, cn);
						let vn = values.get(0, rn, cn);
						if zn != dem.nodata && vn != values.nodata {
							count += 1;
						}
					}
				}
				inflow[i] = count;
			}
		}

		let mut stack = Vec::<usize>::with_capacity(rows * cols);
		for i in 0..rows * cols {
			if inflow[i] == 0 {
				stack.push(i);
			}
		}

		while let Some(i) = stack.pop() {
			let dir = dirs[i];
			if dir >= 0 {
				let r = i / cols;
				let c = i % cols;
				let rn = r as isize + DY[dir as usize];
				let cn = c as isize + DX[dir as usize];
				if in_bounds(rn, cn, rows, cols) {
					let ni = idx(rn as usize, cn as usize, cols);
					if inflow[ni] >= 0 {
						if out[i] > out[ni] {
							out[ni] = out[i];
						}
						inflow[ni] -= 1;
						if inflow[ni] == 0 {
							stack.push(ni);
						}
					}
				}
			}
		}

		let mut raster = vec_to_raster(&dem, &out, DataType::F32);
		raster.nodata = out_nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

impl Tool for LongestFlowpathTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "longest_flowpath",
			display_name: "Longest Flowpath",
			summary: "Delineates longest flowpath lines for each basin in a basin raster.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "dem", description: "Input DEM raster", required: true },
				ToolParamSpec { name: "basins", description: "Input basin raster", required: true },
				ToolParamSpec { name: "output", description: "Output vector path", required: true },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		ToolManifest {
			id: "longest_flowpath".to_string(),
			display_name: "Longest Flowpath".to_string(),
			summary: "Delineates longest flowpath lines for each basin in a basin raster.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults: ToolArgs::new(),
			examples: vec![ToolExample {
				name: "longest_flowpaths".to_string(),
				description: "Create one longest flowpath line per basin".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "basins".to_string(), "flowpath".to_string(), "vector".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem")
			.or_else(|_| parse_raster_path_arg(args, "input"))
			.or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
		parse_raster_path_arg(args, "basins")
			.or_else(|_| parse_raster_path_arg(args, "watersheds"))?;
		args.get("output")
			.and_then(|v| v.as_str())
			.filter(|s| !s.is_empty())
			.ok_or_else(|| ToolError::Validation("'output' path is required".into()))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (dem, _unused_output_path) = parse_dem_and_output(args)?;
		let basins_path = parse_raster_path_arg(args, "basins")
			.or_else(|_| parse_raster_path_arg(args, "watersheds"))?;
		let basins = load_raster(&basins_path)?;
		let output = args
			.get("output")
			.and_then(|v| v.as_str())
			.ok_or_else(|| ToolError::Validation("'output' path is required".into()))?
			.to_string();

		if dem.rows != basins.rows || dem.cols != basins.cols {
			return Err(ToolError::Validation(
				"DEM and basins rasters must have the same dimensions".to_string(),
			));
		}

		let rows = dem.rows;
		let cols = dem.cols;
		let nodata = dem.nodata;
		let basin_nodata = basins.nodata;
		let cell_x = dem.cell_size_x;
		let cell_y = dem.cell_size_y;
		let diag = (cell_x * cell_x + cell_y * cell_y).sqrt();
		let lengths = [diag, cell_x, diag, cell_y, diag, cell_x, diag, cell_y];
		let inflowing_vals: [i8; 8] = [4, 5, 6, 7, 0, 1, 2, 3];

		let mut valid = vec![false; rows * cols];
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				let z = dem.get(0, r as isize, c as isize);
				let b = basins.get(0, r as isize, c as isize);
				valid[i] = z != nodata && b != basin_nodata && b != 0.0;
			}
		}

		let mut dirs = d8_dir_from_dem_local(&dem);
		for i in 0..rows * cols {
			if !valid[i] {
				dirs[i] = -1;
			}
		}

		let mut inflow = vec![-1i32; rows * cols];
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				if !valid[i] {
					continue;
				}
				let bi = basins.get(0, r as isize, c as isize);
				let mut count = 0i32;
				for k in 0..8 {
					let rn = r as isize + DY[k];
					let cn = c as isize + DX[k];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					if !valid[ni] {
						continue;
					}
					if dirs[ni] == inflowing_vals[k] {
						let bn = basins.get(0, rn, cn);
						if (bn - bi).abs() < f64::EPSILON {
							count += 1;
						}
					}
				}
				inflow[i] = count;
			}
		}

		let mut lfp = vec![0.0f64; rows * cols];
		let mut source = vec![usize::MAX; rows * cols];
		let mut endpoints = Vec::<usize>::new();
		let mut stack = Vec::<usize>::with_capacity(rows * cols);
		for i in 0..rows * cols {
			if inflow[i] == 0 {
				stack.push(i);
				source[i] = i;
			}
		}

		while let Some(i) = stack.pop() {
			let dir = dirs[i];
			let r = i / cols;
			let c = i % cols;
			let basin_id = basins.get(0, r as isize, c as isize);
			if dir >= 0 {
				let rn = r as isize + DY[dir as usize];
				let cn = c as isize + DX[dir as usize];
				if in_bounds(rn, cn, rows, cols) {
					let ni = idx(rn as usize, cn as usize, cols);
					if valid[ni] {
						let basin_dn = basins.get(0, rn, cn);
						if (basin_dn - basin_id).abs() < f64::EPSILON {
							let candidate = lfp[i] + lengths[dir as usize];
							if candidate > lfp[ni] {
								lfp[ni] = candidate;
								source[ni] = source[i];
							}
							inflow[ni] -= 1;
							if inflow[ni] == 0 {
								stack.push(ni);
							}
						} else {
							endpoints.push(i);
						}
					} else {
						endpoints.push(i);
					}
				} else {
					endpoints.push(i);
				}
			} else {
				endpoints.push(i);
			}
		}

		let mut best_by_basin: BTreeMap<i64, usize> = BTreeMap::new();
		for i in endpoints {
			if !valid[i] {
				continue;
			}
			let r = i / cols;
			let c = i % cols;
			let basin_val = basins.get(0, r as isize, c as isize);
			let key = basin_val.round() as i64;
			match best_by_basin.get(&key).copied() {
				Some(prev) => {
					if lfp[i] > lfp[prev] {
						best_by_basin.insert(key, i);
					}
				}
				None => {
					best_by_basin.insert(key, i);
				}
			}
		}

		let mut out_layer = wbvector::Layer::new("longest_flowpath")
			.with_geom_type(wbvector::GeometryType::LineString);
		out_layer.add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));
		out_layer.add_field(wbvector::FieldDef::new("BASIN", wbvector::FieldType::Float));
		out_layer.add_field(wbvector::FieldDef::new("UP_ELEV", wbvector::FieldType::Float));
		out_layer.add_field(wbvector::FieldDef::new("DN_ELEV", wbvector::FieldType::Float));
		out_layer.add_field(wbvector::FieldDef::new("LENGTH", wbvector::FieldType::Float));
		out_layer.add_field(wbvector::FieldDef::new("AVG_SLOPE", wbvector::FieldType::Float));

		let mut fid = 1i64;
		for (_key, end_idx) in best_by_basin {
			let src_idx = source[end_idx];
			if src_idx == usize::MAX {
				continue;
			}
			let er = end_idx / cols;
			let ec = end_idx % cols;
			let basin_val = basins.get(0, er as isize, ec as isize);
			let source_z = dem.get(0, (src_idx / cols) as isize, (src_idx % cols) as isize);
			let end_z = dem.get(0, er as isize, ec as isize);
			let length = lfp[end_idx];
			let slope = if length > 0.0 { 100.0 * (source_z - end_z) / length } else { 0.0 };

			let mut pts = Vec::<wbvector::Coord>::new();
			let mut cur = src_idx;
			let mut safety = 0usize;
			while safety < rows * cols {
				safety += 1;
				let r = cur / cols;
				let c = cur % cols;
				if !valid[cur] {
					break;
				}
				let b = basins.get(0, r as isize, c as isize);
				if (b - basin_val).abs() >= f64::EPSILON {
					break;
				}
				pts.push(wbvector::Coord::xy(dem.col_center_x(c as isize), dem.row_center_y(r as isize)));
				if cur == end_idx {
					break;
				}
				let dir = dirs[cur];
				if dir < 0 {
					break;
				}
				let rn = r as isize + DY[dir as usize];
				let cn = c as isize + DX[dir as usize];
				if !in_bounds(rn, cn, rows, cols) {
					break;
				}
				cur = idx(rn as usize, cn as usize, cols);
			}

			if pts.len() < 2 {
				continue;
			}

			out_layer
				.add_feature(
					Some(wbvector::Geometry::line_string(pts)),
					&[
						("FID", wbvector::FieldValue::Integer(fid)),
						("BASIN", wbvector::FieldValue::Float(basin_val)),
						("UP_ELEV", wbvector::FieldValue::Float(source_z)),
						("DN_ELEV", wbvector::FieldValue::Float(end_z)),
						("LENGTH", wbvector::FieldValue::Float(length)),
						("AVG_SLOPE", wbvector::FieldValue::Float(slope)),
					],
				)
				.map_err(|e| ToolError::Execution(format!("failed building longest-flowpath output: {}", e)))?;
			fid += 1;
		}

		if let Some(parent) = std::path::Path::new(&output).parent() {
			if !parent.as_os_str().is_empty() {
				std::fs::create_dir_all(parent)
					.map_err(|e| ToolError::Execution(format!("cannot create output directory: {}", e)))?;
			}
		}
		let fmt = wbvector::VectorFormat::detect(&output).unwrap_or(wbvector::VectorFormat::GeoJson);
		wbvector::write(&out_layer, &output, fmt)
			.map_err(|e| ToolError::Execution(format!("failed writing output vector: {}", e)))?;

		Ok(build_result(output))
	}
}

impl Tool for BasinsTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "basins",
			display_name: "Basins",
			summary: "Delineates all D8 drainage basins that drain to valid-data edges.",
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "d8_pntr", description: "Input D8 pointer raster", required: true },
				ToolParamSpec { name: "esri_pntr", description: "Use ESRI pointer encoding", required: false },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		let mut defaults = ToolArgs::new();
		defaults.insert("esri_pntr".to_string(), json!(false));
		ToolManifest {
			id: "basins".to_string(),
			display_name: "Basins".to_string(),
			summary: "Delineates all D8 drainage basins that drain to valid-data edges.".to_string(),
			category: ToolCategory::Raster,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "edge_basins".to_string(),
				description: "Delineate all basins from a D8 pointer raster".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "watershed".to_string(), "d8".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "d8_pntr")
			.or_else(|_| parse_raster_path_arg(args, "d8_pointer"))
			.or_else(|_| parse_raster_path_arg(args, "input"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let (pntr, output_path) = parse_pointer_input(args)?;
		let esri_style = args
			.get("esri_pntr")
			.or_else(|| args.get("esri_pointer"))
			.and_then(|v| v.as_bool())
			.unwrap_or(false);

		let rows = pntr.rows;
		let cols = pntr.cols;
		let nodata = pntr.nodata;
		let out_nodata = -32768.0;
		let low_value = (i32::MIN + 1) as f64;
		let mut flow_dir = vec![-2i8; rows * cols];
		let mut out = vec![low_value; rows * cols];
		let mut basin_id = 0.0;

		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				let z = pntr.get(0, r as isize, c as isize);
				if z == nodata {
					out[i] = out_nodata;
					continue;
				}
				let dir = decode_d8_pointer_dir(z, esri_style);
				flow_dir[i] = dir;
				if dir < 0 {
					basin_id += 1.0;
					out[i] = basin_id;
				}
			}
		}

		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				if out[i] != low_value {
					continue;
				}
				let (mut y, mut x) = (r as isize, c as isize);
				let mut outlet_id = out_nodata;
				loop {
					let ii = idx(y as usize, x as usize, cols);
					let dir = flow_dir[ii];
					if dir >= 0 {
						y += DY[dir as usize];
						x += DX[dir as usize];
						if !in_bounds(y, x, rows, cols) {
							break;
						}
						let zn = out[idx(y as usize, x as usize, cols)];
						if zn != low_value {
							outlet_id = zn;
							break;
						}
					} else {
						break;
					}
				}

				let (mut y, mut x) = (r as isize, c as isize);
				out[i] = outlet_id;
				loop {
					let ii = idx(y as usize, x as usize, cols);
					let dir = flow_dir[ii];
					if dir >= 0 {
						y += DY[dir as usize];
						x += DX[dir as usize];
						if !in_bounds(y, x, rows, cols) {
							break;
						}
						let ni = idx(y as usize, x as usize, cols);
						if out[ni] != low_value {
							break;
						}
						out[ni] = outlet_id;
					} else {
						break;
					}
				}
			}
		}

		let mut raster = vec_to_raster(&pntr, &out, DataType::I32);
		raster.nodata = out_nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

impl Tool for WatershedFromRasterPourPointsTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "watershed_from_raster_pour_points",
			display_name: "Watershed from Raster Pour Points",
			summary: "Delineates watersheds from a D8 pointer and a raster of pour-point outlet IDs.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "d8_pntr", description: "Input D8 pointer raster", required: true },
				ToolParamSpec { name: "pour_points", description: "Pour-points raster; non-zero non-NoData cells are outlets with their cell value as ID", required: true },
				ToolParamSpec { name: "esri_pntr", description: "Use ESRI pointer encoding", required: false },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		let mut defaults = ToolArgs::new();
		defaults.insert("esri_pntr".to_string(), json!(false));
		ToolManifest {
			id: "watershed_from_raster_pour_points".to_string(),
			display_name: "Watershed from Raster Pour Points".to_string(),
			summary: "Delineates watersheds from a D8 pointer and a raster of pour-point outlet IDs.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "raster_watershed".to_string(),
				description: "Delineate watersheds from a pour-points raster".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "watershed".to_string(), "pour_points".to_string(), "d8".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "d8_pntr")
			.or_else(|_| parse_raster_path_arg(args, "d8_pointer"))
			.or_else(|_| parse_raster_path_arg(args, "input"))?;
		parse_raster_path_arg(args, "pour_points")
			.or_else(|_| parse_raster_path_arg(args, "pour_pts"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let pntr_path = parse_raster_path_arg(args, "d8_pntr")
			.or_else(|_| parse_raster_path_arg(args, "d8_pointer"))
			.or_else(|_| parse_raster_path_arg(args, "input"))?;
		let pp_path = parse_raster_path_arg(args, "pour_points")
			.or_else(|_| parse_raster_path_arg(args, "pour_pts"))?;
		let esri_style = args.get("esri_pntr").and_then(|v| v.as_bool()).unwrap_or(false);
		let output_path = parse_optional_output_path(args, "output")?;

		let pntr = load_raster(&pntr_path)?;
		let pp = load_raster(&pp_path)?;
		let rows = pntr.rows;
		let cols = pntr.cols;
		if pp.rows != rows || pp.cols != cols {
			return Err(ToolError::Validation(
				"d8_pntr and pour_points must have the same number of rows and columns".to_string(),
			));
		}
		let pp_nodata = pp.nodata;
		let out_nodata = -32768.0f64;
		let low_value = (i32::MIN + 1) as f64;
		let mut out = vec![low_value; rows * cols];

		// Seed basin IDs from pour-points raster (non-zero, non-NoData cells)
		for r in 0..rows {
			for c in 0..cols {
				let pp_val = pp.get(0, r as isize, c as isize);
				if pp_val != pp_nodata && pp_val != 0.0 {
					out[idx(r, c, cols)] = pp_val;
				}
			}
		}
		// Build flow_dir and mark NoData cells in output
		let flow_dir = build_flow_dir_and_mark_nodata(&pntr, esri_style, &mut out, out_nodata, cols);
		// Two-pass watershed labeling
		run_watershed_labeling(&mut out, &flow_dir, rows, cols, low_value, out_nodata);

		let mut raster = vec_to_raster(&pntr, &out, DataType::I32);
		raster.nodata = out_nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

impl Tool for WatershedTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "watershed",
			display_name: "Watershed",
			summary: "Delineates watersheds from a D8 pointer and vector pour points.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "d8_pntr", description: "Input D8 pointer raster", required: true },
				ToolParamSpec { name: "pour_pts", description: "Input vector pour-points file (point or multipoint geometries)", required: true },
				ToolParamSpec { name: "esri_pntr", description: "Use ESRI pointer encoding", required: false },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		let mut defaults = ToolArgs::new();
		defaults.insert("esri_pntr".to_string(), json!(false));
		ToolManifest {
			id: "watershed".to_string(),
			display_name: "Watershed".to_string(),
			summary: "Delineates watersheds from a D8 pointer and vector pour points.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "vector_watershed".to_string(),
				description: "Delineate watersheds from a D8 pointer and vector pour points".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "watershed".to_string(), "vector".to_string(), "d8".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "d8_pntr")
			.or_else(|_| parse_raster_path_arg(args, "d8_pointer"))
			.or_else(|_| parse_raster_path_arg(args, "input"))?;
		parse_vector_path_arg(args, "pour_pts")
			.or_else(|_| parse_vector_path_arg(args, "pour_points"))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let pntr_path = parse_raster_path_arg(args, "d8_pntr")
			.or_else(|_| parse_raster_path_arg(args, "d8_pointer"))
			.or_else(|_| parse_raster_path_arg(args, "input"))?;
		let pp_path = parse_vector_path_arg(args, "pour_pts")
			.or_else(|_| parse_vector_path_arg(args, "pour_points"))?;
		let esri_style = args.get("esri_pntr").and_then(|v| v.as_bool()).unwrap_or(false);
		let output_path = parse_optional_output_path(args, "output")?;

		let pntr = load_raster(&pntr_path)?;
		let layer = read_vector_layer_aligned_to_dem(&pntr, &pp_path, "pour_points")?;
		let rows = pntr.rows;
		let cols = pntr.cols;
		let out_nodata = -32768.0f64;
		let low_value = (i32::MIN + 1) as f64;
		let mut out = vec![low_value; rows * cols];

		// Seed basin IDs from vector feature coordinates (1-based sequential IDs)
		for (feat_idx, feature) in layer.features.iter().enumerate() {
			let Some(ref geom) = feature.geometry else { continue };
			let coord = match geom {
				wbvector::Geometry::Point(c) => Some((c.x, c.y)),
				wbvector::Geometry::MultiPoint(cs) => cs.first().map(|c| (c.x, c.y)),
				_ => None,
			};
			if let Some((x, y)) = coord {
				// world_to_pixel returns Option<(col, row)>
				if let Some((col, row)) = pntr.world_to_pixel(x, y) {
					if in_bounds(row, col, rows, cols) {
						out[idx(row as usize, col as usize, cols)] = (feat_idx + 1) as f64;
					}
				}
			}
		}
		// Build flow_dir and mark NoData cells in output
		let flow_dir = build_flow_dir_and_mark_nodata(&pntr, esri_style, &mut out, out_nodata, cols);
		// Two-pass watershed labeling
		run_watershed_labeling(&mut out, &flow_dir, rows, cols, low_value, out_nodata);

		let mut raster = vec_to_raster(&pntr, &out, DataType::I32);
		raster.nodata = out_nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

impl Tool for JensonSnapPourPointsTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "jenson_snap_pour_points",
			display_name: "Jenson Snap Pour Points",
			summary: "Snaps each pour point to the nearest stream cell within a search distance, preserving all input attributes.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "pour_pts", description: "Input vector point file of pour points", required: true },
				ToolParamSpec { name: "streams", description: "Stream-network raster; stream cells have value > 0 and are not NoData", required: true },
				ToolParamSpec { name: "snap_dist", description: "Maximum search radius in map units (defaults to one cell width)", required: false },
				ToolParamSpec { name: "output", description: "Output snapped vector file path", required: true },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		let mut defaults = ToolArgs::new();
		defaults.insert("snap_dist".to_string(), json!(0.0));
		ToolManifest {
			id: "jenson_snap_pour_points".to_string(),
			display_name: "Jenson Snap Pour Points".to_string(),
			summary: "Snaps each pour point to the nearest stream cell within a search distance, preserving all input attributes.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "snap_pour_points".to_string(),
				description: "Snap pour points onto a stream network before watershed delineation".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "watershed".to_string(), "pour_points".to_string(), "snap".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_vector_path_arg(args, "pour_pts")?;
		parse_raster_path_arg(args, "streams")?;
		args.get("output")
			.and_then(|v| v.as_str())
			.filter(|s| !s.is_empty())
			.ok_or_else(|| ToolError::Validation("'output' path is required".into()))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let pp_path = parse_vector_path_arg(args, "pour_pts")?;
		let streams_path = parse_raster_path_arg(args, "streams")?;
		let output = args
			.get("output")
			.and_then(|v| v.as_str())
			.ok_or_else(|| ToolError::Validation("'output' path is required".into()))?
			.to_string();
		let snap_dist_arg = args.get("snap_dist").and_then(|v| v.as_f64());

		let streams = load_raster(&streams_path)?;
		let input_layer = read_vector_layer_aligned_to_dem(&streams, &pp_path, "pour_points")?;

		let cell_size = streams.cell_size_x;
		let snap_dist = match snap_dist_arg {
			Some(d) if d > 0.0 => d,
			_ => cell_size,
		};
		let snap_dist_int = ((snap_dist / cell_size) / 2.0).floor() as isize;
		let nodata = streams.nodata;
		let rows = streams.rows as isize;
		let cols = streams.cols as isize;

		// Build output layer with the same schema as the input
		let mut out_layer = wbvector::Layer::new("snapped_pour_points")
			.with_geom_type(wbvector::GeometryType::Point);
		if let Some(crs) = &input_layer.crs {
			out_layer.crs = Some(crs.clone());
		}
		for field in input_layer.schema.fields() {
			out_layer.add_field(field.clone());
		}

		for feature in &input_layer.features {
			let (px, py) = match &feature.geometry {
				Some(wbvector::Geometry::Point(c)) => (c.x, c.y),
				Some(wbvector::Geometry::MultiPoint(cs)) => {
					if let Some(c) = cs.first() {
						(c.x, c.y)
					} else {
						continue;
					}
				}
				_ => continue,
			};

			let (base_col, base_row) = match streams.world_to_pixel(px, py) {
				Some(cr) => cr,
				None => {
					// Point is outside raster extent — emit unchanged
					let mut f = wbvector::Feature::new();
					f.fid = feature.fid;
					f.geometry = Some(wbvector::Geometry::point(px, py));
					f.attributes = feature.attributes.clone();
					out_layer.features.push(f);
					continue;
				}
			};

			let mut min_dist = f64::INFINITY;
			let mut best_x = px;
			let mut best_y = py;

			for c in (base_col - snap_dist_int)..(base_col + snap_dist_int + 1) {
				for r in (base_row - snap_dist_int)..(base_row + snap_dist_int + 1) {
					if r < 0 || c < 0 || r >= rows || c >= cols {
						continue;
					}
					let zn = streams.get(0, r, c);
					if zn > 0.0 && zn != nodata {
						let sx = streams.col_center_x(c);
						let sy = streams.row_center_y(r);
						let d = (sx - px) * (sx - px) + (sy - py) * (sy - py);
						if d < min_dist {
							min_dist = d;
							best_x = sx;
							best_y = sy;
						}
					}
				}
			}

			let mut f = wbvector::Feature::new();
			f.fid = feature.fid;
			f.geometry = Some(wbvector::Geometry::point(best_x, best_y));
			f.attributes = feature.attributes.clone();
			out_layer.features.push(f);
		}

		// Detect output format from extension, default to GeoJSON
		let fmt = wbvector::VectorFormat::detect(&output).unwrap_or(wbvector::VectorFormat::GeoJson);
		if let Some(parent) = std::path::Path::new(&output).parent() {
			if !parent.as_os_str().is_empty() {
				std::fs::create_dir_all(parent).map_err(|e| {
					ToolError::Execution(format!("cannot create output directory: {e}"))
				})?;
			}
		}
		wbvector::write(&out_layer, &output, fmt)
			.map_err(|e| ToolError::Execution(format!("failed writing output vector: {e}")))?;

		Ok(build_result(output))
	}
}

impl Tool for SnapPourPointsTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "snap_pour_points",
			display_name: "Snap Pour Points",
			summary: "Snaps pour points to the highest flow-accumulation cell within a search distance.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "pour_pts", description: "Input vector point file of pour points", required: true },
				ToolParamSpec { name: "flow_accum", description: "Flow-accumulation raster", required: true },
				ToolParamSpec { name: "snap_dist", description: "Maximum search radius in map units (defaults to one cell width)", required: false },
				ToolParamSpec { name: "output", description: "Output snapped vector file path", required: true },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		let mut defaults = ToolArgs::new();
		defaults.insert("snap_dist".to_string(), json!(0.0));
		ToolManifest {
			id: "snap_pour_points".to_string(),
			display_name: "Snap Pour Points".to_string(),
			summary: "Snaps pour points to the highest flow-accumulation cell within a search distance.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "snap_pour_points_example".to_string(),
				description: "Snap pour points to nearby flow-accumulation maxima".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "watershed".to_string(), "pour_points".to_string(), "snap".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_vector_path_arg(args, "pour_pts").or_else(|_| parse_vector_path_arg(args, "pour_points"))?;
		parse_raster_path_arg(args, "flow_accum").or_else(|_| parse_raster_path_arg(args, "flow_accumulation"))?;
		args.get("output")
			.and_then(|v| v.as_str())
			.filter(|s| !s.is_empty())
			.ok_or_else(|| ToolError::Validation("'output' path is required".into()))?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let pp_path = parse_vector_path_arg(args, "pour_pts").or_else(|_| parse_vector_path_arg(args, "pour_points"))?;
		let flow_accum_path = parse_raster_path_arg(args, "flow_accum").or_else(|_| parse_raster_path_arg(args, "flow_accumulation"))?;
		let output = args
			.get("output")
			.and_then(|v| v.as_str())
			.ok_or_else(|| ToolError::Validation("'output' path is required".into()))?
			.to_string();
		let snap_dist_arg = args.get("snap_dist").and_then(|v| v.as_f64());

		let flow_accum = load_raster(&flow_accum_path)?;
		let input_layer = read_vector_layer_aligned_to_dem(&flow_accum, &pp_path, "pour_points")?;

		let cell_size = flow_accum.cell_size_x.abs().max(1.0e-12);
		let snap_dist = match snap_dist_arg {
			Some(d) if d > 0.0 => d,
			_ => cell_size,
		};
		let snap_dist_int = ((snap_dist / cell_size) / 2.0).floor() as isize;
		let nodata = flow_accum.nodata;
		let rows = flow_accum.rows as isize;
		let cols = flow_accum.cols as isize;

		let mut out_layer = wbvector::Layer::new("snapped_pour_points")
			.with_geom_type(wbvector::GeometryType::Point);
		if let Some(crs) = &input_layer.crs {
			out_layer.crs = Some(crs.clone());
		}
		for field in input_layer.schema.fields() {
			out_layer.add_field(field.clone());
		}

		for feature in &input_layer.features {
			let (px, py) = match &feature.geometry {
				Some(wbvector::Geometry::Point(c)) => (c.x, c.y),
				Some(wbvector::Geometry::MultiPoint(cs)) => {
					if let Some(c) = cs.first() {
						(c.x, c.y)
					} else {
						continue;
					}
				}
				_ => continue,
			};

			let (base_col, base_row) = match flow_accum.world_to_pixel(px, py) {
				Some(cr) => cr,
				None => {
					let mut f = wbvector::Feature::new();
					f.fid = feature.fid;
					f.geometry = Some(wbvector::Geometry::point(px, py));
					f.attributes = feature.attributes.clone();
					out_layer.features.push(f);
					continue;
				}
			};

			let mut max_accum = f64::NEG_INFINITY;
			let mut best_col = base_col;
			let mut best_row = base_row;
			let mut found = false;

			for c in (base_col - snap_dist_int)..(base_col + snap_dist_int + 1) {
				for r in (base_row - snap_dist_int)..(base_row + snap_dist_int + 1) {
					if r < 0 || c < 0 || r >= rows || c >= cols {
						continue;
					}
					let zn = flow_accum.get(0, r, c);
					if zn != nodata && zn > max_accum {
						max_accum = zn;
						best_col = c;
						best_row = r;
						found = true;
					}
				}
			}

			let (out_x, out_y) = if found {
				(flow_accum.col_center_x(best_col), flow_accum.row_center_y(best_row))
			} else {
				(px, py)
			};

			let mut f = wbvector::Feature::new();
			f.fid = feature.fid;
			f.geometry = Some(wbvector::Geometry::point(out_x, out_y));
			f.attributes = feature.attributes.clone();
			out_layer.features.push(f);
		}

		let fmt = wbvector::VectorFormat::detect(&output).unwrap_or(wbvector::VectorFormat::GeoJson);
		if let Some(parent) = std::path::Path::new(&output).parent() {
			if !parent.as_os_str().is_empty() {
				std::fs::create_dir_all(parent).map_err(|e| {
					ToolError::Execution(format!("cannot create output directory: {e}"))
				})?;
			}
		}
		wbvector::write(&out_layer, &output, fmt)
			.map_err(|e| ToolError::Execution(format!("failed writing output vector: {e}")))?;

		Ok(build_result(output))
	}
}

// ─── Shared helper: stream-link ID assignment ────────────────────────────────
/// Assigns a unique sequential ID to every stream link via a topological sort
/// from headwaters downstream. A new ID is started at headwaters and at every
/// confluence (cells with >1 inflowing stream neighbour).
///
/// Returns a flat `Vec<f64>` (`rows * cols`) where:
///   - Stream cells:              their link ID (≥ 1)
///   - Non-stream valid cells:    0.0
///   - NoData / pntr-nodata cells: `out_nodata`
fn stream_link_id_pass(pntr: &Raster, streams: &Raster, esri_style: bool, out_nodata: f64) -> Vec<f64> {
	let rows = pntr.rows;
	let cols = pntr.cols;
	let stream_nodata = streams.nodata;
	let pntr_nodata = pntr.nodata;

	// inflowing_vals[k] = pointer value that a neighbour at direction k must
	// carry to be flowing INTO the current cell.
	let inflowing_vals: [f64; 8] = if !esri_style {
		[16.0, 32.0, 64.0, 128.0, 1.0, 2.0, 4.0, 8.0]
	} else {
		[8.0, 16.0, 32.0, 64.0, 128.0, 1.0, 2.0, 4.0]
	};

	let mut pntr_matches = [999usize; 129];
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

	let mut pourpts = vec![out_nodata; rows * cols];
	let mut num_inflowing = vec![-1i8; rows * cols];
	let mut stack: Vec<(usize, usize)> = Vec::new();
	let mut current_id = 1.0f64;

	for r in 0..rows {
		for c in 0..cols {
			let i = idx(r, c, cols);
			let sv = streams.get(0, r as isize, c as isize);
			let pv = pntr.get(0, r as isize, c as isize);
			if sv > 0.0 && sv != stream_nodata {
				// Stream cell: count inflowing stream neighbours via the pointer
				let mut count = 0i8;
				for k in 0..8 {
					let rn = r as isize + DY[k];
					let cn = c as isize + DX[k];
					if in_bounds(rn, cn, rows, cols) {
						let sn = streams.get(0, rn, cn);
						let pn = pntr.get(0, rn, cn);
						if sn > 0.0 && sn != stream_nodata && pn == inflowing_vals[k] {
							count += 1;
						}
					}
				}
				num_inflowing[i] = count;
				if count == 0 {
					// Headwater: assign a new link ID immediately
					pourpts[i] = current_id;
					current_id += 1.0;
					stack.push((r, c));
				}
			} else if pv != pntr_nodata {
				// Non-stream valid cell
				pourpts[i] = 0.0;
			}
			// pntr-nodata cells stay at out_nodata
		}
	}

	while let Some((row, col)) = stack.pop() {
		let i = idx(row, col, cols);
		let val = pourpts[i];
		let pv = pntr.get(0, row as isize, col as isize) as usize;
		if pv > 0 && pv <= 128 && pntr_matches[pv] != 999 {
			let dir = pntr_matches[pv];
			let rn = row as isize + DY[dir];
			let cn = col as isize + DX[dir];
			if in_bounds(rn, cn, rows, cols) {
				let ni = idx(rn as usize, cn as usize, cols);
				let sv_n = streams.get(0, rn, cn);
				if sv_n > 0.0 && sv_n != stream_nodata {
					// Downstream is also a stream cell
					if num_inflowing[ni] > 1 {
						// Confluence: start a new link ID (checked before decrement,
						// matching legacy behaviour)
						current_id += 1.0;
						pourpts[ni] = current_id;
					} else if pourpts[ni] == out_nodata {
						// Single upstream: inherit the current link ID
						pourpts[ni] = val;
					}
					num_inflowing[ni] -= 1;
					if num_inflowing[ni] == 0 {
						stack.push((rn as usize, cn as usize));
					}
				}
			}
		}
	}

	pourpts
}

// ─── SubbasinsTool ────────────────────────────────────────────────────────────
impl Tool for SubbasinsTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "subbasins",
			display_name: "Subbasins",
			summary: "Identifies the catchment area of each stream link (sub-basins) in a D8 stream network.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "d8_pntr", description: "Input D8 pointer raster", required: true },
				ToolParamSpec { name: "streams", description: "Input streams raster (stream cells > 0)", required: true },
				ToolParamSpec { name: "esri_pntr", description: "Use ESRI D8 pointer encoding", required: false },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		let mut defaults = ToolArgs::new();
		defaults.insert("esri_pntr".to_string(), json!(false));
		ToolManifest {
			id: "subbasins".to_string(),
			display_name: "Subbasins".to_string(),
			summary: "Identifies the catchment area of each stream link (sub-basins) in a D8 stream network.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "subbasins_example".to_string(),
				description: "Delineate sub-basins from a D8 pointer and streams raster".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "watershed".to_string(), "subbasins".to_string(), "d8".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "d8_pntr").or_else(|_| parse_raster_path_arg(args, "d8_pointer"))?;
		parse_raster_path_arg(args, "streams")?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let pntr_path =
			parse_raster_path_arg(args, "d8_pntr").or_else(|_| parse_raster_path_arg(args, "d8_pointer"))?;
		let streams_path = parse_raster_path_arg(args, "streams")?;
		let esri_style = args.get("esri_pntr").and_then(|v| v.as_bool()).unwrap_or(false);
		let output_path = parse_optional_output_path(args, "output")?;

		let pntr = load_raster(&pntr_path)?;
		let streams = load_raster(&streams_path)?;
		let rows = pntr.rows;
		let cols = pntr.cols;
		if streams.rows != rows || streams.cols != cols {
			return Err(ToolError::Validation(
				"d8_pntr and streams must have the same dimensions".to_string(),
			));
		}

		let out_nodata = -32768.0f64;
		let low_value = (i32::MIN + 1) as f64;

		// Step 1: assign stream-link IDs
		let pourpts = stream_link_id_pass(&pntr, &streams, esri_style, out_nodata);

		// Step 2: seed the watershed output from stream-link IDs
		let mut out = vec![low_value; rows * cols];
		let flow_dir = build_flow_dir_and_mark_nodata(&pntr, esri_style, &mut out, out_nodata, cols);
		for i in 0..(rows * cols) {
			if pourpts[i] > 0.0 {
				out[i] = pourpts[i];
			}
		}

		// Step 3: two-pass watershed labeling
		run_watershed_labeling(&mut out, &flow_dir, rows, cols, low_value, out_nodata);

		let mut raster = vec_to_raster(&pntr, &out, DataType::I32);
		raster.nodata = out_nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

// ─── HillslopesTool ───────────────────────────────────────────────────────────
impl Tool for HillslopesTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "hillslopes",
			display_name: "Hillslopes",
			summary: "Identifies hillslope regions draining to each stream link, separating left- and right-bank areas.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "d8_pntr", description: "Input D8 pointer raster", required: true },
				ToolParamSpec { name: "streams", description: "Input streams raster (stream cells > 0)", required: true },
				ToolParamSpec { name: "esri_pntr", description: "Use ESRI D8 pointer encoding", required: false },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		let mut defaults = ToolArgs::new();
		defaults.insert("esri_pntr".to_string(), json!(false));
		ToolManifest {
			id: "hillslopes".to_string(),
			display_name: "Hillslopes".to_string(),
			summary: "Identifies hillslope regions draining to each stream link, separating left- and right-bank areas.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "hillslopes_example".to_string(),
				description: "Delineate hillslopes from a D8 pointer and streams raster".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec!["hydrology".to_string(), "hillslopes".to_string(), "watershed".to_string(), "d8".to_string()],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "d8_pntr").or_else(|_| parse_raster_path_arg(args, "d8_pointer"))?;
		parse_raster_path_arg(args, "streams")?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let pntr_path =
			parse_raster_path_arg(args, "d8_pntr").or_else(|_| parse_raster_path_arg(args, "d8_pointer"))?;
		let streams_path = parse_raster_path_arg(args, "streams")?;
		let esri_style = args.get("esri_pntr").and_then(|v| v.as_bool()).unwrap_or(false);
		let output_path = parse_optional_output_path(args, "output")?;

		let pntr = load_raster(&pntr_path)?;
		let streams = load_raster(&streams_path)?;
		let rows = pntr.rows;
		let cols = pntr.cols;
		if streams.rows != rows || streams.cols != cols {
			return Err(ToolError::Validation(
				"d8_pntr and streams must have the same dimensions".to_string(),
			));
		}

		let out_nodata = -32768.0f64;
		let low_value = (i32::MIN + 1) as f64;
		let stream_nodata = streams.nodata;
		let pntr_nodata = pntr.nodata;

		// Steps 1–3: same as SubbasinsTool
		let pourpts = stream_link_id_pass(&pntr, &streams, esri_style, out_nodata);
		let mut out = vec![low_value; rows * cols];
		let flow_dir = build_flow_dir_and_mark_nodata(&pntr, esri_style, &mut out, out_nodata, cols);
		for i in 0..(rows * cols) {
			if pourpts[i] > 0.0 {
				out[i] = pourpts[i];
			}
		}
		run_watershed_labeling(&mut out, &flow_dir, rows, cols, low_value, out_nodata);

		// Step 4: zero out stream cells
		for r in 0..rows {
			for c in 0..cols {
				let sv = streams.get(0, r as isize, c as isize);
				if sv > 0.0 && sv != stream_nodata {
					out[idx(r, c, cols)] = 0.0;
				}
			}
		}

		// Step 5: clump pass — flood-fill connected regions of the same old
		// sub-basin ID, but never crossing a stream cell diagonally.
		//
		// Direction encoding (DX/DY index):
		//   6 7 0
		//   5 x 1
		//   4 3 2
		// card1[n] = 8  → direction n is cardinal (always crossable)
		// card1[n] = d  → direction n is diagonal; d indexes card2/card3
		// card2[d] / card3[d] are the two adjacent cardinal directions whose
		// stream status controls whether the diagonal can be crossed.
		const CARD1: [usize; 8] = [0, 8, 1, 8, 2, 8, 3, 8];
		const CARD2: [usize; 4] = [7, 1, 3, 5];
		const CARD3: [usize; 4] = [1, 3, 5, 7];

		let mut visited = vec![1i8; rows * cols];
		let mut current_id = 0.0f64;
		let mut clump_stack: Vec<(usize, usize)> = Vec::new();

		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				if visited[i] > 0 && pntr.get(0, r as isize, c as isize) != pntr_nodata && out[i] > 0.0 {
					current_id += 1.0;
					let old_id = out[i];
					clump_stack.push((r, c));
					while let Some((r2, c2)) = clump_stack.pop() {
						let i2 = idx(r2, c2, cols);
						out[i2] = current_id;
						visited[i2] = 0;
						for n in 0..8 {
							let yn = r2 as isize + DY[n];
							let xn = c2 as isize + DX[n];
							if !in_bounds(yn, xn, rows, cols) {
								continue;
							}
							let ni = idx(yn as usize, xn as usize, cols);
							if out[ni] == old_id && visited[ni] > 0 {
								let diag = CARD1[n];
								if diag == 8 {
									// Cardinal direction — always allowed
									clump_stack.push((yn as usize, xn as usize));
								} else {
									// Diagonal — only allowed if NOT both adjacent
									// cardinal cells are streams
									let r_a = r2 as isize + DY[CARD2[diag]];
									let c_a = c2 as isize + DX[CARD2[diag]];
									let r_b = r2 as isize + DY[CARD3[diag]];
									let c_b = c2 as isize + DX[CARD3[diag]];
									let sv_a = if in_bounds(r_a, c_a, rows, cols) {
										streams.get(0, r_a, c_a)
									} else {
										0.0
									};
									let sv_b = if in_bounds(r_b, c_b, rows, cols) {
										streams.get(0, r_b, c_b)
									} else {
										0.0
									};
									if sv_a == 0.0 || sv_b == 0.0 {
										clump_stack.push((yn as usize, xn as usize));
									}
								}
							}
						}
					}
				}
			}
		}

		let mut raster = vec_to_raster(&pntr, &out, DataType::I32);
		raster.nodata = out_nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

// ─── StrahlerOrderBasinsTool ──────────────────────────────────────────────────
impl Tool for StrahlerOrderBasinsTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "strahler_order_basins",
			display_name: "Strahler Order Basins",
			summary: "Delineates watershed basins labelled by the Horton-Strahler order of their draining stream link.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "d8_pntr", description: "Input D8 pointer raster", required: true },
				ToolParamSpec { name: "streams", description: "Input streams raster (stream cells > 0)", required: true },
				ToolParamSpec { name: "esri_pntr", description: "Use ESRI D8 pointer encoding", required: false },
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		let mut defaults = ToolArgs::new();
		defaults.insert("esri_pntr".to_string(), json!(false));
		ToolManifest {
			id: "strahler_order_basins".to_string(),
			display_name: "Strahler Order Basins".to_string(),
			summary: "Delineates watershed basins labelled by the Horton-Strahler order of their draining stream link.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "strahler_basins_example".to_string(),
				description: "Delineate Strahler-order basins from a D8 pointer and streams raster".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec![
				"hydrology".to_string(),
				"strahler".to_string(),
				"watershed".to_string(),
				"stream_order".to_string(),
				"d8".to_string(),
			],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "d8_pntr").or_else(|_| parse_raster_path_arg(args, "d8_pointer"))?;
		parse_raster_path_arg(args, "streams")?;
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let pntr_path =
			parse_raster_path_arg(args, "d8_pntr").or_else(|_| parse_raster_path_arg(args, "d8_pointer"))?;
		let streams_path = parse_raster_path_arg(args, "streams")?;
		let esri_style = args.get("esri_pntr").and_then(|v| v.as_bool()).unwrap_or(false);
		let output_path = parse_optional_output_path(args, "output")?;

		let pntr = load_raster(&pntr_path)?;
		let streams = load_raster(&streams_path)?;
		let rows = pntr.rows;
		let cols = pntr.cols;
		if streams.rows != rows || streams.cols != cols {
			return Err(ToolError::Validation(
				"d8_pntr and streams must have the same dimensions".to_string(),
			));
		}

		let out_nodata = -32768.0f64;
		let low_value = (i32::MIN + 1) as f64;
		let stream_nodata = streams.nodata;
		let pntr_nodata = pntr.nodata;

		// inflowing_vals[k]: the WBT/ESRI pointer value a neighbour at dir k
		// must carry to be flowing INTO the current cell.
		let inflowing_vals: [f64; 8] = if !esri_style {
			[16.0, 32.0, 64.0, 128.0, 1.0, 2.0, 4.0, 8.0]
		} else {
			[8.0, 16.0, 32.0, 64.0, 128.0, 1.0, 2.0, 4.0]
		};

		let mut pntr_matches = [999usize; 129];
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

		// Step 1: Assign Horton-Strahler orders to all stream cells.
		// pourpts[i] = 0.0 for stream cells (initial), out_nodata for non-stream.
		// After the pass every reachable stream cell holds its Strahler order (≥ 1).
		let mut pourpts = vec![out_nodata; rows * cols];
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				let sv = streams.get(0, r as isize, c as isize);
				if sv > 0.0 && sv != stream_nodata {
					pourpts[i] = 0.0;
				}
			}
		}

		for r in 0..rows {
			for c in 0..cols {
				let sv = streams.get(0, r as isize, c as isize);
				if sv <= 0.0 || sv == stream_nodata {
					continue;
				}
				// Check if this is a headwater (no inflowing stream neighbours)
				let mut num_inflow = 0i8;
				for k in 0..8 {
					let rn = r as isize + DY[k];
					let cn = c as isize + DX[k];
					if in_bounds(rn, cn, rows, cols) {
						let sn = streams.get(0, rn, cn);
						let pn = pntr.get(0, rn, cn);
						if sn > 0.0 && sn != stream_nodata && pn == inflowing_vals[k] {
							num_inflow += 1;
						}
					}
				}
				if num_inflow > 0 {
					continue; // not a headwater
				}

				// Headwater: walk downstream assigning Strahler orders
				let mut x = c as isize;
				let mut y = r as isize;
				let mut current_order = 1.0f64;
				pourpts[idx(r, c, cols)] = current_order;

				loop {
					let pv = pntr.get(0, y, x);
					if pv <= 0.0 || pv == pntr_nodata {
						// No valid downstream pointer
						let sv2 = streams.get(0, y, x);
						if sv2 > 0.0 && sv2 != stream_nodata {
							// Edge cell with stream value — bump order by 1 (legacy behaviour)
							let ii = idx(y as usize, x as usize, cols);
							pourpts[ii] += 1.0;
						}
						break;
					}
					let pv_u = pv as usize;
					if pv_u > 128 || pntr_matches[pv_u] == 999 {
						break;
					}
					let dir = pntr_matches[pv_u];
					x += DX[dir];
					y += DY[dir];
					if !in_bounds(y, x, rows, cols) {
						break;
					}
					let sv_next = streams.get(0, y, x);
					if sv_next <= 0.0 || sv_next == stream_nodata {
						break; // left the stream channel
					}
					let ii = idx(y as usize, x as usize, cols);
					let current_val = pourpts[ii];
					if current_val > current_order {
						break; // hit a larger-order stream
					}
					if (current_val - current_order).abs() < 1e-10 {
						// Same order: check if ≥ 2 inflowing stream cells also carry current_order
						let mut same_order_inflow = 0i8;
						for k in 0..8 {
							let x2 = x + DX[k];
							let y2 = y + DY[k];
							if in_bounds(y2, x2, rows, cols) {
								let sn2 = streams.get(0, y2, x2);
								let pn2 = pntr.get(0, y2, x2);
								let ii2 = idx(y2 as usize, x2 as usize, cols);
								if sn2 > 0.0
									&& sn2 != stream_nodata
									&& pn2 == inflowing_vals[k]
									&& (pourpts[ii2] - current_order).abs() < 1e-10
								{
									same_order_inflow += 1;
								}
							}
						}
						if same_order_inflow >= 2 {
							current_order += 1.0; // full Strahler confluence
						} else {
							break;
						}
					}
					if current_val < current_order {
						pourpts[ii] = current_order;
					}
				}
			}
		}

		// Step 2: watershed labeling — seed from stream cells with Strahler order > 0
		let mut out = vec![low_value; rows * cols];
		let flow_dir = build_flow_dir_and_mark_nodata(&pntr, esri_style, &mut out, out_nodata, cols);
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				if pntr.get(0, r as isize, c as isize) == pntr_nodata {
					out[i] = out_nodata;
					continue;
				}
				let z = pourpts[i];
				if z != out_nodata && z > 0.0 {
					out[i] = z;
				}
			}
		}
		run_watershed_labeling(&mut out, &flow_dir, rows, cols, low_value, out_nodata);

		let mut raster = vec_to_raster(&pntr, &out, DataType::I32);
		raster.nodata = out_nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}

// ─── IsobasinsTool ────────────────────────────────────────────────────────────
impl Tool for IsobasinsTool {
	fn metadata(&self) -> ToolMetadata {
		ToolMetadata {
			id: "isobasins",
			display_name: "Isobasins",
			summary: "Divides a landscape into approximately equal-sized watersheds (isobasins) based on a target area threshold.",
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![
				ToolParamSpec { name: "dem", description: "Input hydrologically-conditioned DEM", required: true },
				ToolParamSpec {
					name: "target_size",
					description: "Target isobasin area in number of grid cells",
					required: true,
				},
				ToolParamSpec { name: "output", description: "Output raster path", required: false },
			],
		}
	}

	fn manifest(&self) -> ToolManifest {
		let mut defaults = ToolArgs::new();
		defaults.insert("target_size".to_string(), json!(1000));
		ToolManifest {
			id: "isobasins".to_string(),
			display_name: "Isobasins".to_string(),
			summary: "Divides a landscape into approximately equal-sized watersheds (isobasins) based on a target area threshold.".to_string(),
			category: ToolCategory::Hydrology,
			license_tier: LicenseTier::Open,
			params: vec![],
			defaults,
			examples: vec![ToolExample {
				name: "isobasins_example".to_string(),
				description: "Divide a landscape into equal-sized isobasins".to_string(),
				args: ToolArgs::new(),
			}],
			tags: vec![
				"hydrology".to_string(),
				"watershed".to_string(),
				"isobasins".to_string(),
				"basin".to_string(),
			],
			stability: ToolStability::Experimental,
		}
	}

	fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
		parse_raster_path_arg(args, "dem").or_else(|_| parse_raster_path_arg(args, "input"))?;
		let ts = args.get("target_size").and_then(|v| v.as_f64()).unwrap_or(0.0);
		if ts <= 0.0 {
			return Err(ToolError::Validation("'target_size' must be a positive number of grid cells".to_string()));
		}
		Ok(())
	}

	fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
		let dem_path = parse_raster_path_arg(args, "dem").or_else(|_| parse_raster_path_arg(args, "input"))?;
		let target_fa = args
			.get("target_size")
			.and_then(|v| v.as_f64())
			.filter(|&v| v > 0.0)
			.ok_or_else(|| ToolError::Validation("'target_size' must be a positive number".to_string()))? as usize;
		let output_path = parse_optional_output_path(args, "output")?;

		let dem = load_raster(&dem_path)?;
		let rows = dem.rows;
		let cols = dem.cols;
		let dem_nodata = dem.nodata;
		let out_nodata = -32768.0f64;

		// ── Step 1: D8 flow direction from DEM (steepest descent) ──────────────
		// flow_dir[i]: direction index 0-7, -1=flat/pit, -2=nodata
		let cell_x = dem.cell_size_x;
		let cell_y = dem.cell_size_y;
		let diag = (cell_x * cell_x + cell_y * cell_y).sqrt();
		let lens: [f64; 8] = [diag, cell_x, diag, cell_y, diag, cell_x, diag, cell_y];

		// inflowing_vals_i8[k]: the direction INDEX that a neighbour at direction k
		// must point to flow back INTO the current cell.
		const INFLOWING_I8: [i8; 8] = [4, 5, 6, 7, 0, 1, 2, 3];

		let mut flow_dir = vec![-2i8; rows * cols];
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				let z0 = dem.get(0, r as isize, c as isize);
				if z0 == dem_nodata {
					continue; // stays -2
				}
				let mut best_dir = -1i8;
				let mut best_slope = f64::MIN;
				for k in 0..8 {
					let rn = r as isize + DY[k];
					let cn = c as isize + DX[k];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let zn = dem.get(0, rn, cn);
					if zn == dem_nodata {
						continue;
					}
					let slope = (z0 - zn) / lens[k];
					if slope > best_slope {
						best_slope = slope;
						best_dir = k as i8;
					}
				}
				flow_dir[i] = best_dir;
			}
		}

		// ── Step 2: initialise accumulation array and in-flow counts ───────────
		// accum[i] = 1 for valid cells (each cell contributes itself)
		let mut accum = vec![1usize; rows * cols];
		let mut num_inflowing = vec![-1i8; rows * cols];
		let mut stack: Vec<(usize, usize)> = Vec::with_capacity(rows * cols);

		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				if flow_dir[i] == -2 {
					accum[i] = 0;
					continue;
				}
				let mut count = 0i8;
				for k in 0..8 {
					let rn = r as isize + DY[k];
					let cn = c as isize + DX[k];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					if flow_dir[ni] == INFLOWING_I8[k] {
						count += 1;
					}
				}
				num_inflowing[i] = count;
				if count == 0 {
					stack.push((r, c));
				}
			}
		}

		// ── Step 3: pour-point detection combined with flow accumulation ────────
		let mut output = vec![out_nodata; rows * cols];
		let mut outlet_id = 1.0f64;

		while let Some((row, col)) = stack.pop() {
			let i = idx(row, col, cols);
			let fa = accum[i];

			if fa >= target_fa {
				// Find the inflowing neighbour with the largest accumulation
				let mut inla_mag = 0usize;
				let mut inla_dir: Option<usize> = None;
				for k in 0..8 {
					let rn = row as isize + DY[k];
					let cn = col as isize + DX[k];
					if !in_bounds(rn, cn, rows, cols) {
						continue;
					}
					let ni = idx(rn as usize, cn as usize, cols);
					if flow_dir[ni] == INFLOWING_I8[k] && accum[ni] > inla_mag {
						inla_mag = accum[ni];
						inla_dir = Some(k);
					}
				}

				let split_to_neighbour =
					inla_dir.is_some() && target_fa.saturating_sub(inla_mag) < fa.saturating_sub(target_fa);

				if split_to_neighbour {
					let k = inla_dir.unwrap();
					let rn = (row as isize + DY[k]) as usize;
					let cn = (col as isize + DX[k]) as usize;
					let ni = idx(rn, cn, cols);
					accum[i] -= inla_mag;
					output[ni] = outlet_id;
					outlet_id += 1.0;
				} else {
					accum[i] = 1;
					output[i] = outlet_id;
					outlet_id += 1.0;
				}
			}

			let dir = flow_dir[i];
			if dir >= 0 {
				let rn = row as isize + DY[dir as usize];
				let cn = col as isize + DX[dir as usize];
				if in_bounds(rn, cn, rows, cols) {
					let ni = idx(rn as usize, cn as usize, cols);
					if accum[ni] > 0 {
						accum[ni] += accum[i];
					}
					num_inflowing[ni] -= 1;
					if num_inflowing[ni] == 0 {
						stack.push((rn as usize, cn as usize));
					}
				}
			} else if output[i] == out_nodata {
				// Watershed outlet (flows off edge or into a pit)
				output[i] = outlet_id;
				outlet_id += 1.0;
			}
		}

		// ── Step 4: trace every unlabelled cell downstream to its basin ─────────
		for r in 0..rows {
			for c in 0..cols {
				let i = idx(r, c, cols);
				if flow_dir[i] == -2 || output[i] != out_nodata {
					continue;
				}
				// Walk downstream to find the first labelled cell
				let mut basin_id = out_nodata;
				let (mut yr, mut xc) = (r as isize, c as isize);
				loop {
					let dir = flow_dir[idx(yr as usize, xc as usize, cols)];
					if dir >= 0 {
						yr += DY[dir as usize];
						xc += DX[dir as usize];
						if !in_bounds(yr, xc, rows, cols) {
							break;
						}
						let z = output[idx(yr as usize, xc as usize, cols)];
						if z != out_nodata {
							basin_id = z;
							break;
						}
					} else {
						break;
					}
				}
				// Walk again to stamp the whole path with basin_id
				output[i] = basin_id;
				let (mut yr, mut xc) = (r as isize, c as isize);
				loop {
					let dir = flow_dir[idx(yr as usize, xc as usize, cols)];
					if dir >= 0 {
						yr += DY[dir as usize];
						xc += DX[dir as usize];
						if !in_bounds(yr, xc, rows, cols) {
							break;
						}
						let ni = idx(yr as usize, xc as usize, cols);
						if output[ni] != out_nodata {
							break;
						}
						output[ni] = basin_id;
					} else {
						break;
					}
				}
			}
		}

		let mut raster = vec_to_raster(&dem, &output, DataType::I32);
		raster.nodata = out_nodata;
		Ok(build_result(write_or_store_output(raster, output_path)?))
	}
}
