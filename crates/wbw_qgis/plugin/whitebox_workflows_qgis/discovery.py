from __future__ import annotations

import html
import inspect
import os
import re

from .bootstrap import get_runtime_capabilities, get_tool_catalog, load_whitebox_workflows


_RUNTIME_ENV = None
_RUNTIME_STUB_TEXT = None


def _get_runtime_env():
    global _RUNTIME_ENV
    if _RUNTIME_ENV is not None:
        return _RUNTIME_ENV
    try:
        wbw = load_whitebox_workflows()
        env_cls = getattr(wbw, "WbEnvironment", None)
        if env_cls is None:
            _RUNTIME_ENV = False
            return None
        _RUNTIME_ENV = env_cls()
        return _RUNTIME_ENV
    except Exception:
        _RUNTIME_ENV = False
        return None


def _get_runtime_stub_text() -> str:
    global _RUNTIME_STUB_TEXT
    if isinstance(_RUNTIME_STUB_TEXT, str):
        return _RUNTIME_STUB_TEXT
    try:
        wbw = load_whitebox_workflows()
        pkg_dir = os.path.dirname(getattr(wbw, "__file__", "") or "")
        if not pkg_dir:
            _RUNTIME_STUB_TEXT = ""
            return _RUNTIME_STUB_TEXT
        candidate = os.path.join(pkg_dir, "whitebox_workflows.pyi")
        if not os.path.exists(candidate):
            _RUNTIME_STUB_TEXT = ""
            return _RUNTIME_STUB_TEXT
        with open(candidate, "r", encoding="utf-8") as f:
            _RUNTIME_STUB_TEXT = f.read()
        return _RUNTIME_STUB_TEXT
    except Exception:
        _RUNTIME_STUB_TEXT = ""
        return _RUNTIME_STUB_TEXT


# Transitional compatibility: these tools are now treated as open-core in the
# product split but may still surface as locked from legacy runtime metadata.
_OPEN_CORE_LOCK_OVERRIDES = {
    "assess_route",
    "average_horizon_distance",
    "breakline_mapping",
    "horizon_area",
    "local_hypsometric_analysis",
    "low_points_on_headwater_divides",
    "shadow_animation",
    "shadow_image",
    "skyline_analysis",
    "slope_vs_aspect_plot",
    "smooth_vegetation_residual",
    "topo_render",
    "topographic_hachures",
    "topographic_position_animation",
    "topological_breach_burn",
}


def _split_signature_args(args_text: str) -> list[str]:
    parts: list[str] = []
    current: list[str] = []
    depth = 0
    for ch in args_text:
        if ch == "," and depth == 0:
            part = "".join(current).strip()
            if part:
                parts.append(part)
            current = []
            continue
        if ch in "([{" :
            depth += 1
        elif ch in ")]}":
            depth = max(0, depth - 1)
        current.append(ch)
    tail = "".join(current).strip()
    if tail:
        parts.append(tail)
    return parts


def _humanize_param_name(name: str) -> str:
    cleaned = re.sub(r"[_\-]+", " ", str(name or "").strip())
    cleaned = re.sub(r"\s+", " ", cleaned).strip()
    if not cleaned:
        return "Parameter"
    return cleaned[0].upper() + cleaned[1:]


def _fallback_param_description(name: str, type_text: str, required: bool) -> str:
    n = str(name or "").strip().lower()
    t = str(type_text or "").strip().lower()

    if n in {"dem", "input", "input_raster", "base", "surface", "raster", "grid"}:
        return "Input raster layer."
    if n in {"vector", "input_vector", "lines", "points", "polygons", "streams"}:
        return "Input vector layer."
    if n.startswith(("output", "out")) or n in {"destination", "dst"}:
        return "Output destination path."
    if n in {"z_factor", "zfactor"}:
        return "Z conversion factor."
    if n in {"units", "unit"}:
        return "Output units."

    if "bool" in t:
        return f"{_humanize_param_name(n)} option."
    if any(tok in t for tok in ("int", "float", "double", "number")):
        return f"{_humanize_param_name(n)} value."
    if "raster" in t:
        return "Input raster layer."
    if "vector" in t:
        return "Input vector layer."
    if "lidar" in t:
        return "Input LiDAR file."

    if required:
        return f"{_humanize_param_name(n)} (required)."
    return _humanize_param_name(n)


