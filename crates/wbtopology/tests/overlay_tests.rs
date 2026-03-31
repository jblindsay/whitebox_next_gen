use wbtopology::{
    buffer_polygon_multi,
    polygon_difference,
    polygon_difference_faces,
    polygon_intersection,
    polygon_intersection_with_precision,
    polygon_intersection_faces,
    polygon_overlay_all,
    polygon_sym_diff,
    polygon_sym_diff_faces,
    polygon_unary_dissolve,
    polygon_union,
    polygon_union_faces,
    BufferOptions,
    Coord,
    LinearRing,
    Polygon,
    PrecisionModel,
};

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Polygon {
    Polygon::new(
        LinearRing::new(vec![
            Coord::xy(x0, y0),
            Coord::xy(x1, y0),
            Coord::xy(x1, y1),
            Coord::xy(x0, y1),
        ]),
        vec![],
    )
}

fn poly_area(poly: &Polygon) -> f64 {
    fn ring_area(coords: &[Coord]) -> f64 {
        let mut s = 0.0;
        for i in 0..(coords.len() - 1) {
            s += coords[i].x * coords[i + 1].y - coords[i + 1].x * coords[i].y;
        }
        (0.5 * s).abs()
    }

    let mut area = ring_area(&poly.exterior.coords);
    for hole in &poly.holes {
        area -= ring_area(&hole.coords);
    }
    area
}

fn area_sum(polys: &[Polygon]) -> f64 {
    polys.iter().map(poly_area).sum()
}

fn bbox(poly: &Polygon) -> (f64, f64, f64, f64) {
    let min_x = poly.exterior.coords.iter().map(|c| c.x).fold(f64::INFINITY, f64::min);
    let max_x = poly.exterior.coords.iter().map(|c| c.x).fold(f64::NEG_INFINITY, f64::max);
    let min_y = poly.exterior.coords.iter().map(|c| c.y).fold(f64::INFINITY, f64::min);
    let max_y = poly.exterior.coords.iter().map(|c| c.y).fold(f64::NEG_INFINITY, f64::max);
    (min_x, min_y, max_x, max_y)
}

#[test]
fn overlay_intersection_faces_for_overlapping_rectangles() {
    let a = rect(0.0, 0.0, 2.0, 2.0);
    let b = rect(1.0, 0.5, 3.0, 2.5);
    let out = polygon_intersection_faces(&a, &b, 1.0e-9);

    assert!(!out.is_empty());
    assert!((area_sum(&out) - 1.5).abs() <= 1.0e-9);
}

#[test]
fn overlay_union_faces_returns_face_decomposition() {
    let a = rect(0.0, 0.0, 2.0, 2.0);
    let b = rect(1.0, 0.5, 3.0, 2.5);
    let out = polygon_union_faces(&a, &b, 1.0e-9);

    assert!(!out.is_empty());
    assert!((area_sum(&out) - 6.5).abs() <= 1.0e-9);
}

#[test]
fn overlay_difference_faces_returns_a_minus_b_faces() {
    let a = rect(0.0, 0.0, 2.0, 2.0);
    let b = rect(1.0, 0.5, 3.0, 2.5);
    let out = polygon_difference_faces(&a, &b, 1.0e-9);

    assert!(!out.is_empty());
    assert!((area_sum(&out) - 2.5).abs() <= 1.0e-9);
}

#[test]
fn overlay_sym_diff_faces_returns_non_overlapped_parts() {
    let a = rect(0.0, 0.0, 2.0, 2.0);
    let b = rect(1.0, 0.5, 3.0, 2.5);
    let out = polygon_sym_diff_faces(&a, &b, 1.0e-9);

    assert!(!out.is_empty());
    assert!((area_sum(&out) - 5.0).abs() <= 1.0e-9);
}

