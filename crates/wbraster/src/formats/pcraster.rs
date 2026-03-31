//! PCRaster raster format (`.map`) based on CSF v2 headers.
//!
//! MVP scope:
//! - Read CSF v1/v2 raster headers and data for `CR_UINT1`, `CR_INT4`, `CR_REAL4`, `CR_REAL8`.
//! - Write single-band maps using value-scale aware output:
//!   - `VS_BOOLEAN`/`VS_LDD` -> `CR_UINT1`
//!   - `VS_NOMINAL`/`VS_ORDINAL` -> `CR_UINT1` or `CR_INT4`
//!   - `VS_SCALAR`/`VS_DIRECTION` -> `CR_REAL4` or `CR_REAL8`
//! - Writer metadata overrides:
//!   - `pcraster_valuescale`: `boolean|nominal|ordinal|scalar|direction|ldd`
//!   - `pcraster_cellrepr`: `uint1|int4|real4|real8`

use std::fs::File;
use std::io::{BufWriter, Write};

use crate::error::{RasterError, Result};
use crate::io_utils::with_extension;
use crate::raster::{DataType, Raster, RasterConfig};
use crate::crs_info::CrsInfo;

const CSF_SIG: &str = "RUU CROSS SYSTEM MAP FORMAT";
const ADDR_SECOND_HEADER: usize = 64;
const ADDR_DATA: usize = 256;

const ORD_OK: u32 = 0x0000_0001;
const ORD_SWAB: u32 = 0x0100_0000;

const T_RASTER: u16 = 1;
const CSF_VERSION_1: u16 = 1;
const CSF_VERSION_2: u16 = 2;

const PT_YDECT2B: u16 = 1;

const VS_BOOLEAN: u16 = 0x00;
const VS_NOMINAL: u16 = 0x01;
const VS_ORDINAL: u16 = 0x02;
const VS_SCALAR: u16 = 0xEB;
const VS_DIRECTION: u16 = 0xEE;
const VS_LDD: u16 = 0xF2;

const CR_UINT1: u16 = 0x00;
const CR_INT4: u16 = 0x26;
const CR_REAL4: u16 = 0x5A;
const CR_REAL8: u16 = 0xDB;

#[derive(Clone, Copy)]
struct WriteProfile {
    value_scale: u16,
    cell_repr: u16,
}

#[derive(Clone, Copy)]
enum Encoded {
    U8(u8),
    I32(i32),
    F32(f32),
    F64(f64),
}

impl Encoded {
    fn as_f64(self) -> f64 {
        match self {
            Encoded::U8(v) => v as f64,
            Encoded::I32(v) => v as f64,
            Encoded::F32(v) => v as f64,
            Encoded::F64(v) => v,
        }
    }
}

/// Returns `true` if `path` appears to be a PCRaster CSF file.
pub fn is_pcraster_file(path: &str) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    has_csf_signature(&bytes)
}

