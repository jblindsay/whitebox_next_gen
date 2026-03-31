use wbtopology::{vector_io, Coord, Geometry, LineString, LinearRing, Polygon};

#[test]
fn layer_conversion_roundtrip() {
    let geoms = vec![
        Geometry::Point(Coord::xyz(1.0, 2.0, 3.0)),
        Geometry::LineString(LineString::new(vec![Coord::xy(0.0, 0.0), Coord::xy(3.0, 1.0)])),
        Geometry::Polygon(Polygon::new(
            LinearRing::new(vec![
                Coord::xy(0.0, 0.0),
                Coord::xy(2.0, 0.0),
                Coord::xy(2.0, 2.0),
                Coord::xy(0.0, 2.0),
            ]),
            vec![],
        )),
    ];

    let layer = vector_io::layer_from_geometries("test", &geoms, Some(4326)).unwrap();
    assert_eq!(layer.features.len(), 3);

    let back = vector_io::geometries_from_layer(&layer).unwrap();
    assert_eq!(back.len(), 3);
    assert_eq!(back[0], geoms[0]);
}
