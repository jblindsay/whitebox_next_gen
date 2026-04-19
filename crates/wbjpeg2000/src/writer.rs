//! Bit-level writer with JPEG 2000 byte-stuffing support.

use alloc::vec::Vec;

/// A writer that outputs bits to a byte buffer, supporting JPEG 2000 byte-stuffing.
///
/// After writing a 0xFF byte, the next byte is restricted to 7 bits
/// (a 0-bit is inserted before the first data bit), preventing false marker codes.
#[derive(Debug, Clone)]
pub(crate) struct BitWriter {
    data: Vec<u8>,
    /// Current partial byte being assembled.
    buffer: u32,
    /// Number of valid bits in `buffer` (MSB-first).
    bits_in_buffer: u8,
    /// Whether the last completed byte was 0xFF (triggers bit-stuffing).
    last_byte_was_ff: bool,
}

#[allow(dead_code)]
impl BitWriter {
    pub(crate) fn new() -> Self {
        Self {
            data: Vec::new(),
            buffer: 0,
            bits_in_buffer: 0,
            last_byte_was_ff: false,
        }
    }

    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            buffer: 0,
            bits_in_buffer: 0,
            last_byte_was_ff: false,
        }
    }

    /// Write a single bit (0 or 1).
    #[inline]
    pub(crate) fn write_bit(&mut self, bit: u32) {
        self.buffer = (self.buffer << 1) | (bit & 1);
        self.bits_in_buffer += 1;

        let limit = if self.last_byte_was_ff { 7 } else { 8 };
        if self.bits_in_buffer >= limit {
            self.flush_byte();
        }
    }

    /// Write `count` bits from `value` (MSB first).
    #[inline]
    pub(crate) fn write_bits(&mut self, value: u32, count: u8) {
        for i in (0..count).rev() {
            self.write_bit((value >> i) & 1);
        }
    }

    /// Write a full byte directly (no bit-stuffing applied at byte level).
    #[inline]
    pub(crate) fn write_byte(&mut self, byte: u8) {
        self.write_bits(byte as u32, 8);
    }

    /// Write a big-endian u16 directly to the output buffer.
    /// Flushes any pending bits first.
    pub(crate) fn write_u16_raw(&mut self, value: u16) {
        self.flush();
        self.data.push((value >> 8) as u8);
        self.data.push(value as u8);
        self.last_byte_was_ff = false;
    }

    /// Write a big-endian u32 directly to the output buffer.
    pub(crate) fn write_u32_raw(&mut self, value: u32) {
        self.flush();
        self.data.push((value >> 24) as u8);
        self.data.push((value >> 16) as u8);
        self.data.push((value >> 8) as u8);
        self.data.push(value as u8);
        self.last_byte_was_ff = false;
    }

    /// Write raw bytes directly, flushing bit buffer first.
    pub(crate) fn write_bytes_raw(&mut self, bytes: &[u8]) {
        self.flush();
        self.data.extend_from_slice(bytes);
        self.last_byte_was_ff = bytes.last().copied() == Some(0xFF);
    }

    /// Write a JPEG 2000 marker (0xFF followed by the marker code).
    pub(crate) fn write_marker(&mut self, marker: u8) {
        self.flush();
        self.data.push(0xFF);
        self.data.push(marker);
        self.last_byte_was_ff = false;
    }

    /// Flush the partial byte, padding with zero bits.
    pub(crate) fn flush(&mut self) {
        if self.bits_in_buffer > 0 {
            let limit = if self.last_byte_was_ff { 7 } else { 8 };
            let shift = limit - self.bits_in_buffer;
            let byte = (self.buffer << shift) as u8;
            self.last_byte_was_ff = byte == 0xFF;
            self.data.push(byte);
            self.buffer = 0;
            self.bits_in_buffer = 0;
        }
    }

    /// Flush and return the assembled byte buffer.
    pub(crate) fn finish(mut self) -> Vec<u8> {
        self.flush();
        self.data
    }

    /// Current length of the assembled output (including partial byte).
    pub(crate) fn len(&self) -> usize {
        self.data.len() + if self.bits_in_buffer > 0 { 1 } else { 0 }
    }

    /// Access the underlying data buffer (partial byte not included).
    pub(crate) fn data(&self) -> &[u8] {
        &self.data
    }

    fn flush_byte(&mut self) {
        let limit = if self.last_byte_was_ff { 7 } else { 8 };
        let byte = (self.buffer >> (self.bits_in_buffer - limit)) as u8;
        self.last_byte_was_ff = byte == 0xFF;
        self.data.push(byte);
        self.bits_in_buffer -= limit;
        self.buffer &= (1 << self.bits_in_buffer) - 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_bits_basic() {
        let mut w = BitWriter::new();
        w.write_bits(0b10110011, 8);
        let data = w.finish();
        assert_eq!(data, vec![0b10110011]);
    }

    #[test]
    fn test_write_bits_partial() {
        let mut w = BitWriter::new();
        w.write_bits(0b101, 3);
        w.write_bits(0b11001, 5);
        let data = w.finish();
        assert_eq!(data, vec![0b10111001]);
    }

    #[test]
    fn test_byte_stuffing() {
        let mut w = BitWriter::new();
        // Write 0xFF
        w.write_bits(0xFF, 8);
        // Next byte should be limited to 7 bits due to stuffing
        w.write_bits(0b1010101, 7);
        let data = w.finish();
        assert_eq!(data[0], 0xFF);
        // After 0xFF, the 7 bits are written with a leading 0 (stuffed bit)
        assert_eq!(data[1], 0b1010101);
    }

    #[test]
    fn test_marker_write() {
        let mut w = BitWriter::new();
        w.write_marker(0x51); // SIZ marker
        let data = w.finish();
        assert_eq!(data, vec![0xFF, 0x51]);
    }

    #[test]
    fn test_round_trip_u16() {
        let mut w = BitWriter::new();
        w.write_u16_raw(0x1234);
        let data = w.finish();
        assert_eq!(data, vec![0x12, 0x34]);
    }
}
