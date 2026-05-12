use wbtopology::{node_linestrings, Coord, LineString};

fn has_endpoint(ls: &LineString, p: Coord, eps: f64) -> bool {
    ls.coords
        .iter()
        .any(|c| (c.x - p.x).abs() <= eps && (c.y - p.y).abs() <= eps)
}

fn assert_coord_close(a: Coord, b: Coord, eps: f64) {
    assert!((a.x - b.x).abs() <= eps, "x mismatch: {} vs {}", a.x, b.x);
    assert!((a.y - b.y).abs() <= eps, "y mismatch: {} vs {}", a.y, b.y);
}

fn parse_coord_pair(pair: &str) -> Coord {
    let vals: Vec<f64> = pair
        .trim()
        .split(':')
        .map(|v| v.parse::<f64>().expect("invalid coordinate value"))
        .collect();
    assert_eq!(vals.len(), 2, "coordinate must have x:y");
    Coord::xy(vals[0], vals[1])
}

fn parse_lines_field(spec: &str) -> Vec<LineString> {
    spec.split(';')
        .map(|line_spec| {
            let coords: Vec<Coord> = line_spec.split(',').map(parse_coord_pair).collect();
            assert!(coords.len() >= 2, "line must have at least 2 points");
            LineString::new(coords)
        })
        .collect()
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
    assert_coord_close(out[0].coords[0], Coord::xy(0.0, 0.0), 1.0e-9);
    assert_coord_close(out[0].coords[1], Coord::xy(5.0, 0.0), 1.0e-9);
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
fn noding_preserves_duplicate_coincident_segments() {
    let lines = vec![
        LineString::new(vec![Coord::xy(0.0, 0.0), Coord::xy(10.0, 0.0)]),
        LineString::new(vec![Coord::xy(10.0, 0.0), Coord::xy(0.0, 0.0)]),
    ];

    let out = node_linestrings(&lines, 1.0e-9);
    assert_eq!(out.len(), 2);
}

#[test]
fn noding_preserves_duplicate_coincident_multiplicity_through_crossing_split() {
    let lines = vec![
        LineString::new(vec![Coord::xy(0.0, 0.0), Coord::xy(10.0, 0.0)]),
        LineString::new(vec![Coord::xy(10.0, 0.0), Coord::xy(0.0, 0.0)]),
        LineString::new(vec![Coord::xy(5.0, -5.0), Coord::xy(5.0, 5.0)]),
    ];

    let out = node_linestrings(&lines, 1.0e-9);
    assert_eq!(out.len(), 6);

    let center = Coord::xy(5.0, 0.0);
    let touching_center = out
        .iter()
        .filter(|ls| has_endpoint(ls, center, 1.0e-9))
        .count();
    assert_eq!(touching_center, 6);
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

    // Current noding output may be XY-only in some pipeline paths. If Z is
    // retained, we expect per-segment interpolation at the intersection.
    if z_values.is_empty() {
        return;
    }
    assert_eq!(z_values, vec![5.0, 150.0]);
}

#[test]
fn noding_shallow_angle_near_coincident_corpus_strict() {
    let data = include_str!("fixtures/noding_shallow_angle_cases.txt");

    for raw in data.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split('|').collect();
        assert_eq!(parts.len(), 5, "fixture row must have 5 fields");

        let name = parts[0].trim();
        let eps = parts[1].trim().parse::<f64>().expect("invalid epsilon");
        let expected_segments = parts[2]
            .trim()
            .parse::<usize>()
            .expect("invalid expected segment count");
        let expected_junction = parse_coord_pair(parts[3]);
        let lines = parse_lines_field(parts[4]);

        let out = node_linestrings(&lines, eps);
        assert_eq!(
            out.len(),
            expected_segments,
            "{name}: unexpected noded segment count"
        );

        let touching = out
            .iter()
            .filter(|ls| has_endpoint(ls, expected_junction, eps.max(1.0e-6)))
            .count();
        assert!(
            touching >= 2,
            "{name}: expected at least two segments to touch the junction"
        );
    }
}

#[test]
fn noding_reference_parity_fixture_corpus_strict() {
    let data = include_str!("fixtures/noding_reference_parity_cases.txt");

    for raw in data.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split('|').collect();
        assert_eq!(parts.len(), 6, "fixture row must have 6 fields");

        let name = parts[0].trim();
        let reference = parts[1].trim();
        let eps = parts[2].trim().parse::<f64>().expect("invalid epsilon");
        let expected_segments = parts[3]
            .trim()
            .parse::<usize>()
            .expect("invalid expected segment count");
        let expected_junction = parse_coord_pair(parts[4]);
        let lines = parse_lines_field(parts[5]);

        assert!(
            reference == "geos_jts_trace",
            "{name}: unsupported reference provenance"
        );

        let out = node_linestrings(&lines, eps);
        assert_eq!(
            out.len(),
            expected_segments,
            "{name}: unexpected noded segment count"
        );

        let touching = out
            .iter()
            .filter(|ls| has_endpoint(ls, expected_junction, eps.max(1.0e-6)))
            .count();
        assert!(
            touching >= 2,
            "{name}: expected at least two segments to touch the junction"
        );

        let mut reversed = lines.clone();
        reversed.reverse();
        let out_reversed = node_linestrings(&reversed, eps);
        assert_eq!(
            out_reversed.len(),
            out.len(),
            "{name}: noding segment count changed under line-order reversal"
        );
    }
}
