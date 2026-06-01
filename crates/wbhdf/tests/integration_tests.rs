use wbhdf::btree::{
    parse_node_header, read_chunk_payload_in_file, read_chunked_storage_leaf_chain_records_in_file,
    read_first_chunked_storage_leaf_record_in_file,
};
use wbhdf::attributes::{
    dataset_metadata_contains_text_in_file, dataset_metadata_text_report_in_file,
};
use wbhdf::btree::read_chunked_storage_records_bounded_in_file;
use wbhdf::compare::{compare_f32_with_tolerance, compare_f64_with_tolerance};
use wbhdf::dataset::{
    apply_fill_value_mapping_f32, decode_chunked_i16_row_major_window_in_file,
    decode_chunked_f32_row_major_window_in_file,
    decode_chunked_i16_row_prefix_in_file, DatasetChunkLocator,
    decode_chunked_u16_row_major_window_in_file,
    decode_chunked_u8_row_major_window_in_file,
};
use wbhdf::dataset::read_contiguous_f64_window_in_file;
use wbhdf::dataset::resolve_dataset_in_file;
use wbhdf::dataset::resolve_dataset;
use wbhdf::dataset::read_contiguous_f32_window_in_file;
use wbhdf::datatypes::{decode_f32, decode_f32_slice, decode_fixed_string, Endianness};
use wbhdf::fixtures::{
    external_fixture_dir, external_modis_fixture_dir, external_viirs_fixture_dir,
    fixture_is_available, smoke_fixture_file,
};
use wbhdf::filters::decompress_zlib;
use wbhdf::hdf4::{
    assess_hdf4_sds_i16_decode_readiness,
    assess_hdf4_sds_i16_decode_readiness_in_file,
    attempt_decode_hdf4_sds_i16_window_in_file,
    decode_hdf4_sds_i16_window_at_in_file,
    decode_hdf4_sds_i16_in_file, enumerate_hdf4_dataset_paths, probe_hdf4_eos_metadata_in_file,
    resolve_hdf4_dataset_path, resolve_hdf4_grid_field, derive_hdf4_grid_geometry,
    parse_hdf4_data_descriptors_in_file, find_hdf4_sds_i16_payload_candidates_in_file,
    map_hdf4_sds_i16_descriptor_heuristic_in_file,
    probe_hdf4_sds_i16_payload_window_in_file,
    rank_hdf4_sds_i16_payload_candidates_in_file,
};
use wbhdf::object_header::{
    parse_continuation_chunk_in_file, parse_v1_object_header_in_file, probe_file_object_headers,
    read_contiguous_layout_bytes_in_file,
};
use wbhdf::superblock::probe_file_metadata;

fn fixture_named(name: &str) -> Option<std::path::PathBuf> {
    let root = external_fixture_dir()?;
    let path = root.join(name);
    path.is_file().then_some(path)
}

fn viirs_fixture_named(name: &str) -> Option<std::path::PathBuf> {
    let root = external_viirs_fixture_dir()?;
    let path = root.join(name);
    path.is_file().then_some(path)
}

fn hdf4_example_fixture_named(name: &str) -> Option<std::path::PathBuf> {
    let root = std::path::Path::new("/Users/johnlindsay/Documents/data/hdf5_examples");
    let path = root.join(name);
    path.is_file().then_some(path)
}
fn hdf4_example_fixture_in_data_dir(name: &str) -> Option<std::path::PathBuf> {
    let root = std::path::Path::new("/Users/johnlindsay/Documents/data/hdf5_examples");
    let path = root.join(name);
    path.is_file().then_some(path)
}

fn modis_fixture_named(name: &str) -> Option<std::path::PathBuf> {
    let root = external_modis_fixture_dir()?;
    let path = root.join(name);
    path.is_file().then_some(path)
}

fn find_modis_field<'a>(
    summary: &'a wbhdf::hdf4::Hdf4EosMetadataSummary,
    grid_name: &str,
    field_name: &str,
) -> Option<&'a wbhdf::hdf4::Hdf4DataFieldSummary> {
    summary
        .grids
        .iter()
        .find(|grid| grid.name == grid_name)
        .and_then(|grid| grid.data_fields.iter().find(|field| field.name == field_name))
}

fn find_modis_grid<'a>(
    summary: &'a wbhdf::hdf4::Hdf4EosMetadataSummary,
    grid_name: &str,
) -> Option<&'a wbhdf::hdf4::Hdf4GridSummary> {
    summary.grids.iter().find(|grid| grid.name == grid_name)
}

fn assert_u16_semantics_contract(
    path: &std::path::Path,
    dataset_path: &str,
    index_address: u64,
    valid_min: u16,
    expected_nonfill_raw: u16,
    expected_nonfill_scaled: f64,
    scale_factor: f64,
    add_offset: f64,
    expected_fill_scaled: f64,
) {
    let descriptor = resolve_dataset_in_file(path, dataset_path)
        .expect("VNP21 dataset should be discoverable before decode checks");
    assert_eq!(descriptor.path, dataset_path);

    let nonfill = decode_chunked_u16_row_major_window_in_file(
        path,
        dataset_path,
        index_address,
        3,
        0,
        800,
        2,
        1600,
        4,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 non-fill window should decode through v1 chunk traversal");
    assert!(!nonfill.is_empty());
    assert!(
        nonfill.iter().all(|v| *v >= valid_min),
        "{dataset_path} non-fill window should respect valid_range lower bound"
    );
    assert!(
        nonfill.contains(&expected_nonfill_raw),
        "{dataset_path} should include expected reference raw value {expected_nonfill_raw}"
    );
    let nonfill_scaled: Vec<f64> = nonfill
        .iter()
        .map(|v| *v as f64 * scale_factor + add_offset)
        .collect();
    assert!(
        nonfill_scaled
            .iter()
            .any(|v| (*v - expected_nonfill_scaled).abs() <= 1e-12),
        "{dataset_path} should include expected scaled value {expected_nonfill_scaled}"
    );

    let fill = decode_chunked_u16_row_major_window_in_file(
        path,
        dataset_path,
        index_address,
        3,
        0,
        1500,
        2,
        2500,
        4,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 fill window should decode through v1 chunk traversal");
    assert!(
        fill.iter().all(|v| *v == 0),
        "{dataset_path} fill window should be all _FillValue=0"
    );
    let fill_scaled: Vec<f64> = fill
        .iter()
        .map(|v| *v as f64 * scale_factor + add_offset)
        .collect();
    assert!(
        fill_scaled
            .iter()
            .all(|v| (*v - expected_fill_scaled).abs() <= 1e-12),
        "{dataset_path} fill window should map to expected scaled fill value"
    );
}

fn assert_u8_semantics_contract(
    path: &std::path::Path,
    dataset_path: &str,
    index_address: u64,
    valid_min: u8,
    expected_nonfill_raw: u8,
    expected_nonfill_scaled: f64,
    scale_factor: f64,
    add_offset: f64,
    expected_fill_scaled: f64,
) {
    let descriptor = resolve_dataset_in_file(path, dataset_path)
        .expect("VNP21 dataset should be discoverable before decode checks");
    assert_eq!(descriptor.path, dataset_path);

    let nonfill = decode_chunked_u8_row_major_window_in_file(
        path,
        dataset_path,
        index_address,
        3,
        0,
        800,
        2,
        1600,
        4,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 non-fill window should decode through v1 chunk traversal");
    assert!(!nonfill.is_empty());
    assert!(
        nonfill.iter().all(|v| *v >= valid_min),
        "{dataset_path} non-fill window should respect valid_range lower bound"
    );
    assert!(
        nonfill.contains(&expected_nonfill_raw),
        "{dataset_path} should include expected reference raw value {expected_nonfill_raw}"
    );
    let nonfill_scaled: Vec<f64> = nonfill
        .iter()
        .map(|v| *v as f64 * scale_factor + add_offset)
        .collect();
    assert!(
        nonfill_scaled
            .iter()
            .any(|v| (*v - expected_nonfill_scaled).abs() <= 1e-12),
        "{dataset_path} should include expected scaled value {expected_nonfill_scaled}"
    );

    let fill = decode_chunked_u8_row_major_window_in_file(
        path,
        dataset_path,
        index_address,
        3,
        0,
        1500,
        2,
        2500,
        4,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 fill window should decode through v1 chunk traversal");
    assert!(
        fill.iter().all(|v| *v == 0),
        "{dataset_path} fill window should be all _FillValue=0"
    );
    let fill_scaled: Vec<f64> = fill
        .iter()
        .map(|v| *v as f64 * scale_factor + add_offset)
        .collect();
    assert!(
        fill_scaled
            .iter()
            .all(|v| (*v - expected_fill_scaled).abs() <= 1e-12),
        "{dataset_path} fill window should map to expected scaled fill value"
    );
}

struct U16SemanticsCase {
    dataset_path: &'static str,
    index_address: u64,
    valid_min: u16,
    expected_nonfill_raw: u16,
    expected_nonfill_scaled: f64,
    scale_factor: f64,
    add_offset: f64,
    expected_fill_scaled: f64,
}

struct U8SemanticsCase {
    dataset_path: &'static str,
    index_address: u64,
    valid_min: u8,
    expected_nonfill_raw: u8,
    expected_nonfill_scaled: f64,
    scale_factor: f64,
    add_offset: f64,
    expected_fill_scaled: f64,
}

fn run_u16_semantics_cases(path: &std::path::Path, cases: &[U16SemanticsCase]) {
    for case in cases {
        assert_u16_semantics_contract(
            path,
            case.dataset_path,
            case.index_address,
            case.valid_min,
            case.expected_nonfill_raw,
            case.expected_nonfill_scaled,
            case.scale_factor,
            case.add_offset,
            case.expected_fill_scaled,
        );
    }
}

fn run_u8_semantics_cases(path: &std::path::Path, cases: &[U8SemanticsCase]) {
    for case in cases {
        assert_u8_semantics_contract(
            path,
            case.dataset_path,
            case.index_address,
            case.valid_min,
            case.expected_nonfill_raw,
            case.expected_nonfill_scaled,
            case.scale_factor,
            case.add_offset,
            case.expected_fill_scaled,
        );
    }
}

#[test]
fn canonical_dataset_path_is_accepted() {
    let ds = resolve_dataset("/group/dataset").expect("dataset path should be accepted");
    assert_eq!(ds.path, "/group/dataset");
}

#[test]
fn relative_dataset_path_is_rejected() {
    let err = resolve_dataset("group/dataset").expect_err("relative path should be rejected");
    let msg = format!("{err}");
    assert!(msg.contains("must start"));
}

#[test]
fn metadata_smoke_test_skips_gracefully_without_fixture() {
    let Some(path) = smoke_fixture_file() else {
        return;
    };
    if !fixture_is_available(&path) {
        return;
    }

    let metadata = probe_file_metadata(&path).expect("metadata probe should succeed");
    assert!(metadata.superblock_version <= 3);

    // Group discovery is heuristic at this stage, but the smoke path must not fail.
    let _groups = metadata.top_level_groups;
}

#[test]
fn atl08_fixture_dir_smoke_discovers_beam_groups() {
    let Some(path) = fixture_named("ATL08_20181120185605_08120102_007_01.h5") else {
        return;
    };

    let metadata = probe_file_metadata(&path).expect("ATL08 metadata probe should succeed");
    assert!(metadata.top_level_groups.iter().any(|group| group == "gt1l"));

    let dataset = resolve_dataset_in_file(&path, "/gt1l/land_segments/canopy/h_canopy")
        .expect("ATL08 fixture should expose canonical canopy-height path marker");
    assert_eq!(dataset.path, "/gt1l/land_segments/canopy/h_canopy");
}

#[test]
fn gedi_fixture_dir_smoke_discovers_beam_groups() {
    let Some(path) = fixture_named("GEDI02_A_2025190205730_O37237_01_T04940_02_004_02_V002.h5") else {
        return;
    };

    let metadata = probe_file_metadata(&path).expect("GEDI metadata probe should succeed");
    assert!(
        metadata
            .top_level_groups
            .iter()
            .any(|group| group == "BEAM0000")
    );

    let dataset = resolve_dataset_in_file(&path, "/BEAM0000/shot_number")
        .expect("GEDI fixture should expose canonical BEAM0000 shot-number path marker");
    assert_eq!(dataset.path, "/BEAM0000/shot_number");
}

#[test]
fn atl08_documented_field_vocabulary_is_discoverable_with_reports() {
    let Some(path) = fixture_named("ATL08_20181120185605_08120102_007_01.h5") else {
        return;
    };

    let report = dataset_metadata_text_report_in_file(
        &path,
        "/gt1l/land_segments/canopy/h_canopy",
        &["gt1l", "land_segments", "canopy", "h_canopy"],
    )
    .expect("ATL08 metadata report should succeed");
    assert!(
        report.missing_terms.is_empty(),
        "ATL08 documented vocabulary should be discoverable; present={:?}, missing={:?}",
        report.present_terms,
        report.missing_terms,
    );
}

#[test]
fn gedi_documented_field_vocabulary_is_discoverable_with_reports() {
    let Some(path) = fixture_named("GEDI02_A_2025190205730_O37237_01_T04940_02_004_02_V002.h5") else {
        return;
    };

    let report = dataset_metadata_text_report_in_file(
        &path,
        "/BEAM0000/elev_lowestmode",
        &["BEAM0000", "shot_number", "elev_lowestmode"],
    )
    .expect("GEDI metadata report should succeed");
    assert!(
        report.missing_terms.is_empty(),
        "GEDI documented vocabulary should be discoverable; present={:?}, missing={:?}",
        report.present_terms,
        report.missing_terms,
    );
}

#[test]
fn atl08_fixture_dir_object_header_probe_finds_signatures() {
    let Some(path) = fixture_named("ATL08_20181120185605_08120102_007_01.h5") else {
        return;
    };

    let object_headers =
        probe_file_object_headers(&path).expect("ATL08 fixture should expose object header signatures");
    assert!(!object_headers.signature_offsets.is_empty());
    assert!(
        !object_headers.v2_headers.is_empty(),
        "ATL08 fixture should expose at least one parsable v2 object header"
    );

    let first_header = &object_headers.v2_headers[0];
    let message_ids: Vec<u8> = first_header.messages.iter().map(|message| message.type_id).collect();
    assert_eq!(message_ids, vec![0x01, 0x03, 0x05, 0x10]);
    assert_eq!(first_header.dataspaces.len(), 1);
    assert_eq!(first_header.dataspaces[0].version, 2);
    assert_eq!(first_header.dataspaces[0].rank, 1);
    assert_eq!(first_header.dataspaces[0].dimensions, vec![1]);
    assert_eq!(first_header.dataspaces[0].max_dimensions, vec![1]);
    assert_eq!(first_header.datatypes.len(), 1);
    assert_eq!(first_header.datatypes[0].version, 1);
    assert_eq!(first_header.datatypes[0].class, 3);
    assert_eq!(first_header.datatypes[0].size, 38_726);
    assert_eq!(first_header.continuations.len(), 1);
    assert_eq!(first_header.continuations[0].address, 144130);
    assert_eq!(first_header.continuations[0].size, 52);

    let chunk1 = parse_continuation_chunk_in_file(&path, &first_header.continuations[0])
        .expect("ATL08 first continuation chunk should parse");
    let chunk1_message_ids: Vec<u8> = chunk1.messages.iter().map(|message| message.type_id).collect();
    assert_eq!(chunk1_message_ids, vec![0x10, 0x15]);
    assert_eq!(chunk1.continuations.len(), 1);
    assert_eq!(chunk1.continuations[0].address, 153351);
    assert_eq!(chunk1.continuations[0].size, 160);

    let chunk2 = parse_continuation_chunk_in_file(&path, &chunk1.continuations[0])
        .expect("ATL08 second continuation chunk should parse");
    assert!(!chunk2.layouts.is_empty());
    assert_eq!(chunk2.layouts[0].version, 3);
    assert_eq!(chunk2.layouts[0].layout_class, 1);
    assert_eq!(chunk2.layouts[0].data_address, 8_500_138);
    assert_eq!(chunk2.layouts[0].data_size, 38_726);

    let payload = read_contiguous_layout_bytes_in_file(&path, &chunk2.layouts[0])
        .expect("ATL08 contiguous layout payload should be readable");
    assert_eq!(payload.len(), 38_726);
    assert!(payload.starts_with(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));

    let prefix = decode_fixed_string(&payload[..128]).expect("ATL08 XML prefix should decode as UTF-8");
    assert!(prefix.contains("<gmd:DS_Series"));
}

#[test]
fn gedi_fixture_dir_object_header_probe_finds_signatures() {
    let Some(path) = fixture_named("GEDI02_A_2025190205730_O37237_01_T04940_02_004_02_V002.h5") else {
        return;
    };

    let object_headers =
        probe_file_object_headers(&path).expect("GEDI fixture should expose object header signatures");
    assert!(!object_headers.signature_offsets.is_empty());
    // Current GEDI scan discovers OHDR markers but does not yet guarantee a parsable
    // v2 prefix at those offsets. Full object-header message traversal is pending.
}

#[test]
fn gedi_elev_lowestmode_contiguous_window_matches_h5dump_reference() {
    let Some(path) = fixture_named("GEDI02_A_2025190205730_O37237_01_T04940_02_004_02_V002.h5") else {
        return;
    };

    // Reference values extracted with:
    // h5dump -d /BEAM0000/elev_lowestmode -s 0 -c 12 -m "%.8f" <fixture>
    let expected = vec![
        7373.83593750,
        7373.16357422,
        7373.08935547,
        7373.01513672,
        7373.53857422,
        7373.46435547,
        7372.79248047,
        7372.71777344,
        7372.64355469,
        7373.16650391,
        7373.09228516,
        7372.41943359,
    ];

    let actual = read_contiguous_f32_window_in_file(&path, 1_012_683, expected.len(), Endianness::Little)
        .expect("GEDI elev_lowestmode contiguous f32 window should decode");
    let summary = compare_f32_with_tolerance(&actual, &expected, 1e-5)
        .expect("GEDI first-window comparison should succeed");

    assert_eq!(summary.compared_len, expected.len());
    assert_eq!(summary.mismatches, 0);
    assert!(summary.max_abs_diff <= 1e-5);
}

#[test]
fn gedi_new_granule_fixture_discovers_beam_and_elev_paths() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/spaceborne_lidar/GEDI02_A_2025190205730_O37237_01_T04940_02_004_02_V002.h5",
    );
    if !path.is_file() {
        return;
    }

    let metadata = probe_file_metadata(path).expect("new GEDI metadata probe should succeed");
    assert!(metadata.superblock_version <= 3);
    assert!(metadata.top_level_groups.iter().any(|group| group == "BEAM0000"));

    let shot_number = resolve_dataset_in_file(path, "/BEAM0000/shot_number")
        .expect("new GEDI shot_number dataset should be discoverable by path markers");
    assert_eq!(shot_number.path, "/BEAM0000/shot_number");

    let elev_lowestmode = resolve_dataset_in_file(path, "/BEAM0000/elev_lowestmode")
        .expect("new GEDI elev_lowestmode dataset should be discoverable by path markers");
    assert_eq!(elev_lowestmode.path, "/BEAM0000/elev_lowestmode");
}

#[test]
fn viirs_vnp13_xdim_contiguous_window_matches_h5dump_reference() {
    let Some(path) = viirs_fixture_named("VNP13A4N.A2026150.h12v04.002.2026151015223.h5") else {
        return;
    };

    let xdim = resolve_dataset_in_file(&path, "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/XDim")
        .expect("VIIRS VNP13 XDim dataset should be discoverable by path markers");
    assert_eq!(xdim.path, "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/XDim");

    let ydim = resolve_dataset_in_file(&path, "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/YDim")
        .expect("VIIRS VNP13 YDim dataset should be discoverable by path markers");
    assert_eq!(ydim.path, "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/YDim");

    let ndvi = resolve_dataset_in_file(
        &path,
        "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days NDVI",
    )
    .expect("VIIRS VNP13 NDVI dataset should be discoverable by path markers");
    assert_eq!(ndvi.path, "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days NDVI");

    let evi = resolve_dataset_in_file(
        &path,
        "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days EVI",
    )
    .expect("VIIRS VNP13 EVI dataset should be discoverable by path markers");
    assert_eq!(evi.path, "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days EVI");

    let evi2 = resolve_dataset_in_file(
        &path,
        "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days EVI2",
    )
    .expect("VIIRS VNP13 EVI2 dataset should be discoverable by path markers");
    assert_eq!(evi2.path, "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days EVI2");

    // Reference values extracted with:
    // h5dump -d '/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/XDim' -s 0 -c 8 -m '%.8f' <fixture>
    let expected = [
        -6671703.11800000,
        -6671239.80528347,
        -6670776.49256694,
        -6670313.17985042,
        -6669849.86713389,
        -6669386.55441736,
        -6668923.24170083,
        -6668459.92898430,
    ];

    let actual = read_contiguous_f64_window_in_file(&path, 78_857, expected.len(), Endianness::Little)
        .expect("VIIRS VNP13 XDim contiguous f64 window should decode");

    let summary = compare_f64_with_tolerance(&actual, &expected, 1e-8)
        .expect("VIIRS XDim first-window comparison should succeed");
    assert_eq!(summary.compared_len, expected.len());
    assert_eq!(summary.mismatches, 0);
    assert!(summary.max_abs_diff <= 1e-8);

    let overlap = read_contiguous_f64_window_in_file(&path, 78_857 + 4 * 8, 4, Endianness::Little)
        .expect("VIIRS VNP13 XDim overlapping f64 window should decode");
    assert_eq!(overlap, expected[4..8].to_vec());
}

#[test]
fn viirs_vnp13_documented_vi_field_vocabulary_is_discoverable_with_reports() {
    let Some(path) = viirs_fixture_named("VNP13A4N.A2026150.h12v04.002.2026151015223.h5") else {
        return;
    };

    let ndvi_report = dataset_metadata_text_report_in_file(
        &path,
        "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days NDVI",
        &["500 m 8 days NDVI", "NDVI", "Projection"],
    )
    .expect("VNP13 NDVI metadata report should succeed");
    assert!(
        ndvi_report.missing_terms.is_empty(),
        "VNP13 NDVI documented vocabulary should be discoverable; present={:?}, missing={:?}",
        ndvi_report.present_terms,
        ndvi_report.missing_terms,
    );

    let evi_report = dataset_metadata_text_report_in_file(
        &path,
        "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days EVI",
        &["500 m 8 days EVI", "EVI", "Projection"],
    )
    .expect("VNP13 EVI metadata report should succeed");
    assert!(
        evi_report.missing_terms.is_empty(),
        "VNP13 EVI documented vocabulary should be discoverable; present={:?}, missing={:?}",
        evi_report.present_terms,
        evi_report.missing_terms,
    );

    let evi2_report = dataset_metadata_text_report_in_file(
        &path,
        "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days EVI2",
        &["500 m 8 days EVI2", "EVI2", "Projection"],
    )
    .expect("VNP13 EVI2 metadata report should succeed");
    assert!(
        evi2_report.missing_terms.is_empty(),
        "VNP13 EVI2 documented vocabulary should be discoverable; present={:?}, missing={:?}",
        evi2_report.present_terms,
        evi2_report.missing_terms,
    );
}

#[test]
fn viirs_vnp13_ndvi_first_chunk_decodes_h5dump_reference_prefix() {
    let Some(path) = viirs_fixture_named("VNP13A4N.A2026150.h12v04.002.2026151015223.h5") else {
        return;
    };

    let ndvi = resolve_dataset_in_file(
        &path,
        "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days NDVI",
    )
    .expect("VIIRS VNP13 NDVI dataset should be discoverable before payload decode attempt");
    assert_eq!(ndvi.path, "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days NDVI");

    // Reference values extracted with:
    // h5dump -d '/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days NDVI' -s '0,0' -c '1,12' <fixture>
    let expected_prefix = vec![6177_i16, 6384, 5691, 5145, 4970, 5386, 5606, 5852, 5866, 6390, 5630, 5729];

    // h5debug reports this NDVI dataset as a v2 object header at offset 1570 with
    // layout: chunked v1 B-tree index at address 112552 and logical chunk size {1, 2400, 2}.
    let mut matched = false;
    for row_dim in 0..3 {
        let decoded = decode_chunked_i16_row_prefix_in_file(
            &path,
            "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days NDVI",
            112_552,
            3,
            row_dim,
            0,
            2_400,
            Endianness::Little,
            64,
            2_400,
        );
        let Ok(values) = decoded else {
            continue;
        };
        if values.len() >= expected_prefix.len() && values[..expected_prefix.len()] == expected_prefix {
            matched = true;
            break;
        }
    }

    assert!(
        matched,
        "VNP13 NDVI first-row prefix could not be reproduced through bounded chunked i16 row-prefix decode"
    );
}

#[test]
fn viirs_vnp13_ndvi_two_row_prefix_matches_h5dump_reference() {
    let Some(path) = viirs_fixture_named("VNP13A4N.A2026150.h12v04.002.2026151015223.h5") else {
        return;
    };

    let row0_expected = vec![6177_i16, 6384, 5691, 5145, 4970, 5386, 5606, 5852, 5866, 6390, 5630, 5729];
    let row1_expected = vec![6440_i16, 6052, 5847, 4909, 5304, 5519, 5338, 5356, 5707, 6228, 6288, 5215];

    let mut validated = false;
    for row_dim in 0..3 {
        let row0_values = decode_chunked_i16_row_prefix_in_file(
            &path,
            "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days NDVI",
            112_552,
            3,
            row_dim,
            0,
            row0_expected.len(),
            Endianness::Little,
            64,
            2_400,
        );
        let row1_values = decode_chunked_i16_row_prefix_in_file(
            &path,
            "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days NDVI",
            112_552,
            3,
            row_dim,
            1,
            row1_expected.len(),
            Endianness::Little,
            64,
            2_400,
        );

        let (Ok(row0_values), Ok(row1_values)) = (row0_values, row1_values) else {
            continue;
        };

        if row0_values == row0_expected && row1_values == row1_expected {
            validated = true;
            break;
        }
    }

    assert!(
        validated,
        "VNP13 NDVI bounded multi-row chunk decode did not match h5dump references for row 0 and row 1"
    );
}

#[test]
fn viirs_vnp13_evi_and_evi2_row_prefix_match_h5dump_reference() {
    let Some(path) = viirs_fixture_named("VNP13A4N.A2026150.h12v04.002.2026151015223.h5") else {
        return;
    };

    let evi_path = "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days EVI";
    let evi2_path = "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days EVI2";
    let evi_expected = vec![2304_i16, 2338, 1999, 1463, 1786, 2017, 2007, 2148, 2058, 2190, 2094, 2241];
    let evi2_expected = vec![2263_i16, 2288, 1966, 1364, 1702, 1887, 1964, 2126, 2019, 2125, 1986, 2110];

    let mut evi_matched = false;
    for row_dim in 0..3 {
        let decoded = decode_chunked_i16_row_prefix_in_file(
            &path,
            evi_path,
            115_168,
            3,
            row_dim,
            0,
            evi_expected.len(),
            Endianness::Little,
            64,
            2_400,
        );
        let Ok(values) = decoded else {
            continue;
        };
        if values == evi_expected {
            evi_matched = true;
            break;
        }
    }
    assert!(
        evi_matched,
        "VNP13 EVI first-row prefix did not match h5dump reference via bounded chunked i16 row-prefix decode"
    );

    let mut evi2_matched = false;
    for row_dim in 0..3 {
        let decoded = decode_chunked_i16_row_prefix_in_file(
            &path,
            evi2_path,
            117_784,
            3,
            row_dim,
            0,
            evi2_expected.len(),
            Endianness::Little,
            64,
            2_400,
        );
        let Ok(values) = decoded else {
            continue;
        };
        if values == evi2_expected {
            evi2_matched = true;
            break;
        }
    }
    assert!(
        evi2_matched,
        "VNP13 EVI2 first-row prefix did not match h5dump reference via bounded chunked i16 row-prefix decode"
    );
}

#[test]
fn viirs_vnp13_ndvi_row_major_window_matches_h5dump_reference() {
    let Some(path) = viirs_fixture_named("VNP13A4N.A2026150.h12v04.002.2026151015223.h5") else {
        return;
    };

    // Reference values extracted with:
    // h5dump -d '/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days NDVI' -s '0,5' -c '2,4' <fixture>
    let expected = vec![5386_i16, 5606, 5852, 5866, 5519, 5338, 5356, 5707];

    let mut matched = false;
    for row_dim in 0..3 {
        let decoded = decode_chunked_i16_row_major_window_in_file(
            &path,
            "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days NDVI",
            112_552,
            3,
            row_dim,
            0,
            2,
            5,
            4,
            2_400,
            Endianness::Little,
            64,
            2_400,
        );
        let Ok(values) = decoded else {
            continue;
        };
        if values == expected {
            matched = true;
            break;
        }
    }

    assert!(
        matched,
        "VNP13 NDVI row-major 2D window decode did not match h5dump reference"
    );
}

#[test]
fn viirs_vnp13_ndvi_bounded_chunk_index_probe_returns_expected_chunk_records() {
    let Some(path) = viirs_fixture_named("VNP13A4N.A2026150.h12v04.002.2026151015223.h5") else {
        return;
    };

    let ndvi = resolve_dataset_in_file(
        &path,
        "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days NDVI",
    )
    .expect("VNP13 NDVI dataset should be discoverable before chunk-index probe");
    assert_eq!(ndvi.path, "/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/Data Fields/500 m 8 days NDVI");

    let tree_address = 112_552_usize;
    let header_len = wbhdf::btree::NODE_HEADER_LEN;
    let bytes = std::fs::read(&path).expect("VNP13 fixture should be readable for header probe");
    assert!(
        bytes.len() >= tree_address + header_len,
        "VNP13 fixture should include NDVI chunk-index node header bytes"
    );
    let header = parse_node_header(&bytes[tree_address..tree_address + header_len])
        .expect("VNP13 NDVI chunk-index root header should parse");
    assert!(
        header.node_level > 0,
        "VNP13 NDVI chunk-index root should be non-leaf for multilevel traversal evidence"
    );

    let records = read_chunked_storage_records_bounded_in_file(&path, 112_552, 3, 64, 2_400)
        .expect("VNP13 NDVI bounded chunk-index probe should return chunk records");

    assert!(!records.is_empty());
    assert!(
        records.len() >= 8,
        "VNP13 NDVI multilevel traversal should return a meaningful chunk-record set"
    );

    let nonorigin_record = records
        .iter()
        .find(|record| {
            record.chunk_offsets.len() >= 2
                && ((record.chunk_offsets[0] == 0 && record.chunk_offsets[1] == 0)
                    || (record.chunk_offsets[1] == 0 && record.chunk_offsets[0] == 0))
        })
        .expect("VNP13 NDVI chunk records should include the origin chunk-offset record");

    assert!(nonorigin_record.chunk_size > 0);
    assert!(nonorigin_record.chunk_address > 0);

    let compressed = read_chunk_payload_in_file(&path, nonorigin_record.chunk_address, nonorigin_record.chunk_size)
        .expect("VNP13 NDVI origin chunk payload should be readable");
    let decompressed = decompress_zlib(&compressed)
        .expect("VNP13 NDVI origin chunk payload should zlib-decompress");
    assert!(!decompressed.is_empty());
    assert_eq!(decompressed.len() % 2, 0);

    let decoded = wbhdf::datatypes::decode_i16_slice(&decompressed, Endianness::Little)
        .expect("VNP13 NDVI origin chunk payload should decode as little-endian i16 values");
    assert!(
        decoded.len() >= 2_400,
        "decoded VNP13 NDVI chunk should include at least one full chunk-width row"
    );
    assert!(
        decoded.iter().any(|v| *v == 5_386_i16 || *v == 5_606_i16 || *v == 5_852_i16),
        "decoded VNP13 NDVI chunk should include known NDVI reference-like values"
    );
}

#[test]
fn viirs_vnp09_hdf4_eos_metadata_probe_enumerates_expected_fields() {
    let Some(path) = hdf4_example_fixture_named("VNP09_NRT.A2026150.1906.002.2026150222127.hdf") else {
        return;
    };

    let summary = probe_hdf4_eos_metadata_in_file(&path)
        .expect("VNP09 HDF4 EOS metadata probe should succeed");
    assert!(summary.struct_metadata_markers >= 1);
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "375m Surface Reflectance Band I1")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "land_water_mask")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "QF1 Surface Reflectance")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "375m Surface Reflectance Band I2")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "750m Surface Reflectance Band M1")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "375m Surface Reflectance Band I3")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "750m Surface Reflectance Band M2")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "QF2 Surface Reflectance")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "750m Surface Reflectance Band M3")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "QF3 Surface Reflectance")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "750m Surface Reflectance Band M4")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "QF4 Surface Reflectance")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "750m Surface Reflectance Band M5")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "750m Surface Reflectance Band M7")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "QF5 Surface Reflectance")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "QF6 Surface Reflectance")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "750m Surface Reflectance Band M8")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "QF7 Surface Reflectance")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "750m Surface Reflectance Band M10")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "750m Surface Reflectance Band M11")
    );
}

