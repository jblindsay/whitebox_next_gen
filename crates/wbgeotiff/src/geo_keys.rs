//! GeoTIFF GeoKey directory parsing and construction.
//!
//! The GeoKey directory is stored in three TIFF tags:
//! - `GeoKeyDirectoryTag` (34735): array of u16 values
//! - `GeoDoubleParamsTag` (34736): array of f64 values (referenced by keys)
//! - `GeoAsciiParamsTag` (34737): concatenated ASCII strings (referenced by keys)

#![allow(dead_code)]

use super::error::{GeoTiffError, Result};

// ── GeoKey codes ─────────────────────────────────────────────────────────────

/// Well-known GeoKey codes (GeoTIFF 1.1 specification).
#[allow(non_upper_case_globals, dead_code, missing_docs)]
pub mod key {
    // Configuration keys
    pub const GTModelTypeGeoKey: u16 = 1024;
    pub const GTRasterTypeGeoKey: u16 = 1025;
    pub const GTCitationGeoKey: u16 = 1026;

    // Geographic CS parameter keys
    pub const GeographicTypeGeoKey: u16 = 2048;
    pub const GeogCitationGeoKey: u16 = 2049;
    pub const GeogGeodeticDatumGeoKey: u16 = 2050;
    pub const GeogPrimeMeridianGeoKey: u16 = 2051;
    pub const GeogLinearUnitsGeoKey: u16 = 2052;
    pub const GeogAngularUnitsGeoKey: u16 = 2054;
    pub const GeogEllipsoidGeoKey: u16 = 2056;
    pub const GeogSemiMajorAxisGeoKey: u16 = 2057;
    pub const GeogSemiMinorAxisGeoKey: u16 = 2058;
    pub const GeogInvFlatteningGeoKey: u16 = 2059;
    pub const GeogPrimeMeridianLongGeoKey: u16 = 2061;

    // Projected CS parameter keys
    pub const ProjectedCSTypeGeoKey: u16 = 3072;
    pub const PCSCitationGeoKey: u16 = 3073;
    pub const ProjectionGeoKey: u16 = 3074;
    pub const ProjCoordTransGeoKey: u16 = 3075;
    pub const ProjLinearUnitsGeoKey: u16 = 3076;
    pub const ProjStdParallel1GeoKey: u16 = 3078;
    pub const ProjStdParallel2GeoKey: u16 = 3079;
    pub const ProjNatOriginLongGeoKey: u16 = 3080;
    pub const ProjNatOriginLatGeoKey: u16 = 3081;
    pub const ProjFalseEastingGeoKey: u16 = 3082;
    pub const ProjFalseNorthingGeoKey: u16 = 3083;
    pub const ProjFalseProjOriginLongGeoKey: u16 = 3084;
    pub const ProjFalseProjOriginLatGeoKey: u16 = 3085;
    pub const ProjCenterLongGeoKey: u16 = 3088;
    pub const ProjCenterLatGeoKey: u16 = 3089;
    pub const ProjScaleAtNatOriginGeoKey: u16 = 3092;
    pub const ProjAzimuthAngleGeoKey: u16 = 3094;
    pub const ProjStraightVertPoleLongGeoKey: u16 = 3095;

    // Vertical CS keys
    pub const VerticalCSTypeGeoKey: u16 = 4096;
    pub const VerticalCitationGeoKey: u16 = 4097;
    pub const VerticalDatumGeoKey: u16 = 4098;
    pub const VerticalUnitsGeoKey: u16 = 4099;
}

// ── ModelType ─────────────────────────────────────────────────────────────────

/// GTModelTypeGeoKey values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelType {
    /// Projection coordinate system (metres / feet).
    Projected = 1,
    /// Geographic coordinate system (degrees).
    Geographic = 2,
    /// Geocentric coordinate system (rarely used).
    Geocentric = 3,
    /// User-defined or unknown.
    UserDefined = 32767,
}

impl ModelType {
    /// Parse from a GeoKey value.
    pub fn from_value(v: u16) -> Self {
        match v {
            1 => Self::Projected,
            2 => Self::Geographic,
            3 => Self::Geocentric,
            _ => Self::UserDefined,
        }
    }
}

// ── RasterType ────────────────────────────────────────────────────────────────

/// GTRasterTypeGeoKey values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RasterType {
    /// Pixel-is-Area (the default; pixels represent an area centred on the tiepoint).
    PixelIsArea = 1,
    /// Pixel-is-Point (the tiepoint is at the centre of the pixel).
    PixelIsPoint = 2,
}

impl RasterType {
    /// Parse from a GeoKey value.
    pub fn from_value(v: u16) -> Self {
        if v == 2 { Self::PixelIsPoint } else { Self::PixelIsArea }
    }
}

// ── GeoKeyValue ──────────────────────────────────────────────────────────────

