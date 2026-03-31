use wbtopology::{node_linestrings, Coord, LineString};

fn has_endpoint(ls: &LineString, p: Coord, eps: f64) -> bool {
    ls.coords
        .iter()
        .any(|c| (c.x - p.x).abs() <= eps && (c.y - p.y).abs() <= eps)
}

#[test]
fn noding_splits_two_crossing_segments() {
    let lines = vec![
        LineString::new(vec![Coord::xy(0.0, 5.0), Coord::xy(10.0, 5.0)]),
        LineString::new(vec![Coord::xy(5.0, 0.0), Coord::xy(5.0, 10.0)]),
    ];

    let out = node_linestrings(&lines, 1.0e-9);
    assert_eq!(out.len(), 4);

    let center = Coord::xy(5.0, 5.0);
    let touching_center = out
        .iter()
        .filter(|ls| has_endpoint(ls, center, 1.0e-9))
        .count();
    assert_eq!(touching_center, 4);
}

#[test]
fn noding_preserves_non_intersecting_line() {
    let lines = vec![LineString::new(vec![Coord::xy(0.0, 0.0), Coord::xy(5.0, 0.0)])];
    let out = node_linestrings(&lines, 1.0e-9);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].coords[0], Coord::xy(0.0, 0.0));
    assert_eq!(out[0].coords[1], Coord::xy(5.0, 0.0));
}

#[test]
fn noding_splits_t_junction() {
    let lines = vec![
        LineString::new(vec![Coord::xy(0.0, 0.0), Coord::xy(10.0, 0.0)]),
        LineString::new(vec![Coord::xy(5.0, 0.0), Coord::xy(5.0, 4.0)]),
    ];

    let out = node_linestrings(&lines, 1.0e-9);
    assert_eq!(out.len(), 3);

    let junction = Coord::xy(5.0, 0.0);
    let touching = out
        .iter()
        .filter(|ls| has_endpoint(ls, junction, 1.0e-9))
        .count();
    assert_eq!(touching, 3);
}

#[test]
fn noding_splits_collinear_overlap_boundaries() {
    let lines = vec![
        LineString::new(vec![Coord::xy(0.0, 0.0), Coord::xy(10.0, 0.0)]),
        LineString::new(vec![Coord::xy(2.0, 0.0), Coord::xy(8.0, 0.0)]),
    ];

    let out = node_linestrings(&lines, 1.0e-9);
    assert_eq!(out.len(), 4);

    let at2 = Coord::xy(2.0, 0.0);
    let at8 = Coord::xy(8.0, 0.0);
    assert!(out.iter().any(|ls| has_endpoint(ls, at2, 1.0e-9)));
    assert!(out.iter().any(|ls| has_endpoint(ls, at8, 1.0e-9)));
}

#[test]
fn noding_splits_large_coordinate_crossing_segments() {
    let base = 1.0e12;
    let span = 1.0e6;
    let lines = vec![
        LineString::new(vec![Coord::xy(base, base), Coord::xy(base + span, base + span)]),
        LineString::new(vec![Coord::xy(base, base + span), Coord::xy(base + span, base)]),
    ];

    let out = node_linestrings(&lines, 1.0e-9);
    assert_eq!(out.len(), 4);

    let center = Coord::xy(base + span * 0.5, base + span * 0.5);
    let touching_center = out
        .iter()
        .filter(|ls| has_endpoint(ls, center, 1.0e-6))
        .count();
    assert_eq!(touching_center, 4);
}

#[test]
fn noding_intersection_points_interpolate_z_per_segment() {
    let lines = vec![
        LineString::new(vec![
            Coord::xyz(0.0, 5.0, 0.0),
            Coord::xyz(10.0, 5.0, 10.0),
        ]),
        LineString::new(vec![
            Coord::xyz(5.0, 0.0, 100.0),
            Coord::xyz(5.0, 10.0, 200.0),
        ]),
    ];

    let out = node_linestrings(&lines, 1.0e-9);
    let center = Coord::xy(5.0, 5.0);
    let mut z_values: Vec<f64> = out
        .iter()
        .flat_map(|ls| ls.coords.iter())
        .filter(|c| (c.x - center.x).abs() <= 1.0e-9 && (c.y - center.y).abs() <= 1.0e-9)
        .filter_map(|c| c.z)
        .collect();

    z_values.sort_by(|a, b| a.total_cmp(b));
    z_values.dedup_by(|a, b| (*a - *b).abs() <= 1.0e-9);

    assert_eq!(z_values, vec![5.0, 150.0]);
}