#[test]
fn viirs_vnp09_documented_swath_vocabulary_is_discoverable_with_reports() {
    let Some(path) = hdf4_example_fixture_named("VNP09_NRT.A2026150.1906.002.2026150222127.hdf") else {
        return;
    };

    let i_band_report = dataset_metadata_text_report_in_file(
        &path,
        "DataFieldName=\"375m Surface Reflectance Band I1\"",
        &[
            "375m Surface Reflectance Band I1",
            "375m Surface Reflectance Band I2",
            "375m Surface Reflectance Band I3",
        ],
    )
    .expect("VNP09 I-band metadata report should succeed");
    assert!(
        i_band_report.missing_terms.is_empty(),
        "VNP09 I-band documented vocabulary should be discoverable; present={:?}, missing={:?}",
        i_band_report.present_terms,
        i_band_report.missing_terms,
    );

    let m_band_report = dataset_metadata_text_report_in_file(
        &path,
        "DataFieldName=\"750m Surface Reflectance Band M1\"",
        &[
            "750m Surface Reflectance Band M1",
            "750m Surface Reflectance Band M11",
            "land_water_mask",
        ],
    )
    .expect("VNP09 M-band metadata report should succeed");
    assert!(
        m_band_report.missing_terms.is_empty(),
        "VNP09 M-band documented vocabulary should be discoverable; present={:?}, missing={:?}",
        m_band_report.present_terms,
        m_band_report.missing_terms,
    );

    let qf_report = dataset_metadata_text_report_in_file(
        &path,
        "DataFieldName=\"QF1 Surface Reflectance\"",
        &[
            "QF1 Surface Reflectance",
            "QF4 Surface Reflectance",
            "QF7 Surface Reflectance",
        ],
    )
    .expect("VNP09 QF metadata report should succeed");
    assert!(
        qf_report.missing_terms.is_empty(),
        "VNP09 QF documented vocabulary should be discoverable; present={:?}, missing={:?}",
        qf_report.present_terms,
        qf_report.missing_terms,
    );
}

#[test]
fn viirs_vnp21_netcdf_metadata_probe_discovers_swath_group_and_lst_path() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let metadata = probe_file_metadata(path).expect("VNP21 metadata probe should succeed");
    assert!(metadata.superblock_version <= 3);
    assert!(
        metadata
            .top_level_groups
            .iter()
            .any(|group| group == "VIIRS_Swath_LSTE")
    );

    let descriptor = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/LST")
        .expect("VNP21 LST dataset should be discoverable by path markers");
    assert_eq!(descriptor.path, "/VIIRS_Swath_LSTE/Data Fields/LST");

    let latitude = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Geolocation Fields/latitude")
        .expect("VNP21 latitude dataset should be discoverable by path markers");
    assert_eq!(latitude.path, "/VIIRS_Swath_LSTE/Geolocation Fields/latitude");

    let longitude = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Geolocation Fields/longitude")
        .expect("VNP21 longitude dataset should be discoverable by path markers");
    assert_eq!(longitude.path, "/VIIRS_Swath_LSTE/Geolocation Fields/longitude");

    let lst_err = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/LST_err")
        .expect("VNP21 LST_err dataset should be discoverable by path markers");
    assert_eq!(lst_err.path, "/VIIRS_Swath_LSTE/Data Fields/LST_err");

    let view_angle = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/View_angle")
        .expect("VNP21 View_angle dataset should be discoverable by path markers");
    assert_eq!(view_angle.path, "/VIIRS_Swath_LSTE/Data Fields/View_angle");

    let emis_14 = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/Emis_14")
        .expect("VNP21 Emis_14 dataset should be discoverable by path markers");
    assert_eq!(emis_14.path, "/VIIRS_Swath_LSTE/Data Fields/Emis_14");

    let emis_14_err = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/Emis_14_err")
        .expect("VNP21 Emis_14_err dataset should be discoverable by path markers");
    assert_eq!(emis_14_err.path, "/VIIRS_Swath_LSTE/Data Fields/Emis_14_err");

    let emis_15 = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/Emis_15")
        .expect("VNP21 Emis_15 dataset should be discoverable by path markers");
    assert_eq!(emis_15.path, "/VIIRS_Swath_LSTE/Data Fields/Emis_15");

    let emis_16 = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/Emis_16")
        .expect("VNP21 Emis_16 dataset should be discoverable by path markers");
    assert_eq!(emis_16.path, "/VIIRS_Swath_LSTE/Data Fields/Emis_16");

    let emis_aster = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/Emis_ASTER")
        .expect("VNP21 Emis_ASTER dataset should be discoverable by path markers");
    assert_eq!(emis_aster.path, "/VIIRS_Swath_LSTE/Data Fields/Emis_ASTER");

    let emis_15_err = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/Emis_15_err")
        .expect("VNP21 Emis_15_err dataset should be discoverable by path markers");
    assert_eq!(emis_15_err.path, "/VIIRS_Swath_LSTE/Data Fields/Emis_15_err");

    let emis_16_err = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/Emis_16_err")
        .expect("VNP21 Emis_16_err dataset should be discoverable by path markers");
    assert_eq!(emis_16_err.path, "/VIIRS_Swath_LSTE/Data Fields/Emis_16_err");

}

#[test]
fn viirs_vnp21_lst_row_major_window_matches_h5dump_reference() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let lst = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/LST")
        .expect("VNP21 LST dataset should be discoverable before payload decode attempt");
    assert_eq!(lst.path, "/VIIRS_Swath_LSTE/Data Fields/LST");

    // Reference values extracted with:
    // h5dump -d '/VIIRS_Swath_LSTE/Data Fields/LST' -s '800,1600' -c '2,4' <fixture>
    let expected = vec![14007_u16, 14046, 14028, 13983, 14055, 14036, 14055, 13953];

    let decoded = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/LST",
        65_387_786,
        3,
        0,
        800,
        2,
        1600,
        4,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 LST row-major window should decode through v1 chunk-index traversal");

    assert_eq!(decoded, expected);
}

#[test]
fn viirs_vnp21_lst_bounded_chunk_index_probe_returns_expected_chunk_records() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let lst = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/LST")
        .expect("VNP21 LST dataset should be discoverable before chunk-index probe");
    assert_eq!(lst.path, "/VIIRS_Swath_LSTE/Data Fields/LST");

    let tree_address = 65_387_786_usize;
    let header_len = wbhdf::btree::NODE_HEADER_LEN;
    let bytes = std::fs::read(path).expect("VNP21 fixture should be readable for header probe");
    assert!(
        bytes.len() >= tree_address + header_len,
        "VNP21 fixture should include LST chunk-index node header bytes"
    );
    let header = parse_node_header(&bytes[tree_address..tree_address + header_len])
        .expect("VNP21 LST chunk-index root header should parse");
    assert!(
        header.node_level > 0,
        "VNP21 LST chunk-index root should be non-leaf for multilevel traversal evidence"
    );

    let records = read_chunked_storage_records_bounded_in_file(
        path,
        65_387_786,
        3,
        512,
        8_192,
    )
    .expect("VNP21 LST bounded chunk-index probe should return chunk records");

    assert!(!records.is_empty());
    assert!(
        records.len() >= 128,
        "VNP21 LST multilevel traversal should return a substantial chunk-record set"
    );

    let record_for_col = |col_offset: u64| {
        records.iter().find(|record| {
            record.chunk_offsets.len() >= 2
                && ((record.chunk_offsets[0] == 0 && record.chunk_offsets[1] == col_offset)
                    || (record.chunk_offsets[1] == 0 && record.chunk_offsets[0] == col_offset))
        })
    };

    let nonorigin_col_record = record_for_col(976)
        .expect("chunk records should include the non-origin window column offset (976)");
    assert!(nonorigin_col_record.chunk_size > 0);
    assert!(nonorigin_col_record.chunk_address > 0);
    assert!(
        record_for_col(1_600).is_some(),
        "chunk records should include the row-major reference window column offset (1600)"
    );
    assert!(
        record_for_col(2_496).is_some(),
        "chunk records should include the inland/fill window column offset (2496)"
    );

    let ref_chunk_record = record_for_col(1_600)
        .expect("reference-window chunk offset (1600) should resolve to a chunk record");
    let compressed = read_chunk_payload_in_file(path, ref_chunk_record.chunk_address, ref_chunk_record.chunk_size)
        .expect("VNP21 LST reference chunk payload should be readable");
    let decompressed = decompress_zlib(&compressed)
        .expect("VNP21 LST reference chunk payload should zlib-decompress");
    assert_eq!(decompressed.len(), 102_400);

    let decoded = wbhdf::datatypes::decode_u16_slice(&decompressed, Endianness::Little)
        .expect("VNP21 LST reference chunk payload should decode as little-endian u16 values");
    assert_eq!(decoded.len(), 51_200);

    let non_zero_count = decoded.iter().filter(|v| **v != 0).count();
    assert!(
        non_zero_count > 100,
        "decoded VNP21 chunk should contain non-zero data values"
    );
    assert!(
        decoded.contains(&14007_u16),
        "decoded VNP21 chunk should include known LST reference values"
    );
}

