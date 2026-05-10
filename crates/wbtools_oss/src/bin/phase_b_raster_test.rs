/// Phase B Raster Interop Test Binary
/// Tests roundtrip read/write of raster formats via wbraster
use wbraster::{Raster, RasterFormat};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 4 {
        eprintln!("Usage: phase_b_raster_test <case_id> <input_path> <output_path>");
        eprintln!("Example: phase_b_raster_test R01 /tmp/input.tif /tmp/output.tif");
        std::process::exit(1);
    }

    let case_id = &args[1];
    let input_path = &args[2];
    let output_path = &args[3];

    println!("=== Phase B Raster Test: {} ===", case_id);
    println!("Input: {}", input_path);
    println!("Output: {}", output_path);

    // Read raster
    match read_raster(input_path) {
        Ok(raster) => {
            println!(
                "✓ Read: {} x {} ({})",
                raster.rows, raster.cols, raster.data_type
            );
            println!("  CRS: {:?}", raster.crs);
            println!("  NoData: {:?}", raster.nodata);

            // Write raster
            match write_raster(&raster, output_path) {
                Ok(_) => {
                    println!("✓ Write: {}", output_path);
                    println!("STATUS: PASS");
                    std::process::exit(0);
                }
                Err(e) => {
                    eprintln!("✗ Write failed: {}", e);
                    println!("STATUS: FAIL");
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("✗ Read failed: {}", e);
            println!("STATUS: FAIL");
            std::process::exit(1);
        }
    }
}

fn read_raster(path: &str) -> Result<Raster, String> {
    Raster::read(path).map_err(|e| e.to_string())
}

fn write_raster(raster: &Raster, path: &str) -> Result<(), String> {
    let format = RasterFormat::for_output_path(path).map_err(|e| e.to_string())?;
    raster.write(path, format).map_err(|e| e.to_string())
}
