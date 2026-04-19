//! Top-level JPEG 2000 encode orchestration.
//!
//! Coordinates the full encoding pipeline:
//!   pixels → MCT → DWT → quantize → EBCOT T1 → T2 → codestream
//!
//! Supports both lossless (5-3 reversible) and lossy (9-7 irreversible) encoding.

use alloc::vec;
use alloc::vec::Vec;

use super::bitplane_encode;
use super::build::SubBandType;
use super::codestream_write::{self, BlockCodingMode, EncodeParams};
use super::fdwt::{self, DwtDecomposition};
use super::forward_mct;
use super::ht_block_encode;
use super::packet_encode::{self, CodeBlockPacketData, ResolutionPacket, SubbandPrecinct};
use super::quantize::{self, QuantStepSize};
use crate::{DecodeSettings, Image};

/// Encoding options for JPEG 2000.
#[derive(Debug, Clone)]
pub struct EncodeOptions {
    /// Number of decomposition levels (default: 5).
    pub num_decomposition_levels: u8,
    /// Use reversible (lossless) transform (default: true).
    pub reversible: bool,
    /// Code-block width exponent minus 2 (default: 4, meaning 2^6=64).
    pub code_block_width_exp: u8,
    /// Code-block height exponent minus 2 (default: 4, meaning 2^6=64).
    pub code_block_height_exp: u8,
    /// Number of guard bits (default: 1 for reversible, 2 for irreversible).
    pub guard_bits: u8,
    /// Encode using HT block coding (HTJ2K / Part 15) instead of classic EBCOT.
    pub use_ht_block_coding: bool,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            num_decomposition_levels: 5,
            reversible: true,
            code_block_width_exp: 4,
            code_block_height_exp: 4,
            guard_bits: 1,
            use_ht_block_coding: false,
        }
    }
}

/// Encode pixel data into a JPEG 2000 codestream.
///
/// # Arguments
/// * `pixels` — Raw pixel data. For 8-bit: one byte per sample. For >8-bit: two bytes per sample (little-endian u16).
/// * `width` — Image width in pixels.
/// * `height` — Image height in pixels.
/// * `num_components` — Number of components (1 for grayscale, 3 for RGB).
/// * `bit_depth` — Bits per sample (e.g., 8, 12, 16).
/// * `signed` — Whether samples are signed.
/// * `options` — Encoding parameters.
///
/// # Returns
/// The encoded JPEG 2000 codestream bytes (`.j2c` format).
pub fn encode(
    pixels: &[u8],
    width: u32,
    height: u32,
    num_components: u8,
    bit_depth: u8,
    signed: bool,
    options: &EncodeOptions,
) -> Result<Vec<u8>, &'static str> {
    let block_coding_mode = block_coding_mode(options);
    let codestream = encode_impl(
        pixels,
        width,
        height,
        num_components,
        bit_depth,
        signed,
        options,
        block_coding_mode,
    )?;

    if block_coding_mode == BlockCodingMode::HighThroughput {
        validate_htj2k_codestream(
            &codestream,
            pixels,
            width,
            height,
            num_components,
            bit_depth,
            signed,
            options.reversible,
        )?;
    }

    Ok(codestream)
}

/// Encode pixel data into an HTJ2K codestream.
///
/// Lossless HTJ2K output is self-validated before it is returned.
pub fn encode_htj2k(
    pixels: &[u8],
    width: u32,
    height: u32,
    num_components: u8,
    bit_depth: u8,
    signed: bool,
    options: &EncodeOptions,
) -> Result<Vec<u8>, &'static str> {
    let mut options = options.clone();
    options.use_ht_block_coding = true;
    encode(
        pixels,
        width,
        height,
        num_components,
        bit_depth,
        signed,
        &options,
    )
}

fn block_coding_mode(options: &EncodeOptions) -> BlockCodingMode {
    if options.use_ht_block_coding {
        BlockCodingMode::HighThroughput
    } else {
        BlockCodingMode::Classic
    }
}

fn validate_htj2k_codestream(
    codestream: &[u8],
    pixels: &[u8],
    width: u32,
    height: u32,
    num_components: u8,
    bit_depth: u8,
    signed: bool,
    reversible: bool,
) -> Result<(), &'static str> {
    let image = Image::new(codestream, &DecodeSettings::default())
        .map_err(|_| "generated HTJ2K codestream failed self-validation")?;
    let decoded = image
        .decode_native()
        .map_err(|_| "generated HTJ2K codestream failed self-validation")?;

    if decoded.width != width
        || decoded.height != height
        || decoded.bit_depth != bit_depth
        || decoded.num_components != num_components
    {
        return Err("generated HTJ2K codestream failed self-validation");
    }

    if reversible && !native_samples_equal(pixels, &decoded.data, bit_depth, signed) {
        return Err("generated HTJ2K codestream did not roundtrip");
    }

    Ok(())
}

