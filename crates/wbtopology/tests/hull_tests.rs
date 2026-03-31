use wbtopology::{
    concave_hull,
    concave_hull_geometry,
    concave_hull_with_options,
    concave_hull_with_precision,
    convex_hull,
    convex_hull_geometry,
    convex_hull_with_precision,
    geometry_area,
    ConcaveHullEngine,
    ConcaveHullOptions,
    Coord,
    Geometry,
    LineString,
    LinearRing,
    PrecisionModel,
    Polygon,
};

fn as_polygon(g: &Geometry) -> &Polygon {
    match g {
        Geometry::Polygon(p) => p,
        _ => panic!("expected polygon, got {g:?}"),
    }
}

#[test]
fn convex_hull_of_single_point_returns_point() {
    let g = convex_hull(&[Coord::xy(1.0, 2.0)], 1.0e-12);
    assert_eq!(g, Geometry::Point(Coord::xy(1.0, 2.0)));
}

#[test]
fn convex_hull_of_collinear_points_returns_endpoints() {
    let pts = vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(1.0, 0.0),
        Coord::xy(2.0, 0.0),
        Coord::xy(3.0, 0.0),
    ];
    let g = convex_hull(&pts, 1.0e-12);
    match g {
        Geometry::LineString(ls) => {
            assert_eq!(ls.coords, vec![Coord::xy(0.0, 0.0), Coord::xy(3.0, 0.0)]);
        }
        _ => panic!("expected linestring, got {g:?}"),
    }
}

#[test]
fn convex_hull_discards_interior_points() {
    let pts = vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(4.0, 0.0),
        Coord::xy(4.0, 4.0),
        Coord::xy(0.0, 4.0),
        Coord::xy(2.0, 2.0),
        Coord::xy(1.0, 1.0),
    ];
    let g = convex_hull(&pts, 1.0e-12);
    let poly = as_polygon(&g);
    assert_eq!(poly.exterior.coords.len(), 5);
    assert!((geometry_area(&g) - 16.0).abs() < 1.0e-9);
}

#[test]
fn convex_hull_geometry_collects_across_components() {
    let g = Geometry::GeometryCollection(vec![
        Geometry::Point(Coord::xy(0.0, 0.0)),
        Geometry::LineString(LineString::new(vec![Coord::xy(5.0, 1.0), Coord::xy(6.0, 3.0)])),
        Geometry::Polygon(Polygon::new(
            LinearRing::new(vec![
                Coord::xy(1.0, 4.0),
                Coord::xy(2.0, 4.0),
                Coord::xy(2.0, 5.0),
                Coord::xy(1.0, 5.0),
            ]),
            vec![],
        )),
    ]);
    let hull = convex_hull_geometry(&g, 1.0e-12);
    match hull {
        Geometry::Polygon(_) => {}
        _ => panic!("expected polygon, got {hull:?}"),
    }
}

#[test]
fn concave_hull_of_square_cloud_returns_polygon() {
    let pts = vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(2.0, 0.0),
        Coord::xy(4.0, 0.0),
        Coord::xy(4.0, 2.0),
        Coord::xy(4.0, 4.0),
        Coord::xy(2.0, 4.0),
        Coord::xy(0.0, 4.0),
        Coord::xy(0.0, 2.0),
        Coord::xy(2.0, 2.0),
    ];
    let g = concave_hull(&pts, 3.0, 1.0e-12);
    match g {
        Geometry::Polygon(_) => {}
        _ => panic!("expected polygon, got {g:?}"),
    }
    assert!((geometry_area(&g) - 16.0).abs() < 1.0e-9);
}

