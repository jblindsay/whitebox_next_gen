# Download OSM vector features with wbw_r.
# Example: fetch road centerlines in a small bbox and write to GeoJSON.

library(whiteboxworkflows)

session <- wbw_session()

result <- wbw_run_tool(
  "download_osm_vector",
  args = list(
    west = -80.54,
    south = 43.41,
    east = -80.47,
    north = 43.47,
    filter_preset = "roads",
    include_points = FALSE,
    include_lines = TRUE,
    include_polygons = FALSE,
    timeout_seconds = 30,
    max_elements = 50000,
    output = "kitchener_roads.geojson"
  ),
  session = session
)

cat(sprintf("wrote output: %s\n", result$outputs$path))