/// Read a PCRaster `.map` raster.
pub fn read(path: &str) -> Result<Raster> {
    let bytes = std::fs::read(path)?;
    if bytes.len() < ADDR_DATA {
        return Err(RasterError::CorruptData(
            "PCRaster file too small for CSF headers".into(),
        ));
    }
    if !has_csf_signature(&bytes) {
        return Err(RasterError::CorruptData(
            "invalid PCRaster CSF signature".into(),
        ));
    }

    let byte_order_raw = u32::from_le_bytes(bytes[46..50].try_into().unwrap());
    let little_endian_file = match byte_order_raw {
        ORD_OK => true,
        ORD_SWAB => false,
        _ => {
            return Err(RasterError::CorruptData(format!(
                "unsupported PCRaster byte order marker: 0x{byte_order_raw:08X}"
            )));
        }
    };

    let mut off = 0usize;
    off += 32; // signature space
    let version = read_u16(&bytes, &mut off, little_endian_file)?;
    if version != CSF_VERSION_1 && version != CSF_VERSION_2 {
        return Err(RasterError::CorruptData(format!(
            "unsupported PCRaster version: {version}"
        )));
    }
    let _gis_file_id = read_u32(&bytes, &mut off, little_endian_file)?;
    let _projection = read_u16(&bytes, &mut off, little_endian_file)?;
    let _attr_table = read_u32(&bytes, &mut off, little_endian_file)?;
    let map_type = read_u16(&bytes, &mut off, little_endian_file)?;
    let _byte_order = read_u32(&bytes, &mut off, little_endian_file)?;
    if map_type != T_RASTER {
        return Err(RasterError::CorruptData(format!(
            "unsupported PCRaster map type: {map_type}"
        )));
    }

    off = ADDR_SECOND_HEADER;
    let value_scale = read_u16(&bytes, &mut off, little_endian_file)?;
    let cell_repr = read_u16(&bytes, &mut off, little_endian_file)?;

    let _min_val = read_f64(&bytes, &mut off, little_endian_file)?;
    let _max_val = read_f64(&bytes, &mut off, little_endian_file)?;
    let x_ul = read_f64(&bytes, &mut off, little_endian_file)?;
    let y_ul = read_f64(&bytes, &mut off, little_endian_file)?;
    let rows = read_u32(&bytes, &mut off, little_endian_file)? as usize;
    let cols = read_u32(&bytes, &mut off, little_endian_file)? as usize;
    let cell_size = read_f64(&bytes, &mut off, little_endian_file)?;
    let _cell_size_dupl = read_f64(&bytes, &mut off, little_endian_file)?;
    let _angle = if version == CSF_VERSION_2 {
        read_f64(&bytes, &mut off, little_endian_file)?
    } else {
        0.0
    };

    if rows == 0 || cols == 0 {
        return Err(RasterError::InvalidDimensions { cols, rows });
    }
    if cell_size <= 0.0 {
        return Err(RasterError::CorruptData(format!(
            "invalid PCRaster cell size: {cell_size}"
        )));
    }

    let nodata = nodata_for_cell_repr(cell_repr)?;
    let data_type = match cell_repr {
        CR_UINT1 => DataType::U8,
        CR_INT4 => DataType::I32,
        CR_REAL4 => DataType::F32,
        CR_REAL8 => DataType::F64,
        other => {
            return Err(RasterError::UnsupportedDataType(format!(
                "PCRaster cell representation not supported in MVP: 0x{other:02X}"
            )));
        }
    };

    let cell_bytes = match cell_repr {
        CR_UINT1 => 1,
        CR_INT4 | CR_REAL4 => 4,
        CR_REAL8 => 8,
        _ => unreachable!(),
    };

    let n = rows
        .checked_mul(cols)
        .ok_or_else(|| RasterError::CorruptData("rows*cols overflow".into()))?;
    let data_size = n
        .checked_mul(cell_bytes)
        .ok_or_else(|| RasterError::CorruptData("data byte size overflow".into()))?;
    if bytes.len() < ADDR_DATA + data_size {
        return Err(RasterError::CorruptData(format!(
            "PCRaster file truncated: need at least {} bytes, got {}",
            ADDR_DATA + data_size,
            bytes.len()
        )));
    }

    let mut p = ADDR_DATA;
    let mut data = Vec::with_capacity(n);
    for _row in 0..rows {
        for _col in 0..cols {
            let v = match cell_repr {
                CR_UINT1 => {
                    let raw = bytes[p];
                    p += 1;
                    if raw == 0xFF { nodata } else { raw as f64 }
                }
                CR_INT4 => {
                    let raw = read_i32_at(&bytes, p, little_endian_file)?;
                    p += 4;
                    if raw == i32::MIN { nodata } else { raw as f64 }
                }
                CR_REAL4 => {
                    let raw_bits = read_u32_at(&bytes, p, little_endian_file)?;
                    p += 4;
                    if raw_bits == u32::MAX {
                        nodata
                    } else {
                        let f = f32::from_bits(raw_bits);
                        if f.is_nan() { nodata } else { f as f64 }
                    }
                }
                CR_REAL8 => {
                    let raw_bits = read_u64_at(&bytes, p, little_endian_file)?;
                    p += 8;
                    if raw_bits == u64::MAX {
                        nodata
                    } else {
                        let f = f64::from_bits(raw_bits);
                        if f.is_nan() { nodata } else { f }
                    }
                }
                _ => unreachable!(),
            };
            data.push(v);
        }
    }

    let y_min = y_ul - rows as f64 * cell_size;
    let mut metadata = Vec::new();
    if let Some(vs_name) = value_scale_name(value_scale) {
        metadata.push(("pcraster_valuescale".to_string(), vs_name.to_string()));
    }
    if let Some(cr_name) = cell_repr_name(cell_repr) {
        metadata.push(("pcraster_cellrepr".to_string(), cr_name.to_string()));
    }
    let prj_text = read_prj_sidecar(path);
    if let Some(ref text) = prj_text {
        metadata.push(("pcraster_prj_text".to_string(), text.clone()));
    }
    let crs = if let Some(text) = prj_text {
        if wkt_like(&text) {
            CrsInfo::from_wkt(text)
        } else {
            CrsInfo::default()
        }
    } else {
        CrsInfo::default()
    };

    let cfg = RasterConfig {
        cols,
        rows,
        x_min: x_ul,
        y_min,
        cell_size,
        cell_size_y: Some(cell_size),
        nodata,
        data_type,
        crs: crs,        metadata,
        ..Default::default()
    };
    Raster::from_data(cfg, data)
}

