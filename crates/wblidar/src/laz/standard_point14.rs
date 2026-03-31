//! Standard LASzip layered Point14-family (v3) decoding groundwork.

use std::io::{Cursor, ErrorKind, Read, Write};

use crate::las::header::PointDataFormat;
use crate::laz::arithmetic_decoder::ArithmeticDecoder;
use crate::laz::arithmetic_encoder::ArithmeticEncoder;
use crate::laz::arithmetic_model::ArithmeticSymbolModel;
use crate::laz::fields::point14::RawPoint14;
use crate::laz::integer_codec::IntegerCompressor;
use crate::laz::integer_codec::IntegerDecompressor;
use crate::laz::LaszipItemSpec;
use crate::point::{GpsTime, Rgb16, WaveformPacket};
use crate::{Error, PointRecord, Result};

const LASZIP_ITEM_BYTE: u16 = 0;
const LASZIP_ITEM_POINT14: u16 = 10;
const LASZIP_ITEM_RGB14: u16 = 11;
const LASZIP_ITEM_RGBNIR14: u16 = 12;
const LASZIP_ITEM_WAVEPACKET14: u16 = 13;
const LASZIP_ITEM_BYTE14: u16 = 14;

const LASZIP_GPS_TIME_MULTI: i32 = 500;
const LASZIP_GPS_TIME_MULTI_MINUS: i32 = -10;
const LASZIP_GPS_TIME_MULTI_CODE_FULL: i32 =
    LASZIP_GPS_TIME_MULTI - LASZIP_GPS_TIME_MULTI_MINUS + 1;
const LASZIP_GPS_TIME_MULTI_TOTAL: i32 = LASZIP_GPS_TIME_MULTI - LASZIP_GPS_TIME_MULTI_MINUS + 5;

const NUMBER_RETURN_MAP_6CTX: [[u8; 16]; 16] = [
    [0, 1, 2, 3, 4, 5, 3, 4, 4, 5, 5, 5, 5, 5, 5, 5],
    [1, 0, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3],
    [2, 1, 2, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3],
    [3, 3, 4, 5, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4],
    [4, 3, 4, 4, 5, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4],
    [5, 3, 4, 4, 4, 5, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4],
    [3, 3, 4, 4, 4, 4, 5, 4, 4, 4, 4, 4, 4, 4, 4, 4],
    [4, 3, 4, 4, 4, 4, 4, 5, 4, 4, 4, 4, 4, 4, 4, 4],
    [4, 3, 4, 4, 4, 4, 4, 4, 5, 4, 4, 4, 4, 4, 4, 4],
    [5, 3, 4, 4, 4, 4, 4, 4, 4, 5, 4, 4, 4, 4, 4, 4],
    [5, 3, 4, 4, 4, 4, 4, 4, 4, 4, 5, 4, 4, 4, 4, 4],
    [5, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 4, 4, 4],
    [5, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5, 4, 4],
    [5, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5, 4],
    [5, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5],
    [5, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5],
];

const NUMBER_RETURN_LEVEL_8CTX: [[u8; 16]; 16] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 7, 7, 7, 7, 7, 7, 7, 7],
    [1, 0, 1, 2, 3, 4, 5, 6, 7, 7, 7, 7, 7, 7, 7, 7],
    [2, 1, 0, 1, 2, 3, 4, 5, 6, 7, 7, 7, 7, 7, 7, 7],
    [3, 2, 1, 0, 1, 2, 3, 4, 5, 6, 7, 7, 7, 7, 7, 7],
    [4, 3, 2, 1, 0, 1, 2, 3, 4, 5, 6, 7, 7, 7, 7, 7],
    [5, 4, 3, 2, 1, 0, 1, 2, 3, 4, 5, 6, 7, 7, 7, 7],
    [6, 5, 4, 3, 2, 1, 0, 1, 2, 3, 4, 5, 6, 7, 7, 7],
    [7, 6, 5, 4, 3, 2, 1, 0, 1, 2, 3, 4, 5, 6, 7, 7],
    [7, 7, 6, 5, 4, 3, 2, 1, 0, 1, 2, 3, 4, 5, 6, 7],
    [7, 7, 7, 6, 5, 4, 3, 2, 1, 0, 1, 2, 3, 4, 5, 6],
    [7, 7, 7, 7, 6, 5, 4, 3, 2, 1, 0, 1, 2, 3, 4, 5],
    [7, 7, 7, 7, 7, 6, 5, 4, 3, 2, 1, 0, 1, 2, 3, 4],
    [7, 7, 7, 7, 7, 7, 6, 5, 4, 3, 2, 1, 0, 1, 2, 3],
    [7, 7, 7, 7, 7, 7, 7, 6, 5, 4, 3, 2, 1, 0, 1, 2],
    [7, 7, 7, 7, 7, 7, 7, 7, 6, 5, 4, 3, 2, 1, 0, 1],
    [7, 7, 7, 7, 7, 7, 7, 7, 7, 6, 5, 4, 3, 2, 1, 0],
];

#[derive(Clone, Copy, Default)]
struct StreamingMedianI32 {
    values: [i32; 5],
    high: bool,
}

impl StreamingMedianI32 {
    fn new() -> Self {
        Self {
            values: [0; 5],
            high: true,
        }
    }

    fn add(&mut self, v: i32) {
        if self.high {
            if v < self.values[2] {
                self.values[4] = self.values[3];
                self.values[3] = self.values[2];
                if v < self.values[0] {
                    self.values[2] = self.values[1];
                    self.values[1] = self.values[0];
                    self.values[0] = v;
                } else if v < self.values[1] {
                    self.values[2] = self.values[1];
                    self.values[1] = v;
                } else {
                    self.values[2] = v;
                }
            } else {
                if v < self.values[3] {
                    self.values[4] = self.values[3];
                    self.values[3] = v;
                } else {
                    self.values[4] = v;
                }
                self.high = false;
            }
        } else if self.values[2] < v {
            self.values[0] = self.values[1];
            self.values[1] = self.values[2];
            if self.values[4] < v {
                self.values[2] = self.values[3];
                self.values[3] = self.values[4];
                self.values[4] = v;
            } else if self.values[3] < v {
                self.values[2] = self.values[3];
                self.values[3] = v;
            } else {
                self.values[2] = v;
            }
        } else {
            if self.values[1] < v {
                self.values[0] = self.values[1];
                self.values[1] = v;
            } else {
                self.values[0] = v;
            }
            self.high = true;
        }
    }

    fn get(&self) -> i32 {
        self.values[2]
    }
}

#[inline]
fn u32_zero_bit(n: u32) -> u32 {
    n & 0xFF_FF_FF_FEu32
}

#[inline]
fn point14_flags_layer_symbol(flags_byte: u8) -> u8 {
    ((flags_byte & 0x80) >> 2) | ((flags_byte & 0x40) >> 2) | (flags_byte & 0x0F)
}

#[inline]
fn point14_scanner_channel_bits(flags_byte: u8) -> u8 {
    flags_byte & 0x30
}

#[inline]
fn point14_scanner_channel_index(flags_byte: u8) -> usize {
    ((flags_byte >> 4) & 0x03) as usize
}

struct ScannerChannelSubsetContext {
    x: i32,
    y: i32,
    last_x_diff_median: [StreamingMedianI32; 12],
    last_y_diff_median: [StreamingMedianI32; 12],
    last_z: [i32; 8],
    last_intensity: [u16; 8],
    m_changed_values: [ArithmeticSymbolModel; 8],
    m_scanner_channel: ArithmeticSymbolModel,
    ic_dx: IntegerCompressor,
    ic_dy: IntegerCompressor,
    ic_z: IntegerCompressor,
    ic_intensity: IntegerCompressor,
}

impl ScannerChannelSubsetContext {
    fn new(seed_x: i32, seed_y: i32, seed_z: i32, seed_intensity: u16) -> Self {
        Self {
            x: seed_x,
            y: seed_y,
            last_x_diff_median: [StreamingMedianI32::new(); 12],
            last_y_diff_median: [StreamingMedianI32::new(); 12],
            last_z: [seed_z; 8],
            last_intensity: [seed_intensity; 8],
            m_changed_values: std::array::from_fn(|_| ArithmeticSymbolModel::new(128)),
            m_scanner_channel: ArithmeticSymbolModel::new(3),
            ic_dx: IntegerCompressor::new(32, 2, 8, 0),
            ic_dy: IntegerCompressor::new(32, 22, 8, 0),
            ic_z: IntegerCompressor::new(32, 20, 8, 0),
            ic_intensity: IntegerCompressor::new(16, 4, 8, 0),
        }
    }
}

struct Byte14EncoderChannelState {
    last_item: Vec<u8>,
    models: Vec<ArithmeticSymbolModel>,
}

fn normalize_point14_base_format(point_data_format: PointDataFormat) -> Result<PointDataFormat> {
    match point_data_format {
        // LAS 1.4 base formats
        PointDataFormat::Pdrf6 | PointDataFormat::Pdrf9 => Ok(PointDataFormat::Pdrf6),
        PointDataFormat::Pdrf7 | PointDataFormat::Pdrf10 => Ok(PointDataFormat::Pdrf7),
        PointDataFormat::Pdrf8 => Ok(PointDataFormat::Pdrf8),
        // LAS 1.5 formats (map to equivalent v1.4 base structures)
        PointDataFormat::Pdrf11 => Ok(PointDataFormat::Pdrf6),
        PointDataFormat::Pdrf12 => Ok(PointDataFormat::Pdrf7),
        PointDataFormat::Pdrf13 => Ok(PointDataFormat::Pdrf8),
        PointDataFormat::Pdrf14 => Ok(PointDataFormat::Pdrf7),
        PointDataFormat::Pdrf15 => Ok(PointDataFormat::Pdrf8),
        _ => Err(Error::InvalidValue {
            field: "laz.standard_point14_writer.point_data_format",
            detail: format!(
                "expected PDRF6/7/8/9/10 or v1.5 PDRF11/12/13/14/15 for Point14 encoding, found {:?}",
                point_data_format
            ),
        }),
    }
}

fn encode_waveform_bytes(point: &PointRecord) -> [u8; 29] {
    let wf = point.waveform.unwrap_or_default();
    let mut out = [0u8; 29];
    out[0] = wf.descriptor_index;
    out[1..9].copy_from_slice(&wf.byte_offset.to_le_bytes());
    out[9..13].copy_from_slice(&wf.packet_size.to_le_bytes());
    out[13..17].copy_from_slice(&wf.return_point_location.to_le_bytes());
    out[17..21].copy_from_slice(&wf.dx.to_le_bytes());
    out[21..25].copy_from_slice(&wf.dy.to_le_bytes());
    out[25..29].copy_from_slice(&wf.dz.to_le_bytes());
    out
}

fn collect_point14_payload_bytes(points: &[PointRecord], point_data_format: PointDataFormat) -> Result<Option<Vec<Vec<u8>>>> {
    if points.is_empty() {
        return Ok(None);
    }

    let expected_extra_len = points[0].extra_bytes.len as usize;
    let mut out = Vec::with_capacity(points.len());
    for (point_index, point) in points.iter().enumerate() {
        let actual_len = point.extra_bytes.len as usize;
        if actual_len != expected_extra_len {
            return Err(Error::InvalidValue {
                field: "laz.standard_point14_writer.extra_bytes",
                detail: format!(
                    "point {} extra-bytes length {} does not match chunk seed extra-bytes length {}",
                    point_index, actual_len, expected_extra_len
                ),
            });
        }

        let mut bytes = Vec::with_capacity(expected_extra_len + if point_data_format.has_waveform() { 29 } else { 0 });
        if point_data_format.has_waveform() {
            bytes.extend_from_slice(&encode_waveform_bytes(point));
        }
        if expected_extra_len > 0 {
            bytes.extend_from_slice(&point.extra_bytes.data[..expected_extra_len]);
        }
        out.push(bytes);
    }

    if out.first().map_or(0, Vec::len) == 0 {
        Ok(None)
    } else {
        Ok(Some(out))
    }
}

fn serialize_point14_seed_item_set(
    seed: RawPoint14,
    point_data_format: PointDataFormat,
    seed_extra_bytes: Option<&[u8]>,
) -> Result<Vec<u8>> {
    let mut out = seed.to_bytes(PointDataFormat::Pdrf6).ok_or_else(|| Error::InvalidValue {
        field: "laz.standard_point14_writer.serialize",
        detail: "failed to serialize Point14 core seed point".to_string(),
    })?;

    match point_data_format {
        PointDataFormat::Pdrf6 | PointDataFormat::Pdrf11 => {}
        PointDataFormat::Pdrf7 | PointDataFormat::Pdrf12 | PointDataFormat::Pdrf14 => {
            let rgb = seed.rgb.ok_or_else(|| Error::InvalidValue {
                field: "laz.standard_point14_writer.serialize",
                detail: format!("missing RGB seed payload for {:?}", point_data_format),
            })?;
            out.extend_from_slice(&rgb.red.to_le_bytes());
            out.extend_from_slice(&rgb.green.to_le_bytes());
            out.extend_from_slice(&rgb.blue.to_le_bytes());
        }
        PointDataFormat::Pdrf8 | PointDataFormat::Pdrf13 | PointDataFormat::Pdrf15 => {
            let rgb = seed.rgb.ok_or_else(|| Error::InvalidValue {
                field: "laz.standard_point14_writer.serialize",
                detail: format!("missing RGB seed payload for {:?}", point_data_format),
            })?;
            let nir = seed.nir.ok_or_else(|| Error::InvalidValue {
                field: "laz.standard_point14_writer.serialize",
                detail: format!("missing NIR seed payload for {:?}", point_data_format),
            })?;
            out.extend_from_slice(&rgb.red.to_le_bytes());
            out.extend_from_slice(&rgb.green.to_le_bytes());
            out.extend_from_slice(&rgb.blue.to_le_bytes());
            out.extend_from_slice(&nir.to_le_bytes());
            // Note: TiRGB fields (thermal, additional R/G/B) are handled as extra bytes
        }
        _ => {
            return Err(Error::InvalidValue {
                field: "laz.standard_point14_writer.point_data_format",
                detail: format!(
                    "expected PDRF6/7/8 or v1.5 PDRF11/12/13/14/15 for Point14 seed serialization, found {:?}",
                    point_data_format
                ),
            });
        }
    }

    if let Some(extra) = seed_extra_bytes {
        out.extend_from_slice(extra);
    }

    Ok(out)
}

fn encode_point14_byte14_layers(
    extra_bytes_per_point: &[Vec<u8>],
    point_channels: &[usize],
    seed_channel: usize,
) -> Result<Vec<Vec<u8>>> {
    if extra_bytes_per_point.is_empty() {
        return Ok(Vec::new());
    }

    let extra_byte_count = extra_bytes_per_point[0].len();
    if extra_byte_count == 0 {
        return Ok(Vec::new());
    }
    if point_channels.len() + 1 != extra_bytes_per_point.len() {
        return Err(Error::InvalidValue {
            field: "laz.standard_point14_writer.extra_bytes",
            detail: format!(
                "extra-bytes point count {} does not align with point-channel continuation count {}",
                extra_bytes_per_point.len(),
                point_channels.len()
            ),
        });
    }

    let mut out = Vec::with_capacity(extra_byte_count);
    for byte_idx in 0..extra_byte_count {
        let mut writer = Cursor::new(Vec::<u8>::new());
        let mut enc = ArithmeticEncoder::new(&mut writer);
        let mut changed = false;
        let mut current_channel = seed_channel;
        let mut channel_states: [Option<Byte14EncoderChannelState>; 4] =
            std::array::from_fn(|_| None);
        channel_states[seed_channel] = Some(Byte14EncoderChannelState {
            last_item: extra_bytes_per_point[0].clone(),
            models: (0..extra_byte_count)
                .map(|_| ArithmeticSymbolModel::new(256))
                .collect(),
        });

        for (point_index, point_extra) in extra_bytes_per_point.iter().enumerate().skip(1) {
            let target_channel = point_channels[point_index - 1];
            if target_channel != current_channel {
                if channel_states[target_channel].is_none() {
                    let seed_item = channel_states[current_channel]
                        .as_ref()
                        .ok_or_else(|| Error::InvalidValue {
                            field: "laz.standard_point14_writer.extra_bytes",
                            detail: "missing source BYTE14 scanner-channel context"
                                .to_string(),
                        })?
                        .last_item
                        .clone();
                    channel_states[target_channel] = Some(Byte14EncoderChannelState {
                        last_item: seed_item,
                        models: (0..extra_byte_count)
                            .map(|_| ArithmeticSymbolModel::new(256))
                            .collect(),
                    });
                }
                current_channel = target_channel;
            }

            let state = channel_states[current_channel]
                .as_mut()
                .ok_or_else(|| Error::InvalidValue {
                    field: "laz.standard_point14_writer.extra_bytes",
                    detail: "missing destination BYTE14 scanner-channel context"
                        .to_string(),
                })?;
            let last = state.last_item[byte_idx];
            let value = point_extra[byte_idx];
            let diff = value.wrapping_sub(last) as u32;
            enc.encode_symbol(&mut state.models[byte_idx], diff)
                .map_err(Error::Io)?;
            if value != last {
                changed = true;
                state.last_item[byte_idx] = value;
            }
        }

        let _ = enc.done().map_err(Error::Io)?;
        out.push(if changed { writer.into_inner() } else { Vec::new() });
    }

    Ok(out)
}

