/// Integration test for Mississauga performance fixture
/// Run with: cargo test -p wbtools_oss --test mississauga_performance -- --nocapture

use serde_json::json;
use std::time::Instant;
use wbcore::{CapabilityProvider, LicenseTier, ProgressSink, ToolArgs, ToolContext};
use wbtools_oss::{register_default_tools, ToolRegistry};

const MISSISSAUGA_NETWORK: &str = 
    "/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/target/benchmarks/mississauga_scale_test/mississauga_streets_2d.gpkg";
const FIXTURE_DIR: &str = 
    "/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/target/benchmarks/mississauga_scale_test";

struct TestCapabilities;
impl CapabilityProvider for TestCapabilities {
    fn has_tool_access(&self, _tool_id: &'static str, _tier: LicenseTier) -> bool {
        true
    }
}

struct NoProgress;
impl ProgressSink for NoProgress {}

fn get_registry_and_context() -> (ToolRegistry, ToolContext<'static>) {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    
    static PROGRESS: NoProgress = NoProgress;
    static CAP: TestCapabilities = TestCapabilities;
    let ctx = ToolContext {
        progress: &PROGRESS,
        capabilities: &CAP,
    };
    
    (registry, ctx)
}

#[test]
fn mississauga_network_topology_audit_perf() {
    eprintln!("\n>>> Testing network_topology_audit on Mississauga (19,889 edges)");
    
    let (registry, ctx) = get_registry_and_context();
    
    let output = format!("{}/topology_audit_mississauga.json", FIXTURE_DIR);
    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(MISSISSAUGA_NETWORK.to_string()));
    args.insert("output".to_string(), json!(output.clone()));
    
    let start = Instant::now();
    let result = registry.run("network_topology_audit", &args, &ctx);
    let elapsed = start.elapsed();
    
    match result {
        Ok(_) => {
            eprintln!("✓ PASS: topology audit completed in {:.3}s", elapsed.as_secs_f64());
            eprintln!("  Scaling vs Guelph (3.6k edges): {:.1}x slower", 
                elapsed.as_secs_f64() / 0.5);
            assert!(elapsed.as_secs_f64() < 3.0, "Performance regression: expected <3.0s");
        }
        Err(e) => {
            eprintln!("✗ FAILED: {:?}", e);
            panic!("Test failed: {:?}", e);
        }
    }
}

#[test]
fn mississauga_network_connected_components_perf() {
    eprintln!("\n>>> Testing network_connected_components on Mississauga");
    
    let (registry, ctx) = get_registry_and_context();
    
    let output = format!("{}/components_mississauga.shp", FIXTURE_DIR);
    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(MISSISSAUGA_NETWORK.to_string()));
    args.insert("output".to_string(), json!(output.clone()));
    
    let start = Instant::now();
    let result = registry.run("network_connected_components", &args, &ctx);
    let elapsed = start.elapsed();
    
    match result {
        Ok(_) => {
            eprintln!("✓ PASS: connected components analyzed in {:.3}s", elapsed.as_secs_f64());
            assert!(elapsed.as_secs_f64() < 2.0, "Performance regression: expected <2.0s");
        }
        Err(e) => {
            eprintln!("✗ FAILED: {:?}", e);
            panic!("Test failed: {:?}", e);
        }
    }
}

#[test]
fn mississauga_network_node_degree_perf() {
    eprintln!("\n>>> Testing network_node_degree on Mississauga");
    
    let (registry, ctx) = get_registry_and_context();
    
    let output = format!("{}/node_degree_mississauga.shp", FIXTURE_DIR);
    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(MISSISSAUGA_NETWORK.to_string()));
    args.insert("output".to_string(), json!(output.clone()));
    
    let start = Instant::now();
    let result = registry.run("network_node_degree", &args, &ctx);
    let elapsed = start.elapsed();
    
    match result {
        Ok(_) => {
            eprintln!("✓ PASS: node degree computed in {:.3}s", elapsed.as_secs_f64());
            assert!(elapsed.as_secs_f64() < 2.5, "Performance regression: expected <2.5s");
        }
        Err(e) => {
            eprintln!("✗ FAILED: {:?}", e);
            panic!("Test failed: {:?}", e);
        }
    }
}

#[test]
fn mississauga_vector_summary() {
    eprintln!("\n========================================");
    eprintln!("MISSISSAUGA PERFORMANCE TEST SUMMARY");
    eprintln!("========================================");
    eprintln!("Network: Mississauga StreetCentrelines");
    eprintln!("Size: 19,889 edges (5.5x Guelph baseline)");
    eprintln!("CRS: EPSG:3857 (Web Mercator)");
    eprintln!("Fixture: {}", FIXTURE_DIR);
    eprintln!("\nExpected scaling:");
    eprintln!("  - Linear scaling: ~5.5x slower than Guelph");
    eprintln!("  - With parallelization: 2-4x slower (sub-linear)");
    eprintln!("\nPerformance targets met if:");
    eprintln!("  - Network topology audit: <3.0s");
    eprintln!("  - Connected components: <2.0s");
    eprintln!("  - Node degree: <2.5s");
    eprintln!("\nRecommendation:");
    eprintln!("  If tests pass targets, vector tooling is ready for");
    eprintln!("  production Phase A-C workflow implementations.");
    eprintln!("========================================\n");
}