#[test]
fn overlay_collinear_overlap_rectangles_regression() {
    let a = rect(0.0, 0.0, 2.0, 2.0);
    let b = rect(1.0, 0.0, 3.0, 2.0);

    let inter = polygon_intersection_faces(&a, &b, 1.0e-9);
    let union = polygon_union_faces(&a, &b, 1.0e-9);
    let diff = polygon_difference_faces(&a, &b, 1.0e-9);
    let xor = polygon_sym_diff_faces(&a, &b, 1.0e-9);

    assert!((area_sum(&inter) - 2.0).abs() <= 1.0e-9);
    assert!((area_sum(&union) - 6.0).abs() <= 1.0e-9);
    assert!((area_sum(&diff) - 2.0).abs() <= 1.0e-9);
    assert!((area_sum(&xor) - 4.0).abs() <= 1.0e-9);
}

#[test]
fn overlay_dissolved_union_returns_single_polygon_for_overlap() {
    let a = rect(0.0, 0.0, 2.0, 2.0);
    let b = rect(1.0, 0.0, 3.0, 2.0);
    let out = polygon_union(&a, &b, 1.0e-9);

    assert_eq!(out.len(), 1);
    assert!((area_sum(&out) - 6.0).abs() <= 1.0e-9);
}

#[test]
fn overlay_dissolved_intersection_difference_and_xor_areas() {
    let a = rect(0.0, 0.0, 2.0, 2.0);
    let b = rect(1.0, 0.0, 3.0, 2.0);

    let inter = polygon_intersection(&a, &b, 1.0e-9);
    let diff = polygon_difference(&a, &b, 1.0e-9);
    let xor = polygon_sym_diff(&a, &b, 1.0e-9);

    assert_eq!(inter.len(), 1);
    assert_eq!(diff.len(), 1);
    assert_eq!(xor.len(), 2);

    assert!((area_sum(&inter) - 2.0).abs() <= 1.0e-9);
    assert!((area_sum(&diff) - 2.0).abs() <= 1.0e-9);
    assert!((area_sum(&xor) - 4.0).abs() <= 1.0e-9);
}

#[test]
fn overlay_dissolved_difference_builds_hole() {
    let outer = rect(0.0, 0.0, 4.0, 4.0);
    let inner = rect(1.0, 1.0, 3.0, 3.0);

    let out = polygon_difference(&outer, &inner, 1.0e-9);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].holes.len(), 1);
    assert!((area_sum(&out) - 12.0).abs() <= 1.0e-9);
}

#[test]
fn overlay_dissolved_containment_intersection_and_union() {
    let outer = rect(0.0, 0.0, 4.0, 4.0);
    let inner = rect(1.0, 1.0, 3.0, 3.0);

    let inter = polygon_intersection(&outer, &inner, 1.0e-9);
    let uni = polygon_union(&outer, &inner, 1.0e-9);

    assert_eq!(inter.len(), 1);
    assert_eq!(inter[0].holes.len(), 0);
    assert!((area_sum(&inter) - 4.0).abs() <= 1.0e-9);

    assert_eq!(uni.len(), 1);
    assert_eq!(uni[0].holes.len(), 0);
    assert!((area_sum(&uni) - 16.0).abs() <= 1.0e-9);
}

#[test]
fn overlay_dissolved_containment_symdiff_builds_hole() {
    let outer = rect(0.0, 0.0, 4.0, 4.0);
    let inner = rect(1.0, 1.0, 3.0, 3.0);

    let out = polygon_sym_diff(&outer, &inner, 1.0e-9);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].holes.len(), 1);
    assert!((area_sum(&out) - 12.0).abs() <= 1.0e-9);
}

