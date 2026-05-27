use wbcore::{AllowAllCapabilities, RecordingProgressSink, ToolError, ToolRuntimeRegistry};
use wbtools_pro::{register_default_tools, ToolRegistry};

fn assert_runtime_registry_impl<T: ToolRuntimeRegistry>() {}

#[test]
fn shim_exposes_expected_registry_contract() {
    assert_runtime_registry_impl::<ToolRegistry>();

    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    // Contract requirement: list and manifests are callable and shape-compatible.
    let listed = registry.list();
    let manifests = registry.manifests();
    assert_eq!(listed.len(), manifests.len());
}

#[test]
fn shim_reports_not_found_for_unknown_tool() {
    let registry = ToolRegistry::new();

    let args = wbcore::ToolArgs::new();
    let caps = AllowAllCapabilities;
    let progress = RecordingProgressSink::new();
    let ctx = wbcore::ToolContext {
        progress: &progress,
        capabilities: &caps,
    };

    let err = registry
        .run("definitely_not_a_real_tool", &args, &ctx)
        .expect_err("unknown tool id should return NotFound");

    match err {
        ToolError::NotFound(id) => assert_eq!(id, "definitely_not_a_real_tool"),
        other => panic!("expected ToolError::NotFound, got {other:?}"),
    }
}
