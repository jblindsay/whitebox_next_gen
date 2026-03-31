//! Cross-tool reprojection parity tests against GDAL/PROJ fixture outputs.

use wbraster::{Raster, ReprojectOptions, ResampleMethod};

const FIXTURE_COLS: usize = 14;
const FIXTURE_ROWS: usize = 12;
const FIXTURE_EXTENT_X_MIN: f64 = -222_638.981_586_547;
const FIXTURE_EXTENT_Y_MIN: f64 = -222_684.208_505_544;
const FIXTURE_EXTENT_X_MAX: f64 = 0.0;
const FIXTURE_EXTENT_Y_MAX: f64 = 0.0;

fn fixture_dir() -> &'static str {
    "tests/fixtures/reprojection_parity"
}

fn fixture_path(name: &str) -> String {
    format!("{}/{}", fixture_dir(), name)
}

fn parity_verbose_enabled() -> bool {
    std::env::var("WB_PARITY_VERBOSE")
        .map(|v| {
            let s = v.trim().to_ascii_lowercase();
            s == "1" || s == "true" || s == "yes" || s == "on"
        })
        .unwrap_or(false)
}

fn assert_raster_close(
    actual: &Raster,
    expected: &Raster,
    case_name: &str,
    mean_tol: f64,
    mae_tol: f64,
    rmse_tol: f64,
    max_err_tol: f64,
) {
    assert_eq!(actual.cols, expected.cols, "{case_name}: cols mismatch");
    assert_eq!(actual.rows, expected.rows, "{case_name}: rows mismatch");
    assert_eq!(actual.bands, expected.bands, "{case_name}: bands mismatch");
    assert_eq!(actual.crs.epsg, expected.crs.epsg, "{case_name}: epsg mismatch");

    let a_stats = actual.statistics();
    let e_stats = expected.statistics();
    assert!(a_stats.valid_count > 0, "{case_name}: actual raster contains no valid cells");
    assert!(e_stats.valid_count > 0, "{case_name}: expected raster contains no valid cells");
    assert!(
        (a_stats.mean - e_stats.mean).abs() <= mean_tol,
        "{case_name}: mean mismatch too high (actual={}, expected={}, tol={})",
        a_stats.mean,
        e_stats.mean,
        mean_tol
    );

    let mut overlap_count = 0usize;
    let mut abs_err_sum = 0.0_f64;
    let mut sq_err_sum = 0.0_f64;
    let mut max_err = 0.0_f64;
    for band in 0..actual.bands as isize {
        for row in 0..actual.rows as isize {
            for col in 0..actual.cols as isize {
                let av = actual.get_raw(band, row, col).unwrap();
                let ev = expected.get_raw(band, row, col).unwrap();
                if actual.is_nodata(av) || expected.is_nodata(ev) {
                    continue;
                }
                let err = (av - ev).abs();
                overlap_count += 1;
                abs_err_sum += err;
                sq_err_sum += err * err;
                max_err = max_err.max(err);
            }
        }
    }

    assert!(overlap_count > 0, "{case_name}: no overlapping valid cells found");
    let mae = abs_err_sum / overlap_count as f64;
    let rmse = (sq_err_sum / overlap_count as f64).sqrt();

    assert!(
        mae <= mae_tol,
        "{case_name}: MAE too high (overlap_count={}, mae={}, tol={}, rmse={}, max_err={})",
        overlap_count,
        mae,
        mae_tol,
        rmse,
        max_err
    );
    assert!(
        rmse <= rmse_tol,
        "{case_name}: RMSE too high (overlap_count={}, rmse={}, tol={}, mae={}, max_err={})",
        overlap_count,
        rmse,
        rmse_tol,
        mae,
        max_err
    );
    assert!(
        max_err <= max_err_tol,
        "{case_name}: Max error too high (overlap_count={}, max_err={}, tol={}, mae={}, rmse={})",
        overlap_count,
        max_err,
        max_err_tol,
        mae,
        rmse
    );

    if parity_verbose_enabled() {
        eprintln!(
            "[parity] case={} overlap_count={} mean_actual={} mean_expected={} mae={} rmse={} max_err={}",
            case_name,
            overlap_count,
            a_stats.mean,
            e_stats.mean,
            mae,
            rmse,
            max_err
        );
    }
}

#[test]
fn parity_core_resamplers_match_gdal_fixtures() {
    let src = Raster::read(fixture_path("src_epsg4326_small.tif"))
        .expect("missing source parity fixture");

    let cases = [
        (ResampleMethod::Nearest, "expected_epsg3857_near.tif", 0.8, 1.1, 1.3, 3.0),
        (ResampleMethod::Bilinear, "expected_epsg3857_bilinear.tif", 0.8, 1.1, 1.3, 3.0),
        (ResampleMethod::Cubic, "expected_epsg3857_cubic.tif", 1.2, 1.5, 1.7, 4.0),
        (ResampleMethod::Lanczos, "expected_epsg3857_lanczos.tif", 1.2, 1.5, 1.7, 4.0),
    ];

    for (method, fixture_name, mean_tol, mae_tol, rmse_tol, max_err_tol) in cases {
        let expected = Raster::read(fixture_path(fixture_name))
            .unwrap_or_else(|e| panic!("missing expected parity fixture {fixture_name}: {e}"));
        let opts = ReprojectOptions::new(3857, method)
            .with_size(FIXTURE_COLS, FIXTURE_ROWS)
            .with_extent(wbraster::Extent {
                x_min: FIXTURE_EXTENT_X_MIN,
                y_min: FIXTURE_EXTENT_Y_MIN,
                x_max: FIXTURE_EXTENT_X_MAX,
                y_max: FIXTURE_EXTENT_Y_MAX,
            });
        let out = src
            .reproject_with_options(&opts)
            .unwrap_or_else(|e| panic!("reprojection failed for {fixture_name}: {e}"));

        assert!((out.x_min - FIXTURE_EXTENT_X_MIN).abs() <= 1e-9);
        assert!((out.y_min - FIXTURE_EXTENT_Y_MIN).abs() <= 1e-9);
        assert!((out.x_max() - FIXTURE_EXTENT_X_MAX).abs() <= 1e-9);
        assert!((out.y_max() - FIXTURE_EXTENT_Y_MAX).abs() <= 1e-9);

        assert_raster_close(
            &out,
            &expected,
            fixture_name,
            mean_tol,
            mae_tol,
            rmse_tol,
            max_err_tol,
        );
    }
}

#[test]
fn parity_harness_reports_missing_fixtures_cleanly() {
    let missing = Raster::read("tests/fixtures/reprojection_parity/__missing__.tif");
    assert!(missing.is_err());
}
