# Quick Start

This chapter gives you the shortest path to a successful first run. Think of it
as a minimal "vertical slice": create an environment, load a dataset, run a
tool, and write output. The goal is to verify that your runtime, paths, and
basic mental model are all working before you invest in larger workflows.

After this first pass, the rest of the manual progressively explains why each
step matters and how to make the same pattern robust for production scripts.

The example below demonstrates the default object-first workflow pattern. It
reads a DEM, runs a hydrological tool, and writes an output raster. If this
runs successfully, your environment and core processing path are functioning.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
dem = wbe.read_raster("dem.tif")
filled = wbe.hydrology.fill_depressions(dem)
wbe.write_raster(filled, "dem_filled.tif")
```

## What to Read Next

These chapters deepen the same lifecycle shown above: discovery before
execution, explicit progress handling, and script catalogs for adaptation.

- Session setup and tool visibility: [Environment and Discovery](./environment-and-discovery.md)
- Execution callbacks and progress: [Tool Execution and Progress](./tool-execution-and-progress.md)
- End-to-end workflow scripts: [Script Index](./script-index.md)

## Quick-Start Conventions

- Prefer object-first workflows (`read_*` -> transform -> `write_*`).
- Validate tool visibility for scripted pipelines.
- Persist only outputs you need to keep.
