# Floating-license online bootstrap example for Whitebox Workflows R.
#
# This script assumes your R wrapper exposes the runtime/listing functions from
# wbw_r (for example, via list_tools_json_with_options/include_pro semantics).

Sys.setenv(WBW_LICENSE_PROVIDER_URL = "https://your-provider.example.com")
Sys.setenv(WBW_FLOATING_LICENSE_ID = "FLOAT-ABC-123")

# Optional:
Sys.setenv(WBW_LICENSE_POLICY = "fail_open")
Sys.setenv(WBW_LICENSE_LEASE_SECONDS = "3600")
# Sys.setenv(WBW_LICENSE_STATE_PATH = "/tmp/wbw_license_state.json")
# Sys.setenv(WBW_MACHINE_ID = "my-workstation-01")
# Sys.setenv(WBW_CUSTOMER_ID = "cust_123")

# Replace this with your package's exported wrapper function, e.g.
# tools_json <- whiteboxworkflows::list_tools_json_with_options(include_pro = TRUE, tier = "open")
# cat(tools_json)
