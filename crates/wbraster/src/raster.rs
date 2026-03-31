//! Core `Raster` type — the central data structure for all raster GIS data.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use rayon::prelude::*;
use wbprojection::Crs;
use wide::{f64x4, CmpNe};

use crate::error::{Result, RasterError};
use crate::formats::RasterFormat;
use crate::crs_info::CrsInfo;

// ─── Data type enum ───────────────────────────────────────────────────────────

/// The underlying numeric data type of raster cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DataType {
    /// Unsigned 8-bit integer (byte).
    U8,
    /// Signed 8-bit integer.
    I8,
    /// Unsigned 16-bit integer.
    U16,
    /// Signed 16-bit integer.
    I16,
    /// Unsigned 32-bit integer.
    U32,
    /// Signed 32-bit integer.
    I32,
    /// Unsigned 64-bit integer.
    U64,
    /// Signed 64-bit integer.
    I64,
    /// 32-bit IEEE 754 floating point.
    #[default]
    F32,
    /// 64-bit IEEE 754 floating point.
    F64,
}

impl DataType {
    /// Number of bytes per cell value.
    pub fn size_bytes(self) -> usize {
        match self {
            DataType::U8 | DataType::I8 => 1,
            DataType::U16 | DataType::I16 => 2,
            DataType::U32 | DataType::I32 | DataType::F32 => 4,
            DataType::U64 | DataType::I64 | DataType::F64 => 8,
        }
    }

    /// Human-readable name used in header files.
    pub fn as_str(self) -> &'static str {
        match self {
            DataType::U8 => "uint8",
            DataType::I8 => "int8",
            DataType::U16 => "uint16",
            DataType::I16 => "int16",
            DataType::U32 => "uint32",
            DataType::I32 => "int32",
            DataType::U64 => "uint64",
            DataType::I64 => "int64",
            DataType::F32 => "float32",
            DataType::F64 => "float64",
        }
    }

    /// Parse from a format string (case-insensitive).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "uint8" | "u8" | "byte" => Some(DataType::U8),
            "int8" | "i8" => Some(DataType::I8),
            "uint16" | "u16" => Some(DataType::U16),
            "int16" | "i16" | "integer" | "short" => Some(DataType::I16),
            "uint32" | "u32" => Some(DataType::U32),
            "int32" | "i32" | "long" => Some(DataType::I32),
            "uint64" | "u64" => Some(DataType::U64),
            "int64" | "i64" | "longlong" => Some(DataType::I64),
            "float32" | "f32" | "float" | "real" => Some(DataType::F32),
            "float64" | "f64" | "double" => Some(DataType::F64),
            _ => None,
        }
    }
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Typed in-memory pixel buffer.
#[derive(Debug, Clone)]
pub enum RasterData {
    /// Unsigned 8-bit storage.
    U8(Vec<u8>),
    /// Signed 8-bit storage.
    I8(Vec<i8>),
    /// Unsigned 16-bit storage.
    U16(Vec<u16>),
    /// Signed 16-bit storage.
    I16(Vec<i16>),
    /// Unsigned 32-bit storage.
    U32(Vec<u32>),
    /// Signed 32-bit storage.
    I32(Vec<i32>),
    /// Unsigned 64-bit storage.
    U64(Vec<u64>),
    /// Signed 64-bit storage.
    I64(Vec<i64>),
    /// 32-bit float storage.
    F32(Vec<f32>),
    /// 64-bit float storage.
    F64(Vec<f64>),
}

