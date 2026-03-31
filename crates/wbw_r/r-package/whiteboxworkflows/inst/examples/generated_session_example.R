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
# session <- whiteboxworkflows::whitebox_tools()
# tools <- session$list_tools()
# print(length(tools))
#
# floating <- whiteboxworkflows::whitebox_tools(
#   floating_license_id = "FLOAT-ABC-123",
#   include_pro = TRUE,
#   tier = "open",
#   provider_url = "https://your-provider.example.com"
# )
# print(floating$list_tools())
