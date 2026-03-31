//! PLY reader — supports ASCII and both binary encodings.

use std::io::{BufRead, BufReader, Read, Seek};
use crate::io::PointReader;
use crate::ply::PlyEncoding;
use crate::point::{PointRecord, Rgb16};
use crate::{Error, Result};

/// Property descriptor parsed from the PLY header.
#[derive(Debug, Clone)]
struct Prop {
    name: String,
    dtype: PropType,
}

#[derive(Debug, Clone, Copy)]
enum PropType {
    Char, UChar, Short, UShort, Int, UInt, Float, Double,
}

impl PropType {
    fn byte_size(self) -> usize {
        match self {
            PropType::Char | PropType::UChar => 1,
            PropType::Short | PropType::UShort => 2,
            PropType::Int | PropType::UInt | PropType::Float => 4,
            PropType::Double => 8,
        }
    }
}

fn parse_type(s: &str) -> Option<PropType> {
    match s {
        "char" | "int8"   => Some(PropType::Char),
        "uchar"| "uint8"  => Some(PropType::UChar),
        "short"| "int16"  => Some(PropType::Short),
        "ushort"|"uint16" => Some(PropType::UShort),
        "int"  | "int32"  => Some(PropType::Int),
        "uint" | "uint32" => Some(PropType::UInt),
        "float"| "float32"=> Some(PropType::Float),
        "double"|"float64"=> Some(PropType::Double),
        _ => None,
    }
}

/// A PLY reader.
pub struct PlyReader<R: Read + Seek> {
    inner: BufReader<R>,
    encoding: PlyEncoding,
    props: Vec<Prop>,
    point_count: u64,
    read_count: u64,
    _record_size: usize, // only meaningful for binary formats
}

impl<R: Read + Seek> PlyReader<R> {
    /// Open and parse the PLY header.
    pub fn new(inner: R) -> Result<Self> {
        let mut reader = BufReader::new(inner);
        let (encoding, props, point_count) = parse_header(&mut reader)?;
        let record_size = props.iter().map(|p| p.dtype.byte_size()).sum();
        Ok(PlyReader { inner: reader, encoding, props, point_count, read_count: 0, _record_size: record_size })
    }
}

impl<R: Read + Seek> PointReader for PlyReader<R> {
    fn read_point(&mut self, out: &mut PointRecord) -> Result<bool> {
        if self.read_count >= self.point_count { return Ok(false); }
        *out = PointRecord::default();

        match self.encoding {
            PlyEncoding::Ascii => read_ascii_point(&mut self.inner, &self.props, out)?,
            PlyEncoding::BinaryLittleEndian => {
                read_binary_point(&mut self.inner, &self.props, out, false)?;
            }
            PlyEncoding::BinaryBigEndian => {
                read_binary_point(&mut self.inner, &self.props, out, true)?;
            }
        }
        self.read_count += 1;
        Ok(true)
    }

    fn point_count(&self) -> Option<u64> { Some(self.point_count) }
}

// ── Header parsing ────────────────────────────────────────────────────────────

fn parse_header<R: Read>(r: &mut BufReader<R>) -> Result<(PlyEncoding, Vec<Prop>, u64)> {
    let mut line = String::new();
    r.read_line(&mut line)?;
    if !line.starts_with("ply") {
        return Err(Error::InvalidSignature { format: "PLY", found: line.into_bytes() });
    }

    let mut encoding = PlyEncoding::Ascii;
    let mut props = Vec::new();
    let mut point_count = 0u64;
    let mut in_vertex = false;

    loop {
        line.clear();
        r.read_line(&mut line)?;
        let trimmed = line.trim();
        if trimmed == "end_header" { break; }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        match parts.as_slice() {
            ["format", fmt, _ver] => {
                encoding = match *fmt {
                    "ascii"                      => PlyEncoding::Ascii,
                    "binary_little_endian"       => PlyEncoding::BinaryLittleEndian,
                    "binary_big_endian"          => PlyEncoding::BinaryBigEndian,
                    other => return Err(Error::InvalidValue {
                        field: "ply_format",
                        detail: format!("unknown encoding: {other}"),
                    }),
                };
            }
            ["element", "vertex", count] => {
                in_vertex = true;
                point_count = count.parse().unwrap_or(0);
            }
            ["element", _, _] => { in_vertex = false; }
            ["property", dtype, name] if in_vertex => {
                if let Some(t) = parse_type(dtype) {
                    props.push(Prop { name: name.to_string(), dtype: t });
                }
            }
            _ => {}
        }
    }
    Ok((encoding, props, point_count))
}

