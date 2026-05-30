# Installation and Setup

Treat setup as an explicit validation stage, not a one-time administrative task.
The smoke test in this chapter confirms that the package and runtime can create
a session and enumerate tools, which catches many environment issues before they
surface in long-running processing scripts.

For collaborative projects, this chapter doubles as a baseline checklist for
new machines and CI environments.

## Install

Use this to install the current local package build for development and testing.

```bash
R CMD INSTALL crates/wbw_r/r-package/whiteboxworkflows
```

To install the published binary package from WhiteboxGeo, use the WhiteboxGeo
repository together with CRAN:

```r
install.packages(
	"whiteboxworkflows",
	repos = c(
		WhiteboxGeo = "https://www.whiteboxgeo.com/r",
		CRAN = "https://cloud.r-project.org"
	)
)
```

The WhiteboxGeo repository currently serves binary builds for R 4.5 and R 4.6
on the supported platforms.

## Smoke Test

This verifies that startup succeeds and tool enumeration is available in your
current runtime.

```bash
Rscript -e 'library(whiteboxworkflows); s <- wbw_session(); cat(length(wbw_tool_ids(session = s)), "\n")'
```
