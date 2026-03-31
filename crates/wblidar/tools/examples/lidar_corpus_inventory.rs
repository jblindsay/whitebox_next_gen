use std::collections::HashMap;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use wblidar::frontend::read_with_diagnostics;
use wblidar::copc::{CopcHierarchyParseMode, CopcReader, COPC_INFO_RECORD_ID, COPC_USER_ID};
use wblidar::las::LasReader;
use wblidar::laz::{parse_laszip_vlr, LaszipVlrInfo};

const REPORT_SCHEMA_VERSION: u32 = 2;
const DEFAULT_TOP_N: usize = 20;
const DEFAULT_SAMPLE_CAP: usize = 30;

const COPC_LAYOUT_SINGLE_PAGE: &str = "single-page";
const COPC_LAYOUT_PAGED: &str = "paged";
const COPC_LAYOUT_MIXED_ROOT: &str = "mixed-root";

#[derive(Debug, Clone)]
struct CliConfig {
    root: PathBuf,
    max_files: Option<usize>,
    smoke_read: bool,
    show_partial_samples: bool,
    report_json: bool,
    max_smoke_failures: Option<usize>,
    max_partial_events: Option<u64>,
}

#[derive(Debug)]
struct FileSummary {
    path: PathBuf,
    pdrf: u8,
    is_copc: bool,
    laszip: Option<LaszipVlrInfo>,
}

#[derive(Debug, Clone)]
struct CopcLayoutSummary {
    layout_class: String,
    header_offset_fallback_hit: bool,
}

fn parse_args() -> Result<CliConfig, String> {
    parse_args_from(env::args().collect::<Vec<_>>())
}

fn parse_args_from(args: Vec<String>) -> Result<CliConfig, String> {
    if args.len() < 2 {
        return Err(
            "usage: cargo run --example lidar_corpus_inventory -- <root_dir> [--max-files N] [--smoke-read] [--show-partial-samples] [--report-json] [--max-smoke-failures N] [--max-partial-events N]"
                .to_string(),
        );
    }

    let root = PathBuf::from(&args[1]);
    let mut max_files = None;
    let mut smoke_read = false;
    let mut show_partial_samples = false;
    let mut report_json = false;
    let mut max_smoke_failures = None;
    let mut max_partial_events = None;
    let mut i = 2;
    while i < args.len() {
        if args[i] == "--max-files" {
            if i + 1 >= args.len() {
                return Err("--max-files requires a value".to_string());
            }
            let n = args[i + 1]
                .parse::<usize>()
                .map_err(|_| format!("invalid --max-files value: {}", args[i + 1]))?;
            max_files = Some(n);
            i += 2;
        } else if args[i] == "--max-smoke-failures" {
            if i + 1 >= args.len() {
                return Err("--max-smoke-failures requires a value".to_string());
            }
            let n = args[i + 1]
                .parse::<usize>()
                .map_err(|_| format!("invalid --max-smoke-failures value: {}", args[i + 1]))?;
            max_smoke_failures = Some(n);
            i += 2;
        } else if args[i] == "--max-partial-events" {
            if i + 1 >= args.len() {
                return Err("--max-partial-events requires a value".to_string());
            }
            let n = args[i + 1]
                .parse::<u64>()
                .map_err(|_| format!("invalid --max-partial-events value: {}", args[i + 1]))?;
            max_partial_events = Some(n);
            i += 2;
        } else if args[i] == "--smoke-read" {
            smoke_read = true;
            i += 1;
        } else if args[i] == "--show-partial-samples" {
            show_partial_samples = true;
            i += 1;
        } else if args[i] == "--report-json" {
            report_json = true;
            i += 1;
        } else {
            return Err(format!("unknown argument: {}", args[i]));
        }
    }

    Ok(CliConfig {
        root,
        max_files,
        smoke_read,
        show_partial_samples: show_partial_samples || report_json,
        report_json,
        max_smoke_failures,
        max_partial_events,
    })
}

