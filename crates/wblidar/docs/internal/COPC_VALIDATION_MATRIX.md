# COPC External Validation Matrix

This matrix defines the minimum milestone-4 interoperability coverage for
writer-produced COPC outputs.

## Required Dimensions

- Producer: `wblidar` writer outputs (`las_to_copc`, fixture pack)
- Hierarchy shape: `single-page`, `paged`, `mixed-root`
- Point format: `PDRF6`, `PDRF7`, `PDRF8`
- CRS mode: `none`, `EPSG-only`, `WKT+EPSG`
- Attribute mode: base-only, RGB, RGB+NIR (where representable)

## Minimum Scenario Set

| Scenario ID | Source | Hierarchy | PDRF | CRS | Attributes | External Checks |
|---|---|---|---|---|---|---|
| M4-S1 | Fixture pack | single-page | 6 | none | base-only | LAStools + PDAL + validate.copc.io |
| M4-S2 | Fixture pack | single-page | 7 | none | RGB | LAStools + PDAL + validate.copc.io |
| M4-S3 | Fixture pack | single-page | 8 | none | RGB+NIR | LAStools + PDAL + validate.copc.io |
| M4-S4 | Real Carlos LAS | paged | 6 | source CRS | base-only | LAStools + PDAL |
| M4-S5 | Synthetic large hierarchy | mixed-root | 6/7/8 | varied | varied | LAStools + PDAL |

## Pass Criteria

- External decoders complete without chunk EOF failures.
- Reported point counts match COPC header point counts.
- No fatal parse/decompression errors.
- validate.copc.io reports structural pass for fixture scenarios.

## Notes

- validate.copc.io is currently a manual step; record URL, timestamp, and result.
- If a tool is unavailable on a machine, mark that check as `skipped` with reason.
