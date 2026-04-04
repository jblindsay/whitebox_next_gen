//! Image set ingestion — scan a directory and collect frame metadata.

use exif::{Field, In, Reader as ExifReader, Tag, Value};
use image::{self, imageops::FilterType};
use std::f64::consts::PI;
use std::io::BufReader;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;

const DEFAULT_SENSOR_WIDTH_MM: f64 = 13.2;
const DEFAULT_HFOV_DEG: f64 = 84.0;

/// WGS-84 GPS coordinate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpsCoordinate {
    /// Decimal degrees latitude (positive = North).
    pub lat: f64,
    /// Decimal degrees longitude (positive = East).
    pub lon: f64,
    /// Ellipsoidal altitude in metres.
    pub alt: f64,
}

/// EXIF 0x0112 image raster orientation (how the physical image is rotated on disk).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImageOrientation {
    /// Normal (1): no rotation, top-left origin.
    Normal,
    /// Flipped horizontally (2).
    FlipHorizontal,
    /// Rotated 180° (3).
    Rotate180,
    /// Flipped vertically (4).
    FlipVertical,
    /// Rotated 90° CCW, then flipped horizontally (5).
    Rotate270FlipH,
    /// Rotated 90° CW (6).
    Rotate90,
    /// Rotated 90° CW, then flipped horizontally (7).
    Rotate90FlipH,
    /// Rotated 270° CW (8).
    Rotate270,
}

impl ImageOrientation {
    /// Get the oriented (output) dimensions from raw (input) pixel dimensions.
    /// Swaps width/height for 90° and 270° rotations.
    pub fn oriented_dimensions(&self, raw_width: u32, raw_height: u32) -> (u32, u32) {
        match self {
            ImageOrientation::Rotate90 | ImageOrientation::Rotate270 |
            ImageOrientation::Rotate270FlipH | ImageOrientation::Rotate90FlipH => {
                (raw_height, raw_width)
            }
            _ => (raw_width, raw_height),
        }
    }

    /// Apply this orientation transformation to a `DynamicImage`.
    pub fn apply_to_image(&self, img: image::DynamicImage) -> image::DynamicImage {
        match self {
            ImageOrientation::Normal => img,
            ImageOrientation::FlipHorizontal => img.fliph(),
            ImageOrientation::Rotate180 => img.rotate180(),
            ImageOrientation::FlipVertical => img.flipv(),
            ImageOrientation::Rotate90 => img.rotate90(),
            ImageOrientation::Rotate270 => img.rotate270(),
            ImageOrientation::Rotate270FlipH => {
                let rotated = img.rotate90();
                rotated.fliph()
            }
            ImageOrientation::Rotate90FlipH => {
                let rotated = img.rotate270();
                rotated.fliph()
            }
        }
    }
}

impl Default for ImageOrientation {
    fn default() -> Self {
        ImageOrientation::Normal
    }
}

/// Provenance for an optional per-frame camera attitude prior.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrientationPriorSource {
    /// Standard EXIF `GPSImgDirection` tag.
    ExifGpsImageDirection,
    /// Standard EXIF `GPSTrack` tag.
    ExifGpsTrack,
    /// Generic XMP yaw/pitch/roll fields.
    XmpGeneric,
    /// DJI XMP yaw/pitch/roll fields.
    XmpDji,
    /// DJI MakerNote fallback.
    DjiMakerNote,
}

/// Optional pose prior extracted from EXIF or XMP metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrientationPrior {
    /// Heading of the image top in degrees clockwise from north.
    pub yaw_deg: Option<f64>,
    /// Optional normalized pitch in degrees when confidently available.
    pub pitch_deg: Option<f64>,
    /// Optional normalized roll in degrees when confidently available.
    pub roll_deg: Option<f64>,
    /// Origin of the parsed prior.
    pub source: OrientationPriorSource,
}

