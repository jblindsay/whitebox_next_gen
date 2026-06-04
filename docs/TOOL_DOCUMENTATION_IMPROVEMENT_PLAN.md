# Tool Documentation Improvement Plan

**Date:** June 4, 2026  
**Version:** 1.0  
**Status:** Strategic Framework (Implementation pending)

---

## Executive Summary

The Next Gen codebase has ~700+ tools compared to Legacy's ~540 tools. User feedback indicates documentation quality is "severely lacking compared to the past." This plan outlines a staged approach to significantly improve tool help documentation while remaining pragmatic about resource constraints.

**Key Constraint:** This is a quality, not quantity, problem—users don't need docs for all 700 tools immediately; they need *rich, meaningful* docs for the tools they use most.

---

## Problem Statement

### Current State
- **Next Gen API docs:** Brief descriptions (1-2 sentences), no images, minimal cross-referencing
- **User manuals:** Don't have dedicated tool entries for most tools
- **QGIS help:** Auto-generated from metadata, minimal detail
- **Legacy quality exemplar:** 455 OSS + 81 Pro tools with 2-3 paragraph descriptions, embedded images, references, "See Also" cross-links

### User Impact
- Users can't understand tool purpose from API docs alone
- Complex workflows (kriging, SAR, classification) lack conceptual grounding
- New users struggle to find appropriate tools for their tasks
- Missing images/examples reduce discoverability

---

## Critical Design Decision: Where Descriptions Live

### ❌ NOT in tool_taxonomy.toml
- ✗ Would make configuration file unreadable and difficult to maintain
- ✗ Defeats the purpose of tool_taxonomy.toml (clean, easy reassignment of categories)
- ✗ Mixing concerns: categorization (config) vs. documentation (content)

### ✅ Distributed Across Multiple Locations

**1. Tool Metadata (Primary Discovery)**
- Location: Each tool's `metadata()` method in `crates/wbtools_oss/src/tools/*/`
- Field: Expand the `summary` field from 1-2 sentences to 2-3 paragraphs
- Flow: Automatically visible in Python help(), QGIS tooltips, R documentation
- Scope: Rich enough for users to understand purpose, but concise enough for UI tooltips

**2. Markdown User Manuals (Comprehensive Reference)**
- Location: `docs/user-manuals/tools/` organized by category (terrain/, vector/, lidar/, etc.)
- Format: One `.md` file per tool category with detailed entries
- Content: Full descriptions, parameter explanations, examples, images, references
- Scope: Complete guide for users who want deep understanding

**3. Image Assets (Supporting Material)**
- Location: `docs/user-manuals/tools/img/` organized by category
- Sourcing: Extracted from legacy docs (with proper attribution) + new screenshots
- Format: PNG/SVG with captions and context
- Scope: Key algorithm diagrams, before/after examples, workflow illustrations

---

## Semantic Deep-Dive Principle

⚠️ **Critical:** Legacy docs are *starting points*, NOT gospel truth.

### Why Semantic Deep-Dive is Needed Even for Legacy Equivalents

Many tools have been significantly modified in Next Gen:
- **Example 1:** Vector format → Legacy assumed Shapefile; Next Gen supports multiple formats (GeoPackage, GeoJSON, Shapefile)
- **Example 2:** Algorithm updates → kriging implementations now include cross-validation, prediction intervals, anisotropy
- **Example 3:** Parameter changes → filters may have different kernel sizes, methods, or output interpretations
- **Example 4:** Licensing changes → tools moved between Open/Pro tiers

### Validation Approach

For each legacy-equivalent tool:
1. **Read legacy description** as exemplar for detail/structure
2. **Deep-dive codebase:**
   - Review source implementation in `crates/wbtools_oss/src/tools/*/`
   - Check unit tests to understand expected behavior
   - Review commit history for recent changes
   - Run small examples to validate behavior
3. **Cross-check:**
   - Compare with original publication/paper if relevant
   - Note any API/behavior differences from legacy
   - Flag deprecations or improvements
4. **Write new description:**
   - Adapt legacy language where still accurate
   - Update format/interface references
   - Add new capabilities not in legacy
   - Maintain legacy's rigor without copying outdated specifics