fn native_samples_equal(expected: &[u8], actual: &[u8], bit_depth: u8, signed: bool) -> bool {
    if expected.len() != actual.len() {
        return false;
    }

    let bytes_per_sample = if bit_depth <= 8 { 1 } else { 2 };
    let sample_count = expected.len() / bytes_per_sample;
    (0..sample_count).all(|sample_index| {
        decode_native_sample(expected, sample_index, bit_depth, signed)
            == decode_native_sample(actual, sample_index, bit_depth, signed)
    })
}

fn decode_native_sample(bytes: &[u8], sample_index: usize, bit_depth: u8, signed: bool) -> i32 {
    let byte_offset = sample_index * if bit_depth <= 8 { 1 } else { 2 };
    let mask = (1u32 << u32::from(bit_depth)) - 1;
    let raw = if bit_depth <= 8 {
        u32::from(bytes[byte_offset])
    } else {
        u32::from(u16::from_le_bytes([
            bytes[byte_offset],
            bytes[byte_offset + 1],
        ]))
    } & mask;

    if signed {
        let shift = 32 - u32::from(bit_depth);
        ((raw << shift) as i32) >> shift
    } else {
        raw as i32
    }
}

fn encode_impl(
    pixels: &[u8],
    width: u32,
    height: u32,
    num_components: u8,
    bit_depth: u8,
    signed: bool,
    options: &EncodeOptions,
    block_coding_mode: BlockCodingMode,
) -> Result<Vec<u8>, &'static str> {
    if width == 0 || height == 0 {
        return Err("invalid dimensions");
    }
    if num_components == 0 || num_components > 4 {
        return Err("unsupported component count");
    }
    if bit_depth == 0 || bit_depth > 16 {
        return Err("unsupported bit depth");
    }

    let num_pixels = (width * height) as usize;
    let bytes_per_sample = if bit_depth <= 8 { 1 } else { 2 };
    let expected_len = num_pixels * num_components as usize * bytes_per_sample;
    if pixels.len() < expected_len {
        return Err("pixel data too short");
    }

    #[cfg(feature = "openjph-htj2k")]
    if block_coding_mode == BlockCodingMode::HighThroughput {
        let mut openjph_options = options.clone();
        openjph_options.num_decomposition_levels = options
            .num_decomposition_levels
            .min(max_decomposition_levels(width, height));
        return crate::openjph_htj2k::encode(
            pixels,
            width,
            height,
            num_components,
            bit_depth,
            signed,
            &openjph_options,
        );
    }

    // Step 1: Convert pixel bytes to f32 component arrays
    let mut components = deinterleave_to_f32(pixels, num_pixels, num_components, bit_depth, signed);

    // Step 2: Apply forward MCT if RGB with 3+ components
    let use_mct = num_components >= 3;
    if use_mct {
        if options.reversible {
            forward_mct::forward_rct(&mut components);
        } else {
            forward_mct::forward_ict(&mut components);
        }
    }

    // Step 3: Apply forward DWT to each component
    let num_levels = options.num_decomposition_levels.min(
        // Don't decompose more than the image supports
        max_decomposition_levels(width, height),
    );

    let decompositions: Vec<DwtDecomposition> = components
        .iter()
        .map(|comp| fdwt::forward_dwt(comp, width, height, num_levels, options.reversible))
        .collect();

    // Step 4: Compute quantization step sizes
    let guard_bits = if options.reversible {
        options.guard_bits
    } else {
        options.guard_bits.max(2)
    };

    let step_sizes =
        quantize::compute_step_sizes(bit_depth, num_levels, options.reversible, guard_bits);

    // Step 5: Quantize and encode code-blocks for each component
    let cb_width = 1u32 << (options.code_block_width_exp + 2);
    let cb_height = 1u32 << (options.code_block_height_exp + 2);

    let mut resolution_packets: Vec<ResolutionPacket> = Vec::new();

    for decomp in decompositions.iter().take(num_components as usize) {
        // LL subband (resolution 0)
        let ll_subband = encode_subband(
            &decomp.ll,
            decomp.ll_width,
            decomp.ll_height,
            &step_sizes[0],
            guard_bits,
            options.reversible,
            block_coding_mode,
            cb_width,
            cb_height,
            SubBandType::LowLow,
        )?;
        resolution_packets.push(ResolutionPacket {
            subbands: vec![ll_subband],
        });

        // Higher resolution levels
        for (level_idx, level) in decomp.levels.iter().enumerate() {
            let step_base = 1 + level_idx * 3;

            // HL subband
            let hl_subband = encode_subband(
                &level.hl,
                level.high_width,
                level.low_height,
                &step_sizes[step_base],
                guard_bits,
                options.reversible,
                block_coding_mode,
                cb_width,
                cb_height,
                SubBandType::HighLow,
            )?;

            // LH subband
            let lh_subband = encode_subband(
                &level.lh,
                level.low_width,
                level.high_height,
                &step_sizes[step_base + 1],
                guard_bits,
                options.reversible,
                block_coding_mode,
                cb_width,
                cb_height,
                SubBandType::LowHigh,
            )?;

            // HH subband
            let hh_subband = encode_subband(
                &level.hh,
                level.high_width,
                level.high_height,
                &step_sizes[step_base + 2],
                guard_bits,
                options.reversible,
                block_coding_mode,
                cb_width,
                cb_height,
                SubBandType::HighHigh,
            )?;

            resolution_packets.push(ResolutionPacket {
                subbands: vec![hl_subband, lh_subband, hh_subband],
            });
        }
    }

    // Step 6: Form tile bitstream (T2)
    let tile_data = packet_encode::form_tile_bitstream(&mut resolution_packets, 1, num_components);

    // Step 7: Write codestream
    let quant_params: Vec<(u16, u16)> = step_sizes
        .iter()
        .map(|s| (s.exponent, s.mantissa))
        .collect();

    let params = EncodeParams {
        width,
        height,
        num_components,
        bit_depth,
        signed,
        num_decomposition_levels: num_levels,
        reversible: options.reversible,
        code_block_width_exp: options.code_block_width_exp,
        code_block_height_exp: options.code_block_height_exp,
        num_layers: 1,
        use_mct,
        guard_bits,
        block_coding_mode,
    };

    Ok(codestream_write::write_codestream(
        &params,
        &tile_data,
        &quant_params,
    ))
}