#[test]
fn concave_hull_can_be_smaller_than_convex_hull_for_u_shape() {
    let pts = vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(2.0, 0.0),
        Coord::xy(4.0, 0.0),
        Coord::xy(6.0, 0.0),
        Coord::xy(0.0, 2.0),
        Coord::xy(6.0, 2.0),
        Coord::xy(0.0, 4.0),
        Coord::xy(2.0, 4.0),
        Coord::xy(4.0, 4.0),
        Coord::xy(6.0, 4.0),
    ];

    let convex = convex_hull(&pts, 1.0e-12);
    let concave = concave_hull(&pts, 2.9, 1.0e-12);

    match concave {
        Geometry::Polygon(_) | Geometry::MultiPolygon(_) => {}
        _ => panic!("expected areal concave hull, got {concave:?}"),
    }

    let convex_area = geometry_area(&convex);
    let concave_area = geometry_area(&concave);
    assert!(concave_area < convex_area, "concave area {concave_area} should be < convex area {convex_area}");
}

#[test]
fn concave_hull_geometry_works_for_polygon_vertices() {
    let poly = Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(8.0, 0.0),
            Coord::xy(8.0, 8.0),
            Coord::xy(5.0, 8.0),
            Coord::xy(5.0, 3.0),
            Coord::xy(3.0, 3.0),
            Coord::xy(3.0, 8.0),
            Coord::xy(0.0, 8.0),
        ]),
        vec![],
    ));

    let hull = concave_hull_geometry(&poly, 4.5, 1.0e-12);
    match hull {
        Geometry::Polygon(_) | Geometry::MultiPolygon(_) => {}
        _ => panic!("expected polygonal output, got {hull:?}"),
    }
}

#[test]
fn convex_hull_with_precision_snaps_input_before_hulling() {
    let pts = vec![
        Coord::xy(0.02, 0.03),
        Coord::xy(1.98, 0.01),
        Coord::xy(2.01, 1.97),
        Coord::xy(0.04, 2.02),
    ];
    let g = convex_hull_with_precision(&pts, PrecisionModel::Fixed { scale: 1.0 });
    let poly = as_polygon(&g);
    for c in &poly.exterior.coords {
        assert!((c.x - c.x.round()).abs() < 1.0e-9);
        assert!((c.y - c.y.round()).abs() < 1.0e-9);
    }
}

#[test]
fn concave_hull_with_precision_matches_manual_presnap() {
    let pts = vec![
        Coord::xy(0.02, 0.01),
        Coord::xy(1.98, 0.03),
        Coord::xy(4.02, 0.02),
        Coord::xy(0.01, 2.01),
        Coord::xy(1.98, 2.01),
        Coord::xy(4.01, 2.02),
        Coord::xy(0.03, 4.02),
        Coord::xy(2.02, 3.99),
        Coord::xy(4.00, 3.99),
        Coord::xy(0.02, 1.98),
        Coord::xy(4.02, 1.98),
    ];
    let pm = PrecisionModel::Fixed { scale: 1.0 };

    let by_wrapper = concave_hull_with_precision(&pts, 3.0, pm);
    assert!(matches!(by_wrapper, Geometry::Polygon(_) | Geometry::MultiPolygon(_)));

    let snapped_convex = convex_hull_with_precision(&pts, pm);
    assert!(geometry_area(&by_wrapper) <= geometry_area(&snapped_convex) + 1.0e-9);

    match &by_wrapper {
        Geometry::Polygon(poly) => {
            for c in &poly.exterior.coords {
                assert!((c.x - c.x.round()).abs() < 1.0e-9);
                assert!((c.y - c.y.round()).abs() < 1.0e-9);
            }
        }
        Geometry::MultiPolygon(polys) => {
            for poly in polys {
                for c in &poly.exterior.coords {
                    assert!((c.x - c.x.round()).abs() < 1.0e-9);
                    assert!((c.y - c.y.round()).abs() < 1.0e-9);
                }
            }
        }
        _ => unreachable!(),
    }
}

