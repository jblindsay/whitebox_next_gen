//! JPEG 2000 codestream writer (ITU-T T.800 Annex A).
//!
//! Writes the complete codestream including all required markers:
//! SOC, SIZ, COD, QCD, SOT, SOD, EOC.

use alloc::vec::Vec;

use super::codestream::markers;

/// Code-block coding mode for the codestream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockCodingMode {
    /// Classic JPEG 2000 Part 1 EBCOT block coding.
    Classic,
    /// High-throughput JPEG 2000 Part 15 block coding.
    HighThroughput,
}

/// Parameters for encoding a JPEG 2000 codestream.
#[derive(Debug, Clone)]
pub(crate) struct EncodeParams {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) num_components: u8,
    pub(crate) bit_depth: u8,
    pub(crate) signed: bool,
    pub(crate) num_decomposition_levels: u8,
    pub(crate) reversible: bool,
    pub(crate) code_block_width_exp: u8,
    pub(crate) code_block_height_exp: u8,
    pub(crate) num_layers: u8,
    pub(crate) use_mct: bool,
    pub(crate) guard_bits: u8,
    pub(crate) block_coding_mode: BlockCodingMode,
}

impl Default for EncodeParams {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            num_components: 1,
            bit_depth: 8,
            signed: false,
            num_decomposition_levels: 5,
            reversible: true,
            code_block_width_exp: 4, // 2^(4+2) = 64
            code_block_height_exp: 4,
            num_layers: 1,
            use_mct: false,
            guard_bits: 1,
            block_coding_mode: BlockCodingMode::Classic,
        }
    }
}

/// Write the complete JPEG 2000 codestream.
pub(crate) fn write_codestream(
    params: &EncodeParams,
    tile_data: &[u8],
    quantization_step_sizes: &[(u16, u16)], // (exponent, mantissa)
) -> Vec<u8> {
    let mut out = Vec::with_capacity(tile_data.len() + 256);

    // SOC (Start of codestream)
    write_marker(&mut out, markers::SOC);

    // SIZ (Image and tile sizes)
    write_siz_marker(&mut out, params);

    if params.block_coding_mode == BlockCodingMode::HighThroughput {
        write_cap_marker(&mut out, params);
    }

    // COD (Coding style defaults)
    write_cod_marker(&mut out, params);

    // QCD (Quantization defaults)
    write_qcd_marker(&mut out, params, quantization_step_sizes);

    // SOT (Start of tile-part) — single tile covering entire image
    let tile_part_len = 12 + tile_data.len() as u32; // SOT header(12) + tile data
    write_sot_marker(&mut out, 0, tile_part_len);

    // SOD (Start of data)
    write_marker(&mut out, markers::SOD);

    // Tile bitstream data
    out.extend_from_slice(tile_data);

    // EOC (End of codestream)
    write_marker(&mut out, markers::EOC);

    out
}

fn write_marker(out: &mut Vec<u8>, marker: u8) {
    out.push(0xFF);
    out.push(marker);
}

/// Write SIZ marker segment (A.5.1).
fn write_siz_marker(out: &mut Vec<u8>, params: &EncodeParams) {
    write_marker(out, markers::SIZ);

    let num_comp = params.num_components as u16;
    let marker_len = 38 + 3 * num_comp;

    // Lsiz
    out.extend_from_slice(&marker_len.to_be_bytes());
    // Rsiz (capabilities) — profile 0 (no extensions)
    out.extend_from_slice(&0u16.to_be_bytes());
    // Xsiz (reference grid width)
    out.extend_from_slice(&params.width.to_be_bytes());
    // Ysiz (reference grid height)
    out.extend_from_slice(&params.height.to_be_bytes());
    // XOsiz (image area x offset)
    out.extend_from_slice(&0u32.to_be_bytes());
    // YOsiz (image area y offset)
    out.extend_from_slice(&0u32.to_be_bytes());
    // XTsiz (tile width = image width, single tile)
    out.extend_from_slice(&params.width.to_be_bytes());
    // YTsiz (tile height = image height, single tile)
    out.extend_from_slice(&params.height.to_be_bytes());
    // XTOsiz (tile x offset)
    out.extend_from_slice(&0u32.to_be_bytes());
    // YTOsiz (tile y offset)
    out.extend_from_slice(&0u32.to_be_bytes());
    // Csiz (number of components)
    out.extend_from_slice(&num_comp.to_be_bytes());

    // Per-component info
    for _ in 0..params.num_components {
        // Ssiz: bit depth - 1 (unsigned) or bit depth - 1 + 0x80 (signed)
        let ssiz = if params.signed {
            (params.bit_depth - 1) | 0x80
        } else {
            params.bit_depth - 1
        };
        out.push(ssiz);
        // XRsiz (horizontal sampling factor)
        out.push(1);
        // YRsiz (vertical sampling factor)
        out.push(1);
    }
}

