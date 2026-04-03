//! SAFE bundle mission detection and unified opener.
//!
//! This module provides additive convenience APIs that distinguish Sentinel-1
//! and Sentinel-2 SAFE roots and open the corresponding package type.

use std::fs;
use std::path::Path;

use crate::error::{RasterError, Result};

use super::sentinel1_safe::Sentinel1SafePackage;
use super::sentinel2_safe::Sentinel2SafePackage;

/// Satellite mission type represented by a SAFE bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafeMission {
    /// Sentinel-1 SAFE package.
    Sentinel1,
    /// Sentinel-2 SAFE package.
    Sentinel2,
    /// Unable to determine mission with available cues.
    Unknown,
}

/// Unified SAFE package enum returned by [`open_safe_bundle`].
#[derive(Debug, Clone)]
pub enum SafeBundle {
    /// Sentinel-1 SAFE package.
    Sentinel1(Sentinel1SafePackage),
    /// Sentinel-2 SAFE package.
    Sentinel2(Sentinel2SafePackage),
}

/// Detect the mission family of a SAFE root.
///
/// Detection prefers explicit metadata markers and falls back to file-pattern
/// heuristics (JP2-heavy trees are treated as Sentinel-2; TIFF measurement
/// trees are treated as Sentinel-1).
pub fn detect_safe_mission(safe_root: impl AsRef<Path>) -> Result<SafeMission> {
    let safe_root = safe_root.as_ref();
    if !safe_root.is_dir() {
        return Err(RasterError::Other(format!(
            "SAFE root is not a directory: {}",
            safe_root.display()
        )));
    }

    let mut has_s2_product_xml = false;
    let mut has_manifest_safe = false;

    for entry in fs::read_dir(safe_root)? {
        let entry = entry?;
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let name = p
            .file_name()
            .map(|s| s.to_string_lossy().to_ascii_uppercase())
            .unwrap_or_default();

        if name.starts_with("MTD_MSI") && name.ends_with(".XML") {
            has_s2_product_xml = true;
        }
        if name.eq_ignore_ascii_case("manifest.safe") {
            has_manifest_safe = true;
        }
    }

    if has_s2_product_xml {
        return Ok(SafeMission::Sentinel2);
    }

    if has_manifest_safe {
        let manifest_text = fs::read_to_string(safe_root.join("manifest.safe"))?;
        let u = manifest_text.to_ascii_uppercase();
        if u.contains("SENTINEL-1") || u.contains("S1SAR") {
            return Ok(SafeMission::Sentinel1);
        }
        if u.contains("SENTINEL-2") || u.contains("MSIL1C") || u.contains("MSIL2A") {
            return Ok(SafeMission::Sentinel2);
        }
    }

    let mut jp2_count = 0usize;
    let mut tif_count = 0usize;
    let mut measurement_tif_count = 0usize;
    let mut files = Vec::new();
    collect_files_recursive(safe_root, &mut files)?;
    for path in files {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if ext == "jp2" {
            jp2_count += 1;
            continue;
        }
        if ext == "tif" || ext == "tiff" {
            tif_count += 1;
            if path.components().any(|c| {
                c.as_os_str()
                    .to_string_lossy()
                    .eq_ignore_ascii_case("measurement")
            }) {
                measurement_tif_count += 1;
            }
        }
    }

    if jp2_count > 0 && jp2_count >= tif_count {
        return Ok(SafeMission::Sentinel2);
    }
    if measurement_tif_count > 0 {
        return Ok(SafeMission::Sentinel1);
    }

    Ok(SafeMission::Unknown)
}

/// Open a SAFE root and return a mission-specific package enum.
///
/// This API is additive and does not alter existing direct openers such as
/// [`Sentinel2SafePackage::open`].
pub fn open_safe_bundle(safe_root: impl AsRef<Path>) -> Result<SafeBundle> {
    let safe_root = safe_root.as_ref();
    match detect_safe_mission(safe_root)? {
        SafeMission::Sentinel1 => Ok(SafeBundle::Sentinel1(Sentinel1SafePackage::open(safe_root)?)),
        SafeMission::Sentinel2 => Ok(SafeBundle::Sentinel2(Sentinel2SafePackage::open(safe_root)?)),
        SafeMission::Unknown => Err(RasterError::Other(format!(
            "unable to determine SAFE mission type for '{}'",
            safe_root.display()
        ))),
    }
}

fn collect_files_recursive(root: &Path, out: &mut Vec<std::path::PathBuf>) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            collect_files_recursive(&p, out)?;
        } else {
            out.push(p);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_sentinel2_from_product_xml() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let safe = tmp.path().join("S2_TEST.SAFE");
        fs::create_dir_all(&safe).expect("create safe root");
        fs::write(safe.join("MTD_MSIL2A.xml"), "<root>MSIL2A</root>").expect("write xml");

        let mission = detect_safe_mission(&safe).expect("detect mission");
        assert_eq!(mission, SafeMission::Sentinel2);
    }

    #[test]
    fn detects_sentinel1_from_manifest_marker() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let safe = tmp.path().join("S1_TEST.SAFE");
        fs::create_dir_all(&safe).expect("create safe root");
        fs::write(safe.join("manifest.safe"), "<xfdu>Sentinel-1</xfdu>")
            .expect("write manifest");

        let mission = detect_safe_mission(&safe).expect("detect mission");
        assert_eq!(mission, SafeMission::Sentinel1);
    }

    #[test]
    fn open_safe_bundle_returns_sentinel2_variant() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let safe = tmp.path().join("S2A_MSIL2A_TEST.SAFE");
        fs::create_dir_all(&safe).expect("create safe root");
        fs::write(safe.join("MTD_MSIL2A.xml"), "<n1:Level-2A_User_Product>MSIL2A</n1:Level-2A_User_Product>")
            .expect("write product xml");

        let bundle = open_safe_bundle(&safe).expect("open safe bundle");
        assert!(
            matches!(bundle, SafeBundle::Sentinel2(_)),
            "expected SafeBundle::Sentinel2, got Sentinel1"
        );
        if let SafeBundle::Sentinel2(pkg) = bundle {
            assert_eq!(pkg.safe_root, safe);
        }
    }

    #[test]
    fn open_safe_bundle_returns_sentinel1_variant() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let safe = tmp.path().join("S1A_IW_GRD_TEST.SAFE");
        fs::create_dir_all(&safe).expect("create safe root");
        fs::write(safe.join("manifest.safe"), "<xfdu>Sentinel-1</xfdu>")
            .expect("write manifest");

        let measurement_dir = safe.join("measurement");
        fs::create_dir_all(&measurement_dir).expect("create measurement dir");
        fs::write(
            measurement_dir.join("s1a-iw-grd-vv-20260401t120000.tiff"),
            b"",
        )
        .expect("write measurement tiff");

        let bundle = open_safe_bundle(&safe).expect("open safe bundle");
        assert!(
            matches!(bundle, SafeBundle::Sentinel1(_)),
            "expected SafeBundle::Sentinel1, got Sentinel2"
        );
        if let SafeBundle::Sentinel1(pkg) = bundle {
            assert_eq!(pkg.safe_root, safe);
            assert_eq!(pkg.measurements.len(), 1);
        }
    }
}