#[test]
fn concave_hull_options_can_collapse_disjoint_output_to_largest_component() {
    let pts = vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(2.0, 0.0),
        Coord::xy(4.0, 0.0),
        Coord::xy(4.0, 2.0),
        Coord::xy(4.0, 4.0),
        Coord::xy(2.0, 4.0),
        Coord::xy(0.0, 4.0),
        Coord::xy(0.0, 2.0),
        Coord::xy(2.0, 2.0),
        Coord::xy(20.0, 0.0),
        Coord::xy(21.0, 0.0),
        Coord::xy(21.0, 1.0),
        Coord::xy(20.0, 1.0),
        Coord::xy(20.5, 0.5),
    ];

    let disjoint = concave_hull_with_options(
        &pts,
        ConcaveHullOptions {
            max_edge_length: 3.1,
            epsilon: 1.0e-12,
            allow_disjoint: true,
            ..Default::default()
        },
    );
    match disjoint {
        Geometry::MultiPolygon(_) => {}
        _ => panic!("expected multipolygon, got {disjoint:?}"),
    }

    let single = concave_hull_with_options(
        &pts,
        ConcaveHullOptions {
            max_edge_length: 3.1,
            epsilon: 1.0e-12,
            allow_disjoint: false,
            ..Default::default()
        },
    );
    match single {
        Geometry::Polygon(_) => {}
        _ => panic!("expected single polygon, got {single:?}"),
    }
    assert!(geometry_area(&single) > 10.0);
    assert!(geometry_area(&single) <= geometry_area(&disjoint));
}

#[test]
fn concave_hull_options_min_area_drops_tiny_components() {
    let pts = vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(2.0, 0.0),
        Coord::xy(4.0, 0.0),
        Coord::xy(4.0, 2.0),
        Coord::xy(4.0, 4.0),
        Coord::xy(2.0, 4.0),
        Coord::xy(0.0, 4.0),
        Coord::xy(0.0, 2.0),
        Coord::xy(2.0, 2.0),
        Coord::xy(20.0, 0.0),
        Coord::xy(21.0, 0.0),
        Coord::xy(21.0, 1.0),
        Coord::xy(20.0, 1.0),
        Coord::xy(20.5, 0.5),
    ];

    let all_parts = concave_hull_with_options(
        &pts,
        ConcaveHullOptions {
            max_edge_length: 3.1,
            epsilon: 1.0e-12,
            allow_disjoint: true,
            min_area: 0.0,
            ..Default::default()
        },
    );
    let filtered = concave_hull_with_options(
        &pts,
        ConcaveHullOptions {
            max_edge_length: 3.1,
            epsilon: 1.0e-12,
            allow_disjoint: true,
            min_area: 2.0,
            ..Default::default()
        },
    );

    match filtered {
        Geometry::Polygon(_) | Geometry::MultiPolygon(_) => {}
        _ => panic!("expected polygonal output, got {filtered:?}"),
    }
    assert!(geometry_area(&filtered) < geometry_area(&all_parts));
    assert!(geometry_area(&filtered) > 10.0);
}

#[test]
fn concave_hull_fast_refine_engine_returns_polygonal_output() {
    let pts = vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(2.0, 0.0),
        Coord::xy(4.0, 0.0),
        Coord::xy(6.0, 0.0),
        Coord::xy(8.0, 0.0),
        Coord::xy(0.0, 2.0),
        Coord::xy(8.0, 2.0),
        Coord::xy(0.0, 4.0),
        Coord::xy(2.0, 4.0),
        Coord::xy(4.0, 4.0),
        Coord::xy(6.0, 4.0),
        Coord::xy(8.0, 4.0),
        Coord::xy(2.0, 2.0),
        Coord::xy(6.0, 2.0),
    ];

    let fast = concave_hull_with_options(
        &pts,
        ConcaveHullOptions {
            engine: ConcaveHullEngine::FastRefine,
            max_edge_length: 2.9,
            epsilon: 1.0e-12,
            ..Default::default()
        },
    );
    match fast {
        Geometry::Polygon(_) | Geometry::MultiPolygon(_) => {}
        _ => panic!("expected polygonal output, got {fast:?}"),
    }

    let convex = convex_hull(&pts, 1.0e-12);
    assert!(geometry_area(&fast) <= geometry_area(&convex) + 1.0e-9);
}
