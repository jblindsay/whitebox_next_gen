//! Tests for the EPSG registry.

use crate::{
    CompoundCrs,
    ConstantVerticalOffsetProvider,
    Crs,
    CrsTransformPolicy,
    EpsgResolutionPolicy,
    EpsgIdentifyPolicy,
    GridVerticalOffsetProvider,
    VerticalOffsetGrid,
    clear_runtime_epsg_aliases,
    epsg_alias_catalog,
    epsg_from_srs_reference,
    epsg_from_wkt,
    identify_epsg_from_wkt_report,
    identify_epsg_from_wkt_with_policy,
    compound_from_wkt,
    from_epsg,
    from_epsg_with_catalog,
    from_epsg_with_policy,
    from_wkt,
    register_vertical_offset_grid,
    register_epsg_alias,
    resolve_epsg_with_catalog,
    resolve_epsg_with_policy,
    runtime_epsg_aliases,
    to_esri_wkt,
    to_ogc_wkt,
    to_geotiff_info,
    unregister_vertical_offset_grid,
    unregister_epsg_alias,
    vertical_offset_grid_name,
};
use crate::epsg::{epsg_info, known_epsg_codes};
use std::fs;
use std::sync::{Mutex, OnceLock};

const TOL: f64 = 1e-6;

fn runtime_alias_test_guard() -> std::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("runtime alias test mutex poisoned")
}

// ─── basic lookup ──────────────────────────────────────────────────────────

#[test]
fn epsg_4326_creates_ok() {
    let crs = Crs::from_epsg(4326).unwrap();
    assert!(crs.name.contains("WGS"));
}

#[test]
fn epsg_first_batch_datums_are_mapped() {
    assert_eq!(Crs::from_epsg(3405).unwrap().datum.name, "VN-2000");
    assert_eq!(Crs::from_epsg(3577).unwrap().datum.name, "GDA94");
    assert_eq!(Crs::from_epsg(2193).unwrap().datum.name, "NZGD2000");
    assert_eq!(Crs::from_epsg(6689).unwrap().datum.name, "JGD2011");
    assert_eq!(Crs::from_epsg(2449).unwrap().datum.name, "JGD2000");
    assert_eq!(Crs::from_epsg(6707).unwrap().datum.name, "RDN2008");
}

#[test]
fn epsg_second_batch_datums_are_mapped() {
    assert_eq!(Crs::from_epsg(27700).unwrap().datum.name, "OSGB36");
    assert_eq!(Crs::from_epsg(31467).unwrap().datum.name, "DHDN");
    assert_eq!(Crs::from_epsg(3833).unwrap().datum.name, "Pulkovo 1942(58)");
    assert_eq!(Crs::from_epsg(3834).unwrap().datum.name, "Pulkovo 1942(83)");
    assert_eq!(Crs::from_epsg(5514).unwrap().datum.name, "S-JTSK");
}

#[test]
fn epsg_third_batch_datums_are_mapped() {
    assert_eq!(Crs::from_epsg(31370).unwrap().datum.name, "Belge 1972");
    assert_eq!(Crs::from_epsg(28992).unwrap().datum.name, "Amersfoort");
    assert_eq!(Crs::from_epsg(29903).unwrap().datum.name, "TM65");
    assert_eq!(Crs::from_epsg(3986).unwrap().datum.name, "Katanga 1955");
    assert_eq!(Crs::from_epsg(22275).unwrap().datum.name, "Cape");
}

#[test]
fn epsg_fourth_batch_datums_are_mapped() {
    assert_eq!(Crs::from_epsg(3991).unwrap().datum.name, "Puerto Rico 1927");
    assert_eq!(Crs::from_epsg(3992).unwrap().datum.name, "St. Croix");
    assert_eq!(Crs::from_epsg(21781).unwrap().datum.name, "CH1903");
    assert_eq!(Crs::from_epsg(2056).unwrap().datum.name, "CH1903+");
}

#[test]
fn epsg_fifth_batch_datums_are_mapped() {
    assert_eq!(Crs::from_epsg(6915).unwrap().datum.name, "South East Island 1943");
    assert_eq!(Crs::from_epsg(6927).unwrap().datum.name, "SVY21");
    assert_eq!(Crs::from_epsg(6956).unwrap().datum.name, "VN-2000");
    assert_eq!(Crs::from_epsg(6957).unwrap().datum.name, "VN-2000");
}

#[test]
fn epsg_4000_parity_block_codes_are_mapped() {
    // Geographic definitions
    assert_eq!(Crs::from_epsg(4001).unwrap().datum.name, "D_Airy_1830");
    assert_eq!(Crs::from_epsg(4019).unwrap().datum.name, "D_GRS_1980");
    assert_eq!(Crs::from_epsg(4035).unwrap().datum.name, "D_Sphere");
    assert_eq!(Crs::from_epsg(4046).unwrap().datum.name, "D_Reseau_Geodesique_de_la_RDC_2005");

    // Projected definitions
    assert_eq!(Crs::from_epsg(4026).unwrap().datum.name, "D_MOLDREF99");
    assert_eq!(Crs::from_epsg(4037).unwrap().datum.name, "D_WGS_1984");
    assert_eq!(Crs::from_epsg(4048).unwrap().datum.name, "D_Reseau_Geodesique_de_la_RDC_2005");
    assert_eq!(Crs::from_epsg(4063).unwrap().datum.name, "D_Reseau_Geodesique_de_la_RDC_2005");
}

#[test]
fn epsg_step2_then_step1_parity_blocks_are_mapped() {
    // Step 2 block representatives: 2391-2442 and 2867-2954
    assert!(Crs::from_epsg(2391).unwrap().name.contains("Finland"));
    assert!(Crs::from_epsg(2422).unwrap().name.contains("Beijing"));
    assert!(Crs::from_epsg(2867).unwrap().name.contains("NAD_1983_HARN"));
    assert!(Crs::from_epsg(2954).unwrap().name.contains("Prince_Edward_Island"));

    // Step 1 block representatives: 4120-4147, 4149-4151, 4153-4166, 4168-4176, 4178-4185
    assert!(Crs::from_epsg(4120).unwrap().name.contains("Greek"));
    assert!(Crs::from_epsg(4151).unwrap().name.contains("Swiss"));
    assert!(Crs::from_epsg(4165).unwrap().name.contains("Bissau"));
    assert!(Crs::from_epsg(4185).unwrap().name.contains("Madeira"));
}

#[test]
fn epsg_generated_batch1_codes_build_ok() {
    assert!(Crs::from_epsg(2027).is_ok());
    assert!(Crs::from_epsg(3033).is_ok());
}

#[test]
fn epsg_generated_batch2_codes_build_ok() {
    assert!(Crs::from_epsg(3153).is_ok());
    assert!(Crs::from_epsg(3440).is_ok());
}

#[test]
fn epsg_generated_batch3_codes_build_ok() {
    assert!(Crs::from_epsg(3447).is_ok());
    assert!(Crs::from_epsg(5357).is_ok());
}

#[test]
fn epsg_generated_batch4_codes_build_ok() {
    assert!(Crs::from_epsg(7374).is_ok());
    assert!(Crs::from_epsg(9680).is_ok());
}

#[test]
fn epsg_generated_batch5_codes_build_ok() {
    assert!(Crs::from_epsg(21028).is_ok());
    assert!(Crs::from_epsg(23032).is_ok());
}

#[test]
fn epsg_generated_batch6_codes_build_ok() {
    assert!(Crs::from_epsg(5361).is_ok());
    assert!(Crs::from_epsg(9698).is_ok());
}

#[test]
fn epsg_generated_batch7_codes_build_ok() {
    assert!(Crs::from_epsg(9699).is_ok());
    assert!(Crs::from_epsg(23095).is_ok());
}

#[test]
fn epsg_generated_batch8_codes_build_ok() {
    assert!(Crs::from_epsg(23240).is_ok());
    assert!(Crs::from_epsg(27572).is_ok());
}

#[test]
fn epsg_generated_batch9_codes_build_ok() {
    assert!(Crs::from_epsg(2000).is_ok());
    assert!(Crs::from_epsg(3120).is_ok());
}

#[test]
fn epsg_generated_batch10_codes_build_ok() {
    assert!(Crs::from_epsg(3126).is_ok());
    assert!(Crs::from_epsg(3362).is_ok());
}

#[test]
fn epsg_generated_batch11_codes_build_ok() {
    assert!(Crs::from_epsg(3363).is_ok());
    assert!(Crs::from_epsg(3877).is_ok());
}

#[test]
fn epsg_generated_batch12_codes_build_ok() {
    assert!(Crs::from_epsg(3878).is_ok());
    assert!(Crs::from_epsg(5257).is_ok());
}

#[test]
fn epsg_generated_batch13_codes_build_ok() {
    assert!(Crs::from_epsg(5258).is_ok());
    assert!(Crs::from_epsg(6077).is_ok());
}

#[test]
fn epsg_generated_batch14_codes_build_ok() {
    assert!(Crs::from_epsg(6078).is_ok());
    assert!(Crs::from_epsg(6633).is_ok());
}

#[test]
fn epsg_generated_batch15_codes_build_ok() {
    assert!(Crs::from_epsg(6829).is_ok());
    assert!(Crs::from_epsg(7543).is_ok());
}

#[test]
fn epsg_generated_batch16_codes_build_ok() {
    assert!(Crs::from_epsg(7544).is_ok());
    assert!(Crs::from_epsg(7643).is_ok());
}

#[test]
fn epsg_generated_batch17_codes_build_ok() {
    assert!(Crs::from_epsg(7644).is_ok());
    assert!(Crs::from_epsg(8110).is_ok());
}

#[test]
fn epsg_generated_batch18_codes_build_ok() {
    assert!(Crs::from_epsg(8111).is_ok());
    assert!(Crs::from_epsg(8312).is_ok());
}

#[test]
fn epsg_generated_batch19_codes_build_ok() {
    assert!(Crs::from_epsg(8313).is_ok());
    assert!(Crs::from_epsg(9766).is_ok());
}

#[test]
fn epsg_generated_batch20_codes_build_ok() {
    assert!(Crs::from_epsg(9821).is_ok());
    assert!(Crs::from_epsg(10626).is_ok());
}

#[test]
fn epsg_generated_batch21_codes_build_ok() {
    assert!(Crs::from_epsg(10632).is_ok());
    assert!(Crs::from_epsg(21316).is_ok());
}

#[test]
fn epsg_generated_batch22_codes_build_ok() {
    assert!(Crs::from_epsg(21317).is_ok());
    assert!(Crs::from_epsg(23304).is_ok());
}

#[test]
fn epsg_generated_batch23_codes_build_ok() {
    assert!(Crs::from_epsg(23305).is_ok());
    assert!(Crs::from_epsg(26773).is_ok());
}

#[test]
fn epsg_generated_batch24_codes_build_ok() {
    assert!(Crs::from_epsg(26774).is_ok());
    assert!(Crs::from_epsg(28348).is_ok());
}

#[test]
fn epsg_generated_batch25_codes_build_ok() {
    assert!(Crs::from_epsg(28357).is_ok());
    assert!(Crs::from_epsg(32014).is_ok());
}

#[test]
fn epsg_generated_batch26_codes_build_ok() {
    assert!(Crs::from_epsg(32015).is_ok());
    assert!(Crs::from_epsg(32153).is_ok());
}

#[test]
fn epsg_generated_batch27_codes_build_ok() {
    assert!(Crs::from_epsg(32154).is_ok());
    assert!(Crs::from_epsg(32766).is_ok());
}

#[test]
fn epsg_generated_batch28_codes_build_ok() {
    assert!(Crs::from_epsg(3078).is_ok());
    assert!(Crs::from_epsg(26731).is_ok());
}

#[test]
fn epsg_generated_batch29_codes_build_ok() {
    assert!(Crs::from_epsg(3167).is_ok());
    assert!(Crs::from_epsg(29874).is_ok());
}

#[test]
fn epsg_generated_batch30_codes_build_ok() {
    assert!(Crs::from_epsg(2985).is_ok());
    assert!(Crs::from_epsg(2986).is_ok());
}

#[test]
fn epsg_generated_batch31_codes_build_ok() {
    assert!(Crs::from_epsg(2963).is_ok());
    assert!(Crs::from_epsg(29701).is_ok());
}

#[test]
fn epsg_generated_batch32_codes_build_ok() {
    assert!(Crs::from_epsg(22300).is_ok());
    assert!(Crs::from_epsg(32700).is_ok());
}

#[test]
fn epsg_generated_batch33_codes_build_ok() {
    assert!(Crs::from_epsg(27200).is_ok());
    assert!(Crs::from_epsg(27200).is_ok());
}

#[test]
fn epsg_generated_batch34_codes_build_ok() {
    assert!(Crs::from_epsg(9895).is_ok());
    assert!(Crs::from_epsg(9895).is_ok());
}

#[test]
fn known_epsg_codes_includes_generated_batch1_tail_code() {
    assert!(known_epsg_codes().contains(&3149));
}

#[test]
fn known_epsg_codes_includes_generated_batch2_tail_code() {
    assert!(known_epsg_codes().contains(&3440));
}

#[test]
fn known_epsg_codes_includes_generated_batch3_tail_code() {
    assert!(known_epsg_codes().contains(&5357));
}

#[test]
fn known_epsg_codes_includes_generated_batch4_tail_code() {
    assert!(known_epsg_codes().contains(&9680));
}

#[test]
fn known_epsg_codes_includes_generated_batch5_tail_code() {
    assert!(known_epsg_codes().contains(&23032));
}

#[test]
fn known_epsg_codes_includes_generated_batch6_tail_code() {
    assert!(known_epsg_codes().contains(&9698));
}

#[test]
fn known_epsg_codes_includes_generated_batch7_tail_code() {
    assert!(known_epsg_codes().contains(&23095));
}

#[test]
fn known_epsg_codes_includes_generated_batch8_tail_code() {
    assert!(known_epsg_codes().contains(&27572));
}

#[test]
fn known_epsg_codes_includes_generated_batch9_tail_code() {
    assert!(known_epsg_codes().contains(&3120));
}

#[test]
fn known_epsg_codes_includes_generated_batch10_tail_code() {
    assert!(known_epsg_codes().contains(&3362));
}

#[test]
fn known_epsg_codes_includes_generated_batch11_tail_code() {
    assert!(known_epsg_codes().contains(&3877));
}

#[test]
fn known_epsg_codes_includes_generated_batch12_tail_code() {
    assert!(known_epsg_codes().contains(&5257));
}

#[test]
fn known_epsg_codes_includes_generated_batch13_tail_code() {
    assert!(known_epsg_codes().contains(&6077));
}

#[test]
fn known_epsg_codes_includes_generated_batch14_tail_code() {
    assert!(known_epsg_codes().contains(&6633));
}

#[test]
fn known_epsg_codes_includes_generated_batch15_tail_code() {
    assert!(known_epsg_codes().contains(&7543));
}

#[test]
fn known_epsg_codes_includes_generated_batch16_tail_code() {
    assert!(known_epsg_codes().contains(&7643));
}

#[test]
fn known_epsg_codes_includes_generated_batch17_tail_code() {
    assert!(known_epsg_codes().contains(&8110));
}

#[test]
fn known_epsg_codes_includes_generated_batch18_tail_code() {
    assert!(known_epsg_codes().contains(&8312));
}

#[test]
fn known_epsg_codes_includes_generated_batch19_tail_code() {
    assert!(known_epsg_codes().contains(&9766));
}

#[test]
fn known_epsg_codes_includes_generated_batch20_tail_code() {
    assert!(known_epsg_codes().contains(&10626));
}

#[test]
fn known_epsg_codes_includes_generated_batch21_tail_code() {
    assert!(known_epsg_codes().contains(&21316));
}

#[test]
fn known_epsg_codes_includes_generated_batch22_tail_code() {
    assert!(known_epsg_codes().contains(&23304));
}

#[test]
fn known_epsg_codes_includes_generated_batch23_tail_code() {
    assert!(known_epsg_codes().contains(&26773));
}

#[test]
fn known_epsg_codes_includes_generated_batch24_tail_code() {
    assert!(known_epsg_codes().contains(&28348));
}

#[test]
fn known_epsg_codes_includes_generated_batch25_tail_code() {
    assert!(known_epsg_codes().contains(&32014));
}

#[test]
fn known_epsg_codes_includes_generated_batch26_tail_code() {
    assert!(known_epsg_codes().contains(&32153));
}

#[test]
fn known_epsg_codes_includes_generated_batch27_tail_code() {
    assert!(known_epsg_codes().contains(&32766));
}

#[test]
fn known_epsg_codes_includes_generated_batch28_tail_code() {
    assert!(known_epsg_codes().contains(&26731));
}

#[test]
fn known_epsg_codes_includes_generated_batch29_tail_code() {
    assert!(known_epsg_codes().contains(&29874));
}

#[test]
fn known_epsg_codes_includes_generated_batch30_tail_code() {
    assert!(known_epsg_codes().contains(&2986));
}

#[test]
fn known_epsg_codes_includes_generated_batch31_tail_code() {
    assert!(known_epsg_codes().contains(&29701));
}

#[test]
fn known_epsg_codes_includes_generated_batch32_tail_code() {
    assert!(known_epsg_codes().contains(&32700));
}

#[test]
fn known_epsg_codes_includes_generated_batch33_tail_code() {
    assert!(known_epsg_codes().contains(&27200));
}

#[test]
fn known_epsg_codes_includes_generated_batch34_tail_code() {
    assert!(known_epsg_codes().contains(&9895));
}

#[test]
fn epsg_generated_metadata_is_specific() {
    let info = epsg_info(2027).unwrap();
    assert_eq!(info.name, "NAD27(76) / UTM zone 15N");
    assert_eq!(info.unit, "metre");

    let info = epsg_info(3153).unwrap();
    assert_eq!(info.name, "NAD83(CSRS) / BC Albers");
    assert!(info.area_of_use.contains("British Columbia") || info.area_of_use.contains("Canada"));

    let info = epsg_info(3447).unwrap();
    assert_eq!(info.name, "ETRS89 / Belgian Lambert 2005");
    assert_eq!(info.unit, "metre");

    let info = epsg_info(9680).unwrap();
    assert!(info.name.contains("UTM") || info.name.contains("TM") || info.name.contains("Lambert"));

    let info = epsg_info(21028).unwrap();
    assert!(info.name.contains("TM") || info.name.contains("Transverse") || info.name.contains("Gauss"));

    let info = epsg_info(5361).unwrap();
    assert!(info.name.contains("TM") || info.name.contains("UTM") || info.name.contains("Lambert"));

    let info = epsg_info(9699).unwrap();
    assert!(info.name.contains("TM") || info.name.contains("UTM") || info.name.contains("Lambert"));

    let info = epsg_info(23240).unwrap();
    assert!(
        info.name.contains("TM") || info.name.contains("UTM") || info.name.contains("Gauss")
    );

    let info = epsg_info(2000).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(3126).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(3363).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(3878).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(5258).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(6078).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(6829).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(7544).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(7644).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(8111).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(8313).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(9821).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(10632).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(21317).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(23305).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(26774).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(28357).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(32015).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(32154).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(3078).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(3167).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(2985).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(2963).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(22300).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(27200).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info(9895).unwrap();
    assert!(!info.name.is_empty());
}

#[test]
fn epsg_geocentric_and_vertical_codes_create_ok() {
    let geocentric = Crs::from_epsg(7842).unwrap();
    assert!(geocentric.name.contains("geocentric"));

    let vertical = Crs::from_epsg(7841).unwrap();
    assert!(vertical.name.contains("height"));

    for code in [3855_u32, 5701, 5702, 5703, 5714, 5715, 5773, 8228] {
        let crs = Crs::from_epsg(code).unwrap();
        assert!(crs.name.contains("height") || crs.name.contains("depth"));
    }
}

#[test]
fn compound_crs_epsg_7405_builds_and_transforms() {
    let src = CompoundCrs::from_epsg(7405).unwrap();
    let dst = CompoundCrs::new(
        "WGS84 + EGM96",
        Crs::from_epsg(4326).unwrap(),
        Crs::from_epsg(5773).unwrap(),
    )
    .unwrap();

    let (expected_x, expected_y) = src
        .horizontal
        .transform_to(530_000.0, 180_000.0, &dst.horizontal)
        .unwrap();

    let (x2, y2, z2) = src.transform_to(530_000.0, 180_000.0, 50.0, &dst).unwrap();

    assert!((x2 - expected_x).abs() < 1e-9);
    assert!((y2 - expected_y).abs() < 1e-9);
    assert!((z2 - 50.0).abs() < 1e-9);
}

