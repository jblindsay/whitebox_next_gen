# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]

### Added
### Changed
### Fixed

## [0.1.2] - 2026-06-14

### Added
- Added pilot explicit parameter schemas for `extract_streams` and `vector_stream_network_analysis` via `stream_tool_param_schemas(...)`.
- Added explicit `Tool` metadata/manifests for the two pilot stream tools so emitted metadata includes canonical parameter names/descriptions and defaults.

### Changed
- Re-exported stream tool schema mapping helper from `tools` module for binding/front-end metadata integration.

### Fixed
- Fixed `quantiles` tool catastrophic failure on rasters with extreme positive skewness. The fixed-bin histogram approach (10,000 bins over full [min, max] range) produced bin widths so coarse that quantile boundaries fell within a single bin, causing all valid pixels to be assigned the highest class. Replaced with an adaptive-bin histogram that scales bin count proportionally to valid cell count (capped at 32 MB), ensuring quantile boundaries map to distinct bins regardless of distribution shape. The tool now correctly computes equal-count quantile classes on highly skewed data while maintaining O(n) time complexity and cache-friendly memory usage.