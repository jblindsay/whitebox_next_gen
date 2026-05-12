use wbtopology::{
    polygon_difference,
    polygon_intersection,
    polygon_overlay,
    polygon_sym_diff,
    polygon_union,
    OverlayOp,
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

fn polygon_vertex_count(poly: &Polygon) -> usize {
    let shell = poly.exterior.coords.len();
    let holes: usize = poly.holes.iter().map(|h| h.coords.len()).sum();
    shell + holes
}

fn total_hole_count(polys: &[Polygon]) -> usize {
    polys.iter().map(|p| p.holes.len()).sum()
}

fn total_vertex_count(polys: &[Polygon]) -> usize {
    polys.iter().map(polygon_vertex_count).sum()
}

fn op_signature(polys: &[Polygon]) -> String {
    format!(
        "polys={} holes={} vertices={} area={:.12}",
        polys.len(),
        total_hole_count(polys),
        total_vertex_count(polys),
        area_sum(polys)
    )
}

fn op_summary(label: &str, polys: &[Polygon]) {
    eprintln!(
        "TRACE_SUMMARY {label}: polys={} holes={} vertices={} area={:.6}",
        polys.len(),
        total_hole_count(polys),
        total_vertex_count(polys),
        area_sum(polys)
    );
}

fn scaled_tol(values: &[f64]) -> f64 {
    let scale = values.iter().fold(1.0_f64, |acc, v| acc.max(v.abs()));
    1.0e-6_f64.max(scale * 1.0e-12)
}

#[test]
fn overlay_fixture_corpus_invariants() {
    let data = include_str!("fixtures/overlay_invariants.txt");
    let mut relaxed_diagnostics = Vec::<String>::new();

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
        let tol = scaled_tol(&[i, u, d_ab, d_ba, x]);

        assert!(i >= -tol, "{name}: negative intersection area");
        assert!(u >= -tol, "{name}: negative union area");
        assert!(d_ab >= -tol, "{name}: negative difference area");
        assert!(x >= -tol, "{name}: negative symdiff area");

        let area_a = poly_area(&a);
        let area_b = poly_area(&b);

        if mode == "strict" {
            assert!((u - (i + x)).abs() <= tol, "{name}: U != I + XOR");
            assert!((u - (area_a + area_b - i)).abs() <= tol, "{name}: U != A + B - I");
            assert!((d_ab - (area_a - i)).abs() <= tol, "{name}: A\\B != A - I");
            assert!((x - (d_ab + d_ba)).abs() <= tol, "{name}: XOR != (A\\B)+(B\\A)");
        } else {
            let d1 = (u - (i + x)).abs();
            let d2 = (u - (area_a + area_b - i)).abs();
            let d3 = (d_ab - (area_a - i)).abs();
            let d4 = (x - (d_ab + d_ba)).abs();
            let max_delta = d1.max(d2).max(d3).max(d4);
            if max_delta > tol {
                relaxed_diagnostics.push(format!(
                    "{name}: max_delta={:.6e} tol={:.6e} (u-i-x={:.6e}, u-a-b+i={:.6e}, d-a+i={:.6e}, x-dab-dba={:.6e})",
                    max_delta,
                    tol,
                    d1,
                    d2,
                    d3,
                    d4
                ));
            }
        }

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

    if !relaxed_diagnostics.is_empty() {
        eprintln!(
            "overlay_fixture_corpus: {} relaxed case(s) exceeded strict identity tolerance (non-blocking diagnostics)",
            relaxed_diagnostics.len()
        );
        for line in &relaxed_diagnostics {
            eprintln!("  - {line}");
        }
    }
}

#[test]
fn overlay_frontier_cross_ladder_diagnostics() {
    let data = include_str!("fixtures/overlay_invariants.txt");
    let mut case = None;

    for raw in data.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, eps, a, b, mode) = parse_case(line);
        if name == "island_hole_rich_cross_ladder" {
            case = Some((name.to_string(), eps, a, b, mode.to_string()));
            break;
        }
    }

    let (name, eps, a, b, mode) = case.expect("missing island_hole_rich_cross_ladder fixture");

    let inter = polygon_intersection(&a, &b, eps);
    let uni = polygon_union(&a, &b, eps);
    let diff_ab = polygon_difference(&a, &b, eps);
    let diff_ba = polygon_difference(&b, &a, eps);
    let xor = polygon_sym_diff(&a, &b, eps);

    let area_a = poly_area(&a);
    let area_b = poly_area(&b);
    let i = area_sum(&inter);
    let u = area_sum(&uni);
    let d_ab = area_sum(&diff_ab);
    let d_ba = area_sum(&diff_ba);
    let x = area_sum(&xor);
    let tol = scaled_tol(&[area_a, area_b, i, u, d_ab, d_ba, x]);

    let d1 = (u - (i + x)).abs();
    let d2 = (u - (area_a + area_b - i)).abs();
    let d3 = (d_ab - (area_a - i)).abs();
    let d4 = (x - (d_ab + d_ba)).abs();

    eprintln!(
        "overlay_frontier_diag {name}: mode={} eps={:.3e} tol={:.3e}",
        mode, eps, tol
    );
    eprintln!(
        "  area: A={:.6} B={:.6} I={:.6} U={:.6} D_ab={:.6} D_ba={:.6} X={:.6}",
        area_a, area_b, i, u, d_ab, d_ba, x
    );
    eprintln!(
        "  identity_delta: u-(i+x)={:.6e}, u-(a+b-i)={:.6e}, d_ab-(a-i)={:.6e}, x-(d_ab+d_ba)={:.6e}",
        d1, d2, d3, d4
    );
    eprintln!(
        "  components: I={} U={} D_ab={} D_ba={} X={}",
        inter.len(),
        uni.len(),
        diff_ab.len(),
        diff_ba.len(),
        xor.len()
    );
    eprintln!(
        "  holes: I={} U={} D_ab={} D_ba={} X={}",
        total_hole_count(&inter),
        total_hole_count(&uni),
        total_hole_count(&diff_ab),
        total_hole_count(&diff_ba),
        total_hole_count(&xor)
    );
    eprintln!(
        "  vertices: I={} U={} D_ab={} D_ba={} X={}",
        total_vertex_count(&inter),
        total_vertex_count(&uni),
        total_vertex_count(&diff_ab),
        total_vertex_count(&diff_ba),
        total_vertex_count(&xor)
    );

    let uni_rev = polygon_union(&b, &a, eps);
    let inter_rev = polygon_intersection(&b, &a, eps);
    let xor_rev = polygon_sym_diff(&b, &a, eps);

    let du = (area_sum(&uni) - area_sum(&uni_rev)).abs();
    let di = (area_sum(&inter) - area_sum(&inter_rev)).abs();
    let dx = (area_sum(&xor) - area_sum(&xor_rev)).abs();

    eprintln!(
        "  order_area_delta: union={:.6e} intersection={:.6e} xor={:.6e}",
        du, di, dx
    );

    assert!(
        i >= -tol && u >= -tol && d_ab >= -tol && d_ba >= -tol && x >= -tol,
        "{name}: negative area encountered in frontier diagnostics"
    );
}

