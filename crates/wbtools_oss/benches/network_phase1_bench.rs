use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::json;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use wbcore::{AllowAllCapabilities, ProgressSink, ToolArgs, ToolContext, ToolRuntimeRegistry};
use wbtools_oss::{register_default_tools, ToolRegistry};
use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, Layer, VectorFormat};

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

fn unique_tag(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}_{nanos}_{seq}")
}

struct Phase1NetworkFixture {
    network_path: PathBuf,
    origins_path: PathBuf,
    trajectory_path: PathBuf,
    temporal_csv_path: PathBuf,
    service_area_output_path: PathBuf,
    route_output_path: PathBuf,
    points_output_path: PathBuf,
    report_output_path: PathBuf,
}

impl Phase1NetworkFixture {
    fn new() -> Self {
        let tag = unique_tag("wbtools_oss_phase1_network_bench");
        let tmp = std::env::temp_dir();

        let fixture = Self {
            network_path: tmp.join(format!("{tag}_network.gpkg")),
            origins_path: tmp.join(format!("{tag}_origins.gpkg")),
            trajectory_path: tmp.join(format!("{tag}_trajectory.gpkg")),
            temporal_csv_path: tmp.join(format!("{tag}_temporal.csv")),
            service_area_output_path: tmp.join(format!("{tag}_service_area_out.gpkg")),
            route_output_path: tmp.join(format!("{tag}_route_out.gpkg")),
            points_output_path: tmp.join(format!("{tag}_points_out.gpkg")),
            report_output_path: tmp.join(format!("{tag}_report.json")),
        };

        fixture.write_inputs();
        fixture
    }

    fn write_inputs(&self) {
        let mut network = Layer::new("network")
            .with_geom_type(wbvector::GeometryType::LineString)
            .with_epsg(4326);
        network.schema.add_field(FieldDef::new("EDGE_ID", FieldType::Text));

        // Main corridor edges.
        network
            .add_feature(
                Some(Geometry::line_string(vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)])),
                &[("EDGE_ID", FieldValue::Text("A_B".to_string()))],
            )
            .expect("add edge A_B");
        network
            .add_feature(
                Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(2.0, 0.0)])),
                &[("EDGE_ID", FieldValue::Text("B_C".to_string()))],
            )
            .expect("add edge B_C");
        network
            .add_feature(
                Some(Geometry::line_string(vec![Coord::xy(2.0, 0.0), Coord::xy(3.0, 0.0)])),
                &[("EDGE_ID", FieldValue::Text("C_D".to_string()))],
            )
            .expect("add edge C_D");

        // Detour branch edges.
        network
            .add_feature(
                Some(Geometry::line_string(vec![Coord::xy(1.0, 0.0), Coord::xy(1.0, 1.0)])),
                &[("EDGE_ID", FieldValue::Text("B_E".to_string()))],
            )
            .expect("add edge B_E");
        network
            .add_feature(
                Some(Geometry::line_string(vec![Coord::xy(1.0, 1.0), Coord::xy(2.0, 1.0)])),
                &[("EDGE_ID", FieldValue::Text("E_F".to_string()))],
            )
            .expect("add edge E_F");
        network
            .add_feature(
                Some(Geometry::line_string(vec![Coord::xy(2.0, 1.0), Coord::xy(2.0, 0.0)])),
                &[("EDGE_ID", FieldValue::Text("F_C".to_string()))],
            )
            .expect("add edge F_C");

        wbvector::write(&network, &self.network_path, VectorFormat::GeoPackage)
            .expect("write network fixture");

        let mut origins = Layer::new("origins")
            .with_geom_type(wbvector::GeometryType::Point)
            .with_epsg(4326);
        origins
            .add_feature(Some(Geometry::Point(Coord::xy(0.0, 0.0))), &[])
            .expect("add origin");
        wbvector::write(&origins, &self.origins_path, VectorFormat::GeoPackage)
            .expect("write origins fixture");

        let mut trajectory = Layer::new("trajectory")
            .with_geom_type(wbvector::GeometryType::Point)
            .with_epsg(4326);
        trajectory.schema.add_field(FieldDef::new("TS", FieldType::Text));
        trajectory
            .add_feature(
                Some(Geometry::Point(Coord::xy(0.05, 0.03))),
                &[("TS", FieldValue::Text("2026-04-13T08:30:00Z".to_string()))],
            )
            .expect("add trajectory point 1");
        trajectory
            .add_feature(
                Some(Geometry::Point(Coord::xy(0.95, -0.02))),
                &[("TS", FieldValue::Text("2026-04-13T08:31:00Z".to_string()))],
            )
            .expect("add trajectory point 2");
        trajectory
            .add_feature(
                Some(Geometry::Point(Coord::xy(1.95, 0.02))),
                &[("TS", FieldValue::Text("2026-04-13T08:32:00Z".to_string()))],
            )
            .expect("add trajectory point 3");
        trajectory
            .add_feature(
                Some(Geometry::Point(Coord::xy(2.95, -0.01))),
                &[("TS", FieldValue::Text("2026-04-13T08:33:00Z".to_string()))],
            )
            .expect("add trajectory point 4");

        wbvector::write(&trajectory, &self.trajectory_path, VectorFormat::GeoPackage)
            .expect("write trajectory fixture");

        let temporal_csv = [
            "edge_id,dow,start_minute,end_minute,value",
            "A_B,1,420,600,3.0",
            "B_C,1,420,600,3.0",
            "C_D,1,420,600,3.0",
            "B_E,1,420,600,1.1",
            "E_F,1,420,600,1.1",
            "F_C,1,420,600,1.1",
        ]
        .join("\n");

        std::fs::write(&self.temporal_csv_path, temporal_csv).expect("write temporal profile csv fixture");
    }

    fn service_area_args(&self) -> ToolArgs {
        let mut args = ToolArgs::new();
        args.insert(
            "input".to_string(),
            json!(self.network_path.to_string_lossy().to_string()),
        );
        args.insert(
            "origins".to_string(),
            json!(self.origins_path.to_string_lossy().to_string()),
        );
        args.insert("max_cost".to_string(), json!(4.0));
        args.insert("output_mode".to_string(), json!("nodes"));
        args.insert("ring_costs".to_string(), json!("1.0,2.5,4.0"));
        args.insert(
            "temporal_cost_profile".to_string(),
            json!(self.temporal_csv_path.to_string_lossy().to_string()),
        );
        args.insert("departure_time".to_string(), json!("2026-04-13T08:30:00Z"));
        args.insert("temporal_mode".to_string(), json!("multiplier"));
        args.insert("temporal_fallback".to_string(), json!("static_cost"));
        args.insert(
            "output".to_string(),
            json!(self.service_area_output_path.to_string_lossy().to_string()),
        );
        args
    }

    fn map_matching_args(&self) -> ToolArgs {
        let mut args = ToolArgs::new();
        args.insert(
            "input".to_string(),
            json!(self.network_path.to_string_lossy().to_string()),
        );
        args.insert(
            "trajectory_points".to_string(),
            json!(self.trajectory_path.to_string_lossy().to_string()),
        );
        args.insert("timestamp_field".to_string(), json!("TS"));
        args.insert("search_radius".to_string(), json!(0.40));
        args.insert("candidate_k".to_string(), json!(5));
        args.insert(
            "matched_points_output".to_string(),
            json!(self.points_output_path.to_string_lossy().to_string()),
        );
        args.insert(
            "match_report".to_string(),
            json!(self.report_output_path.to_string_lossy().to_string()),
        );
        args.insert(
            "output".to_string(),
            json!(self.route_output_path.to_string_lossy().to_string()),
        );
        args
    }
}

