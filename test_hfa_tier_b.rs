/// Quick test for HFA Tier B CRS detection
/// Run with: cargo run --release --bin test_hfa_tier_b
use std::path::Path;

fn main() {
    // Test sample HFA files
    let test_files = vec![
        "crates/wbraster/test_data/hfa_samples/spill.img",
        "crates/wbraster/test_data/hfa_samples/utmsmall.img",
        "crates/wbraster/test_data/hfa_samples/int.img",
        "crates/wbraster/test_data/hfa_samples/float.img",
        "crates/wbraster/test_data/hfa_samples/87test.img",
    ];

    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║            HFA Tier B CRS Detection — Sample File Validation                 ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    for file_path in test_files {
        if !Path::new(file_path).exists() {
            println!("⊘ {} — File not found", file_path);
            continue;
        }

        print!("Reading {} ... ", file_path);
        match wbraster::Raster::read(file_path) {
            Ok(raster) => {
                println!("✓");
                
                let crs = &raster.config.crs;
                println!("  Dimensions: {} × {} ({} bands)", 
                    raster.config.width, 
                    raster.config.height,
                    raster.config.band_count
                );
                
                if let Some(epsg) = crs.epsg {
                    println!("  CRS (Tier B): EPSG:{}", epsg);
                } else if let Some(wkt) = &crs.wkt {
                    println!("  CRS (WKT): {}", &wkt[..wkt.len().min(80)]);
                } else {
                    println!("  CRS: No CRS detected (geographic or unrecognized projection)");
                }
                
                // Print data type
                println!("  Data type: {:?}", raster.config.data_type);
                println!();
            }
            Err(e) => {
                println!("✗ Error: {}", e);
                println!();
            }
        }
    }

    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║                        Tier B Test Complete                                 ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝");
}
