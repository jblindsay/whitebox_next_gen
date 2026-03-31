use wbtopology::{
    buffer_linestring,
    buffer_linestring_with_precision,
    buffer_point,
    buffer_point_with_precision,
    buffer_polygon,
    buffer_polygon_multi,
    buffer_polygon_with_precision,
    contains,
    is_valid_polygon,
    make_valid_polygon,
    polygonize_closed_linestrings,
    BufferCapStyle,
    BufferJoinStyle,
    BufferOptions,
    Coord,
    Geometry,
    LineString,
    LinearRing,
    PrecisionModel,
    Polygon,
};

fn ring_area_abs(coords: &[Coord]) -> f64 {
    if coords.len() < 4 {
        return 0.0;
    }
    let mut s = 0.0;
    for i in 0..(coords.len() - 1) {
        s += coords[i].x * coords[i + 1].y - coords[i + 1].x * coords[i].y;
    }
    (0.5 * s).abs()
}

#[test]
fn make_valid_polygon_drops_hole_outside_shell() {
    let poly = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
        ]),
        vec![LinearRing::new(vec![
            Coord::xy(20.0, 20.0),
            Coord::xy(21.0, 20.0),
            Coord::xy(21.0, 21.0),
            Coord::xy(20.0, 21.0),
        ])],
    );

    let out = make_valid_polygon(&poly, 1.0e-9);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].holes.len(), 0);
}

#[test]
fn polygonize_closed_linestrings_builds_shell_and_hole() {
    let shell = LineString::new(vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(10.0, 0.0),
        Coord::xy(10.0, 10.0),
        Coord::xy(0.0, 10.0),
        Coord::xy(0.0, 0.0),
    ]);
    let hole = LineString::new(vec![
        Coord::xy(3.0, 3.0),
        Coord::xy(7.0, 3.0),
        Coord::xy(7.0, 7.0),
        Coord::xy(3.0, 7.0),
        Coord::xy(3.0, 3.0),
    ]);

    let polys = polygonize_closed_linestrings(&[shell, hole], 1.0e-9);
    assert_eq!(polys.len(), 1);
    assert_eq!(polys[0].holes.len(), 1);
}

#[test]
fn point_buffer_has_reasonable_area() {
    let p = Coord::xy(0.0, 0.0);
    let r = 2.0;
    let poly = buffer_point(
        p,
        r,
        BufferOptions {
            quadrant_segments: 16,
            ..Default::default()
        },
    );

    let area = ring_area_abs(&poly.exterior.coords);
    let exact = std::f64::consts::PI * r * r;
    let rel_err = (area - exact).abs() / exact;
    assert!(rel_err < 0.03, "relative error too high: {}", rel_err);
}

#[test]
fn make_valid_polygon_splits_simple_bow_tie() {
    let bow_tie = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(2.0, 2.0),
            Coord::xy(0.0, 2.0),
            Coord::xy(2.0, 0.0),
        ]),
        vec![],
    );

    let out = make_valid_polygon(&bow_tie, 1.0e-9);
    assert_eq!(out.len(), 2);
}

#[test]
fn make_valid_polygon_intersection_point_interpolates_z() {
    let bow_tie = Polygon::new(
        LinearRing::new(vec![
            Coord::xyz(0.0, 0.0, 0.0),
            Coord::xyz(2.0, 2.0, 20.0),
            Coord::xyz(0.0, 2.0, 40.0),
            Coord::xyz(2.0, 0.0, 60.0),
        ]),
        vec![],
    );

    let out = make_valid_polygon(&bow_tie, 1.0e-9);
    assert_eq!(out.len(), 2);

    let mut found = false;
    for poly in &out {
        for c in &poly.exterior.coords {
            if (c.x - 1.0).abs() <= 1.0e-9 && (c.y - 1.0).abs() <= 1.0e-9 {
                assert_eq!(c.z, Some(10.0));
                found = true;
            }
        }
    }
    assert!(found, "expected split intersection vertex at (1, 1)");
}

