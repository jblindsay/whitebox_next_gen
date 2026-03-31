//! Compression and decompression implementations.
//!
//! Supported codecs:
//! - **None** – raw bytes (no-op)
//! - **JPEG** – lossy JPEG (8-bit grayscale/RGB chunks)
//! - **WebP** – lossy WebP (8-bit RGB/RGBA chunks)
//! - **JPEG-XL** – lossy/lossless JPEG-XL (8-bit gray/RGB/RGBA chunks)
//! - **LZW** – via the `weezl` crate (TIFF variant with MSB-first bit order)
//! - **Deflate/ZIP** – via the `flate2` crate
//! - **PackBits** – pure-Rust implementation
//!
//! Each codec exposes `compress(input) -> Result<Vec<u8>>` and
//! `decompress(input, expected_len) -> Result<Vec<u8>>` functions.

#![allow(dead_code)]

use super::error::{GeoTiffError, Result};
use super::tags::Compression;
use std::io::Cursor;

// ── Public dispatch ──────────────────────────────────────────────────────────

/// Compress `input` bytes using the given codec.
pub fn compress(codec: Compression, input: &[u8]) -> Result<Vec<u8>> {
    match codec {
        Compression::None => Ok(input.to_vec()),
        Compression::Lzw => lzw::compress(input),
        Compression::Deflate => deflate::compress(input),
        Compression::PackBits => packbits::compress(input),
        other => Err(GeoTiffError::UnsupportedCompression(other.tag_value())),
    }
}

/// Compress with WebP for one strip/tile chunk.
pub fn compress_webp(
    input: &[u8],
    width: u32,
    height: u32,
    samples_per_pixel: usize,
    quality: f32,
) -> Result<Vec<u8>> {
    use webp::Encoder;

    let encoded = match samples_per_pixel {
        3 => Encoder::from_rgb(input, width, height).encode(quality).to_vec(),
        4 => Encoder::from_rgba(input, width, height).encode(quality).to_vec(),
        _ => {
            return Err(GeoTiffError::CompressionError {
                codec: "WebP",
                message: format!("unsupported samples_per_pixel={}, expected 3 (RGB) or 4 (RGBA)", samples_per_pixel),
            })
        }
    };

    Ok(encoded)
}

/// Decompress one WebP strip/tile chunk.
pub fn decompress_webp(input: &[u8], expected_len: usize) -> Result<Vec<u8>> {
    use webp::Decoder;

    let decoder = Decoder::new(input);
    let img = decoder.decode().ok_or_else(|| GeoTiffError::CompressionError {
        codec: "WebP",
        message: "failed to decode WebP payload".into(),
    })?;

    let decoded = img.to_vec();
    if expected_len == 0 {
        return Ok(decoded);
    }

    if decoded.len() == expected_len {
        return Ok(decoded);
    }

    let pixel_count = img.width() as usize * img.height() as usize;
    if pixel_count > 0 && decoded.len() == pixel_count * 3 {
        if expected_len == pixel_count * 3 {
            return Ok(decoded);
        }
        if expected_len == pixel_count * 4 {
            let mut out = Vec::with_capacity(expected_len);
            for px in decoded.chunks_exact(3) {
                out.extend_from_slice(px);
                out.push(255);
            }
            return Ok(out);
        }
        if expected_len == pixel_count {
            let mut out = Vec::with_capacity(expected_len);
            for px in decoded.chunks_exact(3) {
                out.push(px[0]);
            }
            return Ok(out);
        }
    }

    if pixel_count > 0 && decoded.len() == pixel_count * 4 {
        if expected_len == pixel_count * 4 {
            return Ok(decoded);
        }
        if expected_len == pixel_count * 3 {
            let mut out = Vec::with_capacity(expected_len);
            for px in decoded.chunks_exact(4) {
                out.extend_from_slice(&px[..3]);
            }
            return Ok(out);
        }
        if expected_len == pixel_count {
            let mut out = Vec::with_capacity(expected_len);
            for px in decoded.chunks_exact(4) {
                out.push(px[0]);
            }
            return Ok(out);
        }
    }

    Err(GeoTiffError::CompressionError {
        codec: "WebP",
        message: format!(
            "decoded length mismatch: got {}, expected {}",
            decoded.len(),
            expected_len
        ),
    })
}

