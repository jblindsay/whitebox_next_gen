//! CRC-32 (ISO 3309 / ITU-T V.42) implementation used by E57 page checksums.
//!
//! No external dependency — the lookup table is computed at startup.

/// Compute CRC-32 of a byte slice.
pub fn crc32(data: &[u8]) -> u32 {
    static TABLE: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for i in 0u32..256 {
            let mut c = i;
            for _ in 0..8 {
                if c & 1 != 0 { c = 0xEDB8_8320 ^ (c >> 1); }
                else { c >>= 1; }
            }
            t[i as usize] = c;
        }
        t
    });
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc = table[((crc ^ u32::from(b)) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crc32_empty() { assert_eq!(crc32(b""), 0x0000_0000); }
    #[test]
    fn crc32_known() {
        // CRC-32 of "123456789" = 0xCBF43926
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }
}