fn encode_standard_layered_chunk_point14_v3_scanner_channel_subset(
    raws: &[RawPoint14],
    extra_bytes_per_point: Option<&[Vec<u8>]>,
    point_data_format: PointDataFormat,
) -> Result<Vec<u8>> {
    let seed = raws[0];
    let seed_channel = point14_scanner_channel_index(seed.flags_byte);
    let seed_n = ((seed.return_byte >> 4) & 0x0F) as usize;
    let seed_r = (seed.return_byte & 0x0F) as usize;

    let mut contexts: [Option<ScannerChannelSubsetContext>; 4] = std::array::from_fn(|_| None);
    contexts[seed_channel] = Some(ScannerChannelSubsetContext::new(
        seed.xi,
        seed.yi,
        seed.zi,
        seed.intensity,
    ));
    let mut current_channel = seed_channel;
    let mut channel_last_intensity = [seed.intensity; 4];
    let mut channel_last_n = [seed_n; 4];
    let mut channel_last_r = [seed_r; 4];
    let mut channel_last_classification = [seed.classification; 4];
    let mut channel_last_flags_symbol = [point14_flags_layer_symbol(seed.flags_byte); 4];
    let mut channel_last_user_data = [seed.user_data; 4];
    let mut channel_last_scan_angle = [seed.scan_angle; 4];
    let mut channel_last_point_source = [seed.point_source_id; 4];
    let mut channel_last_gps_bits = [seed.gps_time.to_bits() as i64; 4];
    let mut channel_last_gps_time_change = [false; 4];
    let mut channel_last_rgb = [seed.rgb.unwrap_or(Rgb16 { red: 0, green: 0, blue: 0 }); 4];
    let mut channel_last_nir = [seed.nir.unwrap_or(0); 4];

    let mut xy_writer = std::io::Cursor::new(Vec::<u8>::new());
    let mut k_bits_per_point = Vec::<u32>::with_capacity(raws.len() - 1);
    let mut l_per_point = Vec::<usize>::with_capacity(raws.len() - 1);
    let mut cpr_per_point = Vec::<usize>::with_capacity(raws.len() - 1);
    let mut n_per_point = Vec::<usize>::with_capacity(raws.len() - 1);
    let mut point_channel_per_point = Vec::<usize>::with_capacity(raws.len() - 1);
    let mut gps_time_change_per_point = Vec::<bool>::with_capacity(raws.len() - 1);
    let mut number_of_returns_models: [[Option<ArithmeticSymbolModel>; 16]; 4] =
        std::array::from_fn(|_| std::array::from_fn(|_| None));
    let mut return_number_models: [[Option<ArithmeticSymbolModel>; 16]; 4] =
        std::array::from_fn(|_| std::array::from_fn(|_| None));
    let mut return_number_gps_same_models: [ArithmeticSymbolModel; 4] =
        std::array::from_fn(|_| ArithmeticSymbolModel::new(13));
    {
        let mut enc = ArithmeticEncoder::new(&mut xy_writer);
        for raw in raws.iter().skip(1) {
            let target_channel = point14_scanner_channel_index(raw.flags_byte);
            let switch_channels = target_channel != current_channel;
            let target_has_context = contexts[target_channel].is_some();

            let (seed_x_for_new, seed_y_for_new, seed_z_for_new, seed_intensity_for_new, seed_n_for_new, seed_r_for_new, seed_classification_for_new, seed_flags_symbol_for_new, seed_user_data_for_new, seed_scan_angle_for_new, seed_point_source_for_new, seed_gps_bits_for_new, seed_rgb_for_new, seed_nir_for_new) = {
                let src_ctx = contexts[current_channel].as_ref().ok_or_else(|| {
                    Error::InvalidValue {
                        field: "laz.standard_point14_writer.scanner_channel",
                        detail: "missing source scanner channel context".to_string(),
                    }
                })?;
                let src_n = channel_last_n[current_channel];
                let src_r = channel_last_r[current_channel];
                let src_l = NUMBER_RETURN_LEVEL_8CTX[src_n][src_r] as usize;
                (
                    src_ctx.x,
                    src_ctx.y,
                    src_ctx.last_z[src_l],
                    channel_last_intensity[current_channel],
                    src_n,
                    src_r,
                    channel_last_classification[current_channel],
                    channel_last_flags_symbol[current_channel],
                    channel_last_user_data[current_channel],
                    channel_last_scan_angle[current_channel],
                    channel_last_point_source[current_channel],
                    channel_last_gps_bits[current_channel],
                    channel_last_rgb[current_channel],
                    channel_last_nir[current_channel],
                )
            };

            {
                let src_ctx = contexts[current_channel].as_mut().ok_or_else(|| {
                    Error::InvalidValue {
                        field: "laz.standard_point14_writer.scanner_channel",
                        detail: "missing source scanner channel context".to_string(),
                    }
                })?;
                let target_last_scan_angle = if switch_channels {
                    if target_has_context {
                        channel_last_scan_angle[target_channel]
                    } else {
                        seed_scan_angle_for_new
                    }
                } else {
                    channel_last_scan_angle[current_channel]
                };
                let target_last_point_source = if switch_channels {
                    if target_has_context {
                        channel_last_point_source[target_channel]
                    } else {
                        seed_point_source_for_new
                    }
                } else {
                    channel_last_point_source[current_channel]
                };
                let target_last_gps_bits = if switch_channels {
                    if target_has_context {
                        channel_last_gps_bits[target_channel]
                    } else {
                        seed_gps_bits_for_new
                    }
                } else {
                    channel_last_gps_bits[current_channel]
                };
                let target_last_n = if switch_channels {
                    if target_has_context {
                        channel_last_n[target_channel]
                    } else {
                        seed_n_for_new
                    }
                } else {
                    channel_last_n[current_channel]
                };
                let target_last_r = if switch_channels {
                    if target_has_context {
                        channel_last_r[target_channel]
                    } else {
                        seed_r_for_new
                    }
                } else {
                    channel_last_r[current_channel]
                };
                let n = ((raw.return_byte >> 4) & 0x0F) as usize;
                let r = (raw.return_byte & 0x0F) as usize;
                let scan_angle_change = raw.scan_angle != target_last_scan_angle;
                let point_source_change = raw.point_source_id != target_last_point_source;
                let gps_time_change = (raw.gps_time.to_bits() as i64) != target_last_gps_bits;
                let mut changed_values = 0u32;
                if switch_channels {
                    changed_values |= 1 << 6;
                }
                if point_source_change {
                    changed_values |= 1 << 5;
                }
                if gps_time_change {
                    changed_values |= 1 << 4;
                }
                if scan_angle_change {
                    changed_values |= 1 << 3;
                }
                if n != target_last_n {
                    changed_values |= 1 << 2;
                }
                if r == target_last_r {
                    // no-op: low bits remain 0
                } else if r == ((target_last_r + 1) & 0x0F) {
                    changed_values |= 1;
                } else if r == ((target_last_r + 15) & 0x0F) {
                    changed_values |= 2;
                } else {
                    changed_values |= 3;
                }

                let src_n = channel_last_n[current_channel];
                let src_r = channel_last_r[current_channel];
                let src_lpr = (if src_r == 1 { 1 } else { 0 }) + if src_r >= src_n { 2 } else { 0 };
                let lpr_idx = src_lpr + if channel_last_gps_time_change[current_channel] { 4 } else { 0 };
                enc.encode_symbol(&mut src_ctx.m_changed_values[lpr_idx], changed_values)
                    .map_err(Error::Io)?;

                if switch_channels {
                    let diff = (target_channel + 4 - current_channel - 1) & 0x03;
                    if diff > 2 {
                        return Err(Error::Unimplemented(
                            "scanner-channel subset encountered unsupported channel jump",
                        ));
                    }
                    enc.encode_symbol(&mut src_ctx.m_scanner_channel, diff as u32)
                        .map_err(Error::Io)?;
                }
            }

            if switch_channels {
                if contexts[target_channel].is_none() {
                    contexts[target_channel] = Some(ScannerChannelSubsetContext::new(
                        seed_x_for_new,
                        seed_y_for_new,
                        seed_z_for_new,
                        seed_intensity_for_new,
                    ));
                    channel_last_classification[target_channel] = seed_classification_for_new;
                    channel_last_flags_symbol[target_channel] = seed_flags_symbol_for_new;
                    channel_last_user_data[target_channel] = seed_user_data_for_new;
                    channel_last_scan_angle[target_channel] = seed_scan_angle_for_new;
                    channel_last_point_source[target_channel] = seed_point_source_for_new;
                    channel_last_n[target_channel] = seed_n_for_new;
                    channel_last_r[target_channel] = seed_r_for_new;
                    channel_last_gps_bits[target_channel] = seed_gps_bits_for_new;
                    channel_last_gps_time_change[target_channel] = false;
                    channel_last_rgb[target_channel] = seed_rgb_for_new;
                    channel_last_nir[target_channel] = seed_nir_for_new;
                }
                current_channel = target_channel;
            }

            let n = ((raw.return_byte >> 4) & 0x0F) as usize;
            let r = (raw.return_byte & 0x0F) as usize;
            let last_n = channel_last_n[current_channel];
            let last_r = channel_last_r[current_channel];
            let gps_time_change = (raw.gps_time.to_bits() as i64) != channel_last_gps_bits[current_channel];

            if n != last_n {
                if number_of_returns_models[current_channel][last_n].is_none() {
                    number_of_returns_models[current_channel][last_n] = Some(ArithmeticSymbolModel::new(16));
                }
                enc.encode_symbol(
                    number_of_returns_models[current_channel][last_n]
                        .as_mut()
                        .unwrap(),
                    n as u32,
                )
                .map_err(Error::Io)?;
            }

            if (r != last_r)
                && (r != ((last_r + 1) & 0x0F))
                && (r != ((last_r + 15) & 0x0F))
            {
                if gps_time_change {
                    if return_number_models[current_channel][last_r].is_none() {
                        return_number_models[current_channel][last_r] = Some(ArithmeticSymbolModel::new(16));
                    }
                    enc.encode_symbol(
                        return_number_models[current_channel][last_r]
                            .as_mut()
                            .unwrap(),
                        r as u32,
                    )
                    .map_err(Error::Io)?;
                } else {
                    let diff = (r + 16 - last_r) & 0x0F;
                    let symbol = (diff - 2) as u32;
                    enc.encode_symbol(&mut return_number_gps_same_models[current_channel], symbol)
                        .map_err(Error::Io)?;
                }
            }

            let dst_ctx = contexts[current_channel].as_mut().ok_or_else(|| Error::InvalidValue {
                field: "laz.standard_point14_writer.scanner_channel",
                detail: "missing destination scanner channel context".to_string(),
            })?;
            let m = NUMBER_RETURN_MAP_6CTX[n][r] as usize;
            let l = NUMBER_RETURN_LEVEL_8CTX[n][r] as usize;
            let cpr = (if r == 1 { 2 } else { 0 }) + if r >= n { 1 } else { 0 };
                let idx = (m << 1) | if gps_time_change { 1 } else { 0 };

            let diff_x = raw.xi.wrapping_sub(dst_ctx.x);
            let median_x = dst_ctx.last_x_diff_median[idx].get();
            dst_ctx
                .ic_dx
                .compress(&mut enc, median_x, diff_x, (n == 1) as u32)
                .map_err(Error::Io)?;
            dst_ctx.last_x_diff_median[idx].add(diff_x);
            dst_ctx.x = raw.xi;

            let k_bits = dst_ctx.ic_dx.k();
            let context_y = (n == 1) as u32 + if k_bits < 20 { u32_zero_bit(k_bits) } else { 20 };
            let diff_y = raw.yi.wrapping_sub(dst_ctx.y);
            let median_y = dst_ctx.last_y_diff_median[idx].get();
            dst_ctx
                .ic_dy
                .compress(&mut enc, median_y, diff_y, context_y)
                .map_err(Error::Io)?;
            dst_ctx.last_y_diff_median[idx].add(diff_y);
            dst_ctx.y = raw.yi;

            k_bits_per_point.push((dst_ctx.ic_dx.k() + dst_ctx.ic_dy.k()) / 2);
            l_per_point.push(l);
            cpr_per_point.push(cpr);
            n_per_point.push(n);
            point_channel_per_point.push(current_channel);
            channel_last_intensity[current_channel] = raw.intensity;
            channel_last_n[current_channel] = n;
            channel_last_r[current_channel] = r;
            channel_last_classification[current_channel] = raw.classification;
            channel_last_flags_symbol[current_channel] = point14_flags_layer_symbol(raw.flags_byte);
            channel_last_user_data[current_channel] = raw.user_data;
            channel_last_scan_angle[current_channel] = raw.scan_angle;
            channel_last_point_source[current_channel] = raw.point_source_id;
            let gps_bits = raw.gps_time.to_bits() as i64;
            let gps_time_change = gps_bits != channel_last_gps_bits[current_channel];
            channel_last_gps_bits[current_channel] = gps_bits;
            channel_last_gps_time_change[current_channel] = gps_time_change;
            channel_last_rgb[current_channel] = raw.rgb.unwrap_or(Rgb16 { red: 0, green: 0, blue: 0 });
            channel_last_nir[current_channel] = raw.nir.unwrap_or(0);
            gps_time_change_per_point.push(gps_time_change);
        }
        let _ = enc.done().map_err(Error::Io)?;
    }
    let xy_bytes = xy_writer.into_inner();

    let mut z_bytes = Vec::<u8>::new();
    if raws.iter().skip(1).any(|raw| raw.zi != seed.zi) {
        let mut z_writer = std::io::Cursor::new(Vec::<u8>::new());
        {
            let mut enc = ArithmeticEncoder::new(&mut z_writer);
            for (i, raw) in raws.iter().skip(1).enumerate() {
                let channel = point_channel_per_point[i];
                let ctx = contexts[channel].as_mut().ok_or_else(|| Error::InvalidValue {
                    field: "laz.standard_point14_writer.scanner_channel",
                    detail: "missing z scanner channel context".to_string(),
                })?;
                let l = l_per_point[i];
                let k_bits = k_bits_per_point[i];
                let n = n_per_point[i];
                let context_z = (n == 1) as u32 + if k_bits < 18 { u32_zero_bit(k_bits) } else { 18 };
                ctx.ic_z
                    .compress(&mut enc, ctx.last_z[l], raw.zi, context_z)
                    .map_err(Error::Io)?;
                ctx.last_z[l] = raw.zi;
            }
            let _ = enc.done().map_err(Error::Io)?;
        }
        z_bytes = z_writer.into_inner();
    }

    let mut classification_bytes = Vec::<u8>::new();
    if raws
        .iter()
        .skip(1)
        .any(|raw| raw.classification != seed.classification)
    {
        #[derive(Clone)]
        struct ScannerClassificationState {
            last_classification: u8,
            models: [Option<ArithmeticSymbolModel>; 64],
        }

        let mut classification_state: [Option<ScannerClassificationState>; 4] =
            std::array::from_fn(|_| None);
        classification_state[seed_channel] = Some(ScannerClassificationState {
            last_classification: seed.classification,
            models: std::array::from_fn(|_| None),
        });
        let mut active_channel = seed_channel;

        let mut classification_writer = std::io::Cursor::new(Vec::<u8>::new());
        {
            let mut enc = ArithmeticEncoder::new(&mut classification_writer);
            for (i, raw) in raws.iter().skip(1).enumerate() {
                let channel = point_channel_per_point[i];
                if classification_state[channel].is_none() {
                    let seed_classification = classification_state[active_channel]
                        .as_ref()
                        .ok_or_else(|| Error::InvalidValue {
                            field: "laz.standard_point14_writer.scanner_channel",
                            detail: "missing source classification scanner channel context"
                                .to_string(),
                        })?
                        .last_classification;
                    classification_state[channel] = Some(ScannerClassificationState {
                        last_classification: seed_classification,
                        models: std::array::from_fn(|_| None),
                    });
                }

                active_channel = channel;
                let cls = classification_state[channel].as_mut().ok_or_else(|| {
                    Error::InvalidValue {
                        field: "laz.standard_point14_writer.scanner_channel",
                        detail: "missing destination classification scanner channel context"
                            .to_string(),
                    }
                })?;

                let cpr = cpr_per_point[i];
                let ccc =
                    (((cls.last_classification & 0x1F) << 1) + if cpr == 3 { 1 } else { 0 })
                        as usize;
                if cls.models[ccc].is_none() {
                    cls.models[ccc] = Some(ArithmeticSymbolModel::new(256));
                }
                enc.encode_symbol(cls.models[ccc].as_mut().unwrap(), raw.classification as u32)
                    .map_err(Error::Io)?;
                cls.last_classification = raw.classification;
            }
            let _ = enc.done().map_err(Error::Io)?;
        }
        classification_bytes = classification_writer.into_inner();
    }

    let mut flags_bytes = Vec::<u8>::new();
    let seed_flags_symbol = point14_flags_layer_symbol(seed.flags_byte);
    if raws
        .iter()
        .skip(1)
        .any(|raw| point14_flags_layer_symbol(raw.flags_byte) != seed_flags_symbol)
    {
        #[derive(Clone)]
        struct ScannerFlagsState {
            last_flags_symbol: u8,
            models: [Option<ArithmeticSymbolModel>; 64],
        }

        let mut flags_state: [Option<ScannerFlagsState>; 4] = std::array::from_fn(|_| None);
        flags_state[seed_channel] = Some(ScannerFlagsState {
            last_flags_symbol: seed_flags_symbol,
            models: std::array::from_fn(|_| None),
        });
        let mut active_channel = seed_channel;

        let mut flags_writer = std::io::Cursor::new(Vec::<u8>::new());
        {
            let mut enc = ArithmeticEncoder::new(&mut flags_writer);
            for (i, raw) in raws.iter().skip(1).enumerate() {
                let channel = point_channel_per_point[i];
                if flags_state[channel].is_none() {
                    let seed_flags = flags_state[active_channel]
                        .as_ref()
                        .ok_or_else(|| Error::InvalidValue {
                            field: "laz.standard_point14_writer.scanner_channel",
                            detail: "missing source flags scanner channel context".to_string(),
                        })?
                        .last_flags_symbol;
                    flags_state[channel] = Some(ScannerFlagsState {
                        last_flags_symbol: seed_flags,
                        models: std::array::from_fn(|_| None),
                    });
                }

                active_channel = channel;
                let fs = flags_state[channel].as_mut().ok_or_else(|| Error::InvalidValue {
                    field: "laz.standard_point14_writer.scanner_channel",
                    detail: "missing destination flags scanner channel context".to_string(),
                })?;

                let idx = fs.last_flags_symbol as usize;
                if fs.models[idx].is_none() {
                    fs.models[idx] = Some(ArithmeticSymbolModel::new(64));
                }
                let flags_symbol = point14_flags_layer_symbol(raw.flags_byte);
                enc.encode_symbol(fs.models[idx].as_mut().unwrap(), flags_symbol as u32)
                    .map_err(Error::Io)?;
                fs.last_flags_symbol = flags_symbol;
            }
            let _ = enc.done().map_err(Error::Io)?;
        }
        flags_bytes = flags_writer.into_inner();
    }

    let mut intensity_bytes = Vec::<u8>::new();
    if raws.iter().skip(1).any(|raw| raw.intensity != seed.intensity) {
        let mut intensity_writer = std::io::Cursor::new(Vec::<u8>::new());
        {
            let mut enc = ArithmeticEncoder::new(&mut intensity_writer);
            for (i, raw) in raws.iter().skip(1).enumerate() {
                let channel = point_channel_per_point[i];
                let ctx = contexts[channel].as_mut().ok_or_else(|| Error::InvalidValue {
                    field: "laz.standard_point14_writer.scanner_channel",
                    detail: "missing intensity scanner channel context".to_string(),
                })?;
                let cpr = cpr_per_point[i];
                let intensity_idx = (cpr << 1) | if gps_time_change_per_point[i] { 1 } else { 0 };
                ctx.ic_intensity
                    .compress(
                        &mut enc,
                        ctx.last_intensity[intensity_idx] as i32,
                        raw.intensity as i32,
                        cpr as u32,
                    )
                    .map_err(Error::Io)?;
                ctx.last_intensity[intensity_idx] = raw.intensity;
            }
            let _ = enc.done().map_err(Error::Io)?;
        }
        intensity_bytes = intensity_writer.into_inner();
    }

    let mut user_data_bytes = Vec::<u8>::new();
    if raws.iter().skip(1).any(|raw| raw.user_data != seed.user_data) {
        #[derive(Clone)]
        struct ScannerUserDataState {
            last_user_data: u8,
            models: [Option<ArithmeticSymbolModel>; 64],
        }

        let mut user_data_state: [Option<ScannerUserDataState>; 4] =
            std::array::from_fn(|_| None);
        user_data_state[seed_channel] = Some(ScannerUserDataState {
            last_user_data: seed.user_data,
            models: std::array::from_fn(|_| None),
        });
        let mut active_channel = seed_channel;

        let mut user_data_writer = std::io::Cursor::new(Vec::<u8>::new());
        {
            let mut enc = ArithmeticEncoder::new(&mut user_data_writer);
            for (i, raw) in raws.iter().skip(1).enumerate() {
                let channel = point_channel_per_point[i];
                if user_data_state[channel].is_none() {
                    let seed_user_data = user_data_state[active_channel]
                        .as_ref()
                        .ok_or_else(|| Error::InvalidValue {
                            field: "laz.standard_point14_writer.scanner_channel",
                            detail: "missing source user-data scanner channel context"
                                .to_string(),
                        })?
                        .last_user_data;
                    user_data_state[channel] = Some(ScannerUserDataState {
                        last_user_data: seed_user_data,
                        models: std::array::from_fn(|_| None),
                    });
                }

                active_channel = channel;
                let ud = user_data_state[channel].as_mut().ok_or_else(|| {
                    Error::InvalidValue {
                        field: "laz.standard_point14_writer.scanner_channel",
                        detail: "missing destination user-data scanner channel context"
                            .to_string(),
                    }
                })?;

                let idx = (ud.last_user_data / 4) as usize;
                if ud.models[idx].is_none() {
                    ud.models[idx] = Some(ArithmeticSymbolModel::new(256));
                }
                enc.encode_symbol(ud.models[idx].as_mut().unwrap(), raw.user_data as u32)
                    .map_err(Error::Io)?;
                ud.last_user_data = raw.user_data;
            }
            let _ = enc.done().map_err(Error::Io)?;
        }
        user_data_bytes = user_data_writer.into_inner();
    }

    let mut scan_angle_bytes = Vec::<u8>::new();
    if raws.iter().skip(1).any(|raw| raw.scan_angle != seed.scan_angle) {
        struct ScannerScanAngleState {
            last_scan_angle: i16,
            ic_scan_angle: IntegerCompressor,
        }

        let mut scan_angle_state: [Option<ScannerScanAngleState>; 4] =
            std::array::from_fn(|_| None);
        scan_angle_state[seed_channel] = Some(ScannerScanAngleState {
            last_scan_angle: seed.scan_angle,
            ic_scan_angle: IntegerCompressor::new(16, 2, 8, 0),
        });
        let mut active_channel = seed_channel;

        let mut scan_angle_writer = std::io::Cursor::new(Vec::<u8>::new());
        {
            let mut enc = ArithmeticEncoder::new(&mut scan_angle_writer);
            for (i, raw) in raws.iter().skip(1).enumerate() {
                let channel = point_channel_per_point[i];
                if scan_angle_state[channel].is_none() {
                    let seed_scan_angle = scan_angle_state[active_channel]
                        .as_ref()
                        .ok_or_else(|| Error::InvalidValue {
                            field: "laz.standard_point14_writer.scanner_channel",
                            detail: "missing source scan-angle scanner channel context"
                                .to_string(),
                        })?
                        .last_scan_angle;
                    scan_angle_state[channel] = Some(ScannerScanAngleState {
                        last_scan_angle: seed_scan_angle,
                        ic_scan_angle: IntegerCompressor::new(16, 2, 8, 0),
                    });
                }

                active_channel = channel;
                let sa = scan_angle_state[channel].as_mut().ok_or_else(|| {
                    Error::InvalidValue {
                        field: "laz.standard_point14_writer.scanner_channel",
                        detail: "missing destination scan-angle scanner channel context"
                            .to_string(),
                    }
                })?;

                sa.ic_scan_angle
                    .compress(
                        &mut enc,
                        sa.last_scan_angle as i32,
                        raw.scan_angle as i32,
                        if gps_time_change_per_point[i] { 1 } else { 0 },
                    )
                    .map_err(Error::Io)?;
                sa.last_scan_angle = raw.scan_angle;
            }
            let _ = enc.done().map_err(Error::Io)?;
        }
        scan_angle_bytes = scan_angle_writer.into_inner();
    }

    let mut point_source_bytes = Vec::<u8>::new();
    if raws.iter().skip(1).any(|raw| raw.point_source_id != seed.point_source_id) {
        struct ScannerPointSourceState {
            last_point_source: u16,
            ic_point_source: IntegerCompressor,
        }

        let mut point_source_state: [Option<ScannerPointSourceState>; 4] =
            std::array::from_fn(|_| None);
        point_source_state[seed_channel] = Some(ScannerPointSourceState {
            last_point_source: seed.point_source_id,
            ic_point_source: IntegerCompressor::new(16, 1, 8, 0),
        });
        let mut active_channel = seed_channel;

        let mut point_source_writer = std::io::Cursor::new(Vec::<u8>::new());
        {
            let mut enc = ArithmeticEncoder::new(&mut point_source_writer);
            for (i, raw) in raws.iter().skip(1).enumerate() {
                let channel = point_channel_per_point[i];
                if point_source_state[channel].is_none() {
                    let seed_point_source = point_source_state[active_channel]
                        .as_ref()
                        .ok_or_else(|| Error::InvalidValue {
                            field: "laz.standard_point14_writer.scanner_channel",
                            detail: "missing source point-source scanner channel context"
                                .to_string(),
                        })?
                        .last_point_source;
                    point_source_state[channel] = Some(ScannerPointSourceState {
                        last_point_source: seed_point_source,
                        ic_point_source: IntegerCompressor::new(16, 1, 8, 0),
                    });
                }

                active_channel = channel;
                let ps = point_source_state[channel].as_mut().ok_or_else(|| {
                    Error::InvalidValue {
                        field: "laz.standard_point14_writer.scanner_channel",
                        detail: "missing destination point-source scanner channel context"
                            .to_string(),
                    }
                })?;

                if raw.point_source_id == ps.last_point_source {
                    continue;
                }

                ps.ic_point_source
                    .compress(
                        &mut enc,
                        ps.last_point_source as i32,
                        raw.point_source_id as i32,
                        0,
                    )
                    .map_err(Error::Io)?;
                ps.last_point_source = raw.point_source_id;
            }
            let _ = enc.done().map_err(Error::Io)?;
        }
        point_source_bytes = point_source_writer.into_inner();
    }

    let mut gps_time_bytes = Vec::<u8>::new();
    if gps_time_change_per_point.iter().any(|&changed| changed) {
        // Per-channel GPS state mirroring decoder's Point14ContinuationContext GPS fields.
        // Each channel independently uses the full LASzip multi-book sequential GPS time codec.
        struct ChannelGpsState {
            last_gps: [i64; 4],
            last_gps_diff: [i32; 4],
            multi_extreme_counter: [i32; 4],
            gps_last: usize,
            gps_next: usize,
            m_gpstime_multi: ArithmeticSymbolModel,
            m_gpstime_0diff: ArithmeticSymbolModel,
            ic_gpstime: IntegerCompressor,
        }

        let seed_gps = seed.gps_time.to_bits() as i64;
        let mut channel_gps_state: [Option<ChannelGpsState>; 4] = std::array::from_fn(|_| None);
        channel_gps_state[seed_channel] = Some(ChannelGpsState {
            last_gps: [seed_gps, 0, 0, 0],
            last_gps_diff: [0, 0, 0, 0],
            multi_extreme_counter: [0, 0, 0, 0],
            gps_last: 0,
            gps_next: 0,
            m_gpstime_multi: ArithmeticSymbolModel::new(LASZIP_GPS_TIME_MULTI_TOTAL as u32),
            m_gpstime_0diff: ArithmeticSymbolModel::new(5),
            ic_gpstime: IntegerCompressor::new(32, 9, 8, 0),
        });
        let mut active_channel = seed_channel;

        let mut gps_time_writer = std::io::Cursor::new(Vec::<u8>::new());
        {
            let mut enc = ArithmeticEncoder::new(&mut gps_time_writer);
            for (i, raw) in raws.iter().skip(1).enumerate() {
                let channel = point_channel_per_point[i];
                if channel_gps_state[channel].is_none() {
                    // Seed new channel GPS state from active channel's last GPS time,
                    // matching decoder Point14ContinuationContext::new(seed_state) which
                    // uses current channel state.gps_bits as the initial last_gps[0].
                    let seed_gps_for_new = channel_gps_state[active_channel]
                        .as_ref()
                        .map(|s| s.last_gps[s.gps_last])
                        .unwrap_or(seed_gps);
                    channel_gps_state[channel] = Some(ChannelGpsState {
                        last_gps: [seed_gps_for_new, 0, 0, 0],
                        last_gps_diff: [0, 0, 0, 0],
                        multi_extreme_counter: [0, 0, 0, 0],
                        gps_last: 0,
                        gps_next: 0,
                        m_gpstime_multi: ArithmeticSymbolModel::new(LASZIP_GPS_TIME_MULTI_TOTAL as u32),
                        m_gpstime_0diff: ArithmeticSymbolModel::new(5),
                        ic_gpstime: IntegerCompressor::new(32, 9, 8, 0),
                    });
                }
                active_channel = channel;
                if !gps_time_change_per_point[i] {
                    continue;
                }
                let gs = channel_gps_state[channel].as_mut().unwrap();
                write_gps_time_value(
                    &mut enc,
                    &mut gs.m_gpstime_multi,
                    &mut gs.m_gpstime_0diff,
                    &mut gs.ic_gpstime,
                    &mut gs.last_gps,
                    &mut gs.last_gps_diff,
                    &mut gs.multi_extreme_counter,
                    &mut gs.gps_last,
                    &mut gs.gps_next,
                    raw.gps_time.to_bits() as i64,
                )?;
            }
            let _ = enc.done().map_err(Error::Io)?;
        }
        gps_time_bytes = gps_time_writer.into_inner();
    }

    let has_rgb = matches!(point_data_format, PointDataFormat::Pdrf7 | PointDataFormat::Pdrf8 | PointDataFormat::Pdrf12 | PointDataFormat::Pdrf13 | PointDataFormat::Pdrf14 | PointDataFormat::Pdrf15);
    let has_nir = matches!(point_data_format, PointDataFormat::Pdrf8 | PointDataFormat::Pdrf13 | PointDataFormat::Pdrf15);

    let mut nir_bytes = Vec::<u8>::new();
    if has_nir && raws.iter().skip(1).any(|raw| raw.nir != seed.nir) {
        struct ScannerNirState {
            last_nir: u16,
            m_nir_byte_used: ArithmeticSymbolModel,
            m_nir_diff: [ArithmeticSymbolModel; 2],
        }

        let mut nir_state: [Option<ScannerNirState>; 4] = std::array::from_fn(|_| None);
        nir_state[seed_channel] = Some(ScannerNirState {
            last_nir: seed.nir.unwrap_or(0),
            m_nir_byte_used: ArithmeticSymbolModel::new(4),
            m_nir_diff: std::array::from_fn(|_| ArithmeticSymbolModel::new(256)),
        });
        let mut active_channel = seed_channel;

        let mut nir_writer = std::io::Cursor::new(Vec::<u8>::new());
        {
            let mut enc = ArithmeticEncoder::new(&mut nir_writer);
            for (i, raw) in raws.iter().skip(1).enumerate() {
                let channel = point_channel_per_point[i];
                if nir_state[channel].is_none() {
                    let seed_nir = nir_state[active_channel]
                        .as_ref()
                        .ok_or_else(|| Error::InvalidValue {
                            field: "laz.standard_point14_writer.scanner_channel",
                            detail: "missing source nir scanner channel context".to_string(),
                        })?
                        .last_nir;
                    nir_state[channel] = Some(ScannerNirState {
                        last_nir: seed_nir,
                        m_nir_byte_used: ArithmeticSymbolModel::new(4),
                        m_nir_diff: std::array::from_fn(|_| ArithmeticSymbolModel::new(256)),
                    });
                }

                active_channel = channel;
                let ns = nir_state[channel].as_mut().ok_or_else(|| Error::InvalidValue {
                    field: "laz.standard_point14_writer.scanner_channel",
                    detail: "missing destination nir scanner channel context".to_string(),
                })?;

                write_nir(
                    ns.last_nir,
                    raw.nir.unwrap_or(0),
                    &mut ns.m_nir_byte_used,
                    &mut ns.m_nir_diff,
                    &mut enc,
                )?;
                ns.last_nir = raw.nir.unwrap_or(0);
            }
            let _ = enc.done().map_err(Error::Io)?;
        }
        nir_bytes = nir_writer.into_inner();
    }

    let mut rgb_bytes = Vec::<u8>::new();
    if has_rgb && raws.iter().skip(1).any(|raw| raw.rgb != seed.rgb) {
        struct ScannerRgbState {
            last_rgb: [u16; 3],
            m_rgb_byte_used: ArithmeticSymbolModel,
            m_rgb_byte_used_rgbnir: ArithmeticSymbolModel,
            m_rgb_diff: [ArithmeticSymbolModel; 6],
        }

        let seed_rgb = seed.rgb.unwrap_or(Rgb16 {
            red: 0,
            green: 0,
            blue: 0,
        });
        let mut rgb_state: [Option<ScannerRgbState>; 4] = std::array::from_fn(|_| None);
        rgb_state[seed_channel] = Some(ScannerRgbState {
            last_rgb: [seed_rgb.red, seed_rgb.green, seed_rgb.blue],
            m_rgb_byte_used: ArithmeticSymbolModel::new(128),
            m_rgb_byte_used_rgbnir: ArithmeticSymbolModel::new(128),
            m_rgb_diff: std::array::from_fn(|_| ArithmeticSymbolModel::new(256)),
        });
        let mut active_channel = seed_channel;

        let mut rgb_writer = std::io::Cursor::new(Vec::<u8>::new());
        {
            let mut enc = ArithmeticEncoder::new(&mut rgb_writer);
            for (i, raw) in raws.iter().skip(1).enumerate() {
                let channel = point_channel_per_point[i];
                if rgb_state[channel].is_none() {
                    let seed_rgb = rgb_state[active_channel]
                        .as_ref()
                        .ok_or_else(|| Error::InvalidValue {
                            field: "laz.standard_point14_writer.scanner_channel",
                            detail: "missing source rgb scanner channel context".to_string(),
                        })?
                        .last_rgb;
                    rgb_state[channel] = Some(ScannerRgbState {
                        last_rgb: seed_rgb,
                        m_rgb_byte_used: ArithmeticSymbolModel::new(128),
                        m_rgb_byte_used_rgbnir: ArithmeticSymbolModel::new(128),
                        m_rgb_diff: std::array::from_fn(|_| ArithmeticSymbolModel::new(256)),
                    });
                }

                active_channel = channel;
                let rs = rgb_state[channel].as_mut().ok_or_else(|| Error::InvalidValue {
                    field: "laz.standard_point14_writer.scanner_channel",
                    detail: "missing destination rgb scanner channel context".to_string(),
                })?;

                let rgb = raw.rgb.unwrap_or(Rgb16 {
                    red: 0,
                    green: 0,
                    blue: 0,
                });
                let target = [rgb.red, rgb.green, rgb.blue];
                write_rgb(
                    rs.last_rgb,
                    target,
                    &mut rs.m_rgb_byte_used,
                    &mut rs.m_rgb_byte_used_rgbnir,
                    &mut rs.m_rgb_diff,
                    &mut enc,
                    has_nir,
                )?;
                rs.last_rgb = target;
            }
            let _ = enc.done().map_err(Error::Io)?;
        }
        rgb_bytes = rgb_writer.into_inner();
    }

    let extra_layer_bytes = if let Some(extra_bytes_per_point) = extra_bytes_per_point {
        encode_point14_byte14_layers(extra_bytes_per_point, &point_channel_per_point, seed_channel)?
    } else {
        Vec::new()
    };

    let mut out = seed.to_bytes(point_data_format).ok_or_else(|| Error::InvalidValue {
        field: "laz.standard_point14_writer.serialize",
        detail: "failed to serialize Point14 seed point".to_string(),
    })?;
    if let Some(seed_extra) = extra_bytes_per_point.and_then(|bytes| bytes.first()) {
        out.extend_from_slice(seed_extra);
    }

    let chunk_point_count = raws.len() as u32;
    out.extend_from_slice(&chunk_point_count.to_le_bytes());
    out.extend_from_slice(&(xy_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&(z_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&(classification_bytes.len() as u32).to_le_bytes()); // classification
    out.extend_from_slice(&(flags_bytes.len() as u32).to_le_bytes()); // flags
    out.extend_from_slice(&(intensity_bytes.len() as u32).to_le_bytes()); // intensity
    out.extend_from_slice(&(scan_angle_bytes.len() as u32).to_le_bytes()); // scan_angle
    out.extend_from_slice(&(user_data_bytes.len() as u32).to_le_bytes()); // user_data
    out.extend_from_slice(&(point_source_bytes.len() as u32).to_le_bytes()); // point_source
    out.extend_from_slice(&(gps_time_bytes.len() as u32).to_le_bytes()); // gps_time
    if has_rgb {
        out.extend_from_slice(&(rgb_bytes.len() as u32).to_le_bytes());
    }
    if has_nir {
        out.extend_from_slice(&(nir_bytes.len() as u32).to_le_bytes());
    }
    for bytes in &extra_layer_bytes {
        out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    }
    out.extend_from_slice(&xy_bytes);
    out.extend_from_slice(&z_bytes);
    out.extend_from_slice(&classification_bytes);
    out.extend_from_slice(&flags_bytes);
    out.extend_from_slice(&intensity_bytes);
    out.extend_from_slice(&scan_angle_bytes);
    out.extend_from_slice(&user_data_bytes);
    out.extend_from_slice(&point_source_bytes);
    out.extend_from_slice(&gps_time_bytes);
    out.extend_from_slice(&rgb_bytes);
    out.extend_from_slice(&nir_bytes);
    for bytes in &extra_layer_bytes {
        out.extend_from_slice(bytes);
    }

    Ok(out)
}

#[derive(Clone)]
struct Point14State {
    x: i32,
    y: i32,
    z: i32,
    intensity: u16,
    return_number: u8,
    number_of_returns: u8,
    edge_of_flight_line: bool,
    scan_direction_flag: bool,
    classification_flags: u8,
    classification: u8,
    user_data: u8,
    scan_angle: i16,
    point_source_id: u16,
    gps_bits: i64,
    gps_time_change: bool,
    rgb: Option<Rgb16>,
    nir: Option<u16>,
    extra_bytes: Option<Vec<u8>>,
}

impl Point14State {
    fn from_raw(raw: RawPoint14, extra_bytes: Option<Vec<u8>>) -> Self {
        Self {
            x: raw.xi,
            y: raw.yi,
            z: raw.zi,
            intensity: raw.intensity,
            return_number: raw.return_byte & 0x0F,
            number_of_returns: (raw.return_byte >> 4) & 0x0F,
            edge_of_flight_line: (raw.flags_byte & 0x80) != 0,
            scan_direction_flag: (raw.flags_byte & 0x40) != 0,
            classification_flags: raw.flags_byte & 0x0F,
            classification: raw.classification,
            user_data: raw.user_data,
            scan_angle: raw.scan_angle,
            point_source_id: raw.point_source_id,
            gps_bits: raw.gps_time.to_bits() as i64,
            gps_time_change: false,
            rgb: raw.rgb,
            nir: raw.nir,
            extra_bytes,
        }
    }

    fn to_point_record(&self, scales: [f64; 3], offsets: [f64; 3]) -> PointRecord {
        let mut out = PointRecord {
            x: self.x as f64 * scales[0] + offsets[0],
            y: self.y as f64 * scales[1] + offsets[1],
            z: self.z as f64 * scales[2] + offsets[2],
            intensity: self.intensity,
            classification: self.classification,
            user_data: self.user_data,
            point_source_id: self.point_source_id,
            return_number: self.return_number,
            number_of_returns: self.number_of_returns,
            scan_direction_flag: self.scan_direction_flag,
            edge_of_flight_line: self.edge_of_flight_line,
            scan_angle: self.scan_angle,
            gps_time: Some(GpsTime(f64::from_bits(self.gps_bits as u64))),
            flags: self.classification_flags,
            color: self.rgb,
            nir: self.nir,
            ..PointRecord::default()
        };
        if let Some(extra) = self.extra_bytes.as_deref() {
            let copy_len = usize::min(extra.len(), 192);
            out.extra_bytes.data[..copy_len].copy_from_slice(&extra[..copy_len]);
            out.extra_bytes.len = copy_len as u8;
        }
        out
    }
}

struct Point14ContinuationContext {
    state: Point14State,
    last_intensity: [u16; 8],
    last_x_diff_median: [StreamingMedianI32; 12],
    last_y_diff_median: [StreamingMedianI32; 12],
    last_z: [i32; 8],
    m_changed_values: [ArithmeticSymbolModel; 8],
    m_scanner_channel: ArithmeticSymbolModel,
    m_number_of_returns: [Option<ArithmeticSymbolModel>; 16],
    m_return_number_gps_same: ArithmeticSymbolModel,
    m_return_number: [Option<ArithmeticSymbolModel>; 16],
    ic_dx: IntegerDecompressor,
    ic_dy: IntegerDecompressor,
    ic_z: IntegerDecompressor,
    m_classification: [Option<ArithmeticSymbolModel>; 64],
    m_flags: [Option<ArithmeticSymbolModel>; 64],
    m_user_data: [Option<ArithmeticSymbolModel>; 64],
    ic_intensity: IntegerDecompressor,
    ic_scan_angle: IntegerDecompressor,
    ic_point_source_id: IntegerDecompressor,
    last_gps: [i64; 4],
    last_gps_diff: [i32; 4],
    multi_extreme_counter: [i32; 4],
    gps_last: usize,
    gps_next: usize,
    m_gpstime_multi: ArithmeticSymbolModel,
    m_gpstime_0diff: ArithmeticSymbolModel,
    ic_gpstime: IntegerDecompressor,
    // RGB14 per-context state (only used when has_rgb is true)
    last_rgb: [u16; 3],  // [R, G, B] as u16 (only low byte significant per LASzip)
    m_rgb_byte_used: ArithmeticSymbolModel,   // 128-symbol model for RGB14
    m_rgb_byte_used_rgbnir: ArithmeticSymbolModel, // 128-symbol model for RGBNIR14
    m_rgb_diff: [ArithmeticSymbolModel; 6],   // 256-symbol models, diff_0..5
    // NIR14 per-context state (only used when has_nir is true)
    last_nir: u16,
    m_nir_byte_used: ArithmeticSymbolModel,
    m_nir_diff: [ArithmeticSymbolModel; 2],
    m_extra_bytes: Vec<ArithmeticSymbolModel>,
}

impl Point14ContinuationContext {
    fn new(seed: Point14State) -> Self {
        Self {
            state: seed.clone(),
            last_intensity: [seed.intensity; 8],
            last_x_diff_median: [StreamingMedianI32::new(); 12],
            last_y_diff_median: [StreamingMedianI32::new(); 12],
            last_z: [seed.z; 8],
            m_changed_values: std::array::from_fn(|_| ArithmeticSymbolModel::new(128)),
            m_scanner_channel: ArithmeticSymbolModel::new(3),
            m_number_of_returns: std::array::from_fn(|_| None),
            m_return_number_gps_same: ArithmeticSymbolModel::new(13),
            m_return_number: std::array::from_fn(|_| None),
            ic_dx: IntegerDecompressor::new(32, 2, 8, 0),
            ic_dy: IntegerDecompressor::new(32, 22, 8, 0),
            ic_z: IntegerDecompressor::new(32, 20, 8, 0),
            m_classification: std::array::from_fn(|_| None),
            m_flags: std::array::from_fn(|_| None),
            m_user_data: std::array::from_fn(|_| None),
            ic_intensity: IntegerDecompressor::new(16, 4, 8, 0),
            ic_scan_angle: IntegerDecompressor::new(16, 2, 8, 0),
            ic_point_source_id: IntegerDecompressor::new(16, 1, 8, 0),
            last_gps: [seed.gps_bits, 0, 0, 0],
            last_gps_diff: [0, 0, 0, 0],
            multi_extreme_counter: [0, 0, 0, 0],
            gps_last: 0,
            gps_next: 0,
            m_gpstime_multi: ArithmeticSymbolModel::new(LASZIP_GPS_TIME_MULTI_TOTAL as u32),
            m_gpstime_0diff: ArithmeticSymbolModel::new(5),
            ic_gpstime: IntegerDecompressor::new(32, 9, 8, 0),
            last_rgb: {
                let rgb = seed.rgb.unwrap_or(Rgb16 { red: 0, green: 0, blue: 0 });
                [rgb.red, rgb.green, rgb.blue]
            },
            m_rgb_byte_used: ArithmeticSymbolModel::new(128),
            m_rgb_byte_used_rgbnir: ArithmeticSymbolModel::new(128),
            m_rgb_diff: std::array::from_fn(|_| ArithmeticSymbolModel::new(256)),
            last_nir: seed.nir.unwrap_or(0),
            m_nir_byte_used: ArithmeticSymbolModel::new(4),
            m_nir_diff: std::array::from_fn(|_| ArithmeticSymbolModel::new(256)),
            m_extra_bytes: seed
                .extra_bytes
                .as_ref()
                .map(|bytes| {
                    (0..bytes.len())
                        .map(|_| ArithmeticSymbolModel::new(256))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

/// Decode one standard LASzip layered chunk for Point14-family item lists.
///
/// This implementation currently decodes the uncompressed seed point and
/// validates Point14 item layouts. Arithmetic continuation for remaining points
/// in the chunk is a follow-up step.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Point14LayeredDecodeStatus {
    /// Number of points requested for the layered continuation decode.
    pub expected_points: usize,
    /// Number of points actually decoded and returned.
    pub decoded_points: usize,
    /// True when `decoded_points < expected_points` due to tolerant recovery.
    pub partial: bool,
}

/// Decode one standard LASzip layered Point14 v3 chunk.
///
/// This convenience wrapper discards decode-status metadata and returns only
/// decoded points.
pub fn decode_standard_layered_chunk_point14_v3(
    chunk_bytes: &[u8],
    point_count: usize,
    item_specs: &[LaszipItemSpec],
    point_data_format: PointDataFormat,
    scales: [f64; 3],
    offsets: [f64; 3],
) -> Result<Vec<PointRecord>> {
    decode_standard_layered_chunk_point14_v3_with_status(
        chunk_bytes,
        point_count,
        item_specs,
        point_data_format,
        scales,
        offsets,
    )
    .map(|(points, _status)| points)
}

/// Encode a singleton standard LASzip layered Point14 v3 chunk.
///
/// This writes only the uncompressed seed point item set and is valid for
/// chunks containing exactly one point.
pub fn encode_standard_layered_chunk_point14_v3_singleton(
    points: &[PointRecord],
    point_data_format: PointDataFormat,
    scales: [f64; 3],
    offsets: [f64; 3],
) -> Result<Vec<u8>> {
    if points.len() != 1 {
        return Err(Error::Unimplemented(
            "standard Point14 v3 writer currently supports singleton chunks only",
        ));
    }

    let base_format = normalize_point14_base_format(point_data_format)?;

    let raw = RawPoint14::from_point_record(points[0], base_format, scales, offsets)
        .ok_or_else(|| Error::InvalidValue {
            field: "laz.standard_point14_writer.point",
            detail: "point cannot be represented in requested Point14 format".to_string(),
        })?;

    let payload_bytes = collect_point14_payload_bytes(points, point_data_format)?;
    let seed_extra = payload_bytes
        .as_ref()
        .and_then(|bytes| bytes.first().map(Vec::as_slice));
    let mut out = serialize_point14_seed_item_set(raw, base_format, seed_extra)?;

    // External LASzip encoders still emit a layered tail for singleton Point14
    // chunks. In the observed reference output this tail contains count=1,
    // empty arithmetic streams for XY and Z (4 bytes each), and zero-length
    // optional attribute layers.
    let empty_xy = empty_arithmetic_stream()?;
    let empty_z = empty_arithmetic_stream()?;
    let has_rgb = matches!(base_format, PointDataFormat::Pdrf7 | PointDataFormat::Pdrf8 | PointDataFormat::Pdrf12 | PointDataFormat::Pdrf13 | PointDataFormat::Pdrf14 | PointDataFormat::Pdrf15);
    let has_nir = matches!(base_format, PointDataFormat::Pdrf8 | PointDataFormat::Pdrf13 | PointDataFormat::Pdrf15);
    let extra_len = seed_extra.map_or(0, |b| b.len());

    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&(empty_xy.len() as u32).to_le_bytes());
    out.extend_from_slice(&(empty_z.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    if has_rgb {
        out.extend_from_slice(&0u32.to_le_bytes());
    }
    if has_nir {
        out.extend_from_slice(&0u32.to_le_bytes());
    }
    for _ in 0..extra_len {
        out.extend_from_slice(&0u32.to_le_bytes());
    }
    out.extend_from_slice(&empty_xy);
    out.extend_from_slice(&empty_z);

    Ok(out)
}

fn empty_arithmetic_stream() -> Result<Vec<u8>> {
    let mut out = std::io::Cursor::new(Vec::<u8>::new());
    let enc = ArithmeticEncoder::new(&mut out);
    let _ = enc.done().map_err(Error::Io)?;
    Ok(out.into_inner())
}

/// Encode a standard LASzip layered Point14 v3 chunk for points that share all
/// non-XYZ attributes with the seed point.
///
/// Supported subset:
/// - PDRF6/7/8
/// - per-point scanner-channel/
///   rgb/nir unchanged from seed
/// - per-point return fields may vary
/// - per-point intensity may vary
/// - per-point classification may vary
/// - per-point classification/scan-direction/edge flags may vary
/// - per-point user_data may vary
/// - per-point point_source_id may vary
/// - per-point scan_angle may vary
/// - per-point gps_time may vary
/// - XY and optional Z can vary across tail points
pub fn encode_standard_layered_chunk_point14_v3_constant_attributes(
    points: &[PointRecord],
    point_data_format: PointDataFormat,
    scales: [f64; 3],
    offsets: [f64; 3],
) -> Result<Vec<u8>> {
    if points.is_empty() {
        return Ok(Vec::new());
    }
    if points.len() == 1 {
        return encode_standard_layered_chunk_point14_v3_singleton(
            points,
            point_data_format,
            scales,
            offsets,
        );
    }

    let base_format = normalize_point14_base_format(point_data_format)?;

    let mut raws = Vec::with_capacity(points.len());
    for &p in points {
        let raw = RawPoint14::from_point_record(p, base_format, scales, offsets)
            .ok_or_else(|| Error::InvalidValue {
                field: "laz.standard_point14_writer.point",
                detail: "point cannot be represented in requested Point14 format".to_string(),
            })?;
        raws.push(raw);
    }

    let extra_bytes_per_point = collect_point14_payload_bytes(points, point_data_format)?;
    let seed_extra_bytes = extra_bytes_per_point
        .as_ref()
        .and_then(|bytes| bytes.first())
        .cloned();

    let seed = raws[0];
    let has_rgb = matches!(base_format, PointDataFormat::Pdrf7 | PointDataFormat::Pdrf8 | PointDataFormat::Pdrf12 | PointDataFormat::Pdrf13 | PointDataFormat::Pdrf14 | PointDataFormat::Pdrf15);
    let has_nir = matches!(base_format, PointDataFormat::Pdrf8 | PointDataFormat::Pdrf13 | PointDataFormat::Pdrf15);
    let seed_scanner_channel = point14_scanner_channel_bits(seed.flags_byte);
    let scanner_channel_varies = raws
        .iter()
        .skip(1)
        .any(|raw| point14_scanner_channel_bits(raw.flags_byte) != seed_scanner_channel);

    if scanner_channel_varies {
        for raw in raws.iter().skip(1) {
            if raw.nir != seed.nir && !has_nir {
                return Err(Error::Unimplemented(
                    "standard Point14 multi-point scanner-channel subset currently supports XYZ, return fields, intensity, flags, classification, user_data, scan_angle, point_source, gps_time, and rgb variation; nir variation requires PDRF8",
                ));
            }
        }

        return encode_standard_layered_chunk_point14_v3_scanner_channel_subset(
            &raws,
            extra_bytes_per_point.as_deref(),
            base_format,
        );
    }

    for raw in raws.iter().skip(1) {
        if (!has_rgb && raw.rgb != seed.rgb) || (!has_nir && raw.nir != seed.nir) {
            return Err(Error::Unimplemented(
                "standard Point14 multi-point writer subset supports RGB variation for PDRF7/PDRF8 and NIR variation for PDRF8; attributes must remain representable in the target point format",
            ));
        }
    }

    let mut last_n = ((seed.return_byte >> 4) & 0x0F) as usize;
    let mut last_r = (seed.return_byte & 0x0F) as usize;
    let mut last_point_source_id = seed.point_source_id;
    let mut last_scan_angle = seed.scan_angle;
    let mut last_gps_time_change = false;

    let mut last_x = seed.xi;
    let mut last_y = seed.yi;
    let mut last_x_diff_median = [StreamingMedianI32::new(); 12];
    let mut last_y_diff_median = [StreamingMedianI32::new(); 12];

    let mut xy_writer = std::io::Cursor::new(Vec::<u8>::new());
    let mut k_bits_per_point = Vec::<u32>::with_capacity(points.len() - 1);
    let mut l_per_point = Vec::<usize>::with_capacity(points.len() - 1);
    let mut cpr_per_point = Vec::<usize>::with_capacity(points.len() - 1);
    let mut point_source_change_per_point = Vec::<bool>::with_capacity(points.len() - 1);
    let mut scan_angle_change_per_point = Vec::<bool>::with_capacity(points.len() - 1);
    let mut gps_time_change_per_point = Vec::<bool>::with_capacity(points.len() - 1);
    {
        let mut enc = ArithmeticEncoder::new(&mut xy_writer);
        let mut changed_models: [ArithmeticSymbolModel; 8] =
            std::array::from_fn(|_| ArithmeticSymbolModel::new(128));
        let mut number_of_returns_models: [Option<ArithmeticSymbolModel>; 16] =
            std::array::from_fn(|_| None);
        let mut return_number_models: [Option<ArithmeticSymbolModel>; 16] =
            std::array::from_fn(|_| None);
        let mut return_number_gps_same_model = ArithmeticSymbolModel::new(13);
        let mut ic_dx = IntegerCompressor::new(32, 2, 8, 0);
        let mut ic_dy = IntegerCompressor::new(32, 22, 8, 0);
        let mut last_gps_bits = seed.gps_time.to_bits() as i64;
        for raw in raws.iter().skip(1) {
            let n = ((raw.return_byte >> 4) & 0x0F) as usize;
            let r = (raw.return_byte & 0x0F) as usize;
            let point_source_change = raw.point_source_id != last_point_source_id;
            let scan_angle_change = raw.scan_angle != last_scan_angle;
            let gps_bits = raw.gps_time.to_bits() as i64;
            let gps_time_change = gps_bits != last_gps_bits;

            let mut lpr = if last_r == 1 { 1 } else { 0 };
            if last_r >= last_n {
                lpr += 2;
            }
            if last_gps_time_change {
                lpr += 4;
            }

            let mut changed_values = 0u8;
            if n != last_n {
                changed_values |= 1 << 2;
            }
            if point_source_change {
                changed_values |= 1 << 5;
            }
            if gps_time_change {
                changed_values |= 1 << 4;
            }
            if scan_angle_change {
                changed_values |= 1 << 3;
            }

            if r == last_r {
                // lower two bits remain 0
            } else if r == ((last_r + 1) & 0x0F) {
                changed_values |= 1;
            } else if r == ((last_r + 15) & 0x0F) {
                changed_values |= 2;
            } else {
                changed_values |= 3;
            }

            enc.encode_symbol(&mut changed_models[lpr], changed_values as u32)
                .map_err(Error::Io)?;

            if (changed_values & (1 << 2)) != 0 {
                if number_of_returns_models[last_n].is_none() {
                    number_of_returns_models[last_n] = Some(ArithmeticSymbolModel::new(16));
                }
                enc.encode_symbol(
                    number_of_returns_models[last_n].as_mut().unwrap(),
                    n as u32,
                )
                .map_err(Error::Io)?;
            }

            if (changed_values & 0x03) == 3 {
                if gps_time_change {
                    if return_number_models[last_r].is_none() {
                        return_number_models[last_r] = Some(ArithmeticSymbolModel::new(16));
                    }
                    enc.encode_symbol(return_number_models[last_r].as_mut().unwrap(), r as u32)
                        .map_err(Error::Io)?;
                } else {
                    let diff = (r + 16 - last_r) & 0x0F;
                    debug_assert!((2..=14).contains(&diff));
                    let symbol = (diff - 2) as u32;
                    enc.encode_symbol(&mut return_number_gps_same_model, symbol)
                        .map_err(Error::Io)?;
                }
            }

            let m = NUMBER_RETURN_MAP_6CTX[n][r] as usize;
            let l = NUMBER_RETURN_LEVEL_8CTX[n][r] as usize;
            let cpr = (if r == 1 { 2 } else { 0 }) + (if r >= n { 1 } else { 0 });

            let idx = (m << 1) | if gps_time_change { 1 } else { 0 };
            let diff_x = raw.xi.wrapping_sub(last_x);
            let median_x = last_x_diff_median[idx].get();
            ic_dx
                .compress(&mut enc, median_x, diff_x, (n == 1) as u32)
                .map_err(Error::Io)?;
            last_x_diff_median[idx].add(diff_x);
            last_x = raw.xi;

            let k_bits = ic_dx.k();
            let context_y = (n == 1) as u32 + if k_bits < 20 { u32_zero_bit(k_bits) } else { 20 };
            let diff_y = raw.yi.wrapping_sub(last_y);
            let median_y = last_y_diff_median[idx].get();
            ic_dy
                .compress(&mut enc, median_y, diff_y, context_y)
                .map_err(Error::Io)?;
            last_y_diff_median[idx].add(diff_y);
            last_y = raw.yi;
            k_bits_per_point.push((ic_dx.k() + ic_dy.k()) / 2);
            l_per_point.push(l);
            cpr_per_point.push(cpr);
            point_source_change_per_point.push(point_source_change);
            scan_angle_change_per_point.push(scan_angle_change);
            gps_time_change_per_point.push(gps_time_change);
            last_n = n;
            last_r = r;
            last_gps_time_change = gps_time_change;
            last_point_source_id = raw.point_source_id;
            last_scan_angle = raw.scan_angle;
            last_gps_bits = gps_bits;
        }
        let _ = enc.done().map_err(Error::Io)?;
    }
    let xy_bytes = xy_writer.into_inner();

    let mut z_bytes = Vec::<u8>::new();
    if raws.iter().skip(1).any(|raw| raw.zi != seed.zi) {
        let mut last_z = [seed.zi; 8];
        let mut z_writer = std::io::Cursor::new(Vec::<u8>::new());
        {
            let mut enc = ArithmeticEncoder::new(&mut z_writer);
            let mut ic_z = IntegerCompressor::new(32, 20, 8, 0);
            for (i, raw) in raws.iter().skip(1).enumerate() {
                let l = l_per_point[i];
                let k_bits = k_bits_per_point[i];
                let n = ((raw.return_byte >> 4) & 0x0F) as usize;
                let context_z = (n == 1) as u32 + if k_bits < 18 { u32_zero_bit(k_bits) } else { 18 };
                ic_z
                    .compress(&mut enc, last_z[l], raw.zi, context_z)
                    .map_err(Error::Io)?;
                last_z[l] = raw.zi;
            }
            let _ = enc.done().map_err(Error::Io)?;
        }
        z_bytes = z_writer.into_inner();
    }

    let mut intensity_bytes = Vec::<u8>::new();
    if raws.iter().skip(1).any(|raw| raw.intensity != seed.intensity) {
        let mut last_intensity = [seed.intensity; 8];
        let mut intensity_writer = std::io::Cursor::new(Vec::<u8>::new());
        {
            let mut enc = ArithmeticEncoder::new(&mut intensity_writer);
            let mut ic_intensity = IntegerCompressor::new(16, 4, 8, 0);
            for (i, raw) in raws.iter().skip(1).enumerate() {
                let cpr = cpr_per_point[i];
                let gps_time_change = gps_time_change_per_point[i];
                let intensity_idx = (cpr << 1) | if gps_time_change { 1 } else { 0 };
                ic_intensity
                    .compress(
                        &mut enc,
                        last_intensity[intensity_idx] as i32,
                        raw.intensity as i32,
                        cpr as u32,
                    )
                    .map_err(Error::Io)?;
                last_intensity[intensity_idx] = raw.intensity;
            }
            let _ = enc.done().map_err(Error::Io)?;
        }
        intensity_bytes = intensity_writer.into_inner();
    }

    let mut classification_bytes = Vec::<u8>::new();
    if raws
        .iter()
        .skip(1)
        .any(|raw| raw.classification != seed.classification)
    {
        let mut last_classification = seed.classification;
        let mut classification_writer = std::io::Cursor::new(Vec::<u8>::new());
        {
            let mut enc = ArithmeticEncoder::new(&mut classification_writer);
            let mut models: [Option<ArithmeticSymbolModel>; 64] =
                std::array::from_fn(|_| None);
            for (i, raw) in raws.iter().skip(1).enumerate() {
                let cpr = cpr_per_point[i];
                let ccc = (((last_classification & 0x1F) << 1) + if cpr == 3 { 1 } else { 0 })
                    as usize;
                if models[ccc].is_none() {
                    models[ccc] = Some(ArithmeticSymbolModel::new(256));
                }
                enc.encode_symbol(models[ccc].as_mut().unwrap(), raw.classification as u32)
                    .map_err(Error::Io)?;
                last_classification = raw.classification;
            }
            let _ = enc.done().map_err(Error::Io)?;
        }
        classification_bytes = classification_writer.into_inner();
    }

    let mut flags_bytes = Vec::<u8>::new();
    let seed_flags_symbol = point14_flags_layer_symbol(seed.flags_byte);
    if raws
        .iter()
        .skip(1)
        .any(|raw| point14_flags_layer_symbol(raw.flags_byte) != seed_flags_symbol)
    {
        let mut last_flags_symbol = seed_flags_symbol;
        let mut flags_writer = std::io::Cursor::new(Vec::<u8>::new());
        {
            let mut enc = ArithmeticEncoder::new(&mut flags_writer);
            let mut models: [Option<ArithmeticSymbolModel>; 64] = std::array::from_fn(|_| None);
            for raw in raws.iter().skip(1) {
                let idx = last_flags_symbol as usize;
                if models[idx].is_none() {
                    models[idx] = Some(ArithmeticSymbolModel::new(64));
                }
                let flags_symbol = point14_flags_layer_symbol(raw.flags_byte);
                enc.encode_symbol(models[idx].as_mut().unwrap(), flags_symbol as u32)
                    .map_err(Error::Io)?;
                last_flags_symbol = flags_symbol;
            }
            let _ = enc.done().map_err(Error::Io)?;
        }
        flags_bytes = flags_writer.into_inner();
    }

    let mut user_data_bytes = Vec::<u8>::new();
    if raws.iter().skip(1).any(|raw| raw.user_data != seed.user_data) {
        let mut last_user_data = seed.user_data;
        let mut user_data_writer = std::io::Cursor::new(Vec::<u8>::new());
        {
            let mut enc = ArithmeticEncoder::new(&mut user_data_writer);
            let mut models: [Option<ArithmeticSymbolModel>; 64] = std::array::from_fn(|_| None);
            for raw in raws.iter().skip(1) {
                let idx = (last_user_data / 4) as usize;
                if models[idx].is_none() {
                    models[idx] = Some(ArithmeticSymbolModel::new(256));
                }
                enc.encode_symbol(models[idx].as_mut().unwrap(), raw.user_data as u32)
                    .map_err(Error::Io)?;
                last_user_data = raw.user_data;
            }
            let _ = enc.done().map_err(Error::Io)?;
        }
        user_data_bytes = user_data_writer.into_inner();
    }

    let mut point_source_bytes = Vec::<u8>::new();
    if point_source_change_per_point.iter().any(|&changed| changed) {
        let mut last_point_source = seed.point_source_id;
        let mut point_source_writer = std::io::Cursor::new(Vec::<u8>::new());
        {
            let mut enc = ArithmeticEncoder::new(&mut point_source_writer);
            let mut ic_point_source = IntegerCompressor::new(16, 1, 8, 0);
            for (i, raw) in raws.iter().skip(1).enumerate() {
                if point_source_change_per_point[i] {
                    ic_point_source
                        .compress(
                            &mut enc,
                            last_point_source as i32,
                            raw.point_source_id as i32,
                            0,
                        )
                        .map_err(Error::Io)?;
                }
                last_point_source = raw.point_source_id;
            }
            let _ = enc.done().map_err(Error::Io)?;
        }
        point_source_bytes = point_source_writer.into_inner();
    }

    let mut scan_angle_bytes = Vec::<u8>::new();
    if scan_angle_change_per_point.iter().any(|&changed| changed) {
        let mut last_scan_angle = seed.scan_angle;
        let mut scan_angle_writer = std::io::Cursor::new(Vec::<u8>::new());
        {
            let mut enc = ArithmeticEncoder::new(&mut scan_angle_writer);
            let mut ic_scan_angle = IntegerCompressor::new(16, 2, 8, 0);
            for (i, raw) in raws.iter().skip(1).enumerate() {
                if scan_angle_change_per_point[i] {
                    ic_scan_angle
                        .compress(
                            &mut enc,
                            last_scan_angle as i32,
                            raw.scan_angle as i32,
                            if gps_time_change_per_point[i] { 1 } else { 0 },
                        )
                        .map_err(Error::Io)?;
                }
                last_scan_angle = raw.scan_angle;
            }
            let _ = enc.done().map_err(Error::Io)?;
        }
        scan_angle_bytes = scan_angle_writer.into_inner();
    }

    let mut gps_time_bytes = Vec::<u8>::new();
    if gps_time_change_per_point.iter().any(|&changed| changed) {
        let changed_gps_values: Vec<i64> = raws
            .iter()
            .skip(1)
            .enumerate()
            .filter_map(|(i, raw)| gps_time_change_per_point[i].then_some(raw.gps_time.to_bits() as i64))
            .collect();
        gps_time_bytes = encode_gps_time_sequence(&changed_gps_values, seed.gps_time.to_bits() as i64)?;
    }

    let mut rgb_bytes = Vec::<u8>::new();
    if has_rgb && raws.iter().skip(1).any(|raw| raw.rgb != seed.rgb) {
        let seed_rgb = seed.rgb.unwrap_or(Rgb16 {
            red: 0,
            green: 0,
            blue: 0,
        });
        let mut last_rgb = [seed_rgb.red, seed_rgb.green, seed_rgb.blue];
        let mut m_rgb_byte_used = ArithmeticSymbolModel::new(128);
        let mut m_rgb_byte_used_rgbnir = ArithmeticSymbolModel::new(128);
        let mut m_rgb_diff: [ArithmeticSymbolModel; 6] =
            std::array::from_fn(|_| ArithmeticSymbolModel::new(256));

        let mut rgb_writer = std::io::Cursor::new(Vec::<u8>::new());
        {
            let mut enc = ArithmeticEncoder::new(&mut rgb_writer);
            for raw in raws.iter().skip(1) {
                let rgb = raw.rgb.unwrap_or(Rgb16 {
                    red: 0,
                    green: 0,
                    blue: 0,
                });
                let target = [rgb.red, rgb.green, rgb.blue];
                write_rgb(
                    last_rgb,
                    target,
                    &mut m_rgb_byte_used,
                    &mut m_rgb_byte_used_rgbnir,
                    &mut m_rgb_diff,
                    &mut enc,
                    has_nir,
                )?;
                last_rgb = target;
            }
            let _ = enc.done().map_err(Error::Io)?;
        }
        rgb_bytes = rgb_writer.into_inner();
    }

    let mut nir_bytes = Vec::<u8>::new();
    if has_nir && raws.iter().skip(1).any(|raw| raw.nir != seed.nir) {
        let mut last_nir = seed.nir.unwrap_or(0);
        let mut m_nir_byte_used = ArithmeticSymbolModel::new(4);
        let mut m_nir_diff: [ArithmeticSymbolModel; 2] =
            std::array::from_fn(|_| ArithmeticSymbolModel::new(256));

        let mut nir_writer = std::io::Cursor::new(Vec::<u8>::new());
        {
            let mut enc = ArithmeticEncoder::new(&mut nir_writer);
            for raw in raws.iter().skip(1) {
                write_nir(
                    last_nir,
                    raw.nir.unwrap_or(0),
                    &mut m_nir_byte_used,
                    &mut m_nir_diff,
                    &mut enc,
                )?;
                last_nir = raw.nir.unwrap_or(0);
            }
            let _ = enc.done().map_err(Error::Io)?;
        }
        nir_bytes = nir_writer.into_inner();
    }

    let extra_layer_bytes = if let Some(extra_bytes_per_point) = extra_bytes_per_point.as_deref() {
        let point_channels = vec![0usize; points.len().saturating_sub(1)];
        encode_point14_byte14_layers(extra_bytes_per_point, &point_channels, 0)?
    } else {
        Vec::new()
    };

    let mut out = serialize_point14_seed_item_set(
        seed,
        point_data_format,
        seed_extra_bytes.as_deref(),
    )?;

    let chunk_point_count = points.len() as u32;
    out.extend_from_slice(&chunk_point_count.to_le_bytes());
    out.extend_from_slice(&(xy_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&(z_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&(classification_bytes.len() as u32).to_le_bytes()); // classification
    out.extend_from_slice(&(flags_bytes.len() as u32).to_le_bytes()); // flags
    out.extend_from_slice(&(intensity_bytes.len() as u32).to_le_bytes()); // intensity
    out.extend_from_slice(&(scan_angle_bytes.len() as u32).to_le_bytes()); // scan_angle
    out.extend_from_slice(&(user_data_bytes.len() as u32).to_le_bytes()); // user_data
    out.extend_from_slice(&(point_source_bytes.len() as u32).to_le_bytes()); // point_source
    out.extend_from_slice(&(gps_time_bytes.len() as u32).to_le_bytes()); // gps_time
    if has_rgb {
        out.extend_from_slice(&(rgb_bytes.len() as u32).to_le_bytes());
    }
    if has_nir {
        out.extend_from_slice(&(nir_bytes.len() as u32).to_le_bytes());
    }
    for bytes in &extra_layer_bytes {
        out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    }
    out.extend_from_slice(&xy_bytes);
    out.extend_from_slice(&z_bytes);
    out.extend_from_slice(&classification_bytes);
    out.extend_from_slice(&flags_bytes);
    out.extend_from_slice(&intensity_bytes);
    out.extend_from_slice(&scan_angle_bytes);
    out.extend_from_slice(&user_data_bytes);
    out.extend_from_slice(&point_source_bytes);
    out.extend_from_slice(&gps_time_bytes);
    out.extend_from_slice(&rgb_bytes);
    out.extend_from_slice(&nir_bytes);
    for bytes in &extra_layer_bytes {
        out.extend_from_slice(bytes);
    }

    Ok(out)
}

/// Decode one standard LASzip layered Point14 v3 chunk and return decode status.
///
/// The status indicates whether tolerant recovery produced a partial decode.
pub fn decode_standard_layered_chunk_point14_v3_with_status(
    chunk_bytes: &[u8],
    point_count: usize,
    item_specs: &[LaszipItemSpec],
    point_data_format: PointDataFormat,
    scales: [f64; 3],
    offsets: [f64; 3],
) -> Result<(Vec<PointRecord>, Point14LayeredDecodeStatus)> {
    if point_count == 0 {
        return Ok((
            Vec::new(),
            Point14LayeredDecodeStatus {
                expected_points: 0,
                decoded_points: 0,
                partial: false,
            },
        ));
    }

    let mut cursor = Cursor::new(chunk_bytes);
    let mut out = Vec::with_capacity(point_count);
    let (core, extra) = decode_point14_item_set(&mut cursor, item_specs)?;
    let first_state = Point14State::from_raw(core, extra.clone());
    out.push(point14_record_from_parts(
        core,
        extra.as_deref(),
        point_data_format,
        scales,
        offsets,
    ));

    if point_count > 1 {
        let first_end = cursor.position() as usize;

        let layered_kind = if is_pure_point14_item10_v3(item_specs) {
            Some((false, false, 0usize))
        } else if let Some(extra_byte_count) = point14_layered_has_rgb(item_specs) {
            Some((true, false, extra_byte_count))
        } else if let Some(extra_byte_count) = point14_layered_has_rgb_nir(item_specs) {
            Some((true, true, extra_byte_count))
        } else {
            None
        };

        if let Some((has_rgb, has_nir, extra_byte_count)) = layered_kind {
            match decode_layered_point14_item10_continuation(
                first_state,
                &chunk_bytes[first_end..],
                point_count - 1,
                scales,
                offsets,
                has_rgb,
                has_nir,
                extra_byte_count,
            ) {
                Ok(tail) => {
                    out.extend(tail);
                }
                Err(_) => {
                    // Fallback for mixed streams that store plain per-point items
                    // after the first point.
                    let mut per_point_cursor = Cursor::new(&chunk_bytes[first_end..]);
                    let mut per_point_tail = Vec::with_capacity(point_count - 1);
                    let mut per_point_ok = true;
                    for _ in 1..point_count {
                        match decode_point14_item_set(&mut per_point_cursor, item_specs) {
                            Ok((core, extra)) => {
                                per_point_tail.push(point14_record_from_parts(
                                    core,
                                    extra.as_deref(),
                                    point_data_format,
                                    scales,
                                    offsets,
                                ));
                            }
                            Err(_) => {
                                per_point_ok = false;
                                break;
                            }
                        }
                    }
                    if per_point_ok
                        && per_point_cursor.position() as usize == chunk_bytes[first_end..].len()
                    {
                        out.extend(per_point_tail);
                    } else {
                        return Err(Error::Unimplemented(
                            "standard LASzip Point14 layered arithmetic continuation is not yet implemented",
                        ));
                    }
                }
            }
        } else {
            return Err(Error::Unimplemented(
                "standard LASzip Point14 layered arithmetic continuation is not yet implemented",
            ));
        }
    }

    if point_data_format.has_rgb() && out.iter().any(|p| p.color.is_none()) {
        // Keep tolerant behavior for non-conformant streams that omit RGB14.
    }
    if matches!(point_data_format, PointDataFormat::Pdrf8) && out.iter().any(|p| p.nir.is_none()) {
        // Keep tolerant behavior for non-conformant streams that omit NIR14.
    }

    let decoded_points = out.len();
    Ok((
        out,
        Point14LayeredDecodeStatus {
            expected_points: point_count,
            decoded_points,
            partial: decoded_points != point_count,
        },
    ))
}

fn decode_point14_item_set<R: Read>(
    cursor: &mut R,
    item_specs: &[LaszipItemSpec],
) -> Result<(RawPoint14, Option<Vec<u8>>)> {
    let mut core: Option<RawPoint14> = None;
    let mut pending_rgb: Option<Rgb16> = None;
    let mut pending_nir: Option<u16> = None;
    let mut extra_bytes = Vec::<u8>::new();

    for item in item_specs {
        match item.item_type {
            LASZIP_ITEM_POINT14 => {
                if item.item_version != 3 || item.item_size != 30 {
                    return Err(Error::Unimplemented(
                        "standard LASzip Point14 currently requires item version 3 and size 30",
                    ));
                }
                let mut buf = [0u8; 30];
                read_exact_or_layered_unimplemented(cursor, &mut buf)?;
                core = RawPoint14::from_bytes(&buf, PointDataFormat::Pdrf6);
                if let Some(c) = core.as_mut() {
                    if let Some(rgb) = pending_rgb.take() {
                        c.rgb = Some(rgb);
                    }
                    if let Some(nir) = pending_nir.take() {
                        c.nir = Some(nir);
                    }
                }
            }
            LASZIP_ITEM_RGB14 => {
                if item.item_size != 6 {
                    return Err(Error::Unimplemented(
                        "standard LASzip RGB14 currently requires item size 6",
                    ));
                }
                let mut buf = [0u8; 6];
                read_exact_or_layered_unimplemented(cursor, &mut buf)?;
                let rgb = Rgb16 {
                    red: u16::from_le_bytes([buf[0], buf[1]]),
                    green: u16::from_le_bytes([buf[2], buf[3]]),
                    blue: u16::from_le_bytes([buf[4], buf[5]]),
                };
                if let Some(c) = core.as_mut() {
                    c.rgb = Some(rgb);
                } else {
                    pending_rgb = Some(rgb);
                }
            }
            LASZIP_ITEM_RGBNIR14 => {
                if item.item_size == 2 {
                    let mut buf = [0u8; 2];
                    read_exact_or_layered_unimplemented(cursor, &mut buf)?;
                    let nir = u16::from_le_bytes(buf);
                    if let Some(c) = core.as_mut() {
                        c.nir = Some(nir);
                    } else {
                        pending_nir = Some(nir);
                    }
                    continue;
                }
                if item.item_size != 8 {
                    return Err(Error::Unimplemented(
                        "standard LASzip RGBNIR14 currently requires item size 8",
                    ));
                }
                let mut buf = [0u8; 8];
                read_exact_or_layered_unimplemented(cursor, &mut buf)?;
                let rgb = Rgb16 {
                    red: u16::from_le_bytes([buf[0], buf[1]]),
                    green: u16::from_le_bytes([buf[2], buf[3]]),
                    blue: u16::from_le_bytes([buf[4], buf[5]]),
                };
                let nir = u16::from_le_bytes([buf[6], buf[7]]);
                if let Some(c) = core.as_mut() {
                    c.rgb = Some(rgb);
                    c.nir = Some(nir);
                } else {
                    pending_rgb = Some(rgb);
                    pending_nir = Some(nir);
                }
            }
            LASZIP_ITEM_BYTE | LASZIP_ITEM_BYTE14 => {
                if item.item_version != 2 && item.item_version != 3 {
                    return Err(Error::Unimplemented(
                        "standard LASzip extra-bytes currently requires item version 2 or 3",
                    ));
                }
                let mut bytes = vec![0u8; item.item_size as usize];
                read_exact_or_layered_unimplemented(cursor, &mut bytes)?;
                extra_bytes.extend_from_slice(&bytes);
            }
            LASZIP_ITEM_WAVEPACKET14 => {
                if item.item_version != 3 {
                    return Err(Error::Unimplemented(
                        "standard LASzip WavePacket14 currently requires item version 3",
                    ));
                }
                let mut bytes = vec![0u8; item.item_size as usize];
                read_exact_or_layered_unimplemented(cursor, &mut bytes)?;
                extra_bytes.extend_from_slice(&bytes);
            }
            _ => {
                return Err(Error::Unimplemented(
                    "standard LASzip Point14 path does not yet support this item type",
                ));
            }
        }
    }

    let core = core.ok_or_else(|| Error::InvalidValue {
        field: "laz.laszip_items",
        detail: "Point14 item missing for Point14 standard decode path".to_string(),
    })?;

    let extra = if extra_bytes.is_empty() {
        None
    } else {
        Some(extra_bytes)
    };

    Ok((core, extra))
}

fn read_exact_or_layered_unimplemented<R: Read>(reader: &mut R, buf: &mut [u8]) -> Result<()> {
    match reader.read_exact(buf) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::UnexpectedEof => Err(Error::Unimplemented(
            "standard LASzip Point14 layered arithmetic continuation is not yet implemented",
        )),
        Err(e) => Err(Error::Io(e)),
    }
}

fn point14_record_from_parts(
    core: RawPoint14,
    extra_bytes: Option<&[u8]>,
    point_data_format: PointDataFormat,
    scales: [f64; 3],
    offsets: [f64; 3],
) -> PointRecord {
    let mut out = core.to_point_record(scales, offsets);
    if let Some(extra) = extra_bytes {
        let mut payload = extra;
        if point_data_format.has_waveform() && payload.len() >= 29 {
            out.waveform = Some(WaveformPacket {
                descriptor_index: payload[0],
                byte_offset: u64::from_le_bytes(payload[1..9].try_into().unwrap()),
                packet_size: u32::from_le_bytes(payload[9..13].try_into().unwrap()),
                return_point_location: f32::from_le_bytes(payload[13..17].try_into().unwrap()),
                dx: f32::from_le_bytes(payload[17..21].try_into().unwrap()),
                dy: f32::from_le_bytes(payload[21..25].try_into().unwrap()),
                dz: f32::from_le_bytes(payload[25..29].try_into().unwrap()),
            });
            payload = &payload[29..];
        }
        let copy_len = usize::min(payload.len(), 192);
        out.extra_bytes.data[..copy_len].copy_from_slice(&payload[..copy_len]);
        out.extra_bytes.len = copy_len as u8;
    }
    out
}

fn is_pure_point14_item10_v3(item_specs: &[LaszipItemSpec]) -> bool {
    item_specs.len() == 1
        && item_specs[0].item_type == LASZIP_ITEM_POINT14
        && item_specs[0].item_size == 30
        && item_specs[0].item_version == 3
}

/// Returns true when the item set is [POINT14 v3, RGB14 v3] (PDRF7 layered).
fn point14_layered_has_rgb(item_specs: &[LaszipItemSpec]) -> Option<usize> {
    if item_specs.len() < 2 {
        return None;
    }
    if !(item_specs[0].item_type == LASZIP_ITEM_POINT14
        && item_specs[0].item_size == 30
        && item_specs[0].item_version == 3
        && item_specs[1].item_type == LASZIP_ITEM_RGB14
        && item_specs[1].item_size == 6
        && item_specs[1].item_version == 3)
    {
        return None;
    }
    let mut extra_byte_count = 0usize;
    for item in &item_specs[2..] {
        if item.item_type != LASZIP_ITEM_BYTE14 || (item.item_version != 2 && item.item_version != 3) {
            return None;
        }
        extra_byte_count += item.item_size as usize;
    }
    Some(extra_byte_count)
}

/// Returns extra-byte count when the item set is [POINT14 v3, RGBNIR14 v3, BYTE14*].
fn point14_layered_has_rgb_nir(item_specs: &[LaszipItemSpec]) -> Option<usize> {
    if item_specs.len() < 2 {
        return None;
    }
    let extras = if item_specs[0].item_type == LASZIP_ITEM_POINT14
        && item_specs[0].item_size == 30
        && item_specs[0].item_version == 3
        && item_specs[1].item_type == LASZIP_ITEM_RGBNIR14
        && item_specs[1].item_size == 8
        && item_specs[1].item_version == 3
    {
        &item_specs[2..]
    } else if item_specs.len() >= 3
        && item_specs[0].item_type == LASZIP_ITEM_POINT14
        && item_specs[0].item_size == 30
        && item_specs[0].item_version == 3
        && item_specs[1].item_type == LASZIP_ITEM_RGB14
        && item_specs[1].item_size == 6
        && item_specs[1].item_version == 3
        && item_specs[2].item_type == LASZIP_ITEM_RGBNIR14
        && item_specs[2].item_size == 2
        && item_specs[2].item_version == 3
    {
        &item_specs[3..]
    } else {
        return None;
    };

    let mut extra_byte_count = 0usize;
    for item in extras {
        if item.item_type != LASZIP_ITEM_BYTE14 || (item.item_version != 2 && item.item_version != 3) {
            return None;
        }
        extra_byte_count += item.item_size as usize;
    }
    Some(extra_byte_count)
}

fn read_u32_le_at(buf: &[u8], offset: usize) -> Result<u32> {
    if offset + 4 > buf.len() {
        return Err(Error::Unimplemented(
            "standard LASzip Point14 layered header is truncated",
        ));
    }
    Ok(u32::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ]))
}

fn map_point14_layer_error<E: Into<Error>>(
    err: E,
    layer: &'static str,
    point_index: Option<usize>,
) -> Error {
    let err: Error = err.into();
    let at = point_index
        .map(|i| format!("continuation point {}", i + 1))
        .unwrap_or_else(|| "layer initialization".to_string());
    Error::InvalidValue {
        field: "laz.point14.layered_decode",
        detail: format!("{at} in {layer} layer failed: {err}"),
    }
}

fn point14_layered_tolerant_mode() -> bool {
    match std::env::var("WBLIDAR_STRICT_POINT14_LAYERED") {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            !(v == "1" || v == "true" || v == "yes" || v == "on")
        }
        Err(_) => true,
    }
}

fn decode_layered_point14_item10_continuation(
    seed: Point14State,
    tail_bytes: &[u8],
    count: usize,
    scales: [f64; 3],
    offsets: [f64; 3],
    has_rgb: bool,
    has_nir: bool,
    extra_byte_count: usize,
) -> Result<Vec<PointRecord>> {
    let tolerant_mode = point14_layered_tolerant_mode();

    // The binary layout after the seed point is (per lasreadpoint.cpp):
    //   [count: u32]                  – number of remaining points (already known as `count`)
    //   [num_bytes_channel_returns_XY: u32]
    //   [num_bytes_Z: u32]
    //   [num_bytes_classification: u32]
    //   [num_bytes_flags: u32]
    //   [num_bytes_intensity: u32]
    //   [num_bytes_scan_angle: u32]
    //   [num_bytes_user_data: u32]
    //   [num_bytes_point_source: u32]
    //   [num_bytes_gps_time: u32]
    //   [layer data bytes ...]
    // Total header = 4 (count) + 9 × 4 (layer sizes) = 40 bytes.

    let min_header = (if has_nir { 48 } else if has_rgb { 44 } else { 40 }) + extra_byte_count * 4;
    if tail_bytes.len() < min_header {
        return Err(Error::InvalidValue {
            field: "laz.point14.layered_header",
            detail: format!(
                "tail has {} bytes but layered Point14{}{} requires at least {} bytes (count + {} layer sizes)",
                tail_bytes.len(),
                if has_rgb { "+RGB" } else { "" },
                if has_nir { "+NIR" } else { "" },
                min_header,
                (if has_nir { 11 } else if has_rgb { 10 } else { 9 }) + extra_byte_count,
            ),
        });
    }

    // Skip the 4-byte `count` field at offset 0; read layer sizes from offsets 4..40 (+RGB at 40, +NIR at 44).
    let num_bytes_xy = read_u32_le_at(tail_bytes, 4)? as usize;
    let num_bytes_z = read_u32_le_at(tail_bytes, 8)? as usize;
    let num_bytes_classification = read_u32_le_at(tail_bytes, 12)? as usize;
    let num_bytes_flags = read_u32_le_at(tail_bytes, 16)? as usize;
    let num_bytes_intensity = read_u32_le_at(tail_bytes, 20)? as usize;
    let num_bytes_scan_angle = read_u32_le_at(tail_bytes, 24)? as usize;
    let num_bytes_user_data = read_u32_le_at(tail_bytes, 28)? as usize;
    let num_bytes_point_source = read_u32_le_at(tail_bytes, 32)? as usize;
    let num_bytes_gps_time = read_u32_le_at(tail_bytes, 36)? as usize;
    let num_bytes_rgb = if has_rgb {
        read_u32_le_at(tail_bytes, 40)? as usize
    } else {
        0
    };
    let num_bytes_nir = if has_nir {
        read_u32_le_at(tail_bytes, 44)? as usize
    } else {
        0
    };
    let extra_sizes_base = if has_nir { 48 } else if has_rgb { 44 } else { 40 };
    let mut num_bytes_extra = Vec::with_capacity(extra_byte_count);
    for idx in 0..extra_byte_count {
        num_bytes_extra.push(read_u32_le_at(tail_bytes, extra_sizes_base + idx * 4)? as usize);
    }

    let mut off = min_header;
    let mut layer_overflowed = false;
    let split_tolerant = |n: usize, off_ref: &mut usize, overflowed: &mut bool| -> &[u8] {
        if *overflowed {
            return &tail_bytes[0..0];
        }
        let remaining = tail_bytes.len().saturating_sub(*off_ref);
        if n > remaining {
            *overflowed = true;
            return &tail_bytes[0..0];
        }
        let s = &tail_bytes[*off_ref..*off_ref + n];
        *off_ref += n;
        s
    };

    // Tolerate malformed streams that over-declare one or more layer sizes by
    // clamping each layer to the remaining payload bytes.
    let bytes_xy = split_tolerant(num_bytes_xy, &mut off, &mut layer_overflowed);
    let bytes_z = split_tolerant(num_bytes_z, &mut off, &mut layer_overflowed);
    let bytes_classification =
        split_tolerant(num_bytes_classification, &mut off, &mut layer_overflowed);
    let bytes_flags = split_tolerant(num_bytes_flags, &mut off, &mut layer_overflowed);
    let bytes_intensity = split_tolerant(num_bytes_intensity, &mut off, &mut layer_overflowed);
    let bytes_scan_angle = split_tolerant(num_bytes_scan_angle, &mut off, &mut layer_overflowed);
    let bytes_user_data = split_tolerant(num_bytes_user_data, &mut off, &mut layer_overflowed);
    let bytes_point_source =
        split_tolerant(num_bytes_point_source, &mut off, &mut layer_overflowed);
    let bytes_gps_time = split_tolerant(num_bytes_gps_time, &mut off, &mut layer_overflowed);
    let bytes_rgb = split_tolerant(num_bytes_rgb, &mut off, &mut layer_overflowed);
    let bytes_nir = split_tolerant(num_bytes_nir, &mut off, &mut layer_overflowed);
    let mut bytes_extra = Vec::with_capacity(extra_byte_count);
    for size in &num_bytes_extra {
        bytes_extra.push(split_tolerant(*size, &mut off, &mut layer_overflowed));
    }

    if layer_overflowed && !tolerant_mode {
        return Err(Error::InvalidValue {
            field: "laz.point14.layered_payload",
            detail: format!(
                "strict mode: layered payload declarations exceed available tail bytes (tail={}, declared_total={})",
                tail_bytes.len(),
                min_header + num_bytes_xy + num_bytes_z + num_bytes_classification + num_bytes_flags
                    + num_bytes_intensity + num_bytes_scan_angle + num_bytes_user_data
                    + num_bytes_point_source + num_bytes_gps_time + num_bytes_rgb + num_bytes_nir
                    + num_bytes_extra.iter().sum::<usize>()
            ),
        });
    }

    if bytes_xy.is_empty() {
        return Err(Error::InvalidValue {
            field: "laz.point14.layered_payload",
            detail: "missing required non-empty channel_returns_XY layer".to_string(),
        });
    }

    let mut dec_xy = ArithmeticDecoder::new(Cursor::new(bytes_xy));
    dec_xy
        .read_init_bytes()
        .map_err(|e| map_point14_layer_error(e, "channel_returns_xy", None))?;

    let mut dec_z = if !bytes_z.is_empty() {
        let mut d = ArithmeticDecoder::new(Cursor::new(bytes_z));
        d.read_init_bytes()
            .map_err(|e| map_point14_layer_error(e, "z", None))?;
        Some(d)
    } else {
        None
    };

    let mut dec_classification = if !bytes_classification.is_empty() {
        let mut d = ArithmeticDecoder::new(Cursor::new(bytes_classification));
        d.read_init_bytes()
            .map_err(|e| map_point14_layer_error(e, "classification", None))?;
        Some(d)
    } else {
        None
    };

    let mut dec_flags = if !bytes_flags.is_empty() {
        let mut d = ArithmeticDecoder::new(Cursor::new(bytes_flags));
        d.read_init_bytes()
            .map_err(|e| map_point14_layer_error(e, "flags", None))?;
        Some(d)
    } else {
        None
    };

    let mut dec_intensity = if !bytes_intensity.is_empty() {
        let mut d = ArithmeticDecoder::new(Cursor::new(bytes_intensity));
        d.read_init_bytes()
            .map_err(|e| map_point14_layer_error(e, "intensity", None))?;
        Some(d)
    } else {
        None
    };

    let mut dec_scan_angle = if !bytes_scan_angle.is_empty() {
        let mut d = ArithmeticDecoder::new(Cursor::new(bytes_scan_angle));
        d.read_init_bytes()
            .map_err(|e| map_point14_layer_error(e, "scan_angle", None))?;
        Some(d)
    } else {
        None
    };

    let mut dec_user_data = if !bytes_user_data.is_empty() {
        let mut d = ArithmeticDecoder::new(Cursor::new(bytes_user_data));
        d.read_init_bytes()
            .map_err(|e| map_point14_layer_error(e, "user_data", None))?;
        Some(d)
    } else {
        None
    };

    let mut dec_point_source = if !bytes_point_source.is_empty() {
        let mut d = ArithmeticDecoder::new(Cursor::new(bytes_point_source));
        d.read_init_bytes()
            .map_err(|e| map_point14_layer_error(e, "point_source", None))?;
        Some(d)
    } else {
        None
    };

    let mut dec_gps_time = if !bytes_gps_time.is_empty() {
        let mut d = ArithmeticDecoder::new(Cursor::new(bytes_gps_time));
        d.read_init_bytes()
            .map_err(|e| map_point14_layer_error(e, "gps_time", None))?;
        Some(d)
    } else {
        None
    };

    let mut dec_rgb = if has_rgb && !bytes_rgb.is_empty() {
        let mut d = ArithmeticDecoder::new(Cursor::new(bytes_rgb));
        d.read_init_bytes()
            .map_err(|e| map_point14_layer_error(e, "rgb", None))?;
        Some(d)
    } else {
        None
    };

    let mut dec_nir = if has_nir && !bytes_nir.is_empty() {
        let mut d = ArithmeticDecoder::new(Cursor::new(bytes_nir));
        d.read_init_bytes()
            .map_err(|e| map_point14_layer_error(e, "nir", None))?;
        Some(d)
    } else {
        None
    };

    let mut dec_extra = Vec::with_capacity(extra_byte_count);
    for bytes in bytes_extra {
        if bytes.is_empty() {
            dec_extra.push(None);
        } else {
            let mut d = ArithmeticDecoder::new(Cursor::new(bytes));
            d.read_init_bytes()
                .map_err(|e| map_point14_layer_error(e, "extra_bytes", None))?;
            dec_extra.push(Some(d));
        }
    }

    let mut channel_contexts: [Option<Point14ContinuationContext>; 4] =
        std::array::from_fn(|_| None);
    channel_contexts[0] = Some(Point14ContinuationContext::new(seed));
    let mut current_channel: usize = 0;
    let mut out = Vec::with_capacity(count);

    for point_index in 0..count {
        let lpr = {
            let ctx = channel_contexts[current_channel]
                .as_ref()
                .ok_or_else(|| Error::InvalidValue {
                    field: "laz.point14.context",
                    detail: "missing Point14 continuation context".to_string(),
                })?;
            let mut v = if ctx.state.return_number == 1 { 1 } else { 0 };
            v += if ctx.state.return_number >= ctx.state.number_of_returns {
                2
            } else {
                0
            };
            v + if ctx.state.gps_time_change { 4 } else { 0 }
        };

        let changed_values = {
            let ctx = channel_contexts[current_channel]
                .as_mut()
                .ok_or_else(|| Error::InvalidValue {
                    field: "laz.point14.context",
                    detail: "missing Point14 continuation context".to_string(),
                })?;
            match dec_xy.decode_symbol(&mut ctx.m_changed_values[lpr as usize]) {
                Ok(v) => v as u8,
                Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                    if tolerant_mode {
                        break;
                    }
                    return Err(map_point14_layer_error(
                        Error::Io(e),
                        "channel_returns_xy",
                        Some(point_index),
                    ));
                }
                Err(e) => {
                    return Err(map_point14_layer_error(
                        Error::Io(e),
                        "channel_returns_xy",
                        Some(point_index),
                    ));
                }
            }
        };

        // Handle scanner-channel switching (bit 6 of changed_values).
        // LASzip encodes channel as a forward delta in [1, 3].
        if (changed_values & (1 << 6)) != 0 {
            let diff = {
                let ctx = channel_contexts[current_channel]
                    .as_mut()
                    .ok_or_else(|| Error::InvalidValue {
                        field: "laz.point14.context",
                        detail: "missing Point14 continuation context".to_string(),
                    })?;
                match dec_xy.decode_symbol(&mut ctx.m_scanner_channel) {
                    Ok(v) => v as usize,
                    Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                        if tolerant_mode {
                            break;
                        }
                        return Err(map_point14_layer_error(
                            Error::Io(e),
                            "channel_returns_xy",
                            Some(point_index),
                        ));
                    }
                    Err(e) => {
                        return Err(map_point14_layer_error(
                            Error::Io(e),
                            "channel_returns_xy",
                            Some(point_index),
                        ));
                    }
                }
            };
            let next_channel = (current_channel + diff + 1) & 0x03;

            if channel_contexts[next_channel].is_none() {
                let mut seed_state = channel_contexts[current_channel]
                    .as_ref()
                    .ok_or_else(|| Error::InvalidValue {
                        field: "laz.point14.context",
                        detail: "missing Point14 continuation context".to_string(),
                    })?
                    .state
                    .clone();
                seed_state.gps_time_change = false;
                channel_contexts[next_channel] = Some(Point14ContinuationContext::new(seed_state));
            }

            current_channel = next_channel;
        }

        let ctx = channel_contexts[current_channel]
            .as_mut()
            .ok_or_else(|| Error::InvalidValue {
                field: "laz.point14.context",
                detail: "missing Point14 continuation context".to_string(),
            })?;

        let point_source_change = (changed_values & (1 << 5)) != 0;
        let mut gps_time_change = (changed_values & (1 << 4)) != 0;
        let scan_angle_change = (changed_values & (1 << 3)) != 0;

        let last_n = ctx.state.number_of_returns as usize;
        let last_r = ctx.state.return_number as usize;

        let n = if (changed_values & (1 << 2)) != 0 {
            if ctx.m_number_of_returns[last_n].is_none() {
                ctx.m_number_of_returns[last_n] = Some(ArithmeticSymbolModel::new(16));
            }
            match dec_xy.decode_symbol(ctx.m_number_of_returns[last_n].as_mut().unwrap()) {
                Ok(v) => v as usize,
                Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                    if tolerant_mode {
                        break;
                    }
                    return Err(map_point14_layer_error(
                        Error::Io(e),
                        "channel_returns_xy",
                        Some(point_index),
                    ));
                }
                Err(e) => {
                    return Err(map_point14_layer_error(
                        Error::Io(e),
                        "channel_returns_xy",
                        Some(point_index),
                    ));
                }
            }
        } else {
            last_n
        };
        ctx.state.number_of_returns = n as u8;

        let r = match changed_values & 0x03 {
            0 => last_r,
            1 => (last_r + 1) % 16,
            2 => (last_r + 15) % 16,
            _ => {
                if gps_time_change {
                    if ctx.m_return_number[last_r].is_none() {
                        ctx.m_return_number[last_r] = Some(ArithmeticSymbolModel::new(16));
                    }
                    match dec_xy.decode_symbol(ctx.m_return_number[last_r].as_mut().unwrap()) {
                        Ok(v) => v as usize,
                        Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                            if tolerant_mode {
                                break;
                            }
                            return Err(map_point14_layer_error(
                                Error::Io(e),
                                "channel_returns_xy",
                                Some(point_index),
                            ));
                        }
                        Err(e) => {
                            return Err(map_point14_layer_error(
                                Error::Io(e),
                                "channel_returns_xy",
                                Some(point_index),
                            ));
                        }
                    }
                } else {
                    let sym = match dec_xy.decode_symbol(&mut ctx.m_return_number_gps_same) {
                        Ok(v) => v as usize,
                        Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                            if tolerant_mode {
                                break;
                            }
                            return Err(map_point14_layer_error(
                                Error::Io(e),
                                "channel_returns_xy",
                                Some(point_index),
                            ));
                        }
                        Err(e) => {
                            return Err(map_point14_layer_error(
                                Error::Io(e),
                                "channel_returns_xy",
                                Some(point_index),
                            ));
                        }
                    };
                    (last_r + sym + 2) % 16
                }
            }
        };
        ctx.state.return_number = r as u8;

        let m = NUMBER_RETURN_MAP_6CTX[n][r] as usize;
        let l = NUMBER_RETURN_LEVEL_8CTX[n][r] as usize;
        let cpr = (if r == 1 { 2 } else { 0 }) + (if r >= n { 1 } else { 0 });

        let idx = (m << 1) | (gps_time_change as usize);
        let median_x = ctx.last_x_diff_median[idx].get();
        let diff_x = match ctx.ic_dx.decompress(&mut dec_xy, median_x, (n == 1) as u32) {
            Ok(v) => v,
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                if tolerant_mode {
                    break;
                }
                return Err(map_point14_layer_error(
                    Error::Io(e),
                    "channel_returns_xy",
                    Some(point_index),
                ));
            }
            Err(e) => {
                return Err(map_point14_layer_error(
                    Error::Io(e),
                    "channel_returns_xy",
                    Some(point_index),
                ));
            }
        };
        ctx.state.x = ctx.state.x.wrapping_add(diff_x);
        ctx.last_x_diff_median[idx].add(diff_x);

        let k_bits = ctx.ic_dx.k();
        let median_y = ctx.last_y_diff_median[idx].get();
        let context_y = (n == 1) as u32 + if k_bits < 20 { u32_zero_bit(k_bits) } else { 20 };
        let diff_y = match ctx.ic_dy.decompress(&mut dec_xy, median_y, context_y) {
            Ok(v) => v,
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                if tolerant_mode {
                    break;
                }
                return Err(map_point14_layer_error(
                    Error::Io(e),
                    "channel_returns_xy",
                    Some(point_index),
                ));
            }
            Err(e) => {
                return Err(map_point14_layer_error(
                    Error::Io(e),
                    "channel_returns_xy",
                    Some(point_index),
                ));
            }
        };
        ctx.state.y = ctx.state.y.wrapping_add(diff_y);
        ctx.last_y_diff_median[idx].add(diff_y);

        if let Some(dec) = dec_z.as_mut() {
            let k_bits = (ctx.ic_dx.k() + ctx.ic_dy.k()) / 2;
            let context_z = (n == 1) as u32 + if k_bits < 18 { u32_zero_bit(k_bits) } else { 18 };
            match ctx.ic_z.decompress(dec, ctx.last_z[l], context_z) {
                Ok(z) => {
                    ctx.state.z = z;
                    ctx.last_z[l] = ctx.state.z;
                }
                Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                    if tolerant_mode {
                        dec_z = None;
                    } else {
                        return Err(map_point14_layer_error(Error::Io(e), "z", Some(point_index)));
                    }
                }
                Err(e) => {
                    return Err(map_point14_layer_error(Error::Io(e), "z", Some(point_index)));
                }
            }
        }

        if let Some(dec) = dec_classification.as_mut() {
            let last_classification = ctx.state.classification;
            let ccc = (((last_classification & 0x1F) << 1) + if cpr == 3 { 1 } else { 0 }) as usize;
            if ctx.m_classification[ccc].is_none() {
                ctx.m_classification[ccc] = Some(ArithmeticSymbolModel::new(256));
            }
            match dec.decode_symbol(ctx.m_classification[ccc].as_mut().unwrap()) {
                Ok(v) => {
                    ctx.state.classification = v as u8;
                }
                Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                    if tolerant_mode {
                        dec_classification = None;
                    } else {
                        return Err(map_point14_layer_error(Error::Io(e), "classification", Some(point_index)));
                    }
                }
                Err(e) => {
                    return Err(map_point14_layer_error(Error::Io(e), "classification", Some(point_index)));
                }
            }
        }

        if let Some(dec) = dec_flags.as_mut() {
            let last_flags = ((ctx.state.edge_of_flight_line as u8) << 5)
                | ((ctx.state.scan_direction_flag as u8) << 4)
                | (ctx.state.classification_flags & 0x0F);
            let idx = last_flags as usize;
            if ctx.m_flags[idx].is_none() {
                ctx.m_flags[idx] = Some(ArithmeticSymbolModel::new(64));
            }
            match dec.decode_symbol(ctx.m_flags[idx].as_mut().unwrap()) {
                Ok(v) => {
                    let flags = v as u8;
                    ctx.state.edge_of_flight_line = (flags & (1 << 5)) != 0;
                    ctx.state.scan_direction_flag = (flags & (1 << 4)) != 0;
                    ctx.state.classification_flags = flags & 0x0F;
                }
                Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                    if tolerant_mode {
                        dec_flags = None;
                    } else {
                        return Err(map_point14_layer_error(Error::Io(e), "flags", Some(point_index)));
                    }
                }
                Err(e) => {
                    return Err(map_point14_layer_error(Error::Io(e), "flags", Some(point_index)));
                }
            }
        }

        if let Some(dec) = dec_intensity.as_mut() {
            let iidx = ((cpr << 1) | (gps_time_change as usize)) as usize;
            match ctx
                .ic_intensity
                .decompress(dec, ctx.last_intensity[iidx] as i32, cpr as u32)
            {
                Ok(v) => {
                    let intensity = v as u16;
                    ctx.last_intensity[iidx] = intensity;
                    ctx.state.intensity = intensity;
                }
                Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                    if tolerant_mode {
                        dec_intensity = None;
                    } else {
                        return Err(map_point14_layer_error(Error::Io(e), "intensity", Some(point_index)));
                    }
                }
                Err(e) => {
                    return Err(map_point14_layer_error(Error::Io(e), "intensity", Some(point_index)));
                }
            }
        }

        if let Some(dec) = dec_scan_angle.as_mut() {
            if scan_angle_change {
                match ctx
                    .ic_scan_angle
                    .decompress(dec, ctx.state.scan_angle as i32, gps_time_change as u32)
                {
                    Ok(v) => {
                        ctx.state.scan_angle = v as i16;
                    }
                    Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                        if tolerant_mode {
                            dec_scan_angle = None;
                        } else {
                            return Err(map_point14_layer_error(Error::Io(e), "scan_angle", Some(point_index)));
                        }
                    }
                    Err(e) => {
                        return Err(map_point14_layer_error(Error::Io(e), "scan_angle", Some(point_index)));
                    }
                }
            }
        }

        if let Some(dec) = dec_user_data.as_mut() {
            let uidx = (ctx.state.user_data / 4) as usize;
            if ctx.m_user_data[uidx].is_none() {
                ctx.m_user_data[uidx] = Some(ArithmeticSymbolModel::new(256));
            }
            match dec.decode_symbol(ctx.m_user_data[uidx].as_mut().unwrap()) {
                Ok(v) => {
                    ctx.state.user_data = v as u8;
                }
                Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                    if tolerant_mode {
                        dec_user_data = None;
                    } else {
                        return Err(map_point14_layer_error(Error::Io(e), "user_data", Some(point_index)));
                    }
                }
                Err(e) => {
                    return Err(map_point14_layer_error(Error::Io(e), "user_data", Some(point_index)));
                }
            }
        }

        if let Some(dec) = dec_point_source.as_mut() {
            if point_source_change {
                match ctx
                    .ic_point_source_id
                    .decompress(dec, ctx.state.point_source_id as i32, 0)
                {
                    Ok(v) => {
                        ctx.state.point_source_id = v as u16;
                    }
                    Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                        if tolerant_mode {
                            dec_point_source = None;
                        } else {
                            return Err(map_point14_layer_error(Error::Io(e), "point_source", Some(point_index)));
                        }
                    }
                    Err(e) => {
                        return Err(map_point14_layer_error(Error::Io(e), "point_source", Some(point_index)));
                    }
                }
            }
        }

        if let Some(dec) = dec_gps_time.as_mut() {
            if gps_time_change {
                match read_gps_time(ctx, dec) {
                    Ok(()) => {
                        ctx.state.gps_bits = ctx.last_gps[ctx.gps_last];
                    }
                    Err(Error::Io(e)) if e.kind() == ErrorKind::UnexpectedEof => {
                        if tolerant_mode {
                            dec_gps_time = None;
                            gps_time_change = false;
                        } else {
                            return Err(map_point14_layer_error(Error::Io(e), "gps_time", Some(point_index)));
                        }
                    }
                    Err(e) => {
                        return Err(map_point14_layer_error(e, "gps_time", Some(point_index)));
                    }
                }
            }
        }

        ctx.state.gps_time_change = gps_time_change;

        if has_rgb {
            if let Some(dec) = dec_rgb.as_mut() {
                match read_rgb(ctx, dec, has_nir) {
                    Ok(()) => {}
                    Err(Error::Io(e)) if e.kind() == ErrorKind::UnexpectedEof => {
                        if tolerant_mode {
                            dec_rgb = None;
                        } else {
                            return Err(map_point14_layer_error(Error::Io(e), "rgb", Some(point_index)));
                        }
                    }
                    Err(e) => {
                        return Err(map_point14_layer_error(e, "rgb", Some(point_index)));
                    }
                }
            }
        }

        if has_nir {
            if let Some(dec) = dec_nir.as_mut() {
                match read_nir(ctx, dec) {
                    Ok(()) => {}
                    Err(Error::Io(e)) if e.kind() == ErrorKind::UnexpectedEof => {
                        if tolerant_mode {
                            dec_nir = None;
                        } else {
                            return Err(map_point14_layer_error(Error::Io(e), "nir", Some(point_index)));
                        }
                    }
                    Err(e) => {
                        return Err(map_point14_layer_error(e, "nir", Some(point_index)));
                    }
                }
            }
        }

        if extra_byte_count > 0 {
            let extra = ctx
                .state
                .extra_bytes
                .get_or_insert_with(|| vec![0; extra_byte_count]);
            if extra.len() != extra_byte_count || ctx.m_extra_bytes.len() != extra_byte_count {
                return Err(Error::InvalidValue {
                    field: "laz.point14.extra_bytes",
                    detail: format!(
                        "BYTE14 continuation expected {} bytes but context has {} bytes and {} models",
                        extra_byte_count,
                        extra.len(),
                        ctx.m_extra_bytes.len()
                    ),
                });
            }

            for byte_idx in 0..extra_byte_count {
                let Some(mut dec) = dec_extra[byte_idx].take() else {
                    continue;
                };

                match dec.decode_symbol(&mut ctx.m_extra_bytes[byte_idx]) {
                    Ok(v) => {
                        extra[byte_idx] = u8_fold(extra[byte_idx] as i32 + v as i32);
                        dec_extra[byte_idx] = Some(dec);
                    }
                    Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                        if !tolerant_mode {
                            return Err(map_point14_layer_error(
                                Error::Io(e),
                                "extra_bytes",
                                Some(point_index),
                            ));
                        }
                    }
                    Err(e) => {
                        return Err(map_point14_layer_error(
                            Error::Io(e),
                            "extra_bytes",
                            Some(point_index),
                        ));
                    }
                }
            }
        }

        out.push(ctx.state.to_point_record(scales, offsets));
    }

    Ok(out)
}