#[test]
fn overlay_dissolved_containment_with_holey_contained_polygon() {
    let outer = rect(0.0, 0.0, 10.0, 10.0);
    let inner_donut = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(2.0, 2.0),
            Coord::xy(8.0, 2.0),
            Coord::xy(8.0, 8.0),
            Coord::xy(2.0, 8.0),
        ]),
        vec![LinearRing::new(vec![
            Coord::xy(4.0, 4.0),
            Coord::xy(6.0, 4.0),
            Coord::xy(6.0, 6.0),
            Coord::xy(4.0, 6.0),
        ])],
    );

    let diff = polygon_difference(&outer, &inner_donut, 1.0e-9);
    let xor = polygon_sym_diff(&outer, &inner_donut, 1.0e-9);

    assert_eq!(diff.len(), 2);
    assert_eq!(xor.len(), 2);

    // Area(outer) - Area(donut) = 100 - (36 - 4) = 68
    assert!((area_sum(&diff) - 68.0).abs() <= 1.0e-9);
    assert!((area_sum(&xor) - 68.0).abs() <= 1.0e-9);

    // One polygon should be the large shell with one hole, and one should be the island.
    assert!(diff.iter().any(|p| p.holes.len() == 1));
    assert!(diff.iter().any(|p| p.holes.is_empty()));
}

#[test]
fn overlay_dissolved_containment_with_multiple_hole_islands() {
    let outer = rect(0.0, 0.0, 10.0, 10.0);
    let contained = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(2.0, 2.0),
            Coord::xy(8.0, 2.0),
            Coord::xy(8.0, 8.0),
            Coord::xy(2.0, 8.0),
        ]),
        vec![
            LinearRing::new(vec![
                Coord::xy(3.0, 3.0),
                Coord::xy(4.0, 3.0),
                Coord::xy(4.0, 4.0),
                Coord::xy(3.0, 4.0),
            ]),
            LinearRing::new(vec![
                Coord::xy(6.0, 6.0),
                Coord::xy(7.0, 6.0),
                Coord::xy(7.0, 7.0),
                Coord::xy(6.0, 7.0),
            ]),
        ],
    );

    let out = polygon_difference(&outer, &contained, 1.0e-9);
    assert_eq!(out.len(), 3);
    assert!(out.iter().filter(|p| p.holes.len() == 1).count() == 1);
    assert!(out.iter().filter(|p| p.holes.is_empty()).count() == 2);

    // 100 - (36 - 1 - 1) = 66
    assert!((area_sum(&out) - 66.0).abs() <= 1.0e-9);
}

