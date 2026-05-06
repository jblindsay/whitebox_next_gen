use wbtopology::{
    Coord,
    LineString,
    NodingOptions,
    NodingStrategy,
    PolygonizeOptions,
    polygonize_linework,
};

#[test]
fn polygonize_linework_reports_dangles_for_open_chain() {
    let lines = vec![
        LineString::new(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)]),
        LineString::new(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)]),
    ];

    let out = polygonize_linework(
        &lines,
        PolygonizeOptions {
            epsilon: 1.0e-9,
            noding: NodingOptions {
                epsilon: 1.0e-9,
                strategy: NodingStrategy::Pairwise,
                precision: None,
            },
        },
    );

    assert!(out.polygons.is_empty());
    assert!(!out.dangles.is_empty());
}

#[test]
fn polygonize_linework_produces_polygon_for_closed_square() {
    let lines = vec![LineString::new(vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(1.0, 0.0),
        Coord::xy(1.0, 1.0),
        Coord::xy(0.0, 1.0),
        Coord::xy(0.0, 0.0),
    ])];

    let out = polygonize_linework(&lines, PolygonizeOptions::default());
    assert!(!out.polygons.is_empty());
}
