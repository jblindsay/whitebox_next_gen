use std::fs;
use std::path::{Path, PathBuf};

use wbprojection::{
    EpsgIdentifyPolicy,
    identify_epsg_from_wkt_report,
};

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
                "manifest parse error at line {}: expected at least 4 comma-separated columns",
                line_no + 1
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

fn main() {
    let mut args = std::env::args();
    let _bin = args.next();
    let Some(manifest_arg) = args.next() else {
        eprintln!(
            "Usage: cargo run --example epsg_identify_report_batch -- <manifest.csv> [output.csv]"
        );
        std::process::exit(2);
    };

    let output_arg = args.next();
    let manifest_path = PathBuf::from(manifest_arg);
    let base_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));

    let rows = match parse_manifest(&manifest_path) {
        Ok(v) => v,
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(1);
        }
    };

    let mut out = String::new();
    out.push_str("name,file,expected_lenient,actual_lenient,pass_lenient,expected_strict,actual_strict,pass_strict,ambiguous_lenient,ambiguous_strict,top1_code,top1_score,top2_code,top2_score\n");

    for row in rows {
        let path = base_dir.join(&row.file);
        let wkt = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                out.push_str(&format!(
                    "{},{},{},,false,{},,false,,,,,,\n",
                    row.name,
                    row.file,
                    row.expected_lenient
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    row.expected_strict
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                ));
                eprintln!("warning: failed to read '{}': {e}", path.display());
                continue;
            }
        };

        let lenient = identify_epsg_from_wkt_report(&wkt, EpsgIdentifyPolicy::Lenient);
        let strict = identify_epsg_from_wkt_report(&wkt, EpsgIdentifyPolicy::Strict);

        let actual_lenient = lenient.as_ref().and_then(|r| r.resolved_code);
        let actual_strict = strict.as_ref().and_then(|r| r.resolved_code);
        let pass_lenient = row.expected_lenient == actual_lenient;
        let pass_strict = row.expected_strict == actual_strict;

        let ambiguous_lenient = lenient.as_ref().map(|r| r.ambiguous).unwrap_or(false);
        let ambiguous_strict = strict.as_ref().map(|r| r.ambiguous).unwrap_or(false);

        let top1_code = lenient
            .as_ref()
            .and_then(|r| r.top_candidates.first().map(|c| c.code));
        let top1_score = lenient
            .as_ref()
            .and_then(|r| r.top_candidates.first().map(|c| c.total_score));
        let top2_code = lenient
            .as_ref()
            .and_then(|r| r.top_candidates.get(1).map(|c| c.code));
        let top2_score = lenient
            .as_ref()
            .and_then(|r| r.top_candidates.get(1).map(|c| c.total_score));

        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{:.6},{},{:.6}\n",
            row.name,
            row.file,
            row.expected_lenient
                .map(|v| v.to_string())
                .unwrap_or_default(),
            actual_lenient.map(|v| v.to_string()).unwrap_or_default(),
            pass_lenient,
            row.expected_strict
                .map(|v| v.to_string())
                .unwrap_or_default(),
            actual_strict.map(|v| v.to_string()).unwrap_or_default(),
            pass_strict,
            ambiguous_lenient,
            ambiguous_strict,
            top1_code.map(|v| v.to_string()).unwrap_or_default(),
            top1_score.unwrap_or(0.0),
            top2_code.map(|v| v.to_string()).unwrap_or_default(),
            top2_score.unwrap_or(0.0),
        ));
    }

    if let Some(out_path) = output_arg {
        if let Err(e) = fs::write(&out_path, out.as_bytes()) {
            eprintln!("failed to write '{}': {e}", out_path);
            std::process::exit(1);
        }
    } else {
        print!("{out}");
    }
}
