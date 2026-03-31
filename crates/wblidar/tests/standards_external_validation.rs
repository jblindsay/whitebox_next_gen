use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use wblidar::io::{PointReader, PointWriter};
use wblidar::las::header::PointDataFormat;
use wblidar::las::reader::LasReader;
use wblidar::las::writer::WriterConfig;
use wblidar::laz::{LazReader, LazWriter, LazWriterConfig};
use wblidar::point::{GpsTime, PointRecord, Rgb16};
use wblidar::Result;

#[derive(Clone, Copy)]
struct StandardsProfile {
    pdrf: PointDataFormat,
    extra_bytes: u16,
}

struct InteropProfile {
    source: &'static str,
    path: PathBuf,
    pdrf: u8,
    extra_bytes: u16,
    expected_points: Option<u64>,
}

fn temp_workspace() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("wblidar-standards-interop-{}-{nanos}", std::process::id()))
}

fn command_available(program: &str) -> bool {
    Command::new(program)
        .arg("--help")
        .output()
        .map(|_| true)
        .unwrap_or(false)
}

fn now_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn write_json_report(root: &Path, report: &Value) -> Result<()> {
    let default_path = root.join("standards_external_validation_report.json");
    let out_path = std::env::var("WBLIDAR_LAZ_INTEROP_REPORT")
        .map(PathBuf::from)
        .unwrap_or(default_path);

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let bytes = serde_json::to_vec_pretty(report)
        .map_err(|err| wblidar::Error::Projection(format!("failed to serialize interop report: {err}")))?;
    fs::write(&out_path, bytes)?;
    eprintln!("wrote interoperability report: {}", out_path.display());
    Ok(())
}

fn build_point(pdrf: PointDataFormat, idx: u16, extra_bytes: u16) -> PointRecord {
    let mut point = PointRecord {
        x: f64::from(idx),
        y: 100.0 + f64::from(idx),
        z: 200.0 + f64::from(idx),
        intensity: 1000 + idx,
        classification: (idx % 5) as u8,
        return_number: 1,
        number_of_returns: 1,
        user_data: (idx % 255) as u8,
        point_source_id: 500 + idx,
        ..PointRecord::default()
    };

    if pdrf.has_gps_time() {
        point.gps_time = Some(GpsTime(10.0 + f64::from(idx)));
    }

    if pdrf.has_rgb() {
        point.color = Some(Rgb16 {
            red: 100 + idx,
            green: 200 + idx,
            blue: 300 + idx,
        });
    }

    if pdrf.has_nir() {
        point.nir = Some(400 + idx);
    }

    if extra_bytes > 0 {
        for i in 0..usize::from(extra_bytes) {
            point.extra_bytes.data[i] = (u16::try_from(i).unwrap_or(0) as u8).wrapping_add(idx as u8);
        }
        point.extra_bytes.len = extra_bytes as u8;
    }

    point
}

fn write_profile_file(root: &Path, profile: StandardsProfile) -> Result<PathBuf> {
    let path = root.join(format!("standards_pdrf{}_eb{}.laz", profile.pdrf as u8, profile.extra_bytes));
    let mut cfg = LazWriterConfig::default();
    cfg.standards_compliant = true;
    cfg.chunk_size = 2;
    cfg.las = WriterConfig {
        point_data_format: profile.pdrf,
        extra_bytes_per_point: profile.extra_bytes,
        generating_software: "wblidar standards interop test".to_string(),
        ..WriterConfig::default()
    };

    let mut writer = LazWriter::new(File::create(&path)?, cfg)?;
    for idx in 0..3u16 {
        let point = build_point(profile.pdrf, idx, profile.extra_bytes);
        writer.write_point(&point)?;
    }
    writer.finish()?;

    Ok(path)
}

fn validate_internal_read(path: &Path) -> Result<u64> {
    let mut reader = LazReader::new(BufReader::new(File::open(path)?))?;
    let mut point = PointRecord::default();
    let mut count = 0u64;
    while reader.read_point(&mut point)? {
        count += 1;
    }
    Ok(count)
}

