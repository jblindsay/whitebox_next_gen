# Quick Start

This chapter is a minimal "first success" path. It verifies that your R
installation, package setup, session creation, tool execution, and output write
path all work together before you take on larger workflows.

Once this slice runs end-to-end, the remaining chapters explain how to make the
same pattern more explicit, reproducible, and resilient.

The example below demonstrates the core session-first model in WbW-R: create a
session, load data, run a tool, and inspect the result object.

```r
library(whiteboxworkflows)

s <- wbw_session()
dem <- wbw_read_raster("dem.tif")

result <- wbw_run_tool(
  "slope",
  args = list(dem = dem$file_path(), output = "slope.tif"),
  session = s
)
print(result)
```

## What to Read Next

These chapters extend the same lifecycle with discovery, progress handling, and
larger workflow templates.

- Session lifecycle and discovery: [Session and Discovery](./session-and-discovery.md)
- Progress callbacks and execution patterns: [Tool Execution and Progress](./tool-execution-and-progress.md)
- End-to-end workflow scripts: [Script Index](./script-index.md)

## Quick-Start Conventions

- Prefer explicit `wbw_session(...)` construction in scripts.
- Validate tool visibility before long-running pipelines.
- Persist outputs deliberately and re-open typed objects for verification.
