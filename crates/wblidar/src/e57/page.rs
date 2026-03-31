//! E57 binary page I/O with CRC-32 validation.

use std::io::{Read, Write};
use crate::e57::crc::crc32;
use crate::e57::{PAGE_PAYLOAD, PAGE_SIZE};
use crate::{Error, Result};

/// Read one E57 binary page (1024 bytes), validate its CRC-32, and return
/// the 1020 payload bytes.
pub fn read_page<R: Read>(r: &mut R) -> Result<[u8; PAGE_PAYLOAD]> {
    let mut page = [0u8; PAGE_SIZE];
    r.read_exact(&mut page)?;

    let stored = u32::from_le_bytes(page[PAGE_PAYLOAD..PAGE_SIZE].try_into().unwrap());
    let computed = crc32(&page[..PAGE_PAYLOAD]);
    if stored != computed {
        return Err(Error::CrcMismatch { expected: stored, computed });
    }
    let mut out = [0u8; PAGE_PAYLOAD];
    out.copy_from_slice(&page[..PAGE_PAYLOAD]);
    Ok(out)
}

/// Write one E57 binary page: append a CRC-32 to 1020 bytes of payload.
pub fn write_page<W: Write>(w: &mut W, payload: &[u8; PAGE_PAYLOAD]) -> Result<()> {
    let crc = crc32(payload);
    w.write_all(payload)?;
    w.write_all(&crc.to_le_bytes())?;
    Ok(())
}

/// A streaming page reader that decodes page-by-page into a flat byte buffer.
pub struct PageReader<R: Read> {
    inner: R,
    buf: Vec<u8>,
    pos: usize,
}

impl<R: Read> PageReader<R> {
    /// Create a new page reader.
    pub fn new(inner: R) -> Self {
        PageReader { inner, buf: Vec::new(), pos: 0 }
    }

    /// Read `n` bytes from the paged stream.
    pub fn read_bytes(&mut self, n: usize) -> Result<Vec<u8>> {
        while self.buf.len() - self.pos < n {
            let page = read_page(&mut self.inner)?;
            self.buf.extend_from_slice(&page);
        }
        let out = self.buf[self.pos..self.pos + n].to_vec();
        self.pos += n;
        Ok(out)
    }
}

/// A streaming page writer that buffers up to 1020 bytes and emits a full page.
pub struct PageWriter<W: Write> {
    inner: W,
    buf: Vec<u8>,
}

impl<W: Write> PageWriter<W> {
    /// Create a new page writer.
    pub fn new(inner: W) -> Self { PageWriter { inner, buf: Vec::with_capacity(PAGE_SIZE) } }

    /// Write bytes to the paged stream.
    pub fn write_bytes(&mut self, data: &[u8]) -> Result<()> {
        self.buf.extend_from_slice(data);
        while self.buf.len() >= PAGE_PAYLOAD {
            let mut page = [0u8; PAGE_PAYLOAD];
            page.copy_from_slice(&self.buf[..PAGE_PAYLOAD]);
            write_page(&mut self.inner, &page)?;
            self.buf.drain(..PAGE_PAYLOAD);
        }
        Ok(())
    }

    /// Flush any remaining bytes (zero-padded to a full page).
    pub fn flush_final(&mut self) -> Result<()> {
        if !self.buf.is_empty() {
            let mut page = [0u8; PAGE_PAYLOAD];
            page[..self.buf.len()].copy_from_slice(&self.buf);
            write_page(&mut self.inner, &page)?;
            self.buf.clear();
        }
        Ok(())
    }
}