fn fixture_directory_from_env() -> Option<PathBuf> {
    std::env::var("WBLIDAR_LAZ_INTEROP_FIXTURE_DIR")
        .ok()
        .and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(PathBuf::from(trimmed))
            }
        })
}

fn parse_min_fixture_profiles_value(raw: Option<&str>) -> Result<Option<usize>> {
    let Some(value) = raw else {
        return Ok(None);
    };

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let parsed = trimmed
        .parse::<usize>()
        .map_err(|err| wblidar::Error::InvalidValue {
            field: "WBLIDAR_LAZ_INTEROP_MIN_FIXTURE_PROFILES",
            detail: format!("expected non-negative integer, got '{trimmed}': {err}"),
        })?;

    Ok(Some(parsed))
}

fn min_fixture_profiles_from_env() -> Result<Option<usize>> {
    let raw = std::env::var("WBLIDAR_LAZ_INTEROP_MIN_FIXTURE_PROFILES").ok();
    parse_min_fixture_profiles_value(raw.as_deref())
}

fn discover_laz_files(root: &Path) -> Result<Vec<PathBuf>> {
    if !root.exists() {
        return Err(wblidar::Error::InvalidValue {
            field: "WBLIDAR_LAZ_INTEROP_FIXTURE_DIR",
            detail: format!("fixture directory does not exist: {}", root.display()),
        });
    }
    if !root.is_dir() {
        return Err(wblidar::Error::InvalidValue {
            field: "WBLIDAR_LAZ_INTEROP_FIXTURE_DIR",
            detail: format!("fixture path is not a directory: {}", root.display()),
        });
    }

    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                stack.push(path);
                continue;
            }
            let is_laz = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("laz"))
                .unwrap_or(false);
            if is_laz {
                out.push(path);
            }
        }
    }

    out.sort();
    Ok(out)
}

fn load_fixture_profiles(root: &Path) -> Result<Vec<InteropProfile>> {
    let mut profiles = Vec::new();
    for path in discover_laz_files(root)? {
        let las = LasReader::new(File::open(&path)?)?;
        let header = las.header();
        profiles.push(InteropProfile {
            source: "fixture",
            path,
            pdrf: header.point_data_format as u8,
            extra_bytes: header.extra_bytes_count,
            expected_points: Some(header.point_count()),
        });
    }
    Ok(profiles)
}

