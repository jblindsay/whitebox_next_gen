//! Canonical geometry types shared by all format drivers.
//!
//! Every driver converts its native representation to/from these types, so
//! format conversion is simply: read → [`Geometry`] → write.
//!
//! # WKB type codes (ISO)
//! 1=Point 2=LineString 3=Polygon 4=MultiPoint 5=MultiLineString
//! 6=MultiPolygon 7=GeometryCollection
//! Add 1000 for Z, 2000 for M, 3000 for ZM.
//! EWKB uses OR-flags: 0x80000000=Z, 0x40000000=M.

use std::io::{Cursor, Read};
use crate::error::{GeoError, Result};

// ── Coordinate ────────────────────────────────────────────────────────────────

/// A 2-, 3-, or 4-D coordinate.
#[derive(Debug, Clone, PartialEq)]
pub struct Coord {
    /// X coordinate (or longitude for geographic CRS).
    pub x: f64,
    /// Y coordinate (or latitude for geographic CRS).
    pub y: f64,
    /// Optional Z coordinate (elevation/depth).
    pub z: Option<f64>,
    /// Optional measure value.
    pub m: Option<f64>,
}

impl Coord {
    /// Creates a 2D coordinate.
    pub fn xy(x: f64, y: f64) -> Self { Self { x, y, z: None, m: None } }
    /// Creates a 3D coordinate with Z.
    pub fn xyz(x: f64, y: f64, z: f64) -> Self { Self { x, y, z: Some(z), m: None } }
    /// Returns `true` when the coordinate has a Z value.
    pub fn has_z(&self) -> bool { self.z.is_some() }
    /// Returns `true` when the coordinate has an M value.
    pub fn has_m(&self) -> bool { self.m.is_some() }
}

// ── Ring ─────────────────────────────────────────────────────────────────────

/// A closed linear ring (polygon boundary).
/// Stored without the closing duplicate point.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ring(pub Vec<Coord>);

impl Ring {
    /// Creates a ring from coordinate sequence.
    pub fn new(coords: Vec<Coord>) -> Self { Self(coords) }
    /// Returns ring coordinates.
    pub fn coords(&self) -> &[Coord] { &self.0 }
    /// Returns number of vertices.
    pub fn len(&self) -> usize { self.0.len() }
    /// Returns `true` when ring has no vertices.
    pub fn is_empty(&self) -> bool { self.0.is_empty() }

    /// Signed area (positive = CCW, negative = CW).
    pub fn signed_area(&self) -> f64 {
        let n = self.0.len();
        if n < 3 { return 0.0; }
        let mut a = 0.0f64;
        for i in 0..n {
            let j = (i + 1) % n;
            a += self.0[i].x * self.0[j].y - self.0[j].x * self.0[i].y;
        }
        a * 0.5
    }
}

// ── Geometry ──────────────────────────────────────────────────────────────────

/// The canonical geometry type covering all OGC Simple Features.
#[derive(Debug, Clone, PartialEq)]
pub enum Geometry {
    /// Single point geometry.
    Point(Coord),
    /// Connected path of vertices.
    LineString(Vec<Coord>),
    /// Polygon with one exterior ring and zero or more interior rings.
    Polygon {
        /// Exterior boundary ring.
        exterior: Ring,
        /// Interior hole rings.
        interiors: Vec<Ring>,
    },
    /// Set of points.
    MultiPoint(Vec<Coord>),
    /// Set of line strings.
    MultiLineString(Vec<Vec<Coord>>),
    /// Set of polygons.
    MultiPolygon(Vec<(Ring, Vec<Ring>)>),
    /// Heterogeneous set of geometries.
    GeometryCollection(Vec<Geometry>),
}

impl Geometry {
    // ── Convenience constructors ──────────────────────────────────────────────

    /// Constructs a 2D point geometry.
    pub fn point(x: f64, y: f64) -> Self { Self::Point(Coord::xy(x, y)) }
    /// Constructs a 3D point geometry.
    pub fn point_z(x: f64, y: f64, z: f64) -> Self { Self::Point(Coord::xyz(x, y, z)) }

    /// Constructs a line string geometry.
    pub fn line_string(coords: Vec<Coord>) -> Self { Self::LineString(coords) }