#[test]
fn viirs_vnp21_lst_err_row_major_window_and_semantics_match_h5dump_reference() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let lst_err = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/LST_err")
        .expect("VNP21 LST_err dataset should be discoverable before payload decode attempt");
    assert_eq!(lst_err.path, "/VIIRS_Swath_LSTE/Data Fields/LST_err");

    // Reference values extracted with:
    // h5dump -d '/VIIRS_Swath_LSTE/Data Fields/LST_err' -s '800,1600' -c '2,4' <fixture>
    let expected_raw = vec![22_u8, 22, 22, 22, 22, 22, 22, 22];

    let decoded = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/LST_err",
        78_971_646,
        3,
        0,
        800,
        2,
        1600,
        4,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 LST_err row-major window should decode through v1 chunk-index traversal");

    assert_eq!(decoded, expected_raw);

    // Semantics from h5dump attributes:
    // _FillValue=0, valid_range=[1,255], scale_factor=0.04, add_offset=0.
    assert!(decoded.iter().all(|v| *v >= 1));
    let scaled: Vec<f64> = decoded.iter().map(|v| *v as f64 * 0.04).collect();
    let expected_scaled = vec![0.88_f64; expected_raw.len()];
    for (idx, (actual, expected)) in scaled.iter().zip(expected_scaled.iter()).enumerate() {
        let diff = (actual - expected).abs();
        assert!(
            diff <= 1e-12,
            "VNP21 LST_err scaled mismatch at index {idx}: actual={actual}, expected={expected}, abs_diff={diff}"
        );
    }
}

#[test]
fn viirs_vnp21_view_angle_row_major_window_and_semantics_match_h5dump_reference() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let view_angle = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/View_angle")
        .expect("VNP21 View_angle dataset should be discoverable before payload decode attempt");
    assert_eq!(view_angle.path, "/VIIRS_Swath_LSTE/Data Fields/View_angle");

    // Reference values extracted with:
    // h5dump -d '/VIIRS_Swath_LSTE/Data Fields/View_angle' -s '1234,987' -c '2,6' <fixture>
    let expected_raw = vec![75_u8, 75, 75, 74, 74, 74, 75, 75, 75, 74, 74, 74];

    let decoded = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/View_angle",
        88_316_419,
        3,
        0,
        1234,
        2,
        987,
        6,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 View_angle row-major window should decode through v1 chunk-index traversal");

    assert_eq!(decoded, expected_raw);

    // Semantics from h5dump attributes:
    // _FillValue=255, valid_range=[0,180], scale_factor=0.5, add_offset=0.
    assert!(decoded.iter().all(|v| *v <= 180));
    let scaled: Vec<f64> = decoded.iter().map(|v| *v as f64 * 0.5).collect();
    let expected_scaled = vec![
        37.5_f64, 37.5_f64, 37.5_f64, 37.0_f64, 37.0_f64, 37.0_f64, 37.5_f64, 37.5_f64,
        37.5_f64, 37.0_f64, 37.0_f64, 37.0_f64,
    ];
    for (idx, (actual, expected)) in scaled.iter().zip(expected_scaled.iter()).enumerate() {
        let diff = (actual - expected).abs();
        assert!(
            diff <= 1e-12,
            "VNP21 View_angle scaled mismatch at index {idx}: actual={actual}, expected={expected}, abs_diff={diff}"
        );
    }
}

#[test]
fn viirs_vnp21_emis_aster_row_major_window_and_semantics_match_h5dump_reference() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let emis_aster = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/Emis_ASTER")
        .expect("VNP21 Emis_ASTER dataset should be discoverable before payload decode attempt");
    assert_eq!(emis_aster.path, "/VIIRS_Swath_LSTE/Data Fields/Emis_ASTER");

    // Reference values extracted with:
    // h5dump -d '/VIIRS_Swath_LSTE/Data Fields/Emis_ASTER' -s '800,1600' -c '2,4' <fixture>
    let expected_raw = vec![237_u8, 230, 230, 226, 239, 230, 230, 230];

    let decoded = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/Emis_ASTER",
        76_256_310,
        3,
        0,
        800,
        2,
        1600,
        4,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 Emis_ASTER row-major window should decode through v1 chunk-index traversal");

    assert_eq!(decoded, expected_raw);

    // Semantics from h5dump attributes:
    // _FillValue=0, valid_range=[1,255], scale_factor=0.002, add_offset=0.49.
    assert!(decoded.iter().all(|v| *v >= 1));
    let scaled: Vec<f64> = decoded.iter().map(|v| *v as f64 * 0.002 + 0.49).collect();
    let expected_scaled = vec![
        0.964_f64, 0.950_f64, 0.950_f64, 0.942_f64, 0.968_f64, 0.950_f64, 0.950_f64, 0.950_f64,
    ];
    for (idx, (actual, expected)) in scaled.iter().zip(expected_scaled.iter()).enumerate() {
        let diff = (actual - expected).abs();
        assert!(
            diff <= 1e-12,
            "VNP21 Emis_ASTER scaled mismatch at index {idx}: actual={actual}, expected={expected}, abs_diff={diff}"
        );
    }
}

#[test]
fn viirs_vnp21_latitude_row_major_window_matches_h5dump_reference() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let latitude = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Geolocation Fields/latitude")
        .expect("VNP21 latitude dataset should be discoverable before payload decode attempt");
    assert_eq!(latitude.path, "/VIIRS_Swath_LSTE/Geolocation Fields/latitude");

    // Reference values extracted with:
    // h5dump -d '/VIIRS_Swath_LSTE/Geolocation Fields/latitude' -s '1234,987' -c '2,6' <fixture>
    let expected = vec![
        42.0689_f32, 42.0680, 42.0670, 42.0661, 42.0651, 42.0641, 42.0608, 42.0599, 42.0589,
        42.0580, 42.0570, 42.0561,
    ];

    let decoded = decode_chunked_f32_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Geolocation Fields/latitude",
        5_504,
        3,
        0,
        1234,
        2,
        987,
        6,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 latitude row-major window should decode through v1 chunk-index traversal");

    let summary = compare_f32_with_tolerance(&decoded, &expected, 1e-4)
        .expect("latitude comparison should succeed");
    assert_eq!(summary.mismatches, 0, "latitude mismatches: {summary:?}");
    assert!(decoded.iter().all(|v| *v >= -90.0 && *v <= 90.0));
}

#[test]
fn viirs_vnp21_latitude_bounded_chunk_index_probe_returns_expected_chunk_records() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let latitude = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Geolocation Fields/latitude")
        .expect("VNP21 latitude dataset should be discoverable before chunk-index probe");
    assert_eq!(latitude.path, "/VIIRS_Swath_LSTE/Geolocation Fields/latitude");

    let tree_address = 5_504_usize;
    let header_len = wbhdf::btree::NODE_HEADER_LEN;
    let bytes = std::fs::read(path).expect("VNP21 fixture should be readable for latitude header probe");
    assert!(
        bytes.len() >= tree_address + header_len,
        "VNP21 fixture should include latitude chunk-index node header bytes"
    );
    let header = parse_node_header(&bytes[tree_address..tree_address + header_len])
        .expect("VNP21 latitude chunk-index root header should parse");
    assert!(
        header.node_level > 0,
        "VNP21 latitude chunk-index root should be non-leaf for multilevel traversal evidence"
    );

    let records = read_chunked_storage_records_bounded_in_file(path, 5_504, 3, 512, 8_192)
        .expect("VNP21 latitude bounded chunk-index probe should return chunk records");

    assert!(!records.is_empty());
    assert!(
        records.len() >= 128,
        "VNP21 latitude multilevel traversal should return a substantial chunk-record set"
    );

    let record_for_col = |col_offset: u64| {
        records.iter().find(|record| {
            record.chunk_offsets.len() >= 2
                && ((record.chunk_offsets[0] == 0 && record.chunk_offsets[1] == col_offset)
                    || (record.chunk_offsets[1] == 0 && record.chunk_offsets[0] == col_offset))
        })
    };

    let geoloc_record = record_for_col(976)
        .expect("chunk records should include the non-origin geolocation column offset (976)");
    assert!(geoloc_record.chunk_size > 0);
    assert!(geoloc_record.chunk_address > 0);

    let compressed = read_chunk_payload_in_file(path, geoloc_record.chunk_address, geoloc_record.chunk_size)
        .expect("VNP21 latitude geolocation chunk payload should be readable");
    let decompressed = decompress_zlib(&compressed)
        .expect("VNP21 latitude geolocation chunk payload should zlib-decompress");
    assert_eq!(decompressed.len(), 204_800);

    let decoded = decode_f32_slice(&decompressed, Endianness::Little)
        .expect("VNP21 latitude geolocation chunk payload should decode as little-endian f32 values");
    assert_eq!(decoded.len(), 51_200);

    let valid_geo_count = decoded
        .iter()
        .filter(|v| v.is_finite() && **v >= -90.0 && **v <= 90.0)
        .count();
    assert!(
        valid_geo_count > 1_000,
        "decoded latitude chunk should contain substantial in-range geolocation values"
    );

    let has_reference_like_value = decoded
        .iter()
        .any(|v| (*v - 42.0689_f32).abs() <= 1e-3_f32 || (*v - 42.0608_f32).abs() <= 1e-3_f32);
    assert!(
        has_reference_like_value,
        "decoded latitude chunk should include known reference-like geolocation values"
    );
}

#[test]
fn viirs_vnp21_longitude_row_major_window_matches_h5dump_reference() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let longitude = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Geolocation Fields/longitude")
        .expect("VNP21 longitude dataset should be discoverable before payload decode attempt");
    assert_eq!(longitude.path, "/VIIRS_Swath_LSTE/Geolocation Fields/longitude");

    // Reference values extracted with:
    // h5dump -d '/VIIRS_Swath_LSTE/Geolocation Fields/longitude' -s '1234,987' -c '2,6' <fixture>
    let expected = vec![
        -86.7945_f32,
        -86.7850,
        -86.7754,
        -86.7659,
        -86.7564,
        -86.7469,
        -86.7959,
        -86.7864,
        -86.7769,
        -86.7674,
        -86.7579,
        -86.7484,
    ];

    let decoded = decode_chunked_f32_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Geolocation Fields/longitude",
        31_324_409,
        3,
        0,
        1234,
        2,
        987,
        6,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 longitude row-major window should decode through v1 chunk-index traversal");

    let summary = compare_f32_with_tolerance(&decoded, &expected, 1e-4)
        .expect("longitude comparison should succeed");
    assert_eq!(summary.mismatches, 0, "longitude mismatches: {summary:?}");
    assert!(decoded.iter().all(|v| *v >= -180.0 && *v <= 180.0));
}

#[test]
fn viirs_vnp21_longitude_bounded_chunk_index_probe_returns_expected_chunk_records() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let longitude = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Geolocation Fields/longitude")
        .expect("VNP21 longitude dataset should be discoverable before chunk-index probe");
    assert_eq!(longitude.path, "/VIIRS_Swath_LSTE/Geolocation Fields/longitude");

    let tree_address = 31_324_409_usize;
    let header_len = wbhdf::btree::NODE_HEADER_LEN;
    let bytes = std::fs::read(path).expect("VNP21 fixture should be readable for longitude header probe");
    assert!(
        bytes.len() >= tree_address + header_len,
        "VNP21 fixture should include longitude chunk-index node header bytes"
    );
    let header = parse_node_header(&bytes[tree_address..tree_address + header_len])
        .expect("VNP21 longitude chunk-index root header should parse");
    assert!(
        header.node_level > 0,
        "VNP21 longitude chunk-index root should be non-leaf for multilevel traversal evidence"
    );

    let records = read_chunked_storage_records_bounded_in_file(path, 31_324_409, 3, 512, 8_192)
        .expect("VNP21 longitude bounded chunk-index probe should return chunk records");

    assert!(!records.is_empty());
    assert!(
        records.len() >= 128,
        "VNP21 longitude multilevel traversal should return a substantial chunk-record set"
    );

    let record_for_col = |col_offset: u64| {
        records.iter().find(|record| {
            record.chunk_offsets.len() >= 2
                && ((record.chunk_offsets[0] == 0 && record.chunk_offsets[1] == col_offset)
                    || (record.chunk_offsets[1] == 0 && record.chunk_offsets[0] == col_offset))
        })
    };

    let geoloc_record = record_for_col(976)
        .expect("chunk records should include the non-origin geolocation column offset (976)");
    assert!(geoloc_record.chunk_size > 0);
    assert!(geoloc_record.chunk_address > 0);

    let compressed = read_chunk_payload_in_file(path, geoloc_record.chunk_address, geoloc_record.chunk_size)
        .expect("VNP21 longitude geolocation chunk payload should be readable");
    let decompressed = decompress_zlib(&compressed)
        .expect("VNP21 longitude geolocation chunk payload should zlib-decompress");
    assert_eq!(decompressed.len(), 204_800);

    let decoded = decode_f32_slice(&decompressed, Endianness::Little)
        .expect("VNP21 longitude geolocation chunk payload should decode as little-endian f32 values");
    assert_eq!(decoded.len(), 51_200);

    let valid_geo_count = decoded
        .iter()
        .filter(|v| v.is_finite() && **v >= -180.0 && **v <= 180.0)
        .count();
    assert!(
        valid_geo_count > 1_000,
        "decoded longitude chunk should contain substantial in-range geolocation values"
    );

    let has_reference_like_value = decoded
        .iter()
        .any(|v| (*v + 86.7945_f32).abs() <= 1e-3_f32 || (*v + 86.7484_f32).abs() <= 1e-3_f32);
    assert!(
        has_reference_like_value,
        "decoded longitude chunk should include known reference-like geolocation values"
    );
}

#[test]
fn viirs_vnp21_pwv_row_major_window_and_semantics_match_h5dump_reference() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let pwv = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/PWV")
        .expect("VNP21 PWV dataset should be discoverable before payload decode attempt");
    assert_eq!(pwv.path, "/VIIRS_Swath_LSTE/Data Fields/PWV");

    // Reference values extracted with:
    // h5dump -d '/VIIRS_Swath_LSTE/Data Fields/PWV' -s '1234,987' -c '2,6' <fixture>
    let expected_raw = vec![845_u16, 844, 843, 842, 840, 839, 847, 845, 844, 843, 842, 840];

    let decoded = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/PWV",
        80_778_887,
        3,
        0,
        1234,
        2,
        987,
        6,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 PWV row-major window should decode through v1 chunk-index traversal");

    assert_eq!(decoded, expected_raw);

    // Semantics from h5dump attributes:
    // valid_range=[0,65535], scale_factor=0.001, add_offset=0.
    let scaled: Vec<f64> = decoded.iter().map(|v| *v as f64 * 0.001).collect();
    let expected_scaled = vec![
        0.845_f64, 0.844_f64, 0.843_f64, 0.842_f64, 0.840_f64, 0.839_f64, 0.847_f64, 0.845_f64,
        0.844_f64, 0.843_f64, 0.842_f64, 0.840_f64,
    ];
    for (idx, (actual, expected)) in scaled.iter().zip(expected_scaled.iter()).enumerate() {
        let diff = (actual - expected).abs();
        assert!(
            diff <= 1e-12,
            "VNP21 PWV scaled mismatch at index {idx}: actual={actual}, expected={expected}, abs_diff={diff}"
        );
    }
}

#[test]
fn viirs_vnp21_qc_row_major_window_and_semantics_match_h5dump_reference() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let qc = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/QC")
        .expect("VNP21 QC dataset should be discoverable before payload decode attempt");
    assert_eq!(qc.path, "/VIIRS_Swath_LSTE/Data Fields/QC");

    // Non-origin reference values extracted with:
    // h5dump -d '/VIIRS_Swath_LSTE/Data Fields/QC' -s '1234,987' -c '2,6' <fixture>
    let expected_nonorigin = vec![
        65249_u16, 65216, 65216, 65216, 65216, 65216, 65249, 65216, 65216, 65216, 65216, 65216,
    ];

    let decoded_nonorigin = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/QC",
        70_375_762,
        3,
        0,
        1234,
        2,
        987,
        6,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 QC row-major window should decode through v1 chunk-index traversal");

    assert_eq!(decoded_nonorigin, expected_nonorigin);

    // Origin reference values extracted with:
    // h5dump -d '/VIIRS_Swath_LSTE/Data Fields/QC' -s '800,1600' -c '2,4' <fixture>
    let expected_origin = vec![65216_u16, 65216, 65216, 65216, 65216, 65216, 65216, 65216];

    let decoded_origin = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/QC",
        70_375_762,
        3,
        0,
        800,
        2,
        1600,
        4,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 QC origin window should decode through v1 chunk-index traversal");

    assert_eq!(decoded_origin, expected_origin);

    // Semantics from h5dump attributes:
    // format=unscaled, valid_range=[0,65535], scale_factor=1, add_offset=0.
    assert!(decoded_nonorigin.contains(&65249));
    assert!(decoded_nonorigin.iter().all(|v| *v >= 65216));
    assert!(decoded_origin.iter().all(|v| *v == 65216));
}

#[test]
fn viirs_vnp21_oceanpix_row_major_window_and_semantics_match_h5dump_reference() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let oceanpix = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/oceanpix")
        .expect("VNP21 oceanpix dataset should be discoverable before payload decode attempt");
    assert_eq!(oceanpix.path, "/VIIRS_Swath_LSTE/Data Fields/oceanpix");

    // Non-origin reference values extracted with:
    // h5dump -d '/VIIRS_Swath_LSTE/Data Fields/oceanpix' -s '1234,987' -c '2,6' <fixture>
    let expected_nonorigin = vec![2_u8, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2];

    let decoded_nonorigin = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
        88_214_296,
        3,
        0,
        1234,
        2,
        987,
        6,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 oceanpix row-major window should decode through v1 chunk-index traversal");

    assert_eq!(decoded_nonorigin, expected_nonorigin);

    // Origin reference values extracted with:
    // h5dump -d '/VIIRS_Swath_LSTE/Data Fields/oceanpix' -s '800,1600' -c '2,4' <fixture>
    let expected_origin = vec![0_u8, 0, 0, 0, 0, 0, 0, 0];

    let decoded_origin = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
        88_214_296,
        3,
        0,
        800,
        2,
        1600,
        4,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 oceanpix origin window should decode through v1 chunk-index traversal");

    assert_eq!(decoded_origin, expected_origin);

    // Semantics from h5dump attributes:
    // format=unscaled, valid_range=[0,2], scale_factor=1, add_offset=0.
    assert!(decoded_nonorigin.iter().all(|v| *v <= 2));
    assert!(decoded_nonorigin.contains(&2));
    assert!(decoded_origin.iter().all(|v| *v == 0));
}

#[test]
fn viirs_vnp21_qc_oceanpix_cross_field_bit_pattern_contract_is_stable() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let qc_nonorigin = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/QC",
        70_375_762,
        3,
        0,
        1234,
        2,
        987,
        6,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 QC non-origin window should decode through v1 chunk-index traversal");
    let ocean_nonorigin = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
        88_214_296,
        3,
        0,
        1234,
        2,
        987,
        6,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 oceanpix non-origin window should decode through v1 chunk-index traversal");

    let qc_origin = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/QC",
        70_375_762,
        3,
        0,
        800,
        2,
        1600,
        4,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 QC origin window should decode through v1 chunk-index traversal");
    let ocean_origin = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
        88_214_296,
        3,
        0,
        800,
        2,
        1600,
        4,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 oceanpix origin window should decode through v1 chunk-index traversal");

    assert_eq!(qc_nonorigin.len(), ocean_nonorigin.len());
    assert_eq!(qc_origin.len(), ocean_origin.len());

    // Non-origin window exposes two QC bit patterns while oceanpix is category 2.
    let mut qc_unique = qc_nonorigin.clone();
    qc_unique.sort_unstable();
    qc_unique.dedup();
    assert_eq!(qc_unique, vec![65216_u16, 65249_u16]);
    assert!(ocean_nonorigin.iter().all(|v| *v == 2));

    // Origin window is a single QC bit pattern paired with oceanpix category 0.
    assert!(qc_origin.iter().all(|v| *v == 65216));
    assert!(ocean_origin.iter().all(|v| *v == 0));

    // Bit-pattern invariants for observed QC states in this fixture window pair.
    for value in qc_unique {
        assert_eq!(value & 0xFF00, 0xFE00, "QC high-byte contract changed");
        assert!(
            value == 65216 || value == 65249,
            "unexpected QC state in regression window: {value}"
        );
    }
    assert_eq!(65216_u16 & 0x0021, 0);
    assert_eq!(65249_u16 & 0x0021, 0x0021);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Vnp21QcObservedBits {
    high_byte: u8,
    low_byte: u8,
    low_bit0_set: bool,
    low_bit5_set: bool,
    low_bit6_set: bool,
    low_bit7_set: bool,
}

fn decode_vnp21_qc_observed_bits(value: u16) -> Vnp21QcObservedBits {
    let high_byte = ((value >> 8) & 0x00FF) as u8;
    let low_byte = (value & 0x00FF) as u8;
    Vnp21QcObservedBits {
        high_byte,
        low_byte,
        low_bit0_set: (value & 0x0001) != 0,
        low_bit5_set: (value & 0x0020) != 0,
        low_bit6_set: (value & 0x0040) != 0,
        low_bit7_set: (value & 0x0080) != 0,
    }
}

