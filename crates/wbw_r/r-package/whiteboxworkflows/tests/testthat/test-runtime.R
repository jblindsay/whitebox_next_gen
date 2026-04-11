test_that("low-level JSON listing returns visible tools", {
  tools_json <- list_tools_json_with_options(FALSE, "open")
  tools <- jsonlite::fromJSON(tools_json, simplifyVector = FALSE)

  expect_type(tools, "list")
  expect_gt(length(tools), 0L)

  first_tool <- tools[[1]]
  expect_true(is.list(first_tool))
  expect_true("id" %in% names(first_tool))
  expect_true(is.character(first_tool$id))
  expect_true(nzchar(first_tool$id))
})

test_that("session facade lists tools", {
  session <- whitebox_tools()
  tools <- session$list_tools()
  facade_tools <- wbw_list_tools()

  expect_true(is.environment(session))
  expect_true(is.function(session$list_tools))
  expect_true(is.function(session$run_tool))
  expect_type(tools, "list")
  expect_gt(length(tools), 0L)
  expect_equal(length(facade_tools), length(tools))
})

test_that("wbw_session exposes idiomatic helpers", {
  session <- wbw_session()
  expect_true(inherits(session, "wbw_session"))

  ids <- wbw_tool_ids(session = session)
  expect_type(ids, "character")
  expect_gt(length(ids), 0L)
  expect_true(any(ids == "add"))

  expect_true(wbw_has_tool("add", session = session))
  expect_false(wbw_has_tool("definitely_not_a_real_tool_id", session = session))
  expect_true(is.function(session$run_tool_with_progress))
})

test_that("progress-aware helper returns structured progress payload", {
  session <- new.env(parent = emptyenv())
  session$run_tool_with_progress <- function(tool_id, args = list()) {
    list(
      tool_id = tool_id,
      outputs = list(args = args),
      progress = list(list(message = "ok", pct = 100))
    )
  }

  result <- wbw_run_tool_with_progress("mock_tool", args = list(alpha = 1), session = session)

  expect_true(is.list(result))
  expect_true("tool_id" %in% names(result))
  expect_true("progress" %in% names(result))
  expect_equal(result$tool_id, "mock_tool")
  expect_type(result$progress, "list")
  expect_equal(result$outputs$args$alpha, 1)
})

test_that("wbw_print_progress and wbw_make_progress_printer are available", {
  expect_true(is.function(wbw_print_progress))
  expect_true(is.function(wbw_make_progress_printer))
})

test_that("progress printer normalizes and throttles updates", {
  out <- textConnection("captured", "w", local = TRUE)
  on.exit(close(out), add = TRUE)

  cb <- wbw_make_progress_printer(min_increment = 10, show_messages = TRUE, stream = out)
  cb(0.12, "")
  cb(0.19, "")
  cb(0.26, "")
  cb(0.26, "")
  cb(NA_real_, "hello")
  cb(NA_real_, "Progress (loop 1 of 2): 50%")
  cb(1.0, "")

  expect_equal(captured, c("12%", "26%", "hello", "50%", "Progress (loop 1 of 2): 50%", "100%"))
})

test_that("multi-output path results are coerced into typed raster objects", {
  skip_if_not_installed("terra")

  path1 <- tempfile(fileext = ".tif")
  path2 <- tempfile(fileext = ".tif")
  r <- terra::rast(nrows = 2, ncols = 2, xmin = 0, xmax = 2, ymin = 0, ymax = 2)
  terra::values(r) <- c(1, 2, 3, 4)
  terra::crs(r) <- "EPSG:4326"
  terra::writeRaster(r, path1, overwrite = TRUE)
  terra::writeRaster(r, path2, overwrite = TRUE)

  outputs <- list(
    intensity = path1,
    hue = path2,
    cells_processed = 4L
  )

  coerced <- whiteboxworkflows:::wbw_coerce_tool_output(outputs)

  expect_type(coerced, "list")
  expect_length(coerced, 2L)
  expect_true(inherits(coerced[[1]], "wbw_raster"))
  expect_true(inherits(coerced[[2]], "wbw_raster"))
  expect_true(file.exists(coerced[[1]]$file_path()))
  expect_true(file.exists(coerced[[2]]$file_path()))
})

test_that("single-output convenience path key is not misread as multi-output", {
  skip_if_not_installed("terra")

  path1 <- tempfile(fileext = ".tif")
  r <- terra::rast(nrows = 2, ncols = 2, xmin = 0, xmax = 2, ymin = 0, ymax = 2)
  terra::values(r) <- c(1, 2, 3, 4)
  terra::crs(r) <- "EPSG:4326"
  terra::writeRaster(r, path1, overwrite = TRUE)

  outputs <- list(
    output = path1,
    path = path1,
    active_band = 0L
  )

  coerced <- whiteboxworkflows:::wbw_coerce_tool_output(outputs)

  expect_equal(coerced, outputs)
})

