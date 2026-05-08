use std::path::Path;

fn main() {
    // Test sample HFA files
    let test_files = vec![
        ("crates/wbraster/test_data/hfa_samples/spill.img", "NAD27 UTM Zone 11N (Primary Tier B Test)"),
        ("crates/wbraster/test_data/hfa_samples/utmsmall.img", "NAD27 UTM (Primary Tier B Test)"),
        ("crates/wbraster/test_data/hfa_samples/int.img", "Transverse Mercator (Secondary Test)"),
        ("crates/wbraster/test_data/hfa_samples/float.img", "Transverse Mercator (Secondary Test)"),
        ("crates/wbraster/test_data/hfa_samples/87test.img", "World_Cube (WKT Fallback Test)"),
    ];

    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║            HFA Tier B CRS Detection — Sample File Validation                 ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    for (file_path, description) in test_files {
        if !Path::new(file_path).exists() {
            println!("⊘ {} — File not found ({})", file_path, description);
            continue;
        }

        print!("→ {} ", file_path);
        match wbraster::Raster::read(file_path) {
            Ok(raster) => {
                println!("✓");
                
                println!("  Description: {}", description);
                println!("  Dimensions: {} × {} pixels, {} band(s)", 
                    raster.cols, 
                    raster.rows,
                    raster.bands
                );
                println!("  Data type: {:?}", raster.data_type);
                
                if let Some(epsg) = raster.crs.epsg {
                    println!("  ✓ CRS (Tier B/A): EPSG:{}", epsg);
                } else if let Some(wkt) = &raster.crs.wkt {
                    let wkt_preview = if wkt.len() > 100 { 
                        format!("{}...", &wkt[..97]) 
                    } else { 
                        wkt.to_string() 
                    };
                    println!("  ⓘ CRS (WKT): {}", wkt_preview);
                } else {
                    println!("  ⊘ CRS: Not detected (may be geographic or unrecognized projection)");
                }
                
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
