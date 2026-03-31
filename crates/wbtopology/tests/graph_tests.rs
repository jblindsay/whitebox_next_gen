use wbtopology::{Coord, LineString, TopologyGraph};

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
    assert_eq!(c1_from, Coord::xy(1.0, 0.0));
    assert_eq!(c1_to, Coord::xy(1.0, 1.0));

    let c2_from = g.nodes[g.edges[e2].from].coord;
    let c2_to = g.nodes[g.edges[e2].to].coord;
    assert_eq!(c2_from, Coord::xy(1.0, 1.0));
    assert_eq!(c2_to, Coord::xy(0.0, 1.0));

    let c3_from = g.nodes[g.edges[e3].from].coord;
    let c3_to = g.nodes[g.edges[e3].to].coord;
    assert_eq!(c3_from, Coord::xy(0.0, 1.0));
    assert_eq!(c3_to, Coord::xy(0.0, 0.0));
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
fn graph_collapses_duplicate_coincident_segments() {
    let lines = vec![
        LineString::new(vec![Coord::xy(0.0, 0.0), Coord::xy(2.0, 0.0)]),
        LineString::new(vec![Coord::xy(2.0, 0.0), Coord::xy(0.0, 0.0)]),
    ];

    let g = TopologyGraph::from_linestrings(&lines, 1.0e-9);
    assert_eq!(g.node_count(), 2);
    assert_eq!(g.edge_count(), 1);
    assert_eq!(g.directed_edge_count(), 2);
}