/// Write CAP marker segment (Part 15 extended capabilities).
fn write_cap_marker(out: &mut Vec<u8>, params: &EncodeParams) {
    write_marker(out, markers::CAP);
    out.extend_from_slice(&8u16.to_be_bytes());
    out.extend_from_slice(&0x0002_0000u32.to_be_bytes());
    out.extend_from_slice(&ht_capability_word(params).to_be_bytes());
}

fn ht_capability_word(params: &EncodeParams) -> u16 {
    let magnitude_bits = u32::from(params.bit_depth.saturating_sub(1));
    let bp = if magnitude_bits <= 8 {
        0
    } else if magnitude_bits < 28 {
        magnitude_bits - 8
    } else {
        13 + (magnitude_bits >> 2)
    };

    let wavelet_flag = if params.reversible { 0u16 } else { 0x0020u16 };
    wavelet_flag | (bp as u16)
}

/// Write COD marker segment (A.6.1).
fn write_cod_marker(out: &mut Vec<u8>, params: &EncodeParams) {
    write_marker(out, markers::COD);

    let marker_len = 12u16 + 1; // Fixed length for no precincts
    out.extend_from_slice(&marker_len.to_be_bytes());

    // Scod (coding style flags) — no precincts, no SOP, no EPH
    out.push(0x00);

    // SGcod: Progression order (LRCP = 0)
    out.push(0x00);
    // Number of layers
    out.extend_from_slice(&(params.num_layers as u16).to_be_bytes());
    // Multiple component transform
    out.push(if params.use_mct { 1 } else { 0 });

    // SPcod: Number of decomposition levels
    out.push(params.num_decomposition_levels);
    // Code-block width exponent - 2
    out.push(params.code_block_width_exp);
    // Code-block height exponent - 2
    out.push(params.code_block_height_exp);
    // Code-block style
    out.push(match params.block_coding_mode {
        BlockCodingMode::Classic => 0x00,
        BlockCodingMode::HighThroughput => 0x40,
    });
    // Wavelet transform: 0 = irreversible 9-7, 1 = reversible 5-3
    out.push(if params.reversible { 1 } else { 0 });
}

/// Write QCD marker segment (A.6.4).
fn write_qcd_marker(out: &mut Vec<u8>, params: &EncodeParams, step_sizes: &[(u16, u16)]) {
    write_marker(out, markers::QCD);

    if params.reversible {
        // No quantization: Sqcd = 0x00, then exponent bytes
        let marker_len = 3 + step_sizes.len() as u16;
        out.extend_from_slice(&marker_len.to_be_bytes());

        // Sqcd: no quantization (style 0), guard bits in upper 3 bits
        out.push(params.guard_bits << 5);

        // SPqcd: one byte per subband (exponent in upper 5 bits, mantissa = 0)
        for &(exp, _) in step_sizes {
            out.push((exp as u8) << 3);
        }
    } else {
        // Scalar expounded: Sqcd = 0x02, then 2 bytes per subband
        let marker_len = 3 + step_sizes.len() as u16 * 2;
        out.extend_from_slice(&marker_len.to_be_bytes());

        // Sqcd: scalar expounded quantization, guard bits
        out.push((params.guard_bits << 5) | 0x02);

        // SPqcd: two bytes per subband (5-bit exponent + 11-bit mantissa)
        for &(exp, mant) in step_sizes {
            let val = ((exp & 0x1F) << 11) | (mant & 0x7FF);
            out.extend_from_slice(&val.to_be_bytes());
        }
    }
}