#[test]
fn linestring_buffer_with_square_caps_has_area() {
    let ls = LineString::new(vec![Coord::xy(0.0, 0.0), Coord::xy(10.0, 0.0)]);
    let poly = buffer_linestring(
        &ls,
        1.0,
        BufferOptions {
            quadrant_segments: 12,
            cap_style: BufferCapStyle::Square,
            ..Default::default()
        },
    );

    assert!(ring_area_abs(&poly.exterior.coords) > 0.0);
}

#[test]
fn polygon_buffer_grows_area_for_positive_distance() {
    let p = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(4.0, 0.0),
            Coord::xy(4.0, 4.0),
            Coord::xy(0.0, 4.0),
        ]),
        vec![],
    );
    let base = ring_area_abs(&p.exterior.coords);
    let buffered = buffer_polygon(&p, 0.5, BufferOptions::default());
    let grew = ring_area_abs(&buffered.exterior.coords);
    assert!(grew > base);
}

#[test]
fn polygon_buffer_zero_distance_preserves_simple_polygon() {
    let src = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(4.0, 0.0),
            Coord::xy(4.0, 4.0),
            Coord::xy(0.0, 4.0),
        ]),
        vec![],
    );

    let buf = buffer_polygon(&src, 0.0, BufferOptions::default());
    let gpoly = Geometry::Polygon(buf.clone());

    assert!(contains(&gpoly, &Geometry::Point(Coord::xy(2.0, 2.0))));
    assert!((ring_area_abs(&buf.exterior.coords) - ring_area_abs(&src.exterior.coords)).abs() < 1.0e-9);
}

#[test]
fn polygon_buffer_negative_distance_shrinks_square() {
    let src = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(4.0, 0.0),
            Coord::xy(4.0, 4.0),
            Coord::xy(0.0, 4.0),
        ]),
        vec![],
    );

    let buf = buffer_polygon(&src, -0.5, BufferOptions::default());
    let gpoly = Geometry::Polygon(buf.clone());

    assert!(ring_area_abs(&buf.exterior.coords) < ring_area_abs(&src.exterior.coords));
    assert!(contains(&gpoly, &Geometry::Point(Coord::xy(2.0, 2.0))));
    assert!(!contains(&gpoly, &Geometry::Point(Coord::xy(0.2, 0.2))));
}

#[test]
fn make_valid_polygon_splits_triple_crossing_ring() {
    // Figure-of-eight with an extra crossing: three line segments that all
    // pass through near the origin, creating 3+ self-intersections.
    // The ring visits (0,0)→(4,4)→(0,4)→(4,0)→(2,3)→(2,0)→(0,0).
    // We just assert we get ≥1 simple polygon back, not zero.
    let star = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(4.0, 4.0),
            Coord::xy(0.0, 4.0),
            Coord::xy(4.0, 0.0),
            Coord::xy(2.0, 5.0),
            Coord::xy(2.0, -1.0),
        ]),
        vec![],
    );

    let out = make_valid_polygon(&star, 1.0e-9);
    assert!(!out.is_empty(), "expected at least one valid polygon from split");
}

#[test]
fn linestring_buffer_contains_all_source_vertices() {
    let ls = LineString::new(vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(5.0, 0.0),
        Coord::xy(5.0, 5.0),
    ]);
    let buf = buffer_linestring(&ls, 1.5, BufferOptions::default());
    let gpoly = Geometry::Polygon(buf);

    for v in &ls.coords {
        assert!(
            contains(&gpoly, &Geometry::Point(*v)),
            "vertex {:?} not inside buffer",
            v
        );
    }
}

#[test]
fn polygon_buffer_contains_source_ring_vertices() {
    let src = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(1.0, 1.0),
            Coord::xy(5.0, 1.0),
            Coord::xy(5.0, 5.0),
            Coord::xy(1.0, 5.0),
        ]),
        vec![],
    );
    let buf = buffer_polygon(&src, 1.0, BufferOptions::default());
    let gpoly = Geometry::Polygon(buf);

    for v in &src.exterior.coords {
        assert!(
            contains(&gpoly, &Geometry::Point(*v)),
            "source vertex {:?} not inside polygon buffer",
            v
        );
    }
}

