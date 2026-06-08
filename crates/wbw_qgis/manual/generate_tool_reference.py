#!/usr/bin/env python3
"""Generate QGIS manual tool reference pages from tool_taxonomy.toml and help_static HTML files.

Usage (run from repo root):
    python3 crates/wbw_qgis/manual/generate_tool_reference.py

Generates one markdown file per taxonomy subcategory in crates/wbw_qgis/manual/src/,
then prints the new SUMMARY.md content to stdout.
"""

import os
import re
import sys

# ---------------------------------------------------------------------------
# Paths (relative to repo root)
# ---------------------------------------------------------------------------
TAXONOMY_FILE = "crates/wbw_python/tool_taxonomy.toml"
HELP_DIR = "crates/wbw_qgis/plugin/whitebox_workflows_qgis/help_static"
MANUAL_SRC = "crates/wbw_qgis/manual/src"

# ---------------------------------------------------------------------------
# Human-readable display names
# ---------------------------------------------------------------------------
SUBCATEGORY_DISPLAY = {
    "general": "General Tools",
    "visibility": "Visibility Analysis",
    "derivatives": "Terrain Derivatives",
    "roughness_texture": "Roughness and Texture",
    "landform_indices": "Landform Indices",
    "multiscale_signatures": "Multiscale Signatures",
    "workflow_products": "Workflow Products",
    "flow_routing": "Flow Routing",
    "depressions_storage": "Depressions and Storage",
    "watersheds_basins": "Watersheds and Basins",
    "hydrologic_indices": "Hydrologic Indices",
    "network_extraction": "Stream Network Extraction",
    "longitudinal_analysis": "Longitudinal Profile Analysis",
    "ordering_metrics": "Stream Ordering and Metrics",
    "io_management": "I/O and Data Management",
    "filtering_classification": "Filtering and Classification",
    "interpolation_gridding": "Interpolation and Gridding",
    "analysis_metrics": "Analysis and Metrics",
    "obia": "Object-Based Image Analysis (OBIA)",
    "classification": "Image Classification",
    "change_detection": "Change Detection",
    "radiometric_correction": "Radiometric Correction",
    "thermal_emissivity": "Thermal and Emissivity",
    "spectral_analytics": "Spectral Analytics",
    "edge_feature_detection": "Edge and Feature Detection",
    "enhancement_contrast": "Image Enhancement and Contrast",
    "filters": "Image Filters",
    "sar": "SAR Processing",
    "spatial_statistics": "Spatial Statistics",
    "overlay_math": "Overlay and Math",
    "local_neighborhood": "Local and Neighborhood",
    "reclass_mask": "Reclass and Mask",
    "distance_cost": "Distance and Cost",
    "overlay_analysis": "Overlay Analysis",
    "geometry_processing": "Geometry Processing",
    "shape_metrics": "Shape Metrics",
    "sampling_gridding": "Sampling and Gridding",
    "attribute_analysis": "Attribute Analysis",
    "online_data": "Online Data",
    "network_analysis": "Network Analysis — Tool Reference",
    "linear_referencing": "Linear Referencing — Tool Reference",
    "vector_table_io": "Vector and Table I/O",
    "geometry_topology": "Geometry and Topology",
    "raster_vector_conversion": "Raster-Vector Conversion",
}

