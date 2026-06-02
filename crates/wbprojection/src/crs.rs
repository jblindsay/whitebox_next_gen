//! Coordinate Reference System definitions and transformations.
//!
//! A [`Crs`] combines a datum with a projection and allows transforming
//! coordinates between different CRSes through WGS84 as a pivot datum.

use crate::datum::{Datum, DatumTransformPolicy};
use crate::epsg::EpsgResolutionPolicy;
use crate::error::{ProjectionError, Result};
use crate::operations::get_coordinate_operation;
use crate::projections::{Projection, ProjectionParams, ProjectionKind};
use crate::{
    PreferredOperationPolicy,
    preferred_operation_for_crs_pair_with_policy,
    register_coordinate_operation,
};
use crate::datum::{ecef_to_geodetic, geodetic_to_ecef};
use crate::transform::TransformEpochContext;
use crate::vertical_grid::get_vertical_offset_grid;
use crate::{to_degrees, to_radians};

fn epsg_code_from_crs_name(name: &str) -> Option<u32> {
    let marker = "(EPSG:";
    let start = name.rfind(marker)? + marker.len();
    let rest = &name[start..];
    let end = rest.find(')')?;
    rest[..end].parse::<u32>().ok()
}

fn sample_vertical_offset_with_policy(
    epsg_code: u32,
    lon_deg: f64,
    lat_deg: f64,
    policy: CrsTransformPolicy,
) -> Result<Option<f64>> {
    let Some(grid_name) = crate::epsg::vertical_offset_grid_name(epsg_code) else {
        return Ok(None);
    };

    let Some(grid) = get_vertical_offset_grid(grid_name)? else {
        // Keep vertical<->vertical backward compatible when grids are not yet registered.
        return Ok(None);
    };

    match grid.sample(lon_deg, lat_deg) {
        Ok(v) => Ok(Some(v)),
        Err(e) => match policy {
            CrsTransformPolicy::Strict => Err(e),
            CrsTransformPolicy::Auto => Err(e),
            CrsTransformPolicy::FallbackToIdentityGridShift => Ok(None),
        },
    }
}

fn datums_are_ballpark_equivalent(src: &Datum, dst: &Datum) -> bool {
    if src == dst {
        return true;
    }

    let src_name = src.name;
    let dst_name = dst.name;
    let src_is_wgs84 = src_name == "WGS 84";
    let dst_is_wgs84 = dst_name == "WGS 84";
    let src_is_nad83 = matches!(src_name, "NAD 83" | "NAD83(NSRS2007)" | "NAD83(HARN)");
    let dst_is_nad83 = matches!(dst_name, "NAD 83" | "NAD83(NSRS2007)" | "NAD83(HARN)");

    (src_is_wgs84 && dst_is_nad83) || (src_is_nad83 && dst_is_wgs84)
}

/// Policy controlling behavior when datum transforms cannot be applied exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrsTransformPolicy {
    /// Return errors for missing grids, out-of-extent coordinates, and other datum issues.
    Strict,
    /// Prefer ballpark-equivalent datum treatment for common interoperable pairs
    /// (currently WGS84 <-> NAD83 family) while retaining strict grid behavior.
    Auto,
    /// For grid-shift failures, fall back to identity shift instead of returning an error.
    FallbackToIdentityGridShift,
}

/// Result of a CRS transformation with optional datum-grid trace metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct CrsTransformTrace {
    /// Output x coordinate in target CRS units.
    pub x: f64,
    /// Output y coordinate in target CRS units.
    pub y: f64,
    /// Source-datum grid selected during source → WGS84 leg, if applicable.
    pub source_grid: Option<String>,
    /// Target-datum grid selected during WGS84 → target leg, if applicable.
    pub target_grid: Option<String>,
}

/// Provides source/target vertical offsets for preserve-horizontal 3D workflows.
///
/// Offsets use the same convention as
/// [`Crs::transform_to_3d_preserve_horizontal_with_vertical_offsets`]:
/// - `source_to_ellipsoidal_m`: add to source height to obtain ellipsoidal height.
/// - `target_to_ellipsoidal_m`: subtract from ellipsoidal height to obtain target height.
pub trait VerticalOffsetProvider {
    /// Return `(source_to_ellipsoidal_m, target_to_ellipsoidal_m)` for the given point.
    fn offsets(
        &self,
        x: f64,
        y: f64,
        source: &Crs,
        target: &Crs,
    ) -> Result<(f64, f64)>;
}

/// Fixed vertical-offset provider.
///
/// Useful when source/target vertical offsets are constant for a workflow.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstantVerticalOffsetProvider {
    /// Offset added to source height to obtain ellipsoidal height.
    pub source_to_ellipsoidal_m: f64,
    /// Offset subtracted from ellipsoidal height to obtain target height.
    pub target_to_ellipsoidal_m: f64,
}

impl ConstantVerticalOffsetProvider {
    /// Create a new constant provider.
    pub fn new(source_to_ellipsoidal_m: f64, target_to_ellipsoidal_m: f64) -> Self {
        Self {
            source_to_ellipsoidal_m,
            target_to_ellipsoidal_m,
        }
    }
}

impl VerticalOffsetProvider for ConstantVerticalOffsetProvider {
    fn offsets(
        &self,
        _x: f64,
        _y: f64,
        _source: &Crs,
        _target: &Crs,
    ) -> Result<(f64, f64)> {
        Ok((self.source_to_ellipsoidal_m, self.target_to_ellipsoidal_m))
    }
}

impl<F> VerticalOffsetProvider for F
where
    F: Fn(f64, f64, &Crs, &Crs) -> Result<(f64, f64)>,
{
    fn offsets(
        &self,
        x: f64,
        y: f64,
        source: &Crs,
        target: &Crs,
    ) -> Result<(f64, f64)> {
        self(x, y, source, target)
    }
}

/// Grid-backed vertical-offset provider.
///
/// This provider samples source and target vertical offset grids at the
/// horizontal coordinate context of a transformation point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridVerticalOffsetProvider {
    /// Name of source vertical offset grid.
    pub source_grid: String,
    /// Name of target vertical offset grid.
    pub target_grid: String,
}

impl GridVerticalOffsetProvider {
    /// Create a new grid-backed provider from registered grid names.
    pub fn new(source_grid: impl Into<String>, target_grid: impl Into<String>) -> Self {
        Self {
            source_grid: source_grid.into(),
            target_grid: target_grid.into(),
        }
    }

    fn resolve_horizontal_lon_lat(
        x: f64,
        y: f64,
        source: &Crs,
        target: &Crs,
    ) -> Result<(f64, f64)> {
        let source_is_vertical = matches!(source.projection.params().kind, ProjectionKind::Vertical);
        let target_is_vertical = matches!(target.projection.params().kind, ProjectionKind::Vertical);

        let horizontal_context = if !source_is_vertical {
            source
        } else if !target_is_vertical {
            target
        } else {
            return Err(ProjectionError::DatumError(
                "cannot resolve horizontal context when both source and target are vertical"
                    .to_string(),
            ));
        };

        match horizontal_context.projection.params().kind {
            ProjectionKind::Geographic => Ok((x, y)),
            ProjectionKind::Geocentric => Err(ProjectionError::DatumError(
                "geocentric CRS is not supported for vertical offset grid sampling".to_string(),
            )),
            ProjectionKind::Vertical => Err(ProjectionError::DatumError(
                "vertical CRS cannot provide horizontal sampling coordinates".to_string(),
            )),
            _ => horizontal_context.inverse(x, y),
        }
    }
}

impl VerticalOffsetProvider for GridVerticalOffsetProvider {
    fn offsets(
        &self,
        x: f64,
        y: f64,
        source: &Crs,
        target: &Crs,
    ) -> Result<(f64, f64)> {
        let (lon_deg, lat_deg) = Self::resolve_horizontal_lon_lat(x, y, source, target)?;

        let source_grid = get_vertical_offset_grid(&self.source_grid)?.ok_or_else(|| {
            ProjectionError::DatumError(format!(
                "vertical offset grid '{}' not registered",
                self.source_grid
            ))
        })?;

        let target_grid = get_vertical_offset_grid(&self.target_grid)?.ok_or_else(|| {
            ProjectionError::DatumError(format!(
                "vertical offset grid '{}' not registered",
                self.target_grid
            ))
        })?;

        let source_to_ellipsoidal_m = source_grid.sample(lon_deg, lat_deg)?;
        let target_to_ellipsoidal_m = target_grid.sample(lon_deg, lat_deg)?;

        Ok((source_to_ellipsoidal_m, target_to_ellipsoidal_m))
    }
}

/// A Coordinate Reference System: datum + projection.
#[derive(Clone)]
pub struct Crs {
    /// Human-readable name.
    pub name: String,
    /// The datum (ellipsoid + Helmert params).
    pub datum: Datum,
    /// The map projection applied to geographic coordinates.
    pub projection: Projection,
}

impl Crs {
    /// Create a [`Crs`] from an EPSG numeric code.
    ///
    /// This is the most convenient way to get a well-known projection:
    ///
    /// ```rust
    /// use wbprojection::Crs;
    ///
    /// let utm32n  = Crs::from_epsg(32632).unwrap();  // WGS84 / UTM zone 32N
    /// let web     = Crs::from_epsg(3857).unwrap();   // Web Mercator
    /// let bng     = Crs::from_epsg(27700).unwrap();  // British National Grid
    /// let wgs84   = Crs::from_epsg(4326).unwrap();   // WGS 84 geographic
    /// ```
    ///
    /// See [`crate::epsg::known_epsg_codes()`] for the full list of supported codes.
    pub fn from_epsg(code: u32) -> Result<Self> {
        crate::epsg::from_epsg(code)
    }

    /// Create a [`Crs`] from an EPSG code using explicit resolution policy.
    ///
    /// This allows opt-in fallback to a known supported code when the requested
    /// EPSG is unavailable in the built-in registry.
    pub fn from_epsg_with_policy(code: u32, policy: EpsgResolutionPolicy) -> Result<Self> {
        crate::epsg::from_epsg_with_policy(code, policy)
    }

    /// Create a [`Crs`] from an EPSG code using the built-in alias catalog and policy.
    pub fn from_epsg_with_catalog(code: u32, policy: EpsgResolutionPolicy) -> Result<Self> {
        crate::epsg::from_epsg_with_catalog(code, policy)
    }

