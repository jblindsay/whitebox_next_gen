/// Minimal data type tags for targeted decode wiring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataTypeTag {
    F32,
    F64,
    I16,
    FixedString,
}