/// Wrapping folded byte: mirrors C++ `U8_FOLD(x)`.
#[inline]
fn u8_fold(x: i32) -> u8 {
    if x > 255 { (x - 256) as u8 } else if x < 0 { (x + 256) as u8 } else { x as u8 }
}

/// Clamped byte: mirrors C++ `U8_CLAMP(n)`.
#[inline]
fn u8_clamp(n: i32) -> i32 {
    if n <= 0 { 0 } else if n >= 255 { 255 } else { n }
}

fn write_nir<W: Write>(
    last_nir: u16,
    nir: u16,
    m_nir_byte_used: &mut ArithmeticSymbolModel,
    m_nir_diff: &mut [ArithmeticSymbolModel; 2],
    enc_nir: &mut ArithmeticEncoder<W>,
) -> Result<()> {
    let last_lo = (last_nir & 0x00FF) as u8;
    let last_hi = (last_nir >> 8) as u8;
    let lo = (nir & 0x00FF) as u8;
    let hi = (nir >> 8) as u8;

    let mut sym = 0u32;
    if lo != last_lo {
        sym |= 1 << 0;
    }
    if hi != last_hi {
        sym |= 1 << 1;
    }

    enc_nir
        .encode_symbol(m_nir_byte_used, sym)
        .map_err(Error::Io)?;

    if (sym & (1 << 0)) != 0 {
        let corr = lo.wrapping_sub(last_lo) as u32;
        enc_nir
            .encode_symbol(&mut m_nir_diff[0], corr)
            .map_err(Error::Io)?;
    }
    if (sym & (1 << 1)) != 0 {
        let corr = hi.wrapping_sub(last_hi) as u32;
        enc_nir
            .encode_symbol(&mut m_nir_diff[1], corr)
            .map_err(Error::Io)?;
    }

    Ok(())
}

