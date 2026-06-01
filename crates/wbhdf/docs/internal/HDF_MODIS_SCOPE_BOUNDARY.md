# HDF MODIS Scope Boundary (Default-Enable Gate)

Date: 2026-06-01  
Status: Active boundary definition

## Purpose

This document defines the explicit MODIS/HDF4-HDF-EOS2 support boundary used for
default-integration decisions.

## In-Scope Product Families (Named)

Only the following MODIS land-product families are in scope for current staged support:

- MOD09 and MYD09 (surface reflectance)
- MOD11 and MYD11 (land surface temperature)
- MOD13 and MYD13 (vegetation index)

## Out-of-Scope for Current Default-Enable Decisions

The following are explicitly out of current scope:

- broad or unspecified MODIS catalog claims,
- non-land MODIS products not listed above,
- swath-specific payload-generalization beyond current staged decode/probe paths,
- full-scene deterministic extraction guarantees for all SDS variants.

## Operational Contract

When MODIS/HDF4-HDF-EOS2 companion support is enabled in default integration:

1. User-facing documentation and diagnostics must reference only the named in-scope families.
2. Unsupported product families must fail with explicit, user-actionable diagnostics.
3. No broad "MODIS-supported" wording should be used without the family qualifier.

## Validation Expectations

For each in-scope family, maintain at least one fixture-backed path with:

- metadata/path/shape/georef discovery,
- bounded payload-window probe or decode-attempt evidence,
- deterministic unsupported diagnostics where full extraction remains staged.

## Change Control

Any expansion beyond the named families above requires:

1. explicit update to this document,
2. matching roadmap evidence update,
3. fixture-backed validation evidence for the newly added family.