/// Write SOT marker segment (A.4.2).
fn write_sot_marker(out: &mut Vec<u8>, tile_index: u16, tile_part_length: u32) {
    write_marker(out, markers::SOT);

    // Lsot = 10
    out.extend_from_slice(&10u16.to_be_bytes());
    // Isot (tile index)
    out.extend_from_slice(&tile_index.to_be_bytes());
    // Psot (tile-part length including SOT marker)
    out.extend_from_slice(&(tile_part_length + 2).to_be_bytes()); // +2 for SOT marker bytes
                                                                  // TPsot (tile-part index)
    out.push(0);
    // TNsot (number of tile-parts, 0 = unknown)
    out.push(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find_marker_offset(codestream: &[u8], marker: u8) -> Option<usize> {
        codestream
            .windows(2)
            .position(|window| window == [0xFF, marker])
    }

    #[test]
    fn test_write_minimal_codestream() {
        let params = EncodeParams {
            width: 8,
            height: 8,
            num_components: 1,
            bit_depth: 8,
            num_decomposition_levels: 1,
            reversible: true,
            num_layers: 1,
            ..Default::default()
        };

        let tile_data = vec![0u8; 10];
        let step_sizes = vec![(9u16, 0u16), (8, 0), (8, 0), (7, 0)];
        let codestream = write_codestream(&params, &tile_data, &step_sizes);

        // Verify SOC marker
        assert_eq!(codestream[0], 0xFF);
        assert_eq!(codestream[1], markers::SOC);

        // Verify SIZ marker
        assert_eq!(codestream[2], 0xFF);
        assert_eq!(codestream[3], markers::SIZ);

        // Verify EOC marker
        let len = codestream.len();
        assert_eq!(codestream[len - 2], 0xFF);
        assert_eq!(codestream[len - 1], markers::EOC);
    }

    #[test]
    fn test_ht_capability_word_matches_fixture_examples() {
        let params = EncodeParams {
            bit_depth: 11,
            reversible: true,
            block_coding_mode: BlockCodingMode::HighThroughput,
            ..Default::default()
        };
        assert_eq!(ht_capability_word(&params), 0x0002);

        let params = EncodeParams {
            bit_depth: 12,
            reversible: true,
            block_coding_mode: BlockCodingMode::HighThroughput,
            ..Default::default()
        };
        assert_eq!(ht_capability_word(&params), 0x0003);

        let params = EncodeParams {
            bit_depth: 12,
            reversible: false,
            block_coding_mode: BlockCodingMode::HighThroughput,
            ..Default::default()
        };
        assert_eq!(ht_capability_word(&params), 0x0023);
    }

    #[test]
    fn test_write_ht_lossless_codestream_headers() {
        let params = EncodeParams {
            width: 3,
            height: 5,
            num_components: 1,
            bit_depth: 12,
            num_decomposition_levels: 1,
            reversible: true,
            num_layers: 1,
            block_coding_mode: BlockCodingMode::HighThroughput,
            ..Default::default()
        };

        let tile_data = vec![0u8; 1];
        let step_sizes = vec![(12u16, 0u16), (13, 0), (13, 0), (14, 0)];
        let codestream = write_codestream(&params, &tile_data, &step_sizes);

        let cap_offset = find_marker_offset(&codestream, markers::CAP).expect("CAP marker");
        let cap_len = u16::from_be_bytes([codestream[cap_offset + 2], codestream[cap_offset + 3]]);
        assert_eq!(cap_len, 8);
        assert_eq!(
            &codestream[cap_offset + 4..cap_offset + 10],
            &[0x00, 0x02, 0x00, 0x00, 0x00, 0x03]
        );

        let cod_offset = find_marker_offset(&codestream, markers::COD).expect("COD marker");
        assert_eq!(codestream[cod_offset + 12], 0x40);
        assert!(find_marker_offset(&codestream, markers::CPF).is_none());
    }

    #[test]
    fn test_write_rgb_codestream() {
        let params = EncodeParams {
            width: 16,
            height: 16,
            num_components: 3,
            bit_depth: 8,
            num_decomposition_levels: 2,
            reversible: true,
            use_mct: true,
            num_layers: 1,
            ..Default::default()
        };

        let tile_data = vec![0u8; 50];
        let step_sizes: Vec<(u16, u16)> = (0..7).map(|i| (9 - i / 3, 0)).collect();
        let codestream = write_codestream(&params, &tile_data, &step_sizes);

        // Should start with SOC and end with EOC
        assert_eq!(codestream[0], 0xFF);
        assert_eq!(codestream[1], markers::SOC);
        let len = codestream.len();
        assert_eq!(codestream[len - 2], 0xFF);
        assert_eq!(codestream[len - 1], markers::EOC);
    }
}
