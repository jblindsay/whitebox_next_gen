//! Core coordinate types and transformation traits.

use crate::error::{ProjectionError, Result};

/// Policy controlling behavior when epoch context is incomplete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpochPolicy {
    /// Return an error when epoch context is required but incomplete.
    Strict,
    /// Permit explicit fallback to static routing where caller supports it.
    AllowStaticFallback,
}

/// Shared options for epoch-aware transform routing.
///
/// This type is intended for higher-level platform integrations that need
/// consistent optional epoch parameters across raster/vector/lidar reprojection
/// workflows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EpochTransformOptions {
    /// Observation/coordinate epoch represented by input coordinates.
    pub coordinate_epoch_decimal_year: Option<f64>,
    /// Optional source datum reference epoch.
    pub source_reference_epoch_decimal_year: Option<f64>,
    /// Optional target datum reference epoch.
    pub target_reference_epoch_decimal_year: Option<f64>,
    /// Optional explicit operation code override.
    pub operation_code: Option<u32>,
    /// Try preferred-operation routing when available.
    pub prefer_official_operation: bool,
    /// Incomplete-epoch handling policy.
    pub epoch_policy: EpochPolicy,
}

impl Default for EpochTransformOptions {
    fn default() -> Self {
        Self {
            coordinate_epoch_decimal_year: None,
            source_reference_epoch_decimal_year: None,
            target_reference_epoch_decimal_year: None,
            operation_code: None,
            prefer_official_operation: true,
            epoch_policy: EpochPolicy::Strict,
        }
    }
}

impl EpochTransformOptions {
    /// Construct default epoch transform options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set coordinate epoch (decimal year).
    pub fn with_coordinate_epoch(mut self, coordinate_epoch_decimal_year: f64) -> Self {
        self.coordinate_epoch_decimal_year = Some(coordinate_epoch_decimal_year);
        self
    }

    /// Set source reference epoch (decimal year).
    pub fn with_source_reference_epoch(mut self, source_reference_epoch_decimal_year: f64) -> Self {
        self.source_reference_epoch_decimal_year = Some(source_reference_epoch_decimal_year);
        self
    }

    /// Set target reference epoch (decimal year).
    pub fn with_target_reference_epoch(mut self, target_reference_epoch_decimal_year: f64) -> Self {
        self.target_reference_epoch_decimal_year = Some(target_reference_epoch_decimal_year);
        self
    }

    /// Set explicit operation code override.
    pub fn with_operation_code(mut self, operation_code: u32) -> Self {
        self.operation_code = Some(operation_code);
        self
    }

    /// Enable or disable preferred-operation routing.
    pub fn with_preferred_operation(mut self, enabled: bool) -> Self {
        self.prefer_official_operation = enabled;
        self
    }

    /// Set epoch policy.
    pub fn with_epoch_policy(mut self, epoch_policy: EpochPolicy) -> Self {
        self.epoch_policy = epoch_policy;
        self
    }

    /// Validate option semantics and numeric values.
    pub fn validate(&self) -> Result<()> {
        if let Some(v) = self.coordinate_epoch_decimal_year {
            if !v.is_finite() {
                return Err(ProjectionError::DatumError(
                    "coordinate epoch must be finite".to_string(),
                ));
            }
        }
        if let Some(v) = self.source_reference_epoch_decimal_year {
            if !v.is_finite() {
                return Err(ProjectionError::DatumError(
                    "source reference epoch must be finite".to_string(),
                ));
            }
        }
        if let Some(v) = self.target_reference_epoch_decimal_year {
            if !v.is_finite() {
                return Err(ProjectionError::DatumError(
                    "target reference epoch must be finite".to_string(),
                ));
            }
        }

        if self.coordinate_epoch_decimal_year.is_none()
            && (self.source_reference_epoch_decimal_year.is_some()
                || self.target_reference_epoch_decimal_year.is_some())
        {
            return Err(ProjectionError::DatumError(
                "source/target reference epochs require coordinate epoch".to_string(),
            ));
        }

        Ok(())
    }

    /// Build a transform epoch context when coordinate epoch is provided.
    pub fn build_context(&self) -> Result<Option<TransformEpochContext>> {
        self.validate()?;

        Ok(self.coordinate_epoch_decimal_year.map(|coordinate_epoch_decimal_year| {
            TransformEpochContext::new(
                coordinate_epoch_decimal_year,
                self.source_reference_epoch_decimal_year,
                self.target_reference_epoch_decimal_year,
            )
        }))
    }
}

/// Context describing the epoch assumptions for a coordinate transformation.
///
/// Epoch values use decimal years (for example, `2020.0`, `2024.5`).
///
/// This type is additive scaffolding for dynamic-datum workflows and is currently
/// carried through context-aware APIs without changing static transform behavior.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformEpochContext {
    /// Observation/coordinate epoch represented by the input coordinate.
    pub coordinate_epoch_decimal_year: f64,
    /// Optional source datum reference epoch.
    pub source_reference_epoch_decimal_year: Option<f64>,
    /// Optional target datum reference epoch.
    pub target_reference_epoch_decimal_year: Option<f64>,
}

impl TransformEpochContext {
    /// Create a context with only a coordinate epoch.
    pub const fn at_epoch(coordinate_epoch_decimal_year: f64) -> Self {
        Self {
            coordinate_epoch_decimal_year,
            source_reference_epoch_decimal_year: None,
            target_reference_epoch_decimal_year: None,
        }
    }

    /// Create a context with explicit coordinate and reference epochs.
    pub const fn new(
        coordinate_epoch_decimal_year: f64,
        source_reference_epoch_decimal_year: Option<f64>,
        target_reference_epoch_decimal_year: Option<f64>,
    ) -> Self {
        Self {
            coordinate_epoch_decimal_year,
            source_reference_epoch_decimal_year,
            target_reference_epoch_decimal_year,
        }
    }
}

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