test_that("memory-backed output paths are coerced into typed raster objects", {
  outputs <- list(
    output = list(
      `__wbw_type__` = "raster",
      path = "memory://raster/42",
      active_band = 0L
    ),
    output2 = list(
      `__wbw_type__` = "raster",
      path = "memory://raster/43",
      active_band = 0L
    ),
    cells_processed = 100L
  )

  coerced <- whiteboxworkflows:::wbw_coerce_tool_output(outputs)

  expect_type(coerced, "list")
  expect_length(coerced, 2L)
  expect_true(inherits(coerced[[1]], "wbw_raster"))
  expect_true(inherits(coerced[[2]], "wbw_raster"))
  expect_equal(coerced[[1]]$file_path(), "memory://raster/42")
  expect_equal(coerced[[2]]$file_path(), "memory://raster/43")
})

test_that("entitlement startup argument validation guards are enforced", {
  expect_error(
    wbw_session(
      signed_entitlement_json = "{}",
      entitlement_file = tempfile(fileext = ".json")
    )
  )

  expect_error(
    wbw_session(signed_entitlement_json = "{}")
  )

  expect_error(
    wbw_session(entitlement_file = "missing.json")
  )
})

test_that("invalid entitlement startup inputs fail on first runtime call", {
  session_inline <- wbw_session(
    signed_entitlement_json = "{}",
    public_key_kid = "k1",
    public_key_b64url = "invalid-key"
  )
  expect_error(session_inline$list_tools())

  missing_file <- file.path(tempdir(), "missing_entitlement.json")
  session_file <- wbw_session(
    entitlement_file = missing_file,
    public_key_kid = "k1",
    public_key_b64url = "invalid-key"
  )
  expect_error(session_file$list_tools())
})

test_that("floating startup failure paths are surfaced", {
  missing_provider <- wbw_session(
    floating_license_id = "fl_test",
    include_pro = TRUE
  )
  expect_error(missing_provider$list_tools())

  unreachable_provider <- wbw_session(
    floating_license_id = "fl_test",
    include_pro = TRUE,
    provider_url = "http://127.0.0.1:9"
  )
  expect_error(unreachable_provider$list_tools())
})

test_that("fail-open policy with unreachable provider falls back to open tier", {
  can_use_pro <- tryCatch({
    list_tools_json_with_options(TRUE, "open")
    TRUE
  }, error = function(e) FALSE)
  skip_if(!can_use_pro, "pro build required for fail-open policy test")

  old_policy   <- Sys.getenv("WBW_LICENSE_POLICY",    unset = NA_character_)
  old_provider <- Sys.getenv("WBW_LICENSE_PROVIDER_URL", unset = NA_character_)
  old_state    <- Sys.getenv("WBW_LICENSE_STATE_PATH",   unset = NA_character_)
  on.exit({
    if (is.na(old_policy))   Sys.unsetenv("WBW_LICENSE_POLICY")    else Sys.setenv(WBW_LICENSE_POLICY    = old_policy)
    if (is.na(old_provider)) Sys.unsetenv("WBW_LICENSE_PROVIDER_URL") else Sys.setenv(WBW_LICENSE_PROVIDER_URL = old_provider)
    if (is.na(old_state))    Sys.unsetenv("WBW_LICENSE_STATE_PATH")  else Sys.setenv(WBW_LICENSE_STATE_PATH  = old_state)
  }, add = TRUE)

  Sys.setenv(
    WBW_LICENSE_POLICY       = "fail_open",
    WBW_LICENSE_PROVIDER_URL = "http://127.0.0.1:9",
    WBW_LICENSE_STATE_PATH   = file.path(tempdir(), "missing_state_fail_open.json")
  )

  session <- wbw_session(include_pro = TRUE)
  tools <- session$list_tools()
  expect_type(tools, "list")
  expect_gt(length(tools), 0L)

  ids <- vapply(tools, function(t) t$id %||% "", character(1))
  expect_false(any(ids == "raster_power"), info = "pro tools should be hidden under open fallback")
})

