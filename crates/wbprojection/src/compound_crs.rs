//! Compound CRS support (horizontal + vertical).

use crate::crs::{Crs, CrsTransformPolicy};
use crate::error::{ProjectionError, Result};
use crate::projections::ProjectionKind;

/// Compound CRS combining a horizontal CRS with a vertical CRS.
#[derive(Debug)]
pub struct CompoundCrs {
    /// Human-readable name.
    pub name: String,
    /// Horizontal component CRS (projected or geographic).
    pub horizontal: Crs,
    /// Vertical component CRS.
    pub vertical: Crs,
    /// Optional compound EPSG code.
    pub epsg_code: Option<u32>,
}

impl CompoundCrs {
    /// Construct a custom compound CRS from horizontal and vertical components.
    pub fn new(name: impl Into<String>, horizontal: Crs, vertical: Crs) -> Result<Self> {
        if matches!(horizontal.projection.params().kind, ProjectionKind::Vertical) {
            return Err(ProjectionError::UnsupportedProjection(
                "horizontal component cannot be vertical".to_string(),
            ));
        }
        if !matches!(vertical.projection.params().kind, ProjectionKind::Vertical) {
            return Err(ProjectionError::UnsupportedProjection(
                "vertical component must be a vertical CRS".to_string(),
            ));
        }

        Ok(Self {
            name: name.into(),
            horizontal,
            vertical,
            epsg_code: None,
        })
    }

    /// Build a known compound CRS from an EPSG code.
    ///
    /// Currently supported:
    /// - EPSG:7405 (OSGB36 / British National Grid + ODN height)
    pub fn from_epsg(code: u32) -> Result<Self> {
        match code {
            7405 => {
                let horizontal = Crs::from_epsg(27700)?;
                let vertical = Crs::from_epsg(5701)?;
                Ok(Self {
                    name: "OSGB36 / British National Grid + ODN height (EPSG:7405)".to_string(),
                    horizontal,
                    vertical,
                    epsg_code: Some(code),
                })
            }
            _ => Err(ProjectionError::UnsupportedProjection(format!(
                "compound EPSG:{code} is not currently supported"
            ))),
        }
    }

    /// Transform a 3D point into a target compound CRS.
    pub fn transform_to(&self, x: f64, y: f64, z: f64, target: &CompoundCrs) -> Result<(f64, f64, f64)> {
        self.transform_to_with_policy(x, y, z, target, CrsTransformPolicy::Strict)
    }

    /// Policy-enabled variant of [`CompoundCrs::transform_to`].
    pub fn transform_to_with_policy(
        &self,
        x: f64,
        y: f64,
        z: f64,
        target: &CompoundCrs,
        policy: CrsTransformPolicy,
    ) -> Result<(f64, f64, f64)> {
        let (x_out, y_out) = self
            .horizontal
            .transform_to_with_policy(x, y, &target.horizontal, policy)?;

        // Derive lon/lat context from source horizontal component for vertical-model sampling.
        let (lon_deg, lat_deg) = self.horizontal.inverse(x, y)?;

        let (_, _, z_out) = self
            .vertical
            .transform_to_3d_with_policy(lon_deg, lat_deg, z, &target.vertical, policy)?;

        Ok((x_out, y_out, z_out))
    }
}
