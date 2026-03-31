//! Camera model types and intrinsic parameter structs.

use serde::{Deserialize, Serialize};

/// Camera projection model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CameraModel {
    /// Automatically select based on image metadata (default).
    #[default]
    Auto,
    /// Standard rectilinear (pinhole) model.
    Pinhole,
    /// Fisheye / equidistant projection model.
    Fisheye,
}

impl CameraModel {
    /// Parse from a string arg value ("auto", "pinhole", "fisheye").
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "auto" => Some(Self::Auto),
            "pinhole" => Some(Self::Pinhole),
            "fisheye" => Some(Self::Fisheye),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Pinhole => "pinhole",
            Self::Fisheye => "fisheye",
        }
    }
}

/// Calibrated (or estimated) camera intrinsic parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraIntrinsics {
    /// Focal length in pixels, X axis.
    pub fx: f64,
    /// Focal length in pixels, Y axis.
    pub fy: f64,
    /// Principal point X.
    pub cx: f64,
    /// Principal point Y.
    pub cy: f64,
    /// Radial distortion k1.
    pub k1: f64,
    /// Radial distortion k2.
    pub k2: f64,
    /// Tangential distortion p1.
    pub p1: f64,
    /// Tangential distortion p2.
    pub p2: f64,
}

impl CameraIntrinsics {
    /// Unit (identity) intrinsics — useful as a placeholder.
    pub fn identity(image_width_px: u32, image_height_px: u32) -> Self {
        let fx = image_width_px as f64 * 1.2; // rough 35mm-equivalent heuristic
        Self {
            fx,
            fy: fx,
            cx: image_width_px as f64 / 2.0,
            cy: image_height_px as f64 / 2.0,
            k1: 0.0,
            k2: 0.0,
            p1: 0.0,
            p2: 0.0,
        }
    }
}
