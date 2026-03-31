//! Integration tests for wbprojection.

use crate::{
    Crs, CrsTransformPolicy, Datum, Ellipsoid, GridShiftGrid, GridShiftSample, Projection, ProjectionKind,
    ProjectionParams, get_grid, has_grid, register_grid, register_ntv2_gsb_hierarchy,
    resolve_ntv2_hierarchy_grid_name, resolve_ntv2_hierarchy_subgrid, unregister_grid,
};
use crate::datum::DatumTransform;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

const TOL_DEGREES: f64 = 1e-8;    // ~1 mm at equator

fn round_trip(proj: &Projection, lon: f64, lat: f64) {
    let (x, y) = proj.forward(lon, lat).expect("forward failed");
    let (lon2, lat2) = proj.inverse(x, y).expect("inverse failed");
    assert!(
        (lon2 - lon).abs() < TOL_DEGREES,
        "lon round-trip failed: {} → {} → {} (Δ={})",
        lon, x, lon2, (lon2 - lon).abs()
    );
    assert!(
        (lat2 - lat).abs() < TOL_DEGREES,
        "lat round-trip failed: {} → {} → {} (Δ={})",
        lat, y, lat2, (lat2 - lat).abs()
    );
}

// ─── UTM ───────────────────────────────────────────────────────────────────

#[test]
fn utm_zone32_forward_known() {
    // Stuttgart, Germany: 9.18°E, 48.78°N → approx UTM 32N 513223.539, 5403015.518
    let proj = Projection::new(ProjectionParams::utm(32, false)).unwrap();
    let (x, y) = proj.forward(9.18, 48.78).unwrap();
    // println!("Stuttgart: e={x:.2}, n={y:.2} → lon=9.18, lat=48.78");
    assert!((x - 513_223.539).abs() < 1.0, "easting off: {x}");
    assert!((y - 5_403_015.518).abs() < 1.0, "northing off: {y}");
}

#[test]
fn utm_round_trip() {
    let proj = Projection::new(ProjectionParams::utm(32, false)).unwrap();
    round_trip(&proj, 9.0, 48.0);
    round_trip(&proj, 10.5, 51.2);
}

#[test]
fn utm_southern_hemisphere() {
    let proj = Projection::new(ProjectionParams::utm(34, true)).unwrap();
    round_trip(&proj, 18.0, -34.0);
}

// ─── Web Mercator ───────────────────────────────────────────────────────────

#[test]
fn web_mercator_round_trip() {
    let proj = Projection::new(ProjectionParams::web_mercator()).unwrap();
    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 13.4, 52.5);    // Berlin
    round_trip(&proj, -74.0, 40.7);   // New York
    round_trip(&proj, 139.7, 35.7);   // Tokyo
}

#[test]
fn web_mercator_equator() {
    let proj = Projection::new(ProjectionParams::web_mercator()).unwrap();
    let (x, y) = proj.forward(0.0, 0.0).unwrap();
    assert!(x.abs() < 1e-6, "x should be 0 at origin: {x}");
    assert!(y.abs() < 1e-6, "y should be 0 at origin: {y}");
}

#[test]
fn web_mercator_out_of_bounds() {
    let proj = Projection::new(ProjectionParams::web_mercator()).unwrap();
    assert!(proj.forward(0.0, 86.0).is_err());
}

#[test]
fn crs_is_geographic_reports_expected_kind() {
    assert!(Crs::from_epsg(4326).unwrap().is_geographic());
    assert!(!Crs::from_epsg(3857).unwrap().is_geographic());
}

#[test]
fn crs_is_projected_reports_expected_kind() {
    assert!(!Crs::from_epsg(4326).unwrap().is_projected());
    assert!(Crs::from_epsg(3857).unwrap().is_projected());
    assert!(Crs::from_epsg(32632).unwrap().is_projected());
}

// ─── Mercator ───────────────────────────────────────────────────────────────

#[test]
fn mercator_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::Mercator)).unwrap();
    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 45.0, 30.0);
    round_trip(&proj, -120.0, -45.0);
}

// ─── Lambert Conformal Conic ─────────────────────────────────────────────

#[test]
fn lcc_two_standard_parallels_round_trip() {
    let proj = Projection::new(
        ProjectionParams::new(ProjectionKind::LambertConformalConic {
            lat1: 33.0,
            lat2: Some(45.0),
        })
        .with_lat0(39.0)
        .with_lon0(-96.0),
    )
    .unwrap();
    round_trip(&proj, -96.0, 39.0);
    round_trip(&proj, -100.0, 42.0);
    round_trip(&proj, -80.0, 35.0);
}

#[test]
fn lcc_one_standard_parallel() {
    let proj = Projection::new(
        ProjectionParams::new(ProjectionKind::LambertConformalConic {
            lat1: 40.0,
            lat2: None,
        })
        .with_lon0(-100.0),
    )
    .unwrap();
    round_trip(&proj, -90.0, 45.0);
}

// ─── Albers Equal-Area Conic ────────────────────────────────────────────

#[test]
fn albers_round_trip() {
    let proj = Projection::new(
        ProjectionParams::new(ProjectionKind::AlbersEqualAreaConic {
            lat1: 29.5,
            lat2: 45.5,
        })
        .with_lat0(37.5)
        .with_lon0(-96.0),
    )
    .unwrap();
    round_trip(&proj, -96.0, 37.5);
    round_trip(&proj, -110.0, 40.0);
}

// ─── Azimuthal Equidistant ──────────────────────────────────────────────

#[test]
fn azimuthal_equidistant_round_trip() {
    let proj = Projection::new(
        ProjectionParams::new(ProjectionKind::AzimuthalEquidistant)
            .with_lat0(90.0)
            .with_lon0(0.0),
    )
    .unwrap();
    round_trip(&proj, 0.0, 60.0);
    round_trip(&proj, 45.0, 70.0);
}

// ─── Stereographic ──────────────────────────────────────────────────────

#[test]
fn stereographic_round_trip() {
    let proj = Projection::new(
        ProjectionParams::new(ProjectionKind::Stereographic)
            .with_lat0(90.0),
    )
    .unwrap();
    round_trip(&proj, 0.0, 75.0);
    round_trip(&proj, 90.0, 80.0);
}

// ─── Orthographic ───────────────────────────────────────────────────────

#[test]
fn orthographic_round_trip() {
    let proj = Projection::new(
        ProjectionParams::new(ProjectionKind::Orthographic)
            .with_lat0(45.0)
            .with_lon0(10.0),
    )
    .unwrap();
    round_trip(&proj, 10.0, 45.0);
    round_trip(&proj, 20.0, 50.0);
}

#[test]
fn orthographic_far_side_error() {
    let proj = Projection::new(
        ProjectionParams::new(ProjectionKind::Orthographic)
            .with_lat0(0.0)
            .with_lon0(0.0),
    )
    .unwrap();
    assert!(proj.forward(180.0, 0.0).is_err());
}

// ─── Sinusoidal ─────────────────────────────────────────────────────────

#[test]
fn sinusoidal_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::Sinusoidal)).unwrap();
    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 30.0, 45.0);
    round_trip(&proj, -60.0, -30.0);
}

// ─── Mollweide ──────────────────────────────────────────────────────────

#[test]
fn mollweide_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::Mollweide)).unwrap();
    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 45.0, 30.0);
    round_trip(&proj, -90.0, -60.0);
}

// ─── Equirectangular ────────────────────────────────────────────────────

#[test]
fn equirectangular_round_trip() {
    let proj = Projection::new(
        ProjectionParams::new(ProjectionKind::Equirectangular { lat_ts: 0.0 }),
    )
    .unwrap();
    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 100.0, -20.0);
}

// ─── Robinson ───────────────────────────────────────────────────────────

#[test]
fn robinson_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::Robinson)).unwrap();
    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 60.0, 30.0);
    round_trip(&proj, -120.0, -45.0);
}

// ─── Gnomonic ───────────────────────────────────────────────────────────

