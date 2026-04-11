# Example usage of the generated wbw_r facade.
#
# Assumes the native wbw_r bindings are already loaded into the R session,
# exposing functions such as run_tool_json_with_options().

source("crates/wbw_r/generated/wbw_tools_facade.R")

# Open-tier session.
session <- wbw_session()
ids <- wbw_tool_ids(session = session)
cat(sprintf("Visible tools: %d\n", length(ids)))

# One-liner tool call via session.
result <- wbw_run_tool("add", args = list(input1 = "input1.tif", input2 = "input2.tif", output = "sum.tif"), session = session)
print(result)

# Floating-license session for lab/notebook use.
# floating_session <- wbw_session(
#   floating_license_id = "FLOAT-ABC-123",
#   include_pro = TRUE,
#   tier = "open",
#   provider_url = "https://your-provider.example.com"
# )
# print(wbw_tool_ids(session = floating_session))
