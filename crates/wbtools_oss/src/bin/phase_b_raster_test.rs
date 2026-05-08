/// Phase B Raster Interop Test Binary
/// Tests roundtrip read/write of raster formats via wbraster
use std::fs;
use std::path::Path;
use wbraster::Raster;

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
                raster.configs.rows, raster.configs.cols, raster.configs.data_type
            );
            println!("  CRS: {:?}", raster.get_crs());
            println!("  NoData: {:?}", raster.get_nodata());

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
    let path_obj = Path::new(path);
    let ext = path_obj
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "tif" | "tiff" => wbraster::read_geotiff(path),
        "img" => wbraster::read_hfa(path),
        "flt" => wbraster::read_esri_float_grid(path),
        "dt0" | "dt1" | "dt2" => wbraster::read_dted(path),
        "xyz" => wbraster::read_xyz_grid(path),
        "png" => wbraster::read_png(path),
        "jpg" | "jpeg" => wbraster::read_jpeg(path),
        _ => Err(format!("Unsupported format: {}", ext)),
    }
}

fn write_raster(raster: &Raster, path: &str) -> Result<(), String> {
    let path_obj = Path::new(path);
    let ext = path_obj
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "tif" | "tiff" => wbraster::write_geotiff(raster, path),
        "img" => wbraster::write_hfa(raster, path),
        "flt" => wbraster::write_esri_float_grid(raster, path),
        "dt0" | "dt1" | "dt2" => wbraster::write_dted(raster, path),
        "xyz" => wbraster::write_xyz_grid(raster, path),
        "png" => wbraster::write_png(raster, path),
        "jpg" | "jpeg" => wbraster::write_jpeg(raster, path),
        _ => Err(format!("Unsupported format: {}", ext)),
    }
}
