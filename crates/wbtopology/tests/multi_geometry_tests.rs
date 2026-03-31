use wbtopology::{
    contains,
    contains_with_epsilon,
    covered_by,
    covers,
    disjoint,
    geometry_area,
    geometry_centroid,
    geometry_length,
    intersects,
    Coord,
    Geometry,
    LineString,
    LinearRing,
    Polygon,
};

#[test]
fn intersects_handles_multi_point() {
    let line = Geometry::LineString(LineString::new(vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(10.0, 0.0),
    ]));
    let mpt = Geometry::MultiPoint(vec![Coord::xy(3.0, 0.0), Coord::xy(20.0, 5.0)]);

    assert!(intersects(&line, &mpt));
}

#[test]
fn contains_dispatches_across_multi_components() {
    let square = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
        ]),
        vec![],
    ));

    let inside_points = Geometry::MultiPoint(vec![Coord::xy(1.0, 1.0), Coord::xy(9.0, 9.0)]);
    let mixed_points = Geometry::MultiPoint(vec![Coord::xy(1.0, 1.0), Coord::xy(20.0, 20.0)]);

    assert!(contains(&square, &inside_points));
    assert!(!contains(&square, &mixed_points));

    assert!(contains_with_epsilon(&square, &inside_points, 1.0e-12));
}

#[test]
fn covers_covered_by_and_disjoint_work() {
    let square = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
        ]),
        vec![],
    ));

    let boundary_point = Geometry::Point(Coord::xy(0.0, 5.0));
    let outside_point = Geometry::Point(Coord::xy(20.0, 20.0));

    assert!(covers(&square, &boundary_point));
    assert!(covered_by(&boundary_point, &square));
    assert!(disjoint(&square, &outside_point));
}

#[test]
fn measurements_handle_multi_and_collection() {
    let mp = Geometry::MultiPoint(vec![Coord::xy(0.0, 0.0), Coord::xy(2.0, 2.0)]);
    let ml = Geometry::MultiLineString(vec![
        LineString::new(vec![Coord::xy(0.0, 0.0), Coord::xy(3.0, 4.0)]),
        LineString::new(vec![Coord::xy(0.0, 0.0), Coord::xy(0.0, 5.0)]),
    ]);
    let mpoly = Geometry::MultiPolygon(vec![Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(2.0, 0.0),
            Coord::xy(2.0, 2.0),
            Coord::xy(0.0, 2.0),
        ]),
        vec![],
    )]);

    let gc = Geometry::GeometryCollection(vec![mp.clone(), ml.clone(), mpoly.clone()]);

    assert_eq!(geometry_area(&mp), 0.0);
    assert_eq!(geometry_area(&mpoly), 4.0);
    assert_eq!(geometry_length(&ml), 10.0);

    let c = geometry_centroid(&gc).expect("centroid should exist");
    assert!(c.x.is_finite() && c.y.is_finite());
}

#[test]
fn linestring_centroid_interpolates_z() {
    let ls = Geometry::LineString(LineString::new(vec![
        Coord::xyz(0.0, 0.0, 0.0),
        Coord::xyz(10.0, 0.0, 10.0),
    ]));

    let c = geometry_centroid(&ls).expect("centroid should exist");
    assert_eq!(c.x, 5.0);
    assert_eq!(c.y, 0.0);
    assert_eq!(c.z, Some(5.0));
}
