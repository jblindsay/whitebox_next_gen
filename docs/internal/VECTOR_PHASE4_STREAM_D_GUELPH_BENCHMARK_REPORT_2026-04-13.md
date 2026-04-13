# Vector Phase 4 Stream D Guelph Benchmark Report

Date: 2026-04-13
Scope: Stream D large-network benchmark report and runtime targets

## Fixture and Setup

Source fixture files:
- /Users/johnlindsay/Documents/data/Guelph_data/StreetCentrelines.shp
- /Users/johnlindsay/Documents/data/Guelph_data/Hydrants.shp

Benchmark preparation:
- Converted StreetCentrelines to 2D GeoPackage for strict line-type validation compatibility.
- Converted Hydrants to 2D GeoPackage and sampled first 80 points for tractable repeated runs.

Prepared benchmark layers:
- target/benchmarks/phase4_stream_d/guelph_network_2d.gpkg
- target/benchmarks/phase4_stream_d/guelph_points_2d.gpkg

Representative dataset size:
- Network edges: 3579
- Sampled points: 80 origins and 80 destinations

Execution tool:
- target/debug/examples/run_tool

## Benchmark Commands

Network accessibility benchmark args:
- tool: network_accessibility_metrics
- max_snap_distance: 50.0
- impedance_cutoff: 1000.0
- decay_function: exponential
- decay_parameter: 0.002
- edge_cost_field: ROADLENGTH
- parallel_execution: true/false

OD sensitivity benchmark args:
- tool: od_sensitivity_analysis
- edge_cost_field: ROADLENGTH
- max_snap_distance: 50.0
- impedance_disturbance_range: 0.9,1.1
- monte_carlo_samples: 5
- parallel_execution: true/false

Each configuration was run 3 times and timed using /usr/bin/time -p.
Raw timings table:
- target/benchmarks/phase4_stream_d/timings.tsv

## Results

Mean wall-clock runtime (seconds):

| Tool | parallel_execution=true | parallel_execution=false | Speedup |
|---|---:|---:|---:|
| network_accessibility_metrics | 0.0600 | 0.1200 | 2.00x |
| od_sensitivity_analysis | 0.0933 | 0.3767 | 4.04x |

Observed spread across the 3 runs:
- network_accessibility_metrics parallel=true: 0.06 to 0.06 s
- network_accessibility_metrics parallel=false: 0.12 to 0.12 s
- od_sensitivity_analysis parallel=true: 0.09 to 0.10 s
- od_sensitivity_analysis parallel=false: 0.37 to 0.38 s

## Runtime Targets

Targets for this fixture class (approximately 3.6k-edge network and 80x80 OD points):
- network_accessibility_metrics with parallel_execution=true: <= 0.10 s mean
- network_accessibility_metrics with parallel_execution=false: <= 0.15 s mean
- od_sensitivity_analysis with parallel_execution=true and 5 samples: <= 0.15 s mean
- od_sensitivity_analysis with parallel_execution=false and 5 samples: <= 0.45 s mean

Status against targets:
- All targets met on 2026-04-13 benchmark run.

## Notes

- The benchmark confirms that Stream D parallel execution paths are materially faster than sequential execution on a realistic municipal network fixture.
- The OD sensitivity workflow gains more from parallel execution due to repeated shortest-path sampling work per origin.
