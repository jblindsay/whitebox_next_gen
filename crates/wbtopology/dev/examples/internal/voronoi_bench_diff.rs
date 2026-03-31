use std::collections::BTreeMap;
use std::env;
use std::fs;

#[derive(Debug, Clone)]
struct PatternThreshold {
    pattern: String,
    value: f64,
}

#[derive(Debug, Clone)]
struct CaseOpPatternThreshold {
    case_pattern: String,
    op_pattern: String,
    value: f64,
}

#[derive(Debug, Clone)]
struct BenchRow {
    case_name: String,
    operation: String,
    iters: usize,
    avg_us: f64,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "Usage: cargo run --release --example voronoi_bench_diff -- <baseline.csv> <current.csv> [max_regression_percent] [--op-threshold op=pct ...] [--case-threshold case=pct ...] [--case-op-threshold case:op=pct ...]"
        );
        std::process::exit(2);
    }

    let baseline_path = &args[1];
    let current_path = &args[2];
    let (
        max_regression,
        op_thresholds,
        case_thresholds,
        case_op_thresholds,
        op_wildcard_thresholds,
        case_wildcard_thresholds,
        case_op_wildcard_thresholds,
    ) = parse_threshold_args(&args);

    let baseline = parse_csv(baseline_path);
    let current = parse_csv(current_path);

    let mut any_regression = false;

    println!("case,operation,baseline_avg_us,current_avg_us,delta_percent,threshold_used,status");

    for (key, base_row) in &baseline {
        let Some(cur_row) = current.get(key) else {
            println!(
                "{},{},{:.3},,,-MISSING_IN_CURRENT",
                base_row.case_name, base_row.operation, base_row.avg_us
            );
            any_regression = true;
            continue;
        };

        let delta_percent = if base_row.avg_us.abs() <= f64::EPSILON {
            0.0
        } else {
            ((cur_row.avg_us - base_row.avg_us) / base_row.avg_us) * 100.0
        };

        let threshold = resolve_threshold(
            &base_row.case_name,
            &base_row.operation,
            max_regression,
            &op_thresholds,
            &case_thresholds,
            &case_op_thresholds,
            &op_wildcard_thresholds,
            &case_wildcard_thresholds,
            &case_op_wildcard_thresholds,
        );

        let status = if delta_percent > threshold {
            any_regression = true;
            "REGRESSION"
        } else {
            "OK"
        };

        println!(
            "{},{},{:.3},{:.3},{:.2},{:.2},{}",
            base_row.case_name,
            base_row.operation,
            base_row.avg_us,
            cur_row.avg_us,
            delta_percent,
            threshold,
            status
        );
    }

    for (key, cur_row) in &current {
        if !baseline.contains_key(key) {
            println!(
                "{},{},,{:.3},,NEW_IN_CURRENT",
                cur_row.case_name, cur_row.operation, cur_row.avg_us
            );
        }
    }

    if any_regression {
        std::process::exit(1);
    }
}

