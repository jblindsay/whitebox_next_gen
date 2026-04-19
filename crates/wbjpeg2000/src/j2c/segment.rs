//! Parsing of layers and their segments, as specified in Annex B.

use alloc::boxed::Box;

use super::build::{CodeBlock, Precinct, Segment};
use super::codestream::markers::{EPH, SOP};
use super::codestream::{ComponentInfo, Header};
use super::decode::DecompositionStorage;
use super::progression::ProgressionData;
use super::tag_tree::TagNode;
use super::tile::{Tile, TilePart};
use crate::error::{bail, DecodingError, Result, TileError};
use crate::reader::BitReader;

pub(crate) const MAX_BITPLANE_COUNT: u8 = 32;

pub(crate) fn parse<'a, 'b>(
    tile: &'b Tile<'a>,
    mut progression_iterator: Box<dyn Iterator<Item = ProgressionData> + '_>,
    header: &Header<'_>,
    storage: &mut DecompositionStorage<'a>,
) -> Result<()> {
    if tile.num_layers > 1
        && tile.component_infos.iter().any(|component_info| {
            component_info
                .coding_style
                .parameters
                .code_block_style
                .uses_high_throughput_block_coding()
        })
    {
        bail!(DecodingError::UnsupportedFeature(
            "multi-layer HTJ2K packet assembly"
        ));
    }

    for tile_part in &tile.tile_parts {
        if parse_inner(
            tile_part.clone(),
            &mut progression_iterator,
            &tile.component_infos,
            storage,
        )
        .is_none()
            && header.strict
        {
            bail!(TileError::Invalid);
        }
    }

    Ok(())
}

fn parse_inner<'a>(
    mut tile_part: TilePart<'a>,
    progression_iterator: &mut dyn Iterator<Item = ProgressionData>,
    component_infos: &[ComponentInfo],
    storage: &mut DecompositionStorage<'a>,
) -> Option<()> {
    while !tile_part.header().at_end() {
        let progression_data = progression_iterator.next()?;
        let resolution = progression_data.resolution;
        let component_info = &component_infos[progression_data.component as usize];
        let tile_decompositions =
            &mut storage.tile_decompositions[progression_data.component as usize];
        let sub_band_iter = tile_decompositions.sub_band_iter(resolution, &storage.decompositions);

        let body_reader = tile_part.body();

        if component_info.coding_style.flags.may_use_sop_markers()
            && body_reader.peek_marker() == Some(SOP)
        {
            body_reader.read_marker().ok()?;
            body_reader.skip_bytes(4)?;
        }

        let header_reader = tile_part.header();

        let zero_length = header_reader.read_bits_with_stuffing(1)? == 0;

        // B.10.3 Zero length packet
        // "The first bit in the packet header denotes whether the packet has a length of zero
        // (empty packet). The value 0 indicates a zero length; no code-blocks are included in this
        // case. The value 1 indicates a non-zero length."
        if !zero_length {
            for sub_band in sub_band_iter.clone() {
                resolve_segments(
                    sub_band,
                    &progression_data,
                    header_reader,
                    storage,
                    component_info,
                )?;
            }
        }

        header_reader.align();

        if component_info.coding_style.flags.uses_eph_marker()
            && header_reader.read_marker().ok()? != EPH
        {
            return None;
        }

        // Now read the packet body.
        let body_reader = tile_part.body();

        if !zero_length {
            for sub_band in sub_band_iter {
                let sub_band = &mut storage.sub_bands[sub_band];
                let precinct = &mut storage.precincts[sub_band.precincts.clone()]
                    [progression_data.precinct as usize];
                let code_blocks = &mut storage.code_blocks[precinct.code_blocks.clone()];

                for code_block in code_blocks {
                    let layer = &mut storage.layers[code_block.layers.clone()]
                        [progression_data.layer_num as usize];

                    if let Some(segments) = layer.segments.clone() {
                        let segments = &mut storage.segments[segments.clone()];

                        for segment in segments {
                            segment.data = body_reader.read_bytes(segment.data_length as usize)?;
                        }
                    }
                }
            }
        }
    }

    Some(())
}

