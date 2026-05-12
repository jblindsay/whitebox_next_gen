use wbtopology::{
    node_linestrings_with_options,
    Coord,
    LineString,
    NodingOptions,
    NodingStrategy,
    PrecisionModel,
    TopologyGraph,
};

fn assert_coord_close(a: Coord, b: Coord, eps: f64) {
    assert!((a.x - b.x).abs() <= eps, "x mismatch: {} vs {}", a.x, b.x);
    assert!((a.y - b.y).abs() <= eps, "y mismatch: {} vs {}", a.y, b.y);
}

#[test]
fn graph_from_crossing_lines_has_expected_counts() {
    let lines = vec![
        LineString::new(vec![Coord::xy(0.0, 5.0), Coord::xy(10.0, 5.0)]),
        LineString::new(vec![Coord::xy(5.0, 0.0), Coord::xy(5.0, 10.0)]),
    ];

    let g = TopologyGraph::from_linestrings(&lines, 1.0e-9);
    assert_eq!(g.node_count(), 5);
    assert_eq!(g.edge_count(), 4);
    assert_eq!(g.directed_edge_count(), 8);
}

#[test]
fn graph_center_node_outgoing_is_ccw_sorted() {
    let lines = vec![
        LineString::new(vec![Coord::xy(0.0, 5.0), Coord::xy(10.0, 5.0)]),
        LineString::new(vec![Coord::xy(5.0, 0.0), Coord::xy(5.0, 10.0)]),
    ];

    let g = TopologyGraph::from_linestrings(&lines, 1.0e-9);
    let center_id = g.find_node(Coord::xy(5.0, 5.0), 1.0e-9).expect("center node missing");
    let center = &g.nodes[center_id];
    assert_eq!(center.outgoing.len(), 4);

    for w in center.outgoing.windows(2) {
        let a = g.edges[w[0]].angle;
        let b = g.edges[w[1]].angle;
        assert!(a <= b);
    }
}

#[test]
fn graph_left_face_hook_walks_square_counterclockwise() {
    let ring = LineString::new(vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(1.0, 0.0),
        Coord::xy(1.0, 1.0),
        Coord::xy(0.0, 1.0),
        Coord::xy(0.0, 0.0),
    ]);
    let g = TopologyGraph::from_linestrings(&[ring], 1.0e-9);

    let e0 = g
        .find_directed_edge(Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0), 1.0e-9)
        .expect("missing bottom edge");
    let e1 = g.next_left_face_edge(e0).expect("missing step 1");
    let e2 = g.next_left_face_edge(e1).expect("missing step 2");
    let e3 = g.next_left_face_edge(e2).expect("missing step 3");

    let c1_from = g.nodes[g.edges[e1].from].coord;
    let c1_to = g.nodes[g.edges[e1].to].coord;
    assert_coord_close(c1_from, Coord::xy(1.0, 0.0), 1.0e-9);
    assert_coord_close(c1_to, Coord::xy(1.0, 1.0), 1.0e-9);

    let c2_from = g.nodes[g.edges[e2].from].coord;
    let c2_to = g.nodes[g.edges[e2].to].coord;
    assert_coord_close(c2_from, Coord::xy(1.0, 1.0), 1.0e-9);
    assert_coord_close(c2_to, Coord::xy(0.0, 1.0), 1.0e-9);

    let c3_from = g.nodes[g.edges[e3].from].coord;
    let c3_to = g.nodes[g.edges[e3].to].coord;
    assert_coord_close(c3_from, Coord::xy(0.0, 1.0), 1.0e-9);
    assert_coord_close(c3_to, Coord::xy(0.0, 0.0), 1.0e-9);
}

#[test]
fn graph_extracts_square_faces() {
    let ring = LineString::new(vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(1.0, 0.0),
        Coord::xy(1.0, 1.0),
        Coord::xy(0.0, 1.0),
        Coord::xy(0.0, 0.0),
    ]);
    let g = TopologyGraph::from_linestrings(&[ring], 1.0e-9);

    let all_faces = g.extract_face_rings(1.0e-9);
    let bounded = g.extract_bounded_face_rings(1.0e-9);

    assert_eq!(all_faces.len(), 2);
    assert_eq!(bounded.len(), 1);
    assert_eq!(bounded[0].coords.len(), 5);
}

