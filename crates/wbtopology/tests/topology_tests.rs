use wbtopology::{
    contains, crosses, intersects, is_simple_linestring, is_valid_polygon, overlaps,
    touches, within, contains_with_precision, intersects_with_precision, touches_with_precision,
    contains_with_epsilon, intersects_with_epsilon,
    crosses_with_epsilon, overlaps_with_epsilon, touches_with_epsilon, within_with_epsilon,
    PrecisionModel, Coord, Geometry, LineString, LinearRing, Polygon, PreparedPolygon,
};

#[test]
fn point_polygon_predicates_work() {
    let poly = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
        ]),
        vec![],
    ));

    let p_in = Geometry::Point(Coord::xy(5.0, 5.0));
    let p_out = Geometry::Point(Coord::xy(20.0, 5.0));

    assert!(contains(&poly, &p_in));
    assert!(intersects(&poly, &p_in));
    assert!(!contains(&poly, &p_out));
    assert!(!intersects(&poly, &p_out));
}

#[test]
fn line_simplicity_detects_self_intersection() {
    let simple = LineString::new(vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(2.0, 0.0),
        Coord::xy(4.0, 1.0),
    ]);
    let bow = LineString::new(vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(4.0, 4.0),
        Coord::xy(0.0, 4.0),
        Coord::xy(4.0, 0.0),
    ]);

    assert!(is_simple_linestring(&simple));
    assert!(!is_simple_linestring(&bow));
}

#[test]
fn polygon_validity_checks_hole_and_self_intersection() {
    let valid = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
        ]),
        vec![LinearRing::new(vec![
            Coord::xy(2.0, 2.0),
            Coord::xy(4.0, 2.0),
            Coord::xy(4.0, 4.0),
            Coord::xy(2.0, 4.0),
        ])],
    );

    let invalid = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
            Coord::xy(10.0, 0.0),
        ]),
        vec![],
    );

    assert!(is_valid_polygon(&valid));
    assert!(!is_valid_polygon(&invalid));
}

#[test]
fn polygon_polygon_intersection_works() {
    let a = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(5.0, 0.0),
            Coord::xy(5.0, 5.0),
            Coord::xy(0.0, 5.0),
        ]),
        vec![],
    ));
    let b = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(4.0, 4.0),
            Coord::xy(8.0, 4.0),
            Coord::xy(8.0, 8.0),
            Coord::xy(4.0, 8.0),
        ]),
        vec![],
    ));

    assert!(intersects(&a, &b));
}

#[test]
fn relation_predicates_behave_as_expected() {
    let poly = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
        ]),
        vec![],
    ));
    let p_boundary = Geometry::Point(Coord::xy(0.0, 5.0));
    let p_inside = Geometry::Point(Coord::xy(3.0, 3.0));

    assert!(touches(&poly, &p_boundary));
    assert!(within(&p_inside, &poly));

    let l1 = Geometry::LineString(LineString::new(vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(5.0, 5.0),
    ]));
    let l2 = Geometry::LineString(LineString::new(vec![
        Coord::xy(0.0, 5.0),
        Coord::xy(5.0, 0.0),
    ]));
    assert!(crosses(&l1, &l2));

    let pa = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(6.0, 0.0),
            Coord::xy(6.0, 6.0),
            Coord::xy(0.0, 6.0),
        ]),
        vec![],
    ));
    let pb = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(3.0, 3.0),
            Coord::xy(9.0, 3.0),
            Coord::xy(9.0, 9.0),
            Coord::xy(3.0, 9.0),
        ]),
        vec![],
    ));
    assert!(overlaps(&pa, &pb));
}

