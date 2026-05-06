use wbtopology::{
    Geometry,
    UnaryDissolveOptions,
    UnaryDissolveStrategy,
    from_wkt,
    polygon_unary_dissolve_with_options,
};

fn parse_polygon_wkt(text: &str) -> wbtopology::Polygon {
    match from_wkt(text).expect("failed to parse WKT") {
        Geometry::Polygon(p) => p,
        other => panic!("expected polygon WKT, got {:?}", other),
    }
}

fn parse_expected_memberships(text: &str) -> Vec<Vec<usize>> {
    let mut groups = Vec::<Vec<usize>>::new();
    for grp in text.split(',') {
        let grp = grp.trim();
        if grp.is_empty() {
            continue;
        }
        let mut members = Vec::<usize>::new();
        for idx in grp.split('+') {
            members.push(idx.trim().parse::<usize>().expect("invalid membership index"));
        }
        members.sort_unstable();
        members.dedup();
        groups.push(members);
    }
    groups.sort();
    groups
}

#[test]
fn unary_dissolve_graph_fixture_harness() {
    let data = include_str!("fixtures/unary_dissolve_graph_cases.txt");
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
        let epsilon: f64 = cols[1].trim().parse().expect("invalid epsilon");
        let a = parse_polygon_wkt(cols[2].trim());
        let b = parse_polygon_wkt(cols[3].trim());
        let expected_groups: usize = cols[4]
            .trim()
            .parse()
            .expect("invalid expected group count");
        let expect_merged: bool = cols[5].trim().parse().expect("invalid merged flag");
        let expected_memberships = parse_expected_memberships(cols[6].trim());

        let out = polygon_unary_dissolve_with_options(
            &[a, b],
            UnaryDissolveOptions {
                epsilon,
                strategy: UnaryDissolveStrategy::GraphDriven,
                ..UnaryDissolveOptions::default()
            },
        );

        assert_eq!(
            out.len(), expected_groups,
            "{name}: group count mismatch (expected {expected_groups}, got {})",
            out.len()
        );

        let merged = out.iter().any(|g| g.source_indices.len() == 2);
        assert_eq!(
            merged, expect_merged,
            "{name}: merged expectation mismatch (expected {expect_merged}, got {merged})"
        );

        // Membership correctness gate.
        let mut memberships: Vec<Vec<usize>> = out
            .iter()
            .map(|g| {
                let mut m = g.source_indices.clone();
                m.sort_unstable();
                m.dedup();
                m
            })
            .collect();
        memberships.sort();

        assert_eq!(
            memberships,
            expected_memberships,
            "{name}: expected memberships {:?}, got {:?}",
            expected_memberships,
            memberships
        );
    }
}