/// Encode a single subband into a SubbandPrecinct.
fn encode_subband(
    coefficients: &[f32],
    width: u32,
    height: u32,
    step_size: &QuantStepSize,
    guard_bits: u8,
    reversible: bool,
    block_coding_mode: BlockCodingMode,
    cb_width: u32,
    cb_height: u32,
    sub_band_type: SubBandType,
) -> Result<SubbandPrecinct, &'static str> {
    if width == 0 || height == 0 {
        return Ok(SubbandPrecinct {
            code_blocks: Vec::new(),
            num_cbs_x: 0,
            num_cbs_y: 0,
        });
    }

    // Quantize
    let quantized = quantize::quantize_subband(coefficients, step_size, guard_bits, reversible);
    debug_assert!(step_size.exponent <= u16::from(u8::MAX));
    let total_bitplanes = guard_bits
        .saturating_add(step_size.exponent as u8)
        .saturating_sub(1);

    // Split into code-blocks
    let num_cbs_x = width.div_ceil(cb_width);
    let num_cbs_y = height.div_ceil(cb_height);
    let mut code_blocks = Vec::with_capacity((num_cbs_x * num_cbs_y) as usize);

    for cby in 0..num_cbs_y {
        for cbx in 0..num_cbs_x {
            let x0 = cbx * cb_width;
            let y0 = cby * cb_height;
            let x1 = (x0 + cb_width).min(width);
            let y1 = (y0 + cb_height).min(height);
            let cbw = x1 - x0;
            let cbh = y1 - y0;

            // Extract code-block coefficients
            let mut cb_coeffs = vec![0i32; (cbw * cbh) as usize];
            for y in 0..cbh {
                for x in 0..cbw {
                    cb_coeffs[(y * cbw + x) as usize] =
                        quantized[((y0 + y) * width + (x0 + x)) as usize];
                }
            }

            let encoded = if block_coding_mode == BlockCodingMode::HighThroughput {
                ht_block_encode::encode_code_block(&cb_coeffs, cbw, cbh, total_bitplanes)?
            } else {
                bitplane_encode::encode_code_block(
                    &cb_coeffs,
                    cbw,
                    cbh,
                    sub_band_type,
                    total_bitplanes,
                )
            };

            code_blocks.push(CodeBlockPacketData {
                data: encoded.data,
                num_coding_passes: encoded.num_coding_passes,
                num_zero_bitplanes: encoded.num_zero_bitplanes,
                previously_included: false,
                l_block: 3,
                block_coding_mode,
            });
        }
    }

    Ok(SubbandPrecinct {
        code_blocks,
        num_cbs_x,
        num_cbs_y,
    })
}