#[test]
fn linestring_buffer_l_shape_does_not_fill_convex_hull_corner() {
    let ls = LineString::new(vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(5.0, 0.0),
        Coord::xy(5.0, 5.0),
    ]);
    let buf = buffer_linestring(&ls, 1.0, BufferOptions::default());
    let gpoly = Geometry::Polygon(buf);

    // Distance from (3,3) to the polyline is 2.0, so it should be outside a d=1 buffer.
    assert!(!contains(&gpoly, &Geometry::Point(Coord::xy(3.0, 3.0))));
}

#[test]
fn linestring_buffer_mitre_join_extends_farther_than_bevel() {
    let ls = LineString::new(vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(5.0, 0.0),
        Coord::xy(5.0, 5.0),
    ]);

    let mitre = buffer_linestring(
        &ls,
        1.0,
        BufferOptions {
            join_style: BufferJoinStyle::Mitre,
            ..Default::default()
        },
    );
    let bevel = buffer_linestring(
        &ls,
        1.0,
        BufferOptions {
            join_style: BufferJoinStyle::Bevel,
            ..Default::default()
        },
    );

    let mitre_max_x = mitre
        .exterior
        .coords
        .iter()
        .map(|c| c.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let bevel_max_x = bevel
        .exterior
        .coords
        .iter()
        .map(|c| c.x)
        .fold(f64::NEG_INFINITY, f64::max);

    assert!(mitre_max_x >= bevel_max_x);
}

#[test]
fn polygon_buffer_preserves_concavity_better_than_hull() {
    // Concave "L" polygon; point (3,3) is in the notch and far from boundary.
    let src = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(6.0, 0.0),
            Coord::xy(6.0, 2.0),
            Coord::xy(2.0, 2.0),
            Coord::xy(2.0, 6.0),
            Coord::xy(0.0, 6.0),
        ]),
        vec![],
    );

    let buf = buffer_polygon(&src, 0.5, BufferOptions::default());
    let gpoly = Geometry::Polygon(buf);

    // This point was incorrectly included by old hull-based buffer.
    assert!(!contains(&gpoly, &Geometry::Point(Coord::xy(3.0, 3.0))));
}

#[test]
fn polygon_buffer_shrinks_hole_for_small_distance() {
    let src = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
        ]),
        vec![LinearRing::new(vec![
            Coord::xy(3.0, 3.0),
            Coord::xy(7.0, 3.0),
            Coord::xy(7.0, 7.0),
            Coord::xy(3.0, 7.0),
        ])],
    );

    let buf = buffer_polygon(&src, 0.5, BufferOptions::default());
    let gpoly = Geometry::Polygon(buf.clone());

    // Point just outside original hole should become filled after shrink.
    assert!(contains(&gpoly, &Geometry::Point(Coord::xy(3.2, 5.0))));
    // Hole center should still be excluded for small buffer distance.
    assert!(!contains(&gpoly, &Geometry::Point(Coord::xy(5.0, 5.0))));
    assert_eq!(buf.holes.len(), 1);
}

#[test]
fn polygon_buffer_drops_hole_when_it_collapses() {
    let src = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
        ]),
        vec![LinearRing::new(vec![
            Coord::xy(3.0, 3.0),
            Coord::xy(7.0, 3.0),
            Coord::xy(7.0, 7.0),
            Coord::xy(3.0, 7.0),
        ])],
    );

    let buf = buffer_polygon(&src, 2.5, BufferOptions::default());
    let gpoly = Geometry::Polygon(buf.clone());

    // Hole width is 4.0, so shrinking by 2.5 should remove it.
    assert!(contains(&gpoly, &Geometry::Point(Coord::xy(5.0, 5.0))));
    assert!(buf.holes.is_empty());
}