/// Per-frame metadata extracted from EXIF and image headers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameMetadata {
    /// GPS position, if available.
    pub gps: Option<GpsCoordinate>,
    /// Focal length in millimetres, if available.
    pub focal_length_mm: Option<f64>,
    /// Sensor width in millimetres, if available.
    pub sensor_width_mm: Option<f64>,
    /// Image width in pixels *after* applying EXIF orientation correction.
    pub image_width_px: u32,
    /// Image height in pixels *after* applying EXIF orientation correction.
    pub image_height_px: u32,
    /// EXIF `DateTimeOriginal` string, if available.
    pub timestamp: Option<String>,
    /// EXIF 0x0112 image raster orientation (physical rotation on disk).
    pub image_orientation: ImageOrientation,
    /// Optional metadata-derived orientation prior.
    pub orientation_prior: Option<OrientationPrior>,
    /// Laplacian-variance blur score (higher = sharper). `None` when not computed.
    pub blur_score: Option<f64>,
    /// True when RTK GPS tag was detected.
    pub has_rtk_gps: bool,
}

/// A single drone image frame with its path and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageFrame {
    /// Absolute path to the source image file.
    pub path: String,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Extracted metadata.
    pub metadata: FrameMetadata,
}

/// Aggregate quality checks produced during image-set ingest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestChecks {
    /// Number of discovered input frames.
    pub images_total: usize,
    /// Frames that lacked all key EXIF fields (GPS, focal length, timestamp).
    pub missing_exif_count: u64,
    /// Fraction of frames considered blurry (0..1).
    pub blur_fraction: f64,
    /// Estimated along-track overlap percentage (0..100).
    pub overlap_estimate_pct: f64,
    /// Human-readable ingest warnings.
    pub warnings: Vec<String>,
}

/// Extensions that are accepted as drone image inputs.
const ACCEPTED_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "tif", "tiff"];

/// Scan `images_dir` for drone images and return one [`ImageFrame`] per file.
///
/// Reads image header dimensions, attempts EXIF parsing (GPS/focal/timestamp),
/// and computes a quick Laplacian-variance blur score from a downsampled image.
pub fn ingest_image_set(images_dir: &str) -> Result<Vec<ImageFrame>> {
    let dir = std::path::Path::new(images_dir);
    if !dir.is_dir() {
        return Err(crate::error::PhotogrammetryError::ImageSet(format!(
            "'{}' is not a directory",
            images_dir
        )));
    }

    let mut frames = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());
        let accepted = ext
            .as_deref()
            .map(|e| ACCEPTED_EXTENSIONS.contains(&e))
            .unwrap_or(false);
        if !accepted {
            continue;
        }

        let path_str = path.to_string_lossy().to_string();
        let (raw_width, raw_height) = image::image_dimensions(&path)
            .map_err(|e| crate::error::PhotogrammetryError::ImageSet(format!(
                "failed reading dimensions for '{}': {}",
                path_str, e
            )))?;
        let exif = read_exif_metadata(&path);
        
        // Apply oriented dimensions (swap for 90°/270° rotations)
        let (oriented_width, oriented_height) = exif.image_orientation.oriented_dimensions(raw_width, raw_height);

        frames.push(ImageFrame {
            path: path_str,
            width: oriented_width,
            height: oriented_height,
            metadata: FrameMetadata {
                gps: exif.gps,
                focal_length_mm: exif.focal_length_mm,
                sensor_width_mm: None,
                image_width_px: oriented_width,
                image_height_px: oriented_height,
                timestamp: exif.timestamp,
                image_orientation: exif.image_orientation,
                orientation_prior: exif.orientation_prior,
                blur_score: compute_blur_score(&path),
                has_rtk_gps: exif.has_rtk_gps,
            },
        });
    }

    frames.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(frames)
}

