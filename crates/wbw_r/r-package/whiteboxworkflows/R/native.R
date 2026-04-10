# Generated low-level native wrappers for the wbw_r extendr module.
# These wrappers are intentionally narrow and feed the higher-level facade.

#' @docType package
#' @usage NULL
#' @useDynLib whiteboxworkflows, .registration = TRUE
NULL

list_tools_json <- function() .Call("wrap__list_tools_json", PACKAGE = "whiteboxworkflows")

list_tools_json_with_options <- function(include_pro = FALSE, tier = "open") {
  .Call(
    "wrap__list_tools_json_with_options",
    include_pro,
    tier,
    PACKAGE = "whiteboxworkflows"
  )
}

list_tools_json_with_entitlement_options <- function(signed_entitlement_json,
                                                     public_key_kid,
                                                     public_key_b64url,
                                                     include_pro = FALSE,
                                                     fallback_tier = "open") {
  .Call(
    "wrap__list_tools_json_with_entitlement_options",
    signed_entitlement_json,
    public_key_kid,
    public_key_b64url,
    include_pro,
    fallback_tier,
    PACKAGE = "whiteboxworkflows"
  )
}

list_tools_json_with_entitlement_file_options <- function(entitlement_file,
                                                          public_key_kid,
                                                          public_key_b64url,
                                                          include_pro = FALSE,
                                                          fallback_tier = "open") {
  .Call(
    "wrap__list_tools_json_with_entitlement_file_options",
    entitlement_file,
    public_key_kid,
    public_key_b64url,
    include_pro,
    fallback_tier,
    PACKAGE = "whiteboxworkflows"
  )
}

list_tools_json_with_floating_license_id_options <- function(floating_license_id,
                                                             include_pro = TRUE,
                                                             fallback_tier = "open",
                                                             provider_url = NULL,
                                                             machine_id = NULL,
                                                             customer_id = NULL) {
  .Call(
    "wrap__list_tools_json_with_floating_license_id_options",
    floating_license_id,
    include_pro,
    fallback_tier,
    provider_url,
    machine_id,
    customer_id,
    PACKAGE = "whiteboxworkflows"
  )
}

run_tool_json <- function(tool_id, args_json) {
  .Call("wrap__run_tool_json", tool_id, args_json, PACKAGE = "whiteboxworkflows")
}

run_tool_json_with_progress <- function(tool_id, args_json) {
  .Call("wrap__run_tool_json_with_progress", tool_id, args_json, PACKAGE = "whiteboxworkflows")
}

run_tool_json_with_options <- function(tool_id,
                                       args_json,
                                       include_pro = FALSE,
                                       tier = "open") {
  .Call(
    "wrap__run_tool_json_with_options",
    tool_id,
    args_json,
    include_pro,
    tier,
    PACKAGE = "whiteboxworkflows"
  )
}

run_tool_json_with_progress_options <- function(tool_id,
                                                args_json,
                                                include_pro = FALSE,
                                                tier = "open") {
  .Call(
    "wrap__run_tool_json_with_progress_options",
    tool_id,
    args_json,
    include_pro,
    tier,
    PACKAGE = "whiteboxworkflows"
  )
}

run_tool_json_with_entitlement_options <- function(tool_id,
                                                   args_json,
                                                   signed_entitlement_json,
                                                   public_key_kid,
                                                   public_key_b64url,
                                                   include_pro = FALSE,
                                                   fallback_tier = "open") {
  .Call(
    "wrap__run_tool_json_with_entitlement_options",
    tool_id,
    args_json,
    signed_entitlement_json,
    public_key_kid,
    public_key_b64url,
    include_pro,
    fallback_tier,
    PACKAGE = "whiteboxworkflows"
  )
}

