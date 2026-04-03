//! Unified sensor bundle detection and opener.
//!
//! This module provides additive convenience APIs that detect and open common
//! remote sensing package families supported by `wbraster`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use flate2::read::GzDecoder;
use tar::Archive;
use zip::read::ZipArchive;

use crate::error::{RasterError, Result};

use super::dimap_bundle::DimapBundle;
use super::iceye_bundle::IceyeBundle;
use super::landsat_bundle::LandsatBundle;
use super::maxar_worldview_bundle::MaxarWorldViewBundle;
use super::planetscope_bundle::PlanetScopeBundle;
use super::radarsat2_bundle::Radarsat2Bundle;
use super::rcm_bundle::RcmBundle;
use super::safe_bundle::{SafeBundle, SafeMission, detect_safe_mission, open_safe_bundle};

/// Sensor bundle family represented by a package root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorBundleFamily {
    /// Sentinel-1 SAFE bundle.
    Sentinel1Safe,
    /// Sentinel-2 SAFE bundle.
    Sentinel2Safe,
    /// Landsat Collection scene bundle.
    Landsat,
    /// ICEYE scene bundle.
    Iceye,
    /// PlanetScope scene bundle.
    PlanetScope,
    /// SPOT/Pleiades DIMAP scene bundle.
    Dimap,
    /// Maxar/WorldView scene bundle.
    MaxarWorldView,
    /// RADARSAT-2 scene bundle.
    Radarsat2,
    /// RCM scene bundle.
    Rcm,
    /// Unknown bundle family.
    Unknown,
}

/// Unified bundle enum returned by [`open_sensor_bundle`].
#[derive(Debug, Clone)]
pub enum SensorBundle {
    /// Sentinel SAFE bundle.
    Safe(SafeBundle),
    /// Landsat bundle.
    Landsat(LandsatBundle),
    /// ICEYE bundle.
    Iceye(IceyeBundle),
    /// PlanetScope bundle.
    PlanetScope(PlanetScopeBundle),
    /// SPOT/Pleiades DIMAP bundle.
    Dimap(DimapBundle),
    /// Maxar/WorldView bundle.
    MaxarWorldView(MaxarWorldViewBundle),
    /// RADARSAT-2 bundle.
    Radarsat2(Radarsat2Bundle),
    /// RCM bundle.
    Rcm(RcmBundle),
}

/// Result of opening a sensor bundle from either a directory root or an
/// archive file.
#[derive(Debug, Clone)]
pub struct OpenedSensorBundle {
    /// Parsed sensor bundle.
    pub bundle: SensorBundle,
    /// Temporary extraction root when opened from an archive path.
    ///
    /// When this is `Some`, callers can remove the directory after use.
    pub extracted_root: Option<PathBuf>,
}

