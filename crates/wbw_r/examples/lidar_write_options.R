# Example: Lidar write options and format conversion in R
#
# This example demonstrates the lidar_write_with_options_json() and
# lidar_copy_to_path() functions for writing point clouds with format options.
#
# For Phase 1, these functions validate JSON options but perform basic writes.
# Future phases will apply writer configuration once the wblidar frontend API
# is extended to support configurable writes.

library(wbw_r)

cat("=" %+% strrep("=", 49) %+% "\n")
cat("Whitebox Workflows R: Lidar Write Options Examples\n")
cat("=" %+% strrep("=", 49) %+% "\n\n")

# Example 1: Basic Lidar Copy
# -----------------------------------------------
cat("Example 1: Basic Lidar Copy\n")
cat(strrep("-", 50) %+% "\n")

input_file <- "input.las"
output_file <- "output"  # No extension → saved as COPC

result <- lidar_copy_to_path(input_file, output_file)
cat("Copied:", input_file, "\n")
cat("Output:", result, "\n\n")

# Example 2: Write with LAZ Options
# -----------------------------------------------
cat("Example 2: Write with LAZ Options\n")
cat(strrep("-", 50) %+% "\n")

options_laz <- list(
  laz = list(
    chunk_size = 25000L,       # Points per compressed chunk
    compression_level = 7L     # 0-9, higher = better compression
  )
)

options_json <- jsonlite::toJSON(options_laz, auto_unbox = TRUE)
result <- lidar_write_with_options_json(input_file, "output.laz", options_json)
cat("Wrote to LAZ with custom options:\n")
cat("  chunk_size:", options_laz$laz$chunk_size, "\n")
cat("  compression_level:", options_laz$laz$compression_level, "\n")
cat("Output:", result, "\n\n")

# Example 3: Write with COPC Options
# -----------------------------------------------
cat("Example 3: Write with COPC Options\n")
cat(strrep("-", 50) %+% "\n")

options_copc <- list(
  copc = list(
    max_points_per_node = 50000L,  # Balance lookup speed vs I/O
    max_depth = 10L,                # Maximum octree depth
    node_point_ordering = "hilbert" # auto, morton, or hilbert
  )
)

options_json <- jsonlite::toJSON(options_copc, auto_unbox = TRUE)
result <- lidar_write_with_options_json(
  input_file, 
  "output.copc.laz", 
  options_json
)
cat("Wrote to COPC with spatial options:\n")
cat("  max_points_per_node:", options_copc$copc$max_points_per_node, "\n")
cat("  max_depth:", options_copc$copc$max_depth, "\n")
cat("  node_point_ordering:", options_copc$copc$node_point_ordering, "\n")
cat("Output:", result, "\n\n")

# Example 4: Combined Options
# -----------------------------------------------
cat("Example 4: Combined Options\n")
cat(strrep("-", 50) %+% "\n")

options_combined <- list(
  laz = list(
    chunk_size = 40000L,
    compression_level = 6L
  ),
  copc = list(
    max_points_per_node = 75000L,
    max_depth = 8L,
    node_point_ordering = "auto"
  )
)

options_json <- jsonlite::toJSON(options_combined, auto_unbox = TRUE)
result <- lidar_write_with_options_json(
  input_file,
  "output.copc.laz",
  options_json
)
cat("Wrote with combined options (format auto-detected from extension)\n")
cat("Output:", result, "\n\n")

# Example 5: Path Extension Inference
# -----------------------------------------------
cat("Example 5: Path Extension Inference\n")
cat(strrep("-", 50) %+% "\n")

# No extension → saved as COPC
output1 <- lidar_copy_to_path(input_file, "directory/output")
cat("No extension →", basename(output1), "\n")

# .las extension → saved as LAS
output2 <- lidar_copy_to_path(input_file, "directory/output.las")
cat(".las extension →", basename(output2), "\n")

# .laz extension → saved as LAZ
output3 <- lidar_copy_to_path(input_file, "directory/output.laz")
cat(".laz extension →", basename(output3), "\n")

# .copc.laz extension → saved as COPC
output4 <- lidar_copy_to_path(input_file, "directory/output.copc.laz")
cat(".copc.laz extension →", basename(output4), "\n\n")

cat("Examples completed successfully!\n")