/// Compress with JPEG for one strip/tile chunk.
pub fn compress_jpeg(
    input: &[u8],
    width: u16,
    height: u16,
    samples_per_pixel: usize,
    quality: u8,
) -> Result<Vec<u8>> {
    use jpeg_encoder::{ColorType, Encoder};

    let color = match samples_per_pixel {
        1 => ColorType::Luma,
        3 => ColorType::Rgb,
        _ => {
            return Err(GeoTiffError::CompressionError {
                codec: "JPEG",
                message: format!("unsupported samples_per_pixel={}, expected 1 or 3", samples_per_pixel),
            })
        }
    };

    let mut out = Vec::new();
    let enc = Encoder::new(&mut out, quality);
    enc.encode(input, width, height, color)
        .map_err(|e| GeoTiffError::CompressionError {
            codec: "JPEG",
            message: e.to_string(),
        })?;
    Ok(out)
}

/// Compress with JPEG-XL for one strip/tile chunk.
pub fn compress_jpegxl(
    input: &[u8],
    width: u32,
    height: u32,
    samples_per_pixel: usize,
    quality: u8,
) -> Result<Vec<u8>> {
    use zune_core::bit_depth::BitDepth;
    use zune_core::colorspace::ColorSpace;
    use zune_core::options::EncoderOptions;
    use zune_jpegxl::JxlSimpleEncoder;

    let pixel_count = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| GeoTiffError::CompressionError {
            codec: "JPEG-XL",
            message: "image dimensions overflow".into(),
        })?;

    let (color_space, expected_len) = match samples_per_pixel {
        1 => (ColorSpace::Luma, pixel_count),
        3 => (ColorSpace::RGB, pixel_count * 3),
        4 => (ColorSpace::RGBA, pixel_count * 4),
        _ => {
            return Err(GeoTiffError::CompressionError {
                codec: "JPEG-XL",
                message: format!(
                    "unsupported samples_per_pixel={}, expected 1, 3, or 4",
                    samples_per_pixel
                ),
            })
        }
    };

    if input.len() != expected_len {
        return Err(GeoTiffError::CompressionError {
            codec: "JPEG-XL",
            message: format!("invalid input length {}, expected {}", input.len(), expected_len),
        });
    }

    let effort = ((quality as u16 * 9 + 99) / 100) as u8;
    let options = EncoderOptions::new(width as usize, height as usize, color_space, BitDepth::Eight)
        .set_quality(quality.clamp(1, 100))
        .set_effort(effort.clamp(1, 9));
    let encoder = JxlSimpleEncoder::new(input, options);
    let mut out = Vec::new();
    encoder
        .encode(&mut out)
        .map_err(|e| GeoTiffError::CompressionError {
            codec: "JPEG-XL",
            message: format!("{e:?}"),
        })?;
    Ok(out)
}

/// Decompress one JPEG-XL strip/tile chunk.
pub fn decompress_jpegxl(input: &[u8], expected_len: usize) -> Result<Vec<u8>> {
    use jxl_oxide::JxlImage;

    let image = JxlImage::builder()
        .read(Cursor::new(input))
        .map_err(|e| GeoTiffError::CompressionError {
            codec: "JPEG-XL",
            message: format!("{e:?}"),
        })?;

    let render = image.render_frame(0).map_err(|e| GeoTiffError::CompressionError {
        codec: "JPEG-XL",
        message: format!("{e:?}"),
    })?;
    let fb = render.image_all_channels();
    let pixel_count = fb.width().saturating_mul(fb.height());
    let channels = fb.channels();
    let src = fb.buf();

    if pixel_count == 0 || src.len() != pixel_count * channels {
        return Err(GeoTiffError::CompressionError {
            codec: "JPEG-XL",
            message: format!(
                "decoded framebuffer mismatch: samples={}, expected {}",
                src.len(),
                pixel_count * channels
            ),
        });
    }

    let mut rgba = Vec::with_capacity(pixel_count * 4);
    for i in 0..pixel_count {
        let base = i * channels;
        let c0 = src[base].round().clamp(0.0, 255.0) as u8;
        let c1 = if channels > 1 {
            src[base + 1].round().clamp(0.0, 255.0) as u8
        } else {
            c0
        };
        let c2 = if channels > 2 {
            src[base + 2].round().clamp(0.0, 255.0) as u8
        } else {
            c0
        };
        let a = if channels > 3 {
            src[base + 3].round().clamp(0.0, 255.0) as u8
        } else {
            255
        };
        rgba.extend_from_slice(&[c0, c1, c2, a]);
    }

    if expected_len == 0 || expected_len == rgba.len() {
        return Ok(rgba);
    }

    if expected_len == pixel_count * 3 {
        let mut out = Vec::with_capacity(expected_len);
        for px in rgba.chunks_exact(4) {
            out.extend_from_slice(&px[..3]);
        }
        return Ok(out);
    }

    if expected_len == pixel_count {
        let mut out = Vec::with_capacity(expected_len);
        for px in rgba.chunks_exact(4) {
            out.push(px[0]);
        }
        return Ok(out);
    }

    Err(GeoTiffError::CompressionError {
        codec: "JPEG-XL",
        message: format!(
            "decoded length mismatch: got {}, expected {}",
            rgba.len(),
            expected_len
        ),
    })
}