#[test]
fn crs_transform_to_3d_supports_geocentric_round_trip() {
    let geographic = Crs::from_epsg(7843).unwrap();
    let geocentric = Crs::from_epsg(7842).unwrap();

    let lon = 147.0;
    let lat = -35.0;
    let h = 125.0;

    let (x, y, z) = geographic.transform_to_3d(lon, lat, h, &geocentric).unwrap();
    let (lon2, lat2, h2) = geocentric.transform_to_3d(x, y, z, &geographic).unwrap();

    assert!((lon2 - lon).abs() < 1e-8);
    assert!((lat2 - lat).abs() < 1e-8);
    assert!((h2 - h).abs() < 1e-3);
}

#[test]
fn crs_transform_to_3d_vertical_to_vertical_passthrough() {
    let src_vertical = Crs::from_epsg(7841).unwrap();
    let dst_vertical = Crs::from_epsg(7841).unwrap();

    let (x2, y2, z2) = src_vertical
        .transform_to_3d(147.0, -35.0, 123.456, &dst_vertical)
        .unwrap();

    assert!((x2 - 147.0).abs() < TOL);
    assert!((y2 + 35.0).abs() < TOL);
    assert!((z2 - 123.456).abs() < TOL);
}

#[test]
fn crs_transform_to_3d_vertical_to_vertical_applies_registered_offsets() {
    let src_grid_name = "egm96";
    let dst_grid_name = "egm2008";

    let _ = unregister_vertical_offset_grid(src_grid_name);
    let _ = unregister_vertical_offset_grid(dst_grid_name);

    register_vertical_offset_grid(
        VerticalOffsetGrid::new(
            src_grid_name,
            140.0,
            -40.0,
            10.0,
            10.0,
            2,
            2,
            vec![20.0, 20.0, 20.0, 20.0],
        )
        .unwrap(),
    )
    .unwrap();

    register_vertical_offset_grid(
        VerticalOffsetGrid::new(
            dst_grid_name,
            140.0,
            -40.0,
            10.0,
            10.0,
            2,
            2,
            vec![5.0, 5.0, 5.0, 5.0],
        )
        .unwrap(),
    )
    .unwrap();

    let src_vertical = Crs::from_epsg(5773).unwrap();
    let dst_vertical = Crs::from_epsg(3855).unwrap();

    let (_x2, _y2, z2) = src_vertical
        .transform_to_3d(147.0, -35.0, 100.0, &dst_vertical)
        .unwrap();

    // z_out = (100 + 20) - 5 = 115
    assert!((z2 - 115.0).abs() < TOL);

    unregister_vertical_offset_grid(src_grid_name).unwrap();
    unregister_vertical_offset_grid(dst_grid_name).unwrap();
}

#[test]
fn crs_transform_to_3d_vertical_to_nonvertical_returns_error() {
    let vertical = Crs::from_epsg(7841).unwrap();
    let geographic = Crs::from_epsg(7843).unwrap();

    let err = vertical.transform_to_3d(147.0, -35.0, 10.0, &geographic);
    assert!(err.is_err());
}

#[test]
fn crs_transform_to_3d_preserve_horizontal_allows_vertical_mixed_mode() {
    let vertical = Crs::from_epsg(7841).unwrap();
    let projected = Crs::from_epsg(7846).unwrap();

    let (x2, y2, z2) = projected
        .transform_to_3d_preserve_horizontal(500_123.0, 6_123_456.0, 42.0, &vertical)
        .unwrap();

    assert!((x2 - 500_123.0).abs() < TOL);
    assert!((y2 - 6_123_456.0).abs() < TOL);
    assert!((z2 - 42.0).abs() < TOL);

    let (x3, y3, z3) = vertical
        .transform_to_3d_preserve_horizontal(500_123.0, 6_123_456.0, 42.0, &projected)
        .unwrap();

    assert!((x3 - 500_123.0).abs() < TOL);
    assert!((y3 - 6_123_456.0).abs() < TOL);
    assert!((z3 - 42.0).abs() < TOL);
}

#[test]
fn crs_transform_to_3d_preserve_horizontal_rejects_vertical_geocentric_mix() {
    let vertical = Crs::from_epsg(7841).unwrap();
    let geocentric = Crs::from_epsg(7842).unwrap();

    let err = vertical.transform_to_3d_preserve_horizontal(1.0, 2.0, 3.0, &geocentric);
    assert!(err.is_err());
}

#[test]
fn crs_transform_to_3d_preserve_horizontal_with_vertical_offsets_adjusts_z() {
    let vertical = Crs::from_epsg(7841).unwrap();
    let projected = Crs::from_epsg(7846).unwrap();

    // Example conversion using external vertical offsets:
    // source_to_ellipsoidal=+30m, target_to_ellipsoidal=+10m
    // z_out = (100 + 30) - 10 = 120
    let (x2, y2, z2) = projected
        .transform_to_3d_preserve_horizontal_with_vertical_offsets(
            500_000.0,
            6_120_000.0,
            100.0,
            &vertical,
            30.0,
            10.0,
        )
        .unwrap();

    assert!((x2 - 500_000.0).abs() < TOL);
    assert!((y2 - 6_120_000.0).abs() < TOL);
    assert!((z2 - 120.0).abs() < TOL);
}

#[test]
fn crs_transform_to_3d_preserve_horizontal_with_vertical_offsets_policy_variant_adjusts_z() {
    let vertical = Crs::from_epsg(7841).unwrap();
    let projected = Crs::from_epsg(7846).unwrap();

    let (x2, y2, z2) = projected
        .transform_to_3d_preserve_horizontal_with_vertical_offsets_and_policy(
            500_000.0,
            6_120_000.0,
            100.0,
            &vertical,
            30.0,
            10.0,
            CrsTransformPolicy::FallbackToIdentityGridShift,
        )
        .unwrap();

    assert!((x2 - 500_000.0).abs() < TOL);
    assert!((y2 - 6_120_000.0).abs() < TOL);
    assert!((z2 - 120.0).abs() < TOL);
}

#[test]
fn crs_transform_to_3d_preserve_horizontal_with_vertical_offsets_rejects_vertical_geocentric_mix() {
    let vertical = Crs::from_epsg(7841).unwrap();
    let geocentric = Crs::from_epsg(7842).unwrap();

    let err = vertical.transform_to_3d_preserve_horizontal_with_vertical_offsets(
        1.0,
        2.0,
        3.0,
        &geocentric,
        0.0,
        0.0,
    );
    assert!(err.is_err());
}

#[test]
fn crs_transform_to_3d_preserve_horizontal_with_provider_adjusts_z() {
    let vertical = Crs::from_epsg(7841).unwrap();
    let projected = Crs::from_epsg(7846).unwrap();

    let provider = |_: f64, _: f64, _: &Crs, _: &Crs| -> crate::Result<(f64, f64)> {
        Ok((30.0, 10.0))
    };

    let (x2, y2, z2) = projected
        .transform_to_3d_preserve_horizontal_with_provider(
            500_000.0,
            6_120_000.0,
            100.0,
            &vertical,
            &provider,
        )
        .unwrap();

    assert!((x2 - 500_000.0).abs() < TOL);
    assert!((y2 - 6_120_000.0).abs() < TOL);
    assert!((z2 - 120.0).abs() < TOL);
}

#[test]
fn crs_transform_to_3d_preserve_horizontal_with_provider_propagates_provider_error() {
    let vertical = Crs::from_epsg(7841).unwrap();
    let projected = Crs::from_epsg(7846).unwrap();

    let provider = |_: f64, _: f64, _: &Crs, _: &Crs| -> crate::Result<(f64, f64)> {
        Err(crate::ProjectionError::DatumError("provider failed".to_string()))
    };

    let out = projected.transform_to_3d_preserve_horizontal_with_provider(
        500_000.0,
        6_120_000.0,
        100.0,
        &vertical,
        &provider,
    );
    assert!(out.is_err());
}

#[test]
fn crs_transform_to_3d_preserve_horizontal_with_constant_provider_adjusts_z() {
    let vertical = Crs::from_epsg(7841).unwrap();
    let projected = Crs::from_epsg(7846).unwrap();

    let provider = ConstantVerticalOffsetProvider::new(30.0, 10.0);

    let (x2, y2, z2) = projected
        .transform_to_3d_preserve_horizontal_with_provider(
            500_000.0,
            6_120_000.0,
            100.0,
            &vertical,
            &provider,
        )
        .unwrap();

    assert!((x2 - 500_000.0).abs() < TOL);
    assert!((y2 - 6_120_000.0).abs() < TOL);
    assert!((z2 - 120.0).abs() < TOL);
}

#[test]
fn crs_transform_to_3d_preserve_horizontal_with_grid_provider_adjusts_z() {
    let source_grid_name = "test_vertical_source_grid";
    let target_grid_name = "test_vertical_target_grid";

    let _ = unregister_vertical_offset_grid(source_grid_name);
    let _ = unregister_vertical_offset_grid(target_grid_name);

    register_vertical_offset_grid(
        VerticalOffsetGrid::new(
            source_grid_name,
            140.0,
            -40.0,
            10.0,
            10.0,
            2,
            2,
            vec![30.0, 30.0, 30.0, 30.0],
        )
        .unwrap(),
    )
    .unwrap();

    register_vertical_offset_grid(
        VerticalOffsetGrid::new(
            target_grid_name,
            140.0,
            -40.0,
            10.0,
            10.0,
            2,
            2,
            vec![10.0, 10.0, 10.0, 10.0],
        )
        .unwrap(),
    )
    .unwrap();

    let geographic = Crs::from_epsg(7843).unwrap();
    let vertical = Crs::from_epsg(7841).unwrap();
    let provider = GridVerticalOffsetProvider::new(source_grid_name, target_grid_name);

    let (_x2, _y2, z2) = geographic
        .transform_to_3d_preserve_horizontal_with_provider(147.0, -35.0, 100.0, &vertical, &provider)
        .unwrap();

    assert!((z2 - 120.0).abs() < TOL);

    unregister_vertical_offset_grid(source_grid_name).unwrap();
    unregister_vertical_offset_grid(target_grid_name).unwrap();
}

#[test]
fn epsg_3857_web_mercator() {
    let crs = Crs::from_epsg(3857).unwrap();
    let (x, y) = crs.forward(0.0, 0.0).unwrap();
    assert!(x.abs() < 1e-4);
    assert!(y.abs() < 1e-4);
}

#[test]
fn epsg_unknown_returns_error() {
    assert!(Crs::from_epsg(9999999).is_err());
}

#[test]
fn epsg_policy_strict_still_errors_for_unknown_code() {
    let out = from_epsg_with_policy(9999999, EpsgResolutionPolicy::Strict);
    assert!(out.is_err());
}

#[test]
fn epsg_policy_fallback_to_wgs84_resolves_unknown_code() {
    let crs = from_epsg_with_policy(9999999, EpsgResolutionPolicy::FallbackToWgs84).unwrap();
    assert!(crs.name.contains("WGS"));

    let resolved =
        resolve_epsg_with_policy(9999999, EpsgResolutionPolicy::FallbackToWgs84).unwrap();
    assert_eq!(resolved.requested_code, 9999999);
    assert_eq!(resolved.resolved_code, 4326);
    assert!(!resolved.used_alias_catalog);
    assert!(resolved.used_fallback);
}

#[test]
fn epsg_policy_fallback_to_custom_epsg_resolves_unknown_code() {
    let crs = from_epsg_with_policy(9999999, EpsgResolutionPolicy::FallbackToEpsg(3857)).unwrap();
    assert!(crs.name.contains("Mercator"));

    let resolved =
        resolve_epsg_with_policy(9999999, EpsgResolutionPolicy::FallbackToEpsg(3857)).unwrap();
    assert_eq!(resolved.resolved_code, 3857);
    assert!(!resolved.used_alias_catalog);
    assert!(resolved.used_fallback);
}

#[test]
fn epsg_policy_fallback_reports_error_if_fallback_code_unsupported() {
    let out = from_epsg_with_policy(9999999, EpsgResolutionPolicy::FallbackToEpsg(8888888));
    assert!(out.is_err());
}

#[test]
fn crs_from_epsg_with_policy_mirrors_module_api() {
    let crs = Crs::from_epsg_with_policy(9999999, EpsgResolutionPolicy::FallbackToWebMercator)
        .unwrap();
    assert!(crs.name.contains("Mercator"));
}

#[test]
fn epsg_alias_catalog_contains_legacy_webmercator_codes() {
    let entries = epsg_alias_catalog();
    assert!(entries.iter().any(|e| e.source_code == 900913 && e.target_epsg == 3857));
    assert!(entries.iter().any(|e| e.source_code == 102100 && e.target_epsg == 3857));
}

#[test]
fn epsg_catalog_resolver_maps_legacy_alias_before_fallback() {
    let resolved = resolve_epsg_with_catalog(900913, EpsgResolutionPolicy::FallbackToWgs84).unwrap();
    assert_eq!(resolved.resolved_code, 3857);
    assert!(resolved.used_alias_catalog);
    assert!(!resolved.used_fallback);
}

#[test]
fn epsg_from_with_catalog_constructs_from_legacy_alias_code() {
    let crs = from_epsg_with_catalog(102100, EpsgResolutionPolicy::Strict).unwrap();
    assert!(crs.name.contains("Mercator"));
}

#[test]
fn crs_from_epsg_with_catalog_mirrors_module_api() {
    let crs = Crs::from_epsg_with_catalog(3785, EpsgResolutionPolicy::Strict).unwrap();
    assert!(crs.name.contains("Mercator"));
}

#[test]
fn runtime_alias_registration_resolves_custom_code() {
    let _guard = runtime_alias_test_guard();
    clear_runtime_epsg_aliases();
    register_epsg_alias(9100001, 4326).unwrap();

    let resolved = resolve_epsg_with_catalog(9100001, EpsgResolutionPolicy::Strict).unwrap();
    assert_eq!(resolved.resolved_code, 4326);
    assert!(resolved.used_alias_catalog);
    assert!(!resolved.used_fallback);

    let aliases = runtime_epsg_aliases();
    assert!(aliases.iter().any(|(s, t)| *s == 9100001 && *t == 4326));

    assert_eq!(unregister_epsg_alias(9100001), Some(4326));
    clear_runtime_epsg_aliases();
}

#[test]
fn runtime_alias_overrides_built_in_alias_mapping() {
    let _guard = runtime_alias_test_guard();
    clear_runtime_epsg_aliases();
    register_epsg_alias(900913, 4326).unwrap();

    let resolved = resolve_epsg_with_catalog(900913, EpsgResolutionPolicy::Strict).unwrap();
    assert_eq!(resolved.resolved_code, 4326);
    assert!(resolved.used_alias_catalog);

    unregister_epsg_alias(900913);
    clear_runtime_epsg_aliases();
}

#[test]
fn runtime_alias_rejects_unsupported_target() {
    let _guard = runtime_alias_test_guard();
    clear_runtime_epsg_aliases();
    let out = register_epsg_alias(9100002, 9999999);
    assert!(out.is_err());
    clear_runtime_epsg_aliases();
}

// ─── UTM WGS84 N/S ────────────────────────────────────────────────────────

#[test]
fn epsg_32632_utm32n() {
    // EPSG:32632 = WGS84 UTM zone 32N
    let crs = Crs::from_epsg(32632).unwrap();
    let (e, n) = crs.forward(9.0, 48.0).unwrap();
    // Round-trip
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 9.0).abs() < TOL, "lon: {lon}");
    assert!((lat - 48.0).abs() < TOL, "lat: {lat}");
}

#[test]
fn epsg_32755_utm55s() {
    // EPSG:32755 = WGS84 UTM zone 55S  (eastern Australia)
    let crs = Crs::from_epsg(32755).unwrap();
    let (e, n) = crs.forward(147.0, -35.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 147.0).abs() < TOL);
    assert!((lat - -35.0).abs() < TOL);
}

#[test]
fn epsg_32661_ups_north_roundtrip() {
    let crs = Crs::from_epsg(32661).unwrap();
    let (e, n) = crs.forward(10.0, 85.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 10.0).abs() < 1e-4);
    assert!((lat - 85.0).abs() < 1e-4);
}

#[test]
fn epsg_32761_ups_south_roundtrip() {
    let crs = Crs::from_epsg(32761).unwrap();
    let (e, n) = crs.forward(10.0, -85.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 10.0).abs() < 1e-4);
    assert!((lat - -85.0).abs() < 1e-4);
}

#[test]
fn epsg_utm_all_north_zones_construct() {
    for zone in 1u32..=60 {
        let code = 32600 + zone;
        assert!(Crs::from_epsg(code).is_ok(), "EPSG:{code} failed");
    }
}

#[test]
fn epsg_utm_all_south_zones_construct() {
    for zone in 1u32..=60 {
        let code = 32700 + zone;
        assert!(Crs::from_epsg(code).is_ok(), "EPSG:{code} failed");
    }
}

#[test]
fn epsg_wgs72be_utm_all_north_zones_construct() {
    for zone in 1u32..=60 {
        let code = 32400 + zone;
        assert!(Crs::from_epsg(code).is_ok(), "EPSG:{code} failed");
    }
}

#[test]
fn epsg_wgs72be_utm_all_south_zones_construct() {
    for zone in 1u32..=60 {
        let code = 32500 + zone;
        assert!(Crs::from_epsg(code).is_ok(), "EPSG:{code} failed");
    }
}

// ─── NAD83 / NAD27 UTM ────────────────────────────────────────────────────

#[test]
fn epsg_26918_nad83_utm18n() {
    let crs = Crs::from_epsg(26918).unwrap();
    let (e, n) = crs.forward(-75.0, 40.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -75.0).abs() < TOL);
    assert!((lat - 40.0).abs() < TOL);
}

#[test]
fn epsg_nad83_2011_utm_block_roundtrip() {
    let checks = [
        (6328u32, 170.5, 52.0), // zone 59N
        (6329u32, 176.5, 54.0), // zone 60N
        (6330u32, -177.0, 58.0), // zone 1N
        (6348u32, -69.0, 43.0),  // zone 19N
    ];

    for (code, lon_in, lat_in) in checks {
        let crs = Crs::from_epsg(code).unwrap();
        assert_eq!(crs.datum.name, "NAD83(2011)", "EPSG:{code} datum");
        let (e, n) = crs.forward(lon_in, lat_in).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - lon_in).abs() < 1e-5, "EPSG:{code} lon");
        assert!((lat - lat_in).abs() < 1e-5, "EPSG:{code} lat");
    }
}

#[test]
fn epsg_csrs_utm_active_and_realization_codes_roundtrip() {
    let checks = [
        (2961u32, -63.0, 47.0),  // NAD83(CSRS) zone 20N
        (3154u32, -141.0, 64.0), // NAD83(CSRS) zone 7N
        (9713u32, -39.0, 8.5),   // NAD83(CSRS) zone 24N
        (22207u32, -141.0, 64.0), // NAD83(CSRS)v2 zone 7N
        (22222u32, -45.0, 72.0),  // NAD83(CSRS)v2 zone 22N
        (22807u32, -141.0, 64.0), // NAD83(CSRS)v8 zone 7N
        (22822u32, -45.0, 72.0),  // NAD83(CSRS)v8 zone 22N
    ];

    for (code, lon_in, lat_in) in checks {
        let crs = Crs::from_epsg(code).unwrap();
        assert!(crs.datum.name.contains("NAD83"), "EPSG:{code} datum");
        let (e, n) = crs.forward(lon_in, lat_in).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - lon_in).abs() < 1e-5, "EPSG:{code} lon");
        assert!((lat - lat_in).abs() < 1e-5, "EPSG:{code} lat");
    }
}

#[test]
fn epsg_sirgas2000_utm_active_codes_roundtrip() {
    let checks = [
        (31965u32, -117.2, 14.5), // 11N
        (31985u32, -32.8, -9.0),  // 25S
        (6210u32, -44.0, 8.8),    // 23N
        (5396u32, -27.0, -20.0),  // 26S
    ];

    for (code, lon_in, lat_in) in checks {
        let crs = Crs::from_epsg(code).unwrap();
        assert_eq!(crs.datum.name, "SIRGAS2000");
        let (e, n) = crs.forward(lon_in, lat_in).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - lon_in).abs() < 1e-5, "EPSG:{code} lon");
        assert!((lat - lat_in).abs() < 1e-5, "EPSG:{code} lat");
    }
}

