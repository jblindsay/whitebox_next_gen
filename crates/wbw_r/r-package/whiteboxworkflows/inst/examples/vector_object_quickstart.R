library(whiteboxworkflows)

# Replace with a real vector dataset path.
vector_path <- "roads.gpkg"

if (!file.exists(vector_path)) {
  message("Replace 'roads.gpkg' with a real vector dataset path to run this example.")
} else {
  vec <- wbw_read_vector(vector_path)
  print(vec)

  meta <- vec$metadata()
  str(meta)

  # Native terra view
  tv <- vec$to_terra()
  print(tv)

  # Optional sf conversion
  # install.packages("sf")
  # sf_obj <- vec$to_sf()
  # print(sf_obj)
}