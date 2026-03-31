//! Core geometric and geographic types.

#![allow(dead_code)]

/// A 6-parameter affine transform mapping pixel coordinates to geographic coordinates.
///
/// The transform follows the GDAL convention:
/// ```text
/// X_geo = x_origin + col * pixel_width  + row * row_rotation
/// Y_geo = y_origin + col * col_rotation + row * pixel_height
/// ```
///
/// For a north-up image with no rotation:
/// - `x_origin`: longitude/easting of the upper-left pixel corner
/// - `pixel_width`: positive pixel size in X (degrees or metres)
/// - `row_rotation`: typically 0.0
/// - `y_origin`: latitude/northing of the upper-left pixel corner
/// - `col_rotation`: typically 0.0
/// - `pixel_height`: *negative* pixel size in Y (image rows run south)
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
    ///
    /// # Example
    /// ```ignore
    /// use wbraster::formats::geotiff_core::GeoTransform;
    /// // 0.1-degree pixels, top-left at (-180, 90)
    /// let t = GeoTransform::new(-180.0, 0.1, 0.0, 90.0, 0.0, -0.1);
    /// ```
    pub fn new(
        x_origin: f64,
        pixel_width: f64,
        row_rotation: f64,
        y_origin: f64,
        col_rotation: f64,
        pixel_height: f64,
    ) -> Self {
        Self {
            x_origin,
            pixel_width,
            row_rotation,
            y_origin,
            col_rotation,
            pixel_height,
        }
    }

    /// Create a north-up (no rotation) transform from origin and pixel sizes.
    ///
    /// `pixel_height` should be **negative** for a conventional north-up raster.
    pub fn north_up(x_origin: f64, pixel_width: f64, y_origin: f64, pixel_height: f64) -> Self {
        Self::new(x_origin, pixel_width, 0.0, y_origin, 0.0, pixel_height)
    }

    /// Convert pixel (col, row) → geographic (x, y).
    pub fn pixel_to_geo(&self, col: f64, row: f64) -> (f64, f64) {
        let x = self.x_origin + col * self.pixel_width + row * self.row_rotation;
        let y = self.y_origin + col * self.col_rotation + row * self.pixel_height;
        (x, y)
    }

    /// Convert geographic (x, y) → fractional pixel (col, row).
    ///
    /// Returns `None` if the transform is not invertible (determinant ≈ 0).
    pub fn geo_to_pixel(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let det = self.pixel_width * self.pixel_height - self.row_rotation * self.col_rotation;
        if det.abs() < f64::EPSILON {
            return None;
        }
        let dx = x - self.x_origin;
        let dy = y - self.y_origin;
        let col = (self.pixel_height * dx - self.row_rotation * dy) / det;
        let row = (self.pixel_width * dy - self.col_rotation * dx) / det;
        Some((col, row))
    }

    /// Return the TIFF `ModelPixelScale` tag value `[sx, sy, sz]`.
    pub fn to_pixel_scale(&self) -> [f64; 3] {
        [self.pixel_width, -self.pixel_height, 0.0]
    }

    /// Return the TIFF `ModelTiepoint` tag value for the upper-left pixel.
    pub fn to_tiepoint(&self) -> [f64; 6] {
        [0.0, 0.0, 0.0, self.x_origin, self.y_origin, 0.0]
    }

    /// Build a `GeoTransform` from TIFF `ModelPixelScale` and `ModelTiepoint` tags.
    ///
    /// `scale` is `[sx, sy, sz]`, `tiepoint` is one 6-element entry `[px, py, pz, gx, gy, gz]`.
    pub fn from_scale_tiepoint(scale: &[f64], tiepoint: &[f64]) -> Option<Self> {
        if scale.len() < 2 || tiepoint.len() < 6 {
            return None;
        }
        let (px, py) = (tiepoint[0], tiepoint[1]);
        let (gx, gy) = (tiepoint[3], tiepoint[4]);
        let sx = scale[0];
        let sy = scale[1]; // positive in tag, we store negative for Y
        Some(Self::new(
            gx - px * sx,
            sx,
            0.0,
            gy + py * sy,
            0.0,
            -sy,
        ))
    }
}

impl Default for GeoTransform {
    fn default() -> Self {
        Self::new(0.0, 1.0, 0.0, 0.0, 0.0, -1.0)
    }
}

/// Bounding box in geographic or projected coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundingBox {
    /// Minimum X (west / left)
    pub min_x: f64,
    /// Minimum Y (south / bottom)
    pub min_y: f64,
    /// Maximum X (east / right)
    pub max_x: f64,
    /// Maximum Y (north / top)
    pub max_y: f64,
}

impl BoundingBox {
    /// Create a new bounding box.
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self { min_x, min_y, max_x, max_y }
    }

    /// Width of the bounding box.
    pub fn width(&self) -> f64 {
        self.max_x - self.min_x
    }

    /// Height of the bounding box.
    pub fn height(&self) -> f64 {
        self.max_y - self.min_y
    }

    /// Centre point of the bounding box.
    pub fn center(&self) -> (f64, f64) {
        ((self.min_x + self.max_x) / 2.0, (self.min_y + self.max_y) / 2.0)
    }
}