test_that("fail-closed policy with unreachable provider errors on startup", {
  can_use_pro <- tryCatch({
    list_tools_json_with_options(TRUE, "open")
    TRUE
  }, error = function(e) FALSE)
  skip_if(!can_use_pro, "pro build required for fail-closed policy test")

  old_policy   <- Sys.getenv("WBW_LICENSE_POLICY",    unset = NA_character_)
  old_provider <- Sys.getenv("WBW_LICENSE_PROVIDER_URL", unset = NA_character_)
  old_state    <- Sys.getenv("WBW_LICENSE_STATE_PATH",   unset = NA_character_)
  on.exit({
    if (is.na(old_policy))   Sys.unsetenv("WBW_LICENSE_POLICY")    else Sys.setenv(WBW_LICENSE_POLICY    = old_policy)
    if (is.na(old_provider)) Sys.unsetenv("WBW_LICENSE_PROVIDER_URL") else Sys.setenv(WBW_LICENSE_PROVIDER_URL = old_provider)
    if (is.na(old_state))    Sys.unsetenv("WBW_LICENSE_STATE_PATH")  else Sys.setenv(WBW_LICENSE_STATE_PATH  = old_state)
  }, add = TRUE)

  Sys.setenv(
    WBW_LICENSE_POLICY       = "fail_closed",
    WBW_LICENSE_PROVIDER_URL = "http://127.0.0.1:9",
    WBW_LICENSE_STATE_PATH   = file.path(tempdir(), "missing_state_fail_closed.json")
  )

  session <- wbw_session(include_pro = TRUE)
  expect_error(session$list_tools())
})

test_that("wbw_read_raster returns typed raster wrapper", {
  skip_if_not_installed("terra")

  path <- tempfile(fileext = ".tif")
  r <- terra::rast(nrows = 2, ncols = 3, xmin = 0, xmax = 3, ymin = 0, ymax = 2)
  terra::values(r) <- 1:6
  terra::crs(r) <- "EPSG:4326"
  terra::writeRaster(r, path, overwrite = TRUE)

  x <- wbw_read_raster(path)
  meta <- x$metadata()

  expect_true(inherits(x, "wbw_raster"))
  expect_equal(meta$rows, 2)
  expect_equal(meta$columns, 3)
  expect_equal(meta$bands, 1)
  expect_true(is.character(meta$crs))

  arr <- x$to_array()
  expect_true(is.matrix(arr))
  expect_equal(dim(arr), c(2, 3))
})

test_that("wbw_read_vector returns typed vector wrapper", {
  skip_if_not_installed("terra")

  path <- tempfile(fileext = ".gpkg")
  coords <- rbind(
    c(0, 0),
    c(1, 0),
    c(1, 1),
    c(0, 1),
    c(0, 0)
  )
  v <- terra::vect(coords, type = "polygons")
  terra::values(v) <- data.frame(id = 1L, name = "a")
  terra::crs(v) <- "EPSG:4326"
  terra::writeVector(v, path, overwrite = TRUE)

  x <- wbw_read_vector(path)
  meta <- x$metadata()

  expect_true(inherits(x, "wbw_vector"))
  expect_equal(meta$geometry_type, "polygons")
  expect_equal(meta$feature_count, 1)
  expect_true(all(c("id", "name") %in% meta$fields))

  tv <- x$to_terra()
  expect_true(inherits(tv, "SpatVector"))
  expect_equal(nrow(tv), 1)
})

test_that("wbw_raster convenience accessors and write methods work", {
  skip_if_not_installed("terra")

  path <- tempfile(fileext = ".tif")
  r <- terra::rast(nrows = 4, ncols = 5, xmin = 0, xmax = 5, ymin = 0, ymax = 4)
  terra::values(r) <- seq_len(20)
  terra::crs(r) <- "EPSG:32617"
  terra::writeRaster(r, path, overwrite = TRUE)

  x <- wbw_read_raster(path)

  expect_equal(x$file_path(), normalizePath(path, winslash = "/", mustWork = TRUE))
  expect_equal(x$band_count(), 1L)
  expect_equal(x$active_band(), 1L)

  epsg <- x$crs_epsg()
  expect_true(is.integer(epsg) || is.numeric(epsg))
  expect_equal(as.integer(epsg), 32617L)

  wkt <- x$crs_wkt()
  expect_true(is.character(wkt))
  expect_true(nzchar(wkt))

  copy_path <- tempfile(fileext = ".tif")
  copied <- x$deep_copy(copy_path)
  expect_true(file.exists(copy_path))
  expect_true(inherits(copied, "wbw_raster"))
  expect_equal(copied$band_count(), 1L)

  write_path <- tempfile(fileext = ".tif")
  written <- x$write(write_path)
  expect_true(file.exists(write_path))
  expect_true(inherits(written, "wbw_raster"))
})

test_that("wbw_raster arithmetic convenience methods work", {
  skip_if_not_installed("terra")

  path1 <- tempfile(fileext = ".tif")
  path2 <- tempfile(fileext = ".tif")

  r1 <- terra::rast(nrows = 2, ncols = 2, xmin = 0, xmax = 2, ymin = 0, ymax = 2)
  r2 <- terra::rast(nrows = 2, ncols = 2, xmin = 0, xmax = 2, ymin = 0, ymax = 2)
  terra::values(r1) <- c(1, 2, 3, 4)
  terra::values(r2) <- c(5, 6, 7, 8)
  terra::crs(r1) <- "EPSG:4326"
  terra::crs(r2) <- "EPSG:4326"
  terra::writeRaster(r1, path1, overwrite = TRUE)
  terra::writeRaster(r2, path2, overwrite = TRUE)

  x <- wbw_read_raster(path1)
  y <- wbw_read_raster(path2)

  add_out <- x$add(y)
  sub_out <- x$subtract(path2)
  mul_out <- x$multiply(y)
  div_out <- x$divide(path2)

  expect_true(inherits(add_out, "wbw_raster"))
  expect_true(inherits(sub_out, "wbw_raster"))
  expect_true(inherits(mul_out, "wbw_raster"))
  expect_true(inherits(div_out, "wbw_raster"))

  expect_true(file.exists(add_out$file_path()))
  expect_true(file.exists(sub_out$file_path()))
  expect_true(file.exists(mul_out$file_path()))
  expect_true(file.exists(div_out$file_path()))
  expect_equal(add_out$band_count(), 1L)
})

