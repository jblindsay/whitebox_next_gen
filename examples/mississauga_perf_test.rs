use std::time::Instant;
use wbtools_oss::RuntimeEnvironment;

fn main() {
    println!("========================================");
    println!("MISSISSAUGA VECTOR GIS PERFORMANCE TEST");
    println!("========================================\n");

    let mississauga_network = "/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/target/benchmarks/mississauga_scale_test/mississauga_streets_2d.gpkg";
    let fixture_dir = "/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/target/benchmarks/mississauga_scale_test";

    println!("Fixture: {} (19,889 edges)", mississauga_network);
    println!("Scaling: 5.5x Guelph baseline\n");

    // Initialize runtime
    let mut env = RuntimeEnvironment::new();
    let tool_ids = env.tool_ids();
    
    println!("Loaded {} tools from registry", tool_ids.len());
    println!("\nRunning performance suite...\n");
    println!("{:<50} {:>12} {:>15}", "Tool", "Time (s)", "Status");
    println!("{:-<80}", "");

    // Test 1: Network Topology Audit
    {
        let start = Instant::now();
        let output = format!("{}/topology_audit_mississauga.json", fixture_dir);
        let params = vec![
            ("network", mississauga_network),
            ("output", &output),
        ];
        
        match env.run_tool("network_topology_audit", &params) {
            Ok(_) => {
                let elapsed = start.elapsed().as_secs_f64();
                let status = if elapsed <= 2.0 { "✓ PASS" } else { "⚠ SLOW" };
                println!("{:<50} {:>12.3} {:>15}", "Network Topology Audit", elapsed, status);
            }
            Err(e) => println!("{:<50} {:>12} {:>15}", "Network Topology Audit", "FAILED", &e[..20.min(e.len())]),
        }
    }

    // Test 2: Connected Components
    {
        let start = Instant::now();
        let output = format!("{}/components_mississauga.shp", fixture_dir);
        let params = vec![
            ("network", mississauga_network),
            ("output", &output),
        ];
        
        match env.run_tool("network_connected_components", &params) {
            Ok(_) => {
                let elapsed = start.elapsed().as_secs_f64();
                let status = if elapsed <= 1.5 { "✓ PASS" } else { "⚠ SLOW" };
                println!("{:<50} {:>12.3} {:>15}", "Connected Components", elapsed, status);
            }
            Err(e) => println!("{:<50} {:>12} {:>15}", "Connected Components", "FAILED", &e[..20.min(e.len())]),
        }
    }

    println!("\n{:-<80}", "");
    println!("Performance test completed.\n");

    println!("=== SCALING ANALYSIS ===");
    println!("Mississauga: 19,889 edges (5.5x vs Guelph 3,679)");
    println!("If linear scaling:      ~5.5x slower");
    println!("If good parallelization: 2-4x slower (sub-linear)\n");

    println!("Expected scalability:");
    println!("  - Linear tools (e.g., topology audit):     ~2.5-3.0 seconds");
    println!("  - Network analysis (connectivity):         ~1.2-1.5 seconds");
    println!("  - OD matrix (200x200):                     ~3.0-5.0 seconds");
    println!("  - Service area with 3 rings:               ~2.0-3.0 seconds\n");
}
