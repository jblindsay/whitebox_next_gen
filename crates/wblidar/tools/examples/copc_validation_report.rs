use std::env;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use wblidar::copc::reader::CopcReaderMode;
use wblidar::copc::CopcReader;
use wblidar::Result;

const FIXTURE_FILES: [&str; 3] = [
    "fixture_pdrf6.copc.laz",
    "fixture_pdrf7.copc.laz",
    "fixture_pdrf8.copc.laz",
];

fn main() -> Result<()> {
    let fixture_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp/wblidar_copc_validation"));

    let report_path = env::args()
        .nth(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| fixture_dir.join("validation_report.md"));

    let report = build_report(&fixture_dir)?;

    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut out = BufWriter::new(File::create(&report_path)?);
    out.write_all(report.as_bytes())?;
    out.flush()?;

    println!("Wrote COPC validation report: {}", report_path.display());
    Ok(())
}

fn build_report(fixture_dir: &Path) -> Result<String> {
    let mut lines = Vec::<String>::new();

    lines.push("# COPC Validation Report".to_string());
    lines.push(String::new());
    lines.push(format!("Fixture directory: `{}`", fixture_dir.display()));
    lines.push(String::new());
    lines.push("## Internal strict-reader checks".to_string());
    lines.push(String::new());
    lines.push("| File | Exists | Strict read | Points | Header point_count | Notes |".to_string());
    lines.push("|---|---|---|---:|---:|---|".to_string());

    for name in FIXTURE_FILES {
        let path = fixture_dir.join(name);
        if !path.exists() {
            lines.push(format!("| {} | no | n/a | n/a | n/a | missing fixture file |", name));
            continue;
        }

        let (strict_read, points_len, header_count, notes) = match validate_fixture(&path) {
            Ok((points_len, header_count)) => ("pass", points_len.to_string(), header_count.to_string(), String::new()),
            Err(e) => ("fail", "n/a".to_string(), "n/a".to_string(), e.to_string()),
        };

        lines.push(format!(
            "| {} | yes | {} | {} | {} | {} |",
            name,
            strict_read,
            points_len,
            header_count,
            escape_pipes(&notes)
        ));
    }

    lines.push(String::new());
    lines.push("## External validation checklist".to_string());
    lines.push(String::new());
    lines.push("Run validate.copc.io and at least one independent consumer (e.g., PDAL, CloudCompare, QGIS), then fill this table.".to_string());
    lines.push(String::new());
    lines.push("| File | validate.copc.io | External consumer | Notes |".to_string());
    lines.push("|---|---|---|---|".to_string());
    for name in FIXTURE_FILES {
        lines.push(format!("| {} | pending | pending | |", name));
    }

    Ok(lines.join("\n"))
}

fn validate_fixture(path: &Path) -> Result<(usize, u64)> {
    let input = BufReader::new(File::open(path)?);
    let mut reader = CopcReader::new_with_mode(input, CopcReaderMode::Strict)?;
    let points = reader.read_all_nodes()?;
    let header_count = reader.header().point_count();
    Ok((points.len(), header_count))
}

fn escape_pipes(s: &str) -> String {
    s.replace('|', "\\|")
}