/// A mutable typed view of one raster row.
pub enum RasterRowMut<'a> {
    /// Unsigned 8-bit row.
    U8(&'a mut [u8]),
    /// Signed 8-bit row.
    I8(&'a mut [i8]),
    /// Unsigned 16-bit row.
    U16(&'a mut [u16]),
    /// Signed 16-bit row.
    I16(&'a mut [i16]),
    /// Unsigned 32-bit row.
    U32(&'a mut [u32]),
    /// Signed 32-bit row.
    I32(&'a mut [i32]),
    /// Unsigned 64-bit row.
    U64(&'a mut [u64]),
    /// Signed 64-bit row.
    I64(&'a mut [i64]),
    /// 32-bit float row.
    F32(&'a mut [f32]),
    /// 64-bit float row.
    F64(&'a mut [f64]),
}

/// An immutable typed view of one raster row.
pub enum RasterRowRef<'a> {
    /// Unsigned 8-bit row.
    U8(&'a [u8]),
    /// Signed 8-bit row.
    I8(&'a [i8]),
    /// Unsigned 16-bit row.
    U16(&'a [u16]),
    /// Signed 16-bit row.
    I16(&'a [i16]),
    /// Unsigned 32-bit row.
    U32(&'a [u32]),
    /// Signed 32-bit row.
    I32(&'a [i32]),
    /// Unsigned 64-bit row.
    U64(&'a [u64]),
    /// Signed 64-bit row.
    I64(&'a [i64]),
    /// 32-bit float row.
    F32(&'a [f32]),
    /// 64-bit float row.
    F64(&'a [f64]),
}

impl RasterData {
    /// Create a typed data buffer of length `len`, filled with `value` converted
    /// to the specified data type.
    pub fn new_filled(data_type: DataType, len: usize, value: f64) -> Self {
        match data_type {
            DataType::U8 => Self::U8(vec![value as u8; len]),
            DataType::I8 => Self::I8(vec![value as i8; len]),
            DataType::U16 => Self::U16(vec![value as u16; len]),
            DataType::I16 => Self::I16(vec![value as i16; len]),
            DataType::U32 => Self::U32(vec![value as u32; len]),
            DataType::I32 => Self::I32(vec![value as i32; len]),
            DataType::U64 => Self::U64(vec![value as u64; len]),
            DataType::I64 => Self::I64(vec![value as i64; len]),
            DataType::F32 => Self::F32(vec![value as f32; len]),
            DataType::F64 => Self::F64(vec![value; len]),
        }
    }

    /// Convert an `f64` vector into typed storage.
    pub fn from_f64_vec(data_type: DataType, data: Vec<f64>) -> Self {
        match data_type {
            DataType::U8 => Self::U8(data.into_iter().map(|v| v as u8).collect()),
            DataType::I8 => Self::I8(data.into_iter().map(|v| v as i8).collect()),
            DataType::U16 => Self::U16(data.into_iter().map(|v| v as u16).collect()),
            DataType::I16 => Self::I16(data.into_iter().map(|v| v as i16).collect()),
            DataType::U32 => Self::U32(data.into_iter().map(|v| v as u32).collect()),
            DataType::I32 => Self::I32(data.into_iter().map(|v| v as i32).collect()),
            DataType::U64 => Self::U64(data.into_iter().map(|v| v as u64).collect()),
            DataType::I64 => Self::I64(data.into_iter().map(|v| v as i64).collect()),
            DataType::F32 => Self::F32(data.into_iter().map(|v| v as f32).collect()),
            DataType::F64 => Self::F64(data),
        }
    }

    /// Number of stored cells.
    pub fn len(&self) -> usize {
        match self {
            Self::U8(v) => v.len(),
            Self::I8(v) => v.len(),
            Self::U16(v) => v.len(),
            Self::I16(v) => v.len(),
            Self::U32(v) => v.len(),
            Self::I32(v) => v.len(),
            Self::U64(v) => v.len(),
            Self::I64(v) => v.len(),
            Self::F32(v) => v.len(),
            Self::F64(v) => v.len(),
        }
    }

    /// Returns `true` if no cells are stored.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Native stored data type.
    pub fn data_type(&self) -> DataType {
        match self {
            Self::U8(_) => DataType::U8,
            Self::I8(_) => DataType::I8,
            Self::U16(_) => DataType::U16,
            Self::I16(_) => DataType::I16,
            Self::U32(_) => DataType::U32,
            Self::I32(_) => DataType::I32,
            Self::U64(_) => DataType::U64,
            Self::I64(_) => DataType::I64,
            Self::F32(_) => DataType::F32,
            Self::F64(_) => DataType::F64,
        }
    }

    /// Read one cell as `f64`.
    pub fn get_f64(&self, idx: usize) -> f64 {
        match self {
            Self::U8(v) => v[idx] as f64,
            Self::I8(v) => v[idx] as f64,
            Self::U16(v) => v[idx] as f64,
            Self::I16(v) => v[idx] as f64,
            Self::U32(v) => v[idx] as f64,
            Self::I32(v) => v[idx] as f64,
            Self::U64(v) => v[idx] as f64,
            Self::I64(v) => v[idx] as f64,
            Self::F32(v) => v[idx] as f64,
            Self::F64(v) => v[idx],
        }
    }

    /// Set one cell from an `f64` value using native-type conversion.
    pub fn set_f64(&mut self, idx: usize, value: f64) {
        match self {
            Self::U8(v) => v[idx] = value as u8,
            Self::I8(v) => v[idx] = value as i8,
            Self::U16(v) => v[idx] = value as u16,
            Self::I16(v) => v[idx] = value as i16,
            Self::U32(v) => v[idx] = value as u32,
            Self::I32(v) => v[idx] = value as i32,
            Self::U64(v) => v[idx] = value as u64,
            Self::I64(v) => v[idx] = value as i64,
            Self::F32(v) => v[idx] = value as f32,
            Self::F64(v) => v[idx] = value,
        }
    }

    /// Iterate over all cells as `f64`.
    pub fn iter_f64(&self) -> Box<dyn Iterator<Item = f64> + '_> {
        match self {
            Self::U8(v) => Box::new(v.iter().copied().map(|x| x as f64)),
            Self::I8(v) => Box::new(v.iter().copied().map(|x| x as f64)),
            Self::U16(v) => Box::new(v.iter().copied().map(|x| x as f64)),
            Self::I16(v) => Box::new(v.iter().copied().map(|x| x as f64)),
            Self::U32(v) => Box::new(v.iter().copied().map(|x| x as f64)),
            Self::I32(v) => Box::new(v.iter().copied().map(|x| x as f64)),
            Self::U64(v) => Box::new(v.iter().copied().map(|x| x as f64)),
            Self::I64(v) => Box::new(v.iter().copied().map(|x| x as f64)),
            Self::F32(v) => Box::new(v.iter().copied().map(|x| x as f64)),
            Self::F64(v) => Box::new(v.iter().copied()),
        }
    }

    /// Materialize all cells as `Vec<f64>`.
    pub fn to_f64_vec(&self) -> Vec<f64> {
        self.iter_f64().collect()
    }

    /// Returns data as `u8` slice when storage is `U8`.
    pub fn as_u8_slice(&self) -> Option<&[u8]> {
        match self {
            Self::U8(v) => Some(v.as_slice()),
            _ => None,
        }
    }

    /// Returns data as mutable `u8` slice when storage is `U8`.
    pub fn as_u8_slice_mut(&mut self) -> Option<&mut [u8]> {
        match self {
            Self::U8(v) => Some(v.as_mut_slice()),
            _ => None,
        }
    }

    /// Returns data as `i8` slice when storage is `I8`.
    pub fn as_i8_slice(&self) -> Option<&[i8]> {
        match self {
            Self::I8(v) => Some(v.as_slice()),
            _ => None,
        }
    }

    /// Returns data as mutable `i8` slice when storage is `I8`.
    pub fn as_i8_slice_mut(&mut self) -> Option<&mut [i8]> {
        match self {
            Self::I8(v) => Some(v.as_mut_slice()),
            _ => None,
        }
    }

    /// Returns data as `u16` slice when storage is `U16`.
    pub fn as_u16_slice(&self) -> Option<&[u16]> {
        match self {
            Self::U16(v) => Some(v.as_slice()),
            _ => None,
        }
    }

    /// Returns data as mutable `u16` slice when storage is `U16`.
    pub fn as_u16_slice_mut(&mut self) -> Option<&mut [u16]> {
        match self {
            Self::U16(v) => Some(v.as_mut_slice()),
            _ => None,
        }
    }

    /// Returns data as `i16` slice when storage is `I16`.
    pub fn as_i16_slice(&self) -> Option<&[i16]> {
        match self {
            Self::I16(v) => Some(v.as_slice()),
            _ => None,
        }
    }

    /// Returns data as mutable `i16` slice when storage is `I16`.
    pub fn as_i16_slice_mut(&mut self) -> Option<&mut [i16]> {
        match self {
            Self::I16(v) => Some(v.as_mut_slice()),
            _ => None,
        }
    }

    /// Returns data as `u32` slice when storage is `U32`.
    pub fn as_u32_slice(&self) -> Option<&[u32]> {
        match self {
            Self::U32(v) => Some(v.as_slice()),
            _ => None,
        }
    }

    /// Returns data as mutable `u32` slice when storage is `U32`.
    pub fn as_u32_slice_mut(&mut self) -> Option<&mut [u32]> {
        match self {
            Self::U32(v) => Some(v.as_mut_slice()),
            _ => None,
        }
    }

    /// Returns data as `i32` slice when storage is `I32`.
    pub fn as_i32_slice(&self) -> Option<&[i32]> {
        match self {
            Self::I32(v) => Some(v.as_slice()),
            _ => None,
        }
    }

    /// Returns data as mutable `i32` slice when storage is `I32`.
    pub fn as_i32_slice_mut(&mut self) -> Option<&mut [i32]> {
        match self {
            Self::I32(v) => Some(v.as_mut_slice()),
            _ => None,
        }
    }

    /// Returns data as `u64` slice when storage is `U64`.
    pub fn as_u64_slice(&self) -> Option<&[u64]> {
        match self {
            Self::U64(v) => Some(v.as_slice()),
            _ => None,
        }
    }

    /// Returns data as mutable `u64` slice when storage is `U64`.
    pub fn as_u64_slice_mut(&mut self) -> Option<&mut [u64]> {
        match self {
            Self::U64(v) => Some(v.as_mut_slice()),
            _ => None,
        }
    }

    /// Returns data as `i64` slice when storage is `I64`.
    pub fn as_i64_slice(&self) -> Option<&[i64]> {
        match self {
            Self::I64(v) => Some(v.as_slice()),
            _ => None,
        }
    }

    /// Returns data as mutable `i64` slice when storage is `I64`.
    pub fn as_i64_slice_mut(&mut self) -> Option<&mut [i64]> {
        match self {
            Self::I64(v) => Some(v.as_mut_slice()),
            _ => None,
        }
    }

    /// Returns data as `f32` slice when storage is `F32`.
    pub fn as_f32_slice(&self) -> Option<&[f32]> {
        match self {
            Self::F32(v) => Some(v.as_slice()),
            _ => None,
        }
    }

    /// Returns data as mutable `f32` slice when storage is `F32`.
    pub fn as_f32_slice_mut(&mut self) -> Option<&mut [f32]> {
        match self {
            Self::F32(v) => Some(v.as_mut_slice()),
            _ => None,
        }
    }

    /// Returns data as `f64` slice when storage is `F64`.
    pub fn as_f64_slice(&self) -> Option<&[f64]> {
        match self {
            Self::F64(v) => Some(v.as_slice()),
            _ => None,
        }
    }

    /// Returns data as mutable `f64` slice when storage is `F64`.
    pub fn as_f64_slice_mut(&mut self) -> Option<&mut [f64]> {
        match self {
            Self::F64(v) => Some(v.as_mut_slice()),
            _ => None,
        }
    }
}

// ─── NoData sentinel ─────────────────────────────────────────────────────────

/// Represents the "no data" sentinel value for a raster layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoData(pub f64);

impl NoData {
    /// Common GIS nodata default.
    pub const COMMON: NoData = NoData(-9999.0);
    /// IEEE NaN-based nodata (compare with `is_nan()`).
    pub const NAN: NoData = NoData(f64::NAN);

    /// Returns `true` if `v` represents this nodata value.
    #[inline]
    pub fn matches(self, v: f64) -> bool {
        if self.0.is_nan() {
            v.is_nan()
        } else {
            (v - self.0).abs() < f64::EPSILON * self.0.abs().max(1.0)
        }
    }
}

impl Default for NoData {
    fn default() -> Self {
        Self::COMMON
    }
}

// ─── Spatial extent ───────────────────────────────────────────────────────────

/// Axis-aligned bounding box for a raster.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extent {
    /// West edge (minimum X / longitude).
    pub x_min: f64,
    /// South edge (minimum Y / latitude).
    pub y_min: f64,
    /// East edge (maximum X / longitude).
    pub x_max: f64,
    /// North edge (maximum Y / latitude).
    pub y_max: f64,
}

impl Extent {
    /// Compute the extent from origin + grid dimensions.
    pub fn from_origin(x_min: f64, y_min: f64, cols: usize, rows: usize, cell_size: f64) -> Self {
        Self {
            x_min,
            y_min,
            x_max: x_min + cols as f64 * cell_size,
            y_max: y_min + rows as f64 * cell_size,
        }
    }

    /// Width in spatial units.
    pub fn width(&self) -> f64 { self.x_max - self.x_min }

    /// Height in spatial units.
    pub fn height(&self) -> f64 { self.y_max - self.y_min }
}

// ─── Statistics ───────────────────────────────────────────────────────────────

/// Basic raster statistics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Statistics {
    /// Minimum data value (excluding nodata).
    pub min: f64,
    /// Maximum data value (excluding nodata).
    pub max: f64,
    /// Mean of data values (excluding nodata).
    pub mean: f64,
    /// Standard deviation of data values (excluding nodata).
    pub std_dev: f64,
    /// Number of valid (non-nodata) cells.
    pub valid_count: usize,
    /// Number of nodata cells.
    pub nodata_count: usize,
}

/// Selects the computation path used for raster statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatisticsComputationMode {
    /// Use the crate's default optimized path.
    Auto,
    /// Force the scalar accumulation path.
    Scalar,
    /// Force the SIMD accumulation path.
    Simd,
}

#[derive(Debug, Clone, Copy)]
struct StatsAccumulator {
    min: f64,
    max: f64,
    sum: f64,
    sum_sq: f64,
    valid_count: usize,
    nodata_count: usize,
}

impl Default for StatsAccumulator {
    fn default() -> Self {
        Self {
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
            sum: 0.0,
            sum_sq: 0.0,
            valid_count: 0,
            nodata_count: 0,
        }
    }
}

impl StatsAccumulator {
    fn merge(&mut self, other: Self) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
        self.sum += other.sum;
        self.sum_sq += other.sum_sq;
        self.valid_count += other.valid_count;
        self.nodata_count += other.nodata_count;
    }

    fn to_statistics(self) -> Statistics {
        let (mean, std_dev) = if self.valid_count == 0 {
            (0.0, 0.0)
        } else {
            let n = self.valid_count as f64;
            let mean = self.sum / n;
            let variance = (self.sum_sq / n - mean * mean).max(0.0);
            (mean, variance.sqrt())
        };

        Statistics {
            min: if self.valid_count == 0 { 0.0 } else { self.min },
            max: if self.valid_count == 0 { 0.0 } else { self.max },
            mean,
            std_dev,
            valid_count: self.valid_count,
            nodata_count: self.nodata_count,
        }
    }
}

/// Resampling method used during raster reprojection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResampleMethod {
    /// Nearest-neighbor sampling (fast, category-safe).
    Nearest,
    /// Bilinear interpolation (continuous surfaces).
    Bilinear,
    /// Bicubic interpolation (Catmull-Rom cubic convolution).
    Cubic,
    /// Lanczos interpolation (windowed sinc, radius 3).
    Lanczos,
    /// 3x3 mean filter around nearest source pixel center.
    Average,
    /// 3x3 minimum filter around nearest source pixel center.
    Min,
    /// 3x3 maximum filter around nearest source pixel center.
    Max,
    /// 3x3 mode filter around nearest source pixel center.
    Mode,
    /// 3x3 median filter around nearest source pixel center.
    Median,
    /// 3x3 standard-deviation filter around nearest source pixel center.
    StdDev,
}

/// Nodata handling policy for interpolation-based reprojection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodataPolicy {
    /// Require full valid kernel support; otherwise output nodata.
    Strict,
    /// Use available valid kernel samples and renormalize interpolation weights.
    PartialKernel,
    /// Try strict interpolation first, then fall back to nearest-neighbor sampling.
    Fill,
}

/// Policy controlling longitude bound handling when antimeridian crossing is possible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntimeridianPolicy {
    /// Use linear bounds unless wrapped bounds are strictly narrower.
    Auto,
    /// Always use linear min/max longitude bounds.
    Linear,
    /// Always use wrapped minimal-arc longitude bounds.
    Wrap,
}

/// Policy for converting resolution + extent to integer output dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridSizePolicy {
    /// Expand outward to fully cover requested extent (uses ceil sizing).
    Expand,
    /// Keep grid within requested extent (uses floor sizing, min 1 cell).
    FitInside,
}

/// Destination-footprint handling mode during reprojection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestinationFootprint {
    /// Do not apply a transformed-source footprint mask.
    None,
    /// Mask destination cells outside transformed source boundary ring.
    SourceBoundary,
}

/// Options for raster reprojection.
#[derive(Debug, Clone, Copy)]
pub struct ReprojectOptions {
    /// Destination CRS EPSG code.
    pub dst_epsg: u32,
    /// Resampling method.
    pub resample: ResampleMethod,
    /// Optional output column count (defaults to source `cols`).
    pub cols: Option<usize>,
    /// Optional output row count (defaults to source `rows`).
    pub rows: Option<usize>,
    /// Optional output extent in destination CRS.
    ///
    /// Defaults to transformed bounds of source extent corners.
    pub extent: Option<Extent>,
    /// Optional output X resolution in destination CRS units/pixel.
    ///
    /// Used to derive output `cols` when `cols` is not provided.
    pub x_res: Option<f64>,
    /// Optional output Y resolution in destination CRS units/pixel.
    ///
    /// Used to derive output `rows` when `rows` is not provided.
    pub y_res: Option<f64>,
    /// Optional snap origin X for resolution-derived output grid alignment.
    ///
    /// Used only when `x_res` is provided and `cols` is not explicitly set.
    pub snap_x: Option<f64>,
    /// Optional snap origin Y for resolution-derived output grid alignment.
    ///
    /// Used only when `y_res` is provided and `rows` is not explicitly set.
    pub snap_y: Option<f64>,
    /// Nodata policy used by interpolation methods.
    pub nodata_policy: NodataPolicy,
    /// Policy controlling antimeridian handling for geographic output bounds.
    pub antimeridian_policy: AntimeridianPolicy,
    /// Policy used when deriving integer output size from resolution controls.
    pub grid_size_policy: GridSizePolicy,
    /// Destination-footprint handling policy during reprojection.
    pub destination_footprint: DestinationFootprint,
}

impl ReprojectOptions {
    /// Create options with required destination EPSG and resampling method.
    pub fn new(dst_epsg: u32, resample: ResampleMethod) -> Self {
        Self {
            dst_epsg,
            resample,
            cols: None,
            rows: None,
            extent: None,
            x_res: None,
            y_res: None,
            snap_x: None,
            snap_y: None,
            nodata_policy: NodataPolicy::PartialKernel,
            antimeridian_policy: AntimeridianPolicy::Auto,
            grid_size_policy: GridSizePolicy::Expand,
            destination_footprint: DestinationFootprint::None,
        }
    }

    /// Set nodata handling policy for interpolation and return updated options.
    pub fn with_nodata_policy(mut self, nodata_policy: NodataPolicy) -> Self {
        self.nodata_policy = nodata_policy;
        self
    }

    /// Set output raster size (columns, rows) and return updated options.
    pub fn with_size(mut self, cols: usize, rows: usize) -> Self {
        self.cols = Some(cols);
        self.rows = Some(rows);
        self
    }

    /// Set output raster extent and return updated options.
    pub fn with_extent(mut self, extent: Extent) -> Self {
        self.extent = Some(extent);
        self
    }

    /// Set output resolution (`x_res`, `y_res`) and return updated options.
    ///
    /// Positive finite values are required and interpreted as destination CRS
    /// units per pixel.
    pub fn with_resolution(mut self, x_res: f64, y_res: f64) -> Self {
        self.x_res = Some(x_res);
        self.y_res = Some(y_res);
        self
    }

    /// Set isotropic output resolution and return updated options.
    pub fn with_square_resolution(mut self, res: f64) -> Self {
        self.x_res = Some(res);
        self.y_res = Some(res);
        self
    }

    /// Set snap origin for aligning resolution-derived output grids.
    ///
    /// Snap alignment is applied per axis only when that axis uses
    /// resolution-derived sizing (i.e., no explicit `cols`/`rows`).
    pub fn with_snap_origin(mut self, snap_x: f64, snap_y: f64) -> Self {
        self.snap_x = Some(snap_x);
        self.snap_y = Some(snap_y);
        self
    }

    /// Set antimeridian handling policy for geographic output bounds.
    pub fn with_antimeridian_policy(mut self, policy: AntimeridianPolicy) -> Self {
        self.antimeridian_policy = policy;
        self
    }

    /// Set sizing policy used for resolution-derived output dimensions.
    pub fn with_grid_size_policy(mut self, policy: GridSizePolicy) -> Self {
        self.grid_size_policy = policy;
        self
    }

    /// Set destination-footprint handling policy for reprojection output.
    pub fn with_destination_footprint(mut self, footprint: DestinationFootprint) -> Self {
        self.destination_footprint = footprint;
        self
    }
}

// ─── Configuration / builder ──────────────────────────────────────────────────

/// Parameters used to construct a new `Raster`.
#[derive(Debug, Clone)]
pub struct RasterConfig {
    /// Number of columns (samples).
    pub cols: usize,
    /// Number of rows (lines).
    pub rows: usize,
    /// Number of bands.
    pub bands: usize,
    /// X coordinate of the west edge (left).
    pub x_min: f64,
    /// Y coordinate of the south edge (bottom).
    pub y_min: f64,
    /// Cell size (assumed square unless `cell_size_y` is set).
    pub cell_size: f64,
    /// Optional distinct Y cell size (negative = top-down raster).
    /// If `None`, `cell_size` is used (positive, bottom-up convention).
    pub cell_size_y: Option<f64>,
    /// No-data sentinel value.
    pub nodata: f64,
    /// Underlying storage type.
    pub data_type: DataType,
    /// Spatial reference system.
    pub crs: CrsInfo,
    /// Free-form metadata key/value pairs.
    pub metadata: Vec<(String, String)>,
}

impl Default for RasterConfig {
    fn default() -> Self {
        Self {
            cols: 0,
            rows: 0,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            cell_size_y: None,
            nodata: -9999.0,
            data_type: DataType::F32,
            crs: CrsInfo::default(),
            metadata: Vec::new(),
        }
    }
}

// ─── Main Raster struct ───────────────────────────────────────────────────────

/// A raster grid with one or more bands.
///
/// Data is stored in a typed contiguous buffer matching `data_type`.
/// Conversion to `f64` happens only through accessor helpers when needed.
///
/// Layout: band-major, then row-major top-down within each band
/// (`index = band * rows * cols + row * cols + col`).
#[derive(Debug, Clone)]
pub struct Raster {
    /// Number of columns.
    pub cols: usize,
    /// Number of rows.
    pub rows: usize,
    /// Number of bands.
    pub bands: usize,
    /// West edge X coordinate.
    pub x_min: f64,
    /// South edge Y coordinate.
    pub y_min: f64,
    /// Cell width in map units (always positive).
    pub cell_size_x: f64,
    /// Cell height in map units (always positive, stored as absolute value).
    pub cell_size_y: f64,
    /// No-data value.
    pub nodata: f64,
    /// On-disk data type (used for writing).
    pub data_type: DataType,
    /// Spatial reference.
    pub crs: CrsInfo,
    /// Free-form key/value metadata.
    pub metadata: Vec<(String, String)>,
    /// Raw data buffer, band-major then row-major top-down. Length = `bands * cols * rows`.
    pub data: RasterData,
}

impl Raster {
    // ─── Construction ──────────────────────────────────────────────────────

    /// Create a new raster from a `RasterConfig`, filling all cells with `nodata`.
    pub fn new(cfg: RasterConfig) -> Self {
        let bands = cfg.bands.max(1);
        let n = cfg.cols * cfg.rows * bands;
        let cell_size_y = cfg.cell_size_y.map(|v| v.abs()).unwrap_or(cfg.cell_size);
        Self {
            cols: cfg.cols,
            rows: cfg.rows,
            bands,
            x_min: cfg.x_min,
            y_min: cfg.y_min,
            cell_size_x: cfg.cell_size,
            cell_size_y,
            nodata: cfg.nodata,
            data_type: cfg.data_type,
            crs: cfg.crs,
            metadata: cfg.metadata,
            data: RasterData::new_filled(cfg.data_type, n, cfg.nodata),
        }
    }

    /// Create a raster from a raw `f64` data buffer.
    ///
    /// # Errors
    /// Returns [`RasterError::InvalidDimensions`] if `data.len() != cols * rows * bands`.
    pub fn from_data(cfg: RasterConfig, data: Vec<f64>) -> Result<Self> {
        let bands = cfg.bands.max(1);
        if data.len() != cfg.cols * cfg.rows * bands {
            return Err(RasterError::InvalidDimensions { cols: cfg.cols, rows: cfg.rows });
        }
        let dt = cfg.data_type;
        let mut r = Self::new(cfg);
        r.data = RasterData::from_f64_vec(dt, data);
        Ok(r)
    }

    /// Create a raster from a typed data buffer.
    ///
    /// # Errors
    /// Returns [`RasterError::InvalidDimensions`] if `data.len() != cols * rows * bands`.
    /// Returns [`RasterError::Other`] if `cfg.data_type != data.data_type()`.
    pub fn from_data_native(cfg: RasterConfig, data: RasterData) -> Result<Self> {
        let bands = cfg.bands.max(1);
        if data.len() != cfg.cols * cfg.rows * bands {
            return Err(RasterError::InvalidDimensions { cols: cfg.cols, rows: cfg.rows });
        }
        if cfg.data_type != data.data_type() {
            return Err(RasterError::Other(format!(
                "data_type mismatch: config={}, data={}",
                cfg.data_type,
                data.data_type()
            )));
        }
        let mut r = Self::new(cfg);
        r.data = data;
        Ok(r)
    }

    /// Typed fast-path access to `u8` storage.
    pub fn data_u8(&self) -> Option<&[u8]> { self.data.as_u8_slice() }
    /// Typed fast-path mutable access to `u8` storage.
    pub fn data_u8_mut(&mut self) -> Option<&mut [u8]> { self.data.as_u8_slice_mut() }

    /// Typed fast-path access to `i8` storage.
    pub fn data_i8(&self) -> Option<&[i8]> { self.data.as_i8_slice() }
    /// Typed fast-path mutable access to `i8` storage.
    pub fn data_i8_mut(&mut self) -> Option<&mut [i8]> { self.data.as_i8_slice_mut() }

    /// Typed fast-path access to `u16` storage.
    pub fn data_u16(&self) -> Option<&[u16]> { self.data.as_u16_slice() }
    /// Typed fast-path mutable access to `u16` storage.
    pub fn data_u16_mut(&mut self) -> Option<&mut [u16]> { self.data.as_u16_slice_mut() }

    /// Typed fast-path access to `i16` storage.
    pub fn data_i16(&self) -> Option<&[i16]> { self.data.as_i16_slice() }
    /// Typed fast-path mutable access to `i16` storage.
    pub fn data_i16_mut(&mut self) -> Option<&mut [i16]> { self.data.as_i16_slice_mut() }

    /// Typed fast-path access to `u32` storage.
    pub fn data_u32(&self) -> Option<&[u32]> { self.data.as_u32_slice() }
    /// Typed fast-path mutable access to `u32` storage.
    pub fn data_u32_mut(&mut self) -> Option<&mut [u32]> { self.data.as_u32_slice_mut() }

    /// Typed fast-path access to `i32` storage.
    pub fn data_i32(&self) -> Option<&[i32]> { self.data.as_i32_slice() }
    /// Typed fast-path mutable access to `i32` storage.
    pub fn data_i32_mut(&mut self) -> Option<&mut [i32]> { self.data.as_i32_slice_mut() }

    /// Typed fast-path access to `u64` storage.
    pub fn data_u64(&self) -> Option<&[u64]> { self.data.as_u64_slice() }
    /// Typed fast-path mutable access to `u64` storage.
    pub fn data_u64_mut(&mut self) -> Option<&mut [u64]> { self.data.as_u64_slice_mut() }

    /// Typed fast-path access to `i64` storage.
    pub fn data_i64(&self) -> Option<&[i64]> { self.data.as_i64_slice() }
    /// Typed fast-path mutable access to `i64` storage.
    pub fn data_i64_mut(&mut self) -> Option<&mut [i64]> { self.data.as_i64_slice_mut() }

    /// Typed fast-path access to `f32` storage.
    pub fn data_f32(&self) -> Option<&[f32]> { self.data.as_f32_slice() }
    /// Typed fast-path mutable access to `f32` storage.
    pub fn data_f32_mut(&mut self) -> Option<&mut [f32]> { self.data.as_f32_slice_mut() }

    /// Typed fast-path access to `f64` storage.
    pub fn data_f64(&self) -> Option<&[f64]> { self.data.as_f64_slice() }
    /// Typed fast-path mutable access to `f64` storage.
    pub fn data_f64_mut(&mut self) -> Option<&mut [f64]> { self.data.as_f64_slice_mut() }

    // ─── Pixel access ──────────────────────────────────────────────────────

    /// Return the flat buffer index for signed band, row, and column coordinates.
    /// Returns `None` when coordinates are outside the raster bounds.
    #[inline]
    pub fn index(&self, band: isize, row: isize, col: isize) -> Option<usize> {
        if band < 0
            || row < 0
            || col < 0
            || band >= self.bands as isize
            || row >= self.rows as isize
            || col >= self.cols as isize
        {
            return None;
        }
        let band = band as usize;
        let row = row as usize;
        let col = col as usize;
        let band_stride = self.rows * self.cols;
        Some(band * band_stride + row * self.cols + col)
    }

    /// Get the value at signed pixel coordinates `(band, row, col)`.
    ///
    /// Returns the raster's numeric `nodata` sentinel when coordinates are
    /// out-of-bounds or the stored value is nodata.
    #[inline]
    pub fn get(&self, band: isize, row: isize, col: isize) -> f64 {
        self.get_raw(band, row, col).unwrap_or(self.nodata)
    }

    /// Get the value at signed pixel coordinates `(band, row, col)` as
    /// `Option<f64>`.
    ///
    /// Returns `None` if coordinates are out-of-bounds or the value is nodata.
    #[inline]
    pub fn get_opt(&self, band: isize, row: isize, col: isize) -> Option<f64> {
        let idx = self.index(band, row, col)?;
        let v = self.data.get_f64(idx);
        if self.is_nodata(v) { None } else { Some(v) }
    }

    /// Get the raw value (including nodata) at signed pixel coordinates `(band, row, col)`.
    /// Returns `None` only on out-of-bounds.
    #[inline]
    pub fn get_raw(&self, band: isize, row: isize, col: isize) -> Option<f64> {
        let idx = self.index(band, row, col)?;
        Some(self.data.get_f64(idx))
    }

    /// Set the value at signed pixel coordinates `(band, row, col)`.
    ///
    /// # Errors
    /// Returns [`RasterError::OutOfBounds`] if coordinates are outside the grid.
    #[inline]
    pub fn set(&mut self, band: isize, row: isize, col: isize, value: f64) -> Result<()> {
        if band < 0
            || row < 0
            || col < 0
            || band >= self.bands as isize
            || row >= self.rows as isize
            || col >= self.cols as isize
        {
            return Err(RasterError::OutOfBounds {
                band,
                col,
                row,
                bands: self.bands,
                cols: self.cols,
                rows: self.rows,
            });
        }
        let idx = self.index(band, row, col).expect("set bounds prechecked");
        self.data.set_f64(idx, value);
        Ok(())
    }

    /// Set a value at signed pixel coordinates, panicking on out-of-bounds. Convenience alias.
    #[inline]
    pub fn set_unchecked(&mut self, band: isize, row: isize, col: isize, value: f64) {
        let idx = self
            .index(band, row, col)
            .expect("set_unchecked requires in-bounds coordinates");
        self.data.set_f64(idx, value);
    }

    /// Returns `true` if `v` equals this raster's nodata sentinel.
    #[inline]
    pub fn is_nodata(&self, v: f64) -> bool {
        if self.nodata.is_nan() {
            v.is_nan()
        } else {
            (v - self.nodata).abs() < 1e-10 * self.nodata.abs().max(1.0)
        }
    }

    // ─── Geometry helpers ─────────────────────────────────────────────────

    /// Northern extent (Y max) — top of the grid.
    #[inline]
    pub fn y_max(&self) -> f64 {
        self.y_min + self.rows as f64 * self.cell_size_y
    }

    /// Eastern extent (X max) — right edge of the grid.
    #[inline]
    pub fn x_max(&self) -> f64 {
        self.x_min + self.cols as f64 * self.cell_size_x
    }

    /// The geographic extent of the raster.
    pub fn extent(&self) -> Extent {
        Extent {
            x_min: self.x_min,
            y_min: self.y_min,
            x_max: self.x_max(),
            y_max: self.y_max(),
        }
    }

    /// Cell-center X coordinate for signed column index `col`.
    #[inline]
    pub fn col_center_x(&self, col: isize) -> f64 {
        self.x_min + (col as f64 + 0.5) * self.cell_size_x
    }

    /// Cell-center Y coordinate for signed row index `row` (row 0 = north).
    #[inline]
    pub fn row_center_y(&self, row: isize) -> f64 {
        self.y_max() - (row as f64 + 0.5) * self.cell_size_y
    }

    /// Convert geographic coordinates `(x, y)` to signed pixel indices `(col, row)`.
    /// Returns `None` if the point lies outside the raster extent.
    pub fn world_to_pixel(&self, x: f64, y: f64) -> Option<(isize, isize)> {
        if x < self.x_min || x >= self.x_max() || y < self.y_min || y >= self.y_max() {
            return None;
        }
        let col = ((x - self.x_min) / self.cell_size_x).floor() as isize;
        let row = ((self.y_max() - y) / self.cell_size_y).floor() as isize;
        let col = col.min(self.cols as isize - 1);
        let row = row.min(self.rows as isize - 1);
        Some((col, row))
    }

    /// Assign a CRS to this raster using an EPSG code.
    ///
    /// Replaces the entire `crs` struct with a new `CrsInfo` containing only the EPSG code.
    /// Any existing `wkt` or `proj4` fields are cleared to ensure CRS consistency.
    pub fn assign_crs_epsg(&mut self, epsg: u32) {
        self.crs = CrsInfo {
            epsg: Some(epsg),
            wkt: None,
            proj4: None,
        };
    }

    /// Assign a CRS to this raster using WKT text.
    ///
    /// Replaces the entire `crs` struct with a new `CrsInfo` containing only the WKT definition.
    /// Any existing `epsg` or `proj4` fields are cleared to ensure CRS consistency.
    pub fn assign_crs_wkt(&mut self, wkt: &str) {
        self.crs = CrsInfo {
            epsg: None,
            wkt: Some(wkt.to_string()),
            proj4: None,
        };
    }

    /// Reproject this raster to another EPSG CRS.
    ///
    /// This MVP implementation uses an auto-derived output extent from transformed
    /// sampled source-boundary points (corners + edge densification) and supports
    /// nearest, bilinear, cubic, and Lanczos sampling.
    ///
    /// For explicit output grid controls (`cols`, `rows`, `extent`), use
    /// [`Raster::reproject_with_options`].
    ///
    /// # Errors
    /// Returns an error when source/destination EPSG codes are unsupported,
    /// source CRS is missing, or transformed extents are invalid.
    pub fn reproject_to_epsg(&self, dst_epsg: u32, resample: ResampleMethod) -> Result<Raster> {
        self.reproject_with_options(&ReprojectOptions::new(dst_epsg, resample))
    }

    /// Reproject this raster using detailed output-grid options.
    pub fn reproject_with_options(&self, options: &ReprojectOptions) -> Result<Raster> {
        let src_epsg = self.crs.epsg.ok_or_else(|| {
            RasterError::Other(
                "reproject_to_epsg requires source CRS EPSG in raster.crs.epsg".to_string(),
            )
        })?;
        let src_crs = Crs::from_epsg(src_epsg).map_err(|e| {
            RasterError::Other(format!("source EPSG {src_epsg} is not supported: {e}"))
        })?;
        let dst_crs = Crs::from_epsg(options.dst_epsg).map_err(|e| {
            RasterError::Other(format!(
                "destination EPSG {} is not supported: {e}",
                options.dst_epsg
            ))
        })?;

        self.reproject_internal(&src_crs, &dst_crs, options, None)
    }

    /// Reproject this raster using detailed output-grid options and emit
    /// progress updates in the range [0, 1] as destination rows are completed.
    pub fn reproject_with_options_and_progress<F>(
        &self,
        options: &ReprojectOptions,
        progress: F,
    ) -> Result<Raster>
    where
        F: Fn(f64) + Send + Sync,
    {
        let src_epsg = self.crs.epsg.ok_or_else(|| {
            RasterError::Other(
                "reproject_to_epsg requires source CRS EPSG in raster.crs.epsg".to_string(),
            )
        })?;
        let src_crs = Crs::from_epsg(src_epsg).map_err(|e| {
            RasterError::Other(format!("source EPSG {src_epsg} is not supported: {e}"))
        })?;
        let dst_crs = Crs::from_epsg(options.dst_epsg).map_err(|e| {
            RasterError::Other(format!(
                "destination EPSG {} is not supported: {e}",
                options.dst_epsg
            ))
        })?;

        self.reproject_internal(&src_crs, &dst_crs, options, Some(&progress))
    }

    /// Reproject this raster using caller-supplied source/destination CRS objects.
    ///
    /// This advanced path bypasses the requirement that `self.crs.epsg` is set,
    /// enabling workflows where CRS definitions are managed externally.
    ///
    /// Note: `options.dst_epsg` is still used for output `CrsInfo` metadata and
    /// EPSG-specific extent behavior (e.g., antimeridian handling for EPSG:4326).
    pub fn reproject_with_crs(
        &self,
        src_crs: &Crs,
        dst_crs: &Crs,
        options: &ReprojectOptions,
    ) -> Result<Raster> {
        self.reproject_internal(src_crs, dst_crs, options, None)
    }

    /// Reproject this raster with caller-supplied CRS objects and emit progress
    /// updates in the range [0, 1] as destination rows are completed.
    pub fn reproject_with_crs_and_progress<F>(
        &self,
        src_crs: &Crs,
        dst_crs: &Crs,
        options: &ReprojectOptions,
        progress: F,
    ) -> Result<Raster>
    where
        F: Fn(f64) + Send + Sync,
    {
        self.reproject_internal(src_crs, dst_crs, options, Some(&progress))
    }

    /// Convenience helper for nearest-neighbor reprojection.
    pub fn reproject_to_epsg_nearest(&self, dst_epsg: u32) -> Result<Raster> {
        self.reproject_to_epsg(dst_epsg, ResampleMethod::Nearest)
    }

    /// Convenience helper for bilinear reprojection.
    pub fn reproject_to_epsg_bilinear(&self, dst_epsg: u32) -> Result<Raster> {
        self.reproject_to_epsg(dst_epsg, ResampleMethod::Bilinear)
    }

    /// Convenience helper for cubic reprojection.
    pub fn reproject_to_epsg_cubic(&self, dst_epsg: u32) -> Result<Raster> {
        self.reproject_to_epsg(dst_epsg, ResampleMethod::Cubic)
    }

    /// Reproject to destination EPSG using Lanczos interpolation.
    pub fn reproject_to_epsg_lanczos(&self, dst_epsg: u32) -> Result<Raster> {
        self.reproject_to_epsg(dst_epsg, ResampleMethod::Lanczos)
    }

    /// Reproject to destination EPSG using 3x3 mean resampling.
    pub fn reproject_to_epsg_average(&self, dst_epsg: u32) -> Result<Raster> {
        self.reproject_to_epsg(dst_epsg, ResampleMethod::Average)
    }

    /// Reproject to destination EPSG using 3x3 minimum resampling.
    pub fn reproject_to_epsg_min(&self, dst_epsg: u32) -> Result<Raster> {
        self.reproject_to_epsg(dst_epsg, ResampleMethod::Min)
    }

    /// Reproject to destination EPSG using 3x3 maximum resampling.
    pub fn reproject_to_epsg_max(&self, dst_epsg: u32) -> Result<Raster> {
        self.reproject_to_epsg(dst_epsg, ResampleMethod::Max)
    }

    /// Reproject to destination EPSG using 3x3 modal resampling.
    pub fn reproject_to_epsg_mode(&self, dst_epsg: u32) -> Result<Raster> {
        self.reproject_to_epsg(dst_epsg, ResampleMethod::Mode)
    }

    /// Reproject to destination EPSG using 3x3 median resampling.
    pub fn reproject_to_epsg_median(&self, dst_epsg: u32) -> Result<Raster> {
        self.reproject_to_epsg(dst_epsg, ResampleMethod::Median)
    }

    /// Reproject to destination EPSG using 3x3 standard-deviation resampling.
    pub fn reproject_to_epsg_stddev(&self, dst_epsg: u32) -> Result<Raster> {
        self.reproject_to_epsg(dst_epsg, ResampleMethod::StdDev)
    }

    /// Reproject this raster to match another raster's grid (CRS, extent, rows, cols).
    ///
    /// The `target_grid` provides destination EPSG, output extent, and output
    /// dimensions. This is useful when aligning products from multiple sources
    /// onto a shared reference grid.
    ///
    /// # Errors
    /// Returns an error if `target_grid.crs.epsg` is missing or unsupported.
    pub fn reproject_to_match_grid(
        &self,
        target_grid: &Raster,
        resample: ResampleMethod,
    ) -> Result<Raster> {
        let dst_epsg = target_grid.crs.epsg.ok_or_else(|| {
            RasterError::Other(
                "reproject_to_match_grid requires target grid CRS EPSG in target_grid.crs.epsg"
                    .to_string(),
            )
        })?;

        let opts = ReprojectOptions::new(dst_epsg, resample)
            .with_size(target_grid.cols, target_grid.rows)
            .with_extent(target_grid.extent());

        self.reproject_with_options(&opts)
    }

    /// Reproject this raster to match another raster's grid while emitting
    /// progress updates in the range [0, 1] as destination rows are completed.
    pub fn reproject_to_match_grid_and_progress<F>(
        &self,
        target_grid: &Raster,
        resample: ResampleMethod,
        progress: F,
    ) -> Result<Raster>
    where
        F: Fn(f64) + Send + Sync,
    {
        let dst_epsg = target_grid.crs.epsg.ok_or_else(|| {
            RasterError::Other(
                "reproject_to_match_grid requires target grid CRS EPSG in target_grid.crs.epsg"
                    .to_string(),
            )
        })?;

        let opts = ReprojectOptions::new(dst_epsg, resample)
            .with_size(target_grid.cols, target_grid.rows)
            .with_extent(target_grid.extent());

        self.reproject_with_options_and_progress(&opts, progress)
    }

    /// Reproject this raster using another raster's CRS, resolution, and snap origin.
    ///
    /// Unlike [`Raster::reproject_to_match_grid`], this keeps the destination
    /// extent auto-derived from the transformed source footprint, while aligning
    /// that extent to the reference grid's origin and pixel size.
    ///
    /// # Errors
    /// Returns an error if `reference_grid.crs.epsg` is missing or unsupported.
    pub fn reproject_to_match_resolution(
        &self,
        reference_grid: &Raster,
        resample: ResampleMethod,
    ) -> Result<Raster> {
        let dst_epsg = reference_grid.crs.epsg.ok_or_else(|| {
            RasterError::Other(
                "reproject_to_match_resolution requires reference grid CRS EPSG in reference_grid.crs.epsg"
                    .to_string(),
            )
        })?;

        let opts = ReprojectOptions::new(dst_epsg, resample)
            .with_resolution(reference_grid.cell_size_x, reference_grid.cell_size_y)
            .with_snap_origin(reference_grid.x_min, reference_grid.y_min);

        self.reproject_with_options(&opts)
    }

    /// Reproject this raster while matching a reference raster's resolution
    /// and snap origin, emitting progress updates in [0, 1].
    pub fn reproject_to_match_resolution_and_progress<F>(
        &self,
        reference_grid: &Raster,
        resample: ResampleMethod,
        progress: F,
    ) -> Result<Raster>
    where
        F: Fn(f64) + Send + Sync,
    {
        let dst_epsg = reference_grid.crs.epsg.ok_or_else(|| {
            RasterError::Other(
                "reproject_to_match_resolution requires reference grid CRS EPSG in reference_grid.crs.epsg"
                    .to_string(),
            )
        })?;

        let opts = ReprojectOptions::new(dst_epsg, resample)
            .with_resolution(reference_grid.cell_size_x, reference_grid.cell_size_y)
            .with_snap_origin(reference_grid.x_min, reference_grid.y_min);

        self.reproject_with_options_and_progress(&opts, progress)
    }

    /// Reproject this raster to an explicit destination EPSG while matching a
    /// reference raster's resolution and snap origin.
    ///
    /// If `reference_grid` is in a different CRS than `dst_epsg`, the
    /// reference snap origin and per-axis cell sizes are transformed to
    /// destination CRS using local axis steps at the reference origin.
    ///
    /// # Errors
    /// Returns an error if reference/destination EPSG values are missing or
    /// unsupported, or if transformed reference resolution is invalid.
    pub fn reproject_to_match_resolution_in_epsg(
        &self,
        dst_epsg: u32,
        reference_grid: &Raster,
        resample: ResampleMethod,
    ) -> Result<Raster> {
        let reference_epsg = reference_grid.crs.epsg.ok_or_else(|| {
            RasterError::Other(
                "reproject_to_match_resolution_in_epsg requires reference grid CRS EPSG in reference_grid.crs.epsg"
                    .to_string(),
            )
        })?;

        let (snap_x, snap_y, x_res, y_res) = if reference_epsg == dst_epsg {
            (
                reference_grid.x_min,
                reference_grid.y_min,
                reference_grid.cell_size_x,
                reference_grid.cell_size_y,
            )
        } else {
            let ref_crs = Crs::from_epsg(reference_epsg).map_err(|e| {
                RasterError::Other(format!(
                    "reference EPSG {reference_epsg} is not supported: {e}"
                ))
            })?;
            let dst_crs = Crs::from_epsg(dst_epsg).map_err(|e| {
                RasterError::Other(format!(
                    "destination EPSG {dst_epsg} is not supported: {e}"
                ))
            })?;

            let ox = reference_grid.x_min;
            let oy = reference_grid.y_min;
            let (sx, sy) = ref_crs.transform_to(ox, oy, &dst_crs).map_err(|e| {
                RasterError::Other(format!(
                    "failed to transform reference snap origin to EPSG:{dst_epsg}: {e}"
                ))
            })?;
            let (sx_dx, sy_dx) = ref_crs
                .transform_to(ox + reference_grid.cell_size_x, oy, &dst_crs)
                .map_err(|e| {
                    RasterError::Other(format!(
                        "failed to transform reference X-step to EPSG:{dst_epsg}: {e}"
                    ))
                })?;
            let (sx_dy, sy_dy) = ref_crs
                .transform_to(ox, oy + reference_grid.cell_size_y, &dst_crs)
                .map_err(|e| {
                    RasterError::Other(format!(
                        "failed to transform reference Y-step to EPSG:{dst_epsg}: {e}"
                    ))
                })?;

            let rx = (sx_dx - sx).hypot(sy_dx - sy);
            let ry = (sx_dy - sx).hypot(sy_dy - sy);
            if !rx.is_finite() || !ry.is_finite() || rx <= 0.0 || ry <= 0.0 {
                return Err(RasterError::Other(
                    "invalid transformed reference resolution while matching destination EPSG"
                        .to_string(),
                ));
            }
            (sx, sy, rx, ry)
        };

        let opts = ReprojectOptions::new(dst_epsg, resample)
            .with_resolution(x_res, y_res)
            .with_snap_origin(snap_x, snap_y);

        self.reproject_with_options(&opts)
    }

    /// Reproject this raster to an explicit destination EPSG while matching a
    /// reference raster's transformed resolution/snap, emitting progress in [0, 1].
    pub fn reproject_to_match_resolution_in_epsg_and_progress<F>(
        &self,
        dst_epsg: u32,
        reference_grid: &Raster,
        resample: ResampleMethod,
        progress: F,
    ) -> Result<Raster>
    where
        F: Fn(f64) + Send + Sync,
    {
        let reference_epsg = reference_grid.crs.epsg.ok_or_else(|| {
            RasterError::Other(
                "reproject_to_match_resolution_in_epsg requires reference grid CRS EPSG in reference_grid.crs.epsg"
                    .to_string(),
            )
        })?;

        let (snap_x, snap_y, x_res, y_res) = if reference_epsg == dst_epsg {
            (
                reference_grid.x_min,
                reference_grid.y_min,
                reference_grid.cell_size_x,
                reference_grid.cell_size_y,
            )
        } else {
            let ref_crs = Crs::from_epsg(reference_epsg).map_err(|e| {
                RasterError::Other(format!(
                    "reference EPSG {reference_epsg} is not supported: {e}"
                ))
            })?;
            let dst_crs = Crs::from_epsg(dst_epsg).map_err(|e| {
                RasterError::Other(format!(
                    "destination EPSG {dst_epsg} is not supported: {e}"
                ))
            })?;

            let ox = reference_grid.x_min;
            let oy = reference_grid.y_min;
            let (sx, sy) = ref_crs.transform_to(ox, oy, &dst_crs).map_err(|e| {
                RasterError::Other(format!(
                    "failed to transform reference snap origin to EPSG:{dst_epsg}: {e}"
                ))
            })?;
            let (sx_dx, sy_dx) = ref_crs
                .transform_to(ox + reference_grid.cell_size_x, oy, &dst_crs)
                .map_err(|e| {
                    RasterError::Other(format!(
                        "failed to transform reference X-step to EPSG:{dst_epsg}: {e}"
                    ))
                })?;
            let (sx_dy, sy_dy) = ref_crs
                .transform_to(ox, oy + reference_grid.cell_size_y, &dst_crs)
                .map_err(|e| {
                    RasterError::Other(format!(
                        "failed to transform reference Y-step to EPSG:{dst_epsg}: {e}"
                    ))
                })?;

            let rx = (sx_dx - sx).hypot(sy_dx - sy);
            let ry = (sx_dy - sx).hypot(sy_dy - sy);
            if !rx.is_finite() || !ry.is_finite() || rx <= 0.0 || ry <= 0.0 {
                return Err(RasterError::Other(
                    "invalid transformed reference resolution while matching destination EPSG"
                        .to_string(),
                ));
            }
            (sx, sy, rx, ry)
        };

        let opts = ReprojectOptions::new(dst_epsg, resample)
            .with_resolution(x_res, y_res)
            .with_snap_origin(snap_x, snap_y);

        self.reproject_with_options_and_progress(&opts, progress)
    }

    fn reproject_internal(
        &self,
        src_crs: &Crs,
        dst_crs: &Crs,
        options: &ReprojectOptions,
        progress: Option<&(dyn Fn(f64) + Send + Sync)>,
    ) -> Result<Raster> {
        let src_extent = self.extent();
        let samples_per_edge = (self.cols.max(self.rows) / 32).clamp(8, 128);
        let base_extent = transformed_extent_from_boundary_samples(
            src_crs,
            dst_crs,
            src_extent,
            samples_per_edge,
            options.dst_epsg,
            options.antimeridian_policy,
        )?;
        let out_extent = options.extent.unwrap_or(base_extent);
        let width = out_extent.x_max - out_extent.x_min;
        let height = out_extent.y_max - out_extent.y_min;

        if !out_extent.x_min.is_finite()
            || !out_extent.x_max.is_finite()
            || !out_extent.y_min.is_finite()
            || !out_extent.y_max.is_finite()
            || width <= 0.0
            || height <= 0.0
        {
            return Err(RasterError::CorruptData(
                "invalid transformed extent produced during reprojection".to_string(),
            ));
        }

        let x_res = options.x_res.map(f64::abs);
        let y_res = options.y_res.map(f64::abs);
        if x_res.is_some_and(|v| !v.is_finite() || v <= 0.0)
            || y_res.is_some_and(|v| !v.is_finite() || v <= 0.0)
        {
            return Err(RasterError::CorruptData(
                "invalid reprojection resolution (x_res/y_res must be positive finite values)"
                    .to_string(),
            ));
        }

        let mut x_min = out_extent.x_min;
        let mut x_max = out_extent.x_max;
        let mut y_min = out_extent.y_min;
        let mut y_max = out_extent.y_max;

        let out_cols = match options.cols {
            Some(cols) => cols,
            None => match x_res {
                Some(rx) => {
                    if let Some(sx) = options.snap_x {
                        match options.grid_size_policy {
                            GridSizePolicy::Expand => {
                                x_min = snap_down_to_origin(x_min, sx, rx);
                                x_max = snap_up_to_origin(x_max, sx, rx);
                            }
                            GridSizePolicy::FitInside => {
                                x_min = snap_up_to_origin(x_min, sx, rx);
                                x_max = snap_down_to_origin(x_max, sx, rx);
                            }
                        }
                    }
                    let span = (x_max - x_min).max(0.0);
                    let cols = match options.grid_size_policy {
                        GridSizePolicy::Expand => (span / rx).ceil().max(1.0) as usize,
                        GridSizePolicy::FitInside => (span / rx).floor().max(1.0) as usize,
                    };
                    x_max = x_min + cols as f64 * rx;
                    cols
                }
                None => self.cols,
            },
        };
        let out_rows = match options.rows {
            Some(rows) => rows,
            None => match y_res {
                Some(ry) => {
                    if let Some(sy) = options.snap_y {
                        match options.grid_size_policy {
                            GridSizePolicy::Expand => {
                                y_min = snap_down_to_origin(y_min, sy, ry);
                                y_max = snap_up_to_origin(y_max, sy, ry);
                            }
                            GridSizePolicy::FitInside => {
                                y_min = snap_up_to_origin(y_min, sy, ry);
                                y_max = snap_down_to_origin(y_max, sy, ry);
                            }
                        }
                    }
                    let span = (y_max - y_min).max(0.0);
                    let rows = match options.grid_size_policy {
                        GridSizePolicy::Expand => (span / ry).ceil().max(1.0) as usize,
                        GridSizePolicy::FitInside => (span / ry).floor().max(1.0) as usize,
                    };
                    y_max = y_min + rows as f64 * ry;
                    rows
                }
                None => self.rows,
            },
        };

        let out_extent = Extent {
            x_min,
            y_min,
            x_max,
            y_max,
        };

        if out_cols == 0 || out_rows == 0 {
            return Err(RasterError::InvalidDimensions {
                cols: out_cols,
                rows: out_rows,
            });
        }

        let cfg = RasterConfig {
            cols: out_cols,
            rows: out_rows,
            bands: self.bands,
            x_min: out_extent.x_min,
            y_min: out_extent.y_min,
            cell_size: (out_extent.x_max - out_extent.x_min) / out_cols as f64,
            cell_size_y: Some((out_extent.y_max - out_extent.y_min) / out_rows as f64),
            nodata: self.nodata,
            data_type: self.data_type,
            crs: CrsInfo::from_epsg(options.dst_epsg),
            metadata: self.metadata.clone(),
        };
        let mut out = Raster::new(cfg);

        let out_y_max = out.y_max();
        let footprint_ring = if options.destination_footprint == DestinationFootprint::SourceBoundary {
            let ring = transformed_boundary_ring_samples(
                src_crs,
                dst_crs,
                src_extent,
                samples_per_edge,
                options.dst_epsg,
                options.antimeridian_policy,
            )?;
            if ring.len() >= 3 {
                Some(ring)
            } else {
                None
            }
        } else {
            None
        };

        let total_rows = out.rows;
        let completed_rows = AtomicUsize::new(0);
        let rows_data: Vec<Vec<Option<f64>>> = (0..out.rows as isize)
            .into_par_iter()
            .map(|row| {
                let mut row_values = vec![None; out.cols * out.bands];
                let y = out_y_max - (row as f64 + 0.5) * out.cell_size_y;

                // Collect per-row coordinates that pass the footprint check, then
                // transform them all in one batch call so the SIMD fast paths in
                // transform_to_batch can vectorize across the full row width.
                let mut batch_coords: Vec<(f64, f64)> = Vec::with_capacity(out.cols);
                let mut batch_cols: Vec<usize> = Vec::with_capacity(out.cols);
                for col in 0..out.cols as isize {
                    let x = out.x_min + (col as f64 + 0.5) * out.cell_size_x;
                    if let Some(ring) = &footprint_ring {
                        if !point_in_polygon(x, y, ring) {
                            continue;
                        }
                    }
                    batch_coords.push((x, y));
                    batch_cols.push(col as usize);
                }

                // Single batch CRS transform for all eligible pixels in this row.
                // Successful transforms overwrite batch_coords in-place; errors are
                // returned as Some(Err(_)) at the corresponding index.
                let errors = dst_crs.transform_to_batch(&mut batch_coords, src_crs);

                for (i, &col) in batch_cols.iter().enumerate() {
                    if errors[i].is_some() {
                        continue;
                    }
                    let (sx, sy) = batch_coords[i];
                    for band in 0..out.bands as isize {
                        if let Some(v) = self.sample_world(
                            band,
                            sx,
                            sy,
                            options.resample,
                            options.nodata_policy,
                        ) {
                            row_values[band as usize * out.cols + col] = Some(v);
                        }
                    }
                }

                if let Some(progress_cb) = progress {
                    let done = completed_rows.fetch_add(1, Ordering::Relaxed) + 1;
                    progress_cb(done as f64 / total_rows as f64);
                }

                row_values
            })
            .collect();

        for (row, row_values) in rows_data.into_iter().enumerate() {
            let row = row as isize;
            for band in 0..out.bands {
                for col in 0..out.cols {
                    if let Some(v) = row_values[band * out.cols + col] {
                        out.set_unchecked(band as isize, row, col as isize, v);
                    }
                }
            }
        }

        if let Some(progress_cb) = progress {
            progress_cb(1.0);
        }

        Ok(out)
    }

    /// Sample a raster value at world coordinates using the selected resampling method.
    pub fn sample_world(
        &self,
        band: isize,
        x: f64,
        y: f64,
        method: ResampleMethod,
        nodata_policy: NodataPolicy,
    ) -> Option<f64> {
        let col_f = (x - self.x_min) / self.cell_size_x - 0.5;
        let row_f = (self.y_max() - y) / self.cell_size_y - 0.5;
        match method {
            ResampleMethod::Nearest => self.sample_nearest_pixel(band, col_f, row_f),
            ResampleMethod::Bilinear => match nodata_policy {
                NodataPolicy::Strict => self.sample_bilinear_strict_pixel(band, col_f, row_f),
                NodataPolicy::PartialKernel => {
                    self.sample_bilinear_partial_pixel(band, col_f, row_f)
                }
                NodataPolicy::Fill => self
                    .sample_bilinear_strict_pixel(band, col_f, row_f)
                    .or_else(|| self.sample_nearest_pixel(band, col_f, row_f)),
            },
            ResampleMethod::Cubic => match nodata_policy {
                NodataPolicy::Strict => self.sample_cubic_strict_pixel(band, col_f, row_f),
                NodataPolicy::PartialKernel => self.sample_cubic_partial_pixel(band, col_f, row_f),
                NodataPolicy::Fill => self
                    .sample_cubic_strict_pixel(band, col_f, row_f)
                    .or_else(|| self.sample_nearest_pixel(band, col_f, row_f)),
            },
            ResampleMethod::Lanczos => match nodata_policy {
                NodataPolicy::Strict => self.sample_lanczos_strict_pixel(band, col_f, row_f),
                NodataPolicy::PartialKernel => {
                    self.sample_lanczos_partial_pixel(band, col_f, row_f)
                }
                NodataPolicy::Fill => self
                    .sample_lanczos_strict_pixel(band, col_f, row_f)
                    .or_else(|| self.sample_nearest_pixel(band, col_f, row_f)),
            },
            ResampleMethod::Average => self.sample_window_stat_pixel(
                band,
                col_f,
                row_f,
                WindowStat::Mean,
                nodata_policy,
            ),
            ResampleMethod::Min => self.sample_window_stat_pixel(
                band,
                col_f,
                row_f,
                WindowStat::Min,
                nodata_policy,
            ),
            ResampleMethod::Max => self.sample_window_stat_pixel(
                band,
                col_f,
                row_f,
                WindowStat::Max,
                nodata_policy,
            ),
            ResampleMethod::Mode => self.sample_window_stat_pixel(
                band,
                col_f,
                row_f,
                WindowStat::Mode,
                nodata_policy,
            ),
            ResampleMethod::Median => self.sample_window_stat_pixel(
                band,
                col_f,
                row_f,
                WindowStat::Median,
                nodata_policy,
            ),
            ResampleMethod::StdDev => self.sample_window_stat_pixel(
                band,
                col_f,
                row_f,
                WindowStat::StdDev,
                nodata_policy,
            ),
        }
    }

    fn sample_window_stat_pixel(
        &self,
        band: isize,
        col_f: f64,
        row_f: f64,
        stat: WindowStat,
        nodata_policy: NodataPolicy,
    ) -> Option<f64> {
        if !col_f.is_finite() || !row_f.is_finite() {
            return None;
        }

        let center_col = col_f.round() as isize;
        let center_row = row_f.round() as isize;
        let mut values = Vec::with_capacity(9);
        let mut valid_count = 0usize;

        for dy in -1..=1 {
            for dx in -1..=1 {
                let c = center_col + dx;
                let r = center_row + dy;
                if c < 0 || r < 0 || c >= self.cols as isize || r >= self.rows as isize {
                    continue;
                }
                if let Some(v) = self.get_opt(band, r, c) {
                    values.push(v);
                    valid_count += 1;
                }
            }
        }

        match nodata_policy {
            NodataPolicy::Strict if valid_count < 9 => None,
            NodataPolicy::PartialKernel | NodataPolicy::Strict => {
                reduce_window_values(&values, stat)
            }
            NodataPolicy::Fill => reduce_window_values(&values, stat)
                .or_else(|| self.sample_nearest_pixel(band, col_f, row_f)),
        }
    }

    fn sample_nearest_pixel(&self, band: isize, col_f: f64, row_f: f64) -> Option<f64> {
        if !col_f.is_finite() || !row_f.is_finite() {
            return None;
        }
        let col = col_f.round() as isize;
        let row = row_f.round() as isize;
        self.get_opt(band, row, col)
    }

    fn sample_bilinear_strict_pixel(&self, band: isize, col_f: f64, row_f: f64) -> Option<f64> {
        if !col_f.is_finite() || !row_f.is_finite() {
            return None;
        }
        let c0 = col_f.floor() as isize;
        let r0 = row_f.floor() as isize;
        let c1 = c0 + 1;
        let r1 = r0 + 1;
        if c0 < 0 || r0 < 0 || c1 >= self.cols as isize || r1 >= self.rows as isize {
            return None;
        }

        let tx = col_f - c0 as f64;
        let ty = row_f - r0 as f64;

        if let Some(values) = self.data_f64() {
            return self.sample_bilinear_strict_simd_f64(values, band, r0, c0, tx, ty);
        }
        if let Some(values) = self.data_f32() {
            return self.sample_bilinear_strict_simd_f32(values, band, r0, c0, tx, ty);
        }

        let q00 = self.get_opt(band, r0, c0)?;
        let q10 = self.get_opt(band, r0, c1)?;
        let q01 = self.get_opt(band, r1, c0)?;
        let q11 = self.get_opt(band, r1, c1)?;

        let a = q00 * (1.0 - tx) + q10 * tx;
        let b = q01 * (1.0 - tx) + q11 * tx;
        Some(a * (1.0 - ty) + b * ty)
    }

    #[inline]
    fn sample_bilinear_strict_simd_f64(
        &self,
        values: &[f64],
        band: isize,
        r0: isize,
        c0: isize,
        tx: f64,
        ty: f64,
    ) -> Option<f64> {
        let band_stride = self.rows * self.cols;
        let base = band as usize * band_stride + r0 as usize * self.cols + c0 as usize;
        let q00 = values[base];
        let q10 = values[base + 1];
        let q01 = values[base + self.cols];
        let q11 = values[base + self.cols + 1];

        if self.is_nodata(q00)
            || self.is_nodata(q10)
            || self.is_nodata(q01)
            || self.is_nodata(q11)
        {
            return None;
        }

        let weights = f64x4::new([
            (1.0 - tx) * (1.0 - ty),
            tx * (1.0 - ty),
            (1.0 - tx) * ty,
            tx * ty,
        ]);
        let vals = f64x4::new([q00, q10, q01, q11]);
        let weighted = <[f64; 4]>::from(vals * weights);
        Some(weighted.into_iter().sum())
    }

    #[inline]
    fn sample_bilinear_strict_simd_f32(
        &self,
        values: &[f32],
        band: isize,
        r0: isize,
        c0: isize,
        tx: f64,
        ty: f64,
    ) -> Option<f64> {
        let band_stride = self.rows * self.cols;
        let base = band as usize * band_stride + r0 as usize * self.cols + c0 as usize;
        let q00 = values[base] as f64;
        let q10 = values[base + 1] as f64;
        let q01 = values[base + self.cols] as f64;
        let q11 = values[base + self.cols + 1] as f64;

        if self.is_nodata(q00)
            || self.is_nodata(q10)
            || self.is_nodata(q01)
            || self.is_nodata(q11)
        {
            return None;
        }

        let weights = f64x4::new([
            (1.0 - tx) * (1.0 - ty),
            tx * (1.0 - ty),
            (1.0 - tx) * ty,
            tx * ty,
        ]);
        let vals = f64x4::new([q00, q10, q01, q11]);
        let weighted = <[f64; 4]>::from(vals * weights);
        Some(weighted.into_iter().sum())
    }

    fn sample_bilinear_partial_pixel(&self, band: isize, col_f: f64, row_f: f64) -> Option<f64> {
        if !col_f.is_finite() || !row_f.is_finite() {
            return None;
        }
        let c0 = col_f.floor() as isize;
        let r0 = row_f.floor() as isize;
        let c1 = c0 + 1;
        let r1 = r0 + 1;

        let tx = col_f - c0 as f64;
        let ty = row_f - r0 as f64;

        let neighbors = [
            (c0, r0, (1.0 - tx) * (1.0 - ty)),
            (c1, r0, tx * (1.0 - ty)),
            (c0, r1, (1.0 - tx) * ty),
            (c1, r1, tx * ty),
        ];

        let mut sum = 0.0;
        let mut wsum = 0.0;
        for (c, r, w) in neighbors {
            if w <= 0.0 || c < 0 || r < 0 || c >= self.cols as isize || r >= self.rows as isize {
                continue;
            }
            if let Some(v) = self.get_opt(band, r, c) {
                sum += v * w;
                wsum += w;
            }
        }

        if wsum > 0.0 {
            Some(sum / wsum)
        } else {
            None
        }
    }

    fn sample_cubic_strict_pixel(&self, band: isize, col_f: f64, row_f: f64) -> Option<f64> {
        if !col_f.is_finite() || !row_f.is_finite() {
            return None;
        }
        let c1 = col_f.floor() as isize;
        let r1 = row_f.floor() as isize;
        if c1 - 1 < 0
            || r1 - 1 < 0
            || c1 + 2 >= self.cols as isize
            || r1 + 2 >= self.rows as isize
        {
            return None;
        }

        let tx = col_f - c1 as f64;
        let ty = row_f - r1 as f64;
        let wx = cubic_bspline_weights(tx);
        let wy = cubic_bspline_weights(ty);

        // Use SIMD-accelerated 4×4 dot product
        self.sample_cubic_simd_kernel(&wx, &wy, band, c1 - 1, r1 - 1)
    }

    /// SIMD-accelerated 4×4 kernel dot product for bicubic resampling.
    /// Assumes all 16 pixels are in-bounds and valid.
    #[inline]
    fn sample_cubic_simd_kernel(
        &self,
        wx: &[f64; 4],
        wy: &[f64; 4],
        band: isize,
        c_start: isize,
        r_start: isize,
    ) -> Option<f64> {
        // Process 4 rows, 4 pixels per row, computing weighted sum using SIMD for horizontal accumulation.
        // Load row-wise and multiply by row weights.
        let mut row_sums = [0.0_f64; 4];

        for (j, _wyj) in wy.iter().enumerate() {
            let rr = r_start + j as isize;
            let mut row_sum = 0.0;

            // Load 4 pixels in row and compute weighted sum
            for (i, wxi) in wx.iter().enumerate() {
                let cc = c_start + i as isize;
                let v = self.get_opt(band, rr, cc)?;
                row_sum += v * *wxi;
            }

            row_sums[j] = row_sum;
        }

        // Horizontal reduction: multiply by row weights and sum
        let mut sum = 0.0;
        for (j, wyj) in wy.iter().enumerate() {
            sum += row_sums[j] * *wyj;
        }

        Some(sum)
    }

    fn sample_cubic_partial_pixel(&self, band: isize, col_f: f64, row_f: f64) -> Option<f64> {
        if !col_f.is_finite() || !row_f.is_finite() {
            return None;
        }
        let c1 = col_f.floor() as isize;
        let r1 = row_f.floor() as isize;
        let tx = col_f - c1 as f64;
        let ty = row_f - r1 as f64;
        let wx = cubic_bspline_weights(tx);
        let wy = cubic_bspline_weights(ty);

        let mut sum = 0.0;
        let mut wsum = 0.0;

        for (j, wyj) in wy.iter().enumerate() {
            let rr = clamp_isize(r1 + j as isize - 1, 0, self.rows as isize - 1);
            if *wyj <= 0.0 {
                continue;
            }
            for (i, wxi) in wx.iter().enumerate() {
                let cc = clamp_isize(c1 + i as isize - 1, 0, self.cols as isize - 1);
                let w = *wxi * *wyj;
                if w <= 0.0 {
                    continue;
                }
                if let Some(v) = self.get_opt(band, rr, cc) {
                    sum += v * w;
                    wsum += w;
                }
            }
        }

        if wsum > 0.0 {
            Some(sum / wsum)
        } else {
            None
        }
    }

    fn sample_lanczos_strict_pixel(&self, band: isize, col_f: f64, row_f: f64) -> Option<f64> {
        if !col_f.is_finite() || !row_f.is_finite() {
            return None;
        }

        let c0 = col_f.floor() as isize;
        let r0 = row_f.floor() as isize;
        if c0 - 2 < 0
            || r0 - 2 < 0
            || c0 + 3 >= self.cols as isize
            || r0 + 3 >= self.rows as isize
        {
            return None;
        }

        let wx = lanczos3_weights(col_f, c0);
        let wy = lanczos3_weights(row_f, r0);

        if let Some(values) = self.data_f64() {
            return self.sample_lanczos_strict_simd_f64(values, band, c0, r0, &wx, &wy);
        }
        if let Some(values) = self.data_f32() {
            return self.sample_lanczos_strict_simd_f32(values, band, c0, r0, &wx, &wy);
        }

        let mut sum = 0.0;
        let mut wsum = 0.0;
        for (j, wyj) in wy.iter().enumerate() {
            let rr = r0 + j as isize - 2;
            for (i, wxi) in wx.iter().enumerate() {
                let cc = c0 + i as isize - 2;
                let v = self.get_opt(band, rr, cc)?;
                let w = *wxi * *wyj;
                sum += v * w;
                wsum += w;
            }
        }

        if wsum.abs() > 1e-12 {
            Some(sum / wsum)
        } else {
            None
        }
    }

    #[inline]
    fn sample_lanczos_strict_simd_f64(
        &self,
        values: &[f64],
        band: isize,
        c0: isize,
        r0: isize,
        wx: &[f64; 6],
        wy: &[f64; 6],
    ) -> Option<f64> {
        let band_stride = self.rows * self.cols;
        let base = band as usize * band_stride;

        let wx0 = f64x4::new([wx[0], wx[1], wx[2], wx[3]]);
        let wx1 = f64x4::new([wx[4], wx[5], 0.0, 0.0]);
        let wx_sum: f64 = wx.iter().copied().sum();

        let mut sum = 0.0;
        let mut wsum = 0.0;
        for (j, wyj) in wy.iter().enumerate() {
            let rr = (r0 + j as isize - 2) as usize;
            let cc = (c0 - 2) as usize;
            let row_offset = base + rr * self.cols + cc;

            let v = [
                values[row_offset],
                values[row_offset + 1],
                values[row_offset + 2],
                values[row_offset + 3],
                values[row_offset + 4],
                values[row_offset + 5],
            ];
            if v.into_iter().any(|cell| self.is_nodata(cell)) {
                return None;
            }

            let v0 = f64x4::new([v[0], v[1], v[2], v[3]]);
            let v1 = f64x4::new([v[4], v[5], 0.0, 0.0]);
            let d0 = <[f64; 4]>::from(v0 * wx0);
            let d1 = <[f64; 4]>::from(v1 * wx1);
            let row_sum = d0.into_iter().sum::<f64>() + d1.into_iter().sum::<f64>();
            sum += row_sum * *wyj;
            wsum += wx_sum * *wyj;
        }

        if wsum.abs() > 1e-12 {
            Some(sum / wsum)
        } else {
            None
        }
    }

    #[inline]
    fn sample_lanczos_strict_simd_f32(
        &self,
        values: &[f32],
        band: isize,
        c0: isize,
        r0: isize,
        wx: &[f64; 6],
        wy: &[f64; 6],
    ) -> Option<f64> {
        let band_stride = self.rows * self.cols;
        let base = band as usize * band_stride;

        let wx0 = f64x4::new([wx[0], wx[1], wx[2], wx[3]]);
        let wx1 = f64x4::new([wx[4], wx[5], 0.0, 0.0]);
        let wx_sum: f64 = wx.iter().copied().sum();

        let mut sum = 0.0;
        let mut wsum = 0.0;
        for (j, wyj) in wy.iter().enumerate() {
            let rr = (r0 + j as isize - 2) as usize;
            let cc = (c0 - 2) as usize;
            let row_offset = base + rr * self.cols + cc;

            let v = [
                values[row_offset] as f64,
                values[row_offset + 1] as f64,
                values[row_offset + 2] as f64,
                values[row_offset + 3] as f64,
                values[row_offset + 4] as f64,
                values[row_offset + 5] as f64,
            ];
            if v.into_iter().any(|cell| self.is_nodata(cell)) {
                return None;
            }

            let v0 = f64x4::new([v[0], v[1], v[2], v[3]]);
            let v1 = f64x4::new([v[4], v[5], 0.0, 0.0]);
            let d0 = <[f64; 4]>::from(v0 * wx0);
            let d1 = <[f64; 4]>::from(v1 * wx1);
            let row_sum = d0.into_iter().sum::<f64>() + d1.into_iter().sum::<f64>();
            sum += row_sum * *wyj;
            wsum += wx_sum * *wyj;
        }

        if wsum.abs() > 1e-12 {
            Some(sum / wsum)
        } else {
            None
        }
    }

    fn sample_lanczos_partial_pixel(&self, band: isize, col_f: f64, row_f: f64) -> Option<f64> {
        if !col_f.is_finite() || !row_f.is_finite() {
            return None;
        }

        let c0 = col_f.floor() as isize;
        let r0 = row_f.floor() as isize;
        let wx = lanczos3_weights(col_f, c0);
        let wy = lanczos3_weights(row_f, r0);

        let mut sum = 0.0;
        let mut wsum = 0.0;

        for (j, wyj) in wy.iter().enumerate() {
            let rr = clamp_isize(r0 + j as isize - 2, 0, self.rows as isize - 1);
            for (i, wxi) in wx.iter().enumerate() {
                let cc = clamp_isize(c0 + i as isize - 2, 0, self.cols as isize - 1);
                let w = *wxi * *wyj;
                if w == 0.0 {
                    continue;
                }
                if let Some(v) = self.get_opt(band, rr, cc) {
                    sum += v * w;
                    wsum += w;
                }
            }
        }

        if wsum.abs() > 1e-12 {
            Some(sum / wsum)
        } else {
            None
        }
    }

    // ─── Statistics ───────────────────────────────────────────────────────

    fn stats_accumulator_range_with_mode(
        &self,
        start: usize,
        end: usize,
        mode: StatisticsComputationMode,
    ) -> StatsAccumulator {
        match mode {
            StatisticsComputationMode::Auto | StatisticsComputationMode::Simd => {
                self.stats_accumulator_range_simd(start, end)
            }
            StatisticsComputationMode::Scalar => self.stats_accumulator_range_scalar(start, end),
        }
    }

    fn stats_accumulator_range_scalar(&self, start: usize, end: usize) -> StatsAccumulator {
        let mut accumulator = StatsAccumulator::default();

        for idx in start..end {
            let value = self.data.get_f64(idx);
            if self.is_nodata(value) {
                accumulator.nodata_count += 1;
            } else {
                accumulator.min = accumulator.min.min(value);
                accumulator.max = accumulator.max.max(value);
                accumulator.sum += value;
                accumulator.sum_sq += value * value;
                accumulator.valid_count += 1;
            }
        }

        accumulator
    }

    /// SIMD-accelerated stats accumulation for a range of indices.
    /// Processes groups of 4 values in parallel where possible, falling back to scalar for remainder.
    /// This is automatically used by the statistics pipeline for better performance.
    fn stats_accumulator_range_simd(&self, start: usize, end: usize) -> StatsAccumulator {
        if let Some(values) = self.data_f64() {
            return self.stats_accumulator_range_simd_f64(values, start, end);
        }
        if let Some(values) = self.data_f32() {
            return self.stats_accumulator_range_simd_f32(values, start, end);
        }

        self.stats_accumulator_range_scalar(start, end)
    }

    fn stats_accumulator_range_simd_f64(
        &self,
        values: &[f64],
        start: usize,
        end: usize,
    ) -> StatsAccumulator {
        let mut accumulator = StatsAccumulator::default();
        let nd = self.nodata;

        let simd_end = start + ((end - start) / 4) * 4;
        let mut simd_min = f64x4::splat(f64::INFINITY);
        let mut simd_max = f64x4::splat(f64::NEG_INFINITY);
        let mut simd_sum = f64x4::splat(0.0);
        let mut simd_sum_sq = f64x4::splat(0.0);
        let zero_v = f64x4::splat(0.0);
        let nd_v = f64x4::splat(nd);
        let inf_v = f64x4::splat(f64::INFINITY);
        let neg_inf_v = f64x4::splat(f64::NEG_INFINITY);

        let mut idx = start;
        while idx < simd_end {
            let chunk = &values[idx..idx + 4];
            let values = f64x4::new([chunk[0], chunk[1], chunk[2], chunk[3]]);

            let not_nodata = values.simd_ne(nd_v);

            let values_for_min = not_nodata.blend(values, inf_v);
            let values_for_max = not_nodata.blend(values, neg_inf_v);
            simd_min = simd_min.min(values_for_min);
            simd_max = simd_max.max(values_for_max);

            let masked_values = not_nodata.blend(values, zero_v);
            simd_sum = simd_sum + masked_values;
            simd_sum_sq = simd_sum_sq + masked_values * masked_values;

            for &val in chunk {
                if val == nd {
                    accumulator.nodata_count += 1;
                } else {
                    accumulator.valid_count += 1;
                }
            }

            idx += 4;
        }

        let sum_arr = <[f64; 4]>::from(simd_sum);
        let sum_sq_arr = <[f64; 4]>::from(simd_sum_sq);
        let min_arr = <[f64; 4]>::from(simd_min);
        let max_arr = <[f64; 4]>::from(simd_max);

        for i in 0..4 {
            accumulator.sum += sum_arr[i];
            accumulator.sum_sq += sum_sq_arr[i];
            accumulator.min = accumulator.min.min(min_arr[i]);
            accumulator.max = accumulator.max.max(max_arr[i]);
        }

        for &value in &values[simd_end..end] {
            if value == nd {
                accumulator.nodata_count += 1;
            } else {
                accumulator.min = accumulator.min.min(value);
                accumulator.max = accumulator.max.max(value);
                accumulator.sum += value;
                accumulator.sum_sq += value * value;
                accumulator.valid_count += 1;
            }
        }

        accumulator
    }

    fn stats_accumulator_range_simd_f32(
        &self,
        values: &[f32],
        start: usize,
        end: usize,
    ) -> StatsAccumulator {
        let mut accumulator = StatsAccumulator::default();
        let nd = self.nodata as f32;

        let simd_end = start + ((end - start) / 4) * 4;
        let mut simd_min = f64x4::splat(f64::INFINITY);
        let mut simd_max = f64x4::splat(f64::NEG_INFINITY);
        let mut simd_sum = f64x4::splat(0.0);
        let mut simd_sum_sq = f64x4::splat(0.0);
        let zero_v = f64x4::splat(0.0);
        let nd_v = f64x4::splat(nd as f64);
        let inf_v = f64x4::splat(f64::INFINITY);
        let neg_inf_v = f64x4::splat(f64::NEG_INFINITY);

        let mut idx = start;
        while idx < simd_end {
            let chunk = &values[idx..idx + 4];
            let values = f64x4::new([
                chunk[0] as f64,
                chunk[1] as f64,
                chunk[2] as f64,
                chunk[3] as f64,
            ]);

            let not_nodata = values.simd_ne(nd_v);
            let values_for_min = not_nodata.blend(values, inf_v);
            let values_for_max = not_nodata.blend(values, neg_inf_v);
            simd_min = simd_min.min(values_for_min);
            simd_max = simd_max.max(values_for_max);

            let masked_values = not_nodata.blend(values, zero_v);
            simd_sum = simd_sum + masked_values;
            simd_sum_sq = simd_sum_sq + masked_values * masked_values;

            for &val in chunk {
                if val == nd {
                    accumulator.nodata_count += 1;
                } else {
                    accumulator.valid_count += 1;
                }
            }

            idx += 4;
        }

        let sum_arr = <[f64; 4]>::from(simd_sum);
        let sum_sq_arr = <[f64; 4]>::from(simd_sum_sq);
        let min_arr = <[f64; 4]>::from(simd_min);
        let max_arr = <[f64; 4]>::from(simd_max);

        for i in 0..4 {
            accumulator.sum += sum_arr[i];
            accumulator.sum_sq += sum_sq_arr[i];
            accumulator.min = accumulator.min.min(min_arr[i]);
            accumulator.max = accumulator.max.max(max_arr[i]);
        }

        for &value in &values[simd_end..end] {
            if value == nd {
                accumulator.nodata_count += 1;
            } else {
                let value = value as f64;
                accumulator.min = accumulator.min.min(value);
                accumulator.max = accumulator.max.max(value);
                accumulator.sum += value;
                accumulator.sum_sq += value * value;
                accumulator.valid_count += 1;
            }
        }

        accumulator
    }

    fn statistics_for_index_range_with_mode(
        &self,
        start: usize,
        end: usize,
        mode: StatisticsComputationMode,
    ) -> Statistics {
        const PARALLEL_THRESHOLD: usize = 65_536;
        const CHUNK_SIZE: usize = 16_384;

        let span = end.saturating_sub(start);
        if span == 0 {
            return Statistics {
                min: 0.0,
                max: 0.0,
                mean: 0.0,
                std_dev: 0.0,
                valid_count: 0,
                nodata_count: 0,
            };
        }

        let total = if span < PARALLEL_THRESHOLD {
            self.stats_accumulator_range_with_mode(start, end, mode)
        } else {
            let chunk_starts: Vec<usize> = (start..end).step_by(CHUNK_SIZE).collect();
            let partials: Vec<StatsAccumulator> = chunk_starts
                .into_par_iter()
                .map(|chunk_start| {
                    let chunk_end = (chunk_start + CHUNK_SIZE).min(end);
                    self.stats_accumulator_range_with_mode(chunk_start, chunk_end, mode)
                })
                .collect();

            partials.into_iter().fold(StatsAccumulator::default(), |mut lhs, rhs| {
                lhs.merge(rhs);
                lhs
            })
        };

        total.to_statistics()
    }

    /// Compute basic statistics over all valid (non-nodata) cells.
    pub fn statistics(&self) -> Statistics {
        self.statistics_with_mode(StatisticsComputationMode::Auto)
    }

    /// Compute basic statistics over all valid (non-nodata) cells using a selected computation path.
    pub fn statistics_with_mode(&self, mode: StatisticsComputationMode) -> Statistics {
        self.statistics_for_index_range_with_mode(0, self.data.len(), mode)
    }

    /// Compute basic statistics over all valid (non-nodata) cells in one band.
    pub fn statistics_band(&self, band: isize) -> Result<Statistics> {
        self.statistics_band_with_mode(band, StatisticsComputationMode::Auto)
    }

    /// Compute basic statistics over one band using a selected computation path.
    pub fn statistics_band_with_mode(
        &self,
        band: isize,
        mode: StatisticsComputationMode,
    ) -> Result<Statistics> {
        if band < 0 || band >= self.bands as isize {
            return Err(RasterError::OutOfBounds {
                band,
                col: 0,
                row: 0,
                bands: self.bands,
                cols: self.cols,
                rows: self.rows,
            });
        }

        let band = band as usize;
        let band_stride = self.rows * self.cols;
        let start = band * band_stride;
        let end = start + band_stride;

        Ok(self.statistics_for_index_range_with_mode(start, end, mode))
    }

    // ─── Row iteration ────────────────────────────────────────────────────

    /// Return a copy of one full band as row-major values.
    #[inline]
    pub fn band_slice(&self, band: isize) -> Vec<f64> {
        if band < 0 || band >= self.bands as isize {
            return Vec::new();
        }
        let band_stride = self.rows * self.cols;
        let start = band as usize * band_stride;
        (start..start + band_stride)
            .map(|i| self.data.get_f64(i))
            .collect()
    }

    /// Set one full band from row-major values.
    pub fn set_band_slice(&mut self, band: isize, values: &[f64]) -> Result<()> {
        let expected = self.rows * self.cols;
        if values.len() != expected {
            return Err(RasterError::InvalidDimensions {
                cols: self.cols,
                rows: values.len(),
            });
        }
        if band < 0 || band >= self.bands as isize {
            return Err(RasterError::OutOfBounds {
                band,
                col: 0,
                row: 0,
                bands: self.bands,
                cols: self.cols,
                rows: self.rows,
            });
        }
        let band_stride = self.rows * self.cols;
        let start = band as usize * band_stride;
        for (i, v) in values.iter().copied().enumerate() {
            self.data.set_f64(start + i, v);
        }
        Ok(())
    }

    /// Return a slice of the raw data for signed `(band, row)`.
    #[inline]
    pub fn row_slice(&self, band: isize, row: isize) -> Vec<f64> {
        if band < 0 || band >= self.bands as isize || row < 0 || row >= self.rows as isize {
            return Vec::new();
        }
        let band_stride = self.rows * self.cols;
        let start = band as usize * band_stride + row as usize * self.cols;
        (start..start + self.cols).map(|i| self.data.get_f64(i)).collect()
    }

    /// Set all values in signed `(band, row)` from an `f64` slice.
    pub fn set_row_slice(&mut self, band: isize, row: isize, values: &[f64]) -> Result<()> {
        if values.len() != self.cols {
            return Err(RasterError::InvalidDimensions { cols: values.len(), rows: self.rows });
        }
        if band < 0 || band >= self.bands as isize || row < 0 || row >= self.rows as isize {
            return Err(RasterError::OutOfBounds {
                band,
                col: 0,
                row,
                bands: self.bands,
                cols: self.cols,
                rows: self.rows,
            });
        }
        let band_stride = self.rows * self.cols;
        let start = band as usize * band_stride + row as usize * self.cols;
        for (i, v) in values.iter().copied().enumerate() {
            self.data.set_f64(start + i, v);
        }
        Ok(())
    }

    /// Iterate over signed `(band, row, col, value)` for all valid cells.
    pub fn iter_valid(&self) -> impl Iterator<Item = (isize, isize, isize, f64)> + '_ {
        self.data.iter_f64().enumerate().filter_map(move |(idx, v)| {
            if self.is_nodata(v) {
                None
            } else {
                let band_stride = self.rows * self.cols;
                let band = idx / band_stride;
                let rem = idx % band_stride;
                let row = rem / self.cols;
                let col = rem % self.cols;
                Some((band as isize, row as isize, col as isize, v))
            }
        })
    }

    /// Iterate over signed `(row, col, value)` for all valid cells in one band.
    pub fn iter_valid_band(
        &self,
        band: isize,
    ) -> Result<Box<dyn Iterator<Item = (isize, isize, f64)> + '_>> {
        if band < 0 || band >= self.bands as isize {
            return Err(RasterError::OutOfBounds {
                band,
                col: 0,
                row: 0,
                bands: self.bands,
                cols: self.cols,
                rows: self.rows,
            });
        }

        let b = band as usize;
        let band_stride = self.rows * self.cols;
        let start = b * band_stride;
        Ok(Box::new((0..band_stride).filter_map(move |i| {
            let v = self.data.get_f64(start + i);
            if self.is_nodata(v) {
                None
            } else {
                let row = i / self.cols;
                let col = i % self.cols;
                Some((row as isize, col as isize, v))
            }
        })))
    }

    /// Iterate over row vectors (`Vec<f64>`) for one band from north to south.
    pub fn iter_band_rows(&self, band: isize) -> Result<Box<dyn Iterator<Item = Vec<f64>> + '_>> {
        if band < 0 || band >= self.bands as isize {
            return Err(RasterError::OutOfBounds {
                band,
                col: 0,
                row: 0,
                bands: self.bands,
                cols: self.cols,
                rows: self.rows,
            });
        }

        Ok(Box::new((0..self.rows).map(move |row| self.row_slice(band, row as isize))))
    }

    /// Traverse mutable native row slices for one band from north to south.
    ///
    /// This is a zero-allocation fast path for in-place row-wise processing.
    /// The callback receives `(row_index, typed_row_slice)`.
    pub fn for_each_band_row_mut<F>(&mut self, band: isize, mut f: F) -> Result<()>
    where
        F: FnMut(isize, RasterRowMut<'_>),
    {
        if band < 0 || band >= self.bands as isize {
            return Err(RasterError::OutOfBounds {
                band,
                col: 0,
                row: 0,
                bands: self.bands,
                cols: self.cols,
                rows: self.rows,
            });
        }

        let b = band as usize;
        let band_stride = self.rows * self.cols;
        let start = b * band_stride;
        let end = start + band_stride;

        match &mut self.data {
            RasterData::U8(v) => {
                for (row, chunk) in v[start..end].chunks_mut(self.cols).enumerate() {
                    f(row as isize, RasterRowMut::U8(chunk));
                }
            }
            RasterData::I8(v) => {
                for (row, chunk) in v[start..end].chunks_mut(self.cols).enumerate() {
                    f(row as isize, RasterRowMut::I8(chunk));
                }
            }
            RasterData::U16(v) => {
                for (row, chunk) in v[start..end].chunks_mut(self.cols).enumerate() {
                    f(row as isize, RasterRowMut::U16(chunk));
                }
            }
            RasterData::I16(v) => {
                for (row, chunk) in v[start..end].chunks_mut(self.cols).enumerate() {
                    f(row as isize, RasterRowMut::I16(chunk));
                }
            }
            RasterData::U32(v) => {
                for (row, chunk) in v[start..end].chunks_mut(self.cols).enumerate() {
                    f(row as isize, RasterRowMut::U32(chunk));
                }
            }
            RasterData::I32(v) => {
                for (row, chunk) in v[start..end].chunks_mut(self.cols).enumerate() {
                    f(row as isize, RasterRowMut::I32(chunk));
                }
            }
            RasterData::U64(v) => {
                for (row, chunk) in v[start..end].chunks_mut(self.cols).enumerate() {
                    f(row as isize, RasterRowMut::U64(chunk));
                }
            }
            RasterData::I64(v) => {
                for (row, chunk) in v[start..end].chunks_mut(self.cols).enumerate() {
                    f(row as isize, RasterRowMut::I64(chunk));
                }
            }
            RasterData::F32(v) => {
                for (row, chunk) in v[start..end].chunks_mut(self.cols).enumerate() {
                    f(row as isize, RasterRowMut::F32(chunk));
                }
            }
            RasterData::F64(v) => {
                for (row, chunk) in v[start..end].chunks_mut(self.cols).enumerate() {
                    f(row as isize, RasterRowMut::F64(chunk));
                }
            }
        }

        Ok(())
    }

    /// Traverse immutable native row slices for one band from north to south.
    ///
    /// This is a zero-allocation fast path for read-only row-wise processing.
    /// The callback receives `(row_index, typed_row_slice)`.
    pub fn for_each_band_row<F>(&self, band: isize, mut f: F) -> Result<()>
    where
        F: FnMut(isize, RasterRowRef<'_>),
    {
        if band < 0 || band >= self.bands as isize {
            return Err(RasterError::OutOfBounds {
                band,
                col: 0,
                row: 0,
                bands: self.bands,
                cols: self.cols,
                rows: self.rows,
            });
        }

        let b = band as usize;
        let band_stride = self.rows * self.cols;
        let start = b * band_stride;
        let end = start + band_stride;

        match &self.data {
            RasterData::U8(v) => {
                for (row, chunk) in v[start..end].chunks(self.cols).enumerate() {
                    f(row as isize, RasterRowRef::U8(chunk));
                }
            }
            RasterData::I8(v) => {
                for (row, chunk) in v[start..end].chunks(self.cols).enumerate() {
                    f(row as isize, RasterRowRef::I8(chunk));
                }
            }
            RasterData::U16(v) => {
                for (row, chunk) in v[start..end].chunks(self.cols).enumerate() {
                    f(row as isize, RasterRowRef::U16(chunk));
                }
            }
            RasterData::I16(v) => {
                for (row, chunk) in v[start..end].chunks(self.cols).enumerate() {
                    f(row as isize, RasterRowRef::I16(chunk));
                }
            }
            RasterData::U32(v) => {
                for (row, chunk) in v[start..end].chunks(self.cols).enumerate() {
                    f(row as isize, RasterRowRef::U32(chunk));
                }
            }
            RasterData::I32(v) => {
                for (row, chunk) in v[start..end].chunks(self.cols).enumerate() {
                    f(row as isize, RasterRowRef::I32(chunk));
                }
            }
            RasterData::U64(v) => {
                for (row, chunk) in v[start..end].chunks(self.cols).enumerate() {
                    f(row as isize, RasterRowRef::U64(chunk));
                }
            }
            RasterData::I64(v) => {
                for (row, chunk) in v[start..end].chunks(self.cols).enumerate() {
                    f(row as isize, RasterRowRef::I64(chunk));
                }
            }
            RasterData::F32(v) => {
                for (row, chunk) in v[start..end].chunks(self.cols).enumerate() {
                    f(row as isize, RasterRowRef::F32(chunk));
                }
            }
            RasterData::F64(v) => {
                for (row, chunk) in v[start..end].chunks(self.cols).enumerate() {
                    f(row as isize, RasterRowRef::F64(chunk));
                }
            }
        }

        Ok(())
    }

    // ─── Transformation ───────────────────────────────────────────────────

    /// Fill all cells with `value`.
    pub fn fill(&mut self, value: f64) {
        for i in 0..self.data.len() {
            self.data.set_f64(i, value);
        }
    }

    /// Fill all cells with the nodata value.
    pub fn fill_nodata(&mut self) {
        let nd = self.nodata;
        self.fill(nd);
    }

    /// Apply a function to every valid (non-nodata) cell value in-place.
    pub fn map_valid<F: Fn(f64) -> f64>(&mut self, f: F) {
        let nd = self.nodata;
        let nodata_nan = nd.is_nan();
        for i in 0..self.data.len() {
            let v = self.data.get_f64(i);
            let is_nd = if nodata_nan { v.is_nan() } else { (v - nd).abs() < 1e-10 * nd.abs().max(1.0) };
            if !is_nd {
                self.data.set_f64(i, f(v));
            }
        }
    }

    /// Apply a function to every valid (non-nodata) cell value in one band in-place.
    pub fn map_valid_band<F: Fn(f64) -> f64>(&mut self, band: isize, f: F) -> Result<()> {
        if band < 0 || band >= self.bands as isize {
            return Err(RasterError::OutOfBounds {
                band,
                col: 0,
                row: 0,
                bands: self.bands,
                cols: self.cols,
                rows: self.rows,
            });
        }

        let nd = self.nodata;
        let nodata_nan = nd.is_nan();
        let band_stride = self.rows * self.cols;
        let start = band as usize * band_stride;
        let end = start + band_stride;

        for i in start..end {
            let v = self.data.get_f64(i);
            let is_nd = if nodata_nan {
                v.is_nan()
            } else {
                (v - nd).abs() < 1e-10 * nd.abs().max(1.0)
            };
            if !is_nd {
                self.data.set_f64(i, f(v));
            }
        }
        Ok(())
    }

    /// Replace all occurrences of `from` with `to` in the data buffer.
    pub fn replace(&mut self, from: f64, to: f64) {
        for i in 0..self.data.len() {
            let v = self.data.get_f64(i);
            if (v - from).abs() < f64::EPSILON {
                self.data.set_f64(i, to);
            }
        }
    }

    /// Replace all occurrences of `from` with `to` in one band.
    pub fn replace_band(&mut self, band: isize, from: f64, to: f64) -> Result<()> {
        if band < 0 || band >= self.bands as isize {
            return Err(RasterError::OutOfBounds {
                band,
                col: 0,
                row: 0,
                bands: self.bands,
                cols: self.cols,
                rows: self.rows,
            });
        }

        let band_stride = self.rows * self.cols;
        let start = band as usize * band_stride;
        let end = start + band_stride;
        for i in start..end {
            let v = self.data.get_f64(i);
            if (v - from).abs() < f64::EPSILON {
                self.data.set_f64(i, to);
            }
        }
        Ok(())
    }

    // ─── I/O ──────────────────────────────────────────────────────────────

    /// Read a raster from `path`, detecting the format automatically.
    pub fn read<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_string_lossy().to_string();
        let fmt = RasterFormat::detect(&path)?;
        fmt.read(&path)
    }

    /// Read a raster from `path` using the specified format.
    pub fn read_with_format<P: AsRef<Path>>(path: P, fmt: RasterFormat) -> Result<Self> {
        let path = path.as_ref().to_string_lossy().to_string();
        fmt.read(&path)
    }

    /// Write this raster to `path`, detecting the format from the extension.
    pub fn write<P: AsRef<Path>>(&self, path: P, fmt: RasterFormat) -> Result<()> {
        let path = path.as_ref().to_string_lossy().to_string();
        fmt.write(self, &path)
    }

    /// Write this raster, auto-detecting the format from the file extension.
    pub fn write_auto<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref().to_string_lossy().to_string();
        let fmt = RasterFormat::detect(&path)?;
        fmt.write(self, &path)
    }

    /// Write this raster as GeoTIFF/BigTIFF/COG using typed options.
    pub fn write_geotiff_with_options<P: AsRef<Path>>(
        &self,
        path: P,
        opts: &crate::formats::geotiff::GeoTiffWriteOptions,
    ) -> Result<()> {
        let path = path.as_ref().to_string_lossy().to_string();
        crate::formats::geotiff::write_with_options(self, &path, opts)
    }

    /// Write this raster as a Cloud-Optimized GeoTIFF (COG) using
    /// convenience defaults.
    ///
    /// Defaults:
    /// - compression: Deflate
    /// - BigTIFF: false
    /// - COG tile size: 512
    pub fn write_cog<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let opts = crate::formats::geotiff::GeoTiffWriteOptions {
            compression: Some(crate::formats::geotiff::GeoTiffCompression::Deflate),
            bigtiff: Some(false),
            layout: Some(crate::formats::geotiff::GeoTiffLayout::Cog { tile_size: 512 }),
        };
        self.write_geotiff_with_options(path, &opts)
    }

    /// Write this raster as a Cloud-Optimized GeoTIFF (COG) using
    /// convenience defaults and a custom tile size.
    ///
    /// Defaults:
    /// - compression: Deflate
    /// - BigTIFF: false
    /// - COG tile size: `tile_size`
    pub fn write_cog_with_tile_size<P: AsRef<Path>>(&self, path: P, tile_size: u32) -> Result<()> {
        let opts = crate::formats::geotiff::GeoTiffWriteOptions {
            compression: Some(crate::formats::geotiff::GeoTiffCompression::Deflate),
            bigtiff: Some(false),
            layout: Some(crate::formats::geotiff::GeoTiffLayout::Cog { tile_size }),
        };
        self.write_geotiff_with_options(path, &opts)
    }

    /// Write this raster as a Cloud-Optimized GeoTIFF (COG) using COG-focused
    /// typed options.
    ///
    /// Any option set to `None` uses convenience defaults:
    /// - compression: Deflate
    /// - BigTIFF: false
    /// - COG tile size: 512
    pub fn write_cog_with_options<P: AsRef<Path>>(
        &self,
        path: P,
        opts: &crate::formats::geotiff::CogWriteOptions,
    ) -> Result<()> {
        let geotiff_opts = crate::formats::geotiff::GeoTiffWriteOptions {
            compression: Some(
                opts.compression
                    .unwrap_or(crate::formats::geotiff::GeoTiffCompression::Deflate),
            ),
            bigtiff: Some(opts.bigtiff.unwrap_or(false)),
            layout: Some(crate::formats::geotiff::GeoTiffLayout::Cog {
                tile_size: opts.tile_size.unwrap_or(512),
            }),
        };
        self.write_geotiff_with_options(path, &geotiff_opts)
    }

    /// Write this raster as JPEG2000/GeoJP2 using typed options.
    pub fn write_jpeg2000_with_options<P: AsRef<Path>>(
        &self,
        path: P,
        opts: &crate::formats::jpeg2000::Jpeg2000WriteOptions,
    ) -> Result<()> {
        let path = path.as_ref().to_string_lossy().to_string();
        crate::formats::jpeg2000::write_with_options(self, &path, opts)
    }
}

fn cubic_bspline_weights(t: f64) -> [f64; 4] {
    let t = t.clamp(0.0, 1.0);
    let t2 = t * t;
    let t3 = t2 * t;
    [
        ((1.0 - t) * (1.0 - t) * (1.0 - t)) / 6.0,
        (3.0 * t3 - 6.0 * t2 + 4.0) / 6.0,
        (-3.0 * t3 + 3.0 * t2 + 3.0 * t + 1.0) / 6.0,
        t3 / 6.0,
    ]
}

fn lanczos_kernel(x: f64, a: f64) -> f64 {
    if x.abs() < 1e-12 {
        return 1.0;
    }
    if x.abs() >= a {
        return 0.0;
    }
    let pix = std::f64::consts::PI * x;
    let pix_over_a = pix / a;
    (pix.sin() / pix) * (pix_over_a.sin() / pix_over_a)
}

fn lanczos3_weights(sample_f: f64, floor_idx: isize) -> [f64; 6] {
    let mut w = [0.0_f64; 6];
    for (i, wi) in w.iter_mut().enumerate() {
        let idx = floor_idx + i as isize - 2;
        let dx = sample_f - idx as f64;
        *wi = lanczos_kernel(dx, 3.0);
    }
    w
}

fn sample_extent_boundary_points(extent: Extent, samples_per_edge: usize) -> Vec<(f64, f64)> {
    let n = samples_per_edge.max(1);
    let mut pts = Vec::with_capacity(4 * n);

    // Bottom and top edges include corners.
    for i in 0..=n {
        let t = i as f64 / n as f64;
        let x = extent.x_min + t * (extent.x_max - extent.x_min);
        pts.push((x, extent.y_min));
        pts.push((x, extent.y_max));
    }

    // Left and right edges exclude corners to avoid duplicate corner points.
    for j in 1..n {
        let t = j as f64 / n as f64;
        let y = extent.y_min + t * (extent.y_max - extent.y_min);
        pts.push((extent.x_min, y));
        pts.push((extent.x_max, y));
    }

    pts
}

fn sample_extent_boundary_ring(extent: Extent, samples_per_edge: usize) -> Vec<(f64, f64)> {
    let n = samples_per_edge.max(1);
    let mut ring = Vec::with_capacity(n * 4);

    for i in 0..n {
        let t = if n == 1 {
            0.0
        } else {
            i as f64 / (n - 1) as f64
        };
        let x = extent.x_min + t * (extent.x_max - extent.x_min);
        ring.push((x, extent.y_min));
    }

    for i in 0..n {
        let t = if n == 1 {
            0.0
        } else {
            i as f64 / (n - 1) as f64
        };
        let y = extent.y_min + t * (extent.y_max - extent.y_min);
        ring.push((extent.x_max, y));
    }

    for i in 0..n {
        let t = if n == 1 {
            0.0
        } else {
            i as f64 / (n - 1) as f64
        };
        let x = extent.x_max - t * (extent.x_max - extent.x_min);
        ring.push((x, extent.y_max));
    }

    for i in 0..n {
        let t = if n == 1 {
            0.0
        } else {
            i as f64 / (n - 1) as f64
        };
        let y = extent.y_max - t * (extent.y_max - extent.y_min);
        ring.push((extent.x_min, y));
    }

    ring
}

fn transformed_extent_from_boundary_samples(
    src_crs: &Crs,
    dst_crs: &Crs,
    src_extent: Extent,
    samples_per_edge: usize,
    dst_epsg: u32,
    antimeridian_policy: AntimeridianPolicy,
) -> Result<Extent> {
    let points = sample_extent_boundary_points(src_extent, samples_per_edge);

    let mut tx_min = f64::INFINITY;
    let mut tx_max = f64::NEG_INFINITY;
    let mut ty_min = f64::INFINITY;
    let mut ty_max = f64::NEG_INFINITY;
    let mut tx_values = Vec::new();
    let mut valid = 0usize;

    for (x, y) in points {
        let Ok((tx, ty)) = src_crs.transform_to(x, y, dst_crs) else {
            continue;
        };
        if !tx.is_finite() || !ty.is_finite() {
            continue;
        }
        tx_min = tx_min.min(tx);
        tx_max = tx_max.max(tx);
        ty_min = ty_min.min(ty);
        ty_max = ty_max.max(ty);
        tx_values.push(tx);
        valid += 1;
    }

    if valid == 0 {
        return Err(RasterError::Other(format!(
            "failed to transform source extent boundary samples to EPSG:{}",
            dst_epsg
        )));
    }

    if dst_epsg == 4326 {
        if let Some((x0, x1)) = antimeridian_aware_longitude_bounds(
            &tx_values,
            antimeridian_policy,
        ) {
            tx_min = x0;
            tx_max = x1;
        }
    }

    Ok(Extent {
        x_min: tx_min,
        y_min: ty_min,
        x_max: tx_max,
        y_max: ty_max,
    })
}

fn transformed_boundary_ring_samples(
    src_crs: &Crs,
    dst_crs: &Crs,
    src_extent: Extent,
    samples_per_edge: usize,
    dst_epsg: u32,
    antimeridian_policy: AntimeridianPolicy,
) -> Result<Vec<(f64, f64)>> {
    let ring = sample_extent_boundary_ring(src_extent, samples_per_edge);
    let mut transformed = Vec::with_capacity(ring.len());

    for (x, y) in ring {
        let Ok((tx, ty)) = src_crs.transform_to(x, y, dst_crs) else {
            continue;
        };
        if tx.is_finite() && ty.is_finite() {
            transformed.push((tx, ty));
        }
    }

    if transformed.len() < 3 {
        return Err(RasterError::Other(format!(
            "failed to build transformed boundary ring for EPSG:{}",
            dst_epsg
        )));
    }

    if dst_epsg == 4326 && antimeridian_policy != AntimeridianPolicy::Linear {
        let lons: Vec<f64> = transformed.iter().map(|(x, _)| *x).collect();
        if let Some((base, _)) = minimal_wrapped_longitude_bounds(&lons) {
            for (x, _) in &mut transformed {
                let mut w = wrap_lon_360(*x);
                if w < base {
                    w += 360.0;
                }
                *x = w;
            }
        }
    }

    Ok(transformed)
}

fn snap_down_to_origin(value: f64, origin: f64, step: f64) -> f64 {
    origin + ((value - origin) / step).floor() * step
}

fn snap_up_to_origin(value: f64, origin: f64, step: f64) -> f64 {
    origin + ((value - origin) / step).ceil() * step
}

fn wrap_lon_360(lon: f64) -> f64 {
    let mut v = lon % 360.0;
    if v < 0.0 {
        v += 360.0;
    }
    v
}

fn minimal_wrapped_longitude_bounds(longitudes: &[f64]) -> Option<(f64, f64)> {
    if longitudes.is_empty() {
        return None;
    }

    if longitudes.len() == 1 {
        let v = wrap_lon_360(longitudes[0]);
        return Some((v, v));
    }

    let mut values: Vec<f64> = longitudes.iter().map(|v| wrap_lon_360(*v)).collect();
    values.sort_by(f64::total_cmp);

    let n = values.len();
    let mut max_gap = f64::NEG_INFINITY;
    let mut max_gap_idx = 0usize;
    for i in 0..n {
        let next = if i + 1 < n {
            values[i + 1]
        } else {
            values[0] + 360.0
        };
        let gap = next - values[i];
        if gap > max_gap {
            max_gap = gap;
            max_gap_idx = i;
        }
    }

    let start = values[(max_gap_idx + 1) % n];
    let mut end = values[max_gap_idx];
    if end < start {
        end += 360.0;
    }

    Some((start, end))
}

fn antimeridian_aware_longitude_bounds(
    longitudes: &[f64],
    policy: AntimeridianPolicy,
) -> Option<(f64, f64)> {
    if longitudes.is_empty() {
        return None;
    }

    let mut lin_min = f64::INFINITY;
    let mut lin_max = f64::NEG_INFINITY;
    for lon in longitudes {
        if !lon.is_finite() {
            continue;
        }
        lin_min = lin_min.min(*lon);
        lin_max = lin_max.max(*lon);
    }
    if !lin_min.is_finite() || !lin_max.is_finite() {
        return None;
    }

    if policy == AntimeridianPolicy::Linear {
        return Some((lin_min, lin_max));
    }

    let Some((wrap_min, wrap_max)) = minimal_wrapped_longitude_bounds(longitudes) else {
        return Some((lin_min, lin_max));
    };
    if policy == AntimeridianPolicy::Wrap {
        return Some((wrap_min, wrap_max));
    }

    let linear_width = lin_max - lin_min;
    let wrapped_width = wrap_max - wrap_min;

    if wrapped_width + 1e-9 < linear_width {
        Some((wrap_min, wrap_max))
    } else {
        Some((lin_min, lin_max))
    }
}

fn clamp_isize(v: isize, min_v: isize, max_v: isize) -> isize {
    v.max(min_v).min(max_v)
}

fn point_in_polygon(x: f64, y: f64, polygon: &[(f64, f64)]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = polygon.len() - 1;
    for i in 0..polygon.len() {
        let (xi, yi) = polygon[i];
        let (xj, yj) = polygon[j];
        let intersects = (yi > y) != (yj > y)
            && x < (xj - xi) * (y - yi) / ((yj - yi) + 1e-30) + xi;
        if intersects {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowStat {
    Mean,
    Min,
    Max,
    Mode,
    Median,
    StdDev,
}

fn reduce_window_values(values: &[f64], stat: WindowStat) -> Option<f64> {
    if values.is_empty() {
        return None;
    }

    match stat {
        WindowStat::Mean => Some(values.iter().sum::<f64>() / values.len() as f64),
        WindowStat::Min => values.iter().copied().reduce(f64::min),
        WindowStat::Max => values.iter().copied().reduce(f64::max),
        WindowStat::Mode => {
            let mut pairs: Vec<(f64, usize)> = Vec::new();
            for v in values {
                if let Some((_, count)) = pairs.iter_mut().find(|(u, _)| *u == *v) {
                    *count += 1;
                } else {
                    pairs.push((*v, 1));
                }
            }
            pairs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.total_cmp(&b.0)));
            pairs.first().map(|(v, _)| *v)
        }
        WindowStat::Median => {
            let mut sorted = values.to_vec();
            sorted.sort_by(f64::total_cmp);
            let n = sorted.len();
            if n % 2 == 1 {
                Some(sorted[n / 2])
            } else {
                Some((sorted[n / 2 - 1] + sorted[n / 2]) / 2.0)
            }
        }
        WindowStat::StdDev => {
            let n = values.len() as f64;
            let mean = values.iter().sum::<f64>() / n;
            let variance = values
                .iter()
                .map(|v| {
                    let d = *v - mean;
                    d * d
                })
                .sum::<f64>()
                / n;
            Some(variance.max(0.0).sqrt())
        }
    }
}

impl std::fmt::Display for Raster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Raster({}×{}×{}, cell={:.6}, x=[{:.4},{:.4}], y=[{:.4},{:.4}], type={}, nodata={})",
            self.bands,
            self.cols,
            self.rows,
            self.cell_size_x,
            self.x_min,
            self.x_max(),
            self.y_min,
            self.y_max(),
            self.data_type,
            self.nodata,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn make_raster() -> Raster {
        let cfg = RasterConfig { cols: 4, rows: 3, cell_size: 10.0, nodata: -9999.0, ..Default::default() };
        let mut r = Raster::new(cfg);
        for row in 0..3 {
            for col in 0..4 {
                let _ = r.set(0, row, col, (row * 4 + col) as f64);
            }
        }
        r
    }

    #[test]
    fn get_set() {
        let mut r = make_raster();
        assert_eq!(r.get(0, 0, 0), 0.0);
        assert_eq!(r.get(0, 2, 3), 11.0);
        r.set(0, 1, 1, -9999.0).unwrap();
        assert!(r.is_nodata(r.get(0, 1, 1))); // nodata
        assert_eq!(r.get_opt(0, 1, 1), None); // optional accessor
    }

    #[test]
    fn statistics() {
        let r = make_raster();
        let s = r.statistics();
        assert_eq!(s.valid_count, 12);
        assert_eq!(s.min, 0.0);
        assert_eq!(s.max, 11.0);
        assert!((s.mean - 5.5).abs() < 1e-10);
    }

    #[test]
    fn world_to_pixel() {
        let r = make_raster();
        // y_max = 30.0, x_max = 40.0
        assert_eq!(r.world_to_pixel(5.0, 25.0), Some((0, 0)));
        assert_eq!(r.world_to_pixel(35.0, 5.0), Some((3, 2)));
        assert_eq!(r.world_to_pixel(-1.0, 0.0), None);
    }

    #[test]
    fn extent() {
        let r = make_raster();
        let e = r.extent();
        assert_eq!(e.x_min, 0.0);
        assert_eq!(e.y_min, 0.0);
        assert_eq!(e.x_max, 40.0);
        assert_eq!(e.y_max, 30.0);
    }

    #[test]
    fn sample_extent_boundary_points_count_and_corners() {
        let e = Extent {
            x_min: 0.0,
            y_min: 0.0,
            x_max: 10.0,
            y_max: 5.0,
        };
        let pts = sample_extent_boundary_points(e, 8);
        assert_eq!(pts.len(), 32);
        assert!(pts.contains(&(0.0, 0.0)));
        assert!(pts.contains(&(0.0, 5.0)));
        assert!(pts.contains(&(10.0, 0.0)));
        assert!(pts.contains(&(10.0, 5.0)));
    }

    #[test]
    fn sample_extent_boundary_points_minimum_sampling_when_zero_requested() {
        let e = Extent {
            x_min: -1.0,
            y_min: -2.0,
            x_max: 3.0,
            y_max: 4.0,
        };
        let pts = sample_extent_boundary_points(e, 0);
        assert_eq!(pts.len(), 4);
        assert!(pts.contains(&(-1.0, -2.0)));
        assert!(pts.contains(&(-1.0, 4.0)));
        assert!(pts.contains(&(3.0, -2.0)));
        assert!(pts.contains(&(3.0, 4.0)));
    }

    #[test]
    fn minimal_wrapped_longitude_bounds_handles_antimeridian_cluster() {
        let lons = [179.0, -179.0, 178.0, -178.0];
        let (x0, x1) = minimal_wrapped_longitude_bounds(&lons).unwrap();
        assert!((x1 - x0) < 6.0);
    }

    #[test]
    fn antimeridian_aware_longitude_bounds_prefers_wrapped_interval() {
        let lons = [179.5, -179.5, 179.0, -179.0];
        let (x0, x1) =
            antimeridian_aware_longitude_bounds(&lons, AntimeridianPolicy::Auto).unwrap();
        assert!((x1 - x0) < 5.0);
        assert!(x0 >= 0.0);
    }

    #[test]
    fn antimeridian_aware_longitude_bounds_keeps_linear_when_better() {
        let lons = [-20.0, -10.0, 0.0, 10.0];
        let (x0, x1) =
            antimeridian_aware_longitude_bounds(&lons, AntimeridianPolicy::Auto).unwrap();
        assert!((x0 - (-20.0)).abs() < 1e-12);
        assert!((x1 - 10.0).abs() < 1e-12);
    }

    #[test]
    fn antimeridian_policy_controls_wrapped_vs_linear_behavior() {
        let lons = [179.5, -179.5, 179.0, -179.0];

        let (ax0, ax1) =
            antimeridian_aware_longitude_bounds(&lons, AntimeridianPolicy::Auto).unwrap();
        let (lx0, lx1) =
            antimeridian_aware_longitude_bounds(&lons, AntimeridianPolicy::Linear).unwrap();
        let (wx0, wx1) =
            antimeridian_aware_longitude_bounds(&lons, AntimeridianPolicy::Wrap).unwrap();

        assert!((ax1 - ax0) < (lx1 - lx0));
        assert!((wx1 - wx0) < (lx1 - lx0));
        assert!((wx1 - wx0 - (ax1 - ax0)).abs() < 1e-9);
    }

    #[test]
    fn reproject_antimeridian_policy_linear_vs_wrap_changes_default_extent() {
        let cfg = RasterConfig {
            cols: 8,
            rows: 4,
            x_min: 170.0,
            y_min: -10.0,
            cell_size: 2.5,
            nodata: -9999.0,
            ..Default::default()
        };
        let mut r = Raster::new(cfg);
        r.crs = CrsInfo::from_epsg(4326);
        for row in 0..r.rows {
            for col in 0..r.cols {
                r.set(0, row as isize, col as isize, (row * r.cols + col) as f64)
                    .unwrap();
            }
        }

        let linear_opts = ReprojectOptions::new(4326, ResampleMethod::Nearest)
            .with_antimeridian_policy(AntimeridianPolicy::Linear);
        let wrap_opts = ReprojectOptions::new(4326, ResampleMethod::Nearest)
            .with_antimeridian_policy(AntimeridianPolicy::Wrap);

        let linear = r.reproject_with_options(&linear_opts).unwrap();
        let wrap = r.reproject_with_options(&wrap_opts).unwrap();

        let linear_width = linear.x_max() - linear.x_min;
        let wrap_width = wrap.x_max() - wrap.x_min;

        assert!(linear_width.is_finite() && linear_width > 0.0);
        assert!(wrap_width.is_finite() && wrap_width > 0.0);
        assert!(wrap_width <= linear_width + 1e-9);
    }

    #[test]
    fn reproject_antimeridian_policy_auto_matches_narrower_interval() {
        let cfg = RasterConfig {
            cols: 8,
            rows: 4,
            x_min: 170.0,
            y_min: -10.0,
            cell_size: 2.5,
            nodata: -9999.0,
            ..Default::default()
        };
        let mut r = Raster::new(cfg);
        r.crs = CrsInfo::from_epsg(4326);
        for row in 0..r.rows {
            for col in 0..r.cols {
                r.set(0, row as isize, col as isize, (row * r.cols + col) as f64)
                    .unwrap();
            }
        }

        let linear = r
            .reproject_with_options(
                &ReprojectOptions::new(4326, ResampleMethod::Nearest)
                    .with_antimeridian_policy(AntimeridianPolicy::Linear),
            )
            .unwrap();
        let wrap = r
            .reproject_with_options(
                &ReprojectOptions::new(4326, ResampleMethod::Nearest)
                    .with_antimeridian_policy(AntimeridianPolicy::Wrap),
            )
            .unwrap();
        let auto = r
            .reproject_with_options(
                &ReprojectOptions::new(4326, ResampleMethod::Nearest)
                    .with_antimeridian_policy(AntimeridianPolicy::Auto),
            )
            .unwrap();

        let linear_width = linear.x_max() - linear.x_min;
        let wrap_width = wrap.x_max() - wrap.x_min;
        let auto_width = auto.x_max() - auto.x_min;

        let expected = linear_width.min(wrap_width);
        assert!((auto_width - expected).abs() < 1e-9);
    }

    #[test]
    fn reproject_to_epsg_requires_source_epsg() {
        let r = make_raster();
        let err = r.reproject_to_epsg_nearest(3857).unwrap_err();
        assert!(err
            .to_string()
            .contains("requires source CRS EPSG in raster.crs.epsg"));
    }

    #[test]
    fn reproject_with_crs_allows_missing_source_epsg() {
        let cfg = RasterConfig {
            cols: 4,
            rows: 4,
            x_min: -1.0,
            y_min: -1.0,
            cell_size: 0.5,
            nodata: -9999.0,
            ..Default::default()
        };
        let mut r = Raster::new(cfg);
        // Intentionally leave `r.crs.epsg` unset.
        for row in 0..4 {
            for col in 0..4 {
                r.set(0, row, col, (row * 4 + col) as f64).unwrap();
            }
        }

        let src = Crs::from_epsg(4326).unwrap();
        let dst = Crs::from_epsg(3857).unwrap();
        let opts = ReprojectOptions::new(3857, ResampleMethod::Nearest);

        let out = r.reproject_with_crs(&src, &dst, &opts).unwrap();
        assert_eq!(out.cols, 4);
        assert_eq!(out.rows, 4);
        assert_eq!(out.crs.epsg, Some(3857));
        assert!(out.statistics().valid_count > 0);
    }

    #[test]
    fn reproject_with_options_and_progress_emits_live_updates() {
        let cfg = RasterConfig {
            cols: 6,
            rows: 5,
            x_min: -80.0,
            y_min: 40.0,
            cell_size: 0.1,
            nodata: -9999.0,
            ..Default::default()
        };
        let mut r = Raster::new(cfg);
        r.crs = CrsInfo::from_epsg(4326);
        for row in 0..r.rows {
            for col in 0..r.cols {
                r.set(0, row as isize, col as isize, (row * r.cols + col) as f64)
                    .unwrap();
            }
        }

        let progress_values: Arc<Mutex<Vec<f64>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&progress_values);

        let out = r
            .reproject_with_options_and_progress(
                &ReprojectOptions::new(3857, ResampleMethod::Nearest),
                move |pct| {
                    sink.lock().unwrap().push(pct);
                },
            )
            .unwrap();

        let values = progress_values.lock().unwrap();
        assert!(!values.is_empty());
        assert_eq!(values.len(), out.rows + 1);
        assert!(values.iter().all(|v| v.is_finite() && *v >= 0.0 && *v <= 1.0));
        assert!((values.last().copied().unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn reproject_to_epsg_identity_preserves_data() {
        let mut r = make_raster();
        r.crs = CrsInfo::from_epsg(4326);

        let r2 = r.reproject_to_epsg(4326, ResampleMethod::Nearest).unwrap();
        assert_eq!(r2.cols, r.cols);
        assert_eq!(r2.rows, r.rows);
        assert_eq!(r2.bands, r.bands);
        assert_eq!(r2.crs.epsg, Some(4326));
        assert_eq!(r2.get(0, 0, 0), r.get(0, 0, 0));
        assert_eq!(r2.get(0, 2, 3), r.get(0, 2, 3));
    }

    #[test]
    fn reproject_to_epsg_4326_to_3857_produces_valid_output() {
        let cfg = RasterConfig {
            cols: 4,
            rows: 4,
            x_min: -1.0,
            y_min: -1.0,
            cell_size: 0.5,
            nodata: -9999.0,
            ..Default::default()
        };
        let mut r = Raster::new(cfg);
        r.crs = CrsInfo::from_epsg(4326);
        for row in 0..4 {
            for col in 0..4 {
                r.set(0, row, col, (row * 4 + col) as f64).unwrap();
            }
        }

        let out = r.reproject_to_epsg_nearest(3857).unwrap();
        assert_eq!(out.cols, 4);
        assert_eq!(out.rows, 4);
        assert_eq!(out.crs.epsg, Some(3857));
        assert!(out.x_min.is_finite());
        assert!(out.y_min.is_finite());
        assert!(out.cell_size_x.is_finite() && out.cell_size_x > 0.0);
        assert!(out.cell_size_y.is_finite() && out.cell_size_y > 0.0);
        let s = out.statistics();
        assert!(s.valid_count > 0);
    }

    #[test]
    fn reproject_to_match_grid_honors_target_grid_definition() {
        let src_cfg = RasterConfig {
            cols: 6,
            rows: 4,
            x_min: -3.0,
            y_min: -2.0,
            cell_size: 1.0,
            nodata: -9999.0,
            ..Default::default()
        };
        let mut src = Raster::new(src_cfg);
        src.crs = CrsInfo::from_epsg(4326);
        for row in 0..src.rows {
            for col in 0..src.cols {
                src.set(0, row as isize, col as isize, (row * src.cols + col) as f64)
                    .unwrap();
            }
        }

        let target_cfg = RasterConfig {
            cols: 12,
            rows: 10,
            x_min: -500_000.0,
            y_min: -400_000.0,
            cell_size: 1.0,
            cell_size_y: Some(1.0),
            nodata: -9999.0,
            ..Default::default()
        };
        let mut target = Raster::new(target_cfg);
        target.crs = CrsInfo::from_epsg(3857);
        target.cell_size_x = (500_000.0 - (-500_000.0)) / target.cols as f64;
        target.cell_size_y = (400_000.0 - (-400_000.0)) / target.rows as f64;

        let out = src
            .reproject_to_match_grid(&target, ResampleMethod::Bilinear)
            .unwrap();

        assert_eq!(out.cols, target.cols);
        assert_eq!(out.rows, target.rows);
        assert_eq!(out.crs.epsg, target.crs.epsg);
        assert!((out.x_min - target.x_min).abs() < 1e-9);
        assert!((out.y_min - target.y_min).abs() < 1e-9);
        assert!((out.x_max() - target.x_max()).abs() < 1e-6);
        assert!((out.y_max() - target.y_max()).abs() < 1e-6);
        assert!(out.cell_size_x.is_finite() && out.cell_size_x > 0.0);
        assert!(out.cell_size_y.is_finite() && out.cell_size_y > 0.0);
    }

    #[test]
    fn reproject_to_match_resolution_honors_reference_cellsize_and_snap() {
        let src_cfg = RasterConfig {
            cols: 6,
            rows: 4,
            x_min: -3.0,
            y_min: -2.0,
            cell_size: 1.0,
            nodata: -9999.0,
            ..Default::default()
        };
        let mut src = Raster::new(src_cfg);
        src.crs = CrsInfo::from_epsg(4326);
        for row in 0..src.rows {
            for col in 0..src.cols {
                src.set(0, row as isize, col as isize, (row * src.cols + col) as f64)
                    .unwrap();
            }
        }

        let mut reference = Raster::new(RasterConfig {
            cols: 4,
            rows: 3,
            x_min: 50_000.0,
            y_min: -75_000.0,
            cell_size: 100_000.0,
            cell_size_y: Some(80_000.0),
            nodata: -9999.0,
            ..Default::default()
        });
        reference.crs = CrsInfo::from_epsg(3857);

        let out = src
            .reproject_to_match_resolution(&reference, ResampleMethod::Nearest)
            .unwrap();

        assert_eq!(out.crs.epsg, Some(3857));
        assert!((out.cell_size_x - reference.cell_size_x).abs() < 1e-9);
        assert!((out.cell_size_y - reference.cell_size_y).abs() < 1e-9);

        let kx = ((out.x_min - reference.x_min) / reference.cell_size_x).round();
        let ky = ((out.y_min - reference.y_min) / reference.cell_size_y).round();
        assert!((out.x_min - (reference.x_min + kx * reference.cell_size_x)).abs() < 1e-6);
        assert!((out.y_min - (reference.y_min + ky * reference.cell_size_y)).abs() < 1e-6);
        assert!(out.cols > 0);
        assert!(out.rows > 0);
    }

    #[test]
    fn reproject_to_match_resolution_in_epsg_same_crs_matches_reference_settings() {
        let src_cfg = RasterConfig {
            cols: 6,
            rows: 4,
            x_min: -3.0,
            y_min: -2.0,
            cell_size: 1.0,
            nodata: -9999.0,
            ..Default::default()
        };
        let mut src = Raster::new(src_cfg);
        src.crs = CrsInfo::from_epsg(4326);
        for row in 0..src.rows {
            for col in 0..src.cols {
                src.set(0, row as isize, col as isize, (row * src.cols + col) as f64)
                    .unwrap();
            }
        }

        let mut reference = Raster::new(RasterConfig {
            cols: 4,
            rows: 3,
            x_min: -10.0,
            y_min: -20.0,
            cell_size: 0.5,
            cell_size_y: Some(0.25),
            nodata: -9999.0,
            ..Default::default()
        });
        reference.crs = CrsInfo::from_epsg(4326);

        let out = src
            .reproject_to_match_resolution_in_epsg(4326, &reference, ResampleMethod::Nearest)
            .unwrap();

        assert_eq!(out.crs.epsg, Some(4326));
        assert!((out.cell_size_x - reference.cell_size_x).abs() < 1e-12);
        assert!((out.cell_size_y - reference.cell_size_y).abs() < 1e-12);
    }

    #[test]
    fn reproject_to_match_resolution_in_epsg_cross_crs_converts_resolution() {
        let src_cfg = RasterConfig {
            cols: 6,
            rows: 4,
            x_min: -3.0,
            y_min: -2.0,
            cell_size: 1.0,
            nodata: -9999.0,
            ..Default::default()
        };
        let mut src = Raster::new(src_cfg);
        src.crs = CrsInfo::from_epsg(4326);
        for row in 0..src.rows {
            for col in 0..src.cols {
                src.set(0, row as isize, col as isize, (row * src.cols + col) as f64)
                    .unwrap();
            }
        }

        let mut reference = Raster::new(RasterConfig {
            cols: 4,
            rows: 3,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            cell_size_y: Some(1.0),
            nodata: -9999.0,
            ..Default::default()
        });
        reference.crs = CrsInfo::from_epsg(4326);

        let out = src
            .reproject_to_match_resolution_in_epsg(3857, &reference, ResampleMethod::Nearest)
            .unwrap();

        assert_eq!(out.crs.epsg, Some(3857));
        assert!(out.cell_size_x.is_finite());
        assert!(out.cell_size_y.is_finite());
        assert!(out.cell_size_x > 0.0);
        assert!(out.cell_size_y > 0.0);
    }

    #[test]
    fn reproject_to_epsg_bilinear_cubic_and_lanczos_produce_valid_output() {
        let cfg = RasterConfig {
            cols: 8,
            rows: 8,
            x_min: -2.0,
            y_min: -2.0,
            cell_size: 0.5,
            nodata: -9999.0,
            ..Default::default()
        };
        let mut r = Raster::new(cfg);
        r.crs = CrsInfo::from_epsg(4326);
        for row in 0..8 {
            for col in 0..8 {
                let val = (col as f64) * 10.0 + row as f64;
                r.set(0, row, col, val).unwrap();
            }
        }

        let bilinear = r.reproject_to_epsg_bilinear(3857).unwrap();
        let cubic = r.reproject_to_epsg_cubic(3857).unwrap();
        let lanczos = r.reproject_to_epsg_lanczos(3857).unwrap();

        assert_eq!(bilinear.cols, 8);
        assert_eq!(bilinear.rows, 8);
        assert_eq!(bilinear.crs.epsg, Some(3857));
        assert!(bilinear.statistics().valid_count > 0);

        assert_eq!(cubic.cols, 8);
        assert_eq!(cubic.rows, 8);
        assert_eq!(cubic.crs.epsg, Some(3857));
        assert!(cubic.statistics().valid_count > 0);

        assert_eq!(lanczos.cols, 8);
        assert_eq!(lanczos.rows, 8);
        assert_eq!(lanczos.crs.epsg, Some(3857));
        assert!(lanczos.statistics().valid_count > 0);
    }

    #[test]
    fn reproject_with_options_honors_grid_controls() {
        let cfg = RasterConfig {
            cols: 6,
            rows: 4,
            x_min: -3.0,
            y_min: -2.0,
            cell_size: 1.0,
            nodata: -9999.0,
            ..Default::default()
        };
        let mut r = Raster::new(cfg);
        r.crs = CrsInfo::from_epsg(4326);
        for row in 0..4 {
            for col in 0..6 {
                r.set(0, row, col, (row * 6 + col) as f64).unwrap();
            }
        }

        let opts = ReprojectOptions {
            dst_epsg: 3857,
            resample: ResampleMethod::Bilinear,
            cols: Some(12),
            rows: Some(10),
            extent: Some(Extent {
                x_min: -500_000.0,
                y_min: -400_000.0,
                x_max: 500_000.0,
                y_max: 400_000.0,
            }),
            x_res: None,
            y_res: None,
            snap_x: None,
            snap_y: None,
            nodata_policy: NodataPolicy::PartialKernel,
            antimeridian_policy: AntimeridianPolicy::Auto,
            grid_size_policy: GridSizePolicy::Expand,
            destination_footprint: DestinationFootprint::None,
        };

        let out = r.reproject_with_options(&opts).unwrap();
        assert_eq!(out.cols, 12);
        assert_eq!(out.rows, 10);
        assert!((out.x_min - (-500_000.0)).abs() < 1e-9);
        assert!((out.y_min - (-400_000.0)).abs() < 1e-9);
        assert!((out.x_max() - 500_000.0).abs() < 1e-6);
        assert!((out.y_max() - 400_000.0).abs() < 1e-6);
        assert_eq!(out.crs.epsg, Some(3857));
    }

    #[test]
    fn bilinear_partial_kernel_renormalizes_with_nodata() {
        let cfg = RasterConfig {
            cols: 2,
            rows: 2,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            ..Default::default()
        };
        // row-major top-down: [ [1, nodata], [3, 5] ]
        let r = Raster::from_data(cfg, vec![1.0, -9999.0, 3.0, 5.0]).unwrap();

        let v = r.sample_bilinear_partial_pixel(0, 0.5, 0.5).unwrap();
        // weighted average of valid neighbors only: (1 + 3 + 5) / 3 = 3
        assert!((v - 3.0).abs() < 1e-9);
    }

    #[test]
    fn bilinear_partial_kernel_handles_edges() {
        let cfg = RasterConfig {
            cols: 2,
            rows: 2,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            ..Default::default()
        };
        let r = Raster::from_data(cfg, vec![10.0, 20.0, 30.0, 40.0]).unwrap();
        let v = r.sample_bilinear_partial_pixel(0, -0.2, 0.3).unwrap();
        assert!(v.is_finite());
    }

    #[test]
    fn cubic_partial_kernel_handles_edges_and_nodata() {
        let cfg = RasterConfig {
            cols: 4,
            rows: 4,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            ..Default::default()
        };
        let mut data = Vec::new();
        for row in 0..4 {
            for col in 0..4 {
                data.push((row * 4 + col) as f64);
            }
        }
        data[5] = -9999.0; // inject one nodata sample
        let r = Raster::from_data(cfg, data).unwrap();

        let edge_v = r.sample_cubic_partial_pixel(0, -0.25, 0.2).unwrap();
        assert!(edge_v.is_finite());

        let nodata_v = r.sample_cubic_partial_pixel(0, 1.25, 1.25).unwrap();
        assert!(nodata_v.is_finite());
    }

    #[test]
    fn strict_policy_rejects_incomplete_bilinear_kernel() {
        let cfg = RasterConfig {
            cols: 2,
            rows: 2,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            ..Default::default()
        };
        let r = Raster::from_data(cfg, vec![10.0, 20.0, 30.0, 40.0]).unwrap();

        assert!(r.sample_bilinear_strict_pixel(0, -0.2, 0.3).is_none());
        assert!(r.sample_bilinear_partial_pixel(0, -0.2, 0.3).is_some());
    }

    #[test]
    fn bilinear_strict_simd_matches_expected_for_f64_and_f32_storage() {
        let cfg_f64 = RasterConfig {
            cols: 2,
            rows: 2,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F64,
            ..Default::default()
        };
        let cfg_f32 = RasterConfig {
            data_type: DataType::F32,
            ..cfg_f64.clone()
        };
        let data = vec![1.0, 2.0, 3.0, 5.0];
        let r64 = Raster::from_data(cfg_f64, data.clone()).unwrap();
        let r32 = Raster::from_data(cfg_f32, data).unwrap();

        let expected = 2.375;
        let v64 = r64.sample_bilinear_strict_pixel(0, 0.25, 0.5).unwrap();
        let v32 = r32.sample_bilinear_strict_pixel(0, 0.25, 0.5).unwrap();
        assert!((v64 - expected).abs() < 1e-9);
        assert!((v32 - expected).abs() < 1e-6);
    }

    #[test]
    fn lanczos_strict_simd_matches_between_f64_and_f32_storage() {
        let cfg_f64 = RasterConfig {
            cols: 16,
            rows: 16,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F64,
            ..Default::default()
        };
        let cfg_f32 = RasterConfig {
            data_type: DataType::F32,
            ..cfg_f64.clone()
        };

        let mut data = Vec::with_capacity(16 * 16);
        for row in 0..16 {
            for col in 0..16 {
                data.push(((row * 16 + col) as f64).sin() * 100.0 + (row as f64 * 0.5));
            }
        }

        let r64 = Raster::from_data(cfg_f64, data.clone()).unwrap();
        let r32 = Raster::from_data(cfg_f32, data).unwrap();

        let v64 = r64.sample_lanczos_strict_pixel(0, 7.25, 8.5).unwrap();
        let v32 = r32.sample_lanczos_strict_pixel(0, 7.25, 8.5).unwrap();
        assert!((v64 - v32).abs() < 1e-4);
    }

    #[test]
    fn fill_policy_falls_back_to_nearest_for_bilinear() {
        let cfg = RasterConfig {
            cols: 2,
            rows: 2,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            ..Default::default()
        };
        let r = Raster::from_data(cfg, vec![10.0, 20.0, 30.0, 40.0]).unwrap();

        let v = r.sample_world(0, 0.1, 1.1, ResampleMethod::Bilinear, NodataPolicy::Fill);
        assert_eq!(v, Some(10.0));
    }

    #[test]
    fn fill_policy_falls_back_to_nearest_for_lanczos() {
        let cfg = RasterConfig {
            cols: 2,
            rows: 2,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            ..Default::default()
        };
        let r = Raster::from_data(cfg, vec![10.0, 20.0, 30.0, 40.0]).unwrap();

        let v = r.sample_world(0, 0.1, 1.1, ResampleMethod::Lanczos, NodataPolicy::Fill);
        assert_eq!(v, Some(10.0));
    }

    #[test]
    fn reproject_options_default_to_partial_kernel_policy() {
        let opts = ReprojectOptions::new(3857, ResampleMethod::Cubic);
        assert_eq!(opts.nodata_policy, NodataPolicy::PartialKernel);
        assert_eq!(opts.x_res, None);
        assert_eq!(opts.y_res, None);
        assert_eq!(opts.snap_x, None);
        assert_eq!(opts.snap_y, None);
        assert_eq!(opts.antimeridian_policy, AntimeridianPolicy::Auto);
        assert_eq!(opts.grid_size_policy, GridSizePolicy::Expand);
        assert_eq!(opts.destination_footprint, DestinationFootprint::None);

        let strict = opts.with_nodata_policy(NodataPolicy::Strict);
        assert_eq!(strict.nodata_policy, NodataPolicy::Strict);

        let sized = strict.with_size(321, 123);
        assert_eq!(sized.cols, Some(321));
        assert_eq!(sized.rows, Some(123));

        let e = Extent {
            x_min: -10.0,
            y_min: -5.0,
            x_max: 10.0,
            y_max: 5.0,
        };
        let ext = sized.with_extent(e);
        assert_eq!(ext.extent, Some(e));

        let res = ext.with_resolution(30.0, 40.0);
        assert_eq!(res.x_res, Some(30.0));
        assert_eq!(res.y_res, Some(40.0));

        let square = res.with_square_resolution(25.0);
        assert_eq!(square.x_res, Some(25.0));
        assert_eq!(square.y_res, Some(25.0));

        let snapped = square.with_snap_origin(0.0, 0.0);
        assert_eq!(snapped.snap_x, Some(0.0));
        assert_eq!(snapped.snap_y, Some(0.0));

        let linear = snapped.with_antimeridian_policy(AntimeridianPolicy::Linear);
        assert_eq!(linear.antimeridian_policy, AntimeridianPolicy::Linear);

        let fit_inside = linear.with_grid_size_policy(GridSizePolicy::FitInside);
        assert_eq!(fit_inside.grid_size_policy, GridSizePolicy::FitInside);

        let masked = fit_inside.with_destination_footprint(DestinationFootprint::SourceBoundary);
        assert_eq!(masked.destination_footprint, DestinationFootprint::SourceBoundary);
    }

    #[test]
    fn sample_extent_boundary_ring_count_and_corners() {
        let e = Extent {
            x_min: 0.0,
            y_min: 0.0,
            x_max: 10.0,
            y_max: 5.0,
        };
        let ring = sample_extent_boundary_ring(e, 8);
        assert_eq!(ring.len(), 32);
        assert!(ring.contains(&(0.0, 0.0)));
        assert!(ring.contains(&(10.0, 0.0)));
        assert!(ring.contains(&(10.0, 5.0)));
        assert!(ring.contains(&(0.0, 5.0)));
    }

    #[test]
    fn point_in_polygon_identifies_inside_and_outside_points() {
        let poly = vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)];
        assert!(point_in_polygon(2.0, 2.0, &poly));
        assert!(!point_in_polygon(5.0, 2.0, &poly));
    }

    #[test]
    fn thematic_3x3_resamplers_return_expected_statistics() {
        let cfg = RasterConfig {
            cols: 3,
            rows: 3,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            ..Default::default()
        };
        let data = vec![
            1.0, 2.0, 2.0,
            3.0, 4.0, 4.0,
            5.0, 6.0, 6.0,
        ];
        let expected_mean = data.iter().sum::<f64>() / data.len() as f64;
        let expected_stddev = (data
            .iter()
            .map(|v| {
                let d = *v - expected_mean;
                d * d
            })
            .sum::<f64>()
            / data.len() as f64)
            .sqrt();
        let r = Raster::from_data(cfg, data).unwrap();

        let x = r.col_center_x(1);
        let y = r.row_center_y(1);

        let avg = r
            .sample_world(0, x, y, ResampleMethod::Average, NodataPolicy::Strict)
            .unwrap();
        let min = r
            .sample_world(0, x, y, ResampleMethod::Min, NodataPolicy::Strict)
            .unwrap();
        let max = r
            .sample_world(0, x, y, ResampleMethod::Max, NodataPolicy::Strict)
            .unwrap();
        let mode = r
            .sample_world(0, x, y, ResampleMethod::Mode, NodataPolicy::Strict)
            .unwrap();
        let median = r
            .sample_world(0, x, y, ResampleMethod::Median, NodataPolicy::Strict)
            .unwrap();
        let stddev = r
            .sample_world(0, x, y, ResampleMethod::StdDev, NodataPolicy::Strict)
            .unwrap();

        assert!((avg - (33.0 / 9.0)).abs() < 1e-9);
        assert_eq!(min, 1.0);
        assert_eq!(max, 6.0);
        assert_eq!(mode, 2.0);
        assert_eq!(median, 4.0);
        assert!((stddev - expected_stddev).abs() < 1e-9);
    }

    #[test]
    fn grid_size_policy_fit_inside_reduces_resolution_derived_size() {
        let mut r = make_raster();
        r.crs = CrsInfo::from_epsg(4326);

        let extent = Extent {
            x_min: -500_000.0,
            y_min: -400_000.0,
            x_max: 500_000.0,
            y_max: 400_000.0,
        };

        let expand = r
            .reproject_with_options(
                &ReprojectOptions::new(3857, ResampleMethod::Nearest)
                    .with_extent(extent)
                    .with_resolution(300_000.0, 300_000.0)
                    .with_grid_size_policy(GridSizePolicy::Expand),
            )
            .unwrap();

        let fit = r
            .reproject_with_options(
                &ReprojectOptions::new(3857, ResampleMethod::Nearest)
                    .with_extent(extent)
                    .with_resolution(300_000.0, 300_000.0)
                    .with_grid_size_policy(GridSizePolicy::FitInside),
            )
            .unwrap();

        assert!(fit.cols <= expand.cols);
        assert!(fit.rows <= expand.rows);
    }

    #[test]
    fn destination_footprint_masks_cells_outside_source_boundary() {
        let cfg = RasterConfig {
            cols: 4,
            rows: 4,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            ..Default::default()
        };
        let mut r = Raster::new(cfg);
        r.crs = CrsInfo::from_epsg(4326);
        for row in 0..4 {
            for col in 0..4 {
                r.set(0, row, col, 1.0).unwrap();
            }
        }

        let out = r
            .reproject_with_options(
                &ReprojectOptions::new(4326, ResampleMethod::Nearest)
                    .with_extent(Extent {
                        x_min: -1.0,
                        y_min: -1.0,
                        x_max: 5.0,
                        y_max: 5.0,
                    })
                    .with_size(6, 6)
                    .with_destination_footprint(DestinationFootprint::SourceBoundary),
            )
            .unwrap();

        assert!(out.is_nodata(out.get(0, 0, 0)));
        assert!(!out.is_nodata(out.get(0, 2, 2)));
    }

    #[test]
    fn reproject_with_options_honors_snap_origin_with_resolution() {
        let cfg = RasterConfig {
            cols: 6,
            rows: 4,
            x_min: -3.0,
            y_min: -2.0,
            cell_size: 1.0,
            nodata: -9999.0,
            ..Default::default()
        };
        let mut r = Raster::new(cfg);
        r.crs = CrsInfo::from_epsg(4326);
        for row in 0..4 {
            for col in 0..6 {
                r.set(0, row, col, (row * 6 + col) as f64).unwrap();
            }
        }

        let out_extent = Extent {
            x_min: -510_000.0,
            y_min: -390_000.0,
            x_max: 490_000.0,
            y_max: 410_000.0,
        };
        let opts = ReprojectOptions::new(3857, ResampleMethod::Bilinear)
            .with_extent(out_extent)
            .with_resolution(200_000.0, 160_000.0)
            .with_snap_origin(0.0, 0.0);

        let out = r.reproject_with_options(&opts).unwrap();
        assert!((out.x_min - (-600_000.0)).abs() < 1e-6);
        assert!((out.y_min - (-480_000.0)).abs() < 1e-6);
        assert!((out.cell_size_x - 200_000.0).abs() < 1e-6);
        assert!((out.cell_size_y - 160_000.0).abs() < 1e-6);
    }

    #[test]
    fn reproject_with_options_honors_resolution_controls() {
        let cfg = RasterConfig {
            cols: 6,
            rows: 4,
            x_min: -3.0,
            y_min: -2.0,
            cell_size: 1.0,
            nodata: -9999.0,
            ..Default::default()
        };
        let mut r = Raster::new(cfg);
        r.crs = CrsInfo::from_epsg(4326);
        for row in 0..4 {
            for col in 0..6 {
                r.set(0, row, col, (row * 6 + col) as f64).unwrap();
            }
        }

        let out_extent = Extent {
            x_min: -500_000.0,
            y_min: -400_000.0,
            x_max: 500_000.0,
            y_max: 400_000.0,
        };
        let opts = ReprojectOptions::new(3857, ResampleMethod::Bilinear)
            .with_extent(out_extent)
            .with_resolution(200_000.0, 160_000.0);

        let out = r.reproject_with_options(&opts).unwrap();
        assert_eq!(out.cols, 5);
        assert_eq!(out.rows, 5);
        assert!((out.cell_size_x - 200_000.0).abs() < 1e-6);
        assert!((out.cell_size_y - 160_000.0).abs() < 1e-6);
    }

    #[test]
    fn reproject_with_options_rejects_invalid_resolution_controls() {
        let mut r = make_raster();
        r.crs = CrsInfo::from_epsg(4326);
        let opts = ReprojectOptions::new(3857, ResampleMethod::Nearest)
            .with_square_resolution(0.0);
        let err = r.reproject_with_options(&opts).unwrap_err();
        assert!(err
            .to_string()
            .contains("invalid reprojection resolution"));
    }

    #[test]
    fn band_helpers() {
        let cfg = RasterConfig {
            cols: 3,
            rows: 2,
            bands: 2,
            nodata: -9999.0,
            ..Default::default()
        };
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, -9999.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0];
        let mut r = Raster::from_data(cfg, data).unwrap();

        let b0 = r.band_slice(0);
        assert_eq!(b0.len(), 6);
        assert_eq!(b0[0], 1.0);
        assert_eq!(b0[5], -9999.0);

        let s0 = r.statistics_band(0).unwrap();
        assert_eq!(s0.valid_count, 5);
        assert_eq!(s0.nodata_count, 1);
        assert_eq!(s0.min, 1.0);
        assert_eq!(s0.max, 5.0);

        r.set_band_slice(1, &[7.0, 8.0, 9.0, 10.0, 11.0, 12.0]).unwrap();
        assert_eq!(r.get_raw(1, 0, 0), Some(7.0));
        assert_eq!(r.get_raw(1, 1, 2), Some(12.0));
    }

    #[test]
    fn band_transform_helpers() {
        let cfg = RasterConfig {
            cols: 2,
            rows: 2,
            bands: 2,
            nodata: -9999.0,
            ..Default::default()
        };
        let data = vec![1.0, 2.0, 3.0, -9999.0, 10.0, 20.0, 30.0, 40.0];
        let mut r = Raster::from_data(cfg, data).unwrap();

        r.map_valid_band(0, |v| v * 2.0).unwrap();
        assert_eq!(r.get_raw(0, 0, 0), Some(2.0));
        assert_eq!(r.get_raw(0, 0, 1), Some(4.0));
        assert!(r.is_nodata(r.get(0, 1, 1))); // nodata unchanged
        assert_eq!(r.get_raw(1, 0, 0), Some(10.0)); // other band unchanged

        r.replace_band(1, 20.0, 99.0).unwrap();
        assert_eq!(r.get_raw(1, 0, 1), Some(99.0));
        assert_eq!(r.get_raw(1, 0, 0), Some(10.0));
        assert_eq!(r.get_raw(0, 0, 1), Some(4.0));
    }

    #[test]
    fn band_iterators() {
        let cfg = RasterConfig {
            cols: 3,
            rows: 2,
            bands: 2,
            nodata: -9999.0,
            ..Default::default()
        };
        let data = vec![1.0, 2.0, -9999.0, 4.0, 5.0, 6.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0];
        let r = Raster::from_data(cfg, data).unwrap();

        let valid_b0: Vec<_> = r.iter_valid_band(0).unwrap().collect();
        assert_eq!(valid_b0.len(), 5);
        assert_eq!(valid_b0[0], (0, 0, 1.0));
        assert_eq!(valid_b0[4], (1, 2, 6.0));

        let rows_b1: Vec<Vec<f64>> = r.iter_band_rows(1).unwrap().collect();
        assert_eq!(rows_b1.len(), 2);
        assert_eq!(rows_b1[0], vec![10.0, 20.0, 30.0]);
        assert_eq!(rows_b1[1], vec![40.0, 50.0, 60.0]);
    }

    #[test]
    fn mutable_band_rows_native() {
        let cfg = RasterConfig {
            cols: 3,
            rows: 2,
            bands: 1,
            data_type: DataType::F32,
            nodata: -9999.0,
            ..Default::default()
        };
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let mut r = Raster::from_data(cfg, data).unwrap();

        r.for_each_band_row_mut(0, |row, row_mut| {
            if let RasterRowMut::F32(slice) = row_mut {
                for v in slice.iter_mut() {
                    *v += row as f32;
                }
            }
        })
        .unwrap();

        assert_eq!(r.get_raw(0, 0, 0), Some(1.0));
        assert_eq!(r.get_raw(0, 0, 2), Some(3.0));
        assert_eq!(r.get_raw(0, 1, 0), Some(5.0));
        assert_eq!(r.get_raw(0, 1, 2), Some(7.0));
    }

    #[test]
    fn immutable_band_rows_native() {
        let cfg = RasterConfig {
            cols: 3,
            rows: 2,
            bands: 1,
            data_type: DataType::U16,
            nodata: 0.0,
            ..Default::default()
        };
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let r = Raster::from_data(cfg, data).unwrap();

        let mut sums = Vec::new();
        r.for_each_band_row(0, |_row, row_ref| {
            if let RasterRowRef::U16(slice) = row_ref {
                sums.push(slice.iter().map(|v| *v as u64).sum::<u64>());
            }
        })
        .unwrap();

        assert_eq!(sums, vec![6, 15]);
    }
}