impl Drop for Phase1NetworkFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.network_path);
        let _ = std::fs::remove_file(&self.origins_path);
        let _ = std::fs::remove_file(&self.trajectory_path);
        let _ = std::fs::remove_file(&self.temporal_csv_path);
        let _ = std::fs::remove_file(&self.service_area_output_path);
        let _ = std::fs::remove_file(&self.route_output_path);
        let _ = std::fs::remove_file(&self.points_output_path);
        let _ = std::fs::remove_file(&self.report_output_path);
    }
}

fn bench_phase1_network_tools(c: &mut Criterion) {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let ctx = make_ctx();

    let fixture = Phase1NetworkFixture::new();
    let service_area_args = fixture.service_area_args();
    let map_matching_args = fixture.map_matching_args();

    let mut group = c.benchmark_group("wbtools_oss/network_phase1");
    group.sample_size(10);

    group.bench_function("network_service_area_temporal_ring_nodes", |b| {
        b.iter(|| {
            let _ = std::fs::remove_file(&fixture.service_area_output_path);
            let out = registry
                .run_tool(
                    "network_service_area",
                    black_box(&service_area_args),
                    black_box(&ctx),
                )
                .expect("network_service_area benchmark run failed");
            black_box(out);
        })
    });

    group.bench_function("map_matching_v1_dp", |b| {
        b.iter(|| {
            let _ = std::fs::remove_file(&fixture.route_output_path);
            let _ = std::fs::remove_file(&fixture.points_output_path);
            let _ = std::fs::remove_file(&fixture.report_output_path);
            let out = registry
                .run_tool("map_matching_v1", black_box(&map_matching_args), black_box(&ctx))
                .expect("map_matching_v1 benchmark run failed");
            black_box(out);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_phase1_network_tools);
criterion_main!(benches);
