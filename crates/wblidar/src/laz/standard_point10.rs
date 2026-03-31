//! Standard LASzip pointwise Point10-family (v2) decoding.

use std::io::{Cursor, Read};

use crate::las::header::PointDataFormat;
use crate::laz::arithmetic_decoder::ArithmeticDecoder;
use crate::laz::arithmetic_model::ArithmeticSymbolModel;
use crate::laz::integer_codec::IntegerDecompressor;
use crate::laz::LaszipItemSpec;
use crate::point::{GpsTime, PointRecord, Rgb16, WaveformPacket};
use crate::Result;

const LASZIP_ITEM_BYTE: u16 = 0;
const LASZIP_ITEM_POINT10: u16 = 6;
const LASZIP_ITEM_GPSTIME: u16 = 7;
const LASZIP_ITEM_RGB12: u16 = 8;

const LASZIP_GPS_TIME_MULTI: i32 = 500;
const LASZIP_GPS_TIME_MULTI_MINUS: i32 = -10;
const LASZIP_GPS_TIME_MULTI_UNCHANGED: i32 =
    LASZIP_GPS_TIME_MULTI - LASZIP_GPS_TIME_MULTI_MINUS + 1;
const LASZIP_GPS_TIME_MULTI_CODE_FULL: i32 =
    LASZIP_GPS_TIME_MULTI - LASZIP_GPS_TIME_MULTI_MINUS + 2;
const LASZIP_GPS_TIME_MULTI_TOTAL: i32 = LASZIP_GPS_TIME_MULTI - LASZIP_GPS_TIME_MULTI_MINUS + 6;

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
        } else {
            if self.values[2] < v {
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
    }

    fn get(&self) -> i32 {
        self.values[2]
    }
}

#[inline]
fn u32_zero_bit(n: u32) -> u32 {
    n & 0xFF_FF_FF_FEu32
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
    fn from_20_bytes(buf: &[u8]) -> Result<Self> {
        if buf.len() < 20 {
            return Err(crate::Error::SizeMismatch {
                context: "laz.point10.first_point",
                expected: 20,
                actual: buf.len(),
            });
        }
        Ok(Self {
            x: i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
            y: i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]),
            z: i32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]),
            intensity: u16::from_le_bytes([buf[12], buf[13]]),
            bit_fields: buf[14],
            classification: buf[15],
            scan_angle_rank: buf[16] as i8,
            user_data: buf[17],
            point_source_id: u16::from_le_bytes([buf[18], buf[19]]),
        })
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

struct Point10V2Decompressor {
    last_point: RawPoint10,
    ic_intensity: IntegerDecompressor,
    ic_point_source_id: IntegerDecompressor,
    ic_dx: IntegerDecompressor,
    ic_dy: IntegerDecompressor,
    ic_z: IntegerDecompressor,
    common: Point10Common,
}

impl Point10V2Decompressor {
    fn new() -> Self {
        Self {
            last_point: RawPoint10::default(),
            ic_intensity: IntegerDecompressor::new(16, 4, 8, 0),
            ic_point_source_id: IntegerDecompressor::new(16, 1, 8, 0),
            ic_dx: IntegerDecompressor::new(32, 2, 8, 0),
            ic_dy: IntegerDecompressor::new(32, 22, 8, 0),
            ic_z: IntegerDecompressor::new(32, 20, 8, 0),
            common: Point10Common::new(),
        }
    }

    fn decompress_first<R: Read>(&mut self, src: &mut R) -> Result<RawPoint10> {
        let mut buf = [0u8; 20];
        src.read_exact(&mut buf)?;
        let raw = RawPoint10::from_20_bytes(&buf)?;
        self.last_point = raw;
        self.last_point.intensity = 0;
        Ok(raw)
    }