/// The value of a single GeoKey entry.
#[derive(Debug, Clone, PartialEq)]
pub enum GeoKeyValue {
    /// A 16-bit integer value stored inline in the key directory.
    Short(u16),
    /// One or more 64-bit double values from `GeoDoubleParamsTag`.
    Doubles(Vec<f64>),
    /// A UTF-8 string from `GeoAsciiParamsTag`.
    Ascii(String),
}

// ── GeoKeyEntry ──────────────────────────────────────────────────────────────

/// A single decoded GeoKey entry.
#[derive(Debug, Clone)]
pub struct GeoKeyEntry {
    /// GeoKey code (e.g. `GTModelTypeGeoKey` = 1024).
    pub key_id: u16,
    /// Parsed value.
    pub value: GeoKeyValue,
}

// ── GeoKeyDirectory ───────────────────────────────────────────────────────────

/// Decoded GeoKey directory from a GeoTIFF file.
#[derive(Debug, Clone, Default)]
pub struct GeoKeyDirectory {
    /// GeoTIFF specification version (usually 1).
    pub version: u16,
    /// Major revision (usually 1).
    pub key_revision: u16,
    /// Minor revision (usually 0 or 1).
    pub minor_revision: u16,
    /// All decoded key entries.
    pub entries: Vec<GeoKeyEntry>,
}

impl GeoKeyDirectory {
    /// Parse the GeoKey directory from the raw tag arrays.
    ///
    /// - `dir_words`: raw u16 words from `GeoKeyDirectoryTag` (34735)
    /// - `doubles`: values from `GeoDoubleParamsTag` (34736) — may be empty
    /// - `ascii`: concatenated string from `GeoAsciiParamsTag` (34737) — may be empty
    pub fn parse(dir_words: &[u16], doubles: &[f64], ascii: &str) -> Result<Self> {
        if dir_words.len() < 4 {
            return Err(GeoTiffError::InvalidGeoKey(
                "GeoKeyDirectory must have at least 4 header words".into(),
            ));
        }

        let version = dir_words[0];
        let key_revision = dir_words[1];
        let minor_revision = dir_words[2];
        let num_keys = dir_words[3] as usize;

        if dir_words.len() < 4 + num_keys * 4 {
            return Err(GeoTiffError::InvalidGeoKey(format!(
                "Directory claims {} keys but only {} words available",
                num_keys,
                dir_words.len() - 4
            )));
        }

        let mut entries = Vec::with_capacity(num_keys);

        for i in 0..num_keys {
            let base = 4 + i * 4;
            let key_id = dir_words[base];
            let tiff_tag_location = dir_words[base + 1]; // 0 = inline, 34736, or 34737
            let count = dir_words[base + 2] as usize;
            let value_offset = dir_words[base + 3] as usize;

            let value = match tiff_tag_location {
                0 => GeoKeyValue::Short(value_offset as u16),
                34736 => {
                    // Doubles
                    if value_offset + count > doubles.len() {
                        return Err(GeoTiffError::InvalidGeoKey(format!(
                            "GeoKey {}: double offset {} + count {} out of range ({})",
                            key_id,
                            value_offset,
                            count,
                            doubles.len()
                        )));
                    }
                    GeoKeyValue::Doubles(doubles[value_offset..value_offset + count].to_vec())
                }
                34737 => {
                    // ASCII
                    let start = value_offset;
                    let end = value_offset + count;
                    if end > ascii.len() {
                        return Err(GeoTiffError::InvalidGeoKey(format!(
                            "GeoKey {}: ASCII offset {} + count {} out of range ({})",
                            key_id, value_offset, count, ascii.len()
                        )));
                    }
                    let s = &ascii[start..end];
                    // GeoAscii strings use '|' as separator and may be NUL-terminated
                    let s = s.trim_end_matches('|').trim_end_matches('\0');
                    GeoKeyValue::Ascii(s.to_owned())
                }
                other => {
                    return Err(GeoTiffError::InvalidGeoKey(format!(
                        "GeoKey {}: unknown tiff_tag_location {}",
                        key_id, other
                    )));
                }
            };

            entries.push(GeoKeyEntry { key_id, value });
        }

        Ok(Self { version, key_revision, minor_revision, entries })
    }

    // ── Convenience accessors ────────────────────────────────────────────────

    /// Look up a key and return its value, if present.
    pub fn get(&self, key_id: u16) -> Option<&GeoKeyValue> {
        self.entries.iter().find(|e| e.key_id == key_id).map(|e| &e.value)
    }

    /// Get the `GTModelTypeGeoKey` value.
    pub fn model_type(&self) -> Option<ModelType> {
        self.get(key::GTModelTypeGeoKey).and_then(|v| {
            if let GeoKeyValue::Short(n) = v { Some(ModelType::from_value(*n)) } else { None }
        })
    }