def _parse_signature_default(default_text: str):
    raw = str(default_text or "").strip()
    if not raw:
        return None

    lower = raw.lower()
    if lower in {"true", "false"}:
        return lower == "true"
    if lower in {"none", "null"}:
        return None

    if (raw.startswith("\"") and raw.endswith("\"")) or (
        raw.startswith("'") and raw.endswith("'")
    ):
        return raw[1:-1]

    try:
        if "." in raw or "e" in lower:
            return float(raw)
        return int(raw)
    except Exception:
        pass

    # Leave complex expressions (e.g. float('inf'), enums) unresolved.
    return None


def _infer_params_from_help_static(tool_id: str) -> list[dict]:
    base_dir = os.path.dirname(__file__)
    help_path = os.path.join(base_dir, "help_static", f"{tool_id}.html")
    if not os.path.exists(help_path):
        return []

    try:
        with open(help_path, "r", encoding="utf-8") as f:
            html_text = f.read()
    except Exception:
        return []

    text = html.unescape(html_text)
    sig_match = re.search(
        rf"<code>\s*def\s+{re.escape(tool_id)}\s*\((.*?)\)\s*->\s*[^<:]+:.*?</code>",
        text,
        flags=re.IGNORECASE | re.DOTALL,
    )
    if sig_match is None:
        return []

    args_text = sig_match.group(1).strip()
    if not args_text:
        return []

    params: list[dict] = []
    for raw_part in _split_signature_args(args_text):
        part = raw_part.strip()
        if not part or part in {"self", "*", "/"}:
            continue

        default_provided = "=" in part
        default_value = None
        if default_provided:
            left, right = [x.strip() for x in part.split("=", 1)]
            default_value = _parse_signature_default(right)
        else:
            left = part
        if ":" in left:
            name, type_text = [x.strip() for x in left.split(":", 1)]
        else:
            name, type_text = left.strip(), "string"

        if not name:
            continue

        is_required = not default_provided
        params.append(
            {
                "name": name,
                "description": _fallback_param_description(name, type_text, is_required),
                "required": is_required,
                "default": default_value,
            }
        )

    # If the runtime manifest omitted output params entirely, recover a destination
    # from the static return type when possible.
    has_output = any(
        str(p.get("name", "")).lower().startswith(("output", "out", "destination"))
        for p in params
    )
    if not has_output:
        ret_match = re.search(
            rf"<code>\s*def\s+{re.escape(tool_id)}\s*\(.*?\)\s*->\s*([^<:]+):",
            text,
            flags=re.IGNORECASE | re.DOTALL,
        )
        return_type = ret_match.group(1).strip() if ret_match else ""
        rlower = return_type.lower()

        if "raster" in rlower:
            params.append(
                {
                    "name": "output",
                    "description": "Output raster destination path.",
                    "required": True,
                }
            )
        elif "vector" in rlower:
            params.append(
                {
                    "name": "output",
                    "description": "Output vector destination path.",
                    "required": True,
                }
            )
        elif "lidar" in rlower:
            params.append(
                {
                    "name": "output",
                    "description": "Output LiDAR destination path.",
                    "required": True,
                }
            )

    return params


