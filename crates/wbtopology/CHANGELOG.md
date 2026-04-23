# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]
### Added
- Added a fast Delaunay triangulation implementation in `src/fast_triangulation.rs`, adapted with attribution from the Delaunator algorithm lineage.
- Added public export `delaunay_triangulation_fast` for high-throughput triangulation workflows.

### Changed
- Expanded triangulation test coverage with fast-path baseline tests (square and collinear cases).
- Converted `legalize` from recursive to iterative with explicit work stack to prevent OS stack overflow on large point clouds (65M+ points).
- Fixed hull edge traversal in legalization from `hull.next` to `hull.prev` to match upstream delaunator behavior and prevent runaway legalization cascades.

## [0.1.0] - 2026-03-31
### Added
- Initial published release.