    /// Parse a [`Crs`] from a WKT1 or WKT2 string.
    ///
    /// This is a convenience wrapper around [`crate::epsg::from_wkt`].
    pub fn from_wkt(wkt: &str) -> Result<Self> {
        crate::epsg::from_wkt(wkt)
    }

    /// Serialize this CRS to an Esri-style WKT1 string.
    ///
    /// All coordinate values are expressed in metres regardless of the original
    /// source units (e.g. a State-Plane CRS defined in US survey feet will emit
    /// metre-based parameter values).  If you need the canonical EPSG WKT with
    /// original units preserved, use [`crate::epsg::to_esri_wkt`] with the
    /// known EPSG code.
    ///
    /// # Examples
    /// ```rust
    /// use wbprojection::Crs;
    ///
    /// let crs = Crs::from_epsg(32617).unwrap();
    /// let wkt = crs.to_wkt();
    /// assert!(wkt.starts_with("PROJCS["));
    /// ```
    pub fn to_wkt(&self) -> String {
        crate::epsg::crs_to_wkt(self)
    }

    /// Return the geographic area of use (bounding box in decimal degrees) for
    /// this CRS, if it is known.
    ///
    /// The bounding box is looked up from the EPSG code embedded in `self.name`
    /// (if present), or derived from the projection kind for UTM zones and
    /// world-extent geographic CRSes.
    ///
    /// Returns `None` when no area-of-use information is available (e.g. for
    /// custom CRSes constructed without an EPSG code).
    ///
    /// # Examples
    /// ```rust
    /// use wbprojection::Crs;
    ///
    /// let utm = Crs::from_epsg(32632).unwrap(); // WGS84 / UTM zone 32N
    /// let bb = utm.area_of_use().unwrap();
    /// assert!(bb.contains_geographic(9.0, 51.0)); // central Germany — inside zone 32
    /// assert!(!bb.contains_geographic(15.0, 51.0)); // zone 33 — outside
    /// ```
    pub fn area_of_use(&self) -> Option<crate::epsg::CrsBoundingBox> {
        // Try to extract an EPSG code from the CRS name first.
        if let Some(code) = epsg_code_from_crs_name(&self.name) {
            if let Some(bb) = crate::epsg::epsg_area_of_use(code) {
                return Some(bb);
            }
        }
        // Fallback: derive from projection kind.
        use crate::projections::ProjectionKind;
        match self.projection.params().kind {
            ProjectionKind::Geographic => Some(crate::epsg::CrsBoundingBox::new(
                -180.0, -90.0, 180.0, 90.0,
            )),
            _ => None,
        }
    }

    /// Create a CRS from a datum and projection parameters.
    pub fn new(name: impl Into<String>, datum: Datum, params: ProjectionParams) -> Result<Self> {
        let projection = Projection::new(params)?;
        Ok(Crs {
            name: name.into(),
            datum,
            projection,
        })
    }

    /// WGS84 geographic (no projection, EPSG:4326).
    pub fn wgs84_geographic() -> Self {
        let params = ProjectionParams::new(ProjectionKind::Geographic);
        Crs {
            name: "WGS 84 (geographic)".to_string(),
            datum: Datum::WGS84,
            projection: Projection::new(params).unwrap(),
        }
    }

    /// Returns `true` when this CRS uses geographic longitude/latitude coordinates.
    ///
    /// This is `true` only for CRSes whose projection kind is
    /// [`ProjectionKind::Geographic`]. It is `false` for projected,
    /// geocentric, and vertical CRSes.
    pub fn is_geographic(&self) -> bool {
        matches!(self.projection.params().kind, ProjectionKind::Geographic)
    }

    /// Returns `true` when this CRS is a projected horizontal CRS.
    ///
    /// This helper intentionally excludes geographic, geocentric, and vertical
    /// CRSes. In other words, this is not equivalent to `!self.is_geographic()`;
    /// it is only `true` for map-projected coordinate systems such as UTM,
    /// Web Mercator, Lambert Conformal Conic, and similar projected CRSes.
    pub fn is_projected(&self) -> bool {
        !matches!(
            self.projection.params().kind,
            ProjectionKind::Geographic | ProjectionKind::Geocentric | ProjectionKind::Vertical
        )
    }

    /// Web Mercator (EPSG:3857).
    pub fn web_mercator() -> Self {
        Crs {
            name: "WGS 84 / Web Mercator (EPSG:3857)".to_string(),
            datum: Datum::WGS84,
            projection: Projection::new(ProjectionParams::web_mercator()).unwrap(),
        }
    }

    /// UTM for the given zone and hemisphere.
    pub fn utm(zone: u8, south: bool) -> Self {
        let name = format!("WGS84 / UTM zone {}{}", zone, if south { "S" } else { "N" });
        Crs {
            name,
            datum: Datum::WGS84,
            projection: Projection::new(ProjectionParams::utm(zone, south)).unwrap(),
        }
    }

    /// Forward: geographic (lon, lat) → projected (x, y).
    pub fn forward(&self, lon: f64, lat: f64) -> Result<(f64, f64)> {
        self.projection.forward(lon, lat)
    }

