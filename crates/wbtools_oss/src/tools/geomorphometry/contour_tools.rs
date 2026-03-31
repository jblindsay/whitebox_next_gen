use std::cmp;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use serde_json::json;
use wbcore::{
    parse_raster_path_arg, parse_vector_path_arg, LicenseTier, Tool, ToolArgs, ToolCategory,
    ToolContext, ToolError, ToolManifest, ToolMetadata, ToolParamSpec, ToolRunResult,
    ToolStability,
};
use wbraster::Raster;
use wbtopology::{delaunay_triangulation, Coord as TopoCoord};
use wbvector::{Coord, Crs, FieldDef, FieldType, FieldValue, Geometry, GeometryType, Layer, VectorFormat};

use crate::memory_store;

pub struct ContoursFromRasterTool;
pub struct ContoursFromPointsTool;
pub struct TopographicHachuresTool;

const EPS: f64 = 1.0e-12;

#[derive(Clone)]
struct Segment {
    a: Coord,
    b: Coord,
    level_idx: i64,
    z: f64,
}

fn load_raster(path: &str) -> Result<Raster, ToolError> {
    if memory_store::raster_is_memory_path(path) {
        let id = memory_store::raster_path_to_id(path)
            .ok_or_else(|| ToolError::Validation("malformed in-memory raster path".to_string()))?;
        return memory_store::get_raster_by_id(id)
            .ok_or_else(|| ToolError::Validation(format!("unknown in-memory raster id '{}'", id)));
    }
    Raster::read(path)
        .map_err(|e| ToolError::Execution(format!("failed reading input raster: {}", e)))
}

fn detect_vector_format(path: &str) -> Result<VectorFormat, ToolError> {
    match VectorFormat::detect(path) {
        Ok(fmt) => Ok(fmt),
        Err(_) => {
            if Path::new(path).extension().is_none() {
                Ok(VectorFormat::Shapefile)
            } else {
                Err(ToolError::Validation(format!(
                    "could not determine vector output format from path '{}'",
                    path
                )))
            }
        }
    }
}

fn ensure_parent_dir(path: &str) -> Result<(), ToolError> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ToolError::Execution(format!("failed creating output directory: {}", e)))?;
        }
    }
    Ok(())
}

fn write_vector(layer: &Layer, path: &str) -> Result<String, ToolError> {
    let fmt = detect_vector_format(path)?;
    wbvector::write(layer, path, fmt)
        .map_err(|e| ToolError::Execution(format!("failed writing output vector: {}", e)))?;
    Ok(path.to_string())
}

fn build_result(path: String) -> ToolRunResult {
    let mut outputs = BTreeMap::new();
    outputs.insert("path".to_string(), json!(path));
    ToolRunResult {
        outputs,
        ..Default::default()
    }
}

fn endpoint_key(c: &Coord, eps: f64) -> (i64, i64) {
    ((c.x / eps).round() as i64, (c.y / eps).round() as i64)
}

fn add_if_distinct(points: &mut Vec<Coord>, p: Coord, eps: f64) {
    if let Some(last) = points.last() {
        let dx = last.x - p.x;
        let dy = last.y - p.y;
        if (dx * dx + dy * dy).sqrt() <= eps {
            return;
        }
    }
    points.push(p);
}

fn smooth_polyline(points: &[Coord], filter_size: usize) -> Vec<Coord> {
    if points.len() < 3 || filter_size < 3 {
        return points.to_vec();
    }
    let mut f = filter_size.min(21);
    if f % 2 == 0 {
        f += 1;
    }
    let r = (f / 2) as isize;
    let mut out = Vec::with_capacity(points.len());
    for i in 0..points.len() {
        if i == 0 || i + 1 == points.len() {
            out.push(points[i].clone());
            continue;
        }
        let mut sx = 0.0;
        let mut sy = 0.0;
        let mut n = 0.0;
        for j in -r..=r {
            let mut idx = i as isize + j;
            if idx < 0 {
                idx = 0;
            }
            if idx >= points.len() as isize {
                idx = points.len() as isize - 1;
            }
            sx += points[idx as usize].x;
            sy += points[idx as usize].y;
            n += 1.0;
        }
        out.push(Coord::xy(sx / n, sy / n));
    }
    out
}

fn simplify_by_deflection(points: &[Coord], min_deflection_deg: f64) -> Vec<Coord> {
    if points.len() <= 2 || min_deflection_deg <= 0.0 {
        return points.to_vec();
    }
    let threshold = min_deflection_deg.to_radians();
    let mut kept = Vec::with_capacity(points.len());
    kept.push(points[0].clone());
    for i in 1..(points.len() - 1) {
        let a = &points[i - 1];
        let b = &points[i];
        let c = &points[i + 1];
        let v1x = b.x - a.x;
        let v1y = b.y - a.y;
        let v2x = c.x - b.x;
        let v2y = c.y - b.y;
        let m1 = (v1x * v1x + v1y * v1y).sqrt();
        let m2 = (v2x * v2x + v2y * v2y).sqrt();
        if m1 <= EPS || m2 <= EPS {
            continue;
        }
        let dot = (v1x * v2x + v1y * v2y) / (m1 * m2);
        let angle = dot.clamp(-1.0, 1.0).acos();
        let deflection = std::f64::consts::PI - angle;
        if deflection >= threshold {
            kept.push(points[i].clone());
        }
    }
    kept.push(points[points.len() - 1].clone());
    if kept.len() < 2 {
        points.to_vec()
    } else {
        kept
    }
}

