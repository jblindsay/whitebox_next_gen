/// Minimal data type tags for targeted decode wiring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataTypeTag {
    F32,
    F64,
    I16,
    U16,
    FixedString,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
    Little,
    Big,
}

pub fn decode_f32(bytes: [u8; 4], endianness: Endianness) -> f32 {
    match endianness {
        Endianness::Little => f32::from_le_bytes(bytes),
        Endianness::Big => f32::from_be_bytes(bytes),
    }
}

pub fn decode_f64(bytes: [u8; 8], endianness: Endianness) -> f64 {
    match endianness {
        Endianness::Little => f64::from_le_bytes(bytes),
        Endianness::Big => f64::from_be_bytes(bytes),
    }
}

pub fn decode_i16(bytes: [u8; 2], endianness: Endianness) -> i16 {
    match endianness {
        Endianness::Little => i16::from_le_bytes(bytes),
        Endianness::Big => i16::from_be_bytes(bytes),
    }
}

pub fn decode_u16(bytes: [u8; 2], endianness: Endianness) -> u16 {
    match endianness {
        Endianness::Little => u16::from_le_bytes(bytes),
        Endianness::Big => u16::from_be_bytes(bytes),
    }
}

pub fn decode_f32_slice(bytes: &[u8], endianness: Endianness) -> Result<Vec<f32>, String> {
    if bytes.len() % 4 != 0 {
        return Err("f32 decode requires byte length divisible by 4".to_string());
    }

    Ok(bytes
        .chunks_exact(4)
        .map(|chunk| decode_f32(chunk.try_into().expect("chunk size is 4"), endianness))
        .collect())
}

pub fn decode_f64_slice(bytes: &[u8], endianness: Endianness) -> Result<Vec<f64>, String> {
    if bytes.len() % 8 != 0 {
        return Err("f64 decode requires byte length divisible by 8".to_string());
    }

    Ok(bytes
        .chunks_exact(8)
        .map(|chunk| decode_f64(chunk.try_into().expect("chunk size is 8"), endianness))
        .collect())
}

pub fn decode_i16_slice(bytes: &[u8], endianness: Endianness) -> Result<Vec<i16>, String> {
    if bytes.len() % 2 != 0 {
        return Err("i16 decode requires byte length divisible by 2".to_string());
    }

    Ok(bytes
        .chunks_exact(2)
        .map(|chunk| decode_i16(chunk.try_into().expect("chunk size is 2"), endianness))
        .collect())
}

pub fn decode_u16_slice(bytes: &[u8], endianness: Endianness) -> Result<Vec<u16>, String> {
    if bytes.len() % 2 != 0 {
        return Err("u16 decode requires byte length divisible by 2".to_string());
    }

    Ok(bytes
        .chunks_exact(2)
        .map(|chunk| decode_u16(chunk.try_into().expect("chunk size is 2"), endianness))
        .collect())
}

pub fn decode_fixed_string(bytes: &[u8]) -> Result<String, String> {
    let end = bytes.iter().position(|byte| *byte == 0).unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[..end])
        .map(|value| value.to_string())
        .map_err(|err| format!("fixed string decode requires valid UTF-8: {err}"))
}

#[cfg(test)]
mod tests {
    use super::{
        decode_f32, decode_f32_slice, decode_f64, decode_f64_slice, decode_fixed_string,
        decode_i16, decode_i16_slice, decode_u16, decode_u16_slice, Endianness,
    };

    #[test]
    fn decodes_f32_little_and_big_endian_values() {
        let little = decode_f32([0x00, 0x00, 0x80, 0x3f], Endianness::Little);
        let big = decode_f32([0x3f, 0x80, 0x00, 0x00], Endianness::Big);
        assert_eq!(little, 1.0);
        assert_eq!(big, 1.0);
    }

    #[test]
    fn decodes_f64_little_and_big_endian_values() {
        let little = decode_f64([0, 0, 0, 0, 0, 0, 0xf0, 0x3f], Endianness::Little);
        let big = decode_f64([0x3f, 0xf0, 0, 0, 0, 0, 0, 0], Endianness::Big);
        assert_eq!(little, 1.0);
        assert_eq!(big, 1.0);
    }

    #[test]
    fn decodes_i16_little_and_big_endian_values() {
        let little = decode_i16([0x34, 0x12], Endianness::Little);
        let big = decode_i16([0x12, 0x34], Endianness::Big);
        assert_eq!(little, 0x1234);
        assert_eq!(big, 0x1234);
    }

    #[test]
    fn decodes_u16_little_and_big_endian_values() {
        let little = decode_u16([0x34, 0x12], Endianness::Little);
        let big = decode_u16([0x12, 0x34], Endianness::Big);
        assert_eq!(little, 0x1234);
        assert_eq!(big, 0x1234);
    }

    #[test]
    fn decodes_slices_for_supported_numeric_types() {
        let f32_values = decode_f32_slice(&[0, 0, 0x80, 0x3f, 0, 0, 0, 0x40], Endianness::Little)
            .expect("f32 slice should decode");
        let f64_values = decode_f64_slice(
            &[0, 0, 0, 0, 0, 0, 0xf0, 0x3f, 0, 0, 0, 0, 0, 0, 0, 0x40],
            Endianness::Little,
        )
        .expect("f64 slice should decode");
        let i16_values = decode_i16_slice(&[0x34, 0x12, 0x78, 0x56], Endianness::Little)
            .expect("i16 slice should decode");
        let u16_values = decode_u16_slice(&[0x34, 0x12, 0x78, 0x56], Endianness::Little)
            .expect("u16 slice should decode");

        assert_eq!(f32_values, vec![1.0, 2.0]);
        assert_eq!(f64_values, vec![1.0, 2.0]);
        assert_eq!(i16_values, vec![0x1234, 0x5678]);
        assert_eq!(u16_values, vec![0x1234, 0x5678]);
    }

    #[test]
    fn rejects_invalid_slice_lengths() {
        assert!(decode_f32_slice(&[0, 1, 2], Endianness::Little).is_err());
        assert!(decode_f64_slice(&[0, 1, 2, 3], Endianness::Little).is_err());
        assert!(decode_i16_slice(&[0], Endianness::Little).is_err());
        assert!(decode_u16_slice(&[0], Endianness::Little).is_err());
    }

    #[test]
    fn decodes_fixed_string_and_trims_null_padding() {
        let value = decode_fixed_string(b"ISO 19139 Series XML\0\0").expect("fixed string should decode");
        assert_eq!(value, "ISO 19139 Series XML");
    }
}