/// Write PCRaster map using value-scale aware cell representations.
pub fn write(raster: &Raster, path: &str) -> Result<()> {
    if raster.bands != 1 {
        return Err(RasterError::UnsupportedDataType(
            "PCRaster writer currently supports single-band rasters only".into(),
        ));
    }

    let profile = choose_write_profile(raster)?;
    validate_write_profile(profile)?;

    let f = File::create(path)?;
    let mut w = BufWriter::with_capacity(256 * 1024, f);

    let nodata = nodata_encoded(profile.cell_repr)?;
    let mut min_val: Option<Encoded> = None;
    let mut max_val: Option<Encoded> = None;

    for v in raster.data.iter_f64() {
        if let Some(encoded) = encode_value(v, raster, profile)? {
            match min_val {
                Some(cur) if encoded.as_f64() >= cur.as_f64() => {}
                _ => min_val = Some(encoded),
            }
            match max_val {
                Some(cur) if encoded.as_f64() <= cur.as_f64() => {}
                _ => max_val = Some(encoded),
            }
        }
    }
    let min_hdr = min_val.unwrap_or(nodata);
    let max_hdr = max_val.unwrap_or(nodata);

    let y_ul = raster.y_max();

    let mut sig = [0u8; 32];
    let sig_bytes = CSF_SIG.as_bytes();
    sig[..sig_bytes.len()].copy_from_slice(sig_bytes);
    w.write_all(&sig)?;

    w.write_all(&CSF_VERSION_2.to_le_bytes())?;
    w.write_all(&0u32.to_le_bytes())?; // gisFileId
    w.write_all(&PT_YDECT2B.to_le_bytes())?;
    w.write_all(&0u32.to_le_bytes())?; // attrTable
    w.write_all(&T_RASTER.to_le_bytes())?;
    w.write_all(&ORD_OK.to_le_bytes())?;
    w.write_all(&[0u8; 14])?;

    w.write_all(&profile.value_scale.to_le_bytes())?;
    w.write_all(&profile.cell_repr.to_le_bytes())?;
    write_var_type(&mut w, min_hdr)?;
    write_var_type(&mut w, max_hdr)?;
    w.write_all(&raster.x_min.to_le_bytes())?;
    w.write_all(&y_ul.to_le_bytes())?;
    w.write_all(&(raster.rows as u32).to_le_bytes())?;
    w.write_all(&(raster.cols as u32).to_le_bytes())?;
    w.write_all(&raster.cell_size_x.to_le_bytes())?;
    w.write_all(&raster.cell_size_x.to_le_bytes())?;
    w.write_all(&0.0f64.to_le_bytes())?; // angle
    w.write_all(&[0u8; 124])?;

    for row in 0..raster.rows {
        let slice = raster.row_slice(0, row as isize);
        for v in slice {
            let out = encode_value(v, raster, profile)?.unwrap_or(nodata);
            write_cell(&mut w, out)?;
        }
    }

    w.flush()?;
    write_prj_sidecar(raster, path)
}

