use wbtopology::{
    buffer_polygon_multi,
    contains,
    BufferOptions,
    Coord,
    Geometry,
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

fn parse_bool(token: &str) -> bool {
    matches!(token.trim(), "true" | "True" | "TRUE" | "1" | "yes" | "Yes" | "YES")
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

fn bbox_of_ring(coords: &[Coord]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for c in coords {
        min_x = min_x.min(c.x);
        max_x = max_x.max(c.x);
        min_y = min_y.min(c.y);
        max_y = max_y.max(c.y);
    }

    (min_x, max_x, min_y, max_y)
}

fn bbox_of_polygons(polys: &[Polygon]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for p in polys {
        let (px0, px1, py0, py1) = bbox_of_ring(&p.exterior.coords);
        min_x = min_x.min(px0);
        max_x = max_x.max(px1);
        min_y = min_y.min(py0);
        max_y = max_y.max(py1);
    }

    (min_x, max_x, min_y, max_y)
}

fn point_in_any_polygon(point: Coord, polys: &[Polygon]) -> bool {
    polys.iter()
        .any(|poly| contains(&Geometry::Polygon(poly.clone()), &Geometry::Point(point)))
}

#[test]
fn positive_buffer_fixture_invariants() {
    let data = include_str!("fixtures/buffer_positive_invariants.txt");

    for raw in data.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split('|').collect();
        assert_eq!(
            parts.len(),
            12,
            "fixture line must have 12 fields: {line}"
        );

        let name = parts[0].trim();
        let distance = parts[1].trim().parse::<f64>().expect("invalid distance");
        let outer = parse_ring(parts[2].trim());
        let holes = parse_holes(parts[3].trim());
        let min_components = parts[4]
            .trim()
            .parse::<usize>()
            .expect("invalid min_components");
        let max_components = parts[5]
            .trim()
            .parse::<usize>()
            .expect("invalid max_components");
        let min_area = parts[6].trim().parse::<f64>().expect("invalid min_total_area");
        let max_area = parts[7].trim().parse::<f64>().expect("invalid max_total_area");
        let min_expand = parts[8].trim().parse::<f64>().expect("invalid min_expand");
        let min_total_holes = parts[9]
            .trim()
            .parse::<usize>()
            .expect("invalid min_total_holes");
        let max_total_holes = parts[10]
            .trim()
            .parse::<usize>()
            .expect("invalid max_total_holes");
        let contain_source_vertices = parse_bool(parts[11]);

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

        let total_holes: usize = out.iter().map(|p| p.holes.len()).sum();
        assert!(
            total_holes >= min_total_holes && total_holes <= max_total_holes,
            "{name}: total holes {} out of range [{}, {}]",
            total_holes,
            min_total_holes,
            max_total_holes
        );

        let (src_min_x, src_max_x, src_min_y, src_max_y) = bbox_of_ring(&src.exterior.coords);
        let (out_min_x, out_max_x, out_min_y, out_max_y) = bbox_of_polygons(&out);
        let left_expand = src_min_x - out_min_x;
        let right_expand = out_max_x - src_max_x;
        let bottom_expand = src_min_y - out_min_y;
        let top_expand = out_max_y - src_max_y;

        assert!(
            left_expand >= min_expand,
            "{name}: left expansion {} below minimum {}",
            left_expand,
            min_expand
        );
        assert!(
            right_expand >= min_expand,
            "{name}: right expansion {} below minimum {}",
            right_expand,
            min_expand
        );
        assert!(
            bottom_expand >= min_expand,
            "{name}: bottom expansion {} below minimum {}",
            bottom_expand,
            min_expand
        );
        assert!(
            top_expand >= min_expand,
            "{name}: top expansion {} below minimum {}",
            top_expand,
            min_expand
        );

        if contain_source_vertices {
            for c in &src.exterior.coords {
                assert!(
                    point_in_any_polygon(*c, &out),
                    "{name}: source exterior vertex ({}, {}) not inside any buffer component",
                    c.x,
                    c.y
                );
            }
        }

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
