use wbraster::{DataType, Raster};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RgbMode {
    None,
    Packed,
    ThreeBand,
}

fn metadata_value_case_insensitive<'a>(
    metadata: &'a [(String, String)],
    key: &str,
) -> Option<&'a str> {
    metadata
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.as_str())
}

fn explicit_rgb_mode(input: &Raster) -> Option<RgbMode> {
    let color_interp = metadata_value_case_insensitive(&input.metadata, "color_interpretation")?;

    if color_interp.eq_ignore_ascii_case("packed_rgb") {
        return Some(RgbMode::Packed);
    }

    if color_interp.eq_ignore_ascii_case("rgb")
        || color_interp.eq_ignore_ascii_case("rgba")
        || color_interp.eq_ignore_ascii_case("ycbcr")
    {
        if input.bands >= 3 {
            return Some(RgbMode::ThreeBand);
        }
        if input.bands == 1 && input.data_type == DataType::U32 {
            return Some(RgbMode::Packed);
        }
    }

    Some(RgbMode::None)
}

fn metadata_says_multiband(input: &Raster) -> bool {
    let color_interp_multiband = metadata_value_case_insensitive(&input.metadata, "color_interpretation")
        .map(|v| v.eq_ignore_ascii_case("multiband"))
        .unwrap_or(false);

    let jpeg2000_multiband = metadata_value_case_insensitive(&input.metadata, "jpeg2000_color_space")
        .map(|v| v.eq_ignore_ascii_case("multiband"))
        .unwrap_or(false);

    color_interp_multiband || jpeg2000_multiband
}

fn is_reasonable_three_band_rgb_candidate(input: &Raster) -> bool {
    input.bands == 3
        && matches!(input.data_type, DataType::U8 | DataType::U16)
        && !metadata_says_multiband(input)
}

/// Detect RGB interpretation mode for a raster.
///
/// Priority order:
/// 1) Explicit caller override (`force_rgb`).
/// 2) Explicit standardized metadata (`color_interpretation`).
/// 3) Optional heuristic fallback for common 3-band byte/uint16 imagery.
pub(crate) fn detect_rgb_mode(
    input: &Raster,
    force_rgb: bool,
    allow_three_band_heuristic: bool,
) -> RgbMode {
    if force_rgb {
        if input.bands == 1 && input.data_type == DataType::U32 {
            return RgbMode::Packed;
        }
        if input.bands >= 3 {
            return RgbMode::ThreeBand;
        }
    }

    if let Some(mode) = explicit_rgb_mode(input) {
        if mode != RgbMode::None {
            return mode;
        }
    }

    if allow_three_band_heuristic && is_reasonable_three_band_rgb_candidate(input) {
        return RgbMode::ThreeBand;
    }

    RgbMode::None
}