#[test]
fn epsg_sad69_psad56_utm_active_codes_roundtrip() {
    let checks = [
        (5463u32, -82.0, 12.0, "SAD69"),
        (29168u32, -75.0, 9.0, "SAD69"),
        (29195u32, -31.0, -8.0, "SAD69"),
        (24817u32, -81.0, 11.0, "PSAD56"),
        (24882u32, -48.0, -12.0, "PSAD56"),
    ];

    for (code, lon_in, lat_in, datum_name) in checks {
        let crs = Crs::from_epsg(code).unwrap();
        assert_eq!(crs.datum.name, datum_name, "EPSG:{code} datum");
        let (e, n) = crs.forward(lon_in, lat_in).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - lon_in).abs() < 1e-5, "EPSG:{code} lon");
        assert!((lat - lat_in).abs() < 1e-5, "EPSG:{code} lat");
    }
}

#[test]
fn epsg_26715_nad27_utm15n() {
    let crs = Crs::from_epsg(26715).unwrap();
    let (e, n) = crs.forward(-93.0, 44.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -93.0).abs() < TOL);
    assert!((lat - 44.0).abs() < TOL);
}

// ─── Named / specialty projections ────────────────────────────────────────

#[test]
fn epsg_27700_british_national_grid() {
    let crs = Crs::from_epsg(27700).unwrap();
    // Round-trip test for London
    let (e, n) = crs.forward(-0.1276, 51.5074).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -0.1276).abs() < 1e-4);
    assert!((lat - 51.5074).abs() < 1e-4);
}

#[test]
fn epsg_28992_rd_new_netherlands() {
    let crs = Crs::from_epsg(28992).unwrap();
    // Amsterdam ≈ (5.9°E, 52.37°N)
    let (e, n) = crs.forward(4.9, 52.37).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 4.9).abs() < 1e-4);
    assert!((lat - 52.37).abs() < 1e-4);
}

#[test]
fn epsg_2154_lambert93_france() {
    let crs = Crs::from_epsg(2154).unwrap();
    let (e, n) = crs.forward(2.35, 48.85).unwrap(); // Paris
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 2.35).abs() < 1e-4);
    assert!((lat - 48.85).abs() < 1e-4);
}

#[test]
fn epsg_2157_irish_tm_roundtrip() {
    let crs = Crs::from_epsg(2157).unwrap();
    let (e, n) = crs.forward(-6.26, 53.35).unwrap(); // Dublin
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -6.26).abs() < 1e-4);
    assert!((lat - 53.35).abs() < 1e-4);
}

#[test]
fn epsg_2163_us_national_atlas_equal_area_roundtrip() {
    let crs = Crs::from_epsg(2163).unwrap();
    let (e, n) = crs.forward(-100.0, 40.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -100.0).abs() < 1e-4);
    assert!((lat - 40.0).abs() < 1e-4);
}

#[test]
fn epsg_2193_nztm2000_roundtrip() {
    let crs = Crs::from_epsg(2193).unwrap();
    let (e, n) = crs.forward(174.78, -41.29).unwrap(); // Wellington
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 174.78).abs() < 1e-4);
    assert!((lat - -41.29).abs() < 1e-4);
}

#[test]
fn epsg_3067_tm35fin_roundtrip() {
    let crs = Crs::from_epsg(3067).unwrap();
    let (e, n) = crs.forward(24.94, 60.17).unwrap(); // Helsinki
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 24.94).abs() < 1e-4);
    assert!((lat - 60.17).abs() < 1e-4);
}

#[test]
fn epsg_3006_sweref99_tm_roundtrip() {
    let crs = Crs::from_epsg(3006).unwrap();
    let (e, n) = crs.forward(18.07, 59.33).unwrap(); // Stockholm
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 18.07).abs() < 1e-4);
    assert!((lat - 59.33).abs() < 1e-4);
}

#[test]
fn epsg_29903_irish_grid_roundtrip() {
    let crs = Crs::from_epsg(29903).unwrap();
    let (e, n) = crs.forward(-6.26, 53.35).unwrap(); // Dublin
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -6.26).abs() < 1e-4);
    assert!((lat - 53.35).abs() < 1e-4);
}

#[test]
fn epsg_31370_belgian_lambert72_roundtrip() {
    let crs = Crs::from_epsg(31370).unwrap();
    let (e, n) = crs.forward(4.35, 50.85).unwrap(); // Brussels
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 4.35).abs() < 1e-4);
    assert!((lat - 50.85).abs() < 1e-4);
}

#[test]
fn epsg_5514_krovak_east_north_roundtrip() {
    let crs = Crs::from_epsg(5514).unwrap();
    let (e, n) = crs.forward(14.42, 50.09).unwrap(); // Prague
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 14.42).abs() < 1e-4);
    assert!((lat - 50.09).abs() < 1e-4);
    assert!(e.is_finite() && n.is_finite());
}

#[test]
fn epsg_2227_california_zone3_ftus_roundtrip() {
    let crs = Crs::from_epsg(2227).unwrap();
    let (e, n) = crs.forward(-121.0, 37.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -121.0).abs() < 1e-4);
    assert!((lat - 37.5).abs() < 1e-4);
    assert!(e.abs() > 1_000_000.0);
}

#[test]
fn epsg_spcs83_national_meter_new_codes_roundtrip() {
    let cases = [
        (26929u32, -85.5, 32.5),   // Alabama East (TM)
        (26931u32, -134.2, 57.2),  // Alaska zone 1 (Oblique Mercator)
        (26953u32, -105.2, 40.0),  // Colorado North (LCC)
        (26961u32, -155.2, 19.5),  // Hawaii zone 1 (TM)
        (26991u32, -93.5, 47.8),   // Minnesota North (LCC)
        (26998u32, -94.1, 38.5),   // Missouri West (TM)
    ];

    for (code, lon_in, lat_in) in cases {
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon_in, lat_in).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - lon_in).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - lat_in).abs() < 1e-4, "EPSG:{code} lat");
    }
}

#[test]
fn epsg_spcs83_harn_national_meter_codes_roundtrip() {
    let cases = [
        (2759u32, -85.5, 32.5),   // Alabama East (TM)
        (2772u32, -105.2, 40.0),  // Colorado North (LCC)
        (2824u32, -74.7, 40.2),   // New Jersey (TM)
        (2838u32, -122.0, 45.5),  // Oregon North (LCC)
        (2852u32, -72.6, 44.0),   // Vermont (TM)
        (2866u32, -66.3, 18.3),   // Puerto Rico and Virgin Is. (LCC)
    ];

    for (code, lon_in, lat_in) in cases {
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon_in, lat_in).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - lon_in).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - lat_in).abs() < 1e-4, "EPSG:{code} lat");
    }
}

#[test]
fn epsg_spcs83_nsrs2007_codes_roundtrip() {
    let cases = [
        (3465u32, -85.5, 32.5),   // Alabama East (TM)
        (3468u32, -134.2, 57.2),  // Alaska zone 1 (Oblique Mercator)
        (3477u32, -176.2, 51.7),  // Alaska zone 10 (LCC)
        (3501u32, -105.2, 39.8),  // Colorado Central (LCC)
        (3502u32, -105.2, 39.8),  // Colorado Central (ftUS)
        (3511u32, -80.9, 27.3),   // Florida East (TM)
        (3552u32, -91.2, 29.8),   // Louisiana South (LCC)
    ];

    for (code, lon_in, lat_in) in cases {
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon_in, lat_in).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - lon_in).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - lat_in).abs() < 1e-4, "EPSG:{code} lat");
    }
}

#[test]
fn epsg_spcs83_nad83_2011_codes_roundtrip() {
    let cases = [
        (6355u32, -85.5, 32.5),    // Alabama East (TM, m)
        (6393u32, -150.0, 64.0),   // Alaska Albers (m)
        (6394u32, -133.7, 57.2),   // Alaska zone 1 (Oblique Mercator, m)
        (6429u32, -105.2, 40.0),   // Colorado North (LCC, m)
        (6405u32, -111.9, 33.5),   // Arizona Central (TM, ft)
        (6430u32, -105.2, 40.0),   // Colorado North (LCC, ftUS)
        (6494u32, -84.8, 44.5),    // Michigan Central (LCC, ft)
    ];

    for (code, lon_in, lat_in) in cases {
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon_in, lat_in).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - lon_in).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - lat_in).abs() < 1e-4, "EPSG:{code} lat");
        assert!(e.is_finite() && n.is_finite(), "EPSG:{code} en");
    }
}

#[test]
fn epsg_26947_is_unassigned_returns_error() {
    assert!(Crs::from_epsg(26947).is_err());
}

#[test]
fn epsg_3034_etrs89_lcc_europe() {
    let crs = Crs::from_epsg(3034).unwrap();
    let (e, n) = crs.forward(10.0, 52.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 10.0).abs() < 1e-4);
    assert!((lat - 52.0).abs() < 1e-4);
}

#[test]
fn epsg_3035_etrs89_laea_europe() {
    let crs = Crs::from_epsg(3035).unwrap();
    let (e, n) = crs.forward(10.0, 52.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 10.0).abs() < 1e-4);
    assert!((lat - 52.0).abs() < 1e-4);
    assert!(e.is_finite() && n.is_finite());
}

#[test]
fn epsg_3031_antarctic_polar_stereo_roundtrip() {
    let crs = Crs::from_epsg(3031).unwrap();
    let (e, n) = crs.forward(0.0, -80.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 0.0).abs() < 1e-4);
    assert!((lat - -80.0).abs() < 1e-4);
}

#[test]
fn epsg_3032_australian_antarctic_polar_stereo_roundtrip() {
    let crs = Crs::from_epsg(3032).unwrap();
    let (e, n) = crs.forward(80.0, -75.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 80.0).abs() < 1e-4);
    assert!((lat - -75.0).abs() < 1e-4);
}

#[test]
fn epsg_3413_arctic_polar_stereo_roundtrip() {
    let crs = Crs::from_epsg(3413).unwrap();
    let (e, n) = crs.forward(-45.0, 80.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -45.0).abs() < 1e-4);
    assert!((lat - 80.0).abs() < 1e-4);
}

#[test]
fn epsg_3976_antarctic_polar_stereo_variant_b_roundtrip() {
    let crs = Crs::from_epsg(3976).unwrap();
    let (e, n) = crs.forward(0.0, -75.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 0.0).abs() < 1e-4);
    assert!((lat - -75.0).abs() < 1e-4);
}

#[test]
fn epsg_3996_ibcao_polar_stereo_variant_b_roundtrip() {
    let crs = Crs::from_epsg(3996).unwrap();
    let (e, n) = crs.forward(0.0, 80.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 0.0).abs() < 1e-4);
    assert!((lat - 80.0).abs() < 1e-4);
}

#[test]
fn epsg_3410_ease_grid_global_roundtrip() {
    let crs = Crs::from_epsg(3410).unwrap();
    let (e, n) = crs.forward(20.0, 45.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 20.0).abs() < 1e-4);
    assert!((lat - 45.0).abs() < 1e-4);
}

#[test]
fn epsg_3408_ease_grid_north_roundtrip() {
    let crs = Crs::from_epsg(3408).unwrap();
    let (e, n) = crs.forward(20.0, 80.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 20.0).abs() < 1e-4);
    assert!((lat - 80.0).abs() < 1e-4);
}

#[test]
fn epsg_3409_ease_grid_south_roundtrip() {
    let crs = Crs::from_epsg(3409).unwrap();
    let (e, n) = crs.forward(20.0, -80.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 20.0).abs() < 1e-4);
    assert!((lat - -80.0).abs() < 1e-4);
}

#[test]
fn epsg_6931_ease_grid2_north_roundtrip() {
    let crs = Crs::from_epsg(6931).unwrap();
    let (e, n) = crs.forward(20.0, 80.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 20.0).abs() < 1e-4);
    assert!((lat - 80.0).abs() < 1e-4);
}

#[test]
fn epsg_6932_ease_grid2_south_roundtrip() {
    let crs = Crs::from_epsg(6932).unwrap();
    let (e, n) = crs.forward(20.0, -80.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 20.0).abs() < 1e-4);
    assert!((lat - -80.0).abs() < 1e-4);
}

#[test]
fn epsg_6933_ease_grid2_global_roundtrip() {
    let crs = Crs::from_epsg(6933).unwrap();
    let (e, n) = crs.forward(20.0, 45.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 20.0).abs() < 1e-4);
    assert!((lat - 45.0).abs() < 1e-4);
    assert!(e.is_finite() && n.is_finite());
}

#[test]
fn epsg_8857_equal_earth_roundtrip() {
    let crs = Crs::from_epsg(8857).unwrap();
    let (e, n) = crs.forward(13.4, 52.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 13.4).abs() < 1e-4);
    assert!((lat - 52.5).abs() < 1e-4);
    assert!(e.is_finite() && n.is_finite());
}

#[test]
fn epsg_3395_world_mercator() {
    let crs = Crs::from_epsg(3395).unwrap();
    let (e, n) = crs.forward(13.4, 52.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 13.4).abs() < TOL);
    assert!((lat - 52.5).abs() < TOL);
}

#[test]
fn epsg_3400_alberta_10tm_forest_roundtrip() {
    let crs = Crs::from_epsg(3400).unwrap();
    let (e, n) = crs.forward(-114.0, 53.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -114.0).abs() < 1e-4);
    assert!((lat - 53.0).abs() < 1e-4);
}

#[test]
fn epsg_3401_alberta_10tm_resource_roundtrip() {
    let crs = Crs::from_epsg(3401).unwrap();
    let (e, n) = crs.forward(-114.0, 53.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -114.0).abs() < 1e-4);
    assert!((lat - 53.0).abs() < 1e-4);
}

#[test]
fn epsg_3402_alberta_10tm_csrs_forest_roundtrip() {
    let crs = Crs::from_epsg(3402).unwrap();
    let (e, n) = crs.forward(-114.0, 53.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -114.0).abs() < 1e-4);
    assert!((lat - 53.0).abs() < 1e-4);
}

#[test]
fn epsg_3403_alberta_10tm_roundtrip() {
    let crs = Crs::from_epsg(3403).unwrap();
    let (e, n) = crs.forward(-114.0, 53.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -114.0).abs() < 1e-4);
    assert!((lat - 53.0).abs() < 1e-4);
}

#[test]
fn epsg_3405_vn2000_utm48n_roundtrip() {
    let crs = Crs::from_epsg(3405).unwrap();
    let (e, n) = crs.forward(106.0, 16.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 106.0).abs() < 1e-4);
    assert!((lat - 16.0).abs() < 1e-4);
}

#[test]
fn epsg_3406_vn2000_utm49n_roundtrip() {
    let crs = Crs::from_epsg(3406).unwrap();
    let (e, n) = crs.forward(108.5, 16.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 108.5).abs() < 1e-4);
    assert!((lat - 16.0).abs() < 1e-4);
}

#[test]
fn epsg_3986_katanga_gauss_a_roundtrip() {
    let crs = Crs::from_epsg(3986).unwrap();
    let (e, n) = crs.forward(29.0, -10.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 29.0).abs() < 1e-4);
    assert!((lat - -10.0).abs() < 1e-4);
}

#[test]
fn epsg_3987_katanga_gauss_b_roundtrip() {
    let crs = Crs::from_epsg(3987).unwrap();
    let (e, n) = crs.forward(28.0, -10.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 28.0).abs() < 1e-4);
    assert!((lat - -10.0).abs() < 1e-4);
}

#[test]
fn epsg_3988_katanga_gauss_c_roundtrip() {
    let crs = Crs::from_epsg(3988).unwrap();
    let (e, n) = crs.forward(26.0, -10.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 26.0).abs() < 1e-4);
    assert!((lat - -10.0).abs() < 1e-4);
}

#[test]
fn epsg_3989_katanga_gauss_d_roundtrip() {
    let crs = Crs::from_epsg(3989).unwrap();
    let (e, n) = crs.forward(24.0, -10.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 24.0).abs() < 1e-4);
    assert!((lat - -10.0).abs() < 1e-4);
}

#[test]
fn epsg_3997_dubai_local_tm_roundtrip() {
    let crs = Crs::from_epsg(3997).unwrap();
    let (e, n) = crs.forward(55.3, 25.2).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 55.3).abs() < 1e-4);
    assert!((lat - 25.2).abs() < 1e-4);
}

#[test]
fn epsg_3994_mercator_41_roundtrip() {
    let crs = Crs::from_epsg(3994).unwrap();
    let (e, n) = crs.forward(170.0, -40.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 170.0).abs() < 1e-4);
    assert!((lat - -40.0).abs() < 1e-4);
}

#[test]
fn epsg_3991_puerto_rico_cs27_roundtrip() {
    let crs = Crs::from_epsg(3991).unwrap();
    let (e, n) = crs.forward(-66.1, 18.2).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -66.1).abs() < 1e-4);
    assert!((lat - 18.2).abs() < 1e-4);
}

#[test]
fn epsg_3992_puerto_rico_st_croix_roundtrip() {
    let crs = Crs::from_epsg(3992).unwrap();
    let (e, n) = crs.forward(-64.8, 17.75).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -64.8).abs() < 1e-4);
    assert!((lat - 17.75).abs() < 1e-4);
}

#[test]
fn epsg_4087_world_equidistant_cylindrical_roundtrip() {
    let crs = Crs::from_epsg(4087).unwrap();
    let (e, n) = crs.forward(13.4, 52.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 13.4).abs() < 1e-6);
    assert!((lat - 52.5).abs() < 1e-6);
}

#[test]
fn esri_54008_world_sinusoidal_roundtrip() {
    let crs = Crs::from_epsg(54008).unwrap();
    let (e, n) = crs.forward(13.4, 52.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 13.4).abs() < 1e-4);
    assert!((lat - 52.5).abs() < 1e-4);
}

#[test]
fn esri_54009_world_mollweide_roundtrip() {
    let crs = Crs::from_epsg(54009).unwrap();
    let (e, n) = crs.forward(13.4, 52.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 13.4).abs() < 1e-4);
    assert!((lat - 52.5).abs() < 1e-4);
}

#[test]
fn esri_54030_world_robinson_roundtrip() {
    let crs = Crs::from_epsg(54030).unwrap();
    let (e, n) = crs.forward(13.4, 52.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 13.4).abs() < 1e-4);
    assert!((lat - 52.5).abs() < 1e-4);
}

#[test]
fn epsg_28354_gda94_mga54() {
    let crs = Crs::from_epsg(28354).unwrap();
    let (e, n) = crs.forward(141.0, -33.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 141.0).abs() < TOL);
    assert!((lat - -33.0).abs() < TOL);
}

#[test]
fn epsg_31467_gauss_kruger_zone3() {
    let crs = Crs::from_epsg(31467).unwrap();
    let (e, n) = crs.forward(9.0, 48.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 9.0).abs() < TOL);
    assert!((lat - 48.0).abs() < TOL);
}

#[test]
fn epsg_5070_conus_albers_roundtrip() {
    let crs = Crs::from_epsg(5070).unwrap();
    let (e, n) = crs.forward(-96.0, 39.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -96.0).abs() < 1e-4);
    assert!((lat - 39.0).abs() < 1e-4);
}

#[test]
fn epsg_3577_australian_albers_roundtrip() {
    let crs = Crs::from_epsg(3577).unwrap();
    let (e, n) = crs.forward(132.0, -25.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 132.0).abs() < 1e-4);
    assert!((lat - -25.0).abs() < 1e-4);
}

#[test]
fn epsg_3578_yukon_albers_roundtrip() {
    let crs = Crs::from_epsg(3578).unwrap();
    let (e, n) = crs.forward(-135.0, 64.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -135.0).abs() < 1e-4);
    assert!((lat - 64.0).abs() < 1e-4);
}

#[test]
fn epsg_3579_yukon_albers_csrs_roundtrip() {
    let crs = Crs::from_epsg(3579).unwrap();
    let (e, n) = crs.forward(-135.0, 64.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -135.0).abs() < 1e-4);
    assert!((lat - 64.0).abs() < 1e-4);
}

#[test]
fn epsg_3575_north_pole_laea_europe_roundtrip() {
    let crs = Crs::from_epsg(3575).unwrap();
    let (e, n) = crs.forward(10.0, 70.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 10.0).abs() < 1e-4);
    assert!((lat - 70.0).abs() < 1e-4);
}

#[test]
fn epsg_3576_north_pole_laea_russia_roundtrip() {
    let crs = Crs::from_epsg(3576).unwrap();
    let (e, n) = crs.forward(90.0, 70.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 90.0).abs() < 1e-4);
    assert!((lat - 70.0).abs() < 1e-4);
}

#[test]
fn epsg_3571_north_pole_laea_bering_roundtrip() {
    let crs = Crs::from_epsg(3571).unwrap();
    let (e, n) = crs.forward(170.0, 70.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 170.0).abs() < 1e-4);
    assert!((lat - 70.0).abs() < 1e-4);
}

