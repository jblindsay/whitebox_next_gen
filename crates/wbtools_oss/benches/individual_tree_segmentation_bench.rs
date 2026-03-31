use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use serde_json::json;
use wbcore::{AllowAllCapabilities, ProgressSink, ToolArgs, ToolContext, ToolRuntimeRegistry};
use wblidar::{PointCloud, PointRecord};
use wbtools_oss::{register_default_tools, ToolRegistry};

struct NoopProgress;
impl ProgressSink for NoopProgress {}

fn make_ctx() -> ToolContext<'static> {
    static PROGRESS: NoopProgress = NoopProgress;
    static CAPS: AllowAllCapabilities = AllowAllCapabilities;
    ToolContext {
        progress: &PROGRESS,
        capabilities: &CAPS,
    }
}

fn unique_temp_las_path(tag: &str) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    let pid = std::process::id();
    std::env::temp_dir().join(format!("wbw_{}_{}_{}.las", tag, pid, now))
}

fn synthetic_tree_cloud(tree_count: usize, points_per_tree: usize) -> PointCloud {
    let cols = ((tree_count as f64).sqrt().ceil() as usize).max(1);
    let spacing = 6.5;
    let mut points = Vec::with_capacity(tree_count * points_per_tree + tree_count * 6);

    for t in 0..tree_count {
        let row = t / cols;
        let col = t % cols;
        let cx = col as f64 * spacing;
        let cy = row as f64 * spacing;

        // Canopy points (vegetation class 5)
        for p in 0..points_per_tree {
            let angle = ((p * 137) % 360) as f64 * std::f64::consts::PI / 180.0;
            let radial = 0.25 + ((p % 29) as f64 / 28.0) * 2.4;
            let x = cx + radial * angle.cos();
            let y = cy + radial * angle.sin();
            let z = 2.2 + ((p % 41) as f64 / 40.0) * 13.0 - radial * 0.35;

            let mut pt = PointRecord {
                x,
                y,
                z,
                classification: 5,
                ..PointRecord::default()
            };
            pt.intensity = ((p * 17 + t * 11) % 2048) as u16;
            points.push(pt);
        }

        // Some non-vegetation points mixed in to exercise filters.
        for g in 0..6usize {
            let gx = cx + (g as f64 - 2.5) * 0.8;
            let gy = cy + ((g * 3 % 5) as f64 - 2.0) * 0.8;
            points.push(PointRecord {
                x: gx,
                y: gy,
                z: 0.4 + (g as f64) * 0.03,
                classification: 2,
                ..PointRecord::default()
            });
        }
    }

    PointCloud { points, crs: None }
}

#[derive(Clone, Copy)]
struct BenchVariant {
    name: &'static str,
    grid_acceleration: bool,
    grid_refine_exact: bool,
    grid_refine_iterations: usize,
    tile_size: f64,
    tile_overlap: f64,
    threads: usize,
}

fn make_tree_segmentation_args(input_path: &str, output_path: &str, variant: BenchVariant) -> ToolArgs {
    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path));
    args.insert("output".to_string(), json!(output_path));
    args.insert("only_use_veg".to_string(), json!(true));
    args.insert("veg_classes".to_string(), json!("3,4,5"));
    args.insert("min_height".to_string(), json!(2.0));
    args.insert("bandwidth_min".to_string(), json!(0.9));
    args.insert("bandwidth_max".to_string(), json!(3.8));
    args.insert("adaptive_bandwidth".to_string(), json!(true));
    args.insert("adaptive_neighbors".to_string(), json!(24));
    args.insert("adaptive_sector_count".to_string(), json!(8));
    args.insert("grid_acceleration".to_string(), json!(variant.grid_acceleration));
    args.insert("grid_cell_size".to_string(), json!(0.75));
    args.insert("grid_refine_exact".to_string(), json!(variant.grid_refine_exact));
    args.insert("grid_refine_iterations".to_string(), json!(variant.grid_refine_iterations));
    args.insert("tile_size".to_string(), json!(variant.tile_size));
    args.insert("tile_overlap".to_string(), json!(variant.tile_overlap));
    args.insert("vertical_bandwidth".to_string(), json!(4.5));
    args.insert("max_iterations".to_string(), json!(20));
    args.insert("convergence_tol".to_string(), json!(0.05));
    args.insert("min_cluster_points".to_string(), json!(24));
    args.insert("mode_merge_dist".to_string(), json!(0.9));
    args.insert("simd".to_string(), json!(true));
    args.insert("threads".to_string(), json!(variant.threads));
    args.insert("seed".to_string(), json!(1));
    args.insert("output_id_mode".to_string(), json!("point_source_id"));
    args
}

