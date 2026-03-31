use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};

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

    let file_size = f.metadata().map(|m| m.len()).unwrap_or(0);
    println!("File size: {} bytes", file_size);

    let mut buf = BufReader::new(f);
    
    // Try to read LAS header (first 375 bytes)
    println!("Reading LAS header (375 bytes)...");
    let mut header_bytes = vec![0u8; 375];
    match buf.read_exact(&mut header_bytes) {
        Ok(_) => println!("Header read successfully"),
        Err(e) => {
            eprintln!("Failed to read header: {}", e);
            return;
        }
    }

    // Check magic bytes
    if header_bytes[0..4] == [b'L', b'A', b'S', b'F'] {
        println!("Magic bytes OK: LASF");
    } else {
        println!("WARNING: Invalid magic bytes: {:?}", &header_bytes[0..4]);
    }

    // Check version
    let version_minor = header_bytes[24];
    let version_major = header_bytes[25];
    println!("LAS version: {}.{}", version_major, version_minor);

    // Check header size
    let header_size = u16::from_le_bytes([header_bytes[94], header_bytes[95]]);
    println!("Header size: {} bytes", header_size);

    // Check number of VLRs
    let num_vlrs = u32::from_le_bytes([header_bytes[131], header_bytes[132], header_bytes[133], header_bytes[134]]);
    println!("Number of VLRs: {}", num_vlrs);

    // Check offset to point data
    let offset_to_points = u64::from_le_bytes([
        header_bytes[96], header_bytes[97], header_bytes[98], header_bytes[99],
        header_bytes[100], header_bytes[101], header_bytes[102], header_bytes[103],
    ]);
    println!("Offset to point data: {}", offset_to_points);

    // Check point count
    let point_count = u64::from_le_bytes([
        header_bytes[130-130], header_bytes[131-130], header_bytes[132-130], header_bytes[133-130],
        header_bytes[134-130], header_bytes[135-130], header_bytes[136-130], header_bytes[137-130],
    ]);
    println!("Point count: {}", point_count);

    // Try to read VLRs
    println!("\nReading VLRs...");
    buf.seek(SeekFrom::Start(u64::from(header_size))).ok();
    for i in 0..num_vlrs.min(10) {
        let mut vlr_header = vec![0u8; 54]; // VLR header is 54 bytes
        match buf.read_exact(&mut vlr_header) {
            Ok(_) => {
                let user_id = String::from_utf8_lossy(&vlr_header[0..16]);
                let record_id = u16::from_le_bytes([vlr_header[16], vlr_header[17]]);
                let record_len = u16::from_le_bytes([vlr_header[18], vlr_header[19]]);
                println!("  VLR {}: user_id='{}', record_id={}, length={}", i, user_id.trim_end_matches('\0'), record_id, record_len);
                
                // Skip past the VLR data
                buf.seek(SeekFrom::Current(record_len as i64)).ok();
            }
            Err(e) => {
                eprintln!("Failed to read VLR {}: {}", i, e);
                break;
            }
        }
    }

    println!("\nFile structure appears valid");
}
