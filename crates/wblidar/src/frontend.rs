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

/// Named point-record fields that can be extracted to or applied from
/// columnar numeric arrays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointField {
    /// X coordinate.
    X,
    /// Y coordinate.
    Y,
    /// Z coordinate.
    Z,
    /// Return intensity.
    Intensity,
    /// ASPRS classification code.
    Classification,
    /// Return number.
    ReturnNumber,
    /// Number of returns.
    NumberOfReturns,
    /// Scan direction flag.
    ScanDirectionFlag,
    /// Edge-of-flight-line flag.
    EdgeOfFlightLine,
    /// Scan angle.
    ScanAngle,
    /// Packed LAS flags.
    Flags,
    /// User data byte.
    UserData,
    /// Point source ID.
    PointSourceId,
    /// Red channel from optional color.
    Red,
    /// Green channel from optional color.
    Green,
    /// Blue channel from optional color.
    Blue,
    /// Near infrared optional band.
    Nir,
    /// Optional GPS time value.
    GpsTime,
    /// Optional normal x component.
    NormalX,
    /// Optional normal y component.
    NormalY,
    /// Optional normal z component.
    NormalZ,
}

impl PointField {
    /// Parse a canonical field name into a [`PointField`].
    ///
    /// Accepted names use snake_case (e.g. `point_source_id`).
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "x" => Some(Self::X),
            "y" => Some(Self::Y),
            "z" => Some(Self::Z),
            "intensity" => Some(Self::Intensity),
            "classification" => Some(Self::Classification),
            "return_number" => Some(Self::ReturnNumber),
            "number_of_returns" => Some(Self::NumberOfReturns),
            "scan_direction_flag" => Some(Self::ScanDirectionFlag),
            "edge_of_flight_line" => Some(Self::EdgeOfFlightLine),
            "scan_angle" => Some(Self::ScanAngle),
            "flags" => Some(Self::Flags),
            "user_data" => Some(Self::UserData),
            "point_source_id" => Some(Self::PointSourceId),
            "red" => Some(Self::Red),
            "green" => Some(Self::Green),
            "blue" => Some(Self::Blue),
            "nir" => Some(Self::Nir),
            "gps_time" => Some(Self::GpsTime),
            "normal_x" => Some(Self::NormalX),
            "normal_y" => Some(Self::NormalY),
            "normal_z" => Some(Self::NormalZ),
            _ => None,
        }
    }

    /// Return the canonical snake_case field name.
    pub fn as_name(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Y => "y",
            Self::Z => "z",
            Self::Intensity => "intensity",
            Self::Classification => "classification",
            Self::ReturnNumber => "return_number",
            Self::NumberOfReturns => "number_of_returns",
            Self::ScanDirectionFlag => "scan_direction_flag",
            Self::EdgeOfFlightLine => "edge_of_flight_line",
            Self::ScanAngle => "scan_angle",
            Self::Flags => "flags",
            Self::UserData => "user_data",
            Self::PointSourceId => "point_source_id",
            Self::Red => "red",
            Self::Green => "green",
            Self::Blue => "blue",
            Self::Nir => "nir",
            Self::GpsTime => "gps_time",
            Self::NormalX => "normal_x",
            Self::NormalY => "normal_y",
            Self::NormalZ => "normal_z",
        }
    }
}

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

    /// Number of points currently loaded in memory.
    pub fn point_count(&self) -> usize {
        self.points.len()
    }

    /// Extract selected point fields as numeric columns.
    ///
    /// Optional fields (e.g. color, GPS time, normals) are returned as `NaN`
    /// where values are absent.
    ///
    /// # Errors
    /// Returns an error when no fields are requested.
    pub fn extract_columns(&self, fields: &[PointField]) -> Result<Vec<Vec<f64>>> {
        if fields.is_empty() {
            return Err(Error::InvalidValue {
                field: "fields",
                detail: "at least one point field must be requested".to_string(),
            });
        }

        let mut cols: Vec<Vec<f64>> = fields
            .iter()
            .map(|_| Vec::with_capacity(self.points.len()))
            .collect();

        for p in &self.points {
            for (i, field) in fields.iter().enumerate() {
                cols[i].push(field_value_as_f64(*field, p));
            }
        }
        Ok(cols)
    }

    /// Apply numeric columns to selected point fields.
    ///
    /// Optional fields accept `NaN` to clear the value (`None`).
    ///
    /// # Errors
    /// Returns an error when lengths mismatch or values are out of range for
    /// target integer fields.
    pub fn apply_columns(&mut self, fields: &[PointField], columns: &[Vec<f64>]) -> Result<()> {
        if fields.is_empty() {
            return Err(Error::InvalidValue {
                field: "fields",
                detail: "at least one point field must be provided".to_string(),
            });
        }
        if fields.len() != columns.len() {
            return Err(Error::InvalidValue {
                field: "columns",
                detail: format!(
                    "field count ({}) does not match column count ({})",
                    fields.len(),
                    columns.len()
                ),
            });
        }

        let expected = self.points.len();
        for (field, col) in fields.iter().zip(columns.iter()) {
            if col.len() != expected {
                return Err(Error::InvalidValue {
                    field: "columns",
                    detail: format!(
                        "column '{}' has length {}, expected {}",
                        field.as_name(),
                        col.len(),
                        expected
                    ),
                });
            }
        }

        for row in 0..self.points.len() {
            let p = &mut self.points[row];
            for (col_idx, field) in fields.iter().enumerate() {
                set_field_value_from_f64(*field, p, columns[col_idx][row])?;
            }
        }

        Ok(())
    }

    /// Apply numeric columns to selected fields for a contiguous row range.
    ///
    /// This is intended for chunked workflows where values are produced in
    /// batches and applied incrementally.
    ///
    /// # Errors
    /// Returns an error when lengths mismatch, range exceeds point count, or
    /// values are out of range for target integer fields.
    pub fn apply_columns_range(
        &mut self,
        start_row: usize,
        fields: &[PointField],
        columns: &[Vec<f64>],
    ) -> Result<()> {
        if fields.is_empty() {
            return Err(Error::InvalidValue {
                field: "fields",
                detail: "at least one point field must be provided".to_string(),
            });
        }
        if fields.len() != columns.len() {
            return Err(Error::InvalidValue {
                field: "columns",
                detail: format!(
                    "field count ({}) does not match column count ({})",
                    fields.len(),
                    columns.len()
                ),
            });
        }
        if start_row > self.points.len() {
            return Err(Error::InvalidValue {
                field: "start_row",
                detail: format!(
                    "start_row {} is out of bounds for {} points",
                    start_row,
                    self.points.len()
                ),
            });
        }

        let row_count = columns.first().map_or(0, Vec::len);
        for (field, col) in fields.iter().zip(columns.iter()) {
            if col.len() != row_count {
                return Err(Error::InvalidValue {
                    field: "columns",
                    detail: format!(
                        "column '{}' has length {}, expected {}",
                        field.as_name(),
                        col.len(),
                        row_count
                    ),
                });
            }
        }

        let end_row = start_row.saturating_add(row_count);
        if end_row > self.points.len() {
            return Err(Error::InvalidValue {
                field: "columns",
                detail: format!(
                    "row range [{}..{}) exceeds point count {}",
                    start_row,
                    end_row,
                    self.points.len()
                ),
            });
        }

        for local_row in 0..row_count {
            let point = &mut self.points[start_row + local_row];
            for (col_idx, field) in fields.iter().enumerate() {
                set_field_value_from_f64(*field, point, columns[col_idx][local_row])?;
            }
        }

        Ok(())
    }
}

