# Quick Start

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
dem = wbe.read_raster("dem.tif")
filled = wbe.hydrology.fill_depressions(dem)
wbe.write_raster(filled, "dem_filled.tif")
```

## What to Read Next

- Session setup and tool visibility: [Environment and Discovery](./environment-and-discovery.md)
- Execution callbacks and progress: [Tool Execution and Progress](./tool-execution-and-progress.md)
- End-to-end workflow scripts: [Script Index](./script-index.md)

## Quick-Start Conventions

- Prefer object-first workflows (`read_*` -> transform -> `write_*`).
- Validate tool visibility for scripted pipelines.
- Persist only outputs you need to keep.
