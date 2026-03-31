use std::fs;

use wbtopology::{
    contains, crosses, intersects, overlaps, relate, relate_with_epsilon, relate_with_precision, touches,
    within, Coord, Geometry, LineString, LinearRing, Polygon, PrecisionModel,
};

#[derive(Debug)]
struct CaseSpec {
    id: String,
    pattern: String,
    intersects: bool,
    contains: bool,
    within: bool,
    touches: bool,
    crosses: bool,
    overlaps: bool,
    parity_status: String,
}

#[derive(Debug)]
struct PrecisionCaseSpec {
    id: String,
    scale: f64,
    parity_status: String,
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Polygon {
    Polygon::new(
        LinearRing::new(vec![
            Coord::xy(x0, y0),
            Coord::xy(x1, y0),
            Coord::xy(x1, y1),
            Coord::xy(x0, y1),
        ]),
        vec![],
    )
}

fn rect_with_hole(
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    hx0: f64,
    hy0: f64,
    hx1: f64,
    hy1: f64,
) -> Polygon {
    Polygon::new(
        LinearRing::new(vec![
            Coord::xy(x0, y0),
            Coord::xy(x1, y0),
            Coord::xy(x1, y1),
            Coord::xy(x0, y1),
        ]),
        vec![LinearRing::new(vec![
            Coord::xy(hx0, hy0),
            Coord::xy(hx1, hy0),
            Coord::xy(hx1, hy1),
            Coord::xy(hx0, hy1),
        ])],
    )
}

fn parse_parity_status(token: &str) -> String {
    let s = token.trim();
    assert!(
        s == "converge" || s == "known_diff",
        "invalid parity_status '{}': expected converge or known_diff",
        s
    );
    s.to_string()
}

fn parse_bool(token: &str) -> bool {
    matches!(token.trim(), "true" | "True" | "TRUE" | "1" | "yes" | "Yes" | "YES")
}

fn load_specs() -> Vec<CaseSpec> {
    let path = "tests/fixtures/de9im_differential_cases.csv";
    let txt = fs::read_to_string(path).expect("failed to read DE-9IM differential fixture file");
    let mut out = Vec::<CaseSpec>::new();

    for raw in txt.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let cols: Vec<&str> = line.split(',').map(|c| c.trim()).collect();
        assert!(
            cols.len() == 9,
            "invalid DE-9IM fixture row (expected 9 columns): {}",
            line
        );

        out.push(CaseSpec {
            id: cols[0].to_string(),
            pattern: cols[1].to_string(),
            intersects: parse_bool(cols[2]),
            contains: parse_bool(cols[3]),
            within: parse_bool(cols[4]),
            touches: parse_bool(cols[5]),
            crosses: parse_bool(cols[6]),
            overlaps: parse_bool(cols[7]),
            parity_status: parse_parity_status(cols[8]),
        });
    }

    assert!(!out.is_empty(), "DE-9IM fixture file contained no cases");
    out
}

fn load_precision_specs() -> Vec<PrecisionCaseSpec> {
    let path = "tests/fixtures/de9im_precision_differential_cases.csv";
    let txt = fs::read_to_string(path).expect("failed to read DE-9IM precision differential fixture file");
    let mut out = Vec::<PrecisionCaseSpec>::new();

    for raw in txt.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let cols: Vec<&str> = line.split(',').map(|c| c.trim()).collect();
        assert!(
            cols.len() == 3,
            "invalid DE-9IM precision fixture row (expected 3 columns): {}",
            line
        );

        let scale = cols[1]
            .parse::<f64>()
            .expect("invalid numeric scale in DE-9IM precision fixture row");

        out.push(PrecisionCaseSpec {
            id: cols[0].to_string(),
            scale,
            parity_status: parse_parity_status(cols[2]),
        });
    }

    assert!(!out.is_empty(), "DE-9IM precision fixture file contained no cases");
    out
}

