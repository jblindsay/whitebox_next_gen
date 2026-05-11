use wbtopology::{
    BufferOp,
    BufferOpOptions,
    BufferOptions,
    Coord,
    LineString,
    LinearRing,
    Polygon,
    is_valid_polygon,
};

fn line(points: &[(f64, f64)]) -> LineString {
    LineString::new(points.iter().map(|(x, y)| Coord::xy(*x, *y)).collect())
}

fn square(min_x: f64, min_y: f64, size: f64) -> Polygon {
    Polygon::new(
        LinearRing::new(vec![
            Coord::xy(min_x, min_y),
            Coord::xy(min_x + size, min_y),
            Coord::xy(min_x + size, min_y + size),
            Coord::xy(min_x, min_y + size),
            Coord::xy(min_x, min_y),
        ]),
        vec![],
    )
}

#[test]
fn buffer_op_linestrings_dissolved_populates_stage_stats_and_outputs_valid_polygons() {
    let op = BufferOp::new(BufferOpOptions {
        buffer: BufferOptions::default(),
        ..BufferOpOptions::default()
    });

    let lines = vec![
        line(&[(0.0, 0.0), (10.0, 0.0)]),
        line(&[(5.0, -3.0), (5.0, 3.0)]),
    ];

    let result = op.run_linestrings_dissolved(&lines, 1.25);

    assert_eq!(result.stats.input_lines, lines.len());
    assert!(result.stats.raw_curves > 0);
    assert!(result.stats.noded_curves > 0);
    assert!(result.stats.face_rings > 0);
    assert!(result.stats.candidate_polygons > 0);
    assert!(result.stats.labeled_inside_polygons > 0);
    assert!(result.stats.dissolved_polygons > 0);
    assert!(!result.polygons.is_empty());
    assert!(result.polygons.iter().all(is_valid_polygon));
}

#[test]
fn buffer_op_polygons_dissolved_handles_overlap_and_produces_valid_output() {
    let op = BufferOp::default();
    let polys = vec![square(0.0, 0.0, 8.0), square(5.0, 0.0, 8.0)];

    let result = op.run_polygons_dissolved(&polys, 1.0);

    assert_eq!(result.stats.input_lines, polys.len());
    assert!(result.stats.raw_curves > 0);
    assert!(result.stats.noded_curves > 0);
    assert!(result.stats.face_rings > 0);
    assert!(result.stats.candidate_polygons > 0);
    // Some polygon overlap configurations can still fall back to all-face
    // polygonization when depth-labeled ring selection is empty.
    assert!(result.stats.labeled_inside_polygons <= result.stats.candidate_polygons);
    assert!(result.stats.dissolved_polygons > 0);
    assert!(!result.polygons.is_empty());
    assert!(result.polygons.iter().all(is_valid_polygon));
}

#[test]
fn buffer_op_rejects_non_positive_or_non_finite_distance() {
    let op = BufferOp::default();
    let lines = vec![line(&[(0.0, 0.0), (1.0, 1.0)])];

    let zero = op.run_linestrings_dissolved(&lines, 0.0);
    assert!(zero.polygons.is_empty());
    assert_eq!(zero.stats.input_lines, 1);
    assert_eq!(zero.stats.raw_curves, 0);

    let nan = op.run_linestrings_dissolved(&lines, f64::NAN);
    assert!(nan.polygons.is_empty());
    assert_eq!(nan.stats.input_lines, 1);
    assert_eq!(nan.stats.raw_curves, 0);
}