def _infer_params_from_runtime_callable(tool_id: str) -> list[dict]:
    env = _get_runtime_env()
    if env is None:
        return []

    fn = getattr(env, tool_id, None)
    if not callable(fn):
        return []

    try:
        sig = inspect.signature(fn)
    except Exception:
        return []

    params: list[dict] = []
    for p in sig.parameters.values():
        name = str(p.name)
        if name in {"self", "callback"}:
            continue

        default = p.default
        default_provided = default is not inspect._empty
        default_value = None if default is inspect._empty else default

        # Normalize callable sentinel defaults to None.
        if callable(default_value):
            default_value = None

        is_required = not default_provided
        params.append(
            {
                "name": name,
                "description": _fallback_param_description(name, "", is_required),
                "required": is_required,
                "default": default_value,
            }
        )

    # Map common runtime naming into plugin-facing output naming conventions.
    for p in params:
        n = str(p.get("name", "")).lower()
        if n.endswith("_output_path") or n == "output_path":
            p["name"] = "output"
            p["description"] = "Output destination path."
            p["required"] = True

    return params


def _infer_params_from_runtime_stub(tool_id: str) -> list[dict]:
    text = _get_runtime_stub_text()
    if not text:
        return []

    m = re.search(
        rf"def\s+{re.escape(tool_id)}\s*\(\s*self\s*,\s*(.*?)\)\s*->",
        text,
        flags=re.IGNORECASE | re.DOTALL,
    )
    if m is None:
        return []

    args_text = m.group(1).strip()
    if not args_text:
        return []

    params: list[dict] = []
    for raw_part in _split_signature_args(args_text):
        part = raw_part.strip()
        if not part or part in {"self", "*", "/"}:
            continue

        default_provided = "=" in part
        default_value = None
        if default_provided:
            left, right = [x.strip() for x in part.split("=", 1)]
            default_value = _parse_signature_default(right)
        else:
            left = part

        if ":" in left:
            name, type_text = [x.strip() for x in left.split(":", 1)]
        else:
            name, type_text = left.strip(), "string"
        if not name:
            continue
        if name == "callback":
            continue

        is_required = not default_provided
        params.append(
            {
                "name": name,
                "description": _fallback_param_description(name, type_text, is_required),
                "required": is_required,
                "default": default_value,
            }
        )

    for p in params:
        n = str(p.get("name", "")).lower()
        if n.endswith("_output_path") or n == "output_path":
            p["name"] = "output"
            p["description"] = "Output destination path."
            p["required"] = True

    return params


def _hydrate_missing_params(catalog: list[dict]) -> list[dict]:
    out: list[dict] = []
    for item in catalog:
        fixed = dict(item)
        params = fixed.get("params")
        if isinstance(params, list) and len(params) > 0:
            out.append(fixed)
            continue

        tool_id = str(fixed.get("id", "")).strip()
        inferred = _infer_params_from_help_static(tool_id)
        if not inferred:
            inferred = _infer_params_from_runtime_callable(tool_id)
        if not inferred:
            inferred = _infer_params_from_runtime_stub(tool_id)
        if inferred:
            fixed["params"] = inferred
            existing_defaults = dict(fixed.get("defaults", {}) or {})
            for p in inferred:
                name = str(p.get("name", "")).strip()
                if not name or name in existing_defaults:
                    continue
                if "default" in p and p.get("default") is not None:
                    existing_defaults[name] = p.get("default")
            if existing_defaults:
                fixed["defaults"] = existing_defaults
        out.append(fixed)
    return out


def _normalize_lock_state(catalog: list[dict]) -> list[dict]:
    out: list[dict] = []
    for item in catalog:
        fixed = dict(item)
        tool_id = str(fixed.get("id", "")).strip()
        if tool_id in _OPEN_CORE_LOCK_OVERRIDES:
            fixed["locked"] = False
            fixed["available"] = True
            fixed["locked_reason"] = None
            fixed["license_tier"] = "Open"
            fixed["license_tier_name"] = "open"
            fixed["availability_state"] = "available"
        out.append(fixed)
    return out


