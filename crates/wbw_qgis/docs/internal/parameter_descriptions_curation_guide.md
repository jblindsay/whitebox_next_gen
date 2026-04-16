# Parameter Descriptions Curation Guide

**Last Updated**: April 16, 2026  
**Status**: Active Initiative  
**Owner**: Whitebox Workflows QGIS Plugin

## Overview

This document describes the ongoing effort to enrich parameter descriptions across the Whitebox Workflows QGIS plugin. Parameter descriptions are critical for usability, especially given the technical and specialized nature of many Whitebox tools.

### Motivation

Many tools in the Whitebox toolkit have terse or generic parameter labels that don't adequately communicate:
- What the parameter controls and its impact on the algorithm
- Valid ranges, defaults, and their significance
- When and why to adjust the parameter
- Interdependencies with other parameters

Rich parameter descriptions significantly improve user experience and reduce learning curve, particularly for non-expert users.

### Target Outcome

Transform parameter labels and tooltips from terse technical names into **user-friendly, pedagogically-sound descriptions** that guide users to appropriate values. Example:

**Before**: `smoothing_amount`  
**After**: `Overall smoothing amount in [0,1] (default 0.65). Higher values increase the diffusion budget, especially at coarse scales. [optional]`

## Architecture

### Data Sources

1. **Legacy Help Files** (~515 HTML files)
   - Located: `whitebox_workflows/wbw_qgis/help/`
   - Coverage: ~76% of Tier 1 tools (349/459)
   - Status: Automatically extracted via `ToolHelpProvider` in `help_provider.py`
   - Automation: Parameters extracted and set via `setHelp()` on QGIS parameter widgets

2. **Curated Description Database** (new)
   - Located: `crates/wbw_qgis/plugin/whitebox_workflows_qgis/descriptions/`
   - Format: JSON files organized by category/tool
   - Coverage: Tier 1 tools without legacy help (~110 tools)
   - Status: In development
   - Structure: See "Description Database Schema" below

3. **Auto-Generated Baseline Pack** (new)
  - Generator: `crates/wbw_qgis/plugin/scripts/generate_basic_descriptions.py`
  - Output: `crates/wbw_qgis/plugin/whitebox_workflows_qgis/descriptions/auto_generated_tier1.json`
  - Scope: Tier 1 tools without legacy help and without manual curated entries
  - Purpose: Create a strong baseline quickly; manual curation still preferred for high-impact/specialized tools

### Integration Points

- **`help_provider.py`**: Extracts parameter text from legacy HTML help files
- **`algorithm.py` `initAlgorithm()`**: Applies help text via `qgs_param.setHelp()`
- **`descriptions_provider.py`** (new): Loads and serves curated JSON descriptions
- **QGIS Dialog Rendering**: Parameter labels + tooltips automatically populated

### Description Database Schema

```json
{
  "tool_id": {
    "description": "Brief tool summary (optional, for tool-level help)",
    "parameters": {
      "param_name_1": {
        "label": "User-friendly label with context and defaults",
        "tooltip": "Additional guidance for unclear parameters (optional)",
        "category": "input|output|control|optional",
        "examples": "Optional practical guidance (e.g., typical values, common mistakes)"
      },
      "param_name_2": { ... }
    }
  }
}
```

### Label Guidelines

Rich parameter labels should include:

1. **What it controls**: Clear, non-technical explanation
2. **Valid range/options**: `[min, max]`, `{option1, option2}`, or valid formats
3. **Default value**: Why it was chosen, what it means
4. **Impact guidance**: "Higher values increase X", "Lower values optimize for Y"
5. **Optionality**: `[optional]` if not required
6. **Units**: When applicable (meters, degrees, iterations, etc.)

**Format Pattern**:
```
<Description of what the parameter controls> in <range/options> (default <value>). 
<Impact guidance>. [optional]
```

**Examples**:
- `Radius in pixels (default 3). Larger values smooth broader features; smaller values preserve fine detail. Typical range: 2–8.`
- `Search radius in meters (default 100). Determines how far to search for nearest neighbors. Increase for sparse data.`
- `Iterations count (default 5). Higher values produce smoother results but take longer. Range: 1–20.`
- `Filter method {gaussian, median, bilateral} (default gaussian). Bilateral preserves edges better but is slower.`

### Tooltip Guidelines

Use tooltips for:
- Checkboxes and space-limited parameters
- Complex technical details not fitting in label
- Parameter interdependencies
- Algorithm references or citations

**Example**:
```
Label: "Enable edge preservation [optional]"
Tooltip: "When enabled, uses bilateral filtering to protect feature boundaries. 
Recommended for DEMs with cliffs or breaklines. Increases computation time ~2x."
```

## Implementation Timeline

### Phase 1: Infrastructure & Highest-Priority Tools (Weeks 1-2)
- ✅ Legacy help extraction (`help_provider.py`)
- ✅ Integration into algorithm.py
- 🔄 Create `descriptions_provider.py`
- 🔄 Curate descriptions for ~30 highest-impact tools:
  - Vector network analysis (5 tools)
  - Vector spatial operations (5 tools)
  - Terrain/hydrology core workflows (8 tools)
  - LiDAR QA & terrain products (4 tools)
  - Classification core (3 tools)
  - Advanced terrain (5 tools)

### Phase 2: Scale to Full Tier 1 (Weeks 3-4)
- Remaining ~80 tools without legacy help
- Batch descriptions by category
- Community contribution framework

