//! Unified frontend API for LiDAR point clouds.
//!
//! This module provides:
//! * [`PointCloud`]: an in-memory container shared across formats.
//! * [`LidarFormat`]: format enum with extension/signature detection.
//! * [`read`]/[`write()`]: generic path-based I/O helpers.

use std::fs::File;
use std::io::{BufReader, BufWriter, Read};
use std::path::Path;

use crate::copc::{CopcNodePointOrdering, CopcReader, CopcWriter, CopcWriterConfig};
use crate::crs::Crs;
use crate::e57::{E57Reader, E57Writer, E57WriterConfig};
use crate::io::{PointReader, PointWriter};
use crate::las::{LasReader, LasWriter, PointDataFormat, WriterConfig};
use crate::laz::{parse_laszip_vlr, LazReader, LazWriter, LazWriterConfig};
use crate::ply::{PlyEncoding, PlyReader, PlyWriter};
use crate::reproject::{
    points_in_place_to_epsg_with_options,
    points_to_epsg_with_options,
    points_to_epsg_with_options_and_progress,
    LidarReprojectOptions,
};
use wide::f64x4;
use crate::{Error, PointRecord, Result};

/// Unified in-memory LiDAR dataset.
#[derive(Debug, Clone, Default)]
pub struct PointCloud {
    /// Point records.
    pub points: Vec<PointRecord>,
    /// Optional CRS metadata associated with the dataset.
    pub crs: Option<Crs>,
}

/// Diagnostics emitted by tolerant decode/recovery paths during read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReadDiagnostics {
    /// Number of Point14 layered chunks/nodes that required partial recovery.
    pub point14_partial_events: u64,
    /// Total points decoded from partially recovered Point14 chunks/nodes.
    pub point14_partial_decoded_points: u64,
    /// Total expected points from partially recovered Point14 chunks/nodes.
    pub point14_partial_expected_points: u64,
}

/// Optional format-specific write controls used by LiDAR output helpers.
#[derive(Debug, Clone, Default)]
pub struct LidarWriteOptions {
    /// LAZ-specific write controls.
    pub laz: LazWriteOptions,
    /// COPC-specific write controls.
    pub copc: CopcWriteOptions,
}

/// Optional write controls for LAZ output.
#[derive(Debug, Clone, Default)]
pub struct LazWriteOptions {
    /// LAZ points-per-chunk value.
    pub chunk_size: Option<u32>,
    /// LAZ compression tuning level in the range 0-9.
    pub compression_level: Option<u32>,
}

/// Optional write controls for COPC output.
#[derive(Debug, Clone, Default)]
pub struct CopcWriteOptions {
    /// Maximum points kept in a node before subdivision.
    pub max_points_per_node: Option<usize>,
    /// Maximum octree depth.
    pub max_depth: Option<u32>,
    /// Point ordering policy within nodes.
    pub node_point_ordering: Option<CopcNodePointOrdering>,
}

impl PointCloud {
    /// Read a point cloud from `path`, auto-detecting format.
    ///
    /// # Errors
    /// Returns an error if the file cannot be opened, parsed, or decoded.
    pub fn read<P: AsRef<Path>>(path: P) -> Result<Self> {
        read(path)
    }