    fn decompress_with<R: Read>(&mut self, dec: &mut ArithmeticDecoder<R>) -> Result<RawPoint10> {
        let changed_value = dec.decode_symbol(&mut self.common.changed_values)? as i32;

        let r;
        let n;
        let m;
        let l;

        if changed_value != 0 {
            if (changed_value & (1 << 5)) != 0 {
                let next = dec.decode_symbol(
                    &mut self.common.bit_byte[self.last_point.bit_fields as usize],
                )? as u8;
                self.last_point.bit_fields = next;
            }

            r = self.last_point.return_number();
            n = self.last_point.number_of_returns();
            m = NUMBER_RETURN_MAP[n as usize][r as usize];
            l = NUMBER_RETURN_LEVEL[n as usize][r as usize];

            if (changed_value & (1 << 4)) != 0 {
                self.last_point.intensity = self.ic_intensity.decompress(
                    dec,
                    self.common.last_intensity[m as usize] as i32,
                    if m < 3 { m as u32 } else { 3 },
                )? as u16;
                self.common.last_intensity[m as usize] = self.last_point.intensity;
            } else {
                self.last_point.intensity = self.common.last_intensity[m as usize];
            }

            if (changed_value & (1 << 3)) != 0 {
                self.last_point.classification = dec.decode_symbol(
                    &mut self.common.classification[self.last_point.classification as usize],
                )? as u8;
            }

            if (changed_value & (1 << 2)) != 0 {
                let delta = dec.decode_symbol(
                    &mut self.common.scan_angle_rank[self.last_point.scan_direction_flag() as usize],
                )? as u8;
                self.last_point.scan_angle_rank = self.last_point.scan_angle_rank.wrapping_add(delta as i8);
            }

            if (changed_value & (1 << 1)) != 0 {
                self.last_point.user_data = dec.decode_symbol(
                    &mut self.common.user_data[self.last_point.user_data as usize],
                )? as u8;
            }

            if (changed_value & 1) != 0 {
                self.last_point.point_source_id = self.ic_point_source_id.decompress(
                    dec,
                    self.last_point.point_source_id as i32,
                    0,
                )? as u16;
            }
        } else {
            r = self.last_point.return_number();
            n = self.last_point.number_of_returns();
            m = NUMBER_RETURN_MAP[n as usize][r as usize];
            l = NUMBER_RETURN_LEVEL[n as usize][r as usize];
        }

        let median_x = self.common.last_x_diff_median[m as usize].get();
        let diff_x = self.ic_dx.decompress(dec, median_x, (n == 1) as u32)?;
        self.last_point.x = self.last_point.x.wrapping_add(diff_x);
        self.common.last_x_diff_median[m as usize].add(diff_x);

        let median_y = self.common.last_y_diff_median[m as usize].get();
        let k_bits = self.ic_dx.k();
        let context_y = (n == 1) as u32 + if k_bits < 20 { u32_zero_bit(k_bits) } else { 20 };
        let diff_y = self.ic_dy.decompress(dec, median_y, context_y)?;
        self.last_point.y = self.last_point.y.wrapping_add(diff_y);
        self.common.last_y_diff_median[m as usize].add(diff_y);

        let k_bits = (self.ic_dx.k() + self.ic_dy.k()) / 2;
        let context_z = (n == 1) as u32 + if k_bits < 18 { u32_zero_bit(k_bits) } else { 18 };
        self.last_point.z = self
            .ic_z
            .decompress(dec, self.common.last_height[l as usize], context_z)?;
        self.common.last_height[l as usize] = self.last_point.z;

        Ok(self.last_point)
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
    fn new() -> Self {
        Self {
            gps_time_multi: ArithmeticSymbolModel::new(LASZIP_GPS_TIME_MULTI_TOTAL as u32),
            gps_time_0_diff: ArithmeticSymbolModel::new(6),
            last: 0,
            next: 0,
            last_gps_times: [0; 4],
            last_gps_time_diffs: [0; 4],
            multi_extreme_counters: [0; 4],
        }
    }
}

struct GpsTimeV2Decompressor {
    common: GpsV2Common,
    ic_gps_time: IntegerDecompressor,
}

impl GpsTimeV2Decompressor {
    fn new() -> Self {
        Self {
            common: GpsV2Common::new(),
            ic_gps_time: IntegerDecompressor::new(32, 9, 8, 0),
        }
    }

    fn decompress_first<R: Read>(&mut self, src: &mut R) -> Result<i64> {
        let mut b = [0u8; 8];
        src.read_exact(&mut b)?;
        let v = i64::from_le_bytes(b);
        self.common.last_gps_times[0] = v;
        Ok(v)
    }

