//! Tier-2 packet formation for JPEG 2000 encoding.
//!
//! Organizes encoded code-block bitstreams into packets according to the
//! LRCP progression order. Each packet contains code-block data for a
//! single (layer, resolution, component, precinct) tuple.
//!
//! A packet at resolution 0 has one subband (LL).
//! A packet at resolution r > 0 has three subbands (HL, LH, HH).
//! Each subband has its own tag trees for inclusion and zero bitplanes.
//!
//! See Annex B of ITU-T T.800.

use alloc::vec::Vec;

use super::codestream_write::BlockCodingMode;
use super::tag_tree_encode::TagTreeEncoder;
use crate::writer::BitWriter;

/// A code-block's contribution to a packet.
#[derive(Debug)]
pub(crate) struct CodeBlockPacketData {
    /// Encoded bitstream data.
    pub(crate) data: Vec<u8>,
    /// Number of coding passes in this contribution.
    pub(crate) num_coding_passes: u8,
    /// Number of zero bitplanes (only relevant for first inclusion).
    pub(crate) num_zero_bitplanes: u8,
    /// Whether this code-block has been included in a previous packet.
    pub(crate) previously_included: bool,
    /// L-block value (for segment length encoding, starts at 3).
    pub(crate) l_block: u32,
    /// Block coder used for this contribution.
    pub(crate) block_coding_mode: BlockCodingMode,
}

/// Information about a single subband's precinct.
#[derive(Debug)]
pub(crate) struct SubbandPrecinct {
    /// Code-blocks in this subband's precinct (row-major order).
    pub(crate) code_blocks: Vec<CodeBlockPacketData>,
    /// Number of code-blocks in the x direction.
    pub(crate) num_cbs_x: u32,
    /// Number of code-blocks in the y direction.
    pub(crate) num_cbs_y: u32,
}

/// A resolution-level packet containing one or more subband precincts.
///
/// Resolution 0 has 1 subband (LL).
/// Resolution r>0 has 3 subbands (HL, LH, HH).
#[derive(Debug)]
pub(crate) struct ResolutionPacket {
    /// Subbands in this resolution's precinct.
    pub(crate) subbands: Vec<SubbandPrecinct>,
}

