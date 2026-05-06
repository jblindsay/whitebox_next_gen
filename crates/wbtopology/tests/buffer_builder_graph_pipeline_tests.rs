use wbtopology::{
    BufferBuilder,
    BufferOptions,
    Coord,
    LinearRing,
    Polygon,
    is_valid_polygon,
    polygon_area,
};

fn square(size: f64) -> Polygon {
    Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(size, 0.0),
            Coord::xy(size, size),
            Coord::xy(0.0, size),
            Coord::xy(0.0, 0.0),
        ]),
        vec![],
    )
}

#[test]
fn buffer_builder_graph_pipeline_positive_square_is_valid_and_expands_area() {
    let poly = square(10.0);
    let out = BufferBuilder::new(BufferOptions::default()).build_polygon(&poly, 2.0);

    assert!(is_valid_polygon(&out));
    assert!(polygon_area(&out).abs() > polygon_area(&poly).abs());
}

#[test]
fn buffer_builder_graph_pipeline_negative_uses_legacy_semantics() {
    let poly = square(10.0);
    let out = BufferBuilder::new(BufferOptions::default()).build_polygon(&poly, -1.0);

    assert!(is_valid_polygon(&out));
    assert!(polygon_area(&out).abs() < polygon_area(&poly).abs());
}
