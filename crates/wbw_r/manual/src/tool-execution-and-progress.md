# Tool Execution and Progress

This chapter documents execution styles and progress handling in WbW-R.

## Explicit Session Execution

```r
library(whiteboxworkflows)

s <- wbw_session()

result <- wbw_run_tool(
	'slope',
	args = list(dem = 'dem.tif', output = 'slope.tif'),
	session = s
)
print(result)
```

## Progress-Aware Execution

```r
library(whiteboxworkflows)

s <- wbw_session()

result <- wbw_run_tool_with_progress(
	'slope',
	args = list(dem = 'dem.tif', output = 'slope.tif'),
	session = s,
	on_progress = wbw_print_progress
)

str(result$progress)
```

## Custom Progress Callback

```r
progress_cb <- local({
	last <- -1L
	function(pct = NA_real_, message = '') {
		msg <- if (is.null(message)) '' else as.character(message[[1]])

		if (!is.numeric(pct) || length(pct) == 0L || is.na(pct[[1]])) {
			m <- regexpr('(-?[0-9]+(\\.[0-9]+)?)\\s*%', msg, perl = TRUE)
			if (m[[1]] >= 0L) {
				token <- regmatches(msg, m)
				pct <- as.numeric(sub('%.*$', '', token))
			} else {
				pct <- NA_real_
			}
		}

		if (is.numeric(pct) && length(pct) > 0L && !is.na(pct[[1]])) {
			value <- as.numeric(pct[[1]])
			if (value <= 1.0) value <- value * 100.0
			pct_int <- as.integer(max(0, min(100, floor(value))))
			if (pct_int > last) {
				cat(sprintf('[%3d%%] %s\\n', pct_int, msg))
				last <<- pct_int
			}
		}
	}
})

# Use: on_progress = progress_cb
```

## Recommended Execution Pattern

1. Validate tool visibility first.
2. Run with explicit session.
3. Capture progress for long operations.
4. Persist outputs and re-open typed objects for post-processing.
