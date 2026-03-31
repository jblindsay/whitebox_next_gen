//! Core I/O traits implemented by every format reader and writer.

use crate::{PointRecord, Result};

/// Trait implemented by all format readers.
///
/// Implementors read point data sequentially.  For random-access formats
/// (COPC, E57 with spatial index) see [`SeekableReader`].
pub trait PointReader {
    /// Read the next point into `out`, returning `true` if a point was
    /// available or `false` at end of file.
    ///
    /// # Errors
    /// Returns `Err` if an I/O or decode error occurs.
    fn read_point(&mut self, out: &mut PointRecord) -> Result<bool>;

    /// Total number of points declared in the file header.
    /// Returns `None` if the format does not encode a point count.
    fn point_count(&self) -> Option<u64>;

    /// Read all remaining points into a freshly-allocated `Vec`.
    ///
    /// Pre-allocates based on `point_count()` when available.
    fn read_all(&mut self) -> Result<Vec<PointRecord>> {
        let mut points = Vec::new();
        if let Some(total) = self.point_count() {
            if let Ok(hint) = usize::try_from(total) {
                // Point counts come from file headers and may be malformed.
                // Use fallible reservation so absurd values cannot abort the process.
                let _ = points.try_reserve(hint);
            }
        }
        let mut p = PointRecord::default();
        while self.read_point(&mut p)? {
            points.push(p);
        }
        Ok(points)
    }
}

/// Extension of [`PointReader`] for formats that support seeking to an
/// arbitrary point index (COPC, E57).
pub trait SeekableReader: PointReader {
    /// Seek to the given zero-based point index.
    ///
    /// # Errors
    /// Returns `Err` if `index` is out of range or the seek fails.
    fn seek_to_point(&mut self, index: u64) -> Result<()>;
}

/// Trait implemented by all format writers.
pub trait PointWriter {
    /// Write a single point record.
    ///
    /// # Errors
    /// Returns `Err` on any I/O or encode error.
    fn write_point(&mut self, point: &PointRecord) -> Result<()>;

    /// Flush all buffered data and write any finalisation structures
    /// (e.g. LAS header point-count back-patch, E57 checksum blocks).
    ///
    /// # Errors
    /// Returns `Err` if the flush or finalisation fails.
    fn finish(&mut self) -> Result<()>;

    /// Write a slice of points in one call.  Default implementation loops
    /// over `write_point`; formats may override for bulk performance.
    ///
    /// # Errors
    /// Returns `Err` on the first encode/I/O error encountered.
    fn write_all_points(&mut self, points: &[PointRecord]) -> Result<()> {
        for p in points {
            self.write_point(p)?;
        }
        Ok(())
    }
}

/// Little-endian primitive helpers — zero-dependency alternatives to byteorder.
#[allow(dead_code)]
pub(crate) mod le {
    use std::io::{self, Read, Write};

    #[inline]
    pub fn read_u8<R: Read>(r: &mut R) -> io::Result<u8> {
        let mut b = [0u8; 1];
        r.read_exact(&mut b)?;
        Ok(b[0])
    }

    #[inline]
    pub fn read_u16<R: Read>(r: &mut R) -> io::Result<u16> {
        let mut b = [0u8; 2];
        r.read_exact(&mut b)?;
        Ok(u16::from_le_bytes(b))
    }

    #[inline]
    pub fn read_u32<R: Read>(r: &mut R) -> io::Result<u32> {
        let mut b = [0u8; 4];
        r.read_exact(&mut b)?;
        Ok(u32::from_le_bytes(b))
    }

    #[inline]
    pub fn read_u64<R: Read>(r: &mut R) -> io::Result<u64> {
        let mut b = [0u8; 8];
        r.read_exact(&mut b)?;
        Ok(u64::from_le_bytes(b))
    }

    #[inline]
    pub fn read_i8<R: Read>(r: &mut R) -> io::Result<i8> {
        read_u8(r).map(|v| v as i8)
    }

    #[inline]
    pub fn read_i16<R: Read>(r: &mut R) -> io::Result<i16> {
        let mut b = [0u8; 2];
        r.read_exact(&mut b)?;
        Ok(i16::from_le_bytes(b))
    }

    #[inline]
    pub fn read_i32<R: Read>(r: &mut R) -> io::Result<i32> {
        let mut b = [0u8; 4];
        r.read_exact(&mut b)?;
        Ok(i32::from_le_bytes(b))
    }

    #[inline]
    pub fn read_f32<R: Read>(r: &mut R) -> io::Result<f32> {
        let mut b = [0u8; 4];
        r.read_exact(&mut b)?;
        Ok(f32::from_le_bytes(b))
    }