fn read_prj_sidecar(path: &str) -> Option<String> {
    let prj_path = with_extension(path, "prj");
    std::fs::read_to_string(prj_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn write_prj_sidecar(raster: &Raster, path: &str) -> Result<()> {
    let prj_text = raster
        .metadata
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("pcraster_prj_text"))
        .map(|(_, v)| v.as_str())
        .or(raster.crs.wkt.as_deref());

    if let Some(text) = prj_text {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            let prj_path = with_extension(path, "prj");
            std::fs::write(prj_path, trimmed)?;
        }
    }
    Ok(())
}

fn wkt_like(s: &str) -> bool {
    let t = s.trim();
    let upper = t.to_ascii_uppercase();
    !t.is_empty()
        && (upper.starts_with("GEOGCS[")
            || upper.starts_with("PROJCS[")
            || upper.starts_with("COMPOUNDCRS[")
            || upper.starts_with("GEODCRS[")
            || upper.starts_with("PROJCRS[")
            || upper.starts_with("VERTCRS["))
}

fn has_csf_signature(bytes: &[u8]) -> bool {
    if bytes.len() < CSF_SIG.len() {
        return false;
    }
    &bytes[..CSF_SIG.len()] == CSF_SIG.as_bytes()
}

fn choose_write_profile(raster: &Raster) -> Result<WriteProfile> {
    let value_scale = match metadata_lookup(raster, "pcraster_valuescale") {
        Some(v) => parse_value_scale(v)?,
        None => match raster.data_type {
            DataType::F32 | DataType::F64 => VS_SCALAR,
            _ => VS_NOMINAL,
        },
    };

    let cell_repr = match metadata_lookup(raster, "pcraster_cellrepr") {
        Some(v) => parse_cell_repr(v)?,
        None => match value_scale {
            VS_BOOLEAN | VS_LDD => CR_UINT1,
            VS_NOMINAL | VS_ORDINAL => match raster.data_type {
                DataType::U8 => CR_UINT1,
                _ => CR_INT4,
            },
            VS_SCALAR | VS_DIRECTION => match raster.data_type {
                DataType::F32 => CR_REAL4,
                _ => CR_REAL8,
            },
            _ => {
                return Err(RasterError::UnsupportedDataType(format!(
                    "unsupported PCRaster value scale: 0x{value_scale:02X}"
                )));
            }
        },
    };

    Ok(WriteProfile {
        value_scale,
        cell_repr,
    })
}

fn validate_write_profile(profile: WriteProfile) -> Result<()> {
    match (profile.value_scale, profile.cell_repr) {
        (VS_BOOLEAN, CR_UINT1)
        | (VS_LDD, CR_UINT1)
        | (VS_NOMINAL | VS_ORDINAL, CR_UINT1 | CR_INT4)
        | (VS_SCALAR | VS_DIRECTION, CR_REAL4 | CR_REAL8) => Ok(()),
        _ => Err(RasterError::UnsupportedDataType(format!(
            "unsupported PCRaster writer profile: value_scale={} cell_repr={}",
            value_scale_name(profile.value_scale).unwrap_or("unknown"),
            cell_repr_name(profile.cell_repr).unwrap_or("unknown")
        ))),
    }
}

fn encode_value(v: f64, raster: &Raster, profile: WriteProfile) -> Result<Option<Encoded>> {
    if raster.is_nodata(v) {
        return Ok(None);
    }
    if !v.is_finite() {
        return Err(RasterError::CorruptData(
            "PCRaster writer encountered non-finite valid value".into(),
        ));
    }

    let encoded = match profile.cell_repr {
        CR_UINT1 => {
            let iv = require_integer(v, "PCRaster UINT1")?;
            if profile.value_scale == VS_BOOLEAN && !(iv == 0 || iv == 1) {
                return Err(RasterError::CorruptData(format!(
                    "PCRaster boolean expects values in {{0,1}}; got {iv}"
                )));
            }
            if profile.value_scale == VS_LDD && !(1..=9).contains(&iv) {
                return Err(RasterError::CorruptData(format!(
                    "PCRaster LDD expects values in [1,9]; got {iv}"
                )));
            }
            if !(0..=254).contains(&iv) {
                return Err(RasterError::CorruptData(format!(
                    "PCRaster UINT1 valid range is [0,254] (255 reserved nodata); got {iv}"
                )));
            }
            Encoded::U8(iv as u8)
        }
        CR_INT4 => {
            let iv = require_integer(v, "PCRaster INT4")?;
            if !(i32::MIN as i64 + 1..=i32::MAX as i64).contains(&iv) {
                return Err(RasterError::CorruptData(format!(
                    "PCRaster INT4 valid range is [{},{}] ({} reserved nodata); got {iv}",
                    i32::MIN as i64 + 1,
                    i32::MAX,
                    i32::MIN
                )));
            }
            Encoded::I32(iv as i32)
        }
        CR_REAL4 => {
            let fv = v as f32;
            if !fv.is_finite() {
                return Err(RasterError::CorruptData(format!(
                    "PCRaster REAL4 cannot represent value {v}"
                )));
            }
            Encoded::F32(fv)
        }
        CR_REAL8 => Encoded::F64(v),
        other => {
            return Err(RasterError::UnsupportedDataType(format!(
                "unsupported PCRaster cell representation: 0x{other:02X}"
            )));
        }
    };

    Ok(Some(encoded))
}