test_that("wbw_raster unary math convenience methods work", {
  skip_if_not_installed("terra")

  path <- tempfile(fileext = ".tif")
  r <- terra::rast(nrows = 2, ncols = 2, xmin = 0, xmax = 2, ymin = 0, ymax = 2)
  terra::values(r) <- c(1, 2, 3, 4)
  terra::crs(r) <- "EPSG:4326"
  terra::writeRaster(r, path, overwrite = TRUE)

  x <- wbw_read_raster(path)

  # Ensure full unary API surface exists.
  unary_methods <- c(
    "abs", "ceil", "floor", "round", "square", "sqrt", "log10", "log2",
    "sin", "cos", "tan", "sinh", "cosh", "tanh", "exp", "exp2"
  )
  expect_true(all(vapply(unary_methods, function(m) is.function(x[[m]]), logical(1))))

  abs_out <- x$abs()
  sqrt_out <- x$sqrt()
  log10_out <- x$log10()

  expect_true(inherits(abs_out, "wbw_raster"))
  expect_true(inherits(sqrt_out, "wbw_raster"))
  expect_true(inherits(log10_out, "wbw_raster"))

  expect_true(file.exists(abs_out$file_path()))
  expect_true(file.exists(sqrt_out$file_path()))
  expect_true(file.exists(log10_out$file_path()))
})

test_that("wbw_vector write and deep_copy methods work", {
  skip_if_not_installed("terra")

  path <- tempfile(fileext = ".gpkg")
  coords <- rbind(c(0, 0), c(1, 0), c(1, 1), c(0, 1), c(0, 0))
  v <- terra::vect(coords, type = "polygons")
  terra::values(v) <- data.frame(id = 1L)
  terra::crs(v) <- "EPSG:4326"
  terra::writeVector(v, path, overwrite = TRUE)

  x <- wbw_read_vector(path)

  copy_path <- tempfile(fileext = ".gpkg")
  copied <- x$deep_copy(copy_path)
  expect_true(file.exists(copy_path))
  expect_true(inherits(copied, "wbw_vector"))
  expect_equal(copied$metadata()$feature_count, 1L)

  write_path <- tempfile(fileext = ".gpkg")
  written <- x$write(write_path)
  expect_true(file.exists(write_path))
  expect_true(inherits(written, "wbw_vector"))
})

test_that("discovery helpers search and describe tools", {
  slope <- wbw_describe_tool("slope")
  expect_true(is.list(slope))
  expect_equal(slope$id, "slope")

  matches <- wbw_search_tools("slope")
  expect_true(is.list(matches))
  expect_true(any(vapply(matches, function(t) identical(t$id, "slope"), logical(1))))
})

test_that("wbw_read_lidar returns typed lidar wrapper", {
  csv_path <- tempfile(pattern = "points_", fileext = ".csv")
  out_dir <- tempfile(pattern = "lidar_out_")
  dir.create(out_dir, recursive = TRUE)
  las_path <- file.path(out_dir, paste0(tools::file_path_sans_ext(basename(csv_path)), ".las"))

  writeLines(
    c(
      "x,y,z,i,c,rn,nr,sa,time,r,g,b",
      "0.0,0.0,10.0,100,2,1,1,0,1.25,1000,2000,3000",
      "1.0,1.0,12.0,150,6,1,2,3,2.50,1200,2200,3200"
    ),
    csv_path
  )

  wbw_run_tool(
    "ascii_to_las",
    list(
      inputs = list(csv_path),
      pattern = "x,y,z,i,c,rn,nr,sa,time,r,g,b",
      epsg_code = 4326,
      output_directory = out_dir
    )
  )

  expect_true(file.exists(las_path))

  x <- wbw_read_lidar(las_path)
  meta <- x$metadata()

  expect_true(inherits(x, "wbw_lidar"))
  expect_equal(x$get_short_filename(), basename(las_path))
  expect_equal(meta$format, "las")
  expect_equal(meta$point_count, 2)
  expect_equal(meta$crs_epsg, 4326)

  copied_path <- file.path(out_dir, "copied_points.las")
  copied <- x$deep_copy(copied_path, overwrite = TRUE)
  expect_true(file.exists(copied_path))
  expect_true(inherits(copied, "wbw_lidar"))
  expect_equal(copied$metadata()$point_count, 2)
})