def _looks_like_remote_sensing_tool(item: dict) -> bool:
    tool_id = str(item.get("id", "") or "").strip().lower()
    display = str(item.get("display_name", "") or "").strip().lower()
    summary = str(item.get("summary", "") or "").strip().lower()
    tags = [str(t).strip().lower() for t in item.get("tags", []) if str(t).strip()]

    # Projection and CRS wrappers are intentionally grouped with their source
    # data families rather than remote sensing.
    projection_tokens = {"projection", "reproject", "assign", "crs", "epsg"}
    if any(tok in tool_id for tok in projection_tokens):
        return False
    if any(tok in display for tok in projection_tokens):
        return False
    if any(tok in summary for tok in projection_tokens):
        return False

    text_parts = [tool_id, display, summary] + tags

    # Strong remote-sensing cues seen in both open and pro catalogs.
    remote_tokens = {
        "remote_sensing",
        "remote sensing",
        "image",
        "imagery",
        "spectral",
        "multispectral",
        "hyperspectral",
        "band",
        "bands",
        "ndvi",
        "radiometric",
        "classification",
        "classifier",
        "texture",
        "segmentation",
        "landsat",
        "sentinel",
        "sar",
        "radar",
        "coherence",
        "interferogram",
        "backscatter",
        "pansharpen",
        "pan-sharpen",
    }

    # Terrain/hydrology terms that should not be auto-promoted to remote
    # sensing when they appear alone.
    terrain_hydro_tokens = {
        "terrain",
        "geomorph",
        "hydrology",
        "watershed",
        "flow",
        "stream",
        "depression",
        "breach",
        "fill",
        "d8",
        "dinf",
        "twi",
        "slope",
        "aspect",
        "hillshade",
    }

    remote_hits = 0
    terrain_hydro_hits = 0
    for part in text_parts:
        if not part:
            continue
        for token in remote_tokens:
            if token in part:
                remote_hits += 1
        for token in terrain_hydro_tokens:
            if token in part:
                terrain_hydro_hits += 1

    if remote_hits == 0:
        return False

    # Require remote-sensing signal to be at least as strong as terrain/hydrology
    # signal for broad raster tools.
    return remote_hits >= terrain_hydro_hits


def _derive_remote_sensing_category(item: dict) -> str:
    tool_id = str(item.get("id", "") or "").strip().lower()
    display = str(item.get("display_name", "") or "").strip().lower()
    summary = str(item.get("summary", "") or "").strip().lower()
    tags = [str(t).strip().lower() for t in item.get("tags", []) if str(t).strip()]

    corpus = [tool_id, display, summary] + tags

    def _has_any(tokens: set[str]) -> bool:
        for part in corpus:
            if not part:
                continue
            for token in tokens:
                if token in part:
                    return True
        return False

    # Curated allow-lists for stronger, predictable grouping.
    classification_tokens = {
        "classification",
        "classifier",
        "svm",
        "random forest",
        "knn",
        "kmeans",
        "fuzzy c",
        "training",
        "confusion",
        "accuracy assessment",
    }
    filter_texture_tokens = {
        "texture",
        "filter",
        "convolution",
        "edge",
        "sobel",
        "canny",
        "morpholog",
        "denoise",
        "bilateral",
        "gaussian",
        "median",
    }
    spectral_tokens = {
        "spectral",
        "multispectral",
        "hyperspectral",
        "ndvi",
        "evi",
        "band",
        "radiometric",
        "atmospheric",
        "landsat",
        "sentinel",
    }
    sar_tokens = {
        "sar",
        "radar",
        "interferogram",
        "coherence",
        "backscatter",
        "insar",
        "polarimet",
    }

    if _has_any(sar_tokens):
        return "Remote Sensing - SAR"
    if _has_any(classification_tokens):
        return "Remote Sensing - Classification"
    if _has_any(filter_texture_tokens):
        return "Remote Sensing - Filters"
    if _has_any(spectral_tokens):
        return "Remote Sensing - Spectral"
    return "Remote Sensing"