fn require_integer(v: f64, label: &str) -> Result<i64> {
    let rounded = v.round();
    if (v - rounded).abs() > 1e-9 {
        return Err(RasterError::CorruptData(format!(
            "{label} requires integer values; got {v}"
        )));
    }
    if rounded < i64::MIN as f64 || rounded > i64::MAX as f64 {
        return Err(RasterError::CorruptData(format!(
            "{label} value out of i64 range: {v}"
        )));
    }
    Ok(rounded as i64)
}

fn write_var_type<W: Write>(w: &mut W, v: Encoded) -> Result<()> {
    let mut buf = [0u8; 8];
    match v {
        Encoded::U8(x) => {
            buf[0] = x;
        }
        Encoded::I32(x) => {
            buf[..4].copy_from_slice(&x.to_le_bytes());
        }
        Encoded::F32(x) => {
            buf[..4].copy_from_slice(&x.to_le_bytes());
        }
        Encoded::F64(x) => {
            buf.copy_from_slice(&x.to_le_bytes());
        }
    }
    w.write_all(&buf)?;
    Ok(())
}

fn write_cell<W: Write>(w: &mut W, v: Encoded) -> Result<()> {
    match v {
        Encoded::U8(x) => w.write_all(&[x])?,
        Encoded::I32(x) => w.write_all(&x.to_le_bytes())?,
        Encoded::F32(x) => w.write_all(&x.to_le_bytes())?,
        Encoded::F64(x) => w.write_all(&x.to_le_bytes())?,
    }
    Ok(())
}

fn nodata_encoded(cell_repr: u16) -> Result<Encoded> {
    match cell_repr {
        CR_UINT1 => Ok(Encoded::U8(255)),
        CR_INT4 => Ok(Encoded::I32(i32::MIN)),
        CR_REAL4 => Ok(Encoded::F32(f32::from_bits(u32::MAX))),
        CR_REAL8 => Ok(Encoded::F64(f64::from_bits(u64::MAX))),
        other => Err(RasterError::UnsupportedDataType(format!(
            "unsupported PCRaster cell representation: 0x{other:02X}"
        ))),
    }
}

fn metadata_lookup<'a>(raster: &'a Raster, key: &str) -> Option<&'a str> {
    raster
        .metadata
        .iter()
        .find_map(|(k, v)| k.eq_ignore_ascii_case(key).then_some(v.trim()))
}

fn parse_value_scale(s: &str) -> Result<u16> {
    match s.trim().to_ascii_lowercase().as_str() {
        "boolean" | "vs_boolean" => Ok(VS_BOOLEAN),
        "nominal" | "vs_nominal" => Ok(VS_NOMINAL),
        "ordinal" | "vs_ordinal" => Ok(VS_ORDINAL),
        "scalar" | "vs_scalar" => Ok(VS_SCALAR),
        "direction" | "vs_direction" => Ok(VS_DIRECTION),
        "ldd" | "vs_ldd" => Ok(VS_LDD),
        other => Err(RasterError::UnsupportedDataType(format!(
            "unsupported PCRaster value scale metadata '{other}'"
        ))),
    }
}