run_tool_json_with_progress_entitlement_options <- function(tool_id,
                                                            args_json,
                                                            signed_entitlement_json,
                                                            public_key_kid,
                                                            public_key_b64url,
                                                            include_pro = FALSE,
                                                            fallback_tier = "open") {
  .Call(
    "wrap__run_tool_json_with_progress_entitlement_options",
    tool_id,
    args_json,
    signed_entitlement_json,
    public_key_kid,
    public_key_b64url,
    include_pro,
    fallback_tier,
    PACKAGE = "whiteboxworkflows"
  )
}

run_tool_json_with_entitlement_file_options <- function(tool_id,
                                                        args_json,
                                                        entitlement_file,
                                                        public_key_kid,
                                                        public_key_b64url,
                                                        include_pro = FALSE,
                                                        fallback_tier = "open") {
  .Call(
    "wrap__run_tool_json_with_entitlement_file_options",
    tool_id,
    args_json,
    entitlement_file,
    public_key_kid,
    public_key_b64url,
    include_pro,
    fallback_tier,
    PACKAGE = "whiteboxworkflows"
  )
}

run_tool_json_with_progress_entitlement_file_options <- function(tool_id,
                                                                 args_json,
                                                                 entitlement_file,
                                                                 public_key_kid,
                                                                 public_key_b64url,
                                                                 include_pro = FALSE,
                                                                 fallback_tier = "open") {
  .Call(
    "wrap__run_tool_json_with_progress_entitlement_file_options",
    tool_id,
    args_json,
    entitlement_file,
    public_key_kid,
    public_key_b64url,
    include_pro,
    fallback_tier,
    PACKAGE = "whiteboxworkflows"
  )
}

run_tool_json_with_floating_license_id_options <- function(tool_id,
                                                           args_json,
                                                           floating_license_id,
                                                           include_pro = TRUE,
                                                           fallback_tier = "open",
                                                           provider_url = NULL,
                                                           machine_id = NULL,
                                                           customer_id = NULL) {
  .Call(
    "wrap__run_tool_json_with_floating_license_id_options",
    tool_id,
    args_json,
    floating_license_id,
    include_pro,
    fallback_tier,
    provider_url,
    machine_id,
    customer_id,
    PACKAGE = "whiteboxworkflows"
  )
}

run_tool_json_with_progress_floating_license_id_options <- function(tool_id,
                                                                    args_json,
                                                                    floating_license_id,
                                                                    include_pro = TRUE,
                                                                    fallback_tier = "open",
                                                                    provider_url = NULL,
                                                                    machine_id = NULL,
                                                                    customer_id = NULL) {
  .Call(
    "wrap__run_tool_json_with_progress_floating_license_id_options",
    tool_id,
    args_json,
    floating_license_id,
    include_pro,
    fallback_tier,
    provider_url,
    machine_id,
    customer_id,
    PACKAGE = "whiteboxworkflows"
  )
}

generate_r_wrapper_module_with_options <- function(include_pro = FALSE, tier = "open") {
  .Call(
    "wrap__generate_r_wrapper_module_with_options",
    include_pro,
    tier,
    PACKAGE = "whiteboxworkflows"
  )
}

lidar_metadata_json <- function(path) {
  .Call("wrap__lidar_metadata_json", path, PACKAGE = "whiteboxworkflows")
}

sensor_bundle_metadata_json <- function(path) {
  .Call("wrap__sensor_bundle_metadata_json", path, PACKAGE = "whiteboxworkflows")
}

sensor_bundle_resolve_raster_path <- function(bundle_root, key, key_type) {
  .Call(
    "wrap__sensor_bundle_resolve_raster_path",
    bundle_root,
    key,
    key_type,
    PACKAGE = "whiteboxworkflows"
  )
}

vector_copy_to_path <- function(src, dst) {
  .Call("wrap__vector_copy_to_path", src, dst, PACKAGE = "whiteboxworkflows")
}

raster_metadata_json <- function(path) {
  .Call("wrap__raster_metadata_json", path, PACKAGE = "whiteboxworkflows")
}

vector_metadata_json <- function(path) {
  .Call("wrap__vector_metadata_json", path, PACKAGE = "whiteboxworkflows")
}
