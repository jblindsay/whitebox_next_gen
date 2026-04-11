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

Planned expansion:
- Session lifecycle patterns.
- Discovery checks before execution.
- Memory-first versus explicit output workflows.
