# WbW-QGIS Release Changelog Draft

Date: 2026-05-24
Audience: QGIS plugin repository release notes

## Release Highlights (v2.0.2)

This release upgrades WbW-QGIS to the WbW-Py Next Gen backend.

Whitebox Next Gen is a complete rewrite and redesign of the Whitebox platform, moving from legacy monolithic architecture to a modern modular backend library foundation with equal first-class access through Python, R, and QGIS.

Key updates in this release:

- Next Gen backend integration:
  - WbW-QGIS now works with the WbW-Py Next Gen backend runtime.

- Major capability expansion:
  - Access to 700+ tools in the Next Gen platform.

- Broader geospatial format support:
  - Expanded raster, vector, and LiDAR interoperability in the Next Gen stack.
  - Stronger support for modern data workflows across mixed geospatial formats.

- Improved vector analysis:
  - Major vector-analysis improvements in Next Gen, including stronger topology and network/linear-referencing foundations.

- QGIS 4 support:
  - This plugin release now supports QGIS 4.x.
  - QGIS 3 support remains available in the validated compatibility window.

- Minor point release:
  - Improved runtime bootstrap reliability for macOS/QGIS environments by hardening external interpreter selection and fallback behavior.
  - Better handling of mixed legacy/Next Gen Python environments (clearer Next Gen runtime gating).

## Why This Matters

This is not a routine incremental update. It brings WbW-QGIS onto the new platform foundation so users can run higher-performance, local-first workflows with broader data interoperability and stronger vector-analysis depth.

## Issue Reporting and Support

- GitHub monorepo (please file plugin issues here):
  - https://github.com/jblindsay/whitebox_next_gen

- Product and platform information:
  - https://www.whiteboxgeo.com

- Support contact:
  - support@whiteboxgeo.com

## Suggested Short Version (If Marketplace Text Space Is Limited)

WbW-QGIS now runs on the WbW-Py Next Gen backend, a complete rewrite and redesign of the Whitebox platform. This release brings access to 700+ tools, broader raster/vector/LiDAR format support, major vector-analysis improvements, and new QGIS 4.x support. Please report issues in the Whitebox Next Gen monorepo: https://github.com/jblindsay/whitebox_next_gen. More information: https://www.whiteboxgeo.com. Support: support@whiteboxgeo.com.