test_that("wbw_read_bundle returns typed sensor bundle wrapper", {
  skip_if_not_installed("terra")

  root <- tempfile(pattern = "landsat_bundle_")
  dir.create(root, recursive = TRUE)

  mtl <- c(
    "GROUP = METADATA_FILE_INFO",
    '  LANDSAT_PRODUCT_ID = "LC09_L2SP_018030_20240202_20240210_02_T1"',
    '  SPACECRAFT_ID = "LANDSAT_9"',
    "  COLLECTION_NUMBER = 2",
    '  PROCESSING_LEVEL = "L2SP"',
    "  DATE_ACQUIRED = 2024-02-02",
    '  SCENE_CENTER_TIME = "16:42:31.1234560Z"',
    "  WRS_PATH = 18",
    "  WRS_ROW = 30",
    "  CLOUD_COVER = 12.34",
    "END_GROUP = METADATA_FILE_INFO",
    "END"
  )

  writeLines(mtl, file.path(root, "LC09_L2SP_018030_20240202_20240210_02_T1_MTL.txt"))

  r <- terra::rast(nrows = 2, ncols = 2, xmin = 0, xmax = 2, ymin = 0, ymax = 2)
  terra::values(r) <- c(1, 2, 3, 4)
  terra::crs(r) <- "EPSG:4326"

  terra::writeRaster(
    r,
    file.path(root, "LC09_L2SP_018030_20240202_20240210_02_T1_SR_B2.TIF"),
    overwrite = TRUE
  )
  terra::writeRaster(
    r,
    file.path(root, "LC09_L2SP_018030_20240202_20240210_02_T1_SR_B3.TIF"),
    overwrite = TRUE
  )
  terra::writeRaster(
    r,
    file.path(root, "LC09_L2SP_018030_20240202_20240210_02_T1_SR_B4.TIF"),
    overwrite = TRUE
  )
  terra::writeRaster(
    r,
    file.path(root, "LC09_L2SP_018030_20240202_20240210_02_T1_SR_B5.TIF"),
    overwrite = TRUE
  )
  terra::writeRaster(
    r,
    file.path(root, "LC09_L2SP_018030_20240202_20240210_02_T1_QA_PIXEL.TIF"),
    overwrite = TRUE
  )
  terra::writeRaster(
    r,
    file.path(root, "LC09_L2SP_018030_20240202_20240210_02_T1_SAA.TIF"),
    overwrite = TRUE
  )

  x <- wbw_read_bundle(root)
  meta <- x$metadata()

  expect_true(inherits(x, "wbw_sensor_bundle"))
  expect_equal(meta$family, "landsat")
  expect_equal(x$family, "landsat")
  expect_equal(x$mission(), "Landsat9")
  expect_equal(x$processing_level(), "L2")
  expect_true("B2" %in% x$list_band_keys())
  expect_true("QA_PIXEL" %in% x$list_qa_keys())
  expect_true("SAA" %in% x$list_aux_keys())

  b2 <- x$read_band("B2")
  qa <- x$read_qa_layer("QA_PIXEL")
  aux <- x$read_aux_layer("SAA")
  expect_true(inherits(b2, "wbw_raster"))
  expect_true(inherits(qa, "wbw_raster"))
  expect_true(inherits(aux, "wbw_raster"))
  expect_equal(b2$metadata()$bands, 1)

  preview <- x$read_preview_raster()
  expect_true(is.list(preview))
  expect_true(preview$kind %in% c("band", "measurement", "asset", "qa", "aux"))
  expect_true(inherits(preview$raster, "wbw_raster"))

  true_colour_output <- tempfile(pattern = "true_colour_", fileext = ".tif")
  false_colour_output <- tempfile(pattern = "false_colour_", fileext = ".tif")
  true_colour <- x$write_true_colour(output_path = true_colour_output)
  false_colour <- x$write_false_colour(output_path = false_colour_output)
  expect_true(file.exists(true_colour_output))
  expect_true(file.exists(false_colour_output))
  expect_true(inherits(true_colour, "wbw_raster"))
  expect_true(inherits(false_colour, "wbw_raster"))
  expect_equal(true_colour$metadata()$bands, 1)
  expect_equal(false_colour$metadata()$bands, 1)

  landsat <- wbw_read_landsat(root)
  expect_true(inherits(landsat, "wbw_sensor_bundle"))
  expect_equal(landsat$metadata()$family, "landsat")
})

test_that("bundle key matching supports normalized aliases", {
  keys <- c("B04", "B03", "B02")
  matched <- whiteboxworkflows:::wbw_match_bundle_key(keys, c("B4", "B3"))
  expect_equal(matched, "B04")
})