    /// Write this point cloud to `path`, inferring output format from extension.
    ///
    /// # Errors
    /// Returns an error if extension-based format detection fails or encoding fails.
    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        write_auto(self, path)
    }

    /// Write this point cloud to `path` in an explicitly selected format.
    ///
    /// # Errors
    /// Returns an error if the file cannot be created or encoded.
    pub fn write_as<P: AsRef<Path>>(&self, path: P, format: LidarFormat) -> Result<()> {
        write(self, path, format)
    }

    /// Write this point cloud to `path`, inferring output format from extension
    /// and applying optional format-specific write controls.
    ///
    /// # Errors
    /// Returns an error when extension-based format detection fails or encoding fails.
    pub fn write_with_options<P: AsRef<Path>>(&self, path: P, options: &LidarWriteOptions) -> Result<()> {
        write_auto_with_options(self, path, options)
    }

    /// Write this point cloud to `path` in an explicitly selected format,
    /// applying optional format-specific write controls.
    ///
    /// # Errors
    /// Returns an error if the file cannot be created or encoded.
    pub fn write_as_with_options<P: AsRef<Path>>(
        &self,
        path: P,
        format: LidarFormat,
        options: &LidarWriteOptions,
    ) -> Result<()> {
        write_with_options(self, path, format, options)
    }

    /// Assign a CRS to this point cloud using an EPSG code.
    ///
    /// Replaces the entire `crs` struct with a new `Crs` containing only the EPSG code.
    /// Any existing `wkt` field is cleared to ensure CRS consistency.
    pub fn assign_crs_epsg(&mut self, epsg: u32) {
        self.crs = Some(Crs {
            epsg: Some(epsg),
            wkt: None,
        });
    }

    /// Assign a CRS to this point cloud using WKT text.
    ///
    /// Replaces the entire `crs` struct with a new `Crs` containing only the WKT definition.
    /// Any existing `epsg` field is cleared to ensure CRS consistency.
    pub fn assign_crs_wkt(&mut self, wkt: &str) {
        self.crs = Some(Crs {
            epsg: None,
            wkt: Some(wkt.to_string()),
        });
    }

    /// Return a reprojected copy of this point cloud and updated CRS metadata.
    ///
    /// Source CRS is taken from `self.crs.epsg` and destination CRS is set to
    /// `dst_epsg` in the returned cloud.
    ///
    /// # Errors
    /// Returns an error when source CRS EPSG is missing or transformation fails.
    pub fn reprojected_to_epsg(&self, dst_epsg: u32) -> Result<Self> {
        self.reprojected_to_epsg_with_options(dst_epsg, &LidarReprojectOptions::default())
    }

    /// Return a reprojected copy of this point cloud and updated CRS metadata,
    /// using custom reprojection options.
    ///
    /// # Errors
    /// Returns an error when source CRS EPSG is missing or transformation fails.
    pub fn reprojected_to_epsg_with_options(
        &self,
        dst_epsg: u32,
        options: &LidarReprojectOptions,
    ) -> Result<Self> {
        let src_crs = self.crs.as_ref().ok_or_else(|| Error::Projection(
            "PointCloud reprojection requires source CRS metadata in cloud.crs".to_string(),
        ))?;

        let points = points_to_epsg_with_options(&self.points, src_crs, dst_epsg, options)?;
        Ok(Self {
            points,
            crs: Some(Crs::from_epsg(dst_epsg)),
        })
    }

    /// Return a reprojected copy of this point cloud while emitting progress
    /// updates in the range [0, 1] as points are completed.
    pub fn reprojected_to_epsg_with_options_and_progress<F>(
        &self,
        dst_epsg: u32,
        options: &LidarReprojectOptions,
        progress: F,
    ) -> Result<Self>
    where
        F: Fn(f64) + Send + Sync,
    {
        let src_crs = self.crs.as_ref().ok_or_else(|| Error::Projection(
            "PointCloud reprojection requires source CRS metadata in cloud.crs".to_string(),
        ))?;

        let points =
            points_to_epsg_with_options_and_progress(&self.points, src_crs, dst_epsg, options, progress)?;
        Ok(Self {
            points,
            crs: Some(Crs::from_epsg(dst_epsg)),
        })
    }

    /// Reproject this point cloud in place and update CRS metadata to `dst_epsg`.
    ///
    /// # Errors
    /// Returns an error when source CRS EPSG is missing or transformation fails.
    pub fn reproject_in_place_to_epsg(&mut self, dst_epsg: u32) -> Result<()> {
        self.reproject_in_place_to_epsg_with_options(dst_epsg, &LidarReprojectOptions::default())
    }

    /// Reproject this point cloud in place and update CRS metadata to `dst_epsg`,
    /// using custom reprojection options.
    ///
    /// # Errors
    /// Returns an error when source CRS metadata is missing or transformation fails.
    pub fn reproject_in_place_to_epsg_with_options(
        &mut self,
        dst_epsg: u32,
        options: &LidarReprojectOptions,
    ) -> Result<()> {
        let crs = self.crs.as_mut().ok_or_else(|| Error::Projection(
            "PointCloud reprojection requires source CRS metadata in cloud.crs".to_string(),
        ))?;
        points_in_place_to_epsg_with_options(&mut self.points, crs, dst_epsg, options)
    }
}

