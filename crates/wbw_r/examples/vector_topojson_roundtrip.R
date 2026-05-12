# Simple TopoJSON roundtrip example for WbW-R.

library(whiteboxworkflows)

src <- "input_roads.gpkg"
topo <- "output_roads.topojson"
back <- "output_roads_back.gpkg"

roads <- wbw_read_vector(src)
wbw_write_vector(roads, topo)

roads_topo <- wbw_read_vector(topo)
wbw_write_vector(roads_topo, back)

cat(sprintf("wrote TopoJSON: %s\n", topo))
cat(sprintf("wrote roundtrip GeoPackage: %s\n", back))
