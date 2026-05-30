# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]

### Added
- Added pilot explicit parameter schemas for `extract_streams` and `vector_stream_network_analysis` via `stream_tool_param_schemas(...)`.
- Added explicit `Tool` metadata/manifests for the two pilot stream tools so emitted metadata includes canonical parameter names/descriptions and defaults.

### Changed
- Re-exported stream tool schema mapping helper from `tools` module for binding/front-end metadata integration.

### Fixed
- None yet.