/// Read only the declared point count from file headers.
///
/// # Errors
/// Returns an error if format detection or header parsing fails.
pub fn read_point_count<P: AsRef<Path>>(path: P) -> Result<u64> {
    let path = path.as_ref();
    match LidarFormat::detect(path)? {
        LidarFormat::Las | LidarFormat::Laz | LidarFormat::Copc => {
            let reader = LasReader::new(BufReader::new(File::open(path)?))?;
            Ok(reader.header().point_count())
        }
        LidarFormat::Ply => {
            let reader = PlyReader::new(BufReader::new(File::open(path)?))?;
            reader.point_count().ok_or_else(|| Error::InvalidValue {
                field: "point_count",
                detail: "PLY source does not expose a point count in header".to_string(),
            })
        }
        LidarFormat::E57 => {
            let reader = E57Reader::new(BufReader::new(File::open(path)?))?;
            reader.point_count().ok_or_else(|| Error::InvalidValue {
                field: "point_count",
                detail: "E57 source does not expose a point count".to_string(),
            })
        }
    }
}

/// Read selected fields as numeric columns from a LiDAR file.
///
/// # Errors
/// Returns an error if read fails or requested fields are invalid.
pub fn read_columns<P: AsRef<Path>>(path: P, fields: &[PointField]) -> Result<Vec<Vec<f64>>> {
    let cloud = read(path)?;
    cloud.extract_columns(fields)
}

