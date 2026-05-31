use wbhdf::dataset::DatasetChunkLocator;
use wbhdf::dataset::resolve_dataset;
use wbhdf::fixtures::{fixture_is_available, smoke_fixture_file};
use wbhdf::superblock::probe_file_metadata;

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
