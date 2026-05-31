# Backend Crates Release Plan – 2026-05-09

## Executive Summary

This document specifies the concrete release plan for publishing backend crates to crates.io following interop hardening (Phases A–C.1). The plan includes exact version numbers, changelog entries, and a dependency-safe publish sequence.

---

## 1. Release Scope & Version Strategy

All bumps are **patch-level** (semantic versioning); no API breaking changes.

| Crate | Current | New | Reason |
|-------|---------|-----|--------|
| **wbraster** | 0.1.4 | **0.1.5** | GeoPackage raster producer compatibility (R09 fix) |
| **wbvector** | 0.1.2 | **0.1.3** | FlatGeobuf indexed interop hardening (V04 fix) |
| **wbgeotiff** | 0.1.2 | 0.1.2 | Reaffirm Phase A validation; no code changes |
| **wbprojection** | 0.1.1 | 0.1.1 | Reaffirm Phase A validation; no code changes |
| **wblidar** | 0.1.1 | 0.1.1 | Reaffirm Phase B validation; Python decoupling confirmed |

---

## 2. Publish Sequence (Dependency-Safe Order)

### Phase 1: Immediate (2026-05-09)

#### Step 1: Publish **wbgeotiff 0.1.2** (reaffirmation)
- **Command:** `cd crates/wbgeotiff && cargo publish`
- **Dependencies:** None (upstream)
- **Wait:** ~2 min for crates.io indexing
- **Reason:** Ensures downstream consumers (wbraster) can resolve dependency

#### Step 2: Publish **wbraster 0.1.5** (patch)
- **Command:** `cd crates/wbraster && cargo publish`
- **Dependencies:** wbgeotiff 0.1.2 (just published)
- **Wait:** ~2 min for crates.io indexing
- **Changes:** GeoPackage raster compatibility (Interop C.1 R09)

#### Step 3: Publish **wbvector 0.1.3** (patch)
- **Command:** `cd crates/wbvector && cargo publish`
- **Dependencies:** wbprojection 0.1.0+ (already on crates.io)
- **Wait:** ~2 min for crates.io indexing
- **Changes:** FlatGeobuf indexed interop hardening (Interop B V04)

#### Step 4: Publish **wbprojection 0.1.1** (reaffirmation)
- **Command:** `cd crates/wbprojection && cargo publish` (or `--dry-run` to skip if already published)
- **Dependencies:** None
- **Reason:** Affirm Phase A validation

#### Step 5: Publish **wblidar 0.1.1** (reaffirmation)
- **Command:** `cd crates/wblidar && cargo publish` (or `--dry-run` to skip if already published)
- **Dependencies:** wbprojection 0.1.1 (just published or reaffirmed)
- **Reason:** Affirm Phase B validation; Python binding decoupling complete

### Phase 2: Follow-up (Optional, ~1 week later)

#### Step 6: Publish **wbtopology 0.1.2** (if Phase C topology corpus validates)
- **Command:** `cd crates/wbtopology && cargo publish`
- **Dependencies:** wbvector 0.1.3 (already published in Phase 1)
- **Changes:** Phase C topology stress corpus validation (14/14 fixtures passing)

---

## 3. Pre-Publish Checklist

Before running `cargo publish` for each crate:

- [ ] **wbraster 0.1.5:**
  - [ ] Verify version in Cargo.toml: `0.1.5`
  - [ ] Verify CHANGELOG.md heading: `## [0.1.5] – 2026-05-09`
  - [ ] Run `cargo check -p wbraster`
  - [ ] Run `cargo test -p wbraster` (no panics)
  - [ ] Verify `publish = false` is NOT set in Cargo.toml
  - [ ] Run `cargo publish --dry-run` for dependency resolution check

- [ ] **wbvector 0.1.3:**
  - [ ] Verify version in Cargo.toml: `0.1.3`
  - [ ] Verify CHANGELOG.md heading: `## [0.1.3] – 2026-05-09`
  - [ ] Run `cargo check -p wbvector`
  - [ ] Run `cargo test -p wbvector` (no panics)
  - [ ] Verify `publish = false` is NOT set in Cargo.toml
  - [ ] Run `cargo publish --dry-run` for dependency resolution check

- [ ] **wbgeotiff 0.1.2:**
  - [ ] Verify CHANGELOG.md contains reaffirmation entry
  - [ ] Run `cargo check -p wbgeotiff`
  - [ ] Run `cargo publish --dry-run` (optional; can skip if already published)

- [ ] **wbprojection 0.1.1:**
  - [ ] Verify CHANGELOG.md contains reaffirmation entry
  - [ ] Run `cargo check -p wbprojection`
  - [ ] Run `cargo publish --dry-run` (optional; can skip if already published)

- [ ] **wblidar 0.1.1:**
  - [ ] Verify CHANGELOG.md contains reaffirmation entry
  - [ ] Run `cargo check -p wblidar`
  - [ ] Run `cargo publish --dry-run` (optional; can skip if already published)

---

## 4. Changelog Entries (Finalized)

### wbraster 0.1.5

```markdown
## [0.1.5] – 2026-05-09

### Fixed
- GeoPackage raster producer compatibility: accept GDAL-registered raster content with `data_type = "2d-gridded-coverage"` in addition to plain `"tiles"` (Interop Phase C.1 R09).
- Fallback tile-table discovery from `gpkg_tile_matrix_set` when `gpkg_contents` metadata is unavailable, improving compatibility with diverse GeoPackage raster producers.

### Testing
- Interop Phase C.1 R09 case (GeoPackage raster roundtrip) now passes; full suite 6/6 passing.
```