#[test]
fn unary_dissolve_problem_buildings_regression() {
    let polys = vec![
        Polygon::new(
            LinearRing::new(vec![
                Coord::xy(559228.732088516, 4822194.68808923),
                Coord::xy(559222.131897529, 4822179.58816143),
                Coord::xy(559212.632037973, 4822183.74832338),
                Coord::xy(559219.222147637, 4822198.84826381),
            ]),
            vec![],
        ),
        Polygon::new(
            LinearRing::new(vec![
                Coord::xy(559215.121969858, 4822117.20837978),
                Coord::xy(559215.002023641, 4822115.8682854),
                Coord::xy(559216.931837627, 4822115.71825142),
                Coord::xy(559216.632005572, 4822111.93818215),
                Coord::xy(559214.692019762, 4822112.07813043),
                Coord::xy(559214.022129058, 4822103.77810391),
                Coord::xy(559205.281997818, 4822104.46821035),
                Coord::xy(559205.671954537, 4822109.50825229),
                Coord::xy(559204.092034561, 4822109.63804911),
                Coord::xy(559204.76193503, 4822118.04825415),
            ]),
            vec![],
        ),
        Polygon::new(
            LinearRing::new(vec![
                Coord::xy(559219.161853692, 4822171.84807374),
                Coord::xy(559218.292153371, 4822158.34829678),
                Coord::xy(559217.491928177, 4822158.42820833),
                Coord::xy(559217.561999498, 4822157.39833618),
                Coord::xy(559219.79198939, 4822157.36812643),
                Coord::xy(559219.772093633, 4822155.20823168),
                Coord::xy(559209.542128598, 4822155.56809736),
                Coord::xy(559209.742047135, 4822158.99819877),
                Coord::xy(559208.942056162, 4822159.08822059),
                Coord::xy(559209.412052883, 4822165.23831708),
                Coord::xy(559208.371907525, 4822165.41818645),
                Coord::xy(559208.411860248, 4822169.08811777),
                Coord::xy(559209.61189076, 4822168.99829011),
                Coord::xy(559209.812153922, 4822172.50836368),
            ]),
            vec![],
        ),
        Polygon::new(
            LinearRing::new(vec![
                Coord::xy(559216.52194087, 4822150.22816211),
                Coord::xy(559215.902154869, 4822137.52828037),
                Coord::xy(559206.792002165, 4822137.98832104),
                Coord::xy(559207.192107961, 4822146.02829122),
                Coord::xy(559208.581973866, 4822145.95812962),
                Coord::xy(559208.822114401, 4822150.60822431),
            ]),
            vec![],
        ),
        Polygon::new(
            LinearRing::new(vec![
                Coord::xy(559218.512060728, 4822133.79829766),
                Coord::xy(559218.082068662, 4822128.83820385),
                Coord::xy(559219.132002456, 4822128.7582724),
                Coord::xy(559218.891868758, 4822125.96823493),
                Coord::xy(559218.002130543, 4822126.04813482),
                Coord::xy(559217.821921282, 4822123.93814035),
                Coord::xy(559212.66212975, 4822124.37813285),
                Coord::xy(559212.572086223, 4822123.3481821),
                Coord::xy(559210.082150659, 4822123.55825478),
                Coord::xy(559210.142075648, 4822124.34836612),
                Coord::xy(559207.842001761, 4822124.53812172),
                Coord::xy(559208.362021877, 4822130.55837577),
                Coord::xy(559205.392033239, 4822130.81819783),
                Coord::xy(559205.742060296, 4822134.89814864),
            ]),
            vec![],
        ),
        Polygon::new(
            LinearRing::new(vec![
                Coord::xy(559228.101668686, 4822218.44222239),
                Coord::xy(559234.188184298, 4822216.21055639),
                Coord::xy(559231.956536272, 4822210.73287825),
                Coord::xy(559233.68097148, 4822210.2256205),
                Coord::xy(559227.594495374, 4822196.22690324),
                Coord::xy(559219.580922789, 4822200.38594396),
                Coord::xy(559225.15991351, 4822213.2687205),
                Coord::xy(559226.072933536, 4822213.16736878),
            ]),
            vec![],
        ),
    ];

    let mut buffered = Vec::new();
    for poly in &polys {
        buffered.extend(buffer_polygon_multi(poly, 5.0, BufferOptions::default()));
    }

    let dissolved = polygon_unary_dissolve(&buffered, 1.0e-9);
    assert_eq!(dissolved.len(), 1);

    let (min_x, min_y, max_x, max_y) = bbox(&dissolved[0].poly);
    assert!(min_x < 559199.2, "min_x too large: {min_x}");
    assert!(min_y < 4822098.9, "min_y too large: {min_y}");
    assert!(max_x > 559239.0, "max_x too small: {max_x}");
    assert!(max_y > 4822223.3, "max_y too small: {max_y}");
    assert_eq!(dissolved[0].source_indices, vec![0, 1, 2, 3, 4, 5]);
}

#[test]
fn overlay_dissolved_non_containment_intersection_preserves_hole() {
    let a = Polygon::new(
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

    let b = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(-2.0, 0.0),
            Coord::xy(8.0, 0.0),
            Coord::xy(8.0, 10.0),
            Coord::xy(-2.0, 10.0),
        ]),
        vec![LinearRing::new(vec![
            Coord::xy(1.0, 3.0),
            Coord::xy(5.0, 3.0),
            Coord::xy(5.0, 7.0),
            Coord::xy(1.0, 7.0),
        ])],
    );

    let out = polygon_intersection(&a, &b, 1.0e-9);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].holes.len(), 1);

    // Shell area: [0,8]x[0,10] = 80
    // Removed interior: [1,7]x[3,7] = 24
    assert!((area_sum(&out) - 56.0).abs() <= 1.0e-9);
}