/// Detect the sensor bundle family of a package root.
///
/// Detection prioritizes explicit metadata markers, then falls back to simple
/// file-pattern heuristics.
pub fn detect_sensor_bundle_family(bundle_root: impl AsRef<Path>) -> Result<SensorBundleFamily> {
    let bundle_root = bundle_root.as_ref();
    if !bundle_root.is_dir() {
        return Err(RasterError::Other(format!(
            "bundle root is not a directory: {}",
            bundle_root.display()
        )));
    }

    let mut files = Vec::new();
    collect_files_recursive(bundle_root, &mut files)?;

    let mut has_mtl = false;
    let mut has_tiff = false;
    let mut has_product_xml = false;
    let mut rs2_marker = false;
    let mut rcm_marker = false;
    let mut iceye_marker = false;
    let mut planetscope_marker = false;
    let mut dimap_marker = false;
    let mut maxar_marker = false;
    let mut has_json = false;

    for p in files {
        let filename = p
            .file_name()
            .map(|n| n.to_string_lossy().to_ascii_uppercase())
            .unwrap_or_default();

        if filename.ends_with("_MTL.TXT") {
            has_mtl = true;
        }

        if filename.contains("UDM2") || filename.contains("PLANET") {
            planetscope_marker = true;
        }

        if filename.starts_with("DIM_") && filename.ends_with(".XML") {
            dimap_marker = true;
        }

        if filename.ends_with(".IMD") || filename.contains("WORLDVIEW") || filename.contains("MAXAR") {
            maxar_marker = true;
        }

        if p.extension()
            .map(|e| {
                let ext = e.to_string_lossy();
                ext.eq_ignore_ascii_case("tif") || ext.eq_ignore_ascii_case("tiff")
            })
            .unwrap_or(false)
        {
            has_tiff = true;
        }

        if filename == "PRODUCT.XML" {
            has_product_xml = true;
            if let Ok(text) = fs::read_to_string(&p) {
                let u = text.to_ascii_uppercase();
                if u.contains("RADARSAT-2") || u.contains("RS2") {
                    rs2_marker = true;
                }
                if u.contains("RCM") || u.contains("RADARSAT CONSTELLATION") {
                    rcm_marker = true;
                }
                if u.contains("DIMAP") || u.contains("PLEIADES") || u.contains("SPOT") {
                    dimap_marker = true;
                }
                if u.contains("WORLDVIEW") || u.contains("MAXAR") || u.contains("GEOEYE") || u.contains("QUICKBIRD") {
                    maxar_marker = true;
                }
            }
        }

        if filename.contains("ICEYE") {
            iceye_marker = true;
        }

        if p.extension()
            .map(|e| e.to_string_lossy().eq_ignore_ascii_case("xml"))
            .unwrap_or(false)
            && !filename.eq_ignore_ascii_case("PRODUCT.XML")
        {
            if let Ok(text) = fs::read_to_string(&p) {
                let u = text.to_ascii_uppercase();
                if u.contains("ICEYE") {
                    iceye_marker = true;
                }
                if u.contains("PLANET") {
                    planetscope_marker = true;
                }
                if u.contains("DIMAP") || u.contains("PLEIADES") || u.contains("SPOT") {
                    dimap_marker = true;
                }
                if u.contains("WORLDVIEW") || u.contains("MAXAR") || u.contains("GEOEYE") || u.contains("QUICKBIRD") {
                    maxar_marker = true;
                }
            }
        }

        if p.extension()
            .map(|e| e.to_string_lossy().eq_ignore_ascii_case("json"))
            .unwrap_or(false)
        {
            has_json = true;
            if let Ok(text) = fs::read_to_string(&p) {
                let u = text.to_ascii_uppercase();
                if u.contains("PLANET") || u.contains("PSSCENE") {
                    planetscope_marker = true;
                }
            }
        }

        if filename.contains("ANALYTIC") {
            planetscope_marker = true;
        }
    }

    if has_mtl {
        return Ok(SensorBundleFamily::Landsat);
    }

    if has_product_xml {
        if rs2_marker {
            return Ok(SensorBundleFamily::Radarsat2);
        }
        if rcm_marker {
            return Ok(SensorBundleFamily::Rcm);
        }
    }

    if has_tiff && iceye_marker {
        return Ok(SensorBundleFamily::Iceye);
    }

    if has_tiff && planetscope_marker {
        return Ok(SensorBundleFamily::PlanetScope);
    }

    if has_tiff && has_json {
        return Ok(SensorBundleFamily::PlanetScope);
    }

    if dimap_marker {
        return Ok(SensorBundleFamily::Dimap);
    }

    if maxar_marker {
        return Ok(SensorBundleFamily::MaxarWorldView);
    }

    // SAFE bundles are detected after non-SAFE families to avoid JP2-based
    // false positives on non-SAFE products like DIMAP.
    let root_name = bundle_root
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_uppercase())
        .unwrap_or_default();
    let looks_like_safe_root = root_name.ends_with(".SAFE") || bundle_root.join("manifest.safe").is_file();
    if looks_like_safe_root {
        if let Ok(safe_mission) = detect_safe_mission(bundle_root) {
            match safe_mission {
                SafeMission::Sentinel1 => return Ok(SensorBundleFamily::Sentinel1Safe),
                SafeMission::Sentinel2 => return Ok(SensorBundleFamily::Sentinel2Safe),
                SafeMission::Unknown => {}
            }
        }
    }

    Ok(SensorBundleFamily::Unknown)
}