fn chain_segments(segments: &[Segment], eps: f64) -> Vec<(Vec<Coord>, f64)> {
    let mut by_level: HashMap<i64, Vec<usize>> = HashMap::new();
    for (i, s) in segments.iter().enumerate() {
        by_level.entry(s.level_idx).or_default().push(i);
    }

    let mut all_lines = Vec::new();
    for (_, ids) in by_level {
        let mut endpoint_map: HashMap<(i64, i64), Vec<(usize, bool)>> = HashMap::new();
        for &sid in &ids {
            let s = &segments[sid];
            endpoint_map
                .entry(endpoint_key(&s.a, eps))
                .or_default()
                .push((sid, true));
            endpoint_map
                .entry(endpoint_key(&s.b, eps))
                .or_default()
                .push((sid, false));
        }

        let mut visited: HashMap<usize, bool> = HashMap::new();
        for &sid in &ids {
            if visited.get(&sid).copied().unwrap_or(false) {
                continue;
            }
            visited.insert(sid, true);
            let s = &segments[sid];
            let z = s.z;
            let mut line = vec![s.a.clone(), s.b.clone()];

            let extend = |front: bool,
                          line: &mut Vec<Coord>,
                          visited: &mut HashMap<usize, bool>| {
                loop {
                    let key = if front {
                        endpoint_key(&line[0], eps)
                    } else {
                        endpoint_key(line.last().unwrap(), eps)
                    };
                    let Some(cands) = endpoint_map.get(&key) else {
                        break;
                    };

                    let mut next_seg: Option<(usize, Coord)> = None;
                    for (cand_id, is_start) in cands {
                        if visited.get(cand_id).copied().unwrap_or(false) {
                            continue;
                        }
                        let cand = &segments[*cand_id];
                        let next_point = if *is_start {
                            cand.b.clone()
                        } else {
                            cand.a.clone()
                        };
                        next_seg = Some((*cand_id, next_point));
                        break;
                    }

                    if let Some((nid, np)) = next_seg {
                        visited.insert(nid, true);
                        if front {
                            if endpoint_key(&np, eps) != endpoint_key(&line[0], eps) {
                                line.insert(0, np);
                            } else {
                                break;
                            }
                        } else if endpoint_key(&np, eps) != endpoint_key(line.last().unwrap(), eps)
                        {
                            line.push(np);
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            };

            extend(false, &mut line, &mut visited);
            extend(true, &mut line, &mut visited);

            let mut cleaned = Vec::with_capacity(line.len());
            for p in line {
                add_if_distinct(&mut cleaned, p, eps);
            }
            if cleaned.len() > 1 {
                all_lines.push((cleaned, z));
            }
        }
    }

    all_lines
}

fn interpolate_edge(p1: Coord, z1: f64, p2: Coord, z2: f64, z: f64) -> Option<Coord> {
    let dz = z2 - z1;
    if dz.abs() <= EPS {
        return None;
    }
    let t = (z - z1) / dz;
    if !(-EPS..=1.0 + EPS).contains(&t) {
        return None;
    }
    Some(Coord::xy(
        p1.x + t * (p2.x - p1.x),
        p1.y + t * (p2.y - p1.y),
    ))
}

fn marching_segments_for_cell(
    p00: Coord,
    p10: Coord,
    p11: Coord,
    p01: Coord,
    z00: f64,
    z10: f64,
    z11: f64,
    z01: f64,
    level_idx: i64,
    z: f64,
) -> Vec<Segment> {
    let mut case_idx = 0u8;
    if z00 > z {
        case_idx |= 1;
    }
    if z10 > z {
        case_idx |= 2;
    }
    if z11 > z {
        case_idx |= 4;
    }
    if z01 > z {
        case_idx |= 8;
    }

    let e0 = interpolate_edge(p00.clone(), z00, p10.clone(), z10, z);
    let e1 = interpolate_edge(p10, z10, p11.clone(), z11, z);
    let e2 = interpolate_edge(p01.clone(), z01, p11, z11, z);
    let e3 = interpolate_edge(p00, z00, p01, z01, z);

    let mut out = Vec::new();
    let mut push_seg = |a: Option<Coord>, b: Option<Coord>| {
        if let (Some(pa), Some(pb)) = (a, b) {
            out.push(Segment {
                a: pa,
                b: pb,
                level_idx,
                z,
            });
        }
    };

    match case_idx {
        0 | 15 => {}
        1 => push_seg(e3, e0),
        2 => push_seg(e0, e1),
        3 => push_seg(e3, e1),
        4 => push_seg(e1, e2),
        5 => {
            push_seg(e3.clone(), e2.clone());
            push_seg(e0, e1);
        }
        6 => push_seg(e0, e2),
        7 => push_seg(e3, e2),
        8 => push_seg(e2, e3),
        9 => push_seg(e0, e2),
        10 => {
            push_seg(e0.clone(), e3.clone());
            push_seg(e1, e2);
        }
        11 => push_seg(e1, e2),
        12 => push_seg(e1, e3),
        13 => push_seg(e0, e1),
        14 => push_seg(e0, e3),
        _ => {}
    }

    out
}

fn parse_contour_levels(min_z: f64, max_z: f64, interval: f64, base: f64) -> Option<(i64, i64)> {
    if interval <= 0.0 || !interval.is_finite() {
        return None;
    }
    let lower = ((min_z - base) / interval).ceil() as i64;
    let upper = ((max_z - base) / interval).floor() as i64;
    if lower > upper {
        None
    } else {
        Some((lower, upper))
    }
}

fn output_path_arg(args: &ToolArgs) -> Result<String, ToolError> {
    args.get("output")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ToolError::Validation("missing required parameter 'output' for vector output".to_string()))
}

fn raster_contour_layer(
    raster: &Raster,
    interval: f64,
    base: f64,
    smooth: usize,
    deflection_tolerance: f64,
) -> Result<Layer, ToolError> {
    if raster.rows < 2 || raster.cols < 2 {
        return Err(ToolError::Validation("input raster must have at least 2 rows and 2 cols".to_string()));
    }

    let mut segments = Vec::<Segment>::new();
    let half_x = raster.cell_size_x * 0.5;
    let half_y = raster.cell_size_y * 0.5;

    for row in 0..(raster.rows - 1) {
        for col in 0..(raster.cols - 1) {
            let z00 = raster.get(0, row as isize, col as isize);
            let z10 = raster.get(0, row as isize, (col + 1) as isize);
            let z11 = raster.get(0, (row + 1) as isize, (col + 1) as isize);
            let z01 = raster.get(0, (row + 1) as isize, col as isize);
            if raster.is_nodata(z00) || raster.is_nodata(z10) || raster.is_nodata(z11) || raster.is_nodata(z01) {
                continue;
            }

            let min_z = z00.min(z10.min(z11.min(z01)));
            let max_z = z00.max(z10.max(z11.max(z01)));
            let Some((lower, upper)) = parse_contour_levels(min_z, max_z, interval, base) else {
                continue;
            };

            let cx = raster.col_center_x(col as isize);
            let cy = raster.row_center_y(row as isize);
            let p00 = Coord::xy(cx - half_x, cy + half_y);
            let p10 = Coord::xy(cx + half_x, cy + half_y);
            let p11 = Coord::xy(cx + half_x, cy - half_y);
            let p01 = Coord::xy(cx - half_x, cy - half_y);

            for level_idx in lower..=upper {
                let z = base + level_idx as f64 * interval;
                let cell_segments = marching_segments_for_cell(
                    p00.clone(),
                    p10.clone(),
                    p11.clone(),
                    p01.clone(),
                    z00,
                    z10,
                    z11,
                    z01,
                    level_idx,
                    z,
                );
                segments.extend(cell_segments);
            }
        }
    }

    let eps = (raster.cell_size_x.abs() + raster.cell_size_y.abs()).max(1.0) * 1.0e-9;
    let mut lines = chain_segments(&segments, eps);

    let mut layer = Layer::new("contours").with_geom_type(GeometryType::LineString);
    if raster.crs.epsg.is_some() || raster.crs.wkt.is_some() {
        layer.crs = Some(Crs {
            epsg: raster.crs.epsg,
            wkt: raster.crs.wkt.clone(),
        });
    }
    layer.add_field(FieldDef::new("FID", FieldType::Integer));
    layer.add_field(FieldDef::new("HEIGHT", FieldType::Float));

    let mut fid = 1i64;
    for (pts, z) in lines.drain(..) {
        let smoothed = smooth_polyline(&pts, smooth);
        let simplified = simplify_by_deflection(&smoothed, deflection_tolerance);
        if simplified.len() < 2 {
            continue;
        }
        layer
            .add_feature(
                Some(Geometry::line_string(simplified)),
                &[("FID", FieldValue::Integer(fid)), ("HEIGHT", FieldValue::Float(z))],
            )
            .map_err(|e| ToolError::Execution(format!("failed creating contour feature: {}", e)))?;
        fid += 1;
    }

    Ok(layer)
}

fn points_contour_layer(
    input: &Layer,
    field_name: Option<&str>,
    use_z_values: bool,
    max_triangle_edge_length: f64,
    interval: f64,
    base: f64,
    smooth: usize,
) -> Result<Layer, ToolError> {
    let mut points = Vec::<Coord>::new();
    let mut z_values = Vec::<f64>::new();

    let field_idx = if !use_z_values {
        if let Some(name) = field_name {
            Some(
                input
                    .schema
                    .field_index(name)
                    .ok_or_else(|| ToolError::Validation(format!("field '{}' does not exist", name)))?,
            )
        } else {
            input
                .schema
                .fields()
                .iter()
                .enumerate()
                .find(|(_, f)| matches!(f.field_type, FieldType::Integer | FieldType::Float))
                .map(|(i, _)| i)
        }
    } else {
        None
    };

    if !use_z_values && field_idx.is_none() {
        return Err(ToolError::Validation(
            "'field_name' must be provided (or input must contain at least one numeric field) when use_z_values=false".to_string(),
        ));
    }

    for feat in &input.features {
        let Some(geom) = &feat.geometry else { continue; };

        let base_z = if !use_z_values {
            let idx = field_idx.unwrap();
            feat.attributes
                .get(idx)
                .and_then(|v| v.as_f64())
                .ok_or_else(|| ToolError::Validation("encountered non-numeric attribute value in contour field".to_string()))?
        } else {
            0.0
        };

        match geom {
            Geometry::Point(c) => {
                let z = if use_z_values {
                    c.z.ok_or_else(|| {
                        ToolError::Validation(
                            "input point geometry missing Z value while use_z_values=true".to_string(),
                        )
                    })?
                } else {
                    base_z
                };
                points.push(Coord::xy(c.x, c.y));
                z_values.push(z);
            }
            Geometry::MultiPoint(cs) => {
                for c in cs {
                    let z = if use_z_values {
                        c.z.ok_or_else(|| {
                            ToolError::Validation(
                                "input multipoint geometry missing Z value while use_z_values=true".to_string(),
                            )
                        })?
                    } else {
                        base_z
                    };
                    points.push(Coord::xy(c.x, c.y));
                    z_values.push(z);
                }
            }
            _ => {
                return Err(ToolError::Validation(
                    "input vector must be POINT or MULTIPOINT geometry".to_string(),
                ));
            }
        }
    }

    if points.len() < 3 {
        return Err(ToolError::Validation("too few input points for triangulation".to_string()));
    }

    let topo_points: Vec<TopoCoord> = points.iter().map(|p| TopoCoord::xy(p.x, p.y)).collect();
    let tri = delaunay_triangulation(&topo_points, 1.0e-10);
    if tri.triangles.is_empty() {
        return Err(ToolError::Validation("triangulation failed or produced no triangles".to_string()));
    }

    let mut z_lookup: HashMap<(i64, i64), f64> = HashMap::new();
    for (p, z) in points.iter().zip(z_values.iter()) {
        z_lookup.entry(endpoint_key(p, 1.0e-9)).or_insert(*z);
    }

    let max_edge2 = if max_triangle_edge_length.is_finite() && max_triangle_edge_length > 0.0 {
        max_triangle_edge_length * max_triangle_edge_length
    } else {
        f64::INFINITY
    };

    let mut segments = Vec::<Segment>::new();
    for t in &tri.triangles {
        let p1 = t[0];
        let p2 = t[1];
        let p3 = t[2];
        let a = &tri.points[p1];
        let b = &tri.points[p2];
        let c = &tri.points[p3];
        let pa = Coord::xy(a.x, a.y);
        let pb = Coord::xy(b.x, b.y);
        let pc = Coord::xy(c.x, c.y);

        let z1 = *z_lookup
            .get(&endpoint_key(&pa, 1.0e-9))
            .ok_or_else(|| ToolError::Execution("failed mapping triangulation vertex elevation".to_string()))?;
        let z2 = *z_lookup
            .get(&endpoint_key(&pb, 1.0e-9))
            .ok_or_else(|| ToolError::Execution("failed mapping triangulation vertex elevation".to_string()))?;
        let z3 = *z_lookup
            .get(&endpoint_key(&pc, 1.0e-9))
            .ok_or_else(|| ToolError::Execution("failed mapping triangulation vertex elevation".to_string()))?;

        let d12 = (pa.x - pb.x).powi(2) + (pa.y - pb.y).powi(2);
        let d23 = (pb.x - pc.x).powi(2) + (pb.y - pc.y).powi(2);
        let d13 = (pa.x - pc.x).powi(2) + (pa.y - pc.y).powi(2);
        if d12.max(d23.max(d13)) > max_edge2 {
            continue;
        }

        let min_z = z1.min(z2.min(z3));
        let max_z = z1.max(z2.max(z3));
        let Some((lower, upper)) = parse_contour_levels(min_z, max_z, interval, base) else {
            continue;
        };

        for level_idx in lower..=upper {
            let z = base + level_idx as f64 * interval;
            let mut ints = Vec::<Coord>::new();
            if let Some(p) = interpolate_edge(pa.clone(), z1, pb.clone(), z2, z) {
                ints.push(p);
            }
            if let Some(p) = interpolate_edge(pb.clone(), z2, pc.clone(), z3, z) {
                ints.push(p);
            }
            if let Some(p) = interpolate_edge(pa.clone(), z1, pc.clone(), z3, z) {
                ints.push(p);
            }
            // Deduplicate triangle-vertex intersections.
            let mut unique = Vec::<Coord>::new();
            let eps = 1.0e-9;
            for p in ints {
                if !unique.iter().any(|u| {
                    let dx = u.x - p.x;
                    let dy = u.y - p.y;
                    (dx * dx + dy * dy).sqrt() <= eps
                }) {
                    unique.push(p);
                }
            }
            if unique.len() == 2 {
                segments.push(Segment {
                    a: unique[0].clone(),
                    b: unique[1].clone(),
                    level_idx,
                    z,
                });
            }
        }
    }

    let mut lines = chain_segments(&segments, 1.0e-9);

    let mut layer = Layer::new("contours").with_geom_type(GeometryType::LineString);
    layer.crs = input.crs.clone();
    layer.add_field(FieldDef::new("FID", FieldType::Integer));
    layer.add_field(FieldDef::new("HEIGHT", FieldType::Float));

    let mut fid = 1i64;
    for (pts, z) in lines.drain(..) {
        let smoothed = smooth_polyline(&pts, smooth);
        if smoothed.len() < 2 {
            continue;
        }
        layer
            .add_feature(
                Some(Geometry::line_string(smoothed)),
                &[("FID", FieldValue::Integer(fid)), ("HEIGHT", FieldValue::Float(z))],
            )
            .map_err(|e| ToolError::Execution(format!("failed creating contour feature: {}", e)))?;
        fid += 1;
    }

    Ok(layer)
}

#[derive(Clone)]
struct HachureContour {
    points: Vec<Coord>,
    value: f64,
    closed: bool,
}

struct RasterCoverage<'a> {
    raster: &'a Raster,
}

impl<'a> RasterCoverage<'a> {
    fn new(raster: &'a Raster) -> Self {
        Self { raster }
    }

    fn cell_coords(&self, x: f64, y: f64) -> Option<(isize, isize, f64, f64)> {
        let col_f = (x - self.raster.x_min) / self.raster.cell_size_x - 0.5;
        let row_f = (self.raster.y_max() - y) / self.raster.cell_size_y - 0.5;
        if col_f < 0.0 || row_f < 0.0 {
            return None;
        }
        let col0 = col_f.floor() as isize;
        let row0 = row_f.floor() as isize;
        let col1 = col0 + 1;
        let row1 = row0 + 1;
        if row1 >= self.raster.rows as isize || col1 >= self.raster.cols as isize {
            return None;
        }
        Some((row0, col0, col_f - col0 as f64, row_f - row0 as f64))
    }

    fn value(&self, x: f64, y: f64) -> Option<f64> {
        let (row0, col0, tx, ty) = self.cell_coords(x, y)?;
        let row1 = row0 + 1;
        let col1 = col0 + 1;
        let z00 = self.raster.get(0, row0, col0);
        let z10 = self.raster.get(0, row1, col0);
        let z01 = self.raster.get(0, row0, col1);
        let z11 = self.raster.get(0, row1, col1);
        if self.raster.is_nodata(z00)
            || self.raster.is_nodata(z10)
            || self.raster.is_nodata(z01)
            || self.raster.is_nodata(z11)
        {
            return None;
        }
        let north = z00 * (1.0 - tx) + z01 * tx;
        let south = z10 * (1.0 - tx) + z11 * tx;
        Some(north * (1.0 - ty) + south * ty)
    }

    fn gradient(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let (row0, col0, tx, ty) = self.cell_coords(x, y)?;
        let row1 = row0 + 1;
        let col1 = col0 + 1;
        let z00 = self.raster.get(0, row0, col0);
        let z10 = self.raster.get(0, row1, col0);
        let z01 = self.raster.get(0, row0, col1);
        let z11 = self.raster.get(0, row1, col1);
        if self.raster.is_nodata(z00)
            || self.raster.is_nodata(z10)
            || self.raster.is_nodata(z01)
            || self.raster.is_nodata(z11)
        {
            return None;
        }
        let dzdx = ((z01 - z00) * (1.0 - ty) + (z11 - z10) * ty) / self.raster.cell_size_x;
        let dzdy = ((z00 - z10) * (1.0 - tx) + (z01 - z11) * tx) / self.raster.cell_size_y;
        Some((dzdx, dzdy))
    }

    fn slope(&self, x: f64, y: f64) -> Option<f64> {
        let (dx, dy) = self.gradient(x, y)?;
        Some((dx * dx + dy * dy).sqrt())
    }
}

fn coord_distance(a: &Coord, b: &Coord) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

fn midpoint(a: &Coord, b: &Coord) -> Coord {
    Coord::xy((a.x + b.x) * 0.5, (a.y + b.y) * 0.5)
}

fn path_turn(previous: &Coord, current: &Coord, next: &Coord) -> f64 {
    let v1x = current.x - previous.x;
    let v1y = current.y - previous.y;
    let v2x = next.x - current.x;
    let v2y = next.y - current.y;
    let m1 = (v1x * v1x + v1y * v1y).sqrt();
    let m2 = (v2x * v2x + v2y * v2y).sqrt();
    if m1 <= EPS || m2 <= EPS {
        return 1.0;
    }
    ((v1x * v2x + v1y * v2y) / (m1 * m2)).clamp(-1.0, 1.0)
}

fn point_side(p1: &Coord, p2: &Coord, p3: &Coord) -> bool {
    (p3.x - p1.x) * (p2.y - p1.y) < (p3.y - p1.y) * (p2.x - p1.x)
}

fn is_intersection(p1: &Coord, p2: &Coord, p3: &Coord, p4: &Coord) -> bool {
    (point_side(p1, p2, p3) != point_side(p1, p2, p4))
        && (point_side(p3, p4, p1) != point_side(p3, p4, p2))
}

fn intersection_idx(new_line: &[Coord], lines: &[Vec<Coord>], dist: f64) -> usize {
    let mut min_idx = new_line.len();
    for line in lines.iter().rev() {
        if line.len() < 2 {
            continue;
        }
        let d1 = coord_distance(&new_line[0], &new_line[new_line.len() - 1]);
        let d2 = coord_distance(&line[0], &line[line.len() - 1]);
        let c1 = midpoint(&new_line[0], &new_line[new_line.len() - 1]);
        let c2 = midpoint(&line[0], &line[line.len() - 1]);
        let d3 = coord_distance(&c1, &c2);
        if d3 < (d1 + d2) * 0.5 {
            for i in 1..new_line.len() {
                for j in 1..line.len() {
                    if coord_distance(&new_line[i], &line[j]) < dist
                        || is_intersection(&new_line[i - 1], &new_line[i], &line[j - 1], &line[j])
                    {
                        min_idx = min_idx.min(i);
                        if min_idx == 1 {
                            return min_idx;
                        }
                    }
                }
            }
        }
    }
    min_idx
}

fn get_flowline(
    coverage: &RasterCoverage<'_>,
    start: &Coord,
    discretization: f64,
    z_limit: f64,
    slope_min: f64,
    turn_min_cos: f64,
    down: bool,
) -> Vec<Coord> {
    let mut points = Vec::new();
    let mut p1 = start.clone();
    let mut z_prev = match coverage.value(p1.x, p1.y) {
        Some(v) => v,
        None => return points,
    };
    if (z_prev - z_limit).abs() <= EPS {
        return points;
    }
    points.push(p1.clone());
    let sign = if down { 1.0 } else { -1.0 };
    loop {
        let slope = match coverage.slope(p1.x, p1.y) {
            Some(v) => v,
            None => break,
        };
        if slope < slope_min {
            break;
        }

        let (g1x, g1y) = match coverage.gradient(p1.x, p1.y) {
            Some(v) => v,
            None => break,
        };
        let mut p2 = Coord::xy(
            p1.x - sign * discretization * g1x / slope,
            p1.y - sign * discretization * g1y / slope,
        );

        let mut z_cur = match coverage.value(p2.x, p2.y) {
            Some(v) => v,
            None => break,
        };

        if let Some((g2x, g2y)) = coverage.gradient(p2.x, p2.y) {
            let gx = 0.5 * (g1x + g2x);
            let gy = 0.5 * (g1y + g2y);
            let grad_len = (gx * gx + gy * gy).sqrt();
            if grad_len <= EPS {
                break;
            }
            p2 = Coord::xy(
                p1.x - sign * discretization * gx / grad_len,
                p1.y - sign * discretization * gy / grad_len,
            );
            z_cur = match coverage.value(p2.x, p2.y) {
                Some(v) => v,
                None => break,
            };
        }

        if (down && z_cur < z_limit) || (!down && z_cur > z_limit) {
            let denom = z_prev - z_cur;
            if denom.abs() <= EPS {
                break;
            }
            let t = (z_prev - z_limit) / denom;
            points.push(Coord::xy(
                (1.0 - t) * p1.x + t * p2.x,
                (1.0 - t) * p1.y + t * p2.y,
            ));
            break;
        }

        if (down && z_cur < z_prev) || (!down && z_cur > z_prev) {
            points.push(p2.clone());
            p1 = p2;
            z_prev = z_cur;
        } else {
            break;
        }

        let n = points.len();
        if n >= 3 && path_turn(&points[n - 3], &points[n - 2], &points[n - 1]) < turn_min_cos {
            points.pop();
            break;
        }
    }
    points
}

fn insert_flowlines(
    coverage: &RasterCoverage<'_>,
    flowlines: &mut Vec<Vec<Coord>>,
    n1: usize,
    n2: usize,
    k1: usize,
    k2: usize,
    depth: u8,
    dist_min: f64,
    dist_max: f64,
    discretization: f64,
    z_limit: f64,
    slope_min: f64,
    turn_min_cos: f64,
    down: bool,
) {
    if depth == 0 {
        return;
    }
    let n = cmp::min(flowlines[n1].len().saturating_sub(k1), flowlines[n2].len().saturating_sub(k2));
    for i in 0..n {
        let p1 = flowlines[n1][i + k1].clone();
        let p2 = flowlines[n2][i + k2].clone();
        let dist = coord_distance(&p1, &p2);
        if dist >= dist_max {
            let seed = midpoint(&p1, &p2);
            let mut flowline = get_flowline(
                coverage,
                &seed,
                discretization,
                z_limit,
                slope_min,
                turn_min_cos,
                down,
            );
            if flowline.len() > 1 {
                let idx = intersection_idx(&flowline, flowlines, dist_min);
                flowline.truncate(idx);
                if flowline.len() > 1 {
                    flowlines.push(flowline);
                    let last = flowlines.len() - 1;
                    insert_flowlines(
                        coverage,
                        flowlines,
                        n1,
                        last,
                        i + k1,
                        0,
                        depth - 1,
                        dist_min,
                        dist_max,
                        discretization,
                        z_limit,
                        slope_min,
                        turn_min_cos,
                        down,
                    );
                    insert_flowlines(
                        coverage,
                        flowlines,
                        n2,
                        last,
                        i + k2,
                        0,
                        depth - 1,
                        dist_min,
                        dist_max,
                        discretization,
                        z_limit,
                        slope_min,
                        turn_min_cos,
                        down,
                    );
                }
            }
            return;
        }
    }
}

fn contours_to_hachure_contours(layer: &Layer, eps: f64) -> Result<Vec<HachureContour>, ToolError> {
    let height_idx = layer
        .schema
        .field_index("HEIGHT")
        .ok_or_else(|| ToolError::Execution("contour layer missing HEIGHT attribute".to_string()))?;
    let mut contours = Vec::new();
    for feat in &layer.features {
        let value = feat
            .attributes
            .get(height_idx)
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Execution("contour feature has non-numeric HEIGHT".to_string()))?;
        let Some(geom) = &feat.geometry else {
            continue;
        };
        let push_contour = |coords: &[Coord], value: f64, contours: &mut Vec<HachureContour>| {
            if coords.len() < 2 {
                return;
            }
            let mut pts = coords.to_vec();
            let closed = coord_distance(&pts[0], pts.last().unwrap()) <= eps;
            if closed {
                let first = pts[0].clone();
                let last = pts.last().unwrap().clone();
                if coord_distance(&first, &last) > EPS {
                    pts.push(first);
                } else if let Some(end) = pts.last_mut() {
                    *end = first;
                }
            }
            contours.push(HachureContour { points: pts, value, closed });
        };
        match geom {
            Geometry::LineString(coords) => push_contour(coords, value, &mut contours),
            Geometry::MultiLineString(lines) => {
                for coords in lines {
                    push_contour(coords, value, &mut contours);
                }
            }
            _ => {}
        }
    }
    contours.sort_by(|a, b| b.value.total_cmp(&a.value));
    Ok(contours)
}

