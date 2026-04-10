use std::collections::{BTreeMap, HashMap, VecDeque};

use image::{ImageBuffer, Rgba};
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use rayon::prelude::*;
use serde_json::json;
use smartcore::ensemble::random_forest_classifier::{
    RandomForestClassifier,
    RandomForestClassifierParameters,
};
use smartcore::ensemble::random_forest_regressor::{
    RandomForestRegressor,
    RandomForestRegressorParameters,
};
use smartcore::linear::logistic_regression::{
    LogisticRegression,
    LogisticRegressionParameters,
};
use smartcore::linalg::basic::matrix::DenseMatrix;
use smartcore::svm::svc::{SVC, SVCParameters};
use smartcore::svm::svr::{SVR, SVRParameters};
use smartcore::svm::Kernels;
use wbcore::{
    parse_optional_output_path, parse_raster_path_arg, parse_vector_path_arg, LicenseTier, Tool, ToolArgs, ToolCategory,
    ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata, ToolParamDescriptor,
    ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::{rgb_to_hsi_norm, hsi_to_rgb_norm, value2i, DataType, Raster, RasterConfig, RasterFormat};
use wbvector::Geometry as VectorGeometry;

use crate::memory_store;
use super::color_support;

pub struct BalanceContrastEnhancementTool;
pub struct CreateColourCompositeTool;
pub struct DirectDecorrelationStretchTool;
pub struct FlipImageTool;
pub struct HistogramEqualizationTool;
pub struct HistogramMatchingTool;
pub struct HistogramMatchingTwoImagesTool;
pub struct IntegralImageTransformTool;
pub struct GaussianContrastStretchTool;
pub struct MinMaxContrastStretchTool;
pub struct NormalizedDifferenceIndexTool;
pub struct ClosingTool;
pub struct CornerDetectionTool;
pub struct ChangeVectorAnalysisTool;
pub struct MosaicTool;
pub struct MosaicWithFeatheringTool;
pub struct KMeansClusteringTool;
pub struct ModifiedKMeansClusteringTool;
pub struct CorrectVignettingTool;
pub struct ImageSliderTool;
pub struct ImageStackProfileTool;
pub struct PanchromaticSharpeningTool;
pub struct PiecewiseContrastStretchTool;
pub struct ResampleTool;
pub struct GeneralizeClassifiedRasterTool;
pub struct WriteFunctionMemoryInsertionTool;
pub struct OpeningTool;
pub struct OtsuThresholdingTool;
pub struct PercentageContrastStretchTool;
pub struct RemoveSpursTool;
pub struct SigmoidalContrastStretchTool;
pub struct StandardDeviationContrastStretchTool;
pub struct ThickenRasterLineTool;
pub struct TophatTransformTool;
pub struct LineThinningTool;
pub struct IhsToRgbTool;
pub struct RgbToIhsTool;
pub struct SplitColourCompositeTool;
pub struct MinDistClassificationTool;
pub struct ParallelepipedClassificationTool;
pub struct CannyEdgeDetectionTool;
pub struct EvaluateTrainingSitesTool;
pub struct GeneralizeWithSimilarityTool;
pub struct ImageSegmentationTool;
pub struct KnnClassificationTool;
pub struct KnnRegressionTool;
pub struct FuzzyKnnClassificationTool;
pub struct RandomForestClassificationTool;
pub struct RandomForestRegressionTool;
pub struct RandomForestClassificationFitTool;
pub struct RandomForestClassificationPredictTool;
pub struct RandomForestRegressionFitTool;
pub struct RandomForestRegressionPredictTool;
pub struct SvmClassificationTool;
pub struct SvmRegressionTool;
pub struct LogisticRegressionTool;
pub struct NndClassificationTool;

#[derive(Clone, Copy)]
enum NonFilterOp {
    BalanceContrastEnhancement,
    CreateColourComposite,
    DirectDecorrelationStretch,
    FlipImage,
    HistogramEqualization,
    HistogramMatching,
    HistogramMatchingTwoImages,
    IntegralImageTransform,
    GaussianContrastStretch,
    MinMaxContrastStretch,
    NormalizedDifferenceIndex,
    Closing,
    CornerDetection,
    Opening,
    OtsuThresholding,
    PercentageContrastStretch,
    RemoveSpurs,
    SigmoidalContrastStretch,
    StandardDeviationContrastStretch,
    ThickenRasterLine,
    TophatTransform,
    LineThinning,
}

#[derive(Clone, Copy)]
enum FlipDirection {
    Vertical,
    Horizontal,
    Both,
}

#[derive(Clone, Copy)]
enum TailMode {
    Both,
    Upper,
    Lower,
}

#[derive(Clone, Copy)]
enum TophatVariant {
    White,
    Black,
}

#[derive(Clone, Copy)]
enum PanSharpenMethod {
    Brovey,
    Ihs,
}

#[derive(Clone, Copy)]
enum PanSharpenOutputMode {
    Packed,
    Bands,
}

#[derive(Clone, Copy)]
enum ResampleMethod {
    Nearest,
    Bilinear,
    Cubic,
}

impl NonFilterOp {
    fn id(self) -> &'static str {
        match self {
            Self::BalanceContrastEnhancement => "balance_contrast_enhancement",
            Self::CreateColourComposite => "create_colour_composite",
            Self::DirectDecorrelationStretch => "direct_decorrelation_stretch",
            Self::FlipImage => "flip_image",
            Self::HistogramEqualization => "histogram_equalization",
            Self::HistogramMatching => "histogram_matching",
            Self::HistogramMatchingTwoImages => "histogram_matching_two_images",
            Self::IntegralImageTransform => "integral_image_transform",
            Self::GaussianContrastStretch => "gaussian_contrast_stretch",
            Self::MinMaxContrastStretch => "min_max_contrast_stretch",
            Self::NormalizedDifferenceIndex => "normalized_difference_index",
            Self::Closing => "closing",
            Self::CornerDetection => "corner_detection",
            Self::Opening => "opening",
            Self::OtsuThresholding => "otsu_thresholding",
            Self::PercentageContrastStretch => "percentage_contrast_stretch",
            Self::RemoveSpurs => "remove_spurs",
            Self::SigmoidalContrastStretch => "sigmoidal_contrast_stretch",
            Self::StandardDeviationContrastStretch => "standard_deviation_contrast_stretch",
            Self::ThickenRasterLine => "thicken_raster_line",
            Self::TophatTransform => "tophat_transform",
            Self::LineThinning => "line_thinning",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::BalanceContrastEnhancement => "Balance Contrast Enhancement",
            Self::CreateColourComposite => "Create Colour Composite",
            Self::DirectDecorrelationStretch => "Direct Decorrelation Stretch",
            Self::FlipImage => "Flip Image",
            Self::HistogramEqualization => "Histogram Equalization",
            Self::HistogramMatching => "Histogram Matching",
            Self::HistogramMatchingTwoImages => "Histogram Matching Two Images",
            Self::IntegralImageTransform => "Integral Image Transform",
            Self::GaussianContrastStretch => "Gaussian Contrast Stretch",
            Self::MinMaxContrastStretch => "Min-Max Contrast Stretch",
            Self::NormalizedDifferenceIndex => "Normalized Difference Index",
            Self::Closing => "Closing",
            Self::CornerDetection => "Corner Detection",
            Self::Opening => "Opening",
            Self::OtsuThresholding => "Otsu Thresholding",
            Self::PercentageContrastStretch => "Percentage Contrast Stretch",
            Self::RemoveSpurs => "Remove Spurs",
            Self::SigmoidalContrastStretch => "Sigmoidal Contrast Stretch",
            Self::StandardDeviationContrastStretch => "Standard Deviation Contrast Stretch",
            Self::ThickenRasterLine => "Thicken Raster Line",
            Self::TophatTransform => "Top-Hat Transform",
            Self::LineThinning => "Line Thinning",
        }
    }

    fn summary(self) -> &'static str {
        match self {
            Self::BalanceContrastEnhancement => {
                "Reduces colour bias in a packed RGB image using per-channel parabolic stretches."
            }
            Self::CreateColourComposite => {
                "Creates a packed RGB colour composite from red, green, blue, and optional opacity rasters."
            }
            Self::DirectDecorrelationStretch => {
                "Improves packed RGB colour saturation by reducing the achromatic component and linearly stretching channels."
            }
            Self::FlipImage => "Flips an image vertically, horizontally, or both.",
            Self::HistogramEqualization => {
                "Applies histogram equalization to improve image contrast."
            }
            Self::HistogramMatching => {
                "Matches an image histogram to a supplied reference histogram."
            }
            Self::HistogramMatchingTwoImages => {
                "Matches an input image histogram to a reference image histogram."
            }
            Self::IntegralImageTransform => {
                "Computes a summed-area (integral image) transform for each band."
            }
            Self::GaussianContrastStretch => {
                "Stretches contrast by matching to a Gaussian reference distribution."
            }
            Self::MinMaxContrastStretch => {
                "Linearly stretches values between user-specified minimum and maximum."
            }
            Self::NormalizedDifferenceIndex => {
                "Computes (band1 - band2) / (band1 + band2) from a multiband raster."
            }
            Self::Closing => {
                "Performs a morphological closing operation using a rectangular structuring element."
            }
            Self::CornerDetection => {
                "Identifies corner patterns in binary rasters using hit-and-miss templates."
            }
            Self::Opening => {
                "Performs a morphological opening operation using a rectangular structuring element."
            }
            Self::OtsuThresholding => {
                "Applies Otsu's automatic thresholding to create a binary raster."
            }
            Self::PercentageContrastStretch => {
                "Performs linear contrast stretch with percentile clipping."
            }
            Self::RemoveSpurs => {
                "Removes short spur artifacts from binary raster features by iterative pruning."
            }
            Self::SigmoidalContrastStretch => {
                "Performs sigmoidal contrast stretching using gain and cutoff."
            }
            Self::StandardDeviationContrastStretch => {
                "Performs linear contrast stretch using mean plus/minus a standard deviation multiplier."
            }
            Self::ThickenRasterLine => {
                "Thickens diagonal raster line segments to prevent diagonal leak-through."
            }
            Self::TophatTransform => {
                "Performs a white or black morphological top-hat transform."
            }
            Self::LineThinning => {
                "Reduces connected binary raster features to one-cell-wide skeleton lines."
            }
        }
    }

    fn tags(self) -> Vec<String> {
        vec![
            "remote_sensing".to_string(),
            "raster".to_string(),
            self.id().to_string(),
            "legacy-port".to_string(),
        ]
    }
}

impl FlipImageTool {
    fn parse_input(args: &ToolArgs) -> Result<String, ToolError> {
        parse_raster_path_arg(args, "input")
    }

    fn parse_flip_direction(args: &ToolArgs) -> FlipDirection {
        let raw = args
            .get("direction")
            .and_then(|v| v.as_str())
            .unwrap_or("vertical")
            .to_lowercase();
        if raw == "h" || raw.starts_with("hor") {
            FlipDirection::Horizontal
        } else if raw == "b" || raw == "both" {
            FlipDirection::Both
        } else {
            FlipDirection::Vertical
        }
    }

    fn parse_band_index(args: &ToolArgs, key: &str, default_one_based: usize) -> usize {
        args.get(key)
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(default_one_based)
            .max(1)
            - 1
    }

    fn parse_num_tones(args: &ToolArgs) -> usize {
        args.get("num_tones")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(256)
            .max(2)
    }

    fn parse_clip_percent(args: &ToolArgs) -> f64 {
        args.get("clip")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0)
            .clamp(0.0, 50.0)
    }

    fn parse_tail_mode(args: &ToolArgs) -> TailMode {
        let raw = args
            .get("tail")
            .and_then(|v| v.as_str())
            .unwrap_or("both")
            .to_ascii_lowercase();
        if raw.starts_with("up") {
            TailMode::Upper
        } else if raw.starts_with("low") {
            TailMode::Lower
        } else {
            TailMode::Both
        }
    }

    fn parse_is_cumulative(args: &ToolArgs) -> bool {
        args.get("is_cumulative")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    fn parse_min_val(args: &ToolArgs) -> Result<f64, ToolError> {
        args.get("min_val")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'min_val' is required".to_string()))
    }

    fn parse_max_val(args: &ToolArgs) -> Result<f64, ToolError> {
        args.get("max_val")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'max_val' is required".to_string()))
    }

    fn parse_sigmoid_cutoff(args: &ToolArgs) -> f64 {
        args.get("cutoff")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
            .clamp(0.0, 0.95)
    }

    fn parse_sigmoid_gain(args: &ToolArgs) -> f64 {
        args.get("gain").and_then(|v| v.as_f64()).unwrap_or(1.0)
    }

    fn parse_stdev_clip(args: &ToolArgs) -> f64 {
        args.get("clip").and_then(|v| v.as_f64()).unwrap_or(2.0)
    }

    fn parse_filter_size(args: &ToolArgs, key: &str, default: usize) -> usize {
        args.get(key)
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(default)
    }

    fn parse_tophat_variant(args: &ToolArgs) -> TophatVariant {
        let raw = args
            .get("variant")
            .and_then(|v| v.as_str())
            .unwrap_or("white")
            .to_ascii_lowercase();
        if raw.starts_with('b') {
            TophatVariant::Black
        } else {
            TophatVariant::White
        }
    }

    fn parse_max_iterations(args: &ToolArgs) -> usize {
        args.get("max_iterations")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(10)
    }

    fn parse_histogram_pairs(args: &ToolArgs) -> Result<Vec<(f64, f64)>, ToolError> {
        let v = args.get("histogram").ok_or_else(|| {
            ToolError::Validation("parameter 'histogram' is required".to_string())
        })?;
        let arr = v.as_array().ok_or_else(|| {
            ToolError::Validation("parameter 'histogram' must be a list".to_string())
        })?;

        let mut out = Vec::with_capacity(arr.len());
        for item in arr {
            if let Some(pair) = item.as_array() {
                if pair.len() == 2 {
                    if let (Some(x), Some(y)) = (pair[0].as_f64(), pair[1].as_f64()) {
                        out.push((x, y));
                        continue;
                    }
                }
            }
            if let Some(obj) = item.as_object() {
                let x = obj.get("x").and_then(|vv| vv.as_f64());
                let y = obj.get("y").and_then(|vv| vv.as_f64());
                if let (Some(x), Some(y)) = (x, y) {
                    out.push((x, y));
                    continue;
                }
            }
            return Err(ToolError::Validation(
                "parameter 'histogram' entries must be [value, frequency] pairs or {x, y} objects"
                    .to_string(),
            ));
        }

        if out.len() < 2 {
            return Err(ToolError::Validation(
                "parameter 'histogram' must contain at least two entries".to_string(),
            ));
        }
        Ok(out)
    }

    fn parse_reference_path(args: &ToolArgs) -> Result<String, ToolError> {
        parse_raster_path_arg(args, "reference")
    }

    fn parse_optional_raster_path(args: &ToolArgs, key: &str) -> Result<Option<String>, ToolError> {
        match args.get(key) {
            Some(value) => {
                let s = value.as_str().ok_or_else(|| {
                    ToolError::Validation(format!("parameter '{}' must be a raster path", key))
                })?;
                if s.trim().is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(s.to_string()))
                }
            }
            None => Ok(None),
        }
    }

    fn parse_bool_arg(args: &ToolArgs, key: &str, default: bool) -> bool {
        args.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
    }

    fn parse_create_colour_inputs(
        args: &ToolArgs,
    ) -> Result<(String, String, String, Option<String>, bool, bool), ToolError> {
        let red = parse_raster_path_arg(args, "red")?;
        let green = parse_raster_path_arg(args, "green")?;
        let blue = parse_raster_path_arg(args, "blue")?;
        let opacity = Self::parse_optional_raster_path(args, "opacity")?;
        let enhance = Self::parse_bool_arg(args, "enhance", true);
        let treat_zeros_as_nodata = Self::parse_bool_arg(args, "treat_zeros_as_nodata", false);
        Ok((red, green, blue, opacity, enhance, treat_zeros_as_nodata))
    }

    fn parse_band_mean(args: &ToolArgs) -> f64 {
        args.get("band_mean")
            .and_then(|v| v.as_f64())
            .unwrap_or(100.0)
            .clamp(20.0, 235.0)
    }

    fn parse_achromatic_factor(args: &ToolArgs) -> f64 {
        args.get("achromatic_factor")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5)
            .clamp(0.0, 1.0)
    }

    fn parse_clip_percent_fraction(args: &ToolArgs) -> f64 {
        args.get("clip_percent")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0)
            .clamp(0.0, 50.0)
            / 100.0
    }

    fn load_raster(path: &str) -> Result<Raster, ToolError> {
        if memory_store::raster_is_memory_path(path) {
            let id = memory_store::raster_path_to_id(path).ok_or_else(|| {
                ToolError::Validation("parameter 'input' has malformed in-memory raster path".to_string())
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

    fn store_named_raster_output(
        raster: Raster,
        output_path: Option<std::path::PathBuf>,
    ) -> Result<serde_json::Value, ToolError> {
        let locator = Self::write_or_store_output(raster, output_path)?;
        Ok(json!({"__wbw_type__": "raster", "path": locator, "active_band": 0}))
    }

    fn write_or_store_output(output: Raster, output_path: Option<std::path::PathBuf>) -> Result<String, ToolError> {
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

    fn metadata_for(op: NonFilterOp) -> ToolMetadata {
        let mut params = vec![ToolParamSpec {
            name: "input",
            description: "Input raster path or typed raster object.",
            required: true,
        }];

        match op {
            NonFilterOp::BalanceContrastEnhancement => {
                params.push(ToolParamSpec {
                    name: "band_mean",
                    description: "Desired output mean brightness for each channel (default 100.0).",
                    required: false,
                });
            }
            NonFilterOp::CreateColourComposite => {
                params.push(ToolParamSpec {
                    name: "red",
                    description: "Red-band raster path or typed raster object.",
                    required: true,
                });
                params.push(ToolParamSpec {
                    name: "green",
                    description: "Green-band raster path or typed raster object.",
                    required: true,
                });
                params.push(ToolParamSpec {
                    name: "blue",
                    description: "Blue-band raster path or typed raster object.",
                    required: true,
                });
                params.push(ToolParamSpec {
                    name: "opacity",
                    description: "Optional opacity raster path or typed raster object.",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "enhance",
                    description: "Apply balance contrast enhancement after composing (default true).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "treat_zeros_as_nodata",
                    description: "Treat zero values in RGB inputs as nodata/background (default false).",
                    required: false,
                });
            }
            NonFilterOp::DirectDecorrelationStretch => {
                params.push(ToolParamSpec {
                    name: "achromatic_factor",
                    description: "Grey-component reduction factor from 0 to 1 (default 0.5).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "clip_percent",
                    description: "Percent tail clipping for post-stretch linear rescaling (default 1.0).",
                    required: false,
                });
            }
            NonFilterOp::FlipImage => {
                params.push(ToolParamSpec {
                    name: "direction",
                    description: "Flip direction: vertical (default), horizontal, or both.",
                    required: false,
                });
            }
            NonFilterOp::HistogramEqualization => {
                params.push(ToolParamSpec {
                    name: "num_tones",
                    description: "Number of output tones (default 256).",
                    required: false,
                });
            }
            NonFilterOp::HistogramMatching => {
                params.push(ToolParamSpec {
                    name: "histogram",
                    description: "Reference histogram as [[value, frequency], ...] or [{x, y}, ...].",
                    required: true,
                });
                params.push(ToolParamSpec {
                    name: "is_cumulative",
                    description: "True if supplied histogram values are already cumulative.",
                    required: false,
                });
            }
            NonFilterOp::HistogramMatchingTwoImages => {
                params.push(ToolParamSpec {
                    name: "reference",
                    description: "Reference raster path or typed raster object.",
                    required: true,
                });
            }
            NonFilterOp::IntegralImageTransform => {}
            NonFilterOp::GaussianContrastStretch => {
                params.push(ToolParamSpec {
                    name: "num_tones",
                    description: "Number of output tones (default 256).",
                    required: false,
                });
            }
            NonFilterOp::MinMaxContrastStretch => {
                params.push(ToolParamSpec {
                    name: "min_val",
                    description: "Minimum input value for scaling.",
                    required: true,
                });
                params.push(ToolParamSpec {
                    name: "max_val",
                    description: "Maximum input value for scaling.",
                    required: true,
                });
                params.push(ToolParamSpec {
                    name: "num_tones",
                    description: "Number of output tones (default 256).",
                    required: false,
                });
            }
            NonFilterOp::NormalizedDifferenceIndex => {
                params.push(ToolParamSpec {
                    name: "band1",
                    description: "One-based index of first band (default 1).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "band2",
                    description: "One-based index of second band (default 2).",
                    required: false,
                });
            }
            NonFilterOp::Closing | NonFilterOp::Opening => {
                params.push(ToolParamSpec {
                    name: "filter_size_x",
                    description: "Odd neighborhood width (default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "filter_size_y",
                    description: "Odd neighborhood height (default same as filter_size_x).",
                    required: false,
                });
            }
            NonFilterOp::CornerDetection => {}
            NonFilterOp::OtsuThresholding => {}
            NonFilterOp::PercentageContrastStretch => {
                params.push(ToolParamSpec {
                    name: "clip",
                    description: "Percentile clip percentage (default 1.0).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "tail",
                    description: "Tail clipping mode: both (default), upper, or lower.",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "num_tones",
                    description: "Number of output tones (default 256).",
                    required: false,
                });
            }
            NonFilterOp::RemoveSpurs => {
                params.push(ToolParamSpec {
                    name: "max_iterations",
                    description: "Maximum pruning iterations (default 10).",
                    required: false,
                });
            }
            NonFilterOp::SigmoidalContrastStretch => {
                params.push(ToolParamSpec {
                    name: "cutoff",
                    description: "Sigmoid midpoint in normalized units (default 0.0).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "gain",
                    description: "Sigmoid gain parameter (default 1.0).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "num_tones",
                    description: "Number of output tones (default 256).",
                    required: false,
                });
            }
            NonFilterOp::StandardDeviationContrastStretch => {
                params.push(ToolParamSpec {
                    name: "clip",
                    description: "Standard deviation multiplier used to derive clip bounds (default 2.0).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "num_tones",
                    description: "Number of output tones (default 256).",
                    required: false,
                });
            }
            NonFilterOp::TophatTransform => {
                params.push(ToolParamSpec {
                    name: "filter_size_x",
                    description: "Odd neighborhood width (default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "filter_size_y",
                    description: "Odd neighborhood height (default same as filter_size_x).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "variant",
                    description: "Top-hat variant: white (default) or black.",
                    required: false,
                });
            }
            NonFilterOp::ThickenRasterLine | NonFilterOp::LineThinning => {}
        }

        params.push(ToolParamSpec {
            name: "output",
            description: "Optional output path. If omitted, output remains in memory.",
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

    fn manifest_for(op: NonFilterOp) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        if !matches!(op, NonFilterOp::CreateColourComposite) {
            defaults.insert("input".to_string(), json!("input.tif"));
        }
        match op {
            NonFilterOp::BalanceContrastEnhancement => {
                defaults.insert("band_mean".to_string(), json!(100.0));
            }
            NonFilterOp::CreateColourComposite => {
                defaults.insert("red".to_string(), json!("red.tif"));
                defaults.insert("green".to_string(), json!("green.tif"));
                defaults.insert("blue".to_string(), json!("blue.tif"));
                defaults.insert("enhance".to_string(), json!(true));
                defaults.insert("treat_zeros_as_nodata".to_string(), json!(false));
            }
            NonFilterOp::DirectDecorrelationStretch => {
                defaults.insert("achromatic_factor".to_string(), json!(0.5));
                defaults.insert("clip_percent".to_string(), json!(1.0));
            }
            NonFilterOp::FlipImage => {
                defaults.insert("direction".to_string(), json!("vertical"));
            }
            NonFilterOp::HistogramEqualization => {
                defaults.insert("num_tones".to_string(), json!(256));
            }
            NonFilterOp::HistogramMatching => {
                defaults.insert(
                    "histogram".to_string(),
                    json!([[0.0, 0.1], [128.0, 0.7], [255.0, 1.0]]),
                );
                defaults.insert("is_cumulative".to_string(), json!(true));
            }
            NonFilterOp::HistogramMatchingTwoImages => {
                defaults.insert("reference".to_string(), json!("reference.tif"));
            }
            NonFilterOp::IntegralImageTransform => {}
            NonFilterOp::GaussianContrastStretch => {
                defaults.insert("num_tones".to_string(), json!(256));
            }
            NonFilterOp::MinMaxContrastStretch => {
                defaults.insert("min_val".to_string(), json!(0.0));
                defaults.insert("max_val".to_string(), json!(255.0));
                defaults.insert("num_tones".to_string(), json!(256));
            }
            NonFilterOp::NormalizedDifferenceIndex => {
                defaults.insert("band1".to_string(), json!(1));
                defaults.insert("band2".to_string(), json!(2));
            }
            NonFilterOp::Closing | NonFilterOp::Opening => {
                defaults.insert("filter_size_x".to_string(), json!(11));
                defaults.insert("filter_size_y".to_string(), json!(11));
            }
            NonFilterOp::CornerDetection => {}
            NonFilterOp::OtsuThresholding => {}
            NonFilterOp::PercentageContrastStretch => {
                defaults.insert("clip".to_string(), json!(1.0));
                defaults.insert("tail".to_string(), json!("both"));
                defaults.insert("num_tones".to_string(), json!(256));
            }
            NonFilterOp::RemoveSpurs => {
                defaults.insert("max_iterations".to_string(), json!(10));
            }
            NonFilterOp::SigmoidalContrastStretch => {
                defaults.insert("cutoff".to_string(), json!(0.0));
                defaults.insert("gain".to_string(), json!(1.0));
                defaults.insert("num_tones".to_string(), json!(256));
            }
            NonFilterOp::StandardDeviationContrastStretch => {
                defaults.insert("clip".to_string(), json!(2.0));
                defaults.insert("num_tones".to_string(), json!(256));
            }
            NonFilterOp::ThickenRasterLine | NonFilterOp::LineThinning => {}
            NonFilterOp::TophatTransform => {
                defaults.insert("filter_size_x".to_string(), json!(11));
                defaults.insert("filter_size_y".to_string(), json!(11));
                defaults.insert("variant".to_string(), json!("white"));
            }
        }

        let mut example_args = ToolArgs::new();
        if !matches!(op, NonFilterOp::CreateColourComposite) {
            example_args.insert("input".to_string(), json!("image.tif"));
        }
        example_args.insert("output".to_string(), json!(format!("{}.tif", op.id())));
        if matches!(op, NonFilterOp::BalanceContrastEnhancement) {
            example_args.insert("band_mean".to_string(), json!(100.0));
        }
        if matches!(op, NonFilterOp::CreateColourComposite) {
            example_args.insert("red".to_string(), json!("red.tif"));
            example_args.insert("green".to_string(), json!("green.tif"));
            example_args.insert("blue".to_string(), json!("blue.tif"));
            example_args.insert("enhance".to_string(), json!(true));
            example_args.insert("treat_zeros_as_nodata".to_string(), json!(false));
        }
        if matches!(op, NonFilterOp::DirectDecorrelationStretch) {
            example_args.insert("achromatic_factor".to_string(), json!(0.5));
            example_args.insert("clip_percent".to_string(), json!(1.0));
        }
        if matches!(op, NonFilterOp::FlipImage) {
            example_args.insert("direction".to_string(), json!("horizontal"));
        }
        if matches!(op, NonFilterOp::HistogramEqualization) {
            example_args.insert("num_tones".to_string(), json!(256));
        }
        if matches!(op, NonFilterOp::HistogramMatching) {
            example_args.insert(
                "histogram".to_string(),
                json!([[0.0, 0.05], [64.0, 0.25], [128.0, 0.75], [255.0, 1.0]]),
            );
            example_args.insert("is_cumulative".to_string(), json!(true));
        }
        if matches!(op, NonFilterOp::HistogramMatchingTwoImages) {
            example_args.insert("reference".to_string(), json!("reference.tif"));
        }
        if matches!(op, NonFilterOp::GaussianContrastStretch) {
            example_args.insert("num_tones".to_string(), json!(256));
        }
        if matches!(op, NonFilterOp::MinMaxContrastStretch) {
            example_args.insert("min_val".to_string(), json!(0.0));
            example_args.insert("max_val".to_string(), json!(255.0));
            example_args.insert("num_tones".to_string(), json!(256));
        }
        if matches!(op, NonFilterOp::NormalizedDifferenceIndex) {
            example_args.insert("band1".to_string(), json!(1));
            example_args.insert("band2".to_string(), json!(2));
        }
        if matches!(op, NonFilterOp::Closing | NonFilterOp::Opening) {
            example_args.insert("filter_size_x".to_string(), json!(11));
            example_args.insert("filter_size_y".to_string(), json!(11));
        }
        if matches!(op, NonFilterOp::CornerDetection) {
            example_args.insert("output".to_string(), json!("corners.tif"));
        }
        if matches!(op, NonFilterOp::PercentageContrastStretch) {
            example_args.insert("clip".to_string(), json!(1.0));
            example_args.insert("tail".to_string(), json!("both"));
            example_args.insert("num_tones".to_string(), json!(256));
        }
        if matches!(op, NonFilterOp::RemoveSpurs) {
            example_args.insert("max_iterations".to_string(), json!(10));
        }
        if matches!(op, NonFilterOp::SigmoidalContrastStretch) {
            example_args.insert("cutoff".to_string(), json!(0.5));
            example_args.insert("gain".to_string(), json!(10.0));
            example_args.insert("num_tones".to_string(), json!(256));
        }
        if matches!(op, NonFilterOp::StandardDeviationContrastStretch) {
            example_args.insert("clip".to_string(), json!(2.0));
            example_args.insert("num_tones".to_string(), json!(256));
        }
        if matches!(op, NonFilterOp::TophatTransform) {
            example_args.insert("filter_size_x".to_string(), json!(11));
            example_args.insert("filter_size_y".to_string(), json!(11));
            example_args.insert("variant".to_string(), json!("white"));
        }

        let params = Self::metadata_for(op)
            .params
            .into_iter()
            .map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            })
            .collect();

        ToolManifest {
            id: op.id().to_string(),
            display_name: op.display_name().to_string(),
            summary: op.summary().to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params,
            defaults,
            examples: vec![ToolExample {
                name: format!("basic_{}", op.id()),
                description: format!("Runs {} on an input raster.", op.id()),
                args: example_args,
            }],
            tags: op.tags(),
            stability: ToolStability::Experimental,
        }
    }

    fn run_flip(input: &Raster, direction: FlipDirection) -> Result<Raster, ToolError> {
        let mut output = input.clone();
        let rows = input.rows as isize;
        let cols = input.cols as isize;

        for b in 0..input.bands as isize {
            for r in 0..rows {
                for c in 0..cols {
                    let (src_r, src_c) = match direction {
                        FlipDirection::Vertical => (rows - 1 - r, c),
                        FlipDirection::Horizontal => (r, cols - 1 - c),
                        FlipDirection::Both => (rows - 1 - r, cols - 1 - c),
                    };
                    let z = input.get(b, src_r, src_c);
                    output.set(b, r, c, z).map_err(|e| {
                        ToolError::Execution(format!("failed writing flipped pixel ({r},{c}): {e}"))
                    })?;
                }
            }
        }

        Ok(output)
    }

    fn validate_packed_rgb(input: &Raster, tool_id: &str) -> Result<(), ToolError> {
        let rgb_mode = color_support::detect_rgb_mode(input, false, true);
        let packed_rgb = (matches!(rgb_mode, color_support::RgbMode::Packed)
            || (input.bands == 1 && input.data_type == DataType::U32))
            && input.bands == 1;

        if !packed_rgb {
            return Err(ToolError::Validation(format!(
                "{tool_id} requires a single-band packed RGB raster"
            )));
        }
        Ok(())
    }

    fn unpack_rgba(value: f64) -> (u32, u32, u32, u32) {
        let v = value as u32;
        (v & 0xFF, (v >> 8) & 0xFF, (v >> 16) & 0xFF, (v >> 24) & 0xFF)
    }

    fn pack_rgba(r: u32, g: u32, b: u32, a: u32) -> f64 {
        ((a << 24) | (b << 16) | (g << 8) | r) as f64
    }

    fn run_balance_contrast_enhancement(input: &Raster, band_mean: f64) -> Result<Raster, ToolError> {
        Self::validate_packed_rgb(input, NonFilterOp::BalanceContrastEnhancement.id())?;

        let mut output = input.clone();
        let l = 0.0;
        let h = 255.0;
        let mut num_pixels = 0.0;
        let mut r_min = f64::INFINITY;
        let mut g_min = f64::INFINITY;
        let mut b_min = f64::INFINITY;
        let mut r_max = f64::NEG_INFINITY;
        let mut g_max = f64::NEG_INFINITY;
        let mut b_max = f64::NEG_INFINITY;
        let mut r_sum = 0.0;
        let mut g_sum = 0.0;
        let mut b_sum = 0.0;
        let mut r_sq_sum = 0.0;
        let mut g_sq_sum = 0.0;
        let mut b_sq_sum = 0.0;

        for r in 0..input.rows as isize {
            for c in 0..input.cols as isize {
                let z = input.get(0, r, c);
                if input.is_nodata(z) {
                    continue;
                }
                let (rv, gv, bv, _) = Self::unpack_rgba(z);
                let rf = rv as f64;
                let gf = gv as f64;
                let bf = bv as f64;
                num_pixels += 1.0;
                r_min = r_min.min(rf);
                g_min = g_min.min(gf);
                b_min = b_min.min(bf);
                r_max = r_max.max(rf);
                g_max = g_max.max(gf);
                b_max = b_max.max(bf);
                r_sum += rf;
                g_sum += gf;
                b_sum += bf;
                r_sq_sum += rf * rf;
                g_sq_sum += gf * gf;
                b_sq_sum += bf * bf;
            }
        }

        if num_pixels == 0.0 {
            return Ok(output);
        }

        let r_mean = r_sum / num_pixels;
        let g_mean = g_sum / num_pixels;
        let b_mean = b_sum / num_pixels;
        let r_s = r_sq_sum / num_pixels;
        let g_s = g_sq_sum / num_pixels;
        let b_s = b_sq_sum / num_pixels;

        let parabola = |min_v: f64, max_v: f64, mean_v: f64, sq_mean: f64| {
            let denom = 2.0 * (max_v * (band_mean - l) - mean_v * (h - l) + min_v * (h - band_mean));
            if denom.abs() < 1e-12 || (max_v - min_v).abs() < 1e-12 {
                (1.0, 0.0, 0.0)
            } else {
                let b = (max_v * max_v * (band_mean - l) - sq_mean * (h - l) + min_v * min_v * (h - band_mean)) / denom;
                let a = (h - l) / ((max_v - min_v) * (max_v + min_v - 2.0 * b)).max(1e-12);
                let c = l - a * (min_v - b) * (min_v - b);
                (a, b, c)
            }
        };

        let (ra, rb, rc) = parabola(r_min, r_max, r_mean, r_s);
        let (ga, gb, gc) = parabola(g_min, g_max, g_mean, g_s);
        let (ba, bb, bc) = parabola(b_min, b_max, b_mean, b_s);

        for r in 0..input.rows as isize {
            for c in 0..input.cols as isize {
                let z = input.get(0, r, c);
                if input.is_nodata(z) {
                    continue;
                }
                let (rv, gv, bv, av) = Self::unpack_rgba(z);
                let rn = (ra * (rv as f64 - rb).powi(2) + rc).clamp(0.0, 255.0).round() as u32;
                let gn = (ga * (gv as f64 - gb).powi(2) + gc).clamp(0.0, 255.0).round() as u32;
                let bn = (ba * (bv as f64 - bb).powi(2) + bc).clamp(0.0, 255.0).round() as u32;
                output.set(0, r, c, Self::pack_rgba(rn, gn, bn, av)).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed writing BCE pixel at ({r},{c}): {e}"
                    ))
                })?;
            }
        }

        Ok(output)
    }

    fn run_direct_decorrelation_stretch(
        input: &Raster,
        achromatic_factor: f64,
        clip_percent: f64,
    ) -> Result<Raster, ToolError> {
        Self::validate_packed_rgb(input, NonFilterOp::DirectDecorrelationStretch.id())?;

        let mut stage1 = input.clone();
        let mut hist = [0usize; 256];
        let mut samples = 0usize;

        for r in 0..input.rows as isize {
            for c in 0..input.cols as isize {
                let z = input.get(0, r, c);
                if input.is_nodata(z) {
                    continue;
                }
                let (rv, gv, bv, av) = Self::unpack_rgba(z);
                let min_v = rv.min(gv).min(bv) as f64;
                let rn = (rv as f64 - achromatic_factor * min_v).clamp(0.0, 255.0).round() as u32;
                let gn = (gv as f64 - achromatic_factor * min_v).clamp(0.0, 255.0).round() as u32;
                let bn = (bv as f64 - achromatic_factor * min_v).clamp(0.0, 255.0).round() as u32;
                hist[rn as usize] += 1;
                hist[gn as usize] += 1;
                hist[bn as usize] += 1;
                samples += 3;
                stage1.set(0, r, c, Self::pack_rgba(rn, gn, bn, av)).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed writing DDS intermediate pixel at ({r},{c}): {e}"
                    ))
                })?;
            }
        }

        if samples == 0 {
            return Ok(stage1);
        }

        let clip_tail = (samples as f64 * clip_percent).round() as usize;
        let mut running = 0usize;
        let mut stretch_min = 0usize;
        for (idx, count) in hist.iter().enumerate() {
            if running + *count > clip_tail {
                stretch_min = idx;
                break;
            }
            running += *count;
        }

        running = 0;
        let mut stretch_max = 255usize;
        for idx in (0..256).rev() {
            if running + hist[idx] > clip_tail {
                stretch_max = idx;
                break;
            }
            running += hist[idx];
        }

        let mut output = stage1.clone();
        let width = (stretch_max as f64 - stretch_min as f64).max(1.0);
        for r in 0..stage1.rows as isize {
            for c in 0..stage1.cols as isize {
                let z = stage1.get(0, r, c);
                if stage1.is_nodata(z) {
                    continue;
                }
                let (rv, gv, bv, av) = Self::unpack_rgba(z);
                let scale = |v: u32| {
                    (((v as f64).clamp(stretch_min as f64, stretch_max as f64) - stretch_min as f64) / width * 255.0)
                        .clamp(0.0, 255.0)
                        .round() as u32
                };
                output.set(0, r, c, Self::pack_rgba(scale(rv), scale(gv), scale(bv), av)).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed writing DDS output pixel at ({r},{c}): {e}"
                    ))
                })?;
            }
        }

        Ok(output)
    }

    fn run_create_colour_composite(
        red: &Raster,
        green: &Raster,
        blue: &Raster,
        opacity: Option<&Raster>,
        enhance: bool,
        treat_zeros_as_nodata: bool,
    ) -> Result<Raster, ToolError> {
        if red.rows != green.rows
            || red.cols != green.cols
            || red.rows != blue.rows
            || red.cols != blue.cols
            || red.bands != 1
            || green.bands != 1
            || blue.bands != 1
        {
            return Err(ToolError::Validation(
                "red, green, and blue inputs must be single-band rasters with identical dimensions"
                    .to_string(),
            ));
        }
        if let Some(alpha) = opacity {
            if alpha.rows != red.rows || alpha.cols != red.cols || alpha.bands != 1 {
                return Err(ToolError::Validation(
                    "optional opacity raster must be single-band and match the RGB raster dimensions"
                        .to_string(),
                ));
            }
        }

        let mut output = Raster::new(RasterConfig {
            rows: red.rows,
            cols: red.cols,
            bands: 1,
            x_min: red.x_min,
            y_min: red.y_min,
            cell_size: red.cell_size_x,
            cell_size_y: Some(red.cell_size_y),
            nodata: 0.0,
            data_type: DataType::U32,
            crs: red.crs.clone(),
            metadata: {
                let mut md = red.metadata.clone();
                md.push(("color_interpretation".to_string(), "packed_rgb".to_string()));
                md
            },
        });

        let band_min_max = |r: &Raster| {
            let mut min_v = f64::INFINITY;
            let mut max_v = f64::NEG_INFINITY;
            for row in 0..r.rows as isize {
                for col in 0..r.cols as isize {
                    let z = r.get(0, row, col);
                    if r.is_nodata(z) || (treat_zeros_as_nodata && z == 0.0) {
                        continue;
                    }
                    min_v = min_v.min(z);
                    max_v = max_v.max(z);
                }
            }
            if !min_v.is_finite() || !max_v.is_finite() {
                (0.0, 1.0)
            } else {
                (min_v, (max_v - min_v).max(1e-12))
            }
        };

        let (r_min, r_range) = band_min_max(red);
        let (g_min, g_range) = band_min_max(green);
        let (b_min, b_range) = band_min_max(blue);
        let (a_min, a_range) = opacity
            .map(band_min_max)
            .unwrap_or((0.0, 255.0));

        for row in 0..red.rows as isize {
            for col in 0..red.cols as isize {
                let rv = red.get(0, row, col);
                let gv = green.get(0, row, col);
                let bv = blue.get(0, row, col);
                let invalid = red.is_nodata(rv)
                    || green.is_nodata(gv)
                    || blue.is_nodata(bv)
                    || (treat_zeros_as_nodata && (rv == 0.0 || gv == 0.0 || bv == 0.0));
                if invalid {
                    continue;
                }

                let scale = |z: f64, min_v: f64, range_v: f64| {
                    ((z - min_v) / range_v * 255.0).clamp(0.0, 255.0).round() as u32
                };
                let r8 = scale(rv, r_min, r_range);
                let g8 = scale(gv, g_min, g_range);
                let b8 = scale(bv, b_min, b_range);
                let a8 = if let Some(alpha) = opacity {
                    let av = alpha.get(0, row, col);
                    if alpha.is_nodata(av) {
                        255
                    } else {
                        scale(av, a_min, a_range)
                    }
                } else {
                    255
                };
                output.set(0, row, col, Self::pack_rgba(r8, g8, b8, a8)).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed writing colour composite pixel at ({row},{col}): {e}"
                    ))
                })?;
            }
        }

        if enhance {
            Self::run_balance_contrast_enhancement(&output, 100.0)
        } else {
            Ok(output)
        }
    }

    fn run_integral(input: &Raster) -> Result<Raster, ToolError> {
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;

        for b in 0..input.bands {
            let band = b as isize;
            let mut integral = vec![0.0f64; rows * cols];

            for r in 0..rows {
                for c in 0..cols {
                    let z = input.get(band, r as isize, c as isize);
                    let v = if input.is_nodata(z) { 0.0 } else { z };
                    let left = if c > 0 { integral[r * cols + (c - 1)] } else { 0.0 };
                    let up = if r > 0 { integral[(r - 1) * cols + c] } else { 0.0 };
                    let up_left = if r > 0 && c > 0 {
                        integral[(r - 1) * cols + (c - 1)]
                    } else {
                        0.0
                    };
                    integral[r * cols + c] = v + left + up - up_left;
                }
            }

            for r in 0..rows as isize {
                for c in 0..cols as isize {
                    let z = input.get(band, r, c);
                    if input.is_nodata(z) {
                        output.set(band, r, c, input.nodata).map_err(|e| {
                            ToolError::Execution(format!("failed writing nodata at ({r},{c}): {e}"))
                        })?;
                    } else {
                        let v = integral[r as usize * cols + c as usize];
                        output.set(band, r, c, v).map_err(|e| {
                            ToolError::Execution(format!("failed writing integral value at ({r},{c}): {e}"))
                        })?;
                    }
                }
            }
        }

        Ok(output)
    }

    fn normalized_filter_sizes(filter_size_x: usize, filter_size_y: usize) -> (usize, usize, isize, isize) {
        let mut fx = filter_size_x.max(3);
        let mut fy = filter_size_y.max(3);
        if fx % 2 == 0 {
            fx += 1;
        }
        if fy % 2 == 0 {
            fy += 1;
        }
        let mx = (fx / 2) as isize;
        let my = (fy / 2) as isize;
        (fx, fy, mx, my)
    }

    fn morph_erode(input: &Raster, filter_size_x: usize, filter_size_y: usize) -> Result<Raster, ToolError> {
        let (_fx, _fy, mx, my) = Self::normalized_filter_sizes(filter_size_x, filter_size_y);
        let mut output = input.clone();
        for b in 0..input.bands as isize {
            for r in 0..input.rows as isize {
                for c in 0..input.cols as isize {
                    let center = input.get(b, r, c);
                    if input.is_nodata(center) {
                        continue;
                    }
                    let mut min_v = f64::INFINITY;
                    for rr in (r - my)..=(r + my) {
                        for cc in (c - mx)..=(c + mx) {
                            let z = input.get(b, rr, cc);
                            if !input.is_nodata(z) {
                                min_v = min_v.min(z);
                            }
                        }
                    }
                    if min_v.is_finite() {
                        output.set(b, r, c, min_v).map_err(|e| {
                            ToolError::Execution(format!("failed writing erosion value at ({r},{c}): {e}"))
                        })?;
                    }
                }
            }
        }
        Ok(output)
    }

    fn morph_dilate(input: &Raster, filter_size_x: usize, filter_size_y: usize) -> Result<Raster, ToolError> {
        let (_fx, _fy, mx, my) = Self::normalized_filter_sizes(filter_size_x, filter_size_y);
        let mut output = input.clone();
        for b in 0..input.bands as isize {
            for r in 0..input.rows as isize {
                for c in 0..input.cols as isize {
                    let center = input.get(b, r, c);
                    if input.is_nodata(center) {
                        continue;
                    }
                    let mut max_v = f64::NEG_INFINITY;
                    for rr in (r - my)..=(r + my) {
                        for cc in (c - mx)..=(c + mx) {
                            let z = input.get(b, rr, cc);
                            if !input.is_nodata(z) {
                                max_v = max_v.max(z);
                            }
                        }
                    }
                    if max_v.is_finite() {
                        output.set(b, r, c, max_v).map_err(|e| {
                            ToolError::Execution(format!("failed writing dilation value at ({r},{c}): {e}"))
                        })?;
                    }
                }
            }
        }
        Ok(output)
    }

    fn run_opening(input: &Raster, filter_size_x: usize, filter_size_y: usize) -> Result<Raster, ToolError> {
        let eroded = Self::morph_erode(input, filter_size_x, filter_size_y)?;
        Self::morph_dilate(&eroded, filter_size_x, filter_size_y)
    }

    fn run_closing(input: &Raster, filter_size_x: usize, filter_size_y: usize) -> Result<Raster, ToolError> {
        let dilated = Self::morph_dilate(input, filter_size_x, filter_size_y)?;
        Self::morph_erode(&dilated, filter_size_x, filter_size_y)
    }

    fn run_tophat_transform(
        input: &Raster,
        filter_size_x: usize,
        filter_size_y: usize,
        variant: TophatVariant,
    ) -> Result<Raster, ToolError> {
        let basis = match variant {
            TophatVariant::White => Self::run_opening(input, filter_size_x, filter_size_y)?,
            TophatVariant::Black => Self::run_closing(input, filter_size_x, filter_size_y)?,
        };
        let mut output = input.clone();
        for b in 0..input.bands as isize {
            for r in 0..input.rows as isize {
                for c in 0..input.cols as isize {
                    let z0 = input.get(b, r, c);
                    let z1 = basis.get(b, r, c);
                    if input.is_nodata(z0) || basis.is_nodata(z1) {
                        continue;
                    }
                    let out = match variant {
                        TophatVariant::White => z0 - z1,
                        TophatVariant::Black => z1 - z0,
                    };
                    output.set(b, r, c, out).map_err(|e| {
                        ToolError::Execution(format!("failed writing top-hat value at ({r},{c}): {e}"))
                    })?;
                }
            }
        }
        Ok(output)
    }

    fn run_otsu_thresholding(input: &Raster) -> Result<Raster, ToolError> {
        let rgb_mode = color_support::detect_rgb_mode(input, false, true);
        let packed_rgb = matches!(rgb_mode, color_support::RgbMode::Packed) && input.bands == 1;

        let mut min_v = f64::INFINITY;
        let mut max_v = f64::NEG_INFINITY;
        let mut values = Vec::with_capacity(input.rows * input.cols);
        for r in 0..input.rows as isize {
            for c in 0..input.cols as isize {
                let z_raw = input.get(0, r, c);
                if input.is_nodata(z_raw) {
                    continue;
                }
                let z = if packed_rgb { value2i(z_raw) } else { z_raw };
                min_v = min_v.min(z);
                max_v = max_v.max(z);
                values.push(z);
            }
        }
        if values.is_empty() {
            return Ok(Raster::new(RasterConfig {
                rows: input.rows,
                cols: input.cols,
                bands: 1,
                x_min: input.x_min,
                y_min: input.y_min,
                cell_size: input.cell_size_x,
                cell_size_y: Some(input.cell_size_y),
                nodata: -32768.0,
                data_type: DataType::I16,
                crs: input.crs.clone(),
                metadata: input.metadata.clone(),
            }));
        }

        let mut num_bins = 1024usize;
        let range = (max_v - min_v).max(1e-12);
        if !packed_rgb && range.round() as usize > num_bins {
            num_bins = range.round() as usize;
        }
        let bin_size = range / (num_bins - 1) as f64;
        let mut histo = vec![0usize; num_bins];
        for z in &values {
            let idx = (((*z - min_v) / bin_size).floor() as usize).min(num_bins - 1);
            histo[idx] += 1;
        }
        let total = values.len() as f64;

        let mut cumulative = vec![0usize; num_bins];
        let mut running = 0usize;
        for i in 0..num_bins {
            running += histo[i];
            cumulative[i] = running;
        }
        let cdf = cumulative.iter().map(|&v| v as f64 / total).collect::<Vec<_>>();

        let mut prefix_weighted = vec![0usize; num_bins];
        let mut weighted_running = 0usize;
        for i in 0..num_bins {
            weighted_running += i * histo[i];
            prefix_weighted[i] = weighted_running;
        }
        let total_weighted = *prefix_weighted.last().unwrap_or(&0) as f64;

        let mut max_var = f64::NEG_INFINITY;
        let mut max_i = 0usize;
        for bin in 0..(num_bins - 1) {
            let w0 = cdf[bin];
            let w1 = 1.0 - w0;
            if w0 <= 0.0 || w1 <= 0.0 {
                continue;
            }
            let m0 = prefix_weighted[bin] as f64 / (w0 * total);
            let m1 = (total_weighted - prefix_weighted[bin] as f64) / (w1 * total);
            let var = w0 * w1 * (m0 - m1).powi(2);
            if var > max_var {
                max_var = var;
                max_i = bin;
            }
        }

        let cfg = RasterConfig {
            rows: input.rows,
            cols: input.cols,
            bands: 1,
            x_min: input.x_min,
            y_min: input.y_min,
            cell_size: input.cell_size_x,
            cell_size_y: Some(input.cell_size_y),
            nodata: -32768.0,
            data_type: DataType::I16,
            crs: input.crs.clone(),
            metadata: input.metadata.clone(),
        };
        let mut output = Raster::new(cfg);
        for r in 0..input.rows as isize {
            let mut row = vec![-32768.0; input.cols];
            for c in 0..input.cols as isize {
                let z_raw = input.get(0, r, c);
                if input.is_nodata(z_raw) {
                    continue;
                }
                let z = if packed_rgb { value2i(z_raw) } else { z_raw };
                let idx = (((z - min_v) / bin_size).floor() as usize).min(num_bins - 1);
                row[c as usize] = if idx <= max_i { 0.0 } else { 1.0 };
            }
            output.set_row_slice(0, r, &row).map_err(|e| {
                ToolError::Execution(format!("failed writing otsu row {}: {}", r, e))
            })?;
        }
        Ok(output)
    }

    fn to_binary_raster(input: &Raster) -> Result<Raster, ToolError> {
        let mut output = input.clone();
        for b in 0..input.bands as isize {
            for r in 0..input.rows as isize {
                for c in 0..input.cols as isize {
                    let z = input.get(b, r, c);
                    if input.is_nodata(z) {
                        output.set(b, r, c, input.nodata).map_err(|e| {
                            ToolError::Execution(format!(
                                "failed writing binary nodata at ({r},{c}): {e}"
                            ))
                        })?;
                    } else {
                        let v = if z > 0.0 { 1.0 } else { 0.0 };
                        output.set(b, r, c, v).map_err(|e| {
                            ToolError::Execution(format!(
                                "failed writing binary raster value at ({r},{c}): {e}"
                            ))
                        })?;
                    }
                }
            }
        }
        Ok(output)
    }

    fn run_line_thinning(input: &Raster) -> Result<Raster, ToolError> {
        let mut output = Self::to_binary_raster(input)?;
        let dx: [isize; 8] = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy: [isize; 8] = [-1, 0, 1, 1, 1, 0, -1, -1];
        let elements1: [[usize; 6]; 4] = [
            [6, 7, 0, 4, 3, 2],
            [0, 1, 2, 4, 5, 6],
            [2, 3, 4, 6, 7, 0],
            [4, 5, 6, 0, 1, 2],
        ];
        let elements2: [[usize; 5]; 4] = [
            [7, 0, 1, 3, 5],
            [1, 2, 3, 5, 7],
            [3, 4, 5, 7, 1],
            [5, 6, 7, 1, 3],
        ];
        let vals1 = [0.0f64, 0.0, 0.0, 1.0, 1.0, 1.0];
        let vals2 = [0.0f64, 0.0, 0.0, 1.0, 1.0];

        for b in 0..output.bands as isize {
            let mut did_something = true;
            let nodata = output.nodata;
            let rows = output.rows as isize;
            let cols = output.cols as isize;
            let mut neighbours = [0.0f64; 8];

            while did_something {
                did_something = false;
                for a in 0..4 {
                    for r in 0..rows {
                        for c in 0..cols {
                            let z = output.get(b, r, c);
                            if z <= 0.0 || z == nodata {
                                continue;
                            }
                            for i in 0..8 {
                                neighbours[i] = output.get(b, r + dy[i], c + dx[i]);
                            }

                            let mut pattern_match = true;
                            for i in 0..6 {
                                if neighbours[elements1[a][i]] != vals1[i] {
                                    pattern_match = false;
                                    break;
                                }
                            }
                            if !pattern_match {
                                pattern_match = true;
                                for i in 0..5 {
                                    if neighbours[elements2[a][i]] != vals2[i] {
                                        pattern_match = false;
                                        break;
                                    }
                                }
                            }

                            if pattern_match {
                                output.set(b, r, c, 0.0).map_err(|e| {
                                    ToolError::Execution(format!(
                                        "failed writing thinned pixel at ({r},{c}): {e}"
                                    ))
                                })?;
                                did_something = true;
                            }
                        }
                    }
                }
            }
        }

        Ok(output)
    }

    fn run_remove_spurs(input: &Raster, max_iterations: usize) -> Result<Raster, ToolError> {
        let mut output = Self::to_binary_raster(input)?;
        let dx: [isize; 8] = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy: [isize; 8] = [-1, 0, 1, 1, 1, 0, -1, -1];
        let elements: [[usize; 6]; 8] = [
            [0, 1, 4, 5, 6, 7],
            [0, 1, 2, 5, 6, 7],
            [0, 1, 2, 3, 6, 7],
            [0, 1, 2, 3, 4, 7],
            [0, 1, 2, 3, 4, 5],
            [1, 2, 3, 4, 5, 6],
            [2, 3, 4, 5, 6, 7],
            [0, 3, 4, 5, 6, 7],
        ];

        for b in 0..output.bands as isize {
            let nodata = output.nodata;
            let rows = output.rows as isize;
            let cols = output.cols as isize;
            let mut neighbours = [0.0f64; 8];

            for loop_num in 0..max_iterations {
                let mut did_something = false;
                let reverse_scan = loop_num % 2 == 0;
                for a in 0..8 {
                    if reverse_scan {
                        for r in (0..rows).rev() {
                            for c in (0..cols).rev() {
                                let z = output.get(b, r, c);
                                if z <= 0.0 || z == nodata {
                                    continue;
                                }
                                for i in 0..8 {
                                    neighbours[i] = output.get(b, r + dy[i], c + dx[i]);
                                }
                                let mut pattern_match = true;
                                for i in 0..elements[a].len() {
                                    if neighbours[elements[a][i]] != 0.0 {
                                        pattern_match = false;
                                        break;
                                    }
                                }
                                if pattern_match {
                                    output.set(b, r, c, 0.0).map_err(|e| {
                                        ToolError::Execution(format!(
                                            "failed writing pruned spur pixel at ({r},{c}): {e}"
                                        ))
                                    })?;
                                    did_something = true;
                                }
                            }
                        }
                    } else {
                        for r in 0..rows {
                            for c in 0..cols {
                                let z = output.get(b, r, c);
                                if z <= 0.0 || z == nodata {
                                    continue;
                                }
                                for i in 0..8 {
                                    neighbours[i] = output.get(b, r + dy[i], c + dx[i]);
                                }
                                let mut pattern_match = true;
                                for i in 0..elements[a].len() {
                                    if neighbours[elements[a][i]] != 0.0 {
                                        pattern_match = false;
                                        break;
                                    }
                                }
                                if pattern_match {
                                    output.set(b, r, c, 0.0).map_err(|e| {
                                        ToolError::Execution(format!(
                                            "failed writing pruned spur pixel at ({r},{c}): {e}"
                                        ))
                                    })?;
                                    did_something = true;
                                }
                            }
                        }
                    }
                }
                if !did_something {
                    break;
                }
            }
        }

        Ok(output)
    }

    fn run_thicken_raster_line(input: &Raster) -> Result<Raster, ToolError> {
        let mut output = input.clone();
        let n1x: [isize; 4] = [0, 1, 0, -1];
        let n1y: [isize; 4] = [-1, 0, 1, 0];
        let n2x: [isize; 4] = [1, 1, -1, -1];
        let n2y: [isize; 4] = [-1, 1, 1, -1];
        let n3x: [isize; 4] = [1, 0, -1, 0];
        let n3y: [isize; 4] = [0, 1, 0, -1];

        for b in 0..input.bands as isize {
            for r in 0..input.rows as isize {
                for c in 0..input.cols as isize {
                    let z = input.get(b, r, c);
                    if z != 0.0 && !input.is_nodata(z) {
                        continue;
                    }
                    for i in 0..4 {
                        let zn1 = output.get(b, r + n1y[i], c + n1x[i]);
                        let zn2 = output.get(b, r + n2y[i], c + n2x[i]);
                        let zn3 = output.get(b, r + n3y[i], c + n3x[i]);
                        let n1_fg = !output.is_nodata(zn1) && zn1 > 0.0;
                        let n3_fg = !output.is_nodata(zn3) && zn3 > 0.0;
                        let n2_bg = output.is_nodata(zn2) || zn2 == 0.0;
                        if n1_fg && n3_fg && n2_bg {
                            output.set(b, r, c, zn1).map_err(|e| {
                                ToolError::Execution(format!(
                                    "failed writing thickened line pixel at ({r},{c}): {e}"
                                ))
                            })?;
                            break;
                        }
                    }
                }
            }
        }

        Ok(output)
    }

    fn run_corner_detection(input: &Raster) -> Result<Raster, ToolError> {
        let input = Self::to_binary_raster(input)?;
        let mut output = input.clone();
        let dx: [isize; 8] = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy: [isize; 8] = [-1, 0, 1, 1, 1, 0, -1, -1];

        let elements: [[usize; 5]; 4] = [
            [1, 7, 3, 4, 5],
            [5, 7, 1, 2, 3],
            [3, 5, 0, 1, 7],
            [1, 3, 5, 6, 7],
        ];
        let vals = [1.0f64, 1.0, 0.0, 0.0, 0.0];

        for b in 0..input.bands as isize {
            for r in 0..input.rows as isize {
                for c in 0..input.cols as isize {
                    let z = input.get(b, r, c);
                    if input.is_nodata(z) {
                        output.set(b, r, c, input.nodata).map_err(|e| {
                            ToolError::Execution(format!(
                                "failed writing nodata corner pixel at ({r},{c}): {e}"
                            ))
                        })?;
                        continue;
                    }
                    if z <= 0.0 {
                        output.set(b, r, c, 0.0).map_err(|e| {
                            ToolError::Execution(format!(
                                "failed writing background corner pixel at ({r},{c}): {e}"
                            ))
                        })?;
                        continue;
                    }

                    let mut neighbours = [0.0f64; 8];
                    for i in 0..8 {
                        let zn = input.get(b, r + dy[i], c + dx[i]);
                        neighbours[i] = if !input.is_nodata(zn) && zn > 0.0 { 1.0 } else { 0.0 };
                    }

                    let mut pattern_match = false;
                    for a in 0..4 {
                        let mut matched = true;
                        for i in 0..5 {
                            if neighbours[elements[a][i]] != vals[i] {
                                matched = false;
                                break;
                            }
                        }
                        if matched {
                            pattern_match = true;
                            break;
                        }
                    }

                    let out_v = if pattern_match { 1.0 } else { 0.0 };
                    output.set(b, r, c, out_v).map_err(|e| {
                        ToolError::Execution(format!(
                            "failed writing corner-detection output at ({r},{c}): {e}"
                        ))
                    })?;
                }
            }
        }

        Ok(output)
    }

    fn run_ndi(input: &Raster, band1: usize, band2: usize) -> Result<Raster, ToolError> {
        if input.bands < 2 {
            return Err(ToolError::Validation(
                "normalized_difference_index requires at least two bands".to_string(),
            ));
        }
        if band1 >= input.bands || band2 >= input.bands || band1 == band2 {
            return Err(ToolError::Validation(
                "parameters 'band1' and 'band2' must be distinct valid one-based band indices".to_string(),
            ));
        }

        let cfg = RasterConfig {
            cols: input.cols,
            rows: input.rows,
            bands: 1,
            x_min: input.x_min,
            y_min: input.y_min,
            cell_size: input.cell_size_x,
            cell_size_y: Some(input.cell_size_y),
            nodata: input.nodata,
            data_type: DataType::F32,
            crs: input.crs.clone(),
            metadata: input.metadata.clone(),
        };
        let mut output = Raster::new(cfg);

        let b1 = band1 as isize;
        let b2 = band2 as isize;
        let rows = input.rows as isize;
        let cols = input.cols as isize;

        let row_data: Vec<Vec<f64>> = (0..rows)
            .into_par_iter()
            .map(|r| {
                let mut row = vec![input.nodata; cols as usize];
                for c in 0..cols {
                    let z1 = input.get(b1, r, c);
                    let z2 = input.get(b2, r, c);
                    if input.is_nodata(z1) || input.is_nodata(z2) {
                        continue;
                    }
                    let denom = z1 + z2;
                    if denom.abs() > 1e-12 {
                        row[c as usize] = (z1 - z2) / denom;
                    }
                }
                row
            })
            .collect();

        for (r, row) in row_data.iter().enumerate() {
            output
                .set_row_slice(0, r as isize, row)
                .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
        }

        Ok(output)
    }

    fn collect_valid_values(input: &Raster, band: isize) -> Vec<f64> {
        let mut values = Vec::with_capacity(input.rows * input.cols);
        for r in 0..input.rows as isize {
            for c in 0..input.cols as isize {
                let z = input.get(band, r, c);
                if !input.is_nodata(z) {
                    values.push(z);
                }
            }
        }
        values
    }

    fn quantile_from_sorted(values: &[f64], q: f64) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        let qq = q.clamp(0.0, 1.0);
        let idx = ((values.len() - 1) as f64 * qq).round() as usize;
        values[idx]
    }

    fn build_input_cdf(values: &[f64], bins: usize, min_z: f64, max_z: f64) -> Vec<f64> {
        let mut hist = vec![0usize; bins.max(2)];
        let width = (max_z - min_z).max(1e-12);
        for &z in values {
            let t = ((z - min_z) / width).clamp(0.0, 1.0);
            let idx = (t * (hist.len() - 1) as f64).round() as usize;
            hist[idx] += 1;
        }

        let total = values.len() as f64;
        let mut running = 0.0;
        let mut cdf = vec![0.0; hist.len()];
        if total > 0.0 {
            for (i, &h) in hist.iter().enumerate() {
                running += h as f64;
                cdf[i] = running / total;
            }
        }
        cdf
    }

    fn cdf_value_for_z(z: f64, cdf: &[f64], min_z: f64, max_z: f64) -> f64 {
        let width = (max_z - min_z).max(1e-12);
        let t = ((z - min_z) / width).clamp(0.0, 1.0);
        let idx = (t * (cdf.len() - 1) as f64).round() as usize;
        cdf[idx]
    }

    fn normalize_reference_histogram(
        mut pairs: Vec<(f64, f64)>,
        is_cumulative: bool,
    ) -> Result<(Vec<f64>, Vec<f64>), ToolError> {
        pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
        let xs = pairs.iter().map(|(x, _)| *x).collect::<Vec<_>>();

        let mut ys = if is_cumulative {
            pairs.iter().map(|(_, y)| *y).collect::<Vec<_>>()
        } else {
            let mut running = 0.0;
            let mut out = Vec::with_capacity(pairs.len());
            for &(_, y) in &pairs {
                running += y.max(0.0);
                out.push(running);
            }
            out
        };

        let end = *ys.last().unwrap_or(&0.0);
        if end <= 0.0 {
            return Err(ToolError::Validation(
                "reference histogram cumulative sum must be greater than zero".to_string(),
            ));
        }
        for y in &mut ys {
            *y = (*y / end).clamp(0.0, 1.0);
        }
        Ok((xs, ys))
    }

    fn map_probability_to_reference_value(p: f64, ref_x: &[f64], ref_cdf: &[f64]) -> f64 {
        if ref_x.is_empty() || ref_cdf.is_empty() {
            return p;
        }
        let pp = p.clamp(0.0, 1.0);
        if pp <= ref_cdf[0] {
            return ref_x[0];
        }
        for i in 1..ref_cdf.len() {
            if pp <= ref_cdf[i] {
                let y0 = ref_cdf[i - 1];
                let y1 = ref_cdf[i];
                let x0 = ref_x[i - 1];
                let x1 = ref_x[i];
                if (y1 - y0).abs() < 1e-12 {
                    return x1;
                }
                let t = (pp - y0) / (y1 - y0);
                return x0 + t * (x1 - x0);
            }
        }
        *ref_x.last().unwrap_or(&pp)
    }

    fn run_histogram_equalization(input: &Raster, num_tones: usize) -> Result<Raster, ToolError> {
        let mut output = input.clone();
        let tone_max = (num_tones - 1) as f64;

        for b in 0..input.bands as isize {
            let values = Self::collect_valid_values(input, b);
            if values.is_empty() {
                continue;
            }

            let mut min_z = f64::INFINITY;
            let mut max_z = f64::NEG_INFINITY;
            for &z in &values {
                min_z = min_z.min(z);
                max_z = max_z.max(z);
            }

            if (max_z - min_z).abs() < 1e-12 {
                continue;
            }

            let cdf = Self::build_input_cdf(&values, 1024, min_z, max_z);
            let cdf_min = cdf
                .iter()
                .copied()
                .find(|&v| v > 0.0)
                .unwrap_or(0.0)
                .clamp(0.0, 1.0);

            for r in 0..input.rows as isize {
                for c in 0..input.cols as isize {
                    let z = input.get(b, r, c);
                    if input.is_nodata(z) {
                        continue;
                    }
                    let p = Self::cdf_value_for_z(z, &cdf, min_z, max_z);
                    let mapped = if (1.0 - cdf_min).abs() < 1e-12 {
                        0.0
                    } else {
                        ((p - cdf_min) / (1.0 - cdf_min)).clamp(0.0, 1.0) * tone_max
                    };
                    output.set(b, r, c, mapped).map_err(|e| {
                        ToolError::Execution(format!(
                            "failed writing histogram equalized value at ({r},{c}): {e}"
                        ))
                    })?;
                }
            }
        }

        Ok(output)
    }

    fn run_histogram_matching(
        input: &Raster,
        reference_hist: Vec<(f64, f64)>,
        is_cumulative: bool,
    ) -> Result<Raster, ToolError> {
        let (ref_x, ref_cdf) = Self::normalize_reference_histogram(reference_hist, is_cumulative)?;
        let mut output = input.clone();

        for b in 0..input.bands as isize {
            let values = Self::collect_valid_values(input, b);
            if values.is_empty() {
                continue;
            }

            let mut min_z = f64::INFINITY;
            let mut max_z = f64::NEG_INFINITY;
            for &z in &values {
                min_z = min_z.min(z);
                max_z = max_z.max(z);
            }
            if (max_z - min_z).abs() < 1e-12 {
                continue;
            }

            let cdf = Self::build_input_cdf(&values, 1024, min_z, max_z);

            for r in 0..input.rows as isize {
                for c in 0..input.cols as isize {
                    let z = input.get(b, r, c);
                    if input.is_nodata(z) {
                        continue;
                    }
                    let p = Self::cdf_value_for_z(z, &cdf, min_z, max_z);
                    let mapped = Self::map_probability_to_reference_value(p, &ref_x, &ref_cdf);
                    output.set(b, r, c, mapped).map_err(|e| {
                        ToolError::Execution(format!(
                            "failed writing histogram matched value at ({r},{c}): {e}"
                        ))
                    })?;
                }
            }
        }

        Ok(output)
    }

    fn run_histogram_matching_two_images(input: &Raster, reference: &Raster) -> Result<Raster, ToolError> {
        if reference.bands == 0 {
            return Err(ToolError::Validation(
                "reference raster must contain at least one band".to_string(),
            ));
        }
        let ref_values = Self::collect_valid_values(reference, 0);
        if ref_values.is_empty() {
            return Err(ToolError::Validation(
                "reference raster contains no valid cells".to_string(),
            ));
        }

        let mut sorted = ref_values;
        sorted.sort_by(|a, b| a.total_cmp(b));
        let n = sorted.len();
        let mut pairs = Vec::with_capacity(n.max(2));
        for (i, &z) in sorted.iter().enumerate() {
            pairs.push((z, (i + 1) as f64 / n as f64));
        }
        if pairs.len() == 1 {
            pairs.push((pairs[0].0, 1.0));
        }

        Self::run_histogram_matching(input, pairs, true)
    }

    fn run_percentage_contrast_stretch(
        input: &Raster,
        clip: f64,
        tail: TailMode,
        num_tones: usize,
    ) -> Result<Raster, ToolError> {
        let mut output = input.clone();
        let tone_max = (num_tones - 1) as f64;

        for b in 0..input.bands as isize {
            let mut values = Self::collect_valid_values(input, b);
            if values.is_empty() {
                continue;
            }
            values.sort_by(|a, b| a.total_cmp(b));

            let q = (clip / 100.0).clamp(0.0, 0.5);
            let lower_q = if matches!(tail, TailMode::Both | TailMode::Lower) {
                q
            } else {
                0.0
            };
            let upper_q = if matches!(tail, TailMode::Both | TailMode::Upper) {
                1.0 - q
            } else {
                1.0
            };

            let min_val = Self::quantile_from_sorted(&values, lower_q);
            let max_val = Self::quantile_from_sorted(&values, upper_q);
            let width = (max_val - min_val).max(1e-12);

            for r in 0..input.rows as isize {
                for c in 0..input.cols as isize {
                    let z = input.get(b, r, c);
                    if input.is_nodata(z) {
                        continue;
                    }
                    let mapped = ((z - min_val) / width).clamp(0.0, 1.0) * tone_max;
                    output.set(b, r, c, mapped).map_err(|e| {
                        ToolError::Execution(format!(
                            "failed writing contrast-stretched value at ({r},{c}): {e}"
                        ))
                    })?;
                }
            }
        }

        Ok(output)
    }

    fn run_min_max_contrast_stretch(
        input: &Raster,
        min_val: f64,
        max_val: f64,
        num_tones: usize,
    ) -> Result<Raster, ToolError> {
        let mut output = input.clone();
        let width = max_val - min_val;
        if width <= 0.0 {
            return Err(ToolError::Validation(
                "parameter 'max_val' must be greater than 'min_val'".to_string(),
            ));
        }
        let tone_max = (num_tones - 1) as f64;

        for b in 0..input.bands as isize {
            for r in 0..input.rows as isize {
                for c in 0..input.cols as isize {
                    let z = input.get(b, r, c);
                    if input.is_nodata(z) {
                        continue;
                    }
                    let mapped = ((z - min_val) / width).clamp(0.0, 1.0) * tone_max;
                    output.set(b, r, c, mapped).map_err(|e| {
                        ToolError::Execution(format!(
                            "failed writing min-max stretched value at ({r},{c}): {e}"
                        ))
                    })?;
                }
            }
        }

        Ok(output)
    }

    fn gaussian_reference_pairs(num_tones: usize) -> Vec<(f64, f64)> {
        let n = num_tones.max(2);
        let mut pairs = Vec::with_capacity(n);
        let mut running = 0.0;
        let step = 6.0 / (n - 1) as f64;
        let norm = (2.0 * std::f64::consts::PI).sqrt();
        for i in 0..n {
            let x = -3.0 + i as f64 * step;
            let p = (-0.5 * x * x).exp() / norm;
            running += p;
            pairs.push((x, running));
        }
        if let Some((_, end)) = pairs.last().copied() {
            if end > 0.0 {
                for pair in &mut pairs {
                    pair.1 /= end;
                }
            }
        }
        pairs
    }

    fn run_gaussian_contrast_stretch(input: &Raster, num_tones: usize) -> Result<Raster, ToolError> {
        let reference = Self::gaussian_reference_pairs(num_tones);
        let mut matched = Self::run_histogram_matching(input, reference, true)?;
        let tone_max = (num_tones.max(2) - 1) as f64;

        for b in 0..matched.bands as isize {
            for r in 0..matched.rows as isize {
                for c in 0..matched.cols as isize {
                    let z = matched.get(b, r, c);
                    if matched.is_nodata(z) {
                        continue;
                    }
                    let mapped = ((z + 3.0) / 6.0).clamp(0.0, 1.0) * tone_max;
                    matched.set(b, r, c, mapped).map_err(|e| {
                        ToolError::Execution(format!(
                            "failed writing gaussian contrast-stretched value at ({r},{c}): {e}"
                        ))
                    })?;
                }
            }
        }

        Ok(matched)
    }

    fn run_sigmoidal_contrast_stretch(
        input: &Raster,
        cutoff: f64,
        gain: f64,
        num_tones: usize,
    ) -> Result<Raster, ToolError> {
        let mut output = input.clone();
        let tone_max = (num_tones.max(2) - 1) as f64;

        for b in 0..input.bands as isize {
            let values = Self::collect_valid_values(input, b);
            if values.is_empty() {
                continue;
            }
            let mut min_z = f64::INFINITY;
            let mut max_z = f64::NEG_INFINITY;
            for &z in &values {
                min_z = min_z.min(z);
                max_z = max_z.max(z);
            }

            let width = (max_z - min_z).max(1e-12);
            let a = 1.0 / (1.0 + (gain * cutoff).exp());
            let bcoef = 1.0 / (1.0 + (gain * (cutoff - 1.0)).exp()) - a;
            let denom = if bcoef.abs() < 1e-12 { 1.0 } else { bcoef };

            for r in 0..input.rows as isize {
                for c in 0..input.cols as isize {
                    let z = input.get(b, r, c);
                    if input.is_nodata(z) {
                        continue;
                    }
                    let zn = ((z - min_z) / width).clamp(0.0, 1.0);
                    let mut out = (1.0 / (1.0 + (gain * (cutoff - zn)).exp()) - a) / denom;
                    out = out.clamp(0.0, 1.0) * tone_max;
                    output.set(b, r, c, out).map_err(|e| {
                        ToolError::Execution(format!(
                            "failed writing sigmoidal contrast-stretched value at ({r},{c}): {e}"
                        ))
                    })?;
                }
            }
        }

        Ok(output)
    }

    fn run_standard_deviation_contrast_stretch(
        input: &Raster,
        clip: f64,
        num_tones: usize,
    ) -> Result<Raster, ToolError> {
        let mut output = input.clone();
        let tone_max = (num_tones.max(2) - 1) as f64;

        for b in 0..input.bands as isize {
            let values = Self::collect_valid_values(input, b);
            if values.is_empty() {
                continue;
            }

            let n = values.len() as f64;
            let mean = values.iter().sum::<f64>() / n;
            let variance = if values.len() > 1 {
                values
                    .iter()
                    .map(|z| {
                        let d = *z - mean;
                        d * d
                    })
                    .sum::<f64>()
                    / (n - 1.0)
            } else {
                0.0
            };
            let stdev = variance.sqrt();
            let min_val = mean - stdev * clip;
            let max_val = mean + stdev * clip;
            let width = (max_val - min_val).max(1e-12);

            for r in 0..input.rows as isize {
                for c in 0..input.cols as isize {
                    let z = input.get(b, r, c);
                    if input.is_nodata(z) {
                        continue;
                    }
                    let mapped = ((z - min_val) / width).clamp(0.0, 1.0) * tone_max;
                    output.set(b, r, c, mapped).map_err(|e| {
                        ToolError::Execution(format!(
                            "failed writing standard-deviation stretched value at ({r},{c}): {e}"
                        ))
                    })?;
                }
            }
        }

        Ok(output)
    }

    fn run_with_op(op: NonFilterOp, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info(&format!("running {}", op.id()));
        let output = match op {
            NonFilterOp::BalanceContrastEnhancement => {
                let input_path = Self::parse_input(args)?;
                let input = Self::load_raster(&input_path)?;
                let band_mean = Self::parse_band_mean(args);
                Self::run_balance_contrast_enhancement(&input, band_mean)?
            }
            NonFilterOp::CreateColourComposite => {
                let (red_path, green_path, blue_path, opacity_path, enhance, treat_zeros_as_nodata) =
                    Self::parse_create_colour_inputs(args)?;
                let red = Self::load_raster(&red_path)?;
                let green = Self::load_raster(&green_path)?;
                let blue = Self::load_raster(&blue_path)?;
                let opacity = match opacity_path {
                    Some(path) => Some(Self::load_raster(&path)?),
                    None => None,
                };
                Self::run_create_colour_composite(
                    &red,
                    &green,
                    &blue,
                    opacity.as_ref(),
                    enhance,
                    treat_zeros_as_nodata,
                )?
            }
            NonFilterOp::DirectDecorrelationStretch => {
                let input_path = Self::parse_input(args)?;
                let input = Self::load_raster(&input_path)?;
                let achromatic_factor = Self::parse_achromatic_factor(args);
                let clip_percent = Self::parse_clip_percent_fraction(args);
                Self::run_direct_decorrelation_stretch(&input, achromatic_factor, clip_percent)?
            }
            NonFilterOp::FlipImage => {
                let input_path = Self::parse_input(args)?;
                let input = Self::load_raster(&input_path)?;
                let direction = Self::parse_flip_direction(args);
                Self::run_flip(&input, direction)?
            }
            NonFilterOp::HistogramEqualization => {
                let input_path = Self::parse_input(args)?;
                let input = Self::load_raster(&input_path)?;
                let num_tones = Self::parse_num_tones(args);
                Self::run_histogram_equalization(&input, num_tones)?
            }
            NonFilterOp::HistogramMatching => {
                let input_path = Self::parse_input(args)?;
                let input = Self::load_raster(&input_path)?;
                let histogram = Self::parse_histogram_pairs(args)?;
                let is_cumulative = Self::parse_is_cumulative(args);
                Self::run_histogram_matching(&input, histogram, is_cumulative)?
            }
            NonFilterOp::HistogramMatchingTwoImages => {
                let input_path = Self::parse_input(args)?;
                let input = Self::load_raster(&input_path)?;
                let reference_path = Self::parse_reference_path(args)?;
                let reference = Self::load_raster(&reference_path)?;
                Self::run_histogram_matching_two_images(&input, &reference)?
            }
            NonFilterOp::IntegralImageTransform => {
                let input_path = Self::parse_input(args)?;
                let input = Self::load_raster(&input_path)?;
                Self::run_integral(&input)?
            }
            NonFilterOp::GaussianContrastStretch => {
                let input_path = Self::parse_input(args)?;
                let input = Self::load_raster(&input_path)?;
                let num_tones = Self::parse_num_tones(args);
                Self::run_gaussian_contrast_stretch(&input, num_tones)?
            }
            NonFilterOp::MinMaxContrastStretch => {
                let input_path = Self::parse_input(args)?;
                let input = Self::load_raster(&input_path)?;
                let min_val = Self::parse_min_val(args)?;
                let max_val = Self::parse_max_val(args)?;
                let num_tones = Self::parse_num_tones(args);
                Self::run_min_max_contrast_stretch(&input, min_val, max_val, num_tones)?
            }
            NonFilterOp::NormalizedDifferenceIndex => {
                let input_path = Self::parse_input(args)?;
                let input = Self::load_raster(&input_path)?;
                let band1 = Self::parse_band_index(args, "band1", 1);
                let band2 = Self::parse_band_index(args, "band2", 2);
                Self::run_ndi(&input, band1, band2)?
            }
            NonFilterOp::Closing => {
                let input_path = Self::parse_input(args)?;
                let input = Self::load_raster(&input_path)?;
                let fx = Self::parse_filter_size(args, "filter_size_x", 11);
                let fy = Self::parse_filter_size(args, "filter_size_y", fx);
                Self::run_closing(&input, fx, fy)?
            }
            NonFilterOp::Opening => {
                let input_path = Self::parse_input(args)?;
                let input = Self::load_raster(&input_path)?;
                let fx = Self::parse_filter_size(args, "filter_size_x", 11);
                let fy = Self::parse_filter_size(args, "filter_size_y", fx);
                Self::run_opening(&input, fx, fy)?
            }
            NonFilterOp::CornerDetection => {
                let input_path = Self::parse_input(args)?;
                let input = Self::load_raster(&input_path)?;
                Self::run_corner_detection(&input)?
            }
            NonFilterOp::OtsuThresholding => {
                let input_path = Self::parse_input(args)?;
                let input = Self::load_raster(&input_path)?;
                Self::run_otsu_thresholding(&input)?
            }
            NonFilterOp::PercentageContrastStretch => {
                let input_path = Self::parse_input(args)?;
                let input = Self::load_raster(&input_path)?;
                let clip = Self::parse_clip_percent(args);
                let tail = Self::parse_tail_mode(args);
                let num_tones = Self::parse_num_tones(args);
                Self::run_percentage_contrast_stretch(&input, clip, tail, num_tones)?
            }
            NonFilterOp::RemoveSpurs => {
                let input_path = Self::parse_input(args)?;
                let input = Self::load_raster(&input_path)?;
                let max_iterations = Self::parse_max_iterations(args);
                Self::run_remove_spurs(&input, max_iterations)?
            }
            NonFilterOp::SigmoidalContrastStretch => {
                let input_path = Self::parse_input(args)?;
                let input = Self::load_raster(&input_path)?;
                let cutoff = Self::parse_sigmoid_cutoff(args);
                let gain = Self::parse_sigmoid_gain(args);
                let num_tones = Self::parse_num_tones(args);
                Self::run_sigmoidal_contrast_stretch(&input, cutoff, gain, num_tones)?
            }
            NonFilterOp::StandardDeviationContrastStretch => {
                let input_path = Self::parse_input(args)?;
                let input = Self::load_raster(&input_path)?;
                let clip = Self::parse_stdev_clip(args);
                let num_tones = Self::parse_num_tones(args);
                Self::run_standard_deviation_contrast_stretch(&input, clip, num_tones)?
            }
            NonFilterOp::ThickenRasterLine => {
                let input_path = Self::parse_input(args)?;
                let input = Self::load_raster(&input_path)?;
                Self::run_thicken_raster_line(&input)?
            }
            NonFilterOp::TophatTransform => {
                let input_path = Self::parse_input(args)?;
                let input = Self::load_raster(&input_path)?;
                let fx = Self::parse_filter_size(args, "filter_size_x", 11);
                let fy = Self::parse_filter_size(args, "filter_size_y", fx);
                let variant = Self::parse_tophat_variant(args);
                Self::run_tophat_transform(&input, fx, fy, variant)?
            }
            NonFilterOp::LineThinning => {
                let input_path = Self::parse_input(args)?;
                let input = Self::load_raster(&input_path)?;
                Self::run_line_thinning(&input)?
            }
        };

        ctx.progress.progress(1.0);
        let output_locator = Self::write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("active_band".to_string(), json!(0));
        Ok(ToolRunResult { outputs })
    }
}

macro_rules! define_non_filter_tool {
    ($tool:ident, $op:expr) => {
        impl Tool for $tool {
            fn metadata(&self) -> ToolMetadata {
                FlipImageTool::metadata_for($op)
            }

            fn manifest(&self) -> ToolManifest {
                FlipImageTool::manifest_for($op)
            }

            fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
                let _ = FlipImageTool::parse_input(args)?;
                let _ = parse_optional_output_path(args, "output")?;
                Ok(())
            }

            fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
                FlipImageTool::run_with_op($op, args, ctx)
            }
        }
    };
}

define_non_filter_tool!(
    BalanceContrastEnhancementTool,
    NonFilterOp::BalanceContrastEnhancement
);
define_non_filter_tool!(
    DirectDecorrelationStretchTool,
    NonFilterOp::DirectDecorrelationStretch
);
define_non_filter_tool!(FlipImageTool, NonFilterOp::FlipImage);
define_non_filter_tool!(HistogramEqualizationTool, NonFilterOp::HistogramEqualization);
define_non_filter_tool!(HistogramMatchingTool, NonFilterOp::HistogramMatching);
define_non_filter_tool!(
    HistogramMatchingTwoImagesTool,
    NonFilterOp::HistogramMatchingTwoImages
);
define_non_filter_tool!(IntegralImageTransformTool, NonFilterOp::IntegralImageTransform);
define_non_filter_tool!(GaussianContrastStretchTool, NonFilterOp::GaussianContrastStretch);
define_non_filter_tool!(MinMaxContrastStretchTool, NonFilterOp::MinMaxContrastStretch);
define_non_filter_tool!(NormalizedDifferenceIndexTool, NonFilterOp::NormalizedDifferenceIndex);
define_non_filter_tool!(ClosingTool, NonFilterOp::Closing);
define_non_filter_tool!(CornerDetectionTool, NonFilterOp::CornerDetection);
define_non_filter_tool!(OpeningTool, NonFilterOp::Opening);
define_non_filter_tool!(OtsuThresholdingTool, NonFilterOp::OtsuThresholding);
define_non_filter_tool!(
    PercentageContrastStretchTool,
    NonFilterOp::PercentageContrastStretch
);
define_non_filter_tool!(RemoveSpursTool, NonFilterOp::RemoveSpurs);
define_non_filter_tool!(SigmoidalContrastStretchTool, NonFilterOp::SigmoidalContrastStretch);
define_non_filter_tool!(
    StandardDeviationContrastStretchTool,
    NonFilterOp::StandardDeviationContrastStretch
);
define_non_filter_tool!(ThickenRasterLineTool, NonFilterOp::ThickenRasterLine);
define_non_filter_tool!(TophatTransformTool, NonFilterOp::TophatTransform);
define_non_filter_tool!(LineThinningTool, NonFilterOp::LineThinning);

// ── SplitColourCompositeTool ─────────────────────────────────────────────────

impl Tool for SplitColourCompositeTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "split_colour_composite",
            display_name: "Split Colour Composite",
            summary: "Splits a packed RGB colour composite into separate red, green, and blue single-band rasters.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input packed RGB raster.",
                    required: true,
                },
                ToolParamSpec {
                    name: "red_output",
                    description: "Optional output path for the red band.",
                    required: false,
                },
                ToolParamSpec {
                    name: "green_output",
                    description: "Optional output path for the green band.",
                    required: false,
                },
                ToolParamSpec {
                    name: "blue_output",
                    description: "Optional output path for the blue band.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("composite.tif"));
        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("composite.tif"));
        example.insert("red_output".to_string(), json!("red.tif"));
        example.insert("green_output".to_string(), json!("green.tif"));
        example.insert("blue_output".to_string(), json!("blue.tif"));
        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta.params.into_iter().map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            }).collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_split_colour_composite".to_string(),
                description: "Split a colour composite into R/G/B bands.".to_string(),
                args: example,
            }],
            tags: vec!["remote_sensing".to_string(), "raster".to_string(), "split_colour_composite".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = FlipImageTool::parse_input(args)?;
        let input = FlipImageTool::load_raster(&input_path)?;
        let red_out_path = parse_optional_output_path(args, "red_output")?;
        let green_out_path = parse_optional_output_path(args, "green_output")?;
        let blue_out_path = parse_optional_output_path(args, "blue_output")?;
        let (red, green, blue) = run_split_colour_composite(&input)?;
        ctx.progress.progress(1.0);
        let mut outputs = BTreeMap::new();
        outputs.insert("red".to_string(), FlipImageTool::store_named_raster_output(red, red_out_path)?);
        outputs.insert("green".to_string(), FlipImageTool::store_named_raster_output(green, green_out_path)?);
        outputs.insert("blue".to_string(), FlipImageTool::store_named_raster_output(blue, blue_out_path)?);
        Ok(ToolRunResult { outputs })
    }
}

// ── RgbToIhsTool ─────────────────────────────────────────────────────────────

impl Tool for RgbToIhsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "rgb_to_ihs",
            display_name: "RGB to IHS",
            summary: "Transforms red, green, blue band rasters (or a packed composite) to intensity, hue, and saturation components.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "red",
                    description: "Red-band raster (mutually exclusive with 'composite').",
                    required: false,
                },
                ToolParamSpec {
                    name: "green",
                    description: "Green-band raster (mutually exclusive with 'composite').",
                    required: false,
                },
                ToolParamSpec {
                    name: "blue",
                    description: "Blue-band raster (mutually exclusive with 'composite').",
                    required: false,
                },
                ToolParamSpec {
                    name: "composite",
                    description: "Packed RGB composite raster (mutually exclusive with red/green/blue).",
                    required: false,
                },
                ToolParamSpec {
                    name: "intensity_output",
                    description: "Optional output path for the intensity band.",
                    required: false,
                },
                ToolParamSpec {
                    name: "hue_output",
                    description: "Optional output path for the hue band.",
                    required: false,
                },
                ToolParamSpec {
                    name: "saturation_output",
                    description: "Optional output path for the saturation band.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("red".to_string(), json!("red.tif"));
        defaults.insert("green".to_string(), json!("green.tif"));
        defaults.insert("blue".to_string(), json!("blue.tif"));
        let mut example = ToolArgs::new();
        example.insert("red".to_string(), json!("red.tif"));
        example.insert("green".to_string(), json!("green.tif"));
        example.insert("blue".to_string(), json!("blue.tif"));
        example.insert("intensity_output".to_string(), json!("intensity.tif"));
        example.insert("hue_output".to_string(), json!("hue.tif"));
        example.insert("saturation_output".to_string(), json!("saturation.tif"));
        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta.params.into_iter().map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            }).collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_rgb_to_ihs".to_string(),
                description: "Convert an RGB triple to IHS.".to_string(),
                args: example,
            }],
            tags: vec!["remote_sensing".to_string(), "raster".to_string(), "rgb_to_ihs".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let has_rgb = args.contains_key("red") || args.contains_key("green") || args.contains_key("blue");
        let has_composite = args.contains_key("composite");
        if !has_rgb && !has_composite {
            return Err(ToolError::Validation(
                "provide either 'red'/'green'/'blue' or 'composite'".to_string(),
            ));
        }
        if has_rgb && has_composite {
            return Err(ToolError::Validation(
                "'composite' cannot be combined with 'red'/'green'/'blue'".to_string(),
            ));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let intensity_out = parse_optional_output_path(args, "intensity_output")?;
        let hue_out = parse_optional_output_path(args, "hue_output")?;
        let sat_out = parse_optional_output_path(args, "saturation_output")?;

        let use_composite = args.contains_key("composite");
        let (intensity, hue, saturation) = if use_composite {
            let path = parse_raster_path_arg(args, "composite")?;
            let composite = FlipImageTool::load_raster(&path)?;
            run_rgb_to_ihs_from_composite(&composite)?
        } else {
            let red_path = parse_raster_path_arg(args, "red")?;
            let green_path = parse_raster_path_arg(args, "green")?;
            let blue_path = parse_raster_path_arg(args, "blue")?;
            let red = FlipImageTool::load_raster(&red_path)?;
            let green = FlipImageTool::load_raster(&green_path)?;
            let blue = FlipImageTool::load_raster(&blue_path)?;
            run_rgb_to_ihs_from_bands(&red, &green, &blue)?
        };

        ctx.progress.progress(1.0);
        let mut outputs = BTreeMap::new();
        outputs.insert("intensity".to_string(), FlipImageTool::store_named_raster_output(intensity, intensity_out)?);
        outputs.insert("hue".to_string(), FlipImageTool::store_named_raster_output(hue, hue_out)?);
        outputs.insert("saturation".to_string(), FlipImageTool::store_named_raster_output(saturation, sat_out)?);
        Ok(ToolRunResult { outputs })
    }
}

// ── IhsToRgbTool ─────────────────────────────────────────────────────────────

impl Tool for IhsToRgbTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "ihs_to_rgb",
            display_name: "IHS to RGB",
            summary: "Converts intensity, hue, and saturation band rasters back to red, green, and blue channels (0–255).",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "intensity",
                    description: "Intensity band raster (0–1).",
                    required: true,
                },
                ToolParamSpec {
                    name: "hue",
                    description: "Hue band raster (0–2π radians).",
                    required: true,
                },
                ToolParamSpec {
                    name: "saturation",
                    description: "Saturation band raster (0–1).",
                    required: true,
                },
                ToolParamSpec {
                    name: "red_output",
                    description: "Optional output path for the red band.",
                    required: false,
                },
                ToolParamSpec {
                    name: "green_output",
                    description: "Optional output path for the green band.",
                    required: false,
                },
                ToolParamSpec {
                    name: "blue_output",
                    description: "Optional output path for the blue band.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("intensity".to_string(), json!("intensity.tif"));
        defaults.insert("hue".to_string(), json!("hue.tif"));
        defaults.insert("saturation".to_string(), json!("saturation.tif"));
        let mut example = ToolArgs::new();
        example.insert("intensity".to_string(), json!("intensity.tif"));
        example.insert("hue".to_string(), json!("hue.tif"));
        example.insert("saturation".to_string(), json!("saturation.tif"));
        example.insert("red_output".to_string(), json!("red.tif"));
        example.insert("green_output".to_string(), json!("green.tif"));
        example.insert("blue_output".to_string(), json!("blue.tif"));
        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta.params.into_iter().map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            }).collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_ihs_to_rgb".to_string(),
                description: "Reconstruct RGB channels from IHS components.".to_string(),
                args: example,
            }],
            tags: vec!["remote_sensing".to_string(), "raster".to_string(), "ihs_to_rgb".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "intensity")?;
        let _ = parse_raster_path_arg(args, "hue")?;
        let _ = parse_raster_path_arg(args, "saturation")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let intensity_path = parse_raster_path_arg(args, "intensity")?;
        let hue_path = parse_raster_path_arg(args, "hue")?;
        let sat_path = parse_raster_path_arg(args, "saturation")?;
        let intensity = FlipImageTool::load_raster(&intensity_path)?;
        let hue = FlipImageTool::load_raster(&hue_path)?;
        let saturation = FlipImageTool::load_raster(&sat_path)?;
        let red_out = parse_optional_output_path(args, "red_output")?;
        let green_out = parse_optional_output_path(args, "green_output")?;
        let blue_out = parse_optional_output_path(args, "blue_output")?;
        let (red, green, blue) = run_ihs_to_rgb(&intensity, &hue, &saturation)?;
        ctx.progress.progress(1.0);
        let mut outputs = BTreeMap::new();
        outputs.insert("red".to_string(), FlipImageTool::store_named_raster_output(red, red_out)?);
        outputs.insert("green".to_string(), FlipImageTool::store_named_raster_output(green, green_out)?);
        outputs.insert("blue".to_string(), FlipImageTool::store_named_raster_output(blue, blue_out)?);
        Ok(ToolRunResult { outputs })
    }
}

// ── ChangeVectorAnalysisTool ────────────────────────────────────────────────

impl Tool for ChangeVectorAnalysisTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "change_vector_analysis",
            display_name: "Change Vector Analysis",
            summary: "Performs change vector analysis on two-date multispectral datasets and returns magnitude and direction rasters.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "date1",
                    description: "Earlier-date raster list as an array of paths or a comma/semicolon-delimited string.",
                    required: true,
                },
                ToolParamSpec {
                    name: "date2",
                    description: "Later-date raster list as an array of paths or a comma/semicolon-delimited string.",
                    required: true,
                },
                ToolParamSpec {
                    name: "magnitude_output",
                    description: "Optional output path for vector magnitude raster.",
                    required: false,
                },
                ToolParamSpec {
                    name: "direction_output",
                    description: "Optional output path for direction-code raster.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("date1".to_string(), json!(["d1_band1.tif", "d1_band2.tif", "d1_band3.tif"]));
        defaults.insert("date2".to_string(), json!(["d2_band1.tif", "d2_band2.tif", "d2_band3.tif"]));

        let mut example = ToolArgs::new();
        example.insert("date1".to_string(), json!(["d1_band1.tif", "d1_band2.tif", "d1_band3.tif"]));
        example.insert("date2".to_string(), json!(["d2_band1.tif", "d2_band2.tif", "d2_band3.tif"]));
        example.insert("magnitude_output".to_string(), json!("cva_magnitude.tif"));
        example.insert("direction_output".to_string(), json!("cva_direction.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_change_vector_analysis".to_string(),
                description: "Runs CVA on paired multispectral dates.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "raster".to_string(),
                "change_vector_analysis".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let d1 = parse_raster_list_arg(args, "date1")?;
        let d2 = parse_raster_list_arg(args, "date2")?;
        if d1.is_empty() || d2.is_empty() {
            return Err(ToolError::Validation(
                "parameters 'date1' and 'date2' must each contain at least one raster".to_string(),
            ));
        }
        if d1.len() != d2.len() {
            return Err(ToolError::Validation(
                "parameters 'date1' and 'date2' must contain the same number of rasters".to_string(),
            ));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let date1_paths = parse_raster_list_arg(args, "date1")?;
        let date2_paths = parse_raster_list_arg(args, "date2")?;
        if date1_paths.is_empty() || date2_paths.is_empty() {
            return Err(ToolError::Validation(
                "parameters 'date1' and 'date2' must each contain at least one raster".to_string(),
            ));
        }
        if date1_paths.len() != date2_paths.len() {
            return Err(ToolError::Validation(
                "parameters 'date1' and 'date2' must contain the same number of rasters".to_string(),
            ));
        }

        let mut date1 = Vec::with_capacity(date1_paths.len());
        let mut date2 = Vec::with_capacity(date2_paths.len());
        for p in &date1_paths {
            date1.push(FlipImageTool::load_raster(p)?);
        }
        for p in &date2_paths {
            date2.push(FlipImageTool::load_raster(p)?);
        }

        let magnitude_output = parse_optional_output_path(args, "magnitude_output")?;
        let direction_output = parse_optional_output_path(args, "direction_output")?;

        let (magnitude, direction) = run_change_vector_analysis(&date1, &date2)?;

        ctx.progress.progress(1.0);
        let mut outputs = BTreeMap::new();
        outputs.insert(
            "magnitude".to_string(),
            FlipImageTool::store_named_raster_output(magnitude, magnitude_output)?,
        );
        outputs.insert(
            "direction".to_string(),
            FlipImageTool::store_named_raster_output(direction, direction_output)?,
        );
        Ok(ToolRunResult { outputs })
    }
}

// ── WriteFunctionMemoryInsertionTool ────────────────────────────────────────

impl Tool for WriteFunctionMemoryInsertionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "write_function_memory_insertion",
            display_name: "Write Function Memory Insertion",
            summary: "Creates a packed RGB change-visualization composite from two or three single-band dates.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input1",
                    description: "First-date single-band raster.",
                    required: true,
                },
                ToolParamSpec {
                    name: "input2",
                    description: "Second-date single-band raster.",
                    required: true,
                },
                ToolParamSpec {
                    name: "input3",
                    description: "Optional third-date single-band raster; if omitted, input2 is used for blue channel.",
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

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("input1".to_string(), json!("date1.tif"));
        defaults.insert("input2".to_string(), json!("date2.tif"));

        let mut example = ToolArgs::new();
        example.insert("input1".to_string(), json!("date1.tif"));
        example.insert("input2".to_string(), json!("date2.tif"));
        example.insert("input3".to_string(), json!("date3.tif"));
        example.insert("output".to_string(), json!("wfmi.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_write_function_memory_insertion".to_string(),
                description: "Creates a WFM insertion RGB composite for qualitative change detection.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "raster".to_string(),
                "write_function_memory_insertion".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input1")?;
        let _ = parse_raster_path_arg(args, "input2")?;
        if args.contains_key("input3") {
            let _ = parse_raster_path_arg(args, "input3")?;
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let i1 = parse_raster_path_arg(args, "input1")?;
        let i2 = parse_raster_path_arg(args, "input2")?;
        let i3 = if args.contains_key("input3") {
            Some(parse_raster_path_arg(args, "input3")?)
        } else {
            None
        };
        let output_path = parse_optional_output_path(args, "output")?;

        let red = FlipImageTool::load_raster(&i1)?;
        let green = FlipImageTool::load_raster(&i2)?;
        let blue = match i3 {
            Some(p) => FlipImageTool::load_raster(&p)?,
            None => FlipImageTool::load_raster(&i2)?,
        };

        let output = run_write_function_memory_insertion(&red, &green, &blue)?;
        let output_locator = FlipImageTool::write_or_store_output(output, output_path)?;

        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("active_band".to_string(), json!(0));
        Ok(ToolRunResult { outputs })
    }
}

// ── PanchromaticSharpeningTool ──────────────────────────────────────────────

impl Tool for PanchromaticSharpeningTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "panchromatic_sharpening",
            display_name: "Panchromatic Sharpening",
            summary: "Fuses multispectral and panchromatic rasters using Brovey or IHS methods.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "red",
                    description: "Red-band raster (required unless 'composite' is provided).",
                    required: false,
                },
                ToolParamSpec {
                    name: "green",
                    description: "Green-band raster (required unless 'composite' is provided).",
                    required: false,
                },
                ToolParamSpec {
                    name: "blue",
                    description: "Blue-band raster (required unless 'composite' is provided).",
                    required: false,
                },
                ToolParamSpec {
                    name: "composite",
                    description: "Packed RGB multispectral composite (mutually exclusive with red/green/blue).",
                    required: false,
                },
                ToolParamSpec {
                    name: "pan",
                    description: "Panchromatic raster band.",
                    required: true,
                },
                ToolParamSpec {
                    name: "method",
                    description: "Fusion method: 'brovey' (default) or 'ihs'.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output_mode",
                    description: "Output encoding: 'packed' (default) or 'bands' (3-band RGB raster).",
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

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("red".to_string(), json!("red.tif"));
        defaults.insert("green".to_string(), json!("green.tif"));
        defaults.insert("blue".to_string(), json!("blue.tif"));
        defaults.insert("pan".to_string(), json!("pan.tif"));
        defaults.insert("method".to_string(), json!("brovey"));
        defaults.insert("output_mode".to_string(), json!("packed"));

        let mut example = ToolArgs::new();
        example.insert("composite".to_string(), json!("multispectral_composite.tif"));
        example.insert("pan".to_string(), json!("pan.tif"));
        example.insert("method".to_string(), json!("ihs"));
        example.insert("output_mode".to_string(), json!("bands"));
        example.insert("output".to_string(), json!("pan_sharpened.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_panchromatic_sharpening".to_string(),
                description: "Runs panchromatic sharpening with IHS and 3-band output mode.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "raster".to_string(),
                "panchromatic_sharpening".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let has_rgb = args.contains_key("red") || args.contains_key("green") || args.contains_key("blue");
        let has_composite = args.contains_key("composite");
        if !has_rgb && !has_composite {
            return Err(ToolError::Validation(
                "provide either 'red'/'green'/'blue' or 'composite'".to_string(),
            ));
        }
        if has_rgb && has_composite {
            return Err(ToolError::Validation(
                "'composite' cannot be combined with 'red'/'green'/'blue'".to_string(),
            ));
        }
        let _ = parse_raster_path_arg(args, "pan")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let pan_path = parse_raster_path_arg(args, "pan")?;
        let pan = FlipImageTool::load_raster(&pan_path)?;
        let method = parse_pan_sharpen_method(args);
        let output_mode = parse_pan_sharpen_output_mode(args);
        let output_path = parse_optional_output_path(args, "output")?;

        let use_composite = args.contains_key("composite");
        let ms_packed = if use_composite {
            let composite_path = parse_raster_path_arg(args, "composite")?;
            let comp = FlipImageTool::load_raster(&composite_path)?;
            FlipImageTool::validate_packed_rgb(&comp, "panchromatic_sharpening")?;
            comp
        } else {
            let red_path = parse_raster_path_arg(args, "red")?;
            let green_path = parse_raster_path_arg(args, "green")?;
            let blue_path = parse_raster_path_arg(args, "blue")?;
            let red = FlipImageTool::load_raster(&red_path)?;
            let green = FlipImageTool::load_raster(&green_path)?;
            let blue = FlipImageTool::load_raster(&blue_path)?;
            build_ms_packed_from_bands(&red, &green, &blue)?
        };

        let output = run_panchromatic_sharpening(&ms_packed, &pan, method, output_mode)?;
        let output_locator = FlipImageTool::write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("active_band".to_string(), json!(0));
        Ok(ToolRunResult { outputs })
    }
}

// ── MosaicTool ──────────────────────────────────────────────────────────────

impl Tool for MosaicTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "mosaic",
            display_name: "Mosaic",
            summary: "Mosaics two or more rasters into a new output raster using nearest-neighbour, bilinear, or cubic resampling.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "inputs",
                    description: "Input raster list as an array of paths or comma/semicolon-delimited string.",
                    required: true,
                },
                ToolParamSpec {
                    name: "method",
                    description: "Resampling method: 'nn' (default), 'bilinear', or 'cc'.",
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

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["tile1.tif", "tile2.tif"]));
        defaults.insert("method".to_string(), json!("nn"));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["tile1.tif", "tile2.tif", "tile3.tif"]));
        example.insert("method".to_string(), json!("cc"));
        example.insert("output".to_string(), json!("mosaic.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_mosaic".to_string(),
                description: "Mosaic overlapping tiles into a single raster.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "raster".to_string(),
                "mosaic".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.len() < 2 {
            return Err(ToolError::Validation(
                "parameter 'inputs' must contain at least two rasters".to_string(),
            ));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = parse_raster_list_arg(args, "inputs")?;
        if input_paths.len() < 2 {
            return Err(ToolError::Validation(
                "parameter 'inputs' must contain at least two rasters".to_string(),
            ));
        }
        let method = parse_resample_method(args, "method", ResampleMethod::Nearest);
        let output_path = parse_optional_output_path(args, "output")?;

        let mut inputs = Vec::with_capacity(input_paths.len());
        for p in &input_paths {
            inputs.push(FlipImageTool::load_raster(p)?);
        }

        let output = run_mosaic(&inputs, method)?;
        ctx.progress.progress(1.0);
        let output_locator = FlipImageTool::write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("active_band".to_string(), json!(0));
        Ok(ToolRunResult { outputs })
    }
}

// ── MosaicWithFeatheringTool ────────────────────────────────────────────────

impl Tool for MosaicWithFeatheringTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "mosaic_with_feathering",
            display_name: "Mosaic With Feathering",
            summary: "Mosaics two rasters and feather-blends overlapping cells using edge-distance weights.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input1",
                    description: "First input raster.",
                    required: true,
                },
                ToolParamSpec {
                    name: "input2",
                    description: "Second input raster.",
                    required: true,
                },
                ToolParamSpec {
                    name: "method",
                    description: "Resampling method: 'cc' (default), 'nn', or 'bilinear'.",
                    required: false,
                },
                ToolParamSpec {
                    name: "weight",
                    description: "Distance-weight exponent used for overlap feathering (default 4.0).",
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

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("input1".to_string(), json!("image1.tif"));
        defaults.insert("input2".to_string(), json!("image2.tif"));
        defaults.insert("method".to_string(), json!("cc"));
        defaults.insert("weight".to_string(), json!(4.0));

        let mut example = ToolArgs::new();
        example.insert("input1".to_string(), json!("image1.tif"));
        example.insert("input2".to_string(), json!("image2.tif"));
        example.insert("method".to_string(), json!("bilinear"));
        example.insert("weight".to_string(), json!(4.0));
        example.insert("output".to_string(), json!("mosaic_feathered.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_mosaic_with_feathering".to_string(),
                description: "Feather-blend two overlapping rasters.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "raster".to_string(),
                "mosaic_with_feathering".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input1")?;
        let _ = parse_raster_path_arg(args, "input2")?;
        if let Some(w) = args.get("weight").and_then(|v| v.as_f64()) {
            if w <= 0.0 {
                return Err(ToolError::Validation(
                    "parameter 'weight' must be greater than 0".to_string(),
                ));
            }
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input1_path = parse_raster_path_arg(args, "input1")?;
        let input2_path = parse_raster_path_arg(args, "input2")?;
        let input1 = FlipImageTool::load_raster(&input1_path)?;
        let input2 = FlipImageTool::load_raster(&input2_path)?;
        let method = parse_resample_method(args, "method", ResampleMethod::Cubic);
        let weight = args.get("weight").and_then(|v| v.as_f64()).unwrap_or(4.0);
        if weight <= 0.0 {
            return Err(ToolError::Validation(
                "parameter 'weight' must be greater than 0".to_string(),
            ));
        }
        let output_path = parse_optional_output_path(args, "output")?;

        let output = run_mosaic_with_feathering(&input1, &input2, method, weight)?;
        ctx.progress.progress(1.0);
        let output_locator = FlipImageTool::write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("active_band".to_string(), json!(0));
        Ok(ToolRunResult { outputs })
    }
}

// ── ResampleTool ────────────────────────────────────────────────────────────

impl Tool for ResampleTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "resample",
            display_name: "Resample",
            summary: "Resamples one or more input rasters to a base raster grid or to a user-defined output cell size.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "inputs",
                    description: "Input raster list as an array of paths or comma/semicolon-delimited string.",
                    required: true,
                },
                ToolParamSpec {
                    name: "base",
                    description: "Optional base raster defining output extent and grid. Takes precedence over cell_size.",
                    required: false,
                },
                ToolParamSpec {
                    name: "cell_size",
                    description: "Optional output cell size when base is not provided.",
                    required: false,
                },
                ToolParamSpec {
                    name: "method",
                    description: "Resampling method: 'cc' (default), 'nn', or 'bilinear'.",
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

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["image1.tif", "image2.tif"]));
        defaults.insert("method".to_string(), json!("cc"));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["image1.tif", "image2.tif"]));
        example.insert("cell_size".to_string(), json!(5.0));
        example.insert("method".to_string(), json!("bilinear"));
        example.insert("output".to_string(), json!("resampled.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_resample".to_string(),
                description: "Resample rasters to a new cell size grid.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "raster".to_string(),
                "resample".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.is_empty() {
            return Err(ToolError::Validation(
                "parameter 'inputs' must contain at least one raster".to_string(),
            ));
        }
        let has_base = args.contains_key("base");
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if !has_base && cell_size <= 0.0 {
            return Err(ToolError::Validation(
                "either 'base' or a positive 'cell_size' must be provided".to_string(),
            ));
        }
        if cell_size < 0.0 {
            return Err(ToolError::Validation(
                "parameter 'cell_size' must be greater than 0".to_string(),
            ));
        }
        if has_base {
            let _ = parse_raster_path_arg(args, "base")?;
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = parse_raster_list_arg(args, "inputs")?;
        if input_paths.is_empty() {
            return Err(ToolError::Validation(
                "parameter 'inputs' must contain at least one raster".to_string(),
            ));
        }

        let method = parse_resample_method(args, "method", ResampleMethod::Cubic);
        let output_path = parse_optional_output_path(args, "output")?;
        let base = if args.contains_key("base") {
            let base_path = parse_raster_path_arg(args, "base")?;
            Some(FlipImageTool::load_raster(&base_path)?)
        } else {
            None
        };
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64());

        let mut inputs = Vec::with_capacity(input_paths.len());
        for p in &input_paths {
            inputs.push(FlipImageTool::load_raster(p)?);
        }

        let output = run_resample(&inputs, base.as_ref(), cell_size, method)?;
        ctx.progress.progress(1.0);
        let output_locator = FlipImageTool::write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("active_band".to_string(), json!(0));
        Ok(ToolRunResult { outputs })
    }
}

// ── KMeansClusteringTool ────────────────────────────────────────────────────

impl Tool for KMeansClusteringTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "k_means_clustering",
            display_name: "K-Means Clustering",
            summary: "Performs k-means clustering on a multi-band raster stack and outputs a categorical class raster.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "inputs",
                    description: "Input raster list as an array of paths or comma/semicolon-delimited string.",
                    required: true,
                },
                ToolParamSpec {
                    name: "classes",
                    description: "Number of target classes (k).",
                    required: true,
                },
                ToolParamSpec {
                    name: "max_iterations",
                    description: "Maximum iteration count (default 10).",
                    required: false,
                },
                ToolParamSpec {
                    name: "class_change",
                    description: "Stop when percent of changed valid cells drops below this threshold (default 2.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "initialize",
                    description: "Centroid initialization strategy: 'diagonal' (default) or 'random'.",
                    required: false,
                },
                ToolParamSpec {
                    name: "min_class_size",
                    description: "Minimum class size used when updating centroids (default 10).",
                    required: false,
                },
                ToolParamSpec {
                    name: "out_html",
                    description: "Optional HTML report output path.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional class raster output path. If omitted, output remains in memory.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        defaults.insert("classes".to_string(), json!(8));
        defaults.insert("max_iterations".to_string(), json!(10));
        defaults.insert("class_change".to_string(), json!(2.0));
        defaults.insert("initialize".to_string(), json!("diagonal"));
        defaults.insert("min_class_size".to_string(), json!(10));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["b1.tif", "b2.tif", "b3.tif"]));
        example.insert("classes".to_string(), json!(15));
        example.insert("max_iterations".to_string(), json!(25));
        example.insert("class_change".to_string(), json!(1.5));
        example.insert("initialize".to_string(), json!("random"));
        example.insert("min_class_size".to_string(), json!(500));
        example.insert("out_html".to_string(), json!("kmeans_report.html"));
        example.insert("output".to_string(), json!("kmeans_classes.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_k_means_clustering".to_string(),
                description: "Classify multispectral bands with k-means clustering.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "raster".to_string(),
                "k_means_clustering".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.len() < 2 {
            return Err(ToolError::Validation(
                "parameter 'inputs' must contain at least two rasters".to_string(),
            ));
        }
        let classes = args
            .get("classes")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| ToolError::Validation("parameter 'classes' is required".to_string()))?
            as usize;
        if classes < 2 {
            return Err(ToolError::Validation(
                "parameter 'classes' must be >= 2".to_string(),
            ));
        }
        if let Some(v) = args.get("max_iterations").and_then(|v| v.as_u64()) {
            let mi = v as usize;
            if !(2..=250).contains(&mi) {
                return Err(ToolError::Validation(
                    "parameter 'max_iterations' must be between 2 and 250".to_string(),
                ));
            }
        }
        if let Some(v) = args.get("class_change").and_then(|v| v.as_f64()) {
            if !(0.0..=25.0).contains(&v) {
                return Err(ToolError::Validation(
                    "parameter 'class_change' must be between 0 and 25".to_string(),
                ));
            }
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = parse_raster_list_arg(args, "inputs")?;
        let mut inputs = Vec::with_capacity(input_paths.len());
        for p in &input_paths {
            inputs.push(FlipImageTool::load_raster(p)?);
        }

        let classes = args
            .get("classes")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| ToolError::Validation("parameter 'classes' is required".to_string()))?
            as usize;
        let max_iterations = args
            .get("max_iterations")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(10);
        let class_change = args
            .get("class_change")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0);
        let min_class_size = args
            .get("min_class_size")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(10);
        let initialize_random = args
            .get("initialize")
            .and_then(|v| v.as_str())
            .map(|s| s.to_ascii_lowercase().contains("rand"))
            .unwrap_or(false);
        let out_html = args
            .get("out_html")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let output_path = parse_optional_output_path(args, "output")?;

        let result = run_kmeans(
            &inputs,
            KMeansOptions {
                classes,
                max_iterations,
                class_change_threshold: class_change,
                min_class_size,
                initialize_random,
                merge_distance: None,
            },
        )?;

        if let Some(path) = out_html.as_ref() {
            write_cluster_html_report(path, "k-Means Clustering", &input_paths, &result)?;
        }

        ctx.progress.progress(1.0);
        let output_locator = FlipImageTool::write_or_store_output(result.raster, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("active_band".to_string(), json!(0));
        outputs.insert("num_classes".to_string(), json!(result.centroids.len()));
        if let Some(path) = out_html {
            outputs.insert("report_path".to_string(), json!(path));
        }
        Ok(ToolRunResult { outputs })
    }
}

// ── ModifiedKMeansClusteringTool ────────────────────────────────────────────

impl Tool for ModifiedKMeansClusteringTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "modified_k_means_clustering",
            display_name: "Modified K-Means Clustering",
            summary: "Performs modified k-means clustering with centroid merging based on a user-defined merge distance.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "inputs",
                    description: "Input raster list as an array of paths or comma/semicolon-delimited string.",
                    required: true,
                },
                ToolParamSpec {
                    name: "start_clusters",
                    description: "Initial number of clusters before merging (default 1000).",
                    required: false,
                },
                ToolParamSpec {
                    name: "merge_dist",
                    description: "Cluster merge distance threshold (Euclidean).",
                    required: true,
                },
                ToolParamSpec {
                    name: "max_iterations",
                    description: "Maximum iteration count (default 10).",
                    required: false,
                },
                ToolParamSpec {
                    name: "class_change",
                    description: "Stop when percent of changed valid cells drops below this threshold (default 2.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "out_html",
                    description: "Optional HTML report output path.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional class raster output path. If omitted, output remains in memory.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        defaults.insert("start_clusters".to_string(), json!(1000));
        defaults.insert("merge_dist".to_string(), json!(30.0));
        defaults.insert("max_iterations".to_string(), json!(10));
        defaults.insert("class_change".to_string(), json!(2.0));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["b1.tif", "b2.tif", "b3.tif"]));
        example.insert("start_clusters".to_string(), json!(100));
        example.insert("merge_dist".to_string(), json!(25.0));
        example.insert("max_iterations".to_string(), json!(25));
        example.insert("class_change".to_string(), json!(1.5));
        example.insert("out_html".to_string(), json!("modified_kmeans_report.html"));
        example.insert("output".to_string(), json!("modified_kmeans_classes.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_modified_k_means_clustering".to_string(),
                description: "Classify multispectral bands using modified k-means with centroid merging.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "raster".to_string(),
                "modified_k_means_clustering".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.len() < 2 {
            return Err(ToolError::Validation(
                "parameter 'inputs' must contain at least two rasters".to_string(),
            ));
        }
        let merge_dist = args
            .get("merge_dist")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'merge_dist' is required".to_string()))?;
        if merge_dist <= 0.0 {
            return Err(ToolError::Validation(
                "parameter 'merge_dist' must be greater than 0".to_string(),
            ));
        }
        if let Some(v) = args.get("max_iterations").and_then(|v| v.as_u64()) {
            let mi = v as usize;
            if !(2..=250).contains(&mi) {
                return Err(ToolError::Validation(
                    "parameter 'max_iterations' must be between 2 and 250".to_string(),
                ));
            }
        }
        if let Some(v) = args.get("class_change").and_then(|v| v.as_f64()) {
            if !(0.0..=25.0).contains(&v) {
                return Err(ToolError::Validation(
                    "parameter 'class_change' must be between 0 and 25".to_string(),
                ));
            }
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = parse_raster_list_arg(args, "inputs")?;
        let mut inputs = Vec::with_capacity(input_paths.len());
        for p in &input_paths {
            inputs.push(FlipImageTool::load_raster(p)?);
        }

        let start_clusters = args
            .get("start_clusters")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1000);
        let merge_dist = args
            .get("merge_dist")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'merge_dist' is required".to_string()))?;
        let max_iterations = args
            .get("max_iterations")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(10);
        let class_change = args
            .get("class_change")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0);
        let out_html = args
            .get("out_html")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let output_path = parse_optional_output_path(args, "output")?;

        let result = run_kmeans(
            &inputs,
            KMeansOptions {
                classes: start_clusters,
                max_iterations,
                class_change_threshold: class_change,
                min_class_size: 1,
                initialize_random: true,
                merge_distance: Some(merge_dist),
            },
        )?;

        if let Some(path) = out_html.as_ref() {
            write_cluster_html_report(path, "Modified k-Means Clustering", &input_paths, &result)?;
        }

        ctx.progress.progress(1.0);
        let output_locator = FlipImageTool::write_or_store_output(result.raster, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("active_band".to_string(), json!(0));
        outputs.insert("num_classes".to_string(), json!(result.centroids.len()));
        if let Some(path) = out_html {
            outputs.insert("report_path".to_string(), json!(path));
        }
        Ok(ToolRunResult { outputs })
    }
}

// ── CorrectVignettingTool ───────────────────────────────────────────────────

impl Tool for CorrectVignettingTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "correct_vignetting",
            display_name: "Correct Vignetting",
            summary: "Reduces brightness fall-off away from a principal point using a cosine lens model.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input raster image.",
                    required: true,
                },
                ToolParamSpec {
                    name: "pp",
                    description: "Point vector path (or typed vector object) containing the principal point.",
                    required: true,
                },
                ToolParamSpec {
                    name: "focal_length",
                    description: "Camera focal length in mm (default 304.8).",
                    required: false,
                },
                ToolParamSpec {
                    name: "image_width",
                    description: "Distance between left and right image edges in mm (default 228.6).",
                    required: false,
                },
                ToolParamSpec {
                    name: "n",
                    description: "Cosine model exponent n (default 4.0).",
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

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("pp".to_string(), json!("principal_point.geojson"));
        defaults.insert("focal_length".to_string(), json!(304.8));
        defaults.insert("image_width".to_string(), json!(228.6));
        defaults.insert("n".to_string(), json!(4.0));

        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("input.tif"));
        example.insert("pp".to_string(), json!("principal_point.geojson"));
        example.insert("focal_length".to_string(), json!(304.8));
        example.insert("image_width".to_string(), json!(228.6));
        example.insert("n".to_string(), json!(4.0));
        example.insert("output".to_string(), json!("corrected.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_correct_vignetting".to_string(),
                description: "Correct vignetting using a known principal point.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "raster".to_string(),
                "correct_vignetting".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let _ = parse_principal_point_from_vector(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let input = FlipImageTool::load_raster(&input_path)?;
        let (pp_x, pp_y) = parse_principal_point_from_vector(args)?;
        let focal_length = args
            .get("focal_length")
            .and_then(|v| v.as_f64())
            .unwrap_or(304.8);
        let image_width = args
            .get("image_width")
            .and_then(|v| v.as_f64())
            .unwrap_or(228.6);
        let n_param = args.get("n").and_then(|v| v.as_f64()).unwrap_or(4.0);
        let output_path = parse_optional_output_path(args, "output")?;

        let out = run_correct_vignetting(&input, pp_x, pp_y, focal_length, image_width, n_param)?;
        ctx.progress.progress(1.0);
        let out_locator = FlipImageTool::write_or_store_output(out, output_path)?;

        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(out_locator));
        outputs.insert("active_band".to_string(), json!(0));
        Ok(ToolRunResult { outputs })
    }
}

// ── ImageStackProfileTool ───────────────────────────────────────────────────

impl Tool for ImageStackProfileTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "image_stack_profile",
            display_name: "Image Stack Profile",
            summary: "Extracts per-point profiles across an ordered raster stack and optionally writes an HTML report.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "inputs",
                    description: "Input raster list as an array of paths or comma/semicolon-delimited string.",
                    required: true,
                },
                ToolParamSpec {
                    name: "points",
                    description: "Point vector path (or typed vector object) containing sample locations.",
                    required: true,
                },
                ToolParamSpec {
                    name: "output_html",
                    description: "Optional HTML report output path.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["image1.tif", "image2.tif", "image3.tif"]));
        defaults.insert("points".to_string(), json!("sample_points.geojson"));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["image1.tif", "image2.tif", "image3.tif"]));
        example.insert("points".to_string(), json!("sample_points.geojson"));
        example.insert("output_html".to_string(), json!("stack_profile.html"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_image_stack_profile".to_string(),
                description: "Extract profile signatures for points from a raster stack.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "raster".to_string(),
                "image_stack_profile".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.len() < 2 {
            return Err(ToolError::Validation(
                "parameter 'inputs' must contain at least two rasters".to_string(),
            ));
        }
        let points = parse_vector_points_arg(args, "points")?;
        if points.is_empty() {
            return Err(ToolError::Validation(
                "parameter 'points' must contain at least one point".to_string(),
            ));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = parse_raster_list_arg(args, "inputs")?;
        let mut inputs = Vec::with_capacity(input_paths.len());
        for p in &input_paths {
            inputs.push(FlipImageTool::load_raster(p)?);
        }
        let point_coords = parse_vector_points_arg(args, "points")?;
        let points = map_points_to_rows_cols(&inputs[0], &point_coords);

        let profiles = run_image_stack_profile(&inputs, &points)?;

        if let Some(html_path) = args.get("output_html").and_then(|v| v.as_str()) {
            write_image_stack_profile_html(html_path, &input_paths, &profiles)?;
        }
        ctx.progress.progress(1.0);

        let mut outputs = BTreeMap::new();
        outputs.insert("profiles".to_string(), json!(profiles));
        outputs.insert("num_points".to_string(), json!(points.len()));
        outputs.insert("num_images".to_string(), json!(input_paths.len()));
        if let Some(html_path) = args.get("output_html").and_then(|v| v.as_str()) {
            outputs.insert("report_path".to_string(), json!(html_path));
        }
        Ok(ToolRunResult { outputs })
    }
}

// ── PiecewiseContrastStretchTool ───────────────────────────────────────────

impl Tool for PiecewiseContrastStretchTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "piecewise_contrast_stretch",
            display_name: "Piecewise Contrast Stretch",
            summary: "Performs piecewise linear contrast stretching using user-specified breakpoints.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input raster path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "function",
                    description: "Breakpoint statement string, e.g. '(50,0.1);(120,0.6);(180,0.85)'.",
                    required: true,
                },
                ToolParamSpec {
                    name: "greytones",
                    description: "Number of output tones for non-RGB rasters (default 1024).",
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

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("function".to_string(), json!("(50,0.1);(120,0.6);(180,0.85)"));
        defaults.insert("greytones".to_string(), json!(1024));

        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("input.tif"));
        example.insert("function".to_string(), json!("(80,0.2);(140,0.7);(200,0.92)"));
        example.insert("greytones".to_string(), json!(512));
        example.insert("output".to_string(), json!("piecewise_contrast.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_piecewise_contrast_stretch".to_string(),
                description: "Apply a piecewise transfer function to image brightness values.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "raster".to_string(),
                "piecewise_contrast_stretch".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let statement = args
            .get("function")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'function' is required".to_string()))?;
        let _ = parse_piecewise_statement(statement)?;
        if let Some(g) = args.get("greytones").and_then(|v| v.as_u64()) {
            if g < 32 {
                return Err(ToolError::Validation(
                    "parameter 'greytones' must be >= 32".to_string(),
                ));
            }
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let input = FlipImageTool::load_raster(&input_path)?;
        let statement = args
            .get("function")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'function' is required".to_string()))?;
        let greytones = args
            .get("greytones")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1024)
            .max(32);
        let output_path = parse_optional_output_path(args, "output")?;

        let output = run_piecewise_contrast_stretch(&input, statement, greytones)?;
        let output_locator = FlipImageTool::write_or_store_output(output, output_path)?;

        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("active_band".to_string(), json!(0));
        Ok(ToolRunResult { outputs })
    }
}

// ── GeneralizeClassifiedRasterTool ──────────────────────────────────────────

impl Tool for GeneralizeClassifiedRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "generalize_classified_raster",
            display_name: "Generalize Classified Raster",
            summary: "Generalizes small class patches by merging them into neighboring larger classes.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input classified raster path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "min_size",
                    description: "Minimum feature size in cells; smaller patches are reassigned (default 5).",
                    required: false,
                },
                ToolParamSpec {
                    name: "method",
                    description: "Generalization method: 'longest' (default), 'largest', or 'nearest'.",
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

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("classified.tif"));
        defaults.insert("min_size".to_string(), json!(5));
        defaults.insert("method".to_string(), json!("longest"));

        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("classified.tif"));
        example.insert("min_size".to_string(), json!(15));
        example.insert("method".to_string(), json!("largest"));
        example.insert("output".to_string(), json!("generalized.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_generalize_classified_raster".to_string(),
                description: "Merge undersized class patches into neighboring classes.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "raster".to_string(),
                "generalize_classified_raster".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        if let Some(min_size) = args.get("min_size").and_then(|v| v.as_u64()) {
            if min_size == 0 {
                return Err(ToolError::Validation(
                    "parameter 'min_size' must be greater than 0".to_string(),
                ));
            }
        }
        let _ = parse_generalize_method(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let input = FlipImageTool::load_raster(&input_path)?;
        let min_size = args
            .get("min_size")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(5)
            .max(1);
        let method = parse_generalize_method(args)?;
        let output_path = parse_optional_output_path(args, "output")?;

        let output = run_generalize_classified_raster(&input, min_size, method)?;
        let output_locator = FlipImageTool::write_or_store_output(output, output_path)?;

        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("active_band".to_string(), json!(0));
        Ok(ToolRunResult { outputs })
    }
}

// ── ImageSliderTool ─────────────────────────────────────────────────────────

impl Tool for ImageSliderTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "image_slider",
            display_name: "Image Slider",
            summary: "Creates an interactive HTML image slider from two raster images.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input1",
                    description: "Left image raster path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "input2",
                    description: "Right image raster path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "label1",
                    description: "Optional label shown for left image.",
                    required: false,
                },
                ToolParamSpec {
                    name: "label2",
                    description: "Optional label shown for right image.",
                    required: false,
                },
                ToolParamSpec {
                    name: "height",
                    description: "Display height in pixels (default 600).",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output HTML path.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("input1".to_string(), json!("left.tif"));
        defaults.insert("input2".to_string(), json!("right.tif"));
        defaults.insert("height".to_string(), json!(600));

        let mut example = ToolArgs::new();
        example.insert("input1".to_string(), json!("left.tif"));
        example.insert("input2".to_string(), json!("right.tif"));
        example.insert("label1".to_string(), json!("Before"));
        example.insert("label2".to_string(), json!("After"));
        example.insert("height".to_string(), json!(600));
        example.insert("output".to_string(), json!("image_slider.html"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_image_slider".to_string(),
                description: "Create an interactive two-image swipe slider.".to_string(),
                args: example,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "visualization".to_string(),
                "image_slider".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input1")?;
        let _ = parse_raster_path_arg(args, "input2")?;
        if let Some(h) = args.get("height").and_then(|v| v.as_u64()) {
            if h < 50 {
                return Err(ToolError::Validation(
                    "parameter 'height' must be >= 50".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input1_path = parse_raster_path_arg(args, "input1")?;
        let input2_path = parse_raster_path_arg(args, "input2")?;
        let input1 = FlipImageTool::load_raster(&input1_path)?;
        let input2 = FlipImageTool::load_raster(&input2_path)?;
        let label1 = args
            .get("label1")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let label2 = args
            .get("label2")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let height = args
            .get("height")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(600)
            .max(50);

        let output_path = if let Some(p) = parse_optional_output_path(args, "output")? {
            p
        } else {
            std::env::current_dir()
                .map_err(|e| ToolError::Execution(format!("failed reading current directory: {e}")))?
                .join("image_slider.html")
        };

        let html_path = run_image_slider_html(
            &input1,
            &input2,
            &output_path,
            &label1,
            &label2,
            height,
        )?;

        let mut outputs = BTreeMap::new();
        outputs.insert("path".to_string(), json!(html_path));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for CreateColourCompositeTool {
    fn metadata(&self) -> ToolMetadata {
        FlipImageTool::metadata_for(NonFilterOp::CreateColourComposite)
    }

    fn manifest(&self) -> ToolManifest {
        FlipImageTool::manifest_for(NonFilterOp::CreateColourComposite)
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = FlipImageTool::parse_create_colour_inputs(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        FlipImageTool::run_with_op(NonFilterOp::CreateColourComposite, args, ctx)
    }
}

// ── Compute helpers for SplitColourCompositeTool / RgbToIhsTool / IhsToRgbTool

fn new_f32_band_like(template: &Raster) -> Raster {
    Raster::new(RasterConfig {
        cols: template.cols,
        rows: template.rows,
        bands: 1,
        x_min: template.x_min,
        y_min: template.y_min,
        cell_size: template.cell_size_x,
        cell_size_y: Some(template.cell_size_y),
        nodata: -32768.0,
        data_type: DataType::F32,
        crs: template.crs.clone(),
        metadata: template.metadata.clone(),
    })
}

fn band_min_max(raster: &Raster) -> (f64, f64) {
    let rows = raster.rows as isize;
    let cols = raster.cols as isize;
    let mut min = f64::MAX;
    let mut max = f64::MIN;
    for r in 0..rows {
        for c in 0..cols {
            let v = raster.get(0, r, c);
            if !raster.is_nodata(v) {
                if v < min { min = v; }
                if v > max { max = v; }
            }
        }
    }
    if min > max { (0.0, 1.0) } else { (min, max) }
}

#[inline]
fn norm01(v: f64, min: f64, max: f64) -> f64 {
    let range = max - min;
    if range.abs() < 1e-12 { 0.5 } else { (v - min) / range }
}

fn run_split_colour_composite(input: &Raster) -> Result<(Raster, Raster, Raster), ToolError> {
    let rows = input.rows as isize;
    let cols = input.cols as isize;
    let out_nd = -32768.0f64;
    let mut red = new_f32_band_like(input);
    let mut green = new_f32_band_like(input);
    let mut blue = new_f32_band_like(input);
    for r in 0..rows {
        for c in 0..cols {
            let val = input.get(0, r, c);
            if !input.is_nodata(val) {
                let iv = val as i64;
                let _ = red.set(0, r, c, (iv & 0xFF) as f64);
                let _ = green.set(0, r, c, ((iv >> 8) & 0xFF) as f64);
                let _ = blue.set(0, r, c, ((iv >> 16) & 0xFF) as f64);
            } else {
                let _ = red.set(0, r, c, out_nd);
                let _ = green.set(0, r, c, out_nd);
                let _ = blue.set(0, r, c, out_nd);
            }
        }
    }
    Ok((red, green, blue))
}

fn run_rgb_to_ihs_from_composite(composite: &Raster) -> Result<(Raster, Raster, Raster), ToolError> {
    let rows = composite.rows as isize;
    let cols = composite.cols as isize;
    let out_nd = -32768.0f64;
    let mut intensity = new_f32_band_like(composite);
    let mut hue = new_f32_band_like(composite);
    let mut saturation = new_f32_band_like(composite);
    for r in 0..rows {
        for c in 0..cols {
            let val = composite.get(0, r, c);
            if !composite.is_nodata(val) {
                let iv = val as i64;
                let rn = (iv & 0xFF) as f64 / 255.0;
                let gn = ((iv >> 8) & 0xFF) as f64 / 255.0;
                let bn = ((iv >> 16) & 0xFF) as f64 / 255.0;
                let (h, s, i) = rgb_to_hsi_norm(rn, gn, bn);
                let _ = intensity.set(0, r, c, i);
                let _ = hue.set(0, r, c, h);
                let _ = saturation.set(0, r, c, s);
            } else {
                let _ = intensity.set(0, r, c, out_nd);
                let _ = hue.set(0, r, c, out_nd);
                let _ = saturation.set(0, r, c, out_nd);
            }
        }
    }
    Ok((intensity, hue, saturation))
}

fn run_rgb_to_ihs_from_bands(
    red: &Raster,
    green: &Raster,
    blue: &Raster,
) -> Result<(Raster, Raster, Raster), ToolError> {
    let rows = red.rows as isize;
    let cols = red.cols as isize;
    let out_nd = -32768.0f64;
    let (r_min, r_max) = band_min_max(red);
    let (g_min, g_max) = band_min_max(green);
    let (b_min, b_max) = band_min_max(blue);
    let mut intensity = new_f32_band_like(red);
    let mut hue = new_f32_band_like(red);
    let mut saturation = new_f32_band_like(red);
    for r in 0..rows {
        for c in 0..cols {
            let rv = red.get(0, r, c);
            let gv = green.get(0, r, c);
            let bv = blue.get(0, r, c);
            if !red.is_nodata(rv) && !green.is_nodata(gv) && !blue.is_nodata(bv) {
                let (h, s, i) = rgb_to_hsi_norm(
                    norm01(rv, r_min, r_max),
                    norm01(gv, g_min, g_max),
                    norm01(bv, b_min, b_max),
                );
                let _ = intensity.set(0, r, c, i);
                let _ = hue.set(0, r, c, h);
                let _ = saturation.set(0, r, c, s);
            } else {
                let _ = intensity.set(0, r, c, out_nd);
                let _ = hue.set(0, r, c, out_nd);
                let _ = saturation.set(0, r, c, out_nd);
            }
        }
    }
    Ok((intensity, hue, saturation))
}

fn run_ihs_to_rgb(
    intensity: &Raster,
    hue: &Raster,
    saturation: &Raster,
) -> Result<(Raster, Raster, Raster), ToolError> {
    let rows = intensity.rows as isize;
    let cols = intensity.cols as isize;
    let out_nd = -32768.0f64;
    let mut red = new_f32_band_like(intensity);
    let mut green = new_f32_band_like(intensity);
    let mut blue = new_f32_band_like(intensity);
    for r in 0..rows {
        for c in 0..cols {
            let i_v = intensity.get(0, r, c);
            let h_v = hue.get(0, r, c);
            let s_v = saturation.get(0, r, c);
            if !intensity.is_nodata(i_v) && !hue.is_nodata(h_v) && !saturation.is_nodata(s_v) {
                let (rv, gv, bv) = hsi_to_rgb_norm(h_v, s_v, i_v);
                let _ = red.set(0, r, c, (rv * 255.0).round().clamp(0.0, 255.0));
                let _ = green.set(0, r, c, (gv * 255.0).round().clamp(0.0, 255.0));
                let _ = blue.set(0, r, c, (bv * 255.0).round().clamp(0.0, 255.0));
            } else {
                let _ = red.set(0, r, c, out_nd);
                let _ = green.set(0, r, c, out_nd);
                let _ = blue.set(0, r, c, out_nd);
            }
        }
    }
    Ok((red, green, blue))
}

#[derive(Clone, Copy)]
enum GeneralizeMethod {
    Longest,
    Largest,
    Nearest,
}

fn parse_generalize_method(args: &ToolArgs) -> Result<GeneralizeMethod, ToolError> {
    let raw = args
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("longest")
        .to_ascii_lowercase();
    if raw.contains("near") {
        Ok(GeneralizeMethod::Nearest)
    } else if raw.contains("larg") {
        Ok(GeneralizeMethod::Largest)
    } else if raw.contains("long") {
        Ok(GeneralizeMethod::Longest)
    } else {
        Err(ToolError::Validation(
            "parameter 'method' must be one of: longest, largest, nearest".to_string(),
        ))
    }
}

fn parse_piecewise_statement(statement: &str) -> Result<Vec<(f64, f64)>, ToolError> {
    let mut out = Vec::new();
    for token in statement.split(';') {
        let t = token.trim();
        if t.is_empty() {
            continue;
        }
        let clean = t.replace('(', "").replace(')', "").replace(' ', "");
        let parts: Vec<&str> = clean.split(',').filter(|s| !s.is_empty()).collect();
        if parts.len() != 2 {
            return Err(ToolError::Validation(
                "parameter 'function' contains malformed breakpoint; expected '(x,y);...'".to_string(),
            ));
        }
        let x = parts[0].parse::<f64>().map_err(|_| {
            ToolError::Validation(format!(
                "parameter 'function' contains invalid x-value '{}'",
                parts[0]
            ))
        })?;
        let y = parts[1].parse::<f64>().map_err(|_| {
            ToolError::Validation(format!(
                "parameter 'function' contains invalid y-value '{}'",
                parts[1]
            ))
        })?;
        out.push((x, y));
    }
    if out.is_empty() {
        return Err(ToolError::Validation(
            "parameter 'function' must contain at least one breakpoint".to_string(),
        ));
    }
    out.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    Ok(out)
}

fn map_piecewise_value(v: f64, breakpoints: &[(f64, f64)]) -> f64 {
    if breakpoints.is_empty() {
        return v;
    }
    if v <= breakpoints[0].0 {
        return breakpoints[0].1;
    }
    for i in 1..breakpoints.len() {
        let (x0, y0) = breakpoints[i - 1];
        let (x1, y1) = breakpoints[i];
        if v <= x1 {
            let dx = (x1 - x0).abs();
            if dx < 1e-12 {
                return y1;
            }
            let t = (v - x0) / (x1 - x0);
            return y0 + t * (y1 - y0);
        }
    }
    breakpoints[breakpoints.len() - 1].1
}

fn run_piecewise_contrast_stretch(
    input: &Raster,
    statement: &str,
    num_greytones: usize,
) -> Result<Raster, ToolError> {
    let is_rgb = color_support::detect_rgb_mode(input, false, true) == color_support::RgbMode::Packed;

    let mut img_min = f64::INFINITY;
    let mut img_max = f64::NEG_INFINITY;
    for r in 0..input.rows as isize {
        for c in 0..input.cols as isize {
            let z = input.get(0, r, c);
            if input.is_nodata(z) {
                continue;
            }
            let v = if is_rgb {
                let (rv, gv, bv, _) = FlipImageTool::unpack_rgba(z);
                let (_, _, i) = rgb_to_hsi_norm(rv as f64 / 255.0, gv as f64 / 255.0, bv as f64 / 255.0);
                i
            } else {
                z
            };
            img_min = img_min.min(v);
            img_max = img_max.max(v);
        }
    }
    if !img_min.is_finite() || !img_max.is_finite() {
        return Err(ToolError::Validation(
            "input raster does not contain valid cells".to_string(),
        ));
    }

    let mut breakpoints = vec![(img_min, 0.0)];
    breakpoints.extend(parse_piecewise_statement(statement)?);
    breakpoints.push((img_max, 1.0));
    breakpoints.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    if !is_rgb {
        let tone_max = (num_greytones.max(32) - 1) as f64;
        for p in &mut breakpoints {
            p.1 = p.1.clamp(0.0, 1.0) * tone_max;
        }
    } else {
        for p in &mut breakpoints {
            p.1 = p.1.clamp(0.0, 1.0);
        }
    }

    let mut output = input.clone();
    if !is_rgb {
        output.data_type = DataType::F32;
    } else {
        output.data_type = DataType::U32;
        output
            .metadata
            .push(("color_interpretation".to_string(), "packed_rgb".to_string()));
    }

    for r in 0..input.rows as isize {
        for c in 0..input.cols as isize {
            let z = input.get(0, r, c);
            if input.is_nodata(z) {
                continue;
            }

            if is_rgb {
                let (rv, gv, bv, av) = FlipImageTool::unpack_rgba(z);
                let rn = rv as f64 / 255.0;
                let gn = gv as f64 / 255.0;
                let bn = bv as f64 / 255.0;
                let (h, s, i) = rgb_to_hsi_norm(rn, gn, bn);
                let out_i = map_piecewise_value(i, &breakpoints).clamp(0.0, 1.0);
                let (r2, g2, b2) = hsi_to_rgb_norm(h, s, out_i);
                let packed = FlipImageTool::pack_rgba(
                    (r2 * 255.0).round().clamp(0.0, 255.0) as u32,
                    (g2 * 255.0).round().clamp(0.0, 255.0) as u32,
                    (b2 * 255.0).round().clamp(0.0, 255.0) as u32,
                    av,
                );
                output.set(0, r, c, packed).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed writing piecewise RGB value at ({r},{c}): {e}"
                    ))
                })?;
            } else {
                let out_v = map_piecewise_value(z, &breakpoints);
                output.set(0, r, c, out_v).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed writing piecewise value at ({r},{c}): {e}"
                    ))
                })?;
            }
        }
    }

    Ok(output)
}

fn run_generalize_classified_raster(
    input: &Raster,
    min_size: usize,
    method: GeneralizeMethod,
) -> Result<Raster, ToolError> {
    let rows = input.rows as isize;
    let cols = input.cols as isize;
    let n = input.rows * input.cols;
    let nodata = input.nodata;

    let mut comp_id = vec![-1isize; n];
    let mut comp_class = Vec::<f64>::new();
    let mut comp_size = Vec::<usize>::new();
    let mut comp_cells = Vec::<Vec<usize>>::new();

    let n8 = [
        (-1isize, -1isize),
        (-1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
        (1, 0),
        (1, -1),
        (0, -1),
    ];

    let mut queue = VecDeque::<usize>::new();
    for r in 0..rows {
        for c in 0..cols {
            let idx = r as usize * input.cols + c as usize;
            if comp_id[idx] >= 0 {
                continue;
            }
            let z = input.get(0, r, c);
            if input.is_nodata(z) {
                continue;
            }

            let cid = comp_class.len() as isize;
            comp_class.push(z);
            comp_size.push(0);
            comp_cells.push(Vec::new());

            comp_id[idx] = cid;
            queue.push_back(idx);
            while let Some(cur) = queue.pop_front() {
                comp_cells[cid as usize].push(cur);
                comp_size[cid as usize] += 1;
                let cr = (cur / input.cols) as isize;
                let cc = (cur % input.cols) as isize;
                for (dr, dc) in n8 {
                    let nr = cr + dr;
                    let nc = cc + dc;
                    if nr < 0 || nc < 0 || nr >= rows || nc >= cols {
                        continue;
                    }
                    let ni = nr as usize * input.cols + nc as usize;
                    if comp_id[ni] >= 0 {
                        continue;
                    }
                    let zn = input.get(0, nr, nc);
                    if !input.is_nodata(zn) && (zn - z).abs() < 1e-12 {
                        comp_id[ni] = cid;
                        queue.push_back(ni);
                    }
                }
            }
        }
    }

    let mut output = Raster::new(RasterConfig {
        rows: input.rows,
        cols: input.cols,
        bands: 1,
        x_min: input.x_min,
        y_min: input.y_min,
        cell_size: input.cell_size_x,
        cell_size_y: Some(input.cell_size_y),
        nodata,
        data_type: DataType::I32,
        crs: input.crs.clone(),
        metadata: input.metadata.clone(),
    });

    for i in 0..n {
        let cid = comp_id[i];
        let r = (i / input.cols) as isize;
        let c = (i % input.cols) as isize;
        if cid < 0 {
            output.set(0, r, c, nodata).map_err(|e| {
                ToolError::Execution(format!("failed writing nodata output value at ({r},{c}): {e}"))
            })?;
        } else {
            output.set(0, r, c, comp_class[cid as usize]).map_err(|e| {
                ToolError::Execution(format!("failed writing output value at ({r},{c}): {e}"))
            })?;
        }
    }

    let n4 = [(0isize, 1isize), (1, 0), (0, -1), (-1, 0)];

    if matches!(method, GeneralizeMethod::Nearest) {
        let mut dist = vec![i32::MAX; n];
        let mut owner = vec![-1isize; n];
        let mut q = VecDeque::<usize>::new();

        for cid in 0..comp_class.len() {
            if comp_size[cid] < min_size {
                continue;
            }
            for &idx in &comp_cells[cid] {
                dist[idx] = 0;
                owner[idx] = cid as isize;
                q.push_back(idx);
            }
        }

        while let Some(cur) = q.pop_front() {
            let cr = (cur / input.cols) as isize;
            let cc = (cur % input.cols) as isize;
            let cur_dist = dist[cur];
            let cur_owner = owner[cur];
            for (dr, dc) in n4 {
                let nr = cr + dr;
                let nc = cc + dc;
                if nr < 0 || nc < 0 || nr >= rows || nc >= cols {
                    continue;
                }
                let ni = nr as usize * input.cols + nc as usize;
                let n_cid = comp_id[ni];
                if n_cid < 0 || comp_size[n_cid as usize] >= min_size {
                    continue;
                }
                let nd = cur_dist + 1;
                if nd < dist[ni] {
                    dist[ni] = nd;
                    owner[ni] = cur_owner;
                    q.push_back(ni);
                } else if nd == dist[ni]
                    && owner[ni] >= 0
                    && cur_owner >= 0
                    && comp_size[cur_owner as usize] > comp_size[owner[ni] as usize]
                {
                    owner[ni] = cur_owner;
                }
            }
        }

        for i in 0..n {
            let cid = comp_id[i];
            if cid < 0 || comp_size[cid as usize] >= min_size {
                continue;
            }
            let own = owner[i];
            if own >= 0 {
                let r = (i / input.cols) as isize;
                let c = (i % input.cols) as isize;
                output.set(0, r, c, comp_class[own as usize]).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed writing nearest generalized value at ({r},{c}): {e}"
                    ))
                })?;
            }
        }

        return Ok(output);
    }

    let mut changed = true;
    while changed {
        changed = false;
        for cid in 0..comp_class.len() {
            if comp_size[cid] == 0 || comp_size[cid] >= min_size {
                continue;
            }

            let mut border_counts = HashMap::<usize, usize>::new();
            for &idx in &comp_cells[cid] {
                let r = (idx / input.cols) as isize;
                let c = (idx % input.cols) as isize;
                for (dr, dc) in n4 {
                    let nr = r + dr;
                    let nc = c + dc;
                    if nr < 0 || nc < 0 || nr >= rows || nc >= cols {
                        continue;
                    }
                    let ni = nr as usize * input.cols + nc as usize;
                    let nid = comp_id[ni];
                    if nid >= 0 && nid as usize != cid {
                        *border_counts.entry(nid as usize).or_insert(0) += 1;
                    }
                }
            }

            if border_counts.is_empty() {
                continue;
            }

            let mut chosen: Option<usize> = None;
            match method {
                GeneralizeMethod::Largest => {
                    let mut best_size = 0usize;
                    for nid in border_counts.keys() {
                        let sz = comp_size[*nid];
                        if sz > best_size {
                            best_size = sz;
                            chosen = Some(*nid);
                        }
                    }
                }
                GeneralizeMethod::Longest => {
                    let mut best_count = 0usize;
                    for (nid, border) in &border_counts {
                        if comp_size[*nid] + comp_size[cid] < min_size {
                            continue;
                        }
                        if *border > best_count {
                            best_count = *border;
                            chosen = Some(*nid);
                        }
                    }
                }
                GeneralizeMethod::Nearest => {}
            }

            if let Some(target) = chosen {
                let moved_cells = comp_cells[cid].clone();
                for idx in moved_cells {
                    comp_id[idx] = target as isize;
                    let r = (idx / input.cols) as isize;
                    let c = (idx % input.cols) as isize;
                    output.set(0, r, c, comp_class[target]).map_err(|e| {
                        ToolError::Execution(format!(
                            "failed writing generalized value at ({r},{c}): {e}"
                        ))
                    })?;
                    comp_cells[target].push(idx);
                }
                comp_size[target] += comp_size[cid];
                comp_size[cid] = 0;
                comp_cells[cid].clear();
                changed = true;
            }
        }
    }

    Ok(output)
}

fn raster_to_rgba_image(input: &Raster) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let rows = input.rows as isize;
    let cols = input.cols as isize;
    let mut imgbuf: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(cols as u32, rows as u32);

    let rgb_mode = color_support::detect_rgb_mode(input, false, true);
    let is_rgb = rgb_mode == color_support::RgbMode::Packed;

    let stats = input.statistics();
    let min_v = stats.min;
    let range_v = (stats.max - stats.min).max(1e-12);

    for r in 0..rows {
        for c in 0..cols {
            let z = input.get(0, r, c);
            let px = if input.is_nodata(z) {
                Rgba([0, 0, 0, 0])
            } else if is_rgb {
                let (rv, gv, bv, _) = FlipImageTool::unpack_rgba(z);
                Rgba([rv as u8, gv as u8, bv as u8, 255])
            } else {
                let p = ((z - min_v) / range_v).clamp(0.0, 1.0);
                let v = (p * 255.0).round().clamp(0.0, 255.0) as u8;
                Rgba([v, v, v, 255])
            };
            imgbuf.put_pixel(c as u32, r as u32, px);
        }
    }

    imgbuf
}

fn run_image_slider_html(
    input1: &Raster,
    input2: &Raster,
    html_output_path: &std::path::Path,
    label1: &str,
    label2: &str,
    height_px: usize,
) -> Result<String, ToolError> {
    if input1.rows != input2.rows || input1.cols != input2.cols {
        return Err(ToolError::Validation(
            "image_slider requires input rasters to have matching rows and columns".to_string(),
        ));
    }

    let parent = html_output_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")));
    std::fs::create_dir_all(&parent).map_err(|e| {
        ToolError::Execution(format!(
            "failed creating image_slider output directory '{}': {}",
            parent.display(),
            e
        ))
    })?;

    let stem = html_output_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image_slider");
    let left_png_name = format!("{}_left.png", stem);
    let right_png_name = format!("{}_right.png", stem);
    let left_png = parent.join(&left_png_name);
    let right_png = parent.join(&right_png_name);

    let left_img = raster_to_rgba_image(input1);
    let right_img = raster_to_rgba_image(input2);
    left_img.save(&left_png).map_err(|e| {
        ToolError::Execution(format!("failed writing slider left image '{}': {}", left_png.display(), e))
    })?;
    right_img.save(&right_png).map_err(|e| {
        ToolError::Execution(format!("failed writing slider right image '{}': {}", right_png.display(), e))
    })?;

    let width_px = ((height_px as f64) * (input1.cols as f64 / input1.rows as f64)) as usize;
    let html = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Image Slider</title>\n<style>\nbody {{ font-family: Helvetica, Arial, sans-serif; margin: 0; background: #f4f4f4; }}\n.container {{ width: {width}px; height: {height}px; position: relative; margin: 30px auto; border: 1px solid #222; overflow: hidden; background: #000; }}\n.layer {{ position: absolute; top: 0; left: 0; width: 100%; height: 100%; }}\n.layer img {{ width: 100%; height: 100%; display: block; object-fit: fill; }}\n#rightLayer {{ width: 50%; overflow: hidden; border-right: 2px solid #fff; }}\n.range {{ width: {width}px; margin: 10px auto 24px auto; display: block; }}\n.label {{ position: absolute; top: 8px; padding: 4px 8px; color: #fff; background: rgba(0,0,0,0.5); font-size: 14px; border-radius: 4px; }}\n.leftLabel {{ left: 8px; }}\n.rightLabel {{ right: 8px; }}\n</style></head><body>\n<div class=\"container\" id=\"slider\">\n  <div class=\"layer\"><img src=\"{left}\" alt=\"left image\"></div>\n  <div class=\"layer\" id=\"rightLayer\"><img src=\"{right}\" alt=\"right image\"></div>\n  <div class=\"label leftLabel\">{label_left}</div>\n  <div class=\"label rightLabel\">{label_right}</div>\n</div>\n<input class=\"range\" id=\"sliderInput\" type=\"range\" min=\"0\" max=\"100\" value=\"50\">\n<script>\nconst slider = document.getElementById('sliderInput');\nconst rightLayer = document.getElementById('rightLayer');\nslider.addEventListener('input', () => {{ rightLayer.style.width = slider.value + '%'; }});\n</script>\n</body></html>",
        width = width_px.max(100),
        height = height_px.max(50),
        left = html_escape(&left_png_name),
        right = html_escape(&right_png_name),
        label_left = html_escape(label1),
        label_right = html_escape(label2),
    );

    std::fs::write(html_output_path, html).map_err(|e| {
        ToolError::Execution(format!(
            "failed writing slider HTML '{}': {}",
            html_output_path.display(),
            e
        ))
    })?;

    Ok(html_output_path.to_string_lossy().to_string())
}

fn parse_raster_list_arg(args: &ToolArgs, key: &str) -> Result<Vec<String>, ToolError> {
    let value = args
        .get(key)
        .ok_or_else(|| ToolError::Validation(format!("parameter '{}' is required", key)))?;

    if let Some(s) = value.as_str() {
        let out: Vec<String> = s
            .split(|c| c == ',' || c == ';')
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .map(|p| p.to_string())
            .collect();
        if out.is_empty() {
            return Err(ToolError::Validation(format!(
                "parameter '{}' did not contain any raster paths",
                key
            )));
        }
        return Ok(out);
    }

    if let Some(arr) = value.as_array() {
        let mut out = Vec::with_capacity(arr.len());
        for (i, v) in arr.iter().enumerate() {
            let s = v.as_str().ok_or_else(|| {
                ToolError::Validation(format!(
                    "parameter '{}' array element {} must be a string path",
                    key, i
                ))
            })?;
            let s = s.trim();
            if s.is_empty() {
                return Err(ToolError::Validation(format!(
                    "parameter '{}' array element {} is empty",
                    key, i
                )));
            }
            out.push(s.to_string());
        }
        if out.is_empty() {
            return Err(ToolError::Validation(format!(
                "parameter '{}' did not contain any raster paths",
                key
            )));
        }
        return Ok(out);
    }

    Err(ToolError::Validation(format!(
        "parameter '{}' must be a string list (comma/semicolon-delimited) or an array of strings",
        key
    )))
}

fn parse_pan_sharpen_method(args: &ToolArgs) -> PanSharpenMethod {
    let raw = args
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("brovey")
        .to_ascii_lowercase();
    if raw.contains("ihs") {
        PanSharpenMethod::Ihs
    } else {
        PanSharpenMethod::Brovey
    }
}

fn parse_pan_sharpen_output_mode(args: &ToolArgs) -> PanSharpenOutputMode {
    let raw = args
        .get("output_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("packed")
        .to_ascii_lowercase();
    if raw.contains("band") {
        PanSharpenOutputMode::Bands
    } else {
        PanSharpenOutputMode::Packed
    }
}

fn parse_resample_method(args: &ToolArgs, key: &str, default: ResampleMethod) -> ResampleMethod {
    let Some(raw) = args.get(key).and_then(|v| v.as_str()) else {
        return default;
    };
    let lowered = raw.to_ascii_lowercase();
    if lowered.contains("nn") || lowered.contains("near") {
        ResampleMethod::Nearest
    } else if lowered.contains("bili") || lowered == "bl" || lowered == "bi" {
        ResampleMethod::Bilinear
    } else {
        ResampleMethod::Cubic
    }
}

fn validate_resample_inputs(inputs: &[Raster], op_name: &str) -> Result<(), ToolError> {
    if inputs.is_empty() {
        return Err(ToolError::Validation(format!(
            "{} requires at least one input raster",
            op_name
        )));
    }
    let rows = inputs[0].rows;
    let cols = inputs[0].cols;
    let bands = inputs[0].bands;
    for (idx, r) in inputs.iter().enumerate() {
        if r.bands != bands {
            return Err(ToolError::Validation(format!(
                "{} requires all inputs to have same band count; mismatch at input {}",
                op_name, idx
            )));
        }
        if r.rows == 0 || r.cols == 0 {
            return Err(ToolError::Validation(format!(
                "{} received an empty raster at input {}",
                op_name, idx
            )));
        }
    }
    if rows == 0 || cols == 0 {
        return Err(ToolError::Validation(format!(
            "{} received an empty first raster",
            op_name
        )));
    }
    Ok(())
}

fn sample_nearest(input: &Raster, band: isize, rowf: f64, colf: f64) -> Option<f64> {
    let row = rowf.floor() as isize;
    let col = colf.floor() as isize;
    if row < 0 || col < 0 || row >= input.rows as isize || col >= input.cols as isize {
        return None;
    }
    let v = input.get(band, row, col);
    if input.is_nodata(v) { None } else { Some(v) }
}

fn sample_bilinear(input: &Raster, band: isize, rowf: f64, colf: f64) -> Option<f64> {
    let r0 = rowf.floor() as isize;
    let c0 = colf.floor() as isize;
    let r1 = r0 + 1;
    let c1 = c0 + 1;

    let neighbours = [
        (r0, c0, (1.0 - (rowf - r0 as f64)) * (1.0 - (colf - c0 as f64))),
        (r0, c1, (1.0 - (rowf - r0 as f64)) * (colf - c0 as f64)),
        (r1, c0, (rowf - r0 as f64) * (1.0 - (colf - c0 as f64))),
        (r1, c1, (rowf - r0 as f64) * (colf - c0 as f64)),
    ];

    let mut sum_w = 0.0;
    let mut sum_v = 0.0;
    for (rr, cc, w) in neighbours {
        if rr < 0 || cc < 0 || rr >= input.rows as isize || cc >= input.cols as isize {
            continue;
        }
        let v = input.get(band, rr, cc);
        if input.is_nodata(v) || w <= 0.0 {
            continue;
        }
        sum_w += w;
        sum_v += w * v;
    }
    if sum_w <= 0.0 {
        None
    } else {
        Some(sum_v / sum_w)
    }
}

fn sample_cubic_like(input: &Raster, band: isize, rowf: f64, colf: f64) -> Option<f64> {
    let origin_row = rowf.floor() as isize;
    let origin_col = colf.floor() as isize;
    let shift_x = [-1, 0, 1, 2, -1, 0, 1, 2, -1, 0, 1, 2, -1, 0, 1, 2];
    let shift_y = [-1, -1, -1, -1, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2];

    let mut sum_w = 0.0;
    let mut sum_v = 0.0;
    for i in 0..16 {
        let rr = origin_row + shift_y[i];
        let cc = origin_col + shift_x[i];
        if rr < 0 || cc < 0 || rr >= input.rows as isize || cc >= input.cols as isize {
            continue;
        }
        let v = input.get(band, rr, cc);
        if input.is_nodata(v) {
            continue;
        }
        let dy = rr as f64 - rowf;
        let dx = cc as f64 - colf;
        let d2 = dx * dx + dy * dy;
        if d2 <= 1e-12 {
            return Some(v);
        }
        let w = 1.0 / d2;
        sum_w += w;
        sum_v += w * v;
    }

    if sum_w <= 0.0 {
        None
    } else {
        Some(sum_v / sum_w)
    }
}

fn sample_value(input: &Raster, band: isize, rowf: f64, colf: f64, method: ResampleMethod) -> Option<f64> {
    match method {
        ResampleMethod::Nearest => sample_nearest(input, band, rowf, colf),
        ResampleMethod::Bilinear => sample_bilinear(input, band, rowf, colf),
        ResampleMethod::Cubic => sample_cubic_like(input, band, rowf, colf),
    }
}

fn output_grid_from_extent(
    x_min: f64,
    y_min: f64,
    x_max: f64,
    y_max: f64,
    cell_size_x: f64,
    cell_size_y: f64,
) -> (usize, usize, f64, f64, f64, f64) {
    let rows = ((y_max - y_min).abs() / cell_size_y).ceil() as usize;
    let cols = ((x_max - x_min).abs() / cell_size_x).ceil() as usize;
    let y_min_adj = y_max - rows as f64 * cell_size_y;
    let x_max_adj = x_min + cols as f64 * cell_size_x;
    (rows, cols, x_min, y_min_adj, x_max_adj, y_max)
}

fn run_mosaic(inputs: &[Raster], method: ResampleMethod) -> Result<Raster, ToolError> {
    validate_resample_inputs(inputs, "mosaic")?;
    if inputs.len() < 2 {
        return Err(ToolError::Validation(
            "mosaic requires at least two input rasters".to_string(),
        ));
    }

    let mut x_min = f64::INFINITY;
    let mut y_min = f64::INFINITY;
    let mut x_max = f64::NEG_INFINITY;
    let mut y_max = f64::NEG_INFINITY;
    let mut cell_size_x = f64::INFINITY;
    let mut cell_size_y = f64::INFINITY;

    for r in inputs {
        x_min = x_min.min(r.x_min);
        y_min = y_min.min(r.y_min);
        x_max = x_max.max(r.x_max());
        y_max = y_max.max(r.y_max());
        cell_size_x = cell_size_x.min(r.cell_size_x.abs());
        cell_size_y = cell_size_y.min(r.cell_size_y.abs());
    }

    let (rows, cols, out_x_min, out_y_min, _out_x_max, out_y_max) =
        output_grid_from_extent(x_min, y_min, x_max, y_max, cell_size_x, cell_size_y);
    let first = &inputs[0];
    let mut output = Raster::new(RasterConfig {
        rows,
        cols,
        bands: first.bands,
        x_min: out_x_min,
        y_min: out_y_min,
        cell_size: cell_size_x,
        cell_size_y: Some(cell_size_y),
        nodata: -32768.0,
        data_type: if matches!(method, ResampleMethod::Nearest) {
            first.data_type
        } else {
            DataType::F32
        },
        crs: first.crs.clone(),
        metadata: first.metadata.clone(),
    });

    for band in 0..first.bands as isize {
        for row in 0..rows as isize {
            let y = out_y_max - (row as f64 + 0.5) * cell_size_y;
            for col in 0..cols as isize {
                let x = out_x_min + (col as f64 + 0.5) * cell_size_x;

                let mut chosen = None;
                for input in inputs.iter().rev() {
                    let rowf = (input.y_max() - y) / input.cell_size_y;
                    let colf = (x - input.x_min) / input.cell_size_x;
                    if let Some(v) = sample_value(input, band, rowf, colf, method) {
                        chosen = Some(v);
                        break;
                    }
                }

                if let Some(v) = chosen {
                    output.set(band, row, col, v).map_err(|e| {
                        ToolError::Execution(format!(
                            "failed writing mosaic value at band {band}, ({row},{col}): {e}"
                        ))
                    })?;
                }
            }
        }
    }

    Ok(output)
}

fn edge_distance_weight(rowf: f64, colf: f64, rows: usize, cols: usize) -> f64 {
    if rows == 0 || cols == 0 {
        return 1.0;
    }
    let r = rowf.floor().clamp(0.0, (rows as f64) - 1.0) as isize;
    let c = colf.floor().clamp(0.0, (cols as f64) - 1.0) as isize;
    let top = r;
    let left = c;
    let bottom = rows as isize - 1 - r;
    let right = cols as isize - 1 - c;
    (top.min(left).min(bottom).min(right).max(0) as f64) + 1.0
}

fn unpack_norm_rgb(packed: f64) -> (f64, f64, f64) {
    let (r, g, b, _) = FlipImageTool::unpack_rgba(packed);
    (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0)
}

fn sample_rgb_nearest(input: &Raster, rowf: f64, colf: f64) -> Option<(f64, f64, f64)> {
    let row = rowf.floor() as isize;
    let col = colf.floor() as isize;
    if row < 0 || col < 0 || row >= input.rows as isize || col >= input.cols as isize {
        return None;
    }
    let v = input.get(0, row, col);
    if input.is_nodata(v) {
        None
    } else {
        Some(unpack_norm_rgb(v))
    }
}

fn sample_rgb_bilinear(input: &Raster, rowf: f64, colf: f64) -> Option<(f64, f64, f64)> {
    let r0 = rowf.floor() as isize;
    let c0 = colf.floor() as isize;
    let r1 = r0 + 1;
    let c1 = c0 + 1;

    let neighbours = [
        (r0, c0, (1.0 - (rowf - r0 as f64)) * (1.0 - (colf - c0 as f64))),
        (r0, c1, (1.0 - (rowf - r0 as f64)) * (colf - c0 as f64)),
        (r1, c0, (rowf - r0 as f64) * (1.0 - (colf - c0 as f64))),
        (r1, c1, (rowf - r0 as f64) * (colf - c0 as f64)),
    ];

    let mut sum_w = 0.0;
    let mut sr = 0.0;
    let mut sg = 0.0;
    let mut sb = 0.0;
    for (rr, cc, w) in neighbours {
        if rr < 0 || cc < 0 || rr >= input.rows as isize || cc >= input.cols as isize || w <= 0.0 {
            continue;
        }
        let v = input.get(0, rr, cc);
        if input.is_nodata(v) {
            continue;
        }
        let (rn, gn, bn) = unpack_norm_rgb(v);
        sum_w += w;
        sr += rn * w;
        sg += gn * w;
        sb += bn * w;
    }
    if sum_w <= 0.0 {
        None
    } else {
        Some((sr / sum_w, sg / sum_w, sb / sum_w))
    }
}

fn sample_rgb_cubic_like(input: &Raster, rowf: f64, colf: f64) -> Option<(f64, f64, f64)> {
    let origin_row = rowf.floor() as isize;
    let origin_col = colf.floor() as isize;
    let shift_x = [-1, 0, 1, 2, -1, 0, 1, 2, -1, 0, 1, 2, -1, 0, 1, 2];
    let shift_y = [-1, -1, -1, -1, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2];

    let mut sum_w = 0.0;
    let mut sr = 0.0;
    let mut sg = 0.0;
    let mut sb = 0.0;

    for i in 0..16 {
        let rr = origin_row + shift_y[i];
        let cc = origin_col + shift_x[i];
        if rr < 0 || cc < 0 || rr >= input.rows as isize || cc >= input.cols as isize {
            continue;
        }
        let v = input.get(0, rr, cc);
        if input.is_nodata(v) {
            continue;
        }
        let dy = rr as f64 - rowf;
        let dx = cc as f64 - colf;
        let d2 = dx * dx + dy * dy;
        if d2 <= 1e-12 {
            return Some(unpack_norm_rgb(v));
        }
        let w = 1.0 / d2;
        let (rn, gn, bn) = unpack_norm_rgb(v);
        sum_w += w;
        sr += rn * w;
        sg += gn * w;
        sb += bn * w;
    }

    if sum_w <= 0.0 {
        None
    } else {
        Some((sr / sum_w, sg / sum_w, sb / sum_w))
    }
}

fn sample_rgb_value(input: &Raster, rowf: f64, colf: f64, method: ResampleMethod) -> Option<(f64, f64, f64)> {
    match method {
        ResampleMethod::Nearest => sample_rgb_nearest(input, rowf, colf),
        ResampleMethod::Bilinear => sample_rgb_bilinear(input, rowf, colf),
        ResampleMethod::Cubic => sample_rgb_cubic_like(input, rowf, colf),
    }
}

fn run_mosaic_with_feathering(
    input1: &Raster,
    input2: &Raster,
    method: ResampleMethod,
    distance_weight: f64,
) -> Result<Raster, ToolError> {
    if input1.bands != input2.bands {
        return Err(ToolError::Validation(
            "mosaic_with_feathering requires both inputs to have matching band counts".to_string(),
        ));
    }

    let mode1 = color_support::detect_rgb_mode(input1, false, true);
    let mode2 = color_support::detect_rgb_mode(input2, false, true);
    let packed_rgb = mode1 == color_support::RgbMode::Packed && mode2 == color_support::RgbMode::Packed;

    let x_min = input1.x_min.min(input2.x_min);
    let y_min = input1.y_min.min(input2.y_min);
    let x_max = input1.x_max().max(input2.x_max());
    let y_max = input1.y_max().max(input2.y_max());

    // Match legacy behaviour: output grid uses the coarser input resolution.
    let cell_size_x = input1.cell_size_x.abs().max(input2.cell_size_x.abs());
    let cell_size_y = input1.cell_size_y.abs().max(input2.cell_size_y.abs());
    let (rows, cols, out_x_min, out_y_min, _out_x_max, out_y_max) =
        output_grid_from_extent(x_min, y_min, x_max, y_max, cell_size_x, cell_size_y);

    let mut output = Raster::new(RasterConfig {
        rows,
        cols,
        bands: if packed_rgb { 1 } else { input1.bands },
        x_min: out_x_min,
        y_min: out_y_min,
        cell_size: cell_size_x,
        cell_size_y: Some(cell_size_y),
        nodata: input1.nodata,
        data_type: if packed_rgb { DataType::U32 } else { DataType::F32 },
        crs: input1.crs.clone(),
        metadata: input1.metadata.clone(),
    });
    if packed_rgb {
        output
            .metadata
            .push(("color_interpretation".to_string(), "packed_rgb".to_string()));
    }

    for row in 0..rows as isize {
        let y = out_y_max - (row as f64 + 0.5) * cell_size_y;
        for col in 0..cols as isize {
            let x = out_x_min + (col as f64 + 0.5) * cell_size_x;

            let rowf1 = (input1.y_max() - y) / input1.cell_size_y;
            let colf1 = (x - input1.x_min) / input1.cell_size_x;
            let rowf2 = (input2.y_max() - y) / input2.cell_size_y;
            let colf2 = (x - input2.x_min) / input2.cell_size_x;

            let d1 = edge_distance_weight(rowf1, colf1, input1.rows, input1.cols).powf(distance_weight);
            let d2 = edge_distance_weight(rowf2, colf2, input2.rows, input2.cols).powf(distance_weight);

            if packed_rgb {
                let c1 = sample_rgb_value(input1, rowf1, colf1, method);
                let c2 = sample_rgb_value(input2, rowf2, colf2, method);
                let out_rgb = match (c1, c2) {
                    (Some((r1, g1, b1)), Some((r2, g2, b2))) => {
                        let sw = d1 + d2;
                        if sw <= 0.0 {
                            Some((r1, g1, b1))
                        } else {
                            Some(((r1 * d1 + r2 * d2) / sw, (g1 * d1 + g2 * d2) / sw, (b1 * d1 + b2 * d2) / sw))
                        }
                    }
                    (Some(rgb), None) => Some(rgb),
                    (None, Some(rgb)) => Some(rgb),
                    (None, None) => None,
                };

                if let Some((rn, gn, bn)) = out_rgb {
                    let r_u32 = (rn * 255.0).round().clamp(0.0, 255.0) as u32;
                    let g_u32 = (gn * 255.0).round().clamp(0.0, 255.0) as u32;
                    let b_u32 = (bn * 255.0).round().clamp(0.0, 255.0) as u32;
                    let packed = FlipImageTool::pack_rgba(r_u32, g_u32, b_u32, 255);
                    output.set(0, row, col, packed).map_err(|e| {
                        ToolError::Execution(format!(
                            "failed writing feathered mosaic packed pixel at ({row},{col}): {e}"
                        ))
                    })?;
                }
                continue;
            }

            for band in 0..input1.bands as isize {
                let z1 = sample_value(input1, band, rowf1, colf1, method);
                let z2 = sample_value(input2, band, rowf2, colf2, method);
                let out_v = match (z1, z2) {
                    (Some(v1), Some(v2)) => {
                        let sw = d1 + d2;
                        if sw <= 0.0 {
                            Some(v1)
                        } else {
                            Some((v1 * d1 + v2 * d2) / sw)
                        }
                    }
                    (Some(v), None) => Some(v),
                    (None, Some(v)) => Some(v),
                    (None, None) => None,
                };
                if let Some(v) = out_v {
                    output.set(band, row, col, v).map_err(|e| {
                        ToolError::Execution(format!(
                            "failed writing feathered mosaic value at band {band}, ({row},{col}): {e}"
                        ))
                    })?;
                }
            }
        }
    }

    Ok(output)
}

fn run_resample(
    inputs: &[Raster],
    base: Option<&Raster>,
    cell_size: Option<f64>,
    method: ResampleMethod,
) -> Result<Raster, ToolError> {
    validate_resample_inputs(inputs, "resample")?;

    let first = &inputs[0];
    let (rows, cols, out_x_min, out_y_min, out_cell_x, out_cell_y, out_y_max, out_crs, out_metadata) =
        if let Some(base_r) = base {
            (
                base_r.rows,
                base_r.cols,
                base_r.x_min,
                base_r.y_min,
                base_r.cell_size_x.abs(),
                base_r.cell_size_y.abs(),
                base_r.y_max(),
                base_r.crs.clone(),
                base_r.metadata.clone(),
            )
        } else {
            let cs = cell_size.unwrap_or(0.0);
            if cs <= 0.0 {
                return Err(ToolError::Validation(
                    "either 'base' or a positive 'cell_size' must be provided".to_string(),
                ));
            }
            let mut x_min = f64::INFINITY;
            let mut y_min = f64::INFINITY;
            let mut x_max = f64::NEG_INFINITY;
            let mut y_max = f64::NEG_INFINITY;
            for r in inputs {
                x_min = x_min.min(r.x_min);
                y_min = y_min.min(r.y_min);
                x_max = x_max.max(r.x_max());
                y_max = y_max.max(r.y_max());
            }
            let (rr, cc, xx_min, yy_min, _xx_max, yy_max) =
                output_grid_from_extent(x_min, y_min, x_max, y_max, cs, cs);
            (
                rr,
                cc,
                xx_min,
                yy_min,
                cs,
                cs,
                yy_max,
                first.crs.clone(),
                first.metadata.clone(),
            )
        };

    let mut output = Raster::new(RasterConfig {
        rows,
        cols,
        bands: first.bands,
        x_min: out_x_min,
        y_min: out_y_min,
        cell_size: out_cell_x,
        cell_size_y: Some(out_cell_y),
        nodata: first.nodata,
        data_type: if matches!(method, ResampleMethod::Nearest) {
            first.data_type
        } else {
            DataType::F32
        },
        crs: out_crs,
        metadata: out_metadata,
    });

    for band in 0..first.bands as isize {
        for row in 0..rows as isize {
            let y = out_y_max - (row as f64 + 0.5) * out_cell_y;
            for col in 0..cols as isize {
                let x = out_x_min + (col as f64 + 0.5) * out_cell_x;

                let mut chosen = None;
                for input in inputs.iter().rev() {
                    let rowf = (input.y_max() - y) / input.cell_size_y;
                    let colf = (x - input.x_min) / input.cell_size_x;
                    if let Some(v) = sample_value(input, band, rowf, colf, method) {
                        chosen = Some(v);
                        break;
                    }
                }

                if let Some(v) = chosen {
                    output.set(band, row, col, v).map_err(|e| {
                        ToolError::Execution(format!(
                            "failed writing resample value at band {band}, ({row},{col}): {e}"
                        ))
                    })?;
                }
            }
        }
    }

    Ok(output)
}

struct KMeansOptions {
    classes: usize,
    max_iterations: usize,
    class_change_threshold: f64,
    min_class_size: usize,
    initialize_random: bool,
    merge_distance: Option<f64>,
}

struct KMeansRunResult {
    raster: Raster,
    centroids: Vec<Vec<f64>>,
    counts: Vec<usize>,
    change_history: Vec<f64>,
}

fn sqr_euclidean(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let d = x - y;
            d * d
        })
        .sum()
}

fn deterministic_seed_index(k: usize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    // Deterministic LCG-style mixing to avoid adding RNG dependencies.
    let x = (k as u64)
        .wrapping_mul(1_664_525)
        .wrapping_add(1_013_904_223)
        .wrapping_mul(2_654_435_761);
    (x as usize) % len
}

fn merge_close_centroids(
    centroids: Vec<Vec<f64>>,
    counts: Vec<usize>,
    merge_dist: f64,
) -> (Vec<Vec<f64>>, Vec<usize>) {
    if centroids.len() <= 1 {
        return (centroids, counts);
    }
    let merge_dist2 = merge_dist * merge_dist;
    let mut removed = vec![false; centroids.len()];
    let mut c = centroids;
    let mut n = counts;

    for i in 0..c.len() {
        if removed[i] || n[i] == 0 {
            continue;
        }
        for j in (i + 1)..c.len() {
            if removed[j] || n[j] == 0 {
                continue;
            }
            if sqr_euclidean(&c[i], &c[j]) < merge_dist2 {
                let total = n[i] + n[j];
                if total == 0 {
                    removed[j] = true;
                    continue;
                }
                let w1 = n[i] as f64 / total as f64;
                let w2 = n[j] as f64 / total as f64;
                for d in 0..c[i].len() {
                    c[i][d] = c[i][d] * w1 + c[j][d] * w2;
                }
                n[i] = total;
                n[j] = 0;
                removed[j] = true;
            }
        }
    }

    let mut out_c = Vec::new();
    let mut out_n = Vec::new();
    for i in 0..c.len() {
        if !removed[i] && n[i] > 0 {
            out_c.push(c[i].clone());
            out_n.push(n[i]);
        }
    }
    if out_c.is_empty() {
        (vec![vec![0.0]], vec![0])
    } else {
        (out_c, out_n)
    }
}

fn run_kmeans(inputs: &[Raster], opts: KMeansOptions) -> Result<KMeansRunResult, ToolError> {
    validate_resample_inputs(inputs, "k_means_clustering")?;
    if inputs.len() < 2 {
        return Err(ToolError::Validation(
            "k-means clustering requires at least two input rasters".to_string(),
        ));
    }

    let rows = inputs[0].rows as isize;
    let cols = inputs[0].cols as isize;
    let dims = inputs.len();
    for (i, r) in inputs.iter().enumerate() {
        if r.rows != inputs[0].rows || r.cols != inputs[0].cols {
            return Err(ToolError::Validation(format!(
                "input raster dimensions mismatch at index {}",
                i
            )));
        }
    }

    let mut valid_indices = Vec::<usize>::new();
    let mut values = Vec::<Vec<f64>>::new();
    let mut mins = vec![f64::INFINITY; dims];
    let mut maxs = vec![f64::NEG_INFINITY; dims];

    for r in 0..rows {
        for c in 0..cols {
            let mut feat = vec![0.0; dims];
            let mut valid = true;
            for d in 0..dims {
                let z = inputs[d].get(0, r, c);
                if inputs[d].is_nodata(z) {
                    valid = false;
                    break;
                }
                feat[d] = z;
            }
            if valid {
                for d in 0..dims {
                    mins[d] = mins[d].min(feat[d]);
                    maxs[d] = maxs[d].max(feat[d]);
                }
                valid_indices.push((r as usize) * inputs[0].cols + (c as usize));
                values.push(feat);
            }
        }
    }

    if values.is_empty() {
        return Err(ToolError::Validation(
            "all cells are nodata across at least one input band".to_string(),
        ));
    }

    let mut k = opts.classes.max(2).min(values.len());
    let mut centroids = vec![vec![0.0; dims]; k];
    if opts.initialize_random {
        for (a, c) in centroids.iter_mut().enumerate().take(k) {
            let idx = deterministic_seed_index(a, values.len());
            *c = values[idx].clone();
        }
    } else {
        let denom = (k.saturating_sub(1)).max(1) as f64;
        for (a, c) in centroids.iter_mut().enumerate().take(k) {
            for d in 0..dims {
                let t = a as f64 / denom;
                *c.get_mut(d).unwrap() = mins[d] + t * (maxs[d] - mins[d]);
            }
        }
    }

    let mut labels = vec![0usize; values.len()];
    let mut prev_labels = vec![usize::MAX; values.len()];
    let mut counts = vec![0usize; k];
    let mut change_history = Vec::new();

    for _ in 0..opts.max_iterations {
        counts.fill(0);
        let mut sums = vec![vec![0.0; dims]; k];
        let mut changed = 0usize;

        for (i, feat) in values.iter().enumerate() {
            let mut best_idx = 0usize;
            let mut best_dist = f64::INFINITY;
            for (a, centre) in centroids.iter().enumerate() {
                let dist = sqr_euclidean(feat, centre);
                if dist < best_dist {
                    best_dist = dist;
                    best_idx = a;
                }
            }
            labels[i] = best_idx;
            if labels[i] != prev_labels[i] {
                changed += 1;
            }
            counts[best_idx] += 1;
            for d in 0..dims {
                sums[best_idx][d] += feat[d];
            }
        }

        let change_percent = (changed as f64 / values.len() as f64) * 100.0;
        change_history.push(change_percent);

        for a in 0..k {
            if counts[a] >= opts.min_class_size.max(1) {
                for d in 0..dims {
                    centroids[a][d] = sums[a][d] / counts[a] as f64;
                }
            }
        }

        if let Some(md) = opts.merge_distance {
            let (merged_centroids, merged_counts) = merge_close_centroids(centroids, counts, md);
            centroids = merged_centroids;
            counts = merged_counts;
            k = centroids.len().max(1);
        }

        prev_labels.clone_from(&labels);
        if change_percent < opts.class_change_threshold {
            break;
        }
    }

    // Final assignment against final centroids.
    counts = vec![0usize; centroids.len()];
    for (i, feat) in values.iter().enumerate() {
        let mut best_idx = 0usize;
        let mut best_dist = f64::INFINITY;
        for (a, centre) in centroids.iter().enumerate() {
            let dist = sqr_euclidean(feat, centre);
            if dist < best_dist {
                best_dist = dist;
                best_idx = a;
            }
        }
        labels[i] = best_idx;
        counts[best_idx] += 1;
    }

    let mut out = Raster::new(RasterConfig {
        rows: inputs[0].rows,
        cols: inputs[0].cols,
        bands: 1,
        x_min: inputs[0].x_min,
        y_min: inputs[0].y_min,
        cell_size: inputs[0].cell_size_x,
        cell_size_y: Some(inputs[0].cell_size_y),
        nodata: -32768.0,
        data_type: DataType::I16,
        crs: inputs[0].crs.clone(),
        metadata: inputs[0].metadata.clone(),
    });
    out.metadata
        .push(("color_interpretation".to_string(), "categorical".to_string()));

    for (i, pix) in valid_indices.iter().enumerate() {
        let row = (pix / inputs[0].cols) as isize;
        let col = (pix % inputs[0].cols) as isize;
        out.set(0, row, col, (labels[i] + 1) as f64).map_err(|e| {
            ToolError::Execution(format!(
                "failed writing k-means class value at ({row},{col}): {e}"
            ))
        })?;
    }

    Ok(KMeansRunResult {
        raster: out,
        centroids,
        counts,
        change_history,
    })
}

fn write_cluster_html_report(
    path: &str,
    title: &str,
    input_paths: &[String],
    result: &KMeansRunResult,
) -> Result<(), ToolError> {
    let title_esc = html_escape(title);
    let mut html = String::new();
    html.push_str("<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\"><html><head><meta content=\"text/html; charset=UTF-8\" http-equiv=\"content-type\"><title>");
    html.push_str(&title_esc);
    html.push_str("</title>");
    html.push_str(wbw_report_css());
    html.push_str("</head><body>");
    html.push_str("<h1>");
    html.push_str(&title_esc);
    html.push_str(" Report</h1>");
    html.push_str("<p><strong>Num. bands</strong>: ");
    html.push_str(&input_paths.len().to_string());
    for (i, p) in input_paths.iter().enumerate() {
        html.push_str("<br><strong>Image ");
        html.push_str(&(i + 1).to_string());
        html.push_str("</strong>: ");
        html.push_str(&html_escape(p));
    }
    html.push_str("<br><strong>Num. clusters</strong>: ");
    html.push_str(&result.centroids.len().to_string());
    html.push_str("</p>");

    html.push_str("<p><table><caption>Cluster Size</caption><tr><th>Cluster</th><th>Num. Pixels</th></tr>");
    for (i, n) in result.counts.iter().enumerate() {
        html.push_str(&format!(
            "<tr><td>{}</td><td class=\"numberCell\">{}</td></tr>",
            i + 1,
            n
        ));
    }
    html.push_str("</table></p>");

    html.push_str("<p><table><caption>Cluster Centroid Vector</caption><tr><th>Cluster</th>");
    if let Some(first) = result.centroids.first() {
        for d in 0..first.len() {
            html.push_str(&format!("<th>Band {}</th>", d + 1));
        }
    }
    html.push_str("</tr>");
    for (i, c) in result.centroids.iter().enumerate() {
        html.push_str(&format!("<tr><td>{}</td>", i + 1));
        for v in c {
            html.push_str(&format!("<td class=\"numberCell\">{:.6}</td>", v));
        }
        html.push_str("</tr>");
    }
    html.push_str("</table></p>");

    html.push_str("<p><table><caption>Iteration Change History</caption><tr><th>Iteration</th><th>Cells Changed (%)</th></tr>");
    for (i, pct) in result.change_history.iter().enumerate() {
        html.push_str(&format!(
            "<tr><td class=\"numberCell\">{}</td><td class=\"numberCell\">{:.4}</td></tr>",
            i + 1,
            pct
        ));
    }
    html.push_str("</table></p>");

    let xdata = vec![(1..=result.change_history.len()).map(|v| v as f64).collect::<Vec<f64>>()];
    let ydata = vec![result.change_history.clone()];
    html.push_str("<br><br><h2>Convergence Plot</h2>");
    html.push_str(&render_wbw_line_graph_svg(
        "graph",
        500.0,
        450.0,
        &xdata,
        &ydata,
        &["Line 1".to_string()],
        "Iteration",
        "Cells with class values changed (%)",
        true,
        true,
        false,
    ));

    html.push_str("</body></html>");

    std::fs::write(path, html).map_err(|e| {
        ToolError::Execution(format!(
            "failed writing cluster HTML report '{}': {}",
            path, e
        ))
    })
}

fn parse_principal_point_from_vector(args: &ToolArgs) -> Result<(f64, f64), ToolError> {
    let pp_path = parse_vector_path_arg(args, "pp")?;
    let layer = wbvector::read(&pp_path).map_err(|e| {
        ToolError::Validation(format!(
            "failed reading principal-point vector '{}': {}",
            pp_path, e
        ))
    })?;
    let points = extract_vector_points(&layer, "pp")?;
    points.first().copied().ok_or_else(|| {
        ToolError::Validation("parameter 'pp' vector must contain at least one point".to_string())
    })
}

fn run_correct_vignetting(
    input: &Raster,
    pp_x: f64,
    pp_y: f64,
    focal_length: f64,
    image_width: f64,
    n_param: f64,
) -> Result<Raster, ToolError> {
    if focal_length <= 0.0 || image_width <= 0.0 {
        return Err(ToolError::Validation(
            "parameters 'focal_length' and 'image_width' must be > 0".to_string(),
        ));
    }

    let rows = input.rows as isize;
    let cols = input.cols as isize;
    let scale_factor = image_width / cols.max(1) as f64;
    let rgb_mode = color_support::detect_rgb_mode(input, false, true);

    let mut unscaled = vec![input.nodata; input.rows * input.cols];
    let mut in_min = f64::INFINITY;
    let mut in_max = f64::NEG_INFINITY;
    let mut out_min = f64::INFINITY;
    let mut out_max = f64::NEG_INFINITY;

    for r in 0..rows {
        for c in 0..cols {
            let idx = (r as usize) * input.cols + c as usize;
            let mut i_in = input.get(0, r, c);
            if matches!(rgb_mode, color_support::RgbMode::Packed) {
                if input.is_nodata(i_in) {
                    continue;
                }
                let (rv, gv, bv, _) = FlipImageTool::unpack_rgba(i_in);
                let (_, _, i_norm) = rgb_to_hsi_norm(rv as f64 / 255.0, gv as f64 / 255.0, bv as f64 / 255.0);
                i_in = i_norm;
            } else if input.is_nodata(i_in) {
                continue;
            }

            let dr = r as f64 - pp_y;
            let dc = c as f64 - pp_x;
            let dist = (dr * dr + dc * dc).sqrt();
            let theta = (dist * scale_factor / focal_length).atan();
            let i_out = i_in / theta.cos().powf(n_param);
            unscaled[idx] = i_out;

            in_min = in_min.min(i_in);
            in_max = in_max.max(i_in);
            out_min = out_min.min(i_out);
            out_max = out_max.max(i_out);
        }
    }

    let in_range = (in_max - in_min).max(1e-12);
    let out_range = (out_max - out_min).max(1e-12);

    let mut output = input.clone();
    if matches!(rgb_mode, color_support::RgbMode::Packed) {
        output.data_type = DataType::U32;
        output
            .metadata
            .push(("color_interpretation".to_string(), "packed_rgb".to_string()));
    }

    for r in 0..rows {
        for c in 0..cols {
            let idx = (r as usize) * input.cols + c as usize;
            let iu = unscaled[idx];
            if input.is_nodata(iu) {
                output.set(0, r, c, output.nodata).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed writing vignetting nodata at ({r},{c}): {e}"
                    ))
                })?;
                continue;
            }

            let scaled_i = in_min + (iu - out_min) / out_range * in_range;
            if matches!(rgb_mode, color_support::RgbMode::Packed) {
                let raw = input.get(0, r, c);
                let (rv, gv, bv, _) = FlipImageTool::unpack_rgba(raw);
                let (h, s, _) = rgb_to_hsi_norm(rv as f64 / 255.0, gv as f64 / 255.0, bv as f64 / 255.0);
                let (rn, gn, bn) = hsi_to_rgb_norm(h, s, scaled_i.clamp(0.0, 1.0));
                let packed = FlipImageTool::pack_rgba(
                    (rn * 255.0).round().clamp(0.0, 255.0) as u32,
                    (gn * 255.0).round().clamp(0.0, 255.0) as u32,
                    (bn * 255.0).round().clamp(0.0, 255.0) as u32,
                    255,
                );
                output.set(0, r, c, packed).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed writing vignetting RGB value at ({r},{c}): {e}"
                    ))
                })?;
            } else {
                output.set(0, r, c, scaled_i).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed writing vignetting value at ({r},{c}): {e}"
                    ))
                })?;
            }
        }
    }

    Ok(output)
}

fn parse_vector_points_arg(args: &ToolArgs, param: &str) -> Result<Vec<(f64, f64)>, ToolError> {
    let points_path = parse_vector_path_arg(args, param)?;
    let layer = wbvector::read(&points_path).map_err(|e| {
        ToolError::Validation(format!(
            "failed reading vector '{}' from parameter '{}': {}",
            points_path, param, e
        ))
    })?;
    extract_vector_points(&layer, param)
}

fn extract_vector_points(layer: &wbvector::Layer, param: &str) -> Result<Vec<(f64, f64)>, ToolError> {
    let mut points = Vec::new();
    for feature in &layer.features {
        if let Some(geom) = &feature.geometry {
            collect_points_from_geometry(geom, &mut points)?;
        }
    }
    if points.is_empty() {
        return Err(ToolError::Validation(format!(
            "parameter '{}' vector must contain at least one point geometry",
            param
        )));
    }
    Ok(points)
}

fn collect_points_from_geometry(
    geometry: &VectorGeometry,
    points: &mut Vec<(f64, f64)>,
) -> Result<(), ToolError> {
    match geometry {
        VectorGeometry::Point(c) => {
            points.push((c.x, c.y));
            Ok(())
        }
        VectorGeometry::MultiPoint(coords) => {
            points.extend(coords.iter().map(|c| (c.x, c.y)));
            Ok(())
        }
        VectorGeometry::GeometryCollection(geoms) => {
            for g in geoms {
                collect_points_from_geometry(g, points)?;
            }
            Ok(())
        }
        _ => Err(ToolError::Validation(
            "vector input must contain only point or multipoint geometries".to_string(),
        )),
    }
}

fn map_points_to_rows_cols(raster: &Raster, points: &[(f64, f64)]) -> Vec<(isize, isize)> {
    points
        .iter()
        .map(|(x, y)| {
            raster
                .world_to_pixel(*x, *y)
                .map(|(col, row)| (row, col))
                .unwrap_or((-1, -1))
        })
        .collect()
}

fn run_image_stack_profile(
    inputs: &[Raster],
    points: &[(isize, isize)],
) -> Result<Vec<Vec<f64>>, ToolError> {
    if inputs.len() < 2 {
        return Err(ToolError::Validation(
            "image_stack_profile requires at least two input rasters".to_string(),
        ));
    }
    let mut profiles = vec![vec![0.0; inputs.len()]; points.len()];
    for (i, r) in inputs.iter().enumerate() {
        for (pidx, (row, col)) in points.iter().enumerate() {
            if *row < 0 || *col < 0 || *row >= r.rows as isize || *col >= r.cols as isize {
                profiles[pidx][i] = f64::NAN;
                continue;
            }
            let z = r.get(0, *row, *col);
            if r.is_nodata(z) {
                profiles[pidx][i] = f64::NAN;
            } else {
                profiles[pidx][i] = z;
            }
        }
    }
    Ok(profiles)
}

fn write_image_stack_profile_html(
    path: &str,
    input_paths: &[String],
    profiles: &[Vec<f64>],
) -> Result<(), ToolError> {
    let num_files = input_paths.len();
    let num_points = profiles.len();

    let mut xdata = vec![vec![0.0; num_files]; num_points];
    let mut series_names = Vec::with_capacity(num_points);
    for pidx in 0..num_points {
        series_names.push(format!("Point {}", pidx + 1));
        for i in 0..num_files {
            xdata[pidx][i] = (i + 1) as f64;
        }
    }

    let multiples = num_points > 2 && num_points < 12;

    let mut html = String::new();
    html.push_str("<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\"><html><head><meta content=\"text/html; charset=UTF-8\" http-equiv=\"content-type\"><title>Image Stack Profile</title>");
    html.push_str(wbw_report_css());
    html.push_str("</head><body>");
    html.push_str("<h1>Image Stack Profile</h1><p>");
    for (i, p) in input_paths.iter().enumerate() {
        html.push_str(&format!(
            "<strong>Image {}</strong>: {}<br>",
            i + 1,
            html_escape(p)
        ));
    }
    html.push_str("</p>");

    html.push_str(&render_wbw_line_graph_svg(
        "graph",
        700.0,
        500.0,
        &xdata,
        profiles,
        &series_names,
        "Image",
        "Value",
        false,
        true,
        multiples,
    ));

    html.push_str("<p><table><caption>Profile Data Table</caption><tr><th>Image</th>");
    for pidx in 0..num_points {
        html.push_str(&format!("<th>Point {}</th>", pidx + 1));
    }
    html.push_str("</tr>");
    for i in 0..num_files {
        html.push_str(&format!("<tr><td class=\"numberCell\">{}</td>", i + 1));
        for p in profiles {
            let v = p[i];
            if v.is_nan() {
                html.push_str("<td class=\"numberCell\">NaN</td>");
            } else {
                html.push_str(&format!("<td class=\"numberCell\">{}</td>", v));
            }
        }
        html.push_str("</tr>");
    }
    html.push_str("</table></p></body></html>");
    std::fs::write(path, html).map_err(|e| {
        ToolError::Execution(format!(
            "failed writing image stack profile HTML report '{}': {}",
            path, e
        ))
    })
}

fn wbw_report_css() -> &'static str {
    "<style type=\"text/css\">\
            h1 {\
                font-size: 14pt;\
                margin-left: 15px;\
                margin-right: 15px;\
                text-align: center;\
                font-family: Helvetica, Verdana, Geneva, Arial, sans-serif;\
            }\
            h2 {\
                font-size: 12pt;\
                margin-left: 15px;\
                margin-right: 15px;\
                text-align: center;\
                font-family: Helvetica, Verdana, Geneva, Arial, sans-serif;\
            }\
            p, ol, ul {\
                font-size: 12pt;\
                font-family: Helvetica, Verdana, Geneva, Arial, sans-serif;\
                margin-left: 15px;\
                margin-right: 15px;\
            }\
            caption {\
                font-family: Helvetica, Verdana, Geneva, Arial, sans-serif;\
                font-size: 12pt;\
                margin-left: 15px;\
                margin-right: 15px;\
            }\
            table {\
                font-size: 12pt;\
                font-family: Helvetica, Verdana, Geneva, Arial, sans-serif;\
                border-collapse: collapse;\
                align: center;\
            }\
            td, th {\
                border: 1px solid #222222;\
                text-align: center;\
                padding: 8px;\
            }\
            tr:nth-child(even) {\
                background-color: #dddddd;\
            }\
            tr:hover {\
                background-color: lightyellow;\
            }\
            .numberCell {\
                text-align: right;\
            }\
            .header {\
                font-weight: bold;\
                text-align: center;\
            }\
        </style>"
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn render_wbw_line_graph_svg(
    parent_id: &str,
    width: f64,
    height: f64,
    data_x: &[Vec<f64>],
    data_y: &[Vec<f64>],
    series_labels: &[String],
    x_axis_label: &str,
    y_axis_label: &str,
    draw_points: bool,
    draw_gridlines: bool,
    draw_legend: bool,
) -> String {
    if data_y.is_empty() {
        return format!("<div id='{}' align=\"center\"><p>No graph data.</p></div>", html_escape(parent_id));
    }

    let max_series_label_len = series_labels.iter().map(|s| s.len()).max().unwrap_or(0) as f64;
    let left_margin = 70.0;
    let right_margin = if draw_legend { 65.0 + max_series_label_len * 7.0 } else { 50.0 };
    let bottom_margin = 70.0;
    let top_margin = 40.0;
    let plot_w = (width - left_margin - right_margin).max(10.0);
    let plot_h = (height - top_margin - bottom_margin).max(10.0);

    let mut x_min = f64::INFINITY;
    let mut x_max = f64::NEG_INFINITY;
    let mut y_min = f64::INFINITY;
    let mut y_max = f64::NEG_INFINITY;

    for (sidx, ys) in data_y.iter().enumerate() {
        let xs = data_x.get(sidx);
        for (i, y) in ys.iter().enumerate() {
            if !y.is_finite() {
                continue;
            }
            let x = xs.and_then(|v| v.get(i)).copied().unwrap_or((i + 1) as f64);
            if !x.is_finite() {
                continue;
            }
            x_min = x_min.min(x);
            x_max = x_max.max(x);
            y_min = y_min.min(*y);
            y_max = y_max.max(*y);
        }
    }

    if !x_min.is_finite() || !y_min.is_finite() {
        return format!("<div id='{}' align=\"center\"><p>No finite graph data.</p></div>", html_escape(parent_id));
    }

    let x_pad = ((x_max - x_min).abs() * 0.05).max(1e-9);
    let y_pad = ((y_max - y_min).abs() * 0.05).max(1e-9);
    x_min -= x_pad;
    x_max += x_pad;
    y_min -= y_pad;
    y_max += y_pad;

    let x_range = (x_max - x_min).max(1e-12);
    let y_range = (y_max - y_min).max(1e-12);

    let to_px = |x: f64, y: f64| -> (f64, f64) {
        let px = left_margin + (x - x_min) / x_range * plot_w;
        let py = top_margin + plot_h - (y - y_min) / y_range * plot_h;
        (px, py)
    };

    let palette = [
        "rgb(31,119,180)",
        "rgb(255,127,14)",
        "rgb(44,160,44)",
        "rgb(214,39,40)",
        "rgb(148,103,189)",
        "rgb(140,86,75)",
        "rgb(227,119,194)",
        "rgb(127,127,127)",
        "rgb(188,189,34)",
        "rgb(23,190,207)",
    ];

    let mut svg = String::new();
    svg.push_str(&format!(
        "<div id='{}' align=\"center\"><svg width=\"{}\" height=\"{}\" xmlns=\"http://www.w3.org/2000/svg\">",
        html_escape(parent_id), width, height
    ));
    svg.push_str("<rect x=\"0\" y=\"0\" width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<rect x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\" fill=\"rgb(255,255,255)\" stroke=\"black\" stroke-width=\"0.5\"/>",
        left_margin, top_margin, plot_w, plot_h
    ));

    let tick_count = 10usize;
    for t in 0..=tick_count {
        let frac = t as f64 / tick_count as f64;
        let x = left_margin + frac * plot_w;
        let y = top_margin + frac * plot_h;
        if draw_gridlines && t > 0 && t < tick_count {
            svg.push_str(&format!(
                "<line x1=\"{:.3}\" y1=\"{:.3}\" x2=\"{:.3}\" y2=\"{:.3}\" stroke=\"#EEE\" stroke-width=\"0.8\"/>",
                x,
                top_margin,
                x,
                top_margin + plot_h
            ));
            svg.push_str(&format!(
                "<line x1=\"{:.3}\" y1=\"{:.3}\" x2=\"{:.3}\" y2=\"{:.3}\" stroke=\"#EEE\" stroke-width=\"0.8\"/>",
                left_margin,
                y,
                left_margin + plot_w,
                y
            ));
        }

        let x_val = x_min + frac * x_range;
        let y_val = y_max - frac * y_range;
        svg.push_str(&format!(
            "<line x1=\"{:.3}\" y1=\"{:.3}\" x2=\"{:.3}\" y2=\"{:.3}\" stroke=\"black\" stroke-width=\"0.5\"/>",
            x,
            top_margin + plot_h,
            x,
            top_margin + plot_h + 8.0
        ));
        svg.push_str(&format!(
            "<line x1=\"{:.3}\" y1=\"{:.3}\" x2=\"{:.3}\" y2=\"{:.3}\" stroke=\"black\" stroke-width=\"0.5\"/>",
            left_margin - 8.0,
            y,
            left_margin,
            y
        ));
        svg.push_str(&format!(
            "<text x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"middle\" dominant-baseline=\"hanging\" font-family=\"Sans,Arial\" font-size=\"85%\">{:.3}</text>",
            x,
            top_margin + plot_h + 12.0,
            x_val
        ));
        svg.push_str(&format!(
            "<text x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"end\" dominant-baseline=\"middle\" font-family=\"Sans,Arial\" font-size=\"85%\">{:.3}</text>",
            left_margin - 10.0,
            y,
            y_val
        ));
    }

    svg.push_str(&format!(
        "<text x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"middle\" font-family=\"Sans,Arial\" font-size=\"90%\">{}</text>",
        left_margin + plot_w / 2.0,
        height - 20.0,
        html_escape(x_axis_label)
    ));
    svg.push_str(&format!(
        "<text x=\"22\" y=\"{:.3}\" text-anchor=\"middle\" transform=\"rotate(-90,22,{:.3})\" font-family=\"Sans,Arial\" font-size=\"90%\">{}</text>",
        top_margin + plot_h / 2.0,
        top_margin + plot_h / 2.0,
        html_escape(y_axis_label)
    ));

    for (sidx, ys) in data_y.iter().enumerate() {
        let clr = palette[sidx % palette.len()];
        let xs = data_x.get(sidx);
        let mut points = String::new();
        let mut finite_points = Vec::new();
        for (i, y) in ys.iter().enumerate() {
            if !y.is_finite() {
                continue;
            }
            let x = xs.and_then(|v| v.get(i)).copied().unwrap_or((i + 1) as f64);
            if !x.is_finite() {
                continue;
            }
            let (px, py) = to_px(x, *y);
            points.push_str(&format!("{:.3},{:.3} ", px, py));
            finite_points.push((px, py));
        }

        if finite_points.len() >= 2 {
            svg.push_str(&format!(
                "<polyline fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" points=\"{}\"/>",
                clr,
                points.trim()
            ));
        }
        if draw_points {
            for (px, py) in finite_points {
                svg.push_str(&format!(
                    "<circle cx=\"{:.3}\" cy=\"{:.3}\" r=\"2.8\" fill=\"{}\"/>",
                    px, py, clr
                ));
            }
        }
    }

    if draw_legend && !series_labels.is_empty() {
        let lx = left_margin + plot_w + 16.0;
        let mut ly = top_margin + 14.0;
        for (sidx, label) in series_labels.iter().enumerate() {
            let clr = palette[sidx % palette.len()];
            svg.push_str(&format!(
                "<line x1=\"{:.3}\" y1=\"{:.3}\" x2=\"{:.3}\" y2=\"{:.3}\" stroke=\"{}\" stroke-width=\"2\"/>",
                lx,
                ly,
                lx + 16.0,
                ly,
                clr
            ));
            svg.push_str(&format!(
                "<text x=\"{:.3}\" y=\"{:.3}\" dominant-baseline=\"middle\" font-family=\"Sans,Arial\" font-size=\"85%\">{}</text>",
                lx + 22.0,
                ly,
                html_escape(label)
            ));
            ly += 18.0;
        }
    }

    svg.push_str("</svg></div>");
    svg
}

fn build_ms_packed_from_bands(red: &Raster, green: &Raster, blue: &Raster) -> Result<Raster, ToolError> {
    if red.rows != green.rows || red.cols != green.cols || red.rows != blue.rows || red.cols != blue.cols {
        return Err(ToolError::Validation(
            "red, green, and blue rasters must share dimensions".to_string(),
        ));
    }

    let (r_min, r_max) = band_min_max(red);
    let (g_min, g_max) = band_min_max(green);
    let (b_min, b_max) = band_min_max(blue);

    let mut out = red.clone();
    out.data_type = DataType::U32;
    out.metadata
        .push(("color_interpretation".to_string(), "packed_rgb".to_string()));

    for r in 0..red.rows as isize {
        for c in 0..red.cols as isize {
            let rv = red.get(0, r, c);
            let gv = green.get(0, r, c);
            let bv = blue.get(0, r, c);
            if red.is_nodata(rv) || green.is_nodata(gv) || blue.is_nodata(bv) {
                out.set(0, r, c, out.nodata).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed writing packed MS nodata at ({r},{c}): {e}"
                    ))
                })?;
                continue;
            }
            let r8 = (norm01(rv, r_min, r_max) * 255.0).round().clamp(0.0, 255.0) as u32;
            let g8 = (norm01(gv, g_min, g_max) * 255.0).round().clamp(0.0, 255.0) as u32;
            let b8 = (norm01(bv, b_min, b_max) * 255.0).round().clamp(0.0, 255.0) as u32;
            out.set(0, r, c, FlipImageTool::pack_rgba(r8, g8, b8, 255)).map_err(|e| {
                ToolError::Execution(format!(
                    "failed writing packed MS pixel at ({r},{c}): {e}"
                ))
            })?;
        }
    }
    Ok(out)
}

fn run_panchromatic_sharpening(
    ms_packed: &Raster,
    pan: &Raster,
    method: PanSharpenMethod,
    output_mode: PanSharpenOutputMode,
) -> Result<Raster, ToolError> {
    FlipImageTool::validate_packed_rgb(ms_packed, "panchromatic_sharpening")?;
    let rows_pan = pan.rows as isize;
    let cols_pan = pan.cols as isize;

    let pan_stats = pan.statistics();
    let pan_min = pan_stats.min;
    let pan_range = (pan_stats.max - pan_stats.min).max(1e-12);

    let mut output = Raster::new(RasterConfig {
        rows: pan.rows,
        cols: pan.cols,
        bands: if matches!(output_mode, PanSharpenOutputMode::Bands) { 3 } else { 1 },
        x_min: pan.x_min,
        y_min: pan.y_min,
        cell_size: pan.cell_size_x,
        cell_size_y: Some(pan.cell_size_y),
        nodata: pan.nodata,
        data_type: if matches!(output_mode, PanSharpenOutputMode::Bands) {
            DataType::F32
        } else {
            DataType::U32
        },
        crs: pan.crs.clone(),
        metadata: pan.metadata.clone(),
    });

    if matches!(output_mode, PanSharpenOutputMode::Packed) {
        output
            .metadata
            .push(("color_interpretation".to_string(), "packed_rgb".to_string()));
    }

    let ms_y_max = ms_packed.y_max();
    let pan_y_max = pan.y_max();

    for r in 0..rows_pan {
        let y = pan_y_max - (r as f64 + 0.5) * pan.cell_size_y;
        let src_r = ((ms_y_max - y) / ms_packed.cell_size_y).floor() as isize;
        for c in 0..cols_pan {
            let x = pan.x_min + (c as f64 + 0.5) * pan.cell_size_x;
            let src_c = ((x - ms_packed.x_min) / ms_packed.cell_size_x).floor() as isize;

            let p_raw = pan.get(0, r, c);
            if pan.is_nodata(p_raw)
                || src_r < 0
                || src_c < 0
                || src_r >= ms_packed.rows as isize
                || src_c >= ms_packed.cols as isize
            {
                if matches!(output_mode, PanSharpenOutputMode::Packed) {
                    output.set(0, r, c, output.nodata).map_err(|e| {
                        ToolError::Execution(format!(
                            "failed writing pan-sharpen nodata pixel at ({r},{c}): {e}"
                        ))
                    })?;
                } else {
                    for b in 0..3 {
                        output.set(b, r, c, output.nodata).map_err(|e| {
                            ToolError::Execution(format!(
                                "failed writing pan-sharpen nodata band pixel at ({r},{c}): {e}"
                            ))
                        })?;
                    }
                }
                continue;
            }

            let ms_raw = ms_packed.get(0, src_r, src_c);
            if ms_packed.is_nodata(ms_raw) {
                if matches!(output_mode, PanSharpenOutputMode::Packed) {
                    output.set(0, r, c, output.nodata).map_err(|e| {
                        ToolError::Execution(format!(
                            "failed writing pan-sharpen nodata MS pixel at ({r},{c}): {e}"
                        ))
                    })?;
                } else {
                    for b in 0..3 {
                        output.set(b, r, c, output.nodata).map_err(|e| {
                            ToolError::Execution(format!(
                                "failed writing pan-sharpen nodata MS band pixel at ({r},{c}): {e}"
                            ))
                        })?;
                    }
                }
                continue;
            }

            let (r8, g8, b8, _) = FlipImageTool::unpack_rgba(ms_raw);
            let mut rn = r8 as f64 / 255.0;
            let mut gn = g8 as f64 / 255.0;
            let mut bn = b8 as f64 / 255.0;

            let p = ((p_raw - pan_min) / pan_range).clamp(0.0, 1.0);

            match method {
                PanSharpenMethod::Brovey => {
                    let adj = (rn + gn + bn) / 3.0;
                    if adj > 1e-12 {
                        rn = (rn * p / adj).clamp(0.0, 1.0);
                        gn = (gn * p / adj).clamp(0.0, 1.0);
                        bn = (bn * p / adj).clamp(0.0, 1.0);
                    } else {
                        rn = 0.0;
                        gn = 0.0;
                        bn = 0.0;
                    }
                }
                PanSharpenMethod::Ihs => {
                    if (rn - gn).abs() > 1e-12 || (gn - bn).abs() > 1e-12 {
                        let (h, s, _) = rgb_to_hsi_norm(rn, gn, bn);
                        let (r2, g2, b2) = hsi_to_rgb_norm(h, s, p);
                        rn = r2;
                        gn = g2;
                        bn = b2;
                    } else {
                        rn = (rn * p).clamp(0.0, 1.0);
                        gn = (gn * p).clamp(0.0, 1.0);
                        bn = (bn * p).clamp(0.0, 1.0);
                    }
                }
            }

            if matches!(output_mode, PanSharpenOutputMode::Packed) {
                let r_u32 = (rn * 255.0).round().clamp(0.0, 255.0) as u32;
                let g_u32 = (gn * 255.0).round().clamp(0.0, 255.0) as u32;
                let b_u32 = (bn * 255.0).round().clamp(0.0, 255.0) as u32;
                let packed = FlipImageTool::pack_rgba(r_u32, g_u32, b_u32, 255);
                output.set(0, r, c, packed).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed writing pan-sharpen packed pixel at ({r},{c}): {e}"
                    ))
                })?;
            } else {
                output.set(0, r, c, rn * 255.0).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed writing pan-sharpen red band pixel at ({r},{c}): {e}"
                    ))
                })?;
                output.set(1, r, c, gn * 255.0).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed writing pan-sharpen green band pixel at ({r},{c}): {e}"
                    ))
                })?;
                output.set(2, r, c, bn * 255.0).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed writing pan-sharpen blue band pixel at ({r},{c}): {e}"
                    ))
                })?;
            }
        }
    }

    Ok(output)
}

fn run_change_vector_analysis(date1: &[Raster], date2: &[Raster]) -> Result<(Raster, Raster), ToolError> {
    if date1.is_empty() || date2.is_empty() {
        return Err(ToolError::Validation(
            "change_vector_analysis requires at least one raster in each date list".to_string(),
        ));
    }
    if date1.len() != date2.len() {
        return Err(ToolError::Validation(
            "change_vector_analysis requires equal-length date lists".to_string(),
        ));
    }

    let template = &date1[0];
    let rows = template.rows as isize;
    let cols = template.cols as isize;
    let out_nodata = template.nodata;

    for (idx, (a, b)) in date1.iter().zip(date2.iter()).enumerate() {
        if a.rows != template.rows || a.cols != template.cols || b.rows != template.rows || b.cols != template.cols {
            return Err(ToolError::Validation(format!(
                "all input rasters must share dimensions; mismatch found at pair index {}",
                idx
            )));
        }
    }

    let mut mag = Raster::new(RasterConfig {
        rows: template.rows,
        cols: template.cols,
        bands: 1,
        x_min: template.x_min,
        y_min: template.y_min,
        cell_size: template.cell_size_x,
        cell_size_y: Some(template.cell_size_y),
        nodata: out_nodata,
        data_type: DataType::F32,
        crs: template.crs.clone(),
        metadata: template.metadata.clone(),
    });
    let mut dir = Raster::new(RasterConfig {
        rows: template.rows,
        cols: template.cols,
        bands: 1,
        x_min: template.x_min,
        y_min: template.y_min,
        cell_size: template.cell_size_x,
        cell_size_y: Some(template.cell_size_y),
        nodata: out_nodata,
        data_type: DataType::F32,
        crs: template.crs.clone(),
        metadata: template.metadata.clone(),
    });

    for r in 0..rows {
        for c in 0..cols {
            let mut nodata_detected = false;
            let mut mag_acc = 0.0f64;
            let mut dir_code = 0.0f64;

            for i in 0..date1.len() {
                let a = &date1[i];
                let b = &date2[i];
                let z1 = a.get(0, r, c);
                let z2 = b.get(0, r, c);
                if a.is_nodata(z1) || b.is_nodata(z2) {
                    nodata_detected = true;
                    break;
                }
                let dz = z2 - z1;
                mag_acc += dz * dz;
                if dz >= 0.0 {
                    dir_code += 2f64.powi(i as i32);
                }
            }

            if nodata_detected {
                mag.set(0, r, c, out_nodata).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed writing CVA nodata magnitude at ({r},{c}): {e}"
                    ))
                })?;
                dir.set(0, r, c, out_nodata).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed writing CVA nodata direction at ({r},{c}): {e}"
                    ))
                })?;
            } else {
                mag.set(0, r, c, mag_acc.sqrt()).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed writing CVA magnitude at ({r},{c}): {e}"
                    ))
                })?;
                dir.set(0, r, c, dir_code).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed writing CVA direction at ({r},{c}): {e}"
                    ))
                })?;
            }
        }
    }

    Ok((mag, dir))
}

fn run_write_function_memory_insertion(
    input_r: &Raster,
    input_g: &Raster,
    input_b: &Raster,
) -> Result<Raster, ToolError> {
    if input_r.rows != input_g.rows
        || input_r.cols != input_g.cols
        || input_r.rows != input_b.rows
        || input_r.cols != input_b.cols
    {
        return Err(ToolError::Validation(
            "input1, input2, and input3 must have matching rows and columns".to_string(),
        ));
    }

    let stats_r = input_r.statistics();
    let stats_g = input_g.statistics();
    let stats_b = input_b.statistics();

    let r_min = stats_r.min;
    let g_min = stats_g.min;
    let b_min = stats_b.min;
    let r_range = (stats_r.max - stats_r.min).max(1e-12);
    let g_range = (stats_g.max - stats_g.min).max(1e-12);
    let b_range = (stats_b.max - stats_b.min).max(1e-12);

    let mut out = input_r.clone();
    out.data_type = DataType::U32;
    out.metadata
        .push(("color_interpretation".to_string(), "packed_rgb".to_string()));

    let alpha = 255u32 << 24;
    for r in 0..input_r.rows as isize {
        for c in 0..input_r.cols as isize {
            let rv = input_r.get(0, r, c);
            let gv = input_g.get(0, r, c);
            let bv = input_b.get(0, r, c);
            if input_r.is_nodata(rv) || input_g.is_nodata(gv) || input_b.is_nodata(bv) {
                out.set(0, r, c, out.nodata).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed writing WFM insertion nodata pixel at ({r},{c}): {e}"
                    ))
                })?;
                continue;
            }

            let r8 = (((rv - r_min) / r_range) * 255.0).round().clamp(0.0, 255.0) as u32;
            let g8 = (((gv - g_min) / g_range) * 255.0).round().clamp(0.0, 255.0) as u32;
            let b8 = (((bv - b_min) / b_range) * 255.0).round().clamp(0.0, 255.0) as u32;
            let packed = (alpha | (b8 << 16) | (g8 << 8) | r8) as f64;
            out.set(0, r, c, packed).map_err(|e| {
                ToolError::Execution(format!(
                    "failed writing WFM insertion packed pixel at ({r},{c}): {e}"
                ))
            })?;
        }
    }

    Ok(out)
}

// ── Shared supervised-classification helper ──────────────────────────────────

/// Local `is_between` – true when `val` lies between `a` and `b` (inclusive, either order).
#[inline(always)]
fn is_between_f64(val: f64, a: f64, b: f64) -> bool {
    (a <= val && val <= b) || (b <= val && val <= a)
}

/// Convert a geographic Y coordinate to a raster row index (row 0 = north edge, top-down).
#[inline(always)]
fn geo_y_to_row(raster: &Raster, y: f64) -> isize {
    ((raster.y_max() - y) / raster.cell_size_y).floor() as isize
}

/// Convert a geographic X coordinate to a raster column index.
#[inline(always)]
fn geo_x_to_col(raster: &Raster, x: f64) -> isize {
    ((x - raster.x_min) / raster.cell_size_x).floor() as isize
}

/// Scan-line rasterize all non-hole rings in `ring` against `reference_raster` and
/// push each valid (row, col) pair into `out_cells`.  All cells whose value in every
/// band of `bands` is nodata are silently skipped.
fn scan_rasterize_ring(
    ring: &wbvector::Ring,
    reference_raster: &Raster,
    bands: &[Raster],
    out_cells: &mut Vec<Vec<f64>>,
) {
    let coords = ring.coords();
    let n = coords.len();
    if n < 3 {
        return;
    }
    let num_bands = bands.len();
    let rows = reference_raster.rows as isize;
    let cols = reference_raster.cols as isize;
    let nodata: Vec<f64> = bands.iter().map(|b| b.nodata).collect();

    // Build bounding box of this ring in raster coordinates.
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for c in coords {
        if c.x < min_x { min_x = c.x; }
        if c.x > max_x { max_x = c.x; }
        if c.y < min_y { min_y = c.y; }
        if c.y > max_y { max_y = c.y; }
    }

    let mut top_row = geo_y_to_row(reference_raster, max_y).max(0);
    let mut bottom_row = geo_y_to_row(reference_raster, min_y).min(rows - 1);
    let mut left_col = geo_x_to_col(reference_raster, min_x).max(0);
    let mut right_col = geo_x_to_col(reference_raster, max_x).min(cols - 1);

    if bottom_row <= top_row || right_col <= left_col {
        return;
    }

    // Clamp to raster extent.
    if top_row < 0 { top_row = 0; }
    if bottom_row >= rows { bottom_row = rows - 1; }
    if left_col < 0 { left_col = 0; }
    if right_col >= cols { right_col = cols - 1; }

    // Scan each row: find e-intersections with ring edges.
    for row in top_row..=bottom_row {
        let row_y = reference_raster.row_center_y(row);
        for i in 0..n {
            let j = (i + 1) % n;
            let (y1, y2) = (coords[i].y, coords[j].y);
            if is_between_f64(row_y, y1, y2) && y2 != y1 {
                let (x1, x2) = (coords[i].x, coords[j].x);
                let x_prime = x1 + (row_y - y1) / (y2 - y1) * (x2 - x1);
                let col = geo_x_to_col(reference_raster, x_prime);
                if col >= 0 && col < cols {
                    let mut vals = vec![0f64; num_bands];
                    let mut is_nodata = false;
                    for (b, raster) in bands.iter().enumerate() {
                        let z = raster.get(0, row, col);
                        if z == nodata[b] { is_nodata = true; break; }
                        vals[b] = z;
                    }
                    if !is_nodata {
                        out_cells.push(vals);
                    }
                }
            }
        }
    }

    // Scan each column: find y-intersections with ring edges.
    for col in left_col..=right_col {
        let col_x = reference_raster.col_center_x(col);
        for i in 0..n {
            let j = (i + 1) % n;
            let (x1, x2) = (coords[i].x, coords[j].x);
            if is_between_f64(col_x, x1, x2) && x1 != x2 {
                let (y1, y2) = (coords[i].y, coords[j].y);
                let y_prime = y1 + (col_x - x1) / (x2 - x1) * (y2 - y1);
                let row = geo_y_to_row(reference_raster, y_prime);
                if row >= 0 && row < rows {
                    let mut vals = vec![0f64; num_bands];
                    let mut is_nodata = false;
                    for (b, raster) in bands.iter().enumerate() {
                        let z = raster.get(0, row, col);
                        if z == nodata[b] { is_nodata = true; break; }
                        vals[b] = z;
                    }
                    if !is_nodata {
                        out_cells.push(vals);
                    }
                }
            }
        }
    }
}

/// Extract training pixels from a polygon vector layer.
///
/// Returns `(class_names, per_class_pixels)` where `per_class_pixels[c]` is a `Vec` of
/// multi-band pixel value vectors sampled from the exterior ring of all features of class `c`.
fn extract_training_polygon_pixels(
    bands: &[Raster],
    layer: &wbvector::Layer,
    field_name: &str,
) -> Result<(Vec<String>, Vec<Vec<Vec<f64>>>), ToolError> {
    let schema = &layer.schema;
    let field_idx = schema.field_index(field_name).ok_or_else(|| {
        ToolError::Validation(format!("field '{}' not found in training data", field_name))
    })?;
    let reference = &bands[0];

    // Collect unique sorted class names.
    let mut class_set = std::collections::HashSet::new();
    for f in layer.features.iter() {
        if let Some(val) = f.attributes.get(field_idx) {
            class_set.insert(val.to_string());
        }
    }
    let mut class_names: Vec<String> = class_set.into_iter().collect();
    class_names.sort();
    let num_classes = class_names.len();

    let mut per_class: Vec<Vec<Vec<f64>>> = vec![Vec::new(); num_classes];

    for f in layer.features.iter() {
        let class_str = f
            .attributes
            .get(field_idx)
            .map(|v| v.to_string())
            .unwrap_or_default();
        let class_idx = match class_names.iter().position(|s| *s == class_str) {
            Some(i) => i,
            None => continue,
        };
        let geom = match &f.geometry {
            Some(g) => g,
            None => continue,
        };
        // Rasterize exterior ring(s) only (holes are skipped).
        match geom {
            VectorGeometry::Polygon { exterior, .. } => {
                scan_rasterize_ring(exterior, reference, bands, &mut per_class[class_idx]);
            }
            VectorGeometry::MultiPolygon(parts) => {
                for (exterior, _) in parts {
                    scan_rasterize_ring(exterior, reference, bands, &mut per_class[class_idx]);
                }
            }
            _ => {}
        }
    }

    Ok((class_names, per_class))
}

// ── MinDistClassificationTool ────────────────────────────────────────────────

impl Tool for MinDistClassificationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "min_dist_classification",
            display_name: "Minimum Distance Classification",
            summary: "Performs a supervised minimum-distance classification on multi-spectral rasters using polygon training data.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Array of single-band input rasters (one per spectral band), as paths or a delimited string.", required: true },
                ToolParamSpec { name: "training_data", description: "Path to a polygon vector file containing training areas.", required: true },
                ToolParamSpec { name: "class_field", description: "Name of the attribute field in the training data that holds the class identifier.", required: true },
                ToolParamSpec { name: "dist_threshold", description: "Optional z-score threshold; pixels farther than this from the nearest class mean are left unclassified. Default: no threshold (classify all).", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path. If omitted, the result is kept in memory.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        defaults.insert("training_data".to_string(), json!("training.shp"));
        defaults.insert("class_field".to_string(), json!("class"));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        example.insert("training_data".to_string(), json!("training.shp"));
        example.insert("class_field".to_string(), json!("class"));
        example.insert("dist_threshold".to_string(), json!(3.0));
        example.insert("output".to_string(), json!("classified.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta.params.into_iter().map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            }).collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_min_dist_classification".to_string(),
                description: "Classifies a three-band image with a z-score threshold.".to_string(),
                args: example,
            }],
            tags: vec!["remote_sensing".to_string(), "raster".to_string(), "classification".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let paths = parse_raster_list_arg(args, "inputs")?;
        if paths.is_empty() {
            return Err(ToolError::Validation("'inputs' must contain at least one raster".to_string()));
        }
        parse_vector_path_arg(args, "training_data")?;
        args.get("class_field").ok_or_else(|| ToolError::Validation("parameter 'class_field' is required".to_string()))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let band_paths = parse_raster_list_arg(args, "inputs")?;
        let training_path = parse_vector_path_arg(args, "training_data")?;
        let class_field = args
            .get("class_field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'class_field' is required".to_string()))?
            .to_string();
        let dist_threshold = args
            .get("dist_threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::INFINITY);
        let output_path = parse_optional_output_path(args, "output")?;

        let bands: Vec<Raster> = band_paths.iter().map(|p| FlipImageTool::load_raster(p)).collect::<Result<_, _>>()?;
        let num_bands = bands.len();
        let rows = bands[0].rows as isize;
        let cols_count = bands[0].cols as isize;

        // Verify all bands share dimensions.
        for b in &bands {
            if b.rows as isize != rows || b.cols as isize != cols_count {
                return Err(ToolError::Validation("All input bands must share the same dimensions.".to_string()));
            }
        }

        let layer = wbvector::read(&training_path).map_err(|e| {
            ToolError::Execution(format!("failed reading training data '{}': {}", training_path, e))
        })?;

        ctx.progress.progress(0.05);

        let (class_names, per_class) = extract_training_polygon_pixels(&bands, &layer, &class_field)?;
        let num_classes = class_names.len();
        if num_classes == 0 {
            return Err(ToolError::Validation("No classes found in training data.".to_string()));
        }

        // Compute per-class mean vectors.
        let mut class_mean = vec![vec![0f64; num_bands]; num_classes];
        let mut class_n = vec![0usize; num_classes];
        for c in 0..num_classes {
            for vals in &per_class[c] {
                for b in 0..num_bands { class_mean[c][b] += vals[b]; }
                class_n[c] += 1;
            }
            if class_n[c] > 0 {
                for b in 0..num_bands { class_mean[c][b] /= class_n[c] as f64; }
            }
        }

        // Compute per-class mean distance and standard deviation of distances (for z-score threshold).
        let mut class_mean_dist = vec![0f64; num_classes];
        let mut class_stddev = vec![0f64; num_classes];
        if dist_threshold.is_finite() {
            for c in 0..num_classes {
                let n = per_class[c].len();
                if n > 0 {
                    let mut sum_dist = 0f64;
                    let dists: Vec<f64> = per_class[c].iter().map(|vals| {
                        vals.iter().enumerate().map(|(b, &v)| (v - class_mean[c][b]).powi(2)).sum::<f64>().sqrt()
                    }).collect();
                    for &d in &dists { sum_dist += d; }
                    class_mean_dist[c] = sum_dist / n as f64;
                    let var: f64 = dists.iter().map(|&d| (d - class_mean_dist[c]).powi(2)).sum::<f64>() / n as f64;
                    class_stddev[c] = var.sqrt();
                }
            }
        }

        ctx.progress.progress(0.15);

        // Classify each pixel.
        let nodata_val = -32768f64;
        let mut output = Raster::new(RasterConfig {
            rows: bands[0].rows,
            cols: bands[0].cols,
            bands: 1,
            x_min: bands[0].x_min,
            y_min: bands[0].y_min,
            cell_size: bands[0].cell_size_x,
            cell_size_y: Some(bands[0].cell_size_y),
            nodata: nodata_val,
            data_type: DataType::I16,
            crs: bands[0].crs.clone(),
            metadata: vec![],
        });

        let labels: Vec<Option<usize>> = (0..(rows * cols_count) as usize)
            .into_par_iter()
            .map(|idx| {
                let row = idx as isize / cols_count;
                let col = idx as isize % cols_count;

                let mut pixel = vec![0f64; num_bands];
                for b in 0..num_bands {
                    let z = bands[b].get(0, row, col);
                    if bands[b].is_nodata(z) {
                        return None;
                    }
                    pixel[b] = z;
                }

                let mut min_dist = f64::INFINITY;
                let mut min_class = num_classes;
                for c in 0..num_classes {
                    let d: f64 = pixel
                        .iter()
                        .enumerate()
                        .map(|(b, &v)| (v - class_mean[c][b]).powi(2))
                        .sum::<f64>()
                        .sqrt();
                    if d < min_dist {
                        min_dist = d;
                        min_class = c;
                    }
                }

                if min_class < num_classes {
                    if dist_threshold.is_finite() {
                        let std = class_stddev[min_class];
                        let zscore = if std > 0.0 {
                            (min_dist - class_mean_dist[min_class]) / std
                        } else {
                            0.0
                        };
                        if zscore >= dist_threshold {
                            return None;
                        }
                    }
                    Some(min_class)
                } else {
                    None
                }
            })
            .collect();

        for row in 0..rows {
            let base = (row * cols_count) as usize;
            for col in 0..cols_count {
                if let Some(class_idx) = labels[base + col as usize] {
                    let _ = output.set(0, row, col, (class_idx + 1) as f64);
                }
            }
            if row % 100 == 0 {
                ctx.progress.progress(0.15 + 0.80 * (row as f64 / rows as f64));
            }
        }

        ctx.progress.progress(1.0);
        let raster_out = FlipImageTool::store_named_raster_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), raster_out);
        Ok(ToolRunResult { outputs })
    }
}

// ── ParallelepipedClassificationTool ────────────────────────────────────────

impl Tool for ParallelepipedClassificationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "parallelepiped_classification",
            display_name: "Parallelepiped Classification",
            summary: "Performs a supervised parallelepiped classification on multi-spectral rasters using polygon training data.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Array of single-band input rasters (one per spectral band), as paths or a delimited string.", required: true },
                ToolParamSpec { name: "training_data", description: "Path to a polygon vector file containing training areas.", required: true },
                ToolParamSpec { name: "class_field", description: "Name of the attribute field in the training data that holds the class identifier.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path. If omitted, the result is kept in memory.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        defaults.insert("training_data".to_string(), json!("training.shp"));
        defaults.insert("class_field".to_string(), json!("class"));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        example.insert("training_data".to_string(), json!("training.shp"));
        example.insert("class_field".to_string(), json!("class"));
        example.insert("output".to_string(), json!("classified.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta.params.into_iter().map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            }).collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_parallelepiped_classification".to_string(),
                description: "Classifies a three-band image using the parallelepiped method.".to_string(),
                args: example,
            }],
            tags: vec!["remote_sensing".to_string(), "raster".to_string(), "classification".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let paths = parse_raster_list_arg(args, "inputs")?;
        if paths.is_empty() {
            return Err(ToolError::Validation("'inputs' must contain at least one raster".to_string()));
        }
        parse_vector_path_arg(args, "training_data")?;
        args.get("class_field").ok_or_else(|| ToolError::Validation("parameter 'class_field' is required".to_string()))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let band_paths = parse_raster_list_arg(args, "inputs")?;
        let training_path = parse_vector_path_arg(args, "training_data")?;
        let class_field = args
            .get("class_field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'class_field' is required".to_string()))?
            .to_string();
        let output_path = parse_optional_output_path(args, "output")?;

        let bands: Vec<Raster> = band_paths.iter().map(|p| FlipImageTool::load_raster(p)).collect::<Result<_, _>>()?;
        let num_bands = bands.len();
        let rows = bands[0].rows as isize;
        let cols_count = bands[0].cols as isize;

        for b in &bands {
            if b.rows as isize != rows || b.cols as isize != cols_count {
                return Err(ToolError::Validation("All input bands must share the same dimensions.".to_string()));
            }
        }

        let layer = wbvector::read(&training_path).map_err(|e| {
            ToolError::Execution(format!("failed reading training data '{}': {}", training_path, e))
        })?;

        ctx.progress.progress(0.05);

        let (class_names, per_class) = extract_training_polygon_pixels(&bands, &layer, &class_field)?;
        let num_classes = class_names.len();
        if num_classes == 0 {
            return Err(ToolError::Validation("No classes found in training data.".to_string()));
        }

        // Compute per-class min/max vectors.
        let mut class_min = vec![vec![f64::INFINITY; num_bands]; num_classes];
        let mut class_max = vec![vec![f64::NEG_INFINITY; num_bands]; num_classes];
        for c in 0..num_classes {
            for vals in &per_class[c] {
                for b in 0..num_bands {
                    if vals[b] < class_min[c][b] { class_min[c][b] = vals[b]; }
                    if vals[b] > class_max[c][b] { class_max[c][b] = vals[b]; }
                }
            }
        }

        // Sort classes by hyper-volume (smallest first) so the tightest class wins ties.
        let mut class_index: Vec<(usize, f64)> = (0..num_classes).map(|c| {
            let vol = (0..num_bands).map(|b| (class_max[c][b] - class_min[c][b]).max(0.0)).product::<f64>();
            (c, vol)
        }).collect();
        class_index.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        ctx.progress.progress(0.15);

        let nodata_val = -32768f64;
        let mut output = Raster::new(RasterConfig {
            rows: bands[0].rows,
            cols: bands[0].cols,
            bands: 1,
            x_min: bands[0].x_min,
            y_min: bands[0].y_min,
            cell_size: bands[0].cell_size_x,
            cell_size_y: Some(bands[0].cell_size_y),
            nodata: nodata_val,
            data_type: DataType::I16,
            crs: bands[0].crs.clone(),
            metadata: vec![],
        });

        let labels: Vec<Option<usize>> = (0..(rows * cols_count) as usize)
            .into_par_iter()
            .map(|idx| {
                let row = idx as isize / cols_count;
                let col = idx as isize % cols_count;

                let mut pixel = vec![0f64; num_bands];
                for b in 0..num_bands {
                    let z = bands[b].get(0, row, col);
                    if bands[b].is_nodata(z) {
                        return None;
                    }
                    pixel[b] = z;
                }

                for &(c, _) in &class_index {
                    let mut inside = true;
                    for b in 0..num_bands {
                        if pixel[b] < class_min[c][b] || pixel[b] > class_max[c][b] {
                            inside = false;
                            break;
                        }
                    }
                    if inside {
                        return Some(c);
                    }
                }
                None
            })
            .collect();

        for row in 0..rows {
            let base = (row * cols_count) as usize;
            for col in 0..cols_count {
                if let Some(class_idx) = labels[base + col as usize] {
                    let _ = output.set(0, row, col, (class_idx + 1) as f64);
                }
            }
            if row % 100 == 0 {
                ctx.progress.progress(0.15 + 0.80 * (row as f64 / rows as f64));
            }
        }

        ctx.progress.progress(1.0);
        let raster_out = FlipImageTool::store_named_raster_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), raster_out);
        Ok(ToolRunResult { outputs })
    }
}

// ── CannyEdgeDetectionTool ───────────────────────────────────────────────────

impl Tool for CannyEdgeDetectionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "canny_edge_detection",
            display_name: "Canny Edge Detection",
            summary: "Applies Canny multi-stage edge detection (Gaussian blur → Sobel gradient → non-maximum suppression → double threshold → hysteresis).",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster path (single-band or packed-RGB).", required: true },
                ToolParamSpec { name: "sigma", description: "Standard deviation of the Gaussian smoothing kernel in pixels. Default: 0.5.", required: false },
                ToolParamSpec { name: "low_threshold", description: "Low hysteresis threshold (0–1 fraction of max gradient). Default: 0.05.", required: false },
                ToolParamSpec { name: "high_threshold", description: "High hysteresis threshold (0–1 fraction of max gradient). Default: 0.15.", required: false },
                ToolParamSpec { name: "add_back", description: "If true, edge pixels are zeroed in the original image rather than producing a binary edge map. Default: false.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path. If omitted, the result is kept in memory.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("sigma".to_string(), json!(0.5));
        defaults.insert("low_threshold".to_string(), json!(0.05));
        defaults.insert("high_threshold".to_string(), json!(0.15));
        defaults.insert("add_back".to_string(), json!(false));

        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("image.tif"));
        example.insert("sigma".to_string(), json!(1.0));
        example.insert("low_threshold".to_string(), json!(0.05));
        example.insert("high_threshold".to_string(), json!(0.15));
        example.insert("output".to_string(), json!("edges.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta.params.into_iter().map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            }).collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_canny_edge_detection".to_string(),
                description: "Detects edges with default sigma and thresholds.".to_string(),
                args: example,
            }],
            tags: vec!["remote_sensing".to_string(), "raster".to_string(), "filter".to_string(), "edge_detection".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        parse_raster_path_arg(args, "input")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let sigma = args.get("sigma").and_then(|v| v.as_f64()).unwrap_or(0.5).max(0.15).min(20.0);
        let mut low_threshold = args.get("low_threshold").and_then(|v| v.as_f64()).unwrap_or(0.05_f64).clamp(0.0, 1.0);
        let mut high_threshold = args.get("high_threshold").and_then(|v| v.as_f64()).unwrap_or(0.15_f64).clamp(0.0, 1.0);
        let add_back = args.get("add_back").and_then(|v| v.as_bool()).unwrap_or(false);
        let output_path = parse_optional_output_path(args, "output")?;

        let input = FlipImageTool::load_raster(&input_path)?;
        let rows = input.rows as isize;
        let cols_count = input.cols as isize;
        let nodata = input.nodata;

        let is_rgb = color_support::detect_rgb_mode(&input, false, true) == color_support::RgbMode::Packed;

        // ── Build Gaussian kernel ────────────────────────────────────────────
        let recip_root_2pi_sigma = 1.0 / ((2.0 * std::f64::consts::PI).sqrt() * sigma);
        let two_sigma_sq = 2.0 * sigma * sigma;

        let mut filter_size = 3usize;
        for i in 0usize..250 {
            let w = recip_root_2pi_sigma * (-(i as f64 * i as f64) / two_sigma_sq).exp();
            if w <= 0.001 {
                filter_size = i * 2 + 1;
                break;
            }
        }
        if filter_size % 2 == 0 { filter_size += 1; }
        if filter_size < 3 { filter_size = 3; }

        let half = (filter_size / 2) as isize;
        let mut kernel_weights: Vec<f64> = Vec::with_capacity(filter_size * filter_size);
        let mut kernel_dx: Vec<isize> = Vec::with_capacity(filter_size * filter_size);
        let mut kernel_dy: Vec<isize> = Vec::with_capacity(filter_size * filter_size);
        for ky in 0..filter_size as isize {
            for kx in 0..filter_size as isize {
                let dx = kx - half;
                let dy = ky - half;
                let w = recip_root_2pi_sigma * (-((dx * dx + dy * dy) as f64) / two_sigma_sq).exp();
                kernel_dx.push(dx);
                kernel_dy.push(dy);
                kernel_weights.push(w);
            }
        }
        let kn = kernel_weights.len();

        // Helper: get intensity value at (row, col) – uses hsi 'i' for packed RGB.
        let get_intensity = |raster: &Raster, row: isize, col: isize| -> f64 {
            let z = raster.get(0, row, col);
            if raster.is_nodata(z) { return nodata; }
            if is_rgb { value2i(z) } else { z }
        };

        // ── Stage 1: Gaussian filter → `g` ──────────────────────────────────
        let g_nd = nodata;
        let mut g_data = vec![g_nd; (rows * cols_count) as usize];
        for row in 0..rows {
            for col in 0..cols_count {
                let z = get_intensity(&input, row, col);
                if z == nodata { continue; }
                let mut sum = 0.0f64;
                let mut acc = 0.0f64;
                for k in 0..kn {
                    let nr = row + kernel_dy[k];
                    let nc = col + kernel_dx[k];
                    if nr < 0 || nr >= rows || nc < 0 || nc >= cols_count { continue; }
                    let zn = get_intensity(&input, nr, nc);
                    if zn != nodata {
                        sum += kernel_weights[k];
                        acc += kernel_weights[k] * zn;
                    }
                }
                if sum > 0.0 { g_data[(row * cols_count + col) as usize] = acc / sum; }
            }
        }
        ctx.progress.progress(0.25);

        // ── Stage 2: Sobel gradient magnitude + angle ────────────────────────
        let gget = |row: isize, col: isize| -> f64 {
            if row < 0 || row >= rows || col < 0 || col >= cols_count { return g_nd; }
            g_data[(row * cols_count + col) as usize]
        };
        let sobel_dx = [1isize, 1, 1, 0, -1, -1, -1, 0];
        let sobel_dy = [-1isize, 0, 1, 1, 1, 0, -1, -1];
        let sobel_mx = [1.0f64, 2.0, 1.0, 0.0, -1.0, -2.0, -1.0, 0.0];
        let sobel_my = [1.0f64, 0.0, -1.0, -2.0, -1.0, 0.0, 1.0, 2.0];

        let mut slope_mag = vec![g_nd; (rows * cols_count) as usize];
        let mut theta_data = vec![g_nd; (rows * cols_count) as usize];
        let mut max_slope = 0.0f64;
        for row in 0..rows {
            for col in 0..cols_count {
                let z = gget(row, col);
                if z == g_nd { continue; }
                let mut sx = 0.0f64;
                let mut sy = 0.0f64;
                for i in 0..8 {
                    let zn = gget(row + sobel_dy[i], col + sobel_dx[i]);
                    let zn = if zn == g_nd { z } else { zn };
                    sx += zn * sobel_mx[i];
                    sy += zn * sobel_my[i];
                }
                let mag = sx.hypot(sy);
                slope_mag[(row * cols_count + col) as usize] = mag;
                theta_data[(row * cols_count + col) as usize] = sy.atan2(sx);
                if mag > max_slope { max_slope = mag; }
            }
        }
        // Normalise magnitude to 0–255.
        if max_slope > 0.0 {
            for v in slope_mag.iter_mut() {
                if *v != g_nd { *v = *v / max_slope * 255.0; }
            }
        }
        ctx.progress.progress(0.50);

        // ── Stage 3: Non-maximum suppression ─────────────────────────────────
        let smget = |row: isize, col: isize| -> f64 {
            if row < 0 || row >= rows || col < 0 || col >= cols_count { return 255.0; }
            slope_mag[(row * cols_count + col) as usize]
        };
        let mut max_nms = 0.0f64;
        let mut nms = vec![0.0f64; (rows * cols_count) as usize];
        for row in 0..rows {
            for col in 0..cols_count {
                let v = slope_mag[(row * cols_count + col) as usize];
                if v == g_nd { continue; }
                let angle = theta_data[(row * cols_count + col) as usize] * 180.0 / std::f64::consts::PI;
                let angle = if angle < 0.0 { angle + 180.0 } else { angle };
                let (q, r) = if (0.0 <= angle && angle < 22.5) || (157.5 <= angle && angle <= 180.0) {
                    (smget(row, col + 1), smget(row, col - 1))
                } else if 22.5 <= angle && angle < 67.5 {
                    (smget(row + 1, col - 1), smget(row - 1, col + 1))
                } else if 67.5 <= angle && angle < 112.5 {
                    (smget(row + 1, col), smget(row - 1, col))
                } else {
                    (smget(row - 1, col - 1), smget(row + 1, col + 1))
                };
                if v >= q && v >= r {
                    nms[(row * cols_count + col) as usize] = v;
                    if v > max_nms { max_nms = v; }
                }
            }
        }
        drop(slope_mag);
        drop(theta_data);
        ctx.progress.progress(0.65);

        // ── Stage 4: Double threshold ─────────────────────────────────────────
        high_threshold = max_nms * high_threshold;
        low_threshold = high_threshold * low_threshold;
        const STRONG: f64 = 255.0;
        const WEAK: f64 = 75.0;
        let mut thresh = vec![0.0f64; (rows * cols_count) as usize];
        for row in 0..rows {
            for col in 0..cols_count {
                let v = nms[(row * cols_count + col) as usize];
                thresh[(row * cols_count + col) as usize] = if v >= high_threshold {
                    STRONG
                } else if v >= low_threshold {
                    WEAK
                } else {
                    0.0
                };
            }
        }
        drop(nms);
        ctx.progress.progress(0.80);

        // ── Stage 5: Hysteresis ───────────────────────────────────────────────
        let tget = |row: isize, col: isize| -> f64 {
            if row < 0 || row >= rows || col < 0 || col >= cols_count { return 0.0; }
            thresh[(row * cols_count + col) as usize]
        };
        let out_nodata = if !add_back { -32768.0f64 } else { nodata };
        let mut output = Raster::new(RasterConfig {
            rows: input.rows,
            cols: input.cols,
            bands: 1,
            x_min: input.x_min,
            y_min: input.y_min,
            cell_size: input.cell_size_x,
            cell_size_y: Some(input.cell_size_y),
            nodata: out_nodata,
            data_type: if !add_back || !is_rgb { DataType::I16 } else { DataType::F32 },
            crs: input.crs.clone(),
            metadata: vec![],
        });

        for row in 0..rows {
            for col in 0..cols_count {
                let v = thresh[(row * cols_count + col) as usize];
                let iz = get_intensity(&input, row, col);
                let out_val = if iz == nodata {
                    out_nodata
                } else if v == WEAK {
                    // Hysteresis: promote weak pixels that are 8-connected to a strong pixel.
                    let strong_nbr = tget(row+1,col-1)==STRONG || tget(row+1,col)==STRONG
                        || tget(row+1,col+1)==STRONG || tget(row,col-1)==STRONG
                        || tget(row,col+1)==STRONG || tget(row-1,col-1)==STRONG
                        || tget(row-1,col)==STRONG || tget(row-1,col+1)==STRONG;
                    if !add_back {
                        if strong_nbr { STRONG } else { 0.0 }
                    } else {
                        if strong_nbr { 0.0 } else { iz }
                    }
                } else if v == STRONG {
                    if !add_back { STRONG } else { 0.0 }
                } else {
                    if !add_back { 0.0 } else { iz }
                };
                let _ = output.set(0, row, col, out_val);
            }
        }

        ctx.progress.progress(1.0);
        let raster_out = FlipImageTool::store_named_raster_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), raster_out);
        Ok(ToolRunResult { outputs })
    }
}

fn percentile_sorted(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let pp = p.clamp(0.0, 1.0);
    let x = pp * (sorted.len() - 1) as f64;
    let lo = x.floor() as usize;
    let hi = x.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        let t = x - lo as f64;
        sorted[lo] + t * (sorted[hi] - sorted[lo])
    }
}

fn values_to_box_row(values: &mut [f64]) -> (f64, f64, f64, f64, f64, f64, f64) {
    if values.is_empty() {
        return (f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN);
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let min = values[0];
    let max = values[values.len() - 1];
    let q1 = percentile_sorted(values, 0.25);
    let med = percentile_sorted(values, 0.50);
    let q3 = percentile_sorted(values, 0.75);
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let var = values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / values.len() as f64;
    (min, q1, med, q3, max, mean, var.sqrt())
}

fn raster_index(cols: usize, row: isize, col: isize) -> usize {
    row as usize * cols + col as usize
}

fn sqr_dist(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let d = x - y;
            d * d
        })
        .sum()
}

fn raster_mean_stdev_valid(raster: &Raster) -> (f64, f64) {
    let rows = raster.rows as isize;
    let cols = raster.cols as isize;
    let mut n = 0usize;
    let mut sum = 0.0;
    for row in 0..rows {
        for col in 0..cols {
            let z = raster.get(0, row, col);
            if !raster.is_nodata(z) {
                sum += z;
                n += 1;
            }
        }
    }
    if n == 0 {
        return (0.0, 1.0);
    }
    let mean = sum / n as f64;
    let mut var_sum = 0.0;
    for row in 0..rows {
        for col in 0..cols {
            let z = raster.get(0, row, col);
            if !raster.is_nodata(z) {
                let d = z - mean;
                var_sum += d * d;
            }
        }
    }
    let stdev = (var_sum / n as f64).sqrt();
    (mean, if stdev.abs() < 1e-12 { 1.0 } else { stdev })
}

impl Tool for EvaluateTrainingSitesTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "evaluate_training_sites",
            display_name: "Evaluate Training Sites",
            summary: "Evaluates class separability in multi-band training polygons and writes an HTML report with per-band distribution statistics.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Array of single-band input rasters (one per spectral band), as paths or a delimited string.", required: true },
                ToolParamSpec { name: "training_data", description: "Path to polygon training data.", required: true },
                ToolParamSpec { name: "class_field", description: "Class field name in the training_data attributes.", required: true },
                ToolParamSpec { name: "output", description: "Output HTML file path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        defaults.insert("training_data".to_string(), json!("training.shp"));
        defaults.insert("class_field".to_string(), json!("class"));
        defaults.insert("output".to_string(), json!("training_sites_report.html"));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        example.insert("training_data".to_string(), json!("training.shp"));
        example.insert("class_field".to_string(), json!("class"));
        example.insert("output".to_string(), json!("training_sites_report.html"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta
                .params
                .into_iter()
                .map(|p| ToolParamDescriptor {
                    name: p.name.to_string(),
                    description: p.description.to_string(),
                    required: p.required,
                })
                .collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_evaluate_training_sites".to_string(),
                description: "Create a training-site evaluation HTML report for three image bands.".to_string(),
                args: example,
            }],
            tags: vec!["remote_sensing".to_string(), "classification".to_string(), "report".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.is_empty() {
            return Err(ToolError::Validation("parameter 'inputs' must contain at least one raster".to_string()));
        }
        let _ = parse_vector_path_arg(args, "training_data")?;
        let _ = args
            .get("class_field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'class_field' is required".to_string()))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let band_paths = parse_raster_list_arg(args, "inputs")?;
        let training_path = parse_vector_path_arg(args, "training_data")?;
        let class_field = args
            .get("class_field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'class_field' is required".to_string()))?
            .to_string();

        let output_path = if let Some(path) = parse_optional_output_path(args, "output")? {
            path
        } else {
            std::env::current_dir()
                .map_err(|e| ToolError::Execution(format!("failed reading current directory: {e}")))?
                .join("training_sites_report.html")
        };

        let bands: Vec<Raster> = band_paths
            .iter()
            .map(|p| FlipImageTool::load_raster(p))
            .collect::<Result<_, _>>()?;
        let rows = bands[0].rows;
        let cols = bands[0].cols;
        for (i, b) in bands.iter().enumerate() {
            if b.rows != rows || b.cols != cols {
                return Err(ToolError::Validation(format!(
                    "input raster dimensions mismatch at index {}",
                    i
                )));
            }
        }

        let layer = wbvector::read(&training_path).map_err(|e| {
            ToolError::Execution(format!("failed reading training data '{}': {}", training_path, e))
        })?;

        ctx.progress.progress(0.15);
        let (class_names, class_pixels) = extract_training_polygon_pixels(&bands, &layer, &class_field)?;
        if class_names.is_empty() {
            return Err(ToolError::Validation("no classes found in training data".to_string()));
        }

        let mut html = String::new();
        html.push_str("<!doctype html><html><head><meta charset=\"utf-8\"><title>Evaluate Training Sites</title>");
        html.push_str(wbw_report_css());
        html.push_str("</head><body><h1>Evaluate Training Sites</h1><p>");
        html.push_str(&format!("<strong>Training data</strong>: {}<br>", html_escape(&training_path)));
        html.push_str(&format!("<strong>Class field</strong>: {}<br>", html_escape(&class_field)));
        html.push_str(&format!("<strong>Num. bands</strong>: {}<br>", bands.len()));
        html.push_str("</p>");

        for b in 0..bands.len() {
            html.push_str(&format!("<h2>Band {}</h2>", b + 1));
            html.push_str("<table><tr><th>Class</th><th>Samples</th><th>Min</th><th>Q1</th><th>Median</th><th>Q3</th><th>Max</th><th>Mean</th><th>Std. Dev.</th></tr>");
            for (cidx, cname) in class_names.iter().enumerate() {
                let mut vals: Vec<f64> = class_pixels[cidx].iter().map(|v| v[b]).collect();
                let n = vals.len();
                let (min, q1, med, q3, max, mean, stdev) = values_to_box_row(&mut vals);
                html.push_str(&format!(
                    "<tr><td>{}</td><td class=\"numberCell\">{}</td><td class=\"numberCell\">{:.6}</td><td class=\"numberCell\">{:.6}</td><td class=\"numberCell\">{:.6}</td><td class=\"numberCell\">{:.6}</td><td class=\"numberCell\">{:.6}</td><td class=\"numberCell\">{:.6}</td><td class=\"numberCell\">{:.6}</td></tr>",
                    html_escape(cname), n, min, q1, med, q3, max, mean, stdev
                ));
            }
            html.push_str("</table>");
            ctx.progress.progress(0.20 + 0.75 * ((b + 1) as f64 / bands.len() as f64));
        }
        html.push_str("</body></html>");

        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ToolError::Execution(format!(
                    "failed creating output directory '{}': {}",
                    parent.display(),
                    e
                ))
            })?;
        }
        std::fs::write(&output_path, html).map_err(|e| {
            ToolError::Execution(format!("failed writing report '{}': {}", output_path.display(), e))
        })?;

        ctx.progress.progress(1.0);
        let mut outputs = BTreeMap::new();
        outputs.insert("path".to_string(), json!(output_path.to_string_lossy().to_string()));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for GeneralizeWithSimilarityTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "generalize_with_similarity",
            display_name: "Generalize With Similarity",
            summary: "Generalizes small patches in a classified raster by merging them into the most spectrally similar neighboring patch.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input classified raster path.", required: true },
                ToolParamSpec { name: "similarity", description: "Array of similarity rasters used to compute feature-center similarity.", required: true },
                ToolParamSpec { name: "min_size", description: "Minimum feature size in pixels; smaller features are merged (default 5).", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("classes.tif"));
        defaults.insert("similarity".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        defaults.insert("min_size".to_string(), json!(5));

        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("classes.tif"));
        example.insert("similarity".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        example.insert("min_size".to_string(), json!(8));
        example.insert("output".to_string(), json!("generalized_similarity.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta.params.into_iter().map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            }).collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_generalize_with_similarity".to_string(),
                description: "Merge undersized patches into spectrally nearest neighbors.".to_string(),
                args: example,
            }],
            tags: vec!["remote_sensing".to_string(), "classification".to_string(), "generalization".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let sim = parse_raster_list_arg(args, "similarity")?;
        if sim.is_empty() {
            return Err(ToolError::Validation("parameter 'similarity' must contain at least one raster".to_string()));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let sim_paths = parse_raster_list_arg(args, "similarity")?;
        let min_size = args.get("min_size").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(5).max(1);
        let output_path = parse_optional_output_path(args, "output")?;

        let input = FlipImageTool::load_raster(&input_path)?;
        let sims: Vec<Raster> = sim_paths.iter().map(|p| FlipImageTool::load_raster(p)).collect::<Result<_, _>>()?;
        let rows = input.rows as isize;
        let cols = input.cols as isize;
        let n = input.rows * input.cols;
        for (i, s) in sims.iter().enumerate() {
            if s.rows != input.rows || s.cols != input.cols {
                return Err(ToolError::Validation(format!("similarity raster dimensions mismatch at index {}", i)));
            }
        }

        let n8 = [(-1isize, -1isize), (-1, 0), (-1, 1), (0, 1), (1, 1), (1, 0), (1, -1), (0, -1)];
        let mut comp_id = vec![-1isize; n];
        let mut comp_cells: Vec<Vec<usize>> = Vec::new();
        let mut comp_class: Vec<f64> = Vec::new();
        let mut q = VecDeque::<usize>::new();

        for r in 0..rows {
            for c in 0..cols {
                let idx = raster_index(input.cols, r, c);
                if comp_id[idx] >= 0 {
                    continue;
                }
                let z = input.get(0, r, c);
                if input.is_nodata(z) {
                    continue;
                }
                let cid = comp_cells.len() as isize;
                comp_cells.push(Vec::new());
                comp_class.push(z);
                comp_id[idx] = cid;
                q.push_back(idx);
                while let Some(cur) = q.pop_front() {
                    comp_cells[cid as usize].push(cur);
                    let cr = (cur / input.cols) as isize;
                    let cc = (cur % input.cols) as isize;
                    for (dr, dc) in n8 {
                        let nr = cr + dr;
                        let nc = cc + dc;
                        if nr < 0 || nc < 0 || nr >= rows || nc >= cols {
                            continue;
                        }
                        let ni = raster_index(input.cols, nr, nc);
                        if comp_id[ni] >= 0 {
                            continue;
                        }
                        let zn = input.get(0, nr, nc);
                        if !input.is_nodata(zn) && (zn - z).abs() < 1e-12 {
                            comp_id[ni] = cid;
                            q.push_back(ni);
                        }
                    }
                }
            }
        }
        ctx.progress.progress(0.25);

        let dims = sims.len();
        let mut zscores: Vec<Vec<f64>> = Vec::with_capacity(dims);
        for s in &sims {
            let (mean, stdev) = raster_mean_stdev_valid(s);
            let mut arr = vec![f64::NAN; n];
            for r in 0..rows {
                for c in 0..cols {
                    let idx = raster_index(input.cols, r, c);
                    let z = s.get(0, r, c);
                    if !s.is_nodata(z) {
                        arr[idx] = (z - mean) / stdev;
                    }
                }
            }
            zscores.push(arr);
        }

        let mut comp_size: Vec<usize> = comp_cells.iter().map(|v| v.len()).collect();
        let mut comp_center = vec![vec![0.0; dims]; comp_cells.len()];
        let mut comp_center_n = vec![0usize; comp_cells.len()];
        for cid in 0..comp_cells.len() {
            let mut sum = vec![0.0; dims];
            let mut count = 0usize;
            for &idx in &comp_cells[cid] {
                let mut ok = true;
                for d in 0..dims {
                    if !zscores[d][idx].is_finite() {
                        ok = false;
                        break;
                    }
                }
                if ok {
                    for d in 0..dims {
                        sum[d] += zscores[d][idx];
                    }
                    count += 1;
                }
            }
            if count > 0 {
                for d in 0..dims {
                    comp_center[cid][d] = sum[d] / count as f64;
                }
            }
            comp_center_n[cid] = count;
        }

        let mut changed = true;
        while changed {
            changed = false;
            for cid in 0..comp_cells.len() {
                if comp_size[cid] == 0 || comp_size[cid] >= min_size {
                    continue;
                }
                let mut neigh = std::collections::HashSet::<usize>::new();
                for &idx in &comp_cells[cid] {
                    let r = (idx / input.cols) as isize;
                    let c = (idx % input.cols) as isize;
                    for (dr, dc) in n8 {
                        let nr = r + dr;
                        let nc = c + dc;
                        if nr < 0 || nc < 0 || nr >= rows || nc >= cols {
                            continue;
                        }
                        let ni = raster_index(input.cols, nr, nc);
                        let nid = comp_id[ni];
                        if nid >= 0 && nid as usize != cid {
                            neigh.insert(nid as usize);
                        }
                    }
                }
                if neigh.is_empty() {
                    continue;
                }

                let mut best_nid = None;
                let mut best_dist = f64::INFINITY;
                for nid in neigh {
                    if comp_size[nid] == 0 {
                        continue;
                    }
                    let d = if comp_center_n[cid] > 0 && comp_center_n[nid] > 0 {
                        sqr_dist(&comp_center[cid], &comp_center[nid])
                    } else {
                        0.0
                    };
                    if d < best_dist {
                        best_dist = d;
                        best_nid = Some(nid);
                    }
                }

                if let Some(target) = best_nid {
                    let moved = std::mem::take(&mut comp_cells[cid]);
                    for idx in moved {
                        comp_id[idx] = target as isize;
                        comp_cells[target].push(idx);
                    }

                    let n1 = comp_center_n[target] as f64;
                    let n2 = comp_center_n[cid] as f64;
                    if n1 + n2 > 0.0 {
                        for d in 0..dims {
                            comp_center[target][d] = (comp_center[target][d] * n1 + comp_center[cid][d] * n2) / (n1 + n2);
                        }
                    }
                    comp_center_n[target] += comp_center_n[cid];
                    comp_center_n[cid] = 0;
                    comp_size[target] += comp_size[cid];
                    comp_size[cid] = 0;
                    changed = true;
                }
            }
        }

        ctx.progress.progress(0.85);

        let mut output = Raster::new(RasterConfig {
            rows: input.rows,
            cols: input.cols,
            bands: 1,
            x_min: input.x_min,
            y_min: input.y_min,
            cell_size: input.cell_size_x,
            cell_size_y: Some(input.cell_size_y),
            nodata: input.nodata,
            data_type: DataType::I32,
            crs: input.crs.clone(),
            metadata: input.metadata.clone(),
        });
        for r in 0..rows {
            for c in 0..cols {
                let idx = raster_index(input.cols, r, c);
                let cid = comp_id[idx];
                if cid >= 0 {
                    let _ = output.set(0, r, c, comp_class[cid as usize]);
                }
            }
        }

        ctx.progress.progress(1.0);
        let raster_out = FlipImageTool::store_named_raster_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), raster_out);
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for ImageSegmentationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "image_segmentation",
            display_name: "Image Segmentation",
            summary: "Segments multi-band raster stacks into contiguous homogeneous regions using seeded region growing.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Array of single-band input rasters (one per band).", required: true },
                ToolParamSpec { name: "threshold", description: "Distance threshold for region growing in standardized feature space (default 0.5).", required: false },
                ToolParamSpec { name: "steps", description: "Number of seed-priority bands (default 10).", required: false },
                ToolParamSpec { name: "min_area", description: "Optional minimum region area for post-merge cleanup (default 4).", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        defaults.insert("threshold".to_string(), json!(0.5));
        defaults.insert("steps".to_string(), json!(10));
        defaults.insert("min_area".to_string(), json!(4));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        example.insert("threshold".to_string(), json!(0.45));
        example.insert("steps".to_string(), json!(12));
        example.insert("min_area".to_string(), json!(6));
        example.insert("output".to_string(), json!("segments.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta.params.into_iter().map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            }).collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_image_segmentation".to_string(),
                description: "Segment a three-band stack with seeded region growing.".to_string(),
                args: example,
            }],
            tags: vec!["remote_sensing".to_string(), "segmentation".to_string(), "raster".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.is_empty() {
            return Err(ToolError::Validation("parameter 'inputs' must contain at least one raster".to_string()));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = parse_raster_list_arg(args, "inputs")?;
        let threshold = args.get("threshold").and_then(|v| v.as_f64()).unwrap_or(0.5);
        let steps = args.get("steps").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(10).max(1);
        let min_area = args.get("min_area").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(4).max(1);
        let output_path = parse_optional_output_path(args, "output")?;

        let rasters: Vec<Raster> = input_paths
            .iter()
            .map(|p| FlipImageTool::load_raster(p))
            .collect::<Result<_, _>>()?;
        let rows = rasters[0].rows as isize;
        let cols = rasters[0].cols as isize;
        let n = rasters[0].rows * rasters[0].cols;
        for (i, r) in rasters.iter().enumerate() {
            if r.rows as isize != rows || r.cols as isize != cols {
                return Err(ToolError::Validation(format!("input raster dimensions mismatch at index {}", i)));
            }
        }

        let dims = rasters.len();
        let mut zscores: Vec<Vec<f64>> = Vec::with_capacity(dims);
        for r in &rasters {
            let (mean, stdev) = raster_mean_stdev_valid(r);
            let mut arr = vec![f64::NAN; n];
            for row in 0..rows {
                for col in 0..cols {
                    let idx = raster_index(r.cols, row, col);
                    let z = r.get(0, row, col);
                    if !r.is_nodata(z) {
                        arr[idx] = (z - mean) / stdev;
                    }
                }
            }
            zscores.push(arr);
        }
        ctx.progress.progress(0.20);

        let n8 = [(-1isize, -1isize), (-1, 0), (-1, 1), (0, 1), (1, 1), (1, 0), (1, -1), (0, -1)];
        let threshold2 = if threshold <= 0.0 { f64::INFINITY } else { threshold * threshold };
        let mut seed_bins = vec![Vec::<usize>::new(); steps];
        let mut valid = vec![false; n];
        for row in 0..rows {
            for col in 0..cols {
                let idx = raster_index(rasters[0].cols, row, col);
                let mut ok = true;
                for d in 0..dims {
                    if !zscores[d][idx].is_finite() {
                        ok = false;
                        break;
                    }
                }
                if !ok {
                    continue;
                }
                valid[idx] = true;
                let mut local = 0.0;
                let mut k = 0usize;
                for (dr, dc) in n8 {
                    let nr = row + dr;
                    let nc = col + dc;
                    if nr < 0 || nc < 0 || nr >= rows || nc >= cols {
                        continue;
                    }
                    let ni = raster_index(rasters[0].cols, nr, nc);
                    if !valid[ni] {
                        continue;
                    }
                    let mut d2 = 0.0;
                    for d in 0..dims {
                        let dv = zscores[d][idx] - zscores[d][ni];
                        d2 += dv * dv;
                    }
                    local += d2;
                    k += 1;
                }
                let avg = if k > 0 { local / k as f64 } else { f64::INFINITY };
                let mut bin = if threshold2.is_finite() { (avg / threshold2).floor() as isize } else { 0 };
                if bin < 0 {
                    bin = 0;
                }
                let b = (bin as usize).min(steps - 1);
                seed_bins[b].push(idx);
            }
        }

        let mut seg = vec![-1isize; n];
        let mut seg_cells: Vec<Vec<usize>> = Vec::new();
        let mut seg_center: Vec<Vec<f64>> = Vec::new();

        for b in 0..steps {
            for &seed in &seed_bins[b] {
                if !valid[seed] || seg[seed] >= 0 {
                    continue;
                }
                let sid = seg_cells.len() as isize;
                let mut seed_vec = vec![0.0; dims];
                for d in 0..dims {
                    seed_vec[d] = zscores[d][seed];
                }
                let mut stack = vec![seed];
                seg[seed] = sid;
                let mut cells = Vec::<usize>::new();

                while let Some(cur) = stack.pop() {
                    cells.push(cur);
                    let cr = (cur / rasters[0].cols) as isize;
                    let cc = (cur % rasters[0].cols) as isize;
                    for (dr, dc) in n8 {
                        let nr = cr + dr;
                        let nc = cc + dc;
                        if nr < 0 || nc < 0 || nr >= rows || nc >= cols {
                            continue;
                        }
                        let ni = raster_index(rasters[0].cols, nr, nc);
                        if !valid[ni] || seg[ni] >= 0 {
                            continue;
                        }
                        let mut d2 = 0.0;
                        for d in 0..dims {
                            let dv = zscores[d][ni] - seed_vec[d];
                            d2 += dv * dv;
                        }
                        if d2 <= threshold2 {
                            seg[ni] = sid;
                            stack.push(ni);
                        }
                    }
                }

                let mut center = vec![0.0; dims];
                for &idx in &cells {
                    for d in 0..dims {
                        center[d] += zscores[d][idx];
                    }
                }
                if !cells.is_empty() {
                    for d in 0..dims {
                        center[d] /= cells.len() as f64;
                    }
                }

                seg_cells.push(cells);
                seg_center.push(center);
            }
            ctx.progress.progress(0.20 + 0.45 * ((b + 1) as f64 / steps as f64));
        }

        // Fill any remaining valid, unsolved cells by nearest solved neighbor BFS.
        let n4 = [(0isize, 1isize), (1, 0), (0, -1), (-1, 0)];
        let mut bfs = VecDeque::<usize>::new();
        for idx in 0..n {
            if seg[idx] >= 0 {
                bfs.push_back(idx);
            }
        }
        while let Some(cur) = bfs.pop_front() {
            let cr = (cur / rasters[0].cols) as isize;
            let cc = (cur % rasters[0].cols) as isize;
            for (dr, dc) in n4 {
                let nr = cr + dr;
                let nc = cc + dc;
                if nr < 0 || nc < 0 || nr >= rows || nc >= cols {
                    continue;
                }
                let ni = raster_index(rasters[0].cols, nr, nc);
                if valid[ni] && seg[ni] < 0 {
                    seg[ni] = seg[cur];
                    bfs.push_back(ni);
                }
            }
        }

        let mut seg_size = vec![0usize; seg_cells.len()];
        for idx in 0..n {
            let sid = seg[idx];
            if sid >= 0 {
                seg_size[sid as usize] += 1;
            }
        }

        // Merge undersized segments into most similar neighboring segment.
        let mut changed = true;
        while changed {
            changed = false;
            for sid in 0..seg_cells.len() {
                if seg_size[sid] == 0 || seg_size[sid] >= min_area {
                    continue;
                }
                let mut neigh = std::collections::HashSet::<usize>::new();
                for idx in 0..n {
                    if seg[idx] != sid as isize {
                        continue;
                    }
                    let r = (idx / rasters[0].cols) as isize;
                    let c = (idx % rasters[0].cols) as isize;
                    for (dr, dc) in n8 {
                        let nr = r + dr;
                        let nc = c + dc;
                        if nr < 0 || nc < 0 || nr >= rows || nc >= cols {
                            continue;
                        }
                        let ni = raster_index(rasters[0].cols, nr, nc);
                        let ns = seg[ni];
                        if ns >= 0 && ns as usize != sid {
                            neigh.insert(ns as usize);
                        }
                    }
                }
                if neigh.is_empty() {
                    continue;
                }
                let mut best = None;
                let mut best_d = f64::INFINITY;
                for nid in neigh {
                    if seg_size[nid] == 0 {
                        continue;
                    }
                    let d = sqr_dist(&seg_center[sid], &seg_center[nid]);
                    if d < best_d {
                        best_d = d;
                        best = Some(nid);
                    }
                }
                if let Some(nid) = best {
                    let n1 = seg_size[nid] as f64;
                    let n2 = seg_size[sid] as f64;
                    for d in 0..dims {
                        seg_center[nid][d] = (seg_center[nid][d] * n1 + seg_center[sid][d] * n2) / (n1 + n2).max(1.0);
                    }
                    for idx in 0..n {
                        if seg[idx] == sid as isize {
                            seg[idx] = nid as isize;
                        }
                    }
                    seg_size[nid] += seg_size[sid];
                    seg_size[sid] = 0;
                    changed = true;
                }
            }
        }

        ctx.progress.progress(0.90);

        let mut remap = HashMap::<isize, isize>::new();
        let mut next_label = 1isize;
        let mut output = Raster::new(RasterConfig {
            rows: rasters[0].rows,
            cols: rasters[0].cols,
            bands: 1,
            x_min: rasters[0].x_min,
            y_min: rasters[0].y_min,
            cell_size: rasters[0].cell_size_x,
            cell_size_y: Some(rasters[0].cell_size_y),
            nodata: -1.0,
            data_type: DataType::I32,
            crs: rasters[0].crs.clone(),
            metadata: vec![("color_interpretation".to_string(), "categorical".to_string())],
        });

        for row in 0..rows {
            for col in 0..cols {
                let idx = raster_index(rasters[0].cols, row, col);
                if !valid[idx] {
                    continue;
                }
                let sid = seg[idx];
                if sid < 0 {
                    continue;
                }
                let lbl = *remap.entry(sid).or_insert_with(|| {
                    let v = next_label;
                    next_label += 1;
                    v
                });
                let _ = output.set(0, row, col, lbl as f64);
            }
        }

        ctx.progress.progress(1.0);
        let raster_out = FlipImageTool::store_named_raster_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), raster_out);
        Ok(ToolRunResult { outputs })
    }
}

#[derive(Clone, Copy)]
enum ScalingMode {
    None,
    Normalize,
    Standardize,
}

fn parse_scaling_mode(args: &ToolArgs) -> ScalingMode {
    let raw = args
        .get("scaling")
        .or_else(|| args.get("scaling_method"))
        .and_then(|v| v.as_str())
        .unwrap_or("none")
        .to_ascii_lowercase();
    if raw.contains("norm") {
        ScalingMode::Normalize
    } else if raw.contains("stan") || raw.contains("z") {
        ScalingMode::Standardize
    } else {
        ScalingMode::None
    }
}

fn parse_scaling_mode_str(raw: &str) -> ScalingMode {
    let s = raw.to_ascii_lowercase();
    if s.contains("norm") {
        ScalingMode::Normalize
    } else if s.contains("stan") || s.contains("z") {
        ScalingMode::Standardize
    } else {
        ScalingMode::None
    }
}

fn scaling_mode_name(mode: ScalingMode) -> &'static str {
    match mode {
        ScalingMode::None => "none",
        ScalingMode::Normalize => "normalize",
        ScalingMode::Standardize => "standardize",
    }
}

fn dense_matrix_from_2d(data: &Vec<Vec<f64>>, label: &str) -> Result<DenseMatrix<f64>, ToolError> {
    DenseMatrix::from_2d_vec(data)
        .map_err(|e| ToolError::Execution(format!("failed building dense matrix for {}: {e}", label)))
}

fn raster_min_max_valid(raster: &Raster) -> (f64, f64) {
    let rows = raster.rows as isize;
    let cols = raster.cols as isize;
    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    for row in 0..rows {
        for col in 0..cols {
            let z = raster.get(0, row, col);
            if !raster.is_nodata(z) {
                min_v = min_v.min(z);
                max_v = max_v.max(z);
            }
        }
    }
    if !min_v.is_finite() || !max_v.is_finite() {
        (0.0, 1.0)
    } else {
        (min_v, max_v)
    }
}

fn build_scalers(rasters: &[Raster], mode: ScalingMode) -> Vec<(f64, f64)> {
    rasters
        .iter()
        .map(|r| match mode {
            ScalingMode::None => (0.0, 1.0),
            ScalingMode::Normalize => {
                let (min_v, max_v) = raster_min_max_valid(r);
                let range = (max_v - min_v).abs();
                (min_v, if range < 1e-12 { 1.0 } else { range })
            }
            ScalingMode::Standardize => raster_mean_stdev_valid(r),
        })
        .collect()
}

fn scale_value(mode: ScalingMode, value: f64, offset: f64, scale: f64) -> f64 {
    match mode {
        ScalingMode::None => value,
        ScalingMode::Normalize => (value - offset) / scale,
        ScalingMode::Standardize => (value - offset) / scale,
    }
}

fn sample_scaled_features_at(
    rasters: &[Raster],
    mode: ScalingMode,
    scalers: &[(f64, f64)],
    row: isize,
    col: isize,
) -> Option<Vec<f64>> {
    if row < 0 || col < 0 {
        return None;
    }
    if row >= rasters[0].rows as isize || col >= rasters[0].cols as isize {
        return None;
    }
    let mut feat = vec![0.0; rasters.len()];
    for b in 0..rasters.len() {
        let z = rasters[b].get(0, row, col);
        if rasters[b].is_nodata(z) {
            return None;
        }
        let (offset, scale) = scalers[b];
        feat[b] = scale_value(mode, z, offset, scale);
    }
    Some(feat)
}

fn scale_feature_vec(mode: ScalingMode, scalers: &[(f64, f64)], v: &[f64]) -> Vec<f64> {
    v.iter()
        .enumerate()
        .map(|(i, &z)| {
            let (offset, scale) = scalers[i];
            scale_value(mode, z, offset, scale)
        })
        .collect()
}

fn extract_training_class_samples(
    rasters: &[Raster],
    mode: ScalingMode,
    scalers: &[(f64, f64)],
    layer: &wbvector::Layer,
    field_name: &str,
) -> Result<(Vec<String>, Vec<Vec<f64>>, Vec<usize>), ToolError> {
    let field_idx = layer
        .schema
        .field_index(field_name)
        .ok_or_else(|| ToolError::Validation(format!("field '{}' not found in training data", field_name)))?;

    let mut class_set = std::collections::HashSet::new();
    for f in &layer.features {
        if let Some(v) = f.attributes.get(field_idx) {
            class_set.insert(v.to_string());
        }
    }
    let mut class_names: Vec<String> = class_set.into_iter().collect();
    class_names.sort();
    let mut class_map = HashMap::<String, usize>::new();
    for (i, c) in class_names.iter().enumerate() {
        class_map.insert(c.clone(), i);
    }

    let mut x = Vec::<Vec<f64>>::new();
    let mut y = Vec::<usize>::new();

    for f in &layer.features {
        let class_str = f
            .attributes
            .get(field_idx)
            .map(|v| v.to_string())
            .unwrap_or_default();
        let Some(&class_idx) = class_map.get(&class_str) else {
            continue;
        };
        let Some(geom) = &f.geometry else {
            continue;
        };
        match geom {
            VectorGeometry::Point(c) => {
                if let Some((col, row)) = rasters[0].world_to_pixel(c.x, c.y) {
                    if let Some(feat) = sample_scaled_features_at(rasters, mode, scalers, row, col) {
                        x.push(feat);
                        y.push(class_idx);
                    }
                }
            }
            VectorGeometry::MultiPoint(coords) => {
                for c in coords {
                    if let Some((col, row)) = rasters[0].world_to_pixel(c.x, c.y) {
                        if let Some(feat) = sample_scaled_features_at(rasters, mode, scalers, row, col) {
                            x.push(feat);
                            y.push(class_idx);
                        }
                    }
                }
            }
            VectorGeometry::Polygon { exterior, .. } => {
                let mut raw = Vec::<Vec<f64>>::new();
                scan_rasterize_ring(exterior, &rasters[0], rasters, &mut raw);
                for r in raw {
                    x.push(scale_feature_vec(mode, scalers, &r));
                    y.push(class_idx);
                }
            }
            VectorGeometry::MultiPolygon(parts) => {
                for (ext, _) in parts {
                    let mut raw = Vec::<Vec<f64>>::new();
                    scan_rasterize_ring(ext, &rasters[0], rasters, &mut raw);
                    for r in raw {
                        x.push(scale_feature_vec(mode, scalers, &r));
                        y.push(class_idx);
                    }
                }
            }
            _ => {}
        }
    }

    if x.is_empty() {
        return Err(ToolError::Validation(
            "no valid training samples could be extracted from training_data".to_string(),
        ));
    }

    Ok((class_names, x, y))
}

fn extract_training_regression_samples(
    rasters: &[Raster],
    mode: ScalingMode,
    scalers: &[(f64, f64)],
    layer: &wbvector::Layer,
    field_name: &str,
) -> Result<(Vec<Vec<f64>>, Vec<f64>), ToolError> {
    let field_idx = layer
        .schema
        .field_index(field_name)
        .ok_or_else(|| ToolError::Validation(format!("field '{}' not found in training data", field_name)))?;

    let mut x = Vec::<Vec<f64>>::new();
    let mut y = Vec::<f64>::new();

    for f in &layer.features {
        let Some(attr) = f.attributes.get(field_idx) else {
            continue;
        };
        let Ok(target) = attr.to_string().parse::<f64>() else {
            continue;
        };
        if !target.is_finite() {
            continue;
        }
        let Some(geom) = &f.geometry else {
            continue;
        };
        match geom {
            VectorGeometry::Point(c) => {
                if let Some((col, row)) = rasters[0].world_to_pixel(c.x, c.y) {
                    if let Some(feat) = sample_scaled_features_at(rasters, mode, scalers, row, col) {
                        x.push(feat);
                        y.push(target);
                    }
                }
            }
            VectorGeometry::MultiPoint(coords) => {
                for c in coords {
                    if let Some((col, row)) = rasters[0].world_to_pixel(c.x, c.y) {
                        if let Some(feat) = sample_scaled_features_at(rasters, mode, scalers, row, col) {
                            x.push(feat);
                            y.push(target);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if x.is_empty() {
        return Err(ToolError::Validation(
            "no valid point-based training samples could be extracted for regression".to_string(),
        ));
    }

    Ok((x, y))
}

fn collect_scaled_predictor_rows(
    rasters: &[Raster],
    mode: ScalingMode,
    scalers: &[(f64, f64)],
) -> (Vec<Vec<f64>>, Vec<(isize, isize)>) {
    let rows = rasters[0].rows as isize;
    let cols = rasters[0].cols as isize;
    let mut feats = Vec::<Vec<f64>>::new();
    let mut coords = Vec::<(isize, isize)>::new();

    for row in 0..rows {
        for col in 0..cols {
            if let Some(feat) = sample_scaled_features_at(rasters, mode, scalers, row, col) {
                feats.push(feat);
                coords.push((row, col));
            }
        }
    }

    (feats, coords)
}

fn majority_label(labels: &[usize]) -> usize {
    let mut counts = HashMap::<usize, usize>::new();
    for &l in labels {
        *counts.entry(l).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, c)| *c)
        .map(|(l, _)| l)
        .unwrap_or(0)
}

impl Tool for KnnClassificationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "knn_classification",
            display_name: "kNN Classification",
            summary: "Performs supervised k-nearest-neighbor classification on multi-band input rasters.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Array of single-band input rasters.", required: true },
                ToolParamSpec { name: "training_data", description: "Point/polygon vector training data path.", required: true },
                ToolParamSpec { name: "class_field", description: "Class field in training_data attributes.", required: true },
                ToolParamSpec { name: "scaling", description: "Feature scaling mode: none (default), normalize, standardize.", required: false },
                ToolParamSpec { name: "k", description: "Number of neighbors (default 5).", required: false },
                ToolParamSpec { name: "clip", description: "If true, remove misclassified training samples by leave-one-out pre-pass.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        defaults.insert("training_data".to_string(), json!("training.shp"));
        defaults.insert("class_field".to_string(), json!("class"));
        defaults.insert("scaling".to_string(), json!("none"));
        defaults.insert("k".to_string(), json!(5));
        defaults.insert("clip".to_string(), json!(false));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        example.insert("training_data".to_string(), json!("training.shp"));
        example.insert("class_field".to_string(), json!("class"));
        example.insert("scaling".to_string(), json!("standardize"));
        example.insert("k".to_string(), json!(7));
        example.insert("clip".to_string(), json!(true));
        example.insert("output".to_string(), json!("knn_classified.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta.params.into_iter().map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            }).collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_knn_classification".to_string(),
                description: "Run kNN classification with standardized features and clipping.".to_string(),
                args: example,
            }],
            tags: vec!["remote_sensing".to_string(), "classification".to_string(), "knn".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.is_empty() {
            return Err(ToolError::Validation("parameter 'inputs' must contain at least one raster".to_string()));
        }
        let _ = parse_vector_path_arg(args, "training_data")?;
        let _ = args.get("class_field").and_then(|v| v.as_str()).ok_or_else(|| ToolError::Validation("parameter 'class_field' is required".to_string()))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let paths = parse_raster_list_arg(args, "inputs")?;
        let training_path = parse_vector_path_arg(args, "training_data")?;
        let class_field = args.get("class_field").and_then(|v| v.as_str()).ok_or_else(|| ToolError::Validation("parameter 'class_field' is required".to_string()))?;
        let mode = parse_scaling_mode(args);
        let mut k = args.get("k").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(5).max(1);
        let clip = args.get("clip").and_then(|v| v.as_bool()).unwrap_or(false);
        let output_path = parse_optional_output_path(args, "output")?;

        let rasters: Vec<Raster> = paths.iter().map(|p| FlipImageTool::load_raster(p)).collect::<Result<_, _>>()?;
        for (i, r) in rasters.iter().enumerate() {
            if r.rows != rasters[0].rows || r.cols != rasters[0].cols {
                return Err(ToolError::Validation(format!("input raster dimensions mismatch at index {}", i)));
            }
        }
        let scalers = build_scalers(&rasters, mode);
        let layer = wbvector::read(&training_path).map_err(|e| ToolError::Execution(format!("failed reading training data '{}': {}", training_path, e)))?;
        let (_class_names, mut x_train, mut y_train) = extract_training_class_samples(&rasters, mode, &scalers, &layer, class_field)?;
        if x_train.is_empty() {
            return Err(ToolError::Validation("no training samples extracted".to_string()));
        }
        k = k.min(x_train.len());

        if clip && x_train.len() > 2 {
            let mut tree = KdTree::new(rasters.len());
            for i in 0..x_train.len() {
                tree.add(x_train[i].clone(), i)
                    .map_err(|e| ToolError::Execution(format!("kdtree add failed during clipping: {e}")))?;
            }
            let mut keep = vec![true; x_train.len()];
            for i in 0..x_train.len() {
                let ret = tree
                    .nearest(&x_train[i], (k + 1).min(x_train.len()), &squared_euclidean)
                    .map_err(|e| ToolError::Execution(format!("kdtree query failed during clipping: {e}")))?;
                let mut neigh_labels = Vec::<usize>::new();
                for (_d, idx_ref) in ret {
                    let idx = *idx_ref;
                    if idx != i {
                        neigh_labels.push(y_train[idx]);
                    }
                    if neigh_labels.len() == k {
                        break;
                    }
                }
                if !neigh_labels.is_empty() {
                    let pred = majority_label(&neigh_labels);
                    keep[i] = pred == y_train[i];
                }
            }
            let mut x2 = Vec::new();
            let mut y2 = Vec::new();
            for i in 0..x_train.len() {
                if keep[i] {
                    x2.push(x_train[i].clone());
                    y2.push(y_train[i]);
                }
            }
            if !x2.is_empty() {
                x_train = x2;
                y_train = y2;
                k = k.min(x_train.len()).max(1);
            }
        }

        let mut tree = KdTree::new(rasters.len());
        for i in 0..x_train.len() {
            tree.add(x_train[i].clone(), i)
                .map_err(|e| ToolError::Execution(format!("kdtree add failed: {e}")))?;
        }

        let rows = rasters[0].rows as isize;
        let cols = rasters[0].cols as isize;
        let mut output = Raster::new(RasterConfig {
            rows: rasters[0].rows,
            cols: rasters[0].cols,
            bands: 1,
            x_min: rasters[0].x_min,
            y_min: rasters[0].y_min,
            cell_size: rasters[0].cell_size_x,
            cell_size_y: Some(rasters[0].cell_size_y),
            nodata: -32768.0,
            data_type: DataType::I16,
            crs: rasters[0].crs.clone(),
            metadata: vec![("color_interpretation".to_string(), "categorical".to_string())],
        });

        for row in 0..rows {
            for col in 0..cols {
                let Some(feat) = sample_scaled_features_at(&rasters, mode, &scalers, row, col) else {
                    continue;
                };
                let ret = tree
                    .nearest(&feat, k, &squared_euclidean)
                    .map_err(|e| ToolError::Execution(format!("kdtree query failed: {e}")))?;
                if ret.is_empty() {
                    continue;
                }
                let mut neigh_labels = Vec::<usize>::with_capacity(ret.len());
                for (_d, idx_ref) in ret {
                    neigh_labels.push(y_train[*idx_ref]);
                }
                let pred = majority_label(&neigh_labels);
                let _ = output.set(0, row, col, (pred + 1) as f64);
            }
            if row % 100 == 0 {
                ctx.progress.progress((row as f64 / rows as f64).clamp(0.0, 1.0));
            }
        }
        ctx.progress.progress(1.0);

        let raster_out = FlipImageTool::store_named_raster_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), raster_out);
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for KnnRegressionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "knn_regression",
            display_name: "kNN Regression",
            summary: "Performs supervised k-nearest-neighbor regression on multi-band input rasters.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Array of single-band input rasters.", required: true },
                ToolParamSpec { name: "training_data", description: "Point vector training data path.", required: true },
                ToolParamSpec { name: "field", description: "Numeric target field in training_data attributes.", required: true },
                ToolParamSpec { name: "scaling", description: "Feature scaling mode: none (default), normalize, standardize.", required: false },
                ToolParamSpec { name: "k", description: "Number of neighbors (default 5).", required: false },
                ToolParamSpec { name: "distance_weighted", description: "If true, use inverse-distance weighted averaging.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        defaults.insert("training_data".to_string(), json!("training_points.shp"));
        defaults.insert("field".to_string(), json!("value"));
        defaults.insert("scaling".to_string(), json!("none"));
        defaults.insert("k".to_string(), json!(5));
        defaults.insert("distance_weighted".to_string(), json!(false));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        example.insert("training_data".to_string(), json!("training_points.shp"));
        example.insert("field".to_string(), json!("value"));
        example.insert("scaling".to_string(), json!("standardize"));
        example.insert("k".to_string(), json!(8));
        example.insert("distance_weighted".to_string(), json!(true));
        example.insert("output".to_string(), json!("knn_regression.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta.params.into_iter().map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            }).collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_knn_regression".to_string(),
                description: "Run kNN regression with distance weighting.".to_string(),
                args: example,
            }],
            tags: vec!["remote_sensing".to_string(), "regression".to_string(), "knn".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.is_empty() {
            return Err(ToolError::Validation("parameter 'inputs' must contain at least one raster".to_string()));
        }
        let _ = parse_vector_path_arg(args, "training_data")?;
        let _ = args.get("field").and_then(|v| v.as_str()).ok_or_else(|| ToolError::Validation("parameter 'field' is required".to_string()))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let paths = parse_raster_list_arg(args, "inputs")?;
        let training_path = parse_vector_path_arg(args, "training_data")?;
        let field = args.get("field").and_then(|v| v.as_str()).ok_or_else(|| ToolError::Validation("parameter 'field' is required".to_string()))?;
        let mode = parse_scaling_mode(args);
        let mut k = args.get("k").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(5).max(1);
        let distance_weighted = args.get("distance_weighted").and_then(|v| v.as_bool()).unwrap_or(false);
        let output_path = parse_optional_output_path(args, "output")?;

        let rasters: Vec<Raster> = paths.iter().map(|p| FlipImageTool::load_raster(p)).collect::<Result<_, _>>()?;
        for (i, r) in rasters.iter().enumerate() {
            if r.rows != rasters[0].rows || r.cols != rasters[0].cols {
                return Err(ToolError::Validation(format!("input raster dimensions mismatch at index {}", i)));
            }
        }
        let scalers = build_scalers(&rasters, mode);
        let layer = wbvector::read(&training_path).map_err(|e| ToolError::Execution(format!("failed reading training data '{}': {}", training_path, e)))?;
        let (x_train, y_train) = extract_training_regression_samples(&rasters, mode, &scalers, &layer, field)?;
        k = k.min(x_train.len());

        let mut tree = KdTree::new(rasters.len());
        for i in 0..x_train.len() {
            tree.add(x_train[i].clone(), i)
                .map_err(|e| ToolError::Execution(format!("kdtree add failed: {e}")))?;
        }

        let rows = rasters[0].rows as isize;
        let cols = rasters[0].cols as isize;
        let mut output = Raster::new(RasterConfig {
            rows: rasters[0].rows,
            cols: rasters[0].cols,
            bands: 1,
            x_min: rasters[0].x_min,
            y_min: rasters[0].y_min,
            cell_size: rasters[0].cell_size_x,
            cell_size_y: Some(rasters[0].cell_size_y),
            nodata: -32768.0,
            data_type: DataType::F32,
            crs: rasters[0].crs.clone(),
            metadata: rasters[0].metadata.clone(),
        });

        for row in 0..rows {
            for col in 0..cols {
                let Some(feat) = sample_scaled_features_at(&rasters, mode, &scalers, row, col) else {
                    continue;
                };
                let ret = tree
                    .nearest(&feat, k, &squared_euclidean)
                    .map_err(|e| ToolError::Execution(format!("kdtree query failed: {e}")))?;
                if ret.is_empty() {
                    continue;
                }
                let pred = if distance_weighted {
                    let mut sum_w = 0.0;
                    let mut sum_y = 0.0;
                    for (d2, idx_ref) in ret {
                        let w = 1.0 / d2.max(1e-12);
                        sum_w += w;
                        sum_y += w * y_train[*idx_ref];
                    }
                    if sum_w > 0.0 { sum_y / sum_w } else { output.nodata }
                } else {
                    let mut sum = 0.0;
                    let mut n = 0usize;
                    for (_d2, idx_ref) in ret {
                        sum += y_train[*idx_ref];
                        n += 1;
                    }
                    if n > 0 { sum / n as f64 } else { output.nodata }
                };
                let _ = output.set(0, row, col, pred);
            }
            if row % 100 == 0 {
                ctx.progress.progress((row as f64 / rows as f64).clamp(0.0, 1.0));
            }
        }
        ctx.progress.progress(1.0);

        let raster_out = FlipImageTool::store_named_raster_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), raster_out);
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for FuzzyKnnClassificationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "fuzzy_knn_classification",
            display_name: "Fuzzy kNN Classification",
            summary: "Performs fuzzy k-nearest-neighbor classification and outputs class membership confidence.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Array of single-band input rasters.", required: true },
                ToolParamSpec { name: "training_data", description: "Point/polygon vector training data path.", required: true },
                ToolParamSpec { name: "class_field", description: "Class field in training_data attributes.", required: true },
                ToolParamSpec { name: "scaling", description: "Feature scaling mode: none (default), normalize, standardize.", required: false },
                ToolParamSpec { name: "k", description: "Number of neighbors (default 5).", required: false },
                ToolParamSpec { name: "m", description: "Fuzzy exponent parameter (> 1; default 2.0).", required: false },
                ToolParamSpec { name: "output", description: "Optional output classified raster path.", required: false },
                ToolParamSpec { name: "probability_output", description: "Optional membership-probability raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        defaults.insert("training_data".to_string(), json!("training.shp"));
        defaults.insert("class_field".to_string(), json!("class"));
        defaults.insert("scaling".to_string(), json!("none"));
        defaults.insert("k".to_string(), json!(5));
        defaults.insert("m".to_string(), json!(2.0));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        example.insert("training_data".to_string(), json!("training.shp"));
        example.insert("class_field".to_string(), json!("class"));
        example.insert("k".to_string(), json!(7));
        example.insert("m".to_string(), json!(2.0));
        example.insert("output".to_string(), json!("fuzzy_knn_classified.tif"));
        example.insert("probability_output".to_string(), json!("fuzzy_knn_probability.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta.params.into_iter().map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            }).collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_fuzzy_knn_classification".to_string(),
                description: "Run fuzzy kNN and output both class and confidence rasters.".to_string(),
                args: example,
            }],
            tags: vec!["remote_sensing".to_string(), "classification".to_string(), "knn".to_string(), "fuzzy".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.is_empty() {
            return Err(ToolError::Validation("parameter 'inputs' must contain at least one raster".to_string()));
        }
        let _ = parse_vector_path_arg(args, "training_data")?;
        let _ = args.get("class_field").and_then(|v| v.as_str()).ok_or_else(|| ToolError::Validation("parameter 'class_field' is required".to_string()))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let paths = parse_raster_list_arg(args, "inputs")?;
        let training_path = parse_vector_path_arg(args, "training_data")?;
        let class_field = args.get("class_field").and_then(|v| v.as_str()).ok_or_else(|| ToolError::Validation("parameter 'class_field' is required".to_string()))?;
        let mode = parse_scaling_mode(args);
        let mut k = args.get("k").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(5).max(1);
        let m = args.get("m").and_then(|v| v.as_f64()).unwrap_or(2.0).max(1.01);
        let output_path = parse_optional_output_path(args, "output")?;
        let prob_output_path = parse_optional_output_path(args, "probability_output")?;

        let rasters: Vec<Raster> = paths.iter().map(|p| FlipImageTool::load_raster(p)).collect::<Result<_, _>>()?;
        for (i, r) in rasters.iter().enumerate() {
            if r.rows != rasters[0].rows || r.cols != rasters[0].cols {
                return Err(ToolError::Validation(format!("input raster dimensions mismatch at index {}", i)));
            }
        }
        let scalers = build_scalers(&rasters, mode);
        let layer = wbvector::read(&training_path).map_err(|e| ToolError::Execution(format!("failed reading training data '{}': {}", training_path, e)))?;
        let (class_names, x_train, y_train) = extract_training_class_samples(&rasters, mode, &scalers, &layer, class_field)?;
        k = k.min(x_train.len());

        let mut tree = KdTree::new(rasters.len());
        for i in 0..x_train.len() {
            tree.add(x_train[i].clone(), i)
                .map_err(|e| ToolError::Execution(format!("kdtree add failed: {e}")))?;
        }

        let rows = rasters[0].rows as isize;
        let cols = rasters[0].cols as isize;
        let mut class_out = Raster::new(RasterConfig {
            rows: rasters[0].rows,
            cols: rasters[0].cols,
            bands: 1,
            x_min: rasters[0].x_min,
            y_min: rasters[0].y_min,
            cell_size: rasters[0].cell_size_x,
            cell_size_y: Some(rasters[0].cell_size_y),
            nodata: -32768.0,
            data_type: DataType::I16,
            crs: rasters[0].crs.clone(),
            metadata: vec![("color_interpretation".to_string(), "categorical".to_string())],
        });
        let mut prob_out = Raster::new(RasterConfig {
            rows: rasters[0].rows,
            cols: rasters[0].cols,
            bands: 1,
            x_min: rasters[0].x_min,
            y_min: rasters[0].y_min,
            cell_size: rasters[0].cell_size_x,
            cell_size_y: Some(rasters[0].cell_size_y),
            nodata: -32768.0,
            data_type: DataType::F32,
            crs: rasters[0].crs.clone(),
            metadata: rasters[0].metadata.clone(),
        });

        let p = 2.0 / (m - 1.0);
        for row in 0..rows {
            for col in 0..cols {
                let Some(feat) = sample_scaled_features_at(&rasters, mode, &scalers, row, col) else {
                    continue;
                };
                let ret = tree
                    .nearest(&feat, k, &squared_euclidean)
                    .map_err(|e| ToolError::Execution(format!("kdtree query failed: {e}")))?;
                if ret.is_empty() {
                    continue;
                }
                let mut memb = vec![0.0; class_names.len()];
                let mut sum_w = 0.0;
                for (d2, idx_ref) in ret {
                    let cls = y_train[*idx_ref];
                    let w = 1.0 / d2.max(1e-12).powf(p / 2.0);
                    memb[cls] += w;
                    sum_w += w;
                }
                if sum_w > 0.0 {
                    for v in &mut memb {
                        *v /= sum_w;
                    }
                }
                let mut best_idx = 0usize;
                let mut best_prob = memb[0];
                for i in 1..memb.len() {
                    if memb[i] > best_prob {
                        best_prob = memb[i];
                        best_idx = i;
                    }
                }
                let _ = class_out.set(0, row, col, (best_idx + 1) as f64);
                let _ = prob_out.set(0, row, col, best_prob);
            }
            if row % 100 == 0 {
                ctx.progress.progress((row as f64 / rows as f64).clamp(0.0, 1.0));
            }
        }
        ctx.progress.progress(1.0);

        let class_ref = FlipImageTool::store_named_raster_output(class_out, output_path)?;
        let prob_ref = FlipImageTool::store_named_raster_output(prob_out, prob_output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), class_ref);
        outputs.insert("probability_output".to_string(), prob_ref);
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for RandomForestClassificationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "random_forest_classification",
            display_name: "Random Forest Classification",
            summary: "Performs supervised random forest classification on multi-band input rasters.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Array of single-band input rasters.", required: true },
                ToolParamSpec { name: "training_data", description: "Point/polygon vector training data path.", required: true },
                ToolParamSpec { name: "class_field", description: "Class field in training_data attributes.", required: true },
                ToolParamSpec { name: "scaling", description: "Feature scaling mode: none (default), normalize, standardize.", required: false },
                ToolParamSpec { name: "n_trees", description: "Number of trees in the forest (default 200).", required: false },
                ToolParamSpec { name: "min_samples_leaf", description: "Minimum number of samples required at a leaf node (default 1).", required: false },
                ToolParamSpec { name: "min_samples_split", description: "Minimum number of samples required to split an internal node (default 2).", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        defaults.insert("training_data".to_string(), json!("training.shp"));
        defaults.insert("class_field".to_string(), json!("class"));
        defaults.insert("scaling".to_string(), json!("none"));
        defaults.insert("n_trees".to_string(), json!(200));
        defaults.insert("min_samples_leaf".to_string(), json!(1));
        defaults.insert("min_samples_split".to_string(), json!(2));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        example.insert("training_data".to_string(), json!("training.shp"));
        example.insert("class_field".to_string(), json!("class"));
        example.insert("scaling".to_string(), json!("standardize"));
        example.insert("n_trees".to_string(), json!(300));
        example.insert("output".to_string(), json!("rf_classification.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta.params.into_iter().map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            }).collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_random_forest_classification".to_string(),
                description: "Run random forest classification on multiband predictors.".to_string(),
                args: example,
            }],
            tags: vec!["remote_sensing".to_string(), "classification".to_string(), "random_forest".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.is_empty() {
            return Err(ToolError::Validation("parameter 'inputs' must contain at least one raster".to_string()));
        }
        let _ = parse_vector_path_arg(args, "training_data")?;
        let _ = args
            .get("class_field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'class_field' is required".to_string()))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let paths = parse_raster_list_arg(args, "inputs")?;
        let training_path = parse_vector_path_arg(args, "training_data")?;
        let class_field = args
            .get("class_field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'class_field' is required".to_string()))?;
        let mode = parse_scaling_mode(args);
        let n_trees = args.get("n_trees").and_then(|v| v.as_u64()).map(|v| v as u16).unwrap_or(200).max(1);
        let min_samples_leaf = args.get("min_samples_leaf").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(1).max(1);
        let min_samples_split = args.get("min_samples_split").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(2).max(2);
        let output_path = parse_optional_output_path(args, "output")?;

        let rasters: Vec<Raster> = paths
            .iter()
            .map(|p| FlipImageTool::load_raster(p))
            .collect::<Result<_, _>>()?;
        for (i, r) in rasters.iter().enumerate() {
            if r.rows != rasters[0].rows || r.cols != rasters[0].cols {
                return Err(ToolError::Validation(format!(
                    "input raster dimensions mismatch at index {}",
                    i
                )));
            }
        }

        let scalers = build_scalers(&rasters, mode);
        let layer = wbvector::read(&training_path).map_err(|e| {
            ToolError::Execution(format!("failed reading training data '{}': {}", training_path, e))
        })?;
        let (_class_names, x_train, y_train_raw) =
            extract_training_class_samples(&rasters, mode, &scalers, &layer, class_field)?;

        if x_train.is_empty() {
            return Err(ToolError::Validation("no training samples extracted".to_string()));
        }

        let y_train: Vec<u32> = y_train_raw.into_iter().map(|v| v as u32).collect();
        let x_train_matrix = dense_matrix_from_2d(&x_train, "training features")?;
        let params = RandomForestClassifierParameters::default()
            .with_n_trees(n_trees)
            .with_min_samples_leaf(min_samples_leaf)
            .with_min_samples_split(min_samples_split);

        let model = RandomForestClassifier::fit(&x_train_matrix, &y_train, params).map_err(|e| {
            ToolError::Execution(format!("random forest classification fit failed: {e}"))
        })?;

        let rows = rasters[0].rows as isize;
        let cols = rasters[0].cols as isize;
        let mut output = Raster::new(RasterConfig {
            rows: rasters[0].rows,
            cols: rasters[0].cols,
            bands: 1,
            x_min: rasters[0].x_min,
            y_min: rasters[0].y_min,
            cell_size: rasters[0].cell_size_x,
            cell_size_y: Some(rasters[0].cell_size_y),
            nodata: -32768.0,
            data_type: DataType::I16,
            crs: rasters[0].crs.clone(),
            metadata: vec![("color_interpretation".to_string(), "categorical".to_string())],
        });

        for row in 0..rows {
            let mut batch_feats: Vec<Vec<f64>> = Vec::new();
            let mut batch_cols: Vec<isize> = Vec::new();
            for col in 0..cols {
                if let Some(feat) = sample_scaled_features_at(&rasters, mode, &scalers, row, col) {
                    batch_feats.push(feat);
                    batch_cols.push(col);
                }
            }

            if !batch_feats.is_empty() {
                let batch = dense_matrix_from_2d(&batch_feats, "prediction batch")?;
                let preds = model.predict(&batch).map_err(|e| {
                    ToolError::Execution(format!("random forest classification predict failed: {e}"))
                })?;
                for (i, pred) in preds.iter().enumerate() {
                    let _ = output.set(0, row, batch_cols[i], (*pred as f64) + 1.0);
                }
            }

            if row % 100 == 0 {
                ctx.progress.progress((row as f64 / rows as f64).clamp(0.0, 1.0));
            }
        }
        ctx.progress.progress(1.0);

        let raster_out = FlipImageTool::store_named_raster_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), raster_out);
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for RandomForestRegressionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "random_forest_regression",
            display_name: "Random Forest Regression",
            summary: "Performs supervised random forest regression on multi-band input rasters.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Array of single-band input rasters.", required: true },
                ToolParamSpec { name: "training_data", description: "Point vector training data path.", required: true },
                ToolParamSpec { name: "field", description: "Numeric target field in training_data attributes.", required: true },
                ToolParamSpec { name: "scaling", description: "Feature scaling mode: none (default), normalize, standardize.", required: false },
                ToolParamSpec { name: "n_trees", description: "Number of trees in the forest (default 200).", required: false },
                ToolParamSpec { name: "min_samples_leaf", description: "Minimum number of samples required at a leaf node (default 1).", required: false },
                ToolParamSpec { name: "min_samples_split", description: "Minimum number of samples required to split an internal node (default 2).", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        defaults.insert("training_data".to_string(), json!("training_points.shp"));
        defaults.insert("field".to_string(), json!("value"));
        defaults.insert("scaling".to_string(), json!("none"));
        defaults.insert("n_trees".to_string(), json!(200));
        defaults.insert("min_samples_leaf".to_string(), json!(1));
        defaults.insert("min_samples_split".to_string(), json!(2));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        example.insert("training_data".to_string(), json!("training_points.shp"));
        example.insert("field".to_string(), json!("target"));
        example.insert("scaling".to_string(), json!("standardize"));
        example.insert("n_trees".to_string(), json!(300));
        example.insert("output".to_string(), json!("rf_regression.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta.params.into_iter().map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            }).collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_random_forest_regression".to_string(),
                description: "Run random forest regression on multiband predictors.".to_string(),
                args: example,
            }],
            tags: vec!["remote_sensing".to_string(), "regression".to_string(), "random_forest".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.is_empty() {
            return Err(ToolError::Validation("parameter 'inputs' must contain at least one raster".to_string()));
        }
        let _ = parse_vector_path_arg(args, "training_data")?;
        let _ = args
            .get("field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'field' is required".to_string()))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let paths = parse_raster_list_arg(args, "inputs")?;
        let training_path = parse_vector_path_arg(args, "training_data")?;
        let field = args
            .get("field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'field' is required".to_string()))?;
        let mode = parse_scaling_mode(args);
        let n_trees = args.get("n_trees").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(200).max(1);
        let min_samples_leaf = args.get("min_samples_leaf").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(1).max(1);
        let min_samples_split = args.get("min_samples_split").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(2).max(2);
        let output_path = parse_optional_output_path(args, "output")?;

        let rasters: Vec<Raster> = paths
            .iter()
            .map(|p| FlipImageTool::load_raster(p))
            .collect::<Result<_, _>>()?;
        for (i, r) in rasters.iter().enumerate() {
            if r.rows != rasters[0].rows || r.cols != rasters[0].cols {
                return Err(ToolError::Validation(format!(
                    "input raster dimensions mismatch at index {}",
                    i
                )));
            }
        }

        let scalers = build_scalers(&rasters, mode);
        let layer = wbvector::read(&training_path).map_err(|e| {
            ToolError::Execution(format!("failed reading training data '{}': {}", training_path, e))
        })?;
        let (x_train, y_train) = extract_training_regression_samples(&rasters, mode, &scalers, &layer, field)?;

        if x_train.is_empty() {
            return Err(ToolError::Validation("no training samples extracted".to_string()));
        }

        let x_train_matrix = dense_matrix_from_2d(&x_train, "training features")?;
        let params = RandomForestRegressorParameters::default()
            .with_n_trees(n_trees)
            .with_min_samples_leaf(min_samples_leaf)
            .with_min_samples_split(min_samples_split);
        let model = RandomForestRegressor::fit(&x_train_matrix, &y_train, params).map_err(|e| {
            ToolError::Execution(format!("random forest regression fit failed: {e}"))
        })?;

        let rows = rasters[0].rows as isize;
        let cols = rasters[0].cols as isize;
        let mut output = Raster::new(RasterConfig {
            rows: rasters[0].rows,
            cols: rasters[0].cols,
            bands: 1,
            x_min: rasters[0].x_min,
            y_min: rasters[0].y_min,
            cell_size: rasters[0].cell_size_x,
            cell_size_y: Some(rasters[0].cell_size_y),
            nodata: -32768.0,
            data_type: DataType::F32,
            crs: rasters[0].crs.clone(),
            metadata: rasters[0].metadata.clone(),
        });

        for row in 0..rows {
            let mut batch_feats: Vec<Vec<f64>> = Vec::new();
            let mut batch_cols: Vec<isize> = Vec::new();
            for col in 0..cols {
                if let Some(feat) = sample_scaled_features_at(&rasters, mode, &scalers, row, col) {
                    batch_feats.push(feat);
                    batch_cols.push(col);
                }
            }

            if !batch_feats.is_empty() {
                let batch = dense_matrix_from_2d(&batch_feats, "prediction batch")?;
                let preds = model.predict(&batch).map_err(|e| {
                    ToolError::Execution(format!("random forest regression predict failed: {e}"))
                })?;
                for (i, pred) in preds.iter().enumerate() {
                    let _ = output.set(0, row, batch_cols[i], *pred);
                }
            }

            if row % 100 == 0 {
                ctx.progress.progress((row as f64 / rows as f64).clamp(0.0, 1.0));
            }
        }
        ctx.progress.progress(1.0);

        let raster_out = FlipImageTool::store_named_raster_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), raster_out);
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for RandomForestClassificationFitTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "random_forest_classification_fit",
            display_name: "Random Forest Classification Fit",
            summary: "Fits a random forest classification model and returns serialized model bytes.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Array of single-band input rasters.", required: true },
                ToolParamSpec { name: "training_data", description: "Point/polygon vector training data path.", required: true },
                ToolParamSpec { name: "class_field", description: "Class field in training_data attributes.", required: true },
                ToolParamSpec { name: "scaling", description: "Feature scaling mode: none (default), normalize, standardize.", required: false },
                ToolParamSpec { name: "split_criterion", description: "Tree split criterion hint (retained for legacy compatibility).", required: false },
                ToolParamSpec { name: "n_trees", description: "Number of trees in the forest (default 200).", required: false },
                ToolParamSpec { name: "min_samples_leaf", description: "Minimum number of samples required at a leaf node (default 1).", required: false },
                ToolParamSpec { name: "min_samples_split", description: "Minimum number of samples required to split an internal node (default 2).", required: false },
                ToolParamSpec { name: "test_proportion", description: "Legacy compatibility argument; reserved for future diagnostics.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.is_empty() {
            return Err(ToolError::Validation("parameter 'inputs' must contain at least one raster".to_string()));
        }
        let _ = parse_vector_path_arg(args, "training_data")?;
        let _ = args
            .get("class_field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'class_field' is required".to_string()))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let paths = parse_raster_list_arg(args, "inputs")?;
        let training_path = parse_vector_path_arg(args, "training_data")?;
        let class_field = args
            .get("class_field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'class_field' is required".to_string()))?;
        let mode = parse_scaling_mode(args);
        let n_trees = args.get("n_trees").and_then(|v| v.as_u64()).map(|v| v as u16).unwrap_or(200).max(1);
        let min_samples_leaf = args.get("min_samples_leaf").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(1).max(1);
        let min_samples_split = args.get("min_samples_split").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(2).max(2);

        let rasters: Vec<Raster> = paths
            .iter()
            .map(|p| FlipImageTool::load_raster(p))
            .collect::<Result<_, _>>()?;
        for (i, r) in rasters.iter().enumerate() {
            if r.rows != rasters[0].rows || r.cols != rasters[0].cols {
                return Err(ToolError::Validation(format!("input raster dimensions mismatch at index {}", i)));
            }
        }

        let scalers = build_scalers(&rasters, mode);
        let layer = wbvector::read(&training_path).map_err(|e| {
            ToolError::Execution(format!("failed reading training data '{}': {}", training_path, e))
        })?;
        let (_class_names, x_train, y_train_raw) =
            extract_training_class_samples(&rasters, mode, &scalers, &layer, class_field)?;

        if x_train.is_empty() {
            return Err(ToolError::Validation("no training samples extracted".to_string()));
        }
        let y_train: Vec<u32> = y_train_raw.into_iter().map(|v| v as u32).collect();

        let mut payload = serde_json::Map::new();
        payload.insert("kind".to_string(), json!("rf_classification_v1"));
        payload.insert("scaling".to_string(), json!(scaling_mode_name(mode)));
        payload.insert("n_trees".to_string(), json!(n_trees));
        payload.insert("min_samples_leaf".to_string(), json!(min_samples_leaf));
        payload.insert("min_samples_split".to_string(), json!(min_samples_split));
        payload.insert("x_train".to_string(), json!(x_train));
        payload.insert("y_train".to_string(), json!(y_train));

        let model_bytes = serde_json::to_vec(&serde_json::Value::Object(payload))
            .map_err(|e| ToolError::Execution(format!("failed to serialize model payload: {e}")))?;

        let mut outputs = BTreeMap::new();
        outputs.insert("model_bytes".to_string(), json!(model_bytes));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for RandomForestClassificationPredictTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "random_forest_classification_predict",
            display_name: "Random Forest Classification Predict",
            summary: "Applies a serialized random forest classification model to multi-band predictors.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Array of single-band input rasters.", required: true },
                ToolParamSpec { name: "model_bytes", description: "Model bytes produced by random_forest_classification_fit.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.is_empty() {
            return Err(ToolError::Validation("parameter 'inputs' must contain at least one raster".to_string()));
        }
        let _ = args
            .get("model_bytes")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ToolError::Validation("parameter 'model_bytes' is required and must be a list of bytes".to_string()))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let paths = parse_raster_list_arg(args, "inputs")?;
        let output_path = parse_optional_output_path(args, "output")?;

        let model_bytes_arr = args
            .get("model_bytes")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ToolError::Validation("parameter 'model_bytes' is required and must be a list of bytes".to_string()))?;
        let mut model_bytes = Vec::<u8>::with_capacity(model_bytes_arr.len());
        for v in model_bytes_arr {
            let b = v.as_u64().ok_or_else(|| {
                ToolError::Validation("model_bytes must contain integer values in [0,255]".to_string())
            })?;
            if b > 255 {
                return Err(ToolError::Validation("model_bytes must contain integer values in [0,255]".to_string()));
            }
            model_bytes.push(b as u8);
        }

        let payload: serde_json::Value = serde_json::from_slice(&model_bytes)
            .map_err(|e| ToolError::Validation(format!("failed to parse model_bytes payload: {e}")))?;
        let kind = payload
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if kind != "rf_classification_v1" {
            return Err(ToolError::Validation("model_bytes payload kind is not rf_classification_v1".to_string()));
        }
        let mode = parse_scaling_mode_str(
            payload
                .get("scaling")
                .and_then(|v| v.as_str())
                .unwrap_or("none"),
        );
        let n_trees = payload
            .get("n_trees")
            .and_then(|v| v.as_u64())
            .unwrap_or(200) as u16;
        let min_samples_leaf = payload
            .get("min_samples_leaf")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as usize;
        let min_samples_split = payload
            .get("min_samples_split")
            .and_then(|v| v.as_u64())
            .unwrap_or(2) as usize;

        let x_train: Vec<Vec<f64>> = serde_json::from_value(
            payload
                .get("x_train")
                .cloned()
                .ok_or_else(|| ToolError::Validation("model_bytes payload missing x_train".to_string()))?,
        )
        .map_err(|e| ToolError::Validation(format!("invalid x_train in model payload: {e}")))?;
        let y_train: Vec<u32> = serde_json::from_value(
            payload
                .get("y_train")
                .cloned()
                .ok_or_else(|| ToolError::Validation("model_bytes payload missing y_train".to_string()))?,
        )
        .map_err(|e| ToolError::Validation(format!("invalid y_train in model payload: {e}")))?;

        let x_train_matrix = dense_matrix_from_2d(&x_train, "training features")?;
        let params = RandomForestClassifierParameters::default()
            .with_n_trees(n_trees.max(1))
            .with_min_samples_leaf(min_samples_leaf.max(1))
            .with_min_samples_split(min_samples_split.max(2));
        let model = RandomForestClassifier::fit(&x_train_matrix, &y_train, params)
            .map_err(|e| ToolError::Execution(format!("random forest classification model reconstruction failed: {e}")))?;

        let rasters: Vec<Raster> = paths
            .iter()
            .map(|p| FlipImageTool::load_raster(p))
            .collect::<Result<_, _>>()?;
        for (i, r) in rasters.iter().enumerate() {
            if r.rows != rasters[0].rows || r.cols != rasters[0].cols {
                return Err(ToolError::Validation(format!("input raster dimensions mismatch at index {}", i)));
            }
        }
        let scalers = build_scalers(&rasters, mode);

        let rows = rasters[0].rows as isize;
        let cols = rasters[0].cols as isize;
        let mut output = Raster::new(RasterConfig {
            rows: rasters[0].rows,
            cols: rasters[0].cols,
            bands: 1,
            x_min: rasters[0].x_min,
            y_min: rasters[0].y_min,
            cell_size: rasters[0].cell_size_x,
            cell_size_y: Some(rasters[0].cell_size_y),
            nodata: -32768.0,
            data_type: DataType::I16,
            crs: rasters[0].crs.clone(),
            metadata: vec![("color_interpretation".to_string(), "categorical".to_string())],
        });

        for row in 0..rows {
            let mut batch_feats: Vec<Vec<f64>> = Vec::new();
            let mut batch_cols: Vec<isize> = Vec::new();
            for col in 0..cols {
                if let Some(feat) = sample_scaled_features_at(&rasters, mode, &scalers, row, col) {
                    batch_feats.push(feat);
                    batch_cols.push(col);
                }
            }

            if !batch_feats.is_empty() {
                let batch = dense_matrix_from_2d(&batch_feats, "prediction batch")?;
                let preds = model.predict(&batch).map_err(|e| {
                    ToolError::Execution(format!("random forest classification predict failed: {e}"))
                })?;
                for (i, pred) in preds.iter().enumerate() {
                    let _ = output.set(0, row, batch_cols[i], (*pred as f64) + 1.0);
                }
            }

            if row % 100 == 0 {
                ctx.progress.progress((row as f64 / rows as f64).clamp(0.0, 1.0));
            }
        }
        ctx.progress.progress(1.0);

        let raster_out = FlipImageTool::store_named_raster_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), raster_out);
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for RandomForestRegressionFitTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "random_forest_regression_fit",
            display_name: "Random Forest Regression Fit",
            summary: "Fits a random forest regression model and returns serialized model bytes.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Array of single-band input rasters.", required: true },
                ToolParamSpec { name: "training_data", description: "Point vector training data path.", required: true },
                ToolParamSpec { name: "field", description: "Numeric target field in training_data attributes.", required: true },
                ToolParamSpec { name: "scaling", description: "Feature scaling mode: none (default), normalize, standardize.", required: false },
                ToolParamSpec { name: "n_trees", description: "Number of trees in the forest (default 200).", required: false },
                ToolParamSpec { name: "min_samples_leaf", description: "Minimum number of samples required at a leaf node (default 1).", required: false },
                ToolParamSpec { name: "min_samples_split", description: "Minimum number of samples required to split an internal node (default 2).", required: false },
                ToolParamSpec { name: "test_proportion", description: "Legacy compatibility argument; reserved for future diagnostics.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.is_empty() {
            return Err(ToolError::Validation("parameter 'inputs' must contain at least one raster".to_string()));
        }
        let _ = parse_vector_path_arg(args, "training_data")?;
        let _ = args
            .get("field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'field' is required".to_string()))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let paths = parse_raster_list_arg(args, "inputs")?;
        let training_path = parse_vector_path_arg(args, "training_data")?;
        let field = args
            .get("field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'field' is required".to_string()))?;
        let mode = parse_scaling_mode(args);
        let n_trees = args.get("n_trees").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(200).max(1);
        let min_samples_leaf = args.get("min_samples_leaf").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(1).max(1);
        let min_samples_split = args.get("min_samples_split").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(2).max(2);

        let rasters: Vec<Raster> = paths
            .iter()
            .map(|p| FlipImageTool::load_raster(p))
            .collect::<Result<_, _>>()?;
        for (i, r) in rasters.iter().enumerate() {
            if r.rows != rasters[0].rows || r.cols != rasters[0].cols {
                return Err(ToolError::Validation(format!("input raster dimensions mismatch at index {}", i)));
            }
        }

        let scalers = build_scalers(&rasters, mode);
        let layer = wbvector::read(&training_path).map_err(|e| {
            ToolError::Execution(format!("failed reading training data '{}': {}", training_path, e))
        })?;
        let (x_train, y_train) = extract_training_regression_samples(&rasters, mode, &scalers, &layer, field)?;
        if x_train.is_empty() {
            return Err(ToolError::Validation("no training samples extracted".to_string()));
        }

        let mut payload = serde_json::Map::new();
        payload.insert("kind".to_string(), json!("rf_regression_v1"));
        payload.insert("scaling".to_string(), json!(scaling_mode_name(mode)));
        payload.insert("n_trees".to_string(), json!(n_trees));
        payload.insert("min_samples_leaf".to_string(), json!(min_samples_leaf));
        payload.insert("min_samples_split".to_string(), json!(min_samples_split));
        payload.insert("x_train".to_string(), json!(x_train));
        payload.insert("y_train".to_string(), json!(y_train));

        let model_bytes = serde_json::to_vec(&serde_json::Value::Object(payload))
            .map_err(|e| ToolError::Execution(format!("failed to serialize model payload: {e}")))?;

        let mut outputs = BTreeMap::new();
        outputs.insert("model_bytes".to_string(), json!(model_bytes));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for RandomForestRegressionPredictTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "random_forest_regression_predict",
            display_name: "Random Forest Regression Predict",
            summary: "Applies a serialized random forest regression model to multi-band predictors.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Array of single-band input rasters.", required: true },
                ToolParamSpec { name: "model_bytes", description: "Model bytes produced by random_forest_regression_fit.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.is_empty() {
            return Err(ToolError::Validation("parameter 'inputs' must contain at least one raster".to_string()));
        }
        let _ = args
            .get("model_bytes")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ToolError::Validation("parameter 'model_bytes' is required and must be a list of bytes".to_string()))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let paths = parse_raster_list_arg(args, "inputs")?;
        let output_path = parse_optional_output_path(args, "output")?;

        let model_bytes_arr = args
            .get("model_bytes")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ToolError::Validation("parameter 'model_bytes' is required and must be a list of bytes".to_string()))?;
        let mut model_bytes = Vec::<u8>::with_capacity(model_bytes_arr.len());
        for v in model_bytes_arr {
            let b = v.as_u64().ok_or_else(|| {
                ToolError::Validation("model_bytes must contain integer values in [0,255]".to_string())
            })?;
            if b > 255 {
                return Err(ToolError::Validation("model_bytes must contain integer values in [0,255]".to_string()));
            }
            model_bytes.push(b as u8);
        }

        let payload: serde_json::Value = serde_json::from_slice(&model_bytes)
            .map_err(|e| ToolError::Validation(format!("failed to parse model_bytes payload: {e}")))?;
        let kind = payload
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if kind != "rf_regression_v1" {
            return Err(ToolError::Validation("model_bytes payload kind is not rf_regression_v1".to_string()));
        }
        let mode = parse_scaling_mode_str(
            payload
                .get("scaling")
                .and_then(|v| v.as_str())
                .unwrap_or("none"),
        );
        let n_trees = payload
            .get("n_trees")
            .and_then(|v| v.as_u64())
            .unwrap_or(200) as usize;
        let min_samples_leaf = payload
            .get("min_samples_leaf")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as usize;
        let min_samples_split = payload
            .get("min_samples_split")
            .and_then(|v| v.as_u64())
            .unwrap_or(2) as usize;

        let x_train: Vec<Vec<f64>> = serde_json::from_value(
            payload
                .get("x_train")
                .cloned()
                .ok_or_else(|| ToolError::Validation("model_bytes payload missing x_train".to_string()))?,
        )
        .map_err(|e| ToolError::Validation(format!("invalid x_train in model payload: {e}")))?;
        let y_train: Vec<f64> = serde_json::from_value(
            payload
                .get("y_train")
                .cloned()
                .ok_or_else(|| ToolError::Validation("model_bytes payload missing y_train".to_string()))?,
        )
        .map_err(|e| ToolError::Validation(format!("invalid y_train in model payload: {e}")))?;

        let x_train_matrix = dense_matrix_from_2d(&x_train, "training features")?;
        let params = RandomForestRegressorParameters::default()
            .with_n_trees(n_trees.max(1))
            .with_min_samples_leaf(min_samples_leaf.max(1))
            .with_min_samples_split(min_samples_split.max(2));
        let model = RandomForestRegressor::fit(&x_train_matrix, &y_train, params)
            .map_err(|e| ToolError::Execution(format!("random forest regression model reconstruction failed: {e}")))?;

        let rasters: Vec<Raster> = paths
            .iter()
            .map(|p| FlipImageTool::load_raster(p))
            .collect::<Result<_, _>>()?;
        for (i, r) in rasters.iter().enumerate() {
            if r.rows != rasters[0].rows || r.cols != rasters[0].cols {
                return Err(ToolError::Validation(format!("input raster dimensions mismatch at index {}", i)));
            }
        }
        let scalers = build_scalers(&rasters, mode);

        let rows = rasters[0].rows as isize;
        let cols = rasters[0].cols as isize;
        let mut output = Raster::new(RasterConfig {
            rows: rasters[0].rows,
            cols: rasters[0].cols,
            bands: 1,
            x_min: rasters[0].x_min,
            y_min: rasters[0].y_min,
            cell_size: rasters[0].cell_size_x,
            cell_size_y: Some(rasters[0].cell_size_y),
            nodata: -32768.0,
            data_type: DataType::F32,
            crs: rasters[0].crs.clone(),
            metadata: rasters[0].metadata.clone(),
        });

        for row in 0..rows {
            let mut batch_feats: Vec<Vec<f64>> = Vec::new();
            let mut batch_cols: Vec<isize> = Vec::new();
            for col in 0..cols {
                if let Some(feat) = sample_scaled_features_at(&rasters, mode, &scalers, row, col) {
                    batch_feats.push(feat);
                    batch_cols.push(col);
                }
            }

            if !batch_feats.is_empty() {
                let batch = dense_matrix_from_2d(&batch_feats, "prediction batch")?;
                let preds = model.predict(&batch).map_err(|e| {
                    ToolError::Execution(format!("random forest regression predict failed: {e}"))
                })?;
                for (i, pred) in preds.iter().enumerate() {
                    let _ = output.set(0, row, batch_cols[i], *pred);
                }
            }

            if row % 100 == 0 {
                ctx.progress.progress((row as f64 / rows as f64).clamp(0.0, 1.0));
            }
        }
        ctx.progress.progress(1.0);

        let raster_out = FlipImageTool::store_named_raster_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), raster_out);
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for LogisticRegressionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "logistic_regression",
            display_name: "Logistic Regression",
            summary: "Performs supervised logistic regression classification on multi-band input rasters.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Array of single-band input rasters.", required: true },
                ToolParamSpec { name: "training_data", description: "Point/polygon vector training data path.", required: true },
                ToolParamSpec { name: "class_field", description: "Class field in training_data attributes.", required: true },
                ToolParamSpec { name: "scaling", description: "Feature scaling mode: none (default), normalize, standardize.", required: false },
                ToolParamSpec { name: "alpha", description: "L2 regularization weight (default 0.0).", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        defaults.insert("training_data".to_string(), json!("training.shp"));
        defaults.insert("class_field".to_string(), json!("class"));
        defaults.insert("scaling".to_string(), json!("none"));
        defaults.insert("alpha".to_string(), json!(0.0));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        example.insert("training_data".to_string(), json!("training.shp"));
        example.insert("class_field".to_string(), json!("class"));
        example.insert("scaling".to_string(), json!("standardize"));
        example.insert("alpha".to_string(), json!(0.1));
        example.insert("output".to_string(), json!("logistic_regression.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta.params.into_iter().map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            }).collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_logistic_regression".to_string(),
                description: "Run multinomial logistic regression on multiband predictors.".to_string(),
                args: example,
            }],
            tags: vec!["remote_sensing".to_string(), "classification".to_string(), "logistic_regression".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.is_empty() {
            return Err(ToolError::Validation("parameter 'inputs' must contain at least one raster".to_string()));
        }
        let _ = parse_vector_path_arg(args, "training_data")?;
        let _ = args
            .get("class_field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'class_field' is required".to_string()))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let paths = parse_raster_list_arg(args, "inputs")?;
        let training_path = parse_vector_path_arg(args, "training_data")?;
        let class_field = args
            .get("class_field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'class_field' is required".to_string()))?;
        let mode = parse_scaling_mode(args);
        let alpha = args.get("alpha").and_then(|v| v.as_f64()).unwrap_or(0.0).max(0.0);
        let output_path = parse_optional_output_path(args, "output")?;

        let rasters: Vec<Raster> = paths
            .iter()
            .map(|p| FlipImageTool::load_raster(p))
            .collect::<Result<_, _>>()?;
        for (i, r) in rasters.iter().enumerate() {
            if r.rows != rasters[0].rows || r.cols != rasters[0].cols {
                return Err(ToolError::Validation(format!("input raster dimensions mismatch at index {}", i)));
            }
        }

        let scalers = build_scalers(&rasters, mode);
        let layer = wbvector::read(&training_path)
            .map_err(|e| ToolError::Execution(format!("failed reading training data '{}': {}", training_path, e)))?;
        let (_class_names, x_train, y_train_raw) =
            extract_training_class_samples(&rasters, mode, &scalers, &layer, class_field)?;
        if x_train.is_empty() {
            return Err(ToolError::Validation("no training samples extracted".to_string()));
        }

        let (x_pred, pred_coords) = collect_scaled_predictor_rows(&rasters, mode, &scalers);
        if x_pred.is_empty() {
            return Err(ToolError::Validation("no valid predictor cells available for classification".to_string()));
        }

        let x_train_matrix = dense_matrix_from_2d(&x_train, "training features")?;
        let x_pred_matrix = dense_matrix_from_2d(&x_pred, "prediction features")?;
        let y_train: Vec<u32> = y_train_raw.into_iter().map(|v| v as u32).collect();
        let params = LogisticRegressionParameters::default().with_alpha(alpha);
        let model = LogisticRegression::fit(&x_train_matrix, &y_train, params).map_err(|e| {
            ToolError::Execution(format!("logistic regression fit failed: {e}"))
        })?;

        let preds = model.predict(&x_pred_matrix).map_err(|e| {
            ToolError::Execution(format!("logistic regression predict failed: {e}"))
        })?;

        let mut output = Raster::new(RasterConfig {
            rows: rasters[0].rows,
            cols: rasters[0].cols,
            bands: 1,
            x_min: rasters[0].x_min,
            y_min: rasters[0].y_min,
            cell_size: rasters[0].cell_size_x,
            cell_size_y: Some(rasters[0].cell_size_y),
            nodata: -32768.0,
            data_type: DataType::I16,
            crs: rasters[0].crs.clone(),
            metadata: vec![("color_interpretation".to_string(), "categorical".to_string())],
        });

        for (i, (row, col)) in pred_coords.iter().enumerate() {
            let class_idx = preds[i] as usize;
            let _ = output.set(0, *row, *col, (class_idx + 1) as f64);
        }

        let raster_out = FlipImageTool::store_named_raster_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), raster_out);
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for SvmClassificationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "svm_classification",
            display_name: "SVM Classification",
            summary: "Performs supervised support-vector-machine classification on multi-band input rasters.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Array of single-band input rasters.", required: true },
                ToolParamSpec { name: "training_data", description: "Point/polygon vector training data path.", required: true },
                ToolParamSpec { name: "class_field", description: "Class field in training_data attributes.", required: true },
                ToolParamSpec { name: "scaling", description: "Feature scaling mode: none (default), normalize, standardize.", required: false },
                ToolParamSpec { name: "kernel", description: "Kernel type: linear (default) or rbf.", required: false },
                ToolParamSpec { name: "c", description: "SVM regularization parameter (default 1.0).", required: false },
                ToolParamSpec { name: "gamma", description: "RBF kernel gamma; defaults to 1 / number_of_features.", required: false },
                ToolParamSpec { name: "epoch", description: "Number of training epochs (default 2).", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        defaults.insert("training_data".to_string(), json!("training.shp"));
        defaults.insert("class_field".to_string(), json!("class"));
        defaults.insert("scaling".to_string(), json!("none"));
        defaults.insert("kernel".to_string(), json!("linear"));
        defaults.insert("c".to_string(), json!(1.0));
        defaults.insert("epoch".to_string(), json!(2));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        example.insert("training_data".to_string(), json!("training.shp"));
        example.insert("class_field".to_string(), json!("class"));
        example.insert("scaling".to_string(), json!("standardize"));
        example.insert("kernel".to_string(), json!("rbf"));
        example.insert("c".to_string(), json!(2.0));
        example.insert("gamma".to_string(), json!(0.25));
        example.insert("epoch".to_string(), json!(3));
        example.insert("output".to_string(), json!("svm_classified.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta.params.into_iter().map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            }).collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_svm_classification".to_string(),
                description: "Run one-vs-rest SVM classification on multiband predictors.".to_string(),
                args: example,
            }],
            tags: vec!["remote_sensing".to_string(), "classification".to_string(), "svm".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.is_empty() {
            return Err(ToolError::Validation("parameter 'inputs' must contain at least one raster".to_string()));
        }
        let _ = parse_vector_path_arg(args, "training_data")?;
        let _ = args
            .get("class_field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'class_field' is required".to_string()))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let paths = parse_raster_list_arg(args, "inputs")?;
        let training_path = parse_vector_path_arg(args, "training_data")?;
        let class_field = args
            .get("class_field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'class_field' is required".to_string()))?;
        let mode = parse_scaling_mode(args);
        let kernel_name = args.get("kernel").and_then(|v| v.as_str()).unwrap_or("linear").to_ascii_lowercase();
        let c = args.get("c").and_then(|v| v.as_f64()).unwrap_or(1.0).max(1e-9);
        let epoch = args.get("epoch").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(2).max(1);
        let output_path = parse_optional_output_path(args, "output")?;

        let rasters: Vec<Raster> = paths
            .iter()
            .map(|p| FlipImageTool::load_raster(p))
            .collect::<Result<_, _>>()?;
        for (i, r) in rasters.iter().enumerate() {
            if r.rows != rasters[0].rows || r.cols != rasters[0].cols {
                return Err(ToolError::Validation(format!("input raster dimensions mismatch at index {}", i)));
            }
        }

        let scalers = build_scalers(&rasters, mode);
        let layer = wbvector::read(&training_path)
            .map_err(|e| ToolError::Execution(format!("failed reading training data '{}': {}", training_path, e)))?;
        let (class_names, x_train, y_train_raw) =
            extract_training_class_samples(&rasters, mode, &scalers, &layer, class_field)?;
        if x_train.is_empty() {
            return Err(ToolError::Validation("no training samples extracted".to_string()));
        }

        let (x_pred, pred_coords) = collect_scaled_predictor_rows(&rasters, mode, &scalers);
        if x_pred.is_empty() {
            return Err(ToolError::Validation("no valid predictor cells available for classification".to_string()));
        }

        let n_features = x_train[0].len();
        let gamma = args
            .get("gamma")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0 / (n_features as f64).max(1.0))
            .max(1e-12);

        let x_train_matrix = dense_matrix_from_2d(&x_train, "training features")?;
        let x_pred_matrix = dense_matrix_from_2d(&x_pred, "prediction features")?;

        let mut output = Raster::new(RasterConfig {
            rows: rasters[0].rows,
            cols: rasters[0].cols,
            bands: 1,
            x_min: rasters[0].x_min,
            y_min: rasters[0].y_min,
            cell_size: rasters[0].cell_size_x,
            cell_size_y: Some(rasters[0].cell_size_y),
            nodata: -32768.0,
            data_type: DataType::I16,
            crs: rasters[0].crs.clone(),
            metadata: vec![("color_interpretation".to_string(), "categorical".to_string())],
        });

        if class_names.len() == 2 {
            let y_bin: Vec<i32> = y_train_raw
                .iter()
                .map(|&yy| if yy == 0 { -1 } else { 1 })
                .collect();
            let base_params = SVCParameters::default().with_c(c).with_epoch(epoch);
            let params = if kernel_name == "rbf" {
                base_params.with_kernel(Kernels::rbf().with_gamma(gamma))
            } else {
                base_params.with_kernel(Kernels::linear())
            };
            let model = SVC::fit(&x_train_matrix, &y_bin, &params)
                .map_err(|e| ToolError::Execution(format!("svm classification fit failed: {e}")))?;
            let preds = model
                .predict(&x_pred_matrix)
                .map_err(|e| ToolError::Execution(format!("svm classification predict failed: {e}")))?;

            for (i, (row, col)) in pred_coords.iter().enumerate() {
                let class_idx = if preds[i] > 0.0 { 1usize } else { 0usize };
                let _ = output.set(0, *row, *col, (class_idx + 1) as f64);
            }
        } else {
            let mut votes = vec![vec![0usize; class_names.len()]; x_pred.len()];
            for cls in 0..class_names.len() {
                let y_bin: Vec<i32> = y_train_raw
                    .iter()
                    .map(|&yy| if yy == cls { 1 } else { -1 })
                    .collect();
                let base_params = SVCParameters::default().with_c(c).with_epoch(epoch);
                let params = if kernel_name == "rbf" {
                    base_params.with_kernel(Kernels::rbf().with_gamma(gamma))
                } else {
                    base_params.with_kernel(Kernels::linear())
                };
                let model = SVC::fit(&x_train_matrix, &y_bin, &params).map_err(|e| {
                    ToolError::Execution(format!("svm one-vs-rest fit failed for class {}: {e}", cls + 1))
                })?;
                let preds = model.predict(&x_pred_matrix).map_err(|e| {
                    ToolError::Execution(format!("svm one-vs-rest predict failed for class {}: {e}", cls + 1))
                })?;
                for (i, p) in preds.iter().enumerate() {
                    if *p > 0.0 {
                        votes[i][cls] += 1;
                    }
                }
            }

            for (i, (row, col)) in pred_coords.iter().enumerate() {
                let mut best_idx = 0usize;
                let mut best_vote = votes[i][0];
                for cls in 1..class_names.len() {
                    if votes[i][cls] > best_vote {
                        best_vote = votes[i][cls];
                        best_idx = cls;
                    }
                }
                let _ = output.set(0, *row, *col, (best_idx + 1) as f64);
            }
        }

        let raster_out = FlipImageTool::store_named_raster_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), raster_out);
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for SvmRegressionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "svm_regression",
            display_name: "SVM Regression",
            summary: "Performs supervised support-vector-machine regression on multi-band input rasters.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Array of single-band input rasters.", required: true },
                ToolParamSpec { name: "training_data", description: "Point vector training data path.", required: true },
                ToolParamSpec { name: "field", description: "Numeric target field in training_data attributes.", required: true },
                ToolParamSpec { name: "scaling", description: "Feature scaling mode: none (default), normalize, standardize.", required: false },
                ToolParamSpec { name: "kernel", description: "Kernel type: linear (default) or rbf.", required: false },
                ToolParamSpec { name: "c", description: "SVM regularization parameter (default 1.0).", required: false },
                ToolParamSpec { name: "gamma", description: "RBF kernel gamma; defaults to 1 / number_of_features.", required: false },
                ToolParamSpec { name: "eps", description: "Epsilon-insensitive loss width (default 0.1).", required: false },
                ToolParamSpec { name: "tol", description: "Optimizer tolerance (default 1e-3).", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        defaults.insert("training_data".to_string(), json!("training_points.shp"));
        defaults.insert("field".to_string(), json!("value"));
        defaults.insert("scaling".to_string(), json!("none"));
        defaults.insert("kernel".to_string(), json!("linear"));
        defaults.insert("c".to_string(), json!(1.0));
        defaults.insert("eps".to_string(), json!(0.1));
        defaults.insert("tol".to_string(), json!(1e-3));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        example.insert("training_data".to_string(), json!("training_points.shp"));
        example.insert("field".to_string(), json!("value"));
        example.insert("scaling".to_string(), json!("standardize"));
        example.insert("kernel".to_string(), json!("rbf"));
        example.insert("c".to_string(), json!(2.0));
        example.insert("gamma".to_string(), json!(0.25));
        example.insert("eps".to_string(), json!(0.05));
        example.insert("tol".to_string(), json!(1e-3));
        example.insert("output".to_string(), json!("svm_regression.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta.params.into_iter().map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            }).collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_svm_regression".to_string(),
                description: "Run SVM regression on multiband predictors.".to_string(),
                args: example,
            }],
            tags: vec!["remote_sensing".to_string(), "regression".to_string(), "svm".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.is_empty() {
            return Err(ToolError::Validation("parameter 'inputs' must contain at least one raster".to_string()));
        }
        let _ = parse_vector_path_arg(args, "training_data")?;
        let _ = args
            .get("field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'field' is required".to_string()))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let paths = parse_raster_list_arg(args, "inputs")?;
        let training_path = parse_vector_path_arg(args, "training_data")?;
        let field = args
            .get("field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'field' is required".to_string()))?;
        let mode = parse_scaling_mode(args);
        let kernel_name = args.get("kernel").and_then(|v| v.as_str()).unwrap_or("linear").to_ascii_lowercase();
        let c = args.get("c").and_then(|v| v.as_f64()).unwrap_or(1.0).max(1e-9);
        let eps = args.get("eps").and_then(|v| v.as_f64()).unwrap_or(0.1).max(1e-12);
        let tol = args.get("tol").and_then(|v| v.as_f64()).unwrap_or(1e-3).max(1e-12);
        let output_path = parse_optional_output_path(args, "output")?;

        let rasters: Vec<Raster> = paths
            .iter()
            .map(|p| FlipImageTool::load_raster(p))
            .collect::<Result<_, _>>()?;
        for (i, r) in rasters.iter().enumerate() {
            if r.rows != rasters[0].rows || r.cols != rasters[0].cols {
                return Err(ToolError::Validation(format!("input raster dimensions mismatch at index {}", i)));
            }
        }

        let scalers = build_scalers(&rasters, mode);
        let layer = wbvector::read(&training_path)
            .map_err(|e| ToolError::Execution(format!("failed reading training data '{}': {}", training_path, e)))?;
        let (x_train, y_train) = extract_training_regression_samples(&rasters, mode, &scalers, &layer, field)?;
        if x_train.is_empty() {
            return Err(ToolError::Validation("no training samples extracted".to_string()));
        }

        let (x_pred, pred_coords) = collect_scaled_predictor_rows(&rasters, mode, &scalers);
        if x_pred.is_empty() {
            return Err(ToolError::Validation("no valid predictor cells available for regression".to_string()));
        }

        let n_features = x_train[0].len();
        let gamma = args
            .get("gamma")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0 / (n_features as f64).max(1.0))
            .max(1e-12);

        let x_train_matrix = dense_matrix_from_2d(&x_train, "training features")?;
        let x_pred_matrix = dense_matrix_from_2d(&x_pred, "prediction features")?;

        let base_params = SVRParameters::default()
            .with_c(c)
            .with_eps(eps)
            .with_tol(tol);
        let params = if kernel_name == "rbf" {
            base_params.with_kernel(Kernels::rbf().with_gamma(gamma))
        } else {
            base_params.with_kernel(Kernels::linear())
        };
        let model = SVR::fit(&x_train_matrix, &y_train, &params)
            .map_err(|e| ToolError::Execution(format!("svm regression fit failed: {e}")))?;
        let preds = model
            .predict(&x_pred_matrix)
            .map_err(|e| ToolError::Execution(format!("svm regression predict failed: {e}")))?;

        let mut output = Raster::new(RasterConfig {
            rows: rasters[0].rows,
            cols: rasters[0].cols,
            bands: 1,
            x_min: rasters[0].x_min,
            y_min: rasters[0].y_min,
            cell_size: rasters[0].cell_size_x,
            cell_size_y: Some(rasters[0].cell_size_y),
            nodata: -32768.0,
            data_type: DataType::F32,
            crs: rasters[0].crs.clone(),
            metadata: rasters[0].metadata.clone(),
        });

        for (i, (row, col)) in pred_coords.iter().enumerate() {
            let _ = output.set(0, *row, *col, preds[i]);
        }

        let raster_out = FlipImageTool::store_named_raster_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), raster_out);
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for NndClassificationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "nnd_classification",
            display_name: "NND Classification",
            summary: "Performs nearest-normalized-distance classification with optional outlier rejection.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Array of single-band input rasters.", required: true },
                ToolParamSpec { name: "training_data", description: "Point/polygon vector training data path.", required: true },
                ToolParamSpec { name: "class_field", description: "Class field in training_data attributes.", required: true },
                ToolParamSpec { name: "scaling", description: "Feature scaling mode: none (default), normalize, standardize.", required: false },
                ToolParamSpec { name: "z_threshold", description: "Maximum accepted normalized distance z-score before outlier flagging (default 1.96).", required: false },
                ToolParamSpec { name: "outlier_is_zero", description: "If true, outliers are assigned class value 0; otherwise nodata.", required: false },
                ToolParamSpec { name: "k", description: "Neighborhood size for class-distance estimate (default 25).", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let meta = self.metadata();
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        defaults.insert("training_data".to_string(), json!("training.shp"));
        defaults.insert("class_field".to_string(), json!("class"));
        defaults.insert("scaling".to_string(), json!("none"));
        defaults.insert("z_threshold".to_string(), json!(1.96));
        defaults.insert("outlier_is_zero".to_string(), json!(true));
        defaults.insert("k".to_string(), json!(25));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        example.insert("training_data".to_string(), json!("training.shp"));
        example.insert("class_field".to_string(), json!("class"));
        example.insert("scaling".to_string(), json!("standardize"));
        example.insert("z_threshold".to_string(), json!(2.0));
        example.insert("k".to_string(), json!(25));
        example.insert("output".to_string(), json!("nnd_classified.tif"));

        ToolManifest {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            summary: meta.summary.to_string(),
            category: meta.category,
            license_tier: meta.license_tier,
            params: meta.params.into_iter().map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            }).collect(),
            defaults,
            examples: vec![ToolExample {
                name: "basic_nnd_classification".to_string(),
                description: "Run nearest-normalized-distance classification with outlier thresholding.".to_string(),
                args: example,
            }],
            tags: vec!["remote_sensing".to_string(), "classification".to_string(), "nnd".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.is_empty() {
            return Err(ToolError::Validation("parameter 'inputs' must contain at least one raster".to_string()));
        }
        let _ = parse_vector_path_arg(args, "training_data")?;
        let _ = args.get("class_field").and_then(|v| v.as_str()).ok_or_else(|| ToolError::Validation("parameter 'class_field' is required".to_string()))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let paths = parse_raster_list_arg(args, "inputs")?;
        let training_path = parse_vector_path_arg(args, "training_data")?;
        let class_field = args.get("class_field").and_then(|v| v.as_str()).ok_or_else(|| ToolError::Validation("parameter 'class_field' is required".to_string()))?;
        let mode = parse_scaling_mode(args);
        let z_threshold = args.get("z_threshold").and_then(|v| v.as_f64()).unwrap_or(1.96);
        let outlier_is_zero = args.get("outlier_is_zero").and_then(|v| v.as_bool()).unwrap_or(true);
        let k = args.get("k").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(25).max(1);
        let output_path = parse_optional_output_path(args, "output")?;

        let rasters: Vec<Raster> = paths.iter().map(|p| FlipImageTool::load_raster(p)).collect::<Result<_, _>>()?;
        for (i, r) in rasters.iter().enumerate() {
            if r.rows != rasters[0].rows || r.cols != rasters[0].cols {
                return Err(ToolError::Validation(format!("input raster dimensions mismatch at index {}", i)));
            }
        }
        let scalers = build_scalers(&rasters, mode);
        let layer = wbvector::read(&training_path).map_err(|e| ToolError::Execution(format!("failed reading training data '{}': {}", training_path, e)))?;
        let (class_names, x_train, y_train) = extract_training_class_samples(&rasters, mode, &scalers, &layer, class_field)?;

        let num_classes = class_names.len();
        let mut per_class: Vec<Vec<usize>> = vec![Vec::new(); num_classes];
        for (i, &cls) in y_train.iter().enumerate() {
            per_class[cls].push(i);
        }

        let mut trees: Vec<KdTree<f64, usize, Vec<f64>>> = Vec::with_capacity(num_classes);
        for c in 0..num_classes {
            let mut tree = KdTree::new(rasters.len());
            for &idx in &per_class[c] {
                tree.add(x_train[idx].clone(), idx)
                    .map_err(|e| ToolError::Execution(format!("kdtree add failed: {e}")))?;
            }
            trees.push(tree);
        }

        // Per-class distance normalization stats.
        let mut class_mean = vec![0.0; num_classes];
        let mut class_std = vec![1.0; num_classes];
        for c in 0..num_classes {
            if per_class[c].len() < 2 {
                continue;
            }
            let kk = k.min(per_class[c].len()).max(2);
            let mut dvals = Vec::<f64>::new();
            for &idx in &per_class[c] {
                let ret = trees[c]
                    .nearest(&x_train[idx], kk, &squared_euclidean)
                    .map_err(|e| ToolError::Execution(format!("kdtree query failed: {e}")))?;
                let mut sum = 0.0;
                let mut n = 0usize;
                for (d2, ridx_ref) in ret {
                    if *ridx_ref == idx {
                        continue;
                    }
                    sum += d2.sqrt();
                    n += 1;
                }
                if n > 0 {
                    dvals.push(sum / n as f64);
                }
            }
            if !dvals.is_empty() {
                let mean = dvals.iter().sum::<f64>() / dvals.len() as f64;
                let var = dvals.iter().map(|d| (d - mean) * (d - mean)).sum::<f64>() / dvals.len() as f64;
                class_mean[c] = mean;
                class_std[c] = var.sqrt().max(1e-12);
            }
        }

        let rows = rasters[0].rows as isize;
        let cols = rasters[0].cols as isize;
        let nodata_out = -32768.0;
        let mut output = Raster::new(RasterConfig {
            rows: rasters[0].rows,
            cols: rasters[0].cols,
            bands: 1,
            x_min: rasters[0].x_min,
            y_min: rasters[0].y_min,
            cell_size: rasters[0].cell_size_x,
            cell_size_y: Some(rasters[0].cell_size_y),
            nodata: nodata_out,
            data_type: DataType::I16,
            crs: rasters[0].crs.clone(),
            metadata: vec![("color_interpretation".to_string(), "categorical".to_string())],
        });

        for row in 0..rows {
            for col in 0..cols {
                let Some(feat) = sample_scaled_features_at(&rasters, mode, &scalers, row, col) else {
                    continue;
                };
                let mut best_class = None;
                let mut best_z = f64::INFINITY;
                for c in 0..num_classes {
                    if per_class[c].is_empty() {
                        continue;
                    }
                    let kk = k.min(per_class[c].len()).max(1);
                    let ret = trees[c]
                        .nearest(&feat, kk, &squared_euclidean)
                        .map_err(|e| ToolError::Execution(format!("kdtree query failed: {e}")))?;
                    if ret.is_empty() {
                        continue;
                    }
                    let mut sum = 0.0;
                    for (d2, _) in ret {
                        sum += d2.sqrt();
                    }
                    let d = sum / kk as f64;
                    let z = (d - class_mean[c]) / class_std[c];
                    if z < best_z {
                        best_z = z;
                        best_class = Some(c);
                    }
                }

                if let Some(c) = best_class {
                    if best_z > z_threshold {
                        let ov = if outlier_is_zero { 0.0 } else { nodata_out };
                        let _ = output.set(0, row, col, ov);
                    } else {
                        let _ = output.set(0, row, col, (c + 1) as f64);
                    }
                }
            }
            if row % 100 == 0 {
                ctx.progress.progress((row as f64 / rows as f64).clamp(0.0, 1.0));
            }
        }
        ctx.progress.progress(1.0);

        let raster_out = FlipImageTool::store_named_raster_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), raster_out);
        Ok(ToolRunResult { outputs })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};
    use wbcore::{AllowAllCapabilities, ProgressSink, ToolContext};
    use wbvector::{Layer, VectorFormat};

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

    fn make_raster(rows: usize, cols: usize, bands: usize, vals: &[f64]) -> Raster {
        let mut r = Raster::new(RasterConfig {
            rows,
            cols,
            bands,
            nodata: -9999.0,
            ..Default::default()
        });
        let mut i = 0usize;
        for b in 0..bands as isize {
            for row in 0..rows as isize {
                for col in 0..cols as isize {
                    r.set(b, row, col, vals[i]).unwrap();
                    i += 1;
                }
            }
        }
        r
    }

    fn make_packed_rgb_raster(rows: usize, cols: usize, vals: &[u32]) -> Raster {
        let mut r = Raster::new(RasterConfig {
            rows,
            cols,
            bands: 1,
            nodata: 0.0,
            data_type: DataType::U32,
            ..Default::default()
        });
        r.metadata
            .push(("color_interpretation".to_string(), "packed_rgb".to_string()));
        for row in 0..rows as isize {
            for col in 0..cols as isize {
                let idx = row as usize * cols + col as usize;
                r.set(0, row, col, vals[idx] as f64).unwrap();
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

    fn run_create_composite_with_memory(
        tool: &dyn Tool,
        args: &mut ToolArgs,
        red: Raster,
        green: Raster,
        blue: Raster,
    ) -> Raster {
        let red_id = memory_store::put_raster(red);
        let green_id = memory_store::put_raster(green);
        let blue_id = memory_store::put_raster(blue);
        args.insert(
            "red".to_string(),
            json!(memory_store::make_raster_memory_path(&red_id)),
        );
        args.insert(
            "green".to_string(),
            json!(memory_store::make_raster_memory_path(&green_id)),
        );
        args.insert(
            "blue".to_string(),
            json!(memory_store::make_raster_memory_path(&blue_id)),
        );
        let result = tool.run(args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap().to_string();
        let out_id = memory_store::raster_path_to_id(&out_path).unwrap();
        memory_store::get_raster_by_id(out_id).unwrap()
    }

    fn temp_geojson_path(prefix: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{}_{}_{}.geojson", prefix, std::process::id(), stamp))
    }

    fn write_point_vector(path: &Path, points: &[(f64, f64)]) {
        let mut layer = Layer::new("points");
        for (x, y) in points {
            layer
                .add_feature(Some(VectorGeometry::point(*x, *y)), &[])
                .unwrap();
        }
        wbvector::write(&layer, path, VectorFormat::GeoJson).unwrap();
    }

    #[test]
    fn flip_image_horizontal_flips_columns() {
        let mut args = ToolArgs::new();
        args.insert("direction".to_string(), json!("horizontal"));
        let out = run_with_memory(
            &FlipImageTool,
            &mut args,
            make_raster(2, 3, 1, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
        );
        assert_eq!(out.get(0, 0, 0), 3.0);
        assert_eq!(out.get(0, 0, 2), 1.0);
        assert_eq!(out.get(0, 1, 0), 6.0);
        assert_eq!(out.get(0, 1, 2), 4.0);
    }

    #[test]
    fn balance_contrast_enhancement_preserves_neutral_grey() {
        let mut args = ToolArgs::new();
        args.insert("band_mean".to_string(), json!(100.0));
        let grey = ((255u32 << 24) | (80u32 << 16) | (80u32 << 8) | 80u32) as u32;
        let out = run_with_memory(
            &BalanceContrastEnhancementTool,
            &mut args,
            make_packed_rgb_raster(1, 2, &[grey, grey]),
        );
        let z = out.get(0, 0, 0) as u32;
        assert_eq!(z & 0xFF, (z >> 8) & 0xFF);
        assert_eq!(z & 0xFF, (z >> 16) & 0xFF);
    }

    #[test]
    fn direct_decorrelation_stretch_increases_colour_separation() {
        let mut args = ToolArgs::new();
        args.insert("achromatic_factor".to_string(), json!(0.5));
        args.insert("clip_percent".to_string(), json!(0.0));
        let dull = ((255u32 << 24) | (150u32 << 16) | (140u32 << 8) | 130u32) as u32;
        let out = run_with_memory(
            &DirectDecorrelationStretchTool,
            &mut args,
            make_packed_rgb_raster(1, 1, &[dull]),
        );
        let before = dull;
        let after = out.get(0, 0, 0) as u32;
        let before_range = ((before & 0xFF) as i32 - ((before >> 16) & 0xFF) as i32).unsigned_abs();
        let after_range = ((after & 0xFF) as i32 - ((after >> 16) & 0xFF) as i32).unsigned_abs();
        assert!(after_range >= before_range);
    }

    #[test]
    fn create_colour_composite_builds_packed_rgb() {
        let mut args = ToolArgs::new();
        args.insert("enhance".to_string(), json!(false));
        let out = run_create_composite_with_memory(
            &CreateColourCompositeTool,
            &mut args,
            make_raster(1, 2, 1, &[0.0, 10.0]),
            make_raster(1, 2, 1, &[0.0, 20.0]),
            make_raster(1, 2, 1, &[0.0, 30.0]),
        );
        let z = out.get(0, 0, 1) as u32;
        assert_eq!(z & 0xFF, 255);
        assert_eq!((z >> 8) & 0xFF, 255);
        assert_eq!((z >> 16) & 0xFF, 255);
        assert_eq!((z >> 24) & 0xFF, 255);
    }

    #[test]
    fn opening_removes_single_pixel_spike() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(3));
        args.insert("filter_size_y".to_string(), json!(3));
        let out = run_with_memory(
            &OpeningTool,
            &mut args,
            make_raster(3, 3, 1, &[0.0, 0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 0.0]),
        );
        assert_eq!(out.get(0, 1, 1), 0.0);
    }

    #[test]
    fn closing_fills_single_pixel_hole() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(3));
        args.insert("filter_size_y".to_string(), json!(3));
        let out = run_with_memory(
            &ClosingTool,
            &mut args,
            make_raster(3, 3, 1, &[5.0, 5.0, 5.0, 5.0, 0.0, 5.0, 5.0, 5.0, 5.0]),
        );
        assert_eq!(out.get(0, 1, 1), 5.0);
    }

    #[test]
    fn white_tophat_extracts_spike() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(3));
        args.insert("filter_size_y".to_string(), json!(3));
        args.insert("variant".to_string(), json!("white"));
        let out = run_with_memory(
            &TophatTransformTool,
            &mut args,
            make_raster(3, 3, 1, &[0.0, 0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 0.0]),
        );
        assert_eq!(out.get(0, 1, 1), 10.0);
    }

    #[test]
    fn otsu_thresholding_separates_bimodal_values() {
        let mut args = ToolArgs::new();
        let out = run_with_memory(
            &OtsuThresholdingTool,
            &mut args,
            make_raster(2, 3, 1, &[0.0, 0.0, 0.0, 10.0, 10.0, 10.0]),
        );
        assert_eq!(out.get(0, 0, 0), 0.0);
        assert_eq!(out.get(0, 1, 2), 1.0);
    }

    #[test]
    fn integral_transform_of_ones_accumulates() {
        let mut args = ToolArgs::new();
        let out = run_with_memory(
            &IntegralImageTransformTool,
            &mut args,
            make_raster(3, 3, 1, &[1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]),
        );
        assert_eq!(out.get(0, 0, 0), 1.0);
        assert_eq!(out.get(0, 1, 1), 4.0);
        assert_eq!(out.get(0, 2, 2), 9.0);
    }

    #[test]
    fn ndi_two_band_constant_is_expected_value() {
        let mut args = ToolArgs::new();
        args.insert("band1".to_string(), json!(1));
        args.insert("band2".to_string(), json!(2));
        let vals = vec![5.0; 9]
            .into_iter()
            .chain(vec![3.0; 9].into_iter())
            .collect::<Vec<_>>();
        let out = run_with_memory(&NormalizedDifferenceIndexTool, &mut args, make_raster(3, 3, 2, &vals));
        assert!((out.get(0, 1, 1) - 0.25).abs() < 1e-9);
    }

    #[test]
    fn histogram_equalization_constant_raster_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("num_tones".to_string(), json!(64));
        let out = run_with_memory(
            &HistogramEqualizationTool,
            &mut args,
            make_raster(2, 2, 1, &[7.0, 7.0, 7.0, 7.0]),
        );
        assert_eq!(out.get(0, 0, 0), 7.0);
        assert_eq!(out.get(0, 1, 1), 7.0);
    }

    #[test]
    fn histogram_matching_to_histogram_uses_reference_range() {
        let mut args = ToolArgs::new();
        args.insert(
            "histogram".to_string(),
            json!([[10.0, 0.25], [20.0, 0.5], [30.0, 0.75], [40.0, 1.0]]),
        );
        args.insert("is_cumulative".to_string(), json!(true));
        let out = run_with_memory(
            &HistogramMatchingTool,
            &mut args,
            make_raster(1, 4, 1, &[0.0, 1.0, 2.0, 3.0]),
        );
        assert!(out.get(0, 0, 0) >= 10.0 && out.get(0, 0, 0) <= 40.0);
        assert!(out.get(0, 0, 3) >= out.get(0, 0, 0));
    }

    #[test]
    fn histogram_matching_two_images_tracks_reference_distribution() {
        let mut args = ToolArgs::new();
        let reference = make_raster(1, 4, 1, &[100.0, 110.0, 120.0, 130.0]);
        let reference_id = memory_store::put_raster(reference);
        let reference_path = memory_store::make_raster_memory_path(&reference_id);
        args.insert("reference".to_string(), json!(reference_path));
        let out = run_with_memory(
            &HistogramMatchingTwoImagesTool,
            &mut args,
            make_raster(1, 4, 1, &[1.0, 2.0, 3.0, 4.0]),
        );
        assert!(out.get(0, 0, 0) >= 100.0);
        assert!(out.get(0, 0, 3) <= 130.0);
        assert!(out.get(0, 0, 3) >= out.get(0, 0, 0));
    }

    #[test]
    fn percentage_contrast_stretch_without_clip_maps_to_tone_range() {
        let mut args = ToolArgs::new();
        args.insert("clip".to_string(), json!(0.0));
        args.insert("tail".to_string(), json!("both"));
        args.insert("num_tones".to_string(), json!(101));
        let out = run_with_memory(
            &PercentageContrastStretchTool,
            &mut args,
            make_raster(1, 3, 1, &[0.0, 50.0, 100.0]),
        );
        assert!((out.get(0, 0, 0) - 0.0).abs() < 1e-9);
        assert!((out.get(0, 0, 1) - 50.0).abs() < 1e-9);
        assert!((out.get(0, 0, 2) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn min_max_contrast_stretch_maps_explicit_range() {
        let mut args = ToolArgs::new();
        args.insert("min_val".to_string(), json!(10.0));
        args.insert("max_val".to_string(), json!(20.0));
        args.insert("num_tones".to_string(), json!(101));
        let out = run_with_memory(
            &MinMaxContrastStretchTool,
            &mut args,
            make_raster(1, 3, 1, &[10.0, 15.0, 20.0]),
        );
        assert!((out.get(0, 0, 0) - 0.0).abs() < 1e-9);
        assert!((out.get(0, 0, 1) - 50.0).abs() < 1e-9);
        assert!((out.get(0, 0, 2) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn gaussian_contrast_stretch_stays_within_output_tones() {
        let mut args = ToolArgs::new();
        args.insert("num_tones".to_string(), json!(64));
        let out = run_with_memory(
            &GaussianContrastStretchTool,
            &mut args,
            make_raster(1, 5, 1, &[0.0, 1.0, 2.0, 3.0, 4.0]),
        );
        for c in 0..5 {
            let z = out.get(0, 0, c);
            assert!(z >= 0.0 && z <= 63.0);
        }
    }

    #[test]
    fn sigmoidal_contrast_stretch_midpoint_maps_midrange() {
        let mut args = ToolArgs::new();
        args.insert("cutoff".to_string(), json!(0.5));
        args.insert("gain".to_string(), json!(10.0));
        args.insert("num_tones".to_string(), json!(101));
        let out = run_with_memory(
            &SigmoidalContrastStretchTool,
            &mut args,
            make_raster(1, 3, 1, &[0.0, 50.0, 100.0]),
        );
        let mid = out.get(0, 0, 1);
        assert!(mid > 40.0 && mid < 60.0);
    }

    #[test]
    fn standard_deviation_contrast_stretch_maps_mean_to_midrange() {
        let mut args = ToolArgs::new();
        args.insert("clip".to_string(), json!(1.0));
        args.insert("num_tones".to_string(), json!(101));
        let out = run_with_memory(
            &StandardDeviationContrastStretchTool,
            &mut args,
            make_raster(1, 5, 1, &[0.0, 25.0, 50.0, 75.0, 100.0]),
        );
        let mid = out.get(0, 0, 2);
        assert!(mid > 45.0 && mid < 55.0);
    }

    #[test]
    fn thicken_raster_line_fills_diagonal_leak() {
        let mut args = ToolArgs::new();
        let input = make_raster(3, 3, 1, &[0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0]);
        let mut input_count = 0usize;
        for r in 0..3 {
            for c in 0..3 {
                if input.get(0, r, c) > 0.0 {
                    input_count += 1;
                }
            }
        }
        let out = run_with_memory(
            &ThickenRasterLineTool,
            &mut args,
            input,
        );
        let mut out_count = 0usize;
        for r in 0..3 {
            for c in 0..3 {
                if out.get(0, r, c) > 0.0 {
                    out_count += 1;
                }
            }
        }
        assert!(out_count >= input_count);
    }

    #[test]
    fn line_thinning_reduces_block_to_skeleton() {
        let mut args = ToolArgs::new();
        let out = run_with_memory(
            &LineThinningTool,
            &mut args,
            make_raster(5, 5, 1, &[
                0.0, 0.0, 0.0, 0.0, 0.0, //
                0.0, 1.0, 1.0, 1.0, 0.0, //
                0.0, 1.0, 1.0, 1.0, 0.0, //
                0.0, 1.0, 1.0, 1.0, 0.0, //
                0.0, 0.0, 0.0, 0.0, 0.0,
            ]),
        );
        let mut count = 0usize;
        for r in 0..5 {
            for c in 0..5 {
                if out.get(0, r, c) > 0.0 {
                    count += 1;
                }
            }
        }
        assert!(count < 9);
        assert!(out.get(0, 2, 2) > 0.0);
    }

    #[test]
    fn remove_spurs_prunes_endpoint_branch() {
        let mut args = ToolArgs::new();
        args.insert("max_iterations".to_string(), json!(1));
        let out = run_with_memory(
            &RemoveSpursTool,
            &mut args,
            make_raster(5, 5, 1, &[
                0.0, 0.0, 0.0, 0.0, 0.0, //
                0.0, 0.0, 0.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.0, 0.0, // isolated spur
                0.0, 0.0, 0.0, 0.0, 0.0, //
                0.0, 0.0, 0.0, 0.0, 0.0,
            ]),
        );
        assert_eq!(out.get(0, 2, 2), 0.0);
    }

    #[test]
    fn corner_detection_marks_l_shape_corner() {
        let out = run_with_memory(
            &CornerDetectionTool,
            &mut ToolArgs::new(),
            make_raster(3, 3, 1, &[
                1.0, 1.0, 0.0, //
                1.0, 0.0, 0.0, //
                0.0, 0.0, 0.0,
            ]),
        );
        // The upper-left pixel has east and south neighbours foreground and southeast background.
        assert_eq!(out.get(0, 0, 0), 1.0);
    }

    // ── SplitColourCompositeTool ──────────────────────────────────────────────

    #[test]
    fn split_colour_composite_extracts_bands() {
        // Pack R=100, G=150, B=200
        let packed = (200u32 << 16) | (150u32 << 8) | 100u32;
        let input = make_packed_rgb_raster(1, 1, &[packed]);
        let (red, green, blue) = run_split_colour_composite(&input).unwrap();
        assert!((red.get(0, 0, 0) - 100.0).abs() < 1.0);
        assert!((green.get(0, 0, 0) - 150.0).abs() < 1.0);
        assert!((blue.get(0, 0, 0) - 200.0).abs() < 1.0);
    }

    #[test]
    fn split_colour_composite_tool_run_returns_three_outputs() {
        let packed = (160u32 << 16) | (120u32 << 8) | 80u32;
        let input = make_packed_rgb_raster(1, 1, &[packed]);
        let input_id = memory_store::put_raster(input);
        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&input_id)));
        let ctx = make_ctx();
        let result = SplitColourCompositeTool.run(&args, &ctx).unwrap();
        assert!(result.outputs.contains_key("red"));
        assert!(result.outputs.contains_key("green"));
        assert!(result.outputs.contains_key("blue"));
    }

    // ── RgbToIhsTool ─────────────────────────────────────────────────────────

    #[test]
    fn rgb_to_ihs_composite_grey_has_zero_saturation() {
        // Grey: R=G=B → saturation = 0
        let grey = 128u32;
        let grey_val = grey | (grey << 8) | (grey << 16);
        let composite = make_packed_rgb_raster(1, 1, &[grey_val]);
        let (intensity, _hue, saturation) = run_rgb_to_ihs_from_composite(&composite).unwrap();
        assert!(saturation.get(0, 0, 0).abs() < 1e-6, "grey should be achromatic");
        let expected_i = 128.0 / 255.0;
        assert!((intensity.get(0, 0, 0) - expected_i).abs() < 1e-4);
    }

    #[test]
    fn rgb_to_ihs_bands_round_trip_via_ihs_to_rgb() {
        // 2 rows × 2 cols, 1 band each — distinct non-degenerate colours.
        let r_vals = [200.0f64, 50.0, 100.0, 255.0];
        let g_vals = [100.0f64, 200.0, 150.0,  20.0];
        let b_vals = [ 50.0f64, 100.0,  80.0,  10.0];
        let red_r   = make_raster(2, 2, 1, &r_vals);
        let green_r = make_raster(2, 2, 1, &g_vals);
        let blue_r  = make_raster(2, 2, 1, &b_vals);

        let (intensity, hue, saturation) =
            run_rgb_to_ihs_from_bands(&red_r, &green_r, &blue_r).unwrap();

        // Verify IHS values are in expected ranges
        for r in 0..2isize {
            for c in 0..2isize {
                let i_v = intensity.get(0, r, c);
                let h_v = hue.get(0, r, c);
                let s_v = saturation.get(0, r, c);
                assert!(i_v >= 0.0 && i_v <= 1.0, "intensity out of [0,1] at ({},{}): {}", r, c, i_v);
                assert!(h_v >= 0.0, "hue negative at ({},{}): {}", r, c, h_v);
                assert!(s_v >= 0.0 && s_v <= 1.0, "saturation out of [0,1] at ({},{}): {}", r, c, s_v);
            }
        }

        // Convert back and check values are valid RGB
        let (red_out, green_out, blue_out) =
            run_ihs_to_rgb(&intensity, &hue, &saturation).unwrap();

        for r in 0..2isize {
            for c in 0..2isize {
                let r_v = red_out.get(0, r, c);
                let g_v = green_out.get(0, r, c);
                let b_v = blue_out.get(0, r, c);
                assert!(r_v.is_finite(), "red is non-finite at ({},{}): {}", r, c, r_v);
                assert!(g_v.is_finite(), "green is non-finite at ({},{}): {}", r, c, g_v);
                assert!(b_v.is_finite(), "blue is non-finite at ({},{}): {}", r, c, b_v);
                assert!(r_v >= 0.0 && r_v <= 255.0, "red out of range at ({},{}): {}", r, c, r_v);
                assert!(g_v >= 0.0 && g_v <= 255.0, "green out of range at ({},{}): {}", r, c, g_v);
                assert!(b_v >= 0.0 && b_v <= 255.0, "blue out of range at ({},{}): {}", r, c, b_v);
            }
        }
    }

    // ── IhsToRgbTool ─────────────────────────────────────────────────────────

    #[test]
    fn ihs_to_rgb_tool_run_returns_three_outputs_and_red_dominates() {
        // Fully saturated red: h=0, s=1, i≈0.333
        let intensity  = make_raster(1, 1, 1, &[1.0 / 3.0]);
        let hue        = make_raster(1, 1, 1, &[0.0]);
        let saturation = make_raster(1, 1, 1, &[1.0]);
        let i_id = memory_store::put_raster(intensity);
        let h_id = memory_store::put_raster(hue);
        let s_id = memory_store::put_raster(saturation);
        let mut args = ToolArgs::new();
        args.insert("intensity".to_string(), json!(memory_store::make_raster_memory_path(&i_id)));
        args.insert("hue".to_string(),       json!(memory_store::make_raster_memory_path(&h_id)));
        args.insert("saturation".to_string(),json!(memory_store::make_raster_memory_path(&s_id)));
        let ctx = make_ctx();
        let result = IhsToRgbTool.run(&args, &ctx).unwrap();
        assert!(result.outputs.contains_key("red"));
        assert!(result.outputs.contains_key("green"));
        assert!(result.outputs.contains_key("blue"));
        let red_path = result.outputs["red"].get("path").and_then(|p| p.as_str()).unwrap();
        let red_id = memory_store::raster_path_to_id(red_path).unwrap();
        let red_r = memory_store::get_raster_by_id(red_id).unwrap();
        assert!(red_r.get(0, 0, 0) > 200.0, "expected red-dominant output, got {}", red_r.get(0, 0, 0));
    }

    #[test]
    fn change_vector_analysis_core_returns_expected_magnitude_and_direction() {
        let d1_b1 = make_raster(1, 2, 1, &[1.0, 2.0]);
        let d1_b2 = make_raster(1, 2, 1, &[4.0, 4.0]);
        let d2_b1 = make_raster(1, 2, 1, &[2.0, 1.0]); // +1, -1
        let d2_b2 = make_raster(1, 2, 1, &[3.0, 6.0]); // -1, +2

        let (mag, dir) = run_change_vector_analysis(&[d1_b1, d1_b2], &[d2_b1, d2_b2]).unwrap();

        // Pixel 0: deltas [+1, -1] => magnitude sqrt(2), direction code 1
        assert!((mag.get(0, 0, 0) - 2.0f64.sqrt()).abs() < 1e-5);
        assert_eq!(dir.get(0, 0, 0), 1.0);

        // Pixel 1: deltas [-1, +2] => magnitude sqrt(5), direction code 2
        assert!((mag.get(0, 0, 1) - 5.0f64.sqrt()).abs() < 1e-5);
        assert_eq!(dir.get(0, 0, 1), 2.0);
    }

    #[test]
    fn change_vector_analysis_tool_run_returns_named_outputs() {
        let d1_b1 = make_raster(1, 1, 1, &[1.0]);
        let d1_b2 = make_raster(1, 1, 1, &[4.0]);
        let d2_b1 = make_raster(1, 1, 1, &[3.0]);
        let d2_b2 = make_raster(1, 1, 1, &[3.0]);

        let d1b1_id = memory_store::put_raster(d1_b1);
        let d1b2_id = memory_store::put_raster(d1_b2);
        let d2b1_id = memory_store::put_raster(d2_b1);
        let d2b2_id = memory_store::put_raster(d2_b2);

        let mut args = ToolArgs::new();
        args.insert(
            "date1".to_string(),
            json!([
                memory_store::make_raster_memory_path(&d1b1_id),
                memory_store::make_raster_memory_path(&d1b2_id),
            ]),
        );
        args.insert(
            "date2".to_string(),
            json!([
                memory_store::make_raster_memory_path(&d2b1_id),
                memory_store::make_raster_memory_path(&d2b2_id),
            ]),
        );

        let result = ChangeVectorAnalysisTool.run(&args, &make_ctx()).unwrap();
        assert!(result.outputs.contains_key("magnitude"));
        assert!(result.outputs.contains_key("direction"));
    }

    #[test]
    fn write_function_memory_insertion_two_date_uses_second_for_g_and_b() {
        let input1 = make_raster(1, 1, 1, &[10.0]);
        let input2 = make_raster(1, 1, 1, &[20.0]);
        let out = run_write_function_memory_insertion(&input1, &input2, &input2).unwrap();
        let v = out.get(0, 0, 0) as u32;
        let r = v & 0xFF;
        let g = (v >> 8) & 0xFF;
        let b = (v >> 16) & 0xFF;
        // Single-valued inputs stretch to full range endpoints -> both map to 0.
        assert_eq!(r, 0);
        assert_eq!(g, 0);
        assert_eq!(b, 0);
    }

    #[test]
    fn write_function_memory_insertion_tool_runs_with_three_inputs() {
        let i1 = make_raster(1, 2, 1, &[0.0, 10.0]);
        let i2 = make_raster(1, 2, 1, &[5.0, 15.0]);
        let i3 = make_raster(1, 2, 1, &[8.0, 18.0]);
        let i1_id = memory_store::put_raster(i1);
        let i2_id = memory_store::put_raster(i2);
        let i3_id = memory_store::put_raster(i3);

        let mut args = ToolArgs::new();
        args.insert("input1".to_string(), json!(memory_store::make_raster_memory_path(&i1_id)));
        args.insert("input2".to_string(), json!(memory_store::make_raster_memory_path(&i2_id)));
        args.insert("input3".to_string(), json!(memory_store::make_raster_memory_path(&i3_id)));

        let result = WriteFunctionMemoryInsertionTool.run(&args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap();
        assert!(out_path.starts_with("memory://raster/"));
    }

    #[test]
    fn panchromatic_sharpening_packed_mode_produces_single_band_packed_rgb() {
        let ms = make_packed_rgb_raster(
            2,
            2,
            &[
                (30u32 << 16) | (20u32 << 8) | 10u32,
                (40u32 << 16) | (30u32 << 8) | 20u32,
                (50u32 << 16) | (40u32 << 8) | 30u32,
                (60u32 << 16) | (50u32 << 8) | 40u32,
            ],
        );
        let pan = make_raster(2, 2, 1, &[10.0, 20.0, 30.0, 40.0]);
        let out = run_panchromatic_sharpening(
            &ms,
            &pan,
            PanSharpenMethod::Brovey,
            PanSharpenOutputMode::Packed,
        )
        .unwrap();
        assert_eq!(out.bands, 1);
        assert_eq!(out.data_type, DataType::U32);
        let p = out.get(0, 0, 0);
        assert!(!out.is_nodata(p));
    }

    #[test]
    fn panchromatic_sharpening_bands_mode_produces_three_band_output() {
        let ms = make_packed_rgb_raster(
            1,
            2,
            &[
                (100u32 << 16) | (80u32 << 8) | 60u32,
                (120u32 << 16) | (90u32 << 8) | 70u32,
            ],
        );
        let pan = make_raster(1, 2, 1, &[5.0, 10.0]);
        let out = run_panchromatic_sharpening(
            &ms,
            &pan,
            PanSharpenMethod::Ihs,
            PanSharpenOutputMode::Bands,
        )
        .unwrap();
        assert_eq!(out.bands, 3);
        for b in 0..3isize {
            for c in 0..2isize {
                let v = out.get(b, 0, c);
                assert!(v >= 0.0 && v <= 255.0, "band {} col {} out of range: {}", b, c, v);
            }
        }
    }

    #[test]
    fn mosaic_prefers_last_input_on_overlap() {
        let a = make_raster(2, 2, 1, &[1.0, 1.0, 1.0, 1.0]);
        let b = make_raster(2, 2, 1, &[9.0, 9.0, 9.0, 9.0]);
        let out = run_mosaic(&[a, b], ResampleMethod::Nearest).unwrap();
        assert_eq!(out.get(0, 0, 0), 9.0);
        assert_eq!(out.get(0, 1, 1), 9.0);
    }

    #[test]
    fn resample_with_base_uses_base_grid() {
        let input = make_raster(2, 2, 1, &[1.0, 2.0, 3.0, 4.0]);
        let base = make_raster(3, 4, 1, &[0.0; 12]);
        let out = run_resample(&[input], Some(&base), None, ResampleMethod::Nearest).unwrap();
        assert_eq!(out.rows, 3);
        assert_eq!(out.cols, 4);
    }

    #[test]
    fn resample_tool_runs_with_inputs_and_cell_size() {
        let i1 = make_raster(2, 2, 1, &[1.0, 1.0, 1.0, 1.0]);
        let i2 = make_raster(2, 2, 1, &[5.0, 5.0, 5.0, 5.0]);
        let i1_id = memory_store::put_raster(i1);
        let i2_id = memory_store::put_raster(i2);

        let mut args = ToolArgs::new();
        args.insert(
            "inputs".to_string(),
            json!([
                memory_store::make_raster_memory_path(&i1_id),
                memory_store::make_raster_memory_path(&i2_id)
            ]),
        );
        args.insert("cell_size".to_string(), json!(1.0));
        args.insert("method".to_string(), json!("nn"));

        let result = ResampleTool.run(&args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap();
        let out_id = memory_store::raster_path_to_id(out_path).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        // Last input has precedence where overlap exists.
        assert_eq!(out.get(0, 0, 0), 5.0);
    }

    #[test]
    fn mosaic_tool_runs_and_returns_memory_output() {
        let i1 = make_raster(2, 2, 1, &[2.0, 2.0, 2.0, 2.0]);
        let i2 = make_raster(2, 2, 1, &[7.0, 7.0, 7.0, 7.0]);
        let i1_id = memory_store::put_raster(i1);
        let i2_id = memory_store::put_raster(i2);

        let mut args = ToolArgs::new();
        args.insert(
            "inputs".to_string(),
            json!([
                memory_store::make_raster_memory_path(&i1_id),
                memory_store::make_raster_memory_path(&i2_id)
            ]),
        );
        args.insert("method".to_string(), json!("nn"));

        let result = MosaicTool.run(&args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap();
        assert!(out_path.starts_with("memory://raster/"));
    }

    #[test]
    fn mosaic_with_feathering_blends_overlap_values() {
        let i1 = make_raster(3, 3, 1, &[10.0; 9]);
        let i2 = make_raster(3, 3, 1, &[20.0; 9]);
        let out = run_mosaic_with_feathering(&i1, &i2, ResampleMethod::Nearest, 1.0).unwrap();
        let center = out.get(0, 1, 1);
        assert!((center - 15.0).abs() < 1e-6);
    }

    #[test]
    fn mosaic_with_feathering_packed_rgb_blends_channels() {
        let red = (255u32 << 24) | 255u32;
        let blue = (255u32 << 24) | (255u32 << 16);
        let i1 = make_packed_rgb_raster(3, 3, &[red; 9]);
        let i2 = make_packed_rgb_raster(3, 3, &[blue; 9]);
        let out = run_mosaic_with_feathering(&i1, &i2, ResampleMethod::Nearest, 1.0).unwrap();
        let v = out.get(0, 1, 1);
        let (r, g, b, _) = FlipImageTool::unpack_rgba(v);
        assert!(r > 100 && b > 100);
        assert!(g < 30);
    }

    #[test]
    fn mosaic_with_feathering_tool_runs() {
        let i1 = make_raster(3, 3, 1, &[10.0; 9]);
        let i2 = make_raster(3, 3, 1, &[20.0; 9]);
        let i1_id = memory_store::put_raster(i1);
        let i2_id = memory_store::put_raster(i2);

        let mut args = ToolArgs::new();
        args.insert("input1".to_string(), json!(memory_store::make_raster_memory_path(&i1_id)));
        args.insert("input2".to_string(), json!(memory_store::make_raster_memory_path(&i2_id)));
        args.insert("method".to_string(), json!("cc"));
        args.insert("weight".to_string(), json!(4.0));

        let result = MosaicWithFeatheringTool.run(&args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap();
        assert!(out_path.starts_with("memory://raster/"));
    }

    #[test]
    fn kmeans_core_produces_two_clusters_for_bimodal_data() {
        let b1 = make_raster(2, 2, 1, &[1.0, 1.0, 9.0, 9.0]);
        let b2 = make_raster(2, 2, 1, &[1.0, 1.0, 9.0, 9.0]);
        let result = run_kmeans(
            &[b1, b2],
            KMeansOptions {
                classes: 2,
                max_iterations: 10,
                class_change_threshold: 0.0,
                min_class_size: 1,
                initialize_random: false,
                merge_distance: None,
            },
        )
        .unwrap();
        assert_eq!(result.centroids.len(), 2);
        let c00 = result.raster.get(0, 0, 0);
        let c11 = result.raster.get(0, 1, 1);
        assert!(c00 != result.raster.nodata);
        assert!(c11 != result.raster.nodata);
        assert!(c00 != c11);
    }

    #[test]
    fn modified_kmeans_merges_close_clusters() {
        let b1 = make_raster(2, 2, 1, &[5.0, 5.1, 5.2, 5.3]);
        let b2 = make_raster(2, 2, 1, &[5.0, 5.1, 5.2, 5.3]);
        let result = run_kmeans(
            &[b1, b2],
            KMeansOptions {
                classes: 4,
                max_iterations: 8,
                class_change_threshold: 0.0,
                min_class_size: 1,
                initialize_random: true,
                merge_distance: Some(2.0),
            },
        )
        .unwrap();
        assert!(result.centroids.len() < 4);
        assert!(result.centroids.len() >= 1);
    }

    #[test]
    fn kmeans_tool_runs_and_returns_cluster_count() {
        let b1 = make_raster(2, 2, 1, &[1.0, 1.0, 9.0, 9.0]);
        let b2 = make_raster(2, 2, 1, &[1.0, 1.0, 9.0, 9.0]);
        let b1_id = memory_store::put_raster(b1);
        let b2_id = memory_store::put_raster(b2);
        let mut args = ToolArgs::new();
        args.insert(
            "inputs".to_string(),
            json!([
                memory_store::make_raster_memory_path(&b1_id),
                memory_store::make_raster_memory_path(&b2_id)
            ]),
        );
        args.insert("classes".to_string(), json!(2));
        let result = KMeansClusteringTool.run(&args, &make_ctx()).unwrap();
        assert!(result.outputs.get("num_classes").is_some());
    }

    #[test]
    fn modified_kmeans_tool_runs_and_returns_cluster_count() {
        let b1 = make_raster(2, 2, 1, &[5.0, 5.1, 5.2, 5.3]);
        let b2 = make_raster(2, 2, 1, &[5.0, 5.1, 5.2, 5.3]);
        let b1_id = memory_store::put_raster(b1);
        let b2_id = memory_store::put_raster(b2);
        let mut args = ToolArgs::new();
        args.insert(
            "inputs".to_string(),
            json!([
                memory_store::make_raster_memory_path(&b1_id),
                memory_store::make_raster_memory_path(&b2_id)
            ]),
        );
        args.insert("start_clusters".to_string(), json!(4));
        args.insert("merge_dist".to_string(), json!(2.0));
        let result = ModifiedKMeansClusteringTool.run(&args, &make_ctx()).unwrap();
        assert!(result.outputs.get("num_classes").is_some());
    }

    #[test]
    fn correct_vignetting_preserves_constant_surface_after_rescale() {
        let input = make_raster(3, 3, 1, &[5.0; 9]);
        let out = run_correct_vignetting(&input, 1.0, 1.0, 304.8, 228.6, 4.0).unwrap();
        for r in 0..3isize {
            for c in 0..3isize {
                assert!((out.get(0, r, c) - 5.0).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn correct_vignetting_tool_runs_with_pp_vector() {
        let input = make_raster(3, 3, 1, &[1.0, 2.0, 3.0, 4.0, 8.0, 4.0, 3.0, 2.0, 1.0]);
        let input_id = memory_store::put_raster(input);
        let pp_path = temp_geojson_path("pp");
        write_point_vector(&pp_path, &[(1.0, 1.0)]);
        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&input_id)));
        args.insert("pp".to_string(), json!(pp_path.to_string_lossy().to_string()));
        let result = CorrectVignettingTool.run(&args, &make_ctx()).unwrap();
        assert!(result.outputs.get("path").is_some());
        let _ = std::fs::remove_file(pp_path);
    }

    #[test]
    fn image_stack_profile_extracts_expected_values() {
        let a = make_raster(2, 2, 1, &[1.0, 2.0, 3.0, 4.0]);
        let b = make_raster(2, 2, 1, &[10.0, 20.0, 30.0, 40.0]);
        let profiles = run_image_stack_profile(&[a, b], &[(0, 1), (1, 0)]).unwrap();
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0], vec![2.0, 20.0]);
        assert_eq!(profiles[1], vec![3.0, 30.0]);
    }

    #[test]
    fn image_stack_profile_tool_runs_and_returns_profiles() {
        let a = make_raster(2, 2, 1, &[1.0, 2.0, 3.0, 4.0]);
        let b = make_raster(2, 2, 1, &[10.0, 20.0, 30.0, 40.0]);
        let a_id = memory_store::put_raster(a);
        let b_id = memory_store::put_raster(b);
        let points_path = temp_geojson_path("profile_points");
        write_point_vector(&points_path, &[(1.5, 1.5), (0.5, 0.5)]);
        let mut args = ToolArgs::new();
        args.insert(
            "inputs".to_string(),
            json!([
                memory_store::make_raster_memory_path(&a_id),
                memory_store::make_raster_memory_path(&b_id)
            ]),
        );
        args.insert(
            "points".to_string(),
            json!(points_path.to_string_lossy().to_string()),
        );
        let result = ImageStackProfileTool.run(&args, &make_ctx()).unwrap();
        assert!(result.outputs.get("profiles").is_some());
        let _ = std::fs::remove_file(points_path);
    }

    #[test]
    fn image_stack_profile_html_report_contains_svg_graph() {
        let html_path = temp_geojson_path("stack_profile_report").with_extension("html");
        let inputs = vec!["image1.tif".to_string(), "image2.tif".to_string()];
        let profiles = vec![vec![2.0, 20.0], vec![3.0, 30.0]];
        write_image_stack_profile_html(
            html_path.to_string_lossy().as_ref(),
            &inputs,
            &profiles,
        )
        .unwrap();
        let html = std::fs::read_to_string(&html_path).unwrap();
        assert!(html.contains("<svg"));
        assert!(html.contains("Profile Data Table"));
        let _ = std::fs::remove_file(html_path);
    }

    #[test]
    fn cluster_html_report_contains_svg_convergence_plot() {
        let html_path = temp_geojson_path("kmeans_report").with_extension("html");
        let inputs = vec!["band1.tif".to_string(), "band2.tif".to_string()];
        let result = KMeansRunResult {
            raster: make_raster(1, 1, 1, &[1.0]),
            centroids: vec![vec![1.0, 2.0], vec![3.0, 4.0]],
            counts: vec![5, 7],
            change_history: vec![10.0, 4.5, 1.2],
        };
        write_cluster_html_report(
            html_path.to_string_lossy().as_ref(),
            "k-Means Clustering",
            &inputs,
            &result,
        )
        .unwrap();
        let html = std::fs::read_to_string(&html_path).unwrap();
        assert!(html.contains("Convergence Plot"));
        assert!(html.contains("<svg"));
        let _ = std::fs::remove_file(html_path);
    }
}
