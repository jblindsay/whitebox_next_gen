//! Core geometric, geographic, and image types.

// ── GeoTransform ─────────────────────────────────────────────────────────────

/// A 6-parameter affine transform mapping pixel coordinates to geographic coordinates.
///
/// Follows the GDAL convention:
/// ```text
/// X_geo = x_origin + col * pixel_width  + row * row_rotation
/// Y_geo = y_origin + col * col_rotation + row * pixel_height
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct GeoTransform {
    /// X coordinate of the upper-left corner of the upper-left pixel.
    pub x_origin: f64,
    /// Pixel width (X resolution), usually positive.
    pub pixel_width: f64,
    /// Row rotation (usually 0.0 for north-up images).
    pub row_rotation: f64,
    /// Y coordinate of the upper-left corner of the upper-left pixel.
    pub y_origin: f64,
    /// Column rotation (usually 0.0 for north-up images).
    pub col_rotation: f64,
    /// Pixel height (Y resolution), usually negative for north-up images.
    pub pixel_height: f64,
}

impl GeoTransform {
    /// Create a new `GeoTransform` from its six components.
    pub fn new(
        x_origin: f64, pixel_width: f64, row_rotation: f64,
        y_origin: f64, col_rotation: f64, pixel_height: f64,
    ) -> Self {
        Self { x_origin, pixel_width, row_rotation, y_origin, col_rotation, pixel_height }
    }

    /// Create a north-up (no rotation) transform.
    ///
    /// `pixel_height` should be **negative** for a north-up raster.
    pub fn north_up(x_origin: f64, pixel_width: f64, y_origin: f64, pixel_height: f64) -> Self {
        Self::new(x_origin, pixel_width, 0.0, y_origin, 0.0, pixel_height)
    }

    /// Convert pixel (col, row) → geographic (x, y).
    pub fn pixel_to_geo(&self, col: f64, row: f64) -> (f64, f64) {
        let x = self.x_origin + col * self.pixel_width  + row * self.row_rotation;
        let y = self.y_origin + col * self.col_rotation + row * self.pixel_height;
        (x, y)
    }

    /// Convert geographic (x, y) → fractional pixel (col, row).
    ///
    /// Returns `None` if the transform is singular.
    pub fn geo_to_pixel(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let det = self.pixel_width * self.pixel_height - self.row_rotation * self.col_rotation;
        if det.abs() < f64::EPSILON { return None; }
        let dx = x - self.x_origin;
        let dy = y - self.y_origin;
        Some((
            (self.pixel_height * dx - self.row_rotation * dy) / det,
            (self.pixel_width  * dy - self.col_rotation * dx) / det,
        ))
    }
}

impl Default for GeoTransform {
    fn default() -> Self { Self::new(0.0, 1.0, 0.0, 0.0, 0.0, -1.0) }
}

// ── BoundingBox ───────────────────────────────────────────────────────────────

/// Axis-aligned bounding box in geographic or projected coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundingBox {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl BoundingBox {
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self { min_x, min_y, max_x, max_y }
    }
    pub fn width(&self)  -> f64 { self.max_x - self.min_x }
    pub fn height(&self) -> f64 { self.max_y - self.min_y }
    pub fn center(&self) -> (f64, f64) {
        ((self.min_x + self.max_x) / 2.0, (self.min_y + self.max_y) / 2.0)
    }
}

// ── SampleFormat ─────────────────────────────────────────────────────────────

/// How sample values should be interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SampleFormat {
    /// Unsigned integer (u8, u16, …).
    #[default]
    Uint,
    /// Signed (two's complement) integer.
    Int,
    /// IEEE 754 floating point.
    Float,
}

/// Combined pixel type: format + bit depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelType {
    Uint8,
    Uint16,
    Int16,
    Int32,
    Float32,
    Float64,
}

impl PixelType {
    /// Bytes per sample.
    pub fn byte_size(self) -> usize {
        match self {
            Self::Uint8  => 1,
            Self::Uint16 | Self::Int16 => 2,
            Self::Int32  | Self::Float32 => 4,
            Self::Float64 => 8,
        }
    }
    /// Bits per sample.
    pub fn bits(self) -> u8 { (self.byte_size() * 8) as u8 }

    /// The `SampleFormat` of this type.
    pub fn sample_format(self) -> SampleFormat {
        match self {
            Self::Uint8 | Self::Uint16 => SampleFormat::Uint,
            Self::Int16 | Self::Int32  => SampleFormat::Int,
            Self::Float32 | Self::Float64 => SampleFormat::Float,
        }
    }
    /// Whether the type is signed.
    pub fn is_signed(self) -> bool { self.sample_format() != SampleFormat::Uint }
}

// ── ColorSpace ────────────────────────────────────────────────────────────────

/// JP2 colour space (from the `colr` box).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorSpace {
    /// Greyscale (1 component).
    #[default]
    Greyscale,
    /// sRGB (3 components).
    Srgb,
    /// YCbCr (3 components, JPEG-standard).
    YCbCr,
    /// Generic multi-band (any number of components, e.g. multispectral).
    MultiBand,
}

impl ColorSpace {
    /// JP2 enumerated colourspace value for the `colr` box.
    pub fn enumcs(self) -> u32 {
        match self {
            Self::Greyscale => 17,
            Self::Srgb      => 16,
            Self::YCbCr     => 18,
            Self::MultiBand => 0,
        }
    }
    /// Parse from a JP2 `colr` enumerated colourspace value.
    pub fn from_enumcs(v: u32) -> Self {
        match v {
            16 => Self::Srgb,
            17 => Self::Greyscale,
            18 => Self::YCbCr,
            _  => Self::MultiBand,
        }
    }
}

// ── CompressionMode ───────────────────────────────────────────────────────────

/// JPEG 2000 compression mode.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum CompressionMode {
    /// Lossless compression using the reversible 5/3 integer wavelet transform.
    #[default]
    Lossless,
    /// Lossy compression using the irreversible 9/7 float wavelet transform.
    /// `quality` is a target PSNR hint in dB; higher = better quality.
    Lossy {
        /// Target quality in dB (20–60 is typical; 0.0 = use default quantisation).
        quality_db: f32,
    },
}

impl CompressionMode {
    /// Whether this mode uses the reversible (integer) wavelet.
    pub fn is_lossless(self) -> bool { self == Self::Lossless }
}