fn topographic_hachure_layer(
    raster: &Raster,
    interval: f64,
    base: f64,
    smooth: usize,
    deflection_tolerance_deg: f64,
    separation: f64,
    dist_min: f64,
    dist_max: f64,
    discretization: f64,
    turn_max_deg: f64,
    slope_min_deg: f64,
    depth: u8,
) -> Result<Layer, ToolError> {
    let contour_layer = raster_contour_layer(raster, interval, base, smooth, deflection_tolerance_deg)?;
    let res_xy = 0.5 * (raster.cell_size_x.abs() + raster.cell_size_y.abs());
    let coverage = RasterCoverage::new(raster);
    let eps = res_xy.max(1.0) * 1.0e-9;
    let contours = contours_to_hachure_contours(&contour_layer, eps)?;
    let turn_min_cos = turn_max_deg.to_radians().cos();
    let slope_min = slope_min_deg.to_radians().tan();

    let mut output = Layer::new("topographic_hachures").with_geom_type(GeometryType::LineString);
    if raster.crs.epsg.is_some() || raster.crs.wkt.is_some() {
        output.crs = Some(Crs {
            epsg: raster.crs.epsg,
            wkt: raster.crs.wkt.clone(),
        });
    }
    output.add_field(FieldDef::new("FID", FieldType::Integer));
    output.add_field(FieldDef::new("HEIGHT", FieldType::Float));
    output.add_field(FieldDef::new("SLOPE", FieldType::Float));
    output.add_field(FieldDef::new("ASPECT", FieldType::Float));
    output.add_field(FieldDef::new("N", FieldType::Float));
    output.add_field(FieldDef::new("NE", FieldType::Float));
    output.add_field(FieldDef::new("E", FieldType::Float));
    output.add_field(FieldDef::new("SE", FieldType::Float));
    output.add_field(FieldDef::new("S", FieldType::Float));
    output.add_field(FieldDef::new("SW", FieldType::Float));
    output.add_field(FieldDef::new("W", FieldType::Float));
    output.add_field(FieldDef::new("NW", FieldType::Float));

    let mut hid = 1i64;
    let mut flowlines_prev: Vec<Vec<Coord>> = Vec::new();
    let mut flowlines: Vec<Vec<Coord>> = Vec::new();
    let mut starts = std::collections::BTreeSet::new();
    let mut seed_starts = std::collections::BTreeSet::new();
    seed_starts.insert(0usize);
    let mut level_seeds: Vec<Coord> = Vec::new();

    for (idx, contour) in contours.iter().enumerate() {
        let points = &contour.points;
        if points.len() < 2 {
            continue;
        }

        let mut perimeter = 0.0;
        let mut accumulated = vec![0.0; points.len()];
        for i in 1..points.len() {
            perimeter += coord_distance(&points[i - 1], &points[i]);
            accumulated[i] = perimeter;
        }
        if perimeter <= eps {
            continue;
        }

        let target_step = separation * res_xy;
        let raw_num = (perimeter / target_step).max(1.0);
        let lower = raw_num.floor().max(1.0);
        let upper = raw_num.ceil().max(1.0);
        let chosen = if (upper - raw_num) < (raw_num - lower) {
            upper
        } else {
            lower
        };
        let new_step = perimeter / chosen;
        let num_seeds = (perimeter / new_step).round().max(1.0) as usize;
        let discr = discretization * res_xy;
        let value = contour.value;
        let z_min = value - interval;
        let z_max = value + interval;
        let new_dist_min = dist_min * new_step;
        let new_dist_max = dist_max * new_step;

        let mut seeds = Vec::new();
        seeds.push(points[0].clone());
        let mut j = 1usize;
        for seed_idx in 1..num_seeds {
            let dist = seed_idx as f64 * new_step;
            while j < accumulated.len() && dist > accumulated[j] {
                j += 1;
            }
            if j >= accumulated.len() {
                break;
            }
            let seg_len = accumulated[j] - accumulated[j - 1];
            if seg_len <= EPS {
                continue;
            }
            let t = (dist - accumulated[j - 1]) / seg_len;
            let seed = Coord::xy(
                (1.0 - t) * points[j - 1].x + t * points[j].x,
                (1.0 - t) * points[j - 1].y + t * points[j].y,
            );
            seeds.push(seed.clone());
            level_seeds.push(seed);
        }
        seeds.push(points[points.len() - 1].clone());
        level_seeds.push(points[points.len() - 1].clone());

        starts.insert(flowlines.len());
        seed_starts.insert(level_seeds.len());

        for seed in &seeds {
            let mut flowline = get_flowline(
                &coverage,
                seed,
                discr,
                z_min,
                slope_min,
                turn_min_cos,
                true,
            );
            if flowline.len() > 1 {
                let cut_idx = intersection_idx(&flowline, &flowlines, new_dist_min);
                flowline.truncate(cut_idx);
                if flowline.len() > 1 {
                    flowlines.push(flowline);
                }
            }
        }

        let finished_level = idx + 1 == contours.len() || (contours[idx + 1].value - value).abs() > EPS;
        if finished_level {
            let n = flowlines.len();
            if n > 1 {
                for i in 0..(n - 1) {
                    if !starts.contains(&(i + 1)) {
                        insert_flowlines(
                            &coverage,
                            &mut flowlines,
                            i,
                            i + 1,
                            0,
                            0,
                            depth,
                            new_dist_min,
                            new_dist_max,
                            discr,
                            z_min,
                            slope_min,
                            turn_min_cos,
                            true,
                        );
                    }
                }
            }

            let mut flowlines_up = Vec::new();
            let mut up_idxs = Vec::new();
            for (seed_idx, seed) in level_seeds.iter().enumerate() {
                let mut flowline = get_flowline(
                    &coverage,
                    seed,
                    discr,
                    z_max,
                    slope_min,
                    turn_min_cos,
                    false,
                );
                if flowline.len() > 1 {
                    let idx1 = intersection_idx(&flowline, &flowlines_prev, target_step);
                    let idx2 = intersection_idx(&flowline, &flowlines_up, new_dist_min);
                    flowline.truncate(cmp::min(idx1, idx2));
                    if flowline.len() > 1 {
                        flowlines_up.push(flowline);
                        up_idxs.push(seed_idx);
                    }
                }
            }

            let n_up = flowlines_up.len();
            if n_up > 1 {
                for i in 0..(n_up - 1) {
                    if !seed_starts.contains(&up_idxs[i + 1]) && up_idxs[i + 1] - up_idxs[i] == 1 {
                        insert_flowlines(
                            &coverage,
                            &mut flowlines_up,
                            i,
                            i + 1,
                            0,
                            0,
                            depth,
                            new_dist_min,
                            new_dist_max,
                            discr,
                            z_max,
                            slope_min,
                            turn_min_cos,
                            false,
                        );
                    }
                }
            }

            level_seeds.clear();
            flowlines_prev = flowlines.clone();
            flowlines.append(&mut flowlines_up);

            let sqrt_half = 0.5_f64.sqrt();
            for flowline in &flowlines {
                let mut dx_sum = 0.0;
                let mut dy_sum = 0.0;
                let mut samples = 0usize;
                for point in flowline {
                    if let Some((gx, gy)) = coverage.gradient(point.x, point.y) {
                        dx_sum += gx;
                        dy_sum += gy;
                        samples += 1;
                    }
                }
                if samples == 0 {
                    continue;
                }

                let dx = -dx_sum / samples as f64;
                let dy = -dy_sum / samples as f64;
                let grad_len = (dx * dx + dy * dy).sqrt();
                let (slope, aspect, cos_n, cos_ne, cos_e, cos_se, cos_s, cos_sw, cos_w, cos_nw) =
                    if grad_len <= EPS {
                        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
                    } else {
                        let slope = grad_len.atan().to_degrees();
                        let math_aspect = dy.atan2(dx).to_degrees();
                        let aspect = if math_aspect < 90.0 {
                            90.0 - math_aspect
                        } else {
                            450.0 - math_aspect
                        };
                        let ux = dx / grad_len;
                        let uy = dy / grad_len;
                        (
                            slope,
                            aspect,
                            uy,
                            sqrt_half * ux + sqrt_half * uy,
                            ux,
                            sqrt_half * ux - sqrt_half * uy,
                            -uy,
                            -sqrt_half * ux - sqrt_half * uy,
                            -ux,
                            -sqrt_half * ux + sqrt_half * uy,
                        )
                    };

                output
                    .add_feature(
                        Some(Geometry::line_string(flowline.clone())),
                        &[
                            ("FID", FieldValue::Integer(hid)),
                            ("HEIGHT", FieldValue::Float(value)),
                            ("SLOPE", FieldValue::Float(slope)),
                            ("ASPECT", FieldValue::Float(aspect)),
                            ("N", FieldValue::Float(cos_n)),
                            ("NE", FieldValue::Float(cos_ne)),
                            ("E", FieldValue::Float(cos_e)),
                            ("SE", FieldValue::Float(cos_se)),
                            ("S", FieldValue::Float(cos_s)),
                            ("SW", FieldValue::Float(cos_sw)),
                            ("W", FieldValue::Float(cos_w)),
                            ("NW", FieldValue::Float(cos_nw)),
                        ],
                    )
                    .map_err(|e| ToolError::Execution(format!("failed creating hachure feature: {}", e)))?;
                hid += 1;
            }

            flowlines.clear();
            starts.clear();
            seed_starts.clear();
            seed_starts.insert(0usize);
        }

        let _ = contour.closed;
    }

    Ok(output)
}