fn classify_vnp21_qc_observed_profile(bits: Vnp21QcObservedBits) -> &'static str {
    match (bits.high_byte, bits.low_byte) {
        (0xFE, 0xC0) => "fe_c0_baseline",
        (0xFE, 0xE1) => "fe_e1_elevated",
        (0xFA, 0xC0) => "fa_c0_baseline_alt",
        (0x00, 0x07) => "00_07_inland_water",
        _ => "unknown",
    }
}

#[test]
fn viirs_vnp21_qc_observed_profile_classifier_maps_known_and_unknown_states() {
    let known_fe_c0 = decode_vnp21_qc_observed_bits(65216_u16);
    let known_fe_e1 = decode_vnp21_qc_observed_bits(65249_u16);
    let known_fa_c0 = decode_vnp21_qc_observed_bits(64192_u16);
    let known_00_07 = decode_vnp21_qc_observed_bits(7_u16);
    let unknown_state = decode_vnp21_qc_observed_bits(65535_u16);

    assert_eq!(classify_vnp21_qc_observed_profile(known_fe_c0), "fe_c0_baseline");
    assert_eq!(classify_vnp21_qc_observed_profile(known_fe_e1), "fe_e1_elevated");
    assert_eq!(classify_vnp21_qc_observed_profile(known_fa_c0), "fa_c0_baseline_alt");
    assert_eq!(classify_vnp21_qc_observed_profile(known_00_07), "00_07_inland_water");
    assert_eq!(classify_vnp21_qc_observed_profile(unknown_state), "unknown");
}

#[test]
fn viirs_vnp21_qc_oceanpix_inland_window_profile_contract_is_stable() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    // Reference values extracted with:
    // h5dump -d '/VIIRS_Swath_LSTE/Data Fields/QC' -s '1500,2500' -c '2,4' <fixture>
    // h5dump -d '/VIIRS_Swath_LSTE/Data Fields/oceanpix' -s '1500,2500' -c '2,4' <fixture>
    let expected_qc = vec![7_u16, 7, 7, 7, 7, 7, 7, 7];
    let expected_oceanpix = vec![1_u8, 1, 1, 1, 1, 1, 1, 1];

    let qc = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/QC",
        70_375_762,
        3,
        0,
        1500,
        2,
        2500,
        4,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 QC inland window should decode through v1 chunk-index traversal");
    let oceanpix = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
        88_214_296,
        3,
        0,
        1500,
        2,
        2500,
        4,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 oceanpix inland window should decode through v1 chunk-index traversal");

    assert_eq!(qc, expected_qc);
    assert_eq!(oceanpix, expected_oceanpix);

    for value in qc {
        let profile = classify_vnp21_qc_observed_profile(decode_vnp21_qc_observed_bits(value));
        assert_eq!(profile, "00_07_inland_water");
    }
}

#[test]
fn viirs_vnp21_qc_oceanpix_profile_contract_is_exhaustive_across_key_windows() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let windows = [
        (1234_u64, 987_usize, 2_usize, 6_usize),
        (1000_u64, 1007_usize, 2_usize, 10_usize),
        (800_u64, 1600_usize, 2_usize, 4_usize),
        (1500_u64, 2500_usize, 2_usize, 4_usize),
    ];

    use std::collections::BTreeMap;
    let mut counts = BTreeMap::<(&str, u8), usize>::new();

    for (start_row, start_col, num_rows, num_cols) in windows {
        let qc = decode_chunked_u16_row_major_window_in_file(
            path,
            "/VIIRS_Swath_LSTE/Data Fields/QC",
            70_375_762,
            3,
            0,
            start_row,
            num_rows,
            start_col,
            num_cols,
            3_200,
            16,
            Endianness::Little,
            512,
            8_192,
        )
        .expect("VNP21 QC key window should decode through v1 chunk-index traversal");

        let oceanpix = decode_chunked_u8_row_major_window_in_file(
            path,
            "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
            88_214_296,
            3,
            0,
            start_row,
            num_rows,
            start_col,
            num_cols,
            3_200,
            16,
            512,
            8_192,
        )
        .expect("VNP21 oceanpix key window should decode through v1 chunk-index traversal");

        assert_eq!(qc.len(), oceanpix.len());
        for (qc_value, ocean_value) in qc.iter().zip(oceanpix.iter()) {
            let profile = classify_vnp21_qc_observed_profile(decode_vnp21_qc_observed_bits(*qc_value));
            *counts.entry((profile, *ocean_value)).or_insert(0) += 1;
        }
    }

    assert_eq!(counts.get(&("fe_e1_elevated", 2_u8)), Some(&2_usize));
    assert_eq!(counts.get(&("fe_c0_baseline", 2_u8)), Some(&10_usize));
    assert_eq!(counts.get(&("fe_c0_baseline", 0_u8)), Some(&23_usize));
    assert_eq!(counts.get(&("fa_c0_baseline_alt", 0_u8)), Some(&5_usize));
    assert_eq!(counts.get(&("00_07_inland_water", 1_u8)), Some(&8_usize));

    assert!(counts.get(&("unknown", 0_u8)).is_none());
    assert!(counts.get(&("unknown", 1_u8)).is_none());
    assert!(counts.get(&("unknown", 2_u8)).is_none());

    let total: usize = counts.values().sum();
    assert_eq!(total, 48_usize);
}

#[test]
fn viirs_vnp21_qc_profile_bit_invariants_are_stable_across_key_windows() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let windows = [
        (1234_u64, 987_usize, 2_usize, 6_usize),
        (1000_u64, 1007_usize, 2_usize, 10_usize),
        (800_u64, 1600_usize, 2_usize, 4_usize),
        (1500_u64, 2500_usize, 2_usize, 4_usize),
    ];

    for (start_row, start_col, num_rows, num_cols) in windows {
        let qc = decode_chunked_u16_row_major_window_in_file(
            path,
            "/VIIRS_Swath_LSTE/Data Fields/QC",
            70_375_762,
            3,
            0,
            start_row,
            num_rows,
            start_col,
            num_cols,
            3_200,
            16,
            Endianness::Little,
            512,
            8_192,
        )
        .expect("VNP21 QC key window should decode through v1 chunk-index traversal");

        for value in qc {
            let bits = decode_vnp21_qc_observed_bits(value);
            let profile = classify_vnp21_qc_observed_profile(bits);

            match profile {
                "fe_c0_baseline" => {
                    assert_eq!(bits.high_byte, 0xFE);
                    assert_eq!(bits.low_byte, 0xC0);
                    assert!(!bits.low_bit0_set);
                    assert!(!bits.low_bit5_set);
                    assert!(bits.low_bit6_set);
                    assert!(bits.low_bit7_set);
                }
                "fe_e1_elevated" => {
                    assert_eq!(bits.high_byte, 0xFE);
                    assert_eq!(bits.low_byte, 0xE1);
                    assert!(bits.low_bit0_set);
                    assert!(bits.low_bit5_set);
                    assert!(bits.low_bit6_set);
                    assert!(bits.low_bit7_set);
                }
                "fa_c0_baseline_alt" => {
                    assert_eq!(bits.high_byte, 0xFA);
                    assert_eq!(bits.low_byte, 0xC0);
                    assert!(!bits.low_bit0_set);
                    assert!(!bits.low_bit5_set);
                    assert!(bits.low_bit6_set);
                    assert!(bits.low_bit7_set);
                }
                "00_07_inland_water" => {
                    assert_eq!(bits.high_byte, 0x00);
                    assert_eq!(bits.low_byte, 0x07);
                    assert!(bits.low_bit0_set);
                    assert!(!bits.low_bit5_set);
                    assert!(!bits.low_bit6_set);
                    assert!(!bits.low_bit7_set);
                }
                "unknown" => panic!(
                    "unexpected unknown QC profile in regression windows: value={value}"
                ),
                other => panic!("unexpected profile label: {other}"),
            }
        }
    }
}

#[test]
fn viirs_vnp21_qc_raw_state_whitelist_by_oceanpix_is_stable_across_key_windows() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let windows = [
        (1234_u64, 987_usize, 2_usize, 6_usize),
        (1000_u64, 1007_usize, 2_usize, 10_usize),
        (800_u64, 1600_usize, 2_usize, 4_usize),
        (1500_u64, 2500_usize, 2_usize, 4_usize),
    ];

    use std::collections::{BTreeMap, BTreeSet};
    let mut qc_values_by_oceanpix = BTreeMap::<u8, BTreeSet<u16>>::new();

    for (start_row, start_col, num_rows, num_cols) in windows {
        let qc = decode_chunked_u16_row_major_window_in_file(
            path,
            "/VIIRS_Swath_LSTE/Data Fields/QC",
            70_375_762,
            3,
            0,
            start_row,
            num_rows,
            start_col,
            num_cols,
            3_200,
            16,
            Endianness::Little,
            512,
            8_192,
        )
        .expect("VNP21 QC key window should decode through v1 chunk-index traversal");

        let oceanpix = decode_chunked_u8_row_major_window_in_file(
            path,
            "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
            88_214_296,
            3,
            0,
            start_row,
            num_rows,
            start_col,
            num_cols,
            3_200,
            16,
            512,
            8_192,
        )
        .expect("VNP21 oceanpix key window should decode through v1 chunk-index traversal");

        assert_eq!(qc.len(), oceanpix.len());
        for (qc_value, ocean_value) in qc.iter().zip(oceanpix.iter()) {
            qc_values_by_oceanpix
                .entry(*ocean_value)
                .or_default()
                .insert(*qc_value);
        }
    }

    let expected_ocean_0: BTreeSet<u16> = [65216_u16, 64192_u16].into_iter().collect();
    let expected_ocean_1: BTreeSet<u16> = [7_u16].into_iter().collect();
    let expected_ocean_2: BTreeSet<u16> = [65216_u16, 65249_u16].into_iter().collect();

    assert_eq!(qc_values_by_oceanpix.get(&0_u8), Some(&expected_ocean_0));
    assert_eq!(qc_values_by_oceanpix.get(&1_u8), Some(&expected_ocean_1));
    assert_eq!(qc_values_by_oceanpix.get(&2_u8), Some(&expected_ocean_2));

    assert!(
        qc_values_by_oceanpix
            .keys()
            .all(|ocean| *ocean == 0_u8 || *ocean == 1_u8 || *ocean == 2_u8),
        "unexpected oceanpix category found in key-window regression set"
    );
}

#[test]
fn viirs_vnp21_qc_nonoverlapping_window_cluster_profile_contract_is_stable() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    // Reference values extracted with:
    // h5dump -d '/VIIRS_Swath_LSTE/Data Fields/QC' -s '1700,3000' -c '2,4' <fixture>
    // h5dump -d '/VIIRS_Swath_LSTE/Data Fields/oceanpix' -s '1700,3000' -c '2,4' <fixture>
    // h5dump -d '/VIIRS_Swath_LSTE/Data Fields/QC' -s '1900,500' -c '2,4' <fixture>
    // h5dump -d '/VIIRS_Swath_LSTE/Data Fields/oceanpix' -s '1900,500' -c '2,4' <fixture>
    let windows = [
        (1700_u64, 3000_usize, 2_usize, 4_usize),
        (1900_u64, 500_usize, 2_usize, 4_usize),
    ];

    use std::collections::BTreeMap;
    let mut counts = BTreeMap::<(&str, u8), usize>::new();

    for (start_row, start_col, num_rows, num_cols) in windows {
        let qc = decode_chunked_u16_row_major_window_in_file(
            path,
            "/VIIRS_Swath_LSTE/Data Fields/QC",
            70_375_762,
            3,
            0,
            start_row,
            num_rows,
            start_col,
            num_cols,
            3_200,
            16,
            Endianness::Little,
            512,
            8_192,
        )
        .expect("VNP21 QC non-overlapping window cluster should decode through v1 chunk-index traversal");

        let oceanpix = decode_chunked_u8_row_major_window_in_file(
            path,
            "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
            88_214_296,
            3,
            0,
            start_row,
            num_rows,
            start_col,
            num_cols,
            3_200,
            16,
            512,
            8_192,
        )
        .expect("VNP21 oceanpix non-overlapping window cluster should decode through v1 chunk-index traversal");

        assert_eq!(qc.len(), oceanpix.len());
        for (qc_value, ocean_value) in qc.iter().zip(oceanpix.iter()) {
            let profile = classify_vnp21_qc_observed_profile(decode_vnp21_qc_observed_bits(*qc_value));
            *counts.entry((profile, *ocean_value)).or_insert(0) += 1;
        }
    }

    // Inland-water window remains a pure known-state block.
    assert_eq!(counts.get(&("00_07_inland_water", 1_u8)), Some(&8_usize));

    // Land window introduces currently-unclassified observed QC states.
    assert_eq!(counts.get(&("unknown", 0_u8)), Some(&8_usize));

    // No cross-category leakage in this cluster.
    assert!(counts.get(&("unknown", 1_u8)).is_none());
    assert!(counts.get(&("00_07_inland_water", 0_u8)).is_none());

    let total: usize = counts.values().sum();
    assert_eq!(total, 16_usize);
}

#[test]
fn viirs_vnp21_qc_oceanpix_documented_semantics_vocabulary_is_discoverable_and_consistent() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    assert!(
        dataset_metadata_contains_text_in_file(
            path,
            "/VIIRS_Swath_LSTE/Data Fields/QC",
            "Quality Control for LST and emissivity",
        )
        .expect("QC metadata text search should succeed"),
        "QC documented long_name text should be discoverable in QC metadata scope"
    );
    let oceanpix_report = dataset_metadata_text_report_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
        &["land ocean inland_water", "ocean pixels"],
    )
    .expect("oceanpix metadata text search should succeed");
    assert!(
        oceanpix_report.missing_terms.is_empty(),
        "oceanpix documented vocabulary should be discoverable in oceanpix metadata scope; present={:?}, missing={:?}",
        oceanpix_report.present_terms,
        oceanpix_report.missing_terms,
    );

    let windows = [
        (1234_u64, 987_usize, 2_usize, 6_usize),
        (1000_u64, 1007_usize, 2_usize, 10_usize),
        (800_u64, 1600_usize, 2_usize, 4_usize),
        (1500_u64, 2500_usize, 2_usize, 4_usize),
        (1700_u64, 3000_usize, 2_usize, 4_usize),
        (1900_u64, 500_usize, 2_usize, 4_usize),
    ];

    use std::collections::BTreeSet;
    let mut observed_oceanpix_values = BTreeSet::<u8>::new();

    for (start_row, start_col, num_rows, num_cols) in windows {
        let oceanpix = decode_chunked_u8_row_major_window_in_file(
            path,
            "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
            88_214_296,
            3,
            0,
            start_row,
            num_rows,
            start_col,
            num_cols,
            3_200,
            16,
            512,
            8_192,
        )
        .expect("VNP21 oceanpix bounded windows should decode through v1 chunk-index traversal");

        assert!(
            oceanpix.iter().all(|v| *v <= 2_u8),
            "oceanpix values should respect documented valid_range upper bound"
        );
        observed_oceanpix_values.extend(oceanpix);
    }

    assert_eq!(
        observed_oceanpix_values,
        [0_u8, 1_u8, 2_u8].into_iter().collect(),
        "bounded QA regression windows should span the full documented oceanpix value space"
    );
}

#[test]
fn viirs_vnp21_qc_documented_semantics_vocabulary_and_observed_bitfield_families_are_consistent() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let qc_report = dataset_metadata_text_report_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/QC",
        &["Quality Control for LST and emissivity", "QC"],
    )
    .expect("QC metadata text search should succeed");
    assert!(
        qc_report.missing_terms.is_empty(),
        "QC documented vocabulary should be discoverable in QC metadata scope; present={:?}, missing={:?}",
        qc_report.present_terms,
        qc_report.missing_terms,
    );

    let windows = [
        (1234_u64, 987_usize, 2_usize, 6_usize),
        (1000_u64, 1007_usize, 2_usize, 10_usize),
        (800_u64, 1600_usize, 2_usize, 4_usize),
        (1500_u64, 2500_usize, 2_usize, 4_usize),
        (1700_u64, 3000_usize, 2_usize, 4_usize),
        (1900_u64, 500_usize, 2_usize, 4_usize),
    ];

    use std::collections::BTreeSet;
    let mut observed_high_bytes = BTreeSet::<u8>::new();
    let mut observed_low_bytes = BTreeSet::<u8>::new();

    for (start_row, start_col, num_rows, num_cols) in windows {
        let qc = decode_chunked_u16_row_major_window_in_file(
            path,
            "/VIIRS_Swath_LSTE/Data Fields/QC",
            70_375_762,
            3,
            0,
            start_row,
            num_rows,
            start_col,
            num_cols,
            3_200,
            16,
            Endianness::Little,
            512,
            8_192,
        )
        .expect("VNP21 QC bounded windows should decode through v1 chunk-index traversal");

        for value in qc {
            let bits = decode_vnp21_qc_observed_bits(value);
            observed_high_bytes.insert(bits.high_byte);
            observed_low_bytes.insert(bits.low_byte);
        }
    }

    // Stable observed QC bitfield families across bounded windows.
    assert_eq!(
        observed_high_bytes,
        [0x00_u8, 0xFA_u8, 0xFE_u8].into_iter().collect(),
        "QC high-byte families drifted in bounded-window regression set"
    );
    assert_eq!(
        observed_low_bytes,
        [0x07_u8, 0x0F_u8, 0x32_u8, 0xC0_u8, 0xE1_u8]
            .into_iter()
            .collect(),
        "QC low-byte families drifted in bounded-window regression set"
    );
}

#[test]
fn viirs_vnp21_qc_observed_bitfield_interpretation_matches_oceanpix_windows() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let qc_nonorigin = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/QC",
        70_375_762,
        3,
        0,
        1234,
        2,
        987,
        6,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 QC non-origin window should decode through v1 chunk-index traversal");
    let ocean_nonorigin = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
        88_214_296,
        3,
        0,
        1234,
        2,
        987,
        6,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 oceanpix non-origin window should decode through v1 chunk-index traversal");

    let qc_origin = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/QC",
        70_375_762,
        3,
        0,
        800,
        2,
        1600,
        4,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 QC origin window should decode through v1 chunk-index traversal");
    let ocean_origin = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
        88_214_296,
        3,
        0,
        800,
        2,
        1600,
        4,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 oceanpix origin window should decode through v1 chunk-index traversal");

    assert_eq!(qc_nonorigin.len(), ocean_nonorigin.len());
    assert_eq!(qc_origin.len(), ocean_origin.len());

    let interpreted_nonorigin: Vec<Vnp21QcObservedBits> = qc_nonorigin
        .iter()
        .map(|value| decode_vnp21_qc_observed_bits(*value))
        .collect();
    let interpreted_origin: Vec<Vnp21QcObservedBits> = qc_origin
        .iter()
        .map(|value| decode_vnp21_qc_observed_bits(*value))
        .collect();

    // In this fixture, oceanpix=2 pairs with two observed QC low-byte states.
    for (bits, oceanpix) in interpreted_nonorigin.iter().zip(ocean_nonorigin.iter()) {
        assert_eq!(*oceanpix, 2);
        assert_eq!(bits.high_byte, 0xFE);
        // For observed states, bits 6-7 remain set and bits 0/5 toggle together.
        assert!(bits.low_bit6_set);
        assert!(bits.low_bit7_set);
        assert_eq!(bits.low_bit0_set, bits.low_bit5_set);
        assert!(bits.low_byte == 0xC0 || bits.low_byte == 0xE1);
    }

    // Origin baseline keeps oceanpix=0 with both observed low bits unset.
    for (bits, oceanpix) in interpreted_origin.iter().zip(ocean_origin.iter()) {
        assert_eq!(bits.high_byte, 0xFE);
        assert_eq!(*oceanpix, 0);
        assert!(!bits.low_bit0_set);
        assert!(!bits.low_bit5_set);
        assert!(bits.low_bit6_set);
        assert!(bits.low_bit7_set);
    }

    let mut unique_nonorigin = interpreted_nonorigin
        .iter()
        .map(|bits| {
            (
                bits.high_byte,
                bits.low_byte,
                bits.low_bit0_set,
                bits.low_bit5_set,
                bits.low_bit6_set,
                bits.low_bit7_set,
            )
        })
        .collect::<Vec<(u8, u8, bool, bool, bool, bool)>>();
    unique_nonorigin.sort_unstable();
    unique_nonorigin.dedup();
    assert_eq!(
        unique_nonorigin,
        vec![
            (0xFE, 0xC0, false, false, true, true),
            (0xFE, 0xE1, true, true, true, true),
        ]
    );
}

