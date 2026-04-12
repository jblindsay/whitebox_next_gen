library(whiteboxworkflows)

# Minimal chunked LiDAR edit workflow.
# Replace the input path with your own LAS/LAZ file.
input_path <- "survey.las"
output_path <- "survey_chunked_reclassified.laz"

lidar <- wbw_read_lidar(input_path)
fields <- c("x", "y", "z", "classification")

chunks <- lidar$to_matrix_chunks(chunk_size = 200000, fields = fields)
for (i in seq_along(chunks)) {
  high <- chunks[[i]][, 3] > 250
  chunks[[i]][high, 4] <- 6
}

edited <- lidar$from_matrix_chunks(
  chunks,
  output_path = output_path,
  overwrite = TRUE,
  fields = fields
)

cat("Wrote:", edited$file_path(), "\n")
cat("Point count:", edited$point_count(), "\n")