#[test]
fn epsg_3572_north_pole_laea_alaska_roundtrip() {
    let crs = Crs::from_epsg(3572).unwrap();
    let (e, n) = crs.forward(-150.0, 70.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -150.0).abs() < 1e-4);
    assert!((lat - 70.0).abs() < 1e-4);
}

#[test]
fn epsg_3573_north_pole_laea_canada_roundtrip() {
    let crs = Crs::from_epsg(3573).unwrap();
    let (e, n) = crs.forward(-100.0, 70.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -100.0).abs() < 1e-4);
    assert!((lat - 70.0).abs() < 1e-4);
}

#[test]
fn epsg_3574_north_pole_laea_atlantic_roundtrip() {
    let crs = Crs::from_epsg(3574).unwrap();
    let (e, n) = crs.forward(-40.0, 70.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -40.0).abs() < 1e-4);
    assert!((lat - 70.0).abs() < 1e-4);
}

#[test]
fn epsg_3832_pdc_mercator_roundtrip() {
    let crs = Crs::from_epsg(3832).unwrap();
    let (e, n) = crs.forward(160.0, 10.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 160.0).abs() < 1e-4);
    assert!((lat - 10.0).abs() < 1e-4);
}

#[test]
fn epsg_3833_pulkovo_gk_zone2_roundtrip() {
    let crs = Crs::from_epsg(3833).unwrap();
    let (e, n) = crs.forward(10.0, 52.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 10.0).abs() < 1e-4);
    assert!((lat - 52.0).abs() < 1e-4);
}

#[test]
fn epsg_3834_pulkovo83_gk_zone2_roundtrip() {
    let crs = Crs::from_epsg(3834).unwrap();
    let (e, n) = crs.forward(10.0, 52.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 10.0).abs() < 1e-4);
    assert!((lat - 52.0).abs() < 1e-4);
}

#[test]
fn epsg_3835_pulkovo83_gk_zone3_roundtrip() {
    let crs = Crs::from_epsg(3835).unwrap();
    let (e, n) = crs.forward(15.5, 48.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 15.5).abs() < 1e-4);
    assert!((lat - 48.0).abs() < 1e-4);
}

#[test]
fn epsg_3836_pulkovo83_gk_zone4_roundtrip() {
    let crs = Crs::from_epsg(3836).unwrap();
    let (e, n) = crs.forward(21.5, 48.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 21.5).abs() < 1e-4);
    assert!((lat - 48.0).abs() < 1e-4);
}

#[test]
fn epsg_3837_pulkovo58_3deg_gk_zone3_roundtrip() {
    let crs = Crs::from_epsg(3837).unwrap();
    let (e, n) = crs.forward(10.0, 51.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 10.0).abs() < 1e-4);
    assert!((lat - 51.0).abs() < 1e-4);
}

#[test]
fn epsg_3838_pulkovo58_3deg_gk_zone4_roundtrip() {
    let crs = Crs::from_epsg(3838).unwrap();
    let (e, n) = crs.forward(12.5, 51.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 12.5).abs() < 1e-4);
    assert!((lat - 51.0).abs() < 1e-4);
}

#[test]
fn epsg_3839_pulkovo58_3deg_gk_zone9_roundtrip() {
    let crs = Crs::from_epsg(3839).unwrap();
    let (e, n) = crs.forward(27.5, 45.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 27.5).abs() < 1e-4);
    assert!((lat - 45.0).abs() < 1e-4);
}

#[test]
fn epsg_3840_pulkovo58_3deg_gk_zone10_roundtrip() {
    let crs = Crs::from_epsg(3840).unwrap();
    let (e, n) = crs.forward(29.0, 44.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 29.0).abs() < 1e-4);
    assert!((lat - 44.0).abs() < 1e-4);
}

#[test]
fn epsg_3841_pulkovo83_3deg_gk_zone6_roundtrip() {
    let crs = Crs::from_epsg(3841).unwrap();
    let (e, n) = crs.forward(18.5, 48.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 18.5).abs() < 1e-4);
    assert!((lat - 48.0).abs() < 1e-4);
}

#[test]
fn epsg_3845_rt90_7_5_v_roundtrip() {
    let crs = Crs::from_epsg(3845).unwrap();
    let (e, n) = crs.forward(12.0, 58.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 12.0).abs() < 1e-4);
    assert!((lat - 58.0).abs() < 1e-4);
}

#[test]
fn epsg_3846_rt90_5_v_roundtrip() {
    let crs = Crs::from_epsg(3846).unwrap();
    let (e, n) = crs.forward(14.0, 60.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 14.0).abs() < 1e-4);
    assert!((lat - 60.0).abs() < 1e-4);
}

#[test]
fn epsg_3847_rt90_2_5_v_roundtrip() {
    let crs = Crs::from_epsg(3847).unwrap();
    let (e, n) = crs.forward(16.0, 61.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 16.0).abs() < 1e-4);
    assert!((lat - 61.0).abs() < 1e-4);
}

#[test]
fn epsg_3848_rt90_0_roundtrip() {
    let crs = Crs::from_epsg(3848).unwrap();
    let (e, n) = crs.forward(18.1, 62.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 18.1).abs() < 1e-4);
    assert!((lat - 62.0).abs() < 1e-4);
}

#[test]
fn epsg_3849_rt90_2_5_o_roundtrip() {
    let crs = Crs::from_epsg(3849).unwrap();
    let (e, n) = crs.forward(20.5, 65.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 20.5).abs() < 1e-4);
    assert!((lat - 65.0).abs() < 1e-4);
}

#[test]
fn epsg_3850_rt90_5_o_roundtrip() {
    let crs = Crs::from_epsg(3850).unwrap();
    let (e, n) = crs.forward(22.8, 66.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 22.8).abs() < 1e-4);
    assert!((lat - 66.0).abs() < 1e-4);
}

#[test]
fn epsg_2443_jgd2000_japan_plane_cs1_roundtrip() {
    let crs = Crs::from_epsg(2443).unwrap();
    let (e, n) = crs.forward(130.0, 33.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 130.0).abs() < 1e-4);
    assert!((lat - 33.5).abs() < 1e-4);
}

#[test]
fn epsg_2444_jgd2000_japan_plane_cs2_roundtrip() {
    let crs = Crs::from_epsg(2444).unwrap();
    let (e, n) = crs.forward(131.5, 33.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 131.5).abs() < 1e-4);
    assert!((lat - 33.5).abs() < 1e-4);
}

#[test]
fn epsg_2445_jgd2000_japan_plane_cs3_roundtrip() {
    let crs = Crs::from_epsg(2445).unwrap();
    let (e, n) = crs.forward(132.5, 36.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 132.5).abs() < 1e-4);
    assert!((lat - 36.5).abs() < 1e-4);
}

#[test]
fn epsg_2446_jgd2000_japan_plane_cs4_roundtrip() {
    let crs = Crs::from_epsg(2446).unwrap();
    let (e, n) = crs.forward(134.0, 34.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 134.0).abs() < 1e-4);
    assert!((lat - 34.0).abs() < 1e-4);
}

#[test]
fn epsg_2447_jgd2000_japan_plane_cs5_roundtrip() {
    let crs = Crs::from_epsg(2447).unwrap();
    let (e, n) = crs.forward(135.0, 36.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 135.0).abs() < 1e-4);
    assert!((lat - 36.5).abs() < 1e-4);
}

#[test]
fn epsg_2448_jgd2000_japan_plane_cs6_roundtrip() {
    let crs = Crs::from_epsg(2448).unwrap();
    let (e, n) = crs.forward(136.5, 35.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 136.5).abs() < 1e-4);
    assert!((lat - 35.5).abs() < 1e-4);
}

#[test]
fn epsg_2449_jgd2000_japan_plane_cs7_roundtrip() {
    let crs = Crs::from_epsg(2449).unwrap();
    let (e, n) = crs.forward(137.5, 36.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 137.5).abs() < 1e-4);
    assert!((lat - 36.5).abs() < 1e-4);
}

#[test]
fn epsg_2450_jgd2000_japan_plane_cs8_roundtrip() {
    let crs = Crs::from_epsg(2450).unwrap();
    let (e, n) = crs.forward(139.0, 37.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 139.0).abs() < 1e-4);
    assert!((lat - 37.0).abs() < 1e-4);
}

#[test]
fn epsg_2451_jgd2000_japan_plane_cs9_roundtrip() {
    let crs = Crs::from_epsg(2451).unwrap();
    let (e, n) = crs.forward(140.0, 36.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 140.0).abs() < 1e-4);
    assert!((lat - 36.5).abs() < 1e-4);
}

#[test]
fn epsg_2452_jgd2000_japan_plane_cs10_roundtrip() {
    let crs = Crs::from_epsg(2452).unwrap();
    let (e, n) = crs.forward(141.0, 40.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 141.0).abs() < 1e-4);
    assert!((lat - 40.5).abs() < 1e-4);
}

#[test]
fn epsg_2453_jgd2000_japan_plane_cs11_roundtrip() {
    let crs = Crs::from_epsg(2453).unwrap();
    let (e, n) = crs.forward(141.0, 44.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 141.0).abs() < 1e-4);
    assert!((lat - 44.5).abs() < 1e-4);
}

#[test]
fn epsg_2454_jgd2000_japan_plane_cs12_roundtrip() {
    let crs = Crs::from_epsg(2454).unwrap();
    let (e, n) = crs.forward(143.0, 44.2).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 143.0).abs() < 1e-4);
    assert!((lat - 44.2).abs() < 1e-4);
}

#[test]
fn epsg_2455_jgd2000_japan_plane_cs13_roundtrip() {
    let crs = Crs::from_epsg(2455).unwrap();
    let (e, n) = crs.forward(145.0, 44.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 145.0).abs() < 1e-4);
    assert!((lat - 44.5).abs() < 1e-4);
}

#[test]
fn epsg_2456_jgd2000_japan_plane_cs14_roundtrip() {
    let crs = Crs::from_epsg(2456).unwrap();
    let (e, n) = crs.forward(142.2, 26.3).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 142.2).abs() < 1e-4);
    assert!((lat - 26.3).abs() < 1e-4);
}

#[test]
fn epsg_2457_jgd2000_japan_plane_cs15_roundtrip() {
    let crs = Crs::from_epsg(2457).unwrap();
    let (e, n) = crs.forward(127.8, 26.4).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 127.8).abs() < 1e-4);
    assert!((lat - 26.4).abs() < 1e-4);
}

#[test]
fn epsg_2458_jgd2000_japan_plane_cs16_roundtrip() {
    let crs = Crs::from_epsg(2458).unwrap();
    let (e, n) = crs.forward(124.5, 26.2).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 124.5).abs() < 1e-4);
    assert!((lat - 26.2).abs() < 1e-4);
}

#[test]
fn epsg_2459_jgd2000_japan_plane_cs17_roundtrip() {
    let crs = Crs::from_epsg(2459).unwrap();
    let (e, n) = crs.forward(131.2, 26.2).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 131.2).abs() < 1e-4);
    assert!((lat - 26.2).abs() < 1e-4);
}

#[test]
fn epsg_2460_jgd2000_japan_plane_cs18_roundtrip() {
    let crs = Crs::from_epsg(2460).unwrap();
    let (e, n) = crs.forward(136.3, 20.4).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 136.3).abs() < 1e-4);
    assert!((lat - 20.4).abs() < 1e-4);
}

#[test]
fn epsg_2461_jgd2000_japan_plane_cs19_roundtrip() {
    let crs = Crs::from_epsg(2461).unwrap();
    let (e, n) = crs.forward(154.3, 26.2).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 154.3).abs() < 1e-4);
    assert!((lat - 26.2).abs() < 1e-4);
}

#[test]
fn epsg_6672_jgd2011_japan_plane_cs4_roundtrip() {
    let crs = Crs::from_epsg(6672).unwrap();
    let (e, n) = crs.forward(134.0, 34.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 134.0).abs() < 1e-4);
    assert!((lat - 34.0).abs() < 1e-4);
}

#[test]
fn epsg_6673_jgd2011_japan_plane_cs5_roundtrip() {
    let crs = Crs::from_epsg(6673).unwrap();
    let (e, n) = crs.forward(135.0, 36.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 135.0).abs() < 1e-4);
    assert!((lat - 36.5).abs() < 1e-4);
}

#[test]
fn epsg_6674_jgd2011_japan_plane_cs6_roundtrip() {
    let crs = Crs::from_epsg(6674).unwrap();
    let (e, n) = crs.forward(136.5, 35.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 136.5).abs() < 1e-4);
    assert!((lat - 35.5).abs() < 1e-4);
}

#[test]
fn epsg_6675_jgd2011_japan_plane_cs7_roundtrip() {
    let crs = Crs::from_epsg(6675).unwrap();
    let (e, n) = crs.forward(137.5, 36.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 137.5).abs() < 1e-4);
    assert!((lat - 36.5).abs() < 1e-4);
}

#[test]
fn epsg_6676_jgd2011_japan_plane_cs8_roundtrip() {
    let crs = Crs::from_epsg(6676).unwrap();
    let (e, n) = crs.forward(139.0, 37.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 139.0).abs() < 1e-4);
    assert!((lat - 37.0).abs() < 1e-4);
}

#[test]
fn epsg_6677_jgd2011_japan_plane_cs9_roundtrip() {
    let crs = Crs::from_epsg(6677).unwrap();
    let (e, n) = crs.forward(140.0, 36.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 140.0).abs() < 1e-4);
    assert!((lat - 36.5).abs() < 1e-4);
}

#[test]
fn epsg_6678_jgd2011_japan_plane_cs10_roundtrip() {
    let crs = Crs::from_epsg(6678).unwrap();
    let (e, n) = crs.forward(141.0, 40.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 141.0).abs() < 1e-4);
    assert!((lat - 40.5).abs() < 1e-4);
}

#[test]
fn epsg_6679_jgd2011_japan_plane_cs11_roundtrip() {
    let crs = Crs::from_epsg(6679).unwrap();
    let (e, n) = crs.forward(141.0, 44.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 141.0).abs() < 1e-4);
    assert!((lat - 44.5).abs() < 1e-4);
}

#[test]
fn epsg_6680_jgd2011_japan_plane_cs12_roundtrip() {
    let crs = Crs::from_epsg(6680).unwrap();
    let (e, n) = crs.forward(143.0, 44.2).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 143.0).abs() < 1e-4);
    assert!((lat - 44.2).abs() < 1e-4);
}

#[test]
fn epsg_6681_jgd2011_japan_plane_cs13_roundtrip() {
    let crs = Crs::from_epsg(6681).unwrap();
    let (e, n) = crs.forward(145.0, 44.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 145.0).abs() < 1e-4);
    assert!((lat - 44.5).abs() < 1e-4);
}

#[test]
fn epsg_6682_jgd2011_japan_plane_cs14_roundtrip() {
    let crs = Crs::from_epsg(6682).unwrap();
    let (e, n) = crs.forward(142.2, 26.3).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 142.2).abs() < 1e-4);
    assert!((lat - 26.3).abs() < 1e-4);
}

#[test]
fn epsg_6683_jgd2011_japan_plane_cs15_roundtrip() {
    let crs = Crs::from_epsg(6683).unwrap();
    let (e, n) = crs.forward(127.8, 26.4).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 127.8).abs() < 1e-4);
    assert!((lat - 26.4).abs() < 1e-4);
}

#[test]
fn epsg_6684_jgd2011_japan_plane_cs16_roundtrip() {
    let crs = Crs::from_epsg(6684).unwrap();
    let (e, n) = crs.forward(124.5, 26.2).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 124.5).abs() < 1e-4);
    assert!((lat - 26.2).abs() < 1e-4);
}

#[test]
fn epsg_6685_jgd2011_japan_plane_cs17_roundtrip() {
    let crs = Crs::from_epsg(6685).unwrap();
    let (e, n) = crs.forward(131.2, 26.2).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 131.2).abs() < 1e-4);
    assert!((lat - 26.2).abs() < 1e-4);
}

#[test]
fn epsg_6686_jgd2011_japan_plane_cs18_roundtrip() {
    let crs = Crs::from_epsg(6686).unwrap();
    let (e, n) = crs.forward(136.3, 20.4).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 136.3).abs() < 1e-4);
    assert!((lat - 20.4).abs() < 1e-4);
}

#[test]
fn epsg_6687_jgd2011_japan_plane_cs19_roundtrip() {
    let crs = Crs::from_epsg(6687).unwrap();
    let (e, n) = crs.forward(154.3, 26.2).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 154.3).abs() < 1e-4);
    assert!((lat - 26.2).abs() < 1e-4);
}

#[test]
fn epsg_6688_jgd2011_utm51n_roundtrip() {
    let crs = Crs::from_epsg(6688).unwrap();
    let (e, n) = crs.forward(123.5, 25.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 123.5).abs() < 1e-4);
    assert!((lat - 25.0).abs() < 1e-4);
}

#[test]
fn epsg_6689_jgd2011_utm52n_roundtrip() {
    let crs = Crs::from_epsg(6689).unwrap();
    let (e, n) = crs.forward(129.5, 32.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 129.5).abs() < 1e-4);
    assert!((lat - 32.0).abs() < 1e-4);
}

#[test]
fn epsg_6690_jgd2011_utm53n_roundtrip() {
    let crs = Crs::from_epsg(6690).unwrap();
    let (e, n) = crs.forward(135.5, 35.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 135.5).abs() < 1e-4);
    assert!((lat - 35.0).abs() < 1e-4);
}

#[test]
fn epsg_6691_jgd2011_utm54n_roundtrip() {
    let crs = Crs::from_epsg(6691).unwrap();
    let (e, n) = crs.forward(141.5, 38.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 141.5).abs() < 1e-4);
    assert!((lat - 38.0).abs() < 1e-4);
}

#[test]
fn epsg_6692_jgd2011_utm55n_roundtrip() {
    let crs = Crs::from_epsg(6692).unwrap();
    let (e, n) = crs.forward(147.5, 43.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 147.5).abs() < 1e-4);
    assert!((lat - 43.0).abs() < 1e-4);
}

#[test]
fn epsg_6707_rdn2008_utm32n_roundtrip() {
    let crs = Crs::from_epsg(6707).unwrap();
    let (e, n) = crs.forward(9.5, 44.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 9.5).abs() < 1e-4);
    assert!((lat - 44.0).abs() < 1e-4);
}

#[test]
fn epsg_6708_rdn2008_utm33n_roundtrip() {
    let crs = Crs::from_epsg(6708).unwrap();
    let (e, n) = crs.forward(15.5, 42.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 15.5).abs() < 1e-4);
    assert!((lat - 42.5).abs() < 1e-4);
}

#[test]
fn epsg_6709_rdn2008_utm34n_roundtrip() {
    let crs = Crs::from_epsg(6709).unwrap();
    let (e, n) = crs.forward(21.5, 41.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 21.5).abs() < 1e-4);
    assert!((lat - 41.5).abs() < 1e-4);
}

#[test]
fn epsg_6732_gda94_mga41_roundtrip() {
    let crs = Crs::from_epsg(6732).unwrap();
    let (e, n) = crs.forward(63.5, -20.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 63.5).abs() < 1e-4);
    assert!((lat - -20.0).abs() < 1e-4);
}

#[test]
fn epsg_6733_gda94_mga42_roundtrip() {
    let crs = Crs::from_epsg(6733).unwrap();
    let (e, n) = crs.forward(69.5, -20.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 69.5).abs() < 1e-4);
    assert!((lat - -20.0).abs() < 1e-4);
}

#[test]
fn epsg_6734_gda94_mga43_roundtrip() {
    let crs = Crs::from_epsg(6734).unwrap();
    let (e, n) = crs.forward(75.5, -20.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 75.5).abs() < 1e-4);
    assert!((lat - -20.0).abs() < 1e-4);
}

#[test]
fn epsg_6735_gda94_mga44_roundtrip() {
    let crs = Crs::from_epsg(6735).unwrap();
    let (e, n) = crs.forward(81.5, -20.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 81.5).abs() < 1e-4);
    assert!((lat - -20.0).abs() < 1e-4);
}

#[test]
fn epsg_6736_gda94_mga46_roundtrip() {
    let crs = Crs::from_epsg(6736).unwrap();
    let (e, n) = crs.forward(93.5, -20.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 93.5).abs() < 1e-4);
    assert!((lat - -20.0).abs() < 1e-4);
}

#[test]
fn epsg_6737_gda94_mga47_roundtrip() {
    let crs = Crs::from_epsg(6737).unwrap();
    let (e, n) = crs.forward(99.5, -20.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 99.5).abs() < 1e-4);
    assert!((lat - -20.0).abs() < 1e-4);
}

#[test]
fn epsg_6738_gda94_mga59_roundtrip() {
    let crs = Crs::from_epsg(6738).unwrap();
    let (e, n) = crs.forward(171.5, -20.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 171.5).abs() < 1e-4);
    assert!((lat - -20.0).abs() < 1e-4);
}

