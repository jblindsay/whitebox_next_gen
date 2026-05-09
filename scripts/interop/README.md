# Interop Scripts

This folder holds local/manual orchestration scripts for:
- Projection conformance (Phase A)
- Cross-ecosystem interop matrix (Phase B)
- Topology stress corpus (Phase C, v1.5)

Run scripts from repo root unless noted otherwise.

Planned scripts:
- generate_projection_reference.sh
- run_projection_conformance.sh
- run_roundtrip_raster.sh
- run_roundtrip_vector.sh
- run_roundtrip_lidar.sh
- summarize_results.py

Current Phase C helper:
- generate_phase_c_topology_synthetic_fixtures.py
- phase_c_topology_test.py
- phase_c1_format_expansion_test.py

Example:
- python scripts/interop/generate_phase_c_topology_synthetic_fixtures.py
- python scripts/interop/phase_c_topology_test.py --corpus all
- python scripts/interop/phase_c1_format_expansion_test.py