#[test]
fn gnomonic_round_trip() {
    let proj = Projection::new(
        ProjectionParams::new(ProjectionKind::Gnomonic)
            .with_lat0(45.0)
            .with_lon0(-10.0),
    )
    .unwrap();

    round_trip(&proj, -10.0, 45.0);
    round_trip(&proj, 0.0, 40.0);
    round_trip(&proj, -20.0, 50.0);
}

#[test]
fn gnomonic_far_side_error() {
    let proj = Projection::new(
        ProjectionParams::new(ProjectionKind::Gnomonic)
            .with_lat0(0.0)
            .with_lon0(0.0),
    )
    .unwrap();

    assert!(proj.forward(180.0, 0.0).is_err());
}

// ─── Aitoff ─────────────────────────────────────────────────────────────

#[test]
fn aitoff_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::Aitoff)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 30.0, 20.0);
    round_trip(&proj, -75.0, -35.0);
    round_trip(&proj, 140.0, 50.0);
}

// ─── Van der Grinten ─────────────────────────────────────────────────────

#[test]
fn van_der_grinten_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::VanDerGrinten)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 20.0, 10.0);
    round_trip(&proj, -60.0, 30.0);
    round_trip(&proj, 110.0, -25.0);
}

// ─── Winkel Tripel ───────────────────────────────────────────────────────

#[test]
fn winkel_tripel_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::WinkelTripel)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 25.0, 15.0);
    round_trip(&proj, -70.0, -20.0);
    round_trip(&proj, 130.0, 45.0);
}

// ─── Hammer ─────────────────────────────────────────────────────────────

#[test]
fn hammer_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::Hammer)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 40.0, 20.0);
    round_trip(&proj, -90.0, -35.0);
    round_trip(&proj, 160.0, 55.0);
}

// ─── Eckert IV ──────────────────────────────────────────────────────────

#[test]
fn eckert_iv_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::EckertIV)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 35.0, 18.0);
    round_trip(&proj, -80.0, -28.0);
    round_trip(&proj, 150.0, 48.0);
}

// ─── Eckert I ───────────────────────────────────────────────────────────

#[test]
fn eckert_i_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::EckertI)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 35.0, 18.0);
    round_trip(&proj, -80.0, -28.0);
    round_trip(&proj, 150.0, 48.0);
}

// ─── Eckert II ──────────────────────────────────────────────────────────

#[test]
fn eckert_ii_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::EckertII)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 32.0, 16.0);
    round_trip(&proj, -78.0, -26.0);
    round_trip(&proj, 145.0, 46.0);
}

// ─── Eckert III ─────────────────────────────────────────────────────────

#[test]
fn eckert_iii_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::EckertIII)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 34.0, 19.0);
    round_trip(&proj, -82.0, -29.0);
    round_trip(&proj, 152.0, 49.0);
}

// ─── Eckert V ───────────────────────────────────────────────────────────

#[test]
fn eckert_v_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::EckertV)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 33.0, 17.0);
    round_trip(&proj, -79.0, -27.0);
    round_trip(&proj, 148.0, 47.0);
}

// ─── Miller Cylindrical ────────────────────────────────────────────────

#[test]
fn miller_cylindrical_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::MillerCylindrical)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 40.0, 25.0);
    round_trip(&proj, -85.0, -30.0);
    round_trip(&proj, 160.0, 50.0);
}

// ─── Gall Stereographic ────────────────────────────────────────────────

#[test]
fn gall_stereographic_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::GallStereographic)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 35.0, 20.0);
    round_trip(&proj, -75.0, -25.0);
    round_trip(&proj, 145.0, 48.0);
}

// ─── Gall-Peters ───────────────────────────────────────────────────────

#[test]
fn gall_peters_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::GallPeters)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 30.0, 22.0);
    round_trip(&proj, -80.0, -30.0);
    round_trip(&proj, 150.0, 55.0);
}

// ─── Behrmann ──────────────────────────────────────────────────────────

#[test]
fn behrmann_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::Behrmann)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 25.0, 18.0);
    round_trip(&proj, -70.0, -27.0);
    round_trip(&proj, 130.0, 46.0);
}

// ─── Hobo-Dyer ─────────────────────────────────────────────────────────

#[test]
fn hobo_dyer_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::HoboDyer)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 28.0, 20.0);
    round_trip(&proj, -78.0, -28.0);
    round_trip(&proj, 142.0, 49.0);
}

// ─── Natural Earth ─────────────────────────────────────────────────────

#[test]
fn natural_earth_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::NaturalEarth)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 35.0, 20.0);
    round_trip(&proj, -90.0, -30.0);
    round_trip(&proj, 150.0, 50.0);
}

// ─── Wagner VI ─────────────────────────────────────────────────────────

#[test]
fn wagner_vi_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::WagnerVI)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 30.0, 25.0);
    round_trip(&proj, -85.0, -32.0);
    round_trip(&proj, 145.0, 48.0);
}

// ─── Eckert VI ─────────────────────────────────────────────────────────

#[test]
fn eckert_vi_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::EckertVI)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 35.0, 20.0);
    round_trip(&proj, -88.0, -30.0);
    round_trip(&proj, 155.0, 50.0);
}

// ─── Polyconic ─────────────────────────────────────────────────────────

#[test]
fn polyconic_round_trip() {
    let proj = Projection::new(
        ProjectionParams::new(ProjectionKind::Polyconic)
            .with_lat0(0.0)
            .with_lon0(0.0),
    )
    .unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 12.0, 18.0);
    round_trip(&proj, -18.0, -14.0);
    round_trip(&proj, 26.0, 22.0);
}

// ─── Bonne ─────────────────────────────────────────────────────────────

#[test]
fn bonne_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::Bonne)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 25.0, 20.0);
    round_trip(&proj, -70.0, -25.0);
    round_trip(&proj, 135.0, 45.0);
}

// ─── Craster ───────────────────────────────────────────────────────────

#[test]
fn craster_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::Craster)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 30.0, 20.0);
    round_trip(&proj, -85.0, -30.0);
    round_trip(&proj, 145.0, 50.0);
}

// ─── Putnins P4' ───────────────────────────────────────────────────────

#[test]
fn putnins_p4p_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::PutninsP4p)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 25.0, 18.0);
    round_trip(&proj, -75.0, -28.0);
    round_trip(&proj, 135.0, 48.0);
}

// ─── Fahey ─────────────────────────────────────────────────────────────

#[test]
fn fahey_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::Fahey)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 20.0, 15.0);
    round_trip(&proj, -65.0, -25.0);
    round_trip(&proj, 125.0, 45.0);
}

// ─── Times ─────────────────────────────────────────────────────────────

#[test]
fn times_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::Times)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 30.0, 20.0);
    round_trip(&proj, -80.0, -30.0);
    round_trip(&proj, 150.0, 50.0);
}

// ─── Patterson ─────────────────────────────────────────────────────────

#[test]
fn patterson_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::Patterson)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 35.0, 20.0);
    round_trip(&proj, -90.0, -30.0);
    round_trip(&proj, 155.0, 50.0);
}

// ─── Putnins P3 ────────────────────────────────────────────────────────

#[test]
fn putnins_p3_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::PutninsP3)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 30.0, 20.0);
    round_trip(&proj, -75.0, -25.0);
    round_trip(&proj, 140.0, 45.0);
}

// ─── Putnins P3' ───────────────────────────────────────────────────────

#[test]
fn putnins_p3p_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::PutninsP3p)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 32.0, 18.0);
    round_trip(&proj, -80.0, -28.0);
    round_trip(&proj, 145.0, 48.0);
}

// ─── Putnins P5 ────────────────────────────────────────────────────────

#[test]
fn putnins_p5_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::PutninsP5)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 28.0, 20.0);
    round_trip(&proj, -78.0, -30.0);
    round_trip(&proj, 150.0, 50.0);
}

// ─── Putnins P5' ───────────────────────────────────────────────────────

