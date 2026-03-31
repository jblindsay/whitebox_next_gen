# WKT Corpus Calibration

This folder contains a Phase 3 corpus used to calibrate adaptive WKT->EPSG identification.

## Files

- `manifest.csv`: expected lenient/strict outcomes for each sample.
- `non_na_manifest.csv`: expected outcomes for non-North-American legacy profile.
- `southern_manifest.csv`: expected outcomes for southern hemisphere + national-grid profile.
- `results_initial.csv`: first diagnostic run before Phase 3 tuning.
- `results_tuned.csv`: diagnostic run after tuning.
- `non_na_results_initial.csv`: first run for non-NA profile.
- `non_na_results_tuned.csv`: tuned run for non-NA profile.
- `southern_results_initial.csv`: first run for southern profile.
- `southern_results_tuned.csv`: tuned run for southern profile.
- `*.prj` / `*.wkt`: corpus samples.

## Run Diagnostics

From the `wbprojection` crate root:

- Single sample report:
  - `cargo run --example epsg_identify_report -- src/tests/data/wkt_corpus/legacy_csrs_utm17_a.prj`
- Batch report from manifest:
  - `cargo run --example epsg_identify_report_batch -- src/tests/data/wkt_corpus/manifest.csv src/tests/data/wkt_corpus/results_tuned.csv`
- Batch report from non-NA manifest:
  - `cargo run --example epsg_identify_report_batch -- src/tests/data/wkt_corpus/non_na_manifest.csv src/tests/data/wkt_corpus/non_na_results_tuned.csv`
- Batch report from southern manifest:
  - `cargo run --example epsg_identify_report_batch -- src/tests/data/wkt_corpus/southern_manifest.csv src/tests/data/wkt_corpus/southern_results_tuned.csv`

## CI-style Verification

Run one command to verify all discovered `*manifest.csv` files and fail on any mismatch:

- `cargo run --example epsg_identify_verify_manifests`

Optionally verify specific manifest(s):

- `cargo run --example epsg_identify_verify_manifests -- src/tests/data/wkt_corpus/non_na_manifest.csv`

## Add New Profile

Generate a new manifest template:

- `cargo run --example epsg_identify_manifest_template -- my_new_profile`

Generate using known sample files:

- `cargo run --example epsg_identify_manifest_template -- my_new_profile sample_a.prj sample_b.wkt`

Preview without writing:

- `cargo run --example epsg_identify_manifest_template -- my_new_profile sample_a.prj --dry-run`

## Regression Coverage

Manifest expectations are validated in test:

- `identify_wkt_all_manifests_in_corpus_match_expected`

Run with:

- `cargo test -q epsg_tests::identify_wkt_all_manifests_in_corpus_match_expected`
