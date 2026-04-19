#[cfg(feature = "openjph-htj2k")]
use alloc::vec::Vec;

#[cfg(feature = "openjph-htj2k")]
use openjph_core::codestream::Codestream;
#[cfg(feature = "openjph-htj2k")]
use openjph_core::file::{MemInfile, MemOutfile};
#[cfg(feature = "openjph-htj2k")]
use openjph_core::types::{Point, Size};

#[cfg(feature = "openjph-htj2k")]
use crate::j2c::encode::EncodeOptions;
#[cfg(feature = "openjph-htj2k")]
use crate::RawBitmap;

#[cfg(feature = "openjph-htj2k")]
pub(crate) fn encode(
    pixels: &[u8],
    width: u32,
    height: u32,
    num_components: u8,
    bit_depth: u8,
    signed: bool,
    options: &EncodeOptions,
) -> Result<Vec<u8>, &'static str> {
    let mut cs = Codestream::new();
    cs.access_siz_mut()
        .set_image_extent(Point::new(width, height));
    cs.access_siz_mut().set_tile_size(Size::new(width, height));
    cs.access_siz_mut()
        .set_num_components(u32::from(num_components));
    for comp_idx in 0..u32::from(num_components) {
        cs.access_siz_mut()
            .set_comp_info(comp_idx, Point::new(1, 1), u32::from(bit_depth), signed);
    }
    cs.access_cod_mut()
        .set_num_decomposition(u32::from(options.num_decomposition_levels));
    cs.access_cod_mut().set_reversible(options.reversible);
    cs.access_cod_mut().set_color_transform(num_components >= 3);
    let default_options = EncodeOptions::default();
    let (block_width, block_height) = if options.code_block_width_exp
        == default_options.code_block_width_exp
        && options.code_block_height_exp == default_options.code_block_height_exp
    {
        // OpenJPH's own roundtrip coverage is strongest on the HTJ2K 4x1024 profile,
        // and real 12-bit DICOM slices roundtrip exactly with it.
        (4, 1024)
    } else {
        (
            1u32 << (u32::from(options.code_block_width_exp) + 2),
            1u32 << (u32::from(options.code_block_height_exp) + 2),
        )
    };
    cs.access_cod_mut()
        .set_block_dims(block_width, block_height);
    cs.set_planar(0);

    let mut outfile = MemOutfile::with_capacity(pixels.len());
    cs.write_headers(&mut outfile, &[])
        .map_err(|_| "OpenJPH HTJ2K header write failed")?;

    let width = width as usize;
    let num_components = usize::from(num_components);
    let mut lines = vec![vec![0i32; width]; num_components];

    for y in 0..height as usize {
        let row_start = y * width;
        for x in 0..width {
            let pixel_index = row_start + x;
            let sample_base = pixel_index * num_components;
            for (comp_idx, line) in lines.iter_mut().enumerate() {
                line[x] = decode_input_sample(pixels, sample_base + comp_idx, bit_depth, signed)?;
            }
        }

        for (comp_idx, line) in lines.iter().enumerate() {
            cs.exchange(line, comp_idx as u32)
                .map_err(|_| "OpenJPH HTJ2K line exchange failed")?;
        }
    }

    cs.flush(&mut outfile)
        .map_err(|_| "OpenJPH HTJ2K flush failed")?;
    Ok(outfile.get_data().to_vec())
}