#[test]
fn putnins_p5p_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::PutninsP5p)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 26.0, 18.0);
    round_trip(&proj, -76.0, -26.0);
    round_trip(&proj, 142.0, 46.0);
}

// ─── Werenskiold I ─────────────────────────────────────────────────────

#[test]
fn werenskiold_i_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::WerenskioldI)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 25.0, 18.0);
    round_trip(&proj, -70.0, -27.0);
    round_trip(&proj, 130.0, 46.0);
}

// ─── Collignon ──────────────────────────────────────────────────────────

#[test]
fn collignon_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::Collignon)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 30.0, 20.0);
    round_trip(&proj, -80.0, -30.0);
    round_trip(&proj, 150.0, 50.0);
}

// ─── Wagner II ──────────────────────────────────────────────────────────

#[test]
fn wagner_i_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::WagnerI)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 25.0, 16.0);
    round_trip(&proj, -72.0, -24.0);
    round_trip(&proj, 136.0, 42.0);
}

// ─── Wagner II ──────────────────────────────────────────────────────────

#[test]
fn wagner_ii_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::WagnerII)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 28.0, 17.0);
    round_trip(&proj, -76.0, -26.0);
    round_trip(&proj, 142.0, 46.0);
}

// ─── Wagner III ─────────────────────────────────────────────────────────

#[test]
fn wagner_iii_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::WagnerIII)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 30.0, 19.0);
    round_trip(&proj, -79.0, -27.0);
    round_trip(&proj, 146.0, 45.0);
}

// ─── Wagner IV ──────────────────────────────────────────────────────────

#[test]
fn wagner_iv_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::WagnerIV)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 30.0, 20.0);
    round_trip(&proj, -80.0, -28.0);
    round_trip(&proj, 148.0, 47.0);
}

// ─── Wagner V ───────────────────────────────────────────────────────────

#[test]
fn wagner_v_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::WagnerV)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 32.0, 21.0);
    round_trip(&proj, -82.0, -29.0);
    round_trip(&proj, 152.0, 49.0);
}

// ─── Putnins P1 ─────────────────────────────────────────────────────────

#[test]
fn putnins_p1_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::PutninsP1)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 26.0, 16.0);
    round_trip(&proj, -74.0, -24.0);
    round_trip(&proj, 138.0, 44.0);
}

// ─── Putnins P2 ─────────────────────────────────────────────────────────

#[test]
fn putnins_p2_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::PutninsP2)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 24.0, 15.0);
    round_trip(&proj, -68.0, -23.0);
    round_trip(&proj, 132.0, 42.0);
}

// ─── Putnins P6 ─────────────────────────────────────────────────────────

#[test]
fn putnins_p6_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::PutninsP6)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 23.0, 14.0);
    round_trip(&proj, -66.0, -22.0);
    round_trip(&proj, 126.0, 40.0);
}

// ─── Putnins P6' ────────────────────────────────────────────────────────

#[test]
fn putnins_p6p_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::PutninsP6p)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 22.0, 13.0);
    round_trip(&proj, -64.0, -21.0);
    round_trip(&proj, 122.0, 39.0);
}

// ─── Quartic Authalic ───────────────────────────────────────────────────

#[test]
fn quartic_authalic_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::QuarticAuthalic)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 24.0, 15.0);
    round_trip(&proj, -68.0, -22.0);
    round_trip(&proj, 128.0, 40.0);
}

// ─── Foucaut ────────────────────────────────────────────────────────────

#[test]
fn foucaut_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::Foucaut)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 22.0, 14.0);
    round_trip(&proj, -64.0, -20.0);
    round_trip(&proj, 122.0, 38.0);
}

// ─── Loximuthal ─────────────────────────────────────────────────────────

#[test]
fn loximuthal_round_trip() {
    let proj = Projection::new(
        ProjectionParams::new(ProjectionKind::Loximuthal { lat1: 40.0 })
            .with_lon0(0.0),
    )
    .unwrap();

    round_trip(&proj, 0.0, 40.0);
    round_trip(&proj, 20.0, 45.0);
    round_trip(&proj, -60.0, 10.0);
    round_trip(&proj, 110.0, -20.0);
}

// ─── Nell ───────────────────────────────────────────────────────────────

#[test]
fn nell_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::Nell)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 24.0, 16.0);
    round_trip(&proj, -68.0, -24.0);
    round_trip(&proj, 130.0, 41.0);
}

// ─── Hatano ─────────────────────────────────────────────────────────────

#[test]
fn hatano_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::Hatano)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 22.0, 15.0);
    round_trip(&proj, -64.0, -23.0);
    round_trip(&proj, 124.0, 39.0);
}

// ─── McBryde-Thomas Flat-Pole Sine ──────────────────────────────────────

#[test]
fn mbt_fps_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::MbtFps)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 20.0, 14.0);
    round_trip(&proj, -62.0, -22.0);
    round_trip(&proj, 118.0, 37.0);
}

// ─── McBryde-Thomas Flat-Polar Parabolic ────────────────────────────────

#[test]
fn mbtfpp_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::Mbtfpp)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 18.0, 13.0);
    round_trip(&proj, -58.0, -21.0);
    round_trip(&proj, 114.0, 36.0);
}

// ─── McBryde-Thomas Flat-Polar Quartic ──────────────────────────────────

#[test]
fn mbtfpq_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::Mbtfpq)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 19.0, 12.0);
    round_trip(&proj, -56.0, -20.0);
    round_trip(&proj, 112.0, 35.0);
}

// ─── Winkel I ───────────────────────────────────────────────────────────

#[test]
fn winkel_i_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::WinkelI)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 26.0, 18.0);
    round_trip(&proj, -74.0, -26.0);
    round_trip(&proj, 138.0, 46.0);
}

// ─── Kavrayskiy VII ─────────────────────────────────────────────────────

#[test]
fn kavrayskiy_vii_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::KavrayskiyVII)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 25.0, 18.0);
    round_trip(&proj, -70.0, -27.0);
    round_trip(&proj, 130.0, 46.0);
}

// ─── Nell-Hammer ────────────────────────────────────────────────────────

#[test]
fn nell_hammer_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::NellHammer)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 22.0, 14.0);
    round_trip(&proj, -64.0, -22.0);
    round_trip(&proj, 124.0, 40.0);
}

// ─── Euler ──────────────────────────────────────────────────────────────

#[test]
fn euler_round_trip() {
    let proj = Projection::new(
        ProjectionParams::new(ProjectionKind::Euler {
            lat1: 35.0,
            lat2: 55.0,
        })
        .with_lat0(45.0)
        .with_lon0(0.0),
    )
    .unwrap();

    round_trip(&proj, 0.0, 45.0);
    round_trip(&proj, 20.0, 50.0);
    round_trip(&proj, -25.0, 38.0);
    round_trip(&proj, 40.0, 30.0);
}

// ─── Tissot ─────────────────────────────────────────────────────────────

#[test]
fn tissot_round_trip() {
    let proj = Projection::new(
        ProjectionParams::new(ProjectionKind::Tissot {
            lat1: 35.0,
            lat2: 55.0,
        })
        .with_lat0(45.0)
        .with_lon0(0.0),
    )
    .unwrap();

    round_trip(&proj, 0.0, 40.0);
    round_trip(&proj, 18.0, 44.0);
    round_trip(&proj, -22.0, 34.0);
    round_trip(&proj, 36.0, 28.0);
}

// ─── Murdoch I ──────────────────────────────────────────────────────────

#[test]
fn murdoch_i_round_trip() {
    let proj = Projection::new(
        ProjectionParams::new(ProjectionKind::MurdochI {
            lat1: 35.0,
            lat2: 55.0,
        })
        .with_lat0(45.0)
        .with_lon0(0.0),
    )
    .unwrap();

    round_trip(&proj, 0.0, 45.0);
    round_trip(&proj, 18.0, 50.0);
    round_trip(&proj, -22.0, 40.0);
    round_trip(&proj, 36.0, 32.0);
}