fn run_and_read_output(registry: &ToolRegistry, ctx: &ToolContext, args: &ToolArgs) -> PointCloud {
    let out = registry
        .run_tool("individual_tree_segmentation", args, ctx)
        .expect("individual_tree_segmentation run failed");
    let output_path = out
        .outputs
        .get("path")
        .and_then(|v| v.as_str())
        .expect("missing output path");
    PointCloud::read(output_path).expect("failed to read output point cloud")
}

fn assigned_stats(cloud: &PointCloud) -> (usize, usize) {
    let mut assigned = 0usize;
    let mut cluster_ids = HashSet::new();
    for p in &cloud.points {
        if p.classification == 5 && p.z >= 2.0 {
            if p.point_source_id > 0 {
                assigned += 1;
                cluster_ids.insert(p.point_source_id);
            }
        }
    }
    (assigned, cluster_ids.len())
}

fn report_quality_metrics(
    label: &str,
    variant: &str,
    total_points: usize,
    exact_assigned: usize,
    exact_clusters: usize,
    variant_assigned: usize,
    variant_clusters: usize,
) {
    let assigned_ratio = if exact_assigned > 0 {
        variant_assigned as f64 / exact_assigned as f64
    } else {
        0.0
    };
    let cluster_ratio = if exact_clusters > 0 {
        variant_clusters as f64 / exact_clusters as f64
    } else {
        0.0
    };
    eprintln!(
        "QUALITY_METRICS scenario={label} variant={variant} total_points={total_points} exact_assigned={exact_assigned} variant_assigned={variant_assigned} exact_clusters={exact_clusters} variant_clusters={variant_clusters} assigned_ratio={assigned_ratio:.4} cluster_ratio={cluster_ratio:.4}"
    );
}

