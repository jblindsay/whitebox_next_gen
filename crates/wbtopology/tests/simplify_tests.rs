use wbtopology::{
    is_simple_linestring,
    is_valid_polygon,
    simplify_geometry,
    simplify_geometry_topology_preserving,
    simplify_linestring,
    simplify_linestring_topology_preserving,
    simplify_polygon_coverage_topology_preserving,
    simplify_polygon,
    simplify_polygon_topology_preserving,
    Coord,
    Geometry,
    LineString,
    LinearRing,
    Polygon,
};

#[test]
fn simplify_linestring_reduces_vertices() {
    let ls = LineString::new(vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(1.0, 0.01),
        Coord::xy(2.0, -0.01),
        Coord::xy(3.0, 0.02),
        Coord::xy(4.0, 0.0),
    ]);

    let simp = simplify_linestring(&ls, 0.05);
    assert!(simp.coords.len() < ls.coords.len());
    assert_eq!(simp.coords.first(), ls.coords.first());
    assert_eq!(simp.coords.last(), ls.coords.last());
}

#[test]
fn simplify_polygon_keeps_closed_rings() {
    let poly = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(1.0, 0.01),
            Coord::xy(2.0, 0.0),
            Coord::xy(3.0, 0.01),
            Coord::xy(4.0, 0.0),
            Coord::xy(4.0, 4.0),
            Coord::xy(0.0, 4.0),
        ]),
        vec![],
    );

    let simp = simplify_polygon(&poly, 0.1);
    assert_eq!(simp.exterior.coords.first(), simp.exterior.coords.last());
    assert!(simp.exterior.coords.len() >= 4);
}

#[test]
fn simplify_geometry_handles_collections() {
    let gc = Geometry::GeometryCollection(vec![
        Geometry::LineString(LineString::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(1.0, 0.001),
            Coord::xy(2.0, 0.0),
        ])),
        Geometry::Point(Coord::xy(10.0, 10.0)),
    ]);

    let simp = simplify_geometry(&gc, 0.01);
    match simp {
        Geometry::GeometryCollection(parts) => assert_eq!(parts.len(), 2),
        _ => panic!("expected geometry collection"),
    }
}

#[test]
fn topology_preserving_linestring_simplify_keeps_simple_output() {
    let ls = LineString::new(vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(2.0, 0.2),
        Coord::xy(4.0, 0.0),
        Coord::xy(4.2, 2.0),
        Coord::xy(4.0, 4.0),
        Coord::xy(2.0, 3.8),
        Coord::xy(0.0, 4.0),
    ]);

    let simp = simplify_linestring_topology_preserving(&ls, 0.5);
    assert!(simp.coords.len() <= ls.coords.len());
    assert!(is_simple_linestring(&simp));
    assert_eq!(simp.coords.first(), ls.coords.first());
    assert_eq!(simp.coords.last(), ls.coords.last());
}

#[test]
fn topology_preserving_polygon_simplify_keeps_valid_polygon() {
    let poly = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(2.0, 0.1),
            Coord::xy(4.0, 0.0),
            Coord::xy(6.0, 0.2),
            Coord::xy(8.0, 0.0),
            Coord::xy(8.0, 8.0),
            Coord::xy(6.2, 8.0),
            Coord::xy(6.0, 6.0),
            Coord::xy(4.0, 6.2),
            Coord::xy(2.0, 6.0),
            Coord::xy(1.8, 8.0),
            Coord::xy(0.0, 8.0),
            Coord::xy(0.0, 0.0),
        ]),
        vec![LinearRing::new(vec![
            Coord::xy(2.0, 2.0),
            Coord::xy(3.0, 2.1),
            Coord::xy(4.0, 2.0),
            Coord::xy(5.0, 2.1),
            Coord::xy(6.0, 2.0),
            Coord::xy(6.0, 4.0),
            Coord::xy(5.0, 4.1),
            Coord::xy(4.0, 4.0),
            Coord::xy(3.0, 4.1),
            Coord::xy(2.0, 4.0),
            Coord::xy(2.0, 2.0),
        ])],
    );

    let simp = simplify_polygon_topology_preserving(&poly, 0.25);
    assert!(is_valid_polygon(&simp));
    assert!(simp.exterior.coords.len() <= poly.exterior.coords.len());
    assert_eq!(simp.exterior.coords.first(), simp.exterior.coords.last());
}

#[test]
fn topology_preserving_geometry_simplify_handles_collections() {
    let gc = Geometry::GeometryCollection(vec![
        Geometry::LineString(LineString::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(1.0, 0.01),
            Coord::xy(2.0, 0.0),
        ])),
        Geometry::Polygon(Polygon::new(
            LinearRing::new(vec![
                Coord::xy(0.0, 0.0),
                Coord::xy(1.0, 0.01),
                Coord::xy(2.0, 0.0),
                Coord::xy(2.0, 2.0),
                Coord::xy(0.0, 2.0),
                Coord::xy(0.0, 0.0),
            ]),
            vec![],
        )),
    ]);

    let simp = simplify_geometry_topology_preserving(&gc, 0.05);
    match simp {
        Geometry::GeometryCollection(parts) => assert_eq!(parts.len(), 2),
        _ => panic!("expected geometry collection"),
    }
}

#[test]
fn topology_preserving_coverage_simplify_preserves_shared_boundary() {
    let shared = vec![
        Coord::xy(5.0, 0.0),
        Coord::xy(5.2, 2.0),
        Coord::xy(4.8, 4.0),
        Coord::xy(5.2, 6.0),
        Coord::xy(4.8, 8.0),
        Coord::xy(5.0, 10.0),
    ];

    let left = Polygon::new(
        LinearRing::new(
            [
                vec![Coord::xy(0.0, 0.0), shared[0]],
                shared[1..].to_vec(),
                vec![Coord::xy(0.0, 10.0)],
            ]
            .concat(),
        ),
        vec![],
    );
    let mut shared_rev = shared.clone();
    shared_rev.reverse();
    let right = Polygon::new(
        LinearRing::new(
            [
                vec![Coord::xy(5.0, 0.0), Coord::xy(10.0, 0.0), Coord::xy(10.0, 10.0)],
                shared_rev.clone(),
            ]
            .concat(),
        ),
        vec![],
    );

    let simplified = simplify_polygon_coverage_topology_preserving(&[left, right], 0.35);
    assert_eq!(simplified.len(), 2);
    assert!(is_valid_polygon(&simplified[0]));
    assert!(is_valid_polygon(&simplified[1]));

    let left_shared = forward_path(
        &simplified[0].exterior.coords,
        Coord::xy(5.0, 0.0),
        Coord::xy(5.0, 10.0),
    );
    let right_shared = forward_path(
        &simplified[1].exterior.coords,
        Coord::xy(5.0, 10.0),
        Coord::xy(5.0, 0.0),
    );

    let mut reversed_right = right_shared.clone();
    reversed_right.reverse();
    assert_eq!(left_shared, reversed_right);
    assert!(left_shared.len() < shared.len());
}

fn forward_path(ring: &[Coord], start: Coord, end: Coord) -> Vec<Coord> {
    let start_idx = ring.iter().position(|c| *c == start).unwrap();
    let mut out = vec![start];
    let mut idx = start_idx;
    loop {
        idx = (idx + 1) % (ring.len() - 1);
        out.push(ring[idx]);
        if ring[idx] == end {
            break;
        }
    }
    out
}
