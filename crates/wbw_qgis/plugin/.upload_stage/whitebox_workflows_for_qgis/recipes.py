from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any


# Phase-1 recipe catalog: lightweight guided entry points for common workflows.
# Recipes are tier-aware but operate on existing tool dialogs.
RECIPE_CATALOG: list[dict[str, Any]] = [
    {
        "id": "terrain_baseline_open",
        "title": "Terrain Baseline (Open)",
        "summary": "Generate slope, aspect, and hillshade from a DEM.",
        "input_hint": "Provide a hydrologically sound DEM raster in projected coordinates.",
        "output_hint": "Use a shared output folder and consistent suffixes: _slope, _aspect, _hillshade.",
        "tier": "open",
        "launch_tool": "slope",
        "tools": ["slope", "aspect", "hillshade"],
    },
    {
        "id": "drainage_screen_open",
        "title": "Drainage Screen (Open)",
        "summary": "Prep DEM hydrology and derive initial stream candidates.",
        "input_hint": "Start from a DEM with sinks removed only if already quality-checked.",
        "output_hint": "Review stream threshold assumptions and document chosen cutoff.",
        "tier": "open",
        "launch_tool": "breach_depressions_least_cost",
        "tools": ["breach_depressions_least_cost", "d8_pointer", "extract_streams"],
    },
    {
        "id": "raster_qa_open",
        "title": "Raster QA Quickcheck (Open)",
        "summary": "Use raster calculator and MSTP classes for fast visual QA.",
        "input_hint": "Use aligned rasters (same extent, resolution, and projection).",
        "output_hint": "Save diagnostic outputs next to source data for audit traceability.",
        "tier": "open",
        "launch_tool": "raster_calculator",
        "tools": ["raster_calculator", "multiscale_topographic_position_class"],
    },
    {
        "id": "lidar_building_cleanup_pro",
        "title": "LiDAR Building Cleanup (Pro)",
        "summary": "Classify buildings and remove off-terrain objects.",
        "input_hint": "Use classified LiDAR with reliable ground returns and known vertical units.",
        "output_hint": "Compare before/after class counts to validate cleanup impact.",
        "tier": "pro",
        "launch_tool": "classify_buildings_in_lidar",
        "tools": ["classify_buildings_in_lidar", "remove_off_terrain_objects"],
    },
    {
        "id": "route_assessment_pro",
        "title": "Route Assessment (Open)",
        "summary": "Run network route assessment workflow.",
        "input_hint": "Ensure network topology is connected and cost fields are populated.",
        "output_hint": "Export route summaries and key constraints for stakeholder review.",
        "tier": "open",
        "launch_tool": "assess_route",
        "tools": ["assess_route"],
    },
    {
        "id": "watershed_delineation_open",
        "title": "Watershed Delineation (Open)",
        "summary": "Delineate basins and drainage lines from a DEM.",
        "input_hint": "Use a projected DEM with appropriate sink treatment for the study area.",
        "output_hint": "Store basin and stream outputs with shared naming prefixes for traceability.",
        "tier": "open",
        "launch_tool": "breach_depressions_least_cost",
        "tools": ["breach_depressions_least_cost", "d8_pointer", "watershed", "extract_streams"],
    },
    {
        "id": "stream_burn_in_prep_open",
        "title": "Stream Burn-In Prep (Open)",
        "summary": "Integrate known streams into DEM flow routing prep.",
        "input_hint": "Use a DEM and vetted stream vector layer in the same projected CRS.",
        "output_hint": "Document burn depth choices and keep pre/post-burn DEM outputs.",
        "tier": "open",
        "launch_tool": "burn_streams",
        "tools": ["burn_streams", "d8_pointer", "extract_streams"],
    },
    {
        "id": "terrain_morphometry_pack_open",
        "title": "Terrain Morphometry Pack (Open)",
        "summary": "Generate core terrain derivatives for morphology analysis.",
        "input_hint": "Use a clean DEM with consistent units and no tile edge artifacts.",
        "output_hint": "Use consistent derivative suffixes for slope/aspect/curvature/convergence.",
        "tier": "open",
        "launch_tool": "slope",
        "tools": ["slope", "aspect", "accumulation_curvature", "convergence_index"],
    },
    {
        "id": "vector_qa_cleanup_open",
        "title": "Vector QA Cleanup (Open)",
        "summary": "Prepare vectors for analysis with geometry metrics and clipping.",
        "input_hint": "Validate geometry and projection consistency before running cleanup.",
        "output_hint": "Retain intermediate QA outputs for audit and reproducibility.",
        "tier": "open",
        "launch_tool": "add_geometry_attributes",
        "tools": ["add_geometry_attributes", "clip"],
    },
    {
        "id": "corridor_screening_pro",
        "title": "Corridor Screening (Pro)",
        "summary": "Screen feasible corridors and advance to route assessment.",
        "input_hint": "Ensure constraints/cost surfaces are prepared and aligned.",
        "output_hint": "Export corridor candidates and assessed route summaries together.",
        "tier": "pro",
        "launch_tool": "corridor_mapping_intelligence",
        "tools": ["corridor_mapping_intelligence", "assess_route"],
    },
    {
        "id": "lidar_classification_qa_pro",
        "title": "LiDAR Classification QA (Pro)",
        "summary": "Refine and review LiDAR class quality before downstream use.",
        "input_hint": "Use LiDAR tiles with stable class code conventions.",
        "output_hint": "Capture overlap and class-color diagnostics for QA reports.",
        "tier": "pro",
        "launch_tool": "classify_lidar",
        "tools": ["classify_lidar", "classify_overlap_points", "colourize_based_on_class"],
    },
    {
        "id": "building_footprint_intelligence_pro",
        "title": "Building Footprint Intelligence (Pro)",
        "summary": "Extract building structures and remove off-terrain artifacts.",
        "input_hint": "Start from classified LiDAR with reliable ground/non-ground separation.",
        "output_hint": "Compare building class totals before and after cleanup for QA.",
        "tier": "pro",
        "launch_tool": "classify_buildings_in_lidar",
        "tools": ["classify_buildings_in_lidar", "remove_off_terrain_objects"],
    },
    {
        "id": "projection_harmonization_bundle_open",
        "title": "Projection Harmonization Bundle (Open)",
        "summary": "Standardize projection metadata across raster, vector, and LiDAR.",
        "input_hint": "Confirm target EPSG and apply consistently across all inputs.",
        "output_hint": "Record assigned EPSG and refresh catalog layers after updates.",
        "tier": "open",
        "launch_tool": "assign_projection_raster",
        "tools": ["assign_projection_raster", "assign_projection_vector", "assign_projection_lidar"],
    },
]