fn run_external_validation(require_external_tools: bool) -> Result<()> {
    let profiles = [
        StandardsProfile {
            pdrf: PointDataFormat::Pdrf0,
            extra_bytes: 0,
        },
        StandardsProfile {
            pdrf: PointDataFormat::Pdrf1,
            extra_bytes: 1,
        },
        StandardsProfile {
            pdrf: PointDataFormat::Pdrf2,
            extra_bytes: 1,
        },
        StandardsProfile {
            pdrf: PointDataFormat::Pdrf3,
            extra_bytes: 2,
        },
        StandardsProfile {
            pdrf: PointDataFormat::Pdrf6,
            extra_bytes: 0,
        },
        StandardsProfile {
            pdrf: PointDataFormat::Pdrf7,
            extra_bytes: 0,
        },
        StandardsProfile {
            pdrf: PointDataFormat::Pdrf8,
            extra_bytes: 2,
        },
    ];

    let has_lasinfo = command_available("lasinfo");
    let has_pdal = command_available("pdal");
    let required_min_fixture_profiles = min_fixture_profiles_from_env()?;
    let mut profiles_report: Vec<Value> = Vec::new();

    let root = temp_workspace();
    fs::create_dir_all(&root)?;

    if !has_lasinfo && !has_pdal {
        let report = json!({
            "schema_version": 1,
            "generated_at_epoch_ms": now_epoch_ms(),
            "strict_mode": require_external_tools,
            "tools": {
                "lasinfo_available": has_lasinfo,
                "pdal_available": has_pdal
            },
            "summary": {
                "executed_profiles": 0,
                "failed_profiles": 0,
                "generated_profiles": 0,
                "fixture_profiles": 0,
                "required_min_fixture_profiles": required_min_fixture_profiles,
                "fixture_profile_requirement_met": required_min_fixture_profiles.map(|min| min == 0),
                "policy_error_count": 0,
                "status": if require_external_tools { "failed_missing_tools" } else { "skipped_missing_tools" }
            },
            "policy_errors": [],
            "profiles": []
        });
        let _ = write_json_report(&root, &report);
        if require_external_tools {
            panic!(
                "strict external interoperability validation requires lasinfo or pdal on PATH"
            );
        }
        eprintln!("skipping: neither lasinfo nor pdal found on PATH");
        let _ = fs::remove_dir_all(&root);
        return Ok(());
    }

    let mut failures: Vec<String> = Vec::new();
    let mut targets: Vec<InteropProfile> = Vec::new();

    for profile in profiles {
        let path = write_profile_file(&root, profile)?;
        targets.push(InteropProfile {
            source: "generated",
            path,
            pdrf: profile.pdrf as u8,
            extra_bytes: profile.extra_bytes,
            expected_points: Some(3),
        });
    }

    if let Some(fixture_root) = fixture_directory_from_env() {
        let fixtures = load_fixture_profiles(&fixture_root)?;
        if fixtures.is_empty() {
            eprintln!(
                "fixture directory set but no .laz files found: {}",
                fixture_root.display()
            );
        }
        targets.extend(fixtures);
    }

    for target in targets {
        let path = target.path;
        let count = validate_internal_read(&path)?;
        let mut profile_errors: Vec<String> = Vec::new();
        let expected_points = target.expected_points.unwrap_or(count);

        if count != expected_points {
            let msg = format!(
                "internal read-back mismatch for {}: expected {} points, got {}",
                path.display(),
                expected_points,
                count
            );
            profile_errors.push(msg.clone());
            failures.push(msg);
        }

        let mut lasinfo_status = "skipped".to_string();
        let mut pdal_status = "skipped".to_string();

        if has_lasinfo {
            let output = Command::new("lasinfo")
                .arg("-i")
                .arg(&path)
                .arg("-nv")
                .output();
            match output {
                Ok(out) if out.status.success() => {
                    lasinfo_status = "ok".to_string();
                }
                Ok(out) => {
                    lasinfo_status = format!("failed(exit={})", out.status);
                    let msg = format!(
                        "lasinfo failed for {} (exit={}): {}",
                        path.display(),
                        out.status,
                        String::from_utf8_lossy(&out.stderr)
                    );
                    profile_errors.push(msg.clone());
                    failures.push(msg);
                }
                Err(err) => {
                    lasinfo_status = "exec_error".to_string();
                    let msg = format!("lasinfo exec error for {}: {}", path.display(), err);
                    profile_errors.push(msg.clone());
                    failures.push(msg);
                }
            }
        }

        if has_pdal {
            let output = Command::new("pdal").arg("info").arg(&path).output();
            match output {
                Ok(out) if out.status.success() => {
                    pdal_status = "ok".to_string();
                }
                Ok(out) => {
                    pdal_status = format!("failed(exit={})", out.status);
                    let msg = format!(
                        "pdal info failed for {} (exit={}): {}",
                        path.display(),
                        out.status,
                        String::from_utf8_lossy(&out.stderr)
                    );
                    profile_errors.push(msg.clone());
                    failures.push(msg);
                }
                Err(err) => {
                    pdal_status = "exec_error".to_string();
                    let msg = format!("pdal exec error for {}: {}", path.display(), err);
                    profile_errors.push(msg.clone());
                    failures.push(msg);
                }
            }
        }

        profiles_report.push(json!({
            "source": target.source,
            "file": path.display().to_string(),
            "pdrf": target.pdrf,
            "extra_bytes_per_point": target.extra_bytes,
            "expected_point_count": target.expected_points,
            "internal_read_count": count,
            "lasinfo": lasinfo_status,
            "pdal_info": pdal_status,
            "status": if profile_errors.is_empty() { "ok" } else { "failed" },
            "errors": profile_errors,
        }));
    }

    let failed_profiles = profiles_report
        .iter()
        .filter(|item| item.get("status").and_then(Value::as_str) == Some("failed"))
        .count();
    let generated_profiles = profiles_report
        .iter()
        .filter(|item| item.get("source").and_then(Value::as_str) == Some("generated"))
        .count();
    let fixture_profiles = profiles_report
        .iter()
        .filter(|item| item.get("source").and_then(Value::as_str) == Some("fixture"))
        .count();
    let mut policy_errors: Vec<String> = Vec::new();
    if let Some(min) = required_min_fixture_profiles {
        if fixture_profiles < min {
            let msg = format!(
                "fixture coverage requirement not met: required at least {min} fixture profiles, got {fixture_profiles}"
            );
            policy_errors.push(msg.clone());
            failures.push(msg);
        }
    }
    let fixture_requirement_met = required_min_fixture_profiles.map(|min| fixture_profiles >= min);
    let status = if failed_profiles == 0 && policy_errors.is_empty() {
        "ok"
    } else {
        "failed"
    };

    let report = json!({
        "schema_version": 1,
        "generated_at_epoch_ms": now_epoch_ms(),
        "strict_mode": require_external_tools,
        "tools": {
            "lasinfo_available": has_lasinfo,
            "pdal_available": has_pdal
        },
        "summary": {
            "executed_profiles": profiles_report.len(),
            "failed_profiles": failed_profiles,
            "generated_profiles": generated_profiles,
            "fixture_profiles": fixture_profiles,
            "required_min_fixture_profiles": required_min_fixture_profiles,
            "fixture_profile_requirement_met": fixture_requirement_met,
            "policy_error_count": policy_errors.len(),
            "status": status
        },
        "policy_errors": policy_errors,
        "profiles": profiles_report
    });
    let _ = write_json_report(&root, &report);

    let _ = fs::remove_dir_all(&root);

    if !failures.is_empty() {
        panic!(
            "external interoperability validation failures:\n{}",
            failures.join("\n")
        );
    }

    Ok(())
}