fn write_rgb<W: Write>(
    last_rgb: [u16; 3],
    rgb: [u16; 3],
    m_rgb_byte_used: &mut ArithmeticSymbolModel,
    m_rgb_byte_used_rgbnir: &mut ArithmeticSymbolModel,
    m_rgb_diff: &mut [ArithmeticSymbolModel; 6],
    enc_rgb: &mut ArithmeticEncoder<W>,
    rgbnir_mode: bool,
) -> Result<()> {
    let last_r_lo = (last_rgb[0] & 0x00FF) as u8;
    let last_r_hi = (last_rgb[0] >> 8) as u8;
    let last_g_lo = (last_rgb[1] & 0x00FF) as u8;
    let last_g_hi = (last_rgb[1] >> 8) as u8;
    let last_b_lo = (last_rgb[2] & 0x00FF) as u8;
    let last_b_hi = (last_rgb[2] >> 8) as u8;

    let r_lo = (rgb[0] & 0x00FF) as u8;
    let r_hi = (rgb[0] >> 8) as u8;
    let g_lo = (rgb[1] & 0x00FF) as u8;
    let g_hi = (rgb[1] >> 8) as u8;
    let b_lo = (rgb[2] & 0x00FF) as u8;
    let b_hi = (rgb[2] >> 8) as u8;

    if !rgbnir_mode {
        // RGB14 (PDRF7) uses the same 128-symbol bit-6 + predictive algorithm
        // as RGBNIR14 — the only difference is which model variable is used.
        // C++ reference: LASwriteItemCompressed_RGB14_v3::write(), m_byte_used=128.
        let gb_copy_from_red = rgb[1] == rgb[0] && rgb[2] == rgb[0];
        let mut sym = 0u32;
        if r_lo != last_r_lo { sym |= 1 << 0; }
        if r_hi != last_r_hi { sym |= 1 << 1; }
        if !gb_copy_from_red {
            sym |= 1 << 6;
            if g_lo != last_g_lo { sym |= 1 << 2; }
            if g_hi != last_g_hi { sym |= 1 << 3; }
            if b_lo != last_b_lo { sym |= 1 << 4; }
            if b_hi != last_b_hi { sym |= 1 << 5; }
        }
        enc_rgb.encode_symbol(m_rgb_byte_used, sym).map_err(Error::Io)?;
        if (sym & (1 << 0)) != 0 {
            enc_rgb.encode_symbol(&mut m_rgb_diff[0], r_lo.wrapping_sub(last_r_lo) as u32).map_err(Error::Io)?;
        }
        if (sym & (1 << 1)) != 0 {
            enc_rgb.encode_symbol(&mut m_rgb_diff[1], r_hi.wrapping_sub(last_r_hi) as u32).map_err(Error::Io)?;
        }
        if (sym & (1 << 6)) != 0 {
            let diff_l = r_lo as i32 - last_r_lo as i32;
            if (sym & (1 << 2)) != 0 {
                let pred_g_lo = u8_clamp(diff_l + last_g_lo as i32) as u8;
                enc_rgb.encode_symbol(&mut m_rgb_diff[2], g_lo.wrapping_sub(pred_g_lo) as u32).map_err(Error::Io)?;
            }
            if (sym & (1 << 4)) != 0 {
                let diff_g_lo = g_lo as i32 - last_g_lo as i32;
                let diff_bl = (diff_l + diff_g_lo) / 2;
                let pred_b_lo = u8_clamp(diff_bl + last_b_lo as i32) as u8;
                enc_rgb.encode_symbol(&mut m_rgb_diff[4], b_lo.wrapping_sub(pred_b_lo) as u32).map_err(Error::Io)?;
            }
            let diff_h = r_hi as i32 - last_r_hi as i32;
            if (sym & (1 << 3)) != 0 {
                let pred_g_hi = u8_clamp(diff_h + last_g_hi as i32) as u8;
                enc_rgb.encode_symbol(&mut m_rgb_diff[3], g_hi.wrapping_sub(pred_g_hi) as u32).map_err(Error::Io)?;
            }
            if (sym & (1 << 5)) != 0 {
                let diff_g_hi = g_hi as i32 - last_g_hi as i32;
                let diff_bh = (diff_h + diff_g_hi) / 2;
                let pred_b_hi = u8_clamp(diff_bh + last_b_hi as i32) as u8;
                enc_rgb.encode_symbol(&mut m_rgb_diff[5], b_hi.wrapping_sub(pred_b_hi) as u32).map_err(Error::Io)?;
            }
        }
        return Ok(());
    }

    let mut sym = 0u32;
    if r_lo != last_r_lo {
        sym |= 1 << 0;
    }
    if r_hi != last_r_hi {
        sym |= 1 << 1;
    }

    let gb_copy_from_red = rgb[1] == rgb[0] && rgb[2] == rgb[0];
    if !gb_copy_from_red {
        sym |= 1 << 6;
        if g_lo != last_g_lo {
            sym |= 1 << 2;
        }
        if g_hi != last_g_hi {
            sym |= 1 << 3;
        }
        if b_lo != last_b_lo {
            sym |= 1 << 4;
        }
        if b_hi != last_b_hi {
            sym |= 1 << 5;
        }
    }

    enc_rgb
        .encode_symbol(m_rgb_byte_used_rgbnir, sym)
        .map_err(Error::Io)?;

    if (sym & (1 << 0)) != 0 {
        enc_rgb
            .encode_symbol(&mut m_rgb_diff[0], r_lo.wrapping_sub(last_r_lo) as u32)
            .map_err(Error::Io)?;
    }
    if (sym & (1 << 1)) != 0 {
        enc_rgb
            .encode_symbol(&mut m_rgb_diff[1], r_hi.wrapping_sub(last_r_hi) as u32)
            .map_err(Error::Io)?;
    }

    if (sym & (1 << 6)) != 0 {
        let diff_l = r_lo as i32 - last_r_lo as i32;

        if (sym & (1 << 2)) != 0 {
            let pred_g_lo = u8_clamp(diff_l + last_g_lo as i32) as u8;
            enc_rgb
                .encode_symbol(&mut m_rgb_diff[2], g_lo.wrapping_sub(pred_g_lo) as u32)
                .map_err(Error::Io)?;
        }

        if (sym & (1 << 4)) != 0 {
            let diff_g_lo = g_lo as i32 - last_g_lo as i32;
            let diff_bl = (diff_l + diff_g_lo) / 2;
            let pred_b_lo = u8_clamp(diff_bl + last_b_lo as i32) as u8;
            enc_rgb
                .encode_symbol(&mut m_rgb_diff[4], b_lo.wrapping_sub(pred_b_lo) as u32)
                .map_err(Error::Io)?;
        }

        let diff_h = r_hi as i32 - last_r_hi as i32;

        if (sym & (1 << 3)) != 0 {
            let pred_g_hi = u8_clamp(diff_h + last_g_hi as i32) as u8;
            enc_rgb
                .encode_symbol(&mut m_rgb_diff[3], g_hi.wrapping_sub(pred_g_hi) as u32)
                .map_err(Error::Io)?;
        }

        if (sym & (1 << 5)) != 0 {
            let diff_g_hi = g_hi as i32 - last_g_hi as i32;
            let diff_bh = (diff_h + diff_g_hi) / 2;
            let pred_b_hi = u8_clamp(diff_bh + last_b_hi as i32) as u8;
            enc_rgb
                .encode_symbol(&mut m_rgb_diff[5], b_hi.wrapping_sub(pred_b_hi) as u32)
                .map_err(Error::Io)?;
        }
    }

    Ok(())
}