    /// Constructs a polygon geometry from exterior and interior coordinate rings.
    pub fn polygon(exterior: Vec<Coord>, interiors: Vec<Vec<Coord>>) -> Self {
        Self::Polygon {
            exterior: Ring::new(exterior),
            interiors: interiors.into_iter().map(Ring::new).collect(),
        }
    }

    /// Constructs a multipoint geometry.
    pub fn multi_point(pts: Vec<Coord>) -> Self { Self::MultiPoint(pts) }

    /// Constructs a multilinestring geometry.
    pub fn multi_line_string(lines: Vec<Vec<Coord>>) -> Self { Self::MultiLineString(lines) }

    /// Constructs a multipolygon geometry.
    pub fn multi_polygon(polys: Vec<(Vec<Coord>, Vec<Vec<Coord>>)>) -> Self {
        Self::MultiPolygon(
            polys.into_iter()
                .map(|(e, hs)| (Ring::new(e), hs.into_iter().map(Ring::new).collect()))
                .collect(),
        )
    }

    // ── Geometry type ─────────────────────────────────────────────────────────

    /// Returns the geometry type discriminator.
    pub fn geom_type(&self) -> GeometryType {
        match self {
            Self::Point(_)              => GeometryType::Point,
            Self::LineString(_)         => GeometryType::LineString,
            Self::Polygon { .. }        => GeometryType::Polygon,
            Self::MultiPoint(_)         => GeometryType::MultiPoint,
            Self::MultiLineString(_)    => GeometryType::MultiLineString,
            Self::MultiPolygon(_)       => GeometryType::MultiPolygon,
            Self::GeometryCollection(_) => GeometryType::GeometryCollection,
        }
    }

    /// Returns `true` if any coordinate in the geometry has a Z value.
    pub fn has_z(&self) -> bool {
        self.all_coords().iter().any(|c| c.has_z())
    }

    /// Returns `true` if geometry has no coordinates/components.
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Point(_)              => false,
            Self::LineString(v)         => v.is_empty(),
            Self::Polygon { exterior, .. } => exterior.is_empty(),
            Self::MultiPoint(v)         => v.is_empty(),
            Self::MultiLineString(v)    => v.is_empty(),
            Self::MultiPolygon(v)       => v.is_empty(),
            Self::GeometryCollection(v) => v.is_empty(),
        }
    }

    // ── Coordinate access ─────────────────────────────────────────────────────

    /// Returns flattened coordinate references across all nested components.
    pub fn all_coords(&self) -> Vec<&Coord> {
        match self {
            Self::Point(c) => vec![c],
            Self::LineString(cs) => cs.iter().collect(),
            Self::Polygon { exterior, interiors } => exterior.0.iter()
                .chain(interiors.iter().flat_map(|r| r.0.iter())).collect(),
            Self::MultiPoint(cs) => cs.iter().collect(),
            Self::MultiLineString(ls) => ls.iter().flat_map(|l| l.iter()).collect(),
            Self::MultiPolygon(ps) => ps.iter()
                .flat_map(|(e, hs)| e.0.iter().chain(hs.iter().flat_map(|r| r.0.iter())))
                .collect(),
            Self::GeometryCollection(gs) => gs.iter().flat_map(|g| g.all_coords()).collect(),
        }
    }

    // ── Bounding box ──────────────────────────────────────────────────────────

    /// Computes an axis-aligned bounding box for the geometry.
    pub fn bbox(&self) -> Option<BBox> {
        let cs = self.all_coords();
        if cs.is_empty() { return None; }
        let mut b = BBox {
            min_x: f64::INFINITY, min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY, max_y: f64::NEG_INFINITY,
        };
        for c in cs {
            b.min_x = b.min_x.min(c.x); b.max_x = b.max_x.max(c.x);
            b.min_y = b.min_y.min(c.y); b.max_y = b.max_y.max(c.y);
        }
        Some(b)
    }

    // ── WKT (write-only, for diagnostics) ─────────────────────────────────────

    /// Serializes geometry to WKT text (diagnostic output).
    pub fn to_wkt(&self) -> String {
        let mut s = String::new();
        wkt_write(&mut s, self);
        s
    }

    // ── ISO WKB encode / decode ───────────────────────────────────────────────

    /// Encode to ISO WKB (little-endian).
    pub fn to_wkb(&self) -> Vec<u8> {
        let mut out = Vec::new();
        wkb_write(&mut out, self);
        out
    }

    /// Decode from ISO WKB.
    pub fn from_wkb(data: &[u8]) -> Result<Self> {
        wkb_read(&mut Cursor::new(data))
    }

    // ── GeoPackage WKB (GP header + ISO WKB) ─────────────────────────────────

    /// Encode to GeoPackage WKB (`GP` header + ISO WKB + optional XY envelope).
    pub fn to_gpkg_wkb(&self, srs_id: i32) -> Vec<u8> {
        let bb = self.bbox();
        let env_flag: u8 = if bb.is_some() { 1 } else { 0 };
        let mut out = Vec::new();
        out.extend_from_slice(b"GP");
        out.push(0x00);                         // version
        out.push((env_flag << 1) | 0x01);       // flags: env=XY, LE byte order
        out.extend_from_slice(&srs_id.to_le_bytes());
        if let Some(b) = bb {
            out.extend_from_slice(&b.min_x.to_le_bytes());
            out.extend_from_slice(&b.max_x.to_le_bytes());
            out.extend_from_slice(&b.min_y.to_le_bytes());
            out.extend_from_slice(&b.max_y.to_le_bytes());
        }
        wkb_write(&mut out, self);
        out
    }

    /// Decode from GeoPackage WKB.  Returns `(geometry, srs_id)`.
    pub fn from_gpkg_wkb(data: &[u8]) -> Result<(Self, i32)> {
        if data.len() < 8 || &data[0..2] != b"GP" {
            return Err(GeoError::InvalidWkb { offset: 0, msg: "missing GP magic".into() });
        }
        let flags   = data[3];
        let srs_id  = i32::from_le_bytes(data[4..8].try_into().unwrap());
        let env_type = (flags >> 1) & 0x07; // 0=none 1=XY 2=XYZ 3=XYM 4=XYZM
        let env_bytes: usize = match env_type { 0 => 0, 1 => 32, 2 | 3 => 48, _ => 64 };
        let wkb_start = 8 + env_bytes;
        let geom = Self::from_wkb(&data[wkb_start..])?;
        Ok((geom, srs_id))
    }
}