fn has_lidar_ext(path: &Path) -> bool {
    let s = path.to_string_lossy().to_lowercase();
    s.ends_with(".las") || s.ends_with(".laz") || s.ends_with(".copc.las") || s.ends_with(".copc.laz")
}

fn walk_lidar_files(root: &Path, max_files: Option<usize>) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let rd = fs::read_dir(&dir)
            .map_err(|e| format!("failed to read directory {}: {}", dir.display(), e))?;
        for entry in rd {
            let entry = match entry {
                Ok(v) => v,
                Err(_) => continue,
            };
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if has_lidar_ext(&path) {
                out.push(path);
                if let Some(max_n) = max_files {
                    if out.len() >= max_n {
                        return Ok(out);
                    }
                }
            }
        }
    }

    Ok(out)
}

fn summarize_file(path: &Path) -> Result<FileSummary, String> {
    let file = fs::File::open(path)
        .map_err(|e| format!("open {}: {}", path.display(), e))?;

    let reader = LasReader::new(file)
        .map_err(|e| format!("read header {}: {}", path.display(), e))?;

    let pdrf = reader.header().point_data_format as u8;
    let is_copc = reader
        .vlrs()
        .iter()
        .any(|v| v.key.user_id == COPC_USER_ID && v.key.record_id == COPC_INFO_RECORD_ID);
    let laszip = parse_laszip_vlr(reader.vlrs());

    Ok(FileSummary {
        path: path.to_path_buf(),
        pdrf,
        is_copc,
        laszip,
    })
}

fn classify_copc_layout(path: &Path) -> Result<CopcLayoutSummary, String> {
    let file = fs::File::open(path)
        .map_err(|e| format!("open {}: {}", path.display(), e))?;
    let reader = CopcReader::new(file)
        .map_err(|e| format!("copc parse {}: {}", path.display(), e))?;

    let mut direct_data_entries = 0usize;
    let mut subpage_refs = 0usize;
    for entry in &reader.hierarchy.entries {
        if entry.point_count > 0 && entry.byte_size > 0 {
            direct_data_entries += 1;
        } else if entry.point_count < 0 && entry.byte_size > 0 {
            subpage_refs += 1;
        }
    }

    let layout_class = if subpage_refs == 0 {
        COPC_LAYOUT_SINGLE_PAGE.to_string()
    } else if direct_data_entries > 0 {
        COPC_LAYOUT_MIXED_ROOT.to_string()
    } else {
        COPC_LAYOUT_PAGED.to_string()
    };

    Ok(CopcLayoutSummary {
        layout_class,
        header_offset_fallback_hit: matches!(
            reader.hierarchy_parse_mode,
            CopcHierarchyParseMode::EvlrHeaderOffset
        ),
    })
}

fn laszip_signature(summary: &FileSummary) -> String {
    if let Some(info) = summary.laszip.as_ref() {
        let items = info
            .items
            .iter()
            .map(|it| format!("{}:{}:{}", it.item_type, it.item_size, it.item_version))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "pdrf{}|copc={}|compressor={:?}|coder={}|items=[{}]",
            summary.pdrf, summary.is_copc, info.compressor, info.coder, items
        )
    } else {
        format!("pdrf{}|copc={}|laszip=none", summary.pdrf, summary.is_copc)
    }
}

fn is_supported_by_current_standard_path(summary: &FileSummary) -> bool {
    let Some(info) = summary.laszip.as_ref() else {
        return true;
    };

    if !info.uses_arithmetic_coder() {
        return true;
    }

    if info.has_point10_item() && !info.has_point14_item() {
        for item in &info.items {
            match item.item_type {
                6 => {
                    if item.item_size != 20 || item.item_version != 2 {
                        return false;
                    }
                }
                7 => {
                    if item.item_size != 8 || item.item_version != 2 {
                        return false;
                    }
                }
                8 => {
                    if item.item_size != 6 || item.item_version != 2 {
                        return false;
                    }
                }
                0 => {
                    if item.item_version != 2 {
                        return false;
                    }
                }
                _ => return false,
            }
        }

        // PDRFs with gps_time require GPSTIME item.
        let pdrf_has_gps = !matches!(summary.pdrf, 0 | 2);
        let has_gps_item = info.items.iter().any(|it| it.item_type == 7);
        if pdrf_has_gps && !has_gps_item {
            return false;
        }

        // Tolerant mode: accept non-conformant Point10 streams that omit RGB
        // items even when PDRF advertises RGB-capable layouts.

        return true;
    }

    false
}