/// Decompress one JPEG strip/tile chunk.
pub fn decompress_jpeg(input: &[u8], expected_len: usize) -> Result<Vec<u8>> {
    use jpeg_decoder::Decoder;

    let mut decoder = Decoder::new(Cursor::new(input));
    let mut output = decoder
        .decode()
        .map_err(|e| GeoTiffError::CompressionError {
            codec: "JPEG",
            message: e.to_string(),
        })?;

    if expected_len > 0 {
        if output.len() < expected_len {
            return Err(GeoTiffError::CompressionError {
                codec: "JPEG",
                message: format!(
                    "decoded chunk shorter than expected: {} < {}",
                    output.len(),
                    expected_len
                ),
            });
        }
        if output.len() > expected_len {
            output.truncate(expected_len);
        }
    }

    Ok(output)
}

/// Decompress `input` bytes using the given codec.
///
/// `expected_len` is used as a hint for buffer pre-allocation and, for
/// PackBits, as a stopping criterion.
pub fn decompress(codec: Compression, input: &[u8], expected_len: usize) -> Result<Vec<u8>> {
    match codec {
        Compression::None => Ok(input.to_vec()),
        Compression::Lzw => lzw::decompress(input, expected_len),
        Compression::Deflate => deflate::decompress(input, expected_len),
        Compression::PackBits => packbits::decompress(input, expected_len),
        Compression::Jpeg | Compression::OldJpeg => decompress_jpeg(input, expected_len),
        Compression::WebP => decompress_webp(input, expected_len),
        Compression::JpegXl => decompress_jpegxl(input, expected_len),
        other => Err(GeoTiffError::UnsupportedCompression(other.tag_value())),
    }
}

// ── LZW ──────────────────────────────────────────────────────────────────────

mod lzw {
    use super::*;
    use weezl::BitOrder;

    /// TIFF uses MSB-first bit order and a 8-bit minimum code size.
    const BIT_ORDER: BitOrder = BitOrder::Msb;
    const MIN_CODE_SIZE: u8 = 8;

    /// Compress bytes with TIFF PackBits run-length encoding.
    pub fn compress(input: &[u8]) -> Result<Vec<u8>> {
        let mut encoder = weezl::encode::Encoder::with_tiff_size_switch(BIT_ORDER, MIN_CODE_SIZE);

        encoder.encode(input).map_err(|e| GeoTiffError::CompressionError {
                codec: "LZW",
                message: e.to_string(),
            })
    }

    /// Decompress PackBits bytes, stopping when `expected_len` output bytes are produced.
    pub fn decompress(input: &[u8], expected_len: usize) -> Result<Vec<u8>> {
        let mut decoder = weezl::decode::Decoder::with_tiff_size_switch(BIT_ORDER, MIN_CODE_SIZE);

        let mut output = decoder.decode(input).map_err(|e| GeoTiffError::CompressionError {
                codec: "LZW",
                message: e.to_string(),
            })?;
        if expected_len > 0 && output.len() > expected_len {
            output.truncate(expected_len);
        }
        Ok(output)
    }
}

// ── Deflate ───────────────────────────────────────────────────────────────────

mod deflate {
    use super::*;
    use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression as FlateLevel};
    use std::io::{Read, Write};

    pub fn compress(input: &[u8]) -> Result<Vec<u8>> {
        let mut encoder = ZlibEncoder::new(Vec::new(), FlateLevel::default());
        encoder.write_all(input).map_err(|e| GeoTiffError::CompressionError {
            codec: "Deflate",
            message: e.to_string(),
        })?;
        encoder.finish().map_err(|e| GeoTiffError::CompressionError {
            codec: "Deflate",
            message: e.to_string(),
        })
    }

    pub fn decompress(input: &[u8], expected_len: usize) -> Result<Vec<u8>> {
        let mut decoder = ZlibDecoder::new(input);
        let mut output = Vec::with_capacity(expected_len);
        decoder.read_to_end(&mut output).map_err(|e| GeoTiffError::CompressionError {
            codec: "Deflate",
            message: e.to_string(),
        })?;
        Ok(output)
    }
}

// ── PackBits ──────────────────────────────────────────────────────────────────

pub mod packbits {
    //! Pure-Rust PackBits (Apple/TIFF run-length encoding) codec.
    //!
    //! Format:
    //! - `n` in `[0, 127]`  → copy the next `n + 1` literal bytes.
    //! - `n` in `[-127, -1]` → repeat the next byte `1 - n` times.
    //! - `n == -128 (0x80)` → no-op (skip).