#[test]
#[ignore = "requires external LAZ consumers (lasinfo and/or pdal) installed"]
fn standards_outputs_validate_in_external_consumers() -> Result<()> {
    run_external_validation(false)
}

#[test]
#[ignore = "strict profile: fails if neither lasinfo nor pdal is installed"]
fn standards_outputs_validate_in_external_consumers_strict() -> Result<()> {
    run_external_validation(true)
}

#[test]
fn discovers_laz_fixture_files_recursively() -> Result<()> {
    let root = temp_workspace();
    let nested = root.join("nested");
    fs::create_dir_all(&nested)?;
    fs::write(root.join("a.laz"), b"x")?;
    fs::write(nested.join("b.LAZ"), b"x")?;
    fs::write(nested.join("ignore.txt"), b"x")?;

    let files = discover_laz_files(&root)?;
    assert_eq!(files.len(), 2);

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

#[test]
fn parses_min_fixture_profiles_value_from_env_text() -> Result<()> {
    assert_eq!(parse_min_fixture_profiles_value(None)?, None);
    assert_eq!(parse_min_fixture_profiles_value(Some(""))?, None);
    assert_eq!(parse_min_fixture_profiles_value(Some("  "))?, None);
    assert_eq!(parse_min_fixture_profiles_value(Some("0"))?, Some(0));
    assert_eq!(parse_min_fixture_profiles_value(Some("12"))?, Some(12));

    let err = parse_min_fixture_profiles_value(Some("bad")).unwrap_err();
    assert!(format!("{err}").contains("WBLIDAR_LAZ_INTEROP_MIN_FIXTURE_PROFILES"));
    Ok(())
}