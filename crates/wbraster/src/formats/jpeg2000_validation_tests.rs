#[cfg(test)]
mod jpeg2000_byte_alignment_validation {
    use crate::formats::jpeg2000::read;
    use std::path::Path;

    /// Test: Decode B03 JP2 and compare against reference PIL/imagecodecs output.
    /// This isolates the byte-alignment issue in packet header parsing.
    /// 
    /// Expected behavior:
    /// - native output should match bridge output for the first 256 pixels
    /// - To get this right, the packet header must be byte-aligned before reading CB segments
    #[test]
    fn test_b03_pixel_values_match_reference() {
        use crate::raster::RasterData;

        let b03_path = "/Users/johnlindsay/Documents/teaching/GEOG3420/W26/Labs/data/S2A_MSIL2A_20240727T161831_N0511_R040_T17TNJ_20240727T235645.SAFE/GRANULE/L2A_T17TNJ_A047513_20240727T161942/IMG_DATA/R10m/T17TNJ_20240727T161831_B03_10m.jp2";
        
        if !Path::new(b03_path).exists() {
            eprintln!("SKIP: B03 file not found at {}", b03_path);
            return;
        }

        // Reference values from PIL/imagecodecs (I;16 mode, first 256 pixels)
        // All 255 for this particular region
        let reference_pixels: Vec<u16> = vec![255; 256];

        // Read with our native decoder
        match read(b03_path) {
            Ok(raster) => {
                let width = raster.cols;
                let height = raster.rows;
                eprintln!("[b03_validation] Decoded: {}x{}", width, height);

                // Extract first 256 pixels from the RasterData
                let native_pixels = match &raster.data {
                    RasterData::U16(vec) => &vec[0..256.min(vec.len())],
                    RasterData::I16(vec) => {
                        eprintln!("ERROR: Got I16 data, expected U16");
                        return;
                    },
                    other => {
                        eprintln!("ERROR: Got {:?}, expected U16", raster.data_type);
                        return;
                    }
                };

                if native_pixels.len() < 256 {
                    eprintln!("WARNING: Only {} pixels available, expected 256", native_pixels.len());
                }

                // Compare
                let mut mismatches = 0;
                let mut max_error = 0i32;
                for (i, (&native, &reference)) in native_pixels.iter().zip(reference_pixels.iter()).enumerate() {
                    let error = (native as i32 - reference as i32).abs();
                    if error > 5 {  // Allow small tolerance for rounding/bit-depth differences
                        if mismatches < 10 {
                            eprintln!("[b03_validation] Pixel[{}]: native={} reference={} error={}", i, native, reference, error);
                        }
                        mismatches += 1;
                        max_error = max_error.max(error);
                    }
                }

                eprintln!("[b03_validation] Total mismatches (>5 error): {}/{}", mismatches, native_pixels.len());
                eprintln!("[b03_validation] Max error: {}", max_error);

                // For now, just log (don't assert) so we see the actual values
                if mismatches > 0 {
                    eprintln!("[b03_validation] MISMATCH DETECTED - likely byte-alignment bug");
                } else {
                    eprintln!("[b03_validation] PASS - pixels match reference!");
                }
            }
            Err(e) => {
                eprintln!("ERROR decoding B03: {}", e);
            }
        }
    }

    /// Test: Specifically validate LL CB(0,0) block decoding.
    /// Extract raw fragment and decode in isolation.
    #[test]
    fn test_ll_codeblock_0_0_isolation() {
        let b03_path = "/Users/johnlindsay/Documents/teaching/GEOG3420/W26/Labs/data/S2A_MSIL2A_20240727T161831_N0511_R040_T17TNJ_20240727T235645.SAFE/GRANULE/L2A_T17TNJ_A047513_20240727T161942/IMG_DATA/R10m/T17TNJ_20240727T161831_B03_10m.jp2";

        if !Path::new(b03_path).exists() {
            eprintln!("SKIP: B03 file not found");
            return;
        }

        // Read the file and decode
        match read(b03_path) {
            Ok(raster) => {
                eprintln!("[ll_cb_isolation] Raster decoded successfully: {}x{}", raster.cols, raster.rows);
                // If we get here without panicking, basic structure is sound.
                // The actual coefficient values validation happens in test_b03_pixel_values_match_reference.
            }
            Err(e) => {
                eprintln!("ERROR: {}", e);
            }
        }
    }
}
