use wbtopology::{
    delaunay_triangulation,
    delaunay_triangulation_with_constraints,
    delaunay_triangulation_with_options,
    delaunay_triangulation_with_options_checked,
    delaunay_triangulation_with_precision,
    Coord,
    PrecisionModel,
    TriangulationOptions,
};

fn tri_area2(a: Coord, b: Coord, c: Coord) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn point_in_circumcircle(p: Coord, a: Coord, b: Coord, c: Coord) -> bool {
    // Assumes (a,b,c) is CCW; for CW we swap b/c in caller.
    let ax = a.x - p.x;
    let ay = a.y - p.y;
    let bx = b.x - p.x;
    let by = b.y - p.y;
    let cx = c.x - p.x;
    let cy = c.y - p.y;

    let det = (ax * ax + ay * ay) * (bx * cy - by * cx)
        - (bx * bx + by * by) * (ax * cy - ay * cx)
        + (cx * cx + cy * cy) * (ax * by - ay * bx);

    det > 1.0e-9
}

#[test]
fn delaunay_square_has_two_triangles() {
    let pts = vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(1.0, 0.0),
        Coord::xy(1.0, 1.0),
        Coord::xy(0.0, 1.0),
    ];
    let tri = delaunay_triangulation(&pts, 1.0e-12);
    assert_eq!(tri.points.len(), 4);
    assert_eq!(tri.triangles.len(), 2);

    let mut total_area = 0.0;
    for t in &tri.triangles {
        let a = tri.points[t[0]];
        let b = tri.points[t[1]];
        let c = tri.points[t[2]];
        total_area += tri_area2(a, b, c).abs() * 0.5;
    }

    assert!((total_area - 1.0).abs() <= 1.0e-9);
}

#[test]
fn delaunay_filters_near_duplicate_points() {
    let pts = vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(1.0, 0.0),
        Coord::xy(1.0, 1.0),
        Coord::xy(0.0, 1.0),
        Coord::xy(0.0 + 1.0e-11, 0.0),
        Coord::xy(1.0, 1.0 + 1.0e-11),
    ];
    let tri = delaunay_triangulation(&pts, 1.0e-9);
    assert_eq!(tri.points.len(), 4);
    assert_eq!(tri.triangles.len(), 2);
}

#[test]
fn delaunay_collinear_yields_no_triangles() {
    let pts = vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(1.0, 0.0),
        Coord::xy(2.0, 0.0),
        Coord::xy(3.0, 0.0),
    ];
    let tri = delaunay_triangulation(&pts, 1.0e-12);
    assert_eq!(tri.triangles.len(), 0);
}

#[test]
fn delaunay_empty_circumcircle_property_small_set() {
    let pts = vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(1.0, 0.0),
        Coord::xy(2.0, 0.2),
        Coord::xy(0.2, 1.1),
        Coord::xy(1.2, 1.4),
        Coord::xy(2.1, 1.0),
        Coord::xy(0.6, 2.0),
        Coord::xy(1.8, 2.2),
    ];

    let tri = delaunay_triangulation(&pts, 1.0e-12);

    for t in &tri.triangles {
        let mut a = tri.points[t[0]];
        let mut b = tri.points[t[1]];
        let c = tri.points[t[2]];

        if tri_area2(a, b, c) < 0.0 {
            std::mem::swap(&mut a, &mut b);
        }

        for (i, p) in tri.points.iter().enumerate() {
            if i == t[0] || i == t[1] || i == t[2] {
                continue;
            }
            assert!(
                !point_in_circumcircle(*p, a, b, c),
                "point {:?} lies in circumcircle of triangle {:?}",
                p,
                t
            );
        }
    }
}

#[test]
fn delaunay_with_precision_matches_manual_presnap() {
    let pts = vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(1.0, 0.0),
        Coord::xy(1.0, 1.0),
        Coord::xy(0.0, 1.0),
        Coord::xy(1.00041, 0.99961),
    ];

    let pm = PrecisionModel::Fixed { scale: 1000.0 };
    let by_wrapper = delaunay_triangulation_with_precision(&pts, pm);

    let mut snapped = pts.clone();
    pm.apply_coords_in_place(&mut snapped);
    let by_manual = delaunay_triangulation(&snapped, pm.epsilon());

    assert_eq!(by_wrapper, by_manual);
}

#[test]
fn delaunay_with_options_matches_precision_path() {
    let pts = vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(1.0, 0.0),
        Coord::xy(1.0, 1.0),
        Coord::xy(0.0, 1.0),
        Coord::xy(1.00041, 0.99961),
    ];
    let pm = PrecisionModel::Fixed { scale: 1000.0 };

    let by_precision = delaunay_triangulation_with_precision(&pts, pm);
    let by_options = delaunay_triangulation_with_options(
        &pts,
        TriangulationOptions {
            epsilon: 1.0e-12,
            precision: Some(pm),
        },
    );

    assert_eq!(by_options, by_precision);
}

#[test]
fn constrained_triangulation_accepts_present_constraint_edge() {
    let pts = vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(1.0, 0.0),
        Coord::xy(1.0, 1.0),
        Coord::xy(0.0, 1.0),
    ];

    let tri = delaunay_triangulation(&pts, 1.0e-12);
    let t = tri.triangles[0];
    let a = tri.points[t[0]];
    let b = tri.points[t[1]];

    let checked = delaunay_triangulation_with_constraints(&pts, &[(a, b)], 1.0e-12);
    assert!(checked.is_ok());
}

#[test]
fn constrained_triangulation_rejects_missing_constraint_edge() {
    let pts = vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(1.0, 0.0),
        Coord::xy(1.0, 1.0),
        Coord::xy(0.0, 1.0),
    ];

    let bad = delaunay_triangulation_with_options_checked(
        &pts,
        TriangulationOptions {
            epsilon: 1.0e-12,
            precision: None,
        },
        &[(Coord::xy(0.0, 0.0), Coord::xy(2.0, 2.0))],
    );

    assert!(bad.is_err());
}
