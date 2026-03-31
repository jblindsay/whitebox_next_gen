use std::env;
use std::fs::File;
use std::io::BufReader;

use wblidar::io::PointReader;
use wblidar::las::LasReader;
use wblidar::laz::LazReader;

fn main() {
    let path = env::args().nth(1).expect("path required");
    
    // Diagnose file size and header
    let file_meta = metadata(&path).expect("metadata failed");
    let file_len = file_meta.len();
    println!("FILE_LEN\t{}", file_len);
    
    let file = File::open(&path).expect("open failed");
    let mut reader = BufReader::new(file);
    
    // Try to read first 512 bytes to peek at header
    let mut header_buf = [0u8; 512];
    if let Ok(n) = reader.read(&mut header_buf) {
        println!("READ_HEADER\t{} bytes", n);
        
        if n >= 96 {
            let offset_to_point_data = u32::from_le_bytes([
                header_buf[96],
                header_buf[97],
                header_buf[98],
                header_buf[99],
            ]);
            println!("OFFSET_TO_POINT_DATA\t{}", offset_to_point_data);
        }
    }
    
    // Try full open via normalized PointReader (frontend)
    println!("\n=== PointReader (frontend) ===");
    let file = File::open(&path).expect("open failed");
    let buf_reader = BufReader::new(file);
    
    match wblidar::io::WbReader::new(buf_reader) {
        Ok(mut reader) => {
            println!("WBREADER_INIT\tOK");
            match reader.read_all() {
                Ok(points) => println!("WBREADER_READ\tOK\t{} points", points.len()),
                Err(e) => {
                    println!("WBREADER_READ_ERR\t{}", e);
                    if e.to_string().contains("channel") {
                        println!("ERROR_TYPE\tmulti-channel-not-implemented");
                    } else if e.to_string().contains("arithmetic") {
                        println!("ERROR_TYPE\tarithmetic-not-implemented");
                    }
                }
            }
        }
        Err(e) => {
            println!("WBREADER_INIT_ERR\t{}", e);
        }
    }
    
    // Try direct LAZ reader (no normalization)
    println!("\n=== LazReader (direct) ===");
    let file2 = File::open(&path).expect("open failed");
    let buf_reader2 = BufReader::new(file2);
    
    match LasReader::new(buf_reader2) {
        Ok(las_reader) => {
            let hdr = las_reader.header();
            println!("LASREADER_INIT\tOK\tversion={}.{}", hdr.version_major, hdr.version_minor);
            
            let file3 = File::open(&path).expect("open failed");
            match LazReader::new(BufReader::new(file3)) {
                Ok(mut r) => {
                    println!("LAZREADER_INIT\tOK");
                    match r.read_all() {
                        Ok(points) => println!("LAZREADER_READ\tOK\t{} points", points.len()),
                        Err(e) => {
                            println!("LAZREADER_READ_ERR\t{}", e);
                            if e.to_string().contains("channel") {
                                println!("ERROR_TYPE\tmulti-channel-not-implemented");
                            } else if e.to_string().contains("arithmetic") {
                                println!("ERROR_TYPE\tarithmetic-not-implemented");
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("LAZREADER_INIT_ERR\t{}", e);
                    if e.to_string().contains("channel") {
                        println!("ERROR_TYPE\tmulti-channel-not-implemented");
                    } else if e.to_string().contains("arithmetic") {
                        println!("ERROR_TYPE\tarithmetic-not-implemented");
                    }
                }
            }
        }
        Err(e) => {
            println!("LASREADER_INIT_ERR\t{}", e);
        }
    }
}
