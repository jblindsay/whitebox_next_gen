library(whiteboxworkflows)

# Golden-path workflow script for new users.
#
# This script is intentionally linear and session-centric:
# 1) Start a session and discover tools.
# 2) Run one representative tool.
# 3) Work with typed objects.
# 4) Optional interop and sensor-bundle helpers.

s <- wbw_session()

cat("Visible tools:", length(wbw_tool_ids(session = s)), "\n")
if (!wbw_has_tool("slope", session = s)) {
  stop("Expected tool 'slope' to be visible in the active session.")
}

# 1) Tool discovery (category + keyword)
print(wbw_category_summary(session = s))
print(head(wbw_tools_in_category("Raster", session = s), 5))
print(head(wbw_search_tools("slope", session = s), 5))

# 2) Run a representative tool (edit paths for your workspace)
# wbw_run_tool(
#   "slope",
#   args = list(dem = "dem.tif", output = "slope.tif"),
#   session = s
# )

# 3) Typed raster object workflow
# dem <- wbw_read_raster("dem.tif", session = s)
# print(dem$metadata())
# arr <- dem$to_array()
# wbw_array_to_raster(arr, "dem_copy.tif", template_path = "dem.tif", overwrite = TRUE)

# 4) Typed vector object workflow
# roads <- wbw_read_vector("roads.gpkg", session = s)
# print(roads$schema())
# print(roads$attributes(1))

# 5) Optional sensor-bundle workflow
# bundle <- wbw_read_bundle("LC09_SCENE", session = s)
# print(bundle$key_summary())
# if (bundle$has_key("B04")) {
#   b4 <- bundle$read_any("B04")
#   print(b4$metadata())
# }
# preview <- bundle$read_preview_raster()
# if (!is.null(preview)) {
#   print(preview$key)
# }
