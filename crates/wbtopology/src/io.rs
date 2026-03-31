//! WKT/WKB interoperability for wbtopology geometries.

use crate::error::{Result, TopologyError};
use crate::geom::{Coord, Geometry, LineString, LinearRing, Polygon};

/// Encode a geometry as ISO WKB (little-endian).
pub fn to_wkb(g: &Geometry) -> Vec<u8> {
    to_wbvector_geometry(g).to_wkb()
}

/// Decode a geometry from ISO WKB.
pub fn from_wkb(bytes: &[u8]) -> Result<Geometry> {
    let g = wbvector::Geometry::from_wkb(bytes)
        .map_err(|e| TopologyError::Conversion(format!("wkb decode failed: {e}")))?;
    from_wbvector_geometry(&g)
}

/// Serialize a geometry to WKT.
pub fn to_wkt(g: &Geometry) -> String {
    to_wbvector_geometry(g).to_wkt()
}

/// Parse a geometry from WKT (2D and 3-ordinate forms, with or without an explicit `Z` tag).
pub fn from_wkt(text: &str) -> Result<Geometry> {
    let mut p = WktParser::new(text);
    let g = p.parse_geometry()?;
    p.skip_ws();
    if !p.is_eof() {
        return Err(TopologyError::Conversion("trailing tokens after WKT geometry".to_string()));
    }
    Ok(g)
}

fn to_wbvector_geometry(g: &Geometry) -> wbvector::Geometry {
    match g {
        Geometry::Point(c) => wbvector::Geometry::Point(to_wb_coord(*c)),
        Geometry::LineString(ls) => wbvector::Geometry::LineString(to_wb_coords(&ls.coords)),
        Geometry::Polygon(poly) => wbvector::Geometry::Polygon {
            exterior: wbvector::Ring::new(to_wb_coords(&poly.exterior.coords)),
            interiors: poly
                .holes
                .iter()
                .map(|h| wbvector::Ring::new(to_wb_coords(&h.coords)))
                .collect(),
        },
        Geometry::MultiPoint(pts) => {
            wbvector::Geometry::MultiPoint(pts.iter().copied().map(to_wb_coord).collect())
        }
        Geometry::MultiLineString(lines) => wbvector::Geometry::MultiLineString(
            lines.iter().map(|ls| to_wb_coords(&ls.coords)).collect(),
        ),
        Geometry::MultiPolygon(polys) => wbvector::Geometry::MultiPolygon(
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
        ),
        Geometry::GeometryCollection(parts) => {
            wbvector::Geometry::GeometryCollection(parts.iter().map(to_wbvector_geometry).collect())
        }
    }
}

fn from_wbvector_geometry(g: &wbvector::Geometry) -> Result<Geometry> {
    Ok(match g {
        wbvector::Geometry::Point(c) => Geometry::Point(from_wb_coord(c)),
        wbvector::Geometry::LineString(cs) => Geometry::LineString(LineString::new(from_wb_coords(cs))),
        wbvector::Geometry::Polygon { exterior, interiors } => Geometry::Polygon(Polygon::new(
            LinearRing::new(from_wb_coords(exterior.coords())),
            interiors
                .iter()
                .map(|r| LinearRing::new(from_wb_coords(r.coords())))
                .collect(),
        )),
        wbvector::Geometry::MultiPoint(cs) => {
            Geometry::MultiPoint(cs.iter().map(from_wb_coord).collect())
        }
        wbvector::Geometry::MultiLineString(lines) => Geometry::MultiLineString(
            lines.iter().map(|l| LineString::new(from_wb_coords(l))).collect(),
        ),
        wbvector::Geometry::MultiPolygon(polys) => Geometry::MultiPolygon(
            polys
                .iter()
                .map(|(ext, holes)| {
                    Polygon::new(
                        LinearRing::new(from_wb_coords(ext.coords())),
                        holes
                            .iter()
                            .map(|h| LinearRing::new(from_wb_coords(h.coords())))
                            .collect(),
                    )
                })
                .collect(),
        ),
        wbvector::Geometry::GeometryCollection(parts) => Geometry::GeometryCollection(
            parts
                .iter()
                .map(from_wbvector_geometry)
                .collect::<Result<Vec<_>>>()?,
        ),
    })
}

fn to_wb_coords(cs: &[Coord]) -> Vec<wbvector::Coord> {
    cs.iter().copied().map(to_wb_coord).collect()
}

