use wbtopology::{
    geometry_distance,
    is_within_distance,
    nearest_points,
    Coord,
    Geometry,
    LineString,
    LinearRing,
    Polygon,
};

#[test]
fn point_point_distance() {
    let a = Geometry::Point(Coord::xy(0.0, 0.0));
    let b = Geometry::Point(Coord::xy(3.0, 4.0));
    assert_eq!(geometry_distance(&a, &b), 5.0);
}

#[test]
fn point_linestring_distance() {
    let p = Geometry::Point(Coord::xy(3.0, 2.0));
    let ls = Geometry::LineString(LineString::new(vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(10.0, 0.0),
    ]));
    assert_eq!(geometry_distance(&p, &ls), 2.0);
}

#[test]
fn point_polygon_distance_zero_when_inside() {
    let p = Geometry::Point(Coord::xy(2.0, 2.0));
    let poly = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(5.0, 0.0),
            Coord::xy(5.0, 5.0),
            Coord::xy(0.0, 5.0),
        ]),
        vec![],
    ));
    assert_eq!(geometry_distance(&p, &poly), 0.0);
}

#[test]
fn nearest_points_returns_reasonable_pair() {
    let a = Geometry::Point(Coord::xy(0.0, 0.0));
    let b = Geometry::LineString(LineString::new(vec![
        Coord::xy(5.0, 0.0),
        Coord::xy(5.0, 5.0),
    ]));

    let (pa, pb) = nearest_points(&a, &b);
    assert_eq!(pa, Coord::xy(0.0, 0.0));
    assert_eq!(pb, Coord::xy(5.0, 0.0));
}

#[test]
fn within_distance_handles_multi_geometries() {
    let mpt = Geometry::MultiPoint(vec![Coord::xy(0.0, 0.0), Coord::xy(100.0, 100.0)]);
    let p = Geometry::Point(Coord::xy(0.0, 2.0));

    assert!(is_within_distance(&mpt, &p, 2.0));
    assert!(!is_within_distance(&mpt, &p, 1.5));
}

#[test]
fn nearest_points_interpolates_z_on_segment_projection() {
    let a = Geometry::Point(Coord::xy(5.0, 2.0));
    let b = Geometry::LineString(LineString::new(vec![
        Coord::xyz(0.0, 0.0, 0.0),
        Coord::xyz(10.0, 0.0, 10.0),
    ]));

    let (_pa, pb) = nearest_points(&a, &b);
    assert_eq!(pb.x, 5.0);
    assert_eq!(pb.y, 0.0);
    assert_eq!(pb.z, Some(5.0));
}
