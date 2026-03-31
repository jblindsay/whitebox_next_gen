# Whitebox Workflows for Python — Precision Agriculture Tools

This document covers all **Precision Agriculture** tools exposed through the `WbEnvironment` API.
For common conventions, Raster I/O, and math operators see [TOOLS.md](../TOOLS.md).

---

## Precision Agriculture

The following Pro-tier helpers are available on `WbEnvironment` for agriculture workflows.
Each accepts either absolute output paths or paths relative to `wbe.working_directory`.

### `wbe.recreate_pass_lines`

```
recreate_pass_lines(
    input: Vector,
    output: str,
    yield_field_name: str,
    max_change_in_heading: float = 25.0,
    ignore_zeros: bool = False,
    output_points: str | None = None,
    callback: callable | None = None,
) -> tuple[Vector, Vector]
```

Returns `(pass_lines_vector, pass_points_vector)`.

### `wbe.yield_filter`

```
yield_filter(
    input: Vector,
    output: str,
    yield_field_name: str,
    pass_field_name: str,
    swath_width: float = 6.096,
    z_score_threshold: float = 2.5,
    min_yield: float = 0.0,
    max_yield: float = inf,
    callback: callable | None = None,
) -> Vector
```

### `wbe.yield_map`

```
yield_map(
    input: Vector,
    output: str,
    pass_field_name: str,
    swath_width: float = 6.096,
    max_change_in_heading: float = 25.0,
    callback: callable | None = None,
) -> Vector
```

### `wbe.yield_normalization`

```
yield_normalization(
    input: Vector,
    output: str,
    yield_field_name: str,
    radius: float = 0.0,
    standardize: bool = False,
    min_yield: float = 0.0,
    max_yield: float = inf,
    callback: callable | None = None,
) -> Vector
```

### `wbe.remove_field_edge_points`

```
remove_field_edge_points(
    input: Vector,
    output: str,
    radius: float,
    max_change_in_heading: float = 25.0,
    flag_edges: bool = False,
    callback: callable | None = None,
) -> Vector
```

### `wbe.reconcile_multiple_headers`

```
reconcile_multiple_headers(
    input: Vector,
    output: str,
    region_field_name: str,
    yield_field_name: str,
    radius: float,
    min_yield: float = 2.2250738585072014e-308,
    max_yield: float = inf,
    mean_tonnage: float = -inf,
    callback: callable | None = None,
) -> Vector
```

### Example

```python
pass_lines, pass_points = wbe.recreate_pass_lines(
    input=yield_points,
    output='pass_lines.gpkg',
    output_points='pass_points.gpkg',
    yield_field_name='YIELD',
)

filtered = wbe.yield_filter(
    input=pass_points,
    output='yield_filtered.gpkg',
    yield_field_name='YIELD',
    pass_field_name='PASS_NUM',
)

yield_polygons = wbe.yield_map(
    input=filtered,
    output='yield_map.gpkg',
    pass_field_name='PASS_NUM',
)
```

---

