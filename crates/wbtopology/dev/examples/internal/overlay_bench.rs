use std::env;
use std::f64::consts::PI;
use std::time::Instant;

use wbtopology::{
    polygon_difference,
    polygon_overlay_all,
    polygon_intersection,
    polygon_sym_diff,
    polygon_union,
    Coord,
    LinearRing,
    Polygon,
};

fn main() {
    let args: Vec<String> = env::args().collect();
    let csv = args.iter().any(|a| a == "--csv");
    let repeats = parse_repeats_arg(&args).unwrap_or(1);
    let mode = if csv { OutputMode::Csv } else { OutputMode::Human };
    let eps = 1.0e-9;

    if matches!(mode, OutputMode::Csv) {
        println!("case,operation,iters,total_us,avg_us,repeats,min_avg_us,max_avg_us");
    } else {
        println!("overlay_bench repeats={repeats}");
    }

    if matches!(mode, OutputMode::Human) {
        println!("\n## Baseline fixtures (may hit containment fast-path)");
    }
    run_case("small", case_small(), eps, 200, repeats, mode);
    run_case("medium", case_medium(), eps, 80, repeats, mode);
    run_case("complex", case_complex(), eps, 30, repeats, mode);

    if matches!(mode, OutputMode::Human) {
        println!("\n## Non-containment fixtures (prefer full dissolve path)");
    }
    run_case("nc_medium", case_non_containment_medium(), eps, 80, repeats, mode);
    run_case("nc_complex", case_non_containment_complex(), eps, 30, repeats, mode);

    if matches!(mode, OutputMode::Human) {
        println!("\n## Large synthetic stress fixtures");
    }
    run_case("large_crossing", case_large_crossing(), eps, 8, repeats, mode);
    run_case("large_dense", case_large_dense(), eps, 8, repeats, mode);
}

#[derive(Clone, Copy)]
enum OutputMode {
    Human,
    Csv,
}

struct BenchResult {
    median_total_us: f64,
    median_avg_us: f64,
    min_avg_us: f64,
    max_avg_us: f64,
}

fn run_case(
    name: &str,
    (a, b): (Polygon, Polygon),
    eps: f64,
    iters: usize,
    repeats: usize,
    mode: OutputMode,
) {
    if matches!(mode, OutputMode::Human) {
        println!("\n== overlay bench: {name} ({iters} iters, repeats={repeats}) ==");
    }

    let r = bench_op(iters, repeats, || {
        let _ = polygon_intersection(&a, &b, eps);
    });
    emit_result(mode, name, "intersection", iters, repeats, &r);

    let r = bench_op(iters, repeats, || {
        let _ = polygon_union(&a, &b, eps);
    });
    emit_result(mode, name, "union", iters, repeats, &r);

    let r = bench_op(iters, repeats, || {
        let _ = polygon_difference(&a, &b, eps);
    });
    emit_result(mode, name, "difference", iters, repeats, &r);

    let r = bench_op(iters, repeats, || {
        let _ = polygon_sym_diff(&a, &b, eps);
    });
    emit_result(mode, name, "sym_diff", iters, repeats, &r);

    let r = bench_op(iters, repeats, || {
        let _ = polygon_overlay_all(&a, &b, eps);
    });
    emit_result(mode, name, "all_ops_onepass", iters, repeats, &r);
    let onepass_avg = r.median_avg_us;

    let r = bench_op(iters, repeats, || {
        let _ = polygon_intersection(&a, &b, eps);
        let _ = polygon_union(&a, &b, eps);
        let _ = polygon_difference(&a, &b, eps);
        let _ = polygon_sym_diff(&a, &b, eps);
    });
    emit_result(mode, name, "all_ops_separate", iters, repeats, &r);
    emit_all_ops_speedup(mode, name, iters, repeats, onepass_avg, r.median_avg_us);
}

fn emit_all_ops_speedup(
    mode: OutputMode,
    case: &str,
    iters: usize,
    repeats: usize,
    onepass_avg_us: f64,
    separate_avg_us: f64,
) {
    let speedup = if onepass_avg_us > 0.0 {
        separate_avg_us / onepass_avg_us
    } else {
        0.0
    };

    match mode {
        OutputMode::Human => {
            println!(" all_ops speedup (separate/onepass): {:.3}x", speedup);
        }
        OutputMode::Csv => {
            println!(
                "{case},all_ops_speedup_x,{iters},{:.6},{:.6},{},{:.6},{:.6}",
                speedup,
                speedup,
                repeats,
                speedup,
                speedup
            );
        }
    }
}

fn bench_op<F>(iters: usize, repeats: usize, mut f: F) -> BenchResult
where
    F: FnMut(),
{
    let mut totals = Vec::<f64>::with_capacity(repeats.max(1));
    let mut avgs = Vec::<f64>::with_capacity(repeats.max(1));

    for _ in 0..repeats.max(1) {
        let t0 = Instant::now();
        for _ in 0..iters {
            f();
        }
        let total_us = t0.elapsed().as_secs_f64() * 1.0e6;
        totals.push(total_us);
        avgs.push(total_us / iters as f64);
    }

    totals.sort_by(|a, b| a.total_cmp(b));
    avgs.sort_by(|a, b| a.total_cmp(b));

    let mid = avgs.len() / 2;
    let median_total_us = totals[mid];
    let median_avg_us = avgs[mid];

    BenchResult {
        median_total_us,
        median_avg_us,
        min_avg_us: *avgs.first().unwrap_or(&median_avg_us),
        max_avg_us: *avgs.last().unwrap_or(&median_avg_us),
    }
}