// ─── Murdoch II ─────────────────────────────────────────────────────────

#[test]
fn murdoch_ii_round_trip() {
    let proj = Projection::new(
        ProjectionParams::new(ProjectionKind::MurdochII {
            lat1: 35.0,
            lat2: 55.0,
        })
        .with_lat0(45.0)
        .with_lon0(0.0),
    )
    .unwrap();

    round_trip(&proj, 0.0, 45.0);
    round_trip(&proj, 16.0, 50.0);
    round_trip(&proj, -20.0, 40.0);
    round_trip(&proj, 34.0, 32.0);
}

// ─── Murdoch III ────────────────────────────────────────────────────────

#[test]
fn murdoch_iii_round_trip() {
    let proj = Projection::new(
        ProjectionParams::new(ProjectionKind::MurdochIII {
            lat1: 35.0,
            lat2: 55.0,
        })
        .with_lat0(45.0)
        .with_lon0(0.0),
    )
    .unwrap();

    round_trip(&proj, 0.0, 45.0);
    round_trip(&proj, 17.0, 50.0);
    round_trip(&proj, -21.0, 40.0);
    round_trip(&proj, 35.0, 32.0);
}

// ─── Perspective Conic ──────────────────────────────────────────────────

#[test]
fn perspective_conic_round_trip() {
    let proj = Projection::new(
        ProjectionParams::new(ProjectionKind::PerspectiveConic {
            lat1: 30.0,
            lat2: 50.0,
        })
        .with_lat0(40.0)
        .with_lon0(0.0),
    )
    .unwrap();

    round_trip(&proj, 0.0, 40.0);
    round_trip(&proj, 18.0, 46.0);
    round_trip(&proj, -22.0, 34.0);
    round_trip(&proj, 34.0, 28.0);
}

// ─── Vitkovsky I ────────────────────────────────────────────────────────

#[test]
fn vitkovsky_i_round_trip() {
    let proj = Projection::new(
        ProjectionParams::new(ProjectionKind::VitkovskyI {
            lat1: 30.0,
            lat2: 50.0,
        })
        .with_lat0(40.0)
        .with_lon0(0.0),
    )
    .unwrap();

    round_trip(&proj, 0.0, 40.0);
    round_trip(&proj, 18.0, 46.0);
    round_trip(&proj, -22.0, 34.0);
    round_trip(&proj, 34.0, 28.0);
}

// ─── Tobler-Mercator ────────────────────────────────────────────────────

#[test]
fn tobler_mercator_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::ToblerMercator)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 24.0, 20.0);
    round_trip(&proj, -70.0, -30.0);
    round_trip(&proj, 130.0, 60.0);
}

// ─── Winkel II ──────────────────────────────────────────────────────────

#[test]
fn winkel_ii_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::WinkelII)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 22.0, 16.0);
    round_trip(&proj, -64.0, -24.0);
    round_trip(&proj, 124.0, 42.0);
}

// ─── Kavrayskiy V ───────────────────────────────────────────────────────

#[test]
fn kavrayskiy_v_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::KavrayskiyV)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 24.0, 18.0);
    round_trip(&proj, -68.0, -26.0);
    round_trip(&proj, 130.0, 44.0);
}

// ─── Central Conic ───────────────────────────────────────────────────────

#[test]
fn central_conic_round_trip() {
    let proj = Projection::new(
        ProjectionParams::new(ProjectionKind::CentralConic { lat1: 35.0 })
            .with_lon0(0.0),
    )
    .unwrap();

    round_trip(&proj, 0.0, 35.0);
    round_trip(&proj, 18.0, 42.0);
    round_trip(&proj, -22.0, 28.0);
    round_trip(&proj, 34.0, 20.0);
}

// ─── Lagrange ────────────────────────────────────────────────────────────

#[test]
fn lagrange_round_trip() {
    let proj = Projection::new(
        ProjectionParams::new(ProjectionKind::Lagrange {
            lat1: 20.0,
            w: 2.0,
        })
        .with_lon0(0.0),
    )
    .unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 16.0, 24.0);
    round_trip(&proj, -30.0, -18.0);
    round_trip(&proj, 42.0, 30.0);
}

// ─── McBryde-Thomas Flat-Polar Sine (No. 1) ─────────────────────────────

#[test]
fn mbt_s_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::MbtS)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 20.0, 14.0);
    round_trip(&proj, -62.0, -22.0);
    round_trip(&proj, 118.0, 37.0);
}

// ─── Natural Earth II ────────────────────────────────────────────────────

#[test]
fn natural_earth_ii_round_trip() {
    let proj = Projection::new(ProjectionParams::new(ProjectionKind::NaturalEarthII)).unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 24.0, 16.0);
    round_trip(&proj, -70.0, -24.0);
    round_trip(&proj, 132.0, 42.0);
}

// ─── Transverse Cylindrical Equal Area ──────────────────────────────────

#[test]
fn transverse_cylindrical_equal_area_round_trip() {
    let proj = Projection::new(
        ProjectionParams::new(ProjectionKind::TransverseCylindricalEqualArea)
            .with_scale(1.0)
            .with_lon0(0.0)
            .with_lat0(0.0),
    )
    .unwrap();

    round_trip(&proj, 0.0, 0.0);
    round_trip(&proj, 18.0, 24.0);
    round_trip(&proj, -24.0, -16.0);
    round_trip(&proj, 30.0, 30.0);
}

// ─── CRS datum transform ────────────────────────────────────────────────

#[test]
fn crs_utm_round_trip() {
    let crs = Crs::utm(32, false);
    let (x, y) = crs.forward(9.0, 48.0).unwrap();
    let (lon, lat) = crs.inverse(x, y).unwrap();
    assert!((lon - 9.0).abs() < TOL_DEGREES, "lon: {lon}");
    assert!((lat - 48.0).abs() < TOL_DEGREES, "lat: {lat}");
}

#[test]
fn crs_wgs84_to_web_mercator_to_back() {
    let src = Crs::web_mercator();
    let (x, y) = src.forward(13.4, 52.5).unwrap();
    let (lon, lat) = src.inverse(x, y).unwrap();
    assert!((lon - 13.4).abs() < TOL_DEGREES);
    assert!((lat - 52.5).abs() < TOL_DEGREES);
}

#[test]
fn crs_transform_to_4326_from_3857_returns_degrees() {
    let src = Crs::from_epsg(3857).unwrap();
    let dst = Crs::from_epsg(4326).unwrap();

    let (x, y) = src.forward(-2.0, -0.5).unwrap();
    let (lon, lat) = src.transform_to(x, y, &dst).unwrap();

    assert!((lon - (-2.0)).abs() < 1e-6, "lon={lon}");
    assert!((lat - (-0.5)).abs() < 1e-6, "lat={lat}");
}

#[test]
fn crs_transform_to_3857_from_4326_returns_meters() {
    let src = Crs::from_epsg(4326).unwrap();
    let dst = Crs::from_epsg(3857).unwrap();

    let (x_t, y_t) = src.transform_to(-2.0, -0.5, &dst).unwrap();
    let (x_e, y_e) = dst.forward(-2.0, -0.5).unwrap();

    assert!((x_t - x_e).abs() < 1e-6, "x_t={x_t}, x_e={x_e}");
    assert!((y_t - y_e).abs() < 1e-6, "y_t={y_t}, y_e={y_e}");
}

#[test]
fn crs_nad27_to_wgs84_has_nonzero_shift_and_round_trip() {
    let nad27 = Crs::from_epsg(4267).unwrap();
    let wgs84 = Crs::from_epsg(4326).unwrap();

    let lon0 = -75.0;
    let lat0 = 40.0;

    let (lon_w, lat_w) = nad27.transform_to(lon0, lat0, &wgs84).unwrap();
    let (lon_b, lat_b) = wgs84.transform_to(lon_w, lat_w, &nad27).unwrap();

    let dlon = (lon_w - lon0).abs();
    let dlat = (lat_w - lat0).abs();

    assert!(dlon > 1e-5 || dlat > 1e-5, "expected nonzero NAD27→WGS84 shift");
    assert!((lon_b - lon0).abs() < 1e-7, "lon_b={lon_b}, lon0={lon0}");
    assert!((lat_b - lat0).abs() < 1e-7, "lat_b={lat_b}, lat0={lat0}");
}

