//! TIFF and GeoTIFF tag constants and enumerations.

#![allow(dead_code)]

// ── TIFF tag codes ──────────────────────────────────────────────────────────

/// TIFF tag codes (baseline + extended + GeoTIFF).
#[allow(non_upper_case_globals, dead_code, missing_docs)]
pub mod tag {
    // Baseline
    pub const NewSubFileType: u16 = 254;
    pub const SubFileType: u16 = 255;
    pub const ImageWidth: u16 = 256;
    pub const ImageLength: u16 = 257;
    pub const BitsPerSample: u16 = 258;
    pub const Compression: u16 = 259;
    pub const PhotometricInterpretation: u16 = 262;
    pub const StripOffsets: u16 = 273;
    pub const SamplesPerPixel: u16 = 277;
    pub const RowsPerStrip: u16 = 278;
    pub const StripByteCounts: u16 = 279;
    pub const XResolution: u16 = 282;
    pub const YResolution: u16 = 283;
    pub const PlanarConfiguration: u16 = 284;
    pub const ResolutionUnit: u16 = 296;
    pub const Software: u16 = 305;
    pub const DateTime: u16 = 306;
    pub const SampleFormat: u16 = 339;
    pub const ExtraSamples: u16 = 338;

    // Tiled TIFF
    pub const TileWidth: u16 = 322;
    pub const TileLength: u16 = 323;
    pub const TileOffsets: u16 = 324;
    pub const TileByteCounts: u16 = 325;

    // GeoTIFF / GDAL
    pub const ModelPixelScaleTag: u16 = 33550;
    pub const ModelTiepointTag: u16 = 33922;
    pub const ModelTransformationTag: u16 = 34264;
    pub const GeoKeyDirectoryTag: u16 = 34735;
    pub const GeoDoubleParamsTag: u16 = 34736;
    pub const GeoAsciiParamsTag: u16 = 34737;
    pub const GdalMetadata: u16 = 42112;
    pub const GdalNodata: u16 = 42113;
}

// ── TIFF data types ─────────────────────────────────────────────────────────

/// TIFF field data type codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum DataType {
    /// 8-bit unsigned integer
    Byte = 1,
    /// ASCII string (NUL-terminated)
    Ascii = 2,
    /// 16-bit unsigned integer
    Short = 3,
    /// 32-bit unsigned integer
    Long = 4,
    /// Two longs: numerator / denominator
    Rational = 5,
    /// 8-bit signed integer
    SByte = 6,
    /// Raw bytes (undefined)
    Undefined = 7,
    /// 16-bit signed integer
    SShort = 8,
    /// 32-bit signed integer
    SLong = 9,
    /// Two slongs: numerator / denominator
    SRational = 10,
    /// 32-bit IEEE float
    Float = 11,
    /// 64-bit IEEE double
    Double = 12,
    /// 64-bit unsigned integer (BigTIFF)
    Long8 = 16,
    /// 64-bit signed integer (BigTIFF)
    SLong8 = 17,
}

impl DataType {
    /// Size in bytes of one value of this type.
    pub fn byte_size(self) -> usize {
        match self {
            Self::Byte | Self::SByte | Self::Ascii | Self::Undefined => 1,
            Self::Short | Self::SShort => 2,
            Self::Long | Self::SLong | Self::Float => 4,
            Self::Rational | Self::SRational | Self::Double | Self::Long8 | Self::SLong8 => 8,
        }
    }

    /// Try to parse a u16 into a `DataType`.
    pub fn from_u16(v: u16) -> Option<Self> {
        Some(match v {
            1  => Self::Byte,
            2  => Self::Ascii,
            3  => Self::Short,
            4  => Self::Long,
            5  => Self::Rational,
            6  => Self::SByte,
            7  => Self::Undefined,
            8  => Self::SShort,
            9  => Self::SLong,
            10 => Self::SRational,
            11 => Self::Float,
            12 => Self::Double,
            16 => Self::Long8,
            17 => Self::SLong8,
            _ => return None,
        })
    }
}

// ── Compression ─────────────────────────────────────────────────────────────

/// Compression schemes supported by this library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Compression {
    /// No compression (raw image data).
    #[default]
    None,
    /// Modified Huffman (CCITT 1D run-length), TIFF code 2.
    Huffman,
    /// LZW adaptive compression, TIFF code 5.
    Lzw,
    /// Old-style JPEG, TIFF code 6 (read-only, legacy).
    OldJpeg,
    /// JPEG (new-style), TIFF code 7.
    Jpeg,
    /// Deflate (zlib) — Adobe variant, TIFF code 8.
    Deflate,
    /// WebP compression, TIFF code 50001.
    WebP,
    /// JPEG-XL compression, TIFF code 50002.
    JpegXl,
    /// PackBits run-length encoding, TIFF code 32773.
    PackBits,
    /// Any other compression identified by its TIFF code (read-only).
    Other(u16),
}

impl Compression {
    /// TIFF compression tag value for this variant.
    pub fn tag_value(self) -> u16 {
        match self {
            Self::None => 1,
            Self::Huffman => 2,
            Self::Lzw => 5,
            Self::OldJpeg => 6,
            Self::Jpeg => 7,
            Self::Deflate => 8,
            Self::WebP => 50001,
            Self::JpegXl => 50002,
            Self::PackBits => 32773,
            Self::Other(v) => v,
        }
    }

