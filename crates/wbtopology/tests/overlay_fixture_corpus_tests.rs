use wbtopology::{
    polygon_difference,
    polygon_intersection,
    polygon_sym_diff,
    polygon_union,
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
    if spec.trim() == "-" {
        return Vec::new();
    }
    spec.split(';').map(parse_ring).collect()
}

fn parse_case(line: &str) -> (&str, f64, Polygon, Polygon, &str) {
    let parts: Vec<&str> = line.split('|').collect();
    assert!(
        parts.len() == 6 || parts.len() == 7,
        "fixture line must have 6 or 7 pipe fields"
    );

    let name = parts[0].trim();
    let eps = parts[1].trim().parse::<f64>().expect("invalid epsilon");

    let a = Polygon::new(parse_ring(parts[2].trim()), parse_holes(parts[3].trim()));
    let b = Polygon::new(parse_ring(parts[4].trim()), parse_holes(parts[5].trim()));
    let mode = if parts.len() == 7 {
        parts[6].trim()
    } else {
        "strict"
    };

    (name, eps, a, b, mode)
}

fn poly_area(poly: &Polygon) -> f64 {
    fn ring_area(coords: &[Coord]) -> f64 {
        if coords.len() < 4 {
            return 0.0;
        }
        // Translation-normalized shoelace reduces catastrophic cancellation
        // for large-magnitude coordinates with small relative offsets.
        let origin = coords[0];
        let mut s = 0.0;
        for i in 0..(coords.len() - 1) {
            let xi = coords[i].x - origin.x;
            let yi = coords[i].y - origin.y;
            let xj = coords[i + 1].x - origin.x;
            let yj = coords[i + 1].y - origin.y;
            s += xi * yj - xj * yi;
        }
        (0.5 * s).abs()
    }

    let mut area = ring_area(&poly.exterior.coords);
    for hole in &poly.holes {
        area -= ring_area(&hole.coords);
    }
    area
}

fn area_sum(polys: &[Polygon]) -> f64 {
    polys.iter().map(poly_area).sum()
}

fn scaled_tol(values: &[f64]) -> f64 {
    let scale = values.iter().fold(1.0_f64, |acc, v| acc.max(v.abs()));
    1.0e-6_f64.max(scale * 1.0e-12)
}

#[test]
fn overlay_fixture_corpus_invariants() {
    let data = include_str!("fixtures/overlay_invariants.txt");

    for raw in data.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (name, eps, a, b, mode) = parse_case(line);

        let inter = polygon_intersection(&a, &b, eps);
        let uni = polygon_union(&a, &b, eps);
        let diff_ab = polygon_difference(&a, &b, eps);
        let diff_ba = polygon_difference(&b, &a, eps);
        let xor = polygon_sym_diff(&a, &b, eps);

        let i = area_sum(&inter);
        let u = area_sum(&uni);
        let d_ab = area_sum(&diff_ab);
        let d_ba = area_sum(&diff_ba);
        let x = area_sum(&xor);
        let area_a = poly_area(&a);
        let area_b = poly_area(&b);

        let tol = scaled_tol(&[i, u, d_ab, d_ba, x, area_a, area_b]);

        assert!(i >= -tol, "{name}: negative intersection area");
        assert!(u >= -tol, "{name}: negative union area");
        assert!(d_ab >= -tol, "{name}: negative difference area");
        assert!(x >= -tol, "{name}: negative symdiff area");

        assert!((u - (i + x)).abs() <= tol, "{name}: U != I + XOR");
        assert!((u - (area_a + area_b - i)).abs() <= tol, "{name}: U != A + B - I");
        assert!((d_ab - (area_a - i)).abs() <= tol, "{name}: A\\B != A - I");
        assert!((x - (d_ab + d_ba)).abs() <= tol, "{name}: XOR != (A\\B)+(B\\A)");

        let uni_rev = polygon_union(&b, &a, eps);
        let inter_rev = polygon_intersection(&b, &a, eps);
        let xor_rev = polygon_sym_diff(&b, &a, eps);
        if mode == "strict" {
            assert_eq!(uni, uni_rev, "{name}: union not deterministic across operand order");
            assert_eq!(inter, inter_rev, "{name}: intersection not deterministic across operand order");
            assert_eq!(xor, xor_rev, "{name}: symdiff not deterministic across operand order");
        } else {
            assert!(
                (area_sum(&uni) - area_sum(&uni_rev)).abs() <= tol,
                "{name}: relaxed union area mismatch across operand order"
            );
            assert!(
                (area_sum(&inter) - area_sum(&inter_rev)).abs() <= tol,
                "{name}: relaxed intersection area mismatch across operand order"
            );
            assert!(
                (area_sum(&xor) - area_sum(&xor_rev)).abs() <= tol,
                "{name}: relaxed symdiff area mismatch across operand order"
            );
        }
    }
}