/// Convert interleaved pixel bytes to per-component f32 arrays.
fn deinterleave_to_f32(
    pixels: &[u8],
    num_pixels: usize,
    num_components: u8,
    bit_depth: u8,
    signed: bool,
) -> Vec<Vec<f32>> {
    let nc = num_components as usize;
    let mut components = vec![vec![0.0f32; num_pixels]; nc];
    let unsigned_offset = if signed {
        0.0
    } else {
        (1u32 << (bit_depth as u32 - 1)) as f32
    };

    if bit_depth <= 8 {
        for (i, pixel) in pixels.chunks_exact(nc).take(num_pixels).enumerate() {
            for (c, component) in components.iter_mut().enumerate().take(nc) {
                let val = pixel[c];
                component[i] = if signed {
                    (val as i8) as f32
                } else {
                    val as f32 - unsigned_offset
                };
            }
        }
    } else {
        // 16-bit samples (little-endian)
        for (i, pixel) in pixels.chunks_exact(nc * 2).take(num_pixels).enumerate() {
            for (c, component) in components.iter_mut().enumerate().take(nc) {
                let offset = c * 2;
                let val = u16::from_le_bytes([pixel[offset], pixel[offset + 1]]);
                component[i] = if signed {
                    (val as i16) as f32
                } else {
                    val as f32 - unsigned_offset
                };
            }
        }
    }

    components
}