fn parse_threshold_args(
    args: &[String],
) -> (
    f64,
    BTreeMap<String, f64>,
    BTreeMap<String, f64>,
    BTreeMap<(String, String), f64>,
    Vec<PatternThreshold>,
    Vec<PatternThreshold>,
    Vec<CaseOpPatternThreshold>,
) {
    let mut max_regression = 0.0;
    if args.len() >= 4 && !args[3].starts_with("--") {
        max_regression = args[3].parse::<f64>().unwrap_or(0.0);
    }

    let mut op_thresholds = BTreeMap::<String, f64>::new();
    let mut case_thresholds = BTreeMap::<String, f64>::new();
    let mut case_op_thresholds = BTreeMap::<(String, String), f64>::new();
    let mut op_wildcard_thresholds = Vec::<PatternThreshold>::new();
    let mut case_wildcard_thresholds = Vec::<PatternThreshold>::new();
    let mut case_op_wildcard_thresholds = Vec::<CaseOpPatternThreshold>::new();

    let mut i = 3usize;
    while i < args.len() {
        let token = &args[i];
        if token == "--op-threshold" {
            let Some(spec) = args.get(i + 1) else {
                eprintln!("Missing value after --op-threshold");
                std::process::exit(2);
            };
            let (k, v) = parse_kv_threshold(spec, "--op-threshold");
            if k.contains('*') {
                op_wildcard_thresholds.push(PatternThreshold {
                    pattern: k,
                    value: v,
                });
            } else {
                op_thresholds.insert(k, v);
            }
            i += 2;
            continue;
        }

        if token == "--case-threshold" {
            let Some(spec) = args.get(i + 1) else {
                eprintln!("Missing value after --case-threshold");
                std::process::exit(2);
            };
            let (k, v) = parse_kv_threshold(spec, "--case-threshold");
            if k.contains('*') {
                case_wildcard_thresholds.push(PatternThreshold {
                    pattern: k,
                    value: v,
                });
            } else {
                case_thresholds.insert(k, v);
            }
            i += 2;
            continue;
        }

        if token == "--case-op-threshold" {
            let Some(spec) = args.get(i + 1) else {
                eprintln!("Missing value after --case-op-threshold");
                std::process::exit(2);
            };
            let (case_name, op_name, pct) = parse_case_op_kv_threshold(spec, "--case-op-threshold");
            if case_name.contains('*') || op_name.contains('*') {
                case_op_wildcard_thresholds.push(CaseOpPatternThreshold {
                    case_pattern: case_name,
                    op_pattern: op_name,
                    value: pct,
                });
            } else {
                case_op_thresholds.insert((case_name, op_name), pct);
            }
            i += 2;
            continue;
        }

        if token.starts_with("--") {
            eprintln!("Unknown argument: {}", token);
            std::process::exit(2);
        }

        i += 1;
    }

    (
        max_regression,
        op_thresholds,
        case_thresholds,
        case_op_thresholds,
        op_wildcard_thresholds,
        case_wildcard_thresholds,
        case_op_wildcard_thresholds,
    )
}

fn parse_kv_threshold(spec: &str, arg_name: &str) -> (String, f64) {
    let Some((k, v)) = spec.split_once('=') else {
        eprintln!("Invalid {} value '{}'; expected <name>=<percent>", arg_name, spec);
        std::process::exit(2);
    };
    let key = k.trim();
    let val = v.trim();
    if key.is_empty() {
        eprintln!("Invalid {} key in '{}'", arg_name, spec);
        std::process::exit(2);
    }
    let pct = val.parse::<f64>().unwrap_or_else(|_| {
        eprintln!("Invalid {} percent in '{}'", arg_name, spec);
        std::process::exit(2);
    });
    (key.to_string(), pct)
}

fn parse_case_op_kv_threshold(spec: &str, arg_name: &str) -> (String, String, f64) {
    let Some((lhs, rhs)) = spec.split_once('=') else {
        eprintln!("Invalid {} value '{}'; expected <case>:<operation>=<percent>", arg_name, spec);
        std::process::exit(2);
    };

    let Some((case_name, op_name)) = lhs.split_once(':') else {
        eprintln!(
            "Invalid {} selector '{}'; expected <case>:<operation>",
            arg_name, lhs
        );
        std::process::exit(2);
    };

    let case_name = case_name.trim();
    let op_name = op_name.trim();
    if case_name.is_empty() || op_name.is_empty() {
        eprintln!("Invalid {} selector in '{}'", arg_name, spec);
        std::process::exit(2);
    }

    let pct = rhs.trim().parse::<f64>().unwrap_or_else(|_| {
        eprintln!("Invalid {} percent in '{}'", arg_name, spec);
        std::process::exit(2);
    });

    (case_name.to_string(), op_name.to_string(), pct)
}