    fn decompress_with<R: Read>(&mut self, dec: &mut ArithmeticDecoder<R>) -> Result<i64> {
        loop {
            if self.common.last_gps_time_diffs[self.common.last] == 0 {
                let multi = dec.decode_symbol(&mut self.common.gps_time_0_diff)? as i32;

                if multi == 1 {
                    self.common.last_gps_time_diffs[self.common.last] =
                        self.ic_gps_time.decompress(dec, 0, 0)?;
                    self.common.last_gps_times[self.common.last] = self.common.last_gps_times
                        [self.common.last]
                        .wrapping_add(i64::from(self.common.last_gps_time_diffs[self.common.last]));
                    self.common.multi_extreme_counters[self.common.last] = 0;
                } else if multi == 2 {
                    self.common.next = (self.common.next + 1) & 3;
                    let hi = self.ic_gps_time.decompress(
                        dec,
                        (self.common.last_gps_times[self.common.last] >> 32) as i32,
                        8,
                    )?;
                    let lo = dec.read_int()?;
                    self.common.last_gps_times[self.common.next] = ((hi as i64) << 32) | i64::from(lo);
                    self.common.last = self.common.next;
                    self.common.last_gps_time_diffs[self.common.last] = 0;
                    self.common.multi_extreme_counters[self.common.last] = 0;
                } else if multi > 2 {
                    self.common.last = (self.common.last + (multi as usize) - 2) & 3;
                    continue;
                }
            } else {
                let mut multi = dec.decode_symbol(&mut self.common.gps_time_multi)? as i32;

                if multi == 1 {
                    self.common.last_gps_times[self.common.last] = self.common.last_gps_times
                        [self.common.last]
                        .wrapping_add(i64::from(
                            self.ic_gps_time.decompress(
                                dec,
                                self.common.last_gps_time_diffs[self.common.last],
                                1,
                            )?,
                        ));
                    self.common.multi_extreme_counters[self.common.last] = 0;
                } else if multi < LASZIP_GPS_TIME_MULTI_UNCHANGED {
                    let gps_time_diff: i32;
                    if multi == 0 {
                        gps_time_diff = self.ic_gps_time.decompress(dec, 0, 7)?;
                        self.common.multi_extreme_counters[self.common.last] += 1;
                        if self.common.multi_extreme_counters[self.common.last] > 3 {
                            self.common.last_gps_time_diffs[self.common.last] = gps_time_diff;
                            self.common.multi_extreme_counters[self.common.last] = 0;
                        }
                    } else if multi < LASZIP_GPS_TIME_MULTI {
                        if multi < 10 {
                            gps_time_diff = self.ic_gps_time.decompress(
                                dec,
                                multi.wrapping_mul(self.common.last_gps_time_diffs[self.common.last]),
                                2,
                            )?;
                        } else {
                            gps_time_diff = self.ic_gps_time.decompress(
                                dec,
                                multi.wrapping_mul(self.common.last_gps_time_diffs[self.common.last]),
                                3,
                            )?;
                        }
                    } else if multi == LASZIP_GPS_TIME_MULTI {
                        gps_time_diff = self.ic_gps_time.decompress(
                            dec,
                            multi.wrapping_mul(self.common.last_gps_time_diffs[self.common.last]),
                            4,
                        )?;
                        self.common.multi_extreme_counters[self.common.last] += 1;
                        if self.common.multi_extreme_counters[self.common.last] > 3 {
                            self.common.last_gps_time_diffs[self.common.last] = gps_time_diff;
                            self.common.multi_extreme_counters[self.common.last] = 0;
                        }
                    } else {
                        multi = LASZIP_GPS_TIME_MULTI - multi;
                        if multi > LASZIP_GPS_TIME_MULTI_MINUS {
                            gps_time_diff = self.ic_gps_time.decompress(
                                dec,
                                multi.wrapping_mul(self.common.last_gps_time_diffs[self.common.last]),
                                5,
                            )?;
                        } else {
                            gps_time_diff = self.ic_gps_time.decompress(
                                dec,
                                LASZIP_GPS_TIME_MULTI_MINUS
                                    .wrapping_mul(self.common.last_gps_time_diffs[self.common.last]),
                                6,
                            )?;
                            self.common.multi_extreme_counters[self.common.last] += 1;
                            if self.common.multi_extreme_counters[self.common.last] > 3 {
                                self.common.last_gps_time_diffs[self.common.last] = gps_time_diff;
                                self.common.multi_extreme_counters[self.common.last] = 0;
                            }
                        }
                    }
                    self.common.last_gps_times[self.common.last] = self.common.last_gps_times
                        [self.common.last]
                        .wrapping_add(i64::from(gps_time_diff));
                } else if multi == LASZIP_GPS_TIME_MULTI_CODE_FULL {
                    self.common.next = (self.common.next + 1) & 3;
                    let hi = self.ic_gps_time.decompress(
                        dec,
                        (self.common.last_gps_times[self.common.last] >> 32) as i32,
                        8,
                    )?;
                    let lo = dec.read_int()?;
                    self.common.last_gps_times[self.common.next] = ((hi as i64) << 32) | i64::from(lo);
                    self.common.last = self.common.next;
                    self.common.last_gps_time_diffs[self.common.last] = 0;
                    self.common.multi_extreme_counters[self.common.last] = 0;
                } else if multi > LASZIP_GPS_TIME_MULTI_CODE_FULL {
                    self.common.last =
                        (self.common.last + multi as usize - LASZIP_GPS_TIME_MULTI_CODE_FULL as usize)
                            & 3;
                    continue;
                }
            }

            return Ok(self.common.last_gps_times[self.common.last]);
        }
    }
}

struct ExtraBytesV2Decompressor {
    last_bytes: Vec<u8>,
    diffs: Vec<u8>,
    models: Vec<ArithmeticSymbolModel>,
}

impl ExtraBytesV2Decompressor {
    fn new(count: usize) -> Self {
        Self {
            last_bytes: vec![0; count],
            diffs: vec![0; count],
            models: (0..count).map(|_| ArithmeticSymbolModel::new(256)).collect(),
        }
    }

