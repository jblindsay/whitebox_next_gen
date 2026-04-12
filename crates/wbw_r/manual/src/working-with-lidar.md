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

Stable WbW-R lidar objects are file-backed and tool-oriented, with explicit
columnar point access through matrix/data-frame APIs. Direct point-by-point
iteration is still not the primary API path.

Recommended point-level workflow:
1. Use `to_matrix()` or `to_data_frame()` with selected point fields.
2. Apply vectorized edits in base R/tidy workflows.
3. Write updates with `from_matrix(...)` or `from_data_frame(...)`.

```r
library(whiteboxworkflows)

l <- wbw_read_lidar('survey.las')
pts <- l$to_matrix(fields = c('x', 'y', 'z', 'classification'))

ground <- pts[, 4] == 2
pts[ground, 4] <- 6

edited <- l$from_matrix(
  pts,
  output_path = 'survey_reclassified.laz',
  overwrite = TRUE,
  fields = c('x', 'y', 'z', 'classification')
)

print(edited$point_count())
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

## Chunked Matrix Streaming

For very large point clouds, use chunked matrix streaming to avoid keeping the
full point matrix in memory.

Recommended chunked workflow:
1. Read chunks with `to_matrix_chunks(...)`.
2. Apply vectorized edits in each chunk.
3. Write edited chunks with `from_matrix_chunks(...)`.

```r
library(whiteboxworkflows)

l <- wbw_read_lidar('survey.las')
fields <- c('x', 'y', 'z', 'classification')

chunks <- l$to_matrix_chunks(chunk_size = 200000, fields = fields)
for (i in seq_along(chunks)) {
  high <- chunks[[i]][, 3] > 250
  chunks[[i]][high, 4] <- 6
}

edited <- l$from_matrix_chunks(
  chunks,
  output_path = 'survey_chunked_reclassified.laz',
  overwrite = TRUE,
  fields = fields
)

print(edited$point_count())
```

Notes:
- LAS/LAZ chunk outputs use shared core streaming rewrite.
- Other output formats currently fall back to non-streaming assembly in this API path.

## Best Practices

- Validate CRS before and after reprojection.
- Keep source lidar immutable; write derived products to new files.
- Prefer COPC/LAZ outputs for large cloud workflows.
