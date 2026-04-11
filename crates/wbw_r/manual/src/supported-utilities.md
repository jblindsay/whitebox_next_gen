# Supported Utilities

WbW-R includes utility-oriented APIs in addition to tool-category execution.

## Utility Areas

- Progress callback helpers:
  - `wbw_print_progress`
  - `wbw_make_progress_printer(...)`
- Category/discovery helpers:
  - `wbw_get_all_categories()`
  - `wbw_category_summary()`
  - `wbw_tools_in_category(...)`
  - `wbw_list_tools_by_category()`
- Typed object utility methods:
  - metadata and conversion helpers on raster/vector/lidar/bundle wrappers

## Progress Utilities

```r
library(whiteboxworkflows)

progress_cb <- wbw_make_progress_printer(min_increment = 5, show_messages = TRUE)

result <- wbw_run_tool_with_progress(
  'slope',
  args = list(dem = 'dem.tif', output = 'slope.tif'),
  session = wbw_session(),
  on_progress = progress_cb
)
```

## Discovery Utilities

```r
library(whiteboxworkflows)

print(wbw_category_summary())
print(wbw_get_all_categories())
print(head(wbw_tools_in_category('Hydrology'), 20))
```

## Typed Wrapper Utility Patterns

```r
library(whiteboxworkflows)

r <- wbw_read_raster('dem.tif')
v <- wbw_read_vector('roads.gpkg')
l <- wbw_read_lidar('survey.las')

print(r$metadata())
print(v$metadata())
print(l$metadata())
```

For sensor-bundle utility methods, see [Working with Sensor Bundles](./working-with-sensor-bundles.md).