# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]

## [0.1.2] – 2026-05-09 (Reaffirmed)

### Testing
- Interop Phase A GeoTIFF conformance: 33/33 passing across 11 CRS families.
- Forward/inverse tolerance validation confirmed for all tested CRS.

*Note: Version 0.1.2 is being published as-is as part of the interop release milestone (2026-05-09) to ensure downstream dependency resolution for wbraster 0.1.5.*

## [0.1.2] - 2026-05-07

### Changed
- Added a single-band GeoTIFF read fast path in `GeoTiff::read_band_bytes`:
    for `samples_per_pixel == 1` with chunky layout, the decoded pixel buffer is
    now returned directly instead of running per-pixel band extraction.
- Removed unused direct `zstd` and `ruzstd` dependencies. Current `wbgeotiff`
    codec support covers Deflate, LZW, PackBits, JPEG, WebP, and JPEG-XL; the
    crate does not currently implement GeoTIFF Zstandard compression paths.
- Swapped the WebP backend from the `webp`/`libwebp-sys` stack to pure-Rust
    `webp-rust`. Opaque WebP writes stay lossy; alpha-bearing RGBA WebP writes
    currently fall back to lossless WebP because the pure-Rust lossy encoder
    does not yet support alpha input.

## [0.1.1] - 2026-04-11
### Added
- Added `GeoTiff::value_transform()` to expose optional linear sample-value transforms
    parsed from GDAL metadata (`scale`/`offset` style metadata), enabling higher
    layers to normalize physical values on read.

### Changed
- GeoTIFF write paths now parallelize strip and tile chunk encoding using Rayon,
    improving throughput for large compressed and tiled outputs while preserving
    existing file format behavior.

## [0.1.0] - 2026-03-31
### Added
- Initial published release.
