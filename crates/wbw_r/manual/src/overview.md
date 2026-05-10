# Overview

This manual is the long-form user guide for the WbW-R API.

WbW-R (Whitebox Workflows for R) is the R frontend to Whitebox Next Gen,
providing access to modern geospatial processing for raster, vector, lidar, and
sensor-bundle data through an R-friendly API and workflow model.

Whitebox is a long-running geospatial project that originated in academia and
has grown into a broad analysis platform with recognized strengths in
geomorphometry, hydrology, lidar processing, and remote sensing. Whitebox Next
Gen is the current Rust-based major iteration focused on performance,
cross-platform reliability, and modern data formats.

Whitebox Next Gen is intentionally full-stack: core geospatial capabilities
that are often delegated to external C/C++ dependencies in other GIS platforms
(for example raster I/O, projections, geometry/topology operations, and lidar
handling) are implemented in the Whitebox codebase itself. This architecture is
unusual in GIS and provides practical benefits for users: consistent behavior
across platforms, tighter control over correctness and performance, fewer
system-level dependency issues during installation, and faster iteration when
fixing bugs or introducing new capabilities.

In this architecture, WbW-R is the orchestration layer for R users who need
both practical scripting ergonomics and backend-scale performance.

This manual is written to be both:
- beginner friendly: clear progression and runnable examples,
- canonical reference: explicit operational patterns, constraints, and
	validation guidance aligned with backend behavior.

## How to Use This Manual

This guide is intended for analysts who want to move from one-off exploratory
commands to stable, scriptable geospatial workflows. The examples are practical
and executable, but each chapter also explains the operational intent behind the
code: when to keep work in memory, when to persist outputs, and how to validate
results between stages.

A good first pass is chapter-order reading. After that, use the script index as
a task-oriented entry point for adapting workflows to your own projects.

When adapting examples, keep a consistent script shape:
1. session setup,
2. input loading,
3. transformation chain,
4. validation and output persistence.

This structure keeps scripts easier to review, test, and maintain.

Goals:
- Comprehensive API coverage.
- Script-first documentation style.
- Chapter layout aligned with the project README.

Documentation style rules:
- Every major concept includes runnable code snippets.
- Each chapter includes at least one end-to-end workflow script.
- Tool parameter specifics are linked to shared tool reference docs as needed.

## Common Pitfalls and Validation

- Confirm inputs exist and have the expected CRS/schema/metadata before running long workflows.
- Prefer explicit output names and formats for reproducibility, especially in batch scripts.
- Re-open and inspect outputs after major steps to validate assumptions before chaining more tools.
- For performance-sensitive runs, start with a small representative subset, then scale to full data.

## Write Option References

For quick access to output option tables:

- Raster: [Output Controls -> Raster Write Option Reference](output-controls.md#raster-write-option-reference)
- Vector: [Output Controls -> Vector Write Option Reference](output-controls.md#vector-write-option-reference)
- Lidar: [Output Controls -> Lidar Write Option Reference](output-controls.md#lidar-write-option-reference)
