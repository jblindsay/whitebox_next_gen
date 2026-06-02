# CSRS Pair Checkpoint Acquisition Playbook

Date: 2026-06-02
Scope: How to obtain usable checkpoints for CSRS realization-pair activation across currently tracked forward and reverse corridors.

## Short Answer

If a tool only lets you choose dates and does not let you choose explicit realizations
(for example `NAD83(CSRS)v4` and `NAD83(CSRS)v8`), then its outputs are valid for
epoch-propagation checks but are not sufficient proof of a specific realization pair.

You are not doing anything wrong. This is a source limitation, not a workflow mistake.

Current project stance:

1. NRCan operational interfaces used in this project (web TRX and PPPdirect) are date-oriented.
2. We do not assume strict explicit-label realization metadata will appear in normal outputs.
3. We proceed using documented authoritative-inference policy rather than waiting for impossible labels.

## Realization Anchor Epochs (What We Can Use)

From EPSG WKT metadata reviewed in this project:

1. `NAD83(CSRS)v3` has `ANCHOREPOCH[1997]` (seen in EPSG operation 10715 WKT2 source CRS block).
2. `NAD83(CSRS)v4` has `ANCHOREPOCH[2002]` (EPSG:8244 WKT2).
3. `NAD83(CSRS)v8` has `ANCHOREPOCH[2010]` (seen in EPSG operation 10715 WKT2 target CRS block).

Important distinction:

1. Anchor epoch belongs to the frame definition (realization metadata).
2. The webapp date fields are coordinate/observation epochs for propagation.
3. These are related but not interchangeable evidence.

## Can We Use v4=2002 And v8=2010 Dates?

Yes, as a practical heuristic for exploratory checks:

1. Set origin epoch near source realization anchor epoch (for v4, around 2002).
2. Set destination epoch near target realization anchor epoch (for v8, around 2010).

But this still does not by itself prove a specific realization pair unless source/target
realizations are explicitly stated by the authoritative output.

## What Counts As Pair-Activation Evidence

To activate a pair like v4 -> v8 in code, we need at least one of:

1. Explicit operation metadata tying that pair to a specific operation code.
2. Authoritative checkpoints whose metadata clearly states source realization and target realization.
3. Authoritative export/report where source/target CRS EPSG codes are explicit (for example `22417 -> 22817`).
4. Authoritative-inference package: date-routed authoritative outputs plus realization anchor-epoch
  metadata that uniquely supports one pair interpretation in the implemented corridor model.

Reverse-direction note:

1. Reverse corridors (for example `v8 -> v4`) require their own operation preference evidence.
2. Do not infer reverse preferred-operation activation solely from forward corridor evidence.
3. Keep reverse pairs pending until reverse operation mapping is explicitly documented and tested.

## What the Date-Only NRCan UI Can And Cannot Do

Can do:

1. Provide authoritative epoch-propagation checkpoints within a published frame label (`NAD83(CSRS)`).
2. Support tests that validate epoch handling behavior.

Cannot do (when realization selectors are absent):

1. Prove that output corresponds to one specific realization pair (for example v4 -> v8).
2. Provide traceable source/target EPSG realization codes required for pair activation.

## Practical Paths Forward (Ranked)

1. Best path: authoritative pair-specific export
- Obtain a source that explicitly reports `source_crs_epsg` and `target_crs_epsg` for the pair.
- Fill rows in:
  - `src/tests/data/authoritative/csrs_v4_to_v8_checkpoints_template.csv`

2. Next best: authoritative operation mapping + limited checkpoints
- Get operation code mapping for v4 -> v8 from authoritative metadata.
- Add at least 3 checkpoints with explicit pair metadata.

3. Practical authoritative path when web UI is limited: PPPdirect desktop (Windows)
- Use NRCan PPPdirect with `NAD83(CSRS)` and fixed epoch selections (1997, 2002, 2010) as available.
- Preserve full run metadata: software version, processing mode, selected reference frame, selected epoch option,
  input coordinates, and exported outputs.
- Treat outputs as Tier A-Inferred evidence when realization labels are not emitted and epoch options
  align with supported realization anchors.

PPPdirect viability check (fast go/no-go):

1. Run one small known dataset twice with identical settings except epoch option (for example 2002 vs 2010).
2. Confirm PPPdirect exports numeric coordinates for both runs (not only reports/quality summaries).
3. Confirm outputs are machine-readable and reproducible (CSV/text export with station identifiers).
4. Record exact run metadata (software version, frame choice, epoch option, mode).

Go decision:
- If all four checks pass, PPPdirect is a valid authoritative-inference source for this project.