/// Streaming chunk reader for selected point fields.
///
/// This reader avoids materializing an entire cloud in memory by decoding
/// points in fixed-size chunks and extracting only the requested fields.
pub struct PointColumnChunkReader {
    reader: Box<dyn PointReader>,
    fields: Vec<PointField>,
    chunk_size: usize,
    scratch: PointRecord,
}

impl PointColumnChunkReader {
    /// Open a chunk reader for `path` with requested `fields`.
    ///
    /// # Errors
    /// Returns an error when the source cannot be opened, decoded, or when
    /// `fields` is empty or `chunk_size` is zero.
    pub fn open<P: AsRef<Path>>(path: P, fields: &[PointField], chunk_size: usize) -> Result<Self> {
        if fields.is_empty() {
            return Err(Error::InvalidValue {
                field: "fields",
                detail: "at least one point field must be requested".to_string(),
            });
        }
        if chunk_size == 0 {
            return Err(Error::InvalidValue {
                field: "chunk_size",
                detail: "chunk_size must be greater than zero".to_string(),
            });
        }

        let reader = open_streaming_point_reader(path.as_ref())?;
        Ok(Self {
            reader,
            fields: fields.to_vec(),
            chunk_size,
            scratch: PointRecord::default(),
        })
    }

    /// Read the next chunk of columns.
    ///
    /// Returns `Ok(None)` when EOF is reached.
    ///
    /// # Errors
    /// Returns an error if point decoding fails.
    pub fn next_chunk(&mut self) -> Result<Option<Vec<Vec<f64>>>> {
        let mut cols: Vec<Vec<f64>> = self
            .fields
            .iter()
            .map(|_| Vec::with_capacity(self.chunk_size))
            .collect();

        for _ in 0..self.chunk_size {
            if !self.reader.read_point(&mut self.scratch)? {
                break;
            }
            for (i, field) in self.fields.iter().enumerate() {
                cols[i].push(field_value_as_f64(*field, &self.scratch));
            }
        }

        if cols.first().is_some_and(Vec::is_empty) {
            Ok(None)
        } else {
            Ok(Some(cols))
        }
    }
}

/// Read selected fields as numeric columns in fixed-size chunks.
///
/// Returns a vector of chunk matrices, where each chunk is laid out as
/// `Vec<column>`, and each column contains `f64` values for one field.
///
/// # Errors
/// Returns an error if reader setup fails, field selection is empty,
/// `chunk_size` is zero, or point decoding fails.
pub fn read_columns_chunked<P: AsRef<Path>>(
    path: P,
    fields: &[PointField],
    chunk_size: usize,
) -> Result<Vec<Vec<Vec<f64>>>> {
    let mut reader = PointColumnChunkReader::open(path, fields, chunk_size)?;
    let mut chunks = Vec::new();
    while let Some(chunk) = reader.next_chunk()? {
        chunks.push(chunk);
    }
    Ok(chunks)
}

/// Streaming chunk applier that rewrites selected fields from one lidar file
/// into another without materializing the full point cloud in memory.
///
/// Currently supports LAS/LAZ outputs.
pub struct PointColumnChunkRewriter {
    reader: Box<dyn PointReader>,
    writer: Box<dyn PointWriter>,
    fields: Vec<PointField>,
    scratch: PointRecord,
}

