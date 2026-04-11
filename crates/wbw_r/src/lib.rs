use serde_json::{json, Value};
use parquet::basic::Compression as ParquetCompression;
#[cfg(feature = "pro")]
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use wbcore::{
    generate_wrapper_stub, BindingTarget, ExecuteRequest, LicenseTier, OwnedToolRuntime,
    OwnedToolRuntimeWithCapabilities, RuntimeOptions,
    ProgressSink, ToolArgs, ToolError, ToolManifest, ToolRuntimeBuilder, ToolRuntimeRegistry,
};
use wblicense_core::{
    verify_signed_entitlement_json, EntitlementCapabilities, LicenseError, VerificationKeyStore,
};
use wblidar::e57::E57Reader;
use wblidar::las::LasReader;
use wblidar::ply::PlyReader;
use wblidar::{
    CopcWriteOptions,
    LazWriteOptions,
    LidarFormat,
    LidarWriteOptions,
    PointCloud,
    PointReader,
};
use wbprojection::{epsg_from_srs_reference, identify_epsg_from_wkt, to_ogc_wkt, Crs};
use wbraster::{open_sensor_bundle, open_sensor_bundle_path, OpenedSensorBundle, SafeBundle, SensorBundle};
use wbraster::{GeoTiffCompression, GeoTiffLayout, GeoTiffWriteOptions, Raster, RasterFormat};
use wbtopology::{
    buffer_linestring,
    buffer_point,
    buffer_polygon,
    covered_by,
    covers,
    contains,
    crosses,
    disjoint,
    from_wkt,
    geometry_distance,
    intersects,
    is_valid_polygon,
    make_valid_polygon,
    overlaps,
    relate,
    to_wkt,
    touches,
    within,
    BufferOptions,
    Coord,
    Geometry,
};
use wbvector::VectorFormat;
use wbvector::feature::FieldType;
use wbtools_oss::{register_default_tools as register_default_oss_tools, ToolRegistry as OssRegistry};
#[cfg(feature = "pro")]
use wbtools_pro::{register_default_tools as register_default_pro_tools, ToolRegistry as ProRegistry};

fn to_invalid_request<E: std::fmt::Display>(err: E) -> ToolError {
    ToolError::InvalidRequest(err.to_string())
}

#[derive(Debug, Clone, Default)]
struct OsmPbfReadControls {
    highways_only: Option<bool>,
    named_ways_only: Option<bool>,
    polygons_only: Option<bool>,
    include_tag_keys: Option<Vec<String>>,
    has_fields: bool,
}

#[derive(Debug, Clone, Default)]
struct VectorReadControls {
    strict_format_options: bool,
    osmpbf: OsmPbfReadControls,
}

impl VectorReadControls {
    fn has_osmpbf_controls(&self) -> bool {
        self.osmpbf.has_fields
    }
}

#[derive(Debug, Clone, Default)]
struct GeoParquetWriteControls {
    max_rows_per_group: Option<usize>,
    data_page_size_limit: Option<usize>,
    write_batch_size: Option<usize>,
    data_page_row_count_limit: Option<usize>,
    compression: Option<ParquetCompression>,
    has_fields: bool,
}

#[derive(Debug, Clone, Default)]
struct VectorWriteControls {
    strict_format_options: bool,
    geoparquet: GeoParquetWriteControls,
}

impl VectorWriteControls {
    fn has_geoparquet_controls(&self) -> bool {
        self.geoparquet.has_fields
    }
}

fn parse_parquet_compression(name: &str) -> Option<ParquetCompression> {
    match name.trim().to_ascii_lowercase().as_str() {
        "none" | "off" | "uncompressed" => Some(ParquetCompression::UNCOMPRESSED),
        "snappy" => Some(ParquetCompression::SNAPPY),
        "gzip" | "gz" => Some(ParquetCompression::GZIP(Default::default())),
        "lz4" | "lz4_raw" => Some(ParquetCompression::LZ4_RAW),
        "zstd" | "zstandard" => Some(ParquetCompression::ZSTD(Default::default())),
        "brotli" | "br" => Some(ParquetCompression::BROTLI(Default::default())),
        _ => None,
    }
}

fn parse_usize_option(
    obj: &serde_json::Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<Option<usize>, ToolError> {
    let Some(v) = obj.get(key) else {
        return Ok(None);
    };
    let n = v.as_u64().ok_or_else(|| {
        ToolError::InvalidRequest(format!("{label} must be a positive integer"))
    })?;
    if n == 0 {
        return Err(ToolError::InvalidRequest(format!("{label} must be greater than 0")));
    }
    Ok(Some(n as usize))
}

fn parse_vector_read_controls(options: &Value) -> Result<VectorReadControls, ToolError> {
    if options.is_null() {
        return Ok(VectorReadControls::default());
    }

    let obj = options.as_object().ok_or_else(|| {
        ToolError::InvalidRequest("vector options must be a JSON object".to_string())
    })?;

    let strict_format_options = match obj.get("strict_format_options") {
        Some(Value::Bool(v)) => *v,
        Some(Value::Null) | None => false,
        Some(other) => {
            return Err(ToolError::InvalidRequest(format!(
                "options.strict_format_options must be a boolean when provided, got: {other}"
            )))
        }
    };

    let mut osmpbf = OsmPbfReadControls::default();
    if let Some(osm_val) = obj.get("osmpbf") {
        let osm_obj = osm_val.as_object().ok_or_else(|| {
            ToolError::InvalidRequest("options.osmpbf must be a JSON object".to_string())
        })?;
        osmpbf.has_fields = !osm_obj.is_empty();

        if let Some(v) = osm_obj.get("highways_only") {
            osmpbf.highways_only = Some(v.as_bool().ok_or_else(|| {
                ToolError::InvalidRequest(
                    "options.osmpbf.highways_only must be a boolean".to_string(),
                )
            })?);
        }
        if let Some(v) = osm_obj.get("named_ways_only") {
            osmpbf.named_ways_only = Some(v.as_bool().ok_or_else(|| {
                ToolError::InvalidRequest(
                    "options.osmpbf.named_ways_only must be a boolean".to_string(),
                )
            })?);
        }
        if let Some(v) = osm_obj.get("polygons_only") {
            osmpbf.polygons_only = Some(v.as_bool().ok_or_else(|| {
                ToolError::InvalidRequest(
                    "options.osmpbf.polygons_only must be a boolean".to_string(),
                )
            })?);
        }
        if let Some(v) = osm_obj.get("include_tag_keys") {
            let arr = v.as_array().ok_or_else(|| {
                ToolError::InvalidRequest(
                    "options.osmpbf.include_tag_keys must be an array of strings".to_string(),
                )
            })?;
            let mut keys = Vec::with_capacity(arr.len());
            for item in arr {
                let s = item.as_str().ok_or_else(|| {
                    ToolError::InvalidRequest(
                        "options.osmpbf.include_tag_keys must contain only strings".to_string(),
                    )
                })?;
                if !s.trim().is_empty() {
                    keys.push(s.to_string());
                }
            }
            osmpbf.include_tag_keys = if keys.is_empty() { None } else { Some(keys) };
        }
    }

    Ok(VectorReadControls {
        strict_format_options,
        osmpbf,
    })
}

fn parse_vector_write_controls(options: &Value) -> Result<VectorWriteControls, ToolError> {
    if options.is_null() {
        return Ok(VectorWriteControls::default());
    }

    let obj = options.as_object().ok_or_else(|| {
        ToolError::InvalidRequest("vector options must be a JSON object".to_string())
    })?;

    let strict_format_options = match obj.get("strict_format_options") {
        Some(Value::Bool(v)) => *v,
        Some(Value::Null) | None => false,
        Some(other) => {
            return Err(ToolError::InvalidRequest(format!(
                "options.strict_format_options must be a boolean when provided, got: {other}"
            )))
        }
    };

    let mut geoparquet = GeoParquetWriteControls::default();
    if let Some(gpq_val) = obj.get("geoparquet") {
        let gpq_obj = gpq_val.as_object().ok_or_else(|| {
            ToolError::InvalidRequest("options.geoparquet must be a JSON object".to_string())
        })?;
        geoparquet.has_fields = !gpq_obj.is_empty();

        geoparquet.max_rows_per_group =
            parse_usize_option(gpq_obj, "max_rows_per_group", "options.geoparquet.max_rows_per_group")?;
        geoparquet.data_page_size_limit =
            parse_usize_option(gpq_obj, "data_page_size_limit", "options.geoparquet.data_page_size_limit")?;
        geoparquet.write_batch_size =
            parse_usize_option(gpq_obj, "write_batch_size", "options.geoparquet.write_batch_size")?;
        geoparquet.data_page_row_count_limit = parse_usize_option(
            gpq_obj,
            "data_page_row_count_limit",
            "options.geoparquet.data_page_row_count_limit",
        )?;

        if let Some(v) = gpq_obj.get("compression") {
            let name = v.as_str().ok_or_else(|| {
                ToolError::InvalidRequest(
                    "options.geoparquet.compression must be a string".to_string(),
                )
            })?;
            geoparquet.compression = Some(parse_parquet_compression(name).ok_or_else(|| {
                ToolError::InvalidRequest(format!(
                    "unsupported geoparquet.compression '{name}'. Expected one of: none, snappy, gzip, lz4, zstd, brotli"
                ))
            })?);
        }
    }

    Ok(VectorWriteControls {
        strict_format_options,
        geoparquet,
    })
}

fn read_vector_with_controls(
    src_path: &Path,
    src_format: VectorFormat,
    controls: &VectorReadControls,
) -> Result<wbvector::Layer, ToolError> {
    if controls.has_osmpbf_controls() && src_format != VectorFormat::OsmPbf {
        if controls.strict_format_options {
            return Err(ToolError::InvalidRequest(
                "OSM PBF-specific read options were provided for a non-OSM source path"
                    .to_string(),
            ));
        }
        return wbvector::read(src_path).map_err(to_invalid_request);
    }

    if src_format == VectorFormat::OsmPbf {
        let mut opts = wbvector::osmpbf::OsmPbfReadOptions::new();
        if let Some(v) = controls.osmpbf.highways_only {
            opts = opts.with_highways_only(v);
        }
        if let Some(v) = controls.osmpbf.named_ways_only {
            opts = opts.with_named_ways_only(v);
        }
        if let Some(v) = controls.osmpbf.polygons_only {
            opts = opts.with_polygons_only(v);
        }
        if let Some(ref keys) = controls.osmpbf.include_tag_keys {
            opts = opts.with_include_tag_keys(keys.clone());
        }
        return wbvector::osmpbf::read_with_options(src_path, &opts).map_err(to_invalid_request);
    }

    wbvector::read(src_path).map_err(to_invalid_request)
}

fn write_vector_with_controls(
    layer: &wbvector::Layer,
    dst_path: &Path,
    dst_format: VectorFormat,
    controls: &VectorWriteControls,
) -> Result<(), ToolError> {
    if controls.has_geoparquet_controls() && dst_format != VectorFormat::GeoParquet {
        if controls.strict_format_options {
            return Err(ToolError::InvalidRequest(
                "GeoParquet-specific write options were provided for a non-Parquet output path"
                    .to_string(),
            ));
        }
        return wbvector::write(layer, dst_path, dst_format).map_err(to_invalid_request);
    }

    if dst_format == VectorFormat::GeoParquet {
        let mut opts = wbvector::geoparquet::GeoParquetWriteOptions::new();
        if let Some(v) = controls.geoparquet.max_rows_per_group {
            opts = opts.with_max_rows_per_group(v);
        }
        if let Some(v) = controls.geoparquet.data_page_size_limit {
            opts = opts.with_data_page_size_limit(v);
        }
        if let Some(v) = controls.geoparquet.write_batch_size {
            opts = opts.with_write_batch_size(v);
        }
        if let Some(v) = controls.geoparquet.data_page_row_count_limit {
            opts = opts.with_data_page_row_count_limit(v);
        }
        if let Some(v) = controls.geoparquet.compression {
            opts = opts.with_compression(v);
        }
        return wbvector::geoparquet::write_with_options(layer, dst_path, &opts)
            .map_err(to_invalid_request);
    }

    wbvector::write(layer, dst_path, dst_format).map_err(to_invalid_request)
}

#[derive(Debug, Clone, Default)]
struct GeoTiffWriteControls {
    compression: Option<GeoTiffCompression>,
    bigtiff: Option<bool>,
    layout: Option<GeoTiffLayout>,
    has_fields: bool,
}

#[derive(Debug, Clone, Default)]
struct RasterWriteControls {
    compress: Option<bool>,
    strict_format_options: bool,
    geotiff: GeoTiffWriteControls,
}

impl RasterWriteControls {
    fn has_geotiff_controls(&self) -> bool {
        self.compress.is_some() || self.geotiff.has_fields
    }

    fn geotiff_options(&self) -> Option<GeoTiffWriteOptions> {
        let compression = self
            .geotiff
            .compression
            .or_else(|| match self.compress {
                Some(true) => Some(GeoTiffCompression::Deflate),
                Some(false) => Some(GeoTiffCompression::None),
                None => None,
            });
        let bigtiff = self.geotiff.bigtiff;
        let layout = self.geotiff.layout;

        if compression.is_none() && bigtiff.is_none() && layout.is_none() {
            None
        } else {
            Some(GeoTiffWriteOptions {
                compression,
                bigtiff,
                layout,
            })
        }
    }
}

fn parse_geotiff_compression(name: &str) -> Option<GeoTiffCompression> {
    match name.trim().to_ascii_lowercase().as_str() {
        "none" | "off" | "uncompressed" => Some(GeoTiffCompression::None),
        "deflate" | "zip" => Some(GeoTiffCompression::Deflate),
        "lzw" => Some(GeoTiffCompression::Lzw),
        "packbits" | "pack_bits" => Some(GeoTiffCompression::PackBits),
        "jpeg" => Some(GeoTiffCompression::Jpeg),
        "webp" | "web_p" => Some(GeoTiffCompression::WebP),
        "jpegxl" | "jpeg_xl" | "jxl" => Some(GeoTiffCompression::JpegXl),
        _ => None,
    }
}

fn parse_geotiff_layout(layout_name: &str, geotiff_obj: &serde_json::Map<String, Value>) -> Result<GeoTiffLayout, ToolError> {
    let get_u32 = |keys: &[&str]| -> Option<u32> {
        for key in keys {
            if let Some(v) = geotiff_obj.get(*key).and_then(Value::as_u64) {
                if let Ok(parsed) = u32::try_from(v) {
                    return Some(parsed);
                }
            }
        }
        None
    };

    match layout_name.trim().to_ascii_lowercase().as_str() {
        "standard" => Ok(GeoTiffLayout::Standard),
        "stripped" | "striped" => {
            let rows_per_strip = get_u32(&["rows_per_strip"]).unwrap_or(1);
            Ok(GeoTiffLayout::Stripped { rows_per_strip })
        }
        "tiled" => {
            let tile_width = get_u32(&["tile_width", "tile_size"]).unwrap_or(512);
            let tile_height = get_u32(&["tile_height", "tile_size"]).unwrap_or(tile_width);
            Ok(GeoTiffLayout::Tiled { tile_width, tile_height })
        }
        "cog" => {
            let tile_size = get_u32(&["tile_size", "cog_tile_size"]).unwrap_or(512);
            Ok(GeoTiffLayout::Cog { tile_size })
        }
        other => Err(ToolError::InvalidRequest(format!(
            "unsupported geotiff.layout '{other}'. Expected one of: standard, stripped, tiled, cog"
        ))),
    }
}

fn parse_raster_write_controls(options: &Value) -> Result<RasterWriteControls, ToolError> {
    if options.is_null() {
        return Ok(RasterWriteControls::default());
    }

    let obj = options.as_object().ok_or_else(|| {
        ToolError::InvalidRequest("write options must be a JSON object".to_string())
    })?;

    let compress = match obj.get("compress") {
        Some(Value::Bool(v)) => Some(*v),
        Some(Value::Null) | None => None,
        Some(other) => {
            return Err(ToolError::InvalidRequest(format!(
                "options.compress must be a boolean when provided, got: {other}"
            )))
        }
    };

    let strict_format_options = match obj.get("strict_format_options") {
        Some(Value::Bool(v)) => *v,
        Some(Value::Null) | None => false,
        Some(other) => {
            return Err(ToolError::InvalidRequest(format!(
                "options.strict_format_options must be a boolean when provided, got: {other}"
            )))
        }
    };

    let mut geotiff = GeoTiffWriteControls::default();
    if let Some(gt_val) = obj.get("geotiff") {
        let gt_obj = gt_val.as_object().ok_or_else(|| {
            ToolError::InvalidRequest("options.geotiff must be a JSON object".to_string())
        })?;

        geotiff.has_fields = !gt_obj.is_empty();

        if let Some(v) = gt_obj.get("compression") {
            let name = v.as_str().ok_or_else(|| {
                ToolError::InvalidRequest("options.geotiff.compression must be a string".to_string())
            })?;
            geotiff.compression = Some(parse_geotiff_compression(name).ok_or_else(|| {
                ToolError::InvalidRequest(format!(
                    "unsupported geotiff.compression '{name}'. Expected one of: none, deflate, lzw, packbits, jpeg, webp, jpegxl"
                ))
            })?);
        }

        if let Some(v) = gt_obj.get("bigtiff") {
            geotiff.bigtiff = Some(v.as_bool().ok_or_else(|| {
                ToolError::InvalidRequest("options.geotiff.bigtiff must be a boolean".to_string())
            })?);
        }

        if let Some(v) = gt_obj.get("layout") {
            let layout_name = v.as_str().ok_or_else(|| {
                ToolError::InvalidRequest("options.geotiff.layout must be a string".to_string())
            })?;
            geotiff.layout = Some(parse_geotiff_layout(layout_name, gt_obj)?);
        }
    }

    Ok(RasterWriteControls {
        compress,
        strict_format_options,
        geotiff,
    })
}

fn write_raster_with_controls(raster: &Raster, dst: &Path, output_format: RasterFormat, controls: &RasterWriteControls) -> Result<(), ToolError> {
    if output_format != RasterFormat::GeoTiff && controls.has_geotiff_controls() {
        if controls.strict_format_options {
            return Err(ToolError::InvalidRequest(
                "GeoTIFF-specific write options were provided for a non-GeoTIFF output path".to_string(),
            ));
        }
        return raster.write(dst, output_format).map_err(to_invalid_request);
    }

    if output_format == RasterFormat::GeoTiff {
        if let Some(opts) = controls.geotiff_options() {
            return raster
                .write_geotiff_with_options(dst, &opts)
                .map_err(to_invalid_request);
        }
    }

    raster.write(dst, output_format).map_err(to_invalid_request)
}

#[derive(Debug, Clone, Default)]
struct LazWriteControls {
    chunk_size: Option<u32>,
    compression_level: Option<u32>,
}

#[derive(Debug, Clone, Default)]
struct CopcWriteControls {
    max_points_per_node: Option<usize>,
    max_depth: Option<u32>,
    node_point_ordering: Option<wblidar::copc::CopcNodePointOrdering>,
}

#[derive(Debug, Clone, Default)]
struct LidarWriteControls {
    laz: LazWriteControls,
    copc: CopcWriteControls,
}

impl LidarWriteControls {
    fn to_wblidar_options(&self) -> LidarWriteOptions {
        LidarWriteOptions {
            laz: LazWriteOptions {
                chunk_size: self.laz.chunk_size,
                compression_level: self.laz.compression_level,
            },
            copc: CopcWriteOptions {
                max_points_per_node: self.copc.max_points_per_node,
                max_depth: self.copc.max_depth,
                node_point_ordering: self.copc.node_point_ordering,
            },
        }
    }
}

fn parse_node_point_ordering(name: &str) -> Result<wblidar::copc::CopcNodePointOrdering, ToolError> {
    use wblidar::copc::CopcNodePointOrdering;
    match name.trim().to_ascii_lowercase().as_str() {
        "auto" => Ok(CopcNodePointOrdering::Auto),
        "morton" => Ok(CopcNodePointOrdering::Morton),
        "hilbert" => Ok(CopcNodePointOrdering::Hilbert),
        other => Err(ToolError::InvalidRequest(format!(
            "unsupported node_point_ordering '{}'. Expected one of: auto, morton, hilbert",
            other
        ))),
    }
}

fn parse_lidar_write_controls(options: &Value) -> Result<LidarWriteControls, ToolError> {
    if options.is_null() {
        return Ok(LidarWriteControls::default());
    }

    let obj = options.as_object().ok_or_else(|| {
        ToolError::InvalidRequest("write options must be a JSON object".to_string())
    })?;

    let mut laz = LazWriteControls::default();
    if let Some(laz_val) = obj.get("laz") {
        let laz_obj = laz_val.as_object().ok_or_else(|| {
            ToolError::InvalidRequest("options.laz must be a JSON object".to_string())
        })?;

        if let Some(v) = laz_obj.get("chunk_size") {
            let chunk_size = v.as_u64().ok_or_else(|| {
                ToolError::InvalidRequest("options.laz.chunk_size must be a positive integer".to_string())
            })?;
            if chunk_size == 0 {
                return Err(ToolError::InvalidRequest(
                    "options.laz.chunk_size must be greater than 0".to_string(),
                ));
            }
            laz.chunk_size = Some(chunk_size as u32);
        }

        if let Some(v) = laz_obj.get("compression_level") {
            let compression_level = v.as_u64().ok_or_else(|| {
                ToolError::InvalidRequest("options.laz.compression_level must be a positive integer".to_string())
            })?;
            if compression_level > 9 {
                return Err(ToolError::InvalidRequest(
                    "options.laz.compression_level must be in range 0-9".to_string(),
                ));
            }
            laz.compression_level = Some(compression_level as u32);
        }
    }

    let mut copc = CopcWriteControls::default();
    if let Some(copc_val) = obj.get("copc") {
        let copc_obj = copc_val.as_object().ok_or_else(|| {
            ToolError::InvalidRequest("options.copc must be a JSON object".to_string())
        })?;

        if let Some(v) = copc_obj.get("max_points_per_node") {
            let max_points_per_node = v.as_u64().ok_or_else(|| {
                ToolError::InvalidRequest("options.copc.max_points_per_node must be a positive integer".to_string())
            })?;
            if max_points_per_node == 0 {
                return Err(ToolError::InvalidRequest(
                    "options.copc.max_points_per_node must be greater than 0".to_string(),
                ));
            }
            copc.max_points_per_node = Some(max_points_per_node as usize);
        }

        if let Some(v) = copc_obj.get("max_depth") {
            let max_depth = v.as_u64().ok_or_else(|| {
                ToolError::InvalidRequest("options.copc.max_depth must be a positive integer".to_string())
            })?;
            if max_depth == 0 {
                return Err(ToolError::InvalidRequest(
                    "options.copc.max_depth must be greater than 0".to_string(),
                ));
            }
            copc.max_depth = Some(max_depth as u32);
        }

        if let Some(v) = copc_obj.get("node_point_ordering") {
            let ordering_name = v.as_str().ok_or_else(|| {
                ToolError::InvalidRequest("options.copc.node_point_ordering must be a string".to_string())
            })?;
            copc.node_point_ordering = Some(parse_node_point_ordering(ordering_name)?);
        }
    }

    Ok(LidarWriteControls { laz, copc })
}

fn lidar_format_name(format: LidarFormat) -> &'static str {
    match format {
        LidarFormat::Las => "las",
        LidarFormat::Laz => "laz",
        LidarFormat::Copc => "copc",
        LidarFormat::Ply => "ply",
        LidarFormat::E57 => "e57",
    }
}

