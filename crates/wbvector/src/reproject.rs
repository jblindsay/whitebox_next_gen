//! Layer reprojection utilities backed by `wbprojection`.
//!
//! This module transforms layer geometries between CRS definitions using EPSG
//! codes or explicit `wbprojection::Crs` objects.

use wbprojection::Crs;

use crate::crs;
use crate::error::{GeoError, Result};
use crate::feature::Layer;
use crate::geometry::{Coord, Geometry, Ring};

/// Behavior when a feature fails reprojection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformFailurePolicy {
    /// Abort and return the first reprojection error.
    Error,
    /// Keep the feature but set its geometry to `None`.
    SetNullGeometry,
    /// Drop the failed feature from output.
    SkipFeature,
}

/// Longitude handling policy for geographic destination outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntimeridianPolicy {
    /// Keep longitudes as transformed.
    Keep,
    /// Normalize longitudes to [-180, 180) for EPSG:4326 outputs.
    NormalizeLon180,
    /// Split line geometries at ±180 for EPSG:4326 outputs.
    ///
    /// For `LineString`, output may become `MultiLineString`.
    /// For polygons, output may become `MultiPolygon` with hole assignment.
    SplitAt180,
}

/// Polygon topology handling policy after reprojection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopologyPolicy {
    /// Do not enforce polygon topology checks.
    None,
    /// Validate polygon topology and return an error if invalid.
    Validate,
    /// Validate polygon topology and auto-fix ring orientation when possible.
    ValidateAndFixOrientation,
}

/// Options for vector reprojection.
#[derive(Debug, Clone, Copy)]
pub struct VectorReprojectOptions {
    /// Policy used when a feature cannot be transformed.
    pub failure_policy: TransformFailurePolicy,
    /// Antimeridian handling mode for geographic outputs.
    pub antimeridian_policy: AntimeridianPolicy,
    /// Optional densification threshold before projection (in source units).
    pub max_segment_length: Option<f64>,
    /// Topology validation/fix mode after reprojection.
    pub topology_policy: TopologyPolicy,
}

impl Default for VectorReprojectOptions {
    fn default() -> Self {
        Self {
            failure_policy: TransformFailurePolicy::Error,
            antimeridian_policy: AntimeridianPolicy::Keep,
            max_segment_length: None,
            topology_policy: TopologyPolicy::None,
        }
    }
}

impl VectorReprojectOptions {
    /// Creates default reprojection options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets feature transform failure behavior.
    pub fn with_failure_policy(mut self, policy: TransformFailurePolicy) -> Self {
        self.failure_policy = policy;
        self
    }

    /// Sets antimeridian handling behavior.
    pub fn with_antimeridian_policy(mut self, policy: AntimeridianPolicy) -> Self {
        self.antimeridian_policy = policy;
        self
    }

    /// Enables segment densification using maximum segment length.
    pub fn with_max_segment_length(mut self, max_segment_length: f64) -> Self {
        self.max_segment_length = Some(max_segment_length);
        self
    }

    /// Sets topology validation/fix behavior after reprojection.
    pub fn with_topology_policy(mut self, topology_policy: TopologyPolicy) -> Self {
        self.topology_policy = topology_policy;
        self
    }
}

/// Reproject a layer to a destination EPSG code.
///
/// Source CRS is read from `layer.crs_epsg()`. If no source EPSG is available,
/// returns an error.
pub fn layer_to_epsg(layer: &Layer, dst_epsg: u32) -> Result<Layer> {
    layer_to_epsg_with_options(layer, dst_epsg, &VectorReprojectOptions::default())
}

/// Reproject a layer to a destination EPSG code with options.
pub fn layer_to_epsg_with_options(
    layer: &Layer,
    dst_epsg: u32,
    options: &VectorReprojectOptions,
) -> Result<Layer> {
    let src = source_crs_from_layer(layer)?;
    let dst = Crs::from_epsg(dst_epsg)
        .map_err(|e| GeoError::Projection(format!("invalid destination EPSG {dst_epsg}: {e}")))?;

    layer_with_crs_options(layer, &src, &dst, Some(dst_epsg), options)
}

/// Reproject a layer to a destination EPSG code with options and progress
/// updates in the range [0, 1] as features are completed.
pub fn layer_to_epsg_with_options_and_progress<F>(
    layer: &Layer,
    dst_epsg: u32,
    options: &VectorReprojectOptions,
    progress: F,
) -> Result<Layer>
where
    F: Fn(f64) + Send + Sync,
{
    let src = source_crs_from_layer(layer)?;
    let dst = Crs::from_epsg(dst_epsg)
        .map_err(|e| GeoError::Projection(format!("invalid destination EPSG {dst_epsg}: {e}")))?;

    layer_with_crs_options_internal(layer, &src, &dst, Some(dst_epsg), options, Some(&progress))
}

/// Reproject a layer between explicit source/destination EPSG codes.
pub fn layer_from_to_epsg(layer: &Layer, src_epsg: u32, dst_epsg: u32) -> Result<Layer> {
    layer_from_to_epsg_with_options(layer, src_epsg, dst_epsg, &VectorReprojectOptions::default())
}

/// Reproject a layer between explicit source/destination EPSG codes with options.
pub fn layer_from_to_epsg_with_options(
    layer: &Layer,
    src_epsg: u32,
    dst_epsg: u32,
    options: &VectorReprojectOptions,
) -> Result<Layer> {
    let src = Crs::from_epsg(src_epsg)
        .map_err(|e| GeoError::Projection(format!("invalid source EPSG {src_epsg}: {e}")))?;
    let dst = Crs::from_epsg(dst_epsg)
        .map_err(|e| GeoError::Projection(format!("invalid destination EPSG {dst_epsg}: {e}")))?;

    layer_with_crs_options(layer, &src, &dst, Some(dst_epsg), options)
}

