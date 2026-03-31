.wbw_bindings <- new.env(parent = emptyenv())

wbw_register_bindings <- function(run_tool_json_with_options,
                                  list_tools_json_with_options,
                                  run_tool_json_with_floating_license_id_options = NULL,
                                  list_tools_json_with_floating_license_id_options = NULL) {
  assign("run_tool_json_with_options", run_tool_json_with_options, envir = .wbw_bindings)
  assign("list_tools_json_with_options", list_tools_json_with_options, envir = .wbw_bindings)
  assign(
    "run_tool_json_with_floating_license_id_options",
    run_tool_json_with_floating_license_id_options,
    envir = .wbw_bindings
  )
  assign(
    "list_tools_json_with_floating_license_id_options",
    list_tools_json_with_floating_license_id_options,
    envir = .wbw_bindings
  )
  invisible(TRUE)
}

wbw_clear_bindings <- function() {
  rm(list = ls(envir = .wbw_bindings, all.names = TRUE), envir = .wbw_bindings)
  invisible(TRUE)
}

.wbw_require_binding <- function(name) {
  if (!exists(name, envir = .wbw_bindings, inherits = FALSE)) {
    stop(
      sprintf(
        paste0(
          "wbw_r native binding '%s' is not registered. ",
          "Call wbw_register_bindings(...) from the runtime bridge layer first."
        ),
        name
      ),
      call. = FALSE
    )
  }

  fn <- get(name, envir = .wbw_bindings, inherits = FALSE)
  if (is.null(fn)) {
    stop(
      sprintf("wbw_r native binding '%s' is unavailable in this build.", name),
      call. = FALSE
    )
  }
  fn
}

run_tool_json_with_options <- function(tool_id, args_json, include_pro = FALSE, tier = "open") {
  .wbw_require_binding("run_tool_json_with_options")(tool_id, args_json, include_pro, tier)
}

list_tools_json_with_options <- function(include_pro = FALSE, tier = "open") {
  .wbw_require_binding("list_tools_json_with_options")(include_pro, tier)
}

run_tool_json_with_floating_license_id_options <- function(tool_id,
                                                           args_json,
                                                           floating_license_id,
                                                           include_pro = TRUE,
                                                           fallback_tier = "open",
                                                           provider_url = NULL,
                                                           machine_id = NULL,
                                                           customer_id = NULL) {
  .wbw_require_binding("run_tool_json_with_floating_license_id_options")(
    tool_id,
    args_json,
    floating_license_id,
    include_pro,
    fallback_tier,
    provider_url,
    machine_id,
    customer_id
  )
}

list_tools_json_with_floating_license_id_options <- function(floating_license_id,
                                                             include_pro = TRUE,
                                                             fallback_tier = "open",
                                                             provider_url = NULL,
                                                             machine_id = NULL,
                                                             customer_id = NULL) {
  .wbw_require_binding("list_tools_json_with_floating_license_id_options")(
    floating_license_id,
    include_pro,
    fallback_tier,
    provider_url,
    machine_id,
    customer_id
  )
}
