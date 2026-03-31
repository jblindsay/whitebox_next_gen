//! Standard LASzip pointwise Point10-family (v2) encoding.

use std::io::Write;

use crate::las::header::PointDataFormat;
use crate::laz::arithmetic_encoder::ArithmeticEncoder;
use crate::laz::arithmetic_model::ArithmeticSymbolModel;
use crate::laz::integer_codec::IntegerCompressor;
use crate::point::{PointRecord, Rgb16};
use crate::{Error, Result};

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

const LASZIP_GPS_TIME_MULTI: i32 = 500;
const LASZIP_GPS_TIME_MULTI_MINUS: i32 = -10;
const LASZIP_GPS_TIME_MULTI_UNCHANGED: i32 =
    LASZIP_GPS_TIME_MULTI - LASZIP_GPS_TIME_MULTI_MINUS + 1;
const LASZIP_GPS_TIME_MULTI_CODE_FULL: i32 =
    LASZIP_GPS_TIME_MULTI - LASZIP_GPS_TIME_MULTI_MINUS + 2;

const NUMBER_RETURN_MAP: [[u8; 8]; 8] = [
    [15, 14, 13, 12, 11, 10, 9, 8],
    [14, 0, 1, 3, 6, 10, 10, 9],
    [13, 1, 2, 4, 7, 11, 11, 10],
    [12, 3, 4, 5, 8, 12, 12, 11],
    [11, 6, 7, 8, 9, 13, 13, 12],
    [10, 10, 11, 12, 13, 14, 14, 13],
    [9, 10, 11, 12, 13, 14, 15, 14],
    [8, 9, 10, 11, 12, 13, 14, 15],
];