/// Reproject a layer using caller-supplied source/destination CRS objects.
///
/// When `dst_epsg_hint` is provided, output CRS metadata is recorded as EPSG
/// + generated OGC WKT. Otherwise CRS metadata is preserved as unknown.
pub fn layer_with_crs(
    layer: &Layer,
    src: &Crs,
    dst: &Crs,
    dst_epsg_hint: Option<u32>,
) -> Result<Layer> {
    layer_with_crs_options(
        layer,
        src,
        dst,
        dst_epsg_hint,
        &VectorReprojectOptions::default(),
    )
}

/// Reproject a layer using caller-supplied source/destination CRS objects and options.
pub fn layer_with_crs_options(
    layer: &Layer,
    src: &Crs,
    dst: &Crs,
    dst_epsg_hint: Option<u32>,
    options: &VectorReprojectOptions,
) -> Result<Layer> {
    layer_with_crs_options_internal(layer, src, dst, dst_epsg_hint, options, None)
}

/// Reproject a layer using caller-supplied source/destination CRS objects and
/// options, emitting progress updates in the range [0, 1] as features finish.
pub fn layer_with_crs_options_and_progress<F>(
    layer: &Layer,
    src: &Crs,
    dst: &Crs,
    dst_epsg_hint: Option<u32>,
    options: &VectorReprojectOptions,
    progress: F,
) -> Result<Layer>
where
    F: Fn(f64) + Send + Sync,
{
    layer_with_crs_options_internal(layer, src, dst, dst_epsg_hint, options, Some(&progress))
}

fn layer_with_crs_options_internal(
    layer: &Layer,
    src: &Crs,
    dst: &Crs,
    dst_epsg_hint: Option<u32>,
    options: &VectorReprojectOptions,
    progress: Option<&(dyn Fn(f64) + Send + Sync)>,
) -> Result<Layer> {
    validate_options(options)?;

    let mut out = layer.clone();
    out.features.clear();
    let total_features = layer.features.len();

    for (index, feat) in layer.features.iter().enumerate() {
        let mut out_feat = feat.clone();
        if let Some(geom) = feat.geometry.as_ref() {
            match reproject_geometry(geom, src, dst, dst_epsg_hint, options)
                .and_then(|g| enforce_topology_on_geometry(g, options.topology_policy))
            {
                Ok(g) => out_feat.geometry = Some(g),
                Err(err) => match options.failure_policy {
                    TransformFailurePolicy::Error => return Err(err),
                    TransformFailurePolicy::SetNullGeometry => out_feat.geometry = None,
                    TransformFailurePolicy::SkipFeature => continue,
                },
            }
        }
        out.features.push(out_feat);
        if let Some(progress_cb) = progress {
            progress_cb((index + 1) as f64 / total_features.max(1) as f64);
        }
    }

    out.set_crs_epsg(dst_epsg_hint);
    out.set_crs_wkt(dst_epsg_hint.and_then(crs::ogc_wkt_from_epsg));
    out.extent = None;

    if let Some(progress_cb) = progress {
        progress_cb(1.0);
    }

    Ok(out)
}

fn source_crs_from_layer(layer: &Layer) -> Result<Crs> {
    if let Some(src_epsg) = layer.crs_epsg() {
        return Crs::from_epsg(src_epsg)
            .map_err(|e| GeoError::Projection(format!("invalid source EPSG {src_epsg}: {e}")));
    }

    if let Some(wkt) = layer.crs_wkt() {
        let trimmed = wkt.trim();
        if !trimmed.is_empty() {
            return wbprojection::from_wkt(trimmed)
                .map_err(|e| GeoError::Projection(format!("invalid source CRS WKT: {e}")));
        }
    }

    Err(GeoError::Projection(
        "layer_to_epsg requires source CRS metadata in layer.crs (EPSG or WKT)".to_owned(),
    ))
}