#[test]
fn overlay_with_precision_snaps_near_coincident_edges() {
    let a = rect(0.0, 0.0, 1.0, 1.0);
    let b = rect(0.9994, 0.0, 2.0004, 1.0);
    let pm = PrecisionModel::Fixed { scale: 1000.0 };

    let inter_default = polygon_intersection(&a, &b, 1.0e-9);
    let area_default = area_sum(&inter_default);
    assert!(area_default > 0.0);

    let inter_snapped = polygon_intersection_with_precision(&a, &b, pm);
    let sa = pm.apply_polygon(&a);
    let sb = pm.apply_polygon(&b);
    let inter_manual = polygon_intersection(&sa, &sb, pm.epsilon());

    assert_eq!(inter_snapped, inter_manual);
    assert!((area_sum(&inter_snapped) - area_default).abs() > 1.0e-4);
}

#[test]
fn overlay_dense_junction_stress_area_identities() {
    let a = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(12.0, 0.0),
            Coord::xy(12.0, 12.0),
            Coord::xy(0.0, 12.0),
        ]),
        vec![
            LinearRing::new(vec![
                Coord::xy(2.0, 2.0),
                Coord::xy(3.0, 2.0),
                Coord::xy(3.0, 10.0),
                Coord::xy(2.0, 10.0),
            ]),
            LinearRing::new(vec![
                Coord::xy(5.0, 2.0),
                Coord::xy(6.0, 2.0),
                Coord::xy(6.0, 10.0),
                Coord::xy(5.0, 10.0),
            ]),
            LinearRing::new(vec![
                Coord::xy(8.0, 2.0),
                Coord::xy(9.0, 2.0),
                Coord::xy(9.0, 10.0),
                Coord::xy(8.0, 10.0),
            ]),
        ],
    );

    let b = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(1.0, 1.0),
            Coord::xy(11.0, 1.0),
            Coord::xy(11.0, 11.0),
            Coord::xy(1.0, 11.0),
        ]),
        vec![
            LinearRing::new(vec![
                Coord::xy(2.0, 2.0),
                Coord::xy(10.0, 2.0),
                Coord::xy(10.0, 3.0),
                Coord::xy(2.0, 3.0),
            ]),
            LinearRing::new(vec![
                Coord::xy(2.0, 5.0),
                Coord::xy(10.0, 5.0),
                Coord::xy(10.0, 6.0),
                Coord::xy(2.0, 6.0),
            ]),
            LinearRing::new(vec![
                Coord::xy(2.0, 8.0),
                Coord::xy(10.0, 8.0),
                Coord::xy(10.0, 9.0),
                Coord::xy(2.0, 9.0),
            ]),
        ],
    );

    let inter = polygon_intersection(&a, &b, 1.0e-9);
    let uni = polygon_union(&a, &b, 1.0e-9);
    let diff = polygon_difference(&a, &b, 1.0e-9);
    let xor = polygon_sym_diff(&a, &b, 1.0e-9);

    let i = area_sum(&inter);
    let u = area_sum(&uni);
    let d = area_sum(&diff);
    let x = area_sum(&xor);
    let area_a = poly_area(&a);
    let area_b = poly_area(&b);

    // Set-theoretic identity checks for consistency under dense boundary crossings.
    assert!((u - (i + x)).abs() <= 1.0e-9);
    assert!((u - (area_a + area_b - i)).abs() <= 1.0e-9);
    assert!((d - (area_a - i)).abs() <= 1.0e-9);

    let d_ba = area_sum(&polygon_difference(&b, &a, 1.0e-9));
    assert!((x - (d + d_ba)).abs() <= 1.0e-9);
    assert!(i <= area_a + 1.0e-9);
    assert!(i <= area_b + 1.0e-9);
}

