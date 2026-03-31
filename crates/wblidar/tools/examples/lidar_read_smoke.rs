use std::env;
use std::fs;
use std::path::Path;

use std::collections::BTreeMap;

use wblidar::frontend::PointCloud;

fn main() {
    let mut args = env::args().skip(1);
    let mut debug = false;
    let mut summary = false;
    let mut paths = Vec::new();

    while let Some(arg) = args.next() {
        if arg == "--debug" {
            debug = true;
        } else if arg == "--summary" {
            summary = true;
        } else if arg == "--list" {
            let list_path = args.next().unwrap_or_default();
            if list_path.is_empty() {
                eprintln!("--list requires a file path");
                std::process::exit(2);
            }
            match read_paths_from_list(&list_path) {
                Ok(mut from_file) => paths.append(&mut from_file),
                Err(msg) => {
                    eprintln!("failed to read list '{}': {}", list_path, msg);
                    std::process::exit(2);
                }
            }
        } else {
            paths.push(arg);
        }
    }

    let mut had_input = false;
    let mut ok_count = 0usize;
    let mut err_count = 0usize;
    let mut err_groups: BTreeMap<String, usize> = BTreeMap::new();

    for path in paths {
        had_input = true;
        match PointCloud::read(&path) {
            Ok(cloud) => {
                println!("OK\t{}\t{}", cloud.points.len(), path);
                ok_count += 1;
            }
            Err(err) => {
                println!("ERR\t{}\t{}", err, path);
                err_count += 1;
                *err_groups.entry(err.to_string()).or_insert(0) += 1;
                if debug {
                    println!("ERRDBG\t{:?}\t{}", err, path);
                    let mut source = std::error::Error::source(&err);
                    while let Some(s) = source {
                        println!("CAUSE\t{}\t{}", s, path);
                        source = s.source();
                    }
                }
            }
        }
    }

    if !had_input {
        eprintln!(
            "Usage: cargo run -p wblidar --example lidar_read_smoke -- [--debug] [--summary] [--list <paths.txt>] <file1> [file2 ...]"
        );
        std::process::exit(2);
    }

    if summary || ok_count + err_count > 1 {
        println!("SUMMARY\tok={}\terr={}\ttotal={}", ok_count, err_count, ok_count + err_count);
        if !err_groups.is_empty() {
            println!("SUMMARY_ERRORS");
            for (msg, count) in err_groups {
                println!("SUMMARY_ERR\t{}\t{}", count, msg);
            }
        }
    }
}

fn read_paths_from_list(list_path: &str) -> Result<Vec<String>, String> {
    let path = Path::new(list_path);
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        out.push(trimmed.to_string());
    }
    Ok(out)
}