### wbvector 0.1.3

```markdown
## [0.1.3] – 2026-05-09

### Changed
- FlatGeobuf indexed interoperability restored via internal native reader/writer enhancements (no external flatgeobuf crate dependency).
- Lean dependency compliance enforced: FlatGeobuf indexed operations use internal packed-index logic.

### Fixed
- V04 FlatGeobuf indexed read/write now deterministic and reliable under all interop test conditions.
- Telemetry for indexed parse decisions remains env-gated for operational insight.

### Testing
- Interop Phase B V04 case passes; full Phase B 15/15 passing; Phase C.1 vector cases (V05–V07) all passing.
```

### wbgeotiff 0.1.2 (Reaffirmation)

```markdown
### Testing
- Interop Phase A GeoTIFF conformance: 33/33 passing across 11 CRS families.
- Forward/inverse tolerance validation confirmed for all tested CRS.

*Note: Version 0.1.2 is being published as-is as part of the interop release milestone (2026-05-09) to ensure downstream dependency resolution for wbraster 0.1.5.*
```

### wbprojection 0.1.1 (Reaffirmation)

```markdown
### Testing
- Interop Phase A projection conformance: 33/33 passing across 11 CRS families.
- Forward/inverse tolerance validation confirmed for all tested CRS.

*Note: Version 0.1.1 is being published as-is as part of the interop release milestone (2026-05-09) to affirm Phase A validation.*
```

### wblidar 0.1.1 (Reaffirmation)

```markdown
### Testing
- Interop Phase B LiDAR cases: L01 (LAS 1.4), L02 (LAZ compressed), L03 (COPC) all passing.
- Python binding architecture decoupled from WbW-R; native wblidar write path now fully backend-native.

*Note: Version 0.1.1 is being published as-is as part of the interop release milestone (2026-05-09) to affirm Phase B validation.*
```

---

## 5. Risk Mitigation

### Dependency Ordering Risk
- **Mitigated by:** Publishing wbgeotiff *first* to ensure wbraster resolution.
- **Fallback:** If wbraster publish fails, check crates.io index delay; retry after 5 min.

### Yanking Risk
- **If a publish fails mid-sequence:**
  ```bash
  cargo yank --vers 0.1.5 -p wbraster
  ```
- Re-apply any fixes and retry publish.

### Breaking Changes Risk
- **Minimal:** All bumps are patch-level; no API deletions or renames.
- **Verification:** Reviewed all CHANGELOG entries; no breaking changes identified.

### Timing & Index Refresh Risk
- **Stagger publishes by ~2 min** to allow crates.io index refresh between each.
- **Verification:** Check crates.io manually after each publish:
  - Navigate to `https://crates.io/crates/<crate>/versions`
  - Confirm new version appears within 2–5 minutes.

### Python/Go Binding Compatibility Risk
- **wbw_python decoupling:** Confirmed native backend shim in place; no WbW-R dependency in wbw_python/Cargo.toml.
- **No publish blockers:** Bindings are integration-only; backend crates can publish independently.

---

## 6. Post-Publish Verification

After each successful `cargo publish`:

1. **Verify on crates.io** (within 2–5 minutes):
   ```
   https://crates.io/crates/<crate>/versions
   ```

2. **Test downstream consumer builds** (internal validation):
   ```bash
   cd crates/wbtools_oss
   cargo check  # Verifies new wbraster/wbvector versions resolve
   ```

3. **Tag git commits** for release tracking:
   ```bash
   git tag -a wbraster-0.1.5 -m "Release wbraster 0.1.5: GeoPackage raster compat"
   git push origin wbraster-0.1.5
   ```

4. **Document publish commit SHA** in release notes (GitHub Release):
   - wbraster 0.1.5: `<commit-sha>`
   - wbvector 0.1.3: `<commit-sha>`
   - wbgeotiff 0.1.2: `<commit-sha>` (if first publish)
   - wbprojection 0.1.1: `<commit-sha>` (if first publish)
   - wblidar 0.1.1: `<commit-sha>` (if first publish)

---

## 7. Implementation Timeline

| Stage | Timeline | Action |
|-------|----------|--------|
| **Dry Run** | Now | Run all `cargo publish --dry-run` checks |
| **Publish Phase 1** | 2026-05-09 | Execute steps 1–5 (Phase 1 sequence) |
| **Verification** | 2026-05-09 (post-publish) | Confirm crates.io availability; tag commits |
| **Phase 2 Planning** | ~2026-05-16 | Assess wbtopology topology corpus results |
| **Publish Phase 2** | Optional, ~1 week | Publish wbtopology 0.1.2 if warranted |

---

## 8. Rollback Plan

If a critical issue is discovered post-publish:

1. **Yank the problematic version:**
   ```bash
   cargo yank --vers 0.1.5 -p wbraster
   ```

2. **Fix the issue** in the codebase.

3. **Republish with incremented patch:**
   ```bash
   # Update Cargo.toml: 0.1.5 → 0.1.6
   # Update CHANGELOG.md with fix entry
   cargo publish
   ```

4. **Notify downstream consumers** via GitHub Release notes.

---

## 9. Execution Approval Gate

✅ **Release plan approved and ready for publish (2026-05-09).**

**Awaiting user confirmation before running Phase 1 sequence.**

To proceed: 
```bash
cd /path/to/whitebox_next_gen
# Run Phase 1 publish sequence (steps 1–5 above)
```
