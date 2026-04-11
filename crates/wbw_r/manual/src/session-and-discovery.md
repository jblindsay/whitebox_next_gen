# Session and Discovery

This chapter covers session lifecycle and tool discovery patterns.

## Create Session

```r
library(whiteboxworkflows)

s <- wbw_session()
print(s)
```

## Visibility Checks

```r
library(whiteboxworkflows)

s <- wbw_session()
ids <- wbw_tool_ids(session = s)
cat('Visible tools:', length(ids), '\n')

if (!wbw_has_tool('slope', session = s)) {
	stop('slope is not available in this runtime session')
}
```

## Search and Describe

```r
library(whiteboxworkflows)

matches <- wbw_search_tools('lidar')
print(matches[1:min(5, length(matches))])

desc <- wbw_describe_tool('slope')
print(desc)
```

## Category-Based Discovery

```r
library(whiteboxworkflows)

summary <- wbw_category_summary()
print(summary)

cats <- wbw_get_all_categories()
print(cats)

raster_tools <- wbw_tools_in_category('Raster')
print(head(raster_tools, 20))
```

## Recommended Discovery Pattern

1. Build session explicitly.
2. Check required tools with `wbw_has_tool(...)`.
3. Use `wbw_describe_tool(...)` for parameter verification.
4. Use category functions to drive guided UX or script validation.
