# Raster array roundtrip example for whiteboxworkflows
#
# This example shows the R analogue of Python's NumPy interoperability:
# 1. read a raster into an R matrix/array
# 2. perform ordinary R math
# 3. write the result back to a raster using a template for georeferencing
#
# Optional packages:
# - terra: for matrix/array bridge helpers
# - stars: for stars object bridge helpers

library(whiteboxworkflows)

# Example using terra-backed matrix/array bridge
# arr <- wbw_raster_to_array("dem.tif")
# arr2 <- arr + 1
# wbw_array_to_raster(arr2, "dem_plus1.tif", template_path = "dem.tif", overwrite = TRUE)

# Example using stars bridge
# s <- wbw_raster_to_stars("dem.tif")
# s2 <- s + 1
# wbw_stars_to_raster(s2, "dem_plus1_stars.tif", overwrite = TRUE)

# Example progress-aware execution
# session <- wbw_session()
# result <- wbw_run_tool_with_progress(
#   "slope",
#   args = list(dem = "dem.tif", output = "slope.tif"),
#   session = session
# )
# str(result)
