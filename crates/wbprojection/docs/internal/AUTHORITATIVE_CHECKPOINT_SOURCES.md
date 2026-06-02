# Authoritative Checkpoint Sources (Discovery Notes)

Date: 2026-06-02

## Goal

Identify externally authoritative checkpoint sources to anchor conformance tests.

## Findings

1. NRCan TRX tool is publicly accessible and returns deterministic numeric outputs for single-point calculations.
   - URL: https://webapp.csrs-scrs.nrcan-rncan.gc.ca/geod/tools-outils/trx.php
   - Supports NAD83(CSRS) and ITRF realization transforms with epoch handling.
   - Public sample batch input files are available from:
     - https://webapp.csrs-scrs.nrcan-rncan.gc.ca/geod/tools-outils/descriptions/sample_batch_files/sample-geo.csv
     - https://webapp.csrs-scrs.nrcan-rncan.gc.ca/geod/tools-outils/descriptions/sample_batch_files/sample-cart.csv
     - https://webapp.csrs-scrs.nrcan-rncan.gc.ca/geod/tools-outils/descriptions/sample_batch_files/sample-trx-utm.csv
     - https://webapp.csrs-scrs.nrcan-rncan.gc.ca/geod/tools-outils/descriptions/sample_batch_files/sample-trx-mtm.csv

2. NRCan coordinate-transformation grid download page currently requires sign-in.
   - URL: https://webapp.csrs-scrs.nrcan-rncan.gc.ca/geod/data-donnees/transformations.php
   - This limits direct automated acquisition of some transformation assets.

3. EPSG site scraping was not productive via available tooling in this session for direct machine-readable checkpoints.

4. EPSG operation page confirms operation metadata for code 10715 but does not publish sample input/output checkpoints.
   - URL: https://epsg.io/10715
   - Confirmed fields include method (EPSG:1114) and velocity grid file name (`NAD83v80VG.gvb`).
   - No benchmark coordinate pairs were found on this page.

5. Direct export endpoints for operation 10715 were not fetchable via current tooling in this session.
   - Attempted: https://epsg.io/10715.json
   - Attempted: https://epsg.io/10715.wkt2
   - Result: HTTP 404 in this environment for both URLs.

6. NRCan transformation asset page remains sign-in gated for downloads.
   - URL: https://webapp.csrs-scrs.nrcan-rncan.gc.ca/geod/data-donnees/transformations.php
   - Public page confirms login requirement.

7. Authenticated NRCan Coordinate Transformations UI did not expose direct realization selection for
   `NAD83(CSRS) v3 -> NAD83(CSRS) v8` in the web workflow explored during this session.
   - The UI supports epoch propagation within `NAD83(CSRS)` with interpolated velocities.
   - It did not provide a visible `Origin = v3` / `Destination = v8` path in Single Calculation.
   - Based on this session, direct operation-10715 checkpoint capture appears unavailable from the web UI.

8. NRCan PPPdirect desktop software appears to expose NAD83(CSRS) epoch options (1997, 2002, 2010)
   in its processing configuration UI.
   - Observed from user-provided NRCan page screenshot in this session.
   - This path is currently Windows-only in practice.
    - PPPdirect is operationally equivalent to the date-based CSRS web workflow for realization evidence:
       it exposes epoch selections (1997/2002/2010) but not explicit realization labels (`v3`/`v4`/`v8`).
    - Therefore PPPdirect is useful for reproducible epoch-routed checkpoints, but it is not expected
       to provide strict explicit-label pair metadata unavailable in NRCan operational interfaces.

## Captured Checkpoints

A first authoritative fixture (TRX, NAD83(CSRS) -> ITRF2014, epoch 2010-01-01) was captured and stored at:

- src/tests/data/authoritative/nrcan_trx_nad83csrs_to_itrf2014_epoch2010_checkpoints.csv

A second authoritative fixture (authenticated NRCan Coordinate Transformations tool,
NAD83(CSRS) epoch propagation from 2010-01-01 to 2020-01-01 with interpolated velocities,
Longitude Positive West) was captured from user-provided screenshots and stored at:

- src/tests/data/authoritative/nrcan_nad83csrs_epoch_propagation_2010_to_2020_checkpoints.csv

A third captured TRX batch run (user-provided exact run settings and output file) was preserved at:

- src/tests/data/authoritative/nrcan_trx_nad83csrs_epoch_2002_to_2010_guelph_vancouver_sample2.csv

Run metadata (TRX batch, run id `TRX_2026-06-02_43861`):

- Epoch Transformation: enabled
- Origin Reference Frame: `NAD83(CSRS)`
- Origin Epoch: `2002-01-01`
- Destination Reference Frame: `NAD83(CSRS)`
- Destination Coordinates: `Geographic`
- Destination Epoch: `2010-01-01`
- Input batch file: `nrcan_sample_geo_guelph_vancouver_sample2.csv`

Evidence classification for this run:

- Strong date-routed CSRS propagation evidence (v4-anchor-date to v8-anchor-date behavior).
- Not explicit-label pair evidence by itself because the exported CSV does not include explicit
   source/target realization labels (e.g., `v4`/`v8`) or an operation code.
- Combined with EPSG anchor-epoch metadata and preserved run settings, this fixture is accepted
   as authoritative-inference support for v4 -> v8 activation in this prototype.

Fixture ingestion coverage is now implemented in:

- src/tests/authoritative_tests.rs

The ingestion harness validates all currently captured NRCan fixture formats used in this rollout,
including TRX-style transformed outputs and date-routed CSRS epoch-propagation captures.

## Important Scope Note

The captured TRX checkpoints are authoritative for the TRX workflow above, but they are not direct checkpoints for EPSG operation 10715 (CSRS v3 UTM -> CSRS v8 UTM).

For operation-10715-specific authoritative checkpoints, we still need one of:

1. A public source explicitly publishing operation-10715 sample input/output pairs, or
2. User-provided benchmark points from an authoritative workflow/report, or
3. Exported results from an authoritative service/tool run where source/target are explicitly 223xx -> 228xx with method metadata retained.

Given current NRCan operational tooling (web + PPPdirect), strict explicit-label realization metadata may be
practically unobtainable through normal public workflows. In this repository, that is treated as an evidence ceiling,
not an implementation blocker.

## Immediate Next Target

Promote operation-10715 conformance from "internal consistency" to "external authoritative"
by ingesting at least three published or traceable checkpoints for a same-zone 223xx -> 228xx
pair, with source, destination, epoch, and method metadata preserved.

Status: not blocked for prototype progress. Explicit-label datasets remain preferred, but
authoritative-inference activation is permitted when the authoritative source is date-only.

Practical CSRS usability target:

- Maintain operationally usable preferred mappings for CSRS workflows under the date-routed
   NRCan evidence ceiling using mathematically-driven matched-zone activation.

Working policy conclusion:

- Date-routed NRCan outputs with preserved run metadata are accepted as authoritative-inference evidence.
- Lack of literal realization labels in NRCan interfaces is not treated as a project failure condition.

Reverse-direction policy status (current code):

- Matched-zone reverse preferred-operation corridors are enabled under the same
   CSRS realization-pair routing rule used for forward corridors.
- Preferred-operation APIs no longer block reverse corridors behind pending
   activation errors.

A ready-to-fill template for this capture is available at:

- src/tests/data/authoritative/op10715_csrs_v3_to_v8_checkpoints_template.csv

Additional ready-to-fill templates for active/planned forward pair activations:

- src/tests/data/authoritative/csrs_v4_to_v8_checkpoints_template.csv
- src/tests/data/authoritative/csrs_v5_to_v8_checkpoints_template.csv
- src/tests/data/authoritative/csrs_v6_to_v8_checkpoints_template.csv
- src/tests/data/authoritative/csrs_v7_to_v8_checkpoints_template.csv

Reverse-direction template (added):

- `src/tests/data/authoritative/csrs_v8_to_v3_checkpoints_template.csv`
- `src/tests/data/authoritative/csrs_v8_to_v4_checkpoints_template.csv`
- `src/tests/data/authoritative/csrs_v8_to_v5_checkpoints_template.csv`
- `src/tests/data/authoritative/csrs_v8_to_v6_checkpoints_template.csv`
- `src/tests/data/authoritative/csrs_v8_to_v7_checkpoints_template.csv`
   for ongoing reverse-corridor conformance evidence capture.

