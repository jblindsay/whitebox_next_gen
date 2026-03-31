use wbtopology::{
    from_wkb,
    from_wkt,
    to_wkb,
    to_wkt,
    Coord,
    Geometry,
    LineString,
    LinearRing,
    Polygon,
};

#[test]
fn wkb_roundtrip_polygon() {
    let g = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(5.0, 0.0),
            Coord::xy(5.0, 5.0),
            Coord::xy(0.0, 5.0),
        ]),
        vec![LinearRing::new(vec![
            Coord::xy(1.0, 1.0),
            Coord::xy(2.0, 1.0),
            Coord::xy(2.0, 2.0),
            Coord::xy(1.0, 2.0),
        ])],
    ));

    let bytes = to_wkb(&g);
    let parsed = from_wkb(&bytes).expect("WKB decode should succeed");
    assert_eq!(parsed, g);
}

#[test]
fn wkt_roundtrip_multiline() {
    let g = Geometry::MultiLineString(vec![
        LineString::new(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 1.0)]),
        LineString::new(vec![Coord::xy(2.0, 2.0), Coord::xy(3.0, 3.0)]),
    ]);

    let wkt = to_wkt(&g);
    let parsed = from_wkt(&wkt).expect("WKT parse should succeed");
    assert_eq!(parsed, g);
}

#[test]
fn parse_geometrycollection_wkt() {
    let wkt = "GEOMETRYCOLLECTION(POINT(1 2),LINESTRING(0 0,1 1),POLYGON((0 0,3 0,3 3,0 3,0 0)))";
    let g = from_wkt(wkt).expect("WKT parse should succeed");
    match g {
        Geometry::GeometryCollection(parts) => assert_eq!(parts.len(), 3),
        _ => panic!("expected geometry collection"),
    }
}

#[test]
fn wkt_roundtrip_point_z() {
    let g = Geometry::Point(Coord::xyz(1.0, 2.0, 3.0));
    let wkt = to_wkt(&g);
    let parsed = from_wkt(&wkt).expect("WKT Z parse should succeed");
    assert_eq!(parsed, g);
}

#[test]
fn wkb_roundtrip_linestring_z() {
    let g = Geometry::LineString(LineString::new(vec![
        Coord::xyz(0.0, 0.0, 10.0),
        Coord::xyz(1.0, 1.0, 11.0),
    ]));
    let bytes = to_wkb(&g);
    let parsed = from_wkb(&bytes).expect("WKB Z decode should succeed");
    assert_eq!(parsed, g);
}
