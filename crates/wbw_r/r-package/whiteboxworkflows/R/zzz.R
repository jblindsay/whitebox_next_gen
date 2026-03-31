.onLoad <- function(libname, pkgname) {
  if (!is.loaded("wrap__list_tools_json")) {
    stop(
      "whiteboxworkflows native routines were not loaded during package startup.",
      call. = FALSE
    )
  }

  invisible()
}