/// Decode one RGB14 point from `dec_rgb` into `ctx.state.rgb` and update `ctx.last_rgb`.
/// Mirrors `LASreadItemCompressed_RGB14_v3::read()`.
fn read_rgb<R: Read>(
    ctx: &mut Point14ContinuationContext,
    dec_rgb: &mut ArithmeticDecoder<R>,
    rgbnir_mode: bool,
) -> Result<()> {
    let sym = if rgbnir_mode {
        dec_rgb.decode_symbol(&mut ctx.m_rgb_byte_used_rgbnir)? as u32
    } else {
        dec_rgb.decode_symbol(&mut ctx.m_rgb_byte_used)? as u32
    };

    let last = ctx.last_rgb; // [R, G, B] as u16

    if !rgbnir_mode {
        // RGB14 (PDRF7) uses the same 128-symbol bit-6 + predictive algorithm
        // as RGBNIR14 — matches LASreadItemCompressed_RGB14_v3::read()
        let mut item = [0u16; 3];

        // Red low byte (bit 0)
        if (sym & (1 << 0)) != 0 {
            let corr = dec_rgb.decode_symbol(&mut ctx.m_rgb_diff[0])? as i32;
            item[0] = u8_fold(corr + (last[0] & 0xFF) as i32) as u16;
        } else {
            item[0] = last[0] & 0xFF;
        }
        // Red high byte (bit 1)
        if (sym & (1 << 1)) != 0 {
            let corr = dec_rgb.decode_symbol(&mut ctx.m_rgb_diff[1])? as i32;
            item[0] |= (u8_fold(corr + (last[0] >> 8) as i32) as u16) << 8;
        } else {
            item[0] |= last[0] & 0xFF00;
        }

        // Bit 6: G/B predicted from R's diff
        if (sym & (1 << 6)) != 0 {
            let diff_l = (item[0] & 0x00FF) as i32 - (last[0] & 0xFF) as i32;

            if (sym & (1 << 2)) != 0 {
                let corr = dec_rgb.decode_symbol(&mut ctx.m_rgb_diff[2])? as i32;
                item[1] = u8_fold(corr + u8_clamp(diff_l + (last[1] & 0xFF) as i32)) as u16;
            } else {
                item[1] = last[1] & 0xFF;
            }
            if (sym & (1 << 4)) != 0 {
                let corr = dec_rgb.decode_symbol(&mut ctx.m_rgb_diff[4])? as i32;
                let diff_bl = (diff_l + ((item[1] & 0x00FF) as i32 - (last[1] & 0xFF) as i32)) / 2;
                item[2] = u8_fold(corr + u8_clamp(diff_bl + (last[2] & 0xFF) as i32)) as u16;
            } else {
                item[2] = last[2] & 0xFF;
            }

            let diff_h = (item[0] >> 8) as i32 - (last[0] >> 8) as i32;
            if (sym & (1 << 3)) != 0 {
                let corr = dec_rgb.decode_symbol(&mut ctx.m_rgb_diff[3])? as i32;
                item[1] |= (u8_fold(corr + u8_clamp(diff_h + (last[1] >> 8) as i32)) as u16) << 8;
            } else {
                item[1] |= last[1] & 0xFF00;
            }
            if (sym & (1 << 5)) != 0 {
                let corr = dec_rgb.decode_symbol(&mut ctx.m_rgb_diff[5])? as i32;
                let diff_bh = (diff_h + ((item[1] >> 8) as i32 - (last[1] >> 8) as i32)) / 2;
                item[2] |= (u8_fold(corr + u8_clamp(diff_bh + (last[2] >> 8) as i32)) as u16) << 8;
            } else {
                item[2] |= last[2] & 0xFF00;
            }
        } else {
            // G and B copy from R (greyscale-like, bit 6 = 0)
            item[1] = item[0];
            item[2] = item[0];
        }

        ctx.last_rgb = item;
        ctx.state.rgb = Some(Rgb16 {
            red: item[0],
            green: item[1],
            blue: item[2],
        });
        return Ok(());
    }

    let mut item = [0u16; 3];

    // Red low byte (bit 0)
    if (sym & (1 << 0)) != 0 {
        let corr = dec_rgb.decode_symbol(&mut ctx.m_rgb_diff[0])? as i32;
        item[0] = u8_fold(corr + (last[0] & 0xFF) as i32) as u16;
    } else {
        item[0] = last[0] & 0xFF;
    }

    // Red high byte (bit 1)
    if (sym & (1 << 1)) != 0 {
        let corr = dec_rgb.decode_symbol(&mut ctx.m_rgb_diff[1])? as i32;
        item[0] |= (u8_fold(corr + (last[0] >> 8) as i32) as u16) << 8;
    } else {
        item[0] |= last[0] & 0xFF00;
    }

    // Bit 6: whether green/blue are predicted from red's diff
    if (sym & (1 << 6)) != 0 {
        let diff_l = (item[0] & 0x00FF) as i32 - (last[0] & 0xFF) as i32;

        // Green low byte (bit 2)
        if (sym & (1 << 2)) != 0 {
            let corr = dec_rgb.decode_symbol(&mut ctx.m_rgb_diff[2])? as i32;
            item[1] = u8_fold(corr + u8_clamp(diff_l + (last[1] & 0xFF) as i32)) as u16;
        } else {
            item[1] = last[1] & 0xFF;
        }

        // Blue low byte (bit 4), predicted from average of R diff and G diff
        if (sym & (1 << 4)) != 0 {
            let corr = dec_rgb.decode_symbol(&mut ctx.m_rgb_diff[4])? as i32;
            let diff_bl = (diff_l + ((item[1] & 0x00FF) as i32 - (last[1] & 0xFF) as i32)) / 2;
            item[2] = u8_fold(corr + u8_clamp(diff_bl + (last[2] & 0xFF) as i32)) as u16;
        } else {
            item[2] = last[2] & 0xFF;
        }

        let diff_h = (item[0] >> 8) as i32 - (last[0] >> 8) as i32;

        // Green high byte (bit 3)
        if (sym & (1 << 3)) != 0 {
            let corr = dec_rgb.decode_symbol(&mut ctx.m_rgb_diff[3])? as i32;
            item[1] |= (u8_fold(corr + u8_clamp(diff_h + (last[1] >> 8) as i32)) as u16) << 8;
        } else {
            item[1] |= last[1] & 0xFF00;
        }

        // Blue high byte (bit 5), predicted from average of R diff and G diff
        if (sym & (1 << 5)) != 0 {
            let corr = dec_rgb.decode_symbol(&mut ctx.m_rgb_diff[5])? as i32;
            let diff_bh = (diff_h + ((item[1] >> 8) as i32 - (last[1] >> 8) as i32)) / 2;
            item[2] |= (u8_fold(corr + u8_clamp(diff_bh + (last[2] >> 8) as i32)) as u16) << 8;
        } else {
            item[2] |= last[2] & 0xFF00;
        }
    } else {
        // Green and blue copy from red (greyscale-like)
        item[1] = item[0];
        item[2] = item[0];
    }

    ctx.last_rgb = item;
    ctx.state.rgb = Some(Rgb16 {
        red: item[0],
        green: item[1],
        blue: item[2],
    });

    Ok(())
}