// ── GeometryType ──────────────────────────────────────────────────────────────

/// Enumerates OGC Simple Features geometry classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GeometryType {
    /// Point geometry type.
    Point,
    /// LineString geometry type.
    LineString,
    /// Polygon geometry type.
    Polygon,
    /// MultiPoint geometry type.
    MultiPoint,
    /// MultiLineString geometry type.
    MultiLineString,
    /// MultiPolygon geometry type.
    MultiPolygon,
    /// GeometryCollection geometry type.
    GeometryCollection,
}

impl GeometryType {
    /// Returns canonical geometry type name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Point              => "Point",
            Self::LineString         => "LineString",
            Self::Polygon            => "Polygon",
            Self::MultiPoint         => "MultiPoint",
            Self::MultiLineString    => "MultiLineString",
            Self::MultiPolygon       => "MultiPolygon",
            Self::GeometryCollection => "GeometryCollection",
        }
    }

    /// Returns ISO WKB base type code.
    pub fn wkb_type(self) -> u32 {
        match self {
            Self::Point              => 1,
            Self::LineString         => 2,
            Self::Polygon            => 3,
            Self::MultiPoint         => 4,
            Self::MultiLineString    => 5,
            Self::MultiPolygon       => 6,
            Self::GeometryCollection => 7,
        }
    }
}

impl std::fmt::Display for GeometryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── BBox ─────────────────────────────────────────────────────────────────────

/// Axis-aligned bounding box.
#[derive(Debug, Clone, PartialEq)]
pub struct BBox {
    /// Minimum x coordinate.
    pub min_x: f64,
    /// Minimum y coordinate.
    pub min_y: f64,
    /// Maximum x coordinate.
    pub max_x: f64,
    /// Maximum y coordinate.
    pub max_y: f64,
}