    fn decompress_first<R: Read>(&mut self, src: &mut R) -> Result<Vec<u8>> {
        src.read_exact(&mut self.last_bytes)?;
        Ok(self.last_bytes.clone())
    }

    fn decompress_with<R: Read>(&mut self, dec: &mut ArithmeticDecoder<R>) -> Result<Vec<u8>> {
        for i in 0..self.last_bytes.len() {
            let sym = dec.decode_symbol(&mut self.models[i])? as u8;
            self.diffs[i] = self.last_bytes[i].wrapping_add(sym);
        }
        self.last_bytes.copy_from_slice(&self.diffs);
        Ok(self.last_bytes.clone())
    }
}

enum ItemDecoder {
    Point10(Point10V2Decompressor),
    Gps(GpsTimeV2Decompressor),
    Rgb(Rgb12V2Decompressor),
    Extra(ExtraBytesV2Decompressor),
}

impl ItemDecoder {
    fn name(&self) -> &'static str {
        match self {
            ItemDecoder::Point10(_) => "point10",
            ItemDecoder::Gps(_) => "gps",
            ItemDecoder::Rgb(_) => "rgb",
            ItemDecoder::Extra(_) => "extra-bytes",
        }
    }
}

#[derive(Clone, Copy)]
struct Point10AssembledState {
    point10: RawPoint10,
    gps_bits: Option<i64>,
    rgb: Option<Rgb16>,
}

impl Default for Point10AssembledState {
    fn default() -> Self {
        Self {
            point10: RawPoint10::default(),
            gps_bits: None,
            rgb: None,
        }
    }
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

struct Rgb12V2Decompressor {
    last: Rgb16,
    models: Rgb12ModelsV2,
}

impl Rgb12V2Decompressor {
    fn new() -> Self {
        Self {
            last: Rgb16::default(),
            models: Rgb12ModelsV2::new(),
        }
    }

    fn decompress_first<R: Read>(&mut self, src: &mut R) -> Result<Rgb16> {
        let mut b = [0u8; 6];
        src.read_exact(&mut b)?;
        self.last = Rgb16 {
            red: u16::from_le_bytes([b[0], b[1]]),
            green: u16::from_le_bytes([b[2], b[3]]),
            blue: u16::from_le_bytes([b[4], b[5]]),
        };
        Ok(self.last)
    }

