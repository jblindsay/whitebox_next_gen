# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]

### Added
- Initial crate scaffolding for `wbhdf`.
- Module skeletons for error handling, superblock, object headers, datasets, chunk indexing, filters, datatypes, and attributes.
- Placeholder unit and integration test harnesses.
- Internal design and format-notes documentation stubs.
- Canonical fixture helpers in `src/fixtures.rs` for external fixture directory resolution and smoke fixture detection.
- Initial fixture manifest at `tests/fixtures/manifest.toml` for GEDI, ICESat-2, VIIRS, and MODIS target paths.
- Day 2 metadata smoke-path probe API in `superblock`:
	- HDF5 signature validation,
	- minimal superblock version extraction,
	- heuristic top-level group discovery.
- Day 3 B-tree v1 kickoff implementation in `src/btree.rs`:
	- typed node header / internal record / leaf record parsing,
	- key-range child routing helper,
	- deterministic chunk-address lookup API with explicit error reporting.
- Synthetic B-tree unit tests and integration smoke test that gracefully skips when fixtures are unavailable.
- Day 4 dataset chunk-lookup validation path:
	- added `DatasetChunkLocator` in `src/dataset.rs` to wire B-tree lookup into dataset-level flow,
	- added known-address validation tests for deterministic key->address expectations,
	- documented initial chunk-key/routing assumptions in `docs/FORMAT_NOTES.md`.