fn from_wb_coords(cs: &[wbvector::Coord]) -> Vec<Coord> {
    cs.iter().map(from_wb_coord).collect()
}

fn to_wb_coord(c: Coord) -> wbvector::Coord {
    match c.z {
        Some(z) => wbvector::Coord::xyz(c.x, c.y, z),
        None => wbvector::Coord::xy(c.x, c.y),
    }
}

fn from_wb_coord(c: &wbvector::Coord) -> Coord {
    match c.z {
        Some(z) => Coord::xyz(c.x, c.y, z),
        None => Coord::xy(c.x, c.y),
    }
}

struct WktParser<'a> {
    s: &'a [u8],
    i: usize,
    has_z: bool,
}

impl<'a> WktParser<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            s: s.as_bytes(),
            i: 0,
            has_z: false,
        }
    }

    fn is_eof(&self) -> bool {
        self.i >= self.s.len()
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_ascii_whitespace() {
                self.i += 1;
            } else {
                break;
            }
        }
    }

    fn consume_char(&mut self, c: u8) -> bool {
        self.skip_ws();
        if self.peek() == Some(c) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn expect_char(&mut self, c: u8) -> Result<()> {
        if self.consume_char(c) {
            Ok(())
        } else {
            Err(TopologyError::Conversion(format!("expected '{}'", c as char)))
        }
    }

    fn parse_ident(&mut self) -> Result<String> {
        self.skip_ws();
        let start = self.i;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphabetic() {
                self.i += 1;
            } else {
                break;
            }
        }
        if self.i == start {
            return Err(TopologyError::Conversion("expected geometry type".to_string()));
        }
        Ok(String::from_utf8(self.s[start..self.i].to_vec()).unwrap().to_ascii_uppercase())
    }

    fn parse_number(&mut self) -> Result<f64> {
        self.skip_ws();
        let start = self.i;

        if matches!(self.peek(), Some(b'+') | Some(b'-')) {
            self.i += 1;
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.i += 1;
            } else {
                break;
            }
        }
        if self.peek() == Some(b'.') {
            self.i += 1;
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.i += 1;
                } else {
                    break;
                }
            }
        }
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            self.i += 1;
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.i += 1;
            }
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.i += 1;
                } else {
                    break;
                }
            }
        }

        if self.i == start {
            return Err(TopologyError::Conversion("expected number".to_string()));
        }

        let txt = std::str::from_utf8(&self.s[start..self.i]).unwrap();
        txt.parse::<f64>()
            .map_err(|_| TopologyError::Conversion(format!("invalid number '{txt}'")))
    }

    fn parse_coord(&mut self) -> Result<Coord> {
        let x = self.parse_number()?;
        let y = self.parse_number()?;
        if self.has_z || self.peek_coord_has_extra_ordinate() {
            Ok(Coord::xyz(x, y, self.parse_number()?))
        } else {
            Ok(Coord::xy(x, y))
        }
    }

    fn parse_coord_list(&mut self) -> Result<Vec<Coord>> {
        self.expect_char(b'(')?;
        let mut out = Vec::new();
        loop {
            out.push(self.parse_coord()?);
            if self.consume_char(b',') {
                continue;
            }
            break;
        }
        self.expect_char(b')')?;
        Ok(out)
    }

    fn parse_point_body(&mut self) -> Result<Geometry> {
        if self.parse_empty()? {
            return Err(TopologyError::Conversion("POINT EMPTY is unsupported in wbtopology".to_string()));
        }
        self.expect_char(b'(')?;
        let c = self.parse_coord()?;
        self.expect_char(b')')?;
        Ok(Geometry::Point(c))
    }

    fn parse_linestring_body(&mut self) -> Result<Geometry> {
        if self.parse_empty()? {
            return Ok(Geometry::LineString(LineString::new(vec![])));
        }
        Ok(Geometry::LineString(LineString::new(self.parse_coord_list()?)))
    }

    fn parse_polygon_body(&mut self) -> Result<Geometry> {
        if self.parse_empty()? {
            return Ok(Geometry::Polygon(Polygon::new(LinearRing::new(vec![]), vec![])));
        }
        self.expect_char(b'(')?;
        let mut rings = Vec::new();
        loop {
            rings.push(LinearRing::new(self.parse_coord_list()?));
            if self.consume_char(b',') {
                continue;
            }
            break;
        }
        self.expect_char(b')')?;

        if rings.is_empty() {
            return Err(TopologyError::Conversion("POLYGON requires at least one ring".to_string()));
        }

        let exterior = rings.remove(0);
        Ok(Geometry::Polygon(Polygon::new(exterior, rings)))
    }

    fn parse_multipoint_body(&mut self) -> Result<Geometry> {
        if self.parse_empty()? {
            return Ok(Geometry::MultiPoint(vec![]));
        }

        self.expect_char(b'(')?;
        let mut pts = Vec::new();

        self.skip_ws();
        let nested = self.peek() == Some(b'(');

        if nested {
            loop {
                self.expect_char(b'(')?;
                pts.push(self.parse_coord()?);
                self.expect_char(b')')?;
                if self.consume_char(b',') {
                    continue;
                }
                break;
            }
        } else {
            loop {
                pts.push(self.parse_coord()?);
                if self.consume_char(b',') {
                    continue;
                }
                break;
            }
        }

        self.expect_char(b')')?;
        Ok(Geometry::MultiPoint(pts))
    }

    fn parse_multilinestring_body(&mut self) -> Result<Geometry> {
        if self.parse_empty()? {
            return Ok(Geometry::MultiLineString(vec![]));
        }
        self.expect_char(b'(')?;
        let mut lines = Vec::new();
        loop {
            lines.push(LineString::new(self.parse_coord_list()?));
            if self.consume_char(b',') {
                continue;
            }
            break;
        }
        self.expect_char(b')')?;
        Ok(Geometry::MultiLineString(lines))
    }

    fn parse_multipolygon_body(&mut self) -> Result<Geometry> {
        if self.parse_empty()? {
            return Ok(Geometry::MultiPolygon(vec![]));
        }
        self.expect_char(b'(')?;
        let mut polys = Vec::new();
        loop {
            self.expect_char(b'(')?;
            let mut rings = Vec::new();
            loop {
                rings.push(LinearRing::new(self.parse_coord_list()?));
                if self.consume_char(b',') {
                    continue;
                }
                break;
            }
            self.expect_char(b')')?;

            if rings.is_empty() {
                return Err(TopologyError::Conversion("MULTIPOLYGON member missing exterior ring".to_string()));
            }
            let exterior = rings.remove(0);
            polys.push(Polygon::new(exterior, rings));

            if self.consume_char(b',') {
                continue;
            }
            break;
        }
        self.expect_char(b')')?;
        Ok(Geometry::MultiPolygon(polys))
    }

    fn parse_geometrycollection_body(&mut self) -> Result<Geometry> {
        if self.parse_empty()? {
            return Ok(Geometry::GeometryCollection(vec![]));
        }
        self.expect_char(b'(')?;
        let mut geoms = Vec::new();
        loop {
            geoms.push(self.parse_geometry()?);
            if self.consume_char(b',') {
                continue;
            }
            break;
        }
        self.expect_char(b')')?;
        Ok(Geometry::GeometryCollection(geoms))
    }

    fn parse_empty(&mut self) -> Result<bool> {
        self.skip_ws();
        let save = self.i;
        let ident = self.parse_ident();
        match ident {
            Ok(s) if s == "EMPTY" => Ok(true),
            Ok(_) | Err(_) => {
                self.i = save;
                Ok(false)
            }
        }
    }

    fn parse_geometry(&mut self) -> Result<Geometry> {
        let t = self.parse_ident()?;
        let prev_has_z = self.has_z;
        self.has_z = self.try_parse_z_tag();
        let result = match t.as_str() {
            "POINT" => self.parse_point_body(),
            "LINESTRING" => self.parse_linestring_body(),
            "POLYGON" => self.parse_polygon_body(),
            "MULTIPOINT" => self.parse_multipoint_body(),
            "MULTILINESTRING" => self.parse_multilinestring_body(),
            "MULTIPOLYGON" => self.parse_multipolygon_body(),
            "GEOMETRYCOLLECTION" => self.parse_geometrycollection_body(),
            other => Err(TopologyError::Conversion(format!("unsupported WKT geometry type '{other}'"))),
        };
        self.has_z = prev_has_z;
        result
    }

    fn try_parse_z_tag(&mut self) -> bool {
        self.skip_ws();
        let save = self.i;
        match self.parse_ident() {
            Ok(tag) if tag == "Z" => true,
            _ => {
                self.i = save;
                false
            }
        }
    }

    fn peek_coord_has_extra_ordinate(&mut self) -> bool {
        self.skip_ws();
        matches!(self.peek(), Some(b'+') | Some(b'-') | Some(b'.') | Some(b'0'..=b'9'))
    }
}