fn parse_cell_repr(s: &str) -> Result<u16> {
    match s.trim().to_ascii_lowercase().as_str() {
        "uint1" | "cr_uint1" | "u8" | "byte" => Ok(CR_UINT1),
        "int4" | "cr_int4" | "i32" => Ok(CR_INT4),
        "real4" | "cr_real4" | "f32" | "float32" => Ok(CR_REAL4),
        "real8" | "cr_real8" | "f64" | "float64" => Ok(CR_REAL8),
        other => Err(RasterError::UnsupportedDataType(format!(
            "unsupported PCRaster cell representation metadata '{other}'"
        ))),
    }
}

fn value_scale_name(value_scale: u16) -> Option<&'static str> {
    match value_scale {
        VS_BOOLEAN => Some("boolean"),
        VS_NOMINAL => Some("nominal"),
        VS_ORDINAL => Some("ordinal"),
        VS_SCALAR => Some("scalar"),
        VS_DIRECTION => Some("direction"),
        VS_LDD => Some("ldd"),
        _ => None,
    }
}

fn cell_repr_name(cell_repr: u16) -> Option<&'static str> {
    match cell_repr {
        CR_UINT1 => Some("uint1"),
        CR_INT4 => Some("int4"),
        CR_REAL4 => Some("real4"),
        CR_REAL8 => Some("real8"),
        _ => None,
    }
}

fn nodata_for_cell_repr(cell_repr: u16) -> Result<f64> {
    match cell_repr {
        CR_UINT1 => Ok(255.0),
        CR_INT4 => Ok(i32::MIN as f64),
        CR_REAL4 => Ok(f32::from_bits(u32::MAX) as f64),
        CR_REAL8 => Ok(f64::from_bits(u64::MAX)),
        other => Err(RasterError::UnsupportedDataType(format!(
            "unsupported PCRaster cell representation: 0x{other:02X}"
        ))),
    }
}

fn read_u16(buf: &[u8], off: &mut usize, le: bool) -> Result<u16> {
    if *off + 2 > buf.len() {
        return Err(RasterError::CorruptData("unexpected EOF reading u16".into()));
    }
    let b: [u8; 2] = buf[*off..*off + 2].try_into().unwrap();
    *off += 2;
    Ok(if le { u16::from_le_bytes(b) } else { u16::from_be_bytes(b) })
}

fn read_u32(buf: &[u8], off: &mut usize, le: bool) -> Result<u32> {
    if *off + 4 > buf.len() {
        return Err(RasterError::CorruptData("unexpected EOF reading u32".into()));
    }
    let b: [u8; 4] = buf[*off..*off + 4].try_into().unwrap();
    *off += 4;
    Ok(if le { u32::from_le_bytes(b) } else { u32::from_be_bytes(b) })
}

fn read_f64(buf: &[u8], off: &mut usize, le: bool) -> Result<f64> {
    if *off + 8 > buf.len() {
        return Err(RasterError::CorruptData("unexpected EOF reading f64".into()));
    }
    let b: [u8; 8] = buf[*off..*off + 8].try_into().unwrap();
    *off += 8;
    Ok(if le { f64::from_le_bytes(b) } else { f64::from_be_bytes(b) })
}

fn read_u32_at(buf: &[u8], at: usize, le: bool) -> Result<u32> {
    if at + 4 > buf.len() {
        return Err(RasterError::CorruptData("unexpected EOF reading u32".into()));
    }
    let b: [u8; 4] = buf[at..at + 4].try_into().unwrap();
    Ok(if le { u32::from_le_bytes(b) } else { u32::from_be_bytes(b) })
}

fn read_u64_at(buf: &[u8], at: usize, le: bool) -> Result<u64> {
    if at + 8 > buf.len() {
        return Err(RasterError::CorruptData("unexpected EOF reading u64".into()));
    }
    let b: [u8; 8] = buf[at..at + 8].try_into().unwrap();
    Ok(if le { u64::from_le_bytes(b) } else { u64::from_be_bytes(b) })
}