#[test]
fn crs_ed50_to_wgs84_has_nonzero_shift() {
    let ed50 = Crs::from_epsg(4230).unwrap();
    let wgs84 = Crs::from_epsg(4326).unwrap();

    let lon0 = 2.0;
    let lat0 = 49.0;
    let (lon_w, lat_w) = ed50.transform_to(lon0, lat0, &wgs84).unwrap();

    let dlon = (lon_w - lon0).abs();
    let dlat = (lat_w - lat0).abs();
    assert!(dlon > 1e-5 || dlat > 1e-5, "expected nonzero ED50→WGS84 shift");
}

#[test]
fn crs_nad83_to_wgs84_shift_is_small() {
    let nad83 = Crs::from_epsg(4269).unwrap();
    let wgs84 = Crs::from_epsg(4326).unwrap();

    let lon0 = -100.0;
    let lat0 = 45.0;
    let (lon_w, lat_w) = nad83.transform_to(lon0, lat0, &wgs84).unwrap();

    let dlon = (lon_w - lon0).abs();
    let dlat = (lat_w - lat0).abs();
    assert!(dlon < 0.001 && dlat < 0.001, "unexpectedly large NAD83→WGS84 shift");
}

#[test]
fn crs_nad83_csrs_to_wgs84_round_trip() {
    let csrs = Crs::from_epsg(4617).unwrap();
    let wgs84 = Crs::from_epsg(4326).unwrap();

    let lon0 = -123.1207;
    let lat0 = 49.2827;
    let (lon_w, lat_w) = csrs.transform_to(lon0, lat0, &wgs84).unwrap();
    let (lon_b, lat_b) = wgs84.transform_to(lon_w, lat_w, &csrs).unwrap();

    assert!((lon_b - lon0).abs() < 1e-7, "lon_b={lon_b}, lon0={lon0}");
    assert!((lat_b - lat0).abs() < 1e-7, "lat_b={lat_b}, lat0={lat0}");
}

#[test]
fn grid_registry_register_get_has_unregister() {
    let name = "TEST_GRID_REGISTRY";
    let _ = unregister_grid(name);

    let grid = GridShiftGrid::new(
        name,
        -1.0,
        -1.0,
        1.0,
        1.0,
        2,
        2,
        vec![
            GridShiftSample::new(0.5, -0.5),
            GridShiftSample::new(0.5, -0.5),
            GridShiftSample::new(0.5, -0.5),
            GridShiftSample::new(0.5, -0.5),
        ],
    )
    .unwrap();

    register_grid(grid).unwrap();
    assert!(has_grid(name).unwrap());
    assert!(get_grid(name).unwrap().is_some());
    assert!(unregister_grid(name).unwrap());
    assert!(!has_grid(name).unwrap());
}

#[test]
fn crs_grid_shift_geographic_to_wgs84_and_back() {
    let grid_name = "TEST_GRIDSHIFT_DATUM";
    let _ = unregister_grid(grid_name);

    let grid = GridShiftGrid::new(
        grid_name,
        -10.0,
        -10.0,
        20.0,
        20.0,
        2,
        2,
        vec![
            GridShiftSample::new(1.0, -2.0),
            GridShiftSample::new(1.0, -2.0),
            GridShiftSample::new(1.0, -2.0),
            GridShiftSample::new(1.0, -2.0),
        ],
    )
    .unwrap();
    register_grid(grid).unwrap();

    let src = Crs::new(
        "Test Grid Datum",
        Datum {
            name: "Test Grid Datum",
            ellipsoid: Ellipsoid::WGS84,
            transform: DatumTransform::GridShift { grid_name },
        },
        ProjectionParams::new(ProjectionKind::Geographic),
    )
    .unwrap();
    let dst = Crs::from_epsg(4326).unwrap();

    let lon0 = 1.0;
    let lat0 = 2.0;

    let (lon_w, lat_w) = src.transform_to(lon0, lat0, &dst).unwrap();
    assert!((lon_w - (lon0 + 1.0 / 3600.0)).abs() < 1e-10, "lon_w={lon_w}");
    assert!((lat_w - (lat0 - 2.0 / 3600.0)).abs() < 1e-10, "lat_w={lat_w}");

    let (lon_b, lat_b) = dst.transform_to(lon_w, lat_w, &src).unwrap();
    assert!((lon_b - lon0).abs() < 1e-10, "lon_b={lon_b}");
    assert!((lat_b - lat0).abs() < 1e-10, "lat_b={lat_b}");

    let _ = unregister_grid(grid_name);
}

#[test]
fn crs_grid_shift_policy_strict_vs_fallback_missing_grid() {
    let grid_name = "TEST_MISSING_GRID";
    let _ = unregister_grid(grid_name);

    let src = Crs::new(
        "Missing Grid Datum",
        Datum {
            name: "Missing Grid Datum",
            ellipsoid: Ellipsoid::WGS84,
            transform: DatumTransform::GridShift { grid_name },
        },
        ProjectionParams::new(ProjectionKind::Geographic),
    )
    .unwrap();
    let dst = Crs::from_epsg(4326).unwrap();

    let strict = src.transform_to_with_policy(10.0, 20.0, &dst, CrsTransformPolicy::Strict);
    assert!(strict.is_err(), "strict mode should error when grid is missing");

    let (lon_f, lat_f) = src
        .transform_to_with_policy(10.0, 20.0, &dst, CrsTransformPolicy::FallbackToIdentityGridShift)
        .unwrap();
    assert!((lon_f - 10.0).abs() < 1e-12, "lon_f={lon_f}");
    assert!((lat_f - 20.0).abs() < 1e-12, "lat_f={lat_f}");
}

#[test]
fn crs_grid_shift_policy_strict_vs_fallback_out_of_extent() {
    let grid_name = "TEST_GRID_OUTSIDE";
    let _ = unregister_grid(grid_name);

    let grid = GridShiftGrid::new(
        grid_name,
        -10.0,
        -10.0,
        1.0,
        1.0,
        2,
        2,
        vec![
            GridShiftSample::new(1.0, -1.0),
            GridShiftSample::new(1.0, -1.0),
            GridShiftSample::new(1.0, -1.0),
            GridShiftSample::new(1.0, -1.0),
        ],
    )
    .unwrap();
    register_grid(grid).unwrap();

    let src = Crs::new(
        "Small Grid Datum",
        Datum {
            name: "Small Grid Datum",
            ellipsoid: Ellipsoid::WGS84,
            transform: DatumTransform::GridShift { grid_name },
        },
        ProjectionParams::new(ProjectionKind::Geographic),
    )
    .unwrap();
    let dst = Crs::from_epsg(4326).unwrap();

    // (0,0) is intentionally outside the small grid extent [-10,-9] x [-10,-9].
    let strict = src.transform_to_with_policy(0.0, 0.0, &dst, CrsTransformPolicy::Strict);
    assert!(strict.is_err(), "strict mode should error out-of-extent");

    let (lon_f, lat_f) = src
        .transform_to_with_policy(0.0, 0.0, &dst, CrsTransformPolicy::FallbackToIdentityGridShift)
        .unwrap();
    assert!((lon_f - 0.0).abs() < 1e-12, "lon_f={lon_f}");
    assert!((lat_f - 0.0).abs() < 1e-12, "lat_f={lat_f}");

    let _ = unregister_grid(grid_name);
}