/// Compute summary ingest checks from a frame inventory.
pub fn compute_ingest_checks(frames: &[ImageFrame]) -> IngestChecks {
    let images_total = frames.len();
    if images_total == 0 {
        return IngestChecks {
            images_total: 0,
            missing_exif_count: 0,
            blur_fraction: 0.0,
            overlap_estimate_pct: 0.0,
            warnings: vec!["No images found in input directory.".to_string()],
        };
    }

    let missing_exif_count = frames
        .iter()
        .filter(|f| {
            f.metadata.gps.is_none()
                && f.metadata.focal_length_mm.is_none()
                && f.metadata.timestamp.is_none()
        })
        .count() as u64;

    let gps_frame_count = frames.iter().filter(|f| f.metadata.gps.is_some()).count();
    let blur_scored_count = frames
        .iter()
        .filter(|f| f.metadata.blur_score.is_some())
        .count();

    let blur_fraction = estimate_blur_fraction(frames);
    let overlap_estimate_pct = estimate_overlap_pct(frames);

    let mut warnings = Vec::new();
    if missing_exif_count > 0 {
        warnings.push(format!(
            "{} frame(s) are missing key EXIF fields (GPS/focal/timestamp).",
            missing_exif_count
        ));
    }
    if blur_fraction > 0.30 {
        warnings.push(format!(
            "High blur fraction ({:.0}%). Re-fly with lower speed and faster shutter.",
            blur_fraction * 100.0
        ));
    }
    if blur_scored_count < images_total {
        warnings.push(format!(
            "Blur estimate unavailable for {} frame(s).",
            images_total - blur_scored_count
        ));
    }
    if gps_frame_count < 2 {
        warnings.push(
            "Overlap estimate unavailable: fewer than two geotagged frames were found."
                .to_string(),
        );
    } else if overlap_estimate_pct < 70.0 {
        warnings.push(format!(
            "Estimated overlap is {:.0}%, below 70% guidance.",
            overlap_estimate_pct
        ));
    }

    IngestChecks {
        images_total,
        missing_exif_count,
        blur_fraction,
        overlap_estimate_pct,
        warnings,
    }
}

#[derive(Debug, Default)]
struct ExifMetadata {
    gps: Option<GpsCoordinate>,
    focal_length_mm: Option<f64>,
    timestamp: Option<String>,
    image_orientation: ImageOrientation,
    orientation_prior: Option<OrientationPrior>,
    has_rtk_gps: bool,
}

fn read_exif_metadata(path: &Path) -> ExifMetadata {
    let raw_bytes = std::fs::read(path).unwrap_or_default();
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return ExifMetadata::default(),
    };
    let mut reader = BufReader::new(file);
    let exif = match ExifReader::new().read_from_container(&mut reader) {
        Ok(v) => v,
        Err(_) => return ExifMetadata::default(),
    };

    let lat = parse_gps_coord(
        exif.get_field(Tag::GPSLatitude, In::PRIMARY).map(|f| &f.value),
        exif.get_field(Tag::GPSLatitudeRef, In::PRIMARY)
            .map(|f| &f.value),
    );
    let lon = parse_gps_coord(
        exif.get_field(Tag::GPSLongitude, In::PRIMARY)
            .map(|f| &f.value),
        exif.get_field(Tag::GPSLongitudeRef, In::PRIMARY)
            .map(|f| &f.value),
    );
    let alt = exif
        .get_field(Tag::GPSAltitude, In::PRIMARY)
        .and_then(|f| parse_single_rational(&f.value));

    let gps = match (lat, lon) {
        (Some(lat), Some(lon)) => Some(GpsCoordinate {
            lat,
            lon,
            alt: alt.unwrap_or(0.0),
        }),
        _ => None,
    };

    let focal_length_mm = exif
        .get_field(Tag::FocalLength, In::PRIMARY)
        .and_then(|f| parse_single_rational(&f.value));

    let timestamp = exif
        .get_field(Tag::DateTimeOriginal, In::PRIMARY)
        .and_then(|f| parse_ascii_string(&f.value));

    let image_orientation = parse_exif_image_orientation(&exif);

    let orientation_prior = parse_xmp_orientation(&raw_bytes)
        .or_else(|| parse_standard_exif_orientation(&exif))
        .or_else(|| parse_dji_makernote_orientation(&exif));

    let has_rtk_gps = exif
        .get_field(Tag::GPSDifferential, In::PRIMARY)
        .and_then(|f| match &f.value {
            Value::Short(v) => v.first().copied(),
            _ => None,
        })
        .map(|v| v > 0)
        .unwrap_or(false);

    ExifMetadata {
        gps,
        focal_length_mm,
        timestamp,
        image_orientation,
        orientation_prior,
        has_rtk_gps,
    }
}

