use std::fs;
use std::path::{Path, PathBuf};

const CORPUS_DIR: &str = "src/tests/data/wkt_corpus";

fn sanitize_name(s: &str) -> String {
    let mut out = String::new();
    let mut last_us = false;
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_us = false;
        } else if !last_us {
            out.push('_');
            last_us = true;
        }
    }
    out.trim_matches('_').to_string()
}

fn manifest_filename(profile_name: &str) -> String {
    let profile = sanitize_name(profile_name);
    if profile.ends_with("_manifest") {
        format!("{profile}.csv")
    } else {
        format!("{profile}_manifest.csv")
    }
}

fn usage() {
    eprintln!(
        "Usage: cargo run --example epsg_identify_manifest_template -- <profile_name> [sample_file1 sample_file2 ...] [--dry-run] [--force]"
    );
}

fn main() {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        usage();
        std::process::exit(2);
    }

    let dry_run = args.iter().any(|a| a == "--dry-run");
    let force = args.iter().any(|a| a == "--force");
    args.retain(|a| a != "--dry-run" && a != "--force");

    if args.is_empty() {
        usage();
        std::process::exit(2);
    }

    let profile_name = args.remove(0);
    let samples = args;

    let manifest_name = manifest_filename(&profile_name);
    let manifest_path = PathBuf::from(CORPUS_DIR).join(&manifest_name);

    if manifest_path.exists() && !force {
        eprintln!(
            "manifest already exists: {} (use --force to overwrite)",
            manifest_path.display()
        );
        std::process::exit(1);
    }

    let mut content = String::new();
    content.push_str("name,file,expected_lenient,expected_strict,notes\n");

    if samples.is_empty() {
        content.push_str("example_case_1,example_case_1.prj,,,fill expected codes\n");
        content.push_str("example_case_2,example_case_2.wkt,,,fill expected codes\n");
    } else {
        for sample in samples {
            let display_name = Path::new(&sample)
                .file_stem()
                .and_then(|s| s.to_str())
                .map(sanitize_name)
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "sample_case".to_string());

            content.push_str(&format!(
                "{},{},,,fill expected codes\n",
                display_name,
                sample
            ));
        }
    }

    if dry_run {
        println!("--- {} ---", manifest_path.display());
        print!("{content}");
        return;
    }

    if let Err(e) = fs::write(&manifest_path, content.as_bytes()) {
        eprintln!("failed to write '{}': {e}", manifest_path.display());
        std::process::exit(1);
    }

    println!("created {}", manifest_path.display());
}