#[test]
fn epsg_6784_oregon_baker_cors96_m_roundtrip() {
    let crs = Crs::from_epsg(6784).unwrap();
    let (e, n) = crs.forward(-117.5, 44.6).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -117.5).abs() < 1e-4);
    assert!((lat - 44.6).abs() < 1e-4);
}

#[test]
fn epsg_6786_oregon_baker_2011_m_roundtrip() {
    let crs = Crs::from_epsg(6786).unwrap();
    let (e, n) = crs.forward(-117.5, 44.6).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -117.5).abs() < 1e-4);
    assert!((lat - 44.6).abs() < 1e-4);
}

#[test]
fn epsg_6788_oregon_bend_klamath_cors96_m_roundtrip() {
    let crs = Crs::from_epsg(6788).unwrap();
    let (e, n) = crs.forward(-121.4, 42.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -121.4).abs() < 1e-4);
    assert!((lat - 42.0).abs() < 1e-4);
}

#[test]
fn epsg_6790_oregon_bend_klamath_2011_m_roundtrip() {
    let crs = Crs::from_epsg(6790).unwrap();
    let (e, n) = crs.forward(-121.4, 42.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -121.4).abs() < 1e-4);
    assert!((lat - 42.0).abs() < 1e-4);
}

#[test]
fn epsg_6800_oregon_canyonville_gp_cors96_m_roundtrip() {
    let crs = Crs::from_epsg(6800).unwrap();
    let (e, n) = crs.forward(-123.0, 42.6).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -123.0).abs() < 1e-4);
    assert!((lat - 42.6).abs() < 1e-4);
}

#[test]
fn epsg_6802_oregon_canyonville_gp_2011_m_roundtrip() {
    let crs = Crs::from_epsg(6802).unwrap();
    let (e, n) = crs.forward(-123.0, 42.6).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -123.0).abs() < 1e-4);
    assert!((lat - 42.6).abs() < 1e-4);
}

#[test]
fn epsg_6812_oregon_cottage_canyonville_cors96_m_roundtrip() {
    let crs = Crs::from_epsg(6812).unwrap();
    let (e, n) = crs.forward(-123.0, 43.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -123.0).abs() < 1e-4);
    assert!((lat - 43.0).abs() < 1e-4);
}

#[test]
fn epsg_6814_oregon_cottage_canyonville_2011_m_roundtrip() {
    let crs = Crs::from_epsg(6814).unwrap();
    let (e, n) = crs.forward(-123.0, 43.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -123.0).abs() < 1e-4);
    assert!((lat - 43.0).abs() < 1e-4);
}

#[test]
fn epsg_6816_oregon_dufur_madras_cors96_m_roundtrip() {
    let crs = Crs::from_epsg(6816).unwrap();
    let (e, n) = crs.forward(-121.0, 44.6).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -121.0).abs() < 1e-4);
    assert!((lat - 44.6).abs() < 1e-4);
}

#[test]
fn epsg_6818_oregon_dufur_madras_2011_m_roundtrip() {
    let crs = Crs::from_epsg(6818).unwrap();
    let (e, n) = crs.forward(-121.0, 44.6).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -121.0).abs() < 1e-4);
    assert!((lat - 44.6).abs() < 1e-4);
}

#[test]
fn epsg_6820_oregon_eugene_cors96_m_roundtrip() {
    let crs = Crs::from_epsg(6820).unwrap();
    let (e, n) = crs.forward(-123.0, 43.8).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -123.0).abs() < 1e-4);
    assert!((lat - 43.8).abs() < 1e-4);
}

#[test]
fn epsg_6822_oregon_eugene_2011_m_roundtrip() {
    let crs = Crs::from_epsg(6822).unwrap();
    let (e, n) = crs.forward(-123.0, 43.8).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -123.0).abs() < 1e-4);
    assert!((lat - 43.8).abs() < 1e-4);
}

#[test]
fn epsg_6824_oregon_grants_pass_ashland_cors96_m_roundtrip() {
    let crs = Crs::from_epsg(6824).unwrap();
    let (e, n) = crs.forward(-123.0, 41.9).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -123.0).abs() < 1e-4);
    assert!((lat - 41.9).abs() < 1e-4);
}

#[test]
fn epsg_6826_oregon_grants_pass_ashland_2011_m_roundtrip() {
    let crs = Crs::from_epsg(6826).unwrap();
    let (e, n) = crs.forward(-123.0, 41.9).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -123.0).abs() < 1e-4);
    assert!((lat - 41.9).abs() < 1e-4);
}

#[test]
fn epsg_6828_oregon_gresham_warm_springs_cors96_m_roundtrip() {
    let crs = Crs::from_epsg(6828).unwrap();
    let (e, n) = crs.forward(-122.2, 45.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -122.2).abs() < 1e-4);
    assert!((lat - 45.0).abs() < 1e-4);
}

#[test]
fn epsg_6830_oregon_gresham_warm_springs_2011_m_roundtrip() {
    let crs = Crs::from_epsg(6830).unwrap();
    let (e, n) = crs.forward(-122.2, 45.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -122.2).abs() < 1e-4);
    assert!((lat - 45.0).abs() < 1e-4);
}

#[test]
fn epsg_6832_oregon_la_grande_cors96_m_roundtrip() {
    let crs = Crs::from_epsg(6832).unwrap();
    let (e, n) = crs.forward(-118.0, 45.1).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -118.0).abs() < 1e-4);
    assert!((lat - 45.1).abs() < 1e-4);
}

#[test]
fn epsg_6834_oregon_la_grande_2011_m_roundtrip() {
    let crs = Crs::from_epsg(6834).unwrap();
    let (e, n) = crs.forward(-118.0, 45.1).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -118.0).abs() < 1e-4);
    assert!((lat - 45.1).abs() < 1e-4);
}

#[test]
fn epsg_6836_oregon_ontario_cors96_m_roundtrip() {
    let crs = Crs::from_epsg(6836).unwrap();
    let (e, n) = crs.forward(-117.0, 43.3).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -117.0).abs() < 1e-4);
    assert!((lat - 43.3).abs() < 1e-4);
}

#[test]
fn epsg_6838_oregon_ontario_2011_m_roundtrip() {
    let crs = Crs::from_epsg(6838).unwrap();
    let (e, n) = crs.forward(-117.0, 43.3).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -117.0).abs() < 1e-4);
    assert!((lat - 43.3).abs() < 1e-4);
}

#[test]
fn epsg_6844_oregon_pendleton_cors96_m_roundtrip() {
    let crs = Crs::from_epsg(6844).unwrap();
    let (e, n) = crs.forward(-119.1, 45.3).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -119.1).abs() < 1e-4);
    assert!((lat - 45.3).abs() < 1e-4);
}

#[test]
fn epsg_6846_oregon_pendleton_2011_m_roundtrip() {
    let crs = Crs::from_epsg(6846).unwrap();
    let (e, n) = crs.forward(-119.1, 45.3).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -119.1).abs() < 1e-4);
    assert!((lat - 45.3).abs() < 1e-4);
}

#[test]
fn epsg_6848_oregon_pendleton_la_grande_cors96_m_roundtrip() {
    let crs = Crs::from_epsg(6848).unwrap();
    let (e, n) = crs.forward(-118.3, 45.1).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -118.3).abs() < 1e-4);
    assert!((lat - 45.1).abs() < 1e-4);
}

#[test]
fn epsg_6850_oregon_pendleton_la_grande_2011_m_roundtrip() {
    let crs = Crs::from_epsg(6850).unwrap();
    let (e, n) = crs.forward(-118.3, 45.1).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -118.3).abs() < 1e-4);
    assert!((lat - 45.1).abs() < 1e-4);
}

#[test]
fn epsg_6856_oregon_salem_cors96_m_roundtrip() {
    let crs = Crs::from_epsg(6856).unwrap();
    let (e, n) = crs.forward(-123.0, 44.4).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -123.0).abs() < 1e-4);
    assert!((lat - 44.4).abs() < 1e-4);
}

#[test]
fn epsg_6858_oregon_salem_2011_m_roundtrip() {
    let crs = Crs::from_epsg(6858).unwrap();
    let (e, n) = crs.forward(-123.0, 44.4).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -123.0).abs() < 1e-4);
    assert!((lat - 44.4).abs() < 1e-4);
}

#[test]
fn epsg_6860_oregon_santiam_pass_cors96_m_roundtrip() {
    let crs = Crs::from_epsg(6860).unwrap();
    let (e, n) = crs.forward(-122.5, 44.2).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -122.5).abs() < 1e-4);
    assert!((lat - 44.2).abs() < 1e-4);
}

#[test]
fn epsg_6862_oregon_santiam_pass_2011_m_roundtrip() {
    let crs = Crs::from_epsg(6862).unwrap();
    let (e, n) = crs.forward(-122.5, 44.2).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - -122.5).abs() < 1e-4);
    assert!((lat - 44.2).abs() < 1e-4);
}

#[test]
fn epsg_6870_albania_tm2010_roundtrip() {
    let crs = Crs::from_epsg(6870).unwrap();
    let (e, n) = crs.forward(20.2, 41.3).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 20.2).abs() < 1e-4);
    assert!((lat - 41.3).abs() < 1e-4);
}

#[test]
fn epsg_6875_italy_zone_ne_roundtrip() {
    let crs = Crs::from_epsg(6875).unwrap();
    let (e, n) = crs.forward(12.5, 43.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 12.5).abs() < 1e-4);
    assert!((lat - 43.5).abs() < 1e-4);
}

#[test]
fn epsg_6876_zone12_ne_roundtrip() {
    let crs = Crs::from_epsg(6876).unwrap();
    let (e, n) = crs.forward(12.5, 43.5).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 12.5).abs() < 1e-4);
    assert!((lat - 43.5).abs() < 1e-4);
}

#[test]
fn epsg_6915_sei1943_utm40n_roundtrip() {
    let crs = Crs::from_epsg(6915).unwrap();
    let (e, n) = crs.forward(57.5, 15.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 57.5).abs() < 1e-4);
    assert!((lat - 15.0).abs() < 1e-4);
}

#[test]
fn epsg_6927_svy21_singapore_tm_roundtrip() {
    let crs = Crs::from_epsg(6927).unwrap();
    let (e, n) = crs.forward(103.85, 1.30).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 103.85).abs() < 1e-4);
    assert!((lat - 1.30).abs() < 1e-4);
}

#[test]
fn epsg_6956_vn2000_tm3_zone481_roundtrip() {
    let crs = Crs::from_epsg(6956).unwrap();
    let (e, n) = crs.forward(102.2, 16.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 102.2).abs() < 1e-4);
    assert!((lat - 16.0).abs() < 1e-4);
}

#[test]
fn epsg_6957_vn2000_tm3_zone482_roundtrip() {
    let crs = Crs::from_epsg(6957).unwrap();
    let (e, n) = crs.forward(105.2, 16.0).unwrap();
    let (lon, lat) = crs.inverse(e, n).unwrap();
    assert!((lon - 105.2).abs() < 1e-4);
    assert!((lat - 16.0).abs() < 1e-4);
}

#[test]
fn epsg_ingcs_25_code_batch_roundtrip() {
    let checks: &[(u32, f64, f64)] = &[
        (7257, -84.9, 40.7), (7259, -85.0, 41.0), (7261, -85.8, 39.2), (7263, -87.2, 40.6),
        (7265, -85.3, 40.2), (7267, -86.4, 39.8), (7269, -86.2, 39.2), (7271, -86.6, 40.6),
        (7273, -86.3, 40.7), (7275, -85.5, 38.3), (7277, -87.1, 39.3), (7279, -86.5, 40.3),
        (7281, -86.4, 38.3), (7283, -87.0, 38.6), (7285, -84.8, 38.8), (7287, -85.6, 39.3),
        (7289, -84.9, 41.4), (7291, -86.8, 38.3), (7293, -85.7, 40.8), (7295, -84.9, 39.4),
        (7297, -87.2, 40.1), (7299, -86.2, 41.0), (7301, -87.5, 38.3), (7303, -85.6, 40.5),
        (7305, -85.9, 40.1),
    ];
    for (code, lon_in, lat_in) in checks {
        let crs = Crs::from_epsg(*code).unwrap();
        let (e, n) = crs.forward(*lon_in, *lat_in).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - *lon_in).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - *lat_in).abs() < 1e-4, "EPSG:{code} lat");
    }
}

#[test]
fn epsg_ingcs_second_25_code_batch_roundtrip() {
    let checks: &[(u32, f64, f64)] = &[
        (7307, -85.8, 39.9), (7309, -86.1, 38.2), (7311, -85.4, 39.9), (7313, -86.1, 40.5),
        (7315, -85.5, 40.9), (7317, -86.0, 38.9), (7319, -87.0, 41.1), (7321, -85.0, 40.4),
        (7323, -85.4, 38.7), (7325, -85.7, 39.0), (7327, -86.1, 39.6), (7329, -87.4, 38.6),
        (7331, -85.5, 41.5), (7333, -87.4, 41.1), (7335, -86.8, 41.2), (7337, -86.5, 39.2),
        (7339, -86.9, 39.8), (7341, -86.9, 39.3), (7343, -87.3, 39.8), (7345, -86.7, 38.0),
        (7347, -87.3, 38.1), (7349, -87.9, 38.0), (7351, -85.0, 40.0), (7353, -85.3, 39.1),
        (7355, -85.9, 39.5),
    ];
    for (code, lon_in, lat_in) in checks {
        let crs = Crs::from_epsg(*code).unwrap();
        let (e, n) = crs.forward(*lon_in, *lat_in).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - *lon_in).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - *lat_in).abs() < 1e-4, "EPSG:{code} lat");
    }
}

#[test]
fn epsg_iarcs_rmtcrs_sfcs13_25_code_batch_roundtrip() {
    let checks: &[(u32, f64, f64)] = &[
        (7057, -95.1, 43.3), (7058, -92.6, 43.3), (7059, -91.0, 40.4), (7060, -94.7, 42.7),
        (7061, -92.1, 42.8), (7062, -95.6, 40.4), (7063, -94.5, 40.4), (7064, -93.6, 40.4),
        (7065, -92.7, 40.4), (7066, -91.5, 41.9), (7067, -90.4, 40.4), (7068, -93.6, 41.0),
        (7069, -91.8, 40.4), (7070, -91.1, 40.4), (7109, -112.4, 48.6), (7110, -112.4, 48.1),
        (7111, -110.9, 48.6), (7112, -108.4, 48.6), (7113, -105.4, 48.4), (7114, -105.4, 48.4),
        (7115, -107.6, 44.9), (7116, -111.1, 46.4), (7117, -108.3, 45.9), (7118, -108.2, 42.8),
        (7131, -122.3, 37.8),
    ];
    for (code, lon_in, lat_in) in checks {
        let crs = Crs::from_epsg(*code).unwrap();
        let (e, n) = crs.forward(*lon_in, *lat_in).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - *lon_in).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - *lat_in).abs() < 1e-4, "EPSG:{code} lat");
    }
}

#[test]
fn epsg_ingcs_ftus_25_code_batch_roundtrip() {
    let checks: &[(u32, f64, f64)] = &[
        (7258, -84.9, 40.7), (7260, -85.0, 41.0), (7262, -85.8, 39.2), (7264, -87.2, 40.6),
        (7266, -85.3, 40.2), (7268, -86.4, 39.8), (7270, -86.2, 39.2), (7272, -86.6, 40.6),
        (7274, -86.3, 40.7), (7276, -85.5, 38.3), (7278, -87.1, 39.3), (7280, -86.5, 40.3),
        (7282, -86.4, 38.3), (7284, -87.0, 38.6), (7286, -84.8, 38.8), (7288, -85.6, 39.3),
        (7290, -84.9, 41.4), (7292, -86.8, 38.3), (7294, -85.7, 40.8), (7296, -84.9, 39.4),
        (7298, -87.2, 40.1), (7300, -86.2, 41.0), (7302, -87.5, 38.3), (7304, -85.6, 40.5),
        (7306, -85.9, 40.1),
    ];
    for (code, lon_in, lat_in) in checks {
        let crs = Crs::from_epsg(*code).unwrap();
        let (e, n) = crs.forward(*lon_in, *lat_in).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - *lon_in).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - *lat_in).abs() < 1e-4, "EPSG:{code} lat");
    }
}

#[test]
fn epsg_cgcs2000_global_impact_25_code_batch_roundtrip() {
    for code in 4491u32..=4501 {
        let lon0 = 75.0 + 6.0 * f64::from(code - 4491);
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon0 + 0.2, 35.0).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - (lon0 + 0.2)).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - 35.0).abs() < 1e-4, "EPSG:{code} lat");
    }

    for code in 4502u32..=4512 {
        let lon0 = 75.0 + 6.0 * f64::from(code - 4502);
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon0 + 0.2, 35.0).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - (lon0 + 0.2)).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - 35.0).abs() < 1e-4, "EPSG:{code} lat");
    }
}

#[test]
fn epsg_cgcs2000_sirgas2000_gda2020_geographics_map_to_expected_datums() {
    assert_eq!(Crs::from_epsg(4490).unwrap().datum.name, "CGCS2000");
    assert_eq!(Crs::from_epsg(4674).unwrap().datum.name, "SIRGAS2000");
    assert_eq!(Crs::from_epsg(7844).unwrap().datum.name, "GDA2020");
}

#[test]
fn epsg_cgcs2000_next_25_code_batch_roundtrip() {
    for code in 4513u32..=4533 {
        let lon0 = 75.0 + 3.0 * f64::from(code - 4513);
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon0 + 0.15, 35.0).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - (lon0 + 0.15)).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - 35.0).abs() < 1e-4, "EPSG:{code} lat");
    }

    for code in 4534u32..=4537 {
        let lon0 = 75.0 + 3.0 * f64::from(code - 4534);
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon0 + 0.15, 35.0).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - (lon0 + 0.15)).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - 35.0).abs() < 1e-4, "EPSG:{code} lat");
    }
}

#[test]
fn epsg_cgcs2000_cm_and_new_beijing_next_25_code_batch_roundtrip() {
    for code in 4538u32..=4554 {
        let lon0 = 87.0 + 3.0 * f64::from(code - 4538);
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon0 + 0.15, 35.0).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - (lon0 + 0.15)).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - 35.0).abs() < 1e-4, "EPSG:{code} lat");
    }

    for code in 4568u32..=4575 {
        let lon0 = 75.0 + 6.0 * f64::from(code - 4568);
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon0 + 0.2, 35.0).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - (lon0 + 0.2)).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - 35.0).abs() < 1e-4, "EPSG:{code} lat");
    }
}

#[test]
fn epsg_new_beijing_and_caribbean_next_25_code_batch_roundtrip() {
    for code in 4576u32..=4578 {
        let lon0 = 123.0 + 6.0 * f64::from(code - 4576);
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon0 + 0.2, 35.0).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - (lon0 + 0.2)).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - 35.0).abs() < 1e-4, "EPSG:{code} lat");
    }

    for code in 4579u32..=4589 {
        let lon0 = 75.0 + 6.0 * f64::from(code - 4579);
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon0 + 0.2, 35.0).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - (lon0 + 0.2)).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - 35.0).abs() < 1e-4, "EPSG:{code} lat");
    }

    for code in 4652u32..=4656 {
        let lon0 = 75.0 + 3.0 * f64::from(code - 4652);
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon0 + 0.15, 35.0).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - (lon0 + 0.15)).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - 35.0).abs() < 1e-4, "EPSG:{code} lat");
    }

    assert_eq!(Crs::from_epsg(4601).unwrap().datum.name, "Antigua 1943");
    assert_eq!(Crs::from_epsg(4602).unwrap().datum.name, "Dominica 1945");
    assert_eq!(Crs::from_epsg(4603).unwrap().datum.name, "Grenada 1953");
    assert_eq!(Crs::from_epsg(4604).unwrap().datum.name, "Montserrat 1958");
    assert_eq!(Crs::from_epsg(4605).unwrap().datum.name, "St. Kitts 1955");
    assert_eq!(Crs::from_epsg(4610).unwrap().datum.name, "Xian 1980");
    assert_eq!(Crs::from_epsg(4612).unwrap().datum.name, "JGD2000");
}

#[test]
fn epsg_new_beijing_3deg_next_25_code_batch_roundtrip() {
    for code in 4766u32..=4781 {
        let lon0 = 90.0 + 3.0 * f64::from(code - 4766);
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon0 + 0.15, 35.0).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - (lon0 + 0.15)).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - 35.0).abs() < 1e-4, "EPSG:{code} lat");
    }

    for code in 4782u32..=4790 {
        let lon0 = 75.0 + 3.0 * f64::from(code - 4782);
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon0 + 0.15, 35.0).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - (lon0 + 0.15)).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - 35.0).abs() < 1e-4, "EPSG:{code} lat");
    }
}