#[test]
fn crs_ntv2_hierarchy_selects_smallest_covering_subgrid() {
    fn temp_path(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        p.push(format!("wbproj_{name}_{t}"));
        p
    }

    fn rec_key_u32(key: &str, v: u32) -> [u8; 16] {
        let mut r = [0u8; 16];
        let kb = key.as_bytes();
        r[..kb.len().min(8)].copy_from_slice(&kb[..kb.len().min(8)]);
        r[8..12].copy_from_slice(&v.to_le_bytes());
        r
    }
    fn rec_key_f64(key: &str, v: f64) -> [u8; 16] {
        let mut r = [0u8; 16];
        let kb = key.as_bytes();
        r[..kb.len().min(8)].copy_from_slice(&kb[..kb.len().min(8)]);
        r[8..16].copy_from_slice(&v.to_le_bytes());
        r
    }
    fn rec_key_label(key: &str, value: &str) -> [u8; 16] {
        let mut r = [0u8; 16];
        let kb = key.as_bytes();
        r[..kb.len().min(8)].copy_from_slice(&kb[..kb.len().min(8)]);
        let vb = value.as_bytes();
        let n = vb.len().min(8);
        r[8..8 + n].copy_from_slice(&vb[..n]);
        r
    }
    fn shift_rec(dlat: f32, dlon_west: f32) -> [u8; 16] {
        let mut r = [0u8; 16];
        r[0..4].copy_from_slice(&dlat.to_le_bytes());
        r[4..8].copy_from_slice(&dlon_west.to_le_bytes());
        r
    }

    let path = temp_path("hier_multi.gsb");
    let mut bytes = Vec::new();

    bytes.extend_from_slice(&rec_key_u32("NUM_OREC", 11));
    bytes.extend_from_slice(&rec_key_u32("NUM_SREC", 11));
    bytes.extend_from_slice(&rec_key_u32("NUM_FILE", 2));
    bytes.extend_from_slice(&rec_key_f64("GS_TYPE", 0.0));
    bytes.extend_from_slice(&rec_key_f64("VERSION", 1.0));
    bytes.extend_from_slice(&rec_key_f64("SYSTEM_F", 0.0));
    bytes.extend_from_slice(&rec_key_f64("SYSTEM_T", 0.0));
    bytes.extend_from_slice(&rec_key_f64("MAJOR_F", 0.0));
    bytes.extend_from_slice(&rec_key_f64("MINOR_F", 0.0));
    bytes.extend_from_slice(&rec_key_f64("MAJOR_T", 0.0));
    bytes.extend_from_slice(&rec_key_f64("MINOR_T", 0.0));

    // Parent subgrid: 0..1 lon, 0..1 lat; +1" lon, +1" lat.
    bytes.extend_from_slice(&rec_key_label("SUB_NAME", "PARENT"));
    bytes.extend_from_slice(&rec_key_label("PARENT", "NONE"));
    bytes.extend_from_slice(&rec_key_label("CREATED", "20260313"));
    bytes.extend_from_slice(&rec_key_label("UPDATED", "20260313"));
    bytes.extend_from_slice(&rec_key_f64("S_LAT", 0.0));
    bytes.extend_from_slice(&rec_key_f64("N_LAT", 3600.0));
    bytes.extend_from_slice(&rec_key_f64("E_LONG", -3600.0));
    bytes.extend_from_slice(&rec_key_f64("W_LONG", 0.0));
    bytes.extend_from_slice(&rec_key_f64("LAT_INC", 3600.0));
    bytes.extend_from_slice(&rec_key_f64("LONG_INC", 3600.0));
    bytes.extend_from_slice(&rec_key_f64("GS_COUNT", 4.0));
    for _ in 0..4 {
        bytes.extend_from_slice(&shift_rec(1.0, -1.0));
    }

    // Child subgrid: 0..0.5 lon, 0..0.5 lat; +3" lon, +4" lat.
    bytes.extend_from_slice(&rec_key_label("SUB_NAME", "CHILD"));
    bytes.extend_from_slice(&rec_key_label("PARENT", "PARENT"));
    bytes.extend_from_slice(&rec_key_label("CREATED", "20260313"));
    bytes.extend_from_slice(&rec_key_label("UPDATED", "20260313"));
    bytes.extend_from_slice(&rec_key_f64("S_LAT", 0.0));
    bytes.extend_from_slice(&rec_key_f64("N_LAT", 1800.0));
    bytes.extend_from_slice(&rec_key_f64("E_LONG", -1800.0));
    bytes.extend_from_slice(&rec_key_f64("W_LONG", 0.0));
    bytes.extend_from_slice(&rec_key_f64("LAT_INC", 1800.0));
    bytes.extend_from_slice(&rec_key_f64("LONG_INC", 1800.0));
    bytes.extend_from_slice(&rec_key_f64("GS_COUNT", 4.0));
    for _ in 0..4 {
        bytes.extend_from_slice(&shift_rec(4.0, -3.0));
    }

    fs::write(&path, bytes).unwrap();
    register_ntv2_gsb_hierarchy(&path, "TEST_HIER").unwrap();

    let src = Crs::new(
        "Ntv2 Hier Datum",
        Datum {
            name: "Ntv2 Hier Datum",
            ellipsoid: Ellipsoid::WGS84,
            transform: DatumTransform::Ntv2Hierarchy {
                dataset_name: "TEST_HIER",
            },
        },
        ProjectionParams::new(ProjectionKind::Geographic),
    )
    .unwrap();
    let dst = Crs::from_epsg(4326).unwrap();

    // Inside child extent -> should use child shift.
    let (lon_c, lat_c) = src.transform_to(0.25, 0.25, &dst).unwrap();
    assert!((lon_c - (0.25 + 3.0 / 3600.0)).abs() < 1e-10, "lon_c={lon_c}");
    assert!((lat_c - (0.25 + 4.0 / 3600.0)).abs() < 1e-10, "lat_c={lat_c}");

    // Inside parent but outside child -> should use parent shift.
    let (lon_p, lat_p) = src.transform_to(0.75, 0.75, &dst).unwrap();
    assert!((lon_p - (0.75 + 1.0 / 3600.0)).abs() < 1e-10, "lon_p={lon_p}");
    assert!((lat_p - (0.75 + 1.0 / 3600.0)).abs() < 1e-10, "lat_p={lat_p}");

    // Outside all extents -> strict errors, fallback returns identity.
    let strict = src.transform_to_with_policy(2.0, 2.0, &dst, CrsTransformPolicy::Strict);
    assert!(strict.is_err());

    let (lon_f, lat_f) = src
        .transform_to_with_policy(2.0, 2.0, &dst, CrsTransformPolicy::FallbackToIdentityGridShift)
        .unwrap();
    assert!((lon_f - 2.0).abs() < 1e-12, "lon_f={lon_f}");
    assert!((lat_f - 2.0).abs() < 1e-12, "lat_f={lat_f}");

    let _ = fs::remove_file(&path);
}

