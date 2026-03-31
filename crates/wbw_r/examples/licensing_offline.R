# Offline licensing examples for Whitebox Workflows R.

# 1) Offline OSS fallback (no provider bootstrap):
Sys.unsetenv("WBW_LICENSE_PROVIDER_URL")
Sys.unsetenv("WBW_FLOATING_LICENSE_ID")

# Replace with your package function:
# tools_json <- whiteboxworkflows::list_tools_json_with_options(include_pro = FALSE, tier = "open")
# cat(tools_json)

# 2) Offline signed-entitlement path:
# If a signed entitlement JSON + public key are available locally,
# use your wrapper's entitlement-based list/run functions.
# Example (replace with actual exported R wrapper name/signature):
# tools_json <- whiteboxworkflows::list_tools_json_with_entitlement_options(
#   signed_entitlement_json = readChar("signed_entitlement.json", file.info("signed_entitlement.json")$size),
#   public_key_kid = "k1",
#   public_key_b64url = "REPLACE_WITH_PROVIDER_PUBLIC_KEY",
#   include_pro = TRUE,
#   fallback_tier = "open"
# )
# cat(tools_json)
