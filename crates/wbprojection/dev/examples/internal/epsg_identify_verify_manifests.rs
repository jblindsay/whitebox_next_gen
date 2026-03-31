use std::fs;
use std::path::{Path, PathBuf};

use wbprojection::{
    EpsgIdentifyPolicy,
    identify_epsg_from_wkt_report,
};

const CORPUS_DIR: &str = "src/tests/data/wkt_corpus";

#[derive(Debug)]
struct ManifestRow {
    name: String,
    file: String,
    expected_lenient: Option<u32>,
    expected_strict: Option<u32>,
}

fn parse_opt_u32(s: &str) -> Option<u32> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        t.parse::<u32>().ok()
    }
}

fn parse_manifest(path: &Path) -> Result<Vec<ManifestRow>, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("failed to read manifest '{}': {e}", path.display()))?;

    let mut rows = Vec::new();
    for (line_no, line) in text.lines().enumerate() {
        if line_no == 0 {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let cols: Vec<&str> = trimmed.split(',').collect();
        if cols.len() < 4 {
            return Err(format!(
                "manifest parse error at line {} in '{}': expected at least 4 comma-separated columns",
                line_no + 1,
                path.display(),
            ));
        }
        rows.push(ManifestRow {
            name: cols[0].trim().to_string(),
            file: cols[1].trim().to_string(),
            expected_lenient: parse_opt_u32(cols[2]),
            expected_strict: parse_opt_u32(cols[3]),
        });
    }
    Ok(rows)
}

fn verify_manifest(path: &Path) -> Result<(usize, usize), String> {
    let rows = parse_manifest(path)?;
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));

    let mut checked = 0usize;
    let mut failed = 0usize;

    for row in rows {
        checked += 1;
        let wkt_path = base_dir.join(&row.file);
        let wkt = fs::read_to_string(&wkt_path)
            .map_err(|e| format!("failed to read sample '{}': {e}", wkt_path.display()))?;

        let lenient = identify_epsg_from_wkt_report(&wkt, EpsgIdentifyPolicy::Lenient)
            .and_then(|r| r.resolved_code);
        let strict = identify_epsg_from_wkt_report(&wkt, EpsgIdentifyPolicy::Strict)
            .and_then(|r| r.resolved_code);

        let pass_lenient = lenient == row.expected_lenient;
        let pass_strict = strict == row.expected_strict;

        if !pass_lenient || !pass_strict {
            failed += 1;
            println!(
                "FAIL {} | case={} | expected(lenient={:?}, strict={:?}) got(lenient={:?}, strict={:?})",
                path.display(),
                row.name,
                row.expected_lenient,
                row.expected_strict,
                lenient,
                strict,
            );
        }
    }

    Ok((checked, failed))
}

fn discover_manifests() -> Result<Vec<PathBuf>, String> {
    let dir = PathBuf::from(CORPUS_DIR);
    let entries = fs::read_dir(&dir)
        .map_err(|e| format!("failed to read corpus dir '{}': {e}", dir.display()))?;

    let mut manifests = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read dir entry: {e}"))?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if name.ends_with("manifest.csv") {
            manifests.push(path);
        }
    }

    manifests.sort();
    if manifests.is_empty() {
        return Err(format!("no manifest files found under '{}'", dir.display()));
    }
    Ok(manifests)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let manifests: Vec<PathBuf> = if args.is_empty() {
        match discover_manifests() {
            Ok(v) => v,
            Err(msg) => {
                eprintln!("ERROR: {msg}");
                std::process::exit(2);
            }
        }
    } else {
        args.iter().map(PathBuf::from).collect()
    };

    let mut total_checked = 0usize;
    let mut total_failed = 0usize;

    for manifest in manifests {
        match verify_manifest(&manifest) {
            Ok((checked, failed)) => {
                total_checked += checked;
                total_failed += failed;
                println!(
                    "OK {} | checked={} | failed={}",
                    manifest.display(),
                    checked,
                    failed
                );
            }
            Err(msg) => {
                eprintln!("ERROR: {msg}");
                std::process::exit(2);
            }
        }
    }

    println!(
        "SUMMARY checked={} failed={}",
        total_checked,
        total_failed
    );

    if total_failed > 0 {
        std::process::exit(1);
    }
}