fn emit_result(
    mode: OutputMode,
    case: &str,
    op: &str,
    iters: usize,
    repeats: usize,
    r: &BenchResult,
) {
    match mode {
        OutputMode::Human => {
            println!(
                "{op:>12}: median_total={:.3} us, median_avg={:.2} us, min_avg={:.2} us, max_avg={:.2} us",
                r.median_total_us, r.median_avg_us, r.min_avg_us, r.max_avg_us
            );
        }
        OutputMode::Csv => {
            println!(
                "{case},{op},{iters},{:.3},{:.3},{},{:.3},{:.3}",
                r.median_total_us,
                r.median_avg_us,
                repeats,
                r.min_avg_us,
                r.max_avg_us
            );
        }
    }
}

fn parse_repeats_arg(args: &[String]) -> Option<usize> {
    let mut i = 0usize;
    while i < args.len() {
        if args[i] == "--repeats" {
            let v = args.get(i + 1)?;
            let parsed = v.parse::<usize>().ok()?;
            return Some(parsed.max(1));
        }
        i += 1;
    }
    None
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> LinearRing {
    LinearRing::new(vec![
        Coord::xy(x0, y0),
        Coord::xy(x1, y0),
        Coord::xy(x1, y1),
        Coord::xy(x0, y1),
    ])
}

fn case_small() -> (Polygon, Polygon) {
    let a = Polygon::new(rect(0.0, 0.0, 2.0, 2.0), vec![]);
    let b = Polygon::new(rect(1.0, 0.0, 3.0, 2.0), vec![]);
    (a, b)
}

fn case_medium() -> (Polygon, Polygon) {
    let a = Polygon::new(
        rect(0.0, 0.0, 10.0, 10.0),
        vec![rect(2.0, 2.0, 3.0, 8.0), rect(6.0, 2.0, 7.0, 8.0)],
    );
    let b = Polygon::new(
        rect(1.0, -1.0, 11.0, 9.0),
        vec![rect(2.0, 1.0, 9.0, 2.0), rect(2.0, 5.0, 9.0, 6.0)],
    );
    (a, b)
}

fn case_complex() -> (Polygon, Polygon) {
    let a = Polygon::new(
        rect(0.0, 0.0, 12.0, 12.0),
        vec![
            rect(2.0, 2.0, 3.0, 10.0),
            rect(5.0, 2.0, 6.0, 10.0),
            rect(8.0, 2.0, 9.0, 10.0),
        ],
    );
    let b = Polygon::new(
        rect(1.0, 1.0, 11.0, 11.0),
        vec![
            rect(2.0, 2.0, 10.0, 3.0),
            rect(2.0, 5.0, 10.0, 6.0),
            rect(2.0, 8.0, 10.0, 9.0),
        ],
    );
    (a, b)
}

fn case_non_containment_medium() -> (Polygon, Polygon) {
    let a = Polygon::new(
        rect(0.0, 0.0, 10.0, 10.0),
        vec![rect(2.0, 2.0, 4.0, 8.0), rect(6.0, 2.0, 8.0, 8.0)],
    );
    let b = Polygon::new(
        rect(3.0, -1.0, 13.0, 9.0),
        vec![rect(5.0, 1.0, 11.0, 3.0), rect(5.0, 5.0, 11.0, 7.0)],
    );
    (a, b)
}

fn case_non_containment_complex() -> (Polygon, Polygon) {
    let a = Polygon::new(
        rect(0.0, 0.0, 14.0, 14.0),
        vec![
            rect(2.0, 2.0, 3.5, 12.0),
            rect(5.0, 2.0, 6.5, 12.0),
            rect(8.0, 2.0, 9.5, 12.0),
            rect(11.0, 2.0, 12.5, 12.0),
        ],
    );
    let b = Polygon::new(
        rect(1.0, 1.0, 13.0, 13.0),
        vec![
            rect(2.0, 2.0, 12.0, 3.5),
            rect(2.0, 5.0, 12.0, 6.5),
            rect(2.0, 8.0, 12.0, 9.5),
            rect(2.0, 11.0, 12.0, 12.5),
        ],
    );
    (a, b)
}

fn wavy_ring(cx: f64, cy: f64, base_radius: f64, amp: f64, waves: usize, samples: usize) -> LinearRing {
    let n = samples.max(waves.saturating_mul(16)).max(64);
    let mut coords = Vec::with_capacity(n);
    for i in 0..n {
        let t = 2.0 * PI * (i as f64) / (n as f64);
        let r = base_radius + amp * ((waves as f64) * t).sin();
        coords.push(Coord::xy(cx + r * t.cos(), cy + r * t.sin()));
    }
    LinearRing::new(coords)
}

fn case_large_crossing() -> (Polygon, Polygon) {
    let a = Polygon::new(
        wavy_ring(0.0, 0.0, 140.0, 24.0, 18, 960),
        vec![
            wavy_ring(-25.0, 10.0, 26.0, 4.5, 7, 220),
            wavy_ring(35.0, -20.0, 18.0, 2.5, 5, 180),
        ],
    );
    let b = Polygon::new(
        wavy_ring(12.0, 6.0, 135.0, 22.0, 17, 920),
        vec![
            wavy_ring(-10.0, 18.0, 20.0, 2.0, 5, 180),
            wavy_ring(28.0, -14.0, 24.0, 3.5, 6, 210),
        ],
    );
    (a, b)
}

fn case_large_dense() -> (Polygon, Polygon) {
    let a = Polygon::new(wavy_ring(0.0, 0.0, 180.0, 20.0, 28, 1400), vec![]);
    let b = Polygon::new(wavy_ring(10.0, -8.0, 175.0, 18.0, 27, 1360), vec![]);
    (a, b)
}