    /// Get the `GTRasterTypeGeoKey` value.
    pub fn raster_type(&self) -> Option<RasterType> {
        self.get(key::GTRasterTypeGeoKey).and_then(|v| {
            if let GeoKeyValue::Short(n) = v { Some(RasterType::from_value(*n)) } else { None }
        })
    }

    /// Return the EPSG code for the projected CRS (`ProjectedCSTypeGeoKey`),
    /// or for the geographic CRS (`GeographicTypeGeoKey`), whichever is set.
    pub fn epsg(&self) -> Option<u16> {
        if let Some(GeoKeyValue::Short(n)) = self.get(key::ProjectedCSTypeGeoKey) {
            if *n != 32767 { return Some(*n); }
        }
        if let Some(GeoKeyValue::Short(n)) = self.get(key::GeographicTypeGeoKey) {
            if *n != 32767 { return Some(*n); }
        }
        None
    }

    // ── Serialisation ────────────────────────────────────────────────────────

    /// Encode the GeoKey directory into the three raw tag arrays suitable for
    /// writing into a TIFF file.
    ///
    /// Returns `(dir_words, doubles, ascii)`.
    pub fn encode(&self) -> (Vec<u16>, Vec<f64>, String) {
        let mut dir_words: Vec<u16> = Vec::new();
        let mut doubles: Vec<f64> = Vec::new();
        let mut ascii = String::new();

        // Reserve header slot
        dir_words.extend_from_slice(&[self.version, self.key_revision, self.minor_revision, 0]);

        let mut key_count: u16 = 0;
        for entry in &self.entries {
            key_count += 1;
            match &entry.value {
                GeoKeyValue::Short(v) => {
                    dir_words.extend_from_slice(&[entry.key_id, 0, 1, *v]);
                }
                GeoKeyValue::Doubles(vals) => {
                    let offset = doubles.len() as u16;
                    let count = vals.len() as u16;
                    doubles.extend_from_slice(vals);
                    dir_words.extend_from_slice(&[entry.key_id, 34736, count, offset]);
                }
                GeoKeyValue::Ascii(s) => {
                    let offset = ascii.len() as u16;
                    // GeoAscii: append string + '|' separator
                    let padded = format!("{}|", s);
                    let count = padded.len() as u16;
                    ascii.push_str(&padded);
                    dir_words.extend_from_slice(&[entry.key_id, 34737, count, offset]);
                }
            }
        }
        // Fill in number of keys in header
        dir_words[3] = key_count;

        (dir_words, doubles, ascii)
    }
}

/// Builder for constructing a `GeoKeyDirectory` programmatically.
#[derive(Debug, Default)]
pub struct GeoKeyBuilder {
    entries: Vec<GeoKeyEntry>,
}

impl GeoKeyBuilder {
    /// Create a new empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a short (u16) key.
    pub fn short(mut self, key_id: u16, value: u16) -> Self {
        self.entries.push(GeoKeyEntry { key_id, value: GeoKeyValue::Short(value) });
        self
    }

    /// Add a double key.
    pub fn double(mut self, key_id: u16, value: f64) -> Self {
        self.entries.push(GeoKeyEntry { key_id, value: GeoKeyValue::Doubles(vec![value]) });
        self
    }

    /// Add an ASCII key.
    pub fn ascii(mut self, key_id: u16, value: impl Into<String>) -> Self {
        self.entries.push(GeoKeyEntry { key_id, value: GeoKeyValue::Ascii(value.into()) });
        self
    }

    /// Set the ModelType.
    pub fn model_type(self, t: ModelType) -> Self {
        self.short(key::GTModelTypeGeoKey, t as u16)
    }

    /// Set the RasterType.
    pub fn raster_type(self, t: RasterType) -> Self {
        self.short(key::GTRasterTypeGeoKey, t as u16)
    }

    /// Set both ProjectedCSTypeGeoKey and ModelType to Projected.
    pub fn projected_epsg(self, epsg: u16) -> Self {
        self.short(key::GTModelTypeGeoKey, ModelType::Projected as u16)
            .short(key::GTRasterTypeGeoKey, RasterType::PixelIsArea as u16)
            .short(key::ProjectedCSTypeGeoKey, epsg)
    }

    /// Set both GeographicTypeGeoKey and ModelType to Geographic.
    pub fn geographic_epsg(self, epsg: u16) -> Self {
        self.short(key::GTModelTypeGeoKey, ModelType::Geographic as u16)
            .short(key::GTRasterTypeGeoKey, RasterType::PixelIsArea as u16)
            .short(key::GeographicTypeGeoKey, epsg)
    }

    /// Build the `GeoKeyDirectory`.
    pub fn build(self) -> GeoKeyDirectory {
        GeoKeyDirectory {
            version: 1,
            key_revision: 1,
            minor_revision: 0,
            entries: self.entries,
        }
    }
}