#[test]
fn epsg_new_beijing_cm_and_ntm_next_25_code_batch_roundtrip() {
    for code in 4791u32..=4800 {
        let lon0 = 102.0 + 3.0 * f64::from(code - 4791);
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon0 + 0.15, 35.0).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - (lon0 + 0.15)).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - 35.0).abs() < 1e-4, "EPSG:{code} lat");
    }

    for (code, lon0) in [(4812u32, 132.0), (4822u32, 135.0)] {
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon0 + 0.15, 35.0).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - (lon0 + 0.15)).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - 35.0).abs() < 1e-4, "EPSG:{code} lat");
    }

    for code in 4855u32..=4867 {
        let lon0 = f64::from(code - 4850) + 0.5;
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon0 + 0.2, 63.0).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - (lon0 + 0.2)).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - 63.0).abs() < 1e-4, "EPSG:{code} lat");
    }
}

#[test]
fn epsg_nad83_csrs_realizations_next_25_code_batch_roundtrip() {
    let codes = [
        4954u32, 4955,
        8230, 8231, 8232, 8233, 8235, 8237,
        8238, 8239, 8240, 8242, 8244, 8246,
        8247, 8248, 8249, 8250, 8251, 8252,
        8253, 8254, 8255,
        10413, 10414,
    ];

    for code in codes {
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(-75.0, 45.0).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon + 75.0).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - 45.0).abs() < 1e-4, "EPSG:{code} lat");
        assert!(
            crs.datum.name == "NAD83(CSRS)" || crs.datum.name == "NAD83 (CSRS)",
            "EPSG:{code} datum"
        );
    }
}

#[test]
fn epsg_etrs89_nor_ntm_next_25_code_batch_roundtrip() {
    for code in 5105u32..=5129 {
        let lon0 = f64::from(code - 5100) + 0.5;
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon0 + 0.2, 63.0).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - (lon0 + 0.2)).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - 63.0).abs() < 1e-4, "EPSG:{code} lat");
    }
}

#[test]
fn epsg_pulkovo_6deg_zone_blocks_roundtrip() {
    for code in 20004u32..=20032 {
        let zone = f64::from(code - 20000);
        let mut lon0 = zone * 6.0 - 3.0;
        while lon0 > 180.0 {
            lon0 -= 360.0;
        }
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon0, 55.0).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - lon0).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - 55.0).abs() < 1e-4, "EPSG:{code} lat");
    }

    for code in 28404u32..=28432 {
        let zone = f64::from(code - 28400);
        let mut lon0 = zone * 6.0 - 3.0;
        while lon0 > 180.0 {
            lon0 -= 360.0;
        }
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon0, 55.0).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - lon0).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - 55.0).abs() < 1e-4, "EPSG:{code} lat");
    }
}

#[test]
fn epsg_pulkovo_1995_gk_cm_block_roundtrip() {
    for code in 2463u32..=2491 {
        let idx = f64::from(code - 2463);
        let mut lon0 = 21.0 + 6.0 * idx;
        while lon0 > 180.0 {
            lon0 -= 360.0;
        }
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon0, 55.0).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - lon0).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - 55.0).abs() < 1e-4, "EPSG:{code} lat");
    }
}

#[test]
fn epsg_adjusted_pulkovo_gk_extensions_roundtrip() {
    let three_degree = [
        3329u32, 3330, 3331, 3332,
        4417, 4434,
        5670, 5671, 5672, 5673, 5674, 5675,
    ];
    for code in three_degree {
        let zone = match code {
            3329 => 5.0,
            3330 => 6.0,
            3331 => 7.0,
            3332 => 8.0,
            4417 => 7.0,
            4434 => 8.0,
            5670 | 5673 => 3.0,
            5671 | 5674 => 4.0,
            5672 | 5675 => 5.0,
            _ => unreachable!(),
        };
        let lon0 = 3.0 * zone;
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon0, 52.0).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - lon0).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - 52.0).abs() < 1e-4, "EPSG:{code} lat");
    }

    let six_degree = [3333u32, 3334, 3335, 5631, 5663, 5664, 5665];
    for code in six_degree {
        let zone = match code {
            3333 => 3.0,
            3334 => 4.0,
            3335 => 5.0,
            5631 | 5664 => 2.0,
            5663 | 5665 => 3.0,
            _ => unreachable!(),
        };
        let lon0 = zone * 6.0 - 3.0;
        let crs = Crs::from_epsg(code).unwrap();
        let (e, n) = crs.forward(lon0, 52.0).unwrap();
        let (lon, lat) = crs.inverse(e, n).unwrap();
        assert!((lon - lon0).abs() < 1e-4, "EPSG:{code} lon");
        assert!((lat - 52.0).abs() < 1e-4, "EPSG:{code} lat");
    }
}

// ─── info / listing ────────────────────────────────────────────────────────

#[test]
fn epsg_info_returns_name_for_known_code() {
    let info = epsg_info(3857).unwrap();
    assert_eq!(info.code, 3857);
    assert!(info.name.contains("Mercator") || info.name.contains("mercator"));
}

#[test]
fn epsg_info_returns_none_for_unknown() {
    assert!(epsg_info(99999).is_none());
}

#[test]
fn known_codes_include_utm_and_named() {
    let codes = known_epsg_codes();
    assert!(codes.contains(&32632));  // UTM 32N
    assert!(codes.contains(&32661));  // UPS North
    assert!(codes.contains(&32761));  // UPS South
    assert!(codes.contains(&3857));   // Web Mercator
    assert!(codes.contains(&4087));   // World Equidistant Cylindrical
    assert!(codes.contains(&5070));   // CONUS Albers
    assert!(codes.contains(&2163));   // US National Atlas Equal Area
    assert!(codes.contains(&27700));  // British National Grid
    assert!(codes.contains(&2157));   // Irish TM
    assert!(codes.contains(&29903));  // Irish Grid
    assert!(codes.contains(&3006));   // SWEREF99 TM
    assert!(codes.contains(&3032));   // Australian Antarctic Polar Stereographic
    assert!(codes.contains(&31370));  // Belgian Lambert 72
    assert!(codes.contains(&5514));   // S-JTSK / Krovak East North
    assert!(codes.contains(&6931));   // NSIDC EASE-Grid 2.0 North
    assert!(codes.contains(&6932));   // NSIDC EASE-Grid 2.0 South
    assert!(codes.contains(&6933));   // NSIDC EASE-Grid 2.0 Global
    assert!(codes.contains(&3410));   // NSIDC EASE-Grid Global
    assert!(codes.contains(&3400));   // Alberta 10-TM Forest
    assert!(codes.contains(&3401));   // Alberta 10-TM Resource
    assert!(codes.contains(&3402));   // Alberta 10-TM CSRS Forest
    assert!(codes.contains(&3403));   // Alberta 10-TM
    assert!(codes.contains(&3405));   // VN-2000 UTM zone 48N
    assert!(codes.contains(&3406));   // VN-2000 UTM zone 49N
    assert!(codes.contains(&3408));   // NSIDC EASE-Grid North
    assert!(codes.contains(&3409));   // NSIDC EASE-Grid South
    assert!(codes.contains(&3571));   // North Pole LAEA Bering Sea
    assert!(codes.contains(&3572));   // North Pole LAEA Alaska
    assert!(codes.contains(&3573));   // North Pole LAEA Canada
    assert!(codes.contains(&3574));   // North Pole LAEA Atlantic
    assert!(codes.contains(&3576));   // North Pole LAEA Russia
    assert!(codes.contains(&3578));   // Yukon Albers
    assert!(codes.contains(&3579));   // Yukon Albers CSRS
    assert!(codes.contains(&3832));   // PDC Mercator
    assert!(codes.contains(&3833));   // Pulkovo GK zone 2
    assert!(codes.contains(&3834));   // Pulkovo83 GK zone 2
    assert!(codes.contains(&3835));   // Pulkovo83 GK zone 3
    assert!(codes.contains(&3836));   // Pulkovo83 GK zone 4
    assert!(codes.contains(&3837));   // Pulkovo58 3deg GK zone 3
    assert!(codes.contains(&3838));   // Pulkovo58 3deg GK zone 4
    assert!(codes.contains(&3839));   // Pulkovo58 3deg GK zone 9
    assert!(codes.contains(&3840));   // Pulkovo58 3deg GK zone 10
    assert!(codes.contains(&3841));   // Pulkovo83 3deg GK zone 6
    for code in 2463u32..=2491 {
        assert!(codes.contains(&code), "missing EPSG:{code}");
    }
    for code in 20004u32..=20032 {
        assert!(codes.contains(&code), "missing EPSG:{code}");
    }
    for code in 28404u32..=28432 {
        assert!(codes.contains(&code), "missing EPSG:{code}");
    }
    for code in [
        3329u32, 3330, 3331, 3332, 3333, 3334, 3335,
        4417, 4434,
        5631, 5663, 5664, 5665,
        5670, 5671, 5672, 5673, 5674, 5675,
    ] {
        assert!(codes.contains(&code), "missing EPSG:{code}");
    }
    assert!(codes.contains(&3845));   // SWEREF99 RT90 7.5 gon V emulation
    assert!(codes.contains(&3846));   // SWEREF99 RT90 5 gon V emulation
    assert!(codes.contains(&3847));   // SWEREF99 RT90 2.5 gon V emulation
    assert!(codes.contains(&3848));   // SWEREF99 RT90 0 gon emulation
    assert!(codes.contains(&3849));   // SWEREF99 RT90 2.5 gon O emulation
    assert!(codes.contains(&3850));   // SWEREF99 RT90 5 gon O emulation
    assert!(codes.contains(&3976));   // NSIDC Sea Ice Polar Stereographic South
    assert!(codes.contains(&3986));   // Katanga Gauss A
    assert!(codes.contains(&3987));   // Katanga Gauss B
    assert!(codes.contains(&3988));   // Katanga Gauss C
    assert!(codes.contains(&3989));   // Katanga Gauss D
    assert!(codes.contains(&3997));   // Dubai Local TM
    assert!(codes.contains(&3994));   // Mercator 41
    assert!(codes.contains(&3991));   // Puerto Rico CS27
    assert!(codes.contains(&3992));   // Puerto Rico / St. Croix
    assert!(codes.contains(&3996));   // IBCAO Polar Stereographic
    assert!(codes.contains(&8857));   // Equal Earth Greenwich
    assert!(codes.contains(&54008));  // World Sinusoidal (ESRI)
    assert!(codes.contains(&54009));  // World Mollweide (ESRI)
    assert!(codes.contains(&54030));  // World Robinson (ESRI)
    assert!(codes.contains(&6672));   // JGD2011 Japan Plane CS IV
    assert!(codes.contains(&6673));   // JGD2011 Japan Plane CS V
    assert!(codes.contains(&6674));   // JGD2011 Japan Plane CS VI
    assert!(codes.contains(&6675));   // JGD2011 Japan Plane CS VII
    assert!(codes.contains(&6676));   // JGD2011 Japan Plane CS VIII
    assert!(codes.contains(&6677));   // JGD2011 Japan Plane CS IX
    assert!(codes.contains(&6678));   // JGD2011 Japan Plane CS X
    assert!(codes.contains(&6679));   // JGD2011 Japan Plane CS XI
    assert!(codes.contains(&6680));   // JGD2011 Japan Plane CS XII
    assert!(codes.contains(&6681));   // JGD2011 Japan Plane CS XIII
    assert!(codes.contains(&6682));   // JGD2011 Japan Plane CS XIV
    assert!(codes.contains(&6683));   // JGD2011 Japan Plane CS XV
    assert!(codes.contains(&6684));   // JGD2011 Japan Plane CS XVI
    assert!(codes.contains(&6685));   // JGD2011 Japan Plane CS XVII
    assert!(codes.contains(&6686));   // JGD2011 Japan Plane CS XVIII
    assert!(codes.contains(&6687));   // JGD2011 Japan Plane CS XIX
    assert!(codes.contains(&6688));   // JGD2011 UTM zone 51N
    assert!(codes.contains(&6689));   // JGD2011 UTM zone 52N
    assert!(codes.contains(&6690));   // JGD2011 UTM zone 53N
    assert!(codes.contains(&6691));   // JGD2011 UTM zone 54N
    assert!(codes.contains(&6692));   // JGD2011 UTM zone 55N
    assert!(codes.contains(&6707));   // RDN2008 UTM zone 32N
    assert!(codes.contains(&6708));   // RDN2008 UTM zone 33N
    assert!(codes.contains(&6709));   // RDN2008 UTM zone 34N
    assert!(codes.contains(&6732));   // GDA94 MGA zone 41
    assert!(codes.contains(&6733));   // GDA94 MGA zone 42
    assert!(codes.contains(&6734));   // GDA94 MGA zone 43
    assert!(codes.contains(&6735));   // GDA94 MGA zone 44
    assert!(codes.contains(&6736));   // GDA94 MGA zone 46
    assert!(codes.contains(&6737));   // GDA94 MGA zone 47
    assert!(codes.contains(&6738));   // GDA94 MGA zone 59
    assert!(codes.contains(&6784));   // Oregon Baker (CORS96, m)
    assert!(codes.contains(&6786));   // Oregon Baker (2011, m)
    assert!(codes.contains(&6788));   // Oregon Bend-Klamath Falls (CORS96, m)
    assert!(codes.contains(&6790));   // Oregon Bend-Klamath Falls (2011, m)
    assert!(codes.contains(&6800));   // Oregon Canyonville-Grants Pass (CORS96, m)
    assert!(codes.contains(&6802));   // Oregon Canyonville-Grants Pass (2011, m)
    assert!(codes.contains(&6812));   // Oregon Cottage Grove-Canyonville (CORS96, m)
    assert!(codes.contains(&6814));   // Oregon Cottage Grove-Canyonville (2011, m)
    assert!(codes.contains(&6816));   // Oregon Dufur-Madras (CORS96, m)
    assert!(codes.contains(&6818));   // Oregon Dufur-Madras (2011, m)
    assert!(codes.contains(&6820));   // Oregon Eugene (CORS96, m)
    assert!(codes.contains(&6822));   // Oregon Eugene (2011, m)
    assert!(codes.contains(&6824));   // Oregon Grants Pass-Ashland (CORS96, m)
    assert!(codes.contains(&6826));   // Oregon Grants Pass-Ashland (2011, m)
    assert!(codes.contains(&6828));   // Oregon Gresham-Warm Springs (CORS96, m)
    assert!(codes.contains(&6830));   // Oregon Gresham-Warm Springs (2011, m)
    assert!(codes.contains(&6832));   // Oregon La Grande (CORS96, m)
    assert!(codes.contains(&6834));   // Oregon La Grande (2011, m)
    assert!(codes.contains(&6836));   // Oregon Ontario (CORS96, m)
    assert!(codes.contains(&6838));   // Oregon Ontario (2011, m)
    assert!(codes.contains(&6844));   // Oregon Pendleton (CORS96, m)
    assert!(codes.contains(&6846));   // Oregon Pendleton (2011, m)
    assert!(codes.contains(&6848));   // Oregon Pendleton-La Grande (CORS96, m)
    assert!(codes.contains(&6850));   // Oregon Pendleton-La Grande (2011, m)
    assert!(codes.contains(&6856));   // Oregon Salem (CORS96, m)
    assert!(codes.contains(&6858));   // Oregon Salem (2011, m)
    assert!(codes.contains(&6860));   // Oregon Santiam Pass (CORS96, m)
    assert!(codes.contains(&6862));   // Oregon Santiam Pass (2011, m)
    assert!(codes.contains(&6870));   // ETRS89 / Albania TM 2010
    assert!(codes.contains(&6875));   // RDN2008 / Italy zone (N-E)
    assert!(codes.contains(&6876));   // RDN2008 / Zone 12 (N-E)
    assert!(codes.contains(&6915));   // South East Island 1943 / UTM zone 40N
    assert!(codes.contains(&6927));   // SVY21 / Singapore TM
    assert!(codes.contains(&6956));   // VN-2000 / TM-3 zone 481
    assert!(codes.contains(&6957));   // VN-2000 / TM-3 zone 482
    for code in [7257u32, 7259, 7261, 7263, 7265, 7267, 7269, 7271, 7273, 7275,
                 7277, 7279, 7281, 7283, 7285, 7287, 7289, 7291, 7293, 7295,
                 7297, 7299, 7301, 7303, 7305, 7307, 7309, 7311, 7313, 7315,
                 7317, 7319, 7321, 7323, 7325, 7327, 7329, 7331, 7333, 7335,
                 7337, 7339, 7341, 7343, 7345, 7347, 7349, 7351, 7353, 7355] {
        assert!(codes.contains(&code));
    }
    for code in [7258u32, 7260, 7262, 7264, 7266, 7268, 7270, 7272, 7274, 7276,
                 7278, 7280, 7282, 7284, 7286, 7288, 7290, 7292, 7294, 7296,
                 7298, 7300, 7302, 7304, 7306] {
        assert!(codes.contains(&code));
    }
    for code in [7057u32, 7058, 7059, 7060, 7061, 7062, 7063, 7064, 7065, 7066,
                 7067, 7068, 7069, 7070, 7109, 7110, 7111, 7112, 7113, 7114,
                 7115, 7116, 7117, 7118, 7131] {
        assert!(codes.contains(&code));
    }
    assert!(codes.contains(&4490));   // CGCS2000 geographic
    assert!(codes.contains(&4674));   // SIRGAS 2000 geographic
    assert!(codes.contains(&5396));   // SIRGAS 2000 UTM zone 26S
    assert!(codes.contains(&6210));   // SIRGAS 2000 UTM zone 23N
    assert!(codes.contains(&6211));   // SIRGAS 2000 UTM zone 24N
    for code in 31965u32..=31985 {
        assert!(codes.contains(&code), "missing EPSG:{code}");
    }
    assert!(codes.contains(&5463));   // SAD69 UTM zone 17N
    for code in 29168u32..=29172 {
        assert!(codes.contains(&code), "missing EPSG:{code}");
    }
    for code in 29187u32..=29195 {
        assert!(codes.contains(&code), "missing EPSG:{code}");
    }
    for code in 24817u32..=24821 {
        assert!(codes.contains(&code), "missing EPSG:{code}");
    }
    for code in 24877u32..=24882 {
        assert!(codes.contains(&code), "missing EPSG:{code}");
    }
    assert!(codes.contains(&7844));   // GDA2020 geographic
    for code in 4491u32..=4512 {
        assert!(codes.contains(&code));
    }
    for code in 4513u32..=4537 {
        assert!(codes.contains(&code));
    }
    for code in 4538u32..=4554 {
        assert!(codes.contains(&code));
    }
    for code in 4568u32..=4578 {
        assert!(codes.contains(&code));
    }
    for code in 4579u32..=4589 {
        assert!(codes.contains(&code));
    }
    for code in 4601u32..=4605 {
        assert!(codes.contains(&code));
    }
    assert!(codes.contains(&4610));   // Xian 1980 geographic
    assert!(codes.contains(&4612));   // JGD2000 geographic
    for code in 4652u32..=4656 {
        assert!(codes.contains(&code));
    }
    for code in 4766u32..=4790 {
        assert!(codes.contains(&code));
    }
    for code in 4791u32..=4800 {
        assert!(codes.contains(&code));
    }
    assert!(codes.contains(&4812));
    assert!(codes.contains(&4822));
    for code in 4855u32..=4867 {
        assert!(codes.contains(&code));
    }
    assert!(codes.contains(&3577));   // Australian Albers
    assert!(codes.contains(&3575));   // North Pole LAEA Europe
    assert!(codes.contains(&2227));   // California zone 3 (ftUS)
    for code in 26929u32..=26998 {
        if code == 26947 {
            continue;
        }
        assert!(codes.contains(&code), "missing EPSG:{code}");
    }
    for code in 2759u32..=2866 {
        assert!(codes.contains(&code), "missing EPSG:{code}");
    }
    for code in 3465u32..=3552 {
        assert!(codes.contains(&code), "missing EPSG:{code}");
    }
    for code in 6355u32..=6627 {
        if ((6357..=6365).contains(&code) && code != 6362)
            || ((6372..=6380).contains(&code) && code != 6372)
            || ((6388..=6392).contains(&code) && code != 6391)
        {
            assert!(!codes.contains(&code), "unexpected EPSG:{code}");
            continue;
        }
        assert!(codes.contains(&code), "missing EPSG:{code}");
    }
    assert!(codes.contains(&4326));   // WGS84
    assert!(codes.contains(&2958));   // NAD83(CSRS) / UTM zone 17N - Projected
    for code in [4954u32, 4955,
                 8230, 8231, 8232, 8233, 8235, 8237,
                 8238, 8239, 8240, 8242, 8244, 8246,
                 8247, 8248, 8249, 8250, 8251, 8252,
                 8253, 8254, 8255,
                 10413, 10414] {
        assert!(codes.contains(&code));
    }
    for code in 5105u32..=5129 {
        assert!(codes.contains(&code));
    }
    assert!(codes.contains(&2443));   // JGD2000 Japan Plane CS I
    assert!(codes.contains(&2444));   // JGD2000 Japan Plane CS II
    assert!(codes.contains(&2445));   // JGD2000 Japan Plane CS III
    assert!(codes.contains(&2446));   // JGD2000 Japan Plane CS IV
    assert!(codes.contains(&2447));   // JGD2000 Japan Plane CS V
    assert!(codes.contains(&2448));   // JGD2000 Japan Plane CS VI
    assert!(codes.contains(&2449));   // JGD2000 Japan Plane CS VII
    assert!(codes.contains(&2450));   // JGD2000 Japan Plane CS VIII
    assert!(codes.contains(&2451));   // JGD2000 Japan Plane CS IX
    assert!(codes.contains(&2452));   // JGD2000 Japan Plane CS X
    assert!(codes.contains(&2453));   // JGD2000 Japan Plane CS XI
    assert!(codes.contains(&2454));   // JGD2000 Japan Plane CS XII
    assert!(codes.contains(&2455));   // JGD2000 Japan Plane CS XIII
    assert!(codes.contains(&2456));   // JGD2000 Japan Plane CS XIV
    assert!(codes.contains(&2457));   // JGD2000 Japan Plane CS XV
    assert!(codes.contains(&2458));   // JGD2000 Japan Plane CS XVI
    assert!(codes.contains(&2459));   // JGD2000 Japan Plane CS XVII
    assert!(codes.contains(&2460));   // JGD2000 Japan Plane CS XVIII
    assert!(codes.contains(&2461));   // JGD2000 Japan Plane CS XIX
    assert!(codes.len() > 150);
}

