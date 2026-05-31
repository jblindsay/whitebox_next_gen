# wbhdf

Scoped pure-Rust HDF container reader for Whitebox Next Gen.

## Scope

- Targeted decoding for known product families used by Whitebox workflows.
- HDF5-first implementation with bounded HDF4/HDF-EOS2 module support.
- GZIP-first filter support for initial milestone.

## Non-Goals

- Full general-purpose HDF4/HDF5 implementation.
- C-linked `libhdf5` dependency.
- Broad filter support beyond roadmap targets.