### Phase 3: Tier 2 & Beyond (Future)
- Tools outside Tier 1
- User feedback integration
- Version-to-version refinement

## Priority Categories & High-Impact Tools

### Vector Network & Linear Referencing (Highest Impact)
These enable complex GIS workflows and have emerged tools missing docs:
- `network_od_cost_matrix` — Origin-destination cost analysis
- `network_dijkstra_shortest_path` — Route finding
- `network_shortest_path_between_points` — Point-to-point routing
- `linear_reference_from_points` — Measure position along routes
- `locate_along_route_by_measure` — Place features at route positions

### Vector Spatial Operations (High Impact)
Core tools for spatial analysis; many are new:
- `spatial_join` — Join attributes based on spatial relationship
- `near` — Find nearest neighbors
- `select_by_location` — Filter by spatial criteria
- `buffer` — Expand/contract features
- `convex_hull` — Bounding geometry
- `dissolve` — Aggregate and merge

### Terrain & Hydrology Core (High Impact)
Most popular workflows; some legacy help gaps:
- Feature-preserving smoothing tools (✓ legacy help exists)
- `elev_above_pit` — Depression handling
- `flow_accumulation_d8` — Flow routing variants
- `stream_network_analysis` — Stream feature extraction
- `viewshed` — Line-of-sight analysis

### LiDAR QA & Advanced Products (Medium-High Impact)
Specialized tools critical for LiDAR workflows:
- `lidar_qa_and_confidence` — Quality metrics
- `lidar_terrain_product_suite` — Multi-output workflows
- `max_elev_dev_signature` — Vegetation index computation

### Classification & SAR (Medium Impact)
Emerging remote sensing capabilities:
- `random_forest_classifier_train` — Training workflows
- `sar_coherence_calculation` — SAR processing
- `radar_backscatter_analysis` — Radar interpretation

## Workflow: Adding New Descriptions

### Automated Baseline (recommended first)

1. Run:

```bash
python crates/wbw_qgis/plugin/scripts/generate_basic_descriptions.py
```

2. This generates/refreshes:

`crates/wbw_qgis/plugin/whitebox_workflows_qgis/descriptions/auto_generated_tier1.json`

3. Review a sample of generated entries for readability and correctness.
4. Promote high-impact tools from auto-generated to manually curated category files.
5. Re-run generator after adding new tools to keep baseline current.

Notes:
- Manual curated files should take precedence over generated entries.
- Generated descriptions are intended as a usability floor, not final wording.

### For Individual Tools

1. **Understand the tool**: Read the tool's backend docstring, any existing help, and run it mentally
2. **Extract parameters**: Check `discover_tool_catalog()` for the tool's parameter list
3. **Write descriptions**: Follow the label + tooltip guidelines above
4. **Create or update JSON**: Add entry to `descriptions/<category>.json`
5. **Test in QGIS**: Open the tool dialog and verify labels and tooltips display correctly
6. **Commit**: Include tool ID range in commit message

### For Categories

1. Group tools by category
2. Establish 2-3 example patterns
3. Write batch descriptions (faster than individual)
4. Organize into single JSON file per category
5. Commit with category summary

### For Community Contributions

(To be defined as the system matures)
- Template JSON structure with comments
- Review guidelines
- Integration checklist

## File Structure

```
crates/wbw_qgis/plugin/whitebox_workflows_qgis/
├── help_provider.py                    # Legacy help extraction
├── descriptions_provider.py (new)      # Curated description lookup
├── scripts/
│   └── generate_basic_descriptions.py  # Auto-generate baseline descriptions
└── descriptions/                       # Curated JSON files (new)
  ├── auto_generated_tier1.json       # Generated baseline coverage
    ├── vector_network.json
    ├── vector_spatial.json
    ├── terrain_hydrology.json
    ├── lidar_qa.json
    ├── classification.json
    ├── sar.json
    └── (additional categories)

docs/internal/
└── parameter_descriptions_curation_guide.md  # This file
```

## Testing & Validation

### Pre-Commit Validation
- JSON syntax: `python -m json.tool <file>`
- Parameter name matching: Verify all param_name keys exist in tool manifest
- Label length: Ensure labels fit reasonably in QGIS dialogs (~80–120 chars ideal)

### In-Dialog Testing
1. Open QGIS Whitebox plugin
2. Search for tool with curated descriptions
3. Open tool dialog
4. Verify:
   - Parameter label displays correctly (truncated if needed)
   - Tooltip appears on hover/focus
   - No encoding issues (special chars, symbols)

### User Feedback Loop
- Monitor usage/issues
- Collect feedback on confusing parameters
- Refine descriptions iteratively

## Maintenance & Updates

- **Version updates**: Review descriptions when tool parameters change
- **New tools**: Add descriptions before releasing new tools
- **Community feedback**: Prioritize reported usability issues
- **Toolset expansion**: Apply same framework to Tier 2+ tools

## Related Documentation

- [QGIS Plugin Architecture](WBW_QGIS4_PLUGIN_ARCHITECTURE.md)
- [Algorithm Coverage Report](../qgis_plugin_execution_roadmap.md#phase-1-algorithm-completeness)
- [Help Provider Implementation](../../crates/wbw_qgis/plugin/whitebox_workflows_qgis/help_provider.py)

## Contact & Escalations

For questions or contributions, see the main project README or contact the Whitebox team.

---

**Status**: ACTIVE — Implementing infrastructure (Phase 1, Week 1)  
**Next Milestone**: First 30 tool descriptions + `descriptions_provider.py` integration (Target: ~1 week)