#[test]
fn overlay_dissolved_outputs_are_stable_under_operand_order() {
    let a = Polygon::new(
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
    let b = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(-2.0, 0.0),
            Coord::xy(8.0, 0.0),
            Coord::xy(8.0, 10.0),
            Coord::xy(-2.0, 10.0),
        ]),
        vec![LinearRing::new(vec![
            Coord::xy(1.0, 3.0),
            Coord::xy(5.0, 3.0),
            Coord::xy(5.0, 7.0),
            Coord::xy(1.0, 7.0),
        ])],
    );

    let union_ab = polygon_union(&a, &b, 1.0e-9);
    let union_ba = polygon_union(&b, &a, 1.0e-9);
    let inter_ab = polygon_intersection(&a, &b, 1.0e-9);
    let inter_ba = polygon_intersection(&b, &a, 1.0e-9);
    let xor_ab = polygon_sym_diff(&a, &b, 1.0e-9);
    let xor_ba = polygon_sym_diff(&b, &a, 1.0e-9);

    assert_eq!(union_ab, union_ba);
    assert_eq!(inter_ab, inter_ba);
    assert_eq!(xor_ab, xor_ba);
}

#[test]
fn overlay_edge_touch_has_zero_area_intersection() {
    let a = rect(0.0, 0.0, 1.0, 1.0);
    let b = rect(1.0, 0.0, 2.0, 1.0);

    let inter = polygon_intersection(&a, &b, 1.0e-9);
    let uni = polygon_union(&a, &b, 1.0e-9);
    let diff = polygon_difference(&a, &b, 1.0e-9);
    let xor = polygon_sym_diff(&a, &b, 1.0e-9);

    assert!(inter.is_empty());
    assert_eq!(uni.len(), 1);
    assert_eq!(diff.len(), 1);
    assert_eq!(xor.len(), 1);

    assert!((area_sum(&uni) - 2.0).abs() <= 1.0e-9);
    assert!((area_sum(&diff) - 1.0).abs() <= 1.0e-9);
    assert!((area_sum(&xor) - 2.0).abs() <= 1.0e-9);
}

#[test]
fn overlay_point_touch_keeps_components_separate() {
    let a = rect(0.0, 0.0, 1.0, 1.0);
    let b = rect(1.0, 1.0, 2.0, 2.0);

    let inter = polygon_intersection(&a, &b, 1.0e-9);
    let uni = polygon_union(&a, &b, 1.0e-9);
    let xor = polygon_sym_diff(&a, &b, 1.0e-9);

    assert!(inter.is_empty());
    assert!(!uni.is_empty());
    assert!(!xor.is_empty());
    assert!((area_sum(&uni) - 2.0).abs() <= 1.0e-9);
    assert!((area_sum(&xor) - 2.0).abs() <= 1.0e-9);
    assert!((area_sum(&uni) - area_sum(&xor)).abs() <= 1.0e-9);
}

#[test]
fn overlay_near_collinear_sliver_invariants() {
    let a = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 1.0),
            Coord::xy(0.0, 1.0),
        ]),
        vec![],
    );
    // Slightly tilted thin polygon crossing near-collinearly with A's top boundary.
    let b = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(2.0, 0.999999),
            Coord::xy(9.0, 1.000001),
            Coord::xy(9.0, 2.0),
            Coord::xy(2.0, 2.0),
        ]),
        vec![],
    );

    let inter = polygon_intersection(&a, &b, 1.0e-9);
    let uni = polygon_union(&a, &b, 1.0e-9);
    let diff = polygon_difference(&a, &b, 1.0e-9);
    let xor = polygon_sym_diff(&a, &b, 1.0e-9);

    let i = area_sum(&inter);
    let u = area_sum(&uni);
    let d = area_sum(&diff);
    let x = area_sum(&xor);
    let area_a = poly_area(&a);
    let area_b = poly_area(&b);

    assert!(i >= -1.0e-12);
    assert!(u >= -1.0e-12);
    assert!(d >= -1.0e-12);
    assert!(x >= -1.0e-12);

    assert!((u - (i + x)).abs() <= 1.0e-7);
    assert!((u - (area_a + area_b - i)).abs() <= 1.0e-7);
    assert!((d - (area_a - i)).abs() <= 1.0e-7);
}

