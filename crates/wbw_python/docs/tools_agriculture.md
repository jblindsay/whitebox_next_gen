# Whitebox Workflows for Python — Precision Agriculture Tools

This document covers **Precision Agriculture** workflows exposed through the `WbEnvironment` API.
For common conventions, Raster I/O, and math operators see [TOOLS.md](../TOOLS.md).

---

## Precision Agriculture

Precision agriculture in the Python API is exposed as a packaged Pro workflow.
The former standalone helper methods are intentionally not exposed in the public Python surface.

### Workflow Product Index

- `wbe.yield_data_conditioning_and_qa`
- `wbe.precision_irrigation_optimization`
- `wbe.precision_ag_yield_zone_intelligence`

### `wbe.yield_data_conditioning_and_qa`

```
yield_data_conditioning_and_qa(
    input: Vector,
    output_prefix: str = "yield_pipeline",
    yield_field_name: str = "YIELD",
    profile: str = "balanced",
    swath_width: float = 6.096,
    edge_radius: float | None = None,
    reconcile_radius: float | None = None,
    normalization_radius: float | None = None,
    z_score_threshold: float | None = None,
    min_yield: float | None = None,
    max_yield: float | None = None,
    mean_tonnage: float | None = None,
    header_field_name: str = "HEADER",
    use_field_aliases: bool = True,
    moisture_field_name: str | None = None,
    target_moisture_pct: float = 15.5,
    speed_field_name: str | None = None,
    heading_field_name: str | None = None,
    min_speed_kmh: float = 1.0,
    max_speed_kmh: float = 18.0,
    max_heading_change_deg: float = 35.0,
    lag_correction_mode: str = "none",
    lag_distance_m: float = 0.0,
    filtering_mode: str = "standard",
    robust_mad_threshold: float = 3.0,
    standardize: bool = False,
    ignore_zeros: bool = False,
    max_change_in_heading: float = 25.0,
    callback: callable | None = None,
) -> tuple[Vector, Vector, Vector, Vector, str]
```

Returns a 5-tuple in this order:

1. `qa_flags_vector`
2. `clean_points_vector`
3. `clean_map_vector`
4. `confidence_points_vector`
5. `summary_json_path`

### Example

```python
qa_flags, clean_points, clean_map, confidence_points, summary_json = wbe.yield_data_conditioning_and_qa(
    input=yield_points,
    output_prefix='output/yield_pipeline',
    profile='balanced',
    use_field_aliases=True,
    filtering_mode='robust',
)
```

---

### `wbe.precision_irrigation_optimization`

```
precision_irrigation_optimization(
    dem: Raster,
    field_boundary: Vector,
    output_prefix: str = "irrigation_opt",
    profile: str = "balanced",
    callback: callable | None = None,
) -> tuple[Raster, Raster, Vector, str]
```

Returns a 4-tuple in this order:

1. `irrigation_demand_raster`
2. `stress_risk_raster`
3. `management_zones_vector`
4. `summary_json_path`

### Example

```python
demand, stress, zones, summary = wbe.precision_irrigation_optimization(
    dem=dem,
    field_boundary=field_boundary,
    profile='balanced',
)
```

---

### `wbe.precision_ag_yield_zone_intelligence`

```
precision_ag_yield_zone_intelligence(
    dem: Raster,
    field_boundary: Vector,
    output_prefix: str = "yield_zone_intelligence",
    callback: callable | None = None,
) -> tuple[Raster, Raster, Vector, str]
```

Returns a 4-tuple in this order:

1. `yield_stability_raster`
2. `nutrient_transport_raster`
3. `management_zones_vector`
4. `summary_json_path`

### Example

```python
stability, nutrient, zones, summary = wbe.precision_ag_yield_zone_intelligence(
    dem=dem,
    field_boundary=field_boundary,
)
```

---