fn resolve_segments(
    sub_band_dx: usize,
    progression_data: &ProgressionData,
    reader: &mut BitReader<'_>,
    storage: &mut DecompositionStorage<'_>,
    component_info: &ComponentInfo,
) -> Option<()> {
    if component_info
        .coding_style
        .parameters
        .code_block_style
        .uses_high_throughput_block_coding()
    {
        resolve_ht_segments(sub_band_dx, progression_data, reader, storage)
    } else {
        resolve_classic_segments(
            sub_band_dx,
            progression_data,
            reader,
            storage,
            component_info,
        )
    }
}

fn resolve_classic_segments(
    sub_band_dx: usize,
    progression_data: &ProgressionData,
    reader: &mut BitReader<'_>,
    storage: &mut DecompositionStorage<'_>,
    component_info: &ComponentInfo,
) -> Option<()> {
    // We don't support more than 32-bit precision.
    const MAX_CODING_PASSES: u8 = 1 + 3 * (MAX_BITPLANE_COUNT - 1);

    let sub_band = &storage.sub_bands[sub_band_dx];
    let precincts = &mut storage.precincts[sub_band.precincts.clone()];
    let Some(precinct) = precincts.get_mut(progression_data.precinct as usize) else {
        // An invalid file could trigger this code path.
        lwarn!("progression data yielded invalid precinct index");

        return None;
    };
    let code_blocks = &mut storage.code_blocks[precinct.code_blocks.clone()];

    for code_block in code_blocks {
        let inclusion = resolve_code_block_inclusion(
            code_block,
            precinct,
            progression_data,
            reader,
            &mut storage.tag_tree_nodes,
        )?;

        if !inclusion.included {
            continue;
        }

        let layer =
            &mut storage.layers[code_block.layers.clone()][progression_data.layer_num as usize];

        // B.10.6 Number of coding passes
        // "The number of coding passes included in this packet from each code-block is
        // identified in the packet header using the codewords shown in Table B.4. This
        // table provides for the possibility of signalling up to 164 coding passes."
        let added_coding_passes = decode_num_classic_coding_passes(reader)?;

        ltrace!("number of coding passes: {}", added_coding_passes);

        code_block.l_block = code_block
            .l_block
            .checked_add(read_lblock_increment(reader)?)?;

        let previous_layers_passes = code_block.number_of_coding_passes;
        let cumulative_passes = previous_layers_passes.checked_add(added_coding_passes)?;

        if cumulative_passes > MAX_CODING_PASSES {
            return None;
        }

        let get_segment_idx = |pass_idx: u8| {
            if component_info.code_block_style().termination_on_each_pass {
                // If we terminate on each pass, the segment is just the index
                // of the pass.
                pass_idx
            } else if component_info
                .code_block_style()
                .selective_arithmetic_coding_bypass
            {
                // Use the formula derived from the table in the spec.
                segment_idx_for_bypass(pass_idx)
            } else {
                // If none of the above flags is activated, the number of
                // segments just corresponds to the number of layers.
                code_block.non_empty_layer_count
            }
        };

        let start = storage.segments.len();

        let mut push_segment = |segment: u8, coding_passes_for_segment: u8| {
            let length = {
                assert!(coding_passes_for_segment > 0);

                // "A codeword segment is the number of bytes contributed to a packet by a
                // code-block. The length of a codeword segment is represented by a binary number of length:
                // bits = Lblock + floor(log_2(coding passes added))
                // where Lblock is a code-block state variable. A separate Lblock is used for each
                // code-block in the precinct. The value of Lblock is initially set to three. The
                // number of bytes contributed by each code-block is preceded by signalling bits
                // that increase the value of Lblock, as needed. A signalling bit of zero indicates
                // the current value of Lblock is sufficient. If there are k ones followed by a
                // zero, the value of Lblock is incremented by k. While Lblock can only increase,
                // the number of bits used to signal the length of the code-block contribution can
                // increase or decrease depending on the number of coding passes included."
                let length_bits = code_block.l_block + coding_passes_for_segment.ilog2();
                reader.read_bits_with_stuffing(length_bits as u8)
            }?;

            storage.segments.push(Segment {
                idx: segment,
                data_length: length,
                coding_pases: coding_passes_for_segment,
                // Will be set later.
                data: &[],
            });

            ltrace!("length({segment}) {}", length);

            Some(())
        };

        let mut last_segment = get_segment_idx(previous_layers_passes);
        let mut coding_passes_for_segment = 0;

        for coding_pass in previous_layers_passes..cumulative_passes {
            let segment = get_segment_idx(coding_pass);

            if segment != last_segment {
                push_segment(last_segment, coding_passes_for_segment)?;
                last_segment = segment;
                coding_passes_for_segment = 1;
            } else {
                coding_passes_for_segment += 1;
            }
        }

        // Flush the final segment if applicable.
        if coding_passes_for_segment > 0 {
            push_segment(last_segment, coding_passes_for_segment)?;
        }

        let end = storage.segments.len();
        layer.segments = Some(start..end);
        code_block.number_of_coding_passes += added_coding_passes;
        code_block.non_empty_layer_count += 1;
    }

    Some(())
}

