use wbtopology::{
    concave_hull,
    concave_hull_with_options,
    convex_hull,
    geometry_area,
    ConcaveHullOptions,
    Coord,
    Geometry,
};

fn parse_coords(spec: &str) -> Vec<Coord> {
    spec.trim()
        .split(',')
        .map(|pair| {
            let xy: Vec<f64> = pair
                .split(':')
                .map(|v| v.parse::<f64>().expect("invalid coordinate"))
                .collect();
            assert_eq!(xy.len(), 2, "coord must be x:y");
            Coord::xy(xy[0], xy[1])
        })
        .collect()
}

fn kind(g: &Geometry) -> &'static str {
    match g {
        Geometry::Point(_) => "point",
        Geometry::LineString(_) => "line",
        Geometry::Polygon(_) | Geometry::MultiPolygon(_) => "polygon",
        Geometry::GeometryCollection(v) if v.is_empty() => "empty",
        _ => "other",
    }
}

#[test]
fn hull_fixture_invariants() {
    let data = include_str!("fixtures/hull_invariants.txt");

    for raw in data.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split('|').collect();
        assert_eq!(parts.len(), 8, "fixture line must have 8 fields: {line}");

        let name = parts[0].trim();
        let mode = parts[1].trim();
        let threshold = parts[2].trim().parse::<f64>().expect("invalid threshold");
        let coords = parse_coords(parts[3]);
        let min_kind = parts[4].trim();
        let min_concave_area = parts[5].trim().parse::<f64>().expect("invalid min area");
        let min_convex_area = parts[6].trim().parse::<f64>().expect("invalid convex area");
        let allow_disjoint = parts[7].trim().parse::<bool>().expect("invalid allow_disjoint");

        let convex = convex_hull(&coords, 1.0e-12);
        let concave = match mode {
            "absolute" => concave_hull_with_options(
                &coords,
                ConcaveHullOptions {
                    max_edge_length: threshold,
                    epsilon: 1.0e-12,
                    allow_disjoint,
                    ..Default::default()
                },
            ),
            "relative" => concave_hull_with_options(
                &coords,
                ConcaveHullOptions {
                    relative_edge_length_ratio: Some(threshold),
                    epsilon: 1.0e-12,
                    allow_disjoint,
                    ..Default::default()
                },
            ),
            _ => panic!("unknown mode {mode}"),
        };

        let convex_area = geometry_area(&convex);
        let concave_area = geometry_area(&concave);

        assert_eq!(kind(&convex), min_kind, "{name}: convex kind mismatch");
        assert_eq!(kind(&concave), min_kind, "{name}: concave kind mismatch");
        assert!(convex_area >= min_convex_area, "{name}: convex area too small");
        assert!(concave_area >= min_concave_area, "{name}: concave area too small");
        assert!(concave_area <= convex_area + 1.0e-9, "{name}: concave area exceeds convex area");

        if min_kind == "polygon" {
            let absolute = concave_hull(&coords, threshold.max(0.1), 1.0e-12);
            assert!(geometry_area(&absolute) >= 0.0, "{name}: absolute concave invalid area");
        }
    }
}
