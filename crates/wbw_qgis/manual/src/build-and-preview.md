# Build and Preview

The WbW-QGIS manual uses mdBook, matching the WbW-Python and WbW-R manuals.

## Build the Manual

From the manual directory:

```bash
cd crates/wbw_qgis/manual
mdbook build
```

Generated output will be written to:

- crates/wbw_qgis/manual/book

## Live Preview

For a local preview server:

```bash
cd crates/wbw_qgis/manual
mdbook serve --open
```

This starts a local server and opens the manual in your browser.

## Writing Conventions

- Keep examples task-oriented and reproducible.
- Prefer short, complete QGIS workflows over abstract API descriptions.
- Document expected outputs and validation checks where possible.