#[test]
fn readme_supported_code_counts_are_in_sync() {
    let codes = known_epsg_codes();
    let total_count = codes.len();
    let epsg_count = codes.iter().copied().filter(|c| *c < 53_000).count();

    let readme = include_str!("../../README.md");
    assert!(
        readme.contains(&format!("**{epsg_count} EPSG codes**")),
        "README count mismatch for EPSG-supported code total"
    );
    assert!(
        readme.contains(&format!("**{total_count} total CRS/projection codes**")),
        "README count mismatch for total supported CRS/projection codes"
    );
}

// ─── top-level re-export ───────────────────────────────────────────────────

#[test]
fn top_level_from_epsg() {
    let crs = from_epsg(32636).unwrap(); // UTM 36N
    assert!(crs.name.contains("36"));
}

#[test]
fn top_level_from_wkt_resolves_epsg_authority() {
    let crs = from_wkt("GEOGCS[\"WGS 84\",AUTHORITY[\"EPSG\",\"4326\"]]").unwrap();
    assert!(crs.name.contains("WGS"));
}

#[test]
fn wkt_import_extracts_epsg_from_wkt2_and_srs_references() {
    assert_eq!(epsg_from_wkt("GEOGCRS[\"WGS 84\",ID[\"EPSG\",4326]]"), Some(4326));
    assert_eq!(epsg_from_srs_reference("EPSG:3857"), Some(3857));
    assert_eq!(
        epsg_from_srs_reference("urn:ogc:def:crs:EPSG::32633"),
        Some(32633)
    );
    assert_eq!(
        epsg_from_srs_reference("http://www.opengis.net/def/crs/EPSG/0/4326"),
        Some(4326)
    );
}

#[test]
fn identify_wkt_corpus_lenient_and_strict_modes() {
    let cases = [
        (
            "embedded_authority_wgs84",
            "GEOGCS[\"WGS 84\",AUTHORITY[\"EPSG\",\"4326\"]]",
            Some(4326),
            Some(4326),
        ),
        (
            "wkt2_id_wgs84",
            "GEOGCRS[\"WGS 84\",ID[\"EPSG\",4326]]",
            Some(4326),
            Some(4326),
        ),
        (
            "legacy_esri_nad83_csrs_utm_17n",
            "PROJCS[\"NAD83_CSRS_UTM_zone_17N\",GEOGCS[\"GCS_NAD83(CSRS)\",DATUM[\"D_North_American_1983_CSRS\",SPHEROID[\"GRS_1980\",6378137,298.257222101]],PRIMEM[\"Greenwich\",0],UNIT[\"Degree\",0.017453292519943295]],PROJECTION[\"Transverse_Mercator\"],PARAMETER[\"latitude_of_origin\",0],PARAMETER[\"central_meridian\",-81],PARAMETER[\"scale_factor\",0.9996],PARAMETER[\"false_easting\",500000],PARAMETER[\"false_northing\",0],UNIT[\"Meter\",1]]",
            Some(2958),
            None,
        ),
    ];

    for (label, wkt, expected_lenient, expected_strict) in cases {
        assert_eq!(
            identify_epsg_from_wkt_with_policy(wkt, EpsgIdentifyPolicy::Lenient),
            expected_lenient,
            "lenient mismatch for {label}"
        );
        assert_eq!(
            identify_epsg_from_wkt_with_policy(wkt, EpsgIdentifyPolicy::Strict),
            expected_strict,
            "strict mismatch for {label}"
        );
    }
}

#[test]
fn identify_wkt_report_marks_legacy_csrs_utm_case_as_ambiguous() {
    let wkt = "PROJCS[\"NAD83_CSRS_UTM_zone_17N\",GEOGCS[\"GCS_NAD83(CSRS)\",DATUM[\"D_North_American_1983_CSRS\",SPHEROID[\"GRS_1980\",6378137,298.257222101]],PRIMEM[\"Greenwich\",0],UNIT[\"Degree\",0.017453292519943295]],PROJECTION[\"Transverse_Mercator\"],PARAMETER[\"latitude_of_origin\",0],PARAMETER[\"central_meridian\",-81],PARAMETER[\"scale_factor\",0.9996],PARAMETER[\"false_easting\",500000],PARAMETER[\"false_northing\",0],UNIT[\"Meter\",1]]";

    let report = identify_epsg_from_wkt_report(wkt, EpsgIdentifyPolicy::Lenient)
        .expect("expected identify report");
    assert!(report.passed_threshold);
    assert!(report.ambiguous);
    assert_eq!(report.resolved_code, Some(2958));
    assert!(report.top_candidates.len() >= 2);
    assert_eq!(report.top_candidates[0].code, 2958);
}