fn print_top_counts(title: &str, map: &HashMap<String, usize>, top_n: usize) {
    println!("\n{}", title);
    let mut v = map.iter().collect::<Vec<_>>();
    v.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    for (idx, (k, c)) in v.into_iter().take(top_n).enumerate() {
        println!("{:>3}. {:>6}  {}", idx + 1, c, k);
    }
}

fn classify_error(err: &str) -> String {
    let lower = err.to_lowercase();
    if lower.contains("unexpectedeof") || lower.contains("failed to fill whole buffer") {
        "io-unexpected-eof".to_string()
    } else if lower.contains("point14 layered stream") {
        "point14-layered-unimplemented".to_string()
    } else if lower.contains("chunk table could not be parsed") {
        "chunk-table-parse-failed".to_string()
    } else if lower.contains("not implemented") {
        "other-unimplemented".to_string()
    } else if lower.contains("compression") {
        "compression-error".to_string()
    } else {
        err.lines().next().unwrap_or(err).to_string()
    }
}

fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c <= '\u{1f}' => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

fn top_counts(map: &HashMap<String, usize>, top_n: usize) -> Vec<(String, usize)> {
    let mut v = map
        .iter()
        .map(|(k, c)| (k.clone(), *c))
        .collect::<Vec<_>>();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v.truncate(top_n);
    v
}

fn fmt_json_string(s: &str) -> String {
    format!("\"{}\"", escape_json(s))
}

fn fmt_json_opt_usize(v: Option<usize>) -> String {
    match v {
        Some(n) => n.to_string(),
        None => "null".to_string(),
    }
}

fn fmt_json_opt_u64(v: Option<u64>) -> String {
    match v {
        Some(n) => n.to_string(),
        None => "null".to_string(),
    }
}

fn evaluate_thresholds(
    smoke_fail: usize,
    smoke_partial_events: u64,
    max_smoke_failures: Option<usize>,
    max_partial_events: Option<u64>,
) -> Vec<String> {
    let mut breaches = Vec::new();
    if let Some(limit) = max_smoke_failures {
        if smoke_fail > limit {
            breaches.push(format!(
                "smoke failures {} exceeded max {}",
                smoke_fail, limit
            ));
        }
    }
    if let Some(limit) = max_partial_events {
        if smoke_partial_events > limit {
            breaches.push(format!(
                "partial events {} exceeded max {}",
                smoke_partial_events, limit
            ));
        }
    }
    breaches
}