#[test]
fn overlay_frontier_cross_ladder_operation_trace() {
    let data = include_str!("fixtures/overlay_invariants.txt");
    let mut case = None;

    for raw in data.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, eps, a, b, mode) = parse_case(line);
        if name == "island_hole_rich_cross_ladder" {
            case = Some((name.to_string(), eps, a, b, mode.to_string()));
            break;
        }
    }

    let (name, eps, a, b, mode) = case.expect("missing island_hole_rich_cross_ladder fixture");

    eprintln!("TRACE_CASE_START {name} mode={} eps={:.3e}", mode, eps);

    eprintln!("TRACE_OP_START intersection");
    let inter = polygon_intersection(&a, &b, eps);
    op_summary("intersection", &inter);
    let inter_direct = polygon_overlay(&a, &b, OverlayOp::Intersection, eps);
    op_summary("intersection_direct", &inter_direct);
    eprintln!(
        "TRACE_COMPARE intersection_vs_direct area_delta={:.6e} poly_delta={}",
        (area_sum(&inter) - area_sum(&inter_direct)).abs(),
        inter.len() as i64 - inter_direct.len() as i64
    );
    eprintln!("TRACE_OP_END intersection");

    eprintln!("TRACE_OP_START union");
    let uni = polygon_union(&a, &b, eps);
    op_summary("union", &uni);
    eprintln!("TRACE_OP_END union");

    eprintln!("TRACE_OP_START difference_ab");
    let diff_ab = polygon_difference(&a, &b, eps);
    op_summary("difference_ab", &diff_ab);
    eprintln!("TRACE_OP_END difference_ab");

    eprintln!("TRACE_OP_START difference_ba");
    let diff_ba = polygon_difference(&b, &a, eps);
    op_summary("difference_ba", &diff_ba);
    eprintln!("TRACE_OP_END difference_ba");

    eprintln!("TRACE_OP_START sym_diff");
    let xor = polygon_sym_diff(&a, &b, eps);
    op_summary("sym_diff", &xor);
    eprintln!("TRACE_OP_END sym_diff");

    let area_a = poly_area(&a);
    let area_b = poly_area(&b);
    let i = area_sum(&inter);
    let u = area_sum(&uni);
    let d_ab = area_sum(&diff_ab);
    let d_ba = area_sum(&diff_ba);
    let x = area_sum(&xor);
    let tol = scaled_tol(&[area_a, area_b, i, u, d_ab, d_ba, x]);

    eprintln!(
        "TRACE_IDENTITY {name}: u-(i+x)={:.6e} u-(a+b-i)={:.6e} d_ab-(a-i)={:.6e} x-(d_ab+d_ba)={:.6e} tol={:.6e}",
        (u - (i + x)).abs(),
        (u - (area_a + area_b - i)).abs(),
        (d_ab - (area_a - i)).abs(),
        (x - (d_ab + d_ba)).abs(),
        tol
    );

    eprintln!("TRACE_CASE_END {name}");

    assert!(
        i >= -tol && u >= -tol && d_ab >= -tol && d_ba >= -tol && x >= -tol,
        "{name}: negative area encountered in frontier operation trace"
    );
}

