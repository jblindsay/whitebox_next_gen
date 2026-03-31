# Stable facade over generated wbw_r wrappers.
#
# Source this file after loading the native wbw_r bindings and the generated
# wrapper module file. This keeps user code pointed at a small stable surface
# while generated wrapper internals can be refreshed as tool manifests evolve.

source("crates/wbw_r/generated/wbw_tools_generated.R")

whitebox_tools <- function(floating_license_id = NULL,
                           include_pro = NULL,
                           tier = "open",
                           provider_url = NULL,
                           machine_id = NULL,
                           customer_id = NULL) {
  wbw_make_session(
    floating_license_id = floating_license_id,
    include_pro = include_pro,
    tier = tier,
    provider_url = provider_url,
    machine_id = machine_id,
    customer_id = customer_id
  )
}

wbw_list_tools <- function(floating_license_id = NULL,
                           include_pro = NULL,
                           tier = "open",
                           provider_url = NULL,
                           machine_id = NULL,
                           customer_id = NULL) {
  session <- whitebox_tools(
    floating_license_id = floating_license_id,
    include_pro = include_pro,
    tier = tier,
    provider_url = provider_url,
    machine_id = machine_id,
    customer_id = customer_id
  )
  session$list_tools()
}

wbw_run_tool <- function(tool_id,
                         args = list(),
                         floating_license_id = NULL,
                         include_pro = NULL,
                         tier = "open",
                         provider_url = NULL,
                         machine_id = NULL,
                         customer_id = NULL) {
  session <- whitebox_tools(
    floating_license_id = floating_license_id,
    include_pro = include_pro,
    tier = tier,
    provider_url = provider_url,
    machine_id = machine_id,
    customer_id = customer_id
  )
  session$run_tool(tool_id, args)
}