impl BBox {
    /// Creates a new bounding box from min/max coordinates.
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self { min_x, min_y, max_x, max_y }
    }
    /// Returns bounding box width.
    pub fn width(&self)  -> f64 { self.max_x - self.min_x }
    /// Returns bounding box height.
    pub fn height(&self) -> f64 { self.max_y - self.min_y }
    /// Returns center point as `(x, y)`.
    pub fn center(&self) -> (f64, f64) {
        ((self.min_x + self.max_x) * 0.5, (self.min_y + self.max_y) * 0.5)
    }
    /// Expands this bbox to include another bbox.
    pub fn expand_to(&mut self, other: &BBox) {
        self.min_x = self.min_x.min(other.min_x);
        self.min_y = self.min_y.min(other.min_y);
        self.max_x = self.max_x.max(other.max_x);
        self.max_y = self.max_y.max(other.max_y);
    }
    /// Returns `true` if point `(x, y)` lies inside or on the bbox boundary.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }
    /// Returns `true` if this bbox intersects another bbox.
    pub fn intersects(&self, o: &BBox) -> bool {
        self.min_x <= o.max_x && self.max_x >= o.min_x &&
        self.min_y <= o.max_y && self.max_y >= o.min_y
    }
}

// ── WKB write ─────────────────────────────────────────────────────────────────

fn push_f64(out: &mut Vec<u8>, v: f64) { out.extend_from_slice(&v.to_le_bytes()); }
fn push_u32(out: &mut Vec<u8>, v: u32) { out.extend_from_slice(&v.to_le_bytes()); }

fn wkb_write_coord(out: &mut Vec<u8>, c: &Coord, z: bool) {
    push_f64(out, c.x); push_f64(out, c.y);
    if z { push_f64(out, c.z.unwrap_or(0.0)); }
}

fn wkb_write_coords(out: &mut Vec<u8>, cs: &[Coord], z: bool) {
    push_u32(out, cs.len() as u32);
    for c in cs { wkb_write_coord(out, c, z); }
}

fn wkb_write_ring(out: &mut Vec<u8>, ring: &Ring, z: bool) {
    // WKB rings include the closing point
    let n   = ring.0.len();
    let closes = n > 0 && ring.0.first() != ring.0.last();
    let count  = n + if closes { 1 } else { 0 };
    push_u32(out, count as u32);
    for c in &ring.0 { wkb_write_coord(out, c, z); }
    if closes && n > 0 { wkb_write_coord(out, &ring.0[0], z); }
}

pub(crate) fn wkb_write(out: &mut Vec<u8>, geom: &Geometry) {
    let z   = geom.has_z();
    let base = geom.geom_type().wkb_type();
    let wt   = if z { base + 1000 } else { base };
    out.push(1); // little-endian
    push_u32(out, wt);
    match geom {
        Geometry::Point(c) => wkb_write_coord(out, c, z),
        Geometry::LineString(cs) => wkb_write_coords(out, cs, z),
        Geometry::Polygon { exterior, interiors } => {
            push_u32(out, 1 + interiors.len() as u32);
            wkb_write_ring(out, exterior, z);
            for r in interiors { wkb_write_ring(out, r, z); }
        }
        Geometry::MultiPoint(cs) => {
            push_u32(out, cs.len() as u32);
            for c in cs { wkb_write(out, &Geometry::Point(c.clone())); }
        }
        Geometry::MultiLineString(ls) => {
            push_u32(out, ls.len() as u32);
            for l in ls { wkb_write(out, &Geometry::LineString(l.clone())); }
        }
        Geometry::MultiPolygon(ps) => {
            push_u32(out, ps.len() as u32);
            for (e, hs) in ps {
                wkb_write(out, &Geometry::Polygon { exterior: e.clone(), interiors: hs.clone() });
            }
        }
        Geometry::GeometryCollection(gs) => {
            push_u32(out, gs.len() as u32);
            for g in gs { wkb_write(out, g); }
        }
    }
}

// ── WKB read ──────────────────────────────────────────────────────────────────

fn read_exact<R: Read>(r: &mut R, buf: &mut [u8]) -> Result<()> {
    r.read_exact(buf).map_err(GeoError::Io)
}

fn read_f64(r: &mut Cursor<&[u8]>, le: bool) -> Result<f64> {
    let mut b = [0u8; 8]; read_exact(r, &mut b)?;
    Ok(if le { f64::from_le_bytes(b) } else { f64::from_be_bytes(b) })
}

fn read_u32(r: &mut Cursor<&[u8]>, le: bool) -> Result<u32> {
    let mut b = [0u8; 4]; read_exact(r, &mut b)?;
    Ok(if le { u32::from_le_bytes(b) } else { u32::from_be_bytes(b) })
}