#[test]
fn viirs_vnp21_qc_oceanpix_observed_state_histogram_contract_is_stable() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let qc_nonorigin = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/QC",
        70_375_762,
        3,
        0,
        1234,
        2,
        987,
        6,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 QC non-origin window should decode through v1 chunk-index traversal");
    let ocean_nonorigin = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
        88_214_296,
        3,
        0,
        1234,
        2,
        987,
        6,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 oceanpix non-origin window should decode through v1 chunk-index traversal");

    let qc_origin = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/QC",
        70_375_762,
        3,
        0,
        800,
        2,
        1600,
        4,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 QC origin window should decode through v1 chunk-index traversal");
    let ocean_origin = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
        88_214_296,
        3,
        0,
        800,
        2,
        1600,
        4,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 oceanpix origin window should decode through v1 chunk-index traversal");

    use std::collections::BTreeMap;
    let mut histogram = BTreeMap::<(u16, u8), usize>::new();

    for (qc, ocean) in qc_nonorigin.iter().zip(ocean_nonorigin.iter()) {
        *histogram.entry((*qc, *ocean)).or_insert(0) += 1;
    }
    for (qc, ocean) in qc_origin.iter().zip(ocean_origin.iter()) {
        *histogram.entry((*qc, *ocean)).or_insert(0) += 1;
    }

    // Stable fixture-level pairing contract across both windows.
    assert_eq!(histogram.get(&(65216_u16, 2_u8)), Some(&10_usize));
    assert_eq!(histogram.get(&(65249_u16, 2_u8)), Some(&2_usize));
    assert_eq!(histogram.get(&(65216_u16, 0_u8)), Some(&8_usize));
    assert_eq!(histogram.len(), 3);
}

#[test]
fn viirs_vnp21_qc_oceanpix_row_alignment_contract_is_stable() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let qc_nonorigin = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/QC",
        70_375_762,
        3,
        0,
        1234,
        2,
        987,
        6,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 QC non-origin window should decode through v1 chunk-index traversal");
    let ocean_nonorigin = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
        88_214_296,
        3,
        0,
        1234,
        2,
        987,
        6,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 oceanpix non-origin window should decode through v1 chunk-index traversal");

    let qc_origin = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/QC",
        70_375_762,
        3,
        0,
        800,
        2,
        1600,
        4,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 QC origin window should decode through v1 chunk-index traversal");
    let ocean_origin = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
        88_214_296,
        3,
        0,
        800,
        2,
        1600,
        4,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 oceanpix origin window should decode through v1 chunk-index traversal");

    assert_eq!(qc_nonorigin.len(), 12);
    assert_eq!(ocean_nonorigin.len(), 12);
    assert_eq!(qc_origin.len(), 8);
    assert_eq!(ocean_origin.len(), 8);

    let expected_qc_nonorigin = vec![
        vec![65249_u16, 65216, 65216, 65216, 65216, 65216],
        vec![65249_u16, 65216, 65216, 65216, 65216, 65216],
    ];
    let expected_ocean_nonorigin = vec![vec![2_u8, 2, 2, 2, 2, 2], vec![2_u8, 2, 2, 2, 2, 2]];

    let qc_nonorigin_rows: Vec<Vec<u16>> = qc_nonorigin.chunks(6).map(|row| row.to_vec()).collect();
    let ocean_nonorigin_rows: Vec<Vec<u8>> = ocean_nonorigin
        .chunks(6)
        .map(|row| row.to_vec())
        .collect();

    assert_eq!(qc_nonorigin_rows, expected_qc_nonorigin);
    assert_eq!(ocean_nonorigin_rows, expected_ocean_nonorigin);

    let qc_origin_rows: Vec<Vec<u16>> = qc_origin.chunks(4).map(|row| row.to_vec()).collect();
    let ocean_origin_rows: Vec<Vec<u8>> = ocean_origin.chunks(4).map(|row| row.to_vec()).collect();

    assert_eq!(qc_origin_rows, vec![vec![65216_u16, 65216, 65216, 65216], vec![65216_u16, 65216, 65216, 65216]]);
    assert_eq!(ocean_origin_rows, vec![vec![0_u8, 0, 0, 0], vec![0_u8, 0, 0, 0]]);
}

#[test]
fn viirs_vnp21_qc_oceanpix_additional_nonorigin_window_matches_h5dump_reference() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    // Reference values extracted with:
    // h5dump -d '/VIIRS_Swath_LSTE/Data Fields/QC' -s '1000,1007' -c '2,10' <fixture>
    // h5dump -d '/VIIRS_Swath_LSTE/Data Fields/oceanpix' -s '1000,1007' -c '2,10' <fixture>
    let expected_qc = vec![
        65216_u16, 65216, 65216, 64192, 65216, 65216, 65216, 64192, 64192, 64192, 65216, 65216,
        64192, 65216, 65216, 65216, 65216, 65216, 65216, 65216,
    ];
    let expected_oceanpix = vec![
        0_u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];

    let decoded_qc = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/QC",
        70_375_762,
        3,
        0,
        1000,
        2,
        1007,
        10,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 QC additional window should decode through v1 chunk-index traversal");
    let decoded_oceanpix = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
        88_214_296,
        3,
        0,
        1000,
        2,
        1007,
        10,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 oceanpix additional window should decode through v1 chunk-index traversal");

    assert_eq!(decoded_qc, expected_qc);
    assert_eq!(decoded_oceanpix, expected_oceanpix);

    // Additional observed-state guardrail for this non-origin oceanpix=0 window.
    let mut unique_qc = decoded_qc.clone();
    unique_qc.sort_unstable();
    unique_qc.dedup();
    assert_eq!(unique_qc, vec![64192_u16, 65216_u16]);
    assert!(decoded_oceanpix.iter().all(|v| *v == 0));
}

#[test]
fn viirs_vnp21_qc_observed_bits_in_additional_oceanpix0_window_are_stable() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let qc = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/QC",
        70_375_762,
        3,
        0,
        1000,
        2,
        1007,
        10,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 QC additional window should decode through v1 chunk-index traversal");
    let oceanpix = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
        88_214_296,
        3,
        0,
        1000,
        2,
        1007,
        10,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 oceanpix additional window should decode through v1 chunk-index traversal");

    assert_eq!(qc.len(), oceanpix.len());
    assert!(oceanpix.iter().all(|v| *v == 0));

    let interpreted: Vec<Vnp21QcObservedBits> = qc
        .iter()
        .map(|value| decode_vnp21_qc_observed_bits(*value))
        .collect();

    for bits in &interpreted {
        // In this oceanpix=0 slice, low bits 0/5 remain unset while 6/7 stay set.
        assert!(!bits.low_bit0_set);
        assert!(!bits.low_bit5_set);
        assert!(bits.low_bit6_set);
        assert!(bits.low_bit7_set);
        assert!(bits.high_byte == 0xFA || bits.high_byte == 0xFE);
        assert_eq!(bits.low_byte, 0xC0);
    }

    let mut unique_qc = qc.clone();
    unique_qc.sort_unstable();
    unique_qc.dedup();
    assert_eq!(unique_qc, vec![64192_u16, 65216_u16]);

    let mut unique_interpreted = interpreted
        .iter()
        .map(|bits| {
            (
                bits.high_byte,
                bits.low_byte,
                bits.low_bit0_set,
                bits.low_bit5_set,
                bits.low_bit6_set,
                bits.low_bit7_set,
            )
        })
        .collect::<Vec<(u8, u8, bool, bool, bool, bool)>>();
    unique_interpreted.sort_unstable();
    unique_interpreted.dedup();
    assert_eq!(
        unique_interpreted,
        vec![
            (0xFA, 0xC0, false, false, true, true),
            (0xFE, 0xC0, false, false, true, true),
        ]
    );
}

#[test]
fn viirs_vnp21_qc_observed_profile_classification_is_stable_across_windows() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let qc_window_a = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/QC",
        70_375_762,
        3,
        0,
        1234,
        2,
        987,
        6,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 QC window A should decode through v1 chunk-index traversal");
    let ocean_window_a = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
        88_214_296,
        3,
        0,
        1234,
        2,
        987,
        6,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 oceanpix window A should decode through v1 chunk-index traversal");

    let qc_window_b = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/QC",
        70_375_762,
        3,
        0,
        1000,
        2,
        1007,
        10,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 QC window B should decode through v1 chunk-index traversal");
    let ocean_window_b = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
        88_214_296,
        3,
        0,
        1000,
        2,
        1007,
        10,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 oceanpix window B should decode through v1 chunk-index traversal");

    use std::collections::BTreeMap;
    let mut counts = BTreeMap::<(&str, u8), usize>::new();

    for (qc, ocean) in qc_window_a.iter().zip(ocean_window_a.iter()) {
        let profile = classify_vnp21_qc_observed_profile(decode_vnp21_qc_observed_bits(*qc));
        *counts.entry((profile, *ocean)).or_insert(0) += 1;
    }
    for (qc, ocean) in qc_window_b.iter().zip(ocean_window_b.iter()) {
        let profile = classify_vnp21_qc_observed_profile(decode_vnp21_qc_observed_bits(*qc));
        *counts.entry((profile, *ocean)).or_insert(0) += 1;
    }

    assert_eq!(counts.get(&("fe_e1_elevated", 2_u8)), Some(&2_usize));
    assert_eq!(counts.get(&("fe_c0_baseline", 2_u8)), Some(&10_usize));
    assert_eq!(counts.get(&("fe_c0_baseline", 0_u8)), Some(&15_usize));
    assert_eq!(counts.get(&("fa_c0_baseline_alt", 0_u8)), Some(&5_usize));
    assert!(counts.get(&("unknown", 0_u8)).is_none());
    assert!(counts.get(&("unknown", 2_u8)).is_none());
}

#[test]
fn viirs_vnp21_qc_observed_profile_classification_with_origin_baseline_is_stable() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let qc_window_a = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/QC",
        70_375_762,
        3,
        0,
        1234,
        2,
        987,
        6,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 QC window A should decode through v1 chunk-index traversal");
    let ocean_window_a = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
        88_214_296,
        3,
        0,
        1234,
        2,
        987,
        6,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 oceanpix window A should decode through v1 chunk-index traversal");

    let qc_window_b = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/QC",
        70_375_762,
        3,
        0,
        1000,
        2,
        1007,
        10,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 QC window B should decode through v1 chunk-index traversal");
    let ocean_window_b = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
        88_214_296,
        3,
        0,
        1000,
        2,
        1007,
        10,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 oceanpix window B should decode through v1 chunk-index traversal");

    let qc_origin = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/QC",
        70_375_762,
        3,
        0,
        800,
        2,
        1600,
        4,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 QC origin window should decode through v1 chunk-index traversal");
    let ocean_origin = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/oceanpix",
        88_214_296,
        3,
        0,
        800,
        2,
        1600,
        4,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 oceanpix origin window should decode through v1 chunk-index traversal");

    use std::collections::BTreeMap;
    let mut counts = BTreeMap::<(&str, u8), usize>::new();

    for (qc, ocean) in qc_window_a.iter().zip(ocean_window_a.iter()) {
        let profile = classify_vnp21_qc_observed_profile(decode_vnp21_qc_observed_bits(*qc));
        *counts.entry((profile, *ocean)).or_insert(0) += 1;
    }
    for (qc, ocean) in qc_window_b.iter().zip(ocean_window_b.iter()) {
        let profile = classify_vnp21_qc_observed_profile(decode_vnp21_qc_observed_bits(*qc));
        *counts.entry((profile, *ocean)).or_insert(0) += 1;
    }
    for (qc, ocean) in qc_origin.iter().zip(ocean_origin.iter()) {
        let profile = classify_vnp21_qc_observed_profile(decode_vnp21_qc_observed_bits(*qc));
        *counts.entry((profile, *ocean)).or_insert(0) += 1;
    }

    assert_eq!(counts.get(&("fe_e1_elevated", 2_u8)), Some(&2_usize));
    assert_eq!(counts.get(&("fe_c0_baseline", 2_u8)), Some(&10_usize));
    assert_eq!(counts.get(&("fe_c0_baseline", 0_u8)), Some(&23_usize));
    assert_eq!(counts.get(&("fa_c0_baseline_alt", 0_u8)), Some(&5_usize));
    assert!(counts.get(&("unknown", 0_u8)).is_none());
    assert!(counts.get(&("unknown", 2_u8)).is_none());
}

#[test]
fn viirs_vnp21_emis14_row_major_window_and_semantics_match_h5dump_reference() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let emis14 = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/Emis_14")
        .expect("VNP21 Emis_14 dataset should be discoverable before payload decode attempt");
    assert_eq!(emis14.path, "/VIIRS_Swath_LSTE/Data Fields/Emis_14");

    // Reference values extracted with:
    // h5dump -d '/VIIRS_Swath_LSTE/Data Fields/Emis_14' -s '800,1600' -c '2,4' <fixture>
    let expected_raw = vec![238_u8, 239, 240, 239, 237, 238, 237, 239];

    let decoded = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/Emis_14",
        71_150_869,
        3,
        0,
        800,
        2,
        1600,
        4,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 Emis_14 row-major window should decode through v1 chunk-index traversal");

    assert_eq!(decoded, expected_raw);

    // Semantics from h5dump attributes:
    // _FillValue=0, valid_range=[1,255], scale_factor=0.002, add_offset=0.49.
    assert!(decoded.iter().all(|v| *v >= 1));
    let scaled: Vec<f64> = decoded.iter().map(|v| *v as f64 * 0.002 + 0.49).collect();
    let expected_scaled = vec![
        0.966_f64, 0.968_f64, 0.970_f64, 0.968_f64, 0.964_f64, 0.966_f64, 0.964_f64, 0.968_f64,
    ];
    for (idx, (actual, expected)) in scaled.iter().zip(expected_scaled.iter()).enumerate() {
        let diff = (actual - expected).abs();
        assert!(
            diff <= 1e-12,
            "VNP21 Emis_14 scaled mismatch at index {idx}: actual={actual}, expected={expected}, abs_diff={diff}"
        );
    }
}

#[test]
fn viirs_vnp21_emis15_row_major_window_and_semantics_match_h5dump_reference() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let emis15 = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/Emis_15")
        .expect("VNP21 Emis_15 dataset should be discoverable before payload decode attempt");
    assert_eq!(emis15.path, "/VIIRS_Swath_LSTE/Data Fields/Emis_15");

    // Reference values extracted with:
    // h5dump -d '/VIIRS_Swath_LSTE/Data Fields/Emis_15' -s '800,1600' -c '2,4' <fixture>
    let expected_raw = vec![244_u8, 245, 245, 245, 244, 245, 244, 245];

    let decoded = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/Emis_15",
        73_223_084,
        3,
        0,
        800,
        2,
        1600,
        4,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 Emis_15 row-major window should decode through v1 chunk-index traversal");

    assert_eq!(decoded, expected_raw);

    // Semantics from h5dump attributes:
    // _FillValue=0, valid_range=[1,255], scale_factor=0.002, add_offset=0.49.
    assert!(decoded.iter().all(|v| *v >= 1));
    let scaled: Vec<f64> = decoded.iter().map(|v| *v as f64 * 0.002 + 0.49).collect();
    let expected_scaled = vec![
        0.978_f64, 0.980_f64, 0.980_f64, 0.980_f64, 0.978_f64, 0.980_f64, 0.978_f64, 0.980_f64,
    ];
    for (idx, (actual, expected)) in scaled.iter().zip(expected_scaled.iter()).enumerate() {
        let diff = (actual - expected).abs();
        assert!(
            diff <= 1e-12,
            "VNP21 Emis_15 scaled mismatch at index {idx}: actual={actual}, expected={expected}, abs_diff={diff}"
        );
    }
}

#[test]
fn viirs_vnp21_emis16_row_major_window_and_semantics_match_h5dump_reference() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let emis16 = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/Emis_16")
        .expect("VNP21 Emis_16 dataset should be discoverable before payload decode attempt");
    assert_eq!(emis16.path, "/VIIRS_Swath_LSTE/Data Fields/Emis_16");

    // Reference values extracted with:
    // h5dump -d '/VIIRS_Swath_LSTE/Data Fields/Emis_16' -s '800,1600' -c '2,4' <fixture>
    let expected_raw = vec![245_u8, 245, 245, 245, 245, 245, 245, 245];

    let decoded = decode_chunked_u8_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/Emis_16",
        74_719_447,
        3,
        0,
        800,
        2,
        1600,
        4,
        3_200,
        16,
        512,
        8_192,
    )
    .expect("VNP21 Emis_16 row-major window should decode through v1 chunk-index traversal");

    assert_eq!(decoded, expected_raw);

    // Semantics from h5dump attributes:
    // _FillValue=0, valid_range=[1,255], scale_factor=0.002, add_offset=0.49.
    assert!(decoded.iter().all(|v| *v >= 1));
    let scaled: Vec<f64> = decoded.iter().map(|v| *v as f64 * 0.002 + 0.49).collect();
    let expected_scaled = vec![0.980_f64; expected_raw.len()];
    for (idx, (actual, expected)) in scaled.iter().zip(expected_scaled.iter()).enumerate() {
        let diff = (actual - expected).abs();
        assert!(
            diff <= 1e-12,
            "VNP21 Emis_16 scaled mismatch at index {idx}: actual={actual}, expected={expected}, abs_diff={diff}"
        );
    }
}

#[test]
fn viirs_vnp21_emis14_err_row_major_window_and_semantics_match_h5dump_reference() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let emis14_err = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/Emis_14_err")
        .expect("VNP21 Emis_14_err dataset should be discoverable before payload decode attempt");
    assert_eq!(emis14_err.path, "/VIIRS_Swath_LSTE/Data Fields/Emis_14_err");

    // Reference values extracted with:
    // h5dump -d '/VIIRS_Swath_LSTE/Data Fields/Emis_14_err' -s '800,1600' -c '2,4' <fixture>
    let expected_raw = vec![235_u16, 235, 235, 236, 235, 235, 235, 235];

    let decoded = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/Emis_14_err",
        79_369_210,
        3,
        0,
        800,
        2,
        1600,
        4,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 Emis_14_err row-major window should decode through v1 chunk-index traversal");

    assert_eq!(decoded, expected_raw);

    // Semantics from h5dump attributes:
    // _FillValue=0, valid_range=[1,65535], scale_factor=0.0001, add_offset=0.
    assert!(decoded.iter().all(|v| *v >= 1));
    let scaled: Vec<f64> = decoded.iter().map(|v| *v as f64 * 0.0001).collect();
    let expected_scaled = vec![0.0235_f64, 0.0235_f64, 0.0235_f64, 0.0236_f64, 0.0235_f64, 0.0235_f64, 0.0235_f64, 0.0235_f64];
    for (idx, (actual, expected)) in scaled.iter().zip(expected_scaled.iter()).enumerate() {
        let diff = (actual - expected).abs();
        assert!(
            diff <= 1e-12,
            "VNP21 Emis_14_err scaled mismatch at index {idx}: actual={actual}, expected={expected}, abs_diff={diff}"
        );
    }
}

#[test]
fn viirs_vnp21_emis15_err_row_major_window_and_semantics_match_h5dump_reference() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let emis15_err = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/Emis_15_err")
        .expect("VNP21 Emis_15_err dataset should be discoverable before payload decode attempt");
    assert_eq!(emis15_err.path, "/VIIRS_Swath_LSTE/Data Fields/Emis_15_err");

    // Reference values extracted with:
    // h5dump -d '/VIIRS_Swath_LSTE/Data Fields/Emis_15_err' -s '800,1600' -c '2,4' <fixture>
    let expected_raw = vec![115_u16, 115, 115, 115, 115, 115, 115, 115];

    let decoded = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/Emis_15_err",
        80_003_781,
        3,
        0,
        800,
        2,
        1600,
        4,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 Emis_15_err row-major window should decode through v1 chunk-index traversal");

    assert_eq!(decoded, expected_raw);

    // Semantics from h5dump attributes:
    // _FillValue=0, valid_range=[1,65535], scale_factor=0.0001, add_offset=0.
    assert!(decoded.iter().all(|v| *v >= 1));
    let scaled: Vec<f64> = decoded.iter().map(|v| *v as f64 * 0.0001).collect();
    let expected_scaled = vec![0.0115_f64; expected_raw.len()];
    for (idx, (actual, expected)) in scaled.iter().zip(expected_scaled.iter()).enumerate() {
        let diff = (actual - expected).abs();
        assert!(
            diff <= 1e-12,
            "VNP21 Emis_15_err scaled mismatch at index {idx}: actual={actual}, expected={expected}, abs_diff={diff}"
        );
    }
}

#[test]
fn viirs_vnp21_emis16_err_row_major_window_and_semantics_match_h5dump_reference() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let emis16_err = resolve_dataset_in_file(path, "/VIIRS_Swath_LSTE/Data Fields/Emis_16_err")
        .expect("VNP21 Emis_16_err dataset should be discoverable before payload decode attempt");
    assert_eq!(emis16_err.path, "/VIIRS_Swath_LSTE/Data Fields/Emis_16_err");

    // Reference values extracted with:
    // h5dump -d '/VIIRS_Swath_LSTE/Data Fields/Emis_16_err' -s '800,1600' -c '2,4' <fixture>
    let expected_raw = vec![111_u16, 111, 111, 111, 111, 111, 111, 111];

    let decoded = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/Emis_16_err",
        80_440_648,
        3,
        0,
        800,
        2,
        1600,
        4,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 Emis_16_err row-major window should decode through v1 chunk-index traversal");

    assert_eq!(decoded, expected_raw);

    // Semantics from h5dump attributes:
    // _FillValue=0, valid_range=[1,65535], scale_factor=0.0001, add_offset=0.
    assert!(decoded.iter().all(|v| *v >= 1));
    let scaled: Vec<f64> = decoded.iter().map(|v| *v as f64 * 0.0001).collect();
    let expected_scaled = vec![0.0111_f64; expected_raw.len()];
    for (idx, (actual, expected)) in scaled.iter().zip(expected_scaled.iter()).enumerate() {
        let diff = (actual - expected).abs();
        assert!(
            diff <= 1e-12,
            "VNP21 Emis_16_err scaled mismatch at index {idx}: actual={actual}, expected={expected}, abs_diff={diff}"
        );
    }
}