/// Form a packet from a resolution-level packet (possibly multiple subbands).
///
/// Returns the packet bytes (header + body).
pub(crate) fn form_packet(resolution: &mut ResolutionPacket) -> Vec<u8> {
    let mut header_writer = BitWriter::new();
    let mut body = Vec::new();

    // Check if any code-block across all subbands has data
    let any_data = resolution
        .subbands
        .iter()
        .any(|sb| sb.code_blocks.iter().any(|cb| cb.num_coding_passes > 0));

    if !any_data {
        // Empty packet: just write 0 bit
        header_writer.write_bit(0);
        return header_writer.finish();
    }

    // Non-empty packet indicator
    header_writer.write_bit(1);

    // Process each subband in order (LL for res 0; HL, LH, HH for res > 0)
    for subband in resolution.subbands.iter_mut() {
        // Create tag trees for this subband's code-block inclusion and zero bitplanes
        let mut inclusion_tree = TagTreeEncoder::new(subband.num_cbs_x, subband.num_cbs_y);
        let mut zbp_tree = TagTreeEncoder::new(subband.num_cbs_x, subband.num_cbs_y);

        // Set up tag tree values
        for (i, cb) in subband.code_blocks.iter().enumerate() {
            let x = i as u32 % subband.num_cbs_x;
            let y = i as u32 / subband.num_cbs_x;

            let inclusion_val = if cb.num_coding_passes > 0 {
                0
            } else {
                u32::MAX / 2
            };
            inclusion_tree.set_value(x, y, inclusion_val);
            zbp_tree.set_value(x, y, cb.num_zero_bitplanes as u32);
        }

        // Encode each code-block's packet contribution
        for (i, cb) in subband.code_blocks.iter_mut().enumerate() {
            let x = i as u32 % subband.num_cbs_x;
            let y = i as u32 / subband.num_cbs_x;

            if !cb.previously_included {
                // First inclusion: use tag tree
                inclusion_tree.encode(x, y, 1, &mut header_writer);

                if cb.num_coding_passes == 0 {
                    continue;
                }

                // Zero bitplanes: use tag tree
                zbp_tree.encode(x, y, cb.num_zero_bitplanes as u32 + 1, &mut header_writer);
            } else if cb.num_coding_passes > 0 {
                header_writer.write_bit(1);
            } else {
                header_writer.write_bit(0);
                continue;
            }

            if cb.num_coding_passes == 0 {
                continue;
            }

            let data_len = cb.data.len() as u32;
            match cb.block_coding_mode {
                BlockCodingMode::Classic => {
                    let num_bits = bits_for_length(cb.l_block, cb.num_coding_passes);
                    encode_num_coding_passes(cb.num_coding_passes, &mut header_writer);
                    encode_length(data_len, &mut cb.l_block, num_bits, &mut header_writer);
                }
                BlockCodingMode::HighThroughput => {
                    debug_assert!(
                        cb.num_coding_passes <= 1,
                        "current HT packet writer only supports cleanup-only contributions"
                    );
                    let num_bits = bits_for_ht_cleanup_length(cb.l_block, cb.num_coding_passes);
                    encode_num_ht_coding_passes(cb.num_coding_passes, &mut header_writer);
                    encode_length(data_len, &mut cb.l_block, num_bits, &mut header_writer);
                }
            }

            // Append code-block data to body
            body.extend_from_slice(&cb.data);
            cb.previously_included = true;
        }
    }

    // Assemble: header (byte-aligned) + body
    let mut packet = header_writer.finish();
    packet.extend_from_slice(&body);
    packet
}

/// Encode the number of coding passes using the variable-length code from Table B.4.
fn encode_num_coding_passes(num_passes: u8, writer: &mut BitWriter) {
    match num_passes {
        1 => writer.write_bit(0),
        2 => writer.write_bits(0b10, 2),
        3 => writer.write_bits(0b1100, 4),
        4 => writer.write_bits(0b1101, 4),
        5 => writer.write_bits(0b1110, 4),
        6..=36 => {
            writer.write_bits(0b1111, 4);
            writer.write_bits((num_passes - 6) as u32, 5);
        }
        37..=164 => {
            writer.write_bits(0b1_1111_1111, 9);
            writer.write_bits((num_passes - 37) as u32, 7);
        }
        _ => unreachable!("JPEG 2000 supports 1..=164 coding passes per contribution"),
    }
}

fn encode_num_ht_coding_passes(num_passes: u8, writer: &mut BitWriter) {
    match num_passes {
        1 => writer.write_bit(0),
        2 => writer.write_bits(0b10, 2),
        3..=5 => {
            writer.write_bits(0b11, 2);
            writer.write_bits(u32::from(num_passes - 3), 2);
        }
        6..=36 => {
            writer.write_bits(0b11, 2);
            writer.write_bits(0b11, 2);
            writer.write_bits(u32::from(num_passes - 6), 5);
        }
        37..=164 => {
            writer.write_bits(0b11, 2);
            writer.write_bits(0b11, 2);
            writer.write_bits(31, 5);
            writer.write_bits(u32::from(num_passes - 37), 7);
        }
        _ => unreachable!("JPEG 2000 supports 1..=164 coding passes per contribution"),
    }
}

fn encode_length(length: u32, l_block: &mut u32, mut num_bits: u32, writer: &mut BitWriter) {
    while !value_fits_in_bits(length, num_bits) {
        writer.write_bit(1);
        *l_block += 1;
        num_bits += 1;
    }
    writer.write_bit(0);
    writer.write_bits(length, num_bits as u8);
}

fn value_fits_in_bits(value: u32, bits: u32) -> bool {
    bits >= u32::BITS || value < (1u32 << bits)
}