/// Supported LiDAR file formats in the unified API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LidarFormat {
    /// LAS (uncompressed).
    Las,
    /// LAZ (LASzip-compressed LAS).
    Laz,
    /// COPC (Cloud Optimized Point Cloud, usually `.copc.las`).
    Copc,
    /// PLY (ASCII or binary).
    Ply,
    /// E57.
    E57,
}

impl LidarFormat {
    /// Detect format from extension and file signature.
    ///
    /// # Errors
    /// Returns an error when the format cannot be identified.
    pub fn detect<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if let Some(ext_format) = detect_by_extension(path) {
            return Ok(ext_format);
        }

        let mut f = File::open(path)?;
        let mut sig = [0u8; 16];
        let n = f.read(&mut sig)?;
        let s = &sig[..n];

        if s.starts_with(b"ply\n") {
            return Ok(Self::Ply);
        }
        if s.starts_with(crate::e57::E57_SIGNATURE) {
            return Ok(Self::E57);
        }
        if s.starts_with(b"LASF") {
            let lower = path.to_string_lossy().to_ascii_lowercase();
            if lower.ends_with(".copc.las") || lower.ends_with(".copc.laz") {
                return Ok(Self::Copc);
            }
            if lower.ends_with(".laz") {
                return Ok(Self::Laz);
            }
            return Ok(Self::Las);
        }

        Err(Error::InvalidValue {
            field: "format",
            detail: format!("unable to detect LiDAR format for path: {}", path.display()),
        })
    }
}

/// Read a point cloud from `path`, auto-detecting input format.
///
/// # Errors
/// Returns an error if detection, parsing, or decoding fails.
pub fn read<P: AsRef<Path>>(path: P) -> Result<PointCloud> {
    read_with_diagnostics(path).map(|(cloud, _diag)| cloud)
}

/// Read a point cloud from `path`, returning both cloud data and diagnostics.
///
/// # Errors
/// Returns an error if detection, parsing, or decoding fails.
pub fn read_with_diagnostics<P: AsRef<Path>>(path: P) -> Result<(PointCloud, ReadDiagnostics)> {
    let path = path.as_ref();
    match LidarFormat::detect(path)? {
        LidarFormat::Las => {
            let mut reader = LasReader::new(BufReader::new(File::open(path)?))?;
            let crs = reader.crs().cloned();
            let points = reader.read_all()?;
            Ok((PointCloud { points, crs }, ReadDiagnostics::default()))
        }
        LidarFormat::Laz => {
            let laz_file = File::open(path)?;
            let mut reader = match LazReader::new(BufReader::new(laz_file)) {
                Ok(r) => r,
                Err(e) => {
                    if is_unexpected_eof(&e) && laz_declares_point14(path)? {
                        return Err(Error::Unimplemented(
                            "standard LASzip Point14 layered stream detected, but arithmetic layered decoding is not yet implemented in wblidar standard backend",
                        ));
                    }
                    return Err(e);
                }
            };
            let crs = reader.crs().cloned();
            let points = match reader.read_all() {
                Ok(p) => p,
                Err(e) => {
                    if is_unexpected_eof(&e) && laz_declares_point14(path)? {
                        return Err(Error::Unimplemented(
                            "standard LASzip Point14 layered stream detected, but arithmetic layered decoding is not yet implemented in wblidar standard backend",
                        ));
                    }
                    return Err(e);
                }
            };
            let (events, decoded, expected) = reader.point14_partial_recovery_stats();
            Ok((
                PointCloud { points, crs },
                ReadDiagnostics {
                    point14_partial_events: events,
                    point14_partial_decoded_points: decoded,
                    point14_partial_expected_points: expected,
                },
            ))
        }
        LidarFormat::Copc => {
            // Read COPC points through node traversal.
            let mut reader = CopcReader::new(BufReader::new(File::open(path)?))?;
            let points = reader.read_all_nodes()?;

            // Re-read LAS header/VLRs to extract CRS metadata.
            let las_reader = LasReader::new(BufReader::new(File::open(path)?))?;
            let crs = las_reader.crs().cloned();
            let (events, decoded, expected) = reader.point14_partial_recovery_stats();

            Ok((
                PointCloud { points, crs },
                ReadDiagnostics {
                    point14_partial_events: events,
                    point14_partial_decoded_points: decoded,
                    point14_partial_expected_points: expected,
                },
            ))
        }
        LidarFormat::Ply => {
            let mut reader = PlyReader::new(BufReader::new(File::open(path)?))?;
            let points = reader.read_all()?;
            Ok((PointCloud { points, crs: None }, ReadDiagnostics::default()))
        }
        LidarFormat::E57 => {
            let mut reader = E57Reader::new(BufReader::new(File::open(path)?))?;
            let points = reader.read_all()?;
            Ok((PointCloud { points, crs: None }, ReadDiagnostics::default()))
        }
    }
}