# ---------------------------------------------------------------------------
# Output filename for each (category, subcategory) pair
# ---------------------------------------------------------------------------
FILE_MAP = {
    ("terrain", "general"):               "terrain-general.md",
    ("terrain", "derivatives"):           "terrain-derivatives.md",
    ("terrain", "visibility"):            "terrain-visibility.md",
    ("terrain", "roughness_texture"):     "terrain-roughness-texture.md",
    ("terrain", "landform_indices"):      "terrain-landform-indices.md",
    ("terrain", "multiscale_signatures"): "terrain-multiscale-signatures.md",
    ("terrain", "workflow_products"):     "terrain-workflow-products.md",

    ("hydrology", "flow_routing"):        "hydrology-flow-routing.md",
    ("hydrology", "depressions_storage"): "hydrology-depressions-storage.md",
    ("hydrology", "watersheds_basins"):   "hydrology-watersheds-basins.md",
    ("hydrology", "hydrologic_indices"):  "hydrology-hydrologic-indices.md",

    ("streams", "network_extraction"):    "streams-network-extraction.md",
    ("streams", "longitudinal_analysis"): "streams-longitudinal-analysis.md",
    ("streams", "ordering_metrics"):      "streams-ordering-metrics.md",

    ("lidar", "io_management"):           "lidar-io-management.md",
    ("lidar", "filtering_classification"):"lidar-filtering-classification.md",
    ("lidar", "interpolation_gridding"):  "lidar-interpolation-gridding.md",
    ("lidar", "analysis_metrics"):        "lidar-analysis-metrics.md",
    ("lidar", "workflow_products"):       "lidar-workflow-products.md",

    ("remote_sensing", "obia"):                  "remote-sensing-obia.md",
    ("remote_sensing", "classification"):         "remote-sensing-classification.md",
    ("remote_sensing", "change_detection"):       "remote-sensing-change-detection.md",
    ("remote_sensing", "radiometric_correction"): "remote-sensing-radiometric-correction.md",
    ("remote_sensing", "thermal_emissivity"):     "remote-sensing-thermal-emissivity.md",
    ("remote_sensing", "spectral_analytics"):     "remote-sensing-spectral-analytics.md",
    ("remote_sensing", "edge_feature_detection"): "remote-sensing-edge-feature-detection.md",
    ("remote_sensing", "enhancement_contrast"):   "remote-sensing-enhancement-contrast.md",
    ("remote_sensing", "filters"):                "remote-sensing-filters.md",
    ("remote_sensing", "sar"):                    "remote-sensing-sar.md",
    ("remote_sensing", "workflow_products"):      "remote-sensing-workflow-products.md",

    ("raster", "spatial_statistics"):  "raster-spatial-statistics.md",
    ("raster", "overlay_math"):        "raster-overlay-math.md",
    ("raster", "local_neighborhood"):  "raster-local-neighborhood.md",
    ("raster", "reclass_mask"):        "raster-reclass-mask.md",
    ("raster", "distance_cost"):       "raster-distance-cost.md",
    ("raster", "general"):             "raster-general.md",

    ("vector", "overlay_analysis"):    "vector-overlay-analysis.md",
    ("vector", "geometry_processing"): "vector-geometry-processing.md",
    ("vector", "shape_metrics"):       "vector-shape-metrics.md",
    ("vector", "sampling_gridding"):   "vector-sampling-gridding.md",
    ("vector", "attribute_analysis"):  "vector-attribute-analysis.md",
    ("vector", "spatial_statistics"):  "vector-spatial-statistics.md",
    ("vector", "online_data"):         "vector-online-data.md",
    ("vector", "workflow_products"):   "vector-workflow-products.md",
    ("vector", "network_analysis"):    "vector-network-analysis.md",
    ("vector", "linear_referencing"):  "vector-linear-referencing.md",

    ("projection_georeferencing", "general"): "projection-georeferencing-tools.md",
    ("precision_agriculture", "general"):     "precision-agriculture-tools.md",

    ("conversion", "vector_table_io"):        "conversion-vector-table-io.md",
    ("conversion", "geometry_topology"):      "conversion-geometry-topology.md",
    ("conversion", "raster_vector_conversion"): "conversion-raster-vector.md",
}

# ---------------------------------------------------------------------------
# Token upgrades for tool_display_name()
# ---------------------------------------------------------------------------
WORD_UPGRADES = {
    "d8": "D8", "rho8": "Rho8", "dinf": "D-Infinity", "fd8": "FD8",
    "mdinf": "MD-Infinity", "qin": "Qin", "dem": "DEM", "dems": "DEMs",
    "idw": "IDW", "tin": "TIN", "lidar": "LiDAR", "las": "LAS",
    "obia": "OBIA", "sar": "SAR", "pca": "PCA", "ihs": "IHS",
    "rgb": "RGB", "svm": "SVM", "knn": "kNN", "glcm": "GLCM",
    "tpi": "TPI", "twi": "TWI", "slic": "SLIC", "osm": "OSM",
    "od": "OD", "csv": "CSV", "fft": "FFT", "crs": "CRS",
    "cvrp": "CVRP", "vrptw": "VRPTW", "gcp": "GCP", "qa": "QA",
    "nd": "ND", "ndvi": "NDVI", "ndre": "NDRE", "brdf": "BRDF",
    "ks": "KS", "nn": "NN", "v1": "v1", "2d": "2D",
}