    #[inline]
    pub fn read_f64<R: Read>(r: &mut R) -> io::Result<f64> {
        let mut b = [0u8; 8];
        r.read_exact(&mut b)?;
        Ok(f64::from_le_bytes(b))
    }

    // ── Writers ──────────────────────────────────────────────────────────

    #[inline]
    pub fn write_u8<W: Write>(w: &mut W, v: u8) -> io::Result<()> { w.write_all(&[v]) }

    #[inline]
    pub fn write_u16<W: Write>(w: &mut W, v: u16) -> io::Result<()> {
        w.write_all(&v.to_le_bytes())
    }

    #[inline]
    pub fn write_u32<W: Write>(w: &mut W, v: u32) -> io::Result<()> {
        w.write_all(&v.to_le_bytes())
    }

    #[inline]
    pub fn write_u64<W: Write>(w: &mut W, v: u64) -> io::Result<()> {
        w.write_all(&v.to_le_bytes())
    }

    #[inline]
    pub fn write_i8<W: Write>(w: &mut W, v: i8) -> io::Result<()> {
        w.write_all(&[v as u8])
    }

    #[inline]
    pub fn write_i16<W: Write>(w: &mut W, v: i16) -> io::Result<()> {
        w.write_all(&v.to_le_bytes())
    }

    #[inline]
    pub fn write_i32<W: Write>(w: &mut W, v: i32) -> io::Result<()> {
        w.write_all(&v.to_le_bytes())
    }

    #[inline]
    pub fn write_f32<W: Write>(w: &mut W, v: f32) -> io::Result<()> {
        w.write_all(&v.to_le_bytes())
    }

    #[inline]
    pub fn write_f64<W: Write>(w: &mut W, v: f64) -> io::Result<()> {
        w.write_all(&v.to_le_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::PointReader;
    use crate::{PointRecord, Result};

    struct HugeCountEmptyReader;

    impl PointReader for HugeCountEmptyReader {
        fn read_point(&mut self, _out: &mut PointRecord) -> Result<bool> {
            Ok(false)
        }

        fn point_count(&self) -> Option<u64> {
            Some(u64::MAX)
        }
    }

    #[test]
    fn read_all_does_not_abort_on_huge_header_hint() {
        let mut r = HugeCountEmptyReader;
        let pts = r.read_all().expect("read_all should handle oversized hint safely");
        assert!(pts.is_empty());
    }
}

/// Big-endian helpers (used by PLY big-endian and E57 XML UTF-8 fields).
#[allow(dead_code)]
pub(crate) mod be {
    use std::io::{self, Read, Write};

    #[inline]
    pub fn read_u16<R: Read>(r: &mut R) -> io::Result<u16> {
        let mut b = [0u8; 2]; r.read_exact(&mut b)?; Ok(u16::from_be_bytes(b))
    }
    #[inline]
    pub fn read_u32<R: Read>(r: &mut R) -> io::Result<u32> {
        let mut b = [0u8; 4]; r.read_exact(&mut b)?; Ok(u32::from_be_bytes(b))
    }
    #[inline]
    pub fn read_f32<R: Read>(r: &mut R) -> io::Result<f32> {
        let mut b = [0u8; 4]; r.read_exact(&mut b)?; Ok(f32::from_be_bytes(b))
    }
    #[inline]
    pub fn read_f64<R: Read>(r: &mut R) -> io::Result<f64> {
        let mut b = [0u8; 8]; r.read_exact(&mut b)?; Ok(f64::from_be_bytes(b))
    }
    #[inline]
    pub fn write_u32<W: Write>(w: &mut W, v: u32) -> io::Result<()> {
        w.write_all(&v.to_be_bytes())
    }
    #[inline]
    pub fn write_f32<W: Write>(w: &mut W, v: f32) -> io::Result<()> {
        w.write_all(&v.to_be_bytes())
    }
    #[inline]
    pub fn write_f64<W: Write>(w: &mut W, v: f64) -> io::Result<()> {
        w.write_all(&v.to_be_bytes())
    }
}

/// Buffered I/O helpers.
#[allow(dead_code)]
pub(crate) fn buffered_file_reader(path: &std::path::Path)
    -> std::io::Result<std::io::BufReader<std::fs::File>>
{
    let f = std::fs::File::open(path)?;
    // 256 KiB read buffer keeps system-call overhead low for large files
    Ok(std::io::BufReader::with_capacity(256 * 1024, f))
}

#[allow(dead_code)]
pub(crate) fn buffered_file_writer(path: &std::path::Path)
    -> std::io::Result<std::io::BufWriter<std::fs::File>>
{
    let f = std::fs::File::create(path)?;
    Ok(std::io::BufWriter::with_capacity(256 * 1024, f))
}
