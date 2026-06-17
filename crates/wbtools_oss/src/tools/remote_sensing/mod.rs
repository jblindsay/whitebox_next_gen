use std::collections::BTreeMap;
use wbcore::ToolParamSchema;
use wbcore::ToolVectorGeometry;

mod color_support;
mod obia_tools;
mod radiometric_tools;
mod terrain_corrected_optical_analytics;
mod brdf_normalization;
mod georeference_raster_from_control_points;
mod orthorectification;
mod advanced_filters;
mod bilateral_filter;
mod convolution_filters;
mod convolution_extra_filters;
mod gaussian_filter;
mod phase3_filters;
mod non_filter_tools;
mod texture_glcm_tool;
mod rank_filters;
mod window_stats_filters;

fn param_schema_map(entries: &[(&str, ToolParamSchema)]) -> BTreeMap<String, ToolParamSchema> {
	let mut map = BTreeMap::new();
	for (name, schema) in entries {
		map.insert((*name).to_string(), schema.clone());
	}
	map
}

pub fn remote_sensing_tool_param_schemas(tool_id: &str) -> Option<BTreeMap<String, ToolParamSchema>> {
	match tool_id {
		"corner_detection" | "integral_image_transform" | "line_thinning" | "otsu_thresholding" => {
			Some(param_schema_map(&[
				("input", ToolParamSchema::input_raster()),
				("output", ToolParamSchema::output_raster()),
			]))
		}
		"adaptive_filter" | "closing" | "conservative_smoothing_filter" | "diversity_filter"
		| "high_pass_filter" | "majority_filter" | "maximum_filter" | "mean_filter"
		| "minimum_filter" | "olympic_filter" | "opening" | "range_filter"
		| "refined_lee_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("filter_size_x", ToolParamSchema::scalar_integer()),
			("filter_size_y", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"high_pass_median_filter" | "median_filter" | "percentile_filter" => {
			Some(param_schema_map(&[
				("input", ToolParamSchema::input_raster()),
				("filter_size_x", ToolParamSchema::scalar_integer()),
				("filter_size_y", ToolParamSchema::scalar_integer()),
				("sig_digits", ToolParamSchema::scalar_integer()),
				("output", ToolParamSchema::output_raster()),
			]))
		}
		"fast_almost_gaussian_filter" | "laplacian_of_gaussians_filter" => {
			Some(param_schema_map(&[
				("input", ToolParamSchema::input_raster()),
				("sigma", ToolParamSchema::scalar_float()),
				("output", ToolParamSchema::output_raster()),
			]))
		}
		"gaussian_contrast_stretch" | "histogram_equalization" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("num_tones", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"prewitt_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("clip_tails", ToolParamSchema::bool()),
			("output", ToolParamSchema::output_raster()),
		])),
		"roberts_cross_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("clip_amount", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"remove_spurs" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("max_iterations", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"flip_image" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("direction", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
		])),
		"gamma_correction" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("gamma", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"edge_preserving_mean_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("filter_size", ToolParamSchema::scalar_integer()),
			("threshold", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"emboss_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("direction", ToolParamSchema::string()),
			("clip_amount", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"laplacian_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("variant", ToolParamSchema::string()),
			("clip_amount", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"line_detection_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("variant", ToolParamSchema::string()),
			("abs_values", ToolParamSchema::bool()),
			("clip_tails", ToolParamSchema::bool()),
			("output", ToolParamSchema::output_raster()),
		])),
		"gamma_map_filter" | "kuan_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("radius", ToolParamSchema::scalar_integer()),
			("enl", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"kuwahara_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("radius", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"guided_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("radius", ToolParamSchema::scalar_integer()),
			("epsilon", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"frost_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("radius", ToolParamSchema::scalar_integer()),
			("damping_factor", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"anisotropic_diffusion_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("iterations", ToolParamSchema::scalar_integer()),
			("kappa", ToolParamSchema::scalar_float()),
			("lambda", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"balance_contrast_enhancement" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("band_mean", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"diff_of_gaussians_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("filter_size_x", ToolParamSchema::scalar_integer()),
			("sigma2", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"direct_decorrelation_stretch" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("achromatic_factor", ToolParamSchema::scalar_float()),
			("clip_percent", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"frangi_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("scales", ToolParamSchema::string()),
			("beta", ToolParamSchema::scalar_float()),
			("c", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"gabor_filter_bank" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("sigma", ToolParamSchema::scalar_float()),
			("frequency", ToolParamSchema::scalar_float()),
			("orientations", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
		])),
		"non_local_means_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("search_radius", ToolParamSchema::scalar_integer()),
			("patch_radius", ToolParamSchema::scalar_integer()),
			("h", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"k_nearest_mean_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("filter_size_x", ToolParamSchema::scalar_integer()),
			("filter_size_y", ToolParamSchema::scalar_integer()),
			("k", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"lee_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("filter_size_x", ToolParamSchema::scalar_integer()),
			("filter_size_y", ToolParamSchema::scalar_integer()),
			("sigma", ToolParamSchema::scalar_float()),
			("m_value", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"enhanced_lee_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("filter_size_x", ToolParamSchema::scalar_integer()),
			("filter_size_y", ToolParamSchema::scalar_integer()),
			("enl", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"histogram_matching" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("histogram", ToolParamSchema::string()),
			("is_cumulative", ToolParamSchema::bool()),
			("output", ToolParamSchema::output_raster()),
		])),
		"histogram_matching_two_images" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("reference", ToolParamSchema::input_raster()),
			("output", ToolParamSchema::output_raster()),
		])),
		"min_max_contrast_stretch" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("min_val", ToolParamSchema::scalar_float()),
			("max_val", ToolParamSchema::scalar_float()),
			("num_tones", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"normalized_difference_index" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("band1", ToolParamSchema::scalar_integer()),
			("band2", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"percentage_contrast_stretch" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("clip", ToolParamSchema::scalar_float()),
			("tail", ToolParamSchema::string()),
			("num_tones", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"piecewise_contrast_stretch" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("function", ToolParamSchema::string()),
			("greytones", ToolParamSchema::input_raster()),
			("output", ToolParamSchema::output_raster()),
		])),
		"savitzky_golay_2d_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("window_size", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"scharr_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("clip_tails", ToolParamSchema::bool()),
			("output", ToolParamSchema::output_raster()),
		])),
		"mosaic" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("auto_reproject", ToolParamSchema::bool()),
			(
				"auto_reproject_method",
				ToolParamSchema::enum_values(&[
					"nearest",
					"bilinear",
					"cubic",
					"lanczos",
					"average",
					"min",
					"max",
					"mode",
					"median",
					"stddev",
				]),
			),
			("method", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
		])),
		"mosaic_with_feathering" => Some(param_schema_map(&[
			("input1", ToolParamSchema::input_raster()),
			("input2", ToolParamSchema::input_raster()),
			("method", ToolParamSchema::string()),
			("weight", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"resample" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("base", ToolParamSchema::input_raster()),
			("cell_size", ToolParamSchema::scalar_float()),
			("method", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
		])),
		"canny_edge_detection" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("sigma", ToolParamSchema::scalar_float()),
			("low_threshold", ToolParamSchema::scalar_float()),
			("high_threshold", ToolParamSchema::scalar_float()),
			("add_back", ToolParamSchema::bool()),
			("output", ToolParamSchema::output_raster()),
		])),
		"build_object_hierarchy_multiscale" => Some(param_schema_map(&[
			("coarse_segments", ToolParamSchema::input_raster()),
			("fine_segments", ToolParamSchema::input_raster()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::Table)),
		])),
		"object_features_context_neighbors" | "object_features_shape_basic"
		| "object_features_topology_relations" => Some(param_schema_map(&[
			("segments", ToolParamSchema::input_raster()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::Table)),
		])),
		"object_features_spectral_basic" => Some(param_schema_map(&[
			("segments", ToolParamSchema::input_raster()),
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::Table)),
		])),
		"object_features_texture_glcm_basic" => Some(param_schema_map(&[
			("segments", ToolParamSchema::input_raster()),
			("input", ToolParamSchema::input_raster()),
			("levels", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::Table)),
		])),
		"object_class_probability_maps" => Some(param_schema_map(&[
			(
				"predictions",
				ToolParamSchema::input(wbcore::ToolDatasetSchema::Lidar),
			),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::Table)),
		])),
		"objects_enforce_min_mapping_unit" | "segments_merge_small_regions" => {
			Some(param_schema_map(&[
				("segments", ToolParamSchema::input_raster()),
				("min_size", ToolParamSchema::scalar_integer()),
				("method", ToolParamSchema::string()),
				("output", ToolParamSchema::output_raster()),
			]))
		}
		"objects_boundary_refinement_pro" => Some(param_schema_map(&[
			("segments", ToolParamSchema::input_raster()),
			("iterations", ToolParamSchema::scalar_integer()),
			("min_size", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"evaluate_segmentation_quality_pro" => Some(param_schema_map(&[
			("segments", ToolParamSchema::input_raster()),
			("reference", ToolParamSchema::input_raster()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::Json)),
		])),
		"segment_graph_felzenszwalb" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("auto_reproject", ToolParamSchema::bool()),
			(
				"auto_reproject_method",
				ToolParamSchema::enum_values(&[
					"nearest",
					"bilinear",
					"cubic",
					"lanczos",
					"average",
					"min",
					"max",
					"mode",
					"median",
					"stddev",
				]),
			),
			("k", ToolParamSchema::scalar_float()),
			("sigma", ToolParamSchema::scalar_float()),
			("min_area", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"segment_slic_superpixels" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("auto_reproject", ToolParamSchema::bool()),
			(
				"auto_reproject_method",
				ToolParamSchema::enum_values(&[
					"nearest",
					"bilinear",
					"cubic",
					"lanczos",
					"average",
					"min",
					"max",
					"mode",
					"median",
					"stddev",
				]),
			),
			("region_size", ToolParamSchema::scalar_integer()),
			("compactness", ToolParamSchema::scalar_float()),
			("min_area", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"segment_watershed_markers" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("gradient_weight", ToolParamSchema::scalar_float()),
			("min_area", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"segment_multiresolution_hierarchical" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("coarse_k", ToolParamSchema::scalar_float()),
			("fine_k", ToolParamSchema::scalar_float()),
			(
				"output_prefix",
				ToolParamSchema::output(wbcore::ToolDatasetSchema::File),
			),
		])),
		"segment_scale_parameter_optimizer" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("candidate_scales", ToolParamSchema::string()),
			("target_objects", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::Json)),
		])),
		"polygons_to_segments" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_vector_any()),
			("base", ToolParamSchema::input_raster()),
			("field", ToolParamSchema::field("input", None)),
			("output", ToolParamSchema::output_raster()),
		])),
		"segments_to_polygons" => Some(param_schema_map(&[
			("segments", ToolParamSchema::input_raster()),
			("output", ToolParamSchema::output_vector_any()),
		])),
		"obia_pipeline_basic" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("training", ToolParamSchema::input(wbcore::ToolDatasetSchema::Table)),
			(
				"output_prefix",
				ToolParamSchema::output(wbcore::ToolDatasetSchema::File),
			),
			("segment_method", ToolParamSchema::string()),
			("min_size", ToolParamSchema::scalar_integer()),
			("class_field", ToolParamSchema::string()),
		])),
		"propagate_labels_across_hierarchy" => Some(param_schema_map(&[
			("hierarchy", ToolParamSchema::input(wbcore::ToolDatasetSchema::Table)),
			("parent_labels", ToolParamSchema::input(wbcore::ToolDatasetSchema::Lidar)),
			("child_labels", ToolParamSchema::input(wbcore::ToolDatasetSchema::Lidar)),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::Table)),
		])),
		"evaluate_object_classification_accuracy" => Some(param_schema_map(&[
			("predictions", ToolParamSchema::input(wbcore::ToolDatasetSchema::Lidar)),
			("reference", ToolParamSchema::input(wbcore::ToolDatasetSchema::Lidar)),
			("segment_id_field", ToolParamSchema::string()),
			("predicted_field", ToolParamSchema::input(wbcore::ToolDatasetSchema::Lidar)),
			("reference_field", ToolParamSchema::input(wbcore::ToolDatasetSchema::Lidar)),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::Json)),
		])),
		"knn_classification" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("auto_reproject", ToolParamSchema::bool()),
			(
				"auto_reproject_method",
				ToolParamSchema::enum_values(&[
					"nearest",
					"bilinear",
					"cubic",
					"lanczos",
					"average",
					"min",
					"max",
					"mode",
					"median",
					"stddev",
				]),
			),
			("training_data", ToolParamSchema::input_vector_any()),
			("class_field", ToolParamSchema::field("training_data", None)),
			("scaling", ToolParamSchema::string()),
			("k", ToolParamSchema::scalar_integer()),
			("clip", ToolParamSchema::bool()),
			("output", ToolParamSchema::output_raster()),
		])),
		"knn_regression" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("auto_reproject", ToolParamSchema::bool()),
			(
				"auto_reproject_method",
				ToolParamSchema::enum_values(&[
					"nearest",
					"bilinear",
					"cubic",
					"lanczos",
					"average",
					"min",
					"max",
					"mode",
					"median",
					"stddev",
				]),
			),
			("training_data", ToolParamSchema::input_vector_any()),
			("field", ToolParamSchema::field("training_data", None)),
			("scaling", ToolParamSchema::string()),
			("k", ToolParamSchema::scalar_integer()),
			("distance_weighted", ToolParamSchema::bool()),
			("output", ToolParamSchema::output_raster()),
		])),
		"logistic_regression" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("auto_reproject", ToolParamSchema::bool()),
			(
				"auto_reproject_method",
				ToolParamSchema::enum_values(&[
					"nearest",
					"bilinear",
					"cubic",
					"lanczos",
					"average",
					"min",
					"max",
					"mode",
					"median",
					"stddev",
				]),
			),
			("training_data", ToolParamSchema::input_vector_any()),
			("class_field", ToolParamSchema::field("training_data", None)),
			("scaling", ToolParamSchema::string()),
			("alpha", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"random_forest_classification" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("auto_reproject", ToolParamSchema::bool()),
			(
				"auto_reproject_method",
				ToolParamSchema::enum_values(&[
					"nearest",
					"bilinear",
					"cubic",
					"lanczos",
					"average",
					"min",
					"max",
					"mode",
					"median",
					"stddev",
				]),
			),
			("training_data", ToolParamSchema::input_vector_any()),
			("class_field", ToolParamSchema::field("training_data", None)),
			("scaling", ToolParamSchema::string()),
			("n_trees", ToolParamSchema::scalar_integer()),
			("min_samples_leaf", ToolParamSchema::scalar_integer()),
			("min_samples_split", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"random_forest_regression" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("auto_reproject", ToolParamSchema::bool()),
			(
				"auto_reproject_method",
				ToolParamSchema::enum_values(&[
					"nearest",
					"bilinear",
					"cubic",
					"lanczos",
					"average",
					"min",
					"max",
					"mode",
					"median",
					"stddev",
				]),
			),
			("training_data", ToolParamSchema::input_vector_any()),
			("field", ToolParamSchema::field("training_data", None)),
			("scaling", ToolParamSchema::string()),
			("n_trees", ToolParamSchema::scalar_integer()),
			("min_samples_leaf", ToolParamSchema::scalar_integer()),
			("min_samples_split", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"fuzzy_knn_classification" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("auto_reproject", ToolParamSchema::bool()),
			(
				"auto_reproject_method",
				ToolParamSchema::enum_values(&[
					"nearest",
					"bilinear",
					"cubic",
					"lanczos",
					"average",
					"min",
					"max",
					"mode",
					"median",
					"stddev",
				]),
			),
			("training_data", ToolParamSchema::input_vector_any()),
			("class_field", ToolParamSchema::field("training_data", None)),
			("scaling", ToolParamSchema::string()),
			("k", ToolParamSchema::scalar_integer()),
			("m", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
			("probability_output", ToolParamSchema::output_raster()),
		])),
		"nnd_classification" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("auto_reproject", ToolParamSchema::bool()),
			(
				"auto_reproject_method",
				ToolParamSchema::enum_values(&[
					"nearest",
					"bilinear",
					"cubic",
					"lanczos",
					"average",
					"min",
					"max",
					"mode",
					"median",
					"stddev",
				]),
			),
			("training_data", ToolParamSchema::input_vector_any()),
			("class_field", ToolParamSchema::field("training_data", None)),
			("scaling", ToolParamSchema::string()),
			("z_threshold", ToolParamSchema::scalar_float()),
			("outlier_is_zero", ToolParamSchema::bool()),
			("k", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"min_dist_classification" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("auto_reproject", ToolParamSchema::bool()),
			(
				"auto_reproject_method",
				ToolParamSchema::enum_values(&[
					"nearest",
					"bilinear",
					"cubic",
					"lanczos",
					"average",
					"min",
					"max",
					"mode",
					"median",
					"stddev",
				]),
			),
			("training_data", ToolParamSchema::input_vector_any()),
			("class_field", ToolParamSchema::field("training_data", None)),
			("dist_threshold", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"parallelepiped_classification" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("auto_reproject", ToolParamSchema::bool()),
			(
				"auto_reproject_method",
				ToolParamSchema::enum_values(&[
					"nearest",
					"bilinear",
					"cubic",
					"lanczos",
					"average",
					"min",
					"max",
					"mode",
					"median",
					"stddev",
				]),
			),
			("training_data", ToolParamSchema::input_vector_any()),
			("class_field", ToolParamSchema::field("training_data", None)),
			("output", ToolParamSchema::output_raster()),
		])),
		"k_means_clustering" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("classes", ToolParamSchema::scalar_integer()),
			("max_iterations", ToolParamSchema::scalar_integer()),
			("class_change", ToolParamSchema::scalar_float()),
			("initialize", ToolParamSchema::string()),
			("min_class_size", ToolParamSchema::scalar_integer()),
			("out_html", ToolParamSchema::output(wbcore::ToolDatasetSchema::Text)),
			("output", ToolParamSchema::output_raster()),
		])),
		"modified_k_means_clustering" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("start_clusters", ToolParamSchema::scalar_integer()),
			("merge_dist", ToolParamSchema::scalar_float()),
			("max_iterations", ToolParamSchema::scalar_integer()),
			("class_change", ToolParamSchema::scalar_float()),
			("out_html", ToolParamSchema::output(wbcore::ToolDatasetSchema::Text)),
			("output", ToolParamSchema::output_raster()),
		])),
		"change_vector_analysis" => Some(param_schema_map(&[
			("date1", ToolParamSchema::input_raster()),
			("date2", ToolParamSchema::input_raster()),
			("magnitude_output", ToolParamSchema::output_raster()),
			("direction_output", ToolParamSchema::output_raster()),
		])),
		"image_difference_change_detection" => Some(param_schema_map(&[
			(
				"t1_inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			(
				"t2_inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("mode", ToolParamSchema::string()),
			("threshold_sigma", ToolParamSchema::scalar_float()),
			("auto_reproject", ToolParamSchema::bool()),
			(
				"auto_reproject_method",
				ToolParamSchema::enum_values(&[
					"nearest",
					"bilinear",
					"cubic",
					"lanczos",
					"average",
					"min",
					"max",
					"mode",
					"median",
					"stddev",
				]),
			),
			("output", ToolParamSchema::output_raster()),
			("output_absolute", ToolParamSchema::output_raster()),
			("output_signed", ToolParamSchema::output_raster()),
			("output_mask", ToolParamSchema::output_raster()),
		])),
		"pca_based_change_detection" => Some(param_schema_map(&[
			(
				"t1_inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			(
				"t2_inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("component", ToolParamSchema::scalar_integer()),
			("standardized", ToolParamSchema::bool()),
			("threshold_sigma", ToolParamSchema::scalar_float()),
			("auto_reproject", ToolParamSchema::bool()),
			(
				"auto_reproject_method",
				ToolParamSchema::enum_values(&[
					"nearest",
					"bilinear",
					"cubic",
					"lanczos",
					"average",
					"min",
					"max",
					"mode",
					"median",
					"stddev",
				]),
			),
			("output", ToolParamSchema::output_raster()),
			("output_mask", ToolParamSchema::output_raster()),
			("output_report", ToolParamSchema::output(wbcore::ToolDatasetSchema::Json)),
		])),
		"post_classification_change" => Some(param_schema_map(&[
			("t1_classified", ToolParamSchema::input_raster()),
			("t2_classified", ToolParamSchema::input_raster()),
			(
				"transition_scale",
				ToolParamSchema::input(wbcore::ToolDatasetSchema::Lidar),
			),
			("t1_class_remap", ToolParamSchema::input(wbcore::ToolDatasetSchema::Lidar)),
			("t2_class_remap", ToolParamSchema::input(wbcore::ToolDatasetSchema::Lidar)),
			("auto_reproject", ToolParamSchema::bool()),
			(
				"auto_reproject_method",
				ToolParamSchema::enum_values(&[
					"nearest",
					"bilinear",
					"cubic",
					"lanczos",
					"average",
					"min",
					"max",
					"mode",
					"median",
					"stddev",
				]),
			),
			("output", ToolParamSchema::output_raster()),
		])),
		"classify_objects_random_forest" | "classify_objects_svm"
		| "classify_objects_ensemble_pro" => Some(param_schema_map(&[
			("features", ToolParamSchema::input_vector_any()),
			("training", ToolParamSchema::input(wbcore::ToolDatasetSchema::Table)),
			("segment_id_field", ToolParamSchema::string()),
			("class_field", ToolParamSchema::string()),
			("n_trees", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::Table)),
		])),
		"classify_objects_rules_basic" | "classify_objects_rules_hierarchical" => {
			Some(param_schema_map(&[
				("features", ToolParamSchema::input_vector_any()),
				("rules", ToolParamSchema::input_vector_any()),
				("default_class", ToolParamSchema::input(wbcore::ToolDatasetSchema::Lidar)),
				("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::Table)),
			]))
		}
		"obia_batch_orchestrator_pro" => Some(param_schema_map(&[
			("jobs", ToolParamSchema::string()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::Json)),
		])),
		"obia_audit_report_pro" => Some(param_schema_map(&[
			("artifacts", ToolParamSchema::string()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::Json)),
		])),
		"object_uncertainty_diagnostics_pro" => Some(param_schema_map(&[
			(
				"probabilities",
				ToolParamSchema::input(wbcore::ToolDatasetSchema::Table),
			),
			("low_conf_threshold", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::Json)),
		])),
		"segments_split_low_cohesion" => Some(param_schema_map(&[
			("segments", ToolParamSchema::input_raster()),
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("split_scale", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"split_colour_composite" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("red_output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
			("green_output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
			("blue_output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
		])),
		"ihs_to_rgb" => Some(param_schema_map(&[
			("intensity", ToolParamSchema::input_raster()),
			("hue", ToolParamSchema::input_raster()),
			("saturation", ToolParamSchema::input_raster()),
			("red_output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
			("green_output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
			("blue_output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
		])),
		"rgb_to_ihs" => Some(param_schema_map(&[
			("red", ToolParamSchema::input_raster()),
			("green", ToolParamSchema::input_raster()),
			("blue", ToolParamSchema::input_raster()),
			("composite", ToolParamSchema::input_raster()),
			(
				"intensity_output",
				ToolParamSchema::output(wbcore::ToolDatasetSchema::File),
			),
			("hue_output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
			(
				"saturation_output",
				ToolParamSchema::output(wbcore::ToolDatasetSchema::File),
			),
		])),
		"sigmoidal_contrast_stretch" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("cutoff", ToolParamSchema::scalar_float()),
			("gain", ToolParamSchema::scalar_float()),
			("num_tones", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"standard_deviation_contrast_stretch" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("clip", ToolParamSchema::scalar_float()),
			("num_tones", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"sobel_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("variant", ToolParamSchema::string()),
			("clip_tails", ToolParamSchema::bool()),
			("output", ToolParamSchema::output_raster()),
		])),
		"standard_deviation_filter" | "total_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("filter_size_x", ToolParamSchema::scalar_integer()),
			("filter_size_y", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"thicken_raster_line" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("output", ToolParamSchema::output_raster()),
		])),
		"tophat_transform" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("filter_size_x", ToolParamSchema::scalar_integer()),
			("filter_size_y", ToolParamSchema::scalar_integer()),
			("variant", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
		])),
		"unsharp_masking" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("sigma", ToolParamSchema::scalar_float()),
			("amount", ToolParamSchema::scalar_float()),
			("threshold", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"user_defined_weights_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("weights", ToolParamSchema::string()),
			("kernel_center", ToolParamSchema::string()),
			("normalize_weights", ToolParamSchema::bool()),
			("output", ToolParamSchema::output_raster()),
		])),
		"wiener_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("radius", ToolParamSchema::scalar_integer()),
			("noise_variance", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"write_function_memory_insertion" => Some(param_schema_map(&[
			("input1", ToolParamSchema::input_raster()),
			("input2", ToolParamSchema::input_raster()),
			("input3", ToolParamSchema::input_raster()),
			("output", ToolParamSchema::output_raster()),
		])),
		"brdf_normalization" => Some(param_schema_map(&[
			("input_raster", ToolParamSchema::input_raster()),
			("input_dem", ToolParamSchema::input_raster()),
			("solar_zenith_deg", ToolParamSchema::scalar_float()),
			("solar_azimuth_deg", ToolParamSchema::scalar_float()),
			("method", ToolParamSchema::string()),
			("minnaert_k", ToolParamSchema::scalar_float()),
			("z_factor", ToolParamSchema::scalar_float()),
			("output_prefix", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
			("output", ToolParamSchema::output_raster()),
		])),
		"terrain_corrected_optical_analytics" => Some(param_schema_map(&[
			("input_dem", ToolParamSchema::input_raster()),
			("bundle_root", ToolParamSchema::string()),
			("input_red", ToolParamSchema::input_raster()),
			("input_nir", ToolParamSchema::input_raster()),
			("input_green", ToolParamSchema::input_raster()),
			("input_blue", ToolParamSchema::input_raster()),
			("solar_mode", ToolParamSchema::string()),
			("solar_zenith_deg", ToolParamSchema::scalar_float()),
			("solar_azimuth_deg", ToolParamSchema::scalar_float()),
			("acquisition_datetime_utc", ToolParamSchema::string()),
			("latitude", ToolParamSchema::scalar_float()),
			("longitude", ToolParamSchema::scalar_float()),
			("profile", ToolParamSchema::string()),
			("cloud_threshold", ToolParamSchema::scalar_float()),
			("shadow_threshold", ToolParamSchema::scalar_float()),
			("qa_mask", ToolParamSchema::input_raster()),
			("qa_mask_format", ToolParamSchema::string()),
			("mask_strategy", ToolParamSchema::string()),
			("z_factor", ToolParamSchema::scalar_float()),
			("output_prefix", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
		])),
		"orthorectification" => Some(param_schema_map(&[
			("input_raster", ToolParamSchema::input_raster()),
			("input_dem", ToolParamSchema::input_raster()),
			("output_epsg", ToolParamSchema::scalar_integer()),
			("output_resolution", ToolParamSchema::scalar_float()),
			("resample_method", ToolParamSchema::string()),
			("nodata_value", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"georeference_raster_from_control_points" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("control_points", ToolParamSchema::input(wbcore::ToolDatasetSchema::Table)),
			("epsg", ToolParamSchema::scalar_integer()),
			("transform_type", ToolParamSchema::string()),
			("transform_order", ToolParamSchema::scalar_integer()),
			("resample", ToolParamSchema::string()),
			("allow_auto_downgrade", ToolParamSchema::bool()),
			("output", ToolParamSchema::output_raster()),
			("report", ToolParamSchema::output(wbcore::ToolDatasetSchema::Json)),
		])),
		"land_surface_temperature_single_channel" => Some(param_schema_map(&[
			("thermal_input", ToolParamSchema::input_raster()),
			("input_is_brightness_temp", ToolParamSchema::bool()),
			("emissivity_input", ToolParamSchema::input_raster()),
			("emissivity_constant", ToolParamSchema::scalar_float()),
			("sensor_bundle_root", ToolParamSchema::string()),
			("thermal_band_number", ToolParamSchema::scalar_integer()),
			("radiance_mult", ToolParamSchema::scalar_float()),
			("radiance_add", ToolParamSchema::scalar_float()),
			("k1_constant", ToolParamSchema::scalar_float()),
			("k2_constant", ToolParamSchema::scalar_float()),
			("wavelength_um", ToolParamSchema::scalar_float()),
			("output_units", ToolParamSchema::string()),
			("auto_reproject", ToolParamSchema::bool()),
			("auto_reproject_method", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
		])),
		"land_surface_temperature_split_window" => Some(param_schema_map(&[
			("thermal1_input", ToolParamSchema::input_raster()),
			("thermal2_input", ToolParamSchema::input_raster()),
			("input_is_brightness_temp", ToolParamSchema::bool()),
			("emissivity_mean_input", ToolParamSchema::input_raster()),
			("emissivity_delta_input", ToolParamSchema::input_raster()),
			("emissivity_mean_constant", ToolParamSchema::scalar_float()),
			("emissivity_delta_constant", ToolParamSchema::scalar_float()),
			("sensor_bundle_root", ToolParamSchema::string()),
			("thermal_band1_number", ToolParamSchema::scalar_integer()),
			("thermal_band2_number", ToolParamSchema::scalar_integer()),
			("radiance1_mult", ToolParamSchema::scalar_float()),
			("radiance1_add", ToolParamSchema::scalar_float()),
			("k1_1", ToolParamSchema::scalar_float()),
			("k2_1", ToolParamSchema::scalar_float()),
			("radiance2_mult", ToolParamSchema::scalar_float()),
			("radiance2_add", ToolParamSchema::scalar_float()),
			("k1_2", ToolParamSchema::scalar_float()),
			("k2_2", ToolParamSchema::scalar_float()),
			("coeff_a0", ToolParamSchema::scalar_float()),
			("coeff_a1", ToolParamSchema::scalar_float()),
			("coeff_a2", ToolParamSchema::scalar_float()),
			("coeff_a3", ToolParamSchema::scalar_float()),
			("coeff_a4", ToolParamSchema::scalar_float()),
			("coeff_a5", ToolParamSchema::scalar_float()),
			("output_units", ToolParamSchema::string()),
			("auto_reproject", ToolParamSchema::bool()),
			("auto_reproject_method", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
		])),
		"linear_spectral_unmixing" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("endmembers", ToolParamSchema::string()),
			("sum_to_one", ToolParamSchema::bool()),
			("iterations", ToolParamSchema::scalar_integer()),
			("step_size", ToolParamSchema::scalar_float()),
			("auto_reproject", ToolParamSchema::bool()),
			("auto_reproject_method", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
			("output_residual", ToolParamSchema::output_raster()),
		])),
		"continuum_removal" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("wavelengths", ToolParamSchema::string()),
			("auto_reproject", ToolParamSchema::bool()),
			("auto_reproject_method", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
		])),
		"dn_to_toa_reflectance" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("reflectance_mult", ToolParamSchema::string()),
			("reflectance_add", ToolParamSchema::string()),
			("sensor_bundle_root", ToolParamSchema::string()),
			("sun_elevation_deg", ToolParamSchema::scalar_float()),
			("apply_solar_correction", ToolParamSchema::bool()),
			("clamp_unit_interval", ToolParamSchema::bool()),
			("auto_reproject", ToolParamSchema::bool()),
			("auto_reproject_method", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
		])),
		"dark_object_subtraction" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("percentile", ToolParamSchema::scalar_float()),
			("clamp_non_negative", ToolParamSchema::bool()),
			("auto_reproject", ToolParamSchema::bool()),
			("auto_reproject_method", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
			("output_diagnostic_offsets", ToolParamSchema::output_raster()),
		])),
		"correct_vignetting" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("pp", ToolParamSchema::input_vector(ToolVectorGeometry::Point)),
			("focal_length", ToolParamSchema::scalar_float()),
			("image_width", ToolParamSchema::scalar_float()),
			("n", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
		])),
		"cloude_pottier_decomposition" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("matrix_format", ToolParamSchema::string()),
			("auto_reproject", ToolParamSchema::bool()),
			("auto_reproject_method", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
		])),
		"freeman_durden_decomposition" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("matrix_format", ToolParamSchema::string()),
			("auto_reproject", ToolParamSchema::bool()),
			("auto_reproject_method", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
			("output_clip_mask", ToolParamSchema::output_raster()),
		])),
		"h_alpha_wisart_classification" => Some(param_schema_map(&[
			("h_raster", ToolParamSchema::input_raster()),
			("alpha_raster", ToolParamSchema::input_raster()),
			("auto_reproject", ToolParamSchema::bool()),
			("auto_reproject_method", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
		])),
		"create_colour_composite" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("red", ToolParamSchema::input_raster()),
			("green", ToolParamSchema::input_raster()),
			("blue", ToolParamSchema::input_raster()),
			("opacity", ToolParamSchema::input_raster()),
			("enhance", ToolParamSchema::bool()),
			("treat_zeros_as_nodata", ToolParamSchema::bool()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
		])),
		"glcm_texture" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("window_size", ToolParamSchema::scalar_integer()),
			("distance", ToolParamSchema::scalar_integer()),
			("angles", ToolParamSchema::string()),
			("features", ToolParamSchema::string()),
			("direction_aggregation", ToolParamSchema::string()),
			("levels", ToolParamSchema::scalar_integer()),
			("symmetric", ToolParamSchema::bool()),
			("output", ToolParamSchema::output_raster()),
		])),
		"image_slider" => Some(param_schema_map(&[
			("input1", ToolParamSchema::input_raster()),
			("input2", ToolParamSchema::input_raster()),
			("label1", ToolParamSchema::string()),
			("left_palette", ToolParamSchema::string()),
			("left_reverse_palette", ToolParamSchema::bool()),
			("label2", ToolParamSchema::string()),
			("right_palette", ToolParamSchema::string()),
			("right_reverse_palette", ToolParamSchema::bool()),
			("height", ToolParamSchema::string()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::Text)),
		])),
		"image_stack_profile" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			(
				"points",
				ToolParamSchema::input_vector(ToolVectorGeometry::Point),
			),
			("output_html", ToolParamSchema::output(wbcore::ToolDatasetSchema::Text)),
		])),
		"evaluate_training_sites" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			(
				"training_data",
				ToolParamSchema::input_vector(ToolVectorGeometry::Polygon),
			),
			("class_field", ToolParamSchema::string()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::Text)),
		])),
		"generalize_classified_raster" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("min_size", ToolParamSchema::scalar_integer()),
			("method", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
		])),
		"generalize_with_similarity" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			(
				"similarity",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("min_size", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"image_segmentation" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("auto_reproject", ToolParamSchema::bool()),
			("auto_reproject_method", ToolParamSchema::string()),
			("threshold", ToolParamSchema::scalar_float()),
			("steps", ToolParamSchema::scalar_integer()),
			("min_area", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"bilateral_filter" | "high_pass_bilateral_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("sigma_dist", ToolParamSchema::scalar_float()),
			("sigma_int", ToolParamSchema::scalar_float()),
			("treat_as_rgb", ToolParamSchema::bool()),
			("assume_three_band_rgb", ToolParamSchema::bool()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
		])),
		"gaussian_filter" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("sigma", ToolParamSchema::scalar_float()),
			("treat_as_rgb", ToolParamSchema::bool()),
			("assume_three_band_rgb", ToolParamSchema::bool()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
		])),
		"panchromatic_sharpening" => Some(param_schema_map(&[
			("red", ToolParamSchema::input_raster()),
			("green", ToolParamSchema::input_raster()),
			("blue", ToolParamSchema::input_raster()),
			("composite", ToolParamSchema::input_raster()),
			("pan", ToolParamSchema::input_raster()),
			("method", ToolParamSchema::string()),
			("output_mode", ToolParamSchema::string()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
		])),
		"minimum_noise_fraction" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("num_components", ToolParamSchema::scalar_integer()),
			("noise_mode", ToolParamSchema::string()),
			("auto_reproject", ToolParamSchema::bool()),
			("auto_reproject_method", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
			("output_inverse", ToolParamSchema::output_raster()),
		])),
		"ndvi_based_emissivity" => Some(param_schema_map(&[
			("red_input", ToolParamSchema::input_raster()),
			("nir_input", ToolParamSchema::input_raster()),
			("ndvi_soil", ToolParamSchema::scalar_float()),
			("ndvi_vegetation", ToolParamSchema::scalar_float()),
			("emissivity_soil", ToolParamSchema::scalar_float()),
			("emissivity_vegetation", ToolParamSchema::scalar_float()),
			("auto_reproject", ToolParamSchema::bool()),
			("auto_reproject_method", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
		])),
		"spectral_angle_mapper" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("endmembers", ToolParamSchema::string()),
			("threshold_angle_deg", ToolParamSchema::scalar_float()),
			("auto_reproject", ToolParamSchema::bool()),
			("auto_reproject_method", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
			("output_angle", ToolParamSchema::output_raster()),
		])),
		"spectral_library_matching" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("library", ToolParamSchema::string()),
			(
				"library_csv",
				ToolParamSchema::input(wbcore::ToolDatasetSchema::Table),
			),
			("metric", ToolParamSchema::string()),
			("auto_reproject", ToolParamSchema::bool()),
			("auto_reproject_method", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
			("output_score", ToolParamSchema::output_raster()),
		])),
		"yamaguchi_4component_decomposition" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("matrix_format", ToolParamSchema::string()),
			("auto_reproject", ToolParamSchema::bool()),
			("auto_reproject_method", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
		])),
		"wisart_iterative_clustering" => Some(param_schema_map(&[
			("h_raster", ToolParamSchema::input_raster()),
			("alpha_raster", ToolParamSchema::input_raster()),
			("max_iterations", ToolParamSchema::scalar_integer()),
			("convergence_threshold", ToolParamSchema::scalar_float()),
			("auto_reproject", ToolParamSchema::bool()),
			("auto_reproject_method", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
		])),
		"random_forest_classification_fit" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			(
				"training_data",
				ToolParamSchema::input_vector(ToolVectorGeometry::Any),
			),
			("class_field", ToolParamSchema::field("training_data", None)),
			("scaling", ToolParamSchema::string()),
			("split_criterion", ToolParamSchema::string()),
			("n_trees", ToolParamSchema::scalar_integer()),
			("min_samples_leaf", ToolParamSchema::scalar_integer()),
			("min_samples_split", ToolParamSchema::scalar_integer()),
			("test_proportion", ToolParamSchema::scalar_float()),
		])),
		"random_forest_classification_predict" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("model_bytes", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
		])),
		"random_forest_regression_fit" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			(
				"training_data",
				ToolParamSchema::input_vector(ToolVectorGeometry::Any),
			),
			("field", ToolParamSchema::field("training_data", None)),
			("scaling", ToolParamSchema::string()),
			("n_trees", ToolParamSchema::scalar_integer()),
			("min_samples_leaf", ToolParamSchema::scalar_integer()),
			("min_samples_split", ToolParamSchema::scalar_integer()),
			("test_proportion", ToolParamSchema::scalar_float()),
		])),
		"random_forest_regression_predict" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("model_bytes", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
		])),
		"svm_classification" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("auto_reproject", ToolParamSchema::bool()),
			("auto_reproject_method", ToolParamSchema::string()),
			(
				"training_data",
				ToolParamSchema::input_vector(ToolVectorGeometry::Any),
			),
			("class_field", ToolParamSchema::field("training_data", None)),
			("scaling", ToolParamSchema::string()),
			("kernel", ToolParamSchema::string()),
			("c", ToolParamSchema::scalar_float()),
			("gamma", ToolParamSchema::scalar_float()),
			("epoch", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"svm_regression" => Some(param_schema_map(&[
			(
				"inputs",
				ToolParamSchema::input_multiple(wbcore::ToolDatasetSchema::Raster),
			),
			("auto_reproject", ToolParamSchema::bool()),
			("auto_reproject_method", ToolParamSchema::string()),
			(
				"training_data",
				ToolParamSchema::input_vector(ToolVectorGeometry::Any),
			),
			("field", ToolParamSchema::field("training_data", None)),
			("scaling", ToolParamSchema::string()),
			("kernel", ToolParamSchema::string()),
			("c", ToolParamSchema::scalar_float()),
			("gamma", ToolParamSchema::scalar_float()),
			("eps", ToolParamSchema::scalar_float()),
			("tol", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		_ => None,
	}
}

pub use advanced_filters::AnisotropicDiffusionFilterTool;
pub use advanced_filters::FrostFilterTool;
pub use advanced_filters::FrangiFilterTool;
pub use advanced_filters::GaborFilterBankTool;
pub use advanced_filters::GammaMapFilterTool;
pub use advanced_filters::GammaCorrectionTool;
pub use advanced_filters::GuidedFilterTool;
pub use advanced_filters::KuanFilterTool;
pub use advanced_filters::KuwaharaFilterTool;
pub use advanced_filters::NonLocalMeansFilterTool;
pub use advanced_filters::SavitzkyGolay2dFilterTool;
pub use advanced_filters::WienerFilterTool;
pub use bilateral_filter::BilateralFilterTool;
pub use bilateral_filter::HighPassBilateralFilterTool;
pub use convolution_extra_filters::EmbossFilterTool;
pub use convolution_filters::HighPassFilterTool;
pub use convolution_filters::LaplacianFilterTool;
pub use convolution_extra_filters::LineDetectionFilterTool;
pub use convolution_filters::PrewittFilterTool;
pub use convolution_extra_filters::RobertsCrossFilterTool;
pub use convolution_extra_filters::ScharrFilterTool;
pub use convolution_filters::SobelFilterTool;
pub use convolution_extra_filters::UserDefinedWeightsFilterTool;
pub use rank_filters::DiversityFilterTool;
pub use gaussian_filter::GaussianFilterTool;
pub use phase3_filters::DiffOfGaussiansFilterTool;
pub use phase3_filters::EdgePreservingMeanFilterTool;
pub use phase3_filters::FastAlmostGaussianFilterTool;
pub use phase3_filters::AdaptiveFilterTool;
pub use phase3_filters::LeeFilterTool;
pub use phase3_filters::RefinedLeeFilterTool;
pub use phase3_filters::EnhancedLeeFilterTool;
pub use phase3_filters::ConservativeSmoothingFilterTool;
pub use phase3_filters::OlympicFilterTool;
pub use phase3_filters::KNearestMeanFilterTool;
pub use phase3_filters::HighPassMedianFilterTool;
pub use phase3_filters::LaplacianOfGaussiansFilterTool;
pub use non_filter_tools::BalanceContrastEnhancementTool;
pub use non_filter_tools::ChangeVectorAnalysisTool;
pub use non_filter_tools::ClosingTool;
pub use non_filter_tools::CornerDetectionTool;
pub use non_filter_tools::CreateColourCompositeTool;
pub use non_filter_tools::DirectDecorrelationStretchTool;
pub use non_filter_tools::FlipImageTool;
pub use non_filter_tools::HistogramEqualizationTool;
pub use non_filter_tools::HistogramMatchingTool;
pub use non_filter_tools::HistogramMatchingTwoImagesTool;
pub use non_filter_tools::IntegralImageTransformTool;
pub use non_filter_tools::GaussianContrastStretchTool;
pub use non_filter_tools::MinMaxContrastStretchTool;
pub use non_filter_tools::NormalizedDifferenceIndexTool;
pub use non_filter_tools::PercentageContrastStretchTool;
pub use non_filter_tools::RemoveSpursTool;
pub use non_filter_tools::SigmoidalContrastStretchTool;
pub use non_filter_tools::StandardDeviationContrastStretchTool;
pub use non_filter_tools::ThickenRasterLineTool;
pub use non_filter_tools::TophatTransformTool;
pub use non_filter_tools::LineThinningTool;
pub use non_filter_tools::IhsToRgbTool;
pub use non_filter_tools::RgbToIhsTool;
pub use non_filter_tools::SplitColourCompositeTool;
pub use non_filter_tools::WriteFunctionMemoryInsertionTool;
pub use rank_filters::MajorityFilterTool;
pub use rank_filters::MedianFilterTool;
pub use rank_filters::PercentileFilterTool;
pub use phase3_filters::UnsharpMaskingTool;
pub use window_stats_filters::MaximumFilterTool;
pub use window_stats_filters::MeanFilterTool;
pub use window_stats_filters::MinimumFilterTool;
pub use window_stats_filters::RangeFilterTool;
pub use window_stats_filters::StandardDeviationFilterTool;
pub use window_stats_filters::TotalFilterTool;
pub use non_filter_tools::OpeningTool;
pub use non_filter_tools::PanchromaticSharpeningTool;
pub use non_filter_tools::MosaicTool;
pub use non_filter_tools::MosaicWithFeatheringTool;
pub use non_filter_tools::KMeansClusteringTool;
pub use non_filter_tools::ModifiedKMeansClusteringTool;
pub use non_filter_tools::CorrectVignettingTool;
pub use non_filter_tools::ImageSliderTool;
pub use non_filter_tools::ImageStackProfileTool;
pub use non_filter_tools::PiecewiseContrastStretchTool;
pub use non_filter_tools::ResampleTool;
pub use non_filter_tools::GeneralizeClassifiedRasterTool;
pub use non_filter_tools::OtsuThresholdingTool;
pub use non_filter_tools::MinDistClassificationTool;
pub use non_filter_tools::ParallelepipedClassificationTool;
pub use non_filter_tools::CannyEdgeDetectionTool;
pub use non_filter_tools::EvaluateTrainingSitesTool;
pub use non_filter_tools::GeneralizeWithSimilarityTool;
pub use non_filter_tools::ImageSegmentationTool;
pub use texture_glcm_tool::GlcmTextureTool;
pub use non_filter_tools::KnnClassificationTool;
pub use non_filter_tools::KnnRegressionTool;
pub use non_filter_tools::FuzzyKnnClassificationTool;
pub use non_filter_tools::RandomForestClassificationTool;
pub use non_filter_tools::RandomForestRegressionTool;
pub use non_filter_tools::RandomForestClassificationFitTool;
pub use non_filter_tools::RandomForestClassificationPredictTool;
pub use non_filter_tools::RandomForestRegressionFitTool;
pub use non_filter_tools::RandomForestRegressionPredictTool;
pub use non_filter_tools::SvmClassificationTool;
pub use non_filter_tools::SvmRegressionTool;
pub use non_filter_tools::LogisticRegressionTool;
pub use non_filter_tools::NndClassificationTool;
pub use radiometric_tools::DarkObjectSubtractionTool;
pub use radiometric_tools::DnToToaReflectanceTool;
pub use terrain_corrected_optical_analytics::TerrainCorrectedOpticalTool;
pub use brdf_normalization::BrdfNormalizationTool;
pub use georeference_raster_from_control_points::GeoreferenceRasterFromControlPointsTool;
pub use orthorectification::OrthorectificationTool;
pub use radiometric_tools::CloudePottierDecompositionTool;
pub use radiometric_tools::ContinuumRemovalTool;
pub use radiometric_tools::FreemanDurdenDecompositionTool;
pub use radiometric_tools::ImageDifferenceChangeDetectionTool;
pub use radiometric_tools::LandSurfaceTemperatureSingleChannelTool;
pub use radiometric_tools::LandSurfaceTemperatureSplitWindowTool;
pub use radiometric_tools::LinearSpectralUnmixingTool;
pub use radiometric_tools::MinimumNoiseFractionTool;
pub use radiometric_tools::NdviBasedEmissivityTool;
pub use radiometric_tools::PcaBasedChangeDetectionTool;
pub use radiometric_tools::PostClassificationChangeTool;
pub use radiometric_tools::SpectralAngleMapperTool;
pub use radiometric_tools::SpectralLibraryMatchingTool;
pub use radiometric_tools::YamaguchiDecompositionTool;
pub use radiometric_tools::HAlphaWisartClassificationTool;
pub use radiometric_tools::WishartIterativeClusteringTool;
pub use obia_tools::SegmentSlicSuperpixelsTool;
pub use obia_tools::SegmentGraphFelzenszwalbTool;
pub use obia_tools::SegmentWatershedMarkersTool;
pub use obia_tools::SegmentMultiresolutionHierarchicalTool;
pub use obia_tools::SegmentScaleParameterOptimizerTool;
pub use obia_tools::SegmentsMergeSmallRegionsTool;
pub use obia_tools::SegmentsSplitLowCohesionTool;
pub use obia_tools::SegmentsToPolygonsTool;
pub use obia_tools::PolygonsToSegmentsTool;
pub use obia_tools::ObjectFeaturesSpectralBasicTool;
pub use obia_tools::ObjectFeaturesShapeBasicTool;
pub use obia_tools::ObjectFeaturesTextureGlcmBasicTool;
pub use obia_tools::ObjectFeaturesContextNeighborsTool;
pub use obia_tools::ObjectFeaturesTopologyRelationsTool;
pub use obia_tools::ClassifyObjectsRandomForestTool;
pub use obia_tools::ClassifyObjectsSvmTool;
pub use obia_tools::ClassifyObjectsEnsembleProTool;
pub use obia_tools::ClassifyObjectsRulesBasicTool;
pub use obia_tools::ClassifyObjectsRulesHierarchicalTool;
pub use obia_tools::ObjectClassProbabilityMapsTool;
pub use obia_tools::ObjectUncertaintyDiagnosticsProTool;
pub use obia_tools::EvaluateObjectClassificationAccuracyTool;
pub use obia_tools::BuildObjectHierarchyMultiscaleTool;
pub use obia_tools::PropagateLabelsAcrossHierarchyTool;
pub use obia_tools::ObjectsEnforceMinMappingUnitTool;
pub use obia_tools::ObjectsBoundaryRefinementProTool;
pub use obia_tools::EvaluateSegmentationQualityProTool;
pub use obia_tools::ObiaPipelineBasicTool;
pub use obia_tools::ObiaBatchOrchestratorProTool;
pub use obia_tools::ObiaAuditReportProTool;