/// Calculate the maximum number of decomposition levels for given dimensions.
fn max_decomposition_levels(width: u32, height: u32) -> u8 {
    let min_dim = width.min(height);
    if min_dim <= 1 {
        return 0;
    }
    (min_dim as f32).log2().floor() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_8bit_gray() {
        let width = 8u32;
        let height = 8u32;
        let pixels: Vec<u8> = (0..64).collect();

        let result = encode(
            &pixels,
            width,
            height,
            1,
            8,
            false,
            &EncodeOptions {
                num_decomposition_levels: 2,
                ..Default::default()
            },
        );

        assert!(result.is_ok());
        let codestream = result.unwrap();
        // Verify SOC marker
        assert_eq!(codestream[0], 0xFF);
        assert_eq!(codestream[1], 0x4F);
        // Verify EOC marker
        let len = codestream.len();
        assert_eq!(codestream[len - 2], 0xFF);
        assert_eq!(codestream[len - 1], 0xD9);
    }

    #[test]
    fn test_encode_16bit_gray() {
        let width = 8u32;
        let height = 8u32;
        let mut pixels = Vec::with_capacity(128);
        for i in 0..64u16 {
            let val = i * 100;
            pixels.extend_from_slice(&val.to_le_bytes());
        }

        let result = encode(
            &pixels,
            width,
            height,
            1,
            16,
            false,
            &EncodeOptions {
                num_decomposition_levels: 2,
                ..Default::default()
            },
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_encode_rgb() {
        let width = 16u32;
        let height = 16u32;
        let pixels: Vec<u8> = (0..width * height * 3).map(|i| (i & 0xFF) as u8).collect();

        let result = encode(
            &pixels,
            width,
            height,
            3,
            8,
            false,
            &EncodeOptions {
                num_decomposition_levels: 3,
                ..Default::default()
            },
        );

        assert!(result.is_ok(), "RGB encode failed: {:?}", result.err());
    }

    #[test]
    fn test_encode_lossy() {
        let pixels: Vec<u8> = (0..64).collect();

        let result = encode(
            &pixels,
            8,
            8,
            1,
            8,
            false,
            &EncodeOptions {
                num_decomposition_levels: 2,
                reversible: false,
                guard_bits: 2,
                ..Default::default()
            },
        );

        assert!(result.is_ok());
    }

    fn assert_htj2k_lossless_roundtrip(
        pixels: &[u8],
        width: u32,
        height: u32,
        bit_depth: u8,
        num_decomposition_levels: u8,
    ) {
        let codestream = encode_htj2k(
            pixels,
            width,
            height,
            1,
            bit_depth,
            false,
            &EncodeOptions {
                num_decomposition_levels,
                ..Default::default()
            },
        )
        .expect("HTJ2K encode");

        assert!(codestream.windows(2).any(|window| window == [0xFF, 0x50]));
        let cod_offset = codestream
            .windows(2)
            .position(|window| window == [0xFF, 0x52])
            .expect("COD marker");
        assert_eq!(codestream[cod_offset + 12], 0x40);

        let image = Image::new(
            &codestream,
            &DecodeSettings {
                resolve_palette_indices: true,
                strict: true,
                target_resolution: None,
            },
        )
        .expect("parse HT codestream");
        let decoded = image.decode_native().expect("decode HT codestream");

        assert_eq!(decoded.width, width);
        assert_eq!(decoded.height, height);
        assert_eq!(decoded.bit_depth, bit_depth);
        assert_eq!(decoded.data, pixels);
    }

    fn gradient_u8(width: u32, height: u32) -> Vec<u8> {
        let mut pixels = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            for x in 0..width {
                pixels.push(((x * 17 + y * 31) % 256) as u8);
            }
        }
        pixels
    }

    #[test]
    fn test_encode_high_throughput_zero_image_roundtrip() {
        let width = 4u32;
        let height = 4u32;
        let sample = 2048u16.to_le_bytes();
        let mut pixels = Vec::with_capacity((width * height * 2) as usize);
        for _ in 0..(width * height) {
            pixels.extend_from_slice(&sample);
        }

        let codestream = encode(
            &pixels,
            width,
            height,
            1,
            12,
            false,
            &EncodeOptions {
                num_decomposition_levels: 2,
                use_ht_block_coding: true,
                ..Default::default()
            },
        )
        .expect("HT all-zero encode");

        assert!(codestream.windows(2).any(|window| window == [0xFF, 0x50]));
        let cod_offset = codestream
            .windows(2)
            .position(|window| window == [0xFF, 0x52])
            .expect("COD marker");
        assert_eq!(codestream[cod_offset + 12], 0x40);

        let image =
            Image::new(&codestream, &DecodeSettings::default()).expect("parse HT codestream");
        let decoded = image.decode_native().expect("decode HT codestream");

        assert_eq!(decoded.width, width);
        assert_eq!(decoded.height, height);
        assert_eq!(decoded.bit_depth, 12);
        assert_eq!(decoded.data, pixels);
    }

    #[test]
    fn test_encode_high_throughput_nonzero_roundtrip() {
        let width = 1u32;
        let height = 1u32;
        let pixels = 2049u16.to_le_bytes().to_vec();

        let codestream = encode_htj2k(
            &pixels,
            width,
            height,
            1,
            12,
            false,
            &EncodeOptions {
                num_decomposition_levels: 0,
                ..Default::default()
            },
        )
        .expect("HT non-zero encode");

        assert!(codestream.windows(2).any(|window| window == [0xFF, 0x50]));
        let image =
            Image::new(&codestream, &DecodeSettings::default()).expect("parse HT codestream");
        let decoded = image.decode_native().expect("decode HT codestream");

        assert_eq!(decoded.width, width);
        assert_eq!(decoded.height, height);
        assert_eq!(decoded.bit_depth, 12);
        assert_eq!(decoded.data, pixels);
    }

    #[test]
    fn test_encode_high_throughput_varied_12bit_roundtrip() {
        let mut pixels = Vec::with_capacity(32);
        for i in 0u16..16 {
            pixels.extend_from_slice(&((i * 257) & 0x0FFF).to_le_bytes());
        }

        let codestream = encode_htj2k(
            &pixels,
            4,
            4,
            1,
            12,
            false,
            &EncodeOptions {
                num_decomposition_levels: 1,
                ..Default::default()
            },
        )
        .expect("HT varied encode");

        let image =
            Image::new(&codestream, &DecodeSettings::default()).expect("parse HT codestream");
        let decoded = image.decode_native().expect("decode HT codestream");

        assert_eq!(decoded.width, 4);
        assert_eq!(decoded.height, 4);
        assert_eq!(decoded.bit_depth, 12);
        assert_eq!(decoded.data, pixels);
    }

    #[test]
    fn test_encode_high_throughput_gradient_8bit_roundtrip() {
        let pixels: Vec<u8> = (0..64).collect();

        let codestream = encode_htj2k(
            &pixels,
            8,
            8,
            1,
            8,
            false,
            &EncodeOptions {
                num_decomposition_levels: 3,
                ..Default::default()
            },
        )
        .expect("HT gradient encode");

        let image =
            Image::new(&codestream, &DecodeSettings::default()).expect("parse HT codestream");
        let decoded = image.decode_native().expect("decode HT codestream");

        assert_eq!(decoded.width, 8);
        assert_eq!(decoded.height, 8);
        assert_eq!(decoded.bit_depth, 8);
        assert_eq!(decoded.data, pixels);
    }

    #[test]
    fn test_encode_high_throughput_varied_12bit_large_roundtrip() {
        let width = 16u32;
        let height = 8u32;
        let mut pixels = Vec::with_capacity((width * height * 2) as usize);
        for y in 0u16..height as u16 {
            for x in 0u16..width as u16 {
                let value = (x * 257 + y * 17) & 0x0FFF;
                pixels.extend_from_slice(&value.to_le_bytes());
            }
        }

        assert_htj2k_lossless_roundtrip(&pixels, width, height, 12, 4);
    }

    #[test]
    fn test_encode_high_throughput_ramp_16bit_roundtrip() {
        let width = 48u32;
        let height = 24u32;
        let mut pixels = Vec::with_capacity((width * height * 2) as usize);
        for y in 0u16..height as u16 {
            for x in 0u16..width as u16 {
                let value = x * 521 + y * 997;
                pixels.extend_from_slice(&value.to_le_bytes());
            }
        }

        assert_htj2k_lossless_roundtrip(&pixels, width, height, 16, 4);
    }

    #[test]
    fn test_encode_high_throughput_lossy_large_gradient_is_parseable() {
        let pixels = gradient_u8(128, 128);

        let codestream = encode_htj2k(
            &pixels,
            128,
            128,
            1,
            8,
            false,
            &EncodeOptions {
                num_decomposition_levels: 5,
                reversible: false,
                guard_bits: 2,
                ..Default::default()
            },
        )
        .expect("lossy HT encode");

        assert!(codestream.windows(2).any(|window| window == [0xFF, 0x50]));
        assert!(!codestream.is_empty());

        let image = Image::new(
            &codestream,
            &DecodeSettings {
                resolve_palette_indices: true,
                strict: true,
                target_resolution: None,
            },
        )
        .expect("parse lossy HT codestream");
        let decoded = image.decode_native().expect("decode lossy HT codestream");

        assert_eq!(decoded.width, 128);
        assert_eq!(decoded.height, 128);
        assert_eq!(decoded.bit_depth, 8);
    }

    #[test]
    fn test_encode_invalid_dimensions() {
        let result = encode(&[], 0, 0, 1, 8, false, &EncodeOptions::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_too_short() {
        let pixels = vec![0u8; 10]; // Way too short for 8x8
        let result = encode(&pixels, 8, 8, 1, 8, false, &EncodeOptions::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_deinterleave_rgb() {
        let pixels = vec![
            10u8, 20, 30, // pixel 0: R=10, G=20, B=30
            40, 50, 60, // pixel 1: R=40, G=50, B=60
        ];
        let comps = deinterleave_to_f32(&pixels, 2, 3, 8, false);
        assert_eq!(comps[0], vec![-118.0, -88.0]); // R
        assert_eq!(comps[1], vec![-108.0, -78.0]); // G
        assert_eq!(comps[2], vec![-98.0, -68.0]); // B
    }

    #[test]
    fn test_encode_decode_roundtrip_gray_8bit() {
        use crate::{DecodeSettings, Image};

        // Constant image: all pixels = 42 — simplest possible test
        let original: Vec<u8> = vec![42u8; 64]; // 8x8, all same value
        let encoded = encode(
            &original,
            8,
            8,
            1,
            8,
            false,
            &EncodeOptions {
                num_decomposition_levels: 0,
                reversible: true,
                ..Default::default()
            },
        )
        .expect("encode failed");

        let settings = DecodeSettings {
            resolve_palette_indices: false,
            strict: false,
            target_resolution: None,
        };
        let image = Image::new(&encoded, &settings).expect("parse failed");
        let decoded = image.decode_native().expect("decode failed");

        assert_eq!(decoded.width, 8);
        assert_eq!(decoded.height, 8);
        assert_eq!(decoded.data, original, "round-trip mismatch");
    }
}