def _reclassify_broad_categories(catalog: list[dict]) -> list[dict]:
    out: list[dict] = []
    for item in catalog:
        fixed = dict(item)
        base_category = str(fixed.get("category", "") or "").strip().lower()

        if base_category == "raster" and _looks_like_remote_sensing_tool(fixed):
            fixed["category"] = _derive_remote_sensing_category(fixed)
            tags = [str(t).strip() for t in fixed.get("tags", []) if str(t).strip()]
            tag_set = {t.lower() for t in tags}
            if "remote_sensing" not in tag_set and "remote sensing" not in tag_set:
                tags.append("remote_sensing")
            fixed["tags"] = tags

        out.append(fixed)
    return out


def _inject_projection_wrapper_tools(catalog: list[dict]) -> list[dict]:
    existing_ids = {str(item.get("id", "")).strip() for item in catalog}

    wrappers: list[dict] = [
        {
            "id": "reproject_raster",
            "display_name": "Reproject Raster",
            "summary": "Reprojects a raster to a target EPSG code.",
            "category": "Raster",
            "tags": ["projection", "crs", "reproject", "raster"],
            "params": [
                {"name": "input", "description": "Input raster layer.", "required": True},
                {"name": "epsg", "description": "Target EPSG code.", "required": True},
                {"name": "output", "description": "Output raster destination path.", "required": True},
            ],
            "defaults": {"resample": "nearest"},
            "available": True,
            "locked": False,
            "locked_reason": None,
            "license_tier": "Open",
            "license_tier_name": "open",
            "availability_state": "available",
            "display_default_visible": True,
            "display_default_favorite": False,
            "render_hints": {},
        },
        {
            "id": "reproject_lidar",
            "display_name": "Reproject LiDAR",
            "summary": "Reprojects a LiDAR dataset to a target EPSG code.",
            "category": "Lidar",
            "tags": ["projection", "crs", "reproject", "lidar"],
            "params": [
                {"name": "input", "description": "Input LiDAR file.", "required": True},
                {"name": "epsg", "description": "Target EPSG code.", "required": True},
                {"name": "output", "description": "Output LiDAR destination path.", "required": True},
            ],
            "available": True,
            "locked": False,
            "locked_reason": None,
            "license_tier": "Open",
            "license_tier_name": "open",
            "availability_state": "available",
            "display_default_visible": True,
            "display_default_favorite": False,
            "render_hints": {},
        },
        {
            "id": "assign_projection_raster",
            "display_name": "Assign Projection Raster",
            "summary": "Assigns CRS metadata (EPSG) to a raster in place without coordinate transformation.",
            "category": "Raster",
            "tags": ["projection", "crs", "assign", "raster"],
            "params": [
                {"name": "input", "description": "Input raster layer.", "required": True},
                {"name": "epsg", "description": "EPSG code to assign.", "required": True},
            ],
            "available": True,
            "locked": False,
            "locked_reason": None,
            "license_tier": "Open",
            "license_tier_name": "open",
            "availability_state": "available",
            "display_default_visible": True,
            "display_default_favorite": False,
            "render_hints": {},
        },
        {
            "id": "assign_projection_vector",
            "display_name": "Assign Projection Vector",
            "summary": "Assigns CRS metadata (EPSG) to a vector dataset in place without coordinate transformation.",
            "category": "Vector",
            "tags": ["projection", "crs", "assign", "vector"],
            "params": [
                {"name": "input", "description": "Input vector layer.", "required": True},
                {"name": "epsg", "description": "EPSG code to assign.", "required": True},
            ],
            "available": True,
            "locked": False,
            "locked_reason": None,
            "license_tier": "Open",
            "license_tier_name": "open",
            "availability_state": "available",
            "display_default_visible": True,
            "display_default_favorite": False,
            "render_hints": {},
        },
        {
            "id": "assign_projection_lidar",
            "display_name": "Assign Projection LiDAR",
            "summary": "Assigns CRS metadata (EPSG) to a LiDAR dataset in place without coordinate transformation.",
            "category": "Lidar",
            "tags": ["projection", "crs", "assign", "lidar"],
            "params": [
                {"name": "input", "description": "Input LiDAR file.", "required": True},
                {"name": "epsg", "description": "EPSG code to assign.", "required": True},
            ],
            "available": True,
            "locked": False,
            "locked_reason": None,
            "license_tier": "Open",
            "license_tier_name": "open",
            "availability_state": "available",
            "display_default_visible": True,
            "display_default_favorite": False,
            "render_hints": {},
        },
    ]

    out = list(catalog)
    for item in wrappers:
        tid = str(item.get("id", "")).strip()
        if tid and tid not in existing_ids:
            out.append(item)
            existing_ids.add(tid)
    return out