fn build_case(id: &str) -> (Geometry, Geometry) {
    match id {
        "point_point_equal" => (
            Geometry::Point(Coord::xy(1.0, 1.0)),
            Geometry::Point(Coord::xy(1.0, 1.0)),
        ),
        "point_point_disjoint" => (
            Geometry::Point(Coord::xy(1.0, 1.0)),
            Geometry::Point(Coord::xy(2.0, 2.0)),
        ),
        "point_on_polygon_boundary" => (
            Geometry::Point(Coord::xy(0.0, 5.0)),
            Geometry::Polygon(rect(0.0, 0.0, 10.0, 10.0)),
        ),
        "point_outside_polygon" => (
            Geometry::Point(Coord::xy(-0.5, 5.0)),
            Geometry::Polygon(rect(0.0, 0.0, 10.0, 10.0)),
        ),
        "polygon_contains_point" => (
            Geometry::Polygon(rect(0.0, 0.0, 10.0, 10.0)),
            Geometry::Point(Coord::xy(5.0, 5.0)),
        ),
        "point_in_line_interior" => (
            Geometry::Point(Coord::xy(1.0, 0.0)),
            Geometry::LineString(LineString::new(vec![Coord::xy(0.0, 0.0), Coord::xy(2.0, 0.0)])),
        ),
        "point_on_line_endpoint" => (
            Geometry::Point(Coord::xy(0.0, 0.0)),
            Geometry::LineString(LineString::new(vec![Coord::xy(0.0, 0.0), Coord::xy(2.0, 0.0)])),
        ),
        "line_line_cross" => (
            Geometry::LineString(LineString::new(vec![Coord::xy(0.0, 0.0), Coord::xy(2.0, 2.0)])),
            Geometry::LineString(LineString::new(vec![Coord::xy(0.0, 2.0), Coord::xy(2.0, 0.0)])),
        ),
        "line_line_touch_endpoint" => (
            Geometry::LineString(LineString::new(vec![Coord::xy(0.0, 0.0), Coord::xy(2.0, 0.0)])),
            Geometry::LineString(LineString::new(vec![Coord::xy(2.0, 0.0), Coord::xy(2.0, 2.0)])),
        ),
        "polygon_contains_polygon" => (
            Geometry::Polygon(rect(0.0, 0.0, 10.0, 10.0)),
            Geometry::Polygon(rect(2.0, 2.0, 8.0, 8.0)),
        ),
        "polygon_within_polygon" => (
            Geometry::Polygon(rect(2.0, 2.0, 8.0, 8.0)),
            Geometry::Polygon(rect(0.0, 0.0, 10.0, 10.0)),
        ),
        "polygon_polygon_touch_edge" => (
            Geometry::Polygon(rect(0.0, 0.0, 2.0, 2.0)),
            Geometry::Polygon(rect(2.0, 0.0, 4.0, 2.0)),
        ),
        "point_in_polygon_hole" => (
            Geometry::Point(Coord::xy(5.0, 5.0)),
            Geometry::Polygon(rect_with_hole(0.0, 0.0, 10.0, 10.0, 3.0, 3.0, 7.0, 7.0)),
        ),
        "point_on_hole_boundary" => (
            Geometry::Point(Coord::xy(3.0, 5.0)),
            Geometry::Polygon(rect_with_hole(0.0, 0.0, 10.0, 10.0, 3.0, 3.0, 7.0, 7.0)),
        ),
        "line_crosses_polygon_with_hole" => (
            Geometry::LineString(LineString::new(vec![Coord::xy(-1.0, 5.0), Coord::xy(11.0, 5.0)])),
            Geometry::Polygon(rect_with_hole(0.0, 0.0, 10.0, 10.0, 3.0, 3.0, 7.0, 7.0)),
        ),
        "line_crosses_polygon" => (
            Geometry::LineString(LineString::new(vec![
                Coord::xy(-1.0, 5.0),
                Coord::xy(5.0, 5.0),
                Coord::xy(11.0, 5.0),
            ])),
            Geometry::Polygon(rect(0.0, 0.0, 10.0, 10.0)),
        ),
        "line_within_polygon" => (
            Geometry::LineString(LineString::new(vec![Coord::xy(1.0, 1.0), Coord::xy(9.0, 9.0)])),
            Geometry::Polygon(rect(0.0, 0.0, 10.0, 10.0)),
        ),
        "polygon_polygon_overlap" => (
            Geometry::Polygon(rect(0.0, 0.0, 3.0, 3.0)),
            Geometry::Polygon(rect(2.0, 1.0, 5.0, 4.0)),
        ),
        "near_points_snap_equal" => (
            Geometry::Point(Coord::xy(1.00041, 2.00041)),
            Geometry::Point(Coord::xy(1.00049, 2.00049)),
        ),
        "near_point_line_endpoint_snap" => (
            Geometry::Point(Coord::xy(2.00041, 0.00041)),
            Geometry::LineString(LineString::new(vec![Coord::xy(0.0, 0.0), Coord::xy(2.00049, 0.00049)])),
        ),
        "near_polygon_boundary_snap" => (
            Geometry::Point(Coord::xy(10.00041, 5.0)),
            Geometry::Polygon(rect(0.0, 0.0, 10.00049, 10.0)),
        ),
        "near_line_line_touch_snap" => (
            Geometry::LineString(LineString::new(vec![Coord::xy(0.0, 0.0), Coord::xy(2.00049, 0.0)])),
            Geometry::LineString(LineString::new(vec![Coord::xy(2.00041, 0.00041), Coord::xy(2.00041, 2.0)])),
        ),
        "near_line_polygon_cross_snap" => (
            Geometry::LineString(LineString::new(vec![
                Coord::xy(-1.0, 5.00041),
                Coord::xy(5.0, 5.00049),
                Coord::xy(11.0, 5.00041),
            ])),
            Geometry::Polygon(rect(0.0, 0.0, 10.0, 10.0)),
        ),
        _ => panic!("unknown DE-9IM fixture case id: {}", id),
    }
}

