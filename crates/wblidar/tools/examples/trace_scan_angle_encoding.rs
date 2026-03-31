/// Trace scan-angle encoding and decoding for chunk 51 tail points to identify context/state bug.
use std::io::Cursor;
use wblidar::laz::{
    arithmetic_decoder::ArithmeticDecoder, arithmetic_encoder::ArithmeticEncoder,
    integer_codec::{IntegerCompressor, IntegerDecompressor},
};

fn main() -> std::io::Result<()> {
    // Scan values and change flags from chunk 51's final 10 points.
    let scan_values: Vec<i16> = vec![
        -1599, -1567, -1571, -1575, -1579, -1583, // Correct reference (indices 0-5, actual 970-975 in chunk)
    ];
    let scan_changes: Vec<bool> = vec![
        false, true, true, true, true, true, // False for seed, then changes for next 5 points
    ];
    let gps_changes: Vec<bool> = vec![
        false, true, true, true, true, true, // Same as scan changes for these points
    ];

    println!("=== Scan-Angle Encoding Trace ===");
    println!("Input scan values: {:?}", scan_values);
    println!("Input scan changes: {:?}", scan_changes);
    println!("Input gps changes: {:?}", gps_changes);
    println!();

    // Encode using our implementation
    let mut encoded = Vec::<u8>::new();
    {
        let mut cursor = Cursor::new(&mut encoded);
        let mut enc = ArithmeticEncoder::new(&mut cursor);
        let mut ic_enc = IntegerCompressor::new(16, 2, 8, 0);

        let mut last_scan = scan_values[0];

        println!("Compressor k_array initial: (new compressor, k not yet set)");

        // Skip seed (index 0), process changes for indices 1-5
        for i in 1..scan_values.len() {
            let changed = scan_changes[i];
            let next_scan = scan_values[i];

            if changed {
                println!(
                    "Point {}: last_scan={}, next_scan={}, gps_change={}, context={}",
                    i - 1,
                    last_scan,
                    next_scan,
                    gps_changes[i],
                    if gps_changes[i] { 1 } else { 0 }
                );

                ic_enc
                    .compress(
                        &mut enc,
                        last_scan as i32,
                        next_scan as i32,
                        if gps_changes[i] { 1 } else { 0 },
                    )
                    .unwrap();
            }
            last_scan = next_scan;
        }

        enc.done().unwrap();
    }

    println!();
    println!("Encoded {} bytes", encoded.len());
    println!("First 48 bytes (hex): {:?}", &encoded[..std::cmp::min(48, encoded.len())]);
    println!();

    // Decode and compare
    println!("=== Decoding Trace ===");
    let mut decoded = Vec::<i16>::new();
    {
        let cursor = Cursor::new(&encoded);
        let mut dec = ArithmeticDecoder::new(cursor);
        dec.read_init_bytes().unwrap(); // CRITICAL: Initialize decoder state from first 4 bytes!
        let mut ic = IntegerDecompressor::new(16, 2, 8, 0);

        let mut last_scan = scan_values[0];
        decoded.push(last_scan);

        // Reconstruct points 1-5
        for i in 1..scan_values.len() {
            let changed = scan_changes[i];

            if changed {
                let next_scan_val = ic
                    .decompress(
                        &mut dec,
                        last_scan as i32,
                        if gps_changes[i] { 1 } else { 0 },
                    )
                    .unwrap() as i16;

                println!(
                    "Point {}: decoded_scan={}, expected={}, match={}",
                    i - 1,
                    next_scan_val,
                    scan_values[i],
                    next_scan_val == scan_values[i]
                );

                decoded.push(next_scan_val);
                last_scan = next_scan_val;
            } else {
                decoded.push(last_scan);
            }
        }
    }

    println!();
    println!("Decoded scan values: {:?}", decoded);
    println!("Expected scan values: {:?}", scan_values);
    println!();

    // Compare
    let mut all_match = true;
    for (i, (&decoded_val, &expected_val)) in decoded.iter().zip(scan_values.iter()).enumerate() {
        if decoded_val != expected_val {
            println!("MISMATCH at index {}: {} != {}", i, decoded_val, expected_val);
            all_match = false;
        }
    }

    if all_match {
        println!("✓ All scan values match!");
    } else {
        println!("✗ Scan-angle round-trip failed");
    }

    Ok(())
}