fn sensor_bundle_family_name(bundle: &SensorBundle) -> &'static str {
    match bundle {
        SensorBundle::Safe(SafeBundle::Sentinel1(_)) => "sentinel1_safe",
        SensorBundle::Safe(SafeBundle::Sentinel2(_)) => "sentinel2_safe",
        SensorBundle::Landsat(_) => "landsat",
        SensorBundle::Iceye(_) => "iceye",
        SensorBundle::PlanetScope(_) => "planetscope",
        SensorBundle::Dimap(_) => "dimap",
        SensorBundle::MaxarWorldView(_) => "maxar_worldview",
        SensorBundle::Radarsat2(_) => "radarsat2",
        SensorBundle::Rcm(_) => "rcm",
    }
}

fn sensor_bundle_root_path(bundle: &SensorBundle) -> PathBuf {
    match bundle {
        SensorBundle::Safe(SafeBundle::Sentinel1(pkg)) => pkg.safe_root.clone(),
        SensorBundle::Safe(SafeBundle::Sentinel2(pkg)) => pkg.safe_root.clone(),
        SensorBundle::Landsat(pkg) => pkg.bundle_root.clone(),
        SensorBundle::Iceye(pkg) => pkg.bundle_root.clone(),
        SensorBundle::PlanetScope(pkg) => pkg.bundle_root.clone(),
        SensorBundle::Dimap(pkg) => pkg.bundle_root.clone(),
        SensorBundle::MaxarWorldView(pkg) => pkg.bundle_root.clone(),
        SensorBundle::Radarsat2(pkg) => pkg.bundle_root.clone(),
        SensorBundle::Rcm(pkg) => pkg.bundle_root.clone(),
    }
}

fn sensor_bundle_metadata_json_value(opened: &OpenedSensorBundle, input_path: &Path) -> Value {
    let bundle_root = sensor_bundle_root_path(&opened.bundle);

    let mut base = json!({
        "input_path": input_path.display().to_string(),
        "bundle_root": bundle_root.display().to_string(),
        "opened_from_archive": opened.extracted_root.is_some(),
        "family": sensor_bundle_family_name(&opened.bundle),
        "band_keys": Vec::<String>::new(),
        "measurement_keys": Vec::<String>::new(),
        "qa_keys": Vec::<String>::new(),
        "aux_keys": Vec::<String>::new(),
        "asset_keys": Vec::<String>::new(),
    });

    let extras = match &opened.bundle {
        SensorBundle::Safe(SafeBundle::Sentinel2(pkg)) => json!({
            "product_level": format!("{:?}", pkg.product_level),
            "acquisition_datetime_utc": pkg.acquisition_datetime_utc,
            "tile_id": pkg.tile_id,
            "cloud_cover_percent": pkg.cloud_coverage_assessment,
            "processing_baseline": pkg.processing_baseline,
            "band_keys": pkg.list_band_keys(),
            "qa_keys": pkg.list_qa_keys(),
            "aux_keys": pkg.list_aux_keys(),
        }),
        SensorBundle::Safe(SafeBundle::Sentinel1(pkg)) => json!({
            "product_type": pkg.product_type,
            "acquisition_datetime_utc": pkg.acquisition_datetime_utc,
            "acquisition_mode": pkg.acquisition_mode,
            "polarization": pkg.polarization,
            "polarizations": pkg.list_polarizations(),
            "spatial_bounds": pkg.spatial_bounds,
            "measurement_keys": pkg.list_measurement_keys(),
        }),
        SensorBundle::Landsat(pkg) => json!({
            "mission": format!("{:?}", pkg.mission),
            "processing_level": format!("{:?}", pkg.processing_level),
            "product_id": pkg.product_id,
            "collection_number": pkg.collection_number,
            "acquisition_datetime_utc": match (&pkg.acquisition_date_utc, &pkg.scene_center_time_utc) {
                (Some(d), Some(t)) => Some(format!("{d}T{t}")),
                (Some(d), None) => Some(d.clone()),
                _ => None,
            },
            "cloud_cover_percent": pkg.cloud_cover_percent,
            "path_row": pkg.path_row,
            "band_keys": pkg.list_band_keys(),
            "qa_keys": pkg.list_qa_keys(),
            "aux_keys": pkg.list_aux_keys(),
        }),
        SensorBundle::Iceye(pkg) => json!({
            "product_type": pkg.product_type,
            "acquisition_datetime_utc": pkg.acquisition_datetime_utc,
            "acquisition_mode": pkg.acquisition_mode,
            "polarization": pkg.polarization,
            "polarizations": pkg.list_polarizations(),
            "asset_keys": pkg.list_asset_keys(),
        }),
        SensorBundle::PlanetScope(pkg) => json!({
            "scene_id": pkg.scene_id,
            "acquisition_datetime_utc": pkg.acquisition_datetime_utc,
            "product_type": pkg.product_type,
            "cloud_cover_percent": pkg.cloud_cover_percent,
            "band_keys": pkg.list_band_keys(),
            "qa_keys": pkg.list_qa_keys(),
        }),
        SensorBundle::Dimap(pkg) => json!({
            "mission": pkg.mission,
            "scene_id": pkg.scene_id,
            "acquisition_datetime_utc": pkg.acquisition_datetime_utc,
            "processing_level": pkg.processing_level,
            "cloud_cover_percent": pkg.cloud_cover_percent,
            "band_keys": pkg.list_band_keys(),
        }),
        SensorBundle::MaxarWorldView(pkg) => json!({
            "mission": pkg.satellite,
            "scene_id": pkg.scene_id,
            "acquisition_datetime_utc": pkg.acquisition_datetime_utc,
            "cloud_cover_percent": pkg.cloud_cover_percent,
            "band_keys": pkg.list_band_keys(),
        }),
        SensorBundle::Radarsat2(pkg) => json!({
            "product_type": pkg.product_type,
            "acquisition_datetime_utc": pkg.acquisition_datetime_utc,
            "acquisition_mode": pkg.acquisition_mode,
            "polarizations": pkg.polarizations,
            "measurement_keys": pkg.list_measurement_keys(),
        }),
        SensorBundle::Rcm(pkg) => json!({
            "product_type": pkg.product_type,
            "acquisition_datetime_utc": pkg.acquisition_datetime_utc,
            "acquisition_mode": pkg.acquisition_mode,
            "polarizations": pkg.polarizations,
            "measurement_keys": pkg.list_measurement_keys(),
        }),
    };

    if let (Some(base_obj), Some(extra_obj)) = (base.as_object_mut(), extras.as_object()) {
        for (key, value) in extra_obj {
            base_obj.insert(key.clone(), value.clone());
        }
    }

    base
}