---

## Tiered Implementation Strategy

### TIER 1: High-Impact Quick Wins (Weeks 1-2, ~60 hours)

**Goal:** Prove feasibility and establish quality baseline

**Scope:**
- Top 15-20 tools per category with largest user base
- Spatial Statistics (22 tools) → Full coverage (recent work, high expertise)
- Terrain Analysis (15 tools) → Highest visibility tools
- LiDAR Core (15 tools) → Critical user workflows

**Deliverables:**
- Expanded metadata.summary for 50 tools
- Markdown help files for same tools with examples
- 10-15 images/diagrams extracted and integrated
- Quality template established

**Process:**
1. Identify top tools by likely usage (flow accumulation, kriging, filtering, classification)
2. Semantic deep-dive on each
3. Write 2-3 paragraph metadata summaries
4. Write full markdown help entries
5. Extract/create images
6. Cross-link related tools

---

### TIER 2: Medium-Coverage Systematic Pass (Weeks 3-8, ~120 hours)

**Goal:** Cover all legacy-equivalent tools + 50% of new tools

**Scope:**
- Complete Spatial Statistics (rest of 22)
- Terrain Analysis (full coverage ~50 tools)
- LiDAR Processing (full coverage ~40 tools)
- Vector Operations (selected high-value tools ~30)
- Raster Mathematics (selected core operations ~20)

**Deliverables:**
- Metadata summaries for 250+ tools
- Markdown help files organized by category
- Image library by category
- Consistency guidelines documented

---

### TIER 3: Long Tail Polish (Weeks 9+, ongoing ~80 hours)

**Goal:** Complete coverage of remaining tools

**Scope:**
- Remote Sensing Classification (30 tools)
- Specialized tools (CAR, network analysis, advanced filters)
- Utility tools (simple with standard templates)

**Strategy:** Lower-priority tools get standard template format:
```markdown
## tool_name

### Purpose
[1-2 sentence overview]

### Parameters
[Parameter table with descriptions]

### Outputs
[Output specification]

### See Also
[Related tools]
```

---

## Implementation Architecture

### File Structure

```
docs/
├── TOOL_DOCUMENTATION_IMPROVEMENT_PLAN.md (this file)
├── user-manuals/
│   └── tools/
│       ├── TEMPLATE.md (template for consistency)
│       ├── img/
│       │   ├── terrain/
│       │   ├── vector/
│       │   ├── lidar/
│       │   └── spatial-stats/
│       └── [category]/
│           ├── terrain-analysis.md
│           ├── flow-accumulation.md
│           ├── kriging.md
│           └── ...
└── legacy-doc-mapping.json (tool ID → legacy help section)
```

### Tool Metadata Enhancement

**Current structure (in tool implementations):**
```rust
ToolMetadata {
    id: "ordinary_kriging",
    display_name: "Ordinary Kriging",
    summary: "Interpolates values at unsampled locations.",  // ← TOO BRIEF
    category: ToolCategory::Raster,
    ...
}
```

**Enhanced structure (proposed):**
```rust
ToolMetadata {
    id: "ordinary_kriging",
    display_name: "Ordinary Kriging Interpolation",
    summary: r#"
        Performs ordinary kriging interpolation to estimate values at unsampled 
        locations based on a point dataset and empirical variogram model. 
        This geostatistical method provides both interpolated predictions and 
        associated kriging variance (prediction uncertainty).

        Ordinary kriging assumes an unknown constant mean and uses variogram 
        modeling to capture spatial autocorrelation structure. Appropriate for 
        continuous phenomena with no known trend (e.g., mineral concentrations, 
        pollution levels).
        
        Includes cross-validation diagnostics and prediction intervals.
    "#.trim(),
    category: ToolCategory::Raster,
    ...
}
```

---

## Legacy Documentation Sourcing

### Input Documents
- Legacy OSS: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/docs/User Manual/wbw-user-manual/src/tool_help.md` (455 tools, 11,671 lines)
- Legacy Pro: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/docs/User Manual/wbw-user-manual/src/tool_help_wbwpro.md` (81 tools, 4,830 lines)

