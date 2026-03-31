use std::env;
use std::time::Instant;

use wbtopology::{node_linestrings, Coord, LineString};

#[derive(Clone, Copy)]
enum OutputMode {
    Human,
    Csv,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let csv = args.iter().any(|a| a == "--csv");
    let iters = parse_usize_arg(&args, "--iters").unwrap_or(3).max(1);
    let size = parse_usize_arg(&args, "--size").unwrap_or(120).max(4);
    let eps = parse_f64_arg(&args, "--eps").unwrap_or(1.0e-9).abs();
    let mode = if csv { OutputMode::Csv } else { OutputMode::Human };

    if matches!(mode, OutputMode::Csv) {
        println!("case,size,lines,iters,total_us,avg_us,avg_out_segments");
    } else {
        println!("noding_bench size={size} iters={iters} eps={eps:.3e}");
    }

    run_case("grid_crosshatch", grid_crosshatch(size), eps, iters, mode);
    run_case("offset_grid", offset_grid(size), eps, iters, mode);
    run_case("diagonal_fan", diagonal_fan(size), eps, iters, mode);
}

fn parse_usize_arg(args: &[String], key: &str) -> Option<usize> {
    let mut i = 0usize;
    while i < args.len() {
        if args[i] == key {
            let v = args.get(i + 1)?;
            return v.parse::<usize>().ok();
        }
        i += 1;
    }
    None
}

fn parse_f64_arg(args: &[String], key: &str) -> Option<f64> {
    let mut i = 0usize;
    while i < args.len() {
        if args[i] == key {
            let v = args.get(i + 1)?;
            return v.parse::<f64>().ok();
        }
        i += 1;
    }
    None
}

fn run_case(name: &str, lines: Vec<LineString>, eps: f64, iters: usize, mode: OutputMode) {
    let line_count = lines.len();
    let mut out_sum = 0usize;

    let t0 = Instant::now();
    for _ in 0..iters {
        let out = node_linestrings(&lines, eps);
        out_sum += out.len();
    }
    let total_us = t0.elapsed().as_secs_f64() * 1.0e6;
    let avg_us = total_us / iters as f64;
    let avg_out_segments = out_sum as f64 / iters as f64;

    match mode {
        OutputMode::Human => {
            println!(
                "{name:>16}: lines={line_count}, avg={avg_us:.2} us, out_avg={avg_out_segments:.1}"
            );
        }
        OutputMode::Csv => {
            println!(
                "{name},{},{},{},{:.3},{:.3},{:.3}",
                inferred_size(&lines),
                line_count,
                iters,
                total_us,
                avg_us,
                avg_out_segments
            );
        }
    }
}

fn inferred_size(lines: &[LineString]) -> usize {
    if lines.is_empty() {
        0
    } else {
        lines.len() / 2
    }
}

fn grid_crosshatch(size: usize) -> Vec<LineString> {
    let mut lines = Vec::with_capacity(size * 2);
    let span = size as f64;

    for i in 0..size {
        let y = i as f64;
        lines.push(LineString::new(vec![Coord::xy(0.0, y), Coord::xy(span, y)]));
    }
    for i in 0..size {
        let x = i as f64;
        lines.push(LineString::new(vec![Coord::xy(x, 0.0), Coord::xy(x, span)]));
    }
    lines
}

fn offset_grid(size: usize) -> Vec<LineString> {
    let mut lines = Vec::with_capacity(size * 2);
    let span = size as f64;
    let half = 0.5;

    for i in 0..size {
        let y = i as f64 + half;
        lines.push(LineString::new(vec![Coord::xy(0.0, y), Coord::xy(span, y)]));
    }
    for i in 0..size {
        let x = i as f64;
        lines.push(LineString::new(vec![Coord::xy(x, 0.0), Coord::xy(x + half, span)]));
    }
    lines
}

fn diagonal_fan(size: usize) -> Vec<LineString> {
    let mut lines = Vec::with_capacity(size * 2);
    let span = size as f64;

    for i in 0..size {
        let t = i as f64;
        lines.push(LineString::new(vec![Coord::xy(0.0, t), Coord::xy(span, span - t)]));
    }
    for i in 0..size {
        let t = i as f64;
        lines.push(LineString::new(vec![Coord::xy(t, 0.0), Coord::xy(span - t, span)]));
    }
    lines
}