/// Topographic hachure polyline generation tool.
///
/// Author: Timofey Samsonov
impl Tool for TopographicHachuresTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "topographic_hachures",
            display_name: "Topographic Hachures",
            summary: "Creates topographic hachure polylines from a raster surface.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Pro,
            params: vec![
                ToolParamSpec {
                    name: "dem",
                    description: "Input DEM raster path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "interval",
                    description: "Contour interval value (default 10.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "base",
                    description: "Base contour elevation (default 0.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "tolerance",
                    description: "Minimum contour bend angle in degrees retained during simplification (default 10.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "smooth",
                    description: "Contour smoothing filter size (odd integer preferred, default 9).",
                    required: false,
                },
                ToolParamSpec {
                    name: "separation",
                    description: "Nominal contour seed separation in average-cell units (default 2.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "distmin",
                    description: "Minimum spacing multiplier used to truncate nearby hachures (default 0.5).",
                    required: false,
                },
                ToolParamSpec {
                    name: "distmax",
                    description: "Maximum spacing multiplier used to insert additional hachures (default 2.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "discretization",
                    description: "Flowline step length in average-cell units (default 0.5).",
                    required: false,
                },
                ToolParamSpec {
                    name: "turnmax",
                    description: "Maximum permitted turn angle in degrees while tracing hachures (default 45.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "slopemin",
                    description: "Minimum slope angle in degrees for tracing continuation (default 0.5).",
                    required: false,
                },
                ToolParamSpec {
                    name: "depth",
                    description: "Recursive infill depth used in divergent areas (default 16).",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output vector path.",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("dem".to_string(), json!("dem.tif"));
        defaults.insert("interval".to_string(), json!(10.0));
        defaults.insert("base".to_string(), json!(0.0));
        defaults.insert("tolerance".to_string(), json!(10.0));
        defaults.insert("smooth".to_string(), json!(9));
        defaults.insert("separation".to_string(), json!(2.0));
        defaults.insert("distmin".to_string(), json!(0.5));
        defaults.insert("distmax".to_string(), json!(2.0));
        defaults.insert("discretization".to_string(), json!(0.5));
        defaults.insert("turnmax".to_string(), json!(45.0));
        defaults.insert("slopemin".to_string(), json!(0.5));
        defaults.insert("depth".to_string(), json!(16));
        defaults.insert("output".to_string(), json!("topographic_hachures.shp"));

        ToolManifest {
            id: "topographic_hachures".to_string(),
            display_name: "Topographic Hachures".to_string(),
            summary: "Creates topographic hachure polylines from a DEM using contour-seeded downslope and upslope flowlines. Legacy authorship attribution is intentionally preserved for this tool.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Pro,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec![
                "geomorphometry".to_string(),
                "hachures".to_string(),
                "contours".to_string(),
                "vector".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "dem")?;
        let _ = output_path_arg(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let dem_path = parse_raster_path_arg(args, "dem")?;
        let output = output_path_arg(args)?;
        let interval = args.get("interval").and_then(|v| v.as_f64()).unwrap_or(10.0);
        let base = args.get("base").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let tolerance = args.get("tolerance").and_then(|v| v.as_f64()).unwrap_or(10.0);
        let smooth = args
            .get("smooth")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(9);
        let separation = args.get("separation").and_then(|v| v.as_f64()).unwrap_or(2.0);
        let dist_min = args.get("distmin").and_then(|v| v.as_f64()).unwrap_or(0.5);
        let dist_max = args.get("distmax").and_then(|v| v.as_f64()).unwrap_or(2.0);
        let discretization = args
            .get("discretization")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);
        let turn_max = args.get("turnmax").and_then(|v| v.as_f64()).unwrap_or(45.0);
        let slope_min = args.get("slopemin").and_then(|v| v.as_f64()).unwrap_or(0.5);
        let depth = args.get("depth").and_then(|v| v.as_u64()).unwrap_or(16) as u8;

        if interval <= 0.0 {
            return Err(ToolError::Validation("parameter 'interval' must be > 0".to_string()));
        }
        if separation <= 0.0 {
            return Err(ToolError::Validation("parameter 'separation' must be > 0".to_string()));
        }
        if dist_min <= 0.0 || dist_max <= 0.0 {
            return Err(ToolError::Validation("parameters 'distmin' and 'distmax' must be > 0".to_string()));
        }
        if discretization <= 0.0 {
            return Err(ToolError::Validation("parameter 'discretization' must be > 0".to_string()));
        }
        if dist_max < dist_min {
            return Err(ToolError::Validation("parameter 'distmax' must be >= 'distmin'".to_string()));
        }

        let raster = load_raster(&dem_path)?;
        let layer = topographic_hachure_layer(
            &raster,
            interval,
            base,
            smooth,
            tolerance,
            separation,
            dist_min,
            dist_max,
            discretization,
            turn_max,
            slope_min,
            depth,
        )?;
        ensure_parent_dir(&output)?;
        Ok(build_result(write_vector(&layer, &output)?))
    }
}