fn reproject_geometry(
    g: &Geometry,
    src: &Crs,
    dst: &Crs,
    dst_epsg_hint: Option<u32>,
    options: &VectorReprojectOptions,
) -> Result<Geometry> {
    let is_dst_4326 = dst_epsg_hint == Some(4326);

    Ok(match g {
        Geometry::Point(c) => Geometry::Point(reproject_coord(c, src, dst, dst_epsg_hint, options)?),
        Geometry::LineString(cs) => {
            let densified = densify_coords(cs, options.max_segment_length)?;
            let projected = reproject_coords(&densified, src, dst, dst_epsg_hint, options)?;
            if is_dst_4326 && matches!(options.antimeridian_policy, AntimeridianPolicy::SplitAt180) {
                let parts = split_linestring_at_antimeridian(&projected);
                if parts.len() <= 1 {
                    Geometry::LineString(projected)
                } else {
                    Geometry::MultiLineString(parts)
                }
            } else {
                Geometry::LineString(projected)
            }
        }
        Geometry::Polygon { exterior, interiors } => {
            let exterior_proj = Ring::new(reproject_coords(
                &densify_ring_coords(exterior.coords(), options.max_segment_length)?,
                src,
                dst,
                dst_epsg_hint,
                options,
            )?);
            let interiors_proj = interiors
                .iter()
                .map(|r| {
                    let densified = densify_ring_coords(r.coords(), options.max_segment_length)?;
                    reproject_coords(&densified, src, dst, dst_epsg_hint, options).map(Ring::new)
                })
                .collect::<Result<Vec<_>>>()?;

            if is_dst_4326 && matches!(options.antimeridian_policy, AntimeridianPolicy::SplitAt180) {
                split_polygon_if_needed(&exterior_proj, &interiors_proj)?
            } else {
                Geometry::Polygon {
                    exterior: exterior_proj,
                    interiors: interiors_proj,
                }
            }
        }
        Geometry::MultiPoint(cs) => Geometry::MultiPoint(reproject_coords(cs, src, dst, dst_epsg_hint, options)?),
        Geometry::MultiLineString(lines) => {
            let projected = lines
                .iter()
                .map(|line| {
                    let densified = densify_coords(line, options.max_segment_length)?;
                    reproject_coords(&densified, src, dst, dst_epsg_hint, options)
                })
                .collect::<Result<Vec<_>>>()?;

            if is_dst_4326 && matches!(options.antimeridian_policy, AntimeridianPolicy::SplitAt180) {
                let mut all_parts: Vec<Vec<Coord>> = Vec::new();
                for line in &projected {
                    all_parts.extend(split_linestring_at_antimeridian(line));
                }
                Geometry::MultiLineString(all_parts)
            } else {
                Geometry::MultiLineString(projected)
            }
        }
        Geometry::MultiPolygon(polys) => {
            let mut out_polys: Vec<(Ring, Vec<Ring>)> = Vec::new();
            for (ext, holes) in polys {
                let ext_dense = densify_ring_coords(ext.coords(), options.max_segment_length)?;
                let ext2 = Ring::new(reproject_coords(&ext_dense, src, dst, dst_epsg_hint, options)?);
                let holes2 = holes
                    .iter()
                    .map(|h| {
                        let h_dense = densify_ring_coords(h.coords(), options.max_segment_length)?;
                        reproject_coords(&h_dense, src, dst, dst_epsg_hint, options).map(Ring::new)
                    })
                    .collect::<Result<Vec<_>>>()?;

                if is_dst_4326 && matches!(options.antimeridian_policy, AntimeridianPolicy::SplitAt180) {
                    match split_polygon_if_needed(&ext2, &holes2)? {
                        Geometry::Polygon { exterior, interiors } => out_polys.push((exterior, interiors)),
                        Geometry::MultiPolygon(parts) => out_polys.extend(parts),
                        _ => unreachable!("split_polygon_if_needed returns Polygon or MultiPolygon"),
                    }
                } else {
                    out_polys.push((ext2, holes2));
                }
            }
            Geometry::MultiPolygon(out_polys)
        }
        Geometry::GeometryCollection(gs) => Geometry::GeometryCollection(
            gs.iter()
                .map(|child| reproject_geometry(child, src, dst, dst_epsg_hint, options))
                .collect::<Result<Vec<_>>>()?,
        ),
    })
}

fn validate_options(options: &VectorReprojectOptions) -> Result<()> {
    if let Some(max_len) = options.max_segment_length {
        if !max_len.is_finite() || max_len <= 0.0 {
            return Err(GeoError::Projection(
                "max_segment_length must be a positive finite value".to_owned(),
            ));
        }
    }
    Ok(())
}

fn densify_coords(coords: &[Coord], max_segment_length: Option<f64>) -> Result<Vec<Coord>> {
    let Some(max_len) = max_segment_length else {
        return Ok(coords.to_vec());
    };
    if coords.len() < 2 {
        return Ok(coords.to_vec());
    }

    let mut out = Vec::with_capacity(coords.len());
    out.push(coords[0].clone());

    for pair in coords.windows(2) {
        let a = &pair[0];
        let b = &pair[1];
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let seg_len = (dx * dx + dy * dy).sqrt();
        let n_segments = (seg_len / max_len).ceil() as usize;

        if n_segments > 1 {
            for i in 1..n_segments {
                let t = i as f64 / n_segments as f64;
                out.push(interpolate_coord(a, b, t));
            }
        }

        out.push(b.clone());
    }

    Ok(out)
}

fn densify_ring_coords(coords: &[Coord], max_segment_length: Option<f64>) -> Result<Vec<Coord>> {
    let Some(max_len) = max_segment_length else {
        return Ok(coords.to_vec());
    };
    let n = coords.len();
    if n < 3 {
        return Ok(coords.to_vec());
    }

    let mut out = Vec::with_capacity(n);
    out.push(coords[0].clone());

    for i in 0..n {
        let a = &coords[i];
        let b = &coords[(i + 1) % n];
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let seg_len = (dx * dx + dy * dy).sqrt();
        let n_segments = (seg_len / max_len).ceil() as usize;

        if n_segments > 1 {
            for j in 1..n_segments {
                let t = j as f64 / n_segments as f64;
                out.push(interpolate_coord(a, b, t));
            }
        }

        if i + 1 < n {
            out.push(b.clone());
        }
    }

    Ok(out)
}

fn interpolate_coord(a: &Coord, b: &Coord, t: f64) -> Coord {
    Coord {
        x: a.x + (b.x - a.x) * t,
        y: a.y + (b.y - a.y) * t,
        z: match (a.z, b.z) {
            (Some(az), Some(bz)) => Some(az + (bz - az) * t),
            _ => None,
        },
        m: match (a.m, b.m) {
            (Some(am), Some(bm)) => Some(am + (bm - am) * t),
            _ => None,
        },
    }
}

fn reproject_coords(
    coords: &[Coord],
    src: &Crs,
    dst: &Crs,
    dst_epsg_hint: Option<u32>,
    options: &VectorReprojectOptions,
) -> Result<Vec<Coord>> {
    coords
        .iter()
        .map(|c| reproject_coord(c, src, dst, dst_epsg_hint, options))
        .collect::<Result<Vec<_>>>()
}

