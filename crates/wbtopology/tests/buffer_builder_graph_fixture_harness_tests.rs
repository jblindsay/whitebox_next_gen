use wbtopology::{
    BufferBuilder,
    BufferOptions,
    Geometry,
    from_wkt,
    is_valid_polygon,
    polygon_area,
};

fn parse_polygon_wkt(text: &str) -> wbtopology::Polygon {
    match from_wkt(text).expect("failed to parse WKT") {
        Geometry::Polygon(p) => p,
        other => panic!("expected polygon WKT, got {:?}", other),
    }
}

#[test]
fn buffer_builder_graph_fixture_harness() {
    let data = include_str!("fixtures/buffer_builder_graph_cases.txt");
    for line in data.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let cols: Vec<&str> = trimmed.split('|').collect();
        assert_eq!(
            cols.len(),
            7,
            "expected 7 pipe-delimited columns for line: {trimmed}"
        );

        let name = cols[0].trim();
        let input = parse_polygon_wkt(cols[1].trim());
        let distance: f64 = cols[2].trim().parse().expect("invalid distance");
        let quadrant_segments: usize = cols[3]
            .trim()
            .parse()
            .expect("invalid quadrant_segments");
        let min_area_ratio: f64 = cols[4].trim().parse().expect("invalid min_area_ratio");
        let max_area_ratio: f64 = cols[5].trim().parse().expect("invalid max_area_ratio");
        let expect_valid: bool = cols[6].trim().parse().expect("invalid expect_valid");

        let mut options = BufferOptions::default();
        options.quadrant_segments = quadrant_segments;

        let out = BufferBuilder::new(options).build_polygon(&input, distance);
        assert_eq!(
            is_valid_polygon(&out),
            expect_valid,
            "{name}: validity mismatch"
        );

        let in_area = polygon_area(&input).abs();
        let out_area = polygon_area(&out).abs();
        let area_ratio = if in_area <= 1.0e-12 {
            0.0
        } else {
            out_area / in_area
        };

        assert!(
            area_ratio >= min_area_ratio && area_ratio <= max_area_ratio,
            "{name}: area ratio {} outside [{}, {}]",
            area_ratio,
            min_area_ratio,
            max_area_ratio
        );
    }
}