fn resolve_ht_segments(
    sub_band_dx: usize,
    progression_data: &ProgressionData,
    reader: &mut BitReader<'_>,
    storage: &mut DecompositionStorage<'_>,
) -> Option<()> {
    const MAX_CODING_PASSES: u8 = 1 + 3 * (MAX_BITPLANE_COUNT - 1);

    let sub_band = &storage.sub_bands[sub_band_dx];
    let precincts = &mut storage.precincts[sub_band.precincts.clone()];
    let Some(precinct) = precincts.get_mut(progression_data.precinct as usize) else {
        lwarn!("progression data yielded invalid precinct index");

        return None;
    };
    let code_blocks = &mut storage.code_blocks[precinct.code_blocks.clone()];

    for code_block in code_blocks {
        let inclusion = resolve_code_block_inclusion(
            code_block,
            precinct,
            progression_data,
            reader,
            &mut storage.tag_tree_nodes,
        )?;

        if !inclusion.included {
            continue;
        }

        if !inclusion.included_first_time
            || code_block.number_of_coding_passes != 0
            || code_block.non_empty_layer_count != 0
        {
            return None;
        }

        let layer =
            &mut storage.layers[code_block.layers.clone()][progression_data.layer_num as usize];

        if layer.segments.is_some() {
            return None;
        }

        let raw_num_passes = decode_num_ht_coding_passes(reader)?;

        if raw_num_passes > MAX_CODING_PASSES {
            return None;
        }

        ltrace!("HT raw number of coding passes: {}", raw_num_passes);

        let parsed = parse_ht_segment_lengths(
            reader,
            raw_num_passes,
            code_block.missing_bit_planes,
            &mut code_block.l_block,
        )?;

        code_block.missing_bit_planes = parsed.missing_bit_planes;

        let start = storage.segments.len();
        storage.segments.push(Segment {
            idx: 0,
            coding_pases: 1,
            data_length: parsed.cleanup_length,
            data: &[],
        });

        ltrace!("HT cleanup length {}", parsed.cleanup_length);

        if parsed.actual_passes > 1 {
            storage.segments.push(Segment {
                idx: 1,
                coding_pases: parsed.actual_passes - 1,
                data_length: parsed.refinement_length,
                data: &[],
            });

            ltrace!("HT refinement length {}", parsed.refinement_length);
        }

        let end = storage.segments.len();
        layer.segments = Some(start..end);
        code_block.number_of_coding_passes = parsed.actual_passes;
        code_block.non_empty_layer_count = code_block.non_empty_layer_count.checked_add(1)?;
    }

    Some(())
}

