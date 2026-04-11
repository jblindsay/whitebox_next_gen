# Example usage of the scaffolded whiteboxworkflows R package.
# The native wbw_r bridge layer must register runtime bindings first.
#
# Example shape:
# whiteboxworkflows::wbw_register_bindings(
#   run_tool_json_with_options = native_run_tool_json_with_options,
#   list_tools_json_with_options = native_list_tools_json_with_options,
#   run_tool_json_with_floating_license_id_options = native_run_tool_json_with_floating_license_id_options,
#   list_tools_json_with_floating_license_id_options = native_list_tools_json_with_floating_license_id_options
# )
#
# session <- whiteboxworkflows::wbw_session()
# ids <- whiteboxworkflows::wbw_tool_ids(session = session)
# print(length(ids))
#
# floating <- whiteboxworkflows::wbw_session(
#   floating_license_id = "FLOAT-ABC-123",
#   include_pro = TRUE,
#   tier = "open",
#   provider_url = "https://your-provider.example.com"
# )
# print(whiteboxworkflows::wbw_tool_ids(session = floating))
