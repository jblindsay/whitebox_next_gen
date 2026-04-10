use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap, HashMap, HashSet};
use std::sync::atomic::AtomicUsize;

mod nibble_sieve;
pub use nibble_sieve::{NibbleTool, SieveTool};


fn max_distance_squared(
    (x1, y1): (f64, f64),
    (x2, y2): (f64, f64),
    (x3, y3): (f64, f64),
    z1: f64,
    z2: f64,
    z3: f64,
) -> f64 {
    let d12_sq = (x1 - x2).powi(2) + (y1 - y2).powi(2) + (z1 - z2).powi(2);
    let d23_sq = (x2 - x3).powi(2) + (y2 - y3).powi(2) + (z2 - z3).powi(2);
    let d31_sq = (x3 - x1).powi(2) + (y3 - y1).powi(2) + (z3 - z1).powi(2);
    d12_sq.max(d23_sq).max(d31_sq)
}
use evalexpr::{build_operator_tree, ContextWithMutableVariables, HashMapContext, Value};
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use rayon::prelude::*;
use serde_json::json;
use wbcore::{
    parse_optional_output_path, parse_vector_path_arg, LicenseTier, Tool, ToolArgs, ToolCategory, ToolContext,
    ToolError, ToolExample, ToolManifest, ToolMetadata, ToolParamDescriptor, ToolParamSpec,
    ToolRunResult, ToolStability,
};
use wbraster::{CrsInfo, DataType, Raster, RasterConfig, RasterFormat};
use wbtopology::{
    buffer_linestring, buffer_point, buffer_polygon_multi, make_valid_polygon, BufferCapStyle,
    BufferJoinStyle, BufferOptions, Coord as TopoCoord, LineString as TopoLineString,
    Geometry as TopoGeometry, LinearRing as TopoLinearRing, Polygon as TopoPolygon,
    convex_hull, polygon_difference,
    delaunay_triangulation,
    polygonize_closed_linestrings,
    polygon_intersection, polygon_unary_dissolve,
    voronoi_diagram,
};

use crate::memory_store;

pub struct AggregateRasterTool;
pub struct AverageOverlayTool;
pub struct BufferRasterTool;
pub struct BufferVectorTool;
pub struct ClipRasterToPolygonTool;
pub struct ClipTool;
pub struct BlockMaximumTool;
pub struct BlockMinimumTool;
pub struct ClumpTool;
pub struct CountIfTool;
pub struct CostAllocationTool;
pub struct CostDistanceTool;
pub struct CostPathwayTool;
pub struct CreatePlaneTool;
pub struct CentroidRasterTool;
pub struct CentroidVectorTool;
pub struct DifferenceTool;
pub struct DissolveTool;
pub struct EuclideanAllocationTool;
pub struct EuclideanDistanceTool;
pub struct EliminateCoincidentPointsTool;
pub struct EraseTool;
pub struct ErasePolygonFromRasterTool;
pub struct ExtendVectorLinesTool;
pub struct FilterVectorFeaturesByAreaTool;
pub struct FindLowestOrHighestPointsTool;
pub struct ExtractNodesTool;
pub struct ExtractByAttributeTool;
pub struct HighestPositionTool;
pub struct IntersectTool;
pub struct LayerFootprintRasterTool;
pub struct LayerFootprintVectorTool;
pub struct HexagonalGridFromRasterBaseTool;
pub struct HexagonalGridFromVectorBaseTool;
pub struct RectangularGridFromRasterBaseTool;
pub struct RectangularGridFromVectorBaseTool;
pub struct LineIntersectionsTool;
pub struct MergeLineSegmentsTool;
pub struct MinimumBoundingBoxTool;
pub struct MinimumBoundingCircleTool;
pub struct MinimumBoundingEnvelopeTool;
pub struct MinimumConvexHullTool;
pub struct MedoidTool;
pub struct LowestPositionTool;
pub struct PolygonizeTool;
pub struct PolygonAreaTool;
pub struct PolygonPerimeterTool;
pub struct PolygonShortAxisTool;
pub struct PolygonLongAxisTool;
pub struct RasterAreaTool;
pub struct RasterPerimeterTool;
pub struct MaxAbsoluteOverlayTool;
pub struct MaxOverlayTool;
pub struct MinAbsoluteOverlayTool;
pub struct MinOverlayTool;
pub struct MultiplyOverlayTool;
pub struct PickFromListTool;
pub struct PercentEqualToTool;
pub struct PercentGreaterThanTool;
pub struct PercentLessThanTool;
pub struct StandardDeviationOverlayTool;
pub struct ReclassEqualIntervalTool;
pub struct ReclassTool;
pub struct FilterRasterFeaturesByAreaTool;
pub struct SmoothVectorsTool;
pub struct SnapEndnodesTool;
pub struct SplitVectorLinesTool;
pub struct SplitWithLinesTool;
pub struct SumOverlayTool;
pub struct SymmetricalDifferenceTool;
pub struct UnionTool;
pub struct UpdateNodataCellsTool;
pub struct WeightedOverlayTool;
pub struct WeightedSumTool;
pub struct BoundaryShapeComplexityTool;
pub struct IdwInterpolationTool;
pub struct CompactnessRatioTool;
pub struct DeviationFromRegionalDirectionTool;
pub struct EdgeProportionTool;
pub struct ElongationRatioTool;
pub struct FindPatchEdgeCellsTool;
pub struct HeatMapTool;
pub struct HoleProportionTool;
pub struct LinearityIndexTool;
pub struct NarrownessIndexTool;
pub struct PatchOrientationTool;
pub struct PerimeterAreaRatioTool;
pub struct RadiusOfGyrationTool;
pub struct RelatedCircumscribingCircleTool;
pub struct VoronoiDiagramTool;
pub struct MapFeaturesTool;
pub struct NearestNeighbourInterpolationTool;
pub struct TinInterpolationTool;
pub struct NaturalNeighbourInterpolationTool;
pub struct ModifiedShepardInterpolationTool;
pub struct RadialBasisFunctionInterpolationTool;
pub struct ExtractRasterValuesAtPointsTool;
pub struct RasterCellAssignmentTool;
pub struct ShapeComplexityIndexRasterTool;
pub struct ShapeComplexityIndexVectorTool;
pub struct TravellingSalesmanProblemTool;

pub struct ConstructVectorTinTool;
pub struct VectorHexBinningTool;

#[derive(Clone, Copy, PartialEq, Eq)]
enum GisOverlayOp {
    Average,
    CountIf,
    HighestPosition,
    LowestPosition,
    MaxAbsolute,
    Max,
    MinAbsolute,
    Min,
    Multiply,
    PercentEqualTo,
    PercentGreaterThan,
    PercentLessThan,
    StandardDeviation,
    Sum,
}

impl GisOverlayOp {
    fn id(self) -> &'static str {
        match self {
            Self::Average => "average_overlay",
            Self::CountIf => "count_if",
            Self::HighestPosition => "highest_position",
            Self::LowestPosition => "lowest_position",
            Self::MaxAbsolute => "max_absolute_overlay",
            Self::Max => "max_overlay",
            Self::MinAbsolute => "min_absolute_overlay",
            Self::Min => "min_overlay",
            Self::Multiply => "multiply_overlay",
            Self::PercentEqualTo => "percent_equal_to",
            Self::PercentGreaterThan => "percent_greater_than",
            Self::PercentLessThan => "percent_less_than",
            Self::StandardDeviation => "standard_deviation_overlay",
            Self::Sum => "sum_overlay",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::Average => "Average Overlay",
            Self::CountIf => "Count If",
            Self::HighestPosition => "Highest Position",
            Self::LowestPosition => "Lowest Position",
            Self::MaxAbsolute => "Max Absolute Overlay",
            Self::Max => "Max Overlay",
            Self::MinAbsolute => "Min Absolute Overlay",
            Self::Min => "Min Overlay",
            Self::Multiply => "Multiply Overlay",
            Self::PercentEqualTo => "Percent Equal To",
            Self::PercentGreaterThan => "Percent Greater Than",
            Self::PercentLessThan => "Percent Less Than",
            Self::StandardDeviation => "Standard Deviation Overlay",
            Self::Sum => "Sum Overlay",
        }
    }

    fn summary(self) -> &'static str {
        match self {
            Self::Average => "Computes the per-cell average across a raster stack, ignoring NoData unless all inputs are NoData.",
            Self::CountIf => "Counts the number of input rasters whose cell equals a comparison value.",
            Self::HighestPosition => "Returns the zero-based raster-stack index containing the highest value at each cell.",
            Self::LowestPosition => "Returns the zero-based raster-stack index containing the lowest value at each cell.",
            Self::MaxAbsolute => "Computes the per-cell maximum absolute value across a raster stack, propagating NoData if any input cell is NoData.",
            Self::Max => "Computes the per-cell maximum across a raster stack, propagating NoData if any input cell is NoData.",
            Self::MinAbsolute => "Computes the per-cell minimum absolute value across a raster stack, propagating NoData if any input cell is NoData.",
            Self::Min => "Computes the per-cell minimum across a raster stack, propagating NoData if any input cell is NoData.",
            Self::Multiply => "Computes the per-cell product across a raster stack, propagating NoData if any input cell is NoData.",
            Self::PercentEqualTo => "Computes the fraction of rasters in a stack whose values equal the comparison raster at each cell.",
            Self::PercentGreaterThan => "Computes the fraction of rasters in a stack whose values are greater than the comparison raster at each cell.",
            Self::PercentLessThan => "Computes the fraction of rasters in a stack whose values are less than the comparison raster at each cell.",
            Self::StandardDeviation => "Computes the per-cell standard deviation across a raster stack, propagating NoData if any input cell is NoData.",
            Self::Sum => "Computes the per-cell sum across a raster stack, propagating NoData if any input cell is NoData.",
        }
    }

    fn output_type(self) -> DataType {
        match self {
            Self::CountIf | Self::HighestPosition | Self::LowestPosition => DataType::I16,
            Self::Average
            | Self::MaxAbsolute
            | Self::Max
            | Self::MinAbsolute
            | Self::Min
            | Self::Multiply
            | Self::PercentEqualTo
            | Self::PercentGreaterThan
            | Self::PercentLessThan
            | Self::StandardDeviation
            | Self::Sum => DataType::F64,
        }
    }

    fn default_output_name(self) -> &'static str {
        match self {
            Self::Average => "average_overlay.tif",
            Self::CountIf => "count_if.tif",
            Self::HighestPosition => "highest_position.tif",
            Self::LowestPosition => "lowest_position.tif",
            Self::MaxAbsolute => "max_absolute_overlay.tif",
            Self::Max => "max_overlay.tif",
            Self::MinAbsolute => "min_absolute_overlay.tif",
            Self::Min => "min_overlay.tif",
            Self::Multiply => "multiply_overlay.tif",
            Self::PercentEqualTo => "percent_equal_to.tif",
            Self::PercentGreaterThan => "percent_greater_than.tif",
            Self::PercentLessThan => "percent_less_than.tif",
            Self::StandardDeviation => "standard_deviation_overlay.tif",
            Self::Sum => "sum_overlay.tif",
        }
    }

    fn run_message(self) -> &'static str {
        match self {
            Self::Average => "running average overlay",
            Self::CountIf => "running count if",
            Self::HighestPosition => "running highest position",
            Self::LowestPosition => "running lowest position",
            Self::MaxAbsolute => "running max absolute overlay",
            Self::Max => "running max overlay",
            Self::MinAbsolute => "running min absolute overlay",
            Self::Min => "running min overlay",
            Self::Multiply => "running multiply overlay",
            Self::PercentEqualTo => "running percent equal to",
            Self::PercentGreaterThan => "running percent greater than",
            Self::PercentLessThan => "running percent less than",
            Self::StandardDeviation => "running standard deviation overlay",
            Self::Sum => "running sum overlay",
        }
    }

    fn processing_message(self) -> &'static str {
        match self {
            Self::Average => "averaging raster stack",
            Self::CountIf => "counting comparison matches",
            Self::HighestPosition => "finding highest stack positions",
            Self::LowestPosition => "finding lowest stack positions",
            Self::MaxAbsolute => "computing per-cell maximum absolute values",
            Self::Max => "computing per-cell maxima",
            Self::MinAbsolute => "computing per-cell minimum absolute values",
            Self::Min => "computing per-cell minima",
            Self::Multiply => "multiplying raster stack",
            Self::PercentEqualTo => "comparing raster stack for equality",
            Self::PercentGreaterThan => "comparing raster stack for greater-than matches",
            Self::PercentLessThan => "comparing raster stack for less-than matches",
            Self::StandardDeviation => "computing per-cell standard deviation",
            Self::Sum => "summing raster stack",
        }
    }

    fn tags(self) -> Vec<String> {
        let mut tags = vec!["raster".to_string(), "gis".to_string(), "overlay".to_string(), "legacy-port".to_string()];
        match self {
            Self::Average => tags.push("average".to_string()),
            Self::CountIf => tags.push("count".to_string()),
            Self::HighestPosition | Self::LowestPosition => tags.push("position".to_string()),
            Self::MaxAbsolute | Self::MinAbsolute => tags.push("absolute".to_string()),
            Self::Max => tags.push("maximum".to_string()),
            Self::Min => tags.push("minimum".to_string()),
            Self::Multiply => tags.push("multiply".to_string()),
            Self::PercentEqualTo | Self::PercentGreaterThan | Self::PercentLessThan => tags.push("percent".to_string()),
            Self::StandardDeviation => tags.push("standard-deviation".to_string()),
            Self::Sum => tags.push("sum".to_string()),
        }
        tags
    }

    fn needs_comparison_value(self) -> bool {
        matches!(self, Self::CountIf)
    }

    fn needs_comparison_raster(self) -> bool {
        matches!(self, Self::PercentEqualTo | Self::PercentGreaterThan | Self::PercentLessThan)
    }
}

struct GisOverlayCore;

impl GisOverlayCore {
    fn parse_raster_list_arg(args: &ToolArgs, key: &str) -> Result<Vec<String>, ToolError> {
        let value = args
            .get(key)
            .ok_or_else(|| ToolError::Validation(format!("parameter '{}' is required", key)))?;

        if let Some(s) = value.as_str() {
            let items = s
                .split([',', ';'])
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(|item| item.to_string())
                .collect::<Vec<_>>();
            if items.is_empty() {
                return Err(ToolError::Validation(format!(
                    "parameter '{}' did not contain any raster paths",
                    key
                )));
            }
            return Ok(items);
        }

        if let Some(arr) = value.as_array() {
            let mut items = Vec::with_capacity(arr.len());
            for (index, item) in arr.iter().enumerate() {
                let Some(path) = item.as_str() else {
                    return Err(ToolError::Validation(format!(
                        "parameter '{}' array element {} must be a string path",
                        key, index
                    )));
                };
                let path = path.trim();
                if path.is_empty() {
                    return Err(ToolError::Validation(format!(
                        "parameter '{}' array element {} is empty",
                        key, index
                    )));
                }
                items.push(path.to_string());
            }
            if items.is_empty() {
                return Err(ToolError::Validation(format!(
                    "parameter '{}' did not contain any raster paths",
                    key
                )));
            }
            return Ok(items);
        }

        Err(ToolError::Validation(format!(
            "parameter '{}' must be a string list or array",
            key
        )))
    }

    fn parse_comparison_value(args: &ToolArgs) -> Result<f64, ToolError> {
        args.get("comparison_value")
            .and_then(|value| value.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'comparison_value' must be a number".to_string()))
    }

    fn load_raster(path: &str, param_name: &str) -> Result<Raster, ToolError> {
        if memory_store::raster_is_memory_path(path) {
            let id = memory_store::raster_path_to_id(path).ok_or_else(|| {
                ToolError::Validation(format!(
                    "parameter '{}' has malformed in-memory raster path",
                    param_name
                ))
            })?;
            return memory_store::get_raster_by_id(id).ok_or_else(|| {
                ToolError::Validation(format!(
                    "parameter '{}' references unknown in-memory raster id '{}': store entry is missing",
                    param_name, id
                ))
            });
        }

        Raster::read(path)
            .map_err(|e| ToolError::Execution(format!("failed reading {} raster: {}", param_name, e)))
    }

    fn load_optional_comparison_raster(args: &ToolArgs) -> Result<Option<Raster>, ToolError> {
        let Some(value) = args.get("comparison") else {
            return Ok(None);
        };
        let Some(path) = value.as_str() else {
            return Err(ToolError::Validation(
                "parameter 'comparison' must be a raster path or memory raster handle".to_string(),
            ));
        };
        Ok(Some(Self::load_raster(path.trim(), "comparison")?))
    }

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1.0e-9 * a.abs().max(b.abs()).max(1.0)
    }

    fn rasters_share_grid(first: &Raster, second: &Raster) -> bool {
        second.rows == first.rows
            && second.cols == first.cols
            && second.bands == first.bands
            && Self::approx_eq(second.x_min, first.x_min)
            && Self::approx_eq(second.y_min, first.y_min)
            && Self::approx_eq(second.cell_size_x, first.cell_size_x)
            && Self::approx_eq(second.cell_size_y, first.cell_size_y)
    }

    fn ensure_same_grid(rasters: &[Raster]) -> Result<(), ToolError> {
        let first = &rasters[0];
        for raster in rasters.iter().skip(1) {
            if !Self::rasters_share_grid(first, raster) {
                return Err(ToolError::Validation(
                    "input rasters must have identical rows, columns, bands, cell sizes, and spatial extent"
                        .to_string(),
                ));
            }
        }
        Ok(())
    }

    fn output_raster(template: &Raster, op: GisOverlayOp) -> Raster {
        Raster::new(RasterConfig {
            cols: template.cols,
            rows: template.rows,
            bands: template.bands,
            x_min: template.x_min,
            y_min: template.y_min,
            cell_size: template.cell_size_x,
            cell_size_y: Some(template.cell_size_y),
            nodata: -32768.0,
            data_type: op.output_type(),
            crs: template.crs.clone(),
            metadata: template.metadata.clone(),
        })
    }

    fn compute_value(
        op: GisOverlayOp,
        cell_index: usize,
        rasters: &[Raster],
        comparison_value: Option<f64>,
        comparison_raster: Option<&Raster>,
        nodata: f64,
    ) -> f64 {
        match op {
            GisOverlayOp::Average => {
                let mut sum = 0.0;
                let mut count = 0.0;
                for raster in rasters {
                    let value = raster.data.get_f64(cell_index);
                    if !raster.is_nodata(value) {
                        sum += value;
                        count += 1.0;
                    }
                }
                if count > 0.0 { sum / count } else { nodata }
            }
            GisOverlayOp::CountIf => {
                let target = comparison_value.expect("count_if requires comparison value");
                let mut count = 0.0;
                let mut has_valid = false;
                for raster in rasters {
                    let value = raster.data.get_f64(cell_index);
                    if !raster.is_nodata(value) {
                        has_valid = true;
                        if value == target {
                            count += 1.0;
                        }
                    }
                }
                if has_valid { count } else { nodata }
            }
            GisOverlayOp::HighestPosition => {
                let mut best_value = f64::NEG_INFINITY;
                let mut best_index = nodata;
                for (index, raster) in rasters.iter().enumerate() {
                    let value = raster.data.get_f64(cell_index);
                    if raster.is_nodata(value) {
                        return nodata;
                    }
                    if value > best_value {
                        best_value = value;
                        best_index = index as f64;
                    }
                }
                best_index
            }
            GisOverlayOp::LowestPosition => {
                let mut best_value = f64::INFINITY;
                let mut best_index = nodata;
                for (index, raster) in rasters.iter().enumerate() {
                    let value = raster.data.get_f64(cell_index);
                    if raster.is_nodata(value) {
                        return nodata;
                    }
                    if value < best_value {
                        best_value = value;
                        best_index = index as f64;
                    }
                }
                best_index
            }
            GisOverlayOp::MaxAbsolute => {
                let mut best_value = f64::NEG_INFINITY;
                for raster in rasters {
                    let value = raster.data.get_f64(cell_index);
                    if raster.is_nodata(value) {
                        return nodata;
                    }
                    let abs_value = value.abs();
                    if abs_value > best_value {
                        best_value = abs_value;
                    }
                }
                best_value
            }
            GisOverlayOp::Max => {
                let mut best_value = f64::NEG_INFINITY;
                for raster in rasters {
                    let value = raster.data.get_f64(cell_index);
                    if raster.is_nodata(value) {
                        return nodata;
                    }
                    if value > best_value {
                        best_value = value;
                    }
                }
                best_value
            }
            GisOverlayOp::MinAbsolute => {
                let mut best_value = f64::INFINITY;
                for raster in rasters {
                    let value = raster.data.get_f64(cell_index);
                    if raster.is_nodata(value) {
                        return nodata;
                    }
                    let abs_value = value.abs();
                    if abs_value < best_value {
                        best_value = abs_value;
                    }
                }
                best_value
            }
            GisOverlayOp::Min => {
                let mut best_value = f64::INFINITY;
                for raster in rasters {
                    let value = raster.data.get_f64(cell_index);
                    if raster.is_nodata(value) {
                        return nodata;
                    }
                    if value < best_value {
                        best_value = value;
                    }
                }
                best_value
            }
            GisOverlayOp::Multiply => {
                let mut product = 1.0;
                for raster in rasters {
                    let value = raster.data.get_f64(cell_index);
                    if raster.is_nodata(value) {
                        return nodata;
                    }
                    product *= value;
                }
                product
            }
            GisOverlayOp::PercentEqualTo | GisOverlayOp::PercentGreaterThan | GisOverlayOp::PercentLessThan => {
                let comparison = comparison_raster.expect("percent tools require comparison raster");
                let reference = comparison.data.get_f64(cell_index);
                if comparison.is_nodata(reference) {
                    return nodata;
                }
                let mut matches = 0.0;
                for raster in rasters {
                    let value = raster.data.get_f64(cell_index);
                    if raster.is_nodata(value) {
                        return nodata;
                    }
                    let hit = match op {
                        GisOverlayOp::PercentEqualTo => value == reference,
                        GisOverlayOp::PercentGreaterThan => value > reference,
                        GisOverlayOp::PercentLessThan => value < reference,
                        _ => unreachable!(),
                    };
                    if hit {
                        matches += 1.0;
                    }
                }
                matches / rasters.len() as f64
            }
            GisOverlayOp::StandardDeviation => {
                let mut sum = 0.0;
                for raster in rasters {
                    let value = raster.data.get_f64(cell_index);
                    if raster.is_nodata(value) {
                        return nodata;
                    }
                    sum += value;
                }
                let mean = sum / rasters.len() as f64;
                let mut variance = 0.0;
                for raster in rasters {
                    let value = raster.data.get_f64(cell_index);
                    let delta = value - mean;
                    variance += delta * delta;
                }
                (variance / rasters.len() as f64).sqrt()
            }
            GisOverlayOp::Sum => {
                let mut sum = 0.0;
                for raster in rasters {
                    let value = raster.data.get_f64(cell_index);
                    if raster.is_nodata(value) {
                        return nodata;
                    }
                    sum += value;
                }
                sum
            }
        }
    }

    fn store_or_write_output(output: Raster, output_path: Option<std::path::PathBuf>, ctx: &ToolContext) -> Result<String, ToolError> {
        if let Some(output_path) = output_path {
            if let Some(parent) = output_path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| ToolError::Execution(format!("failed creating output directory: {e}")))?;
                }
            }

            let output_path_str = output_path.to_string_lossy().to_string();
            let output_format = RasterFormat::for_output_path(&output_path_str)
                .map_err(|e| ToolError::Validation(format!("unsupported output path: {e}")))?;
            ctx.progress.info("writing output raster");
            output
                .write(&output_path_str, output_format)
                .map_err(|e| ToolError::Execution(format!("failed writing output raster: {e}")))?;
            Ok(output_path_str)
        } else {
            ctx.progress.info("storing output raster in memory");
            let id = memory_store::put_raster(output);
            Ok(memory_store::make_raster_memory_path(&id))
        }
    }

    fn build_result(path: String) -> ToolRunResult {
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(path));
        outputs.insert("active_band".to_string(), json!(0));
        ToolRunResult { outputs }
    }

    fn metadata_for(op: GisOverlayOp) -> ToolMetadata {
        let mut params = vec![ToolParamSpec {
            name: "input_rasters",
            description: "Input raster stack as an array of paths or a semicolon/comma-delimited string.",
            required: true,
        }];
        if op.needs_comparison_value() {
            params.push(ToolParamSpec {
                name: "comparison_value",
                description: "Comparison value to count within the raster stack.",
                required: true,
            });
        }
        if op.needs_comparison_raster() {
            params.push(ToolParamSpec {
                name: "comparison",
                description: "Comparison raster path or in-memory raster handle.",
                required: true,
            });
        }
        params.push(ToolParamSpec {
            name: "output",
            description: "Optional output raster file path. If omitted, the result remains in memory.",
            required: false,
        });

        ToolMetadata {
            id: op.id(),
            display_name: op.display_name(),
            summary: op.summary(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params,
        }
    }

    fn manifest_for(op: GisOverlayOp) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert(
            "input_rasters".to_string(),
            json!(["input1.tif", "input2.tif"]),
        );
        if op.needs_comparison_value() {
            defaults.insert("comparison_value".to_string(), json!(1.0));
        }
        if op.needs_comparison_raster() {
            defaults.insert("comparison".to_string(), json!("comparison.tif"));
        }

        let mut example_args = ToolArgs::new();
        example_args.insert(
            "input_rasters".to_string(),
            json!(["input1.tif", "input2.tif", "input3.tif"]),
        );
        if op.needs_comparison_value() {
            example_args.insert("comparison_value".to_string(), json!(1.0));
        }
        if op.needs_comparison_raster() {
            example_args.insert("comparison".to_string(), json!("comparison.tif"));
        }
        example_args.insert("output".to_string(), json!(op.default_output_name()));

        let mut params = vec![ToolParamDescriptor {
            name: "input_rasters".to_string(),
            description: "Input raster stack as an array of paths or a semicolon/comma-delimited string.".to_string(),
            required: true,
        }];
        if op.needs_comparison_value() {
            params.push(ToolParamDescriptor {
                name: "comparison_value".to_string(),
                description: "Comparison value to count within the raster stack.".to_string(),
                required: true,
            });
        }
        if op.needs_comparison_raster() {
            params.push(ToolParamDescriptor {
                name: "comparison".to_string(),
                description: "Comparison raster path or in-memory raster handle.".to_string(),
                required: true,
            });
        }
        params.push(ToolParamDescriptor {
            name: "output".to_string(),
            description: "Optional output raster file path. If omitted, the result remains in memory.".to_string(),
            required: false,
        });

        ToolManifest {
            id: op.id().to_string(),
            display_name: op.display_name().to_string(),
            summary: op.summary().to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params,
            defaults,
            examples: vec![ToolExample {
                name: format!("{}_basic", op.id()),
                description: format!("Runs {} on a raster stack and writes the result to {}.", op.id(), op.default_output_name()),
                args: example_args,
            }],
            tags: op.tags(),
            stability: ToolStability::Experimental,
        }
    }

    fn validate(op: GisOverlayOp, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = Self::parse_raster_list_arg(args, "input_rasters")?;
        if op.needs_comparison_value() {
            let _ = Self::parse_comparison_value(args)?;
        }
        if op.needs_comparison_raster() {
            let _ = Self::load_optional_comparison_raster(args)?.ok_or_else(|| {
                ToolError::Validation("parameter 'comparison' is required".to_string())
            })?;
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(op: GisOverlayOp, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = Self::parse_raster_list_arg(args, "input_rasters")?;
        let comparison_value = if op.needs_comparison_value() {
            Some(Self::parse_comparison_value(args)?)
        } else {
            None
        };
        let comparison_raster = if op.needs_comparison_raster() {
            Some(Self::load_optional_comparison_raster(args)?.ok_or_else(|| {
                ToolError::Validation("parameter 'comparison' is required".to_string())
            })?)
        } else {
            None
        };
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info(op.run_message());
        ctx.progress.info("reading input rasters");

        let rasters = input_paths
            .iter()
            .enumerate()
            .map(|(index, path)| Self::load_raster(path, &format!("input_rasters[{index}]")))
            .collect::<Result<Vec<_>, _>>()?;

        if rasters.is_empty() {
            return Err(ToolError::Validation("at least one input raster is required".to_string()));
        }

        Self::ensure_same_grid(&rasters)?;
        if let Some(comparison) = comparison_raster.as_ref() {
            if !Self::rasters_share_grid(&rasters[0], comparison) {
                return Err(ToolError::Validation(
                    "comparison raster must have identical rows, columns, bands, cell sizes, and spatial extent"
                        .to_string(),
                ));
            }
        }

        let mut output = Self::output_raster(&rasters[0], op);
        let nodata = output.nodata;
        let len = output.data.len();
        let mut out_values = vec![nodata; len];
        let chunk_size = 8192usize;

        ctx.progress.info(op.processing_message());
        for (chunk_index, out_chunk) in out_values.chunks_mut(chunk_size).enumerate() {
            let start = chunk_index * chunk_size;
            out_chunk
                .par_iter_mut()
                .enumerate()
                .for_each(|(offset, dst)| {
                    let cell_index = start + offset;
                    *dst = Self::compute_value(
                        op,
                        cell_index,
                        &rasters,
                        comparison_value,
                        comparison_raster.as_ref(),
                        nodata,
                    );
                });

            let done = ((chunk_index + 1) * chunk_size).min(len);
            ctx.progress.progress(done as f64 / len.max(1) as f64);
        }

        for (index, value) in out_values.iter().enumerate() {
            output.data.set_f64(index, *value);
        }

        let output_locator = Self::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(Self::build_result(output_locator))
    }
}

impl Tool for AverageOverlayTool {
    fn metadata(&self) -> ToolMetadata {
        GisOverlayCore::metadata_for(GisOverlayOp::Average)
    }

    fn manifest(&self) -> ToolManifest {
        GisOverlayCore::manifest_for(GisOverlayOp::Average)
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        GisOverlayCore::validate(GisOverlayOp::Average, args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        GisOverlayCore::run(GisOverlayOp::Average, args, ctx)
    }
}

impl Tool for CountIfTool {
    fn metadata(&self) -> ToolMetadata {
        GisOverlayCore::metadata_for(GisOverlayOp::CountIf)
    }

    fn manifest(&self) -> ToolManifest {
        GisOverlayCore::manifest_for(GisOverlayOp::CountIf)
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        GisOverlayCore::validate(GisOverlayOp::CountIf, args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        GisOverlayCore::run(GisOverlayOp::CountIf, args, ctx)
    }
}

impl Tool for HighestPositionTool {
    fn metadata(&self) -> ToolMetadata {
        GisOverlayCore::metadata_for(GisOverlayOp::HighestPosition)
    }

    fn manifest(&self) -> ToolManifest {
        GisOverlayCore::manifest_for(GisOverlayOp::HighestPosition)
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        GisOverlayCore::validate(GisOverlayOp::HighestPosition, args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        GisOverlayCore::run(GisOverlayOp::HighestPosition, args, ctx)
    }
}

impl Tool for LowestPositionTool {
    fn metadata(&self) -> ToolMetadata {
        GisOverlayCore::metadata_for(GisOverlayOp::LowestPosition)
    }

    fn manifest(&self) -> ToolManifest {
        GisOverlayCore::manifest_for(GisOverlayOp::LowestPosition)
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        GisOverlayCore::validate(GisOverlayOp::LowestPosition, args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        GisOverlayCore::run(GisOverlayOp::LowestPosition, args, ctx)
    }
}

impl Tool for MaxOverlayTool {
    fn metadata(&self) -> ToolMetadata {
        GisOverlayCore::metadata_for(GisOverlayOp::Max)
    }

    fn manifest(&self) -> ToolManifest {
        GisOverlayCore::manifest_for(GisOverlayOp::Max)
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        GisOverlayCore::validate(GisOverlayOp::Max, args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        GisOverlayCore::run(GisOverlayOp::Max, args, ctx)
    }
}

impl Tool for MaxAbsoluteOverlayTool {
    fn metadata(&self) -> ToolMetadata {
        GisOverlayCore::metadata_for(GisOverlayOp::MaxAbsolute)
    }

    fn manifest(&self) -> ToolManifest {
        GisOverlayCore::manifest_for(GisOverlayOp::MaxAbsolute)
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        GisOverlayCore::validate(GisOverlayOp::MaxAbsolute, args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        GisOverlayCore::run(GisOverlayOp::MaxAbsolute, args, ctx)
    }
}

impl Tool for MinOverlayTool {
    fn metadata(&self) -> ToolMetadata {
        GisOverlayCore::metadata_for(GisOverlayOp::Min)
    }

    fn manifest(&self) -> ToolManifest {
        GisOverlayCore::manifest_for(GisOverlayOp::Min)
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        GisOverlayCore::validate(GisOverlayOp::Min, args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        GisOverlayCore::run(GisOverlayOp::Min, args, ctx)
    }
}

impl Tool for MinAbsoluteOverlayTool {
    fn metadata(&self) -> ToolMetadata {
        GisOverlayCore::metadata_for(GisOverlayOp::MinAbsolute)
    }

    fn manifest(&self) -> ToolManifest {
        GisOverlayCore::manifest_for(GisOverlayOp::MinAbsolute)
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        GisOverlayCore::validate(GisOverlayOp::MinAbsolute, args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        GisOverlayCore::run(GisOverlayOp::MinAbsolute, args, ctx)
    }
}

impl Tool for MultiplyOverlayTool {
    fn metadata(&self) -> ToolMetadata {
        GisOverlayCore::metadata_for(GisOverlayOp::Multiply)
    }

    fn manifest(&self) -> ToolManifest {
        GisOverlayCore::manifest_for(GisOverlayOp::Multiply)
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        GisOverlayCore::validate(GisOverlayOp::Multiply, args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        GisOverlayCore::run(GisOverlayOp::Multiply, args, ctx)
    }
}

impl Tool for PercentEqualToTool {
    fn metadata(&self) -> ToolMetadata {
        GisOverlayCore::metadata_for(GisOverlayOp::PercentEqualTo)
    }

    fn manifest(&self) -> ToolManifest {
        GisOverlayCore::manifest_for(GisOverlayOp::PercentEqualTo)
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        GisOverlayCore::validate(GisOverlayOp::PercentEqualTo, args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        GisOverlayCore::run(GisOverlayOp::PercentEqualTo, args, ctx)
    }
}

impl Tool for PercentGreaterThanTool {
    fn metadata(&self) -> ToolMetadata {
        GisOverlayCore::metadata_for(GisOverlayOp::PercentGreaterThan)
    }

    fn manifest(&self) -> ToolManifest {
        GisOverlayCore::manifest_for(GisOverlayOp::PercentGreaterThan)
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        GisOverlayCore::validate(GisOverlayOp::PercentGreaterThan, args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        GisOverlayCore::run(GisOverlayOp::PercentGreaterThan, args, ctx)
    }
}

impl Tool for PercentLessThanTool {
    fn metadata(&self) -> ToolMetadata {
        GisOverlayCore::metadata_for(GisOverlayOp::PercentLessThan)
    }

    fn manifest(&self) -> ToolManifest {
        GisOverlayCore::manifest_for(GisOverlayOp::PercentLessThan)
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        GisOverlayCore::validate(GisOverlayOp::PercentLessThan, args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        GisOverlayCore::run(GisOverlayOp::PercentLessThan, args, ctx)
    }
}

impl Tool for SumOverlayTool {
    fn metadata(&self) -> ToolMetadata {
        GisOverlayCore::metadata_for(GisOverlayOp::Sum)
    }

    fn manifest(&self) -> ToolManifest {
        GisOverlayCore::manifest_for(GisOverlayOp::Sum)
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        GisOverlayCore::validate(GisOverlayOp::Sum, args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        GisOverlayCore::run(GisOverlayOp::Sum, args, ctx)
    }
}

impl Tool for StandardDeviationOverlayTool {
    fn metadata(&self) -> ToolMetadata {
        GisOverlayCore::metadata_for(GisOverlayOp::StandardDeviation)
    }

    fn manifest(&self) -> ToolManifest {
        GisOverlayCore::manifest_for(GisOverlayOp::StandardDeviation)
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        GisOverlayCore::validate(GisOverlayOp::StandardDeviation, args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        GisOverlayCore::run(GisOverlayOp::StandardDeviation, args, ctx)
    }
}

impl Tool for UpdateNodataCellsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "update_nodata_cells",
            display_name: "Update NoData Cells",
            summary: "Assigns NoData cells in input1 from corresponding valid cells in input2.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input1", description: "Primary raster to update.", required: true },
                ToolParamSpec { name: "input2", description: "Secondary raster providing replacement values.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input1".to_string(), json!("input1.tif"));
        defaults.insert("input2".to_string(), json!("input2.tif"));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("update_nodata_cells.tif"));

        ToolManifest {
            id: "update_nodata_cells".to_string(),
            display_name: "Update NoData Cells".to_string(),
            summary: "Assigns NoData cells in input1 from corresponding valid cells in input2.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input1".to_string(), description: "Primary raster to update.".to_string(), required: true },
                ToolParamDescriptor { name: "input2".to_string(), description: "Secondary raster providing replacement values.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "update_nodata_cells_basic".to_string(),
                description: "Replaces NoData cells in input1 using input2 values.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "nodata".to_string(), "overlay".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let input1_path = args
            .get("input1")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'input1' is required".to_string()))?;
        let input2_path = args
            .get("input2")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'input2' is required".to_string()))?;

        let input1 = GisOverlayCore::load_raster(input1_path.trim(), "input1")?;
        let input2 = GisOverlayCore::load_raster(input2_path.trim(), "input2")?;
        if !GisOverlayCore::rasters_share_grid(&input1, &input2) {
            return Err(ToolError::Validation(
                "input rasters must have identical rows, columns, bands, cell sizes, and spatial extent"
                    .to_string(),
            ));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input1_path = args
            .get("input1")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'input1' is required".to_string()))?;
        let input2_path = args
            .get("input2")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'input2' is required".to_string()))?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("running update nodata cells");
        let input1 = GisOverlayCore::load_raster(input1_path.trim(), "input1")?;
        let input2 = GisOverlayCore::load_raster(input2_path.trim(), "input2")?;
        if !GisOverlayCore::rasters_share_grid(&input1, &input2) {
            return Err(ToolError::Validation(
                "input rasters must have identical rows, columns, bands, cell sizes, and spatial extent"
                    .to_string(),
            ));
        }

        let mut output = build_output_like_raster(&input1, DataType::F64);
        let len = output.data.len();
        let nodata = output.nodata;
        let mut out_values = vec![nodata; len];
        let chunk_size = 8192usize;

        ctx.progress.info("updating nodata cells from secondary raster");
        for (chunk_index, out_chunk) in out_values.chunks_mut(chunk_size).enumerate() {
            let start = chunk_index * chunk_size;
            out_chunk.par_iter_mut().enumerate().for_each(|(offset, dst)| {
                let index = start + offset;
                let value1 = input1.data.get_f64(index);
                if input1.is_nodata(value1) {
                    let value2 = input2.data.get_f64(index);
                    *dst = if input2.is_nodata(value2) { nodata } else { value2 };
                } else {
                    *dst = value1;
                }
            });
            let done = ((chunk_index + 1) * chunk_size).min(len);
            ctx.progress.progress(done as f64 / len.max(1) as f64);
        }

        for (index, value) in out_values.iter().enumerate() {
            output.data.set_f64(index, *value);
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(GisOverlayCore::build_result(locator))
    }
}

impl Tool for BufferRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "buffer_raster",
            display_name: "Buffer Raster",
            summary: "Creates a binary buffer zone around non-zero, non-NoData raster cells within a specified distance.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster where non-zero cells are buffer targets.", required: true },
                ToolParamSpec { name: "buffer_size", description: "Buffer distance threshold.", required: true },
                ToolParamSpec { name: "grid_cell_units", description: "If true, interpret buffer_size in grid-cell units instead of map units.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("buffer_size".to_string(), json!(5.0));
        defaults.insert("grid_cell_units".to_string(), json!(false));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("buffer_raster.tif"));

        ToolManifest {
            id: "buffer_raster".to_string(),
            display_name: "Buffer Raster".to_string(),
            summary: "Creates a binary buffer zone around non-zero, non-NoData raster cells within a specified distance.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input raster where non-zero cells are buffer targets.".to_string(), required: true },
                ToolParamDescriptor { name: "buffer_size".to_string(), description: "Buffer distance threshold.".to_string(), required: true },
                ToolParamDescriptor { name: "grid_cell_units".to_string(), description: "If true, interpret buffer_size in grid-cell units instead of map units.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "buffer_raster_basic".to_string(),
                description: "Creates a binary buffer around target cells.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "buffer".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "input")?;
        let size = args
            .get("buffer_size")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'buffer_size' must be a number".to_string()))?;
        if size < 0.0 {
            return Err(ToolError::Validation("buffer_size must be non-negative".to_string()));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = args
            .get("input")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'input' is required".to_string()))?;
        let buffer_size = args
            .get("buffer_size")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'buffer_size' must be a number".to_string()))?;
        let grid_cell_units = args
            .get("grid_cell_units")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let output_path = parse_optional_output_path(args, "output")?;

        let input = GisOverlayCore::load_raster(input_path.trim(), "input")?;
        let mut output = build_output_like_raster(&input, DataType::I16);
        let nodata = output.nodata;
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let band_stride = rows * cols;
        let len = output.data.len();
        let mut out_values = vec![nodata; len];

        let mut rx = vec![0.0f64; len];
        let mut ry = vec![0.0f64; len];
        let inf = f64::INFINITY;
        let dx: [isize; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];
        let dy: [isize; 8] = [0, -1, -1, -1, 0, 1, 1, 1];
        let gx: [f64; 8] = [1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0];
        let gy: [f64; 8] = [0.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0];

        ctx.progress.info("running buffer raster");
        ctx.progress.info("initializing target and background cells");
        for b in 0..bands {
            for row in 0..rows {
                for col in 0..cols {
                    let idx = b * band_stride + row * cols + col;
                    let z = input.data.get_f64(idx);
                    if input.is_nodata(z) {
                        out_values[idx] = nodata;
                    } else if z != 0.0 {
                        out_values[idx] = 0.0;
                    } else {
                        out_values[idx] = inf;
                    }
                }
            }
        }

        ctx.progress.info("propagating nearest-target distances");
        for b in 0..bands {
            for row in 0..rows {
                for col in 0..cols {
                    let idx = b * band_stride + row * cols + col;
                    let z = out_values[idx];
                    if z == 0.0 || z == nodata {
                        continue;
                    }
                    let mut z_min = inf;
                    let mut which = 0usize;
                    for i in 0..4 {
                        let nr = row as isize + dy[i];
                        let nc = col as isize + dx[i];
                        if nr < 0 || nc < 0 || nr >= rows as isize || nc >= cols as isize {
                            continue;
                        }
                        let nidx = b * band_stride + nr as usize * cols + nc as usize;
                        let nz = out_values[nidx];
                        if nz == nodata {
                            continue;
                        }
                        let h = match i {
                            0 => 2.0 * rx[nidx] + 1.0,
                            1 => 2.0 * (rx[nidx] + ry[nidx] + 1.0),
                            2 => 2.0 * ry[nidx] + 1.0,
                            _ => 2.0 * (rx[nidx] + ry[nidx] + 1.0),
                        };
                        let cand = nz + h;
                        if cand < z_min {
                            z_min = cand;
                            which = i;
                        }
                    }
                    if z_min < z {
                        let nr = (row as isize + dy[which]) as usize;
                        let nc = (col as isize + dx[which]) as usize;
                        let nidx = b * band_stride + nr * cols + nc;
                        out_values[idx] = z_min;
                        rx[idx] = rx[nidx] + gx[which];
                        ry[idx] = ry[nidx] + gy[which];
                    }
                }
            }

            for row in (0..rows).rev() {
                for col in (0..cols).rev() {
                    let idx = b * band_stride + row * cols + col;
                    let z = out_values[idx];
                    if z == 0.0 || z == nodata {
                        continue;
                    }
                    let mut z_min = inf;
                    let mut which = 4usize;
                    for i in 4..8 {
                        let nr = row as isize + dy[i];
                        let nc = col as isize + dx[i];
                        if nr < 0 || nc < 0 || nr >= rows as isize || nc >= cols as isize {
                            continue;
                        }
                        let nidx = b * band_stride + nr as usize * cols + nc as usize;
                        let nz = out_values[nidx];
                        if nz == nodata {
                            continue;
                        }
                        let h = match i {
                            5 => 2.0 * (rx[nidx] + ry[nidx] + 1.0),
                            4 => 2.0 * rx[nidx] + 1.0,
                            6 => 2.0 * ry[nidx] + 1.0,
                            _ => 2.0 * (rx[nidx] + ry[nidx] + 1.0),
                        };
                        let cand = nz + h;
                        if cand < z_min {
                            z_min = cand;
                            which = i;
                        }
                    }
                    if z_min < z {
                        let nr = (row as isize + dy[which]) as usize;
                        let nc = (col as isize + dx[which]) as usize;
                        let nidx = b * band_stride + nr * cols + nc;
                        out_values[idx] = z_min;
                        rx[idx] = rx[nidx] + gx[which];
                        ry[idx] = ry[nidx] + gy[which];
                    }
                }
            }
            ctx.progress.progress((b + 1) as f64 / bands.max(1) as f64 * 0.8);
        }

        let cell_size = if grid_cell_units {
            1.0
        } else {
            (input.cell_size_x + input.cell_size_y) / 2.0
        };

        ctx.progress.info("thresholding distance transform to binary buffer");
        for idx in 0..len {
            let val = out_values[idx];
            if val == nodata {
                continue;
            }
            let dist = val.sqrt() * cell_size;
            out_values[idx] = if dist <= buffer_size { 1.0 } else { 0.0 };
        }

        for (idx, value) in out_values.iter().enumerate() {
            output.data.set_f64(idx, *value);
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(GisOverlayCore::build_result(locator))
    }
}

impl Tool for ClumpTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "clump",
            display_name: "Clump",
            summary: "Groups contiguous equal-valued raster cells into unique patch identifiers.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input categorical raster.", required: true },
                ToolParamSpec { name: "diag", description: "If true, include diagonal connectivity (8-neighbour); otherwise use 4-neighbour.", required: false },
                ToolParamSpec { name: "zero_background", description: "If true, treat zero-valued cells as background and keep them as zero in output.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("diag".to_string(), json!(false));
        defaults.insert("zero_background".to_string(), json!(false));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("clump.tif"));

        ToolManifest {
            id: "clump".to_string(),
            display_name: "Clump".to_string(),
            summary: "Groups contiguous equal-valued raster cells into unique patch identifiers.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input categorical raster.".to_string(), required: true },
                ToolParamDescriptor { name: "diag".to_string(), description: "If true, include diagonal connectivity (8-neighbour); otherwise use 4-neighbour.".to_string(), required: false },
                ToolParamDescriptor { name: "zero_background".to_string(), description: "If true, treat zero-valued cells as background and keep them as zero in output.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "clump_basic".to_string(),
                description: "Assigns unique IDs to contiguous patches.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "clump".to_string(), "connectivity".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = args
            .get("input")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'input' is required".to_string()))?;
        let diag = args.get("diag").and_then(|v| v.as_bool()).unwrap_or(false);
        let zero_background = args
            .get("zero_background")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let output_path = parse_optional_output_path(args, "output")?;

        let input = GisOverlayCore::load_raster(input_path.trim(), "input")?;
        let mut output = build_output_like_raster(&input, DataType::I32);
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let band_stride = rows * cols;
        let out_nodata = -999.0;
        let mut out_values = vec![out_nodata; output.data.len()];

        let (dx, dy): (&[isize], &[isize]) = if diag {
            (&[1, 1, 1, 0, -1, -1, -1, 0], &[-1, 0, 1, 1, 1, 0, -1, -1])
        } else {
            (&[0, 1, 0, -1], &[-1, 0, 1, 0])
        };

        ctx.progress.info("running clump");
        let mut fid = 0.0;
        let mut solved = 0usize;
        let total = out_values.len();
        let mut stack: Vec<(usize, usize, usize)> = Vec::new();

        for b in 0..bands {
            for row in 0..rows {
                for col in 0..cols {
                    let idx = b * band_stride + row * cols + col;
                    let zin = input.data.get_f64(idx);
                    let zout = out_values[idx];
                    if input.is_nodata(zin) {
                        solved += 1;
                        continue;
                    }
                    if zero_background && zin == 0.0 {
                        out_values[idx] = 0.0;
                        solved += 1;
                        continue;
                    }
                    if zout != out_nodata {
                        continue;
                    }

                    fid += 1.0;
                    out_values[idx] = fid;
                    solved += 1;
                    stack.push((b, row, col));

                    while let Some((cb, cr, cc)) = stack.pop() {
                        let cidx = cb * band_stride + cr * cols + cc;
                        let class_val = input.data.get_f64(cidx);
                        for k in 0..dx.len() {
                            let nr = cr as isize + dy[k];
                            let nc = cc as isize + dx[k];
                            if nr < 0 || nc < 0 || nr >= rows as isize || nc >= cols as isize {
                                continue;
                            }
                            let nidx = cb * band_stride + nr as usize * cols + nc as usize;
                            if out_values[nidx] != out_nodata {
                                continue;
                            }
                            let nval = input.data.get_f64(nidx);
                            if input.is_nodata(nval) {
                                continue;
                            }
                            if zero_background && nval == 0.0 {
                                continue;
                            }
                            if nval == class_val {
                                out_values[nidx] = fid;
                                solved += 1;
                                stack.push((cb, nr as usize, nc as usize));
                            }
                        }
                    }
                }
                if row % 32 == 0 {
                    ctx.progress.progress(solved as f64 / total.max(1) as f64);
                }
            }
        }

        output.nodata = out_nodata;
        for (idx, value) in out_values.iter().enumerate() {
            output.data.set_f64(idx, *value);
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(GisOverlayCore::build_result(locator))
    }
}

fn typed_raster_output(path: String) -> serde_json::Value {
    json!({"__wbw_type__": "raster", "path": path, "active_band": 0})
}

fn build_dual_raster_result(left_key: &str, left_path: String, right_key: &str, right_path: String) -> ToolRunResult {
    let left = typed_raster_output(left_path);
    let right = typed_raster_output(right_path);
    let mut outputs = BTreeMap::new();
    outputs.insert(left_key.to_string(), left.clone());
    outputs.insert(right_key.to_string(), right.clone());
    outputs.insert("__wbw_type__".to_string(), json!("tuple"));
    outputs.insert("items".to_string(), json!([left, right]));
    ToolRunResult { outputs }
}

fn grid_index(row: usize, col: usize, cols: usize) -> usize {
    row * cols + col
}

fn pointer_to_dir_idx(pointer: f64) -> Option<usize> {
    match pointer as i64 {
        1 => Some(0),
        2 => Some(1),
        4 => Some(2),
        8 => Some(3),
        16 => Some(4),
        32 => Some(5),
        64 => Some(6),
        128 => Some(7),
        _ => None,
    }
}

fn euclidean_transform(input: &Raster) -> (Vec<f64>, Vec<f64>) {
    let rows = input.rows;
    let cols = input.cols;
    let bands = input.bands;
    let band_stride = rows * cols;
    let len = input.data.len();
    let nodata = input.nodata;
    let inf = f64::INFINITY;
    let dx: [isize; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];
    let dy: [isize; 8] = [0, -1, -1, -1, 0, 1, 1, 1];
    let gx: [f64; 8] = [1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0];
    let gy: [f64; 8] = [0.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0];

    let mut dist_sq = vec![inf; len];
    let mut alloc = vec![inf; len];
    let mut rx = vec![0.0f64; len];
    let mut ry = vec![0.0f64; len];

    for b in 0..bands {
        for row in 0..rows {
            for col in 0..cols {
                let idx = b * band_stride + grid_index(row, col, cols);
                let z = input.data.get_f64(idx);
                if input.is_nodata(z) {
                    dist_sq[idx] = nodata;
                    alloc[idx] = nodata;
                } else if z != 0.0 {
                    dist_sq[idx] = 0.0;
                    alloc[idx] = z;
                }
            }
        }
    }

    for b in 0..bands {
        for row in 0..rows {
            for col in 0..cols {
                let idx = b * band_stride + grid_index(row, col, cols);
                let z = dist_sq[idx];
                if z == 0.0 || z == nodata {
                    continue;
                }
                let mut z_min = inf;
                let mut which = 0usize;
                for i in 0..4 {
                    let nr = row as isize + dy[i];
                    let nc = col as isize + dx[i];
                    if nr < 0 || nc < 0 || nr >= rows as isize || nc >= cols as isize {
                        continue;
                    }
                    let nidx = b * band_stride + grid_index(nr as usize, nc as usize, cols);
                    let nz = dist_sq[nidx];
                    if nz == nodata {
                        continue;
                    }
                    let h = match i {
                        0 => 2.0 * rx[nidx] + 1.0,
                        1 => 2.0 * (rx[nidx] + ry[nidx] + 1.0),
                        2 => 2.0 * ry[nidx] + 1.0,
                        _ => 2.0 * (rx[nidx] + ry[nidx] + 1.0),
                    };
                    let cand = nz + h;
                    if cand < z_min {
                        z_min = cand;
                        which = i;
                    }
                }
                if z_min < z {
                    let nr = (row as isize + dy[which]) as usize;
                    let nc = (col as isize + dx[which]) as usize;
                    let nidx = b * band_stride + grid_index(nr, nc, cols);
                    dist_sq[idx] = z_min;
                    rx[idx] = rx[nidx] + gx[which];
                    ry[idx] = ry[nidx] + gy[which];
                    alloc[idx] = alloc[nidx];
                }
            }
        }

        for row in (0..rows).rev() {
            for col in (0..cols).rev() {
                let idx = b * band_stride + grid_index(row, col, cols);
                let z = dist_sq[idx];
                if z == 0.0 || z == nodata {
                    continue;
                }
                let mut z_min = inf;
                let mut which = 4usize;
                for i in 4..8 {
                    let nr = row as isize + dy[i];
                    let nc = col as isize + dx[i];
                    if nr < 0 || nc < 0 || nr >= rows as isize || nc >= cols as isize {
                        continue;
                    }
                    let nidx = b * band_stride + grid_index(nr as usize, nc as usize, cols);
                    let nz = dist_sq[nidx];
                    if nz == nodata {
                        continue;
                    }
                    let h = match i {
                        5 => 2.0 * (rx[nidx] + ry[nidx] + 1.0),
                        4 => 2.0 * rx[nidx] + 1.0,
                        6 => 2.0 * ry[nidx] + 1.0,
                        _ => 2.0 * (rx[nidx] + ry[nidx] + 1.0),
                    };
                    let cand = nz + h;
                    if cand < z_min {
                        z_min = cand;
                        which = i;
                    }
                }
                if z_min < z {
                    let nr = (row as isize + dy[which]) as usize;
                    let nc = (col as isize + dx[which]) as usize;
                    let nidx = b * band_stride + grid_index(nr, nc, cols);
                    dist_sq[idx] = z_min;
                    rx[idx] = rx[nidx] + gx[which];
                    ry[idx] = ry[nidx] + gy[which];
                    alloc[idx] = alloc[nidx];
                }
            }
        }
    }

    (dist_sq, alloc)
}

#[derive(Clone, Copy)]
struct MinHeapCell {
    row: usize,
    col: usize,
    priority: f64,
}

impl PartialEq for MinHeapCell {
    fn eq(&self, other: &Self) -> bool {
        self.priority.total_cmp(&other.priority) == Ordering::Equal
            && self.row == other.row
            && self.col == other.col
    }
}

impl Eq for MinHeapCell {}

impl PartialOrd for MinHeapCell {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(other.priority.total_cmp(&self.priority))
    }
}

impl Ord for MinHeapCell {
    fn cmp(&self, other: &Self) -> Ordering {
        other.priority.total_cmp(&self.priority)
    }
}

impl Tool for EuclideanDistanceTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "euclidean_distance",
            display_name: "Euclidean Distance",
            summary: "Computes Euclidean distance to nearest non-zero target cell in a raster.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster with non-zero target cells.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("euclidean_distance.tif"));
        ToolManifest {
            id: "euclidean_distance".to_string(),
            display_name: "Euclidean Distance".to_string(),
            summary: "Computes Euclidean distance to nearest non-zero target cell in a raster.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input raster with non-zero target cells.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "euclidean_distance_basic".to_string(), description: "Computes distance to nearest target cells.".to_string(), args: example_args }],
            tags: vec!["raster".to_string(), "gis".to_string(), "distance".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_required_raster_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("running euclidean distance");
        let (dist_sq, _) = euclidean_transform(&input);

        let mut output = build_output_like_raster(&input, DataType::F64);
        let cell_size = (input.cell_size_x + input.cell_size_y) / 2.0;
        for idx in 0..output.data.len() {
            let z = input.data.get_f64(idx);
            if input.is_nodata(z) {
                output.data.set_f64(idx, output.nodata);
            } else {
                output.data.set_f64(idx, dist_sq[idx].sqrt() * cell_size);
            }
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        Ok(GisOverlayCore::build_result(locator))
    }
}

impl Tool for EuclideanAllocationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "euclidean_allocation",
            display_name: "Euclidean Allocation",
            summary: "Assigns each valid cell the value of its nearest non-zero target cell.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster with non-zero target cells.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("euclidean_allocation.tif"));
        ToolManifest {
            id: "euclidean_allocation".to_string(),
            display_name: "Euclidean Allocation".to_string(),
            summary: "Assigns each valid cell the value of its nearest non-zero target cell.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input raster with non-zero target cells.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "euclidean_allocation_basic".to_string(), description: "Computes nearest-target allocation raster.".to_string(), args: example_args }],
            tags: vec!["raster".to_string(), "gis".to_string(), "distance".to_string(), "allocation".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_required_raster_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("running euclidean allocation");
        let (_, alloc) = euclidean_transform(&input);

        let mut output = build_output_like_raster(&input, DataType::F64);
        for (idx, value) in alloc.iter().enumerate() {
            output.data.set_f64(idx, *value);
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        Ok(GisOverlayCore::build_result(locator))
    }
}

impl Tool for CostDistanceTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "cost_distance",
            display_name: "Cost Distance",
            summary: "Computes accumulated travel cost and backlink rasters from source and cost surfaces.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "source", description: "Source raster with positive source cells.", required: true },
                ToolParamSpec { name: "cost", description: "Cost/friction raster.", required: true },
                ToolParamSpec { name: "output", description: "Optional accumulated-cost output raster path.", required: false },
                ToolParamSpec { name: "backlink_output", description: "Optional backlink output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("source".to_string(), json!("source.tif"));
        defaults.insert("cost".to_string(), json!("cost.tif"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("cost_accum.tif"));
        example_args.insert("backlink_output".to_string(), json!("cost_backlink.tif"));

        ToolManifest {
            id: "cost_distance".to_string(),
            display_name: "Cost Distance".to_string(),
            summary: "Computes accumulated travel cost and backlink rasters from source and cost surfaces.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "source".to_string(), description: "Source raster with positive source cells.".to_string(), required: true },
                ToolParamDescriptor { name: "cost".to_string(), description: "Cost/friction raster.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional accumulated-cost output raster path.".to_string(), required: false },
                ToolParamDescriptor { name: "backlink_output".to_string(), description: "Optional backlink output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "cost_distance_basic".to_string(), description: "Computes cost accumulation and backlink rasters.".to_string(), args: example_args }],
            tags: vec!["raster".to_string(), "gis".to_string(), "distance".to_string(), "cost".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let source = load_required_raster_arg(args, "source")?;
        let cost = load_required_raster_arg(args, "cost")?;
        if !GisOverlayCore::rasters_share_grid(&source, &cost) {
            return Err(ToolError::Validation(
                "source and cost rasters must have identical rows, columns, bands, cell sizes, and spatial extent".to_string(),
            ));
        }
        let _ = parse_optional_output_path(args, "output")?;
        let _ = parse_optional_output_path(args, "backlink_output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let source = load_required_raster_arg(args, "source")?;
        let cost = load_required_raster_arg(args, "cost")?;
        if !GisOverlayCore::rasters_share_grid(&source, &cost) {
            return Err(ToolError::Validation(
                "source and cost rasters must have identical rows, columns, bands, cell sizes, and spatial extent".to_string(),
            ));
        }

        let accum_path = parse_optional_output_path(args, "output")?;
        let backlink_path = parse_optional_output_path(args, "backlink_output")?;

        let rows = source.rows;
        let cols = source.cols;
        let bands = source.bands;
        let band_stride = rows * cols;
        let diag_cell = (source.cell_size_x * source.cell_size_x + source.cell_size_y * source.cell_size_y).sqrt();
        let dist = [diag_cell, source.cell_size_x, diag_cell, source.cell_size_y, diag_cell, source.cell_size_x, diag_cell, source.cell_size_y];
        let dx: [isize; 8] = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy: [isize; 8] = [-1, 0, 1, 1, 1, 0, -1, -1];
        let backlink_dir = [16.0, 32.0, 64.0, 128.0, 1.0, 2.0, 4.0, 8.0];

        let mut accum = build_output_like_raster(&cost, DataType::F64);
        let mut backlink = build_output_like_raster(&cost, DataType::I16);
        let inf = (i32::MAX - 1) as f64;
        for idx in 0..accum.data.len() {
            accum.data.set_f64(idx, inf);
            backlink.data.set_f64(idx, 0.0);
        }

        let mut heap = BinaryHeap::new();
        for b in 0..bands {
            for row in 0..rows {
                for col in 0..cols {
                    let idx = b * band_stride + grid_index(row, col, cols);
                    let cz = cost.data.get_f64(idx);
                    if cost.is_nodata(cz) {
                        accum.data.set_f64(idx, accum.nodata);
                        backlink.data.set_f64(idx, backlink.nodata);
                        continue;
                    }
                    let sz = source.data.get_f64(idx);
                    if !source.is_nodata(sz) && sz > 0.0 {
                        accum.data.set_f64(idx, 0.0);
                        backlink.data.set_f64(idx, 0.0);
                        heap.push(MinHeapCell { row: b * rows + row, col, priority: 0.0 });
                    }
                }
            }
        }

        ctx.progress.info("running cost distance");
        let total = rows * cols * bands;
        let mut touched = 0usize;
        while let Some(cell) = heap.pop() {
            let band = cell.row / rows;
            let row = cell.row % rows;
            let idx = band * band_stride + grid_index(row, cell.col, cols);
            let accum_val = accum.data.get_f64(idx);
            if cell.priority > accum_val {
                continue;
            }
            touched += 1;
            let c1 = cost.data.get_f64(idx);
            for n in 0..8 {
                let nr = row as isize + dy[n];
                let nc = cell.col as isize + dx[n];
                if nr < 0 || nc < 0 || nr >= rows as isize || nc >= cols as isize {
                    continue;
                }
                let nidx = band * band_stride + grid_index(nr as usize, nc as usize, cols);
                if accum.data.get_f64(nidx) == accum.nodata {
                    continue;
                }
                let c2 = cost.data.get_f64(nidx);
                if cost.is_nodata(c2) {
                    continue;
                }
                let new_cost = accum_val + ((c1 + c2) / 2.0) * dist[n];
                if new_cost < accum.data.get_f64(nidx) {
                    accum.data.set_f64(nidx, new_cost);
                    backlink.data.set_f64(nidx, backlink_dir[n]);
                    heap.push(MinHeapCell { row: band * rows + nr as usize, col: nc as usize, priority: new_cost });
                }
            }
            if touched % 10000 == 0 {
                ctx.progress.progress((touched as f64 / total.max(1) as f64).min(0.98));
            }
        }

        let accum_locator = GisOverlayCore::store_or_write_output(accum, accum_path, ctx)?;
        let backlink_locator = GisOverlayCore::store_or_write_output(backlink, backlink_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(build_dual_raster_result("cost_accum", accum_locator, "backlink", backlink_locator))
    }
}

impl Tool for CostAllocationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "cost_allocation",
            display_name: "Cost Allocation",
            summary: "Assigns each cell to a source region using a backlink raster from cost distance analysis.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "source", description: "Source raster with positive source cells.", required: true },
                ToolParamSpec { name: "backlink", description: "Backlink raster from cost_distance.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("source".to_string(), json!("source.tif"));
        defaults.insert("backlink".to_string(), json!("cost_backlink.tif"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("cost_allocation.tif"));
        ToolManifest {
            id: "cost_allocation".to_string(),
            display_name: "Cost Allocation".to_string(),
            summary: "Assigns each cell to a source region using a backlink raster from cost distance analysis.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "source".to_string(), description: "Source raster with positive source cells.".to_string(), required: true },
                ToolParamDescriptor { name: "backlink".to_string(), description: "Backlink raster from cost_distance.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "cost_allocation_basic".to_string(), description: "Allocates cells to source IDs using backlink connectivity.".to_string(), args: example_args }],
            tags: vec!["raster".to_string(), "gis".to_string(), "distance".to_string(), "cost".to_string(), "allocation".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let source = load_required_raster_arg(args, "source")?;
        let backlink = load_required_raster_arg(args, "backlink")?;
        if !GisOverlayCore::rasters_share_grid(&source, &backlink) {
            return Err(ToolError::Validation(
                "source and backlink rasters must have identical rows, columns, bands, cell sizes, and spatial extent".to_string(),
            ));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let source = load_required_raster_arg(args, "source")?;
        let backlink = load_required_raster_arg(args, "backlink")?;
        if !GisOverlayCore::rasters_share_grid(&source, &backlink) {
            return Err(ToolError::Validation(
                "source and backlink rasters must have identical rows, columns, bands, cell sizes, and spatial extent".to_string(),
            ));
        }
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("running cost allocation");
        let rows = source.rows;
        let cols = source.cols;
        let bands = source.bands;
        let band_stride = rows * cols;
        let low = f64::MIN;
        let mut output = build_output_like_raster(&source, DataType::F64);
        for idx in 0..output.data.len() {
            output.data.set_f64(idx, low);
        }

        let dx: [isize; 8] = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy: [isize; 8] = [-1, 0, 1, 1, 1, 0, -1, -1];

        for b in 0..bands {
            for row in 0..rows {
                for col in 0..cols {
                    let idx = b * band_stride + grid_index(row, col, cols);
                    let p = backlink.data.get_f64(idx);
                    if backlink.is_nodata(p) {
                        output.data.set_f64(idx, source.nodata);
                        continue;
                    }
                    let s = source.data.get_f64(idx);
                    if !source.is_nodata(s) && s > 0.0 {
                        output.data.set_f64(idx, s);
                    }
                }
            }
        }

        let max_steps = rows * cols;
        for b in 0..bands {
            for row in 0..rows {
                for col in 0..cols {
                    let idx = b * band_stride + grid_index(row, col, cols);
                    if output.data.get_f64(idx) != low {
                        continue;
                    }

                    let mut x = col as isize;
                    let mut y = row as isize;
                    let mut found = source.nodata;
                    let mut steps = 0usize;
                    while steps < max_steps {
                        let cidx = b * band_stride + grid_index(y as usize, x as usize, cols);
                        let outv = output.data.get_f64(cidx);
                        if outv != low {
                            found = outv;
                            break;
                        }
                        let dir = backlink.data.get_f64(cidx);
                        let Some(di) = pointer_to_dir_idx(dir) else {
                            break;
                        };
                        let nx = x + dx[di];
                        let ny = y + dy[di];
                        if nx < 0 || ny < 0 || nx >= cols as isize || ny >= rows as isize {
                            break;
                        }
                        x = nx;
                        y = ny;
                        steps += 1;
                    }

                    let mut x = col as isize;
                    let mut y = row as isize;
                    let mut steps = 0usize;
                    while steps < max_steps {
                        let cidx = b * band_stride + grid_index(y as usize, x as usize, cols);
                        if output.data.get_f64(cidx) != low {
                            break;
                        }
                        output.data.set_f64(cidx, found);
                        let dir = backlink.data.get_f64(cidx);
                        let Some(di) = pointer_to_dir_idx(dir) else {
                            break;
                        };
                        let nx = x + dx[di];
                        let ny = y + dy[di];
                        if nx < 0 || ny < 0 || nx >= cols as isize || ny >= rows as isize {
                            break;
                        }
                        x = nx;
                        y = ny;
                        steps += 1;
                    }
                }
            }
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        Ok(GisOverlayCore::build_result(locator))
    }
}

impl Tool for CostPathwayTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "cost_pathway",
            display_name: "Cost Pathway",
            summary: "Traces least-cost pathways from destination cells using a backlink raster.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "destination", description: "Destination raster with positive destination cells.", required: true },
                ToolParamSpec { name: "backlink", description: "Backlink raster from cost_distance.", required: true },
                ToolParamSpec { name: "zero_background", description: "If true, output background is zero instead of NoData.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("destination".to_string(), json!("destination.tif"));
        defaults.insert("backlink".to_string(), json!("cost_backlink.tif"));
        defaults.insert("zero_background".to_string(), json!(false));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("cost_pathway.tif"));
        ToolManifest {
            id: "cost_pathway".to_string(),
            display_name: "Cost Pathway".to_string(),
            summary: "Traces least-cost pathways from destination cells using a backlink raster.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "destination".to_string(), description: "Destination raster with positive destination cells.".to_string(), required: true },
                ToolParamDescriptor { name: "backlink".to_string(), description: "Backlink raster from cost_distance.".to_string(), required: true },
                ToolParamDescriptor { name: "zero_background".to_string(), description: "If true, output background is zero instead of NoData.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "cost_pathway_basic".to_string(), description: "Traces least-cost pathways from destinations.".to_string(), args: example_args }],
            tags: vec!["raster".to_string(), "gis".to_string(), "distance".to_string(), "cost".to_string(), "pathway".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let destination = load_required_raster_arg(args, "destination")?;
        let backlink = load_required_raster_arg(args, "backlink")?;
        if !GisOverlayCore::rasters_share_grid(&destination, &backlink) {
            return Err(ToolError::Validation(
                "destination and backlink rasters must have identical rows, columns, bands, cell sizes, and spatial extent".to_string(),
            ));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let destination = load_required_raster_arg(args, "destination")?;
        let backlink = load_required_raster_arg(args, "backlink")?;
        if !GisOverlayCore::rasters_share_grid(&destination, &backlink) {
            return Err(ToolError::Validation(
                "destination and backlink rasters must have identical rows, columns, bands, cell sizes, and spatial extent".to_string(),
            ));
        }
        let zero_background = args.get("zero_background").and_then(|v| v.as_bool()).unwrap_or(false);
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("running cost pathway");
        let rows = destination.rows;
        let cols = destination.cols;
        let bands = destination.bands;
        let band_stride = rows * cols;
        let background = if zero_background { 0.0 } else { destination.nodata };

        let mut output = build_output_like_raster(&destination, DataType::F64);
        for idx in 0..output.data.len() {
            output.data.set_f64(idx, background);
        }

        let dx: [isize; 8] = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy: [isize; 8] = [-1, 0, 1, 1, 1, 0, -1, -1];
        let max_steps = rows * cols;

        for b in 0..bands {
            for row in 0..rows {
                for col in 0..cols {
                    let idx = b * band_stride + grid_index(row, col, cols);
                    if backlink.is_nodata(backlink.data.get_f64(idx)) {
                        output.data.set_f64(idx, destination.nodata);
                        continue;
                    }
                    let dest = destination.data.get_f64(idx);
                    if destination.is_nodata(dest) || dest <= 0.0 {
                        continue;
                    }

                    let mut x = col as isize;
                    let mut y = row as isize;
                    let mut steps = 0usize;
                    while steps < max_steps {
                        let cidx = b * band_stride + grid_index(y as usize, x as usize, cols);
                        let current = output.data.get_f64(cidx);
                        if (current == background) || output.is_nodata(current) {
                            output.data.set_f64(cidx, 1.0);
                        } else {
                            output.data.set_f64(cidx, current + 1.0);
                        }

                        let dir = backlink.data.get_f64(cidx);
                        let Some(di) = pointer_to_dir_idx(dir) else {
                            break;
                        };
                        let nx = x + dx[di];
                        let ny = y + dy[di];
                        if nx < 0 || ny < 0 || nx >= cols as isize || ny >= rows as isize {
                            break;
                        }
                        x = nx;
                        y = ny;
                        steps += 1;
                    }
                }
            }
            ctx.progress.progress((b + 1) as f64 / bands.max(1) as f64);
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        Ok(GisOverlayCore::build_result(locator))
    }
}

fn parse_string_list_value(value: &serde_json::Value, key: &str) -> Result<Vec<String>, ToolError> {
    if let Some(s) = value.as_str() {
        let out = s
            .split([',', ';'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        if out.is_empty() {
            return Err(ToolError::Validation(format!("parameter '{}' contains no values", key)));
        }
        return Ok(out);
    }
    if let Some(arr) = value.as_array() {
        let mut out = Vec::with_capacity(arr.len());
        for (index, item) in arr.iter().enumerate() {
            let Some(s) = item.as_str() else {
                return Err(ToolError::Validation(format!(
                    "parameter '{}' array element {} must be a string",
                    key, index
                )));
            };
            let s = s.trim();
            if s.is_empty() {
                return Err(ToolError::Validation(format!(
                    "parameter '{}' array element {} is empty",
                    key, index
                )));
            }
            out.push(s.to_string());
        }
        if out.is_empty() {
            return Err(ToolError::Validation(format!("parameter '{}' contains no values", key)));
        }
        return Ok(out);
    }
    Err(ToolError::Validation(format!(
        "parameter '{}' must be a string list or array",
        key
    )))
}

fn parse_f64_list_arg(args: &ToolArgs, key: &str) -> Result<Vec<f64>, ToolError> {
    let value = args
        .get(key)
        .ok_or_else(|| ToolError::Validation(format!("parameter '{}' is required", key)))?;
    if let Some(arr) = value.as_array() {
        let mut out = Vec::with_capacity(arr.len());
        for (index, item) in arr.iter().enumerate() {
            let Some(v) = item.as_f64() else {
                return Err(ToolError::Validation(format!(
                    "parameter '{}' array element {} must be numeric",
                    key, index
                )));
            };
            out.push(v);
        }
        if out.is_empty() {
            return Err(ToolError::Validation(format!("parameter '{}' contains no values", key)));
        }
        return Ok(out);
    }
    if let Some(s) = value.as_str() {
        let mut out = Vec::new();
        for token in s.split([',', ';']).map(str::trim).filter(|s| !s.is_empty()) {
            let parsed = token.parse::<f64>().map_err(|_| {
                ToolError::Validation(format!("parameter '{}' contains non-numeric value '{}'", key, token))
            })?;
            out.push(parsed);
        }
        if out.is_empty() {
            return Err(ToolError::Validation(format!("parameter '{}' contains no values", key)));
        }
        return Ok(out);
    }
    Err(ToolError::Validation(format!("parameter '{}' must be a numeric array or delimited string", key)))
}

fn parse_bool_list_arg(args: &ToolArgs, key: &str) -> Result<Vec<bool>, ToolError> {
    let value = args
        .get(key)
        .ok_or_else(|| ToolError::Validation(format!("parameter '{}' is required", key)))?;
    if let Some(arr) = value.as_array() {
        let mut out = Vec::with_capacity(arr.len());
        for (index, item) in arr.iter().enumerate() {
            let Some(v) = item.as_bool() else {
                return Err(ToolError::Validation(format!(
                    "parameter '{}' array element {} must be boolean",
                    key, index
                )));
            };
            out.push(v);
        }
        return Ok(out);
    }
    if let Some(s) = value.as_str() {
        let mut out = Vec::new();
        for token in s.split([',', ';']).map(str::trim).filter(|s| !s.is_empty()) {
            match token.to_ascii_lowercase().as_str() {
                "true" | "1" | "yes" => out.push(true),
                "false" | "0" | "no" => out.push(false),
                _ => {
                    return Err(ToolError::Validation(format!(
                        "parameter '{}' contains non-boolean value '{}'",
                        key, token
                    )))
                }
            }
        }
        return Ok(out);
    }
    Err(ToolError::Validation(format!("parameter '{}' must be a boolean array or delimited string", key)))
}

fn parse_optional_raster_list_arg(args: &ToolArgs, key: &str) -> Result<Vec<String>, ToolError> {
    let Some(value) = args.get(key) else {
        return Ok(Vec::new());
    };
    parse_string_list_value(value, key)
}

fn normalize_weights(weights: &[f64]) -> Result<Vec<f64>, ToolError> {
    if weights.is_empty() {
        return Err(ToolError::Validation("weights must contain at least one value".to_string()));
    }
    let sum = weights.iter().copied().sum::<f64>();
    if !sum.is_finite() || sum.abs() <= f64::EPSILON {
        return Err(ToolError::Validation("weights must sum to a non-zero finite value".to_string()));
    }
    Ok(weights.iter().map(|w| *w / sum).collect())
}

fn load_required_raster_arg(args: &ToolArgs, key: &str) -> Result<Raster, ToolError> {
    let path = args
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::Validation(format!("parameter '{}' is required", key)))?;
    GisOverlayCore::load_raster(path.trim(), key)
}

fn load_optional_raster_arg(args: &ToolArgs, key: &str) -> Result<Option<Raster>, ToolError> {
    let Some(path) = args.get(key).and_then(|v| v.as_str()) else {
        return Ok(None);
    };
    Ok(Some(GisOverlayCore::load_raster(path.trim(), key)?))
}

fn load_vector_arg(args: &ToolArgs, key: &str) -> Result<wbvector::Layer, ToolError> {
    let path = args
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::Validation(format!("parameter '{}' is required", key)))?;
    wbvector::read(path.trim())
        .map_err(|e| ToolError::Execution(format!("failed reading {} vector: {}", key, e)))
}

fn read_vector_layer_aligned_to_raster(
    raster: &Raster,
    path: &str,
    input_name: &str,
) -> Result<wbvector::Layer, ToolError> {
    let layer = wbvector::read(path).map_err(|e| {
        ToolError::Validation(format!(
            "failed reading {} vector '{}': {}",
            input_name, path, e
        ))
    })?;

    let raster_epsg = raster.crs.epsg;
    let raster_wkt = raster.crs.wkt.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let layer_epsg = layer.crs_epsg();
    let layer_wkt = layer.crs_wkt().map(str::trim).filter(|s| !s.is_empty());

    if raster_epsg.is_none() && raster_wkt.is_none() {
        return Ok(layer);
    }

    if layer_epsg.is_none() && layer_wkt.is_none() {
        return Err(ToolError::Validation(format!(
            "{} vector has no CRS metadata; cannot verify alignment with raster CRS",
            input_name
        )));
    }

    let epsg_matches = raster_epsg.is_some() && layer_epsg == raster_epsg;
    let wkt_matches = match (raster_wkt, layer_wkt) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    };
    if epsg_matches || wkt_matches {
        return Ok(layer);
    }

    if let Some(dst_epsg) = raster_epsg {
        let reprojected = layer.reproject_to_epsg(dst_epsg).map_err(|e| {
            ToolError::Validation(format!(
                "{} vector CRS does not match raster CRS; automatic reprojection to EPSG:{} failed: {}",
                input_name, dst_epsg, e
            ))
        })?;
        return Ok(reprojected);
    }

    Err(ToolError::Validation(format!(
        "{} vector CRS does not match raster CRS and raster has no EPSG code for automatic reprojection",
        input_name
    )))
}

fn collect_layer_polygons_recursive(
    geometry: &wbvector::Geometry,
    out: &mut Vec<(wbvector::Ring, Vec<wbvector::Ring>)>,
) -> Result<(), ToolError> {
    match geometry {
        wbvector::Geometry::Polygon { exterior, interiors } => {
            out.push((exterior.clone(), interiors.clone()));
        }
        wbvector::Geometry::MultiPolygon(parts) => {
            for (exterior, interiors) in parts {
                out.push((exterior.clone(), interiors.clone()));
            }
        }
        wbvector::Geometry::GeometryCollection(parts) => {
            for part in parts {
                collect_layer_polygons_recursive(part, out)?;
            }
        }
        _ => {
            return Err(ToolError::Validation(
                "input polygons vector must only contain polygonal geometries".to_string(),
            ))
        }
    }
    Ok(())
}

fn collect_layer_polygons(
    layer: &wbvector::Layer,
) -> Result<Vec<(wbvector::Ring, Vec<wbvector::Ring>)>, ToolError> {
    let mut polygons = Vec::<(wbvector::Ring, Vec<wbvector::Ring>)>::new();
    for feature in &layer.features {
        if let Some(geometry) = &feature.geometry {
            collect_layer_polygons_recursive(geometry, &mut polygons)?;
        }
    }
    if polygons.is_empty() {
        return Err(ToolError::Validation(
            "input polygons vector must contain at least one polygon feature".to_string(),
        ));
    }
    Ok(polygons)
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

fn polygon_world_bbox(exterior: &wbvector::Ring) -> Option<(f64, f64, f64, f64)> {
    if exterior.0.is_empty() {
        return None;
    }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for p in &exterior.0 {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }
    Some((min_x, min_y, max_x, max_y))
}

fn polygon_bbox_pixels(
    raster: &Raster,
    exterior: &wbvector::Ring,
) -> Option<(usize, usize, usize, usize)> {
    let (min_x, min_y, max_x, max_y) = polygon_world_bbox(exterior)?;
    let ext = raster.extent();
    if max_x < ext.x_min || min_x > ext.x_max || max_y < ext.y_min || min_y > ext.y_max {
        return None;
    }

    let clip_min_x = min_x.max(ext.x_min);
    let clip_max_x = max_x.min(ext.x_max);
    let clip_min_y = min_y.max(ext.y_min);
    let clip_max_y = max_y.min(ext.y_max);

    let cols_max = raster.cols as isize - 1;
    let rows_max = raster.rows as isize - 1;
    let cmin = (((clip_min_x - raster.x_min) / raster.cell_size_x).floor() as isize).clamp(0, cols_max) as usize;
    let cmax = (((clip_max_x - raster.x_min) / raster.cell_size_x).floor() as isize).clamp(0, cols_max) as usize;
    let rmin = (((raster.y_max() - clip_max_y) / raster.cell_size_y).floor() as isize).clamp(0, rows_max) as usize;
    let rmax = (((raster.y_max() - clip_min_y) / raster.cell_size_y).floor() as isize).clamp(0, rows_max) as usize;

    Some((rmin.min(rmax), cmin.min(cmax), rmax.max(rmin), cmax.max(cmin)))
}

fn parse_required_vector_path_arg(args: &ToolArgs, key: &str) -> Result<String, ToolError> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ToolError::Validation(format!("parameter '{}' is required", key)))
}

fn vector_crs_to_raster_crs(layer: &wbvector::Layer) -> CrsInfo {
    if let Some(crs) = &layer.crs {
        if let Some(epsg) = crs.epsg {
            return CrsInfo::from_epsg(epsg);
        }
        if let Some(wkt) = &crs.wkt {
            return CrsInfo::from_wkt(wkt.clone());
        }
    }
    CrsInfo::default()
}

fn has_vector_crs(layer: &wbvector::Layer) -> bool {
    layer
        .crs
        .as_ref()
        .map(|crs| crs.epsg.is_some() || crs.wkt.as_deref().map(|w| !w.trim().is_empty()).unwrap_or(false))
        .unwrap_or(false)
}

fn collect_geometry_coords<'a>(geometry: &'a wbvector::Geometry, out: &mut Vec<&'a wbvector::Coord>) {
    match geometry {
        wbvector::Geometry::Point(coord) => out.push(coord),
        wbvector::Geometry::MultiPoint(coords) => out.extend(coords.iter()),
        wbvector::Geometry::GeometryCollection(geometries) => {
            for geometry in geometries {
                collect_geometry_coords(geometry, out);
            }
        }
        _ => {}
    }
}

fn collect_point_samples(
    layer: &wbvector::Layer,
    field_name: Option<&str>,
    use_z: bool,
) -> Result<Vec<(f64, f64, f64)>, ToolError> {
    let field_idx = if use_z {
        None
    } else if let Some(name) = field_name {
        layer.schema.field_index(name)
    } else {
        None
    };

    let field_is_numeric = field_idx
        .and_then(|idx| layer.schema.fields().get(idx))
        .map(|field| matches!(field.field_type, wbvector::FieldType::Integer | wbvector::FieldType::Float))
        .unwrap_or(false);

    let mut samples = Vec::new();
    for feature in &layer.features {
        let Some(geometry) = &feature.geometry else {
            continue;
        };

        let mut coords = Vec::new();
        collect_geometry_coords(geometry, &mut coords);
        if coords.is_empty() {
            continue;
        }

        let attr_value = if use_z {
            None
        } else if field_is_numeric {
            field_idx
                .and_then(|idx| feature.attributes.get(idx))
                .and_then(|value| value.as_f64())
        } else {
            Some(feature.fid as f64)
        };

        for coord in coords {
            let value = if use_z {
                coord.z.ok_or_else(|| {
                    ToolError::Validation(
                        "points geometry does not contain Z values required by use_z=true".to_string(),
                    )
                })?
            } else {
                attr_value.unwrap_or(feature.fid as f64)
            };
            samples.push((coord.x, coord.y, value));
        }
    }

    if samples.is_empty() {
        return Err(ToolError::Validation(
            "points input must contain at least one usable point sample".to_string(),
        ));
    }

    Ok(samples)
}

fn collect_point_weights(
    layer: &wbvector::Layer,
    field_name: Option<&str>,
) -> Result<Vec<(f64, f64, f64)>, ToolError> {
    let field_idx = field_name.and_then(|name| layer.schema.field_index(name));
    let field_is_numeric = field_idx
        .and_then(|idx| layer.schema.fields().get(idx))
        .map(|field| matches!(field.field_type, wbvector::FieldType::Integer | wbvector::FieldType::Float))
        .unwrap_or(false);

    if field_name.is_some() && !field_is_numeric {
        return Err(ToolError::Validation(
            "weight field must exist and be numeric".to_string(),
        ));
    }

    let mut weighted = Vec::new();
    for feature in &layer.features {
        let Some(geometry) = &feature.geometry else {
            continue;
        };
        let weight = if let Some(idx) = field_idx {
            feature
                .attributes
                .get(idx)
                .and_then(|value| value.as_f64())
                .unwrap_or(1.0)
        } else {
            1.0
        };
        let mut coords = Vec::new();
        collect_geometry_coords(geometry, &mut coords);
        for coord in coords {
            weighted.push((coord.x, coord.y, weight));
        }
    }

    if weighted.is_empty() {
        return Err(ToolError::Validation(
            "points input must contain at least one usable point sample".to_string(),
        ));
    }
    Ok(weighted)
}

fn load_required_raster_list_arg(args: &ToolArgs, key: &str) -> Result<Vec<Raster>, ToolError> {
    let value = args
        .get(key)
        .ok_or_else(|| ToolError::Validation(format!("parameter '{}' is required", key)))?;
    let paths = parse_string_list_value(value, key)?;
    paths.into_iter()
        .map(|path| GisOverlayCore::load_raster(path.trim(), key))
        .collect()
}

#[derive(Clone, Copy)]
enum HeatKernel {
    Uniform,
    Triangular,
    Epanechnikov,
    Quartic,
    Triweight,
    Tricube,
    Gaussian,
    Cosine,
    Logistic,
    Sigmoid,
    Silverman,
}

impl HeatKernel {
    fn parse(value: Option<&str>) -> Self {
        let text = value.unwrap_or("quartic").trim().to_ascii_lowercase();
        if text.contains("uniform") || text.contains("rect") {
            Self::Uniform
        } else if text.contains("triang") {
            Self::Triangular
        } else if text.contains("epan") || text.contains("parabolic") {
            Self::Epanechnikov
        } else if text.contains("quartic") || text.contains("biweight") {
            Self::Quartic
        } else if text.contains("triweight") {
            Self::Triweight
        } else if text.contains("tricube") {
            Self::Tricube
        } else if text.contains("gaussian") {
            Self::Gaussian
        } else if text.contains("cosine") {
            Self::Cosine
        } else if text.contains("logistic") {
            Self::Logistic
        } else if text.contains("sigmoid") {
            Self::Sigmoid
        } else {
            Self::Silverman
        }
    }

    fn evaluate(self, d: f64) -> f64 {
        match self {
            Self::Uniform => 0.5,
            Self::Triangular => 1.0 - d.abs(),
            Self::Epanechnikov => 0.75 * (1.0 - d * d),
            Self::Quartic => 0.9375 * (1.0 - d * d).powi(2),
            Self::Triweight => 1.09375 * (1.0 - d * d).powi(3),
            Self::Tricube => 0.864197531 * (1.0 - d.abs().powi(3)).powi(3),
            Self::Gaussian => 0.398942280401433 * (-0.5 * d * d).exp(),
            Self::Cosine => 0.785398163397448 * (1.5707963267949 * d).cos(),
            Self::Logistic => 1.0 / (d.exp() + 2.0 + (-d).exp()),
            Self::Sigmoid => 0.636619772367581 / (d.exp() + (-d).exp()),
            Self::Silverman => {
                let s = d.abs() / 1.4142135623731;
                0.5 * (-s).exp() * (s + 0.785398163397448).sin()
            }
        }
    }
}

fn build_output_like_raster(template: &Raster, data_type: DataType) -> Raster {
    Raster::new(RasterConfig {
        cols: template.cols,
        rows: template.rows,
        bands: template.bands,
        x_min: template.x_min,
        y_min: template.y_min,
        cell_size: template.cell_size_x,
        cell_size_y: Some(template.cell_size_y),
        nodata: -32768.0,
        data_type,
        crs: template.crs.clone(),
        metadata: template.metadata.clone(),
    })
}

fn build_point_interpolation_output(
    points: &wbvector::Layer,
    samples: &[(f64, f64, f64)],
    cell_size: Option<f64>,
    base_raster: Option<Raster>,
    data_type: DataType,
) -> Result<Raster, ToolError> {
    if let Some(base) = base_raster {
        let mut output = build_output_like_raster(&base, data_type);
        if has_vector_crs(points) {
            output.crs = vector_crs_to_raster_crs(points);
        }
        return Ok(output);
    }

    let cell = cell_size.ok_or_else(|| {
        ToolError::Validation("either a positive cell_size or a base_raster must be provided".to_string())
    })?;
    if !cell.is_finite() || cell <= 0.0 {
        return Err(ToolError::Validation(
            "cell_size must be a positive finite value when base_raster is not provided".to_string(),
        ));
    }

    let min_x = samples.iter().map(|(x, _, _)| *x).fold(f64::INFINITY, f64::min);
    let max_x = samples.iter().map(|(x, _, _)| *x).fold(f64::NEG_INFINITY, f64::max);
    let min_y = samples.iter().map(|(_, y, _)| *y).fold(f64::INFINITY, f64::min);
    let max_y = samples.iter().map(|(_, y, _)| *y).fold(f64::NEG_INFINITY, f64::max);

    let cols = (((max_x - min_x) / cell).ceil() as usize).max(1);
    let rows = (((max_y - min_y) / cell).ceil() as usize).max(1);

    Ok(Raster::new(RasterConfig {
        cols,
        rows,
        bands: 1,
        x_min: min_x,
        y_min: max_y - rows as f64 * cell,
        cell_size: cell,
        cell_size_y: Some(cell),
        nodata: -32768.0,
        data_type,
        crs: vector_crs_to_raster_crs(points),
        metadata: Vec::new(),
    }))
}

fn point_in_triangle_with_barycentric(
    x: f64,
    y: f64,
    p1: (f64, f64),
    p2: (f64, f64),
    p3: (f64, f64),
    epsilon: f64,
) -> Option<(f64, f64, f64)> {
    let denom = (p2.1 - p3.1) * (p1.0 - p3.0) + (p3.0 - p2.0) * (p1.1 - p3.1);
    if denom.abs() <= epsilon {
        return None;
    }

    let w1 = ((p2.1 - p3.1) * (x - p3.0) + (p3.0 - p2.0) * (y - p3.1)) / denom;
    let w2 = ((p3.1 - p1.1) * (x - p3.0) + (p1.0 - p3.0) * (y - p3.1)) / denom;
    let w3 = 1.0 - w1 - w2;

    if w1 >= -epsilon && w2 >= -epsilon && w3 >= -epsilon {
        Some((w1, w2, w3))
    } else {
        None
    }
}

fn max_triangle_edge_length_2d_sq(p1: (f64, f64), p2: (f64, f64), p3: (f64, f64)) -> f64 {
    let d12 = (p1.0 - p2.0).powi(2) + (p1.1 - p2.1).powi(2);
    let d13 = (p1.0 - p3.0).powi(2) + (p1.1 - p3.1).powi(2);
    let d23 = (p2.0 - p3.0).powi(2) + (p2.1 - p3.1).powi(2);
    d12.max(d13).max(d23)
}

fn point_key_bits(x: f64, y: f64) -> (u64, u64) {
    (x.to_bits(), y.to_bits())
}

fn cross2d(o: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

fn convex_hull_2d(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    if points.len() <= 3 {
        return points.to_vec();
    }

    let mut pts = points.to_vec();
    pts.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(Ordering::Equal)
            .then(a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))
    });
    pts.dedup_by(|a, b| (a.0 - b.0).abs() <= 1.0e-12 && (a.1 - b.1).abs() <= 1.0e-12);

    if pts.len() <= 3 {
        return pts;
    }

    let mut lower: Vec<(f64, f64)> = Vec::new();
    for p in &pts {
        while lower.len() >= 2
            && cross2d(lower[lower.len() - 2], lower[lower.len() - 1], *p) <= 0.0
        {
            lower.pop();
        }
        lower.push(*p);
    }

    let mut upper: Vec<(f64, f64)> = Vec::new();
    for p in pts.iter().rev() {
        while upper.len() >= 2
            && cross2d(upper[upper.len() - 2], upper[upper.len() - 1], *p) <= 0.0
        {
            upper.pop();
        }
        upper.push(*p);
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn point_on_segment_2d(point: (f64, f64), a: (f64, f64), b: (f64, f64), epsilon: f64) -> bool {
    let cross = cross2d(a, b, point).abs();
    if cross > epsilon {
        return false;
    }
    let min_x = a.0.min(b.0) - epsilon;
    let max_x = a.0.max(b.0) + epsilon;
    let min_y = a.1.min(b.1) - epsilon;
    let max_y = a.1.max(b.1) + epsilon;
    point.0 >= min_x && point.0 <= max_x && point.1 >= min_y && point.1 <= max_y
}

fn point_in_polygon_2d(point: (f64, f64), polygon: &[(f64, f64)]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let (px, py) = point;
    let mut inside = false;
    let mut j = polygon.len() - 1;
    for i in 0..polygon.len() {
        let (xi, yi) = polygon[i];
        let (xj, yj) = polygon[j];
        if point_on_segment_2d(point, (xi, yi), (xj, yj), 1.0e-10) {
            return true;
        }
        let intersects = ((yi > py) != (yj > py))
            && (px < (xj - xi) * (py - yi) / ((yj - yi).abs().max(1.0e-15)) + xi);
        if intersects {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn samples_bbox_diagonal(samples: &[(f64, f64, f64)]) -> f64 {
    let min_x = samples.iter().map(|(x, _, _)| *x).fold(f64::INFINITY, f64::min);
    let max_x = samples
        .iter()
        .map(|(x, _, _)| *x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = samples.iter().map(|(_, y, _)| *y).fold(f64::INFINITY, f64::min);
    let max_y = samples
        .iter()
        .map(|(_, y, _)| *y)
        .fold(f64::NEG_INFINITY, f64::max);
    (max_x - min_x).hypot(max_y - min_y)
}

#[derive(Clone, Copy)]
enum RbfBasisType {
    ThinPlateSpline,
    PolyHarmonic,
    Gaussian,
    MultiQuadric,
    InverseMultiQuadric,
}

impl RbfBasisType {
    fn parse(value: Option<&str>) -> Self {
        let text = value.unwrap_or("thinplatespline").trim().to_ascii_lowercase();
        if text.contains("thin") {
            Self::ThinPlateSpline
        } else if text.contains("polyharmonic") {
            Self::PolyHarmonic
        } else if text.contains("gaussian") {
            Self::Gaussian
        } else if text.contains("multiquadric") {
            Self::MultiQuadric
        } else {
            Self::InverseMultiQuadric
        }
    }
}

fn rbf_similarity_weight(dist: f64, basis: RbfBasisType, weight: f64) -> f64 {
    let eps = weight.abs().max(1.0e-12);
    let r = dist.max(1.0e-12);
    match basis {
        RbfBasisType::ThinPlateSpline => {
            let v = r * r * r.ln().abs();
            1.0 / (v + 1.0e-12)
        }
        RbfBasisType::PolyHarmonic => 1.0 / (r.powf(weight.abs().max(1.0)) + 1.0e-12),
        RbfBasisType::Gaussian => (-(eps * r).powi(2)).exp(),
        RbfBasisType::MultiQuadric => 1.0 / ((1.0 + (eps * r).powi(2)).sqrt() + 1.0e-12),
        RbfBasisType::InverseMultiQuadric => 1.0 / (1.0 + (eps * r).powi(2)).sqrt(),
    }
}

fn min_max_valid(raster: &Raster) -> Option<(f64, f64)> {
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;
    let mut found = false;
    for index in 0..raster.data.len() {
        let value = raster.data.get_f64(index);
        if raster.is_nodata(value) {
            continue;
        }
        found = true;
        if value < min_value {
            min_value = value;
        }
        if value > max_value {
            max_value = value;
        }
    }
    found.then_some((min_value, max_value))
}

#[derive(Clone, Copy)]
enum AggregateType {
    Mean,
    Sum,
    Maximum,
    Minimum,
    Range,
}

impl AggregateType {
    fn parse(value: Option<&str>) -> Result<Self, ToolError> {
        match value.unwrap_or("mean").trim().to_ascii_lowercase().as_str() {
            "mean" => Ok(Self::Mean),
            "sum" => Ok(Self::Sum),
            "maximum" | "max" => Ok(Self::Maximum),
            "minimum" | "min" => Ok(Self::Minimum),
            "range" => Ok(Self::Range),
            other => Err(ToolError::Validation(format!(
                "unsupported aggregation_type '{}'; expected mean, sum, maximum, minimum, or range",
                other
            ))),
        }
    }
}

impl Tool for PickFromListTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "pick_from_list",
            display_name: "Pick From List",
            summary: "Selects per-cell values from a raster stack using a zero-based position raster.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input_rasters",
                    description: "Input raster stack as an array of paths or a semicolon/comma-delimited string.",
                    required: true,
                },
                ToolParamSpec {
                    name: "pos_input",
                    description: "Zero-based raster of stack positions.",
                    required: true,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output raster path.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input_rasters".to_string(), json!(["input1.tif", "input2.tif"]));
        defaults.insert("pos_input".to_string(), json!("positions.tif"));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("pick_from_list.tif"));

        ToolManifest {
            id: "pick_from_list".to_string(),
            display_name: "Pick From List".to_string(),
            summary: "Selects per-cell values from a raster stack using a zero-based position raster.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input_rasters".to_string(),
                    description: "Input raster stack as an array of paths or a semicolon/comma-delimited string.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "pos_input".to_string(),
                    description: "Zero-based raster of stack positions.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output raster path.".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "pick_from_list_basic".to_string(),
                description: "Selects values from a raster stack using a position raster.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "overlay".to_string(), "selection".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = GisOverlayCore::parse_raster_list_arg(args, "input_rasters")?;
        let _ = load_required_raster_arg(args, "pos_input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = GisOverlayCore::parse_raster_list_arg(args, "input_rasters")?;
        let position = load_required_raster_arg(args, "pos_input")?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("running pick from list");
        ctx.progress.info("reading input rasters");

        let rasters = input_paths
            .iter()
            .enumerate()
            .map(|(index, path)| GisOverlayCore::load_raster(path, &format!("input_rasters[{index}]")))
            .collect::<Result<Vec<_>, _>>()?;
        if rasters.is_empty() {
            return Err(ToolError::Validation("at least one input raster is required".to_string()));
        }
        GisOverlayCore::ensure_same_grid(&rasters)?;
        if !GisOverlayCore::rasters_share_grid(&rasters[0], &position) {
            return Err(ToolError::Validation(
                "position raster must have identical rows, columns, bands, cell sizes, and spatial extent".to_string(),
            ));
        }

        let mut output = build_output_like_raster(&rasters[0], DataType::F64);
        let nodata = output.nodata;
        let len = output.data.len();
        let mut out_values = vec![nodata; len];
        let chunk_size = 8192usize;

        ctx.progress.info("selecting raster values by position");
        for (chunk_index, out_chunk) in out_values.chunks_mut(chunk_size).enumerate() {
            let start = chunk_index * chunk_size;
            out_chunk.par_iter_mut().enumerate().for_each(|(offset, dst)| {
                let index = start + offset;
                let pos = position.data.get_f64(index);
                if position.is_nodata(pos) || !pos.is_finite() || pos < 0.0 {
                    *dst = nodata;
                    return;
                }
                let raster_index = pos as usize;
                if raster_index >= rasters.len() {
                    *dst = nodata;
                    return;
                }
                let value = rasters[raster_index].data.get_f64(index);
                *dst = if rasters[raster_index].is_nodata(value) { nodata } else { value };
            });
            let done = ((chunk_index + 1) * chunk_size).min(len);
            ctx.progress.progress(done as f64 / len.max(1) as f64);
        }

        for (index, value) in out_values.iter().enumerate() {
            output.data.set_f64(index, *value);
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(GisOverlayCore::build_result(locator))
    }
}

impl Tool for WeightedSumTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "weighted_sum",
            display_name: "Weighted Sum",
            summary: "Computes a weighted sum across a raster stack after normalizing weights to sum to one.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input_rasters", description: "Input raster stack.", required: true },
                ToolParamSpec { name: "weights", description: "Numeric weights corresponding to each input raster.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input_rasters".to_string(), json!(["factor1.tif", "factor2.tif"]));
        defaults.insert("weights".to_string(), json!([1.0, 1.0]));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("weighted_sum.tif"));
        ToolManifest {
            id: "weighted_sum".to_string(),
            display_name: "Weighted Sum".to_string(),
            summary: "Computes a weighted sum across a raster stack after normalizing weights to sum to one.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input_rasters".to_string(), description: "Input raster stack.".to_string(), required: true },
                ToolParamDescriptor { name: "weights".to_string(), description: "Numeric weights corresponding to each input raster.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "weighted_sum_basic".to_string(), description: "Runs weighted sum on a raster stack.".to_string(), args: example_args }],
            tags: vec!["raster".to_string(), "gis".to_string(), "overlay".to_string(), "weighted".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let rasters = GisOverlayCore::parse_raster_list_arg(args, "input_rasters")?;
        let weights = parse_f64_list_arg(args, "weights")?;
        if rasters.len() != weights.len() {
            return Err(ToolError::Validation("weights length must equal input_rasters length".to_string()));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = GisOverlayCore::parse_raster_list_arg(args, "input_rasters")?;
        let weights = normalize_weights(&parse_f64_list_arg(args, "weights")?)?;
        let output_path = parse_optional_output_path(args, "output")?;
        if input_paths.len() != weights.len() {
            return Err(ToolError::Validation("weights length must equal input_rasters length".to_string()));
        }
        ctx.progress.info("running weighted sum");
        ctx.progress.info("reading input rasters");
        let rasters = input_paths
            .iter()
            .enumerate()
            .map(|(index, path)| GisOverlayCore::load_raster(path, &format!("input_rasters[{index}]")))
            .collect::<Result<Vec<_>, _>>()?;
        GisOverlayCore::ensure_same_grid(&rasters)?;

        let mut output = build_output_like_raster(&rasters[0], DataType::F64);
        let nodata = output.nodata;
        let len = output.data.len();
        let mut out_values = vec![nodata; len];
        let chunk_size = 8192usize;
        ctx.progress.info("computing weighted sum");
        for (chunk_index, out_chunk) in out_values.chunks_mut(chunk_size).enumerate() {
            let start = chunk_index * chunk_size;
            out_chunk.par_iter_mut().enumerate().for_each(|(offset, dst)| {
                let index = start + offset;
                let mut sum = 0.0;
                for (raster, weight) in rasters.iter().zip(weights.iter()) {
                    let value = raster.data.get_f64(index);
                    if raster.is_nodata(value) {
                        *dst = nodata;
                        return;
                    }
                    sum += value * *weight;
                }
                *dst = sum;
            });
            let done = ((chunk_index + 1) * chunk_size).min(len);
            ctx.progress.progress(done as f64 / len.max(1) as f64);
        }
        for (index, value) in out_values.iter().enumerate() {
            output.data.set_f64(index, *value);
        }
        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(GisOverlayCore::build_result(locator))
    }
}

impl Tool for WeightedOverlayTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "weighted_overlay",
            display_name: "Weighted Overlay",
            summary: "Combines factor rasters using normalized weights, optional cost flags, and optional binary constraints.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "factors", description: "Input factor raster stack.", required: true },
                ToolParamSpec { name: "weights", description: "Numeric weights corresponding to each factor raster.", required: true },
                ToolParamSpec { name: "cost", description: "Optional boolean flags indicating cost factors.", required: false },
                ToolParamSpec { name: "constraints", description: "Optional raster constraints stack; values <= 0 force output to zero.", required: false },
                ToolParamSpec { name: "scale_max", description: "Maximum scale value used after factor normalization.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("factors".to_string(), json!(["factor1.tif", "factor2.tif"]));
        defaults.insert("weights".to_string(), json!([1.0, 1.0]));
        defaults.insert("cost".to_string(), json!([false, false]));
        defaults.insert("scale_max".to_string(), json!(1.0));
        let mut example_args = defaults.clone();
        example_args.insert("constraints".to_string(), json!(["constraint.tif"]));
        example_args.insert("output".to_string(), json!("weighted_overlay.tif"));
        ToolManifest {
            id: "weighted_overlay".to_string(),
            display_name: "Weighted Overlay".to_string(),
            summary: "Combines factor rasters using normalized weights, optional cost flags, and optional binary constraints.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "factors".to_string(), description: "Input factor raster stack.".to_string(), required: true },
                ToolParamDescriptor { name: "weights".to_string(), description: "Numeric weights corresponding to each factor raster.".to_string(), required: true },
                ToolParamDescriptor { name: "cost".to_string(), description: "Optional boolean flags indicating cost factors.".to_string(), required: false },
                ToolParamDescriptor { name: "constraints".to_string(), description: "Optional raster constraints stack; values <= 0 force output to zero.".to_string(), required: false },
                ToolParamDescriptor { name: "scale_max".to_string(), description: "Maximum scale value used after factor normalization.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "weighted_overlay_basic".to_string(), description: "Runs weighted overlay on factor rasters.".to_string(), args: example_args }],
            tags: vec!["raster".to_string(), "gis".to_string(), "overlay".to_string(), "weighted".to_string(), "constraints".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let factors = GisOverlayCore::parse_raster_list_arg(args, "factors")?;
        let weights = parse_f64_list_arg(args, "weights")?;
        if factors.len() != weights.len() {
            return Err(ToolError::Validation("weights length must equal factors length".to_string()));
        }
        if args.contains_key("cost") {
            let cost = parse_bool_list_arg(args, "cost")?;
            if cost.len() != factors.len() {
                return Err(ToolError::Validation("cost length must equal factors length".to_string()));
            }
        }
        let _ = parse_optional_raster_list_arg(args, "constraints")?;
        let _ = args.get("scale_max").and_then(|v| v.as_f64()).unwrap_or(1.0);
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let factor_paths = GisOverlayCore::parse_raster_list_arg(args, "factors")?;
        let weights = normalize_weights(&parse_f64_list_arg(args, "weights")?)?;
        if factor_paths.len() != weights.len() {
            return Err(ToolError::Validation("weights length must equal factors length".to_string()));
        }
        let cost_flags = if args.contains_key("cost") {
            let flags = parse_bool_list_arg(args, "cost")?;
            if flags.len() != factor_paths.len() {
                return Err(ToolError::Validation("cost length must equal factors length".to_string()));
            }
            flags
        } else {
            vec![false; factor_paths.len()]
        };
        let constraint_paths = parse_optional_raster_list_arg(args, "constraints")?;
        let scale_max = args.get("scale_max").and_then(|v| v.as_f64()).unwrap_or(1.0);
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("running weighted overlay");
        ctx.progress.info("reading factor rasters");
        let factors = factor_paths
            .iter()
            .enumerate()
            .map(|(index, path)| GisOverlayCore::load_raster(path, &format!("factors[{index}]")))
            .collect::<Result<Vec<_>, _>>()?;
        if factors.is_empty() {
            return Err(ToolError::Validation("at least one factor raster is required".to_string()));
        }
        GisOverlayCore::ensure_same_grid(&factors)?;
        let constraints = constraint_paths
            .iter()
            .enumerate()
            .map(|(index, path)| GisOverlayCore::load_raster(path, &format!("constraints[{index}]")))
            .collect::<Result<Vec<_>, _>>()?;
        for constraint in &constraints {
            if !GisOverlayCore::rasters_share_grid(&factors[0], constraint) {
                return Err(ToolError::Validation(
                    "constraint rasters must have identical rows, columns, bands, cell sizes, and spatial extent as factors"
                        .to_string(),
                ));
            }
        }

        let factor_ranges = factors
            .iter()
            .map(|raster| {
                min_max_valid(raster).ok_or_else(|| {
                    ToolError::Validation("factor rasters must contain at least one valid cell".to_string())
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut output = build_output_like_raster(&factors[0], DataType::F64);
        let nodata = output.nodata;
        let len = output.data.len();
        let mut out_values = vec![nodata; len];
        let chunk_size = 8192usize;
        ctx.progress.info("computing weighted overlay");
        for (chunk_index, out_chunk) in out_values.chunks_mut(chunk_size).enumerate() {
            let start = chunk_index * chunk_size;
            out_chunk.par_iter_mut().enumerate().for_each(|(offset, dst)| {
                let index = start + offset;
                let mut total = 0.0;
                for (((raster, weight), (min_value, max_value)), is_cost) in factors
                    .iter()
                    .zip(weights.iter())
                    .zip(factor_ranges.iter())
                    .zip(cost_flags.iter())
                {
                    let value = raster.data.get_f64(index);
                    if raster.is_nodata(value) {
                        *dst = nodata;
                        return;
                    }
                    let range = max_value - min_value;
                    let mut scaled = if range.abs() <= f64::EPSILON {
                        1.0
                    } else {
                        (value - *min_value) / range
                    };
                    if *is_cost {
                        scaled = 1.0 - scaled;
                    }
                    total += scaled * scale_max * *weight;
                }
                for constraint in &constraints {
                    let value = constraint.data.get_f64(index);
                    if !constraint.is_nodata(value) && value <= 0.0 {
                        total = 0.0;
                        break;
                    }
                }
                *dst = total;
            });
            let done = ((chunk_index + 1) * chunk_size).min(len);
            ctx.progress.progress(done as f64 / len.max(1) as f64);
        }
        for (index, value) in out_values.iter().enumerate() {
            output.data.set_f64(index, *value);
        }
        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(GisOverlayCore::build_result(locator))
    }
}

impl Tool for AggregateRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "aggregate_raster",
            display_name: "Aggregate Raster",
            summary: "Reduces raster resolution by aggregating blocks using mean, sum, min, max, or range.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster.", required: true },
                ToolParamSpec { name: "aggregation_factor", description: "Block size in source cells.", required: false },
                ToolParamSpec { name: "aggregation_type", description: "One of mean, sum, maximum, minimum, or range.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("aggregation_factor".to_string(), json!(2));
        defaults.insert("aggregation_type".to_string(), json!("mean"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("aggregate_raster.tif"));
        ToolManifest {
            id: "aggregate_raster".to_string(),
            display_name: "Aggregate Raster".to_string(),
            summary: "Reduces raster resolution by aggregating blocks using mean, sum, min, max, or range.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input raster.".to_string(), required: true },
                ToolParamDescriptor { name: "aggregation_factor".to_string(), description: "Block size in source cells.".to_string(), required: false },
                ToolParamDescriptor { name: "aggregation_type".to_string(), description: "One of mean, sum, maximum, minimum, or range.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "aggregate_raster_basic".to_string(), description: "Aggregates a raster by a factor of two.".to_string(), args: example_args }],
            tags: vec!["raster".to_string(), "gis".to_string(), "aggregation".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "input")?;
        let factor = args.get("aggregation_factor").and_then(|v| v.as_i64()).unwrap_or(2);
        if factor < 2 {
            return Err(ToolError::Validation("aggregation_factor must be at least 2".to_string()));
        }
        let _ = AggregateType::parse(args.get("aggregation_type").and_then(|v| v.as_str()))?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_required_raster_arg(args, "input")?;
        let factor = args.get("aggregation_factor").and_then(|v| v.as_i64()).unwrap_or(2).max(2) as usize;
        let agg_type = AggregateType::parse(args.get("aggregation_type").and_then(|v| v.as_str()))?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("running aggregate raster");
        let rows_out = ((input.rows as f64 / factor as f64).round().max(1.0)) as usize;
        let cols_out = ((input.cols as f64 / factor as f64).round().max(1.0)) as usize;
        let mut output = Raster::new(RasterConfig {
            cols: cols_out,
            rows: rows_out,
            bands: input.bands,
            x_min: input.x_min,
            y_min: input.y_max() - rows_out as f64 * input.cell_size_y * factor as f64,
            cell_size: input.cell_size_x * factor as f64,
            cell_size_y: Some(input.cell_size_y * factor as f64),
            nodata: input.nodata,
            data_type: DataType::F64,
            crs: input.crs.clone(),
            metadata: input.metadata.clone(),
        });

        let total = input.bands * rows_out * cols_out;
        let mut out_values = vec![output.nodata; total];
        let chunk_size = 4096usize;
        ctx.progress.info("aggregating raster blocks");
        for (chunk_index, out_chunk) in out_values.chunks_mut(chunk_size).enumerate() {
            let start = chunk_index * chunk_size;
            out_chunk.par_iter_mut().enumerate().for_each(|(offset, dst)| {
                let flat = start + offset;
                let band_stride = rows_out * cols_out;
                let band = flat / band_stride;
                let rem = flat % band_stride;
                let row = rem / cols_out;
                let col = rem % cols_out;
                let row_start = row * factor;
                let col_start = col * factor;
                let row_end = (row_start + factor).min(input.rows);
                let col_end = (col_start + factor).min(input.cols);

                let mut count = 0usize;
                let mut sum = 0.0;
                let mut min_value = f64::INFINITY;
                let mut max_value = f64::NEG_INFINITY;
                for src_row in row_start..row_end {
                    for src_col in col_start..col_end {
                        let value = input.get(band as isize, src_row as isize, src_col as isize);
                        if input.is_nodata(value) {
                            continue;
                        }
                        count += 1;
                        sum += value;
                        if value < min_value {
                            min_value = value;
                        }
                        if value > max_value {
                            max_value = value;
                        }
                    }
                }
                *dst = if count == 0 {
                    output.nodata
                } else {
                    match agg_type {
                        AggregateType::Mean => sum / count as f64,
                        AggregateType::Sum => sum,
                        AggregateType::Maximum => max_value,
                        AggregateType::Minimum => min_value,
                        AggregateType::Range => max_value - min_value,
                    }
                };
            });
            let done = ((chunk_index + 1) * chunk_size).min(total);
            ctx.progress.progress(done as f64 / total.max(1) as f64);
        }
        for (index, value) in out_values.iter().enumerate() {
            output.data.set_f64(index, *value);
        }
        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(GisOverlayCore::build_result(locator))
    }
}

fn block_tool_metadata(id: &'static str, display_name: &'static str, summary: &'static str) -> ToolMetadata {
    ToolMetadata {
        id,
        display_name,
        summary,
        category: ToolCategory::Raster,
        license_tier: LicenseTier::Open,
        params: vec![
            ToolParamSpec { name: "points", description: "Input points vector layer.", required: true },
            ToolParamSpec { name: "field_name", description: "Optional numeric attribute field to rasterize; defaults to FID fallback.", required: false },
            ToolParamSpec { name: "use_z", description: "Use Z values from point geometry instead of attributes.", required: false },
            ToolParamSpec { name: "cell_size", description: "Output cell size when base_raster is not provided.", required: false },
            ToolParamSpec { name: "base_raster", description: "Optional base raster controlling output geometry.", required: false },
            ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
        ],
    }
}

fn block_tool_manifest(id: &'static str, display_name: &'static str, summary: &'static str, output_name: &'static str) -> ToolManifest {
    let mut defaults = ToolArgs::new();
    defaults.insert("points".to_string(), json!("points.geojson"));
    defaults.insert("field_name".to_string(), json!("value"));
    defaults.insert("use_z".to_string(), json!(false));
    defaults.insert("cell_size".to_string(), json!(1.0));

    let mut example_args = defaults.clone();
    example_args.insert("output".to_string(), json!(output_name));

    ToolManifest {
        id: id.to_string(),
        display_name: display_name.to_string(),
        summary: summary.to_string(),
        category: ToolCategory::Raster,
        license_tier: LicenseTier::Open,
        params: vec![
            ToolParamDescriptor { name: "points".to_string(), description: "Input points vector layer.".to_string(), required: true },
            ToolParamDescriptor { name: "field_name".to_string(), description: "Optional numeric attribute field to rasterize; defaults to FID fallback.".to_string(), required: false },
            ToolParamDescriptor { name: "use_z".to_string(), description: "Use Z values from point geometry instead of attributes.".to_string(), required: false },
            ToolParamDescriptor { name: "cell_size".to_string(), description: "Output cell size when base_raster is not provided.".to_string(), required: false },
            ToolParamDescriptor { name: "base_raster".to_string(), description: "Optional base raster controlling output geometry.".to_string(), required: false },
            ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
        ],
        defaults,
        examples: vec![ToolExample { name: format!("{}_basic", id), description: format!("Runs {} on a point vector layer.", id), args: example_args }],
        tags: vec!["raster".to_string(), "gis".to_string(), "points".to_string(), "legacy-port".to_string()],
        stability: ToolStability::Experimental,
    }
}

fn run_block_extrema(args: &ToolArgs, ctx: &ToolContext, take_max: bool) -> Result<ToolRunResult, ToolError> {
    let points = load_vector_arg(args, "points")?;
    let field_name = args.get("field_name").and_then(|v| v.as_str());
    let use_z = args.get("use_z").and_then(|v| v.as_bool()).unwrap_or(false);
    let cell_size = args.get("cell_size").and_then(|v| v.as_f64());
    let base_raster = load_optional_raster_arg(args, "base_raster")?;
    let output_path = parse_optional_output_path(args, "output")?;

    if base_raster.is_none() && cell_size.unwrap_or(0.0) <= 0.0 {
        return Err(ToolError::Validation(
            "either a positive cell_size or a base_raster must be provided".to_string(),
        ));
    }

    ctx.progress.info(if take_max { "running block maximum" } else { "running block minimum" });
    let samples = collect_point_samples(&points, field_name, use_z)?;

    let mut output = if let Some(base) = base_raster {
        build_output_like_raster(&base, DataType::F64)
    } else {
        let cell = cell_size.unwrap();
        let min_x = samples.iter().map(|(x, _, _)| *x).fold(f64::INFINITY, f64::min);
        let max_x = samples.iter().map(|(x, _, _)| *x).fold(f64::NEG_INFINITY, f64::max);
        let min_y = samples.iter().map(|(_, y, _)| *y).fold(f64::INFINITY, f64::min);
        let max_y = samples.iter().map(|(_, y, _)| *y).fold(f64::NEG_INFINITY, f64::max);
        let cols = (((max_x - min_x) / cell).ceil() as usize).max(1);
        let rows = (((max_y - min_y) / cell).ceil() as usize).max(1);
        Raster::new(RasterConfig {
            cols,
            rows,
            bands: 1,
            x_min: min_x,
            y_min: max_y - rows as f64 * cell,
            cell_size: cell,
            cell_size_y: Some(cell),
            nodata: -32768.0,
            data_type: DataType::F64,
            crs: vector_crs_to_raster_crs(&points),
            metadata: Vec::new(),
        })
    };

    for (index, value) in (0..output.data.len()).map(|i| (i, output.nodata)) {
        output.data.set_f64(index, value);
    }

    ctx.progress.info("assigning point values to raster blocks");
    for (sample_index, (x, y, value)) in samples.iter().enumerate() {
        if let Some((col, row)) = output.world_to_pixel(*x, *y) {
            let idx = output.index(0, row, col).ok_or_else(|| ToolError::Execution("computed block cell is out of bounds".to_string()))?;
            let current = output.data.get_f64(idx);
            let next = if output.is_nodata(current) {
                *value
            } else if take_max {
                current.max(*value)
            } else {
                current.min(*value)
            };
            output.data.set_f64(idx, next);
        }
        ctx.progress.progress((sample_index + 1) as f64 / samples.len().max(1) as f64);
    }

    let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
    ctx.progress.progress(1.0);
    Ok(GisOverlayCore::build_result(locator))
}

impl Tool for BlockMinimumTool {
    fn metadata(&self) -> ToolMetadata {
        block_tool_metadata(
            "block_minimum",
            "Block Minimum",
            "Rasterizes point features by assigning the minimum value observed within each output cell.",
        )
    }

    fn manifest(&self) -> ToolManifest {
        block_tool_manifest(
            "block_minimum",
            "Block Minimum",
            "Rasterizes point features by assigning the minimum value observed within each output cell.",
            "block_minimum.tif",
        )
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "points")?;
        let _ = load_optional_raster_arg(args, "base_raster")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        run_block_extrema(args, ctx, false)
    }
}

impl Tool for BlockMaximumTool {
    fn metadata(&self) -> ToolMetadata {
        block_tool_metadata(
            "block_maximum",
            "Block Maximum",
            "Rasterizes point features by assigning the maximum value observed within each output cell.",
        )
    }

    fn manifest(&self) -> ToolManifest {
        block_tool_manifest(
            "block_maximum",
            "Block Maximum",
            "Rasterizes point features by assigning the maximum value observed within each output cell.",
            "block_maximum.tif",
        )
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "points")?;
        let _ = load_optional_raster_arg(args, "base_raster")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        run_block_extrema(args, ctx, true)
    }
}

impl Tool for IdwInterpolationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "idw_interpolation",
            display_name: "IDW Interpolation",
            summary: "Interpolates a raster from point samples using inverse-distance weighting.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "points", description: "Input points vector layer.", required: true },
                ToolParamSpec { name: "field_name", description: "Optional numeric attribute field; defaults to FID fallback.", required: false },
                ToolParamSpec { name: "use_z", description: "Use Z values from point geometry instead of attributes.", required: false },
                ToolParamSpec { name: "weight", description: "IDW distance exponent; defaults to 2.0.", required: false },
                ToolParamSpec { name: "radius", description: "Optional neighbourhood radius in map units; <=0 uses k-nearest fallback.", required: false },
                ToolParamSpec { name: "min_points", description: "Minimum number of neighbours to use; defaults to 0.", required: false },
                ToolParamSpec { name: "cell_size", description: "Output cell size when base_raster is not provided.", required: false },
                ToolParamSpec { name: "base_raster", description: "Optional base raster controlling output geometry.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("points".to_string(), json!("points.geojson"));
        defaults.insert("field_name".to_string(), json!("FID"));
        defaults.insert("use_z".to_string(), json!(false));
        defaults.insert("weight".to_string(), json!(2.0));
        defaults.insert("radius".to_string(), json!(0.0));
        defaults.insert("min_points".to_string(), json!(0));
        defaults.insert("cell_size".to_string(), json!(1.0));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("idw_interpolation.tif"));
        ToolManifest {
            id: "idw_interpolation".to_string(),
            display_name: "IDW Interpolation".to_string(),
            summary: "Interpolates a raster from point samples using inverse-distance weighting.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "points".to_string(), description: "Input points vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "field_name".to_string(), description: "Optional numeric attribute field; defaults to FID fallback.".to_string(), required: false },
                ToolParamDescriptor { name: "use_z".to_string(), description: "Use Z values from point geometry instead of attributes.".to_string(), required: false },
                ToolParamDescriptor { name: "weight".to_string(), description: "IDW distance exponent; defaults to 2.0.".to_string(), required: false },
                ToolParamDescriptor { name: "radius".to_string(), description: "Optional neighbourhood radius in map units; <=0 uses k-nearest fallback.".to_string(), required: false },
                ToolParamDescriptor { name: "min_points".to_string(), description: "Minimum number of neighbours to use; defaults to 0.".to_string(), required: false },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output cell size when base_raster is not provided.".to_string(), required: false },
                ToolParamDescriptor { name: "base_raster".to_string(), description: "Optional base raster controlling output geometry.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "idw_interpolation_basic".to_string(),
                description: "Interpolates point attributes to a raster surface using IDW.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "interpolation".to_string(), "idw".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "points")?;
        let _ = load_optional_raster_arg(args, "base_raster")?;
        let _ = parse_optional_output_path(args, "output")?;

        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if base_raster.is_none() && cell_size <= 0.0 {
            return Err(ToolError::Validation(
                "either a positive cell_size or a base_raster must be provided".to_string(),
            ));
        }

        let weight = args.get("weight").and_then(|v| v.as_f64()).unwrap_or(2.0);
        if !weight.is_finite() || weight < 0.0 {
            return Err(ToolError::Validation(
                "weight must be a finite value >= 0".to_string(),
            ));
        }

        let radius = args.get("radius").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if !radius.is_finite() {
            return Err(ToolError::Validation(
                "radius must be a finite value".to_string(),
            ));
        }

        let _ = args.get("min_points").and_then(|v| v.as_u64()).unwrap_or(0);
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let points = load_vector_arg(args, "points")?;
        let field_name = args.get("field_name").and_then(|v| v.as_str());
        let use_z = args.get("use_z").and_then(|v| v.as_bool()).unwrap_or(false);
        let weight = args.get("weight").and_then(|v| v.as_f64()).unwrap_or(2.0);
        let radius = args.get("radius").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let min_points = args.get("min_points").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64());
        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("running idw interpolation");
        let samples = collect_point_samples(&points, field_name, use_z)?;
        let mut output = build_point_interpolation_output(&points, &samples, cell_size, base_raster, DataType::F64)?;

        let mut tree = KdTree::new(2);
        for (x, y, value) in &samples {
            tree.add([*x, *y], *value)
                .map_err(|e| ToolError::Execution(format!("failed building interpolation index: {e}")))?;
        }

        let rows = output.rows;
        let cols = output.cols;
        let bands = output.bands;
        let nodata = output.nodata;
        let x_min = output.x_min;
        let y_max = output.y_max();
        let cell_x = output.cell_size_x;
        let cell_y = output.cell_size_y;
        let mut out_values = vec![nodata; output.data.len()];

        for band in 0..bands {
            for row in 0..rows {
                for col in 0..cols {
                    let x = x_min + (col as f64 + 0.5) * cell_x;
                    let y = y_max - (row as f64 + 0.5) * cell_y;

                    let mut neighbours = if radius > 0.0 {
                        tree.within(&[x, y], radius * radius, &squared_euclidean)
                            .map_err(|e| ToolError::Execution(format!("idw radius search failed: {e}")))?
                    } else {
                        Vec::new()
                    };

                    if radius <= 0.0 || neighbours.len() < min_points.max(1) {
                        let k = min_points.max(1).min(samples.len());
                        neighbours = tree
                            .nearest(&[x, y], k, &squared_euclidean)
                            .map_err(|e| ToolError::Execution(format!("idw nearest-neighbour search failed: {e}")))?;
                    }

                    if neighbours.is_empty() {
                        continue;
                    }

                    let mut weighted_sum = 0.0;
                    let mut sum_w = 0.0;
                    let mut assigned = false;
                    for (dist2, value) in neighbours {
                        if dist2 <= f64::EPSILON {
                            let idx = band * rows * cols + row * cols + col;
                            out_values[idx] = *value;
                            assigned = true;
                            break;
                        }
                        let dist = dist2.sqrt();
                        let w = if weight == 0.0 { 1.0 } else { 1.0 / dist.powf(weight) };
                        weighted_sum += *value * w;
                        sum_w += w;
                    }

                    if !assigned && sum_w > 0.0 {
                        let idx = band * rows * cols + row * cols + col;
                        out_values[idx] = weighted_sum / sum_w;
                    }
                }
                ctx.progress.progress((row + 1) as f64 / rows.max(1) as f64 * 0.99);
            }
        }

        for (index, value) in out_values.iter().enumerate() {
            output.data.set_f64(index, *value);
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(GisOverlayCore::build_result(locator))
    }
}

impl Tool for NearestNeighbourInterpolationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "nearest_neighbour_interpolation",
            display_name: "Nearest Neighbour Interpolation",
            summary: "Interpolates a raster from point samples by assigning each cell the nearest sample value.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "points", description: "Input points vector layer.", required: true },
                ToolParamSpec { name: "field_name", description: "Optional numeric attribute field; defaults to FID fallback.", required: false },
                ToolParamSpec { name: "use_z", description: "Use Z values from point geometry instead of attributes.", required: false },
                ToolParamSpec { name: "cell_size", description: "Output cell size when base_raster is not provided.", required: false },
                ToolParamSpec { name: "base_raster", description: "Optional base raster controlling output geometry.", required: false },
                ToolParamSpec { name: "max_dist", description: "Optional maximum search distance in map units; defaults to infinity.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("points".to_string(), json!("points.geojson"));
        defaults.insert("field_name".to_string(), json!("FID"));
        defaults.insert("use_z".to_string(), json!(false));
        defaults.insert("cell_size".to_string(), json!(1.0));
        let mut example_args = defaults.clone();
        example_args.insert("max_dist".to_string(), json!(10.0));
        example_args.insert("output".to_string(), json!("nearest_neighbour_interpolation.tif"));
        ToolManifest {
            id: "nearest_neighbour_interpolation".to_string(),
            display_name: "Nearest Neighbour Interpolation".to_string(),
            summary: "Interpolates a raster from point samples by assigning each cell the nearest sample value.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "points".to_string(), description: "Input points vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "field_name".to_string(), description: "Optional numeric attribute field; defaults to FID fallback.".to_string(), required: false },
                ToolParamDescriptor { name: "use_z".to_string(), description: "Use Z values from point geometry instead of attributes.".to_string(), required: false },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output cell size when base_raster is not provided.".to_string(), required: false },
                ToolParamDescriptor { name: "base_raster".to_string(), description: "Optional base raster controlling output geometry.".to_string(), required: false },
                ToolParamDescriptor { name: "max_dist".to_string(), description: "Optional maximum search distance in map units; defaults to infinity.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "nearest_neighbour_interpolation_basic".to_string(),
                description: "Interpolates point attributes to a nearest-neighbour raster surface.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "interpolation".to_string(), "nearest".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "points")?;
        let _ = load_optional_raster_arg(args, "base_raster")?;
        let _ = parse_optional_output_path(args, "output")?;

        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if base_raster.is_none() && cell_size <= 0.0 {
            return Err(ToolError::Validation(
                "either a positive cell_size or a base_raster must be provided".to_string(),
            ));
        }

        let max_dist = args.get("max_dist").and_then(|v| v.as_f64()).unwrap_or(f64::INFINITY);
        if !max_dist.is_finite() && !max_dist.is_infinite() {
            return Err(ToolError::Validation(
                "max_dist must be finite or infinity".to_string(),
            ));
        }
        if max_dist < 0.0 {
            return Err(ToolError::Validation(
                "max_dist must be >= 0".to_string(),
            ));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let points = load_vector_arg(args, "points")?;
        let field_name = args.get("field_name").and_then(|v| v.as_str());
        let use_z = args.get("use_z").and_then(|v| v.as_bool()).unwrap_or(false);
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64());
        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let max_dist = args.get("max_dist").and_then(|v| v.as_f64()).unwrap_or(f64::INFINITY);
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("running nearest neighbour interpolation");
        let samples = collect_point_samples(&points, field_name, use_z)?;
        let mut output = build_point_interpolation_output(&points, &samples, cell_size, base_raster, DataType::F64)?;

        let mut tree = KdTree::new(2);
        for (x, y, value) in &samples {
            tree.add([*x, *y], *value)
                .map_err(|e| ToolError::Execution(format!("failed building interpolation index: {e}")))?;
        }

        let rows = output.rows;
        let cols = output.cols;
        let bands = output.bands;
        let nodata = output.nodata;
        let x_min = output.x_min;
        let y_max = output.y_max();
        let cell_x = output.cell_size_x;
        let cell_y = output.cell_size_y;
        let mut out_values = vec![nodata; output.data.len()];

        for band in 0..bands {
            for row in 0..rows {
                for col in 0..cols {
                    let x = x_min + (col as f64 + 0.5) * cell_x;
                    let y = y_max - (row as f64 + 0.5) * cell_y;
                    let nearest = tree
                        .nearest(&[x, y], 1, &squared_euclidean)
                        .map_err(|e| ToolError::Execution(format!("nearest-neighbour search failed: {e}")))?;
                    if let Some((dist2, value)) = nearest.first() {
                        if dist2.sqrt() <= max_dist {
                            let idx = band * rows * cols + row * cols + col;
                            out_values[idx] = **value;
                        }
                    }
                }
                ctx.progress.progress((row + 1) as f64 / rows.max(1) as f64 * 0.99);
            }
        }

        for (index, value) in out_values.iter().enumerate() {
            output.data.set_f64(index, *value);
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(GisOverlayCore::build_result(locator))
    }
}

impl Tool for TinInterpolationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "tin_interpolation",
            display_name: "TIN Interpolation",
            summary: "Interpolates a raster from point samples using Delaunay triangulation and planar interpolation within each triangle.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "points", description: "Input points vector layer.", required: true },
                ToolParamSpec { name: "field_name", description: "Optional numeric attribute field; defaults to FID fallback.", required: false },
                ToolParamSpec { name: "use_z", description: "Use Z values from point geometry instead of attributes.", required: false },
                ToolParamSpec { name: "cell_size", description: "Output cell size when base_raster is not provided.", required: false },
                ToolParamSpec { name: "base_raster", description: "Optional base raster controlling output geometry.", required: false },
                ToolParamSpec { name: "max_triangle_edge_length", description: "Optional maximum allowed triangle edge length in map units.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("points".to_string(), json!("points.geojson"));
        defaults.insert("field_name".to_string(), json!("FID"));
        defaults.insert("use_z".to_string(), json!(false));
        defaults.insert("cell_size".to_string(), json!(1.0));
        let mut example_args = defaults.clone();
        example_args.insert("max_triangle_edge_length".to_string(), json!(10.0));
        example_args.insert("output".to_string(), json!("tin_interpolation.tif"));
        ToolManifest {
            id: "tin_interpolation".to_string(),
            display_name: "TIN Interpolation".to_string(),
            summary: "Interpolates a raster from point samples using Delaunay triangulation and planar interpolation within each triangle.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "points".to_string(), description: "Input points vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "field_name".to_string(), description: "Optional numeric attribute field; defaults to FID fallback.".to_string(), required: false },
                ToolParamDescriptor { name: "use_z".to_string(), description: "Use Z values from point geometry instead of attributes.".to_string(), required: false },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output cell size when base_raster is not provided.".to_string(), required: false },
                ToolParamDescriptor { name: "base_raster".to_string(), description: "Optional base raster controlling output geometry.".to_string(), required: false },
                ToolParamDescriptor { name: "max_triangle_edge_length".to_string(), description: "Optional maximum allowed triangle edge length in map units.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "tin_interpolation_basic".to_string(),
                description: "Interpolates point attributes to a planar-triangle raster surface.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "interpolation".to_string(), "tin".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "points")?;
        let _ = load_optional_raster_arg(args, "base_raster")?;
        let _ = parse_optional_output_path(args, "output")?;

        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if base_raster.is_none() && cell_size <= 0.0 {
            return Err(ToolError::Validation(
                "either a positive cell_size or a base_raster must be provided".to_string(),
            ));
        }

        let max_triangle_edge_length = args
            .get("max_triangle_edge_length")
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::INFINITY);
        if !max_triangle_edge_length.is_finite() && !max_triangle_edge_length.is_infinite() {
            return Err(ToolError::Validation(
                "max_triangle_edge_length must be finite or infinity".to_string(),
            ));
        }
        if max_triangle_edge_length <= 0.0 {
            return Err(ToolError::Validation(
                "max_triangle_edge_length must be > 0".to_string(),
            ));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let points = load_vector_arg(args, "points")?;
        let field_name = args.get("field_name").and_then(|v| v.as_str());
        let use_z = args.get("use_z").and_then(|v| v.as_bool()).unwrap_or(false);
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64());
        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let max_triangle_edge_length = args
            .get("max_triangle_edge_length")
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::INFINITY);
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("running tin interpolation");
        let samples = collect_point_samples(&points, field_name, use_z)?;
        if samples.len() < 3 {
            return Err(ToolError::Validation(
                "points input must contain at least three point samples for triangulation".to_string(),
            ));
        }

        let mut output = build_point_interpolation_output(&points, &samples, cell_size, base_raster, DataType::F64)?;
        let topo_points: Vec<TopoCoord> = samples
            .iter()
            .map(|(x, y, _)| TopoCoord::xy(*x, *y))
            .collect();
        let triangulation = delaunay_triangulation(&topo_points, 1.0e-12);
        if triangulation.triangles.is_empty() {
            return Err(ToolError::Execution(
                "failed to build triangulation from input points".to_string(),
            ));
        }

        let mut value_lookup = HashMap::with_capacity(samples.len());
        for (x, y, value) in &samples {
            value_lookup.entry(point_key_bits(*x, *y)).or_insert(*value);
        }

        let rows = output.rows;
        let cols = output.cols;
        let nodata = output.nodata;
        let x_min = output.x_min;
        let y_max = output.y_max();
        let cell_x = output.cell_size_x;
        let cell_y = output.cell_size_y;
        let max_edge_sq = if max_triangle_edge_length.is_infinite() {
            f64::INFINITY
        } else {
            max_triangle_edge_length * max_triangle_edge_length
        };
        let mut out_values = vec![nodata; output.data.len()];

        for (tri_idx, triangle) in triangulation.triangles.iter().enumerate() {
            let p1 = triangulation.points[triangle[0]];
            let p2 = triangulation.points[triangle[1]];
            let p3 = triangulation.points[triangle[2]];

            let z1 = *value_lookup.get(&point_key_bits(p1.x, p1.y)).ok_or_else(|| {
                ToolError::Execution("triangulation point value lookup failed for vertex 1".to_string())
            })?;
            let z2 = *value_lookup.get(&point_key_bits(p2.x, p2.y)).ok_or_else(|| {
                ToolError::Execution("triangulation point value lookup failed for vertex 2".to_string())
            })?;
            let z3 = *value_lookup.get(&point_key_bits(p3.x, p3.y)).ok_or_else(|| {
                ToolError::Execution("triangulation point value lookup failed for vertex 3".to_string())
            })?;

            if max_triangle_edge_length_2d_sq((p1.x, p1.y), (p2.x, p2.y), (p3.x, p3.y)) > max_edge_sq {
                continue;
            }

            let min_x = p1.x.min(p2.x.min(p3.x));
            let max_x = p1.x.max(p2.x.max(p3.x));
            let min_y = p1.y.min(p2.y.min(p3.y));
            let max_y = p1.y.max(p2.y.max(p3.y));

            let col_start = (((min_x - x_min) / cell_x).floor() as isize).clamp(0, cols as isize - 1) as usize;
            let col_end = (((max_x - x_min) / cell_x).ceil() as isize).clamp(0, cols as isize - 1) as usize;
            let row_start = (((y_max - max_y) / cell_y).floor() as isize).clamp(0, rows as isize - 1) as usize;
            let row_end = (((y_max - min_y) / cell_y).ceil() as isize).clamp(0, rows as isize - 1) as usize;

            for row in row_start..=row_end {
                for col in col_start..=col_end {
                    let x = x_min + (col as f64 + 0.5) * cell_x;
                    let y = y_max - (row as f64 + 0.5) * cell_y;
                    if let Some((w1, w2, w3)) = point_in_triangle_with_barycentric(
                        x,
                        y,
                        (p1.x, p1.y),
                        (p2.x, p2.y),
                        (p3.x, p3.y),
                        1.0e-10,
                    ) {
                        let z = w1 * z1 + w2 * z2 + w3 * z3;
                        out_values[row * cols + col] = z;
                    }
                }
            }

            ctx.progress
                .progress((tri_idx + 1) as f64 / triangulation.triangles.len().max(1) as f64 * 0.99);
        }

        for (index, value) in out_values.iter().enumerate() {
            output.data.set_f64(index, *value);
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(GisOverlayCore::build_result(locator))
    }
}

impl Tool for NaturalNeighbourInterpolationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "natural_neighbour_interpolation",
            display_name: "Natural Neighbour Interpolation",
            summary: "Interpolates a raster from point samples using a Delaunay-neighbour weighted scheme.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "points", description: "Input points vector layer.", required: true },
                ToolParamSpec { name: "field_name", description: "Optional numeric attribute field; defaults to FID fallback.", required: false },
                ToolParamSpec { name: "use_z", description: "Use Z values from point geometry instead of attributes.", required: false },
                ToolParamSpec { name: "cell_size", description: "Output cell size when base_raster is not provided.", required: false },
                ToolParamSpec { name: "base_raster", description: "Optional base raster controlling output geometry.", required: false },
                ToolParamSpec { name: "clip_to_hull", description: "Limit interpolation to the convex hull of points.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("points".to_string(), json!("points.geojson"));
        defaults.insert("field_name".to_string(), json!("FID"));
        defaults.insert("use_z".to_string(), json!(false));
        defaults.insert("cell_size".to_string(), json!(1.0));
        defaults.insert("clip_to_hull".to_string(), json!(true));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("natural_neighbour_interpolation.tif"));
        ToolManifest {
            id: "natural_neighbour_interpolation".to_string(),
            display_name: "Natural Neighbour Interpolation".to_string(),
            summary: "Interpolates a raster from point samples using a Delaunay-neighbour weighted scheme.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "points".to_string(), description: "Input points vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "field_name".to_string(), description: "Optional numeric attribute field; defaults to FID fallback.".to_string(), required: false },
                ToolParamDescriptor { name: "use_z".to_string(), description: "Use Z values from point geometry instead of attributes.".to_string(), required: false },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output cell size when base_raster is not provided.".to_string(), required: false },
                ToolParamDescriptor { name: "base_raster".to_string(), description: "Optional base raster controlling output geometry.".to_string(), required: false },
                ToolParamDescriptor { name: "clip_to_hull".to_string(), description: "Limit interpolation to the convex hull of points.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "natural_neighbour_interpolation_basic".to_string(),
                description: "Interpolates point attributes using Delaunay-neighbour weighting.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "interpolation".to_string(), "natural-neighbour".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "points")?;
        let _ = load_optional_raster_arg(args, "base_raster")?;
        let _ = parse_optional_output_path(args, "output")?;
        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if base_raster.is_none() && cell_size <= 0.0 {
            return Err(ToolError::Validation(
                "either a positive cell_size or a base_raster must be provided".to_string(),
            ));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let points = load_vector_arg(args, "points")?;
        let field_name = args.get("field_name").and_then(|v| v.as_str());
        let use_z = args.get("use_z").and_then(|v| v.as_bool()).unwrap_or(false);
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64());
        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let clip_to_hull = args.get("clip_to_hull").and_then(|v| v.as_bool()).unwrap_or(true);
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("running natural neighbour interpolation");
        let samples = collect_point_samples(&points, field_name, use_z)?;
        if samples.len() < 3 {
            return Err(ToolError::Validation(
                "points input must contain at least three point samples for interpolation".to_string(),
            ));
        }
        let mut output = build_point_interpolation_output(&points, &samples, cell_size, base_raster, DataType::F64)?;

        let topo_points: Vec<TopoCoord> = samples
            .iter()
            .map(|(x, y, _)| TopoCoord::xy(*x, *y))
            .collect();
        let triangulation = delaunay_triangulation(&topo_points, 1.0e-12);
        if triangulation.triangles.is_empty() {
            return Err(ToolError::Execution(
                "failed to build triangulation from input points".to_string(),
            ));
        }

        let mut adjacency: Vec<HashSet<usize>> = vec![HashSet::new(); triangulation.points.len()];
        for tri in &triangulation.triangles {
            adjacency[tri[0]].insert(tri[1]);
            adjacency[tri[0]].insert(tri[2]);
            adjacency[tri[1]].insert(tri[0]);
            adjacency[tri[1]].insert(tri[2]);
            adjacency[tri[2]].insert(tri[0]);
            adjacency[tri[2]].insert(tri[1]);
        }

        let mut value_lookup = HashMap::with_capacity(samples.len());
        for (x, y, value) in &samples {
            value_lookup.entry(point_key_bits(*x, *y)).or_insert(*value);
        }

        let mut tree = KdTree::new(2);
        for (idx, p) in triangulation.points.iter().enumerate() {
            tree.add([p.x, p.y], idx)
                .map_err(|e| ToolError::Execution(format!("failed building interpolation index: {e}")))?;
        }

        let hull = if clip_to_hull {
            Some(convex_hull_2d(
                &triangulation.points.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>(),
            ))
        } else {
            None
        };

        let rows = output.rows;
        let cols = output.cols;
        let nodata = output.nodata;
        let x_min = output.x_min;
        let y_max = output.y_max();
        let cell_x = output.cell_size_x;
        let cell_y = output.cell_size_y;
        let mut out_values = vec![nodata; output.data.len()];

        for row in 0..rows {
            for col in 0..cols {
                let x = x_min + (col as f64 + 0.5) * cell_x;
                let y = y_max - (row as f64 + 0.5) * cell_y;

                if let Some(hull_vertices) = &hull {
                    if !point_in_polygon_2d((x, y), hull_vertices) {
                        continue;
                    }
                }

                let nearest = tree
                    .nearest(&[x, y], 1, &squared_euclidean)
                    .map_err(|e| ToolError::Execution(format!("nearest-neighbour search failed: {e}")))?;
                let Some((_, nearest_idx_ref)) = nearest.first() else {
                    continue;
                };
                let nearest_idx = **nearest_idx_ref;

                let mut weighted_sum = 0.0;
                let mut sum_w = 0.0;
                let mut candidates = Vec::with_capacity(adjacency[nearest_idx].len() + 1);
                candidates.push(nearest_idx);
                candidates.extend(adjacency[nearest_idx].iter().copied());

                for idx in candidates {
                    let p = triangulation.points[idx];
                    let value = *value_lookup
                        .get(&point_key_bits(p.x, p.y))
                        .ok_or_else(|| ToolError::Execution("point value lookup failed".to_string()))?;
                    let dist2 = (x - p.x).powi(2) + (y - p.y).powi(2);
                    if dist2 <= f64::EPSILON {
                        weighted_sum = value;
                        sum_w = 1.0;
                        break;
                    }
                    let w = 1.0 / dist2.max(1.0e-12);
                    weighted_sum += value * w;
                    sum_w += w;
                }

                if sum_w > 0.0 {
                    out_values[row * cols + col] = weighted_sum / sum_w;
                }
            }
            ctx.progress.progress((row + 1) as f64 / rows.max(1) as f64 * 0.99);
        }

        for (index, value) in out_values.iter().enumerate() {
            output.data.set_f64(index, *value);
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(GisOverlayCore::build_result(locator))
    }
}

impl Tool for ModifiedShepardInterpolationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "modified_shepard_interpolation",
            display_name: "Modified Shepard Interpolation",
            summary: "Interpolates a raster from point samples using locally weighted modified-Shepard blending.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "points", description: "Input points vector layer.", required: true },
                ToolParamSpec { name: "field_name", description: "Optional numeric attribute field; defaults to FID fallback.", required: false },
                ToolParamSpec { name: "use_z", description: "Use Z values from point geometry instead of attributes.", required: false },
                ToolParamSpec { name: "weight", description: "Shepard weight exponent; defaults to 2.0.", required: false },
                ToolParamSpec { name: "radius", description: "Optional neighbourhood radius in map units.", required: false },
                ToolParamSpec { name: "min_points", description: "Minimum number of neighbours; defaults to 8.", required: false },
                ToolParamSpec { name: "use_quadratic_basis", description: "Optional local quadratic basis flag (reserved for future parity refinement).", required: false },
                ToolParamSpec { name: "cell_size", description: "Output cell size when base_raster is not provided.", required: false },
                ToolParamSpec { name: "base_raster", description: "Optional base raster controlling output geometry.", required: false },
                ToolParamSpec { name: "use_data_hull", description: "Limit interpolation to the convex hull of points.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("points".to_string(), json!("points.geojson"));
        defaults.insert("field_name".to_string(), json!("FID"));
        defaults.insert("use_z".to_string(), json!(false));
        defaults.insert("weight".to_string(), json!(2.0));
        defaults.insert("radius".to_string(), json!(0.0));
        defaults.insert("min_points".to_string(), json!(8));
        defaults.insert("use_quadratic_basis".to_string(), json!(false));
        defaults.insert("cell_size".to_string(), json!(1.0));
        defaults.insert("use_data_hull".to_string(), json!(false));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("modified_shepard_interpolation.tif"));
        ToolManifest {
            id: "modified_shepard_interpolation".to_string(),
            display_name: "Modified Shepard Interpolation".to_string(),
            summary: "Interpolates a raster from point samples using locally weighted modified-Shepard blending.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "points".to_string(), description: "Input points vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "field_name".to_string(), description: "Optional numeric attribute field; defaults to FID fallback.".to_string(), required: false },
                ToolParamDescriptor { name: "use_z".to_string(), description: "Use Z values from point geometry instead of attributes.".to_string(), required: false },
                ToolParamDescriptor { name: "weight".to_string(), description: "Shepard weight exponent; defaults to 2.0.".to_string(), required: false },
                ToolParamDescriptor { name: "radius".to_string(), description: "Optional neighbourhood radius in map units.".to_string(), required: false },
                ToolParamDescriptor { name: "min_points".to_string(), description: "Minimum number of neighbours; defaults to 8.".to_string(), required: false },
                ToolParamDescriptor { name: "use_quadratic_basis".to_string(), description: "Optional local quadratic basis flag (reserved for future parity refinement).".to_string(), required: false },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output cell size when base_raster is not provided.".to_string(), required: false },
                ToolParamDescriptor { name: "base_raster".to_string(), description: "Optional base raster controlling output geometry.".to_string(), required: false },
                ToolParamDescriptor { name: "use_data_hull".to_string(), description: "Limit interpolation to the convex hull of points.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "modified_shepard_interpolation_basic".to_string(),
                description: "Interpolates point attributes using modified Shepard weighting.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "interpolation".to_string(), "shepard".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "points")?;
        let _ = load_optional_raster_arg(args, "base_raster")?;
        let _ = parse_optional_output_path(args, "output")?;
        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if base_raster.is_none() && cell_size <= 0.0 {
            return Err(ToolError::Validation(
                "either a positive cell_size or a base_raster must be provided".to_string(),
            ));
        }
        let weight = args.get("weight").and_then(|v| v.as_f64()).unwrap_or(2.0);
        if !weight.is_finite() || weight < 0.0 {
            return Err(ToolError::Validation(
                "weight must be a finite value >= 0".to_string(),
            ));
        }
        let radius = args.get("radius").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if !radius.is_finite() || radius < 0.0 {
            return Err(ToolError::Validation(
                "radius must be a finite value >= 0".to_string(),
            ));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let points = load_vector_arg(args, "points")?;
        let field_name = args.get("field_name").and_then(|v| v.as_str());
        let use_z = args.get("use_z").and_then(|v| v.as_bool()).unwrap_or(false);
        let weight = args.get("weight").and_then(|v| v.as_f64()).unwrap_or(2.0);
        let radius = args.get("radius").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let min_points = args.get("min_points").and_then(|v| v.as_u64()).unwrap_or(8) as usize;
        let _use_quadratic_basis = args
            .get("use_quadratic_basis")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64());
        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let use_data_hull = args.get("use_data_hull").and_then(|v| v.as_bool()).unwrap_or(false);
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("running modified shepard interpolation");
        let samples = collect_point_samples(&points, field_name, use_z)?;
        let mut output = build_point_interpolation_output(&points, &samples, cell_size, base_raster, DataType::F64)?;

        let mut tree = KdTree::new(2);
        for (x, y, value) in &samples {
            tree.add([*x, *y], *value)
                .map_err(|e| ToolError::Execution(format!("failed building interpolation index: {e}")))?;
        }

        let hull = if use_data_hull {
            Some(convex_hull_2d(
                &samples.iter().map(|(x, y, _)| (*x, *y)).collect::<Vec<_>>(),
            ))
        } else {
            None
        };

        let max_radius = if radius > 0.0 {
            radius
        } else {
            samples_bbox_diagonal(&samples).max(1.0e-12)
        };
        let radius_sq = radius * radius;
        let k = min_points.max(8).min(samples.len());

        let rows = output.rows;
        let cols = output.cols;
        let nodata = output.nodata;
        let x_min = output.x_min;
        let y_max = output.y_max();
        let cell_x = output.cell_size_x;
        let cell_y = output.cell_size_y;
        let mut out_values = vec![nodata; output.data.len()];

        for row in 0..rows {
            for col in 0..cols {
                let x = x_min + (col as f64 + 0.5) * cell_x;
                let y = y_max - (row as f64 + 0.5) * cell_y;

                if let Some(hull_vertices) = &hull {
                    if !point_in_polygon_2d((x, y), hull_vertices) {
                        continue;
                    }
                }

                let mut neighbours = if radius > 0.0 {
                    tree.within(&[x, y], radius_sq, &squared_euclidean)
                        .map_err(|e| ToolError::Execution(format!("modified shepard radius search failed: {e}")))?
                } else {
                    Vec::new()
                };

                if neighbours.len() < k {
                    neighbours = tree
                        .nearest(&[x, y], k, &squared_euclidean)
                        .map_err(|e| ToolError::Execution(format!("modified shepard nearest-neighbour search failed: {e}")))?;
                }

                if neighbours.is_empty() {
                    continue;
                }

                let mut val = 0.0;
                let mut sum_weights = 0.0;
                let mut assigned = false;
                for (dist2, z) in neighbours {
                    if dist2 <= f64::EPSILON {
                        out_values[row * cols + col] = *z;
                        assigned = true;
                        break;
                    }
                    let dist = dist2.sqrt();
                    let denom = (max_radius * dist).max(1.0e-12);
                    let w = ((max_radius - dist).max(0.0) / denom).powf(weight);
                    if w > 0.0 && w.is_finite() {
                        sum_weights += w;
                        val += *z * w;
                    }
                }

                if !assigned && sum_weights > 0.0 {
                    out_values[row * cols + col] = val / sum_weights;
                }
            }
            ctx.progress.progress((row + 1) as f64 / rows.max(1) as f64 * 0.99);
        }

        for (index, value) in out_values.iter().enumerate() {
            output.data.set_f64(index, *value);
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(GisOverlayCore::build_result(locator))
    }
}

impl Tool for RadialBasisFunctionInterpolationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "radial_basis_function_interpolation",
            display_name: "Radial Basis Function Interpolation",
            summary: "Interpolates a raster from point samples using local radial-basis similarity weighting.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "points", description: "Input points vector layer.", required: true },
                ToolParamSpec { name: "field_name", description: "Optional numeric attribute field; defaults to FID fallback.", required: false },
                ToolParamSpec { name: "use_z", description: "Use Z values from point geometry instead of attributes.", required: false },
                ToolParamSpec { name: "radius", description: "Optional neighbourhood radius in map units.", required: false },
                ToolParamSpec { name: "min_points", description: "Minimum number of neighbours to use.", required: false },
                ToolParamSpec { name: "cell_size", description: "Output cell size when base_raster is not provided.", required: false },
                ToolParamSpec { name: "base_raster", description: "Optional base raster controlling output geometry.", required: false },
                ToolParamSpec { name: "func_type", description: "Radial basis type (thinplatespline, polyharmonic, gaussian, multiquadric, inversemultiquadric).", required: false },
                ToolParamSpec { name: "poly_order", description: "Polynomial order hint (none, constant, quadratic).", required: false },
                ToolParamSpec { name: "weight", description: "Basis shape parameter/exponent depending on func_type.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("points".to_string(), json!("points.geojson"));
        defaults.insert("field_name".to_string(), json!("FID"));
        defaults.insert("use_z".to_string(), json!(false));
        defaults.insert("radius".to_string(), json!(0.0));
        defaults.insert("min_points".to_string(), json!(16));
        defaults.insert("cell_size".to_string(), json!(1.0));
        defaults.insert("func_type".to_string(), json!("thinplatespline"));
        defaults.insert("poly_order".to_string(), json!("none"));
        defaults.insert("weight".to_string(), json!(0.1));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("radial_basis_function_interpolation.tif"));
        ToolManifest {
            id: "radial_basis_function_interpolation".to_string(),
            display_name: "Radial Basis Function Interpolation".to_string(),
            summary: "Interpolates a raster from point samples using local radial-basis similarity weighting.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "points".to_string(), description: "Input points vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "field_name".to_string(), description: "Optional numeric attribute field; defaults to FID fallback.".to_string(), required: false },
                ToolParamDescriptor { name: "use_z".to_string(), description: "Use Z values from point geometry instead of attributes.".to_string(), required: false },
                ToolParamDescriptor { name: "radius".to_string(), description: "Optional neighbourhood radius in map units.".to_string(), required: false },
                ToolParamDescriptor { name: "min_points".to_string(), description: "Minimum number of neighbours to use.".to_string(), required: false },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output cell size when base_raster is not provided.".to_string(), required: false },
                ToolParamDescriptor { name: "base_raster".to_string(), description: "Optional base raster controlling output geometry.".to_string(), required: false },
                ToolParamDescriptor { name: "func_type".to_string(), description: "Radial basis type (thinplatespline, polyharmonic, gaussian, multiquadric, inversemultiquadric).".to_string(), required: false },
                ToolParamDescriptor { name: "poly_order".to_string(), description: "Polynomial order hint (none, constant, quadratic).".to_string(), required: false },
                ToolParamDescriptor { name: "weight".to_string(), description: "Basis shape parameter/exponent depending on func_type.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "radial_basis_function_interpolation_basic".to_string(),
                description: "Interpolates point attributes using local radial-basis weighting.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "interpolation".to_string(), "rbf".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "points")?;
        let _ = load_optional_raster_arg(args, "base_raster")?;
        let _ = parse_optional_output_path(args, "output")?;
        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if base_raster.is_none() && cell_size <= 0.0 {
            return Err(ToolError::Validation(
                "either a positive cell_size or a base_raster must be provided".to_string(),
            ));
        }
        let radius = args.get("radius").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if !radius.is_finite() || radius < 0.0 {
            return Err(ToolError::Validation(
                "radius must be a finite value >= 0".to_string(),
            ));
        }
        let _ = RbfBasisType::parse(args.get("func_type").and_then(|v| v.as_str()));
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let points = load_vector_arg(args, "points")?;
        let field_name = args.get("field_name").and_then(|v| v.as_str());
        let use_z = args.get("use_z").and_then(|v| v.as_bool()).unwrap_or(false);
        let radius = args.get("radius").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let min_points = args.get("min_points").and_then(|v| v.as_u64()).unwrap_or(16) as usize;
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64());
        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let basis = RbfBasisType::parse(args.get("func_type").and_then(|v| v.as_str()));
        let _poly_order = args
            .get("poly_order")
            .and_then(|v| v.as_str())
            .unwrap_or("none")
            .to_ascii_lowercase();
        let shape_weight = args.get("weight").and_then(|v| v.as_f64()).unwrap_or(0.1);
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("running radial basis function interpolation");
        let samples = collect_point_samples(&points, field_name, use_z)?;
        let mut output = build_point_interpolation_output(&points, &samples, cell_size, base_raster, DataType::F64)?;

        let mut tree = KdTree::new(2);
        for (x, y, value) in &samples {
            tree.add([*x, *y], *value)
                .map_err(|e| ToolError::Execution(format!("failed building interpolation index: {e}")))?;
        }

        let hull = convex_hull_2d(&samples.iter().map(|(x, y, _)| (*x, *y)).collect::<Vec<_>>());
        let radius_sq = radius * radius;
        let k = min_points.max(3).min(samples.len());

        let rows = output.rows;
        let cols = output.cols;
        let nodata = output.nodata;
        let x_min = output.x_min;
        let y_max = output.y_max();
        let cell_x = output.cell_size_x;
        let cell_y = output.cell_size_y;
        let mut out_values = vec![nodata; output.data.len()];

        for row in 0..rows {
            for col in 0..cols {
                let x = x_min + (col as f64 + 0.5) * cell_x;
                let y = y_max - (row as f64 + 0.5) * cell_y;
                if !point_in_polygon_2d((x, y), &hull) {
                    continue;
                }

                let mut neighbours = if radius > 0.0 {
                    tree.within(&[x, y], radius_sq, &squared_euclidean)
                        .map_err(|e| ToolError::Execution(format!("rbf radius search failed: {e}")))?
                } else {
                    Vec::new()
                };
                if neighbours.len() < k {
                    neighbours = tree
                        .nearest(&[x, y], k, &squared_euclidean)
                        .map_err(|e| ToolError::Execution(format!("rbf nearest-neighbour search failed: {e}")))?;
                }

                if neighbours.is_empty() {
                    continue;
                }

                let mut weighted_sum = 0.0;
                let mut sum_w = 0.0;
                let mut assigned = false;
                for (dist2, z) in neighbours {
                    if dist2 <= f64::EPSILON {
                        out_values[row * cols + col] = *z;
                        assigned = true;
                        break;
                    }
                    let dist = dist2.sqrt();
                    let w = rbf_similarity_weight(dist, basis, shape_weight);
                    if w.is_finite() && w > 0.0 {
                        weighted_sum += *z * w;
                        sum_w += w;
                    }
                }

                if !assigned && sum_w > 0.0 {
                    out_values[row * cols + col] = weighted_sum / sum_w;
                }
            }
            ctx.progress.progress((row + 1) as f64 / rows.max(1) as f64 * 0.99);
        }

        for (index, value) in out_values.iter().enumerate() {
            output.data.set_f64(index, *value);
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(GisOverlayCore::build_result(locator))
    }
}

impl Tool for HeatMapTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "heat_map",
            display_name: "Heat Map",
            summary: "Generates a kernel-density heat map raster from point occurrences.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "points", description: "Input points vector layer.", required: true },
                ToolParamSpec { name: "field_name", description: "Optional numeric weight field; defaults to unweighted occurrences.", required: false },
                ToolParamSpec { name: "bandwidth", description: "Kernel bandwidth in map units.", required: true },
                ToolParamSpec { name: "cell_size", description: "Output cell size when base_raster is not provided.", required: false },
                ToolParamSpec { name: "base_raster", description: "Optional base raster controlling output geometry.", required: false },
                ToolParamSpec { name: "kernel_function", description: "Kernel function type; defaults to quartic.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("points".to_string(), json!("points.geojson"));
        defaults.insert("bandwidth".to_string(), json!(1.0));
        defaults.insert("cell_size".to_string(), json!(1.0));
        defaults.insert("kernel_function".to_string(), json!("quartic"));
        let mut example_args = defaults.clone();
        example_args.insert("field_name".to_string(), json!("WEIGHT"));
        example_args.insert("output".to_string(), json!("heat_map.tif"));
        ToolManifest {
            id: "heat_map".to_string(),
            display_name: "Heat Map".to_string(),
            summary: "Generates a kernel-density heat map raster from point occurrences.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "points".to_string(), description: "Input points vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "field_name".to_string(), description: "Optional numeric weight field; defaults to unweighted occurrences.".to_string(), required: false },
                ToolParamDescriptor { name: "bandwidth".to_string(), description: "Kernel bandwidth in map units.".to_string(), required: true },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output cell size when base_raster is not provided.".to_string(), required: false },
                ToolParamDescriptor { name: "base_raster".to_string(), description: "Optional base raster controlling output geometry.".to_string(), required: false },
                ToolParamDescriptor { name: "kernel_function".to_string(), description: "Kernel function type; defaults to quartic.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "heat_map_basic".to_string(),
                description: "Creates a kernel-density heat map from weighted or unweighted point events.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "density".to_string(), "heatmap".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "points")?;
        let _ = load_optional_raster_arg(args, "base_raster")?;
        let _ = parse_optional_output_path(args, "output")?;
        let bandwidth = args.get("bandwidth").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if !bandwidth.is_finite() || bandwidth <= 0.0 {
            return Err(ToolError::Validation(
                "bandwidth must be a positive finite value".to_string(),
            ));
        }
        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if base_raster.is_none() && cell_size <= 0.0 {
            return Err(ToolError::Validation(
                "either a positive cell_size or a base_raster must be provided".to_string(),
            ));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let points = load_vector_arg(args, "points")?;
        let field_name = args.get("field_name").and_then(|v| v.as_str());
        let bandwidth = args.get("bandwidth").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64());
        let base_raster = load_optional_raster_arg(args, "base_raster")?;
        let kernel = HeatKernel::parse(args.get("kernel_function").and_then(|v| v.as_str()));
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("running heat map");
        let weighted = collect_point_weights(&points, field_name)?;
        let mut output = build_point_interpolation_output(&points, &weighted, cell_size, base_raster, DataType::F64)?;

        let mut tree = KdTree::new(2);
        for (x, y, weight) in &weighted {
            tree.add([*x, *y], *weight)
                .map_err(|e| ToolError::Execution(format!("failed building heat-map index: {e}")))?;
        }

        let rows = output.rows;
        let cols = output.cols;
        let nodata = output.nodata;
        let x_min = output.x_min;
        let y_max = output.y_max();
        let cell_x = output.cell_size_x;
        let cell_y = output.cell_size_y;
        let bandwidth_sq = bandwidth * bandwidth;
        let mut out_values = vec![nodata; output.data.len()];

        for row in 0..rows {
            for col in 0..cols {
                let x = x_min + (col as f64 + 0.5) * cell_x;
                let y = y_max - (row as f64 + 0.5) * cell_y;
                let neighbours = tree
                    .within(&[x, y], bandwidth_sq, &squared_euclidean)
                    .map_err(|e| ToolError::Execution(format!("heat-map search failed: {e}")))?;
                if neighbours.is_empty() {
                    continue;
                }
                let mut density = 0.0;
                for (dist2, w) in neighbours {
                    let d = dist2.sqrt() / bandwidth;
                    density += *w * kernel.evaluate(d);
                }
                out_values[row * cols + col] = density;
            }
            ctx.progress.progress((row + 1) as f64 / rows.max(1) as f64 * 0.99);
        }

        for (index, value) in out_values.iter().enumerate() {
            output.data.set_f64(index, *value);
        }
        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(GisOverlayCore::build_result(locator))
    }
}

impl Tool for ExtractRasterValuesAtPointsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "extract_raster_values_at_points",
            display_name: "Extract Raster Values At Points",
            summary: "Samples one or more rasters at point locations and writes the values to point attributes.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "rasters", description: "Input raster paths as an array or delimited string.", required: true },
                ToolParamSpec { name: "points", description: "Input points vector layer.", required: true },
                ToolParamSpec { name: "output", description: "Output point vector path.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("rasters".to_string(), json!(["raster1.tif", "raster2.tif"]));
        defaults.insert("points".to_string(), json!("points.geojson"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("points_sampled.geojson"));
        ToolManifest {
            id: "extract_raster_values_at_points".to_string(),
            display_name: "Extract Raster Values At Points".to_string(),
            summary: "Samples one or more rasters at point locations and writes the values to point attributes.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "rasters".to_string(), description: "Input raster paths as an array or delimited string.".to_string(), required: true },
                ToolParamDescriptor { name: "points".to_string(), description: "Input points vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output point vector path.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "extract_raster_values_at_points_basic".to_string(),
                description: "Writes sampled raster values to new VALUE fields on a point layer.".to_string(),
                args: example_args,
            }],
            tags: vec!["vector".to_string(), "gis".to_string(), "sampling".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let rasters = load_required_raster_list_arg(args, "rasters")?;
        if rasters.is_empty() {
            return Err(ToolError::Validation(
                "rasters must contain at least one input raster".to_string(),
            ));
        }
        let points = load_vector_arg(args, "points")?;
        if points.features.is_empty() {
            return Err(ToolError::Validation(
                "points input must contain at least one feature".to_string(),
            ));
        }
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let rasters = load_required_raster_list_arg(args, "rasters")?;
        let input = load_vector_arg(args, "points")?;
        let output_path = parse_vector_path_arg(args, "output")?;

        let mut output = input.clone();
        output.name = format!("{}_sampled", input.name);
        for index in 0..rasters.len() {
            output.schema.add_field(wbvector::FieldDef::new(
                format!("VALUE{}", index + 1),
                wbvector::FieldType::Float,
            ));
        }

        let mut report = String::from("Point values:\n");
        for (feature_idx, feature) in output.features.iter_mut().enumerate() {
            let Some(geometry) = &feature.geometry else {
                continue;
            };
            let mut coords = Vec::new();
            collect_geometry_coords(geometry, &mut coords);
            let Some(coord) = coords.first() else {
                continue;
            };
            let mut sampled_values = Vec::with_capacity(rasters.len());
            for raster in &rasters {
                let col = ((coord.x - raster.x_min) / raster.cell_size_x).floor() as isize;
                let row = ((raster.y_max() - coord.y) / raster.cell_size_y).floor() as isize;
                let value = if row >= 0 && col >= 0 && row < raster.rows as isize && col < raster.cols as isize {
                    raster.get(0, row, col)
                } else {
                    raster.nodata
                };
                feature.attributes.push(wbvector::FieldValue::Float(value));
                sampled_values.push(value);
            }
            report.push_str(&format!("Point {} values: {:?}\n", feature_idx + 1, sampled_values));
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        let mut outputs = BTreeMap::new();
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("report".to_string(), json!(report));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for RasterCellAssignmentTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "raster_cell_assignment",
            display_name: "Raster Cell Assignment",
            summary: "Creates a raster derived from a base raster assigning row, column, x, or y values to each cell.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input base raster.", required: true },
                ToolParamSpec { name: "what_to_assign", description: "Assignment target: column, row, x, or y.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("what_to_assign".to_string(), json!("column"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("raster_cell_assignment.tif"));
        ToolManifest {
            id: "raster_cell_assignment".to_string(),
            display_name: "Raster Cell Assignment".to_string(),
            summary: "Creates a raster derived from a base raster assigning row, column, x, or y values to each cell.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input base raster.".to_string(), required: true },
                ToolParamDescriptor { name: "what_to_assign".to_string(), description: "Assignment target: column, row, x, or y.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "raster_cell_assignment_basic".to_string(),
                description: "Generates a raster of row, column, or coordinate values from a base raster.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "grid".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        let assign = args
            .get("what_to_assign")
            .and_then(|v| v.as_str())
            .unwrap_or("column")
            .to_ascii_lowercase();
        match assign.as_str() {
            "column" | "columns" | "col" | "row" | "rows" | "x" | "y" => Ok(()),
            _ => Err(ToolError::Validation(
                "what_to_assign must be one of column, row, x, or y".to_string(),
            )),
        }
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_required_raster_arg(args, "input")?;
        let assign = args
            .get("what_to_assign")
            .and_then(|v| v.as_str())
            .unwrap_or("column")
            .to_ascii_lowercase();
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("running raster cell assignment");
        let mut output = build_output_like_raster(&input, DataType::F64);
        let rows = output.rows;
        let cols = output.cols;
        for row in 0..rows {
            for col in 0..cols {
                let value = match assign.as_str() {
                    "column" | "columns" | "col" => col as f64,
                    "row" | "rows" => row as f64,
                    "x" => input.x_min + (col as f64 + 0.5) * input.cell_size_x,
                    "y" => input.y_max() - (row as f64 + 0.5) * input.cell_size_y,
                    _ => unreachable!(),
                };
                output.set(0, row as isize, col as isize, value).map_err(|e| {
                    ToolError::Execution(format!("failed assigning raster cell value: {e}"))
                })?;
            }
            ctx.progress.progress((row + 1) as f64 / rows.max(1) as f64 * 0.99);
        }
        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(GisOverlayCore::build_result(locator))
    }
}

impl Tool for LayerFootprintRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "layer_footprint_raster",
            display_name: "Layer Footprint Raster",
            summary: "Creates a polygon footprint representing the full extent of an input raster.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster.", required: true },
                ToolParamSpec { name: "output", description: "Output polygon vector path.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("layer_footprint_raster.geojson"));
        ToolManifest {
            id: "layer_footprint_raster".to_string(),
            display_name: "Layer Footprint Raster".to_string(),
            summary: "Creates a polygon footprint representing the full extent of an input raster.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input raster.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output polygon vector path.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "layer_footprint_raster_basic".to_string(),
                description: "Builds a rectangular polygon footprint from raster extent.".to_string(),
                args: example_args,
            }],
            tags: vec!["vector".to_string(), "gis".to_string(), "footprint".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_required_raster_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;

        let mut output = wbvector::Layer::new(format!("{}_footprint", output_path));
        output.geom_type = Some(wbvector::GeometryType::Polygon);
        output.crs = if input.crs.epsg.is_some() || input.crs.wkt.is_some() {
            Some(wbvector::Crs {
                epsg: input.crs.epsg,
                wkt: input.crs.wkt.clone(),
            })
        } else {
            None
        };
        output.schema.add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));

        let west = input.x_min;
        let east = input.x_min + input.cols as f64 * input.cell_size_x;
        let south = input.y_min;
        let north = input.y_max();
        let polygon = wbvector::Geometry::polygon(
            vec![
                wbvector::Coord::xy(west, south),
                wbvector::Coord::xy(east, south),
                wbvector::Coord::xy(east, north),
                wbvector::Coord::xy(west, north),
                wbvector::Coord::xy(west, south),
            ],
            vec![],
        );
        output.push(wbvector::Feature {
            fid: 1,
            geometry: Some(polygon),
            attributes: vec![wbvector::FieldValue::Integer(1)],
        });

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for LayerFootprintVectorTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "layer_footprint_vector",
            display_name: "Layer Footprint Vector",
            summary: "Creates a polygon footprint representing the full bounding extent of an input vector layer.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector layer.", required: true },
                ToolParamSpec { name: "output", description: "Output polygon vector path.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.geojson"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("layer_footprint_vector.geojson"));
        ToolManifest {
            id: "layer_footprint_vector".to_string(),
            display_name: "Layer Footprint Vector".to_string(),
            summary: "Creates a polygon footprint representing the full bounding extent of an input vector layer.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output polygon vector path.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "layer_footprint_vector_basic".to_string(),
                description: "Builds a rectangular polygon footprint from vector extent.".to_string(),
                args: example_args,
            }],
            tags: vec!["vector".to_string(), "gis".to_string(), "footprint".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let mut input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let bbox = input.bbox().ok_or_else(|| {
            ToolError::Validation("input vector layer must contain at least one geometry".to_string())
        })?;

        let mut output = wbvector::Layer::new(format!("{}_footprint", input.name));
        output.geom_type = Some(wbvector::GeometryType::Polygon);
        output.crs = input.crs.clone();
        output.schema.add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));
        output.push(wbvector::Feature {
            fid: 1,
            geometry: Some(wbvector::Geometry::polygon(
                vec![
                    wbvector::Coord::xy(bbox.min_x, bbox.min_y),
                    wbvector::Coord::xy(bbox.max_x, bbox.min_y),
                    wbvector::Coord::xy(bbox.max_x, bbox.max_y),
                    wbvector::Coord::xy(bbox.min_x, bbox.max_y),
                    wbvector::Coord::xy(bbox.min_x, bbox.min_y),
                ],
                vec![],
            )),
            attributes: vec![wbvector::FieldValue::Integer(1)],
        });

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

fn build_rectangular_grid_cells(
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    width: f64,
    height: f64,
    x_origin: f64,
    y_origin: f64,
) -> Vec<(i64, i64, Vec<wbvector::Coord>)> {
    let start_x_grid = ((min_x - x_origin) / width).floor() as i64;
    let end_x_grid = ((max_x - x_origin) / width).ceil() as i64;
    let start_y_grid = ((min_y - y_origin) / height).floor() as i64;
    let end_y_grid = ((max_y - y_origin) / height).ceil() as i64;

    let mut out = Vec::new();
    for row in start_y_grid..end_y_grid {
        for col in start_x_grid..end_x_grid {
            let x1 = (x_origin + col as f64 * width).clamp(min_x, max_x);
            let y1 = (y_origin + row as f64 * height).clamp(min_y, max_y);
            let x2 = (x_origin + col as f64 * width).clamp(min_x, max_x);
            let y2 = (y_origin + (row + 1) as f64 * height).clamp(min_y, max_y);
            let x3 = (x_origin + (col + 1) as f64 * width).clamp(min_x, max_x);
            let y3 = (y_origin + (row + 1) as f64 * height).clamp(min_y, max_y);
            let x4 = (x_origin + (col + 1) as f64 * width).clamp(min_x, max_x);
            let y4 = (y_origin + row as f64 * height).clamp(min_y, max_y);

            let ring = vec![
                wbvector::Coord::xy(x1, y1),
                wbvector::Coord::xy(x2, y2),
                wbvector::Coord::xy(x3, y3),
                wbvector::Coord::xy(x4, y4),
                wbvector::Coord::xy(x1, y1),
            ];

            out.push((row, col, ring));
        }
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HexOrientation {
    Horizontal,
    Vertical,
}

fn parse_hex_orientation(args: &ToolArgs, key: &str) -> HexOrientation {
    let raw = args
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("h")
        .to_ascii_lowercase();
    if raw.contains('v') {
        HexOrientation::Vertical
    } else {
        HexOrientation::Horizontal
    }
}

fn build_hexagonal_grid_cells(
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    width: f64,
    orientation: HexOrientation,
) -> Result<Vec<(i64, i64, Vec<wbvector::Coord>)>, ToolError> {
    let extent_width = max_x - min_x;
    let extent_height = max_y - min_y;

    let sixty_degrees = std::f64::consts::PI / 6.0;
    let half_width = 0.5 * width;
    let size = half_width / sixty_degrees.cos();
    let height = size * 2.0;
    let three_quarter_height = 0.75 * height;

    let mut out = Vec::new();

    match orientation {
        HexOrientation::Horizontal => {
            let center_x_0 = min_x + half_width;
            let center_y_0 = max_y - 0.25 * height;
            let rows = (extent_height / three_quarter_height).ceil().max(0.0) as i64;
            let mut columns = (extent_width / width).ceil().max(0.0) as i64;

            if rows > 0 && columns > 0 && (rows as i128) * (columns as i128) > 100_000 {
                return Err(ToolError::Validation(
                    "this operation would produce a vector file with too many polygons; increase width"
                        .to_string(),
                ));
            }

            for row in 0..rows {
                let center_y = center_y_0 - row as f64 * three_quarter_height;
                columns = ((extent_width + half_width * (row as f64 % 2.0)) / width)
                    .ceil()
                    .max(0.0) as i64;
                for col in 0..columns {
                    let center_x = (center_x_0 - half_width * (row as f64 % 2.0))
                        + col as f64 * width;
                    let mut ring = Vec::with_capacity(7);
                    for i in (0..=6).rev() {
                        let angle = 2.0 * sixty_degrees * (i as f64 + 0.5);
                        ring.push(wbvector::Coord::xy(
                            center_x + size * angle.cos(),
                            center_y + size * angle.sin(),
                        ));
                    }
                    out.push((row, col, ring));
                }
            }
        }
        HexOrientation::Vertical => {
            let center_x_0 = min_x + 0.5 * size;
            let center_y_0 = max_y - half_width;
            let mut rows = (extent_height / width).ceil().max(0.0) as i64;
            let columns = (extent_width / three_quarter_height).ceil().max(0.0) as i64;

            if rows > 0 && columns > 0 && (rows as i128) * (columns as i128) > 100_000 {
                return Err(ToolError::Validation(
                    "this operation would produce a vector file with too many polygons; increase width"
                        .to_string(),
                ));
            }

            for col in 0..columns {
                rows = ((extent_height + (col as f64 % 2.0) * half_width) / width)
                    .ceil()
                    .max(0.0) as i64;
                for row in 0..rows {
                    let center_x = center_x_0 + col as f64 * three_quarter_height;
                    let center_y = center_y_0 - row as f64 * width + (col as f64 % 2.0) * half_width;
                    let mut ring = Vec::with_capacity(7);
                    for i in (0..=6).rev() {
                        let angle = 2.0 * sixty_degrees * (i as f64 + 0.5) - sixty_degrees;
                        ring.push(wbvector::Coord::xy(
                            center_x + size * angle.cos(),
                            center_y + size * angle.sin(),
                        ));
                    }
                    out.push((row, col, ring));
                }
            }
        }
    }

    Ok(out)
}

impl Tool for HexagonalGridFromRasterBaseTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "hexagonal_grid_from_raster_base",
            display_name: "Hexagonal Grid From Raster Base",
            summary: "Creates a hexagonal polygon grid covering a raster extent.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "base", description: "Base raster controlling grid extent.", required: true },
                ToolParamSpec { name: "width", description: "Hexagon width in map units.", required: true },
                ToolParamSpec { name: "orientation", description: "Hexagon orientation: horizontal or vertical.", required: false },
                ToolParamSpec { name: "output", description: "Output polygon vector path.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("base".to_string(), json!("base.tif"));
        defaults.insert("width".to_string(), json!(100.0));
        defaults.insert("orientation".to_string(), json!("h"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("hex_grid_from_raster.geojson"));

        ToolManifest {
            id: "hexagonal_grid_from_raster_base".to_string(),
            display_name: "Hexagonal Grid From Raster Base".to_string(),
            summary: "Creates a hexagonal polygon grid covering a raster extent.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "base".to_string(), description: "Base raster controlling grid extent.".to_string(), required: true },
                ToolParamDescriptor { name: "width".to_string(), description: "Hexagon width in map units.".to_string(), required: true },
                ToolParamDescriptor { name: "orientation".to_string(), description: "Hexagon orientation: horizontal or vertical.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output polygon vector path.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "hexagonal_grid_from_raster_base_basic".to_string(),
                description: "Builds a hexagonal grid from raster bounds.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "grid".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "base")?;
        let width = args
            .get("width")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'width' is required".to_string()))?;
        if !width.is_finite() || width <= 0.0 {
            return Err(ToolError::Validation(
                "width must be a finite value > 0".to_string(),
            ));
        }
        let _ = parse_hex_orientation(args, "orientation");
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let base = load_required_raster_arg(args, "base")?;
        let width = args
            .get("width")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'width' is required".to_string()))?;
        let orientation = parse_hex_orientation(args, "orientation");
        let output_path = parse_vector_path_arg(args, "output")?;

        let min_x = base.x_min;
        let max_x = base.x_min + base.cols as f64 * base.cell_size_x;
        let min_y = base.y_min;
        let max_y = base.y_max();
        let cells = build_hexagonal_grid_cells(min_x, max_x, min_y, max_y, width, orientation)?;

        let mut output = wbvector::Layer::new("hexagonal_grid_from_raster_base".to_string());
        output.geom_type = Some(wbvector::GeometryType::Polygon);
        output.crs = if base.crs.epsg.is_some() || base.crs.wkt.is_some() {
            Some(wbvector::Crs {
                epsg: base.crs.epsg,
                wkt: base.crs.wkt.clone(),
            })
        } else {
            None
        };
        output.schema.add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));
        output.schema.add_field(wbvector::FieldDef::new("ROW", wbvector::FieldType::Integer));
        output.schema.add_field(wbvector::FieldDef::new("COLUMN", wbvector::FieldType::Integer));

        let total = cells.len().max(1);
        for (idx, (row, col, ring)) in cells.into_iter().enumerate() {
            let fid = (idx + 1) as i64;
            output.push(wbvector::Feature {
                fid: fid as u64,
                geometry: Some(wbvector::Geometry::polygon(ring, vec![])),
                attributes: vec![
                    wbvector::FieldValue::Integer(fid),
                    wbvector::FieldValue::Integer(row),
                    wbvector::FieldValue::Integer(col),
                ],
            });
            ctx.progress.progress((idx + 1) as f64 / total as f64);
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for HexagonalGridFromVectorBaseTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "hexagonal_grid_from_vector_base",
            display_name: "Hexagonal Grid From Vector Base",
            summary: "Creates a hexagonal polygon grid covering a vector-layer bounding extent.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "base", description: "Base vector layer controlling grid extent.", required: true },
                ToolParamSpec { name: "width", description: "Hexagon width in map units.", required: true },
                ToolParamSpec { name: "orientation", description: "Hexagon orientation: horizontal or vertical.", required: false },
                ToolParamSpec { name: "output", description: "Output polygon vector path.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("base".to_string(), json!("base.geojson"));
        defaults.insert("width".to_string(), json!(100.0));
        defaults.insert("orientation".to_string(), json!("h"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("hex_grid_from_vector.geojson"));

        ToolManifest {
            id: "hexagonal_grid_from_vector_base".to_string(),
            display_name: "Hexagonal Grid From Vector Base".to_string(),
            summary: "Creates a hexagonal polygon grid covering a vector-layer bounding extent."
                .to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "base".to_string(), description: "Base vector layer controlling grid extent.".to_string(), required: true },
                ToolParamDescriptor { name: "width".to_string(), description: "Hexagon width in map units.".to_string(), required: true },
                ToolParamDescriptor { name: "orientation".to_string(), description: "Hexagon orientation: horizontal or vertical.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output polygon vector path.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "hexagonal_grid_from_vector_base_basic".to_string(),
                description: "Builds a hexagonal grid from vector bounds.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "grid".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let base = load_vector_arg(args, "base")?;
        if base.features.is_empty() {
            return Err(ToolError::Validation(
                "base vector layer must contain at least one feature".to_string(),
            ));
        }
        let width = args
            .get("width")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'width' is required".to_string()))?;
        if !width.is_finite() || width <= 0.0 {
            return Err(ToolError::Validation(
                "width must be a finite value > 0".to_string(),
            ));
        }
        let _ = parse_hex_orientation(args, "orientation");
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let mut base = load_vector_arg(args, "base")?;
        let width = args
            .get("width")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'width' is required".to_string()))?;
        let orientation = parse_hex_orientation(args, "orientation");
        let output_path = parse_vector_path_arg(args, "output")?;

        let bbox = base.bbox().ok_or_else(|| {
            ToolError::Validation("base vector layer must contain at least one geometry".to_string())
        })?;
        let cells = build_hexagonal_grid_cells(
            bbox.min_x,
            bbox.max_x,
            bbox.min_y,
            bbox.max_y,
            width,
            orientation,
        )?;

        let mut output = wbvector::Layer::new(format!("{}_hex_grid", base.name));
        output.geom_type = Some(wbvector::GeometryType::Polygon);
        output.crs = base.crs.clone();
        output.schema.add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));
        output.schema.add_field(wbvector::FieldDef::new("ROW", wbvector::FieldType::Integer));
        output.schema.add_field(wbvector::FieldDef::new("COLUMN", wbvector::FieldType::Integer));

        let total = cells.len().max(1);
        for (idx, (row, col, ring)) in cells.into_iter().enumerate() {
            let fid = (idx + 1) as i64;
            output.push(wbvector::Feature {
                fid: fid as u64,
                geometry: Some(wbvector::Geometry::polygon(ring, vec![])),
                attributes: vec![
                    wbvector::FieldValue::Integer(fid),
                    wbvector::FieldValue::Integer(row),
                    wbvector::FieldValue::Integer(col),
                ],
            });
            ctx.progress.progress((idx + 1) as f64 / total as f64);
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for RectangularGridFromRasterBaseTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "rectangular_grid_from_raster_base",
            display_name: "Rectangular Grid From Raster Base",
            summary: "Creates a rectangular polygon grid covering a raster extent.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "base", description: "Base raster controlling grid extent.", required: true },
                ToolParamSpec { name: "width", description: "Grid cell width in map units.", required: true },
                ToolParamSpec { name: "height", description: "Grid cell height in map units.", required: true },
                ToolParamSpec { name: "x_origin", description: "Optional x-origin for grid alignment.", required: false },
                ToolParamSpec { name: "y_origin", description: "Optional y-origin for grid alignment.", required: false },
                ToolParamSpec { name: "output", description: "Output polygon vector path.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("base".to_string(), json!("base.tif"));
        defaults.insert("width".to_string(), json!(100.0));
        defaults.insert("height".to_string(), json!(100.0));
        defaults.insert("x_origin".to_string(), json!(0.0));
        defaults.insert("y_origin".to_string(), json!(0.0));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("rect_grid_from_raster.geojson"));

        ToolManifest {
            id: "rectangular_grid_from_raster_base".to_string(),
            display_name: "Rectangular Grid From Raster Base".to_string(),
            summary: "Creates a rectangular polygon grid covering a raster extent.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "base".to_string(), description: "Base raster controlling grid extent.".to_string(), required: true },
                ToolParamDescriptor { name: "width".to_string(), description: "Grid cell width in map units.".to_string(), required: true },
                ToolParamDescriptor { name: "height".to_string(), description: "Grid cell height in map units.".to_string(), required: true },
                ToolParamDescriptor { name: "x_origin".to_string(), description: "Optional x-origin for grid alignment.".to_string(), required: false },
                ToolParamDescriptor { name: "y_origin".to_string(), description: "Optional y-origin for grid alignment.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output polygon vector path.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "rectangular_grid_from_raster_base_basic".to_string(),
                description: "Builds a rectangular grid from raster bounds.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "grid".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "base")?;
        let width = args
            .get("width")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'width' is required".to_string()))?;
        let height = args
            .get("height")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'height' is required".to_string()))?;
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(ToolError::Validation(
                "width and height must be finite values > 0".to_string(),
            ));
        }
        if let Some(v) = args.get("x_origin").and_then(|v| v.as_f64()) {
            if !v.is_finite() {
                return Err(ToolError::Validation("x_origin must be finite".to_string()));
            }
        }
        if let Some(v) = args.get("y_origin").and_then(|v| v.as_f64()) {
            if !v.is_finite() {
                return Err(ToolError::Validation("y_origin must be finite".to_string()));
            }
        }
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let base = load_required_raster_arg(args, "base")?;
        let width = args
            .get("width")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'width' is required".to_string()))?;
        let height = args
            .get("height")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'height' is required".to_string()))?;
        let x_origin = args.get("x_origin").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let y_origin = args.get("y_origin").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let output_path = parse_vector_path_arg(args, "output")?;

        let min_x = base.x_min;
        let max_x = base.x_min + base.cols as f64 * base.cell_size_x;
        let min_y = base.y_min;
        let max_y = base.y_max();

        let cells = build_rectangular_grid_cells(min_x, max_x, min_y, max_y, width, height, x_origin, y_origin);

        let mut output = wbvector::Layer::new("rectangular_grid_from_raster_base".to_string());
        output.geom_type = Some(wbvector::GeometryType::Polygon);
        output.crs = if base.crs.epsg.is_some() || base.crs.wkt.is_some() {
            Some(wbvector::Crs {
                epsg: base.crs.epsg,
                wkt: base.crs.wkt.clone(),
            })
        } else {
            None
        };
        output.schema.add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));
        output.schema.add_field(wbvector::FieldDef::new("ROW", wbvector::FieldType::Integer));
        output.schema.add_field(wbvector::FieldDef::new("COLUMN", wbvector::FieldType::Integer));

        let total = cells.len().max(1);
        for (idx, (row, col, ring)) in cells.into_iter().enumerate() {
            let fid = (idx + 1) as i64;
            output.push(wbvector::Feature {
                fid: fid as u64,
                geometry: Some(wbvector::Geometry::polygon(ring, vec![])),
                attributes: vec![
                    wbvector::FieldValue::Integer(fid),
                    wbvector::FieldValue::Integer(row),
                    wbvector::FieldValue::Integer(col),
                ],
            });
            ctx.progress.progress((idx + 1) as f64 / total as f64);
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for RectangularGridFromVectorBaseTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "rectangular_grid_from_vector_base",
            display_name: "Rectangular Grid From Vector Base",
            summary: "Creates a rectangular polygon grid covering a vector-layer bounding extent.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "base", description: "Base vector layer controlling grid extent.", required: true },
                ToolParamSpec { name: "width", description: "Grid cell width in map units.", required: true },
                ToolParamSpec { name: "height", description: "Grid cell height in map units.", required: true },
                ToolParamSpec { name: "x_origin", description: "Optional x-origin for grid alignment.", required: false },
                ToolParamSpec { name: "y_origin", description: "Optional y-origin for grid alignment.", required: false },
                ToolParamSpec { name: "output", description: "Output polygon vector path.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("base".to_string(), json!("base.geojson"));
        defaults.insert("width".to_string(), json!(100.0));
        defaults.insert("height".to_string(), json!(100.0));
        defaults.insert("x_origin".to_string(), json!(0.0));
        defaults.insert("y_origin".to_string(), json!(0.0));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("rect_grid_from_vector.geojson"));

        ToolManifest {
            id: "rectangular_grid_from_vector_base".to_string(),
            display_name: "Rectangular Grid From Vector Base".to_string(),
            summary: "Creates a rectangular polygon grid covering a vector-layer bounding extent."
                .to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "base".to_string(), description: "Base vector layer controlling grid extent.".to_string(), required: true },
                ToolParamDescriptor { name: "width".to_string(), description: "Grid cell width in map units.".to_string(), required: true },
                ToolParamDescriptor { name: "height".to_string(), description: "Grid cell height in map units.".to_string(), required: true },
                ToolParamDescriptor { name: "x_origin".to_string(), description: "Optional x-origin for grid alignment.".to_string(), required: false },
                ToolParamDescriptor { name: "y_origin".to_string(), description: "Optional y-origin for grid alignment.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output polygon vector path.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "rectangular_grid_from_vector_base_basic".to_string(),
                description: "Builds a rectangular grid from vector bounds.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "grid".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let base = load_vector_arg(args, "base")?;
        if base.features.is_empty() {
            return Err(ToolError::Validation(
                "base vector layer must contain at least one feature".to_string(),
            ));
        }
        let width = args
            .get("width")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'width' is required".to_string()))?;
        let height = args
            .get("height")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'height' is required".to_string()))?;
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(ToolError::Validation(
                "width and height must be finite values > 0".to_string(),
            ));
        }
        if let Some(v) = args.get("x_origin").and_then(|v| v.as_f64()) {
            if !v.is_finite() {
                return Err(ToolError::Validation("x_origin must be finite".to_string()));
            }
        }
        if let Some(v) = args.get("y_origin").and_then(|v| v.as_f64()) {
            if !v.is_finite() {
                return Err(ToolError::Validation("y_origin must be finite".to_string()));
            }
        }
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let mut base = load_vector_arg(args, "base")?;
        let width = args
            .get("width")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'width' is required".to_string()))?;
        let height = args
            .get("height")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'height' is required".to_string()))?;
        let x_origin = args.get("x_origin").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let y_origin = args.get("y_origin").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let output_path = parse_vector_path_arg(args, "output")?;

        let bbox = base.bbox().ok_or_else(|| {
            ToolError::Validation("base vector layer must contain at least one geometry".to_string())
        })?;
        let cells = build_rectangular_grid_cells(
            bbox.min_x,
            bbox.max_x,
            bbox.min_y,
            bbox.max_y,
            width,
            height,
            x_origin,
            y_origin,
        );

        let mut output = wbvector::Layer::new(format!("{}_rect_grid", base.name));
        output.geom_type = Some(wbvector::GeometryType::Polygon);
        output.crs = base.crs.clone();
        output.schema.add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));
        output.schema.add_field(wbvector::FieldDef::new("ROW", wbvector::FieldType::Integer));
        output.schema.add_field(wbvector::FieldDef::new("COLUMN", wbvector::FieldType::Integer));

        let total = cells.len().max(1);
        for (idx, (row, col, ring)) in cells.into_iter().enumerate() {
            let fid = (idx + 1) as i64;
            output.push(wbvector::Feature {
                fid: fid as u64,
                geometry: Some(wbvector::Geometry::polygon(ring, vec![])),
                attributes: vec![
                    wbvector::FieldValue::Integer(fid),
                    wbvector::FieldValue::Integer(row),
                    wbvector::FieldValue::Integer(col),
                ],
            });
            ctx.progress.progress((idx + 1) as f64 / total as f64);
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for MapFeaturesTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "map_features",
            display_name: "Map Features",
            summary: "Maps discrete elevated terrain features from a raster using descending-priority region growth.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster.", required: true },
                ToolParamSpec { name: "min_feature_height", description: "Minimum vertical separation for independent features.", required: true },
                ToolParamSpec { name: "min_feature_size", description: "Minimum feature size in cells.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("min_feature_height".to_string(), json!(1.0));
        defaults.insert("min_feature_size".to_string(), json!(1));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("map_features.tif"));
        ToolManifest {
            id: "map_features".to_string(),
            display_name: "Map Features".to_string(),
            summary: "Maps discrete elevated terrain features from a raster using descending-priority region growth.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input raster.".to_string(), required: true },
                ToolParamDescriptor { name: "min_feature_height".to_string(), description: "Minimum vertical separation for independent features.".to_string(), required: true },
                ToolParamDescriptor { name: "min_feature_size".to_string(), description: "Minimum feature size in cells.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "map_features_basic".to_string(),
                description: "Labels terrain features using descending-elevation region growth.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "features".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "input")?;
        let min_feature_height = args.get("min_feature_height").and_then(|v| v.as_f64()).unwrap_or(f64::NAN);
        if !min_feature_height.is_finite() {
            return Err(ToolError::Validation("min_feature_height must be finite".to_string()));
        }
        let min_feature_size = args.get("min_feature_size").and_then(|v| v.as_u64()).unwrap_or(0);
        if min_feature_size == 0 {
            return Err(ToolError::Validation("min_feature_size must be >= 1".to_string()));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_required_raster_arg(args, "input")?;
        let min_feature_height = args.get("min_feature_height").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let min_feature_size = args.get("min_feature_size").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
        let output_path = parse_optional_output_path(args, "output")?;

        let rows = input.rows;
        let cols = input.cols;
        let out_nodata = -999.0;
        let mut labels = vec![out_nodata; rows * cols];
        let mut heap = BinaryHeap::new();
        let mut processed = 0usize;
        let total = rows * cols;
        for row in 0..rows {
            for col in 0..cols {
                let z = input.get(0, row as isize, col as isize);
                if input.is_nodata(z) {
                    processed += 1;
                } else {
                    heap.push(MapFeatureGridCell { row, col, priority: z });
                }
            }
        }

        let dx = [1isize, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1isize, 0, 1, 1, 1, 0, -1, -1];
        let mut feature_id = 0i32;
        let mut closed_features: HashSet<i32> = HashSet::new();
        let mut continuing_feature: HashMap<i32, i32> = HashMap::new();
        let mut feature_max_z = vec![0.0f64];
        let mut feature_min_z = vec![0.0f64];
        let mut replacement_feature: HashMap<i32, i32> = HashMap::new();
        let mut override_features: Vec<Vec<i32>> = vec![vec![]];

        while let Some(cell) = heap.pop() {
            let row = cell.row;
            let col = cell.col;
            let z = cell.priority;
            let mut assigned_features = HashSet::new();
            for n in 0..8 {
                let nr = row as isize + dy[n];
                let nc = col as isize + dx[n];
                if nr < 0 || nc < 0 || nr >= rows as isize || nc >= cols as isize {
                    continue;
                }
                let label = labels[nr as usize * cols + nc as usize];
                if label > 0.0 {
                    assigned_features.insert(label as i32);
                }
            }

            match assigned_features.len() {
                1 => {
                    let neighbour_id = *assigned_features.iter().next().unwrap();
                    let final_id = if !closed_features.contains(&neighbour_id) {
                        neighbour_id
                    } else {
                        *continuing_feature.get(&neighbour_id).unwrap_or(&neighbour_id)
                    };
                    labels[row * cols + col] = final_id as f64;
                    if final_id as usize >= feature_min_z.len() {
                        feature_min_z.resize(final_id as usize + 1, z);
                    }
                    feature_min_z[final_id as usize] = z;
                }
                n if n > 1 => {
                    let mut highest_feature = i32::MAX;
                    for feature in &assigned_features {
                        if *feature < highest_feature {
                            highest_feature = *feature;
                        }
                    }
                    let mut open = false;
                    for feature in &assigned_features {
                        if !closed_features.contains(feature) {
                            open = true;
                            break;
                        }
                    }
                    if open {
                        for feature in &assigned_features {
                            if *feature != highest_feature {
                                closed_features.insert(*feature);
                                if feature_max_z[*feature as usize] - z < min_feature_height {
                                    override_features[highest_feature as usize].push(*feature);
                                    let values = override_features[*feature as usize].clone();
                                    override_features[highest_feature as usize].extend_from_slice(&values);
                                    override_features[*feature as usize].clear();
                                }
                            }
                            continuing_feature.insert(*feature, highest_feature);
                        }
                    }
                    let final_id = *continuing_feature.get(&highest_feature).unwrap_or(&highest_feature);
                    labels[row * cols + col] = final_id as f64;
                    if final_id as usize >= feature_min_z.len() {
                        feature_min_z.resize(final_id as usize + 1, z);
                    }
                    feature_min_z[final_id as usize] = z;
                }
                _ => {
                    feature_id += 1;
                    labels[row * cols + col] = feature_id as f64;
                    feature_max_z.push(z);
                    feature_min_z.push(z);
                    override_features.push(vec![]);
                }
            }
            processed += 1;
            if processed % 128 == 0 {
                ctx.progress.progress(processed as f64 / total.max(1) as f64 * 0.45);
            }
        }

        for (replacement_id, features) in override_features.iter().enumerate() {
            if features.is_empty() {
                continue;
            }
            for feature in features {
                replacement_feature.insert(*feature, replacement_id as i32);
            }
        }
        for value in &mut labels {
            if *value > 0.0 {
                if let Some(replacement_id) = replacement_feature.get(&(*value as i32)) {
                    *value = *replacement_id as f64;
                }
            }
        }

        let mut visited = vec![false; rows * cols];
        let mut clump_sizes = vec![0usize];
        let mut next_clump = 1usize;
        for row in 0..rows {
            for col in 0..cols {
                let idx = row * cols + col;
                let label = labels[idx];
                if label <= 0.0 || visited[idx] {
                    continue;
                }
                let source_label = label;
                let mut stack = vec![(row, col)];
                visited[idx] = true;
                labels[idx] = next_clump as f64;
                let mut count = 1usize;
                while let Some((r, c)) = stack.pop() {
                    for n in 0..8 {
                        let nr = r as isize + dy[n];
                        let nc = c as isize + dx[n];
                        if nr < 0 || nc < 0 || nr >= rows as isize || nc >= cols as isize {
                            continue;
                        }
                        let nidx = nr as usize * cols + nc as usize;
                        if !visited[nidx] && labels[nidx] == source_label {
                            visited[nidx] = true;
                            labels[nidx] = next_clump as f64;
                            stack.push((nr as usize, nc as usize));
                            count += 1;
                        }
                    }
                }
                clump_sizes.push(count);
                next_clump += 1;
            }
            if row % 16 == 0 {
                ctx.progress.progress(0.45 + (row + 1) as f64 / rows.max(1) as f64 * 0.2);
            }
        }

        loop {
            let num_small = clump_sizes.iter().filter(|&&x| x > 0 && x < min_feature_size).count();
            if num_small == 0 {
                break;
            }
            let mut boundary_counts: Vec<HashMap<i32, usize>> = vec![HashMap::new(); clump_sizes.len()];
            let mut longest_neighbour = vec![0i32; clump_sizes.len()];
            let mut longest_boundary = vec![0usize; clump_sizes.len()];
            for row in 0..rows {
                for col in 0..cols {
                    let idx = row * cols + col;
                    let feature = labels[idx];
                    if feature <= 0.0 {
                        continue;
                    }
                    let feature_usize = feature as usize;
                    for n in 0..8 {
                        let nr = row as isize + dy[n];
                        let nc = col as isize + dx[n];
                        if nr < 0 || nc < 0 || nr >= rows as isize || nc >= cols as isize {
                            continue;
                        }
                        let neighbour = labels[nr as usize * cols + nc as usize];
                        if neighbour > 0.0 && neighbour != feature {
                            let entry = boundary_counts[feature_usize].entry(neighbour as i32).or_insert(0);
                            *entry += 1;
                            if *entry > longest_boundary[feature_usize] {
                                longest_boundary[feature_usize] = *entry;
                                longest_neighbour[feature_usize] = neighbour as i32;
                            }
                        }
                    }
                }
            }

            let mut changed = false;
            for row in 0..rows {
                for col in 0..cols {
                    let idx = row * cols + col;
                    let feature = labels[idx];
                    if feature <= 0.0 {
                        continue;
                    }
                    let feature_usize = feature as usize;
                    if clump_sizes[feature_usize] > 0 && clump_sizes[feature_usize] < min_feature_size {
                        let replacement = longest_neighbour[feature_usize];
                        if replacement > 0 {
                            let source = feature;
                            let mut stack = vec![(row, col)];
                            labels[idx] = replacement as f64;
                            while let Some((r, c)) = stack.pop() {
                                for n in 0..8 {
                                    let nr = r as isize + dy[n];
                                    let nc = c as isize + dx[n];
                                    if nr < 0 || nc < 0 || nr >= rows as isize || nc >= cols as isize {
                                        continue;
                                    }
                                    let nidx = nr as usize * cols + nc as usize;
                                    if labels[nidx] == source {
                                        labels[nidx] = replacement as f64;
                                        stack.push((nr as usize, nc as usize));
                                    }
                                }
                            }
                            clump_sizes[replacement as usize] += clump_sizes[feature_usize];
                            clump_sizes[feature_usize] = 0;
                            changed = true;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }

        let mut remap = HashMap::new();
        let mut next_id = 1i32;
        for value in &mut labels {
            if *value > 0.0 {
                let src = *value as i32;
                let dst = *remap.entry(src).or_insert_with(|| {
                    let id = next_id;
                    next_id += 1;
                    id
                });
                *value = dst as f64;
            }
        }

        let mut output = Raster::new(RasterConfig {
            cols,
            rows,
            bands: 1,
            x_min: input.x_min,
            y_min: input.y_min,
            cell_size: input.cell_size_x,
            cell_size_y: Some(input.cell_size_y),
            nodata: out_nodata,
            data_type: DataType::F64,
            crs: input.crs.clone(),
            metadata: input.metadata.clone(),
        });
        for row in 0..rows {
            for col in 0..cols {
                output.set(0, row as isize, col as isize, labels[row * cols + col]).map_err(|e| {
                    ToolError::Execution(format!("failed writing mapped feature raster: {e}"))
                })?;
            }
            if row % 16 == 0 {
                ctx.progress.progress(0.65 + (row + 1) as f64 / rows.max(1) as f64 * 0.34);
            }
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        ctx.progress.progress(1.0);
        Ok(GisOverlayCore::build_result(locator))
    }
}

#[derive(Debug, PartialEq)]
struct MapFeatureGridCell {
    row: usize,
    col: usize,
    priority: f64,
}

impl Eq for MapFeatureGridCell {}

impl PartialOrd for MapFeatureGridCell {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.priority.partial_cmp(&other.priority)
    }
}

impl Ord for MapFeatureGridCell {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

fn median_value(values: &mut [f64]) -> f64 {
    values.sort_by(|a, b| a.total_cmp(b));
    if values.is_empty() {
        return 0.0;
    }
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        values[mid]
    } else {
        (values[mid - 1] + values[mid]) * 0.5
    }
}

fn medoid_coordinate_index(coords: &[wbvector::Coord]) -> Option<usize> {
    if coords.is_empty() {
        return None;
    }
    let mut xs = coords.iter().map(|c| c.x).collect::<Vec<_>>();
    let mut ys = coords.iter().map(|c| c.y).collect::<Vec<_>>();
    let medx = median_value(&mut xs);
    let medy = median_value(&mut ys);

    let mut best_idx = 0usize;
    let mut best_dist = f64::INFINITY;
    for (idx, coord) in coords.iter().enumerate() {
        let dx = coord.x - medx;
        let dy = coord.y - medy;
        let dist = dx * dx + dy * dy;
        if dist < best_dist {
            best_dist = dist;
            best_idx = idx;
        }
    }
    Some(best_idx)
}

impl Tool for CreatePlaneTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "create_plane",
            display_name: "Create Plane",
            summary: "Creates a raster from a planar equation using a base raster geometry.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "base", description: "Base raster that defines output geometry.", required: true },
                ToolParamSpec { name: "gradient", description: "Plane slope gradient in degrees.", required: true },
                ToolParamSpec { name: "aspect", description: "Plane aspect in degrees.", required: true },
                ToolParamSpec { name: "constant", description: "Additive constant term in plane equation.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("base".to_string(), json!("base.tif"));
        defaults.insert("gradient".to_string(), json!(10.0));
        defaults.insert("aspect".to_string(), json!(315.0));
        defaults.insert("constant".to_string(), json!(0.0));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("create_plane.tif"));

        ToolManifest {
            id: "create_plane".to_string(),
            display_name: "Create Plane".to_string(),
            summary: "Creates a raster from a planar equation using a base raster geometry.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "base".to_string(), description: "Base raster that defines output geometry.".to_string(), required: true },
                ToolParamDescriptor { name: "gradient".to_string(), description: "Plane slope gradient in degrees.".to_string(), required: true },
                ToolParamDescriptor { name: "aspect".to_string(), description: "Plane aspect in degrees.".to_string(), required: true },
                ToolParamDescriptor { name: "constant".to_string(), description: "Additive constant term in plane equation.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "create_plane_basic".to_string(),
                description: "Creates a simple planar raster from gradient/aspect/constant parameters.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "surface".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "base")?;
        let gradient = args.get("gradient").and_then(|v| v.as_f64()).unwrap_or(f64::NAN);
        if !gradient.is_finite() {
            return Err(ToolError::Validation("gradient must be finite".to_string()));
        }
        let aspect = args.get("aspect").and_then(|v| v.as_f64()).unwrap_or(f64::NAN);
        if !aspect.is_finite() {
            return Err(ToolError::Validation("aspect must be finite".to_string()));
        }
        let constant = args.get("constant").and_then(|v| v.as_f64()).unwrap_or(f64::NAN);
        if !constant.is_finite() {
            return Err(ToolError::Validation("constant must be finite".to_string()));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let base = load_required_raster_arg(args, "base")?;
        let gradient = args.get("gradient").and_then(|v| v.as_f64()).unwrap_or(0.0).clamp(-85.0, 85.0);
        let aspect = args.get("aspect").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let constant = args.get("constant").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let output_path = parse_optional_output_path(args, "output")?;

        let slope = gradient.to_radians().tan();
        let aspect_radians = (aspect - 180.0).to_radians();
        let rows = base.rows;
        let cols = base.cols;

        let mut output = Raster::new(RasterConfig {
            cols,
            rows,
            bands: 1,
            x_min: base.x_min,
            y_min: base.y_min,
            cell_size: base.cell_size_x,
            cell_size_y: Some(base.cell_size_y),
            nodata: -32768.0,
            data_type: DataType::F32,
            crs: base.crs.clone(),
            metadata: base.metadata.clone(),
        });

        for row in 0..rows {
            for col in 0..cols {
                let x = base.col_center_x(col as isize);
                let y = base.row_center_y(row as isize);
                let z = slope * aspect_radians.sin() * x + slope * aspect_radians.cos() * y + constant;
                output
                    .set(0, row as isize, col as isize, z)
                    .map_err(|e| ToolError::Execution(format!("failed writing output raster: {e}")))?;
            }
            if row % 16 == 0 {
                ctx.progress.progress((row + 1) as f64 / rows.max(1) as f64);
            }
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        Ok(GisOverlayCore::build_result(locator))
    }
}

impl Tool for CentroidRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "centroid_raster",
            display_name: "Centroid Raster",
            summary: "Calculates the centroid cell for each positive-valued patch ID in a raster.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input patch raster.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("centroid_raster.tif"));

        ToolManifest {
            id: "centroid_raster".to_string(),
            display_name: "Centroid Raster".to_string(),
            summary: "Calculates the centroid cell for each positive-valued patch ID in a raster.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input patch raster.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "centroid_raster_basic".to_string(),
                description: "Writes patch IDs only at centroid cell positions.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "centroid".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_required_raster_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?;

        let rows = input.rows;
        let cols = input.cols;
        let mut sums: HashMap<i64, (usize, usize, usize)> = HashMap::new();
        for row in 0..rows {
            for col in 0..cols {
                let value = input.get(0, row as isize, col as isize);
                if input.is_nodata(value) || value <= 0.0 {
                    continue;
                }
                let key = value as i64;
                let entry = sums.entry(key).or_insert((0, 0, 0));
                entry.0 += row;
                entry.1 += col;
                entry.2 += 1;
            }
            if row % 16 == 0 {
                ctx.progress.progress((row + 1) as f64 / rows.max(1) as f64 * 0.5);
            }
        }

        let mut output = Raster::new(RasterConfig {
            cols,
            rows,
            bands: 1,
            x_min: input.x_min,
            y_min: input.y_min,
            cell_size: input.cell_size_x,
            cell_size_y: Some(input.cell_size_y),
            nodata: input.nodata,
            data_type: DataType::F64,
            crs: input.crs.clone(),
            metadata: input.metadata.clone(),
        });
        for row in 0..rows {
            for col in 0..cols {
                let v = input.get(0, row as isize, col as isize);
                output
                    .set(0, row as isize, col as isize, if input.is_nodata(v) { input.nodata } else { 0.0 })
                    .map_err(|e| ToolError::Execution(format!("failed initializing centroid output raster: {e}")))?;
            }
        }

        let mut report = String::from("Patch Centroid\nPatch ID\tColumn\tRow\n");
        for (patch_id, (sum_row, sum_col, count)) in &sums {
            if *count == 0 {
                continue;
            }
            let centroid_row = (sum_row / count) as isize;
            let centroid_col = (sum_col / count) as isize;
            output
                .set(0, centroid_row, centroid_col, *patch_id as f64)
                .map_err(|e| ToolError::Execution(format!("failed writing patch centroid cell: {e}")))?;
            let mean_col = *sum_col as f64 / *count as f64;
            let mean_row = *sum_row as f64 / *count as f64;
            report.push_str(&format!("{}\t{}\t{}\n", patch_id, mean_col, mean_row));
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("path".to_string(), json!(locator));
        outputs.insert("report".to_string(), json!(report));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for MedoidTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "medoid",
            display_name: "Medoid",
            summary: "Calculates medoid points from vector geometries.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector layer.", required: true },
                ToolParamSpec { name: "output", description: "Output medoid point vector path.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.geojson"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("medoid.geojson"));

        ToolManifest {
            id: "medoid".to_string(),
            display_name: "Medoid".to_string(),
            summary: "Calculates medoid points from vector geometries.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output medoid point vector path.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "medoid_basic".to_string(),
                description: "Creates medoid point output from input features.".to_string(),
                args: example_args,
            }],
            tags: vec!["vector".to_string(), "gis".to_string(), "centroid".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;

        let mut has_point_geom = false;
        let mut has_non_point_geom = false;
        for feature in &input.features {
            let Some(geometry) = feature.geometry.as_ref() else {
                continue;
            };
            match geometry {
                wbvector::Geometry::Point(_) | wbvector::Geometry::MultiPoint(_) => has_point_geom = true,
                _ => has_non_point_geom = true,
            }
        }
        let is_point_layer = has_point_geom && !has_non_point_geom;

        let mut output = wbvector::Layer::new(format!("{}_medoid", input.name));
        output.geom_type = Some(wbvector::GeometryType::Point);
        output.crs = input.crs.clone();

        if is_point_layer {
            output.schema = input.schema.clone();
            let mut all_coords = Vec::<wbvector::Coord>::new();
            let mut owner_feature_index = Vec::<usize>::new();
            for (feature_idx, feature) in input.features.iter().enumerate() {
                if let Some(geometry) = feature.geometry.as_ref() {
                    let start = all_coords.len();
                    collect_all_coords_from_geometry(geometry, &mut all_coords);
                    for _ in start..all_coords.len() {
                        owner_feature_index.push(feature_idx);
                    }
                }
            }
            if let Some(best_idx) = medoid_coordinate_index(&all_coords) {
                let owner = owner_feature_index[best_idx];
                let source_feature = &input.features[owner];
                output.push(wbvector::Feature {
                    fid: if source_feature.fid == 0 { 1 } else { source_feature.fid },
                    geometry: Some(wbvector::Geometry::Point(all_coords[best_idx].clone())),
                    attributes: source_feature.attributes.clone(),
                });
            }
            ctx.progress.progress(1.0);
        } else {
            output.schema = input.schema.clone();
            let total = input.features.len().max(1);
            let mut next_fid = 1u64;
            for (feature_idx, feature) in input.features.iter().enumerate() {
                if let Some(geometry) = feature.geometry.as_ref() {
                    let mut coords = Vec::<wbvector::Coord>::new();
                    collect_all_coords_from_geometry(geometry, &mut coords);
                    if let Some(best_idx) = medoid_coordinate_index(&coords) {
                        output.push(wbvector::Feature {
                            fid: if feature.fid == 0 { next_fid } else { feature.fid },
                            geometry: Some(wbvector::Geometry::Point(coords[best_idx].clone())),
                            attributes: feature.attributes.clone(),
                        });
                        next_fid += 1;
                    }
                }
                ctx.progress.progress((feature_idx + 1) as f64 / total as f64);
            }
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for FindLowestOrHighestPointsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "find_lowest_or_highest_points",
            display_name: "Find Lowest Or Highest Points",
            summary: "Locates lowest and/or highest raster cells and outputs their locations as points.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster.", required: true },
                ToolParamSpec { name: "out_type", description: "Output type: lowest, highest, or both.", required: false },
                ToolParamSpec { name: "output", description: "Output point vector path.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("out_type".to_string(), json!("lowest"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("extrema_points.geojson"));

        ToolManifest {
            id: "find_lowest_or_highest_points".to_string(),
            display_name: "Find Lowest Or Highest Points".to_string(),
            summary: "Locates lowest and/or highest raster cells and outputs their locations as points.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input raster.".to_string(), required: true },
                ToolParamDescriptor { name: "out_type".to_string(), description: "Output type: lowest, highest, or both.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output point vector path.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "find_lowest_or_highest_points_basic".to_string(),
                description: "Writes lowest and highest raster cells as vector point features.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "vector".to_string(), "gis".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "input")?;
        let out_type = args
            .get("out_type")
            .and_then(|v| v.as_str())
            .unwrap_or("lowest")
            .to_ascii_lowercase();
        let valid = matches!(out_type.as_str(), "lowest" | "highest" | "both")
            || out_type.contains("lo")
            || out_type.contains("hi")
            || out_type.contains('b');
        if !valid {
            return Err(ToolError::Validation(
                "out_type must be one of: lowest, highest, both".to_string(),
            ));
        }
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_required_raster_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let mut out_type = args
            .get("out_type")
            .and_then(|v| v.as_str())
            .unwrap_or("lowest")
            .to_ascii_lowercase();
        if out_type.contains("lo") {
            out_type = "lowest".to_string();
        }
        if out_type.contains("hi") {
            out_type = "highest".to_string();
        }
        if out_type.contains('b') {
            out_type = "both".to_string();
        }

        let mut low = (f64::INFINITY, 0usize, 0usize);
        let mut high = (f64::NEG_INFINITY, 0usize, 0usize);
        let mut has_valid = false;
        for row in 0..input.rows {
            for col in 0..input.cols {
                let value = input.get(0, row as isize, col as isize);
                if input.is_nodata(value) {
                    continue;
                }
                has_valid = true;
                if value < low.0 {
                    low = (value, row, col);
                }
                if value > high.0 {
                    high = (value, row, col);
                }
            }
        }
        if !has_valid {
            return Err(ToolError::Validation(
                "input raster contains no valid (non-NoData) cells".to_string(),
            ));
        }

        let mut output = wbvector::Layer::new("extrema_points").with_geom_type(wbvector::GeometryType::Point);
        output.crs = if input.crs.epsg.is_some() || input.crs.wkt.is_some() {
            Some(wbvector::Crs {
                epsg: input.crs.epsg,
                wkt: input.crs.wkt.clone(),
            })
        } else {
            None
        };
        output.schema.add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));
        output.schema.add_field(wbvector::FieldDef::new("Value", wbvector::FieldType::Float));
        output.schema.add_field(wbvector::FieldDef::new("X", wbvector::FieldType::Float));
        output.schema.add_field(wbvector::FieldDef::new("Y", wbvector::FieldType::Float));

        let mut fid = 1i64;
        let mut add_feature = |value: f64, row: usize, col: usize| {
            let x = input.col_center_x(col as isize);
            let y = input.row_center_y(row as isize);
            output.push(wbvector::Feature {
                fid: fid as u64,
                geometry: Some(wbvector::Geometry::Point(wbvector::Coord::xy(x, y))),
                attributes: vec![
                    wbvector::FieldValue::Integer(fid),
                    wbvector::FieldValue::Float(value),
                    wbvector::FieldValue::Float(x),
                    wbvector::FieldValue::Float(y),
                ],
            });
            fid += 1;
        };

        if out_type == "lowest" || out_type == "both" {
            add_feature(low.0, low.1, low.2);
        }
        if out_type == "highest" || out_type == "both" {
            add_feature(high.0, high.1, high.2);
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for EliminateCoincidentPointsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "eliminate_coincident_points",
            display_name: "Eliminate Coincident Points",
            summary: "Removes coincident or near-coincident points within a tolerance distance.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input point vector layer.",
                    required: true,
                },
                ToolParamSpec {
                    name: "tolerance_dist",
                    description: "Distance threshold for considering points coincident.",
                    required: true,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output point vector path.",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("points.geojson"));
        defaults.insert("tolerance_dist".to_string(), json!(0.0));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("points_unique.geojson"));

        ToolManifest {
            id: "eliminate_coincident_points".to_string(),
            display_name: "Eliminate Coincident Points".to_string(),
            summary: "Removes coincident or near-coincident points within a tolerance distance."
                .to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input point vector layer.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "tolerance_dist".to_string(),
                    description: "Distance threshold for considering points coincident.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output point vector path.".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "eliminate_coincident_points_basic".to_string(),
                description: "Removes duplicate points based on a distance tolerance.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "points".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let input = load_vector_arg(args, "input")?;
        if let Some(geom_type) = input.geom_type {
            if geom_type != wbvector::GeometryType::Point {
                return Err(ToolError::Validation(
                    "input vector data must be of POINT base shape type".to_string(),
                ));
            }
        }
        let tolerance = args
            .get("tolerance_dist")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| {
                ToolError::Validation("parameter 'tolerance_dist' is required".to_string())
            })?;
        if !tolerance.is_finite() || tolerance < 0.0 {
            return Err(ToolError::Validation(
                "tolerance_dist must be a finite value >= 0".to_string(),
            ));
        }
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let tolerance = args
            .get("tolerance_dist")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| {
                ToolError::Validation("parameter 'tolerance_dist' is required".to_string())
            })?;
        let output_path = parse_vector_path_arg(args, "output")?;

        let mut output = wbvector::Layer::new(format!("{}_unique", input.name));
        output.geom_type = Some(wbvector::GeometryType::Point);
        output.crs = input.crs.clone();
        output.schema = input.schema.clone();

        let radius_sq = tolerance * tolerance;
        let mut tree = KdTree::new(2);
        let total = input.features.len().max(1);
        let mut next_fid = 1u64;

        for (feature_idx, feature) in input.features.iter().enumerate() {
            let Some(geometry) = feature.geometry.as_ref() else {
                continue;
            };
            let point = match geometry {
                wbvector::Geometry::Point(coord) => coord.clone(),
                _ => {
                    return Err(ToolError::Validation(
                        "input vector data must contain only point geometries".to_string(),
                    ))
                }
            };

            let mut keep = true;
            if radius_sq == 0.0 {
                let ret = tree
                    .within(&[point.x, point.y], 0.0, &squared_euclidean)
                    .map_err(|e| {
                        ToolError::Execution(format!(
                            "failed querying coincident-point search index: {e}"
                        ))
                    })?;
                if !ret.is_empty() {
                    keep = false;
                }
            } else {
                let ret = tree
                    .within(&[point.x, point.y], radius_sq, &squared_euclidean)
                    .map_err(|e| {
                        ToolError::Execution(format!(
                            "failed querying coincident-point search index: {e}"
                        ))
                    })?;
                if !ret.is_empty() {
                    keep = false;
                }
            }

            if keep {
                tree.add([point.x, point.y], output.features.len())
                    .map_err(|e| {
                        ToolError::Execution(format!(
                            "failed updating coincident-point search index: {e}"
                        ))
                    })?;
                output.push(wbvector::Feature {
                    fid: if feature.fid == 0 { next_fid } else { feature.fid },
                    geometry: Some(wbvector::Geometry::Point(point)),
                    attributes: feature.attributes.clone(),
                });
                next_fid += 1;
            }

            ctx.progress
                .progress((feature_idx + 1) as f64 / total as f64);
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ExtendDirection {
    Both,
    Start,
    End,
}

fn parse_extend_direction_arg(args: &ToolArgs, key: &str) -> Result<ExtendDirection, ToolError> {
    let value = args
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "both".to_string());

    if value.is_empty() {
        return Ok(ExtendDirection::Both);
    }

    if value.contains("bo") || value == "both" {
        return Ok(ExtendDirection::Both);
    }
    if value.contains("st") || value == "start" || value == "line_start" {
        return Ok(ExtendDirection::Start);
    }
    if value.contains("end") || value == "en" || value == "line_end" {
        return Ok(ExtendDirection::End);
    }

    Err(ToolError::Validation(
        "extend_direction must be one of: both, start, end".to_string(),
    ))
}

fn extend_line_endpoint(anchor: &wbvector::Coord, neighbour: &wbvector::Coord, distance: f64) -> wbvector::Coord {
    let dx = anchor.x - neighbour.x;
    let dy = anchor.y - neighbour.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= f64::EPSILON {
        return anchor.clone();
    }

    wbvector::Coord::xy(
        anchor.x + distance * (dx / len),
        anchor.y + distance * (dy / len),
    )
}

fn extend_linestring_coords(coords: &mut [wbvector::Coord], distance: f64, direction: ExtendDirection) {
    if coords.len() < 2 {
        return;
    }

    if matches!(direction, ExtendDirection::Both | ExtendDirection::Start) {
        let new_start = extend_line_endpoint(&coords[0], &coords[1], distance);
        coords[0] = new_start;
    }
    if matches!(direction, ExtendDirection::Both | ExtendDirection::End) {
        let n = coords.len();
        let new_end = extend_line_endpoint(&coords[n - 1], &coords[n - 2], distance);
        coords[n - 1] = new_end;
    }
}

impl Tool for ExtendVectorLinesTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "extend_vector_lines",
            display_name: "Extend Vector Lines",
            summary: "Extends polyline endpoints by a specified distance at the start, end, or both.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input polyline vector layer.",
                    required: true,
                },
                ToolParamSpec {
                    name: "distance",
                    description: "Distance to extend polyline endpoints.",
                    required: true,
                },
                ToolParamSpec {
                    name: "extend_direction",
                    description: "Direction to extend endpoints: both, start, or end.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output vector path.",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input_lines.shp"));
        defaults.insert("distance".to_string(), json!(1.0));
        defaults.insert("extend_direction".to_string(), json!("both"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("extended_lines.shp"));

        ToolManifest {
            id: "extend_vector_lines".to_string(),
            display_name: "Extend Vector Lines".to_string(),
            summary: "Extends polyline endpoints by a specified distance at the start, end, or both."
                .to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input polyline vector layer.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "distance".to_string(),
                    description: "Distance to extend polyline endpoints.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "extend_direction".to_string(),
                    description: "Direction to extend endpoints: both, start, or end.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output vector path.".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "extend_vector_lines_basic".to_string(),
                description: "Extends line endpoints by a fixed distance.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "linework".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let input = load_vector_arg(args, "input")?;
        if let Some(geom_type) = input.geom_type {
            if geom_type != wbvector::GeometryType::LineString {
                return Err(ToolError::Validation(
                    "input vector data must be of POLYLINE base shape type".to_string(),
                ));
            }
        }

        let distance = args
            .get("distance")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'distance' is required".to_string()))?;
        if !distance.is_finite() || distance < 0.0 {
            return Err(ToolError::Validation(
                "distance must be a finite value >= 0".to_string(),
            ));
        }

        let _ = parse_extend_direction_arg(args, "extend_direction")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let distance = args
            .get("distance")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'distance' is required".to_string()))?;
        let direction = parse_extend_direction_arg(args, "extend_direction")?;
        let output_path = parse_vector_path_arg(args, "output")?;

        let mut output = wbvector::Layer::new(format!("{}_extended", input.name));
        output.schema = input.schema.clone();
        output.crs = input.crs.clone();
        output.geom_type = Some(wbvector::GeometryType::LineString);

        let total = input.features.len().max(1);
        let completed = AtomicUsize::new(0);
        let results: Vec<Result<Option<wbvector::Feature>, ToolError>> = input
            .features
            .par_iter()
            .enumerate()
            .map(|(index, feature)| {
                let Some(geometry) = feature.geometry.as_ref() else {
                    let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    ctx.progress.progress(done as f64 / total as f64);
                    return Ok(None);
                };

                let out_geom = match geometry {
                    wbvector::Geometry::LineString(coords) => {
                        let mut out = coords.clone();
                        extend_linestring_coords(&mut out, distance, direction);
                        wbvector::Geometry::LineString(out)
                    }
                    wbvector::Geometry::MultiLineString(lines) => {
                        let mut out_lines = lines.clone();
                        for line in &mut out_lines {
                            extend_linestring_coords(line, distance, direction);
                        }
                        wbvector::Geometry::MultiLineString(out_lines)
                    }
                    _ => {
                        return Err(ToolError::Validation(
                            "input vector data must contain only polyline geometries".to_string(),
                        ))
                    }
                };

                let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                ctx.progress.progress(done as f64 / total as f64);
                Ok(Some(wbvector::Feature {
                    fid: if feature.fid == 0 { (index + 1) as u64 } else { feature.fid },
                    geometry: Some(out_geom),
                    attributes: feature.attributes.clone(),
                }))
            })
            .collect();
        for result in results {
            if let Some(feat) = result? {
                output.push(feat);
            }
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

fn parse_smoothing_filter_size(args: &ToolArgs, key: &str) -> Result<usize, ToolError> {
    let mut filter = args
        .get(key)
        .and_then(|v| v.as_u64())
        .unwrap_or(3) as usize;
    if filter < 3 {
        filter = 3;
    }
    if filter % 2 == 0 {
        filter += 1;
    }
    Ok(filter)
}

fn smooth_open_linestring(coords: &[wbvector::Coord], filter: usize) -> Vec<wbvector::Coord> {
    let n = coords.len();
    if n < 4 {
        return coords.to_vec();
    }
    let half = (filter / 2) as isize;
    let mut out = coords.to_vec();

    // Preserve line endpoints to avoid line shortening from edge effects.
    for i in 1..(n - 1) {
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut count = 0.0;
        for j in (i as isize - half)..=(i as isize + half) {
            if j >= 0 && j < n as isize {
                let c = &coords[j as usize];
                sum_x += c.x;
                sum_y += c.y;
                count += 1.0;
            }
        }
        if count > 0.0 {
            out[i] = wbvector::Coord::xy(sum_x / count, sum_y / count);
        }
    }

    out
}

fn smooth_closed_ring(ring: &wbvector::Ring, filter: usize) -> wbvector::Ring {
    let coords = ring.coords();
    let n = coords.len();
    if n < 4 {
        return ring.clone();
    }

    let half = (filter / 2) as isize;
    let mut out = coords.to_vec();
    for i in 0..n {
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut count = 0.0;
        for j in (i as isize - half)..=(i as isize + half) {
            let k = j.rem_euclid(n as isize) as usize;
            sum_x += coords[k].x;
            sum_y += coords[k].y;
            count += 1.0;
        }
        if count > 0.0 {
            out[i] = wbvector::Coord::xy(sum_x / count, sum_y / count);
        }
    }

    wbvector::Ring::new(out)
}

impl Tool for SmoothVectorsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "smooth_vectors",
            display_name: "Smooth Vectors",
            summary: "Smooths polyline or polygon vectors using a moving-average filter.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input line or polygon vector layer.",
                    required: true,
                },
                ToolParamSpec {
                    name: "filter_size",
                    description:
                        "Odd moving-average window size (>= 3). Even values are incremented to next odd value.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output vector path.",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input_vectors.shp"));
        defaults.insert("filter_size".to_string(), json!(3));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("smooth_vectors.shp"));

        ToolManifest {
            id: "smooth_vectors".to_string(),
            display_name: "Smooth Vectors".to_string(),
            summary: "Smooths polyline or polygon vectors using a moving-average filter."
                .to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input line or polygon vector layer.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "filter_size".to_string(),
                    description:
                        "Odd moving-average window size (>= 3). Even values are incremented to next odd value."
                            .to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output vector path.".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "smooth_vectors_basic".to_string(),
                description: "Smooths input vector geometries using a local moving average."
                    .to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "linework".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let input = load_vector_arg(args, "input")?;
        if let Some(geom_type) = input.geom_type {
            if geom_type != wbvector::GeometryType::LineString
                && geom_type != wbvector::GeometryType::Polygon
            {
                return Err(ToolError::Validation(
                    "input vector data must be of POLYLINE or POLYGON base shape type"
                        .to_string(),
                ));
            }
        }
        let _ = parse_smoothing_filter_size(args, "filter_size")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let filter = parse_smoothing_filter_size(args, "filter_size")?;
        let output_path = parse_vector_path_arg(args, "output")?;

        let mut output = wbvector::Layer::new(format!("{}_smoothed", input.name));
        output.schema = input.schema.clone();
        output.crs = input.crs.clone();
        output.geom_type = input.geom_type;

        let total = input.features.len().max(1);
        let completed = AtomicUsize::new(0);
        let results: Vec<Result<Option<wbvector::Feature>, ToolError>> = input
            .features
            .par_iter()
            .enumerate()
            .map(|(index, feature)| {
                let Some(geometry) = feature.geometry.as_ref() else {
                    let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    ctx.progress.progress(done as f64 / total as f64);
                    return Ok(None);
                };

                let out_geom = match geometry {
                    wbvector::Geometry::LineString(coords) => {
                        Ok(wbvector::Geometry::LineString(smooth_open_linestring(coords, filter)))
                    }
                    wbvector::Geometry::MultiLineString(lines) => {
                        Ok(wbvector::Geometry::MultiLineString(
                            lines
                                .iter()
                                .map(|line| smooth_open_linestring(line, filter))
                                .collect(),
                        ))
                    }
                    wbvector::Geometry::Polygon { exterior, interiors } => {
                        Ok(wbvector::Geometry::Polygon {
                            exterior: smooth_closed_ring(exterior, filter),
                            interiors: interiors
                                .iter()
                                .map(|ring| smooth_closed_ring(ring, filter))
                                .collect(),
                        })
                    }
                    wbvector::Geometry::MultiPolygon(parts) => {
                        Ok(wbvector::Geometry::MultiPolygon(
                            parts
                                .iter()
                                .map(|(exterior, interiors)| {
                                    (
                                        smooth_closed_ring(exterior, filter),
                                        interiors
                                            .iter()
                                            .map(|ring| smooth_closed_ring(ring, filter))
                                            .collect(),
                                    )
                                })
                                .collect(),
                        ))
                    }
                    _ => Err(ToolError::Validation(
                        "input vector data must contain only polyline or polygon geometries"
                            .to_string(),
                    )),
                }?;

                let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                ctx.progress.progress(done as f64 / total as f64);
                Ok(Some(wbvector::Feature {
                    fid: if feature.fid == 0 { (index + 1) as u64 } else { feature.fid },
                    geometry: Some(out_geom),
                    attributes: feature.attributes.clone(),
                }))
            })
            .collect();
        for result in results {
            if let Some(feat) = result? {
                output.push(feat);
            }
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

/// Splits each coordinate sequence into segments of at most `segment_length` map units.
/// Returns a Vec of coordinate sequences (each has >= 2 points).
fn split_coords_by_length(
    coords: &[wbvector::Coord],
    segment_length: f64,
) -> Vec<Vec<wbvector::Coord>> {
    if coords.len() < 2 {
        return vec![];
    }

    let mut segments: Vec<Vec<wbvector::Coord>> = Vec::new();
    let mut current: Vec<wbvector::Coord> = vec![wbvector::Coord::xy(coords[0].x, coords[0].y)];
    let mut dist_accum = 0.0_f64;
    let mut i = 1usize;

    while i < coords.len() {
        let x1 = current.last().unwrap().x;
        let y1 = current.last().unwrap().y;
        let x2 = coords[i].x;
        let y2 = coords[i].y;
        let seg_dist = ((x2 - x1) * (x2 - x1) + (y2 - y1) * (y2 - y1)).sqrt();

        if seg_dist <= 0.0 {
            // Duplicate/coincident point — skip without advancing the accumulator.
            i += 1;
            continue;
        }

        if dist_accum + seg_dist <= segment_length {
            current.push(wbvector::Coord::xy(x2, y2));
            dist_accum += seg_dist;
            i += 1;
        } else {
            // Interpolate the split point at exactly segment_length from the last emitted point.
            let ratio = (segment_length - dist_accum) / seg_dist;
            let sx = x1 + ratio * (x2 - x1);
            let sy = y1 + ratio * (y2 - y1);
            current.push(wbvector::Coord::xy(sx, sy));
            segments.push(current);
            // Start the next segment from the split point.
            current = vec![wbvector::Coord::xy(sx, sy)];
            dist_accum = 0.0;
            // Do NOT advance i — re-process coords[i] from the new split origin.
        }
    }

    if current.len() >= 2 {
        segments.push(current);
    }
    segments
}

impl Tool for SplitVectorLinesTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "split_vector_lines",
            display_name: "Split Vector Lines",
            summary: "Splits each polyline feature into segments of a maximum specified length.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input polyline vector layer.",
                    required: true,
                },
                ToolParamSpec {
                    name: "segment_length",
                    description: "Maximum segment length in map units.",
                    required: true,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output polyline vector path.",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input_lines.shp"));
        defaults.insert("segment_length".to_string(), json!(100.0));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("split_lines.shp"));

        ToolManifest {
            id: "split_vector_lines".to_string(),
            display_name: "Split Vector Lines".to_string(),
            summary: "Splits each polyline feature into segments of a maximum specified length."
                .to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input polyline vector layer.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "segment_length".to_string(),
                    description: "Maximum segment length in map units.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output polyline vector path.".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "split_vector_lines_basic".to_string(),
                description: "Splits polylines into segments no longer than a specified length.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "linework".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let input = load_vector_arg(args, "input")?;
        if let Some(geom_type) = input.geom_type {
            if geom_type != wbvector::GeometryType::LineString {
                return Err(ToolError::Validation(
                    "input vector data must be of POLYLINE base shape type".to_string(),
                ));
            }
        }
        let segment_length = args
            .get("segment_length")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'segment_length' is required".to_string()))?;
        if !segment_length.is_finite() || segment_length <= 0.0 {
            return Err(ToolError::Validation(
                "segment_length must be a finite value > 0".to_string(),
            ));
        }
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let segment_length = args
            .get("segment_length")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'segment_length' is required".to_string()))?;
        let output_path = parse_vector_path_arg(args, "output")?;

        // Identify if the input schema has a field named "FID" so we can skip it in the output.
        let fid_field_idx = input.schema.field_index("FID");

        let mut output = wbvector::Layer::new(format!("{}_split", input.name));
        output.geom_type = Some(wbvector::GeometryType::LineString);
        output.crs = input.crs.clone();
        output.schema = wbvector::Schema::new();
        output.schema.add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));
        output.schema.add_field(wbvector::FieldDef::new("PARENT_ID", wbvector::FieldType::Integer));
        for (i, field) in input.schema.fields().iter().enumerate() {
            if Some(i) != fid_field_idx {
                output.schema.add_field(field.clone());
            }
        }

        let total = input.features.len().max(1);
        let completed = AtomicUsize::new(0);
        let batch_results: Vec<Result<Vec<wbvector::Feature>, ToolError>> = input
            .features
            .par_iter()
            .enumerate()
            .map(|(feature_idx, feature)| {
                let Some(geometry) = feature.geometry.as_ref() else {
                    let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    ctx.progress.progress(done as f64 / total as f64);
                    return Ok(Vec::new());
                };

                // Collect all coordinate sequences (parts) from the geometry.
                let parts: Vec<&[wbvector::Coord]> = match geometry {
                    wbvector::Geometry::LineString(coords) => vec![coords.as_slice()],
                    wbvector::Geometry::MultiLineString(lines) => {
                        lines.iter().map(|l| l.as_slice()).collect()
                    }
                    _ => {
                        return Err(ToolError::Validation(
                            "input vector data must contain only polyline geometries".to_string(),
                        ))
                    }
                };

                // Build the parent attributes, skipping the FID field if present.
                let parent_attrs: Vec<wbvector::FieldValue> = feature
                    .attributes
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| Some(*i) != fid_field_idx)
                    .map(|(_, v)| v.clone())
                    .collect();

                let parent_id = feature_idx as i64 + 1;
                let mut feats = Vec::new();
                for part_coords in parts {
                    let segments = split_coords_by_length(part_coords, segment_length);
                    for seg_coords in segments {
                        let mut attrs = vec![
                            wbvector::FieldValue::Integer(0),
                            wbvector::FieldValue::Integer(parent_id),
                        ];
                        attrs.extend_from_slice(&parent_attrs);
                        feats.push(wbvector::Feature {
                            fid: 0,
                            geometry: Some(wbvector::Geometry::LineString(seg_coords)),
                            attributes: attrs,
                        });
                    }
                }
                let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                ctx.progress.progress(done as f64 / total as f64);
                Ok(feats)
            })
            .collect();
        let mut next_fid = 1u64;
        for result in batch_results {
            for mut feat in result? {
                feat.fid = next_fid;
                feat.attributes[0] = wbvector::FieldValue::Integer(next_fid as i64);
                output.push(feat);
                next_fid += 1;
            }
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

#[derive(Clone, Copy)]
struct EndpointRef {
    feature_idx: usize,
    line_idx: Option<usize>,
    is_start: bool,
}

fn set_endpoint_coord(layer: &mut wbvector::Layer, endpoint: EndpointRef, coord: wbvector::Coord) {
    let Some(feature) = layer.features.get_mut(endpoint.feature_idx) else {
        return;
    };
    let Some(geometry) = feature.geometry.as_mut() else {
        return;
    };

    match geometry {
        wbvector::Geometry::LineString(coords) => {
            if coords.is_empty() {
                return;
            }
            let idx = if endpoint.is_start {
                0
            } else {
                coords.len().saturating_sub(1)
            };
            coords[idx] = coord;
        }
        wbvector::Geometry::MultiLineString(lines) => {
            let Some(line_idx) = endpoint.line_idx else {
                return;
            };
            let Some(coords) = lines.get_mut(line_idx) else {
                return;
            };
            if coords.is_empty() {
                return;
            }
            let idx = if endpoint.is_start {
                0
            } else {
                coords.len().saturating_sub(1)
            };
            coords[idx] = coord;
        }
        _ => {}
    }
}

fn uf_find(parent: &mut [usize], mut i: usize) -> usize {
    let mut root = i;
    while parent[root] != root {
        root = parent[root];
    }
    while parent[i] != i {
        let next = parent[i];
        parent[i] = root;
        i = next;
    }
    root
}

fn uf_union(parent: &mut [usize], a: usize, b: usize) {
    let ra = uf_find(parent, a);
    let rb = uf_find(parent, b);
    if ra != rb {
        parent[rb] = ra;
    }
}

impl Tool for SnapEndnodesTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "snap_endnodes",
            display_name: "Snap Endnodes",
            summary: "Snaps nearby polyline endpoints to a shared location within a tolerance.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input polyline vector layer.",
                    required: true,
                },
                ToolParamSpec {
                    name: "snap_tolerance",
                    description: "Endpoint snapping tolerance in map units.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output snapped polyline vector path.",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input_lines.shp"));
        defaults.insert("snap_tolerance".to_string(), json!(f64::EPSILON));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("snapped_endnodes.shp"));

        ToolManifest {
            id: "snap_endnodes".to_string(),
            display_name: "Snap Endnodes".to_string(),
            summary: "Snaps nearby polyline endpoints to a shared location within a tolerance."
                .to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input polyline vector layer.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "snap_tolerance".to_string(),
                    description: "Endpoint snapping tolerance in map units.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output snapped polyline vector path.".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "snap_endnodes_basic".to_string(),
                description: "Snaps adjacent line endpoints by a tolerance threshold.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "linework".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let input = load_vector_arg(args, "input")?;
        if let Some(geom_type) = input.geom_type {
            if geom_type != wbvector::GeometryType::LineString {
                return Err(ToolError::Validation(
                    "input vector data must be of POLYLINE base shape type".to_string(),
                ));
            }
        }

        let tolerance = args
            .get("snap_tolerance")
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::EPSILON);
        if !tolerance.is_finite() || tolerance < 0.0 {
            return Err(ToolError::Validation(
                "snap_tolerance must be a finite value >= 0".to_string(),
            ));
        }

        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let snap_tolerance = args
            .get("snap_tolerance")
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::EPSILON);
        let output_path = parse_vector_path_arg(args, "output")?;
        let tol_sq = snap_tolerance * snap_tolerance;

        let mut output = input.clone();
        output.name = format!("{}_snapped", input.name);
        output.geom_type = Some(wbvector::GeometryType::LineString);

        let mut endpoint_refs = Vec::<EndpointRef>::new();
        let mut endpoint_coords = Vec::<wbvector::Coord>::new();

        for (feature_idx, feature) in output.features.iter().enumerate() {
            let Some(geometry) = feature.geometry.as_ref() else {
                continue;
            };

            match geometry {
                wbvector::Geometry::LineString(coords) => {
                    if let Some(first) = coords.first() {
                        endpoint_refs.push(EndpointRef {
                            feature_idx,
                            line_idx: None,
                            is_start: true,
                        });
                        endpoint_coords.push(first.clone());
                    }
                    if let Some(last) = coords.last() {
                        endpoint_refs.push(EndpointRef {
                            feature_idx,
                            line_idx: None,
                            is_start: false,
                        });
                        endpoint_coords.push(last.clone());
                    }
                }
                wbvector::Geometry::MultiLineString(lines) => {
                    for (line_idx, coords) in lines.iter().enumerate() {
                        if let Some(first) = coords.first() {
                            endpoint_refs.push(EndpointRef {
                                feature_idx,
                                line_idx: Some(line_idx),
                                is_start: true,
                            });
                            endpoint_coords.push(first.clone());
                        }
                        if let Some(last) = coords.last() {
                            endpoint_refs.push(EndpointRef {
                                feature_idx,
                                line_idx: Some(line_idx),
                                is_start: false,
                            });
                            endpoint_coords.push(last.clone());
                        }
                    }
                }
                _ => {
                    return Err(ToolError::Validation(
                        "input vector data must contain only polyline geometries".to_string(),
                    ));
                }
            }
        }

        if endpoint_coords.is_empty() {
            let output_locator = write_vector_output(&output, output_path.trim())?;
            return Ok(build_vector_result(output_locator));
        }

        let mut tree = KdTree::new(2);
        for (i, c) in endpoint_coords.iter().enumerate() {
            tree.add([c.x, c.y], i).map_err(|e| {
                ToolError::Execution(format!("failed to build endpoint search index: {e}"))
            })?;
        }

        let mut parent: Vec<usize> = (0..endpoint_coords.len()).collect();
        let total = endpoint_coords.len().max(1);
        for (i, c) in endpoint_coords.iter().enumerate() {
            let neighbors = tree
                .within(&[c.x, c.y], tol_sq, &squared_euclidean)
                .map_err(|e| ToolError::Execution(format!("failed endpoint-neighbor query: {e}")))?;
            for (_, j_ref) in neighbors {
                uf_union(&mut parent, i, *j_ref);
            }
            ctx.progress.progress((i + 1) as f64 / total as f64);
        }

        let mut sums = HashMap::<usize, (f64, f64, usize)>::new();
        for (i, c) in endpoint_coords.iter().enumerate() {
            let root = uf_find(&mut parent, i);
            let entry = sums.entry(root).or_insert((0.0, 0.0, 0));
            entry.0 += c.x;
            entry.1 += c.y;
            entry.2 += 1;
        }

        let mut snapped = HashMap::<usize, wbvector::Coord>::new();
        for (root, (sum_x, sum_y, n)) in sums {
            if n > 0 {
                snapped.insert(root, wbvector::Coord::xy(sum_x / n as f64, sum_y / n as f64));
            }
        }

        for (i, endpoint) in endpoint_refs.into_iter().enumerate() {
            let root = uf_find(&mut parent, i);
            if let Some(coord) = snapped.get(&root) {
                set_endpoint_coord(&mut output, endpoint, coord.clone());
            }
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for VoronoiDiagramTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "voronoi_diagram",
            display_name: "Voronoi Diagram",
            summary: "Creates Voronoi (Thiessen) polygons from input point locations.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input_points",
                    description: "Input point or multipoint vector layer.",
                    required: true,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output Voronoi polygon vector path.",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input_points".to_string(), json!("points.shp"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("voronoi_diagram.shp"));

        ToolManifest {
            id: "voronoi_diagram".to_string(),
            display_name: "Voronoi Diagram".to_string(),
            summary: "Creates Voronoi (Thiessen) polygons from input point locations.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input_points".to_string(),
                    description: "Input point or multipoint vector layer.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output Voronoi polygon vector path.".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "voronoi_diagram_basic".to_string(),
                description: "Builds Voronoi cells from input points.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "voronoi".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let input = load_vector_arg(args, "input_points")?;
        if let Some(geom_type) = input.geom_type {
            if geom_type != wbvector::GeometryType::Point {
                return Err(ToolError::Validation(
                    "input_points vector data must be of POINT base shape type".to_string(),
                ));
            }
        }
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input_points")?;
        let output_path = parse_vector_path_arg(args, "output")?;

        let mut points = Vec::<TopoCoord>::new();
        let mut source_attrs = Vec::<Vec<wbvector::FieldValue>>::new();

        let total = input.features.len().max(1);
        for (idx, feature) in input.features.iter().enumerate() {
            let Some(geometry) = feature.geometry.as_ref() else {
                ctx.progress.progress((idx + 1) as f64 / total as f64);
                continue;
            };

            match geometry {
                wbvector::Geometry::Point(c) => {
                    points.push(TopoCoord::xy(c.x, c.y));
                    source_attrs.push(feature.attributes.clone());
                }
                wbvector::Geometry::MultiPoint(coords) => {
                    for c in coords {
                        points.push(TopoCoord::xy(c.x, c.y));
                        source_attrs.push(feature.attributes.clone());
                    }
                }
                _ => {
                    return Err(ToolError::Validation(
                        "input_points vector data must contain only point geometries".to_string(),
                    ));
                }
            }

            ctx.progress.progress((idx + 1) as f64 / total as f64);
        }

        if points.is_empty() {
            return Err(ToolError::Validation(
                "input_points layer must contain at least one point geometry".to_string(),
            ));
        }

        let diagram = voronoi_diagram(&points, 1.0e-9);
        let mut site_tree = KdTree::new(2);
        for (i, p) in points.iter().enumerate() {
            site_tree.add([p.x, p.y], i).map_err(|e| {
                ToolError::Execution(format!("failed to build point search index: {e}"))
            })?;
        }

        let mut output = wbvector::Layer::new(format!("{}_voronoi", input.name));
        output.geom_type = Some(wbvector::GeometryType::Polygon);
        output.crs = input.crs.clone();
        output.schema = input.schema.clone();

        let mut next_fid = 1u64;
        let total_cells = diagram.cells.len().max(1);
        for (idx, cell) in diagram.cells.into_iter().enumerate() {
            if cell.exterior.coords.len() < 4 {
                continue;
            }

            let site = diagram.sites[idx];
            let nearest = site_tree
                .nearest(&[site.x, site.y], 1, &squared_euclidean)
                .map_err(|e| ToolError::Execution(format!("failed site-nearest query: {e}")))?;
            let source_idx = nearest
                .first()
                .map(|(_, idx_ref)| **idx_ref)
                .ok_or_else(|| {
                    ToolError::Execution("failed mapping site to source attributes".to_string())
                })?;

            let attrs = source_attrs
                .get(source_idx)
                .cloned()
                .unwrap_or_else(Vec::new);

            push_topo_polygon_feature(&mut output, next_fid, cell, attrs);
            next_fid += 1;

            ctx.progress.progress((idx + 1) as f64 / total_cells as f64);
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for ClipRasterToPolygonTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "clip_raster_to_polygon",
            display_name: "Clip Raster To Polygon",
            summary: "Clips a raster to polygon extents; outside polygon cells are set to NoData.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster.", required: true },
                ToolParamSpec { name: "polygons", description: "Input polygon vector layer.", required: true },
                ToolParamSpec { name: "maintain_dimensions", description: "Preserve original raster dimensions instead of cropping to polygon extent.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("polygons".to_string(), json!("polygons.geojson"));
        defaults.insert("maintain_dimensions".to_string(), json!(false));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("clip_raster_to_polygon.tif"));
        ToolManifest {
            id: "clip_raster_to_polygon".to_string(),
            display_name: "Clip Raster To Polygon".to_string(),
            summary: "Clips a raster to polygon extents; outside polygon cells are set to NoData.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input raster.".to_string(), required: true },
                ToolParamDescriptor { name: "polygons".to_string(), description: "Input polygon vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "maintain_dimensions".to_string(), description: "Preserve original raster dimensions instead of cropping to polygon extent.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "clip_raster_to_polygon_basic".to_string(),
                description: "Clips raster cells to polygon extents.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "clip".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "input")?;
        let _ = parse_required_vector_path_arg(args, "polygons")?;
        let _ = args
            .get("maintain_dimensions")
            .map(|v| v.as_bool().ok_or_else(|| ToolError::Validation("maintain_dimensions must be boolean".to_string())))
            .transpose()?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_required_raster_arg(args, "input")?;
        let polygons_path = parse_required_vector_path_arg(args, "polygons")?;
        let maintain_dimensions = args
            .get("maintain_dimensions")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("running clip raster to polygon");
        let polygons_layer = read_vector_layer_aligned_to_raster(&input, polygons_path.as_str(), "polygons")?;
        let polygons = collect_layer_polygons(&polygons_layer)?;

        let mut output = if maintain_dimensions {
            build_output_like_raster(&input, input.data_type)
        } else {
            let mut min_x = f64::INFINITY;
            let mut min_y = f64::INFINITY;
            let mut max_x = f64::NEG_INFINITY;
            let mut max_y = f64::NEG_INFINITY;
            for (exterior, _) in &polygons {
                if let Some((a, b, c, d)) = polygon_world_bbox(exterior) {
                    min_x = min_x.min(a);
                    min_y = min_y.min(b);
                    max_x = max_x.max(c);
                    max_y = max_y.max(d);
                }
            }
            let ext = input.extent();
            let clip_min_x = min_x.max(ext.x_min);
            let clip_max_x = max_x.min(ext.x_max);
            let clip_min_y = min_y.max(ext.y_min);
            let clip_max_y = max_y.min(ext.y_max);
            if clip_min_x >= clip_max_x || clip_min_y >= clip_max_y {
                return Err(ToolError::Validation(
                    "polygon extent does not overlap the input raster".to_string(),
                ));
            }

            let rows = (((clip_max_y - clip_min_y) / input.cell_size_y).ceil() as usize).max(1);
            let cols = (((clip_max_x - clip_min_x) / input.cell_size_x).ceil() as usize).max(1);
            let y_min = clip_max_y - rows as f64 * input.cell_size_y;
            Raster::new(RasterConfig {
                cols,
                rows,
                bands: input.bands,
                x_min: clip_min_x,
                y_min,
                cell_size: input.cell_size_x,
                cell_size_y: Some(input.cell_size_y),
                nodata: input.nodata,
                data_type: input.data_type,
                crs: input.crs.clone(),
                metadata: input.metadata.clone(),
            })
        };

        for idx in 0..output.data.len() {
            output.data.set_f64(idx, output.nodata);
        }

        let total = polygons.len().max(1);
        for (poly_idx, (exterior, interiors)) in polygons.iter().enumerate() {
            let Some((rmin, cmin, rmax, cmax)) = polygon_bbox_pixels(&output, exterior) else {
                ctx.progress.progress((poly_idx + 1) as f64 / total as f64);
                continue;
            };
            for row in rmin..=rmax {
                let y = output.row_center_y(row as isize);
                for col in cmin..=cmax {
                    let x = output.col_center_x(col as isize);
                    if !polygon_contains_xy(exterior, interiors, x, y) {
                        continue;
                    }
                    let Some((src_col, src_row)) = input.world_to_pixel(x, y) else {
                        continue;
                    };
                    for band in 0..output.bands {
                        let value = input.get(band as isize, src_row, src_col);
                        let out_idx = output
                            .index(band as isize, row as isize, col as isize)
                            .ok_or_else(|| ToolError::Execution("output index out of bounds".to_string()))?;
                        output.data.set_f64(out_idx, value);
                    }
                }
            }
            ctx.progress.progress((poly_idx + 1) as f64 / total as f64);
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        Ok(GisOverlayCore::build_result(locator))
    }
}

impl Tool for ErasePolygonFromRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "erase_polygon_from_raster",
            display_name: "Erase Polygon From Raster",
            summary: "Sets raster cells inside polygons to NoData while preserving cells in polygon holes.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster.", required: true },
                ToolParamSpec { name: "polygons", description: "Input polygon vector layer.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("polygons".to_string(), json!("polygons.geojson"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("erase_polygon_from_raster.tif"));
        ToolManifest {
            id: "erase_polygon_from_raster".to_string(),
            display_name: "Erase Polygon From Raster".to_string(),
            summary: "Sets raster cells inside polygons to NoData while preserving cells in polygon holes.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input raster.".to_string(), required: true },
                ToolParamDescriptor { name: "polygons".to_string(), description: "Input polygon vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "erase_polygon_from_raster_basic".to_string(),
                description: "Erases polygon-covered raster cells to NoData.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "erase".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "input")?;
        let _ = parse_required_vector_path_arg(args, "polygons")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_required_raster_arg(args, "input")?;
        let polygons_path = parse_required_vector_path_arg(args, "polygons")?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("running erase polygon from raster");
        let polygons_layer = read_vector_layer_aligned_to_raster(&input, polygons_path.as_str(), "polygons")?;
        let polygons = collect_layer_polygons(&polygons_layer)?;

        let mut output = build_output_like_raster(&input, input.data_type);
        for idx in 0..output.data.len() {
            output.data.set_f64(idx, input.data.get_f64(idx));
        }

        let total = polygons.len().max(1);
        for (poly_idx, (exterior, interiors)) in polygons.iter().enumerate() {
            let Some((rmin, cmin, rmax, cmax)) = polygon_bbox_pixels(&output, exterior) else {
                ctx.progress.progress((poly_idx + 1) as f64 / total as f64);
                continue;
            };
            for row in rmin..=rmax {
                let y = output.row_center_y(row as isize);
                for col in cmin..=cmax {
                    let x = output.col_center_x(col as isize);
                    if !polygon_contains_xy(exterior, interiors, x, y) {
                        continue;
                    }
                    for band in 0..output.bands {
                        let out_idx = output
                            .index(band as isize, row as isize, col as isize)
                            .ok_or_else(|| ToolError::Execution("output index out of bounds".to_string()))?;
                        output.data.set_f64(out_idx, output.nodata);
                    }
                }
            }
            ctx.progress.progress((poly_idx + 1) as f64 / total as f64);
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        Ok(GisOverlayCore::build_result(locator))
    }
}

fn parse_units_arg(args: &ToolArgs) -> String {
    args.get("units")
        .and_then(|value| value.as_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_else(|| "map units".to_string())
}

fn class_bins_from_raster(input: &Raster) -> Result<(f64, usize), ToolError> {
    let (min_value, max_value) = min_max_valid(input)
        .ok_or_else(|| ToolError::Validation("input raster does not contain valid cells".to_string()))?;
    let min_class = min_value.floor();
    let max_class = max_value.floor();
    if !min_class.is_finite() || !max_class.is_finite() {
        return Err(ToolError::Validation(
            "input raster class range is not finite".to_string(),
        ));
    }

    let num_bins = (max_class - min_class + 1.0).max(1.0) as usize;
    if num_bins > 5_000_000 {
        return Err(ToolError::Validation(
            "input raster class range is too large for raster_area/raster_perimeter".to_string(),
        ));
    }
    Ok((min_class, num_bins))
}

fn class_index(value: f64, min_class: f64, num_bins: usize) -> Option<usize> {
    if !value.is_finite() {
        return None;
    }
    let idx = (value.floor() - min_class) as isize;
    if idx >= 0 && (idx as usize) < num_bins {
        Some(idx as usize)
    } else {
        None
    }
}

fn build_raster_result_with_table(path: String, table: String) -> ToolRunResult {
    let mut outputs = BTreeMap::new();
    outputs.insert("__wbw_type__".to_string(), json!("raster"));
    outputs.insert("path".to_string(), json!(path));
    outputs.insert("active_band".to_string(), json!(0));
    outputs.insert("table".to_string(), json!(table));
    ToolRunResult { outputs }
}

impl Tool for RasterAreaTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "raster_area",
            display_name: "Raster Area",
            summary: "Estimates per-class raster polygon area in grid-cell or map units and writes class totals to each class cell.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input categorical raster.", required: true },
                ToolParamSpec { name: "units", description: "Area units: 'map units' (default) or 'grid cells'.", required: false },
                ToolParamSpec { name: "zero_background", description: "If true, zero-valued cells are excluded from analysis.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("units".to_string(), json!("map units"));
        defaults.insert("zero_background".to_string(), json!(false));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("raster_area.tif"));

        ToolManifest {
            id: "raster_area".to_string(),
            display_name: "Raster Area".to_string(),
            summary: "Estimates per-class raster polygon area in grid-cell or map units and writes class totals to each class cell.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input categorical raster.".to_string(), required: true },
                ToolParamDescriptor { name: "units".to_string(), description: "Area units: 'map units' (default) or 'grid cells'.".to_string(), required: false },
                ToolParamDescriptor { name: "zero_background".to_string(), description: "If true, zero-valued cells are excluded from analysis.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "raster_area_basic".to_string(),
                description: "Computes per-class area from a categorical raster.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "raster".to_string(),
                "gis".to_string(),
                "area".to_string(),
                "categorical".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_required_raster_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let zero_background = args
            .get("zero_background")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let units = parse_units_arg(args);
        let grid_cell_units = units.contains("cell");

        let (min_class, num_bins) = class_bins_from_raster(&input)?;
        let mut class_area = vec![0.0f64; num_bins];
        let cell_area = input.cell_size_x * input.cell_size_y;
        let background = if zero_background { Some(0.0) } else { None };

        ctx.progress.info("running raster area");
        for row in 0..input.rows {
            for col in 0..input.cols {
                let index = input.index(0, row as isize, col as isize).ok_or_else(|| {
                    ToolError::Execution("computed raster index is out of bounds".to_string())
                })?;
                let value = input.data.get_f64(index);
                if input.is_nodata(value) || background == Some(value) {
                    continue;
                }
                if let Some(bin) = class_index(value, min_class, num_bins) {
                    class_area[bin] += if grid_cell_units { 1.0 } else { cell_area };
                }
            }
            ctx.progress.progress((row + 1) as f64 / input.rows.max(1) as f64);
        }

        let mut output = build_output_like_raster(&input, DataType::F64);
        output.nodata = -999.0;
        let out_nodata = output.nodata;
        for row in 0..input.rows {
            for col in 0..input.cols {
                let index = input.index(0, row as isize, col as isize).ok_or_else(|| {
                    ToolError::Execution("computed raster index is out of bounds".to_string())
                })?;
                let value = input.data.get_f64(index);
                if input.is_nodata(value) || background == Some(value) {
                    output.data.set_f64(index, out_nodata);
                    continue;
                }
                if let Some(bin) = class_index(value, min_class, num_bins) {
                    output.data.set_f64(index, class_area[bin]);
                } else {
                    output.data.set_f64(index, out_nodata);
                }
            }
        }

        let mut table = String::from(if grid_cell_units { "Class,Cells\n" } else { "Class,Area\n" });
        for (bin, area) in class_area.iter().enumerate() {
            if *area <= 0.0 {
                continue;
            }
            let class_value = min_class + bin as f64;
            table.push_str(&format!("{},{}\n", class_value, area));
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        Ok(build_raster_result_with_table(locator, table))
    }
}

const PERIMETER_LUT: [f64; 256] = [
    4.000000000, 2.828427125, 2.236067977, 2.414213562, 2.828427125, 3.000000000,
    2.414213562, 2.236067977, 2.236067977, 2.414213562, 2.000000000, 2.000000000,
    2.828427125, 1.414213562, 1.414213562, 1.414213562, 2.236067977, 2.828427125,
    2.000000000, 1.414213562, 2.414213562, 1.414213562, 2.000000000, 1.414213562,
    2.000000000, 2.000000000, 1.000000000, 2.000000000, 2.000000000, 2.000000000,
    2.000000000, 1.000000000, 2.828427125, 3.000000000, 2.828427125, 1.414213562,
    2.000000000, 4.000000000, 2.236067977, 2.236067977, 2.414213562, 2.236067977,
    1.414213562, 1.414213562, 2.236067977, 2.236067977, 1.414213562, 1.414213562,
    2.828427125, 2.236067977, 1.414213562, 1.414213562, 2.236067977, 2.414213562,
    2.000000000, 1.414213562, 2.000000000, 2.000000000, 1.000000000, 1.414213562,
    2.000000000, 2.000000000, 1.000000000, 1.000000000, 2.236067977, 2.828427125,
    2.000000000, 2.000000000, 2.828427125, 2.236067977, 2.000000000, 2.000000000,
    2.000000000, 1.414213562, 1.000000000, 2.000000000, 1.414213562, 1.414213562,
    1.000000000, 1.414213562, 2.000000000, 1.414213562, 1.000000000, 1.000000000,
    1.414213562, 1.414213562, 2.000000000, 1.414213562, 1.000000000, 1.000000000,
    0.000000000, 0.000000000, 1.000000000, 1.000000000, 0.000000000, 0.000000000,
    2.414213562, 1.414213562, 2.000000000, 2.000000000, 2.236067977, 2.414213562,
    2.000000000, 2.000000000, 2.000000000, 1.414213562, 2.000000000, 1.000000000,
    2.000000000, 1.414213562, 1.000000000, 1.000000000, 1.414213562, 1.414213562,
    1.000000000, 1.000000000, 1.414213562, 1.414213562, 1.000000000, 1.000000000,
    2.000000000, 1.414213562, 0.000000000, 0.000000000, 1.000000000, 1.000000000,
    0.000000000, 0.000000000, 2.828427125, 2.000000000, 2.828427125, 2.236067977,
    3.000000000, 4.000000000, 1.414213562, 2.236067977, 2.828427125, 2.236067977,
    1.414213562, 2.000000000, 2.236067977, 2.414213562, 1.414213562, 1.414213562,
    2.414213562, 2.236067977, 1.414213562, 1.414213562, 2.236067977, 2.236067977,
    1.414213562, 1.414213562, 2.000000000, 2.000000000, 1.000000000, 1.000000000,
    2.000000000, 2.000000000, 1.414213562, 1.000000000, 3.000000000, 4.000000000,
    2.236067977, 2.414213562, 4.000000000, 4.000000000, 2.414213562, 2.236067977,
    1.414213562, 2.236067977, 1.414213562, 1.414213562, 2.414213562, 2.236067977,
    1.414213562, 1.414213562, 1.414213562, 2.414213562, 1.414213562, 1.414213562,
    2.236067977, 2.236067977, 1.414213562, 1.414213562, 2.000000000, 2.000000000,
    1.000000000, 1.000000000, 2.000000000, 2.000000000, 1.000000000, 1.000000000,
    2.414213562, 2.000000000, 2.236067977, 2.000000000, 1.414213562, 2.414213562,
    2.000000000, 2.000000000, 1.414213562, 1.414213562, 1.000000000, 1.000000000,
    1.414213562, 1.414213562, 1.000000000, 1.000000000, 2.000000000, 2.000000000,
    2.000000000, 1.000000000, 1.414213562, 1.414213562, 1.000000000, 1.000000000,
    2.000000000, 1.000000000, 0.000000000, 0.000000000, 1.414213562, 1.000000000,
    0.000000000, 0.000000000, 2.236067977, 2.236067977, 2.000000000, 2.000000000,
    2.236067977, 2.236067977, 2.000000000, 2.000000000, 1.414213562, 1.414213562,
    1.414213562, 1.000000000, 1.414213562, 1.414213562, 1.000000000, 1.000000000,
    1.414213562, 1.414213562, 1.414213562, 1.000000000, 1.414213562, 1.414213562,
    1.000000000, 1.000000000, 1.000000000, 1.000000000, 0.000000000, 0.000000000,
    1.000000000, 1.000000000, 0.000000000, 0.000000000,
];
const PERIMETER_DX: [isize; 8] = [1, 1, 1, 0, -1, -1, -1, 0];
const PERIMETER_DY: [isize; 8] = [-1, 0, 1, 1, 1, 0, -1, -1];
const PERIMETER_MASK: [usize; 8] = [1, 2, 4, 8, 16, 32, 64, 128];

impl Tool for RasterPerimeterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "raster_perimeter",
            display_name: "Raster Perimeter",
            summary: "Estimates per-class raster polygon perimeter using an anti-aliasing lookup table and writes class totals to each class cell.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input categorical raster.", required: true },
                ToolParamSpec { name: "units", description: "Perimeter units: 'map units' (default) or 'grid cells'.", required: false },
                ToolParamSpec { name: "zero_background", description: "If true, zero-valued cells are excluded from analysis.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("units".to_string(), json!("map units"));
        defaults.insert("zero_background".to_string(), json!(false));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("raster_perimeter.tif"));

        ToolManifest {
            id: "raster_perimeter".to_string(),
            display_name: "Raster Perimeter".to_string(),
            summary: "Estimates per-class raster polygon perimeter using an anti-aliasing lookup table and writes class totals to each class cell.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input categorical raster.".to_string(), required: true },
                ToolParamDescriptor { name: "units".to_string(), description: "Perimeter units: 'map units' (default) or 'grid cells'.".to_string(), required: false },
                ToolParamDescriptor { name: "zero_background".to_string(), description: "If true, zero-valued cells are excluded from analysis.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "raster_perimeter_basic".to_string(),
                description: "Computes per-class perimeter from a categorical raster.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "raster".to_string(),
                "gis".to_string(),
                "perimeter".to_string(),
                "categorical".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_required_raster_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let zero_background = args
            .get("zero_background")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let units = parse_units_arg(args);
        let grid_cell_units = units.contains("cell");

        let (min_class, num_bins) = class_bins_from_raster(&input)?;
        let mut class_perimeter = vec![0.0f64; num_bins];
        let background = if zero_background { Some(0.0) } else { None };
        let scale = if grid_cell_units {
            1.0
        } else {
            (input.cell_size_x + input.cell_size_y) / 2.0
        };

        ctx.progress.info("running raster perimeter");
        for row in 0..input.rows {
            for col in 0..input.cols {
                let index = input.index(0, row as isize, col as isize).ok_or_else(|| {
                    ToolError::Execution("computed raster index is out of bounds".to_string())
                })?;
                let value = input.data.get_f64(index);
                if input.is_nodata(value) || background == Some(value) {
                    continue;
                }
                let Some(bin) = class_index(value, min_class, num_bins) else {
                    continue;
                };

                let mut pattern = 0usize;
                for n in 0..8 {
                    let rr = row as isize + PERIMETER_DY[n];
                    let cc = col as isize + PERIMETER_DX[n];
                    if rr < 0 || cc < 0 || rr >= input.rows as isize || cc >= input.cols as isize {
                        continue;
                    }
                    let neighbor = input.get(0, rr, cc);
                    if neighbor == value {
                        pattern += PERIMETER_MASK[n];
                    }
                }
                class_perimeter[bin] += PERIMETER_LUT[pattern] * scale;
            }
            ctx.progress.progress((row + 1) as f64 / input.rows.max(1) as f64);
        }

        let mut output = build_output_like_raster(&input, DataType::F64);
        output.nodata = -999.0;
        let out_nodata = output.nodata;
        for row in 0..input.rows {
            for col in 0..input.cols {
                let index = input.index(0, row as isize, col as isize).ok_or_else(|| {
                    ToolError::Execution("computed raster index is out of bounds".to_string())
                })?;
                let value = input.data.get_f64(index);
                if input.is_nodata(value) || background == Some(value) {
                    output.data.set_f64(index, out_nodata);
                    continue;
                }
                if let Some(bin) = class_index(value, min_class, num_bins) {
                    output.data.set_f64(index, class_perimeter[bin]);
                } else {
                    output.data.set_f64(index, out_nodata);
                }
            }
        }

        let mut table = String::from("Class,Perimeter\n");
        for (bin, perimeter) in class_perimeter.iter().enumerate() {
            if *perimeter <= 0.0 {
                continue;
            }
            let class_value = min_class + bin as f64;
            table.push_str(&format!("{},{}\n", class_value, perimeter));
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        Ok(build_raster_result_with_table(locator, table))
    }
}

#[derive(Clone, Copy)]
enum BufferCapChoice {
    Round,
    Flat,
    Square,
}

impl BufferCapChoice {
    fn parse(value: Option<&str>) -> Result<Self, ToolError> {
        match value.unwrap_or("round").trim().to_ascii_lowercase().as_str() {
            "round" => Ok(Self::Round),
            "flat" => Ok(Self::Flat),
            "square" => Ok(Self::Square),
            other => Err(ToolError::Validation(format!(
                "unsupported cap_style '{}'; expected round, flat, or square",
                other
            ))),
        }
    }

    fn to_topology(self) -> BufferCapStyle {
        match self {
            Self::Round => BufferCapStyle::Round,
            Self::Flat => BufferCapStyle::Flat,
            Self::Square => BufferCapStyle::Square,
        }
    }
}

#[derive(Clone, Copy)]
enum BufferJoinChoice {
    Round,
    Bevel,
    Mitre,
}

impl BufferJoinChoice {
    fn parse(value: Option<&str>) -> Result<Self, ToolError> {
        match value.unwrap_or("round").trim().to_ascii_lowercase().as_str() {
            "round" => Ok(Self::Round),
            "bevel" => Ok(Self::Bevel),
            "mitre" | "miter" => Ok(Self::Mitre),
            other => Err(ToolError::Validation(format!(
                "unsupported join_style '{}'; expected round, bevel, or mitre",
                other
            ))),
        }
    }

    fn to_topology(self) -> BufferJoinStyle {
        match self {
            Self::Round => BufferJoinStyle::Round,
            Self::Bevel => BufferJoinStyle::Bevel,
            Self::Mitre => BufferJoinStyle::Mitre,
        }
    }
}

fn detect_vector_output_format(path: &str) -> Result<wbvector::VectorFormat, ToolError> {
    match wbvector::VectorFormat::detect(path) {
        Ok(fmt) => Ok(fmt),
        Err(_) => {
            if std::path::Path::new(path).extension().is_none() {
                Ok(wbvector::VectorFormat::Shapefile)
            } else {
                Err(ToolError::Validation(format!(
                    "could not determine vector output format from path '{}'",
                    path
                )))
            }
        }
    }
}

fn write_vector_output(layer: &wbvector::Layer, path: &str) -> Result<String, ToolError> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ToolError::Execution(format!("failed creating output directory: {}", e)))?;
        }
    }

    let format = detect_vector_output_format(path)?;
    wbvector::write(layer, path, format)
        .map_err(|e| ToolError::Execution(format!("failed writing output vector: {}", e)))?;
    Ok(path.to_string())
}

fn build_vector_result(path: String) -> ToolRunResult {
    let mut outputs = BTreeMap::new();
    outputs.insert("path".to_string(), json!(path));
    ToolRunResult { outputs }
}

fn to_topo_coord(coord: &wbvector::Coord) -> TopoCoord {
    match coord.z {
        Some(z) => TopoCoord::xyz(coord.x, coord.y, z),
        None => TopoCoord::xy(coord.x, coord.y),
    }
}

fn to_wb_coord(coord: &TopoCoord) -> wbvector::Coord {
    match coord.z {
        Some(z) => wbvector::Coord::xyz(coord.x, coord.y, z),
        None => wbvector::Coord::xy(coord.x, coord.y),
    }
}

fn coords_nearly_equal(a: &wbvector::Coord, b: &wbvector::Coord, eps: f64) -> bool {
    (a.x - b.x).abs() <= eps
        && (a.y - b.y).abs() <= eps
        && match (a.z, b.z) {
            (Some(az), Some(bz)) => (az - bz).abs() <= eps,
            _ => true,
        }
}

fn to_wb_ring(ring: &TopoLinearRing) -> wbvector::Ring {
    let mut coords: Vec<wbvector::Coord> = ring.coords.iter().map(to_wb_coord).collect();
    let mut deduped = Vec::<wbvector::Coord>::with_capacity(coords.len());
    for coord in coords.drain(..) {
        if deduped
            .last()
            .map(|prev| coords_nearly_equal(prev, &coord, 1.0e-10))
            .unwrap_or(false)
        {
            continue;
        }
        deduped.push(coord);
    }

    if deduped.len() > 1 {
        let first = &deduped[0];
        let last = &deduped[deduped.len() - 1];
        if coords_nearly_equal(first, last, 1.0e-9) {
            deduped.pop();
        }
    }

    wbvector::Ring::new(deduped)
}

fn to_wb_rings(rings: &[TopoLinearRing]) -> Vec<wbvector::Ring> {
    rings.iter().map(to_wb_ring).collect()
}

fn to_topo_polygon(exterior: &wbvector::Ring, interiors: &[wbvector::Ring]) -> TopoPolygon {
    TopoPolygon::new(
        TopoLinearRing::new(exterior.coords().iter().map(to_topo_coord).collect()),
        interiors
            .iter()
            .map(|ring| TopoLinearRing::new(ring.coords().iter().map(to_topo_coord).collect()))
            .collect(),
    )
}

fn repair_polygons(polygons: Vec<TopoPolygon>) -> Vec<TopoPolygon> {
    let mut repaired = Vec::<TopoPolygon>::new();
    for polygon in polygons {
        let mut accepted = false;
        for epsilon in [1.0e-9, 1.0e-8, 1.0e-7, 1.0e-6, 1.0e-5] {
            let valid = make_valid_polygon(&polygon, epsilon);
            if valid.is_empty() {
                continue;
            }
            for poly in valid {
                if poly.exterior.coords.len() >= 4 {
                    repaired.push(poly);
                    accepted = true;
                }
            }
            if accepted {
                break;
            }
        }

        if !accepted && polygon.exterior.coords.len() >= 4 {
            repaired.push(polygon);
        }
    }
    repaired
}

fn polygons_to_wb_geometry(polygons: Vec<TopoPolygon>) -> Option<wbvector::Geometry> {
    if polygons.is_empty() {
        return None;
    }

    if polygons.len() == 1 {
        let polygon = &polygons[0];
        return Some(wbvector::Geometry::Polygon {
            exterior: to_wb_ring(&polygon.exterior),
            interiors: to_wb_rings(&polygon.holes),
        });
    }

    Some(wbvector::Geometry::MultiPolygon(
        polygons
            .iter()
            .map(|poly| (to_wb_ring(&poly.exterior), to_wb_rings(&poly.holes)))
            .collect(),
    ))
}

fn collect_buffered_polygons_from_geometry(
    geometry: &wbvector::Geometry,
    distance: f64,
    options: BufferOptions,
    polygons: &mut Vec<TopoPolygon>,
) -> Result<(), ToolError> {
    match geometry {
        wbvector::Geometry::Point(coord) => {
            polygons.push(buffer_point(to_topo_coord(coord), distance, options));
        }
        wbvector::Geometry::LineString(coords) => {
            let line = TopoLineString::new(coords.iter().map(to_topo_coord).collect());
            polygons.push(buffer_linestring(&line, distance, options));
        }
        wbvector::Geometry::Polygon { exterior, interiors } => {
            let input = to_topo_polygon(exterior, interiors);
            polygons.extend(buffer_polygon_multi(&input, distance, options));
        }
        wbvector::Geometry::MultiPoint(points) => {
            for point in points {
                polygons.push(buffer_point(to_topo_coord(point), distance, options));
            }
        }
        wbvector::Geometry::MultiLineString(lines) => {
            for line_coords in lines {
                let line = TopoLineString::new(line_coords.iter().map(to_topo_coord).collect());
                polygons.push(buffer_linestring(&line, distance, options));
            }
        }
        wbvector::Geometry::MultiPolygon(parts) => {
            for (exterior, interiors) in parts {
                let input = to_topo_polygon(exterior, interiors);
                polygons.extend(buffer_polygon_multi(&input, distance, options));
            }
        }
        wbvector::Geometry::GeometryCollection(parts) => {
            for part in parts {
                collect_buffered_polygons_from_geometry(part, distance, options, polygons)?;
            }
        }
    }
    Ok(())
}

fn buffer_feature_geometry(
    geometry: &wbvector::Geometry,
    distance: f64,
    options: BufferOptions,
) -> Result<Option<wbvector::Geometry>, ToolError> {
    let mut polygons = Vec::<TopoPolygon>::new();
    collect_buffered_polygons_from_geometry(geometry, distance, options, &mut polygons)?;
    Ok(polygons_to_wb_geometry(repair_polygons(polygons)))
}

impl Tool for BufferVectorTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "buffer_vector",
            display_name: "Buffer Vector",
            summary: "Creates polygon buffers around point, line, and polygon vector geometries with configurable cap and join styles.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector layer.", required: true },
                ToolParamSpec { name: "distance", description: "Buffer distance in map units.", required: true },
                ToolParamSpec { name: "quadrant_segments", description: "Arc resolution in segments per quadrant; defaults to 8.", required: false },
                ToolParamSpec { name: "cap_style", description: "Line end-cap style: round, flat, or square.", required: false },
                ToolParamSpec { name: "join_style", description: "Corner join style: round, bevel, or mitre.", required: false },
                ToolParamSpec { name: "mitre_limit", description: "Mitre join limit used when join_style=mitre; defaults to 5.0.", required: false },
                ToolParamSpec { name: "output", description: "Output vector path.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("features.shp"));
        defaults.insert("distance".to_string(), json!(10.0));
        defaults.insert("quadrant_segments".to_string(), json!(8));
        defaults.insert("cap_style".to_string(), json!("round"));
        defaults.insert("join_style".to_string(), json!("round"));
        defaults.insert("mitre_limit".to_string(), json!(5.0));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("buffer_vector.shp"));

        ToolManifest {
            id: "buffer_vector".to_string(),
            display_name: "Buffer Vector".to_string(),
            summary: "Creates polygon buffers around point, line, and polygon vector geometries with configurable cap and join styles.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "distance".to_string(), description: "Buffer distance in map units.".to_string(), required: true },
                ToolParamDescriptor { name: "quadrant_segments".to_string(), description: "Arc resolution in segments per quadrant; defaults to 8.".to_string(), required: false },
                ToolParamDescriptor { name: "cap_style".to_string(), description: "Line end-cap style: round, flat, or square.".to_string(), required: false },
                ToolParamDescriptor { name: "join_style".to_string(), description: "Corner join style: round, bevel, or mitre.".to_string(), required: false },
                ToolParamDescriptor { name: "mitre_limit".to_string(), description: "Mitre join limit used when join_style=mitre; defaults to 5.0.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector path.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "buffer_vector_basic".to_string(),
                description: "Buffers vector features and writes polygon output to disk.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "buffer".to_string(),
                "distance".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let distance = args
            .get("distance")
            .and_then(|value| value.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'distance' is required".to_string()))?;
        if !distance.is_finite() || distance <= 0.0 {
            return Err(ToolError::Validation(
                "distance must be a finite value greater than zero".to_string(),
            ));
        }

        let quadrant_segments = args
            .get("quadrant_segments")
            .and_then(|value| value.as_u64())
            .unwrap_or(8);
        if quadrant_segments == 0 {
            return Err(ToolError::Validation(
                "quadrant_segments must be greater than or equal to 1".to_string(),
            ));
        }

        let mitre_limit = args
            .get("mitre_limit")
            .and_then(|value| value.as_f64())
            .unwrap_or(5.0);
        if !mitre_limit.is_finite() || mitre_limit <= 0.0 {
            return Err(ToolError::Validation(
                "mitre_limit must be a finite value greater than zero".to_string(),
            ));
        }

        let _ = BufferCapChoice::parse(args.get("cap_style").and_then(|value| value.as_str()))?;
        let _ = BufferJoinChoice::parse(args.get("join_style").and_then(|value| value.as_str()))?;
        let output = parse_vector_path_arg(args, "output")?;
        let _ = detect_vector_output_format(output.trim())?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let distance = args
            .get("distance")
            .and_then(|value| value.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'distance' is required".to_string()))?;
        let quadrant_segments = args
            .get("quadrant_segments")
            .and_then(|value| value.as_u64())
            .unwrap_or(8)
            .max(1) as usize;
        let cap_style = BufferCapChoice::parse(args.get("cap_style").and_then(|value| value.as_str()))?;
        let join_style = BufferJoinChoice::parse(args.get("join_style").and_then(|value| value.as_str()))?;
        let mitre_limit = args
            .get("mitre_limit")
            .and_then(|value| value.as_f64())
            .unwrap_or(5.0);
        let output_path = parse_vector_path_arg(args, "output")?;

        let options = BufferOptions {
            quadrant_segments,
            cap_style: cap_style.to_topology(),
            join_style: join_style.to_topology(),
            mitre_limit,
        };

        ctx.progress.info("running buffer vector");
        let mut output = wbvector::Layer::new(format!("{}_buffer", input.name));
        output.schema = input.schema.clone();
        output.crs = input.crs.clone();
        output.geom_type = Some(wbvector::GeometryType::Polygon);

        let total = input.features.len().max(1);
        let completed = AtomicUsize::new(0);
        let results: Vec<Result<Option<wbvector::Feature>, ToolError>> = input
            .features
            .par_iter()
            .map(|feature| {
                let out = if let Some(geometry) = &feature.geometry {
                    buffer_feature_geometry(geometry, distance, options)?
                        .map(|buffered_geometry| wbvector::Feature {
                            fid: feature.fid,
                            geometry: Some(buffered_geometry),
                            attributes: feature.attributes.clone(),
                        })
                } else {
                    None
                };
                let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                ctx.progress.progress(done as f64 / total as f64);
                Ok(out)
            })
            .collect();
        for result in results {
            if let Some(feat) = result? {
                output.push(feat);
            }
        }

        ctx.progress.info("writing output vector");
        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

#[derive(Clone)]
struct OverlayPolygonPiece {
    attributes: Vec<wbvector::FieldValue>,
    polygon: TopoPolygon,
}

fn extract_polygons_from_geometry(
    geometry: &wbvector::Geometry,
    polygons: &mut Vec<TopoPolygon>,
) -> Result<(), ToolError> {
    match geometry {
        wbvector::Geometry::Polygon { exterior, interiors } => {
            polygons.push(to_topo_polygon(exterior, interiors));
        }
        wbvector::Geometry::MultiPolygon(parts) => {
            for (exterior, interiors) in parts {
                polygons.push(to_topo_polygon(exterior, interiors));
            }
        }
        wbvector::Geometry::GeometryCollection(parts) => {
            for part in parts {
                extract_polygons_from_geometry(part, polygons)?;
            }
        }
        _ => {
            return Err(ToolError::Validation(
                "overlay tools currently support polygon and multipolygon geometries only"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

fn collect_overlay_polygon_pieces(
    layer: &wbvector::Layer,
) -> Result<Vec<OverlayPolygonPiece>, ToolError> {
    let mut pieces = Vec::<OverlayPolygonPiece>::new();
    for feature in &layer.features {
        let Some(geometry) = feature.geometry.as_ref() else {
            continue;
        };
        let mut polygons = Vec::<TopoPolygon>::new();
        extract_polygons_from_geometry(geometry, &mut polygons)?;
        for polygon in polygons {
            pieces.push(OverlayPolygonPiece {
                attributes: feature.attributes.clone(),
                polygon,
            });
        }
    }
    Ok(pieces)
}

fn build_merged_overlay_schema(
    input: &wbvector::Layer,
    overlay: &wbvector::Layer,
) -> (wbvector::Schema, Vec<usize>, Vec<usize>) {
    let mut merged = wbvector::Schema::new();
    let mut input_mapping = vec![0usize; input.schema.len()];
    let mut overlay_mapping = vec![0usize; overlay.schema.len()];

    for (i, field) in input.schema.fields().iter().enumerate() {
        if let Some(idx) = merged.field_index(&field.name) {
            input_mapping[i] = idx;
        } else {
            merged.add_field(field.clone());
            input_mapping[i] = merged.len() - 1;
        }
    }

    for (i, field) in overlay.schema.fields().iter().enumerate() {
        if let Some(idx) = merged.field_index(&field.name) {
            overlay_mapping[i] = idx;
        } else {
            merged.add_field(field.clone());
            overlay_mapping[i] = merged.len() - 1;
        }
    }

    (merged, input_mapping, overlay_mapping)
}

fn merged_overlay_attributes(
    input_attrs: Option<&[wbvector::FieldValue]>,
    overlay_attrs: Option<&[wbvector::FieldValue]>,
    input_mapping: &[usize],
    overlay_mapping: &[usize],
    schema_len: usize,
) -> Vec<wbvector::FieldValue> {
    let mut out = vec![wbvector::FieldValue::Null; schema_len];

    if let Some(attrs) = input_attrs {
        for (src_idx, value) in attrs.iter().enumerate() {
            if src_idx < input_mapping.len() {
                out[input_mapping[src_idx]] = value.clone();
            }
        }
    }

    if let Some(attrs) = overlay_attrs {
        for (src_idx, value) in attrs.iter().enumerate() {
            if src_idx < overlay_mapping.len() {
                out[overlay_mapping[src_idx]] = value.clone();
            }
        }
    }

    out
}

fn push_topo_polygon_feature(
    output: &mut wbvector::Layer,
    fid: u64,
    polygon: TopoPolygon,
    attributes: Vec<wbvector::FieldValue>,
) {
    output.push(wbvector::Feature {
        fid,
        geometry: Some(wbvector::Geometry::Polygon {
            exterior: to_wb_ring(&polygon.exterior),
            interiors: to_wb_rings(&polygon.holes),
        }),
        attributes,
    });
}

fn parse_overlay_snap_tolerance(args: &ToolArgs) -> f64 {
    args.get("snap_tolerance")
        .and_then(|value| value.as_f64())
        .unwrap_or(f64::EPSILON)
}

fn validate_overlay_common_args(args: &ToolArgs) -> Result<(), ToolError> {
    let _ = load_vector_arg(args, "input")?;
    let _ = load_vector_arg(args, "overlay")?;
    let _ = parse_vector_path_arg(args, "output")?;
    let epsilon = parse_overlay_snap_tolerance(args);
    if !epsilon.is_finite() || epsilon < 0.0 {
        return Err(ToolError::Validation(
            "snap_tolerance must be a finite value >= 0".to_string(),
        ));
    }
    Ok(())
}

fn read_vector_layer_from_path(path: &str, input_name: &str) -> Result<wbvector::Layer, ToolError> {
    wbvector::read(path).map_err(|e| {
        ToolError::Validation(format!(
            "failed reading {} vector '{}': {}",
            input_name, path, e
        ))
    })
}

fn align_overlay_layers_crs(
    mut input: wbvector::Layer,
    mut overlay: wbvector::Layer,
) -> Result<(wbvector::Layer, wbvector::Layer), ToolError> {
    let input_epsg = input.crs_epsg();
    let input_wkt = input.crs_wkt().map(str::trim).filter(|s| !s.is_empty());
    let overlay_epsg = overlay.crs_epsg();
    let overlay_wkt = overlay.crs_wkt().map(str::trim).filter(|s| !s.is_empty());

    if input_epsg.is_none() && input_wkt.is_none() {
        return Ok((input, overlay));
    }
    if overlay_epsg.is_none() && overlay_wkt.is_none() {
        return Ok((input, overlay));
    }

    let epsg_matches = input_epsg.is_some() && input_epsg == overlay_epsg;
    let wkt_matches = match (input_wkt, overlay_wkt) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    };
    if epsg_matches || wkt_matches {
        return Ok((input, overlay));
    }

    if let Some(dst_epsg) = input_epsg {
        overlay = overlay.reproject_to_epsg(dst_epsg).map_err(|e| {
            ToolError::Validation(format!(
                "overlay vector CRS does not match input vector CRS; automatic reprojection to EPSG:{} failed: {}",
                dst_epsg, e
            ))
        })?;
        return Ok((input, overlay));
    }

    if let Some(dst_epsg) = overlay_epsg {
        input = input.reproject_to_epsg(dst_epsg).map_err(|e| {
            ToolError::Validation(format!(
                "input vector CRS does not match overlay vector CRS; automatic reprojection to EPSG:{} failed: {}",
                dst_epsg, e
            ))
        })?;
        return Ok((input, overlay));
    }

    Err(ToolError::Validation(
        "input and overlay vector CRS do not match and neither layer provides an EPSG code for automatic reprojection"
            .to_string(),
    ))
}

fn load_overlay_layers_aligned(args: &ToolArgs) -> Result<(wbvector::Layer, wbvector::Layer), ToolError> {
    let input_path = parse_required_vector_path_arg(args, "input")?;
    let overlay_path = parse_required_vector_path_arg(args, "overlay")?;
    let input = read_vector_layer_from_path(input_path.as_str(), "input")?;
    let overlay = read_vector_layer_from_path(overlay_path.as_str(), "overlay")?;
    align_overlay_layers_crs(input, overlay)
}

fn overlay_tool_params() -> Vec<ToolParamSpec> {
    vec![
        ToolParamSpec {
            name: "input",
            description: "Input vector layer.",
            required: true,
        },
        ToolParamSpec {
            name: "overlay",
            description: "Overlay polygon layer.",
            required: true,
        },
        ToolParamSpec {
            name: "snap_tolerance",
            description:
                "Optional snapping tolerance used by topology operations; defaults to machine epsilon.",
            required: false,
        },
        ToolParamSpec {
            name: "output",
            description: "Output vector path.",
            required: true,
        },
    ]
}

fn overlay_manifest(
    id: &str,
    display_name: &str,
    summary: &str,
    example_name: &str,
    example_output: &str,
) -> ToolManifest {
    let mut defaults = ToolArgs::new();
    defaults.insert("input".to_string(), json!("input_polygons.shp"));
    defaults.insert("overlay".to_string(), json!("overlay_polygons.shp"));
    defaults.insert("snap_tolerance".to_string(), json!(f64::EPSILON));

    let mut example_args = defaults.clone();
    example_args.insert("output".to_string(), json!(example_output));

    ToolManifest {
        id: id.to_string(),
        display_name: display_name.to_string(),
        summary: summary.to_string(),
        category: ToolCategory::Vector,
        license_tier: LicenseTier::Open,
        params: vec![
            ToolParamDescriptor {
                name: "input".to_string(),
                description: "Input vector layer.".to_string(),
                required: true,
            },
            ToolParamDescriptor {
                name: "overlay".to_string(),
                description: "Overlay polygon layer.".to_string(),
                required: true,
            },
            ToolParamDescriptor {
                name: "snap_tolerance".to_string(),
                description: "Optional snapping tolerance used by topology operations; defaults to machine epsilon.".to_string(),
                required: false,
            },
            ToolParamDescriptor {
                name: "output".to_string(),
                description: "Output vector path.".to_string(),
                required: true,
            },
        ],
        defaults,
        examples: vec![ToolExample {
            name: example_name.to_string(),
            description: summary.to_string(),
            args: example_args,
        }],
        tags: vec![
            "vector".to_string(),
            "gis".to_string(),
            "overlay".to_string(),
            "legacy-port".to_string(),
        ],
        stability: ToolStability::Experimental,
    }
}

impl Tool for ClipTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "clip",
            display_name: "Clip",
            summary: "Clips input polygons to overlay polygon boundaries using topology-based intersection.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: overlay_tool_params(),
        }
    }

    fn manifest(&self) -> ToolManifest {
        overlay_manifest(
            "clip",
            "Clip",
            "Clips input polygons to overlay polygon boundaries using topology-based intersection.",
            "clip_basic",
            "clip.shp",
        )
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        validate_overlay_common_args(args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (input, overlay) = load_overlay_layers_aligned(args)?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let epsilon = parse_overlay_snap_tolerance(args);

        let input_pieces = collect_overlay_polygon_pieces(&input)?;
        let overlay_pieces = collect_overlay_polygon_pieces(&overlay)?;
        let mut overlay_polys = Vec::with_capacity(overlay_pieces.len());
        for piece in &overlay_pieces {
            overlay_polys.push(piece.polygon.clone());
        }
        let dissolved_overlay = polygon_unary_dissolve(&overlay_polys, epsilon);

        let mut output = wbvector::Layer::new(format!("{}_clip", input.name));
        output.schema = input.schema.clone();
        output.crs = input.crs.clone();
        output.geom_type = Some(wbvector::GeometryType::Polygon);

        let total = input_pieces.len().max(1);
        let mut next_fid = 1u64;
        for (index, piece) in input_pieces.iter().enumerate() {
            let mut intersections = Vec::<TopoPolygon>::new();
            for overlay_group in &dissolved_overlay {
                intersections.extend(polygon_intersection(
                    &piece.polygon,
                    &overlay_group.poly,
                    epsilon,
                ));
            }
            let intersections = polygon_unary_dissolve(&intersections, epsilon);
            for group in intersections {
                push_topo_polygon_feature(
                    &mut output,
                    next_fid,
                    group.poly,
                    piece.attributes.clone(),
                );
                next_fid += 1;
            }
            ctx.progress
                .progress((index + 1) as f64 / total as f64);
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for DifferenceTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "difference",
            display_name: "Difference",
            summary: "Removes overlay polygon areas from input polygons using topology-based difference.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: overlay_tool_params(),
        }
    }

    fn manifest(&self) -> ToolManifest {
        overlay_manifest(
            "difference",
            "Difference",
            "Removes overlay polygon areas from input polygons using topology-based difference.",
            "difference_basic",
            "difference.shp",
        )
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        validate_overlay_common_args(args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (input, overlay) = load_overlay_layers_aligned(args)?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let epsilon = parse_overlay_snap_tolerance(args);

        let input_pieces = collect_overlay_polygon_pieces(&input)?;
        let overlay_pieces = collect_overlay_polygon_pieces(&overlay)?;
        let mut overlay_polys = Vec::with_capacity(overlay_pieces.len());
        for piece in &overlay_pieces {
            overlay_polys.push(piece.polygon.clone());
        }
        let dissolved_overlay = polygon_unary_dissolve(&overlay_polys, epsilon);

        let mut output = wbvector::Layer::new(format!("{}_difference", input.name));
        output.schema = input.schema.clone();
        output.crs = input.crs.clone();
        output.geom_type = Some(wbvector::GeometryType::Polygon);

        let total = input_pieces.len().max(1);
        let mut next_fid = 1u64;
        for (index, piece) in input_pieces.iter().enumerate() {
            let mut remainder = vec![piece.polygon.clone()];
            for overlay_group in &dissolved_overlay {
                let mut next = Vec::<TopoPolygon>::new();
                for poly in &remainder {
                    next.extend(polygon_difference(poly, &overlay_group.poly, epsilon));
                }
                remainder = next;
                if remainder.is_empty() {
                    break;
                }
            }

            for poly in remainder {
                push_topo_polygon_feature(&mut output, next_fid, poly, piece.attributes.clone());
                next_fid += 1;
            }
            ctx.progress
                .progress((index + 1) as f64 / total as f64);
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for EraseTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "erase",
            display_name: "Erase",
            summary: "Erases overlay polygon areas from input polygons and preserves input attributes.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: overlay_tool_params(),
        }
    }

    fn manifest(&self) -> ToolManifest {
        overlay_manifest(
            "erase",
            "Erase",
            "Erases overlay polygon areas from input polygons and preserves input attributes.",
            "erase_basic",
            "erase.shp",
        )
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        validate_overlay_common_args(args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        DifferenceTool.run(args, ctx)
    }
}

impl Tool for IntersectTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "intersect",
            display_name: "Intersect",
            summary: "Intersects input and overlay polygons using topology-based overlay and tracks source feature IDs.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: overlay_tool_params(),
        }
    }

    fn manifest(&self) -> ToolManifest {
        overlay_manifest(
            "intersect",
            "Intersect",
            "Intersects input and overlay polygons using topology-based overlay and tracks source feature IDs.",
            "intersect_basic",
            "intersect.shp",
        )
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        validate_overlay_common_args(args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (input, overlay) = load_overlay_layers_aligned(args)?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let epsilon = parse_overlay_snap_tolerance(args);

        let input_pieces = collect_overlay_polygon_pieces(&input)?;
        let overlay_pieces = collect_overlay_polygon_pieces(&overlay)?;
        let (merged_schema, input_mapping, overlay_mapping) =
            build_merged_overlay_schema(&input, &overlay);
        let merged_len = merged_schema.len();

        let mut output = wbvector::Layer::new(format!("{}_intersect", input.name));
        output.schema = merged_schema;
        output.crs = input.crs.clone();
        output.geom_type = Some(wbvector::GeometryType::Polygon);

        let total = input_pieces.len().max(1);
        let mut next_fid = 1u64;
        for (index, input_piece) in input_pieces.iter().enumerate() {
            for overlay_piece in &overlay_pieces {
                let intersections =
                    polygon_intersection(&input_piece.polygon, &overlay_piece.polygon, epsilon);
                for poly in intersections {
                    let attrs = merged_overlay_attributes(
                        Some(&input_piece.attributes),
                        Some(&overlay_piece.attributes),
                        &input_mapping,
                        &overlay_mapping,
                        merged_len,
                    );
                    push_topo_polygon_feature(
                        &mut output,
                        next_fid,
                        poly,
                        attrs,
                    );
                    next_fid += 1;
                }
            }
            ctx.progress
                .progress((index + 1) as f64 / total as f64);
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for FilterVectorFeaturesByAreaTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "filter_vector_features_by_area",
            display_name: "Filter Vector Features By Area",
            summary: "Filters polygon features below a minimum area threshold.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input polygon vector layer.",
                    required: true,
                },
                ToolParamSpec {
                    name: "threshold",
                    description: "Minimum polygon area to retain, in layer coordinate units squared.",
                    required: true,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output vector path.",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("polygons.shp"));
        defaults.insert("threshold".to_string(), json!(1000.0));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("filtered_polygons.shp"));

        ToolManifest {
            id: "filter_vector_features_by_area".to_string(),
            display_name: "Filter Vector Features By Area".to_string(),
            summary: "Filters polygon features below a minimum area threshold.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input polygon vector layer.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "threshold".to_string(),
                    description: "Minimum polygon area to retain, in layer coordinate units squared.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output vector path.".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "filter_vector_features_by_area_basic".to_string(),
                description: "Removes polygons smaller than the specified area threshold.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "filter".to_string(),
                "polygon".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;

        let threshold = args
            .get("threshold")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'threshold' is required".to_string()))?;
        if !threshold.is_finite() || threshold < 0.0 {
            return Err(ToolError::Validation(
                "threshold must be a finite value >= 0".to_string(),
            ));
        }

        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let threshold = args
            .get("threshold")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'threshold' is required".to_string()))?;
        let output_path = parse_vector_path_arg(args, "output")?;

        let mut output = wbvector::Layer::new(format!("{}_filtered", input.name));
        output.schema = input.schema.clone();
        output.geom_type = input.geom_type;
        output.crs = input.crs.clone();

        let total = input.features.len().max(1);
        let mut next_fid = 1u64;
        for (idx, feature) in input.features.iter().enumerate() {
            if let Some(geometry) = &feature.geometry {
                let area = get_polygon_area(geometry).ok_or_else(|| {
                    ToolError::Validation("input layer must contain polygon geometries".to_string())
                })?;
                if area.abs() > threshold {
                    output.push(wbvector::Feature {
                        fid: next_fid,
                        geometry: Some(geometry.clone()),
                        attributes: feature.attributes.clone(),
                    });
                    next_fid += 1;
                }
            }
            ctx.progress.progress((idx + 1) as f64 / total as f64);
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for ExtractNodesTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "extract_nodes",
            display_name: "Extract Nodes",
            summary: "Converts polyline and polygon vertices into point features.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input polyline or polygon vector layer.",
                    required: true,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output point vector path.",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input_lines.shp"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("nodes.shp"));

        ToolManifest {
            id: "extract_nodes".to_string(),
            display_name: "Extract Nodes".to_string(),
            summary: "Converts polyline and polygon vertices into point features.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input polyline or polygon vector layer.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output point vector path.".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "extract_nodes_basic".to_string(),
                description: "Extracts vertex points from line and polygon features.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "nodes".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;

        let mut output = wbvector::Layer::new(format!("{}_nodes", input.name));
        output.geom_type = Some(wbvector::GeometryType::Point);
        output.crs = input.crs.clone();
        output.schema = wbvector::Schema::new();
        output
            .schema
            .add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));
        output
            .schema
            .add_field(wbvector::FieldDef::new("PARENT_ID", wbvector::FieldType::Integer));

        let total = input.features.len().max(1);
        let mut next_fid = 1u64;

        for (feature_idx, feature) in input.features.iter().enumerate() {
            if let Some(geometry) = &feature.geometry {
                match geometry {
                    wbvector::Geometry::LineString(_)
                    | wbvector::Geometry::MultiLineString(_)
                    | wbvector::Geometry::Polygon { .. }
                    | wbvector::Geometry::MultiPolygon(_) => {}
                    _ => {
                        return Err(ToolError::Validation(
                            "input layer must contain only polyline or polygon geometries"
                                .to_string(),
                        ));
                    }
                }
                let mut coords = Vec::<wbvector::Coord>::new();
                collect_all_coords_from_geometry(geometry, &mut coords);
                for coord in coords {
                    output.push(wbvector::Feature {
                        fid: next_fid,
                        geometry: Some(wbvector::Geometry::Point(coord)),
                        attributes: vec![
                            wbvector::FieldValue::Integer(next_fid as i64),
                            wbvector::FieldValue::Integer((feature_idx as i64) + 1),
                        ],
                    });
                    next_fid += 1;
                }
            }
            ctx.progress.progress((feature_idx + 1) as f64 / total as f64);
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for CentroidVectorTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "centroid_vector",
            display_name: "Centroid Vector",
            summary: "Computes centroid points from vector features.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input vector layer.",
                    required: true,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output point vector path.",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.shp"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("centroid_vector.shp"));

        ToolManifest {
            id: "centroid_vector".to_string(),
            display_name: "Centroid Vector".to_string(),
            summary: "Computes centroid points from vector features.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input vector layer.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output point vector path.".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "centroid_vector_basic".to_string(),
                description: "Creates centroid points from vector input features.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "centroid".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;

        let mut has_point_geom = false;
        let mut has_non_point_geom = false;
        for feature in &input.features {
            let Some(geometry) = feature.geometry.as_ref() else {
                continue;
            };
            match geometry {
                wbvector::Geometry::Point(_) | wbvector::Geometry::MultiPoint(_) => {
                    has_point_geom = true;
                }
                _ => {
                    has_non_point_geom = true;
                }
            }
        }
        let is_point_layer = has_point_geom && !has_non_point_geom;

        let mut output = wbvector::Layer::new(format!("{}_centroid", input.name));
        output.geom_type = Some(wbvector::GeometryType::Point);
        output.crs = input.crs.clone();

        if is_point_layer {
            output.schema = wbvector::Schema::new();
            output
                .schema
                .add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));

            let mut sum_x = 0.0;
            let mut sum_y = 0.0;
            let mut count = 0usize;
            let total = input.features.len().max(1);

            for (idx, feature) in input.features.iter().enumerate() {
                if let Some(geometry) = &feature.geometry {
                    let mut coords = Vec::<wbvector::Coord>::new();
                    collect_all_coords_from_geometry(geometry, &mut coords);
                    for coord in coords {
                        sum_x += coord.x;
                        sum_y += coord.y;
                        count += 1;
                    }
                }
                ctx.progress.progress((idx + 1) as f64 / total as f64);
            }

            if count > 0 {
                output.push(wbvector::Feature {
                    fid: 1,
                    geometry: Some(wbvector::Geometry::Point(wbvector::Coord::xy(
                        sum_x / count as f64,
                        sum_y / count as f64,
                    ))),
                    attributes: vec![wbvector::FieldValue::Integer(1)],
                });
            }
        } else {
            output.schema = input.schema.clone();
            let total = input.features.len().max(1);
            let mut next_fid = 1u64;

            for (idx, feature) in input.features.iter().enumerate() {
                if let Some(geometry) = &feature.geometry {
                    let mut coords = Vec::<wbvector::Coord>::new();
                    collect_all_coords_from_geometry(geometry, &mut coords);
                    if !coords.is_empty() {
                        let mut sum_x = 0.0;
                        let mut sum_y = 0.0;
                        for coord in &coords {
                            sum_x += coord.x;
                            sum_y += coord.y;
                        }
                        output.push(wbvector::Feature {
                            fid: if feature.fid == 0 { next_fid } else { feature.fid },
                            geometry: Some(wbvector::Geometry::Point(wbvector::Coord::xy(
                                sum_x / coords.len() as f64,
                                sum_y / coords.len() as f64,
                            ))),
                            attributes: feature.attributes.clone(),
                        });
                        next_fid += 1;
                    }
                }
                ctx.progress.progress((idx + 1) as f64 / total as f64);
            }
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for ExtractByAttributeTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "extract_by_attribute",
            display_name: "Extract By Attribute",
            summary: "Extracts vector features that satisfy an attribute expression.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input vector layer.",
                    required: true,
                },
                ToolParamSpec {
                    name: "statement",
                    description: "Boolean expression evaluated against attribute fields.",
                    required: true,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output vector path.",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.shp"));
        defaults.insert("statement".to_string(), json!("VALUE >= 10"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("extract_by_attribute.shp"));

        ToolManifest {
            id: "extract_by_attribute".to_string(),
            display_name: "Extract By Attribute".to_string(),
            summary: "Extracts vector features that satisfy an attribute expression.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input vector layer.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "statement".to_string(),
                    description: "Boolean expression evaluated against attribute fields.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output vector path.".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "extract_by_attribute_basic".to_string(),
                description: "Selects features whose attributes satisfy a boolean statement.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "query".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let statement = args
            .get("statement")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .ok_or_else(|| ToolError::Validation("parameter 'statement' is required".to_string()))?;
        if statement.is_empty() {
            return Err(ToolError::Validation(
                "statement must be a non-empty expression".to_string(),
            ));
        }
        build_operator_tree::<evalexpr::DefaultNumericTypes>(statement).map_err(|e| {
            ToolError::Validation(format!("invalid statement expression: {e}"))
        })?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let statement = args
            .get("statement")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .ok_or_else(|| ToolError::Validation("parameter 'statement' is required".to_string()))?;
        let tree = build_operator_tree::<evalexpr::DefaultNumericTypes>(statement)
            .map_err(|e| ToolError::Validation(format!("invalid statement expression: {e}")))?;
        let output_path = parse_vector_path_arg(args, "output")?;

        let mut output = wbvector::Layer::new(format!("{}_extracted", input.name));
        output.schema = input.schema.clone();
        output.geom_type = input.geom_type;
        output.crs = input.crs.clone();

        let has_fid_field = input
            .schema
            .fields()
            .iter()
            .any(|f| f.name.eq_ignore_ascii_case("FID"));

        let total = input.features.len().max(1);
        let mut next_fid = 1u64;

        for (feature_idx, feature) in input.features.iter().enumerate() {
            let mut context = HashMapContext::new();

            for (field_idx, field) in input.schema.fields().iter().enumerate() {
                let value = feature
                    .attributes
                    .get(field_idx)
                    .unwrap_or(&wbvector::FieldValue::Null);
                let eval_value = match value {
                    wbvector::FieldValue::Integer(v) => Value::Int(*v),
                    wbvector::FieldValue::Float(v) => Value::Float(*v),
                    wbvector::FieldValue::Boolean(v) => Value::Boolean(*v),
                    wbvector::FieldValue::Text(v)
                    | wbvector::FieldValue::Date(v)
                    | wbvector::FieldValue::DateTime(v) => Value::String(v.clone()),
                    wbvector::FieldValue::Blob(_) | wbvector::FieldValue::Null => {
                        Value::String("null".to_string())
                    }
                };
                let _ = context.set_value(field.name.clone(), eval_value);
            }

            let _ = context.set_value("null".to_string(), Value::String("null".to_string()));
            let _ = context.set_value("NULL".to_string(), Value::String("null".to_string()));
            let _ = context.set_value("none".to_string(), Value::String("null".to_string()));
            let _ = context.set_value("NONE".to_string(), Value::String("null".to_string()));
            let _ = context.set_value("nodata".to_string(), Value::String("null".to_string()));
            let _ = context.set_value("NoData".to_string(), Value::String("null".to_string()));
            let _ = context.set_value("NODATA".to_string(), Value::String("null".to_string()));
            let _ = context.set_value("pi".to_string(), Value::Float(std::f64::consts::PI));
            let _ = context.set_value("PI".to_string(), Value::Float(std::f64::consts::PI));

            if !has_fid_field {
                let _ = context.set_value("FID".to_string(), Value::Int((feature_idx as i64) + 1));
            }

            let keep = tree.eval_boolean_with_context(&context).map_err(|e| {
                ToolError::Execution(format!("statement evaluation failed for feature {}: {e}", feature_idx + 1))
            })?;

            if keep {
                output.push(wbvector::Feature {
                    fid: if feature.fid == 0 { next_fid } else { feature.fid },
                    geometry: feature.geometry.clone(),
                    attributes: feature.attributes.clone(),
                });
                next_fid += 1;
            }

            ctx.progress.progress((feature_idx + 1) as f64 / total as f64);
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for DissolveTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "dissolve",
            display_name: "Dissolve",
            summary: "Removes shared polygon boundaries globally or by a dissolve attribute field.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input polygon vector layer.",
                    required: true,
                },
                ToolParamSpec {
                    name: "dissolve_field",
                    description: "Optional attribute field used to dissolve polygons within groups; if omitted, dissolves all polygons.",
                    required: false,
                },
                ToolParamSpec {
                    name: "snap_tolerance",
                    description: "Optional snapping tolerance used during topology operations.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output polygon vector path.",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input_polygons.shp"));
        defaults.insert("dissolve_field".to_string(), json!(""));
        defaults.insert("snap_tolerance".to_string(), json!(f64::EPSILON));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("dissolve.shp"));

        ToolManifest {
            id: "dissolve".to_string(),
            display_name: "Dissolve".to_string(),
            summary: "Removes shared polygon boundaries globally or by a dissolve attribute field.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input polygon vector layer.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "dissolve_field".to_string(),
                    description: "Optional attribute field used to dissolve polygons within groups; if omitted, dissolves all polygons.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "snap_tolerance".to_string(),
                    description: "Optional snapping tolerance used during topology operations.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output polygon vector path.".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "dissolve_basic".to_string(),
                description: "Dissolves polygon boundaries by an optional grouping field.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "polygon".to_string(),
                "dissolve".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        let epsilon = args
            .get("snap_tolerance")
            .and_then(|value| value.as_f64())
            .unwrap_or(f64::EPSILON);
        if !epsilon.is_finite() || epsilon < 0.0 {
            return Err(ToolError::Validation(
                "snap_tolerance must be a finite value >= 0".to_string(),
            ));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let epsilon = args
            .get("snap_tolerance")
            .and_then(|value| value.as_f64())
            .unwrap_or(f64::EPSILON);
        let dissolve_field = args
            .get("dissolve_field")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty());

        let dissolve_field_index = dissolve_field.and_then(|name| {
            input.schema.field_index(name).or_else(|| {
                input
                    .schema
                    .fields()
                    .iter()
                    .position(|field| field.name.eq_ignore_ascii_case(name))
            })
        });

        let mut output = wbvector::Layer::new(format!("{}_dissolve", input.name));
        output.geom_type = Some(wbvector::GeometryType::Polygon);
        output.crs = input.crs.clone();
        output.schema = wbvector::Schema::new();
        if let Some(field_idx) = dissolve_field_index {
            if let Some(field_def) = input.schema.fields().get(field_idx) {
                output.schema.add_field(field_def.clone());
            }
        }

        let mut grouped_polygons: Vec<(wbvector::FieldValue, Vec<TopoPolygon>)> = Vec::new();
        let mut all_polygons = Vec::<TopoPolygon>::new();
        let total = input.features.len().max(1);

        for (feature_idx, feature) in input.features.iter().enumerate() {
            let Some(geometry) = feature.geometry.as_ref() else {
                ctx.progress.progress((feature_idx + 1) as f64 / total as f64);
                continue;
            };

            let mut polygons = Vec::<TopoPolygon>::new();
            extract_polygons_from_geometry(geometry, &mut polygons)?;

            if let Some(field_idx) = dissolve_field_index {
                let value = feature
                    .attributes
                    .get(field_idx)
                    .cloned()
                    .unwrap_or(wbvector::FieldValue::Null);
                if let Some((_, group)) = grouped_polygons
                    .iter_mut()
                    .find(|(candidate, _)| *candidate == value)
                {
                    group.extend(polygons);
                } else {
                    grouped_polygons.push((value, polygons));
                }
            } else {
                all_polygons.extend(polygons);
            }

            ctx.progress.progress((feature_idx + 1) as f64 / total as f64);
        }

        let mut next_fid = 1u64;
        if dissolve_field_index.is_some() {
            let total_groups = grouped_polygons.len().max(1);
            for (group_idx, (field_value, polygons)) in grouped_polygons.into_iter().enumerate() {
                for dissolved in polygon_unary_dissolve(&polygons, epsilon) {
                    push_topo_polygon_feature(&mut output, next_fid, dissolved.poly, vec![field_value.clone()]);
                    next_fid += 1;
                }
                ctx.progress.progress((group_idx + 1) as f64 / total_groups as f64);
            }
        } else {
            for dissolved in polygon_unary_dissolve(&all_polygons, epsilon) {
                push_topo_polygon_feature(&mut output, next_fid, dissolved.poly, Vec::new());
                next_fid += 1;
            }
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for SymmetricalDifferenceTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "symmetrical_difference",
            display_name: "Symmetrical Difference",
            summary: "Computes non-overlapping polygon regions from input and overlay layers.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: overlay_tool_params(),
        }
    }

    fn manifest(&self) -> ToolManifest {
        overlay_manifest(
            "symmetrical_difference",
            "Symmetrical Difference",
            "Computes non-overlapping polygon regions from input and overlay layers.",
            "symmetrical_difference_basic",
            "symmetrical_difference.shp",
        )
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        validate_overlay_common_args(args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (input, overlay) = load_overlay_layers_aligned(args)?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let epsilon = parse_overlay_snap_tolerance(args);

        let input_pieces = collect_overlay_polygon_pieces(&input)?;
        let overlay_pieces = collect_overlay_polygon_pieces(&overlay)?;
        let (merged_schema, input_mapping, overlay_mapping) =
            build_merged_overlay_schema(&input, &overlay);
        let merged_len = merged_schema.len();

        let mut input_polys = Vec::with_capacity(input_pieces.len());
        for piece in &input_pieces {
            input_polys.push(piece.polygon.clone());
        }
        let dissolved_input = polygon_unary_dissolve(&input_polys, epsilon);

        let mut overlay_polys = Vec::with_capacity(overlay_pieces.len());
        for piece in &overlay_pieces {
            overlay_polys.push(piece.polygon.clone());
        }
        let dissolved_overlay = polygon_unary_dissolve(&overlay_polys, epsilon);

        let mut output = wbvector::Layer::new(format!("{}_sym_diff", input.name));
        output.schema = merged_schema;
        output.crs = input.crs.clone();
        output.geom_type = Some(wbvector::GeometryType::Polygon);

        let total = (input_pieces.len() + overlay_pieces.len()).max(1);
        let mut done = 0usize;
        let mut next_fid = 1u64;

        for input_piece in &input_pieces {
            let mut remainder = vec![input_piece.polygon.clone()];
            for overlay_group in &dissolved_overlay {
                let mut next = Vec::<TopoPolygon>::new();
                for poly in &remainder {
                    next.extend(polygon_difference(poly, &overlay_group.poly, epsilon));
                }
                remainder = next;
                if remainder.is_empty() {
                    break;
                }
            }
            for poly in remainder {
                let attrs = merged_overlay_attributes(
                    Some(&input_piece.attributes),
                    None,
                    &input_mapping,
                    &overlay_mapping,
                    merged_len,
                );
                push_topo_polygon_feature(
                    &mut output,
                    next_fid,
                    poly,
                    attrs,
                );
                next_fid += 1;
            }
            done += 1;
            ctx.progress.progress(done as f64 / total as f64);
        }

        for overlay_piece in &overlay_pieces {
            let mut remainder = vec![overlay_piece.polygon.clone()];
            for input_group in &dissolved_input {
                let mut next = Vec::<TopoPolygon>::new();
                for poly in &remainder {
                    next.extend(polygon_difference(poly, &input_group.poly, epsilon));
                }
                remainder = next;
                if remainder.is_empty() {
                    break;
                }
            }
            for poly in remainder {
                let attrs = merged_overlay_attributes(
                    None,
                    Some(&overlay_piece.attributes),
                    &input_mapping,
                    &overlay_mapping,
                    merged_len,
                );
                push_topo_polygon_feature(
                    &mut output,
                    next_fid,
                    poly,
                    attrs,
                );
                next_fid += 1;
            }
            done += 1;
            ctx.progress.progress(done as f64 / total as f64);
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for UnionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "union",
            display_name: "Union",
            summary: "Dissolves combined input and overlay polygons into a unified polygon coverage.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: overlay_tool_params(),
        }
    }

    fn manifest(&self) -> ToolManifest {
        overlay_manifest(
            "union",
            "Union",
            "Dissolves combined input and overlay polygons into a unified polygon coverage.",
            "union_basic",
            "union.shp",
        )
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        validate_overlay_common_args(args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (input, overlay) = load_overlay_layers_aligned(args)?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let epsilon = parse_overlay_snap_tolerance(args);

        let input_pieces = collect_overlay_polygon_pieces(&input)?;
        let overlay_pieces = collect_overlay_polygon_pieces(&overlay)?;
        let (merged_schema, input_mapping, overlay_mapping) =
            build_merged_overlay_schema(&input, &overlay);
        let merged_len = merged_schema.len();

        let mut input_polys = Vec::with_capacity(input_pieces.len());
        for piece in &input_pieces {
            input_polys.push(piece.polygon.clone());
        }
        let dissolved_input = polygon_unary_dissolve(&input_polys, epsilon);

        let mut overlay_polys = Vec::with_capacity(overlay_pieces.len());
        for piece in &overlay_pieces {
            overlay_polys.push(piece.polygon.clone());
        }
        let dissolved_overlay = polygon_unary_dissolve(&overlay_polys, epsilon);

        let mut output = wbvector::Layer::new(format!("{}_union", input.name));
        output.schema = merged_schema;
        output.crs = input.crs.clone();
        output.geom_type = Some(wbvector::GeometryType::Polygon);

        let total = (input_pieces.len() + overlay_pieces.len()).max(1);
        let mut done = 0usize;
        let mut next_fid = 1u64;

        for input_piece in &input_pieces {
            let attrs_input_only = merged_overlay_attributes(
                Some(&input_piece.attributes),
                None,
                &input_mapping,
                &overlay_mapping,
                merged_len,
            );

            for overlay_piece in &overlay_pieces {
                let intersections =
                    polygon_intersection(&input_piece.polygon, &overlay_piece.polygon, epsilon);
                for poly in intersections {
                    let attrs_both = merged_overlay_attributes(
                        Some(&input_piece.attributes),
                        Some(&overlay_piece.attributes),
                        &input_mapping,
                        &overlay_mapping,
                        merged_len,
                    );
                    push_topo_polygon_feature(&mut output, next_fid, poly, attrs_both);
                    next_fid += 1;
                }
            }

            let mut remainder = vec![input_piece.polygon.clone()];
            for overlay_group in &dissolved_overlay {
                let mut next = Vec::<TopoPolygon>::new();
                for poly in &remainder {
                    next.extend(polygon_difference(poly, &overlay_group.poly, epsilon));
                }
                remainder = next;
                if remainder.is_empty() {
                    break;
                }
            }

            for poly in remainder {
                push_topo_polygon_feature(&mut output, next_fid, poly, attrs_input_only.clone());
                next_fid += 1;
            }

            done += 1;
            ctx.progress.progress(done as f64 / total as f64);
        }

        for overlay_piece in &overlay_pieces {
            let attrs_overlay_only = merged_overlay_attributes(
                None,
                Some(&overlay_piece.attributes),
                &input_mapping,
                &overlay_mapping,
                merged_len,
            );
            let mut remainder = vec![overlay_piece.polygon.clone()];
            for input_group in &dissolved_input {
                let mut next = Vec::<TopoPolygon>::new();
                for poly in &remainder {
                    next.extend(polygon_difference(poly, &input_group.poly, epsilon));
                }
                remainder = next;
                if remainder.is_empty() {
                    break;
                }
            }

            for poly in remainder {
                push_topo_polygon_feature(&mut output, next_fid, poly, attrs_overlay_only.clone());
                next_fid += 1;
            }

            done += 1;
            ctx.progress.progress(done as f64 / total as f64);
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

#[derive(Clone, Copy)]
enum MbbCriterion {
    Area,
    Perimeter,
    Length,
    Width,
}

impl MbbCriterion {
    fn parse(value: Option<&str>) -> Self {
        let key = value.unwrap_or("area").trim().to_ascii_lowercase();
        if key.contains("len") {
            Self::Length
        } else if key.contains("wi") {
            Self::Width
        } else if key.contains("per") {
            Self::Perimeter
        } else {
            Self::Area
        }
    }

    fn score(self, axis1: f64, axis2: f64) -> f64 {
        match self {
            Self::Area => axis1 * axis2,
            Self::Perimeter => 2.0 * axis1 + 2.0 * axis2,
            Self::Length => axis1.max(axis2),
            Self::Width => axis1.min(axis2),
        }
    }
}

fn collect_all_coords_from_geometry(geometry: &wbvector::Geometry, out: &mut Vec<wbvector::Coord>) {
    match geometry {
        wbvector::Geometry::Point(coord) => out.push(coord.clone()),
        wbvector::Geometry::MultiPoint(coords) => out.extend(coords.iter().cloned()),
        wbvector::Geometry::LineString(coords) => out.extend(coords.iter().cloned()),
        wbvector::Geometry::MultiLineString(lines) => {
            for line in lines {
                out.extend(line.iter().cloned());
            }
        }
        wbvector::Geometry::Polygon { exterior, interiors } => {
            out.extend(exterior.coords().iter().cloned());
            for ring in interiors {
                out.extend(ring.coords().iter().cloned());
            }
        }
        wbvector::Geometry::MultiPolygon(parts) => {
            for (exterior, interiors) in parts {
                out.extend(exterior.coords().iter().cloned());
                for ring in interiors {
                    out.extend(ring.coords().iter().cloned());
                }
            }
        }
        wbvector::Geometry::GeometryCollection(parts) => {
            for part in parts {
                collect_all_coords_from_geometry(part, out);
            }
        }
    }
}

fn bbox_ring_for_coords(coords: &[wbvector::Coord], eps: f64) -> Option<Vec<wbvector::Coord>> {
    if coords.is_empty() {
        return None;
    }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for coord in coords {
        min_x = min_x.min(coord.x);
        min_y = min_y.min(coord.y);
        max_x = max_x.max(coord.x);
        max_y = max_y.max(coord.y);
    }

    let mut width = max_x - min_x;
    let mut height = max_y - min_y;
    let e = eps.max(1.0e-9);
    if width.abs() <= e {
        width = e;
        min_x -= width * 0.5;
        max_x += width * 0.5;
    }
    if height.abs() <= e {
        height = e;
        min_y -= height * 0.5;
        max_y += height * 0.5;
    }

    Some(vec![
        wbvector::Coord::xy(min_x, max_y),
        wbvector::Coord::xy(max_x, max_y),
        wbvector::Coord::xy(max_x, min_y),
        wbvector::Coord::xy(min_x, min_y),
        wbvector::Coord::xy(min_x, max_y),
    ])
}

fn convex_hull_polygon_for_coords(coords: &[wbvector::Coord], eps: f64) -> Option<wbvector::Geometry> {
    if coords.is_empty() {
        return None;
    }
    let topo_coords: Vec<TopoCoord> = coords.iter().map(to_topo_coord).collect();
    match convex_hull(&topo_coords, eps) {
        TopoGeometry::Polygon(poly) => Some(wbvector::Geometry::Polygon {
            exterior: to_wb_ring(&poly.exterior),
            interiors: to_wb_rings(&poly.holes),
        }),
        TopoGeometry::LineString(_) | TopoGeometry::Point(_) => {
            let ring = bbox_ring_for_coords(coords, eps)?;
            Some(wbvector::Geometry::Polygon {
                exterior: wbvector::Ring::new(ring),
                interiors: Vec::new(),
            })
        }
        _ => None,
    }
}

fn minimum_bounding_box_coords(
    coords: &[wbvector::Coord],
    criterion: MbbCriterion,
    eps: f64,
) -> Option<Vec<wbvector::Coord>> {
    if coords.len() < 2 {
        return bbox_ring_for_coords(coords, eps);
    }

    let topo_coords: Vec<TopoCoord> = coords.iter().map(to_topo_coord).collect();
    let hull_coords = match convex_hull(&topo_coords, eps) {
        TopoGeometry::Polygon(poly) => {
            let mut v: Vec<wbvector::Coord> = poly.exterior.coords.iter().map(to_wb_coord).collect();
            if v.len() > 1 && coord_eq_eps(&v[0], &v[v.len() - 1], eps) {
                v.pop();
            }
            v
        }
        TopoGeometry::LineString(line) => line.coords.iter().map(to_wb_coord).collect(),
        TopoGeometry::Point(point) => vec![to_wb_coord(&point)],
        _ => Vec::new(),
    };

    if hull_coords.len() < 2 {
        return bbox_ring_for_coords(coords, eps);
    }

    let mut east = f64::NEG_INFINITY;
    let mut west = f64::INFINITY;
    let mut north = f64::NEG_INFINITY;
    let mut south = f64::INFINITY;
    for point in &hull_coords {
        east = east.max(point.x);
        west = west.min(point.x);
        north = north.max(point.y);
        south = south.min(point.y);
    }

    let midx = west + (east - west) / 2.0;
    let midy = south + (north - south) / 2.0;
    let right_angle = std::f64::consts::PI / 2.0;

    let mut x_axis = f64::INFINITY;
    let mut y_axis = f64::INFINITY;
    let mut slope = 0.0;
    let mut centre_x = 0.0;
    let mut centre_y = 0.0;
    let mut best = f64::INFINITY;

    for i in 0..hull_coords.len() {
        let j = (i + 1) % hull_coords.len();
        let dx = hull_coords[j].x - hull_coords[i].x;
        let dy = hull_coords[j].y - hull_coords[i].y;
        if (dx * dx + dy * dy).sqrt() <= eps.max(1.0e-12) {
            continue;
        }

        let psi = -(dy.atan2(dx));
        east = f64::NEG_INFINITY;
        west = f64::INFINITY;
        north = f64::NEG_INFINITY;
        south = f64::INFINITY;

        for point in &hull_coords {
            let x = point.x - midx;
            let y = point.y - midy;
            let x_rot = x * psi.cos() - y * psi.sin();
            let y_rot = x * psi.sin() + y * psi.cos();
            east = east.max(x_rot);
            west = west.min(x_rot);
            north = north.max(y_rot);
            south = south.min(y_rot);
        }

        let new_x_axis = (east - west).abs();
        let new_y_axis = (north - south).abs();
        let score = criterion.score(new_x_axis, new_y_axis);
        if score < best {
            best = score;
            x_axis = new_x_axis;
            y_axis = new_y_axis;
            slope = if x_axis > y_axis {
                -psi
            } else {
                -(right_angle + psi)
            };

            let x = west + x_axis / 2.0;
            let y = north - y_axis / 2.0;
            centre_x = midx + x * (-psi).cos() - y * (-psi).sin();
            centre_y = midy + x * (-psi).sin() + y * (-psi).cos();
        }
    }

    if !best.is_finite() {
        return bbox_ring_for_coords(coords, eps);
    }

    let long_axis = x_axis.max(y_axis);
    let short_axis = x_axis.min(y_axis);
    Some(vec![
        wbvector::Coord::xy(
            centre_x + long_axis / 2.0 * slope.cos() + short_axis / 2.0 * (right_angle + slope).cos(),
            centre_y + long_axis / 2.0 * slope.sin() + short_axis / 2.0 * (right_angle + slope).sin(),
        ),
        wbvector::Coord::xy(
            centre_x + long_axis / 2.0 * slope.cos() - short_axis / 2.0 * (right_angle + slope).cos(),
            centre_y + long_axis / 2.0 * slope.sin() - short_axis / 2.0 * (right_angle + slope).sin(),
        ),
        wbvector::Coord::xy(
            centre_x - long_axis / 2.0 * slope.cos() - short_axis / 2.0 * (right_angle + slope).cos(),
            centre_y - long_axis / 2.0 * slope.sin() - short_axis / 2.0 * (right_angle + slope).sin(),
        ),
        wbvector::Coord::xy(
            centre_x - long_axis / 2.0 * slope.cos() + short_axis / 2.0 * (right_angle + slope).cos(),
            centre_y - long_axis / 2.0 * slope.sin() + short_axis / 2.0 * (right_angle + slope).sin(),
        ),
        wbvector::Coord::xy(
            centre_x + long_axis / 2.0 * slope.cos() + short_axis / 2.0 * (right_angle + slope).cos(),
            centre_y + long_axis / 2.0 * slope.sin() + short_axis / 2.0 * (right_angle + slope).sin(),
        ),
    ])
}

fn parse_reclass_values_arg(args: &ToolArgs, assign_mode: bool) -> Result<Vec<Vec<f64>>, ToolError> {
    let value = args
        .get("reclass_values")
        .ok_or_else(|| ToolError::Validation("parameter 'reclass_values' is required".to_string()))?;

    if let Some(s) = value.as_str() {
        let nums = s
            .split([',', ';'])
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(|t| t.parse::<f64>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ToolError::Validation("parameter 'reclass_values' string contains non-numeric values".to_string()))?;

        let chunk = if assign_mode { 2 } else { 3 };
        if nums.is_empty() || nums.len() % chunk != 0 {
            return Err(ToolError::Validation(if assign_mode {
                "reclass_values must contain value pairs [new_value, old_value]".to_string()
            } else {
                "reclass_values must contain value triplets [new_value, from_value, to_less_than]".to_string()
            }));
        }

        let mut out = Vec::new();
        for part in nums.chunks(chunk) {
            out.push(part.to_vec());
        }
        return Ok(out);
    }

    if let Some(arr) = value.as_array() {
        let chunk = if assign_mode { 2 } else { 3 };
        let mut out = Vec::<Vec<f64>>::new();
        for (idx, row) in arr.iter().enumerate() {
            let Some(inner) = row.as_array() else {
                return Err(ToolError::Validation(format!(
                    "reclass_values row {} must be an array",
                    idx
                )));
            };
            if inner.len() != chunk {
                return Err(ToolError::Validation(if assign_mode {
                    format!("reclass_values row {} must have exactly 2 numbers", idx)
                } else {
                    format!("reclass_values row {} must have exactly 3 numbers", idx)
                }));
            }
            let mut row_vals = Vec::with_capacity(chunk);
            for val in inner {
                let Some(num) = val.as_f64() else {
                    return Err(ToolError::Validation(format!(
                        "reclass_values row {} contains a non-numeric value",
                        idx
                    )));
                };
                row_vals.push(num);
            }
            out.push(row_vals);
        }
        if out.is_empty() {
            return Err(ToolError::Validation(
                "reclass_values must contain at least one row".to_string(),
            ));
        }
        return Ok(out);
    }

    Err(ToolError::Validation(
        "parameter 'reclass_values' must be a string or an array of arrays".to_string(),
    ))
}

impl Tool for MinimumConvexHullTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "minimum_convex_hull",
            display_name: "Minimum Convex Hull",
            summary: "Creates convex hull polygons around individual features or the full input layer.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input vector layer.",
                    required: true,
                },
                ToolParamSpec {
                    name: "individual_feature_hulls",
                    description: "If true, outputs one hull per input feature; otherwise outputs one hull for the entire layer.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output polygon vector path.",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.shp"));
        defaults.insert("individual_feature_hulls".to_string(), json!(true));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("minimum_convex_hull.shp"));

        ToolManifest {
            id: "minimum_convex_hull".to_string(),
            display_name: "Minimum Convex Hull".to_string(),
            summary: "Creates convex hull polygons around individual features or the full input layer.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input vector layer.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "individual_feature_hulls".to_string(),
                    description: "If true, outputs one hull per input feature; otherwise outputs one hull for the entire layer.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output polygon vector path.".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "minimum_convex_hull_basic".to_string(),
                description: "Creates convex hull polygons for vector features.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "hull".to_string(),
                "shape".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let mut individual = args
            .get("individual_feature_hulls")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        if input.geom_type == Some(wbvector::GeometryType::Point) {
            individual = false;
        }
        let eps = parse_overlay_snap_tolerance(args).max(1.0e-12);

        let mut output = wbvector::Layer::new(format!("{}_minimum_convex_hull", input.name));
        output.geom_type = Some(wbvector::GeometryType::Polygon);
        output.crs = input.crs.clone();

        if individual {
            output.schema = input.schema.clone();
            let total = input.features.len().max(1);
            let completed = AtomicUsize::new(0);
            let results: Vec<Option<wbvector::Feature>> = input
                .features
                .par_iter()
                .map(|feature| {
                    let out = if let Some(geometry) = &feature.geometry {
                        let mut coords = Vec::<wbvector::Coord>::new();
                        collect_all_coords_from_geometry(geometry, &mut coords);
                        convex_hull_polygon_for_coords(&coords, eps).map(|hull_geom| {
                            wbvector::Feature {
                                fid: 0,
                                geometry: Some(hull_geom),
                                attributes: feature.attributes.clone(),
                            }
                        })
                    } else {
                        None
                    };
                    let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    ctx.progress.progress(done as f64 / total as f64);
                    out
                })
                .collect();
            let mut next_fid = 1u64;
            for feat_opt in results {
                if let Some(mut feat) = feat_opt {
                    feat.fid = next_fid;
                    output.push(feat);
                    next_fid += 1;
                }
            }
        } else {
            output.schema = wbvector::Schema::new();
            output
                .schema
                .add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));

            let mut all_coords = Vec::<wbvector::Coord>::new();
            for feature in &input.features {
                if let Some(geometry) = &feature.geometry {
                    collect_all_coords_from_geometry(geometry, &mut all_coords);
                }
            }

            let hull_geom = convex_hull_polygon_for_coords(&all_coords, eps).ok_or_else(|| {
                ToolError::Validation("input layer does not contain coordinates for hull generation".to_string())
            })?;

            output.push(wbvector::Feature {
                fid: 1,
                geometry: Some(hull_geom),
                attributes: vec![wbvector::FieldValue::Integer(1)],
            });
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for MinimumBoundingBoxTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "minimum_bounding_box",
            display_name: "Minimum Bounding Box",
            summary: "Calculates oriented minimum bounding boxes around individual features or the entire layer.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input vector layer.",
                    required: true,
                },
                ToolParamSpec {
                    name: "min_criteria",
                    description: "Minimization criterion: area, perimeter, length, or width. Defaults to area.",
                    required: false,
                },
                ToolParamSpec {
                    name: "individual_feature_hulls",
                    description: "If true, outputs one box per feature; otherwise outputs one box for the full layer.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output polygon vector path.",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.shp"));
        defaults.insert("min_criteria".to_string(), json!("area"));
        defaults.insert("individual_feature_hulls".to_string(), json!(true));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("minimum_bounding_box.shp"));

        ToolManifest {
            id: "minimum_bounding_box".to_string(),
            display_name: "Minimum Bounding Box".to_string(),
            summary: "Calculates oriented minimum bounding boxes around individual features or the entire layer.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input vector layer.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "min_criteria".to_string(),
                    description: "Minimization criterion: area, perimeter, length, or width. Defaults to area.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "individual_feature_hulls".to_string(),
                    description: "If true, outputs one box per feature; otherwise outputs one box for the full layer.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output polygon vector path.".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "minimum_bounding_box_basic".to_string(),
                description: "Calculates oriented minimum bounding boxes for vector features.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "bounding-box".to_string(),
                "shape".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let criterion = MbbCriterion::parse(args.get("min_criteria").and_then(|v| v.as_str()));
        let mut individual = args
            .get("individual_feature_hulls")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        if input.geom_type == Some(wbvector::GeometryType::Point) {
            individual = false;
        }
        let eps = parse_overlay_snap_tolerance(args).max(1.0e-12);

        let mut output = wbvector::Layer::new(format!("{}_minimum_bounding_box", input.name));
        output.geom_type = Some(wbvector::GeometryType::Polygon);
        output.crs = input.crs.clone();

        if individual {
            output.schema = input.schema.clone();
            let total = input.features.len().max(1);
            let completed = AtomicUsize::new(0);
            let results: Vec<Option<wbvector::Feature>> = input
                .features
                .par_iter()
                .map(|feature| {
                    let out = if let Some(geometry) = &feature.geometry {
                        let mut coords = Vec::<wbvector::Coord>::new();
                        collect_all_coords_from_geometry(geometry, &mut coords);
                        minimum_bounding_box_coords(&coords, criterion, eps).map(|ring| {
                            wbvector::Feature {
                                fid: 0,
                                geometry: Some(wbvector::Geometry::Polygon {
                                    exterior: wbvector::Ring::new(ring),
                                    interiors: Vec::new(),
                                }),
                                attributes: feature.attributes.clone(),
                            }
                        })
                    } else {
                        None
                    };
                    let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    ctx.progress.progress(done as f64 / total as f64);
                    out
                })
                .collect();
            let mut next_fid = 1u64;
            for feat_opt in results {
                if let Some(mut feat) = feat_opt {
                    feat.fid = next_fid;
                    output.push(feat);
                    next_fid += 1;
                }
            }
        } else {
            output.schema = wbvector::Schema::new();
            output
                .schema
                .add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));

            let mut all_coords = Vec::<wbvector::Coord>::new();
            for feature in &input.features {
                if let Some(geometry) = &feature.geometry {
                    collect_all_coords_from_geometry(geometry, &mut all_coords);
                }
            }

            let ring = minimum_bounding_box_coords(&all_coords, criterion, eps).ok_or_else(|| {
                ToolError::Validation("input layer does not contain coordinates for bounding box generation".to_string())
            })?;
            output.push(wbvector::Feature {
                fid: 1,
                geometry: Some(wbvector::Geometry::Polygon {
                    exterior: wbvector::Ring::new(ring),
                    interiors: Vec::new(),
                }),
                attributes: vec![wbvector::FieldValue::Integer(1)],
            });
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for MinimumBoundingCircleTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "minimum_bounding_circle",
            display_name: "Minimum Bounding Circle",
            summary: "Calculates minimum enclosing circles around individual features or the entire layer.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input vector layer.",
                    required: true,
                },
                ToolParamSpec {
                    name: "individual_feature_hulls",
                    description: "If true, outputs one circle per feature; otherwise outputs one circle for the full layer.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output polygon vector path.",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.shp"));
        defaults.insert("individual_feature_hulls".to_string(), json!(true));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("minimum_bounding_circle.shp"));

        ToolManifest {
            id: "minimum_bounding_circle".to_string(),
            display_name: "Minimum Bounding Circle".to_string(),
            summary: "Calculates minimum enclosing circles around individual features or the entire layer.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input vector layer.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "individual_feature_hulls".to_string(),
                    description: "If true, outputs one circle per feature; otherwise outputs one circle for the full layer.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output polygon vector path.".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "minimum_bounding_circle_basic".to_string(),
                description: "Calculates minimum enclosing circles for vector features.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "bounding-circle".to_string(),
                "shape".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let mut individual = args
            .get("individual_feature_hulls")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        if input.geom_type == Some(wbvector::GeometryType::Point) {
            individual = false;
        }

        let mut output = wbvector::Layer::new(format!("{}_minimum_bounding_circle", input.name));
        output.geom_type = Some(wbvector::GeometryType::Polygon);
        output.crs = input.crs.clone();

        if individual {
            output.schema = input.schema.clone();
            let total = input.features.len().max(1);
            let completed = AtomicUsize::new(0);
            let results: Vec<Option<wbvector::Feature>> = input
                .features
                .par_iter()
                .map(|feature| {
                    let out = if let Some(geometry) = &feature.geometry {
                        let mut coords = Vec::<wbvector::Coord>::new();
                        collect_all_coords_from_geometry(geometry, &mut coords);
                        smallest_enclosing_circle(&coords).and_then(|(cx, cy, radius)| {
                            let ring = circle_ring_coords(cx, cy, radius, 128);
                            if ring.len() >= 4 {
                                Some(wbvector::Feature {
                                    fid: 0,
                                    geometry: Some(wbvector::Geometry::Polygon {
                                        exterior: wbvector::Ring::new(ring),
                                        interiors: Vec::new(),
                                    }),
                                    attributes: feature.attributes.clone(),
                                })
                            } else {
                                None
                            }
                        })
                    } else {
                        None
                    };
                    let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    ctx.progress.progress(done as f64 / total as f64);
                    out
                })
                .collect();
            let mut next_fid = 1u64;
            for feat_opt in results {
                if let Some(mut feat) = feat_opt {
                    feat.fid = next_fid;
                    output.push(feat);
                    next_fid += 1;
                }
            }
        } else {
            output.schema = wbvector::Schema::new();
            output
                .schema
                .add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));

            let mut all_coords = Vec::<wbvector::Coord>::new();
            for feature in &input.features {
                if let Some(geometry) = &feature.geometry {
                    collect_all_coords_from_geometry(geometry, &mut all_coords);
                }
            }

            let (cx, cy, radius) = smallest_enclosing_circle(&all_coords).ok_or_else(|| {
                ToolError::Validation("input layer does not contain coordinates for bounding circle generation".to_string())
            })?;
            let ring = circle_ring_coords(cx, cy, radius, 128);
            output.push(wbvector::Feature {
                fid: 1,
                geometry: Some(wbvector::Geometry::Polygon {
                    exterior: wbvector::Ring::new(ring),
                    interiors: Vec::new(),
                }),
                attributes: vec![wbvector::FieldValue::Integer(1)],
            });
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for MinimumBoundingEnvelopeTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "minimum_bounding_envelope",
            display_name: "Minimum Bounding Envelope",
            summary: "Calculates axis-aligned minimum bounding envelopes around individual features or the entire layer.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input vector layer.",
                    required: true,
                },
                ToolParamSpec {
                    name: "individual_feature_hulls",
                    description: "If true, outputs one envelope per feature; otherwise outputs one envelope for the full layer.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output polygon vector path.",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.shp"));
        defaults.insert("individual_feature_hulls".to_string(), json!(true));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("minimum_bounding_envelope.shp"));

        ToolManifest {
            id: "minimum_bounding_envelope".to_string(),
            display_name: "Minimum Bounding Envelope".to_string(),
            summary: "Calculates axis-aligned minimum bounding envelopes around individual features or the entire layer.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input vector layer.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "individual_feature_hulls".to_string(),
                    description: "If true, outputs one envelope per feature; otherwise outputs one envelope for the full layer.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output polygon vector path.".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "minimum_bounding_envelope_basic".to_string(),
                description: "Calculates axis-aligned envelopes for vector features.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "bounding-box".to_string(),
                "shape".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let mut individual = args
            .get("individual_feature_hulls")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        if input.geom_type == Some(wbvector::GeometryType::Point) {
            individual = false;
        }

        let mut output = wbvector::Layer::new(format!("{}_minimum_bounding_envelope", input.name));
        output.geom_type = Some(wbvector::GeometryType::Polygon);
        output.crs = input.crs.clone();

        if individual {
            output.schema = input.schema.clone();
            let total = input.features.len().max(1);
            let completed = AtomicUsize::new(0);
            let results: Vec<Option<wbvector::Feature>> = input
                .features
                .par_iter()
                .map(|feature| {
                    let out = if let Some(geometry) = &feature.geometry {
                        get_bounding_box(geometry).map(|(min_x, min_y, max_x, max_y)| {
                            let ring = vec![
                                wbvector::Coord::xy(min_x, min_y),
                                wbvector::Coord::xy(max_x, min_y),
                                wbvector::Coord::xy(max_x, max_y),
                                wbvector::Coord::xy(min_x, max_y),
                                wbvector::Coord::xy(min_x, min_y),
                            ];
                            wbvector::Feature {
                                fid: 0,
                                geometry: Some(wbvector::Geometry::Polygon {
                                    exterior: wbvector::Ring::new(ring),
                                    interiors: Vec::new(),
                                }),
                                attributes: feature.attributes.clone(),
                            }
                        })
                    } else {
                        None
                    };
                    let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    ctx.progress.progress(done as f64 / total as f64);
                    out
                })
                .collect();
            let mut next_fid = 1u64;
            for feat_opt in results {
                if let Some(mut feat) = feat_opt {
                    feat.fid = next_fid;
                    output.push(feat);
                    next_fid += 1;
                }
            }
        } else {
            output.schema = wbvector::Schema::new();
            output
                .schema
                .add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));

            let mut min_x = f64::INFINITY;
            let mut min_y = f64::INFINITY;
            let mut max_x = f64::NEG_INFINITY;
            let mut max_y = f64::NEG_INFINITY;
            let mut found = false;
            for feature in &input.features {
                if let Some(geometry) = &feature.geometry {
                    if let Some((fx0, fy0, fx1, fy1)) = get_bounding_box(geometry) {
                        min_x = min_x.min(fx0);
                        min_y = min_y.min(fy0);
                        max_x = max_x.max(fx1);
                        max_y = max_y.max(fy1);
                        found = true;
                    }
                }
            }
            if !found {
                return Err(ToolError::Validation(
                    "input layer does not contain coordinates for bounding envelope generation".to_string(),
                ));
            }

            let ring = vec![
                wbvector::Coord::xy(min_x, min_y),
                wbvector::Coord::xy(max_x, min_y),
                wbvector::Coord::xy(max_x, max_y),
                wbvector::Coord::xy(min_x, max_y),
                wbvector::Coord::xy(min_x, min_y),
            ];
            output.push(wbvector::Feature {
                fid: 1,
                geometry: Some(wbvector::Geometry::Polygon {
                    exterior: wbvector::Ring::new(ring),
                    interiors: Vec::new(),
                }),
                attributes: vec![wbvector::FieldValue::Integer(1)],
            });
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for FilterRasterFeaturesByAreaTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "filter_raster_features_by_area",
            display_name: "Filter Raster Features By Area",
            summary: "Removes integer-labelled raster features smaller than a cell-count threshold.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input raster containing integer class labels.",
                    required: true,
                },
                ToolParamSpec {
                    name: "threshold",
                    description: "Minimum feature size in cells to retain.",
                    required: true,
                },
                ToolParamSpec {
                    name: "zero_background",
                    description: "If true, removed features are assigned 0; otherwise removed features are assigned NoData.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output raster path. If omitted, result is kept in memory.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("threshold".to_string(), json!(10));
        defaults.insert("zero_background".to_string(), json!(false));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("filter_raster_features_by_area.tif"));

        ToolManifest {
            id: "filter_raster_features_by_area".to_string(),
            display_name: "Filter Raster Features By Area".to_string(),
            summary: "Removes integer-labelled raster features smaller than a cell-count threshold.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input raster containing integer class labels.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "threshold".to_string(),
                    description: "Minimum feature size in cells to retain.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "zero_background".to_string(),
                    description: "If true, removed features are assigned 0; otherwise removed features are assigned NoData.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output raster path. If omitted, result is kept in memory.".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "filter_raster_features_by_area_basic".to_string(),
                description: "Filters small labelled raster features by size threshold.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "raster".to_string(),
                "gis".to_string(),
                "filter".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "input")?;
        let threshold = args
            .get("threshold")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| ToolError::Validation("parameter 'threshold' is required".to_string()))?;
        if threshold == 0 {
            return Err(ToolError::Validation(
                "threshold must be greater than 0".to_string(),
            ));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_required_raster_arg(args, "input")?;
        let threshold = args
            .get("threshold")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| ToolError::Validation("parameter 'threshold' is required".to_string()))? as usize;
        let zero_background = args
            .get("zero_background")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let output_path = parse_optional_output_path(args, "output")?;

        let mut counts = HashMap::<i64, usize>::new();
        for idx in 0..input.data.len() {
            let value = input.data.get_f64(idx);
            if input.is_nodata(value) {
                continue;
            }
            let key = value.round() as i64;
            *counts.entry(key).or_insert(0) += 1;
            if idx % 8192 == 0 {
                ctx.progress.progress(idx as f64 / input.data.len().max(1) as f64);
            }
        }

        let mut output = build_output_like_raster(&input, input.data_type);
        output.nodata = input.nodata;
        let background = if zero_background { 0.0 } else { input.nodata };
        for idx in 0..input.data.len() {
            let value = input.data.get_f64(idx);
            let out_val = if input.is_nodata(value) {
                input.nodata
            } else {
                let key = value.round() as i64;
                if counts.get(&key).copied().unwrap_or(0) >= threshold {
                    value
                } else {
                    background
                }
            };
            output.data.set_f64(idx, out_val);
            if idx % 8192 == 0 {
                ctx.progress.progress(idx as f64 / input.data.len().max(1) as f64);
            }
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        Ok(GisOverlayCore::build_result(locator))
    }
}

impl Tool for ReclassTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "reclass",
            display_name: "Reclass",
            summary: "Reclassifies raster values using either ranges or exact assignment pairs.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input raster.",
                    required: true,
                },
                ToolParamSpec {
                    name: "reclass_values",
                    description: "Reclassification values as rows of [new, from, to_less_than] or [new, old] when assign_mode=true.",
                    required: true,
                },
                ToolParamSpec {
                    name: "assign_mode",
                    description: "If true, use exact assignment mode with [new_value, old_value] pairs.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output raster path. If omitted, result is kept in memory.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("reclass_values".to_string(), json!([[1.0, 0.0, 10.0], [2.0, 10.0, 20.0]]));
        defaults.insert("assign_mode".to_string(), json!(false));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("reclass.tif"));

        ToolManifest {
            id: "reclass".to_string(),
            display_name: "Reclass".to_string(),
            summary: "Reclassifies raster values using either ranges or exact assignment pairs.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input raster.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "reclass_values".to_string(),
                    description: "Reclassification values as rows of [new, from, to_less_than] or [new, old] when assign_mode=true.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "assign_mode".to_string(),
                    description: "If true, use exact assignment mode with [new_value, old_value] pairs.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output raster path. If omitted, result is kept in memory.".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "reclass_basic".to_string(),
                description: "Reclassifies raster values by range.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "raster".to_string(),
                "gis".to_string(),
                "reclass".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "input")?;
        let assign_mode = args.get("assign_mode").and_then(|v| v.as_bool()).unwrap_or(false);
        let _ = parse_reclass_values_arg(args, assign_mode)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_required_raster_arg(args, "input")?;
        let assign_mode = args.get("assign_mode").and_then(|v| v.as_bool()).unwrap_or(false);
        let rules = parse_reclass_values_arg(args, assign_mode)?;
        let output_path = parse_optional_output_path(args, "output")?;

        let mut output = build_output_like_raster(&input, DataType::F64);
        output.nodata = input.nodata;
        for idx in 0..output.data.len() {
            output.data.set_f64(idx, input.nodata);
        }

        ctx.progress.info("running reclass");
        if assign_mode {
            let multiplier = 10000.0;
            let mut map = HashMap::<i64, f64>::new();
            for rule in &rules {
                map.insert((rule[1] * multiplier).round() as i64, rule[0]);
            }
            for idx in 0..input.data.len() {
                let mut value = input.data.get_f64(idx);
                if !input.is_nodata(value) {
                    if let Some(mapped) = map.get(&((value * multiplier).round() as i64)) {
                        value = *mapped;
                    }
                }
                output.data.set_f64(idx, value);
                if idx % 8192 == 0 {
                    ctx.progress.progress(idx as f64 / input.data.len().max(1) as f64);
                }
            }
        } else {
            for idx in 0..input.data.len() {
                let mut value = input.data.get_f64(idx);
                if !input.is_nodata(value) {
                    for rule in &rules {
                        if value >= rule[1] && value < rule[2] {
                            value = rule[0];
                            break;
                        }
                    }
                }
                output.data.set_f64(idx, value);
                if idx % 8192 == 0 {
                    ctx.progress.progress(idx as f64 / input.data.len().max(1) as f64);
                }
            }
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        Ok(GisOverlayCore::build_result(locator))
    }
}

impl Tool for ReclassEqualIntervalTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "reclass_equal_interval",
            display_name: "Reclass Equal Interval",
            summary: "Reclassifies raster values into equal-width intervals over an optional value range.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input raster.",
                    required: true,
                },
                ToolParamSpec {
                    name: "interval_size",
                    description: "Equal-interval bin width.",
                    required: true,
                },
                ToolParamSpec {
                    name: "start_value",
                    description: "Optional lower bound for reclassification; defaults to raster minimum.",
                    required: false,
                },
                ToolParamSpec {
                    name: "end_value",
                    description: "Optional upper bound for reclassification; defaults to raster maximum.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output raster path. If omitted, result is kept in memory.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("interval_size".to_string(), json!(5.0));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("reclass_equal_interval.tif"));

        ToolManifest {
            id: "reclass_equal_interval".to_string(),
            display_name: "Reclass Equal Interval".to_string(),
            summary: "Reclassifies raster values into equal-width intervals over an optional value range.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input raster.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "interval_size".to_string(),
                    description: "Equal-interval bin width.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "start_value".to_string(),
                    description: "Optional lower bound for reclassification; defaults to raster minimum.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "end_value".to_string(),
                    description: "Optional upper bound for reclassification; defaults to raster maximum.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output raster path. If omitted, result is kept in memory.".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "reclass_equal_interval_basic".to_string(),
                description: "Reclassifies raster values to equal intervals.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "raster".to_string(),
                "gis".to_string(),
                "reclass".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "input")?;
        let interval = args
            .get("interval_size")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'interval_size' is required".to_string()))?;
        if !interval.is_finite() || interval <= 0.0 {
            return Err(ToolError::Validation(
                "interval_size must be a finite value > 0".to_string(),
            ));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_required_raster_arg(args, "input")?;
        let interval = args
            .get("interval_size")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'interval_size' is required".to_string()))?;
        let output_path = parse_optional_output_path(args, "output")?;

        let (min_val, max_val) = min_max_valid(&input).unwrap_or((f64::NEG_INFINITY, f64::INFINITY));
        let start_value = args
            .get("start_value")
            .and_then(|v| v.as_f64())
            .unwrap_or(min_val);
        let end_value = args
            .get("end_value")
            .and_then(|v| v.as_f64())
            .unwrap_or(max_val);

        let mut output = build_output_like_raster(&input, DataType::F64);
        output.nodata = input.nodata;
        for idx in 0..output.data.len() {
            output.data.set_f64(idx, input.nodata);
        }

        ctx.progress.info("running reclass_equal_interval");
        for idx in 0..input.data.len() {
            let mut value = input.data.get_f64(idx);
            if !input.is_nodata(value) && value >= start_value && value <= end_value {
                value = (value / interval).floor() * interval;
            }
            output.data.set_f64(idx, value);
            if idx % 8192 == 0 {
                ctx.progress.progress(idx as f64 / input.data.len().max(1) as f64);
            }
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        Ok(GisOverlayCore::build_result(locator))
    }
}

#[derive(Clone)]
struct IndexedLine {
    source_index: usize,
    coords: Vec<wbvector::Coord>,
}

fn coord_dist2(a: &wbvector::Coord, b: &wbvector::Coord) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

fn coord_eq_eps(a: &wbvector::Coord, b: &wbvector::Coord, eps: f64) -> bool {
    coord_dist2(a, b) <= eps * eps
}

fn dedup_coords_eps(coords: &mut Vec<wbvector::Coord>, eps: f64) {
    let mut deduped = Vec::<wbvector::Coord>::with_capacity(coords.len());
    for coord in coords.drain(..) {
        if deduped
            .last()
            .map(|prev| coord_eq_eps(prev, &coord, eps))
            .unwrap_or(false)
        {
            continue;
        }
        deduped.push(coord);
    }
    *coords = deduped;
}

fn push_unique_coord(coords: &mut Vec<wbvector::Coord>, coord: wbvector::Coord, eps: f64) {
    if !coords.iter().any(|c| coord_eq_eps(c, &coord, eps)) {
        coords.push(coord);
    }
}

fn point_on_segment_eps(
    point: &wbvector::Coord,
    a: &wbvector::Coord,
    b: &wbvector::Coord,
    eps: f64,
) -> bool {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let seg_len2 = dx * dx + dy * dy;
    if seg_len2 <= eps * eps {
        return coord_eq_eps(point, a, eps);
    }
    let cross = (point.x - a.x) * dy - (point.y - a.y) * dx;
    if cross.abs() > eps * seg_len2.sqrt().max(1.0) {
        return false;
    }
    let dot = (point.x - a.x) * dx + (point.y - a.y) * dy;
    dot >= -eps && dot <= seg_len2 + eps
}

fn segment_intersection_points(
    a1: &wbvector::Coord,
    a2: &wbvector::Coord,
    b1: &wbvector::Coord,
    b2: &wbvector::Coord,
    eps: f64,
) -> Vec<wbvector::Coord> {
    let mut out = Vec::<wbvector::Coord>::new();

    let aminx = a1.x.min(a2.x) - eps;
    let amaxx = a1.x.max(a2.x) + eps;
    let aminy = a1.y.min(a2.y) - eps;
    let amaxy = a1.y.max(a2.y) + eps;
    let bminx = b1.x.min(b2.x) - eps;
    let bmaxx = b1.x.max(b2.x) + eps;
    let bminy = b1.y.min(b2.y) - eps;
    let bmaxy = b1.y.max(b2.y) + eps;
    if amaxx < bminx || bmaxx < aminx || amaxy < bminy || bmaxy < aminy {
        return out;
    }

    let r_x = a2.x - a1.x;
    let r_y = a2.y - a1.y;
    let s_x = b2.x - b1.x;
    let s_y = b2.y - b1.y;
    let denom = r_x * s_y - r_y * s_x;
    let qp_x = b1.x - a1.x;
    let qp_y = b1.y - a1.y;

    if denom.abs() <= eps {
        let cross_qp_r = qp_x * r_y - qp_y * r_x;
        if cross_qp_r.abs() > eps {
            return out;
        }
        if point_on_segment_eps(a1, b1, b2, eps) {
            push_unique_coord(&mut out, a1.clone(), eps);
        }
        if point_on_segment_eps(a2, b1, b2, eps) {
            push_unique_coord(&mut out, a2.clone(), eps);
        }
        if point_on_segment_eps(b1, a1, a2, eps) {
            push_unique_coord(&mut out, b1.clone(), eps);
        }
        if point_on_segment_eps(b2, a1, a2, eps) {
            push_unique_coord(&mut out, b2.clone(), eps);
        }
        return out;
    }

    let t = (qp_x * s_y - qp_y * s_x) / denom;
    let u = (qp_x * r_y - qp_y * r_x) / denom;
    if t >= -eps && t <= 1.0 + eps && u >= -eps && u <= 1.0 + eps {
        out.push(wbvector::Coord::xy(a1.x + t * r_x, a1.y + t * r_y));
    }
    out
}

fn collect_lines_from_geometry(
    geometry: &wbvector::Geometry,
    source_index: usize,
    include_polygon_boundaries: bool,
    out: &mut Vec<IndexedLine>,
) -> Result<(), ToolError> {
    match geometry {
        wbvector::Geometry::LineString(coords) => {
            if coords.len() >= 2 {
                out.push(IndexedLine {
                    source_index,
                    coords: coords.clone(),
                });
            }
        }
        wbvector::Geometry::MultiLineString(lines) => {
            for line in lines {
                if line.len() >= 2 {
                    out.push(IndexedLine {
                        source_index,
                        coords: line.clone(),
                    });
                }
            }
        }
        wbvector::Geometry::Polygon { exterior, interiors } if include_polygon_boundaries => {
            if exterior.coords().len() >= 2 {
                out.push(IndexedLine {
                    source_index,
                    coords: exterior.coords().to_vec(),
                });
            }
            for ring in interiors {
                if ring.coords().len() >= 2 {
                    out.push(IndexedLine {
                        source_index,
                        coords: ring.coords().to_vec(),
                    });
                }
            }
        }
        wbvector::Geometry::MultiPolygon(parts) if include_polygon_boundaries => {
            for (exterior, interiors) in parts {
                if exterior.coords().len() >= 2 {
                    out.push(IndexedLine {
                        source_index,
                        coords: exterior.coords().to_vec(),
                    });
                }
                for ring in interiors {
                    if ring.coords().len() >= 2 {
                        out.push(IndexedLine {
                            source_index,
                            coords: ring.coords().to_vec(),
                        });
                    }
                }
            }
        }
        wbvector::Geometry::GeometryCollection(parts) => {
            for part in parts {
                collect_lines_from_geometry(part, source_index, include_polygon_boundaries, out)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn collect_layer_linework(
    layer: &wbvector::Layer,
    include_polygon_boundaries: bool,
) -> Result<Vec<IndexedLine>, ToolError> {
    let mut lines = Vec::<IndexedLine>::new();
    for (index, feature) in layer.features.iter().enumerate() {
        if let Some(geometry) = &feature.geometry {
            collect_lines_from_geometry(geometry, index, include_polygon_boundaries, &mut lines)?;
        }
    }
    Ok(lines)
}

fn merge_line_params_with_points(
    a: &wbvector::Coord,
    b: &wbvector::Coord,
    split_points: &[wbvector::Coord],
    eps: f64,
) -> Vec<wbvector::Coord> {
    let mut params = vec![(0.0f64, a.clone()), (1.0f64, b.clone())];
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let seg_len2 = dx * dx + dy * dy;
    if seg_len2 <= eps * eps {
        return vec![a.clone(), b.clone()];
    }

    for point in split_points {
        if !point_on_segment_eps(point, a, b, eps) {
            continue;
        }
        let t = ((point.x - a.x) * dx + (point.y - a.y) * dy) / seg_len2;
        if t > eps && t < 1.0 - eps {
            params.push((t, point.clone()));
        }
    }

    params.sort_by(|lhs, rhs| lhs.0.total_cmp(&rhs.0));
    let mut coords = Vec::<wbvector::Coord>::with_capacity(params.len());
    for (_, coord) in params {
        if coords
            .last()
            .map(|prev| coord_eq_eps(prev, &coord, eps))
            .unwrap_or(false)
        {
            continue;
        }
        coords.push(coord);
    }
    coords
}

fn split_lines_against_splitter(
    line: &[wbvector::Coord],
    splitter: &[IndexedLine],
    eps: f64,
) -> Vec<Vec<wbvector::Coord>> {
    let mut pieces = Vec::<Vec<wbvector::Coord>>::new();
    if line.len() < 2 {
        return pieces;
    }

    for seg_idx in 1..line.len() {
        let a = &line[seg_idx - 1];
        let b = &line[seg_idx];
        let mut cut_points = Vec::<wbvector::Coord>::new();
        for split_line in splitter {
            for split_seg_idx in 1..split_line.coords.len() {
                let s1 = &split_line.coords[split_seg_idx - 1];
                let s2 = &split_line.coords[split_seg_idx];
                let intersections = segment_intersection_points(a, b, s1, s2, eps);
                for intersection in intersections {
                    push_unique_coord(&mut cut_points, intersection, eps);
                }
            }
        }

        let segment_points = merge_line_params_with_points(a, b, &cut_points, eps);
        for i in 1..segment_points.len() {
            let p0 = &segment_points[i - 1];
            let p1 = &segment_points[i];
            if coord_dist2(p0, p1) > eps * eps {
                pieces.push(vec![p0.clone(), p1.clone()]);
            }
        }
    }

    pieces
}

fn quantize_coord_key(coord: &wbvector::Coord, eps: f64) -> (i64, i64) {
    let step = eps.max(1.0e-12);
    (
        (coord.x / step).round() as i64,
        (coord.y / step).round() as i64,
    )
}

fn line_tool_params(input_desc: &'static str, overlay_desc: &'static str) -> Vec<ToolParamSpec> {
    vec![
        ToolParamSpec {
            name: "input",
            description: input_desc,
            required: true,
        },
        ToolParamSpec {
            name: "overlay",
            description: overlay_desc,
            required: true,
        },
        ToolParamSpec {
            name: "snap_tolerance",
            description:
                "Optional snapping tolerance used by geometry operations; defaults to machine epsilon.",
            required: false,
        },
        ToolParamSpec {
            name: "output",
            description: "Output vector path.",
            required: true,
        },
    ]
}

fn line_tool_manifest(
    id: &str,
    display_name: &str,
    summary: &str,
    input_desc: &str,
    overlay_desc: &str,
    input_default: &str,
    overlay_default: &str,
    output_default: &str,
) -> ToolManifest {
    let mut defaults = ToolArgs::new();
    defaults.insert("input".to_string(), json!(input_default));
    defaults.insert("overlay".to_string(), json!(overlay_default));
    defaults.insert("snap_tolerance".to_string(), json!(f64::EPSILON));

    let mut example_args = defaults.clone();
    example_args.insert("output".to_string(), json!(output_default));

    ToolManifest {
        id: id.to_string(),
        display_name: display_name.to_string(),
        summary: summary.to_string(),
        category: ToolCategory::Vector,
        license_tier: LicenseTier::Open,
        params: vec![
            ToolParamDescriptor {
                name: "input".to_string(),
                description: input_desc.to_string(),
                required: true,
            },
            ToolParamDescriptor {
                name: "overlay".to_string(),
                description: overlay_desc.to_string(),
                required: true,
            },
            ToolParamDescriptor {
                name: "snap_tolerance".to_string(),
                description: "Optional snapping tolerance used by geometry operations; defaults to machine epsilon.".to_string(),
                required: false,
            },
            ToolParamDescriptor {
                name: "output".to_string(),
                description: "Output vector path.".to_string(),
                required: true,
            },
        ],
        defaults,
        examples: vec![ToolExample {
            name: format!("{}_basic", id),
            description: summary.to_string(),
            args: example_args,
        }],
        tags: vec![
            "vector".to_string(),
            "gis".to_string(),
            "overlay".to_string(),
            "linework".to_string(),
            "legacy-port".to_string(),
        ],
        stability: ToolStability::Experimental,
    }
}

impl Tool for LineIntersectionsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "line_intersections",
            display_name: "Line Intersections",
            summary: "Finds line intersection points between input and overlay layers and appends parent IDs with merged attributes.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: line_tool_params(
                "Input line or polygon layer.",
                "Overlay line or polygon layer.",
            ),
        }
    }

    fn manifest(&self) -> ToolManifest {
        line_tool_manifest(
            "line_intersections",
            "Line Intersections",
            "Finds line intersection points between input and overlay layers and appends parent IDs with merged attributes.",
            "Input line or polygon layer.",
            "Overlay line or polygon layer.",
            "input_lines.shp",
            "overlay_lines.shp",
            "line_intersections.shp",
        )
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        validate_overlay_common_args(args)
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (input, overlay) = load_overlay_layers_aligned(args)?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let eps = parse_overlay_snap_tolerance(args).max(1.0e-12);

        let input_lines = collect_layer_linework(&input, true)?;
        let overlay_lines = collect_layer_linework(&overlay, true)?;
        let (merged_schema, input_mapping, overlay_mapping) =
            build_merged_overlay_schema(&input, &overlay);

        let mut output = wbvector::Layer::new(format!("{}_line_intersections", input.name));
        output.schema = wbvector::Schema::new();
        output
            .schema
            .add_field(wbvector::FieldDef::new("PARENT1", wbvector::FieldType::Integer));
        output
            .schema
            .add_field(wbvector::FieldDef::new("PARENT2", wbvector::FieldType::Integer));
        for field in merged_schema.fields() {
            output.schema.add_field(field.clone());
        }
        output.crs = input.crs.clone();
        output.geom_type = Some(wbvector::GeometryType::Point);

        let mut seen = HashSet::<(i64, i64, usize, usize)>::new();
        let total = input_lines.len().max(1);
        let mut next_fid = 1u64;

        for (line_idx, lhs) in input_lines.iter().enumerate() {
            for rhs in &overlay_lines {
                for i in 1..lhs.coords.len() {
                    for j in 1..rhs.coords.len() {
                        let intersections = segment_intersection_points(
                            &lhs.coords[i - 1],
                            &lhs.coords[i],
                            &rhs.coords[j - 1],
                            &rhs.coords[j],
                            eps,
                        );
                        for point in intersections {
                            let key_xy = quantize_coord_key(&point, eps);
                            let key = (key_xy.0, key_xy.1, lhs.source_index, rhs.source_index);
                            if !seen.insert(key) {
                                continue;
                            }

                            let input_attrs = input.features[lhs.source_index].attributes.as_slice();
                            let overlay_attrs = overlay.features[rhs.source_index].attributes.as_slice();
                            let mut attrs = Vec::<wbvector::FieldValue>::new();
                            attrs.push(wbvector::FieldValue::Integer(
                                input.features[lhs.source_index].fid.max(1) as i64,
                            ));
                            attrs.push(wbvector::FieldValue::Integer(
                                overlay.features[rhs.source_index].fid.max(1) as i64,
                            ));
                            attrs.extend(merged_overlay_attributes(
                                Some(input_attrs),
                                Some(overlay_attrs),
                                &input_mapping,
                                &overlay_mapping,
                                merged_schema.len(),
                            ));

                            output.push(wbvector::Feature {
                                fid: next_fid,
                                geometry: Some(wbvector::Geometry::Point(point)),
                                attributes: attrs,
                            });
                            next_fid += 1;
                        }
                    }
                }
            }

            ctx.progress
                .progress((line_idx + 1) as f64 / total as f64);
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for MergeLineSegmentsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "merge_line_segments",
            display_name: "Merge Line Segments",
            summary: "Merges connected line segments that meet at non-branching endpoints.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input polyline vector layer.",
                    required: true,
                },
                ToolParamSpec {
                    name: "snap_tolerance",
                    description:
                        "Optional endpoint snapping tolerance used to match line endpoints; defaults to machine epsilon.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output vector path.",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input_lines.shp"));
        defaults.insert("snap_tolerance".to_string(), json!(f64::EPSILON));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("merge_line_segments.shp"));

        ToolManifest {
            id: "merge_line_segments".to_string(),
            display_name: "Merge Line Segments".to_string(),
            summary: "Merges connected line segments that meet at non-branching endpoints.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input polyline vector layer.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "snap_tolerance".to_string(),
                    description: "Optional endpoint snapping tolerance used to match line endpoints; defaults to machine epsilon.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output vector path.".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "merge_line_segments_basic".to_string(),
                description: "Merges connected line segments into longer polylines.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "linework".to_string(),
                "merge".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        let eps = parse_overlay_snap_tolerance(args);
        if !eps.is_finite() || eps < 0.0 {
            return Err(ToolError::Validation(
                "snap_tolerance must be a finite value >= 0".to_string(),
            ));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let eps = parse_overlay_snap_tolerance(args).max(1.0e-12);

        let lines = collect_layer_linework(&input, false)?;
        let mut output = wbvector::Layer::new(format!("{}_merged", input.name));
        output.schema = input.schema.clone();
        output.crs = input.crs.clone();
        output.geom_type = Some(wbvector::GeometryType::LineString);

        if lines.is_empty() {
            let output_locator = write_vector_output(&output, output_path.trim())?;
            return Ok(build_vector_result(output_locator));
        }

        let mut endpoint_to_edges = HashMap::<(i64, i64), Vec<(usize, bool)>>::new();
        for (edge_idx, line) in lines.iter().enumerate() {
            if line.coords.len() < 2 {
                continue;
            }
            let start_key = quantize_coord_key(&line.coords[0], eps);
            let end_key = quantize_coord_key(&line.coords[line.coords.len() - 1], eps);
            endpoint_to_edges.entry(start_key).or_default().push((edge_idx, true));
            endpoint_to_edges.entry(end_key).or_default().push((edge_idx, false));
        }

        let mut visited = vec![false; lines.len()];
        let total = lines.len().max(1);
        let mut next_fid = 1u64;

        for edge_idx in 0..lines.len() {
            if visited[edge_idx] || lines[edge_idx].coords.len() < 2 {
                continue;
            }

            let mut merged = lines[edge_idx].coords.clone();
            visited[edge_idx] = true;

            // Extend forward.
            loop {
                let tail = merged[merged.len() - 1].clone();
                let key = quantize_coord_key(&tail, eps);
                let Some(candidates) = endpoint_to_edges.get(&key) else {
                    break;
                };
                if candidates.len() != 2 {
                    break;
                }

                let mut next = None;
                for (cand_idx, is_start) in candidates {
                    if !visited[*cand_idx] {
                        next = Some((*cand_idx, *is_start));
                    }
                }
                let Some((cand_idx, is_start)) = next else {
                    break;
                };

                let mut part = lines[cand_idx].coords.clone();
                if !is_start {
                    part.reverse();
                }
                if coord_eq_eps(&tail, &part[0], eps) {
                    part.remove(0);
                }
                merged.extend(part);
                visited[cand_idx] = true;
            }

            // Extend backward.
            loop {
                let head = merged[0].clone();
                let key = quantize_coord_key(&head, eps);
                let Some(candidates) = endpoint_to_edges.get(&key) else {
                    break;
                };
                if candidates.len() != 2 {
                    break;
                }

                let mut next = None;
                for (cand_idx, is_start) in candidates {
                    if !visited[*cand_idx] {
                        next = Some((*cand_idx, *is_start));
                    }
                }
                let Some((cand_idx, is_start)) = next else {
                    break;
                };

                let mut part = lines[cand_idx].coords.clone();
                if is_start {
                    part.reverse();
                }
                if coord_eq_eps(&head, &part[part.len() - 1], eps) {
                    part.pop();
                }
                part.extend(merged);
                merged = part;
                visited[cand_idx] = true;
            }

            dedup_coords_eps(&mut merged, eps);
            if merged.len() >= 2 {
                let source = lines[edge_idx].source_index;
                output.push(wbvector::Feature {
                    fid: next_fid,
                    geometry: Some(wbvector::Geometry::LineString(merged)),
                    attributes: input.features[source].attributes.clone(),
                });
                next_fid += 1;
            }

            let done = visited.iter().filter(|done| **done).count();
            ctx.progress.progress(done as f64 / total as f64);
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

fn parse_vector_list_arg(args: &ToolArgs, key: &str) -> Option<Vec<String>> {
    let value = args.get(key)?;
    if let Some(s) = value.as_str() {
        let items = s
            .split([',', ';'])
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(|item| item.to_string())
            .collect::<Vec<_>>();
        return Some(items);
    }

    if let Some(arr) = value.as_array() {
        let mut items = Vec::with_capacity(arr.len());
        for item in arr {
            if let Some(path) = item.as_str() {
                let path = path.trim();
                if !path.is_empty() {
                    items.push(path.to_string());
                }
            }
        }
        return Some(items);
    }

    None
}

fn align_layer_to_reference_crs(
    reference: &wbvector::Layer,
    mut layer: wbvector::Layer,
    label: &str,
) -> Result<wbvector::Layer, ToolError> {
    let reference_epsg = reference.crs_epsg();
    let reference_wkt = reference.crs_wkt().map(str::trim).filter(|s| !s.is_empty());
    let layer_epsg = layer.crs_epsg();
    let layer_wkt = layer.crs_wkt().map(str::trim).filter(|s| !s.is_empty());

    if reference_epsg.is_none() && reference_wkt.is_none() {
        return Ok(layer);
    }
    if layer_epsg.is_none() && layer_wkt.is_none() {
        return Ok(layer);
    }

    let epsg_matches = reference_epsg.is_some() && reference_epsg == layer_epsg;
    let wkt_matches = match (reference_wkt, layer_wkt) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    };
    if epsg_matches || wkt_matches {
        return Ok(layer);
    }

    if let Some(dst_epsg) = reference_epsg {
        layer = layer.reproject_to_epsg(dst_epsg).map_err(|e| {
            ToolError::Validation(format!(
                "{} CRS does not match first input CRS; automatic reprojection to EPSG:{} failed: {}",
                label, dst_epsg, e
            ))
        })?;
    }

    Ok(layer)
}

impl Tool for PolygonizeTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "polygonize",
            display_name: "Polygonize",
            summary: "Creates polygons from closed input linework rings.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "inputs",
                    description:
                        "Input line vector layers as an array or semicolon/comma-delimited string. If omitted, 'input' is used.",
                    required: false,
                },
                ToolParamSpec {
                    name: "input",
                    description: "Optional single input line vector layer path.",
                    required: false,
                },
                ToolParamSpec {
                    name: "snap_tolerance",
                    description:
                        "Optional snapping tolerance used by polygonization; defaults to machine epsilon.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output vector path.",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["lines.shp"]));
        defaults.insert("snap_tolerance".to_string(), json!(f64::EPSILON));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("polygonize.shp"));

        ToolManifest {
            id: "polygonize".to_string(),
            display_name: "Polygonize".to_string(),
            summary: "Creates polygons from closed input linework rings.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "inputs".to_string(),
                    description: "Input line vector layers as an array or semicolon/comma-delimited string. If omitted, 'input' is used.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Optional single input line vector layer path.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "snap_tolerance".to_string(),
                    description: "Optional snapping tolerance used by polygonization; defaults to machine epsilon.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output vector path.".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "polygonize_basic".to_string(),
                description: "Polygonizes closed input linework.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "linework".to_string(),
                "polygonize".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_vector_list_arg(args, "inputs")
            .or_else(|| parse_required_vector_path_arg(args, "input").ok().map(|p| vec![p]))
            .unwrap_or_default();
        if inputs.is_empty() {
            return Err(ToolError::Validation(
                "parameter 'inputs' or 'input' must provide at least one vector path".to_string(),
            ));
        }
        for path in inputs {
            let _ = read_vector_layer_from_path(path.as_str(), "inputs")?;
        }
        let _ = parse_vector_path_arg(args, "output")?;
        let eps = parse_overlay_snap_tolerance(args);
        if !eps.is_finite() || eps < 0.0 {
            return Err(ToolError::Validation(
                "snap_tolerance must be a finite value >= 0".to_string(),
            ));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let output_path = parse_vector_path_arg(args, "output")?;
        let eps = parse_overlay_snap_tolerance(args).max(1.0e-12);
        let input_paths = parse_vector_list_arg(args, "inputs")
            .or_else(|| parse_required_vector_path_arg(args, "input").ok().map(|p| vec![p]))
            .unwrap_or_default();

        if input_paths.is_empty() {
            return Err(ToolError::Validation(
                "parameter 'inputs' or 'input' must provide at least one vector path".to_string(),
            ));
        }

        let mut layers = Vec::<wbvector::Layer>::new();
        for (idx, path) in input_paths.iter().enumerate() {
            let mut layer = read_vector_layer_from_path(path.as_str(), "inputs")?;
            if idx > 0 {
                layer = align_layer_to_reference_crs(&layers[0], layer, "input layer")?;
            }
            layers.push(layer);
        }

        let mut lines = Vec::<TopoLineString>::new();
        for layer in &layers {
            let linework = collect_layer_linework(layer, false)?;
            for line in linework {
                if line.coords.len() < 3 {
                    continue;
                }
                let mut coords = line.coords.clone();
                dedup_coords_eps(&mut coords, eps);
                if coords.len() < 3 {
                    continue;
                }
                if !coord_eq_eps(&coords[0], &coords[coords.len() - 1], eps) {
                    continue;
                }
                lines.push(TopoLineString::new(coords.iter().map(to_topo_coord).collect()));
            }
        }

        let polygons = polygonize_closed_linestrings(&lines, eps);
        let mut output = wbvector::Layer::new("polygonize");
        output.geom_type = Some(wbvector::GeometryType::Polygon);
        output.crs = layers.first().and_then(|layer| layer.crs.clone());

        for (idx, polygon) in polygons.into_iter().enumerate() {
            output.push(wbvector::Feature {
                fid: (idx + 1) as u64,
                geometry: Some(wbvector::Geometry::Polygon {
                    exterior: to_wb_ring(&polygon.exterior),
                    interiors: to_wb_rings(&polygon.holes),
                }),
                attributes: Vec::new(),
            });
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for SplitWithLinesTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "split_with_lines",
            display_name: "Split With Lines",
            summary: "Splits input polylines using intersection points from a split line layer.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input line layer to split.",
                    required: true,
                },
                ToolParamSpec {
                    name: "split",
                    description: "Split line layer used to create break points.",
                    required: true,
                },
                ToolParamSpec {
                    name: "snap_tolerance",
                    description:
                        "Optional snapping tolerance used by split operations; defaults to machine epsilon.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output vector path.",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input_lines.shp"));
        defaults.insert("split".to_string(), json!("split_lines.shp"));
        defaults.insert("snap_tolerance".to_string(), json!(f64::EPSILON));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("split_with_lines.shp"));

        ToolManifest {
            id: "split_with_lines".to_string(),
            display_name: "Split With Lines".to_string(),
            summary: "Splits input polylines using intersection points from a split line layer.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input line layer to split.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "split".to_string(),
                    description: "Split line layer used to create break points.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "snap_tolerance".to_string(),
                    description: "Optional snapping tolerance used by split operations; defaults to machine epsilon.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output vector path.".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "split_with_lines_basic".to_string(),
                description: "Splits line features where split lines intersect them.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "linework".to_string(),
                "split".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_vector_path_arg(args, "input")?;
        let _ = parse_required_vector_path_arg(args, "split")?;
        let _ = parse_vector_path_arg(args, "output")?;
        let eps = parse_overlay_snap_tolerance(args);
        if !eps.is_finite() || eps < 0.0 {
            return Err(ToolError::Validation(
                "snap_tolerance must be a finite value >= 0".to_string(),
            ));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_required_vector_path_arg(args, "input")?;
        let split_path = parse_required_vector_path_arg(args, "split")?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let eps = parse_overlay_snap_tolerance(args).max(1.0e-12);

        let input = read_vector_layer_from_path(input_path.as_str(), "input")?;
        let split = read_vector_layer_from_path(split_path.as_str(), "split")?;
        let (input, split) = align_overlay_layers_crs(input, split)?;

        let input_lines = collect_layer_linework(&input, false)?;
        let split_lines = collect_layer_linework(&split, false)?;

        let mut output = wbvector::Layer::new(format!("{}_split", input.name));
        output.schema = wbvector::Schema::new();
        output
            .schema
            .add_field(wbvector::FieldDef::new("PARENT_FID", wbvector::FieldType::Integer));
        for field in input.schema.fields() {
            output.schema.add_field(field.clone());
        }
        output.crs = input.crs.clone();
        output.geom_type = Some(wbvector::GeometryType::LineString);

        let total = input_lines.len().max(1);
        let mut next_fid = 1u64;
        for (line_idx, line) in input_lines.iter().enumerate() {
            let pieces = split_lines_against_splitter(&line.coords, &split_lines, eps);
            let parent_fid = input.features[line.source_index].fid.max(1) as i64;

            for piece in pieces {
                if piece.len() < 2 {
                    continue;
                }
                let mut attrs = Vec::<wbvector::FieldValue>::new();
                attrs.push(wbvector::FieldValue::Integer(parent_fid));
                attrs.extend(input.features[line.source_index].attributes.clone());

                output.push(wbvector::Feature {
                    fid: next_fid,
                    geometry: Some(wbvector::Geometry::LineString(piece)),
                    attributes: attrs,
                });
                next_fid += 1;
            }

            ctx.progress
                .progress((line_idx + 1) as f64 / total as f64);
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

fn compute_polygon_complexity(ring: &wbvector::Ring) -> f64 {
    let coords = ring.coords();
    if coords.len() < 3 {
        return 0.0;
    }
    let area = polygon_area(coords);
    if area <= 0.0 {
        return 0.0;
    }
    let perimeter = polygon_perimeter(coords);
    if perimeter <= 0.0 {
        return 0.0;
    }
    (perimeter * perimeter) / (4.0 * std::f64::consts::PI * area)
}

fn polygon_area(coords: &[wbvector::Coord]) -> f64 {
    let mut area = 0.0;
    for i in 0..coords.len() {
        let j = (i + 1) % coords.len();
        area += coords[i].x * coords[j].y;
        area -= coords[j].x * coords[i].y;
    }
    (area / 2.0).abs()
}

fn polygon_perimeter(coords: &[wbvector::Coord]) -> f64 {
    let mut perimeter = 0.0;
    for i in 0..coords.len() {
        let j = (i + 1) % coords.len();
        let dx = coords[j].x - coords[i].x;
        let dy = coords[j].y - coords[i].y;
        perimeter += (dx * dx + dy * dy).sqrt();
    }
    perimeter
}

fn get_polygon_area(geom: &wbvector::Geometry) -> Option<f64> {
    match geom {
        wbvector::Geometry::Polygon { exterior, .. } => {
            Some(polygon_area(exterior.coords()))
        }
        wbvector::Geometry::MultiPolygon(parts) => {
            let mut total_area = 0.0;
            for (exterior, _) in parts {
                total_area += polygon_area(exterior.coords());
            }
            Some(total_area)
        }
        _ => None,
    }
}

fn get_polygon_perimeter(geom: &wbvector::Geometry) -> Option<f64> {
    match geom {
        wbvector::Geometry::Polygon { exterior, interiors } => {
            let mut perim = polygon_perimeter(exterior.coords());
            for interior in interiors {
                perim += polygon_perimeter(interior.coords());
            }
            Some(perim)
        }
        wbvector::Geometry::MultiPolygon(parts) => {
            let mut total_perim = 0.0;
            for (exterior, interiors) in parts {
                total_perim += polygon_perimeter(exterior.coords());
                for interior in interiors {
                    total_perim += polygon_perimeter(interior.coords());
                }
            }
            Some(total_perim)
        }
        _ => None,
    }
}

fn get_linestring_length(coords: &[wbvector::Coord]) -> f64 {
    let mut length = 0.0;
    for i in 0..coords.len() {
        let j = i + 1;
        if j < coords.len() {
            let dx = coords[j].x - coords[i].x;
            let dy = coords[j].y - coords[i].y;
            length += (dx * dx + dy * dy).sqrt();
        }
    }
    length
}

fn get_geometry_length(geom: &wbvector::Geometry) -> Option<f64> {
    match geom {
        wbvector::Geometry::LineString(coords) => Some(get_linestring_length(coords)),
        wbvector::Geometry::MultiLineString(lines) => {
            let mut total_length = 0.0;
            for line in lines {
                total_length += get_linestring_length(line);
            }
            Some(total_length)
        }
        wbvector::Geometry::Polygon { exterior, .. } => {
            Some(polygon_perimeter(exterior.coords()))
        }
        wbvector::Geometry::MultiPolygon(parts) => {
            let mut total_perim = 0.0;
            for (exterior, _) in parts {
                total_perim += polygon_perimeter(exterior.coords());
            }
            Some(total_perim)
        }
        _ => None,
    }
}

fn get_bounding_box(geom: &wbvector::Geometry) -> Option<(f64, f64, f64, f64)> {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut found = false;

    fn update_bounds(coord: &wbvector::Coord, min_x: &mut f64, min_y: &mut f64, max_x: &mut f64, max_y: &mut f64, found: &mut bool) {
        *min_x = min_x.min(coord.x);
        *min_y = min_y.min(coord.y);
        *max_x = max_x.max(coord.x);
        *max_y = max_y.max(coord.y);
        *found = true;
    }

    match geom {
        wbvector::Geometry::Point(coord) => {
            update_bounds(coord, &mut min_x, &mut min_y, &mut max_x, &mut max_y, &mut found);
        }
        wbvector::Geometry::LineString(coords) => {
            for coord in coords {
                update_bounds(coord, &mut min_x, &mut min_y, &mut max_x, &mut max_y, &mut found);
            }
        }
        wbvector::Geometry::Polygon { exterior, interiors } => {
            for coord in exterior.coords() {
                update_bounds(coord, &mut min_x, &mut min_y, &mut max_x, &mut max_y, &mut found);
            }
            for interior in interiors {
                for coord in interior.coords() {
                    update_bounds(coord, &mut min_x, &mut min_y, &mut max_x, &mut max_y, &mut found);
                }
            }
        }
        wbvector::Geometry::MultiPoint(points) => {
            for coord in points {
                update_bounds(coord, &mut min_x, &mut min_y, &mut max_x, &mut max_y, &mut found);
            }
        }
        wbvector::Geometry::MultiLineString(lines) => {
            for line in lines {
                for coord in line {
                    update_bounds(coord, &mut min_x, &mut min_y, &mut max_x, &mut max_y, &mut found);
                }
            }
        }
        wbvector::Geometry::MultiPolygon(parts) => {
            for (exterior, interiors) in parts {
                for coord in exterior.coords() {
                    update_bounds(coord, &mut min_x, &mut min_y, &mut max_x, &mut max_y, &mut found);
                }
                for interior in interiors {
                    for coord in interior.coords() {
                        update_bounds(coord, &mut min_x, &mut min_y, &mut max_x, &mut max_y, &mut found);
                    }
                }
            }
        }
        wbvector::Geometry::GeometryCollection(geometries) => {
            for geometry in geometries {
                if let Some((gmin_x, gmin_y, gmax_x, gmax_y)) = get_bounding_box(geometry) {
                    update_bounds(&wbvector::Coord::xy(gmin_x, gmin_y), &mut min_x, &mut min_y, &mut max_x, &mut max_y, &mut found);
                    update_bounds(&wbvector::Coord::xy(gmax_x, gmax_y), &mut min_x, &mut min_y, &mut max_x, &mut max_y, &mut found);
                }
            }
        }
    }

    if found {
        Some((min_x, min_y, max_x, max_y))
    } else {
        None
    }
}

fn polygon_coords_for_orientation(geom: &wbvector::Geometry) -> Option<Vec<wbvector::Coord>> {
    match geom {
        wbvector::Geometry::Polygon { exterior, .. } => Some(exterior.coords().to_vec()),
        wbvector::Geometry::MultiPolygon(parts) => parts.first().map(|(ext, _)| ext.coords().to_vec()),
        _ => None,
    }
}

fn circle_from_diameter(a: &wbvector::Coord, b: &wbvector::Coord) -> (f64, f64, f64) {
    let cx = (a.x + b.x) * 0.5;
    let cy = (a.y + b.y) * 0.5;
    let r = (((a.x - b.x) * (a.x - b.x) + (a.y - b.y) * (a.y - b.y)).sqrt()) * 0.5;
    (cx, cy, r)
}

fn circle_from_three_points(a: &wbvector::Coord, b: &wbvector::Coord, c: &wbvector::Coord) -> Option<(f64, f64, f64)> {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() <= f64::EPSILON {
        return None;
    }
    let ax2ay2 = a.x * a.x + a.y * a.y;
    let bx2by2 = b.x * b.x + b.y * b.y;
    let cx2cy2 = c.x * c.x + c.y * c.y;
    let ux = (ax2ay2 * (b.y - c.y) + bx2by2 * (c.y - a.y) + cx2cy2 * (a.y - b.y)) / d;
    let uy = (ax2ay2 * (c.x - b.x) + bx2by2 * (a.x - c.x) + cx2cy2 * (b.x - a.x)) / d;
    let r = ((ux - a.x) * (ux - a.x) + (uy - a.y) * (uy - a.y)).sqrt();
    Some((ux, uy, r))
}

fn point_in_circle(p: &wbvector::Coord, cx: f64, cy: f64, r: f64) -> bool {
    let dx = p.x - cx;
    let dy = p.y - cy;
    dx * dx + dy * dy <= r * r + 1.0e-10
}

fn smallest_enclosing_circle(coords: &[wbvector::Coord]) -> Option<(f64, f64, f64)> {
    if coords.is_empty() {
        return None;
    }
    if coords.len() == 1 {
        return Some((coords[0].x, coords[0].y, 0.0));
    }

    let mut best = (coords[0].x, coords[0].y, f64::INFINITY);

    for i in 0..coords.len() {
        for j in (i + 1)..coords.len() {
            let (cx, cy, r) = circle_from_diameter(&coords[i], &coords[j]);
            if r >= best.2 {
                continue;
            }
            if coords.iter().all(|p| point_in_circle(p, cx, cy, r)) {
                best = (cx, cy, r);
            }
        }
    }

    for i in 0..coords.len() {
        for j in (i + 1)..coords.len() {
            for k in (j + 1)..coords.len() {
                if let Some((cx, cy, r)) = circle_from_three_points(&coords[i], &coords[j], &coords[k]) {
                    if r >= best.2 {
                        continue;
                    }
                    if coords.iter().all(|p| point_in_circle(p, cx, cy, r)) {
                        best = (cx, cy, r);
                    }
                }
            }
        }
    }

    if best.2.is_finite() {
        Some(best)
    } else {
        None
    }
}

fn circle_ring_coords(cx: f64, cy: f64, radius: f64, num_vertices: usize) -> Vec<wbvector::Coord> {
    let mut ring = Vec::<wbvector::Coord>::with_capacity(num_vertices + 1);
    let angular_resolution = std::f64::consts::TAU / num_vertices.max(3) as f64;
    for i in 0..num_vertices {
        let angle = i as f64 * angular_resolution;
        ring.push(wbvector::Coord::xy(
            cx + radius * angle.sin(),
            cy + radius * angle.cos(),
        ));
    }
    if let Some(first) = ring.first().cloned() {
        ring.push(first);
    }
    ring
}

fn smallest_enclosing_circle_radius(coords: &[wbvector::Coord]) -> f64 {
    smallest_enclosing_circle(coords)
        .map(|(_, _, radius)| radius)
        .unwrap_or(0.0)
}

fn feature_orientation_deg_from_rma(geom: &wbvector::Geometry) -> Option<f64> {
    let coords = polygon_coords_for_orientation(geom)?;
    if coords.len() < 2 {
        return None;
    }
    let (min_x, min_y, max_x, max_y) = get_bounding_box(geom)?;
    let midpoint_x = (max_x - min_x) / 2.0;
    let midpoint_y = (max_y - min_y) / 2.0;
    let n = coords.len() as f64;

    let mut sigma_x = 0.0;
    let mut sigma_y = 0.0;
    let mut sigma_xy = 0.0;
    let mut sigma_xsqr = 0.0;
    let mut sigma_ysqr = 0.0;
    for p in &coords {
        let x = p.x - midpoint_x;
        let y = p.y - midpoint_y;
        sigma_x += x;
        sigma_y += y;
        sigma_xy += x * y;
        sigma_xsqr += x * x;
        sigma_ysqr += y * y;
    }
    let mean = sigma_x / n;
    let sxx = sigma_xsqr / n - mean * mean;
    let syy = sigma_ysqr / n - (sigma_y / n) * (sigma_y / n);
    if sxx.abs() <= f64::EPSILON {
        return Some(0.0);
    }
    let mut slope_rma = (syy / sxx).abs().sqrt();
    if (sigma_xy - mean * sigma_y) / (sigma_xsqr - mean * sigma_x) < 0.0 {
        slope_rma = -slope_rma;
    }
    let slope_deg_rma = slope_rma.atan().to_degrees();
    Some(if slope_deg_rma < 0.0 {
        90.0 + -1.0 * slope_deg_rma
    } else {
        90.0 - slope_deg_rma
    })
}

fn feature_elongation_for_weight(geom: &wbvector::Geometry) -> Option<(f64, f64)> {
    let coords = polygon_coords_for_orientation(geom)?;
    if coords.len() < 2 {
        return None;
    }
    let orient = feature_orientation_deg_from_rma(geom)?.to_radians();
    let sin_t = orient.sin();
    let cos_t = orient.cos();
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for p in &coords {
        let xr = p.x * cos_t + p.y * sin_t;
        let yr = -p.x * sin_t + p.y * cos_t;
        min_x = min_x.min(xr);
        max_x = max_x.max(xr);
        min_y = min_y.min(yr);
        max_y = max_y.max(yr);
    }
    let dist1 = (max_x - min_x).abs();
    let dist2 = (max_y - min_y).abs();
    let short_axis = dist1.min(dist2);
    let long_axis = dist1.max(dist2);
    if long_axis <= f64::EPSILON {
        return None;
    }
    Some((1.0 - short_axis / long_axis, long_axis))
}

impl Tool for HoleProportionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "hole_proportion",
            display_name: "Hole Proportion",
            summary: "Calculates polygon hole area divided by hull area and appends HOLE_PROP.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input polygon vector layer.", required: true },
                ToolParamSpec { name: "output", description: "Output vector path.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("polygons.shp"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("hole_proportion.shp"));
        ToolManifest {
            id: "hole_proportion".to_string(),
            display_name: "Hole Proportion".to_string(),
            summary: "Calculates polygon hole area divided by hull area and appends HOLE_PROP.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input polygon vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector path.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample { name: "hole_proportion_basic".to_string(), description: "Calculates HOLE_PROP for polygons.".to_string(), args: example_args }],
            tags: vec!["vector".to_string(), "gis".to_string(), "shape".to_string(), "holes".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let mut output = input.clone();
        let mut schema = output.schema.clone();
        if schema.field_index("HOLE_PROP").is_none() {
            schema.add_field(wbvector::FieldDef::new("HOLE_PROP", wbvector::FieldType::Float));
            output.schema = schema;
        }
        let total = output.features.len().max(1);
        for (i, feature) in output.features.iter_mut().enumerate() {
            let ratio = if let Some(geom) = &feature.geometry {
                match geom {
                    wbvector::Geometry::Polygon { exterior, interiors } => {
                        let hull = polygon_area(exterior.coords());
                        let holes = interiors.iter().map(|r| polygon_area(r.coords())).sum::<f64>();
                        if hull > 0.0 { holes / hull } else { -999.0 }
                    }
                    wbvector::Geometry::MultiPolygon(parts) => {
                        let mut hull = 0.0;
                        let mut holes = 0.0;
                        for (ext, ints) in parts {
                            hull += polygon_area(ext.coords());
                            holes += ints.iter().map(|r| polygon_area(r.coords())).sum::<f64>();
                        }
                        if hull > 0.0 { holes / hull } else { -999.0 }
                    }
                    _ => -999.0,
                }
            } else { -999.0 };
            feature.attributes.push(wbvector::FieldValue::Float(ratio));
            ctx.progress.progress((i + 1) as f64 / total as f64);
        }
        let locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(locator))
    }
}

impl Tool for PatchOrientationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "patch_orientation",
            display_name: "Patch Orientation",
            summary: "Calculates polygon orientation (degrees from north) using reduced major axis regression and appends ORIENT.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input polygon vector layer.", required: true },
                ToolParamSpec { name: "output", description: "Output vector path.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("polygons.shp"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("patch_orientation.shp"));
        ToolManifest {
            id: "patch_orientation".to_string(),
            display_name: "Patch Orientation".to_string(),
            summary: "Calculates polygon orientation (degrees from north) using reduced major axis regression and appends ORIENT.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input polygon vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector path.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample { name: "patch_orientation_basic".to_string(), description: "Calculates ORIENT for polygons.".to_string(), args: example_args }],
            tags: vec!["vector".to_string(), "gis".to_string(), "shape".to_string(), "orientation".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let mut output = input.clone();
        let mut schema = output.schema.clone();
        if schema.field_index("ORIENT").is_none() {
            schema.add_field(wbvector::FieldDef::new("ORIENT", wbvector::FieldType::Float));
            output.schema = schema;
        }
        let total = output.features.len().max(1);
        for (i, feature) in output.features.iter_mut().enumerate() {
            let orient = feature
                .geometry
                .as_ref()
                .and_then(feature_orientation_deg_from_rma)
                .unwrap_or(-999.0);
            feature.attributes.push(wbvector::FieldValue::Float(orient));
            ctx.progress.progress((i + 1) as f64 / total as f64);
        }
        let locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(locator))
    }
}

impl Tool for PerimeterAreaRatioTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "perimeter_area_ratio",
            display_name: "Perimeter Area Ratio",
            summary: "Calculates polygon perimeter/area ratio and appends P_A_RATIO.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input polygon vector layer.", required: true },
                ToolParamSpec { name: "output", description: "Output vector path.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("polygons.shp"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("perimeter_area_ratio.shp"));
        ToolManifest {
            id: "perimeter_area_ratio".to_string(),
            display_name: "Perimeter Area Ratio".to_string(),
            summary: "Calculates polygon perimeter/area ratio and appends P_A_RATIO.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input polygon vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector path.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample { name: "perimeter_area_ratio_basic".to_string(), description: "Calculates P_A_RATIO for polygons.".to_string(), args: example_args }],
            tags: vec!["vector".to_string(), "gis".to_string(), "shape".to_string(), "perimeter".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let mut output = input.clone();
        let mut schema = output.schema.clone();
        if schema.field_index("P_A_RATIO").is_none() {
            schema.add_field(wbvector::FieldDef::new("P_A_RATIO", wbvector::FieldType::Float));
            output.schema = schema;
        }
        let total = output.features.len().max(1);
        for (i, feature) in output.features.iter_mut().enumerate() {
            let value = if let Some(geom) = &feature.geometry {
                let perimeter = get_polygon_perimeter(geom).unwrap_or(0.0);
                let area = get_polygon_area(geom).unwrap_or(0.0);
                if area > 0.0 { perimeter / area } else { -999.0 }
            } else { -999.0 };
            feature.attributes.push(wbvector::FieldValue::Float(value));
            ctx.progress.progress((i + 1) as f64 / total as f64);
        }
        let locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(locator))
    }
}

impl Tool for RelatedCircumscribingCircleTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "related_circumscribing_circle",
            display_name: "Related Circumscribing Circle",
            summary: "Calculates 1 - (polygon area / smallest circumscribing circle area) and appends RC_CIRCLE.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input polygon vector layer.", required: true },
                ToolParamSpec { name: "output", description: "Output vector path.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("polygons.shp"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("related_circumscribing_circle.shp"));
        ToolManifest {
            id: "related_circumscribing_circle".to_string(),
            display_name: "Related Circumscribing Circle".to_string(),
            summary: "Calculates 1 - (polygon area / smallest circumscribing circle area) and appends RC_CIRCLE.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input polygon vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector path.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample { name: "related_circumscribing_circle_basic".to_string(), description: "Calculates RC_CIRCLE for polygons.".to_string(), args: example_args }],
            tags: vec!["vector".to_string(), "gis".to_string(), "shape".to_string(), "circle".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let mut output = input.clone();
        let mut schema = output.schema.clone();
        if schema.field_index("RC_CIRCLE").is_none() {
            schema.add_field(wbvector::FieldDef::new("RC_CIRCLE", wbvector::FieldType::Float));
            output.schema = schema;
        }
        let total = output.features.len().max(1);
        for (i, feature) in output.features.iter_mut().enumerate() {
            let value = if let Some(geom) = &feature.geometry {
                let mut area = 0.0;
                let mut circ_area = 0.0;
                match geom {
                    wbvector::Geometry::Polygon { exterior, interiors } => {
                        area += polygon_area(exterior.coords());
                        circ_area += std::f64::consts::PI * smallest_enclosing_circle_radius(exterior.coords()).powi(2);
                        for hole in interiors {
                            area -= polygon_area(hole.coords());
                        }
                    }
                    wbvector::Geometry::MultiPolygon(parts) => {
                        for (ext, holes) in parts {
                            area += polygon_area(ext.coords());
                            circ_area += std::f64::consts::PI * smallest_enclosing_circle_radius(ext.coords()).powi(2);
                            for hole in holes {
                                area -= polygon_area(hole.coords());
                            }
                        }
                    }
                    _ => {}
                }
                if circ_area > 0.0 { 1.0 - area / circ_area } else { -999.0 }
            } else { -999.0 };
            feature.attributes.push(wbvector::FieldValue::Float(value));
            ctx.progress.progress((i + 1) as f64 / total as f64);
        }
        let locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(locator))
    }
}

impl Tool for DeviationFromRegionalDirectionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "deviation_from_regional_direction",
            display_name: "Deviation From Regional Direction",
            summary: "Calculates polygon directional deviation from weighted regional mean orientation and appends DEV_DIR.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input polygon vector layer.", required: true },
                ToolParamSpec { name: "elongation_threshold", description: "Minimum elongation used to include polygons in regional direction estimate; default 0.75.", required: false },
                ToolParamSpec { name: "output", description: "Output vector path.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("polygons.shp"));
        defaults.insert("elongation_threshold".to_string(), json!(0.75));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("deviation_from_regional_direction.shp"));
        ToolManifest {
            id: "deviation_from_regional_direction".to_string(),
            display_name: "Deviation From Regional Direction".to_string(),
            summary: "Calculates polygon directional deviation from weighted regional mean orientation and appends DEV_DIR.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input polygon vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "elongation_threshold".to_string(), description: "Minimum elongation used to include polygons in regional direction estimate; default 0.75.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector path.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample { name: "deviation_from_regional_direction_basic".to_string(), description: "Calculates DEV_DIR for polygons.".to_string(), args: example_args }],
            tags: vec!["vector".to_string(), "gis".to_string(), "shape".to_string(), "direction".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        let thresh = args.get("elongation_threshold").and_then(|v| v.as_f64()).unwrap_or(0.75);
        if !(0.0..=1.0).contains(&thresh) {
            return Err(ToolError::Validation("elongation_threshold must be between 0 and 1".to_string()));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;
        let elongation_threshold = args.get("elongation_threshold").and_then(|v| v.as_f64()).unwrap_or(0.75);

        let mut sum_sin = 0.0;
        let mut sum_cos = 0.0;
        let mut used = 0usize;
        for feature in &input.features {
            let Some(geom) = feature.geometry.as_ref() else { continue; };
            let Some(orient_deg) = feature_orientation_deg_from_rma(geom) else { continue; };
            let Some((elongation, long_axis)) = feature_elongation_for_weight(geom) else { continue; };
            let weight = if elongation >= elongation_threshold {
                used += 1;
                long_axis * elongation
            } else {
                0.0
            };
            let angle = (90.0 - orient_deg).to_radians();
            sum_sin += (angle * 2.0).sin() * weight;
            sum_cos += (angle * 2.0).cos() * weight;
        }

        if used == 0 {
            return Err(ToolError::Validation("no polygons met elongation_threshold for regional direction".to_string()));
        }

        let mut regional_angle = -(sum_sin.atan2(sum_cos) / 2.0).to_degrees() + 90.0;
        if regional_angle < 0.0 {
            regional_angle += 180.0;
        }

        let mut output = input.clone();
        let mut schema = output.schema.clone();
        if schema.field_index("DEV_DIR").is_none() {
            schema.add_field(wbvector::FieldDef::new("DEV_DIR", wbvector::FieldType::Float));
            output.schema = schema;
        }

        let total = output.features.len().max(1);
        for (i, feature) in output.features.iter_mut().enumerate() {
            let value = if let Some(geom) = &feature.geometry {
                if let Some(orient) = feature_orientation_deg_from_rma(geom) {
                    let mut d = orient - regional_angle;
                    if d < 0.0 {
                        d += 180.0;
                    }
                    if d > 90.0 {
                        d = 180.0 - d;
                    }
                    d
                } else {
                    -999.0
                }
            } else {
                -999.0
            };
            feature.attributes.push(wbvector::FieldValue::Float(value));
            ctx.progress.progress((i + 1) as f64 / total as f64);
        }

        let locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(locator))
    }
}

fn patch_bin_index(value: f64, min_val: f64, bins: usize) -> Option<usize> {
    if !value.is_finite() {
        return None;
    }
    let idx = (value.floor() - min_val.floor()) as isize;
    if idx < 0 || (idx as usize) >= bins {
        return None;
    }
    Some(idx as usize)
}

fn is_patch_value(value: f64, nodata: f64) -> bool {
    value > 0.0 && value != nodata
}

impl Tool for FindPatchEdgeCellsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "find_patch_edge_cells",
            display_name: "Find Patch Edge Cells",
            summary: "Identifies edge cells for each positive raster patch ID; non-edge patch cells are set to zero.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input patch-ID raster; positive values are treated as patch cells.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("patches.tif"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("find_patch_edge_cells.tif"));
        ToolManifest {
            id: "find_patch_edge_cells".to_string(),
            display_name: "Find Patch Edge Cells".to_string(),
            summary: "Identifies edge cells for each positive raster patch ID; non-edge patch cells are set to zero.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input patch-ID raster; positive values are treated as patch cells.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "find_patch_edge_cells_basic".to_string(),
                description: "Outputs raster cells located on patch boundaries.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "patch".to_string(), "edge".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_required_raster_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let stride = rows * cols;
        let nodata = input.nodata;
        let dx: [isize; 8] = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy: [isize; 8] = [-1, 0, 1, 1, 1, 0, -1, -1];

        ctx.progress.info("running find patch edge cells");
        let mut output = build_output_like_raster(&input, DataType::F64);
        for idx in 0..output.data.len() {
            output.data.set_f64(idx, nodata);
        }

        for b in 0..bands {
            let band_vals: Vec<f64> = (0..stride)
                .into_par_iter()
                .map(|cell_idx| {
                    let row = cell_idx / cols;
                    let col = cell_idx % cols;
                    let idx = b * stride + cell_idx;
                    let z = input.data.get_f64(idx);
                    if !is_patch_value(z, nodata) {
                        return nodata;
                    }
                    let mut is_edge = false;
                    for n in 0..8 {
                        let rr = row as isize + dy[n];
                        let cc = col as isize + dx[n];
                        let zn = if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                            nodata
                        } else {
                            let nidx = b * stride + rr as usize * cols + cc as usize;
                            input.data.get_f64(nidx)
                        };
                        if zn != z {
                            is_edge = true;
                            break;
                        }
                    }
                    if is_edge { z } else { 0.0 }
                })
                .collect();
            for (cell_idx, value) in band_vals.iter().enumerate() {
                output.data.set_f64(b * stride + cell_idx, *value);
            }
            ctx.progress.progress((b + 1) as f64 / bands.max(1) as f64);
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        Ok(GisOverlayCore::build_result(locator))
    }
}

impl Tool for EdgeProportionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "edge_proportion",
            display_name: "Edge Proportion",
            summary: "Calculates the proportion of each patch's cells that are edge cells and maps it back to patch cells.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input patch-ID raster with positive integer-like IDs.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("patches.tif"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("edge_proportion.tif"));
        ToolManifest {
            id: "edge_proportion".to_string(),
            display_name: "Edge Proportion".to_string(),
            summary: "Calculates the proportion of each patch's cells that are edge cells and maps it back to patch cells.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input patch-ID raster with positive integer-like IDs.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "edge_proportion_basic".to_string(),
                description: "Computes per-patch edge proportion.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "patch".to_string(), "edge".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_required_raster_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let rows = input.rows;
        let cols = input.cols;
        let nodata = input.nodata;
        let (raw_min, raw_max) = min_max_valid(&input)
            .ok_or_else(|| ToolError::Validation("input raster does not contain valid patch IDs".to_string()))?;
        let min_val = raw_min.floor();
        let max_val = raw_max.floor();
        if !min_val.is_finite() || !max_val.is_finite() || max_val < min_val {
            return Err(ToolError::Validation("input raster does not contain valid patch IDs".to_string()));
        }
        let bins = (max_val - min_val + 1.0) as usize;
        let dx: [isize; 8] = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy: [isize; 8] = [-1, 0, 1, 1, 1, 0, -1, -1];

        ctx.progress.info("running edge proportion");
        let mut num_cells = vec![0usize; bins];
        let mut num_edge = vec![0usize; bins];

        for row in 0..rows {
            for col in 0..cols {
                let z = input.get(0, row as isize, col as isize);
                if !is_patch_value(z, nodata) {
                    continue;
                }
                let Some(bin) = patch_bin_index(z, min_val, bins) else { continue; };
                num_cells[bin] += 1;
                let mut is_edge = false;
                for n in 0..8 {
                    let zn = input.get(0, row as isize + dy[n], col as isize + dx[n]);
                    if zn != z {
                        is_edge = true;
                        break;
                    }
                }
                if is_edge {
                    num_edge[bin] += 1;
                }
            }
            ctx.progress.progress((row + 1) as f64 / rows.max(1) as f64 * 0.5);
        }

        let mut edge_prop = vec![nodata; bins];
        for bin in 0..bins {
            if num_cells[bin] > 0 {
                edge_prop[bin] = num_edge[bin] as f64 / num_cells[bin] as f64;
            }
        }

        let mut output = build_output_like_raster(&input, DataType::F64);
        for idx in 0..output.data.len() {
            output.data.set_f64(idx, nodata);
        }
        let out_vals: Vec<f64> = (0..rows * cols)
            .into_par_iter()
            .map(|cell_idx| {
                let row = cell_idx / cols;
                let col = cell_idx % cols;
                let z = input.get(0, row as isize, col as isize);
                if !is_patch_value(z, nodata) {
                    return nodata;
                }
                patch_bin_index(z, min_val, bins)
                    .map(|bin| edge_prop[bin])
                    .unwrap_or(nodata)
            })
            .collect();
        for row in 0..rows {
            let row_offset = row * cols;
            for col in 0..cols {
                output.data.set_f64(row_offset + col, out_vals[row_offset + col]);
            }
            ctx.progress.progress(0.5 + (row + 1) as f64 / rows.max(1) as f64 * 0.5);
        }

        let mut table = String::from("Edge Proportion\nPatch ID\tValue\n");
        for bin in 0..bins {
            if edge_prop[bin] > 0.0 && edge_prop[bin] != nodata {
                table.push_str(&format!("{}\t{}\n", (bin as f64 + min_val).floor(), edge_prop[bin]));
            }
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        Ok(build_raster_result_with_table(locator, table))
    }
}

impl Tool for RadiusOfGyrationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "radius_of_gyration",
            display_name: "Radius Of Gyration",
            summary: "Computes per-patch radius of gyration and maps values back to patch cells.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input patch-ID raster with positive integer-like IDs.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("patches.tif"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("radius_of_gyration.tif"));
        ToolManifest {
            id: "radius_of_gyration".to_string(),
            display_name: "Radius Of Gyration".to_string(),
            summary: "Computes per-patch radius of gyration and maps values back to patch cells.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input patch-ID raster with positive integer-like IDs.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "radius_of_gyration_basic".to_string(),
                description: "Computes patch radius of gyration values.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "patch".to_string(), "shape".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_required_raster_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let rows = input.rows;
        let cols = input.cols;
        let nodata = input.nodata;
        let (raw_min, raw_max) = min_max_valid(&input)
            .ok_or_else(|| ToolError::Validation("input raster does not contain valid patch IDs".to_string()))?;
        let min_val = raw_min.floor();
        let max_val = raw_max.ceil();
        if !min_val.is_finite() || !max_val.is_finite() || max_val < min_val {
            return Err(ToolError::Validation("input raster does not contain valid patch IDs".to_string()));
        }
        let bins = (max_val - min_val + 1.0) as usize;

        let mut sum_col = vec![0usize; bins];
        let mut sum_row = vec![0usize; bins];
        let mut count = vec![0usize; bins];

        ctx.progress.info("running radius of gyration");
        for row in 0..rows {
            for col in 0..cols {
                let z = input.get(0, row as isize, col as isize);
                if !is_patch_value(z, nodata) {
                    continue;
                }
                let Some(bin) = patch_bin_index(z, min_val, bins) else { continue; };
                sum_col[bin] += col;
                sum_row[bin] += row;
                count[bin] += 1;
            }
            ctx.progress.progress((row + 1) as f64 / rows.max(1) as f64 * 0.33);
        }

        let mut cx = vec![0.0f64; bins];
        let mut cy = vec![0.0f64; bins];
        for bin in 0..bins {
            if count[bin] > 0 {
                cx[bin] = sum_col[bin] as f64 / count[bin] as f64;
                cy[bin] = sum_row[bin] as f64 / count[bin] as f64;
            }
        }

        let mut gyr = vec![0.0f64; bins];
        for row in 0..rows {
            let mut row_last = vec![0.0f64; bins];
            for col in 0..cols {
                let z = input.get(0, row as isize, col as isize);
                if !is_patch_value(z, nodata) {
                    continue;
                }
                let Some(bin) = patch_bin_index(z, min_val, bins) else { continue; };
                let dx = (col as f64 - cx[bin]) * input.cell_size_x;
                let dy = (row as f64 - cy[bin]) * input.cell_size_y;
                row_last[bin] = dx * dx + dy * dy;
            }
            for bin in 0..bins {
                gyr[bin] += row_last[bin];
            }
            ctx.progress.progress(0.33 + (row + 1) as f64 / rows.max(1) as f64 * 0.33);
        }

        for bin in 0..bins {
            if count[bin] > 0 && gyr[bin] > 0.0 {
                gyr[bin] = (gyr[bin] / count[bin] as f64).sqrt();
            }
        }

        let mut output = build_output_like_raster(&input, DataType::F64);
        let out_vals: Vec<f64> = (0..rows * cols)
            .into_par_iter()
            .map(|cell_idx| {
                let row = cell_idx / cols;
                let col = cell_idx % cols;
                let z = input.get(0, row as isize, col as isize);
                if is_patch_value(z, nodata) {
                    patch_bin_index(z, min_val, bins)
                        .map(|bin| gyr[bin])
                        .unwrap_or(z)
                } else {
                    z
                }
            })
            .collect();
        for row in 0..rows {
            let row_offset = row * cols;
            for col in 0..cols {
                output.data.set_f64(row_offset + col, out_vals[row_offset + col]);
            }
            ctx.progress.progress(0.66 + (row + 1) as f64 / rows.max(1) as f64 * 0.34);
        }

        let mut table = String::from("Patch Radius of Gyration\nPatch ID\tValue\n");
        for bin in 0..bins {
            if count[bin] > 0 {
                table.push_str(&format!("{:.0}\t{:.4}\n", bin as f64 + min_val, gyr[bin]));
            }
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        Ok(build_raster_result_with_table(locator, table))
    }
}

impl Tool for PolygonAreaTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "polygon_area",
            display_name: "Polygon Area",
            summary: "Calculates polygon area and appends an AREA attribute field.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input polygon vector layer.", required: true },
                ToolParamSpec { name: "output", description: "Output vector path.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("polygons.shp"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("polygon_area.shp"));

        ToolManifest {
            id: "polygon_area".to_string(),
            display_name: "Polygon Area".to_string(),
            summary: "Calculates polygon area and appends an AREA attribute field.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input polygon vector layer.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output vector path.".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "polygon_area_basic".to_string(),
                description: "Calculates polygon area values and appends AREA.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "polygon".to_string(),
                "area".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;

        let mut output = input.clone();
        let mut schema = output.schema.clone();
        if schema.field_index("AREA").is_none() {
            schema.add_field(wbvector::FieldDef::new("AREA", wbvector::FieldType::Float));
            output.schema = schema;
        }

        let total = output.features.len().max(1);
        let completed = AtomicUsize::new(0);
        let values: Vec<Result<f64, ToolError>> = output
            .features
            .par_iter()
            .map(|feature| {
                let area = if let Some(geometry) = &feature.geometry {
                    get_polygon_area(geometry).ok_or_else(|| {
                        ToolError::Validation(
                            "input layer must contain polygon geometries".to_string(),
                        )
                    })?
                } else {
                    -999.0
                };
                let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                ctx.progress.progress(done as f64 / total as f64);
                Ok(area)
            })
            .collect();
        for (feature, value) in output.features.iter_mut().zip(values) {
            feature.attributes.push(wbvector::FieldValue::Float(value?));
        }

        let locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(locator))
    }
}

fn polygon_axis_line_from_geometry(
    geometry: &wbvector::Geometry,
    long_axis: bool,
) -> Result<Vec<wbvector::Coord>, ToolError> {
    match geometry {
        wbvector::Geometry::Polygon { .. } | wbvector::Geometry::MultiPolygon(_) => {}
        _ => {
            return Err(ToolError::Validation(
                "input layer must contain polygon geometries".to_string(),
            ));
        }
    }

    let mut coords = Vec::<wbvector::Coord>::new();
    collect_all_coords_from_geometry(geometry, &mut coords);
    let mbb = minimum_bounding_box_coords(&coords, MbbCriterion::Area, f64::EPSILON)
        .ok_or_else(|| ToolError::Execution("failed to compute minimum bounding box".to_string()))?;

    if mbb.len() < 4 {
        return Err(ToolError::Execution(
            "minimum bounding box returned insufficient vertices".to_string(),
        ));
    }

    let p0 = mbb[0].clone();
    let p1 = mbb[1].clone();
    let p2 = mbb[2].clone();
    let p3 = mbb[3].clone();

    let d01 = ((p1.x - p0.x) * (p1.x - p0.x) + (p1.y - p0.y) * (p1.y - p0.y)).sqrt();
    let d12 = ((p2.x - p1.x) * (p2.x - p1.x) + (p2.y - p1.y) * (p2.y - p1.y)).sqrt();

    let centre = wbvector::Coord::xy(
        (p0.x + p1.x + p2.x + p3.x) / 4.0,
        (p0.y + p1.y + p2.y + p3.y) / 4.0,
    );

    let (a, b) = if (long_axis && d01 >= d12) || (!long_axis && d01 <= d12) {
        (p0, p1)
    } else {
        (p1, p2)
    };

    let mid = wbvector::Coord::xy((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    let tx = centre.x - mid.x;
    let ty = centre.y - mid.y;

    Ok(vec![
        wbvector::Coord::xy(a.x + tx, a.y + ty),
        wbvector::Coord::xy(b.x + tx, b.y + ty),
    ])
}

impl Tool for PolygonPerimeterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "polygon_perimeter",
            display_name: "Polygon Perimeter",
            summary: "Calculates polygon perimeter and appends a PERIMETER attribute field.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input polygon vector layer.", required: true },
                ToolParamSpec { name: "output", description: "Output vector path.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("polygons.shp"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("polygon_perimeter.shp"));

        ToolManifest {
            id: "polygon_perimeter".to_string(),
            display_name: "Polygon Perimeter".to_string(),
            summary: "Calculates polygon perimeter and appends a PERIMETER attribute field.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input polygon vector layer.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output vector path.".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "polygon_perimeter_basic".to_string(),
                description: "Calculates polygon perimeter values and appends PERIMETER.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "polygon".to_string(),
                "perimeter".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;

        let mut output = input.clone();
        let mut schema = output.schema.clone();
        if schema.field_index("PERIMETER").is_none() {
            schema.add_field(wbvector::FieldDef::new("PERIMETER", wbvector::FieldType::Float));
            output.schema = schema;
        }

        let total = output.features.len().max(1);
        let completed = AtomicUsize::new(0);
        let values: Vec<Result<f64, ToolError>> = output
            .features
            .par_iter()
            .map(|feature| {
                let perimeter = if let Some(geometry) = &feature.geometry {
                    get_polygon_perimeter(geometry).ok_or_else(|| {
                        ToolError::Validation(
                            "input layer must contain polygon geometries".to_string(),
                        )
                    })?
                } else {
                    -999.0
                };
                let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                ctx.progress.progress(done as f64 / total as f64);
                Ok(perimeter)
            })
            .collect();
        for (feature, value) in output.features.iter_mut().zip(values) {
            feature.attributes.push(wbvector::FieldValue::Float(value?));
        }

        let locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(locator))
    }
}

impl Tool for PolygonShortAxisTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "polygon_short_axis",
            display_name: "Polygon Short Axis",
            summary: "Maps the short axis of each polygon feature's minimum bounding box as line output.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input polygon vector layer.", required: true },
                ToolParamSpec { name: "output", description: "Output polyline vector path.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("polygons.shp"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("polygon_short_axis.shp"));

        ToolManifest {
            id: "polygon_short_axis".to_string(),
            display_name: "Polygon Short Axis".to_string(),
            summary: "Maps the short axis of each polygon feature's minimum bounding box as line output.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input polygon vector layer.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output polyline vector path.".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "polygon_short_axis_basic".to_string(),
                description: "Outputs the short axis line for each polygon feature.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "polygon".to_string(),
                "axis".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;

        let mut output = wbvector::Layer::new(format!("{}_polygon_short_axis", input.name));
        output.schema = input.schema.clone();
        output.crs = input.crs.clone();
        output.geom_type = Some(wbvector::GeometryType::LineString);

        let total = input.features.len().max(1);
        let completed = AtomicUsize::new(0);
        let results: Vec<Result<Option<wbvector::Feature>, ToolError>> = input
            .features
            .par_iter()
            .enumerate()
            .map(|(index, feature)| {
                let out = if let Some(geometry) = &feature.geometry {
                    let axis = polygon_axis_line_from_geometry(geometry, true)?;
                    Some(wbvector::Feature {
                        fid: if feature.fid == 0 { (index + 1) as u64 } else { feature.fid },
                        geometry: Some(wbvector::Geometry::LineString(axis)),
                        attributes: feature.attributes.clone(),
                    })
                } else {
                    None
                };
                let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                ctx.progress.progress(done as f64 / total as f64);
                Ok(out)
            })
            .collect();
        for result in results {
            if let Some(feat) = result? {
                output.push(feat);
            }
        }

        let locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(locator))
    }
}

impl Tool for PolygonLongAxisTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "polygon_long_axis",
            display_name: "Polygon Long Axis",
            summary: "Maps the long axis of each polygon feature's minimum bounding box as line output.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input polygon vector layer.", required: true },
                ToolParamSpec { name: "output", description: "Output polyline vector path.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("polygons.shp"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("polygon_long_axis.shp"));

        ToolManifest {
            id: "polygon_long_axis".to_string(),
            display_name: "Polygon Long Axis".to_string(),
            summary: "Maps the long axis of each polygon feature's minimum bounding box as line output.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input polygon vector layer.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output polyline vector path.".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "polygon_long_axis_basic".to_string(),
                description: "Outputs the long axis line for each polygon feature.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "polygon".to_string(),
                "axis".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;

        let mut output = wbvector::Layer::new(format!("{}_polygon_long_axis", input.name));
        output.schema = input.schema.clone();
        output.crs = input.crs.clone();
        output.geom_type = Some(wbvector::GeometryType::LineString);

        let total = input.features.len().max(1);
        let completed = AtomicUsize::new(0);
        let results: Vec<Result<Option<wbvector::Feature>, ToolError>> = input
            .features
            .par_iter()
            .enumerate()
            .map(|(index, feature)| {
                let out = if let Some(geometry) = &feature.geometry {
                    let axis = polygon_axis_line_from_geometry(geometry, false)?;
                    Some(wbvector::Feature {
                        fid: if feature.fid == 0 { (index + 1) as u64 } else { feature.fid },
                        geometry: Some(wbvector::Geometry::LineString(axis)),
                        attributes: feature.attributes.clone(),
                    })
                } else {
                    None
                };
                let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                ctx.progress.progress(done as f64 / total as f64);
                Ok(out)
            })
            .collect();
        for result in results {
            if let Some(feat) = result? {
                output.push(feat);
            }
        }

        let locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(locator))
    }
}

impl Tool for ShapeComplexityIndexRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "shape_complexity_index_raster",
            display_name: "Shape Complexity Index Raster",
            summary: "Computes raster patch shape complexity from horizontal/vertical transition frequency normalized by patch span.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input patch-ID raster with positive integer-like IDs.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("patches.tif"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("shape_complexity_index_raster.tif"));
        ToolManifest {
            id: "shape_complexity_index_raster".to_string(),
            display_name: "Shape Complexity Index Raster".to_string(),
            summary: "Computes raster patch shape complexity from horizontal/vertical transition frequency normalized by patch span.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input patch-ID raster with positive integer-like IDs.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "shape_complexity_index_raster_basic".to_string(),
                description: "Computes per-patch raster shape complexity values.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "patch".to_string(), "shape".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_required_raster_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let rows = input.rows;
        let cols = input.cols;
        let nodata = input.nodata;
        let (min_val, max_val) = min_max_valid(&input)
            .ok_or_else(|| ToolError::Validation("input raster does not contain valid patch IDs".to_string()))?;
        if !min_val.is_finite() || !max_val.is_finite() || max_val < min_val {
            return Err(ToolError::Validation("input raster does not contain valid patch IDs".to_string()));
        }
        let bins = (max_val - min_val + 0.00001).ceil() as usize;
        if bins == 0 {
            return Err(ToolError::Validation("input raster does not contain valid patch IDs".to_string()));
        }

        ctx.progress.info("running shape complexity index raster");
        let mut freq = vec![0usize; bins];
        let mut min_row = vec![isize::MAX; bins];
        let mut max_row = vec![isize::MIN; bins];
        let mut min_col = vec![isize::MAX; bins];
        let mut max_col = vec![isize::MIN; bins];

        for row in 0..rows {
            for col in 0..cols {
                let val = input.get(0, row as isize, col as isize);
                if !is_patch_value(val, nodata) || val < min_val || val > max_val {
                    continue;
                }
                let Some(bin) = patch_bin_index(val, min_val, bins) else { continue; };
                let left = input.get(0, row as isize, col as isize - 1);
                if val != left {
                    freq[bin] += 1;
                }
                min_row[bin] = min_row[bin].min(row as isize);
                max_row[bin] = max_row[bin].max(row as isize);
                min_col[bin] = min_col[bin].min(col as isize);
                max_col[bin] = max_col[bin].max(col as isize);
            }
            ctx.progress.progress((row + 1) as f64 / rows.max(1) as f64 * 0.4);
        }

        for col in 0..cols {
            for row in 0..rows {
                let val = input.get(0, row as isize, col as isize);
                if !is_patch_value(val, nodata) || val < min_val || val > max_val {
                    continue;
                }
                let up = input.get(0, row as isize - 1, col as isize);
                if val != up {
                    if let Some(bin) = patch_bin_index(val, min_val, bins) {
                        freq[bin] += 1;
                    }
                }
            }
            ctx.progress.progress(0.4 + (col + 1) as f64 / cols.max(1) as f64 * 0.2);
        }

        let mut idx_vals = vec![0.0f64; bins];
        for bin in 1..bins {
            if freq[bin] > 0 {
                let denom = (max_row[bin] - min_row[bin] + 1) + (max_col[bin] - min_col[bin] + 1);
                if denom > 0 {
                    idx_vals[bin] = freq[bin] as f64 / denom as f64;
                }
            }
        }

        let mut output = build_output_like_raster(&input, DataType::F64);
        output.nodata = -999.0;
        let out_nodata = output.nodata;
        for row in 0..rows {
            for col in 0..cols {
                let val = input.get(0, row as isize, col as isize);
                let out_idx = output.index(0, row as isize, col as isize).ok_or_else(|| ToolError::Execution("index out of bounds".to_string()))?;
                if is_patch_value(val, nodata) && val >= min_val && val <= max_val {
                    if let Some(bin) = patch_bin_index(val, min_val, bins) {
                        output.data.set_f64(out_idx, idx_vals[bin]);
                    } else {
                        output.data.set_f64(out_idx, out_nodata);
                    }
                } else if val == 0.0 {
                    output.data.set_f64(out_idx, 0.0);
                } else {
                    output.data.set_f64(out_idx, out_nodata);
                }
            }
            ctx.progress.progress(0.6 + (row + 1) as f64 / rows.max(1) as f64 * 0.4);
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        Ok(GisOverlayCore::build_result(locator))
    }
}

impl Tool for BoundaryShapeComplexityTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "boundary_shape_complexity",
            display_name: "Boundary Shape Complexity",
            summary: "Calculates raster patch boundary-shape complexity using a line-thinned skeleton branch metric.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input patch-ID raster with positive integer-like IDs.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("patches.tif"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("boundary_shape_complexity.tif"));
        ToolManifest {
            id: "boundary_shape_complexity".to_string(),
            display_name: "Boundary Shape Complexity".to_string(),
            summary: "Calculates raster patch boundary-shape complexity using a line-thinned skeleton branch metric.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input patch-ID raster with positive integer-like IDs.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "boundary_shape_complexity_basic".to_string(),
                description: "Computes boundary complexity scores for raster patches.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "patch".to_string(), "shape".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_required_raster_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_required_raster_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let rows = input.rows;
        let cols = input.cols;
        let nodata = input.nodata;
        let (min_val, max_val) = min_max_valid(&input)
            .ok_or_else(|| ToolError::Validation("input raster does not contain valid patch IDs".to_string()))?;
        let bins = (max_val - min_val + 0.00001).ceil() as usize;
        if bins == 0 {
            return Err(ToolError::Validation("input raster does not contain valid patch IDs".to_string()));
        }

        ctx.progress.info("running boundary shape complexity");
        let mut skeleton = vec![0.0f64; rows * cols];
        for row in 0..rows {
            for col in 0..cols {
                let z = input.get(0, row as isize, col as isize);
                let idx = row * cols + col;
                if is_patch_value(z, nodata) {
                    skeleton[idx] = 1.0;
                } else if z == 0.0 {
                    skeleton[idx] = 0.0;
                } else {
                    skeleton[idx] = -999.0;
                }
            }
        }

        let dx: [isize; 8] = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy: [isize; 8] = [-1, 0, 1, 1, 1, 0, -1, -1];
        let elements1: [[usize; 6]; 4] = [[6, 7, 0, 4, 3, 2], [0, 1, 2, 4, 5, 6], [2, 3, 4, 6, 7, 0], [4, 5, 6, 0, 1, 2]];
        let elements2: [[usize; 5]; 4] = [[7, 0, 1, 3, 5], [1, 2, 3, 5, 7], [3, 4, 5, 7, 1], [5, 6, 7, 1, 3]];
        let vals1 = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let vals2 = [0.0, 0.0, 0.0, 1.0, 1.0];

        let mut changed = true;
        while changed {
            changed = false;
            for a in 0..4 {
                for row in 0..rows {
                    for col in 0..cols {
                        let idx = row * cols + col;
                        if skeleton[idx] != 1.0 {
                            continue;
                        }
                        let mut nb = [0.0f64; 8];
                        for i in 0..8 {
                            let rr = row as isize + dy[i];
                            let cc = col as isize + dx[i];
                            nb[i] = if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                                0.0
                            } else {
                                skeleton[rr as usize * cols + cc as usize]
                            };
                        }
                        let mut match1 = true;
                        for i in 0..6 {
                            if nb[elements1[a][i]] != vals1[i] {
                                match1 = false;
                                break;
                            }
                        }
                        let mut do_remove = match1;
                        if !do_remove {
                            let mut match2 = true;
                            for i in 0..5 {
                                if nb[elements2[a][i]] != vals2[i] {
                                    match2 = false;
                                    break;
                                }
                            }
                            do_remove = match2;
                        }
                        if do_remove {
                            skeleton[idx] = 0.0;
                            changed = true;
                        }
                    }
                }
            }
        }

        let mut visited = vec![0i8; rows * cols];
        let dxn: [isize; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];
        let dyns: [isize; 8] = [0, -1, -1, -1, 0, 1, 1, 1];
        let mut num_cells = vec![0usize; bins];
        let mut longest = vec![0usize; bins];
        let mut second_longest = vec![0usize; bins];
        let mut exterior_len = vec![0f64; bins];

        for row in 0..rows {
            for col in 0..cols {
                let sk = skeleton[row * cols + col];
                if sk != 1.0 {
                    continue;
                }
                let polyid = input.get(0, row as isize, col as isize);
                let Some(bin) = patch_bin_index(polyid, min_val, bins) else { continue; };
                num_cells[bin] += 1;
                let mut ncount = 0usize;
                for a in 0..8 {
                    let rr = row as isize + dyns[a];
                    let cc = col as isize + dxn[a];
                    if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                        continue;
                    }
                    let nidx = rr as usize * cols + cc as usize;
                    if skeleton[nidx] == 1.0 && input.get(0, rr, cc) == polyid {
                        ncount += 1;
                    }
                }
                if ncount != 1 {
                    continue;
                }

                let mut rr = row as isize;
                let mut cc = col as isize;
                let mut link_len = 1usize;
                loop {
                    let cidx = rr as usize * cols + cc as usize;
                    visited[cidx] = 1;
                    let mut next_dir = None;
                    let mut local_n = 0usize;
                    for a in 0..8 {
                        let nr = rr + dyns[a];
                        let nc = cc + dxn[a];
                        if nr < 0 || nc < 0 || nr >= rows as isize || nc >= cols as isize {
                            continue;
                        }
                        let nidx = nr as usize * cols + nc as usize;
                        if skeleton[nidx] == 1.0 && input.get(0, nr, nc) == polyid {
                            local_n += 1;
                            if visited[nidx] == 0 {
                                next_dir = Some(a);
                            }
                        }
                    }
                    if local_n < 3 {
                        if let Some(a) = next_dir {
                            rr += dyns[a];
                            cc += dxn[a];
                            link_len += 1;
                            continue;
                        }
                    }
                    break;
                }

                exterior_len[bin] += link_len as f64;
                if link_len > longest[bin] {
                    second_longest[bin] = longest[bin];
                    longest[bin] = link_len;
                } else if link_len > second_longest[bin] {
                    second_longest[bin] = link_len;
                }
            }
            ctx.progress.progress((row + 1) as f64 / rows.max(1) as f64 * 0.8);
        }

        for bin in 1..bins {
            if num_cells[bin] > 0 {
                exterior_len[bin] = 100.0
                    * (exterior_len[bin] - longest[bin] as f64 - second_longest[bin] as f64)
                    / num_cells[bin] as f64;
            }
        }

        let mut output = build_output_like_raster(&input, DataType::F64);
        for row in 0..rows {
            for col in 0..cols {
                let z = input.get(0, row as isize, col as isize);
                let out_idx = output.index(0, row as isize, col as isize).ok_or_else(|| ToolError::Execution("index out of bounds".to_string()))?;
                if z != nodata && z != 0.0 {
                    if let Some(bin) = patch_bin_index(z, min_val, bins) {
                        output.data.set_f64(out_idx, exterior_len[bin]);
                    }
                } else if z == 0.0 {
                    output.data.set_f64(out_idx, 0.0);
                } else {
                    output.data.set_f64(out_idx, nodata);
                }
            }
            ctx.progress.progress(0.8 + (row + 1) as f64 / rows.max(1) as f64 * 0.2);
        }

        let locator = GisOverlayCore::store_or_write_output(output, output_path, ctx)?;
        Ok(GisOverlayCore::build_result(locator))
    }
}

impl Tool for CompactnessRatioTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "compactness_ratio",
            display_name: "Compactness Ratio",
            summary: "Computes compactness ratio (perimeter of equivalent circle / actual perimeter) for polygon features.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input polygon vector layer.", required: true },
                ToolParamSpec { name: "output", description: "Output vector with compactness field appended.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("polygons.shp"));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("compactness_ratio.shp"));

        ToolManifest {
            id: "compactness_ratio".to_string(),
            display_name: "Compactness Ratio".to_string(),
            summary: "Computes compactness ratio (perimeter of equivalent circle / actual perimeter) for polygon features.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input polygon vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector with compactness field appended.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "compactness_ratio_basic".to_string(),
                description: "Computes compactness ratio for polygon features.".to_string(),
                args: example_args,
            }],
            tags: vec!["vector".to_string(), "gis".to_string(), "shape".to_string(), "compactness".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;

        ctx.progress.info("running compactness ratio");
        let mut output = input.clone();
        let mut schema = output.schema.clone();
        if schema.field_index("COMPACTNESS").is_none() {
            schema.add_field(wbvector::FieldDef::new("COMPACTNESS", wbvector::FieldType::Float));
            output.schema = schema;
        }

        let total = output.features.len().max(1);
        let completed = AtomicUsize::new(0);
        let values: Vec<f64> = output
            .features
            .par_iter()
            .map(|feature| {
                let compactness = if let Some(geometry) = &feature.geometry {
                    if let Some(area) = get_polygon_area(geometry) {
                        if let Some(perimeter) = get_polygon_perimeter(geometry) {
                            if area > 0.0 && perimeter > 0.0 {
                                let equiv_circle_perim = 2.0 * (std::f64::consts::PI * area).sqrt() * std::f64::consts::PI.sqrt();
                                equiv_circle_perim / perimeter
                            } else {
                                -999.0
                            }
                        } else {
                            -999.0
                        }
                    } else {
                        -999.0
                    }
                } else {
                    -999.0
                };
                let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                ctx.progress.progress(done as f64 / total as f64);
                compactness
            })
            .collect();
        for (feature, value) in output.features.iter_mut().zip(values) {
            feature.attributes.push(wbvector::FieldValue::Float(value));
        }

        ctx.progress.info("writing output vector");
        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for ElongationRatioTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "elongation_ratio",
            display_name: "Elongation Ratio",
            summary: "Computes elongation ratio (short axis / long axis of bounding rectangle) for polygon features.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input polygon vector layer.", required: true },
                ToolParamSpec { name: "output", description: "Output vector with elongation field appended.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("polygons.shp"));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("elongation_ratio.shp"));

        ToolManifest {
            id: "elongation_ratio".to_string(),
            display_name: "Elongation Ratio".to_string(),
            summary: "Computes elongation ratio (short axis / long axis of bounding rectangle) for polygon features.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input polygon vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector with elongation field appended.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "elongation_ratio_basic".to_string(),
                description: "Computes elongation ratio for polygon features.".to_string(),
                args: example_args,
            }],
            tags: vec!["vector".to_string(), "gis".to_string(), "shape".to_string(), "elongation".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;

        ctx.progress.info("running elongation ratio");
        let mut output = input.clone();
        let mut schema = output.schema.clone();
        if schema.field_index("ELONGATION").is_none() {
            schema.add_field(wbvector::FieldDef::new("ELONGATION", wbvector::FieldType::Float));
            output.schema = schema;
        }

        let total = output.features.len().max(1);
        let completed = AtomicUsize::new(0);
        let values: Vec<f64> = output
            .features
            .par_iter()
            .map(|feature| {
                let elongation = if let Some(geometry) = &feature.geometry {
                    if let Some((min_x, min_y, max_x, max_y)) = get_bounding_box(geometry) {
                        let width = max_x - min_x;
                        let height = max_y - min_y;
                        let (short_axis, long_axis) = if width < height {
                            (width, height)
                        } else {
                            (height, width)
                        };
                        if long_axis > 0.0 {
                            short_axis / long_axis
                        } else {
                            -999.0
                        }
                    } else {
                        -999.0
                    }
                } else {
                    -999.0
                };
                let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                ctx.progress.progress(done as f64 / total as f64);
                elongation
            })
            .collect();
        for (feature, value) in output.features.iter_mut().zip(values) {
            feature.attributes.push(wbvector::FieldValue::Float(value));
        }

        ctx.progress.info("writing output vector");
        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for LinearityIndexTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "linearity_index",
            display_name: "Linearity Index",
            summary: "Computes linearity index (straight-line distance / actual length) for line and polygon features.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input line or polygon vector layer.", required: true },
                ToolParamSpec { name: "output", description: "Output vector with linearity field appended.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("features.shp"));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("linearity_index.shp"));

        ToolManifest {
            id: "linearity_index".to_string(),
            display_name: "Linearity Index".to_string(),
            summary: "Computes linearity index (straight-line distance / actual length) for line and polygon features.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input line or polygon vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector with linearity field appended.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "linearity_index_basic".to_string(),
                description: "Computes linearity index for line/polygon features.".to_string(),
                args: example_args,
            }],
            tags: vec!["vector".to_string(), "gis".to_string(), "shape".to_string(), "linearity".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;

        ctx.progress.info("running linearity index");
        let mut output = input.clone();
        let mut schema = output.schema.clone();
        if schema.field_index("LINEARITY").is_none() {
            schema.add_field(wbvector::FieldDef::new("LINEARITY", wbvector::FieldType::Float));
            output.schema = schema;
        }

        let total = output.features.len().max(1);
        let completed = AtomicUsize::new(0);
        let values: Vec<f64> = output
            .features
            .par_iter()
            .map(|feature| {
                let linearity = if let Some(geometry) = &feature.geometry {
                    if let Some((min_x, min_y, max_x, max_y)) = get_bounding_box(geometry) {
                        let straight_dist = ((max_x - min_x) * (max_x - min_x)
                            + (max_y - min_y) * (max_y - min_y))
                            .sqrt();
                        if let Some(actual_length) = get_geometry_length(geometry) {
                            if actual_length > 0.0 {
                                straight_dist / actual_length
                            } else {
                                -999.0
                            }
                        } else {
                            -999.0
                        }
                    } else {
                        -999.0
                    }
                } else {
                    -999.0
                };
                let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                ctx.progress.progress(done as f64 / total as f64);
                linearity
            })
            .collect();
        for (feature, value) in output.features.iter_mut().zip(values) {
            feature.attributes.push(wbvector::FieldValue::Float(value));
        }

        ctx.progress.info("writing output vector");
        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for NarrownessIndexTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "narrowness_index",
            display_name: "Narrowness Index",
            summary: "Computes narrowness index (perimeter / sqrt(area)) for polygon features.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input polygon vector layer.", required: true },
                ToolParamSpec { name: "output", description: "Output vector with narrowness field appended.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("polygons.shp"));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("narrowness_index.shp"));

        ToolManifest {
            id: "narrowness_index".to_string(),
            display_name: "Narrowness Index".to_string(),
            summary: "Computes narrowness index (perimeter / sqrt(area)) for polygon features.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input polygon vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector with narrowness field appended.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "narrowness_index_basic".to_string(),
                description: "Computes narrowness index for polygon features.".to_string(),
                args: example_args,
            }],
            tags: vec!["vector".to_string(), "gis".to_string(), "shape".to_string(), "narrowness".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;

        ctx.progress.info("running narrowness index");
        let mut output = input.clone();
        let mut schema = output.schema.clone();
        if schema.field_index("NARROWNESS").is_none() {
            schema.add_field(wbvector::FieldDef::new("NARROWNESS", wbvector::FieldType::Float));
            output.schema = schema;
        }

        let total = output.features.len().max(1);
        let completed = AtomicUsize::new(0);
        let values: Vec<f64> = output
            .features
            .par_iter()
            .map(|feature| {
                let narrowness = if let Some(geometry) = &feature.geometry {
                    if let Some(area) = get_polygon_area(geometry) {
                        if let Some(perimeter) = get_polygon_perimeter(geometry) {
                            if area > 0.0 {
                                perimeter / area.sqrt()
                            } else {
                                -999.0
                            }
                        } else {
                            -999.0
                        }
                    } else {
                        -999.0
                    }
                } else {
                    -999.0
                };
                let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                ctx.progress.progress(done as f64 / total as f64);
                narrowness
            })
            .collect();
        for (feature, value) in output.features.iter_mut().zip(values) {
            feature.attributes.push(wbvector::FieldValue::Float(value));
        }

        ctx.progress.info("writing output vector");
        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for ShapeComplexityIndexVectorTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "shape_complexity_index_vector",
            display_name: "Shape Complexity Index Vector",
            summary: "Computes shape complexity index for vector polygon features using normalized form factor.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input polygon vector layer.", required: true },
                ToolParamSpec { name: "output", description: "Output vector with complexity index field appended.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("polygons.shp"));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("shape_complexity_index_vector.shp"));

        ToolManifest {
            id: "shape_complexity_index_vector".to_string(),
            display_name: "Shape Complexity Index Vector".to_string(),
            summary: "Computes shape complexity index for vector polygon features using normalized form factor.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input polygon vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector with complexity index field appended.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "shape_complexity_index_vector_basic".to_string(),
                description: "Computes shape complexity index for vector features.".to_string(),
                args: example_args,
            }],
            tags: vec!["vector".to_string(), "gis".to_string(), "shape".to_string(), "complexity".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = load_vector_arg(args, "input")?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let output_path = parse_vector_path_arg(args, "output")?;

        ctx.progress.info("running shape complexity index vector");
        let mut output = input.clone();
        let mut schema = output.schema.clone();
        if schema.field_index("SCI").is_none() {
            schema.add_field(wbvector::FieldDef::new("SCI", wbvector::FieldType::Float));
            output.schema = schema;
        }

        let total = output.features.len().max(1);
        let completed = AtomicUsize::new(0);
        let values: Vec<f64> = output
            .features
            .par_iter()
            .map(|feature| {
                let sci = if let Some(geometry) = &feature.geometry {
                    match geometry {
                        wbvector::Geometry::Polygon { exterior, .. } => {
                            compute_polygon_complexity(exterior)
                        }
                        wbvector::Geometry::MultiPolygon(parts) => {
                            let mut max_complexity = 0.0;
                            for (exterior, _) in parts {
                                let complexity = compute_polygon_complexity(exterior);
                                if complexity > max_complexity {
                                    max_complexity = complexity;
                                }
                            }
                            max_complexity
                        }
                        _ => -999.0,
                    }
                } else {
                    -999.0
                };
                let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                ctx.progress.progress(done as f64 / total as f64);
                sci
            })
            .collect();
        for (feature, value) in output.features.iter_mut().zip(values) {
            feature.attributes.push(wbvector::FieldValue::Float(value));
        }

        ctx.progress.info("writing output vector");
        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

// ============================================================================
// Travelling Salesman Problem Tool Implementation
// ============================================================================

#[derive(Debug, Clone)]
struct TspPoint {
    x: f64,
    y: f64,
    is_geographic: bool,
}

impl TspPoint {
    fn new(x: f64, y: f64, is_geographic: bool) -> Self {
        TspPoint { x, y, is_geographic }
    }

    fn distance_to(&self, other: &TspPoint) -> f64 {
        if self.is_geographic {
            haversine_distance((self.y, self.x), (other.y, other.x))
        } else {
            let dx = self.x - other.x;
            let dy = self.y - other.y;
            (dx * dx + dy * dy).sqrt()
        }
    }
}

fn haversine_distance(coord1: (f64, f64), coord2: (f64, f64)) -> f64 {
    const EARTH_RADIUS_KM: f64 = 6371.0;
    let (lat1, lon1) = coord1;
    let (lat2, lon2) = coord2;
    
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lon = (lon2 - lon1).to_radians();
    
    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    
    EARTH_RADIUS_KM * c
}

fn calculate_tour_distance(tour: &[usize], points: &[TspPoint]) -> f64 {
    if tour.len() < 2 {
        return 0.0;
    }
    
    let mut total = 0.0;
    for i in 0..tour.len() {
        let from = &points[tour[i]];
        let to = &points[tour[(i + 1) % tour.len()]];
        total += from.distance_to(to);
    }
    total
}

fn two_opt_optimize(tour: &mut Vec<usize>, points: &[TspPoint], max_duration: std::time::Duration) {
    let start_time = std::time::Instant::now();
    let mut improved = true;
    
    while improved && start_time.elapsed() < max_duration {
        improved = false;
        let n = tour.len();
        
        for i in 0..(n - 2) {
            for j in (i + 2)..n {
                // Check if reversing tour[i+1..=j] improves the solution
                let a_idx = tour[i];
                let b_idx = tour[i + 1];
                let c_idx = tour[j];
                let d_idx = tour[(j + 1) % n];
                
                let current_distance =
                    points[a_idx].distance_to(&points[b_idx])
                    + points[c_idx].distance_to(&points[d_idx]);
                
                let new_distance =
                    points[a_idx].distance_to(&points[c_idx])
                    + points[b_idx].distance_to(&points[d_idx]);
                
                if new_distance < current_distance {
                    // Reverse the segment between i+1 and j
                    tour[i + 1..=j].reverse();
                    improved = true;
                    break;
                }
            }
            if improved {
                break;
            }
        }
    }
}

impl Tool for TravellingSalesmanProblemTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "travelling_salesman_problem",
            display_name: "Travelling Salesman Problem",
            summary: "Finds approximate solutions to the travelling salesman problem (TSP) using 2-opt heuristics. Given a set of point locations, identifies the shortest route connecting all points.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input point vector layer.", required: true },
                ToolParamSpec { name: "duration", description: "Maximum optimization duration in seconds.", required: false },
                ToolParamSpec { name: "output", description: "Optional output polyline path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("points.geojson"));
        defaults.insert("duration".to_string(), json!(60));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("tsp_route.geojson"));
        ToolManifest {
            id: "travelling_salesman_problem".to_string(),
            display_name: "Travelling Salesman Problem".to_string(),
            summary: "Finds approximate solutions to the travelling salesman problem (TSP) using 2-opt heuristics. Given a set of point locations, identifies the shortest route connecting all points.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input point vector layer.".to_string(), required: true },
                ToolParamDescriptor { name: "duration".to_string(), description: "Maximum optimization duration in seconds.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output polyline path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "travelling_salesman_problem_basic".to_string(),
                description: "Finds optimal route through point locations.".to_string(),
                args: example_args,
            }],
            tags: vec!["vector".to_string(), "gis".to_string(), "optimization".to_string(), "routing".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let input = load_vector_arg(args, "input")?;
        if let Some(geom_type) = input.geom_type {
            if geom_type != wbvector::GeometryType::Point && geom_type != wbvector::GeometryType::MultiPoint {
                return Err(ToolError::Validation(
                    "Input must be a point or multipoint vector layer".to_string(),
                ));
            }
        }
        if input.features.is_empty() {
            return Err(ToolError::Validation("Input layer has no features".to_string()));
        }
        let _ = args
            .get("duration")
            .map(|v| v.as_i64().and_then(|d| if d > 0 { Some(d as u64) } else { None })
                .ok_or_else(|| ToolError::Validation("duration must be a positive integer".to_string())))
            .transpose()?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input")?;
        let duration = args
            .get("duration")
            .and_then(|v| v.as_i64())
            .unwrap_or(60)
            .max(1) as u64;
        let output_path = parse_vector_path_arg(args, "output")?;

        ctx.progress.info("running travelling salesman problem");

        // Determine if using geographic projection by checking coordinate bounds
        let is_geographic = {
            let mut has_coords = false;
            let mut min_x = f64::INFINITY;
            let mut max_x = f64::NEG_INFINITY;
            let mut min_y = f64::INFINITY;
            let mut max_y = f64::NEG_INFINITY;
            
            for feature in &input.features {
                if let Some(geometry) = &feature.geometry {
                    match geometry {
                        wbvector::Geometry::Point(coord) => {
                            has_coords = true;
                            min_x = min_x.min(coord.x);
                            max_x = max_x.max(coord.x);
                            min_y = min_y.min(coord.y);
                            max_y = max_y.max(coord.y);
                        }
                        wbvector::Geometry::MultiPoint(coords) => {
                            for coord in coords {
                                has_coords = true;
                                min_x = min_x.min(coord.x);
                                max_x = max_x.max(coord.x);
                                min_y = min_y.min(coord.y);
                                max_y = max_y.max(coord.y);
                            }
                        }
                        _ => {}
                    }
                }
            }
            
            has_coords && min_x.abs() <= 180.0 && max_x.abs() <= 180.0 && min_y.abs() <= 90.0 && max_y.abs() <= 90.0
        };

        // Extract points from input layer
        let mut points = Vec::new();
        for feature in &input.features {
            if let Some(geometry) = &feature.geometry {
                match geometry {
                    wbvector::Geometry::Point(coord) => {
                        points.push(TspPoint::new(coord.x, coord.y, is_geographic));
                    }
                    wbvector::Geometry::MultiPoint(coords) => {
                        for coord in coords {
                            points.push(TspPoint::new(coord.x, coord.y, is_geographic));
                        }
                    }
                    _ => {}
                }
            }
        }

        if points.len() < 2 {
            return Err(ToolError::Execution(
                "Need at least 2 points for TSP".to_string(),
            ));
        }

        let msg = format!("found {} points", points.len());
        ctx.progress.info(&msg);

        // Create initial tour (sequential order)
        let mut best_tour: Vec<usize> = (0..points.len()).collect();
        let initial_distance = calculate_tour_distance(&best_tour, &points);
        let msg = format!("initial tour distance: {:.3}", initial_distance);
        ctx.progress.info(&msg);

        // Optimize in parallel threads
        let num_threads = rayon::current_num_threads();
        let optimization_duration = std::time::Duration::from_secs(duration);
        
        let results = (0..num_threads)
            .into_par_iter()
            .map(|thread_id| {
                let mut tour = best_tour.clone();
                let mut rng = rand::rng();
                use rand::seq::SliceRandom;
                
                // Randomize initial tour for this thread
                if thread_id > 0 {
                    tour.shuffle(&mut rng);
                }
                
                let mut best_local_distance = calculate_tour_distance(&tour, &points);
                
                // Run 2-opt optimization with random restarts
                let start = std::time::Instant::now();
                while start.elapsed() < optimization_duration {
                    two_opt_optimize(&mut tour, &points, std::time::Duration::from_millis(100));
                    let current_distance = calculate_tour_distance(&tour, &points);
                    if current_distance < best_local_distance {
                        best_local_distance = current_distance;
                    }
                }
                
                (tour, best_local_distance)
            })
            .collect::<Vec<_>>();

        // Find the best tour from all threads
        for (tour, distance) in results {
            if distance < calculate_tour_distance(&best_tour, &points) {
                best_tour = tour;
            }
        }

        let final_distance = calculate_tour_distance(&best_tour, &points);
        let msg = format!("optimized tour distance: {:.3}", final_distance);
        ctx.progress.info(&msg);

        // Create output polyline layer
        let mut output_layer = wbvector::Layer::new("tour").with_geom_type(wbvector::GeometryType::LineString);
        output_layer.schema = input.schema.clone();
        
        // Add FID and LENGTH fields if not present
        if output_layer.schema.field_index("FID").is_none() {
            output_layer.schema.add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));
        }
        if output_layer.schema.field_index("LENGTH").is_none() {
            output_layer.schema.add_field(wbvector::FieldDef::new("LENGTH", wbvector::FieldType::Float));
        }
        
        // Copy CRS if available
        if let Some(crs) = &input.crs {
            output_layer.crs = Some(crs.clone());
        }

        // Create closed polyline from tour
        let mut line_coords = Vec::new();
        for &point_idx in &best_tour {
            let point = &points[point_idx];
            line_coords.push(wbvector::Coord::xy(point.x, point.y));
        }
        // Close the loop
        if !line_coords.is_empty() {
            let first = line_coords[0].clone();
            line_coords.push(first);
        }

        // Create feature
        let geometry = wbvector::Geometry::LineString(line_coords);
        let num_schema_fields = output_layer.schema.fields().len();
        let mut attributes = vec![
            wbvector::FieldValue::Integer(1i64),
            wbvector::FieldValue::Float(final_distance),
        ];
        
        // Add any remaining fields from input schema
        for _ in 2..num_schema_fields {
            attributes.push(wbvector::FieldValue::Null);
        }

        let feature = wbvector::Feature {
            fid: 1,
            geometry: Some(geometry),
            attributes,
        };
        output_layer.features.push(feature);

        ctx.progress.info("writing output vector");
        let output_locator = write_vector_output(&output_layer, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for ConstructVectorTinTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "construct_vector_tin",
            display_name: "Construct Vector TIN",
            summary:
                "Constructs a triangular irregular network (TIN) from an input point set using Delaunay triangulation.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input_points",
                    description: "Input point or multipoint vector layer.",
                    required: true,
                },
                ToolParamSpec {
                    name: "field_name",
                    description: "Optional field name for vertex heights (numeric field).",
                    required: false,
                },
                ToolParamSpec {
                    name: "max_triangle_edge_length",
                    description:
                        "Maximum allowable triangle edge length (filters long edges). Use 0 or negative for no limit.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output polygon vector layer (TIN).",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input_points".to_string(), json!("points.shp"));
        defaults.insert("field_name".to_string(), json!("FID"));
        defaults.insert("max_triangle_edge_length".to_string(), json!(-1.0));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("construct_vector_tin.shp"));

        ToolManifest {
            id: "construct_vector_tin".to_string(),
            display_name: "Construct Vector TIN".to_string(),
            summary:
                "Constructs a triangular irregular network (TIN) from an input point set using Delaunay triangulation."
                    .to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input_points".to_string(),
                    description: "Input point or multipoint vector layer.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "field_name".to_string(),
                    description: "Optional field name for vertex heights (numeric field)."
                        .to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "max_triangle_edge_length".to_string(),
                    description:
                        "Maximum allowable triangle edge length (filters long edges). Use 0 or negative for no limit."
                            .to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output polygon vector layer (TIN).".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "construct_vector_tin_basic".to_string(),
                description: "Builds a TIN from input points.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "tin".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let input = load_vector_arg(args, "input_points")?;
        if let Some(geom_type) = input.geom_type {
            if geom_type != wbvector::GeometryType::Point
                && geom_type != wbvector::GeometryType::MultiPoint
            {
                return Err(ToolError::Validation(
                    "input_points vector data must be of POINT or MULTIPOINT base shape type"
                        .to_string(),
                ));
            }
        }

        let field_name = args
            .get("field_name")
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .unwrap_or("FID");
        if !field_name.eq_ignore_ascii_case("FID") {
            let field_idx = input.schema.field_index(field_name).ok_or_else(|| {
                ToolError::Validation("field_name must exist in input_points schema".to_string())
            })?;
            let field = input.schema.fields().get(field_idx).ok_or_else(|| {
                ToolError::Validation("field_name must exist in input_points schema".to_string())
            })?;
            if !matches!(field.field_type, wbvector::FieldType::Integer | wbvector::FieldType::Float)
            {
                return Err(ToolError::Validation(
                    "field_name must reference a numeric field".to_string(),
                ));
            }
        }

        if let Some(max_len) = args
            .get("max_triangle_edge_length")
            .and_then(|v| v.as_f64())
        {
            if !max_len.is_finite() {
                return Err(ToolError::Validation(
                    "max_triangle_edge_length must be a finite value".to_string(),
                ));
            }
        }

        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "input_points")?;
        let field_name = args
            .get("field_name")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "FID".to_string());
        let max_triangle_edge_length = args
            .get("max_triangle_edge_length")
            .and_then(|v| v.as_f64())
            .unwrap_or(-1.0);
        let output_path = parse_vector_path_arg(args, "output")?;

        let field_idx = if field_name.eq_ignore_ascii_case("FID") {
            None
        } else {
            Some(
                input
                    .schema
                    .field_index(&field_name)
                    .ok_or_else(|| {
                        ToolError::Validation(
                            "field_name must exist in input_points schema".to_string(),
                        )
                    })?,
            )
        };

        let mut points = Vec::<TopoCoord>::new();
        let mut z_values = Vec::<f64>::new();

        let total_features = input.features.len().max(1);
        for (idx, feature) in input.features.iter().enumerate() {
            let Some(geometry) = feature.geometry.as_ref() else {
                ctx.progress.progress((idx + 1) as f64 / total_features as f64);
                continue;
            };

            let z_value = if let Some(field_idx) = field_idx {
                feature
                    .attributes
                    .get(field_idx)
                    .and_then(|value| value.as_f64())
                    .unwrap_or(feature.fid as f64)
            } else {
                feature.fid as f64
            };

            match geometry {
                wbvector::Geometry::Point(coord) => {
                    points.push(TopoCoord::xy(coord.x, coord.y));
                    z_values.push(z_value);
                }
                wbvector::Geometry::MultiPoint(coords) => {
                    for coord in coords {
                        points.push(TopoCoord::xy(coord.x, coord.y));
                        z_values.push(z_value);
                    }
                }
                _ => {
                    return Err(ToolError::Validation(
                        "input_points vector data must contain only point geometries"
                            .to_string(),
                    ));
                }
            }

            ctx.progress.progress((idx + 1) as f64 / total_features as f64);
        }

        if points.len() < 3 {
            return Err(ToolError::Validation(
                "input_points layer must contain at least three points".to_string(),
            ));
        }

        let triangulation = delaunay_triangulation(&points, 1.0e-9);
        let max_edge_len_sq = if max_triangle_edge_length > 0.0 {
            max_triangle_edge_length * max_triangle_edge_length
        } else {
            f64::INFINITY
        };

        let mut output = wbvector::Layer::new(format!("{}_tin", input.name));
        output.geom_type = Some(wbvector::GeometryType::Polygon);
        output.crs = input.crs.clone();
        output
            .schema
            .add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));

        let total_triangles = triangulation.triangles.len().max(1);
        let mut next_fid: u64 = 1;
        for (tri_idx, tri) in triangulation.triangles.iter().enumerate() {
            let p1 = tri[0];
            let p2 = tri[1];
            let p3 = tri[2];

            let c1 = &points[p1];
            let c2 = &points[p2];
            let c3 = &points[p3];
            let max_edge_sq = max_distance_squared(
                (c1.x, c1.y),
                (c2.x, c2.y),
                (c3.x, c3.y),
                z_values[p1],
                z_values[p2],
                z_values[p3],
            );
            if max_edge_sq > max_edge_len_sq {
                ctx.progress.progress((tri_idx + 1) as f64 / total_triangles as f64);
                continue;
            }

            let ring = vec![
                wbvector::Coord::xy(c1.x, c1.y),
                wbvector::Coord::xy(c2.x, c2.y),
                wbvector::Coord::xy(c3.x, c3.y),
                wbvector::Coord::xy(c1.x, c1.y),
            ];
            output.push(wbvector::Feature {
                fid: next_fid,
                geometry: Some(wbvector::Geometry::polygon(ring, vec![])),
                attributes: vec![wbvector::FieldValue::Integer(next_fid as i64)],
            });

            next_fid += 1;
            ctx.progress.progress((tri_idx + 1) as f64 / total_triangles as f64);
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}

impl Tool for VectorHexBinningTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "vector_hex_binning",
            display_name: "Vector Hex Binning",
            summary:
                "Aggregates point features into hexagonal bins, counting points per hex cell.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "vector_points",
                    description: "Input point or multipoint vector layer.",
                    required: true,
                },
                ToolParamSpec {
                    name: "width",
                    description: "Hexagon width (distance between opposing sides).",
                    required: true,
                },
                ToolParamSpec {
                    name: "orientation",
                    description:
                        "Hexagon orientation: 'h' for horizontal (pointy-top) or 'v' for vertical (flat-top).",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output hexagon polygon layer with counts.",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("vector_points".to_string(), json!("points.shp"));
        defaults.insert("width".to_string(), json!(100.0));
        defaults.insert("orientation".to_string(), json!("h"));
        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("vector_hex_binning.shp"));

        ToolManifest {
            id: "vector_hex_binning".to_string(),
            display_name: "Vector Hex Binning".to_string(),
            summary:
                "Aggregates point features into hexagonal bins, counting points per hex cell.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "vector_points".to_string(),
                    description: "Input point or multipoint vector layer.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "width".to_string(),
                    description: "Hexagon width (distance between opposing sides).".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "orientation".to_string(),
                    description:
                        "Hexagon orientation: 'h' for horizontal (pointy-top) or 'v' for vertical (flat-top)."
                            .to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Output hexagon polygon layer with counts.".to_string(),
                    required: true,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "vector_hex_binning_basic".to_string(),
                description: "Bins points into a hexagonal tessellation.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "gis".to_string(),
                "binning".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let input = load_vector_arg(args, "vector_points")?;
        if let Some(geom_type) = input.geom_type {
            if geom_type != wbvector::GeometryType::Point
                && geom_type != wbvector::GeometryType::MultiPoint
            {
                return Err(ToolError::Validation(
                    "vector_points vector data must be of POINT or MULTIPOINT base shape type"
                        .to_string(),
                ));
            }
        }

        let width = args
            .get("width")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'width' is required".to_string()))?;
        if !width.is_finite() || width <= 0.0 {
            return Err(ToolError::Validation(
                "width must be a finite value > 0".to_string(),
            ));
        }

        let orientation = args
            .get("orientation")
            .and_then(|v| v.as_str())
            .unwrap_or("h")
            .to_ascii_lowercase();
        if !orientation.contains('h') && !orientation.contains('v') {
            return Err(ToolError::Validation(
                "orientation must be 'h' or 'v'".to_string(),
            ));
        }

        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = load_vector_arg(args, "vector_points")?;
        let width = args
            .get("width")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'width' is required".to_string()))?;
        let orientation = parse_hex_orientation(args, "orientation");
        let output_path = parse_vector_path_arg(args, "output")?;

        let mut points = Vec::<wbvector::Coord>::new();
        let total_features = input.features.len().max(1);
        for (idx, feature) in input.features.iter().enumerate() {
            let Some(geometry) = feature.geometry.as_ref() else {
                ctx.progress.progress((idx + 1) as f64 / total_features as f64);
                continue;
            };

            match geometry {
                wbvector::Geometry::Point(coord) => points.push(coord.clone()),
                wbvector::Geometry::MultiPoint(coords) => points.extend(coords.iter().cloned()),
                _ => {
                    return Err(ToolError::Validation(
                        "vector_points vector data must contain only point geometries"
                            .to_string(),
                    ));
                }
            }

            ctx.progress.progress((idx + 1) as f64 / total_features as f64);
        }

        if points.is_empty() {
            return Err(ToolError::Validation(
                "vector_points layer must contain at least one point".to_string(),
            ));
        }

        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for point in &points {
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
        }

        let extent_width = max_x - min_x;
        let extent_height = max_y - min_y;
        let sixty_degrees = std::f64::consts::PI / 6.0;
        let half_width = 0.5 * width;
        let size = half_width / sixty_degrees.cos();
        let height = size * 2.0;
        let three_quarter_height = 0.75 * height;

        let mut grid: Vec<(i64, i64, f64, f64, Vec<wbvector::Coord>)> = Vec::new();

        match orientation {
            HexOrientation::Horizontal => {
                let center_x_0 = min_x + half_width;
                let center_y_0 = max_y - 0.25 * height;
                let rows = (extent_height / three_quarter_height).ceil().max(0.0) as i64;
                let columns = (extent_width / width).ceil().max(0.0) as i64;

                if rows > 0 && columns > 0 && (rows as i128) * (columns as i128) > 100_000 {
                    return Err(ToolError::Validation(
                        "this operation would produce a vector file with too many polygons; increase width"
                            .to_string(),
                    ));
                }

                for row in 0..rows {
                    let center_y = center_y_0 - row as f64 * three_quarter_height;
                    let cols = ((extent_width + half_width * (row as f64 % 2.0)) / width)
                        .ceil()
                        .max(0.0) as i64;
                    for col in 0..cols {
                        let center_x =
                            (center_x_0 - half_width * (row as f64 % 2.0)) + col as f64 * width;
                        let mut ring = Vec::with_capacity(7);
                        for i in (0..=6).rev() {
                            let angle = 2.0 * sixty_degrees * (i as f64 + 0.5);
                            ring.push(wbvector::Coord::xy(
                                center_x + size * angle.cos(),
                                center_y + size * angle.sin(),
                            ));
                        }
                        grid.push((row, col, center_x, center_y, ring));
                    }
                }
            }
            HexOrientation::Vertical => {
                let center_x_0 = min_x + 0.5 * size;
                let center_y_0 = max_y - half_width;
                let rows = (extent_height / width).ceil().max(0.0) as i64;
                let columns = (extent_width / three_quarter_height).ceil().max(0.0) as i64;

                if rows > 0 && columns > 0 && (rows as i128) * (columns as i128) > 100_000 {
                    return Err(ToolError::Validation(
                        "this operation would produce a vector file with too many polygons; increase width"
                            .to_string(),
                    ));
                }

                for col in 0..columns {
                    let rows_for_col = ((extent_height + (col as f64 % 2.0) * half_width) / width)
                        .ceil()
                        .max(0.0) as i64;
                    for row in 0..rows_for_col {
                        let center_x = center_x_0 + col as f64 * three_quarter_height;
                        let center_y =
                            center_y_0 - row as f64 * width + (col as f64 % 2.0) * half_width;
                        let mut ring = Vec::with_capacity(7);
                        for i in (0..=6).rev() {
                            let angle = 2.0 * sixty_degrees * (i as f64 + 0.5) - sixty_degrees;
                            ring.push(wbvector::Coord::xy(
                                center_x + size * angle.cos(),
                                center_y + size * angle.sin(),
                            ));
                        }
                        grid.push((row, col, center_x, center_y, ring));
                    }
                }
            }
        }

        if grid.is_empty() {
            return Err(ToolError::Validation(
                "no hexagons could be generated for the input extent".to_string(),
            ));
        }

        let mut center_index = KdTree::new(2);
        for (idx, (_, _, center_x, center_y, _)) in grid.iter().enumerate() {
            center_index
                .add([*center_x, *center_y], idx)
                .map_err(|e| ToolError::Execution(format!("failed to build hex center index: {e}")))?;
        }

        let mut counts = vec![0i64; grid.len()];
        let total_points = points.len().max(1);
        for (idx, point) in points.iter().enumerate() {
            let nearest = center_index
                .nearest(&[point.x, point.y], 1, &squared_euclidean)
                .map_err(|e| ToolError::Execution(format!("failed nearest-center query: {e}")))?;
            if let Some((_, center_idx_ref)) = nearest.first() {
                counts[**center_idx_ref] += 1;
            }
            ctx.progress.progress((idx + 1) as f64 / total_points as f64);
        }

        let mut output = wbvector::Layer::new(format!("{}_hex_binning", input.name));
        output.geom_type = Some(wbvector::GeometryType::Polygon);
        output.crs = input.crs.clone();
        output
            .schema
            .add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));
        output
            .schema
            .add_field(wbvector::FieldDef::new("ROW", wbvector::FieldType::Integer));
        output
            .schema
            .add_field(wbvector::FieldDef::new("COLUMN", wbvector::FieldType::Integer));
        output
            .schema
            .add_field(wbvector::FieldDef::new("COUNT", wbvector::FieldType::Integer));

        let total_cells = grid.len().max(1);
        for (idx, (row, col, _, _, ring)) in grid.into_iter().enumerate() {
            let fid = (idx + 1) as u64;
            output.push(wbvector::Feature {
                fid,
                geometry: Some(wbvector::Geometry::polygon(ring, vec![])),
                attributes: vec![
                    wbvector::FieldValue::Integer(fid as i64),
                    wbvector::FieldValue::Integer(row),
                    wbvector::FieldValue::Integer(col),
                    wbvector::FieldValue::Integer(counts[idx]),
                ],
            });
            ctx.progress.progress((idx + 1) as f64 / total_cells as f64);
        }

        let output_locator = write_vector_output(&output, output_path.trim())?;
        Ok(build_vector_result(output_locator))
    }
}