    /// Inverse: projected (x, y) → geographic (lon, lat).
    pub fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        self.projection.inverse(x, y)
    }

    /// Transform a point from this CRS into the target CRS.
    ///
    /// The transformation pipeline is:
    /// 1. Inverse project to geodetic (lon, lat, h=0) in source datum
    /// 2. Source datum geodetic → WGS84 geodetic (via configured datum transform)
    /// 3. WGS84 geodetic → target datum geodetic (via inverse datum transform)
    /// 4. Forward project in target CRS
    pub fn transform_to(&self, x: f64, y: f64, target: &Crs) -> Result<(f64, f64)> {
        self.transform_to_with_policy(x, y, target, CrsTransformPolicy::Strict)
    }

    /// Transform a point from this CRS into the target CRS using epoch context.
    ///
    /// Current behavior is intentionally identical to [`Crs::transform_to`].
    /// The epoch context is accepted as additive API scaffolding for future
    /// dynamic-datum workflows.
    pub fn transform_to_with_context(
        &self,
        x: f64,
        y: f64,
        target: &Crs,
        ctx: TransformEpochContext,
    ) -> Result<(f64, f64)> {
        let trace = self.transform_to_with_trace_and_context(
            x,
            y,
            target,
            CrsTransformPolicy::Strict,
            Some(ctx),
        )?;
        Ok((trace.x, trace.y))
    }

    /// Transform a point using an explicit operation code, with optional epoch context.
    ///
    /// This API validates registered operation metadata for source/target CRS
    /// compatibility, then delegates to the existing CRS transform pipeline.
    pub fn transform_to_with_operation(
        &self,
        x: f64,
        y: f64,
        target: &Crs,
        operation_code: u32,
        ctx: Option<TransformEpochContext>,
    ) -> Result<(f64, f64)> {
        let op = get_coordinate_operation(operation_code)?.ok_or_else(|| {
            ProjectionError::DatumError(format!(
                "coordinate operation {operation_code} is not registered"
            ))
        })?;

        if let Some(src_code) = epsg_code_from_crs_name(&self.name) {
            if src_code != op.source_crs_code {
                return Err(ProjectionError::DatumError(format!(
                    "operation {operation_code} source CRS mismatch: expected {}, got {}",
                    op.source_crs_code, src_code
                )));
            }
        }

        if let Some(dst_code) = epsg_code_from_crs_name(&target.name) {
            if dst_code != op.target_crs_code {
                return Err(ProjectionError::DatumError(format!(
                    "operation {operation_code} target CRS mismatch: expected {}, got {}",
                    op.target_crs_code, dst_code
                )));
            }
        }

        match ctx {
            Some(ctx) => self.transform_to_with_context(x, y, target, ctx),
            None => self.transform_to(x, y, target),
        }
    }

    /// Transform a point using a preferred operation for the CRS pair when available.
    ///
    /// Fallback behavior: when no preferred operation mapping is available, this
    /// delegates to the standard transform path.
    pub fn transform_to_with_preferred_operation(
        &self,
        x: f64,
        y: f64,
        target: &Crs,
        ctx: Option<TransformEpochContext>,
    ) -> Result<(f64, f64)> {
        self.transform_to_with_preferred_operation_and_policy(
            x,
            y,
            target,
            ctx,
            PreferredOperationPolicy::default(),
        )
    }

    /// Transform a point using a preferred operation for the CRS pair when available,
    /// using an explicit preferred-operation policy.
    ///
    /// Fallback behavior: when no preferred operation mapping is available under
    /// the given policy, this delegates to the standard transform path.
    pub fn transform_to_with_preferred_operation_and_policy(
        &self,
        x: f64,
        y: f64,
        target: &Crs,
        ctx: Option<TransformEpochContext>,
        preferred_op_policy: PreferredOperationPolicy,
    ) -> Result<(f64, f64)> {
        let src_code = epsg_code_from_crs_name(&self.name);
        let dst_code = epsg_code_from_crs_name(&target.name);

        if let (Some(src), Some(dst)) = (src_code, dst_code) {
            if let Some(op_def) =
                preferred_operation_for_crs_pair_with_policy(src, dst, preferred_op_policy)
            {
                register_coordinate_operation(op_def.clone())?;
                return self.transform_to_with_operation(
                    x,
                    y,
                    target,
                    op_def.operation_code,
                    ctx,
                );
            }
        }

        match ctx {
            Some(ctx) => self.transform_to_with_context(x, y, target, ctx),
            None => self.transform_to(x, y, target),
        }
    }

    /// Forward-project a batch of (lon, lat) pairs in degrees to (x, y) in this CRS.
    ///
    /// Results are returned in the same order as `points`. Each entry is
    /// `Ok((x, y))` on success or an error if a particular point failed.
    pub fn forward_many(&self, points: &[(f64, f64)]) -> Vec<Result<(f64, f64)>> {
        points.iter().map(|&(lon, lat)| self.forward(lon, lat)).collect()
    }

    /// Inverse-project a batch of (x, y) pairs to geographic (lon, lat) in degrees.
    ///
    /// Results are returned in the same order as `points`.
    pub fn inverse_many(&self, points: &[(f64, f64)]) -> Vec<Result<(f64, f64)>> {
        points.iter().map(|&(x, y)| self.inverse(x, y)).collect()
    }

    /// Transform a batch of (x, y) points from this CRS into `target`.
    ///
    /// Results are returned in the same order as `points`.
    pub fn transform_to_many(&self, points: &[(f64, f64)], target: &Crs) -> Vec<Result<(f64, f64)>> {
        points.iter().map(|&(x, y)| self.transform_to(x, y, target)).collect()
    }

    /// Forward-project a batch of (lon, lat) pairs in parallel using Rayon.
    ///
    /// Requires the `parallel` crate feature. Results are in the same order as
    /// `points`; each entry is `Ok((x, y))` or an error for that point.
    #[cfg(feature = "parallel")]
    pub fn forward_many_par(&self, points: &[(f64, f64)]) -> Vec<Result<(f64, f64)>> {
        use rayon::prelude::*;
        points.par_iter().map(|&(lon, lat)| self.forward(lon, lat)).collect()
    }

    /// Inverse-project a batch of (x, y) pairs in parallel using Rayon.
    ///
    /// Requires the `parallel` crate feature.
    #[cfg(feature = "parallel")]
    pub fn inverse_many_par(&self, points: &[(f64, f64)]) -> Vec<Result<(f64, f64)>> {
        use rayon::prelude::*;
        points.par_iter().map(|&(x, y)| self.inverse(x, y)).collect()
    }

    /// Transform a batch of (x, y) points from this CRS into `target` in parallel
    /// using Rayon.
    ///
    /// Requires the `parallel` crate feature.
    #[cfg(feature = "parallel")]
    pub fn transform_to_many_par(&self, points: &[(f64, f64)], target: &Crs) -> Vec<Result<(f64, f64)>> {
        use rayon::prelude::*;
        points.par_iter().map(|&(x, y)| self.transform_to(x, y, target)).collect()
    }

    /// Transform a 3D point from this CRS into the target CRS.
    ///
    /// For projected/geographic CRSes, `z` is treated as ellipsoidal height.
    /// For geocentric CRS, inputs/outputs are ECEF XYZ meters.
    pub fn transform_to_3d(&self, x: f64, y: f64, z: f64, target: &Crs) -> Result<(f64, f64, f64)> {
        self.transform_to_3d_with_policy_and_context(
            x,
            y,
            z,
            target,
            CrsTransformPolicy::Strict,
            None,
        )
    }

    /// Transform a 3D point from this CRS into the target CRS using epoch context.
    ///
    /// Current behavior is intentionally identical to [`Crs::transform_to_3d`].
    /// The epoch context is accepted as additive API scaffolding for future
    /// dynamic-datum workflows.
    pub fn transform_to_3d_with_context(
        &self,
        x: f64,
        y: f64,
        z: f64,
        target: &Crs,
        ctx: TransformEpochContext,
    ) -> Result<(f64, f64, f64)> {
        self.transform_to_3d_with_policy_and_context(
            x,
            y,
            z,
            target,
            CrsTransformPolicy::Strict,
            Some(ctx),
        )
    }

    /// Transform a 3D point using an explicit operation code, with optional epoch context.
    pub fn transform_to_3d_with_operation(
        &self,
        x: f64,
        y: f64,
        z: f64,
        target: &Crs,
        operation_code: u32,
        ctx: Option<TransformEpochContext>,
    ) -> Result<(f64, f64, f64)> {
        let op = get_coordinate_operation(operation_code)?.ok_or_else(|| {
            ProjectionError::DatumError(format!(
                "coordinate operation {operation_code} is not registered"
            ))
        })?;

        if let Some(src_code) = epsg_code_from_crs_name(&self.name) {
            if src_code != op.source_crs_code {
                return Err(ProjectionError::DatumError(format!(
                    "operation {operation_code} source CRS mismatch: expected {}, got {}",
                    op.source_crs_code, src_code
                )));
            }
        }

        if let Some(dst_code) = epsg_code_from_crs_name(&target.name) {
            if dst_code != op.target_crs_code {
                return Err(ProjectionError::DatumError(format!(
                    "operation {operation_code} target CRS mismatch: expected {}, got {}",
                    op.target_crs_code, dst_code
                )));
            }
        }

        match ctx {
            Some(ctx) => self.transform_to_3d_with_context(x, y, z, target, ctx),
            None => self.transform_to_3d(x, y, z, target),
        }
    }

    /// Transform a 3D point using a preferred operation for the CRS pair when available.
    ///
    /// Fallback behavior: when no preferred operation mapping is available, this
    /// delegates to the standard 3D transform path.
    pub fn transform_to_3d_with_preferred_operation(
        &self,
        x: f64,
        y: f64,
        z: f64,
        target: &Crs,
        ctx: Option<TransformEpochContext>,
    ) -> Result<(f64, f64, f64)> {
        self.transform_to_3d_with_preferred_operation_and_policy(
            x,
            y,
            z,
            target,
            ctx,
            PreferredOperationPolicy::default(),
        )
    }

    /// Transform a 3D point using a preferred operation for the CRS pair when available,
    /// using an explicit preferred-operation policy.
    ///
    /// Fallback behavior: when no preferred operation mapping is available under
    /// the given policy, this delegates to the standard 3D transform path.
    pub fn transform_to_3d_with_preferred_operation_and_policy(
        &self,
        x: f64,
        y: f64,
        z: f64,
        target: &Crs,
        ctx: Option<TransformEpochContext>,
        preferred_op_policy: PreferredOperationPolicy,
    ) -> Result<(f64, f64, f64)> {
        let src_code = epsg_code_from_crs_name(&self.name);
        let dst_code = epsg_code_from_crs_name(&target.name);

        if let (Some(src), Some(dst)) = (src_code, dst_code) {
            if let Some(op_def) =
                preferred_operation_for_crs_pair_with_policy(src, dst, preferred_op_policy)
            {
                register_coordinate_operation(op_def.clone())?;
                return self.transform_to_3d_with_operation(
                    x,
                    y,
                    z,
                    target,
                    op_def.operation_code,
                    ctx,
                );
            }
        }

        match ctx {
            Some(ctx) => self.transform_to_3d_with_context(x, y, z, target, ctx),
            None => self.transform_to_3d(x, y, z, target),
        }
    }

    /// Transform a 3D point while explicitly preserving horizontal context in mixed
    /// Vertical <-> Geographic/Projected workflows.
    ///
    /// This is an opt-in API for cases where horizontal coordinates are externally
    /// managed and should be carried through unchanged while only vertical context
    /// is transformed (currently passthrough until vertical datum models are added).
    ///
    /// Rules:
    /// - Vertical <-> Geographic/Projected: returns `(x, y, z)` unchanged.
    /// - Vertical <-> Vertical: returns `(x, y, z)` unchanged.
    /// - Any Geocentric <-> Vertical combination: returns `UnsupportedProjection`.
    /// - All other combinations delegate to [`Crs::transform_to_3d`].
    pub fn transform_to_3d_preserve_horizontal(
        &self,
        x: f64,
        y: f64,
        z: f64,
        target: &Crs,
    ) -> Result<(f64, f64, f64)> {
        self.transform_to_3d_preserve_horizontal_with_policy(
            x,
            y,
            z,
            target,
            CrsTransformPolicy::Strict,
        )
    }

    /// Policy-enabled variant of [`Crs::transform_to_3d_preserve_horizontal`].
    pub fn transform_to_3d_preserve_horizontal_with_policy(
        &self,
        x: f64,
        y: f64,
        z: f64,
        target: &Crs,
        policy: CrsTransformPolicy,
    ) -> Result<(f64, f64, f64)> {
        let source_is_vertical = matches!(self.projection.params().kind, ProjectionKind::Vertical);
        let target_is_vertical = matches!(target.projection.params().kind, ProjectionKind::Vertical);
        let source_is_geocentric = matches!(self.projection.params().kind, ProjectionKind::Geocentric);
        let target_is_geocentric = matches!(target.projection.params().kind, ProjectionKind::Geocentric);

        if (source_is_vertical && target_is_geocentric) || (source_is_geocentric && target_is_vertical) {
            return Err(crate::error::ProjectionError::UnsupportedProjection(
                "vertical CRS cannot preserve horizontal context with geocentric CRS"
                    .to_string(),
            ));
        }

        if source_is_vertical || target_is_vertical {
            // Phase-2 behavior: until vertical datum/geoid models are implemented,
            // preserve both horizontal context and height for explicit mixed-mode calls.
            return Ok((x, y, z));
        }

        self.transform_to_3d_with_policy(x, y, z, target, policy)
    }

    /// Preserve horizontal context and apply source/target vertical offsets to `z`.
    ///
    /// This supports integrating external vertical models (e.g., geoid undulation grids)
    /// without embedding those datasets directly in the crate yet.
    ///
    /// Offset convention:
    /// - `source_to_ellipsoidal_m`: add to source height to get ellipsoidal height.
    /// - `target_to_ellipsoidal_m`: subtract from ellipsoidal height to get target height.
    ///
    /// Formula:
    /// - `h_ellps = z + source_to_ellipsoidal_m`
    /// - `z_out = h_ellps - target_to_ellipsoidal_m`
    pub fn transform_to_3d_preserve_horizontal_with_vertical_offsets(
        &self,
        x: f64,
        y: f64,
        z: f64,
        target: &Crs,
        source_to_ellipsoidal_m: f64,
        target_to_ellipsoidal_m: f64,
    ) -> Result<(f64, f64, f64)> {
        self.transform_to_3d_preserve_horizontal_with_vertical_offsets_and_policy(
            x,
            y,
            z,
            target,
            source_to_ellipsoidal_m,
            target_to_ellipsoidal_m,
            CrsTransformPolicy::Strict,
        )
    }

    /// Policy-enabled variant of
    /// [`Crs::transform_to_3d_preserve_horizontal_with_vertical_offsets`].
    pub fn transform_to_3d_preserve_horizontal_with_vertical_offsets_and_policy(
        &self,
        x: f64,
        y: f64,
        z: f64,
        target: &Crs,
        source_to_ellipsoidal_m: f64,
        target_to_ellipsoidal_m: f64,
        policy: CrsTransformPolicy,
    ) -> Result<(f64, f64, f64)> {
        let (x_out, y_out, z_passthrough) =
            self.transform_to_3d_preserve_horizontal_with_policy(x, y, z, target, policy)?;

        let h_ellps = z_passthrough + source_to_ellipsoidal_m;
        let z_out = h_ellps - target_to_ellipsoidal_m;

        Ok((x_out, y_out, z_out))
    }

    /// Preserve horizontal context and apply vertical offsets resolved by a provider.
    pub fn transform_to_3d_preserve_horizontal_with_provider<P: VerticalOffsetProvider>(
        &self,
        x: f64,
        y: f64,
        z: f64,
        target: &Crs,
        provider: &P,
    ) -> Result<(f64, f64, f64)> {
        self.transform_to_3d_preserve_horizontal_with_provider_and_policy(
            x,
            y,
            z,
            target,
            provider,
            CrsTransformPolicy::Strict,
        )
    }

    /// Policy-enabled variant of [`Crs::transform_to_3d_preserve_horizontal_with_provider`].
    pub fn transform_to_3d_preserve_horizontal_with_provider_and_policy<P: VerticalOffsetProvider>(
        &self,
        x: f64,
        y: f64,
        z: f64,
        target: &Crs,
        provider: &P,
        policy: CrsTransformPolicy,
    ) -> Result<(f64, f64, f64)> {
        let (x_out, y_out, z_passthrough) =
            self.transform_to_3d_preserve_horizontal_with_policy(x, y, z, target, policy)?;

        let (source_to_ellipsoidal_m, target_to_ellipsoidal_m) =
            provider.offsets(x_out, y_out, self, target)?;

        let h_ellps = z_passthrough + source_to_ellipsoidal_m;
        let z_out = h_ellps - target_to_ellipsoidal_m;

        Ok((x_out, y_out, z_out))
    }

    /// Transform a 3D point from this CRS into the target CRS using a transform policy.
    pub fn transform_to_3d_with_policy(
        &self,
        x: f64,
        y: f64,
        z: f64,
        target: &Crs,
        policy: CrsTransformPolicy,
    ) -> Result<(f64, f64, f64)> {
        self.transform_to_3d_with_policy_and_context(x, y, z, target, policy, None)
    }

    fn transform_to_3d_with_policy_and_context(
        &self,
        x: f64,
        y: f64,
        z: f64,
        target: &Crs,
        policy: CrsTransformPolicy,
        ctx: Option<TransformEpochContext>,
    ) -> Result<(f64, f64, f64)> {
        let datum_policy = match policy {
            CrsTransformPolicy::Strict => DatumTransformPolicy::Strict,
            CrsTransformPolicy::Auto => DatumTransformPolicy::Strict,
            CrsTransformPolicy::FallbackToIdentityGridShift => {
                DatumTransformPolicy::FallbackToIdentityGridShift
            }
        };

        let source_is_vertical = matches!(self.projection.params().kind, ProjectionKind::Vertical);
        let target_is_vertical = matches!(target.projection.params().kind, ProjectionKind::Vertical);

        // Default strict behavior for vertical CRS: keep mixed-mode transformations explicit
        // via `transform_to_3d_preserve_horizontal` to avoid accidental ambiguity.
        if source_is_vertical || target_is_vertical {
            if source_is_vertical && target_is_vertical {
                // Treat x/y as geographic lon/lat context for optional automatic
                // vertical offset application when EPSG<->grid mappings are available.
                let src_code = epsg_code_from_crs_name(&self.name);
                let dst_code = epsg_code_from_crs_name(&target.name);

                if let (Some(src), Some(dst)) = (src_code, dst_code) {
                    let src_off = sample_vertical_offset_with_policy(src, x, y, policy)?;
                    let dst_off = sample_vertical_offset_with_policy(dst, x, y, policy)?;

                    if let (Some(source_to_ellipsoidal_m), Some(target_to_ellipsoidal_m)) =
                        (src_off, dst_off)
                    {
                        let h_ellps = z + source_to_ellipsoidal_m;
                        let z_out = h_ellps - target_to_ellipsoidal_m;
                        return Ok((x, y, z_out));
                    }
                }

                return Ok((x, y, z));
            }

            return Err(crate::error::ProjectionError::UnsupportedProjection(
                "vertical CRS can only transform to another vertical CRS in transform_to_3d"
                    .to_string(),
            ));
        }

        // Step 1: source CRS coordinates -> source datum geodetic (radians + height)
        let (src_lat_rad, src_lon_rad, src_h) = match self.projection.params().kind {
            ProjectionKind::Geocentric => {
                ecef_to_geodetic(x, y, z, &self.datum.ellipsoid)
            }
            _ => {
                let (lon_deg, lat_deg) = self.projection.inverse(x, y)?;
                (to_radians(lat_deg), to_radians(lon_deg), z)
            }
        };

        if matches!(policy, CrsTransformPolicy::Auto)
            && datums_are_ballpark_equivalent(&self.datum, &target.datum)
        {
            return match target.projection.params().kind {
                ProjectionKind::Geocentric => Ok(geodetic_to_ecef(
                    src_lat_rad,
                    src_lon_rad,
                    src_h,
                    &target.datum.ellipsoid,
                )),
                _ => {
                    let (out_x, out_y) = target
                        .projection
                        .forward(to_degrees(src_lon_rad), to_degrees(src_lat_rad))?;
                    Ok((out_x, out_y, src_h))
                }
            };
        }

        // Step 2: source datum geodetic -> WGS84 geodetic
        let (wgs_lat, wgs_lon, wgs_h) = match ctx {
            Some(ctx) => self.datum.to_wgs84_geodetic_with_policy_and_context(
                src_lat_rad,
                src_lon_rad,
                src_h,
                datum_policy,
                ctx,
            )?,
            None => self
                .datum
                .to_wgs84_geodetic_with_policy(src_lat_rad, src_lon_rad, src_h, datum_policy)?,
        };

        // Step 3: WGS84 geodetic -> target datum geodetic
        let (dst_lat, dst_lon, dst_h) = match ctx {
            Some(ctx) => target.datum.from_wgs84_geodetic_with_policy_and_context(
                wgs_lat,
                wgs_lon,
                wgs_h,
                datum_policy,
                ctx,
            )?,
            None => target
                .datum
                .from_wgs84_geodetic_with_policy(wgs_lat, wgs_lon, wgs_h, datum_policy)?,
        };

        // Step 4: target datum geodetic -> target CRS coordinates
        match target.projection.params().kind {
            ProjectionKind::Geocentric => Ok(geodetic_to_ecef(dst_lat, dst_lon, dst_h, &target.datum.ellipsoid)),
            _ => {
                let (out_x, out_y) = target
                    .projection
                    .forward(to_degrees(dst_lon), to_degrees(dst_lat))?;
                Ok((out_x, out_y, dst_h))
            }
        }
    }

    /// Transform a point from this CRS into the target CRS using a transform policy.
    pub fn transform_to_with_policy(
        &self,
        x: f64,
        y: f64,
        target: &Crs,
        policy: CrsTransformPolicy,
    ) -> Result<(f64, f64)> {
        let trace = self.transform_to_with_trace_and_context(x, y, target, policy, None)?;
        Ok((trace.x, trace.y))
    }

    /// Transform a point and return output coordinates along with selected datum-grid metadata.
    pub fn transform_to_with_trace(
        &self,
        x: f64,
        y: f64,
        target: &Crs,
        policy: CrsTransformPolicy,
    ) -> Result<CrsTransformTrace> {
        self.transform_to_with_trace_and_context(x, y, target, policy, None)
    }

    fn transform_to_with_trace_and_context(
        &self,
        x: f64,
        y: f64,
        target: &Crs,
        policy: CrsTransformPolicy,
        ctx: Option<TransformEpochContext>,
    ) -> Result<CrsTransformTrace> {
        let datum_policy = match policy {
            CrsTransformPolicy::Strict => DatumTransformPolicy::Strict,
            CrsTransformPolicy::Auto => DatumTransformPolicy::Strict,
            CrsTransformPolicy::FallbackToIdentityGridShift => {
                DatumTransformPolicy::FallbackToIdentityGridShift
            }
        };

        // Step 1: unproject
        let (lon_deg, lat_deg) = self.projection.inverse(x, y)?;
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);

        if matches!(policy, CrsTransformPolicy::Auto)
            && datums_are_ballpark_equivalent(&self.datum, &target.datum)
        {
            let (out_x, out_y) = target.projection.forward(lon_deg, lat_deg)?;
            return Ok(CrsTransformTrace {
                x: out_x,
                y: out_y,
                source_grid: None,
                target_grid: None,
            });
        }

        // Step 2: source datum geodetic → WGS84 geodetic
        let src_trace = self
            .datum
            .to_wgs84_geodetic_with_policy_and_trace(lat, lon, 0.0, datum_policy, ctx)?;

        // Step 3: WGS84 geodetic → target datum geodetic
        let dst_trace = target
            .datum
            .from_wgs84_geodetic_with_policy_and_trace(
                src_trace.lat_rad,
                src_trace.lon_rad,
                src_trace.h,
                datum_policy,
                ctx,
            )?;

        // Step 4: forward project in target CRS
        let (out_x, out_y) = target
            .projection
            .forward(to_degrees(dst_trace.lon_rad), to_degrees(dst_trace.lat_rad))?;

        Ok(CrsTransformTrace {
            x: out_x,
            y: out_y,
            source_grid: src_trace.selected_grid,
            target_grid: dst_trace.selected_grid,
        })
    }

    /// Transform a point and return trace metadata using strict datum behavior.
    pub fn transform_to_with_trace_strict(
        &self,
        x: f64,
        y: f64,
        target: &Crs,
    ) -> Result<CrsTransformTrace> {
        self.transform_to_with_trace(x, y, target, CrsTransformPolicy::Strict)
    }

    /// Batch transform multiple coordinate pairs from this CRS to the target CRS.
    ///
    /// Transforms an array of (x, y) coordinates in-place. For every successful
    /// transformation, the corresponding coordinate is updated; on error, it remains
    /// unchanged and the error is recorded in the output Vec.
    ///
    /// Uses SIMD acceleration (via the `wide` crate) where applicable for Helmert
    /// datum transforms, falling back to scalar operations otherwise.
    ///
    /// # Arguments
    /// - `coords`: Mutable slice of (x, y) tuples to transform in-place.
    /// - `target`: Target CRS.
    ///
    /// # Returns
    /// A `Vec<Option<Result<()>>>` where `None` indicates a successful transform,
    /// and `Some(Err(_))` indicates a transform error for that coordinate.
    pub fn transform_to_batch(
        &self,
        coords: &mut [(f64, f64)],
        target: &Crs,
    ) -> Vec<Option<Result<()>>> {
        if let Some(results) = self.try_transform_to_batch_geographic_simd(coords, target) {
            return results;
        }
        if let Some(results) = self.try_transform_to_batch_projected_simd(coords, target) {
            return results;
        }

        coords
            .iter_mut()
            .map(|(x, y)| {
                match self.transform_to(*x, *y, target) {
                    Ok((new_x, new_y)) => {
                        *x = new_x;
                        *y = new_y;
                        None
                    }
                    Err(e) => Some(Err(e)),
                }
            })
            .collect()
    }

    /// SIMD fast path for projected → projected batch transforms.
    ///
    /// Applies when both source and target are proper projected CRS (not Geographic,
    /// Geocentric, or Vertical) and both datums support ECEF batch SIMD (Helmert or
    /// WGS84 identity). The projection inverse/forward steps remain scalar per-point
    /// but the datum ECEF leg is vectorized in groups of 4, which is the dominant
    /// cost in cross-datum projected workflows.
    fn try_transform_to_batch_projected_simd(
        &self,
        coords: &mut [(f64, f64)],
        target: &Crs,
    ) -> Option<Vec<Option<Result<()>>>> {
        let src_kind = &self.projection.params().kind;
        let dst_kind = &target.projection.params().kind;
        if matches!(
            src_kind,
            ProjectionKind::Geographic | ProjectionKind::Geocentric | ProjectionKind::Vertical
        ) || matches!(
            dst_kind,
            ProjectionKind::Geographic | ProjectionKind::Geocentric | ProjectionKind::Vertical
        ) || !self.datum.supports_ecef_batch_simd()
            || !target.datum.supports_ecef_batch_simd()
        {
            return None;
        }

        let n = coords.len();
        let mut results: Vec<Option<Result<()>>> = vec![None; n];
        let full_chunks = n / 4;

        for c in 0..full_chunks {
            let base = c * 4;
            let mut x4 = [0.0_f64; 4];
            let mut y4 = [0.0_f64; 4];
            let mut z4 = [0.0_f64; 4];
            let mut chunk_ok = true;

            // Inverse project each of 4 points → ECEF
            for lane in 0..4 {
                let (cx, cy) = coords[base + lane];
                match self.projection.inverse(cx, cy) {
                    Ok((lon_deg, lat_deg)) => {
                        let lon_rad = to_radians(lon_deg);
                        let lat_rad = to_radians(lat_deg);
                        let (xe, ye, ze) =
                            geodetic_to_ecef(lat_rad, lon_rad, 0.0, &self.datum.ellipsoid);
                        x4[lane] = xe;
                        y4[lane] = ye;
                        z4[lane] = ze;
                    }
                    Err(e) => {
                        results[base + lane] = Some(Err(e));
                        chunk_ok = false;
                    }
                }
            }

            if !chunk_ok {
                // Retry each lane that hasn't errored yet with the scalar path
                for lane in 0..4 {
                    if results[base + lane].is_none() {
                        let (cx, cy) = coords[base + lane];
                        match self.transform_to(cx, cy, target) {
                            Ok((nx, ny)) => {
                                coords[base + lane] = (nx, ny);
                            }
                            Err(e) => {
                                results[base + lane] = Some(Err(e));
                            }
                        }
                    }
                }
                continue;
            }

            // Batch ECEF datum transform (SIMD for 4 lanes)
            let (xw, yw, zw) = self
                .datum
                .to_wgs84_ecef_batch4(&x4, &y4, &z4)
                .expect("projected batch fast path datum prechecked");
            let (xt, yt, zt) = target
                .datum
                .from_wgs84_ecef_batch4(&xw, &yw, &zw)
                .expect("projected batch fast path datum prechecked");

            // Forward project each of 4 results in the target CRS
            for lane in 0..4 {
                let (lat_rad, lon_rad, _h) =
                    ecef_to_geodetic(xt[lane], yt[lane], zt[lane], &target.datum.ellipsoid);
                match target
                    .projection
                    .forward(to_degrees(lon_rad), to_degrees(lat_rad))
                {
                    Ok((out_x, out_y)) => {
                        coords[base + lane] = (out_x, out_y);
                    }
                    Err(e) => {
                        results[base + lane] = Some(Err(e));
                    }
                }
            }
        }

        // Scalar remainder (< 4 points)
        for i in (full_chunks * 4)..n {
            let (cx, cy) = coords[i];
            match (|| -> Result<(f64, f64)> {
                let (lon_deg, lat_deg) = self.projection.inverse(cx, cy)?;
                let lon_rad = to_radians(lon_deg);
                let lat_rad = to_radians(lat_deg);
                let (xe, ye, ze) =
                    geodetic_to_ecef(lat_rad, lon_rad, 0.0, &self.datum.ellipsoid);
                let (xw, yw, zw) = self.datum.to_wgs84_ecef(xe, ye, ze)?;
                let (xt, yt, zt) = target.datum.from_wgs84_ecef(xw, yw, zw)?;
                let (dst_lat, dst_lon, _h) =
                    ecef_to_geodetic(xt, yt, zt, &target.datum.ellipsoid);
                target
                    .projection
                    .forward(to_degrees(dst_lon), to_degrees(dst_lat))
            })() {
                Ok((nx, ny)) => {
                    coords[i] = (nx, ny);
                }
                Err(e) => {
                    results[i] = Some(Err(e));
                }
            }
        }

        Some(results)
    }

    fn try_transform_to_batch_geographic_simd(
        &self,
        coords: &mut [(f64, f64)],
        target: &Crs,
    ) -> Option<Vec<Option<Result<()>>>> {
        if !matches!(self.projection.params().kind, ProjectionKind::Geographic)
            || !matches!(target.projection.params().kind, ProjectionKind::Geographic)
            || !self.datum.supports_ecef_batch_simd()
            || !target.datum.supports_ecef_batch_simd()
        {
            return None;
        }

        let results = vec![None; coords.len()];
        let mut chunks = coords.chunks_exact_mut(4);

        for chunk in &mut chunks {
            let mut x4 = [0.0_f64; 4];
            let mut y4 = [0.0_f64; 4];
            let mut z4 = [0.0_f64; 4];

            for lane in 0..4 {
                let lon_rad = to_radians(chunk[lane].0);
                let lat_rad = to_radians(chunk[lane].1);
                let (x, y, z) = geodetic_to_ecef(lat_rad, lon_rad, 0.0, &self.datum.ellipsoid);
                x4[lane] = x;
                y4[lane] = y;
                z4[lane] = z;
            }

            let (xw, yw, zw) = self
                .datum
                .to_wgs84_ecef_batch4(&x4, &y4, &z4)
                .expect("batch geographic fast path prechecked");
            let (xt, yt, zt) = target
                .datum
                .from_wgs84_ecef_batch4(&xw, &yw, &zw)
                .expect("batch geographic fast path prechecked");

            for lane in 0..4 {
                let (lat_rad, lon_rad, _h) = ecef_to_geodetic(
                    xt[lane],
                    yt[lane],
                    zt[lane],
                    &target.datum.ellipsoid,
                );
                chunk[lane] = (to_degrees(lon_rad), to_degrees(lat_rad));
            }
        }

        for coord in chunks.into_remainder() {
            let lon_rad = to_radians(coord.0);
            let lat_rad = to_radians(coord.1);
            let (x, y, z) = geodetic_to_ecef(lat_rad, lon_rad, 0.0, &self.datum.ellipsoid);
            let (xw, yw, zw) = self
                .datum
                .to_wgs84_ecef(x, y, z)
                .and_then(|(a, b, c)| target.datum.from_wgs84_ecef(a, b, c))
                .expect("scalar geographic fast path prechecked");
            let (dst_lat, dst_lon, _dst_h) = ecef_to_geodetic(xw, yw, zw, &target.datum.ellipsoid);
            *coord = (to_degrees(dst_lon), to_degrees(dst_lat));
        }

        Some(results)
    }

    /// Batch transform 3D coordinate pairs from this CRS to the target CRS.
    ///
    /// Transforms an array of (x, y, z) coordinates in-place. Similar semantics
    /// to [`Crs::transform_to_batch`] but for 3D points.
    pub fn transform_to_3d_batch(
        &self,
        coords: &mut [(f64, f64, f64)],
        target: &Crs,
    ) -> Vec<Option<Result<()>>> {
        if let Some(results) = self.try_transform_to_3d_batch_geocentric_simd(coords, target) {
            return results;
        }

        coords
            .iter_mut()
            .map(|(x, y, z)| {
                match self.transform_to_3d(*x, *y, *z, target) {
                    Ok((new_x, new_y, new_z)) => {
                        *x = new_x;
                        *y = new_y;
                        *z = new_z;
                        None
                    }
                    Err(e) => Some(Err(e)),
                }
            })
            .collect()
    }

    fn try_transform_to_3d_batch_geocentric_simd(
        &self,
        coords: &mut [(f64, f64, f64)],
        target: &Crs,
    ) -> Option<Vec<Option<Result<()>>>> {
        if !matches!(self.projection.params().kind, ProjectionKind::Geocentric)
            || !matches!(target.projection.params().kind, ProjectionKind::Geocentric)
            || !self.datum.supports_ecef_batch_simd()
            || !target.datum.supports_ecef_batch_simd()
        {
            return None;
        }

        let results = vec![None; coords.len()];
        let mut chunks = coords.chunks_exact_mut(4);

        for chunk in &mut chunks {
            let x4 = [chunk[0].0, chunk[1].0, chunk[2].0, chunk[3].0];
            let y4 = [chunk[0].1, chunk[1].1, chunk[2].1, chunk[3].1];
            let z4 = [chunk[0].2, chunk[1].2, chunk[2].2, chunk[3].2];

            let (xw, yw, zw) = self
                .datum
                .to_wgs84_ecef_batch4(&x4, &y4, &z4)
                .expect("batch ECEF fast path prechecked");
            let (xt, yt, zt) = target
                .datum
                .from_wgs84_ecef_batch4(&xw, &yw, &zw)
                .expect("batch ECEF fast path prechecked");

            for lane in 0..4 {
                chunk[lane] = (xt[lane], yt[lane], zt[lane]);
            }
        }

        for coord in chunks.into_remainder() {
            let (xt, yt, zt) = self
                .datum
                .to_wgs84_ecef(coord.0, coord.1, coord.2)
                .and_then(|(xw, yw, zw)| target.datum.from_wgs84_ecef(xw, yw, zw))
                .expect("scalar ECEF fast path prechecked");
            *coord = (xt, yt, zt);
        }

        Some(results)
    }
}