    /// Parse a TIFF compression tag value.
    pub fn from_tag(v: u16) -> Self {
        match v {
            1 => Self::None,
            2 => Self::Huffman,
            5 => Self::Lzw,
            6 => Self::OldJpeg,
            7 => Self::Jpeg,
            8 | 32946 => Self::Deflate, // 32946 is the "old" deflate code
            50001 => Self::WebP,
            50002 => Self::JpegXl,
            32773 => Self::PackBits,
            other => Self::Other(other),
        }
    }

    /// Human-readable name.
    pub fn name(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Huffman => "Huffman",
            Self::Lzw => "LZW",
            Self::OldJpeg => "OldJPEG",
            Self::Jpeg => "JPEG",
            Self::Deflate => "Deflate",
            Self::WebP => "WebP",
            Self::JpegXl => "JPEG-XL",
            Self::PackBits => "PackBits",
            Self::Other(_) => "Unknown",
        }
    }
}

// ── SampleFormat ─────────────────────────────────────────────────────────────

/// TIFF SampleFormat tag values describing how sample values should be interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SampleFormat {
    /// Unsigned integer (default).
    #[default]
    Uint,
    /// Signed (two's complement) integer.
    Int,
    /// IEEE floating point.
    IeeeFloat,
    /// Undefined / application-specific.
    Undefined,
}

impl SampleFormat {
    /// TIFF SampleFormat tag value.
    pub fn tag_value(self) -> u16 {
        match self {
            Self::Uint => 1,
            Self::Int => 2,
            Self::IeeeFloat => 3,
            Self::Undefined => 4,
        }
    }

    /// Parse a TIFF SampleFormat tag value.
    pub fn from_tag(v: u16) -> Self {
        match v {
            1 => Self::Uint,
            2 => Self::Int,
            3 => Self::IeeeFloat,
            _ => Self::Undefined,
        }
    }
}

/// Convenience alias combining bit depth and format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelType {
    /// 8-bit unsigned integer
    Uint8,
    /// 16-bit unsigned integer
    Uint16,
    /// 32-bit unsigned integer
    Uint32,
    /// 64-bit unsigned integer
    Uint64,
    /// 8-bit signed integer
    Int8,
    /// 16-bit signed integer
    Int16,
    /// 32-bit signed integer
    Int32,
    /// 64-bit signed integer
    Int64,
    /// 32-bit IEEE float
    Float32,
    /// 64-bit IEEE float
    Float64,
}

impl PixelType {
    /// Size in bytes of one sample of this type.
    pub fn byte_size(self) -> usize {
        match self {
            Self::Uint8 | Self::Int8 => 1,
            Self::Uint16 | Self::Int16 => 2,
            Self::Uint32 | Self::Int32 | Self::Float32 => 4,
            Self::Uint64 | Self::Int64 | Self::Float64 => 8,
        }
    }

    /// Bits per sample.
    pub fn bits_per_sample(self) -> u16 {
        (self.byte_size() * 8) as u16
    }

    /// The corresponding `SampleFormat`.
    pub fn sample_format(self) -> SampleFormat {
        match self {
            Self::Uint8 | Self::Uint16 | Self::Uint32 | Self::Uint64 => SampleFormat::Uint,
            Self::Int8 | Self::Int16 | Self::Int32 | Self::Int64 => SampleFormat::Int,
            Self::Float32 | Self::Float64 => SampleFormat::IeeeFloat,
        }
    }
}

// ── Photometric Interpretation ───────────────────────────────────────────────

/// TIFF PhotometricInterpretation tag values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PhotometricInterpretation {
    /// Minimum sample value is white.
    MinIsWhite = 0,
    /// Minimum sample value is black (most common for greyscale/elevation).
    #[default]
    MinIsBlack = 1,
    /// RGB colour model.
    Rgb = 2,
    /// Colour map (palette) image.
    Palette = 3,
    /// Transparency mask.
    Mask = 4,
    /// CMYK colour model.
    Separated = 5,
    /// YCbCr colour model.
    YCbCr = 6,
    /// 1976 CIE L*a*b*.
    CieLab = 8,
}

impl PhotometricInterpretation {
    /// TIFF tag value.
    pub fn tag_value(self) -> u16 {
        self as u16
    }

    /// Parse from TIFF tag value.
    pub fn from_tag(v: u16) -> Self {
        match v {
            0 => Self::MinIsWhite,
            1 => Self::MinIsBlack,
            2 => Self::Rgb,
            3 => Self::Palette,
            4 => Self::Mask,
            5 => Self::Separated,
            6 => Self::YCbCr,
            8 => Self::CieLab,
            _ => Self::MinIsBlack,
        }
    }
}

// ── Planar Configuration ─────────────────────────────────────────────────────

/// TIFF PlanarConfiguration tag values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlanarConfig {
    /// Samples are interleaved (RGBRGB…), the default.
    #[default]
    Chunky = 1,
    /// Samples are stored in separate planes (RRR…GGG…BBB…).
    Planar = 2,
}

impl PlanarConfig {
    /// TIFF tag value.
    pub fn tag_value(self) -> u16 {
        self as u16
    }

    /// Parse from TIFF tag value.
    pub fn from_tag(v: u16) -> Self {
        if v == 2 { Self::Planar } else { Self::Chunky }
    }
}