#[derive(Clone, Copy)]
struct CodeBlockInclusion {
    included: bool,
    included_first_time: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ParsedHtSegments {
    actual_passes: u8,
    missing_bit_planes: u8,
    cleanup_length: u32,
    refinement_length: u32,
}

fn resolve_code_block_inclusion(
    code_block: &mut CodeBlock,
    precinct: &mut Precinct,
    progression_data: &ProgressionData,
    reader: &mut BitReader<'_>,
    tag_tree_nodes: &mut [TagNode],
) -> Option<CodeBlockInclusion> {
    // B.10.4 Code-block inclusion
    let included_first_time = !code_block.has_been_included;
    let is_included = if code_block.has_been_included {
        // "For code-blocks that have been included in a previous packet,
        // a single bit is used to represent the information, where a 1
        // means that the code-block is included in this layer and a 0 means
        // that it is not."
        reader.read_bits_with_stuffing(1)? == 1
    } else {
        // "For code-blocks that have not been previously included in any packet,
        // this information is signalled with a separate tag tree code for each precinct
        // as confined to a sub-band. The values in this tag tree are the number of the
        // layer in which the current code-block is first included. Although the exact
        // sequence of bits that represent the inclusion tag tree appears in the bit
        // stream, only the bits needed for determining whether the code-block is
        // included are placed in the packet header. If some of the tag tree is already
        // known from previous code-blocks or previous layers, it is not repeated.
        // Likewise, only as much of the tag tree as is needed to determine inclusion in
        // the current layer is included. If a code-block is not included until a later
        // layer, then only a partial tag tree is included at that point in the bit
        // stream."
        precinct.code_inclusion_tree.read(
            code_block.x_idx,
            code_block.y_idx,
            reader,
            progression_data.layer_num as u32 + 1,
            tag_tree_nodes,
        )? <= progression_data.layer_num as u32
    };

    ltrace!("code-block inclusion: {}", is_included);

    if !is_included {
        return Some(CodeBlockInclusion {
            included: false,
            included_first_time: false,
        });
    }

    // B.10.5 Zero bit-plane information
    // "If a code-block is included for the first time, the packet header contains
    // information identifying the actual number of bit-planes used to represent
    // coefficients from the code-block."
    if included_first_time {
        code_block.missing_bit_planes = precinct.zero_bitplane_tree.read(
            code_block.x_idx,
            code_block.y_idx,
            reader,
            u32::MAX,
            tag_tree_nodes,
        )? as u8;
        ltrace!(
            "zero bit-plane information: {}",
            code_block.missing_bit_planes
        );
    }

    code_block.has_been_included = true;

    Some(CodeBlockInclusion {
        included: true,
        included_first_time,
    })
}

fn decode_num_classic_coding_passes(reader: &mut BitReader<'_>) -> Option<u8> {
    if reader.peak_bits_with_stuffing(9) == Some(0x1ff) {
        reader.read_bits_with_stuffing(9)?;
        Some((reader.read_bits_with_stuffing(7)? + 37) as u8)
    } else if reader.peak_bits_with_stuffing(4) == Some(0x0f) {
        reader.read_bits_with_stuffing(4)?;
        Some((reader.read_bits_with_stuffing(5)? + 6) as u8)
    } else if reader.peak_bits_with_stuffing(4) == Some(0b1110) {
        reader.read_bits_with_stuffing(4)?;
        Some(5)
    } else if reader.peak_bits_with_stuffing(4) == Some(0b1101) {
        reader.read_bits_with_stuffing(4)?;
        Some(4)
    } else if reader.peak_bits_with_stuffing(4) == Some(0b1100) {
        reader.read_bits_with_stuffing(4)?;
        Some(3)
    } else if reader.peak_bits_with_stuffing(2) == Some(0b10) {
        reader.read_bits_with_stuffing(2)?;
        Some(2)
    } else if reader.peak_bits_with_stuffing(1) == Some(0) {
        reader.read_bits_with_stuffing(1)?;
        Some(1)
    } else {
        None
    }
}

fn decode_num_ht_coding_passes(reader: &mut BitReader<'_>) -> Option<u8> {
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

    u8::try_from(num_passes).ok()
}

fn read_lblock_increment(reader: &mut BitReader<'_>) -> Option<u32> {
    let mut increment = 0;

    while reader.read_bits_with_stuffing(1)? == 1 {
        increment += 1;
    }

    Some(increment)
}

fn parse_ht_segment_lengths(
    reader: &mut BitReader<'_>,
    raw_num_passes: u8,
    missing_bit_planes: u8,
    l_block: &mut u32,
) -> Option<ParsedHtSegments> {
    let placeholder_groups = u32::from(raw_num_passes.saturating_sub(1)) / 3;
    let missing_bit_planes = missing_bit_planes.checked_add(placeholder_groups as u8)?;
    let placeholder_passes = (placeholder_groups * 3) as u8;
    let actual_passes = raw_num_passes.checked_sub(placeholder_passes)?;

    *l_block = l_block.checked_add(read_lblock_increment(reader)?)?;

    let cleanup_length_bits = *l_block + (u32::from(placeholder_passes) + 1).ilog2();
    let cleanup_length = reader.read_bits_with_stuffing(cleanup_length_bits as u8)?;

    if !(2..65535).contains(&cleanup_length) {
        return None;
    }

    let refinement_length = if actual_passes > 1 {
        let length_bits = *l_block + if actual_passes > 2 { 1 } else { 0 };
        let length = reader.read_bits_with_stuffing(length_bits as u8)?;

        if length >= 2047 {
            return None;
        }

        length
    } else {
        0
    };

    Some(ParsedHtSegments {
        actual_passes,
        missing_bit_planes,
        cleanup_length,
        refinement_length,
    })
}

/// Calculate the segment index for the given pass in arithmetic decoder
/// bypass (see section D.6, Table D.9).
fn segment_idx_for_bypass(pass_idx: u8) -> u8 {
    if pass_idx < 10 {
        0
    } else {
        1 + (2 * ((pass_idx - 10) / 3)) + (if ((pass_idx - 10) % 3) == 2 { 1 } else { 0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::j2c::codestream::CodeBlockStyle;
    use crate::writer::BitWriter;

    fn encode_ht_num_passes(num_passes: u8) -> Vec<u8> {
        let mut writer = BitWriter::new();

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
            _ => unreachable!(),
        }

        writer.finish()
    }

    #[test]
    fn test_code_block_style_detects_high_throughput() {
        let style = CodeBlockStyle {
            high_throughput_block_coding: true,
            ..Default::default()
        };
        assert!(style.uses_high_throughput_block_coding());

        let style = CodeBlockStyle::default();
        assert!(!style.uses_high_throughput_block_coding());
    }

    #[test]
    fn test_decode_num_ht_coding_passes_round_trip() {
        for num_passes in [1u8, 2, 3, 4, 5, 6, 19, 37, 38, 100, 164] {
            let data = encode_ht_num_passes(num_passes);
            let mut reader = BitReader::new(&data);

            assert_eq!(decode_num_ht_coding_passes(&mut reader), Some(num_passes));
        }
    }

    #[test]
    fn test_parse_ht_segment_lengths_folds_placeholder_passes() {
        let mut writer = BitWriter::new();
        writer.write_bit(0);
        writer.write_bits(5, 5);

        let data = writer.finish();
        let mut reader = BitReader::new(&data);
        let mut l_block = 3;

        let parsed = parse_ht_segment_lengths(&mut reader, 4, 2, &mut l_block).unwrap();

        assert_eq!(
            parsed,
            ParsedHtSegments {
                actual_passes: 1,
                missing_bit_planes: 3,
                cleanup_length: 5,
                refinement_length: 0,
            }
        );
        assert_eq!(l_block, 3);
    }

    #[test]
    fn test_parse_ht_segment_lengths_reads_refinement_segment() {
        let mut writer = BitWriter::new();
        writer.write_bits(0b110, 3);
        writer.write_bits(9, 5);
        writer.write_bits(17, 6);

        let data = writer.finish();
        let mut reader = BitReader::new(&data);
        let mut l_block = 3;

        let parsed = parse_ht_segment_lengths(&mut reader, 3, 1, &mut l_block).unwrap();

        assert_eq!(
            parsed,
            ParsedHtSegments {
                actual_passes: 3,
                missing_bit_planes: 1,
                cleanup_length: 9,
                refinement_length: 17,
            }
        );
        assert_eq!(l_block, 5);
    }
}