pub fn sensor_bundle_resolve_raster_path(
    bundle_root: &str,
    key: &str,
    key_type: &str,
) -> Result<String, ToolError> {
    let bundle = open_sensor_bundle(bundle_root).map_err(to_invalid_request)?;
    let path = match (&bundle, key_type) {
        (SensorBundle::Safe(SafeBundle::Sentinel2(pkg)), "band") => pkg.band_path(key),
        (SensorBundle::Landsat(pkg), "band") => pkg.band_path(key),
        (SensorBundle::PlanetScope(pkg), "band") => pkg.band_path(key),
        (SensorBundle::Dimap(pkg), "band") => pkg.band_path(key),
        (SensorBundle::MaxarWorldView(pkg), "band") => pkg.band_path(key),

        (SensorBundle::Safe(SafeBundle::Sentinel2(pkg)), "qa") => pkg.qa_path(key),
        (SensorBundle::Landsat(pkg), "qa") => pkg.qa_path(key),
        (SensorBundle::PlanetScope(pkg), "qa") => pkg.qa_path(key),

        (SensorBundle::Safe(SafeBundle::Sentinel2(pkg)), "aux") => pkg.aux_path(key),
        (SensorBundle::Landsat(pkg), "aux") => pkg.aux_path(key),

        (SensorBundle::Safe(SafeBundle::Sentinel1(pkg)), "measurement") => pkg.measurement_path(key),
        (SensorBundle::Radarsat2(pkg), "measurement") => pkg.measurement_path(key),
        (SensorBundle::Rcm(pkg), "measurement") => pkg.measurement_path(key),

        (SensorBundle::Iceye(pkg), "asset") => pkg.asset_path(key),

        _ => {
            return Err(ToolError::InvalidRequest(format!(
                "read_{} is not supported for bundle family '{}'",
                key_type,
                sensor_bundle_family_name(&bundle)
            )))
        }
    };

    let p = path.ok_or_else(|| {
        ToolError::InvalidRequest(format!(
            "{} key '{}' not found in bundle '{}'",
            key_type,
            key,
            bundle_root
        ))
    })?;

    Ok(p.display().to_string())
}

/// Return raster metadata as a JSON string (header-only, no pixel data loaded).
/// Fields: path, cols, rows, bands, x_min, y_min, x_max, y_max,
///         cell_size_x, cell_size_y, nodata, data_type, crs_wkt, crs_epsg.
pub fn raster_metadata_json(path: &str) -> Result<String, ToolError> {
    let raster = Raster::read(path).map_err(to_invalid_request)?;
    let meta = json!({
        "path": path,
        "cols": raster.cols,
        "rows": raster.rows,
        "bands": raster.bands,
        "x_min": raster.x_min,
        "y_min": raster.y_min,
        "x_max": raster.x_min + raster.cell_size_x * raster.cols as f64,
        "y_max": raster.y_min + raster.cell_size_y * raster.rows as f64,
        "cell_size_x": raster.cell_size_x,
        "cell_size_y": raster.cell_size_y,
        "nodata": raster.nodata,
        "data_type": raster.data_type.as_str(),
        "crs_wkt": raster.crs.wkt,
        "crs_epsg": raster.crs.epsg,
    });
    serde_json::to_string(&meta).map_err(|e| ToolError::Execution(e.to_string()))
}

/// Return vector layer metadata as a JSON string.
/// Fields: path, geometry_type, feature_count, crs_wkt, crs_epsg,
///         fields (array of {name, field_type}).
pub fn vector_metadata_json(path: &str) -> Result<String, ToolError> {
    let layer = wbvector::read(path).map_err(to_invalid_request)?;
    let fields: Vec<Value> = layer.schema.fields().iter().map(|f| {
        json!({
            "name": f.name,
            "field_type": match f.field_type {
                FieldType::Integer  => "integer",
                FieldType::Float    => "float",
                FieldType::Text     => "text",
                FieldType::Date     => "date",
                FieldType::DateTime => "datetime",
                FieldType::Boolean  => "boolean",
                FieldType::Blob     => "blob",
                FieldType::Json     => "json",
            }
        })
    }).collect();
    let meta = json!({
        "path": path,
        "geometry_type": layer.geom_type.map(|g| g.as_str()),
        "feature_count": layer.features.len(),
        "crs_wkt": layer.crs_wkt(),
        "crs_epsg": layer.crs_epsg(),
        "fields": fields,
    });
    serde_json::to_string(&meta).map_err(|e| ToolError::Execution(e.to_string()))
}

/// Convert an EPSG code to OGC WKT.
pub fn projection_to_ogc_wkt(epsg: u32) -> Result<String, ToolError> {
    to_ogc_wkt(epsg).map_err(to_invalid_request)
}

/// Identify an EPSG code from WKT or CRS text, when possible.
pub fn projection_identify_epsg(crs_text: &str) -> Result<Option<u32>, ToolError> {
    let trimmed = crs_text.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    if let Some(code) = identify_epsg_from_wkt(trimmed) {
        return Ok(Some(code));
    }

    if let Some(code) = epsg_from_srs_reference(trimmed) {
        return Ok(Some(code));
    }

    Ok(None)
}

/// Reproject a list of XY points across EPSG codes.
///
/// Input JSON format: `[{"x": <number>, "y": <number>}, ...]`
/// Output JSON format: same as input with transformed coordinates.
pub fn projection_reproject_points_json(
    points_json: &str,
    src_epsg: u32,
    dst_epsg: u32,
) -> Result<String, ToolError> {
    let points_value: Value = serde_json::from_str(points_json)
        .map_err(|e| ToolError::InvalidRequest(format!("invalid points JSON: {e}")))?;
    let points = points_value.as_array().ok_or_else(|| {
        ToolError::InvalidRequest("points_json must be an array of objects with x and y".to_string())
    })?;

    let src = Crs::from_epsg(src_epsg).map_err(to_invalid_request)?;
    let dst = Crs::from_epsg(dst_epsg).map_err(to_invalid_request)?;

    let mut out = Vec::with_capacity(points.len());
    for (idx, point) in points.iter().enumerate() {
        let obj = point.as_object().ok_or_else(|| {
            ToolError::InvalidRequest(format!(
                "points_json[{idx}] must be an object with numeric x and y"
            ))
        })?;
        let x = obj
            .get("x")
            .and_then(Value::as_f64)
            .ok_or_else(|| ToolError::InvalidRequest(format!("points_json[{idx}].x must be a number")))?;
        let y = obj
            .get("y")
            .and_then(Value::as_f64)
            .ok_or_else(|| ToolError::InvalidRequest(format!("points_json[{idx}].y must be a number")))?;
        let (tx, ty) = src.transform_to(x, y, &dst).map_err(to_invalid_request)?;
        out.push(json!({"x": tx, "y": ty}));
    }

    serde_json::to_string(&out).map_err(|e| ToolError::Execution(e.to_string()))
}

/// Reproject a single XY point across EPSG codes.
///
/// Output JSON format: `{"x": <number>, "y": <number>}`
pub fn projection_reproject_point_json(
    x: f64,
    y: f64,
    src_epsg: u32,
    dst_epsg: u32,
) -> Result<String, ToolError> {
    let src = Crs::from_epsg(src_epsg).map_err(to_invalid_request)?;
    let dst = Crs::from_epsg(dst_epsg).map_err(to_invalid_request)?;
    let (tx, ty) = src.transform_to(x, y, &dst).map_err(to_invalid_request)?;
    serde_json::to_string(&json!({"x": tx, "y": ty}))
        .map_err(|e| ToolError::Execution(e.to_string()))
}

fn topology_parse_wkt_pair(a_wkt: &str, b_wkt: &str) -> Result<(Geometry, Geometry), ToolError> {
    let a = from_wkt(a_wkt).map_err(to_invalid_request)?;
    let b = from_wkt(b_wkt).map_err(to_invalid_request)?;
    Ok((a, b))
}

/// Return whether two WKT geometries intersect.
pub fn topology_intersects_wkt(a_wkt: &str, b_wkt: &str) -> Result<bool, ToolError> {
    let (a, b) = topology_parse_wkt_pair(a_wkt, b_wkt)?;
    Ok(intersects(&a, &b))
}

/// Return whether geometry A contains geometry B.
pub fn topology_contains_wkt(a_wkt: &str, b_wkt: &str) -> Result<bool, ToolError> {
    let (a, b) = topology_parse_wkt_pair(a_wkt, b_wkt)?;
    Ok(contains(&a, &b))
}

/// Return whether geometry A is within geometry B.
pub fn topology_within_wkt(a_wkt: &str, b_wkt: &str) -> Result<bool, ToolError> {
    let (a, b) = topology_parse_wkt_pair(a_wkt, b_wkt)?;
    Ok(within(&a, &b))
}

/// Return whether two WKT geometries touch at boundaries.
pub fn topology_touches_wkt(a_wkt: &str, b_wkt: &str) -> Result<bool, ToolError> {
    let (a, b) = topology_parse_wkt_pair(a_wkt, b_wkt)?;
    Ok(touches(&a, &b))
}

/// Return whether two WKT geometries are disjoint.
pub fn topology_disjoint_wkt(a_wkt: &str, b_wkt: &str) -> Result<bool, ToolError> {
    let (a, b) = topology_parse_wkt_pair(a_wkt, b_wkt)?;
    Ok(disjoint(&a, &b))
}

/// Return whether two WKT geometries cross.
pub fn topology_crosses_wkt(a_wkt: &str, b_wkt: &str) -> Result<bool, ToolError> {
    let (a, b) = topology_parse_wkt_pair(a_wkt, b_wkt)?;
    Ok(crosses(&a, &b))
}

/// Return whether two WKT geometries overlap.
pub fn topology_overlaps_wkt(a_wkt: &str, b_wkt: &str) -> Result<bool, ToolError> {
    let (a, b) = topology_parse_wkt_pair(a_wkt, b_wkt)?;
    Ok(overlaps(&a, &b))
}

/// Return whether geometry A covers geometry B.
pub fn topology_covers_wkt(a_wkt: &str, b_wkt: &str) -> Result<bool, ToolError> {
    let (a, b) = topology_parse_wkt_pair(a_wkt, b_wkt)?;
    Ok(covers(&a, &b))
}

/// Return whether geometry A is covered by geometry B.
pub fn topology_covered_by_wkt(a_wkt: &str, b_wkt: &str) -> Result<bool, ToolError> {
    let (a, b) = topology_parse_wkt_pair(a_wkt, b_wkt)?;
    Ok(covered_by(&a, &b))
}

/// Return the DE-9IM matrix string for two WKT geometries.
pub fn topology_relate_wkt(a_wkt: &str, b_wkt: &str) -> Result<String, ToolError> {
    let (a, b) = topology_parse_wkt_pair(a_wkt, b_wkt)?;
    Ok(relate(&a, &b).as_str9())
}

/// Return planar geometry distance for two WKT geometries.
pub fn topology_distance_wkt(a_wkt: &str, b_wkt: &str) -> Result<f64, ToolError> {
    let (a, b) = topology_parse_wkt_pair(a_wkt, b_wkt)?;
    Ok(geometry_distance(&a, &b))
}

fn topology_read_feature_geometry_as_wkt(path: &str, feature_index: usize) -> Result<String, ToolError> {
    let layer = wbvector::read(path).map_err(to_invalid_request)?;
    let feature = layer.features.get(feature_index).ok_or_else(|| {
        ToolError::InvalidRequest(format!(
            "feature_index {feature_index} out of range for '{}'(feature_count={})",
            path,
            layer.features.len()
        ))
    })?;
    let geom = feature.geometry.as_ref().ok_or_else(|| {
        ToolError::InvalidRequest(format!(
            "feature_index {feature_index} in '{}' has no geometry",
            path
        ))
    })?;
    Ok(geom.to_wkt())
}

