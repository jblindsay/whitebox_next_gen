/// Test if periodic model reset fixes scan angle encoding (minimal reproduction).
use std::io::Cursor;
use wblidar::laz::{
    arithmetic_decoder::ArithmeticDecoder, arithmetic_encoder::ArithmeticEncoder,
    integer_codec::{IntegerCompressor, IntegerDecompressor},
};

fn main() -> std::io::Result<()> {
   // Reproduce chunk 51 pattern: 974 values with constant -4 deltas at end
    let mut encoded_no_reset = Vec::<u8>::new();
    let mut encoded_with_reset = Vec::<u8>::new();

    // Encode without reset
    {
        let cursor = Cursor::new(&mut encoded_no_reset);
        let mut enc = ArithmeticEncoder::new(cursor);
        let mut ic = IntegerCompressor::new(16, 2, 8, 0);

        let mut last = 0i32;
        // Encode 1000 values
        for i in 0..1000 {
            let value = if i < 970 {
                last + ((i as i32 * 7) % 100 - 50) // Varied deltas
            } else {
                last - 4 // Constant pattern like points 971-974
            };
            ic.compress(&mut enc, last, value, (i % 2) as u32)?;
            last = value;
        }
        enc.done()?;
    }

    // Encode with periodic model reset (every 128 symbols)
    {
        let cursor = Cursor::new(&mut encoded_with_reset);
        let mut enc = ArithmeticEncoder::new(cursor);
        let mut ic = IntegerCompressor::new(16, 2, 8, 0);

        let mut last = 0i32;
        let mut symbol_count = 0;
        for i in 0..1000 {
            let value = if i < 970 {
                last + ((i as i32 * 7) % 100 - 50)
            } else {
                last - 4
            };
            ic.compress(&mut enc, last, value, (i % 2) as u32)?;
            last = value;
            symbol_count += 1;

            // Reset model every 128 symbols (LAStools pattern)
            if symbol_count >= 128 {
                ic = IntegerCompressor::new(16, 2, 8, 0);
                symbol_count = 0;
            }
        }
        enc.done()?;
    }

    println!("Encoded without reset: {} bytes", encoded_no_reset.len());
    println!("Encoded with reset: {} bytes", encoded_with_reset.len());
    println!("Difference: {} bytes", (encoded_no_reset.len() as i32 - encoded_with_reset.len() as i32).abs());

    // Decode both and compare
    println!("\n=== Decoding comparison ===");
    
    let mut last_no_reset = 0i32;
    let mut mismatches_no_reset = 0;
    {
        let cursor = Cursor::new(&encoded_no_reset[..]);
        let mut dec = ArithmeticDecoder::new(cursor);
        dec.read_init_bytes()?;
        let mut ic = IntegerDecompressor::new(16, 2, 8, 0);

        for i in 0..1000 {
            let decoded = ic.decompress(&mut dec, last_no_reset, (i % 2) as u32)?;
            let expected_delta = if i < 970 {
                ((i as i32 * 7) % 100 - 50)
            } else {
                -4
            };
            let actual_delta = decoded - last_no_reset;
            if i >= 968 && i <= 975 {
                println!("No-reset: point[{}] decoded_delta={}", i, actual_delta);
            }
            if actual_delta != expected_delta && i >= 970 {
                mismatches_no_reset += 1;
            }
            last_no_reset = decoded;
        }
    }

    let mut last_with_reset = 0i32;
    let mut mismatches_with_reset = 0;
    let mut symbol_count = 0;
    {
        let cursor = Cursor::new(&encoded_with_reset[..]);
        let mut dec = ArithmeticDecoder::new(cursor);
        dec.read_init_bytes()?;
        let mut ic = IntegerDecompressor::new(16, 2, 8, 0);

        for i in 0..1000 {
            let decoded = ic.decompress(&mut dec, last_with_reset, (i % 2) as u32)?;
            let expected_delta = if i < 970 {
                ((i as i32 * 7) % 100 - 50)
            } else {
                -4
            };
            let actual_delta = decoded - last_with_reset;
            if i >= 968 && i <= 975 {
                println!("With-reset: point[{}] decoded_delta={}", i, actual_delta);
            }
            if actual_delta != expected_delta && i >= 970 {
                mismatches_with_reset +=1;
            }
            last_with_reset = decoded;
            symbol_count += 1;

            if symbol_count >= 128 {
                ic = IntegerDecompressor::new(16, 2, 8, 0);
                symbol_count = 0;
            }
        }
    }

    println!("\nMismatches (points 970-999):");
    println!("Without reset: {}", mismatches_no_reset);
    println!("With reset: {}", mismatches_with_reset);

    Ok(())
}
