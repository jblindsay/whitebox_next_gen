use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use wbtopology::{
    concave_hull,
    concave_hull_with_options,
    convex_hull,
    ConcaveHullEngine,
    ConcaveHullOptions,
    Coord,
};

fn synthetic_ring_points(n: usize, radius: f64, wobble: f64) -> Vec<Coord> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = (i as f64) * std::f64::consts::TAU / (n as f64);
        let r = radius + wobble * (7.0 * t).sin() + 0.5 * wobble * (13.0 * t).cos();
        out.push(Coord::xy(r * t.cos(), r * t.sin()));
    }
    out
}

fn bench_convex_hull(c: &mut Criterion) {
    let mut group = c.benchmark_group("hull/convex");
    for &n in &[1_000usize, 10_000usize, 50_000usize] {
        let pts = synthetic_ring_points(n, 100.0, 20.0);
        group.bench_with_input(BenchmarkId::new("point_cloud", n), &n, |b, _| {
            b.iter(|| black_box(convex_hull(black_box(&pts), black_box(1.0e-12))))
        });
    }
    group.finish();
}

fn bench_concave_hull_absolute(c: &mut Criterion) {
    let mut group = c.benchmark_group("hull/concave_absolute");
    for &n in &[1_000usize, 5_000usize, 10_000usize] {
        let pts = synthetic_ring_points(n, 100.0, 20.0);
        group.bench_with_input(BenchmarkId::new("max_edge=10", n), &n, |b, _| {
            b.iter(|| black_box(concave_hull(black_box(&pts), black_box(10.0), black_box(1.0e-12))))
        });
    }
    group.finish();
}

fn bench_concave_hull_relative(c: &mut Criterion) {
    let mut group = c.benchmark_group("hull/concave_relative");
    for &n in &[1_000usize, 5_000usize, 10_000usize] {
        let pts = synthetic_ring_points(n, 100.0, 20.0);
        group.bench_with_input(BenchmarkId::new("ratio=0.08", n), &n, |b, _| {
            b.iter(|| {
                black_box(concave_hull_with_options(
                    black_box(&pts),
                    black_box(ConcaveHullOptions {
                        relative_edge_length_ratio: Some(0.08),
                        epsilon: 1.0e-12,
                        ..Default::default()
                    }),
                ))
            })
        });
    }
    group.finish();
}

fn bench_concave_hull_engines(c: &mut Criterion) {
    let mut group = c.benchmark_group("hull/concave_engines");
    for &n in &[1_000usize, 5_000usize, 10_000usize] {
        let pts = synthetic_ring_points(n, 100.0, 20.0);
        group.bench_with_input(BenchmarkId::new("delaunay/max_edge=10", n), &n, |b, _| {
            b.iter(|| {
                black_box(concave_hull_with_options(
                    black_box(&pts),
                    black_box(ConcaveHullOptions {
                        engine: ConcaveHullEngine::Delaunay,
                        max_edge_length: 10.0,
                        epsilon: 1.0e-12,
                        ..Default::default()
                    }),
                ))
            })
        });

        group.bench_with_input(BenchmarkId::new("fast_refine/max_edge=10", n), &n, |b, _| {
            b.iter(|| {
                black_box(concave_hull_with_options(
                    black_box(&pts),
                    black_box(ConcaveHullOptions {
                        engine: ConcaveHullEngine::FastRefine,
                        max_edge_length: 10.0,
                        epsilon: 1.0e-12,
                        ..Default::default()
                    }),
                ))
            })
        });
    }
    group.finish();
}

fn bench_concave_hull_engines_relative(c: &mut Criterion) {
    let mut group = c.benchmark_group("hull/concave_engines_relative");
    for &n in &[1_000usize, 5_000usize, 10_000usize] {
        let pts = synthetic_ring_points(n, 100.0, 20.0);
        group.bench_with_input(BenchmarkId::new("delaunay/ratio=0.08", n), &n, |b, _| {
            b.iter(|| {
                black_box(concave_hull_with_options(
                    black_box(&pts),
                    black_box(ConcaveHullOptions {
                        engine: ConcaveHullEngine::Delaunay,
                        relative_edge_length_ratio: Some(0.08),
                        epsilon: 1.0e-12,
                        ..Default::default()
                    }),
                ))
            })
        });

        group.bench_with_input(BenchmarkId::new("fast_refine/ratio=0.08", n), &n, |b, _| {
            b.iter(|| {
                black_box(concave_hull_with_options(
                    black_box(&pts),
                    black_box(ConcaveHullOptions {
                        engine: ConcaveHullEngine::FastRefine,
                        relative_edge_length_ratio: Some(0.08),
                        epsilon: 1.0e-12,
                        ..Default::default()
                    }),
                ))
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_convex_hull,
    bench_concave_hull_absolute,
    bench_concave_hull_relative,
    bench_concave_hull_engines,
    bench_concave_hull_engines_relative
);
criterion_main!(benches);
