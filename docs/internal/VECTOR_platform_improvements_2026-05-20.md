# VECTOR Platform Improvements: Roadmap and Recommendations

**Date:** May 20, 2026
**Author:** Platform Technical Audit

---

## Executive Summary

This document summarizes recommended improvements to Whitebox's vector data capabilities, with a focus on:
- Advanced attribute manipulation (Field Calculator)
- Output clarity for tools generating CSVs (e.g., List Unique Values)
- General platform robustness for vector workflows

These recommendations are based on recent audits, user feedback, and showcase development experience.

---

## 1. Field Calculator: Roadmap for Advanced Attribute Manipulation

### Current State
- The Field Calculator supports basic expressions and field updates.
- Lacks robust SQL-like logic (CASE/WHEN, multi-field updates, type conversion).
- Limited error reporting and preview.

### Recommendations
- **Full SQL Expression Engine:**
  - Support for UPDATE, CASE/WHEN, and nested expressions.
  - Allow creation of new fields (ALTER TABLE ADD COLUMN or equivalent).
  - Type conversion (int, float, string, date, bool).
  - Batch updates across multiple fields.
- **Preview and Error Reporting:**
  - Show preview of calculated values before commit.
  - Clear error messages for invalid expressions or type mismatches.
- **Examples:**
  - Update field with conditional logic:
    ```sql
    UPDATE roads SET ONEWAY = CASE WHEN direction = 'FT' THEN 1 ELSE 0 END;
    ```
  - Create new field with type conversion:
    ```sql
    ALTER TABLE parcels ADD COLUMN area_str TEXT;
    UPDATE parcels SET area_str = CAST(area AS TEXT);
    ```
- **Integration:**
  - Expose engine in both CLI and QGIS plugin.
  - Document with neutral, environment-style examples.

### Implementation Update (May 21, 2026)
- Implemented in open-tier `field_calculator`:
  - SQL-style `CASE WHEN ... THEN ... [ELSE ...] END` expressions.
  - SQL-style simple `CASE field WHEN value THEN ... [ELSE ...] END` expressions.
  - Optional `UPDATE ... SET FIELD = ... [WHERE ...]` wrapper parsing (right-hand expression extraction + conditional update filter).
  - SQL-style condition normalization for `=`, `<>`, `AND`, `OR`, `NOT`, and `IS NULL` / `IS NOT NULL`.
  - `CAST(... AS type)` support for integer, float, text, boolean, and text-backed date/datetime/json casts.
  - `preview_rows` result payload support for preview-first GUI workflows without requiring an output write.
- This now supports direct TYPE-to-SPEED derivation workflows for network impedance preparation.
- Example:
  ```sql
  UPDATE roads SET SPEED = CASE
    WHEN TYPE == "motorway" THEN 100
    WHEN TYPE == "primary" THEN 80
    WHEN TYPE == "collector" THEN 60
    ELSE 40
  END;
  ```
- Not yet implemented in this pass:
  - Full SQL parser, multi-field batch updates, and preview UI.

### Future Add-ons
- **Geometry-aware expression context:**
  - Add expression objects and helpers for geometry-centric workflows rather than limiting formulas to scalar attribute values and a small fixed set of geometry variables.
  - Candidate functions include point construction, centroid access, coordinate extraction, length/perimeter/area accessors, and geometry serialization helpers for debugging and export.
- **Spatial SQL-style predicates and measures:**
  - Add opt-in spatial functions such as `ST_Distance`, `ST_DWithin`, `ST_Intersects`, `ST_Contains`, `ST_Touches`, and `ST_MakePoint`-style constructors where they fit the expression model.
  - Treat these as a focused expression layer, not an attempt to reproduce a full PostGIS or ArcGIS SQL engine inside a single tool.
- **Planned execution model for performance-sensitive spatial expressions:**
  - Use explicit spatial indexing and candidate filtering when expressions reference other geometries or layer-to-layer lookups.
  - Preserve the current fast path for pure per-feature scalar expressions so ordinary attribute calculations do not pay the cost of spatial planning.
  - Be explicit in docs that even after these add-ons, this tool is meant for convenient inline analysis and attribute derivation, not for matching dedicated spatial database query planners on large relational workloads.
- **QGIS-facing implications:**
  - Future GUI work should expose these capabilities as guided building blocks with previews and examples, instead of expecting users to memorize raw spatial-function syntax.
  - Preview UI should surface normalized expressions, conditional filters, and any performance caveats when a formula triggers spatial lookups.

---

## 2. Output Clarity for CSV-Generating Tools

### Current State
- Tools like List Unique Values auto-generate CSVs with minimal context.
- Output file locations and formats are sometimes unclear to users.

### Recommendations
- **Standardized Output Schema:**
  - Consistent headers, metadata rows (tool name, run date, input file, parameters).
  - Clear indication of field types and value meanings.
- **Output Location and Naming:**
  - Predictable, user-facing output paths (e.g., outputs/unique_values_<layer>_<field>.csv).
  - Option to specify output location and filename.
- **User Feedback:**
  - Print output path and summary to console/log after tool run.
  - Optionally open output folder or file in UI.
- **Applies To:**
  - List Unique Values, Summary Statistics, Field Calculator (when exporting), and all tools with autogenerated CSV outputs.

---

## 3. General Vector Platform Robustness

- **Spatial Join:**
  - Expand geometry predicates (touches, crosses, overlaps, contains, etc.).
  - Support join cardinality control (one-to-one, one-to-many, many-to-many).
  - Attribute merge strategies (first, all, summary stats).
  - Topology-aware joins (snap, buffer, etc.).
- **Topology Operations:**
  - Continue to harden overlay, noding, and validation routines.
  - Ensure robust handling of edge cases (slivers, holes, collinear points).
- **Documentation:**
  - Provide clear, example-driven docs for all advanced features.
  - Highlight differences and parity with QGIS/ArcGIS where relevant.

---

## 4. Implementation Plan

1. **Field Calculator Upgrade:**
   - Audit current SQL engine; extend for CASE/WHEN, field creation, type conversion.
   - Add preview and error reporting.
   - Update CLI and QGIS plugin interfaces.
2. **CSV Output Improvements:**
   - Standardize output schema and file naming.
   - Add metadata and user feedback.
   - Refactor affected tools (List Unique Values, etc.).
3. **Spatial Join & Topology:**
   - Expand join and predicate support.
   - Harden overlay and validation routines.
4. **Documentation:**
   - Update docs with new examples and workflows.

---

## 5. Next Steps

- Prioritize Field Calculator and CSV output improvements for immediate user impact.
- Schedule audits and upgrades for spatial join and topology modules.
- Gather user feedback after each release phase.

---

**See also:** Other VECTOR docs in this folder for historical context and technical details.