#[test]
fn crs_ntv2_hierarchy_honors_parent_chain_over_global_smallest() {
    fn temp_path(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        p.push(format!("wbproj_{name}_{t}"));
        p
    }

    fn rec_key_u32(key: &str, v: u32) -> [u8; 16] {
        let mut r = [0u8; 16];
        let kb = key.as_bytes();
        r[..kb.len().min(8)].copy_from_slice(&kb[..kb.len().min(8)]);
        r[8..12].copy_from_slice(&v.to_le_bytes());
        r
    }
    fn rec_key_f64(key: &str, v: f64) -> [u8; 16] {
        let mut r = [0u8; 16];
        let kb = key.as_bytes();
        r[..kb.len().min(8)].copy_from_slice(&kb[..kb.len().min(8)]);
        r[8..16].copy_from_slice(&v.to_le_bytes());
        r
    }
    fn rec_key_label(key: &str, value: &str) -> [u8; 16] {
        let mut r = [0u8; 16];
        let kb = key.as_bytes();
        r[..kb.len().min(8)].copy_from_slice(&kb[..kb.len().min(8)]);
        let vb = value.as_bytes();
        let n = vb.len().min(8);
        r[8..8 + n].copy_from_slice(&vb[..n]);
        r
    }
    fn shift_rec(dlat: f32, dlon_west: f32) -> [u8; 16] {
        let mut r = [0u8; 16];
        r[0..4].copy_from_slice(&dlat.to_le_bytes());
        r[4..8].copy_from_slice(&dlon_west.to_le_bytes());
        r
    }

    let path = temp_path("hier_parent_chain.gsb");
    let mut bytes = Vec::new();

    // 3 subgrids: root A (small root), root B (larger root), child C under B (tiny).
    // Point (0.5,0.5) lies in all 3. Parent-chain resolver should pick A (best root),
    // not C (global smallest area).
    bytes.extend_from_slice(&rec_key_u32("NUM_OREC", 11));
    bytes.extend_from_slice(&rec_key_u32("NUM_SREC", 11));
    bytes.extend_from_slice(&rec_key_u32("NUM_FILE", 3));
    bytes.extend_from_slice(&rec_key_f64("GS_TYPE", 0.0));
    bytes.extend_from_slice(&rec_key_f64("VERSION", 1.0));
    bytes.extend_from_slice(&rec_key_f64("SYSTEM_F", 0.0));
    bytes.extend_from_slice(&rec_key_f64("SYSTEM_T", 0.0));
    bytes.extend_from_slice(&rec_key_f64("MAJOR_F", 0.0));
    bytes.extend_from_slice(&rec_key_f64("MINOR_F", 0.0));
    bytes.extend_from_slice(&rec_key_f64("MAJOR_T", 0.0));
    bytes.extend_from_slice(&rec_key_f64("MINOR_T", 0.0));

    // Root A: extent 0..1 x 0..1; +1", +1"
    bytes.extend_from_slice(&rec_key_label("SUB_NAME", "A"));
    bytes.extend_from_slice(&rec_key_label("PARENT", "NONE"));
    bytes.extend_from_slice(&rec_key_label("CREATED", "20260313"));
    bytes.extend_from_slice(&rec_key_label("UPDATED", "20260313"));
    bytes.extend_from_slice(&rec_key_f64("S_LAT", 0.0));
    bytes.extend_from_slice(&rec_key_f64("N_LAT", 3600.0));
    bytes.extend_from_slice(&rec_key_f64("E_LONG", -3600.0));
    bytes.extend_from_slice(&rec_key_f64("W_LONG", 0.0));
    bytes.extend_from_slice(&rec_key_f64("LAT_INC", 3600.0));
    bytes.extend_from_slice(&rec_key_f64("LONG_INC", 3600.0));
    bytes.extend_from_slice(&rec_key_f64("GS_COUNT", 4.0));
    for _ in 0..4 {
        bytes.extend_from_slice(&shift_rec(1.0, -1.0));
    }

    // Root B: extent 0..2 x 0..2; +2", +2"
    bytes.extend_from_slice(&rec_key_label("SUB_NAME", "B"));
    bytes.extend_from_slice(&rec_key_label("PARENT", "NONE"));
    bytes.extend_from_slice(&rec_key_label("CREATED", "20260313"));
    bytes.extend_from_slice(&rec_key_label("UPDATED", "20260313"));
    bytes.extend_from_slice(&rec_key_f64("S_LAT", 0.0));
    bytes.extend_from_slice(&rec_key_f64("N_LAT", 7200.0));
    bytes.extend_from_slice(&rec_key_f64("E_LONG", -7200.0));
    bytes.extend_from_slice(&rec_key_f64("W_LONG", 0.0));
    bytes.extend_from_slice(&rec_key_f64("LAT_INC", 7200.0));
    bytes.extend_from_slice(&rec_key_f64("LONG_INC", 7200.0));
    bytes.extend_from_slice(&rec_key_f64("GS_COUNT", 4.0));
    for _ in 0..4 {
        bytes.extend_from_slice(&shift_rec(2.0, -2.0));
    }

    // Child C under B: tiny extent 0..0.5 x 0..0.5; +9", +9"
    bytes.extend_from_slice(&rec_key_label("SUB_NAME", "C"));
    bytes.extend_from_slice(&rec_key_label("PARENT", "B"));
    bytes.extend_from_slice(&rec_key_label("CREATED", "20260313"));
    bytes.extend_from_slice(&rec_key_label("UPDATED", "20260313"));
    bytes.extend_from_slice(&rec_key_f64("S_LAT", 0.0));
    bytes.extend_from_slice(&rec_key_f64("N_LAT", 1800.0));
    bytes.extend_from_slice(&rec_key_f64("E_LONG", -1800.0));
    bytes.extend_from_slice(&rec_key_f64("W_LONG", 0.0));
    bytes.extend_from_slice(&rec_key_f64("LAT_INC", 1800.0));
    bytes.extend_from_slice(&rec_key_f64("LONG_INC", 1800.0));
    bytes.extend_from_slice(&rec_key_f64("GS_COUNT", 4.0));
    for _ in 0..4 {
        bytes.extend_from_slice(&shift_rec(9.0, -9.0));
    }

    fs::write(&path, bytes).unwrap();
    register_ntv2_gsb_hierarchy(&path, "TEST_HIER_CHAIN").unwrap();

    let src = Crs::new(
        "Ntv2 Chain Datum",
        Datum {
            name: "Ntv2 Chain Datum",
            ellipsoid: Ellipsoid::WGS84,
            transform: DatumTransform::Ntv2Hierarchy {
                dataset_name: "TEST_HIER_CHAIN",
            },
        },
        ProjectionParams::new(ProjectionKind::Geographic),
    )
    .unwrap();
    let dst = Crs::from_epsg(4326).unwrap();

    let (lon, lat) = src.transform_to(0.5, 0.5, &dst).unwrap();
    assert!((lon - (0.5 + 1.0 / 3600.0)).abs() < 1e-10, "lon={lon}");
    assert!((lat - (0.5 + 1.0 / 3600.0)).abs() < 1e-10, "lat={lat}");

    let _ = fs::remove_file(&path);
}

