use wbtopology::{
    buffer_polygon_multi,
    BufferOptions,
    Coord,
    LinearRing,
    Polygon,
};

fn parse_ring(spec: &str) -> LinearRing {
    let s = spec.trim();
    if let Some(poly_spec) = s.strip_prefix("poly:") {
        let coords: Vec<Coord> = poly_spec
            .split(',')
            .map(|pair| {
                let xy: Vec<f64> = pair
                    .split(':')
                    .map(|v| v.parse::<f64>().expect("invalid poly coordinate"))
                    .collect();
                assert_eq!(xy.len(), 2, "poly coordinate must have x:y");
                Coord::xy(xy[0], xy[1])
            })
            .collect();
        assert!(coords.len() >= 3, "poly ring needs at least 3 vertices");
        return LinearRing::new(coords);
    }

    // Rectangle shorthand: x0:y0:x1:y1
    let vals: Vec<f64> = s
        .split(':')
        .map(|v| v.parse::<f64>().expect("invalid rect coordinate"))
        .collect();
    assert_eq!(vals.len(), 4, "rect spec must have 4 values");
    LinearRing::new(vec![
        Coord::xy(vals[0], vals[1]),
        Coord::xy(vals[2], vals[1]),
        Coord::xy(vals[2], vals[3]),
        Coord::xy(vals[0], vals[3]),
    ])
}

fn parse_holes(spec: &str) -> Vec<LinearRing> {
    let s = spec.trim();
    if s == "-" {
        return Vec::new();
    }
    s.split(';').map(parse_ring).collect()
}

fn ring_area(coords: &[Coord]) -> f64 {
    if coords.len() < 4 {
        return 0.0;
    }
    let mut s = 0.0;
    for i in 0..(coords.len() - 1) {
        s += coords[i].x * coords[i + 1].y - coords[i + 1].x * coords[i].y;
    }
    (0.5 * s).abs()
}

fn polygon_area(poly: &Polygon) -> f64 {
    let mut area = ring_area(&poly.exterior.coords);
    for h in &poly.holes {
        area -= ring_area(&h.coords);
    }
    area.max(0.0)
}

#[test]
fn negative_buffer_multi_fixture_invariants() {
    let data = include_str!("fixtures/buffer_negative_multi_invariants.txt");

    for raw in data.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split('|').collect();
        assert_eq!(parts.len(), 8, "fixture line must have 8 fields: {line}");

        let name = parts[0].trim();
        let distance = parts[1].trim().parse::<f64>().expect("invalid distance");
        let outer = parse_ring(parts[2].trim());
        let holes = parse_holes(parts[3].trim());
        let min_components = parts[4].trim().parse::<usize>().expect("invalid min_components");
        let max_components = parts[5].trim().parse::<usize>().expect("invalid max_components");
        let min_area = parts[6].trim().parse::<f64>().expect("invalid min_total_area");
        let max_area = parts[7].trim().parse::<f64>().expect("invalid max_total_area");

        let src = Polygon::new(outer, holes);
        let out = buffer_polygon_multi(&src, distance, BufferOptions::default());

        assert!(
            out.len() >= min_components && out.len() <= max_components,
            "{name}: component count {} out of range [{}, {}]",
            out.len(),
            min_components,
            max_components
        );

        let total_area: f64 = out.iter().map(polygon_area).sum();
        assert!(
            total_area >= min_area && total_area <= max_area,
            "{name}: total area {} out of range [{}, {}]",
            total_area,
            min_area,
            max_area
        );

        for (i, p) in out.iter().enumerate() {
            assert!(
                p.exterior.coords.len() >= 4,
                "{name}: component {i} has invalid shell size {}",
                p.exterior.coords.len()
            );
            assert!(
                polygon_area(p) > 0.0,
                "{name}: component {i} has non-positive area"
            );
        }
    }
}