// ── ASCII point reader ────────────────────────────────────────────────────────

fn read_ascii_point<R: Read>(
    r: &mut BufReader<R>, props: &[Prop], out: &mut PointRecord,
) -> Result<()> {
    let mut line = String::new();
    r.read_line(&mut line)?;
    let tokens: Vec<&str> = line.split_whitespace().collect();
    for (i, prop) in props.iter().enumerate() {
        if i >= tokens.len() { break; }
        let val: f64 = tokens[i].parse().unwrap_or(0.0);
        apply_prop(prop, val, out);
    }
    Ok(())
}

// ── Binary point reader ───────────────────────────────────────────────────────

fn read_binary_point<R: Read>(
    r: &mut R, props: &[Prop], out: &mut PointRecord, big_endian: bool,
) -> Result<()> {
    for prop in props {
        let val = read_scalar(r, prop.dtype, big_endian)?;
        apply_prop(prop, val, out);
    }
    Ok(())
}

fn read_scalar<R: Read>(r: &mut R, dtype: PropType, big: bool) -> Result<f64> {
    let mut b8 = [0u8; 8];
    let n = dtype.byte_size();
    r.read_exact(&mut b8[..n])?;
    let v = match (dtype, big) {
        (PropType::Char,  _)     => b8[0] as i8 as f64,
        (PropType::UChar, _)     => b8[0] as f64,
        (PropType::Short, false) => i16::from_le_bytes(b8[..2].try_into().unwrap()) as f64,
        (PropType::Short, true)  => i16::from_be_bytes(b8[..2].try_into().unwrap()) as f64,
        (PropType::UShort,false) => u16::from_le_bytes(b8[..2].try_into().unwrap()) as f64,
        (PropType::UShort,true)  => u16::from_be_bytes(b8[..2].try_into().unwrap()) as f64,
        (PropType::Int,   false) => i32::from_le_bytes(b8[..4].try_into().unwrap()) as f64,
        (PropType::Int,   true)  => i32::from_be_bytes(b8[..4].try_into().unwrap()) as f64,
        (PropType::UInt,  false) => u32::from_le_bytes(b8[..4].try_into().unwrap()) as f64,
        (PropType::UInt,  true)  => u32::from_be_bytes(b8[..4].try_into().unwrap()) as f64,
        (PropType::Float, false) => f32::from_le_bytes(b8[..4].try_into().unwrap()) as f64,
        (PropType::Float, true)  => f32::from_be_bytes(b8[..4].try_into().unwrap()) as f64,
        (PropType::Double,false) => f64::from_le_bytes(b8[..8].try_into().unwrap()),
        (PropType::Double,true)  => f64::from_be_bytes(b8[..8].try_into().unwrap()),
    };
    Ok(v)
}

fn apply_prop(prop: &Prop, val: f64, out: &mut PointRecord) {
    match prop.name.as_str() {
        "x"         => out.x = val,
        "y"         => out.y = val,
        "z"         => out.z = val,
        "intensity" | "scalar_Intensity" => out.intensity = val.round() as u16,
        "red"   | "r" | "diffuse_red"   => {
            let c = out.color.get_or_insert(Rgb16::default());
            c.red = (val as u8 as u16) << 8;
        }
        "green" | "g" | "diffuse_green" => {
            let c = out.color.get_or_insert(Rgb16::default());
            c.green = (val as u8 as u16) << 8;
        }
        "blue"  | "b" | "diffuse_blue"  => {
            let c = out.color.get_or_insert(Rgb16::default());
            c.blue = (val as u8 as u16) << 8;
        }
        "nx" | "normal_x" => out.normal_x = Some(val as f32),
        "ny" | "normal_y" => out.normal_y = Some(val as f32),
        "nz" | "normal_z" => out.normal_z = Some(val as f32),
        "classification"  => out.classification = val as u8,
        _ => {}
    }
}