### Extraction Strategy
1. Parse markdown structure: `## tool_name` → extract until next `## `
2. Extract description sections (exclude Function Signature)
3. Extract images with captions
4. Extract "See Also" cross-references
5. Extract "References" section if present
6. Create mapping: legacy_tool_id → next_gen_tool_id

### Attribution
- Note when content is adapted from legacy documentation
- Preserve references and citations
- Update format/terminology (Shapefile → vector, etc.)

---

## Quality Standards

### Metadata Summary (Tool Implementations)
- ✅ 150-250 words minimum
- ✅ First sentence: clear purpose statement
- ✅ Second paragraph: key assumption/limitation
- ✅ Third paragraph: appropriate use cases or output interpretation
- ✅ No jargon without definition
- ✅ Links to related tools in docstrings

### Markdown Help Files
- ✅ 300-500 words for complex tools, 150-200 for simple
- ✅ Sections: Purpose | Parameters | Outputs | Examples | See Also | References
- ✅ Example with actual parameter values shown
- ✅ At least one image for spatial/visual tools
- ✅ Cross-references to 2-3 related tools
- ✅ References section for algorithmic tools (kriging, classification, etc.)

### Consistency Checks
- [ ] All tools in same category use similar structure
- [ ] Cross-references are bidirectional (if A → B, then B → A)
- [ ] No orphaned tools (every tool has "See Also" entries)
- [ ] Format consistency (parameter capitalization, example style)

---

## Priority Order: Which Category First?

### Recommended Start: Spatial Statistics (Your Recent Work)

**Why:**
1. ✅ 22 tools total = manageable scope
2. ✅ Your recent deep-dive provides expertise
3. ✅ High user value (recent complaints in feedback)
4. ✅ Mix of legacy-equivalent (kriging, variography) + new tools (Phase D point process)
5. ✅ Perfect proof-of-concept before scaling

**Next:** Terrain Analysis & LiDAR Core (highest-value categories)

---

## Tools & Automation Opportunities

### Script 1: Legacy Doc Parser
```
Input: legacy tool_help.md
Output: JSON mapping {tool_id → {description, images[], references[]}}
```

### Script 2: Tool Metadata Updater
```
Input: JSON mapping + enhanced descriptions
Output: Updated Rust tool implementations with new metadata
```

### Script 3: Markdown Generator
```
Input: JSON mapping + image paths
Output: Category markdown files with consistent structure
```

### Script 4: Cross-Reference Builder
```
Input: All tool IDs and categories
Output: "See Also" suggestions based on category proximity
```

---

## Rollout & Communication

### Phase 1: Internal Validation
- Community review of Spatial Statistics entries (TIER 1)
- Gather feedback on description depth/style
- Iterate on template and quality standards

### Phase 2: Incremental Publication
- Ship improvements tool-by-tool or category-by-category
- Update CHANGELOG with "Improved documentation for X tools"
- Publicize via blog: "Documentation Refresh Initiative"

### Phase 3: Ongoing Maintenance
- Link improvement PRs to user feedback
- Accept community contributions for tool descriptions
- Annual review cycle

---

## Success Metrics

- [ ] All Spatial Statistics tools have 150+ word metadata summaries
- [ ] 80% of Terrain/LiDAR tools have user manual entries
- [ ] QGIS tooltips show meaningful descriptions (not truncated)
- [ ] User feedback indicates improved discoverability
- [ ] Documentation viewed as production-quality vs. "lacking"

---

## Resource Estimate

| Phase | Scope | Est. Hours | Est. Weeks |
|-------|-------|-----------|-----------|
| TIER 1 | 50 tools + exemplar | 60 | 2 |
| TIER 2 | 200 tools | 120 | 4 |
| TIER 3 | 250+ tools + utilities | 80 | 3+ |
| **Total** | **700+ tools** | **260** | **9+ weeks** |

---

## Revision History

| Version | Date | Author | Change |
|---------|------|--------|--------|
| 1.0 | 2026-06-04 | Copilot | Initial strategic framework |
