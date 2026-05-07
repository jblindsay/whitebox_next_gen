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
fn unary_dissolve_cascaded_strategy_merges_overlaps() {
    let polys = vec![
        square(0.0, 0.0, 10.0),
        square(8.0, 0.0, 10.0),
        square(16.0, 0.0, 10.0),
    ];

    let out = polygon_unary_dissolve_with_options(
        &polys,
        UnaryDissolveOptions {
            epsilon: 1.0e-9,
            strategy: UnaryDissolveStrategy::CascadedHeuristic,
            ..UnaryDissolveOptions::default()
        },
    );

    assert_eq!(out.len(), 1);
    let mut members = out[0].source_indices.clone();
    members.sort_unstable();
    members.dedup();
    assert_eq!(members, vec![0, 1, 2]);
}

#[test]
fn unary_dissolve_cascaded_strategy_preserves_point_touch_separation() {
    let polys = vec![square(0.0, 0.0, 10.0), square(10.0, 10.0, 10.0)];

    let out = polygon_unary_dissolve_with_options(
        &polys,
        UnaryDissolveOptions {
            epsilon: 1.0e-9,
            strategy: UnaryDissolveStrategy::CascadedHeuristic,
            ..UnaryDissolveOptions::default()
        },
    );

    assert_eq!(out.len(), 2);
    let mut memberships: Vec<Vec<usize>> = out
        .iter()
        .map(|group| {
            let mut ids = group.source_indices.clone();
            ids.sort_unstable();
            ids.dedup();
            ids
        })
        .collect();
    memberships.sort();
    assert_eq!(memberships, vec![vec![0], vec![1]]);
}
