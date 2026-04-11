# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]

### Added
- Added an internal Phase 1 execution checklist for WbW-Py usability/capability improvements:
  - `docs/internal/wbw_py_phase1_execution_checklist.md`

### Changed
- Started Phase 1 documentation cleanup by adding a "Preferred API conventions" section to `README.md`.
- Improved stub guidance in `whitebox_workflows.pyi` to clarify preferred canonical methods (`metadata()` over legacy alias paths) and the topology utility-vs-tools namespace split.
- Updated internal planning docs/checklists to reflect two constraints: pre-release
  API clarity can take priority over backward compatibility, and significant
  WbW-Py API changes should include explicit WbW-R parity decisions.