fn reproject_coord(
    coord: &Coord,
    src: &Crs,
    dst: &Crs,
    dst_epsg_hint: Option<u32>,
    options: &VectorReprojectOptions,
) -> Result<Coord> {
    if !coord.x.is_finite() || !coord.y.is_finite() {
        return Err(GeoError::Projection("coordinate transform failed: non-finite coordinate".to_owned()));
    }

    let (mut x, y) = src
        .transform_to(coord.x, coord.y, dst)
        .map_err(|e| GeoError::Projection(format!("coordinate transform failed: {e}")))?;

    if !x.is_finite() || !y.is_finite() {
        return Err(GeoError::Projection("coordinate transform failed: non-finite output".to_owned()));
    }

    if dst_epsg_hint == Some(4326)
        && matches!(
            options.antimeridian_policy,
            AntimeridianPolicy::NormalizeLon180 | AntimeridianPolicy::SplitAt180
        )
    {
        x = wbprojection::normalize_longitude(x);
    }

    Ok(Coord {
        x,
        y,
        z: coord.z,
        m: coord.m,
    })
}

fn split_linestring_at_antimeridian(coords: &[Coord]) -> Vec<Vec<Coord>> {
    if coords.len() < 2 {
        return vec![coords.to_vec()];
    }

    let mut parts: Vec<Vec<Coord>> = Vec::new();
    let mut current: Vec<Coord> = vec![coords[0].clone()];

    for pair in coords.windows(2) {
        let a = &pair[0];
        let b = &pair[1];

        let mut b_unwrapped = b.x;
        let raw_diff = b.x - a.x;
        if raw_diff > 180.0 {
            b_unwrapped -= 360.0;
        } else if raw_diff < -180.0 {
            b_unwrapped += 360.0;
        }

        if b_unwrapped > 180.0 || b_unwrapped < -180.0 {
            let boundary = if b_unwrapped > 180.0 { 180.0 } else { -180.0 };
            let denom = b_unwrapped - a.x;
            if denom.abs() > f64::EPSILON {
                let t = (boundary - a.x) / denom;
                let lat = a.y + (b.y - a.y) * t;
                let z = match (a.z, b.z) {
                    (Some(az), Some(bz)) => Some(az + (bz - az) * t),
                    _ => None,
                };
                let m = match (a.m, b.m) {
                    (Some(am), Some(bm)) => Some(am + (bm - am) * t),
                    _ => None,
                };

                current.push(Coord { x: boundary, y: lat, z, m });
                if current.len() >= 2 {
                    parts.push(current);
                }

                let opposite = if boundary > 0.0 { -180.0 } else { 180.0 };
                current = vec![Coord {
                    x: opposite,
                    y: lat,
                    z,
                    m,
                }];
            }
        }

        let b_norm = Coord {
            x: wbprojection::normalize_longitude(b_unwrapped),
            y: b.y,
            z: b.z,
            m: b.m,
        };
        current.push(b_norm);
    }

    if current.len() >= 2 {
        parts.push(current);
    }

    if parts.is_empty() {
        vec![coords.to_vec()]
    } else {
        parts
    }
}

fn split_polygon_if_needed(exterior: &Ring, interiors: &[Ring]) -> Result<Geometry> {
    if !ring_crosses_antimeridian(exterior.coords()) {
        return Ok(Geometry::Polygon {
            exterior: exterior.clone(),
            interiors: interiors.to_vec(),
        });
    }

    let parts = split_simple_ring_at_antimeridian(exterior.coords());
    if parts.len() <= 1 {
        Ok(Geometry::Polygon {
            exterior: Ring::new(parts.into_iter().next().unwrap_or_else(|| exterior.coords().to_vec())),
            interiors: interiors.to_vec(),
        })
    } else {
        let mut part_polys: Vec<(Ring, Vec<Ring>)> =
            parts.into_iter().map(|ring| (Ring::new(ring), Vec::new())).collect();

        for hole in interiors {
            let hole_parts = split_simple_ring_at_antimeridian(hole.coords());
            for hole_part in hole_parts {
                let (hx, hy) = ring_centroid(&hole_part)
                    .ok_or_else(|| GeoError::Projection("invalid polygon hole during antimeridian split".to_owned()))?;

                let mut assigned = false;
                for (ext, holes) in &mut part_polys {
                    if point_in_ring(hx, hy, ext.coords()) {
                        holes.push(Ring::new(hole_part.clone()));
                        assigned = true;
                        break;
                    }
                }

                if !assigned {
                    return Err(GeoError::Projection(
                        "unable to assign polygon hole during antimeridian split; use NormalizeLon180 or feature failure policy"
                            .to_owned(),
                    ));
                }
            }
        }

        Ok(Geometry::MultiPolygon(part_polys))
    }
}

fn split_simple_ring_at_antimeridian(coords: &[Coord]) -> Vec<Vec<Coord>> {
    if coords.len() < 3 {
        return vec![coords.to_vec()];
    }

    let unwrapped = unwrap_ring(coords);
    let min_x = unwrapped.iter().map(|c| c.x).fold(f64::INFINITY, f64::min);
    let max_x = unwrapped.iter().map(|c| c.x).fold(f64::NEG_INFINITY, f64::max);

    if max_x <= 180.0 && min_x >= -180.0 {
        return vec![coords.to_vec()];
    }

    if max_x > 180.0 {
        let west = clip_ring_against_vertical(&unwrapped, 180.0, true);
        let east = clip_ring_against_vertical(&unwrapped, 180.0, false)
            .into_iter()
            .map(|mut r| {
                for p in &mut r {
                    p.x -= 360.0;
                    p.x = wbprojection::normalize_longitude(p.x);
                }
                r
            })
            .collect::<Vec<_>>();
        west.into_iter().chain(east).collect()
    } else {
        let east = clip_ring_against_vertical(&unwrapped, -180.0, false);
        let west = clip_ring_against_vertical(&unwrapped, -180.0, true)
            .into_iter()
            .map(|mut r| {
                for p in &mut r {
                    p.x += 360.0;
                    p.x = wbprojection::normalize_longitude(p.x);
                }
                r
            })
            .collect::<Vec<_>>();
        east.into_iter().chain(west).collect()
    }
}