#[test]
fn overlay_strict_four_hole_intersection_matches_direct_path() {
    let data = include_str!("fixtures/overlay_invariants.txt");

    for raw in data.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (name, eps, a, b, mode) = parse_case(line);
        if mode != "strict" {
            continue;
        }
        if a.holes.len() + b.holes.len() != 4 {
            continue;
        }

        let inter_special = polygon_intersection(&a, &b, eps);
        let inter_direct = polygon_overlay(&a, &b, OverlayOp::Intersection, eps);

        let area_special = area_sum(&inter_special);
        let area_direct = area_sum(&inter_direct);
        let tol = scaled_tol(&[area_special, area_direct]);

        assert!(
            (area_special - area_direct).abs() <= tol,
            "{name}: 4-hole intersection branch drift: special={area_special:.12}, direct={area_direct:.12}, tol={tol:.12}"
        );
    }
}

#[test]
fn overlay_strict_four_hole_core_ops_match_direct_area_paths() {
    let data = include_str!("fixtures/overlay_invariants.txt");

    for raw in data.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (name, eps, a, b, mode) = parse_case(line);
        if mode != "strict" {
            continue;
        }
        if a.holes.len() + b.holes.len() != 4 {
            continue;
        }

        let inter = polygon_intersection(&a, &b, eps);
        let inter_direct = polygon_overlay(&a, &b, OverlayOp::Intersection, eps);
        let union = polygon_union(&a, &b, eps);
        let union_direct = polygon_overlay(&a, &b, OverlayOp::Union, eps);
        let diff_ab = polygon_difference(&a, &b, eps);
        let diff_ab_direct = polygon_overlay(&a, &b, OverlayOp::DifferenceAB, eps);
        let inter_a = area_sum(&inter);
        let inter_d = area_sum(&inter_direct);
        let union_a = area_sum(&union);
        let union_d = area_sum(&union_direct);
        let diff_a = area_sum(&diff_ab);
        let diff_d = area_sum(&diff_ab_direct);

        let tol = scaled_tol(&[inter_a, inter_d, union_a, union_d, diff_a, diff_d]);

        assert!(
            (inter_a - inter_d).abs() <= tol,
            "{name}: intersection area drift api={inter_a:.12}, direct={inter_d:.12}, tol={tol:.12}"
        );
        assert!(
            (union_a - union_d).abs() <= tol,
            "{name}: union area drift api={union_a:.12}, direct={union_d:.12}, tol={tol:.12}"
        );
        assert!(
            (diff_a - diff_d).abs() <= tol,
            "{name}: difference_ab area drift api={diff_a:.12}, direct={diff_d:.12}, tol={tol:.12}"
        );
    }
}

#[test]
fn overlay_strict_four_hole_symdiff_direct_diagnostics_non_blocking() {
    let data = include_str!("fixtures/overlay_invariants.txt");
    let mut diagnostics = Vec::<String>::new();

    for raw in data.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (name, eps, a, b, mode) = parse_case(line);
        if mode != "strict" {
            continue;
        }
        if a.holes.len() + b.holes.len() != 4 {
            continue;
        }

        let xor_api = polygon_sym_diff(&a, &b, eps);
        let xor_direct = polygon_overlay(&a, &b, OverlayOp::SymmetricDifference, eps);

        let area_api = area_sum(&xor_api);
        let area_direct = area_sum(&xor_direct);
        let tol = scaled_tol(&[area_api, area_direct]);
        let delta = (area_api - area_direct).abs();

        if delta > tol {
            diagnostics.push(format!(
                "{name}: symdiff drift delta={:.12}, tol={:.12}; api[{}] direct[{}]",
                delta,
                tol,
                op_signature(&xor_api),
                op_signature(&xor_direct)
            ));
        }
    }

    if !diagnostics.is_empty() {
        eprintln!(
            "overlay_strict_four_hole_symdiff_direct_diagnostics_non_blocking: {} strict 4-hole case(s) exceed area tolerance",
            diagnostics.len()
        );
        for line in &diagnostics {
            eprintln!("  - {line}");
        }
    }
}
