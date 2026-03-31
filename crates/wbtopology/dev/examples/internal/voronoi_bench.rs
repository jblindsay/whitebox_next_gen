use std::env;
use std::time::Instant;

use wbtopology::{voronoi_diagram, Coord};

#[derive(Clone, Copy)]
enum OutputMode {
    Human,
    Csv,
}

#[derive(Clone)]
enum PointPattern {
    Uniform,
    Clustered,
}

#[derive(Clone)]
struct BenchCase {
    name: String,
    n: usize,
    iters: usize,
    pattern: PointPattern,
}

struct BenchResult {
    median_total_us: f64,
    median_avg_us: f64,
    min_avg_us: f64,
    max_avg_us: f64,
    median_cells: usize,
    median_vertices: usize,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let n = parse_usize_arg(&args, "--n");
    let iters = parse_usize_arg(&args, "--iters").unwrap_or(2);
    let repeats = parse_usize_arg(&args, "--repeats").unwrap_or(5);
    let eps = parse_f64_arg(&args, "--eps").unwrap_or(1.0e-9);
    let csv = args.iter().any(|a| a == "--csv");
    let mode = if csv { OutputMode::Csv } else { OutputMode::Human };

    if matches!(mode, OutputMode::Csv) {
        println!(
            "case,operation,iters,total_us,avg_us,repeats,min_avg_us,max_avg_us,cells,total_vertices"
        );
    } else {
        println!("voronoi_bench repeats={repeats}, eps={eps}");
    }

    let mut cases = Vec::<BenchCase>::new();
    if let Some(custom_n) = n {
        cases.push(BenchCase {
            name: format!("custom_uniform_n{}", custom_n),
            n: custom_n,
            iters,
            pattern: PointPattern::Uniform,
        });
    } else {
        cases.push(BenchCase {
            name: "uniform_small".to_string(),
            n: 3_000,
            iters: 4,
            pattern: PointPattern::Uniform,
        });
        cases.push(BenchCase {
            name: "uniform_medium".to_string(),
            n: 12_000,
            iters: 2,
            pattern: PointPattern::Uniform,
        });
        cases.push(BenchCase {
            name: "uniform_large".to_string(),
            n: 30_000,
            iters: 1,
            pattern: PointPattern::Uniform,
        });
        cases.push(BenchCase {
            name: "clustered_medium".to_string(),
            n: 12_000,
            iters: 2,
            pattern: PointPattern::Clustered,
        });
    }

    for case in cases {
        run_case(&case, repeats, eps, mode);
    }
}

fn run_case(case: &BenchCase, repeats: usize, eps: f64, mode: OutputMode) {
    let points = generate_points(case.n, &case.pattern);
    let result = bench_case(&points, case.iters, repeats, eps);

    match mode {
        OutputMode::Human => {
            println!(
                "  {:>16}: n={}, iters={}, median={:.3} us/iter, min={:.3}, max={:.3}, cells={}, vertices={}",
                case.name,
                case.n,
                case.iters,
                result.median_avg_us,
                result.min_avg_us,
                result.max_avg_us,
                result.median_cells,
                result.median_vertices,
            );
        }
        OutputMode::Csv => {
            println!(
                "{},voronoi,{},{:.3},{:.3},{},{:.3},{:.3},{},{}",
                case.name,
                case.iters,
                result.median_total_us,
                result.median_avg_us,
                repeats,
                result.min_avg_us,
                result.max_avg_us,
                result.median_cells,
                result.median_vertices,
            );
        }
    }
}

fn bench_case(points: &[Coord], iters: usize, repeats: usize, eps: f64) -> BenchResult {
    let mut totals = Vec::<f64>::with_capacity(repeats.max(1));
    let mut avgs = Vec::<f64>::with_capacity(repeats.max(1));
    let mut cells = Vec::<usize>::with_capacity(repeats.max(1));
    let mut vertices = Vec::<usize>::with_capacity(repeats.max(1));

    for _ in 0..repeats.max(1) {
        let t0 = Instant::now();
        let mut cell_count = 0usize;
        let mut vertex_count = 0usize;

        for _ in 0..iters.max(1) {
            let vd = voronoi_diagram(points, eps);
            cell_count = vd.cells.len();
            vertex_count = vd.cells.iter().map(|c| c.exterior.coords.len()).sum();
        }

        let total_us = t0.elapsed().as_secs_f64() * 1.0e6;
        totals.push(total_us);
        avgs.push(total_us / iters.max(1) as f64);
        cells.push(cell_count);
        vertices.push(vertex_count);
    }

    totals.sort_by(|a, b| a.total_cmp(b));
    avgs.sort_by(|a, b| a.total_cmp(b));
    cells.sort_unstable();
    vertices.sort_unstable();

    let mid = avgs.len() / 2;
    let median_total_us = totals[mid];
    let median_avg_us = avgs[mid];

    BenchResult {
        median_total_us,
        median_avg_us,
        min_avg_us: *avgs.first().unwrap_or(&median_avg_us),
        max_avg_us: *avgs.last().unwrap_or(&median_avg_us),
        median_cells: cells[cells.len() / 2],
        median_vertices: vertices[vertices.len() / 2],
    }
}

fn parse_usize_arg(args: &[String], name: &str) -> Option<usize> {
    let mut i = 0usize;
    while i < args.len() {
        if args[i] == name {
            let v = args.get(i + 1)?.parse::<usize>().ok()?;
            return Some(v.max(1));
        }
        i += 1;
    }
    None
}

fn parse_f64_arg(args: &[String], name: &str) -> Option<f64> {
    let mut i = 0usize;
    while i < args.len() {
        if args[i] == name {
            let v = args.get(i + 1)?.parse::<f64>().ok()?;
            return Some(v.abs().max(1.0e-12));
        }
        i += 1;
    }
    None
}

fn generate_points(n: usize, pattern: &PointPattern) -> Vec<Coord> {
    match pattern {
        PointPattern::Uniform => synthetic_points_uniform(n, 0xD1B5_4A32_91C3_7F21),
        PointPattern::Clustered => synthetic_points_clustered(n, 0x6C8E_9CF5_2A71_13D9),
    }
}

fn synthetic_points_uniform(n: usize, mut state: u64) -> Vec<Coord> {
    let mut out = Vec::<Coord>::with_capacity(n);

    for i in 0..n {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let u = ((state >> 11) as f64) / ((1u64 << 53) as f64);
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let v = ((state >> 11) as f64) / ((1u64 << 53) as f64);

        let band = (i % 7) as f64;
        let x = u * 2000.0 + band * 0.07;
        let y = v * 2000.0 + (band * 0.31).sin() * 0.3;
        out.push(Coord::xy(x, y));
    }

    out
}

fn synthetic_points_clustered(n: usize, mut state: u64) -> Vec<Coord> {
    let mut out = Vec::<Coord>::with_capacity(n);
    let centers = [
        Coord::xy(300.0, 300.0),
        Coord::xy(1500.0, 400.0),
        Coord::xy(700.0, 1500.0),
        Coord::xy(1650.0, 1650.0),
    ];

    for i in 0..n {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let u = ((state >> 11) as f64) / ((1u64 << 53) as f64);
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let v = ((state >> 11) as f64) / ((1u64 << 53) as f64);

        let c = centers[i % centers.len()];
        let x = c.x + (u - 0.5) * 450.0;
        let y = c.y + (v - 0.5) * 450.0;
        out.push(Coord::xy(x, y));
    }

    out
}
