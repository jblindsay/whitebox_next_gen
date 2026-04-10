library(whiteboxworkflows)

# Replace with a real bundle root or supported archive path.
bundle_path <- "LC09_SCENE"

bundle <- wbw_read_bundle(bundle_path)
print(bundle)

meta <- bundle$metadata()
str(meta)

cat("Band keys:", paste(bundle$list_band_keys(), collapse = ", "), "\n")
cat("QA keys:", paste(bundle$list_qa_keys(), collapse = ", "), "\n")
cat("Aux keys:", paste(bundle$list_aux_keys(), collapse = ", "), "\n")

band_keys <- bundle$list_band_keys()
if (length(band_keys) > 0) {
	preview <- bundle$read_band(band_keys[[1]])
	print(preview)
}

preview_info <- bundle$read_preview_raster()
if (!is.null(preview_info)) {
	cat("Preview source:", preview_info$kind, preview_info$key, "\n")
	print(preview_info$raster)
}

# Optional composite previews (requires suitable RGB/NIR keys for the bundle family).
# tc <- bundle$write_true_colour("landsat_true_colour.tif")
# fc <- bundle$write_false_colour("landsat_false_colour.tif")

# Family-specific convenience readers are available when you know the type.
# landsat <- wbw_read_landsat(bundle_path)