fn parse_single_rational(value: &Value) -> Option<f64> {
    match value {
        Value::Rational(v) => v.first().map(|r| r.num as f64 / r.denom as f64),
        _ => None,
    }
}

fn parse_ascii_string(value: &Value) -> Option<String> {
    match value {
        Value::Ascii(v) => v
            .first()
            .map(|bytes| String::from_utf8_lossy(bytes).trim_matches('\0').trim().to_string())
            .filter(|s| !s.is_empty()),
        _ => None,
    }
}

fn parse_exif_image_orientation(exif: &exif::Exif) -> ImageOrientation {
    exif.get_field(Tag::Orientation, In::PRIMARY)
        .and_then(|f| match &f.value {
            Value::Short(v) => v.first().copied(),
            _ => None,
        })
        .and_then(|tag_value| match tag_value {
            1 => Some(ImageOrientation::Normal),
            2 => Some(ImageOrientation::FlipHorizontal),
            3 => Some(ImageOrientation::Rotate180),
            4 => Some(ImageOrientation::FlipVertical),
            5 => Some(ImageOrientation::Rotate270FlipH),
            6 => Some(ImageOrientation::Rotate90),
            7 => Some(ImageOrientation::Rotate90FlipH),
            8 => Some(ImageOrientation::Rotate270),
            _ => None,
        })
        .unwrap_or(ImageOrientation::Normal)
}

fn parse_standard_exif_orientation(exif: &exif::Exif) -> Option<OrientationPrior> {
    if let Some(field) = exif.get_field(Tag::GPSImgDirection, In::PRIMARY) {
        let yaw_deg = parse_single_rational(&field.value).map(normalize_heading_deg)?;
        return Some(OrientationPrior {
            yaw_deg: Some(yaw_deg),
            pitch_deg: None,
            roll_deg: None,
            source: OrientationPriorSource::ExifGpsImageDirection,
        });
    }
    if let Some(field) = exif.get_field(Tag::GPSTrack, In::PRIMARY) {
        let yaw_deg = parse_single_rational(&field.value).map(normalize_heading_deg)?;
        return Some(OrientationPrior {
            yaw_deg: Some(yaw_deg),
            pitch_deg: None,
            roll_deg: None,
            source: OrientationPriorSource::ExifGpsTrack,
        });
    }
    None
}

fn parse_xmp_orientation(raw_bytes: &[u8]) -> Option<OrientationPrior> {
    if raw_bytes.is_empty() {
        return None;
    }
    let text = String::from_utf8_lossy(raw_bytes);
    let xmp = extract_xmp_block(&text).unwrap_or(&text);

    let dji_yaw = extract_xmp_attr_f64(xmp, &["drone-dji:FlightYawDegree", "drone-dji:GimbalYawDegree"]);
    let generic_yaw = extract_xmp_attr_f64(xmp, &["Camera:Yaw", "camera:Yaw"]);
    let yaw_deg = dji_yaw.or(generic_yaw).map(normalize_heading_deg);

    let dji_pitch = extract_xmp_attr_f64(xmp, &["drone-dji:GimbalPitchDegree"])
        .map(|v| (v + 90.0).clamp(-45.0, 45.0));
    let generic_pitch = extract_xmp_attr_f64(xmp, &["Camera:Pitch", "camera:Pitch"]);
    let pitch_deg = dji_pitch.or(generic_pitch);

    let dji_roll = extract_xmp_attr_f64(xmp, &["drone-dji:GimbalRollDegree"]);
    let generic_roll = extract_xmp_attr_f64(xmp, &["Camera:Roll", "camera:Roll"]);
    let roll_deg = dji_roll.or(generic_roll);

    if yaw_deg.is_none() && pitch_deg.is_none() && roll_deg.is_none() {
        return None;
    }

    Some(OrientationPrior {
        yaw_deg,
        pitch_deg,
        roll_deg,
        source: if dji_yaw.is_some() || dji_pitch.is_some() || dji_roll.is_some() {
            OrientationPriorSource::XmpDji
        } else {
            OrientationPriorSource::XmpGeneric
        },
    })
}

