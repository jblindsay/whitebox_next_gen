//! Core geometry types.

/// A 2D/3D coordinate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coord {
    /// X coordinate.
    pub x: f64,
    /// Y coordinate.
    pub y: f64,
    /// Optional Z coordinate.
    pub z: Option<f64>,
}

impl Coord {
    /// Create a 2D coordinate.
    #[inline]
    pub const fn xy(x: f64, y: f64) -> Self {
        Self { x, y, z: None }
    }

    /// Create a 3D coordinate.
    #[inline]
    pub const fn xyz(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z: Some(z) }
    }

    /// Returns true when a Z value is present.
    #[inline]
    pub const fn has_z(&self) -> bool {
        self.z.is_some()
    }

    /// Return a copy with a new optional Z value.
    #[inline]
    pub const fn with_z(self, z: Option<f64>) -> Self {
        Self { x: self.x, y: self.y, z }
    }

    /// XY-only equality.
    #[inline]
    pub fn xy_eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }

    /// Interpolate along segment `a -> b` at parameter `t` in `[0, 1]`.
    ///
    /// XY is always linearly interpolated. Z policy:
    /// - if both endpoints have Z, interpolate Z linearly;
    /// - if `t <= 0`, keep `a.z`;
    /// - if `t >= 1`, keep `b.z`;
    /// - otherwise use `None` when one or both endpoint Z values are missing.
    #[inline]
    pub fn interpolate_segment(a: Coord, b: Coord, t: f64) -> Self {
        let tt = t.clamp(0.0, 1.0);
        let z = match (a.z, b.z) {
            (Some(za), Some(zb)) => Some(za + tt * (zb - za)),
            _ if tt <= 0.0 => a.z,
            _ if tt >= 1.0 => b.z,
            _ => None,
        };
        Self {
            x: a.x + tt * (b.x - a.x),
            y: a.y + tt * (b.y - a.y),
            z,
        }
    }
}

/// Axis-aligned envelope (bounding box).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Envelope {
    /// Minimum x.
    pub min_x: f64,
    /// Minimum y.
    pub min_y: f64,
    /// Maximum x.
    pub max_x: f64,
    /// Maximum y.
    pub max_y: f64,
}

impl Envelope {
    /// Create an envelope from bounds.
    #[inline]
    pub const fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    /// Check if the envelope contains a point.
    #[inline]
    pub fn contains_coord(&self, p: Coord) -> bool {
        p.x >= self.min_x && p.x <= self.max_x && p.y >= self.min_y && p.y <= self.max_y
    }

    /// Check if two envelopes intersect.
    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        self.min_x <= other.max_x
            && self.max_x >= other.min_x
            && self.min_y <= other.max_y
            && self.max_y >= other.min_y
    }
}

/// A connected sequence of vertices.
#[derive(Debug, Clone, PartialEq)]
pub struct LineString {
    /// Coordinates in order.
    pub coords: Vec<Coord>,
}

impl LineString {
    /// Construct a linestring from coordinates.
    #[inline]
    pub fn new(coords: Vec<Coord>) -> Self {
        Self { coords }
    }

    /// Number of coordinates.
    #[inline]
    pub fn len(&self) -> usize {
        self.coords.len()
    }

    /// True if no coordinates are present.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.coords.is_empty()
    }

    /// Envelope of this linestring.
    pub fn envelope(&self) -> Option<Envelope> {
        envelope_of_coords(&self.coords)
    }
}

/// A closed linestring used as a ring.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearRing {
    /// Coordinates in order. First and last should be equal.
    pub coords: Vec<Coord>,
}

impl LinearRing {
    /// Construct a ring and close it if needed.
    pub fn new(mut coords: Vec<Coord>) -> Self {
        if let (Some(first), Some(last)) = (coords.first().copied(), coords.last().copied()) {
            if !first.xy_eq(&last) {
                coords.push(first);
            }
        }
        Self { coords }
    }

    /// Number of coordinates.
    #[inline]
    pub fn len(&self) -> usize {
        self.coords.len()
    }

    /// Envelope of this ring.
    pub fn envelope(&self) -> Option<Envelope> {
        envelope_of_coords(&self.coords)
    }
}

/// A polygon with exterior ring and optional holes.
#[derive(Debug, Clone, PartialEq)]
pub struct Polygon {
    /// Exterior ring.
    pub exterior: LinearRing,
    /// Interior holes.
    pub holes: Vec<LinearRing>,
}

