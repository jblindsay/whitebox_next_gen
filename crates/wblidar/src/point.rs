//! Core point-record type shared across all format modules.
//!
//! `PointRecord` is intentionally flat and `Copy` so that large buffers can be
//! stack-allocated or stored in contiguous `Vec<PointRecord>` slices without
//! indirection.

/// Thermal Infrared + RGB colour for LAS 1.5 PDRFs 13–15.
///
/// Combines thermal imaging data with traditional 16-bit RGB, commonly found in
/// drone-based LiDAR systems that simultaneously capture thermal and RGB imagery.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ThermalRgb {
    /// Thermal infrared band value (radiometric measurement from thermal sensor).
    pub thermal: u16,
    /// Red channel (0–65535).
    pub red: u16,
    /// Green channel (0–65535).
    pub green: u16,
    /// Blue channel (0–65535).
    pub blue: u16,
}

/// Optional 16-bit RGB colour carried by several LAS/LAZ PDRFs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Rgb16 {
    /// Red channel (0–65535).
    pub red: u16,
    /// Green channel (0–65535).
    pub green: u16,
    /// Blue channel (0–65535).
    pub blue: u16,
}

/// Convenience alias for 8-bit colour used by PLY and E57.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Color {
    /// Red channel (0–255).
    pub r: u8,
    /// Green channel (0–255).
    pub g: u8,
    /// Blue channel (0–255).
    pub b: u8,
}

impl From<Rgb16> for Color {
    fn from(c: Rgb16) -> Self {
        Color {
            r: (c.red >> 8) as u8,
            g: (c.green >> 8) as u8,
            b: (c.blue >> 8) as u8,
        }
    }
}

impl From<Color> for Rgb16 {
    fn from(c: Color) -> Self {
        Rgb16 {
            red:   u16::from(c.r) << 8 | u16::from(c.r),
            green: u16::from(c.g) << 8 | u16::from(c.g),
            blue:  u16::from(c.b) << 8 | u16::from(c.b),
        }
    }
}

/// GPS time stored as a double (seconds since GPS epoch or adjusted standard
/// GPS time depending on the LAS global encoding bit).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GpsTime(pub f64);

/// Waveform packet reference (LAS 1.3 / 1.4 PDRFs 4, 5, 9, 10).
#[derive(Debug, Clone, Copy, Default)]
pub struct WaveformPacket {
    /// Wave packet descriptor index.
    pub descriptor_index: u8,
    /// Byte offset to waveform data.
    pub byte_offset: u64,
    /// Waveform packet size in bytes.
    pub packet_size: u32,
    /// Return point waveform location.
    pub return_point_location: f32,
    /// Parametric dx.
    pub dx: f32,
    /// Parametric dy.
    pub dy: f32,
    /// Parametric dz.
    pub dz: f32,
}

/// Opaque extra-byte payload (up to 192 bytes, matching the LAS 1.4 limit).
#[derive(Debug, Clone, Copy)]
pub struct ExtraBytes {
    /// Raw bytes.
    pub data: [u8; 192],
    /// Number of valid bytes in `data`.
    pub len: u8,
}

impl Default for ExtraBytes {
    fn default() -> Self { Self { data: [0u8; 192], len: 0 } }
}

/// The canonical point record used throughout wblidar.
///
/// All format readers decode into this type; all format writers encode from it.
/// Fields that a given format does not support are left at their `Default`
/// value and silently ignored during write.
#[derive(Debug, Clone, Copy, Default)]
pub struct PointRecord {
    // ── Geometry ──────────────────────────────────────────────────────────
    /// X coordinate in the coordinate reference system of the file.
    pub x: f64,
    /// Y coordinate.
    pub y: f64,
    /// Z coordinate (elevation / height).
    pub z: f64,

    // ── Radiometry ────────────────────────────────────────────────────────
    /// Return intensity (0 – 65535; some sensors use only the low 8 bits).
    pub intensity: u16,
    /// 16-bit RGB colour. `None` if the format/PDRF does not carry colour.
    pub color: Option<Rgb16>,
    /// Near-infrared channel (LAS 1.4 PDRF 8 / COPC).
    pub nir: Option<u16>,
    /// Thermal infrared + RGB (LAS 1.5 PDRFs 13–15). `None` for most PDRFs.
    pub thermal_rgb: Option<ThermalRgb>,

    // ── Classification ────────────────────────────────────────────────────
    /// Point classification (ASPRS standard codes).
    pub classification: u8,
    /// User data byte (format-specific).
    pub user_data: u8,
    /// Point source ID.
    pub point_source_id: u16,
    /// Synthetic / key-point / withheld / overlap flags (packed as in LAS 1.4).
    pub flags: u8,

    // ── Return information ────────────────────────────────────────────────
    /// Return number (1-based, 0 = not set).
    pub return_number: u8,
    /// Total number of returns for this pulse.
    pub number_of_returns: u8,
    /// Scan direction flag.
    pub scan_direction_flag: bool,
    /// Edge of flight line.
    pub edge_of_flight_line: bool,
    /// Scan angle (–30 000 … +30 000 in units of 0.006°, i.e. ±180°).
    pub scan_angle: i16,

    // ── Time / waveform ───────────────────────────────────────────────────
    /// GPS time. `None` for formats that do not include timing.
    pub gps_time: Option<GpsTime>,
    /// Waveform packet. `None` for most PDRFs.
    pub waveform: Option<WaveformPacket>,

    // ── Extra bytes ───────────────────────────────────────────────────────
    /// Payload for user-defined extra-byte fields.
    pub extra_bytes: ExtraBytes,

    // ── Normal vector (PLY / E57) ─────────────────────────────────────────
    /// Surface normal X component.
    pub normal_x: Option<f32>,
    /// Surface normal Y component.
    pub normal_y: Option<f32>,
    /// Surface normal Z component.
    pub normal_z: Option<f32>,
}
