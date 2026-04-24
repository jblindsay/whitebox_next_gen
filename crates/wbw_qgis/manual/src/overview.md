# Overview

This manual is the long-form user guide for WbW-QGIS.

WbW-QGIS (Whitebox Workflows for QGIS) is the QGIS frontend for Whitebox Next Gen.
It provides a QGIS-native way to discover, configure, and run Whitebox tools
through the Processing framework and plugin UI.

Whitebox Next Gen uses a layered architecture:
- backend geospatial engines and tools in Rust,
- frontend runtimes for Python, R, and QGIS,
- shared tool taxonomy and capability metadata.

Whitebox Next Gen is intentionally full-stack: core geospatial capabilities
that are often delegated to external C/C++ dependencies in other GIS platforms
(for example raster I/O, projections, geometry/topology operations, and lidar
handling) are implemented in the Whitebox codebase itself. This architecture is
unusual in GIS and provides practical benefits for users: consistent behavior
across platforms, tighter control over correctness and performance, fewer
system-level dependency issues during installation, and faster iteration when
fixing bugs or introducing new capabilities.

Within this model, WbW-QGIS is intentionally a thin integration layer. It handles
QGIS presentation and orchestration while computation remains in the Whitebox
backend runtime.

## What This Manual Covers

This guide focuses on practical use of WbW-QGIS:
- setting up the plugin and runtime correctly,
- understanding discovery and provider refresh,
- running tools through QGIS Processing,
- handling output and troubleshooting common issues.

The manual is written for both analysts and developers who use QGIS as the
primary working environment.

## Goals

- Provide a stable onboarding path for local installation.
- Document the operational behavior of plugin discovery and execution.
- Clarify tier and licensing behavior in QGIS.
- Reduce setup friction and runtime ambiguity.

## How to Use This Manual

For first-time setup, read chapters in order:
1. Installation and Setup
2. Build and Preview
3. Quick Start
4. Runtime and Discovery

After setup is stable, use the remaining chapters as reference material.