def tool_display_name(name: str) -> str:
    parts = name.split("_")
    result = []
    for p in parts:
        result.append(WORD_UPGRADES.get(p.lower(), p.capitalize()))
    return " ".join(result)


# ---------------------------------------------------------------------------
# HTML → Markdown converter
# ---------------------------------------------------------------------------

def html_to_markdown(html: str) -> str:
    # Strip Project Links section and everything after it
    html = re.sub(
        r'<h2>\s*Project Links\s*</h2>.*',
        '', html, flags=re.DOTALL | re.IGNORECASE
    )

    # Function Signature → fenced Python code block
    html = re.sub(
        r'<h2>\s*Function Signature\s*</h2>\s*<p>\s*<code>(.*?)</code>\s*</p>',
        lambda m: '\n\n### Python API\n\n```python\n' + m.group(1).strip().rstrip(' .') + '\n```\n',
        html, flags=re.DOTALL | re.IGNORECASE
    )

    # H2 headings
    html = re.sub(r'<h2>(.*?)</h2>', r'\n\n### \1\n\n', html, flags=re.IGNORECASE)
    # H3 headings
    html = re.sub(r'<h3>(.*?)</h3>', r'\n\n#### \1\n\n', html, flags=re.IGNORECASE)

    # Inline code
    html = re.sub(r'<code>(.*?)</code>', r'`\1`', html, flags=re.DOTALL)

    # Emphasis / bold
    html = re.sub(r'<em>(.*?)</em>', r'*\1*', html, flags=re.DOTALL)
    html = re.sub(r'<strong>(.*?)</strong>', r'**\1**', html, flags=re.DOTALL)

    # Links → keep link text only (strip href; legacy manual links aren't useful inline)
    html = re.sub(r'<a\s[^>]*>([^<]*)</a>', r'`\1`', html, flags=re.IGNORECASE)

    # Paragraphs
    html = re.sub(r'<p>', '\n\n', html, flags=re.IGNORECASE)
    html = re.sub(r'</p>', '', html, flags=re.IGNORECASE)

    # Lists
    html = re.sub(r'<ul>', '\n', html, flags=re.IGNORECASE)
    html = re.sub(r'</ul>', '\n', html, flags=re.IGNORECASE)
    html = re.sub(r'<ol>', '\n', html, flags=re.IGNORECASE)
    html = re.sub(r'</ol>', '\n', html, flags=re.IGNORECASE)
    html = re.sub(r'<li>', '\n- ', html, flags=re.IGNORECASE)
    html = re.sub(r'</li>', '', html, flags=re.IGNORECASE)

    # Remove any remaining tags
    html = re.sub(r'<[^>]+>', '', html)

    # HTML entities
    html = (html
            .replace('&gt;', '>')
            .replace('&lt;', '<')
            .replace('&amp;', '&')
            .replace('&nbsp;', ' ')
            .replace('&#39;', "'")
            .replace('&quot;', '"'))

    # Collapse 3+ blank lines → 2
    html = re.sub(r'\n{3,}', '\n\n', html)

    return html.strip()


# ---------------------------------------------------------------------------
# TOML parser (no external dependencies)
# ---------------------------------------------------------------------------

