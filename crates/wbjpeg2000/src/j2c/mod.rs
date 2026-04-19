mod arithmetic_decoder;
pub(crate) mod arithmetic_encoder;
mod bitplane;
pub(crate) mod bitplane_encode;
mod build;
mod codestream;
pub(crate) mod codestream_write;
mod decode;
pub(crate) mod encode;
pub(crate) mod fdwt;
pub(crate) mod forward_mct;
mod ht_block_decode;
mod ht_block_encode;
mod ht_encode_tables;
mod ht_tables;
mod idwt;
mod mct;
pub(crate) mod packet_encode;
mod progression;
pub(crate) mod quantize;
mod rect;
mod segment;
mod tag_tree;
pub(crate) mod tag_tree_encode;
mod tile;

use super::jp2::colr::{ColorSpace, ColorSpecificationBox, EnumeratedColorspace};
use super::jp2::ImageBoxes;
use crate::error::{bail, FormatError, MarkerError, Result};
use crate::j2c::codestream::markers;
use crate::reader::BitReader;
use crate::{resolve_alpha_and_color_space, DecodeSettings, Image};

use crate::math::{SimdBuffer, SIMD_WIDTH};
pub(crate) use codestream::Header;
pub(crate) use decode::decode;
pub use decode::DecoderContext;

pub(crate) struct ParsedCodestream<'a> {
    pub(crate) codestream: &'a [u8],
    pub(crate) header: Header<'a>,
    pub(crate) data: &'a [u8],
}

#[derive(Debug, Clone)]
pub(crate) struct ComponentData {
    pub(crate) container: SimdBuffer<{ SIMD_WIDTH }>,
    pub(crate) bit_depth: u8,
}

pub(crate) fn parse<'a>(stream: &'a [u8], settings: &DecodeSettings) -> Result<Image<'a>> {
    let parsed_codestream = parse_raw(stream, settings)?;
    let header = &parsed_codestream.header;
    let mut boxes = ImageBoxes::default();

    // If we are just decoding a raw codestream, we assume greyscale or
    // RGB.
    let cs = if header.component_infos.len() < 3 {
        ColorSpace::Enumerated(EnumeratedColorspace::Greyscale)
    } else {
        ColorSpace::Enumerated(EnumeratedColorspace::Srgb)
    };

    boxes.color_specification = Some(ColorSpecificationBox { color_space: cs });

    let (color_space, has_alpha) =
        resolve_alpha_and_color_space(&boxes, &parsed_codestream.header, settings)?;
    let uses_openjph_htj2k = parsed_codestream
        .header
        .component_infos
        .iter()
        .any(|info| info.code_block_style().uses_high_throughput_block_coding());

    Ok(Image {
        codestream: parsed_codestream.data,
        encoded_codestream: parsed_codestream.codestream,
        header: parsed_codestream.header,
        boxes,
        settings: *settings,
        uses_openjph_htj2k,
        color_space,
        has_alpha,
    })
}

pub(crate) fn parse_raw<'a>(
    stream: &'a [u8],
    settings: &DecodeSettings,
) -> Result<ParsedCodestream<'a>> {
    let mut reader = BitReader::new(stream);

    let marker = reader.read_marker()?;
    if marker != markers::SOC {
        bail!(MarkerError::Expected("SOC"));
    }

    let header = codestream::read_header(&mut reader, settings)?;
    let code_stream_data = reader.tail().ok_or(FormatError::MissingCodestream)?;

    Ok(ParsedCodestream {
        codestream: stream,
        header,
        data: code_stream_data,
    })
}