/// Compute topology relation summary between two vector features.
///
/// Returns JSON containing DE-9IM matrix, planar distance, and common predicate booleans.
pub fn topology_vector_feature_relation_json(
    a_path: &str,
    a_feature_index: usize,
    b_path: &str,
    b_feature_index: usize,
) -> Result<String, ToolError> {
    let a_wkt = topology_read_feature_geometry_as_wkt(a_path, a_feature_index)?;
    let b_wkt = topology_read_feature_geometry_as_wkt(b_path, b_feature_index)?;
    let (a, b) = topology_parse_wkt_pair(&a_wkt, &b_wkt)?;

    let summary = json!({
        "a_path": a_path,
        "a_feature_index": a_feature_index,
        "b_path": b_path,
        "b_feature_index": b_feature_index,
        "relate": relate(&a, &b).as_str9(),
        "distance": geometry_distance(&a, &b),
        "intersects": intersects(&a, &b),
        "contains": contains(&a, &b),
        "within": within(&a, &b),
        "touches": touches(&a, &b),
        "disjoint": disjoint(&a, &b),
        "crosses": crosses(&a, &b),
        "overlaps": overlaps(&a, &b),
        "covers": covers(&a, &b),
        "covered_by": covered_by(&a, &b),
    });

    serde_json::to_string(&summary).map_err(|e| ToolError::Execution(e.to_string()))
}

/// Validate a polygon (or multipolygon) WKT.
pub fn topology_is_valid_polygon_wkt(wkt: &str) -> Result<bool, ToolError> {
    let g = from_wkt(wkt).map_err(to_invalid_request)?;
    match g {
        Geometry::Polygon(poly) => Ok(is_valid_polygon(&poly)),
        Geometry::MultiPolygon(polys) => Ok(polys.iter().all(is_valid_polygon)),
        _ => Err(ToolError::InvalidRequest(
            "topology_is_valid_polygon_wkt requires POLYGON or MULTIPOLYGON WKT".to_string(),
        )),
    }
}

/// Repair polygon WKT and return repaired geometry as WKT.
pub fn topology_make_valid_polygon_wkt(wkt: &str, epsilon: f64) -> Result<String, ToolError> {
    let g = from_wkt(wkt).map_err(to_invalid_request)?;
    match g {
        Geometry::Polygon(poly) => {
            let repaired = make_valid_polygon(&poly, epsilon);
            Ok(to_wkt(&Geometry::MultiPolygon(repaired)))
        }
        Geometry::MultiPolygon(polys) => {
            let mut repaired = Vec::new();
            for poly in polys {
                repaired.extend(make_valid_polygon(&poly, epsilon));
            }
            Ok(to_wkt(&Geometry::MultiPolygon(repaired)))
        }
        _ => Err(ToolError::InvalidRequest(
            "topology_make_valid_polygon_wkt requires POLYGON or MULTIPOLYGON WKT".to_string(),
        )),
    }
}

/// Buffer WKT geometry and return buffered polygon WKT.
pub fn topology_buffer_wkt(wkt: &str, distance: f64) -> Result<String, ToolError> {
    let g = from_wkt(wkt).map_err(to_invalid_request)?;
    let options = BufferOptions::default();
    let buffered = match g {
        Geometry::Point(pt) => buffer_point(Coord::xy(pt.x, pt.y), distance, options),
        Geometry::LineString(ls) => buffer_linestring(&ls, distance, options),
        Geometry::Polygon(poly) => buffer_polygon(&poly, distance, options),
        _ => {
            return Err(ToolError::InvalidRequest(
                "topology_buffer_wkt currently supports POINT, LINESTRING, and POLYGON WKT"
                    .to_string(),
            ))
        }
    };

    Ok(to_wkt(&Geometry::Polygon(buffered)))
}

/// Copy a vector file from `src` to `dst`, re-encoding in the format detected
/// from `dst`'s file extension.  This keeps the copy entirely inside wbvector
/// rather than round-tripping through a third-party library.
pub fn vector_copy_to_path(src: &str, dst: &str) -> Result<(), ToolError> {
    vector_copy_with_options_json(src, dst, "{}")?;
    Ok(())
}

/// Copy or transcode a vector file from `src` to `dst` using JSON read/write options.
///
/// Supported options:
/// - `strict_format_options`: bool
/// - `osmpbf` (read side):
///   - `highways_only`: bool
///   - `named_ways_only`: bool
///   - `polygons_only`: bool
///   - `include_tag_keys`: [string]
/// - `geoparquet` (write side):
///   - `max_rows_per_group`: integer
///   - `data_page_size_limit`: integer
///   - `write_batch_size`: integer
///   - `data_page_row_count_limit`: integer
///   - `compression`: none|snappy|gzip|lz4|zstd|brotli
pub fn vector_copy_with_options_json(src: &str, dst: &str, options_json: &str) -> Result<String, ToolError> {
    let options_value: Value = serde_json::from_str(options_json)
        .map_err(|e| ToolError::InvalidRequest(format!("invalid options JSON: {e}")))?;
    let read_controls = parse_vector_read_controls(&options_value)?;
    let write_controls = parse_vector_write_controls(&options_value)?;

    let src_path = Path::new(src);
    let mut dst_path = PathBuf::from(dst);
    let missing_ext = dst_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.trim().is_empty())
        .unwrap_or(true);
    if missing_ext {
        dst_path.set_extension("gpkg");
    }

    if let Some(parent) = dst_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent).map_err(to_invalid_request)?;
        }
    }

    let src_fmt = VectorFormat::detect(src_path).map_err(to_invalid_request)?;
    let dst_fmt = VectorFormat::detect(&dst_path).map_err(to_invalid_request)?;

    let layer = read_vector_with_controls(src_path, src_fmt, &read_controls)?;
    write_vector_with_controls(&layer, &dst_path, dst_fmt, &write_controls)?;

    Ok(dst_path.display().to_string())
}

/// Copy or transcode a lidar file from `src` to `dst`.
///
/// When `dst` has no extension, `.copc.laz` is appended and output is written
/// as COPC.
pub fn lidar_copy_to_path(src: &str, dst: &str) -> Result<String, ToolError> {
    let src_path = Path::new(src);
    let mut dst_path = PathBuf::from(dst);
    let missing_ext = dst_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.trim().is_empty())
        .unwrap_or(true);

    if missing_ext {
        dst_path = PathBuf::from(format!("{}.copc.laz", dst_path.to_string_lossy()));
    }

    if let Some(parent) = dst_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent).map_err(to_invalid_request)?;
        }
    }

    if src_path == dst_path {
        return Ok(dst_path.display().to_string());
    }

    if missing_ext {
        let cloud = PointCloud::read(src_path).map_err(to_invalid_request)?;
        cloud
            .write_as(&dst_path, LidarFormat::Copc)
            .map_err(to_invalid_request)?;
    } else {
        std::fs::copy(src_path, &dst_path).map_err(to_invalid_request)?;
    }

    Ok(dst_path.display().to_string())
}

/// Write a lidar point cloud from `src` to `dst` using JSON write options.
///
/// The `options_json` object supports:
/// - `laz`: {`chunk_size`: positive integer, `compression_level`: 0-9}
/// - `copc`: {`max_points_per_node`: positive integer, `max_depth`: positive integer, `node_point_ordering`: auto|morton|hilbert}
///
pub fn lidar_write_with_options_json(src: &str, dst: &str, options_json: &str) -> Result<String, ToolError> {
    let options_value: Value = serde_json::from_str(options_json)
        .map_err(|e| ToolError::InvalidRequest(format!("invalid options JSON: {e}")))?;
    let controls = parse_lidar_write_controls(&options_value)?;

    let src_path = Path::new(src);
    let mut dst_path = PathBuf::from(dst);
    let missing_ext = dst_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.trim().is_empty())
        .unwrap_or(true);

    if missing_ext {
        dst_path = PathBuf::from(format!("{}.copc.laz", dst_path.to_string_lossy()));
    }

    if let Some(parent) = dst_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent).map_err(to_invalid_request)?;
        }
    }

    let cloud = PointCloud::read(src_path).map_err(to_invalid_request)?;
    let output_format = LidarFormat::detect(&dst_path).map_err(to_invalid_request)?;
    let write_options = controls.to_wblidar_options();
    wblidar::write_with_options(&cloud, &dst_path, output_format, &write_options)
        .map_err(to_invalid_request)?;

    Ok(dst_path.display().to_string())
}

/// Write a raster from `src` to `dst` using JSON write options.
///
/// The `options_json` object supports:
/// - `compress`: bool
/// - `strict_format_options`: bool
/// - `geotiff`: {
///     `compression`: none|deflate|lzw|packbits|jpeg|webp|jpegxl,
///     `bigtiff`: bool,
///     `layout`: standard|stripped|tiled|cog,
///     `rows_per_strip`, `tile_width`, `tile_height`, `tile_size`
///   }
pub fn raster_write_with_options_json(src: &str, dst: &str, options_json: &str) -> Result<(), ToolError> {
    let options_value: Value = serde_json::from_str(options_json)
        .map_err(|e| ToolError::InvalidRequest(format!("invalid options JSON: {e}")))?;
    let controls = parse_raster_write_controls(&options_value)?;

    let src_path = Path::new(src);
    let dst_path = Path::new(dst);
    if let Some(parent) = dst_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent).map_err(to_invalid_request)?;
        }
    }

    let output_format = RasterFormat::for_output_path(dst)
        .map_err(to_invalid_request)?;

    let raster = Raster::read(src_path).map_err(to_invalid_request)?;
    write_raster_with_controls(&raster, dst_path, output_format, &controls)
}

pub fn sensor_bundle_metadata_json(path: &str) -> Result<String, ToolError> {
    let bundle_path = Path::new(path);
    let opened = open_sensor_bundle_path(bundle_path).map_err(to_invalid_request)?;
    let meta = sensor_bundle_metadata_json_value(&opened, bundle_path);
    serde_json::to_string(&meta).map_err(|err| ToolError::Execution(err.to_string()))
}

pub fn lidar_metadata_json(path: &str) -> Result<String, ToolError> {
    let lidar_path = Path::new(path);
    let file_size_bytes = std::fs::metadata(lidar_path)
        .map_err(to_invalid_request)?
        .len();
    let format = LidarFormat::detect(lidar_path).map_err(to_invalid_request)?;

    let meta = match format {
        LidarFormat::Las | LidarFormat::Laz | LidarFormat::Copc => {
            let file = File::open(lidar_path).map_err(to_invalid_request)?;
            let reader = LasReader::new(BufReader::new(file)).map_err(to_invalid_request)?;
            let header = reader.header();
            let crs = reader.crs();

            json!({
                "path": lidar_path,
                "format": lidar_format_name(format),
                "file_size_bytes": file_size_bytes,
                "point_count": header.point_count(),
                "version_major": header.version_major,
                "version_minor": header.version_minor,
                "point_data_format_id": header.point_data_format as u8,
                "point_data_record_length": header.point_data_record_length,
                "system_identifier": header.system_identifier,
                "generating_software": header.generating_software,
                "crs_epsg": crs.and_then(|c| c.epsg),
                "crs_wkt": crs.and_then(|c| c.wkt.clone()),
                "bounds": {
                    "min_x": header.min_x,
                    "max_x": header.max_x,
                    "min_y": header.min_y,
                    "max_y": header.max_y,
                    "min_z": header.min_z,
                    "max_z": header.max_z
                }
            })
        }
        LidarFormat::Ply => {
            let file = File::open(lidar_path).map_err(to_invalid_request)?;
            let reader = PlyReader::new(BufReader::new(file)).map_err(to_invalid_request)?;
            json!({
                "path": lidar_path,
                "format": lidar_format_name(format),
                "file_size_bytes": file_size_bytes,
                "point_count": reader.point_count(),
                "crs_epsg": Value::Null,
                "crs_wkt": Value::Null,
                "bounds": Value::Null
            })
        }
        LidarFormat::E57 => {
            let file = File::open(lidar_path).map_err(to_invalid_request)?;
            let reader = E57Reader::new(BufReader::new(file)).map_err(to_invalid_request)?;
            let meta = reader.meta();
            let crs_text = meta.coordinate_metadata.clone();
            let crs_epsg = crs_text.as_ref().and_then(|text| {
                wblidar::crs::epsg_from_srs_reference(text)
                    .or_else(|| wblidar::crs::epsg_from_wkt(text))
            });
            let field_names: Vec<String> = meta.fields.iter().map(|field| field.name.clone()).collect();

            json!({
                "path": lidar_path,
                "format": lidar_format_name(format),
                "file_size_bytes": file_size_bytes,
                "point_count": meta.record_count,
                "name": meta.name,
                "field_names": field_names,
                "crs_epsg": crs_epsg,
                "crs_wkt": crs_text,
                "bounds": Value::Null
            })
        }
    };

    serde_json::to_string(&meta).map_err(|err| ToolError::Execution(err.to_string()))
}

struct CompositeRegistry {
    oss: OssRegistry,
    #[cfg(feature = "pro")]
    pro: Option<ProRegistry>,
}

impl ToolRuntimeRegistry for CompositeRegistry {
    fn list_tools(&self) -> Vec<wbcore::ToolMetadata> {
        #[cfg(feature = "pro")]
        let mut out = self.oss.list();
        #[cfg(not(feature = "pro"))]
        let out = self.oss.list();
        #[cfg(feature = "pro")]
        if let Some(pro) = &self.pro {
            out.extend(pro.list());
        }
        out
    }

    fn list_manifests(&self) -> Vec<ToolManifest> {
        #[cfg(feature = "pro")]
        let mut out = self.oss.manifests();
        #[cfg(not(feature = "pro"))]
        let out = self.oss.manifests();
        #[cfg(feature = "pro")]
        if let Some(pro) = &self.pro {
            out.extend(pro.manifests());
        }
        out
    }

    fn run_tool(&self, id: &str, args: &ToolArgs, ctx: &wbcore::ToolContext) -> Result<wbcore::ToolRunResult, ToolError> {
        match self.oss.run(id, args, ctx) {
            Ok(v) => Ok(v),
            Err(ToolError::NotFound(_)) => {
                #[cfg(feature = "pro")]
                if let Some(pro) = &self.pro {
                    return pro.run(id, args, ctx);
                }
                Err(ToolError::NotFound(id.to_string()))
            }
            Err(e) => Err(e),
        }
    }
}

