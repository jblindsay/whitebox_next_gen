# Phase 4 Regression Gates and Release Checklist

Date: 2026-04-13
Status: Drafted for Stream E closeout

## Scope

This checklist covers Phase 4 additions in `wbtools_oss`:
- Stream A: CVRP/VRPTW solver refinements.
- Stream B: multimodal batched OD matrix and route exports.
- Stream C: temporal profiles, scenario bundles, and uncertainty fixture coverage.
- Stream D: snapping/query reuse/parallel execution and large-network benchmarks.

## Regression Gates

Run these commands from repository root.

### 1) Targeted Phase 4 integration tests

```bash
cargo test -p wbtools_oss --test registry_integration vehicle_routing_cvrp_local_optimization_reduces_route_distance -- --nocapture
cargo test -p wbtools_oss --test registry_integration vehicle_routing_vrptw_benchmark_priority_scoring_reduces_total_lateness_vs_phase3_baseline -- --nocapture
cargo test -p wbtools_oss --test registry_integration multimodal_od_cost_matrix_writes_expected_batch_rows -- --nocapture
cargo test -p wbtools_oss --test registry_integration multimodal_routes_from_od_outputs_route_geometry_and_modes -- --nocapture
cargo test -p wbtools_oss --test registry_integration multimodal_od_cost_matrix_applies_temporal_cost_profile -- --nocapture
cargo test -p wbtools_oss --test registry_integration multimodal_routes_from_od_writes_scenario_bundle_outputs -- --nocapture
cargo test -p wbtools_oss --test registry_integration stream_c_temporal_routing_benchmark_fixture_switches_modes_across_scenarios -- --nocapture
cargo test -p wbtools_oss --test registry_integration stream_c_uncertainty_benchmark_fixture_wider_disturbance_increases_variance -- --nocapture
cargo test -p wbtools_oss --test registry_integration stream_d_accessibility_metrics_benchmark_validates_impedance_cutoff_and_decay_combinations -- --nocapture
cargo test -p wbtools_oss --test registry_integration stream_d_od_sensitivity_analysis_benchmark_validates_scaling_with_network_and_sample_size -- --nocapture
```

### 2) Stream D parallel controls sanity checks

```bash
cargo test -p wbtools_oss --test registry_integration network_accessibility_metrics_computes_weighted_accessibility_by_cutoff_and_decay -- --nocapture
cargo test -p wbtools_oss --test registry_integration od_sensitivity_analysis_computes_perturbed_od_costs_with_variance -- --nocapture
```

### 3) Compile gate

```bash
cargo check -p wbtools_oss
```

## Optional Wrapper Surface Checks

These are smoke checks that the new tool APIs are reachable from wrapper surfaces.

### Python

```bash
python -c "from wbw import WbEnvironment; wb = WbEnvironment(); ids = wb.tool_ids(); required = ['multimodal_od_cost_matrix','multimodal_routes_from_od','network_accessibility_metrics','od_sensitivity_analysis']; print(all(x in ids for x in required))"
```

### R

```bash
Rscript -e "library(whiteboxworkflows); s <- wbw_session(); ids <- wbw_tool_ids(session=s); req <- c('multimodal_od_cost_matrix','multimodal_routes_from_od','network_accessibility_metrics','od_sensitivity_analysis'); cat(all(req %in% ids), '\n')"
```

## Benchmark Artifact Checks

Ensure these outputs exist for the large-network benchmark publication:
- `docs/internal/VECTOR_PHASE4_STREAM_D_GUELPH_BENCHMARK_REPORT_2026-04-13.md`
- `target/benchmarks/phase4_stream_d/timings.tsv`

## Release Checklist

- [ ] All regression gate commands pass on the release candidate branch.
- [ ] No new failing integration tests introduced by Phase 4 changes.
- [ ] Stream D benchmark report remains reproducible with documented fixture setup.
- [ ] Python and R wrappers expose Phase 4 tools (smoke checks pass).
- [ ] Changelog entries updated for any crate modified after this checklist publication.
- [ ] Final Phase 4 tracker status aligned with completed Stream E tasks.

## Notes

- This checklist is intentionally focused on high-signal Phase 4 tests and compile gates.
- Full-suite test execution may still include unrelated known failures outside Phase 4 scope; those should be triaged separately from this release checklist.
