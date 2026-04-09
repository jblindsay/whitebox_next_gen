# WbW Python API — v2.0 Migration Guide

## Overview

WbW Python v2.0 introduces a **category-first** and **domain-first** API as the primary
interface. The legacy flat API (`wbe.tool_name(...)`) is **deprecated** in v2.0 and will
be removed in a future release. Flat methods will continue to work in v2.0 but IDE
autocomplete and documentation will no longer promote them.

---

## What Changed

### 1. Category-first access (primary API)

Each tool is now accessed through its data-type category:

| Category accessor      | Tool types                                |
|------------------------|-------------------------------------------|
| `wbe.raster`           | Raster analysis and processing            |
| `wbe.vector`           | Vector / attribute operations             |
| `wbe.lidar`            | LiDAR point cloud tools                   |
| `wbe.topology`         | Topology repair and analysis              |
| `wbe.hydrology`        | Hydrology and watershed analysis          |
| `wbe.remote_sensing`   | Remote-sensing filters, enhancement, and image analytics |
| `wbe.streams`          | Stream extraction, ordering, and analysis |
| `wbe.terrain`          | Terrain analysis (core)                   |
| `wbe.conversion`       | Format conversion tools                   |
| `wbe.other`            | Miscellaneous tools                       |

```python
# v1 (deprecated)
result = wbe.slope(dem, units="degrees")

# v2 (preferred)
result = wbe.terrain.slope(dem, units="degrees")

# v2 (optional subcategory browsing for autocomplete)
result = wbe.terrain.derivatives.slope(dem=dem, units="degrees")
```

For large categories, optional subcategories improve discoverability without
breaking existing paths. For example:

- `wbe.raster.overlay_math.*`
- `wbe.remote_sensing.filters.*`
- `wbe.terrain.derivatives.*`

The `other` bucket remains accessible through `wbe.other` for compatibility,
but may be omitted from `wbe.categories()` when empty.

### 2. Domain namespace access (workflow-oriented)

Domain namespace access remains available for precision-ag workflows:

| Domain accessor             | Contains                                         |
|-----------------------------|--------------------------------------------------|
| `wbe.precision_agriculture` | Yield, zoning, irrigation tools                  |

```python
# SAR interferogram via category namespace
coherence = wbe.remote_sensing.sar_interferogram_coherence(ref, moving, ...)

# Precision-ag yield zone via domain namespace
zones = wbe.precision_agriculture.precision_ag_yield_zone_intelligence(yield_surf, ...)
```

Dynamic namespace lookup:
```python
# Programmatic domain access
ns = wbe.domain("precision_agriculture")
ns.list_tools()                # list all tools in the domain
ns.list_tools(include_pro_markers=True)   # prefix locked tools with "[PRO]"
tool = ns.yield_data_conditioning_and_qa   # returns a callable
tool(input=yield_points)
```

### 3. PRO tool markers

All PRO-tier tools are now explicitly labelled:

- **IDE autocomplete**: methods appear with `[PRO]` in their docstring when hovering.
- **Runtime error**: calling a PRO tool without a valid entitlement raises a
  `RuntimeError` with a message like:
  ```
  This is a PRO tool: sar_interferogram_coherence. Current runtime: include_pro=False,
  tier=open, effective_tier=open. Reason: pro_not_included. Action: …
  ```
- **Discovery APIs**: `describe_tool()`, `search_tools()`, and `list_tools_detailed()`
  all return `"is_pro": True` and `"available_in_current_session": False` for locked tools.

### 4. Discovery APIs

```python
# Describe a single tool (works even for locked PRO tools with include_locked=True)
info = wbe.describe_tool("sar_coregistration", include_locked=True)
# → {"id": ..., "display_name": ..., "is_pro": True, "available_in_current_session": False, ...}

# Full-text search (id, name, summary, tags)
results = wbe.search_tools("coherence")
results = wbe.search_tools("SAR", include_locked=True)   # show locked PRO tools too

# List everything
all_tools = wbe.list_tools_detailed()
all_tools = wbe.list_tools_detailed(include_locked=True)
```

Each result dict has:

| Field                        | Type      | Description                                       |
|------------------------------|-----------|---------------------------------------------------|
| `id`                         | `str`     | Canonical tool ID                                 |
| `display_name`               | `str`     | Human-readable name                               |
| `summary`                    | `str`     | One-line description                              |
| `category`                   | `str`     | Data-type category (`"Terrain"`, `"Raster"`, …)  |
| `license_tier`               | `str`     | `"open"`, `"pro"`, or `"enterprise"`              |
| `is_pro`                     | `bool`    | `True` if requires Pro/Enterprise licence         |
| `available_in_current_session` | `bool`  | `True` if callable right now                      |
| `availability_reason`        | `str`     | `"available"`, `"pro_not_included_or_tier_insufficient"` |
| `tags`                       | `list[str]` | Semantic tags                                   |
| `params`                     | `list[dict]` | Parameter name/description/required info       |

---

## v1 → v2 Quick Reference

| v1 (deprecated)                             | v2 (preferred)                                        |
|---------------------------------------------|-------------------------------------------------------|
| `wbe.slope(dem)`                            | `wbe.terrain.slope(dem)`                              |
| `wbe.fill_depressions(dem)`                 | `wbe.hydrology.fill_depressions(dem)`                 |
| `wbe.lidar_tin_gridding(las)`               | `wbe.lidar.lidar_tin_gridding(las)`                   |
| `wbe.sar_coregistration(ref, mov)`          | `wbe.remote_sensing.sar_coregistration(ref, mov)`     |
| `wbe.yield_data_conditioning_and_qa(vec)`   | `wbe.precision_agriculture.yield_data_conditioning_and_qa(vec)` |
| `wbe.list_tools()`                          | `wbe.list_tools_detailed()` (richer output)           |
| — (not available)                           | `wbe.describe_tool("slope")`                          |
| — (not available)                           | `wbe.search_tools("flow")`                            |
| — (not available)                           | `wbe.domain("precision_agriculture").list_tools()`    |

---

## Deprecation timeline

| Release | Status                                                         |
|---------|----------------------------------------------------------------|
| v2.0    | Flat API still works; `[DEPRECATED]` markers added to docstrings |
| v2.1    | Flat API emits `DeprecationWarning` at call site               |
| v3.0    | Flat API removed                                               |
