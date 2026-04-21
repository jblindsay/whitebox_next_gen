# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]

### Changed
- Removed the unused direct dependency on `wbgeotiff`. Datum/grid loading in
	`wbprojection` currently uses native parsers (e.g., NTv2, NADCON ASCII,
	GTX) and does not require GeoTIFF IO.

## [0.1.0] - 2026-03-31
### Added
- Initial published release.
