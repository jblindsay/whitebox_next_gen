use wbtopology::{
    Geometry,
    GeometryFixMode,
    GeometryFixOptions,
    from_wkt,
    is_valid_polygon,
    make_valid_geometry,
};

fn count_polygon_parts(g: &Geometry) -> usize {
    match g {
        Geometry::Polygon(p) => {
            if p.exterior.coords.len() >= 4 {
                1
            } else {
                0
            }
        }
        Geometry::MultiPolygon(polys) => polys
            .iter()
            .filter(|p| p.exterior.coords.len() >= 4)
            .count(),
        Geometry::GeometryCollection(geoms) => geoms.iter().map(count_polygon_parts).sum(),
        _ => 0,
    }
}

fn all_polygon_parts_valid(g: &Geometry) -> bool {
    match g {
        Geometry::Polygon(p) => is_valid_polygon(p),
        Geometry::MultiPolygon(polys) => polys.iter().all(is_valid_polygon),
        Geometry::GeometryCollection(geoms) => geoms.iter().all(all_polygon_parts_valid),
        _ => true,
    }
}

#[test]
fn make_valid_geometry_fixture_harness() {
    let data = include_str!("fixtures/make_valid_geometry_cases.txt");
    for line in data.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let cols: Vec<&str> = trimmed.split('|').collect();
        assert_eq!(
            cols.len(),
            6,
            "expected 6 pipe-delimited columns for line: {trimmed}"
        );

        let name = cols[0].trim();
        let input = from_wkt(cols[1].trim()).expect("invalid input WKT");
        let epsilon: f64 = cols[2].trim().parse().expect("invalid epsilon");
        let structure_min_parts: usize = cols[3]
            .trim()
            .parse()
            .expect("invalid structure_min_parts");
        let linework_min_parts: usize = cols[4]
            .trim()
            .parse()
            .expect("invalid linework_min_parts");
        let expect_polygon_output: bool = cols[5]
            .trim()
            .parse()
            .expect("invalid expect_polygon_output");

        let structure_out = make_valid_geometry(
            &input,
            GeometryFixOptions {
                epsilon,
                mode: GeometryFixMode::StructureFirst,
                keep_collapsed: false,
            },
        );
        let linework_out = make_valid_geometry(
            &input,
            GeometryFixOptions {
                epsilon,
                mode: GeometryFixMode::LineworkFirst,
                keep_collapsed: false,
            },
        );

        let structure_parts = count_polygon_parts(&structure_out);
        let linework_parts = count_polygon_parts(&linework_out);

        assert!(
            structure_parts >= structure_min_parts,
            "{name}: structure parts {} below expected minimum {}",
            structure_parts,
            structure_min_parts
        );
        assert!(
            linework_parts >= linework_min_parts,
            "{name}: linework parts {} below expected minimum {}",
            linework_parts,
            linework_min_parts
        );

        assert!(
            all_polygon_parts_valid(&structure_out),
            "{name}: structure output has invalid polygon part"
        );
        assert!(
            all_polygon_parts_valid(&linework_out),
            "{name}: linework output has invalid polygon part"
        );

        if expect_polygon_output {
            assert!(
                structure_parts > 0 && linework_parts > 0,
                "{name}: expected polygonal output in both modes"
            );
        }
    }
}