#[test]
fn prepared_polygon_point_queries_match_contains() {
    let poly = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
        ]),
        vec![LinearRing::new(vec![
            Coord::xy(2.0, 2.0),
            Coord::xy(4.0, 2.0),
            Coord::xy(4.0, 4.0),
            Coord::xy(2.0, 4.0),
        ])],
    );

    let prepared = PreparedPolygon::new(poly.clone());
    let gpoly = Geometry::Polygon(poly);

    let samples = [
        Coord::xy(1.0, 1.0),
        Coord::xy(3.0, 3.0),
        Coord::xy(0.0, 5.0),
        Coord::xy(20.0, 5.0),
    ];

    for p in samples {
        let expected = contains(&gpoly, &Geometry::Point(p));
        assert_eq!(prepared.contains_coord(p), expected);
    }
}

#[test]
fn precision_aware_predicates_snap_nearby_coordinates() {
    let pm = PrecisionModel::Fixed { scale: 10.0 }; // 0.1 grid

    let p1 = Geometry::Point(Coord::xy(1.041, 2.041));
    let p2 = Geometry::Point(Coord::xy(1.049, 2.049));
    assert!(intersects_with_precision(&p1, &p2, pm));

    let poly = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(1.0, 0.0),
            Coord::xy(1.0, 1.0),
            Coord::xy(0.0, 1.0),
        ]),
        vec![],
    ));
    let near_boundary = Geometry::Point(Coord::xy(1.00001, 0.5));

    assert!(contains_with_precision(&poly, &near_boundary, PrecisionModel::Fixed { scale: 1000.0 }));
    assert!(touches_with_precision(&poly, &near_boundary, PrecisionModel::Fixed { scale: 1000.0 }));
}

#[test]
fn epsilon_aware_predicates_handle_near_coincident_inputs() {
    let p1 = Geometry::Point(Coord::xy(1.0, 2.0));
    let p2 = Geometry::Point(Coord::xy(1.0 + 5.0e-5, 2.0 - 5.0e-5));
    assert!(intersects_with_epsilon(&p1, &p2, 1.0e-4));
    assert!(!intersects_with_epsilon(&p1, &p2, 1.0e-6));

    let poly = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(1.0, 0.0),
            Coord::xy(1.0, 1.0),
            Coord::xy(0.0, 1.0),
        ]),
        vec![],
    ));
    let p_near = Geometry::Point(Coord::xy(1.0 + 5.0e-5, 0.5));
    assert!(contains_with_epsilon(&poly, &p_near, 1.0e-4));
    assert!(!contains_with_epsilon(&poly, &p_near, 1.0e-7));
}

#[test]
fn epsilon_aware_relation_predicates_cover_all_main_relations() {
    let poly = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
        ]),
        vec![],
    ));
    let p = Geometry::Point(Coord::xy(10.0 + 5.0e-5, 5.0));
    assert!(touches_with_epsilon(&poly, &p, 1.0e-4));

    let inner = Geometry::Point(Coord::xy(5.0, 5.0));
    assert!(within_with_epsilon(&inner, &poly, 1.0e-8));

    let l1 = Geometry::LineString(LineString::new(vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(5.0, 5.0),
    ]));
    let l2 = Geometry::LineString(LineString::new(vec![
        Coord::xy(0.0, 5.0),
        Coord::xy(5.0, 0.0),
    ]));
    assert!(crosses_with_epsilon(&l1, &l2, 1.0e-8));

    let pa = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(6.0, 0.0),
            Coord::xy(6.0, 6.0),
            Coord::xy(0.0, 6.0),
        ]),
        vec![],
    ));
    let pb = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(3.0, 3.0),
            Coord::xy(9.0, 3.0),
            Coord::xy(9.0, 9.0),
            Coord::xy(3.0, 9.0),
        ]),
        vec![],
    ));
    assert!(overlaps_with_epsilon(&pa, &pb, 1.0e-8));
}

