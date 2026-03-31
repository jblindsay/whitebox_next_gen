use wbtopology::{Coord, Geometry, LineString, LinearRing, Polygon, PrecisionModel};

#[test]
fn fixed_precision_snaps_scalars_and_coords() {
    let pm = PrecisionModel::Fixed { scale: 10.0 };
    assert_eq!(pm.apply_scalar(1.24), 1.2);
    assert_eq!(pm.apply_scalar(1.25), 1.3);

    let c = Coord::xy(10.234, -4.995);
    let s = pm.apply_coord(c);
    assert_eq!(s, Coord::xy(10.2, -5.0));
}

#[test]
fn fixed_precision_equality_respects_grid() {
    let pm = PrecisionModel::Fixed { scale: 100.0 };
    let a = Coord::xy(1.2344, 9.8764);
    let b = Coord::xy(1.2345, 9.8765);
    assert!(pm.eq_coord(a, b));
}

#[test]
fn fixed_precision_applies_across_geometry_types() {
    let pm = PrecisionModel::Fixed { scale: 10.0 };

    let ls = LineString::new(vec![Coord::xy(0.04, 0.06), Coord::xy(1.24, 1.26)]);
    let snapped_ls = pm.apply_linestring(&ls);
    assert_eq!(snapped_ls.coords[0], Coord::xy(0.0, 0.1));
    assert_eq!(snapped_ls.coords[1], Coord::xy(1.2, 1.3));

    let poly = Polygon::new(
        LinearRing::new(vec![
            Coord::xy(0.04, 0.04),
            Coord::xy(1.24, 0.04),
            Coord::xy(1.24, 1.24),
            Coord::xy(0.04, 1.24),
        ]),
        vec![LinearRing::new(vec![
            Coord::xy(0.44, 0.44),
            Coord::xy(0.86, 0.44),
            Coord::xy(0.86, 0.86),
            Coord::xy(0.44, 0.86),
        ])],
    );

    let g = Geometry::Polygon(poly.clone());
    let snapped_g = pm.apply_geometry(&g);
    let Geometry::Polygon(sp) = snapped_g else {
        panic!("expected snapped polygon");
    };

    assert_eq!(sp.exterior.coords[0], Coord::xy(0.0, 0.0));
    assert_eq!(sp.exterior.coords[1], Coord::xy(1.2, 0.0));
    assert_eq!(sp.holes.len(), 1);
    assert_eq!(sp.holes[0].coords[0], Coord::xy(0.4, 0.4));
}

#[test]
fn fixed_precision_preserves_z_values() {
    let pm = PrecisionModel::Fixed { scale: 10.0 };
    let c = Coord::xyz(10.234, -4.995, 42.0);
    let s = pm.apply_coord(c);
    assert_eq!(s, Coord::xyz(10.2, -5.0, 42.0));
}