#[test]
fn viirs_vnp21_emis14_err_nonzero_column_window_matches_h5dump_reference() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let expected_raw = vec![227_u16, 227, 227, 227, 227, 226, 227, 227, 227, 227, 227, 227];

    let decoded = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/Emis_14_err",
        79_369_210,
        3,
        0,
        1234,
        2,
        987,
        6,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 Emis_14_err nonzero-column window should decode through v1 chunk-index traversal");

    assert_eq!(decoded, expected_raw);

    // Semantics from h5dump attributes:
    // _FillValue=0, valid_range=[1,65535], scale_factor=0.0001, add_offset=0.
    assert!(decoded.iter().all(|v| *v >= 1));
    let scaled: Vec<f64> = decoded.iter().map(|v| *v as f64 * 0.0001).collect();
    let expected_scaled = vec![
        0.0227_f64, 0.0227_f64, 0.0227_f64, 0.0227_f64, 0.0227_f64, 0.0226_f64,
        0.0227_f64, 0.0227_f64, 0.0227_f64, 0.0227_f64, 0.0227_f64, 0.0227_f64,
    ];
    for (idx, (actual, expected)) in scaled.iter().zip(expected_scaled.iter()).enumerate() {
        let diff = (actual - expected).abs();
        assert!(
            diff <= 1e-12,
            "VNP21 Emis_14_err nonzero-window scaled mismatch at index {idx}: actual={actual}, expected={expected}, abs_diff={diff}"
        );
    }
}

#[test]
fn viirs_vnp21_emis14_err_fill_window_matches_h5dump_reference() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let expected_raw = vec![0_u16, 0, 0, 0, 0, 0, 0, 0];

    let decoded = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/Emis_14_err",
        79_369_210,
        3,
        0,
        1500,
        2,
        2500,
        4,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 Emis_14_err fill-window should decode through v1 chunk-index traversal");

    assert_eq!(decoded, expected_raw);

    // Fill semantics from h5dump attributes: _FillValue=0.
    assert!(decoded.iter().all(|v| *v == 0));
    let scaled: Vec<f64> = decoded.iter().map(|v| *v as f64 * 0.0001).collect();
    assert!(scaled.iter().all(|v| *v == 0.0));
}

#[test]
fn viirs_vnp21_emis15_err_nonzero_column_window_matches_h5dump_reference() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let expected_raw = vec![114_u16, 114, 114, 114, 114, 114, 114, 114, 114, 114, 114, 114];

    let decoded = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/Emis_15_err",
        80_003_781,
        3,
        0,
        1234,
        2,
        987,
        6,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 Emis_15_err nonzero-column window should decode through v1 chunk-index traversal");

    assert_eq!(decoded, expected_raw);

    // Semantics from h5dump attributes:
    // _FillValue=0, valid_range=[1,65535], scale_factor=0.0001, add_offset=0.
    assert!(decoded.iter().all(|v| *v >= 1));
    let scaled: Vec<f64> = decoded.iter().map(|v| *v as f64 * 0.0001).collect();
    assert!(scaled.iter().all(|v| (*v - 0.0114).abs() <= 1e-12));
}

#[test]
fn viirs_vnp21_emis15_err_fill_window_matches_h5dump_reference() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let expected_raw = vec![0_u16, 0, 0, 0, 0, 0, 0, 0];

    let decoded = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/Emis_15_err",
        80_003_781,
        3,
        0,
        1500,
        2,
        2500,
        4,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 Emis_15_err fill-window should decode through v1 chunk-index traversal");

    assert_eq!(decoded, expected_raw);

    // Fill semantics from h5dump attributes: _FillValue=0.
    assert!(decoded.iter().all(|v| *v == 0));
    let scaled: Vec<f64> = decoded.iter().map(|v| *v as f64 * 0.0001).collect();
    assert!(scaled.iter().all(|v| *v == 0.0));
}

#[test]
fn viirs_vnp21_emis16_err_nonzero_column_window_matches_h5dump_reference() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let expected_raw = vec![111_u16, 111, 111, 111, 111, 111, 111, 111, 111, 111, 111, 111];

    let decoded = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/Emis_16_err",
        80_440_648,
        3,
        0,
        1234,
        2,
        987,
        6,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 Emis_16_err nonzero-column window should decode through v1 chunk-index traversal");

    assert_eq!(decoded, expected_raw);

    // Semantics from h5dump attributes:
    // _FillValue=0, valid_range=[1,65535], scale_factor=0.0001, add_offset=0.
    assert!(decoded.iter().all(|v| *v >= 1));
    let scaled: Vec<f64> = decoded.iter().map(|v| *v as f64 * 0.0001).collect();
    assert!(scaled.iter().all(|v| (*v - 0.0111).abs() <= 1e-12));
}

#[test]
fn viirs_vnp21_emis16_err_fill_window_matches_h5dump_reference() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let expected_raw = vec![0_u16, 0, 0, 0, 0, 0, 0, 0];

    let decoded = decode_chunked_u16_row_major_window_in_file(
        path,
        "/VIIRS_Swath_LSTE/Data Fields/Emis_16_err",
        80_440_648,
        3,
        0,
        1500,
        2,
        2500,
        4,
        3_200,
        16,
        Endianness::Little,
        512,
        8_192,
    )
    .expect("VNP21 Emis_16_err fill-window should decode through v1 chunk-index traversal");

    assert_eq!(decoded, expected_raw);

    // Fill semantics from h5dump attributes: _FillValue=0.
    assert!(decoded.iter().all(|v| *v == 0));
    let scaled: Vec<f64> = decoded.iter().map(|v| *v as f64 * 0.0001).collect();
    assert!(scaled.iter().all(|v| *v == 0.0));
}

#[test]
fn viirs_vnp21_emis_err_cross_field_semantics_contract_is_consistent() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let cases = [
        U16SemanticsCase {
            dataset_path: "/VIIRS_Swath_LSTE/Data Fields/Emis_14_err",
            index_address: 79_369_210,
            valid_min: 1,
            expected_nonfill_raw: 235,
            expected_nonfill_scaled: 0.0235,
            scale_factor: 0.0001,
            add_offset: 0.0,
            expected_fill_scaled: 0.0,
        },
        U16SemanticsCase {
            dataset_path: "/VIIRS_Swath_LSTE/Data Fields/Emis_15_err",
            index_address: 80_003_781,
            valid_min: 1,
            expected_nonfill_raw: 115,
            expected_nonfill_scaled: 0.0115,
            scale_factor: 0.0001,
            add_offset: 0.0,
            expected_fill_scaled: 0.0,
        },
        U16SemanticsCase {
            dataset_path: "/VIIRS_Swath_LSTE/Data Fields/Emis_16_err",
            index_address: 80_440_648,
            valid_min: 1,
            expected_nonfill_raw: 111,
            expected_nonfill_scaled: 0.0111,
            scale_factor: 0.0001,
            add_offset: 0.0,
            expected_fill_scaled: 0.0,
        },
    ];

    run_u16_semantics_cases(path, &cases);
}

#[test]
fn viirs_vnp21_emis_cross_field_semantics_contract_is_consistent() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let cases = [
        U8SemanticsCase {
            dataset_path: "/VIIRS_Swath_LSTE/Data Fields/Emis_14",
            index_address: 71_150_869,
            valid_min: 1,
            expected_nonfill_raw: 238,
            expected_nonfill_scaled: 0.966,
            scale_factor: 0.002,
            add_offset: 0.49,
            expected_fill_scaled: 0.49,
        },
        U8SemanticsCase {
            dataset_path: "/VIIRS_Swath_LSTE/Data Fields/Emis_15",
            index_address: 73_223_084,
            valid_min: 1,
            expected_nonfill_raw: 244,
            expected_nonfill_scaled: 0.978,
            scale_factor: 0.002,
            add_offset: 0.49,
            expected_fill_scaled: 0.49,
        },
        U8SemanticsCase {
            dataset_path: "/VIIRS_Swath_LSTE/Data Fields/Emis_16",
            index_address: 74_719_447,
            valid_min: 1,
            expected_nonfill_raw: 245,
            expected_nonfill_scaled: 0.980,
            scale_factor: 0.002,
            add_offset: 0.49,
            expected_fill_scaled: 0.49,
        },
    ];

    run_u8_semantics_cases(path, &cases);
}

#[test]
fn viirs_vnp21_thermal_cross_field_semantics_contract_is_consistent() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/hdf5_examples/VNP21_NRT.A2026151.0724.002.2026151100853.nc",
    );
    if !path.is_file() {
        return;
    }

    let u16_cases = [U16SemanticsCase {
        dataset_path: "/VIIRS_Swath_LSTE/Data Fields/LST",
        index_address: 65_387_786,
        valid_min: 7_500,
        expected_nonfill_raw: 14_007,
        expected_nonfill_scaled: 280.14,
        scale_factor: 0.02,
        add_offset: 0.0,
        expected_fill_scaled: 0.0,
    }];
    run_u16_semantics_cases(path, &u16_cases);

    let u8_cases = [U8SemanticsCase {
        dataset_path: "/VIIRS_Swath_LSTE/Data Fields/LST_err",
        index_address: 78_971_646,
        valid_min: 1,
        expected_nonfill_raw: 22,
        expected_nonfill_scaled: 0.88,
        scale_factor: 0.04,
        add_offset: 0.0,
        expected_fill_scaled: 0.0,
    }];
    run_u8_semantics_cases(path, &u8_cases);
}

#[test]
fn myd09_hdf4_eos_metadata_probe_and_payload_window_are_exercised() {
    let Some(path) = hdf4_example_fixture_in_data_dir("MYD09A1.A2008057.h01v08.061.2021087165611.hdf") else {
        return;
    };

    let summary = probe_hdf4_eos_metadata_in_file(&path)
        .expect("MYD09 HDF4 EOS metadata probe should succeed");
    assert!(summary.grid_names.iter().any(|name| name == "MOD_Grid_500m_Surface_Reflectance"));
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "sur_refl_b01")
    );

    let field = find_modis_field(&summary, "MOD_Grid_500m_Surface_Reflectance", "sur_refl_b01")
        .expect("MYD09 field metadata should include sur_refl_b01");
    assert_eq!(field.data_type.as_deref(), Some("DFNT_INT16"));

    let probe = probe_hdf4_sds_i16_payload_window_in_file(
        &path,
        "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01",
        8,
    )
    .expect("MYD09 payload probe should succeed before decode attempt");
    let decode_attempt = attempt_decode_hdf4_sds_i16_window_in_file(
        &path,
        "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01",
        8,
    );

    if probe.status == "decoded_preview" {
        let values = decode_attempt.expect("MYD09 decode attempt should return the probe preview");
        assert!(!values.is_empty());
        assert!(values.len() <= 8);
        assert!(values == probe.little_endian_preview || values == probe.big_endian_preview);
    } else {
        let err = decode_attempt.expect_err("MYD09 decode attempt should report diagnostics when the probe cannot decode");
        let msg = format!("{err}");
        assert!(msg.contains("not yet implemented"));
        assert!(msg.contains("/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01"));
        assert!(msg.contains("DFNT_INT16"));
    }
}

#[test]
fn myd11_hdf4_eos_metadata_probe_and_payload_window_are_exercised() {
    let Some(path) = hdf4_example_fixture_in_data_dir("MYD11A2.A2026073.h04v11.061.2026083154149.hdf") else {
        return;
    };

    let summary = probe_hdf4_eos_metadata_in_file(&path)
        .expect("MYD11 HDF4 EOS metadata probe should succeed");
    assert!(summary.grid_names.iter().any(|name| name == "MODIS_Grid_8Day_1km_LST"));
    assert!(summary.data_field_names.iter().any(|name| name == "LST_Day_1km"));

    let field = find_modis_field(&summary, "MODIS_Grid_8Day_1km_LST", "LST_Day_1km")
        .expect("MYD11 field metadata should include LST_Day_1km");
    assert_eq!(field.data_type.as_deref(), Some("DFNT_UINT16"));

    let probe = probe_hdf4_sds_i16_payload_window_in_file(
        &path,
        "/MODIS_Grid_8Day_1km_LST/LST_Day_1km",
        8,
    )
    .expect("MYD11 payload probe should succeed before decode attempt");
    let decode_attempt = attempt_decode_hdf4_sds_i16_window_in_file(
        &path,
        "/MODIS_Grid_8Day_1km_LST/LST_Day_1km",
        8,
    );

    if probe.status == "decoded_preview" {
        let values = decode_attempt.expect("MYD11 decode attempt should return the probe preview");
        assert!(!values.is_empty());
        assert!(values.len() <= 8);
        assert!(values == probe.little_endian_preview || values == probe.big_endian_preview);
    } else {
        let _ = decode_attempt.expect_err("MYD11 decode attempt should either preview values or return diagnostics");
    }
}

#[test]
fn myd13_hdf4_eos_metadata_probe_and_payload_window_are_exercised() {
    let Some(path) = hdf4_example_fixture_in_data_dir("MYD13A1.A2017281.h01v10.061.2021286205049.hdf") else {
        return;
    };

    let summary = probe_hdf4_eos_metadata_in_file(&path)
        .expect("MYD13 HDF4 EOS metadata probe should succeed");
    assert!(summary.grid_names.iter().any(|name| name == "MODIS_Grid_16DAY_500m_VI"));
    assert!(summary.data_field_names.iter().any(|name| name == "500m 16 days NDVI"));

    let field = find_modis_field(&summary, "MODIS_Grid_16DAY_500m_VI", "500m 16 days NDVI")
        .expect("MYD13 field metadata should include NDVI");
    assert_eq!(field.data_type.as_deref(), Some("DFNT_INT16"));

    let probe = probe_hdf4_sds_i16_payload_window_in_file(
        &path,
        "/MODIS_Grid_16DAY_500m_VI/500m 16 days NDVI",
        8,
    )
    .expect("MYD13 payload probe should succeed before decode attempt");
    let decode_attempt = attempt_decode_hdf4_sds_i16_window_in_file(
        &path,
        "/MODIS_Grid_16DAY_500m_VI/500m 16 days NDVI",
        8,
    );

    if probe.status == "decoded_preview" {
        let values = decode_attempt.expect("MYD13 decode attempt should return the probe preview");
        assert!(!values.is_empty());
        assert!(values.len() <= 8);
        assert!(values == probe.little_endian_preview || values == probe.big_endian_preview);
    } else {
        let _ = decode_attempt.expect_err("MYD13 decode attempt should either preview values or return diagnostics");
    }
}

#[test]
fn myd09_documented_field_vocabulary_is_discoverable_with_reports() {
    let Some(path) = hdf4_example_fixture_in_data_dir("MYD09A1.A2008057.h01v08.061.2021087165611.hdf") else {
        return;
    };

    let report = dataset_metadata_text_report_in_file(
        &path,
        "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01",
        &[
            "MOD_Grid_500m_Surface_Reflectance",
            "sur_refl_b01",
            "sur_refl_state_500m",
        ],
    )
    .expect("MYD09 metadata report should succeed");
    assert!(
        report.missing_terms.is_empty(),
        "MYD09 documented vocabulary should be discoverable; present={:?}, missing={:?}",
        report.present_terms,
        report.missing_terms,
    );
}

#[test]
fn myd11_documented_field_vocabulary_is_discoverable_with_reports() {
    let Some(path) = hdf4_example_fixture_in_data_dir("MYD11A2.A2026073.h04v11.061.2026083154149.hdf") else {
        return;
    };

    let report = dataset_metadata_text_report_in_file(
        &path,
        "/MODIS_Grid_8Day_1km_LST/LST_Day_1km",
        &["MODIS_Grid_8Day_1km_LST", "LST_Day_1km", "QC_Day"],
    )
    .expect("MYD11 metadata report should succeed");
    assert!(
        report.missing_terms.is_empty(),
        "MYD11 documented vocabulary should be discoverable; present={:?}, missing={:?}",
        report.present_terms,
        report.missing_terms,
    );
}

#[test]
fn myd13_documented_field_vocabulary_is_discoverable_with_reports() {
    let Some(path) = hdf4_example_fixture_in_data_dir("MYD13A1.A2017281.h01v10.061.2021286205049.hdf") else {
        return;
    };

    let report = dataset_metadata_text_report_in_file(
        &path,
        "/MODIS_Grid_16DAY_500m_VI/500m 16 days NDVI",
        &[
            "MODIS_Grid_16DAY_500m_VI",
            "500m 16 days NDVI",
            "500m 16 days VI Quality",
        ],
    )
    .expect("MYD13 metadata report should succeed");
    assert!(
        report.missing_terms.is_empty(),
        "MYD13 documented vocabulary should be discoverable; present={:?}, missing={:?}",
        report.present_terms,
        report.missing_terms,
    );
}

#[test]
fn viirs_m3_hdf5_fixture_discovers_science_paths() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/viirs/SVM03_j01_d20190505_t0010299_e0011544_b07557_c20210831195745130843_ADu_ops.h5",
    );
    if !path.is_file() {
        return;
    }

    let metadata = probe_file_metadata(path).expect("VIIRS M3 metadata probe should succeed");
    assert!(metadata.superblock_version <= 3);
    assert!(metadata.top_level_groups.iter().any(|group| group == "VIIRS-M3-SDR"));

    let radiance = resolve_dataset_in_file(path, "/All_Data/VIIRS-M3-SDR_All/Radiance")
        .expect("VIIRS M3 radiance dataset should be discoverable by path markers");
    assert_eq!(radiance.path, "/All_Data/VIIRS-M3-SDR_All/Radiance");

    let reflectance = resolve_dataset_in_file(path, "/All_Data/VIIRS-M3-SDR_All/Reflectance")
        .expect("VIIRS M3 reflectance dataset should be discoverable by path markers");
    assert_eq!(reflectance.path, "/All_Data/VIIRS-M3-SDR_All/Reflectance");

    let g_ring_latitude = resolve_dataset_in_file(path, "/All_Data/VIIRS-M3-SDR_All/G-Ring_Latitude")
        .expect("VIIRS M3 G-Ring_Latitude dataset should be discoverable by path markers");
    assert_eq!(g_ring_latitude.path, "/All_Data/VIIRS-M3-SDR_All/G-Ring_Latitude");

    let g_ring_longitude = resolve_dataset_in_file(path, "/All_Data/VIIRS-M3-SDR_All/G-Ring_Longitude")
        .expect("VIIRS M3 G-Ring_Longitude dataset should be discoverable by path markers");
    assert_eq!(g_ring_longitude.path, "/All_Data/VIIRS-M3-SDR_All/G-Ring_Longitude");
}

#[test]
fn viirs_i4_hdf5_fixture_discovers_science_paths() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/viirs/VI4BO_j02_d20260404_t0003522_e0005324_b17602_c20260404002835359000_oebc_ops.h5",
    );
    if !path.is_file() {
        return;
    }

    let metadata = probe_file_metadata(path).expect("VIIRS I4 metadata probe should succeed");
    assert!(metadata.superblock_version <= 3);
    assert!(metadata.top_level_groups.iter().any(|group| group == "VIIRS-I4-IMG-EDR"));

    let brightness = resolve_dataset_in_file(path, "/All_Data/VIIRS-I4-IMG-EDR_All/BrightnessTemperature")
        .expect("VIIRS I4 brightness-temperature dataset should be discoverable by path markers");
    assert_eq!(brightness.path, "/All_Data/VIIRS-I4-IMG-EDR_All/BrightnessTemperature");

    let radiance = resolve_dataset_in_file(path, "/All_Data/VIIRS-I4-IMG-EDR_All/Radiance")
        .expect("VIIRS I4 radiance dataset should be discoverable by path markers");
    assert_eq!(radiance.path, "/All_Data/VIIRS-I4-IMG-EDR_All/Radiance");

    let g_ring_latitude = resolve_dataset_in_file(path, "/All_Data/VIIRS-I4-IMG-EDR_All/G-Ring_Latitude")
        .expect("VIIRS I4 G-Ring_Latitude dataset should be discoverable by path markers");
    assert_eq!(g_ring_latitude.path, "/All_Data/VIIRS-I4-IMG-EDR_All/G-Ring_Latitude");

    let g_ring_longitude = resolve_dataset_in_file(path, "/All_Data/VIIRS-I4-IMG-EDR_All/G-Ring_Longitude")
        .expect("VIIRS I4 G-Ring_Longitude dataset should be discoverable by path markers");
    assert_eq!(g_ring_longitude.path, "/All_Data/VIIRS-I4-IMG-EDR_All/G-Ring_Longitude");
}

#[test]
fn viirs_m3_documented_field_vocabulary_is_discoverable_with_reports() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/viirs/SVM03_j01_d20190505_t0010299_e0011544_b07557_c20210831195745130843_ADu_ops.h5",
    );
    if !path.is_file() {
        return;
    }

    let report = dataset_metadata_text_report_in_file(
        path,
        "/All_Data/VIIRS-M3-SDR_All/Radiance",
        &[
            "VIIRS-M3-SDR",
            "Radiance",
            "Reflectance",
            "G-Ring_Latitude",
            "G-Ring_Longitude",
        ],
    )
    .expect("VIIRS M3 metadata report should succeed");
    assert!(
        report.missing_terms.is_empty(),
        "VIIRS M3 documented vocabulary should be discoverable; present={:?}, missing={:?}",
        report.present_terms,
        report.missing_terms,
    );
}