fn unwrap_ring(coords: &[Coord]) -> Vec<Coord> {
    if coords.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(coords.len());
    out.push(coords[0].clone());

    for c in coords.iter().skip(1) {
        let prev_x = out.last().unwrap().x;
        let mut x = c.x;
        while x - prev_x > 180.0 {
            x -= 360.0;
        }
        while x - prev_x < -180.0 {
            x += 360.0;
        }
        out.push(Coord { x, y: c.y, z: c.z, m: c.m });
    }

    out
}

fn clip_ring_against_vertical(coords: &[Coord], boundary: f64, keep_le: bool) -> Vec<Vec<Coord>> {
    if coords.len() < 3 {
        return Vec::new();
    }

    let mut output: Vec<Coord> = Vec::new();

    for i in 0..coords.len() {
        let s = &coords[i];
        let e = &coords[(i + 1) % coords.len()];
        let s_in = if keep_le { s.x <= boundary } else { s.x >= boundary };
        let e_in = if keep_le { e.x <= boundary } else { e.x >= boundary };

        if s_in && e_in {
            output.push(e.clone());
        } else if s_in && !e_in {
            if let Some(int) = segment_vertical_intersection(s, e, boundary) {
                output.push(int);
            }
        } else if !s_in && e_in {
            if let Some(int) = segment_vertical_intersection(s, e, boundary) {
                output.push(int);
            }
            output.push(e.clone());
        }
    }

    if output.len() < 3 {
        return Vec::new();
    }

    dedup_consecutive(&mut output);
    if output.len() < 3 {
        return Vec::new();
    }

    vec![output]
}

fn segment_vertical_intersection(a: &Coord, b: &Coord, x_boundary: f64) -> Option<Coord> {
    let dx = b.x - a.x;
    if dx.abs() < f64::EPSILON {
        return None;
    }
    let t = (x_boundary - a.x) / dx;
    if !(0.0..=1.0).contains(&t) {
        return None;
    }
    Some(interpolate_coord(a, b, t))
}

fn dedup_consecutive(coords: &mut Vec<Coord>) {
    coords.dedup_by(|a, b| {
        (a.x - b.x).abs() < 1e-12 && (a.y - b.y).abs() < 1e-12 && a.z == b.z && a.m == b.m
    });
}

fn ring_centroid(coords: &[Coord]) -> Option<(f64, f64)> {
    if coords.is_empty() {
        return None;
    }
    let mut sx = 0.0;
    let mut sy = 0.0;
    for c in coords {
        sx += c.x;
        sy += c.y;
    }
    Some((sx / coords.len() as f64, sy / coords.len() as f64))
}