#[test]
fn overlay_tiny_offset_overlap_invariants() {
    let a = rect(0.0, 0.0, 1.0, 1.0);
    let b = rect(1.0e-6, 1.0e-6, 1.000001, 1.000001);

    let inter = polygon_intersection(&a, &b, 1.0e-9);
    let uni = polygon_union(&a, &b, 1.0e-9);
    let diff = polygon_difference(&a, &b, 1.0e-9);
    let xor = polygon_sym_diff(&a, &b, 1.0e-9);

    let i = area_sum(&inter);
    let u = area_sum(&uni);
    let d = area_sum(&diff);
    let x = area_sum(&xor);
    let area_a = poly_area(&a);
    let area_b = poly_area(&b);

    assert!(i >= -1.0e-12);
    assert!(u >= -1.0e-12);
    assert!(d >= -1.0e-12);
    assert!(x >= -1.0e-12);

    assert!((u - (i + x)).abs() <= 1.0e-7);
    assert!((u - (area_a + area_b - i)).abs() <= 1.0e-7);
    assert!((d - (area_a - i)).abs() <= 1.0e-7);
}

#[test]
fn overlay_hole_boundary_touch_invariants() {
    let a = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(6.0, 0.0),
            Coord::xy(6.0, 6.0),
            Coord::xy(0.0, 6.0),
        ]),
        vec![LinearRing::new(vec![
            Coord::xy(2.0, 2.0),
            Coord::xy(4.0, 2.0),
            Coord::xy(4.0, 4.0),
            Coord::xy(2.0, 4.0),
        ])],
    );
    // Touches hole boundary at x=4 segment.
    let b = rect(4.0, 2.5, 5.0, 3.5);

    let inter = polygon_intersection(&a, &b, 1.0e-9);
    let uni = polygon_union(&a, &b, 1.0e-9);
    let diff = polygon_difference(&a, &b, 1.0e-9);
    let xor = polygon_sym_diff(&a, &b, 1.0e-9);

    let i = area_sum(&inter);
    let u = area_sum(&uni);
    let d = area_sum(&diff);
    let x = area_sum(&xor);
    let area_a = poly_area(&a);
    let area_b = poly_area(&b);

    assert!(i >= -1.0e-12);
    assert!(u >= -1.0e-12);
    assert!(d >= -1.0e-12);
    assert!(x >= -1.0e-12);

    assert!((u - (i + x)).abs() <= 1.0e-7);
    assert!((u - (area_a + area_b - i)).abs() <= 1.0e-7);
    assert!((d - (area_a - i)).abs() <= 1.0e-7);
}

#[test]
fn overlay_repeated_vertices_input_invariants() {
    let a = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(4.0, 0.0),
            Coord::xy(4.0, 0.0),
            Coord::xy(4.0, 4.0),
            Coord::xy(0.0, 4.0),
            Coord::xy(0.0, 4.0),
        ]),
        vec![],
    );
    let b = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(2.0, -1.0),
            Coord::xy(5.0, -1.0),
            Coord::xy(5.0, 3.0),
            Coord::xy(5.0, 3.0),
            Coord::xy(2.0, 3.0),
        ]),
        vec![],
    );

    let inter = polygon_intersection(&a, &b, 1.0e-9);
    let uni = polygon_union(&a, &b, 1.0e-9);
    let diff = polygon_difference(&a, &b, 1.0e-9);
    let xor = polygon_sym_diff(&a, &b, 1.0e-9);

    let i = area_sum(&inter);
    let u = area_sum(&uni);
    let d = area_sum(&diff);
    let x = area_sum(&xor);
    let area_a = poly_area(&a);
    let area_b = poly_area(&b);

    assert!(i >= -1.0e-12);
    assert!(u >= -1.0e-12);
    assert!(d >= -1.0e-12);
    assert!(x >= -1.0e-12);

    assert!((u - (i + x)).abs() <= 1.0e-7);
    assert!((u - (area_a + area_b - i)).abs() <= 1.0e-7);
    assert!((d - (area_a - i)).abs() <= 1.0e-7);
}

