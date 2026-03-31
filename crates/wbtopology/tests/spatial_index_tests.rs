use wbtopology::{
    Coord,
    Envelope,
    Geometry,
    LineString,
    LinearRing,
    Polygon,
    SpatialIndex,
};

fn square(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Geometry {
    Geometry::Polygon(Polygon::new(
        LinearRing::new(vec![
            Coord::xy(min_x, min_y),
            Coord::xy(max_x, min_y),
            Coord::xy(max_x, max_y),
            Coord::xy(min_x, max_y),
        ]),
        vec![],
    ))
}

#[test]
fn envelope_query_returns_expected_ids() {
    let geoms = vec![
        square(0.0, 0.0, 2.0, 2.0),
        square(10.0, 10.0, 12.0, 12.0),
        Geometry::LineString(LineString::new(vec![
            Coord::xy(1.0, 5.0),
            Coord::xy(4.0, 5.0),
        ])),
    ];

    let idx = SpatialIndex::from_geometries(&geoms);
    let hits = idx.query_point(Coord::xy(1.5, 1.5));

    assert_eq!(hits.len(), 1);
    let first = idx.get(hits[0]).expect("id should resolve");
    assert_eq!(first.geometry, geoms[0]);
}

#[test]
fn geometry_query_filters_by_envelope_overlap() {
    let geoms = vec![
        square(0.0, 0.0, 2.0, 2.0),
        square(5.0, 5.0, 6.0, 6.0),
        square(20.0, 20.0, 25.0, 25.0),
    ];
    let idx = SpatialIndex::from_geometries(&geoms);

    let probe = square(1.0, 1.0, 5.5, 5.5);
    let hits = idx.query_geometry(&probe);

    assert_eq!(hits.len(), 2);
}

#[test]
fn nearest_neighbor_returns_exact_closest() {
    let geoms = vec![
        Geometry::Point(Coord::xy(0.0, 0.0)),
        Geometry::Point(Coord::xy(10.0, 0.0)),
        Geometry::Point(Coord::xy(3.0, 4.0)),
    ];
    let idx = SpatialIndex::from_geometries(&geoms);

    let target = Geometry::Point(Coord::xy(4.0, 4.0));
    let (id, d) = idx.nearest_neighbor(&target).expect("nearest should exist");

    assert_eq!(id, 2);
    assert!((d - 1.0).abs() < 1.0e-12);
}

#[test]
fn packed_str_build_creates_hierarchy_and_queries_correctly() {
    let geoms = (0..36)
        .map(|i| {
            let x = (i % 6) as f64 * 10.0;
            let y = (i / 6) as f64 * 10.0;
            Geometry::Point(Coord::xy(x, y))
        })
        .collect::<Vec<_>>();

    let idx = SpatialIndex::build_str(&geoms, 4);
    let hits = idx.query_envelope(Envelope::new(14.0, 14.0, 36.0, 36.0));

    assert_eq!(idx.node_capacity(), 4);
    assert!(idx.depth() > 1);
    assert_eq!(hits.len(), 4);
}

#[test]
fn insert_rebuilds_packed_tree() {
    let mut idx = SpatialIndex::with_node_capacity(3);
    for i in 0..10 {
        let inserted = idx.insert(Geometry::Point(Coord::xy(i as f64, i as f64)));
        assert_eq!(inserted, Some(i));
    }

    assert!(idx.depth() > 1);

    let (id, d) = idx
        .nearest_neighbor(&Geometry::Point(Coord::xy(8.2, 8.1)))
        .expect("nearest should exist");
    assert_eq!(id, 8);
    assert!(d < 0.25);
}

#[test]
fn nearest_k_returns_top_k_in_ascending_order() {
    // Place 5 points with known distances from origin.
    let pts = vec![
        Coord::xy(3.0, 4.0),  // id 0, dist=5
        Coord::xy(1.0, 0.0),  // id 1, dist=1
        Coord::xy(0.0, 2.0),  // id 2, dist=2
        Coord::xy(0.0, 10.0), // id 3, dist=10
        Coord::xy(0.0, 3.0),  // id 4, dist=3
    ];
    let geoms: Vec<Geometry> = pts.iter().map(|&p| Geometry::Point(p)).collect();
    let idx = SpatialIndex::from_geometries(&geoms);

    let target = Geometry::Point(Coord::xy(0.0, 0.0));
    let top3 = idx.nearest_k(&target, 3);

    assert_eq!(top3.len(), 3);
    // Must be sorted ascending by distance.
    assert!(top3[0].1 <= top3[1].1);
    assert!(top3[1].1 <= top3[2].1);

    // The three closest points are id 1 (d=1), id 2 (d=2), id 4 (d=3).
    let ids: Vec<usize> = top3.iter().map(|&(id, _)| id).collect();
    assert!(ids.contains(&1));
    assert!(ids.contains(&2));
    assert!(ids.contains(&4));

    assert!((top3[0].1 - 1.0).abs() < 1.0e-12);
    assert!((top3[1].1 - 2.0).abs() < 1.0e-12);
    assert!((top3[2].1 - 3.0).abs() < 1.0e-12);
}

#[test]
fn nearest_k_with_k_larger_than_index_returns_all() {
    let geoms = vec![
        Geometry::Point(Coord::xy(0.0, 0.0)),
        Geometry::Point(Coord::xy(1.0, 0.0)),
    ];
    let idx = SpatialIndex::from_geometries(&geoms);
    let target = Geometry::Point(Coord::xy(0.5, 0.0));
    let all = idx.nearest_k(&target, 100);
    assert_eq!(all.len(), 2);
    // Equidistant: both should appear regardless of order.
    let ids: Vec<usize> = all.iter().map(|&(id, _)| id).collect();
    assert!(ids.contains(&0));
    assert!(ids.contains(&1));
}

#[test]
fn remove_entry_becomes_invisible_to_queries_and_iterators() {
    let geoms = vec![
        square(0.0, 0.0, 2.0, 2.0),   // id 0
        square(10.0, 10.0, 12.0, 12.0), // id 1
        square(20.0, 20.0, 22.0, 22.0), // id 2
    ];
    let mut idx = SpatialIndex::from_geometries(&geoms);
    assert_eq!(idx.len(), 3);

    // Remove id 0
    let removed = idx.remove(0);
    assert!(removed);
    assert_eq!(idx.len(), 2);

    // Envelope query over the old id-0 region returns nothing.
    let hits = idx.query_envelope(Envelope::new(0.0, 0.0, 2.0, 2.0));
    assert!(hits.is_empty());

    // all_entries iterator also does not include the removed entry.
    let ids: Vec<usize> = idx.all_entries().map(|e| e.id).collect();
    assert_eq!(ids.len(), 2);
    assert!(!ids.contains(&0));

    // Removing the same id again returns false.
    assert!(!idx.remove(0));

    // Remaining entries are still queryable.
    let hits2 = idx.query_envelope(Envelope::new(10.0, 10.0, 12.0, 12.0));
    assert_eq!(hits2.len(), 1);
}

#[test]
fn remove_then_nearest_neighbor_skips_removed_entry() {
    let pts = vec![
        Coord::xy(0.0, 0.0),  // id 0 — closest to origin
        Coord::xy(5.0, 0.0),  // id 1
        Coord::xy(10.0, 0.0), // id 2
    ];
    let geoms: Vec<Geometry> = pts.iter().map(|&p| Geometry::Point(p)).collect();
    let mut idx = SpatialIndex::from_geometries(&geoms);

    let target = Geometry::Point(Coord::xy(0.0, 0.0));
    let (id, _) = idx.nearest_neighbor(&target).unwrap();
    assert_eq!(id, 0);

    // After removing id 0, the nearest should be id 1.
    idx.remove(0);
    let (id2, d2) = idx.nearest_neighbor(&target).unwrap();
    assert_eq!(id2, 1);
    assert!((d2 - 5.0).abs() < 1.0e-12);
}

#[test]
fn remove_preserves_surviving_ids_until_compact() {
    let geoms = vec![
        Geometry::Point(Coord::xy(0.0, 0.0)), // id 0
        Geometry::Point(Coord::xy(1.0, 0.0)), // id 1
        Geometry::Point(Coord::xy(2.0, 0.0)), // id 2
        Geometry::Point(Coord::xy(3.0, 0.0)), // id 3
    ];
    let mut idx = SpatialIndex::from_geometries(&geoms);

    assert!(idx.remove(1));

    // `remove` tombstones only: surviving IDs are unchanged.
    let ids: Vec<usize> = idx.all_entries().map(|e| e.id).collect();
    assert_eq!(ids, vec![0, 2, 3]);

    // ID 2 should still resolve to the same geometry.
    let e2 = idx.get(2).expect("id 2 should still be present");
    match &e2.geometry {
        Geometry::Point(c) => assert_eq!(*c, Coord::xy(2.0, 0.0)),
        _ => panic!("unexpected geometry type"),
    }
}

#[test]
fn compact_reassigns_ids_densely_after_removals() {
    let geoms = vec![
        Geometry::Point(Coord::xy(0.0, 0.0)), // id 0
        Geometry::Point(Coord::xy(1.0, 0.0)), // id 1
        Geometry::Point(Coord::xy(2.0, 0.0)), // id 2
        Geometry::Point(Coord::xy(3.0, 0.0)), // id 3
    ];
    let mut idx = SpatialIndex::from_geometries(&geoms);

    assert!(idx.remove(1));
    assert!(idx.remove(3));
    assert_eq!(idx.len(), 2);

    idx.compact();

    // After compact, IDs are dense from 0..len-1.
    let ids: Vec<usize> = idx.all_entries().map(|e| e.id).collect();
    assert_eq!(ids, vec![0, 1]);

    // The two remaining geometries are the original x=0 and x=2 points.
    let got: Vec<f64> = idx
        .all_entries()
        .map(|e| match &e.geometry {
            Geometry::Point(c) => c.x,
            _ => panic!("unexpected geometry type"),
        })
        .collect();
    assert_eq!(got, vec![0.0, 2.0]);
}