fn read_coord(r: &mut Cursor<&[u8]>, le: bool, z: bool, m: bool) -> Result<Coord> {
    let x = read_f64(r, le)?; let y = read_f64(r, le)?;
    let zv = if z { Some(read_f64(r, le)?) } else { None };
    let mv = if m { Some(read_f64(r, le)?) } else { None };
    Ok(Coord { x, y, z: zv, m: mv })
}

fn read_coords(r: &mut Cursor<&[u8]>, le: bool, z: bool, m: bool) -> Result<Vec<Coord>> {
    let n = read_u32(r, le)? as usize;
    (0..n).map(|_| read_coord(r, le, z, m)).collect()
}

fn read_ring(r: &mut Cursor<&[u8]>, le: bool, z: bool, m: bool) -> Result<Ring> {
    let mut cs = read_coords(r, le, z, m)?;
    // drop closing point
    if cs.len() > 1 && cs.first() == cs.last() { cs.pop(); }
    Ok(Ring::new(cs))
}

fn wkb_read(r: &mut Cursor<&[u8]>) -> Result<Geometry> {
    let mut bo = [0u8]; read_exact(r, &mut bo)?;
    let le = bo[0] == 1;
    let raw_type = read_u32(r, le)?;

    // Decode ISO + EWKB type variants
    let (base, z, m) = if raw_type & 0xFFFF_0000 != 0 {
        // EWKB-style flags
        let z = raw_type & 0x8000_0000 != 0;
        let m = raw_type & 0x4000_0000 != 0;
        (raw_type & 0x0000_FFFF, z, m)
    } else if raw_type > 3000 { (raw_type - 3000, true, true)
    } else if raw_type > 2000 { (raw_type - 2000, false, true)
    } else if raw_type > 1000 { (raw_type - 1000, true, false)
    } else { (raw_type, false, false) };

    match base {
        1 => Ok(Geometry::Point(read_coord(r, le, z, m)?)),
        2 => Ok(Geometry::LineString(read_coords(r, le, z, m)?)),
        3 => {
            let n = read_u32(r, le)? as usize;
            if n == 0 { return Ok(Geometry::Polygon { exterior: Ring::default(), interiors: vec![] }); }
            let exterior  = read_ring(r, le, z, m)?;
            let interiors = (1..n).map(|_| read_ring(r, le, z, m)).collect::<Result<_>>()?;
            Ok(Geometry::Polygon { exterior, interiors })
        }
        4 => {
            let n = read_u32(r, le)? as usize;
            let pts: Result<Vec<Coord>> = (0..n).map(|_| {
                let g = wkb_read(r)?;
                if let Geometry::Point(c) = g { Ok(c) }
                else { Err(GeoError::InvalidWkb { offset: 0, msg: "MultiPoint child not Point".into() }) }
            }).collect();
            Ok(Geometry::MultiPoint(pts?))
        }
        5 => {
            let n = read_u32(r, le)? as usize;
            let lines: Result<Vec<Vec<Coord>>> = (0..n).map(|_| {
                let g = wkb_read(r)?;
                if let Geometry::LineString(cs) = g { Ok(cs) }
                else { Err(GeoError::InvalidWkb { offset: 0, msg: "MultiLineString child not LineString".into() }) }
            }).collect();
            Ok(Geometry::MultiLineString(lines?))
        }
        6 => {
            let n = read_u32(r, le)? as usize;
            let polys: Result<Vec<(Ring, Vec<Ring>)>> = (0..n).map(|_| {
                let g = wkb_read(r)?;
                if let Geometry::Polygon { exterior, interiors } = g { Ok((exterior, interiors)) }
                else { Err(GeoError::InvalidWkb { offset: 0, msg: "MultiPolygon child not Polygon".into() }) }
            }).collect();
            Ok(Geometry::MultiPolygon(polys?))
        }
        7 => {
            let n = read_u32(r, le)? as usize;
            let gs: Result<Vec<Geometry>> = (0..n).map(|_| wkb_read(r)).collect();
            Ok(Geometry::GeometryCollection(gs?))
        }
        other => Err(GeoError::UnsupportedWkbType(other)),
    }
}

// ── WKT write (diagnostics only) ─────────────────────────────────────────────

fn wkt_coord(s: &mut String, c: &Coord) {
    s.push_str(&format!("{} {}", c.x, c.y));
    if let Some(z) = c.z { s.push_str(&format!(" {}", z)); }
}

