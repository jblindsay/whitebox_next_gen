library(whiteboxworkflows)

# Replace with a real LAS/LAZ/COPC path.
lidar_path <- "points.las"

if (!file.exists(lidar_path)) {
  message("Replace 'points.las' with a real LAS/LAZ/COPC path to run this example.")
} else {
  lidar <- wbw_read_lidar(lidar_path)
  print(lidar)

  meta <- lidar$metadata()
  str(meta)

  cat("Short name:", lidar$get_short_filename(), "\n")

  # File-backed copy/write helpers
  lidar_copy <- lidar$deep_copy("points_copy.las", overwrite = TRUE)
  lidar$write("points_written.las", overwrite = TRUE)

  print(lidar_copy)
}