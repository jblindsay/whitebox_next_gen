# Quick Start

This walkthrough verifies that WbW-QGIS is running and can execute tools.

## 1. Enable Plugin

1. Start QGIS.
2. Open Plugin Manager.
3. Enable Whitebox Workflows.

## 2. Confirm Provider Availability

1. Open the Processing Toolbox.
2. Confirm Whitebox provider entries appear.
3. If tools are missing, trigger a discovery refresh from the plugin panel.

## 3. Run a First Tool

A common smoke test is a simple raster analysis tool with a small input file.

Recommended pattern:
1. Choose a small test raster.
2. Run a lightweight tool from the Whitebox provider.
3. Write output to a temporary file.
4. Load and inspect the output layer in QGIS.

## 4. Validate Results

- Confirm the output file exists.
- Confirm layer metadata and CRS are as expected.
- Confirm visual result is plausible for the input.

If any step fails, continue to the Troubleshooting chapter.
