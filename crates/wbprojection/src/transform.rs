//! Core coordinate types and transformation traits.

use crate::error::Result;

/// A 2D coordinate pair (x, y or lon, lat or easting, northing).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2D {
    /// First component (x, easting, or longitude in degrees).
    pub x: f64,
    /// Second component (y, northing, or latitude in degrees).
    pub y: f64,
}

impl Point2D {
    /// Create a new 2D point.
    pub fn new(x: f64, y: f64) -> Self {
        Point2D { x, y }
    }

    /// Interpret this point as (longitude, latitude) in degrees.
    pub fn lonlat(lon: f64, lat: f64) -> Self {
        Point2D { x: lon, y: lat }
    }

    /// Return the coordinates as a tuple (x, y).
    pub fn to_tuple(self) -> (f64, f64) {
        (self.x, self.y)
    }
}

impl From<(f64, f64)> for Point2D {
    fn from((x, y): (f64, f64)) -> Self {
        Point2D::new(x, y)
    }
}

impl From<Point2D> for (f64, f64) {
    fn from(p: Point2D) -> Self {
        (p.x, p.y)
    }
}

impl std::fmt::Display for Point2D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:.6}, {:.6})", self.x, self.y)
    }
}

/// A 3D coordinate (x, y, z or lon, lat, height).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point3D {
    /// X or longitude.
    pub x: f64,
    /// Y or latitude.
    pub y: f64,
    /// Z or ellipsoidal height.
    pub z: f64,
}

impl Point3D {
    /// Create a new 3D point.
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Point3D { x, y, z }
    }

    /// Return the 2D part.
    pub fn xy(&self) -> Point2D {
        Point2D::new(self.x, self.y)
    }
}

impl From<(f64, f64, f64)> for Point3D {
    fn from((x, y, z): (f64, f64, f64)) -> Self {
        Point3D::new(x, y, z)
    }
}

/// A coordinate transformation that converts between two coordinate systems.
pub trait CoordTransform {
    /// Transform a single point forward.
    fn transform_fwd(&self, point: Point2D) -> Result<Point2D>;

    /// Transform a single point inverse.
    fn transform_inv(&self, point: Point2D) -> Result<Point2D>;

    /// Transform a slice of points forward in-place.
    fn transform_fwd_many(&self, points: &mut [Point2D]) -> Vec<Result<()>> {
        points
            .iter_mut()
            .map(|p| {
                let result = self.transform_fwd(*p)?;
                *p = result;
                Ok(())
            })
            .collect()
    }

    /// Transform a slice of points inverse in-place.
    fn transform_inv_many(&self, points: &mut [Point2D]) -> Vec<Result<()>> {
        points
            .iter_mut()
            .map(|p| {
                let result = self.transform_inv(*p)?;
                *p = result;
                Ok(())
            })
            .collect()
    }
}