fn point_in_ring(x: f64, y: f64, ring: &[Coord]) -> bool {
    if ring.len() < 3 {
        return false;
    }

    let mut inside = false;
    let mut j = ring.len() - 1;
    for i in 0..ring.len() {
        let xi = ring[i].x;
        let yi = ring[i].y;
        let xj = ring[j].x;
        let yj = ring[j].y;

        let intersects = ((yi > y) != (yj > y))
            && (x < (xj - xi) * (y - yi) / ((yj - yi).abs().max(f64::EPSILON)) + xi);
        if intersects {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn enforce_topology_on_geometry(mut g: Geometry, policy: TopologyPolicy) -> Result<Geometry> {
    if matches!(policy, TopologyPolicy::None) {
        return Ok(g);
    }

    match &mut g {
        Geometry::Polygon { exterior, interiors } => {
            enforce_polygon_topology(exterior, interiors, policy)?;
        }
        Geometry::MultiPolygon(polys) => {
            for (exterior, interiors) in polys {
                enforce_polygon_topology(exterior, interiors, policy)?;
            }
        }
        Geometry::GeometryCollection(geoms) => {
            for geom in geoms.iter_mut() {
                *geom = enforce_topology_on_geometry(geom.clone(), policy)?;
            }
        }
        _ => {}
    }

    Ok(g)
}

fn enforce_polygon_topology(exterior: &mut Ring, interiors: &mut [Ring], policy: TopologyPolicy) -> Result<()> {
    if exterior.coords().len() < 3 {
        return Err(GeoError::Projection("polygon exterior ring has fewer than 3 vertices".to_owned()));
    }
    for hole in interiors.iter() {
        if hole.coords().len() < 3 {
            return Err(GeoError::Projection("polygon interior ring has fewer than 3 vertices".to_owned()));
        }
    }

    if matches!(policy, TopologyPolicy::ValidateAndFixOrientation) {
        orient_ring(exterior, true);
        for hole in interiors.iter_mut() {
            orient_ring(hole, false);
        }
    }

    for hole in interiors.iter() {
        let (hx, hy) = ring_centroid(hole.coords())
            .ok_or_else(|| GeoError::Projection("invalid polygon interior ring".to_owned()))?;
        if !point_in_ring(hx, hy, exterior.coords()) {
            return Err(GeoError::Projection(
                "polygon interior ring lies outside exterior ring".to_owned(),
            ));
        }
    }

    Ok(())
}

fn orient_ring(ring: &mut Ring, want_ccw: bool) {
    let area = ring_signed_area(ring.coords());
    let is_ccw = area > 0.0;
    if is_ccw != want_ccw {
        ring.0.reverse();
    }
}

fn ring_signed_area(coords: &[Coord]) -> f64 {
    let n = coords.len();
    if n < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        area += coords[i].x * coords[j].y - coords[j].x * coords[i].y;
    }
    area * 0.5
}

fn ring_crosses_antimeridian(coords: &[Coord]) -> bool {
    if coords.len() < 2 {
        return false;
    }
    for i in 0..coords.len() {
        let a = &coords[i];
        let b = &coords[(i + 1) % coords.len()];
        if (b.x - a.x).abs() > 180.0 {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature::FieldDef;
    use crate::feature::FieldType;
    use crate::geometry::GeometryType;
    use crate::geometry::{Coord, Geometry};
    use std::sync::{Arc, Mutex};

    fn sample_point_layer() -> Layer {
        let mut layer = Layer::new("cities")
            .with_geom_type(GeometryType::Point)
            .with_crs_epsg(4326);
        layer.add_field(FieldDef::new("name", FieldType::Text));
        layer
            .add_feature(
                Some(Geometry::point(-75.0, 45.0)),
                &[("name", "Ottawa".into())],
            )
            .unwrap();
        layer
    }

    #[test]
    fn reproject_to_epsg_requires_source_epsg() {
        let layer = Layer::new("no_crs").with_geom_type(GeometryType::Point);
        let err = layer_to_epsg(&layer, 3857).unwrap_err();
        assert!(format!("{err}").contains("requires source CRS metadata"));
    }

    #[test]
    fn reproject_to_epsg_accepts_source_wkt_without_epsg() {
        let mut layer = Layer::new("wkt_only").with_geom_type(GeometryType::Point);
        layer.set_crs_wkt(crs::ogc_wkt_from_epsg(4326));
        layer
            .add_feature(
                Some(Geometry::point(-75.0, 45.0)),
                &[],
            )
            .unwrap();

        let out = layer_to_epsg(&layer, 3857).unwrap();
        assert_eq!(out.crs_epsg(), Some(3857));
    }

    #[test]
    fn reproject_to_epsg_updates_coords_and_crs() {
        let layer = sample_point_layer();
        let out = layer_to_epsg(&layer, 3857).unwrap();

        assert_eq!(out.crs_epsg(), Some(3857));
        assert!(out.crs_wkt().map(|w| !w.is_empty()).unwrap_or(false));
        if let Some(Geometry::Point(c)) = &out.features[0].geometry {
            assert!((c.x - (-8_348_961.809)).abs() < 2_500.0);
            assert!((c.y - 5_621_521.486).abs() < 2_500.0);
        } else {
            panic!("expected Point");
        }
    }

    #[test]
    fn reproject_roundtrip_lonlat_stays_close() {
        let mut layer = Layer::new("line")
            .with_geom_type(GeometryType::LineString)
            .with_crs_epsg(4326);
        layer
            .add_feature(
                Some(Geometry::LineString(vec![
                    Coord::xy(-79.0, 43.0),
                    Coord::xy(-78.0, 44.0),
                ])),
                &[],
            )
            .unwrap();

        let to_merc = layer_to_epsg(&layer, 3857).unwrap();
        let back = layer_from_to_epsg(&to_merc, 3857, 4326).unwrap();

        if let Some(Geometry::LineString(cs)) = &back.features[0].geometry {
            assert!((cs[0].x - (-79.0)).abs() < 1e-4);
            assert!((cs[0].y - 43.0).abs() < 1e-4);
            assert!((cs[1].x - (-78.0)).abs() < 1e-4);
            assert!((cs[1].y - 44.0).abs() < 1e-4);
        } else {
            panic!("expected LineString");
        }
    }

    #[test]
    fn reproject_geometry_collection() {
        let mut layer = Layer::new("gc")
            .with_geom_type(GeometryType::GeometryCollection)
            .with_crs_epsg(4326);
        layer
            .add_feature(
                Some(Geometry::GeometryCollection(vec![
                    Geometry::point(-75.0, 45.0),
                    Geometry::LineString(vec![Coord::xy(-75.0, 45.0), Coord::xy(-74.0, 46.0)]),
                ])),
                &[],
            )
            .unwrap();

        let out = layer_to_epsg(&layer, 3857).unwrap();
        assert_eq!(out.crs_epsg(), Some(3857));
        assert!(matches!(
            out.features[0].geometry,
            Some(Geometry::GeometryCollection(_))
        ));
    }

    #[test]
    fn failure_policy_sets_null_geometry() {
        let mut layer = Layer::new("bad")
            .with_geom_type(GeometryType::Point)
            .with_crs_epsg(4326);
        layer.add_field(FieldDef::new("name", FieldType::Text));
        layer
            .add_feature(Some(Geometry::Point(Coord { x: f64::NAN, y: 0.0, z: None, m: None })), &[("name", "bad".into())])
            .unwrap();
        layer
            .add_feature(Some(Geometry::point(-75.0, 45.0)), &[("name", "ok".into())])
            .unwrap();

        let opts = VectorReprojectOptions::new()
            .with_failure_policy(TransformFailurePolicy::SetNullGeometry);
        let out = layer_to_epsg_with_options(&layer, 3857, &opts).unwrap();

        assert_eq!(out.features.len(), 2);
        assert!(out.features[0].geometry.is_none());
        assert!(out.features[1].geometry.is_some());
    }

    #[test]
    fn failure_policy_skips_feature() {
        let mut layer = Layer::new("bad")
            .with_geom_type(GeometryType::Point)
            .with_crs_epsg(4326);
        layer
            .add_feature(Some(Geometry::Point(Coord { x: f64::NAN, y: 0.0, z: None, m: None })), &[])
            .unwrap();
        layer
            .add_feature(Some(Geometry::point(-75.0, 45.0)), &[])
            .unwrap();

        let opts = VectorReprojectOptions::new()
            .with_failure_policy(TransformFailurePolicy::SkipFeature);
        let out = layer_to_epsg_with_options(&layer, 3857, &opts).unwrap();

        assert_eq!(out.features.len(), 1);
    }

    #[test]
    fn reproject_with_progress_emits_feature_updates() {
        let mut layer = Layer::new("cities")
            .with_geom_type(GeometryType::Point)
            .with_crs_epsg(4326);
        layer.add_feature(Some(Geometry::point(-75.0, 45.0)), &[]).unwrap();
        layer.add_feature(Some(Geometry::point(-74.0, 46.0)), &[]).unwrap();
        layer.add_feature(Some(Geometry::point(-73.0, 47.0)), &[]).unwrap();

        let progress_values: Arc<Mutex<Vec<f64>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&progress_values);

        let out = layer_to_epsg_with_options_and_progress(
            &layer,
            3857,
            &VectorReprojectOptions::default(),
            move |pct| {
                sink.lock().unwrap().push(pct);
            },
        )
        .unwrap();

        let values = progress_values.lock().unwrap();
        assert_eq!(out.features.len(), 3);
        assert!(!values.is_empty());
        assert_eq!(values.len(), layer.features.len() + 1);
        assert!(values.iter().all(|v| v.is_finite() && *v >= 0.0 && *v <= 1.0));
        assert!((values.last().copied().unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn antimeridian_policy_normalizes_lon_for_4326() {
        let mut layer = Layer::new("lon")
            .with_geom_type(GeometryType::Point)
            .with_crs_epsg(4326);
        layer
            .add_feature(Some(Geometry::point(190.0, 10.0)), &[])
            .unwrap();

        let opts = VectorReprojectOptions::new()
            .with_antimeridian_policy(AntimeridianPolicy::NormalizeLon180);
        let out = layer_from_to_epsg_with_options(&layer, 4326, 4326, &opts).unwrap();

        if let Some(Geometry::Point(c)) = &out.features[0].geometry {
            assert!((c.x - (-170.0)).abs() < 1e-9);
        } else {
            panic!("expected Point");
        }
    }

    #[test]
    fn densification_adds_vertices_to_lines() {
        let mut layer = Layer::new("line")
            .with_geom_type(GeometryType::LineString)
            .with_crs_epsg(4326);
        layer
            .add_feature(
                Some(Geometry::LineString(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
                &[],
            )
            .unwrap();

        let opts = VectorReprojectOptions::new().with_max_segment_length(0.25);
        let out = layer_from_to_epsg_with_options(&layer, 4326, 4326, &opts).unwrap();

        if let Some(Geometry::LineString(cs)) = &out.features[0].geometry {
            assert_eq!(cs.len(), 5);
        } else {
            panic!("expected LineString");
        }
    }

    #[test]
    fn densification_adds_vertices_to_rings() {
        let mut layer = Layer::new("poly")
            .with_geom_type(GeometryType::Polygon)
            .with_crs_epsg(4326);
        layer
            .add_feature(
                Some(Geometry::polygon(
                    vec![
                        Coord::xy(0.0, 0.0),
                        Coord::xy(2.0, 0.0),
                        Coord::xy(2.0, 2.0),
                        Coord::xy(0.0, 2.0),
                    ],
                    vec![],
                )),
                &[],
            )
            .unwrap();

        let opts = VectorReprojectOptions::new().with_max_segment_length(0.5);
        let out = layer_from_to_epsg_with_options(&layer, 4326, 4326, &opts).unwrap();

        if let Some(Geometry::Polygon { exterior, .. }) = &out.features[0].geometry {
            assert!(exterior.coords().len() > 4);
        } else {
            panic!("expected Polygon");
        }
    }

    #[test]
    fn invalid_densification_length_errors() {
        let layer = sample_point_layer();
        let opts = VectorReprojectOptions::new().with_max_segment_length(0.0);
        let err = layer_to_epsg_with_options(&layer, 3857, &opts).unwrap_err();
        assert!(format!("{err}").contains("max_segment_length"));
    }

    #[test]
    fn antimeridian_split_turns_linestring_into_multiline() {
        let mut layer = Layer::new("cross")
            .with_geom_type(GeometryType::LineString)
            .with_crs_epsg(4326);
        layer
            .add_feature(
                Some(Geometry::LineString(vec![Coord::xy(179.0, 10.0), Coord::xy(-179.0, 10.0)])),
                &[],
            )
            .unwrap();

        let opts = VectorReprojectOptions::new()
            .with_antimeridian_policy(AntimeridianPolicy::SplitAt180);
        let out = layer_from_to_epsg_with_options(&layer, 4326, 4326, &opts).unwrap();

        match &out.features[0].geometry {
            Some(Geometry::MultiLineString(parts)) => {
                assert_eq!(parts.len(), 2);
                assert!((parts[0].last().unwrap().x.abs() - 180.0).abs() < 1e-9);
                assert!((parts[1].first().unwrap().x.abs() - 180.0).abs() < 1e-9);
            }
            other => panic!("expected MultiLineString, got {other:?}"),
        }
    }

    #[test]
    fn polygon_crossing_with_split_policy_can_be_null_geometry() {
        let mut layer = Layer::new("poly_cross")
            .with_geom_type(GeometryType::Polygon)
            .with_crs_epsg(4326);
        layer
            .add_feature(
                Some(Geometry::polygon(
                    vec![
                        Coord::xy(179.0, 0.0),
                        Coord::xy(-179.0, 0.0),
                        Coord::xy(-179.0, 1.0),
                        Coord::xy(179.0, 1.0),
                    ],
                    vec![],
                )),
                &[],
            )
            .unwrap();

        let opts = VectorReprojectOptions::new()
            .with_antimeridian_policy(AntimeridianPolicy::SplitAt180)
            .with_failure_policy(TransformFailurePolicy::SetNullGeometry);
        let out = layer_from_to_epsg_with_options(&layer, 4326, 4326, &opts).unwrap();

        assert!(matches!(
            out.features[0].geometry,
            Some(Geometry::MultiPolygon(_))
        ));
    }

    #[test]
    fn polygon_with_hole_crossing_is_split_and_preserved() {
        let mut layer = Layer::new("poly_hole")
            .with_geom_type(GeometryType::Polygon)
            .with_crs_epsg(4326);
        layer
            .add_feature(
                Some(Geometry::polygon(
                    vec![
                        Coord::xy(179.0, 0.0),
                        Coord::xy(-179.0, 0.0),
                        Coord::xy(-179.0, 2.0),
                        Coord::xy(179.0, 2.0),
                    ],
                    vec![vec![
                        Coord::xy(179.5, 0.5),
                        Coord::xy(-179.5, 0.5),
                        Coord::xy(-179.5, 1.5),
                        Coord::xy(179.5, 1.5),
                    ]],
                )),
                &[],
            )
            .unwrap();

        let opts = VectorReprojectOptions::new()
            .with_antimeridian_policy(AntimeridianPolicy::SplitAt180)
            .with_failure_policy(TransformFailurePolicy::SetNullGeometry);
        let out = layer_from_to_epsg_with_options(&layer, 4326, 4326, &opts).unwrap();

        match &out.features[0].geometry {
            Some(Geometry::MultiPolygon(parts)) => {
                assert!(parts.len() >= 2);
                let total_holes: usize = parts.iter().map(|(_, holes)| holes.len()).sum();
                assert!(total_holes >= 1);
            }
            other => panic!("expected MultiPolygon, got {other:?}"),
        }
    }

    #[test]
    fn polygon_with_non_crossing_hole_is_preserved_across_split_parts() {
        let mut layer = Layer::new("poly_hole_ok")
            .with_geom_type(GeometryType::Polygon)
            .with_crs_epsg(4326);
        layer
            .add_feature(
                Some(Geometry::polygon(
                    vec![
                        Coord::xy(179.0, 0.0),
                        Coord::xy(-179.0, 0.0),
                        Coord::xy(-179.0, 4.0),
                        Coord::xy(179.0, 4.0),
                    ],
                    vec![vec![
                        Coord::xy(179.1, 1.0),
                        Coord::xy(179.4, 1.0),
                        Coord::xy(179.4, 2.0),
                        Coord::xy(179.1, 2.0),
                    ]],
                )),
                &[],
            )
            .unwrap();

        let opts = VectorReprojectOptions::new()
            .with_antimeridian_policy(AntimeridianPolicy::SplitAt180);
        let out = layer_from_to_epsg_with_options(&layer, 4326, 4326, &opts).unwrap();

        match &out.features[0].geometry {
            Some(Geometry::MultiPolygon(parts)) => {
                assert!(parts.len() >= 2);
                let total_holes: usize = parts.iter().map(|(_, holes)| holes.len()).sum();
                assert_eq!(total_holes, 1);
            }
            other => panic!("expected MultiPolygon, got {other:?}"),
        }
    }

    #[test]
    fn topology_validate_rejects_hole_outside() {
        let mut layer = Layer::new("bad_hole")
            .with_geom_type(GeometryType::Polygon)
            .with_crs_epsg(4326);
        layer
            .add_feature(
                Some(Geometry::polygon(
                    vec![
                        Coord::xy(0.0, 0.0),
                        Coord::xy(10.0, 0.0),
                        Coord::xy(10.0, 10.0),
                        Coord::xy(0.0, 10.0),
                    ],
                    vec![vec![
                        Coord::xy(20.0, 20.0),
                        Coord::xy(21.0, 20.0),
                        Coord::xy(21.0, 21.0),
                        Coord::xy(20.0, 21.0),
                    ]],
                )),
                &[],
            )
            .unwrap();

        let opts = VectorReprojectOptions::new().with_topology_policy(TopologyPolicy::Validate);
        let err = layer_from_to_epsg_with_options(&layer, 4326, 4326, &opts).unwrap_err();
        assert!(format!("{err}").contains("interior ring lies outside"));
    }

    #[test]
    fn topology_fix_orients_rings() {
        let mut layer = Layer::new("orient")
            .with_geom_type(GeometryType::Polygon)
            .with_crs_epsg(4326);

        let exterior_cw = vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(0.0, 10.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(10.0, 0.0),
        ];
        let hole_ccw = vec![
            Coord::xy(3.0, 3.0),
            Coord::xy(7.0, 3.0),
            Coord::xy(7.0, 7.0),
            Coord::xy(3.0, 7.0),
        ];

        layer
            .add_feature(Some(Geometry::polygon(exterior_cw, vec![hole_ccw])), &[])
            .unwrap();

        let opts = VectorReprojectOptions::new().with_topology_policy(TopologyPolicy::ValidateAndFixOrientation);
        let out = layer_from_to_epsg_with_options(&layer, 4326, 4326, &opts).unwrap();

        if let Some(Geometry::Polygon { exterior, interiors }) = &out.features[0].geometry {
            assert!(ring_signed_area(exterior.coords()) > 0.0);
            assert!(ring_signed_area(interiors[0].coords()) < 0.0);
        } else {
            panic!("expected Polygon");
        }
    }
}