#[test]
fn identify_wkt_all_manifests_in_corpus_match_expected() {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/tests/data/wkt_corpus");

    let mut manifests = fs::read_dir(&base)
        .expect("failed to read wkt corpus dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(|name| name.ends_with("manifest.csv"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    manifests.sort();
    assert!(!manifests.is_empty(), "expected at least one manifest.csv in corpus");

    for manifest in manifests {
        let manifest_name = manifest
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        assert_manifest_matches_expected(manifest_name);
    }
}

fn assert_manifest_matches_expected(manifest_file: &str) {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/tests/data/wkt_corpus");
    let manifest_path = base.join(manifest_file);
    let manifest = fs::read_to_string(&manifest_path)
        .expect("failed to read WKT corpus manifest");

    for (line_no, line) in manifest.lines().enumerate() {
        if line_no == 0 || line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split(',').collect();
        assert!(
            cols.len() >= 4,
            "manifest parse error at line {}",
            line_no + 1
        );

        let name = cols[0].trim();
        let file = cols[1].trim();
        let expected_lenient = {
            let t = cols[2].trim();
            if t.is_empty() { None } else { Some(t.parse::<u32>().unwrap()) }
        };
        let expected_strict = {
            let t = cols[3].trim();
            if t.is_empty() { None } else { Some(t.parse::<u32>().unwrap()) }
        };

        let wkt = fs::read_to_string(base.join(file))
            .unwrap_or_else(|e| panic!("failed to read corpus file {}: {e}", file));

        let got_lenient = identify_epsg_from_wkt_with_policy(&wkt, EpsgIdentifyPolicy::Lenient);
        let got_strict = identify_epsg_from_wkt_with_policy(&wkt, EpsgIdentifyPolicy::Strict);

        assert_eq!(
            got_lenient, expected_lenient,
            "lenient mismatch for corpus case {} in {}",
            name,
            manifest_file
        );
        assert_eq!(
            got_strict, expected_strict,
            "strict mismatch for corpus case {} in {}",
            name,
            manifest_file
        );
    }
}

#[test]
fn from_wkt_parses_exported_ogc_tm_without_epsg_authority() {
    let wkt = to_ogc_wkt(32632).unwrap();
    let crs = from_wkt(&wkt).unwrap();
    let (x, y) = crs.forward(9.0, 48.0).unwrap();
    let (lon, lat) = crs.inverse(x, y).unwrap();
    assert!((lon - 9.0).abs() < 1e-6);
    assert!((lat - 48.0).abs() < 1e-6);
}

#[test]
fn from_wkt_parses_exported_ogc_swiss_oblique_stereographic() {
    let wkt = to_ogc_wkt(2056).unwrap();
    let crs = from_wkt(&wkt).unwrap();
    let (x, y) = crs.forward(7.4386, 46.9511).unwrap();
    let (lon, lat) = crs.inverse(x, y).unwrap();
    assert!((lon - 7.4386).abs() < 1e-6);
    assert!((lat - 46.9511).abs() < 1e-6);
}

#[test]
fn from_wkt_parses_custom_wkt2_transverse_mercator() {
    let wkt = concat!(
        "PROJCRS[\"Custom TM\",",
        "BASEGEOGCRS[\"WGS 84\",",
        "DATUM[\"World Geodetic System 1984\",ELLIPSOID[\"WGS 84\",6378137,298.257223563]],",
        "PRIMEM[\"Greenwich\",0],ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "CONVERSION[\"Custom TM\",",
        "METHOD[\"Transverse_Mercator\"],",
        "PARAMETER[\"Latitude_of_Origin\",0,ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "PARAMETER[\"Central_Meridian\",9,ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "PARAMETER[\"Scale_Factor\",0.9996,SCALEUNIT[\"unity\",1]],",
        "PARAMETER[\"False_Easting\",500000,LENGTHUNIT[\"metre\",1]],",
        "PARAMETER[\"False_Northing\",0,LENGTHUNIT[\"metre\",1]]],",
        "CS[Cartesian,2],AXIS[\"Easting\",east],AXIS[\"Northing\",north],LENGTHUNIT[\"metre\",1]]"
    );
    let crs = from_wkt(wkt).unwrap();
    let (x, y) = crs.forward(9.0, 48.0).unwrap();
    let (lon, lat) = crs.inverse(x, y).unwrap();
    assert!((lon - 9.0).abs() < 1e-6);
    assert!((lat - 48.0).abs() < 1e-6);
}

#[test]
fn from_wkt_parses_custom_two_point_equidistant() {
    let wkt = concat!(
        "PROJCS[\"Two Point Test\",",
        "GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\",SPHEROID[\"WGS 84\",6378137,298.257223563]],PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]],",
        "PROJECTION[\"Two_Point_Equidistant\"],",
        "PARAMETER[\"Longitude_Of_1st_Point\",-10],",
        "PARAMETER[\"Latitude_Of_1st_Point\",40],",
        "PARAMETER[\"Longitude_Of_2nd_Point\",20],",
        "PARAMETER[\"Latitude_Of_2nd_Point\",50],",
        "PARAMETER[\"False_Easting\",1000],",
        "PARAMETER[\"False_Northing\",2000],",
        "UNIT[\"metre\",1]]"
    );
    let crs = from_wkt(wkt).unwrap();
    let (x, y) = crs.forward(5.0, 45.0).unwrap();
    let (lon, lat) = crs.inverse(x, y).unwrap();
    assert!((lon - 5.0).abs() < 1e-6);
    assert!((lat - 45.0).abs() < 1e-6);
}

#[test]
fn from_wkt_parses_exporter_methods_for_all_known_codes() {
    for code in known_epsg_codes() {
        let ogc = to_ogc_wkt(code).unwrap();
        let ogc_parsed = from_wkt(&ogc);
        assert!(
            ogc_parsed.is_ok(),
            "OGC parse failed for EPSG:{code}: {:?}",
            ogc_parsed.err()
        );

        let esri = to_esri_wkt(code).unwrap();
        let esri_parsed = from_wkt(&esri);
        assert!(
            esri_parsed.is_ok(),
            "ESRI parse failed for EPSG:{code}: {:?}",
            esri_parsed.err()
        );
    }
}

#[test]
fn from_wkt_parses_wkt2_vertical_crs() {
    let wkt = concat!(
        "VERTCRS[\"ODN height\",",
        "VDATUM[\"Ordnance Datum Newlyn\"],",
        "CS[vertical,1],",
        "AXIS[\"gravity-related height\",up],",
        "LENGTHUNIT[\"metre\",1]]"
    );
    let crs = from_wkt(wkt).unwrap();
    assert!(matches!(
        crs.projection.params().kind,
        crate::ProjectionKind::Vertical
    ));
}

#[test]
fn compound_from_wkt_parses_wkt2_compound() {
    let wkt = concat!(
        "COMPOUNDCRS[\"Custom Compound\",",
        "PROJCRS[\"Custom TM\",",
        "BASEGEOGCRS[\"WGS 84\",",
        "DATUM[\"World Geodetic System 1984\",ELLIPSOID[\"WGS 84\",6378137,298.257223563]],",
        "PRIMEM[\"Greenwich\",0],ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "CONVERSION[\"Custom TM\",",
        "METHOD[\"Transverse_Mercator\"],",
        "PARAMETER[\"Latitude_of_Origin\",0,ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "PARAMETER[\"Central_Meridian\",9,ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "PARAMETER[\"Scale_Factor\",0.9996,SCALEUNIT[\"unity\",1]],",
        "PARAMETER[\"False_Easting\",500000,LENGTHUNIT[\"metre\",1]],",
        "PARAMETER[\"False_Northing\",0,LENGTHUNIT[\"metre\",1]]],",
        "CS[Cartesian,2],AXIS[\"Easting\",east],AXIS[\"Northing\",north],LENGTHUNIT[\"metre\",1]],",
        "VERTCRS[\"ODN height\",VDATUM[\"Ordnance Datum Newlyn\"],",
        "CS[vertical,1],AXIS[\"gravity-related height\",up],LENGTHUNIT[\"metre\",1]]",
        "]"
    );

    let comp = compound_from_wkt(wkt).unwrap();
    let (x, y) = comp.horizontal.forward(9.0, 48.0).unwrap();
    let (lon, lat) = comp.horizontal.inverse(x, y).unwrap();
    assert!((lon - 9.0).abs() < 1e-6);
    assert!((lat - 48.0).abs() < 1e-6);
    assert!(matches!(
        comp.vertical.projection.params().kind,
        crate::ProjectionKind::Vertical
    ));
}

#[test]
fn compound_from_wkt_parses_wkt1_compd_cs() {
    let wkt = concat!(
        "COMPD_CS[\"BNG + ODN\",",
        "PROJCS[\"OSGB 1936 / British National Grid\",",
        "GEOGCS[\"OSGB 1936\",DATUM[\"OSGB_1936\",SPHEROID[\"Airy 1830\",6377563.396,299.3249646]],",
        "PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]],",
        "PROJECTION[\"Transverse_Mercator\"],",
        "PARAMETER[\"latitude_of_origin\",49],",
        "PARAMETER[\"central_meridian\",-2],",
        "PARAMETER[\"scale_factor\",0.9996012717],",
        "PARAMETER[\"false_easting\",400000],",
        "PARAMETER[\"false_northing\",-100000],",
        "UNIT[\"metre\",1]],",
        "VERT_CS[\"ODN height\",VERT_DATUM[\"Ordnance Datum Newlyn\",2005],",
        "UNIT[\"metre\",1],AXIS[\"gravity-related height\",UP]]",
        "]"
    );

    let comp = compound_from_wkt(wkt).unwrap();
    let (x, y) = comp.horizontal.forward(-2.0, 52.0).unwrap();
    let (lon, lat) = comp.horizontal.inverse(x, y).unwrap();
    assert!((lon + 2.0).abs() < 1e-6);
    assert!((lat - 52.0).abs() < 1e-6);
    assert!(matches!(
        comp.vertical.projection.params().kind,
        crate::ProjectionKind::Vertical
    ));
}

#[test]
fn compound_from_wkt_rejects_missing_vertical_component() {
    let wkt = concat!(
        "COMPOUNDCRS[\"Invalid compound\",",
        "PROJCRS[\"Only horizontal\",",
        "BASEGEOGCRS[\"WGS 84\",DATUM[\"World Geodetic System 1984\",ELLIPSOID[\"WGS 84\",6378137,298.257223563]],",
        "PRIMEM[\"Greenwich\",0],ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "CONVERSION[\"TM\",METHOD[\"Transverse_Mercator\"],",
        "PARAMETER[\"Latitude_of_Origin\",0,ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "PARAMETER[\"Central_Meridian\",9,ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "PARAMETER[\"Scale_Factor\",0.9996,SCALEUNIT[\"unity\",1]],",
        "PARAMETER[\"False_Easting\",500000,LENGTHUNIT[\"metre\",1]],",
        "PARAMETER[\"False_Northing\",0,LENGTHUNIT[\"metre\",1]]],",
        "CS[Cartesian,2],AXIS[\"Easting\",east],AXIS[\"Northing\",north],LENGTHUNIT[\"metre\",1]]",
        "]"
    );

    let err = compound_from_wkt(wkt).unwrap_err();
    assert!(matches!(err, crate::ProjectionError::UnsupportedProjection(_)));
}

#[test]
fn compound_from_wkt_rejects_non_vertical_second_component() {
    let wkt = concat!(
        "COMPOUNDCRS[\"Invalid compound\",",
        "PROJCRS[\"Horizontal A\",",
        "BASEGEOGCRS[\"WGS 84\",DATUM[\"World Geodetic System 1984\",ELLIPSOID[\"WGS 84\",6378137,298.257223563]],",
        "PRIMEM[\"Greenwich\",0],ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "CONVERSION[\"TM\",METHOD[\"Transverse_Mercator\"],",
        "PARAMETER[\"Latitude_of_Origin\",0,ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "PARAMETER[\"Central_Meridian\",9,ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "PARAMETER[\"Scale_Factor\",0.9996,SCALEUNIT[\"unity\",1]],",
        "PARAMETER[\"False_Easting\",500000,LENGTHUNIT[\"metre\",1]],",
        "PARAMETER[\"False_Northing\",0,LENGTHUNIT[\"metre\",1]]],",
        "CS[Cartesian,2],AXIS[\"Easting\",east],AXIS[\"Northing\",north],LENGTHUNIT[\"metre\",1]],",
        "GEOGCRS[\"WGS 84\",DATUM[\"World Geodetic System 1984\",ELLIPSOID[\"WGS 84\",6378137,298.257223563]],",
        "PRIMEM[\"Greenwich\",0],ANGLEUNIT[\"degree\",0.0174532925199433]]",
        "]"
    );

    let err = compound_from_wkt(wkt).unwrap_err();
    assert!(matches!(err, crate::ProjectionError::UnsupportedProjection(_)));
}

#[test]
fn compound_from_wkt_flattens_nested_compound_tree() {
    let wkt = concat!(
        "COMPOUNDCRS[\"Nested compound\",",
        "COMPOUNDCRS[\"Inner\",",
        "PROJCRS[\"Custom TM\",",
        "BASEGEOGCRS[\"WGS 84\",DATUM[\"World Geodetic System 1984\",ELLIPSOID[\"WGS 84\",6378137,298.257223563]],",
        "PRIMEM[\"Greenwich\",0],ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "CONVERSION[\"TM\",METHOD[\"Transverse_Mercator\"],",
        "PARAMETER[\"Latitude_of_Origin\",0,ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "PARAMETER[\"Central_Meridian\",9,ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "PARAMETER[\"Scale_Factor\",0.9996,SCALEUNIT[\"unity\",1]],",
        "PARAMETER[\"False_Easting\",500000,LENGTHUNIT[\"metre\",1]],",
        "PARAMETER[\"False_Northing\",0,LENGTHUNIT[\"metre\",1]]],",
        "CS[Cartesian,2],AXIS[\"Easting\",east],AXIS[\"Northing\",north],LENGTHUNIT[\"metre\",1]],",
        "VERTCRS[\"Inner vertical\",VDATUM[\"Dummy\"],CS[vertical,1],AXIS[\"gravity-related height\",up],LENGTHUNIT[\"metre\",1]]",
        "],",
        "ID[\"LOCAL\",1]",
        "]"
    );

    let compound = compound_from_wkt(wkt).unwrap();
    let (x, y) = compound.horizontal.forward(9.0, 48.0).unwrap();
    let (lon, lat) = compound.horizontal.inverse(x, y).unwrap();
    assert!((lon - 9.0).abs() < 1e-6);
    assert!((lat - 48.0).abs() < 1e-6);
    assert!(matches!(
        compound.vertical.projection.params().kind,
        crate::ProjectionKind::Vertical
    ));
}

#[test]
fn compound_from_wkt_rejects_nested_ambiguous_components() {
    let wkt = concat!(
        "COMPOUNDCRS[\"Ambiguous nested\",",
        "COMPOUNDCRS[\"Inner A\",",
        "PROJCRS[\"TM A\",",
        "BASEGEOGCRS[\"WGS 84\",DATUM[\"World Geodetic System 1984\",ELLIPSOID[\"WGS 84\",6378137,298.257223563]],",
        "PRIMEM[\"Greenwich\",0],ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "CONVERSION[\"TM\",METHOD[\"Transverse_Mercator\"],",
        "PARAMETER[\"Latitude_of_Origin\",0,ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "PARAMETER[\"Central_Meridian\",9,ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "PARAMETER[\"Scale_Factor\",0.9996,SCALEUNIT[\"unity\",1]],",
        "PARAMETER[\"False_Easting\",500000,LENGTHUNIT[\"metre\",1]],",
        "PARAMETER[\"False_Northing\",0,LENGTHUNIT[\"metre\",1]]],",
        "CS[Cartesian,2],AXIS[\"Easting\",east],AXIS[\"Northing\",north],LENGTHUNIT[\"metre\",1]],",
        "VERTCRS[\"Vertical A\",VDATUM[\"Dummy\"],CS[vertical,1],AXIS[\"gravity-related height\",up],LENGTHUNIT[\"metre\",1]]",
        "],",
        "COMPOUNDCRS[\"Inner B\",",
        "PROJCRS[\"TM B\",",
        "BASEGEOGCRS[\"WGS 84\",DATUM[\"World Geodetic System 1984\",ELLIPSOID[\"WGS 84\",6378137,298.257223563]],",
        "PRIMEM[\"Greenwich\",0],ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "CONVERSION[\"TM\",METHOD[\"Transverse_Mercator\"],",
        "PARAMETER[\"Latitude_of_Origin\",0,ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "PARAMETER[\"Central_Meridian\",9,ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "PARAMETER[\"Scale_Factor\",0.9996,SCALEUNIT[\"unity\",1]],",
        "PARAMETER[\"False_Easting\",500000,LENGTHUNIT[\"metre\",1]],",
        "PARAMETER[\"False_Northing\",0,LENGTHUNIT[\"metre\",1]]],",
        "CS[Cartesian,2],AXIS[\"Easting\",east],AXIS[\"Northing\",north],LENGTHUNIT[\"metre\",1]],",
        "VERTCRS[\"Vertical B\",VDATUM[\"Dummy\"],CS[vertical,1],AXIS[\"gravity-related height\",up],LENGTHUNIT[\"metre\",1]]",
        "]",
        "]"
    );

    let err = compound_from_wkt(wkt).unwrap_err();
    assert!(matches!(err, crate::ProjectionError::UnsupportedProjection(_)));
}

#[test]
fn compound_from_wkt_structure_matrix() {
    const H_TM: &str = concat!(
        "PROJCRS[\"TM\",",
        "BASEGEOGCRS[\"WGS 84\",DATUM[\"World Geodetic System 1984\",ELLIPSOID[\"WGS 84\",6378137,298.257223563]],",
        "PRIMEM[\"Greenwich\",0],ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "CONVERSION[\"TM\",METHOD[\"Transverse_Mercator\"],",
        "PARAMETER[\"Latitude_of_Origin\",0,ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "PARAMETER[\"Central_Meridian\",9,ANGLEUNIT[\"degree\",0.0174532925199433]],",
        "PARAMETER[\"Scale_Factor\",0.9996,SCALEUNIT[\"unity\",1]],",
        "PARAMETER[\"False_Easting\",500000,LENGTHUNIT[\"metre\",1]],",
        "PARAMETER[\"False_Northing\",0,LENGTHUNIT[\"metre\",1]]],",
        "CS[Cartesian,2],AXIS[\"Easting\",east],AXIS[\"Northing\",north],LENGTHUNIT[\"metre\",1]]"
    );
    const H_GEOG: &str = concat!(
        "GEOGCRS[\"WGS 84\",",
        "DATUM[\"World Geodetic System 1984\",ELLIPSOID[\"WGS 84\",6378137,298.257223563]],",
        "PRIMEM[\"Greenwich\",0],ANGLEUNIT[\"degree\",0.0174532925199433]]"
    );
    const V1: &str = concat!(
        "VERTCRS[\"V1\",VDATUM[\"Dummy\"],",
        "CS[vertical,1],AXIS[\"gravity-related height\",up],LENGTHUNIT[\"metre\",1]]"
    );
    const V2: &str = concat!(
        "VERTCRS[\"V2\",VDATUM[\"Dummy\"],",
        "CS[vertical,1],AXIS[\"gravity-related height\",up],LENGTHUNIT[\"metre\",1]]"
    );

    let cases: [(&str, bool); 8] = [
        (&format!("COMPOUNDCRS[\"A\",{H_TM},{V1}]"), true),
        (&format!("COMPOUNDCRS[\"B\",COMPOUNDCRS[\"Inner\",{H_TM},{V1}]]"), true),
        (&format!("COMPOUNDCRS[\"C\",COMPOUNDCRS[\"I1\",COMPOUNDCRS[\"I2\",{H_TM},{V1}]]]"), true),
        (&format!("COMPOUNDCRS[\"D\",{H_TM}]"), false),
        (&format!("COMPOUNDCRS[\"E\",{V1}]"), false),
        (&format!("COMPOUNDCRS[\"F\",{H_TM},{H_GEOG},{V1}]"), false),
        (&format!("COMPOUNDCRS[\"G\",{H_TM},{V1},{V2}]"), false),
        (
            &format!(
                "COMPOUNDCRS[\"H\",COMPOUNDCRS[\"I1\",{H_TM},{V1}],COMPOUNDCRS[\"I2\",{H_GEOG},{V2}]]"
            ),
            false,
        ),
    ];

    for (wkt, should_parse) in cases {
        let result = compound_from_wkt(wkt);
        if should_parse {
            let compound = result.unwrap();
            let (x, y) = compound.horizontal.forward(9.0, 48.0).unwrap();
            let (lon, lat) = compound.horizontal.inverse(x, y).unwrap();
            assert!((lon - 9.0).abs() < 1e-6);
            assert!((lat - 48.0).abs() < 1e-6);
            assert!(matches!(
                compound.vertical.projection.params().kind,
                crate::ProjectionKind::Vertical
            ));
        } else {
            assert!(
                matches!(result, Err(crate::ProjectionError::UnsupportedProjection(_))),
                "expected unsupported compound structure for {wkt}"
            );
        }
    }
}

#[test]
fn from_wkt_applies_projected_foot_units_to_linear_parameters() {
    let m_to_ft = 1.0 / 0.304800609601219;
    let fe_m = 500_000.0;
    let fn_m = 0.0;

    let wkt_m = format!(
        "PROJCRS[\"TM metres\",BASEGEOGCRS[\"WGS 84\",DATUM[\"World Geodetic System 1984\",ELLIPSOID[\"WGS 84\",6378137,298.257223563]],PRIMEM[\"Greenwich\",0],ANGLEUNIT[\"degree\",0.0174532925199433]],CONVERSION[\"TM\",METHOD[\"Transverse_Mercator\"],PARAMETER[\"Latitude_of_Origin\",0,ANGLEUNIT[\"degree\",0.0174532925199433]],PARAMETER[\"Central_Meridian\",9,ANGLEUNIT[\"degree\",0.0174532925199433]],PARAMETER[\"Scale_Factor\",0.9996,SCALEUNIT[\"unity\",1]],PARAMETER[\"False_Easting\",{fe_m},LENGTHUNIT[\"metre\",1]],PARAMETER[\"False_Northing\",{fn_m},LENGTHUNIT[\"metre\",1]]],CS[Cartesian,2],AXIS[\"Easting\",east],AXIS[\"Northing\",north],LENGTHUNIT[\"metre\",1]]"
    );

    let wkt_ft = format!(
        "PROJCRS[\"TM feet\",BASEGEOGCRS[\"WGS 84\",DATUM[\"World Geodetic System 1984\",ELLIPSOID[\"WGS 84\",6378137,298.257223563]],PRIMEM[\"Greenwich\",0],ANGLEUNIT[\"degree\",0.0174532925199433]],CONVERSION[\"TM\",METHOD[\"Transverse_Mercator\"],PARAMETER[\"Latitude_of_Origin\",0,ANGLEUNIT[\"degree\",0.0174532925199433]],PARAMETER[\"Central_Meridian\",9,ANGLEUNIT[\"degree\",0.0174532925199433]],PARAMETER[\"Scale_Factor\",0.9996,SCALEUNIT[\"unity\",1]],PARAMETER[\"False_Easting\",{},LENGTHUNIT[\"US survey foot\",0.304800609601219]],PARAMETER[\"False_Northing\",{},LENGTHUNIT[\"US survey foot\",0.304800609601219]]],CS[Cartesian,2],AXIS[\"Easting\",east],AXIS[\"Northing\",north],LENGTHUNIT[\"US survey foot\",0.304800609601219]]",
        fe_m * m_to_ft,
        fn_m * m_to_ft,
    );

    let crs_m = from_wkt(&wkt_m).unwrap();
    let crs_ft = from_wkt(&wkt_ft).unwrap();

    let (x_m, y_m) = crs_m.forward(9.0, 48.0).unwrap();
    let (x_ft, y_ft) = crs_ft.forward(9.0, 48.0).unwrap();

    assert!((x_m - x_ft).abs() < 5e-6, "x diff = {}", (x_m - x_ft).abs());
    assert!((y_m - y_ft).abs() < 5e-6, "y diff = {}", (y_m - y_ft).abs());
}

// ─── WKT generation ───────────────────────────────────────────────────────

#[test]
fn esri_wkt_contains_projection_and_units() {
    let wkt = to_esri_wkt(32632).unwrap();
    // println!("ESRI WKT:\n{wkt}");
    assert!(wkt.contains("PROJCS"));
    assert!(wkt.contains("Transverse_Mercator") || wkt.contains("Transverse Mercator"));
    assert!(wkt.contains("UNIT[\"Meter\""));
}

#[test]
fn ogc_wkt_contains_projection_and_units() {
    let wkt = to_ogc_wkt(32632).unwrap();
    assert!(wkt.contains("PROJCS"));
    assert!(wkt.contains("Transverse_Mercator"));
    assert!(wkt.contains("UNIT[\"metre\""));
}

#[test]
fn wkt_geographic_returns_geogcs_only() {
    let esri = to_esri_wkt(4326).unwrap();
    let ogc = to_ogc_wkt(4326).unwrap();
    assert!(esri.contains("GEOGCS"));
    assert!(ogc.contains("GEOGCS"));
    assert!(!esri.contains("PROJCS"));
    assert!(!ogc.contains("PROJCS"));
    // The output must be a single-level GEOGCS — no double-nesting such as
    // GEOGCS["name", GEOGCS[...]], which is invalid WKT1 and crashes
    // PROJ-based parsers (e.g. CloudCompare, GDAL).
    assert_eq!(
        ogc.matches("GEOGCS").count(),
        1,
        "to_ogc_wkt(4326) must not contain nested GEOGCS: {ogc}"
    );
    assert_eq!(
        esri.matches("GEOGCS").count(),
        1,
        "to_esri_wkt(4326) must not contain nested GEOGCS: {esri}"
    );
}

#[test]
fn wkt_vertical_depth_uses_down_axis() {
    let esri = to_esri_wkt(5715).unwrap();
    let ogc = to_ogc_wkt(5715).unwrap();
    assert!(esri.contains("AXIS[\"gravity-related depth\",DOWN]"));
    assert!(ogc.contains("AXIS[\"gravity-related depth\",DOWN]"));
}

#[test]
fn wkt_vertical_us_foot_uses_foot_unit() {
    let esri = to_esri_wkt(5702).unwrap();
    let ogc = to_ogc_wkt(5702).unwrap();
    assert!(esri.contains("UNIT[\"Foot_US\","));
    assert!(ogc.contains("UNIT[\"US survey foot\","));
}

#[test]
fn wkt_swiss_codes_use_oblique_stereographic_method_name() {
    let esri = to_esri_wkt(2056).unwrap();
    let ogc = to_ogc_wkt(21781).unwrap();
    assert!(esri.contains("Oblique_Stereographic"));
    assert!(ogc.contains("Oblique_Stereographic"));
}

#[test]
fn two_point_equidistant_wkt_method_name_is_stable() {
    let crs = Crs::new(
        "Two-Point Equidistant Test",
        crate::Datum::WGS84,
        crate::ProjectionParams::new(crate::ProjectionKind::TwoPointEquidistant {
            lon1: -10.0,
            lat1: 40.0,
            lon2: 20.0,
            lat2: 50.0,
        })
        .with_false_easting(1000.0)
        .with_false_northing(2000.0),
    )
    .unwrap();
    let (proj_name, params) = crate::epsg::ogc_projection_params(crs.projection.params());
    assert_eq!(proj_name, "Two_Point_Equidistant");
    assert!(params.iter().any(|(name, value)| *name == "longitude_of_1st_point" && (*value - -10.0).abs() < TOL));
    assert!(params.iter().any(|(name, value)| *name == "latitude_of_2nd_point" && (*value - 50.0).abs() < TOL));
}

#[test]
fn geotiff_info_projected_and_geographic() {
    let projected = to_geotiff_info(32632).unwrap();
    println!("Projected CRS GeotiffInfo: {projected:#?}");
    assert_eq!(projected.model_type, 1);
    assert_eq!(projected.raster_type, 1);
    assert_eq!(projected.projected_cs_type, Some(32632));
    assert_eq!(projected.vertical_cs_type, None);
    assert_eq!(projected.geographic_type, None);
    assert_eq!(projected.linear_units, Some(9001));

    let geographic = to_geotiff_info(4326).unwrap();
    assert_eq!(geographic.model_type, 2);
    assert_eq!(geographic.raster_type, 1);
    assert_eq!(geographic.geographic_type, Some(4326));
    assert_eq!(geographic.projected_cs_type, None);
    assert_eq!(geographic.vertical_cs_type, None);
    assert_eq!(geographic.angular_units, Some(9102));
}

#[test]
fn geotiff_info_vertical_units_respect_registry_unit() {
    let navd88_m = to_geotiff_info(5703).unwrap();
    assert_eq!(navd88_m.model_type, 3);
    assert_eq!(navd88_m.vertical_cs_type, Some(5703));
    assert_eq!(navd88_m.linear_units, Some(9001));

    let navd88_ft = to_geotiff_info(8228).unwrap();
    assert_eq!(navd88_ft.model_type, 3);
    assert_eq!(navd88_ft.vertical_cs_type, Some(8228));
    assert_eq!(navd88_ft.linear_units, Some(9003));
}

#[test]
fn vertical_offset_grid_name_maps_expected_codes() {
    assert_eq!(vertical_offset_grid_name(3855), Some("egm2008"));
    assert_eq!(vertical_offset_grid_name(5773), Some("egm96"));
    assert_eq!(vertical_offset_grid_name(5701), Some("osgm15"));
    assert_eq!(vertical_offset_grid_name(5703), Some("geoid18"));
    assert_eq!(vertical_offset_grid_name(7841), Some("ausgeoid2020"));
    assert_eq!(vertical_offset_grid_name(4326), None);
}

#[test]
fn newly_added_epsg_workflows_batches_resolve() {
    for code in [
        2040_u32, 2041_u32, 2042_u32, 2043_u32,
        2057_u32, 2059_u32, 2060_u32, 2061_u32,
        2063_u32, 2064_u32, 2067_u32,
        2068_u32, 2069_u32, 2070_u32, 2071_u32, 2072_u32, 2073_u32, 2074_u32,
        2075_u32, 2076_u32, 2077_u32, 2078_u32, 2079_u32, 2080_u32,
        2085_u32, 2086_u32,
        2087_u32, 2088_u32, 2089_u32, 2090_u32, 2091_u32, 2092_u32,
        2093_u32, 2094_u32, 2095_u32, 2096_u32, 2097_u32, 2098_u32,
        2105_u32, 2106_u32, 2107_u32, 2108_u32, 2109_u32, 2110_u32,
        2111_u32, 2112_u32, 2113_u32, 2114_u32, 2115_u32, 2116_u32,
        2117_u32, 2118_u32, 2119_u32, 2120_u32, 2121_u32, 2122_u32,
        2123_u32, 2124_u32, 2125_u32, 2126_u32, 2127_u32, 2128_u32,
        2129_u32, 2130_u32, 2131_u32, 2132_u32, 2133_u32, 2134_u32,
        2135_u32,
        2136_u32, 2137_u32, 2138_u32,
        2148_u32, 2149_u32, 2150_u32, 2151_u32, 2152_u32,
        2153_u32, 2158_u32, 2159_u32, 2160_u32, 2161_u32, 2162_u32,
        2164_u32, 2165_u32, 2166_u32, 2167_u32, 2168_u32, 2169_u32, 2170_u32,
        2172_u32, 2173_u32, 2174_u32, 2175_u32,
        2188_u32, 2189_u32, 2190_u32, 2191_u32, 2192_u32,
        2195_u32, 2196_u32, 2197_u32, 2198_u32,
        2205_u32, 2206_u32, 2207_u32, 2208_u32, 2209_u32,
        2210_u32, 2211_u32, 2212_u32, 2213_u32,
        2200_u32, 2201_u32, 2202_u32, 2203_u32, 2204_u32,
        2214_u32, 2215_u32, 2216_u32, 2217_u32, 2219_u32, 2220_u32,
        2222_u32, 2223_u32, 2224_u32, 2225_u32, 2226_u32, 2228_u32,
        2252_u32, 2253_u32, 2254_u32, 2255_u32, 2256_u32, 2257_u32,
        2258_u32, 2259_u32, 2260_u32, 2261_u32, 2262_u32, 2264_u32,
        2265_u32, 2266_u32, 2267_u32, 2268_u32, 2269_u32, 2270_u32,
        2271_u32, 2274_u32, 2275_u32, 2276_u32, 2277_u32, 2278_u32,
        2279_u32, 2280_u32, 2281_u32, 2282_u32,
        2287_u32, 2288_u32, 2289_u32, 2290_u32, 2291_u32, 2292_u32,
        2294_u32, 2295_u32, 2308_u32, 2309_u32, 2310_u32, 2311_u32,
        2312_u32, 2313_u32, 2314_u32,
        2315_u32, 2316_u32, 2317_u32, 2318_u32, 2319_u32, 2320_u32,
        2321_u32, 2322_u32, 2323_u32, 2324_u32, 2325_u32,
        2327_u32, 2328_u32, 2329_u32, 2330_u32, 2331_u32, 2332_u32, 2333_u32,
        2397_u32, 2398_u32, 2399_u32,
    ] {
        assert!(from_epsg(code).is_ok(), "EPSG:{code} should resolve");
    }

    for code in 32201_u32..=32260_u32 {
        assert!(from_epsg(code).is_ok(), "EPSG:{code} should resolve");
    }
    for code in 32301_u32..=32360_u32 {
        assert!(from_epsg(code).is_ok(), "EPSG:{code} should resolve");
    }

    for code in 2494_u32..=2758_u32 {
        assert!(from_epsg(code).is_ok(), "EPSG:{code} should resolve");
    }

    for code in 2334_u32..=2390_u32 {
        assert!(from_epsg(code).is_ok(), "EPSG:{code} should resolve");
    }

    for code in 3580_u32..=3751_u32 {
        assert!(from_epsg(code).is_ok(), "EPSG:{code} should resolve");
    }
}

#[test]
fn known_epsg_codes_have_wkt() {
    // println!("Testing WKT generation for {} known EPSG codes...", known_epsg_codes().len());
    for code in known_epsg_codes() {
        assert!(to_esri_wkt(code).is_ok(), "ESRI WKT failed for EPSG:{code}");
        assert!(to_ogc_wkt(code).is_ok(), "OGC WKT failed for EPSG:{code}");
    }
}