impl PointColumnChunkRewriter {
    /// Open a streaming rewriter.
    ///
    /// # Errors
    /// Returns an error when source/destination setup fails, fields are empty,
    /// or destination format is unsupported.
    pub fn open<PIn: AsRef<Path>, POut: AsRef<Path>>(
        input_path: PIn,
        output_path: POut,
        fields: &[PointField],
    ) -> Result<Self> {
        if fields.is_empty() {
            return Err(Error::InvalidValue {
                field: "fields",
                detail: "at least one point field must be requested".to_string(),
            });
        }

        let input_path = input_path.as_ref();
        let output_path = output_path.as_ref();
        let reader = open_streaming_point_reader(input_path)?;
        let writer = open_streaming_point_writer(input_path, output_path)?;

        Ok(Self {
            reader,
            writer,
            fields: fields.to_vec(),
            scratch: PointRecord::default(),
        })
    }

    /// Apply one chunk of columns.
    ///
    /// Columns are expected in `Vec<column>` layout and all columns must have
    /// the same row count.
    ///
    /// # Errors
    /// Returns an error when dimensions are invalid, source points are
    /// exhausted early, field values are invalid, or write fails.
    pub fn apply_chunk(&mut self, columns: &[Vec<f64>]) -> Result<()> {
        if columns.len() != self.fields.len() {
            return Err(Error::InvalidValue {
                field: "columns",
                detail: format!(
                    "field count ({}) does not match column count ({})",
                    self.fields.len(),
                    columns.len()
                ),
            });
        }

        let row_count = columns.first().map_or(0usize, Vec::len);
        for (field, col) in self.fields.iter().zip(columns.iter()) {
            if col.len() != row_count {
                return Err(Error::InvalidValue {
                    field: "columns",
                    detail: format!(
                        "column '{}' has length {}, expected {}",
                        field.as_name(),
                        col.len(),
                        row_count
                    ),
                });
            }
        }

        for row in 0..row_count {
            if !self.reader.read_point(&mut self.scratch)? {
                return Err(Error::InvalidValue {
                    field: "columns",
                    detail: "chunk rows exceed source point count".to_string(),
                });
            }
            for (col_idx, field) in self.fields.iter().enumerate() {
                set_field_value_from_f64(*field, &mut self.scratch, columns[col_idx][row])?;
            }
            self.writer.write_point(&self.scratch)?;
        }

        Ok(())
    }

    /// Finalize rewrite and validate that chunks covered all source points.
    ///
    /// # Errors
    /// Returns an error when source still has unread points or final write
    /// finalization fails.
    pub fn finish(mut self) -> Result<()> {
        if self.reader.read_point(&mut self.scratch)? {
            return Err(Error::InvalidValue {
                field: "columns",
                detail: "chunk rows did not cover full source point count".to_string(),
            });
        }
        self.writer.finish()
    }
}

/// Rewrite selected fields by applying a sequence of chunks.
///
/// This is a convenience wrapper over [`PointColumnChunkRewriter`].
pub fn rewrite_columns_chunked<PIn: AsRef<Path>, POut: AsRef<Path>>(
    input_path: PIn,
    output_path: POut,
    fields: &[PointField],
    chunks: &[Vec<Vec<f64>>],
) -> Result<()> {
    let mut rewriter = PointColumnChunkRewriter::open(input_path, output_path, fields)?;
    for chunk in chunks {
        rewriter.apply_chunk(chunk)?;
    }
    rewriter.finish()
}

fn open_streaming_point_reader(path: &Path) -> Result<Box<dyn PointReader>> {
    match LidarFormat::detect(path)? {
        LidarFormat::Las => Ok(Box::new(LasReader::new(BufReader::new(File::open(path)?))?)),
        LidarFormat::Laz => {
            let laz_file = File::open(path)?;
            match LazReader::new(BufReader::new(laz_file)) {
                Ok(r) => Ok(Box::new(r)),
                Err(e) => {
                    if is_unexpected_eof(&e) && laz_declares_point14(path)? {
                        return Err(Error::Unimplemented(
                            "standard LASzip Point14 layered stream detected, but arithmetic layered decoding is not yet implemented in wblidar standard backend",
                        ));
                    }
                    Err(e)
                }
            }
        }
        LidarFormat::Copc => Ok(Box::new(CopcReader::new(BufReader::new(File::open(path)?))?)),
        LidarFormat::Ply => Ok(Box::new(PlyReader::new(BufReader::new(File::open(path)?))?)),
        LidarFormat::E57 => Ok(Box::new(E57Reader::new(BufReader::new(File::open(path)?))?)),
    }
}

