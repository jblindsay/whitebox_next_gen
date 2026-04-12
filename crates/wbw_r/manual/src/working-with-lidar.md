# Working with Lidar

This chapter documents lidar workflows in WbW-R, including file-backed processing,
metadata checks, and output control patterns.

Lidar workflows are strongly shaped by dataset size and I/O cost. The guiding
pattern here is to use file-backed objects and vectorized matrix operations for
normal workloads, then switch to chunked processing when point counts exceed
comfortable memory limits.

## Baseline Workflow

Use this as a first-run validation before introducing matrix edits or chunked
processing.

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

The example below uses a simple reclassification rule to illustrate the matrix
roundtrip pattern.

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

Use tool-driven processing when extracting QA outputs and diagnostics from lidar
datasets.

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

Use this when full-matrix operations would exceed available memory.

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

## Lidar Object Method Reference

### Metadata and Access

| Method | Description |
|---|---|
| `metadata` | Return lidar metadata (bounds, point format, CRS, counts). |
| `point_count` | Return total number of points. |
| `file_path`, `path` | Return backing lidar path. |
| `get_short_filename` | Return basename of lidar file. |

### Matrix and Data-Frame Conversion

| Method | Description |
|---|---|
| `to_matrix` | Read selected lidar fields as numeric matrix. |
| `to_data_frame` | Read selected lidar fields as data frame. |
| `to_matrix_chunks` | Read selected lidar fields in chunked matrix blocks. |

### Writing Edited Point Data

| Method | Description |
|---|---|
| `from_matrix` | Write edited matrix data to lidar output. |
| `from_data_frame` | Write edited data frame fields to lidar output. |
| `from_matrix_chunks` | Write chunked matrix edits to lidar output. |

### Persistence

| Method | Description |
|---|---|
| `deep_copy` | Copy lidar to a new path with optional write options. |
| `write` | Write lidar to a new output path. |