def parse_taxonomy(path: str):
    """Return list of (category, subcategory, [tool_names])."""
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()

    mappings = []
    for section in re.split(r'\[\[mapping\]\]', content)[1:]:
        cat_m = re.search(r'category\s*=\s*"([^"]+)"', section)
        sub_m = re.search(r'subcategory\s*=\s*"([^"]+)"', section)
        if not cat_m or not sub_m:
            continue
        category = cat_m.group(1)
        subcategory = sub_m.group(1)

        tools_m = re.search(r'tools\s*=\s*\[(.*?)\]', section, re.DOTALL)
        tools = []
        if tools_m:
            for line in tools_m.group(1).splitlines():
                line = re.sub(r'#.*', '', line).strip()
                tm = re.match(r'"([^"]+)"', line)
                if tm:
                    tools.append(tm.group(1))

        mappings.append((category, subcategory, tools))

    return mappings


# ---------------------------------------------------------------------------
# Page generation
# ---------------------------------------------------------------------------

def load_help(tool_name: str, help_dir: str):
    path = os.path.join(help_dir, f"{tool_name}.html")
    if not os.path.exists(path):
        return None
    with open(path, "r", encoding="utf-8") as f:
        return html_to_markdown(f.read())


def generate_tool_page(category: str, subcategory: str, tools: list, help_dir: str) -> str:
    subcat_name = SUBCATEGORY_DISPLAY.get(subcategory, subcategory.replace("_", " ").title())
    lines = [f"# {subcat_name}\n"]
    missing = []

    for tool in tools:
        display = tool_display_name(tool)
        help_md = load_help(tool, help_dir)

        lines.append(f"\n---\n\n## {display}\n\n**Function name:** `{tool}`\n")

        if help_md:
            lines.append(f"\n{help_md}\n")
        else:
            missing.append(tool)
            lines.append(f"\n*No help documentation available for this tool.*\n")

    if missing:
        print(f"  [WARN] {len(missing)} tools missing help files in "
              f"{category}/{subcategory}: {', '.join(missing)}", file=sys.stderr)

    return "\n".join(lines)


# ---------------------------------------------------------------------------
# New top-level chapter intro pages
# ---------------------------------------------------------------------------

NEW_CHAPTERS = {
    "projection-georeferencing.md": (
        "Projection and Georeferencing",
        """Accurate coordinate reference systems (CRS) and georeferencing are foundational to all GIS work. This chapter covers tools for assigning, reprojecting, and transforming spatial data between coordinate systems, as well as tools for georeferencing rasters from ground control points.

## Key Concepts

- **CRS / Projection**: A mathematical model that defines how geographic coordinates map to a flat surface. All spatial data in a GIS project must share a common CRS for overlay and analysis to be meaningful.
- **Reprojection**: Transforming data from one CRS to another. Whitebox Workflows supports epoch-aware datum transformations for sub-metre accuracy.
- **Georeferencing**: Assigning spatial coordinates to a raster image using ground control points (GCPs), typically for aerial photography, scanned maps, or satellite imagery without embedded metadata.
- **Orthorectification**: Correcting geometric distortions in aerial/satellite imagery caused by terrain relief and sensor tilt.

## Tool Reference

The tools in this chapter are accessible from the QGIS Processing Toolbox under **Whitebox Workflows → Projection and Georeferencing**.
"""
    ),
    "precision-agriculture.md": (
        "Precision Agriculture",
        """Precision agriculture tools integrate high-resolution geospatial data — LiDAR, multispectral imagery, soil surveys, and yield maps — to support evidence-based farm management decisions. These are specialised **Pro tier** workflow tools that automate complex multi-source analyses into actionable management zones and field intelligence reports.

## Key Concepts

- **Management Zones**: Spatially delineated areas of a field that share similar soil, crop, or topographic characteristics, enabling variable-rate application of inputs such as fertiliser, seed, or irrigation water.
- **Yield Data Conditioning**: Raw yield monitor data contains significant noise and artifacts; conditioning standardises and quality-controls it before analysis.
- **Crop Stress Detection**: Using multispectral imagery (NDVI, NDRE, CWSI) to identify in-season stress before visible symptoms appear, enabling targeted intervention.
- **Field Trafficability**: Assessing soil compaction risk and field access conditions based on soil moisture models and terrain analysis.

## Tool Reference

The tools in this chapter are accessible from the QGIS Processing Toolbox under **Whitebox Workflows → Precision Agriculture**.

> **Note:** All Precision Agriculture tools require a **Pro license**.
"""
    ),
    "data-conversion.md": (
        "Data Conversion and Format Tools",
        """Data conversion tools handle format transformations, topology repairs, and attribute table operations that are essential plumbing in any GIS workflow. These tools prepare data for analysis, export results to standard formats, and ensure geometric and topological consistency.

## Key Concepts

- **Vector-Raster Conversion**: Many analysis pipelines require moving between vector and raster representations. Whitebox provides precise control over cell size, nodata handling, and attribute transfer during conversion.
- **Topology Repair**: Real-world vector data often contains geometric errors — dangling arcs, unclosed polygons, multipart features — that cause downstream analysis failures. The topology tools detect and fix these automatically.
- **Attribute Table I/O**: Joining external CSVs, merging attribute tables across layers, and exporting to standard tabular formats is routine data preparation work.

## Tool Reference

The tools in this chapter are accessible from the QGIS Processing Toolbox under **Whitebox Workflows → Data Conversion**.
"""
    ),
}