fn infer_stream_writer_config_from_source(path: &Path) -> Result<WriterConfig> {
    match LidarFormat::detect(path)? {
        LidarFormat::Las | LidarFormat::Laz | LidarFormat::Copc => {
            let reader = LasReader::new(BufReader::new(File::open(path)?))?;
            let hdr = reader.header();
            let mut cfg = WriterConfig::default();
            cfg.point_data_format = hdr.point_data_format;
            cfg.x_scale = hdr.x_scale;
            cfg.y_scale = hdr.y_scale;
            cfg.z_scale = hdr.z_scale;
            cfg.x_offset = hdr.x_offset;
            cfg.y_offset = hdr.y_offset;
            cfg.z_offset = hdr.z_offset;
            cfg.extra_bytes_per_point = hdr.extra_bytes_count;
            cfg.crs = reader.crs().cloned();
            Ok(cfg)
        }
        _ => Ok(WriterConfig::default()),
    }
}

fn open_streaming_point_writer(input_path: &Path, output_path: &Path) -> Result<Box<dyn PointWriter>> {
    let format = detect_by_extension(output_path).ok_or_else(|| Error::InvalidValue {
        field: "format",
        detail: format!(
            "unable to infer output format from extension for path: {}",
            output_path.display()
        ),
    })?;

    let cfg = infer_stream_writer_config_from_source(input_path)?;
    match format {
        LidarFormat::Las => {
            let writer = LasWriter::new(BufWriter::new(File::create(output_path)?), cfg)?;
            Ok(Box::new(writer))
        }
        LidarFormat::Laz => {
            let laz_cfg = LazWriterConfig {
                las: cfg,
                ..LazWriterConfig::default()
            };
            let writer = LazWriter::new(BufWriter::new(File::create(output_path)?), laz_cfg)?;
            Ok(Box::new(writer))
        }
        _ => Err(Error::Unimplemented(
            "chunked streaming rewrite currently supports only LAS/LAZ outputs",
        )),
    }
}

fn field_value_as_f64(field: PointField, p: &PointRecord) -> f64 {
    match field {
        PointField::X => p.x,
        PointField::Y => p.y,
        PointField::Z => p.z,
        PointField::Intensity => f64::from(p.intensity),
        PointField::Classification => f64::from(p.classification),
        PointField::ReturnNumber => f64::from(p.return_number),
        PointField::NumberOfReturns => f64::from(p.number_of_returns),
        PointField::ScanDirectionFlag => {
            if p.scan_direction_flag { 1.0 } else { 0.0 }
        }
        PointField::EdgeOfFlightLine => {
            if p.edge_of_flight_line { 1.0 } else { 0.0 }
        }
        PointField::ScanAngle => f64::from(p.scan_angle),
        PointField::Flags => f64::from(p.flags),
        PointField::UserData => f64::from(p.user_data),
        PointField::PointSourceId => f64::from(p.point_source_id),
        PointField::Red => p.color.map(|c| f64::from(c.red)).unwrap_or(f64::NAN),
        PointField::Green => p.color.map(|c| f64::from(c.green)).unwrap_or(f64::NAN),
        PointField::Blue => p.color.map(|c| f64::from(c.blue)).unwrap_or(f64::NAN),
        PointField::Nir => p.nir.map(f64::from).unwrap_or(f64::NAN),
        PointField::GpsTime => p.gps_time.map(|v| v.0).unwrap_or(f64::NAN),
        PointField::NormalX => p.normal_x.map(f64::from).unwrap_or(f64::NAN),
        PointField::NormalY => p.normal_y.map(f64::from).unwrap_or(f64::NAN),
        PointField::NormalZ => p.normal_z.map(f64::from).unwrap_or(f64::NAN),
    }
}

