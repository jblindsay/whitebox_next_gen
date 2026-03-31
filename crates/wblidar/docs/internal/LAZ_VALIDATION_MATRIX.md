# LAZ External Validation Matrix

This matrix tracks interoperability status for writer-produced standards-compliant
LAZ outputs.

## Scope

- Producer: wblidar standards writer (`LazWriterConfig::standards_compliant=true`)
- Families:
  - Point10 pointwise: PDRF0/1/2/3
  - Point14 layered: PDRF6/7/8
- Validation paths:
  - Internal read-back via `LazReader`
  - External consumers (`lasinfo`, `pdal info`) via
    `tests/standards_external_validation.rs`

## Report Schema

JSON report artifact (`schema_version=1`) includes:

- `tools`: tool availability booleans (`lasinfo_available`, `pdal_available`)
- `summary`: executed/failed counts + run status
  - also includes `generated_profiles` and `fixture_profiles`
- `profiles[]` entries:
  - `source` (`generated` or `fixture`)
  - `pdrf`
  - `extra_bytes_per_point`
  - `expected_point_count`
  - `internal_read_count`
  - `lasinfo`, `pdal_info`
  - `status`
  - `errors[]`

Set report output path with:

- `WBLIDAR_LAZ_INTEROP_REPORT=/path/to/report.json`

Add real-world fixture coverage with:

- `WBLIDAR_LAZ_INTEROP_FIXTURE_DIR=/path/to/laz/fixtures`
- `WBLIDAR_LAZ_INTEROP_MIN_FIXTURE_PROFILES=<non-negative integer>`

When set, the harness recursively discovers `*.laz` files under the fixture
directory and validates them alongside generated synthetic standards profiles.
If `WBLIDAR_LAZ_INTEROP_MIN_FIXTURE_PROFILES` is set, runs fail when the
fixture profile count is lower than the required threshold.

In GitHub Actions, optional fixture bundles can be supplied through
workflow dispatch input `fixture_archive_url` in
`.github/workflows/wblidar-interop.yml`.
Fixture coverage gating can be enforced with workflow dispatch input
`min_fixture_profiles`.

Each CI run now also renders a markdown summary from the JSON report using
`crates/wblidar/scripts/laz_interop_report_summary.py` and uploads it as
`wblidar-interop-summary` artifact while appending the same content to the
workflow step summary.

## Current Run Snapshot

Source: strict ignored run on 2026-03-29 (`schema_version=1`, strict mode).

| PDRF | Extra Bytes | Internal Read-back | lasinfo | pdal info | Status |
|---|---:|---:|---|---|---|
| 0 | 0 | 3/3 | skipped | ok | ok |
| 1 | 1 | 3/3 | skipped | ok | ok |
| 2 | 1 | 3/3 | skipped | ok | ok |
| 3 | 2 | 3/3 | skipped | ok | ok |
| 6 | 0 | 3/3 | skipped | ok | ok |
| 7 | 0 | 3/3 | skipped | ok | ok |
| 8 | 2 | 3/3 | skipped | ok | ok |

Environment status in that run:

- `pdal_available=true`
- `lasinfo_available=false`

## Remaining Validation Gaps

- Populate `WBLIDAR_LAZ_INTEROP_FIXTURE_DIR` in CI or validation hosts with
  representative real-world datasets (especially PDRF8).
- Add LAStools-backed rows once `lasinfo` is available in CI or local validation hosts.
- Expand beyond generated synthetic points to real-world fixture coverage,
  especially PDRF8 datasets with representative RGB+NIR+extra-bytes payloads.
- Keep this matrix in sync with JSON artifacts uploaded by
  `.github/workflows/wblidar-interop.yml`.