No-go decision:
- If PPPdirect only outputs PPP quality diagnostics or non-comparable products, do not use it for CSRS pair checkpoints; continue with TRX/date-routed evidence and inferred-activation policy.

4. Fallback: keep pair pending
- Keep unresolved pairs marked pending in code/docs.
- Continue using date-based outputs for epoch-propagation conformance.

Reverse-corridor template path:

- `src/tests/data/authoritative/csrs_v8_to_v3_checkpoints_template.csv`
- `src/tests/data/authoritative/csrs_v8_to_v4_checkpoints_template.csv`
- `src/tests/data/authoritative/csrs_v8_to_v5_checkpoints_template.csv`
- `src/tests/data/authoritative/csrs_v8_to_v6_checkpoints_template.csv`
- `src/tests/data/authoritative/csrs_v8_to_v7_checkpoints_template.csv`

## Tracked Template Inventory (Current)

Forward corridors (active policy):

1. v3 -> v8: `src/tests/data/authoritative/op10715_csrs_v3_to_v8_checkpoints_template.csv`
2. v4 -> v8: `src/tests/data/authoritative/csrs_v4_to_v8_checkpoints_template.csv`
3. v5 -> v8: `src/tests/data/authoritative/csrs_v5_to_v8_checkpoints_template.csv`
4. v6 -> v8: `src/tests/data/authoritative/csrs_v6_to_v8_checkpoints_template.csv`
5. v7 -> v8: `src/tests/data/authoritative/csrs_v7_to_v8_checkpoints_template.csv`

Reverse corridors (pending policy):

1. v8 -> v3: `src/tests/data/authoritative/csrs_v8_to_v3_checkpoints_template.csv`
2. v8 -> v4: `src/tests/data/authoritative/csrs_v8_to_v4_checkpoints_template.csv`
3. v8 -> v5: `src/tests/data/authoritative/csrs_v8_to_v5_checkpoints_template.csv`
4. v8 -> v6: `src/tests/data/authoritative/csrs_v8_to_v6_checkpoints_template.csv`
5. v8 -> v7: `src/tests/data/authoritative/csrs_v8_to_v7_checkpoints_template.csv`

## Minimal Data Package To Send Back

If you get any usable source, provide rows with:

1. station
2. source_crs_epsg (pair-specific UTM realization code, same-zone source)
3. target_crs_epsg (pair-specific UTM realization code, same-zone target)
4. operation_code (if known)
5. epoch_decimal_year (or date we can convert)
6. input_x_m, input_y_m, input_z_m
7. output_x_m, output_y_m, output_z_m
8. source_reference (URL/report identifier)

These fields match the template header exactly.

## Decision Rule Used In This Repo

1. If realization pair is explicit and traceable -> eligible for activation.
2. If realization pair is not explicit but authoritative-inference criteria are met -> eligible for documented inferred activation.
3. Otherwise keep as pending; use outputs for epoch-propagation validation only.

Reverse-corridor extension:

1. Forward and reverse directions are evaluated independently for preferred-operation activation.
2. If reverse mapping evidence is missing, reverse stays pending even when forward is active.
3. Pending reverse corridors should produce explicit pending-activation errors in preferred-operation APIs.

## Practical Proceed-Now Policy (When UI Is Date-Only)

When authoritative tools expose `NAD83(CSRS)` plus dates but do not expose explicit realization labels,
we proceed using two evidence tiers instead of blocking all progress.

Tier A: Pair-Activation Grade (strict)

1. Requires explicit source/target realization metadata or explicit EPSG CRS pair.
2. Can be used to flip a corridor from Pending to Active.

Tier B: Date-Routed Authoritative Grade (provisional)

1. Requires reproducible authoritative outputs with full run settings preserved (tool, date, frame labels, epochs, input file, output file).
2. Supports epoch-routing behavior validation and regression tests.
3. Does not permit pair activation by itself.

Tier A-Inferred: Pair-Activation Grade (authoritative inference)

1. Used only when the authoritative source is date-only and cannot emit realization labels.
2. Requires reproducible authoritative outputs, preserved run metadata, and matching EPSG anchor-epoch context.
3. Requires a written justification in internal docs and test coverage for the activated corridors.

Why this is still scientifically useful:

1. The numeric outputs are authoritative for the published workflow.
2. Anchor-epoch context (for example v4 around 2002 and v8 around 2010) provides a defensible interpretation layer.
3. We avoid overclaiming by preserving the strict activation gate.

Operationally, this means:

1. Keep unresolved pairs Pending in activation tables.
2. Continue collecting and storing date-routed checkpoints as Tier B evidence.
3. Where justified, promote a pair to Tier A-Inferred with explicit documentation and tests.