/// Calculate number of bits needed to encode a segment length.
fn bits_for_length(l_block: u32, num_coding_passes: u8) -> u32 {
    let log2_passes = if num_coding_passes <= 1 {
        0
    } else {
        (num_coding_passes as u32).ilog2()
    };
    l_block + log2_passes
}

fn bits_for_ht_cleanup_length(l_block: u32, raw_num_passes: u8) -> u32 {
    let placeholder_groups = u32::from(raw_num_passes.saturating_sub(1)) / 3;
    let placeholder_passes = placeholder_groups * 3;
    l_block + (placeholder_passes + 1).ilog2()
}

/// Form tile bitstream from resolution packets in LRCP order.
///
/// `resolution_packets` contains one `ResolutionPacket` per resolution level:
/// - Index 0: LL band (resolution 0)
/// - Index 1..N: higher resolutions (each with HL, LH, HH subbands)
pub(crate) fn form_tile_bitstream(
    resolution_packets: &mut [ResolutionPacket],
    _num_layers: u8,
    _num_components: u8,
) -> Vec<u8> {
    let mut tile_data = Vec::new();

    // LRCP: Layer → Resolution → Component → Position
    // For single layer, single component, this is just resolution order
    for resolution in resolution_packets.iter_mut() {
        let packet = form_packet(resolution);
        tile_data.extend_from_slice(&packet);
    }

    tile_data
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::BitReader;

    fn decode_num_ht_coding_passes_for_test(data: &[u8]) -> Option<u8> {
        let mut reader = BitReader::new(data);
        let mut num_passes = 1u32;

        if reader.read_bits_with_stuffing(1)? == 1 {
            num_passes = 2;

            if reader.read_bits_with_stuffing(1)? == 1 {
                let extension = reader.read_bits_with_stuffing(2)?;
                num_passes = 3 + extension;

                if extension == 3 {
                    let extension = reader.read_bits_with_stuffing(5)?;
                    num_passes = 6 + extension;

                    if extension == 31 {
                        num_passes = 37 + reader.read_bits_with_stuffing(7)?;
                    }
                }
            }
        }

        Some(num_passes as u8)
    }

    fn decode_num_coding_passes_for_test(data: &[u8]) -> Option<u8> {
        let mut reader = BitReader::new(data);
        let passes = if reader.peak_bits_with_stuffing(9) == Some(0x1ff) {
            reader.read_bits_with_stuffing(9)?;
            reader.read_bits_with_stuffing(7)? + 37
        } else if reader.peak_bits_with_stuffing(4) == Some(0x0f) {
            reader.read_bits_with_stuffing(4)?;
            reader.read_bits_with_stuffing(5)? + 6
        } else if reader.peak_bits_with_stuffing(4) == Some(0b1110) {
            reader.read_bits_with_stuffing(4)?;
            5
        } else if reader.peak_bits_with_stuffing(4) == Some(0b1101) {
            reader.read_bits_with_stuffing(4)?;
            4
        } else if reader.peak_bits_with_stuffing(4) == Some(0b1100) {
            reader.read_bits_with_stuffing(4)?;
            3
        } else if reader.peak_bits_with_stuffing(2) == Some(0b10) {
            reader.read_bits_with_stuffing(2)?;
            2
        } else if reader.peak_bits_with_stuffing(1) == Some(0) {
            reader.read_bits_with_stuffing(1)?;
            1
        } else {
            return None;
        };
        Some(passes as u8)
    }

    #[test]
    fn test_empty_packet() {
        let mut resolution = ResolutionPacket {
            subbands: vec![SubbandPrecinct {
                code_blocks: vec![CodeBlockPacketData {
                    data: Vec::new(),
                    num_coding_passes: 0,
                    num_zero_bitplanes: 31,
                    previously_included: false,
                    l_block: 3,
                    block_coding_mode: BlockCodingMode::Classic,
                }],
                num_cbs_x: 1,
                num_cbs_y: 1,
            }],
        };

        let packet = form_packet(&mut resolution);
        assert!(!packet.is_empty());
    }

    #[test]
    fn test_non_empty_packet() {
        let mut resolution = ResolutionPacket {
            subbands: vec![SubbandPrecinct {
                code_blocks: vec![CodeBlockPacketData {
                    data: vec![0x12, 0x34, 0x56],
                    num_coding_passes: 1,
                    num_zero_bitplanes: 20,
                    previously_included: false,
                    l_block: 3,
                    block_coding_mode: BlockCodingMode::Classic,
                }],
                num_cbs_x: 1,
                num_cbs_y: 1,
            }],
        };

        let packet = form_packet(&mut resolution);
        assert!(packet.len() >= 3);
    }

    #[test]
    fn test_multi_subband_packet() {
        let mut resolution = ResolutionPacket {
            subbands: vec![
                SubbandPrecinct {
                    code_blocks: vec![CodeBlockPacketData {
                        data: vec![0x10, 0x20],
                        num_coding_passes: 1,
                        num_zero_bitplanes: 20,
                        previously_included: false,
                        l_block: 3,
                        block_coding_mode: BlockCodingMode::Classic,
                    }],
                    num_cbs_x: 1,
                    num_cbs_y: 1,
                },
                SubbandPrecinct {
                    code_blocks: vec![CodeBlockPacketData {
                        data: vec![0x30, 0x40],
                        num_coding_passes: 1,
                        num_zero_bitplanes: 22,
                        previously_included: false,
                        l_block: 3,
                        block_coding_mode: BlockCodingMode::Classic,
                    }],
                    num_cbs_x: 1,
                    num_cbs_y: 1,
                },
                SubbandPrecinct {
                    code_blocks: vec![CodeBlockPacketData {
                        data: vec![0x50],
                        num_coding_passes: 1,
                        num_zero_bitplanes: 24,
                        previously_included: false,
                        l_block: 3,
                        block_coding_mode: BlockCodingMode::Classic,
                    }],
                    num_cbs_x: 1,
                    num_cbs_y: 1,
                },
            ],
        };

        let packet = form_packet(&mut resolution);
        // Should contain all 5 bytes of code-block data
        assert!(packet.len() >= 5);
    }

    #[test]
    fn test_encode_num_passes() {
        let mut w = BitWriter::new();
        encode_num_coding_passes(1, &mut w);
        let d = w.finish();
        assert_eq!(d.len(), 1);
    }

    #[test]
    fn test_encode_num_passes_round_trip() {
        for num_passes in [1u8, 2, 3, 4, 5, 6, 19, 37, 38, 100, 164] {
            let mut w = BitWriter::new();
            encode_num_coding_passes(num_passes, &mut w);
            let data = w.finish();
            assert_eq!(decode_num_coding_passes_for_test(&data), Some(num_passes));
        }
    }

    #[test]
    fn test_encode_num_ht_passes_round_trip() {
        for num_passes in [1u8, 2, 3, 4, 5, 6, 19, 37, 38, 100, 164] {
            let mut w = BitWriter::new();
            encode_num_ht_coding_passes(num_passes, &mut w);
            let data = w.finish();
            assert_eq!(
                decode_num_ht_coding_passes_for_test(&data),
                Some(num_passes)
            );
        }
    }

    #[test]
    fn test_non_empty_ht_packet() {
        let mut resolution = ResolutionPacket {
            subbands: vec![SubbandPrecinct {
                code_blocks: vec![CodeBlockPacketData {
                    data: vec![0x12, 0x34, 0x56],
                    num_coding_passes: 1,
                    num_zero_bitplanes: 20,
                    previously_included: false,
                    l_block: 3,
                    block_coding_mode: BlockCodingMode::HighThroughput,
                }],
                num_cbs_x: 1,
                num_cbs_y: 1,
            }],
        };

        let packet = form_packet(&mut resolution);
        assert!(packet.len() >= 3);
    }
}