#[test]
fn polygon_buffer_negative_distance_expands_hole() {
    let src = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
        ]),
        vec![LinearRing::new(vec![
            Coord::xy(4.0, 4.0),
            Coord::xy(6.0, 4.0),
            Coord::xy(6.0, 6.0),
            Coord::xy(4.0, 6.0),
        ])],
    );

    let buf = buffer_polygon(&src, -0.5, BufferOptions::default());
    let gpoly = Geometry::Polygon(buf.clone());

    assert_eq!(buf.holes.len(), 1);
    assert!(!contains(&gpoly, &Geometry::Point(Coord::xy(3.75, 5.0))));
    assert!(!contains(&gpoly, &Geometry::Point(Coord::xy(5.0, 5.0))));
    assert!(contains(&gpoly, &Geometry::Point(Coord::xy(2.0, 2.0))));
}

#[test]
fn polygon_buffer_negative_distance_returns_empty_when_hole_consumes_shell() {
    let src = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
        ]),
        vec![LinearRing::new(vec![
            Coord::xy(4.0, 4.0),
            Coord::xy(6.0, 4.0),
            Coord::xy(6.0, 6.0),
            Coord::xy(4.0, 6.0),
        ])],
    );

    let buf = buffer_polygon(&src, -2.0, BufferOptions::default());
    assert!(buf.exterior.coords.is_empty());
}

#[test]
fn linestring_buffer_sharp_angle_is_valid_polygon() {
    let ls = LineString::new(vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(4.0, 0.0),
        Coord::xy(4.2, 5.0),
    ]);

    let buf = buffer_linestring(
        &ls,
        1.0,
        BufferOptions {
            quadrant_segments: 12,
            join_style: BufferJoinStyle::Mitre,
            ..Default::default()
        },
    );

    assert!(is_valid_polygon(&buf));
}

#[test]
fn polygon_buffer_sharp_concavity_is_valid_polygon() {
    let src = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(8.0, 0.0),
            Coord::xy(8.0, 1.0),
            Coord::xy(1.2, 1.0),
            Coord::xy(1.0, 8.0),
            Coord::xy(0.0, 8.0),
        ]),
        vec![],
    );

    let buf = buffer_polygon(
        &src,
        0.8,
        BufferOptions {
            quadrant_segments: 12,
            join_style: BufferJoinStyle::Round,
            ..Default::default()
        },
    );

    assert!(is_valid_polygon(&buf));
}

#[test]
fn linestring_buffer_mitre_limit_clamps_spike_extent() {
    let ls = LineString::new(vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(5.0, 0.0),
        Coord::xy(5.5, 5.0),
    ]);

    let high = buffer_linestring(
        &ls,
        1.0,
        BufferOptions {
            join_style: BufferJoinStyle::Mitre,
            mitre_limit: 10.0,
            ..Default::default()
        },
    );
    let low = buffer_linestring(
        &ls,
        1.0,
        BufferOptions {
            join_style: BufferJoinStyle::Mitre,
            mitre_limit: 1.2,
            ..Default::default()
        },
    );

    let high_max_x = high
        .exterior
        .coords
        .iter()
        .map(|c| c.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let low_max_x = low
        .exterior
        .coords
        .iter()
        .map(|c| c.x)
        .fold(f64::NEG_INFINITY, f64::max);

    assert!(high_max_x >= low_max_x);
}

#[test]
fn precision_aware_buffer_snaps_output_grid() {
    let pm = PrecisionModel::Fixed { scale: 10.0 }; // 0.1 grid

    let point_buf = buffer_point_with_precision(
        Coord::xy(0.03, 0.07),
        1.0,
        BufferOptions::default(),
        pm,
    );

    for c in &point_buf.exterior.coords {
        let sx = c.x * 10.0;
        let sy = c.y * 10.0;
        assert!((sx - sx.round()).abs() < 1.0e-9);
        assert!((sy - sy.round()).abs() < 1.0e-9);
    }

    let ls = LineString::new(vec![Coord::xy(0.02, 0.03), Coord::xy(5.07, 0.04)]);
    let _ = buffer_linestring_with_precision(&ls, 0.5, BufferOptions::default(), pm);

    let poly = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.01, 0.01),
            Coord::xy(2.09, 0.01),
            Coord::xy(2.09, 2.09),
            Coord::xy(0.01, 2.09),
        ]),
        vec![],
    );
    let _ = buffer_polygon_with_precision(&poly, 0.3, BufferOptions::default(), pm);
}