impl Polygon {
    /// Create a polygon.
    #[inline]
    pub fn new(exterior: LinearRing, holes: Vec<LinearRing>) -> Self {
        Self { exterior, holes }
    }

    /// Envelope of this polygon.
    pub fn envelope(&self) -> Option<Envelope> {
        self.exterior.envelope()
    }
}

/// Geometry enum for basic topology predicates.
#[derive(Debug, Clone, PartialEq)]
pub enum Geometry {
    /// Point geometry.
    Point(Coord),
    /// LineString geometry.
    LineString(LineString),
    /// Polygon geometry.
    Polygon(Polygon),
    /// Collection of points.
    MultiPoint(Vec<Coord>),
    /// Collection of linestrings.
    MultiLineString(Vec<LineString>),
    /// Collection of polygons.
    MultiPolygon(Vec<Polygon>),
    /// Heterogeneous collection of geometries.
    GeometryCollection(Vec<Geometry>),
}

impl Geometry {
    /// Envelope of this geometry, or `None` for empty geometries.
    pub fn envelope(&self) -> Option<Envelope> {
        match self {
            Geometry::Point(c) => Some(Envelope::new(c.x, c.y, c.x, c.y)),
            Geometry::LineString(ls) => ls.envelope(),
            Geometry::Polygon(poly) => poly.envelope(),
            Geometry::MultiPoint(pts) => {
                let first = pts.first()?;
                let mut e = Envelope::new(first.x, first.y, first.x, first.y);
                for p in &pts[1..] { e = env_expand(e, p.x, p.y); }
                Some(e)
            }
            Geometry::MultiLineString(lss) => lss.iter()
                .filter_map(|ls| ls.envelope())
                .fold(None, |acc, e| Some(acc.map_or(e, |a| env_merge(a, e)))),
            Geometry::MultiPolygon(polys) => polys.iter()
                .filter_map(|p| p.envelope())
                .fold(None, |acc, e| Some(acc.map_or(e, |a| env_merge(a, e)))),
            Geometry::GeometryCollection(geoms) => geoms.iter()
                .filter_map(|g| g.envelope())
                .fold(None, |acc, e| Some(acc.map_or(e, |a| env_merge(a, e)))),
        }
    }

    /// True if this geometry contains no coordinates.
    pub fn is_empty(&self) -> bool {
        match self {
            Geometry::Point(_) => false,
            Geometry::LineString(ls) => ls.is_empty(),
            Geometry::Polygon(poly) => poly.exterior.coords.is_empty(),
            Geometry::MultiPoint(pts) => pts.is_empty(),
            Geometry::MultiLineString(lss) => lss.is_empty(),
            Geometry::MultiPolygon(polys) => polys.is_empty(),
            Geometry::GeometryCollection(geoms) => geoms.is_empty(),
        }
    }

    /// Flatten this geometry to its primitive `Point`/`LineString`/`Polygon` components.
    pub fn components(&self) -> Vec<Geometry> {
        match self {
            Geometry::MultiPoint(pts) => pts.iter().map(|&p| Geometry::Point(p)).collect(),
            Geometry::MultiLineString(lss) => {
                lss.iter().map(|ls| Geometry::LineString(ls.clone())).collect()
            }
            Geometry::MultiPolygon(polys) => {
                polys.iter().map(|p| Geometry::Polygon(p.clone())).collect()
            }
            Geometry::GeometryCollection(geoms) => {
                geoms.iter().flat_map(|g| g.components()).collect()
            }
            other => vec![other.clone()],
        }
    }
}

#[inline]
fn env_expand(e: Envelope, x: f64, y: f64) -> Envelope {
    Envelope::new(e.min_x.min(x), e.min_y.min(y), e.max_x.max(x), e.max_y.max(y))
}

#[inline]
fn env_merge(a: Envelope, b: Envelope) -> Envelope {
    Envelope::new(a.min_x.min(b.min_x), a.min_y.min(b.min_y), a.max_x.max(b.max_x), a.max_y.max(b.max_y))
}

fn envelope_of_coords(coords: &[Coord]) -> Option<Envelope> {
    let first = *coords.first()?;
    let mut min_x = first.x;
    let mut min_y = first.y;
    let mut max_x = first.x;
    let mut max_y = first.y;

    for &c in &coords[1..] {
        if c.x < min_x {
            min_x = c.x;
        }
        if c.x > max_x {
            max_x = c.x;
        }
        if c.y < min_y {
            min_y = c.y;
        }
        if c.y > max_y {
            max_y = c.y;
        }
    }

    Some(Envelope::new(min_x, min_y, max_x, max_y))
}
