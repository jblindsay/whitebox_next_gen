# Overview

This manual is the long-form user guide for the WbW-Py API.

WbW-Py (Whitebox Workflows for Python) is the Python frontend to Whitebox Next
Gen, exposing a high-performance geospatial engine for raster, vector, lidar,
and sensor-bundle workflows through a Pythonic API.

Whitebox is a long-running geospatial project that began in academia and has
evolved into a broad analysis platform with particular strength in
geomorphometry, hydrological analysis, lidar processing, and remote sensing.
Whitebox Next Gen is the current Rust-based major iteration, designed to pair
high performance with modern APIs.

Within that architecture, WbW-Py is the user-facing orchestration layer for
Python users. It is intended for three common use cases:
- exploratory research notebooks and small scripts,
- reproducible batch workflows in production pipelines,
- integration with the broader Python geospatial ecosystem.

This manual balances beginner accessibility with reference-grade detail:
- beginner friendly: clear chapter flow and runnable examples,
- canonical reference: explicit patterns, boundary conditions, and validation
	guidance aligned with backend capabilities.

## How to Use This Manual

This guide is designed for practitioners who want to move from exploratory scripts
to reliable, repeatable geospatial pipelines. The examples are intentionally
script-first, but each chapter also communicates design intent: when to keep data
in memory, when to persist to disk, and how to validate each processing stage.

Readers get the best results by following chapters in order the first time,
then using the script index as a task-oriented lookup once the core patterns are
familiar.

When adapting examples, treat each script as a template with four parts:
1. session or environment setup,
2. data intake,
3. transformation chain,
4. validation and persistence.

This structure helps keep your own scripts consistent as they grow.

Goals:
- Comprehensive API coverage.
- Practical, script-first learning path.
- Consistent chapter flow aligned with the project README.

Documentation style rules:
- Every major concept must include executable code snippets.
- Each chapter should include at least one complete end-to-end script.
- Tool-specific parameter details are linked to tool reference docs where needed.

## Common Pitfalls and Validation

- Confirm inputs exist and have the expected CRS/schema/metadata before running long workflows.
- Prefer explicit output names and formats for reproducibility, especially in batch scripts.
- Re-open and inspect outputs after major steps to validate assumptions before chaining more tools.
- For performance-sensitive runs, start with a small representative subset, then scale to full data.
