use wbtopology::{
    Coord,
    LinearRing,
    Polygon,
    UnaryDissolveOptions,
    UnaryDissolveStrategy,
    polygon_unary_dissolve_with_options,
};

fn square(x0: f64, y0: f64, size: f64) -> Polygon {
    Polygon::new(
        LinearRing::new(vec![
            Coord::xy(x0, y0),
            Coord::xy(x0 + size, y0),
            Coord::xy(x0 + size, y0 + size),
            Coord::xy(x0, y0 + size),
            Coord::xy(x0, y0),
        ]),
        vec![],
    )
}

#[test]
fn unary_dissolve_graph_strategy_merges_overlapping_squares() {
    let polys = vec![square(0.0, 0.0, 10.0), square(8.0, 0.0, 10.0)];
    let out = polygon_unary_dissolve_with_options(
        &polys,
        UnaryDissolveOptions {
            epsilon: 1.0e-9,
            strategy: UnaryDissolveStrategy::GraphDriven,
            ..UnaryDissolveOptions::default()
        },
    );

    assert_eq!(out.len(), 1);
    let mut memberships: Vec<Vec<usize>> = out
        .iter()
        .map(|d| {
            let mut ids = d.source_indices.clone();
            ids.sort_unstable();
            ids.dedup();
            ids
        })
        .collect();
    memberships.sort();
    assert_eq!(memberships, vec![vec![0, 1]]);
}

#[test]
fn unary_dissolve_graph_strategy_merges_edge_touching_squares() {
    let polys = vec![square(0.0, 0.0, 10.0), square(10.0, 0.0, 10.0)];
    let out = polygon_unary_dissolve_with_options(
        &polys,
        UnaryDissolveOptions {
            epsilon: 1.0e-9,
            strategy: UnaryDissolveStrategy::GraphDriven,
            ..UnaryDissolveOptions::default()
        },
    );

    assert_eq!(out.len(), 1);
    let mut memberships: Vec<Vec<usize>> = out
        .iter()
        .map(|d| {
            let mut ids = d.source_indices.clone();
            ids.sort_unstable();
            ids.dedup();
            ids
        })
        .collect();
    memberships.sort();
    assert_eq!(memberships, vec![vec![0, 1]]);
}

#[test]
fn unary_dissolve_graph_strategy_keeps_point_touching_squares_separate() {
    let polys = vec![square(0.0, 0.0, 10.0), square(10.0, 10.0, 10.0)];
    let out = polygon_unary_dissolve_with_options(
        &polys,
        UnaryDissolveOptions {
            epsilon: 1.0e-9,
            strategy: UnaryDissolveStrategy::GraphDriven,
            ..UnaryDissolveOptions::default()
        },
    );

    assert_eq!(out.len(), 2);
    let mut memberships: Vec<Vec<usize>> = out
        .iter()
        .map(|d| {
            let mut ids = d.source_indices.clone();
            ids.sort_unstable();
            ids.dedup();
            ids
        })
        .collect();
    memberships.sort();
    assert_eq!(memberships, vec![vec![0], vec![1]]);
}