/// Decode one NIR14 point from `dec_nir` into `ctx.state.nir` and update `ctx.last_nir`.
/// Mirrors the NIR layer in `LASreadItemCompressed_RGBNIR14_v3::read()`.
fn read_nir<R: Read>(
    ctx: &mut Point14ContinuationContext,
    dec_nir: &mut ArithmeticDecoder<R>,
) -> Result<()> {
    let sym = dec_nir.decode_symbol(&mut ctx.m_nir_byte_used)? as u32;

    let mut nir = if (sym & (1 << 0)) != 0 {
        let corr = dec_nir.decode_symbol(&mut ctx.m_nir_diff[0])? as i32;
        u8_fold(corr + (ctx.last_nir & 0xFF) as i32) as u16
    } else {
        ctx.last_nir & 0x00FF
    };

    if (sym & (1 << 1)) != 0 {
        let corr = dec_nir.decode_symbol(&mut ctx.m_nir_diff[1])? as i32;
        nir |= (u8_fold(corr + (ctx.last_nir >> 8) as i32) as u16) << 8;
    } else {
        nir |= ctx.last_nir & 0xFF00;
    }

    ctx.last_nir = nir;
    ctx.state.nir = Some(nir);
    Ok(())
}

fn encode_gps_time_sequence(changed_gps_values: &[i64], seed_gps: i64) -> Result<Vec<u8>> {
    if changed_gps_values.is_empty() {
        return Ok(Vec::new());
    }

    let mut out = std::io::Cursor::new(Vec::<u8>::new());
    let mut enc = ArithmeticEncoder::new(&mut out);
    let mut m_gpstime_multi = ArithmeticSymbolModel::new(LASZIP_GPS_TIME_MULTI_TOTAL as u32);
    let mut m_gpstime_0diff = ArithmeticSymbolModel::new(5);
    let mut ic_gpstime = IntegerCompressor::new(32, 9, 8, 0);

    let mut last_gps = [seed_gps, 0, 0, 0];
    let mut last_gps_diff = [0i32; 4];
    let mut multi_extreme_counter = [0i32; 4];
    let mut gps_last = 0usize;
    let mut gps_next = 0usize;

    for &gps_bits in changed_gps_values {
        write_gps_time_value(
            &mut enc,
            &mut m_gpstime_multi,
            &mut m_gpstime_0diff,
            &mut ic_gpstime,
            &mut last_gps,
            &mut last_gps_diff,
            &mut multi_extreme_counter,
            &mut gps_last,
            &mut gps_next,
            gps_bits,
        )?;
    }

    let _ = enc.done().map_err(Error::Io)?;
    Ok(out.into_inner())
}

fn quantize_i32(value: f32) -> i32 {
    if value >= 0.0 {
        (value + 0.5) as i32
    } else {
        (value - 0.5) as i32
    }
}

#[allow(clippy::too_many_arguments)]
fn write_gps_time_value<W: Write>(
    enc: &mut ArithmeticEncoder<W>,
    m_gpstime_multi: &mut ArithmeticSymbolModel,
    m_gpstime_0diff: &mut ArithmeticSymbolModel,
    ic_gpstime: &mut IntegerCompressor,
    last_gps: &mut [i64; 4],
    last_gps_diff: &mut [i32; 4],
    multi_extreme_counter: &mut [i32; 4],
    gps_last: &mut usize,
    gps_next: &mut usize,
    gps_bits: i64,
) -> Result<()> {
    if last_gps_diff[*gps_last] == 0 {
        let curr_gps_diff_64 = gps_bits.wrapping_sub(last_gps[*gps_last]);
        if let Ok(curr_gps_diff) = i32::try_from(curr_gps_diff_64) {
            enc.encode_symbol(m_gpstime_0diff, 0).map_err(Error::Io)?;
            ic_gpstime
                .compress(enc, 0, curr_gps_diff, 0)
                .map_err(Error::Io)?;
            last_gps_diff[*gps_last] = curr_gps_diff;
            multi_extreme_counter[*gps_last] = 0;
        } else {
            for i in 1..4usize {
                let candidate_index = (*gps_last + i) & 3;
                let other_diff_64 = gps_bits.wrapping_sub(last_gps[candidate_index]);
                if i32::try_from(other_diff_64).is_ok() {
                    enc.encode_symbol(m_gpstime_0diff, (i + 1) as u32)
                        .map_err(Error::Io)?;
                    *gps_last = candidate_index;
                    write_gps_time_value(
                        enc,
                        m_gpstime_multi,
                        m_gpstime_0diff,
                        ic_gpstime,
                        last_gps,
                        last_gps_diff,
                        multi_extreme_counter,
                        gps_last,
                        gps_next,
                        gps_bits,
                    )?;
                    return Ok(());
                }
            }

            enc.encode_symbol(m_gpstime_0diff, 1).map_err(Error::Io)?;
            let pred_hi = (last_gps[*gps_last] >> 32) as i32;
            let hi = (gps_bits >> 32) as i32;
            ic_gpstime
                .compress(enc, pred_hi, hi, 8)
                .map_err(Error::Io)?;
            enc.write_bits(32, (gps_bits as u64 & 0xFFFF_FFFF) as u32)
                .map_err(Error::Io)?;
            *gps_next = (*gps_next + 1) & 3;
            *gps_last = *gps_next;
            last_gps_diff[*gps_last] = 0;
            multi_extreme_counter[*gps_last] = 0;
        }
        last_gps[*gps_last] = gps_bits;
        return Ok(());
    }

    let curr_gps_diff_64 = gps_bits.wrapping_sub(last_gps[*gps_last]);
    if let Ok(curr_gps_diff) = i32::try_from(curr_gps_diff_64) {
        let last_diff = last_gps_diff[*gps_last];
        let multi = quantize_i32((curr_gps_diff as f32) / (last_diff as f32));

        if multi == 1 {
            enc.encode_symbol(m_gpstime_multi, 1).map_err(Error::Io)?;
            ic_gpstime
                .compress(enc, last_diff, curr_gps_diff, 1)
                .map_err(Error::Io)?;
            multi_extreme_counter[*gps_last] = 0;
        } else if multi > 0 {
            if multi < LASZIP_GPS_TIME_MULTI {
                enc.encode_symbol(m_gpstime_multi, multi as u32)
                    .map_err(Error::Io)?;
                if multi < 10 {
                    ic_gpstime
                        .compress(enc, multi.wrapping_mul(last_diff), curr_gps_diff, 2)
                        .map_err(Error::Io)?;
                } else {
                    ic_gpstime
                        .compress(enc, multi.wrapping_mul(last_diff), curr_gps_diff, 3)
                        .map_err(Error::Io)?;
                }
            } else {
                enc.encode_symbol(m_gpstime_multi, LASZIP_GPS_TIME_MULTI as u32)
                    .map_err(Error::Io)?;
                ic_gpstime
                    .compress(
                        enc,
                        LASZIP_GPS_TIME_MULTI.wrapping_mul(last_diff),
                        curr_gps_diff,
                        4,
                    )
                    .map_err(Error::Io)?;
                multi_extreme_counter[*gps_last] += 1;
                if multi_extreme_counter[*gps_last] > 3 {
                    last_gps_diff[*gps_last] = curr_gps_diff;
                    multi_extreme_counter[*gps_last] = 0;
                }
            }
        } else if multi < 0 {
            if multi > LASZIP_GPS_TIME_MULTI_MINUS {
                enc.encode_symbol(m_gpstime_multi, (LASZIP_GPS_TIME_MULTI - multi) as u32)
                    .map_err(Error::Io)?;
                ic_gpstime
                    .compress(enc, multi.wrapping_mul(last_diff), curr_gps_diff, 5)
                    .map_err(Error::Io)?;
            } else {
                enc.encode_symbol(
                    m_gpstime_multi,
                    (LASZIP_GPS_TIME_MULTI - LASZIP_GPS_TIME_MULTI_MINUS) as u32,
                )
                .map_err(Error::Io)?;
                ic_gpstime
                    .compress(
                        enc,
                        LASZIP_GPS_TIME_MULTI_MINUS.wrapping_mul(last_diff),
                        curr_gps_diff,
                        6,
                    )
                    .map_err(Error::Io)?;
                multi_extreme_counter[*gps_last] += 1;
                if multi_extreme_counter[*gps_last] > 3 {
                    last_gps_diff[*gps_last] = curr_gps_diff;
                    multi_extreme_counter[*gps_last] = 0;
                }
            }
        } else {
            enc.encode_symbol(m_gpstime_multi, 0).map_err(Error::Io)?;
            ic_gpstime
                .compress(enc, 0, curr_gps_diff, 7)
                .map_err(Error::Io)?;
            multi_extreme_counter[*gps_last] += 1;
            if multi_extreme_counter[*gps_last] > 3 {
                last_gps_diff[*gps_last] = curr_gps_diff;
                multi_extreme_counter[*gps_last] = 0;
            }
        }
    } else {
        for i in 1..4usize {
            let candidate_index = (*gps_last + i) & 3;
            let other_diff_64 = gps_bits.wrapping_sub(last_gps[candidate_index]);
            if i32::try_from(other_diff_64).is_ok() {
                enc.encode_symbol(
                    m_gpstime_multi,
                    (LASZIP_GPS_TIME_MULTI_CODE_FULL + i as i32) as u32,
                )
                .map_err(Error::Io)?;
                *gps_last = candidate_index;
                write_gps_time_value(
                    enc,
                    m_gpstime_multi,
                    m_gpstime_0diff,
                    ic_gpstime,
                    last_gps,
                    last_gps_diff,
                    multi_extreme_counter,
                    gps_last,
                    gps_next,
                    gps_bits,
                )?;
                return Ok(());
            }
        }

        enc.encode_symbol(m_gpstime_multi, LASZIP_GPS_TIME_MULTI_CODE_FULL as u32)
            .map_err(Error::Io)?;
        let pred_hi = (last_gps[*gps_last] >> 32) as i32;
        let hi = (gps_bits >> 32) as i32;
        ic_gpstime
            .compress(enc, pred_hi, hi, 8)
            .map_err(Error::Io)?;
        enc.write_bits(32, (gps_bits as u64 & 0xFFFF_FFFF) as u32)
            .map_err(Error::Io)?;
        *gps_next = (*gps_next + 1) & 3;
        *gps_last = *gps_next;
        last_gps_diff[*gps_last] = 0;
        multi_extreme_counter[*gps_last] = 0;
    }

    last_gps[*gps_last] = gps_bits;
    Ok(())
}