#[test]
fn de9im_differential_fixture_harness() {
    let specs = load_specs();

    for spec in specs {
        let (a, b) = build_case(&spec.id);
        let m = relate(&a, &b);

        assert!(
            m.matches(&spec.pattern),
            "case {}: pattern mismatch: matrix={}, pattern={}",
            spec.id,
            m.as_str9(),
            spec.pattern
        );

        // Differential check: matrix-derived predicates should agree with direct APIs
        // for these core fixture cases.
        assert_eq!(intersects(&a, &b), m.is_intersects(), "case {} intersects mismatch", spec.id);
        assert_eq!(touches(&a, &b), m.is_touches(), "case {} touches mismatch", spec.id);

        assert_eq!(intersects(&a, &b), spec.intersects, "case {} intersects fixture mismatch", spec.id);
        assert_eq!(contains(&a, &b), spec.contains, "case {} contains fixture mismatch", spec.id);
        assert_eq!(within(&a, &b), spec.within, "case {} within fixture mismatch", spec.id);
        assert_eq!(touches(&a, &b), spec.touches, "case {} touches fixture mismatch", spec.id);
        assert_eq!(crosses(&a, &b), spec.crosses, "case {} crosses fixture mismatch", spec.id);
        assert_eq!(overlaps(&a, &b), spec.overlaps, "case {} overlaps fixture mismatch", spec.id);

        assert!(
            spec.parity_status == "converge" || spec.parity_status == "known_diff",
            "case {} has invalid parity status {}",
            spec.id,
            spec.parity_status
        );
    }
}

#[test]
fn de9im_precision_differential_fixture_harness() {
    let specs = load_precision_specs();

    for spec in specs {
        let (a, b) = build_case(&spec.id);
        let pm = PrecisionModel::Fixed { scale: spec.scale };

        let by_precision = relate_with_precision(&a, &b, pm);

        let sa = pm.apply_geometry(&a);
        let sb = pm.apply_geometry(&b);
        let by_manual = relate_with_epsilon(&sa, &sb, pm.epsilon());

        assert_eq!(
            by_precision.as_str9(),
            by_manual.as_str9(),
            "precision differential case {} mismatch",
            spec.id
        );

        assert!(
            spec.parity_status == "converge" || spec.parity_status == "known_diff",
            "precision case {} has invalid parity status {}",
            spec.id,
            spec.parity_status
        );
    }
}