#[test]
fn graph_extracts_two_bounded_faces_when_split_by_diagonal() {
    let ring = LineString::new(vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(1.0, 0.0),
        Coord::xy(1.0, 1.0),
        Coord::xy(0.0, 1.0),
        Coord::xy(0.0, 0.0),
    ]);
    let diagonal = LineString::new(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 1.0)]);
    let g = TopologyGraph::from_linestrings(&[ring, diagonal], 1.0e-9);

    let bounded = g.extract_bounded_face_rings(1.0e-9);
    assert_eq!(bounded.len(), 2);
}

#[test]
fn graph_preserves_duplicate_coincident_segments() {
    let lines = vec![
        LineString::new(vec![Coord::xy(0.0, 0.0), Coord::xy(2.0, 0.0)]),
        LineString::new(vec![Coord::xy(2.0, 0.0), Coord::xy(0.0, 0.0)]),
    ];

    let g = TopologyGraph::from_linestrings(&lines, 1.0e-9);
    assert_eq!(g.node_count(), 2);
    assert_eq!(g.edge_count(), 2);
    assert_eq!(g.directed_edge_count(), 4);
}

fn ring_area(coords: &[Coord]) -> f64 {
    if coords.len() < 4 {
        return 0.0;
    }
    let origin = coords[0];
    let mut s = 0.0;
    for i in 0..(coords.len() - 1) {
        let xi = coords[i].x - origin.x;
        let yi = coords[i].y - origin.y;
        let xj = coords[i + 1].x - origin.x;
        let yj = coords[i + 1].y - origin.y;
        s += xi * yj - xj * yi;
    }
    (0.5 * s).abs()
}

#[test]
fn graph_face_rings_precision_snap_differential_diagnostics() {
    let lines = vec![
        LineString::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(10.0, 0.0),
            Coord::xy(10.0, 10.0),
            Coord::xy(0.0, 10.0),
            Coord::xy(0.0, 0.0),
        ]),
        LineString::new(vec![Coord::xy(0.0, 5.0), Coord::xy(10.0, 5.0 + 1.0e-8)]),
        LineString::new(vec![Coord::xy(5.0 - 1.0e-8, 0.0), Coord::xy(5.0, 10.0)]),
    ];

    let eps = 1.0e-9;

    let floating_noded = node_linestrings_with_options(
        &lines,
        NodingOptions {
            epsilon: eps,
            strategy: NodingStrategy::Pairwise,
            precision: Some(PrecisionModel::Floating),
        },
    );
    let snapped_noded = node_linestrings_with_options(
        &lines,
        NodingOptions {
            epsilon: eps,
            strategy: NodingStrategy::SnapRounding,
            precision: Some(PrecisionModel::Fixed { scale: 1.0e9 }),
        },
    );

    let floating_faces = TopologyGraph::from_noded_linestrings(&floating_noded, eps)
        .extract_bounded_face_rings(eps);
    let snapped_faces = TopologyGraph::from_noded_linestrings(&snapped_noded, eps)
        .extract_bounded_face_rings(eps);

    let floating_area: f64 = floating_faces.iter().map(|r| ring_area(&r.coords)).sum();
    let snapped_area: f64 = snapped_faces.iter().map(|r| ring_area(&r.coords)).sum();
    let delta = (floating_area - snapped_area).abs();
    let tol = 1.0e-6_f64.max(floating_area.abs().max(snapped_area.abs()) * 1.0e-12);

    eprintln!(
        "graph_face_rings_precision_snap_differential: floating_faces={} snapped_faces={} floating_area={:.12} snapped_area={:.12} delta={:.12} tol={:.12}",
        floating_faces.len(),
        snapped_faces.len(),
        floating_area,
        snapped_area,
        delta,
        tol
    );

    assert!(
        !floating_faces.is_empty() && !snapped_faces.is_empty(),
        "expected bounded face rings in both floating and snapped runs"
    );
    assert!(
        delta <= tol,
        "precision differential area drift exceeds tolerance: delta={delta:.12}, tol={tol:.12}"
    );
}