// --- buffer_polygon_multi tests ---

#[test]
fn buffer_polygon_multi_positive_distance_returns_single_polygon() {
    let src = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(4.0, 0.0),
            Coord::xy(4.0, 4.0),
            Coord::xy(0.0, 4.0),
        ]),
        vec![],
    );
    let result = buffer_polygon_multi(&src, 1.0, BufferOptions::default());
    assert_eq!(result.len(), 1);
    // Expanded shell must be larger than the original 4x4 square.
    assert!(ring_area_abs(&result[0].exterior.coords) > 16.0);
}

#[test]
fn buffer_polygon_multi_zero_distance_returns_repaired_copy() {
    let src = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(6.0, 0.0),
            Coord::xy(6.0, 6.0),
            Coord::xy(0.0, 6.0),
        ]),
        vec![],
    );
    let result = buffer_polygon_multi(&src, 0.0, BufferOptions::default());
    assert_eq!(result.len(), 1);
    let area = ring_area_abs(&result[0].exterior.coords);
    assert!((area - 36.0).abs() < 0.1);
}

#[test]
fn buffer_polygon_multi_negative_distance_shrinks_square() {
    let src = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
        ]),
        vec![],
    );
    let result = buffer_polygon_multi(&src, -1.0, BufferOptions::default());
    assert_eq!(result.len(), 1);
    // Eroded 10x10 square by 1 unit all sides -> roughly 8x8 = 64.
    let area = ring_area_abs(&result[0].exterior.coords);
    assert!(area > 60.0 && area < 68.0);
}

#[test]
fn buffer_polygon_multi_negative_fully_eroded_returns_empty() {
    let src = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(2.0, 0.0),
            Coord::xy(2.0, 2.0),
            Coord::xy(0.0, 2.0),
        ]),
        vec![],
    );
    // Erosion by more than the inradius of the square.
    let result = buffer_polygon_multi(&src, -2.0, BufferOptions::default());
    assert!(result.is_empty());
}

#[test]
fn buffer_polygon_multi_simple_concave_erosion_stays_single_component() {
    // An L-shaped polygon: mild erosion shrinks it but does not split it.
    let src = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 6.0),
            Coord::xy(6.0, 6.0),
            Coord::xy(6.0, 10.0),
            Coord::xy(0.0, 10.0),
        ]),
        vec![],
    );
    let result = buffer_polygon_multi(&src, -0.5, BufferOptions::default());
    assert_eq!(result.len(), 1);
    assert!(ring_area_abs(&result[0].exterior.coords) > 0.0);
}

#[test]
fn buffer_polygon_multi_returns_components_for_deeply_eroded_h_shape() {
    // An H-shape can split into two rectangles under sufficient erosion of the
    // narrow connecting bar.  The outer box is 30x20; two rectangular notches
    // cut from the left and right sides leave only a 4-unit-wide central bar.
    //
    //  +---------+   +---------+
    //  |         |   |         |
    //  |  left   +---+  right  |
    //  |  notch     bar notch  |
    //  |         +---+         |
    //  |         |   |         |
    //  +---------+   +---------+
    //
    // Outer box 30x20.
    let outer = LinearRing::new(vec![
        Coord::xy(0.0,  0.0),
        Coord::xy(30.0, 0.0),
        Coord::xy(30.0, 20.0),
        Coord::xy(0.0,  20.0),
    ]);
    // Left notch hole removes columns 0-13, rows 6-14 (8-unit-wide slot).
    let left_notch = LinearRing::new(vec![
        Coord::xy(0.0,  6.0),
        Coord::xy(13.0, 6.0),
        Coord::xy(13.0, 14.0),
        Coord::xy(0.0,  14.0),
    ]);
    // Right notch hole removes columns 17-30, rows 6-14.
    let right_notch = LinearRing::new(vec![
        Coord::xy(17.0, 6.0),
        Coord::xy(30.0, 6.0),
        Coord::xy(30.0, 14.0),
        Coord::xy(17.0, 14.0),
    ]);
    let src = Polygon::new(outer, vec![left_notch, right_notch]);

    // Eroding by 3 units consumes the 4-unit-wide bar (inradius ~2), leaving
    // two separate side rectangles.
    let result = buffer_polygon_multi(&src, -3.0, BufferOptions::default());

    // At least one component must survive.
    assert!(!result.is_empty());
    // Every surviving component must have non-trivial area.
    for comp in &result {
        assert!(ring_area_abs(&comp.exterior.coords) > 1.0);
    }
}