const NUMBER_RETURN_LEVEL: [[u8; 8]; 8] = [
    [0, 1, 2, 3, 4, 5, 6, 7],
    [1, 0, 1, 2, 3, 4, 5, 6],
    [2, 1, 0, 1, 2, 3, 4, 5],
    [3, 2, 1, 0, 1, 2, 3, 4],
    [4, 3, 2, 1, 0, 1, 2, 3],
    [5, 4, 3, 2, 1, 0, 1, 2],
    [6, 5, 4, 3, 2, 1, 0, 1],
    [7, 6, 5, 4, 3, 2, 1, 0],
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

#[derive(Clone, Copy, Default)]
struct RawPoint10 {
    x: i32,
    y: i32,
    z: i32,
    intensity: u16,
    bit_fields: u8,
    classification: u8,
    scan_angle_rank: i8,
    user_data: u8,
    point_source_id: u16,
}

impl RawPoint10 {
    fn from_point_record(
        point: &PointRecord,
        point_data_format: PointDataFormat,
        scales: [f64; 3],
        offsets: [f64; 3],
    ) -> Result<Self> {
        if point_data_format.has_gps_time() && point.gps_time.is_none() {
            return Err(crate::Error::InvalidValue {
                field: "laz.standard_point10_writer.gps_time",
                detail: format!(
                    "point data format {:?} requires gps_time, but point is missing it",
                    point_data_format
                ),
            });
        }
        if point_data_format.has_rgb() && point.color.is_none() {
            return Err(crate::Error::InvalidValue {
                field: "laz.standard_point10_writer.rgb",
                detail: format!(
                    "point data format {:?} requires RGB, but point is missing color",
                    point_data_format
                ),
            });
        }

        Ok(Self {
            x: ((point.x - offsets[0]) / scales[0]).round() as i32,
            y: ((point.y - offsets[1]) / scales[1]).round() as i32,
            z: ((point.z - offsets[2]) / scales[2]).round() as i32,
            intensity: point.intensity,
            bit_fields: (point.return_number & 0x07)
                | ((point.number_of_returns & 0x07) << 3)
                | (u8::from(point.scan_direction_flag) << 6)
                | (u8::from(point.edge_of_flight_line) << 7),
            classification: (point.classification & 0x1F) | ((point.flags & 0x07) << 5),
            scan_angle_rank: point.scan_angle as i8,
            user_data: point.user_data,
            point_source_id: point.point_source_id,
        })
    }

    fn to_20_bytes(self) -> [u8; 20] {
        let mut out = [0u8; 20];
        out[0..4].copy_from_slice(&self.x.to_le_bytes());
        out[4..8].copy_from_slice(&self.y.to_le_bytes());
        out[8..12].copy_from_slice(&self.z.to_le_bytes());
        out[12..14].copy_from_slice(&self.intensity.to_le_bytes());
        out[14] = self.bit_fields;
        out[15] = self.classification;
        out[16] = self.scan_angle_rank as u8;
        out[17] = self.user_data;
        out[18..20].copy_from_slice(&self.point_source_id.to_le_bytes());
        out
    }

    fn return_number(&self) -> u8 {
        self.bit_fields & 0x07
    }

    fn number_of_returns(&self) -> u8 {
        (self.bit_fields >> 3) & 0x07
    }

    fn scan_direction_flag(&self) -> bool {
        (self.bit_fields & 0x40) != 0
    }
}

struct Point10Common {
    last_intensity: [u16; 16],
    last_x_diff_median: [StreamingMedianI32; 16],
    last_y_diff_median: [StreamingMedianI32; 16],
    last_height: [i32; 8],
    changed_values: ArithmeticSymbolModel,
    scan_angle_rank: [ArithmeticSymbolModel; 2],
    bit_byte: Vec<ArithmeticSymbolModel>,
    classification: Vec<ArithmeticSymbolModel>,
    user_data: Vec<ArithmeticSymbolModel>,
}

impl Point10Common {
    fn new() -> Self {
        Self {
            last_intensity: [0; 16],
            last_x_diff_median: [StreamingMedianI32::new(); 16],
            last_y_diff_median: [StreamingMedianI32::new(); 16],
            last_height: [0; 8],
            changed_values: ArithmeticSymbolModel::new(64),
            scan_angle_rank: [ArithmeticSymbolModel::new(256), ArithmeticSymbolModel::new(256)],
            bit_byte: (0..256).map(|_| ArithmeticSymbolModel::new(256)).collect(),
            classification: (0..256).map(|_| ArithmeticSymbolModel::new(256)).collect(),
            user_data: (0..256).map(|_| ArithmeticSymbolModel::new(256)).collect(),
        }
    }
}

struct Point10V2Compressor {
    last_point: RawPoint10,
    ic_intensity: IntegerCompressor,
    ic_point_source_id: IntegerCompressor,
    ic_dx: IntegerCompressor,
    ic_dy: IntegerCompressor,
    ic_z: IntegerCompressor,
    common: Point10Common,
}

impl Point10V2Compressor {
    fn new(first_point: RawPoint10) -> Self {
        Self {
            last_point: first_point,
            ic_intensity: IntegerCompressor::new(16, 4, 8, 0),
            ic_point_source_id: IntegerCompressor::new(16, 1, 8, 0),
            ic_dx: IntegerCompressor::new(32, 2, 8, 0),
            ic_dy: IntegerCompressor::new(32, 22, 8, 0),
            ic_z: IntegerCompressor::new(32, 20, 8, 0),
            common: Point10Common::new(),
        }
    }

    fn compress_with<W: Write>(
        &mut self,
        enc: &mut ArithmeticEncoder<W>,
        current: RawPoint10,
    ) -> Result<()> {
        let r = current.return_number();
        let n = current.number_of_returns();
        let m = NUMBER_RETURN_MAP[n as usize][r as usize];
        let l = NUMBER_RETURN_LEVEL[n as usize][r as usize];

        let changed_values = (((self.last_point.bit_fields != current.bit_fields) as u32) << 5)
            | (((self.common.last_intensity[m as usize] != current.intensity) as u32) << 4)
            | (((self.last_point.classification != current.classification) as u32) << 3)
            | (((self.last_point.scan_angle_rank != current.scan_angle_rank) as u32) << 2)
            | (((self.last_point.user_data != current.user_data) as u32) << 1)
            | ((self.last_point.point_source_id != current.point_source_id) as u32);

        enc.encode_symbol(&mut self.common.changed_values, changed_values)
            .map_err(Error::Io)?;

        if (changed_values & (1 << 5)) != 0 {
            enc.encode_symbol(
                &mut self.common.bit_byte[self.last_point.bit_fields as usize],
                current.bit_fields as u32,
            )
            .map_err(Error::Io)?;
        }

        if (changed_values & (1 << 4)) != 0 {
            self.ic_intensity
                .compress(
                    enc,
                    self.common.last_intensity[m as usize] as i32,
                    current.intensity as i32,
                    if m < 3 { m as u32 } else { 3 },
                )
                .map_err(Error::Io)?;
            self.common.last_intensity[m as usize] = current.intensity;
        }

        if (changed_values & (1 << 3)) != 0 {
            enc.encode_symbol(
                &mut self.common.classification[self.last_point.classification as usize],
                current.classification as u32,
            )
            .map_err(Error::Io)?;
        }

        if (changed_values & (1 << 2)) != 0 {
            enc.encode_symbol(
                &mut self.common.scan_angle_rank[current.scan_direction_flag() as usize],
                u8_fold(current.scan_angle_rank as i32 - self.last_point.scan_angle_rank as i32)
                    as u32,
            )
            .map_err(Error::Io)?;
        }

        if (changed_values & (1 << 1)) != 0 {
            enc.encode_symbol(
                &mut self.common.user_data[self.last_point.user_data as usize],
                current.user_data as u32,
            )
            .map_err(Error::Io)?;
        }

        if (changed_values & 1) != 0 {
            self.ic_point_source_id
                .compress(
                    enc,
                    self.last_point.point_source_id as i32,
                    current.point_source_id as i32,
                    0,
                )
                .map_err(Error::Io)?;
        }

        let median_x = self.common.last_x_diff_median[m as usize].get();
        let diff_x = current.x.wrapping_sub(self.last_point.x);
        self.ic_dx
            .compress(enc, median_x, diff_x, (n == 1) as u32)
            .map_err(Error::Io)?;
        self.common.last_x_diff_median[m as usize].add(diff_x);

        let k_bits = self.ic_dx.k();
        let median_y = self.common.last_y_diff_median[m as usize].get();
        let diff_y = current.y.wrapping_sub(self.last_point.y);
        self.ic_dy
            .compress(
                enc,
                median_y,
                diff_y,
                (n == 1) as u32 + if k_bits < 20 { u32_zero_bit(k_bits) } else { 20 },
            )
            .map_err(Error::Io)?;
        self.common.last_y_diff_median[m as usize].add(diff_y);

        let k_bits = (self.ic_dx.k() + self.ic_dy.k()) / 2;
        self.ic_z
            .compress(
                enc,
                self.common.last_height[l as usize],
                current.z,
                (n == 1) as u32 + if k_bits < 18 { u32_zero_bit(k_bits) } else { 18 },
            )
            .map_err(Error::Io)?;
        self.common.last_height[l as usize] = current.z;

        self.last_point = current;
        Ok(())
    }
}

struct GpsV2Common {
    gps_time_multi: ArithmeticSymbolModel,
    gps_time_0_diff: ArithmeticSymbolModel,
    last: usize,
    next: usize,
    last_gps_times: [i64; 4],
    last_gps_time_diffs: [i32; 4],
    multi_extreme_counters: [i32; 4],
}

impl GpsV2Common {
    fn new(first_gps_bits: i64) -> Self {
        let mut out = Self {
            gps_time_multi: ArithmeticSymbolModel::new(517),
            gps_time_0_diff: ArithmeticSymbolModel::new(6),
            last: 0,
            next: 0,
            last_gps_times: [0; 4],
            last_gps_time_diffs: [0; 4],
            multi_extreme_counters: [0; 4],
        };
        out.last_gps_times[0] = first_gps_bits;
        out
    }
}

struct GpsTimeV2Compressor {
    common: GpsV2Common,
    ic_gps_time: IntegerCompressor,
}

impl GpsTimeV2Compressor {
    fn new(first_gps_bits: i64) -> Self {
        Self {
            common: GpsV2Common::new(first_gps_bits),
            ic_gps_time: IntegerCompressor::new(32, 9, 8, 0),
        }
    }

    fn compress_with<W: Write>(
        &mut self,
        enc: &mut ArithmeticEncoder<W>,
        gps_bits: i64,
    ) -> Result<()> {
        if self.common.last_gps_time_diffs[self.common.last] == 0 {
            if gps_bits == self.common.last_gps_times[self.common.last] {
                enc.encode_symbol(&mut self.common.gps_time_0_diff, 0)
                    .map_err(Error::Io)?;
                return Ok(());
            }

            let curr_gps_diff_64 = gps_bits.wrapping_sub(self.common.last_gps_times[self.common.last]);
            let curr_gps_diff = curr_gps_diff_64 as i32;
            if curr_gps_diff_64 == i64::from(curr_gps_diff) {
                enc.encode_symbol(&mut self.common.gps_time_0_diff, 1)
                    .map_err(Error::Io)?;
                self.ic_gps_time
                    .compress(enc, 0, curr_gps_diff, 0)
                    .map_err(Error::Io)?;
                self.common.last_gps_time_diffs[self.common.last] = curr_gps_diff;
                self.common.multi_extreme_counters[self.common.last] = 0;
                self.common.last_gps_times[self.common.last] = gps_bits;
                return Ok(());
            }

            for i in 1..4usize {
                let candidate_index = (self.common.last + i) & 3;
                let other_gps_diff_64 = gps_bits.wrapping_sub(self.common.last_gps_times[candidate_index]);
                let other_gps_diff = other_gps_diff_64 as i32;
                if other_gps_diff_64 == i64::from(other_gps_diff) {
                    enc.encode_symbol(&mut self.common.gps_time_0_diff, (i + 2) as u32)
                        .map_err(Error::Io)?;
                    self.common.last = candidate_index;
                    return self.compress_with(enc, gps_bits);
                }
            }

            enc.encode_symbol(&mut self.common.gps_time_0_diff, 2)
                .map_err(Error::Io)?;
            self.ic_gps_time
                .compress(
                    enc,
                    (self.common.last_gps_times[self.common.last] >> 32) as i32,
                    (gps_bits >> 32) as i32,
                    8,
                )
                .map_err(Error::Io)?;
            enc.write_bits(32, (gps_bits as u64 & 0xFFFF_FFFF) as u32)
                .map_err(Error::Io)?;
            self.common.next = (self.common.next + 1) & 3;
            self.common.last = self.common.next;
            self.common.last_gps_time_diffs[self.common.last] = 0;
            self.common.multi_extreme_counters[self.common.last] = 0;
            self.common.last_gps_times[self.common.last] = gps_bits;
            return Ok(());
        }

        if gps_bits == self.common.last_gps_times[self.common.last] {
            enc.encode_symbol(
                &mut self.common.gps_time_multi,
                LASZIP_GPS_TIME_MULTI_UNCHANGED as u32,
            )
            .map_err(Error::Io)?;
            return Ok(());
        }

        let curr_gps_diff_64 = gps_bits.wrapping_sub(self.common.last_gps_times[self.common.last]);
        let curr_gps_diff = curr_gps_diff_64 as i32;
        if curr_gps_diff_64 == i64::from(curr_gps_diff) {
            let last_diff = self.common.last_gps_time_diffs[self.common.last];
            let multi = quantize_i32((curr_gps_diff as f32) / (last_diff as f32));

            if multi == 1 {
                enc.encode_symbol(&mut self.common.gps_time_multi, 1)
                    .map_err(Error::Io)?;
                self.ic_gps_time
                    .compress(enc, last_diff, curr_gps_diff, 1)
                    .map_err(Error::Io)?;
                self.common.multi_extreme_counters[self.common.last] = 0;
            } else if multi > 0 {
                if multi < LASZIP_GPS_TIME_MULTI {
                    enc.encode_symbol(&mut self.common.gps_time_multi, multi as u32)
                        .map_err(Error::Io)?;
                    self.ic_gps_time
                        .compress(
                            enc,
                            multi.wrapping_mul(last_diff),
                            curr_gps_diff,
                            if multi < 10 { 2 } else { 3 },
                        )
                        .map_err(Error::Io)?;
                } else {
                    enc.encode_symbol(&mut self.common.gps_time_multi, LASZIP_GPS_TIME_MULTI as u32)
                        .map_err(Error::Io)?;
                    self.ic_gps_time
                        .compress(
                            enc,
                            LASZIP_GPS_TIME_MULTI.wrapping_mul(last_diff),
                            curr_gps_diff,
                            4,
                        )
                        .map_err(Error::Io)?;
                    self.common.multi_extreme_counters[self.common.last] += 1;
                    if self.common.multi_extreme_counters[self.common.last] > 3 {
                        self.common.last_gps_time_diffs[self.common.last] = curr_gps_diff;
                        self.common.multi_extreme_counters[self.common.last] = 0;
                    }
                }
            } else if multi < 0 {
                if multi > LASZIP_GPS_TIME_MULTI_MINUS {
                    enc.encode_symbol(
                        &mut self.common.gps_time_multi,
                        (LASZIP_GPS_TIME_MULTI - multi) as u32,
                    )
                    .map_err(Error::Io)?;
                    self.ic_gps_time
                        .compress(enc, multi.wrapping_mul(last_diff), curr_gps_diff, 5)
                        .map_err(Error::Io)?;
                } else {
                    enc.encode_symbol(
                        &mut self.common.gps_time_multi,
                        (LASZIP_GPS_TIME_MULTI - LASZIP_GPS_TIME_MULTI_MINUS) as u32,
                    )
                    .map_err(Error::Io)?;
                    self.ic_gps_time
                        .compress(
                            enc,
                            LASZIP_GPS_TIME_MULTI_MINUS.wrapping_mul(last_diff),
                            curr_gps_diff,
                            6,
                        )
                        .map_err(Error::Io)?;
                    self.common.multi_extreme_counters[self.common.last] += 1;
                    if self.common.multi_extreme_counters[self.common.last] > 3 {
                        self.common.last_gps_time_diffs[self.common.last] = curr_gps_diff;
                        self.common.multi_extreme_counters[self.common.last] = 0;
                    }
                }
            } else {
                enc.encode_symbol(&mut self.common.gps_time_multi, 0)
                    .map_err(Error::Io)?;
                self.ic_gps_time
                    .compress(enc, 0, curr_gps_diff, 7)
                    .map_err(Error::Io)?;
                self.common.multi_extreme_counters[self.common.last] += 1;
                if self.common.multi_extreme_counters[self.common.last] > 3 {
                    self.common.last_gps_time_diffs[self.common.last] = curr_gps_diff;
                    self.common.multi_extreme_counters[self.common.last] = 0;
                }
            }

            self.common.last_gps_times[self.common.last] = gps_bits;
            return Ok(());
        }

        for i in 1..4usize {
            let candidate_index = (self.common.last + i) & 3;
            let other_gps_diff_64 = gps_bits.wrapping_sub(self.common.last_gps_times[candidate_index]);
            let other_gps_diff = other_gps_diff_64 as i32;
            if other_gps_diff_64 == i64::from(other_gps_diff) {
                enc.encode_symbol(
                    &mut self.common.gps_time_multi,
                    (LASZIP_GPS_TIME_MULTI_CODE_FULL + i as i32) as u32,
                )
                .map_err(Error::Io)?;
                self.common.last = candidate_index;
                return self.compress_with(enc, gps_bits);
            }
        }

        enc.encode_symbol(
            &mut self.common.gps_time_multi,
            LASZIP_GPS_TIME_MULTI_CODE_FULL as u32,
        )
        .map_err(Error::Io)?;
        self.ic_gps_time
            .compress(
                enc,
                (self.common.last_gps_times[self.common.last] >> 32) as i32,
                (gps_bits >> 32) as i32,
                8,
            )
            .map_err(Error::Io)?;
        enc.write_bits(32, (gps_bits as u64 & 0xFFFF_FFFF) as u32)
            .map_err(Error::Io)?;
        self.common.next = (self.common.next + 1) & 3;
        self.common.last = self.common.next;
        self.common.last_gps_time_diffs[self.common.last] = 0;
        self.common.multi_extreme_counters[self.common.last] = 0;
        self.common.last_gps_times[self.common.last] = gps_bits;
        Ok(())
    }
}

struct Rgb12ModelsV2 {
    byte_used: ArithmeticSymbolModel,
    lower_red_byte: ArithmeticSymbolModel,
    upper_red_byte: ArithmeticSymbolModel,
    lower_green_byte: ArithmeticSymbolModel,
    upper_green_byte: ArithmeticSymbolModel,
    lower_blue_byte: ArithmeticSymbolModel,
    upper_blue_byte: ArithmeticSymbolModel,
}

impl Rgb12ModelsV2 {
    fn new() -> Self {
        Self {
            byte_used: ArithmeticSymbolModel::new(128),
            lower_red_byte: ArithmeticSymbolModel::new(256),
            upper_red_byte: ArithmeticSymbolModel::new(256),
            lower_green_byte: ArithmeticSymbolModel::new(256),
            upper_green_byte: ArithmeticSymbolModel::new(256),
            lower_blue_byte: ArithmeticSymbolModel::new(256),
            upper_blue_byte: ArithmeticSymbolModel::new(256),
        }
    }
}

struct Rgb12V2Compressor {
    last: Rgb16,
    models: Rgb12ModelsV2,
}

impl Rgb12V2Compressor {
    fn new(first_rgb: Rgb16) -> Self {
        Self {
            last: first_rgb,
            models: Rgb12ModelsV2::new(),
        }
    }

    fn compress_with<W: Write>(
        &mut self,
        enc: &mut ArithmeticEncoder<W>,
        current: Rgb16,
    ) -> Result<()> {
        let mut diff_l = 0i32;
        let mut diff_h = 0i32;
        let mut corr;
        let sym = (((self.last.red & 0x00FF) != (current.red & 0x00FF)) as u32) << 0
            | (((self.last.red & 0xFF00) != (current.red & 0xFF00)) as u32) << 1
            | (((self.last.green & 0x00FF) != (current.green & 0x00FF)) as u32) << 2
            | (((self.last.green & 0xFF00) != (current.green & 0xFF00)) as u32) << 3
            | (((self.last.blue & 0x00FF) != (current.blue & 0x00FF)) as u32) << 4
            | (((self.last.blue & 0xFF00) != (current.blue & 0xFF00)) as u32) << 5
            | ((((current.red & 0x00FF) != (current.green & 0x00FF)
                || (current.red & 0x00FF) != (current.blue & 0x00FF)
                || (current.red & 0xFF00) != (current.green & 0xFF00)
                || (current.red & 0xFF00) != (current.blue & 0xFF00)) as u32)
                << 6);

        enc.encode_symbol(&mut self.models.byte_used, sym)
            .map_err(Error::Io)?;

        if (sym & (1 << 0)) != 0 {
            diff_l = i32::from((current.red & 0x00FF) as u8) - i32::from((self.last.red & 0x00FF) as u8);
            enc.encode_symbol(&mut self.models.lower_red_byte, u8_fold(diff_l) as u32)
                .map_err(Error::Io)?;
        }

        if (sym & (1 << 1)) != 0 {
            diff_h = i32::from((current.red >> 8) as u8) - i32::from((self.last.red >> 8) as u8);
            enc.encode_symbol(&mut self.models.upper_red_byte, u8_fold(diff_h) as u32)
                .map_err(Error::Io)?;
        }

        if (sym & (1 << 6)) != 0 {
            if (sym & (1 << 2)) != 0 {
                corr = i32::from((current.green & 0x00FF) as u8)
                    - i32::from(u8_clamp(diff_l + i32::from((self.last.green & 0x00FF) as u8)));
                enc.encode_symbol(&mut self.models.lower_green_byte, u8_fold(corr) as u32)
                    .map_err(Error::Io)?;
            }
            if (sym & (1 << 4)) != 0 {
                diff_l = (diff_l
                    + i32::from((current.green & 0x00FF) as u8)
                    - i32::from((self.last.green & 0x00FF) as u8))
                    / 2;
                corr = i32::from((current.blue & 0x00FF) as u8)
                    - i32::from(u8_clamp(diff_l + i32::from((self.last.blue & 0x00FF) as u8)));
                enc.encode_symbol(&mut self.models.lower_blue_byte, u8_fold(corr) as u32)
                    .map_err(Error::Io)?;
            }
            if (sym & (1 << 3)) != 0 {
                corr = i32::from((current.green >> 8) as u8)
                    - i32::from(u8_clamp(diff_h + i32::from((self.last.green >> 8) as u8)));
                enc.encode_symbol(&mut self.models.upper_green_byte, u8_fold(corr) as u32)
                    .map_err(Error::Io)?;
            }
            if (sym & (1 << 5)) != 0 {
                diff_h = (diff_h
                    + i32::from((current.green >> 8) as u8)
                    - i32::from((self.last.green >> 8) as u8))
                    / 2;
                corr = i32::from((current.blue >> 8) as u8)
                    - i32::from(u8_clamp(diff_h + i32::from((self.last.blue >> 8) as u8)));
                enc.encode_symbol(&mut self.models.upper_blue_byte, u8_fold(corr) as u32)
                    .map_err(Error::Io)?;
            }
        }

        self.last = current;
        Ok(())
    }
}

struct ExtraBytesV2Compressor {
    last_bytes: Vec<u8>,
    models: Vec<ArithmeticSymbolModel>,
}

impl ExtraBytesV2Compressor {
    fn new(first_bytes: &[u8]) -> Self {
        Self {
            last_bytes: first_bytes.to_vec(),
            models: (0..first_bytes.len())
                .map(|_| ArithmeticSymbolModel::new(256))
                .collect(),
        }
    }

    fn compress_with<W: Write>(
        &mut self,
        enc: &mut ArithmeticEncoder<W>,
        current_bytes: &[u8],
    ) -> Result<()> {
        for (idx, current) in current_bytes.iter().copied().enumerate() {
            enc.encode_symbol(
                &mut self.models[idx],
                u8_fold(i32::from(current) - i32::from(self.last_bytes[idx])) as u32,
            )
            .map_err(Error::Io)?;
            self.last_bytes[idx] = current;
        }
        Ok(())
    }
}

#[inline]
fn u32_zero_bit(n: u32) -> u32 {
    n & 0xFF_FF_FF_FEu32
}

#[inline]
fn u8_clamp(v: i32) -> u8 {
    if v < 0 {
        0
    } else if v > i32::from(u8::MAX) {
        u8::MAX
    } else {
        v as u8
    }
}

#[inline]
fn u8_fold(v: i32) -> u8 {
    v as u8
}

fn quantize_i32(value: f32) -> i32 {
    if value >= 0.0 {
        (value + 0.5) as i32
    } else {
        (value - 0.5) as i32
    }
}

/// Encode one standard LASzip pointwise chunk for Point10-family item lists.
pub fn encode_standard_pointwise_chunk_point10_v2(
    points: &[PointRecord],
    point_data_format: PointDataFormat,
    expected_extra_bytes_count: usize,
    scales: [f64; 3],
    offsets: [f64; 3],
) -> Result<Vec<u8>> {
    if points.is_empty() {
        return Ok(Vec::new());
    }

    match point_data_format {
        PointDataFormat::Pdrf0
        | PointDataFormat::Pdrf1
        | PointDataFormat::Pdrf2
        | PointDataFormat::Pdrf3
        | PointDataFormat::Pdrf4
        | PointDataFormat::Pdrf5 => {}
        _ => {
            return Err(crate::Error::Unimplemented(
                "standard LASzip Point10 writer currently supports only PDRF0/1/2/3/4/5",
            ));
        }
    }

    let waveform_bytes_count = if point_data_format.has_waveform() {
        29usize
    } else {
        0usize
    };
    let expected_payload_extra_count = expected_extra_bytes_count.saturating_add(waveform_bytes_count);

    let raw_points: Vec<RawPoint10> = points
        .iter()
        .map(|point| RawPoint10::from_point_record(point, point_data_format, scales, offsets))
        .collect::<Result<Vec<_>>>()?;

    let gps_items: Vec<i64> = if point_data_format.has_gps_time() {
        points
            .iter()
            .map(|point| {
                point
                    .gps_time
                    .map(|value| i64::from_ne_bytes(value.0.to_bits().to_ne_bytes()))
                    .ok_or_else(|| crate::Error::InvalidValue {
                        field: "laz.standard_point10_writer.gps_time",
                        detail: "point is missing gps_time required by the point data format"
                            .to_string(),
                    })
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };

    let rgb_items: Vec<Rgb16> = if point_data_format.has_rgb() {
        points
            .iter()
            .map(|point| {
                point.color.ok_or_else(|| crate::Error::InvalidValue {
                    field: "laz.standard_point10_writer.rgb",
                    detail: "point is missing RGB required by the point data format".to_string(),
                })
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };

    let extra_items: Vec<Vec<u8>> = if expected_payload_extra_count > 0 {
        points
            .iter()
            .map(|point| {
                let mut bytes = Vec::with_capacity(expected_payload_extra_count);
                if waveform_bytes_count > 0 {
                    bytes.extend_from_slice(&encode_waveform_bytes(point));
                }
                if expected_extra_bytes_count > 0 {
                    bytes.extend_from_slice(&point.extra_bytes.data[..expected_extra_bytes_count]);
                }
                bytes
            })
            .collect()
    } else {
        Vec::new()
    };

    let mut out = Vec::<u8>::new();
    out.extend_from_slice(&raw_points[0].to_20_bytes());
    if point_data_format.has_gps_time() {
        out.extend_from_slice(&gps_items[0].to_le_bytes());
    }
    if point_data_format.has_rgb() {
        out.extend_from_slice(&rgb_items[0].red.to_le_bytes());
        out.extend_from_slice(&rgb_items[0].green.to_le_bytes());
        out.extend_from_slice(&rgb_items[0].blue.to_le_bytes());
    }
    if expected_payload_extra_count > 0 {
        out.extend_from_slice(&extra_items[0]);
    }

    if points.len() == 1 {
        return Ok(out);
    }

    {
        let mut enc = ArithmeticEncoder::new(&mut out);
        let mut point10 = Point10V2Compressor::new(raw_points[0]);
        let mut gps = point_data_format
            .has_gps_time()
            .then(|| GpsTimeV2Compressor::new(gps_items[0]));
        let mut rgb = point_data_format
            .has_rgb()
            .then(|| Rgb12V2Compressor::new(rgb_items[0]));
        let mut extra = (expected_payload_extra_count > 0)
            .then(|| ExtraBytesV2Compressor::new(&extra_items[0]));

        for point_index in 1..points.len() {
            point10.compress_with(&mut enc, raw_points[point_index])?;
            if let Some(gps_encoder) = gps.as_mut() {
                gps_encoder.compress_with(&mut enc, gps_items[point_index])?;
            }
            if let Some(rgb_encoder) = rgb.as_mut() {
                rgb_encoder.compress_with(&mut enc, rgb_items[point_index])?;
            }
            if let Some(extra_encoder) = extra.as_mut() {
                extra_encoder.compress_with(&mut enc, &extra_items[point_index])?;
            }
        }

        let _ = enc.done().map_err(Error::Io)?;
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::encode_standard_pointwise_chunk_point10_v2;
    use crate::las::header::PointDataFormat;
    use crate::laz::standard_point10::decode_standard_pointwise_chunk_point10_v2;
    use crate::laz::LaszipItemSpec;
    use crate::point::{GpsTime, PointRecord, Rgb16};
    use crate::Result;

    #[test]
    fn point10_codec_roundtrip_pdrf3_with_extra_bytes() -> Result<()> {
        let mut p1 = PointRecord {
            x: 101.0,
            y: 202.0,
            z: 303.0,
            intensity: 500,
            classification: 2,
            flags: 1,
            return_number: 1,
            number_of_returns: 1,
            scan_direction_flag: false,
            edge_of_flight_line: false,
            scan_angle: -5,
            user_data: 9,
            point_source_id: 77,
            gps_time: Some(GpsTime(1000.25)),
            color: Some(Rgb16 {
                red: 1000,
                green: 2000,
                blue: 3000,
            }),
            ..PointRecord::default()
        };
        p1.extra_bytes.data[0] = 11;
        p1.extra_bytes.data[1] = 22;
        p1.extra_bytes.len = 2;

        let mut p2 = p1;
        p2.x = 111.0;
        p2.y = 212.0;
        p2.z = 313.0;
        p2.intensity = 503;
        p2.classification = 3;
        p2.scan_direction_flag = true;
        p2.scan_angle = -3;
        p2.user_data = 10;
        p2.point_source_id = 79;
        p2.gps_time = Some(GpsTime(1001.25));
        p2.color = Some(Rgb16 {
            red: 1100,
            green: 2100,
            blue: 3100,
        });
        p2.extra_bytes.data[0] = 33;

        let points = vec![p1, p2];
        let scales = [1.0, 1.0, 1.0];
        let offsets = [0.0, 0.0, 0.0];
        let item_specs = vec![
            LaszipItemSpec {
                item_type: 6,
                item_size: 20,
                item_version: 2,
            },
            LaszipItemSpec {
                item_type: 7,
                item_size: 8,
                item_version: 2,
            },
            LaszipItemSpec {
                item_type: 8,
                item_size: 6,
                item_version: 2,
            },
            LaszipItemSpec {
                item_type: 0,
                item_size: 2,
                item_version: 2,
            },
        ];

        let encoded = encode_standard_pointwise_chunk_point10_v2(
            &points,
            PointDataFormat::Pdrf3,
            2,
            scales,
            offsets,
        )?;
        let decoded = decode_standard_pointwise_chunk_point10_v2(
            &encoded,
            points.len(),
            &item_specs,
            PointDataFormat::Pdrf3,
            2,
            scales,
            offsets,
        )?;

        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].intensity, 500);
        assert_eq!(decoded[1].intensity, 503);
        assert_eq!(decoded[0].color, p1.color);
        assert_eq!(decoded[1].color, p2.color);
        assert_eq!(decoded[0].extra_bytes.len, 2);
        assert_eq!(decoded[1].extra_bytes.len, 2);
        assert_eq!(decoded[0].extra_bytes.data[0], 11);
        assert_eq!(decoded[1].extra_bytes.data[0], 33);
        Ok(())
    }

    #[test]
    fn point10_codec_rejects_missing_required_rgb() {
        let points = vec![PointRecord {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            gps_time: Some(GpsTime(10.0)),
            ..PointRecord::default()
        }];

        let err = encode_standard_pointwise_chunk_point10_v2(
            &points,
            PointDataFormat::Pdrf3,
            0,
            [1.0, 1.0, 1.0],
            [0.0, 0.0, 0.0],
        )
        .expect_err("expected missing RGB rejection");

        assert!(format!("{err}").contains("requires RGB"));
    }
}