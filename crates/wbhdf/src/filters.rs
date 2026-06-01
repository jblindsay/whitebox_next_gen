use crate::error::{WbhdfError, WbhdfResult};
use flate2::read::GzDecoder;
use flate2::read::ZlibDecoder;
use std::io::Read;

/// Decompresses GZIP payload bytes.
pub fn decompress_gzip(compressed: &[u8]) -> WbhdfResult<Vec<u8>> {
    let mut decoder = GzDecoder::new(compressed);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).map_err(|err| {
        WbhdfError::UnsupportedFilter(format!("GZIP decode failed: {err}"))
    })?;
    Ok(decompressed)
}

/// Decompresses zlib-wrapped DEFLATE payload bytes.
///
/// HDF5 deflate-filter chunks are zlib streams in practice, not gzip wrappers.
pub fn decompress_zlib(compressed: &[u8]) -> WbhdfResult<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(compressed);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).map_err(|err| {
        WbhdfError::UnsupportedFilter(format!("zlib decode failed: {err}"))
    })?;
    Ok(decompressed)
}

#[cfg(test)]
mod tests {
    use super::{decompress_gzip, decompress_zlib};
    use flate2::write::GzEncoder;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    #[test]
    fn decompresses_gzip_payload() {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"atl08-test-payload").unwrap();
        let compressed = encoder.finish().unwrap();

        let decompressed = decompress_gzip(&compressed).expect("gzip payload should decode");
        assert_eq!(decompressed, b"atl08-test-payload");
    }

    #[test]
    fn reports_invalid_gzip_payload() {
        let err = decompress_gzip(b"not-gzip").expect_err("invalid gzip payload should fail");
        let msg = format!("{err}");
        assert!(msg.contains("GZIP decode failed"));
    }

    #[test]
    fn decompresses_zlib_payload() {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"atl08-zlib-chunk").unwrap();
        let compressed = encoder.finish().unwrap();

        let decompressed = decompress_zlib(&compressed).expect("zlib payload should decode");
        assert_eq!(decompressed, b"atl08-zlib-chunk");
    }

    #[test]
    fn reports_invalid_zlib_payload() {
        let err = decompress_zlib(b"not-zlib").expect_err("invalid zlib payload should fail");
        let msg = format!("{err}");
        assert!(msg.contains("zlib decode failed"));
    }
}