test_that("SAR families use measurement-first preview and SAR composite defaults", {
  bundle <- new.env(parent = emptyenv())
  bundle$family <- "sentinel1_safe"
  bundle$list_band_keys <- function() character(0)
  bundle$list_measurement_keys <- function() c("VV", "VH")
  bundle$list_asset_keys <- function() character(0)
  bundle$list_qa_keys <- function() character(0)
  bundle$list_aux_keys <- function() character(0)
  bundle$read_measurement <- function(key) {
    obj <- new.env(parent = emptyenv())
    obj$key <- key
    class(obj) <- c("wbw_raster", "wbw_data_object")
    obj
  }
  bundle$read_band <- function(key) stop("unexpected read_band", call. = FALSE)
  bundle$read_asset <- function(key) stop("unexpected read_asset", call. = FALSE)
  bundle$read_qa_layer <- function(key) stop("unexpected read_qa_layer", call. = FALSE)
  bundle$read_aux_layer <- function(key) stop("unexpected read_aux_layer", call. = FALSE)

  preview <- whiteboxworkflows:::wbw_bundle_pick_preview_raster(bundle)
  expect_equal(preview$kind, "measurement")
  expect_equal(preview$key, "VV")
  expect_true(inherits(preview$raster, "wbw_raster"))

  candidates <- whiteboxworkflows:::wbw_bundle_style_candidates("sentinel1_safe", "false_colour")
  expect_true("VV" %in% candidates$red)
  expect_true("VH" %in% candidates$green)

  picked <- whiteboxworkflows:::wbw_bundle_pick_channel_key(bundle, candidates$green)
  expect_equal(picked$key, "VH")
  expect_equal(picked$key_type, "measurement")
})

test_that("real Sentinel-2 fixture supports preview and composite helpers when available", {
  skip_if_not_installed("terra")

  data_root <- Sys.getenv("WBW_TEST_DATA_ROOT", "/Users/johnlindsay/Documents/data")
  candidates <- c(
    file.path(data_root, "Sentinel_Ontario"),
    file.path(
      data_root,
      "Sentinel_data_samples",
      "S2C_MSIL2A_20250712T112141_N0511_R037_T30UWE_20250712T145316.SAFE"
    )
  )
  existing <- candidates[file.exists(candidates)]
  skip_if(length(existing) == 0L, "No local Sentinel-2 fixture found under WBW_TEST_DATA_ROOT.")

  bundle <- wbw_read_bundle(existing[[1]])
  expect_equal(bundle$metadata()$family, "sentinel2_safe")

  preview <- bundle$read_preview_raster()
  expect_true(is.list(preview))
  expect_true(preview$kind %in% c("band", "measurement", "asset", "qa", "aux"))
  expect_true(inherits(preview$raster, "wbw_raster"))

  bkeys <- bundle$list_band_keys()
  expect_gt(length(bkeys), 0L)
  b <- bundle$read_band(bkeys[[1]])
  expect_true(inherits(b, "wbw_raster"))
})

test_that("real Sentinel-1 fixture supports preview and measurement reads when available", {
  skip_if_not_installed("terra")

  sar_root <- Sys.getenv(
    "WBW_SAR_FIXTURE_ROOT",
    "/Users/johnlindsay/Documents/programming/Rust/wbtools_pro/target/external_datasets/sar_fixtures"
  )
  candidates <- c(
    file.path(
      sar_root,
      "sentinel1_slc",
      "S1B_IW_SLC__1SDV_20170711T170626_20170711T170653_006443_00B539_169F.SAFE.zip"
    ),
    file.path(
      sar_root,
      "sentinel1_slc",
      "S1B_IW_SLC__1SDV_20190803T053358_20190803T053426_017417_020C1C_4B88.SAFE.zip"
    )
  )
  existing <- candidates[file.exists(candidates)]
  skip_if(length(existing) == 0L, "No local Sentinel-1 fixture found under WBW_SAR_FIXTURE_ROOT.")

  bundle <- wbw_read_bundle(existing[[1]])
  expect_equal(bundle$metadata()$family, "sentinel1_safe")

  preview <- bundle$read_preview_raster()
  expect_true(is.list(preview))
  expect_equal(preview$kind, "measurement")
  expect_true(inherits(preview$raster, "wbw_raster"))

  mkeys <- bundle$list_measurement_keys()
  expect_gt(length(mkeys), 0L)
  m <- bundle$read_measurement(mkeys[[1]])
  expect_true(inherits(m, "wbw_raster"))
})