/// Open a sensor bundle root and return a family-specific package enum.
pub fn open_sensor_bundle(bundle_root: impl AsRef<Path>) -> Result<SensorBundle> {
    let bundle_root = bundle_root.as_ref();
    match detect_sensor_bundle_family(bundle_root)? {
        SensorBundleFamily::Sentinel1Safe | SensorBundleFamily::Sentinel2Safe => {
            Ok(SensorBundle::Safe(open_safe_bundle(bundle_root)?))
        }
        SensorBundleFamily::Landsat => Ok(SensorBundle::Landsat(LandsatBundle::open(bundle_root)?)),
        SensorBundleFamily::Iceye => Ok(SensorBundle::Iceye(IceyeBundle::open(bundle_root)?)),
        SensorBundleFamily::PlanetScope => {
            Ok(SensorBundle::PlanetScope(PlanetScopeBundle::open(bundle_root)?))
        }
        SensorBundleFamily::Dimap => Ok(SensorBundle::Dimap(DimapBundle::open(bundle_root)?)),
        SensorBundleFamily::MaxarWorldView => {
            Ok(SensorBundle::MaxarWorldView(MaxarWorldViewBundle::open(bundle_root)?))
        }
        SensorBundleFamily::Radarsat2 => {
            Ok(SensorBundle::Radarsat2(Radarsat2Bundle::open(bundle_root)?))
        }
        SensorBundleFamily::Rcm => Ok(SensorBundle::Rcm(RcmBundle::open(bundle_root)?)),
        SensorBundleFamily::Unknown => Err(RasterError::Other(format!(
            "unable to determine sensor bundle family for '{}'",
            bundle_root.display()
        ))),
    }
}

/// Detect sensor bundle family from either a directory root or a supported
/// archive file path (`.zip`, `.tar`, `.tar.gz`, `.tgz`).
pub fn detect_sensor_bundle_family_path(path: impl AsRef<Path>) -> Result<SensorBundleFamily> {
    let path = path.as_ref();
    if path.is_dir() {
        return detect_sensor_bundle_family(path);
    }
    if !path.is_file() {
        return Err(RasterError::Other(format!(
            "bundle path is neither file nor directory: {}",
            path.display()
        )));
    }

    if !is_supported_archive(path) {
        return Err(RasterError::Other(format!(
            "unsupported bundle file type '{}'; expected .zip, .tar, .tar.gz, or .tgz",
            path.display()
        )));
    }

    let extraction_root = extract_archive_to_temp(path)?;
    detect_sensor_bundle_family(&extraction_root)
}

/// Open a sensor bundle from either a directory root or a supported archive
/// file path (`.zip`, `.tar`, `.tar.gz`, `.tgz`).
///
/// For archive inputs, assets are extracted to a temporary directory and the
/// path is returned in [`OpenedSensorBundle::extracted_root`] for optional
/// caller-managed cleanup.
pub fn open_sensor_bundle_path(path: impl AsRef<Path>) -> Result<OpenedSensorBundle> {
    let path = path.as_ref();
    if path.is_dir() {
        return Ok(OpenedSensorBundle {
            bundle: open_sensor_bundle(path)?,
            extracted_root: None,
        });
    }
    if !path.is_file() {
        return Err(RasterError::Other(format!(
            "bundle path is neither file nor directory: {}",
            path.display()
        )));
    }
    if !is_supported_archive(path) {
        return Err(RasterError::Other(format!(
            "unsupported bundle file type '{}'; expected .zip, .tar, .tar.gz, or .tgz",
            path.display()
        )));
    }

    let extraction_root = extract_archive_to_temp(path)?;
    let bundle = open_sensor_bundle(&extraction_root)?;
    Ok(OpenedSensorBundle {
        bundle,
        extracted_root: Some(extraction_root),
    })
}