// Regression test: buffer of a real-world concave building footprint (CW ring
// from an ESRI shapefile) must produce a polygon that fully contains the source
// polygon and extends to roughly distance=5 on all sides.
#[test]
fn buffer_problem_building_cw_ring() {
    // CW ring as read from ESRI shapefile (negative signed area).
    // IMPORTANT: shapefile exterior rings are CW; wbtopology must handle them.
    let coords = vec![
        Coord::xy(562637.537417164, 4818648.0372987),
        Coord::xy(562644.815914335, 4818647.60529134),
        Coord::xy(562644.928259569, 4818649.42786198),
        Coord::xy(562654.188202102, 4818648.86993476),
        Coord::xy(562653.642237858, 4818639.80165489),
        Coord::xy(562650.180841506, 4818640.01375644),
        Coord::xy(562649.83002479,  4818634.29045826),
        Coord::xy(562638.621044375, 4818634.96351884),
        Coord::xy(562638.991254563, 4818641.20901904),
        Coord::xy(562637.131244156, 4818641.31386885),
        Coord::xy(562637.537417164, 4818648.0372987),  // closing vertex
    ];
    let poly = Polygon::new(LinearRing::new(coords), vec![]);
    let distance = 5.0;
    let options = BufferOptions {
        quadrant_segments: 8,
        cap_style: BufferCapStyle::Round,
        join_style: BufferJoinStyle::Round,
        mitre_limit: 5.0,
    };

    // The result must be a single polygon (one component).
    let result = buffer_polygon_multi(&poly, distance, options);
    assert_eq!(result.len(), 1, "Expected 1 buffer component, got {}", result.len());

    let buf = &result[0];
    // The buffer must be valid.
    assert!(is_valid_polygon(buf), "Buffer polygon is invalid");

    // The buffer must extend to roughly distance=5 on all sides of the input bbox.
    // Input bbox: x=[562637.131, 562654.188], y=[4818634.290, 4818649.428]
    let min_x = buf.exterior.coords.iter().map(|c| c.x).fold(f64::INFINITY, f64::min);
    let max_x = buf.exterior.coords.iter().map(|c| c.x).fold(f64::NEG_INFINITY, f64::max);
    let min_y = buf.exterior.coords.iter().map(|c| c.y).fold(f64::INFINITY, f64::min);
    let max_y = buf.exterior.coords.iter().map(|c| c.y).fold(f64::NEG_INFINITY, f64::max);

    // Should extend at least 4m outward on every side (allow minor rounding).
    assert!(min_x < 562637.131 - 4.0, "Buffer too narrow on left: min_x={}", min_x);
    assert!(max_x > 562654.188 + 4.0, "Buffer too narrow on right: max_x={}", max_x);
    assert!(min_y < 4818634.290 - 4.0, "Buffer too narrow on bottom: min_y={}", min_y);
    assert!(max_y > 4818649.428 + 4.0, "Buffer too narrow on top: max_y={}", max_y);
    let buf_area = ring_area_abs(&buf.exterior.coords);
    assert!(buf_area > 400.0, "Buffer area {} too small (source ~201 m²)", buf_area);
}