test_that("real ICEYE fixture supports preview and asset reads when available", {
  skip_if_not_installed("terra")

  sar_root <- Sys.getenv(
    "WBW_SAR_FIXTURE_ROOT",
    "/Users/johnlindsay/Documents/programming/Rust/wbtools_pro/target/external_datasets/sar_fixtures"
  )
  candidate <- file.path(sar_root, "non_safe", "iceye", "reference_bundle")
  skip_if(!file.exists(candidate), "No local ICEYE fixture found under WBW_SAR_FIXTURE_ROOT.")

  bundle <- wbw_read_bundle(candidate)
  expect_equal(bundle$metadata()$family, "iceye")

  preview <- bundle$read_preview_raster()
  expect_true(is.list(preview))
  expect_equal(preview$kind, "asset")
  expect_true(inherits(preview$raster, "wbw_raster"))

  akeys <- bundle$list_asset_keys()
  expect_gt(length(akeys), 0L)
  a <- bundle$read_asset(akeys[[1]])
  expect_true(inherits(a, "wbw_raster"))
})

# ---------------------------------------------------------------------------
# S3 arithmetic operator tests
# ---------------------------------------------------------------------------

test_that("wbw_raster S3 + operator dispatches to add()", {
  skip_if_not_installed("terra")

  make_r <- function(vals, path) {
    r <- terra::rast(nrows = 2, ncols = 2, xmin = 0, xmax = 2, ymin = 0, ymax = 2)
    terra::values(r) <- vals
    terra::crs(r) <- "EPSG:4326"
    terra::writeRaster(r, path, overwrite = TRUE)
    wbw_read_raster(path)
  }

  p1 <- tempfile(fileext = ".tif")
  p2 <- tempfile(fileext = ".tif")
  r1 <- make_r(c(1, 2, 3, 4), p1)
  r2 <- make_r(c(10, 10, 10, 10), p2)

  result <- r1 + r2
  expect_true(inherits(result, "wbw_raster"))

  arr <- result$to_array()
  expect_equal(sort(as.numeric(arr)), c(11, 12, 13, 14))
})

test_that("wbw_raster S3 - operator dispatches to subtract()", {
  skip_if_not_installed("terra")

  make_r <- function(vals, path) {
    r <- terra::rast(nrows = 2, ncols = 2, xmin = 0, xmax = 2, ymin = 0, ymax = 2)
    terra::values(r) <- vals
    terra::crs(r) <- "EPSG:4326"
    terra::writeRaster(r, path, overwrite = TRUE)
    wbw_read_raster(path)
  }

  p1 <- tempfile(fileext = ".tif")
  p2 <- tempfile(fileext = ".tif")
  r1 <- make_r(c(10, 20, 30, 40), p1)
  r2 <- make_r(c(1, 2, 3, 4), p2)

  result <- r1 - r2
  expect_true(inherits(result, "wbw_raster"))

  arr <- result$to_array()
  expect_equal(sort(as.numeric(arr)), c(9, 18, 27, 36))
})

test_that("wbw_raster S3 * operator dispatches to multiply()", {
  skip_if_not_installed("terra")

  make_r <- function(vals, path) {
    r <- terra::rast(nrows = 2, ncols = 2, xmin = 0, xmax = 2, ymin = 0, ymax = 2)
    terra::values(r) <- vals
    terra::crs(r) <- "EPSG:4326"
    terra::writeRaster(r, path, overwrite = TRUE)
    wbw_read_raster(path)
  }

  p1 <- tempfile(fileext = ".tif")
  p2 <- tempfile(fileext = ".tif")
  r1 <- make_r(c(2, 3, 4, 5), p1)
  r2 <- make_r(c(3, 3, 3, 3), p2)

  result <- r1 * r2
  expect_true(inherits(result, "wbw_raster"))

  arr <- result$to_array()
  expect_equal(sort(as.numeric(arr)), c(6, 9, 12, 15))
})

test_that("wbw_raster S3 / operator dispatches to divide()", {
  skip_if_not_installed("terra")

  make_r <- function(vals, path) {
    r <- terra::rast(nrows = 2, ncols = 2, xmin = 0, xmax = 2, ymin = 0, ymax = 2)
    terra::values(r) <- vals
    terra::crs(r) <- "EPSG:4326"
    terra::writeRaster(r, path, overwrite = TRUE)
    wbw_read_raster(path)
  }

  p1 <- tempfile(fileext = ".tif")
  p2 <- tempfile(fileext = ".tif")
  r1 <- make_r(c(10, 20, 30, 40), p1)
  r2 <- make_r(c(2, 4, 5, 8), p2)

  result <- r1 / r2
  expect_true(inherits(result, "wbw_raster"))

  arr <- result$to_array()
  expect_equal(sort(as.numeric(arr)), c(5, 5, 5, 6))
})

test_that("wbw_raster unary - gives a clear error", {
  skip_if_not_installed("terra")

  path <- tempfile(fileext = ".tif")
  r <- terra::rast(nrows = 2, ncols = 2, xmin = 0, xmax = 2, ymin = 0, ymax = 2)
  terra::values(r) <- 1:4
  terra::crs(r) <- "EPSG:4326"
  terra::writeRaster(r, path, overwrite = TRUE)
  rx <- wbw_read_raster(path)

  expect_error(-rx, regexp = "not yet implemented")
})

