# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]
### Changed
- GeoTIFF write paths now parallelize strip and tile chunk encoding using Rayon,
  improving throughput for large compressed and tiled outputs while preserving
  existing file format behavior.

## [0.1.0] - 2026-03-31
### Added
- Initial published release.