fn is_unexpected_eof(err: &Error) -> bool {
    matches!(err, Error::Io(e) if e.kind() == std::io::ErrorKind::UnexpectedEof)
}

fn laz_declares_point14(path: &Path) -> Result<bool> {
    let las_reader = LasReader::new(BufReader::new(File::open(path)?))?;
    Ok(parse_laszip_vlr(las_reader.vlrs())
        .as_ref()
        .map(|info| info.has_point14_item())
        .unwrap_or(false))
}

/// Write a point cloud to `path` in a specified format.
///
/// # Errors
/// Returns an error if writing or finalization fails.
pub fn write<P: AsRef<Path>>(cloud: &PointCloud, path: P, format: LidarFormat) -> Result<()> {
    let path = path.as_ref();
    match format {
        LidarFormat::Las => write_las(cloud, path),
        LidarFormat::Laz => write_laz(cloud, path),
        LidarFormat::Copc => write_copc(cloud, path),
        LidarFormat::Ply => write_ply(cloud, path),
        LidarFormat::E57 => write_e57(cloud, path),
    }
}

/// Write a point cloud to `path` in a specified format, applying optional
/// format-specific write controls.
///
/// # Errors
/// Returns an error if writing or finalization fails.
pub fn write_with_options<P: AsRef<Path>>(
    cloud: &PointCloud,
    path: P,
    format: LidarFormat,
    options: &LidarWriteOptions,
) -> Result<()> {
    let path = path.as_ref();
    match format {
        LidarFormat::Las => write_las(cloud, path),
        LidarFormat::Laz => write_laz_with_options(cloud, path, &options.laz),
        LidarFormat::Copc => write_copc_with_options(cloud, path, &options.copc),
        LidarFormat::Ply => write_ply(cloud, path),
        LidarFormat::E57 => write_e57(cloud, path),
    }
}

/// Write a point cloud to `path`, inferring output format from extension.
///
/// # Errors
/// Returns an error when extension-based format detection fails or writing fails.
pub fn write_auto<P: AsRef<Path>>(cloud: &PointCloud, path: P) -> Result<()> {
    let path = path.as_ref();
    let format = detect_by_extension(path).ok_or_else(|| Error::InvalidValue {
        field: "format",
        detail: format!(
            "unable to infer output format from extension for path: {}",
            path.display()
        ),
    })?;
    write(cloud, path, format)
}

/// Write a point cloud to `path`, inferring output format from extension,
/// and applying optional format-specific write controls.
///
/// # Errors
/// Returns an error when extension-based format detection fails or writing fails.
pub fn write_auto_with_options<P: AsRef<Path>>(
    cloud: &PointCloud,
    path: P,
    options: &LidarWriteOptions,
) -> Result<()> {
    let path = path.as_ref();
    let format = detect_by_extension(path).ok_or_else(|| Error::InvalidValue {
        field: "format",
        detail: format!(
            "unable to infer output format from extension for path: {}",
            path.display()
        ),
    })?;
    write_with_options(cloud, path, format, options)
}