fn extract_xmp_block<'a>(text: &'a str) -> Option<&'a str> {
    let start = text.find("<x:xmpmeta").or_else(|| text.find("<?xpacket"))?;
    let end = text[start..]
        .find("</x:xmpmeta>")
        .map(|idx| start + idx + "</x:xmpmeta>".len())
        .unwrap_or(text.len());
    Some(&text[start..end])
}

fn extract_xmp_attr_f64(text: &str, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|key| extract_single_xmp_attr_f64(text, key))
}

fn extract_single_xmp_attr_f64(text: &str, key: &str) -> Option<f64> {
    let anchor = text.find(key)?;
    let after_key = &text[anchor + key.len()..];
    let eq_pos = after_key.find('=')?;
    let after_eq = after_key[eq_pos + 1..].trim_start();
    let quote = after_eq.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let after_quote = &after_eq[quote.len_utf8()..];
    let end = after_quote.find(quote)?;
    after_quote[..end].trim().parse::<f64>().ok()
}

fn parse_dji_makernote_orientation(exif: &exif::Exif) -> Option<OrientationPrior> {
    let field = exif.get_field(Tag::MakerNote, In::PRIMARY)?;
    let data = field_bytes(field)?;
    let yaw_deg = parse_dji_makernote_heading(data).map(normalize_heading_deg)?;
    Some(OrientationPrior {
        yaw_deg: Some(yaw_deg),
        pitch_deg: None,
        roll_deg: None,
        source: OrientationPriorSource::DjiMakerNote,
    })
}

fn field_bytes(field: &Field) -> Option<&[u8]> {
    match &field.value {
        Value::Undefined(bytes, _) => Some(bytes.as_slice()),
        Value::Byte(bytes) => Some(bytes.as_slice()),
        _ => None,
    }
}

fn parse_dji_makernote_heading(bytes: &[u8]) -> Option<f64> {
    if bytes.len() < 2 {
        return None;
    }
    let entry_count = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
    let mut offset = 2usize;
    for _ in 0..entry_count {
        if offset + 12 > bytes.len() {
            break;
        }
        let tag = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        let field_type = u16::from_le_bytes([bytes[offset + 2], bytes[offset + 3]]);
        let count = u32::from_le_bytes([
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]);
        if tag == 7 && field_type == 11 && count == 1 {
            return Some(f32::from_le_bytes([
                bytes[offset + 8],
                bytes[offset + 9],
                bytes[offset + 10],
                bytes[offset + 11],
            ]) as f64);
        }
        offset += 12;
    }
    None
}

fn normalize_heading_deg(value: f64) -> f64 {
    let mut heading = value % 360.0;
    if heading < 0.0 {
        heading += 360.0;
    }
    heading
}

pub fn orientation_prior_yaw_rad(prior: &OrientationPrior) -> Option<f64> {
    let heading_deg = prior.yaw_deg?;
    let yaw_rad = (90.0 - heading_deg).to_radians();
    Some(normalize_angle_rad(yaw_rad))
}

fn normalize_angle_rad(mut value: f64) -> f64 {
    while value > PI {
        value -= 2.0 * PI;
    }
    while value < -PI {
        value += 2.0 * PI;
    }
    value
}

fn parse_gps_coord(coord: Option<&Value>, coord_ref: Option<&Value>) -> Option<f64> {
    let sign = match coord_ref {
        Some(Value::Ascii(v)) => {
            let c = v.first().and_then(|b| b.first()).copied().unwrap_or(b'N');
            if matches!(c, b'S' | b'W') {
                -1.0
            } else {
                1.0
            }
        }
        _ => 1.0,
    };

    match coord {
        Some(Value::Rational(v)) if v.len() >= 3 => {
            let d = v[0].num as f64 / v[0].denom as f64;
            let m = v[1].num as f64 / v[1].denom as f64;
            let s = v[2].num as f64 / v[2].denom as f64;
            Some(sign * (d + m / 60.0 + s / 3600.0))
        }
        _ => None,
    }
}