fn set_field_value_from_f64(field: PointField, p: &mut PointRecord, value: f64) -> Result<()> {
    match field {
        PointField::X => {
            if !value.is_finite() {
                return invalid_field_value(field, value, "must be a finite number");
            }
            p.x = value;
        }
        PointField::Y => {
            if !value.is_finite() {
                return invalid_field_value(field, value, "must be a finite number");
            }
            p.y = value;
        }
        PointField::Z => {
            if !value.is_finite() {
                return invalid_field_value(field, value, "must be a finite number");
            }
            p.z = value;
        }
        PointField::Intensity => p.intensity = parse_u16(field, value)?,
        PointField::Classification => p.classification = parse_u8(field, value)?,
        PointField::ReturnNumber => p.return_number = parse_u8(field, value)?,
        PointField::NumberOfReturns => p.number_of_returns = parse_u8(field, value)?,
        PointField::ScanDirectionFlag => p.scan_direction_flag = parse_bool(field, value)?,
        PointField::EdgeOfFlightLine => p.edge_of_flight_line = parse_bool(field, value)?,
        PointField::ScanAngle => p.scan_angle = parse_i16(field, value)?,
        PointField::Flags => p.flags = parse_u8(field, value)?,
        PointField::UserData => p.user_data = parse_u8(field, value)?,
        PointField::PointSourceId => p.point_source_id = parse_u16(field, value)?,
        PointField::Red => {
            if value.is_nan() {
                if let Some(mut c) = p.color {
                    c.red = 0;
                    if c.green == 0 && c.blue == 0 {
                        p.color = None;
                    } else {
                        p.color = Some(c);
                    }
                }
            } else {
                let mut c = p.color.unwrap_or_default();
                c.red = parse_u16(field, value)?;
                p.color = Some(c);
            }
        }
        PointField::Green => {
            if value.is_nan() {
                if let Some(mut c) = p.color {
                    c.green = 0;
                    if c.red == 0 && c.blue == 0 {
                        p.color = None;
                    } else {
                        p.color = Some(c);
                    }
                }
            } else {
                let mut c = p.color.unwrap_or_default();
                c.green = parse_u16(field, value)?;
                p.color = Some(c);
            }
        }
        PointField::Blue => {
            if value.is_nan() {
                if let Some(mut c) = p.color {
                    c.blue = 0;
                    if c.red == 0 && c.green == 0 {
                        p.color = None;
                    } else {
                        p.color = Some(c);
                    }
                }
            } else {
                let mut c = p.color.unwrap_or_default();
                c.blue = parse_u16(field, value)?;
                p.color = Some(c);
            }
        }
        PointField::Nir => {
            if value.is_nan() {
                p.nir = None;
            } else {
                p.nir = Some(parse_u16(field, value)?);
            }
        }
        PointField::GpsTime => {
            if value.is_nan() {
                p.gps_time = None;
            } else if value.is_finite() {
                p.gps_time = Some(crate::GpsTime(value));
            } else {
                return invalid_field_value(field, value, "must be finite or NaN");
            }
        }
        PointField::NormalX => {
            if value.is_nan() {
                p.normal_x = None;
            } else {
                p.normal_x = Some(parse_f32(field, value)?);
            }
        }
        PointField::NormalY => {
            if value.is_nan() {
                p.normal_y = None;
            } else {
                p.normal_y = Some(parse_f32(field, value)?);
            }
        }
        PointField::NormalZ => {
            if value.is_nan() {
                p.normal_z = None;
            } else {
                p.normal_z = Some(parse_f32(field, value)?);
            }
        }
    }
    Ok(())
}

fn invalid_field_value(field: PointField, value: f64, detail: &str) -> Result<()> {
    Err(Error::InvalidValue {
        field: field.as_name(),
        detail: format!("{detail} (got {value})"),
    })
}

fn parse_u8(field: PointField, value: f64) -> Result<u8> {
    if !value.is_finite() {
        return Err(invalid_field_value_err(field, value, "must be a finite integer"));
    }
    if value.fract() != 0.0 {
        return Err(invalid_field_value_err(field, value, "must be an integer value"));
    }
    if value < f64::from(u8::MIN) || value > f64::from(u8::MAX) {
        return Err(invalid_field_value_err(field, value, "out of range for u8"));
    }
    Ok(value as u8)
}

fn parse_u16(field: PointField, value: f64) -> Result<u16> {
    if !value.is_finite() {
        return Err(invalid_field_value_err(field, value, "must be a finite integer"));
    }
    if value.fract() != 0.0 {
        return Err(invalid_field_value_err(field, value, "must be an integer value"));
    }
    if value < f64::from(u16::MIN) || value > f64::from(u16::MAX) {
        return Err(invalid_field_value_err(field, value, "out of range for u16"));
    }
    Ok(value as u16)
}