#[allow(clippy::too_many_arguments)]
fn build_json_report(
    cfg: &CliConfig,
    threshold_breaches: &[String],
    root: &Path,
    files_len: usize,
    ok: usize,
    failed: usize,
    smoke_read: bool,
    smoke_ok: usize,
    smoke_fail: usize,
    smoke_partial_files: usize,
    smoke_partial_events: u64,
    smoke_partial_decoded_points: u64,
    smoke_partial_expected_points: u64,
    signatures: &HashMap<String, usize>,
    unsupported_signatures: &HashMap<String, usize>,
    smoke_errors: &HashMap<String, usize>,
    unsupported_samples: &[(String, PathBuf)],
    smoke_samples: &[(String, PathBuf)],
    partial_samples: &[(u64, u64, u64, PathBuf)],
    copc_files: usize,
    copc_layout_classified: usize,
    copc_layout_parse_failed: usize,
    copc_layout_counts: &HashMap<String, usize>,
    copc_header_offset_fallback_hits: usize,
    copc_header_offset_fallback_samples: &[PathBuf],
    copc_layout_parse_failure_samples: &[(String, PathBuf)],
) -> String {
    let generated_unix_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let mut out = String::new();
    let _ = writeln!(&mut out, "{{");
    let _ = writeln!(&mut out, "  \"schema_version\": {},", REPORT_SCHEMA_VERSION);
    let _ = writeln!(
        &mut out,
        "  \"tool_version\": {},",
        fmt_json_string(env!("CARGO_PKG_VERSION"))
    );
    let _ = writeln!(&mut out, "  \"generated_unix_seconds\": {},", generated_unix_seconds);
    let _ = writeln!(
        &mut out,
        "  \"scanned_root\": {},",
        fmt_json_string(&root.display().to_string())
    );
    let _ = writeln!(&mut out, "  \"candidate_files_found\": {},", files_len);
    let _ = writeln!(&mut out, "  \"headers_parsed\": {},", ok);
    let _ = writeln!(&mut out, "  \"header_parse_failed\": {},", failed);

    let _ = writeln!(&mut out, "  \"thresholds\": {{");
    let _ = writeln!(&mut out, "    \"max_smoke_failures\": {},", fmt_json_opt_usize(cfg.max_smoke_failures));
    let _ = writeln!(&mut out, "    \"max_partial_events\": {},", fmt_json_opt_u64(cfg.max_partial_events));
    let _ = writeln!(&mut out, "    \"passed\": {},", threshold_breaches.is_empty());
    let _ = writeln!(&mut out, "    \"breaches\": [");
    for (idx, breach) in threshold_breaches.iter().enumerate() {
        let comma = if idx + 1 == threshold_breaches.len() { "" } else { "," };
        let _ = writeln!(&mut out, "      {}{}", fmt_json_string(breach), comma);
    }
    let _ = writeln!(&mut out, "    ]");
    let _ = writeln!(&mut out, "  }},");

    let _ = writeln!(&mut out, "  \"smoke_read\": {{");
    let _ = writeln!(&mut out, "    \"enabled\": {},", smoke_read);
    let _ = writeln!(&mut out, "    \"ok\": {},", smoke_ok);
    let _ = writeln!(&mut out, "    \"failed\": {},", smoke_fail);
    let _ = writeln!(&mut out, "    \"partial_files\": {},", smoke_partial_files);
    let _ = writeln!(&mut out, "    \"partial_events\": {},", smoke_partial_events);
    let _ = writeln!(&mut out, "    \"partial_decoded_points\": {},", smoke_partial_decoded_points);
    let _ = writeln!(&mut out, "    \"partial_expected_points\": {}", smoke_partial_expected_points);
    let _ = writeln!(&mut out, "  }},");

    let top_copc_layouts = top_counts(copc_layout_counts, DEFAULT_TOP_N);
    let _ = writeln!(&mut out, "  \"copc_hierarchy\": {{");
    let _ = writeln!(&mut out, "    \"copc_files\": {},", copc_files);
    let _ = writeln!(&mut out, "    \"classified\": {},", copc_layout_classified);
    let _ = writeln!(&mut out, "    \"parse_failed\": {},", copc_layout_parse_failed);
    let _ = writeln!(
        &mut out,
        "    \"header_offset_fallback_hits\": {},",
        copc_header_offset_fallback_hits
    );
    let _ = writeln!(&mut out, "    \"layout_counts\": [");
    for (idx, (layout, count)) in top_copc_layouts.iter().enumerate() {
        let comma = if idx + 1 == top_copc_layouts.len() { "" } else { "," };
        let _ = writeln!(
            &mut out,
            "      {{\"layout\": {}, \"count\": {}}}{}",
            fmt_json_string(layout),
            count,
            comma
        );
    }
    let _ = writeln!(&mut out, "    ],");
    let _ = writeln!(&mut out, "    \"header_offset_fallback_samples\": [");
    for (idx, p) in copc_header_offset_fallback_samples.iter().enumerate() {
        let comma = if idx + 1 == copc_header_offset_fallback_samples.len() { "" } else { "," };
        let _ = writeln!(
            &mut out,
            "      {}{}",
            fmt_json_string(&p.display().to_string()),
            comma
        );
    }
    let _ = writeln!(&mut out, "    ],");
    let _ = writeln!(&mut out, "    \"parse_failure_samples\": [");
    for (idx, (reason, p)) in copc_layout_parse_failure_samples.iter().enumerate() {
        let comma = if idx + 1 == copc_layout_parse_failure_samples.len() { "" } else { "," };
        let _ = writeln!(
            &mut out,
            "      {{\"reason\": {}, \"path\": {}}}{}",
            fmt_json_string(reason),
            fmt_json_string(&p.display().to_string()),
            comma
        );
    }
    let _ = writeln!(&mut out, "    ]");
    let _ = writeln!(&mut out, "  }},");

    let top_signatures = top_counts(signatures, DEFAULT_TOP_N);
    let _ = writeln!(&mut out, "  \"top_signatures\": [");
    for (idx, (sig, count)) in top_signatures.iter().enumerate() {
        let comma = if idx + 1 == top_signatures.len() { "" } else { "," };
        let _ = writeln!(
            &mut out,
            "    {{\"signature\": {}, \"count\": {}}}{}",
            fmt_json_string(sig),
            count,
            comma
        );
    }
    let _ = writeln!(&mut out, "  ],");

    let top_unsupported = top_counts(unsupported_signatures, DEFAULT_TOP_N);
    let _ = writeln!(&mut out, "  \"top_unsupported_signatures\": [");
    for (idx, (sig, count)) in top_unsupported.iter().enumerate() {
        let comma = if idx + 1 == top_unsupported.len() { "" } else { "," };
        let _ = writeln!(
            &mut out,
            "    {{\"signature\": {}, \"count\": {}}}{}",
            fmt_json_string(sig),
            count,
            comma
        );
    }
    let _ = writeln!(&mut out, "  ],");

    let top_smoke_errors = top_counts(smoke_errors, DEFAULT_TOP_N);
    let _ = writeln!(&mut out, "  \"top_smoke_error_categories\": [");
    for (idx, (category, count)) in top_smoke_errors.iter().enumerate() {
        let comma = if idx + 1 == top_smoke_errors.len() { "" } else { "," };
        let _ = writeln!(
            &mut out,
            "    {{\"category\": {}, \"count\": {}}}{}",
            fmt_json_string(category),
            count,
            comma
        );
    }
    let _ = writeln!(&mut out, "  ],");

    let _ = writeln!(&mut out, "  \"unsupported_samples\": [");
    for (idx, (sig, p)) in unsupported_samples.iter().enumerate() {
        let comma = if idx + 1 == unsupported_samples.len() { "" } else { "," };
        let _ = writeln!(
            &mut out,
            "    {{\"signature\": {}, \"path\": {}}}{}",
            fmt_json_string(sig),
            fmt_json_string(&p.display().to_string()),
            comma
        );
    }
    let _ = writeln!(&mut out, "  ],");

    let _ = writeln!(&mut out, "  \"smoke_failure_samples\": [");
    for (idx, (category, p)) in smoke_samples.iter().enumerate() {
        let comma = if idx + 1 == smoke_samples.len() { "" } else { "," };
        let _ = writeln!(
            &mut out,
            "    {{\"category\": {}, \"path\": {}}}{}",
            fmt_json_string(category),
            fmt_json_string(&p.display().to_string()),
            comma
        );
    }
    let _ = writeln!(&mut out, "  ],");

    let _ = writeln!(&mut out, "  \"partial_recovery_samples\": [");
    for (idx, (events, decoded, expected, p)) in partial_samples.iter().enumerate() {
        let comma = if idx + 1 == partial_samples.len() { "" } else { "," };
        let _ = writeln!(
            &mut out,
            "    {{\"events\": {}, \"decoded_points\": {}, \"expected_points\": {}, \"path\": {}}}{}",
            events,
            decoded,
            expected,
            fmt_json_string(&p.display().to_string()),
            comma
        );
    }
    let _ = writeln!(&mut out, "  ]");
    let _ = writeln!(&mut out, "}}");
    out
}