fn detect_by_extension(path: &Path) -> Option<LidarFormat> {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    if lower.ends_with(".copc.las") || lower.ends_with(".copc.laz") {
        return Some(LidarFormat::Copc);
    }
    if lower.ends_with(".laz") {
        return Some(LidarFormat::Laz);
    }
    if lower.ends_with(".las") {
        return Some(LidarFormat::Las);
    }
    if lower.ends_with(".ply") {
        return Some(LidarFormat::Ply);
    }
    if lower.ends_with(".e57") {
        return Some(LidarFormat::E57);
    }
    None
}

fn write_las(cloud: &PointCloud, path: &Path) -> Result<()> {
    let out = BufWriter::new(File::create(path)?);
    let mut cfg = default_las_config(cloud);
    cfg.crs = cloud.crs.clone();
    let mut writer = LasWriter::new(out, cfg)?;
    writer.write_all_points(&cloud.points)?;
    writer.finish()
}

fn write_laz(cloud: &PointCloud, path: &Path) -> Result<()> {
    write_laz_with_options(cloud, path, &LazWriteOptions::default())
}

fn write_laz_with_options(cloud: &PointCloud, path: &Path, options: &LazWriteOptions) -> Result<()> {
    let out = BufWriter::new(File::create(path)?);
    let mut cfg = LazWriterConfig::default();
    cfg.las = default_las_config(cloud);
    cfg.las.crs = cloud.crs.clone();
    if let Some(chunk_size) = options.chunk_size {
        cfg.chunk_size = chunk_size;
    }
    if let Some(compression_level) = options.compression_level {
        cfg.compression_level = compression_level;
    }
    let mut writer = LazWriter::new(out, cfg)?;
    writer.write_all_points(&cloud.points)?;
    writer.finish()
}

fn write_copc(cloud: &PointCloud, path: &Path) -> Result<()> {
    write_copc_with_options(cloud, path, &CopcWriteOptions::default())
}

fn write_copc_with_options(cloud: &PointCloud, path: &Path, options: &CopcWriteOptions) -> Result<()> {
    let out = BufWriter::new(File::create(path)?);
    let mut cfg = default_copc_config(cloud);
    cfg.las.crs = cloud.crs.clone();
    if let Some(max_points_per_node) = options.max_points_per_node {
        cfg.max_points_per_node = max_points_per_node;
    }
    if let Some(max_depth) = options.max_depth {
        cfg.max_depth = max_depth;
    }
    if let Some(node_point_ordering) = options.node_point_ordering {
        cfg.node_point_ordering = node_point_ordering;
    }
    let mut writer = CopcWriter::new(out, cfg);
    writer.write_all_points(&cloud.points)?;
    writer.finish()
}

fn write_ply(cloud: &PointCloud, path: &Path) -> Result<()> {
    let out = BufWriter::new(File::create(path)?);
    let has_color = cloud.points.iter().any(|p| p.color.is_some());
    let has_normals = cloud.points.iter().any(|p| {
        p.normal_x.is_some() || p.normal_y.is_some() || p.normal_z.is_some()
    });
    let mut writer = PlyWriter::new(
        out,
        cloud.points.len() as u64,
        PlyEncoding::BinaryLittleEndian,
        has_color,
        has_normals,
    )?;
    writer.write_all_points(&cloud.points)?;
    writer.finish()
}

fn write_e57(cloud: &PointCloud, path: &Path) -> Result<()> {
    let out = BufWriter::new(File::create(path)?);
    let has_color = cloud.points.iter().any(|p| p.color.is_some());
    let has_intensity = cloud.points.iter().any(|p| p.intensity > 0);
    let cfg = E57WriterConfig {
        has_color,
        has_intensity,
        ..E57WriterConfig::default()
    };
    let mut writer = E57Writer::new(out, cfg);
    writer.write_all_points(&cloud.points)?;
    writer.finish()
}

fn default_las_config(cloud: &PointCloud) -> WriterConfig {
    let mut cfg = WriterConfig::default();
    let has_color = cloud.points.iter().any(|p| p.color.is_some());
    let has_nir = cloud.points.iter().any(|p| p.nir.is_some());
    cfg.point_data_format = if has_nir {
        PointDataFormat::Pdrf8
    } else if has_color {
        PointDataFormat::Pdrf7
    } else {
        PointDataFormat::Pdrf6
    };
    cfg
}