fn parse_i16(field: PointField, value: f64) -> Result<i16> {
    if !value.is_finite() {
        return Err(invalid_field_value_err(field, value, "must be a finite integer"));
    }
    if value.fract() != 0.0 {
        return Err(invalid_field_value_err(field, value, "must be an integer value"));
    }
    if value < f64::from(i16::MIN) || value > f64::from(i16::MAX) {
        return Err(invalid_field_value_err(field, value, "out of range for i16"));
    }
    Ok(value as i16)
}

fn parse_f32(field: PointField, value: f64) -> Result<f32> {
    if !value.is_finite() {
        return Err(invalid_field_value_err(field, value, "must be finite"));
    }
    if value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
        return Err(invalid_field_value_err(field, value, "out of range for f32"));
    }
    Ok(value as f32)
}

fn parse_bool(field: PointField, value: f64) -> Result<bool> {
    if !value.is_finite() {
        return Err(invalid_field_value_err(field, value, "must be 0 or 1"));
    }
    if value == 0.0 {
        Ok(false)
    } else if value == 1.0 {
        Ok(true)
    } else {
        Err(invalid_field_value_err(field, value, "must be exactly 0 or 1"))
    }
}

fn invalid_field_value_err(field: PointField, value: f64, detail: &str) -> Error {
    Error::InvalidValue {
        field: field.as_name(),
        detail: format!("{detail} (got {value})"),
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
    use super::{
        PointCloud,
        PointColumnChunkReader,
        PointColumnChunkRewriter,
        PointField,
        read,
        read_columns_chunked,
        rewrite_columns_chunked,
    };
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

    #[test]
    fn extract_and_apply_columns_round_trip() {
        let mut cloud = PointCloud {
            points: vec![
                PointRecord {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    classification: 2,
                    return_number: 1,
                    number_of_returns: 1,
                    ..PointRecord::default()
                },
                PointRecord {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                    classification: 1,
                    return_number: 1,
                    number_of_returns: 2,
                    ..PointRecord::default()
                },
            ],
            crs: None,
        };

        let cols = cloud
            .extract_columns(&[PointField::X, PointField::Classification])
            .unwrap();
        assert_eq!(cols.len(), 2);
        assert_eq!(cols[0], vec![1.0, 4.0]);
        assert_eq!(cols[1], vec![2.0, 1.0]);

        let updates = vec![vec![10.0, 20.0], vec![6.0, 7.0]];
        cloud
            .apply_columns(&[PointField::X, PointField::Classification], &updates)
            .unwrap();
        assert_eq!(cloud.points[0].x, 10.0);
        assert_eq!(cloud.points[1].x, 20.0);
        assert_eq!(cloud.points[0].classification, 6);
        assert_eq!(cloud.points[1].classification, 7);
    }

    #[test]
    fn apply_columns_rejects_invalid_bool_values() {
        let mut cloud = PointCloud {
            points: vec![PointRecord::default()],
            crs: None,
        };

        let err = cloud
            .apply_columns(&[PointField::ScanDirectionFlag], &[vec![2.0]])
            .unwrap_err();
        assert!(matches!(err, Error::InvalidValue { .. }));
        assert!(err.to_string().contains("must be exactly 0 or 1"));
    }

    #[test]
    fn apply_columns_range_updates_subset() {
        let mut cloud = PointCloud {
            points: vec![
                PointRecord { x: 1.0, ..PointRecord::default() },
                PointRecord { x: 2.0, ..PointRecord::default() },
                PointRecord { x: 3.0, ..PointRecord::default() },
            ],
            crs: None,
        };

        cloud
            .apply_columns_range(1, &[PointField::X], &[vec![20.0, 30.0]])
            .unwrap();

        assert_eq!(cloud.points[0].x, 1.0);
        assert_eq!(cloud.points[1].x, 20.0);
        assert_eq!(cloud.points[2].x, 30.0);
    }

    #[test]
    fn apply_columns_range_rejects_overflow_range() {
        let mut cloud = PointCloud {
            points: vec![PointRecord::default(); 2],
            crs: None,
        };

        let err = cloud
            .apply_columns_range(1, &[PointField::X], &[vec![1.0, 2.0]])
            .err()
            .unwrap();

        assert!(matches!(err, Error::InvalidValue { .. }));
        assert!(err.to_string().contains("exceeds point count"));
    }

    #[test]
    fn read_columns_chunked_streams_expected_batches() {
        let cloud = PointCloud {
            points: vec![
                PointRecord {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    classification: 2,
                    ..PointRecord::default()
                },
                PointRecord {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                    classification: 3,
                    ..PointRecord::default()
                },
                PointRecord {
                    x: 7.0,
                    y: 8.0,
                    z: 9.0,
                    classification: 4,
                    ..PointRecord::default()
                },
            ],
            crs: None,
        };

        let mut path = std::env::temp_dir();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!("wblidar_chunk_stream_{unique}.las"));

        cloud.write(&path).unwrap();

        let chunks = read_columns_chunked(
            &path,
            &[PointField::X, PointField::Classification],
            2,
        )
        .unwrap();

        std::fs::remove_file(&path).ok();

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0][0], vec![1.0, 4.0]);
        assert_eq!(chunks[0][1], vec![2.0, 3.0]);
        assert_eq!(chunks[1][0], vec![7.0]);
        assert_eq!(chunks[1][1], vec![4.0]);
    }

    #[test]
    fn point_column_chunk_reader_rejects_zero_chunk_size() {
        let mut path = std::env::temp_dir();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!("wblidar_chunk_stream_{unique}.las"));

        PointCloud {
            points: vec![PointRecord::default()],
            crs: None,
        }
        .write(&path)
        .unwrap();

        let err = PointColumnChunkReader::open(&path, &[PointField::X], 0)
            .err()
            .unwrap();
        std::fs::remove_file(&path).ok();
        assert!(matches!(err, Error::InvalidValue { .. }));
        assert!(err.to_string().contains("chunk_size must be greater than zero"));
    }

    #[test]
    fn rewrite_columns_chunked_updates_output_streaming() {
        let cloud = PointCloud {
            points: vec![
                PointRecord {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    classification: 2,
                    ..PointRecord::default()
                },
                PointRecord {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                    classification: 3,
                    ..PointRecord::default()
                },
            ],
            crs: None,
        };

        let mut input = std::env::temp_dir();
        let mut output = std::env::temp_dir();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        input.push(format!("wblidar_rewrite_in_{unique}.las"));
        output.push(format!("wblidar_rewrite_out_{unique}.laz"));

        cloud.write(&input).unwrap();

        let chunks = vec![
            vec![vec![1.0], vec![7.0]],
            vec![vec![4.0], vec![7.0]],
        ];

        rewrite_columns_chunked(
            &input,
            &output,
            &[PointField::X, PointField::Classification],
            &chunks,
        )
        .unwrap();

        let edited = read(&output).unwrap();
        std::fs::remove_file(&input).ok();
        std::fs::remove_file(&output).ok();

        assert_eq!(edited.points.len(), 2);
        assert_eq!(edited.points[0].classification, 7);
        assert_eq!(edited.points[1].classification, 7);
        assert!((edited.points[0].x - 1.0).abs() < 1e-9);
        assert!((edited.points[1].x - 4.0).abs() < 1e-9);
    }

    #[test]
    fn point_column_chunk_rewriter_rejects_incomplete_coverage() {
        let cloud = PointCloud {
            points: vec![PointRecord::default(), PointRecord::default()],
            crs: None,
        };

        let mut input = std::env::temp_dir();
        let mut output = std::env::temp_dir();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        input.push(format!("wblidar_rewriter_in_{unique}.las"));
        output.push(format!("wblidar_rewriter_out_{unique}.las"));
        cloud.write(&input).unwrap();

        let mut rewriter = PointColumnChunkRewriter::open(&input, &output, &[PointField::X]).unwrap();
        rewriter.apply_chunk(&[vec![1.0]]).unwrap();
        let err = rewriter.finish().err().unwrap();

        std::fs::remove_file(&input).ok();
        std::fs::remove_file(&output).ok();
        assert!(matches!(err, Error::InvalidValue { .. }));
        assert!(err
            .to_string()
            .contains("chunk rows did not cover full source point count"));
    }
}
