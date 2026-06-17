use std::collections::BTreeMap;
use wbcore::ToolParamSchema;
use wbcore::ToolVectorGeometry;

fn param_schema_map(entries: &[(&str, ToolParamSchema)]) -> BTreeMap<String, ToolParamSchema> {
	let mut map = BTreeMap::new();
	for (name, schema) in entries {
		map.insert((*name).to_string(), schema.clone());
	}
	map
}

pub fn geomorphometry_tool_param_schemas(
	tool_id: &str,
) -> Option<BTreeMap<String, ToolParamSchema>> {
	match tool_id {
		"plan_curvature" | "profile_curvature" | "tangential_curvature"
		| "total_curvature" | "mean_curvature" | "gaussian_curvature"
		| "minimal_curvature" | "maximal_curvature" | "shape_index"
		| "curvedness" | "difference_curvature" | "accumulation_curvature"
		| "unsphericity" | "ring_curvature" | "rotor"
		| "horizontal_excess_curvature" | "vertical_excess_curvature"
		| "generating_function" | "principal_curvature_direction"
		| "casorati_curvature" => {
			Some(param_schema_map(&[
				("input", ToolParamSchema::input_raster()),
				("z_factor", ToolParamSchema::scalar_float()),
				("log_transform", ToolParamSchema::bool()),
				(
					"output",
					ToolParamSchema::output(wbcore::ToolDatasetSchema::File),
				),
			]))
		}
		"slope" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("units", ToolParamSchema::string()),
			("z_factor", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
		])),
		"hillshade" | "multidirectional_hillshade" => Some(param_schema_map(&[])),
		"aspect" | "convergence_index" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("z_factor", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"openness" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("dist", ToolParamSchema::scalar_integer()),
			("pos_output", ToolParamSchema::output_raster()),
			("neg_output", ToolParamSchema::output_raster()),
		])),
		"ruggedness_index" | "surface_area_ratio" | "elev_relative_to_min_max" => {
			Some(param_schema_map(&[
				("input", ToolParamSchema::input_raster()),
				(
					"output",
					ToolParamSchema::output(wbcore::ToolDatasetSchema::File),
				),
			]))
		}
		"percent_elev_range" | "relative_topographic_position" => {
			Some(param_schema_map(&[
				("input", ToolParamSchema::input_raster()),
				("filter_size_x", ToolParamSchema::scalar_integer()),
				("filter_size_y", ToolParamSchema::scalar_integer()),
				("output", ToolParamSchema::output_raster()),
			]))
		}
		"circular_variance_of_aspect" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("filter", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"viewshed" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			(
				"stations",
				ToolParamSchema::input_vector(ToolVectorGeometry::Point),
			),
			("height", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"slope_vs_aspect_plot" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("aspect_bin_size", ToolParamSchema::scalar_float()),
			("min_slope", ToolParamSchema::scalar_float()),
			("z_factor", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
		])),
		"slope_vs_elev_plot" => Some(param_schema_map(&[
			("inputs", ToolParamSchema::string()),
			("watershed", ToolParamSchema::string()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
		])),
		"hypsometric_analysis" => Some(param_schema_map(&[
			("inputs", ToolParamSchema::string()),
			("watershed", ToolParamSchema::string()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
		])),
		"profile" => Some(param_schema_map(&[
			(
				"lines_vector",
				ToolParamSchema::input_vector(ToolVectorGeometry::Line),
			),
			("surface", ToolParamSchema::input_raster()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
		])),
		"assess_route" => Some(param_schema_map(&[
			("routes", ToolParamSchema::input_vector(ToolVectorGeometry::Line)),
			("dem", ToolParamSchema::input_raster()),
			("segment_length", ToolParamSchema::scalar_float()),
			("search_radius", ToolParamSchema::scalar_integer()),
			(
				"output",
				ToolParamSchema::output(wbcore::ToolDatasetSchema::Vector {
					geometry: ToolVectorGeometry::Line,
				}),
			),
		])),
		"geomorphons" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("search_distance", ToolParamSchema::scalar_integer()),
			("flatness_threshold", ToolParamSchema::scalar_float()),
			("flatness_distance", ToolParamSchema::scalar_integer()),
			("skip_distance", ToolParamSchema::scalar_integer()),
			("output_forms", ToolParamSchema::bool()),
			("analyze_residuals", ToolParamSchema::bool()),
			("output", ToolParamSchema::output_raster()),
		])),
		"elevation_percentile" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("filter_size_x", ToolParamSchema::scalar_integer()),
			("filter_size_y", ToolParamSchema::scalar_integer()),
			("sig_digits", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
		])),
		"num_downslope_neighbours" | "num_upslope_neighbours" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("output", ToolParamSchema::output_raster()),
		])),
		"max_downslope_elev_change" | "max_upslope_elev_change"
		| "min_downslope_elev_change" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("output", ToolParamSchema::output_raster()),
		])),
		"max_branch_length" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("log_transform", ToolParamSchema::bool()),
			("output", ToolParamSchema::output_raster()),
		])),
		"find_ridges" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("line_thin", ToolParamSchema::bool()),
			("output", ToolParamSchema::output_raster()),
		])),
		"elev_above_pit" | "elev_above_pit_dist" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("output", ToolParamSchema::output_raster()),
		])),
		"directional_relief" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("azimuth", ToolParamSchema::scalar_float()),
			("max_dist", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"exposure_towards_wind_flux" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("azimuth", ToolParamSchema::scalar_float()),
			("max_dist", ToolParamSchema::scalar_float()),
			("z_factor", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"relative_aspect" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("azimuth", ToolParamSchema::scalar_float()),
			("z_factor", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"fetch_analysis" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("azimuth", ToolParamSchema::scalar_float()),
			("hgt_inc", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"dem_void_filling" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("fill", ToolParamSchema::input_raster()),
			("mean_plane_dist", ToolParamSchema::scalar_integer()),
			("edge_treatment", ToolParamSchema::string()),
			("weight_value", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"fill_missing_data" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("filter_size", ToolParamSchema::scalar_integer()),
			("weight", ToolParamSchema::scalar_float()),
			("exclude_edge_nodata", ToolParamSchema::bool()),
			("output", ToolParamSchema::output_raster()),
		])),
		"downslope_index" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("vertical_drop", ToolParamSchema::scalar_float()),
			("output_type", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
		])),
		"wetness_index" => Some(param_schema_map(&[
			("sca", ToolParamSchema::input_raster()),
			("slope", ToolParamSchema::input_raster()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
		])),
		"relative_stream_power_index" => Some(param_schema_map(&[
			("sca", ToolParamSchema::input_raster()),
			("slope", ToolParamSchema::input_raster()),
			("exponent", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
		])),
		"sediment_transport_index" => Some(param_schema_map(&[
			("sca", ToolParamSchema::input_raster()),
			("slope", ToolParamSchema::input_raster()),
			("sca_exponent", ToolParamSchema::scalar_float()),
			("slope_exponent", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
		])),
		"difference_from_mean_elevation" | "deviation_from_mean_elevation" => {
			Some(param_schema_map(&[
				("input", ToolParamSchema::input_raster()),
				("filter_size_x", ToolParamSchema::scalar_integer()),
				("filter_size_y", ToolParamSchema::scalar_integer()),
				("output", ToolParamSchema::output_raster()),
			]))
		}
		"max_difference_from_mean" | "max_anisotropy_dev" | "multiscale_roughness" => {
			Some(param_schema_map(&[
				("input", ToolParamSchema::input_raster()),
				("min_scale", ToolParamSchema::scalar_float()),
				("max_scale", ToolParamSchema::scalar_float()),
				("step_size", ToolParamSchema::scalar_float()),
				("z_factor", ToolParamSchema::scalar_float()),
				("output", ToolParamSchema::output_raster()),
				("output_scale", ToolParamSchema::output_raster()),
			]))
		}
		"max_elevation_deviation" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("min_scale", ToolParamSchema::scalar_float()),
			("max_scale", ToolParamSchema::scalar_float()),
			("step_size", ToolParamSchema::scalar_float()),
			("min_vertical", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
			("output_scale", ToolParamSchema::output_raster()),
		])),
		"multiscale_elevation_percentile" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("min_scale", ToolParamSchema::scalar_float()),
			("num_steps", ToolParamSchema::scalar_integer()),
			("step_size", ToolParamSchema::scalar_float()),
			("step_nonlinearity", ToolParamSchema::scalar_float()),
			("sig_digits", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
			("output_scale", ToolParamSchema::output_raster()),
		])),
		"multiscale_std_dev_normals" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("min_scale", ToolParamSchema::scalar_float()),
			("step", ToolParamSchema::scalar_float()),
			("num_steps", ToolParamSchema::scalar_integer()),
			("step_nonlinearity", ToolParamSchema::scalar_float()),
			("z_factor", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
			("output_scale", ToolParamSchema::output_raster()),
		])),
		"multiscale_roughness_signature" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			(
				"points",
				ToolParamSchema::input_vector(ToolVectorGeometry::Point),
			),
			("min_scale", ToolParamSchema::scalar_float()),
			("max_scale", ToolParamSchema::scalar_float()),
			("step_size", ToolParamSchema::scalar_float()),
			("z_factor", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
		])),
		"multiscale_std_dev_normals_signature" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			(
				"points",
				ToolParamSchema::input_vector(ToolVectorGeometry::Point),
			),
			("min_scale", ToolParamSchema::scalar_float()),
			("step", ToolParamSchema::scalar_float()),
			("num_steps", ToolParamSchema::scalar_integer()),
			("step_nonlinearity", ToolParamSchema::scalar_float()),
			("z_factor", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
		])),
		"max_elev_dev_signature" | "max_anisotropy_dev_signature" => {
			Some(param_schema_map(&[
				("input", ToolParamSchema::input_raster()),
				(
					"points",
					ToolParamSchema::input_vector(ToolVectorGeometry::Point),
				),
				("min_scale", ToolParamSchema::scalar_float()),
				("max_scale", ToolParamSchema::scalar_float()),
				("step_size", ToolParamSchema::scalar_float()),
				("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
			]))
		}
		"multiscale_elevated_index" | "multiscale_low_lying_index" => {
			Some(param_schema_map(&[
				("input", ToolParamSchema::input_raster()),
				("min_scale", ToolParamSchema::scalar_float()),
				("step_size", ToolParamSchema::scalar_float()),
				("num_steps", ToolParamSchema::scalar_integer()),
				("step_nonlinearity", ToolParamSchema::scalar_float()),
				("output", ToolParamSchema::output_raster()),
				("output_scale", ToolParamSchema::output_raster()),
			]))
		}
		"local_hypsometric_analysis" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("min_scale", ToolParamSchema::scalar_float()),
			("step_size", ToolParamSchema::scalar_float()),
			("num_steps", ToolParamSchema::scalar_integer()),
			("step_nonlinearity", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
			("output_scale", ToolParamSchema::output_raster()),
		])),
		"multiscale_curvatures" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("curv_type", ToolParamSchema::string()),
			("out_mag", ToolParamSchema::output_raster()),
			("out_scale", ToolParamSchema::output_raster()),
			("min_scale", ToolParamSchema::scalar_float()),
			("step", ToolParamSchema::scalar_float()),
			("num_steps", ToolParamSchema::scalar_integer()),
			("step_nonlinearity", ToolParamSchema::scalar_float()),
			("log_transform", ToolParamSchema::bool()),
			("standardize", ToolParamSchema::bool()),
		])),
		"multiscale_topographic_position_image" => Some(param_schema_map(&[
			("local", ToolParamSchema::input_raster()),
			("meso", ToolParamSchema::input_raster()),
			("broad", ToolParamSchema::input_raster()),
			("hillshade", ToolParamSchema::input_raster()),
			("lightness", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"standard_deviation_of_slope" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("filter_size", ToolParamSchema::scalar_integer()),
			("filter_size_x", ToolParamSchema::scalar_integer()),
			("filter_size_y", ToolParamSchema::scalar_integer()),
			("z_factor", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"edge_density" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("filter_size", ToolParamSchema::scalar_integer()),
			("norm_diff", ToolParamSchema::scalar_float()),
			("z_factor", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"spherical_std_dev_of_normals" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("filter_size", ToolParamSchema::scalar_integer()),
			("z_factor", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"average_normal_vector_angular_deviation" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("filter_size", ToolParamSchema::scalar_integer()),
			("z_factor", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"pennock_landform_classification" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("slope_threshold", ToolParamSchema::scalar_float()),
			("prof_curv_threshold", ToolParamSchema::scalar_float()),
			("plan_curv_threshold", ToolParamSchema::scalar_float()),
			("z_factor", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"breakline_mapping" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("threshold", ToolParamSchema::scalar_float()),
			("min_length", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_vector_any()),
		])),
		"horizon_angle" => Some(param_schema_map(&[
			("dem", ToolParamSchema::input_raster()),
			("azimuth", ToolParamSchema::scalar_float()),
			("max_dist", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"hypsometrically_tinted_hillshade" => Some(param_schema_map(&[
			("dem", ToolParamSchema::input_raster()),
			("solar_altitude", ToolParamSchema::scalar_float()),
			("hillshade_weight", ToolParamSchema::scalar_float()),
			("brightness", ToolParamSchema::scalar_float()),
			("atmospheric_effects", ToolParamSchema::bool()),
			("palette", ToolParamSchema::string()),
			("reverse_palette", ToolParamSchema::bool()),
			("full_360_mode", ToolParamSchema::bool()),
			("z_factor", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"map_off_terrain_objects" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("max_slope", ToolParamSchema::scalar_float()),
			("min_feature_size", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
		])),
		"remove_off_terrain_objects" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("filter_size", ToolParamSchema::scalar_integer()),
			("slope_threshold", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
		])),
		"embankment_mapping" => Some(param_schema_map(&[
			("dem", ToolParamSchema::input_raster()),
			(
				"roads_vector",
				ToolParamSchema::input_vector(ToolVectorGeometry::Line),
			),
			("search_dist", ToolParamSchema::scalar_float()),
			("min_road_width", ToolParamSchema::scalar_float()),
			("typical_embankment_width", ToolParamSchema::scalar_float()),
			(
				"typical_embankment_max_height",
				ToolParamSchema::scalar_float(),
			),
			("embankment_max_width", ToolParamSchema::scalar_float()),
			("max_upwards_increment", ToolParamSchema::scalar_float()),
			("spillout_slope", ToolParamSchema::scalar_float()),
			("remove_embankments", ToolParamSchema::bool()),
			("output", ToolParamSchema::output_raster()),
			("output_dem", ToolParamSchema::output_raster()),
		])),
		"smooth_vegetation_residual" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("max_scale", ToolParamSchema::scalar_float()),
			("dev_threshold", ToolParamSchema::scalar_float()),
			("scale_threshold", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
		])),
		"low_points_on_headwater_divides" => Some(param_schema_map(&[
			("dem", ToolParamSchema::input_raster()),
			("streams", ToolParamSchema::input_raster()),
			("output", ToolParamSchema::output_vector_any()),
		])),
		"multiscale_topographic_position_class" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("local_min_scale", ToolParamSchema::scalar_float()),
			("local_max_scale", ToolParamSchema::scalar_float()),
			("local_step_size", ToolParamSchema::scalar_float()),
			("broad_min_scale", ToolParamSchema::scalar_float()),
			("broad_max_scale", ToolParamSchema::scalar_float()),
			("broad_step_size", ToolParamSchema::scalar_float()),
			("local_threshold", ToolParamSchema::scalar_float()),
			("broad_threshold", ToolParamSchema::scalar_float()),
			("min_patch_size", ToolParamSchema::scalar_integer()),
			("output", ToolParamSchema::output_raster()),
			("output_confidence", ToolParamSchema::output_raster()),
		])),
		"elev_relative_to_watershed_min_max" => Some(param_schema_map(&[
			("dem", ToolParamSchema::input_raster()),
			("watersheds", ToolParamSchema::input_raster()),
			("output", ToolParamSchema::output_raster()),
		])),
		"feature_preserving_smoothing_multiscale" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("smoothing_amount", ToolParamSchema::scalar_float()),
			("edge_preservation", ToolParamSchema::scalar_float()),
			("scale_levels", ToolParamSchema::scalar_integer()),
			("fidelity", ToolParamSchema::scalar_float()),
			("z_factor", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"feature_preserving_smoothing" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("filter_size", ToolParamSchema::scalar_integer()),
			("normal_diff_threshold", ToolParamSchema::scalar_float()),
			("iterations", ToolParamSchema::scalar_integer()),
			("max_elevation_diff", ToolParamSchema::scalar_float()),
			("z_factor", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"feature_preserving_smoothing_poisson" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("normal_smoothing_strength", ToolParamSchema::scalar_float()),
			("edge_sensitivity", ToolParamSchema::scalar_float()),
			("outer_iterations", ToolParamSchema::scalar_integer()),
			("lambda", ToolParamSchema::scalar_float()),
			("z_factor", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"topo_render" => Some(param_schema_map(&[
			("dem", ToolParamSchema::input_raster()),
			("palette", ToolParamSchema::string()),
			("reverse_palette", ToolParamSchema::bool()),
			("azimuth", ToolParamSchema::scalar_float()),
			("altitude", ToolParamSchema::scalar_float()),
			(
				"clipping_polygon",
				ToolParamSchema::input_vector(ToolVectorGeometry::Polygon),
			),
			("background_hgt_offset", ToolParamSchema::scalar_float()),
			("background_clr", ToolParamSchema::string()),
			("attenuation_parameter", ToolParamSchema::scalar_float()),
			("ambient_light", ToolParamSchema::scalar_float()),
			("z_factor", ToolParamSchema::scalar_float()),
			("max_dist", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"sky_view_factor" | "average_horizon_distance" => Some(param_schema_map(&[
			("dem", ToolParamSchema::input_raster()),
			("az_fraction", ToolParamSchema::scalar_float()),
			("max_dist", ToolParamSchema::scalar_float()),
			("observer_hgt_offset", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"visibility_index" => Some(param_schema_map(&[
			("dem", ToolParamSchema::input_raster()),
			("station_height", ToolParamSchema::scalar_float()),
			("resolution_factor", ToolParamSchema::scalar_integer()),
			("max_dist", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"time_in_daylight" => Some(param_schema_map(&[
			("dem", ToolParamSchema::input_raster()),
			("az_fraction", ToolParamSchema::scalar_float()),
			("max_dist", ToolParamSchema::scalar_float()),
			("latitude", ToolParamSchema::scalar_float()),
			("longitude", ToolParamSchema::scalar_float()),
			("utc_offset", ToolParamSchema::string()),
			("start_day", ToolParamSchema::scalar_integer()),
			("end_day", ToolParamSchema::scalar_integer()),
			("start_time", ToolParamSchema::string()),
			("end_time", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
		])),
		"shadow_image" => Some(param_schema_map(&[
			("dem", ToolParamSchema::input_raster()),
			("max_dist", ToolParamSchema::scalar_float()),
			("date", ToolParamSchema::string()),
			("time", ToolParamSchema::string()),
			("location", ToolParamSchema::string()),
			("palette", ToolParamSchema::string()),
			("output", ToolParamSchema::output_raster()),
		])),
		"horizon_area" => Some(param_schema_map(&[
			("dem", ToolParamSchema::input_raster()),
			("az_fraction", ToolParamSchema::scalar_float()),
			("max_dist", ToolParamSchema::scalar_float()),
			("observer_hgt_offset", ToolParamSchema::scalar_float()),
			("output", ToolParamSchema::output_raster()),
		])),
		"shadow_animation" => Some(param_schema_map(&[
			("dem", ToolParamSchema::input_raster()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
			("palette", ToolParamSchema::string()),
			("max_dist", ToolParamSchema::scalar_float()),
			("date", ToolParamSchema::string()),
			("time_interval", ToolParamSchema::scalar_integer()),
			("location", ToolParamSchema::string()),
			("image_height", ToolParamSchema::scalar_integer()),
			("delay", ToolParamSchema::scalar_integer()),
			("label", ToolParamSchema::string()),
		])),
		"topographic_position_animation" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("output", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
			("palette", ToolParamSchema::string()),
			("min_scale", ToolParamSchema::scalar_integer()),
			("num_steps", ToolParamSchema::scalar_integer()),
			("step_nonlinearity", ToolParamSchema::scalar_float()),
			("image_height", ToolParamSchema::scalar_integer()),
			("delay", ToolParamSchema::scalar_integer()),
			("label", ToolParamSchema::string()),
			("use_dev_max", ToolParamSchema::bool()),
		])),
		"skyline_analysis" => Some(param_schema_map(&[
			("dem", ToolParamSchema::input_raster()),
			(
				"points",
				ToolParamSchema::input_vector(ToolVectorGeometry::Point),
			),
			("az_fraction", ToolParamSchema::scalar_float()),
			("max_dist", ToolParamSchema::scalar_float()),
			("observer_hgt_offset", ToolParamSchema::scalar_float()),
			("output_as_polygons", ToolParamSchema::bool()),
			("output", ToolParamSchema::output_vector_any()),
			("output_html", ToolParamSchema::output(wbcore::ToolDatasetSchema::File)),
		])),
		"contours_from_raster" => Some(param_schema_map(&[
			("input", ToolParamSchema::input_raster()),
			("interval", ToolParamSchema::scalar_float()),
			("base", ToolParamSchema::scalar_float()),
			("smooth", ToolParamSchema::scalar_integer()),
			("tolerance", ToolParamSchema::scalar_float()),
			(
				"output",
				ToolParamSchema::output(wbcore::ToolDatasetSchema::Vector {
					geometry: ToolVectorGeometry::Line,
				}),
			),
		])),
		"contours_from_points" => Some(param_schema_map(&[
			(
				"input",
				ToolParamSchema::input_vector(ToolVectorGeometry::Point),
			),
			("field_name", ToolParamSchema::field("input", None)),
			("use_z_values", ToolParamSchema::bool()),
			("max_triangle_edge_length", ToolParamSchema::scalar_float()),
			("interval", ToolParamSchema::scalar_float()),
			("base", ToolParamSchema::scalar_float()),
			("smooth", ToolParamSchema::scalar_integer()),
			(
				"output",
				ToolParamSchema::output(wbcore::ToolDatasetSchema::Vector {
					geometry: ToolVectorGeometry::Line,
				}),
			),
		])),
		"topographic_hachures" => Some(param_schema_map(&[
			("dem", ToolParamSchema::input_raster()),
			("interval", ToolParamSchema::scalar_float()),
			("base", ToolParamSchema::scalar_float()),
			("tolerance", ToolParamSchema::scalar_float()),
			("smooth", ToolParamSchema::scalar_integer()),
			("separation", ToolParamSchema::scalar_float()),
			("distmin", ToolParamSchema::scalar_float()),
			("distmax", ToolParamSchema::scalar_float()),
			("discretization", ToolParamSchema::scalar_float()),
			("turnmax", ToolParamSchema::scalar_float()),
			("slopemin", ToolParamSchema::scalar_float()),
			("depth", ToolParamSchema::scalar_integer()),
			(
				"output",
				ToolParamSchema::output(wbcore::ToolDatasetSchema::Vector {
					geometry: ToolVectorGeometry::Line,
				}),
			),
		])),
		_ => None,
	}
}

mod curvature_tools;
mod pro_curvature_tools;
mod basic_terrain_tools;
mod terrain_analysis_tools;
mod hydrologic_index_tools;
mod terrain_window_tools;
mod contour_tools;
mod openness;
mod dem_void_filling;
mod multiscale_curvatures;

mod sky_visibility_tools;

pub use curvature_tools::PlanCurvatureTool;
pub use curvature_tools::ProfileCurvatureTool;
pub use curvature_tools::TangentialCurvatureTool;
pub use curvature_tools::TotalCurvatureTool;
pub use curvature_tools::MeanCurvatureTool;
pub use curvature_tools::GaussianCurvatureTool;
pub use pro_curvature_tools::MinimalCurvatureTool;
pub use pro_curvature_tools::MaximalCurvatureTool;
pub use pro_curvature_tools::ShapeIndexTool;
pub use pro_curvature_tools::CurvednessTool;
pub use pro_curvature_tools::UnsphericityCurvatureTool;
pub use pro_curvature_tools::RingCurvatureTool;
pub use pro_curvature_tools::RotorTool;
pub use pro_curvature_tools::DifferenceCurvatureTool;
pub use pro_curvature_tools::HorizontalExcessCurvatureTool;
pub use pro_curvature_tools::VerticalExcessCurvatureTool;
pub use pro_curvature_tools::AccumulationCurvatureTool;
pub use pro_curvature_tools::GeneratingFunctionTool;
pub use pro_curvature_tools::PrincipalCurvatureDirectionTool;
pub use pro_curvature_tools::CasoratiCurvatureTool;
pub use basic_terrain_tools::SlopeTool;
pub use basic_terrain_tools::AspectTool;
pub use basic_terrain_tools::ConvergenceIndexTool;
pub use basic_terrain_tools::HillshadeTool;
pub use basic_terrain_tools::MultidirectionalHillshadeTool;
pub use basic_terrain_tools::slope_aspect_from_dem;
pub use terrain_analysis_tools::RuggednessIndexTool;
pub use terrain_analysis_tools::SurfaceAreaRatioTool;
pub use terrain_analysis_tools::ElevRelativeToMinMaxTool;
pub use terrain_analysis_tools::WetnessIndexTool;
pub use terrain_analysis_tools::PercentElevRangeTool;
pub use terrain_analysis_tools::RelativeTopographicPositionTool;
pub use terrain_analysis_tools::NumDownslopeNeighboursTool;
pub use terrain_analysis_tools::NumUpslopeNeighboursTool;
pub use terrain_analysis_tools::MaxDownslopeElevChangeTool;
pub use terrain_analysis_tools::MaxUpslopeElevChangeTool;
pub use terrain_analysis_tools::MinDownslopeElevChangeTool;
pub use terrain_analysis_tools::ElevationPercentileTool;
pub use terrain_analysis_tools::DownslopeIndexTool;
pub use terrain_analysis_tools::MaxBranchLengthTool;
pub use terrain_analysis_tools::ElevAbovePitTool;
pub use terrain_analysis_tools::DirectionalReliefTool;
pub use terrain_analysis_tools::ExposureTowardsWindFluxTool;
pub use terrain_analysis_tools::RelativeAspectTool;
pub use terrain_analysis_tools::EdgeDensityTool;
pub use terrain_analysis_tools::SphericalStdDevOfNormalsTool;
pub use terrain_analysis_tools::AverageNormalVectorAngularDeviationTool;
pub use terrain_analysis_tools::HypsometricAnalysisTool;
pub use terrain_analysis_tools::ProfileTool;
pub use terrain_window_tools::MultiscaleTopographicPositionClassTool;
pub use terrain_analysis_tools::SlopeVsAspectPlotTool;
pub use terrain_analysis_tools::SlopeVsElevPlotTool;
pub use terrain_analysis_tools::ElevAbovePitDistTool;
pub use terrain_analysis_tools::CircularVarianceOfAspectTool;
pub use terrain_analysis_tools::FetchAnalysisTool;
pub use terrain_analysis_tools::FindRidgesTool;
pub use terrain_analysis_tools::GeomorphonsTool;
pub use terrain_analysis_tools::PennockLandformClassificationTool;
pub use terrain_analysis_tools::ViewshedTool;
pub use terrain_analysis_tools::AssessRouteTool;
pub use terrain_analysis_tools::BreaklineMappingTool;
pub use terrain_analysis_tools::LowPointsOnHeadwaterDividesTool;
pub use contour_tools::ContoursFromRasterTool;
pub use contour_tools::ContoursFromPointsTool;
pub use contour_tools::TopographicHachuresTool;
pub use hydrologic_index_tools::RelativeStreamPowerIndexTool;
pub use hydrologic_index_tools::SedimentTransportIndexTool;
pub use hydrologic_index_tools::ElevRelativeToWatershedMinMaxTool;
pub use terrain_window_tools::DifferenceFromMeanElevationTool;
pub use terrain_window_tools::DeviationFromMeanElevationTool;
pub use terrain_window_tools::StandardDeviationOfSlopeTool;
pub use terrain_window_tools::MaxDifferenceFromMeanTool;
pub use terrain_window_tools::MaxElevationDeviationTool;
pub use terrain_window_tools::TopographicPositionAnimationTool;
pub use terrain_window_tools::MultiscaleTopographicPositionImageTool;
pub use terrain_window_tools::MultiscaleElevationPercentileTool;
pub use terrain_window_tools::MaxAnisotropyDevTool;
pub use terrain_window_tools::MultiscaleRoughnessTool;
pub use terrain_window_tools::MaxElevDevSignatureTool;
pub use terrain_window_tools::MaxAnisotropyDevSignatureTool;
pub use terrain_window_tools::MultiscaleRoughnessSignatureTool;
pub use terrain_window_tools::MultiscaleStdDevNormalsTool;
pub use terrain_window_tools::MultiscaleStdDevNormalsSignatureTool;
pub use terrain_window_tools::FeaturePreservingSmoothingTool;
pub use terrain_window_tools::FeaturePreservingSmoothingMultiscaleTool;
pub use terrain_window_tools::FillMissingDataTool;
pub use terrain_window_tools::RemoveOffTerrainObjectsTool;
pub use terrain_window_tools::MapOffTerrainObjectsTool;
pub use terrain_window_tools::EmbankmentMappingTool;
pub use terrain_window_tools::SmoothVegetationResidualTool;
pub use terrain_window_tools::LocalHypsometricAnalysisTool;
pub use terrain_window_tools::MultiscaleElevatedIndexTool;
pub use terrain_window_tools::MultiscaleLowLyingIndexTool;

pub use sky_visibility_tools::HorizonAngleTool;
pub use sky_visibility_tools::SkyViewFactorTool;
pub use sky_visibility_tools::VisibilityIndexTool;
pub use sky_visibility_tools::HorizonAreaTool;
pub use sky_visibility_tools::AverageHorizonDistanceTool;
pub use sky_visibility_tools::TimeInDaylightTool;
pub use sky_visibility_tools::ShadowImageTool;
pub use sky_visibility_tools::ShadowAnimationTool;
pub use sky_visibility_tools::HypsometricallyTintedHillshadeTool;
pub use sky_visibility_tools::TopoRenderTool;
pub use sky_visibility_tools::SkylineAnalysisTool;
pub use openness::OpennessTool;
pub use dem_void_filling::DemVoidFillingTool;
pub use multiscale_curvatures::MultiscaleCurvaturesTool;
