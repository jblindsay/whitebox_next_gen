# whiteboxworkflows R Package Scaffold

This directory contains the emerging R package layer for wbw_r.

## Current state

Implemented:

- Package metadata scaffold (`DESCRIPTION`, `NAMESPACE`)
- Stable facade functions in `R/facade.R`
- Generated wrapper layer in `R/zz_generated_wrappers.R`
- Native Rust export layer in the `wbw_r` crate via extendr
- Package-native shared-library build wiring in `src/`

Still required for full polish:

- End-to-end package installation test across more than one host configuration
- Namespace/export cleanup and package metadata polish
- Additional ergonomic tests around the high-level session facade

## Current development workflow

Install the package with standard R tooling:

```bash
R CMD INSTALL crates/wbw_r/r-package/whiteboxworkflows
```

This now builds the `wbw_r` static library during package compilation, links it
into the package shared library, and loads the resulting package library through
`useDynLib(whiteboxworkflows, ...)`.

The installed package now exports the stable facade and low-level JSON runtime
functions explicitly rather than exporting every generated tool wrapper into the
attached search path. That avoids masking unrelated base and stats functions
when the package is attached.

Optional environment variables:

- `WBW_R_PACKAGE_PRO=true` to build against the Pro-enabled runtime.
- `WBW_R_PACKAGE_RELEASE=true` to build the Rust library in release mode.

Note: `jsonlite` must be installed for the generated/session wrapper layer.

## Tests

Run the package smoke tests with:

```bash
Rscript -e 'testthat::test_local("crates/wbw_r/r-package/whiteboxworkflows")'
```

The initial test suite validates:

- low-level JSON tool listing,
- high-level session facade construction,
- facade and direct listing consistency.

## Near-term goal

Validate the new package-native install/load path end to end and then tighten
the package surface around tests, exports, and documentation.
