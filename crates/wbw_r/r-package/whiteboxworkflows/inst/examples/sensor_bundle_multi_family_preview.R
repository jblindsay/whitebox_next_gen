library(whiteboxworkflows)

# Multi-family sensor bundle inspection and preview helper.
#
# Example usage:
# bundles <- list(
#   list(path = "LC09_SCENE", reader = wbw_read_landsat),
#   list(path = "S1_SCENE.SAFE", reader = wbw_read_sentinel1)
# )

summarize_bundle <- function(bundle, label) {
  cat("\n[", label, "]\n", sep = "")
  print(bundle)

  cat("Family:", bundle$family, "\n")
  cat("Band keys:", paste(bundle$list_band_keys(), collapse = ", "), "\n")
  cat("Measurement keys:", paste(bundle$list_measurement_keys(), collapse = ", "), "\n")
  cat("QA keys:", paste(bundle$list_qa_keys(), collapse = ", "), "\n")
  cat("Aux keys:", paste(bundle$list_aux_keys(), collapse = ", "), "\n")

  preview <- bundle$read_preview_raster()
  if (is.null(preview)) {
    cat("Preview: none available\n")
    return(invisible(NULL))
  }

  cat("Preview source:", preview$kind, preview$key, "\n")
  print(preview$raster)
}

write_bundle_previews <- function(bundle, output_dir, stem) {
  dir.create(output_dir, recursive = TRUE, showWarnings = FALSE)

  true_path <- file.path(output_dir, paste0(stem, "_true_colour.tif"))
  false_path <- file.path(output_dir, paste0(stem, "_false_colour.tif"))

  tryCatch({
    bundle$write_true_colour(true_path)
    cat("Wrote:", true_path, "\n")
  }, error = function(e) {
    cat("True-colour skipped:", conditionMessage(e), "\n")
  })

  tryCatch({
    bundle$write_false_colour(false_path)
    cat("Wrote:", false_path, "\n")
  }, error = function(e) {
    cat("False-colour skipped:", conditionMessage(e), "\n")
  })
}

# Uncomment and edit this block with real bundle paths.
# bundles <- list(
#   list(path = "LC09_SCENE", reader = wbw_read_landsat, label = "landsat"),
#   list(path = "S1_SCENE.SAFE", reader = wbw_read_sentinel1, label = "sentinel1")
# )
#
# for (job in bundles) {
#   bundle <- job$reader(job$path)
#   summarize_bundle(bundle, job$label)
#   write_bundle_previews(bundle, output_dir = "bundle_previews", stem = job$label)
# }
