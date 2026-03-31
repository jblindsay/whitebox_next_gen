use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use wbtopology::{Coord, Geometry, SpatialIndex};

fn synthetic_points(n: usize) -> Vec<Geometry> {
    // Lightweight deterministic LCG to avoid extra benchmark dependencies.
    let mut s = 0x9E3779B97F4A7C15u64;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        let x = ((s >> 11) as f64) / ((1u64 << 53) as f64) * 10_000.0;
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        let y = ((s >> 11) as f64) / ((1u64 << 53) as f64) * 10_000.0;
        out.push(Geometry::Point(Coord::xy(x, y)));
    }
    out
}

fn bench_nearest_k(c: &mut Criterion) {
    let mut group = c.benchmark_group("spatial_index/nearest_k");
    for &n in &[10_000usize, 50_000usize, 100_000usize] {
        let geoms = synthetic_points(n);
        let idx = SpatialIndex::build_str(&geoms, 8);
        let target = Geometry::Point(Coord::xy(5000.0, 5000.0));

        group.bench_with_input(BenchmarkId::new("k=8", n), &n, |b, _| {
            b.iter(|| {
                let out = idx.nearest_k(black_box(&target), black_box(8));
                black_box(out)
            })
        });

        group.bench_with_input(BenchmarkId::new("k=32", n), &n, |b, _| {
            b.iter(|| {
                let out = idx.nearest_k(black_box(&target), black_box(32));
                black_box(out)
            })
        });
    }
    group.finish();
}

fn bench_remove_rebuild(c: &mut Criterion) {
    let mut group = c.benchmark_group("spatial_index/remove_rebuild");
    for &n in &[20_000usize, 100_000usize] {
        let geoms = synthetic_points(n);
        group.bench_with_input(BenchmarkId::new("remove_every_3rd", n), &n, |b, _| {
            b.iter(|| {
                let mut idx = SpatialIndex::build_str(&geoms, 8);
                for id in (0..n).step_by(3) {
                    idx.remove(black_box(id));
                }
                black_box(idx.len())
            })
        });
    }
    group.finish();
}

fn bench_compact(c: &mut Criterion) {
    let mut group = c.benchmark_group("spatial_index/compact");
    for &n in &[20_000usize, 100_000usize] {
        let geoms = synthetic_points(n);
        group.bench_with_input(BenchmarkId::new("compact_after_tombstones", n), &n, |b, _| {
            b.iter(|| {
                let mut idx = SpatialIndex::build_str(&geoms, 8);
                for id in (0..n).step_by(4) {
                    idx.remove(id);
                }
                idx.compact();
                black_box(idx.len())
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_nearest_k,
    bench_remove_rebuild,
    bench_compact
);
criterion_main!(benches);