impl Tool for ContoursFromRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "contours_from_raster",
            display_name: "Contours From Raster",
            summary: "Creates contour polylines from an input raster surface.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input raster surface path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "interval",
                    description: "Contour interval value (default 10.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "base",
                    description: "Base contour elevation (default 0.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "smooth",
                    description: "Smoothing filter size (odd integer, default 9).",
                    required: false,
                },
                ToolParamSpec {
                    name: "tolerance",
                    description: "Minimum line deflection angle in degrees for simplification (default 10.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output vector path.",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("interval".to_string(), json!(10.0));
        defaults.insert("base".to_string(), json!(0.0));
        defaults.insert("smooth".to_string(), json!(9));
        defaults.insert("tolerance".to_string(), json!(10.0));
        defaults.insert("output".to_string(), json!("contours.shp"));

        ToolManifest {
            id: "contours_from_raster".to_string(),
            display_name: "Contours From Raster".to_string(),
            summary: "Creates contour polylines from a raster surface model.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec![
                "geomorphometry".to_string(),
                "contours".to_string(),
                "vector".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let _ = output_path_arg(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let output = output_path_arg(args)?;
        let interval = args
            .get("interval")
            .and_then(|v| v.as_f64())
            .unwrap_or(10.0);
        let base = args.get("base").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let smooth = args
            .get("smooth")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(9);
        let tolerance = args
            .get("tolerance")
            .and_then(|v| v.as_f64())
            .unwrap_or(10.0)
            .clamp(0.0, 45.0);

        if interval <= 0.0 {
            return Err(ToolError::Validation("parameter 'interval' must be > 0".to_string()));
        }

        let raster = load_raster(&input_path)?;
        let layer = raster_contour_layer(&raster, interval, base, smooth, tolerance)?;
        ensure_parent_dir(&output)?;
        Ok(build_result(write_vector(&layer, &output)?))
    }
}

impl Tool for ContoursFromPointsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "contours_from_points",
            display_name: "Contours From Points",
            summary: "Creates contour polylines from point elevations using Delaunay triangulation.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input point or multipoint vector path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "field_name",
                    description: "Numeric attribute field containing elevation values (ignored when use_z_values=true).",
                    required: false,
                },
                ToolParamSpec {
                    name: "use_z_values",
                    description: "Use geometry Z values as elevations (default false).",
                    required: false,
                },
                ToolParamSpec {
                    name: "max_triangle_edge_length",
                    description: "Maximum triangle edge length to include in contouring (map units).",
                    required: false,
                },
                ToolParamSpec {
                    name: "interval",
                    description: "Contour interval value (default 10.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "base",
                    description: "Base contour elevation (default 0.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "smooth",
                    description: "Smoothing filter size (odd integer, default 9).",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output vector path.",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("points.shp"));
        defaults.insert("field_name".to_string(), json!("ELEV"));
        defaults.insert("use_z_values".to_string(), json!(false));
        defaults.insert("max_triangle_edge_length".to_string(), json!(f64::INFINITY));
        defaults.insert("interval".to_string(), json!(10.0));
        defaults.insert("base".to_string(), json!(0.0));
        defaults.insert("smooth".to_string(), json!(9));
        defaults.insert("output".to_string(), json!("contours_from_points.shp"));

        ToolManifest {
            id: "contours_from_points".to_string(),
            display_name: "Contours From Points".to_string(),
            summary: "Creates contour polylines from point elevations using a Delaunay TIN.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec![
                "geomorphometry".to_string(),
                "contours".to_string(),
                "vector".to_string(),
                "triangulation".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "input")?;
        let _ = output_path_arg(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_vector_path_arg(args, "input")?;
        let output = output_path_arg(args)?;
        let field_name = args.get("field_name").and_then(|v| v.as_str());
        let use_z_values = args
            .get("use_z_values")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let max_triangle_edge_length = args
            .get("max_triangle_edge_length")
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::INFINITY);
        let interval = args
            .get("interval")
            .and_then(|v| v.as_f64())
            .unwrap_or(10.0);
        let base = args.get("base").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let smooth = args
            .get("smooth")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(9);

        if interval <= 0.0 {
            return Err(ToolError::Validation("parameter 'interval' must be > 0".to_string()));
        }

        let input = wbvector::read(&input_path)
            .map_err(|e| ToolError::Execution(format!("failed reading input vector: {}", e)))?;
        let layer = points_contour_layer(
            &input,
            field_name,
            use_z_values,
            max_triangle_edge_length,
            interval,
            base,
            smooth,
        )?;

        ensure_parent_dir(&output)?;
        Ok(build_result(write_vector(&layer, &output)?))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use wbcore::{AllowAllCapabilities, ProgressSink, ToolContext};
    use wbraster::RasterConfig;
    use wbvector::{FieldDef, FieldType, FieldValue, Geometry, GeometryType, Layer, VectorFormat};

    struct NoopProgress;
    impl ProgressSink for NoopProgress {}

    fn make_ctx() -> ToolContext<'static> {
        static PROGRESS: NoopProgress = NoopProgress;
        static CAPS: AllowAllCapabilities = AllowAllCapabilities;
        ToolContext {
            progress: &PROGRESS,
            capabilities: &CAPS,
        }
    }

    fn unique_temp_shp_path(prefix: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{}_{}_{}.shp", prefix, std::process::id(), nanos))
    }

    #[test]
    fn contours_from_raster_produces_lines() {
        let cfg = RasterConfig {
            rows: 3,
            cols: 3,
            bands: 1,
            nodata: -9999.0,
            cell_size: 1.0,
            ..Default::default()
        };
        let mut dem = Raster::new(cfg);
        for r in 0..3isize {
            for c in 0..3isize {
                dem.set(0, r, c, (r + c) as f64).unwrap();
            }
        }
        let dem_id = memory_store::put_raster(dem);

        let out = std::env::temp_dir().join("contours_from_raster_test.shp");
        let mut args = ToolArgs::new();
        args.insert(
            "input".to_string(),
            json!(memory_store::make_raster_memory_path(&dem_id)),
        );
        args.insert("interval".to_string(), json!(1.0));
        args.insert("base".to_string(), json!(0.0));
        args.insert("output".to_string(), json!(out.to_string_lossy().to_string()));

        let result = ContoursFromRasterTool.run(&args, &make_ctx()).unwrap();
        let path = result.outputs.get("path").unwrap().as_str().unwrap();
        let layer = wbvector::read(path).unwrap();
        assert!(!layer.features.is_empty());
    }

    #[test]
    fn contours_from_points_produces_lines() {
        let mut points = Layer::new("points").with_geom_type(GeometryType::Point);
        points.add_field(FieldDef::new("elev", FieldType::Float));

        let samples = [
            (0.0, 0.0, 0.0),
            (2.0, 0.0, 2.0),
            (2.0, 2.0, 4.0),
            (0.0, 2.0, 2.0),
            (1.0, 1.0, 3.0),
        ];
        for (x, y, z) in samples {
            points
                .add_feature(
                    Some(Geometry::point(x, y)),
                    &[("elev", FieldValue::Float(z))],
                )
                .unwrap();
        }

        let input = unique_temp_shp_path("contours_from_points_input");
        wbvector::write(
            &points,
            input.to_string_lossy().as_ref(),
            VectorFormat::Shapefile,
        )
        .unwrap();

        let output = unique_temp_shp_path("contours_from_points_output");
        let mut args = ToolArgs::new();
        args.insert(
            "input".to_string(),
            json!(input.to_string_lossy().to_string()),
        );
        args.insert("field_name".to_string(), json!("elev"));
        args.insert("interval".to_string(), json!(1.0));
        args.insert("base".to_string(), json!(0.0));
        args.insert("output".to_string(), json!(output.to_string_lossy().to_string()));

        let result = ContoursFromPointsTool.run(&args, &make_ctx()).unwrap();
        let path = result.outputs.get("path").unwrap().as_str().unwrap();
        let layer = wbvector::read(path).unwrap();
        assert!(!layer.features.is_empty());

        let height_idx = layer.schema.field_index("HEIGHT").unwrap();
        let mut level_keys = BTreeSet::new();
        for feat in &layer.features {
            let z = feat.attributes[height_idx].as_f64().unwrap();
            let level = (z / 1.0).round();
            assert!((z - level).abs() < 1.0e-9, "height {z} is not on interval level");
            level_keys.insert(level as i64);
        }
        assert!(!level_keys.is_empty());
    }

    #[test]
    fn topographic_hachures_produces_lines_and_metrics() {
        let cfg = RasterConfig {
            rows: 7,
            cols: 7,
            bands: 1,
            nodata: -9999.0,
            cell_size: 1.0,
            ..Default::default()
        };
        let mut dem = Raster::new(cfg);
        for r in 0..7isize {
            for c in 0..7isize {
                let z = (6 - r) as f64 + ((c as f64 - 3.0).abs() * 0.3);
                dem.set(0, r, c, z).unwrap();
            }
        }
        let dem_id = memory_store::put_raster(dem);

        let output = unique_temp_shp_path("topographic_hachures_output");
        let mut args = ToolArgs::new();
        args.insert(
            "dem".to_string(),
            json!(memory_store::make_raster_memory_path(&dem_id)),
        );
        args.insert("interval".to_string(), json!(1.0));
        args.insert("base".to_string(), json!(0.0));
        args.insert("separation".to_string(), json!(1.5));
        args.insert("distmin".to_string(), json!(0.4));
        args.insert("distmax".to_string(), json!(1.8));
        args.insert("discretization".to_string(), json!(0.5));
        args.insert("output".to_string(), json!(output.to_string_lossy().to_string()));

        let result = TopographicHachuresTool.run(&args, &make_ctx()).unwrap();
        let path = result.outputs.get("path").unwrap().as_str().unwrap();
        let layer = wbvector::read(path).unwrap();
        assert!(!layer.features.is_empty());
        assert!(layer.schema.field_index("HEIGHT").is_some());
        assert!(layer.schema.field_index("SLOPE").is_some());
        assert!(layer.schema.field_index("ASPECT").is_some());
    }
}