impl std::fmt::Debug for Crs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Crs")
            .field("name", &self.name)
            .field("datum", &self.datum.name)
            .field("projection", &self.projection.name())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operations::coordinate_operation_test_guard;
    use crate::{
        CoordinateOperationDef,
        Datum, DynamicGridShiftGrid, DynamicGridShiftSample, Ellipsoid, Projection,
        OperationMethod,
        PreferredOperationPolicy,
        ProjectionParams, TransformEpochContext, clear_coordinate_operations,
        preferred_operation_code_for_crs_pair,
        register_coordinate_operation, register_dynamic_grid, unregister_dynamic_grid,
        unregister_coordinate_operation,
    };
    use crate::datum::DatumTransform;

    fn wgs84_geocentric() -> Crs {
        Crs {
            name: "WGS 84 geocentric test CRS".to_string(),
            datum: Datum::WGS84,
            projection: Projection::new(
                ProjectionParams::new(ProjectionKind::Geocentric).with_ellipsoid(Ellipsoid::WGS84),
            )
            .expect("projection creation failed"),
        }
    }

    fn helmert_geocentric(datum: Datum, name: &str) -> Crs {
        let ellipsoid = datum.ellipsoid.clone();
        Crs {
            name: name.to_string(),
            datum,
            projection: Projection::new(
                ProjectionParams::new(ProjectionKind::Geocentric).with_ellipsoid(ellipsoid),
            )
            .expect("projection creation failed"),
        }
    }

    #[test]
    fn geocentric_batch_fast_path_matches_scalar_results() {
        let source = helmert_geocentric(Datum::ED50, "ED50 geocentric test CRS");
        let target = wgs84_geocentric();
        let original = vec![
            (3_987_654.25, 766_432.5, 4_966_789.0),
            (4_112_345.5, 612_345.75, 4_844_321.25),
            (3_854_210.0, 854_321.0, 5_102_468.5),
            (4_034_567.25, 701_234.5, 4_923_456.75),
            (4_010_000.0, 720_000.0, 4_950_000.0),
        ];

        let mut batched = original.clone();
        let batch_results = source.transform_to_3d_batch(&mut batched, &target);
        assert!(batch_results.iter().all(Option::is_none));

        for (expected, actual) in original
            .iter()
            .map(|&(x, y, z)| source.transform_to_3d(x, y, z, &target).unwrap())
            .zip(batched.iter())
        {
            assert!((expected.0 - actual.0).abs() < 1e-3);
            assert!((expected.1 - actual.1).abs() < 1e-3);
            assert!((expected.2 - actual.2).abs() < 1e-3);
        }
    }

    #[test]
    fn geographic_batch_fast_path_matches_scalar_results() {
        let source = Crs::from_epsg(4230).expect("ED50 geographic load failed");
        let target = Crs::from_epsg(4326).expect("WGS84 geographic load failed");
        let original = vec![
            (-3.7038, 40.4168),
            (2.3522, 48.8566),
            (13.4050, 52.5200),
            (-0.1276, 51.5074),
            (18.0686, 59.3293),
        ];

        let mut batched = original.clone();
        let batch_results = source.transform_to_batch(&mut batched, &target);
        assert!(batch_results.iter().all(Option::is_none));

        for (expected, actual) in original
            .iter()
            .map(|&(x, y)| source.transform_to(x, y, &target).unwrap())
            .zip(batched.iter())
        {
            assert!((expected.0 - actual.0).abs() < 1e-7);
            assert!((expected.1 - actual.1).abs() < 1e-7);
        }
    }

    #[test]
    fn projected_batch_fast_path_matches_scalar_results() {
        // Cross-datum: ED50 UTM 32N → WGS84 UTM 32N — exercises the ECEF Helmert
        // datum leg; correctness is confirmed by matching the per-point scalar path.
        let source = Crs::from_epsg(23032).expect("ED50 UTM 32N load failed");
        let target = Crs::from_epsg(32632).expect("WGS84 UTM 32N load failed");

        // 9 points: two full SIMD chunks of 4 + 1 remainder
        let original = vec![
            (500_000.0, 5_500_000.0),
            (505_000.0, 5_510_000.0),
            (495_000.0, 5_490_000.0),
            (510_000.0, 5_520_000.0),
            (488_000.0, 5_480_000.0),
            (520_000.0, 5_530_000.0),
            (476_000.0, 5_470_000.0),
            (530_000.0, 5_540_000.0),
            (465_000.0, 5_460_000.0),
        ];

        let mut batched = original.clone();
        let batch_results = source.transform_to_batch(&mut batched, &target);
        assert!(batch_results.iter().all(Option::is_none), "unexpected transform errors");

        for (expected, actual) in original
            .iter()
            .map(|&(x, y)| source.transform_to(x, y, &target).unwrap())
            .zip(batched.iter())
        {
            assert!(
                (expected.0 - actual.0).abs() < 0.1,
                "easting mismatch: scalar={}, batch={}",
                expected.0,
                actual.0
            );
            assert!(
                (expected.1 - actual.1).abs() < 0.1,
                "northing mismatch: scalar={}, batch={}",
                expected.1,
                actual.1
            );
        }

        // Same-datum: WGS84 UTM 32N → WGS84 UTM 33N — identity datum leg
        let src2 = Crs::from_epsg(32632).expect("WGS84 UTM 32N load failed");
        let dst2 = Crs::from_epsg(32633).expect("WGS84 UTM 33N load failed");
        let original2 = vec![
            (599_000.0, 5_810_000.0),
            (600_000.0, 5_820_000.0),
            (601_000.0, 5_830_000.0),
            (602_000.0, 5_840_000.0),
            (603_000.0, 5_850_000.0),
        ];
        let mut batched2 = original2.clone();
        let batch_results2 = src2.transform_to_batch(&mut batched2, &dst2);
        assert!(batch_results2.iter().all(Option::is_none), "same-datum errors");
        for (expected, actual) in original2
            .iter()
            .map(|&(x, y)| src2.transform_to(x, y, &dst2).unwrap())
            .zip(batched2.iter())
        {
            assert!((expected.0 - actual.0).abs() < 0.1);
            assert!((expected.1 - actual.1).abs() < 0.1);
        }
    }

    #[test]
    fn transform_to_with_context_matches_transform_to() {
        let source = Crs::from_epsg(4230).expect("ED50 geographic load failed");
        let target = Crs::from_epsg(4326).expect("WGS84 geographic load failed");
        let ctx = TransformEpochContext::at_epoch(2024.5);

        let (x_base, y_base) = source.transform_to(-3.7038, 40.4168, &target).unwrap();
        let (x_ctx, y_ctx) = source
            .transform_to_with_context(-3.7038, 40.4168, &target, ctx)
            .unwrap();

        assert!((x_base - x_ctx).abs() < 1e-12);
        assert!((y_base - y_ctx).abs() < 1e-12);
    }

    #[test]
    fn transform_to_3d_with_context_matches_transform_to_3d() {
        let source = Crs::from_epsg(4326).expect("WGS84 geographic load failed");
        let target = wgs84_geocentric();
        let ctx = TransformEpochContext::new(2024.5, Some(2010.0), Some(2020.0));

        let base = source.transform_to_3d(-75.0, 45.0, 123.4, &target).unwrap();
        let with_ctx = source
            .transform_to_3d_with_context(-75.0, 45.0, 123.4, &target, ctx)
            .unwrap();

        assert!((base.0 - with_ctx.0).abs() < 1e-9);
        assert!((base.1 - with_ctx.1).abs() < 1e-9);
        assert!((base.2 - with_ctx.2).abs() < 1e-6);
    }

    #[test]
    fn transform_to_with_context_dynamic_datum_requires_context() {
        let grid = DynamicGridShiftGrid::new(
            "CRS_DYN_GRID",
            2020.0,
            0.0,
            0.0,
            1.0,
            1.0,
            2,
            2,
            vec![DynamicGridShiftSample::new(1.0, -2.0, 0.5, -1.0); 4],
        )
        .unwrap();
        register_dynamic_grid(grid).unwrap();

        let src = Crs {
            name: "Dynamic source CRS".to_string(),
            datum: Datum {
                name: "dynamic-src",
                ellipsoid: Ellipsoid::WGS84,
                transform: DatumTransform::DynamicGridShift {
                    grid_name: "CRS_DYN_GRID",
                },
            },
            projection: Projection::new(ProjectionParams::new(ProjectionKind::Geographic)).unwrap(),
        };
        let dst = Crs {
            name: "WGS84 target CRS".to_string(),
            datum: Datum::WGS84,
            projection: Projection::new(ProjectionParams::new(ProjectionKind::Geographic)).unwrap(),
        };

        let err = src.transform_to(0.5, 0.5, &dst).unwrap_err();
        assert!(
            format!("{err}").to_ascii_lowercase().contains("requires transformepochcontext")
        );

        let _ = unregister_dynamic_grid("CRS_DYN_GRID");
    }

    #[test]
    fn transform_to_with_context_dynamic_datum_applies_epoch_shift() {
        let grid = DynamicGridShiftGrid::new(
            "CRS_DYN_GRID_CTX",
            2020.0,
            0.0,
            0.0,
            1.0,
            1.0,
            2,
            2,
            vec![DynamicGridShiftSample::new(1.0, -2.0, 0.5, -1.0); 4],
        )
        .unwrap();
        register_dynamic_grid(grid).unwrap();

        let src = Crs {
            name: "Dynamic source CRS".to_string(),
            datum: Datum {
                name: "dynamic-src",
                ellipsoid: Ellipsoid::WGS84,
                transform: DatumTransform::DynamicGridShift {
                    grid_name: "CRS_DYN_GRID_CTX",
                },
            },
            projection: Projection::new(ProjectionParams::new(ProjectionKind::Geographic)).unwrap(),
        };
        let dst = Crs {
            name: "WGS84 target CRS".to_string(),
            datum: Datum::WGS84,
            projection: Projection::new(ProjectionParams::new(ProjectionKind::Geographic)).unwrap(),
        };

        let ctx = TransformEpochContext::at_epoch(2022.0); // dt=+2 => dlon +2", dlat -4"
        let (lon_out, lat_out) = src
            .transform_to_with_context(0.5, 0.5, &dst, ctx)
            .unwrap();

        let dlon_sec = (lon_out - 0.5) * 3600.0;
        let dlat_sec = (lat_out - 0.5) * 3600.0;
        assert!((dlon_sec - 2.0).abs() < 1e-9);
        assert!((dlat_sec - (-4.0)).abs() < 1e-9);

        let _ = unregister_dynamic_grid("CRS_DYN_GRID_CTX");
    }

    #[test]
    fn transform_to_with_operation_validates_codes_and_routes() {
        clear_coordinate_operations().unwrap();

        let src = Crs::from_epsg(4326).unwrap();
        let dst = Crs::from_epsg(3857).unwrap();
        register_coordinate_operation(
            CoordinateOperationDef::new(999001, 4326, 3857, OperationMethod::DatumPipeline),
        )
        .unwrap();

        let via_op = src
            .transform_to_with_operation(10.0, 45.0, &dst, 999001, None)
            .unwrap();
        let base = src.transform_to(10.0, 45.0, &dst).unwrap();

        assert!((via_op.0 - base.0).abs() < 1e-9);
        assert!((via_op.1 - base.1).abs() < 1e-9);

        let _ = unregister_coordinate_operation(999001);
    }

    #[test]
    fn transform_to_with_operation_rejects_crs_mismatch() {
        clear_coordinate_operations().unwrap();

        let src = Crs::from_epsg(4326).unwrap();
        let dst = Crs::from_epsg(3857).unwrap();
        register_coordinate_operation(
            CoordinateOperationDef::new(999002, 4258, 3857, OperationMethod::DatumPipeline),
        )
        .unwrap();

        let err = src
            .transform_to_with_operation(10.0, 45.0, &dst, 999002, None)
            .unwrap_err();
        assert!(
            format!("{err}")
                .to_ascii_lowercase()
                .contains("source crs mismatch")
        );

        let _ = unregister_coordinate_operation(999002);
    }

    #[test]
    fn preferred_operation_mapping_exists_for_csrs_v3_to_v8_pair() {
        assert_eq!(preferred_operation_code_for_crs_pair(22317, 22817), Some(10715));
        assert_eq!(preferred_operation_code_for_crs_pair(22318, 22817), None);
    }

    #[test]
    fn transform_to_with_preferred_operation_falls_back_when_no_mapping() {
        let src = Crs::from_epsg(4326).unwrap();
        let dst = Crs::from_epsg(3857).unwrap();

        let via_pref = src
            .transform_to_with_preferred_operation(10.0, 45.0, &dst, None)
            .unwrap();
        let base = src.transform_to(10.0, 45.0, &dst).unwrap();

        assert!((via_pref.0 - base.0).abs() < 1e-9);
        assert!((via_pref.1 - base.1).abs() < 1e-9);
    }

    #[test]
    fn transform_to_with_preferred_operation_uses_registered_mapping() {
        let _guard = coordinate_operation_test_guard();
        clear_coordinate_operations().unwrap();

        let src = Crs::from_epsg(22317).unwrap();
        let dst = Crs::from_epsg(22817).unwrap();

        let via_pref = src
            .transform_to_with_preferred_operation(500_000.0, 5_500_000.0, &dst, None)
            .unwrap();
        let base = src.transform_to(500_000.0, 5_500_000.0, &dst).unwrap();

        assert!((via_pref.0 - base.0).abs() < 1e-9);
        assert!((via_pref.1 - base.1).abs() < 1e-9);
    }

    #[test]
    fn transform_to_with_preferred_operation_uses_registered_mapping_for_v6_v8() {
        let _guard = coordinate_operation_test_guard();
        clear_coordinate_operations().unwrap();

        let src = Crs::from_epsg(22617).unwrap();
        let dst = Crs::from_epsg(22817).unwrap();

        let via_pref = src
            .transform_to_with_preferred_operation(500_000.0, 5_500_000.0, &dst, None)
            .unwrap();
        let base = src.transform_to(500_000.0, 5_500_000.0, &dst).unwrap();

        assert!((via_pref.0 - base.0).abs() < 1e-9);
        assert!((via_pref.1 - base.1).abs() < 1e-9);
    }

    #[test]
    fn transform_to_with_preferred_operation_uses_registered_mapping_for_v5_v8() {
        let _guard = coordinate_operation_test_guard();
        clear_coordinate_operations().unwrap();

        let src = Crs::from_epsg(22517).unwrap();
        let dst = Crs::from_epsg(22817).unwrap();

        let via_pref = src
            .transform_to_with_preferred_operation(500_000.0, 5_500_000.0, &dst, None)
            .unwrap();
        let base = src.transform_to(500_000.0, 5_500_000.0, &dst).unwrap();

        assert!((via_pref.0 - base.0).abs() < 1e-9);
        assert!((via_pref.1 - base.1).abs() < 1e-9);
    }

    #[test]
    fn transform_to_3d_with_preferred_operation_falls_back_when_no_mapping() {
        let src = Crs::from_epsg(4326).unwrap();
        let dst = Crs::from_epsg(3857).unwrap();

        let via_pref = src
            .transform_to_3d_with_preferred_operation(10.0, 45.0, 123.4, &dst, None)
            .unwrap();
        let base = src.transform_to_3d(10.0, 45.0, 123.4, &dst).unwrap();

        assert!((via_pref.0 - base.0).abs() < 1e-9);
        assert!((via_pref.1 - base.1).abs() < 1e-9);
        assert!((via_pref.2 - base.2).abs() < 1e-9);
    }

    #[test]
    fn transform_to_3d_with_preferred_operation_uses_registered_mapping() {
        let _guard = coordinate_operation_test_guard();
        clear_coordinate_operations().unwrap();

        let src = Crs::from_epsg(22317).unwrap();
        let dst = Crs::from_epsg(22817).unwrap();

        let via_pref = src
            .transform_to_3d_with_preferred_operation(500_000.0, 5_500_000.0, 50.0, &dst, None)
            .unwrap();
        let base = src.transform_to_3d(500_000.0, 5_500_000.0, 50.0, &dst).unwrap();

        assert!((via_pref.0 - base.0).abs() < 1e-9);
        assert!((via_pref.1 - base.1).abs() < 1e-9);
        assert!((via_pref.2 - base.2).abs() < 1e-9);
    }

    #[test]
    fn transform_to_3d_with_preferred_operation_uses_registered_mapping_for_v7_v8() {
        let _guard = coordinate_operation_test_guard();
        clear_coordinate_operations().unwrap();

        let src = Crs::from_epsg(22717).unwrap();
        let dst = Crs::from_epsg(22817).unwrap();

        let via_pref = src
            .transform_to_3d_with_preferred_operation(500_000.0, 5_500_000.0, 10.0, &dst, None)
            .unwrap();
        let base = src.transform_to_3d(500_000.0, 5_500_000.0, 10.0, &dst).unwrap();

        assert!((via_pref.0 - base.0).abs() < 1e-9);
        assert!((via_pref.1 - base.1).abs() < 1e-9);
        assert!((via_pref.2 - base.2).abs() < 1e-9);
    }

    #[test]
    fn transform_to_with_preferred_operation_supports_reverse_csrs_corridor() {
        let _guard = coordinate_operation_test_guard();
        clear_coordinate_operations().unwrap();

        let src = Crs::from_epsg(22817).unwrap();
        let dst = Crs::from_epsg(22317).unwrap();

        let via_pref = src
            .transform_to_with_preferred_operation(500_000.0, 5_500_000.0, &dst, None)
            .unwrap();
        let base = src.transform_to(500_000.0, 5_500_000.0, &dst).unwrap();
        assert!((via_pref.0 - base.0).abs() < 1e-9);
        assert!((via_pref.1 - base.1).abs() < 1e-9);
    }

    #[test]
    fn transform_to_3d_with_preferred_operation_supports_reverse_csrs_corridor() {
        let _guard = coordinate_operation_test_guard();
        clear_coordinate_operations().unwrap();

        let src = Crs::from_epsg(22817).unwrap();
        let dst = Crs::from_epsg(22317).unwrap();

        let via_pref = src
            .transform_to_3d_with_preferred_operation(500_000.0, 5_500_000.0, 42.0, &dst, None)
            .unwrap();
        let base = src.transform_to_3d(500_000.0, 5_500_000.0, 42.0, &dst).unwrap();
        assert!((via_pref.0 - base.0).abs() < 1e-9);
        assert!((via_pref.1 - base.1).abs() < 1e-9);
        assert!((via_pref.2 - base.2).abs() < 1e-9);
    }

    #[test]
    fn transform_to_with_preferred_operation_and_policy_supports_us_corridor_defaults() {
        let _guard = coordinate_operation_test_guard();
        clear_coordinate_operations().unwrap();

        let src = Crs::from_epsg(3582).unwrap();
        let dst = Crs::from_epsg(6487).unwrap();
        let policy = PreferredOperationPolicy {
            us_phase1_default_operation_code: Some(10715),
            europe_phase1_default_operation_code: None,
        };

        let via_pref = src
            .transform_to_with_preferred_operation_and_policy(
                500_000.0,
                5_500_000.0,
                &dst,
                None,
                policy,
            )
            .unwrap();
        let base = src.transform_to(500_000.0, 5_500_000.0, &dst).unwrap();
        assert!((via_pref.0 - base.0).abs() < 1e-9);
        assert!((via_pref.1 - base.1).abs() < 1e-9);
    }

    #[test]
    fn transform_to_with_preferred_operation_and_policy_supports_europe_corridor_defaults() {
        let _guard = coordinate_operation_test_guard();
        clear_coordinate_operations().unwrap();

        let src = Crs::from_epsg(25832).unwrap();
        let dst = Crs::from_epsg(3035).unwrap();
        let policy = PreferredOperationPolicy {
            us_phase1_default_operation_code: None,
            europe_phase1_default_operation_code: Some(10715),
        };

        let via_pref = src
            .transform_to_with_preferred_operation_and_policy(
                500_000.0,
                5_500_000.0,
                &dst,
                None,
                policy,
            )
            .unwrap();
        let base = src.transform_to(500_000.0, 5_500_000.0, &dst).unwrap();
        assert!((via_pref.0 - base.0).abs() < 1e-9);
        assert!((via_pref.1 - base.1).abs() < 1e-9);
    }

    #[test]
    fn transform_to_3d_with_preferred_operation_and_policy_supports_us_corridor_defaults() {
        let _guard = coordinate_operation_test_guard();
        clear_coordinate_operations().unwrap();

        let src = Crs::from_epsg(3582).unwrap();
        let dst = Crs::from_epsg(6487).unwrap();
        let policy = PreferredOperationPolicy {
            us_phase1_default_operation_code: Some(10715),
            europe_phase1_default_operation_code: None,
        };

        let via_pref = src
            .transform_to_3d_with_preferred_operation_and_policy(
                500_000.0,
                5_500_000.0,
                42.0,
                &dst,
                None,
                policy,
            )
            .unwrap();
        let base = src.transform_to_3d(500_000.0, 5_500_000.0, 42.0, &dst).unwrap();
        assert!((via_pref.0 - base.0).abs() < 1e-9);
        assert!((via_pref.1 - base.1).abs() < 1e-9);
        assert!((via_pref.2 - base.2).abs() < 1e-9);
    }

    #[test]
    fn transform_to_3d_with_preferred_operation_and_policy_supports_europe_corridor_defaults() {
        let _guard = coordinate_operation_test_guard();
        clear_coordinate_operations().unwrap();

        let src = Crs::from_epsg(25832).unwrap();
        let dst = Crs::from_epsg(3035).unwrap();
        let policy = PreferredOperationPolicy {
            us_phase1_default_operation_code: None,
            europe_phase1_default_operation_code: Some(10715),
        };

        let via_pref = src
            .transform_to_3d_with_preferred_operation_and_policy(
                500_000.0,
                5_500_000.0,
                42.0,
                &dst,
                None,
                policy,
            )
            .unwrap();
        let base = src.transform_to_3d(500_000.0, 5_500_000.0, 42.0, &dst).unwrap();
        assert!((via_pref.0 - base.0).abs() < 1e-9);
        assert!((via_pref.1 - base.1).abs() < 1e-9);
        assert!((via_pref.2 - base.2).abs() < 1e-9);
    }

    #[test]
    fn transform_to_with_preferred_operation_and_policy_supports_reverse_us_corridor_defaults() {
        let _guard = coordinate_operation_test_guard();
        clear_coordinate_operations().unwrap();

        let src = Crs::from_epsg(6487).unwrap();
        let dst = Crs::from_epsg(3582).unwrap();
        let policy = PreferredOperationPolicy {
            us_phase1_default_operation_code: Some(10715),
            europe_phase1_default_operation_code: None,
        };

        let via_pref = src
            .transform_to_with_preferred_operation_and_policy(
                500_000.0,
                5_500_000.0,
                &dst,
                None,
                policy,
            )
            .unwrap();
        let base = src.transform_to(500_000.0, 5_500_000.0, &dst).unwrap();
        assert!((via_pref.0 - base.0).abs() < 1e-9);
        assert!((via_pref.1 - base.1).abs() < 1e-9);
    }

    #[test]
    fn transform_to_3d_with_preferred_operation_and_policy_supports_reverse_europe_corridor_defaults() {
        let _guard = coordinate_operation_test_guard();
        clear_coordinate_operations().unwrap();

        let src = Crs::from_epsg(3035).unwrap();
        let dst = Crs::from_epsg(25832).unwrap();
        let policy = PreferredOperationPolicy {
            us_phase1_default_operation_code: None,
            europe_phase1_default_operation_code: Some(10715),
        };

        let via_pref = src
            .transform_to_3d_with_preferred_operation_and_policy(
                500_000.0,
                5_500_000.0,
                42.0,
                &dst,
                None,
                policy,
            )
            .unwrap();
        let base = src.transform_to_3d(500_000.0, 5_500_000.0, 42.0, &dst).unwrap();
        assert!((via_pref.0 - base.0).abs() < 1e-9);
        assert!((via_pref.1 - base.1).abs() < 1e-9);
        assert!((via_pref.2 - base.2).abs() < 1e-9);
    }

    #[test]
    fn transform_to_3d_with_preferred_operation_and_policy_supports_reverse_us_secondary_seed_defaults() {
        let _guard = coordinate_operation_test_guard();
        clear_coordinate_operations().unwrap();

        let src = Crs::from_epsg(6568).unwrap();
        let dst = Crs::from_epsg(3600).unwrap();
        let policy = PreferredOperationPolicy {
            us_phase1_default_operation_code: Some(10715),
            europe_phase1_default_operation_code: None,
        };

        let via_pref = src
            .transform_to_3d_with_preferred_operation_and_policy(
                500_000.0,
                5_500_000.0,
                42.0,
                &dst,
                None,
                policy,
            )
            .unwrap();
        let base = src.transform_to_3d(500_000.0, 5_500_000.0, 42.0, &dst).unwrap();
        assert!((via_pref.0 - base.0).abs() < 1e-9);
        assert!((via_pref.1 - base.1).abs() < 1e-9);
        assert!((via_pref.2 - base.2).abs() < 1e-9);
    }

    #[test]
    fn transform_to_with_preferred_operation_and_policy_supports_reverse_europe_broad_defaults() {
        let _guard = coordinate_operation_test_guard();
        clear_coordinate_operations().unwrap();

        let src = Crs::from_epsg(3035).unwrap();
        let dst = Crs::from_epsg(25801).unwrap();
        let policy = PreferredOperationPolicy {
            us_phase1_default_operation_code: None,
            europe_phase1_default_operation_code: Some(10715),
        };

        let via_pref = src
            .transform_to_with_preferred_operation_and_policy(
                500_000.0,
                5_500_000.0,
                &dst,
                None,
                policy,
            )
            .unwrap();
        let base = src.transform_to(500_000.0, 5_500_000.0, &dst).unwrap();
        assert!((via_pref.0 - base.0).abs() < 1e-9);
        assert!((via_pref.1 - base.1).abs() < 1e-9);
    }

    #[test]
    fn transform_to_with_preferred_operation_and_policy_falls_back_for_us_when_default_unset() {
        let _guard = coordinate_operation_test_guard();
        clear_coordinate_operations().unwrap();

        let src = Crs::from_epsg(3582).unwrap();
        let dst = Crs::from_epsg(6487).unwrap();
        let policy = PreferredOperationPolicy {
            us_phase1_default_operation_code: None,
            europe_phase1_default_operation_code: None,
        };

        let via_pref = src
            .transform_to_with_preferred_operation_and_policy(
                500_000.0,
                5_500_000.0,
                &dst,
                None,
                policy,
            )
            .unwrap();
        let base = src.transform_to(500_000.0, 5_500_000.0, &dst).unwrap();
        assert!((via_pref.0 - base.0).abs() < 1e-9);
        assert!((via_pref.1 - base.1).abs() < 1e-9);
    }

    #[test]
    fn transform_to_3d_with_preferred_operation_and_policy_falls_back_for_europe_when_default_unset() {
        let _guard = coordinate_operation_test_guard();
        clear_coordinate_operations().unwrap();

        let src = Crs::from_epsg(25832).unwrap();
        let dst = Crs::from_epsg(3035).unwrap();
        let policy = PreferredOperationPolicy {
            us_phase1_default_operation_code: None,
            europe_phase1_default_operation_code: None,
        };

        let via_pref = src
            .transform_to_3d_with_preferred_operation_and_policy(
                500_000.0,
                5_500_000.0,
                42.0,
                &dst,
                None,
                policy,
            )
            .unwrap();
        let base = src.transform_to_3d(500_000.0, 5_500_000.0, 42.0, &dst).unwrap();
        assert!((via_pref.0 - base.0).abs() < 1e-9);
        assert!((via_pref.1 - base.1).abs() < 1e-9);
        assert!((via_pref.2 - base.2).abs() < 1e-9);
    }

    #[test]
    fn transform_to_with_preferred_operation_and_policy_falls_back_for_europe_when_default_unset() {
        let _guard = coordinate_operation_test_guard();
        clear_coordinate_operations().unwrap();

        let src = Crs::from_epsg(25832).unwrap();
        let dst = Crs::from_epsg(3035).unwrap();
        let policy = PreferredOperationPolicy {
            us_phase1_default_operation_code: None,
            europe_phase1_default_operation_code: None,
        };

        let via_pref = src
            .transform_to_with_preferred_operation_and_policy(
                500_000.0,
                5_500_000.0,
                &dst,
                None,
                policy,
            )
            .unwrap();
        let base = src.transform_to(500_000.0, 5_500_000.0, &dst).unwrap();
        assert!((via_pref.0 - base.0).abs() < 1e-9);
        assert!((via_pref.1 - base.1).abs() < 1e-9);
    }

    #[test]
    fn transform_to_3d_with_preferred_operation_and_policy_falls_back_for_us_when_default_unset() {
        let _guard = coordinate_operation_test_guard();
        clear_coordinate_operations().unwrap();

        let src = Crs::from_epsg(3582).unwrap();
        let dst = Crs::from_epsg(6487).unwrap();
        let policy = PreferredOperationPolicy {
            us_phase1_default_operation_code: None,
            europe_phase1_default_operation_code: None,
        };

        let via_pref = src
            .transform_to_3d_with_preferred_operation_and_policy(
                500_000.0,
                5_500_000.0,
                42.0,
                &dst,
                None,
                policy,
            )
            .unwrap();
        let base = src.transform_to_3d(500_000.0, 5_500_000.0, 42.0, &dst).unwrap();
        assert!((via_pref.0 - base.0).abs() < 1e-9);
        assert!((via_pref.1 - base.1).abs() < 1e-9);
        assert!((via_pref.2 - base.2).abs() < 1e-9);
    }

    #[test]
    fn transform_to_with_preferred_operation_matches_explicit_default_policy() {
        let _guard = coordinate_operation_test_guard();
        clear_coordinate_operations().unwrap();

        let src = Crs::from_epsg(22317).unwrap();
        let dst = Crs::from_epsg(22817).unwrap();

        let via_default_api = src
            .transform_to_with_preferred_operation(500_000.0, 5_500_000.0, &dst, None)
            .unwrap();
        let via_default_policy = src
            .transform_to_with_preferred_operation_and_policy(
                500_000.0,
                5_500_000.0,
                &dst,
                None,
                PreferredOperationPolicy::default(),
            )
            .unwrap();

        assert!((via_default_api.0 - via_default_policy.0).abs() < 1e-9);
        assert!((via_default_api.1 - via_default_policy.1).abs() < 1e-9);
    }

    #[test]
    fn transform_to_3d_with_preferred_operation_matches_explicit_default_policy() {
        let _guard = coordinate_operation_test_guard();
        clear_coordinate_operations().unwrap();

        let src = Crs::from_epsg(22317).unwrap();
        let dst = Crs::from_epsg(22817).unwrap();

        let via_default_api = src
            .transform_to_3d_with_preferred_operation(500_000.0, 5_500_000.0, 42.0, &dst, None)
            .unwrap();
        let via_default_policy = src
            .transform_to_3d_with_preferred_operation_and_policy(
                500_000.0,
                5_500_000.0,
                42.0,
                &dst,
                None,
                PreferredOperationPolicy::default(),
            )
            .unwrap();

        assert!((via_default_api.0 - via_default_policy.0).abs() < 1e-9);
        assert!((via_default_api.1 - via_default_policy.1).abs() < 1e-9);
        assert!((via_default_api.2 - via_default_policy.2).abs() < 1e-9);
    }
}
