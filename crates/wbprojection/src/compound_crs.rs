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
    /// # Supported codes
    ///
    /// | EPSG  | Name |
    /// |-------|------|
    /// | 5498  | NAD83 + NAVD88 height |
    /// | 6649  | NAD83(CSRS) + CGVD2013 height |
    /// | 7405  | OSGB36 / British National Grid + ODN height |
    /// | 9253  | GDA94 + AHD height |
    /// | 9518  | WGS 84 + EGM2008 height |
    ///
    /// For any other compound code, construct the CRS manually with
    /// [`CompoundCrs::new`] from individual horizontal and vertical [`Crs`] components.
    pub fn from_epsg(code: u32) -> Result<Self> {
        match code {
            5498 => {
                let horizontal = Crs::from_epsg(4269)?; // NAD83 geographic
                let vertical = Crs::from_epsg(5703)?;   // NAVD88 height
                Ok(Self {
                    name: "NAD83 + NAVD88 height (EPSG:5498)".to_string(),
                    horizontal,
                    vertical,
                    epsg_code: Some(code),
                })
            }
            6649 => {
                let horizontal = Crs::from_epsg(4617)?; // NAD83(CSRS) geographic
                let vertical = Crs::from_epsg(6647)?;   // CGVD2013 height
                Ok(Self {
                    name: "NAD83(CSRS) + CGVD2013 height (EPSG:6649)".to_string(),
                    horizontal,
                    vertical,
                    epsg_code: Some(code),
                })
            }
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
            9253 => {
                let horizontal = Crs::from_epsg(4283)?; // GDA94 geographic
                let vertical = Crs::from_epsg(5711)?;   // AHD height
                Ok(Self {
                    name: "GDA94 + AHD height (EPSG:9253)".to_string(),
                    horizontal,
                    vertical,
                    epsg_code: Some(code),
                })
            }
            9518 => {
                let horizontal = Crs::from_epsg(4326)?; // WGS84 geographic
                let vertical = Crs::from_epsg(3855)?;   // EGM2008 height
                Ok(Self {
                    name: "WGS 84 + EGM2008 height (EPSG:9518)".to_string(),
                    horizontal,
                    vertical,
                    epsg_code: Some(code),
                })
            }
            _ => Err(ProjectionError::UnsupportedProjection(format!(
                "compound EPSG:{code} is not in the built-in registry; \
                 use CompoundCrs::new() with individual horizontal and vertical Crs components"
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