fn compute_blur_score(path: &Path) -> Option<f64> {
    let img = image::open(path).ok()?;
    let thumb = img.resize(512, 512, FilterType::Triangle).to_luma8();
    if thumb.width() < 3 || thumb.height() < 3 {
        return None;
    }

    let w = thumb.width();
    let h = thumb.height();
    let mut vals = Vec::with_capacity(((w - 2) * (h - 2)) as usize);

    for y in 1..(h - 1) {
        for x in 1..(w - 1) {
            let c = thumb.get_pixel(x, y)[0] as f64;
            let l = thumb.get_pixel(x - 1, y)[0] as f64;
            let r = thumb.get_pixel(x + 1, y)[0] as f64;
            let u = thumb.get_pixel(x, y - 1)[0] as f64;
            let d = thumb.get_pixel(x, y + 1)[0] as f64;
            vals.push((4.0 * c) - l - r - u - d);
        }
    }

    if vals.is_empty() {
        return None;
    }
    let mean = vals.iter().sum::<f64>() / vals.len() as f64;
    let var = vals
        .iter()
        .map(|v| {
            let dv = *v - mean;
            dv * dv
        })
        .sum::<f64>()
        / vals.len() as f64;
    Some(var)
}

fn estimate_blur_fraction(frames: &[ImageFrame]) -> f64 {
    let mut scores: Vec<f64> = frames
        .iter()
        .filter_map(|f| f.metadata.blur_score)
        .filter(|v| v.is_finite() && *v >= 0.0)
        .collect();
    if scores.is_empty() {
        return 0.0;
    }

    scores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = scores.len();
    let q10_idx = ((n as f64) * 0.10).floor() as usize;
    let q50_idx = ((n as f64) * 0.50).floor() as usize;
    let q10 = scores[q10_idx.min(n.saturating_sub(1))];
    let q50 = scores[q50_idx.min(n.saturating_sub(1))];

    // Use a robust lower-tail threshold to avoid labeling uniformly sharp datasets
    // as fully blurry when score spread is narrow.
    let threshold = (q50 * 0.45).max(q10 * 1.05).max(1.0);

    let blurry = scores.iter().filter(|s| **s <= threshold).count();
    blurry as f64 / scores.len() as f64
}

fn estimate_overlap_pct(frames: &[ImageFrame]) -> f64 {
    let gps_frames: Vec<&ImageFrame> = frames.iter().filter(|f| f.metadata.gps.is_some()).collect();
    if gps_frames.len() < 2 {
        return 0.0;
    }

    let mut segment_distances_m = Vec::new();
    for pair in gps_frames.windows(2) {
        let a = pair[0].metadata.gps.as_ref().expect("pre-filtered gps");
        let b = pair[1].metadata.gps.as_ref().expect("pre-filtered gps");
        segment_distances_m.push(great_circle_distance_m(a.lat, a.lon, b.lat, b.lon));
    }
    if segment_distances_m.is_empty() {
        return 0.0;
    }
    let avg_spacing_m = segment_distances_m.iter().sum::<f64>() / segment_distances_m.len() as f64;

    let mut footprint_widths = Vec::new();
    for frame in &gps_frames {
        let gps = frame.metadata.gps.as_ref().expect("pre-filtered gps");
        let altitude_m = gps.alt.abs().max(1.0);
        let footprint = if let Some(focal_mm) = frame.metadata.focal_length_mm {
            if focal_mm > 0.0 {
                let sensor_width_mm = frame
                    .metadata
                    .sensor_width_mm
                    .unwrap_or(DEFAULT_SENSOR_WIDTH_MM)
                    .max(1.0);
                altitude_m * (sensor_width_mm / focal_mm)
            } else {
                2.0 * altitude_m * (0.5 * DEFAULT_HFOV_DEG.to_radians()).tan()
            }
        } else {
            2.0 * altitude_m * (0.5 * DEFAULT_HFOV_DEG.to_radians()).tan()
        };
        footprint_widths.push(footprint.max(1.0));
    }
    let avg_footprint_m = footprint_widths.iter().sum::<f64>() / footprint_widths.len() as f64;
    let overlap = (1.0 - (avg_spacing_m / avg_footprint_m)).clamp(0.0, 1.0);
    overlap * 100.0
}