fn read_i32_at(buf: &[u8], at: usize, le: bool) -> Result<i32> {
    if at + 4 > buf.len() {
        return Err(RasterError::CorruptData("unexpected EOF reading i32".into()));
    }
    let b: [u8; 4] = buf[at..at + 4].try_into().unwrap();
    Ok(if le { i32::from_le_bytes(b) } else { i32::from_be_bytes(b) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn signature_check() {
        let mut b = vec![0u8; 64];
        b[..CSF_SIG.len()].copy_from_slice(CSF_SIG.as_bytes());
        assert!(has_csf_signature(&b));
    }

    #[test]
    fn roundtrip_real8_mvp() {
        let cfg = RasterConfig {
            cols: 3,
            rows: 2,
            x_min: 10.0,
            y_min: 20.0,
            cell_size: 2.0,
            nodata: -9999.0,
            data_type: DataType::F64,
            ..Default::default()
        };
        let data = vec![1.0, 2.0, -9999.0, 4.0, 5.0, 6.0];
        let r = Raster::from_data(cfg, data).unwrap();

        let path = std::env::temp_dir()
            .join("wbraster_pcraster_unit_test.map")
            .to_string_lossy()
            .into_owned();
        write(&r, &path).unwrap();
        let r2 = read(&path).unwrap();

        assert_eq!(r2.cols, 3);
        assert_eq!(r2.rows, 2);
        assert_eq!(r2.get(0, 0, 0), 1.0);
        assert!(r2.is_nodata(r2.get(0, 2, 0)));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn roundtrip_nominal_uint1() {
        let cfg = RasterConfig {
            cols: 3,
            rows: 2,
            x_min: 10.0,
            y_min: 20.0,
            cell_size: 2.0,
            nodata: 255.0,
            data_type: DataType::U8,
            metadata: vec![
                ("pcraster_valuescale".into(), "nominal".into()),
                ("pcraster_cellrepr".into(), "uint1".into()),
            ],
            ..Default::default()
        };
        let data = vec![1.0, 2.0, 255.0, 4.0, 5.0, 6.0];
        let r = Raster::from_data(cfg, data).unwrap();

        let path = std::env::temp_dir()
            .join("wbraster_pcraster_uint1_unit_test.map")
            .to_string_lossy()
            .into_owned();
        write(&r, &path).unwrap();
        let r2 = read(&path).unwrap();

        assert_eq!(r2.data_type, DataType::U8);
        assert_eq!(r2.get(0, 0, 0), 1.0);
        assert!(r2.is_nodata(r2.get(0, 2, 0)));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn boolean_rejects_non_binary_values() {
        let cfg = RasterConfig {
            cols: 2,
            rows: 2,
            nodata: 255.0,
            data_type: DataType::U8,
            metadata: vec![("pcraster_valuescale".into(), "boolean".into())],
            ..Default::default()
        };
        let data = vec![0.0, 1.0, 2.0, 255.0];
        let r = Raster::from_data(cfg, data).unwrap();
        let path = std::env::temp_dir()
            .join("wbraster_pcraster_boolean_invalid_unit_test.map")
            .to_string_lossy()
            .into_owned();

        let e = write(&r, &path).unwrap_err();
        assert!(e.to_string().contains("boolean expects values in {0,1}"));
    }

    #[test]
    fn pcraster_writes_and_reads_prj_sidecar() {
        let cfg = RasterConfig {
            cols: 2,
            rows: 2,
            x_min: 10.0,
            y_min: 20.0,
            cell_size: 2.0,
            nodata: -9999.0,
            data_type: DataType::F64,
            ..Default::default()
        };
        let mut r = Raster::from_data(cfg, vec![1.0, 2.0, 3.0, 4.0]).unwrap();
        let wkt = "GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\"]]";
        r.crs = CrsInfo::from_wkt(wkt);

        let path = std::env::temp_dir()
            .join("wbraster_pcraster_prj_unit_test.map")
            .to_string_lossy()
            .into_owned();
        write(&r, &path).unwrap();

        let prj = with_extension(&path, "prj");
        let txt = std::fs::read_to_string(&prj).unwrap();
        assert_eq!(txt.trim(), wkt);

        let r2 = read(&path).unwrap();
        assert_eq!(r2.crs.wkt.as_deref(), Some(wkt));
        assert!(r2
            .metadata
            .iter()
            .any(|(k, v)| k == "pcraster_prj_text" && v.trim() == wkt));

        let _ = std::fs::remove_file(&path);
        if Path::new(&prj).exists() {
            let _ = std::fs::remove_file(&prj);
        }
    }
}