fn wkt_coords(s: &mut String, cs: &[Coord]) {
    for (i, c) in cs.iter().enumerate() { if i > 0 { s.push(','); } wkt_coord(s, c); }
}

fn wkt_ring(s: &mut String, ring: &Ring) {
    s.push('('); wkt_coords(s, &ring.0);
    if let Some(f) = ring.0.first() { s.push(','); wkt_coord(s, f); }
    s.push(')');
}

fn wkt_write(s: &mut String, g: &Geometry) {
    match g {
        Geometry::Point(c) => { s.push_str("POINT("); wkt_coord(s, c); s.push(')'); }
        Geometry::LineString(cs) => { s.push_str("LINESTRING("); wkt_coords(s, cs); s.push(')'); }
        Geometry::Polygon { exterior, interiors } => {
            s.push_str("POLYGON("); wkt_ring(s, exterior);
            for r in interiors { s.push(','); wkt_ring(s, r); }
            s.push(')');
        }
        Geometry::MultiPoint(cs) => {
            s.push_str("MULTIPOINT(");
            for (i, c) in cs.iter().enumerate() { if i > 0 { s.push(','); } s.push('('); wkt_coord(s, c); s.push(')'); }
            s.push(')');
        }
        Geometry::MultiLineString(ls) => {
            s.push_str("MULTILINESTRING(");
            for (i, l) in ls.iter().enumerate() { if i > 0 { s.push(','); } s.push('('); wkt_coords(s, l); s.push(')'); }
            s.push(')');
        }
        Geometry::MultiPolygon(ps) => {
            s.push_str("MULTIPOLYGON(");
            for (i, (e, hs)) in ps.iter().enumerate() {
                if i > 0 { s.push(','); } s.push('('); wkt_ring(s, e);
                for h in hs { s.push(','); wkt_ring(s, h); }
                s.push(')');
            }
            s.push(')');
        }
        Geometry::GeometryCollection(gs) => {
            s.push_str("GEOMETRYCOLLECTION(");
            for (i, g) in gs.iter().enumerate() { if i > 0 { s.push(','); } wkt_write(s, g); }
            s.push(')');
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_wkb_roundtrip(g: Geometry) {
        assert_eq!(Geometry::from_wkb(&g.to_wkb()).unwrap(), g);
    }

    #[test] fn wkb_point()      { assert_wkb_roundtrip(Geometry::point(1.5, 2.5)); }
    #[test] fn wkb_point_z()    { assert_wkb_roundtrip(Geometry::point_z(1.0, 2.0, 3.0)); }
    #[test] fn wkb_linestring() { assert_wkb_roundtrip(Geometry::line_string(vec![Coord::xy(0.,0.), Coord::xy(1.,1.)])); }
    #[test] fn wkb_polygon()    {
        assert_wkb_roundtrip(Geometry::polygon(
            vec![Coord::xy(0.,0.), Coord::xy(1.,0.), Coord::xy(1.,1.), Coord::xy(0.,1.)],
            vec![],
        ));
    }
    #[test] fn wkb_multipolygon() {
        assert_wkb_roundtrip(Geometry::multi_polygon(vec![
            (vec![Coord::xy(0.,0.), Coord::xy(1.,0.), Coord::xy(0.5,1.)], vec![]),
            (vec![Coord::xy(2.,0.), Coord::xy(3.,0.), Coord::xy(2.5,1.)], vec![]),
        ]));
    }
    #[test] fn gpkg_roundtrip() {
        let g = Geometry::polygon(vec![Coord::xy(-1.,-1.), Coord::xy(1.,-1.), Coord::xy(1.,1.), Coord::xy(-1.,1.)], vec![]);
        let (g2, srs) = Geometry::from_gpkg_wkb(&g.to_gpkg_wkb(4326)).unwrap();
        assert_eq!(g, g2);
        assert_eq!(srs, 4326);
    }
    #[test] fn bbox_polygon() {
        let g = Geometry::polygon(vec![Coord::xy(1.,2.), Coord::xy(3.,2.), Coord::xy(3.,4.), Coord::xy(1.,4.)], vec![]);
        let b = g.bbox().unwrap();
        assert_eq!((b.min_x, b.max_y), (1.0, 4.0));
    }
    #[test] fn wkt_point() { assert_eq!(Geometry::point(10.0, 20.0).to_wkt(), "POINT(10 20)"); }
}