#[test]
fn large_coordinate_line_crossing_predicates_are_stable() {
    let base = 1.0e12;
    let span = 1.0e6;
    let a = Geometry::LineString(LineString::new(vec![
        Coord::xy(base, base),
        Coord::xy(base + span, base + span),
    ]));
    let b = Geometry::LineString(LineString::new(vec![
        Coord::xy(base, base + span),
        Coord::xy(base + span, base),
    ]));

    assert!(intersects(&a, &b));
    assert!(crosses(&a, &b));
    assert!(intersects_with_epsilon(&a, &b, 1.0e-9));
    assert!(crosses_with_epsilon(&a, &b, 1.0e-9));
}

#[test]
fn large_coordinate_near_collinear_intersection_is_detected() {
    let base = 1.0e12;
    let a = Geometry::LineString(LineString::new(vec![
        Coord::xy(base, base),
        Coord::xy(base + 1.0e6, base + 1.0),
    ]));
    let b = Geometry::LineString(LineString::new(vec![
        Coord::xy(base + 5.0e5, base + 0.5 - 1.0e-9),
        Coord::xy(base + 5.0e5, base + 0.5 + 1.0e-9),
    ]));

    assert!(intersects(&a, &b));
    assert!(intersects_with_epsilon(&a, &b, 1.0e-8));
}

#[test]
fn prepared_polygon_geometry_queries_use_fast_paths() {
    let poly = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
        ]),
        vec![],
    );
    let prepared = PreparedPolygon::new(poly.clone());

    let inside_line = Geometry::LineString(LineString::new(vec![
        Coord::xy(1.0, 1.0),
        Coord::xy(9.0, 9.0),
    ]));
    let outside_poly = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(20.0, 20.0),
            Coord::xy(25.0, 20.0),
            Coord::xy(25.0, 25.0),
            Coord::xy(20.0, 25.0),
        ]),
        vec![],
    ));

    assert!(prepared.contains_geometry(&inside_line));
    assert!(prepared.intersects_geometry(&inside_line));
    assert!(!prepared.intersects_geometry(&outside_poly));
}

#[test]
fn prepared_polygon_stress_batch_matches_unprepared() {
    // Pentagon with hole: prepared index should agree exactly with unprepared
    // contains for all 400 grid points, exercising the y-bin fast path on a
    // ring with more than the 64-bin segment count.
    let mut shell_coords = Vec::<Coord>::new();
    let n = 80;
    for i in 0..n {
        let t = i as f64 * std::f64::consts::TAU / n as f64;
        shell_coords.push(Coord::xy(10.0 * t.cos(), 10.0 * t.sin()));
    }
    let poly = Polygon::new(
        LinearRing::new(shell_coords),
        vec![LinearRing::new(vec![
            Coord::xy(-2.0, -2.0),
            Coord::xy(2.0, -2.0),
            Coord::xy(2.0, 2.0),
            Coord::xy(-2.0, 2.0),
        ])],
    );

    let gpoly = Geometry::Polygon(poly.clone());
    let prepared = PreparedPolygon::new(poly);

    let mut mismatches = 0usize;
    for ix in -12..=12 {
        for iy in -12..=12 {
            let p = Coord::xy(ix as f64, iy as f64);
            let by_prepared = prepared.contains_coord(p);
            let by_generic = contains(&gpoly, &Geometry::Point(p));
            if by_prepared != by_generic {
                mismatches += 1;
            }
        }
    }
    assert_eq!(mismatches, 0, "prepared and unprepared disagree on {} points", mismatches);
}

#[test]
fn prepared_polygon_envelope_rejection_is_fast_path() {
    let poly = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(1.0, 0.0),
            Coord::xy(1.0, 1.0),
            Coord::xy(0.0, 1.0),
        ]),
        vec![],
    );
    let prepared = PreparedPolygon::new(poly);

    // Points well outside envelope should be fast-rejected.
    for &(x, y) in &[(-10.0, 0.5), (10.0, 0.5), (0.5, -10.0), (0.5, 10.0)] {
        assert!(!prepared.contains_coord(Coord::xy(x, y)));
        assert!(!prepared.intersects_coord(Coord::xy(x, y)));
    }
}