fn collect_files_recursive(root: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
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

fn is_supported_archive(path: &Path) -> bool {
    let lower = path
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    lower.ends_with(".zip")
        || lower.ends_with(".tar")
        || lower.ends_with(".tar.gz")
        || lower.ends_with(".tgz")
}

fn extract_archive_to_temp(archive_path: &Path) -> Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp_root = std::env::temp_dir().join(format!(
        "wbraster_sensor_bundle_{}_{}",
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(&temp_root)?;

    let lower = archive_path
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();

    if lower.ends_with(".zip") {
        extract_zip_archive(archive_path, &temp_root)?;
    } else if lower.ends_with(".tar") {
        let file = fs::File::open(archive_path)?;
        let mut archive = Archive::new(file);
        archive.unpack(&temp_root)?;
    } else if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        let file = fs::File::open(archive_path)?;
        let gz = GzDecoder::new(file);
        let mut archive = Archive::new(gz);
        archive.unpack(&temp_root)?;
    } else {
        return Err(RasterError::Other(format!(
            "unsupported archive extension '{}': expected .zip, .tar, .tar.gz, or .tgz",
            archive_path.display()
        )));
    }

    Ok(temp_root)
}

fn extract_zip_archive(archive_path: &Path, output_root: &Path) -> Result<()> {
    let file = fs::File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| RasterError::Other(format!("unable to open zip archive: {e}")))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| RasterError::Other(format!("unable to read zip entry {i}: {e}")))?;
        let Some(rel) = entry.enclosed_name().map(|p| p.to_path_buf()) else {
            continue;
        };
        let outpath = output_root.join(rel);

        if entry.is_dir() {
            fs::create_dir_all(&outpath)?;
            continue;
        }

        if let Some(parent) = outpath.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut outfile = fs::File::create(&outpath)?;
        std::io::copy(&mut entry, &mut outfile)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    use tar::Builder as TarBuilder;
    use zip::write::SimpleFileOptions;

    #[test]
    fn detects_landsat_bundle_from_mtl() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("LANDSAT");
        fs::create_dir_all(&root).expect("create root");
        fs::write(root.join("LC09_TEST_MTL.txt"), "SPACECRAFT_ID = \"LANDSAT_9\"")
            .expect("write mtl");

        let fam = detect_sensor_bundle_family(&root).expect("detect");
        assert_eq!(fam, SensorBundleFamily::Landsat);
    }

    #[test]
    fn detects_iceye_bundle_from_xml_marker() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("ICEYE");
        fs::create_dir_all(&root).expect("create root");
        fs::write(root.join("metadata.xml"), "<product>ICEYE</product>").expect("xml");
        fs::write(root.join("ICEYE_TEST_VV.tif"), b"").expect("tif");

        let fam = detect_sensor_bundle_family(&root).expect("detect");
        assert_eq!(fam, SensorBundleFamily::Iceye);
    }

    #[test]
    fn detects_planetscope_bundle_from_udm2_marker() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("PLANET");
        fs::create_dir_all(&root).expect("create root");
        fs::write(root.join("scene_udm2.tif"), b"").expect("udm2");
        fs::write(root.join("scene_analytic_b3.tif"), b"").expect("b3");

        let fam = detect_sensor_bundle_family(&root).expect("detect");
        assert_eq!(fam, SensorBundleFamily::PlanetScope);
    }

    #[test]
    fn detects_dimap_bundle_from_dim_xml_marker() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("DIMAP");
        fs::create_dir_all(&root).expect("create root");
        fs::write(root.join("DIM_PHR1A_PMS_001.XML"), "<Dimap_Document>DIMAP</Dimap_Document>")
            .expect("xml");

        let fam = detect_sensor_bundle_family(&root).expect("detect");
        assert_eq!(fam, SensorBundleFamily::Dimap);
    }

    #[test]
    fn detects_maxar_bundle_from_imd_marker() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("MAXAR");
        fs::create_dir_all(&root).expect("create root");
        fs::write(root.join("scene.IMD"), "satId = \"WV03\"").expect("imd");

        let fam = detect_sensor_bundle_family(&root).expect("detect");
        assert_eq!(fam, SensorBundleFamily::MaxarWorldView);
    }

    #[test]
    fn detects_radarsat2_bundle_from_product_xml_marker() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("RS2");
        fs::create_dir_all(&root).expect("create root");
        fs::write(root.join("product.xml"), "<product>RADARSAT-2</product>").expect("xml");
        fs::write(root.join("img_HH.tif"), b"").expect("tif");

        let fam = detect_sensor_bundle_family(&root).expect("detect");
        assert_eq!(fam, SensorBundleFamily::Radarsat2);
    }

    #[test]
    fn detects_rcm_bundle_from_product_xml_marker() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("RCM");
        fs::create_dir_all(&root).expect("create root");
        fs::write(root.join("product.xml"), "<product>RCM</product>").expect("xml");
        fs::write(root.join("img_VV.tif"), b"").expect("tif");

        let fam = detect_sensor_bundle_family(&root).expect("detect");
        assert_eq!(fam, SensorBundleFamily::Rcm);
    }

    #[test]
    fn open_sensor_bundle_returns_landsat_variant() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("L8");
        fs::create_dir_all(&root).expect("create root");

        let mtl = r#"