fn default_copc_config(cloud: &PointCloud) -> CopcWriterConfig {
    let mut cfg = CopcWriterConfig::default();
    cfg.las = default_las_config(cloud);

    if cloud.points.is_empty() {
        return cfg;
    }

    // Accumulate bounding box using branchless SIMD min/max.
    // Layout: [x, y, z, unused].
    let inf    = f64::INFINITY;
    let neg_inf = f64::NEG_INFINITY;
    let mut acc_min = f64x4::new([inf,     inf,     inf,     inf]);
    let mut acc_max = f64x4::new([neg_inf, neg_inf, neg_inf, neg_inf]);

    for p in &cloud.points {
        let coords = f64x4::new([p.x, p.y, p.z, 0.0]);
        acc_min = acc_min.min(coords);
        acc_max = acc_max.max(coords);
    }

    let min_arr: [f64; 4] = acc_min.into();
    let max_arr: [f64; 4] = acc_max.into();
    let (min_x, min_y, min_z) = (min_arr[0], min_arr[1], min_arr[2]);
    let (max_x, max_y, max_z) = (max_arr[0], max_arr[1], max_arr[2]);

    cfg.center_x = (min_x + max_x) / 2.0;
    cfg.center_y = (min_y + max_y) / 2.0;
    cfg.center_z = (min_z + max_z) / 2.0;

    let dx = (max_x - min_x).abs();
    let dy = (max_y - min_y).abs();
    let dz = (max_z - min_z).abs();
    let extent = dx.max(dy).max(dz);

    cfg.halfsize = (extent / 2.0).max(1.0) * 1.001;
    cfg.spacing = (cfg.halfsize * 2.0 / 256.0).max(0.001);
    cfg
}

#[cfg(test)]
mod tests {
    use super::PointCloud;
    use crate::crs::Crs;
    use crate::error::Error;
    use crate::point::PointRecord;

    fn sample_cloud_with_wgs84() -> PointCloud {
        PointCloud {
            points: vec![PointRecord {
                x: -2.0,
                y: -0.5,
                ..PointRecord::default()
            }],
            crs: Some(Crs::from_epsg(4326)),
        }
    }

    #[test]
    fn reprojected_to_epsg_returns_updated_copy() {
        let cloud = sample_cloud_with_wgs84();
        let out = cloud.reprojected_to_epsg(3857).unwrap();

        assert_eq!(out.crs.as_ref().and_then(|c| c.epsg), Some(3857));
        assert!(out.points[0].x.abs() > 1000.0);
        assert!(out.points[0].y.abs() > 100.0);

        // Original cloud remains unchanged.
        assert_eq!(cloud.crs.as_ref().and_then(|c| c.epsg), Some(4326));
        assert!((cloud.points[0].x + 2.0).abs() < 1e-9);
        assert!((cloud.points[0].y + 0.5).abs() < 1e-9);
    }

    #[test]
    fn reproject_in_place_updates_points_and_crs() {
        let mut cloud = sample_cloud_with_wgs84();
        cloud.reproject_in_place_to_epsg(3857).unwrap();

        assert_eq!(cloud.crs.as_ref().and_then(|c| c.epsg), Some(3857));
        assert!(cloud.points[0].x.abs() > 1000.0);
        assert!(cloud.points[0].y.abs() > 100.0);
    }

    #[test]
    fn reprojected_to_epsg_requires_cloud_crs() {
        let cloud = PointCloud {
            points: vec![PointRecord::default()],
            crs: None,
        };

        let err = cloud.reprojected_to_epsg(3857).unwrap_err();
        assert!(matches!(err, Error::Projection(_)));
        assert!(err
            .to_string()
            .contains("PointCloud reprojection requires source CRS metadata"));
    }

    #[test]
    fn reproject_in_place_requires_cloud_crs() {
        let mut cloud = PointCloud {
            points: vec![PointRecord::default()],
            crs: None,
        };

        let err = cloud.reproject_in_place_to_epsg(3857).unwrap_err();
        assert!(matches!(err, Error::Projection(_)));
        assert!(err
            .to_string()
            .contains("PointCloud reprojection requires source CRS metadata"));
    }
}
