use std::collections::BTreeMap;
use wbcore::ToolParamSchema;

mod raster_add;
mod raster_unary_math;
mod raster_stats;

fn param_schema_map(entries: &[(&str, ToolParamSchema)]) -> BTreeMap<String, ToolParamSchema> {
        let mut map = BTreeMap::new();
        for (name, schema) in entries {
                map.insert((*name).to_string(), schema.clone());
        }
        map
}

pub fn raster_tool_param_schemas(tool_id: &str) -> Option<BTreeMap<String, ToolParamSchema>> {
        match tool_id {
                "abs" | "arccos" | "arcosh" | "arcsin" | "arctan" | "arsinh" | "artanh"
                | "bool_not" | "ceil" | "cos" | "cosh" | "exp" | "exp2" | "floor"
                | "is_nodata" | "ln" | "log10" | "log2" | "negate"
                | "reciprocal" | "round" | "sin" | "sinh" | "sqrt" | "square" | "tan"
                | "tanh" | "to_degrees" | "to_radians" | "truncate" => Some(param_schema_map(&[
                        ("input", ToolParamSchema::input_raster()),
                        ("output", ToolParamSchema::output_raster()),
                ])),
                "decrement" | "increment" => Some(param_schema_map(&[
                        ("input", ToolParamSchema::input_raster()),
                        ("value", ToolParamSchema::scalar_float()),
                        ("output", ToolParamSchema::output_raster()),
                ])),
                "add" | "atan2" | "bool_and" | "bool_or" | "bool_xor" | "divide"
                | "equal_to" | "greater_than" | "greater_than_or_equal_to" | "integer_division"
                | "less_than" | "less_than_or_equal_to" | "modulo" | "multiply" | "not_equal_to"
                | "power" | "subtract" => Some(param_schema_map(&[
                        ("input1", ToolParamSchema::input_raster()),
                        ("input2", ToolParamSchema::input_raster()),
                        ("output", ToolParamSchema::output_raster()),
                ])),
                "inplace_add" | "inplace_subtract" | "inplace_multiply" | "inplace_divide" => {
                        Some(param_schema_map(&[
                                ("input1", ToolParamSchema::input_raster()),
                                (
                                        "input2",
                                        ToolParamSchema::input_existing_or_number(
                                                wbcore::ToolDatasetSchema::Raster,
                                        ),
                                ),
                        ]))
                },
                "raster_summary_stats" => Some(param_schema_map(&[
                        ("input", ToolParamSchema::input_raster()),
                        ("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
                ])),
                "raster_histogram" => Some(param_schema_map(&[
                        ("input", ToolParamSchema::input_raster()),
                        ("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
                        ("bins", ToolParamSchema::scalar_integer()),
                ])),
                "list_unique_values_raster" => Some(param_schema_map(&[
                        ("input", ToolParamSchema::input_raster()),
                        ("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
                        ("strict_parity", ToolParamSchema::bool()),
                        ("max_values", ToolParamSchema::scalar_integer()),
                ])),
                "z_scores" => Some(param_schema_map(&[
                        ("input", ToolParamSchema::input_raster()),
                        ("output", ToolParamSchema::output_raster()),
                ])),
                "rescale_value_range" => Some(param_schema_map(&[
                        ("input", ToolParamSchema::input_raster()),
                        ("out_min", ToolParamSchema::scalar_float()),
                        ("out_max", ToolParamSchema::scalar_float()),
                        ("clip_min", ToolParamSchema::scalar_float()),
                        ("clip_max", ToolParamSchema::scalar_float()),
                        ("output", ToolParamSchema::output_raster()),
                ])),
                "ks_normality_test" => Some(param_schema_map(&[
                        ("input", ToolParamSchema::input_raster()),
                        ("num_samples", ToolParamSchema::scalar_integer()),
                        ("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
                ])),
                "cumulative_distribution" => Some(param_schema_map(&[
                        ("input", ToolParamSchema::input_raster()),
                        ("output", ToolParamSchema::output_raster()),
                ])),
                "random_field" => Some(param_schema_map(&[
                        ("base", ToolParamSchema::input_raster()),
                        ("output", ToolParamSchema::output_raster()),
                ])),
                "fft_random_field" => Some(param_schema_map(&[
                        ("base_raster", ToolParamSchema::input_raster()),
                        ("range", ToolParamSchema::scalar_float()),
                        ("output", ToolParamSchema::output_raster()),
                ])),
                "random_sample" => Some(param_schema_map(&[
                        ("base", ToolParamSchema::input_raster()),
                        ("num_samples", ToolParamSchema::scalar_integer()),
                        ("output", ToolParamSchema::output_raster()),
                ])),
                "attribute_histogram" => Some(param_schema_map(&[
                        ("input", ToolParamSchema::input_vector_any()),
                        ("field", ToolParamSchema::string()),
                        ("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
                ])),
                "attribute_scattergram" => Some(param_schema_map(&[
                        ("input", ToolParamSchema::input_vector_any()),
                        ("fieldx", ToolParamSchema::string()),
                        ("fieldy", ToolParamSchema::string()),
                        ("trendline", ToolParamSchema::bool()),
                        ("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
                ])),
                "attribute_correlation" => Some(param_schema_map(&[
                        ("input", ToolParamSchema::input_vector_any()),
                        ("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
                ])),
                "cross_tabulation" => Some(param_schema_map(&[
                        ("input1", ToolParamSchema::input_raster()),
                        ("input2", ToolParamSchema::input_raster()),
                        ("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
                ])),
                "anova" => Some(param_schema_map(&[
                        ("input", ToolParamSchema::input_raster()),
                        ("features", ToolParamSchema::input_raster()),
                        ("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
                ])),
                "crispness_index" => Some(param_schema_map(&[
                        ("input", ToolParamSchema::input_raster()),
                        ("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
                ])),
                "phi_coefficient" => Some(param_schema_map(&[
                        ("input1", ToolParamSchema::input_raster()),
                        ("input2", ToolParamSchema::input_raster()),
                        ("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
                ])),
                "max" | "min" => Some(param_schema_map(&[
                        (
                                "input1",
                                ToolParamSchema::input_existing_or_number(wbcore::ToolDatasetSchema::Raster),
                        ),
                        (
                                "input2",
                                ToolParamSchema::input_existing_or_number(wbcore::ToolDatasetSchema::Raster),
                        ),
                        ("output", ToolParamSchema::output_raster()),
                ])),
                "quantiles" => Some(param_schema_map(&[
                        ("input", ToolParamSchema::input_raster()),
                        ("num_quantiles", ToolParamSchema::scalar_integer()),
                        ("output", ToolParamSchema::output_raster()),
                ])),
                "list_unique_values" => Some(param_schema_map(&[
                        ("input", ToolParamSchema::input_vector_any()),
                        ("field", ToolParamSchema::string()),
                        ("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
                ])),
                "root_mean_square_error" => Some(param_schema_map(&[
                        ("input", ToolParamSchema::input_raster()),
                        ("base", ToolParamSchema::input_raster()),
                ])),
                "zonal_statistics" => Some(param_schema_map(&[
                        ("input", ToolParamSchema::input_raster()),
                        ("features", ToolParamSchema::input_raster()),
                        ("stat_type", ToolParamSchema::string()),
                        ("zero_is_background", ToolParamSchema::bool()),
                        ("output", ToolParamSchema::output_raster()),
                ])),
                "conditional_evaluation" => Some(param_schema_map(&[
                        ("input", ToolParamSchema::input_raster()),
                        ("statement", ToolParamSchema::string()),
                        ("true", ToolParamSchema::string()),
                        ("false", ToolParamSchema::string()),
                        ("output", ToolParamSchema::output_raster()),
                ])),
                "image_correlation" => Some(param_schema_map(&[
                        (
                                "inputs",
                                ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
                        ),
                ])),
                "image_autocorrelation" => Some(param_schema_map(&[
                        (
                                "inputs",
                                ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
                        ),
                        ("contiguity", ToolParamSchema::string()),
                ])),
                "image_correlation_neighbourhood_analysis" => Some(param_schema_map(&[
                        ("input1", ToolParamSchema::input_raster()),
                        ("input2", ToolParamSchema::input_raster()),
                        ("filter_size", ToolParamSchema::scalar_integer()),
                        ("correlation_stat", ToolParamSchema::string()),
                        ("output1", ToolParamSchema::output_raster()),
                        ("output2", ToolParamSchema::output_raster()),
                ])),
                "image_regression" => Some(param_schema_map(&[
                        ("input1", ToolParamSchema::input_raster()),
                        ("input2", ToolParamSchema::input_raster()),
                        ("standardize_residuals", ToolParamSchema::bool()),
                        ("output", ToolParamSchema::output_raster()),
                ])),
                "dbscan" => Some(param_schema_map(&[
                        (
                                "inputs",
                                ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
                        ),
                        ("scaling_method", ToolParamSchema::string()),
                        ("search_distance", ToolParamSchema::scalar_float()),
                        ("min_points", ToolParamSchema::scalar_integer()),
                        ("output", ToolParamSchema::output_raster()),
                ])),
                "kappa_index" => Some(param_schema_map(&[
                        ("input1", ToolParamSchema::input_raster()),
                        ("input2", ToolParamSchema::input_raster()),
                        ("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
                ])),
                "paired_sample_t_test" => Some(param_schema_map(&[
                        ("input1", ToolParamSchema::input_raster()),
                        ("input2", ToolParamSchema::input_raster()),
                        ("num_samples", ToolParamSchema::scalar_integer()),
                        ("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
                ])),
                "two_sample_ks_test" | "wilcoxon_signed_rank_test" => Some(param_schema_map(&[
                        ("input1", ToolParamSchema::input_raster()),
                        ("input2", ToolParamSchema::input_raster()),
                        ("num_samples", ToolParamSchema::scalar_integer()),
                ])),
                "turning_bands_simulation" => Some(param_schema_map(&[
                        ("input", ToolParamSchema::input_raster()),
                        ("range", ToolParamSchema::scalar_float()),
                        ("iterations", ToolParamSchema::scalar_integer()),
                        ("output", ToolParamSchema::output_raster()),
                ])),
                "trend_surface" => Some(param_schema_map(&[
                        ("input", ToolParamSchema::input_raster()),
                        ("polynomial_order", ToolParamSchema::scalar_integer()),
                        ("output", ToolParamSchema::output_raster()),
                ])),
                "trend_surface_vector_points" => Some(param_schema_map(&[
                        ("input", ToolParamSchema::input_vector_any()),
                        ("cell_size", ToolParamSchema::scalar_float()),
                        ("field_name", ToolParamSchema::string()),
                        ("polynomial_order", ToolParamSchema::scalar_integer()),
                        ("output", ToolParamSchema::output_raster()),
                ])),
                "raster_calculator" => Some(param_schema_map(&[
                        ("expression", ToolParamSchema::string()),
                        (
                                "inputs",
                                ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
                        ),
                        ("auto_reproject", ToolParamSchema::bool()),
                        ("auto_reproject_method", ToolParamSchema::string()),
                        ("output", ToolParamSchema::output_raster()),
                ])),
                "principal_component_analysis" => Some(param_schema_map(&[
                        (
                                "inputs",
                                ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
                        ),
                        ("auto_reproject", ToolParamSchema::bool()),
                        ("auto_reproject_method", ToolParamSchema::string()),
                        ("num_components", ToolParamSchema::scalar_integer()),
                        ("standardized", ToolParamSchema::bool()),
                        ("output", ToolParamSchema::output_raster()),
                ])),
                "inverse_pca" => Some(param_schema_map(&[
                        (
                                "inputs",
                                ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
                        ),
                        ("auto_reproject", ToolParamSchema::bool()),
                        ("auto_reproject_method", ToolParamSchema::string()),
                        ("pca_report", ToolParamSchema::string()),
                        ("output", ToolParamSchema::output_raster()),
                ])),
                _ => None,
        }
}

pub use raster_add::RasterAddTool;
pub use raster_add::RasterAtan2Tool;
pub use raster_add::RasterBoolAndTool;
pub use raster_add::RasterBoolOrTool;
pub use raster_add::RasterBoolXorTool;
pub use raster_add::RasterDivideTool;
pub use raster_add::RasterEqualToTool;
pub use raster_add::RasterGreaterThanTool;
pub use raster_add::RasterGreaterThanOrEqualToTool;
pub use raster_add::RasterIntegerDivisionTool;
pub use raster_add::RasterLessThanTool;
pub use raster_add::RasterLessThanOrEqualToTool;
pub use raster_add::RasterModuloTool;
pub use raster_add::RasterMultiplyTool;
pub use raster_add::RasterNotEqualToTool;
pub use raster_add::RasterPowerTool;
pub use raster_add::RasterSubtractTool;
pub use raster_stats::ListUniqueValuesRasterTool;
pub use raster_stats::ListUniqueValuesTool;
pub use raster_stats::MaxTool;
pub use raster_stats::MinTool;
pub use raster_stats::QuantilesTool;
pub use raster_stats::RasterHistogramTool;
pub use raster_stats::RasterSummaryStatsTool;
pub use raster_stats::RescaleValueRangeTool;
pub use raster_stats::RootMeanSquareErrorTool;
pub use raster_stats::CumulativeDistributionTool;
pub use raster_stats::CrispnessIndexTool;
pub use raster_stats::ConditionalEvaluationTool;
pub use raster_stats::InPlaceAddTool;
pub use raster_stats::AttributeCorrelationTool;
pub use raster_stats::AttributeHistogramTool;
pub use raster_stats::AttributeScattergramTool;
pub use raster_stats::AnovaTool;
pub use raster_stats::CrossTabulationTool;
pub use raster_stats::InPlaceDivideTool;
pub use raster_stats::InPlaceMultiplyTool;
pub use raster_stats::InPlaceSubtractTool;
pub use raster_stats::FftRandomFieldTool;
pub use raster_stats::KappaIndexTool;
pub use raster_stats::KsNormalityTestTool;
pub use raster_stats::PairedSampleTTestTool;
pub use raster_stats::PhiCoefficientTool;
pub use raster_stats::RandomFieldTool;
pub use raster_stats::RandomSampleTool;
pub use raster_stats::TwoSampleKsTestTool;
pub use raster_stats::WilcoxonSignedRankTestTool;
pub use raster_stats::ZScoresTool;
pub use raster_stats::ImageCorrelationTool;
pub use raster_stats::ImageAutocorrelationTool;
pub use raster_stats::ImageCorrelationNeighbourhoodAnalysisTool;
pub use raster_stats::ImageRegressionTool;
pub use raster_stats::DbscanTool;
pub use raster_stats::ZonalStatisticsTool;
pub use raster_stats::TurningBandsSimulationTool;
pub use raster_stats::TrendSurfaceTool;
pub use raster_stats::TrendSurfaceVectorPointsTool;
pub use raster_stats::RasterCalculatorTool;
pub use raster_stats::PrincipalComponentAnalysisTool;
pub use raster_stats::InversePcaTool;
pub use raster_unary_math::{
	RasterAbsTool,
        RasterArccosTool,
        RasterArcoshTool,
        RasterArcsinTool,
        RasterArctanTool,
        RasterArsinhTool,
        RasterArtanhTool,
        RasterBoolNotTool,
        RasterCeilTool,
        RasterCosTool,
        RasterCoshTool,
        RasterDecrementTool,
        RasterExp2Tool,
        RasterExpTool,
        RasterFloorTool,
        RasterIncrementTool,
        RasterIsNodataTool,
        RasterLnTool,
        RasterLog10Tool,
        RasterLog2Tool,
        RasterNegateTool,
        RasterReciprocalTool,
        RasterRoundTool,
        RasterSinTool,
        RasterSinhTool,
        RasterSqrtTool,
        RasterSquareTool,
        RasterTanTool,
        RasterTanhTool,
        RasterToDegTool,
        RasterToRadTool,
        RasterTruncateTool,
};
