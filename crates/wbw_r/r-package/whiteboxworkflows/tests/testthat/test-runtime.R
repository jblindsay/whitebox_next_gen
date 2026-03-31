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