#[test]
fn viirs_i4_documented_field_vocabulary_is_discoverable_with_reports() {
    let path = std::path::Path::new(
        "/Users/johnlindsay/Documents/data/viirs/VI4BO_j02_d20260404_t0003522_e0005324_b17602_c20260404002835359000_oebc_ops.h5",
    );
    if !path.is_file() {
        return;
    }

    let report = dataset_metadata_text_report_in_file(
        path,
        "/All_Data/VIIRS-I4-IMG-EDR_All/BrightnessTemperature",
        &[
            "VIIRS-I4-IMG-EDR",
            "BrightnessTemperature",
            "Radiance",
            "G-Ring_Latitude",
            "G-Ring_Longitude",
        ],
    )
    .expect("VIIRS I4 metadata report should succeed");
    assert!(
        report.missing_terms.is_empty(),
        "VIIRS I4 documented vocabulary should be discoverable; present={:?}, missing={:?}",
        report.present_terms,
        report.missing_terms,
    );
}

#[test]
fn modis_mod09a1_fixture_has_hdf4_signature() {
    let Some(path) = modis_fixture_named("MOD09A1.A2022169.h04v09.061.2022178214640.hdf") else {
        return;
    };

    let bytes = std::fs::read(&path).expect("MODIS fixture should be readable");
    assert!(bytes.len() >= 4, "MODIS fixture should contain at least 4 bytes");
    assert_eq!(
        &bytes[0..4],
        &[0x0E, 0x03, 0x13, 0x01],
        "MODIS fixture should have HDF4 magic signature"
    );
}

#[test]
fn modis_mod11_mod13_fixture_variants_are_present_and_hdf4() {
    let Some(root) = external_modis_fixture_dir() else {
        return;
    };

    let variants = [
        "MOD11A2.A2024041.h01v10.061.2024051155231.hdf",
        "MOD13A1.A2021065.h03v07.061.2021082133352.hdf",
        "MYD11A2.A2026073.h04v11.061.2026083154149.hdf",
        "MYD13A1.A2017281.h01v10.061.2021286205049.hdf",
    ];

    for file_name in variants {
        let path = root.join(file_name);
        assert!(path.is_file(), "expected MODIS fixture to exist: {}", file_name);
        let bytes = std::fs::read(&path).expect("MODIS variant should be readable");
        assert!(bytes.len() >= 4, "MODIS variant should contain at least 4 bytes");
        assert_eq!(
            &bytes[0..4],
            &[0x0E, 0x03, 0x13, 0x01],
            "MODIS variant should have HDF4 magic signature: {}",
            file_name
        );
    }
}

#[test]
fn modis_mod09_hdf4_eos_metadata_probe_enumerates_expected_fields() {
    let Some(path) = modis_fixture_named("MOD09A1.A2022169.h04v09.061.2022178214640.hdf") else {
        return;
    };

    let summary = probe_hdf4_eos_metadata_in_file(&path)
        .expect("MOD09 HDF4 EOS metadata probe should succeed");
    assert!(summary.struct_metadata_markers >= 1);
    assert!(
        summary
            .grid_names
            .iter()
            .any(|name| name == "MOD_Grid_500m_Surface_Reflectance")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "sur_refl_b01")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "sur_refl_state_500m")
    );

    let field = find_modis_field(&summary, "MOD_Grid_500m_Surface_Reflectance", "sur_refl_b01")
        .expect("MOD09 field metadata should include sur_refl_b01");
    assert_eq!(field.data_type.as_deref(), Some("DFNT_INT16"));
    assert_eq!(field.dim_list, vec!["YDim", "XDim"]);

    let grid = find_modis_grid(&summary, "MOD_Grid_500m_Surface_Reflectance")
        .expect("MOD09 grid metadata should be present");
    assert_eq!(grid.projection.as_deref(), Some("GCTP_SNSOID"));
    assert_eq!(grid.sphere_code, Some(-1));
    assert_eq!(grid.upper_left_mtrs, Some((-15567307.275333, 0.0)));
    assert_eq!(grid.lower_right_mtrs, Some((-14455356.755667, -1111950.519667)));
    assert_eq!(grid.proj_params.len(), 13);
    assert_eq!(grid.proj_params[0], 6_371_007.181);

    let resolved = resolve_hdf4_grid_field(&summary, "MOD_Grid_500m_Surface_Reflectance", "sur_refl_b01")
        .expect("MOD09 field should resolve with shape");
    assert_eq!(resolved.shape, vec![2400, 2400]);
    assert_eq!(resolved.projection.as_deref(), Some("GCTP_SNSOID"));
    assert_eq!(resolved.upper_left_mtrs, Some((-15567307.275333, 0.0)));
    assert_eq!(resolved.lower_right_mtrs, Some((-14455356.755667, -1111950.519667)));

    let geometry = derive_hdf4_grid_geometry(&resolved)
        .expect("MOD09 resolved field should derive grid geometry");
    assert_eq!(geometry.rows, 2400);
    assert_eq!(geometry.cols, 2400);
    assert!((geometry.pixel_size_x - 463.3127165275).abs() < 1e-6);
    assert!((geometry.pixel_size_y + 463.31271652791664).abs() < 1e-6);

    let resolved_path = resolve_hdf4_dataset_path(
        &summary,
        "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01",
    )
    .expect("MOD09 canonical path should resolve");
    assert_eq!(resolved_path.shape, vec![2400, 2400]);

    let paths = enumerate_hdf4_dataset_paths(&summary);
    assert!(paths.iter().any(|p| p == "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01"));
    assert!(paths.iter().any(|p| p == "/MOD_Grid_500m_Surface_Reflectance/sur_refl_state_500m"));
}

#[test]
fn modis_mod09_hdf4_sds_decode_attempt_returns_window_or_diagnostics() {
    let Some(path) = modis_fixture_named("MOD09A1.A2022169.h04v09.061.2022178214640.hdf") else {
        return;
    };

    const MOD09_WINDOW_VALUES: usize = 64;

    let probe = probe_hdf4_sds_i16_payload_window_in_file(
        &path,
        "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01",
        MOD09_WINDOW_VALUES,
    )
    .expect("MOD09 payload probe should succeed before decode attempt");

    let decode_result = decode_hdf4_sds_i16_in_file(
        &path,
        "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01",
    );

    if probe.status == "decoded_preview" {
        let values = decode_result.expect("MOD09 decode attempt should return the probe preview");
        assert!(!values.is_empty(), "decoded window values should not be empty");
        assert!(
            values.len() <= MOD09_WINDOW_VALUES,
            "decoded window should respect max_values"
        );
        assert!(
            values == probe.little_endian_preview || values == probe.big_endian_preview,
            "decoded window should match one of the probe previews"
        );
    } else {
        let err = decode_result.expect_err("MOD09 decode attempt should report diagnostics when the probe cannot decode");
        let msg = format!("{err}");
        assert!(msg.contains("not yet implemented"));
        assert!(msg.contains("/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01"));
        assert!(msg.contains("DFNT_INT16"));
        assert!(msg.contains("shape=[2400, 2400]"));
        assert!(msg.contains(&format!("status={}", probe.status)));
    }
}

#[test]
fn modis_mod09_hdf4_sds_readiness_reports_only_backend_blocker() {
    let Some(path) = modis_fixture_named("MOD09A1.A2022169.h04v09.061.2022178214640.hdf") else {
        return;
    };
    let summary = probe_hdf4_eos_metadata_in_file(&path)
        .expect("MOD09 HDF4 EOS metadata probe should succeed");

    let readiness = assess_hdf4_sds_i16_decode_readiness(
        &summary,
        "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01",
    )
    .expect("readiness should evaluate for MOD09 sur_refl_b01");

    assert_eq!(readiness.resolved_field.shape, vec![2400, 2400]);
    let geometry = readiness
        .geometry
        .expect("MOD09 readiness should include derived geometry");
    assert_eq!(geometry.geotransform.len(), 6);
    assert_eq!(readiness.blockers.len(), 1);
    assert!(readiness.blockers[0].contains("not yet implemented"));
}

#[test]
fn modis_mod09_hdf4_descriptor_enumeration_and_candidate_discovery() {
    let Some(path) = modis_fixture_named("MOD09A1.A2022169.h04v09.061.2022178214640.hdf") else {
        return;
    };

    let descriptors = parse_hdf4_data_descriptors_in_file(&path)
        .expect("MOD09 descriptor enumeration should succeed");
    assert!(!descriptors.is_empty(), "MOD09 descriptors should not be empty");

    let candidates = find_hdf4_sds_i16_payload_candidates_in_file(
        &path,
        "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01",
    )
    .expect("MOD09 candidate discovery should succeed");

    let readiness = assess_hdf4_sds_i16_decode_readiness_in_file(
        &path,
        "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01",
    )
    .expect("MOD09 readiness-in-file should succeed");
    assert_eq!(readiness.payload_candidates, candidates);
    assert!(
        readiness
            .blockers
            .iter()
            .any(|b| {
                b.contains("descriptor-to-field mapping")
                    || b.contains("no in-bounds HDF4 descriptor candidates")
            }),
        "readiness blockers should include descriptor-mapping or no-candidate blocker"
    );

    let mapping = map_hdf4_sds_i16_descriptor_heuristic_in_file(
        &path,
        "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01",
    )
    .expect("MOD09 heuristic descriptor mapping should succeed");
    assert!(
        mapping.selected.is_some() || mapping.rationale.contains("no in-bounds"),
        "MOD09 heuristic mapping should either select a candidate or report no-candidate rationale"
    );

    if mapping.selected.is_some() {
        assert!(
            readiness
                .blockers
                .iter()
                .any(|b| b.contains("heuristic descriptor mapping: confidence=")),
            "readiness blockers should include heuristic mapping confidence summary when a candidate is selected"
        );
    }

    let ranked = rank_hdf4_sds_i16_payload_candidates_in_file(
        &path,
        "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01",
        5,
    )
    .expect("MOD09 ranked candidate inspection should succeed");
    assert!(
        !ranked.is_empty(),
        "MOD09 ranked candidate inspection should return nearest descriptors"
    );
    assert!(
        ranked
            .windows(2)
            .all(|w| w[0].length_delta <= w[1].length_delta),
        "MOD09 ranked candidates should be sorted by length delta"
    );

    let probe = probe_hdf4_sds_i16_payload_window_in_file(
        &path,
        "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01",
        8,
    )
    .expect("MOD09 payload probe should succeed");
    assert!(
        ["decoded_preview", "compressed_payload", "textual_payload", "no_candidate", "candidate_out_of_bounds", "insufficient_bytes"]
            .iter()
            .any(|status| *status == probe.status),
        "MOD09 payload probe should report a known status"
    );

    let decode_attempt = attempt_decode_hdf4_sds_i16_window_in_file(
        &path,
        "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01",
        8,
    );
    match decode_attempt {
        Ok(values) => {
            assert!(!values.is_empty(), "decoded window values should not be empty");
            assert!(values.len() <= 8, "decoded window should respect max_values");
            assert!(
                values == probe.little_endian_preview || values == probe.big_endian_preview,
                "decoded MOD09 window should match one of the probe previews"
            );
        }
        Err(err) => {
            let msg = format!("{err}");
            assert!(
                msg.contains("status="),
                "failed decode attempts should report probe status in diagnostics"
            );
        }
    }
}

#[test]
fn modis_mod09_hdf4_offset_window_decode_is_exercised() {
    let Some(path) = modis_fixture_named("MOD09A1.A2022169.h04v09.061.2022178214640.hdf") else {
        return;
    };

    let first_window = decode_hdf4_sds_i16_window_at_in_file(
        &path,
        "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01",
        0,
        8,
    );
    match first_window {
        Ok(values) => {
            assert!(!values.is_empty(), "decoded first window should not be empty");
            assert!(values.len() <= 8, "decoded first window should respect max_values");
            let probe = probe_hdf4_sds_i16_payload_window_in_file(
                &path,
                "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01",
                8,
            )
            .expect("MOD09 first-window probe should succeed");
            assert!(
                values == probe.little_endian_preview || values == probe.big_endian_preview,
                "decoded first window should match one of the probe previews"
            );
        }
        Err(err) => {
            let msg = format!("{err}");
            assert!(
                msg.contains("status="),
                "failed offset-window decode should include status diagnostics"
            );
        }
    }

    let offset_window = decode_hdf4_sds_i16_window_at_in_file(
        &path,
        "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01",
        4,
        8,
    );
    match offset_window {
        Ok(values) => {
            assert!(!values.is_empty(), "decoded offset window should not be empty");
            assert!(values.len() <= 8, "decoded offset window should respect max_values");
            let probe = probe_hdf4_sds_i16_payload_window_in_file(
                &path,
                "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01",
                8,
            )
            .expect("MOD09 offset-window probe should succeed");
            assert!(
                values == probe.little_endian_preview || values == probe.big_endian_preview,
                "decoded offset window should match one of the probe previews"
            );
        }
        Err(err) => {
            let msg = format!("{err}");
            assert!(
                msg.contains("status="),
                "failed offset-window decode should include status diagnostics"
            );
        }
    }
}

#[test]
fn modis_mod09_hdf4_offset_window_decode_reports_out_of_bounds_start() {
    let Some(path) = modis_fixture_named("MOD09A1.A2022169.h04v09.061.2022178214640.hdf") else {
        return;
    };

    let err = decode_hdf4_sds_i16_window_at_in_file(
        &path,
        "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01",
        usize::MAX,
        8,
    )
    .expect_err("out-of-bounds window start should return invalid-input diagnostics");
    let msg = format!("{err}");
    assert!(
        msg.contains("out of bounds"),
        "out-of-bounds decode should report invalid start diagnostics"
    );
}

#[test]
fn modis_mod09_hdf4_offset_window_decode_rejects_zero_max_values() {
    let Some(path) = modis_fixture_named("MOD09A1.A2022169.h04v09.061.2022178214640.hdf") else {
        return;
    };

    let err = decode_hdf4_sds_i16_window_at_in_file(
        &path,
        "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01",
        0,
        0,
    )
    .expect_err("zero max_values should return invalid-input diagnostics");
    let msg = format!("{err}");
    assert!(
        msg.contains("max_values") || msg.contains("window decode requires"),
        "zero-length window decode should report max_values diagnostics"
    );
}

#[test]
fn modis_mod11_hdf4_eos_metadata_probe_enumerates_expected_fields() {
    let Some(path) = modis_fixture_named("MOD11A2.A2024041.h01v10.061.2024051155231.hdf") else {
        return;
    };

    let summary = probe_hdf4_eos_metadata_in_file(&path)
        .expect("MOD11 HDF4 EOS metadata probe should succeed");
    assert!(summary.struct_metadata_markers >= 1);
    assert!(
        summary
            .grid_names
            .iter()
            .any(|name| name == "MODIS_Grid_8Day_1km_LST")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "LST_Day_1km")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "QC_Day")
    );

    let field = find_modis_field(&summary, "MODIS_Grid_8Day_1km_LST", "LST_Day_1km")
        .expect("MOD11 field metadata should include LST_Day_1km");
    assert_eq!(field.data_type.as_deref(), Some("DFNT_UINT16"));
    assert_eq!(field.dim_list, vec!["YDim", "XDim"]);

    let grid = find_modis_grid(&summary, "MODIS_Grid_8Day_1km_LST")
        .expect("MOD11 grid metadata should be present");
    assert_eq!(grid.projection.as_deref(), Some("GCTP_SNSOID"));
    assert_eq!(grid.sphere_code, Some(-1));
    assert_eq!(grid.upper_left_mtrs, Some((-18903158.836031, -1111950.519767)));
    assert_eq!(grid.lower_right_mtrs, Some((-17791208.316264, -2223901.039533)));
    assert_eq!(grid.proj_params.len(), 13);

    let resolved = resolve_hdf4_grid_field(&summary, "MODIS_Grid_8Day_1km_LST", "LST_Day_1km")
        .expect("MOD11 field should resolve with shape");
    assert_eq!(resolved.shape, vec![1200, 1200]);
    assert_eq!(resolved.projection.as_deref(), Some("GCTP_SNSOID"));
    assert_eq!(resolved.upper_left_mtrs, Some((-18903158.836031, -1111950.519767)));
    assert_eq!(resolved.lower_right_mtrs, Some((-17791208.316264, -2223901.039533)));

    let geometry = derive_hdf4_grid_geometry(&resolved)
        .expect("MOD11 resolved field should derive grid geometry");
    assert_eq!(geometry.rows, 1200);
    assert_eq!(geometry.cols, 1200);
    assert!((geometry.pixel_size_x - 926.6254331391667).abs() < 1e-6);
    assert!((geometry.pixel_size_y + 926.6254331383334).abs() < 1e-6);

    let resolved_path =
        resolve_hdf4_dataset_path(&summary, "/MODIS_Grid_8Day_1km_LST/LST_Day_1km")
            .expect("MOD11 canonical path should resolve");
    assert_eq!(resolved_path.shape, vec![1200, 1200]);

    let paths = enumerate_hdf4_dataset_paths(&summary);
    assert!(paths.iter().any(|p| p == "/MODIS_Grid_8Day_1km_LST/LST_Day_1km"));
    assert!(paths.iter().any(|p| p == "/MODIS_Grid_8Day_1km_LST/LST_Night_1km"));

    let probe = probe_hdf4_sds_i16_payload_window_in_file(
        &path,
        "/MODIS_Grid_8Day_1km_LST/LST_Day_1km",
        8,
    )
    .expect("MOD11 payload probe should succeed before decode attempt");

    let decode_attempt = attempt_decode_hdf4_sds_i16_window_in_file(
        &path,
        "/MODIS_Grid_8Day_1km_LST/LST_Day_1km",
        8,
    );

    if probe.status == "decoded_preview" {
        let values = decode_attempt.expect("MOD11 decode attempt should return the probe preview");
        assert!(!values.is_empty(), "decoded MOD11 window values should not be empty");
        assert!(values.len() <= 8, "decoded MOD11 window should respect max_values");
        assert!(
            values == probe.little_endian_preview || values == probe.big_endian_preview,
            "decoded MOD11 window should match one of the probe previews"
        );
    } else {
        let err = decode_attempt.expect_err("MOD11 decode attempt should report diagnostics when the probe cannot decode");
        let msg = format!("{err}");
        assert!(msg.contains("not yet implemented"));
        assert!(msg.contains("/MODIS_Grid_8Day_1km_LST/LST_Day_1km"));
        assert!(msg.contains("DFNT_UINT16"));
        assert!(msg.contains("shape=[1200, 1200]"));
        assert!(msg.contains(&format!("status={}", probe.status)));
    }
}

#[test]
fn modis_mod13_hdf4_eos_metadata_probe_enumerates_expected_fields() {
    let Some(path) = modis_fixture_named("MOD13A1.A2021065.h03v07.061.2021082133352.hdf") else {
        return;
    };

    let summary = probe_hdf4_eos_metadata_in_file(&path)
        .expect("MOD13 HDF4 EOS metadata probe should succeed");
    assert!(summary.struct_metadata_markers >= 1);
    assert!(
        summary
            .grid_names
            .iter()
            .any(|name| name == "MODIS_Grid_16DAY_500m_VI")
    );
    assert!(
        summary
            .data_field_names
            .iter()
            .any(|name| name == "500m 16 days VI Quality")
    );

    let field =
        find_modis_field(&summary, "MODIS_Grid_16DAY_500m_VI", "500m 16 days NDVI")
            .expect("MOD13 field metadata should include NDVI");
    assert_eq!(field.data_type.as_deref(), Some("DFNT_INT16"));
    assert_eq!(field.dim_list, vec!["YDim", "XDim"]);

    let grid = find_modis_grid(&summary, "MODIS_Grid_16DAY_500m_VI")
        .expect("MOD13 grid metadata should be present");
    assert_eq!(grid.projection.as_deref(), Some("GCTP_SNSOID"));
    assert_eq!(grid.sphere_code, Some(-1));
    assert_eq!(grid.upper_left_mtrs, Some((-16679257.795000, 2223901.039333)));
    assert_eq!(grid.lower_right_mtrs, Some((-15567307.275333, 1111950.519667)));
    assert_eq!(grid.proj_params.len(), 13);

    let resolved =
        resolve_hdf4_grid_field(&summary, "MODIS_Grid_16DAY_500m_VI", "500m 16 days NDVI")
            .expect("MOD13 field should resolve with shape");
    assert_eq!(resolved.shape, vec![2400, 2400]);
    assert_eq!(resolved.projection.as_deref(), Some("GCTP_SNSOID"));
    assert_eq!(resolved.upper_left_mtrs, Some((-16679257.795000, 2223901.039333)));
    assert_eq!(resolved.lower_right_mtrs, Some((-15567307.275333, 1111950.519667)));

    let geometry = derive_hdf4_grid_geometry(&resolved)
        .expect("MOD13 resolved field should derive grid geometry");
    assert_eq!(geometry.rows, 2400);
    assert_eq!(geometry.cols, 2400);
    assert!((geometry.pixel_size_x - 463.31271652791664).abs() < 1e-6);
    assert!((geometry.pixel_size_y + 463.3127165275).abs() < 1e-6);

    let resolved_path = resolve_hdf4_dataset_path(
        &summary,
        "/MODIS_Grid_16DAY_500m_VI/500m 16 days NDVI",
    )
    .expect("MOD13 canonical path should resolve");
    assert_eq!(resolved_path.shape, vec![2400, 2400]);

    let paths = enumerate_hdf4_dataset_paths(&summary);
    assert!(paths.iter().any(|p| p == "/MODIS_Grid_16DAY_500m_VI/500m 16 days NDVI"));
    assert!(
        paths
            .iter()
            .any(|p| p == "/MODIS_Grid_16DAY_500m_VI/500m 16 days VI Quality")
    );

    let probe = probe_hdf4_sds_i16_payload_window_in_file(
        &path,
        "/MODIS_Grid_16DAY_500m_VI/500m 16 days NDVI",
        8,
    )
    .expect("MOD13 payload probe should succeed before decode attempt");

    let decode_attempt = attempt_decode_hdf4_sds_i16_window_in_file(
        &path,
        "/MODIS_Grid_16DAY_500m_VI/500m 16 days NDVI",
        8,
    );

    if probe.status == "decoded_preview" {
        let values = decode_attempt.expect("MOD13 decode attempt should return the probe preview");
        assert!(!values.is_empty(), "decoded MOD13 window values should not be empty");
        assert!(values.len() <= 8, "decoded MOD13 window should respect max_values");
        assert!(
            values == probe.little_endian_preview || values == probe.big_endian_preview,
            "decoded MOD13 window should match one of the probe previews"
        );
    } else {
        let err = decode_attempt.expect_err("MOD13 decode attempt should report diagnostics when the probe cannot decode");
        let msg = format!("{err}");
        assert!(msg.contains("not yet implemented"));
        assert!(msg.contains("/MODIS_Grid_16DAY_500m_VI/500m 16 days NDVI"));
        assert!(msg.contains("DFNT_INT16"));
        assert!(msg.contains("shape=[2400, 2400]"));
        assert!(msg.contains(&format!("status={}", probe.status)));
    }
}

