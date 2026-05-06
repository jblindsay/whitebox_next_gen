use wbtopology::{
    buffer_polygon,
    from_wkt,
    geometry_distance,
    is_valid_polygon,
    polygon_area,
    BufferOptions,
    Coord,
    Geometry,
    Polygon,
};

fn parse_polygon_wkt(text: &str) -> Polygon {
    match from_wkt(text).expect("failed to parse WKT") {
        Geometry::Polygon(p) => p,
        other => panic!("expected polygon WKT, got {:?}", other),
    }
}

fn polygon_vertices(poly: &Polygon) -> Vec<Coord> {
    let mut out = poly.exterior.coords.clone();
    for hole in &poly.holes {
        out.extend(hole.coords.iter().copied());
    }
    out
}

fn directed_hausdorff_vertices(a: &Polygon, b: &Polygon) -> f64 {
    let b_geom = Geometry::Polygon(b.clone());
    polygon_vertices(a)
        .into_iter()
        .map(|p| geometry_distance(&Geometry::Point(p), &b_geom))
        .fold(0.0, f64::max)
}

fn approx_hausdorff(a: &Polygon, b: &Polygon) -> f64 {
    directed_hausdorff_vertices(a, b).max(directed_hausdorff_vertices(b, a))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GateMode {
    StrictParity,
    InvariantOnly,
}

fn parse_gate_mode(text: &str) -> GateMode {
    match text.trim().to_ascii_lowercase().as_str() {
        "strict" | "strict_parity" => GateMode::StrictParity,
        "invariant" | "invariant_only" => GateMode::InvariantOnly,
        other => panic!("unknown gate mode: {other}"),
    }
}

#[test]
fn buffer_geos_parity_fixture_harness() {
    let data = include_str!("fixtures/buffer_geos_parity_cases.txt");
    for line in data.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let cols: Vec<&str> = trimmed.split('|').collect();
        assert_eq!(
            cols.len(),
            8,
            "expected 8 pipe-delimited columns for line: {trimmed}"
        );

        let name = cols[0].trim();
        let input = parse_polygon_wkt(cols[1].trim());
        let distance: f64 = cols[2]
            .trim()
            .parse()
            .expect("invalid distance in parity fixture");
        let qseg: usize = cols[3]
            .trim()
            .parse()
            .expect("invalid quadrant segment count in parity fixture");
        let expected = parse_polygon_wkt(cols[4].trim());
        let max_area_delta: f64 = cols[5]
            .trim()
            .parse()
            .expect("invalid max_area_delta in parity fixture");
        let max_hausdorff: f64 = cols[6]
            .trim()
            .parse()
            .expect("invalid max_hausdorff in parity fixture");
        let gate_mode = parse_gate_mode(cols[7]);

        let mut options = BufferOptions::default();
        options.quadrant_segments = qseg;
        let out = buffer_polygon(&input, distance, options);

        // Topology invariants gate.
        assert!(is_valid_polygon(&out), "{name}: output polygon invalid");

        match gate_mode {
            GateMode::StrictParity => {
                // Area-delta gate.
                let area_delta = (polygon_area(&out).abs() - polygon_area(&expected).abs()).abs();
                assert!(
                    area_delta <= max_area_delta,
                    "{name}: area delta {} exceeded max {}",
                    area_delta,
                    max_area_delta
                );

                // Approximate Hausdorff gate (vertex-sampled).
                let haus = approx_hausdorff(&out, &expected);
                assert!(
                    haus <= max_hausdorff,
                    "{name}: approx Hausdorff {} exceeded max {}",
                    haus,
                    max_hausdorff
                );
            }
            GateMode::InvariantOnly => {
                let input_area = polygon_area(&input).abs();
                let out_area = polygon_area(&out).abs();
                let tol = 1.0e-12;
                if distance > 0.0 {
                    assert!(
                        out_area + tol >= input_area,
                        "{name}: positive buffer decreased area (in={}, out={})",
                        input_area,
                        out_area
                    );
                } else if distance < 0.0 {
                    assert!(
                        out_area <= input_area + tol,
                        "{name}: negative buffer increased area (in={}, out={})",
                        input_area,
                        out_area
                    );
                }
                assert!(
                    out.exterior.coords.len() >= 4,
                    "{name}: invariant-only case produced degenerate exterior"
                );
            }
        }
    }
}