fn validate_include_pro(include_pro: bool) -> Result<(), ToolError> {
    #[cfg(feature = "pro")]
    let _ = include_pro;

    #[cfg(not(feature = "pro"))]
    if include_pro {
        return Err(ToolError::InvalidRequest(
            "include_pro=true requested but this build does not include Pro support; rebuild with feature 'pro'".to_string(),
        ));
    }
    Ok(())
}

pub struct RToolRuntime {
    runtime: RuntimeMode,
}

enum RuntimeMode {
    Tier(OwnedToolRuntime<CompositeRegistry>),
    Entitled(OwnedToolRuntimeWithCapabilities<CompositeRegistry, EntitlementCapabilities>),
}

impl Default for RToolRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl RToolRuntime {
    pub fn new() -> Self {
        Self::new_with_options(false, LicenseTier::Open)
            .expect("default runtime construction should not fail")
    }

    #[cfg(feature = "pro")]
    pub fn new_with_options(include_pro: bool, max_tier: LicenseTier) -> Result<Self, ToolError> {
        validate_include_pro(include_pro)?;
        let mut oss = OssRegistry::new();
        register_default_oss_tools(&mut oss);

        let pro = if include_pro {
            let mut pro = ProRegistry::new();
            register_default_pro_tools(&mut pro);
            Some(pro)
        } else {
            None
        };

        Ok(Self {
            runtime: RuntimeMode::Tier(
                ToolRuntimeBuilder::new(CompositeRegistry { oss, pro })
                    .max_tier(max_tier)
                    .build(),
            ),
        })
    }

    #[cfg(feature = "pro")]
    pub fn new_with_floating_license_id(
        include_pro: bool,
        fallback_tier: LicenseTier,
        floating_license_id: &str,
        provider_url: Option<&str>,
        machine_id: Option<&str>,
        customer_id: Option<&str>,
    ) -> Result<Self, ToolError> {
        validate_include_pro(include_pro)?;

        let mut oss = OssRegistry::new();
        register_default_oss_tools(&mut oss);

        let pro = if include_pro {
            let mut pro = ProRegistry::new();
            register_default_pro_tools(&mut pro);
            Some(pro)
        } else {
            None
        };

        if include_pro {
            let capabilities = entitlement_capabilities_from_floating_provider(
                floating_license_id,
                provider_url,
                machine_id,
                customer_id,
            )?;

            return Ok(Self {
                runtime: RuntimeMode::Entitled(OwnedToolRuntimeWithCapabilities::new(
                    CompositeRegistry { oss, pro },
                    RuntimeOptions {
                        max_tier: fallback_tier,
                        expose_locked_tools: false,
                    },
                    capabilities,
                )),
            });
        }

        let _ = (provider_url, floating_license_id, machine_id, customer_id);

        Ok(Self {
            runtime: RuntimeMode::Tier(
                ToolRuntimeBuilder::new(CompositeRegistry { oss, pro })
                    .max_tier(fallback_tier)
                    .build(),
            ),
        })
    }

    #[cfg(not(feature = "pro"))]
    pub fn new_with_options(include_pro: bool, max_tier: LicenseTier) -> Result<Self, ToolError> {
        validate_include_pro(include_pro)?;
        let mut oss = OssRegistry::new();
        register_default_oss_tools(&mut oss);

        Ok(Self {
            runtime: RuntimeMode::Tier(
                ToolRuntimeBuilder::new(CompositeRegistry { oss })
                    .max_tier(max_tier)
                    .build(),
            ),
        })
    }

    #[cfg(feature = "pro")]
    pub fn new_with_entitlement_json(
        include_pro: bool,
        fallback_tier: LicenseTier,
        signed_entitlement_json: &str,
        public_key_kid: &str,
        public_key_b64url: &str,
    ) -> Result<Self, ToolError> {
        validate_include_pro(include_pro)?;
        let mut oss = OssRegistry::new();
        register_default_oss_tools(&mut oss);

        let pro = if include_pro {
            let mut pro = ProRegistry::new();
            register_default_pro_tools(&mut pro);
            Some(pro)
        } else {
            None
        };

        let capabilities = entitlement_capabilities_from_json(
            signed_entitlement_json,
            public_key_kid,
            public_key_b64url,
        )?;

        Ok(Self {
            runtime: RuntimeMode::Entitled(OwnedToolRuntimeWithCapabilities::new(
                CompositeRegistry { oss, pro },
                RuntimeOptions {
                    max_tier: fallback_tier,
                    expose_locked_tools: false,
                },
                capabilities,
            )),
        })
    }

    #[cfg(not(feature = "pro"))]
    pub fn new_with_entitlement_json(
        include_pro: bool,
        fallback_tier: LicenseTier,
        signed_entitlement_json: &str,
        public_key_kid: &str,
        public_key_b64url: &str,
    ) -> Result<Self, ToolError> {
        validate_include_pro(include_pro)?;
        let mut oss = OssRegistry::new();
        register_default_oss_tools(&mut oss);

        let capabilities = entitlement_capabilities_from_json(
            signed_entitlement_json,
            public_key_kid,
            public_key_b64url,
        )?;

        Ok(Self {
            runtime: RuntimeMode::Entitled(OwnedToolRuntimeWithCapabilities::new(
                CompositeRegistry { oss },
                RuntimeOptions {
                    max_tier: fallback_tier,
                    expose_locked_tools: false,
                },
                capabilities,
            )),
        })
    }

    pub fn list_tools_json(&self) -> Value {
        let tools: Vec<Value> = self
            .list_visible_manifests()
            .into_iter()
            .map(|m| json!(m))
            .collect();
        Value::Array(tools)
    }

    pub fn run_tool_json(&self, tool_id: &str, args_json: &str) -> Result<Value, ToolError> {
        let args = parse_args_json(args_json)?;

        let response = self.execute(ExecuteRequest {
            tool_id: tool_id.to_string(),
            args,
        })?;
        Ok(Value::Object(response.outputs.into_iter().collect()))
    }

    pub fn run_tool_json_with_progress(&self, tool_id: &str, args_json: &str) -> Result<Value, ToolError> {
        let args = parse_args_json(args_json)?;

        let response = self.execute(ExecuteRequest {
            tool_id: tool_id.to_string(),
            args,
        })?;

        Ok(json!({
            "tool_id": response.tool_id,
            "outputs": response.outputs,
            "progress": response.progress,
        }))
    }

    pub fn run_tool_json_with_progress_sink(
        &self,
        tool_id: &str,
        args_json: &str,
        progress: &dyn ProgressSink,
    ) -> Result<Value, ToolError> {
        let args = parse_args_json(args_json)?;
        let response = self.execute_with_progress_sink(
            ExecuteRequest {
                tool_id: tool_id.to_string(),
                args,
            },
            progress,
        )?;

        Ok(json!({
            "tool_id": response.tool_id,
            "outputs": response.outputs,
            "progress": response.progress,
        }))
    }

    fn list_visible_manifests(&self) -> Vec<ToolManifest> {
        match &self.runtime {
            RuntimeMode::Tier(runtime) => runtime.list_visible_manifests(),
            RuntimeMode::Entitled(runtime) => runtime.list_visible_manifests(),
        }
    }

    fn execute(&self, req: ExecuteRequest) -> Result<wbcore::ExecuteResponse, ToolError> {
        match &self.runtime {
            RuntimeMode::Tier(runtime) => runtime.execute(req),
            RuntimeMode::Entitled(runtime) => runtime.execute(req),
        }
    }

    fn execute_with_progress_sink(
        &self,
        req: ExecuteRequest,
        progress: &dyn ProgressSink,
    ) -> Result<wbcore::ExecuteResponse, ToolError> {
        match &self.runtime {
            RuntimeMode::Tier(runtime) => runtime.execute_with_progress_sink(req, progress),
            RuntimeMode::Entitled(runtime) => runtime.execute_with_progress_sink(req, progress),
        }
    }
}

fn entitlement_capabilities_from_json(
    signed_entitlement_json: &str,
    public_key_kid: &str,
    public_key_b64url: &str,
) -> Result<EntitlementCapabilities, ToolError> {
    let mut key_store = VerificationKeyStore::new();
    key_store
        .insert_base64url_public_key(public_key_kid, public_key_b64url)
        .map_err(map_license_error)?;
    let verified = verify_signed_entitlement_json(signed_entitlement_json, &key_store, current_unix())
        .map_err(map_license_error)?;
    Ok(EntitlementCapabilities::from_verified(&verified, current_unix()))
}