    use super::*;

    /// Compress a byte slice using PackBits run-length encoding.
    pub fn compress(input: &[u8]) -> Result<Vec<u8>> {
        let mut output = Vec::with_capacity(input.len() + input.len() / 128 + 1);
        let mut i = 0;

        while i < input.len() {
            // Check for a run
            let run_len = {
                let mut len = 1usize;
                while len < 128 && i + len < input.len() && input[i + len] == input[i] {
                    len += 1;
                }
                len
            };

            if run_len >= 2 {
                // Encode run
                output.push((1i8.wrapping_sub(run_len as i8)) as u8);
                output.push(input[i]);
                i += run_len;
            } else {
                // Gather literal bytes (stop before a run of ≥ 2)
                let lit_start = i;
                i += 1;
                while i < input.len() && i - lit_start < 128 {
                    let run = {
                        let mut len = 1usize;
                        while len < 3 && i + len < input.len() && input[i + len] == input[i] {
                            len += 1;
                        }
                        len
                    };
                    if run >= 2 {
                        break;
                    }
                    i += 1;
                }
                let lit_bytes = &input[lit_start..i];
                output.push((lit_bytes.len() - 1) as u8);
                output.extend_from_slice(lit_bytes);
            }
        }

        Ok(output)
    }

    /// Decompress PackBits-encoded bytes, using `expected_len` as output-size bound.
    pub fn decompress(input: &[u8], expected_len: usize) -> Result<Vec<u8>> {
        let mut output = Vec::with_capacity(expected_len);
        let mut i = 0;

        while i < input.len() && output.len() < expected_len {
            let header = input[i] as i8;
            i += 1;

            if header == -128 {
                // No-op
                continue;
            } else if header >= 0 {
                // Literal run: copy (header + 1) bytes
                let count = header as usize + 1;
                if i + count > input.len() {
                    return Err(GeoTiffError::CompressionError {
                        codec: "PackBits",
                        message: format!(
                            "Literal run extends beyond input (need {} bytes at offset {})",
                            count, i
                        ),
                    });
                }
                output.extend_from_slice(&input[i..i + count]);
                i += count;
            } else {
                // Replicate run: repeat next byte (1 - header) times
                let count = (1i32 - header as i32) as usize;
                if i >= input.len() {
                    return Err(GeoTiffError::CompressionError {
                        codec: "PackBits",
                        message: "Replicate run at end of input".into(),
                    });
                }
                let byte = input[i];
                i += 1;
                for _ in 0..count {
                    output.push(byte);
                }
            }
        }

        Ok(output)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn roundtrip_literal() {
            let data: Vec<u8> = (0..200u8).collect();
            let compressed = compress(&data).unwrap();
            let decompressed = decompress(&compressed, data.len()).unwrap();
            assert_eq!(data, decompressed);
        }

        #[test]
        fn roundtrip_run() {
            let data = vec![0xAAu8; 256];
            let compressed = compress(&data).unwrap();
            // A run of 256 should compress to ~4 bytes
            assert!(compressed.len() < 20, "compressed len = {}", compressed.len());
            let decompressed = decompress(&compressed, data.len()).unwrap();
            assert_eq!(data, decompressed);
        }

        #[test]
        fn roundtrip_mixed() {
            let mut data = vec![42u8; 50];
            data.extend_from_slice(b"Hello, World!");
            data.extend(vec![7u8; 100]);
            let compressed = compress(&data).unwrap();
            let decompressed = decompress(&compressed, data.len()).unwrap();
            assert_eq!(data, decompressed);
        }
    }
}

#[cfg(test)]
mod codec_tests {
    use super::*;

    fn test_roundtrip(codec: Compression, data: &[u8]) {
        let compressed = compress(codec, data).unwrap();
        let decompressed = decompress(codec, &compressed, data.len()).unwrap();
        assert_eq!(data, decompressed.as_slice(), "roundtrip failed for {:?}", codec);
    }

    #[test]
    fn none_roundtrip() {
        test_roundtrip(Compression::None, b"Hello, GeoTIFF!");
    }

    #[test]
    fn packbits_roundtrip() {
        let data: Vec<u8> = (0..=255u8).cycle().take(1024).collect();
        test_roundtrip(Compression::PackBits, &data);
    }

    #[test]
    fn lzw_roundtrip() {
        let data: Vec<u8> = (0..=255u8).cycle().take(4096).collect();
        test_roundtrip(Compression::Lzw, &data);
    }

    #[test]
    fn deflate_roundtrip() {
        let data: Vec<u8> = (0..=255u8).cycle().take(4096).collect();
        test_roundtrip(Compression::Deflate, &data);
    }
}