# ---------------------------------------------------------------------------
# on_progress callback test
# ---------------------------------------------------------------------------

test_that("on_progress callback is invoked for each progress event", {
  session <- new.env(parent = emptyenv())
  session$run_tool_with_progress <- function(tool_id, args = list()) {
    list(
      tool_id = tool_id,
      outputs = list(),
      progress = list(
        list(pct = 0,  message = "start"),
        list(pct = 50, message = "halfway"),
        list(pct = 100, message = "done")
      )
    )
  }

  calls <- list()
  wbw_run_tool_with_progress(
    "mock_tool",
    args = list(),
    session = session,
    on_progress = function(pct, msg) {
      calls[[length(calls) + 1L]] <<- list(pct = pct, msg = msg)
    }
  )

  expect_equal(length(calls), 3L)
  expect_equal(calls[[1]]$pct, 0)
  expect_equal(calls[[2]]$pct, 50)
  expect_equal(calls[[3]]$pct, 100)
  expect_equal(calls[[3]]$msg, "done")
})

test_that("wbw_run_tool_with_progress without on_progress works unchanged", {
  session <- new.env(parent = emptyenv())
  session$run_tool_with_progress <- function(tool_id, args = list()) {
    list(tool_id = tool_id, outputs = list(), progress = list())
  }

  result <- wbw_run_tool_with_progress("mock_tool", args = list(), session = session)
  expect_true(is.list(result))
  expect_equal(result$tool_id, "mock_tool")
})

# ---------------------------------------------------------------------------
# Fixture-backed happy-path licensing tests (skipped without env credentials)
# ---------------------------------------------------------------------------

test_that("open-mode session list_tools happy path", {
  session <- wbw_session()
  tools <- session$list_tools()
  expect_type(tools, "list")
  expect_gt(length(tools), 0L)
})

test_that("entitlement session happy path when credentials provided via env", {
  ent_json <- Sys.getenv("WBW_TEST_ENTITLEMENT_JSON", unset = "")
  pub_kid   <- Sys.getenv("WBW_TEST_PUBLIC_KEY_KID",  unset = "")
  pub_key   <- Sys.getenv("WBW_TEST_PUBLIC_KEY_B64",  unset = "")
  skip_if(
    nchar(ent_json) == 0 || nchar(pub_kid) == 0 || nchar(pub_key) == 0,
    "Entitlement fixture env vars not set (WBW_TEST_ENTITLEMENT_JSON, WBW_TEST_PUBLIC_KEY_KID, WBW_TEST_PUBLIC_KEY_B64)"
  )

  session <- wbw_session(
    signed_entitlement_json = ent_json,
    public_key_kid          = pub_kid,
    public_key_b64url       = pub_key,
    include_pro             = TRUE,
    tier                    = "open"
  )
  tools <- session$list_tools()
  expect_type(tools, "list")
  expect_gt(length(tools), 0L)
})

test_that("floating session happy path when credentials provided via env", {
  lic_id       <- Sys.getenv("WBW_TEST_FLOATING_LICENSE_ID", unset = "")
  provider_url <- Sys.getenv("WBW_TEST_PROVIDER_URL",        unset = "")
  skip_if(
    nchar(lic_id) == 0 || nchar(provider_url) == 0,
    "Floating fixture env vars not set (WBW_TEST_FLOATING_LICENSE_ID, WBW_TEST_PROVIDER_URL)"
  )

  session <- wbw_session(
    floating_license_id = lic_id,
    include_pro         = TRUE,
    tier                = "open",
    provider_url        = provider_url
  )
  tools <- session$list_tools()
  expect_type(tools, "list")
  expect_gt(length(tools), 0L)
})

test_that("SAR composite candidates enhanced with available channel keys", {
  # Test that wbw_bundle_enhance_candidates intelligently expands candidates 
  # based on available keys in the bundle
  base_cand <- list(
    red = c("VV", "HH"),
    green = c("VH", "HV"),
    blue = c("VV", "HH")
  )

  # Test with NULL bundle returns base candidates unchanged
  result <- wbw_bundle_enhance_candidates("sentinel1_safe", base_cand, NULL)
  expect_equal(result$red, c("VV", "HH"))
  expect_equal(result$green, c("VH", "HV"))
  expect_equal(result$blue, c("VV", "HH"))

  # Test that the function exists and is properly exported
  expect_true(is.function(wbw_bundle_enhance_candidates))
  
  # Test that base_candidates are safely handled when provided but bundle is NULL
  empty_cand <- list(red = character(0), green = character(0), blue = character(0))
  result <- wbw_bundle_enhance_candidates("sentinel1_safe", empty_cand, NULL)
  expect_equal(result$red, character(0))
})