#[cfg(feature = "openjph-htj2k")]
pub(crate) fn decode_native(
    codestream: &[u8],
    width: u32,
    height: u32,
    bit_depth: u8,
    num_components: u8,
    skipped_resolution_levels: u8,
) -> Result<RawBitmap, &'static str> {
    let mut infile = MemInfile::new(codestream);
    let mut cs = Codestream::new();
    cs.read_headers(&mut infile)
        .map_err(|_| "OpenJPH HTJ2K header read failed")?;

    let signed = cs.access_siz().is_signed(0);
    let actual_components = cs.access_siz().get_num_components() as u8;
    let actual_bit_depth = cs.access_siz().get_bit_depth(0) as u8;
    if actual_components != num_components || actual_bit_depth != bit_depth {
        return Err("OpenJPH HTJ2K metadata mismatch");
    }

    if skipped_resolution_levels != 0 {
        cs.restrict_input_resolution(
            u32::from(skipped_resolution_levels),
            u32::from(skipped_resolution_levels),
        );
    }
    cs.create(&mut infile)
        .map_err(|_| "OpenJPH HTJ2K codestream decode failed")?;

    let width_usize = width as usize;
    let height_usize = height as usize;
    let num_components_usize = usize::from(num_components);
    let bytes_per_sample = if bit_depth <= 8 { 1 } else { 2 };
    let mut data =
        Vec::with_capacity(width_usize * height_usize * num_components_usize * bytes_per_sample);

    for _ in 0..height_usize {
        let mut component_lines = Vec::with_capacity(num_components_usize);
        for comp_idx in 0..u32::from(num_components) {
            let line = cs.pull(comp_idx).ok_or("OpenJPH HTJ2K line pull failed")?;
            if line.len() != width_usize {
                return Err("OpenJPH HTJ2K decoded line width mismatch");
            }
            component_lines.push(line);
        }

        for x in 0..width_usize {
            for line in &component_lines {
                append_output_sample(&mut data, line[x], bit_depth, signed);
            }
        }
    }

    Ok(RawBitmap {
        data,
        width,
        height,
        bit_depth,
        num_components,
        bytes_per_sample: bytes_per_sample as u8,
    })
}

#[cfg(feature = "openjph-htj2k")]
fn decode_input_sample(
    pixels: &[u8],
    sample_index: usize,
    bit_depth: u8,
    signed: bool,
) -> Result<i32, &'static str> {
    let byte_offset = sample_index * if bit_depth <= 8 { 1 } else { 2 };
    if bit_depth <= 8 {
        let raw = u32::from(
            *pixels
                .get(byte_offset)
                .ok_or("OpenJPH HTJ2K pixel data too short")?,
        );
        let masked = raw & bit_mask(bit_depth);
        Ok(if signed {
            sign_extend(masked, bit_depth)
        } else {
            masked as i32
        })
    } else {
        let lo = *pixels
            .get(byte_offset)
            .ok_or("OpenJPH HTJ2K pixel data too short")?;
        let hi = *pixels
            .get(byte_offset + 1)
            .ok_or("OpenJPH HTJ2K pixel data too short")?;
        let raw = u32::from(u16::from_le_bytes([lo, hi]));
        let masked = raw & bit_mask(bit_depth);
        Ok(if signed {
            sign_extend(masked, bit_depth)
        } else {
            masked as i32
        })
    }
}

#[cfg(feature = "openjph-htj2k")]
fn append_output_sample(out: &mut Vec<u8>, sample: i32, bit_depth: u8, signed: bool) {
    if signed {
        let min = -(1i32 << (u32::from(bit_depth) - 1));
        let max = (1i32 << (u32::from(bit_depth) - 1)) - 1;
        let sample = sample.clamp(min, max);
        if bit_depth <= 8 {
            out.push(sample as i8 as u8);
        } else {
            out.extend_from_slice(&(sample as i16).to_le_bytes());
        }
    } else {
        let max = bit_mask(bit_depth) as i32;
        let sample = sample.clamp(0, max);
        if bit_depth <= 8 {
            out.push(sample as u8);
        } else {
            out.extend_from_slice(&(sample as u16).to_le_bytes());
        }
    }
}

#[cfg(feature = "openjph-htj2k")]
fn sign_extend(raw: u32, bit_depth: u8) -> i32 {
    let shift = 32 - u32::from(bit_depth);
    ((raw << shift) as i32) >> shift
}

#[cfg(feature = "openjph-htj2k")]
fn bit_mask(bit_depth: u8) -> u32 {
    (1u32 << u32::from(bit_depth)) - 1
}
