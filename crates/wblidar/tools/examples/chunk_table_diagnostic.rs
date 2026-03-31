use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use wblidar::laz::{parse_laszip_vlr, LaszipCompressorType};
use wblidar::las::LasReader;

fn read_u32_le<R: Read>(r: &mut R) -> std::io::Result<u32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}

fn read_i64_le<R: Read>(r: &mut R) -> std::io::Result<i64> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b)?;
    Ok(i64::from_le_bytes(b))
}

fn main() {
    let file_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/Users/johnlindsay/Documents/data/Christchurch/points/points_row1_col1.laz".to_string());

    println!("=== LAZ Chunk Table Diagnostic ===");
    println!("File: {}\n", file_path);

    let f = match File::open(&file_path) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Failed to open file: {}", e);
            return;
        }
    };

    let buf = BufReader::new(f);
    let file_len = std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);
    println!("File size: {} bytes\n", file_len);

    // Read LAS header
    let mut las = match LasReader::new(buf) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to read LAS header: {:?}", e);
            return;
        }
    };

    let header = las.header();
    println!("LAS Header:");
    println!("  Version: {}.{}", header.version_major, header.version_minor);
    println!("  Point data format: {:?}", header.point_data_format);
    println!("  Number of VLRs: {}", header.number_of_vlrs);
    println!("  Offset to point data: {} (0x{:X})", header.offset_to_point_data, header.offset_to_point_data);
    println!("  Point count: {}\n", header.point_count());
    
    let chunk_table_pos = header.offset_to_point_data as u64;

    // Check LASzip VLR
    let laszip_info = parse_laszip_vlr(las.vlrs());
    
    match laszip_info {
        Some(info) => {
            println!("LASzip VLR found:");
            println!("  Compressor: {:?}", info.compressor);
            println!("  Uses arithmetic coder: {}", info.uses_arithmetic_coder());
            println!("  Has Point14: {}", info.has_point14_item());
            match info.compressor {
                LaszipCompressorType::PointWise => println!("  Layout: PointWise (uncompressed)"),
                LaszipCompressorType::PointWiseChunked => println!("  Layout: PointWiseChunked"),
                LaszipCompressorType::LayeredChunked => {
                    println!("  Layout: LayeredChunked (this is Point14 arithmetic)");
                }
                _ => println!("  Layout: Other"),
            }
            println!();
        }
        None => {
            println!("No LASzip VLR found - file uses standard LAS compression\n");
            return;
        }
    }

    // Try to read chunk table pointer
    let inner = las.inner_mut();
    
    println!("Attempting to read chunk table from offset {}:", chunk_table_pos);
    
    if let Ok(_) = inner.seek(SeekFrom::Start(chunk_table_pos)) {
        match (read_u32_le(inner), read_u32_le(inner)) {
            (Ok(v), Ok(c)) => {
                println!("  Found at offset {}: version={}, chunk_count={}", chunk_table_pos, v, c);
                if v == 3 {
                    println!("  ✓ Looks like wb-native format");
                }
            }
            _ => println!("  ✗ Failed to read chunk table header"),
        }
    }

    // Try to read as standard LASzip chunk table pointer
    if let Ok(_) = inner.seek(SeekFrom::Start(chunk_table_pos)) {
        match read_i64_le(inner) {
            Ok(ptr) => {
                println!("\nAttempting standard LASzip chunk table pointer:");
                println!("  Pointer at offset {}: {} (0x{:X})", chunk_table_pos, ptr, ptr as u64);
                
                if ptr > 0 && (ptr as u64) < file_len {
                    println!("  ✓ Pointer is within file bounds");
                    
                    // Try to read chunk table header
                    if let Ok(_) = inner.seek(SeekFrom::Start(ptr as u64)) {
                        match (read_u32_le(inner), read_u32_le(inner)) {
                            (Ok(version), Ok(chunk_count)) => {
                                println!("  Chunk table header at {}: version={}, chunk_count={}", ptr, version, chunk_count);
                                
                                // Calculate expected chunk table size
                                let expected_entry_size = if chunk_count > 0 {
                                    16  // Assuming 16 bytes per entry (u64 offset + u32 count + u32 byte_count)
                                } else {
                                    0
                                };
                                let total_size = 8 + (chunk_count as u64) * expected_entry_size as u64;
                                println!("  Expected chunk table size: {} bytes", total_size);
                                println!("  Ends at: {} (file_len={})", ptr as u64 + total_size, file_len);
                                
                                if ptr as u64 + total_size > file_len {
                                    println!("  ⚠ WARNING: Chunk table extends beyond file! EOF will occur when reading chunks.");
                                }
                            }
                            _ => println!("  ✗ Failed to read chunk table header"),
                        }
                    }
                } else {
                    println!("  ✗ Pointer is outside file bounds");
                }
            }
            Err(e) => println!("✗ Failed to read chunk table pointer: {}", e),
        }
    }

    println!("\n=== Diagnostic Complete ===");
}