#[cfg(feature = "pro")]
fn entitlement_capabilities_from_floating_provider(
    floating_license_id: &str,
    provider_url: Option<&str>,
    machine_id: Option<&str>,
    customer_id: Option<&str>,
) -> Result<EntitlementCapabilities, ToolError> {
    let base = provider_url
        .map(|s| s.to_string())
        .or_else(|| env::var("WBW_LICENSE_PROVIDER_URL").ok())
        .ok_or_else(|| {
            ToolError::LicenseDenied(
                "floating-license startup requires provider_url or WBW_LICENSE_PROVIDER_URL"
                    .to_string(),
            )
        })?;

    let machine = machine_id
        .map(|s| s.to_string())
        .or_else(|| env::var("WBW_MACHINE_ID").ok())
        .unwrap_or_else(|| "local-machine".to_string());

    let customer = customer_id
        .map(|s| s.to_string())
        .or_else(|| env::var("WBW_CUSTOMER_ID").ok());

    let activation_url = format!("{}/api/v2/entitlements/activate-floating", base.trim_end_matches('/'));
    let mut body = json!({
        "floating_license_id": floating_license_id,
        "machine_id": machine,
        "product": "whitebox_next_gen"
    });
    if let Some(customer_id) = customer {
        body["customer_id"] = Value::String(customer_id);
    }

    let activation_resp = ureq::post(&activation_url)
        .send_json(body)
        .map_err(|e| ToolError::LicenseDenied(format!("floating activation failed: {e}")))?;
    let activation_json: Value = activation_resp
        .into_json()
        .map_err(|e| ToolError::LicenseDenied(format!("invalid activation response json: {e}")))?;

    let kid = activation_json
        .get("kid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::LicenseDenied("activation response missing 'kid'".to_string()))?;
    let signed_entitlement_json = serde_json::to_string(&activation_json)
        .map_err(|e| ToolError::LicenseDenied(format!("failed to serialize entitlement envelope: {e}")))?;

    let keys_url = format!("{}/api/v2/public-keys", base.trim_end_matches('/'));
    let keys_resp = ureq::get(&keys_url)
        .call()
        .map_err(|e| ToolError::LicenseDenied(format!("public-key fetch failed: {e}")))?;
    let keys_json: Value = keys_resp
        .into_json()
        .map_err(|e| ToolError::LicenseDenied(format!("invalid public-keys response json: {e}")))?;

    let public_key_b64url = keys_json
        .get("keys")
        .and_then(|v| v.as_array())
        .and_then(|keys| {
            keys.iter().find_map(|k| {
                let k_kid = k.get("kid")?.as_str()?;
                if k_kid == kid {
                    k.get("public_key_b64url")?.as_str()
                } else {
                    None
                }
            })
        })
        .ok_or_else(|| {
            ToolError::LicenseDenied(format!(
                "provider did not return public key for kid '{kid}'"
            ))
        })?;

    entitlement_capabilities_from_json(&signed_entitlement_json, kid, public_key_b64url)
}

fn current_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn map_license_error(err: LicenseError) -> ToolError {
    ToolError::LicenseDenied(err.to_string())
}

fn read_entitlement_file(path: &str) -> Result<String, ToolError> {
    std::fs::read_to_string(path)
        .map_err(|e| ToolError::InvalidRequest(format!("failed to read entitlement file '{path}': {e}")))
}

fn parse_args_json(args_json: &str) -> Result<ToolArgs, ToolError> {
    let value: Value = serde_json::from_str(args_json)
        .map_err(|e| ToolError::Validation(format!("invalid JSON arguments: {e}")))?;

    let map = value
        .as_object()
        .ok_or_else(|| ToolError::Validation("arguments must be a JSON object".to_string()))?;

    let mut args = ToolArgs::new();
    for (k, v) in map {
        args.insert(k.clone(), v.clone());
    }
    Ok(args)
}

pub fn parse_tier(tier: &str) -> Result<LicenseTier, ToolError> {
    match tier.to_ascii_lowercase().as_str() {
        "open" => Ok(LicenseTier::Open),
        "pro" => Ok(LicenseTier::Pro),
        "enterprise" => Ok(LicenseTier::Enterprise),
        _ => Err(ToolError::InvalidRequest(format!(
            "invalid tier '{tier}', expected open|pro|enterprise"
        ))),
    }
}

pub fn list_tools_json() -> Result<String, ToolError> {
    serde_json::to_string(&RToolRuntime::new().list_tools_json())
        .map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

pub fn list_tools_json_with_options(include_pro: bool, tier: &str) -> Result<String, ToolError> {
    let parsed_tier = parse_tier(tier)?;
    let rt = RToolRuntime::new_with_options(include_pro, parsed_tier)?;
    serde_json::to_string(&rt.list_tools_json())
        .map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

pub fn list_tools_json_with_entitlement_options(
    signed_entitlement_json: &str,
    public_key_kid: &str,
    public_key_b64url: &str,
    include_pro: bool,
    fallback_tier: &str,
) -> Result<String, ToolError> {
    let parsed_tier = parse_tier(fallback_tier)?;
    let rt = RToolRuntime::new_with_entitlement_json(
        include_pro,
        parsed_tier,
        signed_entitlement_json,
        public_key_kid,
        public_key_b64url,
    )?;
    serde_json::to_string(&rt.list_tools_json())
        .map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

pub fn list_tools_json_with_entitlement_file_options(
    entitlement_file: &str,
    public_key_kid: &str,
    public_key_b64url: &str,
    include_pro: bool,
    fallback_tier: &str,
) -> Result<String, ToolError> {
    let signed_entitlement_json = read_entitlement_file(entitlement_file)?;
    list_tools_json_with_entitlement_options(
        &signed_entitlement_json,
        public_key_kid,
        public_key_b64url,
        include_pro,
        fallback_tier,
    )
}

#[cfg(feature = "pro")]
pub fn list_tools_json_with_floating_license_id_options(
    floating_license_id: &str,
    include_pro: bool,
    fallback_tier: &str,
    provider_url: Option<&str>,
    machine_id: Option<&str>,
    customer_id: Option<&str>,
) -> Result<String, ToolError> {
    let parsed_tier = parse_tier(fallback_tier)?;
    let rt = RToolRuntime::new_with_floating_license_id(
        include_pro,
        parsed_tier,
        floating_license_id,
        provider_url,
        machine_id,
        customer_id,
    )?;
    serde_json::to_string(&rt.list_tools_json())
        .map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

#[cfg(not(feature = "pro"))]
pub fn list_tools_json_with_floating_license_id_options(
    _floating_license_id: &str,
    include_pro: bool,
    fallback_tier: &str,
    _provider_url: Option<&str>,
    _machine_id: Option<&str>,
    _customer_id: Option<&str>,
) -> Result<String, ToolError> {
    list_tools_json_with_options(include_pro, fallback_tier)
}

pub fn run_tool_json(tool_id: &str, args_json: &str) -> Result<String, ToolError> {
    let out = RToolRuntime::new().run_tool_json(tool_id, args_json)?;
    serde_json::to_string(&out).map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

pub fn run_tool_json_with_progress(tool_id: &str, args_json: &str) -> Result<String, ToolError> {
    let out = RToolRuntime::new().run_tool_json_with_progress(tool_id, args_json)?;
    serde_json::to_string(&out).map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

pub fn run_tool_json_with_options(
    tool_id: &str,
    args_json: &str,
    include_pro: bool,
    tier: &str,
) -> Result<String, ToolError> {
    let parsed_tier = parse_tier(tier)?;
    let out = RToolRuntime::new_with_options(include_pro, parsed_tier)?.run_tool_json(tool_id, args_json)?;
    serde_json::to_string(&out).map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

pub fn run_tool_json_with_entitlement_options(
    tool_id: &str,
    args_json: &str,
    signed_entitlement_json: &str,
    public_key_kid: &str,
    public_key_b64url: &str,
    include_pro: bool,
    fallback_tier: &str,
) -> Result<String, ToolError> {
    let parsed_tier = parse_tier(fallback_tier)?;
    let out = RToolRuntime::new_with_entitlement_json(
        include_pro,
        parsed_tier,
        signed_entitlement_json,
        public_key_kid,
        public_key_b64url,
    )?
    .run_tool_json(tool_id, args_json)?;
    serde_json::to_string(&out).map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

pub fn run_tool_json_with_progress_entitlement_options(
    tool_id: &str,
    args_json: &str,
    signed_entitlement_json: &str,
    public_key_kid: &str,
    public_key_b64url: &str,
    include_pro: bool,
    fallback_tier: &str,
) -> Result<String, ToolError> {
    let parsed_tier = parse_tier(fallback_tier)?;
    let out = RToolRuntime::new_with_entitlement_json(
        include_pro,
        parsed_tier,
        signed_entitlement_json,
        public_key_kid,
        public_key_b64url,
    )?
    .run_tool_json_with_progress(tool_id, args_json)?;
    serde_json::to_string(&out).map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

pub fn run_tool_json_with_entitlement_file_options(
    tool_id: &str,
    args_json: &str,
    entitlement_file: &str,
    public_key_kid: &str,
    public_key_b64url: &str,
    include_pro: bool,
    fallback_tier: &str,
) -> Result<String, ToolError> {
    let signed_entitlement_json = read_entitlement_file(entitlement_file)?;
    run_tool_json_with_entitlement_options(
        tool_id,
        args_json,
        &signed_entitlement_json,
        public_key_kid,
        public_key_b64url,
        include_pro,
        fallback_tier,
    )
}

pub fn run_tool_json_with_progress_entitlement_file_options(
    tool_id: &str,
    args_json: &str,
    entitlement_file: &str,
    public_key_kid: &str,
    public_key_b64url: &str,
    include_pro: bool,
    fallback_tier: &str,
) -> Result<String, ToolError> {
    let signed_entitlement_json = read_entitlement_file(entitlement_file)?;
    run_tool_json_with_progress_entitlement_options(
        tool_id,
        args_json,
        &signed_entitlement_json,
        public_key_kid,
        public_key_b64url,
        include_pro,
        fallback_tier,
    )
}

#[cfg(feature = "pro")]
pub fn run_tool_json_with_floating_license_id_options(
    tool_id: &str,
    args_json: &str,
    floating_license_id: &str,
    include_pro: bool,
    fallback_tier: &str,
    provider_url: Option<&str>,
    machine_id: Option<&str>,
    customer_id: Option<&str>,
) -> Result<String, ToolError> {
    let parsed_tier = parse_tier(fallback_tier)?;
    let out = RToolRuntime::new_with_floating_license_id(
        include_pro,
        parsed_tier,
        floating_license_id,
        provider_url,
        machine_id,
        customer_id,
    )?
    .run_tool_json(tool_id, args_json)?;
    serde_json::to_string(&out).map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

#[cfg(feature = "pro")]
pub fn run_tool_json_with_progress_floating_license_id_options(
    tool_id: &str,
    args_json: &str,
    floating_license_id: &str,
    include_pro: bool,
    fallback_tier: &str,
    provider_url: Option<&str>,
    machine_id: Option<&str>,
    customer_id: Option<&str>,
) -> Result<String, ToolError> {
    let parsed_tier = parse_tier(fallback_tier)?;
    let out = RToolRuntime::new_with_floating_license_id(
        include_pro,
        parsed_tier,
        floating_license_id,
        provider_url,
        machine_id,
        customer_id,
    )?
    .run_tool_json_with_progress(tool_id, args_json)?;
    serde_json::to_string(&out).map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

#[cfg(not(feature = "pro"))]
pub fn run_tool_json_with_floating_license_id_options(
    tool_id: &str,
    args_json: &str,
    _floating_license_id: &str,
    include_pro: bool,
    fallback_tier: &str,
    _provider_url: Option<&str>,
    _machine_id: Option<&str>,
    _customer_id: Option<&str>,
) -> Result<String, ToolError> {
    run_tool_json_with_options(tool_id, args_json, include_pro, fallback_tier)
}

#[cfg(not(feature = "pro"))]
pub fn run_tool_json_with_progress_floating_license_id_options(
    tool_id: &str,
    args_json: &str,
    _floating_license_id: &str,
    include_pro: bool,
    fallback_tier: &str,
    _provider_url: Option<&str>,
    _machine_id: Option<&str>,
    _customer_id: Option<&str>,
) -> Result<String, ToolError> {
    run_tool_json_with_progress_options(tool_id, args_json, include_pro, fallback_tier)
}

pub fn run_tool_json_with_progress_options(
    tool_id: &str,
    args_json: &str,
    include_pro: bool,
    tier: &str,
) -> Result<String, ToolError> {
    let parsed_tier = parse_tier(tier)?;
    let out = RToolRuntime::new_with_options(include_pro, parsed_tier)?
        .run_tool_json_with_progress(tool_id, args_json)?;
    serde_json::to_string(&out).map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

pub fn generate_wrapper_stubs_json_with_options(
    include_pro: bool,
    tier: &str,
    target: &str,
) -> Result<String, ToolError> {
    let parsed_tier = parse_tier(tier)?;
    let rt = RToolRuntime::new_with_options(include_pro, parsed_tier)?;
    let target = match target.to_ascii_lowercase().as_str() {
        "python" => BindingTarget::Python,
        "r" => BindingTarget::R,
        _ => {
            return Err(ToolError::InvalidRequest(
                "invalid target, expected 'python' or 'r'".to_string(),
            ))
        }
    };

    let mut stubs = serde_json::Map::new();
    for manifest in rt.list_visible_manifests() {
        stubs.insert(manifest.id.clone(), Value::String(generate_wrapper_stub(&manifest, target)));
    }
    serde_json::to_string(&Value::Object(stubs))
        .map_err(|e| ToolError::Execution(format!("serialization error: {e}")))
}

pub fn generate_r_wrapper_module_with_options(
    include_pro: bool,
    tier: &str,
) -> Result<String, ToolError> {
    let parsed_tier = parse_tier(tier)?;
    let rt = RToolRuntime::new_with_options(include_pro, parsed_tier)?;

    let mut manifests = rt.list_visible_manifests();
    manifests.sort_by(|a, b| a.id.cmp(&b.id));

    let mut out = String::new();
    out.push_str("# Auto-generated wbw_r wrappers\n");
    out.push_str("# Regenerate via generate_r_wrapper_module_with_options(include_pro, tier).\n\n");
    out.push_str("wbw_make_session <- function(floating_license_id = NULL, include_pro = NULL, tier = \"");
    out.push_str(tier);
    out.push_str("\", provider_url = NULL, machine_id = NULL, customer_id = NULL) {\n");
    out.push_str("  resolved_include_pro <- if (is.null(include_pro)) !is.null(floating_license_id) else include_pro\n");
    out.push_str("\n");
    out.push_str("  run_tool <- function(tool_id, args = list()) {\n");
    out.push_str("    args_json <- jsonlite::toJSON(args, auto_unbox = TRUE, null = \"null\")\n");
    out.push_str("    if (!is.null(floating_license_id)) {\n");
    out.push_str("      out_json <- run_tool_json_with_floating_license_id_options(\n");
    out.push_str("        tool_id,\n");
    out.push_str("        args_json,\n");
    out.push_str("        floating_license_id,\n");
    out.push_str("        resolved_include_pro,\n");
    out.push_str("        tier,\n");
    out.push_str("        provider_url,\n");
    out.push_str("        machine_id,\n");
    out.push_str("        customer_id\n");
    out.push_str("      )\n");
    out.push_str("    } else {\n");
    out.push_str("      out_json <- run_tool_json_with_options(tool_id, args_json, resolved_include_pro, tier)\n");
    out.push_str("    }\n");
    out.push_str("    out <- jsonlite::fromJSON(out_json, simplifyVector = FALSE)\n");
    out.push_str("    wbw_coerce_tool_output(out, session = session)\n");
    out.push_str("  }\n\n");
    out.push_str("  list_tools <- function() {\n");
    out.push_str("    if (!is.null(floating_license_id)) {\n");
    out.push_str("      out_json <- list_tools_json_with_floating_license_id_options(\n");
    out.push_str("        floating_license_id,\n");
    out.push_str("        resolved_include_pro,\n");
    out.push_str("        tier,\n");
    out.push_str("        provider_url,\n");
    out.push_str("        machine_id,\n");
    out.push_str("        customer_id\n");
    out.push_str("      )\n");
    out.push_str("    } else {\n");
    out.push_str("      out_json <- list_tools_json_with_options(resolved_include_pro, tier)\n");
    out.push_str("    }\n");
    out.push_str("    jsonlite::fromJSON(out_json, simplifyVector = FALSE)\n");
    out.push_str("  }\n\n");
    out.push_str("  session <- new.env(parent = emptyenv())\n");
    out.push_str("  session$run_tool <- run_tool\n");
    out.push_str("  session$list_tools <- list_tools\n");

    for manifest in manifests {
        let fn_name = manifest.id.replace('-', "_");
        out.push_str(&format!(
            "  session${fn_name} <- function(...) {{\n    # {summary}\n    run_tool(\"{tool_id}\", list(...))\n  }}\n",
            fn_name = fn_name,
            summary = manifest.summary.replace('\n', " "),
            tool_id = manifest.id,
        ));
    }

    out.push_str("\n  session\n");
    out.push_str("}\n\n");

    out.push_str(&format!(
        "wbw_run_tool <- function(tool_id, args = list()) {{\n  session <- wbw_make_session(include_pro = {}, tier = \"{}\")\n  session$run_tool(tool_id, args)\n}}\n\n",
        if include_pro { "TRUE" } else { "FALSE" },
        tier,
    ));

    for manifest in rt.list_visible_manifests() {
        let fn_name = manifest.id.replace('-', "_");
        out.push_str(&format!(
            "{fn_name} <- function(...) {{\n  # {summary}\n  session <- wbw_make_session(include_pro = {}, tier = \"{}\")\n  session${fn_name}(...)\n}}\n\n",
            if include_pro { "TRUE" } else { "FALSE" },
            tier,
            fn_name = fn_name,
            summary = manifest.summary.replace('\n', " "),
        ));
    }

    Ok(out)
}

#[cfg(feature = "pro")]
pub fn whitebox_tools(
    floating_license_id: Option<&str>,
    include_pro: Option<bool>,
    tier: &str,
    provider_url: Option<&str>,
    machine_id: Option<&str>,
    customer_id: Option<&str>,
) -> Result<RToolRuntime, ToolError> {
    let parsed_tier = parse_tier(tier)?;
    let resolved_include_pro = include_pro.unwrap_or(floating_license_id.is_some());

    if let Some(license_id) = floating_license_id {
        RToolRuntime::new_with_floating_license_id(
            resolved_include_pro,
            parsed_tier,
            license_id,
            provider_url,
            machine_id,
            customer_id,
        )
    } else {
        RToolRuntime::new_with_options(resolved_include_pro, parsed_tier)
    }
}

#[cfg(not(feature = "pro"))]
pub fn whitebox_tools(
    _floating_license_id: Option<&str>,
    include_pro: Option<bool>,
    tier: &str,
    _provider_url: Option<&str>,
    _machine_id: Option<&str>,
    _customer_id: Option<&str>,
) -> Result<RToolRuntime, ToolError> {
    let parsed_tier = parse_tier(tier)?;
    let resolved_include_pro = include_pro.unwrap_or(false);
    RToolRuntime::new_with_options(resolved_include_pro, parsed_tier)
}

mod native_exports {
    use super::*;
    use extendr_api::prelude::{extendr, extendr_module, Nullable};

    fn map_extendr_err(err: ToolError) -> extendr_api::Error {
        extendr_api::Error::Other(err.to_string())
    }

    fn nullable_string_to_option(value: Nullable<String>) -> Option<String> {
        match value {
            Nullable::NotNull(v) => Some(v),
            Nullable::Null => None,
        }
    }

    #[extendr]
    fn list_tools_json() -> extendr_api::Result<String> {
        super::list_tools_json().map_err(map_extendr_err)
    }

    #[extendr]
    fn list_tools_json_with_options(include_pro: bool, tier: &str) -> extendr_api::Result<String> {
        super::list_tools_json_with_options(include_pro, tier).map_err(map_extendr_err)
    }

    #[extendr]
    fn list_tools_json_with_entitlement_options(
        signed_entitlement_json: &str,
        public_key_kid: &str,
        public_key_b64url: &str,
        include_pro: bool,
        fallback_tier: &str,
    ) -> extendr_api::Result<String> {
        super::list_tools_json_with_entitlement_options(
            signed_entitlement_json,
            public_key_kid,
            public_key_b64url,
            include_pro,
            fallback_tier,
        )
        .map_err(map_extendr_err)
    }

    #[extendr]
    fn list_tools_json_with_entitlement_file_options(
        entitlement_file: &str,
        public_key_kid: &str,
        public_key_b64url: &str,
        include_pro: bool,
        fallback_tier: &str,
    ) -> extendr_api::Result<String> {
        super::list_tools_json_with_entitlement_file_options(
            entitlement_file,
            public_key_kid,
            public_key_b64url,
            include_pro,
            fallback_tier,
        )
        .map_err(map_extendr_err)
    }

    #[extendr]
    fn list_tools_json_with_floating_license_id_options(
        floating_license_id: &str,
        include_pro: bool,
        fallback_tier: &str,
        provider_url: Nullable<String>,
        machine_id: Nullable<String>,
        customer_id: Nullable<String>,
    ) -> extendr_api::Result<String> {
        let provider_url = nullable_string_to_option(provider_url);
        let machine_id = nullable_string_to_option(machine_id);
        let customer_id = nullable_string_to_option(customer_id);
        super::list_tools_json_with_floating_license_id_options(
            floating_license_id,
            include_pro,
            fallback_tier,
            provider_url.as_deref(),
            machine_id.as_deref(),
            customer_id.as_deref(),
        )
        .map_err(map_extendr_err)
    }

    #[extendr]
    fn run_tool_json(tool_id: &str, args_json: &str) -> extendr_api::Result<String> {
        super::run_tool_json(tool_id, args_json).map_err(map_extendr_err)
    }

    #[extendr]
    fn run_tool_json_with_progress(tool_id: &str, args_json: &str) -> extendr_api::Result<String> {
        super::run_tool_json_with_progress(tool_id, args_json).map_err(map_extendr_err)
    }

    #[extendr]
    fn run_tool_json_with_options(
        tool_id: &str,
        args_json: &str,
        include_pro: bool,
        tier: &str,
    ) -> extendr_api::Result<String> {
        super::run_tool_json_with_options(tool_id, args_json, include_pro, tier)
            .map_err(map_extendr_err)
    }

    #[extendr]
    fn run_tool_json_with_progress_options(
        tool_id: &str,
        args_json: &str,
        include_pro: bool,
        tier: &str,
    ) -> extendr_api::Result<String> {
        super::run_tool_json_with_progress_options(tool_id, args_json, include_pro, tier)
            .map_err(map_extendr_err)
    }

    #[extendr]
    fn run_tool_json_with_entitlement_options(
        tool_id: &str,
        args_json: &str,
        signed_entitlement_json: &str,
        public_key_kid: &str,
        public_key_b64url: &str,
        include_pro: bool,
        fallback_tier: &str,
    ) -> extendr_api::Result<String> {
        super::run_tool_json_with_entitlement_options(
            tool_id,
            args_json,
            signed_entitlement_json,
            public_key_kid,
            public_key_b64url,
            include_pro,
            fallback_tier,
        )
        .map_err(map_extendr_err)
    }

    #[extendr]
    fn run_tool_json_with_progress_entitlement_options(
        tool_id: &str,
        args_json: &str,
        signed_entitlement_json: &str,
        public_key_kid: &str,
        public_key_b64url: &str,
        include_pro: bool,
        fallback_tier: &str,
    ) -> extendr_api::Result<String> {
        super::run_tool_json_with_progress_entitlement_options(
            tool_id,
            args_json,
            signed_entitlement_json,
            public_key_kid,
            public_key_b64url,
            include_pro,
            fallback_tier,
        )
        .map_err(map_extendr_err)
    }

    #[extendr]
    fn run_tool_json_with_entitlement_file_options(
        tool_id: &str,
        args_json: &str,
        entitlement_file: &str,
        public_key_kid: &str,
        public_key_b64url: &str,
        include_pro: bool,
        fallback_tier: &str,
    ) -> extendr_api::Result<String> {
        super::run_tool_json_with_entitlement_file_options(
            tool_id,
            args_json,
            entitlement_file,
            public_key_kid,
            public_key_b64url,
            include_pro,
            fallback_tier,
        )
        .map_err(map_extendr_err)
    }

    #[extendr]
    fn run_tool_json_with_progress_entitlement_file_options(
        tool_id: &str,
        args_json: &str,
        entitlement_file: &str,
        public_key_kid: &str,
        public_key_b64url: &str,
        include_pro: bool,
        fallback_tier: &str,
    ) -> extendr_api::Result<String> {
        super::run_tool_json_with_progress_entitlement_file_options(
            tool_id,
            args_json,
            entitlement_file,
            public_key_kid,
            public_key_b64url,
            include_pro,
            fallback_tier,
        )
        .map_err(map_extendr_err)
    }

    #[extendr]
    fn run_tool_json_with_floating_license_id_options(
        tool_id: &str,
        args_json: &str,
        floating_license_id: &str,
        include_pro: bool,
        fallback_tier: &str,
        provider_url: Nullable<String>,
        machine_id: Nullable<String>,
        customer_id: Nullable<String>,
    ) -> extendr_api::Result<String> {
        let provider_url = nullable_string_to_option(provider_url);
        let machine_id = nullable_string_to_option(machine_id);
        let customer_id = nullable_string_to_option(customer_id);
        super::run_tool_json_with_floating_license_id_options(
            tool_id,
            args_json,
            floating_license_id,
            include_pro,
            fallback_tier,
            provider_url.as_deref(),
            machine_id.as_deref(),
            customer_id.as_deref(),
        )
        .map_err(map_extendr_err)
    }

    #[extendr]
    fn run_tool_json_with_progress_floating_license_id_options(
        tool_id: &str,
        args_json: &str,
        floating_license_id: &str,
        include_pro: bool,
        fallback_tier: &str,
        provider_url: Nullable<String>,
        machine_id: Nullable<String>,
        customer_id: Nullable<String>,
    ) -> extendr_api::Result<String> {
        let provider_url = nullable_string_to_option(provider_url);
        let machine_id = nullable_string_to_option(machine_id);
        let customer_id = nullable_string_to_option(customer_id);
        super::run_tool_json_with_progress_floating_license_id_options(
            tool_id,
            args_json,
            floating_license_id,
            include_pro,
            fallback_tier,
            provider_url.as_deref(),
            machine_id.as_deref(),
            customer_id.as_deref(),
        )
        .map_err(map_extendr_err)
    }

    #[extendr]
    fn generate_r_wrapper_module_with_options(include_pro: bool, tier: &str) -> extendr_api::Result<String> {
        super::generate_r_wrapper_module_with_options(include_pro, tier).map_err(map_extendr_err)
    }

    #[extendr]
    fn lidar_metadata_json(path: &str) -> extendr_api::Result<String> {
        super::lidar_metadata_json(path).map_err(map_extendr_err)
    }

    #[extendr]
    fn sensor_bundle_metadata_json(path: &str) -> extendr_api::Result<String> {
        super::sensor_bundle_metadata_json(path).map_err(map_extendr_err)
    }

    #[extendr]
    fn sensor_bundle_resolve_raster_path(
        bundle_root: &str,
        key: &str,
        key_type: &str,
    ) -> extendr_api::Result<String> {
        super::sensor_bundle_resolve_raster_path(bundle_root, key, key_type)
            .map_err(map_extendr_err)
    }

    #[extendr]
    fn vector_copy_to_path(src: &str, dst: &str) -> extendr_api::Result<()> {
        super::vector_copy_to_path(src, dst).map_err(map_extendr_err)
    }

    #[extendr]
    fn vector_copy_with_options_json(src: &str, dst: &str, options_json: &str) -> extendr_api::Result<String> {
        super::vector_copy_with_options_json(src, dst, options_json).map_err(map_extendr_err)
    }

    #[extendr]
    fn lidar_copy_to_path(src: &str, dst: &str) -> extendr_api::Result<String> {
        super::lidar_copy_to_path(src, dst).map_err(map_extendr_err)
    }

    #[extendr]
    fn lidar_write_with_options_json(src: &str, dst: &str, options_json: &str) -> extendr_api::Result<String> {
        super::lidar_write_with_options_json(src, dst, options_json).map_err(map_extendr_err)
    }

    #[extendr]
    fn raster_write_with_options_json(src: &str, dst: &str, options_json: &str) -> extendr_api::Result<()> {
        super::raster_write_with_options_json(src, dst, options_json).map_err(map_extendr_err)
    }

    #[extendr]
    fn raster_metadata_json(path: &str) -> extendr_api::Result<String> {
        super::raster_metadata_json(path).map_err(map_extendr_err)
    }

    #[extendr]
    fn vector_metadata_json(path: &str) -> extendr_api::Result<String> {
        super::vector_metadata_json(path).map_err(map_extendr_err)
    }

    #[extendr]
    fn projection_to_ogc_wkt(epsg: i32) -> extendr_api::Result<String> {
        if epsg <= 0 {
            return Err("epsg must be a positive integer".into());
        }
        super::projection_to_ogc_wkt(epsg as u32).map_err(map_extendr_err)
    }

    #[extendr]
    fn projection_identify_epsg(crs_text: &str) -> extendr_api::Result<Nullable<i32>> {
        let code = super::projection_identify_epsg(crs_text).map_err(map_extendr_err)?;
        Ok(match code {
            Some(value) => Nullable::NotNull(value as i32),
            None => Nullable::Null,
        })
    }

    #[extendr]
    fn projection_reproject_points_json(
        points_json: &str,
        src_epsg: i32,
        dst_epsg: i32,
    ) -> extendr_api::Result<String> {
        if src_epsg <= 0 || dst_epsg <= 0 {
            return Err("src_epsg and dst_epsg must be positive integers".into());
        }
        super::projection_reproject_points_json(points_json, src_epsg as u32, dst_epsg as u32)
            .map_err(map_extendr_err)
    }

    #[extendr]
    fn projection_reproject_point_json(
        x: f64,
        y: f64,
        src_epsg: i32,
        dst_epsg: i32,
    ) -> extendr_api::Result<String> {
        if src_epsg <= 0 || dst_epsg <= 0 {
            return Err("src_epsg and dst_epsg must be positive integers".into());
        }
        super::projection_reproject_point_json(x, y, src_epsg as u32, dst_epsg as u32)
            .map_err(map_extendr_err)
    }

    #[extendr]
    fn topology_intersects_wkt(a_wkt: &str, b_wkt: &str) -> extendr_api::Result<bool> {
        super::topology_intersects_wkt(a_wkt, b_wkt).map_err(map_extendr_err)
    }

    #[extendr]
    fn topology_contains_wkt(a_wkt: &str, b_wkt: &str) -> extendr_api::Result<bool> {
        super::topology_contains_wkt(a_wkt, b_wkt).map_err(map_extendr_err)
    }

    #[extendr]
    fn topology_within_wkt(a_wkt: &str, b_wkt: &str) -> extendr_api::Result<bool> {
        super::topology_within_wkt(a_wkt, b_wkt).map_err(map_extendr_err)
    }

    #[extendr]
    fn topology_touches_wkt(a_wkt: &str, b_wkt: &str) -> extendr_api::Result<bool> {
        super::topology_touches_wkt(a_wkt, b_wkt).map_err(map_extendr_err)
    }

    #[extendr]
    fn topology_disjoint_wkt(a_wkt: &str, b_wkt: &str) -> extendr_api::Result<bool> {
        super::topology_disjoint_wkt(a_wkt, b_wkt).map_err(map_extendr_err)
    }

    #[extendr]
    fn topology_crosses_wkt(a_wkt: &str, b_wkt: &str) -> extendr_api::Result<bool> {
        super::topology_crosses_wkt(a_wkt, b_wkt).map_err(map_extendr_err)
    }

    #[extendr]
    fn topology_overlaps_wkt(a_wkt: &str, b_wkt: &str) -> extendr_api::Result<bool> {
        super::topology_overlaps_wkt(a_wkt, b_wkt).map_err(map_extendr_err)
    }

    #[extendr]
    fn topology_covers_wkt(a_wkt: &str, b_wkt: &str) -> extendr_api::Result<bool> {
        super::topology_covers_wkt(a_wkt, b_wkt).map_err(map_extendr_err)
    }

    #[extendr]
    fn topology_covered_by_wkt(a_wkt: &str, b_wkt: &str) -> extendr_api::Result<bool> {
        super::topology_covered_by_wkt(a_wkt, b_wkt).map_err(map_extendr_err)
    }

    #[extendr]
    fn topology_relate_wkt(a_wkt: &str, b_wkt: &str) -> extendr_api::Result<String> {
        super::topology_relate_wkt(a_wkt, b_wkt).map_err(map_extendr_err)
    }

    #[extendr]
    fn topology_distance_wkt(a_wkt: &str, b_wkt: &str) -> extendr_api::Result<f64> {
        super::topology_distance_wkt(a_wkt, b_wkt).map_err(map_extendr_err)
    }

    #[extendr]
    fn topology_vector_feature_relation_json(
        a_path: &str,
        a_feature_index: i32,
        b_path: &str,
        b_feature_index: i32,
    ) -> extendr_api::Result<String> {
        if a_feature_index < 0 || b_feature_index < 0 {
            return Err("feature indices must be >= 0".into());
        }
        super::topology_vector_feature_relation_json(
            a_path,
            a_feature_index as usize,
            b_path,
            b_feature_index as usize,
        )
        .map_err(map_extendr_err)
    }

    #[extendr]
    fn topology_is_valid_polygon_wkt(wkt: &str) -> extendr_api::Result<bool> {
        super::topology_is_valid_polygon_wkt(wkt).map_err(map_extendr_err)
    }

    #[extendr]
    fn topology_make_valid_polygon_wkt(wkt: &str, epsilon: f64) -> extendr_api::Result<String> {
        super::topology_make_valid_polygon_wkt(wkt, epsilon).map_err(map_extendr_err)
    }

    #[extendr]
    fn topology_buffer_wkt(wkt: &str, distance: f64) -> extendr_api::Result<String> {
        super::topology_buffer_wkt(wkt, distance).map_err(map_extendr_err)
    }

    extendr_module! {
        mod wbw_r;
        fn list_tools_json;
        fn list_tools_json_with_options;
        fn list_tools_json_with_entitlement_options;
        fn list_tools_json_with_entitlement_file_options;
        fn list_tools_json_with_floating_license_id_options;
        fn run_tool_json;
        fn run_tool_json_with_progress;
        fn run_tool_json_with_options;
        fn run_tool_json_with_progress_options;
        fn run_tool_json_with_entitlement_options;
        fn run_tool_json_with_progress_entitlement_options;
        fn run_tool_json_with_entitlement_file_options;
        fn run_tool_json_with_progress_entitlement_file_options;
        fn run_tool_json_with_floating_license_id_options;
        fn run_tool_json_with_progress_floating_license_id_options;
        fn generate_r_wrapper_module_with_options;
        fn lidar_metadata_json;
        fn sensor_bundle_metadata_json;
        fn sensor_bundle_resolve_raster_path;
        fn vector_copy_to_path;
        fn vector_copy_with_options_json;
        fn lidar_copy_to_path;
        fn lidar_write_with_options_json;
        fn raster_write_with_options_json;
        fn raster_metadata_json;
        fn vector_metadata_json;
        fn projection_to_ogc_wkt;
        fn projection_identify_epsg;
        fn projection_reproject_points_json;
        fn projection_reproject_point_json;
        fn topology_intersects_wkt;
        fn topology_contains_wkt;
        fn topology_within_wkt;
        fn topology_touches_wkt;
        fn topology_disjoint_wkt;
        fn topology_crosses_wkt;
        fn topology_overlaps_wkt;
        fn topology_covers_wkt;
        fn topology_covered_by_wkt;
        fn topology_relate_wkt;
        fn topology_distance_wkt;
        fn topology_vector_feature_relation_json;
        fn topology_is_valid_polygon_wkt;
        fn topology_make_valid_polygon_wkt;
        fn topology_buffer_wkt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "pro")]
    use std::sync::OnceLock;
    use std::sync::Mutex;
    use wbcore::ProgressEvent;

    #[derive(Default)]
    struct TestCollectSink {
        events: Mutex<Vec<ProgressEvent>>,
    }

    impl ProgressSink for TestCollectSink {
        fn info(&self, msg: &str) {
            if let Ok(mut events) = self.events.lock() {
                events.push(ProgressEvent::Info(msg.to_string()));
            }
        }

        fn progress(&self, pct: f64) {
            if let Ok(mut events) = self.events.lock() {
                events.push(ProgressEvent::Percent(pct));
            }
        }
    }

    #[cfg(feature = "pro")]
    fn license_env_lock() -> &'static std::sync::Mutex<()> {
        static LOCK: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
    }

    #[cfg(feature = "pro")]
    struct EnvGuard {
        saved: Vec<(String, Option<String>)>,
    }

    #[cfg(feature = "pro")]
    impl EnvGuard {
        fn set(entries: &[(&str, Option<String>)]) -> Self {
            let mut saved = Vec::with_capacity(entries.len());
            for (key, new_val) in entries {
                saved.push(((*key).to_string(), std::env::var(key).ok()));
                match new_val {
                    Some(v) => unsafe { std::env::set_var(key, v) },
                    None => unsafe { std::env::remove_var(key) },
                }
            }
            Self { saved }
        }
    }

    #[cfg(feature = "pro")]
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, old_val) in &self.saved {
                match old_val {
                    Some(v) => unsafe { std::env::set_var(key, v) },
                    None => unsafe { std::env::remove_var(key) },
                }
            }
        }
    }

    #[cfg(feature = "pro")]
    fn unique_missing_state_path(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "wbw_r_license_state_{}_{}_{}.json",
            tag,
            std::process::id(),
            nanos
        ))
    }

    #[test]
    fn list_tools_contains_known_tool() {
        let rt = RToolRuntime::new();
        let tools = rt.list_tools_json();
        let arr = tools.as_array().expect("list should be an array");
        let has_add = arr
            .iter()
            .any(|v| v.get("id").and_then(Value::as_str) == Some("add"));
        assert!(has_add);
    }

    #[test]
    #[cfg(feature = "pro")]
    fn run_tool_json_executes_registry_tool() {
        let rt = RToolRuntime::new_with_options(true, LicenseTier::Pro)
            .expect("pro runtime construction should succeed");
        let out = rt
            .run_tool_json("raster_power", "{\"input\":[2,3],\"exponent\":2}")
            .expect("tool should run");

        assert_eq!(out.get("result"), Some(&json!([4.0, 9.0])));
    }

    #[test]
    fn pro_tools_hidden_without_pro_options() {
        let rt = RToolRuntime::new();
        let tools = rt.list_tools_json();
        let arr = tools.as_array().expect("list should be an array");
        let has_pro = arr
            .iter()
            .any(|v| v.get("id").and_then(Value::as_str) == Some("raster_power"));
        assert!(!has_pro);
    }

    #[test]
    #[cfg(feature = "pro")]
    fn run_tool_json_with_progress_returns_progress_events() {
        let rt = RToolRuntime::new_with_options(true, LicenseTier::Pro)
            .expect("pro runtime construction should succeed");
        let out = rt
            .run_tool_json_with_progress("raster_power", "{\"input\":[2],\"exponent\":2}")
            .expect("tool should run");

        let progress = out
            .get("progress")
            .and_then(Value::as_array)
            .expect("progress should be array");
        assert!(!progress.is_empty());
    }

    #[test]
    #[cfg(feature = "pro")]
    fn run_tool_json_with_progress_sink_emits_live_events() {
        let rt = RToolRuntime::new_with_options(true, LicenseTier::Pro)
            .expect("pro runtime construction should succeed");
        let sink = TestCollectSink::default();
        let _ = rt
            .run_tool_json_with_progress_sink("raster_power", "{\"input\":[2],\"exponent\":2}", &sink)
            .expect("tool should run");

        let events = sink.events.lock().expect("events lock");
        assert!(!events.is_empty());
    }

    #[test]
    #[cfg(feature = "pro")]
    fn pro_tools_visible_and_runnable_with_pro_options() {
        let rt = RToolRuntime::new_with_options(true, LicenseTier::Pro)
            .expect("pro runtime construction should succeed");
        let tools = rt.list_tools_json();
        let arr = tools.as_array().expect("list should be an array");
        let has_pro = arr
            .iter()
            .any(|v| v.get("id").and_then(Value::as_str) == Some("raster_power"));
        assert!(has_pro);

        let out = rt
            .run_tool_json("raster_power", "{\"input\":[2,3],\"exponent\":2}")
            .expect("pro tool should run");
        assert_eq!(out.get("result"), Some(&json!([4.0, 9.0])));
    }

    #[test]
    #[cfg(feature = "pro")]
    fn provider_bootstrap_fail_open_with_missing_state_defaults_to_open() {
        let env_guard = license_env_lock().lock().expect("env lock");
        let state_path = unique_missing_state_path("fail_open");
        let _ = std::fs::remove_file(&state_path);

        let _guard = EnvGuard::set(&[
            ("WBW_LICENSE_PROVIDER_URL", Some("http://127.0.0.1:9".to_string())),
            ("WBW_LICENSE_POLICY", Some("fail_open".to_string())),
            (
                "WBW_LICENSE_STATE_PATH",
                Some(state_path.to_string_lossy().to_string()),
            ),
            ("WBW_LICENSE_LEASE_SECONDS", Some("3600".to_string())),
        ]);

        let rt = RToolRuntime::new_with_options(true, LicenseTier::Open)
            .expect("fail-open bootstrap should not block runtime construction");

        let tools = rt.list_tools_json();
        let arr = tools.as_array().expect("list should be an array");
        let has_pro = arr
            .iter()
            .any(|v| v.get("id").and_then(Value::as_str) == Some("raster_power"));
        assert!(!has_pro, "expected OSS/open fallback to hide pro tools");

        let _ = std::fs::remove_file(state_path);
        drop(env_guard);
    }

    #[test]
    #[cfg(feature = "pro")]
    fn provider_bootstrap_fail_closed_with_missing_state_returns_error() {
        let env_guard = license_env_lock().lock().expect("env lock");
        let state_path = unique_missing_state_path("fail_closed");
        let _ = std::fs::remove_file(&state_path);

        let _guard = EnvGuard::set(&[
            ("WBW_LICENSE_PROVIDER_URL", Some("http://127.0.0.1:9".to_string())),
            ("WBW_LICENSE_POLICY", Some("fail_closed".to_string())),
            (
                "WBW_LICENSE_STATE_PATH",
                Some(state_path.to_string_lossy().to_string()),
            ),
            ("WBW_LICENSE_LEASE_SECONDS", Some("3600".to_string())),
        ]);

        match RToolRuntime::new_with_options(true, LicenseTier::Open) {
            Ok(_) => panic!("fail-closed bootstrap should reject runtime construction"),
            Err(err) => assert!(matches!(err, ToolError::LicenseDenied(_))),
        }

        let _ = std::fs::remove_file(state_path);
        drop(env_guard);
    }

    #[test]
    #[cfg(feature = "pro")]
    fn floating_bootstrap_requires_provider_url_when_not_in_env() {
        let env_guard = license_env_lock().lock().expect("env lock");
        let _guard = EnvGuard::set(&[("WBW_LICENSE_PROVIDER_URL", None)]);

        let err = match RToolRuntime::new_with_floating_license_id(
            true,
            LicenseTier::Open,
            "fl_test",
            None,
            Some("test-machine"),
            None,
        ) {
            Ok(_) => panic!("missing provider URL should be rejected"),
            Err(err) => err,
        };

        match err {
            ToolError::LicenseDenied(msg) => {
                assert!(msg.contains("requires provider_url"));
            }
            other => panic!("expected LicenseDenied, got {other}"),
        }

        drop(env_guard);
    }

    #[test]
    #[cfg(feature = "pro")]
    fn floating_bootstrap_rejects_unreachable_provider() {
        let env_guard = license_env_lock().lock().expect("env lock");

        let err = match RToolRuntime::new_with_floating_license_id(
            true,
            LicenseTier::Open,
            "fl_test",
            Some("http://127.0.0.1:9"),
            Some("test-machine"),
            None,
        ) {
            Ok(_) => panic!("unreachable provider should be rejected"),
            Err(err) => err,
        };

        match err {
            ToolError::LicenseDenied(msg) => {
                assert!(msg.contains("floating activation failed"));
            }
            other => panic!("expected LicenseDenied, got {other}"),
        }

        drop(env_guard);
    }

    #[test]
    #[cfg(not(feature = "pro"))]
    fn include_pro_rejected_when_pro_feature_disabled() {
        match RToolRuntime::new_with_options(true, LicenseTier::Pro) {
            Ok(_) => panic!("include_pro should be rejected without 'pro' feature"),
            Err(err) => assert!(matches!(err, ToolError::InvalidRequest(_))),
        }
    }

    #[test]
    fn invalid_tier_rejected() {
        let err = parse_tier("gold").expect_err("should reject invalid tier");
        assert!(matches!(err, ToolError::InvalidRequest(_)));
    }

    #[test]
    fn wrapper_stub_generation_returns_known_tool() {
        let txt = generate_wrapper_stubs_json_with_options(false, "open", "r")
            .expect("stub generation should succeed");
        let value: Value = serde_json::from_str(&txt).expect("valid JSON output");
        assert!(value.get("add").is_some());
    }

    #[test]
    fn r_wrapper_module_generation_contains_helper_and_known_tool() {
        let txt = generate_r_wrapper_module_with_options(false, "open")
            .expect("R wrapper module generation should succeed");
        assert!(txt.contains("wbw_make_session <- function"));
        assert!(txt.contains("wbw_run_tool <- function"));
        assert!(txt.contains("run_tool_json_with_options"));
        assert!(txt.contains("list_tools_json_with_options"));
        assert!(txt.contains("session$add <- function"));
        assert!(txt.contains("add <- function"));
    }

    #[test]
    fn r_wrapper_module_generation_matches_manifest_count_and_names() {
        let rt = RToolRuntime::new_with_options(false, LicenseTier::Open)
            .expect("runtime construction should succeed");
        let manifests = rt.list_visible_manifests();

        let txt = generate_r_wrapper_module_with_options(false, "open")
            .expect("R wrapper module generation should succeed");

        let function_def_count = txt.matches(" <- function(").count();
        assert_eq!(
            function_def_count,
            (manifests.len() * 2) + 4,
            "generated module should include session/global wrappers plus helper functions"
        );

        for manifest in manifests {
            let fn_name = manifest.id.replace('-', "_");
            assert!(
                txt.contains(&format!("session${fn_name} <- function(")),
                "missing generated session wrapper for manifest id '{}'",
                manifest.id
            );
            assert!(
                txt.contains(&format!("{fn_name} <- function(")),
                "missing generated global wrapper for manifest id '{}'",
                manifest.id
            );
        }
    }
}