# ---------------------------------------------------------------------------
# SUMMARY.md target structure
# ---------------------------------------------------------------------------

NEW_SUMMARY = """\
# Summary

- [Overview](./overview.md)
- [Installation and Setup](./installation-and-setup.md)
- [Build and Preview](./build-and-preview.md)
- [Quick Start](./quick-start.md)
- [Runtime and Discovery](./runtime-and-discovery.md)
- [Tool Execution in QGIS](./tool-execution-in-qgis.md)
- [Recipes](./recipes.md)
- [Licensing and Tiers](./licensing-and-tiers.md)
- [Supported Data Formats](./data-formats.md)
- [Reprojection and CRS](./reprojection-crs.md)
- [Terrain Analysis and Geomorphometry](./terrain-analysis.md)
  - [Terrain Derivatives](./terrain-derivatives.md)
  - [Visibility Analysis](./terrain-visibility.md)
  - [Roughness and Texture](./terrain-roughness-texture.md)
  - [Landform Indices](./terrain-landform-indices.md)
  - [Multiscale Signatures](./terrain-multiscale-signatures.md)
  - [General Tools](./terrain-general.md)
  - [Workflow Products](./terrain-workflow-products.md)
- [Spatial Hydrology](./spatial-hydrology.md)
  - [Flow Routing](./hydrology-flow-routing.md)
  - [Depressions and Storage](./hydrology-depressions-storage.md)
  - [Watersheds and Basins](./hydrology-watersheds-basins.md)
  - [Hydrologic Indices](./hydrology-hydrologic-indices.md)
  - [Stream Network Extraction](./streams-network-extraction.md)
  - [Longitudinal Profile Analysis](./streams-longitudinal-analysis.md)
  - [Stream Ordering and Metrics](./streams-ordering-metrics.md)
- [LiDAR Processing](./lidar-processing.md)
  - [I/O and Data Management](./lidar-io-management.md)
  - [Filtering and Classification](./lidar-filtering-classification.md)
  - [Interpolation and Gridding](./lidar-interpolation-gridding.md)
  - [Analysis and Metrics](./lidar-analysis-metrics.md)
  - [Workflow Products](./lidar-workflow-products.md)
- [Remote Sensing Analysis](./remote-sensing.md)
  - [Image Enhancement and Contrast](./remote-sensing-enhancement-contrast.md)
  - [Image Filters](./remote-sensing-filters.md)
  - [Edge and Feature Detection](./remote-sensing-edge-feature-detection.md)
  - [Image Classification](./remote-sensing-classification.md)
  - [Object-Based Image Analysis](./remote-sensing-obia.md)
  - [Change Detection](./remote-sensing-change-detection.md)
  - [Radiometric Correction](./remote-sensing-radiometric-correction.md)
  - [Thermal and Emissivity](./remote-sensing-thermal-emissivity.md)
  - [Spectral Analytics](./remote-sensing-spectral-analytics.md)
  - [SAR Processing](./remote-sensing-sar.md)
  - [Workflow Products](./remote-sensing-workflow-products.md)
- [Raster Analysis](./raster-analysis.md)
  - [Overlay and Math](./raster-overlay-math.md)
  - [Distance and Cost](./raster-distance-cost.md)
  - [Spatial Statistics](./raster-spatial-statistics.md)
  - [Reclass and Mask](./raster-reclass-mask.md)
  - [Local and Neighborhood](./raster-local-neighborhood.md)
  - [General Tools](./raster-general.md)
- [Vector Analysis](./vector-analysis.md)
  - [Overlay Analysis](./vector-overlay-analysis.md)
  - [Geometry Processing](./vector-geometry-processing.md)
  - [Shape Metrics](./vector-shape-metrics.md)
  - [Sampling and Gridding](./vector-sampling-gridding.md)
  - [Attribute Analysis](./vector-attribute-analysis.md)
  - [Spatial Statistics](./vector-spatial-statistics.md)
  - [Online Data](./vector-online-data.md)
  - [Workflow Products](./vector-workflow-products.md)
- [Network Analysis](./network-analysis.md)
  - [Tool Reference](./vector-network-analysis.md)
- [Linear Referencing](./linear-referencing.md)
  - [Tool Reference](./vector-linear-referencing.md)
- [Projection and Georeferencing](./projection-georeferencing.md)
  - [Tool Reference](./projection-georeferencing-tools.md)
- [Precision Agriculture](./precision-agriculture.md)
  - [Tool Reference](./precision-agriculture-tools.md)
- [Data Conversion](./data-conversion.md)
  - [Vector and Table I/O](./conversion-vector-table-io.md)
  - [Geometry and Topology](./conversion-geometry-topology.md)
  - [Raster-Vector Conversion](./conversion-raster-vector.md)
- [Workflow Index](./script-index.md)
- [Troubleshooting](./troubleshooting.md)
"""


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    # Validate we're running from repo root
    if not os.path.exists(TAXONOMY_FILE):
        sys.exit(f"ERROR: Cannot find {TAXONOMY_FILE}. Run this script from the repo root.")

    mappings = parse_taxonomy(TAXONOMY_FILE)
    total_tools = sum(len(t) for _, _, t in mappings)
    print(f"Parsed taxonomy: {len(mappings)} subcategories, {total_tools} tool entries",
          file=sys.stderr)

    generated = 0
    skipped = 0

    for category, subcategory, tools in mappings:
        key = (category, subcategory)
        filename = FILE_MAP.get(key)
        if not filename:
            print(f"  [SKIP] No file mapping for ({category}, {subcategory})", file=sys.stderr)
            skipped += 1
            continue

        out_path = os.path.join(MANUAL_SRC, filename)
        print(f"  Generating {filename} ({len(tools)} tools)...", file=sys.stderr)

        content = generate_tool_page(category, subcategory, tools, HELP_DIR)
        with open(out_path, "w", encoding="utf-8") as f:
            f.write(content)
        generated += 1

    # Write new top-level chapter pages (only if they don't exist yet)
    for filename, (title, intro) in NEW_CHAPTERS.items():
        out_path = os.path.join(MANUAL_SRC, filename)
        if os.path.exists(out_path):
            print(f"  [SKIP] {filename} already exists, not overwriting.", file=sys.stderr)
            continue
        content = f"# {title}\n\n{intro}\n"
        with open(out_path, "w", encoding="utf-8") as f:
            f.write(content)
        print(f"  Created new chapter: {filename}", file=sys.stderr)

    # Write SUMMARY.md
    summary_path = os.path.join(MANUAL_SRC, "SUMMARY.md")
    with open(summary_path, "w", encoding="utf-8") as f:
        f.write(NEW_SUMMARY)
    print(f"  Updated SUMMARY.md", file=sys.stderr)

    print(
        f"\nDone. Generated {generated} tool reference pages, skipped {skipped} unmapped subcategories.",
        file=sys.stderr
    )


if __name__ == "__main__":
    main()
