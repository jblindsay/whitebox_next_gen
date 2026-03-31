//! Interoperability with wbvector for vector read/write workflows.

use crate::error::{Result, TopologyError};
use crate::geom::{Coord, Geometry, LineString, LinearRing, Polygon};

/// Read geometries from any wbvector-supported path.
pub fn read_geometries(path: &str) -> Result<Vec<Geometry>> {
    let layer = wbvector::read(path).map_err(|e| TopologyError::Io(e.to_string()))?;
    geometries_from_layer(&layer)
}

/// Write geometries to a path using wbvector format detection.
pub fn write_geometries(path: &str, geometries: &[Geometry]) -> Result<()> {
    let format = wbvector::VectorFormat::detect(path).map_err(|e| TopologyError::Io(e.to_string()))?;
    let layer = layer_from_geometries("wbtopology", geometries, None)?;
    wbvector::write(&layer, path, format).map_err(|e| TopologyError::Io(e.to_string()))
}

/// Convert wbvector Layer to wbtopology geometries.
pub fn geometries_from_layer(layer: &wbvector::Layer) -> Result<Vec<Geometry>> {
    let mut out = Vec::with_capacity(layer.features.len());
    for feat in &layer.features {
        if let Some(g) = &feat.geometry {
            flatten_wbvector_geometry(g, &mut out)?;
        }
    }
    Ok(out)
}

/// Build a wbvector Layer from wbtopology geometries.
pub fn layer_from_geometries(name: &str, geometries: &[Geometry], epsg: Option<u32>) -> Result<wbvector::Layer> {
    let mut layer = wbvector::Layer::new(name);
    if let Some(code) = epsg {
        layer = layer.with_epsg(code);
    }

    for g in geometries {
        let wb_geom = to_wbvector_geometry(g)?;
        layer.push(wbvector::Feature {
            fid: 0,
            geometry: Some(wb_geom),
            attributes: Vec::new(),
        });
    }

    Ok(layer)
}

fn flatten_wbvector_geometry(geom: &wbvector::Geometry, out: &mut Vec<Geometry>) -> Result<()> {
    match geom {
        wbvector::Geometry::Point(c) => out.push(Geometry::Point(from_wb_coord(c))),
        wbvector::Geometry::LineString(cs) => out.push(Geometry::LineString(LineString::new(from_wb_coords(cs)))),
        wbvector::Geometry::Polygon { exterior, interiors } => {
            out.push(Geometry::Polygon(Polygon::new(
                LinearRing::new(from_wb_coords(exterior.coords())),
                interiors
                    .iter()
                    .map(|r| LinearRing::new(from_wb_coords(r.coords())))
                    .collect(),
            )));
        }
        wbvector::Geometry::MultiPoint(cs) => {
            for c in cs {
                out.push(Geometry::Point(from_wb_coord(c)));
            }
        }
        wbvector::Geometry::MultiLineString(lines) => {
            for l in lines {
                out.push(Geometry::LineString(LineString::new(from_wb_coords(l))));
            }
        }
        wbvector::Geometry::MultiPolygon(polys) => {
            for (exterior, interiors) in polys {
                out.push(Geometry::Polygon(Polygon::new(
                    LinearRing::new(from_wb_coords(exterior.coords())),
                    interiors
                        .iter()
                        .map(|r| LinearRing::new(from_wb_coords(r.coords())))
                        .collect(),
                )));
            }
        }
        wbvector::Geometry::GeometryCollection(gs) => {
            for g in gs {
                flatten_wbvector_geometry(g, out)?;
            }
        }
    }
    Ok(())
}

fn to_wbvector_geometry(g: &Geometry) -> Result<wbvector::Geometry> {
    match g {
        Geometry::Point(c) => Ok(wbvector::Geometry::Point(to_wb_coord(*c))),
        Geometry::LineString(ls) => Ok(wbvector::Geometry::LineString(to_wb_coords(&ls.coords))),
        Geometry::Polygon(poly) => Ok(wbvector::Geometry::Polygon {
            exterior: wbvector::Ring::new(to_wb_coords(&poly.exterior.coords)),
            interiors: poly
                .holes
                .iter()
                .map(|h| wbvector::Ring::new(to_wb_coords(&h.coords)))
                .collect(),
        }),
        Geometry::MultiPoint(pts) => Ok(wbvector::Geometry::MultiPoint(
            pts.iter().copied().map(to_wb_coord).collect(),
        )),
        Geometry::MultiLineString(lines) => Ok(wbvector::Geometry::MultiLineString(
            lines.iter().map(|ls| to_wb_coords(&ls.coords)).collect(),
        )),
        Geometry::MultiPolygon(polys) => Ok(wbvector::Geometry::MultiPolygon(
            polys
                .iter()
                .map(|poly| {
                    (
                        wbvector::Ring::new(to_wb_coords(&poly.exterior.coords)),
                        poly.holes
                            .iter()
                            .map(|h| wbvector::Ring::new(to_wb_coords(&h.coords)))
                            .collect(),
                    )
                })
                .collect(),
        )),
        Geometry::GeometryCollection(parts) => {
            let mut out = Vec::with_capacity(parts.len());
            for p in parts {
                out.push(to_wbvector_geometry(p)?);
            }
            Ok(wbvector::Geometry::GeometryCollection(out))
        }
    }
}

#[inline]
fn from_wb_coord(c: &wbvector::Coord) -> Coord {
    match c.z {
        Some(z) => Coord::xyz(c.x, c.y, z),
        None => Coord::xy(c.x, c.y),
    }
}

fn from_wb_coords(cs: &[wbvector::Coord]) -> Vec<Coord> {
    cs.iter().map(from_wb_coord).collect()
}

#[inline]
fn to_wb_coord(c: Coord) -> wbvector::Coord {
    match c.z {
        Some(z) => wbvector::Coord::xyz(c.x, c.y, z),
        None => wbvector::Coord::xy(c.x, c.y),
    }
}

fn to_wb_coords(cs: &[Coord]) -> Vec<wbvector::Coord> {
    cs.iter().copied().map(to_wb_coord).collect()
}