#[test]
fn overlay_ultra_thin_corridor_touch_invariants() {
    let a = rect(0.0, 0.0, 2.0, 2.0);
    // Ultra-thin corridor-like polygon that touches A at x=2 boundary.
    let b = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(2.0, 0.9),
            Coord::xy(4.0, 0.9),
            Coord::xy(4.0, 1.1),
            Coord::xy(2.0, 1.1),
        ]),
        vec![],
    );

    let inter = polygon_intersection(&a, &b, 1.0e-9);
    let uni = polygon_union(&a, &b, 1.0e-9);
    let diff = polygon_difference(&a, &b, 1.0e-9);
    let xor = polygon_sym_diff(&a, &b, 1.0e-9);

    let i = area_sum(&inter);
    let u = area_sum(&uni);
    let d = area_sum(&diff);
    let x = area_sum(&xor);
    let area_a = poly_area(&a);
    let area_b = poly_area(&b);

    assert!(i.abs() <= 1.0e-7);
    assert!((u - (area_a + area_b)).abs() <= 1.0e-7);
    assert!((d - area_a).abs() <= 1.0e-7);
    assert!((x - u).abs() <= 1.0e-7);
}

#[test]
fn overlay_all_matches_individual_ops() {
    let a = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
        ]),
        vec![
            LinearRing::new(vec![
                Coord::xy(2.0, 2.0),
                Coord::xy(3.0, 2.0),
                Coord::xy(3.0, 8.0),
                Coord::xy(2.0, 8.0),
            ]),
            LinearRing::new(vec![
                Coord::xy(6.0, 2.0),
                Coord::xy(7.0, 2.0),
                Coord::xy(7.0, 8.0),
                Coord::xy(6.0, 8.0),
            ]),
        ],
    );

    let b = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(1.0, -1.0),
            Coord::xy(11.0, -1.0),
            Coord::xy(11.0, 9.0),
            Coord::xy(1.0, 9.0),
        ]),
        vec![
            LinearRing::new(vec![
                Coord::xy(2.0, 1.0),
                Coord::xy(9.0, 1.0),
                Coord::xy(9.0, 2.0),
                Coord::xy(2.0, 2.0),
            ]),
            LinearRing::new(vec![
                Coord::xy(2.0, 5.0),
                Coord::xy(9.0, 5.0),
                Coord::xy(9.0, 6.0),
                Coord::xy(2.0, 6.0),
            ]),
        ],
    );

    let eps = 1.0e-9;
    let all = polygon_overlay_all(&a, &b, eps);

    let inter = polygon_intersection(&a, &b, eps);
    let union = polygon_union(&a, &b, eps);
    let diff = polygon_difference(&a, &b, eps);
    let xor = polygon_sym_diff(&a, &b, eps);

    let tol = 1.0e-7;
    assert!((area_sum(&all.intersection) - area_sum(&inter)).abs() <= tol);
    assert!((area_sum(&all.union) - area_sum(&union)).abs() <= tol);
    assert!((area_sum(&all.difference_ab) - area_sum(&diff)).abs() <= tol);
    assert!((area_sum(&all.sym_diff) - area_sum(&xor)).abs() <= tol);

    // Preserve set-theoretic consistency in one-pass output.
    let i = area_sum(&all.intersection);
    let u = area_sum(&all.union);
    let d = area_sum(&all.difference_ab);
    let x = area_sum(&all.sym_diff);
    let area_a = poly_area(&a);
    let area_b = poly_area(&b);
    let d_ba = area_sum(&polygon_difference(&b, &a, eps));

    assert!((u - (i + x)).abs() <= tol);
    assert!((u - (area_a + area_b - i)).abs() <= tol);
    assert!((d - (area_a - i)).abs() <= tol);
    assert!((x - (d + d_ba)).abs() <= tol);
}
