use wbraster::{DataType, Raster, RasterConfig};

#[test]
fn scalar_api_contract_smoke_new_get_set_statistics() {
    let mut raster = Raster::new(RasterConfig {
        cols: 4,
        rows: 3,
        bands: 1,
        data_type: DataType::F32,
        nodata: -9999.0,
        ..Default::default()
    });

    raster.set(0, 1, 2, 42.5).expect("set should succeed");
    let v = raster.get(0, 1, 2);
    assert!((v - 42.5).abs() < 1e-6);

    let stats = raster.statistics();
    assert!(stats.valid_count > 0);
}

#[test]
fn scalar_api_contract_smoke_band_view_path() {
    let raster = Raster::new(RasterConfig {
        cols: 2,
        rows: 2,
        bands: 1,
        data_type: DataType::F64,
        nodata: -9999.0,
        ..Default::default()
    });

    let view = raster.band_view(0);
    let flat = raster.band_to_vec_f64(0);

    assert_eq!(flat.len(), 4);
    assert_eq!(view.cols, 2);
    assert_eq!(view.rows, 2);
}

#[test]
fn scalar_api_contract_smoke_datatype_helpers() {
    assert_eq!(DataType::F32.size_bytes(), 4);
    assert_eq!(DataType::F64.size_bytes(), 8);
    assert_eq!(DataType::from_str("float32"), Some(DataType::F32));
    assert_eq!(DataType::from_str("float64"), Some(DataType::F64));
}