def _ensure_feature_preserving_smoothing(catalog: list[dict]) -> list[dict]:
    """Guarantee the legacy smoothing tool remains discoverable in the panel.

    In mixed/runtime-transition environments the tool can occasionally be omitted
    from catalog payloads. Keep a stable fallback entry so users can still run
    it from QGIS when the runtime surface supports it.
    """
    target_id = "feature_preserving_smoothing"
    existing_ids = {str(item.get("id", "")).strip() for item in catalog}
    if target_id in existing_ids:
        return catalog

    params = _infer_params_from_help_static(target_id)
    if not params:
        params = _infer_params_from_runtime_callable(target_id)
    if not params:
        params = _infer_params_from_runtime_stub(target_id)

    fallback = {
        "id": target_id,
        "display_name": "Feature Preserving Smoothing",
        "summary": "Smooths DEM roughness while preserving sharp terrain edges.",
        "category": "Terrain",
        "tags": ["terrain", "dem", "smoothing", "feature-preserving"],
        "params": params or [
            {"name": "input", "description": "Input raster layer.", "required": True},
            {"name": "output", "description": "Output raster destination path.", "required": True},
        ],
        "defaults": {},
        "available": True,
        "locked": False,
        "locked_reason": None,
        "license_tier": "Open",
        "license_tier_name": "open",
        "availability_state": "available",
        "display_default_visible": True,
        "display_default_favorite": False,
        "render_hints": {},
    }

    out = list(catalog)
    out.append(fallback)
    return out


def discover_runtime(include_pro: bool = True, tier: str = "open") -> dict:
    return get_runtime_capabilities(include_pro=include_pro, tier=tier)


def discover_tool_catalog(include_pro: bool = True, tier: str = "open") -> list[dict]:
    catalog = get_tool_catalog(include_pro=include_pro, tier=tier)
    catalog = _ensure_feature_preserving_smoothing(catalog)
    catalog = _normalize_lock_state(catalog)
    catalog = _reclassify_broad_categories(catalog)
    catalog = _inject_projection_wrapper_tools(catalog)
    catalog = _hydrate_missing_params(catalog)

    def _rank_value(item: dict) -> int:
        raw = item.get("display_default_rank")
        try:
            if raw is None:
                return 999_999
            return int(raw)
        except Exception:
            return 999_999

    return sorted(
        catalog,
        key=lambda item: (
            _rank_value(item),
            item.get("category", ""),
            item.get("display_name", ""),
            item.get("id", ""),
        ),
    )


def split_catalog_by_availability(catalog: list[dict]) -> tuple[list[dict], list[dict]]:
    available = [item for item in catalog if item.get("available")]
    locked = [item for item in catalog if not item.get("available")]
    return available, locked


def refresh_and_build_help(
    include_pro: bool = True,
    tier: str = "open",
    *,
    force: bool = False,
) -> tuple[list[dict], dict[str, str]]:
    """Discover the current tool catalog and generate help HTML files.

    Intended as a top-level convenience for the settings/refresh action.

    Returns:
        (catalog, help_index) where help_index maps tool_id → HTML file path.
    """
    from .help import generate_help_files

    catalog = discover_tool_catalog(include_pro=include_pro, tier=tier)
    wbw = load_whitebox_workflows()
    help_index = generate_help_files(wbw, catalog, force=force)
    return catalog, help_index