#[test]
fn ntv2_hierarchy_introspection_reports_selected_subgrid() {
    fn temp_path(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        p.push(format!("wbproj_{name}_{t}"));
        p
    }

    fn rec_key_u32(key: &str, v: u32) -> [u8; 16] {
        let mut r = [0u8; 16];
        let kb = key.as_bytes();
        r[..kb.len().min(8)].copy_from_slice(&kb[..kb.len().min(8)]);
        r[8..12].copy_from_slice(&v.to_le_bytes());
        r
    }
    fn rec_key_f64(key: &str, v: f64) -> [u8; 16] {
        let mut r = [0u8; 16];
        let kb = key.as_bytes();
        r[..kb.len().min(8)].copy_from_slice(&kb[..kb.len().min(8)]);
        r[8..16].copy_from_slice(&v.to_le_bytes());
        r
    }
    fn rec_key_label(key: &str, value: &str) -> [u8; 16] {
        let mut r = [0u8; 16];
        let kb = key.as_bytes();
        r[..kb.len().min(8)].copy_from_slice(&kb[..kb.len().min(8)]);
        let vb = value.as_bytes();
        let n = vb.len().min(8);
        r[8..8 + n].copy_from_slice(&vb[..n]);
        r
    }
    fn shift_rec(dlat: f32, dlon_west: f32) -> [u8; 16] {
        let mut r = [0u8; 16];
        r[0..4].copy_from_slice(&dlat.to_le_bytes());
        r[4..8].copy_from_slice(&dlon_west.to_le_bytes());
        r
    }

    let path = temp_path("hier_introspect.gsb");
    let mut bytes = Vec::new();

    bytes.extend_from_slice(&rec_key_u32("NUM_OREC", 11));
    bytes.extend_from_slice(&rec_key_u32("NUM_SREC", 11));
    bytes.extend_from_slice(&rec_key_u32("NUM_FILE", 2));
    bytes.extend_from_slice(&rec_key_f64("GS_TYPE", 0.0));
    bytes.extend_from_slice(&rec_key_f64("VERSION", 1.0));
    bytes.extend_from_slice(&rec_key_f64("SYSTEM_F", 0.0));
    bytes.extend_from_slice(&rec_key_f64("SYSTEM_T", 0.0));
    bytes.extend_from_slice(&rec_key_f64("MAJOR_F", 0.0));
    bytes.extend_from_slice(&rec_key_f64("MINOR_F", 0.0));
    bytes.extend_from_slice(&rec_key_f64("MAJOR_T", 0.0));
    bytes.extend_from_slice(&rec_key_f64("MINOR_T", 0.0));

    // Parent: +1",+1"
    bytes.extend_from_slice(&rec_key_label("SUB_NAME", "PARENT"));
    bytes.extend_from_slice(&rec_key_label("PARENT", "NONE"));
    bytes.extend_from_slice(&rec_key_label("CREATED", "20260313"));
    bytes.extend_from_slice(&rec_key_label("UPDATED", "20260313"));
    bytes.extend_from_slice(&rec_key_f64("S_LAT", 0.0));
    bytes.extend_from_slice(&rec_key_f64("N_LAT", 3600.0));
    bytes.extend_from_slice(&rec_key_f64("E_LONG", -3600.0));
    bytes.extend_from_slice(&rec_key_f64("W_LONG", 0.0));
    bytes.extend_from_slice(&rec_key_f64("LAT_INC", 3600.0));
    bytes.extend_from_slice(&rec_key_f64("LONG_INC", 3600.0));
    bytes.extend_from_slice(&rec_key_f64("GS_COUNT", 4.0));
    for _ in 0..4 {
        bytes.extend_from_slice(&shift_rec(1.0, -1.0));
    }

    // Child: +4",+5"
    bytes.extend_from_slice(&rec_key_label("SUB_NAME", "CHILD"));
    bytes.extend_from_slice(&rec_key_label("PARENT", "PARENT"));
    bytes.extend_from_slice(&rec_key_label("CREATED", "20260313"));
    bytes.extend_from_slice(&rec_key_label("UPDATED", "20260313"));
    bytes.extend_from_slice(&rec_key_f64("S_LAT", 0.0));
    bytes.extend_from_slice(&rec_key_f64("N_LAT", 1800.0));
    bytes.extend_from_slice(&rec_key_f64("E_LONG", -1800.0));
    bytes.extend_from_slice(&rec_key_f64("W_LONG", 0.0));
    bytes.extend_from_slice(&rec_key_f64("LAT_INC", 1800.0));
    bytes.extend_from_slice(&rec_key_f64("LONG_INC", 1800.0));
    bytes.extend_from_slice(&rec_key_f64("GS_COUNT", 4.0));
    for _ in 0..4 {
        bytes.extend_from_slice(&shift_rec(5.0, -4.0));
    }

    fs::write(&path, bytes).unwrap();
    register_ntv2_gsb_hierarchy(&path, "TEST_HIER_INTROSPECT").unwrap();

    let grid_name = resolve_ntv2_hierarchy_grid_name("TEST_HIER_INTROSPECT", 0.25, 0.25)
        .unwrap()
        .unwrap();
    assert_eq!(grid_name, "TEST_HIER_INTROSPECT::CHILD");

    let subgrid = resolve_ntv2_hierarchy_subgrid("TEST_HIER_INTROSPECT", 0.25, 0.25)
        .unwrap()
        .unwrap();
    assert_eq!(subgrid, "CHILD");

    let none_subgrid = resolve_ntv2_hierarchy_subgrid("TEST_HIER_INTROSPECT", 9.0, 9.0).unwrap();
    assert!(none_subgrid.is_none());

    let _ = fs::remove_file(&path);
}

#[test]
fn crs_transform_trace_reports_selected_source_and_target_grids() {
    let src_grid = GridShiftGrid {
        name: "TRACE_SRC".to_string(),
        lon_min: -1.0,
        lat_min: -1.0,
        lon_step: 2.0,
        lat_step: 2.0,
        width: 2,
        height: 2,
        samples: vec![
            GridShiftSample { dlon_arcsec: 1.0, dlat_arcsec: 1.0 },
            GridShiftSample { dlon_arcsec: 1.0, dlat_arcsec: 1.0 },
            GridShiftSample { dlon_arcsec: 1.0, dlat_arcsec: 1.0 },
            GridShiftSample { dlon_arcsec: 1.0, dlat_arcsec: 1.0 },
        ],
    };
    let dst_grid = GridShiftGrid {
        name: "TRACE_DST".to_string(),
        lon_min: -1.0,
        lat_min: -1.0,
        lon_step: 2.0,
        lat_step: 2.0,
        width: 2,
        height: 2,
        samples: vec![
            GridShiftSample { dlon_arcsec: 2.0, dlat_arcsec: 2.0 },
            GridShiftSample { dlon_arcsec: 2.0, dlat_arcsec: 2.0 },
            GridShiftSample { dlon_arcsec: 2.0, dlat_arcsec: 2.0 },
            GridShiftSample { dlon_arcsec: 2.0, dlat_arcsec: 2.0 },
        ],
    };

    register_grid(src_grid).unwrap();
    register_grid(dst_grid).unwrap();

    let src = Crs::new(
        "Source Trace CRS",
        Datum {
            name: "Source Trace Datum",
            ellipsoid: Ellipsoid::WGS84,
            transform: DatumTransform::GridShift {
                grid_name: "TRACE_SRC",
            },
        },
        ProjectionParams::new(ProjectionKind::Geographic),
    )
    .unwrap();

    let dst = Crs::new(
        "Target Trace CRS",
        Datum {
            name: "Target Trace Datum",
            ellipsoid: Ellipsoid::WGS84,
            transform: DatumTransform::GridShift {
                grid_name: "TRACE_DST",
            },
        },
        ProjectionParams::new(ProjectionKind::Geographic),
    )
    .unwrap();

    let trace = src
        .transform_to_with_trace(0.0, 0.0, &dst, CrsTransformPolicy::Strict)
        .unwrap();

    assert_eq!(trace.source_grid.as_deref(), Some("TRACE_SRC"));
    assert_eq!(trace.target_grid.as_deref(), Some("TRACE_DST"));

    unregister_grid("TRACE_SRC").unwrap();
    unregister_grid("TRACE_DST").unwrap();
}

#[test]
fn crs_transform_trace_strict_convenience_matches_strict_policy() {
    let src = Crs::from_epsg(4326).unwrap();
    let dst = Crs::from_epsg(3857).unwrap();

    let (x, y) = src.forward(-73.9857, 40.7484).unwrap();

    let via_policy = src
        .transform_to_with_trace(x, y, &dst, CrsTransformPolicy::Strict)
        .unwrap();

    let via_helper = src.transform_to_with_trace_strict(x, y, &dst).unwrap();

    assert!((via_policy.x - via_helper.x).abs() < 1e-12);
    assert!((via_policy.y - via_helper.y).abs() < 1e-12);
    assert_eq!(via_policy.source_grid, via_helper.source_grid);
    assert_eq!(via_policy.target_grid, via_helper.target_grid);
}

#[test]
fn datum_builder_with_grid_shift_sets_transform() {
    let datum = Datum::NAD27.with_grid_shift("NADCON_TEST_GRID");
    match datum.transform {
        DatumTransform::GridShift { grid_name } => assert_eq!(grid_name, "NADCON_TEST_GRID"),
        _ => panic!("expected GridShift transform"),
    }
}

#[test]
fn datum_builder_with_ntv2_hierarchy_sets_transform() {
    let datum = Datum::OSGB36.with_ntv2_hierarchy("OSTN15_DATASET");
    match datum.transform {
        DatumTransform::Ntv2Hierarchy { dataset_name } => {
            assert_eq!(dataset_name, "OSTN15_DATASET")
        }
        _ => panic!("expected Ntv2Hierarchy transform"),
    }
}