fn bench_individual_tree_segmentation(c: &mut Criterion) {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let ctx = make_ctx();

    let mut group = c.benchmark_group("wbtools_oss/individual_tree_segmentation");
    group.sample_size(10);

    let scenarios = [("small", 32usize, 180usize), ("medium", 64usize, 220usize)];

    let exact_variant = BenchVariant {
        name: "exact",
        grid_acceleration: false,
        grid_refine_exact: false,
        grid_refine_iterations: 2,
        tile_size: 0.0,
        tile_overlap: 0.0,
        threads: 0,
    };

    for (label, tree_count, points_per_tree) in scenarios {
        let cloud = synthetic_tree_cloud(tree_count, points_per_tree);
        let input_path = unique_temp_las_path(&format!("its_in_{}", label));
        cloud
            .write(&input_path)
            .expect("failed to write synthetic input LAS");

        let mut variants = vec![
            BenchVariant {
                name: "grid_accel",
                grid_acceleration: true,
                grid_refine_exact: false,
                grid_refine_iterations: 2,
                tile_size: 0.0,
                tile_overlap: 0.0,
                threads: 0,
            },
            BenchVariant {
                name: "grid_refine",
                grid_acceleration: true,
                grid_refine_exact: true,
                grid_refine_iterations: 2,
                tile_size: 0.0,
                tile_overlap: 0.0,
                threads: 0,
            },
            BenchVariant {
                name: "grid_refine_tiled",
                grid_acceleration: true,
                grid_refine_exact: true,
                grid_refine_iterations: 2,
                tile_size: 6.0,
                tile_overlap: 1.0,
                threads: 0,
            },
            BenchVariant {
                name: "grid_refine_tiled_t1",
                grid_acceleration: true,
                grid_refine_exact: true,
                grid_refine_iterations: 2,
                tile_size: 6.0,
                tile_overlap: 1.0,
                threads: 1,
            },
        ];

        if label == "medium" {
            variants.push(BenchVariant {
                name: "grid_refine_tiled_t4",
                grid_acceleration: true,
                grid_refine_exact: true,
                grid_refine_iterations: 2,
                tile_size: 6.0,
                tile_overlap: 1.0,
                threads: 4,
            });
        }

        let exact_out = unique_temp_las_path(&format!("its_{}_exact", label));
        let exact_args = make_tree_segmentation_args(
            &input_path.to_string_lossy(),
            &exact_out.to_string_lossy(),
            exact_variant,
        );

        // One-time quality sanity check so performance runs compare plausible outputs.
        let exact_cloud = run_and_read_output(&registry, &ctx, &exact_args);
        let (exact_assigned, exact_clusters) = assigned_stats(&exact_cloud);
        assert!(exact_assigned > 0, "exact path assigned no vegetation points");

        for variant in &variants {
            let out_path = unique_temp_las_path(&format!("its_{}_{}", label, variant.name));
            let args = make_tree_segmentation_args(
                &input_path.to_string_lossy(),
                &out_path.to_string_lossy(),
                *variant,
            );
            let variant_cloud = run_and_read_output(&registry, &ctx, &args);
            let (variant_assigned, variant_clusters) = assigned_stats(&variant_cloud);
            assert!(variant_assigned > 0, "{} path assigned no vegetation points", variant.name);
            assert!(
                variant_assigned as f64 >= exact_assigned as f64 * 0.80,
                "{} assigned too few points: variant={}, exact={}",
                variant.name,
                variant_assigned,
                exact_assigned
            );
            assert!(
                exact_clusters > 0 && variant_clusters > 0,
                "{} produced zero clusters",
                variant.name
            );
            let cluster_ratio = variant_clusters as f64 / exact_clusters as f64;
            assert!(
                (0.35..=2.8).contains(&cluster_ratio),
                "cluster-count drift too large for {}: variant={}, exact={}",
                variant.name,
                variant_clusters,
                exact_clusters
            );
            report_quality_metrics(
                label,
                variant.name,
                cloud.points.len(),
                exact_assigned,
                exact_clusters,
                variant_assigned,
                variant_clusters,
            );
            let _ = std::fs::remove_file(&out_path);
        }

        group.bench_with_input(
            BenchmarkId::new("exact", format!("{}_t{}_p{}", label, tree_count, points_per_tree)),
            &(tree_count, points_per_tree),
            |b, _| {
                b.iter(|| {
                    let out = registry
                        .run_tool("individual_tree_segmentation", black_box(&exact_args), black_box(&ctx))
                        .expect("exact benchmark run failed");
                    black_box(out);
                })
            },
        );

        for variant in variants {
            let variant_out = unique_temp_las_path(&format!("its_{}_{}_bench", label, variant.name));
            let variant_args = make_tree_segmentation_args(
                &input_path.to_string_lossy(),
                &variant_out.to_string_lossy(),
                variant,
            );
            group.bench_with_input(
                BenchmarkId::new(
                    variant.name,
                    format!("{}_t{}_p{}", label, tree_count, points_per_tree),
                ),
                &(tree_count, points_per_tree),
                |b, _| {
                    b.iter(|| {
                        let out = registry
                            .run_tool(
                                "individual_tree_segmentation",
                                black_box(&variant_args),
                                black_box(&ctx),
                            )
                            .expect("variant benchmark run failed");
                        black_box(out);
                    })
                },
            );
            let _ = std::fs::remove_file(&variant_out);
        }

        let _ = std::fs::remove_file(&input_path);
        let _ = std::fs::remove_file(&exact_out);
    }

    group.finish();
}

criterion_group!(benches, bench_individual_tree_segmentation);
criterion_main!(benches);
