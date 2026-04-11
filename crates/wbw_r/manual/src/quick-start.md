# Quick Start

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

- Session lifecycle and discovery: [Session and Discovery](./session-and-discovery.md)
- Progress callbacks and execution patterns: [Tool Execution and Progress](./tool-execution-and-progress.md)
- End-to-end workflow scripts: [Script Index](./script-index.md)

## Quick-Start Conventions

- Prefer explicit `wbw_session(...)` construction in scripts.
- Validate tool visibility before long-running pipelines.
- Persist outputs deliberately and re-open typed objects for verification.
