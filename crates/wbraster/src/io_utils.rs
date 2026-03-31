//! Low-level I/O helpers — byte-order conversion, buffered I/O.
//!
//! We implement our own byteorder helpers so we need no external crates.

use std::io::{self, Read, Write};

// ─── Byte-order primitives ────────────────────────────────────────────────────

/// Read a little-endian `f32` from a byte slice starting at `offset`.
#[inline]
pub fn read_f32_le(buf: &[u8], offset: usize) -> f32 {
    let b: [u8; 4] = buf[offset..offset + 4].try_into().unwrap();
    f32::from_le_bytes(b)
}

/// Read a big-endian `f32` from a byte slice starting at `offset`.
#[inline]
pub fn read_f32_be(buf: &[u8], offset: usize) -> f32 {
    let b: [u8; 4] = buf[offset..offset + 4].try_into().unwrap();
    f32::from_be_bytes(b)
}

/// Read a little-endian `f64` from a byte slice starting at `offset`.
#[inline]
pub fn read_f64_le(buf: &[u8], offset: usize) -> f64 {
    let b: [u8; 8] = buf[offset..offset + 8].try_into().unwrap();
    f64::from_le_bytes(b)
}

/// Read a big-endian `f64` from a byte slice starting at `offset`.
#[inline]
pub fn read_f64_be(buf: &[u8], offset: usize) -> f64 {
    let b: [u8; 8] = buf[offset..offset + 8].try_into().unwrap();
    f64::from_be_bytes(b)
}

/// Read a little-endian `i16` from a byte slice.
#[inline]
pub fn read_i16_le(buf: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes(buf[offset..offset + 2].try_into().unwrap())
}

/// Read a big-endian `i16` from a byte slice.
#[inline]
pub fn read_i16_be(buf: &[u8], offset: usize) -> i16 {
    i16::from_be_bytes(buf[offset..offset + 2].try_into().unwrap())
}

/// Read a little-endian `i32` from a byte slice.
#[inline]
pub fn read_i32_le(buf: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap())
}

/// Read a big-endian `i32` from a byte slice.
#[inline]
pub fn read_i32_be(buf: &[u8], offset: usize) -> i32 {
    i32::from_be_bytes(buf[offset..offset + 4].try_into().unwrap())
}

/// Read a little-endian `u16` from a byte slice.
#[inline]
pub fn read_u16_le(buf: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(buf[offset..offset + 2].try_into().unwrap())
}

/// Read a little-endian `u32` from a byte slice.
#[inline]
pub fn read_u32_le(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap())
}

// ─── Streaming I/O helpers ────────────────────────────────────────────────────

/// Read exactly `n` bytes from `r` into a `Vec<u8>`.
pub fn read_exact_vec(r: &mut impl Read, n: usize) -> io::Result<Vec<u8>> {
    let mut buf = vec![0u8; n];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

/// Read a single `f32` (little-endian) from a reader.
pub fn read_f32_le_stream(r: &mut impl Read) -> io::Result<f32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(f32::from_le_bytes(b))
}

/// Read a single `f32` (big-endian) from a reader.
pub fn read_f32_be_stream(r: &mut impl Read) -> io::Result<f32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(f32::from_be_bytes(b))
}

/// Read a single `f64` (little-endian) from a reader.
pub fn read_f64_le_stream(r: &mut impl Read) -> io::Result<f64> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b)?;
    Ok(f64::from_le_bytes(b))
}

/// Read a single `f64` (big-endian) from a reader.
pub fn read_f64_be_stream(r: &mut impl Read) -> io::Result<f64> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b)?;
    Ok(f64::from_be_bytes(b))
}

/// Read a single `i16` (little-endian) from a reader.
pub fn read_i16_le_stream(r: &mut impl Read) -> io::Result<i16> {
    let mut b = [0u8; 2];
    r.read_exact(&mut b)?;
    Ok(i16::from_le_bytes(b))
}

/// Read a single `i16` (big-endian) from a reader.
pub fn read_i16_be_stream(r: &mut impl Read) -> io::Result<i16> {
    let mut b = [0u8; 2];
    r.read_exact(&mut b)?;
    Ok(i16::from_be_bytes(b))
}

/// Read a single `i32` (little-endian) from a reader.
pub fn read_i32_le_stream(r: &mut impl Read) -> io::Result<i32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(i32::from_le_bytes(b))
}

/// Read a single `i32` (big-endian) from a reader.
pub fn read_i32_be_stream(r: &mut impl Read) -> io::Result<i32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(i32::from_be_bytes(b))
}

/// Write a `f32` as little-endian bytes.
pub fn write_f32_le(w: &mut impl Write, v: f32) -> io::Result<()> {
    w.write_all(&v.to_le_bytes())
}

