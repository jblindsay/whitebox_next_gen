# Lidar Write Options: Phase 1 Implementation

## Overview

This document describes the Phase 1 implementation of lidar write options across the Whitebox Workflows backend (Rust), and Python/R frontends.

**Status**: ✅ **Complete** — Rust backend and Python/R bindings implemented and compiled successfully.

### What's Included

- **Rust Backend** (`wbw_r`):
  - `lidar_copy_to_path(src, dst)` — Copy/transcode with automatic COPC default
  - `lidar_write_with_options_json(src, dst, options_json)` — Write with validated options
  - `LidarWriteControls` — Options parser for LAZ and COPC
  - `parse_lidar_write_controls()` — Validates JSON options
  - `parse_node_point_ordering()` — Validates COPC spatial ordering

- **Python Frontend** (`wbw_python`):
  - `Lidar.copy_to_path(dst)` — Convenience wrapper
  - `Lidar.write_to_path(dst, options_json)` — Write with options

- **R Frontend** (`wbw_r`):
  - `lidar_copy_to_path(src, dst)` — Copy with format inference
  - `lidar_write_with_options_json(src, dst, options_json)` — Write with options

---

## Options Format

### JSON Schema

```json
{
  "laz": {
    "chunk_size": <positive_integer>,
    "compression_level": <0-9>
  },
  "copc": {
    "max_points_per_node": <positive_integer>,
    "max_depth": <positive_integer>,
    "node_point_ordering": "auto|morton|hilbert"
  }
}
```

### LAZ Options

| Option | Type | Default | Range | Description |
|--------|------|---------|-------|-------------|
| `chunk_size` | integer | 50,000 | ≥1 | Points per compressed chunk |
| `compression_level` | integer | 6 | 0–9 | Compression tuning: 0=fastest, 9=smallest |

### COPC Options

| Option | Type | Default | Values | Description |
|--------|------|---------|--------|-------------|
| `max_points_per_node` | integer | 100,000 | ≥1 | Max points before node subdivision |
| `max_depth` | integer | 8 | ≥1 | Maximum octree depth |
| `node_point_ordering` | string | "auto" | auto\|morton\|hilbert | Point ordering within nodes before compression |

#### Node Point Ordering Strategies

- **`auto`**: GPS time when present, otherwise Morton (Z-order)
- **`morton`**: Z-order spatial curve (fastest cache behavior)
- **`hilbert`**: Hilbert curve (improved spatial locality)

---

## API Reference

### Python

```python
from whitebox_workflows import WbEnvironment
import json

env = WbEnvironment()
lidar = env.lidar("input.las")

# Basic copy (no extension → COPC)
result = lidar.copy_to_path("output")

# Write with LAZ options
options = {
    "laz": {
        "chunk_size": 25000,
        "compression_level": 7
    }
}
result = lidar.write_to_path("output.laz", options_json=json.dumps(options))

# Write with COPC options
options = {
    "copc": {
        "max_points_per_node": 50000,
        "max_depth": 10,
        "node_point_ordering": "hilbert"
    }
}
result = lidar.write_to_path("output.copc.laz", options_json=json.dumps(options))
```

### R

```r
library(wbw_r)
library(jsonlite)

# Basic copy (no extension → COPC)
result <- lidar_copy_to_path("input.las", "output")

# Write with LAZ options
options <- list(
  laz = list(
    chunk_size = 25000L,
    compression_level = 7L
  )
)
result <- lidar_write_with_options_json(
  "input.las",
  "output.laz",
  toJSON(options, auto_unbox = TRUE)
)

# Write with COPC options
options <- list(
  copc = list(
    max_points_per_node = 50000L,
    max_depth = 10L,
    node_point_ordering = "hilbert"
  )
)
result <- lidar_write_with_options_json(
  "input.las",
  "output.copc.laz",
  toJSON(options, auto_unbox = TRUE)
)
```

---

## Implementation Details

### Phase 1 Scope

In Phase 1, the implementation:

✅ **Validates** JSON options at the Rust layer  
✅ **Parses and type-checks** all LAZ and COPC parameters  
✅ **Infers output format** from file extension  
✅ **Handles missing extensions** (defaults to COPC)  
✅ **Creates parent directories** automatically  
✅ **Applies LAZ options** (`chunk_size`, `compression_level`) through `wblidar::write_with_options(...)`
✅ **Applies COPC options** (`max_points_per_node`, `max_depth`, `node_point_ordering`) through `wblidar::write_with_options(...)`

### Build Status

- ✅ `cargo build -p wbw_r` — Success (no warnings after dead code annotation)
- ✅ `cargo build -p wbw_python` — Success
- ✅ R extendr bindings — Registered and compiled
- ✅ Python PyO3 bindings — Registered and compiled

---

## Future Enhancements (Phase 2+)

`wblidar` now exposes configurable write helpers (`write_with_options` and
`write_auto_with_options`), and the current Python/R entry points already flow
through these APIs.

### Additional Options

Future phases may add:
- ✅ Raster write options (already implemented for GeoTIFF)
- ✅ Vector write options
- LAS specification version selection
- Point format selection
- Custom VLR (Variable Length Record) handling
- Multi-threaded COPC generation

---

## Testing

### Test Files

- Python examples: `crates/wbw_python/examples/lidar_write_options.py`
- R examples: `crates/wbw_r/examples/lidar_write_options.R`

### Compilation Verification

```bash
# Backend
cd whitebox_next_gen
cargo build -p wbw_r

# Python
cargo build -p wbw_python

# R (requires extendr tooling)
cd crates/wbw_r
R CMD build --no-build-vignettes .
```

---

## Error Handling

### Invalid Options

```python
# Invalid node_point_ordering value
options = {"copc": {"node_point_ordering": "spline"}}
# Error: unsupported node_point_ordering 'spline'. Expected one of: auto, morton, hilbert
```

### Malformed JSON

```python
options_json = "{invalid json"
# Error: invalid options JSON: ...
```

### File I/O Errors

File creation, permission, and I/O errors are propagated with descriptive messages.

---

## Backward Compatibility

All APIs are new (Phase 1), so backward compatibility is not a concern. The existing `Lidar` class and lidar tools remain unchanged.

---

## Related Documentation

- [Raster Write Options](./raster_write_options.md) — Similar Phase 1 implementation for GeoTIFF
- [wblidar API](../crates/wblidar/README.md) — Underlying point cloud I/O
- [Whitebox Workflows Python Frontend](../../wbw_python/README.md)
- [Whitebox Workflows R Frontend](../../wbw_r/README.md)

---

## Maintenance Notes

### Registered Public APIs

- **Rust** (`wbw_r`): `lidar_copy_to_path`, `lidar_write_with_options_json`
- **Python** (`wbw_python`): `Lidar.copy_to_path`, `Lidar.write_to_path`
- **R** (`wbw_r`): `lidar_copy_to_path`, `lidar_write_with_options_json`

### Code Entry Points

- Rust options parser: `wbw_r/src/lib.rs` (lines 200–240)
- Python bindings: `wbw_python/src/wb_environment.rs` (lines 5315–5360)
- R extendr wrappers: `wbw_r/src/lib.rs` (lines 1935–1945)

