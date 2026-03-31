use wbtopology::algorithms::point_in_ring::{classify_point_in_ring_eps, PointInRing};
use wbtopology::{
    voronoi_diagram,
    voronoi_diagram_with_clip,
    voronoi_diagram_with_clip_with_precision,
    voronoi_diagram_with_options,
    voronoi_diagram_with_precision,
    Coord,
    Envelope,
    PrecisionModel,
    VoronoiOptions,
};

fn poly_area_abs(coords: &[Coord]) -> f64 {
    if coords.len() < 4 {
        return 0.0;
    }
    let mut s = 0.0;
    for i in 0..(coords.len() - 1) {
        let a = coords[i];
        let b = coords[i + 1];
        s += a.x * b.y - b.x * a.y;
    }
    0.5 * s.abs()
}

#[test]
fn voronoi_square_sites_contain_own_sites() {
    let pts = vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(1.0, 0.0),
        Coord::xy(1.0, 1.0),
        Coord::xy(0.0, 1.0),
    ];
    let clip = Envelope::new(-1.0, -1.0, 2.0, 2.0);
    let vd = voronoi_diagram_with_clip(&pts, 1.0e-9, clip);

    assert_eq!(vd.sites.len(), 4);
    assert_eq!(vd.cells.len(), 4);

    for i in 0..vd.sites.len() {
        let site = vd.sites[i];
        let ring = &vd.cells[i].exterior.coords;
        let cls = classify_point_in_ring_eps(site, ring, 1.0e-7);
        assert!(matches!(cls, PointInRing::Inside | PointInRing::Boundary));
    }
}

#[test]
fn voronoi_cells_partition_clip_area() {
    let pts = vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(2.0, 0.3),
        Coord::xy(1.2, 1.8),
        Coord::xy(0.3, 1.1),
        Coord::xy(1.8, 1.2),
    ];
    let clip = Envelope::new(-0.5, -0.5, 2.5, 2.5);
    let clip_area = (clip.max_x - clip.min_x) * (clip.max_y - clip.min_y);

    let vd = voronoi_diagram_with_clip(&pts, 1.0e-9, clip);
    assert_eq!(vd.cells.len(), vd.sites.len());

    let mut area_sum = 0.0;
    for cell in &vd.cells {
        area_sum += poly_area_abs(&cell.exterior.coords);
    }

    assert!((area_sum - clip_area).abs() <= 1.0e-6);
}

#[test]
fn voronoi_single_point_is_clip_rect() {
    let pts = vec![Coord::xy(5.0, 6.0)];
    let clip = Envelope::new(0.0, 0.0, 10.0, 8.0);
    let vd = voronoi_diagram_with_clip(&pts, 1.0e-9, clip);

    assert_eq!(vd.cells.len(), 1);
    let area = poly_area_abs(&vd.cells[0].exterior.coords);
    assert!((area - 80.0).abs() <= 1.0e-9);
}

#[test]
fn voronoi_with_precision_matches_manual_presnap() {
    let pts = vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(1.0, 0.0),
        Coord::xy(1.0, 1.0),
        Coord::xy(0.0, 1.0),
        Coord::xy(1.00041, 0.99961),
    ];
    let pm = PrecisionModel::Fixed { scale: 1000.0 };

    let by_wrapper = voronoi_diagram_with_precision(&pts, pm);

    let mut snapped = pts.clone();
    pm.apply_coords_in_place(&mut snapped);
    let by_manual = voronoi_diagram(&snapped, pm.epsilon());

    assert_eq!(by_wrapper, by_manual);
}

#[test]
fn voronoi_with_clip_with_precision_matches_manual_presnap() {
    let pts = vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(2.0, 0.3),
        Coord::xy(1.2, 1.8),
        Coord::xy(0.3, 1.1),
        Coord::xy(1.8, 1.2),
    ];
    let clip = Envelope::new(-0.5, -0.5, 2.5, 2.5);
    let pm = PrecisionModel::Fixed { scale: 1000.0 };

    let by_wrapper = voronoi_diagram_with_clip_with_precision(&pts, pm, clip);

    let mut snapped = pts.clone();
    pm.apply_coords_in_place(&mut snapped);
    let by_manual = voronoi_diagram_with_clip(&snapped, pm.epsilon(), clip);

    assert_eq!(by_wrapper, by_manual);
}

#[test]
fn voronoi_with_options_matches_precision_clip_path() {
    let pts = vec![
        Coord::xy(0.0, 0.0),
        Coord::xy(2.0, 0.3),
        Coord::xy(1.2, 1.8),
        Coord::xy(0.3, 1.1),
        Coord::xy(1.8, 1.2),
    ];
    let clip = Envelope::new(-0.5, -0.5, 2.5, 2.5);
    let pm = PrecisionModel::Fixed { scale: 1000.0 };

    let by_precision = voronoi_diagram_with_clip_with_precision(&pts, pm, clip);
    let by_options = voronoi_diagram_with_options(
        &pts,
        VoronoiOptions {
            epsilon: 1.0e-9,
            precision: Some(pm),
            clip: Some(clip),
        },
    );

    assert_eq!(by_options, by_precision);
}