fn resolve_threshold(
    case_name: &str,
    operation: &str,
    default_threshold: f64,
    op_thresholds: &BTreeMap<String, f64>,
    case_thresholds: &BTreeMap<String, f64>,
    case_op_thresholds: &BTreeMap<(String, String), f64>,
    op_wildcard_thresholds: &[PatternThreshold],
    case_wildcard_thresholds: &[PatternThreshold],
    case_op_wildcard_thresholds: &[CaseOpPatternThreshold],
) -> f64 {
    if let Some(v) = case_op_thresholds.get(&(case_name.to_string(), operation.to_string())) {
        return *v;
    }
    if let Some(v) = case_thresholds.get(case_name) {
        return *v;
    }
    if let Some(v) = op_thresholds.get(operation) {
        return *v;
    }

    if let Some(v) = match_case_op_wildcard_threshold(case_name, operation, case_op_wildcard_thresholds)
    {
        return v;
    }
    if let Some(v) = match_wildcard_threshold(case_name, case_wildcard_thresholds) {
        return v;
    }
    if let Some(v) = match_wildcard_threshold(operation, op_wildcard_thresholds) {
        return v;
    }

    default_threshold
}

fn match_wildcard_threshold(text: &str, rules: &[PatternThreshold]) -> Option<f64> {
    for rule in rules {
        if wildcard_match(&rule.pattern, text) {
            return Some(rule.value);
        }
    }
    None
}

fn match_case_op_wildcard_threshold(
    case_name: &str,
    operation: &str,
    rules: &[CaseOpPatternThreshold],
) -> Option<f64> {
    for rule in rules {
        if wildcard_match(&rule.case_pattern, case_name)
            && wildcard_match(&rule.op_pattern, operation)
        {
            return Some(rule.value);
        }
    }
    None
}

fn wildcard_match(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    let p = pattern.as_bytes();
    let t = text.as_bytes();
    let mut pi = 0usize;
    let mut ti = 0usize;
    let mut star_idx: Option<usize> = None;
    let mut match_idx = 0usize;

    while ti < t.len() {
        if pi < p.len() && p[pi] == t[ti] {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == b'*' {
            star_idx = Some(pi);
            match_idx = ti;
            pi += 1;
        } else if let Some(s) = star_idx {
            pi = s + 1;
            match_idx += 1;
            ti = match_idx;
        } else {
            return false;
        }
    }

    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }

    pi == p.len()
}

fn parse_csv(path: &str) -> BTreeMap<(String, String), BenchRow> {
    let txt = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Failed reading {}: {}", path, e);
        std::process::exit(2);
    });

    let mut out = BTreeMap::<(String, String), BenchRow>::new();
    let mut new_schema = false;

    for (i, raw) in txt.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if i == 0 {
            new_schema = line.starts_with("case,");
            continue;
        }

        let cols: Vec<&str> = line.split(',').collect();
        let (case_name, operation, iters, avg_us) = if new_schema {
            if cols.len() < 5 {
                eprintln!("Invalid CSV row in {}: {}", path, line);
                std::process::exit(2);
            }
            let case_name = cols[0].trim().to_string();
            let operation = cols[1].trim().to_string();
            let iters = cols[2].trim().parse::<usize>().unwrap_or(0);
            let avg_us = cols[4].trim().parse::<f64>().unwrap_or_else(|_| {
                eprintln!("Invalid avg_us in {}: {}", path, line);
                std::process::exit(2);
            });
            (case_name, operation, iters, avg_us)
        } else {
            if cols.len() < 9 {
                eprintln!("Invalid CSV row in {}: {}", path, line);
                std::process::exit(2);
            }
            let n = cols[0].trim().parse::<usize>().unwrap_or(0);
            let iters = cols[1].trim().parse::<usize>().unwrap_or(0);
            let eps = cols[3].trim().parse::<f64>().unwrap_or(0.0);
            let case_name = format!("n{}_iters{}_eps{:.3e}", n, iters, eps);
            let operation = "voronoi".to_string();
            let avg_us = cols[6].trim().parse::<f64>().unwrap_or_else(|_| {
                eprintln!("Invalid median_us in {}: {}", path, line);
                std::process::exit(2);
            });
            (case_name, operation, iters, avg_us)
        };

        let row = BenchRow {
            case_name: case_name.clone(),
            operation: operation.clone(),
            iters,
            avg_us,
        };

        let key = (case_name, operation);
        if let Some(prev) = out.get(&key) {
            if prev.iters != row.iters {
                eprintln!(
                    "Warning: iteration count mismatch for {:?}: {} vs {}",
                    key, prev.iters, row.iters
                );
            }
        }

        out.insert(key, row);
    }

    out
}