def user_recipe_file_path() -> Path:
    env_path = os.environ.get("WBW_QGIS_USER_RECIPES")
    if env_path:
        return Path(env_path).expanduser()
    return Path.home() / ".whitebox_workflows_qgis" / "recipes.json"


def user_recipe_template() -> dict[str, Any]:
    return {
        "recipes": [
            {
                "id": "my_custom_terrain_recipe",
                "title": "My Custom Terrain Recipe",
                "summary": "User-defined recipe example.",
                "tier": "open",
                "launch_tool": "slope",
                "tools": ["slope", "aspect", "hillshade"],
                "input_hint": "Set a DEM raster as the primary input.",
                "output_hint": "Write outputs to a dedicated project output folder.",
            }
        ]
    }


def ensure_user_recipe_file() -> Path:
    path = user_recipe_file_path()
    path.parent.mkdir(parents=True, exist_ok=True)
    if not path.exists():
        with open(path, "w", encoding="utf-8") as f:
            json.dump(user_recipe_template(), f, indent=2)
            f.write("\n")
    return path


def _normalize_recipe_entry(entry: Any) -> tuple[dict[str, Any] | None, str]:
    if not isinstance(entry, dict):
        return None, "Recipe entry is not an object."

    recipe_id = str(entry.get("id", "")).strip()
    if not recipe_id:
        return None, "Recipe entry is missing required field 'id'."

    tools_raw = entry.get("tools", [])
    if not isinstance(tools_raw, list):
        return None, f"Recipe '{recipe_id}' has non-list 'tools'."
    tools = [str(t).strip() for t in tools_raw if str(t).strip()]
    if not tools:
        return None, f"Recipe '{recipe_id}' has no tools."

    tier = str(entry.get("tier", "open")).strip().lower() or "open"
    if tier not in {"open", "pro", "enterprise"}:
        return None, f"Recipe '{recipe_id}' has invalid tier '{tier}'."

    launch_tool = str(entry.get("launch_tool", "")).strip() or tools[0]

    normalized = dict(entry)
    normalized["id"] = recipe_id
    normalized["title"] = str(entry.get("title", recipe_id)).strip() or recipe_id
    normalized["summary"] = str(entry.get("summary", "")).strip()
    normalized["tier"] = tier
    normalized["launch_tool"] = launch_tool
    normalized["tools"] = tools
    normalized["input_hint"] = str(entry.get("input_hint", "")).strip()
    normalized["output_hint"] = str(entry.get("output_hint", "")).strip()
    return normalized, ""


