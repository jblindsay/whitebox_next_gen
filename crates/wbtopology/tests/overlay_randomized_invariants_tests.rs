use wbtopology::{
    polygon_difference,
    polygon_intersection,
    polygon_sym_diff,
    polygon_union,
    Coord,
    LinearRing,
    Polygon,
};

#[derive(Clone)]
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        // Numerical Recipes LCG constants.
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }

    fn next_f64(&mut self) -> f64 {
        let v = self.next_u64() >> 11;
        (v as f64) / ((1u64 << 53) as f64)
    }

    fn range_f64(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next_f64()
    }

    fn coin(&mut self, p: f64) -> bool {
        self.next_f64() < p
    }
}

fn ring_area(coords: &[Coord]) -> f64 {
    let mut s = 0.0;
    for i in 0..(coords.len() - 1) {
        s += coords[i].x * coords[i + 1].y - coords[i + 1].x * coords[i].y;
    }
    (0.5 * s).abs()
}

fn poly_area(poly: &Polygon) -> f64 {
    let mut area = ring_area(&poly.exterior.coords);
    for h in &poly.holes {
        area -= ring_area(&h.coords);
    }
    area
}

fn area_sum(polys: &[Polygon]) -> f64 {
    polys.iter().map(poly_area).sum()
}

fn rect_ring(x0: f64, y0: f64, x1: f64, y1: f64) -> LinearRing {
    LinearRing::new(vec![
        Coord::xy(x0, y0),
        Coord::xy(x1, y0),
        Coord::xy(x1, y1),
        Coord::xy(x0, y1),
    ])
}

fn random_rect(rng: &mut Lcg, cx: f64, cy: f64, span: f64) -> (f64, f64, f64, f64) {
    let w = rng.range_f64(0.3, 4.0);
    let h = rng.range_f64(0.3, 4.0);
    let x0 = rng.range_f64(cx - span, cx + span);
    let y0 = rng.range_f64(cy - span, cy + span);
    (x0, y0, x0 + w, y0 + h)
}

fn random_polygon_with_optional_hole(rng: &mut Lcg) -> Polygon {
    let (x0, y0, x1, y1) = random_rect(rng, 0.0, 0.0, 6.0);
    let exterior = rect_ring(x0, y0, x1, y1);

    if !rng.coin(0.35) {
        return Polygon::new(exterior, vec![]);
    }

    // Build a guaranteed-inside hole with margin.
    let margin_x = (x1 - x0) * 0.15;
    let margin_y = (y1 - y0) * 0.15;
    let hx0 = x0 + margin_x;
    let hy0 = y0 + margin_y;
    let hx1 = x1 - margin_x;
    let hy1 = y1 - margin_y;

    if hx1 - hx0 <= 0.1 || hy1 - hy0 <= 0.1 {
        return Polygon::new(exterior, vec![]);
    }

    let hole = rect_ring(
        rng.range_f64(hx0, (hx0 + hx1) * 0.5),
        rng.range_f64(hy0, (hy0 + hy1) * 0.5),
        rng.range_f64((hx0 + hx1) * 0.5, hx1),
        rng.range_f64((hy0 + hy1) * 0.5, hy1),
    );

    Polygon::new(exterior, vec![hole])
}

#[test]
fn overlay_randomized_area_identities_seeded() {
    let mut rng = Lcg::new(0xC0FFEE1234ABCD);
    let eps = 1.0e-9;
    let tol = 1.0e-6;

    for i_case in 0..200usize {
        let a = random_polygon_with_optional_hole(&mut rng);
        let b = random_polygon_with_optional_hole(&mut rng);

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

        assert!(i >= -tol, "case {i_case}: negative intersection area");
        assert!(u >= -tol, "case {i_case}: negative union area");
        assert!(d_ab >= -tol, "case {i_case}: negative A\\B area");
        assert!(d_ba >= -tol, "case {i_case}: negative B\\A area");
        assert!(x >= -tol, "case {i_case}: negative xor area");

        assert!((u - (i + x)).abs() <= tol, "case {i_case}: U != I + XOR");
        assert!(
            (u - (area_a + area_b - i)).abs() <= tol,
            "case {i_case}: U != A + B - I"
        );
        assert!((d_ab - (area_a - i)).abs() <= tol, "case {i_case}: A\\B != A - I");
        assert!((d_ba - (area_b - i)).abs() <= tol, "case {i_case}: B\\A != B - I");
        assert!(
            (x - (d_ab + d_ba)).abs() <= tol,
            "case {i_case}: XOR != (A\\B)+(B\\A)"
        );

        let uni_rev = polygon_union(&b, &a, eps);
        let inter_rev = polygon_intersection(&b, &a, eps);
        let xor_rev = polygon_sym_diff(&b, &a, eps);

        assert!(
            (area_sum(&uni) - area_sum(&uni_rev)).abs() <= tol,
            "case {i_case}: union area order mismatch"
        );
        assert!(
            (area_sum(&inter) - area_sum(&inter_rev)).abs() <= tol,
            "case {i_case}: intersection area order mismatch"
        );
        assert!(
            (area_sum(&xor) - area_sum(&xor_rev)).abs() <= tol,
            "case {i_case}: xor area order mismatch"
        );
    }
}