#[test]
fn modis_mod09_documented_field_vocabulary_is_discoverable_with_reports() {
    let Some(path) = modis_fixture_named("MOD09A1.A2022169.h04v09.061.2022178214640.hdf") else {
        return;
    };

    let report = dataset_metadata_text_report_in_file(
        &path,
        "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01",
        &[
            "MOD_Grid_500m_Surface_Reflectance",
            "sur_refl_b01",
            "sur_refl_state_500m",
        ],
    )
    .expect("MOD09 metadata report should succeed");
    assert!(
        report.missing_terms.is_empty(),
        "MOD09 documented vocabulary should be discoverable; present={:?}, missing={:?}",
        report.present_terms,
        report.missing_terms,
    );
}

#[test]
fn modis_mod11_documented_field_vocabulary_is_discoverable_with_reports() {
    let Some(path) = modis_fixture_named("MOD11A2.A2024041.h01v10.061.2024051155231.hdf") else {
        return;
    };

    let report = dataset_metadata_text_report_in_file(
        &path,
        "/MODIS_Grid_8Day_1km_LST/LST_Day_1km",
        &["MODIS_Grid_8Day_1km_LST", "LST_Day_1km", "QC_Day"],
    )
    .expect("MOD11 metadata report should succeed");
    assert!(
        report.missing_terms.is_empty(),
        "MOD11 documented vocabulary should be discoverable; present={:?}, missing={:?}",
        report.present_terms,
        report.missing_terms,
    );
}

#[test]
fn modis_mod13_documented_field_vocabulary_is_discoverable_with_reports() {
    let Some(path) = modis_fixture_named("MOD13A1.A2021065.h03v07.061.2021082133352.hdf") else {
        return;
    };

    let report = dataset_metadata_text_report_in_file(
        &path,
        "/MODIS_Grid_16DAY_500m_VI/500m 16 days NDVI",
        &[
            "MODIS_Grid_16DAY_500m_VI",
            "500m 16 days NDVI",
            "500m 16 days VI Quality",
        ],
    )
    .expect("MOD13 metadata report should succeed");
    assert!(
        report.missing_terms.is_empty(),
        "MOD13 documented vocabulary should be discoverable; present={:?}, missing={:?}",
        report.present_terms,
        report.missing_terms,
    );
}

#[test]
fn atl08_h_canopy_v1_object_header_layout_and_filter_are_decoded() {
    let Some(path) = fixture_named("ATL08_20181120185605_08120102_007_01.h5") else {
        return;
    };

    let parsed = parse_v1_object_header_in_file(&path, 328_097)
        .expect("ATL08 h_canopy v1 object header should parse");

    assert_eq!(parsed.message_count, 16);
    assert_eq!(parsed.dataspaces.len(), 1);
    assert_eq!(parsed.dataspaces[0].dimensions, vec![7415]);
    assert_eq!(parsed.datatypes.len(), 1);
    assert_eq!(parsed.datatypes[0].size, 4);

    assert_eq!(parsed.fill_values.len(), 1);
    assert_eq!(parsed.fill_values[0].version, 4);
    assert_eq!(parsed.fill_values[0].allocation_time, 2);
    assert_eq!(parsed.fill_values[0].fill_time, 2);
    assert_eq!(parsed.fill_values[0].value_defined, 1);
    assert_eq!(parsed.fill_values[0].value_size, 4);
    assert_eq!(parsed.fill_values[0].value_bytes, vec![0xff, 0xff, 0x7f, 0x7f]);
    let fill = decode_f32(
        parsed.fill_values[0].value_bytes.clone().try_into().expect("fill value should be f32"),
        Endianness::Little,
    );
    assert_eq!(fill, f32::MAX);

    assert_eq!(parsed.filter_pipelines.len(), 1);
    assert_eq!(parsed.filter_pipelines[0].filters.len(), 1);
    assert_eq!(parsed.filter_pipelines[0].filters[0].id, 1);
    assert_eq!(parsed.filter_pipelines[0].filters[0].name, "deflate");
    assert_eq!(parsed.filter_pipelines[0].filters[0].client_data, vec![6]);

    assert_eq!(parsed.chunked_layouts.len(), 1);
    assert_eq!(parsed.chunked_layouts[0].version, 3);
    assert_eq!(parsed.chunked_layouts[0].layout_class, 2);
    assert_eq!(parsed.chunked_layouts[0].index_address, 326_001);
    assert_eq!(parsed.chunked_layouts[0].chunk_dimensions, vec![10_000, 4]);

    assert_eq!(parsed.continuations.len(), 1);
    assert_eq!(parsed.continuations[0].address, 2_011_663);
    assert_eq!(parsed.continuations[0].size, 112);
}

#[test]
fn atl08_h_canopy_first_chunk_decodes_to_f32_values() {
    let Some(path) = fixture_named("ATL08_20181120185605_08120102_007_01.h5") else {
        return;
    };

    let parsed = parse_v1_object_header_in_file(&path, 328_097)
        .expect("ATL08 h_canopy v1 object header should parse");
    let layout = &parsed.chunked_layouts[0];
    let first_record = read_first_chunked_storage_leaf_record_in_file(
        &path,
        layout.index_address,
        layout.num_dimensions as usize,
    )
    .expect("ATL08 first chunked-storage leaf record should parse");

    assert_eq!(first_record.chunk_size, 13_494);
    assert_eq!(first_record.chunk_address, 9_489_637);
    assert_eq!(first_record.chunk_offsets, vec![0, 0]);

    let compressed = read_chunk_payload_in_file(&path, first_record.chunk_address, first_record.chunk_size)
        .expect("ATL08 first compressed chunk should be readable");
    let decompressed = decompress_zlib(&compressed).expect("ATL08 first chunk should zlib-decompress");
    assert_eq!(decompressed.len(), 40_000);

    let values = decode_f32_slice(&decompressed, Endianness::Little)
        .expect("ATL08 first chunk payload should decode as little-endian f32");
    assert_eq!(values.len(), 10_000);
    assert_eq!(values[0], f32::MAX);
    assert_eq!(values[1], f32::MAX);
    assert_eq!(values[2], f32::MAX);

    let fill = decode_f32(
        parsed.fill_values[0].value_bytes.clone().try_into().expect("fill value should be f32"),
        Endianness::Little,
    );
    let mapped = apply_fill_value_mapping_f32(&values, Some(fill), -9999.0);
    assert_eq!(mapped.values.len(), 10_000);
    assert_eq!(mapped.nodata_value, -9999.0);
    assert_eq!(mapped.valid_count, 3_640);
    assert_eq!(mapped.nodata_count, 6_360);
    assert_eq!(mapped.values[0], -9999.0);
}

#[test]
fn atl08_h_canopy_bounded_chunk_index_probe_returns_records() {
    let Some(path) = fixture_named("ATL08_20181120185605_08120102_007_01.h5") else {
        return;
    };

    let parsed = parse_v1_object_header_in_file(&path, 328_097)
        .expect("ATL08 h_canopy v1 object header should parse");
    let layout = &parsed.chunked_layouts[0];
    let first_record = read_first_chunked_storage_leaf_record_in_file(
        &path,
        layout.index_address,
        layout.num_dimensions as usize,
    )
    .expect("ATL08 first chunked-storage leaf record should parse");
    let direct_leaf_chain_records = read_chunked_storage_leaf_chain_records_in_file(
        &path,
        layout.index_address,
        layout.num_dimensions as usize,
        8,
        8,
    )
    .expect("ATL08 direct leaf-chain probe should return chunk records");

    let records = read_chunked_storage_records_bounded_in_file(
        &path,
        layout.index_address,
        layout.num_dimensions as usize,
        8,
        8,
    )
    .expect("ATL08 bounded chunk index probe should return chunk records");

    assert!(!records.is_empty());
    assert_eq!(records, direct_leaf_chain_records);
    if records.len() >= 4 {
        let mid_start = (records.len() - 2) / 2;
        let mid_end = mid_start + 2;
        assert_eq!(
            &records[mid_start..mid_end],
            &direct_leaf_chain_records[mid_start..mid_end]
        );
    }
    let tail_len = records.len().min(2);
    assert_eq!(
        &records[records.len() - tail_len..],
        &direct_leaf_chain_records[direct_leaf_chain_records.len() - tail_len..]
    );
    assert_eq!(records[0], first_record);
    assert_eq!(records[0].chunk_offsets, vec![0, 0]);
    assert_eq!(records[0].chunk_size, 13_494);
    assert_eq!(records[0].chunk_address, 9_489_637);

    let compressed = read_chunk_payload_in_file(&path, records[0].chunk_address, records[0].chunk_size)
        .expect("ATL08 bounded first chunk should be readable");
    let decompressed = decompress_zlib(&compressed)
        .expect("ATL08 bounded first chunk should zlib-decompress");
    assert_eq!(decompressed.len(), 40_000);

    let values = decode_f32_slice(&decompressed, Endianness::Little)
        .expect("ATL08 bounded first chunk payload should decode as little-endian f32");
    assert_eq!(values.len(), 10_000);
    assert_eq!(values[0], f32::MAX);
    assert_eq!(values[1], f32::MAX);
    assert_eq!(values[2], f32::MAX);

    let fill = decode_f32(
        parsed.fill_values[0].value_bytes.clone().try_into().expect("fill value should be f32"),
        Endianness::Little,
    );
    let mapped = apply_fill_value_mapping_f32(&values, Some(fill), -9999.0);
    assert_eq!(mapped.values.len(), 10_000);
    assert_eq!(mapped.nodata_value, -9999.0);
    assert_eq!(mapped.valid_count, 3_640);
    assert_eq!(mapped.nodata_count, 6_360);
    assert_eq!(mapped.values[0], -9999.0);
}

#[test]
fn atl08_h_te_best_fit_bounded_chunk_index_probe_returns_records() {
    let Some(path) = fixture_named("ATL08_20181120185605_08120102_007_01.h5") else {
        return;
    };

    let descriptor = resolve_dataset_in_file(&path, "/gt1l/land_segments/terrain/h_te_best_fit")
        .expect("ATL08 fixture should expose canonical terrain-height path marker");
    assert_eq!(descriptor.path, "/gt1l/land_segments/terrain/h_te_best_fit");

    let parsed = parse_v1_object_header_in_file(&path, 392_317)
        .expect("ATL08 h_te_best_fit v1 object header should parse");
    assert!(!parsed.chunked_layouts.is_empty());
    let layout = &parsed.chunked_layouts[0];
    let first_record = read_first_chunked_storage_leaf_record_in_file(
        &path,
        layout.index_address,
        layout.num_dimensions as usize,
    )
    .expect("ATL08 h_te_best_fit first chunked-storage leaf record should parse");
    let direct_leaf_chain_records = read_chunked_storage_leaf_chain_records_in_file(
        &path,
        layout.index_address,
        layout.num_dimensions as usize,
        8,
        8,
    )
    .expect("ATL08 h_te_best_fit direct leaf-chain probe should return chunk records");

    let records = read_chunked_storage_records_bounded_in_file(
        &path,
        layout.index_address,
        layout.num_dimensions as usize,
        8,
        8,
    )
    .expect("ATL08 h_te_best_fit bounded chunk index probe should return chunk records");

    assert!(!records.is_empty());
    assert_eq!(records, direct_leaf_chain_records);
    assert_eq!(records[0], first_record);
    assert_eq!(records[0].chunk_offsets.len(), layout.num_dimensions as usize);
    assert_eq!(records[0].chunk_offsets[0], 0);

    let compressed = read_chunk_payload_in_file(&path, records[0].chunk_address, records[0].chunk_size)
        .expect("ATL08 h_te_best_fit bounded first chunk should be readable");
    let decompressed =
        decompress_zlib(&compressed).expect("ATL08 h_te_best_fit bounded first chunk should zlib-decompress");
    assert!(!decompressed.is_empty());

    let values = decode_f32_slice(&decompressed, Endianness::Little)
        .expect("ATL08 h_te_best_fit bounded first chunk payload should decode as little-endian f32");
    assert!(!values.is_empty());
    assert!(
        values.iter().any(|v| v.is_finite() && *v > -500.0 && *v < 9000.0),
        "ATL08 h_te_best_fit should contain plausible finite terrain elevations"
    );
}

#[test]
fn atl08_terrain_slope_bounded_chunk_index_probe_returns_records() {
    let Some(path) = fixture_named("ATL08_20181120185605_08120102_007_01.h5") else {
        return;
    };

    let descriptor = resolve_dataset_in_file(&path, "/gt1l/land_segments/terrain/terrain_slope")
        .expect("ATL08 fixture should expose canonical terrain_slope path marker");
    assert_eq!(descriptor.path, "/gt1l/land_segments/terrain/terrain_slope");

    let parsed = parse_v1_object_header_in_file(&path, 441_845)
        .expect("ATL08 terrain_slope v1 object header should parse");
    assert!(!parsed.chunked_layouts.is_empty());
    let layout = &parsed.chunked_layouts[0];
    assert_eq!(layout.index_address, 439_749);

    let tree_address = layout.index_address as usize;
    let header_len = wbhdf::btree::NODE_HEADER_LEN;
    let bytes = std::fs::read(&path).expect("ATL08 fixture should be readable for header probe");
    assert!(
        bytes.len() >= tree_address + header_len,
        "ATL08 fixture should include terrain_slope chunk-index node header bytes"
    );
    let root_header = parse_node_header(&bytes[tree_address..tree_address + header_len])
        .expect("ATL08 terrain_slope chunk-index root header should parse");
    assert!(
        root_header.node_level <= 8,
        "ATL08 terrain_slope chunk-index root level should be bounded"
    );

    let first_record = read_first_chunked_storage_leaf_record_in_file(
        &path,
        layout.index_address,
        layout.num_dimensions as usize,
    )
    .expect("ATL08 terrain_slope first chunked-storage leaf record should parse");
    let direct_leaf_chain_records = read_chunked_storage_leaf_chain_records_in_file(
        &path,
        layout.index_address,
        layout.num_dimensions as usize,
        8,
        8,
    )
    .expect("ATL08 terrain_slope direct leaf-chain probe should return chunk records");

    let records = read_chunked_storage_records_bounded_in_file(
        &path,
        layout.index_address,
        layout.num_dimensions as usize,
        8,
        8,
    )
    .expect("ATL08 terrain_slope bounded chunk index probe should return chunk records");

    assert!(!records.is_empty());
    assert_eq!(records, direct_leaf_chain_records);
    assert_eq!(records[0], first_record);

    let compressed = read_chunk_payload_in_file(&path, records[0].chunk_address, records[0].chunk_size)
        .expect("ATL08 terrain_slope bounded first chunk should be readable");
    let decompressed =
        decompress_zlib(&compressed).expect("ATL08 terrain_slope bounded first chunk should zlib-decompress");
    assert!(!decompressed.is_empty());

    let values = decode_f32_slice(&decompressed, Endianness::Little)
        .expect("ATL08 terrain_slope bounded first chunk payload should decode as little-endian f32");
    assert!(!values.is_empty());
    assert!(
        values.iter().any(|v| v.is_finite() && *v > -10.0 && *v < 90.0),
        "ATL08 terrain_slope should contain plausible finite slope values"
    );
}

#[test]
fn atl08_h_canopy_20m_bounded_chunk_index_probe_returns_records() {
    let Some(path) = fixture_named("ATL08_20181120185605_08120102_007_01.h5") else {
        return;
    };

    let descriptor = resolve_dataset_in_file(&path, "/gt1l/land_segments/canopy/h_canopy_20m")
        .expect("ATL08 fixture should expose canonical h_canopy_20m path marker");
    assert_eq!(descriptor.path, "/gt1l/land_segments/canopy/h_canopy_20m");

    let parsed = parse_v1_object_header_in_file(&path, 331_609)
        .expect("ATL08 h_canopy_20m v1 object header should parse");
    assert!(!parsed.chunked_layouts.is_empty());
    let layout = &parsed.chunked_layouts[0];
    assert_eq!(layout.index_address, 328_993);
    assert_eq!(layout.chunk_dimensions, vec![10_000, 5, 4]);

    let tree_address = layout.index_address as usize;
    let header_len = wbhdf::btree::NODE_HEADER_LEN;
    let bytes = std::fs::read(&path).expect("ATL08 fixture should be readable for header probe");
    assert!(
        bytes.len() >= tree_address + header_len,
        "ATL08 fixture should include h_canopy_20m chunk-index node header bytes"
    );
    let root_header = parse_node_header(&bytes[tree_address..tree_address + header_len])
        .expect("ATL08 h_canopy_20m chunk-index root header should parse");
    assert!(
        root_header.node_level <= 8,
        "ATL08 h_canopy_20m chunk-index root level should be bounded"
    );

    let first_record = read_first_chunked_storage_leaf_record_in_file(
        &path,
        layout.index_address,
        layout.num_dimensions as usize,
    )
    .expect("ATL08 h_canopy_20m first chunked-storage leaf record should parse");
    let direct_leaf_chain_records = read_chunked_storage_leaf_chain_records_in_file(
        &path,
        layout.index_address,
        layout.num_dimensions as usize,
        8,
        8,
    )
    .expect("ATL08 h_canopy_20m direct leaf-chain probe should return chunk records");

    let records = read_chunked_storage_records_bounded_in_file(
        &path,
        layout.index_address,
        layout.num_dimensions as usize,
        8,
        8,
    )
    .expect("ATL08 h_canopy_20m bounded chunk index probe should return chunk records");

    assert!(!records.is_empty());
    assert_eq!(records, direct_leaf_chain_records);
    assert_eq!(records[0], first_record);

    let compressed = read_chunk_payload_in_file(&path, records[0].chunk_address, records[0].chunk_size)
        .expect("ATL08 h_canopy_20m bounded first chunk should be readable");
    let decompressed =
        decompress_zlib(&compressed).expect("ATL08 h_canopy_20m bounded first chunk should zlib-decompress");
    assert_eq!(decompressed.len(), 200_000);

    let values = decode_f32_slice(&decompressed, Endianness::Little)
        .expect("ATL08 h_canopy_20m bounded first chunk payload should decode as little-endian f32");
    assert_eq!(values.len(), 50_000);
    assert!(
        values.iter().any(|v| v.is_finite() && *v > -500.0 && *v < 9000.0),
        "ATL08 h_canopy_20m should contain plausible finite canopy elevations"
    );
}

#[test]
fn dataset_chunk_locator_matches_known_reference_addresses() {
    let locator = DatasetChunkLocator::with_known_addresses(
        "/HDFEOS/GRIDS/VNP_Grid_1km_2D/Data Fields/SurfReflect_M1",
        &[(0, 1200), (1, 2400), (2, 3600)],
    )
    .expect("locator should construct");

    let expectations = [(0_u64, 1200_u64), (1_u64, 2400_u64), (2_u64, 3600_u64)];
    for (coord, expected) in expectations {
        let actual = locator
            .locate_chunk_address(&[coord])
            .expect("known chunk key should resolve");
        assert_eq!(actual, expected);
    }
}