def load_all_recipe_definitions() -> tuple[list[dict[str, Any]], list[str]]:
    merged: list[dict[str, Any]] = [dict(r) for r in RECIPE_CATALOG]
    warnings: list[str] = []

    path = user_recipe_file_path()
    if not path.exists():
        return merged, warnings

    try:
        with open(path, "r", encoding="utf-8") as f:
            payload = json.load(f)
    except Exception as exc:
        warnings.append(f"Failed to read user recipes at {path}: {exc}")
        return merged, warnings

    if isinstance(payload, dict):
        user_entries = payload.get("recipes", [])
    elif isinstance(payload, list):
        user_entries = payload
    else:
        warnings.append(f"User recipes at {path} must be a list or an object with 'recipes'.")
        return merged, warnings

    if not isinstance(user_entries, list):
        warnings.append(f"User recipes at {path} contain invalid 'recipes' value.")
        return merged, warnings

    normalized_user: list[dict[str, Any]] = []
    for idx, raw in enumerate(user_entries, start=1):
        recipe, warn = _normalize_recipe_entry(raw)
        if recipe is None:
            if warn:
                recipe_id = ""
                if isinstance(raw, dict):
                    recipe_id = str(raw.get("id", "")).strip()
                if recipe_id:
                    warnings.append(f"Entry {idx} (id={recipe_id}): {warn}")
                else:
                    warnings.append(f"Entry {idx}: {warn}")
            continue
        normalized_user.append(recipe)

    # User recipes override built-ins with same ID.
    by_id = {str(item.get("id", "")): item for item in merged if str(item.get("id", "")).strip()}
    for recipe in normalized_user:
        by_id[recipe["id"]] = recipe
    merged = list(by_id.values())
    return merged, warnings


def _is_tier_allowed(recipe_tier: str, effective_tier: str) -> bool:
    r = str(recipe_tier or "open").strip().lower()
    e = str(effective_tier or "open").strip().lower()
    if r == "open":
        return True
    if r == "pro":
        return e in {"pro", "enterprise"}
    if r == "enterprise":
        return e == "enterprise"
    return False


def visible_recipes(
    *,
    effective_tier: str,
    catalog: list[dict[str, Any]],
    include_locked_discovery: bool = True,
    recipe_catalog: list[dict[str, Any]] | None = None,
) -> list[dict[str, Any]]:
    source_catalog = recipe_catalog if isinstance(recipe_catalog, list) else RECIPE_CATALOG
    catalog_map = {str(item.get("id", "")): item for item in catalog}
    available_tools = {
        tool_id
        for tool_id, item in catalog_map.items()
        if tool_id and not bool(item.get("locked", False))
    }
    catalog_tools = {tool_id for tool_id in catalog_map.keys() if tool_id}

    visible: list[dict[str, Any]] = []
    for recipe in source_catalog:
        recipe_tier = str(recipe.get("tier", "open"))
        tier_allowed = _is_tier_allowed(recipe_tier, effective_tier)
        if not tier_allowed and not include_locked_discovery:
            continue

        tools = [str(t) for t in recipe.get("tools", []) if str(t).strip()]
        if not tools:
            continue

        # Keep recipe visible only if all referenced tools exist in the current catalog.
        if any(t not in catalog_tools for t in tools):
            continue

        launch_tool = str(recipe.get("launch_tool", "")).strip() or tools[0]
        if launch_tool not in catalog_tools:
            continue

        normalized = dict(recipe)
        normalized["launch_tool"] = launch_tool
        normalized["tools"] = tools

        launch_locked = launch_tool not in available_tools
        locked = (not tier_allowed) or launch_locked
        normalized["locked"] = bool(locked)
        if not tier_allowed:
            normalized["locked_reason"] = f"Requires {recipe_tier.upper()} tier runtime."
        elif launch_locked:
            normalized["locked_reason"] = "Launch tool is not available in current runtime catalog."
        else:
            normalized["locked_reason"] = ""

        visible.append(normalized)

    visible.sort(
        key=lambda r: (
            str(r.get("title", "")).strip().lower() or str(r.get("id", "")).strip().lower(),
            str(r.get("id", "")).strip().lower(),
        )
    )
    return visible


def recipe_steps_text(recipe: dict[str, Any]) -> str:
    title = str(recipe.get("title", recipe.get("id", "Recipe"))).strip() or "Recipe"
    tier = str(recipe.get("tier", "open")).strip().upper() or "OPEN"
    summary = str(recipe.get("summary", "")).strip()
    input_hint = str(recipe.get("input_hint", "")).strip()
    output_hint = str(recipe.get("output_hint", "")).strip()
    tools = [str(t) for t in recipe.get("tools", []) if str(t).strip()]

    lines = [f"Recipe: {title}", f"Tier: {tier}"]
    if summary:
        lines.append(f"Summary: {summary}")
    if input_hint:
        lines.append(f"Input hint: {input_hint}")
    if output_hint:
        lines.append(f"Output hint: {output_hint}")

    lines.append("Steps:")
    if tools:
        for idx, tool_id in enumerate(tools, start=1):
            lines.append(f"{idx}. {tool_id}")
    else:
        lines.append("1. (no steps defined)")

    return "\n".join(lines)