    fn decompress_with<R: Read>(&mut self, dec: &mut ArithmeticDecoder<R>) -> Result<Rgb16> {
        let sym = dec.decode_symbol(&mut self.models.byte_used)?;
        let changed = sym as u8;

        let mut current = Rgb16::default();

        let lower_red_changed = (changed & (1 << 0)) != 0;
        let upper_red_changed = (changed & (1 << 1)) != 0;
        let lower_green_changed = (changed & (1 << 2)) != 0;
        let upper_green_changed = (changed & (1 << 3)) != 0;
        let lower_blue_changed = (changed & (1 << 4)) != 0;
        let upper_blue_changed = (changed & (1 << 5)) != 0;

        let last_red_lo = (self.last.red & 0x00FF) as u8;
        let last_red_hi = (self.last.red >> 8) as u8;
        let last_green_lo = (self.last.green & 0x00FF) as u8;
        let last_green_hi = (self.last.green >> 8) as u8;
        let last_blue_lo = (self.last.blue & 0x00FF) as u8;
        let last_blue_hi = (self.last.blue >> 8) as u8;

        let red_lo = if lower_red_changed {
            let corr = dec.decode_symbol(&mut self.models.lower_red_byte)? as u8;
            corr.wrapping_add(last_red_lo)
        } else {
            last_red_lo
        };
        current.red = u16::from(red_lo);

        let red_hi = if upper_red_changed {
            let corr = dec.decode_symbol(&mut self.models.upper_red_byte)? as u8;
            corr.wrapping_add(last_red_hi)
        } else {
            last_red_hi
        };
        current.red |= u16::from(red_hi) << 8;

        if (sym & (1 << 6)) != 0 {
            let mut diff = i32::from(red_lo) - i32::from(last_red_lo);

            let green_lo = if lower_green_changed {
                let corr = dec.decode_symbol(&mut self.models.lower_green_byte)? as u8;
                corr.wrapping_add(u8_clamp(diff + i32::from(last_green_lo)))
            } else {
                last_green_lo
            };
            current.green = u16::from(green_lo);

            let blue_lo = if lower_blue_changed {
                let corr = dec.decode_symbol(&mut self.models.lower_blue_byte)? as u8;
                diff = (diff + i32::from(green_lo) - i32::from(last_green_lo)) / 2;
                corr.wrapping_add(u8_clamp(diff + i32::from(last_blue_lo)))
            } else {
                last_blue_lo
            };
            current.blue = u16::from(blue_lo);

            diff = i32::from(red_hi) - i32::from(last_red_hi);

            let green_hi = if upper_green_changed {
                let corr = dec.decode_symbol(&mut self.models.upper_green_byte)? as u8;
                corr.wrapping_add(u8_clamp(diff + i32::from(last_green_hi)))
            } else {
                last_green_hi
            };
            current.green |= u16::from(green_hi) << 8;

            let blue_hi = if upper_blue_changed {
                let corr = dec.decode_symbol(&mut self.models.upper_blue_byte)? as u8;
                diff = (diff + i32::from(green_hi) - i32::from(last_green_hi)) / 2;
                corr.wrapping_add(u8_clamp(diff + i32::from(last_blue_hi)))
            } else {
                last_blue_hi
            };
            current.blue |= u16::from(blue_hi) << 8;
        } else {
            current.green = current.red;
            current.blue = current.red;
        }

        self.last = current;
        Ok(current)
    }
}

fn assemble_point_record(
    state: Point10AssembledState,
    extra: Option<&[u8]>,
    point_data_format: PointDataFormat,
    expected_extra_bytes_count: usize,
    scales: [f64; 3],
    offsets: [f64; 3],
) -> PointRecord {
    let mut out = PointRecord {
        x: state.point10.x as f64 * scales[0] + offsets[0],
        y: state.point10.y as f64 * scales[1] + offsets[1],
        z: state.point10.z as f64 * scales[2] + offsets[2],
        intensity: state.point10.intensity,
        classification: state.point10.classification & 0x1F,
        user_data: state.point10.user_data,
        point_source_id: state.point10.point_source_id,
        flags: (state.point10.classification >> 5) & 0x07,
        return_number: state.point10.bit_fields & 0x07,
        number_of_returns: (state.point10.bit_fields >> 3) & 0x07,
        scan_direction_flag: (state.point10.bit_fields & 0x40) != 0,
        edge_of_flight_line: (state.point10.bit_fields & 0x80) != 0,
        scan_angle: state.point10.scan_angle_rank as i16,
        gps_time: state.gps_bits.map(|v| GpsTime(f64::from_bits(v as u64))),
        color: state.rgb,
        ..PointRecord::default()
    };

    if let Some(extra_bytes) = extra {
        let mut payload = extra_bytes;
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

        let declared = expected_extra_bytes_count.min(payload.len());
        let copy_len = usize::min(declared, 192);
        out.extra_bytes.data[..copy_len].copy_from_slice(&payload[..copy_len]);
        out.extra_bytes.len = copy_len as u8;
    }

    out
}

/// Decode one standard LASzip pointwise chunk for Point10-family item lists.
pub fn decode_standard_pointwise_chunk_point10_v2(
    chunk_bytes: &[u8],
    point_count: usize,
    item_specs: &[LaszipItemSpec],
    point_data_format: PointDataFormat,
    expected_extra_bytes_count: usize,
    scales: [f64; 3],
    offsets: [f64; 3],
) -> Result<Vec<PointRecord>> {
    if point_count == 0 {
        return Ok(Vec::new());
    }

    let mut decoders = Vec::<ItemDecoder>::new();
    let mut has_point10 = false;
    let mut has_gps_item = false;
    let mut has_rgb_item = false;
    let waveform_bytes_count = if point_data_format.has_waveform() {
        29usize
    } else {
        0usize
    };
    let expected_payload_extra_count = expected_extra_bytes_count.saturating_add(waveform_bytes_count);
    let mut remaining_expected_extra = expected_payload_extra_count;

    for item in item_specs {
        match item.item_type {
            LASZIP_ITEM_POINT10 => {
                if item.item_version != 2 || item.item_size != 20 {
                    return Err(crate::Error::Unimplemented(
                        "standard LASzip Point10 currently requires item version 2 and size 20",
                    ));
                }
                has_point10 = true;
                decoders.push(ItemDecoder::Point10(Point10V2Decompressor::new()));
            }
            LASZIP_ITEM_GPSTIME => {
                if item.item_version != 2 || item.item_size != 8 {
                    return Err(crate::Error::Unimplemented(
                        "standard LASzip GPS currently requires item version 2 and size 8",
                    ));
                }
                has_gps_item = true;
                decoders.push(ItemDecoder::Gps(GpsTimeV2Decompressor::new()));
            }
            LASZIP_ITEM_RGB12 => {
                if item.item_version != 2 || item.item_size != 6 {
                    return Err(crate::Error::Unimplemented(
                        "standard LASzip RGB12 currently requires item version 2 and size 6",
                    ));
                }
                has_rgb_item = true;
                decoders.push(ItemDecoder::Rgb(Rgb12V2Decompressor::new()));
            }
            LASZIP_ITEM_BYTE => {
                if item.item_version != 2 {
                    return Err(crate::Error::Unimplemented(
                        "standard LASzip extra-bytes currently requires item version 2",
                    ));
                }

                // Some producer pipelines emit LASzip BYTE items that do not match
                // LAS header extra-bytes declarations. Keep decoding aligned with
                // header-declared point record size.
                if remaining_expected_extra == 0 {
                    continue;
                }

                let advertised = item.item_size as usize;
                let to_decode = advertised.min(remaining_expected_extra);
                if to_decode == 0 {
                    continue;
                }

                decoders.push(ItemDecoder::Extra(ExtraBytesV2Decompressor::new(to_decode)));
                remaining_expected_extra -= to_decode;
            }
            _ => {
                return Err(crate::Error::Unimplemented(
                    "standard LASzip Point10 path does not yet support this item type",
                ));
            }
        }
    }

    if !has_point10 {
        return Err(crate::Error::InvalidValue {
            field: "laz.laszip_items",
            detail: "Point10 item missing for Point10 standard decode path".to_string(),
        });
    }
    if point_data_format.has_gps_time() && !has_gps_item {
        return Err(crate::Error::InvalidValue {
            field: "laz.laszip_items",
            detail: "GPS item missing for PDRF requiring gps_time".to_string(),
        });
    }
    if remaining_expected_extra > 0 {
        return Err(crate::Error::InvalidValue {
            field: "laz.laszip_items",
            detail: format!(
                "LAS header + waveform declares {} bytes per point, but LASzip item list only declares {} bytes",
                expected_payload_extra_count,
                expected_payload_extra_count - remaining_expected_extra
            ),
        });
    }
    if point_data_format.has_rgb() && !has_rgb_item {
        // Some producer pipelines emit non-conformant PDRF3 streams that omit
        // RGB12 in the LASzip item list. Decode core attributes and leave
        // `PointRecord.color` as `None` rather than hard-failing.
    }

    let mut cursor = Cursor::new(chunk_bytes);
    let mut state = Point10AssembledState::default();
    let mut extra_bytes: Option<Vec<u8>> = None;

    for d in &mut decoders {
        let name = d.name();
        match d {
            ItemDecoder::Point10(p) => {
                state.point10 = p.decompress_first(&mut cursor).map_err(|err| {
                    crate::Error::InvalidValue {
                        field: "laz.standard_point10_chunk_seed",
                        detail: format!(
                            "failed while reading first raw {name} item of standard Point10 chunk: {err}"
                        ),
                    }
                })?;
            }
            ItemDecoder::Gps(g) => {
                state.gps_bits = Some(g.decompress_first(&mut cursor).map_err(|err| {
                    crate::Error::InvalidValue {
                        field: "laz.standard_point10_chunk_seed",
                        detail: format!(
                            "failed while reading first raw {name} item of standard Point10 chunk: {err}"
                        ),
                    }
                })?);
            }
            ItemDecoder::Rgb(rgb) => {
                state.rgb = Some(rgb.decompress_first(&mut cursor).map_err(|err| {
                    crate::Error::InvalidValue {
                        field: "laz.standard_point10_chunk_seed",
                        detail: format!(
                            "failed while reading first raw {name} item of standard Point10 chunk: {err}"
                        ),
                    }
                })?);
            }
            ItemDecoder::Extra(e) => {
                extra_bytes = Some(e.decompress_first(&mut cursor).map_err(|err| {
                    crate::Error::InvalidValue {
                        field: "laz.standard_point10_chunk_seed",
                        detail: format!(
                            "failed while reading first raw {name} item of standard Point10 chunk: {err}"
                        ),
                    }
                })?);
            }
        }
    }

    let mut out = Vec::with_capacity(point_count);
    out.push(assemble_point_record(
        state,
        extra_bytes.as_deref(),
        point_data_format,
        expected_extra_bytes_count,
        scales,
        offsets,
    ));

    if point_count == 1 {
        return Ok(out);
    }

    let mut ad = ArithmeticDecoder::new(&mut cursor);
    ad.read_init_bytes()?;

    for point_index in 1..point_count {
        for d in &mut decoders {
            let name = d.name();
            match d {
                ItemDecoder::Point10(p) => {
                    state.point10 = p.decompress_with(&mut ad).map_err(|err| {
                        crate::Error::InvalidValue {
                            field: "laz.standard_point10_chunk_decode",
                            detail: format!(
                                "failed while decoding point {} {name} item in standard Point10 chunk ({} compressed bytes, {} points): {err}",
                                point_index,
                                chunk_bytes.len(),
                                point_count,
                            ),
                        }
                    })?;
                }
                ItemDecoder::Gps(g) => {
                    state.gps_bits = Some(g.decompress_with(&mut ad).map_err(|err| {
                        crate::Error::InvalidValue {
                            field: "laz.standard_point10_chunk_decode",
                            detail: format!(
                                "failed while decoding point {} {name} item in standard Point10 chunk ({} compressed bytes, {} points): {err}",
                                point_index,
                                chunk_bytes.len(),
                                point_count,
                            ),
                        }
                    })?);
                }
                ItemDecoder::Rgb(rgb) => {
                    state.rgb = Some(rgb.decompress_with(&mut ad).map_err(|err| {
                        crate::Error::InvalidValue {
                            field: "laz.standard_point10_chunk_decode",
                            detail: format!(
                                "failed while decoding point {} {name} item in standard Point10 chunk ({} compressed bytes, {} points): {err}",
                                point_index,
                                chunk_bytes.len(),
                                point_count,
                            ),
                        }
                    })?);
                }
                ItemDecoder::Extra(e) => {
                    extra_bytes = Some(e.decompress_with(&mut ad).map_err(|err| {
                        crate::Error::InvalidValue {
                            field: "laz.standard_point10_chunk_decode",
                            detail: format!(
                                "failed while decoding point {} {name} item in standard Point10 chunk ({} compressed bytes, {} points): {err}",
                                point_index,
                                chunk_bytes.len(),
                                point_count,
                            ),
                        }
                    })?);
                }
            }
        }
        out.push(assemble_point_record(
            state,
            extra_bytes.as_deref(),
            point_data_format,
            expected_extra_bytes_count,
            scales,
            offsets,
        ));
    }

    Ok(out)
}
