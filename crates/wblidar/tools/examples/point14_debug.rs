use std::fs::File;
use std::io::BufReader;
use wblidar::laz::LazReader;
use wblidar::las::LasReader;
use wblidar::PointReader;

fn main() {
    let file_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/Users/johnlindsay/Documents/data/Christchurch/points/points_row1_col1.laz".to_string());

    println!("Opening file: {}", file_path);
    
    let f = match File::open(&file_path) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Failed to open file: {}", e);
            return;
        }
    };

    let buf = BufReader::new(f);
    
    println!("Creating LasReader...");
    let mut las_reader = match LasReader::new(buf) {
        Ok(r) => {
            println!("LasReader created successfully");
            r
        }
        Err(e) => {
            eprintln!("Failed to create LasReader: {:?}", e);
            return;
        }
    };

    let header = las_reader.header();
    println!("LAS Header Info:");
    println!("  Version: {}.{}", header.version_major, header.version_minor);
    println!("  Point data format: {:?}", header.point_data_format);
    println!("  Number of VLRs: {}", header.number_of_vlrs);
    println!("  Offset to point data: {}", header.offset_to_point_data);
    println!("  Point count: {}", header.point_count());
    
    let f2 = match File::open(&file_path) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Failed to reopen file: {}", e);
            return;
        }
    };

    let buf2 = BufReader::new(f2);
    
    println!("\nCreating LazReader...");
    let mut reader = match LazReader::new(buf2) {
        Ok(r) => {
            println!("LazReader created successfully");
            r
        }
        Err(e) => {
            eprintln!("Failed to create LazReader: {:?}", e);
            return;
        }
    };

    println!("Reading points...");
    match reader.read_all() {
        Ok(points) => {
            println!("Success! Read {} points", points.len());
            if let Some(p) = points.first() {
                println!("First point: x={}, y={}, z={}", p.x, p.y, p.z);
            }
        }
        Err(e) => {
            eprintln!("Error reading points: {:?}", e);
            println!("Error chain: {}", format_error(&e));
        }
    }
}

fn format_error(e: &wblidar::Error) -> String {
    format!("{:?}", e)
}
