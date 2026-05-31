use wbhdf::dataset::resolve_dataset;

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
