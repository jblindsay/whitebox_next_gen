# FORMAT_NOTES

Working notes for discovered format variations and assumptions.

- Initial scope: HDF5 superblock versions 0/1 and GZIP-compressed targeted layouts.
- HDF4/HDF-EOS2 notes will be tracked under dedicated sections as they are validated.

## Day 4 Notes (Chunk Lookup Validation Path)

- The current synthetic chunk index model uses the first coordinate component as the key for
	deterministic lookup tests.
- Internal-node routing uses the first key upper-bound match, falling back to the last record
	when the lookup key exceeds all explicit bounds.
- Node sibling pointers are parsed in headers but are not yet traversed in this stage.
- Lookup errors are intentionally explicit:
	- unknown dataset path -> `DatasetPathNotFound`
	- missing key in known index -> `ChunkAddressNotFound`

These are temporary scaffolding assumptions for Week 1 validation and will be tightened against
real GEDI/ICESat-2 chunk-layout fixtures in subsequent phases.