/// Write a `f32` as big-endian bytes.
pub fn write_f32_be(w: &mut impl Write, v: f32) -> io::Result<()> {
    w.write_all(&v.to_be_bytes())
}

/// Write a `f64` as little-endian bytes.
pub fn write_f64_le(w: &mut impl Write, v: f64) -> io::Result<()> {
    w.write_all(&v.to_le_bytes())
}

/// Write a `f64` as big-endian bytes.
pub fn write_f64_be(w: &mut impl Write, v: f64) -> io::Result<()> {
    w.write_all(&v.to_be_bytes())
}

/// Write a `i16` as little-endian bytes.
pub fn write_i16_le(w: &mut impl Write, v: i16) -> io::Result<()> {
    w.write_all(&v.to_le_bytes())
}

/// Write a `i32` as little-endian bytes.
pub fn write_i32_le(w: &mut impl Write, v: i32) -> io::Result<()> {
    w.write_all(&v.to_le_bytes())
}

// ─── Text helpers ─────────────────────────────────────────────────────────────

/// Trim whitespace and strip an inline comment that starts with `//` or `#`.
pub fn strip_comment(line: &str) -> &str {
    let line = line.trim();
    // strip // comments
    let line = if let Some(pos) = line.find("//") { &line[..pos] } else { line };
    // strip # comments (but not the leading # of values like -9999)
    // Only strip if '#' is preceded by whitespace or is the first char
    let line = if let Some(pos) = line.find('#') {
        if pos == 0 || line.as_bytes()[pos - 1] == b' ' || line.as_bytes()[pos - 1] == b'\t' {
            &line[..pos]
        } else {
            line
        }
    } else {
        line
    };
    line.trim()
}

/// Parse a key/value pair from a header line like `NCOLS  100`.
/// Returns `None` if the line is empty or a comment.
pub fn parse_key_value(line: &str) -> Option<(String, String)> {
    let line = strip_comment(line);
    if line.is_empty() {
        return None;
    }
    // Split on first whitespace or '='
    let sep = line.find(|c: char| c == '=' || c.is_ascii_whitespace())?;
    let key = line[..sep].trim().to_ascii_lowercase();
    let rest = line[sep..].trim_start_matches(|c: char| c == '=' || c.is_ascii_whitespace());
    Some((key, rest.trim().to_string()))
}

// ─── Path helpers ─────────────────────────────────────────────────────────────

/// Replace or add a file extension, returning the new path as a `String`.
pub fn with_extension(path: &str, ext: &str) -> String {
    let p = std::path::Path::new(path);
    p.with_extension(ext).to_string_lossy().into_owned()
}

/// Return the lowercase file extension of `path`, or `""` if none.
pub fn extension_lower(path: &str) -> String {
    std::path::Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

// ─── Fast ASCII float writer ─────────────────────────────────────────────────

/// Write an `f64` value using `ryu`-style grisu algorithm without external crates.
/// Falls back to Rust's built-in `{:.prec$}` formatting for simplicity and correctness.
/// Uses up to `decimals` decimal places and trims trailing zeros.
pub fn format_float(v: f64, decimals: usize) -> String {
    if v.is_nan() || v.is_infinite() {
        return format!("{v}");
    }
    let s = format!("{v:.prec$}", prec = decimals);
    // Trim trailing zeros after decimal point
    if s.contains('.') {
        let s = s.trim_end_matches('0');
        let s = s.trim_end_matches('.');
        s.to_string()
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_f32_le() {
        let v: f32 = std::f32::consts::PI;
        let mut buf = Vec::new();
        write_f32_le(&mut buf, v).unwrap();
        let got = read_f32_le(&buf, 0);
        assert_eq!(v, got);
    }

    #[test]
    fn roundtrip_f64_be() {
        let v: f64 = std::f64::consts::E;
        let mut buf = Vec::new();
        write_f64_be(&mut buf, v).unwrap();
        let got = read_f64_be(&buf, 0);
        assert_eq!(v, got);
    }

    #[test]
    fn parse_kv() {
        assert_eq!(parse_key_value("NCOLS 100"), Some(("ncols".into(), "100".into())));
        assert_eq!(parse_key_value("cellsize = 0.5"), Some(("cellsize".into(), "0.5".into())));
        assert_eq!(parse_key_value("  # comment  "), None);
        assert_eq!(parse_key_value(""), None);
    }

    #[test]
    fn format_float_precision() {
        assert_eq!(format_float(1.0, 6), "1");
        assert_eq!(format_float(1.5, 6), "1.5");
        assert_eq!(format_float(-9999.0, 6), "-9999");
    }
}
