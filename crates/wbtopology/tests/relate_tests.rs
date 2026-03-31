use wbtopology::{
    relate, relate_with_epsilon, relate_with_precision, Coord, Geometry, LineString, LinearRing, Location,
    Polygon, PrecisionModel,
};

#[test]
fn relate_point_in_polygon_sets_interior_intersection() {
    let poly = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
        ]),
        vec![],
    ));
    let p = Geometry::Point(Coord::xy(5.0, 5.0));

    let m = relate(&p, &poly);
    assert_eq!(m.get(Location::Interior, Location::Interior), '0');
}

#[test]
fn relate_disjoint_line_and_polygon_sets_false_ii() {
    let line = Geometry::LineString(LineString::new(vec![
        Coord::xy(-5.0, -5.0),
        Coord::xy(-1.0, -1.0),
    ]));
    let poly = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
        ]),
        vec![],
    ));

    let m = relate(&line, &poly);
    assert_eq!(m.get(Location::Interior, Location::Interior), 'F');
}

#[test]
fn relate_touching_sets_false_ii() {
    let poly = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
        ]),
        vec![],
    ));
    let p = Geometry::Point(Coord::xy(0.0, 5.0));
    let m = relate(&p, &poly);
    assert_eq!(m.get(Location::Interior, Location::Interior), 'F');
}

#[test]
fn relate_with_precision_snaps_before_evaluation() {
    let a = Geometry::Point(Coord::xy(1.00041, 2.00041));
    let b = Geometry::Point(Coord::xy(1.00049, 2.00049));
    let m = relate_with_precision(&a, &b, PrecisionModel::Fixed { scale: 1000.0 });
    assert_eq!(m.get(Location::Interior, Location::Interior), '0');
}

#[test]
fn relate_with_epsilon_matches_precision_for_near_points() {
    let a = Geometry::Point(Coord::xy(1.00041, 2.00041));
    let b = Geometry::Point(Coord::xy(1.00049, 2.00049));

    let by_precision = relate_with_precision(&a, &b, PrecisionModel::Fixed { scale: 1000.0 });
    let by_epsilon = relate_with_epsilon(&a, &b, 0.0005);

    assert_eq!(by_precision.as_str9(), by_epsilon.as_str9());
}

#[test]
fn relate_matrix_pattern_matching_and_derived_predicates() {
    let disjoint_a = Geometry::Point(Coord::xy(-5.0, -5.0));
    let disjoint_b = Geometry::Point(Coord::xy(5.0, 5.0));
    let disjoint = relate(&disjoint_a, &disjoint_b);
    assert!(disjoint.matches("FF*FF****"));
    assert!(disjoint.is_disjoint());
    assert!(!disjoint.is_intersects());

    let poly = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
        ]),
        vec![],
    ));
    let inside = Geometry::Point(Coord::xy(5.0, 5.0));
    let contains_m = relate(&poly, &inside);
    assert!(contains_m.is_contains());
    assert!(!contains_m.is_within());

    let within_m = relate(&inside, &poly);
    assert!(within_m.is_within());
    assert!(!within_m.is_contains());

    let boundary = Geometry::Point(Coord::xy(0.0, 5.0));
    let touches_m = relate(&boundary, &poly);
    assert!(touches_m.is_touches());
}

#[test]
fn relate_linestring_polygon_crosses_sets_expected_core_cells() {
    let line = Geometry::LineString(LineString::new(vec![
        Coord::xy(-1.0, 5.0),
        Coord::xy(5.0, 5.0),
        Coord::xy(11.0, 5.0),
    ]));
    let poly = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
        ]),
        vec![],
    ));

    let m = relate(&line, &poly);
    assert_eq!(m.get(Location::Interior, Location::Interior), '1');
    assert_eq!(m.get(Location::Interior, Location::Boundary), '0');
    assert_eq!(m.get(Location::Boundary, Location::Interior), '0');
}

#[test]
fn relate_point_on_polygon_boundary_sets_boundary_contact_cell() {
    let poly = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
        ]),
        vec![],
    ));
    let p = Geometry::Point(Coord::xy(0.0, 5.0));

    let m = relate(&p, &poly);
    assert_eq!(m.get(Location::Interior, Location::Interior), 'F');
    assert_eq!(m.get(Location::Interior, Location::Boundary), '0');
}

#[test]
fn relate_polygon_contains_point_sets_interior_exterior_false() {
    let poly = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
        ]),
        vec![],
    ));
    let p = Geometry::Point(Coord::xy(5.0, 5.0));

    let m = relate(&poly, &p);
    assert_eq!(m.get(Location::Exterior, Location::Interior), 'F');
    assert!(m.is_contains());
}
