# Working with Lidar

This chapter documents lidar workflows in WbW-R, including file-backed processing,
metadata checks, and output control patterns.

## Baseline Workflow

```r
library(whiteboxworkflows)

l <- wbw_read_lidar('survey.las')
print(l$metadata())

copy <- l$deep_copy('survey_copy.las', overwrite = TRUE)
print(copy$metadata())
```

## Iterating Through Lidar Points

Current stable WbW-R lidar objects are file-backed and tool-oriented. Direct
point-by-point iteration is not the primary API path.

Recommended point-level workflow:
1. Use WbW-R for lidar processing and reprojection.
2. For explicit point iteration, use an R point-cloud package on an output file.
3. Return to WbW-R for downstream geoprocessing.

```r
library(whiteboxworkflows)
# Optional bridge package for point-level inspection.
# library(lidR)

l <- wbw_read_lidar('survey.las')
out <- l$deep_copy('survey_filtered.laz', overwrite = TRUE)

# las <- lidR::readLAS(out$file_path())
# pts <- las@data
# for (i in seq_len(min(1000, nrow(pts)))) {
#   x <- pts$X[i]; y <- pts$Y[i]; z <- pts$Z[i]
# }
```

## Tool-Driven Lidar Processing

```r
library(whiteboxworkflows)

s <- wbw_session()
wbw_run_tool(
  'lidar_info',
  args = list(input = 'survey.las', output = 'survey_info.html'),
  session = s
)
```

## Best Practices

- Validate CRS before and after reprojection.
- Keep source lidar immutable; write derived products to new files.
- Prefer COPC/LAZ outputs for large cloud workflows.