SPACECRAFT_ID = "LANDSAT_8"
PROCESSING_LEVEL = "L2SP"
WRS_PATH = 1
WRS_ROW = 1
"#;
        fs::write(root.join("LC08_TEST_MTL.txt"), mtl).expect("mtl");
        fs::write(root.join("LC08_TEST_SR_B2.TIF"), b"").expect("band");

        let bundle = open_sensor_bundle(&root).expect("open");
        assert!(matches!(bundle, SensorBundle::Landsat(_)));
    }

    #[test]
    fn open_sensor_bundle_returns_planetscope_variant() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("PLANET");
        fs::create_dir_all(&root).expect("create root");

        fs::write(root.join("metadata.json"), r#"{"id":"PSScene_01"}"#).expect("meta");
        fs::write(root.join("scene_analytic_b3.tif"), b"").expect("band");

        let bundle = open_sensor_bundle(&root).expect("open");
        assert!(matches!(bundle, SensorBundle::PlanetScope(_)));
    }

    #[test]
    fn open_sensor_bundle_returns_dimap_variant() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("DIMAP");
        fs::create_dir_all(&root).expect("create root");

        fs::write(
            root.join("DIM_PHR1A_PMS_001.XML"),
            "<Dimap_Document><MISSION>PLEIADES</MISSION></Dimap_Document>",
        )
        .expect("xml");
        fs::write(root.join("IMG_B1.JP2"), b"").expect("band");

        let bundle = open_sensor_bundle(&root).expect("open");
        assert!(matches!(bundle, SensorBundle::Dimap(_)));
    }

    #[test]
    fn open_sensor_bundle_returns_maxar_variant() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("MAXAR");
        fs::create_dir_all(&root).expect("create root");

        fs::write(root.join("scene.IMD"), "satId = \"WV03\"").expect("imd");
        fs::write(root.join("IMG_BAND_R.TIF"), b"").expect("band");

        let bundle = open_sensor_bundle(&root).expect("open");
        assert!(matches!(bundle, SensorBundle::MaxarWorldView(_)));
    }

    #[test]
    fn open_sensor_bundle_path_supports_zip_archive() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let zip_path = tmp.path().join("landsat.zip");

        {
            let file = fs::File::create(&zip_path).expect("create zip");
            let mut zip = zip::ZipWriter::new(file);
            let options = SimpleFileOptions::default();

            zip.start_file("LC08_TEST/LC08_TEST_MTL.txt", options)
                .expect("start mtl");
            zip.write_all(
                br#"SPACECRAFT_ID = "LANDSAT_8"
PROCESSING_LEVEL = "L2SP"
WRS_PATH = 1
WRS_ROW = 1
"#,
            )
            .expect("write mtl");

            zip.start_file("LC08_TEST/LC08_TEST_SR_B2.TIF", options)
                .expect("start band");
            zip.write_all(b"dummy").expect("write band");

            zip.finish().expect("finish zip");
        }

        let opened = open_sensor_bundle_path(&zip_path).expect("open zip bundle");
        assert!(matches!(opened.bundle, SensorBundle::Landsat(_)));
        assert!(opened.extracted_root.is_some());
    }

    #[test]
    fn open_sensor_bundle_path_supports_tar_gz_archive() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let tgz_path = tmp.path().join("landsat.tar.gz");

        {
            let file = fs::File::create(&tgz_path).expect("create tgz");
            let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
            let mut tar = TarBuilder::new(enc);

            let mtl_data = br#"SPACECRAFT_ID = "LANDSAT_9"
PROCESSING_LEVEL = "L2SP"
WRS_PATH = 2
WRS_ROW = 3
"#;
            let mut mtl_header = tar::Header::new_gnu();
            mtl_header.set_size(mtl_data.len() as u64);
            mtl_header.set_mode(0o644);
            mtl_header.set_cksum();
            tar.append_data(
                &mut mtl_header,
                "LC09_TEST/LC09_TEST_MTL.txt",
                &mtl_data[..],
            )
            .expect("append mtl");

            let tif_data = b"dummy";
            let mut tif_header = tar::Header::new_gnu();
            tif_header.set_size(tif_data.len() as u64);
            tif_header.set_mode(0o644);
            tif_header.set_cksum();
            tar.append_data(
                &mut tif_header,
                "LC09_TEST/LC09_TEST_SR_B2.TIF",
                &tif_data[..],
            )
            .expect("append tif");

            tar.finish().expect("finish tar");
        }

        let opened = open_sensor_bundle_path(&tgz_path).expect("open tgz bundle");
        assert!(matches!(opened.bundle, SensorBundle::Landsat(_)));
        assert!(opened.extracted_root.is_some());
    }
}