fn read_gps_time<R: Read>(
    ctx: &mut Point14ContinuationContext,
    dec_gps_time: &mut ArithmeticDecoder<R>,
) -> Result<()> {
    if ctx.last_gps_diff[ctx.gps_last] == 0 {
        let multi = dec_gps_time.decode_symbol(&mut ctx.m_gpstime_0diff)? as i32;
        if multi == 0 {
            ctx.last_gps_diff[ctx.gps_last] = ctx.ic_gpstime.decompress(dec_gps_time, 0, 0)?;
            ctx.last_gps[ctx.gps_last] = ctx.last_gps[ctx.gps_last]
                .wrapping_add(i64::from(ctx.last_gps_diff[ctx.gps_last]));
            ctx.multi_extreme_counter[ctx.gps_last] = 0;
        } else if multi == 1 {
            ctx.gps_next = (ctx.gps_next + 1) & 3;
            let hi = ctx.ic_gpstime.decompress(dec_gps_time, (ctx.last_gps[ctx.gps_last] >> 32) as i32, 8)?;
            let lo = dec_gps_time.read_int()?;
            ctx.last_gps[ctx.gps_next] = ((hi as i64) << 32) | i64::from(lo);
            ctx.gps_last = ctx.gps_next;
            ctx.last_gps_diff[ctx.gps_last] = 0;
            ctx.multi_extreme_counter[ctx.gps_last] = 0;
        } else {
            ctx.gps_last = (ctx.gps_last + multi as usize - 1) & 3;
            read_gps_time(ctx, dec_gps_time)?;
        }
    } else {
        let mut multi = dec_gps_time.decode_symbol(&mut ctx.m_gpstime_multi)? as i32;
        if multi == 1 {
            ctx.last_gps[ctx.gps_last] = ctx.last_gps[ctx.gps_last].wrapping_add(i64::from(
                ctx.ic_gpstime
                    .decompress(dec_gps_time, ctx.last_gps_diff[ctx.gps_last], 1)?,
            ));
            ctx.multi_extreme_counter[ctx.gps_last] = 0;
        } else if multi < LASZIP_GPS_TIME_MULTI_CODE_FULL {
            let gps_time_diff: i32;
            if multi == 0 {
                gps_time_diff = ctx.ic_gpstime.decompress(dec_gps_time, 0, 7)?;
                ctx.multi_extreme_counter[ctx.gps_last] += 1;
                if ctx.multi_extreme_counter[ctx.gps_last] > 3 {
                    ctx.last_gps_diff[ctx.gps_last] = gps_time_diff;
                    ctx.multi_extreme_counter[ctx.gps_last] = 0;
                }
            } else if multi < LASZIP_GPS_TIME_MULTI {
                gps_time_diff = if multi < 10 {
                    ctx.ic_gpstime.decompress(
                        dec_gps_time,
                        multi.wrapping_mul(ctx.last_gps_diff[ctx.gps_last]),
                        2,
                    )?
                } else {
                    ctx.ic_gpstime.decompress(
                        dec_gps_time,
                        multi.wrapping_mul(ctx.last_gps_diff[ctx.gps_last]),
                        3,
                    )?
                };
            } else if multi == LASZIP_GPS_TIME_MULTI {
                gps_time_diff = ctx.ic_gpstime.decompress(
                    dec_gps_time,
                    LASZIP_GPS_TIME_MULTI.wrapping_mul(ctx.last_gps_diff[ctx.gps_last]),
                    4,
                )?;
                ctx.multi_extreme_counter[ctx.gps_last] += 1;
                if ctx.multi_extreme_counter[ctx.gps_last] > 3 {
                    ctx.last_gps_diff[ctx.gps_last] = gps_time_diff;
                    ctx.multi_extreme_counter[ctx.gps_last] = 0;
                }
            } else {
                multi = LASZIP_GPS_TIME_MULTI - multi;
                if multi > LASZIP_GPS_TIME_MULTI_MINUS {
                    gps_time_diff = ctx.ic_gpstime.decompress(
                        dec_gps_time,
                        multi.wrapping_mul(ctx.last_gps_diff[ctx.gps_last]),
                        5,
                    )?;
                } else {
                    gps_time_diff = ctx.ic_gpstime.decompress(
                        dec_gps_time,
                        LASZIP_GPS_TIME_MULTI_MINUS.wrapping_mul(ctx.last_gps_diff[ctx.gps_last]),
                        6,
                    )?;
                    ctx.multi_extreme_counter[ctx.gps_last] += 1;
                    if ctx.multi_extreme_counter[ctx.gps_last] > 3 {
                        ctx.last_gps_diff[ctx.gps_last] = gps_time_diff;
                        ctx.multi_extreme_counter[ctx.gps_last] = 0;
                    }
                }
            }
            ctx.last_gps[ctx.gps_last] =
                ctx.last_gps[ctx.gps_last].wrapping_add(i64::from(gps_time_diff));
        } else if multi == LASZIP_GPS_TIME_MULTI_CODE_FULL {
            ctx.gps_next = (ctx.gps_next + 1) & 3;
            let hi = ctx.ic_gpstime.decompress(dec_gps_time, (ctx.last_gps[ctx.gps_last] >> 32) as i32, 8)?;
            let lo = dec_gps_time.read_int()?;
            ctx.last_gps[ctx.gps_next] = ((hi as i64) << 32) | i64::from(lo);
            ctx.gps_last = ctx.gps_next;
            ctx.last_gps_diff[ctx.gps_last] = 0;
            ctx.multi_extreme_counter[ctx.gps_last] = 0;
        } else {
            ctx.gps_last = (ctx.gps_last + (multi - LASZIP_GPS_TIME_MULTI_CODE_FULL) as usize) & 3;
            read_gps_time(ctx, dec_gps_time)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{
        decode_standard_layered_chunk_point14_v3,
        encode_standard_layered_chunk_point14_v3_constant_attributes,
        encode_standard_layered_chunk_point14_v3_singleton,
    };
    use crate::las::header::PointDataFormat;
    use crate::laz::arithmetic_encoder::ArithmeticEncoder;
    use crate::laz::arithmetic_model::ArithmeticSymbolModel;
    use crate::laz::integer_codec::IntegerCompressor;
    use crate::laz::LaszipItemSpec;

    fn encode_symbol_stream(symbol_count: u32, symbols: &[u32]) -> Vec<u8> {
        let mut writer = Cursor::new(Vec::<u8>::new());
        {
            let mut encoder = ArithmeticEncoder::new(&mut writer);
            let mut model = ArithmeticSymbolModel::new(symbol_count);
            for &symbol in symbols {
                encoder
                    .encode_symbol(&mut model, symbol)
                    .expect("symbol stream should encode");
            }
            let _ = encoder.done().expect("symbol stream should finalize");
        }
        writer.into_inner()
    }

    fn encode_xy_zero_delta_stream() -> Vec<u8> {
        let mut writer = Cursor::new(Vec::<u8>::new());
        {
            let mut encoder = ArithmeticEncoder::new(&mut writer);
            let mut changed_values_model = ArithmeticSymbolModel::new(128);
            let mut dx = IntegerCompressor::new(32, 2, 8, 0);
            let mut dy = IntegerCompressor::new(32, 22, 8, 0);

            encoder
                .encode_symbol(&mut changed_values_model, 0)
                .expect("changed-values symbol should encode");
            dx.compress(&mut encoder, 0, 0, 1)
                .expect("x delta should encode");
            dy.compress(&mut encoder, 0, 0, 1)
                .expect("y delta should encode");

            let _ = encoder.done().expect("xy stream should finalize");
        }
        writer.into_inner()
    }

    #[test]
    fn decodes_seed_point14_single_point_chunk() {
        let mut chunk = Vec::new();
        chunk.extend_from_slice(&123i32.to_le_bytes());
        chunk.extend_from_slice(&456i32.to_le_bytes());
        chunk.extend_from_slice(&789i32.to_le_bytes());
        chunk.extend_from_slice(&200u16.to_le_bytes());
        chunk.push(0x21);
        chunk.push(0x40);
        chunk.push(2u8);
        chunk.push(9u8);
        chunk.extend_from_slice(&(-12i16).to_le_bytes());
        chunk.extend_from_slice(&77u16.to_le_bytes());
        chunk.extend_from_slice(&(1000.0f64).to_le_bytes());

        let items = vec![LaszipItemSpec {
            item_type: 10,
            item_size: 30,
            item_version: 3,
        }];

        let points = decode_standard_layered_chunk_point14_v3(
            &chunk,
            1,
            &items,
            PointDataFormat::Pdrf6,
            [0.01, 0.01, 0.01],
            [0.0, 0.0, 0.0],
        )
        .expect("seed point14 should decode");

        assert_eq!(points.len(), 1);
        assert_eq!(points[0].classification, 2);
        assert_eq!(points[0].point_source_id, 77);
        assert_eq!(points[0].return_number, 1);
        assert_eq!(points[0].number_of_returns, 2);
    }

    #[test]
    fn decodes_non_arithmetic_per_point_item_sets() {
        let mut chunk = Vec::new();
        for i in 0..2i32 {
            chunk.extend_from_slice(&(100 + i).to_le_bytes());
            chunk.extend_from_slice(&(200 + i).to_le_bytes());
            chunk.extend_from_slice(&(300 + i).to_le_bytes());
            chunk.extend_from_slice(&(400u16 + i as u16).to_le_bytes());
            chunk.push(0x21);
            chunk.push(0x40);
            chunk.push(2u8);
            chunk.push(9u8);
            chunk.extend_from_slice(&(-12i16).to_le_bytes());
            chunk.extend_from_slice(&(77u16 + i as u16).to_le_bytes());
            chunk.extend_from_slice(&(1000.0f64 + i as f64).to_le_bytes());
        }

        let items = vec![LaszipItemSpec {
            item_type: 10,
            item_size: 30,
            item_version: 3,
        }];

        let points = decode_standard_layered_chunk_point14_v3(
            &chunk,
            2,
            &items,
            PointDataFormat::Pdrf6,
            [0.01, 0.01, 0.01],
            [0.0, 0.0, 0.0],
        )
        .expect("per-point item sets should decode");

        assert_eq!(points.len(), 2);
        assert_eq!(points[1].point_source_id, 78);
        assert!((points[1].x - 1.01).abs() < 1e-9);
    }

    #[test]
    fn decodes_point14_with_wavepacket_item_as_extra_bytes() {
        let mut chunk = Vec::new();
        chunk.extend_from_slice(&123i32.to_le_bytes());
        chunk.extend_from_slice(&456i32.to_le_bytes());
        chunk.extend_from_slice(&789i32.to_le_bytes());
        chunk.extend_from_slice(&200u16.to_le_bytes());
        chunk.push(0x21);
        chunk.push(0x40);
        chunk.push(2u8);
        chunk.push(9u8);
        chunk.extend_from_slice(&(-12i16).to_le_bytes());
        chunk.extend_from_slice(&77u16.to_le_bytes());
        chunk.extend_from_slice(&(1000.0f64).to_le_bytes());
        chunk.extend_from_slice(&[1u8, 2u8, 3u8, 4u8]);

        let items = vec![
            LaszipItemSpec {
                item_type: 10,
                item_size: 30,
                item_version: 3,
            },
            LaszipItemSpec {
                item_type: 14,
                item_size: 4,
                item_version: 3,
            },
        ];

        let points = decode_standard_layered_chunk_point14_v3(
            &chunk,
            1,
            &items,
            PointDataFormat::Pdrf6,
            [0.01, 0.01, 0.01],
            [0.0, 0.0, 0.0],
        )
        .expect("point14 with wavepacket item should decode");

        assert_eq!(points.len(), 1);
        assert_eq!(points[0].extra_bytes.len, 4);
        assert_eq!(points[0].extra_bytes.data[0], 1);
        assert_eq!(points[0].extra_bytes.data[3], 4);
    }

    #[test]
    fn decodes_point14_when_rgb_item_precedes_point_item() {
        let mut chunk = Vec::new();
        chunk.extend_from_slice(&1000u16.to_le_bytes());
        chunk.extend_from_slice(&2000u16.to_le_bytes());
        chunk.extend_from_slice(&3000u16.to_le_bytes());
        chunk.extend_from_slice(&123i32.to_le_bytes());
        chunk.extend_from_slice(&456i32.to_le_bytes());
        chunk.extend_from_slice(&789i32.to_le_bytes());
        chunk.extend_from_slice(&200u16.to_le_bytes());
        chunk.push(0x21);
        chunk.push(0x40);
        chunk.push(2u8);
        chunk.push(9u8);
        chunk.extend_from_slice(&(-12i16).to_le_bytes());
        chunk.extend_from_slice(&77u16.to_le_bytes());
        chunk.extend_from_slice(&(1000.0f64).to_le_bytes());

        let items = vec![
            LaszipItemSpec {
                item_type: 11,
                item_size: 6,
                item_version: 3,
            },
            LaszipItemSpec {
                item_type: 10,
                item_size: 30,
                item_version: 3,
            },
        ];

        let points = decode_standard_layered_chunk_point14_v3(
            &chunk,
            1,
            &items,
            PointDataFormat::Pdrf7,
            [0.01, 0.01, 0.01],
            [0.0, 0.0, 0.0],
        )
        .expect("point14 should decode regardless of item order");

        assert_eq!(points.len(), 1);
        let color = points[0].color.expect("rgb should be present");
        assert_eq!(color.red, 1000);
        assert_eq!(color.green, 2000);
        assert_eq!(color.blue, 3000);
    }

    #[test]
    fn decodes_rgbnir14_layered_continuation_with_zero_deltas() {
        let mut chunk = Vec::new();

        chunk.extend_from_slice(&123i32.to_le_bytes());
        chunk.extend_from_slice(&456i32.to_le_bytes());
        chunk.extend_from_slice(&789i32.to_le_bytes());
        chunk.extend_from_slice(&200u16.to_le_bytes());
        chunk.push(0x11);
        chunk.push(0x00);
        chunk.push(2u8);
        chunk.push(9u8);
        chunk.extend_from_slice(&(-12i16).to_le_bytes());
        chunk.extend_from_slice(&77u16.to_le_bytes());
        chunk.extend_from_slice(&(1000.0f64).to_le_bytes());
        chunk.extend_from_slice(&1000u16.to_le_bytes());
        chunk.extend_from_slice(&2000u16.to_le_bytes());
        chunk.extend_from_slice(&3000u16.to_le_bytes());
        chunk.extend_from_slice(&4000u16.to_le_bytes());

        let xy_bytes = encode_xy_zero_delta_stream();
        let rgb_bytes = encode_symbol_stream(128, &[64]);
        let nir_bytes = encode_symbol_stream(4, &[0]);

        chunk.extend_from_slice(&1u32.to_le_bytes());
        chunk.extend_from_slice(&(xy_bytes.len() as u32).to_le_bytes());
        chunk.extend_from_slice(&0u32.to_le_bytes());
        chunk.extend_from_slice(&0u32.to_le_bytes());
        chunk.extend_from_slice(&0u32.to_le_bytes());
        chunk.extend_from_slice(&0u32.to_le_bytes());
        chunk.extend_from_slice(&0u32.to_le_bytes());
        chunk.extend_from_slice(&0u32.to_le_bytes());
        chunk.extend_from_slice(&0u32.to_le_bytes());
        chunk.extend_from_slice(&0u32.to_le_bytes());
        chunk.extend_from_slice(&(rgb_bytes.len() as u32).to_le_bytes());
        chunk.extend_from_slice(&(nir_bytes.len() as u32).to_le_bytes());
        chunk.extend_from_slice(&xy_bytes);
        chunk.extend_from_slice(&rgb_bytes);
        chunk.extend_from_slice(&nir_bytes);

        let items = vec![
            LaszipItemSpec {
                item_type: 10,
                item_size: 30,
                item_version: 3,
            },
            LaszipItemSpec {
                item_type: 12,
                item_size: 8,
                item_version: 3,
            },
        ];

        let points = decode_standard_layered_chunk_point14_v3(
            &chunk,
            2,
            &items,
            PointDataFormat::Pdrf8,
            [0.01, 0.01, 0.01],
            [0.0, 0.0, 0.0],
        )
        .expect("rgbnir14 layered chunk should decode");

        assert_eq!(points.len(), 2);
        assert_eq!(points[1].point_source_id, points[0].point_source_id);
        assert_eq!(points[1].classification, points[0].classification);
        assert_eq!(points[1].nir, points[0].nir);
        let color = points[1].color.expect("rgb should be present");
        let seed_color = points[0].color.expect("seed rgb should be present");
        assert_eq!(color.red, seed_color.red);
        assert_eq!(color.green, seed_color.green);
        assert_eq!(color.blue, seed_color.blue);
        assert!((points[1].x - points[0].x).abs() < 1e-9);
        assert!((points[1].y - points[0].y).abs() < 1e-9);
        assert!((points[1].z - points[0].z).abs() < 1e-9);
    }

    #[test]
    fn encodes_singleton_point14_seed_chunk_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let point = crate::point::PointRecord {
            x: 10.0,
            y: -20.0,
            z: 30.0,
            intensity: 512,
            return_number: 1,
            number_of_returns: 1,
            classification: 2,
            user_data: 7,
            scan_angle: -5,
            point_source_id: 11,
            gps_time: Some(crate::point::GpsTime(42.0)),
            ..crate::point::PointRecord::default()
        };

        let encoded = encode_standard_layered_chunk_point14_v3_singleton(
            &[point],
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("singleton point14 should encode");

        let items = vec![LaszipItemSpec {
            item_type: 10,
            item_size: 30,
            item_version: 3,
        }];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            1,
            &items,
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("singleton point14 should decode");

        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].classification, 2);
        assert_eq!(decoded[0].point_source_id, 11);
        assert!((decoded[0].x - 10.0).abs() < 1e-9);
    }

    #[test]
    fn encodes_multipoint_point14_constant_attributes_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let base = crate::point::PointRecord {
            intensity: 512,
            return_number: 1,
            number_of_returns: 1,
            classification: 2,
            user_data: 7,
            scan_angle: -5,
            point_source_id: 11,
            gps_time: Some(crate::point::GpsTime(42.0)),
            ..crate::point::PointRecord::default()
        };
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: -20.0,
            z: 30.0,
            ..base
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: -18.0,
            z: 31.0,
            ..base
        };
        let p2 = crate::point::PointRecord {
            x: 9.0,
            y: -21.0,
            z: 29.0,
            ..base
        };

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1, p2],
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("multipoint point14 subset should encode");

        let items = vec![LaszipItemSpec {
            item_type: 10,
            item_size: 30,
            item_version: 3,
        }];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            3,
            &items,
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("multipoint point14 subset should decode");

        assert_eq!(decoded.len(), 3);
        assert!((decoded[0].x - 10.0).abs() < 1e-9);
        assert!((decoded[1].x - 11.0).abs() < 1e-9);
        assert!((decoded[2].x - 9.0).abs() < 1e-9);
        assert!((decoded[1].z - 31.0).abs() < 1e-9);
        assert!((decoded[2].z - 29.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_multipoint_point14_when_non_xyz_attributes_change() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: -20.0,
            z: 30.0,
            intensity: 512,
            return_number: 1,
            number_of_returns: 1,
            classification: 2,
            ..crate::point::PointRecord::default()
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: -19.0,
            z: 30.0,
            intensity: 512,
            return_number: 1,
            number_of_returns: 1,
            classification: 2,
            color: Some(crate::point::Rgb16 {
                red: 1200,
                green: 2200,
                blue: 3200,
            }),
            flags: 0x00,
            ..crate::point::PointRecord::default()
        };

        let err = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1],
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect_err("attribute changes should be rejected by subset encoder");
        let msg = format!("{err}");
        assert!(
            msg.contains("point cannot be represented in requested Point14 format")
                || msg.contains("attributes must remain representable in the target point format")
        );
    }

    #[test]
    fn encodes_multipoint_point14_with_varying_rgb_pdrf7_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let base = crate::point::PointRecord {
            intensity: 100,
            classification: 2,
            user_data: 11,
            scan_angle: 3,
            point_source_id: 10,
            return_number: 1,
            number_of_returns: 1,
            flags: 0x00,
            ..crate::point::PointRecord::default()
        };
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            color: Some(crate::point::Rgb16 {
                red: 1000,
                green: 2000,
                blue: 3000,
            }),
            ..base
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: 1.0,
            z: 0.0,
            color: Some(crate::point::Rgb16 {
                red: 1200,
                green: 2400,
                blue: 3600,
            }),
            ..base
        };
        let p2 = crate::point::PointRecord {
            x: 12.0,
            y: 2.0,
            z: 1.0,
            color: Some(crate::point::Rgb16 {
                red: 1300,
                green: 2600,
                blue: 3900,
            }),
            ..base
        };

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1, p2],
            PointDataFormat::Pdrf7,
            scales,
            offsets,
        )
        .expect("multipoint rgb PDRF7 subset should encode");

        let items = vec![
            LaszipItemSpec {
                item_type: 10,
                item_size: 30,
                item_version: 3,
            },
            LaszipItemSpec {
                item_type: 11,
                item_size: 6,
                item_version: 3,
            },
        ];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            3,
            &items,
            PointDataFormat::Pdrf7,
            scales,
            offsets,
        )
        .expect("multipoint rgb PDRF7 subset should decode");

        assert_eq!(decoded[0].color.map(|c| c.red), Some(1000));
        assert_eq!(decoded[1].color.map(|c| c.red), Some(1200));
        assert_eq!(decoded[2].color.map(|c| c.red), Some(1300));
    }

    #[test]
    fn encodes_multipoint_point14_with_varying_scanner_channel_and_return_fields_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let base = crate::point::PointRecord {
            intensity: 100,
            classification: 2,
            user_data: 11,
            scan_angle: 3,
            point_source_id: 10,
            ..crate::point::PointRecord::default()
        };
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            return_number: 1,
            number_of_returns: 2,
            flags: 0x00,
            ..base
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: 1.0,
            z: 0.0,
            return_number: 2,
            number_of_returns: 2,
            flags: 0x10,
            ..base
        };
        let p2 = crate::point::PointRecord {
            x: 12.0,
            y: 2.0,
            z: 1.0,
            return_number: 1,
            number_of_returns: 3,
            flags: 0x20,
            ..base
        };

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1, p2],
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("scanner-channel+return-fields subset should encode");

        let items = vec![LaszipItemSpec {
            item_type: 10,
            item_size: 30,
            item_version: 3,
        }];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            3,
            &items,
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("scanner-channel+return-fields subset should decode");

        assert_eq!(decoded[0].return_number, 1);
        assert_eq!(decoded[0].number_of_returns, 2);
        assert_eq!(decoded[1].return_number, 2);
        assert_eq!(decoded[1].number_of_returns, 2);
        assert_eq!(decoded[2].return_number, 1);
        assert_eq!(decoded[2].number_of_returns, 3);
    }

    #[test]
    fn encodes_multipoint_point14_with_varying_scanner_channel_and_flags_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let base = crate::point::PointRecord {
            intensity: 100,
            classification: 2,
            user_data: 11,
            scan_angle: 3,
            point_source_id: 10,
            return_number: 1,
            number_of_returns: 1,
            ..crate::point::PointRecord::default()
        };
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            flags: 0x00,
            ..base
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: 1.0,
            z: 0.0,
            scan_direction_flag: true,
            flags: 0x10,
            ..base
        };
        let p2 = crate::point::PointRecord {
            x: 12.0,
            y: 2.0,
            z: 1.0,
            edge_of_flight_line: true,
            flags: 0x21,
            ..base
        };

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1, p2],
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("scanner-channel+flags subset should encode");

        let items = vec![LaszipItemSpec {
            item_type: 10,
            item_size: 30,
            item_version: 3,
        }];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            3,
            &items,
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("scanner-channel+flags subset should decode");

        assert!(!decoded[0].scan_direction_flag);
        assert!(decoded[1].scan_direction_flag);
        assert!(decoded[2].edge_of_flight_line);
        assert_eq!(decoded[2].flags & 0x0F, 0x01);
    }

    #[test]
    fn encodes_multipoint_point14_with_varying_intensity_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let base = crate::point::PointRecord {
            return_number: 1,
            number_of_returns: 1,
            classification: 2,
            ..crate::point::PointRecord::default()
        };
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            intensity: 100,
            ..base
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: 0.0,
            z: 0.0,
            intensity: 110,
            ..base
        };
        let p2 = crate::point::PointRecord {
            x: 12.0,
            y: 0.0,
            z: 0.0,
            intensity: 130,
            ..base
        };

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1, p2],
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("varying-intensity subset should encode");

        let items = vec![LaszipItemSpec {
            item_type: 10,
            item_size: 30,
            item_version: 3,
        }];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            3,
            &items,
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("varying-intensity subset should decode");

        assert_eq!(decoded[0].intensity, 100);
        assert_eq!(decoded[1].intensity, 110);
        assert_eq!(decoded[2].intensity, 130);
    }

    #[test]
    fn encodes_multipoint_point14_with_varying_classification_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let base = crate::point::PointRecord {
            intensity: 100,
            return_number: 1,
            number_of_returns: 1,
            ..crate::point::PointRecord::default()
        };
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            classification: 1,
            ..base
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: 0.0,
            z: 0.0,
            classification: 2,
            ..base
        };
        let p2 = crate::point::PointRecord {
            x: 12.0,
            y: 0.0,
            z: 0.0,
            classification: 5,
            ..base
        };

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1, p2],
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("varying-classification subset should encode");

        let items = vec![LaszipItemSpec {
            item_type: 10,
            item_size: 30,
            item_version: 3,
        }];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            3,
            &items,
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("varying-classification subset should decode");

        assert_eq!(decoded[0].classification, 1);
        assert_eq!(decoded[1].classification, 2);
        assert_eq!(decoded[2].classification, 5);
    }

    #[test]
    fn encodes_multipoint_point14_with_varying_return_fields_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let base = crate::point::PointRecord {
            intensity: 100,
            classification: 2,
            ..crate::point::PointRecord::default()
        };
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            return_number: 1,
            number_of_returns: 3,
            ..base
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: 0.0,
            z: 0.0,
            return_number: 2,
            number_of_returns: 3,
            ..base
        };
        let p2 = crate::point::PointRecord {
            x: 12.0,
            y: 0.0,
            z: 0.0,
            return_number: 3,
            number_of_returns: 3,
            ..base
        };

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1, p2],
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("varying-return-fields subset should encode");

        let items = vec![LaszipItemSpec {
            item_type: 10,
            item_size: 30,
            item_version: 3,
        }];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            3,
            &items,
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("varying-return-fields subset should decode");

        assert_eq!(decoded[0].return_number, 1);
        assert_eq!(decoded[0].number_of_returns, 3);
        assert_eq!(decoded[1].return_number, 2);
        assert_eq!(decoded[1].number_of_returns, 3);
        assert_eq!(decoded[2].return_number, 3);
        assert_eq!(decoded[2].number_of_returns, 3);
    }

    #[test]
    fn encodes_multipoint_point14_with_varying_rgb_nir_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let base = crate::point::PointRecord {
            intensity: 100,
            classification: 2,
            user_data: 11,
            scan_angle: 3,
            point_source_id: 10,
            return_number: 1,
            number_of_returns: 1,
            flags: 0x00,
            ..crate::point::PointRecord::default()
        };
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            color: Some(crate::point::Rgb16 {
                red: 1000,
                green: 2000,
                blue: 3000,
            }),
            nir: Some(100),
            ..base
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: 1.0,
            z: 0.0,
            color: Some(crate::point::Rgb16 {
                red: 1200,
                green: 2400,
                blue: 3600,
            }),
            nir: Some(180),
            ..base
        };
        let p2 = crate::point::PointRecord {
            x: 12.0,
            y: 2.0,
            z: 1.0,
            color: Some(crate::point::Rgb16 {
                red: 1300,
                green: 2600,
                blue: 3900,
            }),
            nir: Some(260),
            ..base
        };

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1, p2],
            PointDataFormat::Pdrf8,
            scales,
            offsets,
        )
        .expect("multipoint rgbnir subset should encode");

        let items = vec![
            LaszipItemSpec {
                item_type: 10,
                item_size: 30,
                item_version: 3,
            },
            LaszipItemSpec {
                item_type: 12,
                item_size: 8,
                item_version: 3,
            },
        ];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            3,
            &items,
            PointDataFormat::Pdrf8,
            scales,
            offsets,
        )
        .expect("multipoint rgbnir subset should decode");

        assert_eq!(decoded[0].color.map(|c| c.red), Some(1000));
        assert_eq!(decoded[1].color.map(|c| c.red), Some(1200));
        assert_eq!(decoded[2].color.map(|c| c.red), Some(1300));
        assert_eq!(decoded[0].nir, Some(100));
        assert_eq!(decoded[1].nir, Some(180));
        assert_eq!(decoded[2].nir, Some(260));
    }

    #[test]
    fn encodes_multipoint_point14_with_varying_user_data_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let base = crate::point::PointRecord {
            intensity: 100,
            return_number: 1,
            number_of_returns: 1,
            classification: 2,
            ..crate::point::PointRecord::default()
        };
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            user_data: 1,
            ..base
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: 0.0,
            z: 0.0,
            user_data: 5,
            ..base
        };
        let p2 = crate::point::PointRecord {
            x: 12.0,
            y: 0.0,
            z: 0.0,
            user_data: 9,
            ..base
        };

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1, p2],
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("varying-user-data subset should encode");

        let items = vec![LaszipItemSpec {
            item_type: 10,
            item_size: 30,
            item_version: 3,
        }];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            3,
            &items,
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("varying-user-data subset should decode");

        assert_eq!(decoded[0].user_data, 1);
        assert_eq!(decoded[1].user_data, 5);
        assert_eq!(decoded[2].user_data, 9);
    }

    #[test]
    fn encodes_multipoint_point14_with_varying_point_source_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let base = crate::point::PointRecord {
            intensity: 100,
            return_number: 1,
            number_of_returns: 1,
            classification: 2,
            ..crate::point::PointRecord::default()
        };
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            point_source_id: 10,
            ..base
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: 0.0,
            z: 0.0,
            point_source_id: 11,
            ..base
        };
        let p2 = crate::point::PointRecord {
            x: 12.0,
            y: 0.0,
            z: 0.0,
            point_source_id: 13,
            ..base
        };

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1, p2],
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("varying-point-source subset should encode");

        let items = vec![LaszipItemSpec {
            item_type: 10,
            item_size: 30,
            item_version: 3,
        }];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            3,
            &items,
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("varying-point-source subset should decode");

        assert_eq!(decoded[0].point_source_id, 10);
        assert_eq!(decoded[1].point_source_id, 11);
        assert_eq!(decoded[2].point_source_id, 13);
    }

    #[test]
    fn encodes_multipoint_point14_with_varying_scan_angle_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let base = crate::point::PointRecord {
            intensity: 100,
            return_number: 1,
            number_of_returns: 1,
            classification: 2,
            ..crate::point::PointRecord::default()
        };
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            scan_angle: -3,
            ..base
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: 0.0,
            z: 0.0,
            scan_angle: -1,
            ..base
        };
        let p2 = crate::point::PointRecord {
            x: 12.0,
            y: 0.0,
            z: 0.0,
            scan_angle: 2,
            ..base
        };

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1, p2],
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("varying-scan-angle subset should encode");

        let items = vec![LaszipItemSpec {
            item_type: 10,
            item_size: 30,
            item_version: 3,
        }];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            3,
            &items,
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("varying-scan-angle subset should decode");

        assert_eq!(decoded[0].scan_angle, -3);
        assert_eq!(decoded[1].scan_angle, -1);
        assert_eq!(decoded[2].scan_angle, 2);
    }

    #[test]
    fn encodes_multipoint_point14_with_varying_gps_time_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let base = crate::point::PointRecord {
            intensity: 100,
            return_number: 1,
            number_of_returns: 1,
            classification: 2,
            ..crate::point::PointRecord::default()
        };
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            gps_time: Some(crate::point::GpsTime(1000.0)),
            ..base
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: 0.0,
            z: 0.0,
            gps_time: Some(crate::point::GpsTime(1001.0)),
            ..base
        };
        let p2 = crate::point::PointRecord {
            x: 12.0,
            y: 0.0,
            z: 0.0,
            gps_time: Some(crate::point::GpsTime(1002.5)),
            ..base
        };

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1, p2],
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("varying-gps-time subset should encode");

        let items = vec![LaszipItemSpec {
            item_type: 10,
            item_size: 30,
            item_version: 3,
        }];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            3,
            &items,
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("varying-gps-time subset should decode");

        assert_eq!(decoded[0].gps_time.map(|t| t.0), Some(1000.0));
        assert_eq!(decoded[1].gps_time.map(|t| t.0), Some(1001.0));
        assert_eq!(decoded[2].gps_time.map(|t| t.0), Some(1002.5));
    }

    #[test]
    fn encodes_multipoint_point14_with_varying_scanner_channel_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let base = crate::point::PointRecord {
            intensity: 100,
            return_number: 1,
            number_of_returns: 1,
            classification: 2,
            ..crate::point::PointRecord::default()
        };
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            flags: 0x00,
            ..base
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: 1.0,
            z: 0.0,
            flags: 0x10,
            ..base
        };
        let p2 = crate::point::PointRecord {
            x: 12.0,
            y: 2.0,
            z: 1.0,
            flags: 0x20,
            ..base
        };

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1, p2],
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("varying-scanner-channel subset should encode");

        let items = vec![LaszipItemSpec {
            item_type: 10,
            item_size: 30,
            item_version: 3,
        }];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            3,
            &items,
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("varying-scanner-channel subset should decode");

        assert!((decoded[0].x - 10.0).abs() < 1e-9);
        assert!((decoded[1].x - 11.0).abs() < 1e-9);
        assert!((decoded[2].x - 12.0).abs() < 1e-9);
        assert!((decoded[0].y - 0.0).abs() < 1e-9);
        assert!((decoded[1].y - 1.0).abs() < 1e-9);
        assert!((decoded[2].y - 2.0).abs() < 1e-9);
        assert!((decoded[0].z - 0.0).abs() < 1e-9);
        assert!((decoded[1].z - 0.0).abs() < 1e-9);
        assert!((decoded[2].z - 1.0).abs() < 1e-9);
    }

    #[test]
    fn encodes_multipoint_point14_with_varying_scanner_channel_and_intensity_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let base = crate::point::PointRecord {
            return_number: 1,
            number_of_returns: 1,
            classification: 2,
            ..crate::point::PointRecord::default()
        };
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            intensity: 100,
            flags: 0x00,
            ..base
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: 1.0,
            z: 0.0,
            intensity: 110,
            flags: 0x10,
            ..base
        };
        let p2 = crate::point::PointRecord {
            x: 12.0,
            y: 2.0,
            z: 1.0,
            intensity: 90,
            flags: 0x20,
            ..base
        };

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1, p2],
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("scanner-channel+intensity subset should encode");

        let items = vec![LaszipItemSpec {
            item_type: 10,
            item_size: 30,
            item_version: 3,
        }];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            3,
            &items,
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("scanner-channel subset should decode");

        assert!((decoded[0].x - 10.0).abs() < 1e-9);
        assert!((decoded[1].x - 11.0).abs() < 1e-9);
        assert!((decoded[2].x - 12.0).abs() < 1e-9);
        assert!((decoded[0].y - 0.0).abs() < 1e-9);
        assert!((decoded[1].y - 1.0).abs() < 1e-9);
        assert!((decoded[2].y - 2.0).abs() < 1e-9);
        assert!((decoded[0].z - 0.0).abs() < 1e-9);
        assert!((decoded[1].z - 0.0).abs() < 1e-9);
        assert!((decoded[2].z - 1.0).abs() < 1e-9);
        assert_eq!(decoded[0].intensity, 100);
        assert_eq!(decoded[1].intensity, 110);
        assert_eq!(decoded[2].intensity, 90);
    }

    #[test]
    fn encodes_multipoint_point14_with_varying_scanner_channel_and_classification_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let base = crate::point::PointRecord {
            intensity: 100,
            return_number: 1,
            number_of_returns: 1,
            ..crate::point::PointRecord::default()
        };
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            classification: 2,
            flags: 0x00,
            ..base
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: 1.0,
            z: 0.0,
            classification: 4,
            flags: 0x10,
            ..base
        };
        let p2 = crate::point::PointRecord {
            x: 12.0,
            y: 2.0,
            z: 1.0,
            classification: 7,
            flags: 0x20,
            ..base
        };

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1, p2],
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("scanner-channel+classification subset should encode");

        let items = vec![LaszipItemSpec {
            item_type: 10,
            item_size: 30,
            item_version: 3,
        }];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            3,
            &items,
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("scanner-channel+classification subset should decode");

        assert_eq!(decoded[0].classification, 2);
        assert_eq!(decoded[1].classification, 4);
        assert_eq!(decoded[2].classification, 7);
    }

    #[test]
    fn encodes_multipoint_point14_with_varying_scanner_channel_and_user_data_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let base = crate::point::PointRecord {
            intensity: 100,
            classification: 2,
            return_number: 1,
            number_of_returns: 1,
            ..crate::point::PointRecord::default()
        };
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            user_data: 11,
            flags: 0x00,
            ..base
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: 1.0,
            z: 0.0,
            user_data: 21,
            flags: 0x10,
            ..base
        };
        let p2 = crate::point::PointRecord {
            x: 12.0,
            y: 2.0,
            z: 1.0,
            user_data: 31,
            flags: 0x20,
            ..base
        };

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1, p2],
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("scanner-channel+user-data subset should encode");

        let items = vec![LaszipItemSpec {
            item_type: 10,
            item_size: 30,
            item_version: 3,
        }];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            3,
            &items,
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("scanner-channel+user-data subset should decode");

        assert_eq!(decoded[0].user_data, 11);
        assert_eq!(decoded[1].user_data, 21);
        assert_eq!(decoded[2].user_data, 31);
    }

    #[test]
    fn encodes_multipoint_point14_with_varying_scanner_channel_and_scan_angle_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let base = crate::point::PointRecord {
            intensity: 100,
            classification: 2,
            user_data: 11,
            return_number: 1,
            number_of_returns: 1,
            ..crate::point::PointRecord::default()
        };
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            scan_angle: 3,
            flags: 0x00,
            ..base
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: 1.0,
            z: 0.0,
            scan_angle: 12,
            flags: 0x10,
            ..base
        };
        let p2 = crate::point::PointRecord {
            x: 12.0,
            y: 2.0,
            z: 1.0,
            scan_angle: -8,
            flags: 0x20,
            ..base
        };

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1, p2],
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("scanner-channel+scan-angle subset should encode");

        let items = vec![LaszipItemSpec {
            item_type: 10,
            item_size: 30,
            item_version: 3,
        }];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            3,
            &items,
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("scanner-channel+scan-angle subset should decode");

        assert_eq!(decoded[0].scan_angle, 3);
        assert_eq!(decoded[1].scan_angle, 12);
        assert_eq!(decoded[2].scan_angle, -8);
    }

    #[test]
    fn encodes_multipoint_point14_with_varying_scanner_channel_and_point_source_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let base = crate::point::PointRecord {
            intensity: 100,
            classification: 2,
            user_data: 11,
            scan_angle: 3,
            return_number: 1,
            number_of_returns: 1,
            ..crate::point::PointRecord::default()
        };
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            point_source_id: 10,
            flags: 0x00,
            ..base
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: 1.0,
            z: 0.0,
            point_source_id: 77,
            flags: 0x10,
            ..base
        };
        let p2 = crate::point::PointRecord {
            x: 12.0,
            y: 2.0,
            z: 1.0,
            point_source_id: 222,
            flags: 0x20,
            ..base
        };

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1, p2],
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("scanner-channel+point-source subset should encode");

        let items = vec![LaszipItemSpec {
            item_type: 10,
            item_size: 30,
            item_version: 3,
        }];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            3,
            &items,
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("scanner-channel+point-source subset should decode");

        assert_eq!(decoded[0].point_source_id, 10);
        assert_eq!(decoded[1].point_source_id, 77);
        assert_eq!(decoded[2].point_source_id, 222);
    }

    #[test]
    fn encodes_multipoint_point14_with_varying_scanner_channel_and_gps_time_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let base = crate::point::PointRecord {
            intensity: 100,
            classification: 2,
            user_data: 11,
            scan_angle: 3,
            point_source_id: 10,
            return_number: 1,
            number_of_returns: 1,
            ..crate::point::PointRecord::default()
        };
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            gps_time: Some(crate::point::GpsTime(1000.0)),
            flags: 0x00,
            ..base
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: 1.0,
            z: 0.0,
            gps_time: Some(crate::point::GpsTime(1001.0)),
            flags: 0x10,
            ..base
        };
        let p2 = crate::point::PointRecord {
            x: 12.0,
            y: 2.0,
            z: 1.0,
            gps_time: Some(crate::point::GpsTime(1002.5)),
            flags: 0x20,
            ..base
        };

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1, p2],
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("scanner-channel+gps-time subset should encode");

        let items = vec![LaszipItemSpec {
            item_type: 10,
            item_size: 30,
            item_version: 3,
        }];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            3,
            &items,
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("scanner-channel+gps-time subset should decode");

        assert_eq!(decoded[0].gps_time.map(|t| t.0), Some(1000.0));
        assert_eq!(decoded[1].gps_time.map(|t| t.0), Some(1001.0));
        assert_eq!(decoded[2].gps_time.map(|t| t.0), Some(1002.5));
    }

    #[test]
    fn encodes_multipoint_point14_with_varying_scanner_channel_and_nir_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let base = crate::point::PointRecord {
            intensity: 100,
            classification: 2,
            user_data: 11,
            scan_angle: 3,
            point_source_id: 10,
            color: Some(crate::point::Rgb16 {
                red: 1000,
                green: 2000,
                blue: 3000,
            }),
            return_number: 1,
            number_of_returns: 1,
            ..crate::point::PointRecord::default()
        };
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            nir: Some(100),
            flags: 0x00,
            ..base
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: 1.0,
            z: 0.0,
            nir: Some(140),
            flags: 0x10,
            ..base
        };
        let p2 = crate::point::PointRecord {
            x: 12.0,
            y: 2.0,
            z: 1.0,
            nir: Some(500),
            flags: 0x20,
            ..base
        };

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1, p2],
            PointDataFormat::Pdrf8,
            scales,
            offsets,
        )
        .expect("scanner-channel+nir subset should encode");

        let items = vec![
            LaszipItemSpec {
                item_type: 10,
                item_size: 30,
                item_version: 3,
            },
            LaszipItemSpec {
                item_type: 12,
                item_size: 8,
                item_version: 3,
            },
        ];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            3,
            &items,
            PointDataFormat::Pdrf8,
            scales,
            offsets,
        )
        .expect("scanner-channel+nir subset should decode");

        assert_eq!(decoded[0].nir, Some(100));
        assert_eq!(decoded[1].nir, Some(140));
        assert_eq!(decoded[2].nir, Some(500));
    }

    #[test]
    fn encodes_multipoint_point14_with_varying_scanner_channel_and_rgb_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let base = crate::point::PointRecord {
            intensity: 100,
            classification: 2,
            user_data: 11,
            scan_angle: 3,
            point_source_id: 10,
            return_number: 1,
            number_of_returns: 1,
            ..crate::point::PointRecord::default()
        };
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            color: Some(crate::point::Rgb16 {
                red: 1000,
                green: 2000,
                blue: 3000,
            }),
            nir: Some(100),
            flags: 0x00,
            ..base
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: 1.0,
            z: 0.0,
            color: Some(crate::point::Rgb16 {
                red: 1111,
                green: 2333,
                blue: 3555,
            }),
            nir: Some(100),
            flags: 0x10,
            ..base
        };
        let p2 = crate::point::PointRecord {
            x: 12.0,
            y: 2.0,
            z: 1.0,
            color: Some(crate::point::Rgb16 {
                red: 1222,
                green: 2666,
                blue: 3999,
            }),
            nir: Some(100),
            flags: 0x20,
            ..base
        };

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1, p2],
            PointDataFormat::Pdrf8,
            scales,
            offsets,
        )
        .expect("scanner-channel+rgb subset should encode");

        let items = vec![
            LaszipItemSpec {
                item_type: 10,
                item_size: 30,
                item_version: 3,
            },
            LaszipItemSpec {
                item_type: 12,
                item_size: 8,
                item_version: 3,
            },
        ];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            3,
            &items,
            PointDataFormat::Pdrf8,
            scales,
            offsets,
        )
        .expect("scanner-channel+rgb subset should decode");

        assert_eq!(decoded[0].color.map(|c| c.red), Some(1000));
        assert_eq!(decoded[1].color.map(|c| c.red), Some(1111));
        assert_eq!(decoded[2].color.map(|c| c.red), Some(1222));
    }

    #[test]
    fn encodes_multipoint_point14_with_varying_scanner_channel_and_rgb_pdrf7_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let base = crate::point::PointRecord {
            intensity: 100,
            classification: 2,
            user_data: 11,
            scan_angle: 3,
            point_source_id: 10,
            return_number: 1,
            number_of_returns: 1,
            ..crate::point::PointRecord::default()
        };
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            color: Some(crate::point::Rgb16 {
                red: 1000,
                green: 2000,
                blue: 3000,
            }),
            flags: 0x00,
            ..base
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: 1.0,
            z: 0.0,
            color: Some(crate::point::Rgb16 {
                red: 1200,
                green: 2200,
                blue: 3200,
            }),
            flags: 0x10,
            ..base
        };

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1],
            PointDataFormat::Pdrf7,
            scales,
            offsets,
        )
        .expect("scanner-channel+rgb PDRF7 subset should encode");

        let items = vec![
            LaszipItemSpec {
                item_type: 10,
                item_size: 30,
                item_version: 3,
            },
            LaszipItemSpec {
                item_type: 11,
                item_size: 6,
                item_version: 3,
            },
        ];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            2,
            &items,
            PointDataFormat::Pdrf7,
            scales,
            offsets,
        )
        .expect("scanner-channel+rgb PDRF7 subset should decode");

        assert_eq!(decoded[0].color.map(|c| c.red), Some(1000));
        assert_eq!(decoded[1].color.map(|c| c.red), Some(1200));
    }

    #[test]
    fn encodes_multipoint_point14_with_varying_flags_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let base = crate::point::PointRecord {
            intensity: 100,
            return_number: 1,
            number_of_returns: 1,
            classification: 2,
            ..crate::point::PointRecord::default()
        };
        let p0 = crate::point::PointRecord {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            flags: 0x01,
            scan_direction_flag: false,
            edge_of_flight_line: false,
            ..base
        };
        let p1 = crate::point::PointRecord {
            x: 11.0,
            y: 0.0,
            z: 0.0,
            flags: 0x03,
            scan_direction_flag: true,
            edge_of_flight_line: false,
            ..base
        };
        let p2 = crate::point::PointRecord {
            x: 12.0,
            y: 0.0,
            z: 0.0,
            flags: 0x0A,
            scan_direction_flag: true,
            edge_of_flight_line: true,
            ..base
        };

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1, p2],
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("varying-flags subset should encode");

        let items = vec![LaszipItemSpec {
            item_type: 10,
            item_size: 30,
            item_version: 3,
        }];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            3,
            &items,
            PointDataFormat::Pdrf6,
            scales,
            offsets,
        )
        .expect("varying-flags subset should decode");

        assert_eq!(decoded[0].flags & 0x0F, 0x01);
        assert!(!decoded[0].scan_direction_flag);
        assert!(!decoded[0].edge_of_flight_line);

        assert_eq!(decoded[1].flags & 0x0F, 0x03);
        assert!(decoded[1].scan_direction_flag);
        assert!(!decoded[1].edge_of_flight_line);

        assert_eq!(decoded[2].flags & 0x0F, 0x0A);
        assert!(decoded[2].scan_direction_flag);
        assert!(decoded[2].edge_of_flight_line);
    }

    #[test]
    fn encodes_multipoint_point14_with_varying_extra_bytes_roundtrip() {
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let mut p0 = crate::point::PointRecord {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            intensity: 100,
            classification: 2,
            return_number: 1,
            number_of_returns: 1,
            flags: 0x00,
            color: Some(crate::point::Rgb16 {
                red: 1000,
                green: 2000,
                blue: 3000,
            }),
            nir: Some(400),
            ..crate::point::PointRecord::default()
        };
        p0.extra_bytes.data[0] = 1;
        p0.extra_bytes.data[1] = 5;
        p0.extra_bytes.len = 2;

        let mut p1 = p0;
        p1.x = 11.0;
        p1.flags = 0x10;
        p1.extra_bytes.data[0] = 4;
        p1.extra_bytes.data[1] = 5;

        let mut p2 = p0;
        p2.x = 12.0;
        p2.flags = 0x20;
        p2.extra_bytes.data[0] = 4;
        p2.extra_bytes.data[1] = 9;

        let encoded = encode_standard_layered_chunk_point14_v3_constant_attributes(
            &[p0, p1, p2],
            PointDataFormat::Pdrf8,
            scales,
            offsets,
        )
        .expect("varying BYTE14 subset should encode");

        let items = vec![
            LaszipItemSpec {
                item_type: 10,
                item_size: 30,
                item_version: 3,
            },
            LaszipItemSpec {
                item_type: 12,
                item_size: 8,
                item_version: 3,
            },
            LaszipItemSpec {
                item_type: 14,
                item_size: 2,
                item_version: 3,
            },
        ];

        let decoded = decode_standard_layered_chunk_point14_v3(
            &encoded,
            3,
            &items,
            PointDataFormat::Pdrf8,
            scales,
            offsets,
        )
        .expect("varying BYTE14 subset should decode");

        assert_eq!(decoded[0].extra_bytes.len, 2);
        assert_eq!(decoded[1].extra_bytes.len, 2);
        assert_eq!(decoded[2].extra_bytes.len, 2);
        assert_eq!(decoded[0].extra_bytes.data[0], 1);
        assert_eq!(decoded[0].extra_bytes.data[1], 5);
        assert_eq!(decoded[1].extra_bytes.data[0], 4);
        assert_eq!(decoded[1].extra_bytes.data[1], 5);
        assert_eq!(decoded[2].extra_bytes.data[0], 4);
        assert_eq!(decoded[2].extra_bytes.data[1], 9);
    }
}