Convenience batch-input CSV for NRCan geographic batch mode (Guelph + two additional points):

- src/tests/data/authoritative/nrcan_sample_geo_guelph_vancouver_sample2.csv

Template harness behavior:

- `src/tests/authoritative_tests.rs` now parses all CSRS template fixtures currently tracked (forward and reverse).
- Empty templates are valid while external checkpoint acquisition is still in progress.
- Once rows are added, tests validate schema and numeric/zone constraints automatically.

Template inventory (current):

Forward corridors:

1. v3 -> v8: src/tests/data/authoritative/op10715_csrs_v3_to_v8_checkpoints_template.csv
2. v4 -> v8: src/tests/data/authoritative/csrs_v4_to_v8_checkpoints_template.csv
3. v5 -> v8: src/tests/data/authoritative/csrs_v5_to_v8_checkpoints_template.csv
4. v6 -> v8: src/tests/data/authoritative/csrs_v6_to_v8_checkpoints_template.csv
5. v7 -> v8: src/tests/data/authoritative/csrs_v7_to_v8_checkpoints_template.csv

Reverse corridors:

1. v8 -> v3: src/tests/data/authoritative/csrs_v8_to_v3_checkpoints_template.csv
2. v8 -> v4: src/tests/data/authoritative/csrs_v8_to_v4_checkpoints_template.csv
3. v8 -> v5: src/tests/data/authoritative/csrs_v8_to_v5_checkpoints_template.csv
4. v8 -> v6: src/tests/data/authoritative/csrs_v8_to_v6_checkpoints_template.csv
5. v8 -> v7: src/tests/data/authoritative/csrs_v8_to_v7_checkpoints_template.csv

US/Europe prioritized expansion templates (new scaffold):

1. US NSRS2007 -> NAD83(2011):
   src/tests/data/authoritative/us_nsrs2007_to_nad83_2011_checkpoints_template.csv
2. Europe ETRS89 realization corridors:
   src/tests/data/authoritative/europe_etrs89_realization_checkpoints_template.csv

Notes for these new templates:

- They use the same schema/parse harness as CSRS corridor templates.
- Empty templates are valid until authoritative checkpoints are captured.
- Immediate priority order for fill-in is US first, then Europe.

Phase-1 strict fill rules (enforced by tests when rows are present):

- US template allowlist corridors:
   - 3582 -> 6487
   - 3600 -> 6568
- Europe template allowlist corridors:
   - 4258 -> 4258
   - 25832 -> 3035
- For both templates, each filled row must include `operation_code`.
- For both templates, each filled row must keep `epoch_decimal_year` in [1980, 2100].
- For both templates, each filled row `source_reference` must:
   - Be non-empty and not a placeholder token (`tbd`, `todo`, `pending`, `n/a`, etc.).
   - Include either a namespaced code (`EPSG:...`) or a URL-style path.

Runtime scaffolding note:

- `wbprojection` now exposes phase-1 US and Europe support snapshots for these
   seeded corridor pairs.
- Snapshot entries are now `Active` for broad mathematical rollout.
- Preferred operation code remains optional (`None`) until corresponding
   authoritative checkpoint rows are filled and validated.

## EPSG Anchor-Epoch Notes (Realization Metadata)

Observed in EPSG WKT content used during this rollout:

1. `NAD83(CSRS)v3` anchor epoch: 1997.
2. `NAD83(CSRS)v4` anchor epoch: 2002.
3. `NAD83(CSRS)v8` anchor epoch: 2010.

These anchor epochs are useful context, but date-only tool outputs still require explicit
source/target realization metadata before they can be treated as strict (non-inferred)
pair-activation evidence.

For a plain-language step-by-step on collecting pair-activation evidence when a source is date-based only, see:

- docs/internal/CSRS_PAIR_CHECKPOINT_ACQUISITION_PLAYBOOK.md
