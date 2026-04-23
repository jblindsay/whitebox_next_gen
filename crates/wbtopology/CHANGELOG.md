# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]
### Added
- Added a fast Delaunay triangulation implementation in `src/fast_triangulation.rs`, adapted with attribution from the Delaunator algorithm lineage.
- Added public export `delaunay_triangulation_fast` for high-throughput triangulation workflows.

### Changed
- Expanded triangulation test coverage with fast-path baseline tests (square and collinear cases).
- Reset `src/fast_triangulation.rs` to a closer upstream-style delaunator port so performance work can restart from a simpler baseline.

## [0.1.0] - 2026-03-31
### Added
- Initial published release.