fn great_circle_distance_m(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6_378_137.0_f64;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let lat1r = lat1.to_radians();
    let lat2r = lat2.to_radians();
    let a = (dlat * 0.5).sin().powi(2)
        + lat1r.cos() * lat2r.cos() * (dlon * 0.5).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    r * c
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(path: &str, gps: Option<GpsCoordinate>, blur_score: Option<f64>) -> ImageFrame {
        ImageFrame {
            path: path.to_string(),
            width: 4000,
            height: 3000,
            metadata: FrameMetadata {
                gps,
                focal_length_mm: Some(8.8),
                sensor_width_mm: Some(13.2),
                image_width_px: 4000,
                image_height_px: 3000,
                timestamp: Some("2026:03:26 12:00:00".to_string()),
                orientation_prior: None,
                blur_score,
                has_rtk_gps: false,
            },
        }
    }

    #[test]
    fn ingest_checks_estimate_overlap_and_blur_fraction() {
        let frames = vec![
            frame(
                "a.jpg",
                Some(GpsCoordinate {
                    lat: 43.0,
                    lon: -81.0,
                    alt: 100.0,
                }),
                Some(120.0),
            ),
            frame(
                "b.jpg",
                Some(GpsCoordinate {
                    lat: 43.00005,
                    lon: -81.0,
                    alt: 100.0,
                }),
                Some(95.0),
            ),
            frame(
                "c.jpg",
                Some(GpsCoordinate {
                    lat: 43.00010,
                    lon: -81.0,
                    alt: 100.0,
                }),
                Some(8.0),
            ),
        ];

        let checks = compute_ingest_checks(&frames);
        assert_eq!(checks.images_total, 3);
        assert!(checks.overlap_estimate_pct > 0.0);
        assert!(checks.overlap_estimate_pct <= 100.0);
        assert!(checks.blur_fraction > 0.0);
    }

    #[test]
    fn ingest_checks_flags_missing_exif() {
        let mut frames = vec![frame("a.jpg", None, Some(10.0))];
        frames[0].metadata.focal_length_mm = None;
        frames[0].metadata.timestamp = None;

        let checks = compute_ingest_checks(&frames);
        assert_eq!(checks.missing_exif_count, 1);
        assert!(checks.warnings.iter().any(|w| w.contains("missing key EXIF")));
    }

    #[test]
    fn ingest_checks_warn_when_overlap_cannot_be_estimated() {
        let frames = vec![
            frame("a.jpg", None, Some(180.0)),
            frame("b.jpg", None, Some(175.0)),
            frame("c.jpg", None, Some(170.0)),
        ];

        let checks = compute_ingest_checks(&frames);
        assert_eq!(checks.images_total, 3);
        assert_eq!(checks.overlap_estimate_pct, 0.0);
        assert!(checks
            .warnings
            .iter()
            .any(|w| w.contains("Overlap estimate unavailable")));
    }

    #[test]
    fn blur_fraction_avoids_all_blurry_for_mostly_sharp_frames() {
        let mut frames = vec![
            frame("a.jpg", None, Some(320.0)),
            frame("b.jpg", None, Some(295.0)),
            frame("c.jpg", None, Some(280.0)),
            frame("d.jpg", None, Some(45.0)),
            frame("e.jpg", None, Some(35.0)),
        ];
        for f in &mut frames {
            f.metadata.focal_length_mm = Some(8.8);
            f.metadata.timestamp = Some("2026:03:26 12:00:00".to_string());
        }

        let checks = compute_ingest_checks(&frames);
        assert!(checks.blur_fraction > 0.0);
        assert!(checks.blur_fraction < 1.0);
    }
}