fn main() {
    let cfg = match parse_args() {
        Ok(v) => v,
        Err(msg) => {
            eprintln!("{}", msg);
            std::process::exit(2);
        }
    };

    if !cfg.root.exists() {
        eprintln!("root does not exist: {}", cfg.root.display());
        std::process::exit(2);
    }

    let files = match walk_lidar_files(&cfg.root, cfg.max_files) {
        Ok(v) => v,
        Err(msg) => {
            eprintln!("{}", msg);
            std::process::exit(1);
        }
    };

    let mut ok = 0usize;
    let mut failed = 0usize;
    let mut signatures: HashMap<String, usize> = HashMap::new();
    let mut unsupported_signatures: HashMap<String, usize> = HashMap::new();
    let mut unsupported_samples: Vec<(String, PathBuf)> = Vec::new();
    let mut smoke_ok = 0usize;
    let mut smoke_fail = 0usize;
    let mut smoke_errors: HashMap<String, usize> = HashMap::new();
    let mut smoke_samples: Vec<(String, PathBuf)> = Vec::new();
    let mut smoke_partial_files = 0usize;
    let mut smoke_partial_events = 0u64;
    let mut smoke_partial_decoded_points = 0u64;
    let mut smoke_partial_expected_points = 0u64;
    let mut partial_samples: Vec<(u64, u64, u64, PathBuf)> = Vec::new();
    let mut copc_files = 0usize;
    let mut copc_layout_classified = 0usize;
    let mut copc_layout_parse_failed = 0usize;
    let mut copc_layout_counts: HashMap<String, usize> = HashMap::new();
    let mut copc_header_offset_fallback_hits = 0usize;
    let mut copc_header_offset_fallback_samples: Vec<PathBuf> = Vec::new();
    let mut copc_layout_parse_failure_samples: Vec<(String, PathBuf)> = Vec::new();

    for path in &files {
        match summarize_file(path) {
            Ok(summary) => {
                ok += 1;
                let sig = laszip_signature(&summary);
                *signatures.entry(sig.clone()).or_insert(0) += 1;

                if !is_supported_by_current_standard_path(&summary) {
                    *unsupported_signatures.entry(sig.clone()).or_insert(0) += 1;
                    if unsupported_samples.len() < 30 {
                        unsupported_samples.push((sig, summary.path));
                    }
                }

                if summary.is_copc {
                    copc_files += 1;
                    match classify_copc_layout(path) {
                        Ok(layout) => {
                            copc_layout_classified += 1;
                            *copc_layout_counts.entry(layout.layout_class).or_insert(0) += 1;
                            if layout.header_offset_fallback_hit {
                                copc_header_offset_fallback_hits += 1;
                                if copc_header_offset_fallback_samples.len() < DEFAULT_SAMPLE_CAP {
                                    copc_header_offset_fallback_samples.push(path.clone());
                                }
                            }
                        }
                        Err(err) => {
                            copc_layout_parse_failed += 1;
                            if copc_layout_parse_failure_samples.len() < DEFAULT_SAMPLE_CAP {
                                copc_layout_parse_failure_samples.push((
                                    classify_error(&err),
                                    path.clone(),
                                ));
                            }
                        }
                    }
                }

                if cfg.smoke_read {
                    match read_with_diagnostics(path) {
                        Ok((_, diag)) => {
                            smoke_ok += 1;
                            if diag.point14_partial_events > 0 {
                                smoke_partial_files += 1;
                                smoke_partial_events += diag.point14_partial_events;
                                smoke_partial_decoded_points += diag.point14_partial_decoded_points;
                                smoke_partial_expected_points += diag.point14_partial_expected_points;
                                if cfg.show_partial_samples && partial_samples.len() < DEFAULT_SAMPLE_CAP {
                                    partial_samples.push((
                                        diag.point14_partial_events,
                                        diag.point14_partial_decoded_points,
                                        diag.point14_partial_expected_points,
                                        path.clone(),
                                    ));
                                }
                            }
                        }
                        Err(err) => {
                            smoke_fail += 1;
                            let category = classify_error(&err.to_string());
                            *smoke_errors.entry(category.clone()).or_insert(0) += 1;
                            if smoke_samples.len() < DEFAULT_SAMPLE_CAP {
                                smoke_samples.push((category, path.clone()));
                            }
                        }
                    }
                }
            }
            Err(_) => {
                failed += 1;
            }
        }
    }

    let threshold_breaches = evaluate_thresholds(
        smoke_fail,
        smoke_partial_events,
        cfg.max_smoke_failures,
        cfg.max_partial_events,
    );

    if cfg.report_json {
        let report = build_json_report(
            &cfg,
            &threshold_breaches,
            &cfg.root,
            files.len(),
            ok,
            failed,
            cfg.smoke_read,
            smoke_ok,
            smoke_fail,
            smoke_partial_files,
            smoke_partial_events,
            smoke_partial_decoded_points,
            smoke_partial_expected_points,
            &signatures,
            &unsupported_signatures,
            &smoke_errors,
            &unsupported_samples,
            &smoke_samples,
            &partial_samples,
            copc_files,
            copc_layout_classified,
            copc_layout_parse_failed,
            &copc_layout_counts,
            copc_header_offset_fallback_hits,
            &copc_header_offset_fallback_samples,
            &copc_layout_parse_failure_samples,
        );
        print!("{}", report);
        if !threshold_breaches.is_empty() {
            std::process::exit(3);
        }
        return;
    }

    println!("Scanned root: {}", cfg.root.display());
    println!("Candidate files found: {}", files.len());
    println!("Headers parsed: {}", ok);
    println!("Header parse failed: {}", failed);
    if cfg.smoke_read {
        println!("Smoke read OK: {}", smoke_ok);
        println!("Smoke read failed: {}", smoke_fail);
        println!("Smoke read partial files: {}", smoke_partial_files);
        println!("Smoke read partial events: {}", smoke_partial_events);
        println!(
            "Smoke read partial decoded/expected points: {}/{}",
            smoke_partial_decoded_points, smoke_partial_expected_points
        );
    }

    println!("COPC files: {}", copc_files);
    println!("COPC hierarchy classified: {}", copc_layout_classified);
    println!("COPC hierarchy parse failed: {}", copc_layout_parse_failed);
    println!(
        "COPC hierarchy header-offset fallback hits: {}",
        copc_header_offset_fallback_hits
    );

    print_top_counts("Top COPC hierarchy layouts:", &copc_layout_counts, DEFAULT_TOP_N);
    print_top_counts("Top LASzip/COPC signatures:", &signatures, DEFAULT_TOP_N);
    print_top_counts(
        "Top signatures likely unsupported by current standard LAZ path:",
        &unsupported_signatures,
        DEFAULT_TOP_N,
    );
    if cfg.smoke_read {
        print_top_counts("Top smoke-read error categories:", &smoke_errors, DEFAULT_TOP_N);
    }

    if !unsupported_samples.is_empty() {
        println!("\nUnsupported sample files:");
        for (idx, (sig, p)) in unsupported_samples.iter().enumerate() {
            println!("{:>3}. {}\n     {}", idx + 1, sig, p.display());
        }
    }

    if cfg.smoke_read && !smoke_samples.is_empty() {
        println!("\nSmoke-read failure samples:");
        for (idx, (category, p)) in smoke_samples.iter().enumerate() {
            println!("{:>3}. {}\n     {}", idx + 1, category, p.display());
        }
    }

    if cfg.smoke_read && cfg.show_partial_samples && !partial_samples.is_empty() {
        println!("\nSmoke-read partial recovery samples:");
        for (idx, (events, decoded, expected, p)) in partial_samples.iter().enumerate() {
            println!(
                "{:>3}. events={} decoded/expected={}/{}\n     {}",
                idx + 1,
                events,
                decoded,
                expected,
                p.display()
            );
        }
    }

    if !copc_header_offset_fallback_samples.is_empty() {
        println!("\nCOPC header-offset fallback samples:");
        for (idx, p) in copc_header_offset_fallback_samples.iter().enumerate() {
            println!("{:>3}. {}", idx + 1, p.display());
        }
    }

    if !copc_layout_parse_failure_samples.is_empty() {
        println!("\nCOPC hierarchy parse failure samples:");
        for (idx, (reason, p)) in copc_layout_parse_failure_samples.iter().enumerate() {
            println!("{:>3}. {}\n     {}", idx + 1, reason, p.display());
        }
    }

    if !threshold_breaches.is_empty() {
        eprintln!("\nThreshold checks failed:");
        for b in &threshold_breaches {
            eprintln!("- {}", b);
        }
        std::process::exit(3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_report_json_enables_partial_samples() {
        let cfg = parse_args_from(vec![
            "lidar_corpus_inventory".to_string(),
            "/tmp".to_string(),
            "--report-json".to_string(),
        ])
        .expect("args should parse");

        assert!(cfg.report_json);
        assert!(cfg.show_partial_samples);
        assert_eq!(cfg.max_smoke_failures, None);
        assert_eq!(cfg.max_partial_events, None);
    }

    #[test]
    fn parse_args_thresholds() {
        let cfg = parse_args_from(vec![
            "lidar_corpus_inventory".to_string(),
            "/tmp".to_string(),
            "--smoke-read".to_string(),
            "--max-smoke-failures".to_string(),
            "2".to_string(),
            "--max-partial-events".to_string(),
            "7".to_string(),
        ])
        .expect("args should parse");

        assert!(cfg.smoke_read);
        assert_eq!(cfg.max_smoke_failures, Some(2));
        assert_eq!(cfg.max_partial_events, Some(7));
    }

    #[test]
    fn json_report_shape_is_stable_and_parseable() {
        let cfg = CliConfig {
            root: PathBuf::from("/tmp"),
            max_files: Some(10),
            smoke_read: true,
            show_partial_samples: true,
            report_json: true,
            max_smoke_failures: Some(1),
            max_partial_events: Some(3),
        };

        let mut signatures = HashMap::new();
        signatures.insert("pdrf1|copc=false|laszip=none".to_string(), 2usize);
        let unsupported = HashMap::new();
        let mut smoke_errors = HashMap::new();
        smoke_errors.insert("compression-error".to_string(), 1usize);

        let mut copc_layout_counts = HashMap::new();
        copc_layout_counts.insert(COPC_LAYOUT_MIXED_ROOT.to_string(), 2usize);

        let report = build_json_report(
            &cfg,
            &["partial events 4 exceeded max 3".to_string()],
            &cfg.root,
            10,
            10,
            0,
            true,
            9,
            1,
            1,
            4,
            123,
            456,
            &signatures,
            &unsupported,
            &smoke_errors,
            &[],
            &[("compression-error".to_string(), PathBuf::from("/tmp/a.laz"))],
            &[(1, 123, 456, PathBuf::from("/tmp/b.copc.laz"))],
            3,
            2,
            1,
            &copc_layout_counts,
            1,
            &[PathBuf::from("/tmp/hdr_offset.copc.laz")],
            &[("copc-parse-failed".to_string(), PathBuf::from("/tmp/bad.copc.laz"))],
        );

        let parsed: serde_json::Value = serde_json::from_str(&report)
            .expect("json report must be valid JSON");
        assert_eq!(parsed["schema_version"], REPORT_SCHEMA_VERSION);
        assert_eq!(parsed["thresholds"]["passed"], false);
        assert!(parsed["smoke_read"]["partial_events"].is_number());
        assert!(parsed["top_signatures"].is_array());
        assert_eq!(parsed["copc_hierarchy"]["copc_files"], 3);
        assert_eq!(parsed["copc_hierarchy"]["header_offset_fallback_hits"], 1);
        assert!(parsed["copc_hierarchy"]["layout_counts"].is_array());
    }
}
